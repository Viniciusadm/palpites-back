use async_trait::async_trait;

use crate::application::jobs::{NotificationSender, OutboundNotification};
use crate::domain::notifications::Channel;
use crate::errors::AppError;

/// No-op push sender used when FCM is not configured. Delivery stays dormant
/// (logged only) until `FCM_PROJECT_ID` and `FCM_SERVICE_ACCOUNT_PATH` are set.
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
            "push delivery is not configured (FCM_PROJECT_ID / FCM_SERVICE_ACCOUNT_PATH); logging only"
        );
        Ok(())
    }
}
