use anyhow::Result;
use keyring::Entry;

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

pub struct SecretStore;
impl SecretStore {
    fn entry(key: &str) -> Result<Entry> {
        let entry = Entry::new(SERVICE_NAME, key)?;
        Ok(entry)
    }

    pub fn get(key: &str) -> Result<String> {
        let stored = Self::entry(key)?.get_password()?;
        let Some(n) = chunk_count(&stored) else { return Ok(stored) };
        let mut out = String::new();
        for i in 0..n {
            out.push_str(&Self::entry(&chunk_key(key, i))?.get_password()?);
        }
        Ok(out)
    }

    pub fn set(key: &str, value: &str) -> Result<()> {
        // 예전 조각이 남지 않게 먼저 지운다
        Self::delete_chunks(key);
        if value.chars().count() <= CHUNK_CHARS {
            Self::entry(key)?.set_password(value)?;
            return Ok(());
        }
        let parts = split_chunks(value);
        for (i, p) in parts.iter().enumerate() {
            Self::entry(&chunk_key(key, i))?.set_password(p)?;
        }
        Self::entry(key)?.set_password(&format!("{CHUNK_MARK}{}", parts.len()))?;
        Ok(())
    }

    pub fn delete(key: &str) -> Result<()> {
        Self::delete_chunks(key);
        Self::entry(key)?.delete_credential()?;
        Ok(())
    }

    /// 조각으로 나눠 저장했던 것만 지운다 (원래 키는 그대로)
    fn delete_chunks(key: &str) {
        let Ok(stored) = Self::entry(key).and_then(|e| Ok(e.get_password()?)) else { return };
        if let Some(n) = chunk_count(&stored) {
            for i in 0..n {
                if let Ok(e) = Self::entry(&chunk_key(key, i)) {
                    let _ = e.delete_credential();
                }
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
}
