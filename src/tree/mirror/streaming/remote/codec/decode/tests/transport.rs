//! Frame boundaries and error context under partial delivery.

use super::*;
use crate::testing::run_to_quiescence;

/// A valid frame and the byte where its supply content begins, if any.
#[derive(Clone, Debug)]
struct FrameCase {
    bytes: Vec<u8>,
    expected: WireFrame,
    /// First record byte, after the outer supply length header.
    supply_at: Option<usize>,
}

/// Build expected values and error positions independently of the decoder.
impl FrameCase {
    /// Encode a frame with the test writers and remember its body boundary.
    fn new(stream: Stream, frame: Frame) -> Self {
        let (bytes, supply_at) = match &frame {
            Frame::Reaction(Reaction::Supply(run), flow) => {
                let bytes = supply(stream, *flow, run.as_bytes());
                let at = bytes.len() - run.encoded_len();
                (bytes, Some(at))
            }
            Frame::Reaction(Reaction::Query(children), flow) if !children.is_empty() => {
                (query(stream, *flow, children), None)
            }
            Frame::Reaction(Reaction::Match, flow) => {
                (bare_frame(stream, Signal::Match(*flow)), None)
            }
            Frame::Reaction(Reaction::Query(_), flow) => {
                (bare_frame(stream, Signal::QueryEmpty(*flow)), None)
            }
            Frame::End(end) => (bare_frame(stream, Signal::End(*end)), None),
        };
        Self {
            bytes,
            expected: (stream, frame),
            supply_at,
        }
    }

    /// The component containing the next unread byte of this valid frame.
    fn part_at(&self, cut: usize) -> FramePart {
        // All valid openers are three one-byte items: array, stream, state.
        match cut {
            0 => FramePart::FrameHead,
            1..=2 => FramePart::Signal,
            _ => match self.supply_at {
                Some(at) if cut < at => FramePart::SupplyLength,
                Some(_) => FramePart::SupplyRun,
                None => FramePart::QueryChildren,
            },
        }
    }

    /// Exercise normal buffering and, for one record, prefix inspection too.
    fn budgets(&self) -> Vec<RunBudget> {
        let mut budgets = vec![RunBudget::default()];
        if matches!(&self.expected.1, Frame::Reaction(Reaction::Supply(run), _) if run.record_count() == 1)
        {
            budgets.push(RunBudget::from_bytes(0));
        }
        budgets
    }
}

/// Frames spanning header-width changes, wide listings, and payload chunks.
fn arb_frame_case() -> impl Strategy<Value = FrameCase> {
    let contents = prop_oneof![
        prop::collection::vec(any::<u8>(), 0..300),
        (
            prop::sample::select(vec![0, 1, 23, 24, 255, 256, 65_535, 65_536, 65_537]),
            any::<u8>()
        )
            .prop_map(|(len, byte)| vec![byte; len]),
    ];
    let reaction = prop_oneof![
        Just(Reaction::Match),
        prop::collection::btree_map(any::<u8>(), any::<[u8; MERKLE_HASH_LEN]>(), 0..=256).prop_map(
            |children| Reaction::Query(children.into_iter().map(|(r, h)| (r, Hash(h))).collect())
        ),
        prop::collection::vec(contents, 1..=3).prop_map(|records| {
            let bytes = records.iter().flat_map(|r| raw_record(r)).collect();
            // These records have valid framing. Version and payload decoding
            // belong to the record iterator, outside this suite's contract.
            Reaction::Supply(LeafRun::from_encoded(bytes).unwrap())
        }),
    ];
    // Favor reactions so listing and supply reads receive substantial coverage.
    let frame = prop_oneof![
        6 => (reaction, arb_flow()).prop_map(|(reaction, flow)| Frame::Reaction(reaction, flow)),
        1 => Just(Frame::End(End::Reply)),
        1 => Just(Frame::End(End::Stream)),
    ];
    // Interior streams admit every reaction for either speaker. The signal
    // suite separately checks the opening and terminal streams' restrictions.
    (1u8..Stream::MAX, frame).prop_map(|(index, frame)| FrameCase::new(stream(index), frame))
}

/// Favor structural boundaries without excluding cuts anywhere in the body.
fn arb_interrupted_frame() -> impl Strategy<Value = (FrameCase, usize)> {
    arb_frame_case().prop_flat_map(|case| {
        let len = case.bytes.len();
        let mut boundaries = vec![0, 1, 2, len - 1];
        if len > 3 {
            boundaries.push(3);
        }
        if let Some(at) = case.supply_at {
            boundaries.extend([at - 1, at]);
        }
        (
            Just(case),
            prop_oneof![0..len, prop::sample::select(boundaries)],
        )
    })
}

/// Read schedules include both one-byte delivery and a whole-frame offer.
fn arb_chunks() -> impl Strategy<Value = Vec<usize>> {
    prop_oneof![
        Just(vec![1]),
        Just(vec![usize::MAX]),
        prop::collection::vec(1usize..1024, 1..8),
    ]
}

