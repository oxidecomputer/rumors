//! The packed bit-stream storage forms: the mutable build buffer ([`BitsMut`]),
//! the refcounted frozen at-rest form ([`Bits`]), and the canonicality helpers
//! both rest on.
//!
//! # The identity fast-path ladder, and where each rung belongs
//!
//! Equality of frozen streams is a three-rung ladder: clone identity
//! ([`Bits::ptr_eq`]), then the one-`memcmp` byte compare ([`canonical_eq`]'s
//! second rung), then whatever walk the operation would run anyway. The
//! operations choose deliberately how far down the ladder to look before
//! walking. The decision rule, applied per call site (each site cites its law
//! and states its choice):
//!
//! - **`ptr_eq` is free insurance**: `O(1)` on hit and miss alike
//!   (two word compares), so every site where equality settles the
//!   answer takes it. Clones share buffers, so the rung fires wherever
//!   a value meets its own clone.
//!
//! - **The `memcmp` rung pays for itself exactly where the walk it
//!   replaces is expensive relative to a byte scan.** A miss costs an
//!   early-exiting byte compare over the operands' shared prefix —
//!   byte-parallel, roughly an order of magnitude cheaper per bit than
//!   a decoding walk — and a hit deletes the walk whole. The
//!   arithmetic and allocating walks take it: join/meet/span (an
//!   emission plus its buffers), `distance`/`lag` (accumulator folds
//!   and `Base` products), `Ranked`'s total order (the rank co-sweep).
//!   Equal operands are also *common* at those seams: idempotent
//!   re-joins, converged replicas, metric self-checks.
//!
//! - **The comparison sweep keeps ptr-only.** `causal_cmp`'s fallback
//!   is itself a cheap early-exiting scan, and an *order* query's
//!   common case is unequal operands — where a memcmp rung can only
//!   answer `Equal`, so a miss double-reads the shared prefix (long,
//!   for causally related versions) and still owes the sweep. The
//!   equal case that matters in production is clone-borne, which the
//!   ptr rung catches. (Byte-equal wire-decoded operands at gossip
//!   convergence are the one workload where a memcmp rung in cmp
//!   might win; adopt only with measurements in hand.)
//!
//! - **The party predicates take no rung at all.** `covers` and
//!   `is_disjoint` are asked of *linear* values: a live clone-shared
//!   party pair has no production witness (`dangerously_alias` is a
//!   boundary hand-off, not a live operand pair), so a rung there
//!   would dispatch only inside test harnesses — while costing the
//!   fuel-band instrument its full-walk anchor samples.
//!
//! Two instruments hold the ladder honest: the `identity_fast_paths` pins in
//! `tests/meter.rs` assert exactly zero walk work on every adopted rung beside
//! walking legs on the same values (a lost rung or a dead meter both read red),
//! and the fuzzfit fuel bands route identity-outcome samples out of their fits
//! so the walked laws stay unimodal (`Step::identity` in the harness carries
//! that argument).

// The storage forms' docs name crate-private machinery by intra-doc link
// (`ptr_eq`, `canonical_eq`) so a rename cannot rot the prose (the internal doc
// build resolves every link); on the public build those links render as plain
// code spans — the items are private — which this allow accepts.
#![allow(rustdoc::private_intra_doc_links)]

use core::hash::Hasher;
use core::ops::Deref;

use bitvec::domain::Domain;
use bitvec::prelude::*;
use bytes::Bytes;

use crate::error::Decode;

/// The mutable build-side form of a packed bit stream: a
/// most-significant-bit-first bit vector over bytes.
///
/// Every emitter and builder writes into one of these (the crate's
/// packed-stream builder wraps one with the metered move set); a finished
/// stream freezes into the at-rest [`Bits`] at the storage seam. The
/// `Bytes`/`BytesMut` naming echo is deliberate: `BitsMut` is where mutation
/// happens, [`Bits`] is the shared, immutable result.
pub type BitsMut = BitVec<u8, Msb0>;

/// A borrowed view of a packed bit stream, mutable or frozen.
pub type BitsSlice = BitSlice<u8, Msb0>;

