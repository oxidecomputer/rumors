//! The decoder: the end-of-stream check every decoder runs once its
//! question queue closes, and how it charges the chunks it parks out of the
//! room each question set aside.

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
    Local,
    channel::{QueueKind, QueueRole, channel},
    erased::Reaction,
    holding::Holding,
    ledger::{Ledger, buffer},
    materialized::{parked_bytes, parked_room},
    message::{Grandchildren, Listed, listed},
    remote::{
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
    let (mut frames, mut incoming) = channel(QueueRole::new(QueueKind::PipeIncoming, 0), 1);
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
    let (mut frames, mut incoming) = channel(QueueRole::new(QueueKind::PipeIncoming, 0), 1);
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
/// `entries` children, and after which of its frames the peer pauses.
#[derive(Clone, Debug)]
struct SplitReply {
    /// Each position's reaction: `None` matches, `Some(entries)` queries.
    reactions: Vec<Option<u8>>,
    /// Whether the frames stop arriving after each reaction's frame.
    pauses: Vec<bool>,
}

/// Replies of one to a dozen positions, each a match or a query listing up
/// to four children, paused after any of their frames.
fn split_replies() -> impl Strategy<Value = SplitReply> {
    prop::collection::vec((prop::option::of(0..=4_u8), any::<bool>()), 1..=12).prop_map(
        |positions| {
            let (reactions, pauses) = positions.into_iter().unzip();
            SplitReply { reactions, pauses }
        },
    )
}

/// The listing a query of `entries` children carries, at ascending radices.
fn listing(entries: u8) -> Vec<Listed> {
    (0..entries)
        .map(|radix| listed(radix, Hash::default()))
        .collect()
}

proptest! {
    /// However a reply's frames arrive, the decoder charges each chunk it
    /// parks its own price out of the room the question set aside, never
    /// more than the room in all, and returns the rest of the room once the
    /// reply is decoded.
    ///
    /// The question lists one child per position, and its peer's count is
    /// the reply's listing entries, so the room prices exactly the entries
    /// the reply carries and one parked chunk per question and one more.
    /// The decoder delivers a chunk wherever the frames pause after a
    /// query, so the pauses split the reply anywhere a peer's could, and
    /// the test predicts each split. While
    /// the reply decodes, the place holds exactly the room, so no chunk
    /// fell short of it; each chunk's charge is its price, its vectors at
    /// capacities equal to their lengths, as the room's length-based price
    /// assumes; and once decoded, the place holds the chunks' charges and
    /// nothing more, which return with the chunks.
    #[test]
    fn parked_chunks_take_their_price_from_the_room_and_return_the_rest(
        reply in split_replies(),
    ) {
        type E = <Local as crate::tree::mirror::streaming::Backend>::Erased;
        let positions = reply.reactions.len();
        let entries: usize = reply.reactions.iter().flatten().map(|&entries| usize::from(entries)).sum();
        let peer_count = Grandchildren::new(entries.max(1)).expect("a dozen queries of four fit");
        let asked: Vec<Listed> = (0..positions)
            .map(|radix| Listed {
                radix: u8::try_from(radix).expect("a dozen positions fit a radix"),
                hash: Hash::default(),
                grandchildren: Grandchildren::ONE,
            })
            .collect();
        let scope = Scope::opening(&asked, peer_count);
        let ledger = Ledger::default();
        let room_bytes = parked_room::<E>(scope.reply_bound(), positions);
        let room = ledger.charge(Holding::ParkedReplies, room_bytes);

        let ingress = Ingress::new(
            Local,
            u64::MAX,
            u64::MAX,
            Grandchildren::MAX,
            PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
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

        let mut chunks = Vec::new();
        // The decoder delivers what it holds at a pause only if a query is
        // among it, and always at the reply's end.
        let (mut expected, mut asks) = (1, false);
        for (index, (reaction, pause)) in reply.reactions.iter().zip(&reply.pauses).enumerate() {
            let wire = match reaction {
                None => WireReaction::Match,
                Some(entries) => {
                    asks = true;
                    WireReaction::Query(listing(*entries))
                }
            };
            let last = index + 1 == positions;
            let flow = if last { Flow::End } else { Flow::Continue };
            frames
                .try_send(Frame::Reaction(wire, flow))
                .unwrap_or_else(|_| panic!("the stream has room for every frame"));
            if !(last || *pause) {
                continue;
            }
            if !last && asks {
                expected += 1;
                asks = false;
            }
            prop_assert!(decoder.as_mut().poll(&mut context).is_pending(), "the decoder awaits more");
            while let Poll::Ready(Some(chunk)) = parked.poll_next_unpin(&mut context) {
                chunks.push(chunk);
            }
            if !last {
                prop_assert_eq!(
                    ledger.held_for(Holding::ParkedReplies),
                    room_bytes,
                    "while the reply decodes, its chunks and the rest of its room hold the room",
                );
            }
        }

        let mut charged = 0;
        for parked in &chunks {
            let chunk = &parked.chunk;
            prop_assert_eq!(chunk.charge.bytes(), parked_bytes(chunk), "a chunk carries its price");
            prop_assert_eq!(chunk.reactions.capacity(), chunk.reactions.len());
            for reaction in &chunk.reactions {
                if let Reaction::Query(listing) = reaction {
                    prop_assert_eq!(buffer(listing), listing.len() * size_of::<Listed>());
                }
            }
            charged += chunk.charge.bytes();
        }
        prop_assert_eq!(chunks.len(), expected, "the pauses after queries split the reply");
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
