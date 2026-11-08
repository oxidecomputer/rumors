//! A composable test backend which injects one source failure after N
//! operations, or misreports a count of the nodes at one height.
//!
//! The misreported count is a child count or a grandchild count, as a backend
//! reading corrupted storage would give.

use std::{
    fmt,
    pin::pin,
    sync::{Arc, Mutex},
};

use async_stream::stream;
use futures::StreamExt;

use before::Span;

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::{
            Backend, ErasedNode, Leaf, Node, Summary, backend::NodeStream, byte_set::ByteSet,
        },
        typed::{
            ErasedPrefix, Hash, HashKey, Path, Prefix,
            height::{Height, Root, S, Z},
        },
    },
};

/// One backend operation observed by [`Failing`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Exploding a node at the recorded source height.
    Children { height: usize },
    /// Assembling a node at the recorded result height.
    Parent { height: usize },
}

impl fmt::Display for Operation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Children { height } => write!(formatter, "children at height {height}"),
            Self::Parent { height } => write!(formatter, "parent at height {height}"),
        }
    }
}

/// An error returned by a [`Failing`] backend.
#[derive(Debug, thiserror::Error)]
pub enum Failure<E> {
    /// The wrapper's countdown reached zero at this operation.
    #[error("injected backend failure during {0}")]
    Injected(Operation),
    /// The wrapped backend itself failed.
    #[error("inner backend failed")]
    Inner(#[source] E),
}

#[derive(Debug)]
struct State {
    remaining: Option<usize>,
    operations: Vec<Operation>,
    /// The count to report for the nodes at a height.
    miscount: Option<Miscount>,
    /// How many times a session read a misreported count.
    miscounts_read: usize,
    /// How many leaves the backend's traversals have yielded.
    leaves_yielded: usize,
}

/// A count a [`Failing`] backend misreports for the nodes at one height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Miscount {
    /// The typed height of the nodes whose count is misreported: every node the
    /// backend lists as a child at this height, and the root, wrapped by
    /// [`Failing::root`], at the root's height.
    pub height: usize,
    /// The count those nodes report.
    pub report: Misreport,
}

/// The count a [`Miscount`] replaces, and the value reported in its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Misreport {
    /// The node's child count.
    Children(usize),
    /// The node's grandchild count.
    Grandchildren(usize),
}

/// A backend handle which delegates N operations, then injects one failure.
///
/// Clones share a countdown and operation history, just as clones of a real
/// backend share storage. After injecting once, the wrapper resumes delegation;
/// this makes any erroneous work after the failure observable in [`history`].
/// Wrappers may be nested to give each layer an independent fault schedule.
#[derive(Debug)]
pub struct Failing<B> {
    inner: B,
    state: Arc<Mutex<State>>,
}

impl<B: Clone> Clone for Failing<B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

impl<B> Failing<B> {
    /// Wrap `inner`, allowing `operations` delegated calls before failing.
    pub fn after(inner: B, operations: usize) -> Self {
        Self::with(inner, Some(operations), None)
    }

    /// Wrap `inner`, never failing an operation, but reporting `miscount`'s
    /// count for every node at `miscount`'s height.
    pub fn miscounting(inner: B, miscount: Miscount) -> Self {
        Self::with(inner, None, Some(miscount))
    }

    /// Wrap `inner` with a countdown of `remaining` operations and an optional
    /// misreported count.
    fn with(inner: B, remaining: Option<usize>, miscount: Option<Miscount>) -> Self {
        Self {
            inner,
            state: Arc::new(Mutex::new(State {
                remaining,
                operations: Vec::new(),
                miscount,
                miscounts_read: 0,
                leaves_yielded: 0,
            })),
        }
    }

    /// Return how many times a session read a misreported count.
    pub fn miscounts_read(&self) -> usize {
        self.lock().miscounts_read
    }

    /// Wrap the root node a session starts from, misreporting its count if this
    /// wrapper misreports at the root's height.
    pub fn root<N>(&self, root: N) -> FailingNode<N> {
        FailingNode(root, self.miscount_at(Root::HEIGHT))
    }

    /// The count to report for a node at `height`, if this wrapper misreports
    /// there.
    fn miscount_at(&self, height: usize) -> Option<Miscounted> {
        let miscount = self.lock().miscount?;
        (miscount.height == height).then(|| Miscounted {
            report: miscount.report,
            state: Arc::clone(&self.state),
        })
    }

