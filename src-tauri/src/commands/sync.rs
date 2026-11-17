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

// ---------------------------------------------------------------------------
// 폴더 동기화 (2-C)
// ---------------------------------------------------------------------------

/// 동기화가 도는 중인가 (지금 동기화와 자동 동기화가 겹치지 않게)
static RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct RunGuard;
impl Drop for RunGuard {
    fn drop(&mut self) {
        RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// 진행 단계 (화면에 "다른 기기 확인 / 기기 … 합치는 중 / 올리는 중")
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub step: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncRun {
    pub id: String,
    pub target: String,
    pub run_by: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub ok: bool,
    pub error: Option<String>,
    pub summary: Option<crate::service::data::sync_run::SyncRunReport>,
}

fn sync_setting(app: &AppHandle) -> Result<crate::models::settings::SyncSetting> {
    Ok(app.state::<SettingsState<PluginSetting>>().0.lock().map_err(|_| lock_err())?.sync.clone())
}

/// 지금 동기화 (폴더). `auto`는 자동 동기화가 부를 때 (설정 > 동기화의 자동 동기화가 켜져 있어야 함)
#[tauri::command]
#[specta::specta]
pub async fn sync_now(
    app: AppHandle,
    state: State<'_, AppState>,
    tasks: State<'_, Arc<TaskManager>>,
    auto: bool,
) -> Result<crate::service::data::sync_run::SyncRunReport> {
    use crate::service::data::{sync_run, sync_target::FolderTarget};
    require(&app, id::SYNC)?;
    let setting = sync_setting(&app)?;
    if auto && !setting.auto {
        return Err(AppError::validation("자동 동기화가 꺼져 있습니다", None));
    }
    let folder = setting
        .folder
        .ok_or_else(|| AppError::validation("동기화 폴더를 먼저 고르세요 (설정 > 동기화)", None))?;
    if RUNNING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err(AppError::validation("동기화가 이미 진행 중입니다", None));
    }
    let _guard = RunGuard;

    let base = app_data_dir(&app)?;
    let me = device::get_or_create(&base)?;
    let before = settings_map(&app)?;
    let mut settings = before.clone();
    let run_id = uuid::Uuid::new_v4().to_string();
    let started = chrono::Utc::now().timestamp_millis();
    sqlx::query("INSERT INTO sync_runs (id, target, run_by, started_at) VALUES (?, 'folder', ?, ?)")
        .bind(&run_id)
        .bind(if auto { "auto" } else { "user" })
        .bind(started)
        .execute(&state.db)
        .await?;

    let job = CreateTaskRequest::inline(TaskType::BackgroundSync, TaskOwner::System, id::SYNC, "폴더 동기화", auto);
    let progress = |step: &str| {
        let _ = app.emit("sync::progress", SyncProgress { step: step.to_string() });
    };
    let result = tasks
        .run_inline(job, |_| async {
            let target = FolderTarget::new(std::path::Path::new(&folder))?;
            sync_run::run(&state.db, &base, &mut settings, &me.id, &target, &progress).await
        })
        .await;

    let finished = chrono::Utc::now().timestamp_millis();
    match &result {
        Ok(r) => {
            sqlx::query("UPDATE sync_runs SET finished_at = ?, ok = 1, summary = ? WHERE id = ?")
                .bind(finished)
                .bind(serde_json::to_string(r).unwrap_or_default())
                .bind(&run_id)
                .execute(&state.db)
                .await?;
            put::<AppSetting>(&app, &before, &settings)?;
            put::<ThemeSetting>(&app, &before, &settings)?;
            put::<PluginSetting>(&app, &before, &settings)?;
        }
        Err(e) => {
            sqlx::query("UPDATE sync_runs SET finished_at = ?, ok = 0, error = ? WHERE id = ?")
                .bind(finished)
                .bind(e.to_string())
                .bind(&run_id)
                .execute(&state.db)
                .await?;
        }
    }
    // 오래된 기록은 100개만 남긴다
    sqlx::query("DELETE FROM sync_runs WHERE id NOT IN (SELECT id FROM sync_runs ORDER BY started_at DESC LIMIT 100)")
        .execute(&state.db)
        .await?;
    result
}

/// id, target, run_by, started_at, finished_at, ok, error, summary
type SyncRunRow = (String, String, String, i64, Option<i64>, bool, Option<String>, Option<String>);

/// 동기화 기록 (최근부터)
#[tauri::command]
#[specta::specta]
pub async fn list_sync_runs(app: AppHandle, state: State<'_, AppState>, limit: u32) -> Result<Vec<SyncRun>> {
    require(&app, id::SYNC)?;
    let rows: Vec<SyncRunRow> =
        sqlx::query_as("SELECT id, target, run_by, started_at, finished_at, ok, error, summary FROM sync_runs ORDER BY started_at DESC LIMIT ?")
            .bind(limit.min(100))
            .fetch_all(&state.db)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, target, run_by, started_at, finished_at, ok, error, summary)| SyncRun {
            id,
            target,
            run_by,
            started_at,
            finished_at,
            ok,
            error,
            summary: summary.and_then(|s| serde_json::from_str(&s).ok()),
        })
        .collect())
}
