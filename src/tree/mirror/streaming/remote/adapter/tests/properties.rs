//! Laws which hold uniformly across the adapter's type-level height ladder.

use std::convert::Infallible;

use futures::{StreamExt, TryStreamExt, stream};
use proptest::{collection::btree_set, prelude::*, test_runner::TestCaseResult};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Local, Node,
        convert::Convert,
        message::{Reaction, Reply},
    },
    typed::{
        self, Hash, Path, Prefix,
        height::{Height, S, Z},
    },
};

use super::{
    super::{DecodeError, Scope, decode_leaf_reply, decode_reply, encode_leaf_reply, encode_reply},
    LeafCase, hash, runtime,
};
use crate::tree::mirror::streaming::remote::codec::{End, Flow, Frame, Reaction as WireReaction};

#[derive(Clone, Debug)]
struct PositionalCase {
    radixes: Vec<u8>,
    queries: u64,
    nested: Vec<(u8, Hash)>,
}

impl PositionalCase {
    fn is_query(&self, position: usize) -> bool {
        self.queries & (1 << position) != 0
    }

    fn listing(&self) -> Vec<(u8, Hash)> {
        self.radixes
            .iter()
            .map(|&radix| (radix, hash(radix)))
            .collect()
    }

    fn with_supply(&self, supply_radix: u8) -> (Self, usize) {
        let mut case = self.clone();
        case.radixes.retain(|radix| *radix != supply_radix);
        let supply_at = case.radixes.partition_point(|radix| *radix < supply_radix);
        (case, supply_at)
    }
}

/// Exercise an adapter law at one concrete type-level reply height.
trait AdapterHeight: Convert {
    fn node(leaf: &LeafCase) -> typed::Node<u64, Self>;

