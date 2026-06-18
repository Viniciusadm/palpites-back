use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::api::dto::common::ErrorResponse;
use crate::errors::{AppError, CodedKind};

pub fn app_error(error: AppError) -> Response {
    let (status, code) = match &error {
        AppError::Configuration(_) => (StatusCode::SERVICE_UNAVAILABLE, "configuration_error"),
        AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
        AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
        AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
        AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
        AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_error"),
        AppError::Persistence(_) | AppError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
        }
        // Client-facing errors carry their own machine code and HTTP semantics.
        AppError::Coded { kind, code, .. } => {
            let status = match kind {
                CodedKind::Conflict => StatusCode::CONFLICT,
                CodedKind::Forbidden => StatusCode::FORBIDDEN,
                CodedKind::NotFound => StatusCode::NOT_FOUND,
                CodedKind::Validation => StatusCode::UNPROCESSABLE_ENTITY,
            };
            (status, *code)
        }
    };
    (
        status,
        Json(ErrorResponse {
            code: code.to_owned(),
            // The body carries the message without the technical type prefix;
            // `error.to_string()` (with prefix) is reserved for logs/tracing.
            message: error.client_message().to_owned(),
        }),
    )
        .into_response()
}
