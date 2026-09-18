//! Reconstruction of the dispute skeleton from outgoing replies.
//!
//! These tests reconstruct the dispute skeleton from the payload-erased
//! reply transcript and compare it with the walk's internal progress trace.
//! The comparison checks that replies announce every choice needed to
//! determine how a session consumes its channels.
//!
//! Each case also runs with different message contents but identical paths and
//! versions. This changes the leaf and ancestor hashes while preserving the
//! dispute shape and role election. The publication sequence on each channel
//! and reply sequence on each stream must remain unchanged.
//!
//! The formal model's payload-independence premise concerns each channel's
//! operation count and order. Cross-channel interleavings are outside this
//! comparison: the model allows them to vary. Local scheduling replay is
//! checked separately by `local_session_schedule_replays`.

use proptest::prelude::*;

use super::fixtures::arb_divergence;
use super::skeleton::{announced, decode, trace_channels, transcript_streams};
use super::transcribed_mirror_sides;

/// A message body distinct from the fixture's baseline value.
const TWIN_PAYLOAD: u64 = 1;

proptest! {
    /// Outgoing replies reconstruct the walk's dispute skeleton. Changing only
    /// message contents preserves each channel's publication order and each
    /// stream's payload-erased reply sequence.
    #[test]
    fn announced_skeleton_reconstructs_the_session(spec in arb_divergence()) {
        let run = |value: u64| {
            let (local, remote, _) = spec.trees(&value);
            transcribed_mirror_sides(local, remote)
        };

        let (ours, theirs, trace, transcript) = run(0);
        prop_assert_eq!(ours, theirs, "sanity: the session converges");

        let decoded = decode(&trace);
        let reconstructed = announced(&transcript);
        prop_assert_eq!(
            &reconstructed.skel,
            &decoded.skel,
            "the reply transcript alone reconstructs the dispute skeleton"
        );
        prop_assert_eq!(
            reconstructed.initiator,
            decoded.initiator,
            "the transcript's causally-first opening names the initiator"
        );

        // The payload twin: same divergence, every leaf's content changed.
        let (_, _, twin_trace, twin_transcript) = run(TWIN_PAYLOAD);
        prop_assert_eq!(
            trace_channels(&trace),
            trace_channels(&twin_trace),
            "per-channel op count and order are payload-independent"
        );
        prop_assert_eq!(
            transcript_streams(&transcript),
            transcript_streams(&twin_transcript),
            "the payload-erased per-stream replies are payload-independent"
        );
    }
}
