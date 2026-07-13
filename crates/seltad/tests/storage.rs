//! The storage contract, pinned once and run against every backend — the
//! two must be indistinguishable through the trait (docs/06 §Catalog).

use serde_json::{json, Value};

use seltad::storage::{
    self, FileStorage, PoolConfig, SqliteStorage, StatsRow, Storage, StorageBackend,
};

fn pool(name: &str) -> PoolConfig {
    serde_json::from_value(json!({ "name": name, "extensions": ["judge"] })).expect("pool config")
}

fn schema(marker: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({ "type": "str", "description": marker })).expect("schema serializes")
}

fn schema_json(source: &[u8]) -> Value {
    serde_json::from_slice(source).expect("stored test schema parses")
}

/// Every behavioral guarantee the trait makes, in one pass.
fn exercise(storage: &dyn Storage) {
    // Pools: created once, listed sorted, loaded back, updated whole.
    assert!(storage.create_pool(&pool("app_b")).expect("create"));
    assert!(storage.create_pool(&pool("app_a")).expect("create"));
    assert!(!storage
        .create_pool(&pool("app_a"))
        .expect("duplicate is Ok(false)"));
    assert!(storage.create_pool(&pool("bad name")).is_err());
    assert_eq!(storage.list_pools().expect("list"), vec!["app_a", "app_b"]);
    assert!(storage
        .load_pool("missing")
        .expect("missing pool is None")
        .is_none());
    assert!(storage
        .load_pool("../escape")
        .expect("invalid name is None")
        .is_none());
    let mut config = pool("app_a");
    config
        .settings
        .insert("judge".to_string(), json!({ "model": "cheap" }));
    storage.update_pool(&config).expect("update");
    let loaded = storage.load_pool("app_a").expect("load").expect("exists");
    assert_eq!(loaded.settings["judge"], json!({ "model": "cheap" }));
    assert!(storage.update_pool(&pool("missing")).is_err());

    // Schemas: versions assigned sequentially from 1, immutable, pinnable.
    assert_eq!(
        storage
            .register_schema("app_a", "slug", &schema("v1"))
            .expect("v1"),
        1
    );
    assert_eq!(
        storage
            .register_schema("app_a", "slug", &schema("v2"))
            .expect("v2"),
        2
    );
    assert_eq!(
        storage
            .register_schema("app_a", "other", &schema("o1"))
            .expect("o1"),
        1
    );
    assert!(storage
        .register_schema("app_a", "bad name", &schema("x"))
        .is_err());
    let (version, latest) = storage.load_schema("app_a", "slug", None).expect("latest");
    assert_eq!(
        (version, schema_json(&latest)["description"].as_str()),
        (2, Some("v2"))
    );
    let (version, pinned) = storage
        .load_schema("app_a", "slug", Some(1))
        .expect("pinned");
    assert_eq!(
        (version, schema_json(&pinned)["description"].as_str()),
        (1, Some("v1"))
    );
    let missing = storage.load_schema("app_a", "nope", None).unwrap_err();
    assert!(
        missing.to_string().contains("not found in pool 'app_a'"),
        "{missing}"
    );
    assert!(storage.load_schema("app_a", "slug", Some(9)).is_err());
    let listing = storage.list_schemas("app_a").expect("list schemas");
    assert_eq!(listing["slug"], vec![1, 2]);
    assert_eq!(listing["other"], vec![1]);

    // Storage is deliberately syntax-agnostic and byte-preserving. Strict
    // admission happens above this trait and must still be able to see a
    // duplicate key or malformed source after a reload.
    let raw = br#"{ "type": "str", "type": "int" }"#;
    storage
        .register_schema("app_a", "raw", raw)
        .expect("raw source");
    let (_, loaded_raw) = storage
        .load_schema("app_a", "raw", None)
        .expect("raw reload");
    assert_eq!(loaded_raw, raw);
    assert!(storage
        .register_schema("app_a", "not_utf8", &[0xff])
        .is_err());

    // Stats: opaque rows, empty until saved, upserted whole.
    assert!(storage.load_stats().expect("empty stats").is_empty());
    let row = |data: Value| StatsRow {
        pool: "app_a".to_string(),
        ext: "judge".to_string(),
        data,
    };
    storage
        .save_stats(&[row(json!({ "calls": 1 }))])
        .expect("save");
    storage
        .save_stats(&[row(json!({ "calls": 5 }))])
        .expect("upsert");
    let rows = storage.load_stats().expect("load stats");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].data, json!({ "calls": 5 }));
}

