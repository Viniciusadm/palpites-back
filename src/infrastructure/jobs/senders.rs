use async_trait::async_trait;

use crate::application::jobs::{NotificationSender, OutboundNotification};
use crate::domain::notifications::Channel;
use crate::errors::AppError;

#[derive(Debug, Clone, Copy, Default)]
pub struct LogEmailSender;

#[async_trait]
impl NotificationSender for LogEmailSender {
    fn channel(&self) -> Channel {
        Channel::Email
    }

    async fn send(&self, notification: &OutboundNotification) -> Result<(), AppError> {
        tracing::info!(
            user_id = notification.user_id,
            title = notification.title,
            "email delivery is not wired to a provider yet; logging only (future extension point)"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LogPushSender;

#[async_trait]
impl NotificationSender for LogPushSender {
    fn channel(&self) -> Channel {
        Channel::Push
    }

    async fn send(&self, notification: &OutboundNotification) -> Result<(), AppError> {
        tracing::info!(
            user_id = notification.user_id,
            title = notification.title,
            "push delivery is not wired to a provider yet; logging only (future extension point)"
        );
        Ok(())
    }
}
