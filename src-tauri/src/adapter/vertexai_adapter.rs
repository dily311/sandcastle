//! Google Vertex AI (Gemini 퍼블리셔 모델)
//!
//! credential은 API 키가 아니라 JSON 문자열이다:
//! `{ "service_account": "<서비스 계정 JSON 문자열>", "location": "us-central1", "project_id": "..." }`
use async_trait::async_trait;
use gcp_auth::{CustomServiceAccount, TokenProvider};
use serde_json::Value;

use crate::{
    adapter::{
        ensure_success,
        gemini_adapter::{gemini_body_with, model_id, read_gemini_response, task_type},
        l2_normalize, net_err,
        openai_adapter::floats,
        parse_err, AiProvider, ChatExtras, EmbedRequest, EmbedTask, Embeddings, Generation, OnDelta, ProviderCall,
    },
    error::{AppError, ErrorCode, Result},
    models::ai_service::{PromptMessage, VertexChatRequestAPIKey},
};

/// predict 한 번에 보내는 글 수 (text-embedding-005 등은 250개·토큰 2만 개 한도라 여유를 둠). `gemini-embedding-*`은 요청당 1개
const EMBED_BATCH: usize = 32;

fn embed_batch(model: &str) -> usize {
    if model.contains("gemini-embedding") {
        1
    } else {
        EMBED_BATCH
    }
}

/// `:predict` 본문. `autoTruncate`는 기본이 true라 긴 글이 소리 없이 잘리므로 끄고(너무 길면 오류), 차원 줄이기는 `outputDimensionality`
pub(crate) fn embed_body(chunk: &[String], task: EmbedTask, dimensions: Option<u32>) -> Value {
    let instances: Vec<Value> = chunk
        .iter()
        .map(|t| serde_json::json!({ "content": t, "task_type": task_type(task) }))
        .collect();
    let mut params = serde_json::json!({ "autoTruncate": false });
    if let Some(d) = dimensions {
        params["outputDimensionality"] = serde_json::json!(d);
    }
    serde_json::json!({ "instances": instances, "parameters": params })
}

/// predict 응답 → (벡터들, 입력 토큰 합계). 토큰은 `embeddings.statistics.token_count`가 있을 때만
pub(crate) fn parse_embeddings(json: &Value, expect: usize, normalize: bool) -> Result<(Vec<Vec<f32>>, Option<u64>)> {
    let rows = json["predictions"].as_array().ok_or_else(|| parse_err("임베딩", "predictions가 없음"))?;
    if rows.len() != expect {
        return Err(parse_err("임베딩", format!("{expect}개를 보냈는데 {}개가 옴", rows.len())));
    }
    let mut tokens: Option<u64> = None;
    let mut vectors = vec![];
    for r in rows {
        let e = &r["embeddings"];
        let mut v = floats(&e["values"]).ok_or_else(|| parse_err("임베딩", "values가 숫자 목록이 아님"))?;
        if normalize {
            l2_normalize(&mut v);
        }
        if let Some(t) = e["statistics"]["token_count"].as_f64() {
            *tokens.get_or_insert(0) += t as u64;
        }
        vectors.push(v);
    }
    Ok((vectors, tokens))
}

const SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

pub struct VertexAIClient;

struct VertexAuth {
    token: String,
    location: String,
    project_id: String,
}

impl VertexAIClient {
    async fn auth(credential: &str) -> Result<VertexAuth> {
        let auth_err = |msg: String| AppError::ai_provider(ErrorCode::AiProviderAuthFailed, msg, false);
        let key: VertexChatRequestAPIKey = serde_json::from_str(credential).map_err(|e| auth_err(format!("Vertex 인증 정보 형식이 올바르지 않습니다: {e}")))?;
        let account = CustomServiceAccount::from_json(&key.service_account).map_err(|e| auth_err(format!("서비스 계정을 읽을 수 없습니다: {e}")))?;
        let token = account.token(&[SCOPE]).await.map_err(|e| auth_err(format!("GCP 인증 토큰 발급 실패: {e}")))?;
        Ok(VertexAuth {
            token: token.as_str().to_string(),
            location: key.location,
            project_id: key.project_id,
        })
    }

    fn host(location: &str) -> String {
        if location == "global" {
            "https://aiplatform.googleapis.com".to_string()
        } else {
            format!("https://{location}-aiplatform.googleapis.com")
        }
    }
}

