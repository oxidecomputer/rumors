//! Backend conformance tests and fault-injection controls for each check.

use std::convert::Infallible;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_stream::stream;
use futures::{Stream, StreamExt, stream as futures_stream};
use proptest::prelude::*;

use before::Span;

use super::{Measure, check, histories, ledger, run};
use crate::{
    Version,
    message::Message,
    testing::run_to_quiescence,
    tree::{
        mirror::streaming::{
            Backend, BoxNodeStream, ErasedNode, Leaf, Local, Node, NodeStream, QueueKind, Summary,
            byte_set::ByteSet, convert::Convert, with_kind_capacity,
        },
        typed::{
            self, ErasedPrefix, Hash, HashKey, Path, Prefix,
            height::{Height, S, Z},
        },
    },
};

/// Serializes the tests in this module.
///
/// The census ledger and every knob are process-global, so tests
/// overlapping in one process (plain `cargo test` runs tests as threads)
/// would change each other's in-flight sessions and drain each other's
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

/// The in-memory backend's operations keep their contracts, and
/// reconciliation at [`CENSUS_BUDGET`] stays within its peak-increase
/// allowance.
#[test]
fn local_backend_conforms() {
    let _serial = serialized();
    pollster::block_on(check(Local, CENSUS_BUDGET));
}

/// A materializing reference backend, shaped like a database row store.
///
/// Each node value owns a buffer sized like the row it would keep
/// resident (header, child table, version bounds), on top of a `Local`
/// handle standing in for the store. Its handle is wider and more aligned
/// than a pointer ([`MaterializedNode`]).
#[derive(Clone, Copy, Debug)]
struct Materializing;

/// Leaves the bulk walk silently drops: honest at zero.
static WALK_SKIPS: Knob = Knob::new(0);

/// Whether the bulk walk yields its first two leaves in swapped order:
/// honest at zero, unordered above it.
static LEAVES_SWAP: Knob = Knob::new(0);

/// Whether the bulk walk re-tags its first leaf's prefix outside the
/// walked subtree: honest at zero, escaping above it.
static WALK_ESCAPES: Knob = Knob::new(0);

/// Interior `parent` calls (a fan of at least one real child) answered
/// with `Ok(None)`, spent one per call: honest at zero.
static PARENT_DROPS: Knob = Knob::new(0);

/// Whether an empty group presented to `parent` is answered with a
/// conjured node instead of `None`: honest at zero.
static PARENT_CONJURES: Knob = Knob::new(0);

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

/// Grandchildren added to every branch's `grandchildren` answer: honest at
/// zero, an overcount of the children its children list above it.
static GRANDCHILDREN_SKEW: Knob = Knob::new(0);

/// Bytes added to a leaf's `version_bytes` answer: honest at zero,
/// pushing a leaf's claimed encoding past its parents' aggregates above
/// it.
static LEAF_VERSION_INFLATE: Knob = Knob::new(0);

/// Children added to every node's `child_count` answer: honest at zero, an
/// overcount of the children listing it yields above it.
static CHILDREN_SKEW: Knob = Knob::new(0);

/// Whether every node's `child_radices` answer drops its radices: honest
/// at zero, an answer of no radices for a node with children above it.
static RADICES_DROP: Knob = Knob::new(0);

/// Bytes listing each child allocates and frees, beyond the row it
/// yields: honest at zero, scratch the listing never reports above it.
static LISTING_SCRATCH: Knob = Knob::new(0);

/// Extra bytes a row keeps after [`Backend::assume`] re-tags it: honest
/// at zero, a residency-changing re-tag above it.
static ASSUME_SLACK: Knob = Knob::new(0);

/// Extra bytes a row keeps after [`Backend::erase`] re-tags it: honest
/// at zero, a residency-changing re-tag above it.
static ERASE_SLACK: Knob = Knob::new(0);

/// The bytes a materialized row spends beyond its child table and bounds.
const ROW_HEADER: usize = 64;

