//! 게임 패키지 관리 command (만들기/저장/지우기/가져오기/내보내기). 얇은 wrapper — 실제 로직은 game::manager.
//! 파일 경로(zip, 내보낼 폴더)는 호스트 UI가 파일 대화상자로 고른 값이라 호스트를 신뢰한다
//! (게임 iframe은 이 command를 못 부른다 — capabilities에서 호스트 창에만 허용할 것).
use crate::state::{AppState, GameArchiveHandle};
use crate::error::AppError;
use crate::game::manager::{GameManager, SaveReport};
use crate::game::package::PackageData;
use crate::game::runtime::{GameRepository, SqlxGameRepository};
use crate::models::game::GameManifest; // ⚠ GameManifest 실제 모듈 경로 확인 (appstate.rs의 import와 같아야 함)
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

fn games_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_data_dir()?.join("games"))
}

#[tauri::command]
pub async fn create_game(app: AppHandle, state: State<'_, AppState>, title: String, template_id: String) -> Result<String, AppError> {
    GameManager::create_game(&state.db, &games_root(&app)?, title, template_id).await
}

/// 에디터 저장. 기존 행은 id를 달아 보내야 한다 (없는 id는 새 행). 응답의 deleted/issues를 사용자에게 보여줄 것.
#[tauri::command]
pub async fn save_package(state: State<'_, AppState>, game_id: String, data: PackageData) -> Result<SaveReport, AppError> {
    GameManager::save_package(&state.db, &game_id, data).await
}

/// 게임 삭제. 플레이 기록(세션/세이브/일지/대화)도 같이 지워지므로 UI에서 반드시 확인을 받을 것.
/// 지금 플레이 중이거나 마운트된 게임은 지우지 않는다.
#[tauri::command]
pub async fn delete_game(app: AppHandle, state: State<'_, AppState>, game_id: String) -> Result<(), AppError> {
    let mounted = state.active_archive.lock().map_err(|_| AppError::Archive("archive lock poisoned".into()))?.as_ref().map(|a| a.game_id.clone());
    if mounted.as_deref() == Some(game_id.as_str()) {
        return Err(AppError::Validation("지금 열려 있는 게임은 지울 수 없습니다".into()));
    }
    let playing = state.active_runtime.lock().await.as_ref().map(|r| r.definition().game_id.clone());
    if playing.as_deref() == Some(game_id.as_str()) {
        return Err(AppError::Validation("지금 플레이 중인 게임은 지울 수 없습니다".into()));
    }
    GameManager::delete_package(&state.db, &games_root(&app)?, &game_id).await
}

#[tauri::command]
pub async fn import_game(app: AppHandle, state: State<'_, AppState>, zip_path: String) -> Result<String, AppError> {
    GameManager::import_package(&state.db, &games_root(&app)?, &PathBuf::from(zip_path)).await
}

#[tauri::command]
pub async fn export_game(app: AppHandle, state: State<'_, AppState>, game_id: String, output_dir: String) -> Result<String, AppError> {
    let path = GameManager::export_package(&state.db, &games_root(&app)?, &game_id, &PathBuf::from(output_dir)).await?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn game_list(state: State<'_, AppState> ) -> Result<Vec<GameManifest>, AppError> {
    let project = crate::db::game::get_all_game_projects(&state.db).await?; // 없으면 NotFound

  Ok(project.into_iter().map(Into::into).collect())
}

/// 게임 열기(마운트): 이 게임을 "현재 게임"으로 지정한다. game:// 가 이 폴더를 서빙하고, 이후 커맨드들이
/// 이 game_id 범위로 일한다. 데이터는 여기서 안 읽는다 — 정의 전체는 플레이를 시작할 때 한 번에 올라온다.
/// 다른 게임이 플레이 중이었으면 그 런타임은 내린다 (game_id가 어긋난 채 커맨드가 가는 걸 막으려는 것).
#[tauri::command]
pub async fn open_game(app: AppHandle, state: State<'_, AppState>, game_id: String) -> Result<GameManifest, AppError> {
    let path = GameManager::load_package(&state.db, &games_root(&app)?, &game_id).await?;
    let project = crate::db::game::get_game_project(&state.db, &game_id).await?; // 없으면 NotFound

    let parse = |s: &Option<String>| s.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let manifest = GameManifest {
        id: project.id.clone(),
        title: project.title.clone(),
        description: project.description.clone(),
        version: project.version.clone(),
        template_id: project.template_id.clone(),
        api_version: project.api_version.clone(),
        start_location_id: project.start_location_id.clone(),
        cover_image: project.cover_image.clone(),
        custom_css: project.custom_css.as_ref().map(|s| serde_json::Value::String(s.clone())),
        settings_json: parse(&project.settings_json),
        created_at: project.created_at,
        updated_at: project.updated_at,
    };

    {
        let mut runtime = state.active_runtime.lock().await;
        if runtime.as_ref().map_or(false, |r| r.definition().game_id != game_id) {
            *runtime = None;
        }
    }
    *state.active_archive.lock().map_err(|_| AppError::Archive("archive lock poisoned".into()))? =
        Some(GameArchiveHandle { path, manifest: manifest.clone(), game_id });
    Ok(manifest)
}

#[tauri::command]
pub async fn close_game(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    {
        let mut runtime = state.active_runtime.lock().await;
        *runtime = None;
    }

    Ok(())
}

/// 에디터용: 저장(save_package)이 받는 것과 같은 모양의 전체 데이터. 조건/효과/풀까지 다 들어 있다.
/// 호스트 에디터 전용이다 (게임 iframe이 부르면 안 된다).
#[tauri::command]
pub async fn get_package_data(state: State<'_, AppState>, game_id: String) -> Result<PackageData, AppError> {
    let repo = SqlxGameRepository::new(&state.db);
    let definition = repo.load_definition(&game_id).await.map_err(|e| AppError::Database(e.to_string()))?;
    Ok(crate::game::package::to_editor_data(&definition))
}
