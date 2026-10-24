//! The typed boundary between the focused-fuel guest and its native harness.
//!
//! The host names one [`Probe`] and two integer parameters. The guest builds
//! the probe's operands, reads Wasmtime fuel around one call of the measured
//! operation, checks the operation's answer outside that window, and keeps
//! the reading, and the operation's units of work where the probe states
//! them, for the host. A wrong answer is a guest panic, which the host
//! reports with its message.
//!
//! Each probe documents what parameters `a` and `b` select. A parameter a
//! probe does not use must be zero.

#![cfg_attr(not(test), no_std)]

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

    /// `Span::dominance` of the right-half plateau `R` against `[C, C | R]`,
    /// where `C` is the comb `C(1024, a)`: a start concurrent with the probe,
    /// refuted at the first interval.
    ExitDominanceConcurrentStart = 50,
    /// `Span::dominance` of the one-tick version `S` against `[S | C,
    /// above(S | C)]`: a start strictly above the probe, refuted at the first
    /// interval; `C` is `C(1024, a)`.
    ExitDominanceDominatingStart = 51,
    /// `Span::precedence` of `C(1024, a)` against `[empty, R]`: an end
    /// concurrent with the probe, refuted at the first interval.
    ExitPrecedenceConcurrentEnd = 52,
    /// `Span::precedence` of `S | C` against `[empty, S]`: an end strictly
    /// below the probe, refuted at the first interval; `C` is `C(1024, a)`.
    ExitPrecedencePrecededEnd = 53,
    /// `Span::contains` of `S | C` against `[empty, S]`: a probe above the
    /// end, refuted at the first interval; `C` is `C(1024, a)`.
    ExitContainsAboveEnd = 54,
    /// `Span::contains` of `S` against `[S | C, above(S | C)]`: a probe below
    /// the start, refuted at the first interval; `C` is `C(1024, a)`.
    ExitContainsBelowStart = 55,
    /// `Span::dominance` of `S` against `[empty, S | C]`: a contained probe
    /// whose end stream is dropped at its first interval; `C` is `C(1024, a)`.
    ExitDominanceDropsTheEnd = 56,
    /// `Span::precedence` of the uniform `U = 2^65` against `[C(64, a), U +
    /// 1]`: a contained probe whose start stream is dropped at its first
    /// interval.
    ExitPrecedenceDropsTheStart = 57,
    /// `Query::from(causally::before(S)).contains(S | C)`: a ceiling refuted
    /// at the first interval; `C` is `C(1024, a)`.
    ExitCeilingRefuted = 58,
    /// `causally::since(S).contains(S | C)`: a hole satisfied at the first
    /// interval; `C` is `C(1024, a)`.
    ExitHoleSatisfied = 59,

    /// `x.partial_cmp(y)` over the history of `a` senders;
    /// `b` packs the snapshots `[x, y]` (see [`pack`]).
    HistoryCompare = 100,
    /// `Query::from(causally::before(x)).contains(y)` over the history of
    /// `a` senders; `b` packs `[x, y]`.
    HistoryBefore = 101,
    /// `causally::before(x).contains(y)`, the ceiling's own membership test,
    /// over the history of `a` senders; `b` packs `[x, y]`.
    HistoryBeforeBound = 102,
    /// `causally::after(x).contains(y)`, the floor's own membership test,
    /// over the history of `a` senders; `b` packs `[x, y]`.
    HistoryAfterBound = 103,
    /// `causally::since(x).contains(y)` over the history of `a` senders;
    /// `b` packs `[x, y]`.
    HistorySince = 104,
    /// `(causally::since(x) & causally::before(y)).contains(z)` over the
    /// history of `a` senders; `b` packs `[x, y, z]`.
    HistorySinceBefore = 105,
    /// `x & y` over the history of `a` senders; `b` packs `[x, y]`.
    HistoryMeet = 106,
    /// `x | y` over the history of `a` senders; `b` packs `[x, y]`.
    HistoryJoin = 107,
    /// `x.span(y)` over the history of `a` senders; `b` packs `[x, y]`.
    HistorySpan = 108,
    /// `x.span_all([y])` over the history of `a` senders; `b` packs `[x, y]`.
    HistorySpanAll = 109,
    /// `Span::new(lo, hi).place(probe)` over the history of `a` senders;
    /// `b` packs `[lo, hi, probe]`.
    SpanPlace = 110,
    /// `Span::new(lo, hi).dominance(probe)` over the history of `a` senders;
    /// `b` packs `[lo, hi, probe]`.
    SpanDominance = 111,
    /// `Span::new(lo, hi).precedence(probe)` over the history of `a` senders;
    /// `b` packs `[lo, hi, probe]`.
    SpanPrecedence = 112,
    /// `Span::new(lo, hi).contains(&probe)` over the history of `a` senders;
    /// `b` packs `[lo, hi, probe]`.
    SpanContains = 113,
    /// `Span::new(lo, hi).contains(probe.clone())`, an owned argument, over
    /// the history of `a` senders; `b` packs `[lo, hi, probe]`.
    SpanContainsOwned = 114,
    /// `Span::new(lo, hi).contains(Span::at(&probe))`, a point-span argument
    /// sharing the probe's buffer, over the history of `a` senders; `b` packs
    /// `[lo, hi, probe]`.
    SpanContainsPoint = 115,
    /// `Span::new(lo, hi).contains(&coincident)`, where `coincident` spans
    /// the probe and its byte-equal re-decode, two distinct buffers, over the
    /// history of `a` senders; `b` packs `[lo, hi, probe]`.
    SpanContainsDistinctPoint = 116,
    /// `endpoint.span(&endpoint.clone()).place(probe)`, a span whose two
    /// endpoints share one buffer, over the history of `a` senders; `b` packs
    /// `[endpoint, probe]`.
    CoincidentPlace = 117,
    /// `endpoint.span(&endpoint.clone()).dominance(probe)` over the history
    /// of `a` senders; `b` packs `[endpoint, probe]`.
    CoincidentDominance = 118,
    /// `Span::new(endpoint, redecoded).place(probe)`, coincident endpoints in
    /// distinct buffers, over the history of `a` senders; `b` packs
    /// `[endpoint, probe]`.
    DistinctPlace = 119,
    /// `Span::new(endpoint, redecoded).dominance(probe)` over the history of
    /// `a` senders; `b` packs `[endpoint, probe]`.
    DistinctDominance = 120,
    /// `start.span_all(population)` over the history of `a` senders, whose
    /// population holds every seventh snapshot of its first two phases.
    PopulationSpanAll = 121,
    /// `start.meet_all(population)` over the history of `a` senders.
    PopulationMeetAll = 122,
    /// `start.join_all(population)` over the history of `a` senders.
    PopulationJoinAll = 123,
    /// `x.partial_cmp(x')` over the history of `a` senders, where `x'` is the
    /// byte-equal re-decode of `x` in a buffer of its own, so the comparison
    /// walks both streams to exhaustion; `b` packs `[x]`.
    HistoryRedecodedCompare = 127,
    /// `Span::decode` of the canonical wire bytes of the span `[lo, hi]` over
    /// the history of `a` senders; `b` packs `[lo, hi]`.
    SpanDecode = 124,
    /// `Version::decode` of the span `[lo, hi]`'s first component, the
    /// encoding of `lo`, over the history of `a` senders; `b` packs
    /// `[lo, hi]`.
    SpanDecodeLower = 125,
    /// `Version::decode` of the span `[lo, hi]`'s second component, the
    /// encoding of `hi`, over the history of `a` senders; `b` packs
    /// `[lo, hi]`.
    SpanDecodeUpper = 126,

    /// `x.join_all(copies)` over the duplicate fixture of `a` shares, with `b`
    /// copies.
    ///
    /// The fixture is a history ticked on `a` shares of the seed (16 or 64),
    /// and `x` and `d` are each one tick past it on two different shares, so
    /// they are concurrent; the copies are `b` clones of `d`, all sharing one
    /// buffer.
    DuplicateVersionJoinAll = 200,
    /// `x.meet_all(copies)` over the duplicate fixture of `a` shares, with `b`
    /// copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateVersionMeetAll = 201,
    /// `x.span_all(copies)` over the duplicate fixture of `a` shares, with `b`
    /// copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateVersionSpanAll = 202,
    /// `Sum<&Version>` over `x` then the copies over the duplicate fixture of
    /// `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateSumBorrowed = 203,
    /// `Sum<Version>` over `x` then owned copies over the duplicate fixture of
    /// `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateSumOwned = 204,
    /// `FromIterator<Version>` over `x` then owned copies over the duplicate
    /// fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateCollectOwned = 205,
    /// `FromIterator<&Version>` over `x` then the copies over the duplicate
    /// fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateCollectBorrowed = 206,
    /// `Span::union_all` of the span to `x` over the copied span to `d` over
    /// the duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateSpanUnionAll = 207,
    /// `Span::intersect_all` of the span to `x` over the copied span to `d`
    /// over the duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateSpanIntersectAll = 208,
    /// `Span::join_all` of the span to `x` over the copied span to `d` over the
    /// duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateSpanJoinAll = 209,
    /// `Span::meet_all` of the span to `x` over the copied span to `d` over the
    /// duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateSpanMeetAll = 210,
    /// `Span::at(&x).join_all(copies)` over the duplicate fixture of `a`
    /// shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicatePointJoinAll = 211,
    /// `Option<Span>: Sum<&Span>` over the span to `x` then copied spans to `d`
    /// over the duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanSumBorrowedSpans = 212,
    /// `Option<Span>: Sum<Span>` over the span to `x` then owned copied spans
    /// over the duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanSumOwnedSpans = 213,
    /// `Option<Span>: Sum<&Version>` over `x` then the copies over the
    /// duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanSumBorrowedVersions = 214,
    /// `Option<Span>: Sum<Version>` over `x` then owned copies over the
    /// duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanSumOwnedVersions = 215,
    /// `Option<Span>: FromIterator<&Span>` over the span to `x` then copied
    /// spans over the duplicate fixture of `a` shares, with `b` copies of `d`,
    /// as in [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanCollectBorrowedSpans = 216,
    /// `Option<Span>: FromIterator<Span>` over the span to `x` then owned
    /// copied spans over the duplicate fixture of `a` shares, with `b` copies
    /// of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanCollectOwnedSpans = 217,
    /// `Option<Span>: FromIterator<&Version>` over `x` then the copies over the
    /// duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanCollectBorrowedVersions = 218,
    /// `Option<Span>: FromIterator<Version>` over `x` then owned copies over
    /// the duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanCollectOwnedVersions = 219,
    /// `Option<Span>: Product<&Span>` over the span to `x` then copied spans
    /// over the duplicate fixture of `a` shares, with `b` copies of `d`, as in
    /// [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanProductBorrowedSpans = 220,
    /// `Option<Span>: Product<Span>` over the span to `x` then owned copied
    /// spans over the duplicate fixture of `a` shares, with `b` copies of `d`,
    /// as in [`DuplicateVersionJoinAll`](Probe::DuplicateVersionJoinAll).
    DuplicateOptionSpanProductOwnedSpans = 221,

    /// `Clock::sync` of two clocks with empty versions whose parties are the
    /// delegated pair at depth `a`: two `a`-level spines that share their
    /// path and split their deep cells.
    SyncDelegated = 300,
    /// `Clock::sync` of two clocks with empty versions whose parties are the
    /// spliced pair at depth `a`: the left half and a deep cell of the right.
    SyncSplice = 301,
    /// `Clock::sync` of two clocks with empty versions whose parties are the
    /// adjacent pair at depth `a`: the leftmost `2^-a` cell and its sibling.
    SyncAdjacent = 302,
    /// `Party::join` then `Party::fork` of the delegated pair at depth `a`.
    JoinForkDelegated = 303,
    /// `Party::join` then `Party::fork` of the spliced pair at depth `a`.
    JoinForkSplice = 304,
    /// `Party::join` then `Party::fork` of the adjacent pair at depth `a`.
    JoinForkAdjacent = 305,
    /// `Party::without` of a party holding the leftmost `2^-a` cell and the
    /// right half, less one quarter of the right half: the deep cell lies
    /// under an unowned cover and survives whole.
    WithoutSplice = 306,
    /// `Party::without` of a party holding the leftmost `2^-a` cell and the
    /// right half, less the left half and one quarter: the deep cell lies
    /// under an owned cover and vanishes whole.
    WithoutBlock = 307,
    /// `Party::seed().without(leftmost)`, carving the leftmost `2^-a` cell
    /// out of the seed: the walk builds the cell's complement level by level.
    WithoutComplement = 308,

    /// `later <= empty`, a directed comparison, for the comb `later =
    /// C(1024, a)` against the empty version.
    DirectedLe = 400,
    /// `later.partial_cmp(empty)`, the full relation, for the comb
    /// `C(1024, a)`.
    DirectedFullCompare = 401,
    /// `Span::at(later).dominance(empty)` for the comb `C(1024, a)`.
    DirectedPointDominance = 402,
    /// `causally::after(later).contains(empty)` for the comb `C(1024, a)`.
    DirectedFloorContains = 403,
    /// `projection <= empty` for the comb `C(1024, a)` viewed through the
    /// seed.
    DirectedProjectedLe = 404,
    /// `projection.partial_cmp(empty)` for the comb `C(1024, a)` viewed
    /// through the seed.
    DirectedProjectedCompare = 405,

    /// `flat | spine`, where `spine` is the re-anchoring spine of depth `a`
    /// and `flat` a one-leaf version of height `2^(10a) − 1`, which dominates
    /// it; the units are the operands' encoded bits.
    ReanchorJoinFlatFirst = 500,
    /// `spine | flat`, the other operand order of
    /// [`ReanchorJoinFlatFirst`](Probe::ReanchorJoinFlatFirst).
    ReanchorJoinSpineFirst = 501,

    /// `join_all` over the staggered population of `a` versions, the first
    /// as the receiver; the units are the population's encoded bytes.
    StaggerJoinAll = 600,
    /// `meet_all` over the staggered notch population of `a` versions, the
    /// first as the receiver; the units are its encoded bytes.
    StaggerMeetAll = 601,
    /// `span_all` over the staggered population of `a` versions, the first
    /// as the receiver; the units are its encoded bytes.
    StaggerSpanAll = 602,
    /// `Party::join_all` over the staggered parties of `a` ids, the first as
    /// the receiver; the units are their encoded bytes.
    StaggerPartyJoinAll = 603,
    /// `Clock::join_all` over clocks pairing each staggered id with its
    /// staggered version, the first as the receiver; the units are their
    /// encoded bytes.
    StaggerClockJoinAll = 604,
}

