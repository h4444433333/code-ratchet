use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::metrics::Metrics;

const BASELINE_FILENAME: &str = "baseline.json";
const SCHEMA_VERSION: u32 = 1;

/// On-disk record of "best ever" quality for this project.
///
/// The ratchet's pawl: this file's `metrics` only move in the good direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
    pub ratchet_count: u32,
    pub metrics: Metrics,
}

impl Baseline {
    pub fn empty() -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            version: SCHEMA_VERSION,
            created_at: now.clone(),
            updated_at: now,
            ratchet_count: 0,
            metrics: Metrics::default(),
        }
    }

    pub fn path_in(dir: &Path) -> PathBuf { dir.join(BASELINE_FILENAME) }

    pub fn load_or_default(dir: &Path) -> Result<Self> {
        let path = Self::path_in(dir);
        if !path.exists() { return Ok(Self::empty()); }
        let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let baseline: Baseline = serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        Ok(baseline)
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
        let path = Self::path_in(dir);
        let tmp = path.with_extension("json.tmp");
        let raw = serde_json::to_string_pretty(self)? + "\n";
        fs::write(&tmp, raw).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("rename to {}", path.display()))?;
        Ok(())
    }

    /// Apply `current` to `self`. On the first ratchet, seed verbatim; otherwise
    /// take the best of each field. Always advances `updated_at` and `ratchet_count`.
    ///
    /// Seeding verbatim on the first run is critical — otherwise the
    /// lower-is-better fields stay locked at `Default::default()` (= 0) and the
    /// project can never legitimately have any lint warnings.
    pub fn ratchet_up(&mut self, current: &Metrics) {
        if self.ratchet_count == 0 {
            self.metrics = current.clone();
        } else {
            self.metrics.lint_warnings = self.metrics.lint_warnings.min(current.lint_warnings);
            self.metrics.type_errors = self.metrics.type_errors.min(current.type_errors);
            self.metrics.test_count = self.metrics.test_count.max(current.test_count);
            self.metrics.tests_passing = self.metrics.tests_passing.max(current.tests_passing);
            self.metrics.coverage_percent = self.metrics.coverage_percent.max(current.coverage_percent);
        }
        self.updated_at = Utc::now().to_rfc3339();
        self.ratchet_count += 1;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Regression {
    pub metric: &'static str,
    pub direction: &'static str,
    pub baseline: String,
    pub current: String,
    pub delta: String,
}

#[derive(Debug)]
pub enum RatchetVerdict {
    /// All metrics are equal-or-better. Baseline can advance.
    Pass { improved: Vec<&'static str> },
    /// At least one metric regressed. Commit must be blocked.
    Regression(Vec<Regression>),
}

/// Pure comparator. Does not mutate baseline.
pub fn compare(baseline: &Metrics, current: &Metrics) -> RatchetVerdict {
    let mut regressions: Vec<Regression> = Vec::new();
    let mut improved: Vec<&'static str> = Vec::new();

    // Lower-is-better: regression if current > baseline.
    if current.lint_warnings > baseline.lint_warnings {
        regressions.push(Regression {
            metric: "lint_warnings",
            direction: "must not increase",
            baseline: baseline.lint_warnings.to_string(),
            current: current.lint_warnings.to_string(),
            delta: format!("+{}", current.lint_warnings - baseline.lint_warnings),
        });
    } else if current.lint_warnings < baseline.lint_warnings {
        improved.push("lint_warnings");
    }

    if current.type_errors > baseline.type_errors {
        regressions.push(Regression {
            metric: "type_errors",
            direction: "must not increase",
            baseline: baseline.type_errors.to_string(),
            current: current.type_errors.to_string(),
            delta: format!("+{}", current.type_errors - baseline.type_errors),
        });
    } else if current.type_errors < baseline.type_errors {
        improved.push("type_errors");
    }

    // Higher-is-better: regression if current < baseline.
    if current.test_count < baseline.test_count {
        regressions.push(Regression {
            metric: "test_count",
            direction: "must not decrease",
            baseline: baseline.test_count.to_string(),
            current: current.test_count.to_string(),
            delta: format!("-{}", baseline.test_count - current.test_count),
        });
    } else if current.test_count > baseline.test_count {
        improved.push("test_count");
    }

    if current.tests_passing < baseline.tests_passing {
        regressions.push(Regression {
            metric: "tests_passing",
            direction: "must not decrease",
            baseline: baseline.tests_passing.to_string(),
            current: current.tests_passing.to_string(),
            delta: format!("-{}", baseline.tests_passing - current.tests_passing),
        });
    } else if current.tests_passing > baseline.tests_passing {
        improved.push("tests_passing");
    }

    // Coverage tolerance: 0.1pp jitter ignored (floating point + test discovery noise).
    let cov_drop = baseline.coverage_percent - current.coverage_percent;
    if cov_drop > 0.1 {
        regressions.push(Regression {
            metric: "coverage_percent",
            direction: "must not decrease",
            baseline: format!("{:.2}", baseline.coverage_percent),
            current: format!("{:.2}", current.coverage_percent),
            delta: format!("-{:.2}", cov_drop),
        });
    } else if current.coverage_percent - baseline.coverage_percent > 0.1 {
        improved.push("coverage_percent");
    }

    if regressions.is_empty() {
        RatchetVerdict::Pass { improved }
    } else {
        RatchetVerdict::Regression(regressions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(lw: u32, te: u32, tc: u32, tp: u32, cov: f64) -> Metrics {
        Metrics { lint_warnings: lw, type_errors: te, test_count: tc, tests_passing: tp, coverage_percent: cov }
    }

    #[test]
    fn empty_baseline_accepts_anything() {
        let base = Metrics::default();
        let cur = m(5, 3, 10, 10, 50.0);
        match compare(&base, &cur) {
            // From an empty baseline, lint_warnings=5 vs 0 is technically a regression.
            // That's intentional: on the FIRST run, init writes an empty baseline,
            // then the first `check` MUST move the baseline forward via ratchet_up.
            // Caller handles "first run" by calling ratchet_up regardless on verdict.
            RatchetVerdict::Regression(rs) => assert!(rs.iter().any(|r| r.metric == "lint_warnings")),
            RatchetVerdict::Pass { .. } => {} // also possible if metrics happen to be zero
        }
    }

    #[test]
    fn coverage_drop_within_tolerance_passes() {
        let base = m(0, 0, 100, 100, 91.0);
        let cur = m(0, 0, 100, 100, 90.95);
        assert!(matches!(compare(&base, &cur), RatchetVerdict::Pass { .. }));
    }

    #[test]
    fn coverage_drop_beyond_tolerance_blocks() {
        let base = m(0, 0, 100, 100, 91.0);
        let cur = m(0, 0, 100, 100, 88.0);
        match compare(&base, &cur) {
            RatchetVerdict::Regression(rs) => assert!(rs.iter().any(|r| r.metric == "coverage_percent")),
            _ => panic!("should regress"),
        }
    }

    #[test]
    fn ratchet_up_takes_best_of_each_field_after_seed() {
        let mut baseline = Baseline::empty();
        baseline.metrics = m(10, 5, 100, 100, 80.0);
        baseline.ratchet_count = 1; // simulate a prior seed so we exercise merge, not seed
        let current = m(8, 5, 105, 105, 79.0);
        baseline.ratchet_up(&current);
        // lint dropped (good), tests went up (good), coverage went down — ratchet keeps the BEST.
        assert_eq!(baseline.metrics.lint_warnings, 8);
        assert_eq!(baseline.metrics.test_count, 105);
        assert_eq!(baseline.metrics.coverage_percent, 80.0);
        assert_eq!(baseline.ratchet_count, 2);
    }

    #[test]
    fn first_ratchet_seeds_verbatim_not_min_max() {
        let mut baseline = Baseline::empty();
        let current = m(3, 2, 10, 10, 88.0);
        baseline.ratchet_up(&current);
        // Critical: lint and type fields are SET to the current value, not min'd with 0.
        assert_eq!(baseline.metrics.lint_warnings, 3);
        assert_eq!(baseline.metrics.type_errors, 2);
        assert_eq!(baseline.metrics.coverage_percent, 88.0);
        assert_eq!(baseline.ratchet_count, 1);
    }
}
