// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CampusError>;

#[derive(Debug, Error)]
pub enum CampusError {
    #[error("authentication is required")]
    AuthRequired { json: bool },
    #[error("authentication expired or was rejected")]
    AuthExpired { json: bool },
    #[error("permission denied")]
    PermissionDenied { json: bool },
    #[error("network error: {message}")]
    Network { message: String, json: bool },
    #[error("rate limited")]
    RateLimited { json: bool },
    #[error("Moodle API error: {message}")]
    MoodleApi { message: String, json: bool },
    #[error("unsupported Moodle feature: {message}")]
    UnsupportedMoodleFeature { message: String, json: bool },
    #[error("invalid argument: {message}")]
    InvalidArgument {
        message: String,
        hint: Option<String>,
        json: bool,
    },
    #[error("not found: {message}")]
    NotFound { message: String, json: bool },
    #[error("config error: {message}")]
    Config { message: String, json: bool },
    #[error("keychain unavailable: {message}")]
    Keychain { message: String, json: bool },
    #[error("cache error: {message}")]
    Cache { message: String, json: bool },
    #[error("parse error: {message}")]
    Parse { message: String, json: bool },
    #[error("unknown error: {message}")]
    Unknown { message: String, json: bool },
}

impl CampusError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AuthRequired { .. } => "AUTH_REQUIRED",
            Self::AuthExpired { .. } => "AUTH_EXPIRED",
            Self::PermissionDenied { .. } => "PERMISSION_DENIED",
            Self::Network { .. } => "NETWORK_ERROR",
            Self::RateLimited { .. } => "RATE_LIMITED",
            Self::MoodleApi { .. } => "MOODLE_API_ERROR",
            Self::UnsupportedMoodleFeature { .. } => "UNSUPPORTED_MOODLE_FEATURE",
            Self::InvalidArgument { .. } => "INVALID_ARGUMENT",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Config { .. } => "CONFIG_ERROR",
            Self::Keychain { .. } => "KEYCHAIN_UNAVAILABLE",
            Self::Cache { .. } => "CACHE_ERROR",
            Self::Parse { .. } => "PARSE_ERROR",
            Self::Unknown { .. } => "UNKNOWN_ERROR",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Unknown { .. } => 1,
            Self::InvalidArgument { .. } => 2,
            Self::AuthRequired { .. } => 10,
            Self::PermissionDenied { .. } => 11,
            Self::Network { .. } => 12,
            Self::RateLimited { .. } => 13,
            Self::AuthExpired { .. } => 14,
            Self::MoodleApi { .. } => 20,
            Self::UnsupportedMoodleFeature { .. } => 21,
            Self::Config { .. } => 30,
            Self::Keychain { .. } => 31,
            Self::Cache { .. } => 32,
            Self::Parse { .. } => 40,
            Self::NotFound { .. } => 40,
        }
    }

    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Network { .. } | Self::RateLimited { .. } | Self::Cache { .. }
        )
    }

    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::AuthRequired { .. } => {
                Some("Run: campus-lms auth verify --json, then campus-lms auth login or auth import-token.")
            }
            Self::AuthExpired { .. } => Some("Run: campus-lms auth login or auth import-token."),
            Self::Network { .. } => Some("Check your network connection or Moodle base URL."),
            Self::UnsupportedMoodleFeature { .. } => {
                Some("Ask your university LMS administrator whether Moodle Web Services or Mobile Web Services are enabled.")
            }
            Self::Keychain { .. } => Some("Check whether the OS credential store is available."),
            Self::Cache { .. } => {
                Some("Retry the command. If it keeps failing, clear the cache with cleanup --cache.")
            }
            Self::Config { .. } => Some("Check campus-lms config.toml or run auth login again."),
            Self::InvalidArgument { hint, .. } => hint.as_deref(),
            _ => None,
        }
    }

    pub fn next_steps(&self) -> Vec<&'static str> {
        match self {
            Self::AuthRequired { .. } => vec![
                "Run: campus-lms auth status --json",
                "Run: campus-lms auth verify --json",
                "If no token is stored, run: campus-lms auth login",
                "If your university uses SSO/MFA, use an administrator-issued token with: campus-lms auth import-token",
            ],
            Self::AuthExpired { .. } => vec![
                "Run: campus-lms auth status --live --json",
                "Run: campus-lms auth login",
                "If password login is blocked by SSO/MFA, run: campus-lms auth import-token",
            ],
            Self::PermissionDenied { .. } => vec![
                "Run: campus-lms doctor --json",
                "Ask the LMS administrator whether Mobile Web Services are enabled for your account.",
            ],
            Self::UnsupportedMoodleFeature { .. } => vec![
                "Run: campus-lms doctor --json",
                "Ask the LMS administrator whether the missing Moodle Web Service function is enabled.",
            ],
            Self::Keychain { .. } => vec![
                "Run: campus-lms auth verify --json",
                "Check whether the OS credential store is available.",
                "On Windows, check Windows Credential Manager access and reinstall the latest campus-lms build.",
            ],
            Self::Network { .. } => vec![
                "Check the Moodle base URL.",
                "Check network, VPN, proxy, or captive portal requirements.",
                "Run: campus-lms doctor --json",
            ],
            Self::RateLimited { .. } => vec![
                "Wait and retry later.",
                "Use --offline when cached data is acceptable.",
            ],
            Self::MoodleApi { .. } => vec![
                "Run: campus-lms doctor --json",
                "Check whether Moodle Web Services are enabled for this site and token.",
            ],
            Self::Config { .. } => vec![
                "Run: campus-lms auth status --json",
                "Run: campus-lms init",
                "Run: campus-lms auth login",
            ],
            Self::InvalidArgument { .. } => vec!["Run the command with --help to see valid arguments."],
            _ => Vec::new(),
        }
    }

    pub fn json_requested(&self) -> bool {
        match self {
            Self::AuthRequired { json }
            | Self::AuthExpired { json }
            | Self::PermissionDenied { json }
            | Self::Network { json, .. }
            | Self::RateLimited { json }
            | Self::MoodleApi { json, .. }
            | Self::UnsupportedMoodleFeature { json, .. }
            | Self::InvalidArgument { json, .. }
            | Self::NotFound { json, .. }
            | Self::Config { json, .. }
            | Self::Keychain { json, .. }
            | Self::Cache { json, .. }
            | Self::Parse { json, .. }
            | Self::Unknown { json, .. } => *json,
        }
    }

    pub fn with_json(self, json: bool) -> Self {
        match self {
            Self::AuthRequired { .. } => Self::AuthRequired { json },
            Self::AuthExpired { .. } => Self::AuthExpired { json },
            Self::PermissionDenied { .. } => Self::PermissionDenied { json },
            Self::Network { message, .. } => Self::Network { message, json },
            Self::RateLimited { .. } => Self::RateLimited { json },
            Self::MoodleApi { message, .. } => Self::MoodleApi { message, json },
            Self::UnsupportedMoodleFeature { message, .. } => {
                Self::UnsupportedMoodleFeature { message, json }
            }
            Self::InvalidArgument { message, hint, .. } => Self::InvalidArgument {
                message,
                hint,
                json,
            },
            Self::NotFound { message, .. } => Self::NotFound { message, json },
            Self::Config { message, .. } => Self::Config { message, json },
            Self::Keychain { message, .. } => Self::Keychain { message, json },
            Self::Cache { message, .. } => Self::Cache { message, json },
            Self::Parse { message, .. } => Self::Parse { message, json },
            Self::Unknown { message, .. } => Self::Unknown { message, json },
        }
    }

    pub fn invalid_argument(message: impl Into<String>, hint: Option<&str>) -> Self {
        Self::InvalidArgument {
            message: message.into(),
            hint: hint.map(str::to_string),
            json: false,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            json: false,
        }
    }

    pub fn keychain(message: impl Into<String>) -> Self {
        Self::Keychain {
            message: message.into(),
            json: false,
        }
    }

    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache {
            message: message.into(),
            json: false,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse<'a> {
    pub schema_version: &'static str,
    pub error: ErrorBody<'a>,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody<'a> {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    pub hint: Option<&'a str>,
    pub next_steps: Vec<&'a str>,
}
