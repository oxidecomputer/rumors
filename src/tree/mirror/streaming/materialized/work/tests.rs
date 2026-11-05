//! The contract of [`assemble`], the session's reassembly primitive.
//!
//! It must fill `Pending` slots from the level stream strictly in order,
//! honor `Ready(None)` as deletion, resolve the empty resolution to `None`
//! through [`Backend::parent`], chain across two instances, and pass
//! errors through unchanged. Counterparty-fault classification lives in
//! [`violations`].

use crate::network::NetworkKeys;
use crate::tree::mirror::streaming::message::LaterMatches;
use std::{collections::BTreeSet, convert::Infallible, future};

use futures::stream::{self, StreamExt, TryStreamExt};
use proptest::prelude::*;

use super::{Work, answer, assembly::assemble, charges, resolver::Resolver};

mod violations;

use crate::tree::mirror::streaming::remote::{Replay, ReplyLevel, Scope, derive_questions};
use crate::tree::mirror::streaming::stats::Recorder;
use crate::{
    Version,
    message::Message,
    testing::run_to_quiescence,
    tree::{
        arb::nth_party,
        mirror::streaming::{
            Backend, ErasedNode, FAN, Failing, Failure, Local, Operation,
            byte_set::ByteSet,
            erased::{Reaction, Reply},
            holding::Holding,
            ledger::{Charge, Ledger, ReplyBound},
            materialized::{Error, Resolution, Resolve, SupplyLedger, Violation},
            message::{Grandchildren, Listed},
        },
        typed::{
            self, ErasedPrefix, Path, Prefix,
            height::{S, Z},
        },
    },
};

/// The in-memory backend's erased node representation, which resolutions
/// and level returns carry.
type Erased = <Local as Backend>::Erased;

/// Erase one typed leaf the way the walk's payloads carry it.
fn erased(node: typed::View<Z>) -> Erased {
    <Local as Backend>::erase(node)
}

/// A distinct leaf per call: the versions differ so hashes do.
fn leaf(version: &mut Version) -> typed::View<Z> {
    version.tick(&nth_party(0));
    typed::View::leaf(version.clone(), Message::new(()))
}

/// The height-`Z` key with `parent` as its first byte and `radix` as its
/// last, zeros between.
fn key(parent: u8, radix: u8) -> Prefix<Z> {
    let mut bytes = [0u8; 32];
    bytes[0] = parent;
    bytes[31] = radix;
    Path::from(bytes).into()
}

/// The parent-height prefix of [`key`]`(parent, _)`.
fn parent_prefix(parent: u8) -> Prefix<S<Z>> {
    key(parent, 0).pop().0
}

/// Build the expected parent of a radix group directly through the backend.
fn parent_of(
    prefix: Prefix<S<Z>>,
    children: Vec<(u8, Option<typed::View<Z>>)>,
) -> Option<typed::View<S<Z>>> {
    pollster::block_on(<Local as Backend>::parent(Local, prefix, None, children))
        .unwrap_or_else(|e| match e {})
}

/// One scope awaiting reassembly, or the error that prevents it.
type Item = Result<Resolution<Erased>, Error<Infallible>>;
/// One reconstructed child, its deletion, or its failure.
type Level = Result<Option<Erased>, Error<Infallible>>;

proptest! {
    /// A detected response error ends the work even when its reply queue is
    /// full or its consumer has gone away, ahead of a terminal failure.
    #[test]
    fn response_failure_does_not_wait_for_its_consumer(
        buffered in any::<bool>(),
        dropped in any::<bool>(),
        terminal_ready in any::<bool>(),
    ) {
        let mut work = Work::new(Local, Recorder::default(), Default::default(), Some(crate::network::NetworkKeys::fixture()));
        // A closed consumer may stop a pump before later items are read.
        // Put the error first in that case so it is actually detected.
        let buffered = buffered && !dropped;
        let messages = stream::iter(
            buffered.then_some(Ok(Reply {
                reactions: Vec::new(), charge: Default::default(), last: true, carried: Default::default(),
            })).into_iter().chain([Err(Error::Violation(Violation::UnaskedReply))]),
        );
        let responses = work.respond::<Z>(messages);
        let _responses = (!dropped).then_some(responses);
        let finish = async move {
            if terminal_ready {
                Err(Error::Violation(Violation::UnansweredQuery))
            } else {
                future::pending::<Result<(), _>>().await
            }
        };
        let result = run_to_quiescence(work.execute(Box::pin(finish)));
        prop_assert!(matches!(result, Ok(Err(Error::Violation(Violation::UnaskedReply)))),
            "response error was lost: {:?}", result);
    }
}

