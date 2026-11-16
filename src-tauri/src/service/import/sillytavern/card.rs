//! SillyTavern 캐릭터 카드 (V1 / V2 `chara_card_v2` / V3 `chara_card_v3`)
//!
//! - PNG: tEXt/iTXt 청크의 `ccv3`(우선) 또는 `chara`에 base64 JSON이 들어 있다. 이미지 자체는 아바타.
//! - JSON: 카드 JSON 그대로.
//!
//! 우리 캐릭터에 없는 필드 처리
//! - personality / scenario → 설명 뒤에 `[Personality]`, `[Scenario]` 단락으로 합침
//! - post_history_instructions, depth_prompt → 지원하지 않음 (경고)
use base64::{prelude::BASE64_STANDARD, Engine};
use serde_json::Value;

use crate::{
    error::{AppError, Result},
    models::{
        conversation::{CharacterInput, WorldInfoBookInput, WorldInfoEntryInput},
        regex::{RegexScope, RegexScriptInput},
    },
};

use super::{as_str, regex::map_regex_scripts, str_list, worldinfo::map_entries};

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&PNG_SIGNATURE)
}

/// PNG 텍스트 청크에서 카드 JSON 꺼내기
pub fn card_json_from_png(bytes: &[u8]) -> Result<Value> {
    let mut chara = None;
    let mut ccv3 = None;
    for (keyword, text) in png_text_chunks(bytes)? {
        match keyword.to_ascii_lowercase().as_str() {
            "ccv3" => ccv3 = Some(text),
            "chara" => chara = Some(text),
            _ => {}
        }
    }
    let encoded = ccv3
        .or(chara)
        .ok_or_else(|| AppError::validation("PNG에 캐릭터 카드 데이터(chara/ccv3)가 없습니다", None))?;
    let cleaned: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = BASE64_STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|e| AppError::validation(format!("카드 데이터 base64 해석 실패: {e}"), None))?;
    serde_json::from_slice(&decoded).map_err(|e| AppError::validation(format!("카드 JSON 해석 실패: {e}"), None))
}

/// (키워드, 텍스트) 목록. 압축된 zTXt / 압축 iTXt는 건너뛴다(카드는 보통 비압축 tEXt).
pub(crate) fn png_text_chunks(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    if !is_png(bytes) {
        return Err(AppError::validation("PNG 파일이 아닙니다", None));
    }
    let mut out = vec![];
    let mut pos = PNG_SIGNATURE.len();
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = data_start
            .checked_add(len)
            .filter(|e| *e + 4 <= bytes.len())
            .ok_or_else(|| AppError::validation("PNG 청크가 손상되었습니다", None))?;
        let data = &bytes[data_start..data_end];
        match kind {
            b"tEXt" => {
                if let Some(nul) = data.iter().position(|b| *b == 0) {
                    // tEXt는 Latin-1이지만 카드 데이터는 base64(ASCII)라 그대로 읽어도 된다
                    let keyword = String::from_utf8_lossy(&data[..nul]).to_string();
                    let text = String::from_utf8_lossy(&data[nul + 1..]).to_string();
                    out.push((keyword, text));
                }
            }
            b"iTXt" => {
                if let Some(text) = parse_itxt(data) {
                    out.push(text);
                }
            }
            b"IEND" => break,
            _ => {}
        }
        pos = data_end + 4; // CRC
    }
    Ok(out)
}

/// iTXt: keyword\0 압축플래그 압축방식 언어\0 번역키워드\0 텍스트
fn parse_itxt(data: &[u8]) -> Option<(String, String)> {
    let nul = data.iter().position(|b| *b == 0)?;
    let keyword = String::from_utf8_lossy(&data[..nul]).to_string();
    let rest = &data[nul + 1..];
    if rest.len() < 2 || rest[0] != 0 {
        return None; // 압축된 iTXt는 지원하지 않음
    }
    let rest = &rest[2..];
    let lang_end = rest.iter().position(|b| *b == 0)?;
    let rest = &rest[lang_end + 1..];
    let tr_end = rest.iter().position(|b| *b == 0)?;
    Some((keyword, String::from_utf8_lossy(&rest[tr_end + 1..]).to_string()))
}

pub struct ImportedCharacter {
    pub character: CharacterInput,
    /// (책, 항목). 항목의 book_id는 저장할 때 채운다.
    pub book: Option<(WorldInfoBookInput, Vec<WorldInfoEntryInput>)>,
    /// scope_id(캐릭터 id)는 저장할 때 채운다.
    pub regex: Vec<RegexScriptInput>,
    pub warnings: Vec<String>,
}

