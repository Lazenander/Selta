//! The sqlite backend (default): one file, transactional version assignment,
//! WAL so readers never block. The bundled sqlite keeps the daemon
//! dependency-free at deploy time.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use super::{valid_name, PoolConfig, StatsRow, Storage};
use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OptionalExtension};

pub struct SqliteStorage {
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    pub fn open(path: &Path) -> Result<SqliteStorage> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating catalog at {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening catalog at {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pools (
                 name   TEXT PRIMARY KEY,
                 config TEXT NOT NULL
             ) STRICT;
             CREATE TABLE IF NOT EXISTS schemas (
                 pool    TEXT NOT NULL,
                 name    TEXT NOT NULL,
                 version INTEGER NOT NULL,
                 body    TEXT NOT NULL,
                 PRIMARY KEY (pool, name, version)
             ) STRICT;
             CREATE TABLE IF NOT EXISTS stats (
                 pool TEXT NOT NULL,
                 ext  TEXT NOT NULL,
                 data TEXT NOT NULL,
                 PRIMARY KEY (pool, ext)
             ) STRICT;
             PRAGMA user_version = 1;",
        )?;
        Ok(SqliteStorage {
            conn: Mutex::new(conn),
        })
    }
}

impl Storage for SqliteStorage {
    fn create_pool(&self, config: &PoolConfig) -> Result<bool> {
        if !valid_name(&config.name) {
            bail!("invalid pool name '{}'", config.name);
        }
        let conn = self.conn.lock().unwrap();
        let inserted = conn.execute(
            "INSERT INTO pools (name, config) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
            (&config.name, serde_json::to_string_pretty(config)?),
        )?;
        Ok(inserted == 1)
    }

    fn update_pool(&self, config: &PoolConfig) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE pools SET config = ?2 WHERE name = ?1",
            (&config.name, serde_json::to_string_pretty(config)?),
        )?;
        if updated == 0 {
            bail!("pool '{}' not found", config.name);
        }
        Ok(())
    }

    fn load_pool(&self, name: &str) -> Result<Option<PoolConfig>> {
        if !valid_name(name) {
            return Ok(None);
        }
        let conn = self.conn.lock().unwrap();
        let text: Option<String> = conn
            .query_row("SELECT config FROM pools WHERE name = ?1", [name], |row| {
                row.get(0)
            })
            .optional()?;
        match text {
            Some(text) => Ok(Some(serde_json::from_str(&text)?)),
            None => Ok(None),
        }
    }

    fn list_pools(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut statement = conn.prepare("SELECT name FROM pools ORDER BY name")?;
        let names = statement
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(names)
    }

    fn register_schema(&self, pool: &str, name: &str, source: &[u8]) -> Result<u32> {
        if !valid_name(name) {
            bail!("invalid schema name '{name}'");
        }
        let source = std::str::from_utf8(source).context("schema source is not UTF-8")?;
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let version: u32 = tx.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM schemas WHERE pool = ?1 AND name = ?2",
            [pool, name],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT INTO schemas (pool, name, version, body) VALUES (?1, ?2, ?3, ?4)",
            (pool, name, version, source),
        )?;
        tx.commit()?;
        Ok(version)
    }

    fn load_schema(&self, pool: &str, name: &str, version: Option<u32>) -> Result<(u32, Vec<u8>)> {
        let conn = self.conn.lock().unwrap();
        let row: Option<(u32, String)> = match version {
            Some(v) => conn
                .query_row(
                    "SELECT version, body FROM schemas
                     WHERE pool = ?1 AND name = ?2 AND version = ?3",
                    (pool, name, v),
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?,
            None => conn
                .query_row(
                    "SELECT version, body FROM schemas
                     WHERE pool = ?1 AND name = ?2 ORDER BY version DESC LIMIT 1",
                    (pool, name),
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?,
        };
        match (row, version) {
            (Some((version, body)), _) => Ok((version, body.into_bytes())),
            (None, Some(v)) => bail!("schema '{name}@{v}' not found in pool '{pool}'"),
            (None, None) => bail!("schema '{name}' not found in pool '{pool}'"),
        }
    }

    fn list_schemas(&self, pool: &str) -> Result<BTreeMap<String, Vec<u32>>> {
        let conn = self.conn.lock().unwrap();
        let mut statement = conn
            .prepare("SELECT name, version FROM schemas WHERE pool = ?1 ORDER BY name, version")?;
        let mut out: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        let rows = statement.query_map([pool], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?;
        for row in rows {
            let (name, version) = row?;
            out.entry(name).or_default().push(version);
        }
        Ok(out)
    }

    fn load_stats(&self) -> Result<Vec<StatsRow>> {
        let conn = self.conn.lock().unwrap();
        let mut statement = conn.prepare("SELECT pool, ext, data FROM stats ORDER BY pool, ext")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (pool, ext, data) = row?;
            out.push(StatsRow {
                pool,
                ext,
                data: serde_json::from_str(&data)?,
            });
        }
        Ok(out)
    }

    fn save_stats(&self, rows: &[StatsRow]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for row in rows {
            tx.execute(
                "INSERT INTO stats (pool, ext, data) VALUES (?1, ?2, ?3)
                 ON CONFLICT (pool, ext) DO UPDATE SET data = excluded.data",
                (&row.pool, &row.ext, serde_json::to_string(&row.data)?),
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