    fn supplied_leaf_is_lossless(
        leaf: &LeafCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult;

    fn matches_are_lossless(
        leaf: &LeafCase,
        radixes: &[u8],
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult;

    fn positioned_reactions_are_lossless(
        leaf: &LeafCase,
        case: &PositionalCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult;

    fn mixed_reactions_are_lossless(
        leaf: &LeafCase,
        case: &PositionalCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult;

    fn duplicate_leaf_is_rejected(
        leaf: &LeafCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult;

    fn foreign_leaf_is_rejected(
        leaf: &LeafCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult;
}

impl AdapterHeight for Z {
    fn node(leaf: &LeafCase) -> typed::Node<u64, Self> {
        <typed::Node<u64, Z> as Leaf<u64>>::leaf(leaf.version.clone(), leaf.message.clone())
    }

    fn supplied_leaf_is_lossless(
        leaf: &LeafCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let (parent, radix) = Prefix::<Z>::containing(&leaf.path()).pop();
        let scope = Scope::new(parent, &[]);
        let frame = supplied_frame(leaf, Flow::End(end));
        let mut frames = stream::iter([frame.clone()]);
        let decoded = runtime
            .block_on(decode_leaf_reply(Local, scope.clone(), &mut frames))
            .expect("an in-scope leaf decodes");

        prop_assert!(decoded.questions.is_empty(), "height 0");
        assert_decoded_supply::<Z>(&decoded.reply, radix, leaf, runtime)?;
        prop_assert_eq!(decoded.end, end, "height 0");
        let reencoded = runtime.block_on(async {
            encode_leaf_reply(Local, scope, decoded.reply, end)
                .map_ok(|encoded| encoded.into_parts().0)
                .try_collect::<Vec<_>>()
                .await
                .expect("the local backend is infallible")
        });
        prop_assert_eq!(reencoded, vec![frame], "height 0");
        Ok(())
    }

    fn matches_are_lossless(
        leaf: &LeafCase,
        radixes: &[u8],
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let parent = Prefix::<S<Z>>::containing(&leaf.path());
        let scope = Scope::new(parent, &listing(radixes));
        let reply = Reply::<Local, (), Z> {
            replies: radixes.iter().map(|_| Reaction::Match).collect(),
        };
        let encoded = runtime.block_on(async {
            encode_leaf_reply(Local, scope.clone(), reply, end)
                .map_ok(|encoded| encoded.into_parts())
                .try_collect::<Vec<_>>()
                .await
                .expect("the local backend is infallible")
        });
        assert_match_encoding(&encoded, radixes.len(), end, 0)?;

        let sentinel = Frame::End(opposite(end));
        let mut frames = stream::iter(
            encoded
                .into_iter()
                .map(|(frame, _)| frame)
                .chain([sentinel.clone()]),
        );
        let decoded = runtime
            .block_on(decode_leaf_reply(Local, scope, &mut frames))
            .expect("canonical matches decode");
        prop_assert!(decoded.questions.is_empty(), "height 0");
        assert_matches(&decoded.reply, radixes.len(), 0)?;
        prop_assert_eq!(decoded.end, end, "height 0");
        prop_assert_eq!(runtime.block_on(frames.next()), Some(sentinel), "height 0");
        Ok(())
    }

    fn positioned_reactions_are_lossless(
        leaf: &LeafCase,
        case: &PositionalCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let parent = Prefix::<S<Z>>::containing(&leaf.path());
        let scope = Scope::new(parent, &case.listing());
        let mut leaf_case = case.clone();
        leaf_case.nested.clear();
        let reply = Reply::<Local, (), Z> {
            replies: (0..leaf_case.radixes.len())
                .map(|position| {
                    if leaf_case.is_query(position) {
                        Reaction::Query(Vec::new())
                    } else {
                        Reaction::Match
                    }
                })
                .collect(),
        };
        let expected_frames = expected_positional_frames(&leaf_case, end);
        let expected_questions = leaf_case
            .radixes
            .iter()
            .enumerate()
            .filter(|(position, _)| leaf_case.is_query(*position))
            .map(|(_, &radix)| Scope::leaf(parent.push(radix)))
            .collect::<Vec<_>>();
        let expected_publications = if leaf_case.radixes.is_empty() {
            vec![None]
        } else {
            leaf_case
                .radixes
                .iter()
                .enumerate()
                .map(|(position, &radix)| {
                    leaf_case
                        .is_query(position)
                        .then(|| Scope::leaf(parent.push(radix)))
                })
                .collect::<Vec<_>>()
        };

        let encoded = runtime.block_on(async {
            encode_leaf_reply(Local, scope.clone(), reply, end)
                .map_ok(|encoded| encoded.into_parts())
                .try_collect::<Vec<_>>()
                .await
                .expect("canonical leaf reactions encode")
        });
        let actual_frames = encoded
            .iter()
            .map(|(frame, _)| frame.clone())
            .collect::<Vec<_>>();
        let publications = encoded
            .into_iter()
            .map(|(_, question)| question)
            .collect::<Vec<_>>();
        prop_assert_eq!(&actual_frames, &expected_frames, "height 0");
        prop_assert_eq!(&publications, &expected_publications, "height 0");

        let mut frames = stream::iter(actual_frames);
        let decoded = runtime
            .block_on(decode_leaf_reply(Local, scope, &mut frames))
            .expect("canonical leaf reactions decode");
        prop_assert_eq!(decoded.end, end, "height 0");
        prop_assert_eq!(&decoded.questions, &expected_questions, "height 0");
        assert_positional_reply(&decoded.reply, &leaf_case, 0)
    }

    fn mixed_reactions_are_lossless(
        leaf: &LeafCase,
        case: &PositionalCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let (parent, supply_radix) = Prefix::<Z>::containing(&leaf.path()).pop();
        let (case, supply_at) = case.with_supply(supply_radix);
        let scope = Scope::new(parent, &case.listing());
        let reply = mixed_reply(&case, &[], supply_at, supply_radix, Self::node(leaf));
        let expected_frames = expected_mixed_frames(&case, &[], supply_at, leaf, end);
        let expected_publications =
            mixed_publications(&case, supply_at, |radix| Scope::leaf(parent.push(radix)));
        let expected_questions = expected_publications
            .iter()
            .filter_map(Clone::clone)
            .collect::<Vec<_>>();

        let encoded = runtime.block_on(async {
            encode_leaf_reply(Local, scope.clone(), reply, end)
                .map_ok(|encoded| encoded.into_parts())
                .try_collect::<Vec<_>>()
                .await
                .expect("canonical mixed leaf reactions encode")
        });
        let actual_frames = encoded
            .iter()
            .map(|(frame, _)| frame.clone())
            .collect::<Vec<_>>();
        let publications = encoded
            .into_iter()
            .map(|(_, question)| question)
            .collect::<Vec<_>>();
        prop_assert_eq!(&actual_frames, &expected_frames, "height 0");
        prop_assert_eq!(&publications, &expected_publications, "height 0");

        let sentinel = Frame::End(opposite(end));
        let mut frames = stream::iter(actual_frames.into_iter().chain([sentinel.clone()]));
        let decoded = runtime
            .block_on(decode_leaf_reply(Local, scope, &mut frames))
            .expect("canonical mixed leaf reactions decode");
        prop_assert_eq!(decoded.end, end, "height 0");
        prop_assert_eq!(&decoded.questions, &expected_questions, "height 0");
        assert_mixed_reply(
            &decoded.reply,
            &case,
            &[],
            supply_at,
            supply_radix,
            leaf,
            runtime,
        )?;
        prop_assert_eq!(runtime.block_on(frames.next()), Some(sentinel), "height 0");
        Ok(())
    }

    fn duplicate_leaf_is_rejected(
        leaf: &LeafCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let (parent, _) = Prefix::<Z>::containing(&leaf.path()).pop();
        let mut frames = stream::iter(duplicate_frames(leaf));
        let error = runtime
            .block_on(decode_leaf_reply(
                Local,
                Scope::new(parent, &[]),
                &mut frames,
            ))
            .err()
            .expect("duplicate leaves are not strictly ascending");
        assert_duplicate_error(error, leaf, 0)
    }

    fn foreign_leaf_is_rejected(
        leaf: &LeafCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let actual = Prefix::<Z>::containing(&leaf.path()).pop().0;
        let foreign = foreign_parent::<Z>(leaf, actual);
        let mut frames = stream::iter([supplied_frame(leaf, Flow::End(End::Reply))]);
        let error = runtime
            .block_on(decode_leaf_reply(
                Local,
                Scope::new(foreign, &[]),
                &mut frames,
            ))
            .err()
            .expect("a leaf outside the retained scope must fail");
        assert_foreign_error(error, foreign, leaf, 0)
    }
}

impl<H> AdapterHeight for S<H>
where
    H: AdapterHeight + PartialEq,
    S<H>: Convert,
    S<S<H>>: Height,
{
    fn node(leaf: &LeafCase) -> typed::Node<u64, Self> {
        let path: [u8; 32] = leaf.path().into();
        typed::Node::beneath(H::node(leaf), path[31 - H::HEIGHT])
    }

    fn supplied_leaf_is_lossless(
        leaf: &LeafCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let (parent, radix) = Prefix::<S<H>>::containing(&leaf.path()).pop();
        let scope = Scope::new(parent, &[]);
        let frame = supplied_frame(leaf, Flow::End(end));
        let mut frames = stream::iter([frame.clone()]);
        let decoded = runtime
            .block_on(decode_reply::<Local, u64, H, _>(
                Local,
                scope.clone(),
                &mut frames,
            ))
            .expect("an in-scope leaf decodes");

        prop_assert!(decoded.questions.is_empty(), "height {}", Self::HEIGHT);
        assert_decoded_supply::<Self>(&decoded.reply, radix, leaf, runtime)?;
        prop_assert_eq!(decoded.end, end, "height {}", Self::HEIGHT);
        let reencoded = runtime.block_on(async {
            encode_reply(Local, scope, decoded.reply, end)
                .map_ok(|encoded| encoded.into_parts().0)
                .try_collect::<Vec<_>>()
                .await
                .expect("the local backend is infallible")
        });
        prop_assert_eq!(reencoded, vec![frame], "height {}", Self::HEIGHT);
        Ok(())
    }

    fn matches_are_lossless(
        leaf: &LeafCase,
        radixes: &[u8],
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let parent = Prefix::<S<S<H>>>::containing(&leaf.path());
        let scope = Scope::new(parent, &listing(radixes));
        let reply = Reply::<Local, (), Self> {
            replies: radixes.iter().map(|_| Reaction::Match).collect(),
        };
        let encoded = runtime.block_on(async {
            encode_reply(Local, scope.clone(), reply, end)
                .map_ok(|encoded| encoded.into_parts())
                .try_collect::<Vec<_>>()
                .await
                .expect("the local backend is infallible")
        });
        assert_match_encoding(&encoded, radixes.len(), end, Self::HEIGHT)?;

        let sentinel = Frame::End(opposite(end));
        let mut frames = stream::iter(
            encoded
                .into_iter()
                .map(|(frame, _)| frame)
                .chain([sentinel.clone()]),
        );
        let decoded = runtime
            .block_on(decode_reply::<Local, (), H, _>(Local, scope, &mut frames))
            .expect("canonical matches decode");
        prop_assert!(decoded.questions.is_empty(), "height {}", Self::HEIGHT);
        assert_matches(&decoded.reply, radixes.len(), Self::HEIGHT)?;
        prop_assert_eq!(decoded.end, end, "height {}", Self::HEIGHT);
        prop_assert_eq!(
            runtime.block_on(frames.next()),
            Some(sentinel),
            "height {}",
            Self::HEIGHT
        );
        Ok(())
    }

    fn positioned_reactions_are_lossless(
        leaf: &LeafCase,
        case: &PositionalCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let parent = Prefix::<S<S<H>>>::containing(&leaf.path());
        let scope = Scope::new(parent, &case.listing());
        let reply = Reply::<Local, (), Self> {
            replies: (0..case.radixes.len())
                .map(|position| {
                    if case.is_query(position) {
                        Reaction::Query(case.nested.clone())
                    } else {
                        Reaction::Match
                    }
                })
                .collect(),
        };
        let expected_questions = case
            .radixes
            .iter()
            .enumerate()
            .filter(|(position, _)| case.is_query(*position))
            .map(|(_, &radix)| Scope::new(parent.push(radix), &case.nested))
            .collect::<Vec<Scope<H>>>();
        let expected_frames = expected_positional_frames(case, end);
        let expected_publications = if case.radixes.is_empty() {
            vec![None]
        } else {
            case.radixes
                .iter()
                .enumerate()
                .map(|(position, &radix)| {
                    case.is_query(position)
                        .then(|| Scope::new(parent.push(radix), &case.nested))
                })
                .collect::<Vec<_>>()
        };

        let encoded = runtime.block_on(async {
            encode_reply(Local, scope.clone(), reply, end)
                .map_ok(|encoded| encoded.into_parts())
                .try_collect::<Vec<_>>()
                .await
                .expect("the local backend is infallible")
        });
        let actual_frames = encoded
            .iter()
            .map(|(frame, _)| frame.clone())
            .collect::<Vec<_>>();
        let publications = encoded
            .into_iter()
            .map(|(_, question)| question)
            .collect::<Vec<_>>();
        prop_assert_eq!(&actual_frames, &expected_frames, "height {}", Self::HEIGHT);
        prop_assert_eq!(
            &publications,
            &expected_publications,
            "height {}",
            Self::HEIGHT
        );

        let mut frames = stream::iter(actual_frames);
        let decoded = runtime
            .block_on(decode_reply::<Local, (), H, _>(Local, scope, &mut frames))
            .expect("canonical positional reactions decode");
        prop_assert_eq!(decoded.end, end, "height {}", Self::HEIGHT);
        prop_assert_eq!(
            &decoded.questions,
            &expected_questions,
            "height {}",
            Self::HEIGHT
        );
        assert_positional_reply(&decoded.reply, case, Self::HEIGHT)
    }

    fn mixed_reactions_are_lossless(
        leaf: &LeafCase,
        case: &PositionalCase,
        end: End,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let (parent, supply_radix) = Prefix::<Self>::containing(&leaf.path()).pop();
        let (case, supply_at) = case.with_supply(supply_radix);
        let scope = Scope::new(parent, &case.listing());
        let reply = mixed_reply(
            &case,
            &case.nested,
            supply_at,
            supply_radix,
            Self::node(leaf),
        );
        let expected_frames = expected_mixed_frames(&case, &case.nested, supply_at, leaf, end);
        let expected_publications = mixed_publications(&case, supply_at, |radix| {
            Scope::new(parent.push(radix), &case.nested)
        });
        let expected_questions = expected_publications
            .iter()
            .filter_map(Clone::clone)
            .collect::<Vec<_>>();

        let encoded = runtime.block_on(async {
            encode_reply(Local, scope.clone(), reply, end)
                .map_ok(|encoded| encoded.into_parts())
                .try_collect::<Vec<_>>()
                .await
                .expect("canonical mixed reactions encode")
        });
        let actual_frames = encoded
            .iter()
            .map(|(frame, _)| frame.clone())
            .collect::<Vec<_>>();
        let publications = encoded
            .into_iter()
            .map(|(_, question)| question)
            .collect::<Vec<_>>();
        prop_assert_eq!(&actual_frames, &expected_frames, "height {}", Self::HEIGHT);
        prop_assert_eq!(
            &publications,
            &expected_publications,
            "height {}",
            Self::HEIGHT
        );

        let sentinel = Frame::End(opposite(end));
        let mut frames = stream::iter(actual_frames.into_iter().chain([sentinel.clone()]));
        let decoded = runtime
            .block_on(decode_reply::<Local, u64, H, _>(Local, scope, &mut frames))
            .expect("canonical mixed reactions decode");
        prop_assert_eq!(decoded.end, end, "height {}", Self::HEIGHT);
        prop_assert_eq!(
            &decoded.questions,
            &expected_questions,
            "height {}",
            Self::HEIGHT
        );
        assert_mixed_reply(
            &decoded.reply,
            &case,
            &case.nested,
            supply_at,
            supply_radix,
            leaf,
            runtime,
        )?;
        prop_assert_eq!(
            runtime.block_on(frames.next()),
            Some(sentinel),
            "height {}",
            Self::HEIGHT
        );
        Ok(())
    }

    fn duplicate_leaf_is_rejected(
        leaf: &LeafCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let parent = Prefix::<Self>::containing(&leaf.path()).pop().0;
        let mut frames = stream::iter(duplicate_frames(leaf));
        let error = runtime
            .block_on(decode_reply::<Local, u64, H, _>(
                Local,
                Scope::new(parent, &[]),
                &mut frames,
            ))
            .err()
            .expect("duplicate leaves are not strictly ascending");
        assert_duplicate_error(error, leaf, Self::HEIGHT)
    }

    fn foreign_leaf_is_rejected(
        leaf: &LeafCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        // The height-31 reply's parent is the unique root prefix, so there is
        // no alternative scope against which an otherwise valid leaf can be
        // tested.
        if Self::HEIGHT == 31 {
            return Ok(());
        }
        let actual = Prefix::<Self>::containing(&leaf.path()).pop().0;
        let foreign = foreign_parent::<Self>(leaf, actual);
        let mut frames = stream::iter([supplied_frame(leaf, Flow::End(End::Reply))]);
        let error = runtime
            .block_on(decode_reply::<Local, u64, H, _>(
                Local,
                Scope::new(foreign, &[]),
                &mut frames,
            ))
            .err()
            .expect("a leaf outside the retained scope must fail");
        assert_foreign_error(error, foreign, leaf, Self::HEIGHT)
    }
}

fn supplied_frame(leaf: &LeafCase, flow: Flow) -> Frame<u64> {
    Frame::Reaction(
        WireReaction::Supply(leaf.version.clone(), leaf.message.clone()),
        flow,
    )
}

fn duplicate_frames(leaf: &LeafCase) -> [Frame<u64>; 2] {
    [
        supplied_frame(leaf, Flow::Continue),
        supplied_frame(leaf, Flow::End(End::Reply)),
    ]
}

fn listing(radixes: &[u8]) -> Vec<(u8, Hash)> {
    radixes.iter().map(|&radix| (radix, hash(radix))).collect()
}

fn opposite(end: End) -> End {
    match end {
        End::Reply => End::Stream,
        End::Stream => End::Reply,
    }
}

fn mixed_reply<H: Height>(
    case: &PositionalCase,
    query_listing: &[(u8, Hash)],
    supply_at: usize,
    supply_radix: u8,
    supply: typed::Node<u64, H>,
) -> Reply<Local, u64, H> {
    let mut supply = Some(supply);
    let mut replies = Vec::with_capacity(case.radixes.len() + 1);
    for position in 0..=case.radixes.len() {
        if position == supply_at {
            replies.push(Reaction::Supply(
                supply_radix,
                supply.take().expect("the supply has one insertion point"),
            ));
        }
        if position < case.radixes.len() {
            replies.push(if case.is_query(position) {
                Reaction::Query(query_listing.to_vec())
            } else {
                Reaction::Match
            });
        }
    }
    Reply { replies }
}

fn expected_mixed_frames(
    case: &PositionalCase,
    query_listing: &[(u8, Hash)],
    supply_at: usize,
    leaf: &LeafCase,
    end: End,
) -> Vec<Frame<u64>> {
    let mut reactions = Vec::with_capacity(case.radixes.len() + 1);
    for position in 0..=case.radixes.len() {
        if position == supply_at {
            reactions.push(WireReaction::Supply(
                leaf.version.clone(),
                leaf.message.clone(),
            ));
        }
        if position < case.radixes.len() {
            reactions.push(if case.is_query(position) {
                WireReaction::Query(query_listing.to_vec())
            } else {
                WireReaction::Match
            });
        }
    }
    let count = reactions.len();
    reactions
        .into_iter()
        .enumerate()
        .map(|(position, reaction)| {
            let flow = if position + 1 == count {
                Flow::End(end)
            } else {
                Flow::Continue
            };
            Frame::Reaction(reaction, flow)
        })
        .collect()
}

fn mixed_publications<Q>(
    case: &PositionalCase,
    supply_at: usize,
    mut question: impl FnMut(u8) -> Q,
) -> Vec<Option<Q>> {
    let mut publications = Vec::with_capacity(case.radixes.len() + 1);
    for position in 0..=case.radixes.len() {
        if position == supply_at {
            publications.push(None);
        }
        if position < case.radixes.len() {
            publications.push(
                case.is_query(position)
                    .then(|| question(case.radixes[position])),
            );
        }
    }
    publications
}

fn assert_mixed_reply<H: Convert>(
    reply: &Reply<Local, u64, H>,
    case: &PositionalCase,
    query_listing: &[(u8, Hash)],
    supply_at: usize,
    supply_radix: u8,
    leaf: &LeafCase,
    runtime: &tokio::runtime::Runtime,
) -> TestCaseResult
where
    S<H>: Height,
{
    prop_assert_eq!(reply.replies.len(), case.radixes.len() + 1);
    let mut reaction = 0;
    for position in 0..=case.radixes.len() {
        if position == supply_at {
            let Reaction::Supply(radix, node) = &reply.replies[reaction] else {
                return Err(TestCaseError::fail(format!(
                    "height {} lost its supply at reaction {reaction}",
                    H::HEIGHT
                )));
            };
            prop_assert_eq!(*radix, supply_radix, "height {}", H::HEIGHT);
            assert_node_leaf::<H>(node, leaf, runtime)?;
            reaction += 1;
        }
        if position < case.radixes.len() {
            match (case.is_query(position), &reply.replies[reaction]) {
                (false, Reaction::Match) => {}
                (true, Reaction::Query(actual)) => {
                    prop_assert_eq!(actual, query_listing, "height {}", H::HEIGHT);
                }
                _ => {
                    return Err(TestCaseError::fail(format!(
                        "height {} changed positional reaction {position}",
                        H::HEIGHT
                    )));
                }
            }
            reaction += 1;
        }
    }
    Ok(())
}

fn assert_decoded_supply<H: Convert>(
    reply: &Reply<Local, u64, H>,
    expected_radix: u8,
    expected_leaf: &LeafCase,
    runtime: &tokio::runtime::Runtime,
) -> TestCaseResult
where
    S<H>: Height,
{
    let [Reaction::Supply(radix, node)] = reply.replies.as_slice() else {
        return Err(TestCaseError::fail(format!(
            "height {} did not reconstruct exactly one supply",
            H::HEIGHT
        )));
    };
    prop_assert_eq!(*radix, expected_radix, "height {}", H::HEIGHT);
    assert_node_leaf::<H>(node, expected_leaf, runtime)
}

fn assert_node_leaf<H: Convert>(
    node: &typed::Node<u64, H>,
    expected_leaf: &LeafCase,
    runtime: &tokio::runtime::Runtime,
) -> TestCaseResult
where
    S<H>: Height,
{
    let prefix = Prefix::<H>::containing(&expected_leaf.path());
    let leaves = runtime.block_on(async {
        Local
            .leaves(prefix, node.clone())
            .try_collect::<Vec<_>>()
            .await
            .expect("the local backend is infallible")
    });
    prop_assert_eq!(leaves.len(), 1, "height {}", H::HEIGHT);
    let (actual_prefix, actual_leaf) = &leaves[0];
    prop_assert_eq!(
        *actual_prefix,
        Prefix::<Z>::containing(&expected_leaf.path())
    );
    prop_assert_eq!(actual_leaf.ceiling(), &expected_leaf.version);
    prop_assert_eq!(
        actual_leaf.message().as_slice(),
        expected_leaf.message.as_slice()
    );
    Ok(())
}

fn assert_match_encoding<Q>(
    encoded: &[(Frame<()>, Option<Q>)],
    count: usize,
    end: End,
    height: usize,
) -> TestCaseResult {
    let expected = if count == 0 {
        vec![Frame::End(end)]
    } else {
        (0..count)
            .map(|position| {
                let flow = if position + 1 == count {
                    Flow::End(end)
                } else {
                    Flow::Continue
                };
                Frame::Reaction(WireReaction::Match, flow)
            })
            .collect()
    };
    let actual = encoded
        .iter()
        .map(|(frame, _)| frame.clone())
        .collect::<Vec<_>>();
    prop_assert_eq!(actual, expected, "height {}", height);
    prop_assert!(
        encoded.iter().all(|(_, question)| question.is_none()),
        "matches published a question at height {height}"
    );
    Ok(())
}

fn assert_matches<H: Height>(
    reply: &Reply<Local, (), H>,
    count: usize,
    height: usize,
) -> TestCaseResult {
    prop_assert_eq!(reply.replies.len(), count, "height {}", height);
    prop_assert!(
        reply
            .replies
            .iter()
            .all(|reaction| matches!(reaction, Reaction::Match)),
        "a match changed shape at height {height}"
    );
    Ok(())
}

fn expected_positional_frames(case: &PositionalCase, end: End) -> Vec<Frame<()>> {
    if case.radixes.is_empty() {
        return vec![Frame::End(end)];
    }
    (0..case.radixes.len())
        .map(|position| {
            let reaction = if case.is_query(position) {
                WireReaction::Query(case.nested.clone())
            } else {
                WireReaction::Match
            };
            let flow = if position + 1 == case.radixes.len() {
                Flow::End(end)
            } else {
                Flow::Continue
            };
            Frame::Reaction(reaction, flow)
        })
        .collect()
}

fn assert_positional_reply<H: Height>(
    reply: &Reply<Local, (), H>,
    case: &PositionalCase,
    height: usize,
) -> TestCaseResult {
    prop_assert_eq!(reply.replies.len(), case.radixes.len(), "height {}", height);
    for (position, reaction) in reply.replies.iter().enumerate() {
        match (case.is_query(position), reaction) {
            (false, Reaction::Match) => {}
            (true, Reaction::Query(actual)) => {
                prop_assert_eq!(
                    actual,
                    &case.nested,
                    "height {}, position {}",
                    height,
                    position
                );
            }
            _ => {
                return Err(TestCaseError::fail(format!(
                    "reaction changed shape at height {height}, position {position}"
                )));
            }
        }
    }
    Ok(())
}

fn assert_duplicate_error(
    error: DecodeError<Infallible>,
    leaf: &LeafCase,
    height: usize,
) -> TestCaseResult {
    let path: [u8; 32] = leaf.path().into();
    match error {
        DecodeError::LeafOrder { previous, current } => {
            prop_assert_eq!(previous, path, "height {}", height);
            prop_assert_eq!(current, path, "height {}", height);
            Ok(())
        }
        other => Err(TestCaseError::fail(format!(
            "height {height} reported {other:?} instead of LeafOrder"
        ))),
    }
}

fn foreign_parent<H>(leaf: &LeafCase, actual: Prefix<S<H>>) -> Prefix<S<H>>
where
    H: Height,
    S<H>: Height,
{
    (1..=u16::MAX)
        .map(|offset| LeafCase::new(leaf.value.wrapping_add(u64::from(offset)), 0))
        .map(|candidate| Prefix::<S<H>>::containing(&candidate.path()))
        .find(|candidate| *candidate != actual)
        .expect("a non-root prefix has another content-derived value")
}

fn assert_foreign_error<H>(
    error: DecodeError<Infallible>,
    expected: Prefix<S<H>>,
    leaf: &LeafCase,
    height: usize,
) -> TestCaseResult
where
    H: Height,
    S<H>: Height,
{
    let actual: [u8; 32] = leaf.path().into();
    match error {
        DecodeError::LeafOutsideScope {
            expected: reported,
            actual: reported_actual,
        } => {
            prop_assert_eq!(reported, expected.as_bytes(), "height {}", height);
            prop_assert_eq!(reported_actual, actual, "height {}", height);
            Ok(())
        }
        other => Err(TestCaseError::fail(format!(
            "height {height} reported {other:?} instead of LeafOutsideScope"
        ))),
    }
}

fn arb_end() -> impl Strategy<Value = End> {
    prop_oneof![Just(End::Reply), Just(End::Stream)]
}

/// Recurse from a runtime height to its type-level adapter implementation.
macro_rules! dispatch_height {
    ($height:expr, $method:ident($($argument:expr),*); $type:ty, $number:expr; _ $($rest:tt)*) => {
        if $height == $number {
            <$type as AdapterHeight>::$method($($argument),*)
        } else {
            dispatch_height!(
                $height, $method($($argument),*);
                S<$type>, $number + 1;
                $($rest)*
            )
        }
    };
    ($height:expr, $method:ident($($argument:expr),*); $type:ty, $number:expr;) => {
        panic!("reply height {} is outside the adapter", $height)
    };
}

macro_rules! at_height {
    ($height:expr, $method:ident($($argument:expr),*)) => {
        dispatch_height!($height, $method($($argument),*); Z, 0;
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _ _
        )
    };
}

proptest! {
    /// For every reply height, assembling one supplied wire leaf and then
    /// exploding the resulting backend node reproduces the exact frame.
    #[test]
    fn supplied_leaf_is_lossless_at_every_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
        end in arb_end(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, supplied_leaf_is_lossless(&leaf, end, &runtime))?;
        }
    }

    /// At every reply height, positional matches preserve their exact count,
    /// use the unique canonical boundary, publish no question, and consume no
    /// frame from the following reply.
    #[test]
    fn matches_and_boundaries_are_lossless_at_every_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
        radixes in btree_set(any::<u8>(), 0..=8),
        end in arb_end(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let radixes = radixes.into_iter().collect::<Vec<_>>();
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, matches_are_lossless(&leaf, &radixes, end, &runtime))?;
        }
    }

    /// At every height, arbitrary mixtures of matches and queries encode and
    /// decode exactly; leaf queries are empty and publish terminal scopes at
    /// the same height, while higher queries publish lower positional scopes.
    #[test]
    fn positional_reactions_are_lossless_at_every_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
        radixes in btree_set(any::<u8>(), 0..=8),
        queries in any::<u64>(),
        nested in btree_set(any::<u8>(), 0..=8),
        salt in any::<u8>(),
        end in arb_end(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let case = PositionalCase {
            radixes: radixes.into_iter().collect(),
            queries,
            nested: nested
                .into_iter()
                .map(|radix| (radix, hash(radix ^ salt)))
                .collect(),
        };
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, positioned_reactions_are_lossless(
                &leaf, &case, end, &runtime
            ))?;
        }
    }

