//! Wire reconciliation through disputes at chosen tree depths.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use futures::join;
use proptest::prelude::*;

use crate::Version;
use crate::link::memory_with_capacity;
use crate::message::Message;
use crate::observe::{
    Attachment, Direction, Observer, Role, SessionHandle, SessionInfo, SessionKind,
    SessionObserver, StreamId, StreamInfo, StreamObserver,
};
use crate::testing::{IoPlan, IoReport, IoSide, reorder_accepts, run_to_quiescence, wrap_link};
use crate::tree::mirror::streaming::channel::{ChannelReport, QueueKind, with_observation};
use crate::tree::mirror::streaming::remote::codec::{
    Frame, RunBudget, Speaker, Stream, decode_exact,
};
use crate::tree::mirror::streaming::window::{Window, WindowConfig};
use crate::tree::mirror::streaming::{
    Local, Root, materialized::Handshaking, mirror, remote::Handshaking as RemoteHandshaking,
};
use crate::tree::typed::Path;
use crate::tree::typed::height::{Height, Root as RootHeight};
use crate::tree::{Action, Root as TreeRoot, Tree, arb::nth_party};

use super::harness::codec;

/// Leaf addresses and edits for a pair of replicas in one universe.
struct Divergence {
    /// Inserts made before the replicas separate.
    shared: Vec<Path>,
    /// Concurrent inserts made by each replica on its own party.
    novel: [Vec<Path>; 2],
    /// Leaves each replica forgets after its inserts.
    redacted: [Vec<Path>; 2],
}

impl Divergence {
    /// Give each prefix two shared leaves and independent additions per side.
    ///
    /// Leaf slots start at one, leaving radix zero available for a deeper
    /// prefix. Zero-filled prefixes of different lengths therefore build
    /// nested branches without overlapping any leaf addresses.
    fn new(prefixes: impl IntoIterator<Item = Vec<u8>>, novel: [usize; 2]) -> Self {
        let mut pair = Self {
            shared: Vec::new(),
            novel: Default::default(),
            redacted: Default::default(),
        };
        for prefix in prefixes {
            pair.shared
                .extend((1..=2).map(|slot| leaf_path(&prefix, slot)));
            let mut slot = 3;
            for (side, count) in novel.into_iter().enumerate() {
                pair.novel[side].extend((slot..slot + count).map(|slot| leaf_path(&prefix, slot)));
                slot += count;
            }
        }
        pair
    }

    /// Build the version-addressed trees, reconcile them, and check `Tree::join`.
    ///
    /// Preassign versions using `Tree::act`'s tick order. Each payload contains
    /// its address bytes, so a wrong leaf reconstruction also changes content.
    /// All trees stay inside the path-mapping scope; only observations escape.
    fn check(&self, session: &Session, plans: [IoPlan; 2]) -> Observations {
        let mut mapping = Vec::new();
        let mut shared = Version::new();
        for (party, paths) in std::iter::once(&self.shared)
            .chain(self.novel.iter())
            .enumerate()
        {
            let mut version = shared.clone();
            for path in paths {
                version.tick(&nth_party(party));
                mapping.push((version.clone(), *path));
            }
            if party == 0 {
                shared = version;
            }
        }
        Path::with_leaf_paths(mapping, || {
            let insert =
                |path: &Path| Action::Insert(Message::new(<[u8; 32]>::from(*path).to_vec()));
            let mut base = Tree::<Vec<u8>>::new();
            base.act(&nth_party(0), self.shared.iter().map(insert));
            let sides = std::array::from_fn(|side| {
                let mut tree = base.clone();
                let party = nth_party(side + 1);
                tree.act(&party, self.novel[side].iter().map(insert));
                tree.act(
                    &party,
                    self.redacted[side].iter().copied().map(Action::Forget),
                );
                tree
            });
            let [left, right] = sides;
            let mut expected = left.clone();
            expected.join(right.clone());
            let (actual, observations) = session.run([left.root, right.root], plans);
            for root in actual {
                let tree = Tree::<Vec<u8>>::from_root(root);
                assert_eq!(tree.root, expected.root);
                // Merkle equality identifies versions; inspect payloads too.
                for (version, payload) in expected.iter() {
                    assert_eq!(tree.get(version), Some(payload));
                }
            }
            observations
        })
    }
}

/// Append one nonzero leaf slot to a prefix, then pad the address with zeros.
fn leaf_path(prefix: &[u8], slot: usize) -> Path {
    let mut bytes = [0; RootHeight::HEIGHT];
    bytes[..prefix.len()].copy_from_slice(prefix);
    bytes[prefix.len()] = u8::try_from(slot).unwrap();
    bytes.into()
}

