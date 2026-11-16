//! 데이터 점검 (CLAUDE.md 1-B-3): DB 무결성과 아무 데도 쓰이지 않는 파일.
//!
//! 보여주기만 하고 지우지 않는다 (Reversibility). 고아 파일은 `characters/` 아래에서 찾는다:
//! 아바타·커버(캐릭터·페르소나의 절대 경로)와 캐릭터 이미지(앱 데이터 기준 상대 경로)가 가리키지 않는 파일.
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{db::conn::DbPool, error::Result};

/// 목록에 보여줄 최대 개수 (개수·크기 합계는 전부 셈)
const LIST_LIMIT: usize = 500;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OrphanFile {
    /// 앱 데이터 폴더 기준 (슬래시)
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DataCheck {
    /// PRAGMA integrity_check 결과가 "ok"
    pub ok: bool,
    /// 문제가 있으면 그 내용 (최대 100줄)
    pub problems: Vec<String>,
    /// 살펴본 파일 수 (characters/)
    pub checked_files: u32,
    pub orphan_count: u32,
    pub orphan_bytes: u64,
    /// 최대 500개
    pub orphans: Vec<OrphanFile>,
}

/// 앱 데이터 기준 상대 경로 (슬래시). 앱 데이터 밖이면 None
fn relative(base: &Path, p: &str) -> Option<String> {
    let path = Path::new(p);
    let rel = if path.is_absolute() { path.strip_prefix(base).ok()? } else { path };
    Some(rel.to_string_lossy().replace('\\', "/"))
}

async fn referenced(db: &DbPool, base: &Path) -> Result<HashSet<String>> {
    let mut paths: Vec<String> = vec![];
    for sql in [
        "SELECT avatar FROM characters WHERE avatar IS NOT NULL",
        "SELECT cover FROM characters WHERE cover IS NOT NULL",
        "SELECT avatar FROM personas WHERE avatar IS NOT NULL",
        "SELECT file_path FROM character_images",
        "SELECT thumb_path FROM character_images WHERE thumb_path IS NOT NULL",
    ] {
        paths.extend(sqlx::query_scalar::<_, String>(sql).fetch_all(db).await?);
    }
    Ok(paths.iter().filter_map(|p| relative(base, p)).collect())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for e in read.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

pub async fn check_data(db: &DbPool, base: &Path) -> Result<DataCheck> {
    let rows: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check(100)").fetch_all(db).await?;
    let ok = rows.len() == 1 && rows[0] == "ok";
    let used = referenced(db, base).await?;

    let (root, base_owned) = (base.join("characters"), base.to_path_buf());
    let files = tokio::task::spawn_blocking(move || {
        let mut v = vec![];
        walk(&root, &mut v);
        v.into_iter()
            .filter_map(|p| {
                let rel = p.strip_prefix(&base_owned).ok()?.to_string_lossy().replace('\\', "/");
                Some((rel, std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0)))
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| crate::error::AppError::internal(e.to_string()))?;

    let mut orphans: Vec<OrphanFile> = files
        .iter()
        .filter(|(rel, _)| !used.contains(rel))
        .map(|(path, bytes)| OrphanFile {
            path: path.clone(),
            bytes: *bytes,
        })
        .collect();
    orphans.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
    Ok(DataCheck {
        ok,
        problems: if ok { vec![] } else { rows },
        checked_files: n(files.len()),
        orphan_count: n(orphans.len()),
        orphan_bytes: orphans.iter().map(|o| o.bytes).sum(),
        orphans: orphans.into_iter().take(LIST_LIMIT).collect(),
    })
}