    /// At every height, a content-derived supply remains correctly keyed when
    /// merge-ordered among arbitrary positional matches and queries.
    #[test]
    fn mixed_reactions_are_lossless_at_every_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
        radixes in btree_set(any::<u8>(), 0..=8),
        queries in any::<u64>(),
        nested in btree_set(any::<u8>(), 0..=8),
        salt in any::<u8>(),
        end in arb_end(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let case = PositionalCase {
            radixes: radixes.into_iter().collect(),
            queries,
            nested: nested
                .into_iter()
                .map(|radix| (radix, hash(radix ^ salt)))
                .collect(),
        };
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, mixed_reactions_are_lossless(
                &leaf, &case, end, &runtime
            ))?;
        }
    }

    /// At every reply height, repeating a supplied leaf is rejected as the
    /// exact strict-order violation, with both offending paths retained.
    #[test]
    fn duplicate_supply_is_rejected_at_every_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, duplicate_leaf_is_rejected(&leaf, &runtime))?;
        }
    }

    /// At every height with more than one possible parent scope, a supplied
    /// leaf is rejected unless its content-derived path is under that scope.
    #[test]
    fn foreign_supply_is_rejected_at_every_scopable_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, foreign_leaf_is_rejected(&leaf, &runtime))?;
        }
    }
}
