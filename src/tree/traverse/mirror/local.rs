//! Two replicas reconcile their trees while honoring deletions: leaves one side
//! has and the other has since *forgotten* (their version is `<=` the other's
//! version vector) vanish; leaves not yet seen are transmitted. The protocol
//! recurses down the *disjoint frontier* of the two trees, alternating sender
//! each message, so it costs `O(log n)` round-trips and never re-sends a hash
//! the other side can already infer.
//!
//! # State machine
//!
//! Each side keeps a [`Levels`](crate::tree::typed::Levels) zipper: a stack of
//! level maps from `Root` down to the height currently under comparison. In one
//! round, the sender examines its zipper's bottom level, sends a message, and
//! pushes two new (mostly empty) levels onto the bottom; the receiver's next
//! round operates on its own zipper, offset by one height. Heights on which the
//! parties have agreed end up nearer the top of the zipper; heights still in
//! dispute live at the bottom. After max 16 rounds (likely fewer), both have
//! reached `Z` (leaf height) and the zippers collapse back to roots.
//!
//! The wire conversation:
//!
//!   1. Initiator sends [`message::Initiate`] (a single hash at the empty
//!      prefix: our root hash).
//!   2. Responder replies with [`message::Opening`], enumerating the hashes
//!      of every child of its root (empty if it has none).
//!   3. Both sides alternately send [`message::Exchange`]s, each round
//!      descending the sender's zipper by two heights.
//!   4. The initiator's last outgoing message is [`message::Closing`] in
//!      lieu of an `Exchange` at leaf height (whose `uncertain` would be
//!      vacuous); the responder replies with [`message::Complete`] carrying
//!      only the final `providing`; the initiator absorbs that and is done.
//!
//! # Three channels
//!
//! The wire format has three independent flows of information. Each message
//! type carries the subset of fields that are non-vacuous for its role:
//! `Initiate` and `Opening` carry only `uncertain`; `Complete` only
//! `providing`; `Closing` carries `providing` and `requested`; the
//! steady-state `Exchange` carries all three.
//!
//! | Field       | Sender's claim                                 | Receiver's action            |
//! |-------------|------------------------------------------------|------------------------------|
//! | `uncertain` | "I have these hashes at this height"           | compare against my own       |
//! | `requested` | "your last `uncertain` listed hashes I lack"   | answer via `providing` next  |
//! | `providing` | "you asked for these, or I know you lack them" | insert into my zipper        |
//!
//! # Asymmetry matrix
//!
//! For every prefix at the current comparison height there are four cases,
//! depending on whether each side has the node. The protocol is correct iff
//! every case routes its information to some channel:
//!
//! |                | counterparty has it                                 | counterparty lacks it                         |
//! |----------------|-----------------------------------------------------|-----------------------------------------------|
//! | **we have it** | hashes match: no action; hashes differ: recur below | we `provide`                                  |
//! | **we lack it** | we `request`                                        | (impossible: neither side would mention it)   |
//!
//! Each cell is realized by one arm of the `merge_join_by` inside
//! [`Exchange::partition_uncertain`].

use std::convert::Infallible;

use crate::{
    Version,
    tree::{
        self,
        traverse::unknown::Unknown,
        typed::{
            Hash, Levels, Node, Prefix,
            height::{Height, Root, S, Z},
            levels::{Below, Top},
        },
    },
};

use super::message::{self, UnderRoot, UnderUnderRoot};
use super::protocol;

mod partition;

/// The version state for an [`Exchange`] which has just been initialized but
/// has not yet connected. Carries the fields the [`Connect`](protocol::Connect)
/// / [`Accept`](protocol::Accept) step needs to build its outgoing
/// [`message::Handshake`]: our universe [`Network`], our latest [`Version`], and
/// our [`Intent`](message::Intent) ([`Retire`](message::Intent::Retire) iff we
/// will hand the peer our party in a trailing frame once reconciliation
/// completes).
pub struct Start {
    our_version: Version,
}

