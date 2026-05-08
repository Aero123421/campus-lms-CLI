// SPDX-License-Identifier: Apache-2.0

use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

use crate::{
    cli::Cli,
    config,
    error::CampusError,
    keychain,
    moodle::{models::*, params::flatten_params},
};

pub struct MoodleClient {
    pub base_url: Url,
    token: String,
    http: Client,
}

pub fn client_from_profile(cli: &Cli) -> crate::error::Result<MoodleClient> {
    let config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile = config::active_profile(cli, &config).map_err(|err| err.with_json(cli.json))?;
    let token = keychain::get_token(profile).map_err(|err| err.with_json(cli.json))?;
    Ok(MoodleClient {
        base_url: profile.base_url.clone(),
        token,
        http: Client::builder()
            .user_agent(format!("campus-lms-cli/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|err| CampusError::Network {
                message: err.to_string(),
                json: cli.json,
            })?,
    })
}

impl MoodleClient {
    pub fn call<T: DeserializeOwned>(
        &self,
        function: &str,
        params: Value,
    ) -> crate::error::Result<T> {
        let endpoint = self
            .base_url
            .join("webservice/rest/server.php")
            .map_err(|err| {
                CampusError::invalid_argument(format!("invalid REST endpoint: {err}"), None)
            })?;
        let mut form = vec![
            ("wstoken".to_string(), self.token.clone()),
            ("wsfunction".to_string(), function.to_string()),
            ("moodlewsrestformat".to_string(), "json".to_string()),
        ];
        form.extend(flatten_params(&params));

        let response =
            self.http
                .post(endpoint)
                .form(&form)
                .send()
                .map_err(|err| CampusError::Network {
                    message: err.to_string(),
                    json: false,
                })?;
        let status = response.status();
        let text = response.text().map_err(|err| CampusError::Network {
            message: err.to_string(),
            json: false,
        })?;
        if !status.is_success() {
            return Err(CampusError::Network {
                message: format!("Moodle REST returned HTTP {status}"),
                json: false,
            });
        }
        if let Ok(exception) = serde_json::from_str::<MoodleException>(&text) {
            if exception.exception.is_some() || exception.errorcode.is_some() {
                let message = exception
                    .message
                    .or(exception.errorcode)
                    .unwrap_or_else(|| "Moodle API returned an error".to_string());
                return Err(CampusError::MoodleApi {
                    message,
                    json: false,
                });
            }
        }
        serde_json::from_str(&text).map_err(|err| CampusError::Parse {
            message: format!("{err}; Moodle response body could not be parsed as expected JSON"),
            json: false,
        })
    }

    pub fn site_info(&self) -> crate::error::Result<SiteInfo> {
        self.call("core_webservice_get_site_info", serde_json::json!({}))
    }

    pub fn user_courses(&self, user_id: i64) -> crate::error::Result<Vec<Course>> {
        self.call(
            "core_enrol_get_users_courses",
            serde_json::json!({ "userid": user_id }),
        )
    }
}
