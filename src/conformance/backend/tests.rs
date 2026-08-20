//! The backend conformance suite, run against this crate's backends and
//! against reference backends built to prove the suite has teeth.

use std::convert::Infallible;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_stream::stream;
use futures::{StreamExt, stream as futures_stream};

use before::Span;

use super::{Charged, Measure, check, ledger};
use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::{
            Backend, BoxNodeStream, ErasedNode, Leaf, Local, Node, NodeStream, convert::Convert,
        },
        typed::{
            self, Hash, Prefix,
            height::{Height, S, Z},
        },
    },
};

/// Serializes the tests in this module.
///
/// The census ledger and every honesty knob are process-global, so tests
/// overlapping in one process (plain `cargo test` runs tests as threads)
/// would reprice each other's in-flight sessions and drain each other's
/// violations. Every test holds this lock for its whole body — through
/// [`serialized`] directly or through a [`Knob`] guard — which makes
/// plain `cargo test` sound, not merely nextest's process-per-test.
static SERIAL: Mutex<()> = Mutex::new(());

/// A test's hold on [`SERIAL`]; dropping it drains any violations the
/// test left on the ledger, so a failure cannot leak into its successor.
struct Serialized {
    _guard: MutexGuard<'static, ()>,
}

impl Drop for Serialized {
    fn drop(&mut self) {
        let _ = ledger::take_violations();
    }
}

/// Take the module's serialization lock for one test's lifetime.
///
/// A `should_panic` test poisons the lock by design; the poison guards
/// no invariant here (knob guards restore honesty and [`Serialized`]
/// drains the ledger), so it clears and continues.
fn serialized() -> Serialized {
    Serialized {
        _guard: SERIAL.lock().unwrap_or_else(PoisonError::into_inner),
    }
}

/// A reference-backend honesty knob: process-global state resting at an
/// honest value.
///
/// State rather than a const parameter deliberately: every distinct
/// backend type instantiates the whole height-indexed protocol tower
/// (measured at +0.7 GiB of rustc peak memory per additional
/// instantiation), so the honest and lying variants must share one type.
/// A knob rests at its honest value, [`set`](Knob::set) is the only way
/// to move it, and the guard `set` returns holds [`SERIAL`] and restores
/// honesty on drop — a test that never sets a knob gets the honest
/// backend.
struct Knob {
    cell: AtomicUsize,
    honest: usize,
}

impl Knob {
    const fn new(honest: usize) -> Self {
        Self {
            cell: AtomicUsize::new(honest),
            honest,
        }
    }

    fn get(&self) -> usize {
        self.cell.load(Ordering::Relaxed)
    }

    /// Move the knob off its honest value for one test's lifetime.
    fn set(&'static self, value: usize) -> Dishonest {
        let serial = serialized();
        self.cell.store(value, Ordering::Relaxed);
        Dishonest {
            knob: self,
            _serial: serial,
        }
    }
}

/// A lying test's hold: the serialization guard plus the obligation to
/// restore the knob's honest value on drop.
struct Dishonest {
    knob: &'static Knob,
    _serial: Serialized,
}

impl Drop for Dishonest {
    fn drop(&mut self) {
        self.knob.cell.store(self.knob.honest, Ordering::Relaxed);
    }
}

impl Measure for Local {
    fn measure<H: Height>(node: &Self::Node<H>) -> usize {
        // A `Local` node is an `Arc` handle into the session-resident
        // tree: its shallow size is everything it keeps resident.
        std::mem::size_of_val(node)
    }
}

/// The in-memory backend's pointer-priced account holds end to end.
///
/// `Local` is the trivial case — handles into a resident tree — so the
/// suite's pointwise check reduces to the pointer-size constant, and the
/// end-to-end census confirms the window's byte admittance under a tight
/// budget that genuinely binds at this scale.
#[test]
fn local_backend_conforms() {
    let _serial = serialized();
    pollster::block_on(check(Local, 64 * 1024));
}

/// A materializing reference backend, shaped like a database row store.
///
/// Each node value owns a buffer sized like the row it would keep
/// resident (header, child table, version bounds), on top of a `Local`
/// handle standing in for the store.
///
/// The priced header is the honesty knob ([`PRICED_HEADER`]): the real
/// row header is [`ROW_HEADER`] bytes and the cost function prices the
/// knob's value, so the knob's honest resting value is the real header
/// and anything less underprices — what the lying tests opt into
/// through the knob's guard.
#[derive(Clone, Copy, Debug)]
struct Materializing;

/// The header bytes [`Materializing::node_bytes`] prices: honest at the
/// real [`ROW_HEADER`], lying below it.
static PRICED_HEADER: Knob = Knob::new(ROW_HEADER);

/// Extra bytes the bulk walk's yielded rows keep resident: honest at
/// zero, over-holding above it.
static WALK_SLACK: Knob = Knob::new(0);

