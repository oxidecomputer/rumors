//! The pipe's multiplexer and demultiplexer, driven against in-memory byte
//! buffers under the closed-world poller.

use crate::tree::mirror::streaming::message::LaterMatches;
use std::collections::VecDeque;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Wake, Waker};

use futures::{StreamExt, future::Either};
use proptest::prelude::*;
use tokio::io::{AsyncReadExt, BufReader};

use super::*;
use crate::Version;
use crate::message::Message;
use crate::testing::{IoPlan, IoSide, Quiescence, run_to_quiescence, wrap_io};
use crate::tree::mirror::streaming::remote::codec::{CUTS_TO_GROW, Flow, LeafRun, encode};
use crate::tree::mirror::streaming::{holding::Holding, ledger::Ledger};

/// The speaker whose direction these tests write and read.
const SPEAKER: Speaker = Speaker::Initiator;

/// The completion marker that closes a session: a CBOR text item.
const MARKER: [u8; 2] = crate::testing::COMPLETION_MARKER;

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

/// Build one reply frame of `kind` whose content names `tag`, so frames of one
/// kind stay distinguishable after decoding.
fn frame(kind: Kind, tag: u64) -> ReplyFrame {
    match kind {
        Kind::Match => ReplyFrame::reaction(Reaction::Match(LaterMatches::NONE), Flow::End),
        Kind::ReplyEnd => ReplyFrame::reply_end(),
        Kind::Supply => {
            let mut run = LeafRun::new();
            run.push(&Version::new(), &Message::new(tag), RunBudget::default())
                .expect("one small record fits a fresh run");
            ReplyFrame::reaction(Reaction::Supply(run), Flow::End)
        }
    }
}

/// `frame` on its way to the multiplexer with no charge.
fn bare(frame: Frame) -> Outbound {
    Outbound {
        frame,
        charge: Charge::default(),
        permit: WirePermit::default(),
    }
}

/// An interior logical stream, where every reaction form is admissible for
/// either speaker.
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
        out.frame(frame, Charge::default(), WirePermit::default())
            .await;
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
        let observe = SessionHandle::default();
        let run = multiplexer.run(&mut written, SPEAKER, Recorder::default(), &observe);
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

/// How a frame's length stands against the multiplexer's write buffer.
#[derive(Clone, Copy, Debug)]
enum Size {
    /// A thin frame of a few bytes, copied into the buffer.
    Thin,
    /// A supply frame just over half the buffer: copied, but two of them
    /// overflow it.
    Half,
    /// A supply frame one byte short of the buffer: the largest copied.
    Below,
    /// A supply frame exactly the buffer's size: the smallest written directly.
    At,
    /// A supply frame one byte over the buffer: written directly.
    Above,
}

impl Size {
    /// The encoded length of a supply frame of this size, or `None` for a thin
    /// frame.
    fn supply_len(self) -> Option<usize> {
        match self {
            Self::Thin => None,
            Self::Half => Some(WRITE_BUFFER_BYTES / 2 + 1),
            Self::Below => Some(WRITE_BUFFER_BYTES - 1),
            Self::At => Some(WRITE_BUFFER_BYTES),
            Self::Above => Some(WRITE_BUFFER_BYTES + 1),
        }
    }
}

/// The length of `frame`'s encoding on `stream`.
fn encoded_len(stream: Stream, frame: &ReplyFrame) -> usize {
    let mut bytes = Vec::new();
    encode(SPEAKER, &(stream, Frame::from(frame.clone())), &mut bytes)
        .expect("a test frame encodes");
    bytes.len()
}

/// A supply frame whose encoding on `stream` is exactly `len` bytes: one record
/// whose payload is a string of the length that makes it so.
///
/// Near the buffer's size every head in the frame has a fixed width, so the
/// encoding grows byte for byte with the string, and one correction from a
/// first guess lands exactly.
fn supply_of_len(stream: Stream, len: usize) -> ReplyFrame {
    let build = |text: usize| {
        let mut run = LeafRun::new();
        run.push(
            &Version::new(),
            &Message::new("x".repeat(text)),
            RunBudget::default(),
        )
        .expect("a record within the run cap fits a fresh run");
        ReplyFrame::reaction(Reaction::Supply(run), Flow::End)
    };
    let guess = len / 2;
    let text = guess + len - encoded_len(stream, &build(guess));
    let frame = build(text);
    assert_eq!(
        encoded_len(stream, &frame),
        len,
        "the corrected guess lands exactly"
    );
    frame
}

/// A write half's destination the test can watch.
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<SinkState>>);

/// What has reached a [`Sink`], and who waits for more.
#[derive(Default)]
struct SinkState {
    /// The bytes written to the sink.
    bytes: Vec<u8>,
    /// The waker of a task waiting for more bytes.
    waker: Option<Waker>,
    /// How many bytes had been written at each flush, in order.
    flushes: Vec<usize>,
    /// Whether each flush, in order, waits once before completing; flushes past
    /// the end of the script complete at once.
    waits: VecDeque<bool>,
    /// Whether the next flush has already waited once.
    waited: bool,
}

impl Sink {
    /// A sink whose flushes wait once or not, in order, as `waits` says.
    fn waiting(waits: &[bool]) -> Self {
        let sink = Self::default();
        sink.0.lock().expect("sink lock").waits = waits.iter().copied().collect();
        sink
    }

