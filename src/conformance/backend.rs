//! Conformance checks for a storage backend's session pricing.
//!
//! The sync budget ([`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget))
//! bounds a session's memory by pricing every in-flight node through the
//! backend's own cost function. That function is the one input whose
//! mis-statement breaches *memory* rather than latency, and only the
//! backend's author can know a node value's real resident bytes — so this
//! suite is how a backend implementation proves its account:
//!
//! - **Pointwise**: every node the session assembles is measured (via
//!   [`Measure`]) against the cost function at that node's actual fan and
//!   version bounds. An underpriced node fails the run by name.
//! - **End to end**: identical divergent corpora reconcile once at the
//!   zero-budget floor and once under a stated budget, with every live
//!   node value's measured bytes on a census ledger. The peak difference
//!   — the bytes the *window* itself admitted — must fit the budget.
//!
//! # Accounting premises
//!
//! Leaf values are deliberately outside the account: leaf payloads are
//! priced by [`target_message_size`](crate::Peer::target_message_size),
//! so leaves constructed at the conversion boundary charge nothing here.
//! The ledger is process-global (run one `check` per process, as nextest
//! does), and the differencing baseline absorbs what exists regardless of
//! the window: the resting corpora, the assembly fans' correctness floor,
//! and the commit join's transients.
//!
//! # Visibility
//!
//! The backend boundary is crate-internal today, so this suite runs as
//! this crate's own gate over its backends; the entry point goes public
//! with the [`Backend`] trait when a deployment-implementable storage
//! boundary ships, the way [`conformance::link`](super::link) shipped
//! with the [`Link`](crate::link::Link) boundary.

use std::pin::pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_stream::stream;
use futures::{StreamExt, stream as futures_stream};

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::{
            self, Backend, Leaf, Node, NodeStream, Root, materialized, window::WindowConfig,
        },
        typed::{
            Hash, Path, Prefix,
            height::{self, Height, S, Z},
        },
    },
};

/// The measurement oracle a backend brings to its conformance run.
///
/// `measure` reports the actual bytes one node value keeps resident
/// beyond the replica's shared storage — everything the handle owns or
/// keeps alive per session reference. It is the ground truth the
/// [`node_bytes`](Backend::node_bytes) contract is checked against, so
/// it must not consult the cost function it validates.
pub(crate) trait Measure<T: Send + Sync + 'static>: Backend<T, Node<Z>: Leaf<T>> {
    /// The actual resident bytes of one node value, measured.
    fn measure<H: Height>(node: &Self::Node<H>) -> usize;
}

/// The process-global byte census the charged decorator maintains.
mod ledger {
    use super::{AtomicUsize, Mutex, Ordering};

    /// Measured bytes of node values alive right now.
    static LIVE: AtomicUsize = AtomicUsize::new(0);
    /// The most bytes ever concurrently alive since the last reset.
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    /// Pointwise contract violations, reported at the end of a run.
    static VIOLATIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub(super) fn charge(bytes: usize) {
        let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
        PEAK.fetch_max(live, Ordering::Relaxed);
    }

    pub(super) fn discharge(bytes: usize) {
        LIVE.fetch_sub(bytes, Ordering::Relaxed);
    }

    pub(super) fn violation(report: String) {
        VIOLATIONS
            .lock()
            .expect("the violation ledger mutex is not poisoned")
            .push(report);
    }

    /// Restart the high-water mark from the current live bytes.
    pub(super) fn reset_peak() {
        PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
    }

    pub(super) fn peak() -> usize {
        PEAK.load(Ordering::Relaxed)
    }

    pub(super) fn take_violations() -> Vec<String> {
        std::mem::take(
            &mut VIOLATIONS
                .lock()
                .expect("the violation ledger mutex is not poisoned"),
        )
    }
}

/// The byte-charging census decorator.
///
/// Delegates every operation to the wrapped backend, keeping each node
/// value's measured bytes on the [`ledger`] for as long as any handle to
/// it lives, and checking each assembled parent against the cost
/// function and the aggregate recurrences.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Charged<B> {
    inner: B,
}

impl<B> Charged<B> {
    pub(crate) fn new(inner: B) -> Self {
        Self { inner }
    }
}

/// A node handle whose measured bytes ride the census ledger.
///
/// The inner slot is an `Option` only so consuming the wrapper can settle
/// the ledger exactly once: `into_inner` discharges and empties it, and
/// `Drop` discharges only when it is still full.
#[derive(Debug)]
pub(crate) struct ChargedNode<N> {
    inner: Option<N>,
    bytes: usize,
}

impl<N> ChargedNode<N> {
    fn wrap(inner: N, bytes: usize) -> Self {
        ledger::charge(bytes);
        Self {
            inner: Some(inner),
            bytes,
        }
    }

    fn inner(&self) -> &N {
        self.inner
            .as_ref()
            .expect("a live charged node always holds its inner handle")
    }

