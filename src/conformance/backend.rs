//! Conformance checks for a storage backend's session pricing.
//!
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) sizes queues
//! using statistical tree-shape estimates and the backend's node prices. This
//! suite checks those prices against measured nodes and exercises the resulting
//! windows on concrete fixtures. It does not prove a hard bound on session
//! memory:
//!
//! - **Shape**: the cost function is swept for monotonicity in both
//!   arguments over a fan and version-bound grid before any session
//!   runs. This property keeps the window's quantile evaluation an
//!   upper bound.
//! - **Pointwise**: every node the session constructs, assembles, walks,
//!   or explodes is measured (via [`Measure`]) against the cost function
//!   at that node's actual fan and version bounds, or, where the fan
//!   is invisible, at the widest fan the node can have, which
//!   monotonicity makes an upper bound on the price at its own. The
//!   measurement carries the slot padding the window's own constants
//!   leave to the backend (a node aligned wider than a pointer pads the
//!   decode-fan and reference slots it sits in), and a node re-tagged
//!   across heights is re-measured. An underpriced node fails the run
//!   by name.
//! - **Bulk seams**: the backend's own [`leaves`](Backend::leaves) and
//!   [`assemble`](Backend::assemble) overrides — the paths the wire codec
//!   runs — are delegated to, their yields priced on the same census,
//!   held to the walked or assembled node's aggregates, and held to the
//!   clauses the trait states for them: a walk stays inside the walked
//!   prefix and ascends, an assembly yields one node per run in run
//!   order, and [`parent`](Backend::parent) answers a real child with a
//!   parent and an empty group with none.
//! - **End to end**: identical divergent corpora reconcile once at the
//!   zero-budget floor and once under a stated budget, with every live
//!   node value's measured bytes on a census ledger. The measured peak
//!   increase must fit the budget, and the result must contain the union.
//!   This is a fixture-specific comparison: changing a window also changes
//!   the schedule, so wider queues need not increase the measured peak.
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
use std::mem::size_of;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_stream::stream;
use before::Span;
use futures::{StreamExt, stream as futures_stream};

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::{
            self, Backend, BoxNodeStream, ErasedNode, Leaf, Local, Node, NodeStream, Root,
            convert::Convert,
            materialized::{self, Resolve},
            stats::{Recorder, SessionStats},
            window::{FAN, FAN_SLOT_BYTES, REFERENCE_SLOT_BYTES, WindowConfig},
        },
        typed::{
            self, Hash, Path, Prefix,
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
pub(crate) trait Measure: Backend<Node<Z>: Leaf> {
    /// The actual resident bytes of one node value, measured.
    fn measure<H: Height>(node: &Self::Node<H>) -> usize;

    /// The actual resident bytes of one height-erased node value,
    /// measured: the oracle for the re-tag clause in the
    /// [`erase`](Backend::erase) direction.
    fn measure_erased(node: &Self::Erased) -> usize;
}

/// Bytes one decode-fan slot pads around a leaf handle of `B` beyond
/// what the window charges for the in-memory handle's slot.
///
/// The window prices a fan slot at `node_bytes(0, bound)` plus
/// [`FAN_SLOT_BYTES`], the pair's padding under the pointer-class
/// handle, and leaves a wider-aligned handle's extra padding to the
/// backend's own price: this is that extra, the obligation the leaf
/// checks fold into their comparison. Never negative: a handle narrower
/// than a pointer is overcharged, which is the safe direction.
const fn fan_slot_excess<B: Backend<Node<Z>: Leaf>>() -> usize {
    (size_of::<(Prefix<Z>, B::Node<Z>)>() - size_of::<B::Node<Z>>()).saturating_sub(FAN_SLOT_BYTES)
}

/// Bytes the per-level reference slots pad around a height-`H` handle
/// of `B` beyond what the window charges for the in-memory handle's.
///
/// A held reference sits in a query slot `(u8, node)` and a resolution
/// slot `(u8, Resolve<erased>)`; the window prices both at
/// [`REFERENCE_SLOT_BYTES`], derived for the pointer-class handle, and
/// leaves a wider layout's extra padding to the backend's price. The
/// listing slot carries a hash, not a handle, so it pads the same for
/// every backend.
const fn reference_slot_excess<B: Backend<Node<Z>: Leaf>, H: Height>() -> usize {
    let real = (size_of::<(u8, B::Node<H>)>() - size_of::<B::Node<H>>())
        + (size_of::<(u8, Resolve<B::Erased>)>() - size_of::<B::Erased>());
    real.saturating_sub(LOCAL_REFERENCE_PADDING)
}

/// The padding the window's reference-slot constant carries for the
/// in-memory handle: the query and resolution slots less the handle in
/// each.
const LOCAL_REFERENCE_PADDING: usize = (size_of::<(u8, typed::Node<Z>)>()
    - size_of::<typed::Node<Z>>())
    + (size_of::<(u8, Resolve<<Local as Backend>::Erased>)>()
        - size_of::<<Local as Backend>::Erased>());

// The decomposition above is the window constant's own: the two
// handle-bearing slots' padding, the two handles, and the listing slot.
const _: () = assert!(
    LOCAL_REFERENCE_PADDING
        + size_of::<typed::Node<Z>>()
        + size_of::<<Local as Backend>::Erased>()
        + size_of::<(u8, Hash)>()
        == REFERENCE_SLOT_BYTES,
    "the reference-slot padding decomposes the window's constant",
);

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

/// Fail the run for `reason`, with every contract violation still
/// pending on the ledger appended.
///
/// A by-name report is never masked by the failure that followed it: a
/// backend fault first recorded on the ledger and then felt as a lost
/// root or a short session names itself in the panic.
fn fail(reason: &str) -> ! {
    let pending = ledger::take_violations();
    if pending.is_empty() {
        panic!("{reason}");
    }
    panic!(
        "{reason}; contract violations pending:\n{}",
        pending.join("\n"),
    );
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

/// Preserve typed-node observations while tracking handle residency.
impl<N> Node for ChargedNode<N>
where
    N: Node + Clone + Send + 'static,
    N::Backend: Measure,
{
    type Backend = Charged<N::Backend>;
    type Height = N::Height;

    /// Borrow the wrapped node's version bounds.
    fn span(&self) -> Span<'_> {
        self.inner().span()
    }

    /// Read the wrapped node's leaf count.
    fn len(&self) -> usize {
        self.inner().len()
    }

    /// Read the wrapped node's largest encoded version bound.
    fn version_bytes(&self) -> usize {
        self.inner().version_bytes()
    }
}

impl<E: ErasedNode> ErasedNode for ChargedNode<E> {
    fn span(&self) -> Span<'_> {
        self.inner().span()
    }

    fn hash(&self) -> Hash {
        self.inner().hash()
    }

    fn len(&self) -> usize {
        self.inner().len()
    }
}

