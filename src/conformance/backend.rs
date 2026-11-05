//! Conformance checks for storage backends used by streaming reconciliation.
//!
//! This suite wraps a backend so that every node value it builds is counted
//! at the size an independent [`Measure`] oracle reports, checks the
//! backend's operations as they run, then reconciles concrete fixtures at a
//! budget of zero and a larger budget. It checks:
//!
//! - **Residency:** height erasure and restoration preserve a node's
//!   measured residency.
//! - **Operations:** parent presence, node aggregates, ordered leaf walks
//!   confined to their prefix, one assembled node per supplied prefix run,
//!   child counts equal to the children listing a node yields, and
//!   explosion reports that cover what listing a node's children
//!   allocates, by the allocation meter, within the backend's bound on
//!   them.
//! - **Reconciliation:** both replicas converge to a set of the expected union
//!   size, and any measured peak increase over the zero-budget run must fit
//!   within the larger budget.
//!
//! # What the census measures
//!
//! The ledger counts each live node value at its measured size. For a leaf,
//! this is what remains resident after [`Leaf::leaf`] has had the opportunity
//! to persist its payload. Corpus handles still alive during reconciliation
//! are included; shared tree storage and wire buffers are not.
//!
//! The peak comparison is fixture-specific. Changing the budget can change
//! when nodes are created and released, so subtracting the zero-budget run's
//! peak does not isolate the budgeted work, and a larger budget need not
//! produce a higher peak. The comparison does not prove a hard memory
//! bound.
//!
//! The ledger is process-global. Tests must hold the suite's serialization
//! lock so their charges and violation reports cannot interfere.
//!
//! # Scope
//!
//! The backend interface is crate-internal, so this suite runs in the crate's
//! own tests.

use std::collections::BTreeMap;
use std::future::poll_fn;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_stream::stream;
use before::Span;
use futures::{Stream, StreamExt, stream as futures_stream};

