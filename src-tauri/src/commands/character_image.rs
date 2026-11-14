//! 채팅용 캐릭터 이미지 커맨드
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::{
    db,
    error::{AppError, Result},
    models::character_image::{CharacterImage, CharacterImageRow, CharacterImageUpdate},
    service::chat::chat_images::{key_from_path, normalize_key, remove_image_files, store_image_file},
    state::AppState,
};

pub(crate) fn app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::io(format!("앱 데이터 폴더를 찾을 수 없습니다: {e}")))
}

#[tauri::command]
pub async fn list_character_images(
    app: AppHandle,
    state: State<'_, AppState>,
    character_id: String,
) -> Result<Vec<CharacterImage>> {
    let base = app_data_dir(&app)?;
    let rows = db::character_image::get_character_images(&state.db, &character_id, false).await?;
    Ok(rows.into_iter().map(|r| CharacterImage::from_row(r, &base)).collect())
}

/// 사용자가 파일 대화상자로 고른 이미지들을 캐릭터에 추가.
/// 키는 파일 이름에서 만들고, 겹치면 " 2", " 3"…을 붙인다. 하나라도 실패하면 그 파일에서 멈추고
/// 이미 추가된 것은 남긴다(오류 메시지에 실패한 파일 표시).
#[tauri::command]
pub async fn add_character_images(
    app: AppHandle,
    state: State<'_, AppState>,
    character_id: String,
    paths: Vec<String>,
) -> Result<Vec<CharacterImage>> {
    if !db::conversation::char_exists(&state.db, &character_id).await? {
        return Err(AppError::not_found("존재하지 않는 캐릭터입니다"));
    }
    let base = app_data_dir(&app)?;
    let mut added = vec![];

    for path in paths {
        let source = PathBuf::from(&path);
        let id = Uuid::new_v4().to_string();

        let (file_path, thumb_path) = {
            let (base, cid, id, source) = (base.clone(), character_id.clone(), id.clone(), source.clone());
            tauri::async_runtime::spawn_blocking(move || store_image_file(&base, &cid, &id, &source))
                .await
                .map_err(|e| AppError::internal(e.to_string()))??
        };

        let key = unique_key(&state, &character_id, &key_from_path(&source)).await?;
        let now = chrono::Utc::now().timestamp_millis();
        let row = CharacterImageRow {
            id: id.clone(),
            character_id: character_id.clone(),
            key,
            file_path,
            thumb_path: Some(thumb_path),
            enabled: true,
            sort_order: db::character_image::next_image_sort_order(&state.db, &character_id).await?,
            created_at: now,
            updated_at: now,
        };
        if let Err(e) = db::character_image::insert_character_image(&state.db, &row).await {
            remove_image_files(&base, &[&row.file_path, row.thumb_path.as_deref().unwrap_or_default()]);
            return Err(e.into());
        }
        added.push(CharacterImage::from_row(row, &base));
    }
    Ok(added)
}

#[tauri::command]
pub async fn update_character_image(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: CharacterImageUpdate,
) -> Result<CharacterImage> {
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
pub async fn delete_character_image(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    let row = db::character_image::get_character_image(&state.db, &id).await?;
    db::character_image::delete_character_image(&state.db, &id).await?;
    remove_image_files(
        &app_data_dir(&app)?,
        &[&row.file_path, row.thumb_path.as_deref().unwrap_or_default()],
    );
    Ok(())
}

/// 대화별 이미지 기능 켜기/끄기
#[tauri::command]
pub async fn set_conversation_images_enabled(
    state: State<'_, AppState>,
    conversation_id: String,
    enabled: bool,
) -> Result<()> {
    if !db::conversation::set_conversation_images_enabled(&state.db, &conversation_id, enabled).await? {
        return Err(AppError::not_found(format!("대화를 찾을 수 없습니다: {conversation_id}")));
    }
    Ok(())
}

/// 파일 이름 기반 키가 겹치면 " 2", " 3"… 을 붙인다
async fn unique_key(state: &State<'_, AppState>, character_id: &str, base_key: &str) -> Result<String> {
    let base_key = normalize_key(base_key).unwrap_or_else(|_| "image".to_string());
    let mut key = base_key.clone();
    let mut n = 2;
    while db::character_image::image_key_exists(&state.db, character_id, &key, None).await? {
        key = format!("{base_key} {n}");
        n += 1;
    }
    Ok(key)
}
