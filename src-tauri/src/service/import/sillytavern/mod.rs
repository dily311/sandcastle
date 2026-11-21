//! SillyTavern 가져오기.
//!
//! 파일 하나를 받아 종류를 판별하고, 각 형식의 변환기(순수 함수)로 바꾼 뒤 한 트랜잭션에 저장한다.
//!
//! | 파일                                   | 결과                                         |
//! |---------------------------------------|---------------------------------------------|
//! | 캐릭터 카드 (.png / .json, V1~V3)        | 캐릭터 + 아바타 + 로어북(연결) + 캐릭터 정규식      |
//! | Chat Completion 프리셋 (.json)          | 프롬프트 프리셋 + 블록 + 프리셋 정규식              |
//! | 정규식 스크립트 (.json, 하나 또는 배열)      | 전역 정규식                                    |
//! | 월드인포 (.json)                        | 로어북                                        |
//! | 채팅 기록 (.jsonl)                       | 대화 + 메시지 (스와이프·숨김·생성 정보 포함)        |
//!
//! 지원하지 않는 필드는 버리지 않고 원본(compatibility_json / extra_json)에 남기거나 경고로 알린다.
pub mod card;
pub mod chat;
pub mod preset;
pub mod regex;
pub mod worldinfo;

use std::{collections::HashMap, path::Path};

use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    db::{self, conn::DbPool},
    error::{AppError, Result},
    models::{
        conversation::{Character, Conversation, ConversationInput, MessageInput, WorldInfoBook, WorldInfoBookInput},
        regex::{RegexScope, RegexScript, RegexScriptInput},
    },
    service::chat::regex_processor::validate_script,
};

/// 가져올 파일 최대 크기
const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SillyTavernKind {
    Character,
    Preset,
    Regex,
    WorldInfo,
    Chat,
}

/// 가져오기 옵션
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// 프리셋/로어북/대화 이름 (없으면 파일 이름)
    pub name: Option<String>,
    /// 채팅을 붙일 캐릭터. 없으면 메시지의 캐릭터 이름으로 찾는다.
    pub character_id: Option<String>,
    /// 정규식 파일을 이 프리셋에 묶어서 가져온다 (프리셋과 함께 받은 정규식). 없으면 전역.
    pub preset_id: Option<String>,
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SillyTavernImport {
    #[serde(rename_all = "camelCase")]
    Character {
        character: Character,
        world_info_book_id: Option<String>,
        regex_count: usize,
        warnings: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Preset {
        preset_id: String,
        name: String,
        block_count: usize,
        regex_count: usize,
        warnings: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Regex { scripts: Vec<RegexScript>, warnings: Vec<String> },
    #[serde(rename_all = "camelCase")]
    WorldInfo {
        book: WorldInfoBook,
        entry_count: usize,
        warnings: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Chat {
        conversation: Conversation,
        message_count: usize,
        hidden_count: usize,
        warnings: Vec<String>,
    },
}

/// 파일 내용으로 종류 판별
pub fn detect(bytes: &[u8]) -> Result<(SillyTavernKind, Value)> {
    if card::is_png(bytes) {
        return Ok((SillyTavernKind::Character, card::card_json_from_png(bytes)?));
    }
    // .jsonl 채팅은 파일 전체가 JSON 하나가 아니므로 먼저 본다 (내용은 import_chat에서 다시 읽음)
    if chat::looks_like_chat(bytes) {
        return Ok((SillyTavernKind::Chat, Value::Null));
    }
    let json: Value = serde_json::from_slice(bytes).map_err(|e| AppError::validation(format!("JSON 파일을 읽을 수 없습니다: {e}"), None))?;
    let has = |k: &str| json.get(k).is_some();
    let kind = if as_str(&json, "spec").is_some_and(|s| s.starts_with("chara_card"))
        || (has("name") && (has("first_mes") || has("mes_example") || has("personality")))
    {
        SillyTavernKind::Character
    } else if has("prompts") || has("prompt_order") {
        SillyTavernKind::Preset
    } else if has("findRegex") || json.as_array().is_some_and(|a| a.iter().any(|v| v.get("findRegex").is_some())) {
        SillyTavernKind::Regex
    } else if has("entries") {
        SillyTavernKind::WorldInfo
    } else {
        return Err(AppError::validation(
            "SillyTavern 캐릭터 카드·프리셋·정규식·월드인포 중 어느 것도 아닙니다",
            None,
        ));
    };
    Ok((kind, json))
}

/// 파일을 읽어 가져온다. `name`은 프리셋/로어북 이름 (없으면 파일 이름)
pub async fn import_file(db: &DbPool, app_data_dir: &Path, path: &Path, name: Option<String>) -> Result<SillyTavernImport> {
    import_file_with(db, app_data_dir, path, ImportOptions { name, ..Default::default() }).await
}

/// 가져오지 않고 종류만 본다 (여러 파일을 함께 가져올 때 순서를 정하려고: 프리셋 먼저, 그다음 정규식을 그 프리셋에)
pub fn detect_file(path: &Path) -> Result<SillyTavernKind> {
    let meta = std::fs::metadata(path).map_err(|e| AppError::io(format!("파일을 열 수 없습니다: {e}")))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(AppError::validation("파일이 너무 큽니다 (50MB 이하)", None));
    }
    Ok(detect(&std::fs::read(path)?)?.0)
}

pub async fn import_file_with(db: &DbPool, app_data_dir: &Path, path: &Path, options: ImportOptions) -> Result<SillyTavernImport> {
    let name = options.name;
    let meta = std::fs::metadata(path).map_err(|e| AppError::io(format!("파일을 열 수 없습니다: {e}")))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(AppError::validation("파일이 너무 큽니다 (50MB 이하)", None));
    }
    let bytes = std::fs::read(path)?;
    let (kind, json) = detect(&bytes)?;
    let file_name = name
        .filter(|s| !s.trim().is_empty())
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "가져온 항목".into());

    match kind {
        SillyTavernKind::Character => {
            let avatar_bytes = card::is_png(&bytes).then_some(bytes.as_slice());
            import_character(db, app_data_dir, &json, avatar_bytes).await
        }
        SillyTavernKind::Preset => import_preset(db, &json, &file_name).await,
        SillyTavernKind::Regex => import_regex(db, &json, options.preset_id).await,
        SillyTavernKind::WorldInfo => import_worldinfo(db, &json, &file_name).await,
        SillyTavernKind::Chat => import_chat(db, &bytes, &file_name, options.character_id).await,
    }
}

