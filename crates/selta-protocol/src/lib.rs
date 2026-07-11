//! Wire types for the extension-host protocol (docs/05): JSON-RPC 2.0, one
//! message per line over the host process's stdio. This crate is types only —
//! the boundary is the protocol, never a language.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 1;

pub const CODE_UNKNOWN: i64 = -32601;
pub const CODE_INVALID_PARAMS: i64 = -32602;
pub const CODE_VERIFIER_FAILED: i64 = -32000;
pub const CODE_BUDGET_DECLINED: i64 = -32001;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    /// Absent for notifications (`cancel`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Request {
    pub fn new(id: u64, method: &str, params: Value) -> Self {
        Request {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: method.to_string(),
            params,
        }
    }

    pub fn notification(method: &str, params: Value) -> Self {
        Request {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol: u32,
    pub server: PeerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub host: PeerInfo,
    pub extensions: Vec<ExtensionManifest>,
}

/// The host's registration (docs/05 §initialize). `determinism` is
/// load-bearing: the engine decides run-once vs sample-and-vote from it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub name: String,
    pub determinism: String,
    /// Stable verifier-semantics identity. Optional for protocol compatibility;
    /// a declaration without one is treated as unversioned and non-cacheable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_revision: Option<String>,
    /// Opt-in only. Older hosts omit this field and safely default to false.
    #[serde(default)]
    pub cacheable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<Value>,
    #[serde(default)]
    pub needs: Vec<String>,
    /// A Selta schema for the extension's operational settings (docs/05
    /// §Three kinds of configuration). Values live in the server catalog,
    /// never in schemas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings_schema: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyParams {
    pub ext: String,
    pub config: Value,
    /// Resolved server ⊕ pool settings, secrets already injected.
    #[serde(default = "empty_object")]
    pub settings: Value,
    pub value: Value,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<Value>,
    pub budget: Budget,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Budget {
    pub depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CancelParams {
    pub id: u64,
}

fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}
