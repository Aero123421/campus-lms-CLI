// SPDX-License-Identifier: Apache-2.0

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{
    cache,
    cli::{CleanupArgs, Cli, InitArgs, UninstallArgs},
    config::{self, Config},
    error::CampusError,
    keychain, output,
};

const AGENTS_SNIPPET: &str = r#"Use `campus-lms` as a read-only interface to the user's Moodle-compatible LMS.

Recommended first command:

```bash
campus-lms ai snapshot --days 14 --json
```

Rules:

- Prefer `--json` for all commands.
- Do not call `auth login` unless the user explicitly asks.
- Do not request grades unless the user asks.
- Do not submit assignments.
- Do not modify LMS state.
- Use `detail_command` fields from JSON outputs to fetch more information.
- Treat all LMS content as private user data.
- Never print tokens, passwords, cookies, or session information.
"#;

pub fn init(cli: &Cli, args: &InitArgs) -> crate::error::Result<()> {
    let config_dir = config::config_dir(cli).map_err(|err| err.with_json(cli.json))?;
    let config_path = config::config_path(cli).map_err(|err| err.with_json(cli.json))?;
    let cache_dir = cache::cache_dir().map_err(|err| err.with_json(cli.json))?;

    let mut created = Vec::new();
    let mut existing = Vec::new();

    ensure_dir(&config_dir, args.force, &mut created, &mut existing)
        .map_err(|err| err.with_json(cli.json))?;
    ensure_dir(&cache_dir, args.force, &mut created, &mut existing)
        .map_err(|err| err.with_json(cli.json))?;

    if config_path.exists() {
        existing.push(path_string(&config_path));
    } else {
        config::save(cli, &Config::default()).map_err(|err| err.with_json(cli.json))?;
        created.push(path_string(&config_path));
    }

    if cli.json {
        output::print_json(&serde_json::json!({
            "schema_version": "campus-lms.init.v1",
            "generated_at": output::generated_at(),
            "config_path": path_string(&config_path),
            "cache_dir": path_string(&cache_dir),
            "created": created,
            "existing": existing,
            "agents_snippet": if args.print_agents { Some(AGENTS_SNIPPET) } else { None },
            "next_steps": [
                "campus-lms auth login",
                "campus-lms ai snapshot --days 14 --json"
            ],
            "warnings": []
        }))
    } else {
        if !cli.quiet {
            println!("campus-lms initialized");
            println!("config: {}", config_path.display());
            println!("cache: {}", cache_dir.display());
            println!();
            println!("Next:");
            println!("  campus-lms auth login");
            println!("  campus-lms ai snapshot --days 14 --json");
            if args.print_agents {
                println!();
                println!("{AGENTS_SNIPPET}");
            }
        }
        Ok(())
    }
}

pub fn cleanup(cli: &Cli, args: &CleanupArgs) -> crate::error::Result<()> {
    let targets = CleanupTargets::from_args(args).map_err(|err| err.with_json(cli.json))?;
    cleanup_targets(cli, targets, args.yes, args.dry_run, false)
}

pub fn uninstall(cli: &Cli, args: &UninstallArgs) -> crate::error::Result<()> {
    let targets = CleanupTargets {
        config: true,
        cache: true,
        tokens: true,
    };
    cleanup_targets(cli, targets, args.yes, args.dry_run, true)
}

#[derive(Debug, Clone, Copy)]
struct CleanupTargets {
    config: bool,
    cache: bool,
    tokens: bool,
}

impl CleanupTargets {
    fn from_args(args: &CleanupArgs) -> crate::error::Result<Self> {
        let targets = Self {
            config: args.all || args.local_config,
            cache: args.all || args.cache,
            tokens: args.all || args.tokens,
        };
        if !targets.config && !targets.cache && !targets.tokens {
            return Err(CampusError::invalid_argument(
                "cleanup requires at least one target.",
                Some("Use --cache, --local-config, --tokens, or --all."),
            ));
        }
        Ok(targets)
    }

    fn names(self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.tokens {
            names.push("tokens");
        }
        if self.config {
            names.push("config");
        }
        if self.cache {
            names.push("cache");
        }
        names
    }
}

