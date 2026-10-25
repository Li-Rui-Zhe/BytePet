//! SQLite stores opaque references, never newly entered API keys.
const PREFIX: &str = "@keyring:v1:";
#[cfg(not(test))]
const SERVICE: &str = "com.bytepet.app.ai";

#[cfg(not(test))]
fn entry(id: &str) -> Result<keyring::Entry, String> {
    #[cfg(all(not(test), not(any(target_os = "windows", target_os = "macos"))))]
    return Err("此平台尚未实现安全凭据存储，不能保存 API Key。".into());
    #[allow(unreachable_code)]
    keyring::Entry::new(SERVICE, id).map_err(|_| "无法访问系统凭据库。".into())
}

pub fn is_reference(value: &str) -> bool {
    value.starts_with(PREFIX)
}

pub fn store(value: &str) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    put(&id, value)?;
    match get(&id) {
        Ok(saved) if saved == value => Ok(format!("{PREFIX}{id}")),
        _ => {
            let _ = remove(&id);
            Err("系统凭据保存校验失败，原配置未更改。".into())
        }
    }
}

pub fn load(value: &str) -> Result<String, String> {
    match value.strip_prefix(PREFIX) {
        Some(id) => get(id),
        // Compatibility only: bootstrap migrates existing plaintext before use.
        None => Ok(value.to_string()),
    }
}

pub fn delete(value: &str) -> Result<(), String> {
    if let Some(id) = value.strip_prefix(PREFIX) {
        remove(id)?;
    }
    Ok(())
}

#[cfg(not(test))]
fn put(id: &str, value: &str) -> Result<(), String> {
    entry(id)?
        .set_password(value)
        .map_err(|_| "保存 API Key 到系统凭据库失败，原配置未更改。".into())
}
#[cfg(not(test))]
fn get(id: &str) -> Result<String, String> {
    entry(id)?
        .get_password()
        .map_err(|_| "无法读取已保存的 API Key，请在设置中重新填写。".into())
}
#[cfg(not(test))]
fn remove(id: &str) -> Result<(), String> {
    match entry(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("清理旧系统凭据失败。".into()),
    }
}

#[cfg(test)]
static MOCK: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashMap<String, String>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
#[cfg(test)]
fn put(id: &str, value: &str) -> Result<(), String> {
    MOCK.lock().unwrap().insert(id.into(), value.into());
    Ok(())
}
#[cfg(test)]
fn get(id: &str) -> Result<String, String> {
    MOCK.lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or("test credential missing".into())
}
#[cfg(test)]
fn remove(id: &str) -> Result<(), String> {
    MOCK.lock().unwrap().remove(id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_is_opaque_recoverable_and_deletable() {
        let secret = "test-only-never-a-real-key";
        let reference = store(secret).unwrap();
        assert!(is_reference(&reference));
        assert!(!reference.contains(secret));
        assert_eq!(load(&reference).unwrap(), secret);
        delete(&reference).unwrap();
        assert!(load(&reference).is_err());
    }

    #[test]
    #[ignore = "Explicit OS credential round-trip; creates and deletes only a random test credential"]
    fn native_credential_round_trip() {
        let id = uuid::Uuid::new_v4().to_string();
        let credential = keyring::Entry::new("com.bytepet.app.test", &id).unwrap();
        credential
            .set_password("test-only-not-a-real-api-key")
            .unwrap();
        let recovered = keyring::Entry::new("com.bytepet.app.test", &id)
            .unwrap()
            .get_password();
        credential.delete_credential().unwrap();
        assert_eq!(recovered.unwrap(), "test-only-not-a-real-api-key");
    }
}
