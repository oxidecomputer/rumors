use std::sync::Arc;

mod node;
mod version;

pub use version::Version;

pub struct Tree<P: Ord> {
    #[allow(dead_code, reason = "Tree::insert is being redesigned")]
    root: Arc<node::Node<P>>,
}

impl<P: Ord> Tree<P> {
    pub fn new() -> Self {
        Self {
            root: Arc::new(node::Node::new()),
        }
    }
}

impl<P: Ord> Default for Tree<P> {
    fn default() -> Self {
        Self::new()
    }
}
