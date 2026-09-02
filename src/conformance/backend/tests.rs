//! The backend conformance suite, run against this crate's backends and
//! against reference backends built to prove the suite has teeth.

use std::convert::Infallible;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_stream::stream;
use futures::{Stream, StreamExt, stream as futures_stream};
use proptest::prelude::*;

use before::Span;

use super::{
    BOUND_SWEEP_CEILING, Charged, Measure, check, fan_slot_excess, ledger, node_bytes_monotone,
    reference_slot_excess, run,
};
use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::{
            Backend, BoxNodeStream, ErasedNode, Leaf, Local, Node, NodeStream,
            convert::Convert,
            window::{FAN, SUPPLY_DECODE_ENVELOPE_BYTES, WindowConfig},
        },
        typed::{
            self, Hash, Path, Prefix,
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

    /// Spend one unit of the knob's value: true while it was positive.
    ///
    /// For knobs that count faults to inject rather than bytes to add;
    /// the guard restores the honest value on drop regardless.
    fn take(&self) -> bool {
        self.cell
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_sub(1)
            })
            .is_ok()
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

    fn measure_erased(node: &Self::Erased) -> usize {
        std::mem::size_of_val(node)
    }
}

/// The stated budget the in-memory check runs under.
///
/// It must clear the flat decode-fan pre-charge the window solve takes
/// off every budget before widening any stage
/// ([`SUPPLY_DECODE_ENVELOPE_BYTES`], the in-memory pricing of that
/// term) with room left for dispute scopes; a budget at or below the
/// pre-charge resolves to the serialization floor, and `check`'s
/// liveness floor fails it by name. The 64 KiB above the pre-charge
/// widens the suite's corpora to a few scopes per stage (a widest
/// capacity of four), so the window binds and is seen to.
const LOCAL_BUDGET: usize = SUPPLY_DECODE_ENVELOPE_BYTES + 64 * 1024;

/// The in-memory backend's pointer-priced account holds end to end.
///
/// `Local` is the trivial case -- handles into a resident tree -- so the
/// suite's pointwise check reduces to the pointer-size constant, and the
/// end-to-end census confirms the window's byte admittance under
/// [`LOCAL_BUDGET`]: the budgeted run widens the window past the floor
/// and its census peak exceeds the floor run's.
#[test]
fn local_backend_conforms() {
    let _serial = serialized();
    pollster::block_on(check(Local, LOCAL_BUDGET));
}

