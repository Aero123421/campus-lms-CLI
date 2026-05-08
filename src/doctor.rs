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
    let profile = match config.profile.get(&profile_name) {
        Some(profile) => profile,
        None => {
            return output::print_json(&DoctorOutput {
                schema_version: "campus-lms.doctor.v1",
                generated_at: output::generated_at(),
                profile: profile_name,
                base_url: None,
                authenticated: false,
                checks: vec![DoctorCheck {
                    name: "profile".to_string(),
                    ok: false,
                    detail: "No profile is configured.".to_string(),
                }],
                missing_functions: REQUIRED_FUNCTIONS
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                warnings: vec![Warning::new(
                    "AUTH_REQUIRED",
                    "No campus-lms profile is configured.",
                    Some("Run campus-lms auth login. SSO/MFA-only Moodle sites may require an administrator-issued Web Services token instead of password login.".to_string()),
                )],
            });
        }
    };

    let mut checks = vec![DoctorCheck {
        name: "profile".to_string(),
        ok: true,
        detail: format!("Profile {profile_name} is configured."),
    }];
    let mut warnings = Vec::new();

    let token_available = keychain::get_token(profile).is_ok();
    checks.push(DoctorCheck {
        name: "token".to_string(),
        ok: token_available,
        detail: if token_available {
            "A token is available in the OS credential store.".to_string()
        } else {
            "No token was found in the OS credential store.".to_string()
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
            base_url: Some(profile.base_url.to_string()),
            authenticated: false,
            checks,
            missing_functions: REQUIRED_FUNCTIONS
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            warnings,
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
                base_url: Some(profile.base_url.to_string()),
                authenticated: false,
                checks,
                missing_functions: REQUIRED_FUNCTIONS
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                warnings,
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

    output::print_json(&DoctorOutput {
        schema_version: "campus-lms.doctor.v1",
        generated_at: output::generated_at(),
        profile: profile_name,
        base_url: Some(profile.base_url.to_string()),
        authenticated: true,
        checks,
        missing_functions,
        warnings,
    })
}
