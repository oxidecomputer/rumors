//! The self-delimiting frame grammar shared by every logical wire stream.
//!
//! Every frame is one CBOR array item, so a directed stream's frames form
//! an RFC 8742 CBOR sequence a generic tool can walk: `[signal]` for a
//! body-free frame, `[signal, body]` otherwise. The wire is *emitted* as
//! deterministic-encoding CBOR — shortest-form heads everywhere, definite
//! lengths only, one spelling per value
//! ([`cbor`](crate::tree::mirror::cbor)) — which is what keeps the
//! byte-pinning snapshot discipline meaningful. Ingress validates
//! structure and definite lengths everywhere; every head the codec
//! hand-parses (frame heads, signals, listings, run and record framing)
//! additionally rejects non-shortest spellings, while a record's version
//! atom and application payload are decoded by a general CBOR reader that
//! does not re-judge spelling — the atom's *content* canonicality is
//! enforced by its own strict decoder.
//!
//! The signal is an unsigned int carrying the dense `(frame state,
//! stream)` code. There are ten frame states — four reaction forms, each
//! continuing or ending its reply, plus a bare empty-reply end and a bare
//! stream-end control — and 17 streams. `state * 17 + stream` occupies
//! values 0 through 169; the rest of the byte-ranged code space is
//! reserved, and the signal's stream component deliberately restates the
//! transport label so a mislabeled stream is its own diagnosis. Speaker
//! and stream then select a phase-specific subset: the initiator admits
//! 162 placements and the responder 163, rejecting the rest before their
//! frame body is read.
//!
//! Reply and stream lifetimes are deliberately orthogonal. Every nonempty
//! reply ends on its final reaction; an empty reply is one bare reply-end
//! frame. After its final reply, a producer sends a separate bare stream-end
//! control. The session demultiplexer consumes that control and closes the
//! logical receiver, so the protocol adapter sees only complete replies. This
//! lets a lazy reply stream flush each item immediately without looking ahead
//! to discover whether that item is also the stream's last.
//!
//! An empty query is wholly represented by its signal. A nonempty query's
//! body is a `{radix: hash}` map of one to 256 children: CBOR
//! deterministic encoding mandates ascending keys and the wire's canonical
//! form mandates strictly ascending radixes, so the two disciplines are
//! one rule, enforced once at ingress. A supply body is a [`LeafRun`]
//! behind the embedded-CBOR-sequence tag (63) wrapping a byte string —
//! the byte-string head is the run's exact length, preserving O(1) skip
//! and up-front pricing — and the run's records are each the same shape
//! in miniature: tag 63 wrapping a byte string whose content is the
//! tagged version atom followed by the message's own CBOR payload. The
//! codec validates the run's record framing once its whole body arrives
//! but leaves the records encoded; the adapter decodes them one at a
//! time, constructs its backend-specific leaves, and validates their
//! version-derived paths. How many records share one run is the sender's
//! choice within the session's [`RunBudget`], and the decoder holds
//! arriving frames to that same budget: any within-budget batching
//! decodes, a single record of any size decodes (the encoder's
//! minimum-one-record overhang), and a frame batching multiple records
//! past the budget is rejected typed
//! ([`DecodeErrorKind::OverbatchedRun`]) before its body is buffered.
//!
//! Encoding trusts the protocol and adapter to produce phase-correct,
//! canonically ordered frames; it performs no redundant semantic validation.
//! Decoding is the trust boundary and validates every peer-controlled signal,
//! query, and supply-run structure before returning a frame. [`FrameRead`]
//! and [`FrameWrite`] apply that same grammar directly to Tokio byte streams
//! without buffering a complete outgoing frame.

mod budget;
#[cfg(any(test, feature = "test-internals"))]
mod capture;
mod decode;
mod encode;
mod error;
mod frame;
pub(crate) mod greeting;
mod signal;

#[cfg(test)]
pub use budget::SUPPLY_FRAME_OVERHEAD;
pub use budget::{DEFAULT_TARGET_MESSAGE_SIZE, RunBudget};