    /// Lock the shared state.
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .expect("the failing-backend state mutex is not poisoned")
    }

    /// Return how many leaves this wrapper's traversals have yielded.
    ///
    /// That is the leaves a consumer of [`Backend::leaves`] has read so far,
    /// since the default enumeration explodes each node through `children` and
    /// yields each leaf as its parent's traversal does.
    pub fn leaves_yielded(&self) -> usize {
        self.lock().leaves_yielded
    }

    /// Return every operation attempted through this wrapper, in call order.
    pub fn history(&self) -> Vec<Operation> {
        self.lock().operations.clone()
    }

    fn checkpoint<E>(&self, operation: Operation) -> Result<(), Failure<E>> {
        let mut state = self.lock();
        state.operations.push(operation);
        match state.remaining {
            Some(0) => {
                state.remaining = None;
                Err(Failure::Injected(operation))
            }
            Some(remaining) => {
                state.remaining = Some(remaining - 1);
                Ok(())
            }
            None => Ok(()),
        }
    }
}

/// A node handle translated through a [`Failing`] backend layer.
#[derive(Clone, Debug)]
pub struct FailingNode<N>(N, Option<Miscounted>);

/// The count a [`FailingNode`] misreports, and the backend state that counts
/// each read of it.
#[derive(Clone, Debug)]
struct Miscounted {
    /// The count replaced and the value reported.
    report: Misreport,
    /// The wrapping backend's state.
    state: Arc<Mutex<State>>,
}

impl Miscounted {
    /// The misreported grandchild count, if this misreports it, counting the
    /// read.
    fn grandchild_count(&self) -> Option<usize> {
        match self.report {
            Misreport::Grandchildren(count) => Some(self.read(count)),
            Misreport::Children(_) => None,
        }
    }

    /// The misreported child count, if this misreports it, counting the read.
    fn children(&self) -> Option<usize> {
        match self.report {
            Misreport::Children(count) => Some(self.read(count)),
            Misreport::Grandchildren(_) => None,
        }
    }

    /// Count one read of `value`, the misreported count, and return it.
    fn read(&self, value: usize) -> usize {
        self.state
            .lock()
            .expect("the failing-backend state mutex is not poisoned")
            .miscounts_read += 1;
        value
    }
}

impl<N> FailingNode<N> {
    /// Wrap an inner backend's node without changing its contents.
    pub fn new(inner: N) -> Self {
        Self(inner, None)
    }

    /// Remove this test backend's node wrapper.
    pub fn into_inner(self) -> N {
        self.0
    }
}

/// Preserve typed-node observations while testing failures in backend
/// operations.
impl<N> Node for FailingNode<N>
where
    N: Node + Clone + Send + 'static,
{
    type Backend = Failing<N::Backend>;
    type Height = N::Height;

    /// Borrow the wrapped node's version bounds.
    fn span(&self) -> Span<'_> {
        self.0.span()
    }

    /// Read the wrapped node's leaf count.
    fn len(&self) -> usize {
        self.0.len()
    }

    /// Read the wrapped node's grandchild count, or the misreported one.
    fn grandchild_count(&self) -> usize {
        self.1
            .as_ref()
            .and_then(Miscounted::grandchild_count)
            .unwrap_or_else(|| self.0.grandchild_count())
    }

    /// Read the wrapped node's largest encoded version bound.
    fn version_bytes(&self) -> usize {
        self.0.version_bytes()
    }
}

impl<E: ErasedNode> ErasedNode for FailingNode<E> {
    fn span(&self) -> Span<'_> {
        self.0.span()
    }

    fn hash(&self, at: &ErasedPrefix, key: &HashKey) -> Hash {
        self.0.hash(at, key)
    }

    /// Read the wrapped node's summary, with the misreported count, if any, in
    /// place of the wrapped one, as the count queries report it.
    fn summary(&self, at: &ErasedPrefix, key: &HashKey) -> Summary {
        let mut summary = self.0.summary(at, key);
        if let Some(miscounted) = &self.1 {
            if let Some(children) = miscounted.children() {
                summary.child_count = children;
            }
            if let Some(grandchildren) = miscounted.grandchild_count() {
                summary.grandchildren = grandchildren;
            }
        }
        summary
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    /// Read the wrapped node's grandchild count, or the misreported one.
    fn grandchild_count(&self, at: &ErasedPrefix) -> usize {
        self.1
            .as_ref()
            .and_then(Miscounted::grandchild_count)
            .unwrap_or_else(|| self.0.grandchild_count(at))
    }

    /// Read the wrapped node's child radices.
    fn child_radices(&self, at: &ErasedPrefix) -> ByteSet {
        self.0.child_radices(at)
    }

    /// Read the wrapped node's child count, or the misreported one.
    fn child_count(&self, at: &ErasedPrefix) -> usize {
        self.1
            .as_ref()
            .and_then(Miscounted::children)
            .unwrap_or_else(|| self.0.child_count(at))
    }
}

