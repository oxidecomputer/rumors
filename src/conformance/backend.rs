//! Conformance checks for a storage backend's session pricing.
//!
//! The sync budget ([`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget))
//! bounds a session's memory by pricing every in-flight node through the
//! backend's own cost function. That function is the one input whose
//! mis-statement breaches *memory* rather than latency, and only the
//! backend's author can know a node value's real resident bytes — so this
//! suite is how a backend implementation proves its account:
//!
//! - **Shape**: the cost function is swept for monotonicity in both
//!   arguments over a fan and version-bound grid before any session
//!   runs — the property that keeps the window's quantile evaluation an
//!   upper bound.
//! - **Pointwise**: every node the session assembles is measured (via
//!   [`Measure`]) against the cost function at that node's actual fan and
//!   version bounds. An underpriced node fails the run by name.
//! - **Bulk seams**: the backend's own [`leaves`](Backend::leaves) and
//!   [`assemble`](Backend::assemble) overrides — the paths the wire codec
//!   runs — are delegated to, their yields priced on the same census and
//!   held to the walked or assembled node's aggregates.
//! - **End to end**: identical divergent corpora reconcile once at the
//!   zero-budget floor and once under a stated budget, with every live
//!   node value's measured bytes on a census ledger. The peak difference
//!   — the bytes the *window* itself admitted — must fit the budget.
//!
//! # Accounting premises
//!
//! Leaves are in the account at their post-custody price: construction
//! ([`Leaf::leaf`]) is the backend's chance to persist the payload, so a
//! leaf charges what its handle keeps resident afterward, checked
//! pointwise against `node_bytes(0, bounds)` — the price the session
//! budget charges every decode-fan slot. Payload bytes still crossing
//! inside one wire message are priced by
//! [`target_message_size`](crate::Peer::target_message_size), not here.
//! The ledger is process-global, so checks in one process must not
//! overlap (this module's tests hold one lock across each test body).
//! The differencing baseline absorbs what exists regardless of the
//! window: the resting corpora, the assembly fans' correctness floor,
//! and the commit join's transients.
//!
//! # Visibility
//!
//! The backend boundary is crate-internal, so this suite runs as this
//! crate's own gate over its backends rather than as a public entry
//! point — a suite is caller-visible exactly where its boundary is
//! caller-implementable, as [`conformance::link`](super::link) is for
//! the [`Link`](crate::link::Link) boundary.

use std::collections::BTreeMap;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_stream::stream;
use futures::{StreamExt, stream as futures_stream};

