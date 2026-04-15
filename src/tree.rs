use std::hash::Hash;

mod typed;

use typed::node::Root;

#[derive(Clone, Debug)]
pub struct Tree<P: Clone + Hash + Eq> {
    root: Option<Root<P>>,
}

impl<P: Hash + Eq + Clone> Default for Tree<P> {
    fn default() -> Self {
        Self { root: None }
    }
}

impl<P: Clone + Hash + Eq> Tree<P> {}

// #[cfg(test)]
// mod test;