/// Stream identities of sent reactions, shared with a session observer.
#[derive(Clone, Default)]
struct SentFrames(Arc<Mutex<Vec<StreamId>>>);

impl SentFrames {
    /// Attach this log to a wire endpoint.
    fn handle(&self) -> SessionHandle {
        let mut attachment = Attachment::default();
        attachment.attach(Arc::new(self.clone()));
        attachment.begin(SessionKind::Gossip)
    }

    /// Check every interior stream down to the fixture's deepest branch.
    fn assert_reach(&self, width: u8) {
        let frames = self.0.lock().unwrap();
        let StreamId::Data { speaker: role, .. } =
            *frames.first().expect("no data frames were sent")
        else {
            unreachable!("only sent data frames are recorded")
        };
        let speaker = speaker(role);
        let branch_height = RootHeight::HEIGHT - usize::from(width);
        for index in 0..Stream::COUNT {
            let height = Stream::new(index).unwrap().height(speaker);
            // Initiator stream zero supplies exclusive root children, which
            // some fixtures do not have. Its traffic is checked separately.
            if height >= branch_height && !(role == Role::Initiator && index == 0) {
                assert!(
                    frames.contains(&StreamId::Data {
                        speaker: role,
                        index
                    }),
                    "{role:?} sent no frame on stream {index}, prefix {width}: {frames:?}"
                );
            }
        }
    }
}

impl Observer for SentFrames {
    /// Observe one endpoint's session using this same log.
    fn session(&self, _: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        Some(Box::new(self.clone()))
    }
}

impl SessionObserver for SentFrames {
    /// Record outgoing data frames; control and incoming streams are irrelevant.
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        match stream.id {
            StreamId::Data { .. } if stream.direction == Direction::Sent => {
                Some(Box::new(SentStream {
                    id: stream.id,
                    frames: self.clone(),
                }))
            }
            _ => None,
        }
    }
}

/// Add reactions from one data stream to its endpoint's log.
struct SentStream {
    /// The outgoing stream whose frames this handler observes.
    id: StreamId,
    /// The endpoint's shared frame log.
    frames: SentFrames,
}

impl StreamObserver for SentStream {
    /// Record only reactions; stream endings alone prove no descent.
    fn message(&mut self, bytes: &[u8]) {
        let StreamId::Data {
            speaker: role,
            index,
        } = self.id
        else {
            unreachable!("only sent data streams are observed")
        };
        let (stream, frame) = decode_exact(speaker(role), RunBudget::default(), bytes).unwrap();
        assert_eq!(stream.index(), index);
        if matches!(frame, Frame::Reaction(..)) {
            self.frames.0.lock().unwrap().push(self.id);
        }
    }
}

/// Convert an observer role to the codec's corresponding speaker.
fn speaker(role: Role) -> Speaker {
    match role {
        Role::Initiator => Speaker::Initiator,
        Role::Responder => Speaker::Responder,
    }
}

/// Transport and queue capacities used by one closed-world session.
struct Session {
    /// Bytes buffered independently by each transport stream.
    capacity: usize,
    /// Per-height protocol queue capacities, shared by all participants.
    window: Window,
    /// Per-endpoint batch sizes for reversed arrivals; one keeps their order.
    reorder: [usize; 2],
}

/// Evidence retained after both endpoints and their trees have been dropped.
struct Observations {
    /// Actual reactions sent by each endpoint.
    sent: [SentFrames; 2],
    /// Completed I/O and injected delays at each endpoint.
    io: [IoReport; 2],
    /// Genuine arrival inversions at each acceptor.
    inversions: [usize; 2],
    /// Occupancy of the protocol queues, grouped by kind and height.
    queues: ChannelReport,
}

