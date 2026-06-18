use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::notifications::{NewNotificationRecord, NotificationRepository};
use crate::database::transaction::DatabaseTransaction;
use crate::domain::notifications::Notification;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const NOTIFICATION_COLUMNS: &str = "SELECT id, user_id, pool_id, type, title, body, \
     related_match_id, read_at, created_at \
     FROM notifications";

#[derive(Clone)]
pub struct MySqlNotificationRepository {
    pool: MySqlPool,
}

impl MySqlNotificationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationRepository for MySqlNotificationRepository {
    async fn create_many(&self, records: Vec<NewNotificationRecord>) -> Result<(), AppError> {
        if records.is_empty() {
            return Ok(());
        }
        let mut tx: DatabaseTransaction = self.pool.begin().await?;
        for record in &records {
            sqlx::query(
                "INSERT INTO notifications \
                 (id, user_id, pool_id, type, title, body, related_match_id) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&record.id)
            .bind(&record.user_id)
            .bind(&record.pool_id)
            .bind(&record.notification_type)
            .bind(&record.title)
            .bind(&record.body)
            .bind(&record.related_match_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_for_user(
        &self,
        user_id: &str,
        only_unread: bool,
    ) -> Result<Vec<Notification>, AppError> {
        let sql = if only_unread {
            format!("{NOTIFICATION_COLUMNS} WHERE user_id = ? AND read_at IS NULL ORDER BY created_at DESC")
        } else {
            format!("{NOTIFICATION_COLUMNS} WHERE user_id = ? ORDER BY created_at DESC")
        };
        let rows = sqlx::query(&sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_notification).collect()
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Notification>, AppError> {
        let sql = format!("{NOTIFICATION_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_notification).transpose()
    }

    async fn mark_read(&self, id: &str, read_at: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE notifications SET read_at = ? WHERE id = ? AND read_at IS NULL")
            .bind(read_at)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn mark_all_read(&self, user_id: &str, read_at: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE notifications SET read_at = ? WHERE user_id = ? AND read_at IS NULL")
            .bind(read_at)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn map_notification(row: sqlx::mysql::MySqlRow) -> Result<Notification, AppError> {
    Ok(Notification {
        id: mapper::id(row.try_get("id")?)?,
        user_id: mapper::id(row.try_get("user_id")?)?,
        pool_id: mapper::opt_id(row.try_get("pool_id")?)?,
        notification_type: mapper::notification_type(row.try_get("type")?)?,
        title: mapper::non_empty(row.try_get("title")?, "notification.title")?,
        body: mapper::non_empty(row.try_get("body")?, "notification.body")?,
        related_match_id: mapper::opt_id(row.try_get("related_match_id")?)?,
        read_at: mapper::opt_datetime(row.try_get("read_at")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
    })
}
