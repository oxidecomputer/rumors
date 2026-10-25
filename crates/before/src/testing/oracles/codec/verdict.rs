//! Reads any byte string as the wire formats lay it out, and gives the
//! verdict a strict decoder owes it.
//!
//! Each function here reads an input as its format lays it out and returns a
//! [`Verdict`]: the canonical value the input encodes, or every class of
//! defect it carries. `Decode`'s rustdoc says that an input may have more
//! than one defect and that, unless a decoder documents a precedence rule,
//! callers should handle any applicable variant. So a rejection here lists
//! every applicable class, never only the one a sequential parse meets first.
//! The one documented precedence, a span's structural errors before its
//! endpoint ordering, narrows the span's list ([`span`]).
//!
//! The layouts are the encoder's, from the parent module. The class of each
//! defect follows `Decode`'s variant docs and each decoder's `# Errors`
//! section:
//!
//! | Defect | Class |
//! |---|---|
//! | the input ends inside a tree, an integer, or a rank, or before a field's padding byte | [`Class::Truncated`] |
//! | a cleared marker, a set padding bit, an all-zero final rank group, or bytes after the value | [`Class::TrailingBits`] |
//! | a collapsible sibling pair, a negative height, a rank header that declares a width that does not fit `u64`, crossed or concurrent span endpoints, or a ranked key whose rank is not its version's | [`Class::NotCanonical`] |
//!
//! The reading is lenient, so that a defect early in the input never hides a
//! later one:
//!
//! - A tree that breaks canonical form still has a shape, so reading goes on
//!   past a collapsible pair or a negative height, to the tree's end and its
//!   padding.
//! - A marker-padded field ends at the byte boundary after its tree's last
//!   live bit and the marker's place, whether or not that padding is well
//!   formed, so a composite's second field starts there. A rank stream ends
//!   at the byte boundary after its closing bit.
//! - Running out of input ends the reading: no later field exists.
//!
//! Like the encoder, the reading shares no code with production and keeps
//! its trees flat, so no input depth can exhaust the stack.

use std::collections::BTreeSet;
use std::iter;

use num_bigint::{BigInt, BigUint, Sign};

use super::{pack, Party, Version};
use crate::testing::oracles::flat;

/// The fewest ones that make a rank's header declare a width that does not
/// fit `u64`.
///
/// For a rank's integral part `I`, the header gives the width `w` of
/// `m = I + 1`: `rho` ones, a zero, then the `rho` low bits of `w`. The
/// *mantissa*, `m`'s `w - 1` low bits, follows it. A header of 64 ones or
/// more declares `w >= 2^64`.
///
/// `Rank::decode` calls a header `NotCanonical` when it declares "a mantissa
/// width outside the format's `u64` range", which is ambiguous at
/// `w = 2^64` exactly: `w` does not fit `u64`, while the mantissa's `w - 1`
/// bits do. Under either reading the mantissa runs past the end of any
/// input, so the specification's set for every such header holds both
/// `NotCanonical` and `Truncated`, and a decoder following either reading
/// passes.
const RANK_HEADER_ONES: u64 = u64::BITS as u64;

/// A class of decoding defect: one for each `Decode` variant that describes
/// the input rather than the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Class {
    /// The input ends before the value does.
    Truncated,
    /// A complete value's padding is malformed, or bytes follow the value.
    TrailingBits,
    /// The input is readable but is not the canonical spelling of a value.
    NotCanonical,
}

impl Class {
    /// Whether the class describes the encoding's structure rather than its
    /// canonical form: the classes a span reports before its endpoint order.
    pub(crate) fn is_structural(self) -> bool {
        matches!(self, Class::Truncated | Class::TrailingBits)
    }
}

/// Where a value's encoding ends in the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    /// The encoding is the whole input, so bytes after it are a defect: the
    /// reader decodes, hexadecimal text, and the serde impls.
    Whole,
    /// The encoding opens a stream whose later bytes belong to the next
    /// field: borsh.
    Stream,
}

/// What a strict decoder owes one input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verdict<T> {
    /// The input is, or under [`Ending::Stream`] opens with, the canonical
    /// encoding of `value`, which spans its first `len` bytes.
    Accept {
        /// The value the encoding spells.
        value: T,
        /// The encoding's length in bytes.
        len: usize,
    },
    /// The input encodes no value. A decoder may report any of these
    /// classes, and the set is never empty.
    Reject(BTreeSet<Class>),
}

