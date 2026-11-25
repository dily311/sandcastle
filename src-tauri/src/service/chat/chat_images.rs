//! chat/phone 모드의 캐릭터 이미지.
//!
//! 게임 모드의 연출 태그(감정·모션·효과 …)와 달리 가볍게 간다:
//! 모델에게 사용 가능한 이미지 키 목록만 보내고, 모델은 어울릴 때 `<cg>키</cg>`를 적는다.
//!
//! - 켜짐 조건: 앱 전체 스위치(FeatureSetting `chat.images`, 없으면 켜짐)
//!   && 대화 스위치(conversations.images_enabled) && 캐릭터에 켜진 이미지가 있음
//! - 응답의 `<cg>` 태그는 목록에 있는 키만 남기고(키 표기는 등록된 그대로), 나머지는 지운다.
//!   저장되는 본문에 태그를 그대로 두어서, 프론트는 그 자리에 이미지를 그리고
//!   모델은 이후 대화 기록에서 자기가 띄운 이미지를 알 수 있다.
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::{
    db::{self, conn::DbPool},
    error::{AppError, Result},
    models::character_image::{CharacterImage, CharacterImageRow, ChatImageRef},
};

/// 앱 전체 스위치 키 (FeatureSetting)
pub const FEATURE_KEY: &str = "chat.images";
/// 프롬프트에 넣을 최대 키 수
pub const MAX_PROMPT_KEYS: usize = 100;
/// 가져올 원본 파일 최대 크기
pub const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_KEY_CHARS: usize = 40;
const THUMBNAIL_SIZE: u32 = 256;

const OPEN: &str = "<cg>";
const CLOSE: &str = "</cg>";

// =======================================
// 키 규칙
// =======================================

/// 사용자 입력 키 정리. 앞뒤 공백 제거, 연속 공백은 하나로. 태그를 깨는 문자는 거부.
pub fn normalize_key(raw: &str) -> Result<String> {
    let key = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if key.is_empty() {
        return Err(AppError::validation("이미지 키가 비어 있습니다", None));
    }
    if key.chars().any(|c| matches!(c, '<' | '>' | '/' | '\\' | ',')) {
        return Err(AppError::validation("이미지 키에는 < > / \\ , 를 쓸 수 없습니다", None));
    }
    if key.chars().count() > MAX_KEY_CHARS {
        return Err(AppError::validation(format!("이미지 키는 {MAX_KEY_CHARS}자 이하로 정해 주세요"), None));
    }
    Ok(key)
}

/// 파일 이름에서 기본 키 만들기 ("Smile Big.png" → "Smile Big")
pub fn key_from_path(path: &Path) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let cleaned: String = stem.chars().map(|c| if matches!(c, '<' | '>' | '/' | '\\' | ',') { '_' } else { c }).collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated: String = cleaned.chars().take(MAX_KEY_CHARS).collect();
    if truncated.is_empty() {
        "image".to_string()
    } else {
        truncated
    }
}

// =======================================
// 파일 저장
// =======================================

/// 캐릭터 이미지 폴더 (앱 데이터 기준 상대 경로)
pub fn relative_dir(character_id: &str) -> PathBuf {
    PathBuf::from("characters").join(character_id).join("images")
}

