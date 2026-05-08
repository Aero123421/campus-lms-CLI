// SPDX-License-Identifier: Apache-2.0

use std::{
    env,
    io::{self, Read, Write},
    time::Duration,
};

use reqwest::blocking::Client;
use serde::Deserialize;
use url::Url;
use zeroize::Zeroizing;

use crate::{
    cli::{AuthStatusArgs, AuthVerifyArgs, Cli, ImportTokenArgs, LoginArgs, LogoutArgs},
    config::{self, Profile},
    dto::{
        AuthImportTokenOutput, AuthLoginOutput, AuthLogoutOutput, AuthStatusOutput,
        AuthVerifyOutput, Warning,
    },
    error::CampusError,
    keychain,
    moodle::client::client_from_profile_data,
    output,
};

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: Option<String>,
    error: Option<String>,
    errorcode: Option<String>,
}

pub fn login(cli: &Cli, args: &LoginArgs) -> crate::error::Result<()> {
    if cli.json && (args.base_url.is_none() || args.username.is_none()) {
        return Err(CampusError::invalid_argument(
            "auth login --json requires --base-url and --username.",
            Some("Run: campus-lms auth login --base-url <URL> --username <USER> --password-stdin --json"),
        )
        .with_json(true));
    }
    if cli.json && !args.password_stdin && args.password_env.is_none() {
        return Err(CampusError::invalid_argument(
            "auth login --json requires --password-stdin or --password-env.",
            Some("Example: $env:MOODLE_PASSWORD='...'; campus-lms auth login --json --base-url <URL> --username <USER> --password-env MOODLE_PASSWORD"),
        )
        .with_json(true));
    }
    let base_url = match &args.base_url {
        Some(value) => value.clone(),
        None => prompt("base_url: ")?,
    };
    let base_url = normalize_base_url(&base_url, args.allow_insecure_localhost)?;
    let username = match &args.username {
        Some(value) => value.clone(),
        None => prompt("username: ")?,
    };
    let password = read_secret(
        args.password_stdin,
        args.password_env.as_deref(),
        "password: ",
        "password",
        cli.json,
    )?;

    let token = request_token(
        &base_url,
        &username,
        password.as_str(),
        &args.service,
        cli.json,
    )?;

    let profile = Profile {
        base_url,
        username,
        service: args.service.clone(),
        cache_ttl_seconds: config::default_cache_ttl_seconds(),
    };
    keychain::set_token(&profile, &token).map_err(|err| err.with_json(cli.json))?;
    keychain::verify_token_roundtrip(&profile, &token).map_err(|err| err.with_json(cli.json))?;

    let mut config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile_name = cli
        .profile
        .clone()
        .unwrap_or_else(|| config.active_profile.clone());
    config.active_profile = profile_name.clone();
    config.profile.insert(profile_name.clone(), profile.clone());
    config::save(cli, &config).map_err(|err| err.with_json(cli.json))?;

    if cli.json {
        output::print_json(&AuthLoginOutput {
            schema_version: "campus-lms.auth_login.v1",
            generated_at: output::generated_at(),
            authenticated: true,
            profile: profile_name,
            base_url: profile.base_url.to_string(),
            username: profile.username.clone(),
            credential_target: keychain::credential_target(&profile),
            token_verified: true,
            warnings: Vec::new(),
            next_steps: vec![
                "Run: campus-lms auth status --live --json".to_string(),
                "Run: campus-lms doctor --json".to_string(),
                "Run: campus-lms ai snapshot --days 14 --json".to_string(),
            ],
        })
    } else if !cli.quiet {
        println!("Logged in as {} for {}", profile.username, profile.base_url);
        println!("Token storage verified in the OS credential store.");
        Ok(())
    } else {
        Ok(())
    }
}

