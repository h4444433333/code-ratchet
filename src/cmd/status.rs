use std::path::Path;

use anyhow::Result;

use crate::baseline::Baseline;
use crate::cmd::ratchet_dir;

pub fn run(repo_root: &Path) -> Result<()> {
    let dir = ratchet_dir(repo_root);
    let baseline = Baseline::load_or_default(&dir)?;
    println!("Baseline (ratchet_count={}, updated_at={}):", baseline.ratchet_count, baseline.updated_at);
    println!("  lint_warnings: {}", baseline.metrics.lint_warnings);
    println!("  type_errors:   {}", baseline.metrics.type_errors);
    println!("  test_count:    {}", baseline.metrics.test_count);
    println!("  tests_passing: {}", baseline.metrics.tests_passing);
    println!("  coverage:      {:.2}%", baseline.metrics.coverage_percent);
    Ok(())
}
