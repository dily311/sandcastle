//! 고른 파일 · 저장 위치를 실제 경로로 (CLAUDE.md 1-C-2).
//!
//! Android의 파일 고르기는 경로 대신 `content://` 주소를 돌려준다. 앱의 가져오기·내보내기 코드는 모두
//! 파일 경로(std::fs)를 쓰므로, 여기서 한 번만 바꾼다:
//! - 열기: content 주소를 `tmp/picked/`에 복사하고 그 경로를 돌려준다. 주소에는 파일 이름이 없어 확장자는 내용으로 정한다
//! - 저장: `tmp/saving/`에 먼저 쓰게 하고, 끝나면 고른 content 주소로 복사한다
//!
//! 데스크톱은 늘 실제 경로라 프론트가 이 커맨드를 부르지 않는다.
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use tauri::{AppHandle, Runtime};
use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};
use uuid::Uuid;

use crate::error::{AppError, Result};

pub fn is_content_uri(s: &str) -> bool {
    s.starts_with("content://")
}

/// 파일 내용으로 확장자를 고른다. `allowed`(고르기 필터의 확장자) 안에서만 고르고, 모르겠으면 첫 번째
pub fn sniff_extension(head: &[u8], allowed: &[String]) -> String {
    let has = |e: &str| allowed.iter().any(|a| a.eq_ignore_ascii_case(e));
    let first = || allowed.first().cloned().unwrap_or_else(|| "bin".into());
    let image = if head.starts_with(b"\x89PNG") {
        Some("png")
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(if has("jpg") { "jpg" } else { "jpeg" })
    } else if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some("webp")
    } else if head.starts_with(b"GIF8") {
        Some("gif")
    } else if head.len() >= 12 && &head[4..8] == b"ftyp" && (&head[8..12] == b"avif" || &head[8..12] == b"avis") {
        Some("avif")
    } else {
        None
    };
    if let Some(e) = image {
        return if has(e) { e.into() } else { first() };
    }
    // zip 계열 (.zip .scpack .scbackup .tgame) — 필터가 고른 것
    if head.starts_with(b"PK\x03\x04") {
        return ["scbackup", "scpack", "tgame", "zip"]
            .into_iter()
            .find(|e| has(e))
            .map_or_else(first, str::to_string);
    }
    // 글: JSON 한 덩어리 / 줄마다 JSON(jsonl)
    let text = String::from_utf8_lossy(head);
    let t = text.trim_start_matches('\u{feff}').trim_start();
    if t.starts_with('{') || t.starts_with('[') {
        let mut lines = t.lines().filter(|l| !l.trim().is_empty());
        let jsonl = matches!((lines.next(), lines.next()), (Some(a), Some(b)) if a.trim_end().ends_with('}') && b.trim_start().starts_with('{'));
        if jsonl && has("jsonl") {
            return "jsonl".into();
        }
        if has("json") {
            return "json".into();
        }
    }
    first()
}

fn tmp_dir(app_data_dir: &Path, sub: &str) -> Result<PathBuf> {
    let dir = app_data_dir.join("tmp").join(sub);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn parse(uri: &str) -> Result<FilePath> {
    uri.parse::<FilePath>()
        .map_err(|e| AppError::validation(format!("잘못된 파일 주소: {e}"), None))
}

/// 고른 content 주소를 앱 임시 폴더로 복사하고 경로를 돌려준다 (경로면 그대로).
/// 수 GB짜리 SillyTavern zip·백업 파일도 메모리에 다 올리지 않도록 앞 4KB만 읽어 확장자를 판별한 뒤 스트리밍 복사한다.
pub fn stage_picked<R: Runtime>(app: &AppHandle<R>, app_data_dir: &Path, uri: &str, allowed: &[String]) -> Result<String> {
    if !is_content_uri(uri) {
        return Ok(uri.to_string());
    }
    let mut opts = OpenOptions::new();
    opts.read(true);
    let mut src = app
        .fs()
        .open(parse(uri)?, opts)
        .map_err(|e| AppError::io(format!("고른 파일을 열 수 없습니다: {e}")))?;
    let mut head = [0u8; 4096];
    let mut head_len = 0usize;
    while head_len < head.len() {
        let n = src.read(&mut head[head_len..])?;
        if n == 0 {
            break;
        }
        head_len += n;
    }
    let ext = sniff_extension(&head[..head_len], allowed);
    let out = tmp_dir(app_data_dir, "picked")?.join(format!("{}.{ext}", Uuid::new_v4()));
    let mut writer = std::io::BufWriter::new(std::fs::File::create(&out)?);
    if head_len > 0 {
        writer.write_all(&head[..head_len])?;
    }
    std::io::copy(&mut src, &mut writer)?;
    writer.flush()?;
    Ok(out.to_string_lossy().to_string())
}

/// 저장할 임시 경로 (이름은 기본 파일 이름 그대로 — 내보내기가 확장자로 형식을 정하는 곳이 있어서)
pub fn save_target(app_data_dir: &Path, file_name: &str) -> Result<String> {
    let name = Path::new(file_name)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "export".into());
    let dir = tmp_dir(app_data_dir, &format!("saving/{}", Uuid::new_v4()))?;
    Ok(dir.join(name).to_string_lossy().to_string())
}