async fn import_chat(db: &DbPool, bytes: &[u8], title: &str, character_id: Option<String>) -> Result<SillyTavernImport> {
    let (header, raw) = chat::parse_jsonl(bytes)?;
    if raw.is_empty() {
        return Err(AppError::validation("채팅에 메시지가 없습니다", None));
    }
    let mut warnings = vec![];
    let messages = chat::map_messages(&raw, &mut warnings);

    // 말한 캐릭터 이름 (많이 말한 순)
    let mut counts: Vec<(String, usize)> = vec![];
    for m in messages.iter().filter(|m| m.role == "assistant" && !m.name.is_empty()) {
        match counts.iter_mut().find(|(n, _)| *n == m.name) {
            Some((_, c)) => *c += 1,
            None => counts.push((m.name.clone(), 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let speakers: Vec<String> = counts.into_iter().map(|(n, _)| n).collect();

    // 이름 → 캐릭터 id (동명이인이 없을 때만)
    let mut by_name: HashMap<String, Option<String>> = HashMap::new();
    for s in &speakers {
        let ids = db::conversation::find_character_ids_by_name(db, s).await?;
        by_name.insert(s.clone(), (ids.len() == 1).then(|| ids[0].clone()));
    }

    let main_character = match character_id.filter(|s| !s.is_empty()) {
        Some(id) => {
            if !db::conversation::char_exists(db, &id).await? {
                return Err(AppError::not_found("존재하지 않는 캐릭터입니다"));
            }
            id
        }
        None => speakers
            .iter()
            .find_map(|s| by_name.get(s).cloned().flatten())
            .ok_or_else(|| AppError::Validation {
                code: crate::error::ErrorCode::InvalidInput,
                message: "어떤 캐릭터의 채팅인지 찾지 못했습니다. 캐릭터를 먼저 가져오거나 직접 골라 주세요".into(),
                details: Some(serde_json::json!({ "speakers": speakers })),
            })?,
    };
    // 1:1 채팅이면 이름이 달라도 그 캐릭터의 말로 본다
    if speakers.len() == 1 {
        by_name.insert(speakers[0].clone(), Some(main_character.clone()));
    }
    let unmatched: Vec<&str> = speakers
        .iter()
        .filter(|s| by_name.get(*s).cloned().flatten().is_none())
        .map(String::as_str)
        .collect();
    if !unmatched.is_empty() {
        warnings.push(format!("캐릭터를 찾지 못한 화자: {} (이름만 표시됨)", unmatched.join(", ")));
    }

    // 페르소나: 사용자 이름과 같은 페르소나가 하나 있으면 그것, 아니면 기본 페르소나
    let user_name = messages.iter().find(|m| m.is_user()).map(|m| m.name.clone()).unwrap_or_default();
    let persona_ids = db::conversation::find_persona_ids_by_name(db, &user_name).await?;
    let persona_id = if persona_ids.len() == 1 {
        persona_ids[0].clone()
    } else {
        db::conversation::get_system_default_persona(db).await.unwrap_or_else(|_| "default".into())
    };

    // 작가 노트 (Author's Note): 글·깊이는 대화의 작가 노트로. 빈도·위치(대화 밖)는 없어서 "대화 안 깊이"로 가져온다
    let author_note = chat::author_note(&header["chat_metadata"], &mut warnings);

    let start = chrono::Utc::now().timestamp_millis() - messages.len() as i64;
    let times = chat::monotonic_times(&messages, start);
    let hidden_count = messages.iter().filter(|m| m.hidden).count();
    let message_count = messages.len();

    let mut tx = db.begin().await?;
    let conversation = db::conversation::create_conversation(
        &mut *tx,
        ConversationInput {
            title: title.to_string(),
            character_id: main_character.clone(),
            persona_id: persona_id.clone(),
            world_info_ids: vec![],
            mode: "chat".into(),
            meta: Some({
                let mut meta = serde_json::json!({
                "importedFrom": "sillytavern",
                "sillytavern": {
                    "chatMetadata": header["chat_metadata"],
                    "userName": header["user_name"],
                    "characterName": header["character_name"],
                }
                });
                if let Some(n) = author_note {
                    meta["authorNote"] = n;
                }
                meta
            }),
            images_enabled: None,
        },
    )
    .await?;

    for (i, (m, at)) in messages.into_iter().zip(times).enumerate() {
        let character_id = match m.role.as_str() {
            "assistant" => by_name.get(&m.name).cloned().flatten(),
            _ => None,
        };
        let mut meta = m.meta;
        let sender_id = if m.role == "user" { Some(persona_id.clone()) } else { character_id.clone() };
        if let (Some(id), Some(sender)) = (sender_id, meta.get_mut("sender").and_then(Value::as_object_mut)) {
            sender.insert("id".into(), Value::String(id));
        }
        db::conversation::insert_message_at(
            &mut *tx,
            MessageInput {
                conversation_id: conversation.id.clone(),
                seq: i as i64 + 1,
                role: m.role,
                name: Some(m.name).filter(|n| !n.is_empty()),
                content: m.content,
                character_id,
                meta: Some(meta),
            },
            at,
        )
        .await?;
    }
    tx.commit().await?;

    let conversation = db::conversation::get_conversation(db, &conversation.id).await?;
    Ok(SillyTavernImport::Chat {
        conversation,
        message_count,
        hidden_count,
        warnings,
    })
}

async fn import_character(db: &DbPool, app_data_dir: &Path, card_json: &Value, avatar_png: Option<&[u8]>) -> Result<SillyTavernImport> {
    let mut imported = card::map_card(card_json)?;

    // 아바타는 DB 저장 전에 파일로 (실패하면 경고만)
    let mut avatar_file = None;
    if let Some(png) = avatar_png {
        let dir = app_data_dir.join("characters").join("avatars");
        match crate::sys::image::save_image_to_origin(dir, &Uuid::new_v4().to_string(), png) {
            Ok(path) => avatar_file = Some(path),
            Err(e) => imported.warnings.push(format!("아바타 이미지를 저장하지 못함: {e}")),
        }
    }
    imported.character.avatar = avatar_file.clone();

    let result: Result<_> = async {
        let mut tx = db.begin().await?;
        let character = db::conversation::create_character(&mut *tx, imported.character).await?;

        let mut book_id = None;
        if let Some((book, entries)) = imported.book {
            let book = db::conversation::create_worldinfo_book(&mut *tx, book).await?;
            for mut e in entries {
                e.book_id = book.id.clone();
                db::conversation::upsert_worldinfo_entry(&mut *tx, e).await?;
            }
            db::conversation::create_character_worldinfo_ids(&mut *tx, &character.id, &book.id).await?;
            book_id = Some(book.id);
        }

        let mut regex = imported.regex;
        for r in regex.iter_mut() {
            r.scope_id = Some(character.id.clone());
        }
        let saved = save_regex(&mut tx, regex, &mut imported.warnings).await?;
        tx.commit().await?;
        Ok((character, book_id, saved.len()))
    }
    .await;

    match result {
        Ok((mut character, world_info_book_id, regex_count)) => {
            if let Some(id) = &world_info_book_id {
                character.world_info_ids = vec![id.clone()];
            }
            Ok(SillyTavernImport::Character {
                character,
                world_info_book_id,
                regex_count,
                warnings: imported.warnings,
            })
        }
        Err(e) => {
            if let Some(path) = avatar_file {
                let _ = std::fs::remove_file(path);
            }
            Err(e)
        }
    }
}

async fn import_preset(db: &DbPool, json: &Value, name: &str) -> Result<SillyTavernImport> {
    let preset_id = Uuid::new_v4().to_string();
    let mut imported = preset::map_preset(json, &preset_id, name);
    let block_count = imported.blocks.len();

    let mut tx = db.begin().await?;
    db::ai_service::upsert_prompt_preset(&mut *tx, imported.preset).await?;
    for b in imported.blocks {
        db::ai_service::upsert_prompt_block(&mut *tx, &preset_id, b).await?;
    }
    let saved = save_regex(&mut tx, imported.regex, &mut imported.warnings).await?;
    tx.commit().await?;

    Ok(SillyTavernImport::Preset {
        preset_id,
        name: name.to_string(),
        block_count,
        regex_count: saved.len(),
        warnings: imported.warnings,
    })
}

async fn import_regex(db: &DbPool, json: &Value, preset_id: Option<String>) -> Result<SillyTavernImport> {
    let mut warnings = vec![];
    let scope = if preset_id.is_some() { RegexScope::Preset } else { RegexScope::Global };
    let scripts = regex::map_regex_scripts(json, scope, preset_id, &mut warnings);
    let mut tx = db.begin().await?;
    let ids = save_regex(&mut tx, scripts, &mut warnings).await?;
    tx.commit().await?;

    let mut saved = vec![];
    for id in ids {
        saved.push(db::regex::get_regex_script(db, &id).await?);
    }
    if saved.is_empty() {
        return Err(AppError::Validation {
            code: crate::error::ErrorCode::InvalidInput,
            message: "가져올 수 있는 정규식이 없습니다".into(),
            details: Some(serde_json::json!({ "warnings": warnings })),
        });
    }
    Ok(SillyTavernImport::Regex { scripts: saved, warnings })
}

async fn import_worldinfo(db: &DbPool, json: &Value, name: &str) -> Result<SillyTavernImport> {
    let mut warnings = vec![];
    let entries = worldinfo::map_entries(json, &mut warnings);
    let book_name = as_str(json, "name").filter(|s| !s.trim().is_empty()).unwrap_or(name);

    let mut tx = db.begin().await?;
    let book = db::conversation::create_worldinfo_book(
        &mut *tx,
        WorldInfoBookInput {
            id: None,
            name: book_name.to_string(),
            description: as_str(json, "description").map(str::to_string),
        },
    )
    .await?;
    let entry_count = entries.len();
    for mut e in entries {
        e.book_id = book.id.clone();
        db::conversation::upsert_worldinfo_entry(&mut *tx, e).await?;
    }
    tx.commit().await?;
    Ok(SillyTavernImport::WorldInfo { book, entry_count, warnings })
}

/// 정규식 저장. 패턴이 우리 엔진에서 컴파일되지 않으면 그 스크립트만 건너뛰고 경고.
async fn save_regex(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, scripts: Vec<RegexScriptInput>, warnings: &mut Vec<String>) -> Result<Vec<String>> {
    let mut ids = vec![];
    for s in scripts {
        if let Err(e) = validate_script(&s.to_script(String::new(), 0)) {
            warnings.push(format!("정규식 '{}'을(를) 건너뜀: {}", s.name, e.to_response().message));
            continue;
        }
        ids.push(db::regex::upsert_regex_script(&mut **tx, &s).await?);
    }
    Ok(ids)
}

// =======================================
// JSON 도우미
// =======================================

pub(crate) fn as_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// 문자열 배열 (문자열 하나면 그 하나, 그 외 무시)
pub(crate) fn str_list(v: &Value) -> Vec<String> {
    match v {
        Value::Array(a) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Value::String(s) if !s.is_empty() => vec![s.clone()],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests;