use crate::{
    Version, causally,
    message::Message,
    tree::{
        mirror::streaming::{
            self, Backend, BoxNodeStream, Leaf, Node, NodeStream, Root,
            convert::Convert,
            materialized,
            window::{FAN, WindowConfig},
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
/// Delegates every operation to the wrapped backend — the bulk
/// [`leaves`](Backend::leaves)/[`assemble`](Backend::assemble) overrides
/// included, so the paths production runs are the paths on trial —
/// keeping each node value's measured bytes on the [`ledger`] for as
/// long as any handle to it lives, and checking each assembled parent
/// against the cost function and the aggregate recurrences.
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

    fn dominance_of(&self, known: &Version) -> causally::Dominance {
        self.inner().dominance_of(known)
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

    async fn leaf(
        version: Version,
        message: Message<T>,
    ) -> Result<Self, <N::Backend as Backend<T>>::Error> {
        let node = N::leaf(version, message).await?;
        let measured = <N::Backend as Measure<T>>::measure::<N::Height>(&node);
        // The pointwise contract at the leaf seam: after construction has
        // had its chance to persist the payload, the cost function at
        // `children = 0` and the node's own bounds must cover what the
        // handle keeps resident — this is the price the session budget
        // charges every decode-fan slot.
        let priced = <N::Backend as Backend<T>>::node_bytes(0, bound_bytes::<T, _>(&node));
        if measured > priced {
            ledger::violation(format!(
                "underpriced leaf: measured {measured} B, node_bytes priced {priced} B",
            ));
        }
        Ok(Self::wrap(node, measured))
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

    fn leaves<H: Convert>(
        self,
        prefix: Prefix<H>,
        node: Self::Node<H>,
    ) -> impl NodeStream<Self, T, Z> {
        // Delegate to the wrapped backend's own override: the bulk walk
        // is the path the wire encoder runs, so its yields must land on
        // the census and answer the walked node's aggregates.
        let expected = node.len();
        let aggregate = node.version_bytes();
        stream! {
            let mut walked = 0usize;
            let mut failed = false;
            let mut leaves = pin!(self.inner.leaves(prefix, node.into_inner()));
            while let Some(leaf) = leaves.next().await {
                failed |= leaf.is_err();
                yield leaf.map(|(prefix, leaf)| {
                    walked += 1;
                    let measured = B::measure(&leaf);
                    // The pointwise contract at the walk: a walked leaf
                    // is a fan slot the session budget prices at
                    // `children = 0`.
                    let priced = B::node_bytes(0, bound_bytes::<T, _>(&leaf));
                    if measured > priced {
                        ledger::violation(format!(
                            "underpriced walked leaf: measured {measured} B, \
                             node_bytes priced {priced} B",
                        ));
                    }
                    // Every version bound under the walked node is in its
                    // aggregate, each walked leaf's own included.
                    let encoded = leaf.version_bytes();
                    if encoded > aggregate {
                        ledger::violation(format!(
                            "deflated version_bytes: walked leaf encodes {encoded} B, \
                             the walked node's aggregate answers {aggregate} B",
                        ));
                    }
                    (prefix, ChargedNode::wrap(leaf, measured))
                });
            }
            // The len aggregate is exact, so a completed walk returns it.
            if !failed && walked != expected {
                ledger::violation(format!(
                    "mis-sized leaf walk: yielded {walked} leaves, \
                     the walked node's len answers {expected}",
                ));
            }
        }
    }

    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, T, Z>,
    ) -> impl NodeStream<Self, T, H> + 'a {
        // Delegate to the wrapped backend's own override: bulk assembly
        // is the path the wire decoder runs. The wrapper records each
        // maximal same-prefix run as it feeds the inner stream, then
        // holds every assembled node to account for its run.
        let runs: Arc<Mutex<BTreeMap<Prefix<H>, Run>>> = Arc::default();
        let recorded = Arc::clone(&runs);
        let supplied: BoxNodeStream<'a, B, T, Z> = Box::pin(leaves.map(move |item| {
            item.map(|(prefix, leaf)| {
                let mut runs = recorded
                    .lock()
                    .expect("the run ledger mutex is not poisoned");
                let run = runs
                    .entry(Prefix::<H>::containing(&Path::from(prefix)))
                    .or_default();
                run.leaves += 1;
                run.version_bytes = run.version_bytes.max(leaf.version_bytes());
                (prefix, leaf.into_inner())
            })
        }));
        let assembled = self.inner.assemble::<H>(supplied);
        stream! {
            let mut assembled = pin!(assembled);
            let mut failed = false;
            while let Some(item) = assembled.next().await {
                failed |= item.is_err();
                yield item.map(|(prefix, node)| {
                    let run = runs
                        .lock()
                        .expect("the run ledger mutex is not poisoned")
                        .remove(&prefix);
                    let measured = B::measure(&node);
                    match run {
                        None => ledger::violation(format!(
                            "unsupplied assembly: a node yielded at {prefix:?} \
                             without a leaf run",
                        )),
                        Some(run) => check_assembled::<B, T, H>(&node, measured, &run),
                    }
                    (prefix, ChargedNode::wrap(node, measured))
                });
            }
            // Every recorded run must have assembled a node; leftovers
            // are swallowed leaves. Meaningless after an error: the
            // session is failing for its own reasons.
            if !failed {
                let leftovers = std::mem::take(
                    &mut *runs.lock().expect("the run ledger mutex is not poisoned"),
                );
                for (prefix, run) in leftovers {
                    ledger::violation(format!(
                        "unassembled run: {} leaves at {prefix:?} never yielded a node",
                        run.leaves,
                    ));
                }
            }
        }
    }
}

