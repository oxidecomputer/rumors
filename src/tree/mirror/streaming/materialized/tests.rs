//! The initiator's terminal absorb loop: the [`Completing`](super::Completing)
//! seam's containment enforcement.
//!
//! The session's closing leg is the one ingress the descent walks never
//! see: the initiator's pending leaf requests are answered directly by the
//! counterparty's terminal supplies and absorbed by [`absorb`](super::absorb),
//! so its containment check is a chokepoint of its own and gets its own
//! scripted counterparty here.

use std::convert::Infallible;

use futures::stream;

use super::{
    Error, SupplyLedger, Violation, absorb,
    channel::{QueueKind, QueueRole, channel},
};
use crate::tree::mirror::streaming::erased;
use crate::tree::mirror::streaming::stats::Recorder;
use crate::{
    Version,
    message::Message,
    tree::{
        arb::nth_party,
        mirror::streaming::{Backend, Local},
        typed::{
            self, Path, Prefix,
            height::{Height, Z},
        },
    },
};

/// One tick on the disjoint party `index` (see [`nth_party`]).
fn ticked(index: usize) -> Version {
    let mut version = Version::new();
    version.tick(&nth_party(index));
    version
}

/// Drive [`absorb`](super::absorb) against one scripted closing-leg reply.
///
/// A single pending leaf request, answered by a single leaf supply carrying
/// `leaf_version`, from a counterparty whose greeting declared `declared`
/// and whose set-length ledger is `ledger`.
///
/// Returns the loop's result and what, if anything, it passed up to the
/// assembly above it.
#[allow(clippy::type_complexity)]
fn absorb_scripted(
    declared: Version,
    ledger: SupplyLedger,
    leaf_version: Version,
) -> (
    Result<(), Error<Infallible>>,
    Option<Option<typed::Node<(), Z>>>,
) {
    // The request whose answer the script supplies: the leaf radix is the
    // path's last byte, zero here.
    let path = Path::from([0u8; 32]);
    let (queries, queries_rx) =
        channel::<Prefix<Z>>(QueueRole::new(QueueKind::LeafRequests, Z::HEIGHT), 1);
    pollster::block_on(queries.send(Prefix::containing(&path))).expect("the loop is live");
    drop(queries);

    let (returns, mut returns_rx) = channel::<Option<<Local as Backend<()>>::Erased>>(
        QueueRole::new(QueueKind::TerminalLeafResolutions, Z::HEIGHT),
        1,
    );

    let leaf = typed::Node::leaf(leaf_version, Message::new(()));
    let requests = stream::iter(vec![erased::Reply {
        replies: vec![erased::Reaction::Supply(
            0,
            <Local as Backend<()>>::erase(leaf),
        )],
    }]);

    let result = pollster::block_on(absorb::<Local, ()>(
        declared,
        ledger,
        requests,
        queries_rx,
        returns,
        Recorder::default(),
    ));
    let returned = pollster::block_on(async move { returns_rx.recv().await })
        .map(|leaf| leaf.map(<Local as Backend<()>>::assume::<Z>));
    (result, returned)
}

/// A terminal leaf supply whose version the declared greeting version
/// contains is absorbed and passed up to the assembly.
///
/// The happy path that keeps the rejections below honest: the scripted
/// shape differs from theirs only in the supplied version.
#[test]
fn terminal_absorb_accepts_a_contained_supply() {
    let declared = ticked(0);
    let (result, returned) = absorb_scripted(
        declared.clone(),
        SupplyLedger::new(u64::MAX),
        declared.clone(),
    );
    assert!(result.is_ok(), "a contained supply is absorbed: {result:?}");
    let leaf = returned
        .expect("the absorbed leaf is passed up")
        .expect("the supplied leaf resolves to a node");
    assert_eq!(
        leaf.ceiling(),
        &declared,
        "the absorbed leaf carries the supplied version",
    );
}

/// A terminal leaf supply whose version strictly dominates the declared
/// greeting version fails the closing leg with `UncontainedSupply`.
///
/// The closing leg is the descent's last ingress; waving the escaped leaf
/// through here would plant an unredactable record after every other
/// chokepoint held.
#[test]
fn terminal_absorb_rejects_a_dominating_supply() {
    let declared = ticked(0);
    let mut escaped = declared.clone();
    escaped.tick(&nth_party(0));
    let (result, returned) = absorb_scripted(declared, SupplyLedger::new(u64::MAX), escaped);
    assert!(
        matches!(result, Err(Error::Violation(Violation::UncontainedSupply))),
        "a dominating supply is rejected: {result:?}",
    );
    assert!(returned.is_none(), "nothing is passed up past a rejection");
}

/// A terminal leaf supply whose version is incomparable with the declared
/// greeting version fails the closing leg with `UncontainedSupply`.
///
/// Containment is judged on a partial order: an escape onto a disjoint
/// party is just as uncontained as strict dominance, the misreading a bare
/// `!(a <= b)` invites.
#[test]
fn terminal_absorb_rejects_an_incomparable_supply() {
    let declared = ticked(0);
    let escaped = ticked(31);
    let (result, returned) = absorb_scripted(declared, SupplyLedger::new(u64::MAX), escaped);
    assert!(
        matches!(result, Err(Error::Violation(Violation::UncontainedSupply))),
        "an incomparable supply is rejected: {result:?}",
    );
    assert!(returned.is_none(), "nothing is passed up past a rejection");
}

/// A terminal leaf supply past the declared set length fails the closing
/// leg with `OverdrawnSupply`.
///
/// The closing leg is the one ingress the connected greeting-lie family
/// cannot reach: an empty declaration trips at the session's first
/// absorbed supply, never at a terminal leaf, so the terminal arm of the
/// set-length guard is pinned here at its own seam — a spent ledger, one
/// contained honest leaf.
#[test]
fn terminal_absorb_rejects_an_overdrawn_supply() {
    let declared = ticked(0);
    let (result, returned) = absorb_scripted(declared.clone(), SupplyLedger::new(0), declared);
    assert!(
        matches!(result, Err(Error::Violation(Violation::OverdrawnSupply))),
        "a supply past the declared set length is rejected: {result:?}",
    );
    assert!(returned.is_none(), "nothing is passed up past a rejection");
}
