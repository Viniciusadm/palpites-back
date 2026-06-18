use palpites_back::api;
use palpites_back::api::state::AppState;

#[test]
fn router_builds_with_pool_routes() {
    let _router = api::router(AppState::new(None, "test-secret", Default::default()));
}
