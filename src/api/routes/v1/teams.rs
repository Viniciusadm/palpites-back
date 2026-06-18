use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use crate::api::authz::require_admin;
use crate::api::dto::teams::{
    CreateTeamRequest, TeamResponse, TeamsListResponse, UpdateTeamRequest,
};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::teams::TeamUseCases;
use crate::infrastructure::repositories::mysql_teams::MySqlTeamRepository;
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/teams", get(list).post(create))
        .route("/teams/:id", get(detail).put(update).delete(delete))
}

async fn list(State(state): State<AppState>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let use_cases = TeamUseCases::new(MySqlTeamRepository::new(db));

    match use_cases.list().await {
        Ok(teams) => Json(TeamsListResponse {
            teams: teams.iter().map(TeamResponse::from_team).collect(),
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
    let use_cases = TeamUseCases::new(MySqlTeamRepository::new(db));

    match use_cases.get(&id).await {
        Ok(team) => Json(TeamResponse::from_team(&team)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateTeamRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(error) = require_admin(&auth, &MySqlUserRepository::new(db.clone())).await {
        return app_error(error);
    }
    let use_cases = TeamUseCases::new(MySqlTeamRepository::new(db));

    match use_cases.create(request.into()).await {
        Ok(team) => (StatusCode::CREATED, Json(TeamResponse::from_team(&team))).into_response(),
        Err(error) => app_error(error),
    }
}

async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateTeamRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(error) = require_admin(&auth, &MySqlUserRepository::new(db.clone())).await {
        return app_error(error);
    }
    let use_cases = TeamUseCases::new(MySqlTeamRepository::new(db));

    match use_cases.update(&id, request.into()).await {
        Ok(team) => Json(TeamResponse::from_team(&team)).into_response(),
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
    if let Err(error) = require_admin(&auth, &MySqlUserRepository::new(db.clone())).await {
        return app_error(error);
    }
    let use_cases = TeamUseCases::new(MySqlTeamRepository::new(db));

    match use_cases.delete(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}
