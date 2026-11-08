//! The distinguished opening reply: the question that rides the greeting.
//!
//! No wire frame exists for the opening: [`opening_listing`] validates the
//! locally produced opening reply and returns the listing the root scope
//! derives from, while [`opening_reply`] replays the peer greeting's listing as
//! the message the responder answers. The two must agree on the scope, since
//! one side builds it from its own message and the other from the listing that
//! crossed the wire.

use crate::tree::mirror::streaming::message::LaterMatches;
use crate::tree::mirror::streaming::message::listed;
use before::Version;

use crate::message::Message;
use crate::tree::{
    mirror::streaming::{
        Backend, Local,
        erased::{Reaction, Reply},
    },
    typed::{
        self,
        height::{Height, S, UnderRoot, Z},
    },
};

use super::{
    super::{OpeningError, Scope, opening_listing, opening_reply},
    hash,
};

/// Build a fixed opening node at any type-level height.
trait OpeningNode: Height {
    /// Build the node for this height.
    fn node() -> typed::View<Self>;
}

/// The opening fixture begins with one unit leaf.
impl OpeningNode for Z {
    fn node() -> typed::View<Self> {
        typed::View::leaf(Version::new(), Message::new(()))
    }
}

/// Each higher fixture wraps the lower node beneath radix zero.
impl<H: OpeningNode> OpeningNode for S<H>
where
    S<H>: Height,
{
    fn node() -> typed::View<Self> {
        typed::Node::beneath(H::node().into_node(), 0).into()
    }
}

/// The local opening's listing derives the same scope the remote side replays
/// from the listing carried in the greeting.
#[test]
fn opening_listing_agrees_with_greeting_replay() {
    let listing = vec![listed(3, hash(1)), listed(9, hash(2))];
    let reply = Reply::<<Local as Backend>::Erased> {
        reactions: vec![Reaction::Query(listing.clone())],
        charge: Default::default(),
        last: true,
        carried: Default::default(),
    };

    let local = opening_listing(reply.reactions).expect("canonical opening");
    assert_eq!(local, listing);
    let scope = Scope::opening_unbounded(&local);

    let (replayed, replayed_scope) = opening_reply::<<Local as Backend>::Erased>(listing.clone());
    assert_eq!(replayed_scope, scope);
    let [Reaction::Query(replayed)] = replayed.reactions.as_slice() else {
        panic!("the replayed opening must remain one query")
    };
    assert_eq!(replayed, &listing);
}

/// An empty carried listing replays the empty-tree initiator's opening: one
/// empty `Query`, meaning "I lack the root, send everything", with a root scope
/// holding no positional children.
#[test]
fn empty_listing_replays_the_empty_opening() {
    let (replayed, scope) = opening_reply::<<Local as Backend>::Erased>(Vec::new());
    let [Reaction::Query(listing)] = replayed.reactions.as_slice() else {
        panic!("the replayed opening must be one query")
    };
    assert!(listing.is_empty());
    assert_eq!(scope, Scope::opening_unbounded(&[]));
    assert_eq!(
        scope.positions().next(),
        None,
        "an empty root scope positions nothing"
    );
}

/// One reaction kind in an opening word.
#[derive(Clone, Copy, Debug)]
enum Symbol {
    Query,
    Match,
    Supply,
}

/// Every opening reply of at most three reactions is accepted exactly when it
/// is one query, and otherwise rejected by the clause it breaks: empty, not led
/// by a query, or continuing after its query.
#[test]
fn opening_accepts_exactly_one_query() {
    /// Build the reply a word spells.
    fn reply(word: &[Symbol]) -> Reply<<Local as Backend>::Erased> {
        let node = UnderRoot::node();
        Reply {
            reactions: word
                .iter()
                .map(|symbol| match symbol {
                    Symbol::Query => Reaction::Query(vec![listed(3, hash(1))]),
                    Symbol::Match => Reaction::Match(LaterMatches::NONE),
                    Symbol::Supply => Reaction::Supply(5, <Local as Backend>::erase(node.clone())),
                })
                .collect(),
            charge: Default::default(),
            last: true,
            carried: Default::default(),
        }
    }

    let symbols = [Symbol::Query, Symbol::Match, Symbol::Supply];
    let mut words: Vec<Vec<Symbol>> = vec![Vec::new()];
    for length in 1..=3 {
        let shorter: Vec<_> = words
            .iter()
            .filter(|word| word.len() == length - 1)
            .cloned()
            .collect();
        for word in shorter {
            for symbol in symbols {
                let mut longer = word.clone();
                longer.push(symbol);
                words.push(longer);
            }
        }
    }
    assert_eq!(words.len(), 1 + 3 + 9 + 27, "every word up to length three");

    for word in words {
        let expected = match word.as_slice() {
            [] => Err(OpeningError::Empty),
            [Symbol::Query] => Ok(()),
            [Symbol::Query, ..] => Err(OpeningError::Trailing),
            [_, ..] => Err(OpeningError::NotQuery),
        };
        let actual = opening_listing(reply(&word).reactions).map(|_| ());
        assert_eq!(actual, expected, "word: {word:?}");
    }
}
