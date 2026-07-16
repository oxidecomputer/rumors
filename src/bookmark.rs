//! Identity checkpoints that survive an ungraceful restart.
//!
//! A [`Bookmark`] is application-supplied persistent storage for *who* a
//! [`Peer`](crate::Peer) is and how far it has advanced, so a peer that crashed
//! can recover its identity instead of leaking it. The crate drives it through
//! [`Bookmarked`], the in-memory cache that folds the live party into the stored
//! record before each gossip round and slices a donated party back out before it
//! crosses the wire.
//!
//! The default [`NoBookmark`] persists nothing: a peer that never retires simply
//! strands its identity, which costs a few bits of timestamp width but corrupts
//! nothing (see the crate docs on membership as custody).

use std::collections::BTreeMap;
use std::pin::Pin;

use before::{Clock, Party, Version};
use futures_util::FutureExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::Network;

pub(crate) mod format;

pub use format::{BOOKMARK_FORMAT_VERSION, BOOKMARK_MAGIC, FormatError};

/// The error a [`Bookmark`] reports when persistence fails.
pub trait BookmarkError {
    /// What a [`load`](Bookmark::load) or [`store`](Bookmark::store)
    /// reports when it fails.
    type Error: std::error::Error + Send + Sync + 'static;
}

/// The crate's serialize step, lent to [`Bookmark::store`] as a boxed future.
///
/// `store` hands this closure a writer; the closure writes the framed record
/// into it. It is a *boxed* future because it both borrows the writer across an
/// `.await` and must be [`Send`] (the engine's futures are `Send`); naming the
/// future as a trait object is the stable way to carry both bounds at once.
pub type Serialized<'a> = Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + 'a>>;

/// Application-supplied byte storage for a [`Peer`](crate::Peer)'s identity.
///
/// A bookmark records *who* a peer is and how far it has causally advanced, but
/// none of *what* it knows: content is recovered the same way any peer gets it,
/// by [`gossip`](crate::Rumors::gossip)ing. The crate owns the on-disk *format*
/// and asks the implementor only for raw byte storage: a reader to
/// [`load`](Bookmark::load) the stored bytes and a writer to atomically
/// [`store`](Bookmark::store) them. The implementor also supplies the
/// [`Error`](BookmarkError::Error) type, on the [`BookmarkError`] supertrait.
///
/// # One bookmark per peer, handled linearly
///
/// A bookmark is the durable identity of a *single* peer across *its own*
/// restarts. Sharing one between distinct, concurrently-live peers is the one
/// misuse that turns this tool against itself: reclamation folds back every
/// stored identity the live party has caught up to, so a shared bookmark can
/// hand the same identity to two live parties at once. A bookmark, like the
/// identity it records, **must be persisted atomically and never duplicated**.
pub trait Bookmark: BookmarkError {
    /// The byte source [`load`](Self::load) hands back.
    type Reader: AsyncRead + Unpin + Send;

    /// Open the stored record for reading, or `Ok(None)` if nothing is stored.
    ///
    /// Called once per [`Peer`](crate::Peer), lazily, before the first write.
    /// `Ok(None)` means *nothing has ever been written*. A present-but-short or
    /// unreadable bookmark is **not** `None`: it surfaces as a corruption error
    /// once the crate validates the frame.
    fn load(&self) -> impl Future<Output = Result<Option<Self::Reader>, Self::Error>> + Send;

    /// Atomically replace the stored record.
    ///
    /// The crate serializes the framed record by calling `write` with a lent
    /// writer. The implementor **must commit the written bytes atomically iff
    /// `write` returns `Ok`** and must report an error rather than leave a
    /// partial frame where the next [`load`](Self::load) could read it.
    fn store<F>(&self, write: F) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        F: for<'a> FnOnce(&'a mut (dyn AsyncWrite + Unpin + Send)) -> Serialized<'a> + Send;
}

/// What a bookmark round trip failed at: I/O, or stored format.
///
/// [`Io`](Self::Io) is the implementor's [`Error`](BookmarkError::Error),
/// recording failures opening, staging, or committing the storage.
/// [`Format`](Self::Format) is this crate's: a stream fault while reading the
/// lent bytes, or a format that is foreign or corrupt (see [`FormatError`]).
#[derive(Debug, thiserror::Error)]
pub enum BookmarkIo<E> {
    /// The backend could not open, stage, or commit its storage.
    #[error(transparent)]
    Io(E),