/// A work error cancels parked peers and retains its original error identity.
#[test]
fn work_failure_preempts_parked_tasks() {
    let mut work: Work<Failing<Local>> = Work::new(
        Failing::after(Local, usize::MAX),
        Recorder::default(),
        Default::default(),
        Some(crate::network::NetworkKeys::fixture()),
    );
    for _ in 0..31 {
        work.tasks.push(Box::pin(future::pending()));
    }
    work.tasks.push(Box::pin(async {
        Err(Error::Backend(Failure::Injected(Operation::Children {
            height: 1,
        })))
    }));

    let finish = future::pending::<Result<(), Error<Failure<Infallible>>>>();
    let result = run_to_quiescence(work.execute(Box::pin(finish)));
    let error = result
        .expect("a work failure must terminate the executor")
        .expect_err("the injected task must fail");
    assert!(matches!(
        error,
        Error::Backend(Failure::Injected(Operation::Children { height: 1 }))
    ));
}

/// Drive [`assemble`] over in-memory streams and collect its outputs.
fn assembled(
    resolutions: Vec<Item>,
    level: Vec<Level>,
) -> Vec<Result<Option<Erased>, Error<Infallible>>> {
    pollster::block_on(
        assemble::<Local>(Local, stream::iter(resolutions), stream::iter(level))
            .collect::<Vec<_>>(),
    )
}

/// `Pending` slots are filled from the level stream strictly in order,
/// interleaved with `Ready` slots at their stated radices.
#[test]
fn fills_pendings_in_order() {
    let mut version = Version::new();
    let (a, b, c) = (leaf(&mut version), leaf(&mut version), leaf(&mut version));

    let resolution = Resolution {
        prefix: parent_prefix(3).erase(),
        resolved: vec![
            (1, Resolve::Pending),
            (2, Resolve::Ready(Some(erased(b.clone())))),
            (5, Resolve::Pending),
        ],
        original: None,
        charge: Default::default(),
    };
    let output = assembled(
        vec![Ok(resolution)],
        vec![Ok(Some(erased(a.clone()))), Ok(Some(erased(c.clone())))],
    );

    let expected = parent_of(
        parent_prefix(3),
        vec![(1, Some(a)), (2, Some(b)), (5, Some(c))],
    );
    assert_eq!(
        output
            .into_iter()
            .map(|item| {
                item.expect("no errors were fed in")
                    .map(|node| node.hash(&parent_prefix(3).erase(), NetworkKeys::fixture().hash()))
            })
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash(NetworkKeys::fixture().hash()))],
    );
}

/// A `Ready(None)` slot is a deletion: it reaches `Backend::parent` as a
/// `None` entry, and the parent assembles from the survivors alone.
#[test]
fn ready_none_is_deletion() {
    let mut version = Version::new();
    let a = leaf(&mut version);

    let resolution = Resolution {
        prefix: parent_prefix(3).erase(),
        resolved: vec![
            (1, Resolve::Ready(Some(erased(a.clone())))),
            (2, Resolve::Ready(None)),
        ],
        original: None,
        charge: Default::default(),
    };
    let output = assembled(vec![Ok(resolution)], vec![]);

    let expected = parent_of(parent_prefix(3), vec![(1, Some(a))]);
    assert_eq!(
        output
            .into_iter()
            .map(|item| {
                item.expect("no errors were fed in")
                    .map(|node| node.hash(&parent_prefix(3).erase(), NetworkKeys::fixture().hash()))
            })
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash(NetworkKeys::fixture().hash()))],
    );
}

