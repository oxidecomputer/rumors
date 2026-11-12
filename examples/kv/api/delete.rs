use std::sync::Arc;

use axum::extract::{self, State};
use axum::http::StatusCode;

use crate::node::Node;
use crate::table::{Key, Update};

use super::update::update;

pub(super) async fn delete(
    State(node): State<Arc<Node>>,
    extract::Path(key): extract::Path<Key>,
) -> Result<StatusCode, (StatusCode, String)> {
    update(&node, &[Update { key, value: None }]).await
}
