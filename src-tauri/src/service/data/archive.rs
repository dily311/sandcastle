//! 백업/팩 zip 공용 도우미
use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

use crate::error::{AppError, ErrorCode, Result};

pub fn zip_err(e: impl std::fmt::Display) -> AppError {
    AppError::archive(ErrorCode::ArchiveInvalid, e.to_string())
}

pub fn options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true)
}

pub fn write_json<W: Write + Seek>(zip: &mut ZipWriter<W>, name: &str, value: &impl Serialize) -> Result<()> {
    zip.start_file(name, options()).map_err(zip_err)?;
    let bytes = serde_json::to_vec_pretty(value)?;
    zip.write_all(&bytes)?;
    Ok(())
}

pub fn write_file<W: Write + Seek>(zip: &mut ZipWriter<W>, name: &str, source: &Path) -> Result<()> {
    zip.start_file(name, options()).map_err(zip_err)?;
    let mut f = File::open(source)?;
    std::io::copy(&mut f, zip)?;
    Ok(())
}

/// dir 아래 파일을 `prefix/상대경로`로 담는다. 담은 파일 수와 바이트를 돌려준다. 없는 폴더는 건너뜀.
pub fn write_dir<W: Write + Seek>(zip: &mut ZipWriter<W>, dir: &Path, prefix: &str) -> Result<(u64, u64)> {
    let mut stats = (0, 0);
    if !dir.is_dir() {
        return Ok(stats);
    }
    for path in walk_files(dir)? {
        let rel = path
            .strip_prefix(dir)
            .map_err(|e| AppError::internal(e.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        write_file(zip, &format!("{prefix}/{rel}"), &path)?;
        stats.0 += 1;
        stats.1 += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    }
    Ok(stats)
}

/// 폴더 아래 모든 파일 (심볼릭 링크는 따라가지 않음)
pub fn walk_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            if ft.is_dir() {
                stack.push(entry.path());
            } else if ft.is_file() {
                out.push(entry.path());
            }
        }
    }
    out.sort();
    Ok(out)
}

pub fn dir_size(dir: &Path) -> u64 {
    walk_files(dir)
        .unwrap_or_default()
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum()
}

pub fn open_archive(path: &Path) -> Result<ZipArchive<File>> {
    let f = File::open(path).map_err(|e| AppError::io(format!("파일을 열 수 없습니다: {e}")))?;
    ZipArchive::new(f).map_err(zip_err)
}

pub fn read_json<R: Read + Seek, T: DeserializeOwned>(archive: &mut ZipArchive<R>, name: &str) -> Result<T> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| AppError::validation(format!("{name}이(가) 없습니다"), None))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    serde_json::from_slice(&buf).map_err(|e| AppError::validation(format!("{name}을(를) 읽을 수 없습니다: {e}"), None))
}

/// zip 안의 상대 경로가 안전한지 (절대 경로, `..`, 드라이브 문자 거부)
pub fn safe_relative(name: &str) -> Result<PathBuf> {
    let p = Path::new(name);
    // ':'는 Windows 드라이브(C:)·대체 데이터 스트림(a.txt:x) 방지 — 다른 OS에서도 같은 판정이 되게 문자로 막는다
    let ok = !name.is_empty()
        && !name.contains(':')
        && p.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    if !ok {
        return Err(AppError::archive(ErrorCode::ArchiveUnsafePath, format!("잘못된 경로: {name}")));
    }
    Ok(p.to_path_buf())
}

/// `prefix/`로 시작하는 항목을 dest 아래에 푼다 (prefix는 떼고). 경로 탈출은 거부.
pub fn extract_prefix<R: Read + Seek>(archive: &mut ZipArchive<R>, prefix: &str, dest: &Path) -> Result<u64> {
    let mut count = 0;
    let pre = format!("{}/", prefix.trim_end_matches('/'));
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_err)?;
        let name = entry.name().replace('\\', "/");
        let Some(rel) = name.strip_prefix(&pre) else { continue };
        if rel.is_empty() || entry.is_dir() {
            continue;
        }
        if entry.enclosed_name().is_none() {
            return Err(AppError::archive(ErrorCode::ArchiveUnsafePath, format!("잘못된 경로: {name}")));
        }
        let out = dest.join(safe_relative(rel)?);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = File::create(&out)?;
        std::io::copy(&mut entry, &mut f)?;
        count += 1;
    }
    Ok(count)
}

/// 이름 하나를 파일로 푼다
pub fn extract_file<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str, out: &Path) -> Result<()> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| AppError::validation(format!("{name}이(가) 없습니다"), None))?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = File::create(out)?;
    std::io::copy(&mut entry, &mut f)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_paths() {
        assert!(safe_relative("a/b.png").is_ok());
        for bad in ["../x", "/etc/passwd", "a/../../x", "C:/x", ""] {
            assert!(safe_relative(bad).is_err(), "{bad}");
        }
    }
}
