//! JSON-RPC transport for extension hosts (docs/05): one message per line.
//! `RpcPeer` is the transport-agnostic half — request/response bookkeeping,
//! timeouts, cancel — and `RpcHost` drives it over a child process's stdio.
//! Pool hosts drive the same peer over a websocket; whoever opens the
//! connection, seltad is the JSON-RPC client.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};

use selta_protocol as proto;

use super::{
    Determinism, EffectClass, Envelope, ExtensionDecl, ExtensionHost, HostCall, InputDomain,
    InputKind, Needs,
};
use crate::{AdmittedNode, MetaIssue, Node};

const DEFAULT_CALL_TIMEOUT_MS: u64 = 30_000;
const INITIALIZE_TIMEOUT_MS: u64 = 10_000;

type Pending = Mutex<HashMap<u64, oneshot::Sender<Result<Box<RawValue>, String>>>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<Box<RawValue>>,
    #[serde(default)]
    error: Option<proto::RpcError>,
}

/// Cancellation-safe ownership of one pending call. A dropped call future is
/// indistinguishable from an explicit timeout to the transport: remove its
/// sender and emit best-effort cancellation. If a response or `fail_all`
/// already removed the entry, dropping the guard is a no-op.
struct PendingCall<'a> {
    peer: &'a RpcPeer,
    id: u64,
}

impl Drop for PendingCall<'_> {
    fn drop(&mut self) {
        let removed = self.peer.pending.lock().unwrap().remove(&self.id);
        if removed.is_some() {
            self.peer.notify(
                "cancel",
                serde_json::to_value(proto::CancelParams { id: self.id })
                    .expect("cancel params serialize"),
            );
        }
    }
}

/// The client side of one JSON-RPC connection, independent of transport.
/// The caller pumps incoming lines into `accept_line` and drains the
/// returned receiver into whatever carries outgoing lines.
pub struct RpcPeer {
    outgoing: mpsc::UnboundedSender<String>,
    pending: Pending,
    next_id: AtomicU64,
}

impl RpcPeer {
    pub fn new() -> (Arc<RpcPeer>, mpsc::UnboundedReceiver<String>) {
        let (outgoing, outbox) = mpsc::unbounded_channel();
        (
            Arc::new(RpcPeer {
                outgoing,
                pending: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
            }),
            outbox,
        )
    }

    /// Route one incoming line to its waiting call; unparseable lines are
    /// ignored (they are not ours).
    pub fn accept_line(&self, line: &str) {
        let Ok(response) = serde_json::from_str::<RawResponse>(line) else {
            return;
        };
        if response.jsonrpc != "2.0" {
            return;
        }
        let sender = self.pending.lock().unwrap().remove(&response.id);
        if let Some(sender) = sender {
            let outcome = match (response.result, response.error) {
                (Some(result), None) => Ok(result),
                (None, Some(error)) => Err(format!("host error {}: {}", error.code, error.message)),
                (Some(_), Some(_)) => Err("host response had both result and error".to_string()),
                (None, None) => Err("host response had neither result nor error".to_string()),
            };
            let _ = sender.send(outcome);
        }
    }

    /// Fail everything still in flight — an error, never a vote.
    pub fn fail_all(&self, reason: &str) {
        let mut map = self.pending.lock().unwrap();
        for (_, sender) in map.drain() {
            let _ = sender.send(Err(reason.to_string()));
        }
    }

    pub async fn call(
        &self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        let raw = self.call_raw(method, params, timeout_ms).await?;
        serde_json::from_str(raw.get()).map_err(|error| {
            format!("host result for method '{method}' is not valid JSON: {error}")
        })
    }