pub use crate::tree::mirror::cbor::HeadError;
#[cfg(any(test, feature = "test-internals"))]
pub use capture::{
    HookCapture, HookStream, LinkCapture, assert_items_account_for, render_hook_capture,
    stream_label,
};
pub use decode::FrameRead;
#[cfg(test)]
pub use decode::{decode, decode_exact};
pub use encode::FrameWrite;
#[cfg(test)]
pub use encode::encode;
pub use error::{
    DecodeError, DecodeErrorKind, DecodeLeafError, EncodeError, EncodeErrorKind, FramePart, Origin,
    QueryOrderError,
};
#[cfg(test)]
pub use frame::WireFrame;
pub use frame::{Frame, LeafRun, LeafRunError, Reaction};
#[cfg(test)]
pub(crate) use frame::{parse_listing_map, write_listing};
pub use signal::{
    DecodeSignalError, End, Flow, InvalidSignalPlacement, InvalidWireSignal, Speaker, Stream,
    StreamClass,
};

/// The whole wire prefix of one initiator-spoken, reply-ending supply
/// frame declaring a `declared`-byte run: the frame's array head, its
/// signal head, and the run's embedded-sequence tag and byte-string head.
///
/// The allocator meter (`tests/decode_alloc.rs`) prepends it to a
/// hand-built run body so the codec's supply read path is drivable from
/// outside the crate.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn supply_frame_head(declared: usize) -> Vec<u8> {
    use crate::tree::mirror::cbor;
    let code = signal::WireSignal::encode(
        Stream::new(0).expect("stream 0 is within the stream range"),
        signal::Signal::Supply(Flow::End),
    );
    let mut bytes = Vec::new();
    cbor::write_head(&mut bytes, cbor::MAJOR_ARRAY, 2);
    cbor::write_head(&mut bytes, cbor::MAJOR_UINT, u64::from(code));
    cbor::write_tag(&mut bytes, cbor::TAG_CBOR_SEQUENCE);
    cbor::write_head(&mut bytes, cbor::MAJOR_BSTR, declared as u64);
    bytes
}

/// A structurally valid lone-record run of exactly `len` bytes: one
/// record whose heads plus arbitrary content span the run.
///
/// Record content decodes lazily, so the meter's bodies need only pass
/// run-record framing. Panics when no single record's head widths can
/// reach `len` exactly (the head-width gaps); the meters' lengths are
/// chosen away from those gaps.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn lone_record_run(len: usize) -> Vec<u8> {
    use crate::tree::mirror::cbor;
    for width in [1usize, 2, 3, 5, 9] {
        let Some(content) = len.checked_sub(cbor::head_len(cbor::TAG_CBOR_SEQUENCE) + width) else {
            continue;
        };
        if cbor::head_len(content as u64) != width {
            continue;
        }
        let mut run = Vec::with_capacity(len);
        cbor::write_tag(&mut run, cbor::TAG_CBOR_SEQUENCE);
        cbor::write_head(&mut run, cbor::MAJOR_BSTR, content as u64);
        run.extend((0..content).map(|i| i as u8));
        return run;
    }
    panic!("no lone record spans exactly {len} bytes");
}

/// Decode one initiator-spoken frame from `read` under `budget`, dropping
/// the decoded value.
///
/// The allocator meter (`tests/decode_alloc.rs`) drives the supply read path
/// through this to price a supply body in bytes requested from the
/// allocator; the decoded value is noise to that meter, but the typed error
/// passes through so the meter can also assert how a failure classified.
/// The budget is the meter's to choose: the framing ceiling keeps every
/// well-framed declaration on the body-read path being priced, while a
/// small budget prices the ingress gate itself.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) async fn decode_frame_discarded(
    read: impl tokio::io::AsyncRead + Unpin,
    budget: RunBudget,
) -> Result<(), DecodeError> {
    let mut read = FrameRead::new(Speaker::Initiator, budget, read);
    read.frame().await.map(|_| ())
}

#[cfg(test)]
mod tests;
