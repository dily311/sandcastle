//! 동기화 저장소 (CLAUDE.md 2-C-1, ADR-011).
//!
//! 저장소는 이름 → 파일만 다룬다(list / get / put / delete). 폴더(2-C)와 구글 드라이브 앱 전용 폴더(2-D)가 같은 모양을
//! 따르고, 무엇을 어디에 둘지는 `sync_run`이 정한다:
//!
//! ```text
//! devices/<기기 id>.scsync   기기마다 자기 묶음 하나 (자기 것만 쓴다 — 두 기기가 같은 파일을 쓰지 않아 동기화 앱과 다투지 않음)
//! blobs/<sha256>             파일 내용 (한 번 올리면 모든 기기가 같이 씀)
//! ```
//!
//! 이름은 `/`로 나눈 상대 경로만 받는다(`..`·절대 경로·드라이브 문자 거부).
use std::path::{Path, PathBuf};

use crate::error::{AppError, ErrorCode, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteEntry {
    /// 저장소 안 이름 (`devices/…`)
    pub name: String,
    pub size: u64,
}

#[allow(async_fn_in_trait)] // 앱 안에서만 쓰고 dyn으로 쓰지 않는다
pub trait SyncTarget {
    /// 기록·화면용 이름 ("folder", "drive")
    fn kind(&self) -> &'static str;
    /// `prefix`(예: "devices/") 바로 아래의 파일
    async fn list(&self, prefix: &str) -> Result<Vec<RemoteEntry>>;
    /// 저장소의 파일을 `dest`로 내려받는다
    async fn get(&self, name: &str, dest: &Path) -> Result<()>;
    /// `src`를 저장소에 올린다 (있으면 바꾼다)
    async fn put(&self, name: &str, src: &Path) -> Result<()>;
    async fn delete(&self, name: &str) -> Result<()>;
}

fn check_name(name: &str) -> Result<PathBuf> {
    let p = Path::new(name);
    let ok = !name.is_empty() && !name.contains(':') && !name.contains('\\') && p.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    if !ok {
        return Err(AppError::archive(ErrorCode::ArchiveUnsafePath, format!("잘못된 동기화 파일 이름: {name}")));
    }
    Ok(p.to_path_buf())
}

/// 사용자가 고른 폴더 (그 안의 `sandcastle-sync/`). OneDrive·Dropbox처럼 이미 쓰는 동기화 앱이 다른 기기로 옮긴다
pub struct FolderTarget {
    root: PathBuf,
}

pub const FOLDER_NAME: &str = "sandcastle-sync";

impl FolderTarget {
    /// 고른 폴더. 없으면 오류 (저장소를 마음대로 만들지 않는다 — 안의 `sandcastle-sync/`만 만든다)
    pub fn new(chosen: &Path) -> Result<Self> {
        if !chosen.is_dir() {
            return Err(AppError::validation(format!("동기화 폴더가 없음: {}", chosen.display()), None));
        }
        let root = chosen.join(FOLDER_NAME);
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl SyncTarget for FolderTarget {
    fn kind(&self) -> &'static str {
        "folder"
    }

    async fn list(&self, prefix: &str) -> Result<Vec<RemoteEntry>> {
        let dir = self.root.join(check_name(prefix.trim_end_matches('/'))?);
        let Ok(read) = std::fs::read_dir(&dir) else { return Ok(vec![]) };
        let mut out = vec![];
        for e in read.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            let name = e.file_name().to_string_lossy().to_string();
            // 쓰는 중인 파일·동기화 앱의 임시 파일은 빼고
            if !meta.is_file() || name.ends_with(".partial") || name.starts_with('.') || name.starts_with('~') {
                continue;
            }
            out.push(RemoteEntry {
                name: format!("{}/{name}", prefix.trim_end_matches('/')),
                size: meta.len(),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    async fn get(&self, name: &str, dest: &Path) -> Result<()> {
        let src = self.root.join(check_name(name)?);
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(&src, dest).map_err(|e| AppError::io(format!("동기화 폴더에서 읽지 못함 ({name}): {e}")))?;
        Ok(())
    }

    async fn put(&self, name: &str, src: &Path) -> Result<()> {
        let dest = self.root.join(check_name(name)?);
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p)?;
        }
        // 다른 이름으로 다 쓴 뒤 바꿔 넣는다 — 동기화 앱이 반쯤 쓴 파일을 올리지 않게
        let tmp = dest.with_file_name(format!("{}.partial", dest.file_name().unwrap_or_default().to_string_lossy()));
        std::fs::copy(src, &tmp).map_err(|e| AppError::io(format!("동기화 폴더에 쓰지 못함 ({name}): {e}")))?;
        std::fs::rename(&tmp, &dest)?;
        Ok(())
    }

    async fn delete(&self, name: &str) -> Result<()> {
        let p = self.root.join(check_name(name)?);
        if p.exists() {
            std::fs::remove_file(p)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn folder_target_round_trip_and_rejects_bad_names() {
        let dir = tempfile::tempdir().unwrap();
        assert!(FolderTarget::new(&dir.path().join("없음")).is_err(), "없는 폴더는 만들지 않음");
        let t = FolderTarget::new(dir.path()).unwrap();
        assert!(t.root().ends_with(FOLDER_NAME));
        let src = dir.path().join("a.bin");
        std::fs::write(&src, b"hello").unwrap();
        t.put("devices/d1.scsync", &src).await.unwrap();
        t.put("blobs/abc", &src).await.unwrap();
        std::fs::write(t.root().join("devices/d2.scsync.partial"), b"half").unwrap();
        let list = t.list("devices/").await.unwrap();
        assert_eq!(
            list,
            vec![RemoteEntry {
                name: "devices/d1.scsync".into(),
                size: 5
            }],
            "쓰는 중인 파일은 빼고"
        );
        let out = dir.path().join("got/b.bin");
        t.get("devices/d1.scsync", &out).await.unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"hello");
        t.delete("blobs/abc").await.unwrap();
        assert!(t.list("blobs/").await.unwrap().is_empty());
        assert!(t.list("없는것/").await.unwrap().is_empty());
        for bad in ["../x", "/abs", "C:/x", "a\\b", ""] {
            assert!(t.put(bad, &src).await.is_err(), "{bad}");
        }
    }
}