impl<N> Leaf for ChargedNode<N>
where
    N: Leaf + Clone + Send + 'static,
    N::Backend: Measure,
{
    fn message(&self) -> &Message {
        self.inner().message()
    }

    async fn leaf(
        version: Version,
        message: Message,
    ) -> Result<Self, <N::Backend as Backend>::Error> {
        let node = N::leaf(version, message).await?;
        let measured = <N::Backend as Measure>::measure::<N::Height>(&node);
        // The pointwise contract at the leaf seam: after construction has
        // had its chance to persist the payload, the cost function at
        // `children = 0` and the node's own bounds must cover what the
        // handle keeps resident, its decode-fan slot's padding included
        // — this is the price the session budget charges every
        // decode-fan slot.
        let padding = fan_slot_excess::<N::Backend>();
        let priced = <N::Backend as Backend>::node_bytes(0, bound_bytes(&node));
        if measured + padding > priced {
            ledger::violation(format!(
                "underpriced leaf: measured {measured} B plus {padding} B of decode-slot \
                 padding, node_bytes priced {priced} B",
            ));
        }
        Ok(Self::wrap(node, measured))
    }
}

/// The two encoded version bounds a node keeps resident, in bytes.
fn bound_bytes<N>(node: &N) -> usize
where
    N: Node,
{
    let bounds = node.span();
    bounds.hi().as_bytes().len() + bounds.lo().as_bytes().len()
}

