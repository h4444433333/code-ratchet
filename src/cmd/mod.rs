pub mod check;
pub mod detect;
pub mod init;
pub mod install_hook;
pub mod setup;
pub mod status;
pub mod uninstall;
pub mod watch;

use std::path::{Path, PathBuf};

/// Resolve and return the `.ratchet/` directory for this repo.
/// Creates it lazily if it doesn't exist.
pub fn ratchet_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".ratchet")
}

/// Resolve the user-facing config path: `.ratchet.yml` at the repo root.
pub fn config_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".ratchet.yml")
}
