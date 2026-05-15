//! `code-ratchet watch` — real-time L0 feedback while you (or your LLM) edits.
//!
//! Watches the repo root, debounces, runs just the cheapest layer (L0 lint)
//! on each batch, and writes the resulting feedback to `.ratchet/feedback.md`.
//! Does not advance the baseline — this is informational. The Stop-/commit-time
//! gate is still the authoritative ratchet.

use std::path::Path;
use std::fs::{self, File};
use std::process::{Command, Stdio};
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
const WATCH_PID_FILE: &str = "watch.pid";
const WATCH_LOG_FILE: &str = "watch.log";

pub enum WatchBackgroundStatus {
    Started(u32),
    AlreadyRunning(u32),
}

struct WatchPidGuard {
    path: std::path::PathBuf,
    pid: u32,
}

impl Drop for WatchPidGuard {
    fn drop(&mut self) {
        clear_watch_pid_path(&self.path, Some(self.pid)).ok();
    }
}

pub fn run(repo_root: &Path) -> Result<()> {
    let cfg_path = config_path(repo_root);
    if !cfg_path.exists() {
        bail!("{} not found. Run `code-ratchet setup` first.", cfg_path.display());
    }
    if !claim_watch_pid(repo_root)? {
        println!("code-ratchet watch already running for {}", repo_root.display());
        return Ok(());
    }
    let _guard = WatchPidGuard { path: watch_pid_path(repo_root), pid: std::process::id() };

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

pub fn start_background(repo_root: &Path) -> Result<WatchBackgroundStatus> {
    if let Some(pid) = active_watch_pid(repo_root) {
        return Ok(WatchBackgroundStatus::AlreadyRunning(pid));
    }
    clear_watch_pid(repo_root)?;
    fs::create_dir_all(ratchet_dir(repo_root))?;
    let log_path = watch_log_path(repo_root);
    let log = File::options().create(true).append(true).open(&log_path)
        .with_context(|| format!("open {}", log_path.display()))?;
    let err = log.try_clone().with_context(|| format!("clone {}", log_path.display()))?;
    let child = Command::new(std::env::current_exe()?)
        .arg("--repo")
        .arg(repo_root)
        .arg("watch")
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err))
        .spawn()
        .with_context(|| format!("spawn background watch for {}", repo_root.display()))?;
    let pid = child.id();
    write_watch_pid(repo_root, pid)?;
    Ok(WatchBackgroundStatus::Started(pid))
}

pub fn stop_background(repo_root: &Path) -> Result<Option<u32>> {
    let Some(pid) = read_watch_pid(repo_root)? else { return Ok(None); };
    if !is_process_running(pid) {
        clear_watch_pid(repo_root)?;
        return Ok(None);
    }
    let status = if cfg!(windows) {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("kill -TERM {}", pid))
            .status()
    }.with_context(|| format!("stop background watch pid {}", pid))?;
    if !status.success() {
        bail!("failed to stop background watch pid {}", pid);
    }
    clear_watch_pid(repo_root)?;
    Ok(Some(pid))
}

pub fn active_watch_pid(repo_root: &Path) -> Option<u32> {
    let Ok(Some(pid)) = read_watch_pid(repo_root) else { return None; };
    if is_process_running(pid) {
        Some(pid)
    } else {
        clear_watch_pid(repo_root).ok();
        None
    }
}

pub fn watch_log_path(repo_root: &Path) -> std::path::PathBuf {
    ratchet_dir(repo_root).join(WATCH_LOG_FILE)
}

fn watch_pid_path(repo_root: &Path) -> std::path::PathBuf {
    ratchet_dir(repo_root).join(WATCH_PID_FILE)
}

fn claim_watch_pid(repo_root: &Path) -> Result<bool> {
    let pid = std::process::id();
    if let Some(active_pid) = read_watch_pid(repo_root)? {
        if active_pid != pid && is_process_running(active_pid) {
            return Ok(false);
        }
    }
    write_watch_pid(repo_root, pid)?;
    Ok(true)
}

fn read_watch_pid(repo_root: &Path) -> Result<Option<u32>> {
    let path = watch_pid_path(repo_root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    match raw.trim().parse::<u32>() {
        Ok(pid) => Ok(Some(pid)),
        Err(_) => {
            clear_watch_pid_path(&path, None)?;
            Ok(None)
        }
    }
}

fn write_watch_pid(repo_root: &Path, pid: u32) -> Result<()> {
    fs::create_dir_all(ratchet_dir(repo_root))?;
    let path = watch_pid_path(repo_root);
    fs::write(&path, format!("{pid}\n")).with_context(|| format!("write {}", path.display()))
}

fn clear_watch_pid(repo_root: &Path) -> Result<()> {
    clear_watch_pid_path(&watch_pid_path(repo_root), None)
}

fn clear_watch_pid_path(path: &Path, expected_pid: Option<u32>) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if let Some(expected) = expected_pid {
        let current = fs::read_to_string(path).unwrap_or_default();
        if current.trim() != expected.to_string() {
            return Ok(());
        }
    }
    fs::remove_file(path).or_else(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(err)
        }
    }).with_context(|| format!("remove {}", path.display()))
}

fn is_process_running(pid: u32) -> bool {
    let status = if cfg!(windows) {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .ok()
            .map(|out| String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()))
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("kill -0 {}", pid))
            .status()
            .ok()
            .map(|s| s.success())
    };
    status.unwrap_or(false)
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
