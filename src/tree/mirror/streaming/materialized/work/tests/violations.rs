//! Semantic-violation injection across every materialized walk height.

use std::{collections::BTreeSet, convert::Infallible};

use futures::stream::{self, StreamExt};
use proptest::prelude::*;

use super::leaf;
use crate::{
    Version,
    tree::mirror::streaming::{
        Local,
        materialized::{
            Error, Query, Violation, Work,
            channel::{Receiver, with_schedule},
            unknown::Unknown,
            work::queues::internal_child_queries,
        },
        message::{Reaction, Reply},
        protocol::BoxResponses,
    },
    tree::typed::{
        self, Path, Prefix,
        height::{Height, S, Z},
    },
};
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
    ]
}

/// Build a node at any traversal height from one path-compressed leaf.
trait TestHeight: Height + Sized {
    fn node(version: &mut Version) -> typed::Node<(), Self>;
}

impl TestHeight for Z {
    fn node(version: &mut Version) -> typed::Node<(), Self> {
        leaf(version)
    }
}

impl<H: TestHeight> TestHeight for S<H>
where
    S<H>: Height,
{
    fn node(version: &mut Version) -> typed::Node<(), Self> {
        typed::Node::beneath(H::node(version), 0)
    }
}

/// Build one malformed reply script at query height `H`.
fn violation_script<H>(
    injection: Injection,
    parent: u8,
    radixes: &BTreeSet<u8>,
) -> (Option<Query<Local, (), H>>, Vec<Reply<Local, (), H>>)
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
    let mut path = [0; 32];
    path[0] = parent;
    let prefix = Prefix::<S<H>>::containing(&Path::from(path));
    let query = Query {
        prefix,
        ours: ours.clone(),
    };

    let matches = || {
        std::iter::repeat_with(|| Reaction::Match)
            .take(ours.len())
            .collect::<Vec<_>>()
    };
    match injection {
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
                    prefix,
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
    }
}

/// Put the script's optional outstanding query into the walk's pairing queue.
fn query_receiver<H>(query: Option<Query<Local, (), H>>) -> Receiver<Query<Local, (), H>>
where
    H: Height,
    S<H>: Height,
{
    let (queries, queries_rx) = internal_child_queries::<Local, (), H>();
    if let Some(query) = query {
        pollster::block_on(queries.send(query)).expect("the walk is live");
    }
    drop(queries);
    queries_rx
}

/// Drive a walk's response pump until it surfaces the injected violation.
fn reported_violation<H: Height>(
    work: Work<Local, ()>,
    mut responses: BoxResponses<Local, (), H, Error<Infallible>>,
) -> Violation {
    let response = pollster::block_on(async move {
        let drive = work.execute(Box::pin(std::future::pending::<
            Result<(), Error<Infallible>>,
        >()));
        tokio::pin!(drive);
        tokio::select! {
            response = responses.next() => response,
            result = &mut drive => panic!("the pending driver unexpectedly completed: {result:?}"),
        }
    });

    match response {
        Some(Err(Error::Violation(violation))) => violation,
        Some(Err(Error::Backend(error))) => match error {},
        Some(Ok(_)) => panic!("the malformed reply produced a successful response"),
        None => panic!("the malformed reply ended without a violation"),
    }
}

/// Inject a malformed script through the walk assigned to this query height.
trait InjectHeight: TestHeight {
    fn inject(injection: Injection, parent: u8, radixes: &BTreeSet<u8>) -> Violation;
}

impl InjectHeight for Z {
    fn inject(injection: Injection, parent: u8, radixes: &BTreeSet<u8>) -> Violation {
        let (query, requests) = violation_script::<Self>(injection, parent, radixes);
        let queries = query_receiver(query);
        let mut work = Work::new(Local);
        let (responses, _resolutions) =
            work.leaf_level(Version::new(), stream::iter(requests), queries);
        reported_violation(work, responses)
    }
}

impl InjectHeight for S<Z> {
    fn inject(injection: Injection, parent: u8, radixes: &BTreeSet<u8>) -> Violation {
        let (query, requests) = violation_script::<Self>(injection, parent, radixes);
        let queries = query_receiver(query);
        let mut work = Work::new(Local);
        let (responses, _asked, _upper, _lower) =
            work.leaf_parent_level(Version::new(), stream::iter(requests), queries);
        reported_violation(work, responses)
    }
}

impl<H> InjectHeight for S<S<H>>
where
    H: TestHeight + Unknown,
    S<H>: Unknown,
    S<S<H>>: TestHeight + Unknown,
    S<S<S<H>>>: Height,
{
    fn inject(injection: Injection, parent: u8, radixes: &BTreeSet<u8>) -> Violation {
        let (query, requests) = violation_script::<Self>(injection, parent, radixes);
        let queries = query_receiver(query);
        let mut work = Work::new(Local);
        let (responses, _asked, _upper, _lower) =
            work.internal_level::<H>(Version::new(), stream::iter(requests), queries);
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
}