/// The empty resolution — the pruned-to-nothing reply to a request — reaches
/// `Backend::parent` with an empty group and resolves to `None`.
#[test]
fn empty_resolution_assembles_to_none() {
    let resolution = Resolution {
        prefix: parent_prefix(3).erase(),
        resolved: vec![],
        original: None,
        charge: Default::default(),
    };
    let output = assembled(vec![Ok(resolution)], vec![]);
    assert!(matches!(output.as_slice(), [Ok(None)]));
}

/// An all-deleted resolution cascades: every slot `Ready(None)` assembles to
/// a `None` return, deleting the scope one level up.
#[test]
fn all_deleted_resolution_assembles_to_none() {
    let resolution = Resolution {
        prefix: parent_prefix(3).erase(),
        resolved: vec![(1, Resolve::Ready(None)), (2, Resolve::Ready(None))],
        original: None,
        charge: Default::default(),
    };
    let output = assembled(vec![Ok(resolution)], vec![]);
    assert!(matches!(output.as_slice(), [Ok(None)]));
}

/// Two chained instances reproduce a stage's shape: the lower assembler's
/// outputs fill the upper assembler's `Pending`s, in order.
#[test]
fn chains_two_instances() {
    let mut version = Version::new();
    let (a, b) = (leaf(&mut version), leaf(&mut version));

    let lower: Vec<Item> = vec![Ok(Resolution {
        prefix: parent_prefix(3).erase(),
        resolved: vec![
            (1, Resolve::Ready(Some(erased(a.clone())))),
            (7, Resolve::Ready(Some(erased(b.clone())))),
        ],
        original: None,
        charge: Default::default(),
    })];
    let upper: Vec<Item> = vec![Ok(Resolution {
        prefix: parent_prefix(3).pop().0.erase(),
        resolved: vec![(3, Resolve::Pending)],
        original: None,
        charge: Default::default(),
    })];

    let chained = assemble::<Local>(
        Local,
        stream::iter(upper),
        assemble::<Local>(Local, stream::iter(lower), stream::empty()),
    );
    let output =
        pollster::block_on(chained.try_collect::<Vec<_>>()).expect("no errors were fed in");

    let inner = parent_of(parent_prefix(3), vec![(1, Some(a)), (7, Some(b))])
        .expect("a non-empty resolved group constructs its parent");
    let expected = pollster::block_on(<Local as Backend>::parent(
        Local,
        parent_prefix(3).pop().0,
        None,
        vec![(3, Some(inner))],
    ))
    .unwrap_or_else(|e| match e {});
    assert_eq!(
        output
            .into_iter()
            .map(|node| node.map(|node| node.hash(
                &parent_prefix(3).pop().0.erase(),
                NetworkKeys::fixture().hash()
            )))
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash(NetworkKeys::fixture().hash()))],
    );
}

/// An error arriving on the resolutions stream ends the output with that
/// error: nothing downstream sees a partial scope.
#[test]
fn resolution_error_passes_through() {
    let output = assembled(vec![Err(Error::Violation(Violation::UnaskedReply))], vec![]);
    assert!(matches!(
        output.as_slice(),
        [Err(Error::Violation(Violation::UnaskedReply))],
    ));
}

/// An error arriving on the level stream surfaces through the `Pending`
/// slot that pulled it.
#[test]
fn level_error_passes_through() {
    let resolution = Resolution {
        prefix: parent_prefix(3).erase(),
        resolved: vec![(1, Resolve::Pending)],
        original: None,
        charge: Default::default(),
    };
    let output = assembled(
        vec![Ok(resolution)],
        vec![Err(Error::Violation(Violation::UnansweredQuery))],
    );
    assert!(matches!(
        output.as_slice(),
        [Err(Error::Violation(Violation::UnansweredQuery))],
    ));
}

