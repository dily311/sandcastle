//! 게임 패키지 관리 command (만들기/저장/지우기/가져오기/내보내기). 얇은 wrapper — 실제 로직은 game::manager.
//! 파일 경로(zip, 내보낼 폴더)는 호스트 UI가 파일 대화상자로 고른 값이라 호스트를 신뢰한다
//! (게임 iframe은 이 command를 못 부른다 — capabilities에서 호스트 창에만 허용할 것).
use crate::state::{AppState, GameArchiveHandle};
use crate::error::{AppError, ErrorCode};
use crate::game::manager::{GameManager, SaveReport};
use crate::game::package::PackageData;
use crate::game::runtime::{GameRepository, SqlxGameRepository};
use crate::models::game::GameProject;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

fn games_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_data_dir()?.join("games"))
}

#[tauri::command]
#[specta::specta]
pub async fn create_game(app: AppHandle, state: State<'_, AppState>, title: String, template_id: String) -> Result<GameProject, AppError> {
    let id = GameManager::create_game(&state.db, &games_root(&app)?, title, template_id).await?;
    Ok(crate::db::game::get_game_project(&state.db, &id).await?)
}

/// 앱에 포함된 템플릿 목록 (새 게임 만들기 화면용). capabilities로 어떤 기능을 쓰는 템플릿인지 알 수 있다.
#[tauri::command]
#[specta::specta]
pub fn list_templates() -> Vec<crate::game::template::Template> {
    crate::game::template::all()
}

/// 에디터 저장. 기존 행은 id를 달아 보내야 한다 (없는 id는 새 행). 응답의 deleted/issues를 사용자에게 보여줄 것.
#[tauri::command]
#[specta::specta]
pub async fn save_package(app: AppHandle, state: State<'_, AppState>, game_id: String, data: PackageData) -> Result<SaveReport, AppError> {
    GameManager::save_package(&state.db, &games_root(&app)?, &game_id, data).await
}

