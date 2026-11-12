use axum::{Router, routing::get};

fn router(node: Arc<Node>) -> Router {
    Router::new()
        .route("/", get(list).post(set))
        .route("/{key}", get(read).delete(delete))
        .with_state(node)
}
