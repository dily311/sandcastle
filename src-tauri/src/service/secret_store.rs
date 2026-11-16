use anyhow::Result;
#[cfg(not(test))]
use keyring::Entry;

#[cfg_attr(test, allow(dead_code))]
const SERVICE_NAME: &str = "sandcastle-secret";

/// Windows 자격 증명 관리자는 비밀 하나를 UTF-16 2560자까지만 받는다 (Vertex 서비스 계정 JSON은 이보다 길다).
/// 긴 값은 여러 조각으로 나눠 `키#0`, `키#1`… 에 두고, 원래 키에는 조각 수만 적는다.
const CHUNK_CHARS: usize = 1000;
const CHUNK_MARK: &str = "\u{1}sandcastle-chunks:";

/**
 * api key를 저장할때는 이 함수를 사용해서 문자열 형식 지정함.
 * 다른 비밀 키를 저장할 수도 있어, 구분을 위해 사용.
 */
pub fn api_key_name(provider_id: &str) -> String {
    format!("provider:{provider_id}:api_key")
}

fn chunk_key(key: &str, i: usize) -> String {
    format!("{key}#{i}")
}

/// 값을 조각으로 (글자 경계에서 자름)
fn split_chunks(value: &str) -> Vec<String> {
    let chars: Vec<char> = value.chars().collect();
    chars.chunks(CHUNK_CHARS).map(|c| c.iter().collect()).collect()
}

fn chunk_count(stored: &str) -> Option<usize> {
    stored.strip_prefix(CHUNK_MARK).and_then(|n| n.parse().ok())
}

/// 실제 저장소: 앱은 OS keyring
#[cfg(not(test))]
mod backend {
    use super::*;

    fn entry(key: &str) -> Result<Entry> {
        Ok(Entry::new(SERVICE_NAME, key)?)
    }
    pub fn get(key: &str) -> Result<String> {
        Ok(entry(key)?.get_password()?)
    }
    pub fn set(key: &str, value: &str) -> Result<()> {
        Ok(entry(key)?.set_password(value)?)
    }
    pub fn delete(key: &str) -> Result<()> {
        Ok(entry(key)?.delete_credential()?)
    }
}

/// 테스트: 메모리 (OS 자격 증명 관리자를 건드리지 않게, CI에서도 돌게)
#[cfg(test)]
mod backend {
    use std::{collections::HashMap, sync::Mutex};

    use super::Result;

    static MAP: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
    fn with<T>(f: impl FnOnce(&mut HashMap<String, String>) -> T) -> T {
        let mut g = MAP.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        f(g.get_or_insert_with(HashMap::new))
    }
    pub fn get(key: &str) -> Result<String> {
        with(|m| m.get(key).cloned()).ok_or_else(|| anyhow::anyhow!("no entry"))
    }
    pub fn set(key: &str, value: &str) -> Result<()> {
        with(|m| m.insert(key.to_string(), value.to_string()));
        Ok(())
    }
    pub fn delete(key: &str) -> Result<()> {
        with(|m| m.remove(key)).map(|_| ()).ok_or_else(|| anyhow::anyhow!("no entry"))
    }
}

pub struct SecretStore;
impl SecretStore {
    pub fn get(key: &str) -> Result<String> {
        let stored = backend::get(key)?;
        let Some(n) = chunk_count(&stored) else { return Ok(stored) };
        let mut out = String::new();
        for i in 0..n {
            out.push_str(&backend::get(&chunk_key(key, i))?);
        }
        Ok(out)
    }

    pub fn set(key: &str, value: &str) -> Result<()> {
        // 예전 조각이 남지 않게 먼저 지운다
        Self::delete_chunks(key);
        if value.chars().count() <= CHUNK_CHARS {
            backend::set(key, value)?;
            return Ok(());
        }
        let parts = split_chunks(value);
        for (i, p) in parts.iter().enumerate() {
            backend::set(&chunk_key(key, i), p)?;
        }
        backend::set(key, &format!("{CHUNK_MARK}{}", parts.len()))?;
        Ok(())
    }

    pub fn delete(key: &str) -> Result<()> {
        Self::delete_chunks(key);
        backend::delete(key)?;
        Ok(())
    }

    /// 조각으로 나눠 저장했던 것만 지운다 (원래 키는 그대로)
    fn delete_chunks(key: &str) {
        let Ok(stored) = backend::get(key) else { return };
        if let Some(n) = chunk_count(&stored) {
            for i in 0..n {
                let _ = backend::delete(&chunk_key(key, i));
            }
        }
    }

    pub fn exists(key: &str) -> bool {
        Self::get(key).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_values_split_on_char_boundaries_and_rejoin() {
        let value: String = "가나다{\"k\":1}".repeat(400);
        let parts = split_chunks(&value);
        assert!(parts.len() > 1);
        assert!(parts.iter().all(|p| p.chars().count() <= CHUNK_CHARS));
        assert_eq!(parts.concat(), value);
        assert_eq!(chunk_count(&format!("{CHUNK_MARK}3")), Some(3));
        assert_eq!(chunk_count("sk-plain-key"), None);
    }

    #[test]
    fn set_get_delete_round_trip_with_chunks() {
        let long: String = "k".repeat(2500);
        SecretStore::set("t:long", &long).unwrap();
        assert_eq!(SecretStore::get("t:long").unwrap(), long);
        SecretStore::set("t:long", "short").unwrap(); // 예전 조각이 지워짐
        assert_eq!(SecretStore::get("t:long").unwrap(), "short");
        assert!(SecretStore::get(&chunk_key("t:long", 0)).is_err());
        SecretStore::delete("t:long").unwrap();
        assert!(!SecretStore::exists("t:long"));
    }
}
