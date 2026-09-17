//! Builds a canonical skyline stream from its leaves.
//!
//! # Contract
//!
//! A caller supplies the output leaves from left to right. For each leaf, it
//! gives the tree depth and writes one complete gamma code into a scoped
//! [`PayloadBuilder`]. The first code is an absolute height; each later code is
//! a signed delta from the preceding leaf. The depths determine the binary
//! tree's topology. The builder writes that topology in preorder, placing each
//! code after its leaf flag.
//!
//! The builder also owns canonicalization. Callers compute leaves but never
//! construct nodes or edit encoded ranges. They may copy the unchanged
//! remainder of a canonical subtree with
//! [`continue_verbatim`](SkylineBuilder::continue_verbatim).
//!
//! # Canonicalization
//!
//! Two equal leaf siblings collapse into their parent. Equality appears as the
//! right leaf's one-bit zero-delta code. The payload builder holds up to 63 bits
//! in a `u64`, so an ordinary payload remains pending until the next
//! leaf decides whether to emit or collapse it. A collapse keeps the left code,
//! discards the right zero delta, and may repeat at the next level.
//!
//! The left code remains correct because it expresses the merged leaf's height
//! relative to the leaf before the collapsing region. Compact stacks record
//! the pending leaf's path and the lengths of possible left-sibling codes. They
//! use bits per open ancestor; the builder keeps neither tree nodes nor a
//! per-leaf index.
//!
//! # Wide payloads
//!
//! Once a payload exceeds 63 bits, the payload builder writes it directly into
//! the output. If that resident payload later collapses, moving it through an
//! interleaved stream at each level would take `Θ(depth × payload width)` time.
//! The first such collapse therefore separates the prefix already written into
//! topology and payload streams. Further collapses edit only topology while the
//! wide payload remains in place.
//!
//! The separated form exists only inside the builder and is interleaved once at
//! the end, so the stored representation does not change. The first wide signed
//! delta has magnitude `2^31`: uncommon, but reachable and subject to the same
//! bound as every other input.
//!
//! # Bounds
//!
//! On the ordinary path, each payload is written once into the returned buffer.
//! The wide path adds one linear separation pass and one linear final
//! interleaving pass. Total work is linear in the input and output streams,
//! even when one wide code survives at every depth.
//!
//! The wide path uses a constant number of stream-sized buffers plus bits per
//! open ancestor. No allocation scales with depth times code width or with a
//! decoded numeric value.

use crate::codec::{
    gamma::Sink, BitBuilder, BitCursor, BitStack, BitsBuf, BitsView, DsiCursor, PopStack,
};

/// The 1-bit payload code: `gamma(zigzag(0))`, the zero delta.
///
/// The absolute code of a height-zero first leaf is also one bit, but that leaf
/// lies on the leftmost path and cannot be mistaken for an equal right sibling.
const ZERO_DELTA_CODE_BITS: u64 = 1;

/// The widest complete gamma code that can move through one `u64`.
///
/// Gamma-code widths are odd, so the largest one that fits a 64-bit word is 63
/// bits. This covers absolute values through `2^32 - 2` and signed-delta
/// magnitudes through `2^31 - 1`.
const NARROW_CODE_BITS: u64 = u64::BITS as u64 - 1;

/// The newest leaf's payload and whether it already resides in the output.
enum Pending {
    /// A code held outside the output in one `u64`.
    Narrow {
        /// The code, right-aligned.
        bits: u64,
        /// The code's length in bits.
        len: u8,
    },
    /// A code already ending the output's payload stream.
    Wide {
        /// The code's length in bits.
        len: u64,
    },
}

impl Pending {
    /// The payload's encoded length.
    fn len(&self) -> u64 {
        match self {
            Pending::Narrow { len, .. } => u64::from(*len),
            Pending::Wide { len } => *len,
        }
    }
}

/// Output storage before and after a wide collapse.
enum Output {
    /// The canonical representation, built directly.
    Interleaved(BitBuilder),
    /// Topology and payloads separated so collapses only truncate them.
    Split(SplitOutput),
}

/// Separate output streams used after a wide code reaches a collapse.
struct SplitOutput {
    /// Preorder internal-node and leaf flags.
    topology: BitBuilder,
    /// Payload codes in leaf order.
    payloads: BitBuilder,
}