#[async_trait]
impl AiProvider for VertexAIClient {
    async fn list_models(&self, call: &ProviderCall<'_>) -> Result<Vec<String>> {
        let auth = Self::auth(call.credential).await?;
        let url = format!("{}/v1beta1/publishers/google/models", Self::host(&auth.location));
        let resp = call
            .http
            .get(url)
            .bearer_auth(&auth.token)
            .header("x-goog-user-project", &auth.project_id)
            .send()
            .await
            .map_err(net_err)?;
        let json: Value = ensure_success(resp).await?.json().await.map_err(|e| parse_err("모델 목록", e))?;
        Ok(json["publisherModels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str())
                    .map(|n| n.rsplit('/').next().unwrap_or(n).to_string())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn generate_with(&self, call: &ProviderCall<'_>, prompt: &PromptMessage, extras: &ChatExtras, on_delta: &mut OnDelta<'_>) -> Result<Generation> {
        let auth = Self::auth(call.credential).await?;
        let stream = prompt.chat_options.stream;
        let method = if stream { "streamGenerateContent?alt=sse" } else { "generateContent" };
        let url = format!(
            "{}/v1/projects/{}/locations/{}/publishers/google/models/{}:{method}",
            Self::host(&auth.location),
            auth.project_id,
            auth.location,
            model_id(&call.profile.model),
        );
        let resp = call
            .http
            .post(url)
            .bearer_auth(&auth.token)
            .json(&gemini_body_with(prompt, extras))
            .send()
            .await
            .map_err(net_err)?;
        read_gemini_response(resp, stream, on_delta).await
    }

    async fn embed(&self, call: &ProviderCall<'_>, req: &EmbedRequest<'_>) -> Result<Embeddings> {
        let auth = Self::auth(call.credential).await?;
        let model = model_id(req.model);
        let url = format!(
            "{}/v1/projects/{}/locations/{}/publishers/google/models/{model}:predict",
            Self::host(&auth.location),
            auth.project_id,
            auth.location,
        );
        let mut out = Embeddings::default();
        let mut tokens: Option<u64> = None;
        for chunk in req.inputs.chunks(embed_batch(model)) {
            let resp = call
                .http
                .post(&url)
                .bearer_auth(&auth.token)
                .header("x-goog-user-project", &auth.project_id)
                .json(&embed_body(chunk, req.task, req.dimensions))
                .send()
                .await
                .map_err(net_err)?;
            let json: Value = ensure_success(resp).await?.json().await.map_err(|e| parse_err("임베딩", e))?;
            let (vectors, t) = parse_embeddings(&json, chunk.len(), req.dimensions.is_some())?;
            out.vectors.extend(vectors);
            if let Some(t) = t {
                *tokens.get_or_insert(0) += t;
            }
        }
        out.tokens = tokens;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 시험용으로 만든 키 (어디에도 등록되지 않은 가짜 서비스 계정)
    const FAKE_KEY: [&str; 28] = [
        "-----BEGIN PRIVATE KEY-----
",
        "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCh1RyW4TNhgpTH
",
        "jLItWR0lz8S9W1kqhxgiPkV+rlQrueRIu9PCKQllDlZFz7os7M8FGlxbU1/wvhtW
",
        "4iUKoRAycN3Gr9YpEs1TMrcopudpt+4mqWoZsFaPxdTrU/0W5Tw2r2BRvKyhxAJF
",
        "zAJ/gAlxaE9JS9DBAcuf1aO9Ud1QWFRv+UjiEWqLHeSM7aaUBSvYRoxBJPV5kntm
",
        "1hoh86X7Jk4va5AIXfA0qIUeILEV8pAt+9HYd+ZZ+Wpze4JUH8IJAnPuVAAJ8/M4
",
        "+Ir0YCtR0fp1B8uAd1mfhTx135+ioystnUOaD3y+YqYHxdIcSLMwahT2Cvud//ug
",
        "Jax3Mdf1AgMBAAECggEAEeNIgbTqxQB2CYiXuUN8YU2R5YxPUtAT7tULQXFrhFnL
",
        "ze5U/cUjcLQg/R506bpTN7EuPDn/BNE+nF/q0D31WF35ZZjSB0A4ydV1QV0RfzIB
",
        "dOyM+cHwD3xCiLCD7R/xg+nmr6I3QrozthtSu10frhVljih/LpZQN56ZtrdxywuL
",
        "M9byVgwEx6eNoWTjfq0YRNVCON756AZiTmeGDd5D8AYPCcdHCyzOt22C8OuWx8ki
",
        "+58vMdhpHro9f3Io/W7CxyCRNgCZXd/8ga+qUZVNfWwfWqYo9uIgLDnsZG2iHlkg
",
        "fywaEdBJ0p3jMmNaveoMqHCwQbcKJHNnIMMCS7GaAQKBgQDgYQiRKLb20bqpG+yP
",
        "QgxLNKqaFZq3B5ezszMX6VgGhwJzG/2/XCQGoT0uhPJOG9ZVAymeMGn+HAMaMIKt
",
        "kWIy/3ijwct2dof20lBIQX+EKU+8wqUUDKCTAQgvCRm3DPhWAA9pK/KJAOI9IMEt
",
        "32Mr1yDgGkoZ7sU/NJM3XVVKYQKBgQC4o5LHYTQvdapKQCWHtyn3VGT9r6rw3/I3
",
        "WNHsZxp/tWCIGrKgqmTRiV/yJeJ83h/PAFZFtsUAuj0gSBKvLHw+ZXHFMtScSgJO
",
        "nMqcRpxErS/xaB50RTkNo6UI9a2BQJijaMMZ+USLmQqYGzsGS7YVL0FECkkDxRXb
",
        "OwK6sfd+FQKBgQChWUWG1MdVD5Xxu1HN5onenr27VaPO0BfnsgogvLx+3SEG028R
",
        "51lEpzfOtu1i+6ORuFd6GIhvoYcdkxq1Y2FBGPKQl3AO/Mdi1vPCMc9XozlinHmG
",
        "xFnHvCs9X4btWddOSHbMgzzCBi+qE1629PdkdCuA24bDxS5z+AORAX2uoQKBgQCh
",
        "c89WbuYuFerpcHwg14Zn0rFYr7J/XKQ1x4IRAPtH7vdRLF9EZJsyKrjuSjhpTeZs
",
        "oPPPlAliIHhw8PT9np2qBhTdBhyp6YFo8mutCYV9/B7vLkdrDljcV1tpoUKtwil2
",
        "wpXP8YsBsrpz1WPglnXKwn6Em6XLGRyKjmhNjm9HHQKBgD+WR7iG/n1sN9WgBoqK
",
        "XpresPzYq7JwyRzp3onuEJ+jWAyEHmZGQ2GKxLpVK3CUmm7ogvwAnk13sLrKjE38
",
        "oW3rlaVhbF6x0XI6qVpO8+pDFojVxUirKsI92KUEuXjixgyixam2tLHP8etEm34R
",
        "m/AUZhmNn0ZhbXSaFTtncOG4
",
        "-----END PRIVATE KEY-----
",
    ];

    fn fake_account() -> String {
        serde_json::json!({
            "type": "service_account",
            "project_id": "sandcastle-test",
            "private_key_id": "0",
            "private_key": FAKE_KEY.concat(),
            "client_email": "fake@sandcastle-test.iam.gserviceaccount.com",
            "client_id": "0",
            "token_uri": "https://oauth2.googleapis.com/token"
        })
        .to_string()
    }

    /// 0.2.0 실기기: Android에는 OS 루트 인증서를 읽는 길이 없어 서비스 계정을 읽는 순간(HTTPS 클라이언트를 만들 때) 실패했다.
    /// 루트 인증서를 바이너리에 넣었으므로(gcp_auth webpki-roots) OS 인증서가 없어도 읽혀야 한다.
    /// PC에서 Android처럼 확인하려면: SSL_CERT_DIR을 빈 폴더로 두고 이 테스트를 돌린다 (rustls-native-certs는 그 폴더만 본다)
    #[test]
    fn predict_body_and_response_shapes() {
        let b = embed_body(&["가".to_string(), "나".to_string()], EmbedTask::Query, Some(256));
        assert_eq!(b["instances"][1], serde_json::json!({ "content": "나", "task_type": "RETRIEVAL_QUERY" }));
        assert_eq!(
            b["parameters"],
            serde_json::json!({ "autoTruncate": false, "outputDimensionality": 256 }),
            "조용히 잘리지 않게 autoTruncate를 끈다"
        );
        assert!(embed_body(&["a".to_string()], EmbedTask::Document, None)["parameters"]
            .get("outputDimensionality")
            .is_none());
        assert_eq!((embed_batch("gemini-embedding-001"), embed_batch("text-embedding-005")), (1, 32));

        let resp = serde_json::json!({ "predictions": [
            { "embeddings": { "values": [3.0, 4.0], "statistics": { "token_count": 5, "truncated": false } } },
            { "embeddings": { "values": [0.0, 2.0], "statistics": { "token_count": 7.0 } } }
        ] });
        let (v, t) = parse_embeddings(&resp, 2, true).unwrap();
        assert_eq!((v[0].clone(), v[1].clone(), t), (vec![0.6, 0.8], vec![0.0, 1.0], Some(12)));
        assert_eq!(parse_embeddings(&resp, 2, false).unwrap().0[0], vec![3.0, 4.0]);
        assert!(parse_embeddings(&resp, 3, false).is_err());
        assert_eq!(
            parse_embeddings(&serde_json::json!({ "predictions": [{ "embeddings": { "values": [1.0] } }] }), 1, false)
                .unwrap()
                .1,
            None
        );
    }

    #[test]
    fn service_account_loads_without_os_root_certificates() {
        CustomServiceAccount::from_json(&fake_account()).expect("OS 루트 인증서 없이도 서비스 계정을 읽어야 함");
    }
}
