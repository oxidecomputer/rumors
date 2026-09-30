//! Canonical bit-stream storage and traversal.
//!
//! A party or version stores one preorder bit stream. A final marker bit and
//! zero padding make each byte string self-delimiting and canonical, so byte
//! equality is value equality. Decoding validates the complete stream before
//! adopting its bytes. Domain readers and writers traverse this storage without
//! reconstructing trees or exposing bit operations to the tree algorithms.

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
