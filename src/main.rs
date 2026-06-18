use palpites_back::infrastructure::jobs::scheduler;
use palpites_back::infrastructure::storage::FileStorageSettings;
use palpites_back::{api, config::AppConfig, database};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dotenv_result = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    match dotenv_result {
        Ok(path) => info!("loaded environment from {}", path.display()),
        Err(err) if err.not_found() => {}
        Err(err) => warn!("failed to load .env file: {err}"),
    }

    let config = AppConfig::from_env();
    let db = if let Some(database_url) = &config.database_url {
        let pool = database::pool::connect(database_url).await?;
        if config.run_migrations {
            info!("running database migrations");
            sqlx::migrate!("./migrations").run(&pool).await?;
        }
        Some(pool)
    } else {
        warn!("DATABASE_URL not configured; API endpoints that require persistence will return configuration errors");
        None
    };

    if let Some(pool) = &db {
        scheduler::spawn(pool.clone(), &config);
    }

    let file_storage = FileStorageSettings {
        backend: config.files_backend.clone(),
        bucket: config.files_bucket.clone(),
        local_dir: config.files_local_dir.clone(),
        public_base_url: config.files_public_base_url.clone(),
        max_byte_size: config.files_max_byte_size,
        s3_region: config.s3_region.clone(),
        s3_endpoint: config.s3_endpoint.clone(),
    };

    let app = api::router(api::state::AppState::new(
        db,
        config.jwt_secret.clone(),
        file_storage,
    ));
    let listener = TcpListener::bind(config.http_addr()).await?;

    info!("backend listening on {}", config.http_addr());
    axum::serve(listener, app).await?;

    Ok(())
}