pub fn import_token(cli: &Cli, args: &ImportTokenArgs) -> crate::error::Result<()> {
    if cli.json && !args.token_stdin && args.token_env.is_none() {
        return Err(CampusError::invalid_argument(
            "auth import-token --json requires --token-stdin or --token-env.",
            Some("Example: $env:MOODLE_TOKEN='...'; campus-lms auth import-token --json --base-url <URL> --username <USER> --token-env MOODLE_TOKEN"),
        )
        .with_json(true));
    }
    let base_url = normalize_base_url(&args.base_url, args.allow_insecure_localhost)?;
    let token = read_secret(
        args.token_stdin,
        args.token_env.as_deref(),
        "token: ",
        "token",
        cli.json,
    )?;
    let profile = Profile {
        base_url,
        username: args.username.clone(),
        service: args.service.clone(),
        cache_ttl_seconds: config::default_cache_ttl_seconds(),
    };
    keychain::set_token(&profile, token.as_str()).map_err(|err| err.with_json(cli.json))?;
    keychain::verify_token_roundtrip(&profile, token.as_str())
        .map_err(|err| err.with_json(cli.json))?;

    let mut config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile_name = cli
        .profile
        .clone()
        .unwrap_or_else(|| config.active_profile.clone());
    config.active_profile = profile_name.clone();
    config.profile.insert(profile_name.clone(), profile.clone());
    config::save(cli, &config).map_err(|err| err.with_json(cli.json))?;

    if cli.json {
        output::print_json(&AuthImportTokenOutput {
            schema_version: "campus-lms.auth_import_token.v1",
            generated_at: output::generated_at(),
            authenticated: true,
            profile: profile_name,
            base_url: profile.base_url.to_string(),
            username: profile.username.clone(),
            credential_target: keychain::credential_target(&profile),
            token_verified: true,
            warnings: Vec::new(),
            next_steps: vec![
                "Run: campus-lms auth status --live --json".to_string(),
                "Run: campus-lms doctor --json".to_string(),
                "Run: campus-lms ai snapshot --days 14 --json".to_string(),
            ],
        })
    } else if !cli.quiet {
        println!(
            "Imported token for {} at {}",
            profile.username, profile.base_url
        );
        println!("Token storage verified in the OS credential store.");
        Ok(())
    } else {
        Ok(())
    }
}

pub fn logout(cli: &Cli, args: &LogoutArgs) -> crate::error::Result<()> {
    let mut config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config::active_profile(cli, &config).cloned();
    if let Ok(profile) = profile {
        keychain::delete_token(&profile).map_err(|err| err.with_json(cli.json))?;
    }
    if !args.keep_config {
        config::remove_active_profile(cli, &mut config);
        config::save(cli, &config).map_err(|err| err.with_json(cli.json))?;
    }

    if cli.json {
        output::print_json(&AuthLogoutOutput {
            schema_version: "campus-lms.auth_logout.v1",
            generated_at: output::generated_at(),
            logged_out: true,
            profile: profile_name,
            warnings: Vec::new(),
        })
    } else if !cli.quiet {
        println!("Logged out profile {profile_name}");
        Ok(())
    } else {
        Ok(())
    }
}