/// A budget that covers only the flat decode-fan pre-charge fails the
/// liveness floor by name.
///
/// It leaves nothing for dispute scopes: the solve floors every capacity
/// at one, the budgeted run is the floor run, and the census reads one
/// peak twice, which the admittance ceiling alone would pass vacuously.
#[test]
#[should_panic(expected = "admitted nothing above the floor")]
fn a_pre_charge_only_budget_fails_the_liveness_floor() {
    let _serial = serialized();
    pollster::block_on(check(Local, SUPPLY_DECODE_ENVELOPE_BYTES));
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
///
/// Its node is aligned wider than a pointer ([`MaterializedNode`]), so
/// the decode-fan and reference slots the session keeps it in pad
/// beyond the in-memory layout the window's slot constants derive from;
/// that padding ([`SLOT_PADDING`]) is the backend's to price, and
/// [`PRICED_SLOT_PADDING`] is the knob that prices it honestly at rest.
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

/// Whether the bulk walk yields its first two leaves in swapped order:
/// honest at zero, unordered above it.
static LEAVES_SWAP: Knob = Knob::new(0);

/// Whether the bulk walk re-tags its first leaf's prefix outside the
/// walked subtree: honest at zero, escaping above it.
static WALK_ESCAPES: Knob = Knob::new(0);

/// Extra bytes the exploded interior child rows keep resident (leaf
/// children stay honest, so the walk's leaf check is silent): honest at
/// zero, over-holding above it.
static CHILDREN_SLACK: Knob = Knob::new(0);

/// Interior `parent` calls (a fan of at least one real child) answered
/// with `Ok(None)`, spent one per call: honest at zero.
static PARENT_DROPS: Knob = Knob::new(0);

/// Extra bytes bulk-assembled rows keep resident: honest at zero,
/// over-holding above it.
static ASSEMBLE_SLACK: Knob = Knob::new(0);

/// Leaves bulk assembly silently drops: honest at zero.
static ASSEMBLE_SKIPS: Knob = Knob::new(0);

/// Whether bulk assembly yields its first two nodes in swapped order:
/// honest at zero, unordered above it.
static ASSEMBLE_SWAP: Knob = Knob::new(0);

/// The one-based position of the assembled node bulk assembly folds and
/// then swallows: honest at zero (none swallowed).
static ASSEMBLED_DROPS: Knob = Knob::new(0);

/// The one-based position of the assembled node bulk assembly re-tags to
/// its neighbor's prefix: honest at zero (none re-tagged).
static ASSEMBLE_RETAGS: Knob = Knob::new(0);

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

/// The version bound above which [`Materializing::node_bytes`] prices
/// [`STEP_DROP`] fewer header bytes: honest at zero (no step), a
/// step-shaped monotonicity dip above it.
static PRICED_BOUND_STEP: Knob = Knob::new(0);

/// The one version bound at which [`Materializing::node_bytes`] prices
/// one byte less than at the bound before it: honest at zero (no dip),
/// a point dip above it.
static PRICED_BOUND_DIP: Knob = Knob::new(0);

/// The bound the point-dip control dips at: inside the dense sweep and
/// off every power of two and its neighbors, so only a sweep dense
/// through it compares across the dip.
const DIP_BOUND: usize = 101;

/// The slot padding [`Materializing::node_bytes`] prices: honest at the
/// real [`SLOT_PADDING`], lying below it.
static PRICED_SLOT_PADDING: Knob = Knob::new(SLOT_PADDING);

/// Extra bytes a row keeps after [`Backend::assume`] re-tags it: honest
/// at zero, a residency-changing re-tag above it.
static ASSUME_SLACK: Knob = Knob::new(0);

/// Extra bytes a row keeps after [`Backend::erase`] re-tags it: honest
/// at zero, a residency-changing re-tag above it.
static ERASE_SLACK: Knob = Knob::new(0);

/// The threshold bound the step control places strictly inside the
/// sweep grid's widest gap, between the neighbors of the two largest
/// powers of two: no adjacent grid pair straddles it, so the grid alone
/// cannot see the step.
const STEP_THRESHOLD: usize = (BOUND_SWEEP_CEILING / 2 + 1 + BOUND_SWEEP_CEILING - 1) / 2;

/// The header bytes the step drops: the width of that gap, the largest
/// drop the gap's two grid endpoints still ascend across.
const STEP_DROP: usize = (BOUND_SWEEP_CEILING - 1) - (BOUND_SWEEP_CEILING / 2 + 1);

/// The bytes the session's slots pad around a [`MaterializedNode`]
/// beyond the in-memory layout the window prices: the larger of the
/// decode-fan slot's and the reference slots' excess, so one price
/// covers both.
const SLOT_PADDING: usize = {
    let fan = fan_slot_excess::<Materializing>();
    let reference = reference_slot_excess::<Materializing, Z>();
    if fan > reference { fan } else { reference }
};

// The control below zeroes the priced padding; it convicts only if the
// real padding is positive, which the node's alignment guarantees.
const _: () = assert!(
    SLOT_PADDING > 0,
    "a node aligned wider than a pointer pads its slots",
);

/// The one fan [`PRICED_DIP`] carves the dip at: an arbitrary interior
/// value the monotonicity sweep's adjacent-fan comparisons must cross.
const DIP_FAN: usize = 7;

/// The bytes a materialized row spends beyond its child table and bounds.
const ROW_HEADER: usize = 64;

/// The bytes one child entry occupies in a materialized row.
const ROW_ENTRY: usize = 24;

/// The stated budget every materializing check runs under.
///
/// The rows make this backend's flat decode-fan pre-charge several
/// times the in-memory one (each fan slot carries a header and bounds,
/// not a pointer); 4 MiB clears it with room for a window tens of
/// scopes wide per stage, which `check`'s liveness floor holds it to,
/// while the rows keep the admitted bytes a real fraction of the budget.
const MATERIALIZING_BUDGET: usize = 4 * 1024 * 1024;

/// A node value that owns its simulated row.
///
/// Aligned wider than a pointer, as a handle carrying a 16-byte row id
/// would be: the shape the window's slot constants say owes its extra
/// slot padding to its own price.
#[derive(Clone, Debug)]
#[repr(align(16))]
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
    // movement from either conversion -- until the re-tag-lying knobs
    // ([`ERASE_SLACK`], [`ASSUME_SLACK`]) grow the row across it.
    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
        let MaterializedNode { inner, mut row } = node;
        row.resize(row.len() + ERASE_SLACK.get(), 0);
        MaterializedNode {
            inner: inner.into_untyped(),
            row,
        }
    }

    fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H> {
        let MaterializedNode { inner, mut row } = erased;
        row.resize(row.len() + ASSUME_SLACK.get(), 0);
        MaterializedNode {
            inner: typed::Node::from_untyped(inner),
            row,
        }
    }

    fn node_bytes(children: usize, version_bound: usize) -> usize {
        let priced = std::mem::size_of::<MaterializedNode<typed::Node<Z>>>()
            + PRICED_SLOT_PADDING.get()
            + PRICED_HEADER.get()
            + ROW_ENTRY * children
            + version_bound;
        // The monotonicity-lying knobs: a dip at one fan, invisible to
        // every check that does not compare across it, and a step in
        // the bound, invisible to a grid whose points all sit on one
        // side of it.
        let priced = if children == DIP_FAN {
            priced.saturating_sub(PRICED_DIP.get())
        } else {
            priced
        };
        let priced = if version_bound != 0 && version_bound == PRICED_BOUND_DIP.get() {
            priced.saturating_sub(2)
        } else {
            priced
        };
        let step = PRICED_BOUND_STEP.get();
        if step != 0 && version_bound > step {
            priced.saturating_sub(STEP_DROP)
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
        // The presence-lying knob: an interior fan answered with no
        // parent, the fault the trait's `parent` clause forbids.
        if fan > 0 && PARENT_DROPS.take() {
            return Ok(None);
        }
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
                    // table not yet materialized. The over-holding knob
                    // ([`CHILDREN_SLACK`]) inflates interior rows only:
                    // leaf-height children re-enter the walk's own leaf
                    // check through `leaves`, and the control's subject
                    // is the check on exploded interior references.
                    let slack = if H::HEIGHT > 0 { CHILDREN_SLACK.get() } else { 0 };
                    let row = ROW_HEADER + bounds_of(&node) + slack;
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
        // that drop leaves ([`WALK_SKIPS`]), inflate the yielded rows
        // ([`WALK_SLACK`]), re-tag the first leaf out of the walked
        // subtree ([`WALK_ESCAPES`]), and swap the first two leaves
        // ([`LEAVES_SWAP`]) — honest at rest, the negative controls'
        // subject when set.
        let walked = H::explode::<Self>(
            self,
            Box::pin(futures_stream::once(async move { Ok((prefix, node)) })),
        )
        .skip(WALK_SKIPS.get())
        .enumerate()
        .map(|(index, item)| {
            item.map(|(prefix, mut leaf)| {
                leaf.row.resize(leaf.row.len() + WALK_SLACK.get(), 0);
                let prefix = if index == 0 && WALK_ESCAPES.get() != 0 {
                    escaped(prefix)
                } else {
                    prefix
                };
                (prefix, leaf)
            })
        });
        swapped_head(LEAVES_SWAP.get() != 0, walked)
    }

    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, Z>,
    ) -> impl NodeStream<Self, H> + 'a {
        // The reference bulk assembly: the default fold behind knobs
        // that drop supplied leaves ([`ASSEMBLE_SKIPS`]), inflate the
        // assembled rows ([`ASSEMBLE_SLACK`]), swallow one assembled
        // node ([`ASSEMBLED_DROPS`]), re-tag one to its neighbor's
        // prefix ([`ASSEMBLE_RETAGS`]), and swap the first two
        // ([`ASSEMBLE_SWAP`]) — honest at rest, the negative controls'
        // subject when set.
        let supplied: BoxNodeStream<'a, Self, Z> = Box::pin(leaves.skip(ASSEMBLE_SKIPS.get()));
        let assembled = H::assemble(self, supplied)
            .enumerate()
            .filter_map(|(index, item)| async move {
                (index + 1 != ASSEMBLED_DROPS.get()).then_some((index, item))
            })
            .map(|(index, item)| {
                item.map(|(prefix, mut node)| {
                    node.row.resize(node.row.len() + ASSEMBLE_SLACK.get(), 0);
                    let prefix = if index + 1 == ASSEMBLE_RETAGS.get() {
                        neighbor(prefix)
                    } else {
                        prefix
                    };
                    (prefix, node)
                })
            });
        swapped_head(ASSEMBLE_SWAP.get() != 0, assembled)
    }
}

