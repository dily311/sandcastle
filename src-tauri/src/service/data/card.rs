//! 캐릭터를 SillyTavern 카드(V2 `chara_card_v2`)로 내보내기
//!
//! - JSON: 카드 JSON 그대로
//! - PNG: 아바타 그림(없으면 단색)에 tEXt `chara` 청크로 base64 카드를 넣는다 (SillyTavern이 읽는 방식)
//! 붙어 있는 로어북은 항목을 모아 `character_book`으로 넣는다.
use std::path::Path;

use base64::Engine;
use serde_json::{json, Value};

use crate::{
    db::{conn::DbPool, conversation},
    error::{AppError, Result},
    models::conversation::{Character, WorldInfoEntry},
};

/// 카드 JSON (V2)
pub fn card_json(c: &Character, book_name: Option<&str>, entries: &[WorldInfoEntry]) -> Value {
    let mut data = json!({
        "name": c.name,
        "description": c.description,
        "personality": "",
        "scenario": "",
        "first_mes": c.first_message.clone().unwrap_or_default(),
        "mes_example": c.example_dialogue.clone().unwrap_or_default(),
        "creator_notes": c.creator_notes.clone().unwrap_or_default(),
        "system_prompt": c.system_prompt_override.clone().unwrap_or_default(),
        "post_history_instructions": "",
        "alternate_greetings": c.alternate_first_message,
        "tags": c.tags,
        "creator": "",
        "character_version": "",
        "extensions": {},
    });
    if !entries.is_empty() {
        data["character_book"] = json!({
            "name": book_name.unwrap_or(&c.name),
            "entries": entries.iter().enumerate().map(|(i, e)| json!({
                "id": i,
                "keys": e.keys,
                "secondary_keys": e.secondary_keys,
                "selective": !e.secondary_keys.is_empty(),
                "comment": e.comment,
                "content": e.content,
                "constant": e.constant,
                "enabled": e.enabled,
                "insertion_order": e.priority,
                "extensions": {},
            })).collect::<Vec<_>>(),
        });
    }
    json!({ "spec": "chara_card_v2", "spec_version": "2.0", "data": data })
}

/// 캐릭터 하나를 카드 파일로. format: "json" | "png"
pub async fn export_card(db: &DbPool, character_id: &str, format: &str, path: &Path) -> Result<()> {
    let c = conversation::get_character(db, character_id).await?;
    let mut entries = vec![];
    let mut book_name = None;
    for id in &c.world_info_ids {
        entries.extend(conversation::get_all_worldinfo_entries(db, id).await?);
        if book_name.is_none() {
            book_name = conversation::get_all_worldinfo_books(db)
                .await?
                .into_iter()
                .find(|b| &b.id == id)
                .map(|b| b.name);
        }
    }
    let card = card_json(&c, book_name.as_deref(), &entries);
    let bytes = match format {
        "json" => serde_json::to_vec_pretty(&card).map_err(|e| AppError::internal(e.to_string()))?,
        "png" => {
            let png = avatar_png(c.avatar.as_deref())?;
            let text = base64::engine::general_purpose::STANDARD.encode(card.to_string());
            insert_text_chunk(&png, "chara", &text)?
        }
        _ => return Err(AppError::validation(format!("모르는 형식: {format}"), None)),
    };
    std::fs::write(path, bytes).map_err(|e| AppError::io(format!("파일을 쓸 수 없습니다: {e}")))
}

/// 아바타를 PNG로 (없거나 못 읽으면 400x600 단색)
fn avatar_png(avatar: Option<&str>) -> Result<Vec<u8>> {
    let img = avatar
        .filter(|p| Path::new(p).is_file())
        .and_then(|p| image::open(p).ok())
        .unwrap_or_else(|| image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(400, 600, image::Rgb([92, 104, 120]))));
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| AppError::internal(format!("PNG로 바꿀 수 없습니다: {e}")))?;
    Ok(out.into_inner())
}

/// IHDR 바로 뒤에 tEXt 청크 넣기 (같은 키워드의 tEXt가 있으면 뺌)
pub fn insert_text_chunk(png: &[u8], keyword: &str, text: &str) -> Result<Vec<u8>> {
    const SIG: &[u8] = b"\x89PNG\r\n\x1a\n";
    if !png.starts_with(SIG) {
        return Err(AppError::validation("PNG가 아님", None));
    }
    let mut out = SIG.to_vec();
    let mut pos = SIG.len();
    let mut inserted = false;
    while pos + 8 <= png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
        let end = pos + 12 + len;
        if end > png.len() {
            return Err(AppError::validation("PNG가 잘려 있음", None));
        }
        let kind = &png[pos + 4..pos + 8];
        let data = &png[pos + 8..pos + 8 + len];
        let same_key = kind == b"tEXt" && data.split(|b| *b == 0).next() == Some(keyword.as_bytes());
        if !same_key {
            out.extend_from_slice(&png[pos..end]);
        }
        if kind == b"IHDR" && !inserted {
            let mut body = keyword.as_bytes().to_vec();
            body.push(0);
            body.extend_from_slice(text.as_bytes());
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            let mut crc_input = b"tEXt".to_vec();
            crc_input.extend_from_slice(&body);
            out.extend_from_slice(&crc_input);
            out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
            inserted = true;
        }
        pos = end;
    }
    Ok(out)
}

/// PNG 청크용 CRC-32 (IEEE)
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::import::sillytavern::card::png_text_chunks;

    #[test]
    fn crc_matches_known_value() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }

    #[test]
    fn png_card_round_trips() {
        let png = avatar_png(None).unwrap();
        let out = insert_text_chunk(&png, "chara", "aGk=").unwrap();
        let again = insert_text_chunk(&out, "chara", "Ynll").unwrap();
        let chunks = png_text_chunks(&again).unwrap();
        assert_eq!(chunks.iter().filter(|(k, _)| k == "chara").count(), 1, "같은 키는 하나만");
        assert!(chunks.iter().any(|(k, v)| k == "chara" && v == "Ynll"));
        assert!(image::load_from_memory(&again).is_ok(), "그림으로도 열림");
    }
}
