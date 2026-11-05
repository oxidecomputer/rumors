//! The decoder: the end-of-stream check every decoder runs once its
//! question queue closes, and how it charges the chunks it parks out of the
//! room each question set aside.

use crate::tree::mirror::streaming::message::LaterMatches;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use futures::StreamExt;
use proptest::prelude::*;

use super::super::{Ingress, Outstanding, queues};
use super::reject_extra;
use crate::message::{PayloadCodec, PayloadDepthLimit};
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::{
    FAN, Local,
    byte_set::ByteSet,
    channel::{QueueKind, QueueRole, channel},
    erased::Reaction,
    holding::Holding,
    ledger::{Ledger, buffer},
    materialized::{Deliverable, parked_bytes, parked_price, parked_room},
    message::{Grandchildren, Listed, listed},
    remote::{
        LANE_CAPACITY,
        adapter::{ReplyLevel, Scope},
        codec::{End, Flow, Frame, Reaction as WireReaction},
        proxy::Error,
    },
};
use crate::tree::typed::{
    Hash,
    height::{Height, UnderRoot},
};

/// A decoder that received no question reports a frame already waiting on
/// its stream as an unasked reply, and its closed channel refuses every
/// later frame, so no unasked frame is dropped unseen.
#[test]
fn an_unquestioned_decoder_reports_a_waiting_frame() {
    let (mut frames, mut incoming) =
        channel(QueueRole::new(QueueKind::PipeIncoming, 0), LANE_CAPACITY);
    run_to_quiescence(async {
        frames.send(Frame::End(End::Reply)).await.unwrap();
        let result = reject_extra::<()>(false, &mut incoming).await;
        assert!(matches!(result, Err(Error::UnaskedReply)), "{result:?}");
        assert!(
            frames.send(Frame::End(End::Reply)).await.is_err(),
            "the closed channel refuses later frames"
        );
    })
    .expect("the check never waits");
}

/// A decoder that received no question finishes at once on an empty
/// stream, and its closed channel refuses a frame that arrives afterward.
#[test]
fn an_unquestioned_decoder_finishes_on_an_empty_stream() {
    let (mut frames, mut incoming) =
        channel(QueueRole::new(QueueKind::PipeIncoming, 0), LANE_CAPACITY);
    run_to_quiescence(async {
        let result = reject_extra::<()>(false, &mut incoming).await;
        assert!(result.is_ok(), "{result:?}");
        assert!(
            frames.send(Frame::End(End::Reply)).await.is_err(),
            "the closed channel refuses a frame that arrives afterward"
        );
    })
    .expect("the check never waits");
}

/// One reply's positional reactions, each a match or a query listing
/// `entries` children, this side's child count of each position's child,
/// and after which of its frames the peer pauses.
#[derive(Clone, Debug)]
struct SplitReply {
    /// Each position's reaction: `None` matches, `Some(entries)` queries.
    reactions: Vec<Option<u8>>,
    /// This side's child count of each position's child, which a match
    /// shows is the peer's too.
    children: Vec<u8>,
    /// Whether the frames stop arriving after each reaction's frame.
    pauses: Vec<bool>,
}

/// Replies of one to a dozen positions, each a match or a query listing up
/// to four children, of children with one to four children each, paused
/// after any of their frames.
fn split_replies() -> impl Strategy<Value = SplitReply> {
    prop::collection::vec(
        (prop::option::of(0..=4_u8), 1..=4_u8, any::<bool>()),
        1..=12,
    )
    .prop_map(|positions| {
        let mut reply = SplitReply {
            reactions: Vec::new(),
            children: Vec::new(),
            pauses: Vec::new(),
        };
        for (reaction, children, pause) in positions {
            reply.reactions.push(reaction);
            reply.children.push(children);
            reply.pauses.push(pause);
        }
        reply
    })
}

/// The listing entries the peer's child at each position contributes to
/// its grandchild count of the scope: what a query lists, or for a match,
/// this side's child count, the same on both sides.
fn shares(reply: &SplitReply) -> impl Iterator<Item = usize> + '_ {
    reply
        .reactions
        .iter()
        .zip(&reply.children)
        .map(|(reaction, &children)| usize::from(reaction.unwrap_or(children)))
}

/// The listing a query of `entries` children carries, at ascending radices.
fn listing(entries: u8) -> Vec<Listed> {
    (0..entries)
        .map(|radix| listed(radix, Hash::default()))
        .collect()
}