/// The at-rest storage form of a `Party`/`Version`: the canonical packed
/// preorder bit stream, marker-padded to a byte boundary, over a refcounted
/// byte buffer.
///
/// The raw byte slice ([`as_raw_slice`](Self::as_raw_slice)) *is* the wire
/// encoding: the live bits, then a single `1` marker bit, then zero bits to the
/// byte boundary — the empty stream is the empty byte string, no marker and no
/// bytes. The marker makes the bytes *injective* on bit streams: the buffer's
/// final set bit ends the stream's spelling, so no two streams share a byte
/// string, byte equality alone is stream equality, and the live length is
/// recovered in `O(1)` from the final byte ([`len`](Self::len)) rather than
/// stored beside the buffer or owed a length header on the wire.
///
/// The backing store is [`Bytes`], so [`Clone`] is a refcount bump: two clones
/// share one buffer, cost `O(1)`, and that shared identity is observable
/// through [`ptr_eq`](Self::ptr_eq) — the fast path the identity-law shortcuts
/// (`x ∨ x`, `cmp(x, x)`, `distance(x, x)`) dispatch on. Reading is still
/// `bitvec`'s: the struct [derefs](core::ops::Deref) to [`BitsSlice`], which
/// exposes exactly the live bits — the padding stays behind the view — so
/// every cursor and walk consumes the frozen form exactly as it consumes a
/// [`BitsMut`].
#[derive(Clone)]
pub struct Bits {
    /// The canonical marker-padded bytes: the live bits, one `1`, then
    /// zeros to the byte boundary — `(len + 1).div_ceil(8)` bytes
    /// exactly; the empty stream is the empty buffer.
    bytes: Bytes,
}

impl Bits {
    /// The frozen empty stream: no bits, no bytes, no allocation.
    pub(crate) fn empty() -> Self {
        Bits {
            bytes: Bytes::new(),
        }
    }

    /// Freeze a built stream into the at-rest form, canonicalizing its storage:
    /// the single gate between the mutable build-side world and the shared
    /// frozen one.
    ///
    /// Seals the padding ([`seal_padding`]: the `1` marker, then zeroed dead
    /// bits — see the type docs for what the marker underpins, and
    /// [`seal_padding`] for why a tree op can leave the tail dirty), then
    /// adopts the buffer without copying: [`BitVec::into_vec`] hands back the
    /// underlying allocation and `Bytes::from(vec)` wraps it in place.
    pub(crate) fn freeze(mut buf: BitsMut) -> Self {
        seal_padding(&mut buf);
        Bits {
            bytes: Bytes::from(buf.into_vec()),
        }
    }

    /// Adopt already-canonical bytes as a frozen stream: the decode-side door,
    /// for buffers whose padding a validator has already proven canonical.
    ///
    /// `bytes` must be marker-padded — the live bits, one `1`, then zeros to
    /// the byte boundary; empty for the empty stream — which is what
    /// `require_marker_padding` accepts. Debug builds assert it; release builds
    /// trust the validator.
    ///
    /// # Panics
    ///
    /// Panics when the buffer's bit count exceeds `usize` — reachable only on
    /// targets narrower than 64 bits, from 512 MiB of buffer. Stored bit
    /// positions and lengths are `usize`-denominated throughout the crate, so
    /// a stream past that bound has no in-memory form here: adopting it would
    /// make [`len`](Self::len) and every walk incorrect, and this check keeps
    /// the failure at the door instead. On 64-bit targets the bound is 2^61
    /// bytes, which no allocator can hand over, so the check is dead there.
    pub(crate) fn from_canonical(bytes: Bytes) -> Self {
        assert!(
            bytes.len() as u128 <= (usize::MAX as u128 + 1) / 8,
            "stored streams denominate bit positions in usize: \
             a {}-byte buffer's bits do not fit this target's usize",
            bytes.len(),
        );
        let bits = Bits { bytes };
        debug_assert!(
            padding_is_canonical(&bits),
            "from_canonical: the buffer must end in the canonical `1 0*` padding",
        );
        bits
    }

