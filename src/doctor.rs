// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

use crate::{
    config,
    dto::{DoctorCheck, DoctorOutput, Warning},
    keychain,
    moodle::client::client_from_profile_data,
    output,
};

const REQUIRED_FUNCTIONS: &[&str] = &[
    "core_webservice_get_site_info",
    "core_enrol_get_users_courses",
    "mod_assign_get_assignments",
    "mod_assign_get_submission_status",
    "core_calendar_get_action_events_by_timesort",
];

pub fn run(cli: &crate::cli::Cli) -> crate::error::Result<()> {
    let config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile_name = config::selected_profile_name(cli, &config);
    let active_profile = config.active_profile.clone();
    let config_path = config::config_path(cli)
        .ok()
        .map(|path| path.display().to_string());
    let profile = match config.profile.get(&profile_name) {
        Some(profile) => profile,
        None => {
            return output::print_json(&DoctorOutput {
                schema_version: "campus-lms.doctor.v1",
                generated_at: output::generated_at(),
                profile: profile_name,
                active_profile,
                config_path,
                base_url: None,
                username: None,
                credential_target: None,
                authenticated: false,
                checks: vec![DoctorCheck {
                    name: "profile".to_string(),
                    ok: false,
                    detail: "No profile is configured.".to_string(),
                }],
                missing_functions: Vec::new(),
                unchecked_functions: REQUIRED_FUNCTIONS
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                warnings: vec![Warning::new(
                    "AUTH_REQUIRED",
                    "No campus-lms profile is configured.",
                    Some("Run campus-lms auth login. SSO/MFA-only Moodle sites may require an administrator-issued Web Services token instead of password login.".to_string()),
                )],
                next_steps: vec![
                    "Run: campus-lms auth login".to_string(),
                    "For SSO/MFA sites, run: campus-lms auth import-token".to_string(),
                ],
            });
        }
    };

    let mut checks = vec![DoctorCheck {
        name: "profile".to_string(),
        ok: true,
        detail: format!("Profile {profile_name} is configured."),
    }];
    let mut warnings = Vec::new();

    let credential_target = keychain::credential_target(profile);
    match keychain::verify_backend_roundtrip(profile) {
        Ok(()) => checks.push(DoctorCheck {
            name: "credential_backend".to_string(),
            ok: true,
            detail: format!(
                "Credential backend {} can store and read a test value.",
                credential_target.backend
            ),
        }),
        Err(err) => {
            checks.push(DoctorCheck {
                name: "credential_backend".to_string(),
                ok: false,
                detail: err.to_string(),
            });
            warnings.push(Warning::new(
                "KEYCHAIN_ROUNDTRIP_FAILED",
                err.to_string(),
                Some(
                    "The OS credential store could not write/read/delete a test credential."
                        .to_string(),
                ),
            ));
        }
    }

    let token_result = keychain::get_token_detailed(profile);
    let token_available = token_result.is_ok();
    checks.push(DoctorCheck {
        name: "token".to_string(),
        ok: token_available,
        detail: if token_available {
            "A token is available in the OS credential store.".to_string()
        } else {
            match &token_result {
                Err(err) if !matches!(err, crate::error::CampusError::AuthRequired { .. }) => {
                    format!("Token could not be read: {err}")
                }
                _ => "No token was found in the OS credential store.".to_string(),
            }
        },
    });

    if !token_available {
        warnings.push(Warning::new(
            "TOKEN_UNAVAILABLE",
            "The profile exists, but no Moodle token is available.",
            Some("Run campus-lms auth login. If the university uses SSO/MFA, ask whether Mobile Web Services or user token issuance is enabled.".to_string()),
        ));
        return output::print_json(&DoctorOutput {
            schema_version: "campus-lms.doctor.v1",
            generated_at: output::generated_at(),
            profile: profile_name,
            active_profile,
            config_path,
            base_url: Some(profile.base_url.to_string()),
            username: Some(profile.username.clone()),
            credential_target: Some(credential_target),
            authenticated: false,
            checks,
            missing_functions: Vec::new(),
            unchecked_functions: REQUIRED_FUNCTIONS
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            warnings,
            next_steps: vec![
                "Run: campus-lms auth verify --json".to_string(),
                "Run: campus-lms auth login".to_string(),
                "For SSO/MFA sites, run: campus-lms auth import-token".to_string(),
            ],
        });
    }

    let client = client_from_profile_data(cli, &profile_name, profile)
        .map_err(|err| err.with_json(cli.json))?;
    let site = match client.site_info() {
        Ok(site) => site,
        Err(err) => {
            checks.push(DoctorCheck {
                name: "core_webservice_get_site_info".to_string(),
                ok: false,
                detail: err.to_string(),
            });
            warnings.push(Warning::new(
                "SITE_INFO_UNAVAILABLE",
                err.to_string(),
                Some("Mobile Web Services may be disabled, the token may be expired, or the service may not include core_webservice_get_site_info.".to_string()),
            ));
            return output::print_json(&DoctorOutput {
                schema_version: "campus-lms.doctor.v1",
                generated_at: output::generated_at(),
                profile: profile_name,
                active_profile,
                config_path,
                base_url: Some(profile.base_url.to_string()),
                username: Some(profile.username.clone()),
                credential_target: Some(credential_target),
                authenticated: false,
                checks,
                missing_functions: Vec::new(),
                unchecked_functions: REQUIRED_FUNCTIONS
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                warnings,
                next_steps: vec![
                    "Run: campus-lms auth status --live --json".to_string(),
                    "Run: campus-lms auth login or auth import-token if the token is expired."
                        .to_string(),
                    "Ask the LMS administrator whether Mobile Web Services are enabled."
                        .to_string(),
                ],
            });
        }
    };

    checks.push(DoctorCheck {
        name: "core_webservice_get_site_info".to_string(),
        ok: true,
        detail: format!(
            "Connected as {} on {}.",
            site.username,
            site.sitename.unwrap_or_else(|| "Moodle".to_string())
        ),
    });

    let available = site
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut missing_functions = Vec::new();
    for required in REQUIRED_FUNCTIONS {
        let ok = available.contains(required);
        if !ok {
            missing_functions.push((*required).to_string());
        }
        checks.push(DoctorCheck {
            name: (*required).to_string(),
            ok,
            detail: if ok {
                "Available in the current Web Services token.".to_string()
            } else {
                "Missing from the current Web Services token.".to_string()
            },
        });
    }

    if !missing_functions.is_empty() {
        warnings.push(Warning::new(
            "MOODLE_FUNCTIONS_MISSING",
            "Some Moodle Web Service functions required by campus-lms are unavailable.",
            Some("Ask the LMS administrator whether Mobile Web Services are enabled and whether the token's service includes the missing functions.".to_string()),
        ));
    }

    let next_steps = if missing_functions.is_empty() {
        vec!["Run: campus-lms ai snapshot --days 14 --json".to_string()]
    } else {
        vec![
            "Ask the LMS administrator to enable the missing Web Service functions.".to_string(),
            "Use campus-lms auth import-token if a different service token is required."
                .to_string(),
        ]
    };

    output::print_json(&DoctorOutput {
        schema_version: "campus-lms.doctor.v1",
        generated_at: output::generated_at(),
        profile: profile_name,
        active_profile,
        config_path,
        base_url: Some(profile.base_url.to_string()),
        username: Some(profile.username.clone()),
        credential_target: Some(credential_target),
        authenticated: true,
        checks,
        missing_functions,
        unchecked_functions: Vec::new(),
        warnings,
        next_steps,
    })
}
