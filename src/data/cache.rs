use anyhow::Result;
use std::path::PathBuf;

/// Get the cache directory path, creating it if needed
pub fn cache_dir() -> Result<PathBuf> {
    let dir = std::env::current_dir()?.join("cache");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Save data to a JSON cache file
pub fn save_json<T: serde::Serialize>(filename: &str, data: &T) -> Result<()> {
    let path = cache_dir()?.join(filename);
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load data from a JSON cache file
pub fn load_json<T: serde::de::DeserializeOwned>(filename: &str) -> Result<T> {
    let path = cache_dir()?.join(filename);
    let json = std::fs::read_to_string(path)?;
    let data = serde_json::from_str(&json)?;
    Ok(data)
}

/// Check if a cache file exists and is recent (within max_age_hours)
pub fn is_cache_fresh(filename: &str, max_age_hours: u64) -> bool {
    let path = match cache_dir() {
        Ok(dir) => dir.join(filename),
        Err(_) => return false,
    };
    if !path.exists() {
        return false;
    }
    match std::fs::metadata(&path) {
        Ok(meta) => match meta.modified() {
            Ok(modified) => {
                let age = std::time::SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default();
                age.as_secs() < max_age_hours * 3600
            }
            Err(_) => false,
        },
        Err(_) => false,
    }
}
