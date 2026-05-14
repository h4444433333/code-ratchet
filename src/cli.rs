use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "code-ratchet",
    version,
    about = "A complexity ratchet for AI-assisted code. Quality only goes up, never down.",
    long_about = None
)]
pub struct Cli {
    /// Path to the project root (default: current dir).
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// One-command setup: detect language, write .ratchet.yml + AGENTS.md,
    /// seed baseline, install git pre-commit hook. The recommended entry
    /// point for new users.
    Setup {
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Overwrite existing files.
        #[arg(long)]
        force: bool,
        /// Print the plan but do not execute.
        #[arg(long)]
        dry_run: bool,
    },
    /// Reverse `setup`: remove files we own. Hand-edited AGENTS.md and
    /// third-party pre-commit hooks are left alone.
    Uninstall {
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Keep `.ratchet/` (baseline + feedback) on disk.
        #[arg(long)]
        keep_baseline: bool,
    },
    /// Initialize `.ratchet/` with a default config and an empty baseline.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Run L0/L1/L2 checks, compare against baseline, write feedback,
    /// and (on pass) advance the baseline. Exit non-zero on regression.
    Check {
        #[arg(long)]
        no_ratchet: bool,
        #[arg(long)]
        json: bool,
    },
    /// Watch the repo for file changes and re-run L0 (lint) on each save,
    /// keeping `.ratchet/feedback.md` fresh in real time. Does NOT advance
    /// the baseline — that still happens at `check`/commit time.
    Watch,
    /// Print the current baseline metrics.
    Status,
    /// Install a git pre-commit hook that runs `code-ratchet check`.
    InstallHook {
        #[arg(long)]
        force: bool,
    },
}
