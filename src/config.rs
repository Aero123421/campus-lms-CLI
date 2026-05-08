// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, fs, path::PathBuf};

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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Privacy {
    #[serde(default)]
    pub include_grades_in_ai_snapshot: bool,
    #[serde(default)]
    pub include_feedback_in_ai_snapshot: bool,
    #[serde(default)]
    pub include_user_email: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    #[serde(default = "default_output_format")]
    pub default_format: String,
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
            default_format: default_output_format(),
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

fn default_output_format() -> String {
    "text".to_string()
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
    toml::from_str(&text)
        .map_err(|err| CampusError::config(format!("failed to parse {}: {err}", path.display())))
}

pub fn save(cli: &Cli, config: &Config) -> crate::error::Result<()> {
    let path = config_path(cli)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CampusError::config(format!("failed to create {}: {err}", parent.display()))
        })?;
    }
    let text = toml::to_string_pretty(config)
        .map_err(|err| CampusError::config(format!("failed to serialize config: {err}")))?;
    fs::write(&path, text)
        .map_err(|err| CampusError::config(format!("failed to write {}: {err}", path.display())))
}

pub fn active_profile<'a>(cli: &Cli, config: &'a Config) -> crate::error::Result<&'a Profile> {
    config
        .profile
        .get(&cli.profile)
        .ok_or(CampusError::AuthRequired { json: cli.json })
}

pub fn remove_active_profile(cli: &Cli, config: &mut Config) {
    config.profile.remove(&cli.profile);
    if config.active_profile == cli.profile {
        config.active_profile = default_profile();
    }
}
