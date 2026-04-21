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
    pub mpv_bin: Option<String>,
    pub mpv_args: Option<Vec<String>>,
    #[serde(default)]
    pub mpv_ontop: bool,
    #[serde(default)]
    pub accent_color: Option<String>,
    #[serde(default)]
    pub terminal_colors: bool,
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
