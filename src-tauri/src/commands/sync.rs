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
    /// 그 기기가 이 기기 내용을 어디까지 받았나 (이 기기 시각, 그 기기 묶음의 acks). 아직이면 없음
    pub their_ack: Option<i64>,
    /// 오래(STALE_PEER_MS) 못 본 기기. 지운 기록 정리가 이 기기를 기다리는 중이라 사용자가 제외할 수 있다
    pub stale: bool,
    /// 사용자가 제외함: 지운 기록 정리가 기다리지 않는다. 돌아와 합치면 저절로 풀린다
    pub excluded: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub device_id: String,
    pub peers: Vec<SyncPeer>,
    /// 아직 고르지 않은 충돌 (2-E에서 고른다)
    pub open_conflicts: i64,
    /// 다음 시작 때 되돌릴 합치기 전 사본 (2-E)
    pub pending_rollback: Option<String>,
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
    let now = chrono::Utc::now().timestamp_millis();
    let peers = sqlx::query_as::<_, (String, i64, i64, Option<i64>, bool)>(
        "SELECT device_id, last_merge_at, merges, their_ack, excluded FROM sync_peers ORDER BY last_merge_at DESC",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(device_id, last_merge_at, merges, their_ack, excluded)| SyncPeer {
        device_id,
        last_merge_at,
        merges,
        their_ack,
        stale: last_merge_at < now - crate::service::data::sync_run::STALE_PEER_MS,
        excluded,
    })
    .collect();
    let open_conflicts = crate::service::data::sync_conflicts::open_count(&state.db).await?;
    Ok(SyncStatus {
        device_id: me.id,
        peers,
        open_conflicts,
        pending_rollback: crate::service::data::sync_conflicts::pending_rollback(&app_data_dir(&app)?),
    })
}

