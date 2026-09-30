//! Operations on parties and their encoded trees.
//!
//! Whole-party operations are associated with [`Party`](super::Party), which
//! guarantees a complete canonical tree. Cursors appear only inside walks that
//! must address a subtree. The operation modules keep those walks iterative
//! and normalize every tree they build.

mod build;
mod compare;
mod fork;
mod join;
mod sync;
mod without;
