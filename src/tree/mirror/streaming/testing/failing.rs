//! A composable test backend which injects one source failure after N operations.

use std::{
    fmt,
    pin::pin,
    sync::{Arc, Mutex},
};

use async_stream::stream;
use futures::StreamExt;

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::{Backend, Leaf, Node, backend::NodeStream},
        typed::{
            Hash, Path, Prefix,
            height::{Height, S, Z},
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
        Self {
            inner,
            state: Arc::new(Mutex::new(State {
                remaining: Some(operations),
                operations: Vec::new(),
            })),
        }
    }

    /// Return every operation attempted through this wrapper, in call order.
    pub fn history(&self) -> Vec<Operation> {
        self.state
            .lock()
            .expect("the failing-backend state mutex is not poisoned")
            .operations
            .clone()
    }

    fn checkpoint<E>(&self, operation: Operation) -> Result<(), Failure<E>> {
        let mut state = self
            .state
            .lock()
            .expect("the failing-backend state mutex is not poisoned");
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
pub struct FailingNode<N>(N);

impl<N> FailingNode<N> {
    /// Wrap an inner backend's node without changing its contents.
    pub fn new(inner: N) -> Self {
        Self(inner)
    }

    /// Remove this test backend's node wrapper.
    pub fn into_inner(self) -> N {
        self.0
    }
}

impl<T, N> Node<T> for FailingNode<N>
where
    T: Send + Sync + 'static,
    N: Node<T> + Clone + Send + 'static,
{
    type Backend = Failing<N::Backend>;
    type Height = N::Height;

    fn ceiling(&self) -> &Version {
        self.0.ceiling()
    }

    fn floor(&self) -> &Version {
        self.0.floor()
    }

    fn hash(&self) -> Hash {
        self.0.hash()
    }
}

impl<T, N> Leaf<T> for FailingNode<N>
where
    T: Send + Sync + 'static,
    N: Leaf<T> + Clone + Send + 'static,
{
    fn message(&self) -> &Message<T> {
        self.0.message()
    }

    fn leaf(version: Version, message: Message<T>) -> Self {
        Self(N::leaf(version, message))
    }
}

impl<B, T> Backend<T> for Failing<B>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Node<H: Height> = FailingNode<B::Node<H>>;
    type Error = Failure<B::Error>;

    async fn parent<H>(
        self,
        prefix: Prefix<S<H>>,
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
            .parent(prefix, children)
            .await
            .map(|node| node.map(FailingNode::new))
            .map_err(Failure::Inner)
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
            if let Err(error) = self.checkpoint(Operation::Children {
                height: S::<H>::HEIGHT,
            }) {
                yield Err(error);
                return;
            }
            let mut children = pin!(self.inner.children(prefix, parent.into_inner()));
            while let Some(child) = children.next().await {
                yield child
                    .map(|(prefix, node)| (prefix, FailingNode::new(node)))
                    .map_err(Failure::Inner);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::mirror::streaming::Local;

    /// Nested wrappers retain independent countdowns and distinguish their errors.
    #[test]
    fn failing_backends_compose() {
        let inner = Failing::after(Local, 0);
        let outer = Failing::after(inner.clone(), 1);
        let prefix = Prefix::<S<Z>>::containing(&Path::from([0; 32]));

        let first = pollster::block_on(Backend::<()>::parent::<Z>(
            outer.clone(),
            prefix,
            Vec::new(),
        ));
        assert!(matches!(
            first,
            Err(Failure::Inner(Failure::Injected(Operation::Parent {
                height: 1
            })))
        ));

        let second = pollster::block_on(Backend::<()>::parent::<Z>(
            outer.clone(),
            prefix,
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
