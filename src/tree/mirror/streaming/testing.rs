//! Test-only decorators for protocol and backend adversity, and the
//! whole-session driver the suites share.
//!
//! [`Faulting<P>`] injects either a greeting lie or a semantic violation in one
//! outgoing reply phase. [`Failing<B>`] returns a typed source error from one
//! backend operation, or misreports a child or grandchild count. The wrappers
//! compose, allowing tests to schedule the two failure sources independently.
//! [`session`] runs a remote session between two replicas over one in-memory
//! pipe.

mod failing;
mod faulting;
pub mod session;

pub use failing::{Failing, FailingNode, Failure, Miscount, Misreport, Operation};
pub use faulting::{Fault, Faulting, GreetingLie, ReplyCorruption};
