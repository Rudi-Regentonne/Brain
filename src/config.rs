use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TargetPlatform {
    #[serde(rename = "opencode")]
    OpenCode,
    #[serde(rename = "claude")]
    Claude,
}

impl TargetPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetPlatform::OpenCode => "opencode",
            TargetPlatform::Claude => "claude",
        }
    }

    pub fn default_dir_name(&self) -> &'static str {
        match self {
            TargetPlatform::OpenCode => ".opencode/skills",
            TargetPlatform::Claude => ".claude/skills",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "opencode" => Some(TargetPlatform::OpenCode),
            "claude" => Some(TargetPlatform::Claude),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompilerConfig {
    #[serde(default)]
    pub targets: Vec<TargetPlatform>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SafetyConfig {
    #[serde(default = "default_false")]
    pub overwrite_foreign_skills: bool,
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainConfig {
    pub project: ProjectConfig,
    pub compiler: CompilerConfig,
    #[serde(default)]
    pub safety: SafetyConfig,
}

impl BrainConfig {
    pub fn new(name: String, targets: Vec<TargetPlatform>) -> Self {
        Self {
            project: ProjectConfig { name },
            compiler: CompilerConfig { targets },
            safety: SafetyConfig::default(),
        }
    }

    pub fn load_from_dir(dir: &Path) -> Result<(Self, PathBuf), Box<dyn std::error::Error>> {
        let config_path = dir.join("brain.toml");
        if !config_path.exists() {
            return Err(
                format!("Configuration file not found at {}", config_path.display()).into(),
            );
        }
        let content = std::fs::read_to_string(&config_path)?;
        let config: BrainConfig = toml::from_str(&content)?;
        Ok((config, config_path))
    }

    pub fn save_to_dir(&self, dir: &Path) -> Result<PathBuf, std::io::Error> {
        std::fs::create_dir_all(dir)?;
        let config_path = dir.join("brain.toml");
        let content = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&config_path, content)?;
        Ok(config_path)
    }
}
