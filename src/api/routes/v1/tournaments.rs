use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};

use crate::api::authz::require_admin;
use crate::api::dto::tournaments::{
    AssignTeamRequest, CreateGroupRequest, CreateStageRequest, CreateTournamentRequest,
    GroupResponse, StageResponse, TournamentDetailResponse, TournamentResponse,
    TournamentTeamResponse, TournamentTeamsListResponse, TournamentsListResponse,
    UpdateGroupRequest, UpdateStageRequest, UpdateTournamentRequest,
};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::tournaments::TournamentUseCases;
use crate::infrastructure::repositories::mysql_tournaments::MySqlTournamentRepository;
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;
use sqlx::MySqlPool;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tournaments", get(list).post(create))
        .route("/tournaments/:id", get(detail).put(update).delete(delete))
        .route("/tournaments/:id/stages", post(add_stage))
        .route("/stages/:id", put(update_stage).delete(delete_stage))
        .route("/tournaments/:id/groups", post(add_group))
        .route("/groups/:id", put(update_group).delete(delete_group))
        .route("/tournaments/:id/teams", get(list_teams).post(assign_team))
        .route(
            "/tournaments/:id/teams/:tournament_team_id",
            axum::routing::delete(unassign_team),
        )
}

fn use_cases(db: MySqlPool) -> TournamentUseCases<MySqlTournamentRepository> {
    TournamentUseCases::new(MySqlTournamentRepository::new(db))
}

async fn ensure_admin(db: &MySqlPool, auth: &AuthenticatedUser) -> Result<(), Response> {
    require_admin(auth, &MySqlUserRepository::new(db.clone()))
        .await
        .map_err(app_error)
}

async fn list(State(state): State<AppState>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match use_cases(db).list().await {
        Ok(tournaments) => Json(TournamentsListResponse {
            tournaments: tournaments
                .iter()
                .map(TournamentResponse::from_tournament)
                .collect(),
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

    match use_cases(db).get_detail(&id).await {
        Ok(detail) => Json(TournamentDetailResponse::from_detail(&detail)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateTournamentRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).create(request.into()).await {
        Ok(tournament) => (
            StatusCode::CREATED,
            Json(TournamentResponse::from_tournament(&tournament)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}

async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateTournamentRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).update(&id, request.into()).await {
        Ok(tournament) => Json(TournamentResponse::from_tournament(&tournament)).into_response(),
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

async fn add_stage(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<CreateStageRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).add_stage(&id, request.into()).await {
        Ok(stage) => {
            (StatusCode::CREATED, Json(StageResponse::from_stage(&stage))).into_response()
        }
        Err(error) => app_error(error),
    }
}

async fn update_stage(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateStageRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).update_stage(&id, request.into()).await {
        Ok(stage) => Json(StageResponse::from_stage(&stage)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn delete_stage(
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

    match use_cases(db).delete_stage(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn add_group(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<CreateGroupRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).add_group(&id, request.into()).await {
        Ok(group) => {
            (StatusCode::CREATED, Json(GroupResponse::from_group(&group))).into_response()
        }
        Err(error) => app_error(error),
    }
}

async fn update_group(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateGroupRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).update_group(&id, request.into()).await {
        Ok(group) => Json(GroupResponse::from_group(&group)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn delete_group(
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

    match use_cases(db).delete_group(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn list_teams(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match use_cases(db).list_teams(&id).await {
        Ok(assignments) => Json(TournamentTeamsListResponse {
            teams: assignments
                .iter()
                .map(TournamentTeamResponse::from_assignment)
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn assign_team(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<AssignTeamRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).assign_team(&id, request.into()).await {
        Ok(assignment) => (
            StatusCode::CREATED,
            Json(TournamentTeamResponse::from_assignment(&assignment)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}

async fn unassign_team(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((id, tournament_team_id)): Path<(String, String)>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(response) = ensure_admin(&db, &auth).await {
        return response;
    }

    match use_cases(db).unassign_team(&id, &tournament_team_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}