/// 사용자가 고른 파일을 읽어 webp 원본 + 썸네일로 저장. (원본 상대 경로, 썸네일 상대 경로) 반환.
/// 이미지 디코딩/인코딩이 무거우므로 blocking 스레드에서 부를 것.
pub fn store_image_file(app_data_dir: &Path, character_id: &str, image_id: &str, source: &Path) -> Result<(String, String)> {
    let meta = std::fs::metadata(source).map_err(|e| AppError::io(format!("파일을 열 수 없습니다 ({}): {e}", source.display())))?;
    if !meta.is_file() {
        return Err(AppError::validation("파일이 아닙니다", None));
    }
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(AppError::validation(
            format!("이미지는 {}MB 이하만 넣을 수 있습니다", MAX_IMAGE_BYTES / 1024 / 1024),
            None,
        ));
    }
    let bytes = std::fs::read(source)?;
    if !infer::is_image(&bytes) {
        return Err(AppError::validation(format!("이미지 파일이 아닙니다: {}", source.display()), None));
    }

    let rel_dir = relative_dir(character_id);
    let abs_dir = app_data_dir.join(&rel_dir);
    crate::sys::image::save_image_to_origin(abs_dir.clone(), image_id, &bytes)?;
    crate::sys::image::save_image_to_thumbnail(abs_dir.join("thumb"), image_id, &bytes, THUMBNAIL_SIZE)?;

    let rel = |p: PathBuf| p.to_string_lossy().replace('\\', "/");
    Ok((
        rel(rel_dir.join(format!("{image_id}.webp"))),
        rel(rel_dir.join("thumb").join(format!("{image_id}.webp"))),
    ))
}

/// 여러 이미지를 캐릭터에 추가. 하나가 실패하면 거기서 멈추고, 앞서 추가된 것은 남긴다.
/// 이미지 변환은 blocking 스레드에서 한다.
pub async fn add_images(db: &DbPool, app_data_dir: &Path, character_id: &str, paths: &[PathBuf]) -> Result<Vec<CharacterImage>> {
    let with_folder: Vec<(PathBuf, String)> = paths.iter().map(|p| (p.clone(), String::new())).collect();
    add_images_in(db, app_data_dir, character_id, &with_folder).await
}

/// 판 폴더와 함께 넣기 (ZIP의 하위 폴더 `v1/smile.png` → 판 `v1`, 5-E-21)
pub async fn add_images_in(db: &DbPool, app_data_dir: &Path, character_id: &str, paths: &[(PathBuf, String)]) -> Result<Vec<CharacterImage>> {
    let items: Vec<(PathBuf, String, Option<String>)> = paths.iter().map(|(p, f)| (p.clone(), f.clone(), None)).collect();
    add_images_keyed(db, app_data_dir, character_id, &items).await
}