    /// The crate could not read or validate the stored frame.
    #[error(transparent)]
    Format(#[from] FormatError),
}

/// The crate-internal decoded persistence driver used by [`Bookmarked`].
///
/// [`Bookmark`] deliberately exposes raw storage while this layer owns the
/// durable format and its validation.
pub(crate) trait Persist: BookmarkError {
    /// Read and decode the persisted record, or an empty map if nothing is
    /// stored.
    fn read(
        &self,
    ) -> impl Future<Output = Result<BTreeMap<Network, Vec<Clock>>, BookmarkIo<Self::Error>>> + Send;

    /// Encode and durably replace the persisted record with `bookmarks`.
    fn write(
        &self,
        bookmarks: &BTreeMap<Network, Vec<Clock>>,
    ) -> impl Future<Output = Result<(), BookmarkIo<Self::Error>>> + Send;
}

// The trait methods are called eagerly and their already-`Send` futures are
// adapted with combinators. Re-wrapping them in an `async` body would capture
// `&self` across an await and unnecessarily require `B: Sync`.
impl<B: Bookmark> Persist for B {
    fn read(
        &self,
    ) -> impl Future<Output = Result<BTreeMap<Network, Vec<Clock>>, BookmarkIo<Self::Error>>> + Send
    {
        Bookmark::load(self).then(|loaded| async move {
            let mut reader = match loaded.map_err(BookmarkIo::Io)? {
                None => return Ok(BTreeMap::new()),
                Some(reader) => reader,
            };
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .await
                .map_err(|e| BookmarkIo::Format(FormatError::Read(e)))?;
            format::decode(&bytes).map_err(BookmarkIo::Format)
        })
    }

    fn write(
        &self,
        bookmarks: &BTreeMap<Network, Vec<Clock>>,
    ) -> impl Future<Output = Result<(), BookmarkIo<Self::Error>>> + Send {
        let bytes = format::encode(bookmarks);
        Bookmark::store(self, move |w| {
            Box::pin(async move { w.write_all(&bytes).await })
        })
        .map(|result| result.map_err(BookmarkIo::Io))
    }
}

/// The placeholder [`Bookmark`] that persists nothing.
///
/// The default for every [`Peer`](crate::Peer); a peer using it never recovers
/// a stranded identity, because it never recorded one.
#[derive(Debug)]
pub struct NoBookmark;

impl BookmarkError for NoBookmark {
    type Error = std::convert::Infallible;
}

impl Bookmark for NoBookmark {
    type Reader = tokio::io::Empty;

    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(None)
    }

