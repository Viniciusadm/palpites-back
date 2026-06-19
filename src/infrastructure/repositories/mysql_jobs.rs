use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::jobs::{
    LiveMatchRepository, OutboundNotification, OutboundNotificationRepository,
    ReminderQueryRepository, RemindedRecipient,
};
use crate::domain::matches::Match;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_matches::{map_match, MATCH_COLUMNS};

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
}

#[async_trait]
impl OutboundNotificationRepository for MySqlJobsRepository {
    async fn list_pending(&self, _limit: u32) -> Result<Vec<OutboundNotification>, AppError> {
        tracing::debug!(
            "outbound notification delivery is not backed by storage yet; \
             a durable email/push outbox is a future extension point"
        );
        Ok(Vec::new())
    }

    async fn mark_delivered(&self, _id: &str, _delivered_at: &str) -> Result<(), AppError> {
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
