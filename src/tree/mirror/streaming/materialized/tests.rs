//! The initiator's terminal absorb loop: the closing leg's classification
//! of every reply it can receive.
//!
//! The session's closing leg is the one ingress the descending walks never
//! see: the initiator's pending leaf requests are answered directly by the
//! counterparty's terminal supplies and absorbed by [`absorb`](super::absorb),
//! so it gets its own scripted counterparty here, for the accepted shape
//! and for every malformed one.

use std::convert::Infallible;

use futures::stream;
use proptest::prelude::*;

use super::{
    Error, SupplyLedger, Violation, absorb,
    channel::{QueueKind, QueueRole, channel, with_schedule},
};
use crate::tree::mirror::streaming::erased::{Reaction, Reply};
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

/// The in-memory backend's erased node representation, which the closing
/// leg's replies carry.
type Erased = <Local as Backend>::Erased;

/// The radix the one scripted request asks for: the last byte of its path.
const REQUESTED: u8 = 0;

/// One tick on the disjoint party `index` (see [`nth_party`]).
fn ticked(index: usize) -> Version {
    let mut version = Version::new();
    version.tick(&nth_party(index));
    version
}

/// A leaf supply at `radix` carrying `version`.
fn supply(radix: u8, version: Version) -> Reaction<Erased> {
    let leaf = typed::Node::leaf(version, Message::new(()));
    Reaction::Supply(radix, <Local as Backend>::erase(leaf))
}

/// Drive [`absorb`](super::absorb) against one scripted closing leg.
///
/// A single pending leaf request at radix [`REQUESTED`], answered by the
/// scripted `replies`, from a counterparty whose greeting declared
/// `declared` and whose set-length ledger is `ledger`.
///
/// Returns the loop's result and what, if anything, it passed up to the
/// assembly above it.
#[allow(clippy::type_complexity)]
fn absorb_scripted(
    declared: Version,
    ledger: SupplyLedger,
    replies: Vec<Reply<Erased>>,
) -> (
    Result<(), Error<Infallible>>,
    Option<Option<typed::Node<Z>>>,
) {
    let mut bytes = [0u8; 32];
    bytes[31] = REQUESTED;
    let path = Path::from(bytes);
    let (queries, queries_rx) =
        channel::<Prefix<Z>>(QueueRole::new(QueueKind::LeafRequests, Z::HEIGHT), 1);
    pollster::block_on(queries.send(Prefix::containing(&path))).expect("the loop is live");
    drop(queries);

    let (returns, mut returns_rx) = channel::<Option<Erased>>(
        QueueRole::new(QueueKind::TerminalLeafResolutions, Z::HEIGHT),
        1,
    );

    let result = pollster::block_on(absorb::<Local>(
        declared,
        ledger,
        stream::iter(replies),
        queries_rx,
        returns,
        Recorder::default(),
    ));
    let returned = pollster::block_on(async move { returns_rx.recv().await })
        .map(|leaf| leaf.map(<Local as Backend>::assume::<Z>));
    (result, returned)
}

/// The accepted shape: the one requested leaf, supplied once.
fn requested(version: Version) -> Vec<Reply<Erased>> {
    vec![Reply {
        replies: vec![supply(REQUESTED, version)],
    }]
}