/// The floor allowance of a walk over the in-memory backend is the "about
/// 26 MiB" its documentation states, on the 64-bit targets the figure is
/// measured on.
#[cfg(target_pointer_width = "64")]
#[test]
fn the_floor_allowance_is_the_documented_figure() {
    const MIB: usize = 1 << 20;
    let allowance = super::floor_allowance::<Erased, Infallible>();
    assert!(
        (25 * MIB + MIB / 2..26 * MIB + MIB / 2).contains(&allowance),
        "the floor allowance is {allowance} B, not about 26 MiB",
    );
}

/// A finished resolver returns its question's own charge, leaving the
/// resolution its slots' charge alone while it waits for assembly.
///
/// The question's queue slot is gone once its stage received it, and the
/// buffer of this side's children is gone once the resolver has walked it
/// into the resolution's slots, which the slots' charge covers.
#[test]
fn finished_resolution_keeps_no_charge_for_its_question() {
    let ledger = Ledger::default();
    let mut version = Version::default();
    let prefix = parent_prefix(7).erase();
    let ours: Vec<(u8, Erased)> = (1..=3)
        .map(|radix| (radix, erased(leaf(&mut version))))
        .collect();
    let children = ours.len();
    let bound = ReplyBound::of(prefix.height(), children, Grandchildren::ONE);
    let mut grant = ledger.charge(Holding::Questions, 1 << 20);
    let query = charges::query(&mut grant, prefix, None, ours, bound);
    drop(grant);
    assert!(
        ledger.held_for(Holding::Questions) > 0,
        "the question charges its queue slot and its children's buffer",
    );

    let supplies = SupplyLedger::new(u64::MAX);
    let stats = Recorder::default();
    let mut resolver = Resolver::<Local>::new(query, &version, &supplies, &stats);
    // One match at the first child, covering the rest.
    let dispute = resolver
        .react(Reaction::Match(LaterMatches::new((2..=3).collect())))
        .expect("the match pairs with the first child and covers the rest");
    assert!(dispute.is_none(), "a match resolves its children here");
    let resolution = resolver.finish().expect("every child was answered");

    assert_eq!(
        ledger.held_for(Holding::Questions),
        0,
        "the question's charge returns when its reply is finished",
    );
    assert_eq!(
        ledger.held(),
        resolution.charge.bytes(),
        "the waiting resolution holds its slots' charge and nothing more",
    );
    drop(resolution);
    assert_eq!(ledger.held(), 0, "assembly returns the rest");
}

proptest! {
    /// An answer's resolution has a slot per child either side holds, and
    /// exactly the capacity its unit's grant prices: the union of this side's
    /// radices, as the backend reports them, and the peer's listing.
    ///
    /// A child both sides hold sits at one radix and takes one slot, so the
    /// price counts it once, and every price is at most a fan. Each side's
    /// radices are drawn with repeats and deduplicated here, so the sides
    /// range from empty to full and from disjoint to identical.
    #[test]
    fn answer_resolution_holds_exactly_its_price(
        ours in prop::collection::vec(any::<u8>(), 0..=2 * FAN),
        theirs in prop::collection::vec(any::<u8>(), 0..=2 * FAN),
    ) {
        let ours: BTreeSet<u8> = ours.into_iter().collect();
        let theirs: BTreeSet<u8> = theirs.into_iter().collect();
        let mut version = Version::default();
        let ours_nodes: Vec<(u8, Erased)> = ours
            .iter()
            .map(|&radix| (radix, erased(leaf(&mut version))))
            .collect();
        let theirs_listing: Vec<Listed> = theirs
            .iter()
            .map(|&radix| Listed {
                radix,
                hash: leaf(&mut version).hash(NetworkKeys::fixture().hash()),
                grandchildren: Grandchildren::ONE,
                children_less_one: 0,
            })
            .collect();
        let children = ours.union(&theirs).count();
        let unknown = Version::default();
        let stats = Recorder::default();

        let (_, resolved) = answer::leaf_parent(
            &unknown,
            ours_nodes.clone(),
            theirs_listing.clone(),
            &stats,
            &mut Charge::default(),
        )
        .into_parts();
        prop_assert_eq!(resolved.len(), children);
        // The price reads this side's radices from a node holding exactly
        // these children, as the walk reads them before exploding it.
        let reported: ByteSet = ours.iter().copied().collect();
        let priced = charges::scope_children(reported, &theirs_listing).either;
        prop_assert_eq!(priced, children);
        prop_assert!(priced <= FAN);
        prop_assert_eq!(resolved.capacity(), priced, "a leaf parent's slots");

        // The scope sits one level above the leaves, where the drawn leaves
        // sit as its children.
        let internal = pollster::block_on(answer::internal(
            &Local,
            &unknown,
            parent_prefix(0).erase(),
            ours_nodes,
            theirs_listing,
            Some(&*NetworkKeys::fixture()),
            &stats,
            &mut Charge::default(),
        ))
        .expect("the in-memory backend lists exactly the children it reports");
        let (_, resolved) = internal.into_parts();
        prop_assert_eq!(resolved.len(), children);
        prop_assert_eq!(resolved.capacity(), priced, "an internal scope's slots");
    }
}

