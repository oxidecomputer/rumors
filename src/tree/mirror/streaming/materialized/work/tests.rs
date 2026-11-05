//! The contract of [`assemble`], the session's reassembly primitive.
//!
//! It must fill `Pending` slots from the level stream strictly in order,
//! honor `Ready(None)` as deletion, resolve the empty resolution to `None`
//! through [`Backend::parent`], chain across two instances, and pass
//! errors through unchanged. Counterparty-fault classification lives in
//! [`violations`].

use std::{collections::BTreeSet, convert::Infallible, future};

use futures::stream::{self, StreamExt, TryStreamExt};
use proptest::prelude::*;

use super::{Work, answer, assembly::assemble, charges, resolver::Resolver};

mod violations;

use crate::tree::mirror::streaming::stats::Recorder;
use crate::{
    Version,
    message::Message,
    testing::run_to_quiescence,
    tree::{
        arb::nth_party,
        mirror::streaming::{
            Backend, FAN, Failing, Failure, Local, Operation,
            erased::{Reaction, Reply},
            holding::Holding,
            ledger::{Ledger, ReplyBound},
            materialized::{Error, Resolution, Resolve, SupplyLedger, Violation},
            message::{Grandchildren, Listed},
        },
        typed::{
            self, Path, Prefix,
            height::{S, Z},
        },
    },
};

/// The in-memory backend's erased node representation, which resolutions
/// and level returns carry.
type Erased = <Local as Backend>::Erased;

/// Erase one typed leaf the way the walk's payloads carry it.
fn erased(node: typed::Node<Z>) -> Erased {
    <Local as Backend>::erase(node)
}

/// A distinct leaf per call: the versions differ so hashes do.
fn leaf(version: &mut Version) -> typed::Node<Z> {
    version.tick(&nth_party(0));
    typed::Node::leaf(version.clone(), Message::new(()))
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
    children: Vec<(u8, Option<typed::Node<Z>>)>,
) -> Option<typed::Node<S<Z>>> {
    pollster::block_on(<Local as Backend>::parent(Local, prefix, children))
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
        let mut work = Work::new(Local, Recorder::default(), Default::default());
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
        charge: Default::default(),
        copies: Default::default(),
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
            .map(|item| item.expect("no errors were fed in").map(|node| node.hash()))
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash())],
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
        charge: Default::default(),
        copies: Default::default(),
    };
    let output = assembled(vec![Ok(resolution)], vec![]);

    let expected = parent_of(parent_prefix(3), vec![(1, Some(a))]);
    assert_eq!(
        output
            .into_iter()
            .map(|item| item.expect("no errors were fed in").map(|node| node.hash()))
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash())],
    );
}

/// The empty resolution — the pruned-to-nothing reply to a request — reaches
/// `Backend::parent` with an empty group and resolves to `None`.
#[test]
fn empty_resolution_assembles_to_none() {
    let resolution = Resolution {
        prefix: parent_prefix(3).erase(),
        resolved: vec![],
        charge: Default::default(),
        copies: Default::default(),
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
        charge: Default::default(),
        copies: Default::default(),
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
        charge: Default::default(),
        copies: Default::default(),
    })];
    let upper: Vec<Item> = vec![Ok(Resolution {
        prefix: parent_prefix(3).pop().0.erase(),
        resolved: vec![(3, Resolve::Pending)],
        charge: Default::default(),
        copies: Default::default(),
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
        vec![(3, Some(inner))],
    ))
    .unwrap_or_else(|e| match e {});
    assert_eq!(
        output
            .into_iter()
            .map(|node| node.map(|node| node.hash()))
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash())],
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
        charge: Default::default(),
        copies: Default::default(),
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
/// 34 MiB" its documentation states, on the 64-bit targets the figure is
/// measured on.
#[cfg(target_pointer_width = "64")]
#[test]
fn the_floor_allowance_is_the_documented_figure() {
    const MIB: usize = 1 << 20;
    let allowance = super::floor_allowance::<Erased, Infallible>();
    assert!(
        (33 * MIB + MIB / 2..34 * MIB + MIB / 2).contains(&allowance),
        "the floor allowance is {allowance} B, not about 34 MiB",
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
    let query = charges::query(&mut grant, prefix, ours, bound, 0);
    drop(grant);
    assert!(
        ledger.held_for(Holding::Questions) > 0,
        "the question charges its queue slot and its children's buffer",
    );

    let supplies = SupplyLedger::new(u64::MAX);
    let stats = Recorder::default();
    let mut resolver = Resolver::<Local>::new(query, &version, &supplies, &stats);
    for _ in 0..children {
        let dispute = resolver
            .react(Reaction::Match)
            .expect("each match pairs with a child");
        assert!(dispute.is_none(), "a match resolves its child here");
    }
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
    /// An answer's resolution has a slot per child either side holds, and no
    /// more capacity than its unit's grant prices: the two listings' sum, at
    /// most a fan.
    ///
    /// Every child sits at its own radix, so the listings' union is at most
    /// their sum and at most a fan.
    /// Each side's radices are drawn with repeats and deduplicated here, so
    /// the sides range from empty to nearly full.
    #[test]
    fn answer_resolution_holds_at_most_a_fan(
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
                hash: erased(leaf(&mut version)).hash(),
                grandchildren: Grandchildren::ONE,
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
        )
        .into_parts();
        prop_assert_eq!(resolved.len(), children);
        let priced = charges::answer_children(ours.len(), theirs.len());
        prop_assert!(priced <= FAN);
        prop_assert!(resolved.capacity() <= priced, "a leaf parent's slots: {}", resolved.capacity());

        let internal = pollster::block_on(answer::internal(
            &Local,
            &unknown,
            Prefix::new().erase(),
            ours_nodes,
            theirs_listing,
            &stats,
        ))
        .unwrap_or_else(|never| match never {});
        let (_, resolved) = internal.into_parts();
        prop_assert_eq!(resolved.len(), children);
        prop_assert!(resolved.capacity() <= priced, "an internal scope's slots: {}", resolved.capacity());
    }
}
