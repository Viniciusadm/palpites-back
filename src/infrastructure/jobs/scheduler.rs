use std::time::Duration;

use sqlx::MySqlPool;
use tokio::time::interval;

use crate::application::jobs::{
    DispatchNotifications, NotificationSender, ReminderWindow, SendPredictionReminders,
    StartLiveMatches,
};
use crate::application::notifications::NotificationUseCases;
use crate::config::AppConfig;
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::jobs::senders::{LogEmailSender, LogPushSender};
use crate::infrastructure::repositories::mysql_jobs::MySqlJobsRepository;
use crate::infrastructure::repositories::mysql_notification_preferences::MySqlNotificationPreferenceRepository;
use crate::infrastructure::repositories::mysql_notifications::MySqlNotificationRepository;
use crate::infrastructure::repositories::mysql_pool_members::MySqlPoolMemberRepository;
use crate::infrastructure::repositories::mysql_pools::MySqlPoolRepository;
use crate::infrastructure::repositories::mysql_predictions::MySqlPredictionRepository;

const PENDING_DISPATCH_LIMIT: u32 = 100;

pub fn spawn(pool: MySqlPool, config: &AppConfig) {
    if !config.jobs_enabled {
        tracing::info!("background jobs are disabled (JOBS_ENABLED=false)");
        return;
    }

    let reminder_interval = Duration::from_secs(config.jobs_reminder_interval_seconds);
    let reminder_window = ReminderWindow::new(config.jobs_reminder_window_minutes);
    let reminder_pool = pool.clone();
    tokio::spawn(async move {
        let mut ticker = interval(reminder_interval);
        loop {
            ticker.tick().await;
            if let Err(error) = run_reminders(&reminder_pool, reminder_window).await {
                tracing::error!(%error, "send_prediction_reminders job failed");
            }
        }
    });

    let dispatch_interval = Duration::from_secs(config.jobs_dispatch_interval_seconds);
    let dispatch_pool = pool.clone();
    tokio::spawn(async move {
        let mut ticker = interval(dispatch_interval);
        loop {
            ticker.tick().await;
            if let Err(error) = run_dispatch(&dispatch_pool).await {
                tracing::error!(%error, "dispatch_notifications job failed");
            }
        }
    });

    let live_interval = Duration::from_secs(config.jobs_live_interval_seconds);
    let live_pool = pool;
    tokio::spawn(async move {
        let mut ticker = interval(live_interval);
        loop {
            ticker.tick().await;
            if let Err(error) = run_live_transitions(&live_pool).await {
                tracing::error!(%error, "start_live_matches job failed");
            }
        }
    });

    tracing::info!("background jobs scheduler started");
}

async fn run_reminders(
    pool: &MySqlPool,
    window: ReminderWindow,
) -> Result<u32, crate::errors::AppError> {
    let notifier = NotificationUseCases::new(
        MySqlNotificationRepository::new(pool.clone()),
        MySqlNotificationPreferenceRepository::new(pool.clone()),
        MySqlPoolRepository::new(pool.clone()),
        MySqlPoolMemberRepository::new(pool.clone()),
        SystemClock,
    );
    let runner = SendPredictionReminders::new(
        MySqlJobsRepository::new(pool.clone()),
        MySqlPoolRepository::new(pool.clone()),
        MySqlPoolMemberRepository::new(pool.clone()),
        MySqlPredictionRepository::new(pool.clone()),
        notifier,
        SystemClock,
    );
    runner.run(window).await
}

async fn run_dispatch(pool: &MySqlPool) -> Result<(), crate::errors::AppError> {
    let senders: Vec<Box<dyn NotificationSender>> =
        vec![Box::new(LogEmailSender), Box::new(LogPushSender)];
    let runner = DispatchNotifications::new(MySqlJobsRepository::new(pool.clone()), senders, SystemClock);
    runner.run(PENDING_DISPATCH_LIMIT).await?;
    Ok(())
}

async fn run_live_transitions(pool: &MySqlPool) -> Result<(), crate::errors::AppError> {
    let runner = StartLiveMatches::new(MySqlJobsRepository::new(pool.clone()), SystemClock);
    runner.run().await?;
    Ok(())
}
