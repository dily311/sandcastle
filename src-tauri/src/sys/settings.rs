use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;

use crate::{
    error::{AppError, ErrorCode},
    models::settings::SettingsDomain,
};

const STORE_PATH: &str = "settings.json";
pub struct SettingsState<T>(pub Mutex<T>);

pub fn merge_json(base: &mut serde_json::Value, patch: &serde_json::Value) {
    if let (Some(b), Some(p)) = (base.as_object_mut(), patch.as_object()) {
        for (k, v) in p {
            b.insert(k.clone(), v.clone());
        }
    }
}

pub fn apply_patch<T: SettingsDomain>(
    app: &AppHandle,
    state: &Mutex<T>,
    patch: serde_json::Value,
) -> Result<T, AppError> {
    let mut guard = state
        .lock()
        .map_err(|_| AppError::archive(ErrorCode::InternalError, "archive lock poisoned"))?;
    let mut current = serde_json::to_value(&*guard).unwrap();

    // 이전 state와 통합
    merge_json(&mut current, &patch);
    // 역직렬화 및 state 반영
    *guard = serde_json::from_value(current).map_err(|e| AppError::io(e.to_string()))?;

    // 상태 반영
    save(app, &*guard)?;

    Ok(guard.clone())
}

/// 앱 시작시 데이터를 불러오는 데 사용
/// 앱이 돌아가는 동안 해당 함수를 부를 일은 없음
pub fn load<T: SettingsDomain>(app: &AppHandle) -> T {
    let store = app.store(STORE_PATH).expect("store 로드 실패");
    store
        .get(T::STORE_KEY)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn save<T: SettingsDomain>(app: &AppHandle, settings: &T) -> Result<(), AppError> {
    let store = app
        .store(STORE_PATH)
        .map_err(|e| AppError::internal(e.to_string()))?;
    store.set(T::STORE_KEY, serde_json::to_value(settings).unwrap());
    store.save().map_err(|e| AppError::io(e.to_string()))?;

    Ok(())
}