    /// The live bit length of the stream, recovered from the padding.
    ///
    /// The marker is the buffer's final set bit, so the length is one
    /// `trailing_zeros` over the final byte: `O(1)`, no walk. The one storage
    /// invariant this rests on — a nonempty buffer's final byte is nonzero — is
    /// exactly what the freeze and decode doors establish.
    pub fn len(&self) -> usize {
        match self.bytes.last() {
            None => 0,
            Some(&last) => {
                debug_assert!(last != 0, "stored stream missing its padding marker");
                self.bytes.len() * 8 - 1 - last.trailing_zeros() as usize
            }
        }
    }

    /// Whether the stream holds no bits at all.
    ///
    /// This is emptiness of the *storage* (the anonymous id), not of the value
    /// a stream spells: the empty `Version` is a 2-bit stream.
    /// [`len`](Self::len)'s conventional partner; the walks ask the question of
    /// slices, so only the meter surface and the tests reach it.
    #[cfg(any(test, feature = "meter"))]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The canonical marker-padded bytes: the wire encoding, borrowed
    /// without copying.
    pub fn as_raw_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Whether two frozen streams share one buffer: clone identity, the `O(1)`
    /// entry of [`canonical_eq`]'s ladder.
    ///
    /// The guarantee a fast path may rest on is exactly *value equality*:
    /// marker padding makes the stored bytes injective on streams, so same
    /// pointer and same byte length name one memory region (or, for the empty
    /// stream, no region) spelling one stream — `ptr_eq` implies byte equality,
    /// never the converse: two independently built equal nonempty streams live
    /// in distinct buffers and fall through to the byte compare. Clone
    /// provenance is the *production* source of sharing, not the predicate's
    /// meaning: every zero-byte allocation carries the same dangling pointer,
    /// so two independently frozen **empty** streams also read `ptr_eq` true. A
    /// rung may therefore derive from `ptr_eq` only what equality gives it — a
    /// predicate that is not constant on the diagonal (`is_disjoint`, vacuously
    /// true for the empty share against itself) admits no rung here at all (the
    /// committed `Leaf(false)`-pair seed in the party differential suite pins
    /// the case).
    pub(crate) fn ptr_eq(&self, other: &Bits) -> bool {
        self.bytes.len() == other.bytes.len() && self.bytes.as_ptr() == other.bytes.as_ptr()
    }
}

/// Read the frozen stream as live bits — the padding stays behind the view:
/// every cursor and walk consumes [`Bits`] through this slice, exactly as it
/// consumes a [`BitsMut`].
///
/// # Panics
///
/// Panics inside `bitvec` when the buffer's bit count exceeds the borrowed
/// view's encoding limit, `usize::MAX >> 3` bits — on a 32-bit target, any
/// stored stream past 64 MiB. Such streams exist there: the byte decode
/// doors admit valid encodings up to the `usize` bit-position bound (512
/// MiB), and every byte-level operation on them (equality, hashing, the
/// canonical bytes, re-encoding, the length) is exact — but the walk surface
/// reads through this one borrowed view, whose fat-pointer length field is
/// three bits narrower than `usize`, so a walk over such a value fails loudly
/// here rather than reading a mis-lengthed view. Widening the walk surface
/// past the borrowed-view encoding requires a crate-owned view type in place
/// of the `bitvec` borrow. On 64-bit targets the limit is 2^58 bytes,
/// unreachable by allocation.
impl Deref for Bits {
    type Target = BitsSlice;
    fn deref(&self) -> &BitsSlice {
        &self.bytes.view_bits::<Msb0>()[..self.len()]
    }
}

/// Equality of frozen streams is [`canonical_eq`]: the clone-identity fast
/// path, then the one-`memcmp` byte compare that marker-padded storage makes
/// exactly bit equality.
impl PartialEq for Bits {
    fn eq(&self, other: &Self) -> bool {
        canonical_eq(self, other)
    }
}

impl Eq for Bits {}

