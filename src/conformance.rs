//! Conformance suites for the pieces a deployment implements itself.
//!
//! Enable the `conformance` feature in a dev-dependency to use these suites:
//!
//! - [`link`] checks transport independence, stream completion, and cancellation.
//! - [`bookmark`] checks repeatable loads and complete record replacement.
//!
//! Both accept a deadline factory, so checks run under the application's clock
//! and executor. Each suite documents the obligations its probes cannot verify.

pub mod bookmark;
pub mod link;

/// Race a check against its caller's deadline, naming the check on timeout.
fn timed(
    name: &str,
    deadline: impl Future<Output = ()>,
    check: impl Future<Output = ()>,
) -> impl Future<Output = ()> {
    // Link checks can contain large session futures. Box the check before
    // constructing the race so nesting it does not multiply stack usage.
    let check = Box::pin(check);
    async move {
        match futures::future::select(std::pin::pin!(deadline), check).await {
            futures::future::Either::Left(_) => panic!("conformance: {name} timed out"),
            futures::future::Either::Right(_) => {}
        }
    }
}

// Compiled as this crate's own gate: the storage-backend boundary is
// crate-internal; see the module docs' visibility section.
#[cfg(test)]
pub(crate) mod backend;