/// 키를 직접 정해 넣기 (None이면 파일 이름). 게임 → 캐릭터 복사에서 NPC 스탠딩을 `이름_감정` 키로 (5-F-3)
pub async fn add_images_keyed(
    db: &DbPool,
    app_data_dir: &Path,
    character_id: &str,
    items: &[(PathBuf, String, Option<String>)],
) -> Result<Vec<CharacterImage>> {
    if !db::conversation::char_exists(db, character_id).await? {
        return Err(AppError::not_found("존재하지 않는 캐릭터입니다"));
    }
    let mut added = vec![];
    for (source, folder, given) in items {
        // 같은 키(파일 이름)가 이미 있으면 그 이미지를 새 파일로 덮어쓴다 (id·키·순서 유지)
        let key = normalize_key(&given.clone().unwrap_or_else(|| key_from_path(source))).unwrap_or_else(|_| "image".to_string());
        let existing: Option<(String, i64, bool, i64)> =
            sqlx::query_as("SELECT id, sort_order, enabled, created_at FROM character_images WHERE character_id = ? AND folder = ? AND key = ?")
                .bind(character_id)
                .bind(folder)
                .bind(&key)
                .fetch_optional(db)
                .await?;
        let id = existing.as_ref().map(|e| e.0.clone()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let (file_path, thumb_path) = {
            let (base, cid, id, source) = (app_data_dir.to_path_buf(), character_id.to_string(), id.clone(), source.clone());
            tokio::task::spawn_blocking(move || store_image_file(&base, &cid, &id, &source))
                .await
                .map_err(|e| AppError::internal(e.to_string()))??
        };

        let now = chrono::Utc::now().timestamp_millis();
        if let Some((_, sort_order, enabled, created_at)) = existing {
            sqlx::query("UPDATE character_images SET file_path = ?, thumb_path = ?, updated_at = ? WHERE id = ?")
                .bind(&file_path)
                .bind(&thumb_path)
                .bind(now)
                .bind(&id)
                .execute(db)
                .await?;
            let row = CharacterImageRow {
                id,
                character_id: character_id.to_string(),
                key,
                file_path,
                thumb_path: Some(thumb_path),
                enabled,
                sort_order,
                created_at,
                updated_at: now,
                folder: folder.clone(),
            };
            added.push(CharacterImage::from_row(row, app_data_dir));
            continue;
        }
        let row = CharacterImageRow {
            id,
            character_id: character_id.to_string(),
            key,
            file_path,
            thumb_path: Some(thumb_path),
            enabled: true,
            sort_order: db::character_image::next_image_sort_order(db, character_id).await?,
            created_at: now,
            updated_at: now,
            folder: folder.clone(),
        };
        if let Err(e) = db::character_image::insert_character_image(db, &row).await {
            remove_image_files(app_data_dir, &[&row.file_path, row.thumb_path.as_deref().unwrap_or_default()]);
            return Err(e.into());
        }
        added.push(CharacterImage::from_row(row, app_data_dir));
    }
    Ok(added)
}

/// 한 ZIP에 넣을 수 있는 그림 수
pub const MAX_ZIP_IMAGES: usize = 1000;

fn is_zip_image_name(name: &str) -> bool {
    let Some(file) = std::path::Path::new(name).file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return false;
    };
    let ext = file.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    !file.starts_with('.') && ["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.as_str())
}

/// ZIP 안의 그림들을 임시 폴더에 풀어 경로를 돌려준다 (폴더 구조는 버리고 파일 이름만).
/// 그림이 MAX_ZIP_IMAGES장을 넘으면 풀기 전에 거절한다 (일부만 넣고 나머지를 조용히 버리지 않게)
/// ZIP 안 그림을 풀고 (경로, 판 폴더)를 돌려준다. 판 폴더는 그림 바로 위 폴더 이름 (맨 위에 있으면 빈 글 = 공통)
pub fn extract_zip_images(zip_path: &Path, out_dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let file = std::fs::File::open(zip_path).map_err(|e| AppError::io(format!("ZIP을 열 수 없습니다: {e}")))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| AppError::validation(format!("ZIP 파일이 아닙니다: {e}"), None))?;
    let total = (0..zip.len()).filter(|&i| zip.name_for_index(i).is_some_and(is_zip_image_name)).count();
    if total > MAX_ZIP_IMAGES {
        return Err(AppError::validation(
            format!("ZIP에 그림이 {total}장 있어 넣을 수 없습니다 (한 번에 {MAX_ZIP_IMAGES}장까지). 나눠서 넣어 주세요"),
            None,
        ));
    }
    std::fs::create_dir_all(out_dir)?;
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| AppError::io(e.to_string()))?;
        if f.is_dir() || f.size() > MAX_IMAGE_BYTES {
            continue;
        }
        let Some(name) = std::path::Path::new(f.name()).file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let ext = name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
        if name.starts_with('.') || !["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.as_str()) {
            continue;
        }
        let dirs: Vec<String> = std::path::Path::new(f.name())
            .parent()
            .map(|p| p.iter().map(|c| c.to_string_lossy().trim().to_string()).filter(|c| !c.is_empty()).collect())
            .unwrap_or_default();
        if dirs.iter().any(|d| d.starts_with('.') || d == "__MACOSX") {
            continue; // macOS가 넣는 숨은 파일
        }
        // 판마다 같은 파일 이름이 있을 수 있어 하나씩 따로 푼다
        let dir = out_dir.join(format!("f-{}", out.len()));
        std::fs::create_dir_all(&dir)?;
        let dest = dir.join(&name);
        let mut w = std::fs::File::create(&dest)?;
        std::io::copy(&mut f, &mut w)?;
        out.push((dest, dirs));
    }
    Ok(version_folders(out))
}

