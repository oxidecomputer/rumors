//! The typed boundary between the focused-fuel guest and its native harness.
//!
//! The host names one [`Probe`] and two integer parameters. The guest builds
//! the probe's operands, reads Wasmtime fuel around one call of the measured
//! operation, checks the operation's answer outside that window, and keeps
//! the reading for the host. A wrong answer is a guest panic, which the host
//! reports with its message.
//!
//! Each probe documents what parameters `a` and `b` select. A parameter a
//! probe does not use must be zero.

#![no_std]

use strum_macros::FromRepr;

/// One measured operation on operands the guest builds from two parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq, FromRepr)]
#[repr(u32)]
pub enum Probe {
    /// The semantic equality walk (`testing::meter::version::equal`) of the
    /// boundary comb `C(1024, a)` against the empty version, which the first
    /// elementary interval refutes; `a` is the tooth count.
    EqRefutedAtFirstInterval = 0,

    /// `partial_cmp` of a dense spine of depth `a` with its clone.
    CloneCompare = 10,
    /// `|` of a dense spine of depth `a` with its clone.
    CloneJoin = 11,
    /// `&` of a dense spine of depth `a` with its clone.
    CloneMeet = 12,
    /// `span` of a dense spine of depth `a` with its clone.
    CloneSpan = 13,
    /// `join_all` of a dense spine of depth `a` over its clone and itself.
    CloneJoinAll = 14,
    /// `meet_all` of a dense spine of depth `a` over its clone and itself.
    CloneMeetAll = 15,
    /// `span_all` of a dense spine of depth `a` over its clone and itself.
    CloneSpanAll = 16,
    /// `distance` of a dense spine of depth `a` to its clone.
    CloneDistance = 17,
    /// `lag` of a dense spine of depth `a` behind its clone.
    CloneLag = 18,

    /// `partial_cmp` of a dense spine of depth `a` with its byte-equal
    /// re-decode, which holds a distinct buffer.
    RedecodedCompare = 20,
    /// `|` of a dense spine of depth `a` with its byte-equal re-decode.
    RedecodedJoin = 21,
    /// `&` of a dense spine of depth `a` with its byte-equal re-decode.
    RedecodedMeet = 22,
    /// `span` of a dense spine of depth `a` with its byte-equal re-decode.
    RedecodedSpan = 23,
    /// `distance` of a dense spine of depth `a` to its byte-equal re-decode.
    RedecodedDistance = 24,
    /// `lag` of a dense spine of depth `a` behind its byte-equal re-decode.
    RedecodedLag = 25,

    /// `empty | v` for a dense spine `v` of depth `a`.
    EmptyJoinLeft = 30,
    /// `v | empty` for a dense spine `v` of depth `a`.
    EmptyJoinRight = 31,
    /// `empty & v` for a dense spine `v` of depth `a`.
    EmptyMeetLeft = 32,
    /// `v & empty` for a dense spine `v` of depth `a`.
    EmptyMeetRight = 33,
    /// `empty.span(v)` for a dense spine `v` of depth `a`.
    EmptySpanLeft = 34,
    /// `v.span(empty)` for a dense spine `v` of depth `a`.
    EmptySpanRight = 35,
    /// `acc |= v` on an empty `acc`, for a dense spine `v` of depth `a`.
    EmptyJoinAssignLeft = 36,
    /// `acc |= empty` on `acc = v`, for a dense spine `v` of depth `a`.
    EmptyJoinAssignRight = 37,
    /// `acc &= v` on an empty `acc`, for a dense spine `v` of depth `a`.
    EmptyMeetAssignLeft = 38,
    /// `acc &= empty` on `acc = v`, for a dense spine `v` of depth `a`.
    EmptyMeetAssignRight = 39,
    /// `v.join_all([empty])` for a dense spine `v` of depth `a`.
    EmptyJoinAllRight = 40,
    /// `empty.join_all([v])` for a dense spine `v` of depth `a`.
    EmptyJoinAllLeft = 41,
    /// `v.meet_all([empty])` for a dense spine `v` of depth `a`.
    EmptyMeetAllRight = 42,
    /// `empty.meet_all([v])` for a dense spine `v` of depth `a`.
    EmptyMeetAllLeft = 43,
}

impl From<Probe> for u32 {
    /// Returns the stable number that names `probe` across the wasm ABI.
    fn from(probe: Probe) -> Self {
        probe as u32
    }
}

impl TryFrom<u32> for Probe {
    type Error = ();

    /// Decodes a probe number, the inverse of `u32::from`.
    ///
    /// Rejects values that name no probe. `FromRepr` derives this decoder
    /// from the declared numbers, so it cannot disagree with them.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_repr(value).ok_or(())
    }
}

/// The guest measured the probe and holds its reading.
pub const MEASURED: u32 = 0;

/// The host supplied a probe number this guest does not know.
pub const UNKNOWN_PROBE: u32 = 1;