impl Measure for Materializing {
    fn measure<H: Height>(node: &Self::Node<H>) -> usize {
        std::mem::size_of_val(node) + node.row.len()
    }

    fn measure_erased(node: &Self::Erased) -> usize {
        std::mem::size_of_val(node) + node.row.len()
    }
}

/// `inner` with its first two items in swapped order when `swap` holds:
/// the order-lying adaptor behind the walk's and the assembly's controls.
fn swapped_head<T>(swap: bool, inner: impl Stream<Item = T>) -> impl Stream<Item = T> {
    stream! {
        let mut inner = pin!(inner);
        if swap {
            let first = inner.next().await;
            let second = inner.next().await;
            for item in [second, first].into_iter().flatten() {
                yield item;
            }
        }
        while let Some(item) = inner.next().await {
            yield item;
        }
    }
}

/// A leaf prefix moved out of every subtree but the root's: its first
/// path byte flipped at the top bit.
fn escaped(prefix: Prefix<Z>) -> Prefix<Z> {
    let mut path = <[u8; 32]>::from(prefix);
    path[0] ^= 0x80;
    Prefix::from(path)
}

/// The prefix's neighbor at its own height, one radix bit over on its
/// last byte; the root prefix has no neighbor and is returned as is.
fn neighbor<H: Height>(prefix: Prefix<H>) -> Prefix<H> {
    let depth = prefix.as_bytes().len();
    if depth == 0 {
        return prefix;
    }
    let mut path = [0u8; 32];
    path[..depth].copy_from_slice(prefix.as_bytes());
    path[depth - 1] ^= 1;
    Prefix::<H>::containing(&Path::from(Prefix::<Z>::from(path)))
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

/// A `parent` that answers a fan of real children with no parent is
/// convicted by name at the presence check.
///
/// The knob spends itself on the first interior `parent` call of the
/// run, which is the default fold's over the charged decorator, where
/// the check lives; this backend's own bulk assembly folds through its
/// `parent` directly, where the same fault is felt only as a short run.
#[test]
#[should_panic(expected = "parent contract: fan")]
fn an_interior_parent_answering_none_fails_the_presence_check() {
    let _dishonest = PARENT_DROPS.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A bulk walk that yields two leaves out of path order is convicted by
/// name at the walk's order check: count and prices are unchanged, so
/// nothing else would notice.
#[test]
#[should_panic(expected = "unordered leaf walk")]
fn a_swapped_walk_fails_the_order_check() {
    let _dishonest = LEAVES_SWAP.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A bulk walk that yields a leaf outside the walked prefix is convicted
/// by name at the walk's containment check, which has content at the
/// sub-root walk the run drives (every leaf is inside the root's empty
/// prefix).
#[test]
#[should_panic(expected = "escaped leaf walk")]
fn an_escaping_walk_fails_the_containment_check() {
    let _dishonest = WALK_ESCAPES.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A `children` override whose exploded interior rows over-hold memory
/// is convicted by name at the explosion's pointwise check.
///
/// The walk's leaf check never sees an interior child, so this check is
/// the only one that prices the references the window prices per depth.
#[test]
#[should_panic(expected = "underpriced child")]
fn overholding_children_fail_the_pointwise_check() {
    let _dishonest = CHILDREN_SLACK.set(BULK_OVERHOLD);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// Bulk assembly that yields two nodes out of run order is convicted by
/// name at the assembly's order check, in the multi-run regime the run
/// drives at a sub-root height (the root assembly has one node to
/// order).
#[test]
#[should_panic(expected = "unordered assembly")]
fn a_swapped_assembly_fails_the_order_check() {
    let _dishonest = ASSEMBLE_SWAP.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// Bulk assembly that folds a run and then swallows its node is
/// convicted by name: the run it supplied never yielded a node.
///
/// The same fault at the root assembly costs the corpus its root, and
/// the by-name report rides in that failure rather than being masked by
/// it.
#[test]
#[should_panic(expected = "unassembled run")]
fn a_swallowed_assembled_node_fails_the_run_check() {
    let _dishonest = ASSEMBLED_DROPS.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// Bulk assembly that yields a node at a prefix other than its own run's
/// is convicted by name: a node arrives at a prefix no run supplied.
#[test]
#[should_panic(expected = "unsupplied assembly")]
fn a_re_tagged_assembled_node_fails_the_run_check() {
    let _dishonest = ASSEMBLE_RETAGS.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A loss both sides share is invisible to their agreement and caught
/// by the completeness oracle: the reconciled root must hold the
/// corpora's whole union.
///
/// Driven at the run level with one shared message left out of both
/// corpora, the fault a symmetric backend defect produces and the only
/// shape the two-sided convergence check cannot see.
#[test]
#[should_panic(expected = "converged short")]
fn a_symmetric_loss_fails_the_completeness_check() {
    let _serial = serialized();
    pollster::block_on(run(Materializing, WindowConfig::Budget(0), 1));
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

/// A backend whose node is aligned wider than a pointer and whose price
/// omits the slot padding that alignment costs is convicted by name at
/// the first leaf it constructs.
///
/// The window's slot constants derive from the in-memory layout and
/// leave a wider layout's extra padding to the backend's price; the
/// leaf check folds that excess into its comparison, so pricing the
/// node value alone falls short by exactly the padding.
#[test]
#[should_panic(expected = "of decode-slot padding")]
fn unpriced_slot_padding_fails_the_pointwise_check() {
    let _dishonest = PRICED_SLOT_PADDING.set(0);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A re-tag that grows the node's residency on the way back to a typed
/// handle is convicted by name: the measurement after `assume` differs
/// from the bytes the erased handle carried.
#[test]
#[should_panic(expected = "re-tagged node changed residency: erased")]
fn a_residency_changing_assume_fails_the_re_tag_check() {
    let _dishonest = ASSUME_SLACK.set(BULK_OVERHOLD);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// A re-tag that grows the node's residency on the way to an erased
/// handle is convicted by name: the measurement after `erase` differs
/// from the bytes the typed handle carried.
#[test]
#[should_panic(expected = "re-tagged node changed residency: typed")]
fn a_residency_changing_erase_fails_the_re_tag_check() {
    let _dishonest = ERASE_SLACK.set(BULK_OVERHOLD);
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

/// One case of the bound-monotonicity property: the price at `bound`
/// does not exceed the price at any larger bound, at one fan.
fn assert_monotone_in_bound<B: Backend<Node<Z>: Leaf>>(fan: usize, bound: usize, delta: usize) {
    let there = bound.saturating_add(delta);
    let here_priced = B::node_bytes(fan, bound);
    let there_priced = B::node_bytes(fan, there);
    assert!(
        here_priced <= there_priced,
        "node_bytes must be monotone in version bound: bound {bound} prices {here_priced} B, \
         bound {there} prices {there_priced} B, at fan {fan}",
    );
}

proptest! {
    /// The in-memory cost function is monotone in the version bound over
    /// the whole family: for any bound and any increase, at any fan, the
    /// price does not fall.
    ///
    /// The sweep in `check` samples adjacent grid points; this covers
    /// the pairs between them.
    #[test]
    fn local_node_bytes_is_monotone_in_the_version_bound(
        fan in 0..=FAN,
        bound in 0..=BOUND_SWEEP_CEILING,
        delta in 0..=BOUND_SWEEP_CEILING,
    ) {
        let _serial = serialized();
        assert_monotone_in_bound::<Local>(fan, bound, delta);
    }

    /// The materializing cost function is monotone in the version bound
    /// over the whole family: for any bound and any increase, at any fan,
    /// the price does not fall.
    ///
    /// The sweep in `check` samples adjacent grid points; this covers
    /// the pairs between them.
    #[test]
    fn materializing_node_bytes_is_monotone_in_the_version_bound(
        fan in 0..=FAN,
        bound in 0..=BOUND_SWEEP_CEILING,
        delta in 0..=BOUND_SWEEP_CEILING,
    ) {
        let _serial = serialized();
        assert_monotone_in_bound::<Materializing>(fan, bound, delta);
    }

    /// A step-shaped dip strictly inside a grid gap fails the
    /// bound-monotonicity property by name: the case the grid sweep
    /// cannot see, which the sibling control shows it passing.
    ///
    /// The knob is set for each case's lifetime; the failing case's seed
    /// is committed, so the conviction replays deterministically.
    #[test]
    #[should_panic(expected = "monotone in version bound")]
    fn a_step_dip_fails_the_monotonicity_property(
        fan in 0..=FAN,
        bound in 0..=BOUND_SWEEP_CEILING,
        delta in 0..=BOUND_SWEEP_CEILING,
    ) {
        let _dishonest = PRICED_BOUND_STEP.set(STEP_THRESHOLD);
        assert_monotone_in_bound::<Materializing>(fan, bound, delta);
    }
}

/// A point dip at one bound inside the dense sweep is caught by the
/// sweep's adjacent-bound comparison before any session runs: the
/// price at [`DIP_BOUND`] falls one byte below the bound before it.
#[test]
#[should_panic(expected = "monotone in version bound")]
fn a_point_dip_fails_the_dense_sweep() {
    let _dishonest = PRICED_BOUND_DIP.set(DIP_BOUND);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}

/// The same step-shaped dip passes the grid sweep: every adjacent grid
/// pair sits on one side of the threshold, so the sweep is a sample of
/// the family and the property test above is what holds the family.
#[test]
fn a_step_dip_hides_from_the_grid_sweep() {
    let _dishonest = PRICED_BOUND_STEP.set(STEP_THRESHOLD);
    node_bytes_monotone::<Materializing>();
}
