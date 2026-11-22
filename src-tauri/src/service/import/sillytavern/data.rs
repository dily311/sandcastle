//! SillyTavern 전체 데이터 가져오기 (5-E-20-4-1).
//! SillyTavern의 사용자 폴더(`data/default-user`)를 압축한 zip — 또는 그 폴더 — 에서 한 번에 가져온다.
//!
//! 순서: 로어북(worlds) → 캐릭터(characters, 카드의 `extensions.world`로 로어북 연결) → 채팅(chats/<캐릭터 파일 이름>)
//! → 프리셋(OpenAI Settings) → 정규식(settings.json의 extension_settings.regex, 전역) → 페르소나(settings.json의 power_user + User Avatars).
//! 하나가 실패해도 나머지는 계속하고, 실패·건너뛴 것은 warnings로 돌려준다. 새로 만들기만 하고 지금 데이터는 고치지 않는다
//! (다시 가져오면 같은 것이 한 벌 더 생긴다). 단체 채팅·배경·테마·확장 프로그램은 가져오지 않는다.
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::Value;

use super::{import_file_with, import_regex, ImportOptions, SillyTavernImport};
use crate::{
    db::{self, conn::DbPool},
    error::{AppError, Result},
};

/// zip 안 파일 수·풀었을 때 전체 크기 상한 (지나치게 큰 압축 폭탄 막기)
const MAX_ZIP_FILES: usize = 50_000;
const MAX_ZIP_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SillyTavernDataImport {
    pub characters: usize,
    pub chats: usize,
    pub books: usize,
    pub presets: usize,
    pub regex: usize,
    pub personas: usize,
    /// 가져오지 못했거나 건너뛴 것 (파일 이름: 이유)
    pub warnings: Vec<String>,
}

/// zip을 임시 폴더에 풀어 가져온다 (끝나면 지움)
pub async fn import_data_zip(db: &DbPool, app_data_dir: &Path, zip_path: &Path) -> Result<SillyTavernDataImport> {
    let tmp = std::env::temp_dir().join(format!("sandcastle-st-{}", uuid::Uuid::new_v4()));
    let (zip, out) = (zip_path.to_path_buf(), tmp.clone());
    let unzipped = tokio::task::spawn_blocking(move || extract_zip(&zip, &out))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let result = match unzipped {
        Ok(()) => import_data_dir(db, app_data_dir, &tmp).await,
        Err(e) => Err(e),
    };
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// zip을 푼다. 폴더 밖으로 나가는 경로(`../`, 절대 경로)는 건너뛴다
fn extract_zip(zip_path: &Path, out: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path).map_err(|e| AppError::io(format!("ZIP을 열 수 없습니다: {e}")))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| AppError::validation(format!("ZIP 파일이 아닙니다: {e}"), None))?;
    if zip.len() > MAX_ZIP_FILES {
        return Err(AppError::validation(format!("ZIP에 파일이 너무 많습니다 ({MAX_ZIP_FILES}개까지)"), None));
    }
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| AppError::io(e.to_string()))?;
        let Some(rel) = f.enclosed_name() else { continue };
        if f.is_dir() || rel.components().any(|c| c.as_os_str() == "__MACOSX") {
            continue;
        }
        total += f.size();
        if total > MAX_ZIP_BYTES {
            return Err(AppError::validation("ZIP을 풀면 4GB가 넘어 가져올 수 없습니다", None));
        }
        let dest = out.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).map_err(|e| AppError::io(e.to_string()))?;
        std::fs::write(&dest, buf)?;
    }
    Ok(())
}

/// SillyTavern 사용자 폴더 찾기: `characters`·`worlds`·`chats` 폴더나 `settings.json`이 있는 가장 얕은 폴더
pub fn find_user_root(dir: &Path) -> Option<PathBuf> {
    let is_root = |d: &Path| ["characters", "worlds", "chats"].iter().any(|s| d.join(s).is_dir()) || d.join("settings.json").is_file();
    let mut level = vec![dir.to_path_buf()];
    for _ in 0..6 {
        if let Some(found) = level.iter().find(|d| is_root(d)) {
            return Some(found.clone());
        }
        level = level
            .iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        level.sort();
    }
    None
}

/// 폴더의 파일들 (확장자가 맞는 것, 이름순)
fn files_in(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    out.retain(|p| p.is_file() && p.extension().is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x))));
    out.sort();
    out
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