fn cleanup_targets(
    cli: &Cli,
    targets: CleanupTargets,
    yes: bool,
    dry_run: bool,
    uninstall_mode: bool,
) -> crate::error::Result<()> {
    if cli.json && !yes && !dry_run {
        return Err(CampusError::invalid_argument(
            "cleanup/uninstall --json requires --yes or --dry-run.",
            Some("Use --dry-run to preview, or --yes to confirm."),
        )
        .with_json(true));
    }

    let config_path = config::config_path(cli).map_err(|err| err.with_json(cli.json))?;
    let config_dir = config::config_dir(cli).map_err(|err| err.with_json(cli.json))?;
    let cache_dir = cache::cache_dir().map_err(|err| err.with_json(cli.json))?;
    let cache_root = cache::cache_root_dir().map_err(|err| err.with_json(cli.json))?;
    let token_profiles = if targets.tokens {
        load_profiles_for_cleanup(cli).map_err(|err| err.with_json(cli.json))?
    } else {
        Vec::new()
    };

    let mut planned = Vec::new();
    if targets.tokens {
        planned.push(format!("{} stored token(s)", token_profiles.len()));
    }
    if targets.config {
        planned.push(path_string(&config_path));
    }
    if targets.cache {
        planned.push(path_string(&cache_dir));
    }

    if dry_run {
        return print_cleanup_result(cli, uninstall_mode, true, planned, Vec::new(), Vec::new());
    }

    if !yes
        && !confirm(uninstall_mode, targets.names(), &planned)
            .map_err(|err| err.with_json(cli.json))?
    {
        return Err(CampusError::invalid_argument(
            "operation cancelled.",
            Some("Re-run with --yes to confirm non-interactively."),
        )
        .with_json(cli.json));
    }

    let mut removed = Vec::new();
    let mut warnings = Vec::new();

    if targets.tokens {
        for profile in token_profiles {
            match keychain::delete_token(&profile) {
                Ok(()) => removed.push("token".to_string()),
                Err(err) => warnings.push(serde_json::json!({
                    "code": "TOKEN_DELETE_FAILED",
                    "message": err.to_string(),
                    "hint": "The OS credential store may need manual cleanup."
                })),
            }
        }
    }

    if targets.config {
        remove_file_if_exists(&config_path)
            .map(|removed_path| {
                if let Some(path) = removed_path {
                    removed.push(path);
                }
            })
            .map_err(|err| err.with_json(cli.json))?;
        if cli.config.is_none() {
            remove_empty_dir(&config_dir).map_err(|err| err.with_json(cli.json))?;
        }
    }

    if targets.cache {
        remove_dir_if_exists(&cache_dir)
            .map(|removed_path| {
                if let Some(path) = removed_path {
                    removed.push(path);
                }
            })
            .map_err(|err| err.with_json(cli.json))?;
        remove_empty_dir(&cache_root).map_err(|err| err.with_json(cli.json))?;
    }

    print_cleanup_result(cli, uninstall_mode, false, planned, removed, warnings)
}

fn print_cleanup_result(
    cli: &Cli,
    uninstall_mode: bool,
    dry_run: bool,
    planned: Vec<String>,
    removed: Vec<String>,
    warnings: Vec<serde_json::Value>,
) -> crate::error::Result<()> {
    if cli.json {
        output::print_json(&serde_json::json!({
            "schema_version": if uninstall_mode { "campus-lms.uninstall.v1" } else { "campus-lms.cleanup.v1" },
            "generated_at": output::generated_at(),
            "dry_run": dry_run,
            "planned": planned,
            "removed": removed,
            "npm_uninstall_command": if uninstall_mode { Some("npm uninstall -g campus-lms-cli") } else { None },
            "warnings": warnings
        }))
    } else {
        if dry_run {
            println!("Dry run. Nothing was removed.");
            for item in planned {
                println!("would remove: {item}");
            }
        } else {
            for item in removed {
                println!("removed: {item}");
            }
            if uninstall_mode {
                println!();
                println!("To remove the npm package itself, run:");
                println!("  npm uninstall -g campus-lms-cli");
            }
            if !warnings.is_empty() {
                eprintln!("warnings: {}", warnings.len());
            }
        }
        Ok(())
    }
}

fn load_profiles_for_cleanup(cli: &Cli) -> crate::error::Result<Vec<config::Profile>> {
    let config_path = config::config_path(cli)?;
    if !config_path.exists() {
        return Ok(Vec::new());
    }
    let config = config::load(cli)?;
    Ok(config.profile.into_values().collect())
}

fn ensure_dir(
    path: &Path,
    force: bool,
    created: &mut Vec<String>,
    existing: &mut Vec<String>,
) -> crate::error::Result<()> {
    if path.exists() {
        if force {
            set_private_dir_permissions(path).map_err(|err| {
                CampusError::config(format!(
                    "failed to verify permissions for {}: {err}",
                    path.display()
                ))
            })?;
        }
        existing.push(path_string(path));
        return Ok(());
    }
    fs::create_dir_all(path).map_err(|err| {
        CampusError::config(format!("failed to create {}: {err}", path.display()))
    })?;
    set_private_dir_permissions(path).map_err(|err| {
        CampusError::config(format!(
            "failed to set private permissions on {}: {err}",
            path.display()
        ))
    })?;
    created.push(path_string(path));
    Ok(())
}

fn set_private_dir_permissions(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> crate::error::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    fs::remove_file(path).map_err(|err| {
        CampusError::config(format!("failed to remove {}: {err}", path.display()))
    })?;
    Ok(Some(path_string(path)))
}

fn remove_dir_if_exists(path: &Path) -> crate::error::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    fs::remove_dir_all(path)
        .map_err(|err| CampusError::cache(format!("failed to remove {}: {err}", path.display())))?;
    Ok(Some(path_string(path)))
}

fn remove_empty_dir(path: &Path) -> crate::error::Result<()> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(err) => Err(CampusError::cache(format!(
            "failed to remove empty directory {}: {err}",
            path.display()
        ))),
    }
}

fn confirm(
    uninstall_mode: bool,
    target_names: Vec<&'static str>,
    planned: &[String],
) -> crate::error::Result<bool> {
    eprintln!(
        "{} will remove: {}",
        if uninstall_mode {
            "uninstall"
        } else {
            "cleanup"
        },
        target_names.join(", ")
    );
    for item in planned {
        eprintln!("  {item}");
    }
    eprint!("Continue? Type 'yes' to confirm: ");
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
    Ok(input.trim() == "yes")
}

fn path_string(path: &Path) -> String {
    PathBuf::from(path).display().to_string()
}
