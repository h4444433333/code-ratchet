use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;

use crate::baseline::{Baseline, Regression};
use crate::metrics::{CheckOutcome, Layer, Metrics};

const FEEDBACK_JSON: &str = "feedback.json";
const FEEDBACK_MD: &str = "feedback.md";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict { Pass, RegressionBlocked, FirstRun, LayerFailed }

#[derive(Debug, Clone, Serialize)]
pub struct LayerReport {
    pub layer: &'static str,
    pub command: String,
    pub exit_code: i32,
    pub numeric_signal: u32,
    pub passing: Option<u32>,
    pub coverage_percent: Option<f64>,
    pub executed: bool,
    pub note: Option<String>,
}

impl LayerReport {
    pub fn from_outcome(o: &CheckOutcome) -> Self {
        Self {
            layer: match o.layer { Layer::L0 => "L0", Layer::L1 => "L1", Layer::L2 => "L2" },
            command: o.command.clone(),
            exit_code: o.exit_code,
            numeric_signal: o.numeric_signal,
            passing: o.passing,
            coverage_percent: o.coverage,
            executed: o.executed,
            note: o.note.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Feedback {
    pub verdict: Verdict,
    pub generated_at: String,
    pub baseline_metrics: Metrics,
    pub current_metrics: Metrics,
    pub regressions: Vec<Regression>,
    pub layer_reports: Vec<LayerReport>,
    pub improved: Vec<&'static str>,
    pub suggestions: Vec<String>,
}

impl Feedback {
    pub fn write(&self, dir: &Path) -> Result<(PathBuf, PathBuf)> {
        fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
        let json_path = dir.join(FEEDBACK_JSON);
        let md_path = dir.join(FEEDBACK_MD);
        fs::write(&json_path, serde_json::to_string_pretty(self)? + "\n")?;
        fs::write(&md_path, self.to_markdown())?;
        Ok((json_path, md_path))
    }

    /// Markdown is the canonical surface the LLM reads. Keep it terse and
    /// structured so it survives prompt injection into a Claude turn.
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let title = match self.verdict {
            Verdict::Pass => "Pass — ratchet advanced.",
            Verdict::RegressionBlocked => "Blocked — quality regression detected.",
            Verdict::FirstRun => "First run — baseline established.",
            Verdict::LayerFailed => "Blocked — a required check layer failed to execute.",
        };
        s.push_str(&format!("# code-ratchet feedback\n\n**Verdict:** {}\n\n", title));
        s.push_str(&format!("_Generated: {}_\n\n", self.generated_at));

        if !self.regressions.is_empty() {
            s.push_str("## Regressions (these block the commit)\n\n");
            s.push_str("| Metric | Direction | Baseline | Current | Delta |\n");
            s.push_str("|---|---|---|---|---|\n");
            for r in &self.regressions {
                s.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} |\n",
                    r.metric, r.direction, r.baseline, r.current, r.delta
                ));
            }
            s.push('\n');
        }

        s.push_str("## Layer results\n\n");
        for lr in &self.layer_reports {
            let exec = if lr.executed { "ran" } else { "skipped" };
            let mut line = format!("- **{}** [{}] `{}` → exit {}, signal {}",
                lr.layer, exec, abbreviate(&lr.command, 80), lr.exit_code, lr.numeric_signal);
            if let Some(p) = lr.passing { line.push_str(&format!(", passing {}", p)); }
            if let Some(c) = lr.coverage_percent { line.push_str(&format!(", coverage {:.2}%", c)); }
            if let Some(n) = &lr.note { line.push_str(&format!(" ({})", n)); }
            line.push('\n');
            s.push_str(&line);
        }
        s.push('\n');

        if !self.improved.is_empty() {
            s.push_str("## Improved this run\n\n");
            for i in &self.improved { s.push_str(&format!("- `{}`\n", i)); }
            s.push('\n');
        }

        if !self.suggestions.is_empty() {
            s.push_str("## Suggestions for the next agent turn\n\n");
            for sug in &self.suggestions { s.push_str(&format!("- {}\n", sug)); }
            s.push('\n');
        }

        s.push_str("## Baseline snapshot\n\n");
        s.push_str(&format!("- lint_warnings: {} → {}\n", self.baseline_metrics.lint_warnings, self.current_metrics.lint_warnings));
        s.push_str(&format!("- type_errors:   {} → {}\n", self.baseline_metrics.type_errors, self.current_metrics.type_errors));
        s.push_str(&format!("- test_count:    {} → {}\n", self.baseline_metrics.test_count, self.current_metrics.test_count));
        s.push_str(&format!("- tests_passing: {} → {}\n", self.baseline_metrics.tests_passing, self.current_metrics.tests_passing));
        s.push_str(&format!("- coverage:      {:.2}% → {:.2}%\n", self.baseline_metrics.coverage_percent, self.current_metrics.coverage_percent));
        s
    }
}

fn abbreviate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.into() }
    else { let mut t: String = s.chars().take(max - 3).collect(); t.push_str("..."); t }
}

pub fn build_suggestions(regressions: &[Regression], current: &Metrics, baseline: &Baseline) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in regressions {
        match r.metric {
            "lint_warnings" => out.push(format!(
                "Lint warnings increased to {}. Either fix the new warnings or justify them via tool config.",
                current.lint_warnings
            )),
            "type_errors" => out.push(format!(
                "Type errors increased to {}. Add type annotations or fix the new errors before claiming success.",
                current.type_errors
            )),
            "test_count" => out.push(format!(
                "Test count dropped from {} to {}. If you removed a test, replace it with equivalent or stronger coverage.",
                baseline.metrics.test_count, current.test_count
            )),
            "tests_passing" => out.push(format!(
                "{} test(s) regressed from passing to failing. Investigate before committing.",
                baseline.metrics.tests_passing - current.tests_passing
            )),
            "coverage_percent" => out.push(format!(
                "Coverage dropped from {:.2}% to {:.2}%. Add tests covering the changed code paths — Capers Jones research shows steep defect-escape cliff below ~85%.",
                baseline.metrics.coverage_percent, current.coverage_percent
            )),
            _ => {}
        }
    }
    out
}

pub fn now_iso() -> String { Utc::now().to_rfc3339() }
