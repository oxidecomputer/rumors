use std::convert::Infallible;
use std::mem;
use std::ops::Bound;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::task::{Context, Poll};

use async_stream::try_stream;
use futures::{Stream, StreamExt, future, stream};

use crate::{
    Version, causally,
    message::Message,
    tree::{
        self,
        backend::Store,
        mirror::streaming::{
            Backend, Leaf, Node,
            backend::{BoxNodeStream, NodeStream},
            convert::Convert,
        },
        traverse::store::walk::VersionBounds,
        typed::{
            self, Path, Prefix,
            height::{Height, S, Z},
        },
    },
};

#[cfg(test)]
mod adversarial;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub use adversarial::with_schedule;

/// The in-memory backend's [`Store::range`] walk: the synchronous owned
/// leaf walk behind an always-ready [`Stream`] face.
///
/// No box, no vtable, no executor round-trip per item — every poll answers
/// [`Poll::Ready`] straight off the resident tree.
/// This is the static-dispatch arm the [`Store::Walk`] associated type
/// exists to permit; a storage-owning backend's walk suspends on real
/// reads instead.
pub struct LocalWalk<T: Send + Sync + 'static>(
    typed::untyped::RangeOwned<T, (Bound<Version>, Bound<Version>)>,
);

impl<T: Send + Sync + 'static> Stream for LocalWalk<T> {
    type Item = Result<(tree::Key, typed::Node<T, Z>), Infallible>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(
            self.get_mut()
                .0
                .next()
                .map(|(key, leaf)| Ok((key, typed::Node::from_walk(leaf)))),
        )
    }
}

impl<T: Send + Sync + 'static, H: Height> Node<T> for typed::Node<T, H> {
    type Backend = Local;
    type Height = H;

    fn hash(&self) -> typed::Hash {
        self.hash()
    }

    // Answered by reborrowing the branch's stored bounds span (a leaf's
    // bounds coincide at its version), so the ordering the trait
    // obligates is carried by construction — no per-read validation
    // anywhere in this backend.
    fn span(&self) -> causally::Span<'_> {
        self.span()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn version_bytes(&self) -> usize {
        self.version_bytes()
    }
}

impl<T: Send + Sync + 'static> Leaf<T> for typed::Node<T, Z> {
    fn message(&self) -> &Message<T> {
        self.message()
    }

    fn version(&self) -> &Arc<Version> {
        self.version_interned()
    }

    // Custody is free: the handle owns the payload and the tree it will
    // join is resident regardless, so construction completes immediately.
    async fn leaf(version: Version, message: Message<T>) -> Result<Self, Infallible> {
        Ok(Self::leaf(version, message))
    }
}

/// The in-memory backend: [`typed::Node`] handles over the crate's own tree.
///
/// Zero-sized — the nodes carry all the state — so the cloneable-handle
/// contract of [`Backend`] is satisfied by `Copy`.
#[derive(Default, Clone, Copy, Debug)]
pub struct Local;

impl Local {
    /// One in-flight `Local` reference costs one pointer, at every fan
    /// and version bound.
    ///
    /// A node is an `Arc` handle into the session-resident tree — its
    /// children, hash memo, and version bounds live in the tree, shared,
    /// not per-session — verified against the node type below; the
    /// height-typed veneer is `repr(transparent)` over that handle, so
    /// every height costs the same.
    pub(crate) fn node_bytes(_children: usize, _version_bound: usize) -> usize {
        std::mem::size_of::<typed::Node<(), Z>>()
    }
}

/// The handle really is pointer-sized: the window's per-reference price
/// rests on it.
const _: () =
    assert!(std::mem::size_of::<typed::Node<(), Z>>() == std::mem::size_of::<*const ()>());

