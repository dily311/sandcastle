//! 샌드캐슬 → SillyTavern 사용자 폴더 모양 zip (5-E-20-4-2, ADR-024).
//! zip 안 `default-user/` 아래를 SillyTavern의 `data/default-user`에 그대로 넣으면 된다.
//!
//! - characters/<이름>.png — 카드(아바타 그림 안에 카드 JSON). 파일 이름이 채팅 폴더 이름이 된다
//! - chats/<이름>/<대화 제목>.jsonl — 그 캐릭터의 대화(폰 대화 포함)
//! - worlds/<로어북 이름>.json
//! - User Avatars/<파일>.png + personas.json — 페르소나 그림과 "페르소나 관리 > 복원"용 백업
//! - regex/sandcastle-regex.json — 전역 정규식 ("정규식 > 가져오기"로 넣음)
//!
//! `settings.json`은 만들지 않는다 (사용자의 SillyTavern 설정을 덮어쓰지 않게).
//! 프리셋(5-E-20-4-3)·게임·위젯·상태 같은 샌드캐슬 전용 기능은 넣지 않는다.
use std::{collections::HashSet, io::Write, path::Path};

use serde::Serialize;
use serde_json::{json, Map, Value};

use super::export::{to_jsonl, JsonlChat, JsonlOptions};
use crate::{
    db::{self, conn::DbPool},
    error::{AppError, Result},
    models::regex::RegexScope,
};

const ROOT: &str = "default-user";

#[derive(Debug, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SillyTavernDataExport {
    pub characters: usize,
    pub chats: usize,
    pub books: usize,
    pub personas: usize,
    pub regex: usize,
    /// 넣지 못한 것 (이름: 이유)
    pub warnings: Vec<String>,
}

/// 파일 이름으로 쓸 수 없는 글자를 바꾸고, 같은 폴더 안에서 겹치지 않게 " (2)"…를 붙인다
fn unique_name(raw: &str, taken: &mut HashSet<String>) -> String {
    let clean: String = raw
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim().trim_end_matches('.').trim();
    let base = if clean.is_empty() {
        "unnamed".to_string()
    } else {
        clean.chars().take(80).collect()
    };
    let mut name = base.clone();
    let mut n = 2;
    while !taken.insert(name.to_lowercase()) {
        name = format!("{base} ({n})");
        n += 1;
    }
    name
}

struct Zip {
    w: zip::ZipWriter<std::io::BufWriter<std::fs::File>>,
}
impl Zip {
    fn add(&mut self, rel: &str, bytes: &[u8]) -> Result<()> {
        let opt = zip::write::SimpleFileOptions::default().large_file(bytes.len() as u64 > u32::MAX as u64);
        self.w
            .start_file(format!("{ROOT}/{rel}"), opt)
            .map_err(|e| AppError::io(format!("zip에 쓸 수 없습니다: {e}")))?;
        self.w.write_all(bytes).map_err(|e| AppError::io(format!("zip에 쓸 수 없습니다: {e}")))
    }
}