/// A snapshot of the history fixture, which a history probe's parameter
/// `b` names.
///
/// The fixture is one clock's history with `a` forked senders, in phases of
/// four rounds per sender, each round a tick and a received send from the
/// next sender, so every stream has one region per sender and grows with
/// `a`. In order: [`Start`](Self::Start) after one
/// phase; [`Middle`](Self::Middle) after a raise of `2^80` and a second phase,
/// so streams carry wide codes; [`End`](Self::End) after a third phase;
/// [`Divergent`](Self::Divergent), a fork taken at `Start` and ticked once
/// per round, concurrent with `Middle` and `End`; and [`Top`](Self::Top), the join
/// of `End` and `Divergent`. So `Start < Middle < End <= Top`, `Start <=
/// Divergent <= Top`, and `Divergent` is concurrent with `Middle` and `End`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, FromRepr)]
#[repr(u8)]
pub enum Snapshot {
    /// The snapshot after the first phase.
    Start = 1,
    /// The snapshot after the wide raise and the second phase.
    Middle = 2,
    /// The snapshot after the third phase.
    End = 3,
    /// The fork taken at `Start`, ticked once per round.
    Divergent = 4,
    /// The join of `End` and `Divergent`.
    Top = 5,
}

/// The bits [`pack`] gives each snapshot.
const SNAPSHOT_BITS: u32 = 4;

/// Packs `snapshots` into a probe parameter, the first in the lowest bits.
///
/// Inverse to [`unpack`].
pub const fn pack<const N: usize>(snapshots: [Snapshot; N]) -> u64 {
    let mut packed = 0;
    let mut index = N;
    while index > 0 {
        index -= 1;
        packed = (packed << SNAPSHOT_BITS) | snapshots[index] as u64;
    }
    packed
}

/// Unpacks exactly `N` snapshots from a probe parameter, or `None` if it
/// holds fewer or more than `N`, or names a snapshot that does not exist.
///
/// No snapshot is numbered zero, so an absent snapshot cannot be mistaken
/// for one, and a parameter packed with one arity never unpacks at another.
///
/// Inverse to [`pack`].
pub fn unpack<const N: usize>(mut packed: u64) -> Option<[Snapshot; N]> {
    let mut snapshots = [Snapshot::Start; N];
    for snapshot in &mut snapshots {
        let low = u8::try_from(packed & ((1 << SNAPSHOT_BITS) - 1)).ok()?;
        *snapshot = Snapshot::from_repr(low)?;
        packed >>= SNAPSHOT_BITS;
    }
    (packed == 0).then_some(snapshots)
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

#[cfg(test)]
mod tests;
