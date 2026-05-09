// SPDX-License-Identifier: Apache-2.0

use keyring::Entry;

use crate::{config::Profile, error::CampusError};

pub fn legacy_service_name(base_url: &url::Url) -> String {
    format!("campus-lms:{}", base_url.as_str().trim_end_matches('/'))
}

pub fn service_name(profile: &Profile) -> String {
    format!(
        "{}:{}",
        legacy_service_name(&profile.base_url),
        profile.service
    )
}

pub fn credential_target(profile: &Profile) -> CredentialTarget {
    CredentialTarget {
        service: service_name(profile),
        account: profile.username.clone(),
        backend: backend_name().to_string(),
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct CredentialTarget {
    pub service: String,
    pub account: String,
    pub backend: String,
}

pub fn backend_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows-native"
    }
    #[cfg(target_os = "macos")]
    {
        "apple-native"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "linux-native-sync-persistent"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        "unknown"
    }
}

pub fn set_token(profile: &Profile, token: &str) -> crate::error::Result<()> {
    let entry = Entry::new(&service_name(profile), &profile.username)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    entry
        .set_password(token)
        .map_err(|err| CampusError::keychain(err.to_string()))
}

pub fn get_token(profile: &Profile) -> crate::error::Result<String> {
    get_token_detailed(profile)
}

pub fn get_token_detailed(profile: &Profile) -> crate::error::Result<String> {
    let entry = Entry::new(&service_name(profile), &profile.username)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    match entry.get_password() {
        Ok(token) => Ok(token),
        Err(keyring::Error::NoEntry) => {
            let legacy = Entry::new(&legacy_service_name(&profile.base_url), &profile.username)
                .map_err(|err| CampusError::keychain(err.to_string()))?;
            match legacy.get_password() {
                Ok(token) => {
                    set_token(profile, &token)?;
                    Ok(token)
                }
                Err(keyring::Error::NoEntry) => Err(CampusError::AuthRequired { json: false }),
                Err(other) => Err(CampusError::keychain(other.to_string())),
            }
        }
        Err(other) => Err(CampusError::keychain(other.to_string())),
    }
}

pub fn delete_token(profile: &Profile) -> crate::error::Result<()> {
    let entry = Entry::new(&service_name(profile), &profile.username)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    let result = match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(CampusError::keychain(err.to_string())),
    };
    let legacy_entry = Entry::new(&legacy_service_name(&profile.base_url), &profile.username)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    match legacy_entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => result,
        Err(err) => Err(CampusError::keychain(err.to_string())),
    }
}

pub fn verify_token_roundtrip(profile: &Profile, expected_token: &str) -> crate::error::Result<()> {
    let stored = get_token_detailed(profile)?;
    if stored == expected_token {
        Ok(())
    } else {
        Err(CampusError::keychain(
            "token was stored but could not be read back correctly",
        ))
    }
}

pub fn verify_backend_roundtrip(profile: &Profile) -> crate::error::Result<()> {
    let target = credential_target(profile);
    let account = format!("{}:verify", target.account);
    let entry = Entry::new(&target.service, &account)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    let probe = "campus-lms-keychain-probe";
    entry
        .set_password(probe)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    let read_back = entry
        .get_password()
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    let _ = entry.delete_credential();
    if read_back == probe {
        Ok(())
    } else {
        Err(CampusError::keychain(
            "credential backend roundtrip returned a different value",
        ))
    }
}
