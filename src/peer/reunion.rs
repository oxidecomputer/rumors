//! Track a peer's handles and retire it.

use std::mem;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::io::{AsyncBufRead, AsyncWrite};
use tokio::sync::{OwnedMutexGuard, oneshot, watch};

use super::{Peer, Replica, Retire, connection::Connection};
use crate::bookmark::MaybeBookmark;
use crate::tree::mirror::preamble::Intent;

/// Track a generation's live handles and its retirement.
///
/// Each handle owns one `alive` sender. Closing that channel therefore marks
/// the point when every handle has either been dropped or begun
/// [`retire`](Peer::retire).
///
/// Every handle gives up its view before its `alive` sender: a retiring handle
/// parks its view or drops it, and a dropped handle drops its fields in
/// declaration order. So once the channel closes, the parked view is the
/// generation's only view, and it owns its bookmark lock alone. That is what
/// lets retirement release a lock held by a leaked session future
/// ([`BookmarkHandle::release_abandoned_lock`](crate::bookmark::BookmarkHandle::release_abandoned_lock)).
pub(super) struct Reunion<T: Send + Sync + 'static, B: MaybeBookmark> {
    /// One sender per live handle; channel closure signals quiescence.
    alive: watch::Sender<()>,
    /// The generation's retirement state.
    retirement: Arc<Retirement<T, B>>,
}

/// Count another handle and share the parking place, without requiring `Clone`
/// payloads or storage.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Clone for Reunion<T, B> {
    /// Add a sender for the new handle and share the parking place.
    fn clone(&self) -> Self {
        Self {
            alive: self.alive.clone(),
            retirement: Arc::clone(&self.retirement),
        }
    }
}

/// The parked view and the turns retiring handles take.
struct Retirement<T: Send + Sync + 'static, B: MaybeBookmark> {
    /// Held by the handle whose turn it is.
    turn: Arc<tokio::sync::Mutex<()>>,
    /// The view and the bookkeeping of turns.
    state: Mutex<State<T, B>>,
}

/// The parked view and the retiring handles of one generation.
struct State<T: Send + Sync + 'static, B: MaybeBookmark> {
    /// The parked view.
    replica: Option<Replica<T, B>>,
    /// Whether a retirement session passed the point of no return.
    retired: bool,
    /// Retiring handles that have not finished their turn.
    waiting: usize,
    /// Senders for the verdicts of handles whose session failed.
    failed: Vec<oneshot::Sender<Verdict<T, B>>>,
}

/// How retirement ended for a handle whose session failed.
enum Verdict<T: Send + Sync + 'static, B: MaybeBookmark> {
    /// Another handle's session passed the point of no return.
    Retired,
    /// Every session failed; a handle of a new generation.
    Remain(Peer<T, B>),
}

/// Verdicts to send once the state's lock is released.
#[must_use = "the verdicts must be sent"]
#[allow(clippy::type_complexity)]
struct Verdicts<T: Send + Sync + 'static, B: MaybeBookmark>(
    Vec<(oneshot::Sender<Verdict<T, B>>, Verdict<T, B>)>,
);

/// Deliver verdicts.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Verdicts<T, B> {
    /// Send each verdict.
    fn send(self) {
        for (sender, verdict) in self.0 {
            drop(sender.send(verdict));
        }
    }
}

/// Settle the generation.
impl<T: Send + Sync + 'static, B: MaybeBookmark> State<T, B> {
    /// Collect the verdicts the state now decides.
    fn settle(&mut self) -> Verdicts<T, B> {
        if self.retired {
            let failed = mem::take(&mut self.failed);
            return Verdicts(failed.into_iter().map(|f| (f, Verdict::Retired)).collect());
        }
        if self.waiting > 0 || self.failed.is_empty() {
            return Verdicts(Vec::new());
        }
        let replica = self.replica.take().expect(
            "every retiring handle parks a view before it waits, a turn returns the \
             view unless it retires, and none is waiting, so the view is parked",
        );
        let mut failed = mem::take(&mut self.failed);
        let last = failed.pop().expect("there is a failed handle");
        let peer = Peer::new(replica);
        let mut verdicts: Vec<_> = failed
            .into_iter()
            .map(|f| (f, Verdict::Remain(peer.clone())))
            .collect();
        verdicts.push((last, Verdict::Remain(peer)));
        Verdicts(verdicts)
    }
}

/// Construct and lock the state.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Retirement<T, B> {
    /// Create the state of a generation with no retiring handle.
    fn new() -> Self {
        Self {
            turn: Arc::new(tokio::sync::Mutex::new(())),
            state: Mutex::new(State {
                replica: None,
                retired: false,
                waiting: 0,
                failed: Vec::new(),
            }),
        }
    }

    /// Lock the state.
    fn lock(&self) -> MutexGuard<'_, State<T, B>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a retiring handle finds when its turn comes.
enum Turn {
    /// The handle holds the view and runs its session.
    Holding,
    /// Another handle's session already retired the generation.
    Retired,
}

/// One retiring handle's place in its generation.
struct Seat<T: Send + Sync + 'static, B: MaybeBookmark> {
    /// The generation.
    retirement: Arc<Retirement<T, B>>,
    /// The view, while this handle holds the turn.
    replica: Option<Replica<T, B>>,
    /// Whether this handle's session passed the point of no return.
    passed_point_of_no_return: Intent,
    /// The sender for this handle's verdict, once its session failed.
    failed: Option<oneshot::Sender<Verdict<T, B>>>,
    /// This handle's turn.
    turn: Option<OwnedMutexGuard<()>>,
}

