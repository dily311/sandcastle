use anyhow::Result;
use keyring::Entry;

const SERVICE_NAME: &str = "sandcastle-secret";

/** 
 * api key를 저장할때는 이 함수를 사용해서 문자열 형식 지정함.
 * 다른 비밀 키를 저장할 수도 있어, 구분을 위해 사용.
 */
pub fn api_key_name(provider_id: &str) -> String {
    format!("provider:{provider_id}:api_key")
}

pub struct SecretStore;
impl SecretStore {
    fn entry(key: &str) -> Result<Entry> {
        let entry = Entry::new(SERVICE_NAME, key)?;
        Ok(entry)
    }
    pub fn get(key: &str) -> Result<String> {
        let entry = Self::entry(key)?;
        let result = entry.get_password()?;
        Ok(result)
    }   
    
    pub fn set(key: &str, value: &str) -> Result<()> {
        let entry = Self::entry(key)?;
        entry.set_password(value)?;
        Ok(())
    } 
    
    pub fn delete(key: &str) -> Result<()> {
        let entry = Self::entry(key)?;
        entry.delete_credential()?;
        Ok(())
    }

    pub fn exists(key: &str) -> bool {
        Self::get(key).is_ok()
    }
}
