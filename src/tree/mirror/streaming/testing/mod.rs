//! Test-only decorators for protocol, backend, and transport adversity.
//!
//! [`Faulting<P>`] wraps a protocol state and manufactures a genuine semantic
//! violation in one outgoing phase. [`Failing<B>`] wraps its materialized
//! backend and returns a typed source error from one backend operation. A
//! materialized state built on `Failing<B>` may itself be wrapped in
//! `Faulting<P>`, so one test can independently schedule both failure kinds.
//! [`IoPlan`] composes successful fragmentation and delay with typed physical
//! failures around an ordered transport.

mod failing;
mod faulting;
mod quiescence;
mod transport;

pub use failing::{Failing, FailingNode, Failure, Operation};
pub use faulting::Faulting;
pub use quiescence::{Quiescence, run_to_quiescence};
pub use transport::{
    AdversarialRead, AdversarialWrite, FaultUnit as IoFaultUnit, InjectedIo, IoFault, IoPlan,
    IoReport, IoReportHandle, Operation as IoOperation, Side as IoSide, wrap_io,
};
