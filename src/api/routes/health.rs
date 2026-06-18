use axum::{routing::get, Router};

use crate::api::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

async fn health_check() -> &'static str {
    "ok"
}
