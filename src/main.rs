use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use code_ratchet::cli::{Cli, Command};
use code_ratchet::cmd;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let repo_root = match cli.repo {
        Some(p) => p,
        None => env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    match run(cli.command, &repo_root) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("error: {e:?}");
            ExitCode::from(3)
        }
    }
}

fn run(command: Command, repo_root: &std::path::Path) -> Result<i32> {
    match command {
        Command::Setup { yes, force, dry_run } => {
            cmd::setup::run(repo_root, cmd::setup::SetupOpts { yes, force, dry_run })?;
            Ok(0)
        }
        Command::Uninstall { yes, keep_baseline } => {
            cmd::uninstall::run(repo_root, cmd::uninstall::UninstallOpts { yes, keep_baseline })?;
            Ok(0)
        }
        Command::Init { force }      => { cmd::init::run(repo_root, force)?; Ok(0) }
        Command::Status              => { cmd::status::run(repo_root)?; Ok(0) }
        Command::Watch               => { cmd::watch::run(repo_root)?; Ok(0) }
        Command::InstallHook { force } => { cmd::install_hook::run(repo_root, force)?; Ok(0) }
        Command::Check { no_ratchet, json } => {
            cmd::check::run(repo_root, cmd::check::CheckOpts { no_ratchet, json, quiet: false })
        }
    }
}
