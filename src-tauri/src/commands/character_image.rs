//! 채팅용 캐릭터 이미지 커맨드
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::{
    db,
    error::{AppError, Result},
    models::character_image::{CharacterImage, CharacterImageUpdate},
    service::chat::chat_images::{add_images, normalize_key, remove_image_files},
    state::AppState,
};

pub(crate) fn app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::io(format!("앱 데이터 폴더를 찾을 수 없습니다: {e}")))
}

#[tauri::command]
#[specta::specta]
pub async fn list_character_images(app: AppHandle, state: State<'_, AppState>, character_id: String) -> Result<Vec<CharacterImage>> {
    let base = app_data_dir(&app)?;
    let rows = db::character_image::get_character_images(&state.db, &character_id, false).await?;
    Ok(rows.into_iter().map(|r| CharacterImage::from_row(r, &base)).collect())
}

/// 사용자가 파일 대화상자로 고른 이미지들을 캐릭터에 추가.
/// 키는 파일 이름에서 만들고 겹치면 " 2", " 3"…을 붙인다.
/// (SillyTavern 표정 스프라이트 폴더의 파일들을 그대로 넘기면 감정 이름이 키가 된다)
#[tauri::command]
#[specta::specta]
pub async fn add_character_images(app: AppHandle, state: State<'_, AppState>, character_id: String, paths: Vec<String>) -> Result<Vec<CharacterImage>> {
    let base = app_data_dir(&app)?;
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    add_images(&state.db, &base, &character_id, &paths).await
}

/// ZIP으로 에셋 한꺼번에 넣기 (파일 이름이 키, 같은 키는 덮어씀)
#[tauri::command]
#[specta::specta]
pub async fn add_character_images_zip(app: AppHandle, state: State<'_, AppState>, character_id: String, zip_path: String) -> Result<Vec<CharacterImage>> {
    let base = app_data_dir(&app)?;
    let tmp = std::env::temp_dir().join(format!("sandcastle-zip-{}", uuid::Uuid::new_v4()));
    let zip = PathBuf::from(zip_path);
    let tmp2 = tmp.clone();
    let paths = tokio::task::spawn_blocking(move || crate::service::chat::chat_images::extract_zip_images(&zip, &tmp2))
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    let result = add_images(&state.db, &base, &character_id, &paths).await;
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn update_character_image(app: AppHandle, state: State<'_, AppState>, id: String, input: CharacterImageUpdate) -> Result<CharacterImage> {
    let current = db::character_image::get_character_image(&state.db, &id).await?;
    let key = normalize_key(&input.key)?;
    if db::character_image::image_key_exists(&state.db, &current.character_id, &key, Some(&id)).await? {
        return Err(AppError::validation(format!("이미 같은 키가 있습니다: {key}"), None));
    }
    db::character_image::update_character_image(&state.db, &id, &key, input.enabled, input.sort_order).await?;
    let row = db::character_image::get_character_image(&state.db, &id).await?;
    Ok(CharacterImage::from_row(row, &app_data_dir(&app)?))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_character_image(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    let row = db::character_image::get_character_image(&state.db, &id).await?;
    db::character_image::delete_character_image(&state.db, &id).await?;
    remove_image_files(&app_data_dir(&app)?, &[&row.file_path, row.thumb_path.as_deref().unwrap_or_default()]);
    Ok(())
}

/// 대화별 이미지 기능 켜기/끄기
#[tauri::command]
#[specta::specta]
pub async fn set_conversation_images_enabled(state: State<'_, AppState>, conversation_id: String, enabled: bool) -> Result<()> {
    if !db::conversation::set_conversation_images_enabled(&state.db, &conversation_id, enabled).await? {
        return Err(AppError::not_found(format!("대화를 찾을 수 없습니다: {conversation_id}")));
    }
    Ok(())
}
