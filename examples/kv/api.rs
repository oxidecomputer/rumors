use std::{collections::BTreeMap, sync::Arc};

use axum::extract::{Path, State};
use axum::{Json, http::StatusCode};

use crate::node::Node;
use crate::table::{Key, Value};

async fn list(State(node): State<Arc<Node>>) -> Json<BTreeMap<Key, Value>> {
    Json(node.entries())
}

async fn read(State(node): State<Arc<Node>>, Path(key): Path<Key>) -> Result<String, StatusCode> {
    node.get(&key)
        .map(Value::into_inner)
        .ok_or(StatusCode::NOT_FOUND)
}

use axum::Form;

use crate::table::Update;

async fn update(node: &Node, updates: &[Update]) -> Result<StatusCode, (StatusCode, String)> {
    match node.update(updates).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}"))),
    }
}

async fn set(
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

async fn delete(
    State(node): State<Arc<Node>>,
    Path(key): Path<Key>,
) -> Result<StatusCode, (StatusCode, String)> {
    update(&node, &[Update { key, value: None }]).await
}

use axum::{Router, routing::get};

pub fn router(node: Arc<Node>) -> Router {
    Router::new()
        .route("/", get(list).post(set))
        .route("/{key}", get(read).delete(delete))
        .with_state(node)
}