/// The version state for an [`Exchange`] which has sent its version to its peer
/// but has not yet received its peer's version.
pub struct Connecting {
    our_version: Version,
}

/// The version state for an [`Exchange`] which has sent and received versions
/// with its peer, and so can proceed to the rest of the protocol.
pub struct Connected {
    our_version: Version,
    their_version: Version,
}

/// The parent prefixes a counterparty may legitimately `provide` against, given
/// the `requested` and `uncertain` we are about to send: each `requested`
/// prefix is the parent of the subtree-children it will answer with, and each
/// `uncertain` prefix's parent is the parent of the Left-case siblings it may
/// unilaterally provide. Returned as raw bytes so the membership test in
/// [`Exchange::absorb_providing`] is height-agnostic. Debug-only: it backs a
/// `debug_assert!`.
#[cfg(debug_assertions)]
fn expected_providing_parents<A, B>(
    requested: &[Prefix<A>],
    uncertain: &[(Prefix<B>, Hash)],
) -> std::collections::BTreeSet<Box<[u8]>>
where
    A: Height,
    B: Height,
{
    let mut parents = std::collections::BTreeSet::default();
    // The root is always implicitly compared, so the counterparty may always
    // provide root's children (the first round's asymmetric-root drain, where
    // the initiator hands over root children an empty/divergent responder never
    // listed). Its parent is the empty prefix.
    parents.insert(Box::from(&[][..]));
    for prefix in requested {
        parents.insert(Box::from(prefix.as_bytes()));
    }
    for (prefix, _) in uncertain {
        let bytes = prefix.as_bytes();
        parents.insert(Box::from(&bytes[..bytes.len().saturating_sub(1)]));
    }
    parents
}

/// An in-progress mirror synchronization on one side of the wire.
///
/// `L` is our zipper, parameterised by the height of its bottom level; as the
/// protocol descends, each [`Self::exchange`] call returns a new `Exchange`
/// whose `L` is two heights below the previous one.
pub struct Exchange<V, L> {
    /// Our multi-level zipper: agreed heights live near the top, the height
    /// currently under comparison lives at the bottom.
    levels: L,
    /// The counterparty's version vector, used to honor their deletions: any
    /// node of ours at or causally prior to this version that they lack must
    /// have been forgotten on their side.
    versions: V,
    /// The parent prefixes (raw bytes, height-agnostic) the counterparty is
    /// allowed to `provide` next: those we `requested` last round, plus the
    /// parents of those we listed as `uncertain` (whose siblings the
    /// counterparty may unilaterally provide as the Left case). Used by
    /// [`absorb_providing`](Self::absorb_providing) to reject a peer that
    /// provides subtrees we had no basis to receive.
    ///
    /// Tracked only in debug builds, since it backs a `debug_assert!`: release
    /// builds carry no field and pay nothing to maintain it.
    #[cfg(debug_assertions)]
    expected_parents: std::collections::BTreeSet<Box<[u8]>>,
}

impl<T> Exchange<Start, Top<T>>
where
    T: Send + Sync,
{
    pub fn start(node: tree::Root<T>) -> Self {
        Self {
            versions: Start {
                our_version: node.ceiling.clone(),
            },
            levels: Node::levels(Option::from(node)),
            #[cfg(debug_assertions)]
            expected_parents: Default::default(),
        }
    }
}

// A local `Exchange`'s participation in the protocol:

impl<V, L> protocol::Stage for Exchange<V, L>
where
    L: Levels,
{
    type Height = L::Height;
    type Output = tree::Root<L::Message>;
    type Error = Infallible;
}

impl<T> protocol::Connect<T> for Exchange<Start, Top<T>>
where
    T: Send + Sync,
{
    type Next = Exchange<Connecting, Top<T>>;

    async fn connect(
        self,
    ) -> Result<protocol::Step<message::Handshake, Self::Next, Infallible>, Self::Error> {
        let Start { our_version } = self.versions;

        let next = Exchange {
            levels: self.levels,
            versions: Connecting {
                our_version: our_version.clone(),
            },
            #[cfg(debug_assertions)]
            expected_parents: self.expected_parents,
        };

        Ok(protocol::Step::Continue {
            msg: message::Handshake {
                version: our_version,
            },
            next,
        })
    }
}