/// Borrow bytes as an MSB-first bit stream without first copying them into a
/// [`BitsMut`]: the meter surface's and the test suites' view — production
/// decode walks raw bytes instead, since this view's encoding caps below the
/// buffer sizes the decode doors admit on 32-bit targets.
#[cfg(any(test, feature = "meter"))]
pub(crate) fn bytes_as_bits(bytes: &[u8]) -> &BitsSlice {
    bytes.view_bits::<Msb0>()
}

/// Whether two bit-slice views read one memory region: [`Bits::ptr_eq`]'s clone
/// identity, observable at the slice level the walk kernels consume.
///
/// Two views of one frozen buffer — a [`Bits`] deref'd twice, through any
/// number of `O(1)` clones — carry the same bit pointer and length, so view
/// identity implies bit-for-bit equality and an identity-law fast path may
/// answer without a walk. Never the converse: equal streams in distinct buffers
/// fall through to the walk that reads them.
pub(crate) fn slice_ptr_eq(a: &BitsSlice, b: &BitsSlice) -> bool {
    a.as_bitptr() == b.as_bitptr() && a.len() == b.len()
}

/// Seal a built stream's canonical padding: one `1` marker bit, then zeroed
/// dead bits to the byte boundary.
///
/// Sealing makes the packed bytes ([`BitVec::as_raw_slice`]) the canonical wire
/// spelling — injective, byte-equal if and only if the bit content is equal.
///
/// The zeroing is load-bearing on its own: the tree builders write into a
/// reused buffer, and a collapsing node (the party `sum`/`diff` ops, via
/// `IdBuilder::close_node`) `truncate`s it, shrinking the live length while
/// leaving the bits it shed in the final partial byte, where `as_raw_slice`
/// would expose them. The marker then pins the live length inside the sealed
/// byte. The empty stream seals to itself: no marker, no bytes.
/// [`Bits::freeze`] applies this at the storage seam; the standalone form seals
/// buffers that stay build-side — all of them meter/test instruments producing
/// decodable bytes.
pub(crate) fn seal_padding(bits: &mut BitsMut) {
    if !bits.is_empty() {
        bits.push(true);
    }
    bits.set_uninitialized(false);
}

/// Byte-level equality of two canonical stored streams: equal raw
/// bytes, entered through the clone-identity fast path.
///
/// The ladder: [`Bits::ptr_eq`] first — clones of one freeze share every byte
/// by construction, `O(1)` — then the byte compare. The byte rung rests on the
/// canonical-padding invariant ([`Bits::freeze`] seals every storage seam),
/// under which the bytes are injective on streams — the marker pins each
/// stream's live length inside its final byte, so no two streams of any lengths
/// share a byte spelling — and raw-byte equality alone is exactly bit equality,
/// decided by one `memcmp` instead of a bit-domain-chunked compare.
pub(crate) fn canonical_eq(a: &Bits, b: &Bits) -> bool {
    debug_assert!(
        padding_is_canonical(a) && padding_is_canonical(b),
        "canonical_eq compares raw bytes: both operands must be marker-padded",
    );
    a.ptr_eq(b) || a.as_raw_slice() == b.as_raw_slice()
}

/// Byte-level hash of a canonical stored stream: the raw bytes alone.
///
/// [`canonical_eq`]'s hash counterpart — marker padding makes the raw bytes a
/// complete identity (injective on streams), so the hasher is fed exactly what
/// equality compares and equal values hash equally by construction. An order of
/// magnitude cheaper than hashing bit by bit: the hasher consumes the byte
/// slice in one call rather than one update per bit.
pub(crate) fn canonical_hash<H: Hasher>(bits: &Bits, state: &mut H) {
    use core::hash::Hash;
    debug_assert!(
        padding_is_canonical(bits),
        "canonical_hash reads raw bytes: the operand must be marker-padded",
    );
    bits.as_raw_slice().hash(state);
}

