mod list;
mod read;
mod update;
mod set;
mod delete;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::node::Node;

use delete::delete;
use list::list;
use read::read;
use set::set;

pub(crate) fn router(node: Arc<Node>) -> Router {
    Router::new()
        .route("/", get(list).post(set))
        .route("/{key}", get(read).delete(delete))
        .with_state(node)
}