impl<T> Verdict<T> {
    /// Whether the verdict accepts.
    pub(crate) fn accepts(&self) -> bool {
        matches!(self, Verdict::Accept { .. })
    }
}

// ───────────────────────────── bits ─────────────────────────────

/// A reading position in an input, in bits from its first byte's high bit.
///
/// Positions are `u64`, so a bit count never depends on the width of
/// `usize`.
struct Cursor<'a> {
    /// The whole input.
    bytes: &'a [u8],
    /// The next bit to read.
    at: u64,
}

impl<'a> Cursor<'a> {
    /// A cursor at the start of byte `byte`.
    fn at_byte(bytes: &'a [u8], byte: usize) -> Self {
        Cursor {
            bytes,
            at: byte as u64 * 8,
        }
    }

    /// The input's length in bits.
    fn len(&self) -> u64 {
        self.bytes.len() as u64 * 8
    }

    /// The bit at `at`, which lies inside the input.
    fn bit_at(&self, at: u64) -> bool {
        let byte = usize::try_from(at / 8).expect("a bit inside the input indexes its byte");
        self.bytes[byte] & (0x80 >> (at % 8)) != 0
    }

    /// Reads one bit, or `None` at the end of the input.
    fn bit(&mut self) -> Option<bool> {
        (self.at < self.len()).then(|| {
            self.at += 1;
            self.bit_at(self.at - 1)
        })
    }

    /// Reads `count` bits, or `None` if the input ends first.
    fn bits(&mut self, count: u64) -> Option<Vec<bool>> {
        (0..count).map(|_| self.bit()).collect()
    }

    /// Reads `count` bits as an unsigned integer, most significant first, or
    /// `None` if the input ends first.
    fn uint(&mut self, count: u64) -> Option<BigUint> {
        if count > self.len() - self.at {
            return None;
        }
        Some(uint_from_bits(&self.bits(count)?))
    }

    /// Reads ones up to the first zero or the end of the input, and returns
    /// how many it read, leaving the zero unread.
    fn ones(&mut self) -> u64 {
        let start = self.at;
        while self.at < self.len() && self.bit_at(self.at) {
            self.at += 1;
        }
        self.at - start
    }

    /// Reads an Elias gamma code and returns `n` for the code of `n + 1`, or
    /// `None` if the input ends inside it.
    fn gamma(&mut self) -> Option<BigUint> {
        let mut zeros = 0u64;
        while !self.bit()? {
            zeros += 1;
        }
        let low = self.uint(zeros)?;
        Some(((BigUint::from(1u8) << zeros) | low) - 1u32)
    }
}

/// The unsigned integer whose binary digits are `bits`, most significant
/// first.
fn uint_from_bits(bits: &[bool]) -> BigUint {
    let lead = (8 - bits.len() % 8) % 8;
    let aligned: Vec<bool> = iter::repeat_n(false, lead)
        .chain(bits.iter().copied())
        .collect();
    BigUint::from_bytes_be(&pack(&aligned))
}

/// The height change a zigzag code spells: `c / 2` for an even code, and
/// `-(c + 1) / 2` for an odd one.
fn unzigzag(code: &BigUint) -> BigInt {
    let magnitude = BigInt::from((code + u32::from(code.bit(0))) >> 1u32);
    if code.bit(0) {
        -magnitude
    } else {
        magnitude
    }
}

// ───────────────────────────── trees ─────────────────────────────

/// A place in a tree being read: its depth, and whether it is its parent's
/// right child.
#[derive(Clone, Copy)]
struct Slot {
    /// The depth of the node that fills the slot.
    depth: u64,
    /// Whether the slot is a right child.
    right: bool,
}

impl Slot {
    /// The root's slot.
    const ROOT: Slot = Slot {
        depth: 0,
        right: false,
    };

    /// A branch's two child slots, right first, so a stack fills the left
    /// one first.
    fn children(self) -> [Slot; 2] {
        let depth = self.depth + 1;
        [
            Slot { depth, right: true },
            Slot {
                depth,
                right: false,
            },
        ]
    }
}

/// One tree read leniently: its leaves so far, whether they break canonical
/// form, and where the tree ends.
struct TreeRead<T> {
    /// The leaves read, left to right, each with its depth.
    leaves: Vec<(u64, T)>,
    /// Whether the leaves read hold a collapsible sibling pair or a negative
    /// height.
    non_canonical: bool,
    /// The position after the tree's last live bit, or `None` if the input
    /// ends inside the tree.
    end: Option<u64>,
}

