//! A two-peer session driver over a link that holds every byte until both peers
//! stall.
//!
//! The link stands for a connection of unbounded latency and unbounded
//! buffering. Bytes a peer writes are held, out of the reader's sight, and the
//! writer never waits. When neither peer can progress, the driver asks its
//! schedule how much of each direction's held bytes to release (see
//! [`Release`]), then delivers them, in order, and resumes polling. Each side
//! therefore fills whatever its admission lets it have outstanding before any
//! reply arrives, and each release hands the reader everything its peer
//! produced in the meantime at once. Byte order within a direction is
//! preserved, as TCP preserves it.
//!
//! The schedule also chooses, at each stall, which kinds of each side's tasks
//! to hold back until the next stall ([`Held`]). A session runs its tasks
//! inside one future, so no executor can choose among them, but a backend can:
//! a storage read or write that waits holds back the walk stage or assembler
//! that called it while every other task runs. The driver installs a task pacer
//! ([`pace_tasks`]) that holds back a task the same way, keeping its waker,
//! outside any trace, until its kind is released.
//!
//! Everything runs on the calling thread, polled in a fixed order, so a
//! session's allocator traffic is a deterministic function of the fixture and
//! the schedule.
//!
//! [`drive`] polls the two futures itself rather than spawning them on an
//! executor such as `futures::executor::LocalPool`. Its stall is the one
//! `run_until_stalled` detects, a round in which no future was woken, but it
//! must also poll each peer under that peer's label, which a pool's task loop
//! offers no way to do.
//!
//! # What the measurement sees
//!
//! The driver polls each peer inside [`labelled`] with that peer's
//! [`Side::label`], so a trace attributes every block to the peer whose session
//! allocated it, and [`polling`] tells code running inside a poll which peer it
//! serves. The link's own buffers are harness state, not session state: every
//! operation that can allocate them runs inside [`untraced`], so they reach no
//! trace.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::io;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Wake, Waker};

use alloc_meter::{Label, labelled, untraced};
use rumors::testing::{Holding, TaskRole, pace_tasks};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// The distance between two sides' labels: room for the side's own label and
/// one per [`Holding`] above it.
const SIDE_LABEL_STRIDE: u32 = 16;

// Every holding's label stays below the next side's.
const _: () = assert!(Holding::ALL.len() < SIDE_LABEL_STRIDE as usize);

/// One of the two peers in a driven session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    /// The first peer, whose writes travel toward [`Side::B`].
    A,
    /// The second peer, whose writes travel toward [`Side::A`].
    B,
}

impl Side {
    /// Both sides, in the order the driver polls them.
    pub const BOTH: [Self; 2] = [Self::A, Self::B];

    /// The label the driver polls this side's session under, carried by every
    /// block the session allocates outside a region it attributes to a holding.
    pub const fn label(self) -> Label {
        Label(SIDE_LABEL_STRIDE * (self.index() as u32 + 1))
    }

    /// The label of the blocks this side's session allocates as work for
    /// `holding`.
    pub fn holding_label(self, holding: Holding) -> Label {
        let place = Holding::ALL
            .iter()
            .position(|other| *other == holding)
            .expect("ALL lists every holding");
        Label(self.label().0 + 1 + place as u32)
    }

    /// The side a label belongs to, whatever holding it names.
    pub fn of_label(label: Label) -> Label {
        Label(label.0 / SIDE_LABEL_STRIDE * SIDE_LABEL_STRIDE)
    }

    /// This side's position in [`Side::BOTH`], which also indexes the direction
    /// it writes into.
    pub const fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
        }
    }

    /// The other side.
    pub const fn peer(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

// Clippy's `missing_const_for_thread_local` can reject const-block initializers
// on targets that lower `thread_local!` through fallback TLS. The allow keeps
// `-D warnings` clean on those targets.
thread_local! {
    /// The side whose poll is running on this thread, if any.
    #[allow(clippy::missing_const_for_thread_local)]
    static POLLING: Cell<Option<Side>> = const { Cell::new(None) };
}

/// The side whose session the driver is polling on this thread, or `None`
/// outside [`drive`]'s polls.
pub fn polling() -> Option<Side> {
    POLLING.get()
}

/// Which kinds of one side's tasks a stall holds back until the next stall.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Held {
    /// Walk stages, which take replies, answer the questions in them, and ask
    /// their own, as a slow backend read would hold them.
    pub walk: bool,
    /// Assemblers, as a slow backend write would hold them.
    pub assembly: bool,
    /// The proxy's encoders of this side's replies.
    pub encoder: bool,
    /// The proxy's decoders, which park the peer's replies.
    pub decoder: bool,
}