impl Output {
    /// Create an interleaved output with room for `capacity` bits.
    fn with_capacity(capacity: u64) -> Self {
        Output::Interleaved(BitBuilder::with_capacity(capacity))
    }

    /// Append one internal-node flag.
    fn push_internal(&mut self) {
        match self {
            Output::Interleaved(out) => out.push_bit(false),
            Output::Split(out) => out.topology.push_bit(false),
        }
    }

    /// Append one leaf flag.
    fn push_leaf(&mut self) {
        match self {
            Output::Interleaved(out) => out.push_bit(true),
            Output::Split(out) => out.topology.push_bit(true),
        }
    }

    /// Append a pending narrow leaf; a wide leaf is already present.
    fn flush(&mut self, pending: &Pending) {
        if let Pending::Narrow { bits, len } = pending {
            self.push_leaf();
            self.payload().push_bits(*bits, u32::from(*len));
        }
    }

    /// The buffer into which a resident payload is written.
    fn payload(&mut self) -> &mut BitBuilder {
        match self {
            Output::Interleaved(out) => out,
            Output::Split(out) => &mut out.payloads,
        }
    }

    /// Collapse a pending left leaf with an incoming zero-delta right sibling.
    fn collapse_direct(&mut self, pending: &Pending) {
        match pending {
            Pending::Narrow { .. } => match self {
                Output::Interleaved(out) => out.truncate(out.len() - 1),
                Output::Split(out) => out.topology.truncate(out.topology.len() - 1),
            },
            Pending::Wide { .. } => {
                self.ensure_split();
                let Output::Split(out) = self else {
                    unreachable!()
                };
                out.topology.truncate(out.topology.len() - 2);
                out.topology.push_bit(true);
            }
        }
    }

    /// Remove an encoded left sibling and make its payload pending.
    fn take_left(&mut self, code_len: u64) -> Pending {
        if code_len > NARROW_CODE_BITS {
            self.ensure_split();
            let Output::Split(out) = self else {
                unreachable!()
            };
            out.topology.truncate(out.topology.len() - 2);
            out.topology.push_bit(true);
            return Pending::Wide { len: code_len };
        }

        let len = code_len as u32;
        match self {
            Output::Interleaved(out) => {
                let start = out.len() - code_len;
                let bits = out.read_word(start, len);
                out.truncate(start - 2);
                Pending::Narrow {
                    bits,
                    len: len as u8,
                }
            }
            Output::Split(out) => {
                let start = out.payloads.len() - code_len;
                let bits = out.payloads.read_word(start, len);
                out.payloads.truncate(start);
                out.topology.truncate(out.topology.len() - 2);
                Pending::Narrow {
                    bits,
                    len: len as u8,
                }
            }
        }
    }

    /// Append an already-canonical continuation.
    fn splice_continuation(&mut self, src: BitsView<'_>, start: u64, end: u64) {
        match self {
            Output::Interleaved(out) => out.splice(src, start, end),
            Output::Split(out) => out.splice_continuation(src, start, end),
        }
    }

    /// Finish the canonical interleaved stream.
    fn finish(self) -> BitsBuf {
        match self {
            Output::Interleaved(out) => out.finish(),
            Output::Split(out) => out.finish(),
        }
    }

    /// Separate the partial stream once, when a wide collapse first requires
    /// independent topology and payload edits.
    fn ensure_split(&mut self) {
        if matches!(self, Output::Split(_)) {
            return;
        }
        let Output::Interleaved(out) = self else {
            unreachable!()
        };
        let bits = std::mem::replace(out, BitBuilder::with_capacity(0)).finish();
        *self = Output::Split(SplitOutput::from_interleaved(bits));
    }
}

impl SplitOutput {
    /// Separate a complete interleaved skyline prefix.
    fn from_interleaved(bits: BitsBuf) -> Self {
        let src = crate::codec::built_view(&bits);
        let mut topology = BitBuilder::with_capacity(0);
        let mut payloads = BitBuilder::with_capacity(0);
        let mut cursor = DsiCursor::new(src);
        while cursor.position() < src.len() {
            let internal_nodes = cursor
                .read_unary()
                .expect("the partial output ends at a payload boundary");
            for _ in 0..internal_nodes {
                topology.push_bit(false);
            }
            topology.push_bit(true);
            let code_start = cursor.position();
            cursor
                .skip_int()
                .expect("the partial output contains complete payloads");
            payloads.splice(src, code_start, cursor.position());
        }
        SplitOutput { topology, payloads }
    }

