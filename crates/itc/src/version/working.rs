//! The transient fixed-width working form for event mutation (plan §5.5).
//!
//! A `Version` at rest is a variable-width packed bit stream; mutating it in place
//! would require back-patching variable-width integers. Instead a mutating batch
//! unpacks to this fixed-width form — a preorder topology bit per node plus a
//! [`Base`] (arbitrary-precision) per node — mutates (Phase 5), and repacks once at the
//! batch boundary. The indexed base array makes a node's integer an O(1) indexed
//! read/overwrite, and the unbounded value type means path sums can never overflow.
//!
//! Both `unpack` and `repack` are single iterative passes (no recursion on depth).

use crate::codec::{self, decode_int, Base, Bits, BitsSlice};
use crate::step;

/// Preorder topology + payload split. `topo[i]` is `true` iff node `i` is internal
/// (two children); `base[i]` is its stored (relative) integer. The left child of an
/// internal node is the next node in preorder; the right child follows the left
/// subtree. `topo.len() == base.len() == node count`.
pub(crate) struct WorkingVersion {
    pub(crate) topo: Bits,
    pub(crate) base: Vec<Base>,
}

impl WorkingVersion {
    /// Number of nodes. (Used by the working-form layout test.)
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.base.len()
    }
}

/// Unpack a canonical packed event tree into the working form. One forward pass over
/// the preorder stream: each node contributes one topology bit (its flag) and one
/// base. The input must be exactly one `enc_ev` tree (a `Version`'s stored bits).
pub(crate) fn unpack(packed: &BitsSlice) -> WorkingVersion {
    let mut topo = Bits::new();
    let mut base = Vec::new();
    let mut pos = 0;
    while pos < packed.len() {
        step!(); // one step per node processed
        let flag = packed[pos];
        pos += 1;
        let (b, next) = decode_int(packed, pos).expect("a Version holds canonical event bits");
        pos = next;
        topo.push(flag);
        base.push(b);
    }
    WorkingVersion { topo, base }
}

/// Repack the working form into the canonical packed stream. One forward pass: emit
/// each node's flag followed by its base as `gamma(base + 1)`, in preorder. Canonical
/// whenever the working form is in normal form.
pub(crate) fn repack(work: &WorkingVersion) -> Bits {
    let mut out = Bits::new();
    for (flag, base) in work.topo.iter().by_vals().zip(&work.base) {
        step!(); // one step per node processed
        out.push(flag);
        codec::encode_int(&mut out, base);
    }
    out
}
