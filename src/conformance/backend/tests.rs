//! The backend conformance suite, run against this crate's backends and
//! against reference backends built to prove the suite has teeth.

use std::convert::Infallible;
use std::pin::pin;

use async_stream::stream;
use futures::StreamExt;

use super::{Charged, Measure, check};
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
    pollster::block_on(check(Local, 64 * 1024));
}

/// A materializing reference backend, shaped like a database row store.
///
/// Each node value owns a buffer sized like the row it would keep
/// resident (header, child table, version bounds), on top of a `Local`
/// handle standing in for the store.
///
/// The priced header is the honesty knob: the real row header is
/// `ROW_HEADER` bytes, and the cost function prices `priced_header()`,
/// so a priced header at or above the real one is honest and anything
/// less underprices — which `underpricing_fails_the_pointwise_check`
/// relies on. The knob is a process-global rather than a const
/// parameter deliberately: every distinct backend type instantiates the
/// whole height-indexed protocol tower (see
/// `design/height-erasure.md`), so the honest and lying variants must
/// share one type.
#[derive(Clone, Copy, Debug)]
struct Materializing;

/// The header bytes [`Materializing::node_bytes`] prices; see the
/// struct docs for why this is state rather than a type parameter.
static PRICED_HEADER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The header bytes the cost function prices in this process.
fn priced_header() -> usize {
    PRICED_HEADER.load(std::sync::atomic::Ordering::Relaxed)
}

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

    fn leaf(version: Version, message: Message<T>) -> Self {
        // Leaf rows carry the payload, which the account leaves to the
        // wire's message-size target; no interior row is simulated.
        Self::wrap(typed::Node::leaf(version, message), 0)
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
            + priced_header()
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
#[test]
fn materializing_backend_conforms() {
    PRICED_HEADER.store(ROW_HEADER, std::sync::atomic::Ordering::Relaxed);
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
    PRICED_HEADER.store(0, std::sync::atomic::Ordering::Relaxed);
    pollster::block_on(check(Materializing, 4 * 1024 * 1024));
}

/// The decorator's ledger accounting is exact over wrap, clone, and drop.
///
/// A wrapped leaf charges nothing (out of scope), a cloned handle charges
/// its bytes again, and drops settle to the starting balance — the
/// arithmetic the end-to-end census rests on.
#[test]
fn ledger_settles_over_clone_and_drop() {
    use super::ledger;
    let before = {
        ledger::reset_peak();
        ledger::peak()
    };
    let leaf = <super::ChargedNode<typed::Node<u64, Z>> as Leaf<u64>>::leaf(
        Version::new(),
        Message::new(7),
    );
    let clone = leaf.clone();
    drop(leaf);
    drop(clone);
    assert_eq!(
        ledger::peak(),
        before,
        "zero-charged handles must not move the ledger",
    );

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
