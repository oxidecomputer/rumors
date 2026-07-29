//! Exhaustive enumeration of the packed grammars at small sizes.
//!
//! The adequacy pins need ground truth the samplers cannot supply about
//! themselves: every member of an exact-size input space, listed by brute
//! force straight from the grammar rules (no counting tables anywhere in
//! this module, so a table bug cannot hide here). The tests compare these
//! listings against the counting tables, against the real decoders' accept
//! sets over all short byte strings, and against the samplers' draw
//! frequencies.
//!
//! Enumeration is exponential in the bit length by design; callers stay in
//! the low twenties of bits.

/// One enumerated version-grammar member.
pub struct VersionMember {
    /// The packed bit stream (unpadded).
    pub bits: Vec<bool>,
    /// Whether the member is a single bare leaf (the enclosing split
    /// enumeration needs it for the sibling rule).
    bare_leaf: bool,
    /// Leaf gamma values in preorder (= left-to-right) order.
    values: Vec<u64>,
}

impl VersionMember {
    /// Whether every running leaf height stays nonnegative — the one
    /// canonical-form rule the sibling-rule family does not enforce, so
    /// this filter cuts the enumeration down to exactly the streams
    /// `Version::decode` accepts.
    ///
    /// The first leaf's value is its absolute height; each later value is
    /// a zigzag delta (`2m -> +m`, `2m - 1 -> -m`).
    pub fn heights_nonnegative(&self) -> bool {
        let mut height: i128 = 0;
        for (i, &v) in self.values.iter().enumerate() {
            if i == 0 {
                height = i128::from(v);
            } else if v % 2 == 1 {
                height -= i128::from(v / 2 + 1);
            } else {
                height += i128::from(v / 2);
            }
            if height < 0 {
                return false;
            }
        }
        true
    }
}

/// Append the gamma code of `v` (`k` zeros, then `v + 1` in `k + 1` bits).
fn gamma_bits(bits: &mut Vec<bool>, v: u64) {
    let m = v + 1;
    let k = 63 - m.leading_zeros() as usize;
    for _ in 0..k {
        bits.push(false);
    }
    for i in (0..=k).rev() {
        bits.push(m >> i & 1 == 1);
    }
}

/// Every version-grammar subtree of exactly `n` bits.
///
/// The preorder flag-plus-gamma coding under the sibling rule (an
/// internal node's two bare-leaf children may not carry a zero right
/// code), heights unconstrained — filter with
/// [`VersionMember::heights_nonnegative`].
pub fn version_subtrees(n: usize) -> Vec<VersionMember> {
    let mut out = Vec::new();
    // A bare leaf: flag 1 plus one gamma bucket, n = 2k + 2.
    if n >= 2 && n.is_multiple_of(2) {
        let k = (n - 2) / 2;
        let lo = (1u64 << k) - 1;
        let hi = (1u64 << (k + 1)) - 1;
        for v in lo..hi {
            let mut bits = vec![true];
            gamma_bits(&mut bits, v);
            out.push(VersionMember {
                bits,
                bare_leaf: true,
                values: vec![v],
            });
        }
    }
    // An internal node: flag 0 plus a split (a, b), a + b = n - 1.
    if n >= 5 {
        for a in 2..=(n - 3) {
            let b = n - 1 - a;
            // Materialize each side once: re-enumerating the right side
            // per left member would be exponential in depth.
            let rights = version_subtrees(b);
            for left in version_subtrees(a) {
                for right in &rights {
                    // The collapsible sibling pair: both children bare
                    // leaves, right code zero.
                    if left.bare_leaf && right.bare_leaf && right.values == [0] {
                        continue;
                    }
                    let mut bits = vec![false];
                    bits.extend_from_slice(&left.bits);
                    bits.extend_from_slice(&right.bits);
                    let mut values = left.values.clone();
                    values.extend_from_slice(&right.values);
                    out.push(VersionMember {
                        bits,
                        bare_leaf: false,
                        values,
                    });
                }
            }
        }
    }
    out
}

/// Every canonical id subtree of exactly `n` bits: 2-bit presence tags,
/// no node with two terminal children. The `bool` is "this subtree is the
/// bare terminal".
pub fn party_subtrees(n: usize) -> Vec<(Vec<bool>, bool)> {
    let mut out = Vec::new();
    if n == 2 {
        out.push((vec![false, false], true));
    }
    if n >= 4 {
        for tag in [[true, false], [false, true]] {
            for (child, _) in party_subtrees(n - 2) {
                let mut bits = tag.to_vec();
                bits.extend_from_slice(&child);
                out.push((bits, false));
            }
        }
    }
    if n >= 6 {
        for a in 2..=(n - 4) {
            let b = n - 2 - a;
            let rights = party_subtrees(b);
            for (left, left_term) in party_subtrees(a) {
                for (right, right_term) in &rights {
                    if left_term && *right_term {
                        continue;
                    }
                    let mut bits = vec![true, true];
                    bits.extend_from_slice(&left);
                    bits.extend_from_slice(right);
                    out.push((bits, false));
                }
            }
        }
    }
    out
}

/// Pack an unpadded bit stream into canonical stored bytes (MSB-first,
/// final partial byte zero-padded).
pub fn pack(bits: &[bool]) -> Vec<u8> {
    let mut sink = crate::sample::BitSink::default();
    for &b in bits {
        sink.push(b);
    }
    sink.into_bytes()
}
