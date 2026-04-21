use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server_url: String,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub user_id: Option<String>,
    #[serde(default)]
    pub mpv: MpvConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneralConfig {
    #[serde(default = "default_true")]
    pub mouse: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self { mouse: true }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    pub accent: Option<String>,
    #[serde(default)]
    pub terminal_colors: bool,
    pub selection_bg: Option<String>,
    pub selection_fg: Option<String>,
    pub muted_fg: Option<String>,
    pub title_fg: Option<String>,
    pub hint_fg: Option<String>,
    pub description_fg: Option<String>,
    pub breadcrumb_current_fg: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MpvConfig {
    pub bin: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub ontop: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        let mut config: Self = toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML config at {}", path.display()))?;

        config.server_url = config.server_url.trim_end_matches('/').to_string();
        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.server_url.is_empty() {
            bail!("config field `server_url` must not be empty");
        }

        let has_api_key = self
            .api_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let has_credentials = self
            .username
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            && self
                .password
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());

        if !has_api_key && !has_credentials {
            bail!("set either `api_key` or both `username` and `password` in the config");
        }

        Ok(())
    }
}

fn config_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("GELTUI_CONFIG") {
        return Ok(PathBuf::from(path));
    }

    let base = dirs::config_dir().context("could not resolve config directory")?;
    Ok(base.join("geltui").join("config.toml"))
}
