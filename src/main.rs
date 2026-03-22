use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use clap::{ArgAction, Parser};
use proxy_broker::{
    AppState, AuthConfig, AuthConfigOptions, BrokerService, BrokerServiceOptions, BrokerStore,
    ManagedMihomoRuntime, MemoryStore, MihomoRuntime, MihomoRuntimeOptions, SqliteStore,
    build_router,
};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Parser)]
#[command(
    author,
    version = env!("PROXY_BROKER_BUILD_VERSION"),
    about = "proxy-broker REST service"
)]
struct Cli {
    #[arg(
        long,
        env = "PROXY_BROKER_LISTEN_ADDR",
        default_value = proxy_broker::constants::DEFAULT_SERVICE_ADDR
    )]
    listen: String,

    #[arg(
        long,
        env = "PROXY_BROKER_SESSION_LISTEN_IP",
        default_value = proxy_broker::constants::DEFAULT_SESSION_LISTEN_IP
    )]
    session_listen_ip: IpAddr,

    #[arg(long, env = "PROXY_BROKER_STORE", default_value = "sqlite")]
    store: String,

    #[arg(
        long,
        env = "PROXY_BROKER_SQLITE_PATH",
        default_value = ".proxy-broker/state.sqlite"
    )]
    sqlite_path: PathBuf,

    #[arg(long, env = "PROXY_BROKER_MIHOMO_BINARY")]
    mihomo_binary: Option<PathBuf>,

    #[arg(
        long,
        env = "PROXY_BROKER_MIHOMO_AUTO_DOWNLOAD",
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true",
        default_value_t = true
    )]
    mihomo_auto_download: bool,

    #[arg(
        long,
        env = "PROXY_BROKER_RUNTIME_DIR",
        default_value = ".proxy-broker/runtime"
    )]
    runtime_dir: PathBuf,

    #[arg(
        long,
        env = "PROXY_BROKER_DATA_DIR",
        default_value = ".proxy-broker/data"
    )]
    data_dir: PathBuf,

    #[arg(
        long,
        env = "PROXY_BROKER_PROBE_CONCURRENCY",
        default_value_t = proxy_broker::constants::DEFAULT_PROBE_CONCURRENCY
    )]
    probe_concurrency: usize,

    #[arg(
        long,
        env = "PROXY_BROKER_GEO_ONLINE_CONCURRENCY",
        default_value_t = proxy_broker::constants::DEFAULT_GEO_ONLINE_CONCURRENCY
    )]
    geo_online_concurrency: usize,

    #[arg(
        long,
        env = "PROXY_BROKER_ONLINE_GEO_BASE",
        default_value = proxy_broker::constants::DEFAULT_ONLINE_GEO_BASE
    )]
    online_geo_base: String,

    #[arg(
        long,
        env = "PROXY_BROKER_MMDB_URL",
        default_value = proxy_broker::constants::DEFAULT_MMDB_URL
    )]
    mmdb_url: String,

    #[arg(long, env = "PROXY_BROKER_STARTUP_TIMEOUT_SEC", default_value_t = 15)]
    startup_timeout_sec: u64,

    #[arg(long, env = "PROXY_BROKER_MIHOMO_SECRET")]
    mihomo_secret: Option<String>,

    #[arg(long, env = "PROXY_BROKER_LOG_JSON")]
    log_json: bool,

    #[arg(
        long,
        env = "PROXY_BROKER_AUTH_MODE",
        default_value = proxy_broker::constants::DEFAULT_AUTH_MODE
    )]
    auth_mode: String,

    #[arg(
        long,
        env = "PROXY_BROKER_AUTH_SUBJECT_HEADERS",
        default_value = proxy_broker::constants::DEFAULT_AUTH_SUBJECT_HEADERS
    )]
    auth_subject_headers: String,

    #[arg(
        long,
        env = "PROXY_BROKER_AUTH_EMAIL_HEADERS",
        default_value = proxy_broker::constants::DEFAULT_AUTH_EMAIL_HEADERS
    )]
    auth_email_headers: String,

    #[arg(
        long,
        env = "PROXY_BROKER_AUTH_GROUPS_HEADERS",
        default_value = proxy_broker::constants::DEFAULT_AUTH_GROUPS_HEADERS
    )]
    auth_groups_headers: String,

    #[arg(
        long,
        env = "PROXY_BROKER_AUTH_TRUSTED_PROXIES",
        default_value = proxy_broker::constants::DEFAULT_AUTH_TRUSTED_PROXIES
    )]
    auth_trusted_proxies: String,

    #[arg(long, env = "PROXY_BROKER_AUTH_ADMIN_USERS", default_value = "")]
    auth_admin_users: String,

    #[arg(long, env = "PROXY_BROKER_AUTH_ADMIN_GROUPS", default_value = "")]
    auth_admin_groups: String,

    #[arg(
        long,
        env = "PROXY_BROKER_AUTH_DEV_USER",
        default_value = proxy_broker::constants::DEFAULT_AUTH_DEV_USER
    )]
    auth_dev_user: String,

    #[arg(
        long,
        env = "PROXY_BROKER_AUTH_DEV_EMAIL",
        default_value = proxy_broker::constants::DEFAULT_AUTH_DEV_EMAIL
    )]
    auth_dev_email: String,

    #[arg(
        long,
        env = "PROXY_BROKER_AUTH_DEV_GROUPS",
        default_value = proxy_broker::constants::DEFAULT_AUTH_DEV_GROUPS
    )]
    auth_dev_groups: String,
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
        session_listen_ip: args.session_listen_ip,
        ..BrokerServiceOptions::default()
    };
    let service = Arc::new(BrokerService::new(store, runtime, service_opts));
    service
        .reconcile_startup_sessions()
        .await
        .context("failed to reconcile startup sessions")?;
    service.start_background_workers();

    let auth = AuthConfig::from_options(AuthConfigOptions {
        mode: args.auth_mode,
        subject_headers: args.auth_subject_headers,
        email_headers: args.auth_email_headers,
        groups_headers: args.auth_groups_headers,
        trusted_proxies: args.auth_trusted_proxies,
        admin_users: args.auth_admin_users,
        admin_groups: args.auth_admin_groups,
        dev_user: args.auth_dev_user,
        dev_email: args.auth_dev_email,
        dev_groups: args.auth_dev_groups,
    })?;

    let app = build_router(AppState {
        service,
        auth: Arc::new(auth),
    });
    let listener = tokio::net::TcpListener::bind(&args.listen)
        .await
        .with_context(|| format!("failed to bind listen address: {}", args.listen))?;

    tracing::info!(listen = %args.listen, "proxy-broker service started");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("axum server stopped with error")?;

    Ok(())
}
