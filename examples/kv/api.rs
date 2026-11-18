mod list;
mod read;
mod update;
mod insert;
mod remove;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::node::Node;

use insert::insert;
use list::list;
use read::read;
use remove::remove;

pub fn router(node: Arc<Node>) -> Router {
    Router::new()
        .route("/", get(list).post(insert))
        .route("/{key}", get(read).delete(remove))
        .with_state(node)
}