/// The bytes one child entry occupies in a materialized row.
const ROW_ENTRY: usize = 24;

/// The budget each census runs its backend's session at beside a budget of
/// zero, 4 MiB, and the most the larger run's peak may exceed the
/// zero-budget run's.
const CENSUS_BUDGET: usize = 4 * 1024 * 1024;

/// A node value that owns its simulated row.
///
/// Aligned wider than a pointer, as a handle carrying a 16-byte row id
/// would be.
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

/// Supply node metadata, with controllable size errors for conformance tests.
impl<H> Node for MaterializedNode<typed::View<H>>
where
    H: Height,
{
    type Backend = Materializing;
    type Height = H;

    /// Borrow the underlying node's valid version bounds.
    fn span(&self) -> Span<'_> {
        self.inner.span()
    }

    /// Return the underlying node's exact leaf count.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Read the underlying node's grandchild count, skewed above the leaves
    /// to exercise a miscounting backend.
    fn grandchild_count(&self) -> usize {
        let skew = if H::HEIGHT == 0 {
            0
        } else {
            GRANDCHILDREN_SKEW.get()
        };
        self.inner.grandchild_count() + skew
    }

    /// Alter the version-size aggregate to exercise undercounts and overcounts.
    fn version_bytes(&self) -> usize {
        // Assembly checks must catch undercounts; leaf walks must catch
        // inflated leaf sizes that disagree with the stored aggregate.
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

impl Leaf for MaterializedNode<typed::View<Z>> {
    fn message(&self) -> &Message {
        self.inner.message()
    }

    async fn leaf(version: Version, message: Message) -> Result<Self, Infallible> {
        // Eager persistence at the conversion boundary: the payload is
        // written to the store here, so the resident row keeps only the
        // header and bounds.
        let node = typed::View::leaf(version, message);
        let row = ROW_HEADER + bounds_of(&node);
        Ok(Self::wrap(node, row))
    }
}

/// The two encoded bounds of a freshly built node, in bytes.
fn bounds_of<H: Height>(node: &typed::View<H>) -> usize {
    let span = node.span();
    span.hi().as_bytes().len() + span.lo().as_bytes().len()
}

// The erased observations pass through the row wrapper; the row itself
// carries no readable state.
impl<E: ErasedNode> ErasedNode for MaterializedNode<E> {
    fn span(&self) -> Span<'_> {
        self.inner.span()
    }

    fn hash(&self, at: &ErasedPrefix, key: &HashKey) -> Hash {
        self.inner.hash(at, key)
    }

    /// Read the wrapped node's summary, its child count skewed as
    /// [`child_count`](ErasedNode::child_count) skews it.
    fn summary(&self, at: &ErasedPrefix, key: &HashKey) -> Summary {
        let summary = self.inner.summary(at, key);
        Summary {
            child_count: summary.child_count + CHILDREN_SKEW.get(),
            ..summary
        }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Read the wrapped node's grandchild count.
    fn grandchild_count(&self, at: &ErasedPrefix) -> usize {
        self.inner.grandchild_count(at)
    }

    /// Read the wrapped node's child count, skewed to exercise a backend
    /// whose count disagrees with its listing.
    fn child_count(&self, at: &ErasedPrefix) -> usize {
        self.inner.child_count(at) + CHILDREN_SKEW.get()
    }

    /// Read the wrapped node's child radices, dropped to exercise a backend
    /// whose radices disagree with its listing.
    fn child_radices(&self, at: &ErasedPrefix) -> ByteSet {
        if RADICES_DROP.get() > 0 {
            return ByteSet::EMPTY;
        }
        self.inner.child_radices(at)
    }
}

impl Backend for Materializing {
    type Node<H: Height> = MaterializedNode<typed::View<H>>;
    type Erased = MaterializedNode<typed::ErasedView>;
    type Error = Infallible;

    // Erasure re-tags the store's handle; the resident row rides along
    // unchanged, so the census this backend exists to exercise sees no
    // movement from either conversion -- until the re-tag-lying knobs
    // ([`ERASE_SLACK`], [`ASSUME_SLACK`]) grow the row across it.
    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
        let MaterializedNode { inner, mut row } = node;
        row.resize(row.len() + ERASE_SLACK.get(), 0);
        MaterializedNode {
            inner: <Local as Backend>::erase(inner),
            row,
        }
    }

    fn assume<H: Height>(at: &Prefix<H>, erased: Self::Erased) -> Self::Node<H> {
        let MaterializedNode { inner, mut row } = erased;
        row.resize(row.len() + ASSUME_SLACK.get(), 0);
        MaterializedNode {
            inner: <Local as Backend>::assume(at, inner),
            row,
        }
    }

    async fn parent<H>(
        self,
        prefix: Prefix<S<H>>,
        original: Option<Self::Node<S<H>>>,
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
        // The other direction of the presence lie: a group with no real
        // child answered with a node conjured from nothing (a leaf under a
        // path reaching the parent's height; the check on trial reads only
        // the answer's presence).
        if fan == 0 && PARENT_CONJURES.get() != 0 {
            let leaf = typed::untyped::Node::leaf(Version::new(), Message::new(0));
            let lifted = (0..S::<H>::HEIGHT).fold(leaf, |node, _| node.beneath(0));
            let conjured = typed::View::from(typed::Node::<S<H>>::from_untyped(lifted));
            return Ok(Some(MaterializedNode::wrap(conjured, ROW_HEADER)));
        }
        let children = children
            .into_iter()
            .map(|(radix, child)| (radix, child.map(|child| child.inner)))
            .collect();
        let original = original.map(|original| original.inner);
        let parent = <Local as Backend>::parent(Local, prefix, original, children).await?;
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
                // Scratch the listing fills and frees, held nowhere.
                std::hint::black_box(Vec::<u8>::with_capacity(LISTING_SCRATCH.get()));
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
        // that drop leaves ([`WALK_SKIPS`]), re-tag the first leaf out of the walked
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
            item.map(|(prefix, leaf)| {
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
        // that drop supplied leaves ([`ASSEMBLE_SKIPS`]), swallow one assembled
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
                item.map(|(prefix, node)| {
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

/// The row store's operations keep their contracts, and reconciliation
/// stays within the fixture's peak-increase allowance.
#[test]
fn materializing_backend_conforms() {
    let _serial = serialized();
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A larger budget can reconcile a clustered fixture without increasing its
/// peak, with the outgoing replies' edge at one slot.
///
/// The edge's progress argument needs one slot; its second slot lets a
/// budgeted walk queue a second reply chunk ahead of its encoder, and the
/// rows that chunk's nodes keep alive raise this fixture's budgeted peak
/// above the zero-budget run's, by 12,216 bytes on the row store, well
/// within the budget the check allows. At one slot both runs peak alike,
/// which is the witness this test needs: the census comparison must accept
/// a budgeted peak no higher than the zero-budget run's.
#[test]
fn a_larger_budget_need_not_increase_the_peak() {
    let _serial = serialized();
    // Put every leaf below one 30-byte prefix, so every dispute sits deep.
    let paths = histories()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, (version, _))| {
            let mut path = [0; 32];
            path[30..].copy_from_slice(&u16::try_from(index).unwrap().to_be_bytes());
            (version, Path::from(path))
        });
    with_kind_capacity(QueueKind::OutgoingResponses, 1, || {
        Path::with_leaf_paths(paths, || {
            let floor = run_to_quiescence(run(Materializing, 0, 0))
                .expect("the zero-budget session must complete");
            let wider = run_to_quiescence(run(Materializing, CENSUS_BUDGET, 0))
                .expect("the budgeted session must complete");
            assert!(wider <= floor, "budgeted {wider}, zero-budget {floor}");
            run_to_quiescence(check(Materializing, CENSUS_BUDGET))
                .expect("conformance must accept the lower or equal peak");
        })
    });
}

/// Row slack far past any row the reference backend builds: a full fan's
/// child table is [`FAN`](crate::tree::mirror::streaming::FAN) ×
/// [`ROW_ENTRY`] bytes, 6 KiB.
const BULK_OVERHOLD: usize = 64 * 1024;

/// A bulk walk that silently drops a leaf is caught by the walk seam's
/// count against the walked node's exact `len` aggregate.
#[test]
#[should_panic(expected = "mis-sized leaf walk")]
fn short_walk_fails_the_len_check() {
    let _dishonest = WALK_SKIPS.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// Bulk assembly that silently drops a supplied leaf is caught by the
/// assembly seam's exact `len` accounting of each leaf run.
#[test]
#[should_panic(expected = "bulk-assembled len")]
fn lossy_bulk_assembly_fails_the_len_check() {
    let _dishonest = ASSEMBLE_SKIPS.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
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
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A `parent` that answers an empty group with a node is convicted by
/// name at the presence check's other direction, on the empty group the
/// run presents at rest.
#[test]
#[should_panic(expected = "parent contract: fan 0 yielded Some")]
fn a_parent_conjured_from_an_empty_group_fails_the_presence_check() {
    let _dishonest = PARENT_CONJURES.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A bulk walk that yields two leaves out of path order is convicted by
/// name at the walk's order check: the count is unchanged, so nothing else
/// would notice.
#[test]
#[should_panic(expected = "unordered leaf walk")]
fn a_swapped_walk_fails_the_order_check() {
    let _dishonest = LEAVES_SWAP.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A bulk walk that yields a leaf outside the walked prefix is convicted
/// by name at the walk's containment check, which has content at the
/// sub-root walk the run drives (every leaf is inside the root's empty
/// prefix).
#[test]
#[should_panic(expected = "escaped leaf walk")]
fn an_escaping_walk_fails_the_containment_check() {
    let _dishonest = WALK_ESCAPES.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// Bulk assembly that yields two nodes out of run order is convicted by
/// name at the assembly's order check, in the multi-run regime the run
/// drives at a sub-root height (the root assembly has one node to
/// order).
#[test]
#[should_panic(expected = "unordered assembly")]
fn a_swapped_assembly_fails_the_order_check() {
    let _dishonest = ASSEMBLE_SWAP.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
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
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// Bulk assembly that yields a node at a prefix other than its own run's
/// is convicted by name.
///
/// When no run supplied the re-tagged prefix, the re-tagged node itself
/// arrives unsupplied. When the neighbor has a run (the case at this
/// corpus, where nearly every one-byte prefix does), the re-tagged node
/// consumes that run and is convicted as a `bulk-assembled len`
/// mismatch, and the honest neighbor that follows finds its run gone:
/// `unsupplied assembly` fires either way.
#[test]
#[should_panic(expected = "unsupplied assembly")]
fn a_re_tagged_assembled_node_fails_the_run_check() {
    let _dishonest = ASSEMBLE_RETAGS.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
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
    pollster::block_on(run(Materializing, 0, 1));
}

/// A listing that allocates beyond the rows it yields is convicted by
/// name at the listing's allocation check.
///
/// The census counts each listed child at its handle's inline bytes beside
/// its row, though listing allocates only the row; so each child listed
/// allocates scratch one byte wider than a handle.
#[test]
#[should_panic(expected = "allocating listing")]
fn an_allocating_listing_fails_the_listing_check() {
    let _dishonest = LISTING_SCRATCH.set(size_of::<MaterializedNode<typed::View<Z>>>() + 1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// Drain one listing of `node`, viewed whole at `prefix`, through the
/// charged in-memory backend, returning the contract violations it
/// recorded.
fn violations_listing<H>(prefix: Prefix<S<H>>, node: typed::Node<S<H>>) -> Vec<String>
where
    H: Height,
    S<H>: Height,
{
    let view = typed::View::from(node);
    let measured = Local::measure(&view);
    let parent = super::ChargedNode::wrap(view, measured);
    let listed = pollster::block_on(
        super::Charged::new(Local)
            .children(prefix, parent)
            .collect::<Vec<_>>(),
    );
    assert_eq!(listed.len(), 1, "a compressed node lists one child");
    ledger::take_violations()
}

/// Listing a compressed leaf that another handle shares allocates nothing
/// beyond the row it yields, though the leaf's version and payload bytes
/// have never been cloned.
///
/// The leaf sits one level below its parent's height, so its listing
/// yields it with one path byte hidden; the retained handle is the one the
/// replica keeps.
#[test]
fn listing_a_shared_fresh_compressed_leaf_allocates_nothing() {
    let _serial = serialized();
    let mut version = Version::new();
    version.tick(&crate::tree::arb::nth_party(0));
    let node = typed::Node::<S<Z>>::beneath(typed::Node::leaf(version, Message::new(7u64)), 1);
    let retained = node.clone();
    let mut path = [0; 32];
    path[31] = 1;
    let prefix = Prefix::<S<Z>>::containing(&Path::from(path));
    let violations = violations_listing(prefix, node);
    assert!(violations.is_empty(), "{violations:?}");
    drop(retained);
}

/// A providing prune allocates nothing beyond its rows at any listing,
/// including the listings its recursion makes below the node it starts at.
///
/// The root holds a compressed child, mixed because the peer's version
/// knows one of its two leaves and not the other, beside a third, unknown
/// leaf. Providing the root lists the root and then, in the recursion, the
/// compressed child, which the replica's root still shares.
#[test]
fn a_providing_prune_lists_without_allocating() {
    use crate::tree::{
        mirror::streaming::{materialized::unknown::unknown_providing, stats::Recorder},
        traverse::{Action, act},
    };
    let _serial = serialized();
    let path_from = |head: &[u8]| {
        let mut bytes = [0u8; 32];
        bytes[..head.len()].copy_from_slice(head);
        Path::from(bytes)
    };
    let versions = [0, 1, 2].map(|party| {
        let mut version = Version::new();
        version.tick(&crate::tree::arb::nth_party(party));
        version
    });
    let known = versions[0].clone();
    let actions = vec![
        (
            path_from(&[1, 2, 5, 7]),
            versions[0].clone(),
            Action::Insert(Message::new(())),
        ),
        (
            path_from(&[1, 2, 5, 8]),
            versions[1].clone(),
            Action::Insert(Message::new(())),
        ),
        (
            path_from(&[9]),
            versions[2].clone(),
            Action::Insert(Message::new(())),
        ),
    ];
    let root = act(None, actions, &mut |_, _| ()).expect("three leaves build a root");
    let replica = root.clone();
    let charged = super::Charged::new(Local);
    let view = typed::View::from(root);
    let measured = Local::measure(&view);
    let erased =
        <super::Charged<Local> as Backend>::erase(super::ChargedNode::wrap(view, measured));
    let (survivor, _) = pollster::block_on(unknown_providing(
        &charged,
        &known,
        Prefix::new().erase(),
        erased,
        &Recorder::default(),
    ))
    .expect("the in-memory backend lists exactly the children it reports");
    assert!(survivor.is_some(), "the unknown leaves survive");
    let violations = ledger::take_violations();
    assert!(violations.is_empty(), "{violations:?}");
    drop(replica);
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
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A branch whose `grandchildren` answer disagrees with the children its
/// children list is caught where the session lists its children.
#[test]
#[should_panic(expected = "mis-propagated grandchildren")]
fn miscounted_grandchildren_fails_the_listing_check() {
    let _dishonest = GRANDCHILDREN_SKEW.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A node whose `child_count` answer disagrees with the children listing
/// it yields is convicted by name where the session lists its children.
///
/// Skewed by one, every node's answer overcounts, so the root's listing
/// alone, which the suite makes before the session, yields one child
/// fewer than its parent reports.
#[test]
#[should_panic(expected = "miscounted children")]
fn miscounted_children_fail_the_listing_check() {
    let _dishonest = CHILDREN_SKEW.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A node whose `child_radices` answer disagrees with the radices listing
/// it yields is convicted by name where the session lists its children.
///
/// Dropped to nothing, every branch's answer misses every child, so the
/// root's listing alone, which the suite makes before the session, shows
/// it.
#[test]
#[should_panic(expected = "misreported child radices")]
fn misreported_radices_fail_the_listing_check() {
    let _dishonest = RADICES_DROP.set(1);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A leaf whose `version_bytes` answer exceeds the walked node's
/// aggregate is caught by the walk seam's membership check: every bound
/// under a node is in the node's aggregate.
#[test]
#[should_panic(expected = "walked leaf encodes")]
fn inflated_leaf_version_bytes_fails_the_walk_check() {
    let _dishonest = LEAF_VERSION_INFLATE.set(1024 * 1024);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A re-tag that grows the node's residency on the way back to a typed
/// handle is convicted by name: the measurement after `assume` differs
/// from the bytes the erased handle carried.
#[test]
#[should_panic(expected = "re-tagged node changed residency: erased")]
fn a_residency_changing_assume_fails_the_re_tag_check() {
    let _dishonest = ASSUME_SLACK.set(BULK_OVERHOLD);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// A re-tag that grows the node's residency on the way to an erased
/// handle is convicted by name: the measurement after `erase` differs
/// from the bytes the typed handle carried.
#[test]
#[should_panic(expected = "re-tagged node changed residency: typed")]
fn a_residency_changing_erase_fails_the_re_tag_check() {
    let _dishonest = ERASE_SLACK.set(BULK_OVERHOLD);
    pollster::block_on(check(Materializing, CENSUS_BUDGET));
}

/// Leaf construction and cloning charge both handles; dropping them restores
/// the live balance while preserving the peak until it is reset.
#[test]
fn ledger_settles_over_clone_and_drop() {
    let _serial = serialized();
    ledger::reset_peak();
    let before = ledger::peak();
    let leaf = pollster::block_on(<super::ChargedNode<typed::View<Z>> as Leaf>::leaf(
        Version::new(),
        Message::new(7),
    ))
    .expect("a local leaf constructs infallibly");
    let handle = std::mem::size_of::<typed::View<Z>>();
    let clone = leaf.clone();
    assert_eq!(ledger::peak(), before + 2 * handle);
    drop(leaf);
    drop(clone);
    assert_eq!(
        ledger::peak(),
        before + 2 * handle,
        "drops preserve the peak"
    );

    // Reset samples LIVE, so this checks settlement rather than the old peak.
    ledger::reset_peak();
    assert_eq!(ledger::peak(), before, "all node charges must be released");
}

proptest! {
    /// Dropping and unwrapping differently sized cloned nodes each releases
    /// exactly one charge, regardless of their release order.
    #[test]
    fn ledger_settles_over_mixed_node_lifetimes(
        nodes in prop::collection::vec((1usize..1024, 0usize..4, any::<bool>(), any::<u8>()), 1..16),
    ) {
        let _serial = serialized();
        ledger::reset_peak();
        let before = ledger::peak();
        let mut handles = Vec::new();
        let mut total = 0;
        for (bytes, copies, unwrap, order) in nodes {
            // The wrapper's accounting is independent of the node representation.
            let node = super::ChargedNode::wrap((), bytes);
            for _ in 0..copies {
                handles.push((order, unwrap, node.clone()));
            }
            handles.push((order, !unwrap, node));
            total += bytes * (copies + 1);
        }
        prop_assert_eq!(ledger::peak(), before + total);
        handles.sort_by_key(|(order, _, _)| *order);
        for (_, unwrap, node) in handles {
            if unwrap {
                node.into_inner();
            } else {
                drop(node);
            }
        }
        prop_assert_eq!(ledger::peak(), before + total, "releases preserve the peak");
        ledger::reset_peak();
        prop_assert_eq!(ledger::peak(), before, "every charge must settle exactly once");
    }
}
