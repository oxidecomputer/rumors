use std::collections::BTreeSet;
use std::fmt::Write;

use super::*;

/// Protocol-valid signal placements in either speaker direction.
const VALID_PLACEMENTS_PER_SPEAKER: usize = 223;

/// The semantic signal product maps bijectively onto bytes 0 through 237.
#[test]
fn encoding_is_bijective() {
    let mut bytes = BTreeSet::new();
    for (state, signal) in Signal::STATES.into_iter().enumerate() {
        let state = state as u8;
        assert_eq!(Signal::from_state(state), Ok(signal));
        for index in 0..Stream::COUNT {
            let stream = Stream::new(index).unwrap();
            let wire = WireSignal::pair(stream, signal);
            let byte = wire.to_byte();
            assert!(bytes.insert(byte), "duplicate signal byte {byte:#04x}");
            assert_eq!(WireSignal::parse(byte).unwrap(), wire);
        }
    }
    assert_eq!(bytes.len(), usize::from(WireSignal::BYTE_COUNT));

    for byte in 0..=u8::MAX {
        match WireSignal::parse(byte) {
            Ok(wire) => {
                assert!(byte < WireSignal::BYTE_COUNT);
                assert_eq!(wire.to_byte(), byte);
            }
            Err(invalid) => {
                assert_eq!(invalid.byte(), byte);
                assert_eq!(invalid.stream(), Stream(byte % Stream::COUNT));
                assert_eq!(invalid.state(), byte / Stream::COUNT);
                assert!(std::error::Error::source(&invalid).is_some());
                assert!(byte >= WireSignal::BYTE_COUNT);
            }
        }
    }

    for state in Signal::STATE_COUNT..=u8::MAX {
        assert_eq!(Signal::from_state(state), Err(InvalidSignalState { state }));
    }
}

/// Both directions accept exactly their protocol-valid subset of the product.
#[test]
fn placements_match_the_phase_schedule_exhaustively() {
    for speaker in [Speaker::Initiator, Speaker::Responder] {
        let mut accepted = 0;
        for index in 0..Stream::COUNT {
            let stream = Stream::new(index).unwrap();
            assert_eq!(stream.class(speaker), expected_class(speaker, index));
            for signal in Signal::STATES {
                let byte = WireSignal::pair(stream, signal).to_byte();
                let expected = placement_is_valid(speaker, index, signal);
                let constructed = WireSignal::new(speaker, stream, signal);
                let decoded = WireSignal::from_byte(speaker, byte);
                if expected {
                    accepted += 1;
                    let wire = constructed.unwrap();
                    assert_eq!(decoded.unwrap(), wire);
                } else {
                    let invalid = constructed.unwrap_err();
                    assert_eq!(invalid.byte(), byte);
                    assert_eq!(invalid.class(), expected_class(speaker, index));
                    assert_eq!(decoded, Err(DecodeSignalError::Placement(invalid)));
                }
            }
        }
        assert_eq!(accepted, VALID_PLACEMENTS_PER_SPEAKER);
    }
}

/// Every phase-invalid placement is pinned separately from the raw byte layout.
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
                    "  {:02x}: stream {index:02} {signal:?} -> {:?}",
                    invalid.byte(),
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
        (Speaker::Initiator, 0) => StreamClass::OpeningQuestion,
        (Speaker::Responder, 0) => StreamClass::OpeningReply,
        (Speaker::Initiator, Stream::MAX) => StreamClass::LeafParentReplies,
        (Speaker::Responder, Stream::MAX) => StreamClass::TerminalLeafReplies,
        (_, _) => StreamClass::InteriorReplies,
    }
}

fn placement_is_valid(speaker: Speaker, index: u8, signal: Signal) -> bool {
    match (speaker, index) {
        (Speaker::Initiator, 0) => matches!(
            signal,
            Signal::QueryEmpty(Flow::End(End::Stream)) | Signal::Query(Flow::End(End::Stream))
        ),
        (Speaker::Responder, 0) => matches!(
            signal,
            Signal::Match(Flow::Continue | Flow::End(End::Stream))
                | Signal::QueryEmpty(Flow::Continue | Flow::End(End::Stream))
                | Signal::Query(Flow::Continue | Flow::End(End::Stream))
                | Signal::Supply(Flow::Continue | Flow::End(End::Stream))
                | Signal::End(End::Stream)
        ),
        (Speaker::Initiator, Stream::MAX) => !matches!(signal, Signal::Query(_)),
        (Speaker::Responder, Stream::MAX) => matches!(
            signal,
            Signal::Supply(Flow::End(End::Reply | End::Stream)) | Signal::End(_)
        ),
        (_, _) => true,
    }
}

/// Every byte's exact stream/state interpretation is pinned as wire format.
#[test]
fn wire_byte_layout_snapshot() {
    let mut layout = String::new();
    for byte in u8::MIN..=u8::MAX {
        match WireSignal::parse(byte) {
            Ok(wire) => {
                let (stream, signal) = wire.into_parts();
                writeln!(
                    layout,
                    "{byte:02x}: stream {:02} {signal:?}",
                    stream.index()
                )
                .unwrap();
            }
            Err(invalid) => {
                writeln!(
                    layout,
                    "{byte:02x}: stream {:02} InvalidState({})",
                    invalid.stream().index(),
                    invalid.state(),
                )
                .unwrap();
            }
        }
    }
    insta::assert_snapshot!(layout);
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
