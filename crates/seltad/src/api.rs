//! The HTTP surface (docs/06): pools, schemas, verify, jobs, SSE.

use std::convert::Infallible;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use selta_core::meta::structure_only_ok;
use selta_core::{ExtensionDecl, Input, Mode, Node, Options, Runtime};

use crate::settings::{self, PoolSettings};
use crate::state::{pool_validate, AppState, Job, BUILTINS};
use crate::stats::PoolMonitor;
use crate::storage::PoolConfig;
use crate::ws;

pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

fn not_found(message: impl Into<String>) -> ApiError {
    ApiError(StatusCode::NOT_FOUND, message.into())
}

fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message.into())
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/extensions", get(list_extensions))
        .route("/pools", post(create_pool).get(list_pools))
        .route("/pools/{pool}", get(get_pool))
        .route("/pools/{pool}/schemas/{name}", put(put_schema).get(get_schema))
        .route("/pools/{pool}/extensions", get(pool_extensions))
        .route("/pools/{pool}/settings/{ext}", put(put_settings))
        .route("/pools/{pool}/hosts/connect", get(host_connect))
        .route("/pools/{pool}/stats", get(pool_stats))
        .route("/pools/{pool}/verify", post(verify))
        .route("/pools/{pool}/jobs/{id}", get(get_job).delete(cancel_job))
        .route("/pools/{pool}/jobs/{id}/events", get(job_events))
        .with_state(state)
}

fn schema_json(node: &Option<Node>) -> Value {
    node.as_ref()
        .and_then(|n| serde_json::to_value(n).ok())
        .unwrap_or(Value::Null)
}

fn decl_json(decl: &ExtensionDecl, source: &str, available: bool) -> Value {
    let mut needs = Vec::new();
    if decl.needs.root {
        needs.push("root");
    }
    if decl.needs.env {
        needs.push("env");
    }
    json!({
        "name": decl.name,
        "determinism": decl.determinism,
        "needs": needs,
        "config_schema": schema_json(&decl.config_schema),
        "settings_schema": schema_json(&decl.settings_schema),
        "delta_schema": schema_json(&decl.delta_schema),
        "source": source,
        "available": available,
    })
}

/// Server-scope extension manifests, for schema authors (docs/06).
async fn list_extensions(State(state): State<Arc<AppState>>) -> Response {
    let extensions: Vec<Value> = state
        .registry
        .decls()
        .iter()
        .map(|decl| {
            let source = if BUILTINS.contains(&decl.name.as_str()) {
                "builtin"
            } else {
                "server"
            };
            decl_json(decl, source, true)
        })
        .collect();
    Json(json!({ "extensions": extensions })).into_response()
}

/// The pool's view: what it may use, whether it is reachable, and the
/// resolved settings — secrets redacted (docs/06).
async fn pool_extensions(
    State(state): State<Arc<AppState>>,
    Path(pool): Path<String>,
) -> Result<Response, ApiError> {
    let pool_config = state
        .catalog
        .load_pool(&pool)
        .map_err(internal)?
        .ok_or_else(|| not_found(format!("pool '{pool}' not found")))?;
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for decl in state.registry.decls() {
        let builtin = BUILTINS.contains(&decl.name.as_str());
        if !builtin && !pool_config.extensions.iter().any(|e| *e == decl.name) {
            continue;
        }
        seen.insert(decl.name.clone());
        let source = if builtin { "builtin" } else { "server" };
        items.push(with_settings(decl_json(&decl, source, true), &state, &pool_config, &decl.name));
    }
    let hosts = state.pool_hosts.read().await;
    if let Some(entries) = hosts.get(&pool) {
        let mut names: Vec<&String> = entries.keys().collect();
        names.sort();
        for name in names {
            seen.insert(name.clone());
            let entry = &entries[name];
            items.push(with_settings(decl_json(&entry.decl, "pool", true), &state, &pool_config, name));
        }
    }
    drop(hosts);
    // Grants with no provider yet: real, just not reachable right now.
    for name in &pool_config.extensions {
        if !seen.contains(name) {
            items.push(with_settings(
                json!({ "name": name, "source": "granted", "available": false }),
                &state,
                &pool_config,
                name,
            ));
        }
    }
    Ok(Json(json!({ "pool": pool, "extensions": items })).into_response())
}

