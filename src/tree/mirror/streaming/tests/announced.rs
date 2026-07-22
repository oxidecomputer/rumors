//! Bridge 3: announced-skeleton reconstruction — the payload-independence
//! bridge B5.
//!
//! The announced skeleton is reconstructible from the frame transcript
//! alone: payload contents carry the announcements charter locality
//! rests on, per channel (never globally — the terminal select draws
//! scheduler randomness).
//!
//! MODEL.md §1's extraction premise — the count and order of channel
//! operations depend only on each child's merge-join arm, never on payloads
//! — became load-bearing in the mux adjudication: σ*'s locality (hence
//! C1-literal's falsity) rests on every consumption-order discriminator
//! being announced in-band. This bridge checks both halves against real
//! sessions:
//!
//! - the *announced* skeleton, reconstructed from the payload-erased frame
//!   transcript alone (no tree access, no internal events — [`announced`]),
//!   equals the session's *actual* dispute skeleton as decoded from the
//!   internal progress trace;
//! - per channel, the session's op count and order are a function of that
//!   skeleton only: a payload-perturbed twin — identical paths, versions,
//!   and divergence pattern, different leaf contents, hence different
//!   hashes on every differing subtree — produces identical per-channel
//!   trace sequences and identical per-stream transcripts.
//!
//! Two deliberate scopings. Content perturbation (`u64` payloads) rather
//! than version perturbation: versions feed the handshake ceilings, so
//! perturbing them could flip role election and change the session for a
//! modeled reason; contents feed only hashes and supply bodies — exactly
//! the "payload" the premise erases. And the claim is per channel, not the
//! global interleaving: the model quantifies over interleavings
//! adversarially, and the real global publication order is not even a
//! function of the trees — `complete_initiator`'s terminal `tokio::select!`
//! is unbiased, so its branch order draws tokio's thread-local RNG and
//! reorders the tail of otherwise identical back-to-back runs (observed
//! while building this bridge: the committed regression seed reorders the
//! final absorb-side events between two runs of the SAME trees; the
//! per-channel projections are unaffected).

use proptest::prelude::*;

use super::fixtures::arb_divergence;
use super::skeleton::{announced, decode, trace_channels, transcript_streams};
use super::transcribed_mirror_sides;

proptest! {
    /// B5 ANNOUNCED-SKELETON RECONSTRUCTION: the wire transcript alone
    /// determines the dispute skeleton, and the skeleton alone determines
    /// every channel's op count and order.
    ///
    /// For any generated divergence, the skeleton reconstructed from the
    /// payload-erased transcript equals the session's actual dispute
    /// skeleton (with the transcript and the trace agreeing on who
    /// initiated), and a content-perturbed twin of the same divergence
    /// produces identical per-channel publication sequences and identical
    /// per-stream wire transcripts — never a function of payload bytes.
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
