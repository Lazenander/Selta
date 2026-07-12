//! Server configuration: `selta.toml` (docs/06). Hosts and command templates
//! are server-level resources; pools opt into them by name.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use selta_core::CmdTemplate;
use serde::Deserialize;

use crate::storage::StorageBackend;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_data")]
    pub data: PathBuf,
    /// Catalog backend: sqlite (default) or files (docs/06 §Catalog).
    #[serde(default)]
    pub storage: StorageBackend,
    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,
    #[serde(default)]
    pub cmd: HashMap<String, CmdTemplate>,
    /// Server-scope extension settings: `[extensions.NAME.settings]`
    /// (docs/06). Pools override per key; secrets stay `$secret` references.
    #[serde(default)]
    pub extensions: HashMap<String, ExtensionConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostConfig {
    pub run: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtensionConfig {
    pub settings: Option<toml::Value>,
}

fn default_listen() -> String {
    "127.0.0.1:7466".to_string()
}

fn default_data() -> PathBuf {
    PathBuf::from("./selta-data")
}

impl Config {
    pub fn load(path: &Path) -> Result<Config> {
        if path.exists() {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
        } else {
            Ok(toml::from_str("").expect("empty config is all defaults"))
        }
    }

    /// Server-scope settings as JSON, keyed by extension name.
    pub fn server_settings(&self) -> std::collections::HashMap<String, serde_json::Value> {
        self.extensions
            .iter()
            .filter_map(|(name, ext)| {
                let settings = ext.settings.as_ref()?;
                let value = serde_json::to_value(settings).ok()?;
                Some((name.clone(), value))
            })
            .collect()
    }
}
