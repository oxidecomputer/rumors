//! Semantic-violation injection across every materialized walk height.

use std::{collections::BTreeSet, convert::Infallible};

use futures::stream::{self, StreamExt};
use proptest::prelude::*;

use super::leaf;
use crate::tree::mirror::streaming::stats::Recorder;
use crate::{
    Version,
    tree::mirror::streaming::{
        Backend, Local,
        materialized::{
            Error, Query, SupplyLedger, Violation, Work,
            channel::{Receiver, with_schedule},
            work::queues::internal_child_queries,
        },
        message::{Reaction, Reply},
        protocol::BoxResponses,
        window::Window,
    },
    tree::typed::{
        self, Hash, Path, Prefix,
        height::{Height, S, UnderRoot, Z},
    },
};

/// The in-memory backend's erased node representation, which the walk's
/// query queues carry.
type Erased = <Local as Backend>::Erased;
/// One deliberately malformed counterparty script and the exact violation it
/// must surface.
#[derive(Clone, Copy, Debug)]
enum Injection {
    UnaskedReply,
    UnansweredQuery,
    UnfinishedReply,
    UnexpectedMatch,
    UnexpectedQuery,
    UnexpectedSupply,
    InvalidSupply,
    UncontainedSupply,
}

impl Injection {
    fn expected(self) -> Violation {
        match self {
            Self::UnaskedReply => Violation::UnaskedReply,
            Self::UnansweredQuery => Violation::UnansweredQuery,
            Self::UnfinishedReply => Violation::UnfinishedReply,
            Self::UnexpectedMatch => Violation::UnexpectedMatch,
            Self::UnexpectedQuery => Violation::UnexpectedQuery,
            Self::UnexpectedSupply => Violation::UnexpectedSupply,
            Self::InvalidSupply => Violation::InvalidSupply,
            Self::UncontainedSupply => Violation::UncontainedSupply,
        }
    }
}

fn arb_injection() -> impl Strategy<Value = Injection> {
    prop_oneof![
        Just(Injection::UnaskedReply),
        Just(Injection::UnansweredQuery),
        Just(Injection::UnfinishedReply),
        Just(Injection::UnexpectedMatch),
        Just(Injection::UnexpectedQuery),
        Just(Injection::UnexpectedSupply),
        Just(Injection::InvalidSupply),
        Just(Injection::UncontainedSupply),
    ]
}

/// Build a node at any traversal height from one path-compressed leaf.
trait TestHeight: Height + Sized {
    fn node(version: &mut Version) -> typed::Node<Self>;
}

impl TestHeight for Z {
    fn node(version: &mut Version) -> typed::Node<Self> {
        leaf(version)
    }
}

impl<H: TestHeight> TestHeight for S<H>
where
    S<H>: Height,
{
    fn node(version: &mut Version) -> typed::Node<Self> {
        typed::Node::beneath(H::node(version), 0)
    }
}

/// Build one malformed reply script at query height `H`, with the version
/// the scripted counterparty is taken to have declared.
///
/// The declared version is snapshotted after every honest node is built,
/// so containment never preempts the structural fault under injection; the
/// `UncontainedSupply` script alone ticks past the snapshot.
#[allow(clippy::type_complexity)]
fn violation_script<H>(
    injection: Injection,
    parent: u8,
    radixes: &BTreeSet<u8>,
) -> (Option<Query<Erased>>, Vec<Reply<Local, H>>, Version)
where
    H: TestHeight,
    S<H>: Height,
{
    let mut version = Version::new();
    let ours = radixes
        .iter()
        .map(|&radix| (radix, H::node(&mut version)))
        .collect::<Vec<_>>();
    let supplied = H::node(&mut version);
    let declared = version.clone();
    let escaped = H::node(&mut version);
    let mut path = [0; 32];
    path[0] = parent;
    let prefix = Prefix::<S<H>>::containing(&Path::from(path));
    let query = Query {
        prefix: prefix.erase(),
        ours: ours
            .iter()
            .map(|(radix, node)| (*radix, <Local as Backend>::erase(node.clone())))
            .collect(),
    };

    let matches = || {
        std::iter::repeat_with(|| Reaction::Match)
            .take(ours.len())
            .collect::<Vec<_>>()
    };
    let (query, replies) = match injection {
        Injection::UnaskedReply => (
            None,
            vec![Reply {
                replies: Vec::new(),
            }],
        ),
        Injection::UnansweredQuery => (Some(query), Vec::new()),
        Injection::UnfinishedReply => (
            Some(query),
            vec![Reply {
                replies: std::iter::repeat_with(|| Reaction::Match)
                    .take(ours.len() - 1)
                    .collect(),
            }],
        ),
        Injection::UnexpectedMatch => {
            let mut replies = matches();
            replies.push(Reaction::Match);
            (Some(query), vec![Reply { replies }])
        }
        Injection::UnexpectedQuery => {
            let mut replies = matches();
            replies.push(Reaction::Query(Vec::new()));
            (Some(query), vec![Reply { replies }])
        }
        Injection::UnexpectedSupply => (
            Some(query),
            vec![Reply {
                replies: vec![Reaction::Supply(
                    *radixes.first().expect("the strategy produces a child"),
                    supplied,
                )],
            }],
        ),
        Injection::InvalidSupply => {
            let radix = *radixes.first().expect("the strategy produces a child");
            (
                Some(Query {
                    prefix: prefix.erase(),
                    ours: Vec::new(),
                }),
                vec![Reply {
                    replies: vec![
                        Reaction::Supply(radix, supplied.clone()),
                        Reaction::Supply(radix, supplied),
                    ],
                }],
            )
        }
        Injection::UncontainedSupply => {
            // Structurally a legal supply — the query holds nothing, the
            // radix is fresh — so only the version escape is at fault.
            let radix = *radixes.first().expect("the strategy produces a child");
            (
                Some(Query {
                    prefix: prefix.erase(),
                    ours: Vec::new(),
                }),
                vec![Reply {
                    replies: vec![Reaction::Supply(radix, escaped)],
                }],
            )
        }
    };
    (query, replies, declared)
}