pub fn map_card(card: &Value) -> Result<ImportedCharacter> {
    // V2/V3는 data 아래, V1은 최상위
    let data = if card["data"].is_object() && card["spec"].is_string() {
        &card["data"]
    } else {
        card
    };
    let name = as_str(data, "name")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::validation("캐릭터 이름이 없습니다", None))?
        .to_string();
    let text = |k: &str| as_str(data, k).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let mut warnings = vec![];

    let mut description = text("description").unwrap_or_default();
    for (key, label) in [("personality", "Personality"), ("scenario", "Scenario")] {
        if let Some(v) = text(key) {
            if !description.is_empty() {
                description.push_str("\n\n");
            }
            description.push_str(&format!("[{label}]\n{v}"));
        }
    }
    if text("post_history_instructions").is_some() {
        warnings.push("post_history_instructions(기록 뒤 지시)는 지원하지 않아 가져오지 않음".into());
    }
    if as_str(&data["extensions"]["depth_prompt"], "prompt").is_some_and(|s| !s.trim().is_empty()) {
        warnings.push("depth_prompt(캐릭터 노트)는 지원하지 않아 가져오지 않음".into());
    }

    let book = if data["character_book"].is_object() {
        let book = &data["character_book"];
        let entries = map_entries(book, &mut warnings);
        (!entries.is_empty()).then(|| {
            (
                WorldInfoBookInput {
                    id: None,
                    name: as_str(book, "name")
                        .filter(|s| !s.trim().is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("{name} 로어북")),
                    description: as_str(book, "description").map(str::to_string),
                },
                entries,
            )
        })
    } else {
        None
    };

    let regex = map_regex_scripts(&data["extensions"]["regex_scripts"], RegexScope::Character, None, &mut warnings);

    Ok(ImportedCharacter {
        character: CharacterInput {
            name,
            avatar: None,
            description,
            alternate_descriptions: vec![],
            first_message: text("first_mes"),
            alternate_first_message: str_list(&data["alternate_greetings"]).into_iter().filter(|s| !s.trim().is_empty()).collect(),
            example_dialogue: text("mes_example"),
            system_prompt_override: text("system_prompt"),
            creator_notes: text("creator_notes"),
            world_info_ids: vec![],
            tags: str_list(&data["tags"]),
            color: None,
            category: None,
            default_persona_id: None,
            default_prompt_preset_id: None,
            favorite: false,
        },
        book,
        regex,
        warnings,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// 테스트용: PNG에 tEXt 청크 하나를 IEND 앞에 끼운다 (CRC는 파서가 보지 않으므로 0)
    pub(crate) fn png_with_text(keyword: &str, text: &str) -> Vec<u8> {
        let mut img = vec![];
        image::RgbImage::from_pixel(4, 4, image::Rgb([1, 2, 3]))
            .write_to(&mut std::io::Cursor::new(&mut img), image::ImageFormat::Png)
            .unwrap();
        let iend = img.len() - 12;
        let mut data = keyword.as_bytes().to_vec();
        data.push(0);
        data.extend_from_slice(text.as_bytes());
        let mut chunk = (data.len() as u32).to_be_bytes().to_vec();
        chunk.extend_from_slice(b"tEXt");
        chunk.extend_from_slice(&data);
        chunk.extend_from_slice(&[0, 0, 0, 0]);
        img.splice(iend..iend, chunk);
        img
    }

    pub(crate) fn v2_card() -> Value {
        json!({
            "spec": "chara_card_v2", "spec_version": "2.0",
            "data": {
                "name": "Aria", "description": "A knight.", "personality": "Brave", "scenario": "",
                "first_mes": "Hello.", "mes_example": "<START>", "alternate_greetings": ["Hi", " "],
                "system_prompt": "", "post_history_instructions": "Stay brave", "tags": ["fantasy"],
                "character_book": { "entries": [ { "keys": ["sword"], "content": "Her sword.", "enabled": true } ] },
                "extensions": { "regex_scripts": [ { "scriptName": "r", "findRegex": "/a/g", "placement": [2] } ] }
            }
        })
    }

    #[test]
    fn reads_card_from_png_preferring_ccv3() {
        let v2 = BASE64_STANDARD.encode(v2_card().to_string());
        let png = png_with_text("chara", &v2);
        assert_eq!(card_json_from_png(&png).unwrap()["data"]["name"], "Aria");

        let v3 = BASE64_STANDARD.encode(json!({ "spec": "chara_card_v3", "data": { "name": "V3" } }).to_string());
        let mut both = png_with_text("chara", &v2);
        let extra = png_with_text("ccv3", &v3);
        // 두 번째 PNG의 ccv3 청크를 첫 PNG에 추가
        let start = extra.windows(4).position(|w| w == b"tEXt").unwrap() - 4;
        let chunk = extra[start..extra.len() - 12].to_vec();
        let iend = both.len() - 12;
        both.splice(iend..iend, chunk);
        assert_eq!(card_json_from_png(&both).unwrap()["data"]["name"], "V3");

        let plain = png_with_text("Comment", "hi");
        assert!(card_json_from_png(&plain).is_err());
    }

    #[test]
    fn maps_v2_card() {
        let c = map_card(&v2_card()).unwrap();
        assert_eq!(c.character.name, "Aria");
        assert_eq!(c.character.description, "A knight.\n\n[Personality]\nBrave");
        assert_eq!(c.character.first_message.as_deref(), Some("Hello."));
        assert_eq!(c.character.alternate_first_message, vec!["Hi"]);
        assert_eq!(c.character.system_prompt_override, None);
        let (book, entries) = c.book.unwrap();
        assert_eq!(book.name, "Aria 로어북");
        assert_eq!(entries[0].keys, vec!["sword"]);
        assert_eq!(c.regex.len(), 1);
        assert_eq!(c.regex[0].scope, RegexScope::Character);
        assert_eq!(c.warnings.len(), 1, "{:?}", c.warnings);
    }

    #[test]
    fn maps_v1_card_and_requires_name() {
        let c = map_card(&json!({ "name": "Old", "description": "d", "first_mes": "" })).unwrap();
        assert_eq!(c.character.name, "Old");
        assert_eq!(c.character.first_message, None);
        assert!(map_card(&json!({ "description": "no name" })).is_err());
    }
}
