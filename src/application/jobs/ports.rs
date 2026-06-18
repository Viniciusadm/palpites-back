use async_trait::async_trait;

use crate::domain::matches::Match;
use crate::domain::notifications::Channel;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemindedRecipient {
    pub user_id: String,
    pub pool_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundNotification {
    pub id: String,
    pub channel: Channel,
    pub user_id: String,
    pub title: String,
    pub body: String,
}

#[async_trait]
pub trait ReminderQueryRepository: Send + Sync {
    async fn upcoming_scheduled_matches(
        &self,
        after: &str,
        until: &str,
    ) -> Result<Vec<Match>, AppError>;
    async fn reminded_recipients_for_match(
        &self,
        match_id: &str,
    ) -> Result<Vec<RemindedRecipient>, AppError>;
}

#[async_trait]
pub trait OutboundNotificationRepository: Send + Sync {
    async fn list_pending(&self, limit: u32) -> Result<Vec<OutboundNotification>, AppError>;
    async fn mark_delivered(&self, id: &str, delivered_at: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait NotificationSender: Send + Sync {
    fn channel(&self) -> Channel;
    async fn send(&self, notification: &OutboundNotification) -> Result<(), AppError>;
}
