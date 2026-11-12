use std::sync::Arc;

use axum::extract::{self, State};
use axum::http::StatusCode;

use crate::node::Node;
use crate::table::{Key, Value};

pub(super) async fn read(
    State(node): State<Arc<Node>>,
    extract::Path(key): extract::Path<Key>,
) -> Result<String, StatusCode> {
    node.get(&key)
        .map(Value::into_inner)
        .ok_or(StatusCode::NOT_FOUND)
}
