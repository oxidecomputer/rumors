use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use crate::node::Node;
use crate::table::{Key, Value};

pub(super) async fn list(State(node): State<Arc<Node>>) -> Json<BTreeMap<Key, Value>> {
    Json(node.entries())
}
