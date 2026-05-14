use std::fs;
use std::path::Path;

use anyhow::{bail, Result};

use crate::baseline::Baseline;
use crate::config::Config;
use crate::cmd::{config_path, ratchet_dir};

pub fn run(repo_root: &Path, force: bool) -> Result<()> {
    let cfg_path = config_path(repo_root);
    let dir = ratchet_dir(repo_root);

    if cfg_path.exists() && !force {
        bail!("{} already exists. Pass --force to overwrite.", cfg_path.display());
    }
    Config::write_default(&cfg_path)?;
    println!("wrote {}", cfg_path.display());

    fs::create_dir_all(&dir)?;
    let baseline_path = Baseline::path_in(&dir);
    if !baseline_path.exists() {
        Baseline::empty().save(&dir)?;
        println!("wrote {}", baseline_path.display());
    } else {
        println!("baseline already exists at {}, leaving it alone", baseline_path.display());
    }

    let gitignore = dir.join(".gitignore");
    fs::write(&gitignore, "feedback.json\nfeedback.md\n").ok();

    println!();
    println!("Next steps:");
    println!("  1. Edit {} to point at your real lint/typecheck/test commands.", cfg_path.display());
    println!("  2. Run `code-ratchet check` to seed the baseline.");
    println!("  3. (optional) `code-ratchet install-hook` to gate commits.");
    Ok(())
}
