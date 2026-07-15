//! Test-only decorators for injecting failures at distinct protocol layers.
//!
//! [`Faulting<P>`] wraps a protocol state and manufactures a genuine semantic
//! violation in one outgoing phase. [`Failing<B>`] wraps its materialized
//! backend and returns a typed source error from one backend operation. A
//! materialized state built on `Failing<B>` may itself be wrapped in
//! `Faulting<P>`, so one test can independently schedule both failure kinds.

mod failing;
mod faulting;

pub use failing::{Failing, FailingNode, Failure, Operation};
pub use faulting::Faulting;
