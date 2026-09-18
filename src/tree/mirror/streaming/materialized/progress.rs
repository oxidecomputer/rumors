//! Ordering instrumentation for the materialized walk.
//!
//! [`Progress`] travels through production code as a zero-sized value. Tests
//! give it an endpoint identity and record the publications needed to check
//! the walk's ordering and wire transcript.

use crate::tree::{
    mirror::streaming::{
        erased::Reply,
        materialized::{Query, Resolution},
    },
    typed::{ErasedPrefix, Prefix, height::Height},
};

#[cfg(test)]
use super::transcript;
#[cfg(test)]
use crate::tree::mirror::streaming::materialized::Resolve;

/// Records one endpoint's progress without adding production state.
#[derive(Clone, Copy)]
pub struct Progress {
    /// Distinguishes this endpoint from its peer in test traces.
    #[cfg(test)]
    work: usize,
}

/// A query-like item with a scope that the trace can record.
pub(super) trait Scoped {
    /// Return the scope's prefix bytes.
    #[cfg(test)]
    fn scope(&self) -> &[u8];
}

/// Typed prefixes carry their own scope bytes.
impl<H: Height> Scoped for Prefix<H> {
    #[cfg(test)]
    fn scope(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// Materialized queries name their scope in `prefix`.
impl<E> Scoped for Query<E> {
    #[cfg(test)]
    fn scope(&self) -> &[u8] {
        self.prefix.as_bytes()
    }
}

/// Record the walk's publication boundaries.
impl Progress {
    /// Begin tracing one materialized endpoint.
    pub fn new() -> Self {
        Self {
            #[cfg(test)]
            work: trace::new_work(),
        }
    }

    /// Record an outgoing wire action before its internal twin.
    pub fn wire(self, _scope: ErasedPrefix) {
        #[cfg(test)]
        trace::record(self.work, _scope.as_bytes(), trace::Kind::Wire);
    }

    /// Record the initiator's root query after its wire action.
    pub fn initial_query<E>(self, _query: &Query<E>) {
        #[cfg(test)]
        trace::record(
            self.work,
            _query.prefix.as_bytes(),
            trace::Kind::InitialQuery,
        );
    }

    /// Record a scope resolution before launching its dependent queries.
    pub fn resolution<E>(self, _resolution: &Resolution<E>) {
        #[cfg(test)]
        trace::record(
            self.work,
            _resolution.prefix.as_bytes(),
            trace::Kind::Resolution {
                pending: pending(&_resolution.resolved),
            },
        );
    }

    /// Record dependent work after the resolution that reserved it.
    pub(super) fn dependent(self, _item: &impl Scoped) {
        #[cfg(test)]
        trace::record(self.work, _item.scope(), trace::Kind::DependentWork);
    }

    /// Record a whole subtree resolved locally after its wire action.
    pub fn ready(self, _scope: ErasedPrefix) {
        #[cfg(test)]
        trace::record(self.work, _scope.as_bytes(), trace::Kind::Ready);
    }

    /// Record a parent summary after all work for its pending slots has begun.
    pub fn parent_resolution<E>(self, _resolution: &Resolution<E>) {
        #[cfg(test)]
        trace::record(
            self.work,
            _resolution.prefix.as_bytes(),
            trace::Kind::ParentResolution {
                pending: pending(&_resolution.resolved),
            },
        );
    }

    /// Record one reply where the response pump publishes it.
    pub fn reply<E>(self, _height: usize, _reply: &Reply<E>) {
        #[cfg(test)]
        transcript::reply(self.work, _height, _reply);
    }
}

/// Count the child resolutions that must arrive from the next stage.
#[cfg(test)]
fn pending<E>(resolved: &[(u8, Resolve<E>)]) -> usize {
    resolved
        .iter()
        .filter(|(_, slot)| matches!(slot, Resolve::Pending))
        .count()
}

#[cfg(test)]
pub use trace::{Event, Kind, Trace, with_trace};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod trace;
