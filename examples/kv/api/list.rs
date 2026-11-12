use axum::extract::{self, State};
use axum::{Json, http::StatusCode};

async fn list(State(node): State<Arc<Node>>) -> Json<BTreeMap<Key, Value>> {
    Json(node.entries())
}
