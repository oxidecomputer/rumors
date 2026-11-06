//! The *wire pools*: the bytes a session's supply runs may hold on their
//! way through the pipe, counted per direction.
//!
//! A supply run carries the user's records, and nothing bounds how many runs
//! a reply supplies, so runs cannot be priced per question or per logical
//! stream. Each direction of a session instead holds one pool of bytes, sized
//! at [`WIRE_POOL_RUNS`] runs' worth at the peer's own message-size target,
//! and a run holds a *permit* for its bytes from before they exist until they
//! are gone:
//!
//! - sending, the encoder takes a whole run's worth before it builds a run
//!   and gives back what the run did not use once it closes; the multiplexer
//!   drops the rest with the run once it has written it, or copied it into
//!   its write buffer;
//! - receiving, the frame reader takes the run's exact length once the run's
//!   heads declare it, before it reserves the body's buffer at exactly that
//!   length and reads into it, never reallocating it; the decoder drops it
//!   with the run once it has copied the run's last record into its leaf.
//!
//! So many small runs can be in flight at once, and large runs are limited
//! to the pool. A permit also covers the channel node that carries its run
//! between the pipe and the stage, so the pools hold every byte a run
//! occupies but the multiplexer's fixed write buffer. A record larger than
//! the pool takes the whole pool, in one acquisition, and its buffer holds
//! exactly its own bytes: a session exceeds its bound by at most that one
//! record's excess in each direction.
//!
//! Each pool is a Tokio [`Semaphore`], which serves acquisitions in the
//! order they arrive ([`WirePool::acquire`]). One permit stands for
//! [`PERMIT_BYTES`] bytes, the least power of two at which the largest pool
//! any target can size counts within both what one acquisition takes, a
//! `u32`, and what a semaphore holds ([`Semaphore::MAX_PERMITS`]); every
//! acquisition rounds its bytes up to whole permits, and the pool's size is
//! counted at whole permits too, so the bytes a pool stands for bound the
//! bytes its permits cover.

#[cfg(test)]
use std::cell::Cell;
use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

use crate::tree::mirror::streaming::ledger::{max, min, queued};

use super::super::codec::{MAX_RUN_BUDGET_BYTES, SUPPLY_FRAME_OVERHEAD};
use super::{Inbound, Outbound};

/// Runs' worth of bytes each direction's pool holds at the peer's own
/// message-size target: one run being written or read while the next is
/// built or decoded.
///
/// One is enough for every session to complete: a permit is held only by a
/// run already built or already being read, and each such run leaves the
/// pipe without waiting on the walk (the pipe module's wait table). The
/// second lets an encoder build its next run while the multiplexer writes
/// the last, and the reader read a run while a decoder decodes the last;
/// the pipe moves one frame at a time, so a third has nothing to overlap.
pub(crate) const WIRE_POOL_RUNS: usize = 2;

/// The largest channel node a permit covers: one carrying an outgoing run
/// to the multiplexer, or an incoming one to its decoder.
const LARGEST_NODE: usize = max(queued::<Outbound>(), queued::<Inbound>());

/// The most bytes any pool stands for: [`WIRE_POOL_RUNS`] runs at the
/// largest target, [`MAX_RUN_BUDGET_BYTES`], each with its node, counted in
/// a `u64` so that it does not wrap on targets whose `usize` is narrower.
const LARGEST_POOL: u64 = WIRE_POOL_RUNS as u64
    * ((MAX_RUN_BUDGET_BYTES - SUPPLY_FRAME_OVERHEAD) as u64 + LARGEST_NODE as u64);

/// The bytes one permit stands for: the least power of two at which the
/// largest pool counts within both one acquisition's `u32` and a
/// semaphore's [`MAX_PERMITS`](Semaphore::MAX_PERMITS).
///
/// On 64-bit targets the acquisition's `u32` is the tighter limit, and a
/// permit stands for a few bytes, so rounding each run up to whole permits
/// costs it at most that much beside its bytes.
pub(crate) const PERMIT_BYTES: usize = {
    let limit = min(u32::MAX as usize, Semaphore::MAX_PERMITS) as u64;
    let mut bytes: u64 = 1;
    while LARGEST_POOL.div_ceil(bytes) > limit {
        bytes *= 2;
    }
    bytes as usize
};

