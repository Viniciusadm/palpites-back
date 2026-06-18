use std::collections::HashSet;

use chrono::Duration;

use crate::application::jobs::{
    DispatchSummary, NotificationSender, OutboundNotificationRepository, ReminderQueryRepository,
    ReminderWindow,
};
use crate::application::pools::{PoolMemberRepository, PoolRepository};
use crate::application::predictions::PredictionRepository;
use crate::application::shared::{parse_datetime, Clock, Notifier, ReminderRecipient};
use crate::domain::pools::{MemberStatus, PoolStatus};
use crate::errors::AppError;

pub struct SendPredictionReminders<Q, PoolR, MemberR, PredR, N, C> {
    queries: Q,
    pools: PoolR,
    members: MemberR,
    predictions: PredR,
    notifier: N,
    clock: C,
}

impl<Q, PoolR, MemberR, PredR, N, C> SendPredictionReminders<Q, PoolR, MemberR, PredR, N, C>
where
    Q: ReminderQueryRepository,
    PoolR: PoolRepository,
    MemberR: PoolMemberRepository,
    PredR: PredictionRepository,
    N: Notifier,
    C: Clock,
{
    pub fn new(
        queries: Q,
        pools: PoolR,
        members: MemberR,
        predictions: PredR,
        notifier: N,
        clock: C,
    ) -> Self {
        Self {
            queries,
            pools,
            members,
            predictions,
            notifier,
            clock,
        }
    }

    pub async fn run(&self, window: ReminderWindow) -> Result<u32, AppError> {
        let now = self.clock.now();
        let now_dt = parse_datetime(now.as_str())
            .ok_or_else(|| AppError::Internal("clock produced an invalid date-time".to_owned()))?;
        let until_dt = now_dt + Duration::minutes(i64::from(window.window_minutes));
        let until = until_dt.to_string();

        let matches = self
            .queries
            .upcoming_scheduled_matches(now.as_str(), &until)
            .await?;

        let mut created = 0u32;
        for game in matches {
            let kickoff_dt = match parse_datetime(game.kickoff_at.as_str()) {
                Some(value) => value,
                None => continue,
            };

            let already: HashSet<(String, String)> = self
                .queries
                .reminded_recipients_for_match(game.id.as_str())
                .await?
                .into_iter()
                .map(|recipient| (recipient.user_id, recipient.pool_id))
                .collect();

            let predicted: HashSet<String> = self
                .predictions
                .list_for_match(game.id.as_str())
                .await?
                .into_iter()
                .map(|prediction| prediction.pool_member_id.as_str().to_owned())
                .collect();

            let pools = self
                .pools
                .list_for_tournament(game.tournament_id.as_str())
                .await?;

            let mut recipients = Vec::new();
            for pool in pools {
                if pool.status != PoolStatus::Active {
                    continue;
                }
                let lock_at =
                    kickoff_dt - Duration::minutes(i64::from(pool.prediction_lock_offset_minutes));
                if now_dt >= lock_at {
                    continue;
                }

                let members = self.members.list_for_pool(pool.id.as_str()).await?;
                for member in members {
                    if member.status != MemberStatus::Active {
                        continue;
                    }
                    if predicted.contains(member.id.as_str()) {
                        continue;
                    }
                    let user_id = member.user_id.as_str().to_owned();
                    let pool_id = pool.id.as_str().to_owned();
                    if already.contains(&(user_id.clone(), pool_id.clone())) {
                        continue;
                    }
                    recipients.push(ReminderRecipient { user_id, pool_id });
                }
            }

            if !recipients.is_empty() {
                self.notifier
                    .prediction_reminder(game.id.as_str(), &recipients)
                    .await?;
                created += recipients.len() as u32;
            }
        }

        Ok(created)
    }
}

pub struct DispatchNotifications<O, C> {
    outbound: O,
    senders: Vec<Box<dyn NotificationSender>>,
    clock: C,
}

impl<O, C> DispatchNotifications<O, C>
where
    O: OutboundNotificationRepository,
    C: Clock,
{
    pub fn new(outbound: O, senders: Vec<Box<dyn NotificationSender>>, clock: C) -> Self {
        Self {
            outbound,
            senders,
            clock,
        }
    }

    pub async fn run(&self, limit: u32) -> Result<DispatchSummary, AppError> {
        let pending = self.outbound.list_pending(limit).await?;
        let mut delivered = 0u32;
        for notification in pending {
            let Some(sender) = self
                .senders
                .iter()
                .find(|sender| sender.channel() == notification.channel)
            else {
                tracing::debug!(
                    channel = notification.channel.as_str(),
                    "no notification sender configured for channel; skipping"
                );
                continue;
            };
            sender.send(&notification).await?;
            let now = self.clock.now();
            self.outbound
                .mark_delivered(&notification.id, now.as_str())
                .await?;
            delivered += 1;
        }
        Ok(DispatchSummary { delivered })
    }
}

pub async fn sync_live_matches() -> Result<(), AppError> {
    tracing::info!(
        "sync_live_matches has no external score feed configured; this is a future extension point"
    );
    Ok(())
}
