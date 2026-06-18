use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};

use crate::api::authz::require_admin;
use crate::api::dto::matches::{
    CreateMatchRequest, MatchFilterQuery, MatchResponse, MatchesListResponse, UpdateMatchRequest,
};
use crate::api::dto::results::EnterResultRequest;
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::routes::v1::notifications::{notifier, NotifierImpl};
use crate::api::routes::v1::results::result_use_cases;
use crate::api::state::AppState;
use crate::application::matches::MatchUseCases;
use crate::infrastructure::repositories::mysql_matches::MySqlMatchRepository;
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;
use sqlx::MySqlPool;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tournaments/:id/matches", get(list).post(create))
        .route(
            "/matches/:id",
            get(detail).put(update).delete(delete),
        )
        .route("/matches/:id/result", put(enter_result))
}

fn use_cases(db: MySqlPool) -> MatchUseCases<MySqlMatchRepository, NotifierImpl> {
    MatchUseCases::new(MySqlMatchRepository::new(db.clone()), notifier(db))
}

async fn ensure_admin(db: &MySqlPool, auth: &AuthenticatedUser) -> Result<(), Response> {
    require_admin(auth, &MySqlUserRepository::new(db.clone()))
        .await
        .map_err(app_error)
}

async fn list(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(filters): Query<MatchFilterQuery>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match use_cases(db).list(&id, filters.into()).await {
        Ok(matches) => Json(MatchesListResponse {
            matches: matches.iter().map(MatchResponse::from_match).collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn detail(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match use_cases(db).get(&id).await {
        Ok(item) => Json(MatchResponse::from_match(&item)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<CreateMatchRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).create(&id, request.into()).await {
        Ok(item) => (
            StatusCode::CREATED,
            Json(MatchResponse::from_match(&item)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}

async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateMatchRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).update(&id, request.into()).await {
        Ok(item) => Json(MatchResponse::from_match(&item)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).delete(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn enter_result(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<EnterResultRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match result_use_cases(db).enter_result(&id, request.into()).await {
        Ok(item) => Json(MatchResponse::from_match(&item)).into_response(),
        Err(error) => app_error(error),
    }
}