impl<T: PartialEq> TreeRead<T> {
    /// A tree with nothing read.
    fn new() -> Self {
        TreeRead {
            leaves: Vec::new(),
            non_canonical: false,
            end: None,
        }
    }

    /// Places a leaf in `slot`, noting a collapsible pair: a right child
    /// whose left sibling is a leaf of the same value.
    ///
    /// The left sibling's subtree ends with the previous leaf, which lies at
    /// the slot's own depth exactly when that sibling is a single leaf.
    fn place(&mut self, slot: Slot, value: T) {
        let collapsible = slot.right
            && self
                .leaves
                .last()
                .is_some_and(|(depth, last)| *depth == slot.depth && *last == value);
        self.non_canonical |= collapsible;
        self.leaves.push((slot.depth, value));
    }
}

/// Reads one version tree: preorder, a `0` for each branch and a `1` for
/// each leaf followed by its Elias gamma code.
///
/// The first leaf's code is its height plus one; each later leaf's is its
/// zigzag-coded change from the previous height, plus one. A running height
/// below zero breaks canonical form.
fn read_version(cursor: &mut Cursor) -> TreeRead<BigInt> {
    let mut tree = TreeRead::new();
    let mut slots = vec![Slot::ROOT];
    let mut previous: Option<BigInt> = None;
    while let Some(slot) = slots.pop() {
        let Some(leaf) = cursor.bit() else {
            return tree;
        };
        if !leaf {
            slots.extend(slot.children());
            continue;
        }
        let Some(code) = cursor.gamma() else {
            return tree;
        };
        let height = match &previous {
            None => BigInt::from(code),
            Some(previous) => previous + unzigzag(&code),
        };
        tree.non_canonical |= height.sign() == Sign::Minus;
        previous = Some(height.clone());
        tree.place(slot, height);
    }
    tree.end = Some(cursor.at);
    tree
}

/// Reads one party tree: preorder, two bits per node present in the stream.
///
/// `00` is an owned leaf. Any other pair is a branch's presence bits, and an
/// absent child is an unowned leaf that the stream spells by nothing.
fn read_party(cursor: &mut Cursor) -> TreeRead<bool> {
    let mut tree = TreeRead::new();
    let mut slots = vec![(Slot::ROOT, true)];
    while let Some((slot, present)) = slots.pop() {
        if !present {
            tree.place(slot, false);
            continue;
        }
        let (Some(left), Some(right)) = (cursor.bit(), cursor.bit()) else {
            return tree;
        };
        if !left && !right {
            tree.place(slot, true);
            continue;
        }
        let [right_slot, left_slot] = slot.children();
        slots.push((right_slot, right));
        slots.push((left_slot, left));
    }
    tree.end = Some(cursor.at);
    tree
}

/// A marker-padded field read from the input.
struct Sealed<T> {
    /// The tree's leaves, left to right, each with its depth.
    leaves: Vec<(u64, T)>,
    /// The byte after the field's padding, or `None` if the input ends
    /// before the padding's byte.
    next: Option<usize>,
}

/// Reads a marker-padded field that starts at byte `start`, adding its
/// defects to `defects`, or returns `None` if the input ends inside its
/// tree.
///
/// The padding is a `1` marker after the tree's last live bit, then zeros to
/// the byte boundary. A cleared marker or a set padding bit is
/// `TrailingBits`; an input that ends before the padding's byte is
/// `Truncated`, as `Decode::Truncated` documents.
fn sealed<T: PartialEq>(
    bytes: &[u8],
    start: usize,
    read: fn(&mut Cursor) -> TreeRead<T>,
    defects: &mut BTreeSet<Class>,
) -> Option<Sealed<T>> {
    let mut cursor = Cursor::at_byte(bytes, start);
    let tree = read(&mut cursor);
    if tree.non_canonical {
        defects.insert(Class::NotCanonical);
    }
    let Some(end) = tree.end else {
        defects.insert(Class::Truncated);
        return None;
    };
    let next = (end + 1).div_ceil(8);
    if next * 8 > cursor.len() {
        defects.insert(Class::Truncated);
        return Some(Sealed {
            leaves: tree.leaves,
            next: None,
        });
    }
    let marked = cursor.bit_at(end);
    let clean = (end + 1..next * 8).all(|at| !cursor.bit_at(at));
    if !(marked && clean) {
        defects.insert(Class::TrailingBits);
    }
    let next = usize::try_from(next).expect("a byte inside the input has a usize offset");
    Some(Sealed {
        leaves: tree.leaves,
        next: Some(next),
    })
}