    /// Separate an interleaved continuation while copying it.
    ///
    /// `end` may stop immediately before the continuation's final leaf flag
    /// when that leaf has a narrow payload held outside the output.
    fn splice_continuation(&mut self, src: BitsView<'_>, start: u64, end: u64) {
        let mut cursor = DsiCursor::new_at(src, start);
        while cursor.position() < end {
            let internal_nodes = cursor
                .read_unary()
                .expect("a canonical continuation has a complete next leaf");
            for _ in 0..internal_nodes {
                self.topology.push_bit(false);
            }
            if cursor.position() > end {
                debug_assert_eq!(cursor.position() - 1, end);
                return;
            }
            self.topology.push_bit(true);
            let code_start = cursor.position();
            cursor
                .skip_int()
                .expect("a canonical continuation has a complete payload");
            assert!(
                cursor.position() <= end,
                "a continuation ends at a payload boundary"
            );
            self.payloads.splice(src, code_start, cursor.position());
        }
        debug_assert_eq!(cursor.position(), end);
    }

    /// Interleave the completed topology and payload streams.
    fn finish(self) -> BitsBuf {
        let topology = self.topology.finish();
        let payloads = self.payloads.finish();
        let topology = crate::codec::built_view(&topology);
        let payloads = crate::codec::built_view(&payloads);
        let mut topology_cursor = DsiCursor::new(topology);
        let mut payload_cursor = DsiCursor::new(payloads);
        let mut out = BitBuilder::with_capacity(topology.len() + payloads.len());
        while topology_cursor.position() < topology.len() {
            let leaf = topology_cursor
                .read_bit()
                .expect("the built topology is complete");
            out.push_bit(leaf);
            if leaf {
                let code_start = payload_cursor.position();
                payload_cursor
                    .skip_int()
                    .expect("every built leaf has a complete payload");
                out.splice(payloads, code_start, payload_cursor.position());
            }
        }
        debug_assert_eq!(payload_cursor.position(), payloads.len());
        out.finish()
    }
}

/// Writes one leaf payload into a skyline builder.
///
/// The first 63 bits remain in a word. Writing another bit spills that word
/// directly into the skyline output and sends every remaining bit there.
pub(super) struct PayloadBuilder<'a> {
    /// The skyline receiving the completed payload.
    skyline: &'a mut SkylineBuilder,
    /// The leaf's depth.
    depth: u64,
    /// Bits staged before the payload becomes wide.
    staged: u64,
    /// Number of bits in `staged`.
    staged_len: u32,
    /// Total payload bits after spilling, or zero before spilling.
    wide_len: u64,
}

impl PayloadBuilder<'_> {
    /// Copy one complete payload code from an existing stream.
    pub(super) fn splice(&mut self, src: BitsView<'_>, start: u64, end: u64) {
        assert!(
            start < end && end <= src.len(),
            "a copied payload is a nonempty range within its source"
        );
        let len = end - start;
        if self.wide_len == 0 && u64::from(self.staged_len) + len <= NARROW_CODE_BITS {
            let bits = src.load_be(start, len as u32);
            self.push_bits(bits, len as u32);
            return;
        }
        self.spill();
        self.skyline.out.payload().splice(src, start, end);
        self.wide_len += len;
    }

    /// Append one payload bit.
    pub(super) fn push_bit(&mut self, bit: bool) {
        self.push_bits(u64::from(bit), 1);
    }

    /// Append the low `len <= 64` payload bits of `value`.
    pub(super) fn push_bits(&mut self, value: u64, len: u32) {
        assert!(len <= 64, "one payload append holds at most a word");
        assert!(
            len == 64 || value >> len == 0,
            "payload append has no bits above its stated width"
        );
        if len == 0 {
            return;
        }
        if self.wide_len == 0 && self.staged_len + len <= NARROW_CODE_BITS as u32 {
            self.staged = (self.staged << len) | value;
            self.staged_len += len;
            return;
        }
        self.spill();
        self.skyline.out.payload().push_bits(value, len);
        self.wide_len += u64::from(len);
    }

    /// Spill the staged prefix once when the payload becomes wide.
    fn spill(&mut self) {
        if self.wide_len != 0 {
            return;
        }
        self.skyline.begin_wide(self.depth);
        self.skyline
            .out
            .payload()
            .push_bits(self.staged, self.staged_len);
        self.wide_len = u64::from(self.staged_len);
    }

    /// Install the completed narrow or wide payload as the newest leaf.
    fn finish(self) {
        if self.wide_len == 0 {
            assert!(self.staged_len > 0, "a leaf payload code is never empty");
            self.skyline
                .accept_narrow(self.depth, self.staged, self.staged_len as u8);
        } else {
            self.skyline.pending = Some(Pending::Wide { len: self.wide_len });
        }
    }
}