/// Leaves the bulk walk silently drops: honest at zero.
static WALK_SKIPS: Knob = Knob::new(0);

/// Extra bytes bulk-assembled rows keep resident: honest at zero,
/// over-holding above it.
static ASSEMBLE_SLACK: Knob = Knob::new(0);

/// Leaves bulk assembly silently drops: honest at zero.
static ASSEMBLE_SKIPS: Knob = Knob::new(0);

/// Bytes subtracted from every node's `version_bytes` answer: honest at
/// zero, deflating the aggregate above it.
static VERSION_DEFLATE: Knob = Knob::new(0);

/// Bytes added to a leaf's `version_bytes` answer: honest at zero,
/// pushing a leaf's claimed encoding past its parents' aggregates above
/// it.
static LEAF_VERSION_INFLATE: Knob = Knob::new(0);

/// Bytes [`Materializing::node_bytes`] subtracts at [`DIP_FAN`] alone:
/// honest at zero, a monotonicity dip above it.
static PRICED_DIP: Knob = Knob::new(0);

/// The one fan [`PRICED_DIP`] carves the dip at: an arbitrary interior
/// value the monotonicity sweep's adjacent-fan comparisons must cross.
const DIP_FAN: usize = 7;

/// The bytes a materialized row spends beyond its child table and bounds.
const ROW_HEADER: usize = 64;

/// The bytes one child entry occupies in a materialized row.
const ROW_ENTRY: usize = 24;

/// The stated budget every materializing check runs under: the rows
/// make it genuinely binding at the suite's corpus scale.
const MATERIALIZING_BUDGET: usize = 4 * 1024 * 1024;

/// A node value that owns its simulated row.
#[derive(Clone, Debug)]
struct MaterializedNode<N> {
    inner: N,
    row: Vec<u8>,
}

impl<N> MaterializedNode<N> {
    fn wrap(inner: N, row_bytes: usize) -> Self {
        Self {
            inner,
            row: vec![0; row_bytes],
        }
    }
}

impl<H> Node for MaterializedNode<typed::Node<H>>
where
    H: Height,
{
    type Backend = Materializing;
    type Height = H;

    fn span(&self) -> Span<'_> {
        self.inner.span()
    }

    fn hash(&self) -> Hash {
        self.inner.hash()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn version_bytes(&self) -> usize {
        // The aggregate-lying knobs: a deflated answer must be caught by
        // the assembly seam's floor, and an inflated leaf answer by the
        // walk seam's aggregate-membership check.
        let inflate = if H::HEIGHT == 0 {
            LEAF_VERSION_INFLATE.get()
        } else {
            0
        };
        self.inner
            .version_bytes()
            .saturating_sub(VERSION_DEFLATE.get())
            + inflate
    }
}

impl Leaf for MaterializedNode<typed::Node<Z>> {
    fn message(&self) -> &Message {
        self.inner.message()
    }

    async fn leaf(version: Version, message: Message) -> Result<Self, Infallible> {
        // Eager persistence at the conversion boundary: the payload is
        // written to the store here, so the resident row keeps only the
        // header and bounds — the thin-handle shape the leaf seam prices
        // at `node_bytes(0, bounds)`.
        let node = typed::Node::leaf(version, message);
        let row = ROW_HEADER + bounds_of(&node);
        Ok(Self::wrap(node, row))
    }
}

/// The two encoded bounds of a freshly built node, in bytes.
fn bounds_of<H: Height>(node: &typed::Node<H>) -> usize {
    node.ceiling().as_bytes().len() + node.floor().as_bytes().len()
}

