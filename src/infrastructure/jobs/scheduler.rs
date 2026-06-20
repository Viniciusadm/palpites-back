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
use crate::infrastructure::jobs::fcm::{FcmPushSender, FcmSettings};
use crate::infrastructure::jobs::senders::LogPushSender;
use crate::infrastructure::repositories::mysql_device_tokens::MySqlDeviceTokenRepository;
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
    let reminder_window = ReminderWindow::new(
        config.jobs_reminder_window_minutes,
        config.jobs_reminder_throttle_minutes,
    );
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
    let dispatch_fcm = fcm_settings(config);
    tokio::spawn(async move {
        let mut ticker = interval(dispatch_interval);
        loop {
            ticker.tick().await;
            if let Err(error) = run_dispatch(&dispatch_pool, dispatch_fcm.clone()).await {
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
        MySqlJobsRepository::new(pool.clone()),
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

fn fcm_settings(config: &AppConfig) -> Option<FcmSettings> {
    match (&config.fcm_project_id, &config.fcm_service_account_path) {
        (Some(project_id), Some(service_account_path)) => Some(FcmSettings {
            project_id: project_id.clone(),
            service_account_path: service_account_path.clone(),
        }),
        _ => None,
    }
}

async fn run_dispatch(
    pool: &MySqlPool,
    fcm: Option<FcmSettings>,
) -> Result<(), crate::errors::AppError> {
    let push: Box<dyn NotificationSender> = match fcm {
        Some(settings) => {
            match FcmPushSender::new(settings, MySqlDeviceTokenRepository::new(pool.clone())) {
                Ok(sender) => Box::new(sender),
                Err(error) => {
                    tracing::error!(%error, "failed to initialize FCM push sender; falling back to no-op");
                    Box::new(LogPushSender)
                }
            }
        }
        None => Box::new(LogPushSender),
    };
    let senders: Vec<Box<dyn NotificationSender>> = vec![push];
    let runner =
        DispatchNotifications::new(MySqlJobsRepository::new(pool.clone()), senders, SystemClock);
    runner.run(PENDING_DISPATCH_LIMIT).await?;
    Ok(())
}

async fn run_live_transitions(pool: &MySqlPool) -> Result<(), crate::errors::AppError> {
    let runner = StartLiveMatches::new(MySqlJobsRepository::new(pool.clone()), SystemClock);
    runner.run().await?;
    Ok(())
}
