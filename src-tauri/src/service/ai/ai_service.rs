use reqwest::Client;
use tokio_util::sync::CancellationToken;

use serde_json::Value;

use crate::{
    adapter::{
        capabilities::{capabilities, AiCapabilities, Feature},
        provider_for, ChatExtras, EmbedRequest, EmbedTask, Generation, ProviderCall, ProviderState, ResponseSchema, StopKind, ToolCall,
    },
    db::{
        self,
        conn::DbPool,
        usage::{UsageEntry, UsageKind},
    },
    error::{AppError, ErrorCode, Result},
    models::ai_service::{ApiProfiles, PromptMessage, PromptMessageItem, PromptRole},
    service::secret_store::{api_key_name, SecretStore},
};

/// 도구·구조화 출력 요청의 출력 한도 하한. 인자 JSON이 중간에 끊긴 채 "호출"이 되는 일을 줄인다
/// (Claude 기본 1024, 생각 토큰도 출력 한도를 먹는다). 상한을 올릴 뿐 실제 요금은 모델이 쓴 만큼이다
const MIN_EXT_TOKENS: u32 = 2048;
/// 임베딩 한 번에 받는 글 수 (OpenAI 한도). 더 많으면 호출하는 쪽이 나눠서 부른다 — 한 번에 큰 요금이 나가지 않게
pub const MAX_EMBED_INPUTS: usize = 2048;

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

/// 도구 호출이 든 응답 (3-A-2)
#[derive(Debug, Clone)]
pub struct AiExtResponse {
    pub response: AiResponse,
    pub tool_calls: Vec<ToolCall>,
    pub stop: StopKind,
    /// 도구 호출이 있으면, 다음 턴(`ToolTurn::Assistant.state`)에 그대로 돌려보낼 응답 원본
    pub state: Option<ProviderState>,
}

/// 구조화 출력 요청 (3-A-3)
#[derive(Debug, Clone)]
pub struct StructuredRequest {
    pub profile_id: String,
    pub prompt: PromptMessage,
    pub schema: ResponseSchema,
}

#[derive(Debug, Clone)]
pub struct StructuredResponse {
    /// 스키마를 만족하는 값
    pub value: Value,
    /// 모델이 낸 글 그대로
    pub raw: String,
    /// 공급자의 네이티브 기능을 썼나 (false = 지시 + 검증으로 대신함)
    pub native: bool,
    pub profile_id: String,
    pub model: String,
}

