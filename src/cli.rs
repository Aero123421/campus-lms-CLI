// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "campus-lms",
    version,
    about = "A read-only CLI for Moodle-compatible university LMS sites.",
    after_help = "Recommended AI entrypoint:\n  campus-lms ai snapshot --days 14 --json\n\nSafety:\n  Read-only by default. Tokens and passwords are never printed."
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub profile: Option<String>,

    #[arg(long, global = true)]
    pub config: Option<std::path::PathBuf>,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true)]
    pub no_color: bool,

    #[arg(long, global = true)]
    pub verbose: bool,

    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Whoami,
    Doctor,
    Courses(CachedArgs),
    Todo(TodoArgs),
    Assignment {
        #[command(subcommand)]
        command: AssignmentCommand,
    },
    Ai {
        #[command(subcommand)]
        command: AiCommand,
    },
    Capabilities,
    Errors,
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    Init(InitArgs),
    Cleanup(CleanupArgs),
    Uninstall(UninstallArgs),
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Login(LoginArgs),
    Logout(LogoutArgs),
    Status,
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    #[arg(long)]
    pub base_url: Option<String>,

    #[arg(long)]
    pub username: Option<String>,

    #[arg(long, default_value = "moodle_mobile_app")]
    pub service: String,

    #[arg(long)]
    pub allow_insecure_localhost: bool,
}

#[derive(Debug, Args)]
pub struct LogoutArgs {
    #[arg(long)]
    pub keep_config: bool,
}

#[derive(Debug, Args, Clone)]
pub struct CachedArgs {
    #[arg(long)]
    pub refresh: bool,

    #[arg(long)]
    pub offline: bool,
}

#[derive(Debug, Args, Clone)]
pub struct TodoArgs {
    #[arg(long, default_value_t = 14)]
    pub days: u32,

    #[arg(long)]
    pub max_items: Option<usize>,

    #[arg(long)]
    pub refresh: bool,

    #[arg(long)]
    pub offline: bool,

    #[arg(long)]
    pub include_submitted: bool,

    #[arg(long)]
    pub course: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum AssignmentCommand {
    Show(AssignmentShowArgs),
}

#[derive(Debug, Args, Clone)]
pub struct AssignmentShowArgs {
    pub id: String,

    #[arg(long, default_value_t = 8000)]
    pub max_chars: usize,

    #[arg(long)]
    pub include_html: bool,

    #[arg(long)]
    pub refresh: bool,

    #[arg(long)]
    pub offline: bool,
}

#[derive(Debug, Subcommand)]
pub enum AiCommand {
    Snapshot(AiSnapshotArgs),
    Instructions,
}

#[derive(Debug, Args, Clone)]
pub struct AiSnapshotArgs {
    #[arg(long, default_value_t = 14)]
    pub days: u32,

    #[arg(long, default_value_t = 30)]
    pub max_items: usize,

    #[arg(long)]
    pub include_grades: bool,

    #[arg(long)]
    pub include_feedback: bool,

    #[arg(long)]
    pub refresh: bool,

    #[arg(long)]
    pub offline: bool,
}

#[derive(Debug, Subcommand)]
pub enum SchemaCommand {
    List,
    Show { name: String },
}

#[derive(Debug, Args, Clone)]
pub struct InitArgs {
    #[arg(
        long,
        help = "Recreate missing directories, but never overwrite existing config."
    )]
    pub force: bool,

    #[arg(long)]
    pub print_agents: bool,
}

#[derive(Debug, Args, Clone)]
pub struct CleanupArgs {
    #[arg(long)]
    pub all: bool,

    #[arg(long = "local-config")]
    pub local_config: bool,

    #[arg(long)]
    pub cache: bool,

    #[arg(long)]
    pub tokens: bool,

    #[arg(long)]
    pub yes: bool,

    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args, Clone)]
pub struct UninstallArgs {
    #[arg(long)]
    pub yes: bool,

    #[arg(long)]
    pub dry_run: bool,
}

pub fn ensure_cache_flags(refresh: bool, offline: bool) -> crate::error::Result<()> {
    if refresh && offline {
        return Err(crate::error::CampusError::invalid_argument(
            "--refresh and --offline cannot be used together.",
            Some("Choose either --refresh or --offline."),
        ));
    }
    Ok(())
}

pub fn ensure_profile_name(profile: &str) -> crate::error::Result<()> {
    let valid = !profile.is_empty()
        && profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if valid {
        Ok(())
    } else {
        Err(crate::error::CampusError::invalid_argument(
            "profile must contain only letters, numbers, dot, underscore, or hyphen.",
            Some("Use a profile name such as default, school, or my-university."),
        ))
    }
}

pub fn ensure_days(days: u32) -> crate::error::Result<()> {
    if (1..=365).contains(&days) {
        Ok(())
    } else {
        Err(crate::error::CampusError::invalid_argument(
            "--days must be between 1 and 365.",
            Some("For the AI entrypoint, use: campus-lms ai snapshot --days 14 --json"),
        ))
    }
}

pub fn ensure_max_items(max_items: Option<usize>) -> crate::error::Result<()> {
    match max_items {
        Some(0) => Err(crate::error::CampusError::invalid_argument(
            "--max-items must be at least 1.",
            None,
        )),
        Some(value) if value > 500 => Err(crate::error::CampusError::invalid_argument(
            "--max-items must be 500 or less.",
            None,
        )),
        _ => Ok(()),
    }
}

pub fn ensure_max_chars(max_chars: usize) -> crate::error::Result<()> {
    if max_chars <= 100_000 {
        Ok(())
    } else {
        Err(crate::error::CampusError::invalid_argument(
            "--max-chars must be 100000 or less.",
            None,
        ))
    }
}