    fn into_inner(mut self) -> N {
        ledger::discharge(self.bytes);
        self.inner
            .take()
            .expect("a live charged node always holds its inner handle")
    }
}

impl<N: Clone> Clone for ChargedNode<N> {
    fn clone(&self) -> Self {
        Self::wrap(self.inner().clone(), self.bytes)
    }
}

impl<N> Drop for ChargedNode<N> {
    fn drop(&mut self) {
        if self.inner.is_some() {
            ledger::discharge(self.bytes);
        }
    }
}

impl<T, N> Node<T> for ChargedNode<N>
where
    T: Send + Sync + 'static,
    N: Node<T> + Clone + Send + 'static,
    N::Backend: Measure<T>,
{
    type Backend = Charged<N::Backend>;
    type Height = N::Height;

    fn ceiling(&self) -> &Version {
        self.inner().ceiling()
    }

    fn floor(&self) -> &Version {
        self.inner().floor()
    }

    fn hash(&self) -> Hash {
        self.inner().hash()
    }

    fn len(&self) -> usize {
        self.inner().len()
    }

    fn version_bytes(&self) -> usize {
        self.inner().version_bytes()
    }
}

impl<T, N> Leaf<T> for ChargedNode<N>
where
    T: Send + Sync + 'static,
    N: Leaf<T> + Clone + Send + 'static,
    N::Backend: Measure<T>,
{
    fn message(&self) -> &Message<T> {
        self.inner().message()
    }

    fn leaf(version: Version, message: Message<T>) -> Self {
        // Leaf values are outside the account by the module's premise:
        // their payloads are priced by the wire's message-size target.
        Self::wrap(N::leaf(version, message), 0)
    }
}

/// The two encoded version bounds a node keeps resident, in bytes.
fn bound_bytes<T, N>(node: &N) -> usize
where
    T: Send + Sync + 'static,
    N: Node<T>,
{
    node.ceiling().as_bytes().len() + node.floor().as_bytes().len()
}

impl<B, T> Backend<T> for Charged<B>
where
    B: Measure<T> + Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Node<H: Height> = ChargedNode<B::Node<H>>;
    type Error = B::Error;

    fn node_bytes(children: usize, version_bound: usize) -> usize {
        B::node_bytes(children, version_bound)
    }

    async fn parent<H>(
        self,
        prefix: Prefix<S<H>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> Result<Option<Self::Node<S<H>>>, Self::Error>
    where
        H: Height,
        S<H>: Height,
    {
        let fan = children.iter().filter(|(_, child)| child.is_some()).count();
        // The aggregate recurrences the children in hand determine: what
        // the assembled parent must answer.
        let (leaves, version_bytes) = children
            .iter()
            .filter_map(|(_, child)| child.as_ref())
            .fold((0usize, 0usize), |(leaves, bytes), child| {
                (leaves + child.len(), bytes.max(child.version_bytes()))
            });
        let children = children
            .into_iter()
            .map(|(radix, child)| (radix, child.map(ChargedNode::into_inner)))
            .collect();
        let parent = self.inner.parent(prefix, children).await?;
        Ok(parent.map(|node| {
            let measured = B::measure(&node);
            // The pointwise contract: the cost function evaluated at this
            // node's own fan and bounds must cover its measured bytes.
            let priced = B::node_bytes(fan, bound_bytes::<T, _>(&node));
            if measured > priced {
                ledger::violation(format!(
                    "underpriced node: fan {fan} measured {measured} B, \
                     node_bytes priced {priced} B",
                ));
            }
            // The aggregate contract: a parent answers the sum of its
            // children's leaves, and the max of their version bytes and
            // its own two bounds' encodings — interior ceilings and
            // floors join many leaves and can outgrow every one of them,
            // so the aggregate must cover the bounds it assembles.
            if fan > 0 && node.len() != leaves {
                ledger::violation(format!(
                    "mis-propagated len: parent answers {}, children sum to {leaves}",
                    node.len(),
                ));
            }
            let version_bytes = version_bytes
                .max(node.ceiling().as_bytes().len())
                .max(node.floor().as_bytes().len());
            if fan > 0 && node.version_bytes() != version_bytes {
                ledger::violation(format!(
                    "mis-propagated version_bytes: parent answers {}, \
                     children and own bounds max to {version_bytes}",
                    node.version_bytes(),
                ));
            }
            ChargedNode::wrap(node, measured)
        }))
    }

    fn children<H>(
        self,
        prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
    ) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height,
    {
        stream! {
            let mut children = pin!(self.inner.children(prefix, parent.into_inner()));
            while let Some(child) = children.next().await {
                yield child.map(|(prefix, node)| {
                    let measured = B::measure(&node);
                    (prefix, ChargedNode::wrap(node, measured))
                });
            }
        }
    }
}