/// The whole permits that cover `bytes`, at most `u32::MAX`.
const fn permits(bytes: usize) -> u32 {
    let permits = bytes.div_ceil(PERMIT_BYTES);
    if permits > u32::MAX as usize {
        u32::MAX
    } else {
        permits as u32
    }
}

/// The bytes a pool of `runs` runs' worth at the message-size target
/// `target` stands for, each run's channel node `node` bytes: whole
/// permits' worth, saturating at `usize::MAX`.
///
/// A run holds at most the run body a frame within the target carries,
/// the target less [`SUPPLY_FRAME_OVERHEAD`], beside its node. A target
/// past [`MAX_RUN_BUDGET_BYTES`] counts as that, as a session's run budget
/// does. On targets whose `usize` cannot count the largest pools, the
/// count saturates, and so does every sum it enters.
const fn pool_bytes(target: usize, runs: usize, node: usize) -> usize {
    let body = min(target, MAX_RUN_BUDGET_BYTES).saturating_sub(SUPPLY_FRAME_OVERHEAD);
    let run = body.saturating_add(node);
    (permits(run.saturating_mul(runs)) as usize).saturating_mul(PERMIT_BYTES)
}

/// The bytes the two wire pools of a session at the peer's own
/// message-size target `target` stand for together: the send pool's and
/// the receive pool's.
pub(crate) const fn wire_pools_bytes(target: usize) -> usize {
    pool_bytes(target, WIRE_POOL_RUNS, queued::<Outbound>()).saturating_add(pool_bytes(
        target,
        WIRE_POOL_RUNS,
        queued::<Inbound>(),
    ))
}

// Clippy's `missing_const_for_thread_local` can reject const-block
// initializers on targets that lower `thread_local!` through fallback TLS.
// The allow keeps `-D warnings` clean on those targets.
#[cfg(test)]
std::thread_local! {
    /// How many acquisitions from a receive pool on this thread found too
    /// few permits free and had to wait.
    #[allow(clippy::missing_const_for_thread_local)]
    static RECEIVE_WAITS: Cell<usize> = const { Cell::new(0) };
}

/// How many acquisitions from a receive pool on this thread have had to
/// wait for permits: evidence that a test drove its receive pool dry.
///
/// The closed-world poller runs both endpoints of a session on the test's
/// thread, so the count covers exactly the sessions the test polled.
#[cfg(test)]
pub(crate) fn receive_waits() -> usize {
    RECEIVE_WAITS.with(Cell::get)
}

/// One direction's wire pool: a semaphore of whole permits, and the size
/// of the channel node each run's permit also covers.
///
/// Clones share the pool.
#[derive(Clone)]
pub(crate) struct WirePool {
    /// The pool's permits.
    permits: Arc<Semaphore>,
    /// The permits the whole pool holds, which no acquisition exceeds.
    size: u32,
    /// The bytes of the channel node carrying each run.
    node: usize,
    /// Whether this is a receive pool, whose waits [`receive_waits`]
    /// counts.
    #[cfg(test)]
    receiving: bool,
}

impl WirePool {
    /// The pool for runs on their way to the multiplexer: `runs` runs'
    /// worth at the message-size target `target`.
    ///
    /// # Panics
    ///
    /// Panics if `runs` is zero: a programmer error, since no run could
    /// then be built.
    pub(crate) fn sending(target: usize, runs: usize) -> Self {
        Self::new(target, runs, queued::<Outbound>(), false)
    }

    /// The pool for runs on their way from the demultiplexer to their
    /// decoders: `runs` runs' worth at the message-size target `target`.
    ///
    /// # Panics
    ///
    /// As [`sending`](Self::sending).
    pub(crate) fn receiving(target: usize, runs: usize) -> Self {
        Self::new(target, runs, queued::<Inbound>(), true)
    }

    /// The pool of `runs` runs' worth at `target`, each run's node `node`
    /// bytes, for runs received if `receiving`.
    #[cfg_attr(not(test), expect(unused_variables))]
    fn new(target: usize, runs: usize, node: usize, receiving: bool) -> Self {
        assert!(runs > 0, "a wire pool holds at least one run's worth");
        let size = permits(pool_bytes(target, runs, node));
        Self {
            permits: Arc::new(Semaphore::new(size as usize)),
            size,
            node,
            #[cfg(test)]
            receiving,
        }
    }

