//! The distinguished opening question, whose content rides the greeting.
//!
//! No wire frame exists for the opening: [`opening_scope`] validates the
//! locally produced opening reply and derives the root scope the initiator
//! decodes the top-level reply against, while [`opening_reply`] replays the
//! peer greeting's listing as the message the responder answers. The two
//! must agree on the scope, since one side builds it from its own message
//! and the other from the listing that crossed the wire.

use before::Version;

use crate::message::Message;
use crate::tree::{
    mirror::streaming::{
        Local,
        message::{Reaction, Reply},
    },
    typed::{
        self,
        height::{Height, S, UnderRoot, Z},
    },
};

use super::{
    super::{OpeningError, Scope, opening_reply, opening_scope},
    hash,
};

trait OpeningNode: Height {
    fn node() -> typed::Node<(), Self>;
}

impl OpeningNode for Z {
    fn node() -> typed::Node<(), Self> {
        typed::Node::leaf(Version::new(), Message::new(()))
    }
}

impl<H: OpeningNode> OpeningNode for S<H>
where
    S<H>: Height,
{
    fn node() -> typed::Node<(), Self> {
        typed::Node::beneath(H::node(), 0)
    }
}

/// The local opening's derived scope agrees with the scope the remote side
/// replays from the same listing carried in the greeting.
#[test]
fn opening_scope_agrees_with_greeting_replay() {
    let listing = vec![(3, hash(1)), (9, hash(2))];
    let reply = Reply::<Local, (), UnderRoot> {
        replies: vec![Reaction::Query(listing.clone())],
    };

    let scope = opening_scope(reply).expect("canonical opening");
    assert_eq!(scope, Scope::opening(&listing));

    let (replayed, replayed_scope) = opening_reply::<Local, ()>(listing.clone());
    assert_eq!(replayed_scope, scope);
    let [Reaction::Query(replayed)] = replayed.replies.as_slice() else {
        panic!("the replayed opening must remain one query")
    };
    assert_eq!(replayed, &listing);
}

/// An empty carried listing replays the empty-tree initiator's opening: one
/// empty `Query`, meaning "I lack the root, send everything", with a root
/// scope holding no positional children.
#[test]
fn empty_listing_replays_the_empty_opening() {
    let (replayed, mut scope) = opening_reply::<Local, ()>(Vec::new());
    let [Reaction::Query(listing)] = replayed.replies.as_slice() else {
        panic!("the replayed opening must be one query")
    };
    assert!(listing.is_empty());
    assert_eq!(scope, Scope::opening(&[]));
    assert_eq!(scope.next(), None, "an empty root scope positions nothing");
}

/// Every semantic opening shape is either the one valid query or its exact
/// typed rejection.
#[test]
fn opening_rejections_are_exhaustive() {
    for count in 0..=3 {
        let reply = Reply::<Local, (), UnderRoot> {
            replies: (0..count).map(|_| Reaction::Match).collect(),
        };
        let expected = if count == 1 {
            OpeningError::NotQuery
        } else {
            OpeningError::ReactionCount { count }
        };
        assert_eq!(opening_scope(reply).err(), Some(expected));
    }
    let supplied = Reply::<Local, (), UnderRoot> {
        replies: vec![Reaction::Supply(0, UnderRoot::node())],
    };
    assert_eq!(opening_scope(supplied).err(), Some(OpeningError::NotQuery));
}
