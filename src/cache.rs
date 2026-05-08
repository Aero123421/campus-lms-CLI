// SPDX-License-Identifier: Apache-2.0

use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

use crate::error::CampusError;

pub fn cache_dir() -> crate::error::Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "campus-lms")
        .ok_or_else(|| CampusError::cache("could not determine cache directory"))?;
    Ok(dirs.cache_dir().to_path_buf())
}

pub fn cache_root_dir() -> crate::error::Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "campus-lms")
        .ok_or_else(|| CampusError::cache("could not determine cache directory"))?;
    Ok(dirs.cache_dir().to_path_buf())
}

pub fn key(name: &str, input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{name}-{:x}.json", hasher.finalize())
}

pub fn get<T: DeserializeOwned>(
    cache_key: &str,
    ttl: Duration,
    refresh: bool,
    offline: bool,
) -> crate::error::Result<Option<T>> {
    if refresh {
        return Ok(None);
    }
    let path = cache_dir()?.join(cache_key);
    if !path.exists() {
        if offline {
            return Err(CampusError::cache(format!(
                "offline cache entry not found: {}",
                path.display()
            )));
        }
        return Ok(None);
    }
    let metadata = fs::metadata(&path)
        .map_err(|err| CampusError::cache(format!("failed to stat {}: {err}", path.display())))?;
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::from_secs(u64::MAX));
    if age > ttl && !offline {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| CampusError::cache(format!("failed to read {}: {err}", path.display())))?;
    let value = serde_json::from_str(&text)
        .map_err(|err| CampusError::cache(format!("failed to parse {}: {err}", path.display())))?;
    Ok(Some(value))
}

pub fn set<T: Serialize>(cache_key: &str, value: &T) -> crate::error::Result<()> {
    let dir = cache_dir()?;
    fs::create_dir_all(&dir)
        .map_err(|err| CampusError::cache(format!("failed to create {}: {err}", dir.display())))?;
    let path = dir.join(cache_key);
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| CampusError::cache(format!("failed to serialize cache: {err}")))?;
    fs::write(&path, text)
        .map_err(|err| CampusError::cache(format!("failed to write {}: {err}", path.display())))
}