/// Put the script's optional outstanding query into the walk's pairing
/// queue, labeled at the script's height.
fn query_receiver<H>(query: Option<Query<Erased>>) -> Receiver<Query<Erased>>
where
    H: Height,
    S<H>: Height,
{
    let (queries, queries_rx) = internal_child_queries::<Local>(H::HEIGHT, 1);
    if let Some(query) = query {
        pollster::block_on(queries.send(query)).expect("the walk is live");
    }
    drop(queries);
    queries_rx
}

/// Drive a walk's response pump until it surfaces the injected violation.
///
/// Drain successful replies so backpressure cannot prevent the walk from
/// reaching the fault. The executor must report it even if replies remain buffered.
fn reported_violation<H: Height>(
    work: Work<Local>,
    mut responses: BoxResponses<Local, H, Error<Infallible>>,
) -> Violation {
    let finish = async move {
        while let Some(response) = responses.next().await {
            response?;
        }
        std::future::pending::<Result<(), Error<Infallible>>>().await
    };
    let error = crate::testing::run_to_quiescence(work.execute(Box::pin(finish)))
        .expect("a detected violation must terminate the work")
        .expect_err("the malformed reply must fail");
    match error {
        Error::Violation(violation) => violation,
        Error::Backend(error) => match error {},
    }
}

/// Inject a malformed script through the walk assigned to this query height.
trait InjectHeight: TestHeight {
    /// Run the malformed script through the walk at this height.
    fn inject(injection: Injection, parent: u8, radixes: &BTreeSet<u8>) -> Violation;
}

/// Exercise the terminal leaf walk.
impl InjectHeight for Z {
    /// Inject the script into leaf reconciliation and collect its failure.
    fn inject(injection: Injection, parent: u8, radixes: &BTreeSet<u8>) -> Violation {
        let (query, requests, declared) = violation_script::<Self>(injection, parent, radixes);
        let queries = query_receiver::<Self>(query);
        let mut work = Work::new(Local, Window::FLOOR, Recorder::default());
        let (responses, _resolutions) = work.leaf_level(
            declared,
            SupplyLedger::new(u64::MAX),
            stream::iter(requests),
            queries,
        );
        reported_violation(work, responses)
    }
}

/// Exercise the walk which opens leaf requests.
impl InjectHeight for S<Z> {
    /// Inject the script into leaf-parent reconciliation and collect its failure.
    fn inject(injection: Injection, parent: u8, radixes: &BTreeSet<u8>) -> Violation {
        let (query, requests, declared) = violation_script::<Self>(injection, parent, radixes);
        let queries = query_receiver::<Self>(query);
        let mut work = Work::new(Local, Window::FLOOR, Recorder::default());
        let (responses, _asked, _upper, _lower) = work.leaf_parent_level(
            declared,
            SupplyLedger::new(u64::MAX),
            stream::iter(requests),
            queries,
        );
        reported_violation(work, responses)
    }
}

/// Exercise an interior walk above the leaf-parent stage.
impl<H> InjectHeight for S<S<H>>
where
    H: TestHeight,
    S<H>: Height,
    S<S<H>>: TestHeight,
    S<S<S<H>>>: Height,
{
    /// Inject the script into interior reconciliation and collect its failure.
    fn inject(injection: Injection, parent: u8, radixes: &BTreeSet<u8>) -> Violation {
        let (query, requests, declared) = violation_script::<Self>(injection, parent, radixes);
        let queries = query_receiver::<Self>(query);
        let mut work = Work::new(Local, Window::FLOOR, Recorder::default());
        let (responses, _asked, _upper, _lower) = work.internal_level::<H>(
            declared,
            SupplyLedger::new(u64::MAX),
            None,
            None,
            stream::iter(requests),
            queries,
        );
        reported_violation(work, responses)
    }
}

