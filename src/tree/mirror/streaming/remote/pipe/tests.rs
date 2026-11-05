//! The pipe's multiplexer and demultiplexer, driven against in-memory byte
//! buffers under the closed-world poller.

use std::pin::pin;

use futures::{StreamExt, future::Either};
use proptest::prelude::*;
use tokio::io::AsyncReadExt;

use super::*;
use crate::Version;
use crate::message::Message;
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::remote::codec::{Flow, LeafRun};

/// The speaker whose direction these tests write and read.
const SPEAKER: Speaker = Speaker::Initiator;

/// The completion marker that closes a session: a CBOR text item.
const MARKER: [u8; 2] = [0x61, b'.'];

/// The kinds of reply frame the multiplexer distinguishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A thin reaction: served in the descent class.
    Match,
    /// The boundary of an empty reply: served in the descent class.
    ReplyEnd,
    /// A supply run: served in the bulk class.
    Supply,
}

/// Build one reply frame of `kind` whose content names `tag`, so frames of
/// one kind stay distinguishable after decoding.
fn frame(kind: Kind, tag: u64) -> ReplyFrame {
    match kind {
        Kind::Match => ReplyFrame::reaction(Reaction::Match, Flow::End),
        Kind::ReplyEnd => ReplyFrame::reply_end(),
        Kind::Supply => {
            let mut run = LeafRun::new();
            run.push(&Version::new(), &Message::new(tag))
                .expect("one small record fits a fresh run");
            ReplyFrame::reaction(Reaction::Supply(run), Flow::End)
        }
    }
}

/// An interior logical stream, where every reaction form is admissible
/// for either speaker.
fn interior(index: u8) -> Stream {
    Stream::new(index).expect("interior indices name streams")
}

/// Decode every frame in `bytes`, in pipe order.
fn decode_all(bytes: &[u8]) -> Vec<(Stream, Frame)> {
    run_to_quiescence(async {
        let mut read = FrameRead::new(SPEAKER, RunBudget::from_bytes(usize::MAX), bytes);
        let mut frames = Vec::new();
        while let Some(frame) = read.frame().await.expect("the pipe carries whole frames") {
            frames.push(frame);
        }
        frames
    })
    .expect("decoding a buffer completes")
}

/// Hand `frames` to one lane, then end it.
async fn produce(mut out: Outgoing, frames: Vec<ReplyFrame>) {
    for frame in frames {
        out.frame(frame).await;
    }
    out.finish().await;
}

/// Run producers for `lanes` against a multiplexer writing into a buffer.
fn multiplex(lanes: Vec<Vec<ReplyFrame>>) -> Vec<u8> {
    let mut multiplexer = Multiplexer::default();
    let producers: Vec<_> = lanes
        .into_iter()
        .enumerate()
        .map(|(index, frames)| {
            let index = u8::try_from(index + 1).expect("few lanes");
            produce(multiplexer.lane(interior(index), 0), frames)
        })
        .collect();
    run_to_quiescence(async move {
        let mut written = Vec::new();
        let run = multiplexer.run(&mut written, SPEAKER, Recorder::default());
        let (result, _) = futures::join!(run, futures::future::join_all(producers));
        result.expect("writing into a vector cannot fail");
        written
    })
    .expect("the multiplexer drains every lane")
}

