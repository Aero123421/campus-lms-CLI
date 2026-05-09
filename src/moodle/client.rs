// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use reqwest::{blocking::Client, redirect::Policy};
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    cli::Cli,
    config::{self, Profile},
    error::CampusError,
    keychain,
    moodle::{api::MoodleApi, models::*, params::flatten_params},
};

pub struct MoodleClient {
    pub base_url: Url,
    token: Zeroizing<String>,
    http: Client,
}

pub fn client_from_profile(cli: &Cli) -> crate::error::Result<MoodleClient> {
    let config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config::active_profile(cli, &config).map_err(|err| err.with_json(cli.json))?;
    client_from_profile_data(cli, &profile_name, profile)
}

pub fn client_from_profile_data(
    cli: &Cli,
    _profile_name: &str,
    profile: &Profile,
) -> crate::error::Result<MoodleClient> {
    let token = keychain::get_token(profile).map_err(|err| err.with_json(cli.json))?;
    Ok(MoodleClient {
        base_url: profile.base_url.clone(),
        token: Zeroizing::new(token),
        http: Client::builder()
            .user_agent(format!("campus-lms-cli/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .redirect(Policy::none())
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
            ("wstoken".to_string(), self.token.as_str().to_string()),
            ("wsfunction".to_string(), function.to_string()),
            ("moodlewsrestformat".to_string(), "json".to_string()),
        ];
        form.extend(flatten_params(&params));

        let response_result = self.http.post(endpoint).form(&form).send();
        for (_, value) in &mut form {
            value.zeroize();
        }
        let response = response_result.map_err(|err| CampusError::Network {
            message: err.to_string(),
            json: false,
        })?;
        let status = response.status();
        let text = response.text().map_err(|err| CampusError::Network {
            message: err.to_string(),
            json: false,
        })?;
        if !status.is_success() {
            return match status.as_u16() {
                401 => Err(CampusError::AuthExpired { json: false }),
                403 => Err(CampusError::PermissionDenied { json: false }),
                429 => Err(CampusError::RateLimited { json: false }),
                _ => Err(CampusError::Network {
                    message: format!("Moodle REST returned HTTP {status}"),
                    json: false,
                }),
            };
        }
        if let Ok(exception) = serde_json::from_str::<MoodleException>(&text) {
            if exception.exception.is_some() || exception.errorcode.is_some() {
                return Err(classify_moodle_exception(exception));
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
            serde_json::json!({
                "userid": user_id,
                "returnusercount": false
            }),
        )
    }

    pub fn assignments(&self, course_ids: &[i64]) -> crate::error::Result<AssignmentsResponse> {
        let params = if course_ids.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::json!({ "courseids": course_ids })
        };
        self.call("mod_assign_get_assignments", params)
    }

    pub fn submission_status(
        &self,
        assign_id: i64,
    ) -> crate::error::Result<SubmissionStatusResponse> {
        self.call(
            "mod_assign_get_submission_status",
            serde_json::json!({ "assignid": assign_id }),
        )
    }

    pub fn action_events_by_timesort(
        &self,
        from: i64,
        to: i64,
        after_event_id: i64,
        limit_num: i64,
    ) -> crate::error::Result<ActionEventsResponse> {
        self.call(
            "core_calendar_get_action_events_by_timesort",
            serde_json::json!({
                "timesortfrom": from,
                "timesortto": to,
                "aftereventid": after_event_id,
                "limitnum": limit_num,
                "limittononsuspendedevents": true
            }),
        )
    }
}

impl MoodleApi for MoodleClient {
    fn site_info(&self) -> crate::error::Result<SiteInfo> {
        self.site_info()
    }

    fn user_courses(&self, user_id: i64) -> crate::error::Result<Vec<Course>> {
        self.user_courses(user_id)
    }

    fn assignments(&self, course_ids: &[i64]) -> crate::error::Result<AssignmentsResponse> {
        self.assignments(course_ids)
    }

    fn submission_status(&self, assign_id: i64) -> crate::error::Result<SubmissionStatusResponse> {
        self.submission_status(assign_id)
    }

    fn action_events_by_timesort(
        &self,
        from: i64,
        to: i64,
        after_event_id: i64,
        limit_num: i64,
    ) -> crate::error::Result<ActionEventsResponse> {
        self.action_events_by_timesort(from, to, after_event_id, limit_num)
    }
}

fn classify_moodle_exception(exception: MoodleException) -> CampusError {
    let message = exception
        .message
        .clone()
        .or_else(|| exception.errorcode.clone())
        .unwrap_or_else(|| "Moodle API returned an error".to_string());
    match exception.errorcode.as_deref() {
        Some("invalidtoken") => CampusError::AuthExpired { json: false },
        Some("accessexception") | Some("nopermission") => {
            CampusError::PermissionDenied { json: false }
        }
        Some("servicenotavailable") => CampusError::UnsupportedMoodleFeature {
            message,
            json: false,
        },
        _ => CampusError::MoodleApi {
            message,
            json: false,
        },
    }
}
