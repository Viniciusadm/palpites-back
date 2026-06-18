use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::notifications::{
    NewNotificationRecord, NewPreferenceRecord, NotificationPreferenceRepository,
    NotificationRepository, UpdatePreferences,
};
use crate::application::pools::{PoolMemberRepository, PoolRepository};
use crate::application::shared::{Clock, Notifier, ReminderRecipient};
use crate::domain::matches::Match;
use crate::domain::notifications::{
    Channel, Notification, NotificationPreference, NotificationType,
};
use crate::domain::pools::MemberStatus;
use crate::errors::AppError;

pub struct NotificationUseCases<N, P, PoolR, MemberR, C> {
    notifications: N,
    preferences: P,
    pools: PoolR,
    members: MemberR,
    clock: C,
}

struct Recipient {
    user_id: String,
    pool_id: String,
}

impl<N, P, PoolR, MemberR, C> NotificationUseCases<N, P, PoolR, MemberR, C>
where
    N: NotificationRepository,
    P: NotificationPreferenceRepository,
    PoolR: PoolRepository,
    MemberR: PoolMemberRepository,
    C: Clock,
{
    pub fn new(notifications: N, preferences: P, pools: PoolR, members: MemberR, clock: C) -> Self {
        Self {
            notifications,
            preferences,
            pools,
            members,
            clock,
        }
    }

    pub async fn list(
        &self,
        user_id: &str,
        only_unread: bool,
    ) -> Result<Vec<Notification>, AppError> {
        self.notifications.list_for_user(user_id, only_unread).await
    }

    pub async fn mark_read(&self, user_id: &str, id: &str) -> Result<Notification, AppError> {
        let notification = self.owned_notification(user_id, id).await?;
        if notification.read_at.is_none() {
            let now = self.clock.now().as_str().to_owned();
            self.notifications.mark_read(id, &now).await?;
        }
        self.owned_notification(user_id, id).await
    }

    pub async fn mark_all_read(&self, user_id: &str) -> Result<(), AppError> {
        let now = self.clock.now().as_str().to_owned();
        self.notifications.mark_all_read(user_id, &now).await
    }

    pub async fn get_preferences(
        &self,
        user_id: &str,
        pool_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError> {
        self.preferences.list_for_scope(user_id, pool_id).await
    }

    pub async fn update_preferences(
        &self,
        user_id: &str,
        pool_id: &str,
        command: UpdatePreferences,
    ) -> Result<Vec<NotificationPreference>, AppError> {
        let mut records = Vec::with_capacity(command.items.len());
        for item in command.items {
            let notification_type = NotificationType::parse(&item.notification_type)?;
            if !notification_type.is_preferenceable() {
                return Err(AppError::validation_code(
                    "notification_type_not_preferenceable",
                    "this notification type has no configurable preference",
                ));
            }
            let channel = Channel::parse(&item.channel)?;
            records.push(NewPreferenceRecord {
                id: new_id(),
                user_id: user_id.to_owned(),
                pool_id: Some(pool_id.to_owned()),
                notification_type: notification_type.as_str().to_owned(),
                channel: channel.as_str().to_owned(),
                enabled: item.enabled,
            });
        }
        self.preferences.upsert_many(records).await?;
        self.get_preferences(user_id, pool_id).await
    }

    async fn owned_notification(
        &self,
        user_id: &str,
        id: &str,
    ) -> Result<Notification, AppError> {
        self.notifications
            .find_by_id(id)
            .await?
            .filter(|notification| notification.user_id.as_str() == user_id)
            .ok_or_else(|| AppError::NotFound("notification was not found".to_owned()))
    }

    async fn dispatch(
        &self,
        recipients: Vec<Recipient>,
        notification_type: NotificationType,
        related_match_id: Option<&str>,
    ) -> Result<(), AppError> {
        let (title, body) = copy_for(notification_type);
        let mut cache: HashMap<String, Vec<NotificationPreference>> = HashMap::new();
        let mut records = Vec::new();

        for recipient in recipients {
            if !cache.contains_key(&recipient.user_id) {
                let prefs = self.preferences.list_for_user(&recipient.user_id).await?;
                cache.insert(recipient.user_id.clone(), prefs);
            }
            let prefs = &cache[&recipient.user_id];

            if !resolve_enabled(prefs, &recipient.pool_id, notification_type, Channel::InApp) {
                continue;
            }

            records.push(NewNotificationRecord {
                id: new_id(),
                user_id: recipient.user_id,
                pool_id: Some(recipient.pool_id),
                notification_type: notification_type.as_str().to_owned(),
                title: title.to_owned(),
                body: body.to_owned(),
                related_match_id: related_match_id.map(ToOwned::to_owned),
            });
        }

        if records.is_empty() {
            return Ok(());
        }
        self.notifications.create_many(records).await
    }

    async fn active_members(&self, pool_id: &str) -> Result<Vec<Recipient>, AppError> {
        Ok(self
            .members
            .list_for_pool(pool_id)
            .await?
            .into_iter()
            .filter(|member| member.status == MemberStatus::Active)
            .map(|member| Recipient {
                user_id: member.user_id.as_str().to_owned(),
                pool_id: pool_id.to_owned(),
            })
            .collect())
    }
}

#[async_trait]
impl<N, P, PoolR, MemberR, C> Notifier for NotificationUseCases<N, P, PoolR, MemberR, C>
where
    N: NotificationRepository,
    P: NotificationPreferenceRepository,
    PoolR: PoolRepository,
    MemberR: PoolMemberRepository,
    C: Clock,
{
    async fn new_match(&self, game: &Match) -> Result<(), AppError> {
        let pools = self.pools.list_for_tournament(game.tournament_id.as_str()).await?;
        let mut recipients = Vec::new();
        for pool in pools {
            recipients.extend(self.active_members(pool.id.as_str()).await?);
        }
        self.dispatch(recipients, NotificationType::NewMatch, Some(game.id.as_str()))
            .await
    }

    async fn member_joined(&self, pool_id: &str, joined_user_id: &str) -> Result<(), AppError> {
        let recipients = self
            .members
            .list_for_pool(pool_id)
            .await?
            .into_iter()
            .filter(|member| member.status == MemberStatus::Active)
            .filter(|member| member.role.can_manage_members())
            .filter(|member| member.user_id.as_str() != joined_user_id)
            .map(|member| Recipient {
                user_id: member.user_id.as_str().to_owned(),
                pool_id: pool_id.to_owned(),
            })
            .collect();
        self.dispatch(recipients, NotificationType::MemberJoined, None)
            .await
    }

    async fn match_result(&self, game: &Match, pool_ids: &[String]) -> Result<(), AppError> {
        let mut recipients = Vec::new();
        for pool_id in pool_ids {
            recipients.extend(self.active_members(pool_id).await?);
        }
        self.dispatch(recipients, NotificationType::MatchResult, Some(game.id.as_str()))
            .await
    }

    async fn ranking_update(&self, pool_ids: &[String]) -> Result<(), AppError> {
        let mut recipients = Vec::new();
        for pool_id in pool_ids {
            recipients.extend(self.active_members(pool_id).await?);
        }
        self.dispatch(recipients, NotificationType::RankingUpdate, None)
            .await
    }

    async fn prediction_reminder(
        &self,
        match_id: &str,
        recipients: &[ReminderRecipient],
    ) -> Result<(), AppError> {
        let recipients = recipients
            .iter()
            .map(|recipient| Recipient {
                user_id: recipient.user_id.clone(),
                pool_id: recipient.pool_id.clone(),
            })
            .collect();
        self.dispatch(
            recipients,
            NotificationType::PredictionReminder,
            Some(match_id),
        )
        .await
    }
}

fn resolve_enabled(
    prefs: &[NotificationPreference],
    pool_id: &str,
    notification_type: NotificationType,
    channel: Channel,
) -> bool {
    let matches_type = |pref: &&NotificationPreference| {
        pref.notification_type == notification_type && pref.channel == channel
    };
    if let Some(pref) = prefs
        .iter()
        .filter(matches_type)
        .find(|pref| pref.pool_id.as_ref().map(|id| id.as_str()) == Some(pool_id))
    {
        return pref.enabled;
    }
    if let Some(pref) = prefs
        .iter()
        .filter(matches_type)
        .find(|pref| pref.pool_id.is_none())
    {
        return pref.enabled;
    }
    true
}

fn copy_for(notification_type: NotificationType) -> (&'static str, &'static str) {
    match notification_type {
        NotificationType::NewMatch => (
            "New match scheduled",
            "A new match has been added to your pool.",
        ),
        NotificationType::MatchResult => (
            "Match result available",
            "A match you predicted now has a final result.",
        ),
        NotificationType::RankingUpdate => {
            ("Ranking updated", "Your pool ranking has changed.")
        }
        NotificationType::MemberJoined => {
            ("New member joined", "A new member has joined your pool.")
        }
        NotificationType::PredictionReminder => (
            "Prediction reminder",
            "You still have matches waiting for a prediction.",
        ),
        NotificationType::Invite => ("Pool invite", "You have been invited to a pool."),
    }
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
