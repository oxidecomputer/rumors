//! Canonical bit-stream storage and traversal.
//!
//! A [`Party`](crate::Party) or [`Version`](crate::Version) stores one preorder
//! bit stream. A final marker bit and zero padding make each byte string
//! self-delimiting and canonical, so byte equality is value equality. Decoding
//! validates the complete stream before adopting its bytes. Domain readers and
//! writers traverse this storage without reconstructing trees or exposing bit
//! operations to the tree algorithms.
//!
//! A *walk* is a single left-to-right pass over one or more encoded trees'
//! preorder bit streams, as opposed to a *traversal* of a structure held in
//! memory, such as an explicit stack or a recursive oracle's tree. The crate's
//! docs use the word in this sense, as do
//! [`TickWalk`](crate::version::tick::TickWalk),
//! [`VersionWalk`](crate::version::shape::VersionWalk), and
//! [`PartyWalk`](crate::version::shape::PartyWalk).

mod reader;
pub mod stack;
mod storage;
mod writer;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use reader::ReferenceBitsReader;
pub(crate) use reader::{BitRead, BitsReader};
pub use storage::Bits;
#[cfg(test)]
pub(crate) use writer::bits_writer;
pub use writer::BitsWriter;
