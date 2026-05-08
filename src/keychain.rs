// SPDX-License-Identifier: Apache-2.0

use keyring::Entry;

use crate::{config::Profile, error::CampusError};

pub fn service_name(base_url: &url::Url) -> String {
    format!("campus-lms:{}", base_url.as_str().trim_end_matches('/'))
}

pub fn set_token(profile: &Profile, token: &str) -> crate::error::Result<()> {
    let entry = Entry::new(&service_name(&profile.base_url), &profile.username)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    entry
        .set_password(token)
        .map_err(|err| CampusError::keychain(err.to_string()))
}

pub fn get_token(profile: &Profile) -> crate::error::Result<String> {
    let entry = Entry::new(&service_name(&profile.base_url), &profile.username)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    entry
        .get_password()
        .map_err(|_| CampusError::AuthRequired { json: false })
}

pub fn delete_token(profile: &Profile) -> crate::error::Result<()> {
    let entry = Entry::new(&service_name(&profile.base_url), &profile.username)
        .map_err(|err| CampusError::keychain(err.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(CampusError::keychain(err.to_string())),
    }
}
