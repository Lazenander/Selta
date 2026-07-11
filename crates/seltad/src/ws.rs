//! Pool hosts (docs/05 §websocket): an application runs its verifier
//! wherever and however it likes and dials in to its pool. Transport
//! direction is independent of protocol direction — seltad is the JSON-RPC
//! client and sends `initialize` first. While a pool host is disconnected,
//! its checks are errors (→ inconclusive), never failures.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use selta_core::{
    initialize_over_peer, verify_over_peer, Envelope, ExtensionDecl, ExtensionHost, HostCall,
    RpcPeer,
};
use uuid::Uuid;

use crate::state::AppState;

pub struct WsHost {
    peer: Arc<RpcPeer>,
}

#[async_trait]
impl ExtensionHost for WsHost {
    async fn verify(&self, call: HostCall<'_>) -> Result<Envelope, String> {
        verify_over_peer(&self.peer, call).await
    }
}

#[derive(Clone)]
pub struct PoolHostEntry {
    pub decl: Arc<ExtensionDecl>,
    pub host: Arc<WsHost>,
    connection: Uuid,
}

/// Extension name → its dialed-in provider, per pool.
pub type PoolHosts = HashMap<String, PoolHostEntry>;

pub async fn handle_socket(state: Arc<AppState>, pool: String, socket: WebSocket) {
    let connection = Uuid::new_v4();
    let (mut sink, mut stream) = socket.split();
    let (peer, mut outbox) = RpcPeer::new();

    let mut writer = tokio::spawn(async move {
        while let Some(line) = outbox.recv().await {
            if sink.send(Message::Text(line.into())).await.is_err() {
                break;
            }
        }
    });
    let reader_peer = peer.clone();
    let mut reader = tokio::spawn(async move {
        while let Some(Ok(message)) = stream.next().await {
            match message {
                Message::Text(text) => reader_peer.accept_line(&text),
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    match register(&state, &pool, &peer, connection).await {
        Ok(names) => {
            eprintln!(
                "seltad: pool '{pool}' host connected ({})",
                names.join(", ")
            );
        }
        Err(error) => {
            eprintln!("seltad: pool '{pool}' host rejected: {error}");
            reader.abort();
            writer.abort();
            peer.fail_all("pool host registration rejected");
            return;
        }
    }

    // Either transport half ending makes the host unavailable. In particular,
    // a failed writer must not leave declarations advertised until the reader
    // happens to notice the dead connection.
    tokio::select! {
        _ = &mut reader => writer.abort(),
        _ = &mut writer => reader.abort(),
    }
    peer.fail_all("pool host disconnected");
    let mut hosts = state.pool_hosts.write().await;
    if let Some(entries) = hosts.get_mut(&pool) {
        entries.retain(|_, entry| entry.connection != connection);
        if entries.is_empty() {
            hosts.remove(&pool);
        }
    }
    eprintln!("seltad: pool '{pool}' host disconnected");
}

async fn register(
    state: &Arc<AppState>,
    pool: &str,
    peer: &Arc<RpcPeer>,
    connection: Uuid,
) -> Result<Vec<String>, String> {
    let decls = initialize_over_peer(peer, "seltad").await?;
    if decls.is_empty() {
        return Err("host declared no extensions".to_string());
    }
    let host = Arc::new(WsHost { peer: peer.clone() });
    let mut hosts = state.pool_hosts.write().await;
    let entries = hosts.entry(pool.to_string()).or_default();

    // Route the complete declaration batch through the same prospective,
    // atomic registry validation as stdio hosts. This catches same-batch
    // duplicates and declaration-schema faults before any entry is visible.
    let mut prospective = (*state.registry).clone();
    for entry in entries.values() {
        let existing_host: Arc<dyn ExtensionHost> = entry.host.clone();
        prospective.register(vec![(*entry.decl).clone()], existing_host)?;
    }
    let prospective_host: Arc<dyn ExtensionHost> = host.clone();
    prospective.register(decls.clone(), prospective_host)?;

    let mut names = Vec::new();
    for decl in decls {
        names.push(decl.name.clone());
        entries.insert(
            decl.name.clone(),
            PoolHostEntry {
                decl: Arc::new(decl),
                host: host.clone(),
                connection,
            },
        );
    }
    Ok(names)
}
