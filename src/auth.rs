// SPDX-License-Identifier: Apache-2.0

use std::{
    io::{self, Write},
    time::Duration,
};

use reqwest::blocking::Client;
use serde::Deserialize;
use url::Url;
use zeroize::Zeroizing;

use crate::{
    cli::{Cli, LoginArgs, LogoutArgs},
    config::{self, Profile},
    dto::{AuthLoginOutput, AuthLogoutOutput, AuthStatusOutput},
    error::CampusError,
    keychain, output,
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
            Some("Run: campus-lms auth login --base-url <URL> --username <USER> --json"),
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
    eprint!("password: ");
    io::stderr().flush().map_err(|err| CampusError::Unknown {
        message: err.to_string(),
        json: cli.json,
    })?;
    let password =
        Zeroizing::new(
            rpassword::read_password().map_err(|err| CampusError::Unknown {
                message: format!("failed to read password: {err}"),
                json: cli.json,
            })?,
        );

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
            warnings: Vec::new(),
        })
    } else if !cli.quiet {
        println!("Logged in as {} for {}", profile.username, profile.base_url);
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

pub fn status(cli: &Cli, json: bool) -> crate::error::Result<()> {
    let config = config::load(cli).map_err(|err| err.with_json(json))?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config.profile.get(&profile_name);
    let token_available = match profile {
        Some(profile) => keychain::get_token(profile).is_ok(),
        None => false,
    };

    let value = AuthStatusOutput {
        schema_version: "campus-lms.auth_status.v1",
        generated_at: output::generated_at(),
        authenticated: profile.is_some() && token_available,
        profile: profile_name,
        base_url: profile.map(|p| p.base_url.to_string()),
        username: profile.map(|p| p.username.clone()),
        token_available,
        warnings: Vec::new(),
    };

    if json {
        output::print_json(&value)
    } else {
        println!(
            "{}",
            if profile.is_some() && token_available {
                "authenticated"
            } else {
                "not authenticated"
            }
        );
        Ok(())
    }
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
