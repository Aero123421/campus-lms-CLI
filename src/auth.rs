// SPDX-License-Identifier: Apache-2.0

use std::io::{self, Write};

use reqwest::blocking::Client;
use serde::Deserialize;
use url::Url;

use crate::{
    cli::{Cli, LoginArgs, LogoutArgs},
    config::{self, Profile},
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
    let password = rpassword::read_password().map_err(|err| CampusError::Unknown {
        message: format!("failed to read password: {err}"),
        json: cli.json,
    })?;

    let token = request_token(&base_url, &username, &password, &args.service, cli.json)?;
    drop(password);

    let profile = Profile {
        base_url,
        username,
        service: args.service.clone(),
        cache_ttl_seconds: config::default_cache_ttl_seconds(),
    };
    keychain::set_token(&profile, &token).map_err(|err| err.with_json(cli.json))?;

    let mut config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    config.active_profile = cli.profile.clone();
    config.profile.insert(cli.profile.clone(), profile.clone());
    config::save(cli, &config).map_err(|err| err.with_json(cli.json))?;

    if cli.json {
        output::print_json(&serde_json::json!({
            "schema_version": "campus-lms.auth_login.v1",
            "generated_at": output::generated_at(),
            "authenticated": true,
            "profile": cli.profile.as_str(),
            "base_url": profile.base_url.as_str(),
            "username": profile.username.as_str(),
            "warnings": []
        }))
    } else if !cli.quiet {
        println!("Logged in as {} for {}", profile.username, profile.base_url);
        Ok(())
    } else {
        Ok(())
    }
}

pub fn logout(cli: &Cli, args: &LogoutArgs) -> crate::error::Result<()> {
    let mut config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile = config::active_profile(cli, &config).cloned();
    if let Ok(profile) = profile {
        keychain::delete_token(&profile).map_err(|err| err.with_json(cli.json))?;
    }
    if !args.keep_config {
        config::remove_active_profile(cli, &mut config);
        config::save(cli, &config).map_err(|err| err.with_json(cli.json))?;
    }

    if cli.json {
        output::print_json(&serde_json::json!({
            "schema_version": "campus-lms.auth_logout.v1",
            "generated_at": output::generated_at(),
            "logged_out": true,
            "profile": cli.profile.as_str(),
            "warnings": []
        }))
    } else if !cli.quiet {
        println!("Logged out profile {}", cli.profile);
        Ok(())
    } else {
        Ok(())
    }
}

pub fn status(cli: &Cli, json: bool) -> crate::error::Result<()> {
    let config = config::load(cli).map_err(|err| err.with_json(json))?;
    let profile = config.profile.get(&cli.profile);
    let token_available = match profile {
        Some(profile) => keychain::get_token(profile).is_ok(),
        None => false,
    };

    let value = serde_json::json!({
        "schema_version": "campus-lms.auth_status.v1",
        "generated_at": output::generated_at(),
        "authenticated": profile.is_some() && token_available,
        "profile": cli.profile.as_str(),
        "base_url": profile.map(|p| p.base_url.as_str()),
        "username": profile.map(|p| p.username.as_str()),
        "token_available": token_available,
        "warnings": []
    });

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

fn normalize_base_url(input: &str, allow_insecure_localhost: bool) -> crate::error::Result<Url> {
    let url = Url::parse(input).map_err(|err| {
        CampusError::invalid_argument(
            format!("invalid base URL: {err}"),
            Some("Use a full URL such as https://lms.example.ac.jp"),
        )
    })?;
    if url.scheme() == "https" {
        return Ok(url);
    }
    if allow_insecure_localhost
        && url.scheme() == "http"
        && matches!(url.host_str(), Some("localhost") | Some("127.0.0.1"))
    {
        return Ok(url);
    }
    Err(CampusError::invalid_argument(
        "auth login requires HTTPS base_url.",
        Some("Use HTTPS, or --allow-insecure-localhost for http://localhost / http://127.0.0.1 only."),
    ))
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
    let body: TokenResponse = response.json().map_err(|err| CampusError::Parse {
        message: err.to_string(),
        json,
    })?;

    if !status.is_success() {
        return Err(CampusError::Network {
            message: format!("token endpoint returned HTTP {status}"),
            json,
        });
    }
    if let Some(token) = body.token {
        return Ok(token);
    }
    let message = body
        .error
        .or(body.errorcode)
        .unwrap_or_else(|| "login failed without a Moodle error message".to_string());
    Err(CampusError::MoodleApi { message, json })
}