/// Writes a gamma code directly into the pending skyline leaf.
impl Sink for PayloadBuilder<'_> {
    fn push_bit(&mut self, bit: bool) {
        self.push_bit(bit);
    }

    fn push_bits(&mut self, value: u64, len: u32) {
        self.push_bits(value, len);
    }
}

/// A canonical-skyline stream builder driven by the output leaf sequence.
pub(super) struct SkylineBuilder {
    /// The output, split only if a wide payload reaches a collapse.
    out: Output,
    /// The newest leaf, omitted from the output only when narrow.
    pending: Option<Pending>,
    /// Root-to-pending-leaf branch directions: `false` for left, `true` for
    /// right.
    path: BitStack,
    /// Parallel to `path`: whether a right branch's completed left sibling is
    /// a single leaf.
    ///
    /// A left branch and a level introduced by [`continue_verbatim`](Self::continue_verbatim)
    /// both store `false` because neither can participate in a collapse.
    left_leaf: BitStack,
    /// Payload lengths of possible left-sibling leaves, deepest last.
    lens: PopStack,
}

impl SkylineBuilder {
    /// Create a builder with room for `capacity` output bits.
    pub(super) fn with_capacity(capacity: u64) -> Self {
        SkylineBuilder {
            out: Output::with_capacity(capacity),
            pending: None,
            path: BitStack::new(),
            left_leaf: BitStack::new(),
            lens: PopStack::new(),
        }
    }

