//! 이 기기(설치)의 id (CLAUDE.md 2-A-4, ADR-011).
//!
//! 동기화가 "어느 기기에서 바꿨나"를 적을 때 쓴다. 설치마다 하나이고 **동기화·백업에 넣지 않는다** — 그래서 DB나
//! settings.json이 아니라 앱 데이터 폴더의 `device.json`에 둔다(백업은 app.db·settings.json·정해진 폴더만 담음).
//! 백업을 다른 기기에 복원해도 그 기기의 id는 그대로다.
//!
//! 처음 필요할 때 만든다(동기화를 쓰지 않으면 파일도 생기지 않는다 — Lazy Initialization).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

pub const DEVICE_FILE: &str = "device.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub created_at: i64,
}

/// 있으면 읽고, 없거나 깨졌으면 새로 만든다
pub fn get_or_create(app_data_dir: &Path) -> Result<DeviceInfo> {
    let path = app_data_dir.join(DEVICE_FILE);
    if let Ok(text) = std::fs::read_to_string(&path) {
        match serde_json::from_str::<DeviceInfo>(&text) {
            Ok(d) if uuid::Uuid::parse_str(&d.id).is_ok() => return Ok(d),
            _ => eprintln!("[device] {DEVICE_FILE}가 깨져 새 id를 만듦"),
        }
    }
    let info = DeviceInfo {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    std::fs::create_dir_all(app_data_dir)?;
    // 다른 이름으로 쓰고 바꿔 넣어, 쓰다 꺼져도 반쪽짜리 파일이 남지 않게
    let tmp = app_data_dir.join(format!("{DEVICE_FILE}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec_pretty(&info).map_err(|e| AppError::internal(e.to_string()))?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_once_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!dir.path().join(DEVICE_FILE).exists(), "부르기 전에는 파일이 없다");
        let a = get_or_create(dir.path()).unwrap();
        let b = get_or_create(dir.path()).unwrap();
        assert_eq!(a, b);
        assert!(uuid::Uuid::parse_str(&a.id).is_ok());
        // 다른 설치는 다른 id
        let other = tempfile::tempdir().unwrap();
        assert_ne!(get_or_create(other.path()).unwrap().id, a.id);
        // 깨진 파일은 새로 만든다
        std::fs::write(dir.path().join(DEVICE_FILE), "{oops").unwrap();
        assert_ne!(get_or_create(dir.path()).unwrap().id, a.id);
    }
}
