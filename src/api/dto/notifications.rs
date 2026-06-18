use serde::{Deserialize, Serialize};

use crate::application::notifications::{PreferenceInput, UpdatePreferences};
use crate::domain::notifications::{Notification, NotificationPreference};

#[derive(Debug, Default, Deserialize)]
pub struct NotificationFilterQuery {
    pub unread: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub id: String,
    pub pool_id: Option<String>,
    pub r#type: String,
    pub title: String,
    pub body: String,
    pub related_match_id: Option<String>,
    pub read_at: Option<String>,
    pub created_at: String,
}

impl NotificationResponse {
    pub fn from_notification(value: &Notification) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            pool_id: value.pool_id.as_ref().map(|id| id.as_str().to_owned()),
            r#type: value.notification_type.as_str().to_owned(),
            title: value.title.as_str().to_owned(),
            body: value.body.as_str().to_owned(),
            related_match_id: value
                .related_match_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            read_at: value.read_at.as_ref().map(|value| value.as_str().to_owned()),
            created_at: value.created_at.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NotificationsListResponse {
    pub notifications: Vec<NotificationResponse>,
}

#[derive(Debug, Serialize)]
pub struct NotificationPreferenceResponse {
    pub id: String,
    pub pool_id: Option<String>,
    pub r#type: String,
    pub channel: String,
    pub enabled: bool,
}

impl NotificationPreferenceResponse {
    pub fn from_preference(value: &NotificationPreference) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            pool_id: value.pool_id.as_ref().map(|id| id.as_str().to_owned()),
            r#type: value.notification_type.as_str().to_owned(),
            channel: value.channel.as_str().to_owned(),
            enabled: value.enabled,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NotificationPreferencesListResponse {
    pub preferences: Vec<NotificationPreferenceResponse>,
}

#[derive(Debug, Deserialize)]
pub struct PreferenceItemRequest {
    pub r#type: String,
    pub channel: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub preferences: Vec<PreferenceItemRequest>,
}

impl From<UpdatePreferencesRequest> for UpdatePreferences {
    fn from(value: UpdatePreferencesRequest) -> Self {
        Self {
            items: value
                .preferences
                .into_iter()
                .map(|item| PreferenceInput {
                    notification_type: item.r#type,
                    channel: item.channel,
                    enabled: item.enabled,
                })
                .collect(),
        }
    }
}
