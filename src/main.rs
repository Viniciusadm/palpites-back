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
        if let Err(error) = rescore_penalty_winner_no_draw(pool).await {
            warn!("penalties_winner_no_draw rescore failed: {error}");
        }
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

/// One-time backfill: re-scores every pool that has a finished penalty shootout
/// so the new `penalties_winner_no_draw` category is reflected in the cached
/// `predictions.points_awarded` and `pool_standings`. Reuses the real scoring
/// engine (`recompute_pool`, idempotent) instead of duplicating logic in SQL,
/// and runs exactly once (guarded by the `applied_backfills` marker).
async fn rescore_penalty_winner_no_draw(
    pool: &sqlx::MySqlPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use sqlx::Row;

    const MARKER: &str = "penalties_winner_no_draw_v2";

    let already = sqlx::query("SELECT 1 FROM applied_backfills WHERE name = ? LIMIT 1")
        .bind(MARKER)
        .fetch_optional(pool)
        .await?;
    if already.is_some() {
        return Ok(());
    }

    let pool_ids: Vec<String> = sqlx::query(
        "SELECT DISTINCT pm.pool_id AS pool_id \
         FROM matches m \
         JOIN predictions p ON p.match_id = m.id \
         JOIN pool_members pm ON pm.id = p.pool_member_id \
         WHERE m.penalties_winner IS NOT NULL",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.get::<String, _>("pool_id"))
    .collect();

    let use_cases =
        palpites_back::api::routes::v1::results::result_use_cases(pool.clone());

    let mut succeeded = 0usize;
    let mut failed = 0usize;
    for pool_id in &pool_ids {
        match use_cases.recompute_pool(pool_id).await {
            Ok(()) => succeeded += 1,
            Err(error) => {
                failed += 1;
                warn!(%pool_id, "rescore failed for pool: {error}");
            }
        }
    }

    // Only claim the marker when every affected pool was re-scored; otherwise let
    // the next startup retry (recompute_pool is idempotent).
    if failed == 0 {
        sqlx::query("INSERT INTO applied_backfills (name) VALUES (?)")
            .bind(MARKER)
            .execute(pool)
            .await?;
    }

    info!(
        pools_rescored = succeeded,
        pools_failed = failed,
        "penalties_winner_no_draw rescore complete"
    );
    Ok(())
}