/// ZIP 안 폴더 → 판 폴더: 모든 그림이 같은 겉 폴더 안에 있으면(폴더째 압축) 그 겉 폴더를 벗기고,
/// 남은 경로의 첫 폴더가 판이다 (`pack/v1/smile.png` → v1, `pack/cry.png` → 공통)
fn version_folders(items: Vec<(PathBuf, Vec<String>)>) -> Vec<(PathBuf, String)> {
    let mut items = items;
    while let Some(first) = items.first().and_then(|(_, d)| d.first().cloned()) {
        if !items.iter().all(|(_, d)| d.first() == Some(&first)) {
            break;
        }
        for (_, d) in &mut items {
            d.remove(0);
        }
    }
    items
        .into_iter()
        .map(|(p, d)| (p, d.first().map(|f| f.chars().take(40).collect()).unwrap_or_default()))
        .collect()
}

/// 이미지 파일 삭제 (없으면 무시)
pub fn remove_image_files(app_data_dir: &Path, rel_paths: &[&str]) {
    for p in rel_paths {
        let _ = std::fs::remove_file(app_data_dir.join(p));
    }
}

/// 캐릭터 이미지 폴더 통째로 삭제 (캐릭터 삭제 시)
pub fn remove_character_dir(app_data_dir: &Path, character_id: &str) {
    if character_id.is_empty() || character_id.contains(['/', '\\', '.']) {
        return; // 경로 조작 방지
    }
    let _ = std::fs::remove_dir_all(app_data_dir.join("characters").join(character_id));
}

// =======================================
// 프롬프트
// =======================================

/// 모델에게 보낼 이미지 규칙 + 키 목록 ({{char}}는 PromptManager가 치환)
pub fn image_rules(images: &[CharacterImage], preset: &crate::service::app_prompts::PresetPrompts) -> Option<String> {
    // 사용자가 설정 > 앱 프롬프트에서 고칠 수 있다
    image_keys(images).map(|k| crate::service::app_prompts::chat_images(&k, preset))
}

/// 켜진 이미지 키 목록 ("a, b, c", 같은 키는 한 번). 없으면 None ({{img_keys}})
pub fn image_keys(images: &[CharacterImage]) -> Option<String> {
    let mut keys: Vec<&str> = vec![];
    for i in images.iter().filter(|i| i.enabled) {
        if keys.len() >= MAX_PROMPT_KEYS {
            break;
        }
        if !keys.iter().any(|k| k.eq_ignore_ascii_case(&i.key)) {
            keys.push(i.key.as_str());
        }
    }
    (!keys.is_empty()).then(|| keys.join(", "))
}

/// 같은 키 묶기의 키: 끝의 `-숫자`를 뗀다 (`smile-2` → `smile`, `-1`처럼 앞이 비면 그대로)
pub fn base_key(key: &str) -> &str {
    match key.rsplit_once('-') {
        Some((head, tail)) if !head.is_empty() && !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) => head,
        _ => key,
    }
}

/// 같은 키 묶기 (5-E-25): 키를 묶음 키로 바꾼다. 프롬프트에는 묶음 키가 한 번만, 응답의 `<cg>묶음 키</cg>`는 그대로 남고
/// 어느 그림을 보일지는 화면이 그 묶음에서 메시지마다 고른다
pub fn group_variants(images: Vec<CharacterImage>) -> Vec<CharacterImage> {
    images
        .into_iter()
        .map(|mut i| {
            i.key = base_key(&i.key).to_string();
            i
        })
        .collect()
}

// =======================================
// 응답 처리
// =======================================