fn with_settings(mut item: Value, state: &AppState, pool: &PoolConfig, ext: &str) -> Value {
    let (redacted, fingerprint) = settings::public_view(&state.server_settings, &pool.settings, ext);
    item["settings"] = redacted;
    item["fingerprint"] = Value::String(fingerprint);
    item
}

/// Pool-scope settings override (docs/06): validated against the extension's
/// settings_schema when the extension is known, persisted in pool.json.
async fn put_settings(
    State(state): State<Arc<AppState>>,
    Path((pool, ext)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    if !body.is_object() {
        return Err(bad_request("settings must be a JSON object"));
    }
    let mut pool_config = state
        .catalog
        .load_pool(&pool)
        .map_err(internal)?
        .ok_or_else(|| not_found(format!("pool '{pool}' not found")))?;
    let registry = state.effective_registry(&pool).await;
    if let Some(decl) = registry.decl(&ext) {
        if let Some(schema) = &decl.settings_schema {
            let merged = settings::merge(state.server_settings.get(&ext), Some(&body));
            if !structure_only_ok(schema, &settings::redact(&merged)) {
                return Err(bad_request(format!(
                    "settings do not satisfy the settings_schema of '{ext}'"
                )));
            }
        }
    }
    pool_config.settings.insert(ext.clone(), body);
    state
        .catalog
        .update_pool(&pool_config)
        .map_err(internal)?;
    let (redacted, fingerprint) =
        settings::public_view(&state.server_settings, &pool_config.settings, &ext);
    Ok(Json(json!({ "ext": ext, "settings": redacted, "fingerprint": fingerprint }))
        .into_response())
}

/// An app-provided pool host dials in (docs/05 §websocket).
async fn host_connect(
    State(state): State<Arc<AppState>>,
    Path(pool): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    state
        .catalog
        .load_pool(&pool)
        .map_err(internal)?
        .ok_or_else(|| not_found(format!("pool '{pool}' not found")))?;
    Ok(upgrade.on_upgrade(move |socket| ws::handle_socket(state, pool, socket)))
}

/// Per-extension counters (docs/06 §Monitoring). In-memory v1.
async fn pool_stats(
    State(state): State<Arc<AppState>>,
    Path(pool): Path<String>,
) -> Result<Response, ApiError> {
    state
        .catalog
        .load_pool(&pool)
        .map_err(internal)?
        .ok_or_else(|| not_found(format!("pool '{pool}' not found")))?;
    Ok(Json(json!({ "pool": pool, "extensions": state.stats.pool_json(&pool) })).into_response())
}

async fn create_pool(
    State(state): State<Arc<AppState>>,
    Json(config): Json<PoolConfig>,
) -> Result<Response, ApiError> {
    // Grants are names, not bindings — an extension may arrive later via a
    // pool host dial-in. Settings for known extensions are validated now.
    for (ext, value) in &config.settings {
        if let Some(decl) = state.registry.decl(ext) {
            if let Some(schema) = &decl.settings_schema {
                let merged = settings::merge(state.server_settings.get(ext), Some(value));
                if !structure_only_ok(schema, &settings::redact(&merged)) {
                    return Err(bad_request(format!(
                        "settings do not satisfy the settings_schema of '{ext}'"
                    )));
                }
            }
        }
    }
    let created = state
        .catalog
        .create_pool(&config)
        .map_err(|e| bad_request(e.to_string()))?;
    if created {
        Ok((StatusCode::CREATED, Json(json!({ "pool": config.name }))).into_response())
    } else {
        Err(ApiError(
            StatusCode::CONFLICT,
            format!("pool '{}' already exists", config.name),
        ))
    }
}

async fn list_pools(State(state): State<Arc<AppState>>) -> Result<Response, ApiError> {
    let pools = state.catalog.list_pools().map_err(internal)?;
    Ok(Json(json!({ "pools": pools })).into_response())
}

async fn get_pool(
    State(state): State<Arc<AppState>>,
    Path(pool): Path<String>,
) -> Result<Response, ApiError> {
    let config = state
        .catalog
        .load_pool(&pool)
        .map_err(internal)?
        .ok_or_else(|| not_found(format!("pool '{pool}' not found")))?;
    let schemas = state.catalog.list_schemas(&pool).map_err(internal)?;
    Ok(Json(json!({ "pool": config, "schemas": schemas })).into_response())
}

async fn put_schema(
    State(state): State<Arc<AppState>>,
    Path((pool, name)): Path<(String, String)>,
    Json(schema_value): Json<Value>,
) -> Result<Response, ApiError> {
    let pool_config = state
        .catalog
        .load_pool(&pool)
        .map_err(internal)?
        .ok_or_else(|| not_found(format!("pool '{pool}' not found")))?;
    let node = Node::from_value(schema_value.clone())
        .map_err(|e| bad_request(format!("schema does not parse: {e}")))?;
    let registry = state.effective_registry(&pool).await;
    let pool_host_exts = state.pool_host_extensions(&pool).await;
    let errors = pool_validate(&node, &pool_config, &registry, &pool_host_exts);
    if !errors.is_empty() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            format!("schema rejected: {}", errors.join("; ")),
        ));
    }
    let version = state
        .catalog
        .register_schema(&pool, &name, &schema_value)
        .map_err(|e| bad_request(e.to_string()))?;
    Ok(Json(json!({ "name": name, "version": version })).into_response())
}

