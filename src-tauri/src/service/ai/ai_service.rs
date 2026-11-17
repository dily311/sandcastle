use reqwest::Client;
use tokio_util::sync::CancellationToken;

use crate::{
    adapter::{provider_for, ProviderCall},
    db::{
        self,
        conn::DbPool,
        usage::{UsageEntry, UsageKind},
    },
    error::{AppError, ErrorCode, Result},
    models::ai_service::{ApiProfiles, PromptMessage},
    service::secret_store::{api_key_name, SecretStore},
};

/// AI 생성 요청
#[derive(Debug, Clone)]
pub struct AiRequest {
    /// api_profiles.id
    pub profile_id: String,
    /// PromptManager가 만든 최종 프롬프트 (stream 여부는 prompt.chat_options.stream)
    pub prompt: PromptMessage,
}

#[derive(Debug, Clone)]
pub struct AiResponse {
    pub content: String,
    pub finish_reason: Option<String>,
    pub profile_id: String,
    pub model: String,
}

pub struct AiService {
    db: DbPool,
    http: Client,
}

impl AiService {
    pub fn new(db: DbPool, http: Client) -> Self {
        Self { db, http }
    }

    /// 플러그인(음성·이미지)이 공급자 API를 직접 부를 때 쓰는 HTTP 클라이언트
    pub fn http(&self) -> &Client {
        &self.http
    }

    /// 사용량 한 줄 기록 (음성·이미지 플러그인도 이걸로 남긴다)
    pub async fn record_usage(&self, entry: UsageEntry) {
        db::usage::record(&self.db, entry).await;
    }

    /// 예산 한도 확인 (공급자를 부르기 직전에. 넘었으면 BUDGET_EXCEEDED)
    pub async fn ensure_budget(&self, profile_id: &str, kind: UsageKind) -> Result<()> {
        crate::service::budget::ensure(&self.db, profile_id, kind).await
    }

    /// 프로필의 API 키 (keyring)
    pub fn credential(&self, profile: &ApiProfiles) -> Result<String> {
        load_credential(profile)
    }

    pub async fn load_profile(&self, profile_id: &str) -> Result<ApiProfiles> {
        if profile_id.is_empty() {
            return Err(AppError::validation("선택된 API 프로필이 없습니다", None));
        }
        Ok(db::ai_service::get_api_profile(&self.db, profile_id).await?)
    }

    /// 프로필의 공급자에서 사용 가능한 모델 목록
    pub async fn list_models(&self, profile_id: &str) -> Result<Vec<String>> {
        let profile = self.load_profile(profile_id).await?;
        let provider = provider_for(&profile.format_type)?;
        let credential = load_credential(&profile)?;
        let call = ProviderCall {
            http: &self.http,
            profile: &profile,
            credential: &credential,
        };
        provider.list_models(&call).await
    }

    /// 텍스트 생성.
    /// - `on_delta`: 스트리밍 조각마다 호출 (stream=false면 호출되지 않음)
    /// - `cancel`: 취소되면 진행 중인 HTTP 요청을 버리고 `AiRequestCancelled`로 끝남
    pub async fn generate<F>(&self, request: AiRequest, cancel: &CancellationToken, mut on_delta: F) -> Result<AiResponse>
    where
        F: FnMut(&str) + Send,
    {
        let profile = self.load_profile(&request.profile_id).await?;
        let provider = provider_for(&profile.format_type)?;
        let credential = load_credential(&profile)?;
        self.ensure_budget(&profile.id, UsageKind::Chat).await?;
        let call = ProviderCall {
            http: &self.http,
            profile: &profile,
            credential: &credential,
        };

        let started = std::time::Instant::now();
        // 스트림 중 받은 글 (취소·실패해도 그만큼은 요금이 나가므로 출력 토큰 추정에 쓴다)
        let mut streamed = String::new();
        let mut forward = |d: &str| {
            streamed.push_str(d);
            on_delta(d);
        };
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(cancelled()),
            r = provider.generate(&call, &request.prompt, &mut forward) => r,
        };

        let mut entry = UsageEntry::new(UsageKind::Chat, &profile.id, &profile.model);
        entry.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let output_text = result.as_ref().map_or(streamed.as_str(), |g| g.content.as_str());
        match result.as_ref().ok().and_then(|g| g.usage) {
            Some((i, o)) => (entry.input_tokens, entry.output_tokens) = (i, o),
            None => {
                entry.estimated = true;
                entry.input_tokens = estimate_prompt_tokens(&request.prompt);
                entry.output_tokens = crate::service::prompt::token::count_tokens(output_text) as u64;
            }
        }
        if let Err(e) = &result {
            entry.ok = false;
            entry.error = Some(e.to_response().message);
        }
        self.record_usage(entry).await;
        let generation = result?;

        Ok(AiResponse {
            content: generation.content,
            finish_reason: generation.finish_reason,
            profile_id: profile.id.clone(),
            model: profile.model.clone(),
        })
    }

    /// 취소/스트리밍이 필요 없는 단발 호출
    pub async fn generate_once(&self, mut request: AiRequest) -> Result<AiResponse> {
        request.prompt.chat_options.stream = false;
        self.generate(request, &CancellationToken::new(), |_| {}).await
    }
}

/// API 키는 프로필 id 기준으로 keyring에 저장된다 (commands::ai_service::set_api_key와 같은 규칙)
fn load_credential(profile: &ApiProfiles) -> Result<String> {
    SecretStore::get(&api_key_name(&profile.id)).map_err(|_| {
        AppError::ai_provider(
            ErrorCode::AiProviderAuthFailed,
            format!(
                "이 기기에 '{}' 프로필의 API 키가 없습니다. 설정 > API 키에서 넣어 주세요 (키는 기기마다 따로 저장되고 백업·동기화되지 않음)",
                profile.name
            ),
            false,
        )
    })
}

/// 프롬프트의 입력 토큰 추정 (PromptManager가 매긴 값이 없으면 다시 센다)
fn estimate_prompt_tokens(prompt: &PromptMessage) -> u64 {
    prompt
        .messages
        .iter()
        .map(|m| {
            if m.token > 0 {
                m.token as u64
            } else {
                crate::service::prompt::token::count_message_tokens(&m.content) as u64
            }
        })
        .sum()
}

fn cancelled() -> AppError {
    AppError::ai_provider(ErrorCode::AiRequestCancelled, "AI 요청이 취소되었습니다", false)
}
