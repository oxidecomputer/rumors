//! Deterministic operand-content walks: the quantities the liveness floors and
//! denominators are stated in, derived from packed operands entirely outside
//! any measurement.

use crate::codec::{self, Base};
use crate::{Clock, Party, Version};

use super::ceilings::MACHINE_WORD_MAGNITUDE_BITS;

/// The *nonzero* stored delta codes of a version's packed stream: every leaf
/// payload code after the first (the absolute root height) whose delta is
/// nonzero.
///
/// The delta-folding kernels — single-operand (validate, the query folds, the
/// tick walk, the text parse) and the pair walks alike — land each of these in
/// a running accumulator, so the count is the touch column's
/// deterministic-liveness floor (the pair walks take the max over their two
/// operands: a shared boundary lands both codes in one fold). A zero delta is
/// decoded but folds nothing — an accumulator add of zero is a no-op — so a
/// floor that counted every delta would demand touch work no conforming fold
/// does (a plateau-heavy stream legitimately reads near zero). Iterative over
/// the packed form, outside any measurement.
pub(super) fn stored_nonzero_deltas(v: &Version) -> u64 {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut first = true;
    let mut nonzero = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (payload, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        if !first && payload != Base::ZERO {
            nonzero += 1;
        }
        first = false;
    }
    nonzero
}

/// The mandatory limb count of a version's stored stream: one limb per 64 bits
/// of every payload code wider than [`MACHINE_WORD_MAGNITUDE_BITS`].
///
/// A walk over the stream must decode each stored code to fold it, and decoding
/// a wide code cannot touch fewer limbs than the code has; narrower codes may
/// legitimately live in machine words and count zero. Unlike
/// [`mandatory_limbs_version`], this counts the stream's own delta codes, never
/// the decoded tree's absolute values: it is the honest floor for operations
/// that read the stored form as-is. Iterative over the packed form, outside any
/// measurement.
pub(super) fn mandatory_limbs_stream(v: &Version) -> u64 {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut limbs = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let width = code.bits();
        if width > MACHINE_WORD_MAGNITUDE_BITS {
            limbs += width.div_ceil(64);
        }
    }
    limbs
}

/// A version's value content in bytes: the summed bit widths of its absolute
/// leaf heights (one bit minimum per leaf), rounded to bytes.
///
/// This is the content that delta coding lets ride behind asymptotically fewer
/// wire bits, and the scaling denominator of the flat-denominator shape's
/// exponent fits: the boundary comb at fixed tooth magnitude doubles its value
/// content (and every operation's honest per-tooth work) per level while its
/// packed bytes grow only by the unit delta codes over a fixed wide intercept.
/// Iterative over the packed form, outside any measurement.
pub(super) fn value_content_bytes(v: &Version) -> usize {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut last: Option<Base> = None;
    let mut content = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let value = match last {
            None => code,
            Some(prev) => {
                let odd = code.bit(0);
                let magnitude = if odd {
                    (code + 1u32) >> 1u32
                } else {
                    code >> 1u32
                };
                if odd {
                    prev - &magnitude
                } else {
                    prev + &magnitude
                }
            }
        };
        content += value.bits().max(1);
        last = Some(value);
    }
    (content.div_ceil(8)) as usize
}

/// The mandatory limb count of a version's stored magnitudes: one limb per 64
/// bits of every base wider than [`MACHINE_WORD_MAGNITUDE_BITS`].
///
/// Materializing or folding such a value cannot touch fewer limbs than the
/// value has, whatever the representation; narrower values may legitimately
/// live in machine words and count zero. This is the floor for the
/// value-materializing parse rows alone (`FromStr` converts every spelled
/// base); rows that read the stored form as-is floor at
/// [`mandatory_limbs_stream`], whose derivation explains the split. The walk
/// mirrors [`radix_units_version`]: iterative over the packed form, outside any
/// measurement.
pub(super) fn mandatory_limbs_version(v: &Version) -> u64 {
    let mut limbs = 0u64;
    for base in stored_bases(v) {
        let width = base.bits();
        if width > MACHINE_WORD_MAGNITUDE_BITS {
            limbs += width.div_ceil(64);
        }
    }
    limbs
}