// ───────────────────────────── ranks ─────────────────────────────

/// Reads a rank stream from the input's start, as the `rank` module doc
/// lays it out, adding its defects to `defects`.
///
/// Returns the normalized rank `(num, exp)` and the byte after the stream,
/// or `None` if the input ends inside the stream. A header of
/// [`RANK_HEADER_ONES`] ones or more declares a width that does not fit
/// `u64`, which is non-canonical; such a header still says where the stream
/// goes on, so reading continues, and an input that ends inside the declared
/// mantissa is truncated as well.
fn rank_field(bytes: &[u8], defects: &mut BTreeSet<Class>) -> Option<((BigUint, u64), usize)> {
    let mut cursor = Cursor::at_byte(bytes, 0);
    let rank = read_rank(&mut cursor, defects);
    let Some(rank) = rank else {
        defects.insert(Class::Truncated);
        return None;
    };
    let end = cursor.at;
    let next = end.div_ceil(8);
    if (end..next * 8).any(|at| cursor.bit_at(at)) {
        defects.insert(Class::TrailingBits);
    }
    let next = usize::try_from(next).expect("a byte inside the input has a usize offset");
    Some((rank, next))
}

/// Reads a rank stream through its closing bit, or returns `None` if the
/// input ends first; see [`rank_field`].
///
/// The digits lose their trailing zeros, so the rank comes out normalized.
fn read_rank(cursor: &mut Cursor, defects: &mut BTreeSet<Class>) -> Option<(BigUint, u64)> {
    let ones = cursor.ones();
    if ones >= RANK_HEADER_ONES {
        defects.insert(Class::NotCanonical);
    }
    cursor.bit()?;
    let width = (BigUint::from(1u8) << ones) | cursor.uint(ones)?;
    // A mantissa whose bit count does not fit `u64` is longer than any
    // input, so the input ends inside it.
    let mantissa_bits = u64::try_from(width - 1u32).ok()?;
    // Reading the mantissa before shifting bounds the shift by the input's
    // length: a header may declare far more bits than the input holds.
    let mantissa = cursor.uint(mantissa_bits)?;
    let integral = ((BigUint::from(1u8) << mantissa_bits) | mantissa) - 1u32;
    let mut digits = Vec::new();
    let mut final_group_zero = false;
    while cursor.bit()? {
        let group = cursor.bits(8)?;
        final_group_zero = !group.contains(&true);
        digits.extend(group);
    }
    if final_group_zero {
        defects.insert(Class::TrailingBits);
    }
    while digits.last() == Some(&false) {
        digits.pop();
    }
    let exp = digits.len() as u64;
    Some(((integral << exp) | uint_from_bits(&digits), exp))
}

// ───────────────────────────── verdicts ─────────────────────────────

/// Adds the defect of bytes after a value's last field, which ends before
/// byte `next`: trailing under [`Ending::Whole`], and the next field's under
/// [`Ending::Stream`].
fn check_trailing(defects: &mut BTreeSet<Class>, ending: Ending, bytes: &[u8], next: usize) {
    if ending == Ending::Whole && next < bytes.len() {
        defects.insert(Class::TrailingBits);
    }
}

/// Accepts `value`, spanning `len` bytes, when reading found no defect.
fn conclude<T>(defects: BTreeSet<Class>, value: impl FnOnce() -> T, len: usize) -> Verdict<T> {
    if defects.is_empty() {
        Verdict::Accept {
            value: value(),
            len,
        }
    } else {
        Verdict::Reject(defects)
    }
}

/// The verdict on a version encoding.
pub(crate) fn version(bytes: &[u8], ending: Ending) -> Verdict<Version> {
    let mut defects = BTreeSet::new();
    let Some(Sealed {
        leaves,
        next: Some(next),
    }) = sealed(bytes, 0, read_version, &mut defects)
    else {
        return Verdict::Reject(defects);
    };
    check_trailing(&mut defects, ending, bytes, next);
    conclude(defects, || Version::from_leaves(leaves), next)
}

/// The verdict on a party encoding.
pub(crate) fn party(bytes: &[u8], ending: Ending) -> Verdict<Party> {
    let mut defects = BTreeSet::new();
    let Some(Sealed {
        leaves,
        next: Some(next),
    }) = sealed(bytes, 0, read_party, &mut defects)
    else {
        return Verdict::Reject(defects);
    };
    check_trailing(&mut defects, ending, bytes, next);
    conclude(defects, || Party::from_leaves(leaves), next)
}

