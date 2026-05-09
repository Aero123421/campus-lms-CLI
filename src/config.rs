// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{cli::Cli, error::CampusError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_profile")]
    pub active_profile: String,
    #[serde(default)]
    pub profile: BTreeMap<String, Profile>,
    #[serde(default)]
    pub privacy: Privacy,
    #[serde(default)]
    pub output: Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub base_url: Url,
    pub username: String,
    #[serde(default = "default_service")]
    pub service: String,
    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: u64,
    #[serde(default = "default_cache_retention_seconds")]
    pub cache_retention_seconds: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Privacy {
    #[serde(default)]
    pub include_grades_in_ai_snapshot: bool,
    #[serde(default)]
    pub include_feedback_in_ai_snapshot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_profile: default_profile(),
            profile: BTreeMap::new(),
            privacy: Privacy::default(),
            output: Output::default(),
        }
    }
}

impl Default for Output {
    fn default() -> Self {
        Self {
            timezone: default_timezone(),
        }
    }
}

pub fn default_profile() -> String {
    "default".to_string()
}

pub fn default_service() -> String {
    "moodle_mobile_app".to_string()
}

pub fn default_cache_ttl_seconds() -> u64 {
    300
}

pub fn default_cache_retention_seconds() -> u64 {
    30 * 24 * 60 * 60
}

fn default_timezone() -> String {
    "Asia/Tokyo".to_string()
}

pub fn config_path(cli: &Cli) -> crate::error::Result<PathBuf> {
    if let Some(path) = &cli.config {
        return Ok(path.clone());
    }
    let dirs = ProjectDirs::from("", "", "campus-lms")
        .ok_or_else(|| CampusError::config("could not determine config directory"))?;
    Ok(dirs.config_dir().join("config.toml"))
}

pub fn config_dir(cli: &Cli) -> crate::error::Result<PathBuf> {
    if let Some(path) = &cli.config {
        return path
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| CampusError::config("custom --config path has no parent directory"));
    }
    let dirs = ProjectDirs::from("", "", "campus-lms")
        .ok_or_else(|| CampusError::config("could not determine config directory"))?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn load(cli: &Cli) -> crate::error::Result<Config> {
    let path = config_path(cli)?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| CampusError::config(format!("failed to read {}: {err}", path.display())))?;
    let config: Config = toml::from_str(&text)
        .map_err(|err| CampusError::config(format!("failed to parse {}: {err}", path.display())))?;
    validate(&config)?;
    Ok(config)
}

pub fn save(cli: &Cli, config: &Config) -> crate::error::Result<()> {
    let path = config_path(cli)?;
    if let Some(parent) = path.parent() {
        create_private_dir(parent).map_err(|err| {
            CampusError::config(format!("failed to create {}: {err}", parent.display()))
        })?;
    }
    let text = toml::to_string_pretty(config)
        .map_err(|err| CampusError::config(format!("failed to serialize config: {err}")))?;
    write_private(&path, &text)
        .map_err(|err| CampusError::config(format!("failed to write {}: {err}", path.display())))
}

pub fn active_profile<'a>(cli: &Cli, config: &'a Config) -> crate::error::Result<&'a Profile> {
    let name = selected_profile_name(cli, config);
    config
        .profile
        .get(&name)
        .ok_or(CampusError::AuthRequired { json: cli.json })
}

pub fn remove_active_profile(cli: &Cli, config: &mut Config) {
    let name = selected_profile_name(cli, config);
    config.profile.remove(&name);
    if config.active_profile == name {
        config.active_profile = default_profile();
    }
}

pub fn selected_profile_name(cli: &Cli, config: &Config) -> String {
    cli.profile
        .clone()
        .unwrap_or_else(|| config.active_profile.clone())
}

pub fn validate(config: &Config) -> crate::error::Result<()> {
    validate_timezone(&config.output.timezone)?;
    Ok(())
}

pub fn validate_timezone(timezone: &str) -> crate::error::Result<()> {
    match timezone {
        "UTC" | "Asia/Tokyo" => Ok(()),
        other => Err(CampusError::config(format!(
            "unsupported output.timezone '{other}'; supported values are UTC and Asia/Tokyo"
        ))),
    }
}

fn create_private_dir(path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
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