fn label(p: &Path, root: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

pub async fn import_data_dir(db: &DbPool, app_data_dir: &Path, dir: &Path) -> Result<SillyTavernDataImport> {
    let root = find_user_root(dir)
        .ok_or_else(|| AppError::validation("SillyTavern 데이터 폴더가 아닙니다 (characters·worlds·chats 폴더나 settings.json이 없음)", None))?;
    let mut out = SillyTavernDataImport::default();
    let settings: Value = std::fs::read(root.join("settings.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null);

    // 1. 로어북: 파일 이름(= SillyTavern의 로어북 이름) → 새 로어북 id
    let mut books: HashMap<String, String> = HashMap::new();
    for p in files_in(&root.join("worlds"), &["json"]) {
        match import_file_with(db, app_data_dir, &p, ImportOptions::default()).await {
            Ok(SillyTavernImport::WorldInfo { book, .. }) => {
                books.insert(stem(&p), book.id);
                out.books += 1;
            }
            Ok(_) => out.warnings.push(format!("{}: 로어북이 아님", label(&p, &root))),
            Err(e) => out.warnings.push(format!("{}: {}", label(&p, &root), e)),
        }
    }

    // 2. 캐릭터: 파일 이름(채팅 폴더 이름과 같음) → 새 캐릭터 id
    let mut chars: HashMap<String, String> = HashMap::new();
    for p in files_in(&root.join("characters"), &["png", "json"]) {
        match import_file_with(db, app_data_dir, &p, ImportOptions::default()).await {
            Ok(SillyTavernImport::Character { character, .. }) => {
                out.characters += 1;
                // 카드가 이름으로 가리키는 로어북 (worlds 폴더의 것)도 연결
                if let Some(book) = linked_world(&p).and_then(|w| books.get(&w).cloned()) {
                    if let Err(e) = link_book(db, &character.id, &book).await {
                        out.warnings.push(format!("{}: 로어북 연결 못 함 ({})", label(&p, &root), e));
                    }
                }
                chars.insert(stem(&p), character.id);
            }
            Ok(_) => out.warnings.push(format!("{}: 캐릭터 카드가 아님", label(&p, &root))),
            Err(e) => out.warnings.push(format!("{}: {}", label(&p, &root), e)),
        }
    }

    // 3. 채팅: chats/<캐릭터 파일 이름>/*.jsonl
    let mut chat_dirs: Vec<PathBuf> = std::fs::read_dir(root.join("chats"))
        .map(|r| r.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
        .unwrap_or_default();
    chat_dirs.sort();
    for d in chat_dirs {
        let character_id = d.file_name().and_then(|n| chars.get(&n.to_string_lossy().to_string()).cloned());
        for p in files_in(&d, &["jsonl"]) {
            let opts = ImportOptions {
                character_id: character_id.clone(),
                ..Default::default()
            };
            match import_file_with(db, app_data_dir, &p, opts).await {
                Ok(_) => out.chats += 1,
                Err(e) => out.warnings.push(format!("{}: {}", label(&p, &root), e)),
            }
        }
    }

    // 4. 프리셋 (Chat Completion). 다른 API 폴더(TextGen·Kobold·NovelAI Settings)는 샘플러 값뿐이라 건너뜀
    for p in files_in(&root.join("OpenAI Settings"), &["json"]) {
        match import_file_with(db, app_data_dir, &p, ImportOptions::default()).await {
            Ok(SillyTavernImport::Preset { .. }) => out.presets += 1,
            Ok(_) => out.warnings.push(format!("{}: 프리셋이 아님", label(&p, &root))),
            Err(e) => out.warnings.push(format!("{}: {}", label(&p, &root), e)),
        }
    }
    for other in ["TextGen Settings", "KoboldAI Settings", "NovelAI Settings", "instruct", "context"] {
        let n = files_in(&root.join(other), &["json"]).len();
        if n > 0 {
            out.warnings.push(format!("{other}: {n}개 건너뜀 (Chat Completion 프리셋만 가져옴)"));
        }
    }

    // 5. 전역 정규식
    if let Some(scripts) = settings
        .pointer("/extension_settings/regex")
        .filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))
    {
        match import_regex(db, scripts, None).await {
            Ok(SillyTavernImport::Regex { scripts, warnings }) => {
                out.regex += scripts.len();
                out.warnings.extend(warnings.into_iter().map(|w| format!("정규식: {w}")));
            }
            Ok(_) => {}
            Err(e) => out.warnings.push(format!("settings.json 정규식: {}", e)),
        }
    }

    // 6. 페르소나: power_user.personas {그림 파일: 이름}, persona_descriptions {그림 파일: {description}}
    if let Some(personas) = settings.pointer("/power_user/personas").and_then(Value::as_object) {
        let descriptions = settings.pointer("/power_user/persona_descriptions");
        let avatar_dir = app_data_dir.join("characters").join("avatars");
        let mut list: Vec<(&String, &Value)> = personas.iter().collect();
        list.sort_by(|a, b| a.0.cmp(b.0));
        for (file, name) in list {
            let Some(name) = name.as_str().map(str::trim).filter(|n| !n.is_empty()) else {
                continue;
            };
            let description = descriptions
                .and_then(|d| d.get(file))
                .and_then(|d| d.get("description"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let src = root.join("User Avatars").join(file);
            let avatar = (src.is_file() && !file.contains(['/', '\\']))
                .then(|| crate::commands::conversation::copy_avatar(&avatar_dir, &src.to_string_lossy(), "persona").ok())
                .flatten();
            let input = crate::models::conversation::PersonaInput {
                name: name.to_string(),
                avatar,
                description,
                world_info_ids: vec![],
                tags: vec![],
                color: String::new(),
                is_system_default: false,
            };
            match db::conversation::create_persona(db, input).await {
                Ok(_) => out.personas += 1,
                Err(e) => out.warnings.push(format!("페르소나 {name}: {e}")),
            }
        }
    }

    let groups = files_in(&root.join("groups"), &["json"]).len();
    if groups > 0 {
        out.warnings.push(format!("단체 채팅 {groups}개 건너뜀 (지원하지 않음)"));
    }
    Ok(out)
}

/// 카드가 이름으로 가리키는 로어북 (`data.extensions.world`)
fn linked_world(card: &Path) -> Option<String> {
    let bytes = std::fs::read(card).ok()?;
    let json = super::detect(&bytes).ok()?.1;
    json.pointer("/data/extensions/world")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_string)
}

async fn link_book(db: &DbPool, character_id: &str, book_id: &str) -> Result<()> {
    let c = db::conversation::get_character(db, character_id).await?;
    if c.world_info_ids.iter().any(|b| b == book_id) {
        return Ok(());
    }
    let mut ids = c.world_info_ids;
    ids.push(book_id.to_string());
    let mut tx = db.begin().await?;
    db::conversation::replace_character_worldinfo(&mut tx, character_id, &ids).await?;
    tx.commit().await?;
    Ok(())
}
