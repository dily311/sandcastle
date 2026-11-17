//! 동기화 커맨드 (CLAUDE.md 2-B-3, ADR-011). 코어 플러그인 `sync`(기본 꺼짐)가 켜져 있을 때만.
//! 만들기·합치기는 사용자가 누를 때만 하고, 실행 기록에 `sync` 작업으로 남는다.
use std::{path::PathBuf, sync::Arc};

use serde::Serialize;
use serde_json::{Map, Value};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    commands::character_image::app_data_dir,
    error::{AppError, Result},
    models::settings::{AppSetting, FeatureSetting, PluginSetting, SettingsDomain, ThemeSetting},
    service::{
        data::sync_exchange::{self, SyncApplied, SyncBundleInfo, SyncPreview},
        device,
    },
    state::AppState,
    sys::{
        plugins::{id, require},
        settings::{save, SettingsState},
    },
    task::{
        task_models::{CreateTaskRequest, TaskOwner, TaskType},
        TaskManager,
    },
};

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncPeer {
    pub device_id: String,
    /// 이 기기 시각
    pub last_merge_at: i64,
    pub merges: i64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub device_id: String,
    pub peers: Vec<SyncPeer>,
    /// 아직 고르지 않은 충돌 (2-E에서 고른다)
    pub open_conflicts: i64,
}

fn lock_err() -> AppError {
    AppError::internal("설정 잠금 실패")
}

fn get<T: SettingsDomain>(app: &AppHandle) -> Result<Value> {
    let s = app.state::<SettingsState<T>>();
    let v = serde_json::to_value(&*s.0.lock().map_err(|_| lock_err())?).map_err(|e| AppError::internal(e.to_string()))?;
    Ok(v)
}

/// settings.json 전체 ({"app-setting": …}) — 같이 쓰는 칸을 고르는 것은 settings_scope가 한다
fn settings_map(app: &AppHandle) -> Result<Map<String, Value>> {
    let mut m = Map::new();
    m.insert(AppSetting::STORE_KEY.into(), get::<AppSetting>(app)?);
    m.insert(ThemeSetting::STORE_KEY.into(), get::<ThemeSetting>(app)?);
    m.insert(FeatureSetting::STORE_KEY.into(), get::<FeatureSetting>(app)?);
    m.insert(PluginSetting::STORE_KEY.into(), get::<PluginSetting>(app)?);
    Ok(m)
}

/// 바뀐 설정만 저장하고 화면에 알린다 (기능 켜기는 기기 전용이라 바뀌지 않음)
fn put<T: SettingsDomain>(app: &AppHandle, before: &Map<String, Value>, after: &Map<String, Value>) -> Result<()> {
    let (Some(new), old) = (after.get(T::STORE_KEY), before.get(T::STORE_KEY)) else {
        return Ok(());
    };
    if Some(new) == old {
        return Ok(());
    }
    let value: T = serde_json::from_value(new.clone()).map_err(|e| AppError::validation(format!("받은 설정을 읽지 못함: {e}"), None))?;
    let state = app.state::<SettingsState<T>>();
    *state.0.lock().map_err(|_| lock_err())? = value.clone();
    save(app, &value)?;
    app.emit(T::EVENT_NAME, &value).map_err(|e| AppError::io(e.to_string()))?;
    Ok(())
}

fn job(label: &str) -> CreateTaskRequest {
    CreateTaskRequest::inline(TaskType::BackgroundSync, TaskOwner::System, id::SYNC, label, false)
}

/// 이 기기 id, 합친 적 있는 기기, 남은 충돌 수. 동기화 화면을 열 때 (기기 id는 이때 처음 만든다)
#[tauri::command]
#[specta::specta]
pub async fn get_sync_status(app: AppHandle, state: State<'_, AppState>) -> Result<SyncStatus> {
    require(&app, id::SYNC)?;
    let me = device::get_or_create(&app_data_dir(&app)?)?;
    let peers = sqlx::query_as::<_, (String, i64, i64)>("SELECT device_id, last_merge_at, merges FROM sync_peers ORDER BY last_merge_at DESC")
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(|(device_id, last_merge_at, merges)| SyncPeer {
            device_id,
            last_merge_at,
            merges,
        })
        .collect();
    let open_conflicts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_conflicts WHERE resolved_at IS NULL")
        .fetch_one(&state.db)
        .await?;
    Ok(SyncStatus {
        device_id: me.id,
        peers,
        open_conflicts,
    })
}

/// 동기화 파일 만들기. `path`는 저장 대화상자에서 고른 경로 (.scsync)
#[tauri::command]
#[specta::specta]
pub async fn export_sync_bundle(
    app: AppHandle,
    state: State<'_, AppState>,
    tasks: State<'_, Arc<TaskManager>>,
    path: String,
    include_files: bool,
) -> Result<SyncBundleInfo> {
    require(&app, id::SYNC)?;
    let base = app_data_dir(&app)?;
    let me = device::get_or_create(&base)?;
    let settings = settings_map(&app)?;
    let out = PathBuf::from(path);
    tasks
        .run_inline(job("동기화 파일 만들기"), |_| {
            sync_exchange::export(&state.db, &base, &settings, &me.id, include_files, &out)
        })
        .await
}

/// 합치기 전에 숫자만 (DB·파일·설정을 바꾸지 않음)
#[tauri::command]
#[specta::specta]
pub async fn preview_sync_bundle(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<SyncPreview> {
    require(&app, id::SYNC)?;
    let base = app_data_dir(&app)?;
    let me = device::get_or_create(&base)?;
    sync_exchange::preview(&state.db, &base, &settings_map(&app)?, &me.id, &PathBuf::from(path)).await
}

/// 합치기: 이 기기 DB·설정 사본을 먼저 만들고, 없는 파일을 넣고, DB를 합치고, 같이 쓰는 설정을 바꾼다
#[tauri::command]
#[specta::specta]
pub async fn apply_sync_bundle(app: AppHandle, state: State<'_, AppState>, tasks: State<'_, Arc<TaskManager>>, path: String) -> Result<SyncApplied> {
    require(&app, id::SYNC)?;
    let base = app_data_dir(&app)?;
    let me = device::get_or_create(&base)?;
    let before = settings_map(&app)?;
    let mut settings = before.clone();
    let p = PathBuf::from(path);
    let applied = tasks
        .run_inline(job("동기화 파일 합치기"), |_| {
            sync_exchange::apply(&state.db, &base, &mut settings, &me.id, &p)
        })
        .await?;
    put::<AppSetting>(&app, &before, &settings)?;
    put::<ThemeSetting>(&app, &before, &settings)?;
    put::<PluginSetting>(&app, &before, &settings)?;
    Ok(applied)
}
