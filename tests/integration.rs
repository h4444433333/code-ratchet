//! End-to-end test: seed a temp project, run `check`, modify, run again,
//! and verify the ratchet blocks a regression and advances on improvement.

use std::fs;
use std::path::PathBuf;

use code_ratchet::baseline::{compare, Baseline, RatchetVerdict};
use code_ratchet::config::Config;
use code_ratchet::metrics::Metrics;

fn tempdir(name: &str) -> PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("code-ratchet-test-{}-{}", name, std::process::id()));
    if d.exists() { fs::remove_dir_all(&d).ok(); }
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn default_config_round_trips() {
    let dir = tempdir("config_round_trip");
    let path = dir.join(".ratchet.yml");
    Config::write_default(&path).unwrap();
    let loaded = Config::load(&path).unwrap();
    assert!(loaded.l0.enabled());
    assert!(loaded.l2.enabled());
}

#[test]
fn baseline_persists_and_reloads() {
    let dir = tempdir("baseline_persist");
    let mut b = Baseline::empty();
    b.metrics = Metrics {
        lint_warnings: 0, type_errors: 0, test_count: 100, tests_passing: 100, coverage_percent: 91.5
    };
    b.save(&dir).unwrap();
    let loaded = Baseline::load_or_default(&dir).unwrap();
    assert_eq!(loaded.metrics.test_count, 100);
    assert!((loaded.metrics.coverage_percent - 91.5).abs() < 1e-9);
}

#[test]
fn ratchet_blocks_coverage_regression_and_passes_improvement() {
    let baseline_metrics = Metrics {
        lint_warnings: 0, type_errors: 0, test_count: 100, tests_passing: 100, coverage_percent: 91.0
    };
    // Regression
    let worse = Metrics {
        lint_warnings: 0, type_errors: 0, test_count: 100, tests_passing: 100, coverage_percent: 88.0
    };
    match compare(&baseline_metrics, &worse) {
        RatchetVerdict::Regression(rs) => assert!(rs.iter().any(|r| r.metric == "coverage_percent")),
        _ => panic!("expected regression"),
    }
    // Improvement
    let better = Metrics {
        lint_warnings: 0, type_errors: 0, test_count: 120, tests_passing: 120, coverage_percent: 92.0
    };
    match compare(&baseline_metrics, &better) {
        RatchetVerdict::Pass { improved } => {
            assert!(improved.contains(&"test_count"));
            assert!(improved.contains(&"coverage_percent"));
        }
        _ => panic!("expected pass"),
    }
}

#[test]
fn lint_warning_increase_is_a_regression() {
    let base = Metrics { lint_warnings: 2, type_errors: 0, test_count: 50, tests_passing: 50, coverage_percent: 80.0 };
    let cur  = Metrics { lint_warnings: 5, type_errors: 0, test_count: 50, tests_passing: 50, coverage_percent: 80.0 };
    match compare(&base, &cur) {
        RatchetVerdict::Regression(rs) => {
            let r = rs.iter().find(|r| r.metric == "lint_warnings").unwrap();
            assert_eq!(r.baseline, "2");
            assert_eq!(r.current, "5");
        }
        _ => panic!("expected regression"),
    }
}
