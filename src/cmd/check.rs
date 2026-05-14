use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::baseline::{compare, Baseline, RatchetVerdict};
use crate::checks::run_layer;
use crate::cmd::{config_path, ratchet_dir};
use crate::config::Config;
use crate::feedback::{build_suggestions, now_iso, Feedback, LayerReport, Verdict};
use crate::metrics::{Layer, Metrics};

pub struct CheckOpts {
    pub no_ratchet: bool,
    pub json: bool,
    pub quiet: bool,
}

pub fn run(repo_root: &Path, opts: CheckOpts) -> Result<i32> {
    let cfg_path = config_path(repo_root);
    if !cfg_path.exists() {
        bail!("{} not found. Run `code-ratchet init` first.", cfg_path.display());
    }
    let cfg = Config::load(&cfg_path).with_context(|| "loading .ratchet.yml")?;
    let dir = ratchet_dir(repo_root);
    let mut baseline = Baseline::load_or_default(&dir)?;
    let is_first_run = baseline.ratchet_count == 0
        && baseline.metrics == Metrics::default();

    let mut current = Metrics::default();
    let mut layer_reports: Vec<LayerReport> = Vec::new();
    let mut hard_failed_layer: Option<&'static str> = None;

    for (layer, lc) in [(Layer::L0, &cfg.l0), (Layer::L1, &cfg.l1), (Layer::L2, &cfg.l2)] {
        let outcome = run_layer(layer, lc, repo_root)?;
        // "Tool crashed" = exit code we couldn't read OR a non-numeric signal we can't trust.
        // For L0/L1 a non-zero exit with parseable warnings/errors is fine — that's the count.
        // Only escalate to "layer failed" when the tool itself was missing or signaled.
        if lc.enabled() && !outcome.executed && lc.required {
            hard_failed_layer = Some(layer.label());
        }
        current.merge_check(&outcome);
        layer_reports.push(LayerReport::from_outcome(&outcome));
        // Stop the chain on hard failure to save time, but keep the report.
        if hard_failed_layer.is_some() { break; }
    }

    let (verdict, regressions, improved): (Verdict, Vec<_>, Vec<_>) = if let Some(_label) = hard_failed_layer {
        (Verdict::LayerFailed, Vec::new(), Vec::new())
    } else if is_first_run {
        (Verdict::FirstRun, Vec::new(), Vec::new())
    } else {
        match compare(&baseline.metrics, &current) {
            RatchetVerdict::Pass { improved } => (Verdict::Pass, Vec::new(), improved),
            RatchetVerdict::Regression(rs) => (Verdict::RegressionBlocked, rs, Vec::new()),
        }
    };

    let suggestions = build_suggestions(&regressions, &current, &baseline);
    let feedback = Feedback {
        verdict: verdict.clone(),
        generated_at: now_iso(),
        baseline_metrics: baseline.metrics.clone(),
        current_metrics: current.clone(),
        regressions: regressions.clone(),
        layer_reports: layer_reports.clone(),
        improved: improved.clone(),
        suggestions,
    };
    let (json_path, md_path) = feedback.write(&dir)?;

    // Advance baseline if and only if we're going to return success.
    let exit_code = match &verdict {
        Verdict::Pass | Verdict::FirstRun => {
            if !opts.no_ratchet {
                baseline.ratchet_up(&current);
                baseline.save(&dir)?;
            }
            0
        }
        Verdict::RegressionBlocked => 1,
        Verdict::LayerFailed => 2,
    };

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&feedback)?);
    } else if !opts.quiet {
        print_human_summary(&feedback);
        eprintln!("feedback: {} | {}", md_path.display(), json_path.display());
    }
    Ok(exit_code)
}

fn print_human_summary(fb: &Feedback) {
    let banner = match fb.verdict {
        Verdict::Pass => "PASS — ratchet advanced",
        Verdict::FirstRun => "FIRST RUN — baseline seeded",
        Verdict::RegressionBlocked => "BLOCKED — quality regression",
        Verdict::LayerFailed => "BLOCKED — check layer failed",
    };
    println!("{}", banner);
    println!();
    for lr in &fb.layer_reports {
        let exec = if lr.executed { "ran" } else { "skipped" };
        let mut line = format!("  {}  [{}]  signal={}", lr.layer, exec, lr.numeric_signal);
        if let Some(c) = lr.coverage_percent { line.push_str(&format!("  cov={:.2}%", c)); }
        if let Some(p) = lr.passing { line.push_str(&format!("  passing={}", p)); }
        if let Some(n) = &lr.note { line.push_str(&format!("  ({})", n)); }
        println!("{}", line);
    }
    if !fb.regressions.is_empty() {
        println!();
        println!("Regressions:");
        for r in &fb.regressions {
            println!("  {} ({}): {} → {} (delta {})", r.metric, r.direction, r.baseline, r.current, r.delta);
        }
    }
    if !fb.improved.is_empty() {
        println!();
        println!("Improved: {}", fb.improved.join(", "));
    }
}
