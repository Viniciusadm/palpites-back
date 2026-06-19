use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::api::dto::notifications::{
    NotificationFilterQuery, NotificationPreferenceResponse, NotificationPreferencesListResponse,
    NotificationResponse, NotificationsListResponse, RegisterDeviceRequest, UpdatePreferencesRequest,
};
use crate::api::extractors::{AuthenticatedUser, PoolMember};
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::notifications::{DeviceTokenRepository, NewDeviceToken, NotificationUseCases};
use crate::errors::AppError;
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::repositories::mysql_device_tokens::MySqlDeviceTokenRepository;
use crate::infrastructure::repositories::mysql_jobs::MySqlJobsRepository;
use crate::infrastructure::repositories::mysql_notification_preferences::MySqlNotificationPreferenceRepository;
use crate::infrastructure::repositories::mysql_notifications::MySqlNotificationRepository;
use crate::infrastructure::repositories::mysql_pool_members::MySqlPoolMemberRepository;
use crate::infrastructure::repositories::mysql_pools::MySqlPoolRepository;

pub type NotifierImpl = NotificationUseCases<
    MySqlNotificationRepository,
    MySqlNotificationPreferenceRepository,
    MySqlPoolRepository,
    MySqlPoolMemberRepository,
    MySqlJobsRepository,
    SystemClock,
>;

pub fn notifier(db: MySqlPool) -> NotifierImpl {
    NotificationUseCases::new(
        MySqlNotificationRepository::new(db.clone()),
        MySqlNotificationPreferenceRepository::new(db.clone()),
        MySqlPoolRepository::new(db.clone()),
        MySqlPoolMemberRepository::new(db.clone()),
        MySqlJobsRepository::new(db),
        SystemClock,
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list))
        .route("/notifications/read-all", patch(read_all))
        .route("/notifications/:id/read", patch(mark_read))
        .route("/notifications/devices", post(register_device))
        .route("/notifications/devices/:token", delete(unregister_device))
        .route(
            "/notifications/preferences",
            get(get_global_preferences).put(update_global_preferences),
        )
        .route(
            "/pools/:id/notification-preferences",
            get(get_preferences).put(update_preferences),
        )
}

async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<NotificationFilterQuery>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match notifier(db)
        .list(&auth.user_id, query.unread.unwrap_or(false))
        .await
    {
        Ok(notifications) => Json(NotificationsListResponse {
            notifications: notifications
                .iter()
                .map(NotificationResponse::from_notification)
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn mark_read(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match notifier(db).mark_read(&auth.user_id, &id).await {
        Ok(notification) => {
            Json(NotificationResponse::from_notification(&notification)).into_response()
        }
        Err(error) => app_error(error),
    }
}

async fn read_all(State(state): State<AppState>, auth: AuthenticatedUser) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match notifier(db).mark_all_read(&auth.user_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn register_device(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<RegisterDeviceRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    let token = request.token.trim();
    if token.is_empty() {
        return app_error(AppError::validation_code(
            "device_token_required",
            "a device token is required",
        ));
    }

    let repo = MySqlDeviceTokenRepository::new(db);
    let record = NewDeviceToken {
        id: Uuid::new_v4().to_string(),
        user_id: auth.user_id.clone(),
        token: token.to_owned(),
        platform: request
            .platform
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "web".to_owned()),
    };

    match repo.upsert(record).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn unregister_device(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(token): Path<String>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match MySqlDeviceTokenRepository::new(db).remove(&token).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn get_global_preferences(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match notifier(db).get_global_preferences(&auth.user_id).await {
        Ok(preferences) => Json(NotificationPreferencesListResponse {
            preferences: preferences
                .iter()
                .map(NotificationPreferenceResponse::from_preference)
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_global_preferences(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<UpdatePreferencesRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match notifier(db)
        .update_global_preferences(&auth.user_id, request.into())
        .await
    {
        Ok(preferences) => Json(NotificationPreferencesListResponse {
            preferences: preferences
                .iter()
                .map(NotificationPreferenceResponse::from_preference)
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn get_preferences(State(state): State<AppState>, member: PoolMember) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match notifier(db)
        .get_preferences(&member.user_id, &member.pool_id)
        .await
    {
        Ok(preferences) => Json(NotificationPreferencesListResponse {
            preferences: preferences
                .iter()
                .map(NotificationPreferenceResponse::from_preference)
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_preferences(
    State(state): State<AppState>,
    member: PoolMember,
    Json(request): Json<UpdatePreferencesRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match notifier(db)
        .update_preferences(&member.user_id, &member.pool_id, request.into())
        .await
    {
        Ok(preferences) => Json(NotificationPreferencesListResponse {
            preferences: preferences
                .iter()
                .map(NotificationPreferenceResponse::from_preference)
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}
