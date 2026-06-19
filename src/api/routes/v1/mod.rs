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

use crate::api::middleware::audit::audit_admin_changes;
use crate::api::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    let audited = Router::new()
        .merge(teams::router())
        .merge(tournaments::router())
        .merge(matches::router())
        .merge(files::admin_router())
        .layer(axum::middleware::from_fn_with_state(
            state,
            audit_admin_changes,
        ));

    Router::new()
        .merge(audited)
        .merge(auth::router())
        .merge(files::public_router())
        .merge(pools::router())
        .merge(predictions::router())
        .merge(notifications::router())
}
