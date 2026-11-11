//! `reelearn-api`: HTTP server. `reelearn-api seed` loads the demo tenant, `reelearn-api migrate` only migrates,
//! `reelearn-api grant-platform-operator <email>` lets someone run the platform area.

use reelearn_api::{build_router, AppState, Config};
use std::net::SocketAddr;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn"));
    let json = std::env::var("LOG_FORMAT")
        .map(|v| v == "json")
        .unwrap_or(false);
    let registry = tracing_subscriber::registry().with(filter);
    if json {
        registry.with(fmt::layer().json()).init();
    } else {
        registry.with(fmt::layer()).init();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    init_tracing();
    let config = Config::from_env()?;
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("seed") => {
            let state = AppState::connect(config).await?;
            let summary = reelearn_api::seed::seed_demo(&state).await?;
            println!("{summary}");
            return Ok(());
        }
        Some("grant-platform-operator") => {
            // `reelearn-api grant-platform-operator <email>`: the people who run the platform.
            let email = args
                .get(2)
                .ok_or("usage: reelearn-api grant-platform-operator <email>")?;
            let state = AppState::connect(config).await?;
            let mut tx = state.db.admin_tx().await?;
            let user = reelearn_db::repos::users::find_by_email(&mut tx, email)
                .await?
                .ok_or("no user with that email")?;
            reelearn_db::repos::ai::grant_platform_operator_admin(&mut tx, user.id).await?;
            tx.commit().await?;
            println!("{email} can now open the platform area");
            return Ok(());
        }
        Some("migrate") => {
            let state = AppState::connect(config).await?;
            tracing::info!(connected = state.db.ping().await, "migrations applied");
            return Ok(());
        }
        _ => {}
    }

    let port = config.port;
    let state = AppState::connect(config).await?;
    let app = build_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "reelearn-api listening (Swagger UI at /docs)");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("shutting down");
}
