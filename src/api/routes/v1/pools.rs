use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use sqlx::MySqlPool;

use crate::api::dto::pools::{
    ChangeMemberRoleRequest, CreatePoolRequest, JoinPoolRequest, JoinPoolResponse,
    PoolMemberResponse, PoolMembersListResponse, PoolResponse, PoolsListResponse,
    ScoringRuleResponse, ScoringRulesListResponse, UpdatePoolRequest, UpdateScoringRulesRequest,
};
use crate::api::dto::results::{HistoryResponse, MemberPredictionsResponse, RankingResponse};
use crate::api::extractors::{AuthenticatedUser, PoolMember};
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::routes::v1::notifications::{notifier, NotifierImpl};
use crate::api::routes::v1::results::result_use_cases;
use crate::api::state::AppState;
use crate::application::pools::{MembershipUseCases, PoolUseCases, ScoringRuleUseCases};
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::repositories::mysql_pool_members::MySqlPoolMemberRepository;
use crate::infrastructure::repositories::mysql_pools::MySqlPoolRepository;
use crate::infrastructure::repositories::mysql_scoring_rules::MySqlScoringRuleRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pools", get(list).post(create))
        .route("/pools/join", post(join))
        .route("/pools/:id", get(detail).put(update).delete(delete))
        .route("/pools/:id/leave", post(leave))
        .route("/pools/:id/members", get(list_members))
        .route(
            "/pools/:id/members/:member_id",
            axum::routing::delete(remove_member).patch(change_role),
        )
        .route(
            "/pools/:id/scoring-rules",
            get(read_rules).put(update_rules),
        )
        .route("/pools/:id/ranking", get(ranking))
        .route("/pools/:id/history", get(history))
        .route(
            "/pools/:id/members/:member_id/predictions",
            get(member_predictions),
        )
}

fn pool_use_cases(
    db: MySqlPool,
) -> PoolUseCases<MySqlPoolRepository, MySqlPoolMemberRepository, SystemClock, NotifierImpl> {
    PoolUseCases::new(
        MySqlPoolRepository::new(db.clone()),
        MySqlPoolMemberRepository::new(db.clone()),
        SystemClock,
        notifier(db),
    )
}

fn membership_use_cases(
    db: MySqlPool,
) -> MembershipUseCases<MySqlPoolMemberRepository, SystemClock> {
    MembershipUseCases::new(MySqlPoolMemberRepository::new(db), SystemClock)
}

fn scoring_use_cases(db: MySqlPool) -> ScoringRuleUseCases<MySqlScoringRuleRepository> {
    ScoringRuleUseCases::new(MySqlScoringRuleRepository::new(db))
}

async fn list(State(state): State<AppState>, auth: AuthenticatedUser) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match pool_use_cases(db).list_mine(&auth.user_id).await {
        Ok(pools) => Json(PoolsListResponse {
            pools: pools.iter().map(PoolResponse::from_pool).collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreatePoolRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match pool_use_cases(db).create(&auth.user_id, request.into()).await {
        Ok(pool) => (
            StatusCode::CREATED,
            Json(PoolResponse::from_pool(&pool)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}

async fn join(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<JoinPoolRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match pool_use_cases(db).join_by_code(&auth.user_id, request.into()).await {
        Ok(outcome) => Json(JoinPoolResponse::from_outcome(&outcome)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn detail(State(state): State<AppState>, member: PoolMember) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match pool_use_cases(db).get(&member.pool_id).await {
        Ok(pool) => Json(PoolResponse::from_pool(&pool)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn update(
    State(state): State<AppState>,
    member: PoolMember,
    Json(request): Json<UpdatePoolRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match pool_use_cases(db)
        .update_settings(&member.pool_id, member.pool_role, request.into())
        .await
    {
        Ok(pool) => Json(PoolResponse::from_pool(&pool)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn delete(State(state): State<AppState>, member: PoolMember) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match pool_use_cases(db).delete(&member.pool_id, member.pool_role).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn leave(State(state): State<AppState>, member: PoolMember) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match pool_use_cases(db).leave(&member.pool_id, &member.user_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn list_members(State(state): State<AppState>, member: PoolMember) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match membership_use_cases(db).list(&member.pool_id).await {
        Ok(members) => Json(PoolMembersListResponse {
            members: members
                .iter()
                .map(PoolMemberResponse::from_member_with_name)
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn remove_member(
    State(state): State<AppState>,
    member: PoolMember,
    Path((_pool_id, member_id)): Path<(String, String)>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match membership_use_cases(db)
        .remove(&member.pool_id, &member_id, member.pool_role)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn change_role(
    State(state): State<AppState>,
    member: PoolMember,
    Path((_pool_id, member_id)): Path<(String, String)>,
    Json(request): Json<ChangeMemberRoleRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match membership_use_cases(db)
        .change_role(&member.pool_id, &member_id, member.pool_role, request.into())
        .await
    {
        Ok(updated) => Json(PoolMemberResponse::from_member(&updated)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn read_rules(State(state): State<AppState>, member: PoolMember) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match scoring_use_cases(db).read(&member.pool_id).await {
        Ok(rules) => Json(ScoringRulesListResponse {
            rules: rules.iter().map(ScoringRuleResponse::from_rule).collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_rules(
    State(state): State<AppState>,
    member: PoolMember,
    Json(request): Json<UpdateScoringRulesRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    let rules = match scoring_use_cases(db.clone())
        .update(&member.pool_id, member.pool_role, request.into())
        .await
    {
        Ok(rules) => rules,
        Err(error) => return app_error(error),
    };

    if let Err(error) = result_use_cases(db).recompute_pool(&member.pool_id).await {
        return app_error(error);
    }

    Json(ScoringRulesListResponse {
        rules: rules.iter().map(ScoringRuleResponse::from_rule).collect(),
    })
    .into_response()
}

async fn ranking(State(state): State<AppState>, auth: AuthenticatedUser, Path(pool_id): Path<String>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match result_use_cases(db).ranking(&pool_id, &auth.user_id).await {
        Ok(ranking) => Json(RankingResponse::from_ranking(&ranking)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn history(State(state): State<AppState>, member: PoolMember) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match result_use_cases(db).history(&member.pool_id, &member.user_id).await {
        Ok(summary) => Json(HistoryResponse::from_summary(&summary)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn member_predictions(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((pool_id, member_id)): Path<(String, String)>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    match result_use_cases(db)
        .member_predictions(&pool_id, &member_id, &auth.user_id)
        .await
    {
        Ok(views) => Json(MemberPredictionsResponse::from_views(&views)).into_response(),
        Err(error) => app_error(error),
    }
}
