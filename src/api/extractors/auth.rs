use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::api::dto::common::ErrorResponse;
use crate::api::state::AppState;
use crate::application::auth::TokenVerifier;
use crate::infrastructure::auth::JwtTokenService;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = (axum::http::StatusCode, axum::Json<ErrorResponse>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    axum::Json(ErrorResponse {
                        code: "unauthorized".to_owned(),
                        message: "missing bearer token".to_owned(),
                    }),
                )
            })?;

        let user_id = JwtTokenService::new(state.jwt_secret.clone())
            .verify_access_token(&token)
            .map_err(|error| {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    axum::Json(ErrorResponse {
                        code: "unauthorized".to_owned(),
                        message: error.to_string(),
                    }),
                )
            })?;

        Ok(Self { user_id })
    }
}