proptest! {
    /// However a reply's frames arrive, the decoder keeps its room at the
    /// price of what the reply can still deliver, and leaves none of it
    /// once the reply is decoded.
    ///
    /// Each chunk it parks takes its own price out of that room, holds at
    /// most one question, and ends at it.
    ///
    /// The question lists one child per position, each with this side's
    /// child count of it, and the peer's count is the sum of each child's
    /// share: its query's listing, or for a matched child, that child
    /// count. The peer sends a frame per position but the matches after
    /// the first, which the first match covers, so its last frame may come
    /// before its last position. The decoder ends a chunk at each query
    /// but the reply's last,
    /// whatever the pauses, so the test predicts the chunks from the reply
    /// alone: one per query before the last reaction, and one more. The
    /// pauses only choose where the test looks. At every
    /// pause, the room the chunks have not taken is the price of what the
    /// reply can still deliver, restated here from the reply so far: later
    /// reactions at most the slots the bound leaves, a covered child taking
    /// one beside each frame, and one per radix above the latest that the
    /// match does not cover, later queries only at positions not yet
    /// reached and not covered, later listing entries at most what the
    /// peer's count leaves beside the listings and matched or covered
    /// shares so far and a fan per later query, chunks at most one per
    /// query to come and one more, and the covered set its match carries,
    /// ready, with its covered children's slots, or still to come.
    /// That bound must cover what the reply then actually delivers, so no
    /// chunk falls short; each chunk's charge is its price, its vectors at
    /// capacities equal to their lengths; and once decoded, the place holds
    /// the chunks' charges alone, which return with the chunks.
    #[test]
    fn parked_chunks_keep_the_room_at_what_the_reply_can_still_deliver(
        reply in split_replies(),
    ) {
        type E = <Local as crate::tree::mirror::streaming::Backend>::Erased;
        let positions = reply.reactions.len();
        let entries: usize = shares(&reply).sum();
        let peer_count = Grandchildren::new(entries.max(1)).expect("a dozen shares of four fit");
        let asked: Vec<Listed> = reply
            .children
            .iter()
            .enumerate()
            .map(|(radix, &children)| Listed {
                radix: u8::try_from(radix).expect("a dozen positions fit a radix"),
                hash: Hash::default(),
                grandchildren: Grandchildren::ONE,
                children_less_one: children - 1,
            })
            .collect();
        let scope = Scope::opening(&asked, peer_count);
        let bound = scope.reply_bound();
        let ledger = Ledger::default();
        let room = ledger.charge(Holding::ParkedReplies, parked_room::<E>(bound, positions));

        let ingress = Ingress::new(
            Local,
            u64::MAX,
            u64::MAX,
            Grandchildren::MAX,
            PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
            Some(crate::network::NetworkKeys::fixture()),
        );
        let height = UnderRoot::HEIGHT;
        let (mut publish, questions) = channel(QueueRole::new(QueueKind::ProxyLocalQuestions, height), 1);
        let (mut frames, incoming) = channel(QueueRole::new(QueueKind::PipeIncoming, height), positions);
        let (parking, mut parked) = queues::parked_replies::<E>(height);
        publish
            .try_send(Outstanding { scope, record: Default::default(), room })
            .unwrap_or_else(|_| panic!("the queue has room for one question"));
        let mut decoder = pin!(ingress.decode_and_park(ReplyLevel::Branch, questions, incoming, parking));
        let mut context = Context::from_waker(Waker::noop());

        // Each pause's chunk boundary, and the reactions delivered by then.
        let mut chunks = Vec::new();
        let mut boundaries = Vec::new();
        let mut expected = 1;
        // A conforming peer sends a frame per position but for the matches
        // after the first, which the first match covers.
        let radix = |position: usize| u8::try_from(position).expect("a dozen positions fit a radix");
        let first_match = reply.reactions.iter().position(Option::is_none);
        let covered: ByteSet = (0..positions)
            .filter(|&position| reply.reactions[position].is_none() && Some(position) != first_match)
            .map(radix)
            .collect();
        let framed: Vec<usize> = (0..positions)
            .filter(|&position| !covered.contains(radix(position)))
            .collect();
        let shares: Vec<usize> = shares(&reply).collect();
        let covered_shares: usize = (0..positions)
            .filter(|&position| covered.contains(radix(position)))
            .map(|position| shares[position])
            .sum();
        let last_frame = *framed.last().expect("a reply of one position or more sends a frame");
        // What the reply so far has carried: frames, the listing entries and
        // matched or covered shares of the peer's count, and its match.
        let (mut spent, mut matched) = (0, false);
        for (index, &position) in framed.iter().enumerate() {
            let reaction = reply.reactions[position];
            let pause = reply.pauses[position];
            let sent = index + 1;
            spent += shares[position];
            let wire = match reaction {
                None => {
                    matched = true;
                    spent += covered_shares;
                    WireReaction::Match(LaterMatches::new(covered))
                }
                Some(entries) => WireReaction::Query(listing(entries)),
            };
            let last = position == last_frame;
            if reaction.is_some() && !last {
                expected += 1;
            }
            let flow = if last { Flow::End } else { Flow::Continue };
            frames
                .try_send(Frame::Reaction(wire, flow))
                .unwrap_or_else(|_| panic!("the stream has room for every frame"));
            if !(last || pause) {
                continue;
            }
            prop_assert!(decoder.as_mut().poll(&mut context).is_pending(), "the decoder awaits more");
            while let Poll::Ready(Some(chunk)) = parked.poll_next_unpin(&mut context) {
                chunks.push(chunk);
            }
            if last {
                continue;
            }
            let delivered: usize = chunks.iter().map(|parked| parked.chunk.reactions.len()).sum();
            let charged: usize = chunks.iter().map(|parked| parked.chunk.charge.bytes()).sum();
            let room = ledger.held_for(Holding::ParkedReplies) - charged;
            // What is decoded but not yet delivered.
            let ready: Vec<Option<u8>> = framed[delivered..sent]
                .iter()
                .map(|&position| reply.reactions[position])
                .collect();
            let ready_questions = ready.iter().flatten().count();
            let ready_listing: usize = ready.iter().flatten().map(|&entries| usize::from(entries)).sum();
            let ready_covered = if ready.contains(&None) { covered.len() } else { 0 };
            // What can still come: later reactions in the slots the frames
            // and covered children so far leave, at radices above this
            // position's that a match received so far does not cover, later
            // queries at the positions left that such a match does not
            // cover, and a covered set only with the match, which needs two
            // positions left.
            let (slots, covered_above) = if matched {
                let above = covered.iter().filter(|&radix| usize::from(radix) > position).count();
                (sent + covered.len(), above)
            } else {
                (sent, 0)
            };
            let reactions = (bound.reactions - slots).min(FAN - (position + 1) - covered_above);
            let left = if matched { framed.len() - sent } else { positions - (position + 1) };
            let questions = left.min(reactions);
            let listing = bound.listing.saturating_sub(spent).min(FAN * questions);
            let still = Deliverable {
                questions: ready_questions + questions,
                reactions: ready.len() + ready_covered + reactions,
                listing: ready_listing + listing,
                sets: usize::from(ready_covered > 0) + usize::from(!matched && left >= 2),
            };
            prop_assert_eq!(room, parked_price::<E>(still), "the room is the price of what can still come");
            boundaries.push((chunks.len(), room));
        }

        // Each bound covers what the reply delivered after it.
        for (taken, room) in boundaries {
            let later: usize = chunks[taken..].iter().map(|parked| parked.chunk.charge.bytes()).sum();
            prop_assert!(later <= room, "{later} B delivered after a room of {room} B");
        }
        let mut charged = 0;
        for parked in &chunks {
            let chunk = &parked.chunk;
            prop_assert_eq!(chunk.charge.bytes(), parked_bytes(chunk), "a chunk carries its price");
            let asked = chunk.reactions.iter().position(|reaction| matches!(reaction, Reaction::Query(_)));
            if let Some(at) = asked {
                prop_assert_eq!(at + 1, chunk.reactions.len(), "a chunk ends at its question");
            }
            prop_assert_eq!(chunk.reactions.capacity(), chunk.reactions.len());
            for reaction in &chunk.reactions {
                if let Reaction::Query(listing) = reaction {
                    prop_assert_eq!(buffer(listing), listing.len() * size_of::<Listed>());
                }
            }
            charged += chunk.charge.bytes();
        }
        prop_assert_eq!(chunks.len(), expected, "the reply's queries split it");
        prop_assert!(chunks.last().is_some_and(|parked| parked.chunk.last), "the reply ends");
        prop_assert_eq!(
            ledger.held_for(Holding::ParkedReplies),
            charged,
            "once the reply is decoded, only its chunks' charges remain",
        );
        drop(chunks);
        prop_assert_eq!(ledger.held(), 0, "the chunks return the rest as they go");
        drop(publish);
        drop(frames);
        prop_assert!(
            matches!(decoder.as_mut().poll(&mut context), Poll::Ready(Ok(()))),
            "the decoder ends with its queues",
        );
    }
}