impl Held {
    /// Holds back nothing.
    pub const NOTHING: Self = Self {
        walk: false,
        assembly: false,
        encoder: false,
        decoder: false,
    };

    /// Whether this holds back a task of `role`.
    fn holds(self, role: TaskRole) -> bool {
        match role {
            TaskRole::Walk { .. } => self.walk,
            TaskRole::Assembly { .. } => self.assembly,
            TaskRole::Encoder => self.encoder,
            TaskRole::Decoder { .. } => self.decoder,
        }
    }
}

/// How many polls the task pacer refused to one side's tasks, by kind.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Refused {
    /// Polls of walk stages held back.
    pub walk: usize,
    /// Polls of assemblers held back.
    pub assembly: usize,
    /// Polls of encoders held back.
    pub encoder: usize,
    /// Polls of decoders held back.
    pub decoder: usize,
}

impl Refused {
    /// Counts one refused poll of a task of `role`.
    fn count(&mut self, role: TaskRole) {
        let count = match role {
            TaskRole::Walk { .. } => &mut self.walk,
            TaskRole::Assembly { .. } => &mut self.assembly,
            TaskRole::Encoder => &mut self.encoder,
            TaskRole::Decoder { .. } => &mut self.decoder,
        };
        *count += 1;
    }
}

/// The task pacer's state on the driving thread.
#[derive(Default)]
struct Pacing {
    /// What each side holds back, by [`Side::index`].
    held: [Held; 2],
    /// Each side's tasks the pacer holds back, with their roles.
    waiting: [Vec<(TaskRole, Waker)>; 2],
    /// The polls the pacer has refused to each side's tasks.
    refused: [Refused; 2],
}

thread_local! {
    /// The task pacer's state, read by [`pace`] during the driver's polls.
    static PACING: RefCell<Pacing> = RefCell::default();
}

/// The task pacer the driver installs: lets a task run unless the side being
/// polled holds back its kind, in which case it keeps the task's waker for the
/// release.
///
/// The waiting list grows outside any trace, so the pacer allocates nothing the
/// measurement sees.
fn pace(role: TaskRole, waker: &Waker) -> bool {
    let Some(side) = polling() else {
        return true;
    };
    PACING.with_borrow_mut(|pacing| {
        if !pacing.held[side.index()].holds(role) {
            return true;
        }
        pacing.refused[side.index()].count(role);
        let waiting = &mut pacing.waiting[side.index()];
        if !waiting
            .iter()
            .any(|(held, kept)| *held == role && kept.will_wake(waker))
        {
            untraced(|| waiting.push((role, waker.clone())));
        }
        false
    })
}

/// Installs the driver's task pacer and initializes its state on this thread,
/// so that neither allocates during a measurement.
///
/// # Panics
///
/// Panics if another task pacer is installed.
pub fn prepare_pacing() {
    assert!(
        pace_tasks(pace),
        "the driver's pacer is the process's task pacer"
    );
    set_pacing([Held::NOTHING; 2]);
}

/// Holds back what `held` names on each side from now on, and wakes every
/// waiting task it no longer holds back, returning how many it woke.
fn set_pacing(held: [Held; 2]) -> usize {
    let woken: Vec<Waker> = PACING.with_borrow_mut(|pacing| {
        pacing.held = held;
        untraced(|| {
            let mut woken = Vec::new();
            for side in Side::BOTH {
                let held = held[side.index()];
                pacing.waiting[side.index()].retain(|(role, waker)| {
                    let release = !held.holds(*role);
                    if release {
                        woken.push(waker.clone());
                    }
                    !release
                });
            }
            woken
        })
    });
    let count = woken.len();
    for waker in woken {
        waker.wake();
    }
    count
}

/// One direction of the link: bytes one side wrote, split into those still held
/// and those released to the reader.
#[derive(Debug, Default)]
struct Direction {
    /// Written and not yet released.
    held: Vec<u8>,
    /// Released and not yet read, from `read_at` on.
    released: Vec<u8>,
    /// How much of `released` the reader has consumed.
    read_at: usize,
    /// Whether the writer shut its half down.
    closed: bool,
    /// The reader's waker, while it waits for bytes.
    reader: Option<Waker>,
}

/// Both directions, indexed by the writing side's [`Side::index`].
type Shared = Arc<Mutex<[Direction; 2]>>;

/// Locks the link's state.
fn lock(shared: &Shared) -> MutexGuard<'_, [Direction; 2]> {
    shared
        .lock()
        .expect("no link operation panics while holding the lock")
}

/// The link between two peers, from which each side's halves are taken.
#[derive(Debug, Clone)]
pub struct Link {
    /// Both directions' state.
    shared: Shared,
}