/// A terminal leaf supply whose version the declared greeting version
/// contains is absorbed and passed up to the assembly.
///
/// The accepted shape, from which the rejections below differ only in the
/// scripted reply.
#[test]
fn terminal_absorb_accepts_a_contained_supply() {
    let declared = ticked(0);
    let (result, returned) = absorb_scripted(
        declared.clone(),
        SupplyLedger::new(u64::MAX),
        requested(declared.clone()),
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
    let (result, returned) =
        absorb_scripted(declared, SupplyLedger::new(u64::MAX), requested(escaped));
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
    let (result, returned) =
        absorb_scripted(declared, SupplyLedger::new(u64::MAX), requested(escaped));
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
/// set-length guard is pinned here directly: a spent ledger, one contained
/// leaf.
#[test]
fn terminal_absorb_rejects_an_overdrawn_supply() {
    let declared = ticked(0);
    let (result, returned) =
        absorb_scripted(declared.clone(), SupplyLedger::new(0), requested(declared));
    assert!(
        matches!(result, Err(Error::Violation(Violation::OverdrawnSupply))),
        "a supply past the declared set length is rejected: {result:?}",
    );
    assert!(returned.is_none(), "nothing is passed up past a rejection");
}

/// One deliberately malformed closing-leg script and the exact violation
/// it must surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Injection {
    /// The reply stream ends with the request outstanding.
    UnansweredQuery,
    /// A second reply follows the one the request claimed.
    UnaskedReply,
    /// A `Match`, leading or after the requested supply.
    Match { after_supply: bool },
    /// A `Query`, leading or after the requested supply.
    Query { after_supply: bool },
    /// The requested radix supplied twice.
    DuplicateSupply,
    /// A supply at a radix nobody requested, alone or after the requested
    /// one.
    ForeignSupply { after_supply: bool },
}

impl Injection {
    fn expected(self) -> Violation {
        match self {
            Self::UnansweredQuery => Violation::UnansweredQuery,
            Self::UnaskedReply => Violation::UnaskedReply,
            Self::Match { .. } => Violation::UnexpectedMatch,
            Self::Query { .. } => Violation::UnexpectedQuery,
            Self::DuplicateSupply | Self::ForeignSupply { .. } => Violation::InvalidSupply,
        }
    }

    /// The replies answering the one request, every supply carrying the
    /// contained `version`.
    fn script(self, version: Version) -> Vec<Reply<Erased>> {
        let requested = || supply(REQUESTED, version.clone());
        let one = |after_supply: bool, reaction: Reaction<Erased>| {
            let mut replies = Vec::new();
            if after_supply {
                replies.push(requested());
            }
            replies.push(reaction);
            vec![Reply { replies }]
        };
        match self {
            Self::UnansweredQuery => Vec::new(),
            Self::UnaskedReply => vec![
                Reply {
                    replies: vec![requested()],
                },
                Reply {
                    replies: Vec::new(),
                },
            ],
            Self::Match { after_supply } => one(after_supply, Reaction::Match),
            Self::Query { after_supply } => one(after_supply, Reaction::Query(Vec::new())),
            Self::DuplicateSupply => one(true, requested()),
            Self::ForeignSupply { after_supply } => {
                one(after_supply, supply(REQUESTED + 1, version.clone()))
            }
        }
    }
}

fn arb_injection() -> impl Strategy<Value = Injection> {
    prop_oneof![
        Just(Injection::UnansweredQuery),
        Just(Injection::UnaskedReply),
        any::<bool>().prop_map(|after_supply| Injection::Match { after_supply }),
        any::<bool>().prop_map(|after_supply| Injection::Query { after_supply }),
        Just(Injection::DuplicateSupply),
        any::<bool>().prop_map(|after_supply| Injection::ForeignSupply { after_supply }),
    ]
}

proptest! {
    /// Every malformed closing-leg reply is reported as its exact public
    /// `Violation`, under arbitrary channel poll order.
    ///
    /// The closing leg classifies through the resolver every descending
    /// stage uses; its one rule of its own is that a leaf request names a
    /// single radix, so a supply at any other radix is out of order. Only
    /// the reply that trails an accepted one passes anything up: every
    /// other rejection precedes the pass-up.
    #[test]
    fn terminal_absorb_reports_exact_violation(
        injection in arb_injection(),
        schedule in proptest::collection::vec(0u8..=2, 0..=64),
    ) {
        let declared = ticked(0);
        let (result, returned) = with_schedule(schedule, || {
            absorb_scripted(
                declared.clone(),
                SupplyLedger::new(u64::MAX),
                injection.script(declared.clone()),
            )
        });
        let expected = injection.expected();
        prop_assert!(
            matches!(result, Err(Error::Violation(actual)) if actual == expected),
            "{injection:?} reported {result:?}, expected {expected:?}",
        );
        prop_assert_eq!(
            returned.is_some(),
            injection == Injection::UnaskedReply,
            "only the accepted reply ahead of an unasked one is passed up",
        );
    }
}