/// A prefix at `height`, its bytes all `byte`.
fn prefix_at(height: usize, byte: u8) -> ErasedPrefix {
    (height..32).fold(Prefix::new().erase(), |prefix, _| prefix.push(byte))
}

/// One reply a question about a node at `height` can receive, within the
/// bound its question reserved for.
///
/// That is the listing the question sent, the peer's grandchild count of
/// the node, and for each listed child, in order, a match or a query
/// listing that many children.
///
/// About a leaf's parent, every query requests its leaf and lists nothing.
fn arb_reply_to_question() -> impl Strategy<Value = (usize, usize, usize, Vec<Option<usize>>)> {
    (1..=4_usize, 1..=24_usize).prop_flat_map(|(height, listed)| {
        let grandchildren = 1..=4 * listed;
        (Just(height), Just(listed), grandchildren).prop_flat_map(move |(height, listed, count)| {
            let entries = if height == 1 {
                0..=0_usize
            } else {
                0..=4_usize
            };
            let reactions = proptest::collection::vec(proptest::option::of(entries), listed);
            (Just(height), Just(listed), Just(count), reactions)
        })
    })
}

proptest! {
    /// The records the take derives from any reply within its question's
    /// bound hold no more than the question's records part, at a leaf's
    /// parent as at every height above.
    ///
    /// Each record holds exactly what [`charges::derived_record_bytes`]
    /// prices it at.
    ///
    /// The reply's queries list at most what the peer's count leaves, which
    /// the reply's bound allows; about a leaf's parent, each query requests
    /// its leaf, and the record the take derives for it names that leaf.
    #[test]
    fn derived_records_fit_the_records_part(
        (height, listed, count, reactions) in arb_reply_to_question(),
    ) {
        let parent = prefix_at(height, 0);
        let asked: Vec<Listed> = (0..listed)
            .map(|radix| Listed {
                radix: u8::try_from(radix).expect("a few dozen radices fit a byte"),
                hash: Default::default(),
                grandchildren: Grandchildren::ONE,
                children_less_one: 0,
            })
            .collect();
        let peer_count = Grandchildren::new(count).expect("the count is in range");
        let scope = Scope::new(parent, &asked, peer_count);
        let bound = scope.reply_bound();
        // The peer's queries list at most what its count allows.
        let mut left = bound.listing;
        // A match at the first matching position covers the later ones,
        // which take no reaction.
        let first_match = reactions.iter().position(Option::is_none);
        let later_matches = reactions
            .iter()
            .enumerate()
            .filter(|&(position, reaction)| reaction.is_none() && Some(position) != first_match)
            .map(|(position, _)| u8::try_from(position).expect("a few radices fit a byte"))
            .collect();
        let mut later_matches = Some(LaterMatches::new(later_matches));
        let replied: Vec<Reaction<Erased>> = reactions
            .iter()
            .enumerate()
            .filter(|&(position, reaction)| reaction.is_some() || Some(position) == first_match)
            .map(|(_, reaction)| match reaction {
                None => Reaction::Match(later_matches.take().expect("one first match")),
                Some(entries) => {
                    let entries = (*entries).min(left);
                    left -= entries;
                    let listing = (0..entries)
                        .map(|radix| Listed {
                            radix: u8::try_from(radix).expect("a few radices fit a byte"),
                            hash: Default::default(),
                            grandchildren: Grandchildren::ONE,
                            children_less_one: 0,
                        })
                        .collect();
                    Reaction::Query(listing)
                }
            })
            .collect();
        let level = if height == 1 { ReplyLevel::Leaf } else { ReplyLevel::Branch };
        let derived = derive_questions(level, &scope, &mut Replay::default(), &replied)
            .expect("a reply at its positions derives");
        let listings = replied.iter().filter_map(|reaction| match reaction {
            Reaction::Query(listing) => Some(listing.len()),
            _ => None,
        });
        for (record, listing) in derived.iter().zip(listings) {
            prop_assert_eq!(record.held_bytes(), charges::derived_record_bytes(height, listing));
        }
        let held: usize = derived.iter().map(Scope::held_bytes).sum();
        let room = charges::records_room(height, bound, listed);
        prop_assert!(held <= room, "{held} B of records over a records part of {room} B");
    }
}

