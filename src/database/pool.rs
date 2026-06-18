use std::time::Duration;

use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

pub async fn connect(database_url: &str) -> Result<MySqlPool, sqlx::Error> {
    MySqlPoolOptions::new()
        .max_connections(30)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
}
