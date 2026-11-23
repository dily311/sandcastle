//! 캐릭터 팩 (.scpack): 고른 캐릭터와 그 채팅을 통째로 옮기기
//!
//! ```text
//! manifest.json          형식/버전/개수
//! data.json              캐릭터(+이미지 목록, 정규식), 로어북(+항목), 대화(+메시지)
//! files/avatars/<캐릭터id>.<확장자>
//! files/images/<캐릭터id>/<이미지id>.webp
//! ```
//!
//! 가져올 때는 모든 id를 새로 발급해서 기존 데이터와 섞여도 덮어쓰지 않는다(병합).
//! 대화의 페르소나가 이 앱에 없으면 기본 페르소나로 붙인다.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    db::{self, conn::DbPool},
    error::{AppError, ErrorCode, Result},
    models::{
        character_image::CharacterImageRow,
        conversation::{CharacterInput, ConversationInput, MessageInput, WorldInfoBookInput, WorldInfoEntryInput},
        regex::{RegexScope, RegexScript, RegexScriptInput},
    },
    service::chat::chat_images::{remove_character_dir, store_image_file},
};

use super::archive::{extract_file, open_archive, read_json, safe_relative, write_file, write_json, zip_err};

pub const PACK_FORMAT: &str = "sandcastle-pack";
pub const PACK_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackManifest {
    pub format: String,
    pub version: u32,
    pub app_version: String,
    pub created_at: i64,
    pub character_count: usize,
    pub conversation_count: usize,
    pub message_count: usize,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct PackData {
    characters: Vec<PackCharacter>,
    world_info_books: Vec<PackBook>,
    conversations: Vec<PackConversation>,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct PackCharacter {
    /// Character JSON 그대로 (id 포함)
    character: Value,
    avatar_file: Option<String>,
    images: Vec<PackImage>,
    regex: Vec<RegexScript>,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct PackImage {
    key: String,
    enabled: bool,
    sort_order: i64,
    file: String,
    /// 판 폴더 (예전 팩에는 없음 = 공통)
    #[serde(default)]
    folder: String,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct PackBook {
    id: String,
    name: String,
    description: Option<String>,
    entries: Vec<PackEntry>,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct PackEntry {
    keys: Vec<String>,
    secondary_keys: Vec<String>,
    content: String,
    enabled: bool,
    priority: i64,
    constant: bool,
    /// 제목·분류 (예전 팩에는 없음)
    #[serde(default)]
    comment: String,
    #[serde(default)]
    category: String,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct PackConversation {
    /// Conversation JSON 그대로
    conversation: Value,
    /// Message JSON 목록 (createdAt 포함)
    messages: Vec<Value>,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackExportSummary {
    pub path: String,
    pub bytes: u64,
    pub character_count: usize,
    pub conversation_count: usize,
    pub message_count: usize,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCharacterRef {
    pub old_id: String,
    pub new_id: String,
    pub name: String,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackImportSummary {
    pub characters: Vec<ImportedCharacterRef>,
    pub conversation_count: usize,
    pub message_count: usize,
    pub warnings: Vec<String>,
}

fn ext_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .filter(|e| !e.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "png".into())
}

// =======================================
// 내보내기
// =======================================

/// 캐릭터들과 그 대화를 팩으로 내보낸다.
/// `conversation_ids`가 있으면 그 대화만 (고른 캐릭터의 대화여야 함), 없으면 고른 캐릭터의 모든 대화.
pub async fn export_pack(
    db: &DbPool,
    app_data_dir: &Path,
    character_ids: &[String],
    conversation_ids: Option<&[String]>,
    out: &Path,
) -> Result<PackExportSummary> {
    if character_ids.is_empty() {
        return Err(AppError::validation("내보낼 캐릭터를 골라 주세요", None));
    }
    // (zip 이름, 원본 파일)
    let mut files: Vec<(String, PathBuf)> = vec![];
    let mut characters = vec![];
    let mut book_ids: Vec<String> = vec![];

    for cid in character_ids {
        let character = db::conversation::get_character(db, cid)
            .await
            .map_err(|_| AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {cid}")))?;
        for b in &character.world_info_ids {
            if !book_ids.contains(b) {
                book_ids.push(b.clone());
            }
        }
        let avatar_file = character.avatar.as_deref().map(PathBuf::from).filter(|p| p.is_file()).map(|p| {
            let name = format!("files/avatars/{cid}.{}", ext_of(&p));
            files.push((name.clone(), p));
            name
        });
        let images = db::character_image::get_character_images(db, cid, false)
            .await?
            .into_iter()
            .filter_map(|r| {
                let src = app_data_dir.join(&r.file_path);
                src.is_file().then(|| {
                    let name = format!("files/images/{cid}/{}.webp", r.id);
                    files.push((name.clone(), src));
                    PackImage {
                        key: r.key,
                        enabled: r.enabled,
                        sort_order: r.sort_order,
                        file: name,
                        folder: r.folder.clone(),
                    }
                })
            })
            .collect();
        let regex = db::regex::get_regex_scripts_for(db, Some(cid), None)
            .await?
            .into_iter()
            .filter(|s| s.scope == RegexScope::Character)
            .collect();
        // 변수 정의 (5-F-4-3)는 캐릭터 칸이 아니라 따로 읽어 캐릭터 JSON에 싣는다 (예전 팩에는 없음)
        let mut character_json = serde_json::to_value(&character)?;
        let var_defs = db::conversation::get_character_var_defs(db, cid).await.unwrap_or_default();
        if !var_defs.is_empty() {
            character_json["varDefs"] = serde_json::to_value(&var_defs)?;
        }
        characters.push(PackCharacter {
            character: character_json,
            avatar_file,
            images,
            regex,
        });
    }

    let mut books = vec![];
    for id in &book_ids {
        let Ok(book) = db::conversation::get_worldinfo_book(db, id).await else {
            continue;
        };
        let entries = db::conversation::get_all_worldinfo_entries(db, id)
            .await?
            .into_iter()
            .map(|e| PackEntry {
                comment: e.comment,
                category: e.category,
                keys: e.keys,
                secondary_keys: e.secondary_keys,
                content: e.content,
                enabled: e.enabled,
                priority: e.priority,
                constant: e.constant,
            })
            .collect();
        books.push(PackBook {
            id: book.id,
            name: book.name,
            description: book.description,
            entries,
        });
    }

    let summaries = super::overview::conversation_summaries(db).await?;
    let mut conv_ids: Vec<String> = summaries
        .iter()
        .filter(|c| c.character_id.as_ref().is_some_and(|id| character_ids.contains(id)))
        .map(|c| c.id.clone())
        .collect();
    if let Some(only) = conversation_ids {
        if let Some(bad) = only.iter().find(|id| !conv_ids.contains(id)) {
            return Err(AppError::validation(format!("고른 캐릭터의 대화가 아닙니다: {bad}"), None));
        }
        conv_ids.retain(|id| only.contains(id));
    }

    let mut conversations = vec![];
    let mut message_count = 0;
    for id in &conv_ids {
        let conv = db::conversation::get_conversation(db, id).await?;
        let messages = db::conversation::get_messages(db, id, i64::MAX, None).await?;
        message_count += messages.len();
        conversations.push(PackConversation {
            conversation: serde_json::to_value(&conv)?,
            messages: messages.iter().map(serde_json::to_value).collect::<std::result::Result<_, _>>()?,
        });
    }

    let manifest = PackManifest {
        format: PACK_FORMAT.into(),
        version: PACK_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created_at: chrono::Utc::now().timestamp_millis(),
        character_count: characters.len(),
        conversation_count: conversations.len(),
        message_count,
    };
    let data = PackData {
        characters,
        world_info_books: books,
        conversations,
    };

    let out_path = out.to_path_buf();
    let summary_manifest = manifest.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(&out_path)?;
        let mut zip = zip::ZipWriter::new(file);
        write_json(&mut zip, "manifest.json", &manifest)?;
        write_json(&mut zip, "data.json", &data)?;
        for (name, src) in &files {
            write_file(&mut zip, name, src)?;
        }
        zip.finish().map_err(zip_err)?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;

    Ok(PackExportSummary {
        bytes: std::fs::metadata(out).map(|m| m.len()).unwrap_or(0),
        path: out.to_string_lossy().to_string(),
        character_count: summary_manifest.character_count,
        conversation_count: summary_manifest.conversation_count,
        message_count,
    })
}

// =======================================
// 가져오기
// =======================================

pub fn check_manifest(m: &PackManifest) -> Result<()> {
    if m.format != PACK_FORMAT {
        return Err(AppError::archive(ErrorCode::ArchiveInvalid, "Sandcastle 캐릭터 팩이 아닙니다"));
    }
    if m.version > PACK_VERSION {
        return Err(AppError::archive(
            ErrorCode::ArchiveInvalid,
            "더 새 버전의 앱에서 만든 팩입니다. 앱을 업데이트해 주세요",
        ));
    }
    Ok(())
}

pub async fn import_pack(db: &DbPool, app_data_dir: &Path, path: &Path) -> Result<PackImportSummary> {
    let p = path.to_path_buf();
    let (manifest, data): (PackManifest, PackData) = tokio::task::spawn_blocking(move || -> Result<_> {
        let mut a = open_archive(&p)?;
        Ok((read_json(&mut a, "manifest.json")?, read_json(&mut a, "data.json")?))
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    check_manifest(&manifest)?;

    // 실패하면 지울 것들
    let mut created_dirs: Vec<String> = vec![];
    let mut created_files: Vec<PathBuf> = vec![];
    let tmp = tempdir_in(app_data_dir)?;
    let result = import_pack_inner(db, app_data_dir, path, &tmp, data, &mut created_dirs, &mut created_files).await;
    let _ = std::fs::remove_dir_all(&tmp);
    if result.is_err() {
        for f in &created_files {
            let _ = std::fs::remove_file(f);
        }
        for cid in &created_dirs {
            remove_character_dir(app_data_dir, cid);
        }
    }
    result
}

async fn import_pack_inner(
    db: &DbPool,
    app_data_dir: &Path,
    pack_path: &Path,
    tmp: &Path,
    data: PackData,
    created_dirs: &mut Vec<String>,
    created_files: &mut Vec<PathBuf>,
) -> Result<PackImportSummary> {
    let mut warnings = vec![];
    let mut archive = open_archive(pack_path)?;
    let extract = |name: &str, archive: &mut zip::ZipArchive<std::fs::File>| -> Result<PathBuf> {
        let out = tmp.join(safe_relative(name)?);
        extract_file(archive, name, &out)?;
        Ok(out)
    };

    let mut tx = db.begin().await?;

    // 로어북
    let mut book_map: HashMap<String, String> = HashMap::new();
    for b in data.world_info_books {
        let book = db::conversation::create_worldinfo_book(
            &mut *tx,
            WorldInfoBookInput {
                id: None,
                name: b.name,
                description: b.description,
            },
        )
        .await?;
        for e in b.entries {
            db::conversation::upsert_worldinfo_entry(
                &mut *tx,
                WorldInfoEntryInput {
                    comment: e.comment,
                    category: e.category,
                    id: None,
                    book_id: book.id.clone(),
                    keys: e.keys,
                    secondary_keys: e.secondary_keys,
                    content: e.content,
                    enabled: e.enabled,
                    priority: e.priority,
                    constant: e.constant,
                },
            )
            .await?;
        }
        book_map.insert(b.id, book.id);
    }

    // 캐릭터
    let mut char_map: HashMap<String, String> = HashMap::new();
    let mut refs = vec![];
    for pc in data.characters {
        let old_id = pc.character["id"].as_str().unwrap_or_default().to_string();
        let favorite = pc.character["favorite"].as_bool().unwrap_or(false);
        // 프로필 그림 확대·위치 (5-E-17-3, 예전 팩에는 없음)
        let avatar_fit = serde_json::from_value::<crate::models::conversation::AvatarFit>(pc.character["avatarFit"].clone()).ok();
        // 변수 정의 (5-F-4-3, 예전 팩에는 없음). 확인을 거쳐 넣고, 읽을 수 없으면 경고만
        let var_defs = match pc.character.get("varDefs") {
            Some(v) => serde_json::from_value::<Vec<crate::models::conversation::VarDef>>(v.clone())
                .map_err(|e| e.to_string())
                .and_then(|d| crate::service::chat::vars::normalize(d).map_err(|e| e.to_response().message)),
            None => Ok(vec![]),
        };
        let mut input: CharacterInput =
            serde_json::from_value(pc.character).map_err(|e| AppError::validation(format!("캐릭터 데이터를 읽을 수 없습니다: {e}"), None))?;

        input.avatar = match &pc.avatar_file {
            Some(name) => {
                let src = extract(name, &mut archive)?;
                let dir = app_data_dir.join("characters").join("avatars");
                std::fs::create_dir_all(&dir)?;
                let dest = dir.join(format!("{}.{}", Uuid::new_v4(), ext_of(&src)));
                std::fs::copy(&src, &dest)?;
                created_files.push(dest.clone());
                Some(dest.to_string_lossy().to_string())
            }
            // 다른 PC의 파일 경로는 쓸 수 없다. 경로가 아닌 값(data URL 등)만 유지
            None => input.avatar.filter(|a| !looks_like_path(a)),
        };
        let old_books = std::mem::take(&mut input.world_info_ids);
        let character = db::conversation::create_character(&mut *tx, input).await?;
        let new_id = character.id.clone();
        if favorite {
            db::conversation::update_character_is_favorite(&mut *tx, &new_id, true).await?;
        }
        if character.avatar.is_some() && avatar_fit.is_some() {
            db::conversation::set_avatar_fit(&mut *tx, "characters", &new_id, avatar_fit).await?;
        }
        match var_defs {
            Ok(defs) if !defs.is_empty() => {
                db::conversation::upsert_character_var_defs(&mut *tx, &new_id, &defs).await?;
            }
            Ok(_) => {}
            Err(why) => warnings.push(format!("'{}'의 변수 정의를 읽지 못해 뺌: {why}", character.name)),
        }
        for b in old_books {
            match book_map.get(&b) {
                Some(nb) => db::conversation::create_character_worldinfo_ids(&mut *tx, &new_id, nb).await?,
                None => warnings.push(format!("'{}'의 로어북 하나가 팩에 없어 연결하지 못함", character.name)),
            }
        }

        if !pc.images.is_empty() {
            created_dirs.push(new_id.clone());
        }
        for img in pc.images {
            let src = extract(&img.file, &mut archive)?;
            let image_id = Uuid::new_v4().to_string();
            let (file_path, thumb_path) = {
                let (base, cid, iid) = (app_data_dir.to_path_buf(), new_id.clone(), image_id.clone());
                tokio::task::spawn_blocking(move || store_image_file(&base, &cid, &iid, &src))
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))??
            };
            let now = chrono::Utc::now().timestamp_millis();
            db::character_image::insert_character_image(
                &mut *tx,
                &CharacterImageRow {
                    id: image_id,
                    character_id: new_id.clone(),
                    key: img.key,
                    file_path,
                    thumb_path: Some(thumb_path),
                    enabled: img.enabled,
                    sort_order: img.sort_order,
                    created_at: now,
                    updated_at: now,
                    folder: img.folder,
                },
            )
            .await?;
        }

        for s in pc.regex {
            let input = RegexScriptInput {
                id: None,
                name: s.name,
                find_regex: s.find_regex,
                replace_string: s.replace_string,
                trim_strings: s.trim_strings,
                placement: s.placement,
                target: s.target,
                scope: RegexScope::Character,
                scope_id: Some(new_id.clone()),
                substitute_macros: s.substitute_macros,
                run_on_edit: s.run_on_edit,
                min_depth: s.min_depth,
                max_depth: s.max_depth,
                enabled: s.enabled,
                sort_order: s.sort_order,
                source_format: Some(s.source_format),
                extra_json: s.extra_json,
            };
            db::regex::upsert_regex_script(&mut *tx, &input).await?;
        }

        char_map.insert(old_id.clone(), new_id.clone());
        refs.push(ImportedCharacterRef {
            old_id,
            new_id,
            name: character.name,
        });
    }

    // 대화
    let default_persona = db::conversation::get_system_default_persona(&mut *tx)
        .await
        .unwrap_or_else(|_| "default".into());
    let mut conversation_count = 0;
    let mut message_count = 0;
    for pc in data.conversations {
        let mut input: ConversationInput =
            serde_json::from_value(pc.conversation).map_err(|e| AppError::validation(format!("대화 데이터를 읽을 수 없습니다: {e}"), None))?;
        let Some(new_char) = char_map.get(&input.character_id).cloned() else {
            warnings.push(format!("캐릭터가 팩에 없는 대화 '{}'를 건너뜀", input.title));
            continue;
        };
        let old_char = std::mem::replace(&mut input.character_id, new_char.clone());
        if !persona_exists(&mut tx, &input.persona_id).await? {
            input.persona_id = default_persona.clone();
        }
        let dropped = input.world_info_ids.len();
        input.world_info_ids = input.world_info_ids.iter().filter_map(|b| book_map.get(b).cloned()).collect();
        if input.world_info_ids.len() < dropped {
            warnings.push(format!("대화 '{}'에 붙은 로어북 일부는 팩에 없어 빠짐", input.title));
        }
        let conv = db::conversation::create_conversation(&mut *tx, input).await?;

        for (i, mv) in pc.messages.into_iter().enumerate() {
            let created_at = mv["createdAt"].as_i64().unwrap_or(conv.created_at + i as i64);
            let mut m: MessageInput = serde_json::from_value(mv).map_err(|e| AppError::validation(format!("메시지 데이터를 읽을 수 없습니다: {e}"), None))?;
            m.conversation_id = conv.id.clone();
            m.character_id = m.character_id.and_then(|c| char_map.get(&c).cloned());
            if let Some(sender) = m.meta.as_mut().and_then(|v| v.get_mut("sender")).and_then(Value::as_object_mut) {
                if sender.get("id").and_then(Value::as_str) == Some(old_char.as_str()) {
                    sender.insert("id".into(), Value::String(new_char.clone()));
                }
            }
            db::conversation::insert_message_at(&mut *tx, m, created_at).await?;
            message_count += 1;
        }
        conversation_count += 1;
    }

    tx.commit().await?;
    Ok(PackImportSummary {
        characters: refs,
        conversation_count,
        message_count,
        warnings,
    })
}

async fn persona_exists(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: &str) -> Result<bool> {
    Ok(db::conversation::persona_exists(&mut **tx, id).await?)
}

/// 파일 경로처럼 보이는 문자열 (data:/http: URL이 아닌 것)
fn looks_like_path(s: &str) -> bool {
    let s = s.trim();
    !(s.is_empty() || s.starts_with("data:") || s.starts_with("http://") || s.starts_with("https://"))
}

/// 앱 데이터 폴더 아래 임시 폴더 (같은 드라이브라 복사가 빠르고, 끝나면 지운다)
fn tempdir_in(base: &Path) -> Result<PathBuf> {
    let dir = base.join("tmp").join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
