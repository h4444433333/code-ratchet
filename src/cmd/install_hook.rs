use std::fs;
use std::path::Path;

use anyhow::{bail, Result};

const HOOK_SCRIPT: &str = include_str!("../../templates/pre-commit.sh");

pub fn run(repo_root: &Path, force: bool) -> Result<()> {
    let git_dir = repo_root.join(".git");
    if !git_dir.is_dir() {
        bail!(".git not found at {} — initialize a git repo first.", git_dir.display());
    }
    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    let hook_path = hooks_dir.join("pre-commit");

    if hook_path.exists() && !force {
        bail!("{} already exists. Pass --force to overwrite.", hook_path.display());
    }
    fs::write(&hook_path, HOOK_SCRIPT)?;
    chmod_executable(&hook_path)?;

    println!("installed pre-commit hook at {}", hook_path.display());
    println!("any future `git commit` that worsens metrics will be blocked.");
    Ok(())
}

#[cfg(unix)]
fn chmod_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn chmod_executable(_path: &Path) -> Result<()> {
    // On Windows, git hooks don't need an executable bit; git runs the
    // script via the shell on its own. No-op.
    Ok(())
}
