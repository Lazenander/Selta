//! The file backend: one directory per pool, one file per schema version.
//! Diffable and debuggable by design — the catalog reads as a tree.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use super::{valid_name, PoolConfig, StatsRow, Storage};

pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    pub fn open(root: PathBuf) -> Result<FileStorage> {
        fs::create_dir_all(root.join("pools"))
            .with_context(|| format!("creating catalog at {}", root.display()))?;
        Ok(FileStorage { root })
    }

    fn pool_dir(&self, pool: &str) -> PathBuf {
        self.root.join("pools").join(pool)
    }

    fn versions(&self, pool: &str, name: &str) -> Result<Vec<u32>> {
        let dir = self.pool_dir(pool).join("schemas").join(name);
        let mut versions = Vec::new();
        if dir.exists() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                    if let Ok(version) = stem.parse::<u32>() {
                        versions.push(version);
                    }
                }
            }
        }
        versions.sort();
        Ok(versions)
    }
}

impl Storage for FileStorage {
    fn create_pool(&self, config: &PoolConfig) -> Result<bool> {
        if !valid_name(&config.name) {
            bail!("invalid pool name '{}'", config.name);
        }
        let dir = self.pool_dir(&config.name);
        if dir.exists() {
            return Ok(false);
        }
        fs::create_dir_all(dir.join("schemas"))?;
        fs::write(dir.join("pool.json"), serde_json::to_vec_pretty(config)?)?;
        Ok(true)
    }

    fn update_pool(&self, config: &PoolConfig) -> Result<()> {
        let path = self.pool_dir(&config.name).join("pool.json");
        if !path.exists() {
            bail!("pool '{}' not found", config.name);
        }
        fs::write(path, serde_json::to_vec_pretty(config)?)?;
        Ok(())
    }

    fn load_pool(&self, name: &str) -> Result<Option<PoolConfig>> {
        if !valid_name(name) {
            return Ok(None);
        }
        let path = self.pool_dir(name).join("pool.json");
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path)?;
        Ok(Some(serde_json::from_str(&text)?))
    }

    fn list_pools(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in fs::read_dir(self.root.join("pools"))? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        Ok(names)
    }

    fn register_schema(&self, pool: &str, name: &str, schema: &Value) -> Result<u32> {
        if !valid_name(name) {
            bail!("invalid schema name '{name}'");
        }
        let dir = self.pool_dir(pool).join("schemas").join(name);
        fs::create_dir_all(&dir)?;
        let version = self.versions(pool, name)?.last().copied().unwrap_or(0) + 1;
        fs::write(
            dir.join(format!("{version}.json")),
            serde_json::to_vec_pretty(schema)?,
        )?;
        Ok(version)
    }

    fn load_schema(&self, pool: &str, name: &str, version: Option<u32>) -> Result<(u32, Value)> {
        let version = match version {
            Some(v) => v,
            None => match self.versions(pool, name)?.last() {
                Some(latest) => *latest,
                None => bail!("schema '{name}' not found in pool '{pool}'"),
            },
        };
        let path = self
            .pool_dir(pool)
            .join("schemas")
            .join(name)
            .join(format!("{version}.json"));
        let text = fs::read_to_string(&path)
            .with_context(|| format!("schema '{name}@{version}' not found in pool '{pool}'"))?;
        Ok((version, serde_json::from_str(&text)?))
    }

    fn list_schemas(&self, pool: &str) -> Result<BTreeMap<String, Vec<u32>>> {
        let dir = self.pool_dir(pool).join("schemas");
        let mut out = BTreeMap::new();
        if dir.exists() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let versions = self.versions(pool, &name)?;
                    out.insert(name, versions);
                }
            }
        }
        Ok(out)
    }

    fn load_stats(&self) -> Result<Vec<StatsRow>> {
        let path = self.root.join("stats.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        Ok(serde_json::from_str(&fs::read_to_string(&path)?)?)
    }

    fn save_stats(&self, rows: &[StatsRow]) -> Result<()> {
        // Write-then-rename: a crash mid-flush never truncates the counters.
        let tmp = self.root.join("stats.json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(rows)?)?;
        fs::rename(&tmp, self.root.join("stats.json"))?;
        Ok(())
    }
}