impl<B> Backend for Charged<B>
where
    B: Measure + Backend<Node<Z>: Leaf>,
{
    type Node<H: Height> = ChargedNode<B::Node<H>>;
    type Erased = ChargedNode<B::Erased>;
    type Error = B::Error;

    // Both conversions settle the wrapper's ledger entry and open one
    // around the re-tagged handle at its measured size: the running
    // total dips by one node's bytes between the two calls and never
    // rises, so the census peak is untouched exactly when the re-tag
    // leaves residency unchanged. That is the trait's re-tag clause (a
    // tag forgotten and restored "without changing the value";
    // `assume::<H>(erase::<H>(node)) == node`), checked here rather than
    // relied on: a re-tag that changes residency is recorded by name and
    // charged at what it measures.
    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
        let bytes = node.bytes;
        let erased = B::erase(node.into_inner());
        let measured = B::measure_erased(&erased);
        if measured != bytes {
            ledger::violation(format!(
                "re-tagged node changed residency: typed {bytes} B, erased {measured} B",
            ));
        }
        ChargedNode::wrap(erased, measured)
    }

    fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H> {
        let bytes = erased.bytes;
        let node = B::assume::<H>(erased.into_inner());
        let measured = B::measure::<H>(&node);
        if measured != bytes {
            ledger::violation(format!(
                "re-tagged node changed residency: erased {bytes} B, assumed {measured} B",
            ));
        }
        ChargedNode::wrap(node, measured)
    }

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
        // The presence contract: at least one real child yields a
        // parent, and a group with no real child yields none.
        if parent.is_some() != (fan > 0) {
            ledger::violation(format!(
                "parent contract: fan {fan} yielded {}",
                if parent.is_some() { "Some" } else { "None" },
            ));
        }
        Ok(parent.map(|node| {
            let measured = B::measure(&node);
            // The pointwise contract: the cost function evaluated at this
            // node's own fan and bounds must cover its measured bytes.
            let priced = B::node_bytes(fan, bound_bytes(&node));
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
            let bounds = node.span();
            let version_bytes = version_bytes
                .max(bounds.hi().as_bytes().len())
                .max(bounds.lo().as_bytes().len());
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

    fn children<H>(self, prefix: Prefix<S<H>>, parent: Self::Node<S<H>>) -> impl NodeStream<Self, H>
    where
        H: Height,
        S<H>: Height,
    {
        stream! {
            let mut children = pin!(self.inner.children(prefix, parent.into_inner()));
            while let Some(child) = children.next().await {
                yield child.map(|(prefix, node)| {
                    let measured = B::measure(&node);
                    // The pointwise contract at the explosion: an exploded
                    // child is one of the held references the window
                    // prices per depth, sitting in the reference slots
                    // whose padding beyond the pointer-class layout is
                    // the backend's to price. Its own fan is invisible
                    // here, so the price is taken at the widest fan it
                    // can have (its leaf count, capped at the radix),
                    // which the contract's monotonicity in fan makes an
                    // upper bound on the price at its own fan.
                    let padding = reference_slot_excess::<B, H>();
                    let priced = B::node_bytes(node.len().min(FAN), bound_bytes(&node));
                    if measured + padding > priced {
                        ledger::violation(format!(
                            "underpriced child: measured {measured} B plus {padding} B of \
                             reference-slot padding, node_bytes at the widest fan prices \
                             {priced} B",
                        ));
                    }
                    (prefix, ChargedNode::wrap(node, measured))
                });
            }
        }
    }

    fn leaves<H: Convert>(
        self,
        prefix: Prefix<H>,
        node: Self::Node<H>,
    ) -> impl NodeStream<Self, Z> {
        // Delegate to the wrapped backend's own override: the bulk walk
        // is the path the wire encoder runs, so its yields must land on
        // the census and answer the walked node's aggregates.
        let expected = node.len();
        let aggregate = node.version_bytes();
        stream! {
            let mut walked = 0usize;
            let mut failed = false;
            let mut previous: Option<Prefix<Z>> = None;
            let mut leaves = pin!(self.inner.leaves(prefix, node.into_inner()));
            while let Some(leaf) = leaves.next().await {
                failed |= leaf.is_err();
                yield leaf.map(|(leaf_prefix, leaf)| {
                    walked += 1;
                    // The containment and order clauses the trait states
                    // for an override: every yielded prefix extends the
                    // walked one, and the walk ascends strictly.
                    if Prefix::<H>::containing(&Path::from(leaf_prefix)) != prefix {
                        ledger::violation(format!(
                            "escaped leaf walk: a leaf at {leaf_prefix:?} yielded \
                             outside the walked prefix {prefix:?}",
                        ));
                    }
                    if let Some(before) = previous.replace(leaf_prefix)
                        && before >= leaf_prefix
                    {
                        ledger::violation(format!(
                            "unordered leaf walk: {leaf_prefix:?} yielded after {before:?}",
                        ));
                    }
                    let measured = B::measure(&leaf);
                    // The pointwise contract at the walk: a walked leaf
                    // is a fan slot the session budget prices at
                    // `children = 0`, its slot's padding included.
                    let padding = fan_slot_excess::<B>();
                    let priced = B::node_bytes(0, bound_bytes(&leaf));
                    if measured + padding > priced {
                        ledger::violation(format!(
                            "underpriced walked leaf: measured {measured} B plus {padding} B \
                             of decode-slot padding, node_bytes priced {priced} B",
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
                    (leaf_prefix, ChargedNode::wrap(leaf, measured))
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
        leaves: BoxNodeStream<'a, Self, Z>,
    ) -> impl NodeStream<Self, H> + 'a {
        // Delegate to the wrapped backend's own override: bulk assembly
        // is the path the wire decoder runs. The wrapper records each
        // maximal same-prefix run as it feeds the inner stream, then
        // holds every assembled node to account for its run.
        let runs: Arc<Mutex<BTreeMap<Prefix<H>, Run>>> = Arc::default();
        let recorded = Arc::clone(&runs);
        let supplied: BoxNodeStream<'a, B, Z> = Box::pin(leaves.map(move |item| {
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
            let mut previous: Option<Prefix<H>> = None;
            while let Some(item) = assembled.next().await {
                failed |= item.is_err();
                yield item.map(|(prefix, node)| {
                    // The order clause the trait states for an override:
                    // nodes come one per run, in run order, so their
                    // prefixes ascend strictly.
                    if let Some(before) = previous.replace(prefix)
                        && before >= prefix
                    {
                        ledger::violation(format!(
                            "unordered assembly: a node at {prefix:?} yielded after {before:?}",
                        ));
                    }
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
                        Some(run) => check_assembled::<B, H>(&node, measured, &run),
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
fn check_assembled<B, H>(node: &B::Node<H>, measured: usize, run: &Run)
where
    B: Backend<Node<Z>: Leaf>,
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
    let bounds = node.span();
    let floor = run
        .version_bytes
        .max(bounds.hi().as_bytes().len())
        .max(bounds.lo().as_bytes().len());
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
    let priced = B::node_bytes(run.leaves.min(FAN), bound_bytes(node));
    if measured > priced {
        ledger::violation(format!(
            "bulk-assembled node over-holds: measured {measured} B, \
             node_bytes at the run's widest fan prices {priced} B",
        ));
    }
}

/// Every version bound up to this many bytes is swept pairwise.
///
/// Far past the encodings the suite's corpora exchange (tens of bytes),
/// so a dip at any one bound a session at this scale can evaluate is a
/// grid point's neighbor. Dense costs nothing here: the cost function
/// is pure arithmetic.
const BOUND_DENSE_CEILING: usize = 4096;

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
///
/// The fan dimension is exhaustive; the bound dimension is a sample of
/// the family, dense where sessions evaluate and sparse above. A dip
/// that begins and recovers strictly between two sparse grid points
/// passes the sweep; the family itself (any bound, any increase) is
/// held by a property test over each backend in this module's tests.
fn node_bytes_monotone<B>()
where
    B: Measure + Clone,
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
/// each side, drives each corpus's sorted leaves through the default
/// fold and through the bulk assembly at a sub-root height (many runs)
/// and at the root (one),
/// walks each corpus through the bulk leaf walk at the root and, after
/// exploding the root, at each child's own prefix, and reconciles the
/// corpora twice: once at the zero-budget floor, once under
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
/// - any constructed, walked, exploded, or assembled node was
///   underpriced;
/// - a bulk seam mis-answered an aggregate, yielded out of order or
///   outside its prefix, merged, split, or swallowed a run, or
///   `parent` answered a real child with no parent or an empty group
///   with one;
/// - the stated budget resolves to the serialization floor, or the
///   budgeted run's census peak does not exceed the floor run's (the
///   admittance ceiling would otherwise compare two identical runs);
/// - the window's measured byte admittance exceeded the budget; or
/// - the session failed to converge the corpora, or converged them to
///   less than their union.
pub(crate) async fn check<B>(backend: B, budget_bytes: usize)
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    node_bytes_monotone::<B>();

    let (floor_peak, floor_stats) = run(backend.clone(), WindowConfig::Budget(0), 0).await;
    let (budget_peak, budget_stats) = run(backend, WindowConfig::Budget(budget_bytes), 0).await;
    eprintln!(
        "conformance census at budget {budget_bytes} B: floor run peaked at {floor_peak} B \
         (widest capacity {}), budgeted run at {budget_peak} B (widest capacity {})",
        floor_stats.window_granted, budget_stats.window_granted,
    );

    let violations = ledger::take_violations();
    assert!(
        violations.is_empty(),
        "the backend's node_bytes contract failed pointwise:\n{}",
        violations.join("\n"),
    );

    // The liveness floor under the admittance ceiling. A budget that
    // binds widens some stage past the one-scope serialization floor, and
    // a wider stage holds more node values in flight at the session's
    // peak instant than the floor does, so the least the census can read
    // is one node value more: equal peaks mean the two runs were one run
    // and the ceiling below could not fail.
    assert!(
        budget_peak > floor_peak,
        "the budgeted window admitted nothing above the floor: both runs peaked at \
         {floor_peak} B, so the budget does not bind at this corpus scale",
    );
    assert!(
        budget_stats.window_granted > 1,
        "the budget resolves to the serialization floor (widest capacity {}) yet the \
         census moved: the peak difference is not the window's",
        budget_stats.window_granted,
    );

    let admitted = budget_peak - floor_peak;
    assert!(
        admitted <= budget_bytes,
        "widening the window from the floor admitted {admitted} measured \
         bytes at peak; the stated budget is {budget_bytes}",
    );
}

/// One controlled-divergence reconciliation; returns the ledger's peak
/// measured bytes above the resting corpora, with the session's own
/// account of itself (the window it resolved, above all).
///
/// `omitted` shared messages are left out of *both* corpora: zero for
/// every honest run, and the completeness control's fault, a loss the
/// two sides' agreement cannot see.
pub(super) async fn run<B>(
    backend: B,
    window: WindowConfig,
    omitted: usize,
) -> (usize, SessionStats)
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    let charged = Charged::new(backend);

    // Two concurrent histories from one universe: the left clock produces the
    // shared corpus and its own tail; the right fork produces the other tail.
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

    let shared = common.iter().skip(omitted);
    let left_leaves = sorted_leaves::<B>(shared.clone().chain(&left_tail)).await;
    let right_leaves = sorted_leaves::<B>(shared.chain(&right_tail)).await;

    // The default fold over the charged backend: the path a backend
    // without an `assemble` override runs, and the one that brings every
    // interior group of a resting corpus through `parent`'s own checks
    // (presence, price, aggregates). First, so the first interior
    // `parent` call of a run is one those checks see.
    fold_default(&charged, left_leaves.clone()).await;
    fold_default(&charged, right_leaves.clone()).await;
    empty_group(&charged).await;

    // The bulk assembly boundary in the regime the wire decoder runs it:
    // many maximal same-prefix runs per stream, one node each, in run
    // order. A one-byte prefix partitions each corpus into up to `FAN`
    // runs at this scale. Before the corpora assemble at the root, so a
    // fault felt there is already named on the ledger.
    assemble_runs(&charged, left_leaves.clone()).await;
    assemble_runs(&charged, right_leaves.clone()).await;

    let left = corpus(&charged, left_leaves).await;
    let right = corpus(&charged, right_leaves).await;

    // The bulk walk boundary, exercised the way the wire encoder runs it:
    // every leaf of each corpus once from the root, then once more from
    // each of the root's children at the child's own prefix, where the
    // walk's containment clause has content and the explosion prices
    // every child. Before the baseline resets, so the checks land on
    // the ledger while the transient charges stay out of the session's
    // differenced peak.
    walk(&charged, &left).await;
    walk(&charged, &right).await;
    walk_children(&charged, &left).await;
    walk_children(&charged, &right).await;

    // The corpora are what exists regardless of the window; measure the
    // session's own admittance above them. The greeting's sizes need no
    // stating: they are the roots' own aggregates.
    ledger::reset_peak();

    // The client's recorder is read afterwards: the two sides exchange
    // the same pair of sizes, so both resolve the same window.
    let stats = Recorder::default();
    let client = materialized::Handshaking::start(charged.clone(), left)
        .window(window)
        .stats(stats.clone());
    let server = materialized::Handshaking::start(charged, right).window(window);
    let (ours, theirs) = streaming::mirror(client, server)
        .await
        .unwrap_or_else(|error| fail(&format!("the conformance session reconciles: {error:?}")));
    let peak = ledger::peak();

    // Convergence is agreement on one root; completeness is that root
    // holding the corpora's whole union, which agreement alone cannot
    // show when both sides lose the same leaves.
    let (converged, reconciled) = match (ours.root, theirs.root) {
        (Some(left_root), Some(right_root)) => {
            let left_root = Charged::<B>::erase(left_root);
            let right_root = Charged::<B>::erase(right_root);
            (left_root.hash() == right_root.hash(), left_root.len())
        }
        _ => (false, 0),
    };
    if !converged {
        fail("the conformance session must converge both corpora to one root");
    }
    let union = COMMON + 2 * DIVERGENT;
    if reconciled != union {
        fail(&format!(
            "the conformance session converged short: the root holds {reconciled} \
             messages; the corpora are built to hold {union} between them",
        ));
    }
    (peak, stats.snapshot())
}

/// Drain one corpus's bulk leaf walk from the root so the walk's checks
/// run.
async fn walk<B>(charged: &Charged<B>, corpus: &Root<Charged<B>>)
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    let Some(root) = corpus.root.clone() else {
        return;
    };
    let mut leaves = pin!(charged.clone().leaves(Prefix::<height::Root>::new(), root));
    while let Some(leaf) = leaves.next().await {
        leaf.unwrap_or_else(|error| fail(&format!("corpus leaves walk at rest: {error:?}")));
    }
}

/// Explode one corpus's root into its children and drain each child's
/// bulk leaf walk at the child's own prefix.
///
/// The explosion prices every child; the walk's containment clause has
/// content only when the walked prefix is not the root's empty one.
async fn walk_children<B>(charged: &Charged<B>, corpus: &Root<Charged<B>>)
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    let Some(root) = corpus.root.clone() else {
        return;
    };
    let mut children = pin!(
        charged
            .clone()
            .children::<height::UnderRoot>(Prefix::<height::Root>::new(), root)
    );
    while let Some(child) = children.next().await {
        let (prefix, child) = child
            .unwrap_or_else(|error| fail(&format!("corpus children explode at rest: {error:?}")));
        let mut leaves = pin!(charged.clone().leaves(prefix, child));
        while let Some(leaf) = leaves.next().await {
            leaf.unwrap_or_else(|error| {
                fail(&format!("corpus subtree leaves walk at rest: {error:?}"))
            });
        }
    }
}

/// One corpus's leaves, constructed through the charged backend and
/// sorted into path order.
// The inline pair type is clearer than a name coined only to satisfy the
// lint.
#[allow(clippy::type_complexity)]
async fn sorted_leaves<B>(
    messages: impl Iterator<Item = &(Version, u64)>,
) -> Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)>
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    let mut leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)> = Vec::new();
    for (version, payload) in messages {
        let message = Message::new(*payload);
        let path = Path::for_leaf(version);
        let leaf = <ChargedNode<B::Node<Z>> as Leaf>::leaf(version.clone(), message)
            .await
            .unwrap_or_else(|error| fail(&format!("corpus leaves construct at rest: {error:?}")));
        leaves.push((Prefix::from(path), leaf));
    }
    leaves.sort_by_key(|(prefix, _)| *prefix);
    leaves
}

/// Fold sorted leaves up to the height just under the root through the
/// default level-by-level assembly over the charged backend, and drain
/// it: every interior group passes through [`Charged::parent`].
#[allow(clippy::type_complexity)]
async fn fold_default<B>(charged: &Charged<B>, leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)>)
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    let mut folded = pin!(<height::UnderRoot as Convert>::assemble::<Charged<B>>(
        charged.clone(),
        Box::pin(futures_stream::iter(leaves.into_iter().map(Ok))),
    ));
    while let Some(node) = folded.next().await {
        node.unwrap_or_else(|error| fail(&format!("corpus leaves fold at rest: {error:?}")));
    }
}

/// Present `parent` with an empty group, the trait's stated case of a
/// scope that resolved to nothing at all, which must answer `None`.
///
/// Driven at rest because the session over this suite's corpora never
/// produces one: a mixed scope keeps at least one unpruned child, and
/// no resolution here comes back empty. Without this drive the
/// presence check's second direction has no reachable input.
async fn empty_group<B>(charged: &Charged<B>)
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    let answered = charged
        .clone()
        .parent::<height::UnderRoot>(Prefix::<height::Root>::new(), Vec::new())
        .await
        .unwrap_or_else(|error| fail(&format!("an empty group assembles at rest: {error:?}")));
    drop(answered);
}

/// Drive sorted leaves through the charged backend's bulk assembly at
/// the height just under the root, one node per one-byte prefix, and
/// drain it: the multi-run regime, held to account by the assembly's
/// own checks.
#[allow(clippy::type_complexity)]
async fn assemble_runs<B>(charged: &Charged<B>, leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)>)
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    let mut assembled = pin!(
        charged
            .clone()
            .assemble::<height::UnderRoot>(Box::pin(futures_stream::iter(
                leaves.into_iter().map(Ok)
            )))
    );
    while let Some(node) = assembled.next().await {
        node.unwrap_or_else(|error| fail(&format!("corpus runs assemble at rest: {error:?}")));
    }
}

/// Assemble one corpus's sorted leaves through the charged backend into
/// its root.
#[allow(clippy::type_complexity)]
async fn corpus<B>(
    charged: &Charged<B>,
    leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)>,
) -> Root<Charged<B>>
where
    B: Measure + Clone,
    B::Error: std::fmt::Debug,
{
    // The spans are bound to a local so their borrowed join endpoints
    // outlive the fold's iterator.
    let bounds: Vec<Span<'_>> = leaves.iter().map(|(_, leaf)| leaf.span()).collect();
    let ceiling: Version = bounds.iter().map(|bounds| bounds.hi()).sum();

    let mut assembled = pin!(
        charged
            .clone()
            .assemble::<height::Root>(Box::pin(futures_stream::iter(leaves.into_iter().map(Ok))))
    );
    let root = assembled
        .next()
        .await
        .unwrap_or_else(|| fail("a non-empty corpus assembles a root"))
        .unwrap_or_else(|error| fail(&format!("corpus assembly is infallible at rest: {error:?}")));
    if assembled.next().await.is_some() {
        fail("one sorted run assembles exactly one root");
    }
    Root {
        ceiling,
        root: Some(root.1),
    }
}

#[cfg(test)]
mod tests;