/// 게임 삭제. 플레이 기록(세션/세이브/일지/대화)도 같이 지워지므로 UI에서 반드시 확인을 받을 것.
/// 지금 플레이 중이거나 마운트된 게임은 지우지 않는다.
#[tauri::command]
#[specta::specta]
pub async fn delete_game(app: AppHandle, state: State<'_, AppState>, game_id: String) -> Result<(), AppError> {
    let mounted = state.active_archive.lock().map_err(|_| AppError::Archive { code: ErrorCode::ArchiveInvalid, message: "archive lock poisoned".into(), details: None })?.as_ref().map(|a| a.game_id.clone());
    if mounted.as_deref() == Some(game_id.as_str()) {
        return Err(AppError::game_invalid_state("지금 열려 있는 게임은 지울 수 없습니다"));
    }
    let playing = state.active_runtime.lock().await.as_ref().map(|r| r.definition().game_id.clone());
    if playing.as_deref() == Some(game_id.as_str()) {
        return Err(AppError::game_invalid_state("지금 플레이 중인 게임은 지울 수 없습니다"));
    }
    GameManager::delete_package(&state.db, &games_root(&app)?, &game_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn import_game(app: AppHandle, state: State<'_, AppState>, zip_path: String) -> Result<GameProject, AppError> {
    let id = GameManager::import_package(&state.db, &games_root(&app)?, &PathBuf::from(zip_path)).await?;
    Ok(crate::db::game::get_game_project(&state.db, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn export_game(app: AppHandle, state: State<'_, AppState>, game_id: String, output_dir: Option<String>) -> Result<String, AppError> {
    // 폴더를 안 주면 사용자의 다운로드 폴더에 저장한다. 다운로드 폴더를 못 찾는 환경(일부 모바일)이면
    // 앱 데이터 폴더의 exports로 대신한다.
    let dir = match output_dir {
        Some(d) => PathBuf::from(d),
        None => app.path().download_dir().unwrap_or(app.path().app_data_dir()?.join("exports")),
    };
    let path = GameManager::export_package(&state.db, &games_root(&app)?, &game_id, &dir).await?;
    Ok(path.to_string_lossy().into_owned())
}

/// open_game의 응답: 프로젝트 정보 + 화면에 그릴 정적 데이터(get_game_view와 같은 값) + 그 내용 해시 + 진단 결과.
#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenGameResponse {
    pub project: GameProject,
    pub view: serde_json::Value,
    /// view의 내용 해시. 프론트가 캐시해둔 정적 데이터가 아직 유효한지 비교한다 (같으면 그대로 쓰면 된다).
    pub content_hash: String,
    /// 에셋을 불러오는 기준 URL (끝에 `/`). view와 이벤트/무대 상태의 에셋 path는 게임 폴더 기준 상대경로이고,
    /// 프론트는 `assetBaseUrl + path`로 이미지/오디오를 불러온다 (경로의 각 조각은 URL 인코딩).
    pub asset_base_url: String,
    /// 데이터 진단 결과(같은 진단은 한 번만). Error가 하나라도 있으면 start_new_game이 시작을 막는다.
    pub diagnostics: Vec<crate::game::validate::Issue>,
}


/// 게임 열기: 이 게임을 현재 게임으로 지정하고, 정의를 **여기서 한 번** 메모리에 올린다.
/// 이후 start_new_game / continue_game / load_save는 이 정의를 그대로 쓴다 (다시 읽지 않는다).
/// 다른 게임을 플레이 중이었으면 그 런타임은 내린다 (game_id가 어긋난 채 커맨드가 가는 걸 막으려는 것).
/// 에디터에서 정의를 저장했다면 다시 open_game을 불러야 바뀐 내용이 반영된다.
#[tauri::command]
#[specta::specta]
pub async fn open_game(app: AppHandle, state: State<'_, AppState>, game_id: String) -> Result<OpenGameResponse, AppError> {
    let path = GameManager::load_package(&state.db, &games_root(&app)?, &game_id).await?;
    let project = crate::db::game::get_game_project(&state.db, &game_id).await?; // 없으면 NotFound

    let repo = SqlxGameRepository::new(&state.db);
    let definition = std::sync::Arc::new(repo.load_definition(&game_id).await.map_err(|e| AppError::database(e.to_string()))?);
    let diagnostics = crate::game::validate::diagnose(&definition, Some(path.as_path()));
    let (view, content_hash) = definition.view_with_hash();
    let asset_base_url = crate::game::protocol::base_url(&game_id);

    {
        let mut runtime = state.active_runtime.lock().await;
        if runtime.as_ref().map_or(false, |r| r.definition().game_id != game_id) {
            *runtime = None;
        }
    }
    // ⚠ GameArchiveHandle에 `definition: Arc<GameDefinition>` 필드가 있어야 한다 (manifest 자리에는 GameProject).
    *state.active_archive.lock().map_err(|_| AppError::internal("archive lock poisoned"))? =
        Some(GameArchiveHandle { path, manifest: project.clone(), game_id, definition });
    Ok(OpenGameResponse { project, view, content_hash, asset_base_url, diagnostics })
}

/// 게임 종료
#[tauri::command]
#[specta::specta]
pub async fn close_game(state: State<'_, AppState>) -> Result<(), AppError> {
    {
        let mut runtime = state.active_runtime.lock().await;
        *runtime = None;
    }
    *state.active_archive.lock().map_err(|_| AppError::internal("archive lock poisoned"))? = None;

    Ok(())
}

/// 게임 커버(배너) 그림 바꾸기: 고른 파일을 게임 폴더 cover/에 복사하고 cover_image로. None이면 지운다.
/// 앱이 cover/에 넣어 둔 예전 커버 파일은 지운다. 바뀐 게임을 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn set_game_cover(app: AppHandle, state: State<'_, AppState>, game_id: String, path: Option<String>) -> Result<GameProject, AppError> {
    if game_id.is_empty() || game_id.contains(['/', '\\', '.']) {
        return Err(AppError::validation("게임 id가 올바르지 않습니다", None));
    }
    let dir = games_root(&app)?.join(&game_id);
    if !dir.is_dir() {
        return Err(AppError::not_found("게임 폴더를 찾을 수 없습니다"));
    }
    let old: Option<String> = sqlx::query_scalar("SELECT cover_image FROM game_projects WHERE id = ?")
        .bind(&game_id)
        .fetch_optional(&state.db)
        .await?
        .flatten();
    let rel = match path {
        Some(src) => {
            let src = PathBuf::from(src);
            let ext = src.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
            if !["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.as_str()) {
                return Err(AppError::validation("png·jpg·webp·gif 그림만 커버로 쓸 수 있습니다", None));
            }
            std::fs::create_dir_all(dir.join("cover"))?;
            let name = format!("cover/{}.{ext}", uuid::Uuid::new_v4());
            std::fs::copy(&src, dir.join(&name))?;
            Some(name)
        }
        None => None,
    };
    sqlx::query("UPDATE game_projects SET cover_image = ?, updated_at = ? WHERE id = ?")
        .bind(&rel)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(&game_id)
        .execute(&state.db)
        .await?;
    if let Some(old) = old.filter(|o| o.starts_with("cover/") && Some(o) != rel.as_ref()) {
        let _ = std::fs::remove_file(dir.join(old));
    }
    crate::db::game::get_all_game_projects(&state.db)
        .await?
        .into_iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| AppError::not_found("게임을 찾을 수 없습니다"))
}

/// 이 앱의 게임 목록 (라이브러리 화면용).
#[tauri::command]
#[specta::specta]
pub async fn game_list(state: State<'_, AppState>) -> Result<Vec<GameProject>, AppError> {
    Ok(crate::db::game::get_all_game_projects(&state.db).await?)
}

/// 에디터용: 저장(save_package)이 받는 것과 같은 모양의 전체 데이터. 조건/효과/풀까지 다 들어 있다.
/// 호스트 에디터 전용이다 (게임 iframe이 부르면 안 된다).
#[tauri::command]
#[specta::specta]
pub async fn get_package_data(state: State<'_, AppState>, game_id: String) -> Result<PackageData, AppError> {
    let repo = SqlxGameRepository::new(&state.db);
    let definition = repo.load_definition(&game_id).await.map_err(|e| AppError::database(e.to_string()))?;
    Ok(crate::game::package::to_editor_data(&definition))
}