impl<T> protocol::CompleteConnect<T> for Exchange<Connecting, Top<T>>
where
    T: Send + Sync,
{
    type Next = Exchange<Connected, Top<T>>;

    async fn complete_connect(
        self,
        their_version: Version,
    ) -> Result<protocol::Step<(), Self::Next, Self::Output>, Self::Error> {
        let our_version = self.versions.our_version;

        // If the two versions are the same, both sides are immediately done
        if our_version == their_version {
            return Ok(protocol::Step::Done {
                msg: (),
                output: tree::Root {
                    ceiling: our_version,
                    root: self.levels.collapse(),
                },
            });
        }

        let next = Exchange {
            levels: self.levels,
            versions: Connected {
                our_version,
                their_version,
            },
            #[cfg(debug_assertions)]
            expected_parents: self.expected_parents,
        };

        Ok(protocol::Step::Continue { msg: (), next })
    }
}

impl<T> protocol::Accept<T> for Exchange<Start, Top<T>>
where
    T: Send + Sync,
{
    type Next = Exchange<Connected, Top<T>>;

    async fn accept(
        self,
        request: message::Handshake,
    ) -> Result<protocol::Step<message::Handshake, Self::Next, Self::Output>, Self::Error> {
        let Start { our_version } = self.versions;
        let their_version = request.version;

        // If the two versions are the same, both sides are immediately done
        if our_version == their_version {
            return Ok(protocol::Step::Done {
                msg: message::Handshake {
                    version: our_version.clone(),
                },
                output: tree::Root {
                    ceiling: our_version,
                    root: self.levels.collapse(),
                },
            });
        }

        let next = Exchange {
            levels: self.levels,
            versions: Connected {
                our_version: our_version.clone(),
                their_version,
            },
            #[cfg(debug_assertions)]
            expected_parents: self.expected_parents,
        };

        Ok(protocol::Step::Continue {
            msg: message::Handshake {
                version: our_version,
            },
            next,
        })
    }
}

impl<T> protocol::Initiator<T> for Exchange<Connected, Top<T>>
where
    T: Send + Sync,
{
    type Next = Exchange<Connected, Top<T>>;

    async fn initiator(
        self,
    ) -> Result<protocol::Step<message::Initiate, Self::Next, Infallible>, Infallible> {
        let msg = message::Initiate {
            uncertain: self
                .levels
                .level()
                .iter()
                .map(|(prefix, node)| (*prefix, node.hash()))
                .collect(),
        };

        Ok(protocol::Step::Continue { msg, next: self })
    }
}

impl<T> protocol::Responder<T> for Exchange<Connected, Top<T>>
where
    T: Send + Sync,
{
    type Next = Exchange<Connected, Below<UnderRoot, Top<T>>>;

    async fn responder(
        mut self,
        _request: message::Initiate,
    ) -> Result<protocol::Step<message::Opening, Self::Next, Self::Output>, Infallible> {
        // Always explode our root one level down and enumerate the resulting
        // children, regardless of the initiator's root hash. We deliberately do
        // *not* short-circuit on matched roots: an empty `Opening` is the
        // unambiguous "responder has no children" signal that drives the
        // initiator's [`Self::open_initiator`] "we have, they lack" Left case
        // when the responder is empty. Pushing the matched case through the
        // steady-state pipeline costs one round's worth of child hashes
        // (~16 entries) but keeps a single termination path on the wire.
        let levels = Node::levels(None).down(
            self.levels
                .level_mut()
                .remove(&Prefix::new())
                .map(|n| {
                    n.into_children()
                        .into_iter()
                        .map(|(radix, child)| (Prefix::new().push(radix), child))
                        .collect()
                })
                .unwrap_or_default(),
        );

        let msg = message::Opening {
            uncertain: levels
                .level()
                .iter()
                .map(|(prefix, child)| (*prefix, child.hash()))
                .collect(),
        };

        let next = Exchange {
            levels,
            versions: self.versions,
            // The `Opening` carries only `uncertain`; the initiator may answer
            // it with Left-case `providing` whose parents are the parents of
            // these prefixes (it carries no `requested` to honor).
            #[cfg(debug_assertions)]
            expected_parents: expected_providing_parents::<UnderRoot, _>(&[], &msg.uncertain),
        };

        Ok(protocol::Step::Continue { msg, next })
    }
}

