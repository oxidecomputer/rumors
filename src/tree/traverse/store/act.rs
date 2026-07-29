//! The backend-generic batch-apply: one pass over the tree, rebuilding the
//! touched spine through the backend.
//!
//! The generic twin of [`fn@crate::tree::traverse::act`],
//! reproducing its fold exactly: the same radix grouping and
//! delete-of-nothing short-circuit at every branch level, and the same
//! last-writer-wins resolution — running version join, causally-prior skip,
//! effectual-action observation — at the leaf. The one divergence is
//! mechanical: a surviving insert constructs its leaf **once**, after the
//! fold settles, because [`Leaf::leaf`] may persist eagerly and an occupant
//! constructed mid-fold would be instant garbage.

use futures::future::BoxFuture;
use itertools::Itertools as _;

use crate::{
    Version,
    message::Message,
    tree::{
        backend::{Leaf, Node, Store, children_of},
        traverse::act::Action,
        typed::{
            Path, Prefix,
            height::{self, Height, S, Z},
        },
    },
};

/// Apply one action batch to the (possibly absent) root, rebuilding through
/// the backend; the entry point of the tower.
///
/// `on_action` fires once per *effectual* action — a leaf inserted,
/// replaced, or removed — with that action's version, exactly as the
/// synchronous tower observes them.
pub async fn act<B, T, F>(
    backend: &B,
    root: Option<B::Node<height::Root>>,
    actions: Vec<(Path, Version, Action<T>)>,
    mut on_action: F,
) -> Result<Option<B::Node<height::Root>>, B::Error>
where
    B: Store<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
    F: FnMut(&Version) + Send,
{
    Act::act(backend, Prefix::new(), root, actions, &mut on_action).await
}

/// The inductive step of the generic batch-apply, implemented per
/// [`Height`].
pub trait Act: Height {
    fn act<'a, B, T, F>(
        backend: &'a B,
        prefix: Prefix<Self>,
        node: Option<B::Node<Self>>,
        actions: Vec<(Path<Self>, Version, Action<T>)>,
        on_action: &'a mut F,
    ) -> BoxFuture<'a, Result<Option<B::Node<Self>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
        F: FnMut(&Version) + Send;
}

impl<H: Act> Act for S<H>
where
    S<H>: Height,
{
    fn act<'a, B, T, F>(
        backend: &'a B,
        prefix: Prefix<S<H>>,
        node: Option<B::Node<S<H>>>,
        actions: Vec<(Path<Self>, Version, Action<T>)>,
        on_action: &'a mut F,
    ) -> BoxFuture<'a, Result<Option<B::Node<S<H>>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
        F: FnMut(&Version) + Send,
    {
        Box::pin(async move {
            // Group the actions by their next radix, ascending — the same
            // radix sort as the synchronous tower, collected per group so the
            // recursion's item type stays flat at every level.
            let groups: Vec<(u8, Vec<(Path<H>, Version, Action<T>)>)> = actions
                .into_iter()
                .map(|(path, version, action)| {
                    let (radix, path) = path.pop();
                    (radix, path, version, action)
                })
                .sorted_by_key(|(radix, _, _, _)| *radix)
                .chunk_by(|(radix, _, _, _)| *radix)
                .into_iter()
                .map(|(radix, group)| {
                    (
                        radix,
                        group
                            .map(|(_, path, version, action)| (path, version, action))
                            .collect(),
                    )
                })
                .collect();

            // Explode the existing node one level — the backend fetch the
            // whole level shares — or start from the empty fan.
            let existing = match node {
                Some(node) => children_of(backend, prefix, node).await?,
                None => Vec::new(),
            };

            // Merge-walk the existing fan and the action groups by radix,
            // recursing exactly where actions land: untouched children carry
            // over as `Some`, a child whose group emptied it becomes an
            // explicit `None` deletion, and the whole group reassembles
            // through the backend in one strictly-ascending pass.
            let mut group_out = Vec::with_capacity(existing.len() + groups.len());
            let mut existing = existing.into_iter().peekable();
            for (radix, group) in groups {
                while existing.peek().is_some_and(|(r, _)| *r < radix) {
                    let (r, child) = existing.next().expect("peeked entry is present");
                    group_out.push((r, Some(child)));
                }
                let child = existing.next_if(|(r, _)| *r == radix).map(|(_, c)| c);

                // Short-circuit when solely trying to delete from a
                // non-existent child.
                if child.is_none()
                    && group
                        .iter()
                        .all(|(_, _, action)| matches!(action, Action::Forget))
                {
                    continue;
                }

                let updated =
                    H::act(backend, prefix.push(radix), child, group, &mut *on_action).await?;
                group_out.push((radix, updated));
            }
            for (radix, child) in existing {
                group_out.push((radix, Some(child)));
            }

            // A group that emptied out entirely resolves to `None`,
            // cascading the deletion one level up.
            backend.clone().parent(prefix, group_out).await
        })
    }
}

/// The evolving occupant of one leaf path during the fold: the stored
/// handle it started with, or a pending fresh insert constructed once the
/// fold settles.
enum Slot<N, T> {
    Vacant,
    Stored(N),
    Fresh(Version, Message<T>),
}

impl Act for Z {
    fn act<'a, B, T, F>(
        _backend: &'a B,
        _prefix: Prefix<Z>,
        node: Option<B::Node<Z>>,
        actions: Vec<(Path<Z>, Version, Action<T>)>,
        on_action: &'a mut F,
    ) -> BoxFuture<'a, Result<Option<B::Node<Z>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
        F: FnMut(&Version) + Send,
    {
        Box::pin(async move {
            let existed_before = node.is_some();
            let mut greatest_version = Version::default();
            let mut slot = match node {
                Some(node) => Slot::Stored(node),
                None => Slot::Vacant,
            };

            // Sequentially apply the operations pertaining to this node; the
            // causally posterior operation wins, with concurrent or equal
            // actions biasing towards the last in the sequence.
            for (_, version, action) in actions {
                // Join by reference: `version` is still needed for the
                // causality comparison just below, and the join doesn't
                // consume it.
                greatest_version |= &version;

                // Skip updates that are strictly causally prior to the
                // current version at this node. A fresh occupant's ceiling is
                // the version it was staged at, exactly as a constructed
                // leaf's would be.
                let skip = match &slot {
                    Slot::Vacant => version < Version::default(),
                    Slot::Stored(node) => version < *node.span().join(),
                    Slot::Fresh(staged, _) => version < *staged,
                };
                if skip {
                    continue;
                }

                slot = match action {
                    Action::Forget => Slot::Vacant,
                    Action::Insert(message) => Slot::Fresh(greatest_version.clone(), message),
                };
            }

            // A surviving insert takes custody now, once: this is the
            // backend's one chance to persist the payload.
            let node = match slot {
                Slot::Vacant => None,
                Slot::Stored(node) => Some(node),
                Slot::Fresh(version, message) => {
                    Some(<B::Node<Z> as Leaf<T>>::leaf(version, message).await?)
                }
            };

            // Observe the action, provided that the net action wasn't nil.
            match (existed_before, &node) {
                (false, None) => {}
                _ => on_action(&greatest_version),
            }

            Ok(node)
        })
    }
}
