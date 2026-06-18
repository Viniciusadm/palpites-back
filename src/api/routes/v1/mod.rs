pub mod auth;
pub mod files;
pub mod matches;
pub(crate) mod mod_helpers;
pub mod notifications;
pub mod pools;
pub mod predictions;
pub mod results;
pub mod teams;
pub mod tournaments;

use axum::Router;

use crate::api::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .merge(files::router())
        .merge(teams::router())
        .merge(tournaments::router())
        .merge(matches::router())
        .merge(pools::router())
        .merge(predictions::router())
        .merge(notifications::router())
}