/// Whether a stored stream's padding is canonical: the `O(1)` check behind the
/// `as_bytes` debug asserts.
///
/// Canonical padding puts the marker — the buffer's final set bit — in the
/// final byte, so the check is one byte test: a nonempty buffer's final byte
/// must be nonzero (an all-zero final byte has no marker to recover the live
/// length from), and a lone marker with no live bits is not the empty stream's
/// spelling (the empty stream is the empty buffer).
pub(crate) fn padding_is_canonical(bits: &Bits) -> bool {
    match bits.as_raw_slice() {
        [] => true,
        [0x80] => false,
        [.., 0] => false,
        _ => true,
    }
}

/// The direct byte view of a bit slice that starts on a byte boundary of its
/// backing store: the whole body bytes plus the masked partial tail byte, if
/// any.
///
/// `None` for the one shape with no direct byte view — a slice whose
/// backing-store offset puts live bits behind a partial head element. Every
/// stored stream starts on a byte boundary (offsets travel as bit positions,
/// never as re-sliced heads), so the `None` arm is a caller policy decision,
/// not a reachable production state: the gamma window loader degrades to its
/// bit-addressed fallback, the dsi cursor treats it as a violated precondition.
/// The destructuring lives here once so its callers cannot drift on the
/// byte-alignment invariant while keeping their deliberately different failure
/// policies.
pub(crate) fn byte_view(bits: &BitsSlice) -> Option<(&[u8], Option<u8>)> {
    match bits.domain() {
        Domain::Region {
            head: None,
            body,
            tail,
        } => Some((body, tail.map(|elem| elem.load_value()))),
        Domain::Enclave(elem) if elem.head().into_inner() == 0 => {
            Some((&[], Some(elem.load_value())))
        }
        Domain::Region { head: Some(_), .. } | Domain::Enclave(_) => None,
    }
}

/// Require that the bits from `pos` to the buffer's end are exactly the
/// canonical padding: one `1` marker bit, then zeros to the byte boundary.
/// The byte decode doors' padding judge, over the raw stream bytes.
///
/// Positions are `u64` because a door walks the whole byte buffer as bits and
/// `8·bytes.len()` itself can exceed a 32-bit `usize`; the raw-byte form is
/// also what lets the doors admit buffers past the borrowed bit view's
/// encoding cap (see [`Bits`]'s deref). A canonical encoding pads with a
/// single marker and at most 7 zeros, all inside the final byte, so an intact
/// remainder here is 1 to 8 bits — a `1`, then zeros — decided by one mask
/// compare on the final byte. (A stream whose live bits end flush against a
/// byte boundary carries its marker in a whole final `1000_0000` byte.) The
/// rejections split by genre:
///
/// - An empty remainder is [`Decode::Truncated`]: the input ends where the
///   padding should begin — a flush stream cut before its whole marker byte —
///   so required data is missing, exactly what a byte-starved reader reports
///   at the same boundary.
/// - Everything else is [`Decode::TrailingBits`]: a leading `0`, a second set
///   bit, or a remainder of 9+ bits (a spurious trailing byte, even a
///   well-formed marker followed by an all-zero byte).
///
/// The marker plus the length bound are what make `decode` injective on
/// bytes: every stream has exactly one padded spelling, and no byte string
/// spells two streams.
///
/// # Panics
///
/// `pos` must be at or before the buffer's end in bits (it is a walk's end
/// position over this very buffer).
pub(crate) fn require_marker_padding_bytes(bytes: &[u8], pos: u64) -> Result<(), Decode> {
    let total = bytes.len() as u64 * 8;
    assert!(
        pos <= total,
        "padding checked at a position inside the buffer"
    );
    let remainder = total - pos;
    match remainder {
        0 => Err(Decode::Truncated),
        1..=8 => {
            // The remainder lives entirely in the final byte: its low
            // `remainder` bits must be a `1` followed by zeros.
            let last = bytes[bytes.len() - 1];
            let mask = if remainder == 8 {
                0xFF
            } else {
                (1u8 << remainder) - 1
            };
            if last & mask == 1 << (remainder - 1) {
                Ok(())
            } else {
                Err(Decode::TrailingBits)
            }
        }
        _ => Err(Decode::TrailingBits),
    }
}