/// The min-lifted stored bases of a version's canonical event tree, in
/// preorder: the values the paper notation renders and any base-per-node
/// representation must hold.
///
/// Reconstructed from the stored skyline stream in three linear passes
/// (absolute leaf heights, bottom-up subtree floors, per-node relative bases),
/// entirely outside any measurement.
pub(super) fn stored_bases(v: &Version) -> Vec<Base> {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    // Pass 1: topology flags and absolute leaf heights.
    let mut pos = 0usize;
    let mut topology: Vec<bool> = Vec::new();
    let mut heights: Vec<Base> = Vec::new();
    let mut pending = 1usize;
    while pending > 0 {
        pending -= 1;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        topology.push(internal);
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let value = match heights.last() {
            None => code,
            Some(prev) => {
                let odd = code.bit(0);
                let magnitude = if odd {
                    (code + 1u32) >> 1u32
                } else {
                    code >> 1u32
                };
                if odd {
                    prev.clone() - &magnitude
                } else {
                    prev + &magnitude
                }
            }
        };
        heights.push(value);
    }
    // Pass 2: per-node floors (minimum leaf height in the subtree), bottom-up
    // over the preorder topology.
    let nodes = topology.len();
    let mut floors: Vec<Base> = vec![Base::ZERO; nodes];
    let mut open: Vec<(usize, Option<Base>)> = Vec::new();
    let mut next_leaf = 0usize;
    for (index, &internal) in topology.iter().enumerate() {
        if internal {
            open.push((index, None));
            continue;
        }
        floors[index] = heights[next_leaf].clone();
        next_leaf += 1;
        let mut summary = floors[index].clone();
        loop {
            match open.pop() {
                None => break,
                Some((parent, None)) => {
                    open.push((parent, Some(summary)));
                    break;
                }
                Some((parent, Some(left))) => {
                    let floor = if left <= summary { left } else { summary };
                    floors[parent] = floor.clone();
                    summary = floor;
                }
            }
        }
    }
    // Pass 3: each node's stored base is its floor minus its parent's.
    let mut bases = Vec::with_capacity(nodes);
    let mut parent_floors: Vec<Base> = vec![Base::ZERO];
    for (index, &internal) in topology.iter().enumerate() {
        let parent = parent_floors
            .pop()
            .expect("preorder supplies one inherited floor per node");
        bases.push(floors[index].clone() - &parent);
        if internal {
            parent_floors.push(floors[index].clone());
            parent_floors.push(floors[index].clone());
        }
    }
    bases
}

/// `Σ digits × limbs` over the decimal values an event tree's text spells:
/// every node's stored base, exactly what `Display` renders and `FromStr`
/// parses.
///
/// `digits` is the value's exact decimal length; `limbs` its 64-bit limb count
/// (at least 1, so single-digit zeros still cost a unit). The walk is iterative
/// over the packed form; only the per-value `digits × limbs` products enter the
/// denominator, so the term prices schoolbook conversion work without assuming
/// any converter.
pub(super) fn radix_units_version(v: &Version) -> u64 {
    let mut units = 0u64;
    for base in stored_bases(v) {
        let digits = base.to_string().len() as u64;
        let limbs = base.bits().div_ceil(64).max(1);
        units += digits * limbs;
    }
    units
}

/// `Σ digits × limbs` over an id tree's text: one unit per rendered `0`/`1`
/// token (terminals and absent children), each a single digit of a single-limb
/// value.
pub(super) fn radix_units_party(p: &Party) -> u64 {
    let bits = p.as_bits();
    if bits.is_empty() {
        return 1; // the empty id renders one `0` token
    }
    let mut pos = 0usize;
    let mut pending = 1u64;
    let mut units = 0u64;
    while pending > 0 {
        pending -= 1;
        let left = bits[pos];
        let right = bits[pos + 1];
        pos += 2;
        if !left && !right {
            units += 1; // a terminal renders `1`
            continue;
        }
        for present in [left, right] {
            if present {
                pending += 1;
            } else {
                units += 1; // an absent child renders `0`
            }
        }
    }
    units
}

/// `Σ digits × limbs` over a clock's text: its party's and version's terms.
pub(super) fn radix_units_clock(c: &Clock) -> u64 {
    radix_units_party(c.party()) + radix_units_version(c.version())
}

/// The packed byte size of a version produced by a measured body.
pub(super) fn version_output_bytes(v: &Version) -> usize {
    v.encoded_bits().div_ceil(8)
}
