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
    ai_service::update_api_profile(&state.db, &id, input).await?;
    if !api_key.is_empty() {
        store_verified_key(&ai, &id, api_key).await?;
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

/// 연결 확인: 짧은 메시지 하나를 보내고 받은 답을 돌려준다 (저장하지 않음)
#[tauri::command]
#[specta::specta]
pub async fn test_message(ai: State<'_, Arc<AiService>>, tasks: State<'_, Arc<crate::task::TaskManager>>, profile_id: String) -> Result<String> {
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
            return Ok(format!("목소리 {n}개"));
        }
        "novelai" => return crate::service::plugins::novelai_check(&ai, &profile).await,
        _ => {}
    }
    use crate::task::{CreateTaskRequest, TaskOwner, TaskType};
    let job = CreateTaskRequest::inline(TaskType::ApiCall, TaskOwner::System, "settings", "API 연결 확인", false);
    let res = tasks
        .run_inline(job, |_| {
            ai.generate_once(AiRequest {
                profile_id,
                prompt,
                model: None,
            })
        })
        .await?;
    Ok(res.content)
}