#[test]
#[cfg(not(windows))]
fn files_backend_honors_the_contract() {
    let dir = tempfile::tempdir().expect("tempdir");
    exercise(&FileStorage::open(dir.path().to_path_buf()).expect("opens"));
}

#[test]
#[cfg(windows)]
fn configured_files_backend_fails_closed_on_windows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let Err(direct_error) = FileStorage::open(dir.path().to_path_buf()) else {
        panic!("the public file catalog unexpectedly opened")
    };
    assert!(direct_error
        .to_string()
        .contains("not supported on Windows"));
    let Err(error) = storage::open(StorageBackend::Files, dir.path()) else {
        panic!("the unsupported file catalog unexpectedly opened")
    };
    assert!(error.to_string().contains("not supported on Windows"));
}

#[test]
fn sqlite_backend_honors_the_contract() {
    let dir = tempfile::tempdir().expect("tempdir");
    exercise(&SqliteStorage::open(&dir.path().join("selta.db")).expect("opens"));
}

#[test]
fn sqlite_persists_across_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("selta.db");
    {
        let storage = SqliteStorage::open(&db).expect("opens");
        storage.create_pool(&pool("app")).expect("create");
        storage
            .register_schema("app", "slug", &schema("v1"))
            .expect("v1");
        storage
            .save_stats(&[StatsRow {
                pool: "app".to_string(),
                ext: "judge".to_string(),
                data: json!({ "calls": 7 }),
            }])
            .expect("save");
    }
    let storage = SqliteStorage::open(&db).expect("reopens");
    assert_eq!(storage.list_pools().expect("pools"), vec!["app"]);
    let (version, _) = storage.load_schema("app", "slug", None).expect("schema");
    assert_eq!(version, 1);
    assert_eq!(storage.load_stats().expect("stats")[0].data["calls"], 7);
}

#[test]
#[cfg(not(windows))]
fn fresh_sqlite_imports_an_existing_file_catalog_exactly_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let files = FileStorage::open(dir.path().to_path_buf()).expect("opens");
    files.create_pool(&pool("app")).expect("create");
    files
        .register_schema("app", "slug", &schema("v1"))
        .expect("v1");
    files
        .register_schema("app", "slug", &schema("v2"))
        .expect("v2");

    // First sqlite open at this data dir: the file catalog comes along.
    let storage = storage::open(StorageBackend::Sqlite, dir.path()).expect("opens");
    assert_eq!(storage.list_pools().expect("pools"), vec!["app"]);
    let (version, body) = storage.load_schema("app", "slug", None).expect("schema");
    assert_eq!(
        (version, schema_json(&body)["description"].as_str()),
        (2, Some("v2"))
    );
    drop(storage);

    // Later file-catalog writes stay where they are: the import ran once.
    files.create_pool(&pool("late")).expect("create");
    let storage = storage::open(StorageBackend::Sqlite, dir.path()).expect("reopens");
    assert_eq!(storage.list_pools().expect("pools"), vec!["app"]);
}

#[test]
#[cfg(windows)]
fn fresh_sqlite_rejects_a_legacy_file_catalog_before_creating_a_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("pools")).expect("legacy catalog marker");
    let Err(error) = storage::open(StorageBackend::Sqlite, dir.path()) else {
        panic!("the unsupported automatic import unexpectedly opened")
    };
    assert!(error.to_string().contains("import"));
    assert!(!dir.path().join("selta.db").exists());
}