pub fn status(cli: &Cli, args: &AuthStatusArgs) -> crate::error::Result<()> {
    let json = cli.json;
    let config = config::load(cli).map_err(|err| err.with_json(json))?;
    let profile_name = config::selected_profile_name(cli, &config);
    let active_profile = config.active_profile.clone();
    let profile = config.profile.get(&profile_name);
    let config_path = config::config_path(cli)
        .ok()
        .map(|path| path.display().to_string());
    let mut warnings = Vec::new();
    let mut token_available = false;
    let mut token_readable = false;
    let mut live_ok = None;
    let mut live_status = if args.live {
        "not_checked".to_string()
    } else {
        "not_requested".to_string()
    };

    if let Some(profile) = profile {
        match keychain::get_token_detailed(profile) {
            Ok(_) => {
                token_available = true;
                token_readable = true;
            }
            Err(CampusError::AuthRequired { .. }) => {
                warnings.push(Warning::new(
                    "TOKEN_UNAVAILABLE",
                    "No token was found in the OS credential store.",
                    Some(
                        "Run campus-lms auth login, or auth import-token for SSO/MFA environments."
                            .to_string(),
                    ),
                ));
            }
            Err(err) => {
                warnings.push(Warning::new(
                    "KEYCHAIN_UNAVAILABLE",
                    err.to_string(),
                    Some(
                        "Run campus-lms auth verify --json to test the credential backend."
                            .to_string(),
                    ),
                ));
            }
        }
        if args.live && token_readable {
            match client_from_profile_data(cli, &profile_name, profile)
                .and_then(|client| client.site_info())
            {
                Ok(_) => {
                    live_ok = Some(true);
                    live_status = "verified".to_string();
                }
                Err(err) => {
                    live_ok = Some(false);
                    live_status = "failed".to_string();
                    warnings.push(Warning::new(
                        err.code(),
                        err.to_string(),
                        err.hint().map(str::to_string),
                    ));
                }
            }
        } else if args.live {
            live_ok = Some(false);
            live_status = "not_checked".to_string();
        }
    } else {
        warnings.push(Warning::new(
            "PROFILE_UNAVAILABLE",
            "No profile is configured for the selected profile name.",
            Some("Run campus-lms auth login or auth import-token.".to_string()),
        ));
    }

    let value = AuthStatusOutput {
        schema_version: "campus-lms.auth_status.v1",
        generated_at: output::generated_at(),
        authenticated: profile.is_some() && token_readable && live_ok.unwrap_or(true),
        profile: profile_name,
        active_profile,
        config_path,
        base_url: profile.map(|p| p.base_url.to_string()),
        username: profile.map(|p| p.username.clone()),
        credential_target: profile.map(keychain::credential_target),
        token_available,
        token_readable,
        live_check: live_check(&live_status),
        live_status,
        live_ok,
        warnings,
        next_steps: auth_next_steps(profile.is_some(), token_readable, live_ok),
    };

    if json {
        output::print_json(&value)
    } else {
        println!(
            "{}",
            if value.authenticated {
                "authenticated"
            } else {
                "not authenticated"
            }
        );
        Ok(())
    }
}

pub fn verify(cli: &Cli, args: &AuthVerifyArgs) -> crate::error::Result<()> {
    let config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile_name = config::selected_profile_name(cli, &config);
    let active_profile = config.active_profile.clone();
    let config_path = config::config_path(cli)
        .ok()
        .map(|path| path.display().to_string());
    let profile = config.profile.get(&profile_name);
    let mut warnings = Vec::new();
    let mut backend_roundtrip_ok = false;
    let mut token_available = false;
    let mut token_readable = false;
    let mut live_ok = None;
    let mut live_status = if args.live {
        "not_checked".to_string()
    } else {
        "not_requested".to_string()
    };

    if let Some(profile) = profile {
        match keychain::verify_backend_roundtrip(profile) {
            Ok(()) => backend_roundtrip_ok = true,
            Err(err) => warnings.push(Warning::new(
                "KEYCHAIN_ROUNDTRIP_FAILED",
                err.to_string(),
                Some(
                    "The OS credential store could not write/read/delete a test credential."
                        .to_string(),
                ),
            )),
        }

        match keychain::get_token_detailed(profile) {
            Ok(_) => {
                token_available = true;
                token_readable = true;
            }
            Err(CampusError::AuthRequired { .. }) => warnings.push(Warning::new(
                "TOKEN_UNAVAILABLE",
                "No Moodle token was found for this profile.",
                Some(
                    "Run campus-lms auth login, or auth import-token for SSO/MFA environments."
                        .to_string(),
                ),
            )),
            Err(err) => warnings.push(Warning::new(
                "TOKEN_READ_FAILED",
                err.to_string(),
                Some("The configured profile exists, but the token could not be read.".to_string()),
            )),
        }

        if args.live && token_readable {
            match client_from_profile_data(cli, &profile_name, profile)
                .and_then(|client| client.site_info())
            {
                Ok(_) => {
                    live_ok = Some(true);
                    live_status = "verified".to_string();
                }
                Err(err) => {
                    live_ok = Some(false);
                    live_status = "failed".to_string();
                    warnings.push(Warning::new(
                        err.code(),
                        err.to_string(),
                        err.hint().map(str::to_string),
                    ));
                }
            }
        } else if args.live {
            live_ok = Some(false);
            live_status = "not_checked".to_string();
        }
    } else {
        warnings.push(Warning::new(
            "PROFILE_UNAVAILABLE",
            "No profile is configured for the selected profile name.",
            Some("Run campus-lms auth login or auth import-token.".to_string()),
        ));
    }

    let value = AuthVerifyOutput {
        schema_version: "campus-lms.auth_verify.v1",
        generated_at: output::generated_at(),
        profile: profile_name,
        active_profile,
        config_path,
        profile_configured: profile.is_some(),
        base_url: profile.map(|p| p.base_url.to_string()),
        username: profile.map(|p| p.username.clone()),
        credential_target: profile.map(keychain::credential_target),
        backend_roundtrip_ok,
        credential_backend_ok: backend_roundtrip_ok,
        token_available,
        token_readable,
        live_check: live_check(&live_status),
        live_status,
        live_ok,
        warnings,
        next_steps: auth_next_steps(profile.is_some(), token_readable, live_ok),
    };

    if cli.json {
        output::print_json(&value)
    } else {
        println!(
            "{}",
            if value.profile_configured && value.backend_roundtrip_ok && value.token_readable {
                "auth verification passed"
            } else {
                "auth verification found issues"
            }
        );
        Ok(())
    }
}

