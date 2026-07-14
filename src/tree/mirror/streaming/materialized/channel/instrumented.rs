use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::task::{Context, Poll};

use futures::{Stream, future::poll_fn};
use tokio::sync::mpsc;

use super::{QueueKind, QueueRole};

/// Aggregated observations for every channel created with one role.
#[derive(Clone, Copy, Debug, Default)]
pub struct RoleStats {
    pub channels: usize,
    pub effective_capacity: usize,
    pub sends: usize,
    pub receives: usize,
    pub blocked_send_polls: usize,
    pub high_water: usize,
}

/// Observations collected during one test session.
#[derive(Debug, Default)]
pub struct ChannelReport(BTreeMap<QueueRole, RoleStats>);

impl ChannelReport {
    /// Return the aggregated statistics for `role`.
    pub fn role(&self, role: QueueRole) -> RoleStats {
        self.0.get(&role).copied().unwrap_or_default()
    }

    /// Aggregate statistics for `kind` over all instantiated heights.
    pub fn kind(&self, kind: QueueKind) -> RoleStats {
        self.0.iter().filter(|(role, _)| role.kind == kind).fold(
            RoleStats::default(),
            |mut total, (_, stats)| {
                total.channels += stats.channels;
                total.effective_capacity = total.effective_capacity.max(stats.effective_capacity);
                total.sends += stats.sends;
                total.receives += stats.receives;
                total.blocked_send_polls += stats.blocked_send_polls;
                total.high_water = total.high_water.max(stats.high_water);
                total
            },
        )
    }

    /// Iterate over each observed role and typed height.
    pub fn roles(&self) -> impl Iterator<Item = (QueueRole, RoleStats)> + '_ {
        self.0.iter().map(|(role, stats)| (*role, *stats))
    }
}

struct Stats {
    role: QueueRole,
    effective_capacity: usize,
    sends: AtomicUsize,
    receives: AtomicUsize,
    blocked_send_polls: AtomicUsize,
    occupancy: AtomicUsize,
    high_water: AtomicUsize,
}

impl Stats {
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

    fn sent(&self) {
        self.sends.fetch_add(1, Ordering::Relaxed);
        let occupancy = self.occupancy.fetch_add(1, Ordering::Relaxed) + 1;
        self.high_water.fetch_max(occupancy, Ordering::Relaxed);
    }

    fn received(&self) {
        self.receives.fetch_add(1, Ordering::Relaxed);
        self.occupancy.fetch_sub(1, Ordering::Relaxed);
    }

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
    inner: mpsc::Sender<T>,
    stats: Arc<Stats>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            stats: self.stats.clone(),
        }
    }
}

impl<T> Sender<T> {
    /// Send one item, applying a scheduled suspension before every poll.
    pub async fn send(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
        let mut sending = Box::pin(self.inner.send(item));
        let mut delay = None;
        let result = poll_fn(|cx| {
            if delay.is_none() {
                delay = Some(next_delay());
            }
            if delay.is_some_and(|delay| delay > 0) {
                delay = delay.map(|delay| delay - 1);
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            delay = None;
            if self.inner.capacity() == 0 {
                self.stats
                    .blocked_send_polls
                    .fetch_add(1, Ordering::Relaxed);
            }
            sending.as_mut().poll(cx)
        })
        .await;
        if result.is_ok() {
            self.stats.sent();
        }
        result
    }
}

/// The receiving half of a bounded channel.
pub struct Receiver<T> {
    inner: mpsc::Receiver<T>,
    stats: Arc<Stats>,
    delay: Option<u8>,
}

impl<T> Receiver<T> {
    /// Receive one item after the scheduled suspension points.
    pub async fn recv(&mut self) -> Option<T> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        if self.delay.is_none() {
            self.delay = Some(next_delay());
        }
        if self.delay.is_some_and(|delay| delay > 0) {
            self.delay = self.delay.map(|delay| delay - 1);
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.delay = None;
        let polled = Pin::new(&mut self.inner).poll_recv(cx);
        if matches!(polled, Poll::Ready(Some(_))) {
            self.stats.received();
        }
        polled
    }
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}

/// Create a Tokio channel with test-only capacity and observation hooks.
pub fn channel<T>(role: QueueRole, capacity: usize) -> (Sender<T>, Receiver<T>) {
    let effective_capacity = limit_capacity(role, capacity);
    let stats = Arc::new(Stats::new(role, effective_capacity));
    OBSERVATIONS.with(|observations| {
        if let Some(observations) = observations.borrow_mut().as_mut() {
            observations.push(stats.clone());
        }
    });
    let (sender, receiver) = mpsc::channel(effective_capacity);
    (
        Sender {
            inner: sender,
            stats: stats.clone(),
        },
        Receiver {
            inner: receiver,
            stats,
            delay: None,
        },
    )
}

struct Schedule {
    delays: Vec<u8>,
    step: usize,
}

std::thread_local! {
    static SCHEDULE: RefCell<Option<Schedule>> = const { RefCell::new(None) };
    static CAPACITY_LIMIT: RefCell<Option<usize>> = const { RefCell::new(None) };
    static ROLE_CAPACITIES: RefCell<BTreeMap<QueueRole, usize>> = const { RefCell::new(BTreeMap::new()) };
    static KIND_CAPACITIES: RefCell<BTreeMap<QueueKind, usize>> = const { RefCell::new(BTreeMap::new()) };
    static OBSERVATIONS: RefCell<Option<Vec<Arc<Stats>>>> = const { RefCell::new(None) };
}

/// Run `f` with an explicit sequence of delays at channel poll boundaries.
pub fn with_schedule<R>(delays: Vec<u8>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Schedule>);

