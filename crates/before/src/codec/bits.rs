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
//!   (three word compares), so every site where equality settles the
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
//! Two instruments hold the ladder honest: the `identity_fast_paths`
//! pins in `tests/meter.rs` assert exactly zero walk work on every
//! adopted rung beside walking legs on the same values (a lost rung or
//! a dead meter both read red), and the fuzzfit fuel bands route
//! identity-outcome samples out of their fits so the walked laws stay
//! unimodal (`Step::identity` in the harness carries that argument).

// The storage forms' docs name crate-private machinery by intra-doc link
// (`ptr_eq`, `canonical_eq`) so a rename cannot rot the prose (the internal doc
// build resolves every link); on the public build those links render as plain
// code spans — the items are private — which this allow accepts.
#![allow(rustdoc::private_intra_doc_links)]

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
/// preorder bit stream together with its exact live length, over a refcounted
/// byte buffer.
///
/// The raw byte slice ([`as_raw_slice`](Self::as_raw_slice)) *is* the wire
/// encoding — exactly `ceil(len/8)` bytes, the final partial byte's dead bits
/// zeroed at the freeze seam — and the live length is a cached parse product
/// the wire legitimately omits, because the streams are self-delimiting at the
/// bit level.
///
/// The backing store is [`Bytes`], so [`Clone`] is a refcount bump: two clones
/// share one buffer, cost `O(1)`, and that shared identity is observable
/// through [`ptr_eq`](Self::ptr_eq) — the fast path the identity-law shortcuts
/// (`x ∨ x`, `cmp(x, x)`, `distance(x, x)`) dispatch on. Reading is still
/// `bitvec`'s: the struct [derefs](core::ops::Deref) to [`BitsSlice`], so every
/// cursor and walk consumes the frozen form exactly as it consumes a
/// [`BitsMut`].
#[derive(Clone)]
pub struct Bits {
    /// The canonical packed bytes: exactly the live bits' bytes, dead
    /// bits in the final partial byte zeroed.
    bytes: Bytes,
    /// The live bit length: `bytes` holds exactly `bit_len.div_ceil(8)`
    /// bytes.
    bit_len: usize,
}

impl Bits {
    /// The frozen empty stream: no bits, no bytes, no allocation.
    pub(crate) fn empty() -> Self {
        Bits {
            bytes: Bytes::new(),
            bit_len: 0,
        }
    }

    /// Freeze a built stream into the at-rest form, canonicalizing its storage:
    /// the single gate between the mutable build-side world and the shared
    /// frozen one.
    ///
    /// Zeroes the dead bits past the live length (see the type docs for what
    /// byte-canonicity underpins: a collapsing `truncate` leaves stale bits in
    /// the final partial byte, and `as_raw_slice` would expose them), then
    /// adopts the buffer without copying: [`BitVec::into_vec`] hands back the
    /// underlying allocation and `Bytes::from(vec)` wraps it in place.
    pub(crate) fn freeze(mut buf: BitsMut) -> Self {
        buf.set_uninitialized(false);
        let bit_len = buf.len();
        Bits {
            bytes: Bytes::from(buf.into_vec()),
            bit_len,
        }
    }

    /// Adopt already-canonical bytes as a frozen stream: the decode-side door,
    /// for buffers whose padding a validator has already proven zero.
    ///
    /// `bytes` must hold exactly `bit_len.div_ceil(8)` bytes with the dead bits
    /// zero — what `require_zero_padding` accepts. Debug builds assert both;
    /// release builds trust the validator.
    pub(crate) fn from_canonical(bytes: Bytes, bit_len: usize) -> Self {
        debug_assert_eq!(
            bytes.len(),
            bit_len.div_ceil(8),
            "from_canonical: the buffer must cover exactly the live bits' bytes",
        );
        let bits = Bits { bytes, bit_len };
        debug_assert!(
            dead_bits_are_zero(&bits),
            "from_canonical: dead bits past the live length must be zero",
        );
        bits
    }

    /// The live bit length of the stream.
    pub fn len(&self) -> usize {
        self.bit_len
    }

    /// Whether the stream holds no bits at all.
    ///
    /// This is emptiness of the *storage* (the anonymous id), not of the value
    /// a stream spells: the empty `Version` is a 2-bit stream.
    /// [`len`](Self::len)'s conventional partner; the walks ask the question of
    /// slices, so only the meter surface and the tests reach it.
    #[cfg(any(test, feature = "meter"))]
    pub fn is_empty(&self) -> bool {
        self.bit_len == 0
    }

    /// The canonical packed bytes: the wire encoding, borrowed without
    /// copying.
    pub fn as_raw_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Whether two frozen streams share one buffer: clone identity, the `O(1)`
    /// entry of [`canonical_eq`]'s ladder.
    ///
    /// The guarantee a fast path may rest on is exactly *value equality*: same
    /// pointer and same lengths name one memory region (or, for the empty
    /// stream, no region), so `ptr_eq` implies byte equality — never the
    /// converse: two independently built equal nonempty streams live in
    /// distinct buffers and fall through to the byte compare. Clone provenance
    /// is the *production* source of sharing, not the predicate's meaning:
    /// every zero-byte allocation carries the same dangling pointer, so two
    /// independently frozen **empty** streams also read `ptr_eq` true. A rung
    /// may therefore derive from `ptr_eq` only what equality gives it — a
    /// predicate that is not constant on the diagonal (`is_disjoint`, vacuously
    /// true for the empty share against itself) admits no rung here at all (the
    /// committed `Leaf(false)`-pair seed in the party differential suite pins
    /// the case).
    pub(crate) fn ptr_eq(&self, other: &Bits) -> bool {
        self.bit_len == other.bit_len
            && self.bytes.len() == other.bytes.len()
            && self.bytes.as_ptr() == other.bytes.as_ptr()
    }
}