/// 응답의 `<cg>키</cg>`를 검증한다.
/// - 목록에 있는 키(대소문자·앞뒤 공백·따옴표 무시) → 등록된 키 표기로 바꿔서 남김
/// - 없는 키 / 빈 태그 → 태그째 지움
/// - 닫히지 않은 `<cg>` (응답이 잘린 경우 등) → 그 줄 끝까지 지움
pub fn sanitize_cg_tags(text: &str, images: &[CharacterImage]) -> (String, Vec<ChatImageRef>) {
    let mut refs = vec![];
    let out = rewrite_cg_tags(text, |key| {
        let wanted = key.trim_matches(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`'));
        let found = images
            .iter()
            .filter(|i| i.enabled)
            .find(|i| i.key.eq_ignore_ascii_case(wanted) || i.key.to_lowercase() == wanted.to_lowercase())?;
        refs.push(ChatImageRef {
            key: found.key.clone(),
            image_id: found.id.clone(),
            path: found.path.clone(),
        });
        Some(format!("{OPEN}{}{CLOSE}", found.key))
    });
    (out, refs)
}

/// `<cg>…</cg>` 태그를 모두 지운다 (이미지 기능이 꺼진 대화의 기록을 프롬프트에 넣을 때)
pub fn strip_cg_tags(text: &str) -> String {
    rewrite_cg_tags(text, |_| None)
}

/// `<cg>`(대소문자 무시) 태그마다 `replace(key)`를 불러, Some이면 그 문자열로, None이면 태그를 지운다
fn rewrite_cg_tags(text: &str, mut replace: impl FnMut(&str) -> Option<String>) -> String {
    // ASCII 소문자화는 바이트 위치를 바꾸지 않으므로 같은 인덱스를 원문에 그대로 쓸 수 있다
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while let Some(rel) = lower[pos..].find(OPEN) {
        let start = pos + rel;
        out.push_str(&text[pos..start]);
        let key_start = start + OPEN.len();
        let line_end = lower[key_start..].find('\n').map(|i| key_start + i).unwrap_or(text.len());
        match lower[key_start..line_end].find(CLOSE) {
            Some(rel_close) => {
                let key_end = key_start + rel_close;
                let key = &text[key_start..key_end];
                if let Some(rep) = replace(key) {
                    out.push_str(&rep);
                }
                pos = key_end + CLOSE.len();
            }
            None => {
                // 닫는 태그 없음: 줄 끝까지 버림
                pos = line_end;
            }
        }
    }
    out.push_str(&text[pos..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_with(dir: &Path, names: &[String]) -> PathBuf {
        use std::io::Write;
        let path = dir.join("pack.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for n in names {
            w.start_file(n.as_str(), opts).unwrap();
            w.write_all(b"x").unwrap();
        }
        w.finish().unwrap();
        path
    }

    #[test]
    fn zip_over_the_limit_is_rejected_before_extracting() {
        let dir = std::env::temp_dir().join(format!("sc-zip-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut names: Vec<String> = (0..=MAX_ZIP_IMAGES).map(|i| format!("a/{i}.png")).collect();
        names.push("readme.txt".into()); // 그림이 아닌 파일은 세지 않음
        let zip = zip_with(&dir, &names);
        let out = dir.join("out");
        assert!(extract_zip_images(&zip, &out).is_err());
        assert!(!out.exists(), "거절하면 아무것도 풀지 않음");
        // 딱 한도면 통과 (내용은 그림이 아니어도 파일은 풀림)
        let ok = zip_with(&dir, &names[..MAX_ZIP_IMAGES].to_vec());
        assert_eq!(extract_zip_images(&ok, &out).unwrap().len(), MAX_ZIP_IMAGES);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn zip_subfolders_become_version_folders_but_a_single_wrapper_does_not() {
        let dir = std::env::temp_dir().join(format!("sc-zip-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let folders = |names: &[&str]| {
            let zip = zip_with(&dir, &names.iter().map(|s| s.to_string()).collect::<Vec<_>>());
            let out = dir.join(format!("out-{}", Uuid::new_v4()));
            let mut f: Vec<String> = extract_zip_images(&zip, &out).unwrap().into_iter().map(|(_, f)| f).collect();
            f.sort();
            f
        };
        assert_eq!(folders(&["pack/v1/smile.png", "pack/v2/smile.png", "pack/cry.png"]), vec!["", "v1", "v2"]);
        assert_eq!(folders(&["smile.png", "v2/smile.png"]), vec!["", "v2"]);
        assert_eq!(
            folders(&["내 캐릭터/smile.png", "내 캐릭터/cry.png"]),
            vec!["", ""],
            "폴더째 압축한 겉 폴더는 판이 아님"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    fn img(id: &str, key: &str, enabled: bool) -> CharacterImage {
        CharacterImage {
            id: id.into(),
            character_id: "c".into(),
            key: key.into(),
            path: format!("/data/{id}.webp"),
            thumbnail_path: None,
            enabled,
            sort_order: 0,
            created_at: 0,
            updated_at: 0,
            folder: String::new(),
        }
    }

    #[test]
    fn sanitize_keeps_known_keys_and_drops_others() {
        let images = vec![img("1", "Smile", true), img("2", "울음", true), img("3", "secret", false)];
        let (text, refs) = sanitize_cg_tags(
            "안녕! <CG> smile </cg>\n흑 <cg>\"울음\"</cg> <cg>angry</cg><cg>secret</cg><cg></cg> 끝 <cg>잘린",
            &images,
        );
        assert_eq!(text, "안녕! <cg>Smile</cg>\n흑 <cg>울음</cg>  끝 ");
        let keys: Vec<_> = refs.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["Smile", "울음"]);
        assert_eq!(refs[0].image_id, "1");
    }

    #[test]
    fn unclosed_tag_only_removes_its_line() {
        assert_eq!(strip_cg_tags("a <cg>smi\nnext line"), "a \nnext line");
        assert_eq!(strip_cg_tags("x<cg>a</cg>y<cg>b</cg>z"), "xyz");
        assert_eq!(strip_cg_tags("no tags"), "no tags");
    }

    #[test]
    fn rules_list_enabled_keys_only() {
        assert!(image_rules(&[img("1", "a", false)], &Default::default()).is_none());
        let rules = image_rules(&[img("1", "smile", true), img("2", "off", false), img("3", "cry", true)], &Default::default()).unwrap();
        assert!(rules.ends_with("Available keys: smile, cry"), "{rules}");
        assert!(rules.contains("{{char}}"));
    }

    #[test]
    fn stores_webp_and_thumbnail_and_rejects_non_images() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("Smile.png");
        image::RgbImage::from_pixel(600, 400, image::Rgb([200, 100, 50])).save(&src).unwrap();

        let base = dir.path().join("appdata");
        let (file, thumb) = store_image_file(&base, "c1", "img1", &src).unwrap();
        assert_eq!(file, "characters/c1/images/img1.webp");
        assert_eq!(thumb, "characters/c1/images/thumb/img1.webp");
        let t = image::open(base.join(&thumb)).unwrap();
        assert!(t.width() <= THUMBNAIL_SIZE && t.height() <= THUMBNAIL_SIZE);
        assert!(base.join(&file).is_file());

        let txt = dir.path().join("note.txt");
        std::fs::write(&txt, "not an image").unwrap();
        assert!(store_image_file(&base, "c1", "img2", &txt).is_err());

        remove_character_dir(&base, "c1");
        assert!(!base.join("characters/c1").exists());
    }

    #[test]
    fn key_rules() {
        assert_eq!(normalize_key("  big   smile ").unwrap(), "big smile");
        assert!(normalize_key("a<b").is_err());
        assert!(normalize_key("   ").is_err());
        assert!(normalize_key(&"가".repeat(41)).is_err());
        assert_eq!(key_from_path(Path::new("C:/x/Smile, big.PNG")), "Smile_ big");
        assert_eq!(key_from_path(Path::new("/x/.png")), ".png");
    }
}

#[cfg(test)]
mod group_tests {
    use super::*;

    #[test]
    fn variants_share_a_base_key() {
        assert_eq!(base_key("smile-1"), "smile");
        assert_eq!(base_key("smile-12"), "smile");
        assert_eq!(base_key("smile"), "smile");
        assert_eq!(base_key("blue-eyes"), "blue-eyes");
        assert_eq!(base_key("-1"), "-1");
        assert_eq!(base_key("cg-"), "cg-");
    }
}
