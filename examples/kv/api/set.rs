use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;

use crate::node::Node;
use crate::table::{Key, Update, Value};

use super::update::update;

pub(super) async fn set(
    State(node): State<Arc<Node>>,
    Form(pairs): Form<BTreeMap<Key, Value>>,
) -> Result<StatusCode, (StatusCode, String)> {
    let updates: Vec<Update> = pairs
        .into_iter()
        .map(|(key, value)| Update {
            key,
            value: Some(value),
        })
        .collect();
    update(&node, &updates).await
}
