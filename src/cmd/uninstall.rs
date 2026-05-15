//! `code-ratchet uninstall` — reverse `setup`.
//!
//! Only removes files that we wrote (detected via ownership marker for
//! AGENTS.md and pre-commit; `.ratchet/` and `.ratchet.yml` are always
//! ours by name).

use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::cmd::setup::OWNERSHIP_MARKER;
use crate::cmd::{config_path, ratchet_dir};
use crate::cmd::watch;

pub struct UninstallOpts {
    pub yes: bool,
    pub keep_baseline: bool,
}

pub fn run(repo_root: &Path, opts: UninstallOpts) -> Result<()> {
    println!("code-ratchet uninstall");
    println!("======================");
    let plan = plan_removals(repo_root, opts.keep_baseline);
    if plan.is_empty() {
        println!("Nothing managed by code-ratchet found at {}", repo_root.display());
        return Ok(());
    }
    println!("Will remove:");
    for r in &plan { println!("  - {}", r); }
    println!();
    if !opts.yes && !confirm("Proceed? [y/N] ")? {
        println!("aborted.");
        return Ok(());
    }
    for r in plan {
        match remove(repo_root, &r) {
            Ok(msg) => println!("  ✓ {}", msg),
            Err(e)  => println!("  ✗ {}: {}", r, e),
        }
    }
    println!();
    println!("✓ Uninstall complete.");
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum Removal {
    BackgroundWatch,
    RatchetDir,
    RatchetYml,
    AgentsMd,
    PreCommitHook,
}

impl std::fmt::Display for Removal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Removal::BackgroundWatch => "background watch process (if running)",
            Removal::RatchetDir    => ".ratchet/ (baseline + feedback)",
            Removal::RatchetYml    => ".ratchet.yml",
            Removal::AgentsMd      => "AGENTS.md (if ours)",
            Removal::PreCommitHook => ".git/hooks/pre-commit (if ours)",
        })
    }
}

fn plan_removals(repo_root: &Path, keep_baseline: bool) -> Vec<Removal> {
    let mut out: Vec<Removal> = Vec::new();
    if watch::active_watch_pid(repo_root).is_some() { out.push(Removal::BackgroundWatch); }
    if !keep_baseline && ratchet_dir(repo_root).is_dir() { out.push(Removal::RatchetDir); }
    if config_path(repo_root).exists() { out.push(Removal::RatchetYml); }
    if agents_md_is_ours(repo_root)    { out.push(Removal::AgentsMd); }
    if pre_commit_is_ours(repo_root)   { out.push(Removal::PreCommitHook); }
    out
}

fn remove(repo_root: &Path, r: &Removal) -> Result<String> {
    match r {
        Removal::BackgroundWatch => {
            match watch::stop_background(repo_root)? {
                Some(pid) => Ok(format!("stopped background watch pid {}", pid)),
                None => Ok("background watch was not running".into()),
            }
        }
        Removal::RatchetDir => {
            let p = ratchet_dir(repo_root);
            fs::remove_dir_all(&p)?;
            Ok(format!("removed {}", p.display()))
        }
        Removal::RatchetYml => {
            let p = config_path(repo_root);
            fs::remove_file(&p)?;
            Ok(format!("removed {}", p.display()))
        }
        Removal::AgentsMd => {
            let p = repo_root.join("AGENTS.md");
            fs::remove_file(&p)?;
            Ok(format!("removed {}", p.display()))
        }
        Removal::PreCommitHook => {
            let p = repo_root.join(".git/hooks/pre-commit");
            fs::remove_file(&p)?;
            Ok(format!("removed {}", p.display()))
        }
    }
}

fn agents_md_is_ours(repo_root: &Path) -> bool {
    let p = repo_root.join("AGENTS.md");
    let Ok(text) = fs::read_to_string(&p) else { return false; };
    text.contains(OWNERSHIP_MARKER)
}

fn pre_commit_is_ours(repo_root: &Path) -> bool {
    let p = repo_root.join(".git/hooks/pre-commit");
    let Ok(text) = fs::read_to_string(&p) else { return false; };
    text.lines().take(5).any(|l| l.contains("code-ratchet"))
}

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::{self, Write};
    print!("{}", prompt);
    io::stdout().flush().ok();
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    let a = s.trim().to_lowercase();
    Ok(a == "y" || a == "yes")
}
