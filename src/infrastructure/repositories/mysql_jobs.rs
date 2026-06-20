use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::jobs::{
    LiveMatchRepository, OutboundNotification, OutboundNotificationRepository,
    ReminderQueryRepository, RemindedRecipient,
};
use crate::application::notifications::{NewOutboxRecord, NotificationOutboxWriter};
use crate::database::transaction::DatabaseTransaction;
use crate::domain::matches::Match;
use crate::domain::notifications::Channel;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_matches::{map_match, MATCH_COLUMNS};

/// Delivery is retried up to this many times before the outbox row is marked
/// as permanently failed.
const MAX_DELIVERY_ATTEMPTS: i32 = 5;

#[derive(Clone)]
pub struct MySqlJobsRepository {
    pool: MySqlPool,
}

impl MySqlJobsRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReminderQueryRepository for MySqlJobsRepository {
    async fn upcoming_scheduled_matches(
        &self,
        after: &str,
        until: &str,
    ) -> Result<Vec<Match>, AppError> {
        let sql = format!(
            "{MATCH_COLUMNS} WHERE status = 'scheduled' AND kickoff_at > ? AND kickoff_at <= ? \
             ORDER BY kickoff_at"
        );
        let rows = sqlx::query(&sql)
            .bind(after)
            .bind(until)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_match).collect()
    }

    async fn reminded_recipients_for_match(
        &self,
        match_id: &str,
    ) -> Result<Vec<RemindedRecipient>, AppError> {
        let rows = sqlx::query(
            "SELECT user_id, pool_id FROM notifications \
             WHERE type = 'prediction_reminder' AND related_match_id = ? AND pool_id IS NOT NULL",
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(RemindedRecipient {
                    user_id: row.try_get("user_id")?,
                    pool_id: row.try_get("pool_id")?,
                })
            })
            .collect()
    }

    async fn users_reminded_since(&self, since: &str) -> Result<Vec<String>, AppError> {
        // Considera tanto lembretes in-app (notifications) quanto push enfileirados
        // (notification_outbox), para que o throttle entre runs também cubra
        // usuários que recebem apenas push.
        let rows = sqlx::query(
            "SELECT user_id FROM notifications \
             WHERE type = 'prediction_reminder' AND created_at >= ? \
             UNION \
             SELECT user_id FROM notification_outbox \
             WHERE type = 'prediction_reminder' AND created_at >= ?",
        )
        .bind(since)
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| Ok(row.try_get("user_id")?))
            .collect()
    }
}

#[async_trait]
impl OutboundNotificationRepository for MySqlJobsRepository {
    async fn list_pending(&self, limit: u32) -> Result<Vec<OutboundNotification>, AppError> {
        let rows = sqlx::query(
            "SELECT id, channel, user_id, title, body FROM notification_outbox \
             WHERE status = 'pending' AND attempts < ? \
             ORDER BY created_at LIMIT ?",
        )
        .bind(MAX_DELIVERY_ATTEMPTS)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let channel: String = row.try_get("channel")?;
                Ok(OutboundNotification {
                    id: row.try_get("id")?,
                    channel: Channel::parse(&channel)
                        .map_err(|_| AppError::Internal("invalid outbox channel".to_owned()))?,
                    user_id: row.try_get("user_id")?,
                    title: row.try_get("title")?,
                    body: row.try_get("body")?,
                })
            })
            .collect()
    }

    async fn mark_delivered(&self, id: &str, delivered_at: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE notification_outbox \
             SET status = 'delivered', delivered_at = ? WHERE id = ?",
        )
        .bind(delivered_at)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_failed(&self, id: &str, error: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE notification_outbox \
             SET attempts = attempts + 1, last_error = ?, \
                 status = IF(attempts + 1 >= ?, 'failed', 'pending') \
             WHERE id = ?",
        )
        .bind(error)
        .bind(MAX_DELIVERY_ATTEMPTS)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl NotificationOutboxWriter for MySqlJobsRepository {
    async fn enqueue_many(&self, records: Vec<NewOutboxRecord>) -> Result<(), AppError> {
        if records.is_empty() {
            return Ok(());
        }
        let mut tx: DatabaseTransaction = self.pool.begin().await?;
        for record in &records {
            sqlx::query(
                "INSERT INTO notification_outbox \
                 (id, channel, type, user_id, title, body, related_match_id, pool_id) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&record.id)
            .bind(&record.channel)
            .bind(&record.notification_type)
            .bind(&record.user_id)
            .bind(&record.title)
            .bind(&record.body)
            .bind(&record.related_match_id)
            .bind(&record.pool_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl LiveMatchRepository for MySqlJobsRepository {
    async fn start_due_matches(&self, now: &str) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE matches SET status = 'live' \
             WHERE status = 'scheduled' AND kickoff_at <= ?",
        )
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
