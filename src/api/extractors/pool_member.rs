use std::collections::HashMap;

use axum::async_trait;
use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::Json;

use crate::api::dto::common::ErrorResponse;
use crate::api::extractors::AuthenticatedUser;
use crate::api::state::AppState;
use crate::application::pools::PoolMemberRepository;
use crate::domain::pools::{MemberStatus, PoolRole};
use crate::infrastructure::repositories::mysql_pool_members::MySqlPoolMemberRepository;

#[derive(Debug, Clone)]
pub struct PoolMember {
    pub user_id: String,
    pub pool_id: String,
    pub pool_role: PoolRole,
}

type Rejection = (StatusCode, Json<ErrorResponse>);

#[async_trait]
impl FromRequestParts<AppState> for PoolMember {
    type Rejection = Rejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = AuthenticatedUser::from_request_parts(parts, state).await?;

        let Path(params) = Path::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|_| forbidden("you are not a member of this pool"))?;
        let pool_id = params
            .get("id")
            .cloned()
            .ok_or_else(|| forbidden("you are not a member of this pool"))?;

        let db = state.db().map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    code: "configuration_error".to_owned(),
                    message: "database is not configured".to_owned(),
                }),
            )
        })?;

        let membership = MySqlPoolMemberRepository::new(db)
            .find_membership(&pool_id, &auth.user_id)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        code: "internal_error".to_owned(),
                        message: "could not resolve pool membership".to_owned(),
                    }),
                )
            })?
            .filter(|member| member.status == MemberStatus::Active)
            .ok_or_else(|| forbidden("you are not a member of this pool"))?;

        Ok(Self {
            user_id: auth.user_id,
            pool_id,
            pool_role: membership.role,
        })
    }
}

fn forbidden(message: &str) -> Rejection {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            code: "forbidden".to_owned(),
            message: message.to_owned(),
        }),
    )
}