proptest! {
    /// Every lane's frames reach the pipe whole and in lane order, a lane
    /// that carried any frame ends with exactly one end-of-stream frame, and
    /// a lane that carried none writes nothing at all.
    #[test]
    fn multiplexed_lanes_arrive_whole_and_in_order(
        lanes in prop::collection::vec(
            prop::collection::vec(
                prop::sample::select(vec![Kind::Match, Kind::ReplyEnd, Kind::Supply]),
                0..6,
            ),
            1..4,
        )
    ) {
        let sent: Vec<Vec<ReplyFrame>> = lanes
            .iter()
            .enumerate()
            .map(|(lane, kinds)| {
                kinds
                    .iter()
                    .enumerate()
                    .map(|(position, &kind)| frame(kind, (lane * 100 + position) as u64))
                    .collect()
            })
            .collect();
        let received = decode_all(&multiplex(sent.clone()));
        for (lane, frames) in sent.into_iter().enumerate() {
            let stream = interior(u8::try_from(lane + 1).unwrap());
            let carried: Vec<Frame> = received
                .iter()
                .filter(|(on, _)| *on == stream)
                .map(|(_, frame)| frame.clone())
                .collect();
            let mut expected: Vec<Frame> = frames.into_iter().map(Frame::from).collect();
            if !expected.is_empty() {
                expected.push(Frame::End(End::Stream));
            }
            prop_assert_eq!(carried, expected);
        }
    }
}

/// A thin frame that is ready goes ahead of a supply run that is also
/// ready, even on an earlier lane.
///
/// Both producers place their first frame in their lanes before the
/// multiplexer first polls, so it chooses between a staged supply run on
/// the first lane and a staged match on the second.
#[test]
fn a_ready_thin_frame_precedes_a_ready_supply_run() {
    let mut multiplexer = Multiplexer::default();
    let mut bulk = multiplexer.lane(interior(1), 0);
    let mut thin = multiplexer.lane(interior(2), 0);
    let written = run_to_quiescence(async move {
        bulk.frame(frame(Kind::Supply, 1)).await;
        thin.frame(frame(Kind::Match, 2)).await;
        drop((bulk, thin));
        let mut written = Vec::new();
        multiplexer
            .run(&mut written, SPEAKER, Recorder::default())
            .await
            .expect("writing into a vector cannot fail");
        written
    })
    .expect("the multiplexer drains both lanes");
    let order: Vec<Stream> = decode_all(&written)
        .into_iter()
        .map(|(stream, _)| stream)
        .collect();
    assert_eq!(order, [interior(2), interior(1)]);
}

/// The lane the multiplexer should serve among `staged`, the frame kind
/// each lane has staged, given each class's cursor.
///
/// That is the first class in service order with a staged frame, and
/// within it the staged lane cyclically nearest at or after that class's
/// cursor, which then moves past it.
///
/// This restates [`Multiplexer::choose`] as a minimum over cyclic
/// distances rather than a forward scan.
fn modeled_choice(
    staged: &[Option<Kind>],
    cursors: &mut [usize; Class::ORDER.len()],
) -> Option<usize> {
    let count = staged.len();
    for (rank, bulk) in [(0, false), (1, true)] {
        let cursor = cursors[rank] % count;
        let nearest = (0..count)
            .filter(|&lane| staged[lane].is_some_and(|kind| (kind == Kind::Supply) == bulk))
            .min_by_key(|&lane| (lane + count - cursor) % count);
        if let Some(lane) = nearest {
            cursors[rank] = (lane + 1) % count;
            return Some(lane);
        }
    }
    None
}

proptest! {
    /// Over any sequence of staged frames on any number of lanes, each of
    /// the multiplexer's choices equals [`modeled_choice`]'s.
    ///
    /// So it serves the thin frames' class before the bulk class, and
    /// within a class serves lanes round-robin from where it last served.
    #[test]
    fn each_class_is_served_round_robin(
        rounds in (1usize..=5).prop_flat_map(|lanes| prop::collection::vec(
            prop::collection::vec(
                prop::option::of(prop::sample::select(vec![
                    Kind::Match,
                    Kind::ReplyEnd,
                    Kind::Supply,
                ])),
                lanes,
            ),
            1..24,
        )),
    ) {
        let lanes = rounds[0].len();
        let mut multiplexer = Multiplexer::default();
        let outgoing: Vec<Outgoing> = (1..=lanes)
            .map(|index| multiplexer.lane(interior(u8::try_from(index).unwrap()), 0))
            .collect();
        drop(outgoing);
        for lane in &mut multiplexer.lanes {
            lane.frames = None;
        }
        let mut cursors = [0; Class::ORDER.len()];
        let mut modeled = [0; Class::ORDER.len()];
        for staged in rounds {
            for (lane, kind) in multiplexer.lanes.iter_mut().zip(&staged) {
                lane.staged = kind.map(|kind| frame(kind, 0).into());
            }
            prop_assert_eq!(
                multiplexer.choose(&mut cursors),
                modeled_choice(&staged, &mut modeled)
            );
        }
    }
}

