use crate::db::ai_service;
use crate::models::ai_service::{ApiProfiles, ApiProfilesInput, ChatOptions, PromptMessage, PromptMessageItem, PromptRole};
use crate::service::ai::{AiRequest, AiService};
use crate::service::secret_store::{api_key_name, SecretStore};
use crate::{
    error::{AppError, ErrorCode, Result},
    state::AppState,
};
use std::sync::Arc;
use std::time::Duration;
use tauri::State;

/// 키 확인(모델 목록 조회)을 기다릴 시간. 넘으면 확인하지 못한 것으로 보고 키는 그대로 둔다.
const KEY_CHECK_TIMEOUT: Duration = Duration::from_secs(10);

/// 새 키를 저장하고 모델 목록을 불러 확인한다.
/// 공급자가 인증 실패(401/403)로 답하면 이전 키로 되돌리고 그 오류를 돌려준다.
/// 네트워크 오류·시간 초과·모델 목록을 지원하지 않는 공급자처럼 확인할 수 없는 경우에는 저장을 유지한다.
async fn store_verified_key(ai: &AiService, profile_id: &str, api_key: String) -> Result<()> {
    let name = api_key_name(profile_id);
    let previous = SecretStore::get(&name).ok();
    set_api_key(profile_id.to_string(), api_key)?;

    let rejected = match tokio::time::timeout(KEY_CHECK_TIMEOUT, ai.list_models(profile_id)).await {
        Ok(Err(e)) if e.to_response().code == ErrorCode::AiProviderAuthFailed => Some(e),
        _ => None,
    };
    if let Some(err) = rejected {
        match previous {
            Some(old) => SecretStore::set(&name, &old),
            None => SecretStore::delete(&name),
        }
        .map_err(|e| AppError::io(e.to_string()))?;
        return Err(err);
    }
    Ok(())
}

// =======================================
// API Key
// =======================================