    async fn call_raw(
        &self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> Result<Box<RawValue>, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        let line = serde_json::to_string(&proto::Request::new(id, method, params))
            .map_err(|e| e.to_string())?;
        self.pending.lock().unwrap().insert(id, tx);
        if self.outgoing.send(line).is_err() {
            self.pending.lock().unwrap().remove(&id);
            return Err("host is gone".to_string());
        }
        let _pending_call = PendingCall { peer: self, id };
        match tokio::time::timeout(Duration::from_millis(timeout_ms.max(1)), rx).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err("host dropped the call".to_string()),
            Err(_) => Err(format!(
                "host call '{method}' timed out after {timeout_ms} ms"
            )),
        }
    }

    pub fn notify(&self, method: &str, params: Value) {
        if let Ok(line) = serde_json::to_string(&proto::Request::notification(method, params)) {
            let _ = self.outgoing.send(line);
        }
    }
}

/// The initialize handshake: seltad introduces itself, the host answers with
/// its manifest, parsed into declarations.
pub async fn initialize_over_peer(
    peer: &RpcPeer,
    server_name: &str,
) -> Result<Vec<ExtensionDecl>, String> {
    let params = serde_json::to_value(proto::InitializeParams {
        protocol: proto::PROTOCOL_VERSION,
        server: proto::PeerInfo {
            name: server_name.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    })
    .expect("initialize params serialize");
    let init = peer
        .call_raw("initialize", params, INITIALIZE_TIMEOUT_MS)
        .await?;
    let manifest: proto::InitializeResult =
        serde_json::from_str(init.get()).map_err(|e| format!("bad initialize result: {e}"))?;
    manifest
        .extensions
        .into_iter()
        .map(decl_from_manifest)
        .collect()
}

/// One `verify` round-trip over a peer — identical on every transport.
pub async fn verify_over_peer(peer: &RpcPeer, call: HostCall<'_>) -> Result<Envelope, String> {
    let timeout_ms = call.deadline_ms.unwrap_or(DEFAULT_CALL_TIMEOUT_MS);
    let params = serde_json::to_value(proto::VerifyParams {
        ext: call.ext.to_string(),
        config: call.config.clone(),
        settings: call.settings.clone(),
        value: call.value.clone(),
        path: call.path.to_string(),
        root: call.root.cloned(),
        env: call.env.cloned(),
        budget: proto::Budget {
            depth: call.depth,
            deadline_ms: call.deadline_ms,
        },
    })
    .map_err(|e| e.to_string())?;
    let result = peer.call("verify", params, timeout_ms).await?;
    serde_json::from_value(result).map_err(|e| format!("malformed result envelope: {e}"))
}

pub fn decl_from_manifest(manifest: proto::ExtensionManifest) -> Result<ExtensionDecl, String> {
    let determinism = match manifest.determinism.as_str() {
        "deterministic" => Determinism::Deterministic,
        "nondeterministic" => Determinism::Nondeterministic,
        other => {
            return Err(format!(
                "extension '{}': unknown determinism '{other}'",
                manifest.name
            ))
        }
    };
    let semantic_revision = match manifest.semantic_revision.as_deref() {
        Some(revision) if revision.trim().is_empty() => {
            return Err(format!(
                "extension '{}': semantic_revision must not be empty",
                manifest.name
            ));
        }
        Some(revision) => revision.to_string(),
        None if manifest.cacheable => {
            return Err(format!(
                "extension '{}': cacheable extensions require semantic_revision",
                manifest.name
            ));
        }
        None => "selta.extension.unversioned".to_string(),
    };
    if manifest.cacheable && determinism != Determinism::Deterministic {
        return Err(format!(
            "extension '{}': only deterministic extensions may be cacheable",
            manifest.name
        ));
    }
    let effect_class = match manifest.effect_class.as_deref() {
        Some("pure") => EffectClass::Pure,
        Some("process_io") => EffectClass::ProcessIo,
        Some("unknown") | None => EffectClass::Unknown,
        Some(other) => {
            return Err(format!(
                "extension '{}': unknown effect_class '{other}'",
                manifest.name
            ));
        }
    };
    if manifest.cacheable && effect_class != EffectClass::Pure {
        return Err(format!(
            "extension '{}': cacheable extensions must declare effect_class 'pure'",
            manifest.name
        ));
    }
    let accepted_input = match &manifest.accepted_input {
        None => InputDomain::any(),
        Some(names) => {
            let mut kinds = Vec::with_capacity(names.len());
            for name in names {
                let Some(kind) = InputKind::parse(name) else {
                    return Err(format!(
                        "extension '{}': unknown accepted_input kind '{name}'",
                        manifest.name
                    ));
                };
                kinds.push(kind);
            }
            InputDomain::new(kinds)
                .map_err(|error| format!("extension '{}': {error}", manifest.name))?
        }
    };
    let needs = Needs::try_from_list(&manifest.needs)
        .map_err(|error| format!("extension '{}': {error}", manifest.name))?;
    let config_schema = parse_schema(&manifest.name, manifest.config_schema, "config_schema")?;
    let settings_schema =
        parse_schema(&manifest.name, manifest.settings_schema, "settings_schema")?;
    let delta_schema = parse_schema(&manifest.name, manifest.delta_schema, "delta_schema")?;
    Ok(ExtensionDecl {
        semantic_revision,
        cacheable: manifest.cacheable,
        determinism,
        effect_class,
        accepted_input,
        config_schema,
        config_preflight: None,
        needs,
        settings_schema,
        delta_schema,
        name: manifest.name,
    })
}

fn parse_schema(
    name: &str,
    value: Option<Box<RawValue>>,
    what: &str,
) -> Result<Option<Node>, String> {
    match value {
        None => Ok(None),
        Some(raw) if raw.get().trim() == "null" => Ok(None),
        Some(raw) => AdmittedNode::admit_source(raw.get().as_bytes())
            .map(AdmittedNode::into_node)
            .map(Some)
            .map_err(|issues| {
                format!(
                    "extension '{name}': bad {what}: {}",
                    format_meta_issues(&issues)
                )
            }),
    }
}

fn format_meta_issues(issues: &[MetaIssue]) -> String {
    issues
        .iter()
        .map(|issue| format!("{} at {}: {}", issue.code, issue.pointer, issue.detail))
        .collect::<Vec<_>>()
        .join("; ")
}

/// A server host: spawned by seltad, spoken to over stdio (docs/05 §stdio).
pub struct RpcHost {
    peer: Arc<RpcPeer>,
    _child: Mutex<Child>,
}

impl RpcHost {
    /// Spawn a host process, run the initialize handshake, and return its
    /// declared extensions.
    pub async fn spawn(
        command: &[String],
        server_name: &str,
    ) -> Result<(Vec<ExtensionDecl>, Arc<RpcHost>), String> {
        let (program, args) = command.split_first().ok_or("host command is empty")?;
        let mut child = Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("failed to spawn host '{program}': {e}"))?;

        let mut stdin = child.stdin.take().ok_or("host stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("host stdout unavailable")?;

        let (peer, mut outbox) = RpcPeer::new();
        tokio::spawn(async move {
            while let Some(line) = outbox.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err()
                    || stdin.write_all(b"\n").await.is_err()
                    || stdin.flush().await.is_err()
                {
                    break;
                }
            }
        });
        let reader_peer = peer.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                reader_peer.accept_line(&line);
            }
            reader_peer.fail_all("host exited");
        });

        let decls = initialize_over_peer(&peer, server_name).await?;
        let host = Arc::new(RpcHost {
            peer,
            _child: Mutex::new(child),
        });
        Ok((decls, host))
    }

    pub async fn shutdown(&self) {
        let _ = self.peer.call("shutdown", json!({}), 2_000).await;
    }
}

#[async_trait]
impl ExtensionHost for RpcHost {
    async fn verify(&self, call: HostCall<'_>) -> Result<Envelope, String> {
        verify_over_peer(&self.peer, call).await
    }
}
