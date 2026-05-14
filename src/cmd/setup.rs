//! `code-ratchet setup` — the one-command UX.
//!
//! Minimalist: detects the project language, writes 4 things, asks once.
//! No IDE adapters — a single `AGENTS.md` covers any LLM/IDE that reads
//! repo-root rules files.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;

use crate::baseline::Baseline;
use crate::cmd::detect::{detect_language, DetectedLanguage};
use crate::cmd::{config_path, ratchet_dir};
use crate::config::Config;

const AGENTS_MD:        &str = include_str!("../../templates/AGENTS.md");

pub const OWNERSHIP_MARKER: &str = "Managed by `code-ratchet setup`";

pub struct SetupOpts {
    pub yes: bool,
    pub force: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy)]
enum Action {
    WriteConfig(DetectedLanguage),
    WriteAgentsMd,
    RunBaselineCheck,
    InstallGitHook,
}

impl Action {
    fn label(self) -> String {
        match self {
            Action::WriteConfig(lang) => format!("Write .ratchet.yml ({} defaults)", lang.label()),
            Action::WriteAgentsMd     => "Write AGENTS.md (universal LLM rules)".into(),
            Action::RunBaselineCheck  => "Seed baseline (runs L0/L1/L2 once)".into(),
            Action::InstallGitHook    => "Install git pre-commit hook".into(),
        }
    }
}

pub fn run(repo_root: &Path, opts: SetupOpts) -> Result<()> {
    let lang = detect_language(repo_root);
    let actions = plan(repo_root, lang, opts.force);

    println!("code-ratchet setup");
    println!("==================");
    println!("Detected language : {}", lang.label());
    println!();
    println!("Planned actions ({}):", actions.len());
    for a in &actions { println!("  • {}", a.label()); }

    if opts.dry_run {
        println!("\n(dry run — no changes made)");
        return Ok(());
    }

    if !opts.yes {
        println!();
        if !confirm("Proceed? [Y/n] ")? {
            println!("aborted.");
            return Ok(());
        }
    }
    println!();

    for action in actions {
        match execute(repo_root, action, opts.force) {
            Ok(msg) => println!("  ✓ {}", msg),
            Err(e)  => println!("  ✗ {}: {}", action.label(), e),
        }
    }

    println!();
    println!("✓ Setup complete. Ratchet engaged.");
    println!();
    println!("  Check    : code-ratchet check");
    println!("  Watch    : code-ratchet watch    (real-time L0 feedback)");
    println!("  Status   : code-ratchet status");
    println!("  Uninstall: code-ratchet uninstall");
    Ok(())
}

fn plan(repo_root: &Path, lang: DetectedLanguage, force: bool) -> Vec<Action> {
    let mut out: Vec<Action> = Vec::new();
    if !config_path(repo_root).exists() || force {
        out.push(Action::WriteConfig(lang));
    }
    if !repo_root.join("AGENTS.md").exists() || force {
        out.push(Action::WriteAgentsMd);
    }
    out.push(Action::RunBaselineCheck);
    if repo_root.join(".git").is_dir() {
        out.push(Action::InstallGitHook);
    }
    out
}

fn execute(repo_root: &Path, action: Action, force: bool) -> Result<String> {
    match action {
        Action::WriteConfig(lang) => {
            let path = config_path(repo_root);
            if path.exists() && !force {
                return Ok(".ratchet.yml exists; leaving alone (use --force to overwrite)".into());
            }
            Config::write_for_language(&path, lang)?;
            Ok(format!("Wrote {}", path.display()))
        }
        Action::WriteAgentsMd => {
            let path = repo_root.join("AGENTS.md");
            if path.exists() && !force {
                let existing = fs::read_to_string(&path).unwrap_or_default();
                if existing.contains(OWNERSHIP_MARKER) {
                    return Ok("AGENTS.md already managed by code-ratchet; leaving alone".into());
                }
                return Ok("AGENTS.md exists and is not ours; leaving alone (use --force to overwrite)".into());
            }
            fs::write(&path, AGENTS_MD)?;
            Ok(format!("Wrote {}", path.display()))
        }
        Action::RunBaselineCheck => {
            if !config_path(repo_root).exists() {
                return Ok("Skipped (no .ratchet.yml present)".into());
            }
            let exit = crate::cmd::check::run(
                repo_root,
                crate::cmd::check::CheckOpts { no_ratchet: false, json: false, quiet: true },
            )?;
            let baseline = Baseline::load_or_default(&ratchet_dir(repo_root))?;
            match exit {
                0 => Ok(format!(
                    "Baseline at ratchet_count={} (tests={}, cov={:.2}%)",
                    baseline.ratchet_count,
                    baseline.metrics.test_count,
                    baseline.metrics.coverage_percent
                )),
                1 => Ok("Baseline check ran; regression vs existing baseline (see .ratchet/feedback.md)".into()),
                _ => Ok("Baseline check ran; a layer failed to execute (see .ratchet/feedback.md)".into()),
            }
        }
        Action::InstallGitHook => {
            if !repo_root.join(".git").is_dir() {
                return Ok("Skipped (not a git repo)".into());
            }
            crate::cmd::install_hook::run(repo_root, force)?;
            Ok("Installed .git/hooks/pre-commit".into())
        }
    }
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{}", prompt);
    io::stdout().flush().ok();
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    let answer = s.trim().to_lowercase();
    Ok(answer.is_empty() || answer == "y" || answer == "yes")
}
