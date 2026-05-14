//! `code-ratchet watch` — real-time L0 feedback while you (or your LLM) edits.
//!
//! Watches the repo root, debounces, runs just the cheapest layer (L0 lint)
//! on each batch, and writes the resulting feedback to `.ratchet/feedback.md`.
//! Does not advance the baseline — this is informational. The Stop-/commit-time
//! gate is still the authoritative ratchet.

use std::path::Path;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::checks::run_layer;
use crate::cmd::{config_path, ratchet_dir};
use crate::config::Config;
use crate::feedback::{now_iso, Feedback, LayerReport, Verdict};
use crate::metrics::{Layer, Metrics};

const DEBOUNCE: Duration = Duration::from_millis(400);
const IGNORE_DIRS: &[&str] = &[".git", ".ratchet", "target", "node_modules", ".venv", "__pycache__", "dist", "build"];

pub fn run(repo_root: &Path) -> Result<()> {
    let cfg_path = config_path(repo_root);
    if !cfg_path.exists() {
        bail!("{} not found. Run `code-ratchet setup` first.", cfg_path.display());
    }

    println!("code-ratchet watch — {}", repo_root.display());
    println!("Re-running L0 (lint) on every save. Feedback at .ratchet/feedback.md.");
    println!("Press Ctrl+C to stop.");
    println!();

    // Initial run so feedback.md exists immediately.
    if let Err(e) = run_once(repo_root) { eprintln!("warn: initial check failed: {e}"); }

    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(repo_root, RecursiveMode::Recursive)
        .with_context(|| format!("watch {}", repo_root.display()))?;

    let mut last_run = Instant::now();
    loop {
        let event = match rx.recv() {
            Ok(Ok(e)) => e,
            Ok(Err(e)) => { eprintln!("watch error: {e}"); continue; }
            Err(_) => break, // sender dropped
        };
        if !is_relevant(&event, repo_root) { continue; }
        if last_run.elapsed() < DEBOUNCE { continue; }
        std::thread::sleep(DEBOUNCE);
        while rx.try_recv().is_ok() {}
        last_run = Instant::now();

        match run_once(repo_root) {
            Ok((warnings, exit)) => {
                let stamp = chrono::Local::now().format("%H:%M:%S");
                println!("[{}] L0 → {} warning(s), exit {}", stamp, warnings, exit);
            }
            Err(e) => eprintln!("watch run failed: {e}"),
        }
    }
    Ok(())
}

fn is_relevant(event: &Event, repo_root: &Path) -> bool {
    // Only files-changed-on-disk events matter.
    matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_))
        && event.paths.iter().any(|p| {
            let Ok(rel) = p.strip_prefix(repo_root) else { return false; };
            !rel.components().any(|c| {
                let name = c.as_os_str().to_string_lossy();
                IGNORE_DIRS.iter().any(|i| name == *i)
            })
        })
}

fn run_once(repo_root: &Path) -> Result<(u32, i32)> {
    let cfg = Config::load(&config_path(repo_root))?;

    let outcome = run_layer(Layer::L0, &cfg.l0, repo_root)?;
    let mut current = Metrics::default();
    current.merge_check(&outcome);

    let layer_report = LayerReport::from_outcome(&outcome);
    let warnings = outcome.numeric_signal;
    let exit = outcome.exit_code;

    let verdict = if !outcome.executed { Verdict::LayerFailed } else { Verdict::Pass };

    let feedback = Feedback {
        verdict,
        generated_at: now_iso(),
        baseline_metrics: Metrics::default(),
        current_metrics: current,
        regressions: Vec::new(),
        layer_reports: vec![layer_report],
        improved: Vec::new(),
        suggestions: if warnings > 0 {
            vec![format!("L0 reports {} lint warning(s). Run `code-ratchet check` for full gate evaluation.", warnings)]
        } else { Vec::new() },
    };
    feedback.write(&ratchet_dir(repo_root))?;
    Ok((warnings, exit))
}