impl Session {
    /// Drive two observed proxies, checking liveness without an external runtime.
    fn run(&self, roots: [TreeRoot; 2], plans: [IoPlan; 2]) -> ([TreeRoot; 2], Observations) {
        let sent = [SentFrames::default(), SentFrames::default()];
        let inversions = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
        let (left_link, right_link) = memory_with_capacity(self.capacity);
        let [left_plan, right_plan] = plans;
        let (left_link, left_io) = wrap_link(IoSide::Left, left_plan, left_link);
        let (right_link, right_io) = wrap_link(IoSide::Right, right_plan, right_link);
        let left_link = reorder_accepts(left_link, self.reorder[0], inversions[0].clone());
        let right_link = reorder_accepts(right_link, self.reorder[1], inversions[1].clone());
        let [left, right] = roots;
        let window = WindowConfig::Fixed(self.window);
        let (roots, queues) = with_observation(|| {
            run_to_quiescence(async {
                let left = Handshaking::start(Local, Root::from(left)).window(window);
                let right = Handshaking::start(Local, Root::from(right)).window(window);
                let left_proxy = RemoteHandshaking::start(Local, left_link, codec::<Vec<u8>>())
                    .window(window)
                    .observe(sent[0].handle());
                let right_proxy = RemoteHandshaking::start(Local, right_link, codec::<Vec<u8>>())
                    .window(window)
                    .observe(sent[1].handle());
                let (left, right) = join!(
                    Box::pin(mirror(left, left_proxy)),
                    Box::pin(mirror(right, right_proxy)),
                );
                [
                    left.expect("left session failed").0.into(),
                    right.expect("right session failed").0.into(),
                ]
            })
            .expect("deep reconciliation stopped making progress")
        });
        (
            roots,
            Observations {
                sent,
                io: [left_io.snapshot(), right_io.snapshot()],
                inversions: inversions.map(|count| count.load(Ordering::Relaxed)),
                queues,
            },
        )
    }
}

impl Observations {
    /// Check actual reactions on both sides down to the deepest disputed prefix.
    fn assert_reach(&self, depth: usize) {
        for sent in &self.sent {
            sent.assert_reach(u8::try_from(depth).unwrap());
        }
    }
}

/// A root child with a zero-filled prefix of the requested length.
fn root_prefix(child: usize, depth: usize) -> Vec<u8> {
    let mut prefix = vec![0; depth];
    prefix[0] = u8::try_from(child).unwrap();
    prefix
}

/// Full root fans with mixed depths converge and send terminal leaf reactions
/// under both minimum and wider protocol windows.
#[test]
fn full_root_with_leaf_dispute_reaches_final_stream() {
    let pair = Divergence::new(
        (0..256).map(|child| {
            root_prefix(
                child,
                match child {
                    0 => 28,
                    1 => 31,
                    _ => 1,
                },
            )
        }),
        [1, 1],
    );
    for width in [1, 8] {
        let session = Session {
            capacity: 1,
            window: Window::uniform(width),
            reorder: [1, 1],
        };
        let observed = pair.check(&session, [IoPlan::default(), IoPlan::default()]);
        observed.assert_reach(31);
        for sent in &observed.sent {
            let frames = sent.0.lock().unwrap();
            assert!(
                frames.iter().any(|frame| matches!(
                    frame,
                    StreamId::Data {
                        index: Stream::MAX,
                        ..
                    }
                )),
                "no leaf reaction: {frames:?}"
            );
        }
    }
}

/// Every branching position and a chain branching at all positions converge
/// in both orientations and window sizes, with writes visible only on flush.
#[test]
fn every_branching_depth_matches_join() {
    let shapes = (0..RootHeight::HEIGHT)
        .map(|depth| vec![vec![0; depth]])
        .chain(std::iter::once(
            (0..RootHeight::HEIGHT)
                .map(|depth| vec![0; depth])
                .collect(),
        ));
    for prefixes in shapes {
        let depth = prefixes.last().unwrap().len();
        for novel in [[1, 3], [3, 1]] {
            let pair = Divergence::new(prefixes.clone(), novel);
            for width in [1, 8] {
                let session = Session {
                    capacity: 1,
                    window: Window::uniform(width),
                    reorder: [1, 1],
                };
                let observed = pair.check(
                    &session,
                    std::array::from_fn(|_| IoPlan {
                        hold_until_flush: true,
                        ..IoPlan::default()
                    }),
                );
                if depth > 1 {
                    observed.assert_reach(depth);
                }
            }
        }
    }
}

/// Multiple disputes at deep heights use more than one queue slot, while
/// reversed stream arrivals preserve both peers' reconciliation results.
#[test]
fn deep_queues_use_the_wider_window() {
    let mut pair = Divergence::new((0..16).map(|child| root_prefix(child, 31)), [2, 3]);
    pair.novel[0].extend((128..136).map(|slot| leaf_path(&[], slot)));
    pair.novel[1].extend((136..144).map(|slot| leaf_path(&[], slot)));
    for swap in [false, true] {
        if swap {
            pair.novel.swap(0, 1);
        }
        for width in [1, 8] {
            let session = Session {
                capacity: 4096,
                window: Window::uniform(width),
                // Hold the initiator's opening supply while its opposite
                // acceptor keeps reading. That allows a later stream to
                // arrive before the opening stream is delivered.
                reorder: if swap { [3, 1] } else { [1, 3] },
            };
            let observed = pair.check(&session, [IoPlan::default(), IoPlan::default()]);
            observed.assert_reach(31);
            assert!(
                observed.inversions.iter().sum::<usize>() > 0,
                "no stream arrivals were reversed"
            );
            let peak = observed
                .queues
                .roles()
                .filter(|(role, _)| {
                    role.height <= 4
                        && matches!(
                            role.kind,
                            QueueKind::ProxyLocalQuestions | QueueKind::ProxyNextScopes
                        )
                })
                .map(|(_, stats)| stats.high_water)
                .max()
                .unwrap();
            assert!(peak <= width);
            if width == 1 {
                assert_eq!(peak, 1);
            } else {
                assert!(
                    peak > 1,
                    "deep queues never used the wider window: {:?}",
                    observed.queues
                );
            }
        }
    }
}

