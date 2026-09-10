//! Channel order and message contents: a check of model premise B5.
//!
//! These tests reconstruct the dispute skeleton from the payload-erased
//! frame transcript and compare it with the walk's internal progress trace.
//! The comparison checks that frames announce the choices which determine
//! how a session consumes its channels.
//!
//! Each case also runs with different message contents but identical paths
//! and versions. Versions determine node hashes and role election, so those
//! remain fixed. Changing message bodies must preserve the publication
//! sequence on each channel and the frame sequence on each stream.
//!
//! The formal model's payload-independence premise concerns each channel's
//! operation count and order. Cross-channel interleavings are outside this
//! comparison: the model allows them to vary. Local scheduling replay is
//! checked separately by `local_session_schedule_replays`.

use proptest::prelude::*;

use super::fixtures::arb_divergence;
use super::skeleton::{announced, decode, trace_channels, transcript_streams};
use super::transcribed_mirror_sides;

proptest! {
    /// Frames reconstruct the walk's dispute skeleton. Changing only message
    /// contents preserves each channel's publication order and each stream's
    /// payload-erased frame sequence.
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
            "the frame transcript alone reconstructs the dispute skeleton"
        );
        prop_assert_eq!(
            reconstructed.initiator,
            decoded.initiator,
            "the transcript's causally-first opening names the initiator"
        );

        // The payload twin: same divergence, every leaf's content changed.
        let (_, _, twin_trace, twin_transcript) = run(0x0abe_1e57ed_u64);
        prop_assert_eq!(
            trace_channels(&trace),
            trace_channels(&twin_trace),
            "per-channel op count and order are payload-independent"
        );
        prop_assert_eq!(
            transcript_streams(&transcript),
            transcript_streams(&twin_transcript),
            "the payload-erased per-stream wire transcript is payload-independent"
        );
    }
}
