use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub token: Option<String>,
}

fn config_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "rlwy")
        .ok_or_else(|| anyhow!("could not resolve a config directory on this platform"))?;
    Ok(dirs.config_dir().join("config.json"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading config file at {}", path.display()))?;
    let cfg: Config = serde_json::from_str(&raw).context("parsing config file")?;
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<PathBuf> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }
    Ok(path)
}

/// Token lookup precedence: `$RLWY_TOKEN` (or `$RAILWAY_TOKEN`) env var
/// → config file written by `rlwy login`.
pub fn require_token() -> Result<String> {
    if let Ok(t) = std::env::var("RLWY_TOKEN") {
        if !t.trim().is_empty() {
            return Ok(t.trim().to_string());
        }
    }
    if let Ok(t) = std::env::var("RAILWAY_TOKEN") {
        if !t.trim().is_empty() {
            return Ok(t.trim().to_string());
        }
    }
    load()?
        .token
        .ok_or_else(|| anyhow!("no token found. set RLWY_TOKEN, add it to .env, or run `rlwy login`."))
}
