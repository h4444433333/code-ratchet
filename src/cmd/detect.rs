//! Lightweight project language detection used by `code-ratchet setup`.
//! Conservative: degrades to `Unknown` if signals are weak.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedLanguage {
    Python,
    JavaScript,
    TypeScript,
    Unknown,
}

impl DetectedLanguage {
    pub fn label(self) -> &'static str {
        match self {
            DetectedLanguage::Python => "Python",
            DetectedLanguage::JavaScript => "JavaScript",
            DetectedLanguage::TypeScript => "TypeScript",
            DetectedLanguage::Unknown => "Unknown",
        }
    }
}

pub fn detect_language(repo_root: &Path) -> DetectedLanguage {
    let py = repo_root.join("pyproject.toml").exists()
        || repo_root.join("setup.py").exists()
        || repo_root.join("requirements.txt").exists();
    let pkg = repo_root.join("package.json").exists();
    let ts = pkg && (repo_root.join("tsconfig.json").exists() || repo_root.join("tsconfig.base.json").exists());

    match (py, pkg, ts) {
        (_, true, true) => DetectedLanguage::TypeScript,
        (_, true, false) => DetectedLanguage::JavaScript,
        (true, false, _) => DetectedLanguage::Python,
        _ => DetectedLanguage::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tempdir(name: &str) -> std::path::PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("cr-detect-{}-{}", name, std::process::id()));
        if d.exists() { fs::remove_dir_all(&d).ok(); }
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn detects_python_via_pyproject() {
        let d = tempdir("py");
        fs::write(d.join("pyproject.toml"), "").unwrap();
        assert_eq!(detect_language(&d), DetectedLanguage::Python);
    }

    #[test]
    fn detects_typescript_via_tsconfig() {
        let d = tempdir("ts");
        fs::write(d.join("package.json"), "{}").unwrap();
        fs::write(d.join("tsconfig.json"), "{}").unwrap();
        assert_eq!(detect_language(&d), DetectedLanguage::TypeScript);
    }

    #[test]
    fn detects_javascript_without_tsconfig() {
        let d = tempdir("js");
        fs::write(d.join("package.json"), "{}").unwrap();
        assert_eq!(detect_language(&d), DetectedLanguage::JavaScript);
    }

    #[test]
    fn unknown_when_nothing_matches() {
        let d = tempdir("nothing");
        assert_eq!(detect_language(&d), DetectedLanguage::Unknown);
    }
}
