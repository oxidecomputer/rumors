//! Laws which hold uniformly across the adapter's type-level height ladder.

use crate::tree::mirror::streaming::message::{Grandchildren, LaterMatches, Listed, listed};
use std::{collections::BTreeSet, convert::Infallible, pin::pin};

use futures::{StreamExt, TryStreamExt, stream};
use proptest::{collection::vec, prelude::*, test_runner::TestCaseResult};

use crate::tree::{
    mirror::streaming::{
        Backend, Local,
        byte_set::ByteSet,
        convert::Convert,
        erased::{Reaction, Reply},
        holding::Holding,
        ledger::Ledger,
    },
    typed::{
        self, Prefix,
        height::{Height, S, Z},
    },
};

/// The in-memory backend's erased node representation.
type Erased = <Local as Backend>::Erased;

use super::{
    super::{
        DecodeError, LeafEdge, Replay, ReplyEncoder, ReplyLevel, Scope, decode_whole_reply,
        derive_questions, encode_leaf_reply, encode_reply,
    },
    LeafCase, NodeAt, codec, hash, leaf_run, nothing_seen, reply_frames, runtime, unbounded,
};
use crate::tree::mirror::streaming::remote::codec::{
    End, Flow, Frame, Reaction as WireReaction, RunBudget,
};

/// A listed scope plus a bitset selecting which positions are queries.
#[derive(Clone, Debug)]
struct PositionalCase {
    radixes: Vec<u8>,
    queries: u64,
    nested: Vec<Listed>,
}

impl PositionalCase {
    /// Whether the reaction at `position` is a query rather than a match.
    fn is_query(&self, position: usize) -> bool {
        self.queries & (1 << position) != 0
    }

    /// Build the parent listing from the selected radixes.
    fn listing(&self) -> Vec<Listed> {
        self.radixes
            .iter()
            .map(|&radix| Listed {
                radix,
                hash: hash(radix),
                grandchildren: count_at(radix),
                children_less_one: 0,
            })
            .collect()
    }

    /// The positional reaction a conforming sender sends at `position`: a
    /// query, the reply's first match carrying the later matches it
    /// covers, or nothing for a later match, which that first match covers.
    fn sent(&self, position: usize) -> Option<Sent> {
        if self.is_query(position) {
            return Some(Sent::Query);
        }
        let matches = (0..self.radixes.len()).filter(|&position| !self.is_query(position));
        let mut matches = matches.map(|position| (position, self.radixes[position]));
        let (first, _) = matches.next().expect("this position is a match");
        (first == position).then(|| Sent::Match(matches.map(|(_, radix)| radix).collect()))
    }

    /// The protocol reaction sent at `position`, if any, a query listing
    /// `nested`.
    fn protocol(&self, position: usize, nested: &[Listed]) -> Option<Reaction<Erased>> {
        self.sent(position).map(|sent| match sent {
            Sent::Query => Reaction::Query(nested.to_vec()),
            Sent::Match(later) => Reaction::Match(LaterMatches::new(later)),
        })
    }

    /// The wire reaction sent at `position`, if any, a query listing
    /// `nested`.
    fn wire(&self, position: usize, nested: &[Listed]) -> Option<WireReaction> {
        self.sent(position).map(|sent| match sent {
            Sent::Query => WireReaction::Query(nested.to_vec()),
            Sent::Match(later) => WireReaction::Match(LaterMatches::new(later)),
        })
    }

    /// Remove the supplied radix and return its merge position.
    fn with_supply(&self, supply_radix: u8) -> (Self, usize) {
        let mut case = self.clone();
        case.radixes.retain(|radix| *radix != supply_radix);
        let supply_at = case.radixes.partition_point(|radix| *radix < supply_radix);
        (case, supply_at)
    }
}

/// One positional reaction a conforming sender sends.
enum Sent {
    /// A question about the position's child.
    Query,
    /// The reply's first match, with the later matching radixes it covers.
    Match(ByteSet),
}