/// Encode `frames` as `SPEAKER` writes them, followed by `trailer`.
fn wire(frames: &[(Stream, Frame)], trailer: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for wire in frames {
        crate::tree::mirror::streaming::remote::codec::encode(SPEAKER, wire, &mut bytes)
            .expect("test frames encode");
    }
    bytes.extend_from_slice(trailer);
    bytes
}

/// Run a demultiplexer over `bytes` with lanes claimed for `claimed`, until
/// it fails or every claimed lane's receiver ends. Returns the failure, if
/// any, each lane's frames, and the pipe's lookahead byte.
#[allow(clippy::type_complexity)]
fn demultiplex(
    bytes: &[u8],
    claimed: &[Stream],
) -> (Option<PipeError>, Vec<Vec<Frame>>, Option<u8>) {
    let mut demultiplexer = Demultiplexer::default();
    let receivers: Vec<_> = claimed
        .iter()
        .map(|&stream| demultiplexer.lane(stream, 0))
        .collect();
    let mut read = PipeRead::new(bytes);
    let (failure, frames) = run_to_quiescence(async {
        let run = demultiplexer.run(
            &mut read,
            SPEAKER,
            RunBudget::from_bytes(usize::MAX),
            Recorder::default(),
        );
        let consume = futures::future::join_all(
            receivers
                .into_iter()
                .map(|receiver| receiver.collect::<Vec<Frame>>()),
        );
        match futures::future::select(pin!(run), pin!(consume)).await {
            Either::Left((failure, consume)) => {
                // The failure dropped the demultiplexer's lanes, so every
                // receiver ends.
                (Some(failure), consume.await)
            }
            Either::Right((frames, _)) => (None, frames),
        }
    })
    .expect("the demultiplexer either fails or closes every lane");
    (failure, frames, read.pending)
}

/// The demultiplexer routes each frame to its logical stream's decoder,
/// closes a stream at its end-of-stream frame without delivering it, and
/// stops at the first item that is not a frame, holding only that item's
/// first byte.
#[test]
fn frames_route_by_stream_and_the_closing_item_stays_unread() {
    let reply = |tag| Frame::from(frame(Kind::Supply, tag));
    let bytes = wire(
        &[
            (interior(1), reply(1)),
            (interior(2), reply(2)),
            (interior(1), reply(3)),
            (interior(2), Frame::End(End::Stream)),
            (interior(1), Frame::End(End::Stream)),
        ],
        &MARKER,
    );
    let (failure, frames, pending) = demultiplex(&bytes, &[interior(1), interior(2)]);
    assert!(failure.is_none(), "{failure:?}");
    assert_eq!(frames, [vec![reply(1), reply(3)], vec![reply(2)]]);
    assert_eq!(pending, Some(MARKER[0]));
}

/// The read half returns the lookahead byte before the transport, so the
/// closing exchange reads the whole closing item.
#[test]
fn the_closing_exchange_reads_the_lookahead_first() {
    let bytes = [MARKER[0], MARKER[1]];
    let marker = run_to_quiescence(async {
        let mut read = PipeRead::new(&bytes[..]);
        assert_eq!(read.peek().await.unwrap(), Some(MARKER[0]));
        assert_eq!(
            read.peek().await.unwrap(),
            Some(MARKER[0]),
            "peeking consumes nothing"
        );
        let mut marker = [0; 2];
        read.read_exact(&mut marker).await.unwrap();
        marker
    })
    .expect("reading a buffer completes");
    assert_eq!(marker, MARKER);
}

