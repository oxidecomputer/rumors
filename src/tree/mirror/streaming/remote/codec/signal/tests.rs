use std::collections::BTreeSet;

use super::*;

/// The semantic signal product maps bijectively onto bytes 0 through 237.
#[test]
fn encoding_is_bijective() {
    let mut bytes = BTreeSet::new();
    for (state, signal) in Signal::STATES.into_iter().enumerate() {
        let state = state as u8;
        assert_eq!(Signal::from_state(state), Ok(signal));
        for index in 0..Stream::COUNT {
            let stream = Stream::new(index).unwrap();
            let wire = WireSignal::new(stream, signal);
            let byte = wire.to_byte();
            assert!(bytes.insert(byte), "duplicate signal byte {byte:#04x}");
            assert_eq!(WireSignal::from_byte(byte).unwrap(), wire);
        }
    }
    assert_eq!(bytes.len(), usize::from(WireSignal::BYTE_COUNT));

    for byte in 0..=u8::MAX {
        match WireSignal::from_byte(byte) {
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