/// One answer processing a reply builds about a child the reply queries:
/// the entries the query lists and the children this side holds beyond
/// them.
type ChildAnswer = (usize, usize);

/// The answers processing a fully disputed reply builds one level down,
/// one for each query the reply carries, each listing at least one entry.
fn arb_child_answers() -> impl Strategy<Value = Vec<ChildAnswer>> {
    proptest::collection::vec((1..=FAN, 0..=FAN), 1..=6)
}

/// The listing entries the queries behind `answers` carry together: the
/// listing a reply's bound must allow for processing to build them.
fn listing(answers: &[ChildAnswer]) -> usize {
    answers.iter().map(|&(entries, ..)| entries).sum()
}

/// What processing admits one level down for `answers`: each answer's grant
/// as the walk prices it.
///
/// Each answers the children its query lists and those this side holds
/// beyond them, at most a fan, sharing none of the listed children, which
/// prices no covered set: the least such an answer costs.
fn answers_cost(answers: &[ChildAnswer]) -> usize {
    answers
        .iter()
        .map(|&(entries, extra)| {
            charges::answer_bound::<Erased, Infallible>((entries + extra).min(FAN), 0)
        })
        .sum()
}

proptest! {
    /// A question's need is a lower bound on what processing any fully
    /// disputed reply within its bound admits one level down.
    ///
    /// A fully disputed reply lists every entry its bound allows, and
    /// processing builds an answer one level down for each query it
    /// carries, answering at least the children the query lists, whatever
    /// this side holds beyond them; each costs at least what the need prices it at. The questions that
    /// processing sends two levels down are no part of the need: they claim
    /// at their own depth.
    #[test]
    fn the_need_bounds_a_fully_disputed_replys_answers_below(answers in arb_child_answers()) {
        let bound = ReplyBound { reactions: FAN, listing: listing(&answers) };
        let need = charges::processing_need::<Erased, Infallible>(bound);
        let cost = answers_cost(&answers);
        prop_assert!(need <= cost, "a need of {need} B over {cost} B admitted one level down");
    }

    /// A question's need is exactly what processing the cheapest fully
    /// disputed reply within its bound admits one level down, computed
    /// answer by answer, and nothing more.
    ///
    /// That reply makes as few answers as hold its entries, a fan each,
    /// answering only those children and exploding nothing. The need prices
    /// those answers alone: the questions two levels down, which claim at
    /// their own depth, add nothing to it. A reply that lists nothing is
    /// predicted to need nothing.
    #[test]
    fn the_need_prices_only_the_cheapest_disputed_replys_answers(listing in 0..=65_536usize) {
        let bound = ReplyBound { reactions: FAN, listing };
        let need = charges::processing_need::<Erased, Infallible>(bound);
        let answers: Vec<ChildAnswer> = (0..listing)
            .step_by(FAN)
            .map(|first| ((listing - first).min(FAN), 0))
            .collect();
        prop_assert_eq!(need, answers_cost(&answers));
    }
}
