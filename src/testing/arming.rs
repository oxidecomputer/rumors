//! Fixtures a test installs on a builder before a peer exists, inert until the
//! test arms them after its setup sessions.
//!
//! A peer's settings are fixed when its builder seeds or joins, so a test that
//! must observe, or impose a deadline on, only the sessions after its setup
//! installs an armable fixture on the builder and arms it when setup ends. An
//! inert deadline never expires, and an inert observer observes nothing, so
//! the setup sessions run exactly as they would with neither installed.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::observe::{Observer, SessionInfo, SessionObserver};

/// A switch shared by a test and the fixtures it installs: the fixtures act
/// only while it is armed.
///
/// A fixture reads the switch when a session starts, so arming or disarming
/// changes only the sessions that start afterwards.
#[derive(Clone, Debug, Default)]
pub struct Arming(Arc<AtomicBool>);

/// Arm, disarm, and read the switch.
impl Arming {
    /// Make the fixtures act on sessions that start from now on.
    pub fn arm(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Make the fixtures inert for sessions that start from now on.
    pub fn disarm(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    /// Whether the fixtures act on a session starting now.
    pub fn is_armed(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A deadline one session runs against.
pub type Deadline = Pin<Box<dyn Future<Output = ()> + Send>>;

/// A gossip timeout factory returning `factory`'s deadline for a session that
/// starts while `arming` is armed, and a deadline that never expires otherwise.
///
/// Install it as the builder's
/// [`gossip_timeout`](crate::PeerBuilder::gossip_timeout).
pub fn armed_timeout<D, F>(
    arming: Arming,
    factory: D,
) -> impl Fn() -> Deadline + Send + Sync + 'static
where
    D: Fn() -> F + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    move || -> Deadline {
        if arming.is_armed() {
            Box::pin(factory())
        } else {
            Box::pin(std::future::pending())
        }
    }
}

/// An observer for [`PeerBuilder::observe`](crate::PeerBuilder::observe) that
/// forwards to `inner` the sessions that start while its switch is armed.
pub struct ArmedObserver {
    /// The switch that decides, as each session starts, whether to observe it.
    arming: Arming,
    /// The observer that sessions started while armed are forwarded to.
    inner: Arc<dyn Observer>,
}

/// Show whether the observer is armed, without requiring a debuggable inner
/// observer.
impl std::fmt::Debug for ArmedObserver {
    /// Format the switch.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArmedObserver")
            .field("arming", &self.arming)
            .finish_non_exhaustive()
    }
}

/// Construct an armable observer.
impl ArmedObserver {
    /// Forward to `inner` the sessions that start while `arming` is armed.
    pub fn new(arming: Arming, inner: Arc<dyn Observer>) -> Arc<Self> {
        Arc::new(Self { arming, inner })
    }
}

/// Observe a session only if it starts while armed.
impl Observer for ArmedObserver {
    /// Forward the session to the inner observer if armed, otherwise skip it.
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        if self.arming.is_armed() {
            self.inner.session(session)
        } else {
            None
        }
    }
}

/// A one-shot deadline a test supplies after its setup sessions: the next
/// session to start takes it, and every other session runs against a deadline
/// that never expires.
///
/// Install [`factory`](Self::factory) as the builder's
/// [`gossip_timeout`](crate::PeerBuilder::gossip_timeout); an empty slot is
/// inert.
#[derive(Clone, Default)]
pub struct DeadlineSlot(Arc<std::sync::Mutex<Option<Deadline>>>);

/// Show whether a deadline is waiting, without requiring a debuggable one.
impl std::fmt::Debug for DeadlineSlot {
    /// Format whether the slot holds a deadline.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let supplied = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some();
        f.debug_struct("DeadlineSlot")
            .field("supplied", &supplied)
            .finish()
    }
}

/// Supply a deadline, and build the factory that hands it out.
impl DeadlineSlot {
    /// Make `deadline` the deadline of the next session to start.
    pub fn supply(&self, deadline: impl Future<Output = ()> + Send + 'static) {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::pin(deadline));
    }

    /// A gossip timeout factory that hands out the supplied deadline once, and
    /// a deadline that never expires otherwise.
    pub fn factory(&self) -> impl Fn() -> Deadline + Send + Sync + 'static {
        let slot = Arc::clone(&self.0);
        move || -> Deadline {
            slot.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
                .unwrap_or_else(|| Box::pin(std::future::pending()))
        }
    }
}