/// 임베딩 요청 (3-A-1)
#[derive(Debug, Clone)]
pub struct EmbedInput {
    pub profile_id: String,
    /// 임베딩 모델. 프로필의 채팅 모델과 다르다 — 호출하는 쪽이 정한다 (3-C ADR)
    pub model: String,
    pub inputs: Vec<String>,
    pub task: EmbedTask,
    /// 벡터 길이를 줄일 때 (지원하는 모델만)
    pub dimensions: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct EmbedOutput {
    /// 입력 순서대로, 모두 같은 길이
    pub vectors: Vec<Vec<f32>>,
    /// 입력 토큰 (공급자가 알려 준 값, 없으면 앱이 센 추정)
    pub input_tokens: u64,
    pub estimated: bool,
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

    /// 이 프로필(공급자 형식 + 주소 + 모델)이 임베딩·도구·구조화 출력을 지원하는지. 공급자를 부르지 않는다 (3-A-4)
    pub fn capabilities_of(profile: &ApiProfiles) -> AiCapabilities {
        capabilities(&profile.format_type, &profile.base_url, &profile.model)
    }

    /// 텍스트 생성.
    /// - `on_delta`: 스트리밍 조각마다 호출 (stream=false면 호출되지 않음)
    /// - `cancel`: 취소되면 진행 중인 HTTP 요청을 버리고 `AiRequestCancelled`로 끝남
    pub async fn generate<F>(&self, request: AiRequest, cancel: &CancellationToken, on_delta: F) -> Result<AiResponse>
    where
        F: FnMut(&str) + Send,
    {
        Ok(self.run(request, &ChatExtras::default(), cancel, on_delta).await?.0)
    }

    /// 도구 호출이 있는 생성 (3-A-2). 도구를 부르려고 멈추면 `tool_calls`에 담겨 온다 — 실행하고 결과를 `extras.tail`에 덧붙여 다시 부르는 것은 호출하는 쪽(5-B 루프)의 일이다.
    /// 호출 전에 요청 모양과 연결의 지원 여부를 확인한다(요금 없이 거절). 스트리밍이어도 글 조각만 `on_delta`로 가고 도구 호출은 끝난 뒤에 모아 돌려준다.
    /// 출력 한도에서 잘려 인자가 끊긴 호출은 실행하지 않고 `AiOutputTruncated`로 돌려준다
    pub async fn generate_ext<F>(&self, request: AiRequest, extras: ChatExtras, cancel: &CancellationToken, on_delta: F) -> Result<AiExtResponse>
    where
        F: FnMut(&str) + Send,
    {
        let (response, g) = self.run(request, &extras, cancel, on_delta).await?;
        Ok(AiExtResponse {
            response,
            tool_calls: g.tool_calls,
            stop: g.stop,
            state: g.state,
        })
    }

    /// 스키마에 맞는 JSON 값을 받는다 (3-A-3). 연결이 네이티브 기능을 "지원함"으로 확인된 경우에만 그것을 쓰고,
    /// 아니면 프롬프트 끝에 지시를 덧붙여 부른 뒤 결과를 검증한다. 어느 쪽이든 결과는 앱이 스키마로 다시 검사하고, 실패해도 다시 부르지 않는다
    pub async fn generate_structured(&self, request: StructuredRequest, cancel: &CancellationToken) -> Result<StructuredResponse> {
        let profile = self.load_profile(&request.profile_id).await?;
        let extras = ChatExtras {
            response_schema: Some(request.schema.clone()),
            ..Default::default()
        };
        extras.validate()?; // 쓸 수 없는 스키마는 호출 전에 거절
        let caps = Self::capabilities_of(&profile);
        caps.ensure(Feature::Structured, &profile.name)?;
        let native = caps.structured.is_supported();

        let mut prompt = request.prompt;
        prompt.chat_options.stream = false;
        prompt.chat_options.max_tokens = Some(prompt.chat_options.max_tokens.unwrap_or(0).max(MIN_EXT_TOKENS));
        let extras = if native {
            extras
        } else {
            let text = super::structured::instruction(&request.schema);
            prompt.messages.push(PromptMessageItem {
                role: PromptRole::System,
                token: crate::service::prompt::token::count_message_tokens(&text) as i64,
                content: text,
                images: vec![],
            });
            ChatExtras::default()
        };
        let (response, g) = self
            .run(
                AiRequest {
                    profile_id: profile.id.clone(),
                    prompt,
                },
                &extras,
                cancel,
                |_| {},
            )
            .await?;
        let value = super::structured::parse_output(&response.content, g.stop, &request.schema, native)?;
        Ok(StructuredResponse {
            value,
            raw: response.content,
            native,
            profile_id: response.profile_id,
            model: response.model,
        })
    }

    /// 임베딩 (3-A-1). 호출 전에 지원 여부·입력·예산 한도를 확인하고, 사용량(`embedding`, 입력 토큰)을 기록한다.
    /// 공급자가 토큰 수를 알려 주지 않으면 입력 글을 세어 추정으로 기록한다. 작업(TaskManager) 안에서 부르는 것은 호출하는 쪽의 몫이다
    pub async fn embed(&self, request: EmbedInput, cancel: &CancellationToken) -> Result<EmbedOutput> {
        let profile = self.load_profile(&request.profile_id).await?;
        let provider = provider_for(&profile.format_type)?;
        let bad = |m: &str| AppError::validation(m, None);
        if request.model.trim().is_empty() {
            return Err(bad("임베딩 모델 이름이 비어 있음"));
        }
        if request.inputs.is_empty() {
            return Err(bad("임베딩할 글이 없음"));
        }
        if request.inputs.len() > MAX_EMBED_INPUTS {
            return Err(bad(&format!("한 번에 {MAX_EMBED_INPUTS}개까지 임베딩할 수 있음 — 나눠서 부르세요")));
        }
        // 빈 글은 공급자가 거절한다 (요청 전체가 실패해 요금만 날 수 있다)
        if let Some(i) = request.inputs.iter().position(|t| t.trim().is_empty()) {
            return Err(bad(&format!("{}번째 글이 비어 있음 — 빈 글은 임베딩할 수 없음", i + 1)));
        }
        if request.dimensions == Some(0) {
            return Err(bad("벡터 길이는 1 이상이어야 함"));
        }
        Self::capabilities_of(&profile).ensure(Feature::Embeddings, &profile.name)?;
        let credential = load_credential(&profile)?;
        self.ensure_budget(&profile.id, UsageKind::Embedding).await?;
        let call = ProviderCall {
            http: &self.http,
            profile: &profile,
            credential: &credential,
        };
        let req = EmbedRequest {
            model: &request.model,
            inputs: &request.inputs,
            task: request.task,
            dimensions: request.dimensions,
        };

        let started = std::time::Instant::now();
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(cancelled()),
            r = provider.embed(&call, &req) => r,
        };
        let estimate = || {
            request
                .inputs
                .iter()
                .map(|t| crate::service::prompt::token::count_tokens(t) as u64)
                .sum::<u64>()
        };
        let mut entry = UsageEntry::new(UsageKind::Embedding, &profile.id, &request.model);
        entry.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        match result.as_ref().ok().and_then(|e| e.tokens) {
            Some(t) => entry.input_tokens = t,
            None => {
                entry.estimated = true;
                entry.input_tokens = estimate();
            }
        }
        if let Err(e) = &result {
            entry.ok = false;
            entry.error = Some(e.to_response().message);
        }
        let (tokens, estimated) = (entry.input_tokens, entry.estimated);
        self.record_usage(entry).await;
        let emb = result?;

        // 응답 점검: 개수와 길이가 맞아야 벡터를 믿고 저장할 수 있다
        let dim = emb.vectors.first().map_or(0, Vec::len);
        if emb.vectors.len() != request.inputs.len() || dim == 0 || emb.vectors.iter().any(|v| v.len() != dim) {
            return Err(crate::adapter::parse_err(
                "임베딩",
                format!("벡터 {}개(길이 {dim})를 받았는데 글은 {}개", emb.vectors.len(), request.inputs.len()),
            ));
        }
        Ok(EmbedOutput {
            vectors: emb.vectors,
            input_tokens: tokens,
            estimated,
            profile_id: profile.id,
            model: request.model,
        })
    }