    /// Wait until at least `len` bytes have reached the sink.
    async fn wait_for(&self, len: usize) {
        poll_fn(|cx| {
            let mut sink = self.0.lock().expect("sink lock");
            if sink.bytes.len() >= len {
                Poll::Ready(())
            } else {
                sink.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        })
        .await;
    }

    /// The bytes that have reached the sink.
    fn bytes(&self) -> Vec<u8> {
        self.0.lock().expect("sink lock").bytes.clone()
    }

    /// The bytes each flush carried: those written since the flush before it,
    /// in order, omitting flushes that carried nothing.
    fn flushed(&self) -> Vec<usize> {
        let sink = self.0.lock().expect("sink lock");
        let mut previous = 0;
        let mut groups = Vec::new();
        for &at in &sink.flushes {
            if at > previous {
                groups.push(at - previous);
            }
            previous = at;
        }
        groups
    }
}

/// Take every write whole, waking the waiting task, and complete each flush
/// once, or after one wait where the script says so.
impl AsyncWrite for Sink {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut sink = self.0.lock().expect("sink lock");
        sink.bytes.extend_from_slice(bytes);
        if let Some(waker) = sink.waker.take() {
            waker.wake();
        }
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut sink = self.0.lock().expect("sink lock");
        if sink.waits.front() == Some(&true) && !mem::replace(&mut sink.waited, true) {
            // A full socket would wake the writer once it drains; this one
            // drains at once.
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        sink.waits.pop_front();
        sink.waited = false;
        let at = sink.bytes.len();
        sink.flushes.push(at);
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

proptest! {
    /// Every frame the multiplexer writes reaches the write half whole, in
    /// order, and flushed before the multiplexer waits for the next one,
    /// at every write batch range.
    ///
    /// That holds whatever a frame's length against the write buffer, and
    /// however the write half takes writes.
    ///
    /// Each bound of the range is drawn as in
    /// `flushes_keep_batches_within_their_ceilings`, so ranges that turn
    /// batching across passes off, and an unbounded end, which leaves a
    /// batch of large frames no ceiling, both occur: whatever the range, a
    /// pass that brings no frame must flush.
    ///
    /// Each batch mixes thin frames with supply frames just below, at, and
    /// just above [`WRITE_BUFFER_BYTES`], so some frames are copied and some
    /// pass the buffer, and frames over half of it, two of which overflow it
    /// when the multiplexer writes them between the same two flushes; how
    /// often that happens depends on scheduling, which this test does not
    /// control or count. The write half takes
    /// at most `write_chunk` bytes per write, vectored or first slice only,
    /// delays some writes and flushes, and holds what it takes until a
    /// flush. The producer hands over each batch only once the previous
    /// batches have reached the sink, so if the multiplexer waited while a
    /// frame it wrote sat in its buffer or unflushed, the run would stall
    /// with nothing left to wake it.
    #[test]
    fn written_frames_are_flushed_before_the_multiplexer_waits(
        batches in prop::collection::vec(
            prop::collection::vec(
                prop::sample::select(vec![Size::Thin, Size::Half, Size::Below, Size::At, Size::Above]),
                1..5,
            ),
            1..4,
        ),
        write_chunk in prop_oneof![Just(usize::MAX), 1_000usize..=70_000],
        delays in prop::collection::vec(0u8..=2, 0..16),
        vectored in any::<bool>(),
        start in batch_bound(),
        end in batch_bound(),
    ) {
        let stream = interior(1);
        let mut supplies = std::collections::HashMap::new();
        let batches: Vec<Vec<ReplyFrame>> = batches
            .iter()
            .map(|sizes| {
                sizes
                    .iter()
                    .map(|size| match size.supply_len() {
                        None => frame(Kind::Match, 0),
                        Some(len) => supplies
                            .entry(len)
                            .or_insert_with(|| supply_of_len(stream, len))
                            .clone(),
                    })
                    .collect()
            })
            .collect();
        let flushed_after: Vec<usize> = batches
            .iter()
            .scan(0, |sent, batch| {
                *sent += batch.iter().map(|frame| encoded_len(stream, frame)).sum::<usize>();
                Some(*sent)
            })
            .collect();
        let sink = Sink::default();
        let plan = IoPlan {
            write_chunk,
            write_delays: delays.clone(),
            flush_delays: delays,
            hold_until_flush: true,
            vectored,
            ..IoPlan::default()
        };
        let (_, mut write, _) = wrap_io(IoSide::Left, plan, tokio::io::empty(), sink.clone());
        let mut multiplexer = Multiplexer::new(WriteBatch::from_range((start, end)));
        let mut lane = multiplexer.lane(stream, 0);
        let (handed, sink_ref) = (&batches, &sink);
        let producer = async move {
            for (batch, &sent) in handed.iter().zip(&flushed_after) {
                for frame in batch {
                    lane.frame(frame.clone(), Charge::default(), WirePermit::default()).await;
                }
                sink_ref.wait_for(sent).await;
            }
            lane.finish().await;
        };
        let written = run_to_quiescence(async {
            let observe = SessionHandle::default();
            let run = multiplexer.run(&mut write, SPEAKER, Recorder::default(), &observe);
            futures::join!(run, producer).0
        })
        .map_err(|stopped| TestCaseError::fail(format!(
            "the multiplexer waited with a written frame unflushed: {stopped:?}",
        )))?;
        prop_assert!(written.is_ok(), "{written:?}");
        let mut expected: Vec<(Stream, Frame)> = batches
            .into_iter()
            .flatten()
            .map(|frame| (stream, Frame::from(frame)))
            .collect();
        expected.push((stream, Frame::End(End::Stream)));
        prop_assert_eq!(decode_all(&sink.bytes()), expected);
    }
}

/// A multiplexer fed a thin frame on every pass, at the default write batch
/// range, first flushes once its batch reaches the segment size, long before
/// its buffer fills.
///
/// The producer shares one future with the multiplexer and keeps its lane full,
/// so every poll that follows a yield finds a frame, and no pass ever brings
/// nothing: only the batch's ceiling can end the batch. A thin frame fits a
/// segment and nothing has waited yet, so the ceiling is the segment size. The
/// multiplexer flushes before the frame that would push the batch past it, so
/// the first flush carries more than the segment size less one frame, and at
/// most the segment size. That is more frames than one pass hands it, a lane's
/// worth at most, so the batch grew across passes, and far fewer bytes than the
/// buffer holds, as the first assertion checks, so the ceiling, not the buffer,
/// ended it.
#[test]
fn a_multiplexer_fed_every_pass_flushes_at_the_segment_size() {
    let stream = interior(1);
    let thin = frame(Kind::Match, 0);
    let thin_len = encoded_len(stream, &thin);
    let batch = WriteBatch::default().segment();
    assert!(
        batch < WRITE_BUFFER_BYTES / 2,
        "a batch fits in the write buffer with room to spare, so only the ceiling flushes it"
    );
    let sink = Sink::default();
    let mut write = sink.clone();
    let mut multiplexer = Multiplexer::default();
    let mut lane = multiplexer.lane(stream, 0);
    run_to_quiescence(async {
        let producer = async {
            while sink.bytes().is_empty() {
                lane.frame(thin.clone(), Charge::default(), WirePermit::default())
                    .await;
            }
            drop(lane);
        };
        let observe = SessionHandle::default();
        let run = multiplexer.run(&mut write, SPEAKER, Recorder::default(), &observe);
        let (written, ()) = futures::join!(run, producer);
        written.expect("writing into the sink cannot fail");
    })
    .expect("the multiplexer drains its lane once the producer stops");
    let first = sink.flushed()[0];
    assert!(
        first > batch - thin_len && first <= batch,
        "the first flush carried {first} bytes, against a segment size of {batch}"
    );
    assert!(first / thin_len > LANE_CAPACITY, "{first} bytes");
}

/// One bound of a write batch range: included, excluded, or unbounded.
///
/// Its sizes run from zero through the default range's ends to past the write
/// buffer's capacity.
fn batch_bound() -> impl Strategy<Value = Bound<usize>> {
    let size = prop_oneof![
        Just(0usize),
        1usize..64,
        64usize..4_000,
        Just(*crate::default::tls::WRITE_BATCH_RANGE.start()),
        Just(*crate::default::tls::WRITE_BATCH_RANGE.end()),
        Just(WRITE_BUFFER_BYTES),
        Just(WRITE_BUFFER_BYTES * 2),
    ];
    prop_oneof![
        size.clone().prop_map(Bound::Included),
        size.prop_map(Bound::Excluded),
        Just(Bound::Unbounded),
    ]
}

/// The ceiling under `batch` of a batch whose first frame is `first` bytes,
/// when writing out the batch before it did not wait and the `cuts` batches in
/// a row before it were cut at their ceiling.
///
/// It is the segment size if the frame fits it and `cuts` is under
/// [`CUTS_TO_GROW`], and otherwise the largest batch, or `None` for no ceiling.
fn unwaited_ceiling(batch: WriteBatch, first: usize, cuts: usize) -> Option<usize> {
    if first <= batch.segment() && cuts < CUTS_TO_GROW {
        Some(batch.segment())
    } else {
        batch.largest().map(NonZeroUsize::get)
    }
}

proptest! {
    /// Over a write half that never waits, every flush of a multiplexer
    /// carries whole frames within its batch's ceiling, or a single frame
    /// larger than it, whatever the write batch range and frame lengths.
    ///
    /// A batch that starts with a frame that fits a segment, after fewer
    /// than [`CUTS_TO_GROW`] batches in a row cut at their ceiling, stays
    /// within the segment size, so the frames batched before a larger
    /// frame are flushed before it; any other batch stays within the
    /// largest batch. A batch was cut at its ceiling only if the frame
    /// after it would not have fitted beside it under that ceiling, so only
    /// if it would not have fitted under the smallest ceiling the batch
    /// could have had: the segment size when its first frame fits it. The
    /// test counts the batches in a row that may have been cut by that
    /// measure, which are at least as many as were, and predicts the
    /// segment size wherever fewer than [`CUTS_TO_GROW`] precede a batch,
    /// where the multiplexer must have used it; the largest batch bounds
    /// every other flush.
    ///
    /// The frames mix thin frames with supply frames from tiny to several
    /// times the larger segment sizes, all smaller than the write buffer.
    /// Each bound of the range is drawn included, excluded, or unbounded,
    /// from zero through the default range's ends to past the buffer's
    /// capacity, so empty ranges, starts above ends, saturating ends and
    /// an unbounded end, which leaves a large frame's batch no ceiling,
    /// all occur.
    #[test]
    fn flushes_keep_batches_within_their_ceilings(
        texts in prop::collection::vec(prop::option::of(0usize..3_000), 1..60),
        start in batch_bound(),
        end in batch_bound(),
    ) {
        let stream = interior(1);
        let frames: Vec<ReplyFrame> = texts
            .iter()
            .map(|text| match text {
                None => frame(Kind::Match, 0),
                Some(text) => {
                    let mut run = LeafRun::new();
                    run.push(&Version::new(), &Message::new("x".repeat(*text)), RunBudget::default())
                        .expect("a record within the run cap fits a fresh run");
                    ReplyFrame::reaction(Reaction::Supply(run), Flow::End)
                }
            })
            .collect();
        let lens: Vec<usize> = frames.iter().map(|frame| encoded_len(stream, frame)).collect();
        let batch = WriteBatch::from_range((start, end));
        let sink = Sink::default();
        let mut write = sink.clone();
        let mut multiplexer = Multiplexer::new(batch);
        let lane = multiplexer.lane(stream, 0);
        run_to_quiescence(async {
            let observe = SessionHandle::default();
            let run = multiplexer.run(&mut write, SPEAKER, Recorder::default(), &observe);
            let (written, ()) = futures::join!(run, produce(lane, frames));
            written.expect("writing into the sink cannot fail");
        })
        .expect("the multiplexer drains its lane");
        let mut end = Vec::new();
        encode(SPEAKER, &(stream, Frame::End(End::Stream)), &mut end)
            .expect("an end-of-stream frame encodes");
        let end_len = end.len();
        let mut frames = lens.iter().copied().chain(std::iter::once(end_len)).peekable();
        let mut possible_cuts = 0;
        for group in sink.flushed() {
            let mut carried = Vec::new();
            while carried.iter().sum::<usize>() < group {
                carried.push(frames.next().expect("a flush carries only written frames"));
            }
            prop_assert_eq!(carried.iter().sum::<usize>(), group, "a flush ends at a frame boundary");
            let ceiling = unwaited_ceiling(batch, carried[0], possible_cuts);
            let smallest = unwaited_ceiling(batch, carried[0], 0);
            let possibly_cut =
                smallest.is_some_and(|smallest| frames.peek().is_some_and(|&next| group + next > smallest));
            possible_cuts = if possibly_cut { (possible_cuts + 1).min(CUTS_TO_GROW) } else { 0 };
            prop_assert!(
                ceiling.is_none_or(|ceiling| group <= ceiling) || carried.len() == 1,
                "a flush of {} bytes in {} frames, against a ceiling of {:?} under {:?}",
                group,
                carried.len(),
                ceiling,
                batch
            );
        }
        prop_assert!(frames.next().is_none(), "every frame was flushed");
    }

    /// A batch after one that waited on the write half, or after
    /// [`CUTS_TO_GROW`] batches in a row cut at their ceiling, grows to the
    /// largest batch; any other batch keeps to the segment size.
    ///
    /// A batch after a single cut keeps to the segment size too.
    /// A batch *ended idle* when it was flushed because nothing more was
    /// ready, which ends a run of cuts. The producer shares one future with
    /// the multiplexer and hands over bursts of thin frames, all of one
    /// length, waiting after each burst until all of it has reached the
    /// sink. Within a burst it keeps the lane full, as in
    /// `a_multiplexer_fed_every_pass_flushes_at_the_segment_size`, so only
    /// a batch's ceiling ends a flush before the burst's end: such a flush
    /// is a cut, and carries more than its ceiling less one frame, and at
    /// most the ceiling. The flush that ends a burst ended idle, unless it
    /// filled its ceiling exactly, which cuts it. The write half's flushes
    /// wait once or not as a drawn script says. Each flush's ceiling
    /// follows from the run of cuts before it and whether the flush before
    /// it waited; the first batch keeps to the segment size. Bursts run
    /// from one frame to many segments' worth, so runs of no cut, one cut,
    /// and many cuts all end in idle flushes.
    #[test]
    fn a_waiting_flush_or_a_run_of_cuts_lets_the_next_batch_grow_to_the_largest(
        bursts in prop::collection::vec(1usize..=1_000, 1..6),
        waits in prop::collection::vec(any::<bool>(), 0..16),
        segment in 64usize..=600,
        growth in 1usize..=4,
    ) {
        let stream = interior(1);
        let thin = frame(Kind::Match, 0);
        let thin_len = encoded_len(stream, &thin);
        let largest = segment * growth;
        let batch = WriteBatch::from_range(segment..=largest);
        let sink = Sink::waiting(&waits);
        let mut write = sink.clone();
        let mut multiplexer = Multiplexer::new(batch);
        let mut lane = multiplexer.lane(stream, 0);
        let ends: Vec<usize> = bursts
            .iter()
            .scan(0, |sent, &frames| {
                *sent += frames * thin_len;
                Some(*sent)
            })
            .collect();
        let (bursts_ref, ends_ref, sink_ref) = (&bursts, &ends, &sink);
        run_to_quiescence(async {
            let producer = async move {
                for (&frames, &end) in bursts_ref.iter().zip(ends_ref) {
                    for _ in 0..frames {
                        lane.frame(thin.clone(), Charge::default(), WirePermit::default()).await;
                    }
                    sink_ref.wait_for(end).await;
                }
                drop(lane);
            };
            let observe = SessionHandle::default();
            let run = multiplexer.run(&mut write, SPEAKER, Recorder::default(), &observe);
            let (written, ()) = futures::join!(run, producer);
            written.expect("writing into the sink cannot fail");
        })
        .expect("the multiplexer drains its lane once the producer stops");
        // The closing end-of-stream frame's own flush follows the bursts.
        let flushed = sink.flushed();
        let (mut sent, mut cuts, mut waited) = (0, 0, false);
        for (index, &group) in flushed.iter().enumerate() {
            if sent == *ends.last().expect("at least one burst") {
                break;
            }
            let ceiling = if cuts >= CUTS_TO_GROW || waited { largest } else { segment };
            sent += group;
            let ends_burst = ends.contains(&sent);
            prop_assert!(
                group <= ceiling && (ends_burst || group > ceiling - thin_len),
                "flush {} carried {} bytes, against a ceiling of {} (segment {}, largest {}, \
                 ends a burst: {}): {:?}",
                index,
                group,
                ceiling,
                segment,
                largest,
                ends_burst,
                flushed
            );
            let cut = !ends_burst || group >= ceiling;
            cuts = if cut { (cuts + 1).min(CUTS_TO_GROW) } else { 0 };
            waited = waits.get(index).copied().unwrap_or(false);
        }
        prop_assert_eq!(sent, *ends.last().expect("at least one burst"), "every burst was flushed");
    }
}

/// A write half that takes every write whole and logs each flush that delivers
/// bytes, with the pass of the test's clock it completed in.
struct PassSink {
    /// The pass the test's future is in: how many times it has been polled.
    clock: Arc<AtomicUsize>,
    /// The bytes written since the last flush.
    pending: usize,
    /// Each delivering flush's pass and bytes, in order.
    flushes: Arc<Mutex<Vec<(usize, usize)>>>,
}

/// Take every write whole, and log every flush that delivers bytes.
impl AsyncWrite for PassSink {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.get_mut().pending += bytes.len();
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let sink = self.get_mut();
        if sink.pending > 0 {
            let pass = sink.clock.load(Ordering::Relaxed);
            let bytes = mem::take(&mut sink.pending);
            sink.flushes
                .lock()
                .expect("flush log lock")
                .push((pass, bytes));
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Drive a multiplexer batching within `batch` while a producer hands it
/// `frames` thin frames, one every `gap` passes, counting passes on a clock.
///
/// The future polls the clock, then the multiplexer, then the producer, so each
/// of its polls is one pass, and the multiplexer polls ahead of the producer
/// within a pass. Once it has handed over its last frame, the producer waits
/// for every frame's flush before closing its lane, since a closing lane would
/// end the last batch at once.
///
/// Returns a thin frame's length, the pass in which the producer handed over
/// each frame, and the pass and bytes of each flush that delivered bytes.
fn paced(
    batch: WriteBatch,
    frames: usize,
    gap: usize,
) -> Result<(usize, Vec<usize>, Vec<(usize, usize)>), Quiescence> {
    let stream = interior(1);
    let thin = frame(Kind::Match, 0);
    let len = encoded_len(stream, &thin);
    let clock = Arc::new(AtomicUsize::new(0));
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut write = PassSink {
        clock: Arc::clone(&clock),
        pending: 0,
        flushes: Arc::clone(&log),
    };
    let mut multiplexer = Multiplexer::new(batch);
    let mut lane = multiplexer.lane(stream, 0);
    let (clock_ref, log_ref) = (&clock, &log);
    let flushed = || -> usize {
        log_ref
            .lock()
            .expect("flush log lock")
            .iter()
            .map(|&(_, bytes)| bytes)
            .sum()
    };
    let producer = async move {
        let mut handed = Vec::with_capacity(frames);
        for _ in 0..frames {
            handed.push(clock_ref.load(Ordering::Relaxed));
            lane.frame(thin.clone(), Charge::default(), WirePermit::default())
                .await;
            for _ in 0..gap {
                yield_to_members().await;
            }
        }
        while flushed() < frames * len {
            yield_to_members().await;
        }
        drop(lane);
        handed
    };
    let tick = poll_fn(|_| {
        clock_ref.fetch_add(1, Ordering::Relaxed);
        Poll::<()>::Pending
    });
    let handed = run_to_quiescence(async {
        let observe = SessionHandle::default();
        let run = multiplexer.run(&mut write, SPEAKER, Recorder::default(), &observe);
        let work = async { futures::join!(run, producer) };
        match futures::future::select(pin!(tick), pin!(work)).await {
            Either::Right(((written, handed), _)) => {
                written.expect("writing into the sink cannot fail");
                handed
            }
            Either::Left(((), _)) => unreachable!("the clock never completes"),
        }
    })?;
    let flushes = log.lock().expect("flush log lock").clone();
    Ok((len, handed, flushes))
}

/// The pass and bytes of each flush a multiplexer batching within `batch`
/// makes, over a write half that never waits, when it writes frames of `len`
/// bytes in the passes `written`, and nothing else arrives.
///
/// A batch ends idle [`EMPTY_PASSES`] passes after its last write when no frame
/// is written in those passes, and is cut at its ceiling in the pass that
/// writes the frame reaching the ceiling, or the frame that would pass it.
fn modeled_flushes(
    batch: WriteBatch,
    len: usize,
    written: impl IntoIterator<Item = usize>,
) -> Vec<(usize, usize)> {
    let mut flushes = Vec::new();
    let (mut bytes, mut cuts, mut ceiling, mut last) = (0, 0, None, 0);
    for written in written {
        if bytes > 0 && written > last + EMPTY_PASSES {
            flushes.push((last + EMPTY_PASSES, bytes));
            (bytes, cuts) = (0, 0);
        }
        if bytes > 0 && ceiling.is_some_and(|ceiling| bytes + len > ceiling) {
            flushes.push((written, bytes));
            (bytes, cuts) = (0, (cuts + 1).min(CUTS_TO_GROW));
        }
        if bytes == 0 {
            ceiling = unwaited_ceiling(batch, len, cuts);
        }
        bytes += len;
        last = written;
        if ceiling.is_some_and(|ceiling| bytes >= ceiling) {
            flushes.push((written, bytes));
            (bytes, cuts) = (0, (cuts + 1).min(CUTS_TO_GROW));
        }
    }
    if bytes > 0 {
        flushes.push((last + EMPTY_PASSES, bytes));
    }
    flushes
}

proptest! {
    /// The multiplexer flushes a batch [`EMPTY_PASSES`] passes after its
    /// last write when those passes bring no frame, and keeps it open
    /// across any shorter gap.
    ///
    /// So each frame of a batch adds at most [`EMPTY_PASSES`] passes of the
    /// session's task set to the wait of the batch's first frame for its
    /// flush.
    ///
    /// A producer hands over a thin frame every `gap` passes ([`paced`]).
    /// The multiplexer polls ahead of the producer within a pass, so it
    /// writes each frame in the pass after the frame is handed over, and
    /// finds a frame handed over within [`EMPTY_PASSES`] passes of its last
    /// write in one of the polls that follow its yields. Every flush must
    /// match, in pass and in bytes, the model of the rule
    /// ([`modeled_flushes`]). An idle flush comes a fixed, nonzero number
    /// of passes after its last write, so the match also shows that the
    /// clock counts passes. Gaps run past [`EMPTY_PASSES`], ceilings run
    /// from below one frame to many, and an unbounded end leaves a grown
    /// batch no ceiling, so idle flushes, cuts and frames flushed alone all
    /// occur.
    ///
    /// Every flush must also meet the per-frame bound behind the one
    /// [`Multiplexer::run`] documents: it comes within [`EMPTY_PASSES`]
    /// passes per frame it carries of its first frame's write, and a frame
    /// takes at least a byte of its ceiling.
    #[test]
    fn a_batch_is_flushed_once_empty_passes_bring_no_frame(
        frames in 1usize..=200,
        gap in 1usize..=2 * EMPTY_PASSES,
        segment in 1usize..=300,
        growth in prop::option::of(1usize..=4),
    ) {
        let batch = WriteBatch::from_range((
            Bound::Included(segment),
            growth.map_or(Bound::Unbounded, |growth| Bound::Included(segment * growth)),
        ));
        let (len, handed, flushes) = paced(batch, frames, gap)
            .map_err(|stopped| TestCaseError::fail(format!("the multiplexer stopped: {stopped:?}")))?;
        let written: Vec<usize> = handed.iter().map(|&pass| pass + 1).collect();
        prop_assert_eq!(
            &flushes,
            &modeled_flushes(batch, len, written.iter().copied()),
            "{:?} at a gap of {} passes",
            batch,
            gap
        );
        let mut first = 0;
        for &(pass, bytes) in &flushes {
            let carried = bytes / len;
            let waited = pass - written[first];
            prop_assert!(
                waited <= EMPTY_PASSES * carried,
                "a flush of {} frames came {} passes after its first frame's write",
                carried,
                waited
            );
            first += carried;
        }
    }
}

/// A thin frame that is ready goes ahead of a supply run that is also ready,
/// even on an earlier lane.
///
/// Both producers place their first frame in their lanes before the multiplexer
/// first polls, so it chooses between a staged supply run on the first lane and
/// a staged match on the second.
#[test]
fn a_ready_thin_frame_precedes_a_ready_supply_run() {
    let mut multiplexer = Multiplexer::default();
    let mut bulk = multiplexer.lane(interior(1), 0);
    let mut thin = multiplexer.lane(interior(2), 0);
    let written = run_to_quiescence(async move {
        bulk.frame(
            frame(Kind::Supply, 1),
            Charge::default(),
            WirePermit::default(),
        )
        .await;
        thin.frame(
            frame(Kind::Match, 2),
            Charge::default(),
            WirePermit::default(),
        )
        .await;
        drop((bulk, thin));
        let mut written = Vec::new();
        let observe = SessionHandle::default();
        multiplexer
            .run(&mut written, SPEAKER, Recorder::default(), &observe)
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

/// A write half that records, at each write it takes, how many bytes it had
/// taken before it and how many bytes `ledger` held then.
struct LedgerWatch<'a> {
    /// The ledger whose holdings each write records.
    ledger: &'a Ledger,
    /// The bytes taken so far.
    taken: usize,
    /// Each write's offset and the ledger's holdings at it, in order.
    writes: Vec<(usize, usize)>,
}

/// Take every write whole, recording the ledger's holdings first.
impl AsyncWrite for LedgerWatch<'_> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let watch = self.get_mut();
        if !bytes.is_empty() {
            watch.writes.push((watch.taken, watch.ledger.held()));
        }
        watch.taken += bytes.len();
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

proptest! {
    /// Each frame's charge returns once the multiplexer has written the
    /// frame, and not before.
    ///
    /// The write half takes every byte of a frame, and of every earlier
    /// frame, while the frame's charge is held, and every byte of a later
    /// frame after it has returned. So a chunk's charge, which rides its
    /// last frame, outlives every frame of the chunk and nothing after it.
    ///
    /// Every frame is one supply run whose record is over twice
    /// [`WRITE_BUFFER_BYTES`] long, so each passes the write buffer during
    /// its own write and none of its bytes wait there afterwards. At each write, the ledger therefore
    /// holds exactly the charges of the frame being written and of the
    /// frames after it; charges of zero bytes stand for frames that carry
    /// none.
    #[test]
    fn a_charge_returns_once_its_frame_is_written(
        charges in prop::collection::vec(0..=64usize, 1..5),
    ) {
        let stream = interior(1);
        let mut run = LeafRun::new();
        run.push(
            &Version::new(),
            &Message::new("x".repeat(2 * WRITE_BUFFER_BYTES)),
            RunBudget::default(),
        )
        .expect("a record within the run cap fits a fresh run");
        let frame = ReplyFrame::reaction(Reaction::Supply(run), Flow::Continue);
        let len = encoded_len(stream, &frame);
        let ledger = Ledger::default();
        let items: Vec<(ReplyFrame, Charge)> = charges
            .iter()
            .map(|&bytes| (frame.clone(), ledger.charge(Holding::OutgoingReplies, bytes)))
            .collect();
        let mut multiplexer = Multiplexer::default();
        let mut lane = multiplexer.lane(stream, 0);
        let mut write = LedgerWatch {
            ledger: &ledger,
            taken: 0,
            writes: Vec::new(),
        };
        run_to_quiescence(async {
            // Dropping the lane closes it without an end-of-stream frame,
            // so the write half takes the charged frames alone.
            let producer = async move {
                for (frame, charge) in items {
                    lane.frame(frame, charge, WirePermit::default()).await;
                }
            };
            let observe = SessionHandle::default();
            let run = multiplexer.run(&mut write, SPEAKER, Recorder::default(), &observe);
            futures::join!(run, producer).0
        })
        .expect("the multiplexer drains its lane")
        .expect("the watch takes every write");
        prop_assert_eq!(write.taken, len * charges.len());
        for &(at, held) in &write.writes {
            let unwritten: usize = charges[at / len..].iter().sum();
            prop_assert_eq!(held, unwritten, "the write at byte {}", at);
        }
        prop_assert_eq!(ledger.held(), 0, "every charge returns once written");
    }
}

/// The lane the multiplexer should serve among `staged`, the frame kind each
/// lane has staged, given each class's cursor.
///
/// That is the first class in service order with a staged frame, and within it
/// the staged lane cyclically nearest at or after that class's cursor, which
/// then moves past it.
///
/// This restates [`Multiplexer::choose`] as a minimum over cyclic distances
/// rather than a forward scan.
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
                lane.staged = kind.map(|kind| bare(frame(kind, 0).into()));
            }
            prop_assert_eq!(
                multiplexer.choose(&mut cursors),
                modeled_choice(&staged, &mut modeled)
            );
        }
    }
}

/// One step of a [`polls_match_the_full_scan`] case.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Hand a frame of this kind to the lane at this index, modulo the lane
    /// count, if its channel is open and has room: an encoder whose send
    /// completes.
    Send(usize, Kind),
    /// Drop the sender of the lane at this index, modulo the lane count: an
    /// encoder that finishes.
    Close(usize),
    /// Poll the multiplexer once, as a wake of its task does, whether or not
    /// anything woke it.
    Poll,
}

/// One lane as a multiplexer that polls every lane's channel sees it.
#[derive(Default)]
struct ModeledLane {
    /// The frames in the lane's channel, oldest first: at most
    /// [`LANE_CAPACITY`].
    queued: VecDeque<(Kind, Frame)>,
    /// Whether the lane's sender has gone.
    closed: bool,
    /// Whether a poll has found the channel closed and empty.
    ended: bool,
    /// The frame taken from the channel and awaiting its turn.
    staged: Option<(Kind, Frame)>,
}

/// A multiplexer that polls every lane's channel on every poll: the reference
/// [`polls_match_the_full_scan`] holds the multiplexer to.
struct FullScan {
    /// Every lane, in creation order.
    lanes: Vec<ModeledLane>,
    /// Each class's round-robin cursor, as [`modeled_choice`] keeps it.
    cursors: [usize; Class::ORDER.len()],
}

impl FullScan {
    /// Whether a poll now would stage a frame or observe a close: what a
    /// multiplexer that returned `Pending` must be woken for.
    fn has_news(&self) -> bool {
        self.lanes.iter().any(|lane| {
            lane.staged.is_none() && !lane.ended && (!lane.queued.is_empty() || lane.closed)
        })
    }