impl Link {
    /// Creates an empty link.
    ///
    /// The lock is taken once here, so that any allocation the platform makes
    /// on a mutex's first use happens before a measurement begins.
    pub fn new() -> Self {
        let shared: Shared = Arc::default();
        drop(lock(&shared));
        Self { shared }
    }

    /// Returns `side`'s read half: the bytes released toward it.
    pub fn reader(&self, side: Side) -> Reader {
        Reader {
            shared: self.shared.clone(),
            from: side.peer().index(),
        }
    }

    /// Returns `side`'s write half.
    pub fn writer(&self, side: Side) -> Writer {
        Writer {
            shared: self.shared.clone(),
            into: side.index(),
        }
    }

    /// Returns the bytes each side has written that are not yet released.
    fn held(&self) -> [usize; 2] {
        let directions = lock(&self.shared);
        [directions[0].held.len(), directions[1].held.len()]
    }

    /// Returns the bytes released toward each side that it has not yet read.
    fn unread(&self) -> [usize; 2] {
        let directions = lock(&self.shared);
        Side::BOTH.map(|side| {
            let direction = &directions[side.peer().index()];
            direction.released.len() - direction.read_at
        })
    }

    /// Releases the first `bytes` held bytes that `writer` wrote, and wakes the
    /// reader if it waits.
    fn release(&self, writer: Side, bytes: usize) {
        let waker = {
            let mut directions = lock(&self.shared);
            let direction = &mut directions[writer.index()];
            untraced(|| {
                direction
                    .released
                    .extend_from_slice(&direction.held[..bytes]);
                direction.held.drain(..bytes);
            });
            direction.reader.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// A side's read half.
#[derive(Debug)]
pub struct Reader {
    /// The link's state.
    shared: Shared,
    /// The direction this half reads.
    from: usize,
}

/// Reads released bytes, waiting for a release when none remain.
impl AsyncRead for Reader {
    /// Copies released bytes into `buf`, or reports end of stream once the
    /// writer has shut down and every byte it wrote has been read.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut directions = lock(&self.shared);
        let direction = &mut directions[self.from];
        let unread = &direction.released[direction.read_at..];
        if !unread.is_empty() {
            let count = unread.len().min(buf.remaining());
            buf.put_slice(&unread[..count]);
            direction.read_at += count;
            if direction.read_at == direction.released.len() {
                direction.released.clear();
                direction.read_at = 0;
            }
            return Poll::Ready(Ok(()));
        }
        if direction.closed && direction.held.is_empty() {
            return Poll::Ready(Ok(()));
        }
        direction.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// A side's write half.
#[derive(Debug)]
pub struct Writer {
    /// The link's state.
    shared: Shared,
    /// The direction this half writes.
    into: usize,
}

/// Accepts every write at once, holding the bytes until a release.
impl AsyncWrite for Writer {
    /// Holds all of `buf`; the link never makes a writer wait.
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut directions = lock(&self.shared);
        untraced(|| directions[self.into].held.extend_from_slice(buf));
        Poll::Ready(Ok(buf.len()))
    }

    /// Completes at once: held bytes wait for a release, not a flush.
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    /// Marks the direction closed, so the reader sees end of stream after the
    /// last held byte.
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        lock(&self.shared)[self.into].closed = true;
        Poll::Ready(Ok(()))
    }
}

/// How much of one direction's held bytes a stall releases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Portion {
    /// Release nothing.
    Hold,
    /// Release a prefix of `sixteenths / 16` of the held bytes, at least one
    /// byte.
    ///
    /// Values from 1 through 15 are meaningful. A coarse fraction keeps every
    /// release a sizable share of what is held, so a schedule cannot stretch a
    /// session into millions of one-byte rounds.
    Prefix(u8),
    /// Release every held byte.
    All,
}

impl Portion {
    /// The bytes this portion releases from `held` held bytes.
    fn of(self, held: usize) -> usize {
        match self {
            Self::Hold => 0,
            Self::Prefix(sixteenths) if held > 0 => {
                (held / 16 * usize::from(sixteenths)).clamp(1, held)
            }
            Self::Prefix(_) => 0,
            Self::All => held,
        }
    }
}

/// What one stall releases: a portion of each side's held writes, and which of
/// each side's task kinds stay held back until the next stall, both in
/// [`Side::BOTH`] order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Release {
    /// The portion of each side's held writes delivered to its peer.
    pub bytes: [Portion; 2],
    /// The task kinds each side holds back until the next stall.
    pub held: [Held; 2],
}

impl Release {
    /// Releases the given portions and holds back no task.
    pub const fn bytes(bytes: [Portion; 2]) -> Self {
        Self {
            bytes,
            held: [Held::NOTHING; 2],
        }
    }
}

