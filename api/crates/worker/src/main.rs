//! `reelearn-worker`: job loops + scheduler. `reelearn-worker drain` runs due jobs once and exits.

use reelearn_api::{AppState, Config};
use reelearn_worker::providers::Providers;
use reelearn_worker::{Ctx, RunOptions};
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .init();

    let config = Config::from_env()?;
    let use_real = std::env::var("PROVIDERS")
        .map(|v| v == "real")
        .unwrap_or(!config.app_env.is_development());
    let state = AppState::connect(config).await?;
    // Real mode resolves each company's keys per job (its own, else the platform's, else the
    // environment); development runs every company on the deterministic fakes.
    let ctx = if use_real {
        tracing::info!("worker providers: per company (company key, platform key, environment)");
        Arc::new(Ctx::per_company(state))
    } else {
        let providers = Providers::fake();
        tracing::info!(providers = %providers.summary(), "worker providers");
        Arc::new(Ctx::new(state, providers))
    };

    let mut opts = RunOptions::default();
    if let Ok(n) = std::env::var("WORKER_CONCURRENCY") {
        opts.concurrency = n.parse().unwrap_or(opts.concurrency);
    }
    opts.run_scheduler = std::env::var("WORKER_SCHEDULER")
        .map(|v| v != "false")
        .unwrap_or(true);

    if std::env::args().nth(1).as_deref() == Some("drain") {
        let n = reelearn_worker::drain(&ctx, &opts.worker_id, 10_000).await?;
        println!("ran {n} jobs");
        return Ok(());
    }

    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutting down worker");
        let _ = stop_tx.send(true);
    });
    reelearn_worker::run(ctx, opts, stop_rx).await;
    Ok(())
}
