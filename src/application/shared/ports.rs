use async_trait::async_trait;

use crate::domain::matches::Match;
use crate::domain::UtcDateTime;
use crate::errors::AppError;

pub trait Clock: Send + Sync {
    fn now(&self) -> UtcDateTime;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderRecipient {
    pub user_id: String,
    pub pool_id: String,
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn new_match(&self, game: &Match) -> Result<(), AppError>;
    async fn member_joined(&self, pool_id: &str, joined_user_id: &str) -> Result<(), AppError>;
    async fn match_result(&self, game: &Match, pool_ids: &[String]) -> Result<(), AppError>;
    async fn ranking_update(&self, pool_ids: &[String]) -> Result<(), AppError>;
    async fn prediction_reminder(
        &self,
        match_id: &str,
        recipients: &[ReminderRecipient],
    ) -> Result<(), AppError>;
}
