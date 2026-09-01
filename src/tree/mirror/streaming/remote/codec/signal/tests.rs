use std::collections::BTreeSet;
use std::fmt::Write;

use super::*;

/// Protocol-valid signal placements in either speaker direction.
const VALID_PLACEMENTS: [usize; 2] = [162, 163];

/// The state roster is a bijection between the ten signals and the state
/// codes 0 through 9; every other code is reserved.
#[test]
fn state_roster_is_bijective() {
    let mut codes = BTreeSet::new();
    for (state, signal) in Signal::STATES.into_iter().enumerate() {
        let state = state as u8;
        assert_eq!(signal.state(), state);
        assert_eq!(Signal::from_state(state), Ok(signal));
        assert!(codes.insert(state), "duplicate state code {state}");
    }
    assert_eq!(codes.len(), usize::from(Signal::STATE_COUNT));
    for state in Signal::STATE_COUNT..=u8::MAX {
        let invalid = Signal::from_state(state).unwrap_err();
        assert_eq!(invalid, InvalidSignalState { state });
        assert_eq!(invalid.state(), state);
    }
}

/// Decoding an opener rejects a reserved stream index before looking at
/// the state, and a reserved state on a known stream names that stream.
#[test]
fn reserved_opener_items_are_typed() {
    for speaker in [Speaker::Initiator, Speaker::Responder] {
        for index in [
            u64::from(Stream::COUNT),
            u64::from(u8::MAX),
            u64::from(u8::MAX) + 1,
        ] {
            let invalid = WireSignal::decode(speaker, index, 0).unwrap_err();
            assert_eq!(invalid, DecodeSignalError::Stream { index });
            assert_eq!(invalid.stream(), None);
        }
        for state in [
            u64::from(Signal::STATE_COUNT),
            u64::from(u8::MAX),
            u64::from(u8::MAX) + 1,
        ] {
            let stream = Stream::new(3).unwrap();
            let invalid =
                WireSignal::decode(speaker, u64::from(stream.index()), state).unwrap_err();
            assert_eq!(invalid, DecodeSignalError::State { stream, state });
            assert_eq!(invalid.stream(), Some(stream));
        }
    }
}

/// Both directions accept exactly their protocol-valid subset of the product.
#[test]
fn placements_match_the_phase_schedule_exhaustively() {
    for (direction, speaker) in [Speaker::Initiator, Speaker::Responder]
        .into_iter()
        .enumerate()
    {
        let mut accepted = 0;
        for index in 0..Stream::COUNT {
            let stream = Stream::new(index).unwrap();
            assert_eq!(stream.class(speaker), expected_class(speaker, index));
            for signal in Signal::STATES {
                let expected = placement_is_valid(speaker, index, signal);
                let constructed = WireSignal::new(speaker, stream, signal);
                let decoded =
                    WireSignal::decode(speaker, u64::from(index), u64::from(signal.state()));
                if expected {
                    accepted += 1;
                    let wire = constructed.unwrap();
                    assert_eq!(decoded.unwrap(), wire);
                    assert_eq!(wire.into_parts(), (stream, signal));
                } else {
                    let invalid = constructed.unwrap_err();
                    assert_eq!(invalid.stream(), stream);
                    assert_eq!(invalid.signal(), signal);
                    assert_eq!(invalid.class(), expected_class(speaker, index));
                    assert_eq!(decoded, Err(DecodeSignalError::Placement(invalid)));
                }
            }
        }
        assert_eq!(accepted, VALID_PLACEMENTS[direction]);
    }
}

/// Every phase-invalid placement is pinned by stream and signal.
#[test]
fn invalid_placement_snapshot() {
    let mut rejected = String::new();
    for speaker in [Speaker::Initiator, Speaker::Responder] {
        writeln!(rejected, "{speaker:?}").unwrap();
        for index in 0..Stream::COUNT {
            let stream = Stream::new(index).unwrap();
            for signal in Signal::STATES {
                let Err(invalid) = WireSignal::new(speaker, stream, signal) else {
                    continue;
                };
                writeln!(
                    rejected,
                    "  stream {index:02} {signal:?} -> {:?}",
                    invalid.class(),
                )
                .unwrap();
            }
        }
    }
    insta::assert_snapshot!(rejected);
}

fn expected_class(speaker: Speaker, index: u8) -> StreamClass {
    match (speaker, index) {
        (Speaker::Initiator, 0) => StreamClass::OpeningSupplies,
        (Speaker::Responder, 0) => StreamClass::OpeningReply,
        (Speaker::Initiator, Stream::MAX) => StreamClass::LeafParentReplies,
        (Speaker::Responder, Stream::MAX) => StreamClass::TerminalLeafReplies,
        (_, _) => StreamClass::InteriorReplies,
    }
}

fn placement_is_valid(speaker: Speaker, index: u8, signal: Signal) -> bool {
    match (speaker, index) {
        (Speaker::Initiator, 0) => matches!(signal, Signal::Supply(_) | Signal::End(_)),
        (Speaker::Responder, 0) => true,
        (Speaker::Initiator, Stream::MAX) => !matches!(signal, Signal::Query(_)),
        (Speaker::Responder, Stream::MAX) => {
            matches!(signal, Signal::Supply(Flow::End) | Signal::End(_))
        }
        (_, _) => true,
    }
}

/// The state roster is wire format: each state code's signal is pinned.
#[test]
fn state_roster_snapshot() {
    let mut roster = String::new();
    for state in 0..Signal::STATE_COUNT {
        writeln!(roster, "{state}: {:?}", Signal::from_state(state).unwrap()).unwrap();
    }
    insta::assert_snapshot!(roster);
}

/// Both elected speakers map their 17 stream indices bijectively to schedule heights.
#[test]
fn stream_height_mappings_are_bijective() {
    for speaker in [Speaker::Initiator, Speaker::Responder] {
        for index in 0..=Stream::MAX {
            let stream = Stream::new(index).unwrap();
            let height = stream.height(speaker);
            assert_eq!(Stream::at_height(speaker, height), Some(stream));
        }
    }

    for height in LEAF_HEIGHT..=STREAMED_HEIGHT_COUNT {
        let [initiator, responder] = [Speaker::Initiator, Speaker::Responder]
            .map(|speaker| Stream::at_height(speaker, height).is_some());
        if height == LEAF_HEIGHT || height == HIGHEST_STREAM_HEIGHT {
            assert!(initiator && responder, "height {height}");
        } else if height < STREAMED_HEIGHT_COUNT {
            assert_ne!(initiator, responder, "height {height}");
        } else {
            assert!(!initiator && !responder, "height {height}");
        }
    }
}
