//! 동기화 묶음 형식 (CLAUDE.md 2-B-1, ADR-011).
//!
//! 기기 사이에 주고받는 zip 파일(`.scsync`). 백업(`.scbackup`)과 달리 DB 통째가 아니라 **행 단위**라 받는 쪽이 항목별로
//! 합칠 수 있다(2-B-2).
//!
//! ```text
//! manifest.json            형식·스키마 버전, 보낸 기기 id, 만든 시각, 기준 시각(since), 개수
//! tables/<테이블>.jsonl     바뀐 행 (한 줄에 한 행, {열: 값}). 동기화 대상 테이블만 (db::change_tracking)
//! tombstones.json          지운 기록 [{tbl, key, deletedAt}]
//! settings.json            같이 쓰는 설정 칸만 (service::data::settings_scope)
//! files.json               사용자 데이터 파일 목록 [{path, size, hash, modifiedAt, included}]
//! blobs/<sha256>           파일 내용 (included인 것만, 같은 내용은 한 번)
//! ```
//!
//! - **경로**: 행과 설정의 글자 안에 든 보낸 기기의 앱 데이터 경로를 `$SANDCASTLE_DATA$`로 바꿔 넣는다. 받는 쪽은 자기
//!   경로로 되돌린다. Windows ↔ Android 사이에서도 경로가 맞고, 묶음에 보낸 사람의 사용자 이름이 든 경로가 남지 않는다.
//! - **비밀**: API 키·토큰은 DB·설정에 없으므로 들어가지 않는다(2-B-4 테스트로 고정). 기기 id 파일·휴지통·실행 기록도 넣지 않는다.
//! - **버전**: 받는 앱보다 스키마가 새 묶음은 열지 않고 업데이트를 안내한다. 암호화(ADR-011 ⑤)는 아직 없어 `encryption`이
//!   있는 묶음도 열지 않는다.
#![cfg_attr(not(test), allow(dead_code))] // 2-B-2·2-B-3에서 쓴다

use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use zip::{ZipArchive, ZipWriter};

use super::archive::{self, zip_err};
use super::settings_scope;
use crate::db::{change_tracking, schema::Schema};
use crate::error::{AppError, ErrorCode, Result};