fn read_secret(
    from_stdin: bool,
    from_env: Option<&str>,
    prompt_label: &str,
    secret_name: &str,
    json: bool,
) -> crate::error::Result<Zeroizing<String>> {
    if from_stdin && from_env.is_some() {
        return Err(CampusError::invalid_argument(
            format!("use only one of --{secret_name}-stdin or --{secret_name}-env"),
            None,
        )
        .with_json(json));
    }
    let value = if from_stdin {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|err| CampusError::Unknown {
                message: format!("failed to read {secret_name} from stdin: {err}"),
                json,
            })?;
        input.trim_end_matches(['\r', '\n']).to_string()
    } else if let Some(name) = from_env {
        env::var(name).map_err(|_| {
            CampusError::invalid_argument(
                format!("environment variable {name} is not set"),
                Some("Set the variable or use stdin."),
            )
            .with_json(json)
        })?
    } else {
        eprint!("{prompt_label}");
        io::stderr().flush().map_err(|err| CampusError::Unknown {
            message: err.to_string(),
            json,
        })?;
        rpassword::read_password().map_err(|err| CampusError::Unknown {
            message: format!("failed to read {secret_name}: {err}"),
            json,
        })?
    };
    if value.is_empty() {
        return Err(CampusError::invalid_argument(
            format!("{secret_name} must not be empty"),
            None,
        )
        .with_json(json));
    }
    Ok(Zeroizing::new(value))
}

fn auth_next_steps(
    profile_configured: bool,
    token_readable: bool,
    live_ok: Option<bool>,
) -> Vec<String> {
    if !profile_configured {
        return vec![
            "Run: campus-lms auth login".to_string(),
            "For SSO/MFA sites, run: campus-lms auth import-token".to_string(),
        ];
    }
    if !token_readable {
        return vec![
            "Run: campus-lms auth verify --json".to_string(),
            "Run: campus-lms auth login".to_string(),
            "For SSO/MFA sites, run: campus-lms auth import-token".to_string(),
        ];
    }
    if live_ok == Some(false) {
        return vec![
            "Run: campus-lms doctor --json".to_string(),
            "If the token is expired, run campus-lms auth login or auth import-token.".to_string(),
            "Ask the LMS administrator whether Mobile Web Services are enabled for this account."
                .to_string(),
        ];
    }
    vec![
        "Run: campus-lms doctor --json".to_string(),
        "Run: campus-lms ai snapshot --days 14 --json".to_string(),
    ]
}