    /// Wait for a permit covering a run of `run` bytes and its node, or the
    /// whole pool if that is less.
    ///
    /// Each acquisition is one request, and the semaphore serves requests
    /// in the order they arrive: the first waiting request collects the
    /// permits that return until it has all it asked for, and no later one
    /// takes any before it completes. So waiting runs never divide the pool
    /// between them, and a run larger than the pool takes the whole pool,
    /// which every run then in flight returns without waiting on the pool.
    ///
    /// A request the pool can serve at once is served without polling the
    /// semaphore, which would spend the task's cooperative budget in
    /// Tokio: a session runs its tasks inside one future, so every unit
    /// spent brings sooner the moment its transport's next operation
    /// returns `Pending` for the budget alone, which the multiplexer could
    /// not tell from a full write half. Only a request that must wait polls
    /// the semaphore, and a request waiting first takes every free permit,
    /// so serving at once never passes a waiting request.
    ///
    /// # Cancel safety
    ///
    /// Cancel safe: a dropped acquisition holds nothing.
    pub(crate) async fn acquire(&self, run: usize) -> WirePermit {
        let wanted = permits(run.saturating_add(self.node)).min(self.size);
        let permits = Arc::clone(&self.permits);
        let permit = match permits.clone().try_acquire_many_owned(wanted) {
            Ok(permit) => permit,
            Err(TryAcquireError::NoPermits) => {
                #[cfg(test)]
                if self.receiving {
                    RECEIVE_WAITS.with(|waits| waits.set(waits.get() + 1));
                }
                permits
                    .acquire_many_owned(wanted)
                    .await
                    .expect("no wire pool's semaphore is ever closed")
            }
            Err(TryAcquireError::Closed) => unreachable!("no wire pool's semaphore is ever closed"),
        };
        WirePermit(Some(permit))
    }

    /// Return what `permit` holds beyond a run of `run` bytes and its node.
    ///
    /// The encoder takes a whole run's worth before it builds a run, and
    /// keeps only what the built run holds once it closes.
    pub(crate) fn keep(&self, permit: &mut WirePermit, run: usize) {
        let Some(held) = &mut permit.0 else {
            return;
        };
        let needed = permits(run.saturating_add(self.node)) as usize;
        if let Some(excess) = held.num_permits().checked_sub(needed)
            && excess > 0
        {
            drop(held.split(excess));
        }
    }

    /// The bytes the whole pool stands for.
    #[cfg(test)]
    pub(crate) fn bytes(&self) -> usize {
        self.size as usize * PERMIT_BYTES
    }

    /// The bytes the pool's permits not held by any run stand for.
    #[cfg(test)]
    pub(crate) fn available(&self) -> usize {
        self.permits.available_permits() * PERMIT_BYTES
    }
}

/// A run's hold on its direction's wire pool, which returns its bytes when
/// dropped; the [`Default`] permit, which frames that are not runs carry,
/// holds nothing.
#[derive(Debug, Default)]
pub(crate) struct WirePermit(Option<OwnedSemaphorePermit>);

impl WirePermit {
    /// The bytes this permit holds.
    #[cfg(test)]
    pub(crate) fn bytes(&self) -> usize {
        self.0.as_ref().map_or(0, OwnedSemaphorePermit::num_permits) * PERMIT_BYTES
    }
}

/// A session's two wire pools: one for the runs it sends, one for the runs
/// it receives.
pub(crate) struct WirePools {
    /// The pool of the runs on their way to the multiplexer.
    pub(crate) sending: WirePool,
    /// The pool of the runs on their way from the demultiplexer to their
    /// decoders.
    pub(crate) receiving: WirePool,
}

impl WirePools {
    /// Both pools of a session, each of `runs` runs' worth at the peer's own
    /// message-size target, `target`, which bounds every run in either
    /// direction ([`wire_pools_bytes`] at [`WIRE_POOL_RUNS`]).
    ///
    /// The greeting settles a session's run budget as the smaller of the two
    /// endpoints' targets, for both directions, the encoder flushes every
    /// run within it, and the decoder rejects a run beyond it unless the run
    /// is one record. So no run but a lone record outgrows the body a frame
    /// within this side's own target carries, which is known before the
    /// handshake, when the memory budget sets the pools aside; and a lone
    /// record takes at most the whole pool.
    ///
    /// # Panics
    ///
    /// As [`WirePool::sending`].
    pub(crate) fn new(target: usize, runs: usize) -> Self {
        Self {
            sending: WirePool::sending(target, runs),
            receiving: WirePool::receiving(target, runs),
        }
    }
}

#[cfg(test)]
mod tests;