pub const FORMAT: &str = "sandcastle-sync";
pub const FORMAT_VERSION: u32 = 1;
pub const EXTENSION: &str = "scsync";
/// 동기화하는 파일 폴더 (ADR-008의 사용자 데이터 폴더. 휴지통 `trash/`는 기기마다라 뺀다)
pub const SYNC_DIRS: [&str; 5] = ["characters", "games", "themes", "generated", "backgrounds"];
/// 앱 데이터 경로 자리표시
pub const DATA_MARK: &str = "$SANDCASTLE_DATA$";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncManifest {
    pub format: String,
    pub format_version: u32,
    /// 보낸 기기의 DB 마이그레이션 버전 (가장 큰 번호)
    pub schema_version: i64,
    pub app_version: String,
    pub device_id: String,
    pub created_at: i64,
    /// 이 시각 뒤에 바뀐 것만 담았다 (None = 전부)
    pub since: Option<i64>,
    /// 테이블별 행 수
    pub tables: BTreeMap<String, u64>,
    pub tombstones: u64,
    pub files: u64,
    pub blobs: u64,
    /// 암호화 방식 (아직 없음 — 2-C 뒤, ADR-011 ⑤)
    #[serde(default)]
    pub encryption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tombstone {
    pub tbl: String,
    /// 기본 키(없으면 UNIQUE 묶음) 값의 JSON 배열 글자 (예: `["id"]`)
    pub key: String,
    pub deleted_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// 앱 데이터 폴더 기준, `/`로 나눈 경로 (예: `characters/abc/joy.png`)
    pub path: String,
    pub size: u64,
    /// sha256 (소문자 16진수)
    pub hash: String,
    pub modified_at: i64,
    /// 내용이 blobs/에 들어 있나 (아니면 목록에만 — 받는 쪽은 같은 해시가 있으면 그대로 씀)
    pub included: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BundleOptions {
    /// 이 시각(밀리초) 뒤에 바뀐 행·지운 기록·파일만. None = 전부
    pub since: Option<i64>,
    /// 파일 내용을 넣을지 (false면 목록과 해시만)
    pub include_files: bool,
}

// ---------------------------------------------------------------------------
// 경로 바꾸기
// ---------------------------------------------------------------------------

/// 경로가 끝나는 글자 (자리표시 뒤의 구분자를 `/`로 맞출 범위). 빈칸은 폴더 이름에 들어갈 수 있어 넣지 않는다
fn ends_path(c: char) -> bool {
    matches!(c, '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | ',' | '\n' | '\r' | '\t')
}

/// 보낸 기기의 앱 데이터 경로 → 자리표시. 자리표시 뒤의 `\`(JSON 안이면 `\\`)는 `/`로
pub fn to_portable(text: &str, base: &str) -> String {
    if base.is_empty() {
        return text.to_string();
    }
    let mut forms = vec![base.to_string()];
    if base.contains('\\') {
        forms.push(base.replace('\\', "\\\\"));
        forms.push(base.replace('\\', "/"));
    }
    if !forms.iter().any(|f| text.contains(f.as_str())) {
        return text.to_string();
    }
    let mut s = text.to_string();
    // 긴 모양(JSON 안)부터 바꿔야 짧은 모양이 그 일부를 먼저 먹지 않는다
    forms.sort_by_key(|f| std::cmp::Reverse(f.len()));
    for f in &forms {
        s = s.replace(f.as_str(), DATA_MARK);
    }
    // 자리표시 뒤 경로 부분의 구분자를 / 로
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(i) = rest.find(DATA_MARK) {
        out.push_str(&rest[..i + DATA_MARK.len()]);
        rest = &rest[i + DATA_MARK.len()..];
        let mut chars = rest.char_indices().peekable();
        let mut end = rest.len();
        let mut seg = String::new();
        while let Some((j, c)) = chars.next() {
            if c == '\\' {
                match chars.peek() {
                    Some((_, '\\')) => {
                        chars.next();
                        seg.push('/');
                    }
                    Some((_, '"')) | None => {
                        end = j;
                        break;
                    }
                    _ => seg.push('/'),
                }
            } else if ends_path(c) {
                end = j;
                break;
            } else {
                seg.push(c);
            }
        }
        out.push_str(&seg);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// 자리표시 → 이 기기의 앱 데이터 경로 (구분자는 `/` — Windows도 읽는다)
pub fn from_portable(text: &str, base: &str) -> String {
    if !text.contains(DATA_MARK) {
        return text.to_string();
    }
    text.replace(DATA_MARK, base.replace('\\', "/").trim_end_matches('/'))
}

fn map_strings(v: &mut Value, f: &dyn Fn(&str) -> String) {
    match v {
        Value::String(s) => *s = f(s),
        Value::Array(a) => a.iter_mut().for_each(|x| map_strings(x, f)),
        Value::Object(o) => o.values_mut().for_each(|x| map_strings(x, f)),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// 만들기
// ---------------------------------------------------------------------------

pub async fn schema_version(db: &SqlitePool) -> Result<i64> {
    Ok(
        sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM _sqlx_migrations WHERE success = 1")
            .fetch_one(db)
            .await?
            .unwrap_or(0),
    )
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn modified_ms(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 사용자 데이터 파일 목록 (해시 포함)
pub fn list_files(base: &Path) -> Result<Vec<FileEntry>> {
    let mut out = vec![];
    for dir in SYNC_DIRS {
        let root = base.join(dir);
        if !root.exists() {
            continue;
        }
        for f in archive::walk_files(&root)? {
            let Ok(rel) = f.strip_prefix(base) else { continue };
            out.push(FileEntry {
                path: rel.to_string_lossy().replace('\\', "/"),
                size: std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0),
                hash: sha256_file(&f)?,
                modified_at: modified_ms(&f),
                included: false,
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// 묶음 만들기. `settings`는 settings.json 전체({"app-setting": …, …}), `device_id`는 service::device
pub async fn write_bundle(
    db: &SqlitePool,
    base: &Path,
    settings: &Map<String, Value>,
    device_id: &str,
    opts: &BundleOptions,
    out: &Path,
) -> Result<SyncManifest> {
    let base_str = base.to_string_lossy().to_string();
    let portable = |v: &mut Value| map_strings(v, &|s| to_portable(s, &base_str));
    let since = opts.since.unwrap_or(i64::MIN);

    // ---- DB에서 읽기 ----
    let mut conn = db.acquire().await?;
    let schema = Schema::load(&mut conn).await?;
    let mut tables: Vec<(String, Vec<String>)> = vec![];
    for t in change_tracking::synced_tables(&mut *conn).await? {
        let Some(cols) = schema.columns.get(&t) else { continue };
        if !cols.iter().any(|c| c == "updated_at") {
            continue;
        }
        let obj = cols
            .iter()
            .map(|c| format!("'{}', {}", c.replace('\'', "''"), crate::db::schema::quote(c)))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT json_object({obj}) FROM {} WHERE updated_at > ? ORDER BY rowid",
            crate::db::schema::quote(&t)
        );
        let rows: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql)).bind(since).fetch_all(&mut *conn).await?;
        let mut lines = Vec::with_capacity(rows.len());
        for r in rows {
            let mut v: Value = serde_json::from_str(&r).map_err(|e| AppError::internal(format!("{t} 행을 읽지 못함: {e}")))?;
            portable(&mut v);
            lines.push(v.to_string());
        }
        if !lines.is_empty() {
            tables.push((t, lines));
        }
    }
    let tombstones: Vec<Tombstone> =
        sqlx::query_as::<_, (String, String, i64)>("SELECT tbl, key, deleted_at FROM sync_tombstones WHERE deleted_at > ? ORDER BY deleted_at")
            .bind(since)
            .fetch_all(&mut *conn)
            .await?
            .into_iter()
            .map(|(tbl, key, deleted_at)| Tombstone { tbl, key, deleted_at })
            .collect();
    let schema_version = sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM _sqlx_migrations WHERE success = 1")
        .fetch_one(&mut *conn)
        .await?
        .unwrap_or(0);
    drop(conn);

    let mut shared = Value::Object(settings_scope::shared_part(settings));
    portable(&mut shared);

    let manifest = SyncManifest {
        format: FORMAT.into(),
        format_version: FORMAT_VERSION,
        schema_version,
        app_version: env!("CARGO_PKG_VERSION").into(),
        device_id: device_id.into(),
        created_at: chrono::Utc::now().timestamp_millis(),
        since: opts.since,
        tables: tables.iter().map(|(t, l)| (t.clone(), l.len() as u64)).collect(),
        tombstones: tombstones.len() as u64,
        files: 0,
        blobs: 0,
        encryption: None,
    };

    // ---- 파일과 zip (막는 작업이라 따로) ----
    let (base, out, include) = (base.to_path_buf(), out.to_path_buf(), opts.include_files);
    tokio::task::spawn_blocking(move || -> Result<SyncManifest> {
        let mut files = list_files(&base)?;
        for f in &mut files {
            f.included = include && f.modified_at > since;
        }
        let mut manifest = manifest;
        manifest.files = files.len() as u64;

        let tmp = out.with_extension("partial");
        let mut zip = ZipWriter::new(File::create(&tmp)?);
        for (t, lines) in &tables {
            zip.start_file(format!("tables/{t}.jsonl"), archive::options()).map_err(zip_err)?;
            for l in lines {
                zip.write_all(l.as_bytes())?;
                zip.write_all(b"\n")?;
            }
        }
        archive::write_json(&mut zip, "tombstones.json", &tombstones)?;
        archive::write_json(&mut zip, "settings.json", &shared)?;
        let mut written: HashSet<String> = HashSet::new();
        for f in files.iter().filter(|f| f.included) {
            if written.insert(f.hash.clone()) {
                archive::write_file(&mut zip, &format!("blobs/{}", f.hash), &base.join(&f.path))?;
            }
        }
        manifest.blobs = written.len() as u64;
        archive::write_json(&mut zip, "files.json", &files)?;
        archive::write_json(&mut zip, "manifest.json", &manifest)?;
        zip.finish().map_err(zip_err)?;
        std::fs::rename(&tmp, &out)?;
        Ok(manifest)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

// ---------------------------------------------------------------------------
// 읽기
// ---------------------------------------------------------------------------

pub struct Bundle {
    pub manifest: SyncManifest,
    archive: ZipArchive<File>,
    base: String,
}

fn invalid(msg: impl Into<String>) -> AppError {
    AppError::archive(ErrorCode::ArchiveInvalid, msg)
}

/// 묶음 열기. `local_schema`는 이 기기 DB의 스키마 버전, `base`는 이 기기의 앱 데이터 폴더
pub fn open(path: &Path, local_schema: i64, base: &Path) -> Result<Bundle> {
    let mut archive = archive::open_archive(path)?;
    let manifest: SyncManifest = archive::read_json(&mut archive, "manifest.json").map_err(|_| invalid("동기화 묶음이 아님 (manifest.json 없음)"))?;
    if manifest.format != FORMAT {
        return Err(invalid(format!("동기화 묶음이 아님 ({})", manifest.format)));
    }
    if manifest.format_version > FORMAT_VERSION {
        return Err(invalid(format!(
            "더 새 앱에서 만든 묶음 (형식 v{}) — 이 기기의 앱을 업데이트하세요",
            manifest.format_version
        )));
    }
    if manifest.schema_version > local_schema {
        return Err(invalid(format!(
            "더 새 앱에서 만든 묶음 (DB 버전 {} > 이 기기 {}) — 이 기기의 앱({})을 업데이트하세요",
            manifest.schema_version,
            local_schema,
            env!("CARGO_PKG_VERSION")
        )));
    }
    if let Some(e) = &manifest.encryption {
        return Err(invalid(format!("암호화된 묶음({e})은 아직 열 수 없음")));
    }
    Ok(Bundle {
        manifest,
        archive,
        base: base.to_string_lossy().to_string(),
    })
}

impl Bundle {
    fn localize(&self, v: &mut Value) {
        let base = self.base.clone();
        map_strings(v, &move |s| from_portable(s, &base));
    }

    /// 한 테이블의 행 (경로는 이 기기 것으로). 이 기기가 동기화하지 않는 테이블은 읽지 않는다 (호출하는 쪽이 거름)
    pub fn rows(&mut self, table: &str) -> Result<Vec<Map<String, Value>>> {
        if !self.manifest.tables.contains_key(table) {
            return Ok(vec![]);
        }
        let base = self.base.clone();
        let entry = self
            .archive
            .by_name(&format!("tables/{table}.jsonl"))
            .map_err(|_| invalid(format!("{table} 행이 없음")))?;
        let mut out = vec![];
        for line in BufReader::new(entry).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let mut v: Value = serde_json::from_str(&line).map_err(|e| invalid(format!("{table} 행을 읽지 못함: {e}")))?;
            map_strings(&mut v, &|s| from_portable(s, &base));
            match v {
                Value::Object(m) => out.push(m),
                _ => return Err(invalid(format!("{table} 행 모양이 이상함"))),
            }
        }
        Ok(out)
    }

    pub fn tombstones(&mut self) -> Result<Vec<Tombstone>> {
        archive::read_json(&mut self.archive, "tombstones.json")
    }

    /// 같이 쓰는 설정 칸 (경로는 이 기기 것으로)
    pub fn settings(&mut self) -> Result<Map<String, Value>> {
        let mut v: Value = archive::read_json(&mut self.archive, "settings.json")?;
        self.localize(&mut v);
        match v {
            Value::Object(m) => Ok(m),
            _ => Err(invalid("settings.json 모양이 이상함")),
        }
    }

    /// 파일 목록. 경로가 동기화 폴더 밖이거나 이상하면 거부
    pub fn files(&mut self) -> Result<Vec<FileEntry>> {
        let files: Vec<FileEntry> = archive::read_json(&mut self.archive, "files.json")?;
        for f in &files {
            let rel = archive::safe_relative(&f.path)?;
            let top = rel.components().next().map(|c| c.as_os_str().to_string_lossy().to_string()).unwrap_or_default();
            if !SYNC_DIRS.contains(&top.as_str()) {
                return Err(AppError::archive(ErrorCode::ArchiveUnsafePath, format!("동기화 폴더 밖의 파일: {}", f.path)));
            }
            if f.hash.len() != 64 || !f.hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(invalid(format!("파일 해시가 이상함: {}", f.path)));
            }
        }
        Ok(files)
    }

    /// 파일 내용을 꺼낸다. 해시가 맞지 않으면 지우고 거부
    pub fn extract_blob(&mut self, hash: &str, dest: &Path) -> Result<()> {
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("파일 해시가 이상함"));
        }
        archive::extract_file(&mut self.archive, &format!("blobs/{hash}"), dest)?;
        if sha256_file(dest)? != hash {
            let _ = std::fs::remove_file(dest);
            return Err(invalid(format!("파일 내용이 해시와 다름: {hash}")));
        }
        Ok(())
    }
}

/// 묶음 안의 이름 목록 (테스트·점검용)
#[cfg(test)]
fn entry_names<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Vec<String> {
    (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_become_portable_and_come_back() {
        let win = r"C:\Users\kim\AppData\Roaming\com.dilly311.sandcastle";
        // 칸 전체가 경로
        let p = to_portable(&format!(r"{win}\characters\c1\joy.png"), win);
        assert_eq!(p, "$SANDCASTLE_DATA$/characters/c1/joy.png");
        assert!(!p.contains("kim"), "사용자 이름이 남으면 안 됨");
        let android = "/data/user/0/com.dilly311.sandcastle/files";
        assert_eq!(from_portable(&p, android), "/data/user/0/com.dilly311.sandcastle/files/characters/c1/joy.png");
        // JSON 글 안 (역슬래시 두 개)
        let json = serde_json::json!({ "bg": format!(r"{win}\backgrounds\a b.png"), "n": 1 }).to_string();
        let p = to_portable(&json, win);
        assert!(p.contains(r#""$SANDCASTLE_DATA$/backgrounds/a b.png""#), "{p}");
        let back: Value = serde_json::from_str(&from_portable(&p, android)).unwrap();
        assert_eq!(back["bg"], "/data/user/0/com.dilly311.sandcastle/files/backgrounds/a b.png");
        // 글 속 슬래시 모양, 뒤에 다른 글
        let md = format!("그림 ![x]({}/generated/1.png) 끝", win.replace('\\', "/"));
        assert_eq!(to_portable(&md, win), "그림 ![x]($SANDCASTLE_DATA$/generated/1.png) 끝");
        // 경로가 없으면 그대로, 유닉스 경로도
        assert_eq!(to_portable("안녕", win), "안녕");
        assert_eq!(to_portable(&format!("{android}/games/g"), android), "$SANDCASTLE_DATA$/games/g");
        assert_eq!(
            from_portable("$SANDCASTLE_DATA$/games/g", win),
            "C:/Users/kim/AppData/Roaming/com.dilly311.sandcastle/games/g"
        );
    }

    #[tokio::test]
    async fn bundle_round_trip() {
        use crate::models::settings::{AppSetting, PluginSetting, SettingsDomain, ThemeSetting};
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("appdata");
        let db = crate::db::conn::init_pool(base.clone()).await.unwrap();
        let avatar = base.join("characters").join("avatars").join("a.png");
        std::fs::create_dir_all(avatar.parent().unwrap()).unwrap();
        std::fs::write(&avatar, b"png-bytes").unwrap();
        std::fs::create_dir_all(base.join("trash/t1")).unwrap();
        std::fs::write(base.join("trash/t1/x.png"), b"trashed").unwrap();
        sqlx::query("INSERT INTO characters (id, name, avatar, created_at, updated_at) VALUES ('c1', 'Aria', ?, 1, 1000)")
            .bind(avatar.to_string_lossy().to_string())
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c2', 'Old', 1, 1000)")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("DELETE FROM characters WHERE id = 'c2'").execute(&db).await.unwrap();
        // 동기화하지 않는 테이블
        sqlx::query("INSERT INTO translation_cache (source_hash, lang, text, created_at) VALUES ('h', 'ko', '번역', 1)")
            .execute(&db)
            .await
            .ok();

        let mut settings = Map::new();
        let mut app = serde_json::to_value(AppSetting::default()).unwrap();
        app["selectedApiProfile"] = "p-local".into();
        settings.insert(AppSetting::STORE_KEY.into(), app);
        let mut theme = serde_json::to_value(ThemeSetting::default()).unwrap();
        theme["backgroundImage"] = base.join("backgrounds").join("bg.png").to_string_lossy().to_string().into();
        settings.insert(ThemeSetting::STORE_KEY.into(), theme);
        settings.insert(PluginSetting::STORE_KEY.into(), serde_json::to_value(PluginSetting::default()).unwrap());

        let out = dir.path().join("a.scsync");
        let m = write_bundle(
            &db,
            &base,
            &settings,
            "dev-a",
            &BundleOptions {
                since: None,
                include_files: true,
            },
            &out,
        )
        .await
        .unwrap();
        assert_eq!(m.tables.get("characters"), Some(&1));
        assert!(!m.tables.contains_key("translation_cache"));
        assert!(!m.tables.contains_key("sync_tombstones") && !m.tables.contains_key("trash_items"));
        assert_eq!(m.tombstones, 1);
        assert_eq!((m.files, m.blobs), (1, 1), "휴지통 파일은 넣지 않음");
        assert_eq!(m.device_id, "dev-a");
        assert_eq!(m.schema_version, schema_version(&db).await.unwrap());

        // 묶음 안에 보낸 기기의 경로가 남지 않는다
        let mut raw = archive::open_archive(&out).unwrap();
        let base_str = base.to_string_lossy().to_string();
        for name in entry_names(&mut raw) {
            let mut s = String::new();
            if raw.by_name(&name).unwrap().read_to_string(&mut s).is_ok() {
                assert!(
                    !s.contains(&base_str) && !s.contains(&base_str.replace('\\', "/")),
                    "{name}에 앱 데이터 경로가 남음"
                );
            }
        }

        // 다른 기기(경로가 다름)에서 열기
        let other = dir.path().join("other");
        let mut b = open(&out, m.schema_version, &other).unwrap();
        let rows = b.rows("characters").unwrap();
        assert_eq!(rows[0]["id"], "c1");
        assert_eq!(
            rows[0]["avatar"].as_str().unwrap(),
            format!("{}/characters/avatars/a.png", other.to_string_lossy().replace('\\', "/"))
        );
        assert_eq!(b.tombstones().unwrap()[0].key, r#"["c2"]"#);
        let s = b.settings().unwrap();
        assert!(s["app-setting"].get("selectedApiProfile").is_none(), "기기 전용 설정은 빠짐");
        assert!(s["theme-setting"]["backgroundImage"]
            .as_str()
            .unwrap()
            .starts_with(&other.to_string_lossy().replace('\\', "/")));
        let files = b.files().unwrap();
        assert_eq!(files[0].path, "characters/avatars/a.png");
        let dest = other.join("x.png");
        b.extract_blob(&files[0].hash, &dest).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"png-bytes");

        // since: 그 뒤에 바뀐 것만
        let out2 = dir.path().join("b.scsync");
        let m2 = write_bundle(
            &db,
            &base,
            &settings,
            "dev-a",
            &BundleOptions {
                since: Some(chrono::Utc::now().timestamp_millis() + 60_000),
                include_files: false,
            },
            &out2,
        )
        .await
        .unwrap();
        assert!(m2.tables.is_empty() && m2.tombstones == 0 && m2.blobs == 0);
        assert_eq!(m2.files, 1, "목록은 늘 전부 (받는 쪽이 빠진 파일을 알 수 있게)");

        // 더 새 스키마는 거부
        let err = open(&out, m.schema_version - 1, &other).err().unwrap();
        assert!(err.to_string().contains("업데이트"), "{err}");
    }

    #[test]
    fn rejects_unsafe_or_foreign_bundles() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, manifest: &Value, files: &Value| {
            let p = dir.path().join(name);
            let mut zip = ZipWriter::new(File::create(&p).unwrap());
            archive::write_json(&mut zip, "manifest.json", manifest).unwrap();
            archive::write_json(&mut zip, "files.json", files).unwrap();
            zip.finish().unwrap();
            p
        };
        let good = serde_json::json!({
            "format": FORMAT, "formatVersion": 1, "schemaVersion": 1, "appVersion": "0.1.0", "deviceId": "d",
            "createdAt": 0, "since": null, "tables": {}, "tombstones": 0, "files": 1, "blobs": 0
        });
        let hash = "a".repeat(64);
        let f = |path: &str| serde_json::json!([{ "path": path, "size": 1, "hash": hash, "modifiedAt": 0, "included": false }]);
        let base = dir.path();
        for bad in ["../x.png", "trash/t/x.png", "C:/x.png", "settings.json"] {
            let p = write("bad.scsync", &good, &f(bad));
            assert!(open(&p, 999, base).unwrap().files().is_err(), "{bad}");
        }
        let p = write("ok.scsync", &good, &f("generated/1.png"));
        assert_eq!(open(&p, 999, base).unwrap().files().unwrap().len(), 1);

        let mut other = good.clone();
        other["format"] = "sandcastle-backup".into();
        assert!(open(&write("o.scsync", &other, &f("generated/1.png")), 999, base).is_err());
        let mut enc = good.clone();
        enc["encryption"] = "aes".into();
        assert!(open(&write("e.scsync", &enc, &f("generated/1.png")), 999, base).is_err());
    }
}