// The erased observations pass through the row wrapper; the row itself
// carries no readable state.
impl<E: ErasedNode> ErasedNode for MaterializedNode<E> {
    fn span(&self) -> Span<'_> {
        self.inner.span()
    }

    fn hash(&self) -> Hash {
        self.inner.hash()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl Backend for Materializing {
    type Node<H: Height> = MaterializedNode<typed::Node<H>>;
    type Erased = MaterializedNode<typed::untyped::Node>;
    type Error = Infallible;

    // Erasure re-tags the store's handle; the resident row rides along
    // unchanged, so the census this backend exists to exercise sees no
    // movement from either conversion.
    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
        let MaterializedNode { inner, row } = node;
        MaterializedNode {
            inner: inner.into_untyped(),
            row,
        }
    }

    fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H> {
        let MaterializedNode { inner, row } = erased;
        MaterializedNode {
            inner: typed::Node::from_untyped(inner),
            row,
        }
    }

    fn node_bytes(children: usize, version_bound: usize) -> usize {
        let priced = std::mem::size_of::<MaterializedNode<typed::Node<Z>>>()
            + PRICED_HEADER.get()
            + ROW_ENTRY * children
            + version_bound;
        // The monotonicity-lying knob: a dip at one fan, invisible to
        // every check that does not compare across it.
        if children == DIP_FAN {
            priced.saturating_sub(PRICED_DIP.get())
        } else {
            priced
        }
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
        let children = children
            .into_iter()
            .map(|(radix, child)| (radix, child.map(|child| child.inner)))
            .collect();
        let parent = <Local as Backend>::parent(Local, prefix, children).await?;
        Ok(parent.map(|node| {
            let row = ROW_HEADER + ROW_ENTRY * fan + bounds_of(&node);
            MaterializedNode::wrap(node, row)
        }))
    }

    fn children<H>(self, prefix: Prefix<S<H>>, parent: Self::Node<S<H>>) -> impl NodeStream<Self, H>
    where
        H: Height,
        S<H>: Height,
    {
        stream! {
            let mut children = pin!(<Local as Backend>::children(Local, prefix, parent.inner));
            while let Some(child) = children.next().await {
                yield child.map(|(prefix, node)| {
                    // A lazily loaded row: header and bounds, its child
                    // table not yet materialized.
                    let row = ROW_HEADER + bounds_of(&node);
                    (prefix, MaterializedNode::wrap(node, row))
                });
            }
        }
    }

    fn leaves<H: Convert>(
        self,
        prefix: Prefix<H>,
        node: Self::Node<H>,
    ) -> impl NodeStream<Self, Z> {
        // The reference bulk walk: the default explosion behind knobs
        // that drop leaves ([`WALK_SKIPS`]) and inflate the yielded rows
        // ([`WALK_SLACK`]) — honest at rest, the negative controls'
        // subject when set.
        H::explode::<Self>(
            self,
            Box::pin(futures_stream::once(async move { Ok((prefix, node)) })),
        )
        .skip(WALK_SKIPS.get())
        .map(|item| {
            item.map(|(prefix, mut leaf)| {
                leaf.row.resize(leaf.row.len() + WALK_SLACK.get(), 0);
                (prefix, leaf)
            })
        })
    }

    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, Z>,
    ) -> impl NodeStream<Self, H> + 'a {
        // The reference bulk assembly: the default fold behind knobs
        // that drop supplied leaves ([`ASSEMBLE_SKIPS`]) and inflate the
        // assembled rows ([`ASSEMBLE_SLACK`]) — honest at rest, the
        // negative controls' subject when set.
        let supplied: BoxNodeStream<'a, Self, Z> = Box::pin(leaves.skip(ASSEMBLE_SKIPS.get()));
        H::assemble(self, supplied).map(|item| {
            item.map(|(prefix, mut node)| {
                node.row.resize(node.row.len() + ASSEMBLE_SLACK.get(), 0);
                (prefix, node)
            })
        })
    }
}

impl Measure for Materializing {
    fn measure<H: Height>(node: &Self::Node<H>) -> usize {
        std::mem::size_of_val(node) + node.row.len()
    }
}