/// One maximal same-prefix leaf run fed to the assembly seam: what its
/// assembled node must answer.
#[derive(Default)]
struct Run {
    /// Leaves supplied to the run: the assembled node's exact `len`.
    leaves: usize,
    /// The largest supplied version encoding: a floor on the assembled
    /// node's `version_bytes` aggregate.
    version_bytes: usize,
}

/// Hold one bulk-assembled node to its run's account.
fn check_assembled<B, T, H>(node: &B::Node<H>, measured: usize, run: &Run)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    // The len aggregate is exact: a run's node answers its leaf count.
    if node.len() != run.leaves {
        ledger::violation(format!(
            "bulk-assembled len: node answers {}, its run supplied {} leaves",
            node.len(),
            run.leaves,
        ));
    }
    // Interior bounds under the node are invisible at this seam, so the
    // aggregate check is a floor: the run's largest leaf encoding and
    // the node's own two bounds are all in the aggregate. The parent
    // seam keeps the exact recurrence where children are in hand.
    let floor = run
        .version_bytes
        .max(node.ceiling().as_bytes().len())
        .max(node.floor().as_bytes().len());
    if node.version_bytes() < floor {
        ledger::violation(format!(
            "deflated version_bytes: bulk-assembled node answers {} B, \
             its run's leaves and own bounds reach {floor} B",
            node.version_bytes(),
        ));
    }
    // The pointwise contract, evaluated at the widest fan the run
    // admits: the node's own fan is invisible here but never exceeds
    // the radix (`FAN`) or the run's leaf count, and the contract
    // requires `node_bytes` monotone in fan, so a measurement above
    // this price is above the price at the node's own fan too.
    let priced = B::node_bytes(run.leaves.min(FAN), bound_bytes::<T, _>(node));
    if measured > priced {
        ledger::violation(format!(
            "bulk-assembled node over-holds: measured {measured} B, \
             node_bytes at the run's widest fan prices {priced} B",
        ));
    }
}

/// Every version bound up to this many bytes is swept pairwise: the
/// small encodings real sessions exchange, where an off-by-one hides.
const BOUND_DENSE_CEILING: usize = 64;

/// The sweep's largest version bound: 1 MiB, far past any canonical
/// encoding the suite's corpus scale reaches, sampled at powers of two.
const BOUND_SWEEP_CEILING: usize = 1 << 20;

/// The monotonicity sweep's version-bound grid, ascending.
///
/// Dense to [`BOUND_DENSE_CEILING`], then each power of two with both
/// neighbors (the neighbors catch a cost function that special-cases
/// round sizes) up to [`BOUND_SWEEP_CEILING`].
fn sweep_bounds() -> Vec<usize> {
    let mut bounds: Vec<usize> = (0..=BOUND_DENSE_CEILING).collect();
    let mut power = BOUND_DENSE_CEILING << 1;
    while power <= BOUND_SWEEP_CEILING {
        bounds.extend([power - 1, power, power + 1]);
        power <<= 1;
    }
    bounds
}

