// SPDX-License-Identifier: Apache-2.0

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process,
    time::{Duration, SystemTime},
};

use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

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
    Ok(get_entry(cache_key, ttl, refresh, offline)?.map(|entry| entry.value))
}

#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub fetched_at: Option<String>,
    pub age: Duration,
    pub stale: bool,
}

pub fn get_entry<T: DeserializeOwned>(
    cache_key: &str,
    ttl: Duration,
    refresh: bool,
    offline: bool,
) -> crate::error::Result<Option<CacheEntry<T>>> {
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
    let fetched_at = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|duration| OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64).ok())
        .and_then(|dt| dt.format(&Rfc3339).ok());
    Ok(Some(CacheEntry {
        value,
        fetched_at,
        age,
        stale: age > ttl,
    }))
}

pub fn set<T: Serialize>(cache_key: &str, value: &T) -> crate::error::Result<()> {
    let dir = cache_dir()?;
    fs::create_dir_all(&dir)
        .map_err(|err| CampusError::cache(format!("failed to create {}: {err}", dir.display())))?;
    let path = dir.join(cache_key);
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| CampusError::cache(format!("failed to serialize cache: {err}")))?;
    write_private(&path, &text)
        .map_err(|err| CampusError::cache(format!("failed to write {}: {err}", path.display())))
}

pub fn namespace(parts: &[&str]) -> String {
    parts.join("|")
}

pub fn profile_namespace(
    profile_name: &str,
    profile: &crate::config::Profile,
    user_id: Option<i64>,
) -> String {
    let user_id = user_id.map(|id| id.to_string()).unwrap_or_default();
    namespace(&[
        "v1",
        profile_name,
        profile.base_url.as_str(),
        &profile.username,
        &profile.service,
        &user_id,
    ])
}

fn write_private(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let unique = format!(
        "tmp-{}-{}",
        process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let tmp = path.with_extension(unique);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}
