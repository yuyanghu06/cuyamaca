use keyring::Entry;

const SERVICE_PREFIX: &str = "cuyamaca";

pub fn store_api_key(provider: &str, key: &str) -> Result<(), String> {
    let entry = Entry::new(&format!("{}-{}", SERVICE_PREFIX, provider), "api_key")
        .map_err(|e| format!("Keyring entry error: {}", e))?;
    entry
        .set_password(key)
        .map_err(|e| format!("Failed to store key: {}", e))?;
    Ok(())
}

pub fn get_api_key(provider: &str) -> Result<Option<String>, String> {
    let entry = Entry::new(&format!("{}-{}", SERVICE_PREFIX, provider), "api_key")
        .map_err(|e| format!("Keyring entry error: {}", e))?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to read key: {}", e)),
    }
}

#[allow(dead_code)]
pub fn delete_api_key(provider: &str) -> Result<(), String> {
    let entry = Entry::new(&format!("{}-{}", SERVICE_PREFIX, provider), "api_key")
        .map_err(|e| format!("Keyring entry error: {}", e))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete key: {}", e)),
    }
}