use crate::{
    DEFAULT_TARGET_MESSAGE_SIZE, Version,
    counter::widen,
    message::Message,
    tree::{
        mirror::streaming::{
            self, Backend, BoxNodeStream, ErasedNode, Leaf, Node, NodeStream, Root,
            convert::Convert, materialized, ops,
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
/// keeps alive per session reference. The census counts every node at
/// it, and the re-tag clause is checked against it.
pub(crate) trait Measure: Backend<Node<Z>: Leaf> {
    /// The actual resident bytes of one node value, measured.
    fn measure<H: Height>(node: &Self::Node<H>) -> usize;

    /// The actual resident bytes of one height-erased node value,
    /// measured: the oracle for the re-tag clause in the
    /// [`erase`](Backend::erase) direction.
    fn measure_erased(node: &Self::Erased) -> usize;
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

    /// Add a live node's measured bytes and update the peak.
    pub(super) fn charge(bytes: usize) {
        let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
        PEAK.fetch_max(live, Ordering::Relaxed);
    }

    /// Remove the charge for a node that leaves the census.
    pub(super) fn discharge(bytes: usize) {
        LIVE.fetch_sub(bytes, Ordering::Relaxed);
    }

    /// Retain a contract failure for the session's final report.
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

    /// Return the most measured bytes concurrently alive since the last reset.
    pub(super) fn peak() -> usize {
        PEAK.load(Ordering::Relaxed)
    }

    /// Drain the recorded contract failures.
    pub(super) fn take_violations() -> Vec<String> {
        std::mem::take(
            &mut VIOLATIONS
                .lock()
                .expect("the violation ledger mutex is not poisoned"),
        )
    }
}

/// Run `work`, returning the bytes it allocated on this thread beside its
/// output.
///
/// The census charges node values by the [`Measure`] oracle, which sees
/// residency, not allocation; this meter sees what a backend allocates to
/// explode a node, whether or not the copies it makes stay resident.
fn allocated_by<T>(work: impl FnOnce() -> T) -> (u64, T) {
    let (stats, output) = alloc_meter::measure(work);
    (widen(stats.bytes_allocated), output)
}

/// Fail with the recorded violations so a later symptom cannot hide its cause.
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

/// Wrap a backend to check its operations and count its live node values.
///
/// Delegates to the backend's own bulk overrides so the suite exercises the
/// same assembly and leaf-walk implementations that wire sessions use.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Charged<B> {
    /// The backend whose operations are being checked.
    inner: B,
}

/// Construct a backend wrapper without changing its storage.
impl<B> Charged<B> {
    /// Start checking operations on `inner`.
    pub(crate) fn new(inner: B) -> Self {
        Self { inner }
    }
}

/// A node value counted in the live-byte census until dropped or unwrapped.
///
/// The inner slot is an `Option` only so consuming the wrapper can settle
/// the ledger exactly once: `into_inner` discharges and empties it, and
/// `Drop` discharges only when it is still full.
#[derive(Debug)]
pub(crate) struct ChargedNode<N> {
    /// Present until the node is consumed by `into_inner`.
    inner: Option<N>,
    /// The node's measured charge, released when it leaves the wrapper.
    bytes: usize,
}

/// Track a node for as long as it is held by the conformance wrapper.
impl<N> ChargedNode<N> {
    /// Enter a measured node in the census.
    fn wrap(inner: N, bytes: usize) -> Self {
        ledger::charge(bytes);
        Self {
            inner: Some(inner),
            bytes,
        }
    }

    /// Borrow the node without changing its charge.
    fn inner(&self) -> &N {
        self.inner
            .as_ref()
            .expect("a live charged node always holds its inner handle")
    }

    /// Release the charge and return the node, preventing a second discharge on drop.
    fn into_inner(mut self) -> N {
        ledger::discharge(self.bytes);
        self.inner
            .take()
            .expect("a live charged node always holds its inner handle")
    }
}

/// Count each cloned handle as another live node value.
impl<N: Clone> Clone for ChargedNode<N> {
    /// Clone the node and charge the copy independently.
    fn clone(&self) -> Self {
        Self::wrap(self.inner().clone(), self.bytes)
    }
}

/// Settle the charge unless `into_inner` already released it.
impl<N> Drop for ChargedNode<N> {
    /// Remove this node's charge exactly once.
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

    /// Read the wrapped node's grandchild count.
    fn grandchild_count(&self) -> usize {
        self.inner().grandchild_count()
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

    /// Read the wrapped node's grandchild count.
    fn grandchild_count(&self) -> usize {
        self.inner().grandchild_count()
    }

    /// Read the wrapped node's explosion cost, and record a violation if it
    /// exceeds the backend's bound.
    ///
    /// The bound is checked at every read rather than at the listing: the
    /// walk fails the session on a report over the bound before it lists
    /// the node, so a listing never sees one.
    fn explosion_bytes(&self) -> usize {
        let reported = self.inner().explosion_bytes();
        if reported > E::MAX_EXPLOSION_BYTES {
            ledger::violation(format!(
                "explosion over its bound: a node answers {reported} B, over the backend's \
                 MAX_EXPLOSION_BYTES of {} B",
                E::MAX_EXPLOSION_BYTES,
            ));
        }
        reported
    }

    /// The wrapped node's bound on its explosion cost.
    const MAX_EXPLOSION_BYTES: usize = E::MAX_EXPLOSION_BYTES;

    /// Read the wrapped node's child count.
    fn child_count(&self) -> usize {
        self.inner().child_count()
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
        Ok(Self::wrap(node, measured))
    }
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
        // The grandchild contract: the parent answers the children its
        // children list, each child at the leaves counting one. The count
        // explodes each child through the wrapped backend, outside the
        // ledger, whose census follows only wrapped nodes.
        let stored = parent.grandchild_count();
        // The explosion contract: listing allocates, beyond the rows the
        // handles it yields keep resident, no more than the parent's
        // explosion report, read as the walk reads it, on the erased
        // handle just before the listing. The allocation meter counts the
        // wrapped backend's own work alone: building its stream and each
        // poll of it, not the grandchild count's explosions between polls.
        let erased = B::erase(parent.into_inner());
        let reported = erased.explosion_bytes();
        // The child-count contract: listing yields exactly the children the
        // parent reports, read as the walk reads it before listing.
        let children_reported = erased.child_count();
        let parent = B::assume::<S<H>>(erased);
        let counter = self.inner.clone();
        stream! {
            let mut counted = 0_u64;
            let mut listed = 0_u64;
            let mut failed = false;
            let (mut allocated, children) = allocated_by(|| self.inner.children(prefix, parent));
            let mut rows = 0_u64;
            let mut children = pin!(children);
            while let Some(child) = poll_fn(|cx| {
                let (bytes, poll) = allocated_by(|| children.as_mut().poll_next(cx));
                allocated += bytes;
                poll
            })
            .await
            {
                if child.is_ok() {
                    listed += 1;
                }
                match &child {
                    Ok(_) if H::HEIGHT == 0 => counted += 1,
                    Ok((child_prefix, node)) => {
                        match ops::children_of(&counter, child_prefix.erase(), B::erase(node.clone())).await {
                            Ok(grandchildren) => counted += widen(grandchildren.len()),
                            // A child whose own listing breaks its count is
                            // the child-count contract's failure, not one to
                            // pass over.
                            Err(materialized::Error::Count(error)) => {
                                ledger::violation(format!("miscounted children: {error}"));
                                failed = true;
                            }
                            Err(_) => failed = true,
                        }
                    }
                    Err(_) => failed = true,
                }
                yield child.map(|(prefix, node)| {
                    let measured = B::measure(&node);
                    rows += widen(measured);
                    (prefix, ChargedNode::wrap(node, measured))
                });
            }
            if !failed && listed != widen(children_reported) {
                ledger::violation(format!(
                    "miscounted children: the parent answers {children_reported}, \
                     its listing yields {listed}",
                ));
            }
            if !failed && counted != widen(stored) {
                ledger::violation(format!(
                    "mis-propagated grandchildren: parent answers {stored}, \
                     its children list {counted}",
                ));
            }
            if !failed && allocated.saturating_sub(rows) > widen(reported) {
                ledger::violation(format!(
                    "undercounted explosion: the parent answers {reported} B, its listing \
                     allocated {allocated} B beside {rows} B of yielded rows",
                ));
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
            let mut walked = 0_u64;
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
            if !failed && walked != widen(expected) {
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
                        Some(run) => check_assembled::<B, H>(&node, &run),
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
    leaves: u64,
    /// The largest supplied version encoding: a floor on the assembled
    /// node's `version_bytes` aggregate.
    version_bytes: usize,
}

/// Hold one bulk-assembled node to its run's account.
fn check_assembled<B, H>(node: &B::Node<H>, run: &Run)
where
    B: Backend<Node<Z>: Leaf>,
    H: Height,
{
    // The len aggregate is exact: a run's node answers its leaf count.
    if widen(node.len()) != run.leaves {
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
}

/// Messages each side holds before the fork: the shared corpus.
const COMMON: usize = 512;

/// Messages each side originates alone: the mutual divergence.
const DIVERGENT: usize = 1_024;

/// Check node residency, bulk operations, and reconciliation under two
/// budgets.
///
/// Reconciles the same divergent corpora at a budget of zero and at
/// `budget_bytes`. Any increase in the measured peak must fit the budget,
/// but the budgeted run may also peak lower: the budget changes the
/// schedule as well as available space.
/// Callers must serialize checks because the ledger is process-global.
///
/// # Panics
///
/// Panics with the violated clause if:
///
/// - a re-tag changed a node's measured residency;
/// - a bulk seam mis-answered an aggregate, yielded out of order or
///   outside its prefix, merged, split, or swallowed a run, or
///   `parent` answered a real child with no parent or an empty group
///   with one;
/// - a node listed a different number of children than its child count
///   reports;
/// - a node's explosion report fell short of what listing its children
///   allocated, or exceeded the backend's bound on every report;
/// - the measured peak increase exceeds the budget; or
/// - the session failed to converge the corpora, or converged them to
///   less than their union.
pub(crate) async fn check<B>(backend: B, budget_bytes: usize)
where
    B: Measure,
    B::Error: std::fmt::Debug,
{
    let floor_peak = run(backend.clone(), 0, 0).await;
    let budget_peak = run(backend, budget_bytes, 0).await;
    eprintln!(
        "conformance census at budget {budget_bytes} B: the zero-budget run peaked at \
         {floor_peak} B, the budgeted run at {budget_peak} B",
    );

    let violations = ledger::take_violations();
    assert!(
        violations.is_empty(),
        "the backend's contract failed pointwise:\n{}",
        violations.join("\n"),
    );

    let increase = budget_peak.saturating_sub(floor_peak);
    assert!(
        increase <= budget_bytes,
        "the measured peak increased by {increase} bytes; the stated budget is {budget_bytes}",
    );
}

/// Versions and payloads for the shared history and each replica's additions.
///
/// The deterministic tick order also lets path fixtures preassign these
/// versions without creating trees outside their scoped path mapping.
fn histories() -> [Vec<(Version, u64)>; 3] {
    // Two concurrent histories from one universe: the left clock produces the
    // shared corpus and its own tail; the right fork produces the other tail.
    let mut left_clock = before::Clock::seed();
    let mut right_clock = left_clock.fork();

    let common: Vec<(Version, u64)> = (0..widen(COMMON))
        .map(|payload| (left_clock.tick().clone(), payload))
        .collect();
    let left_tail: Vec<(Version, u64)> = (0..widen(DIVERGENT))
        .map(|payload| (left_clock.tick().clone(), 1 << 32 | payload))
        .collect();
    let right_tail: Vec<(Version, u64)> = (0..widen(DIVERGENT))
        .map(|payload| (right_clock.tick().clone(), 2 << 32 | payload))
        .collect();

    [common, left_tail, right_tail]
}

/// Reconcile divergent corpora and return the absolute node census peak.
///
/// `omitted` shared messages are left out of *both* corpora to test that
/// agreement alone cannot satisfy the completeness check. Pass zero for
/// a complete fixture. Corpus construction is excluded from the peak, but
/// corpus handles still alive during reconciliation are included.
pub(super) async fn run<B>(backend: B, budget: usize, omitted: usize) -> usize
where
    B: Measure,
    B::Error: std::fmt::Debug,
{
    let charged = Charged::new(backend);
    let [common, left_tail, right_tail] = histories();

    let shared = common.iter().skip(omitted);
    let left_leaves = sorted_leaves::<B>(shared.clone().chain(&left_tail)).await;
    let right_leaves = sorted_leaves::<B>(shared.chain(&right_tail)).await;

    // Exercise `parent` through the default fold before using bulk overrides.
    // Faults injected on the first parent call must encounter its checks.
    fold_default(&charged, left_leaves.clone()).await;
    fold_default(&charged, right_leaves.clone()).await;
    empty_group(&charged).await;

    // Assemble below the root to exercise multiple prefix runs in one stream.
    // Do this first so a fault reports its specific violation before it can
    // prevent construction of the session's roots.
    assemble_runs(&charged, left_leaves.clone()).await;
    assemble_runs(&charged, right_leaves.clone()).await;

    let left = corpus(&charged, left_leaves).await;
    let right = corpus(&charged, right_leaves).await;

    // Walk at the root and at each child's prefix: the latter can expose
    // leaves escaping their subtree. These extra checks run before the peak
    // resets so they do not contribute to the session measurement.
    walk(&charged, &left).await;
    walk(&charged, &right).await;
    walk_children(&charged, &left).await;
    walk_children(&charged, &right).await;

    // Start the peak at the currently live corpus handles. The measurement
    // includes them and any later handles held by either endpoint.
    ledger::reset_peak();

    let client =
        materialized::Handshaking::start(charged.clone(), left, DEFAULT_TARGET_MESSAGE_SIZE as u64)
            .budget(budget);
    let server =
        materialized::Handshaking::start(charged, right, DEFAULT_TARGET_MESSAGE_SIZE as u64)
            .budget(budget);
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
    peak
}

/// Drain one corpus's bulk leaf walk from the root so the walk's checks
/// run.
async fn walk<B>(charged: &Charged<B>, corpus: &Root<Charged<B>>)
where
    B: Measure,
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
/// The explosion counts every child; the walk's containment clause has
/// content only when the walked prefix is not the root's empty one.
async fn walk_children<B>(charged: &Charged<B>, corpus: &Root<Charged<B>>)
where
    B: Measure,
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
    B: Measure,
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
    B: Measure,
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
    B: Measure,
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
    B: Measure,
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
    B: Measure,
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