/// Check the interrupted component, known sender, and original I/O kind.
fn assert_failure(
    error: DecodeError,
    case: &FrameCase,
    speaker: Speaker,
    cut: usize,
    kind: std::io::ErrorKind,
) {
    let expected_origin = if cut < 3 {
        Origin::direction(speaker)
    } else {
        Origin::stream(speaker, case.expected.0)
    };
    assert_eq!(error.origin, expected_origin);
    let (part, source) = match error.kind {
        DecodeErrorKind::Truncated { missing, source }
            if kind == std::io::ErrorKind::UnexpectedEof =>
        {
            (missing, source)
        }
        DecodeErrorKind::Read { part, source } if kind != std::io::ErrorKind::UnexpectedEof => {
            (part, source)
        }
        other => panic!("cut {cut}, {kind:?}: unexpected failure {other:?}"),
    };
    assert_eq!(part, case.part_at(cut), "cut {cut}");
    assert_eq!(source.kind(), kind);
}

proptest! {
    /// Chunk sizes and pending polls change neither frames nor consumption:
    /// both readers stop at each boundary and leave the next frame intact.
    #[test]
    fn delivery_preserves_frames_and_boundaries(
        case in arb_frame_case(),
        chunks in arb_chunks(),
        speaker in arb_speaker(),
    ) {
        let tail = bare_frame(case.expected.0, Signal::End(End::Stream));
        let bytes = [case.bytes.as_slice(), tail.as_slice()].concat();
        for budget in case.budgets() {
            let mut sync = TestRead::new(&bytes).chunked(chunks.clone());
            let mut reader = FrameRead::new(speaker, budget, TestRead::new(&bytes).chunked(chunks.clone()));
            let actual = run_to_quiescence(reader.frame()).expect("finite input makes progress").unwrap();
            prop_assert_eq!(actual.as_ref(), Some(&case.expected));
            prop_assert_eq!(&decode(speaker, budget, &mut sync).unwrap(), &case.expected);
            prop_assert_eq!(sync.unread(), tail.as_slice());
            // Inspect the returned transport too: decoding the next frame alone
            // could conceal read-ahead retained inside the decoder.
            let transport = reader.into_inner();
            prop_assert_eq!(transport.unread(), tail.as_slice());
            let mut reader = FrameRead::new(speaker, budget, transport);
            let end = run_to_quiescence(reader.frame()).unwrap().unwrap();
            prop_assert_eq!(end, Some((case.expected.0, Frame::End(End::Stream))));
            prop_assert_eq!(run_to_quiescence(reader.frame()).unwrap().unwrap(), None);
        }
    }

    /// Closing within any frame reports its missing component; closing before
    /// one starts is clean on the async interface, which permits no next frame.
    #[test]
    fn every_truncation_reports_its_component(
        (case, cut) in arb_interrupted_frame(),
        chunks in arb_chunks(),
        speaker in arb_speaker(),
    ) {
        for budget in case.budgets() {
            let mut sync = TestRead::new(&case.bytes[..cut]).chunked(chunks.clone());
            let mut reader = FrameRead::new(speaker, budget, TestRead::new(&case.bytes[..cut]).chunked(chunks.clone()));
            let actual = run_to_quiescence(reader.frame()).expect("a closed input makes progress");
            if cut == 0 {
                prop_assert_eq!(actual.unwrap(), None);
            } else {
                assert_failure(actual.unwrap_err(), &case, speaker, cut, std::io::ErrorKind::UnexpectedEof);
            }
            assert_failure(decode(speaker, budget, &mut sync).unwrap_err(), &case, speaker, cut, std::io::ErrorKind::UnexpectedEof);
        }
    }

    /// An observed error is returned without another transport read, even if
    /// a later read would close or resume. Its kind and position survive.
    #[test]
    fn read_failures_preserve_the_original_error(
        (case, cut) in arb_interrupted_frame(),
        chunks in arb_chunks(),
        speaker in arb_speaker(),
        after in prop::sample::select(vec![AfterFailure::Fail, AfterFailure::Close, AfterFailure::Resume]),
        kind in prop::sample::select(vec![std::io::ErrorKind::Other, std::io::ErrorKind::ConnectionReset, std::io::ErrorKind::UnexpectedEof]),
    ) {
        for budget in case.budgets() {
            let make = || TestRead::new(&case.bytes).chunked(chunks.clone()).fail_after(cut, after, kind);
            let mut sync = make();
            let mut reader = FrameRead::new(speaker, budget, make());
            let actual = run_to_quiescence(reader.frame()).expect("an error makes progress").unwrap_err();
            assert_failure(actual, &case, speaker, cut, kind);
            assert_failure(decode(speaker, budget, &mut sync).unwrap_err(), &case, speaker, cut, kind);
            for input in [sync, reader.into_inner()] {
                prop_assert!(input.failed);
                prop_assert_eq!(input.reads_after_failure, 0);
                prop_assert_eq!(input.unread(), &case.bytes[cut..]);
            }
        }
    }
}