/// Messages each side holds before the fork: the shared corpus.
const COMMON: usize = 512;

/// Messages each side originates alone: the mutual divergence.
const DIVERGENT: usize = 1_024;

/// Run one backend through the full conformance check.
///
/// Builds two corpora sharing [`COMMON`] messages and diverging by
/// [`DIVERGENT`] more on each side, reconciles them twice — once at the
/// zero-budget floor, once under `budget_bytes` — and panics with the
/// violated clause if any assembled node was underpriced, if the window's
/// measured byte admittance exceeded the budget, or if the session failed
/// to converge the corpora. Run one check per process: the census ledger
/// is process-global.
pub(crate) async fn check<B>(backend: B, budget_bytes: usize)
where
    B: Measure<u64> + Clone,
    B::Error: std::fmt::Debug,
{
    let floor_peak = run(backend.clone(), WindowConfig::Budget(0)).await;
    let budget_peak = run(backend, WindowConfig::Budget(budget_bytes)).await;

    let violations = ledger::take_violations();
    assert!(
        violations.is_empty(),
        "the backend's node_bytes contract failed pointwise:\n{}",
        violations.join("\n"),
    );

    let admitted = budget_peak.saturating_sub(floor_peak);
    assert!(
        admitted <= budget_bytes,
        "widening the window from the floor admitted {admitted} measured \
         bytes at peak; the stated budget is {budget_bytes}",
    );
}

/// One controlled-divergence reconciliation; returns the ledger's peak
/// measured bytes above the resting corpora.
async fn run<B>(backend: B, window: WindowConfig) -> usize
where
    B: Measure<u64> + Clone,
    B::Error: std::fmt::Debug,
{
    let charged = Charged::new(backend);

    // Two concurrent histories from one universe: the left clock mints the
    // shared corpus and its own tail; the right fork mints the other tail.
    let mut left_clock = before::Clock::seed();
    let mut right_clock = left_clock.fork();

    let common: Vec<(Version, u64)> = (0..COMMON as u64)
        .map(|payload| (left_clock.tick().clone(), payload))
        .collect();
    let left_tail: Vec<(Version, u64)> = (0..DIVERGENT as u64)
        .map(|payload| (left_clock.tick().clone(), 1 << 32 | payload))
        .collect();
    let right_tail: Vec<(Version, u64)> = (0..DIVERGENT as u64)
        .map(|payload| (right_clock.tick().clone(), 2 << 32 | payload))
        .collect();

    let left = corpus(&charged, common.iter().chain(&left_tail)).await;
    let right = corpus(&charged, common.iter().chain(&right_tail)).await;

    // The corpora are what exists regardless of the window; measure the
    // session's own admittance above them. The greeting's sizes need no
    // stating: they are the roots' own aggregates.
    ledger::reset_peak();

    let client = materialized::Handshaking::start(charged.clone(), left).window(window);
    let server = materialized::Handshaking::start(charged, right).window(window);
    let (ours, theirs) = streaming::mirror(client, server)
        .await
        .expect("the conformance session reconciles");
    let peak = ledger::peak();

    let converged = match (&ours.root, &theirs.root) {
        (Some(left_root), Some(right_root)) => left_root.hash() == right_root.hash(),
        _ => false,
    };
    assert!(
        converged,
        "the conformance session must converge both corpora to one root",
    );
    peak
}

/// Assemble one corpus through the charged backend, leaves in path order.
// The inline pair type is clearer than a name minted only to satisfy the
// lint.
#[allow(clippy::type_complexity)]
async fn corpus<B>(
    charged: &Charged<B>,
    messages: impl Iterator<Item = &(Version, u64)>,
) -> Root<Charged<B>, u64>
where
    B: Measure<u64> + Clone,
    B::Error: std::fmt::Debug,
{
    let mut leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)> = messages
        .map(|(version, payload)| {
            let message = Message::new(*payload);
            let path = Path::for_leaf(version, message.bytes());
            let leaf = <ChargedNode<B::Node<Z>> as Leaf<u64>>::leaf(version.clone(), message);
            (Prefix::from(path), leaf)
        })
        .collect();
    leaves.sort_by_key(|(prefix, _)| *prefix);
    let ceiling = Version::join_all(leaves.iter().map(|(_, leaf)| leaf.ceiling().clone()));

    let mut assembled = pin!(
        charged
            .clone()
            .assemble::<height::Root>(Box::pin(futures_stream::iter(leaves.into_iter().map(Ok))))
    );
    let root = assembled
        .next()
        .await
        .expect("a non-empty corpus assembles a root")
        .expect("corpus assembly is infallible at rest");
    assert!(
        assembled.next().await.is_none(),
        "one sorted run assembles exactly one root",
    );
    Root {
        ceiling,
        root: Some(root.1),
    }
}

#[cfg(test)]
mod tests;
