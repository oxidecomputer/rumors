//! The sizes of the futures every session boxes.
//!
//! Every gossip and bootstrap session boxes two futures: the session's own
//! ([`Reconciliation::session`]), and inside it the reconciliation after the
//! greeting exchange ([`Handshaken::descent`]). Boxing moves a future from
//! the stack into its allocation, so every session builds each of these
//! values on the stack and copies all of its bytes, whatever its replicas
//! hold: an identical-replica session pays for the largest state a
//! divergent one can reach.
//!
//! A future's size is the largest set of values it holds across any one of
//! its awaits. Those values are anonymous compiler-built state machines,
//! nested from the session down to each participant's terminal future, so
//! no bound on the total follows from sizes this test can name. The test
//! states the measured sizes instead, as exact baselines: a change to what
//! any future on the session's path holds across an await moves them, and
//! so does a toolchain bump that changes layout. Each move is re-accepted
//! in the commit that causes it, with both sizes in its message.
//!
//! The sizes are a test build's. Test builds wrap session work in the
//! session-budget meter's instrumentation, which production builds compile
//! out, so production's futures are smaller; both test profiles, optimized
//! and not, measure the same sizes on the pinned toolchain. The futures
//! hold pointers, so the baselines are stated for 64-bit targets only.

use std::mem::size_of;

use super::super::{LocalParticipant, Reconciliation, RemoteParticipant};
use crate::tree::mirror::streaming::{Handshaken, Local};

/// The bytes the future `f` returns occupies, read from `f`'s type without
/// calling it.
fn output_size<F, A, Fut>(_: F) -> usize
where
    F: FnOnce(A) -> Fut,
{
    size_of::<Fut>()
}

/// The bytes of a session's two boxed futures.
#[derive(Debug, PartialEq, Eq)]
struct Boxed {
    /// [`Reconciliation::session`]'s future, which holds the greeting
    /// exchange and then waits on the boxed descent.
    session: usize,
    /// [`Handshaken::descent`]'s future: the whole reconciliation after the
    /// greeting exchange.
    descent: usize,
}

/// The measured sizes, in a 64-bit test build.
const MEASURED: Boxed = Boxed {
    session: 1_680,
    descent: 3_784,
};

/// A session's two boxed futures hold exactly their measured bytes.
#[test]
fn boxed_session_futures_keep_their_measured_sizes() {
    let measured = Boxed {
        session: output_size(Reconciliation::session),
        descent: output_size(
            Handshaken::<LocalParticipant, RemoteParticipant<'static>, Local>::descent,
        ),
    };
    assert_eq!(measured, MEASURED);
}
