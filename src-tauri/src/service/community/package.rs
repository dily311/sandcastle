//! 플러그인 묶음 읽기: zip 파일이나 폴더 → 검사한 파일 목록 (CLAUDE.md 6-D-1, ADR-012).
//!
//! 디스크에 쓰기 전에 전부 메모리에서 검사한다: 경로(zip slip·절대 경로·심볼릭 링크), 파일 종류, 개수·크기 상한,
//! manifest. 통과한 것만 설치(`super::install`)가 쓴다.
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use super::manifest::{self, safe_rel_path, PluginManifest};
use crate::error::{AppError, Result};

pub const MANIFEST: &str = "manifest.json";
/// 플러그인 하나의 파일 수·전체 크기 상한
pub const MAX_FILES: usize = 300;
pub const MAX_TOTAL_BYTES: u64 = 5 * 1024 * 1024;
/// zip 안의 항목 수 상한 (폴더·건너뛰는 파일 포함 — 항목이 수만 개인 zip을 끝까지 읽지 않게)
const MAX_ENTRIES: usize = 2000;

/// 넣을 수 있는 파일 종류. 실행 파일·라이브러리 같은 것은 받지 않는다
const ALLOWED_EXT: [&str; 17] = [
    "js", "mjs", "css", "html", "json", "md", "txt", "png", "jpg", "jpeg", "webp", "gif", "svg", "woff", "woff2", "ttf", "otf",
];

#[derive(Debug, Clone)]
pub struct Package {
    pub manifest: PluginManifest,
    /// 상대 경로(`/` 구분) → 내용. 정렬됨
    pub files: BTreeMap<String, Vec<u8>>,
    pub size: u64,
    /// 경로와 내용 전체의 sha256 (미리보기와 설치 사이에 파일이 바뀌지 않았는지 확인)
    pub hash: String,
}

fn bad(msg: impl Into<String>) -> AppError {
    AppError::validation(format!("플러그인 묶음: {}", msg.into()), None)
}

/// 설치하지 않고 건너뛰는 것: 숨김 파일·폴더(.git 등), macOS·Windows가 만드는 군더더기
fn skipped(rel: &str) -> bool {
    rel.split('/')
        .any(|s| s.starts_with('.') || s == "__MACOSX" || s.eq_ignore_ascii_case("Thumbs.db"))
}

fn allowed(rel: &str) -> bool {
    Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ALLOWED_EXT.iter().any(|a| a.eq_ignore_ascii_case(e)))
}

/// 파일 하나를 목록에 넣는다 (경로·종류·상한 검사)
fn push(files: &mut BTreeMap<String, Vec<u8>>, total: &mut u64, rel: String, bytes: Vec<u8>) -> Result<()> {
    if !safe_rel_path(&rel) {
        return Err(bad(format!("허용되지 않는 경로: {rel:?}")));
    }
    if skipped(&rel) {
        return Ok(());
    }
    if !allowed(&rel) {
        return Err(bad(format!("넣을 수 없는 파일 종류: {rel} (js·css·html·json·그림·글꼴만)")));
    }
    *total += bytes.len() as u64;
    if *total > MAX_TOTAL_BYTES {
        return Err(bad(format!("너무 큼 (최대 {}MB)", MAX_TOTAL_BYTES / 1024 / 1024)));
    }
    if files.insert(rel.clone(), bytes).is_some() {
        return Err(bad(format!("같은 경로가 두 번 있음: {rel}")));
    }
    if files.len() > MAX_FILES {
        return Err(bad(format!("파일이 너무 많음 (최대 {MAX_FILES}개)")));
    }
    Ok(())
}