/// End of stream at a frame boundary, before any closing item, is the
/// peer's departure.
#[test]
fn end_of_stream_between_frames_is_departure() {
    let bytes = wire(&[(interior(1), Frame::from(frame(Kind::Match, 0)))], &[]);
    let (failure, _, _) = demultiplex(&bytes, &[interior(1)]);
    assert!(
        matches!(&failure, Some(PipeError::Departed(source)) if source.kind() == io::ErrorKind::UnexpectedEof),
        "{failure:?}"
    );
}

/// A frame on a logical stream no decoder was created for answers no
/// question, and fails at that frame.
#[test]
fn a_frame_on_an_unclaimed_stream_is_unasked() {
    let bytes = wire(
        &[(interior(2), Frame::from(frame(Kind::Match, 0)))],
        &MARKER,
    );
    let (failure, _, _) = demultiplex(&bytes, &[interior(1)]);
    assert!(
        matches!(&failure, Some(PipeError::Unasked { origin }) if *origin == Origin::stream(SPEAKER, interior(2))),
        "{failure:?}"
    );
}

/// An end-of-stream frame on a claimed logical stream that carried no
/// frame answers no question: a conforming peer ends only a stream it
/// started, so the lone end fails at that frame.
#[test]
fn a_lone_end_on_an_unstarted_stream_is_unasked() {
    let bytes = wire(&[(interior(1), Frame::End(End::Stream))], &MARKER);
    let (failure, _, _) = demultiplex(&bytes, &[interior(1)]);
    assert!(
        matches!(&failure, Some(PipeError::Unasked { origin }) if *origin == Origin::stream(SPEAKER, interior(1))),
        "{failure:?}"
    );
}

/// A frame after its started logical stream's end answers no question,
/// and fails at that frame: the frames before the end were delivered.
#[test]
fn a_frame_after_its_stream_ended_is_unasked() {
    let first = Frame::from(frame(Kind::Match, 0));
    let bytes = wire(
        &[
            (interior(1), first.clone()),
            (interior(1), Frame::End(End::Stream)),
            (interior(1), Frame::from(frame(Kind::Match, 1))),
        ],
        &MARKER,
    );
    let (failure, frames, _) = demultiplex(&bytes, &[interior(1)]);
    assert!(
        matches!(&failure, Some(PipeError::Unasked { origin }) if *origin == Origin::stream(SPEAKER, interior(1))),
        "{failure:?}"
    );
    assert_eq!(frames, [vec![first]]);
}

/// A closing item while a started logical stream is still open breaks the
/// pipe's order: every frame precedes the closing items.
#[test]
fn a_closing_item_before_a_started_stream_ends_is_unfinished() {
    let bytes = wire(
        &[(interior(1), Frame::from(frame(Kind::Match, 0)))],
        &MARKER,
    );
    let (failure, frames, _) = demultiplex(&bytes, &[interior(1)]);
    assert!(
        matches!(&failure, Some(PipeError::Unfinished { origin }) if *origin == Origin::stream(SPEAKER, interior(1))),
        "{failure:?}"
    );
    assert_eq!(
        frames[0].len(),
        1,
        "the frame before the item was delivered"
    );
}

/// A lane that never started closes cleanly at the closing item: its
/// decoder, if it still expected a reply, finds its stream ended.
#[test]
fn an_unstarted_stream_closes_at_the_closing_item() {
    let (failure, frames, pending) = demultiplex(&MARKER, &[interior(1)]);
    assert!(failure.is_none(), "{failure:?}");
    assert_eq!(frames, [Vec::<Frame>::new()]);
    assert_eq!(pending, Some(MARKER[0]));
}
