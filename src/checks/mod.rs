//! Layer execution: spawn the configured command, capture output, extract metrics.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::config::LayerCommand;
use crate::metrics::{CheckOutcome, Layer};

pub fn run_layer(layer: Layer, cmd: &LayerCommand, cwd: &Path) -> Result<CheckOutcome> {
    if !cmd.enabled() {
        return Ok(CheckOutcome {
            layer, command: String::new(), exit_code: 0,
            stdout: String::new(), stderr: String::new(),
            numeric_signal: 0, passing: None, coverage: None,
            executed: false, note: Some("layer disabled in config".into()),
        });
    }
    let (shell, flag) = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    let output = Command::new(shell)
        .arg(flag)
        .arg(&cmd.command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("spawn `{}`", cmd.command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output.status.code().unwrap_or(-1);

    let (numeric_signal, passing, coverage) = match layer {
        Layer::L0 => (parse_l0_warnings(&stdout, exit_code), None, None),
        Layer::L1 => (parse_l1_errors(&stdout, exit_code), None, None),
        Layer::L2 => {
            let (count, passing) = parse_l2_tests(&stdout, exit_code);
            let coverage = parse_l2_coverage(&stdout);
            (count, passing, coverage)
        }
    };

    // The tool ran if we got an exit code we can interpret; we count it as
    // "executed" even on non-zero exit, because lint warnings legitimately
    // exit non-zero. The CLI layer treats "tool crashed" (signaled / -1)
    // differently from "tool reported issues".
    let executed = output.status.code().is_some();

    Ok(CheckOutcome {
        layer, command: cmd.command.clone(), exit_code, stdout, stderr,
        numeric_signal, passing, coverage, executed, note: None,
    })
}

/// Heuristic: ruff/eslint emit one warning per line `path:line:col: CODE message`.
/// Count non-empty stdout lines on non-zero exit. Exit 0 → 0 warnings.
pub fn parse_l0_warnings(stdout: &str, exit_code: i32) -> u32 {
    if exit_code == 0 { return 0; }
    let count = stdout.lines()
        .filter(|l| {
            // Match `something:NUMBER:NUMBER:` prefix (ruff/eslint concise format)
            let mut parts = l.splitn(4, ':');
            let _file = parts.next();
            let line = parts.next();
            let col = parts.next();
            matches!((line, col), (Some(l), Some(c)) if l.trim().parse::<u32>().is_ok() && c.trim().parse::<u32>().is_ok())
        })
        .count();
    if count == 0 { 1 } else { count as u32 }
}

/// Heuristic: mypy emits `Found N errors in M files (checked K source files)`.
pub fn parse_l1_errors(stdout: &str, exit_code: i32) -> u32 {
    if exit_code == 0 { return 0; }
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Found ") {
            if let Some(num_str) = rest.split_whitespace().next() {
                if let Ok(n) = num_str.parse::<u32>() { return n; }
            }
        }
    }
    1
}

/// Parse pytest summary `5 passed, 1 failed, 2 skipped in 1.23s` style line.
/// Returns (total_tests, passing).
pub fn parse_l2_tests(stdout: &str, _exit_code: i32) -> (u32, Option<u32>) {
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut errors = 0u32;
    let mut skipped = 0u32;
    let mut found = false;
    for line in stdout.lines() {
        let line = line.trim();
        // Match pytest's `== 5 passed, 1 failed in 0.12s ==` or similar.
        if !(line.contains("passed") || line.contains("failed") || line.contains("error")) { continue; }
        if !line.contains(" in ") && !line.contains("=") { continue; }
        let tokens: Vec<&str> = line.split(|c: char| c.is_whitespace() || c == ',' || c == '=').filter(|s| !s.is_empty()).collect();
        for w in tokens.windows(2) {
            let num = w[0].trim_end_matches('s').parse::<u32>();
            if let Ok(n) = num {
                match w[1] {
                    "passed" => { passed = n; found = true; }
                    "failed" => { failed = n; found = true; }
                    "error" | "errors" => { errors = n; found = true; }
                    "skipped" => { skipped = n; found = true; }
                    _ => {}
                }
            }
        }
    }
    if !found { return (0, None); }
    let total = passed + failed + errors + skipped;
    (total, Some(passed))
}

/// Parse `TOTAL ... 90%` line from `pytest --cov` terminal report.
pub fn parse_l2_coverage(stdout: &str) -> Option<f64> {
    for line in stdout.lines() {
        let line = line.trim();
        if !line.starts_with("TOTAL") { continue; }
        if let Some(pct_token) = line.split_whitespace().last() {
            if let Some(num) = pct_token.strip_suffix('%') {
                if let Ok(v) = num.parse::<f64>() { return Some(v); }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_counts_ruff_lines() {
        let out = "src/a.py:5:1: F401 unused\nsrc/b.py:10:80: E501 line too long\n";
        assert_eq!(parse_l0_warnings(out, 1), 2);
    }

    #[test]
    fn l0_clean_when_exit_zero() {
        assert_eq!(parse_l0_warnings("All checks passed!", 0), 0);
    }

    #[test]
    fn l1_extracts_mypy_count() {
        let out = "src/a.py:5: error: bad\nFound 3 errors in 1 file (checked 2 source files)\n";
        assert_eq!(parse_l1_errors(out, 1), 3);
    }

    #[test]
    fn l2_extracts_pytest_summary() {
        let out = "=========================== 12 passed in 0.34s ============================\n";
        let (total, passing) = parse_l2_tests(out, 0);
        assert_eq!(total, 12);
        assert_eq!(passing, Some(12));
    }

    #[test]
    fn l2_extracts_mixed_pytest_summary() {
        let out = "== 5 passed, 1 failed, 2 skipped in 0.12s ==";
        let (total, passing) = parse_l2_tests(out, 1);
        assert_eq!(total, 8);
        assert_eq!(passing, Some(5));
    }

    #[test]
    fn l2_extracts_coverage_total_line() {
        let out = "Name           Stmts   Miss  Cover\n---\nsrc/a.py        100      9    91%\nTOTAL           100      9    91%\n";
        assert_eq!(parse_l2_coverage(out), Some(91.0));
    }
}
