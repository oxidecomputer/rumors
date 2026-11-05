//! Test-only ledger of the parking queues a session used, on this thread.
//!
//! Each side sizes its parking queues from its own window. This ledger
//! records each parking queue's derived capacity, one queue per height and
//! side, so a session test can judge which window sized it. The parking task
//! runs on the session's own thread, so a thread-local ledger sees every
//! park of a session driven there.

use std::cell::RefCell;
use std::collections::BTreeMap;

use super::Lane;
use crate::tree::mirror::streaming::remote::codec::Speaker;

/// One parking queue a session parked a reply in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Peak {
    /// The typed height of the replies parked there.
    pub(crate) height: usize,
    /// The local speaker of the side that parked there.
    pub(crate) owner: Speaker,
    /// The queue's derived capacity
    /// ([`parking_capacity`](super::queues::parking_capacity)).
    pub(crate) capacity: usize,
}

// clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
// fallback-TLS lowering (illumos among the gate's targets) and denies an
// initializer that already sits in a `const` block; the allow keeps
// `-D warnings` passing on every platform the gate runs.
thread_local! {
    /// The derived capacity of each queue the session this thread is
    /// driving parked in, by height and by whether the parking side is the
    /// initiator: both sides park leaf replies, so a height alone does not
    /// name one queue.
    #[allow(clippy::missing_const_for_thread_local)]
    static LEDGER: RefCell<BTreeMap<(usize, bool), usize>> = const { RefCell::new(BTreeMap::new()) };
}

/// Record a reply parked in `lane`.
pub(super) fn parked(lane: Lane) {
    LEDGER.with_borrow_mut(|ledger| {
        ledger.insert(
            (lane.height, lane.local == Speaker::Initiator),
            lane.capacity,
        );
    });
}

/// Run `session` on a fresh ledger, and return each parking queue it used
/// beside its result.
pub(crate) fn with_census<T>(session: impl FnOnce() -> T) -> (T, Vec<Peak>) {
    LEDGER.with_borrow_mut(BTreeMap::clear);
    let result = session();
    let peaks = LEDGER.with_borrow_mut(|ledger| {
        std::mem::take(ledger)
            .into_iter()
            .map(|((height, initiator), capacity)| Peak {
                height,
                owner: if initiator {
                    Speaker::Initiator
                } else {
                    Speaker::Responder
                },
                capacity,
            })
            .collect()
    });
    (result, peaks)
}
