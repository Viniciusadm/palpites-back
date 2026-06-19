use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::notifications::{DeviceTokenRepository, NewDeviceToken};
use crate::errors::AppError;

#[derive(Clone)]
pub struct MySqlDeviceTokenRepository {
    pool: MySqlPool,
}

impl MySqlDeviceTokenRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeviceTokenRepository for MySqlDeviceTokenRepository {
    async fn upsert(&self, record: NewDeviceToken) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO device_tokens (id, user_id, token, platform) \
             VALUES (?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
                 user_id = VALUES(user_id), \
                 platform = VALUES(platform), \
                 last_seen_at = CURRENT_TIMESTAMP(6)",
        )
        .bind(&record.id)
        .bind(&record.user_id)
        .bind(&record.token)
        .bind(&record.platform)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query("SELECT token FROM device_tokens WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| Ok(row.try_get("token")?))
            .collect()
    }

    async fn remove(&self, token: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM device_tokens WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
