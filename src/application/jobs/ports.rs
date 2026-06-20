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
    /// user_ids que já receberam um `prediction_reminder` desde `since`
    /// (timestamp UTC). Usado para garantir no máximo um lembrete por usuário
    /// dentro da janela de throttle, independentemente de quantos jogos existam.
    async fn users_reminded_since(&self, since: &str) -> Result<Vec<String>, AppError>;
}

#[async_trait]
pub trait OutboundNotificationRepository: Send + Sync {
    async fn list_pending(&self, limit: u32) -> Result<Vec<OutboundNotification>, AppError>;
    async fn mark_delivered(&self, id: &str, delivered_at: &str) -> Result<(), AppError>;
    /// Records a failed delivery attempt. Implementations increment the attempt
    /// counter and persist the error, keeping the record pending for retry until
    /// a maximum number of attempts is reached.
    async fn mark_failed(&self, id: &str, error: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait NotificationSender: Send + Sync {
    fn channel(&self) -> Channel;
    async fn send(&self, notification: &OutboundNotification) -> Result<(), AppError>;
}

#[async_trait]
pub trait LiveMatchRepository: Send + Sync {
    /// Transitions scheduled matches whose kickoff time has already passed to the live
    /// status. `now` is an ISO-8601 UTC timestamp and is compared against `kickoff_at`,
    /// which is also stored in UTC, so the result is independent of any client timezone.
    /// Returns the number of matches that were transitioned.
    async fn start_due_matches(&self, now: &str) -> Result<u64, AppError>;
}