async fn get_schema(
    State(state): State<Arc<AppState>>,
    Path((pool, reference)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let (name, version) = parse_schema_ref(&reference)?;
    let (version, schema) = state
        .catalog
        .load_schema(&pool, &name, version)
        .map_err(|e| not_found(e.to_string()))?;
    Ok(Json(json!({ "name": name, "version": version, "schema": schema })).into_response())
}

fn parse_schema_ref(reference: &str) -> Result<(String, Option<u32>), ApiError> {
    match reference.split_once('@') {
        Some((name, version)) => {
            let version = version
                .parse()
                .map_err(|_| bad_request(format!("bad version in '{reference}'")))?;
            Ok((name.to_string(), Some(version)))
        }
        None => Ok((reference.to_string(), None)),
    }
}

#[derive(Deserialize)]
struct VerifyRequest {
    schema: String,
    /// Already-parsed JSON — exactly one of `value` / `text`.
    #[serde(default)]
    value: Option<Value>,
    /// Raw model output; goes through intake (docs/03 §1).
    #[serde(default)]
    text: Option<String>,
    #[serde(default = "default_env")]
    env: Value,
    #[serde(default)]
    options: VerifyOptions,
}

fn default_env() -> Value {
    json!({})
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct VerifyOptions {
    mode: Option<Mode>,
    fail_fast: bool,
    wait_ms: Option<u64>,
    max_depth: Option<u32>,
    max_samples: Option<u32>,
    deadline_ms: Option<u64>,
}

enum OwnedInput {
    Text(String),
    Value(Value),
}

async fn verify(
    State(state): State<Arc<AppState>>,
    Path(pool): Path<String>,
    Json(request): Json<VerifyRequest>,
) -> Result<Response, ApiError> {
    let pool_config = state
        .catalog
        .load_pool(&pool)
        .map_err(internal)?
        .ok_or_else(|| not_found(format!("pool '{pool}' not found")))?;
    let (schema_name, requested_version) = parse_schema_ref(&request.schema)?;
    let (version, schema_value) = state
        .catalog
        .load_schema(&pool, &schema_name, requested_version)
        .map_err(|e| not_found(e.to_string()))?;
    let node = Node::from_value(schema_value)
        .map_err(|e| internal(format!("registered schema no longer parses: {e}")))?;

    // Request budgets are clamped to pool budgets before the engine sees them.
    let budget = pool_config.budget;
    let options = Options {
        mode: request.options.mode.unwrap_or_default(),
        fail_fast: request.options.fail_fast,
        max_depth: request
            .options
            .max_depth
            .unwrap_or(budget.max_depth)
            .min(budget.max_depth),
        max_samples: request
            .options
            .max_samples
            .unwrap_or(budget.max_samples_per_request)
            .min(budget.max_samples_per_request),
        deadline_ms: request.options.deadline_ms,
        schema_name: Some(format!("{schema_name}@{version}")),
    };
    let input = match (request.value, request.text) {
        (Some(value), None) => OwnedInput::Value(value),
        (None, Some(text)) => OwnedInput::Text(text),
        _ => {
            return Err(bad_request(
                "provide exactly one of 'value' (parsed JSON) or 'text' (raw model output)",
            ))
        }
    };
    let wait_ms = request.options.wait_ms.unwrap_or(2_000);
    let env = request.env;

    let (job, id) = Job::new(pool.clone());
    state.jobs.write().await.insert(id, job.clone());
    let semaphore = state
        .pool_semaphore(&pool, budget.max_concurrency)
        .await;
    let registry = state.effective_registry(&pool).await;
    let cache = state.cache.clone();
    let pool_settings = PoolSettings {
        server: state.server_settings.clone(),
        pool: pool_config.settings.clone(),
    };
    let monitor = PoolMonitor {
        pool: pool.clone(),
        stats: state.stats.clone(),
    };
    let task_job = job.clone();
    let handle = tokio::spawn(async move {
        let _permit = semaphore.acquire_owned().await.ok();
        let runtime = Runtime {
            registry: &registry,
            cache: &*cache,
            settings: &pool_settings,
            monitor: Some(&monitor),
        };
        let report = match &input {
            OwnedInput::Text(text) => {
                selta_core::verify(&node, Input::Text(text), &env, &options, &runtime).await
            }
            OwnedInput::Value(value) => {
                selta_core::verify(&node, Input::Value(value.clone()), &env, &options, &runtime)
                    .await
            }
        };
        *task_job.report.write().await = Some(report);
        let _ = task_job.done.send(true);
    });
    *job.handle.lock().await = Some(handle);

    let mut done = job.done.subscribe();
    let settled = tokio::time::timeout(Duration::from_millis(wait_ms), async {
        while !*done.borrow() {
            if done.changed().await.is_err() {
                break;
            }
        }
    })
    .await;
    if settled.is_ok() {
        if let Some(report) = job.report.read().await.clone() {
            return Ok(Json(report).into_response());
        }
    }
    Ok((StatusCode::ACCEPTED, Json(json!({ "job": id }))).into_response())
}

async fn lookup_job(state: &AppState, pool: &str, id: Uuid) -> Result<Arc<Job>, ApiError> {
    let job = state
        .jobs
        .read()
        .await
        .get(&id)
        .cloned()
        .ok_or_else(|| not_found(format!("job {id} not found")))?;
    if job.pool != pool {
        return Err(not_found(format!("job {id} not found in pool '{pool}'")));
    }
    Ok(job)
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    Path((pool, id)): Path<(String, Uuid)>,
) -> Result<Response, ApiError> {
    let job = lookup_job(&state, &pool, id).await?;
    if let Some(report) = job.report.read().await.clone() {
        return Ok(Json(report).into_response());
    }
    let status = if job.canceled.load(Ordering::Relaxed) {
        "canceled"
    } else {
        "running"
    };
    Ok((StatusCode::ACCEPTED, Json(json!({ "status": status }))).into_response())
}

async fn cancel_job(
    State(state): State<Arc<AppState>>,
    Path((pool, id)): Path<(String, Uuid)>,
) -> Result<Response, ApiError> {
    let job = lookup_job(&state, &pool, id).await?;
    if job.report.read().await.is_some() {
        return Ok(Json(json!({ "status": "done" })).into_response());
    }
    if let Some(handle) = job.handle.lock().await.take() {
        handle.abort();
    }
    job.canceled.store(true, Ordering::Relaxed);
    let _ = job.done.send(true);
    Ok(Json(json!({ "status": "canceled" })).into_response())
}

async fn job_events(
    State(state): State<Arc<AppState>>,
    Path((pool, id)): Path<(String, Uuid)>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let job = lookup_job(&state, &pool, id).await?;
    let stream = futures::stream::once(async move {
        let mut done = job.done.subscribe();
        while !*done.borrow() {
            if done.changed().await.is_err() {
                break;
            }
        }
        let event = match job.report.read().await.clone() {
            Some(report) => Event::default()
                .event("report")
                .json_data(&report)
                .unwrap_or_else(|_| Event::default().event("error").data("serialization failed")),
            None => Event::default().event("canceled").data("{}"),
        };
        Ok::<Event, Infallible>(event)
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