/// An honestly priced materializing backend passes the whole suite.
///
/// Rows own real bytes (header, per-child entries, encoded bounds), the
/// cost function covers each term, and the end-to-end census holds the
/// window's measured admittance inside a budget the rows make expensive.
/// Runs at the knobs' honest resting values: honesty is the default, not
/// something this test has to establish.
#[test]
fn materializing_backend_conforms() {
    let _serial = serialized();
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// An underpricing cost function fails the run by name.
///
/// The same backend with a header priced below the row's real header
/// must be caught by the pointwise check the moment a session assembles
/// a node — this is the suite's reason to exist, so its detection is
/// itself pinned.
#[test]
#[should_panic(expected = "underpriced node")]
fn underpricing_fails_the_pointwise_check() {
    let _dishonest = PRICED_HEADER.set(0);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// Row slack that clears the assembly seam's fan headroom.
///
/// The capped pointwise check prices at the run's widest fan, which
/// exceeds the honest row's own fan by at most the radix's worth of
/// entries ([`FAN`](crate::tree::mirror::streaming::window::FAN) ×
/// [`ROW_ENTRY`] bytes, 6 KiB); 64 KiB of slack is unambiguously past
/// it.
const BULK_OVERHOLD: usize = 64 * 1024;

/// A bulk walk that over-holds memory in its yielded rows is caught by
/// the walk seam's pointwise check: the negative control proving the
/// suite exercises and prices the backend's own `leaves` override.
///
/// One extra byte per row suffices: the honest reference walk is priced
/// exactly, so the check is tight at this seam.
#[test]
#[should_panic(expected = "underpriced walked leaf")]
fn overholding_walk_fails_the_pointwise_check() {
    let _dishonest = WALK_SLACK.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A bulk walk that silently drops a leaf is caught by the walk seam's
/// count against the walked node's exact `len` aggregate.
#[test]
#[should_panic(expected = "mis-sized leaf walk")]
fn short_walk_fails_the_len_check() {
    let _dishonest = WALK_SKIPS.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A bulk assembly whose nodes over-hold memory is caught by the
/// assembly seam's capped pointwise check: the negative control proving
/// the suite exercises and prices the backend's own `assemble` override.
#[test]
#[should_panic(expected = "bulk-assembled node over-holds")]
fn overholding_bulk_assembly_fails_the_capped_check() {
    let _dishonest = ASSEMBLE_SLACK.set(BULK_OVERHOLD);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// Bulk assembly that silently drops a supplied leaf is caught by the
/// assembly seam's exact `len` accounting of each leaf run.
#[test]
#[should_panic(expected = "bulk-assembled len")]
fn lossy_bulk_assembly_fails_the_len_check() {
    let _dishonest = ASSEMBLE_SKIPS.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A deflated `version_bytes` answer is caught by the assembly seam's floor.
///
/// The run's own leaf encodings and the node's two bounds are all in
/// the aggregate, so an answer below them is a lie: deflation is the
/// direction that breaches the memory envelope.
#[test]
#[should_panic(expected = "deflated version_bytes: bulk-assembled node")]
fn deflated_version_bytes_fails_the_assembly_floor() {
    let _dishonest = VERSION_DEFLATE.set(usize::MAX);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A leaf whose `version_bytes` answer exceeds the walked node's
/// aggregate is caught by the walk seam's membership check: every bound
/// under a node is in the node's aggregate.
#[test]
#[should_panic(expected = "walked leaf encodes")]
fn inflated_leaf_version_bytes_fails_the_walk_check() {
    let _dishonest = LEAF_VERSION_INFLATE.set(1024 * 1024);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A cost function with a dip between quantile evaluation points is
/// caught by the monotonicity sweep before any session runs.
///
/// The dip is the smallest adjacent fans reveal: one byte more than the
/// per-child entry, so [`DIP_FAN`] prices one byte below the fan before
/// it. The sweep runs first and catches it a priori — no session has to
/// happen to assemble a node at the dipped fan for the lie to surface.
#[test]
#[should_panic(expected = "monotone in children")]
fn dipping_node_bytes_fails_the_monotonicity_sweep() {
    let _dishonest = PRICED_DIP.set(ROW_ENTRY + 1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A leaf priced below its post-custody residency is caught at
/// construction: the negative control for the leaf seam of the account.
///
/// The session budget charges every decode-fan slot at
/// `node_bytes(0, bounds)`, so a backend whose leaf handles keep more
/// resident than that price must fail the pointwise check the moment
/// one is constructed — before any parent assembles.
#[test]
#[should_panic(expected = "underpriced leaf")]
fn leaf_underpricing_fails_at_construction() {
    let _dishonest = PRICED_HEADER.set(0);
    let leaf = pollster::block_on(
        <super::ChargedNode<MaterializedNode<typed::Node<Z>>> as Leaf>::leaf(
            Version::new(),
            Message::new(7),
        ),
    )
    .expect("the reference backend constructs leaves infallibly");
    drop(leaf);
    let violations = ledger::take_violations();
    assert!(
        !violations.is_empty(),
        "a lying leaf price must land a violation on the ledger",
    );
    panic!("{}", violations.join("\n"));
}

/// The decorator's ledger accounting is exact over wrap, clone, and drop.
///
/// A wrapped leaf charges its measured post-custody bytes, a cloned
/// handle charges its bytes again, and drops settle to the starting
/// balance — the arithmetic the end-to-end census rests on.
#[test]
fn ledger_settles_over_clone_and_drop() {
    let _serial = serialized();
    let before = {
        ledger::reset_peak();
        ledger::peak()
    };
    let leaf = pollster::block_on(<super::ChargedNode<typed::Node<Z>> as Leaf>::leaf(
        Version::new(),
        Message::new(7),
    ))
    .expect("a local leaf constructs infallibly");
    let handle = std::mem::size_of::<typed::Node<Z>>();
    let clone = leaf.clone();
    assert_eq!(
        ledger::peak(),
        before + 2 * handle,
        "two live leaf handles charge their measured bytes twice",
    );
    drop(leaf);
    drop(clone);

    let node: typed::Node<crate::tree::typed::height::Z> =
        typed::Node::leaf(Version::new(), Message::new(7));
    let charged = Charged::<Local>::new(Local);
    let _ = &charged;
    let wrapped = super::ChargedNode::wrap(node, 100);
    let clone = wrapped.clone();
    assert_eq!(
        ledger::peak(),
        before + 200,
        "two live handles charge twice"
    );
    drop(wrapped);
    drop(clone);
    assert_eq!(
        ledger::peak(),
        before + 200,
        "the peak persists after handles settle",
    );
}
