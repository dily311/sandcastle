//! Rust panic 기록 (CLAUDE.md 1-D-6).
//!
//! panic이 나면 앱 데이터 폴더의 `logs/panic.log`에 한 줄(JSON)을 남기고, 화면이 떠 있으면 `app-panic` 이벤트로 알린다.
//! 설정 > 관리 > 로그가 지난 실행의 것까지 보여준다. 밖으로 보내지 않는다. 파일이 커지면 뒤쪽 절반만 남긴다.
//! 원래의 panic 처리(표준 오류 출력 등)는 그대로 이어서 부른다.
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// 이 크기를 넘으면 뒤쪽 절반만 남긴다
const MAX_BYTES: u64 = 256 * 1024;
pub const EVENT: &str = "app-panic";

static FILE: OnceLock<PathBuf> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PanicEntry {
    /// ms
    pub time: i64,
    pub thread: String,
    pub message: String,
    /// 파일:줄
    pub location: Option<String>,
}

pub fn log_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("logs").join("panic.log")
}

/// 앱 시작 때 한 번 (DB를 열기 전, 가능한 한 일찍)
pub fn install(app_data_dir: &Path) {
    if FILE.set(log_path(app_data_dir)).is_err() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "알 수 없는 panic".into());
        let entry = PanicEntry {
            time: chrono::Utc::now().timestamp_millis(),
            thread: std::thread::current().name().unwrap_or("이름 없음").to_string(),
            message,
            location: info.location().map(|l| format!("{}:{}", l.file(), l.line())),
        };
        if let Some(path) = FILE.get() {
            let _ = append(path, &entry);
        }
        if let Some(app) = APP.get() {
            let _ = app.emit(EVENT, &entry);
        }
        previous(info);
    }));
}

/// 화면이 생긴 뒤: 실시간 알림을 보낼 수 있게
pub fn attach(app: AppHandle) {
    let _ = APP.set(app);
}

pub fn append(path: &Path, entry: &PanicEntry) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let half = text.len() / 2;
        // 줄 경계에서 자른다
        let cut = text[half..].find('\n').map_or(text.len(), |i| half + i + 1);
        std::fs::write(path, &text[cut..])?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{}", serde_json::to_string(entry).unwrap_or_default())
}

/// 최근 것부터 limit개 (깨진 줄은 건너뜀)
pub fn read(path: &Path, limit: usize) -> Vec<PanicEntry> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines().rev().filter_map(|l| serde_json::from_str(l).ok()).take(limit).collect()
}

pub fn clear(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(i: i64) -> PanicEntry {
        PanicEntry {
            time: i,
            thread: "main".into(),
            message: format!("boom {i}"),
            location: Some("src/x.rs:1".into()),
        }
    }

    #[test]
    fn appends_reads_newest_first_and_clears() {
        let dir = tempfile::tempdir().unwrap();
        let p = log_path(dir.path());
        assert!(read(&p, 10).is_empty());
        for i in 0..3 {
            append(&p, &entry(i)).unwrap();
        }
        std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"not json\n").unwrap();
        let got = read(&p, 2);
        assert_eq!(got, vec![entry(2), entry(1)]);
        clear(&p).unwrap();
        clear(&p).unwrap(); // 없어도 괜찮음
        assert!(read(&p, 10).is_empty());
    }

    #[test]
    fn keeps_file_small() {
        let dir = tempfile::tempdir().unwrap();
        let p = log_path(dir.path());
        let mut big = entry(0);
        big.message = "x".repeat(10_000);
        for _ in 0..60 {
            append(&p, &big).unwrap();
        }
        assert!(std::fs::metadata(&p).unwrap().len() <= MAX_BYTES + 20_000);
        assert!(!read(&p, 100).is_empty(), "잘라도 온전한 줄이 남음");
    }
}
