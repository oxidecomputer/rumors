//! The backend conformance suite, run against this crate's backends and
//! against reference backends built to prove the suite has teeth.

use std::convert::Infallible;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_stream::stream;
use futures::StreamExt;

use super::{Charged, Measure, check, ledger};
use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::{Backend, Leaf, Local, Node, NodeStream},
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

impl<T: Send + Sync + 'static> Measure<T> for Local {
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

/// The bytes a materialized row spends beyond its child table and bounds.
const ROW_HEADER: usize = 64;

/// The bytes one child entry occupies in a materialized row.
const ROW_ENTRY: usize = 24;

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

impl<T, H> Node<T> for MaterializedNode<typed::Node<T, H>>
where
    T: Send + Sync + 'static,
    H: Height,
{
    type Backend = Materializing;
    type Height = H;

    fn ceiling(&self) -> &Version {
        self.inner.ceiling()
    }

    fn floor(&self) -> &Version {
        self.inner.floor()
    }

    fn hash(&self) -> Hash {
        self.inner.hash()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn version_bytes(&self) -> usize {
        self.inner.version_bytes()
    }
}

impl<T> Leaf<T> for MaterializedNode<typed::Node<T, Z>>
where
    T: Send + Sync + 'static,
{
    fn message(&self) -> &Message<T> {
        self.inner.message()
    }

    async fn leaf(version: Version, message: Message<T>) -> Result<Self, Infallible> {
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
fn bounds_of<T: Send + Sync + 'static, H: Height>(node: &typed::Node<T, H>) -> usize {
    node.ceiling().as_bytes().len() + node.floor().as_bytes().len()
}

impl<T> Backend<T> for Materializing
where
    T: Send + Sync + 'static,
{
    type Node<H: Height> = MaterializedNode<typed::Node<T, H>>;
    type Error = Infallible;

    fn node_bytes(children: usize, version_bound: usize) -> usize {
        std::mem::size_of::<MaterializedNode<typed::Node<T, Z>>>()
            + PRICED_HEADER.get()
            + ROW_ENTRY * children
            + version_bound
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
        let parent = Local.parent(prefix, children).await?;
        Ok(parent.map(|node| {
            let row = ROW_HEADER + ROW_ENTRY * fan + bounds_of(&node);
            MaterializedNode::wrap(node, row)
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
            let mut children = pin!(Local.children(prefix, parent.inner));
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
}

impl<T> Measure<T> for Materializing
where
    T: Send + Sync + 'static,
{
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
    pollster::block_on(check(Materializing, 4 * 1024 * 1024));
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
    pollster::block_on(check(Materializing, 4 * 1024 * 1024));
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
        <super::ChargedNode<MaterializedNode<typed::Node<u64, Z>>> as Leaf<u64>>::leaf(
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
    let leaf = pollster::block_on(
        <super::ChargedNode<typed::Node<u64, Z>> as Leaf<u64>>::leaf(
            Version::new(),
            Message::new(7),
        ),
    )
    .expect("a local leaf constructs infallibly");
    let handle = std::mem::size_of::<typed::Node<u64, Z>>();
    let clone = leaf.clone();
    assert_eq!(
        ledger::peak(),
        before + 2 * handle,
        "two live leaf handles charge their measured bytes twice",
    );
    drop(leaf);
    drop(clone);

    let node = typed::Node::leaf(Version::new(), Message::new(7));
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