/// Enroll and take a turn.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Seat<T, B> {
    /// Count a retiring handle and park its view.
    fn enroll(retirement: Arc<Retirement<T, B>>, replica: Replica<T, B>) -> Self {
        let redundant = {
            let mut state = retirement.lock();
            state.waiting += 1;
            match &state.replica {
                Some(_) => Some(replica),
                None => {
                    state.replica = Some(replica);
                    None
                }
            }
        };
        drop(redundant);
        Self {
            retirement,
            replica: None,
            passed_point_of_no_return: Intent::Remain,
            failed: None,
            turn: None,
        }
    }

    /// Wait for this handle's turn and take the view, unless the generation
    /// retired.
    async fn take_turn(&mut self) -> Turn {
        let turn = Arc::clone(&self.retirement.turn).lock_owned().await;
        let mut replica = {
            let mut state = self.retirement.lock();
            if state.retired {
                return Turn::Retired;
            }
            state.replica.take().expect(
                "every retiring handle parks a view before it waits, a turn returns \
                 the view unless it retires, and this handle is waiting, so the view \
                 is parked",
            )
        };
        // No session future of this generation can run or be dropped again
        // now: every future that borrowed a handle has ended, since the
        // handle could not otherwise have been consumed or dropped, and a
        // future owning a handle would have kept the channel open. So a held
        // bookmark lock, or an outstanding fork reservation, belongs to a
        // leaked future that can never run again, and we release both.
        replica.bookmark = replica.bookmark.release_abandoned_lock();
        replica.release_abandoned_reservations();
        self.replica = Some(replica);
        self.turn = Some(turn);
        Turn::Holding
    }
}

/// Release this handle's place.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Drop for Seat<T, B> {
    /// Return the view, record a failure, and settle the generation.
    fn drop(&mut self) {
        let mut discarded = None;
        let verdicts = {
            let mut state = self.retirement.lock();
            state.waiting -= 1;
            if let Some(replica) = self.replica.take() {
                match self.passed_point_of_no_return {
                    Intent::Remain => state.replica = Some(replica),
                    Intent::Retire => {
                        state.retired = true;
                        discarded = Some(replica);
                    }
                }
            }
            if let Some(failed) = self.failed.take() {
                state.failed.push(failed);
            }
            state.settle()
        };
        verdicts.send();
        drop(discarded);
    }
}

/// Construct handles.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Peer<T, B> {
    /// Assemble the first handle of a fresh generation around `replica`. Every
    /// other handle is a [`Clone`] of this one, so the channel's sender count
    /// faithfully counts handles.
    pub(crate) fn new(replica: Replica<T, B>) -> Self {
        Self {
            replica,
            reunion: Reunion {
                alive: watch::Sender::new(()),
                retirement: Arc::new(Retirement::new()),
            },
        }
    }
}

/// Share the replica, configuration, and storage.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Clone for Peer<T, B> {
    /// Create another handle referring to the same [`Peer`].
    fn clone(&self) -> Self {
        Self {
            replica: self.replica.share(),
            reunion: self.reunion.clone(),
        }
    }
}

/// Retire through any handle.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Peer<T, B> {
    /// Leave the gossip network after synchronizing one final time with another
    /// peer.
    ///
    /// While it is possible to simply drop a [`Peer`] with no special ceremony,
    /// explicitly retiring the peer helps keep the representation of
    /// [`Version`](crate::Version)s efficient as peers come and go.
    ///
    /// The outcome of this function, [`Retire`], reports whether this peer
    /// successfully retired from the network, or whether it remains (e.g. due
    /// to a failure which could be retried), and whether the connection can
    /// carry another session.
    pub async fn retire<R, W>(self, mut read: R, mut write: W) -> Retire<T, R, W, B>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let Self {
            replica,
            reunion: Reunion { alive, retirement },
        } = self;
        // Give up this view before this handle's `alive` sender.
        let mut seat = Seat::enroll(retirement, replica);
        let mut closed = alive.subscribe();
        drop(alive);

        // Nothing is sent on this channel. Closure means every handle is gone,
        // and no new sender can be cloned after that point.
        while closed.changed().await.is_ok() {}

        if let Turn::Retired = seat.take_turn().await {
            return Retire::Retired {
                connection: Connection::Reusable { read, write },
            };
        }
        let replica = seat.replica.as_ref().expect("the turn holds the view");
        let result = replica
            .retire_inner(&mut seat.passed_point_of_no_return, &mut read, &mut write)
            .await;
        if seat.passed_point_of_no_return == Intent::Retire {
            return Retire::Retired {
                connection: Connection::from_session(result, read, write),
            };
        }

        let (verdict, settled) = oneshot::channel();
        seat.failed = Some(verdict);
        drop(seat);
        let settled = settled.await.expect(
            "a failed handle's sender is sent the verdict before the state drops it: \
             the generation settles once it retires or no handle waits, and a \
             waiting handle holds the state",
        );
        match (settled, result) {
            (Verdict::Retired, result) => Retire::Retired {
                connection: Connection::from_session(result, read, write),
            },
            (Verdict::Remain(peer), Ok(())) => Retire::Declined { peer, read, write },
            (Verdict::Remain(peer), Err(error)) => Retire::Recovered { peer, error },
        }
    }
}