/// 오래 못 본 기기를 제외하거나(지운 기록 정리가 기다리지 않음) 다시 포함한다. 제외한 기기가 돌아와 합치면 저절로 풀리고, 그때는 전체를 다시 합친다
#[tauri::command]
#[specta::specta]
pub async fn set_sync_peer_excluded(app: AppHandle, state: State<'_, AppState>, device_id: String, excluded: bool) -> Result<()> {
    require(&app, id::SYNC)?;
    let n = sqlx::query("UPDATE sync_peers SET excluded = ? WHERE device_id = ?")
        .bind(excluded)
        .bind(&device_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(AppError::not_found("모르는 기기"));
    }
    Ok(())
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
    let drive = setting.target == "drive";
    let folder = setting.folder.clone().unwrap_or_default();
    if drive {
        if !crate::service::drive::auth::available() {
            return Err(AppError::validation("이 빌드에는 구글 드라이브 연결이 들어 있지 않음", None));
        }
        if !crate::service::drive::auth::is_connected() {
            return Err(AppError::validation("구글 드라이브에 먼저 로그인하세요 (설정 > 동기화)", None));
        }
    } else if folder.is_empty() {
        return Err(AppError::validation("동기화 폴더를 먼저 고르세요 (설정 > 동기화)", None));
    }
    let target_kind = if drive { "drive" } else { "folder" };
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
    sqlx::query("INSERT INTO sync_runs (id, target, run_by, started_at) VALUES (?, ?, ?, ?)")
        .bind(&run_id)
        .bind(target_kind)
        .bind(if auto { "auto" } else { "user" })
        .bind(started)
        .execute(&state.db)
        .await?;

    let label = if drive { "구글 드라이브 동기화" } else { "폴더 동기화" };
    let job = CreateTaskRequest::inline(TaskType::BackgroundSync, TaskOwner::System, id::SYNC, label, auto);
    let progress = |step: &str| {
        let _ = app.emit("sync::progress", SyncProgress { step: step.to_string() });
    };
    let http = state.http_client.clone();
    let result = tasks
        .run_inline(job, |_| async {
            if drive {
                let client = drive_client()?;
                let target = crate::service::drive::target::DriveTarget::new(http, client, Default::default());
                sync_run::run(&state.db, &base, &mut settings, &me.id, &target, &progress).await
            } else {
                let target = FolderTarget::new(std::path::Path::new(&folder))?;
                sync_run::run(&state.db, &base, &mut settings, &me.id, &target, &progress).await
            }
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

// ---------------------------------------------------------------------------
// 둘 다 남긴 것 고르기 · 되돌리기 (2-E)
// ---------------------------------------------------------------------------

/// 아직 고르지 않은 충돌 (최근부터)
#[tauri::command]
#[specta::specta]
pub async fn list_sync_conflicts(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<crate::service::data::sync_conflicts::SyncConflict>> {
    require(&app, id::SYNC)?;
    crate::service::data::sync_conflicts::list(&state.db).await
}

/// 고르기: 이 기기 것 두기 / 받은 것으로 바꾸기 / 버리기(확인함)
#[tauri::command]
#[specta::specta]
pub async fn resolve_sync_conflict(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    choice: crate::service::data::sync_conflicts::ConflictChoice,
) -> Result<()> {
    require(&app, id::SYNC)?;
    crate::service::data::sync_conflicts::resolve(&state.db, &id, choice).await
}

/// 합치기 전 사본 (sync-backups, 최근부터)
#[tauri::command]
#[specta::specta]
pub fn list_sync_backups(app: AppHandle) -> Result<Vec<crate::service::data::sync_conflicts::SyncBackup>> {
    require(&app, id::SYNC)?;
    Ok(crate::service::data::sync_conflicts::list_backups(&app_data_dir(&app)?))
}

/// 되돌리기 준비 — 앱을 다시 시작하면 적용된다 (지금 DB·설정은 restore-backups/로 치움, 자동 동기화는 꺼짐)
#[tauri::command]
#[specta::specta]
pub fn request_sync_rollback(app: AppHandle, name: String) -> Result<()> {
    require(&app, id::SYNC)?;
    crate::service::data::sync_conflicts::request_rollback(&app_data_dir(&app)?, &name)
}

#[tauri::command]
#[specta::specta]
pub fn cancel_sync_rollback(app: AppHandle) -> Result<()> {
    crate::service::data::sync_conflicts::cancel_rollback(&app_data_dir(&app)?);
    Ok(())
}

// ---------------------------------------------------------------------------
// 구글 드라이브 (2-D, 데스크톱)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DriveStatus {
    /// 이 빌드에 OAuth 클라이언트가 들어 있나 (없으면 드라이브를 쓸 수 없음)
    pub available: bool,
    pub connected: bool,
    pub email: Option<String>,
}

fn save_sync_setting(app: &AppHandle, f: impl FnOnce(&mut crate::models::settings::SyncSetting)) -> Result<()> {
    let state = app.state::<SettingsState<PluginSetting>>();
    let value = {
        let mut cur = state.0.lock().map_err(|_| lock_err())?;
        f(&mut cur.sync);
        cur.clone()
    };
    save(app, &value)?;
    app.emit(PluginSetting::EVENT_NAME, &value).map_err(|e| AppError::io(e.to_string()))?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn drive_status(app: AppHandle) -> Result<DriveStatus> {
    require(&app, id::SYNC)?;
    let connected = crate::service::drive::auth::is_connected();
    Ok(DriveStatus {
        available: crate::service::drive::auth::available(),
        connected,
        email: if connected { sync_setting(&app)?.drive_email } else { None },
    })
}

/// 구글 로그인: 기본 브라우저를 열고 돌아올 때까지 기다린다(5분). 권한은 앱 전용 폴더만
#[tauri::command]
#[specta::specta]
pub async fn drive_login(app: AppHandle, state: State<'_, AppState>) -> Result<DriveStatus> {
    use crate::service::drive::{auth, target::DriveTarget};
    use tauri_plugin_opener::OpenerExt;
    require(&app, id::SYNC)?;
    let client = auth::built_in_client().ok_or_else(|| AppError::validation("이 빌드에는 구글 드라이브 연결이 들어 있지 않음", None))?;
    let ep = auth::Endpoints::default();
    auth::login(&state.http_client, &client, &ep, |url| {
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(|e| AppError::io(format!("브라우저를 열지 못함: {e}")))
    })
    .await?;
    // 연결 확인 겸 계정 이름 (드라이브 about — 앱 전용 폴더 권한으로 읽을 수 있음)
    let email = DriveTarget::new(state.http_client.clone(), client, ep).account_email().await.ok().flatten();
    save_sync_setting(&app, |s| {
        s.drive_email = email.clone();
        s.target = "drive".into();
    })?;
    drive_status(app)
}

/// 연결 끊기. `delete_remote`면 먼저 드라이브의 앱 전용 폴더를 비운다(되돌릴 수 없음 — 화면에서 확인)
#[tauri::command]
#[specta::specta]
pub async fn drive_logout(app: AppHandle, state: State<'_, AppState>, delete_remote: bool) -> Result<u32> {
    use crate::service::drive::{auth, target::DriveTarget};
    require(&app, id::SYNC)?;
    let ep = auth::Endpoints::default();
    let mut removed = 0;
    if delete_remote {
        removed = DriveTarget::new(state.http_client.clone(), drive_client()?, ep.clone()).delete_all().await?;
    }
    auth::disconnect(&state.http_client, &ep).await?;
    save_sync_setting(&app, |s| {
        s.drive_email = None;
        if s.target == "drive" {
            s.target = "folder".into();
            s.auto = false;
        }
    })?;
    Ok(removed)
}

/// 드라이브 요청에 쓸 클라이언트: 데스크톱은 빌드에 넣은 것, Android는 필요 없음(Play 서비스가 토큰을 줌)
fn drive_client() -> Result<crate::service::drive::auth::Client> {
    use crate::service::drive::auth;
    match auth::built_in_client() {
        Some(c) => Ok(c),
        None if auth::available() => Ok(auth::Client {
            id: String::new(),
            secret: String::new(),
        }),
        None => Err(AppError::validation("이 빌드에는 구글 드라이브 연결이 들어 있지 않음", None)),
    }
}

/// Android: 폰이 Play 서비스에서 받은 access token을 넣는다 (메모리에만). `connect`면 처음 연결 — 계정을 읽고 저장소를 드라이브로
#[tauri::command]
#[specta::specta]
pub async fn drive_use_token(app: AppHandle, state: State<'_, AppState>, token: String, connect: bool) -> Result<DriveStatus> {
    use crate::service::drive::{auth, target::DriveTarget};
    require(&app, id::SYNC)?;
    auth::use_android_token(&token)?;
    if connect {
        let email = DriveTarget::new(state.http_client.clone(), drive_client()?, auth::Endpoints::default())
            .account_email()
            .await
            .ok()
            .flatten();
        save_sync_setting(&app, |s| {
            s.drive_email = email.clone();
            s.target = "drive".into();
        })?;
    }
    drive_status(app)
}
