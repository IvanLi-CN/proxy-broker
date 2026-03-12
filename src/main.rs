use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use clap::{ArgAction, Parser};
use proxy_broker::{
    AppState, BrokerService, BrokerServiceOptions, BrokerStore, ManagedMihomoRuntime, MemoryStore,
    MihomoRuntime, MihomoRuntimeOptions, SqliteStore, build_router,
};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Parser)]
#[command(author, version, about = "proxy-broker REST service")]
struct Cli {
    #[arg(long, default_value = proxy_broker::constants::DEFAULT_SERVICE_ADDR)]
    listen: String,

    #[arg(long, default_value = "sqlite")]
    store: String,

    #[arg(long, default_value = ".proxy-broker/state.sqlite")]
    sqlite_path: PathBuf,

    #[arg(long)]
    mihomo_binary: Option<PathBuf>,

    #[arg(
        long,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true",
        default_value_t = true
    )]
    mihomo_auto_download: bool,

    #[arg(long, default_value = ".proxy-broker/runtime")]
    runtime_dir: PathBuf,

    #[arg(long, default_value = ".proxy-broker/data")]
    data_dir: PathBuf,

    #[arg(long, default_value_t = proxy_broker::constants::DEFAULT_PROBE_CONCURRENCY)]
    probe_concurrency: usize,

    #[arg(
        long,
        default_value_t = proxy_broker::constants::DEFAULT_GEO_ONLINE_CONCURRENCY
    )]
    geo_online_concurrency: usize,

    #[arg(long, default_value = proxy_broker::constants::DEFAULT_ONLINE_GEO_BASE)]
    online_geo_base: String,

    #[arg(long, default_value = proxy_broker::constants::DEFAULT_MMDB_URL)]
    mmdb_url: String,

    #[arg(long, default_value_t = 15)]
    startup_timeout_sec: u64,

    #[arg(long)]
    mihomo_secret: Option<String>,

    #[arg(long)]
    log_json: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if args.log_json {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
    }

    let store: Arc<dyn BrokerStore> =
        match args.store.as_str() {
            "memory" => Arc::new(MemoryStore::new()),
            "sqlite" => Arc::new(SqliteStore::open(&args.sqlite_path).await.with_context(
                || {
                    format!(
                        "failed to initialize sqlite: {}",
                        args.sqlite_path.display()
                    )
                },
            )?),
            other => anyhow::bail!("unsupported --store value: {other} (expected memory|sqlite)"),
        };

    let runtime_opts = MihomoRuntimeOptions {
        binary_path: args.mihomo_binary,
        auto_download: args.mihomo_auto_download,
        work_dir: args.runtime_dir,
        startup_timeout_sec: args.startup_timeout_sec,
        secret: args.mihomo_secret,
    };
    let runtime: Arc<dyn MihomoRuntime> = Arc::new(ManagedMihomoRuntime::new(runtime_opts));

    let service_opts = BrokerServiceOptions {
        data_dir: args.data_dir,
        probe_concurrency: args.probe_concurrency.max(1),
        geo_online_concurrency: args.geo_online_concurrency.max(1),
        online_geo_base: args.online_geo_base,
        mmdb_url: args.mmdb_url,
        ..BrokerServiceOptions::default()
    };
    let service = Arc::new(BrokerService::new(store, runtime, service_opts));
    service
        .reconcile_startup_sessions()
        .await
        .context("failed to reconcile startup sessions")?;

    let app = build_router(AppState { service });
    let listener = tokio::net::TcpListener::bind(&args.listen)
        .await
        .with_context(|| format!("failed to bind listen address: {}", args.listen))?;

    tracing::info!(listen = %args.listen, "proxy-broker service started");
    axum::serve(listener, app)
        .await
        .context("axum server stopped with error")?;

    Ok(())
}
