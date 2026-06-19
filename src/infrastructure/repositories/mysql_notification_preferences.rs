use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::notifications::{NewPreferenceRecord, NotificationPreferenceRepository};
use crate::database::transaction::DatabaseTransaction;
use crate::domain::notifications::NotificationPreference;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const PREFERENCE_COLUMNS: &str = "SELECT id, user_id, pool_id, type, channel, enabled, \
     created_at, updated_at \
     FROM notification_preferences";

#[derive(Clone)]
pub struct MySqlNotificationPreferenceRepository {
    pool: MySqlPool,
}

impl MySqlNotificationPreferenceRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationPreferenceRepository for MySqlNotificationPreferenceRepository {
    async fn list_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError> {
        let sql = format!("{PREFERENCE_COLUMNS} WHERE user_id = ?");
        let rows = sqlx::query(&sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_preference).collect()
    }

    async fn list_for_scope(
        &self,
        user_id: &str,
        pool_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError> {
        let sql = format!(
            "{PREFERENCE_COLUMNS} WHERE user_id = ? AND pool_id = ? ORDER BY type, channel"
        );
        let rows = sqlx::query(&sql)
            .bind(user_id)
            .bind(pool_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_preference).collect()
    }

    async fn list_for_global(
        &self,
        user_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError> {
        let sql = format!(
            "{PREFERENCE_COLUMNS} WHERE user_id = ? AND pool_id IS NULL ORDER BY type, channel"
        );
        let rows = sqlx::query(&sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_preference).collect()
    }

    async fn upsert_many(&self, records: Vec<NewPreferenceRecord>) -> Result<(), AppError> {
        if records.is_empty() {
            return Ok(());
        }
        let mut tx: DatabaseTransaction = self.pool.begin().await?;
        for record in &records {
            if record.pool_id.is_none() {
                // The unique key treats NULL pool_id as distinct, so ON DUPLICATE
                // KEY UPDATE never fires for global rows. Delete-then-insert keeps
                // a single row per (user_id, type, channel) global preference.
                sqlx::query(
                    "DELETE FROM notification_preferences \
                     WHERE user_id = ? AND pool_id IS NULL AND type = ? AND channel = ?",
                )
                .bind(&record.user_id)
                .bind(&record.notification_type)
                .bind(&record.channel)
                .execute(&mut *tx)
                .await?;

                sqlx::query(
                    "INSERT INTO notification_preferences \
                     (id, user_id, pool_id, type, channel, enabled) \
                     VALUES (?, NULL, ?, ?, ?, ?)",
                )
                .bind(&record.id)
                .bind(&record.user_id)
                .bind(&record.notification_type)
                .bind(&record.channel)
                .bind(record.enabled)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "INSERT INTO notification_preferences \
                     (id, user_id, pool_id, type, channel, enabled) \
                     VALUES (?, ?, ?, ?, ?, ?) \
                     ON DUPLICATE KEY UPDATE enabled = VALUES(enabled)",
                )
                .bind(&record.id)
                .bind(&record.user_id)
                .bind(&record.pool_id)
                .bind(&record.notification_type)
                .bind(&record.channel)
                .bind(record.enabled)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

fn map_preference(row: sqlx::mysql::MySqlRow) -> Result<NotificationPreference, AppError> {
    Ok(NotificationPreference {
        id: mapper::id(row.try_get("id")?)?,
        user_id: mapper::id(row.try_get("user_id")?)?,
        pool_id: mapper::opt_id(row.try_get("pool_id")?)?,
        notification_type: mapper::notification_type(row.try_get("type")?)?,
        channel: mapper::channel(row.try_get("channel")?)?,
        enabled: row.try_get("enabled")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
