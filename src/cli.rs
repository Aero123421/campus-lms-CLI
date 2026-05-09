// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "campus-lms",
    version,
    about = "A read-only CLI for Moodle-compatible university LMS sites.",
    long_about = "campus-lms reads Moodle-compatible university LMS data through Moodle Web Services and prints machine-friendly JSON for users, scripts, and AI tools.\n\nIt is designed for checking courses, upcoming tasks, assignment details, and connection health without submitting, editing, deleting, posting, or changing completion state.",
    after_help = "Quick start:\n  npm install -g https://github.com/Aero123421/campus-lms-CLI/archive/refs/heads/main.tar.gz\n  campus-lms init\n  campus-lms auth login\n  campus-lms ai snapshot --days 14 --json\n\nCommon checks:\n  campus-lms doctor --json\n  campus-lms todo --days 14 --json\n  campus-lms assignment show assign:12345 --json\n\nSafety:\n  Read-only by default. Tokens and passwords are never printed.\n  If your university uses SSO/MFA, Moodle Web Services or an admin-issued token may be required."
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "NAME",
        help = "Use a named profile from the config file",
        long_help = "Use a named profile from the config file. If omitted, campus-lms uses the active profile recorded during auth login.\n\nAllowed characters: letters, numbers, dot, underscore, hyphen."
    )]
    pub profile: Option<String>,

    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Use a custom config.toml path"
    )]
    pub config: Option<std::path::PathBuf>,

    #[arg(
        long,
        global = true,
        help = "Print JSON output when the command supports it"
    )]
    pub json: bool,

    #[arg(
        long,
        global = true,
        hide = true,
        help = "Reserved compatibility flag; current output does not use colors"
    )]
    pub no_color: bool,

    #[arg(
        long,
        global = true,
        help = "Print extra diagnostic details when available"
    )]
    pub verbose: bool,

    #[arg(
        long,
        global = true,
        value_name = "N|all",
        help = "Include this many warning detail rows in JSON output; default is 0, --verbose implies all"
    )]
    pub warning_details: Option<String>,

    #[arg(long, global = true, help = "Reduce human-readable output")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Log in, log out, or check local authentication state")]
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    #[command(about = "Show the Moodle user linked to the current token as JSON")]
    Whoami,
    #[command(about = "Diagnose profile, token, and required Moodle Web Service functions")]
    Doctor,
    #[command(about = "List visible Moodle courses as JSON")]
    Courses(CachedArgs),
    #[command(about = "List upcoming tasks, deadlines, and calendar actions as JSON")]
    Todo(TodoArgs),
    #[command(about = "Read assignment details without changing LMS state")]
    Assignment {
        #[command(subcommand)]
        command: AssignmentCommand,
    },
    #[command(about = "Commands intended for AI or script integration")]
    Ai {
        #[command(subcommand)]
        command: AiCommand,
    },
    #[command(about = "Print machine-readable command capabilities")]
    Capabilities,
    #[command(about = "Print machine-readable error code documentation")]
    Errors,
    #[command(about = "List or show JSON schemas for command outputs")]
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    #[command(about = "Create local config/cache directories")]
    Init(InitArgs),
    #[command(about = "Remove selected local data such as cache, config, or tokens")]
    Cleanup(CleanupArgs),
    #[command(about = "Remove campus-lms local data and print npm uninstall guidance")]
    Uninstall(UninstallArgs),
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    #[command(
        about = "Store a Moodle Web Services token in the OS credential store",
        long_about = "Prompts for a Moodle base URL, username, and password, then requests a token from /login/token.php and stores the token in the OS credential store.\n\nThis does not bypass SSO or MFA. If password login is disabled by the university, ask whether Moodle Mobile Web Services or user token issuance is available.",
        after_help = "Examples:\n  campus-lms auth login\n  campus-lms auth login --base-url https://lms.example.edu/moodle/ --username student123\n  $env:MOODLE_PASSWORD = \"...\"; campus-lms auth login --json --base-url https://lms.example.edu/ --username student123 --password-env MOODLE_PASSWORD\n  Get-Content .\\password.txt | campus-lms auth login --json --base-url https://lms.example.edu/ --username student123 --password-stdin\n\nNotes:\n  Passwords are never saved.\n  The default Moodle service is moodle_mobile_app.\n  Login verifies that the token can be read back from the OS credential store before reporting success."
    )]
    Login(LoginArgs),
    #[command(
        name = "import-token",
        about = "Store an existing Moodle Web Services token",
        long_about = "Stores an administrator-issued or manually generated Moodle Web Services token in the OS credential store.\n\nUse this when password login is blocked by SSO/MFA but Moodle Web Services tokens are allowed by the university.",
        after_help = "Examples:\n  $env:MOODLE_TOKEN = \"...\"; campus-lms auth import-token --base-url https://lms.example.edu/moodle/ --username student123 --token-env MOODLE_TOKEN --json\n  Get-Content .\\token.txt | campus-lms auth import-token --base-url https://lms.example.edu/moodle/ --username student123 --token-stdin --json\n  campus-lms auth import-token --base-url https://lms.example.edu/moodle/ --username student123 --token-stdin --live --json"
    )]
    ImportToken(ImportTokenArgs),
    #[command(about = "Delete the stored token for the selected profile")]
    Logout(LogoutArgs),
    #[command(
        about = "Check whether a profile and token are available",
        after_help = "Examples:\n  campus-lms auth status\n  campus-lms auth status --json\n  campus-lms auth status --live --json"
    )]
    Status(AuthStatusArgs),
    #[command(
        about = "Verify profile, credential-store roundtrip, and optional Moodle API access",
        after_help = "Examples:\n  campus-lms auth verify --json\n  campus-lms auth verify --live --json\n\nThis command prints the credential target name without printing the token."
    )]
    Verify(AuthVerifyArgs),
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    #[arg(
        long,
        value_name = "URL",
        help = "Moodle base URL, for example https://lms.example.edu/moodle/"
    )]
    pub base_url: Option<String>,

    #[arg(long, value_name = "USER", help = "Moodle username")]
    pub username: Option<String>,

    #[arg(
        long,
        default_value = "moodle_mobile_app",
        help = "Moodle external service shortname"
    )]
    pub service: String,

    #[arg(
        long,
        help = "Allow http://localhost or http://127.0.0.1 for local Moodle development only"
    )]
    pub allow_insecure_localhost: bool,

    #[arg(
        long,
        help = "Read the Moodle password from standard input instead of prompting"
    )]
    pub password_stdin: bool,

    #[arg(
        long,
        value_name = "ENV",
        help = "Read the Moodle password from an environment variable"
    )]
    pub password_env: Option<String>,
}

