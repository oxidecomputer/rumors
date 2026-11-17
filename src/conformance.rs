//! A conformance suite for testing any caller-provided bookmark store.
//!
//! Enable the `conformance` feature in your own crate's dev-dependency to use
//! [`rumors::conformance::bookmark`](crate::conformance::bookmark). This module
//! allows you to check that your own implementation of
//! [`Bookmark`](crate::Bookmark) satisfies the obligations for that trait.

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
// crate-internal; see the module docs' Scope section.
#[cfg(test)]
pub(crate) mod backend;
