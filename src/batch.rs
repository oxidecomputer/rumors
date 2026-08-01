use std::sync::Arc;

use before::Clock;
use borsh::BorshSerialize;
use tokio::sync::{Mutex, watch};

use crate::message::Message;
use crate::tree::Action;
use crate::tree::backend::{Local, Store};
use crate::{Inner, Key, StorageError};

/// A batch of insertions and redactions against a [`Rumors`](crate::Rumors),
/// committed atomically by [`commit`](Batch::commit).
///
/// Returned by [`batch`](crate::Rumors::batch) on
/// [`Rumors`](crate::Rumors); the single-action case reads as a plain
/// awaited call (`rumors.send(message).await?`), and chaining accumulates
/// (`rumors.batch().send(a).send(b).redact(key).commit().await?`) into one
/// commit: observers and concurrent gossip sessions see either none of the
/// batch or all of it, never a prefix.
///
/// Building a [`Batch`] holds no lock; committing serializes with the
/// set's other committers. Dropping an uncommitted batch *aborts* it: the
/// staged actions are discarded, no version is minted, and no observer
/// wakes — the batch never happened.
///
/// Commit is the causal moment: a sent message's version dominates
/// everything this replica had observed when the batch committed, not when
/// the batch was built ([`Rumors::send`](crate::Rumors::send) states the
/// contract and its boundary). Because building holds no lock,
/// concurrent synchronization can land between building and committing,
/// and two batches carry no guaranteed causal relationship to one another
/// unless the application synchronizes them itself.
#[must_use = "a batch does nothing until `commit` is awaited; dropping it aborts"]
pub struct Batch<'a, T: Send + Sync + 'static, S: Store<T> = Local> {
    inner: &'a watch::Sender<Inner<T, S>>,
    commit: &'a Arc<Mutex<()>>,
    network: crate::Network,
    actions: Vec<Action<T>>,
}

impl<'a, T: Send + Sync + 'static, S: Store<T>> Batch<'a, T, S> {
    pub(crate) fn new(
        inner: &'a watch::Sender<Inner<T, S>>,
        commit: &'a Arc<Mutex<()>>,
        network: crate::Network,
    ) -> Self {
        Self {
            inner,
            commit,
            network,
            actions: Vec::new(),
        }
    }

    /// Sends a message as part of this batch.
    ///
    /// # Panics
    ///
    /// If `message` fails to serialize. Serialization runs here, not at
    /// commit: the failure surfaces at the offending call.
    pub fn send(mut self, message: T) -> Self
    where
        T: BorshSerialize,
    {
        self.actions.push(Action::Insert(Message::from(message)));
        self
    }

    /// Redacts a [`Key`] as part of this batch.
    ///
    /// Redacting a key not held at commit time is a no-op.
    pub fn redact(mut self, key: Key) -> Self {
        self.actions.push(Action::Forget(key));
        self
    }

    /// Commit every staged action as one atomic unit.
    ///
    /// This is a root-replacing commit, run as the commit protocol's
    /// phases: acquire the set's commit lock, stamp each action against the
    /// commit-time frontier, build the new tree off the `watch` (readers
    /// and observers are never blocked by the build), and publish the
    /// built root in one final critical section, waking observers exactly
    /// once iff the tree changed.
    ///
    /// # Cancellation
    ///
    /// Dropping the returned future before it resolves either commits the
    /// whole batch or aborts it, never a prefix. A caller that never saw
    /// `Ok` must treat the batch as "may or may not have committed": with
    /// the in-memory backend the commit is atomic with the future's final
    /// poll, so cancellation before that poll aborts cleanly, and a
    /// persistent backend may additionally leave a committed-but-
    /// unacknowledged write that the next local commit supersedes.
    pub async fn commit(self) -> Result<(), StorageError<S::Error>> {
        let Self {
            inner,
            commit,
            network,
            actions,
        } = self;
        if actions.is_empty() {
            return Ok(());
        }

        // Phase 1: the commit lock. Serializes this commit against every
        // other root replacement *and* every party shrink (the gossip fork
        // section), so the `(party, frontier)` pair read below stays valid
        // through the publish. Lock order: bookmark → commit → watch; this
        // path never touches the bookmark.
        let _commit = commit.lock().await;

        // Phase 2 (prep): stamp every action against the commit-time
        // frontier. A read-only borrow suffices: minting versions ticks a
        // local working copy, not the party, and the lock holds the pair
        // stable. Party growth (a returned or reclaimed fork) may land
        // between prep and publish, harmlessly: versions minted from the
        // narrower party stay exclusively this replica's. A persisting
        // backend additionally gets an alias of the committing party, to
        // record beside the flipped root (`Store::commit`); recording is
        // the one sanctioned use of an alias.
        let (reactions, base, alias) = {
            let inner = inner.borrow();
            // The party is present on every reachable handle: `retire`
            // consumes the `Peer`, and the `Peer`/`Rumors` XOR keeps a
            // retiring set's handles from coexisting with it.
            let Some(party) = inner.party.as_ref() else {
                debug_assert!(false, "no party to tick in a `Batch` commit");
                return Ok(());
            };
            let alias = S::PERSISTS.then(|| party.dangerously_alias());
            (inner.tree.assign(party, actions), inner.tree.clone(), alias)
        };

        // Phase 3 (build): apply the stamped reactions to a clone of the
        // published root, off the `watch`. Copy-on-write shares everything
        // untouched; only the rebuilt spines are fresh. Change detection is
        // `react`'s own flag: no root hash is read on this path
        // (`Tree::act` states the flag's contract).
        let mut built = base.clone();
        let changed = built.react(reactions).await.map_err(StorageError)?;

        // Persist before publishing: the backend's root-flip transaction
        // records the built root and the identity clock (the committing
        // party at the built frontier) atomically, so the store can never
        // hold a tree whose minted coordinates its recorded identity does
        // not dominate. The in-memory backend's `commit` is a no-op.
        built
            .persist(
                alias.map(|alias| Clock::from_parts(alias, built.latest().clone())),
                network,
            )
            .await
            .map_err(StorageError)?;

        // Phase 4 (publish): swap the built root in. This must follow the
        // *persist* with no intervening await, in the same poll: a future
        // is only dropped between polls, so "persisted → published" is
        // indivisible under cancellation. (A drop parked inside the persist
        // itself leaves the store ahead of the published tree — benign: a
        // store-ahead root never reaches the wire, since sessions snapshot
        // the watch, and the next commit's flip supersedes it.) The lock
        // guarantees the published root is still the one the build started
        // from, making the swap a plain replacement — never a merge, so no
        // walk on this path ever compares a tree against its own
        // clone-derived build.
        inner.send_if_modified(move |inner| {
            debug_assert!(
                inner.tree == base,
                "the commit lock held the published root stable through the build"
            );
            inner.tree = built;
            changed
        });
        Ok(())
    }
}