    /// Append the next leaf at `depth`, writing its complete payload through
    /// `write`.
    ///
    /// The leaves must describe one tree from left to right. Each new depth
    /// must be reachable from the previous leaf by closing right edges, taking
    /// one right edge, and descending left; malformed sequences panic.
    pub(super) fn leaf(&mut self, depth: u64, write: impl FnOnce(&mut PayloadBuilder<'_>)) {
        let mut payload = PayloadBuilder {
            skyline: self,
            depth,
            staged: 0,
            staged_len: 0,
            wide_len: 0,
        };
        write(&mut payload);
        payload.finish();
    }

    /// Splice the remainder of a canonical multi-leaf subtree verbatim.
    ///
    /// The caller has just supplied the subtree's first leaf at
    /// `root_depth + first_rel_depth`. `start..end` is the source range after
    /// that payload through the subtree's final payload. Deltas strictly inside
    /// a canonical subtree are unchanged by its surroundings, so the builder
    /// copies the range as one unit and resumes from its final leaf.
    ///
    /// Both relative depths are positive. A single-leaf subtree is supplied
    /// entirely through [`leaf`](Self::leaf) instead.
    #[allow(clippy::too_many_arguments)] // (src, start, end) is one logical range argument
    pub(super) fn continue_verbatim(
        &mut self,
        src: BitsView<'_>,
        start: u64,
        end: u64,
        root_depth: u64,
        first_rel_depth: u64,
        last_rel_depth: u64,
        last_code_len: u64,
    ) {
        debug_assert!(
            first_rel_depth >= 1 && last_rel_depth >= 1,
            "a multi-leaf subtree's edge leaves sit below its root"
        );
        debug_assert!(
            self.pending.is_some() && self.path.len() == root_depth + first_rel_depth,
            "the subtree's first leaf remains at its supplied depth"
        );
        debug_assert!(
            end - start > last_code_len,
            "the continuation holds at least the last leaf's flag and code"
        );
        let last_flag = end - last_code_len - 1;
        debug_assert!(src.bit(last_flag), "the continuation ends with a leaf");

        let first = self
            .pending
            .take()
            .expect("the subtree's first leaf was supplied");
        self.out.flush(&first);
        if last_code_len <= NARROW_CODE_BITS {
            self.out.splice_continuation(src, start, last_flag);
            self.pending = Some(Pending::Narrow {
                bits: src.load_be(last_flag + 1, last_code_len as u32),
                len: last_code_len as u8,
            });
        } else {
            self.out.splice_continuation(src, start, end);
            self.pending = Some(Pending::Wide { len: last_code_len });
        }

        // Replace the first leaf's left-edge path with the last leaf's
        // right-edge path. Canonicity rules out collapses inside the copied
        // subtree, so these levels need no left-sibling records.
        while self.path.len() > root_depth {
            let direction = self.path.pop();
            debug_assert_eq!(
                direction,
                Some(false),
                "a subtree's first leaf lies on its leftmost path"
            );
            self.left_leaf.pop();
        }
        for _ in 0..last_rel_depth {
            self.path.push(true);
            self.left_leaf.push(false);
        }
    }

    /// Take the finished canonical stream.
    ///
    /// # Panics
    ///
    /// Panics if no leaf was appended or the leaves do not complete one tree.
    pub(super) fn finish(mut self) -> BitsBuf {
        let pending = self
            .pending
            .take()
            .expect("a skyline stream has at least one leaf");
        self.out.flush(&pending);
        assert!(
            self.path.all_set(),
            "the final leaf closes every open ancestor from the right"
        );
        self.out.finish()
    }

    /// Accept a completed narrow payload, collapsing an equal direct sibling
    /// before either narrow code is written.
    fn accept_narrow(&mut self, depth: u64, bits: u64, len: u8) {
        if u64::from(len) == ZERO_DELTA_CODE_BITS
            && self.pending.is_some()
            && depth == self.path.len()
            && self.path.last() == Some(false)
        {
            let pending = self.pending.take().expect("the left leaf is pending");
            self.out.collapse_direct(&pending);
            self.path.pop();
            self.left_leaf.pop();
            self.pending = Some(pending);
            self.collapse();
            return;
        }

        self.begin_leaf(depth);
        self.pending = Some(Pending::Narrow { bits, len });
    }

    /// Begin a wide leaf and place its flag before its payload spills.
    fn begin_wide(&mut self, depth: u64) {
        self.begin_leaf(depth);
        self.out.push_leaf();
    }

    /// Emit the previous leaf and advance its path to a non-collapsing leaf.
    fn begin_leaf(&mut self, depth: u64) {
        let Some(previous) = self.pending.take() else {
            self.descend_to(depth);
            return;
        };
        let previous_len = previous.len();
        self.out.flush(&previous);

        let mut closed_rights = 0u64;
        loop {
            match self.path.pop() {
                Some(true) => {
                    if self.left_leaf.pop() == Some(true) {
                        self.lens.pop();
                    }
                    closed_rights += 1;
                }
                Some(false) => {
                    self.left_leaf.pop();
                    break;
                }
                None => panic!("a leaf arrived after the tree was complete"),
            }
        }

        let left_is_leaf = closed_rights == 0;
        self.path.push(true);
        self.left_leaf.push(left_is_leaf);
        if left_is_leaf {
            self.lens.push(previous_len);
        }
        assert!(
            depth >= self.path.len(),
            "a leaf depth is above its required right edge"
        );
        self.descend_to(depth);
    }

    /// Collapse equal leaf siblings exposed by a direct collapse.
    fn collapse(&mut self) {
        while self.pending.as_ref().map(Pending::len) == Some(ZERO_DELTA_CODE_BITS)
            && self.path.last() == Some(true)
            && self.left_leaf.last() == Some(true)
        {
            let left_len = self.lens.pop();
            self.pending = Some(self.out.take_left(left_len));
            self.path.pop();
            self.left_leaf.pop();
        }
    }

    /// Descend left to `depth`, writing one internal-node flag per level.
    fn descend_to(&mut self, depth: u64) {
        for _ in self.path.len()..depth {
            self.out.push_internal();
            self.path.push(false);
            self.left_leaf.push(false);
        }
    }
}

#[cfg(test)]
mod tests;
