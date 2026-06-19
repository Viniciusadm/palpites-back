use async_trait::async_trait;

use crate::domain::notifications::{Notification, NotificationPreference};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNotificationRecord {
    pub id: String,
    pub user_id: String,
    pub pool_id: Option<String>,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub related_match_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPreferenceRecord {
    pub id: String,
    pub user_id: String,
    pub pool_id: Option<String>,
    pub notification_type: String,
    pub channel: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOutboxRecord {
    pub id: String,
    pub channel: String,
    pub user_id: String,
    pub title: String,
    pub body: String,
    pub related_match_id: Option<String>,
    pub pool_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDeviceToken {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub platform: String,
}

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn create_many(&self, records: Vec<NewNotificationRecord>) -> Result<(), AppError>;
    async fn list_for_user(
        &self,
        user_id: &str,
        only_unread: bool,
    ) -> Result<Vec<Notification>, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Notification>, AppError>;
    async fn mark_read(&self, id: &str, read_at: &str) -> Result<(), AppError>;
    async fn mark_all_read(&self, user_id: &str, read_at: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait NotificationPreferenceRepository: Send + Sync {
    async fn list_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError>;
    async fn list_for_scope(
        &self,
        user_id: &str,
        pool_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError>;
    async fn upsert_many(&self, records: Vec<NewPreferenceRecord>) -> Result<(), AppError>;
}

/// Write side of the durable notification outbox. Records enqueued here are
/// picked up by the dispatch job and delivered through external channels (push).
#[async_trait]
pub trait NotificationOutboxWriter: Send + Sync {
    async fn enqueue_many(&self, records: Vec<NewOutboxRecord>) -> Result<(), AppError>;
}

#[async_trait]
pub trait DeviceTokenRepository: Send + Sync {
    async fn upsert(&self, record: NewDeviceToken) -> Result<(), AppError>;
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<String>, AppError>;
    async fn remove(&self, token: &str) -> Result<(), AppError>;
}