    /// `generate`·`generate_ext`·`generate_structured`의 공통 몸통: 프로필·키·한도 확인 → 공급자 호출 → 사용량 기록
    async fn run<F>(&self, request: AiRequest, extras: &ChatExtras, cancel: &CancellationToken, mut on_delta: F) -> Result<(AiResponse, Generation)>
    where
        F: FnMut(&str) + Send,
    {
        let profile = self.load_profile(&request.profile_id).await?;
        let provider = provider_for(&profile.format_type)?;
        // 요금이 나가기 전에: 요청 모양과 연결의 지원 여부
        if !extras.is_empty() {
            extras.validate()?;
            let caps = Self::capabilities_of(&profile);
            if !extras.tools.is_empty() {
                caps.ensure(Feature::Tools, &profile.name)?;
            }
            if extras.response_schema.is_some() {
                caps.ensure(Feature::Structured, &profile.name)?;
            }
        }
        let credential = load_credential(&profile)?;
        self.ensure_budget(&profile.id, UsageKind::Chat).await?;
        let mut request = request;
        if !extras.tools.is_empty() || extras.response_schema.is_some() {
            let opts = &mut request.prompt.chat_options;
            opts.max_tokens = Some(opts.max_tokens.unwrap_or(0).max(MIN_EXT_TOKENS));
        }
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
            r = provider.generate_with(&call, &request.prompt, extras, &mut forward) => r,
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
        let mut generation = result?;
        // 출력 한도에서 잘려 인자가 끊긴 도구 호출은 실행하지 않는다 (이미 쓴 토큰은 위에서 기록됨)
        if generation.stop == StopKind::MaxTokens && generation.tool_calls.iter().any(|c| c.parse_error.is_some()) {
            return Err(super::structured::truncated("도구 호출"));
        }
        let response = AiResponse {
            content: std::mem::take(&mut generation.content),
            finish_reason: generation.finish_reason.clone(),
            profile_id: profile.id.clone(),
            model: profile.model.clone(),
        };
        Ok((response, generation))
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