    /// Stage every lane's oldest waiting frame, observe every close, and return
    /// the frame to write as [`Multiplexer::poll_next`] would.
    ///
    /// That is the lane's index and the frame, `Ready(None)` once every lane
    /// has ended with nothing staged, and `Pending` otherwise.
    fn poll(&mut self) -> Poll<Option<(usize, Frame)>> {
        for lane in &mut self.lanes {
            if lane.staged.is_none() && !lane.ended {
                match lane.queued.pop_front() {
                    Some(frame) => lane.staged = Some(frame),
                    None => lane.ended = lane.closed,
                }
            }
        }
        let staged: Vec<Option<Kind>> = self
            .lanes
            .iter()
            .map(|lane| lane.staged.as_ref().map(|(kind, _)| *kind))
            .collect();
        match modeled_choice(&staged, &mut self.cursors) {
            Some(index) => {
                let (_, frame) = self.lanes[index]
                    .staged
                    .take()
                    .expect("chosen lanes are staged");
                Poll::Ready(Some((index, frame)))
            }
            None if self.lanes.iter().all(|lane| lane.ended) => Poll::Ready(None),
            None => Poll::Pending,
        }
    }
}

/// Counts the wakes of the task a test polls the multiplexer in.
#[derive(Default)]
struct WakeCount(AtomicUsize);

/// Count each wake.
impl Wake for WakeCount {
    /// Counts the wake, then drops this reference.
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Counts the wake.
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

/// The frame kinds a step can send.
fn kind() -> impl Strategy<Value = Kind> {
    prop::sample::select(vec![Kind::Match, Kind::ReplyEnd, Kind::Supply])
}

/// A step, weighted toward sends and polls so that lanes carry several frames
/// before they close.
fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        4 => (0usize..8, kind()).prop_map(|(lane, kind)| Step::Send(lane, kind)),
        1 => (0usize..8).prop_map(Step::Close),
        4 => Just(Step::Poll),
    ]
}

proptest! {
    /// Under any interleaving of sends, closes, and polls, the multiplexer
    /// returns what one polling every lane on every poll returns, and is
    /// woken after `Pending` before any lane has news for it.
    ///
    /// The reference is [`FullScan`], on up to five lanes, each of whose
    /// channels holds up to [`LANE_CAPACITY`] frames. After a poll returns
    /// `Pending`, the task has been woken by the time any lane's channel
    /// holds a frame or a close the multiplexer has not taken.
    ///
    /// So polling only marked lanes changes neither the frames the
    /// multiplexer writes nor their order, and never leaves a frame
    /// waiting with the multiplexer unwoken. After the generated steps
    /// every lane closes and the multiplexer is polled until it returns
    /// `Ready(None)`, so every case also checks the drain.
    #[test]
    fn polls_match_the_full_scan(
        lanes in 1usize..=5,
        steps in prop::collection::vec(step(), 0..64),
    ) {
        let mut multiplexer = Multiplexer::default();
        let mut senders: Vec<Option<Outgoing>> = (1..=lanes)
            .map(|index| Some(multiplexer.lane(interior(u8::try_from(index).unwrap()), 0)))
            .collect();
        let mut full_scan = FullScan {
            lanes: (0..lanes).map(|_| ModeledLane::default()).collect(),
            cursors: [0; Class::ORDER.len()],
        };
        let wakes = Arc::new(WakeCount::default());
        let waker = Waker::from(Arc::clone(&wakes));
        let mut cx = Context::from_waker(&waker);
        let mut cursors = [0; Class::ORDER.len()];
        // The wake count when the multiplexer last returned `Pending`, until
        // it is polled again.
        let mut waiting_since: Option<usize> = None;
        let mut tag = 0;
        let close_all = (0..lanes).map(Step::Close);
        // Each poll writes one frame, and a lane holds at most its
        // channel's frames and one staged; the last poll finds every lane
        // ended.
        let drain = std::iter::repeat_n(Step::Poll, (LANE_CAPACITY + 1) * lanes + 1);
        for step in steps.into_iter().chain(close_all).chain(drain) {
            match step {
                Step::Send(lane, kind) => {
                    let lane = lane % lanes;
                    let modeled = &mut full_scan.lanes[lane];
                    if let Some(out) = &mut senders[lane]
                        && modeled.queued.len() < LANE_CAPACITY
                    {
                        tag += 1;
                        let sent = Frame::from(frame(kind, tag));
                        prop_assert!(
                            out.frames.try_send(bare(sent.clone())).is_ok(),
                            "an open channel with room accepts a frame"
                        );
                        modeled.queued.push_back((kind, sent));
                    }
                }
                Step::Close(lane) => {
                    let lane = lane % lanes;
                    if senders[lane].take().is_some() {
                        full_scan.lanes[lane].closed = true;
                    }
                }
                Step::Poll => {
                    let polled = multiplexer.poll_next(&mut cx, &mut cursors);
                    let expected = full_scan.poll();
                    let done = matches!(expected, Poll::Ready(None));
                    let pending = expected.is_pending();
                    prop_assert_eq!(
                        polled.map(|next| next.map(|(stream, item)| (stream.index(), item.frame))),
                        expected.map(|next| next.map(|(lane, frame)| {
                            (u8::try_from(lane + 1).unwrap(), frame)
                        })),
                    );
                    waiting_since = pending.then(|| wakes.0.load(Ordering::Relaxed));
                    if done {
                        break;
                    }
                }
            }
            if let Some(since) = waiting_since
                && full_scan.has_news()
            {
                prop_assert!(
                    wakes.0.load(Ordering::Relaxed) > since,
                    "a lane has news for a multiplexer that is waiting unwoken"
                );
            }
        }
        prop_assert!(
            full_scan.lanes.iter().all(|lane| lane.ended && lane.staged.is_none()),
            "the drain ran to the end"
        );
    }
}

/// The lanes of [`lane_wakes_from_other_threads_reach_the_multiplexer`]: twice
/// its runtime's workers, so that producers share workers.
const CROSS_THREAD_LANES: usize = 2 * CROSS_THREAD_WORKERS;

/// The worker threads of that test's runtime.
const CROSS_THREAD_WORKERS: usize = 6;

/// The frames each of that test's lanes carries before it ends: fewer than a
/// lane's channel holds, so no producer ever waits for room, and every wait on
/// a lane in the test is the multiplexer's.
const CROSS_THREAD_FRAMES: usize = 64;
const _: () = assert!(CROSS_THREAD_FRAMES < LANE_CAPACITY);

/// A producer of that test yields its worker after every this many frames, at
/// an offset its lane sets, so that each worker interleaves the sends of the
/// lanes it runs.
const CROSS_THREAD_YIELD_EVERY: usize = 7;

/// The multiplexer runs of that test. The scheduler places each run's threads
/// anew, so each run samples other interleavings of the lanes' wakes with the
/// multiplexer's registering and polling.
const CROSS_THREAD_ROUNDS: usize = 200;

/// Lane wakes from producers on other threads reach the multiplexer: every
/// lane's frames arrive whole and in lane order, each lane ending with one
/// end-of-stream frame.
///
/// The multiplexer runs on the test's thread while producers on a
/// multi-threaded runtime's workers hand frames to many lanes. A lane's channel
/// wakes the multiplexer through the lane's own waker, which records the lane
/// in the multiplexer's `Wakes` and wakes its task; here a producer calls it
/// from another thread while the multiplexer may be registering its task,
/// collecting the woken lanes, or polling a lane.
///
/// The producers hand over their frames far faster than the multiplexer writes
/// them, so producers started with the multiplexer have usually queued every
/// frame before it first looks, and it rarely waits on a lane at all. So the
/// producers start only once the multiplexer's run has first returned
/// `Pending`, having found every lane empty and registered its task: every
/// round then completes only through at least one wake from another thread.
///
/// [`polls_match_the_full_scan`] checks that protocol under every interleaving
/// on one thread; this test runs it across threads. A lost wake leaves the
/// multiplexer waiting on a lane that holds frames, so its run never completes,
/// and nextest's slow-timeout fails the test. The rounds cycle through the
/// default write batch range and the ranges whose end is zero and unbounded,
/// which change when the multiplexer flushes and so when it waits.
#[test]
fn lane_wakes_from_other_threads_reach_the_multiplexer() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(CROSS_THREAD_WORKERS)
        .build()
        .expect("a multi-threaded runtime builds");
    let batches = [
        WriteBatch::default(),
        WriteBatch::from_range(..=0),
        WriteBatch::from_range(0..),
    ];
    let lanes: Vec<Vec<ReplyFrame>> = (0..CROSS_THREAD_LANES)
        .map(|lane| {
            (0..CROSS_THREAD_FRAMES)
                .map(|position| {
                    let kind = if position % 3 == 0 {
                        Kind::Supply
                    } else {
                        Kind::Match
                    };
                    frame(kind, (lane * CROSS_THREAD_FRAMES + position) as u64)
                })
                .collect()
        })
        .collect();
    for round in 0..CROSS_THREAD_ROUNDS {
        let mut multiplexer = Multiplexer::new(batches[round % batches.len()]);
        let (start, started) = tokio::sync::watch::channel(false);
        let producers: Vec<_> = lanes
            .iter()
            .enumerate()
            .map(|(lane, frames)| {
                let mut out =
                    multiplexer.lane(interior(u8::try_from(lane + 1).expect("few lanes")), 0);
                let frames = frames.clone();
                let mut started = started.clone();
                runtime.spawn(async move {
                    started
                        .wait_for(|&started| started)
                        .await
                        .expect("the round signals its start before it ends");
                    for (position, frame) in frames.into_iter().enumerate() {
                        if position % CROSS_THREAD_YIELD_EVERY == lane % CROSS_THREAD_YIELD_EVERY {
                            tokio::task::yield_now().await;
                        }
                        out.frame(frame, Charge::default(), WirePermit::default())
                            .await;
                    }
                    out.finish().await;
                })
            })
            .collect();
        let written = runtime.block_on(async {
            let mut written = Vec::new();
            let observe = SessionHandle::default();
            {
                let mut run =
                    pin!(multiplexer.run(&mut written, SPEAKER, Recorder::default(), &observe));
                poll_fn(|cx| {
                    let polled = run.as_mut().poll(cx);
                    if polled.is_pending() {
                        start.send_replace(true);
                    }
                    polled
                })
                .await
                .expect("writing into a vector cannot fail");
            }
            for producer in producers {
                producer.await.expect("a producer runs to its end");
            }
            written
        });
        let received = decode_all(&written);
        assert_eq!(
            received.len(),
            CROSS_THREAD_LANES * (CROSS_THREAD_FRAMES + 1),
            "round {round}: every lane's frames and end-of-stream reach the pipe",
        );
        for (lane, frames) in lanes.iter().enumerate() {
            let stream = interior(u8::try_from(lane + 1).expect("few lanes"));
            let carried: Vec<Frame> = received
                .iter()
                .filter(|(on, _)| *on == stream)
                .map(|(_, frame)| frame.clone())
                .collect();
            let mut expected: Vec<Frame> = frames.iter().cloned().map(Frame::from).collect();
            expected.push(Frame::End(End::Stream));
            assert_eq!(
                carried, expected,
                "round {round}: lane {lane}'s frames, in order"
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

/// Run a demultiplexer over `bytes` with lanes claimed for `claimed`, until it
/// fails or every claimed lane's receiver ends. Returns the failure, if any,
/// each lane's frames, and the bytes the pipe left unconsumed.
#[allow(clippy::type_complexity)]
fn demultiplex(bytes: &[u8], claimed: &[Stream]) -> (Option<PipeError>, Vec<Vec<Frame>>, Vec<u8>) {
    let mut demultiplexer = Demultiplexer::default();
    let receivers: Vec<_> = claimed
        .iter()
        .map(|&stream| demultiplexer.lane(stream, 0))
        .collect();
    let mut read = PipeRead::new(bytes);
    let (failure, frames) = run_to_quiescence(async {
        let observe = SessionHandle::default();
        let mut demultiplexer = demultiplexer;
        let mut consume = pin!(futures::future::join_all(receivers.into_iter().map(
            |receiver| {
                receiver
                    .map(|inbound| inbound.frame)
                    .collect::<Vec<Frame>>()
            }
        ),));
        let pool = WirePool::receiving(usize::MAX, WIRE_POOL_RUNS);
        let run = demultiplexer.run(
            &mut read,
            SPEAKER,
            RunBudget::from_bytes(usize::MAX),
            &pool,
            Recorder::default(),
            &observe,
        );
        let failure = match futures::future::select(pin!(run), consume.as_mut()).await {
            Either::Left((failure, _)) => failure,
            Either::Right((frames, _)) => return (None, frames),
        };
        // A failed run leaves the lanes open; dropping the demultiplexer
        // closes them, so every receiver ends.
        drop(demultiplexer);
        (Some(failure), consume.await)
    })
    .expect("the demultiplexer either fails or closes every lane");
    (failure, frames, read.into_inner().to_vec())
}

/// The demultiplexer routes each frame to its logical stream's decoder, closes
/// a stream at its end-of-stream frame without delivering it, and stops at the
/// first item that is not a frame, consuming none of it.
///
/// Here the read half's buffer holds the whole two-byte marker, which stays
/// there for the closing exchange.
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
    let (failure, frames, ahead) = demultiplex(&bytes, &[interior(1), interior(2)]);
    assert!(failure.is_none(), "{failure:?}");
    assert_eq!(frames, [vec![reply(1), reply(3)], vec![reply(2)]]);
    assert_eq!(ahead, MARKER);
}

/// Peeking consumes nothing, so after any number of peeks the closing exchange
/// reads the whole closing item.
#[test]
fn peeking_leaves_the_closing_item_whole() {
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

/// End of stream at a frame boundary, before any closing item, is the peer's
/// departure.
#[test]
fn end_of_stream_between_frames_is_departure() {
    let bytes = wire(&[(interior(1), Frame::from(frame(Kind::Match, 0)))], &[]);
    let (failure, _, _) = demultiplex(&bytes, &[interior(1)]);
    assert!(
        matches!(&failure, Some(PipeError::Departed(source)) if source.kind() == io::ErrorKind::UnexpectedEof),
        "{failure:?}"
    );
}

/// A frame on a logical stream no decoder was created for answers no question,
/// and fails at that frame.
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

/// An end-of-stream frame on a claimed logical stream that carried no frame
/// answers no question: a conforming peer ends only a stream it started, so the
/// lone end fails at that frame.
#[test]
fn a_lone_end_on_an_unstarted_stream_is_unasked() {
    let bytes = wire(&[(interior(1), Frame::End(End::Stream))], &MARKER);
    let (failure, _, _) = demultiplex(&bytes, &[interior(1)]);
    assert!(
        matches!(&failure, Some(PipeError::Unasked { origin }) if *origin == Origin::stream(SPEAKER, interior(1))),
        "{failure:?}"
    );
}

/// A frame after its started logical stream's end answers no question, and
/// fails at that frame: the frames before the end were delivered.
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

/// A lane that never started closes cleanly at the closing item: its decoder,
/// if it still expected a reply, finds its stream ended.
#[test]
fn an_unstarted_stream_closes_at_the_closing_item() {
    let (failure, frames, ahead) = demultiplex(&MARKER, &[interior(1)]);
    assert!(failure.is_none(), "{failure:?}");
    assert_eq!(frames, [Vec::<Frame>::new()]);
    assert_eq!(ahead, MARKER);
}

/// A transport that delivers its bytes in scripted pieces, finding nothing
/// ready before each piece and waking its reader as it does.
struct Trickle {
    /// Every byte the transport delivers.
    bytes: Vec<u8>,
    /// How many of them it has delivered.
    delivered: usize,
    /// The sizes of its pieces, used in turn.
    pieces: Vec<usize>,
    /// The index of the next piece.
    piece: usize,
    /// Whether the next poll delivers a piece, rather than finding nothing
    /// ready.
    ready: bool,
}

/// Find nothing ready, waking the reader, and deliver one piece, in turn.
impl AsyncRead for Trickle {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if !this.ready {
            this.ready = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        this.ready = false;
        let piece = this.pieces[this.piece % this.pieces.len()];
        this.piece += 1;
        let taken = piece
            .min(buf.remaining())
            .min(this.bytes.len() - this.delivered);
        buf.put_slice(&this.bytes[this.delivered..this.delivered + taken]);
        this.delivered += taken;
        Poll::Ready(Ok(()))
    }
}

/// One use of the read half.
#[derive(Clone, Copy, Debug)]
enum Use {
    /// Peek at the next byte, polling until the peek resolves.
    Peek,
    /// Read into a buffer of this many bytes, polling until the read resolves.
    Read(usize),
    /// Start a peek, poll it once, and drop it unresolved if it found nothing
    /// ready.
    DroppedPeek,
    /// Start a read of this many bytes, poll it once, and drop it unresolved if
    /// it found nothing ready.
    DroppedRead(usize),
}

/// Any use of the read half, reading up to six bytes at once.
fn usage() -> impl Strategy<Value = Use> {
    prop_oneof![
        Just(Use::Peek),
        Just(Use::DroppedPeek),
        (1usize..=6).prop_map(Use::Read),
        (1usize..=6).prop_map(Use::DroppedRead),
    ]
}

/// Poll `future` once with a waker that does nothing.
fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

/// Poll `future` until it resolves; a [`Trickle`] transport always has a piece
/// ready by the second poll.
fn resolve<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = poll_once(future.as_mut()) {
            return output;
        }
    }
}

proptest! {
    /// The read half hands on every byte of its transport once and in
    /// order, and a peek returns the next byte not yet handed on.
    ///
    /// That holds however the transport splits its delivery, whatever the
    /// capacity of the buffer it is read through, wherever peeks fall
    /// between reads, and whenever a peek or a read is dropped after a poll
    /// that found nothing ready: a peek consumes nothing, and a dropped one
    /// loses nothing.
    #[test]
    fn the_read_half_hands_on_every_byte_once(
        bytes in proptest::collection::vec(any::<u8>(), 0..32),
        pieces in proptest::collection::vec(1usize..=5, 1..8),
        capacity in 1usize..=8,
        uses in proptest::collection::vec(usage(), 0..24),
    ) {
        let trickle = Trickle {
            bytes: bytes.clone(),
            delivered: 0,
            pieces,
            piece: 0,
            ready: false,
        };
        let mut read = PipeRead::new(BufReader::with_capacity(capacity, trickle));
        let mut handed = Vec::new();
        for usage in uses {
            let peeked = match usage {
                Use::Peek => Some(resolve(read.peek())),
                Use::DroppedPeek => match poll_once(pin!(read.peek())) {
                    Poll::Ready(peeked) => Some(peeked),
                    Poll::Pending => None,
                },
                Use::Read(len) => {
                    let mut buf = vec![0; len];
                    let filled = resolve(read.read(&mut buf)).expect("the transport never fails");
                    handed.extend_from_slice(&buf[..filled]);
                    None
                }
                Use::DroppedRead(len) => {
                    let mut buf = vec![0; len];
                    if let Poll::Ready(filled) = poll_once(pin!(read.read(&mut buf))) {
                        let filled = filled.expect("the transport never fails");
                        handed.extend_from_slice(&buf[..filled]);
                    }
                    None
                }
            };
            if let Some(peeked) = peeked {
                let peeked = peeked.expect("the transport never fails");
                prop_assert_eq!(peeked, bytes.get(handed.len()).copied());
            }
            prop_assert_eq!(&handed[..], &bytes[..handed.len()]);
        }
        let mut rest = Vec::new();
        resolve(read.read_to_end(&mut rest)).expect("the transport never fails");
        handed.extend(rest);
        prop_assert_eq!(handed, bytes);
    }
}
