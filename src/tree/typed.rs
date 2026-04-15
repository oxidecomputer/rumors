use std::hash::Hash;

pub mod height;
pub mod node;
pub mod path;

mod untyped;

use bytes::Bytes;
pub use height::Height;
pub use node::Node;
pub use path::Path;

pub trait NodeExt<P: Clone + Hash + Eq, H: Height>: Sized {
    fn construct<I>(i: I) -> Option<Self>
    where
        I: IntoIterator<Item = (Path<H>, P, u64, Bytes)>;

    fn delete<I>(self, i: I) -> Option<Self>
    where
        I: IntoIterator<Item = Path<H>>;

    // TODO: interactive synchronization
}
