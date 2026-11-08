//! Test channels with scheduled polling and per-edge occupancy statistics.
//!
//! These wrappers retain the production channel's behavior while allowing
//! protocol tests to delay polls, cap selected edges, and inspect queue use.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::task::{Context, Poll};

use futures::{Stream, future::poll_fn};

use super::{QueueKind, QueueRole, SendError, TrySendError, bounded};
use crate::testing::schedule::{CHANNEL_SCHEDULE, Countdown};

/// Aggregated observations for every channel created with one role.
#[derive(Clone, Copy, Debug, Default)]
pub struct RoleStats {
    /// Number of channels included in this aggregate.
    pub channels: usize,
    /// Largest configured capacity among those channels after test caps.
    pub effective_capacity: usize,
    /// Items successfully sent.
    pub sends: usize,
    /// Items successfully received.
    pub receives: usize,
    /// Send polls made while the channel had no available capacity.
    pub blocked_send_polls: usize,
    /// Largest observed number of queued items. When a channel's sender and
    /// receiver run on different threads, it can count one more: the item a
    /// send had counted but not yet handed over.
    pub high_water: usize,
}

/// Combine statistics across channels or heights.
impl RoleStats {
    /// Merge another channel aggregate into this one.
    fn absorb(&mut self, other: Self) {
        self.channels += other.channels;
        self.effective_capacity = self.effective_capacity.max(other.effective_capacity);
        self.sends += other.sends;
        self.receives += other.receives;
        self.blocked_send_polls += other.blocked_send_polls;
        self.high_water = self.high_water.max(other.high_water);
    }
}

/// Observations collected during one test session.
#[derive(Debug, Default)]
pub struct ChannelReport(
    /// Statistics grouped by semantic edge and item height.
    BTreeMap<QueueRole, RoleStats>,
);

/// Query observations from one instrumented run.
impl ChannelReport {
    /// Aggregate statistics for `kind` over all instantiated heights.
    pub fn kind(&self, kind: QueueKind) -> RoleStats {
        self.0.iter().filter(|(role, _)| role.kind == kind).fold(
            RoleStats::default(),
            |mut total, (_, stats)| {
                total.absorb(*stats);
                total
            },
        )
    }

    /// Statistics for one role at one typed height.
    ///
    /// # Panics
    ///
    /// Panics if no channel of `role` was observed: a test that reads a role
    /// expects the session to have created it.
    pub fn role(&self, role: QueueRole) -> RoleStats {
        *self
            .0
            .get(&role)
            .unwrap_or_else(|| panic!("no channel observed for {role:?}"))
    }

    /// Iterate over each observed role and typed height.
    pub fn roles(&self) -> impl Iterator<Item = (QueueRole, RoleStats)> + '_ {
        self.0.iter().map(|(role, stats)| (*role, *stats))
    }
}

/// Atomic observations shared by one channel's sender and receiver.
struct Stats {
    /// Semantic edge and item height represented by this channel.
    role: QueueRole,
    /// Capacity after applying any test limit.
    effective_capacity: usize,
    /// Successful sends.
    sends: AtomicUsize,
    /// Successful receives.
    receives: AtomicUsize,
    /// Send polls observed at zero available capacity.
    blocked_send_polls: AtomicUsize,
    /// Items counted into the channel and not yet received.
    ///
    /// A send counts its item before handing it to the channel
    /// ([`Stats::sending`]), so the receiver's count of the same item, made
    /// after it takes the item and on whatever thread it runs, always follows
    /// it, and the occupancy never falls below zero. It is therefore the queued
    /// items, and across threads one more while the channel's one sender is
    /// handing an item over.
    occupancy: AtomicUsize,
    /// Largest observed occupancy: the most queued items, and across threads
    /// possibly one more, the item a send had counted but not yet handed over.
    high_water: AtomicUsize,
}

/// Record and snapshot one channel's observations.
impl Stats {
    /// Start observations for a newly created channel.
    fn new(role: QueueRole, effective_capacity: usize) -> Self {
        Self {
            role,
            effective_capacity,
            sends: AtomicUsize::new(0),
            receives: AtomicUsize::new(0),
            blocked_send_polls: AtomicUsize::new(0),
            occupancy: AtomicUsize::new(0),
            high_water: AtomicUsize::new(0),
        }
    }