impl<T, L> protocol::OpenInitiator<T> for Exchange<Connected, L>
where
    T: Send + Sync,
    L: Levels<Message = T, Height = Root>,
{
    type Next = Exchange<Connected, Below<UnderUnderRoot, Below<UnderRoot, L>>>;

    async fn open_initiator(
        self,
        request: message::Opening,
    ) -> Result<
        protocol::Step<message::Exchange<T, UnderUnderRoot>, Self::Next, Self::Output>,
        Infallible,
    > {
        Ok(self.reply(request))
    }
}

impl<T, H, L> protocol::Exchange<T> for Exchange<Connected, L>
where
    T: Send + Sync,
    L: Levels<Message = T, Height = S<S<H>>>,
    S<S<H>>: Height,
    S<H>: Height,
    H: Height + Unknown,
    // Assumed at impl-validation time so we don't have to case-analyze `H`
    // here: at use sites `H` is concrete and one of the three blanket impls
    // discharges it.
    Exchange<Connected, Below<H, Below<S<H>, L>>>: protocol::AfterExchange<T, H>,
{
    type Next = Exchange<Connected, Below<H, Below<S<H>, L>>>;

    async fn exchange(
        self,
        request: message::Exchange<T, S<H>>,
    ) -> Result<protocol::Step<message::Exchange<T, H>, Self::Next, Self::Output>, Infallible> {
        Ok(self.reply(request))
    }
}

impl<T, L> protocol::CloseInitiator<T> for Exchange<Connected, L>
where
    T: Send + Sync,
    L: Levels<Message = T, Height = S<S<Z>>>,
{
    type Next = Exchange<Connected, Below<Z, Below<S<Z>, L>>>;

    async fn close_initiator(
        self,
        request: message::Exchange<T, S<Z>>,
    ) -> Result<protocol::Step<message::Closing<T>, Self::Next, Self::Output>, Infallible> {
        Ok(self.reply(request))
    }
}

impl<T, L> protocol::CompleteResponder<T> for Exchange<Connected, L>
where
    T: Send + Sync,
    L: Levels<Message = T, Height = S<Z>>,
{
    async fn complete_responder(
        mut self,
        request: message::Closing<T>,
    ) -> Result<protocol::Step<message::Complete<T>, Infallible, Self::Output>, Infallible> {
        self.absorb_providing(request.providing);
        let providing = self.answer_requested(request.requested);
        Ok(protocol::Step::Done {
            msg: message::Complete {
                providing: providing.into_iter().collect(),
            },
            output: tree::Root {
                ceiling: self.versions.our_version | self.versions.their_version,
                root: self.levels.collapse(),
            },
        })
    }
}

impl<T, L> protocol::CompleteInitiator<T> for Exchange<Connected, L>
where
    T: Send + Sync,
    L: Levels<Message = T, Height = Z>,
{
    async fn complete_initiator(
        mut self,
        request: message::Complete<T>,
    ) -> Result<protocol::Step<(), Infallible, Self::Output>, Infallible> {
        self.absorb_providing(request.providing);
        Ok(protocol::Step::Done {
            msg: (),
            output: tree::Root {
                ceiling: self.versions.our_version | self.versions.their_version,
                root: self.levels.collapse(),
            },
        })
    }
}