pub async fn export_data_zip(db: &DbPool, path: &Path) -> Result<SillyTavernDataExport> {
    let file = std::fs::File::create(path).map_err(|e| AppError::io(format!("파일을 만들 수 없습니다: {e}")))?;
    let mut zip = Zip {
        w: zip::ZipWriter::new(std::io::BufWriter::new(file)),
    };
    let mut out = SillyTavernDataExport::default();
    let personas = db::conversation::get_all_personas(db).await?;

    // 캐릭터 + 그 캐릭터의 대화
    let mut char_names = HashSet::new();
    let convs = db::conversation::get_all_conversations(db).await?;
    for c in db::conversation::get_all_characters(db).await? {
        let png = match crate::service::data::card::card_bytes(db, &c.id, "png").await {
            Ok(b) => b,
            Err(e) => {
                out.warnings.push(format!("캐릭터 {}: {e}", c.name));
                continue;
            }
        };
        let stem = unique_name(&c.name, &mut char_names);
        zip.add(&format!("characters/{stem}.png"), &png)?;
        out.characters += 1;

        let mut chat_names = HashSet::new();
        for conv in convs.iter().filter(|v| v.character_id == c.id) {
            let messages = db::conversation::get_messages(db, &conv.id, i64::MAX, None).await?;
            if messages.is_empty() {
                continue;
            }
            let user_name = personas
                .iter()
                .find(|p| p.id == conv.persona_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "User".into());
            let text = to_jsonl(
                &JsonlChat {
                    user_name: &user_name,
                    char_name: &c.name,
                    created_at: conv.created_at,
                    conv_meta: conv.meta.as_ref(),
                    messages: &messages,
                },
                &JsonlOptions { cg_as_img: false },
            );
            let name = unique_name(&conv.title, &mut chat_names);
            zip.add(&format!("chats/{stem}/{name}.jsonl"), text.as_bytes())?;
            out.chats += 1;
        }
    }

    // 로어북
    let mut book_names = HashSet::new();
    for b in db::conversation::get_all_worldinfo_books(db).await? {
        let entries = db::conversation::get_all_worldinfo_entries(db, &b.id).await?;
        let json = crate::commands::conversation::sillytavern_book_json(&entries);
        let name = unique_name(&b.name, &mut book_names);
        zip.add(&format!("worlds/{name}.json"), json.to_string().as_bytes())?;
        out.books += 1;
    }

    // 페르소나: 그림 + SillyTavern 페르소나 백업 모양
    let mut files = HashSet::new();
    let (mut names, mut descriptions) = (Map::new(), Map::new());
    for p in &personas {
        let file = format!("{}.png", unique_name(&p.name, &mut files));
        match crate::service::data::card::avatar_png(p.avatar.as_deref()) {
            Ok(png) => zip.add(&format!("User Avatars/{file}"), &png)?,
            Err(e) => out.warnings.push(format!("페르소나 {} 그림: {e}", p.name)),
        }
        names.insert(file.clone(), json!(p.name));
        descriptions.insert(file, json!({ "description": p.description, "position": 0 }));
        out.personas += 1;
    }
    if out.personas > 0 {
        let backup = json!({ "personas": names, "persona_descriptions": descriptions, "default_persona": Value::Null });
        zip.add("personas.json", serde_json::to_string_pretty(&backup).unwrap_or_default().as_bytes())?;
    }

    // 전역 정규식 (캐릭터 것은 카드 안에 들어감, 프리셋 것은 프리셋과 함께 — 5-E-20-4-3)
    let scripts: Vec<Value> = db::regex::get_all_regex_scripts(db)
        .await?
        .iter()
        .filter(|r| r.scope == RegexScope::Global)
        .map(super::regex::regex_to_st)
        .collect();
    if !scripts.is_empty() {
        out.regex = scripts.len();
        zip.add(
            "regex/sandcastle-regex.json",
            serde_json::to_string_pretty(&scripts).unwrap_or_default().as_bytes(),
        )?;
    }

    zip.add(
        "README.txt",
        "샌드캐슬에서 SillyTavern 모양으로 내보낸 파일\n\n\
         1. SillyTavern을 끄고, 이 default-user 폴더 안의 characters·chats·worlds·User Avatars를 SillyTavern의 data/default-user에 같은 이름 폴더로 복사합니다 (같은 이름 파일은 덮어씁니다).\n\
         2. SillyTavern을 켜고 페르소나 관리 > 복원(Restore)에서 personas.json을 고릅니다.\n\
         3. 확장 > 정규식 > 가져오기(Import)에서 regex/sandcastle-regex.json을 고릅니다.\n"
            .as_bytes(),
    )?;
    zip.w.finish().map_err(|e| AppError::io(format!("zip을 마무리할 수 없습니다: {e}")))?;
    Ok(out)
}
