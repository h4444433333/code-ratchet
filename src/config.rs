use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cmd::detect::DetectedLanguage;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language { Python, JavaScript, TypeScript, #[default] Auto }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerCommand {
    /// Shell command line. Empty string means "skip this layer".
    #[serde(default)]
    pub command: String,
    /// Treat layer failure as a hard block. Default true.
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool { true }

impl LayerCommand {
    pub fn enabled(&self) -> bool { !self.command.trim().is_empty() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub language: Language,
    pub l0: LayerCommand,
    pub l1: LayerCommand,
    pub l2: LayerCommand,
}

impl Config {
    pub fn default_python() -> Self {
        Self {
            language: Language::Python,
            l0: LayerCommand { command: "ruff check .".into(), required: true },
            l1: LayerCommand { command: "mypy .".into(), required: false },
            l2: LayerCommand { command: "pytest --cov=. --cov-report=term-missing -q".into(), required: true },
        }
    }

    pub fn default_javascript() -> Self {
        Self {
            language: Language::JavaScript,
            l0: LayerCommand { command: "npx --no-install eslint .".into(), required: true },
            l1: LayerCommand { command: "".into(), required: false },
            l2: LayerCommand { command: "npx --no-install jest --coverage --silent".into(), required: true },
        }
    }

    pub fn default_typescript() -> Self {
        Self {
            language: Language::TypeScript,
            l0: LayerCommand { command: "npx --no-install eslint .".into(), required: true },
            l1: LayerCommand { command: "npx --no-install tsc --noEmit".into(), required: true },
            l2: LayerCommand { command: "npx --no-install jest --coverage --silent".into(), required: true },
        }
    }

    pub fn default_unknown() -> Self {
        Self {
            language: Language::Auto,
            l0: LayerCommand { command: "".into(), required: false },
            l1: LayerCommand { command: "".into(), required: false },
            l2: LayerCommand { command: "".into(), required: false },
        }
    }

    pub fn for_detected(lang: DetectedLanguage) -> Self {
        match lang {
            DetectedLanguage::Python => Self::default_python(),
            DetectedLanguage::JavaScript => Self::default_javascript(),
            DetectedLanguage::TypeScript => Self::default_typescript(),
            DetectedLanguage::Unknown => Self::default_unknown(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let cfg: Config = serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        Ok(cfg)
    }

    pub fn write_default(path: &Path) -> Result<()> {
        Self::write_for_language(path, DetectedLanguage::Python)
    }

    pub fn write_for_language(path: &Path, lang: DetectedLanguage) -> Result<()> {
        let cfg = Self::for_detected(lang);
        let yaml = serde_yaml::to_string(&cfg)?;
        let prelude = format!(
            "# code-ratchet config (defaults: {})\n# Quality only goes up. Edit each command for your stack.\n# Empty `command` disables the layer.\n\n",
            lang.label()
        );
        if let Some(parent) = path.parent() { fs::create_dir_all(parent).ok(); }
        fs::write(path, format!("{}{}", prelude, yaml))?;
        Ok(())
    }
}
