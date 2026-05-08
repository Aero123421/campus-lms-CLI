// SPDX-License-Identifier: Apache-2.0

mod ai;
mod auth;
mod cache;
mod cli;
mod config;
mod docs;
mod doctor;
mod dto;
mod error;
mod keychain;
mod lifecycle;
mod moodle;
mod output;
mod schema;

use clap::Parser;
use cli::{AssignmentCommand, AuthCommand, Cli, Commands};
use dto::{UserInfo, WhoamiOutput};
use error::CampusError;

fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    if let Err(err) = run(cli) {
        let wants_json = json || err.json_requested();
        let err = err.with_json(wants_json);
        let code = err.exit_code();
        if err.json_requested() {
            let _ = output::print_error(&err);
        } else {
            eprintln!("error: {}", err);
            if let Some(hint) = err.hint() {
                eprintln!("hint: {}", hint);
            }
        }
        std::process::exit(code);
    }
}

fn run(cli: Cli) -> Result<(), CampusError> {
    let json = cli.json;
    if let Some(profile) = &cli.profile {
        cli::ensure_profile_name(profile).map_err(|err| err.with_json(json))?;
    }
    match &cli.command {
        Commands::Auth { command } => match command {
            AuthCommand::Login(args) => auth::login(&cli, args),
            AuthCommand::ImportToken(args) => auth::import_token(&cli, args),
            AuthCommand::Logout(args) => auth::logout(&cli, args),
            AuthCommand::Status(args) => auth::status(&cli, args),
            AuthCommand::Verify(args) => auth::verify(&cli, args),
        },
        Commands::Whoami => {
            let client = moodle::client_from_profile(&cli)?;
            let site = client.site_info()?;
            output::print_json(&WhoamiOutput {
                schema_version: "campus-lms.whoami.v1",
                generated_at: output::generated_at(),
                user: UserInfo {
                    id: format!("user:{}", site.userid),
                    username: site.username,
                    fullname: site.fullname,
                    site_name: site.sitename,
                },
                warnings: Vec::new(),
            })
        }
        Commands::Doctor => doctor::run(&cli),
        Commands::Courses(args) => moodle::courses::run(&cli, args),
        Commands::Todo(args) => moodle::calendar::todo(&cli, args),
        Commands::Assignment { command } => match command {
            AssignmentCommand::Show(args) => moodle::assignments::show(&cli, args),
        },
        Commands::Ai { command } => ai::run(&cli, command),
        Commands::Capabilities => docs::capabilities::print(),
        Commands::Errors => docs::errors::print(),
        Commands::Schema { command } => schema::run(command),
        Commands::Init(args) => lifecycle::init(&cli, args),
        Commands::Cleanup(args) => lifecycle::cleanup(&cli, args),
        Commands::Uninstall(args) => lifecycle::uninstall(&cli, args),
    }
}
