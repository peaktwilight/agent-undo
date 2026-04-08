use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::ProjectPaths;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub gc: GcConfig,
    #[serde(default)]
    pub watch: WatchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcConfig {
    #[serde(default = "default_keep_last")]
    pub keep_last: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            keep_last: default_keep_last(),
        }
    }
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            max_file_size_mb: default_max_file_size_mb(),
            ignore_patterns: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn load(paths: &ProjectPaths) -> Result<Self> {
        if !paths.config_path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&paths.config_path)
            .with_context(|| format!("reading {}", paths.config_path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", paths.config_path.display()))
    }

    pub fn write_default_if_missing(paths: &ProjectPaths) -> Result<bool> {
        if paths.config_path.exists() {
            return Ok(false);
        }
        Self::write(paths, &Self::default())?;
        Ok(true)
    }

    pub fn write(paths: &ProjectPaths, config: &Self) -> Result<()> {
        if let Some(parent) = paths.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(config)?;
        std::fs::write(&paths.config_path, body.as_bytes())
            .with_context(|| format!("writing {}", paths.config_path.display()))?;
        Ok(())
    }

    pub fn gc_keep_last_ns(&self) -> Result<i64> {
        parse_duration_ns(&self.gc.keep_last)
    }

    pub fn max_file_size_bytes(&self) -> u64 {
        self.watch.max_file_size_mb.saturating_mul(1024 * 1024)
    }
}

fn default_keep_last() -> String {
    "7d".into()
}

fn default_max_file_size_mb() -> u64 {
    100
}

fn parse_duration_ns(s: &str) -> Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("duration cannot be empty");
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let n: i64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("could not parse duration '{s}'"))?;
    let mult = match unit {
        "s" => 1_000_000_000,
        "m" => 60 * 1_000_000_000,
        "h" => 60 * 60 * 1_000_000_000,
        "d" => 24 * 60 * 60 * 1_000_000_000_i64,
        _ => anyhow::bail!("unknown duration unit '{unit}', use s/m/h/d"),
    };
    Ok(n * mult)
}