/// 임시 파일을 고른 content 주소로 스트리밍 복사하고 임시 파일은 지운다
pub fn deliver_saved<R: Runtime>(app: &AppHandle<R>, app_data_dir: &Path, tmp: &str, uri: &str) -> Result<()> {
    let tmp_path = PathBuf::from(tmp);
    // 앱 임시 폴더 안의 파일만 (다른 파일을 내보내는 데 쓰이지 않게)
    let root = app_data_dir.join("tmp").join("saving");
    if !tmp_path.starts_with(&root) {
        return Err(AppError::validation("임시 저장 파일이 아닙니다", None));
    }
    let mut src = std::io::BufReader::new(std::fs::File::open(&tmp_path)?);
    let mut opts = OpenOptions::new();
    opts.write(true).truncate(true).create(true);
    let dst = app
        .fs()
        .open(parse(uri)?, opts)
        .map_err(|e| AppError::io(format!("저장할 곳을 열 수 없습니다: {e}")))?;
    let mut writer = std::io::BufWriter::new(dst);
    std::io::copy(&mut src, &mut writer)?;
    writer.flush()?;
    drop(src);
    if let Some(dir) = tmp_path.parent() {
        let _ = std::fs::remove_dir_all(dir);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exts(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sniffs_by_content_within_filter() {
        let st = exts(&["png", "json", "jsonl"]);
        assert_eq!(sniff_extension(b"\x89PNG\r\n\x1a\n....", &st), "png");
        assert_eq!(sniff_extension(b"  {\"name\":\"a\"}", &st), "json");
        assert_eq!(sniff_extension(b"{\"user_name\":\"u\"}\n{\"mes\":\"hi\"}\n", &st), "jsonl");
        assert_eq!(sniff_extension(b"\xef\xbb\xbf{\"a\":1}", &st), "json");
        assert_eq!(sniff_extension(b"PK\x03\x04....", &exts(&["scbackup"])), "scbackup");
        assert_eq!(sniff_extension(b"PK\x03\x04....", &exts(&["zip"])), "zip");
        assert_eq!(sniff_extension(&[0xFF, 0xD8, 0xFF, 0xE0], &exts(&["png", "jpg", "jpeg"])), "jpg");
        assert_eq!(sniff_extension(b"RIFF\0\0\0\0WEBPVP8 ", &exts(&["png", "webp"])), "webp");
        // 모르는 내용은 필터의 첫 확장자
        assert_eq!(sniff_extension(b"hello", &exts(&["txt", "md"])), "txt");
        assert!(is_content_uri("content://com.android.providers.downloads.documents/document/msf%3A36"));
        assert!(!is_content_uri("C:\\Users\\a.png"));
    }

    #[test]
    fn save_target_keeps_file_name_inside_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let p = save_target(dir.path(), "../../evil/백업.scbackup").unwrap();
        assert!(Path::new(&p).starts_with(dir.path().join("tmp").join("saving")));
        assert!(p.ends_with("백업.scbackup"));
    }
}