/// Sweep one cost function for monotonicity in both arguments.
///
/// The [`node_bytes`](Backend::node_bytes) contract requires an upper
/// bound monotone in the child count and in the version bound, because
/// the window derivation evaluates the cost at per-depth quantiles and
/// monotonicity is what keeps a quantile evaluation an upper bound: a
/// pointwise-honest function with a dip between evaluation points
/// under-prices every in-flight reference.
///
/// The derivation's own check is a four-point `debug_assert`, compiled
/// out of release, so the suite sweeps the grid: every adjacent fan pair
/// up to the radix ([`FAN`]), crossed with [`sweep_bounds`]'s version
/// bounds, and every adjacent bound pair at each swept fan.
fn node_bytes_monotone<B>()
where
    B: Measure<u64> + Clone,
    B::Error: std::fmt::Debug,
{
    let bounds = sweep_bounds();
    for &bound in &bounds {
        for fan in 0..FAN {
            let here = B::node_bytes(fan, bound);
            let there = B::node_bytes(fan + 1, bound);
            assert!(
                here <= there,
                "node_bytes must be monotone in children: fan {fan} prices {here} B, \
                 fan {} prices {there} B, at version bound {bound}",
                fan + 1,
            );
        }
    }
    for pair in bounds.windows(2) {
        for fan in 0..=FAN {
            let here = B::node_bytes(fan, pair[0]);
            let there = B::node_bytes(fan, pair[1]);
            assert!(
                here <= there,
                "node_bytes must be monotone in version bound: bound {} prices {here} B, \
                 bound {} prices {there} B, at fan {fan}",
                pair[0],
                pair[1],
            );
        }
    }
}

/// Messages each side holds before the fork: the shared corpus.
const COMMON: usize = 512;

/// Messages each side originates alone: the mutual divergence.
const DIVERGENT: usize = 1_024;

/// Run one backend through the full conformance check.
///
/// Sweeps the cost function for monotonicity, then builds two corpora
/// sharing [`COMMON`] messages and diverging by [`DIVERGENT`] more on
/// each side, walks each corpus through the bulk leaf seam, and
/// reconciles them twice: once at the zero-budget floor, once under
/// `budget_bytes`.
///
/// Checks in one process must not overlap: the census ledger is
/// process-global, so callers serialize (this module's tests hold one
/// static lock per test; nextest's process-per-test isolates them
/// regardless).
///
/// # Panics
///
/// Panics with the violated clause if:
///
/// - the cost function dips anywhere on the swept fan and version-bound
///   grid;
/// - any constructed, walked, or assembled node was underpriced;
/// - a bulk seam mis-answered an aggregate;
/// - the window's measured byte admittance exceeded the budget; or
/// - the session failed to converge the corpora.
pub(crate) async fn check<B>(backend: B, budget_bytes: usize)
where
    B: Measure<u64> + Clone,
    B::Error: std::fmt::Debug,
{
    node_bytes_monotone::<B>();

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

    // The bulk walk seam, exercised the way the wire encoder runs it:
    // every leaf of each corpus once. Before the baseline resets, so the
    // walk's checks land on the ledger while its transient charges stay
    // out of the session's differenced peak.
    walk(&charged, &left).await;
    walk(&charged, &right).await;

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

/// Drain one corpus's bulk leaf walk so the walk seam's checks run.
async fn walk<B>(charged: &Charged<B>, corpus: &Root<Charged<B>, u64>)
where
    B: Measure<u64> + Clone,
    B::Error: std::fmt::Debug,
{
    let Some(root) = corpus.root.clone() else {
        return;
    };
    let mut leaves = pin!(charged.clone().leaves(Prefix::<height::Root>::new(), root));
    while let Some(leaf) = leaves.next().await {
        leaf.expect("corpus leaves walk at rest");
    }
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
    let mut leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)> = Vec::new();
    for (version, payload) in messages {
        let message = Message::new(*payload);
        let path = Path::for_leaf(version, message.bytes());
        let leaf = <ChargedNode<B::Node<Z>> as Leaf<u64>>::leaf(version.clone(), message)
            .await
            .expect("corpus leaves construct at rest");
        leaves.push((Prefix::from(path), leaf));
    }
    leaves.sort_by_key(|(prefix, _)| *prefix);
    let ceiling = Version::join_all(leaves.iter().map(|(_, leaf)| leaf.ceiling()));

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
