use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use sqlx::MySqlPool;

use crate::api::dto::predictions::{
    PredictionResponse, PredictionsListResponse, UpsertPredictionRequest,
};
use crate::api::extractors::PoolMember;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::predictions::PredictionUseCases;
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::repositories::mysql_matches::MySqlMatchRepository;
use crate::infrastructure::repositories::mysql_pool_members::MySqlPoolMemberRepository;
use crate::infrastructure::repositories::mysql_pools::MySqlPoolRepository;
use crate::infrastructure::repositories::mysql_predictions::MySqlPredictionRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pools/:id/predictions", get(list_mine))
        .route(
            "/pools/:id/matches/:match_id/prediction",
            put(upsert),
        )
}

type Predictions = PredictionUseCases<
    MySqlPredictionRepository,
    MySqlMatchRepository,
    MySqlPoolRepository,
    MySqlPoolMemberRepository,
    SystemClock,
>;

fn prediction_use_cases(db: MySqlPool) -> Predictions {
    PredictionUseCases::new(
        MySqlPredictionRepository::new(db.clone()),
        MySqlMatchRepository::new(db.clone()),
        MySqlPoolRepository::new(db.clone()),
        MySqlPoolMemberRepository::new(db),
        SystemClock,
    )
}

async fn list_mine(State(state): State<AppState>, member: PoolMember) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match prediction_use_cases(db)
        .list_mine(&member.pool_id, &member.user_id)
        .await
    {
        Ok(predictions) => Json(PredictionsListResponse {
            predictions: predictions
                .iter()
                .map(PredictionResponse::from_prediction)
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn upsert(
    State(state): State<AppState>,
    member: PoolMember,
    Path((_pool_id, match_id)): Path<(String, String)>,
    Json(request): Json<UpsertPredictionRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match prediction_use_cases(db)
        .upsert(&member.pool_id, &member.user_id, &match_id, request.into())
        .await
    {
        Ok(prediction) => Json(PredictionResponse::from_prediction(&prediction)).into_response(),
        Err(error) => app_error(error),
    }
}
