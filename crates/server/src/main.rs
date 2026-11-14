use std::{env, error::Error, future::Future, io, net::SocketAddr, time::Duration};

use axum::{Router, extract::State, http::StatusCode, routing::get};
use rustsurvey_migration::{Migrator, MigratorTrait};
use rustsurvey_server::settings::Settings;
use rustsurvey_server::storage::StorageConfiguration;
use rustsurvey_server::workspace::PasswordPolicy;
use sea_orm::{Database, DatabaseConnection};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().init();
    let settings = Settings::load(".env")?;
    let web = rustsurvey_server::web::configured_assets(&settings).await?;
    let storage = StorageConfiguration::read(&settings)?;
    let password_policy = PasswordPolicy::from_development_flag(
        settings.boolean("DEV_ALLOW_WEAK_PASSWORDS", false)?,
    )?;
    let address: SocketAddr = settings
        .value_or("BIND_ADDRESS", "127.0.0.1:3000")?
        .parse()
        .map_err(|_| io::Error::other("BIND_ADDRESS must be an IP address and port"))?;
    let automatic_port = cfg!(debug_assertions) && settings.get("BIND_ADDRESS")?.is_none();
    let database = connect_database(&storage).await?;
    Migrator::up(&database, None)
        .await
        .map_err(|_| io::Error::other("database migration failed; startup stopped"))?;
    let secure_cookie = settings.boolean("ADMIN_COOKIE_SECURE", !cfg!(debug_assertions))?;
    let objects = storage.object_store(database.clone()).await?;
    let write_queue_capacity = settings
        .value_or("COLLECTION_WRITE_QUEUE_CAPACITY", "8")?
        .parse::<usize>()
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "COLLECTION_WRITE_QUEUE_CAPACITY must be an integer between 0 and 32",
            )
        })?;
    let workspace =
        rustsurvey_server::workspace::WorkspaceState::new(database.clone(), secure_cookie)
            .with_participant_write_queue_capacity(write_queue_capacity)?
            .with_object_store(objects);
    if let Some(password) = settings.get("ADMIN_PASSWORD")? {
        let username = settings.value_or("ADMIN_USERNAME", "admin")?;
        rustsurvey_server::workspace::bootstrap_admin(
            &workspace,
            &username,
            &password,
            password_policy,
        )
        .await?;
    }
    let mut router = Router::new()
        .route("/api/health", get(health))
        .with_state(database.clone())
        .merge(rustsurvey_server::inspection::routes())
        .merge(rustsurvey_server::workspace::routes(workspace));
    if let Some(web) = web {
        router = router.merge(web.routes());
    }
    let shutdown = shutdown_signal()?;
    let listener = rustsurvey_server::network::bind_listener(address, automatic_port).await?;
    tracing::info!(address = %listener.local_addr()?, "HTTP server listening");
    if env::var("RUSTSURVEY_DEV_READY").as_deref() == Ok("1") {
        let mut address = listener.local_addr()?;
        if address.ip().is_unspecified() {
            if address.is_ipv4() {
                address.set_ip(std::net::Ipv4Addr::LOCALHOST.into());
            } else {
                address.set_ip(std::net::Ipv6Addr::LOCALHOST.into());
            }
        }
        println!("RUSTSURVEY_DEV_API_URL=http://{address}");
    }
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await?;
    database
        .close()
        .await
        .map_err(|_| io::Error::other("cannot close database"))?;
    Ok(())
}

async fn connect_database(storage: &StorageConfiguration) -> io::Result<DatabaseConnection> {
    let url = storage.database_url(std::path::Path::new(".")).await?;
    let options = rustsurvey_server::database::options(url);
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