/// Read the frozen stream as live bits: every cursor and walk consumes [`Bits`]
/// through this view, exactly as it consumes a [`BitsMut`].
impl Deref for Bits {
    type Target = BitsSlice;
    fn deref(&self) -> &BitsSlice {
        &self.bytes.view_bits::<Msb0>()[..self.bit_len]
    }
}

/// Equality of frozen streams is [`canonical_eq`]: the clone-identity fast
/// path, then the one-`memcmp` byte compare that canonical storage makes
/// exactly bit equality.
impl PartialEq for Bits {
    fn eq(&self, other: &Self) -> bool {
        canonical_eq(self, other)
    }
}

impl Eq for Bits {}

/// Borrow bytes as an MSB-first bit stream without first copying them into a
/// [`BitsMut`].
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

/// Zero the dead bits past the live length, making the packed bytes
/// ([`BitVec::as_raw_slice`]) canonical: byte-equal if and only if the bit
/// content is equal.
///
/// The tree builders write into a reused buffer, and a collapsing node (the
/// party `sum`/`diff` ops, via `IdBuilder::close_node`) `truncate`s it,
/// shrinking the live length while leaving the bits it shed in the final
/// partial byte. `as_raw_slice` exposes those stale bits, so two equal parties
/// built different ways would serialize to different bytes and a joined party
/// could fail to decode on the wire. [`Bits::freeze`] applies this at the
/// storage seam; this standalone form canonicalizes buffers that stay
/// build-side — all of them meter/test instruments, hence the gate.
#[cfg(any(test, feature = "meter"))]
pub(crate) fn zero_dead_bits(bits: &mut BitsMut) {
    bits.set_uninitialized(false);
}

/// Byte-level equality of two canonical stored streams: equal live lengths and
/// equal raw bytes, entered through the clone-identity fast path.
///
/// The ladder: [`Bits::ptr_eq`] first — clones of one freeze share every byte
/// by construction, `O(1)` — then the byte compare. The byte rung rests on the
/// canonical-raw-slice invariant ([`Bits::freeze`] zeroes dead bits at every
/// storage seam), under which raw-byte equality plus live-length equality is
/// exactly bit equality, decided by one `memcmp` instead of a
/// bit-domain-chunked compare (measured 2–54x faster on equal pairs from 23
/// bits to 32 Kbits, and 5x on a hash-map workload, in the 2026-07
/// storage-migration probe). The length check is load-bearing: two streams of
/// different live length can share raw bytes (`01` vs `010` are both the byte
/// `0x40`).
pub(crate) fn canonical_eq(a: &Bits, b: &Bits) -> bool {
    debug_assert!(
        dead_bits_are_zero(a) && dead_bits_are_zero(b),
        "canonical_eq compares raw bytes: both operands' dead bits must be zero",
    );
    a.ptr_eq(b) || (a.len() == b.len() && a.as_raw_slice() == b.as_raw_slice())
}

/// Byte-level hash of a canonical stored stream: the raw bytes, then the live
/// length.
///
/// [`canonical_eq`]'s hash counterpart — it feeds the hasher exactly the pair
/// that equality compares, so equal values hash equally by construction. Rests
/// on the same canonical-raw-slice invariant, and is an order of magnitude
/// cheaper than hashing bit by bit (same probe as [`canonical_eq`]'s).
pub(crate) fn canonical_hash<H: core::hash::Hasher>(bits: &Bits, state: &mut H) {
    use core::hash::Hash;
    debug_assert!(
        dead_bits_are_zero(bits),
        "canonical_hash reads raw bytes: dead bits must be zero",
    );
    bits.as_raw_slice().hash(state);
    bits.len().hash(state);
}

/// Whether a stored stream's dead bits are zero: the canonical-storage check
/// behind the `as_bytes` debug asserts.
///
/// Only the final partial byte of the raw slice can hold dead bits (the slice
/// covers exactly the live bits' bytes), so this is one mask test — `O(1)`,
/// cheap enough to assert on every raw-byte read.
pub(crate) fn dead_bits_are_zero(bits: &Bits) -> bool {
    let live_in_last = bits.len() % 8;
    live_in_last == 0
        || bits
            .as_raw_slice()
            .last()
            .is_none_or(|last| last & (0xFF >> live_in_last) == 0)
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

/// Require that the bits from `pos` onward are exactly the canonical padding: a
/// run of zeros shorter than a byte.
///
/// A canonical encoding zero-pads only the final partial byte (the stored
/// form's raw slice covers exactly the live bits' bytes, dead bits zeroed), so
/// it has at most 7 trailing zero bits; both a nonzero padding bit AND a whole
/// spurious zero byte (`>= 8` trailing bits, even if all zero) are
/// non-canonical. Bounding the length is what makes `decode` injective on bytes
/// — without it, `decode([.., 0x00])` would accept the same value under
/// infinitely many byte strings, re-encoding to a shorter stream than its own
/// input.
pub(crate) fn require_zero_padding(bits: &BitsSlice, pos: usize) -> Result<(), Decode> {
    if bits.len() - pos >= 8 || bits[pos..].any() {
        Err(Decode::TrailingBits)
    } else {
        Ok(())
    }
}