/// A decoded reply with the scopes a take derives from it.
struct Decoded {
    /// The decoded reply.
    reply: Reply<Erased>,
    /// The scopes the reply's questions create, in wire order.
    questions: Vec<Scope>,
}

/// Exercise the adapter laws at one concrete type-level reply height.
trait AdapterHeight: NodeAt
where
    S<Self>: Height,
{
    /// Decode through the entry point for this height.
    async fn decode<F>(scope: Scope, frames: &mut F) -> Result<Decoded, DecodeError<Infallible>>
    where
        F: futures::Stream<Item = Frame> + Unpin;

    /// Encode through the entry point for this height.
    fn encode(
        scope: Scope,
        reply: Reply<Erased>,
    ) -> super::super::encode::Frames<'static, Infallible>;

    /// Begin a chunked encoding of a reply at this height.
    fn encoder(scope: Scope) -> ReplyEncoder<Local>;

    /// Return the query listing valid at this height.
    fn nested(case: &PositionalCase) -> &[Listed];

    /// Build the question below one listed child at this height.
    fn question(
        parent: crate::tree::typed::ErasedPrefix,
        nested: &[Listed],
        bound: Grandchildren,
    ) -> Scope;

    /// Check positional matches and queries in both adapter directions.
    fn positioned_reactions_are_lossless(
        leaf: &LeafCase,
        case: &PositionalCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let parent = Prefix::<S<Self>>::containing(&leaf.path());
        let scope = Scope::unbounded(parent.erase(), &case.listing());
        let nested = Self::nested(case);
        let reply = Reply::<Erased> {
            reactions: (0..case.radixes.len())
                .filter_map(|position| case.protocol(position, nested))
                .collect(),
            charge: Default::default(),
            last: true,
            carried: Default::default(),
        };
        let expected_questions = case
            .radixes
            .iter()
            .enumerate()
            .filter(|(position, _)| case.is_query(*position))
            .map(|(_, &radix)| Self::question(parent.push(radix).erase(), nested, count_at(radix)))
            .collect::<Vec<Scope>>();
        let expected_frames = expected_positional_frames(case, nested);
        let expected_publications = if case.radixes.is_empty() {
            vec![None]
        } else {
            case.radixes
                .iter()
                .enumerate()
                .filter(|(position, _)| case.sent(*position).is_some())
                .map(|(position, &radix)| {
                    case.is_query(position).then(|| {
                        Self::question(parent.push(radix).erase(), nested, count_at(radix))
                    })
                })
                .collect::<Vec<_>>()
        };

        let encoded = runtime.block_on(async {
            Self::encode(scope.clone(), reply)
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

        let sentinel = Frame::End(End::Reply);
        let mut frames = stream::iter(actual_frames.into_iter().chain([sentinel.clone()]));
        let decoded = runtime
            .block_on(Self::decode(scope, &mut frames))
            .expect("canonical positional reactions decode");
        prop_assert_eq!(
            &decoded.questions,
            &expected_questions,
            "height {}",
            Self::HEIGHT
        );
        assert_positional_reply(&decoded.reply, case, nested, Self::HEIGHT)?;
        prop_assert_eq!(
            runtime.block_on(frames.next()),
            Some(sentinel),
            "height {}",
            Self::HEIGHT
        );
        Ok(())
    }

    /// Check a supplied node merge-ordered among positional reactions.
    fn mixed_reactions_are_lossless(
        leaf: &LeafCase,
        case: &PositionalCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let (parent, supply_radix) = Prefix::<Self>::containing(&leaf.path()).pop();
        let (case, supply_at) = case.with_supply(supply_radix);
        let scope = Scope::unbounded(parent.erase(), &case.listing());
        let nested = Self::nested(&case);
        let reply = mixed_reply(&case, nested, supply_at, supply_radix, Self::node(leaf));
        let expected_frames = expected_mixed_frames(&case, nested, supply_at, leaf);
        let expected_publications = mixed_publications(&case, supply_at, |radix| {
            Self::question(parent.push(radix).erase(), nested, count_at(radix))
        });
        let expected_questions = expected_publications
            .iter()
            .filter_map(Clone::clone)
            .collect::<Vec<_>>();

        let encoded = runtime.block_on(async {
            Self::encode(scope.clone(), reply)
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

        let sentinel = Frame::End(End::Reply);
        let mut frames = stream::iter(actual_frames.into_iter().chain([sentinel.clone()]));
        let decoded = runtime
            .block_on(Self::decode(scope, &mut frames))
            .expect("canonical mixed reactions decode");
        prop_assert_eq!(
            &decoded.questions,
            &expected_questions,
            "height {}",
            Self::HEIGHT
        );
        assert_mixed_reply::<Self>(
            &decoded.reply,
            &case,
            nested,
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

    /// Check that a mixed reply encoded in the chunks `cuts` selects
    /// yields exactly the frames and publications of its whole encoding,
    /// and that each chunk's charge rides its last frame alone.
    fn chunked_encoding_is_whole(
        leaf: &LeafCase,
        case: &PositionalCase,
        cuts: u64,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let (parent, supply_radix) = Prefix::<Self>::containing(&leaf.path()).pop();
        let (case, supply_at) = case.with_supply(supply_radix);
        let scope = Scope::unbounded(parent.erase(), &case.listing());
        let nested = Self::nested(&case);
        let reply = || mixed_reply(&case, nested, supply_at, supply_radix, Self::node(leaf));
        let ledger = Ledger::new(0);
        let (whole, chunked, charged) = runtime.block_on(async {
            let whole = Self::encode(scope.clone(), reply())
                .map_ok(|encoded| encoded.into_parts())
                .try_collect::<Vec<_>>()
                .await
                .expect("canonical mixed reactions encode");
            let mut encoder = Self::encoder(scope.clone());
            let mut chunked = Vec::new();
            let mut charged = Vec::new();
            for chunk in chunks(reply(), cuts, &ledger) {
                let mut frames = pin!(encoder.chunk(chunk));
                let mut carried = Vec::new();
                while let Some(frame) = frames.next().await {
                    let frame = frame.expect("canonical mixed reactions encode");
                    carried.push(frame.charged());
                    chunked.push(frame.into_parts());
                }
                charged.push(carried);
            }
            (whole, chunked, charged)
        });
        prop_assert_eq!(chunked, whole, "height {}", Self::HEIGHT);
        for (index, carried) in charged.iter().enumerate() {
            let (last, earlier) = carried
                .split_last()
                .expect("every chunk holds a reaction, so it encodes a frame");
            prop_assert_eq!(*last, chunk_charge(index), "height {}", Self::HEIGHT);
            prop_assert!(
                earlier.iter().all(|&bytes| bytes == 0),
                "height {}: a chunk's earlier frame carried a charge",
                Self::HEIGHT
            );
        }
        prop_assert_eq!(ledger.held(), 0, "height {}", Self::HEIGHT);
        Ok(())
    }

    /// Check that duplicate supplied paths fail with their exact order error.
    fn duplicate_leaf_is_rejected(
        leaf: &LeafCase,
        runtime: &tokio::runtime::Runtime,
    ) -> TestCaseResult {
        let parent = Prefix::<Self>::containing(&leaf.path()).pop().0;
        let mut frames = stream::iter(duplicate_frames(leaf));
        let error = runtime
            .block_on(Self::decode(
                Scope::unbounded(parent.erase(), &[]),
                &mut frames,
            ))
            .err()
            .expect("duplicate leaves are not strictly ascending");
        assert_duplicate_error(error, leaf, Self::HEIGHT)
    }

    /// Check that a supplied path outside the reply scope is rejected.
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
        let mut frames = stream::iter([supplied_frame(leaf, Flow::End)]);
        let error = runtime
            .block_on(Self::decode(
                Scope::unbounded(foreign.erase(), &[]),
                &mut frames,
            ))
            .err()
            .expect("a leaf outside the retained scope must fail");
        assert_foreign_error(error, foreign, leaf, Self::HEIGHT)
    }
}

/// Select the leaf-level adapter entries and terminal questions.
impl AdapterHeight for Z {
    async fn decode<F>(scope: Scope, frames: &mut F) -> Result<Decoded, DecodeError<Infallible>>
    where
        F: futures::Stream<Item = Frame> + Unpin,
    {
        let level = ReplyLevel::Leaf;
        let reply = decode_whole_reply(
            &mut LeafEdge::new(),
            Local,
            &nothing_seen(),
            unbounded(),
            level,
            &scope,
            frames,
            codec(),
            Some(&crate::tree::typed::PathKey::fixture()),
        )
        .await?;
        let questions = derive_questions(level, &scope, &mut Replay::default(), &reply.reactions)
            .expect("decoding validated positions");
        Ok(Decoded { reply, questions })
    }

    fn encode(
        scope: Scope,
        reply: Reply<Erased>,
    ) -> super::super::encode::Frames<'static, Infallible> {
        encode_leaf_reply(Local, RunBudget::default(), scope, reply)
    }

    fn encoder(scope: Scope) -> ReplyEncoder<Local> {
        ReplyEncoder::leaf(Local, RunBudget::default(), scope)
    }

    fn nested(_case: &PositionalCase) -> &[Listed] {
        &[]
    }

    fn question(
        parent: crate::tree::typed::ErasedPrefix,
        _nested: &[Listed],
        _bound: Grandchildren,
    ) -> Scope {
        Scope::leaf(parent)
    }
}

/// Select the branch-level adapter entries and nested questions.
impl<H> AdapterHeight for S<H>
where
    H: AdapterHeight + PartialEq,
    S<H>: Convert,
    S<S<H>>: Height,
{
    async fn decode<F>(scope: Scope, frames: &mut F) -> Result<Decoded, DecodeError<Infallible>>
    where
        F: futures::Stream<Item = Frame> + Unpin,
    {
        let level = ReplyLevel::Branch;
        let reply = decode_whole_reply(
            &mut LeafEdge::new(),
            Local,
            &nothing_seen(),
            unbounded(),
            level,
            &scope,
            frames,
            codec(),
            Some(&crate::tree::typed::PathKey::fixture()),
        )
        .await?;
        let questions = derive_questions(level, &scope, &mut Replay::default(), &reply.reactions)
            .expect("decoding validated positions");
        Ok(Decoded { reply, questions })
    }

    fn encode(
        scope: Scope,
        reply: Reply<Erased>,
    ) -> super::super::encode::Frames<'static, Infallible> {
        encode_reply(Local, RunBudget::default(), scope, reply)
    }

    fn encoder(scope: Scope) -> ReplyEncoder<Local> {
        ReplyEncoder::branch(Local, RunBudget::default(), scope)
    }

    fn nested(case: &PositionalCase) -> &[Listed] {
        &case.nested
    }

    fn question(
        parent: crate::tree::typed::ErasedPrefix,
        nested: &[Listed],
        bound: Grandchildren,
    ) -> Scope {
        Scope::new(parent, nested, bound)
    }
}

/// The grandchild count a case's listing gives the child at `radix`: a
/// distinct count per position, so a question that took its bound from the
/// wrong position fails to match.
fn count_at(radix: u8) -> Grandchildren {
    Grandchildren::new(usize::from(radix) + 1).expect("a radix plus one is a valid count")
}

/// Encode one leaf as one supplied wire frame.
fn supplied_frame(leaf: &LeafCase, flow: Flow) -> Frame {
    Frame::Reaction(
        WireReaction::Supply(leaf_run(&[(&leaf.version, &leaf.message)])),
        flow,
    )
}

/// Repeat one supplied leaf across two consecutive frames.
fn duplicate_frames(leaf: &LeafCase) -> [Frame; 2] {
    [
        supplied_frame(leaf, Flow::Continue),
        supplied_frame(leaf, Flow::End),
    ]
}

/// Merge one supplied node into the positional reactions for `case`.
fn mixed_reply<H: Height>(
    case: &PositionalCase,
    query_listing: &[Listed],
    supply_at: usize,
    supply_radix: u8,
    supply: typed::Node<H>,
) -> Reply<Erased> {
    let mut supply = Some(<Local as Backend>::erase(supply.into()));
    let mut reactions = Vec::with_capacity(case.radixes.len() + 1);
    for position in 0..=case.radixes.len() {
        if position == supply_at {
            reactions.push(Reaction::Supply(
                supply_radix,
                supply.take().expect("the supply has one insertion point"),
            ));
        }
        if position < case.radixes.len() {
            reactions.extend(case.protocol(position, query_listing));
        }
    }
    Reply {
        reactions,
        charge: Default::default(),
        last: true,
        carried: Default::default(),
    }
}

/// `reply`'s reactions as chunks, cut after reaction `i` wherever bit `i`
/// of `cuts` is set but after the last, which the last chunk holds as a
/// reply's last chunk must.
///
/// Chunk `i` carries a charge on `ledger` of [`chunk_charge`]`(i)` bytes,
/// so each chunk's charge is told apart from every other's.
fn chunks(reply: Reply<Erased>, cuts: u64, ledger: &Ledger) -> Vec<Reply<Erased>> {
    let count = reply.reactions.len();
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    for (index, reaction) in reply.reactions.into_iter().enumerate() {
        current.push(reaction);
        if index + 1 < count && index < 64 && (cuts >> index) & 1 == 1 {
            chunks.push(Reply {
                reactions: std::mem::take(&mut current),
                charge: ledger.charge(Holding::OutgoingReplies, chunk_charge(chunks.len())),
                last: false,
                carried: Default::default(),
            });
        }
    }
    chunks.push(Reply {
        reactions: current,
        charge: ledger.charge(Holding::OutgoingReplies, chunk_charge(chunks.len())),
        last: true,
        carried: Default::default(),
    });
    chunks
}

/// The bytes [`chunks`] charges its chunk at `index`: distinct for every
/// chunk, and never zero.
fn chunk_charge(index: usize) -> usize {
    index + 1
}

/// Build the canonical frames for a mixed reply independently of the encoder.
fn expected_mixed_frames(
    case: &PositionalCase,
    query_listing: &[Listed],
    supply_at: usize,
    leaf: &LeafCase,
) -> Vec<Frame> {
    let mut reactions = Vec::with_capacity(case.radixes.len() + 1);
    for position in 0..=case.radixes.len() {
        if position == supply_at {
            reactions.push(WireReaction::Supply(leaf_run(&[(
                &leaf.version,
                &leaf.message,
            )])));
        }
        if position < case.radixes.len() {
            reactions.extend(case.wire(position, query_listing));
        }
    }
    reply_frames(reactions)
}

/// Build the question publication sequence for a mixed reply.
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
        if position < case.radixes.len() && case.sent(position).is_some() {
            publications.push(
                case.is_query(position)
                    .then(|| question(case.radixes[position])),
            );
        }
    }
    publications
}

/// Require a decoded mixed reply to preserve every reaction and supplied
/// leaf, its reactions allocated at exactly their count.
fn assert_mixed_reply<H: Convert>(
    reply: &Reply<Erased>,
    case: &PositionalCase,
    query_listing: &[Listed],
    supply_at: usize,
    supply_radix: u8,
    leaf: &LeafCase,
    runtime: &tokio::runtime::Runtime,
) -> TestCaseResult
where
    S<H>: Height,
{
    let sent = (0..case.radixes.len())
        .filter(|&position| case.sent(position).is_some())
        .count();
    prop_assert_eq!(reply.reactions.len(), sent + 1);
    prop_assert_eq!(reply.reactions.capacity(), reply.reactions.len());
    let mut reaction = 0;
    for position in 0..=case.radixes.len() {
        if position == supply_at {
            let Reaction::Supply(radix, node) = &reply.reactions[reaction] else {
                return Err(TestCaseError::fail(format!(
                    "height {} lost its supply at reaction {reaction}",
                    H::HEIGHT
                )));
            };
            prop_assert_eq!(*radix, supply_radix, "height {}", H::HEIGHT);
            assert_node_leaf::<H>(node, leaf, runtime)?;
            reaction += 1;
        }
        if let Some(sent) = (position < case.radixes.len())
            .then(|| case.sent(position))
            .flatten()
        {
            match (sent, &reply.reactions[reaction]) {
                (Sent::Match(later), Reaction::Match(actual)) => {
                    prop_assert_eq!(actual.set(), later, "height {}", H::HEIGHT);
                }
                (Sent::Query, Reaction::Query(actual)) => {
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

/// Require an erased node to contain exactly the expected leaf.
fn assert_node_leaf<H: Convert>(
    node: &Erased,
    expected_leaf: &LeafCase,
    runtime: &tokio::runtime::Runtime,
) -> TestCaseResult
where
    S<H>: Height,
{
    let prefix = Prefix::<H>::containing(&expected_leaf.path());
    let leaves = runtime.block_on(async {
        <Local as Backend>::leaves(
            Local,
            prefix,
            <Local as Backend>::assume(&prefix, node.clone()),
        )
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
    let bounds = actual_leaf.span();
    prop_assert_eq!(bounds.hi(), &expected_leaf.version);
    prop_assert_eq!(
        actual_leaf.message().as_slice(),
        expected_leaf.message.as_slice()
    );
    Ok(())
}

/// Build the canonical frames for a positional reply.
fn expected_positional_frames(case: &PositionalCase, nested: &[Listed]) -> Vec<Frame> {
    reply_frames((0..case.radixes.len()).filter_map(|position| case.wire(position, nested)))
}

/// Require a decoded positional reply to preserve its reaction sequence,
/// allocated at exactly its length.
fn assert_positional_reply(
    reply: &Reply<Erased>,
    case: &PositionalCase,
    nested: &[Listed],
    height: usize,
) -> TestCaseResult {
    let sent: Vec<(usize, Sent)> = (0..case.radixes.len())
        .filter_map(|position| case.sent(position).map(|sent| (position, sent)))
        .collect();
    prop_assert_eq!(reply.reactions.len(), sent.len(), "height {}", height);
    prop_assert_eq!(
        reply.reactions.capacity(),
        reply.reactions.len(),
        "height {}",
        height
    );
    for ((position, sent), reaction) in sent.into_iter().zip(&reply.reactions) {
        match (sent, reaction) {
            (Sent::Match(later), Reaction::Match(actual)) => {
                prop_assert_eq!(
                    actual.set(),
                    later,
                    "height {}, position {}",
                    height,
                    position
                );
            }
            (Sent::Query, Reaction::Query(actual)) => {
                prop_assert_eq!(actual, nested, "height {}, position {}", height, position);
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

/// Require the exact repeated path in a leaf-order rejection.
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

/// Find a version-derived parent distinct from `actual` at the same height.
fn foreign_parent<H>(leaf: &LeafCase, actual: Prefix<S<H>>) -> Prefix<S<H>>
where
    H: Height,
    S<H>: Height,
{
    (1..=u16::MAX)
        .map(|offset| LeafCase::new(leaf.value.wrapping_add(u64::from(offset)), 0))
        .map(|candidate| Prefix::<S<H>>::containing(&candidate.path()))
        .find(|candidate| *candidate != actual)
        .expect("a non-root prefix has another version-derived value")
}

/// Require the supplied and expected paths in an out-of-scope rejection.
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

proptest! {
    /// At every height, arbitrary mixtures of matches and queries encode and
    /// decode exactly; leaf queries are empty and publish terminal scopes at
    /// the same height, while higher queries publish lower positional scopes.
    ///
    /// A decoded reply's reactions are allocated at exactly their count: a
    /// decoded reply can park, and its question's reservation prices it per
    /// reaction, which a growth buffer's slack would exceed.
    ///
    /// The generators explicitly include the all-match case.
    #[test]
    fn positional_reactions_are_lossless_at_every_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
        radixes in prop_oneof![
            Just(BTreeSet::new()),
            vec(any::<u8>(), 0..=8).prop_map(BTreeSet::from_iter),
        ],
        queries in prop_oneof![Just(0), any::<u64>()],
        nested in vec(any::<u8>(), 0..=8).prop_map(BTreeSet::from_iter),
        salt in any::<u8>(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let case = PositionalCase {
            radixes: radixes.into_iter().collect(),
            queries,
            nested: nested
                .into_iter()
                .map(|radix| listed(radix, hash(radix ^ salt)))
                .collect(),
        };
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, AdapterHeight::positioned_reactions_are_lossless(
                &leaf, &case, &runtime
            ); 0..32)?;
        }
    }

    /// At every height, a version-derived supply remains correctly keyed when
    /// merge-ordered among arbitrary positional matches and queries, and the
    /// decoded reactions are allocated at exactly their count.
    ///
    /// The generators explicitly include the single-supply case.
    #[test]
    fn mixed_reactions_are_lossless_at_every_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
        radixes in prop_oneof![
            Just(BTreeSet::new()),
            vec(any::<u8>(), 0..=8).prop_map(BTreeSet::from_iter),
        ],
        queries in prop_oneof![Just(0), any::<u64>()],
        nested in vec(any::<u8>(), 0..=8).prop_map(BTreeSet::from_iter),
        salt in any::<u8>(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let case = PositionalCase {
            radixes: radixes.into_iter().collect(),
            queries,
            nested: nested
                .into_iter()
                .map(|radix| listed(radix, hash(radix ^ salt)))
                .collect(),
        };
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, AdapterHeight::mixed_reactions_are_lossless(
                &leaf, &case, &runtime
            ); 0..32)?;
        }
    }

    /// At every height, a mixed reply encoded chunk by chunk, cut at any
    /// reaction boundaries, writes exactly its whole encoding's frames and
    /// questions.
    ///
    /// Every chunk's last frame continues the reply, and only the reply's
    /// last reaction ends it. Each chunk's charge rides its last frame, and
    /// every earlier frame carries none.
    #[test]
    fn chunked_replies_encode_as_whole_replies_at_every_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
        radixes in vec(any::<u8>(), 0..=8).prop_map(BTreeSet::from_iter),
        queries in any::<u64>(),
        nested in vec(any::<u8>(), 0..=8).prop_map(BTreeSet::from_iter),
        cuts in any::<u64>(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let case = PositionalCase {
            radixes: radixes.into_iter().collect(),
            queries,
            nested: nested.into_iter().map(|radix| listed(radix, hash(radix))).collect(),
        };
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, AdapterHeight::chunked_encoding_is_whole(
                &leaf, &case, cuts, &runtime
            ); 0..32)?;
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
            at_height!(height, AdapterHeight::duplicate_leaf_is_rejected(&leaf, &runtime); 0..32)?;
        }
    }

    /// At every height with more than one possible parent scope, a supplied
    /// leaf is rejected unless its version-derived path is under that scope.
    #[test]
    fn foreign_supply_is_rejected_at_every_scopable_height(
        value in any::<u64>(),
        ticks in any::<u8>(),
    ) {
        let leaf = LeafCase::new(value, ticks);
        let runtime = runtime();
        for height in 0..32 {
            at_height!(height, AdapterHeight::foreign_leaf_is_rejected(&leaf, &runtime); 0..32)?;
        }
    }
}
