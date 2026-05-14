use serde::{Deserialize, Serialize};

/// A snapshot of measurable quality at a point in time.
///
/// Two classes of fields:
/// - `lint_warnings`, `type_errors`: lower is better, monotonically non-increasing.
/// - `test_count`, `tests_passing`, `coverage_percent`: higher is better, monotonically non-decreasing.
///
/// The ratchet rejects any commit that worsens ANY of these versus the persisted baseline.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    pub lint_warnings: u32,
    pub type_errors: u32,
    pub test_count: u32,
    pub tests_passing: u32,
    pub coverage_percent: f64,
}

impl Metrics {
    pub fn merge_check(&mut self, outcome: &CheckOutcome) {
        match outcome.layer {
            Layer::L0 => self.lint_warnings = outcome.numeric_signal,
            Layer::L1 => self.type_errors = outcome.numeric_signal,
            Layer::L2 => {
                self.test_count = outcome.numeric_signal;
                if let Some(p) = outcome.passing { self.tests_passing = p; }
                if let Some(c) = outcome.coverage { self.coverage_percent = c; }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer { L0, L1, L2 }

impl Layer {
    pub fn label(self) -> &'static str {
        match self { Layer::L0 => "L0 lint", Layer::L1 => "L1 typecheck", Layer::L2 => "L2 test" }
    }
}

/// Result of running one stage check.
#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub layer: Layer,
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    /// Primary numeric signal extracted from the check.
    /// - L0: lint warning count (0 = clean)
    /// - L1: type error count (0 = clean)
    /// - L2: total test count
    pub numeric_signal: u32,
    /// L2 only: number of passing tests.
    pub passing: Option<u32>,
    /// L2 only: coverage percent 0.0..=100.0.
    pub coverage: Option<f64>,
    /// Whether the layer is considered to have *executed correctly*.
    /// Distinct from "metrics are good" — a tool crash counts as `executed = false`,
    /// a lint with warnings counts as `executed = true, numeric_signal > 0`.
    pub executed: bool,
    pub note: Option<String>,
}
