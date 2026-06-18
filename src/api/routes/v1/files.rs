use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};

use crate::api::authz::require_admin;
use crate::api::dto::files::{FileResponse, LinkFileRequest, UploadFileResponse};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::files::{FileUseCases, LinkUseCases, UploadFile};
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_files::{
    MySqlFileLinkRepository, MySqlFileRepository,
};
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;
use crate::infrastructure::storage;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/files", post(upload))
        .route("/files/:id", get(detail))
        .route("/users/me/avatar", put(link_avatar))
        .route("/teams/:id/flag", put(link_team_flag))
}

async fn upload(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    multipart: Multipart,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    let field = match read_file_field(multipart).await {
        Ok(field) => field,
        Err(error) => return app_error(error),
    };

    let storage = storage::build(&state.file_storage);
    let use_cases = FileUseCases::new(
        MySqlFileRepository::new(db),
        storage,
        state.file_storage.max_byte_size,
    );

    let command = UploadFile {
        owner_user_id: Some(auth.user_id),
        content_type: field.content_type,
        original_name: field.original_name,
        bytes: field.bytes,
    };

    match use_cases.upload(command).await {
        Ok(stored) => (
            StatusCode::CREATED,
            Json(UploadFileResponse::from_stored(&stored)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}

async fn detail(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    let storage = storage::build(&state.file_storage);
    let use_cases = FileUseCases::new(
        MySqlFileRepository::new(db),
        storage,
        state.file_storage.max_byte_size,
    );

    match use_cases.get(&id).await {
        Ok(stored) => Json(FileResponse::from_stored(&stored)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn link_avatar(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<LinkFileRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    let use_cases = LinkUseCases::new(
        MySqlFileRepository::new(db.clone()),
        MySqlFileLinkRepository::new(db),
    );

    match use_cases
        .link_user_avatar(&auth.user_id, request.into())
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn link_team_flag(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(request): Json<LinkFileRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    if let Err(error) = require_admin(&auth, &MySqlUserRepository::new(db.clone())).await {
        return app_error(error);
    }

    let use_cases = LinkUseCases::new(
        MySqlFileRepository::new(db.clone()),
        MySqlFileLinkRepository::new(db),
    );

    match use_cases.link_team_flag(&id, request.into()).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

struct UploadedField {
    content_type: String,
    original_name: String,
    bytes: Vec<u8>,
}

async fn read_file_field(mut multipart: Multipart) -> Result<UploadedField, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::validation_code("invalid_multipart", error.to_string()))?
    {
        if field.file_name().is_none() {
            continue;
        }
        let original_name = field.file_name().unwrap_or_default().to_owned();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_owned();
        let bytes = field
            .bytes()
            .await
            .map_err(|error| AppError::validation_code("invalid_multipart", error.to_string()))?
            .to_vec();
        return Ok(UploadedField {
            content_type,
            original_name,
            bytes,
        });
    }
    Err(AppError::validation_code(
        "missing_file",
        "no file part was provided in the multipart request",
    ))
}