impl<N> Leaf for FailingNode<N>
where
    N: Leaf + Clone + Send + 'static,
{
    fn message(&self) -> &Message {
        self.0.message()
    }

    // Leaf construction has no `Failing` backend instance from which to
    // consume a failure point or record history. Delegate it and preserve
    // inner errors; this instrument injects only into traversal calls, which
    // do receive the backend instance.
    async fn leaf(
        version: Version,
        message: Message,
    ) -> Result<Self, Failure<<N::Backend as Backend>::Error>> {
        N::leaf(version, message)
            .await
            .map(Self::new)
            .map_err(Failure::Inner)
    }
}

/// Inject failures into the level-by-level backend operations.
///
/// This implementation intentionally retains the default `leaves` and
/// `assemble` methods. They traverse every level through `children` and
/// `parent`; forwarding the wrapped backend's bulk overrides would bypass those
/// failure checkpoints.
impl<B> Backend for Failing<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    type Node<H: Height> = FailingNode<B::Node<H>>;
    // Erasure passes through the wrapper: fault injection targets the
    // traversal operations, and re-tagging is not one.
    type Erased = FailingNode<B::Erased>;
    type Error = Failure<B::Error>;

    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
        FailingNode(B::erase(node.0), node.1)
    }

    fn assume<H: Height>(at: &Prefix<H>, erased: Self::Erased) -> Self::Node<H> {
        FailingNode(B::assume(at, erased.0), erased.1)
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
        self.checkpoint(Operation::Parent {
            height: S::<H>::HEIGHT,
        })?;
        let children = children
            .into_iter()
            .map(|(radix, child)| (radix, child.map(FailingNode::into_inner)))
            .collect();
        self.inner
            .parent(prefix, original.map(FailingNode::into_inner), children)
            .await
            .map(|node| node.map(FailingNode::new))
            .map_err(Failure::Inner)
    }

    fn children<H>(self, prefix: Prefix<S<H>>, parent: Self::Node<S<H>>) -> impl NodeStream<Self, H>
    where
        H: Height,
        S<H>: Height,
    {
        stream! {
            if let Err(error) = self.checkpoint(Operation::Children {
                height: S::<H>::HEIGHT,
            }) {
                yield Err(error);
                return;
            }
            let miscounted = self.miscount_at(H::HEIGHT);
            let state = Arc::clone(&self.state);
            let mut children = pin!(self.inner.children(prefix, parent.into_inner()));
            while let Some(child) = children.next().await {
                if H::HEIGHT == Z::HEIGHT && child.is_ok() {
                    state
                        .lock()
                        .expect("the failing-backend state mutex is not poisoned")
                        .leaves_yielded += 1;
                }
                yield child
                    .map(|(prefix, node)| (prefix, FailingNode(node, miscounted.clone())))
                    .map_err(Failure::Inner);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::mirror::streaming::Local;

    /// Nested wrappers retain independent countdowns and distinguish their
    /// errors.
    #[test]
    fn failing_backends_compose() {
        let inner = Failing::after(Local, 0);
        let outer = Failing::after(inner.clone(), 1);
        let prefix = Prefix::<S<Z>>::containing(&Path::from([0; 32]));

        let first = pollster::block_on(Backend::parent::<Z>(
            outer.clone(),
            prefix,
            None,
            Vec::new(),
        ));
        assert!(matches!(
            first,
            Err(Failure::Inner(Failure::Injected(Operation::Parent {
                height: 1
            })))
        ));

        let second = pollster::block_on(Backend::parent::<Z>(
            outer.clone(),
            prefix,
            None,
            Vec::new(),
        ));
        assert!(matches!(
            second,
            Err(Failure::Injected(Operation::Parent { height: 1 }))
        ));
        assert_eq!(
            outer.history(),
            vec![
                Operation::Parent { height: 1 },
                Operation::Parent { height: 1 },
            ]
        );
        assert_eq!(inner.history(), vec![Operation::Parent { height: 1 }]);
    }
}
