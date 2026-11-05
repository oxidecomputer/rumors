//! A conformance suite for the piece of persistence a deployment implements
//! itself: the bookmark store.
//!
//! Enable the `conformance` feature in a dev-dependency to use [`bookmark`],
//! which checks repeatable loads and complete record replacement. The suite
//! accepts a deadline factory, so checks run under the application's clock
//! and executor. It documents the obligations its probes cannot verify.

pub mod bookmark;

/// Race a check against its caller's deadline, naming the check on timeout.
fn timed(
    name: &str,
    deadline: impl Future<Output = ()>,
    check: impl Future<Output = ()>,
) -> impl Future<Output = ()> {
    // Checks can contain large futures. Box the check before constructing
    // the race so nesting it does not multiply stack usage.
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