    impl Drop for Restore {
        fn drop(&mut self) {
            SCHEDULE.with(|schedule| schedule.replace(self.0.take()));
        }
    }

    let previous = SCHEDULE.with(|schedule| schedule.replace(Some(Schedule { delays, step: 0 })));
    let _restore = Restore(previous);
    f()
}

/// Run `f` with every requested channel capacity capped at `limit`.
pub fn with_capacity_limit<R>(limit: usize, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<usize>);

    impl Drop for Restore {
        fn drop(&mut self) {
            CAPACITY_LIMIT.with(|capacity| capacity.replace(self.0.take()));
        }
    }

    assert!(limit > 0);
    let previous = CAPACITY_LIMIT.with(|capacity| capacity.replace(Some(limit)));
    let _restore = Restore(previous);
    f()
}

/// Run `f` with one named queue's requested capacity capped at `limit`.
pub fn with_role_capacity<R>(role: QueueRole, limit: usize, f: impl FnOnce() -> R) -> R {
    struct Restore {
        role: QueueRole,
        previous: Option<usize>,
    }

    impl Drop for Restore {
        fn drop(&mut self) {
            ROLE_CAPACITIES.with(|capacities| {
                let mut capacities = capacities.borrow_mut();
                match self.previous {
                    Some(previous) => {
                        capacities.insert(self.role, previous);
                    }
                    None => {
                        capacities.remove(&self.role);
                    }
                }
            });
        }
    }

    assert!(limit > 0);
    let previous = ROLE_CAPACITIES.with(|capacities| capacities.borrow_mut().insert(role, limit));
    let _restore = Restore { role, previous };
    f()
}

/// Run `f` with every height of one queue kind capped at `limit`.
pub fn with_kind_capacity<R>(kind: QueueKind, limit: usize, f: impl FnOnce() -> R) -> R {
    struct Restore {
        kind: QueueKind,
        previous: Option<usize>,
    }

    impl Drop for Restore {
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
    struct Restore(Option<Vec<Arc<Stats>>>);

    impl Drop for Restore {
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
        role.channels += snapshot.channels;
        role.effective_capacity = role.effective_capacity.max(snapshot.effective_capacity);
        role.sends += snapshot.sends;
        role.receives += snapshot.receives;
        role.blocked_send_polls += snapshot.blocked_send_polls;
        role.high_water = role.high_water.max(snapshot.high_water);
    }
    (result, report)
}

fn limit_capacity(role: QueueRole, capacity: usize) -> usize {
    let capacity = CAPACITY_LIMIT
        .with(|limit| (*limit.borrow()).map_or(capacity, |limit| capacity.min(limit)));
    ROLE_CAPACITIES.with(|capacities| {
        let capacity = capacities
            .borrow()
            .get(&role)
            .map_or(capacity, |limit| capacity.min(*limit));
        KIND_CAPACITIES.with(|capacities| {
            capacities
                .borrow()
                .get(&role.kind)
                .map_or(capacity, |limit| capacity.min(*limit))
        })
    })
}

fn next_delay() -> u8 {
    SCHEDULE.with(|schedule| {
        let mut schedule = schedule.borrow_mut();
        let Some(current) = schedule.as_mut() else {
            return 0;
        };
        let delay = current.delays.get(current.step).copied().unwrap_or(0);
        current.step += 1;
        delay.min(2)
    })
}
