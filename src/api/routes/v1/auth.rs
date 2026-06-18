use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::api::dto::auth::{LoginRequest, LoginResponse, MeResponse, MeUser, RegisterRequest};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::auth::{AuthUseCases, UserRepository};
use crate::infrastructure::auth::{Argon2PasswordHasher, JwtTokenService};
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/register", post(register))
        .route("/auth/me", get(me))
}

async fn me(State(state): State<AppState>, auth: AuthenticatedUser) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let users = MySqlUserRepository::new(db);

    let user = match users.find_by_id(&auth.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return app_error(crate::errors::AppError::NotFound(
                "user was not found".to_owned(),
            ))
        }
        Err(error) => return app_error(error),
    };

    Json(MeResponse {
        user: MeUser {
            id: user.id.as_str().to_owned(),
            display_name: user.display_name.as_str().to_owned(),
            email: user.email.as_str().to_owned(),
            role: user.role.as_str().to_owned(),
        },
    })
    .into_response()
}

async fn login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let use_cases = AuthUseCases::new(
        MySqlUserRepository::new(db),
        Argon2PasswordHasher,
        JwtTokenService::new(state.jwt_secret),
    );

    match use_cases.login(request.into()).await {
        Ok(session) => Json(LoginResponse {
            access_token: session.access_token,
            token_type: "Bearer".to_owned(),
            user_id: session.user_id,
            display_name: session.display_name,
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn register(State(state): State<AppState>, Json(request): Json<RegisterRequest>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let use_cases = AuthUseCases::new(
        MySqlUserRepository::new(db),
        Argon2PasswordHasher,
        JwtTokenService::new(state.jwt_secret),
    );

    match use_cases.register(request.into()).await {
        Ok(session) => Json(LoginResponse {
            access_token: session.access_token,
            token_type: "Bearer".to_owned(),
            user_id: session.user_id,
            display_name: session.display_name,
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}