/// Recurse from a runtime height to its type-level materialized walk.
macro_rules! dispatch_injection_height {
    ($height:expr, $injection:expr, $parent:expr, $radixes:expr; $type:ty, $number:expr; _ $($rest:tt)*) => {
        if $height == $number {
            <$type as InjectHeight>::inject($injection, $parent, $radixes)
        } else {
            dispatch_injection_height!(
                $height, $injection, $parent, $radixes;
                S<$type>, $number + 1;
                $($rest)*
            )
        }
    };
    ($height:expr, $injection:expr, $parent:expr, $radixes:expr; $type:ty, $number:expr;) => {
        panic!("query height {} is outside the traversal", $height)
    };
}

/// Dispatch a runtime query height to its type-level materialized walk.
macro_rules! inject_at_height {
    ($height:expr, $injection:expr, $parent:expr, $radixes:expr) => {
        // Query heights 0..=31: the leaves through the children of the root.
        dispatch_injection_height!($height, $injection, $parent, $radixes; Z, 0;
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _ _
        )
    };
}

proptest! {
    /// Every injected semantic fault is reported as its exact public
    /// `Violation`.
    ///
    /// Every generated case runs at all 32 query heights; arbitrary scope,
    /// held-child shape, and channel poll order pin the counterparty-fault
    /// taxonomy through every materialized walk's response pump.
    #[test]
    fn injected_fault_reports_exact_violation(
        injection in arb_injection(),
        parent in any::<u8>(),
        radixes in proptest::collection::btree_set(any::<u8>(), 1..=8),
        schedule in proptest::collection::vec(0u8..=2, 0..=64),
    ) {
        let expected = injection.expected();
        for height in 0..32 {
            let actual = with_schedule(schedule.clone(), || {
                inject_at_height!(height, injection, parent, &radixes)
            });
            prop_assert_eq!(actual, expected, "query height {}", height);
        }
    }

    /// Every malformed opening reply is reported as its exact public
    /// `Violation`.
    ///
    /// The responder's opening leg reads the root question and the
    /// initiator's early supplies with arms of its own: the opening reply
    /// is not paired positionally against the held fan, so the resolver
    /// cannot classify it. An arbitrary held fan and channel poll order
    /// pin those arms to the taxonomy.
    #[test]
    fn opening_fault_reports_exact_violation(
        injection in arb_opening_injection(),
        radixes in proptest::collection::btree_set(any::<u8>(), 1..=8),
        schedule in proptest::collection::vec(0u8..=2, 0..=64),
    ) {
        let expected = injection.expected();
        let actual = with_schedule(schedule, || opening_violation(injection, &radixes));
        prop_assert_eq!(actual, expected, "{:?}", injection);
    }
}

/// One deliberately malformed opening reply and the exact violation it
/// must surface.
///
/// The opening reply is the root question followed by the initiator's
/// early supplies, in ascending radix order and at radices the responder
/// does not hold; each shape below breaks one clause of that grammar.
#[derive(Clone, Copy, Debug)]
enum OpeningInjection {
    /// The reply stream ends before the opening reply.
    UnansweredQuery,
    /// A second reply follows the opening.
    UnaskedReply,
    /// An empty opening reply: no root question.
    UnfinishedReply,
    /// A `Match`, ahead of the root question or after it.
    Match { trailing: bool },
    /// A supply ahead of the root question, at a radix this side holds or
    /// at a fresh one.
    LeadingSupply { held: bool },
    /// A second `Query` after the root question.
    TrailingQuery,
    /// Two early supplies in descending radix order.
    DescendingSupplies,
    /// The same early radix supplied twice.
    DuplicateSupply,
    /// An early supply at a radix this side holds.
    HeldSupply,
    /// An early supply whose version escapes the declared version.
    UncontainedSupply,
    /// An early supply past the declared set length.
    OverdrawnSupply,
}

impl OpeningInjection {
    fn expected(self) -> Violation {
        match self {
            Self::UnansweredQuery => Violation::UnansweredQuery,
            Self::UnaskedReply => Violation::UnaskedReply,
            Self::UnfinishedReply => Violation::UnfinishedReply,
            Self::Match { .. } => Violation::UnexpectedMatch,
            Self::LeadingSupply { held: true } | Self::HeldSupply => Violation::UnexpectedSupply,
            Self::LeadingSupply { held: false }
            | Self::DescendingSupplies
            | Self::DuplicateSupply => Violation::InvalidSupply,
            Self::TrailingQuery => Violation::UnexpectedQuery,
            Self::UncontainedSupply => Violation::UncontainedSupply,
            Self::OverdrawnSupply => Violation::OverdrawnSupply,
        }
    }