fn read_zip(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| bad(format!("zip을 열 수 없음 ({e})")))?;
    if zip.len() > MAX_ENTRIES {
        return Err(bad(format!("zip 항목이 너무 많음 (최대 {MAX_ENTRIES}개)")));
    }
    let mut files = BTreeMap::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| bad(format!("zip 항목을 읽을 수 없음 ({e})")))?;
        let name = entry.name().trim_start_matches("./").to_string();
        if entry.is_dir() {
            continue;
        }
        if entry.is_symlink() {
            return Err(bad(format!("링크는 넣을 수 없음: {name}")));
        }
        // 경로 검사가 먼저 — `..`도 점으로 시작해 건너뛰기에 걸리지만, 그런 묶음은 조용히 넘기지 않고 거절한다
        if name.contains('\\') || !safe_rel_path(&name) {
            return Err(bad(format!("허용되지 않는 경로: {name:?}")));
        }
        if skipped(&name) {
            continue;
        }
        // 헤더의 크기를 믿지 않고 상한 + 1까지만 읽는다 (압축 폭탄)
        let left = MAX_TOTAL_BYTES.saturating_sub(total) + 1;
        let mut bytes = vec![];
        (&mut entry)
            .take(left)
            .read_to_end(&mut bytes)
            .map_err(|e| bad(format!("{name}을 읽을 수 없음 ({e})")))?;
        push(&mut files, &mut total, name, bytes)?;
    }
    Ok(files)
}

fn read_dir_tree(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let mut total = 0u64;
    let mut seen = 0usize;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            seen += 1;
            if seen > MAX_ENTRIES {
                return Err(bad(format!("폴더 안 항목이 너무 많음 (최대 {MAX_ENTRIES}개)")));
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .map_err(|_| bad("폴더 밖의 경로"))?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if skipped(&rel) {
                continue;
            }
            // 링크(심볼릭·정션)는 따라가지 않는다 — 폴더 밖 파일을 끌어오지 않게
            let meta = std::fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                return Err(bad(format!("링크는 넣을 수 없음: {rel}")));
            }
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                if meta.len() > MAX_TOTAL_BYTES {
                    return Err(bad(format!("너무 큼 (최대 {}MB)", MAX_TOTAL_BYTES / 1024 / 1024)));
                }
                push(&mut files, &mut total, rel, std::fs::read(&path)?)?;
            }
        }
    }
    Ok(files)
}

/// manifest가 맨 위가 아니라 폴더 하나 안에 있으면(`my-plugin/manifest.json` — zip으로 묶을 때 흔함) 그 폴더를 벗긴다
fn strip_single_root(files: BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    if files.contains_key(MANIFEST) {
        return files;
    }
    let Some(first) = files.keys().next().and_then(|k| k.split_once('/')).map(|(top, _)| top.to_string()) else {
        return files;
    };
    let prefix = format!("{first}/");
    if !files.contains_key(&format!("{prefix}{MANIFEST}")) || !files.keys().all(|k| k.starts_with(&prefix)) {
        return files;
    }
    files.into_iter().map(|(k, v)| (k[prefix.len()..].to_string(), v)).collect()
}

pub fn hash_files(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut h = Sha256::new();
    for (path, bytes) in files {
        h.update(path.as_bytes());
        h.update([0]);
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(bytes);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// zip 파일이나 폴더를 읽어 검사한다. 아무것도 쓰지 않는다.
pub fn read(path: &Path, app_version: &str) -> Result<Package> {
    let meta = std::fs::metadata(path).map_err(|e| AppError::io(format!("플러그인을 열 수 없음: {e}")))?;
    let files = if meta.is_dir() { read_dir_tree(path)? } else { read_zip(path)? };
    let files = strip_single_root(files);
    let raw = files.get(MANIFEST).ok_or_else(|| bad("맨 위에 manifest.json이 없음"))?;
    let manifest = manifest::parse(raw, app_version)?;
    if !files.contains_key(&manifest.main) {
        return Err(bad(format!("main 파일이 없음: {}", manifest.main)));
    }
    let size = files.values().map(|b| b.len() as u64).sum();
    let hash = hash_files(&files);
    Ok(Package { manifest, files, size, hash })
}