fn live_check(live_status: &str) -> String {
    match live_status {
        "verified" => "ok",
        "failed" => "failed",
        "not_checked" => "not_checked",
        _ => "not_run",
    }
    .to_string()
}

fn prompt(label: &str) -> crate::error::Result<String> {
    eprint!("{label}");
    io::stderr().flush().map_err(|err| CampusError::Unknown {
        message: err.to_string(),
        json: false,
    })?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| CampusError::Unknown {
            message: err.to_string(),
            json: false,
        })?;
    Ok(input.trim().to_string())
}

pub(crate) fn normalize_base_url(
    input: &str,
    allow_insecure_localhost: bool,
) -> crate::error::Result<Url> {
    let mut url = Url::parse(input).map_err(|err| {
        CampusError::invalid_argument(
            format!("invalid base URL: {err}"),
            Some("Use a full URL such as https://lms.example.ac.jp/moodle/"),
        )
    })?;

    if !url.username().is_empty() || url.password().is_some() {
        return Err(CampusError::invalid_argument(
            "base_url must not contain username or password.",
            None,
        ));
    }
    url.set_query(None);
    url.set_fragment(None);

    let allowed_https = url.scheme() == "https";
    let allowed_localhost = allow_insecure_localhost
        && url.scheme() == "http"
        && matches!(
            url.host_str(),
            Some("localhost") | Some("127.0.0.1") | Some("::1")
        );
    if !allowed_https && !allowed_localhost {
        return Err(CampusError::invalid_argument(
            "auth login requires HTTPS base_url.",
            Some("Use HTTPS, or --allow-insecure-localhost for local development only."),
        ));
    }

    let path = url.path().to_string();
    if path.is_empty() {
        url.set_path("/");
    } else if !path.ends_with('/') {
        url.set_path(&format!("{path}/"));
    }

    Ok(url)
}

fn request_token(
    base_url: &Url,
    username: &str,
    password: &str,
    service: &str,
    json: bool,
) -> crate::error::Result<String> {
    let endpoint = base_url.join("login/token.php").map_err(|err| {
        CampusError::invalid_argument(format!("invalid token endpoint: {err}"), None)
    })?;
    let response = Client::builder()
        .user_agent(format!("campus-lms-cli/{}", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| CampusError::Network {
            message: err.to_string(),
            json,
        })?
        .post(endpoint)
        .form(&[
            ("username", username),
            ("password", password),
            ("service", service),
        ])
        .send()
        .map_err(|err| CampusError::Network {
            message: err.to_string(),
            json,
        })?;

    let status = response.status();
    if !status.is_success() {
        return match status.as_u16() {
            401 => Err(CampusError::AuthExpired { json }),
            403 => Err(CampusError::PermissionDenied { json }),
            429 => Err(CampusError::RateLimited { json }),
            _ => Err(CampusError::Network {
                message: format!("token endpoint returned HTTP {status}"),
                json,
            }),
        };
    }
    let body: TokenResponse = response.json().map_err(|err| CampusError::Parse {
        message: err.to_string(),
        json,
    })?;
    if let Some(token) = body.token {
        return Ok(token);
    }
    let message = body
        .error
        .or(body.errorcode)
        .unwrap_or_else(|| "login failed without a Moodle error message".to_string());
    Err(CampusError::MoodleApi { message, json })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_subdirectory_base_url_without_trailing_slash() {
        let url = normalize_base_url("https://example.ac.jp/moodle", false).unwrap();
        assert_eq!(
            url.join("webservice/rest/server.php").unwrap().as_str(),
            "https://example.ac.jp/moodle/webservice/rest/server.php"
        );
    }

    #[test]
    fn removes_query_fragment_and_rejects_credentials() {
        let url = normalize_base_url("https://example.ac.jp/lms?x=1#top", false).unwrap();
        assert_eq!(url.as_str(), "https://example.ac.jp/lms/");
        assert!(normalize_base_url("https://user@example.ac.jp/lms", false).is_err());
    }
}