/// What the driver did over one session.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Drive {
    /// Times both peers stalled and the driver released held bytes.
    pub releases: usize,
    /// The most bytes one release delivered in one direction.
    pub largest_delivery: usize,
    /// The polls the task pacer refused to each side's tasks, in [`Side::BOTH`]
    /// order.
    pub refused: [Refused; 2],
}

/// Records whether a task asked to be polled again.
#[derive(Debug, Default)]
struct WakeFlag(AtomicBool);

/// Turns a wake into a flag the driver reads.
impl Wake for WakeFlag {
    /// Records the wake.
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }

    /// Records the wake.
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }
}

/// Polls `a` and `b` over `link` until both complete, releasing held bytes and
/// held-back tasks at each stall as `schedule` directs.
///
/// Each round polls every side that was woken, in [`Side::BOTH`] order, under
/// its label; a side's future is dropped under its label as soon as it
/// completes, so its session state is freed before the other side's next poll.
/// When no side was woken, both have stalled: the driver asks `schedule` for
/// the release, its argument the bytes each side holds unreleased. A release
/// that would move no byte and wake no held-back task releases every held byte
/// and holds back nothing instead, so every stall makes progress.
///
/// Call [`prepare_pacing`] first.
///
/// The driver's own wake flags are harness state, allocated outside any trace,
/// so every block a trace sees inside the drive is the sessions'.
///
/// # Panics
///
/// Panics if both peers stall with no byte held and no task held back: the
/// session deadlocked.
pub fn drive<A: Future, B: Future>(
    link: &Link,
    a: A,
    b: B,
    mut schedule: impl FnMut([usize; 2]) -> Release,
) -> (A::Output, B::Output, Drive) {
    set_pacing([Held::NOTHING; 2]);
    PACING.with_borrow_mut(|pacing| pacing.refused = [Refused::default(); 2]);
    let flags = untraced(|| [Arc::new(WakeFlag::default()), Arc::new(WakeFlag::default())]);
    let wakers = flags.clone().map(Waker::from);
    let mut a = pin!(Some(a));
    let mut b = pin!(Some(b));
    let (mut a_out, mut b_out) = (None, None);
    let mut drive = Drive::default();
    for flag in &flags {
        flag.0.store(true, Ordering::Release);
    }
    while a_out.is_none() || b_out.is_none() {
        let mut polled = false;
        for side in Side::BOTH {
            if !flags[side.index()].0.swap(false, Ordering::AcqRel) {
                continue;
            }
            let mut cx = Context::from_waker(&wakers[side.index()]);
            POLLING.set(Some(side));
            labelled(side.label(), || match side {
                Side::A => poll_slot(a.as_mut(), &mut cx, &mut a_out, &mut polled),
                Side::B => poll_slot(b.as_mut(), &mut cx, &mut b_out, &mut polled),
            });
            POLLING.set(None);
        }
        if polled {
            continue;
        }
        let held = link.held();
        let waiting =
            PACING.with_borrow(|pacing| pacing.waiting.iter().map(Vec::len).sum::<usize>());
        assert!(
            held.iter().any(|&bytes| bytes > 0) || waiting > 0,
            "both peers stalled with nothing held: the session deadlocked \
             ({} unread bytes toward A, {} toward B)",
            link.unread()[0],
            link.unread()[1],
        );
        let release = schedule(held);
        let mut amounts = [0; 2];
        for (index, portion) in release.bytes.into_iter().enumerate() {
            amounts[index] = portion.of(held[index]);
        }
        let woken = set_pacing(release.held);
        if amounts.iter().all(|&bytes| bytes == 0) && woken == 0 {
            amounts = held;
            set_pacing([Held::NOTHING; 2]);
        }
        for side in Side::BOTH {
            let bytes = amounts[side.index()];
            if bytes > 0 {
                link.release(side, bytes);
                drive.largest_delivery = drive.largest_delivery.max(bytes);
            }
        }
        drive.releases += 1;
    }
    set_pacing([Held::NOTHING; 2]);
    drive.refused = PACING.with_borrow(|pacing| pacing.refused);
    (
        a_out.expect("the loop ends once A completes"),
        b_out.expect("the loop ends once B completes"),
        drive,
    )
}

/// Polls the future in `slot`, if it has not completed; on completion, stores
/// its output in `out` and drops the future in place.
fn poll_slot<F: Future>(
    mut slot: Pin<&mut Option<F>>,
    cx: &mut Context<'_>,
    out: &mut Option<F::Output>,
    polled: &mut bool,
) {
    let Some(future) = slot.as_mut().as_pin_mut() else {
        return;
    };
    *polled = true;
    if let Poll::Ready(output) = future.poll(cx) {
        *out = Some(output);
        slot.set(None);
    }
}