    /// Count one item into the channel before the send hands it over, returning
    /// the occupancy that includes it.
    ///
    /// The count must come first: once the item is in the channel, a receiver
    /// on another thread can take it and count it out at once. A send that then
    /// fails takes its count back ([`unsent`](Self::unsent)).
    fn sending(&self) -> usize {
        self.occupancy.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Record one successful send, whose item [`sending`](Self::sending)
    /// counted at `occupancy`.
    fn sent(&self, occupancy: usize) {
        self.sends.fetch_add(1, Ordering::Relaxed);
        self.high_water.fetch_max(occupancy, Ordering::Relaxed);
    }

    /// Take back the count of an item whose send failed.
    fn unsent(&self) {
        self.occupancy.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record one successful receive and its resulting occupancy.
    fn received(&self) {
        self.receives.fetch_add(1, Ordering::Relaxed);
        self.occupancy.fetch_sub(1, Ordering::Relaxed);
    }

    /// Read all counters into an aggregate value.
    fn snapshot(&self) -> RoleStats {
        RoleStats {
            channels: 1,
            effective_capacity: self.effective_capacity,
            sends: self.sends.load(Ordering::Relaxed),
            receives: self.receives.load(Ordering::Relaxed),
            blocked_send_polls: self.blocked_send_polls.load(Ordering::Relaxed),
            high_water: self.high_water.load(Ordering::Relaxed),
        }
    }
}

/// The sending half of a bounded channel.
pub struct Sender<T> {
    /// Underlying production sender.
    inner: bounded::Sender<T>,
    /// Observations shared with the receiver.
    stats: Arc<Stats>,
}

/// Send through an instrumented bounded channel.
impl<T> Sender<T> {
    /// Send one item if the channel has room, without waiting, recording a
    /// successful send.
    ///
    /// Unlike [`send`](Self::send), this applies no scheduled delay: a delay
    /// suspends a poll, and this send is never polled. Schedules still reach
    /// its callers through the receivers they wait on.
    pub fn try_send(&mut self, item: T) -> Result<(), TrySendError<T>> {
        let occupancy = self.stats.sending();
        let result = self.inner.try_send(item);
        match result {
            Ok(()) => self.stats.sent(occupancy),
            Err(_) => self.stats.unsent(),
        }
        result
    }

    /// Send one item, applying a scheduled suspension before every poll.
    pub async fn send(&mut self, item: T) -> Result<(), SendError<T>> {
        self.send_with(item, || ()).await
    }

    /// Send one item as [`send`](Self::send) does, calling `on_full` first if
    /// the channel is full when the send arrives, before any scheduled delay.
    pub async fn send_with(&mut self, item: T, on_full: impl FnOnce()) -> Result<(), SendError<T>> {
        let mut on_full = Some(on_full);
        let mut waited = false;
        let mut delay = Countdown::default();
        let ready = poll_fn(|cx| {
            if let Some(on_full) = on_full.take()
                && self.inner.poll_ready(cx).is_pending()
            {
                waited = true;
                on_full();
            }
            if delay.suspend(|| CHANNEL_SCHEDULE.next(), cx) {
                return Poll::Pending;
            }
            delay.clear();
            let ready = self.inner.poll_ready(cx);
            if ready.is_pending() {
                self.stats
                    .blocked_send_polls
                    .fetch_add(1, Ordering::Relaxed);
            }
            ready
        })
        .await;
        match ready {
            Ok(()) => {
                let occupancy = self.stats.sending();
                let result = self.inner.start_send(item, waited);
                match result {
                    Ok(()) => self.stats.sent(occupancy),
                    Err(_) => self.stats.unsent(),
                }
                result
            }
            Err(SendError(())) => Err(SendError(item)),
        }
    }
}

/// The receiving half of a bounded channel.
pub struct Receiver<T> {
    /// Underlying production receiver.
    inner: bounded::Receiver<T>,
    /// Observations shared with the sender.
    stats: Arc<Stats>,
    /// Scheduled delay before the next receiver poll.
    delay: Countdown,
}

/// Receive through an instrumented bounded channel.
impl<T> Receiver<T> {
    /// Receive one item after the scheduled suspension points.
    pub async fn recv(&mut self) -> Option<T> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    /// Poll the receiver after spending any hold on its role and this poll's
    /// scheduled delay.
    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let held = ROLE_HOLDS.with(|holds| {
            let mut holds = holds.borrow_mut();
            match holds.get_mut(&self.stats.role) {
                Some(remaining) if *remaining > 0 => {
                    *remaining -= 1;
                    true
                }
                _ => false,
            }
        });
        if held {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        if self.delay.suspend(|| CHANNEL_SCHEDULE.next(), cx) {
            return Poll::Pending;
        }
        self.delay.clear();
        let polled = self.inner.poll_recv(cx);
        if matches!(polled, Poll::Ready(Some(_))) {
            self.stats.received();
        }
        polled
    }

    /// Refuse every further item, then take one already queued without waiting,
    /// as the production receiver does; no scheduled delay applies, since
    /// nothing waits.
    pub fn close_and_try_recv(&mut self) -> Option<T> {
        let taken = self.inner.close_and_try_recv();
        if taken.is_some() {
            self.stats.received();
        }
        taken
    }
}

/// Expose the receiver as a stream without changing its instrumentation.
impl<T> Stream for Receiver<T> {
    type Item = T;

    /// Poll for the next received item.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}

/// Create a production channel with test-only capacity and observation hooks.
pub fn channel<T>(role: QueueRole, capacity: usize) -> (Sender<T>, Receiver<T>) {
    let effective_capacity = limit_capacity(role, capacity);
    let stats = Arc::new(Stats::new(role, effective_capacity));
    OBSERVATIONS.with(|observations| {
        if let Some(observations) = observations.borrow_mut().as_mut() {
            observations.push(stats.clone());
        }
    });
    let (sender, receiver) = bounded::channel(effective_capacity);
    let sender = sender.edge(role.kind);
    (
        Sender {
            inner: sender,
            stats: stats.clone(),
        },
        Receiver {
            inner: receiver,
            stats,
            delay: Countdown::default(),
        },
    )
}

// Clippy's `missing_const_for_thread_local` can reject const-block initializers
// on targets that lower `thread_local!` through fallback TLS. The allow keeps
// `-D warnings` clean on those targets.
std::thread_local! {
    /// Per-kind channel capacity limits active on this test thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static KIND_CAPACITIES: RefCell<BTreeMap<QueueKind, usize>> = const { RefCell::new(BTreeMap::new()) };
    /// Channel observations active on this test thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static OBSERVATIONS: RefCell<Option<Vec<Arc<Stats>>>> = const { RefCell::new(None) };
    /// Receiver polls still to be held pending, per queue role.
    #[allow(clippy::missing_const_for_thread_local)]
    static ROLE_HOLDS: RefCell<BTreeMap<QueueRole, usize>> = const { RefCell::new(BTreeMap::new()) };
}

/// Run `f` with an explicit sequence of delays at channel poll boundaries.
pub fn with_schedule<R>(delays: Vec<u8>, f: impl FnOnce() -> R) -> R {
    CHANNEL_SCHEDULE.with(delays, f)
}

/// Run `f` with the receivers of one queue role held pending for their first
/// `polls` polls, counted across every receiver of that role.
///
/// A held receiver wakes itself, so everything else keeps running while its
/// consumer waits: the way a test holds one stage still.
pub fn with_role_hold<R>(role: QueueRole, polls: usize, f: impl FnOnce() -> R) -> R {
    /// Remove the hold when the scope ends or unwinds.
    struct Release(QueueRole);

    /// Removes the role's hold.
    impl Drop for Release {
        /// Forget any polls the hold has left.
        fn drop(&mut self) {
            ROLE_HOLDS.with(|holds| holds.borrow_mut().remove(&self.0));
        }
    }

    ROLE_HOLDS.with(|holds| holds.borrow_mut().insert(role, polls));
    let _release = Release(role);
    f()
}

/// Run `f` with every height of one queue kind capped at `limit`.
pub fn with_kind_capacity<R>(kind: QueueKind, limit: usize, f: impl FnOnce() -> R) -> R {
    /// Restore the previous limit for one queue kind.
    struct Restore {
        /// Queue kind whose limit was replaced.
        kind: QueueKind,
        /// Previous limit, if one was active.
        previous: Option<usize>,
    }

    /// Restore the displaced queue limit.
    impl Drop for Restore {
        /// Reinstate or remove the previous limit.
        fn drop(&mut self) {
            KIND_CAPACITIES.with(|capacities| {
                let mut capacities = capacities.borrow_mut();
                match self.previous {
                    Some(previous) => {
                        capacities.insert(self.kind, previous);
                    }
                    None => {
                        capacities.remove(&self.kind);
                    }
                }
            });
        }
    }

    assert!(limit > 0);
    let previous = KIND_CAPACITIES.with(|capacities| capacities.borrow_mut().insert(kind, limit));
    let _restore = Restore { kind, previous };
    f()
}

/// Run `f` while collecting per-role channel statistics.
pub fn with_observation<R>(f: impl FnOnce() -> R) -> (R, ChannelReport) {
    /// Restore an enclosing observation run after return or panic.
    struct Restore(
        /// Observations displaced by this run.
        Option<Vec<Arc<Stats>>>,
    );

    /// Restore the displaced observation state.
    impl Drop for Restore {
        /// Reinstate the enclosing observation collection.
        fn drop(&mut self) {
            OBSERVATIONS.with(|observations| observations.replace(self.0.take()));
        }
    }

    let previous = OBSERVATIONS.with(|observations| observations.replace(Some(Vec::new())));
    let restore = Restore(previous);
    let result = f();
    let observations = OBSERVATIONS.with(|observations| observations.take().unwrap_or_default());
    drop(restore);

    let mut report = ChannelReport::default();
    for stats in observations {
        let snapshot = stats.snapshot();
        let role = report.0.entry(stats.role).or_default();
        role.absorb(snapshot);
    }
    (result, report)
}

/// Apply any test limit configured for this queue kind.
fn limit_capacity(role: QueueRole, capacity: usize) -> usize {
    KIND_CAPACITIES.with(|capacities| {
        capacities
            .borrow()
            .get(&role.kind)
            .map_or(capacity, |limit| capacity.min(*limit))
    })
}
