//! Test-only decorators for protocol and backend adversity.
//!
//! [`Faulting<P>`] injects either a greeting lie or a semantic violation in one
//! outgoing reply phase. [`Failing<B>`] returns a typed source error from one
//! backend operation. The wrappers compose, allowing tests to schedule the two
//! failure sources independently.

mod failing;
mod faulting;

pub use failing::{Failing, FailingNode, Failure, Operation};
pub use faulting::{Fault, Faulting, GreetingLie, ReplyCorruption};
