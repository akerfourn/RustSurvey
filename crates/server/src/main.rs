use std::{env, error::Error, future::Future, io, net::SocketAddr, time::Duration};

use axum::{Router, extract::State, http::StatusCode, routing::get};
use rustsurvey_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().init();
    let address: SocketAddr = setting("BIND_ADDRESS", "127.0.0.1:3000")?
        .parse()
        .map_err(|_| io::Error::other("BIND_ADDRESS must be an IP address and port"))?;
    let database = connect_database().await?;
    Migrator::up(&database, None)
        .await
        .map_err(|_| io::Error::other("database migration failed; startup stopped"))?;
    let router = Router::new()
        .route("/api/health", get(health))
        .with_state(database.clone());
    let shutdown = shutdown_signal()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(address = %listener.local_addr()?, "HTTP server listening");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await?;
    database
        .close()
        .await
        .map_err(|_| io::Error::other("cannot close database"))?;
    Ok(())
}

async fn connect_database() -> io::Result<DatabaseConnection> {
    let url = setting("DATABASE_URL", "sqlite://rustsurvey.db?mode=rwc")?;
    let mut options = ConnectOptions::new(url);
    options
        .min_connections(0)
        .max_connections(4)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .sqlx_logging(false);
    Database::connect(options).await.map_err(|_| {
        io::Error::other("database connection failed; check DATABASE_URL and compiled drivers")
    })
}

async fn health(State(database): State<DatabaseConnection>) -> (StatusCode, &'static str) {
    match tokio::time::timeout(Duration::from_secs(2), database.ping()).await {
        Ok(Ok(())) => (StatusCode::OK, "ok"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    }
}

fn setting(name: &str, default: &str) -> io::Result<String> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_owned()),
        Err(env::VarError::NotUnicode(_)) => {
            Err(io::Error::other(format!("{name} must contain valid UTF-8")))
        }
    }
}

fn shutdown_signal() -> io::Result<impl Future<Output = ()>> {
    #[cfg(unix)]
    let mut signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    #[cfg(unix)]
    let termination = async move {
        signal.recv().await;
    };
    #[cfg(not(unix))]
    let termination = std::future::pending::<()>();

    Ok(async move {
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if result.is_err() {
                    tracing::error!("cannot listen for Ctrl+C; shutting down");
                }
            }
            () = termination => {}
        }
        tracing::info!("shutting down");
    })
}
