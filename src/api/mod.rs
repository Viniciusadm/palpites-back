pub mod authz;
pub mod dto;
pub mod extractors;
pub mod middleware;
pub mod routes;
pub mod state;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use state::AppState;

pub fn router(state: AppState) -> Router {
    Router::<AppState>::new()
        .merge(routes::health::router())
        .nest("/api/v1", routes::v1::router(state.clone()))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