#[derive(Debug, Args)]
pub struct ImportTokenArgs {
    #[arg(
        long,
        value_name = "URL",
        help = "Moodle base URL, for example https://lms.example.edu/moodle/"
    )]
    pub base_url: String,

    #[arg(long, value_name = "USER", help = "Moodle username for the token")]
    pub username: String,

    #[arg(
        long,
        default_value = "moodle_mobile_app",
        help = "Moodle external service shortname"
    )]
    pub service: String,

    #[arg(
        long,
        help = "Allow http://localhost or http://127.0.0.1 for local Moodle development only"
    )]
    pub allow_insecure_localhost: bool,

    #[arg(long, help = "Read the Moodle token from standard input")]
    pub token_stdin: bool,

    #[arg(
        long,
        value_name = "ENV",
        help = "Read the Moodle token from an environment variable"
    )]
    pub token_env: Option<String>,

    #[arg(long, help = "Also call Moodle to verify the imported token works")]
    pub live: bool,
}

#[derive(Debug, Args)]
pub struct LogoutArgs {
    #[arg(long, help = "Delete only the token and keep profile config")]
    pub keep_config: bool,
}

#[derive(Debug, Args)]
pub struct AuthStatusArgs {
    #[arg(long, help = "Also call Moodle to verify that the token still works")]
    pub live: bool,
}

#[derive(Debug, Args)]
pub struct AuthVerifyArgs {
    #[arg(
        long,
        help = "Also call Moodle to verify API access and token validity"
    )]
    pub live: bool,
}

#[derive(Debug, Args, Clone)]
pub struct CachedArgs {
    #[arg(long, help = "Ignore cache and fetch fresh data from Moodle")]
    pub refresh: bool,

    #[arg(long, help = "Use cached data only and do not contact Moodle")]
    pub offline: bool,
}

#[derive(Debug, Args, Clone)]
pub struct TodoArgs {
    #[arg(
        long,
        default_value_t = 14,
        value_name = "DAYS",
        help = "Number of days to look ahead, 1 to 365"
    )]
    pub days: u32,

    #[arg(long, value_name = "N", help = "Limit returned items, 1 to 500")]
    pub max_items: Option<usize>,

    #[arg(long, help = "Ignore cache and fetch fresh data from Moodle")]
    pub refresh: bool,

    #[arg(long, help = "Use cached data only and do not contact Moodle")]
    pub offline: bool,

    #[arg(
        long,
        help = "Keep tasks that Moodle marks as submitted or not actionable"
    )]
    pub include_submitted: bool,

    #[arg(
        long,
        value_name = "course:ID",
        help = "Return tasks only for one course; accepts course:123 or 123"
    )]
    pub course: Option<String>,

    #[arg(
        long,
        default_value_t = 20,
        value_name = "N",
        help = "Maximum fallback assignment submission-status checks, 0 to 500"
    )]
    pub status_check_limit: usize,

    #[arg(
        long,
        help = "Do not call per-assignment submission-status APIs for fallback tasks"
    )]
    pub no_submission_status_check: bool,
}

#[derive(Debug, Subcommand)]
pub enum AssignmentCommand {
    #[command(
        about = "Show one assignment as JSON",
        after_help = "Examples:\n  campus-lms assignment show assign:12345 --json\n  campus-lms assignment show 12345 --json --max-chars 12000\n\nThis command reads details and submission status, but does not submit or mark anything as viewed."
    )]
    Show(AssignmentShowArgs),
}