    async fn store<F>(&self, _write: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnOnce(&'a mut (dyn AsyncWrite + Unpin + Send)) -> Serialized<'a> + Send,
    {
        // Persisting nothing: the serializer is never invoked, so the in-memory
        // record stays exactly as it was — the same no-op as before.
        Ok(())
    }
}

/// The crate-internal pairing of a [`Bookmark`] with its in-memory record, held
/// behind an async [`Mutex`](tokio::sync::Mutex) on the [`Peer`](crate::Peer).
///
/// It does not live in the `watch`-guarded [`Inner`](crate::Inner) because its
/// [`load`](Bookmark::load)/[`store`](Bookmark::store) are `async` and the
/// record's [`Clock`]s are `!Clone` (a clock owns an identity region), so the
/// record can be neither borrowed across an `.await` from under a `watch` guard
/// nor copied out to persist outside one. Instead a session locks this mutex,
/// reflects the live party into the record — [`reclaim`](Self::reclaim)ing
/// before a gossip round or [`slice`](Self::slice)ing before a donation, under
/// a brief `watch` critical section nested inside the mutex so the party and
/// record move together — and [`write`](Self::write)s, all without releasing
/// the lock.
///
/// Loading is *lazy*: the record is born unloaded (`None`) and read from
/// storage by [`ensure_loaded`](Self::ensure_loaded) on first use, the first
/// point a write could otherwise clobber it. The mutex serializes access, so
/// the read is the record's first content rather than a merge.
pub(crate) struct Bookmarked<B> {
    persist: B,
    /// The in-memory record, or `None` until [`load`](Bookmark::load) has run.
    ///
    /// `None` is the unloaded state: a fresh cache is not yet authoritative,
    /// and is distinct from a loaded-but-empty `Some(BTreeMap::new())`. A
    /// failed [`write`](Self::write) resets it to `None`, so the diverged,
    /// unpersisted mutation is discarded and the next use reloads the
    /// authoritative on-disk state.
    inner: Option<BTreeMap<Network, Vec<Clock>>>,
    /// The `(party, version)` last recorded by [`reclaim`](Self::reclaim) and
    /// believed persisted, or `None` when no token is valid.
    ///
    /// No token is valid before the first reclaim, after a
    /// [`slice`](Self::slice) shrinks the identity, or after a failed
    /// [`write`](Self::write). An update whose live identity still matches the
    /// token is a no-op and is suppressed, since it would only re-record an
    /// identical alias.
    last: Option<(Party, Version)>,
}

impl<B> Bookmarked<B> {
    /// Pair `persist` with an unloaded record and no recorded identity.
    pub(crate) fn new(persist: B) -> Self {
        Bookmarked {
            persist,
            inner: None,
            last: None,
        }
    }

    /// Whether `(party, version)` is exactly what the last update persisted, so
    /// re-recording it would be a no-op. The suppression test for
    /// [`update`](crate::Peer::bookmark_update).
    pub(crate) fn is_current(&self, party: &Party, version: &Version) -> bool {
        self.last.as_ref().is_some_and(|(p, v)| {
            // We only need to record the bookmark when our party is the same as
            // the last time we recorded, and the two versions *quotiented by
            // our current party* are not equal, because we're trying to ensure
            // that we persist prior to gossiping any messages which originate
            // from our own party (there's no risk of causal violation in a
            // non-owned identity interval). If the version advances solely in a
            // region that is non-overlapping with our party, then this change
            // is irrelevant for bookmarking: it would be *correct* to persist
            // then, but it is *unnecessary*.
            p == party && v / p == version / p
        })
    }
}

impl<B: Persist> Bookmarked<B> {
    /// Read the stored record on first use, returning it for mutation. A no-op
    /// once loaded; the mutex serializes access, and no mutation precedes a
    /// load, so the read is the record's first content.
    ///
    /// Run under the bookmark mutex, before [`reclaim`](Self::reclaim) or
    /// [`slice`](Self::slice).
    pub(crate) async fn ensure_loaded(&mut self) -> Result<(), BookmarkIo<B::Error>> {
        if self.inner.is_none() {
            self.inner = Some(self.persist.read().await?);
        }
        Ok(())
    }

    /// Persist the current record.
    ///
    /// A no-op while unloaded (nothing has been mutated to persist). Run under
    /// the bookmark mutex, after a [`reclaim`](Self::reclaim) (which has
    /// already staged the suppression token) or a [`slice`](Self::slice).
    ///
    /// On failure both the record and the suppression token are reset to
    /// `None`: the in-memory mutation never reached storage, so it is discarded
    /// (the next [`ensure_loaded`](Self::ensure_loaded) reloads the
    /// authoritative on-disk state — this is what reverts a
    /// [`slice`](Self::slice) whose donation could not be persisted), and
    /// clearing the token forces the next update to re-record rather than
    /// suppress against a `(party, version)` that never reached storage.
    pub(crate) async fn write(&mut self) -> Result<(), BookmarkIo<B::Error>> {
        let result = match &self.inner {
            Some(inner) => self.persist.write(inner).await,
            None => return Ok(()),
        };
        if result.is_err() {
            self.inner = None;
            self.last = None;
        }
        result
    }

    /// Slice the donated `party` out of the record, since it has now left for
    /// the network.
    ///
    /// The synchronous half of donation, run inside the caller's `watch`
    /// critical section (so it moves with the party leaving `Inner`); the
    /// caller [`write`](Self::write)s afterwards. Must run *before* sending the
    /// [`Party`] over the network, and after
    /// [`ensure_loaded`](Self::ensure_loaded).
    pub(crate) fn slice(&mut self, network: Network, party: &Party) {
        let inner = self.inner.as_mut().expect("loaded before mutation");
        if let Some(clocks) = inner.remove(&network) {
            let clocks: Vec<_> = clocks
                .into_iter()
                .filter_map(|clock| {
                    let (mut p, v) = clock.into_parts();
                    p = p.without(party)?;
                    Some(Clock::from_parts(p, v))
                })
                .collect();
            if !clocks.is_empty() {
                inner.insert(network, clocks);
            }
        }

        // Donating shrinks our live identity, so the suppression token is now
        // stale: clear it. Leaving it would let a later update wrongly suppress
        // if the party happened to return to its pre-donation value at the same
        // version (e.g. forking for a bootstrap, then absorbing that peer's
        // retirement) — persisting nothing while the live identity has grown
        // back past what is on disk.
        self.last = None;
    }

    /// Record `party`'s identity at `version` without reclaiming anything:
    /// append our current alias to the network's clocks, leaving every other
    /// stored entry exactly as it lies.
    ///
    /// The attach-time persist behind
    /// [`Peer::bookmark`](crate::Peer::bookmark), where the live party **must
    /// not move** — not even transiently — so that a failed
    /// [`write`](Self::write) can hand the peer back untouched. Reclaiming
    /// (which grows the live party) is therefore deferred to the first gossip,
    /// behind that path's persist gate, rather than done here. Run under the
    /// bookmark mutex, after [`ensure_loaded`](Self::ensure_loaded), before
    /// [`write`](Self::write).
    ///
    /// The suppression token is deliberately *not* staged: the next
    /// [`reclaim`](Self::reclaim) must run rather than be suppressed against
    /// this record, so any stranded region this peer already dominates is
    /// folded back in at the first gossip rather than stranded until the next
    /// event.
    pub(crate) fn record(&mut self, network: Network, party: &Party, version: &Version) {
        let inner = self.inner.as_mut().expect("loaded before mutation");
        inner.entry(network).or_default().push(Clock::from_parts(
            party.dangerously_alias(),
            version.clone(),
        ));
    }

    /// Fold the live `party` and `version` into the record, reclaiming every
    /// stored identity that `version` has caught up to and growing `party` in
    /// place by the (disjoint) reclaimed regions.
    ///
    /// The synchronous half of an update, run inside the caller's `watch`
    /// critical section (so the party grows atomically with the record); the
    /// caller [`write`](Self::write)s afterwards. Must run *before* gossiping
    /// over the network (and after [`ensure_loaded`](Self::ensure_loaded)), if
    /// any changes have occurred since the last call.
    pub(crate) fn reclaim(&mut self, network: Network, party: &mut Party, version: &Version) {
        let inner = self.inner.as_mut().expect("loaded before mutation");
        // Get the clocks for this network
        let clocks = inner.entry(network).or_default();

        // Reclaim every dominated region disjoint from our party by joining it
        // back in, setting aside any that overlap.
        let mut overlapping = Vec::new();
        // We use `.own_version()` because we can more-eagerly reclaim a `Party`
        // if only the region *it owns* is causally dominated by the current
        // version: we just need to guarantee that any events we generate using
        // that identity region will be causally future to any previously
        // generated by it, which does not require knowing *everything it knew*;
        // it merely requires knowing *everything it did*.
        for clock in clocks.extract_if(.., |clock| clock.own_version() <= *version) {
            let (p, v) = clock.into_parts();
            if let Err(p) = party.join(p) {
                overlapping.push(Clock::from_parts(p, v));
            }
        }

        // Retain only the overlapping clocks the *fully-grown* party does not
        // already cover: regions still outstanding *above* us (a strict
        // superset of our party), which we must never drop on the floor.
        clocks.extend(
            overlapping
                .into_iter()
                .filter(|clock| !party.covers(clock.party())),
        );

        // Store an alias of our party at its current version.
        clocks.push(Clock::from_parts(
            party.dangerously_alias(),
            version.clone(),
        ));

        // Stage the suppression token: an [`update`] that finds this same
        // `(party, version)` still live will skip, since it would re-record an
        // identical alias. A subsequent failed [`write`] clears it again.
        self.last = Some((party.dangerously_alias(), version.clone()));
    }
}