impl<T: Send + Sync + 'static> Backend<T> for Local {
    type Node<H: Height> = typed::Node<T, H>;
    type Error = Infallible;

    fn node_bytes(children: usize, version_bound: usize) -> usize {
        Local::node_bytes(children, version_bound)
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
        let children = stream::iter(
            parent
                .into_children()
                .into_iter()
                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
        );
        #[cfg(test)]
        return adversarial::stream(adversarial::Role::Children { height: H::HEIGHT }, children);
        #[cfg(not(test))]
        children
    }

    fn parent<H>(
        self,
        _prefix: Prefix<S<H>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> impl Future<Output = Result<Option<Self::Node<S<H>>>, Self::Error>> + Send
    where
        H: Height,
        S<H>: Height,
    {
        // A deleted child simply doesn't join the reassembly, and deleting
        // every child deletes the parent: `branch` of the empty set is `None`.
        let parent = future::ready(Ok(typed::Node::branch(
            children
                .into_iter()
                .filter_map(|(radix, child)| Some((radix, child?)))
                .collect(),
        )));
        #[cfg(test)]
        return adversarial::future(
            adversarial::Role::Parent {
                height: <S<H>>::HEIGHT,
            },
            parent,
        );
        #[cfg(not(test))]
        parent
    }

    fn leaves<H: Convert>(
        self,
        prefix: Prefix<H>,
        node: Self::Node<H>,
    ) -> impl NodeStream<Self, T, Z> {
        // The default level-by-level explosion pays an allocation per
        // *virtual* level — ruinous for path-compressed spines. In-memory
        // nodes walk their own leaves directly, skipping compressed spans.
        let leaves = stream::iter(node.leaves(&prefix).map(Ok));
        #[cfg(test)]
        return adversarial::stream(adversarial::Role::Children { height: H::HEIGHT }, leaves);
        #[cfg(not(test))]
        leaves
    }

    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, T, Z>,
    ) -> impl NodeStream<Self, T, H> + 'a {
        // The bulk counterpart of `leaves`: buffer each maximal
        // same-prefix run and build its subtree in one pass, rather than
        // folding it up one virtual level at a time. The buffered run is
        // transient state for a subtree this in-memory backend is about to
        // hold whole anyway, so the streaming session's memory story is
        // unchanged.
        let assembled = try_stream! {
            let mut leaves = pin!(leaves);
            let mut current: Option<Prefix<H>> = None;
            let mut run: Vec<(Prefix<Z>, typed::Node<T, Z>)> = Vec::new();
            while let Some(item) = leaves.next().await {
                let (prefix, leaf) = item?;
                let target = Prefix::<H>::containing(&Path::from(prefix));
                if current != Some(target)
                    && let Some(finished) = current.replace(target)
                {
                    yield (
                        finished,
                        typed::Node::from_sorted_leaves(&finished, mem::take(&mut run)),
                    );
                }
                run.push((prefix, leaf));
            }
            if let Some(finished) = current {
                yield (finished, typed::Node::from_sorted_leaves(&finished, run));
            }
        };
        #[cfg(test)]
        return adversarial::stream(adversarial::Role::Parent { height: H::HEIGHT }, assembled);
        #[cfg(not(test))]
        assembled
    }
}

impl<T: Send + Sync + 'static> Store<T> for Local {
    // Identity is the handle's own allocation: forked trees share their
    // unchanged subtrees by `Arc`.
    fn same<H: Height>(a: &typed::Node<T, H>, b: &typed::Node<T, H>) -> bool {
        a.ptr_eq(b)
    }

    // Every seam below overrides its generic default with the synchronous
    // in-memory engine wrapped in an immediately-ready future or iterator
    // stream: the tree is resident, so nothing awaits, and the generic
    // towers never monomorphize for `Local`.

    fn child<H>(
        self,
        _prefix: Prefix<S<H>>,
        parent: typed::Node<T, S<H>>,
        radix: u8,
    ) -> impl Future<Output = Result<Option<typed::Node<T, H>>, Infallible>> + Send
    where
        H: Height,
        S<H>: Height,
    {
        future::ready(Ok(parent.into_children().get(radix)))
    }

    fn act<F>(
        self,
        root: Option<typed::Node<T, typed::height::Root>>,
        actions: Vec<(Path, Version, tree::traverse::Action<T>)>,
        on_action: F,
    ) -> impl Future<Output = Result<Option<typed::Node<T, typed::height::Root>>, Infallible>> + Send
    where
        F: FnMut(&Version) + Send,
    {
        future::ready(Ok(tree::traverse::act(root, actions, on_action)))
    }

    fn join(
        self,
        a: Option<typed::Node<T, typed::height::Root>>,
        b: Option<typed::Node<T, typed::height::Root>>,
        a_version: &Version,
        b_version: &Version,
        changed: &mut bool,
    ) -> impl Future<Output = Result<Option<typed::Node<T, typed::height::Root>>, Infallible>> + Send
    {
        future::ready(Ok(tree::traverse::join(
            a, b, a_version, b_version, changed,
        )))
    }

    fn get(
        self,
        root: Option<typed::Node<T, typed::height::Root>>,
        path: Path,
    ) -> impl Future<Output = Result<Option<typed::Node<T, Z>>, Infallible>> + Send {
        let path = <[u8; 32]>::from(path);
        future::ready(Ok(root.and_then(|node| node.get_leaf(&path))))
    }

    type Walk = LocalWalk<T>;

    fn range(
        self,
        root: Option<typed::Node<T, typed::height::Root>>,
        bounds: VersionBounds,
    ) -> LocalWalk<T> {
        LocalWalk(typed::node::Root::range_owned(
            root.as_ref(),
            (bounds.start, bounds.end),
        ))
    }

    // `commit` keeps its no-op default: the resident tree has nothing to
    // flip.
}
