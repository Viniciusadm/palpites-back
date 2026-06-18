use async_trait::async_trait;
use palpites_back::application::shared::{Clock, Notifier, ReminderRecipient};
use palpites_back::domain::matches::Match;
use palpites_back::domain::UtcDateTime;
use palpites_back::errors::AppError;

#[derive(Debug, Clone)]
pub struct FixedClock {
    now: UtcDateTime,
}

impl FixedClock {
    pub fn new(iso8601: &str) -> Self {
        Self {
            now: UtcDateTime::new_iso8601(iso8601.to_owned()).expect("valid fixed timestamp"),
        }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> UtcDateTime {
        self.now.clone()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopNotifier;

#[async_trait]
impl Notifier for NoopNotifier {
    async fn new_match(&self, _game: &Match) -> Result<(), AppError> {
        Ok(())
    }

    async fn member_joined(&self, _pool_id: &str, _joined_user_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn match_result(&self, _game: &Match, _pool_ids: &[String]) -> Result<(), AppError> {
        Ok(())
    }

    async fn ranking_update(&self, _pool_ids: &[String]) -> Result<(), AppError> {
        Ok(())
    }

    async fn prediction_reminder(
        &self,
        _match_id: &str,
        _recipients: &[ReminderRecipient],
    ) -> Result<(), AppError> {
        Ok(())
    }
}
