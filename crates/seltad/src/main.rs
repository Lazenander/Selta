//! seltad: the Selta server (docs/06). A shell around selta-core that adds
//! the catalog, pools, jobs, settings, stats, and host supervision — engine
//! semantics are byte-identical embedded and served.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use selta_core::{CmdTemplates, MemoryCache, Registry, RpcHost};
use tokio::sync::RwLock;

use seltad::api;
use seltad::config::{Config, ListenEndpoint};
use seltad::state::AppState;
use seltad::stats::Stats;
use seltad::storage::Storage;

/// How often recorded counters reach storage; one final flush runs on
/// graceful shutdown.
const STATS_FLUSH_INTERVAL: Duration = Duration::from_secs(5);

fn flush_stats(catalog: &dyn Storage, stats: &Stats) {
    if stats.take_dirty() {
        if let Err(error) = catalog.save_stats(&stats.snapshot()) {
            eprintln!("seltad: stats flush failed: {error}");
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("sigterm handler installs");
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "selta.toml".to_string());
    let config = Config::load(Path::new(&config_path))?;
    let endpoint = config.listen_endpoint()?;
    endpoint.ensure_supported()?;
    config.storage.ensure_supported()?;

    let templates: Option<Arc<dyn CmdTemplates>> =
        (!config.cmd.is_empty()).then(|| Arc::new(config.cmd.clone()) as Arc<dyn CmdTemplates>);
    let mut registry = Registry::with_builtins(templates);
    let mut spawned_hosts = Vec::new();
    for (name, host_config) in &config.hosts {
        let (decls, host) = RpcHost::spawn(&host_config.run, "seltad")
            .await
            .map_err(|e| anyhow::anyhow!("host '{name}': {e}"))?;
        let extensions: Vec<String> = decls.iter().map(|d| d.name.clone()).collect();
        registry
            .register(decls, host.clone())
            .map_err(|e| anyhow::anyhow!("host '{name}': {e}"))?;
        spawned_hosts.push(host);
        eprintln!("seltad: host '{name}' up ({})", extensions.join(", "));
    }

    let catalog = seltad::storage::open(config.storage, &config.data)
        .with_context(|| format!("opening catalog at {}", config.data.display()))?;
    let stats = Arc::new(Stats::default());
    stats.restore(catalog.load_stats().context("restoring stats")?);
    let state = Arc::new(AppState {
        registry: Arc::new(registry),
        catalog: catalog.clone(),
        cache: Arc::new(MemoryCache::default()),
        server_settings: Arc::new(config.server_settings()),
        stats: stats.clone(),
        semaphores: RwLock::new(HashMap::new()),
        jobs: RwLock::new(HashMap::new()),
        pool_hosts: RwLock::new(HashMap::new()),
    });
    let app = api::router(state);

    let flusher = {
        let (catalog, stats) = (catalog.clone(), stats.clone());
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(STATS_FLUSH_INTERVAL);
            loop {
                tick.tick().await;
                flush_stats(&*catalog, &stats);
            }
        })
    };

    match endpoint {
        ListenEndpoint::Tcp(address) => {
            let listener = tokio::net::TcpListener::bind(&address)
                .await
                .with_context(|| format!("binding {address}"))?;
            let bound = listener
                .local_addr()
                .context("reading TCP listener address")?;
            eprintln!("seltad: listening on http://{bound}");
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }
        ListenEndpoint::Unix(socket_path) => {
            #[cfg(unix)]
            {
                let _ = std::fs::remove_file(&socket_path);
                let listener = tokio::net::UnixListener::bind(&socket_path)
                    .with_context(|| format!("binding unix:{}", socket_path.display()))?;
                eprintln!("seltad: listening on unix:{}", socket_path.display());
                axum::serve(listener, app)
                    .with_graceful_shutdown(shutdown_signal())
                    .await?;
            }
            #[cfg(not(unix))]
            {
                anyhow::bail!(
                    "unix listener endpoint '{}' is not supported on this platform",
                    socket_path.display()
                );
            }
        }
    }
    flusher.abort();
    flush_stats(&*catalog, &stats);
    for host in spawned_hosts {
        host.shutdown().await;
    }
    Ok(())
}