#[tauri::command]
#[specta::specta]
pub fn set_api_key(profile_id: String, api_key: String) -> Result<()> {
    SecretStore::set(&api_key_name(&profile_id), &api_key).map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn delete_api_key(profile_id: String) -> Result<()> {
    SecretStore::delete(&api_key_name(&profile_id)).map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn has_api_key(profile_id: String) -> Result<bool> {
    Ok(SecretStore::exists(&api_key_name(&profile_id)))
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VertexKeyInfo {
    pub project_id: String,
    pub location: String,
    pub client_email: Option<String>,
}

/// Vertex AI 키의 공개 메타정보(프로젝트 ID·지역·서비스 계정 이메일)만 돌려준다 (비밀키는 절대 노출하지 않음)
#[tauri::command]
#[specta::specta]
pub fn get_vertex_key_info(profile_id: String) -> Result<Option<VertexKeyInfo>> {
    let Ok(raw) = SecretStore::get(&api_key_name(&profile_id)) else {
        return Ok(None);
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Ok(None);
    };
    let Some(project_id) = v.get("project_id").and_then(|x| x.as_str()).map(str::to_string) else {
        return Ok(None);
    };
    let location = v.get("location").and_then(|x| x.as_str()).unwrap_or("global").to_string();
    let client_email = v
        .get("service_account")
        .and_then(|x| x.as_str())
        .and_then(|sa| serde_json::from_str::<serde_json::Value>(sa).ok())
        .and_then(|sa| sa.get("client_email").and_then(|x| x.as_str()).map(str::to_string));
    Ok(Some(VertexKeyInfo {
        project_id,
        location,
        client_email,
    }))
}

// =======================================
// API Profile
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_all_api_profile(state: State<'_, AppState>) -> Result<Vec<ApiProfiles>> {
    let get_all_api_profile = ai_service::get_all_api_profile(&state.db).await?;
    let result = get_all_api_profile;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn get_api_profile(state: State<'_, AppState>, id: String) -> Result<ApiProfiles> {
    let result = ai_service::get_api_profile(&state.db, &id).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn create_api_profile(state: State<'_, AppState>, ai: State<'_, Arc<AiService>>, input: ApiProfilesInput, api_key: String) -> Result<ApiProfiles> {
    let api_profile = ai_service::create_api_profile(&state.db, input).await?;
    if !api_key.is_empty() {
        // 키가 거절되면 만든 프로필도 지운다 (화면은 실패로 알고 다시 저장하므로 중복이 생기지 않게).
        // 입력한 주소·모델은 화면에 그대로 남아 있다.
        if let Err(e) = store_verified_key(&ai, &api_profile.id, api_key).await {
            let _ = ai_service::delete_api_profile(&state.db, &api_profile.id).await;
            return Err(e);
        }
    }
    Ok(api_profile)
}

#[tauri::command]
#[specta::specta]
pub async fn update_api_profile(state: State<'_, AppState>, ai: State<'_, Arc<AiService>>, id: String, input: ApiProfilesInput, api_key: String) -> Result<()> {
    let prev = ai_service::get_api_profile(&state.db, &id).await?;
    let endpoint_changed = prev.base_url.trim() != input.base_url.trim() || prev.format_type != input.format_type;
    ai_service::update_api_profile(&state.db, &id, input).await?;
    if !api_key.is_empty() {
        store_verified_key(&ai, &id, api_key).await?;
        let _ = ai_service::set_verified(&state.db, &id, false).await;
    } else if endpoint_changed {
        let _ = ai_service::set_verified(&state.db, &id, false).await;
    }
    Ok(())
}

/// 직접 입력한 모델 이름을 이 프로필의 "내가 추가한 모델"로 남긴다. 바뀐 목록을 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn add_api_profile_model(state: State<'_, AppState>, id: String, model: String) -> Result<Vec<String>> {
    Ok(ai_service::add_custom_model(&state.db, &id, &model).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_api_profile_model(state: State<'_, AppState>, id: String, model: String) -> Result<Vec<String>> {
    Ok(ai_service::remove_custom_model(&state.db, &id, &model).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_api_profile(state: State<'_, AppState>, id: String) -> Result<()> {
    delete_api_key(id.clone())?;
    ai_service::delete_api_profile(&state.db, &id).await?;
    Ok(())
}

// =======================================
// AI 연결
// =======================================
/// 연결(형식·주소·모델)로 되는 것: 임베딩·도구 호출·구조화 출력. 요청을 보내지 않는 순수 계산이라 저장 전에도 부를 수 있다 (3-A-4)
#[tauri::command]
#[specta::specta]
pub fn get_ai_capabilities(format_type: String, base_url: String, model: String) -> crate::adapter::capabilities::AiCapabilities {
    crate::adapter::capabilities::capabilities(&format_type, &base_url, &model)
}

#[tauri::command]
#[specta::specta]
pub async fn get_models(ai: State<'_, Arc<AiService>>, profile_id: String) -> Result<Vec<String>> {
    ai.list_models(&profile_id).await
}

/// 공급자 계정의 잔여 (NovelAI·ElevenLabs. 요금 없음, 사용자가 누를 때만)
#[tauri::command]
#[specta::specta]
pub async fn get_provider_quota(ai: State<'_, Arc<AiService>>, profile_id: String) -> Result<Vec<crate::service::plugins::QuotaLine>> {
    let profile = ai.load_profile(&profile_id).await?;
    crate::service::plugins::provider_quota(&ai, &profile).await
}

/// 연결 확인: 짧은 메시지("Hi") 하나를 보내고 받은 답을 돌려준다 (저장하지 않음).
/// `model`을 주면 그 모델로 일시 테스트하고, 없으면 프로필 기본 모델 → 모델 목록 조회 순으로 확인한다.
#[tauri::command]
#[specta::specta]
pub async fn test_message(
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<crate::task::TaskManager>>,
    profile_id: String,
    model: Option<String>,
) -> Result<String> {
    let prompt = PromptMessage {
        chat_options: ChatOptions {
            max_tokens: Some(32),
            stream: false,
            ..ChatOptions::default()
        },
        messages: vec![PromptMessageItem {
            images: vec![],
            role: PromptRole::User,
            content: "Hi".into(),
            token: 4,
            source: None,
        }],
    };
    // 음성·이미지 API는 대화 요청을 받지 않으므로 요금이 없는 확인 요청을 쓴다
    let profile = ai.load_profile(&profile_id).await?;
    match profile.format_type.as_str() {
        "elevenlabs" => {
            let n = crate::service::plugins::voices(&ai, &profile_id).await?.len();
            let _ = ai_service::set_verified(&state.db, &profile_id, true).await;
            return Ok(format!("목소리 {n}개"));
        }
        "novelai" => {
            let out = crate::service::plugins::novelai_check(&ai, &profile).await?;
            let _ = ai_service::set_verified(&state.db, &profile_id, true).await;
            return Ok(out);
        }
        _ => {}
    }
    let chosen = model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string);
    // 테스트 모델도 없고 기본 모델도 없으면 모델 목록 조회로 연결을 확인한다 (토큰 비용 없음)
    if chosen.is_none() && profile.model.trim().is_empty() {
        let models = ai.list_models(&profile_id).await?;
        let _ = ai_service::set_verified(&state.db, &profile_id, true).await;
        return Ok(format!("모델 {}개 확인됨", models.len()));
    }
    use crate::task::{CreateTaskRequest, TaskOwner, TaskType};
    let job = CreateTaskRequest::inline(TaskType::ApiCall, TaskOwner::System, "settings", "API 연결 확인", false);
    let res = tasks
        .run_inline(job, |_| {
            ai.generate_once(AiRequest {
                profile_id,
                prompt,
                model: chosen,
            })
        })
        .await?;
    Ok(res.content)
}