proptest! {
    /// Disputes at mixed depths preserve messages and redactions under both
    /// narrow and wide protocol windows, with arbitrary early I/O delays.
    #[test]
    fn deep_disputes_match_join(
        prefixes in proptest::collection::vec(1u8..=31, 1..=8),
        novel in prop::array::uniform2(1usize..=4),
        redact in any::<bool>(),
        width in prop_oneof![Just(1usize), Just(2), Just(8)],
        capacity in prop_oneof![Just(1usize), Just(37), Just(4096)],
        delays in prop::array::uniform2(proptest::collection::vec(0u8..=2, 0..=32)),
    ) {
        let mut pair = Divergence::new(prefixes.iter().enumerate()
            .map(|(child, &depth)| root_prefix(child, usize::from(depth))), novel);
        if redact {
            for (index, &path) in pair.shared.iter().enumerate() {
                pair.redacted[index % 2].push(path);
            }
        }
        let session = Session { capacity, window: Window::uniform(width), reorder: [1, 1] };
        let observed = pair.check(&session, delays.map(|delays| IoPlan {
            read_delays: delays.clone(),
            write_delays: delays.clone(),
            flush_delays: delays,
            ..IoPlan::default()
        }));
        observed.assert_reach(usize::from(*prefixes.iter().max().unwrap()));
    }

    /// Nested disputes progress beside exclusive root supplies and an entirely
    /// redacted shared subtree, with reordered arrivals and delayed flushes
    /// extending through the complete descent.
    #[test]
    fn nested_disputes_with_supplies_match_join(
        (outer, inner) in (1usize..31).prop_flat_map(|outer| (Just(outer), outer + 1..32)),
        novel in prop::array::uniform2(1usize..=4),
        redact_left in any::<bool>(),
        width in prop_oneof![Just(1usize), Just(2), Just(8)],
        capacity in prop_oneof![Just(1usize), Just(37), Just(4096)],
        delay in prop::array::uniform2(1u8..=2),
    ) {
        // The empty prefix adds root leaves after the radix-zero dispute.
        // Within that dispute, the outer branch also has a deeper child.
        let mut pair = Divergence::new([vec![], vec![0; outer], vec![0; inner]], novel);
        let redacted = [leaf_path(&[254], 1), leaf_path(&[254], 2)];
        pair.shared.extend(redacted);
        pair.redacted[usize::from(!redact_left)].extend(redacted);
        let session = Session { capacity, window: Window::uniform(width), reorder: [1, 3] };
        let plans = std::array::from_fn(|_| IoPlan { hold_until_flush: true, ..IoPlan::default() });
        let baseline = pair.check(&session, plans.clone());
        let mut delayed = plans;
        for (side, plan) in delayed.iter_mut().enumerate() {
            plan.flush_delays = vec![delay[side]; baseline.io[side].flushes];
        }
        let observed = pair.check(&session, delayed);
        observed.assert_reach(inner);
        for (side, delay) in delay.into_iter().enumerate() {
            // A finite delay schedule must cover the whole run, rather than
            // silently expire before the deep frames. The flush count comes
            // from this fixture's baseline, not a shallow session constant.
            prop_assert!(observed.io[side].flushes <= baseline.io[side].flushes);
            prop_assert_eq!(observed.io[side].delayed_polls,
                observed.io[side].flushes * usize::from(delay));
        }
        prop_assert!(observed.sent.iter().any(|sent| sent.0.lock().unwrap().iter().any(|frame|
            matches!(frame, StreamId::Data { speaker: Role::Initiator, index: 0 })
        )), "no opening subtree supplies");
        // Actual reversal is guaranteed by the dedicated wide fixture; this
        // family also includes schedules with no simultaneous arrivals.
    }
}