#[derive(Debug, Args, Clone)]
pub struct AssignmentShowArgs {
    #[arg(help = "Assignment id such as assign:12345, or the numeric id 12345")]
    pub id: String,

    #[arg(
        long,
        default_value_t = 8000,
        value_name = "CHARS",
        help = "Maximum description text length, up to 100000"
    )]
    pub max_chars: usize,

    #[arg(long, help = "Include original HTML description in JSON output")]
    pub include_html: bool,

    #[arg(long, help = "Ignore cache and fetch fresh data from Moodle")]
    pub refresh: bool,

    #[arg(long, help = "Use cached data only and do not contact Moodle")]
    pub offline: bool,

    #[arg(long, help = "Do not read or write the assignment detail cache")]
    pub no_cache: bool,
}

#[derive(Debug, Subcommand)]
pub enum AiCommand {
    #[command(
        about = "Print a compact JSON snapshot for AI or scripts",
        after_help = "Recommended first command for AI tools:\n  campus-lms ai snapshot --days 14 --json\n\nThe snapshot excludes grades, feedback, email, tokens, passwords, cookies, and sessions."
    )]
    Snapshot(AiSnapshotArgs),
    #[command(about = "Print short operating instructions for AI agents")]
    Instructions,
}

#[derive(Debug, Args, Clone)]
pub struct AiSnapshotArgs {
    #[arg(
        long,
        default_value_t = 14,
        value_name = "DAYS",
        help = "Number of days to look ahead, 1 to 365"
    )]
    pub days: u32,

    #[arg(
        long,
        default_value_t = 30,
        value_name = "N",
        help = "Limit returned tasks, 1 to 500"
    )]
    pub max_items: usize,

    #[arg(long, help = "Reserved flag; grades are not fetched in this version")]
    pub include_grades: bool,

    #[arg(long, help = "Reserved flag; feedback is not fetched in this version")]
    pub include_feedback: bool,

    #[arg(long, help = "Ignore cache and fetch fresh data from Moodle")]
    pub refresh: bool,

    #[arg(long, help = "Use cached data only and do not contact Moodle")]
    pub offline: bool,
}

#[derive(Debug, Subcommand)]
pub enum SchemaCommand {
    #[command(about = "List available output schemas")]
    List,
    #[command(about = "Print one output schema")]
    Show {
        #[arg(help = "Schema name, for example todo.v1 or campus-lms.todo.v1")]
        name: String,
    },
}

#[derive(Debug, Args, Clone)]
pub struct InitArgs {
    #[arg(
        long,
        help = "Recreate missing directories, but never overwrite existing config."
    )]
    pub force: bool,

    #[arg(long, help = "Print a short AGENTS.md snippet for AI agent workspaces")]
    pub print_agents: bool,
}

#[derive(Debug, Args, Clone)]
pub struct CleanupArgs {
    #[arg(long, help = "Remove config, cache, and tokens")]
    pub all: bool,

    #[arg(long = "local-config", help = "Remove local config.toml")]
    pub local_config: bool,

    #[arg(long, help = "Remove cached Moodle data")]
    pub cache: bool,

    #[arg(long, help = "Remove stored Moodle tokens")]
    pub tokens: bool,

    #[arg(long, help = "Confirm deletion without prompting")]
    pub yes: bool,

    #[arg(long, help = "Show what would be removed without deleting anything")]
    pub dry_run: bool,
}

#[derive(Debug, Args, Clone)]
pub struct UninstallArgs {
    #[arg(long, help = "Confirm deletion without prompting")]
    pub yes: bool,

    #[arg(long, help = "Show what would be removed without deleting anything")]
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

pub fn ensure_status_check_limit(limit: usize) -> crate::error::Result<()> {
    if limit <= 500 {
        Ok(())
    } else {
        Err(crate::error::CampusError::invalid_argument(
            "--status-check-limit must be 500 or less.",
            None,
        ))
    }
}

pub fn normalize_course_filter(input: &str) -> crate::error::Result<(String, i64)> {
    let raw = input.strip_prefix("course:").unwrap_or(input);
    let id = raw.parse::<i64>().map_err(|_| {
        crate::error::CampusError::invalid_argument(
            "--course must look like course:123 or 123.",
            Some("Example: campus-lms todo --course course:123 --json"),
        )
    })?;
    if id <= 0 {
        return Err(crate::error::CampusError::invalid_argument(
            "--course id must be a positive integer.",
            Some("Example: campus-lms todo --course course:123 --json"),
        ));
    }
    Ok((format!("course:{id}"), id))
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

pub fn warning_detail_limit(cli: &Cli) -> crate::error::Result<Option<usize>> {
    match cli.warning_details.as_deref() {
        Some("all") => Ok(None),
        Some(value) => {
            let limit = value.parse::<usize>().map_err(|_| {
                crate::error::CampusError::invalid_argument(
                    "--warning-details must be 0, a positive integer, or all.",
                    Some("Use --warning-details 20 for debugging, or --verbose for all details."),
                )
            })?;
            Ok(Some(limit))
        }
        None if cli.verbose => Ok(None),
        None => Ok(Some(0)),
    }
}