    /// The declared set length the script runs under: spent only where
    /// the overrun is the fault.
    fn ledger(self) -> SupplyLedger {
        match self {
            Self::OverdrawnSupply => SupplyLedger::new(0),
            _ => SupplyLedger::new(u64::MAX),
        }
    }
}

fn arb_opening_injection() -> impl Strategy<Value = OpeningInjection> {
    prop_oneof![
        Just(OpeningInjection::UnansweredQuery),
        Just(OpeningInjection::UnaskedReply),
        Just(OpeningInjection::UnfinishedReply),
        any::<bool>().prop_map(|trailing| OpeningInjection::Match { trailing }),
        any::<bool>().prop_map(|held| OpeningInjection::LeadingSupply { held }),
        Just(OpeningInjection::TrailingQuery),
        Just(OpeningInjection::DescendingSupplies),
        Just(OpeningInjection::DuplicateSupply),
        Just(OpeningInjection::HeldSupply),
        Just(OpeningInjection::UncontainedSupply),
        Just(OpeningInjection::OverdrawnSupply),
    ]
}

/// Inject one malformed opening reply through the responder's opening leg,
/// whose root fan holds one child per radix in `radixes`.
///
/// The root question matches the held fan exactly, so a script whose
/// fault lies past the question is answered without a dispute. The
/// declared version is snapshotted after every contained node is built;
/// the `UncontainedSupply` script alone ticks past the snapshot.
fn opening_violation(injection: OpeningInjection, radixes: &BTreeSet<u8>) -> Violation {
    let mut version = Version::new();
    let ours = radixes
        .iter()
        .map(|&radix| (radix, UnderRoot::node(&mut version)))
        .collect::<Vec<_>>();
    let theirs = ours
        .iter()
        .map(|(radix, node)| (*radix, node.hash()))
        .collect::<Vec<(u8, Hash)>>();
    let fan = ours
        .into_iter()
        .map(|(radix, node)| (radix, <Local as Backend>::erase(node)))
        .collect::<Vec<(u8, Erased)>>();
    let contained = [UnderRoot::node(&mut version), UnderRoot::node(&mut version)];
    let declared = version.clone();
    let escaped = UnderRoot::node(&mut version);

    let held = *radixes.first().expect("the strategy produces a child");
    let mut fresh = (0..=u8::MAX).filter(|radix| !radixes.contains(radix));
    let low = fresh.next().expect("at most eight radices are held");
    let high = fresh.next().expect("at most eight radices are held");
    let question = || Reaction::Query(theirs.clone());
    let supply = |radix: u8, index: usize| Reaction::Supply(radix, contained[index].clone());
    let one = |replies: Vec<Reaction<Local, UnderRoot>>| vec![Reply { replies }];
    let requests = match injection {
        OpeningInjection::UnansweredQuery => Vec::new(),
        OpeningInjection::UnaskedReply => vec![
            Reply {
                replies: vec![question()],
            },
            Reply {
                replies: Vec::new(),
            },
        ],
        OpeningInjection::UnfinishedReply => one(Vec::new()),
        OpeningInjection::Match { trailing: false } => one(vec![Reaction::Match]),
        OpeningInjection::Match { trailing: true } => one(vec![question(), Reaction::Match]),
        OpeningInjection::LeadingSupply { held: true } => one(vec![supply(held, 0), question()]),
        OpeningInjection::LeadingSupply { held: false } => one(vec![supply(low, 0), question()]),
        OpeningInjection::TrailingQuery => one(vec![question(), Reaction::Query(Vec::new())]),
        OpeningInjection::DescendingSupplies => {
            one(vec![question(), supply(high, 0), supply(low, 1)])
        }
        OpeningInjection::DuplicateSupply => one(vec![question(), supply(low, 0), supply(low, 1)]),
        OpeningInjection::HeldSupply => one(vec![question(), supply(held, 0)]),
        OpeningInjection::UncontainedSupply => {
            one(vec![question(), Reaction::Supply(low, escaped)])
        }
        OpeningInjection::OverdrawnSupply => one(vec![question(), supply(low, 0)]),
    };

    let mut work = Work::new(Local, Window::FLOOR, Recorder::default());
    // The channels the leg publishes into stay open for the run: a closed
    // one ends the leg quietly instead of reporting.
    let (responses, _asked, _returns, _early, _finish) = work.responder_level(
        declared.clone(),
        injection.ledger(),
        declared,
        fan,
        stream::iter(requests),
    );
    reported_violation(work, responses)
}