/// The verdict on a rank encoding, whose value is the normalized
/// `(num, exp)`.
pub(crate) fn rank(bytes: &[u8], ending: Ending) -> Verdict<(BigUint, u64)> {
    let mut defects = BTreeSet::new();
    let Some((rank, next)) = rank_field(bytes, &mut defects) else {
        return Verdict::Reject(defects);
    };
    check_trailing(&mut defects, ending, bytes, next);
    conclude(defects, || rank, next)
}

/// The verdict on a clock encoding: a party field, then a version field.
pub(crate) fn clock(bytes: &[u8], ending: Ending) -> Verdict<(Party, Version)> {
    let mut defects = BTreeSet::new();
    let Some(Sealed {
        leaves: party,
        next: Some(start),
    }) = sealed(bytes, 0, read_party, &mut defects)
    else {
        return Verdict::Reject(defects);
    };
    let Some(Sealed {
        leaves: version,
        next: Some(next),
    }) = sealed(bytes, start, read_version, &mut defects)
    else {
        return Verdict::Reject(defects);
    };
    check_trailing(&mut defects, ending, bytes, next);
    conclude(
        defects,
        || (Party::from_leaves(party), Version::from_leaves(version)),
        next,
    )
}

/// The verdict on a span encoding: a lower endpoint, then an upper endpoint
/// at or above it.
///
/// `Span::decode` documents that structural errors take precedence over
/// endpoint ordering, so endpoints out of order add
/// [`NotCanonical`](Class::NotCanonical) only when no structural class
/// applies. A non-canonical endpoint adds it in any case.
pub(crate) fn span(bytes: &[u8], ending: Ending) -> Verdict<(Version, Version)> {
    let mut defects = BTreeSet::new();
    let Some(Sealed {
        leaves: lo,
        next: Some(start),
    }) = sealed(bytes, 0, read_version, &mut defects)
    else {
        return Verdict::Reject(defects);
    };
    let Some(Sealed {
        leaves: hi,
        next: Some(next),
    }) = sealed(bytes, start, read_version, &mut defects)
    else {
        return Verdict::Reject(defects);
    };
    check_trailing(&mut defects, ending, bytes, next);
    let (lo, hi) = (Version::from_leaves(lo), Version::from_leaves(hi));
    // Out-of-order endpoints are judged only when no class applies yet: a
    // structural class takes precedence over the order, and when the only
    // class is `NotCanonical` the order adds nothing. With no class, both
    // endpoints are canonical, so the flat model can order them.
    if defects.is_empty()
        && !flat::Version::from_spelling(&lo).le(&flat::Version::from_spelling(&hi))
    {
        defects.insert(Class::NotCanonical);
    }
    conclude(defects, || (lo, hi), next)
}

/// The length of a span encoding's lower endpoint, through its padding, when
/// the input holds its whole tree and its padding's byte.
///
/// Splitting there gives the endpoints of the span's human-readable record,
/// whatever defects either one carries.
pub(crate) fn span_lower_len(bytes: &[u8]) -> Option<usize> {
    sealed(bytes, 0, read_version, &mut BTreeSet::new())?.next
}

/// The verdict on a ranked-key encoding: a rank stream, then a version field
/// whose own rank it is.
///
/// The rank comparison applies only where neither field is already
/// non-canonical: a negative height leaves the version with no rank.
pub(crate) fn ranked(bytes: &[u8], ending: Ending) -> Verdict<Version> {
    let mut defects = BTreeSet::new();
    let Some((rank, start)) = rank_field(bytes, &mut defects) else {
        return Verdict::Reject(defects);
    };
    let Some(Sealed { leaves, next }) = sealed(bytes, start, read_version, &mut defects) else {
        return Verdict::Reject(defects);
    };
    let version = Version::from_leaves(leaves);
    // Without `NotCanonical`, the version is canonical, so the flat model can
    // take its rank.
    if !defects.contains(&Class::NotCanonical)
        && flat::Version::from_spelling(&version).rank() != rank
    {
        defects.insert(Class::NotCanonical);
    }
    let Some(next) = next else {
        return Verdict::Reject(defects);
    };
    check_trailing(&mut defects, ending, bytes, next);
    conclude(defects, || version, next)
}
