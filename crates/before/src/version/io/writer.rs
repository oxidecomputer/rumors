//! Builds a canonical Version stream from its leaves.
//!
//! # Contract
//!
//! A caller supplies output leaves from left to right. It writes the first
//! leaf with [`height`](VersionWriter::height), then each later leaf with its
//! signed [`change`](VersionWriter::change) from the preceding height. Each
//! call also gives the leaf's depth, which determines the tree topology. The
//! values must describe natural leaf heights; the writer canonicalizes their
//! topology but does not validate the cumulative height.
//!
//! The writer also owns canonicalization. Callers compute leaves but never
//! construct nodes or edit encoded ranges. They may copy the unchanged
//! remainder of a canonical subtree with
//! [`copy_subtree_remainder`](VersionWriter::copy_subtree_remainder).
//!
//! # Canonicalization
//!
//! Two equal leaf siblings collapse into their parent. Equality appears as the
//! right leaf's one-bit zero-delta code. A payload of at most 63 bits remains
//! pending in a `u64` until the next
//! leaf decides whether to emit or collapse it. A collapse keeps the left code,
//! discards the right zero delta, and may repeat at the next level.
//!
//! The left code remains correct because it expresses the merged leaf's height
//! relative to the leaf before the collapsing region. Compact stacks record
//! the pending leaf's path and the lengths of possible left-sibling codes. They
//! use bits per open ancestor; the writer keeps neither tree nodes nor a
//! per-leaf index.
//!
//! # Wide payloads
//!
//! Once a payload exceeds 63 bits, the writer emits it directly into
//! the output. If that resident payload later collapses, moving it through an
//! interleaved stream at each level would take `Θ(depth × payload width)` time.
//! The first such collapse therefore separates the prefix already written into
//! topology and payload streams. Further collapses edit only topology while the
//! wide payload remains in place.
//!
//! The separated form exists only inside the writer and is interleaved once at
//! the end, so the stored representation does not change. The first wide signed
//! delta has magnitude `2^31`: uncommon, but reachable and subject to the same
//! bound as every other input.
//!
//! # Bounds
//!
//! On the ordinary path, each narrow payload incurs bounded work: a collapse
//! may recover and emit a previously written left code once more. The wide path
//! adds one linear separation pass and one linear final interleaving pass.
//! Total work is linear in the input and output streams, even when one wide
//! code survives at every depth.
//!
//! The wide path uses a constant number of stream-sized buffers plus bits per
//! open ancestor. No allocation scales with depth times code width or with a
//! decoded numeric value.

use num_bigint::{BigInt, BigUint, Sign};

use crate::bits::stack::{BitStack, PackedU64Stack};
use crate::bits::{BitRead, Bits, BitsReader, BitsWriter};
use crate::version::io::PayloadRange;
use crate::Version;

mod pending;

use pending::PendingPayload;

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

/// Output storage before and after a wide collapse.
enum Output {
    /// The canonical representation, built directly.
    Interleaved(BitsWriter),
    /// Topology and payloads separated so collapses only truncate them.
    Split(SplitOutput),
}

/// Separate output streams used after a wide code reaches a collapse.
struct SplitOutput {
    /// Preorder internal-node and leaf flags.
    topology: BitsWriter,
    /// Payload codes in leaf order.
    payloads: BitsWriter,
}

impl Output {
    /// Create an interleaved output with room for `capacity` bits.
    fn with_capacity(capacity: u64) -> Self {
        Output::Interleaved(BitsWriter::with_capacity(capacity))
    }

    /// Append one internal-node flag.
    fn push_internal(&mut self) {
        match self {
            Output::Interleaved(out) => out.push(false),
            Output::Split(out) => out.topology.push(false),
        }
    }

    /// Append one leaf flag.
    fn push_leaf(&mut self) {
        match self {
            Output::Interleaved(out) => out.push(true),
            Output::Split(out) => out.topology.push(true),
        }
    }

    /// Append a pending narrow leaf; a wide leaf is already present.
    fn flush(&mut self, pending: &PendingPayload) {
        if let PendingPayload::Narrow { bits, len } = pending {
            self.push_leaf();
            self.payload().push_bits(*bits, u32::from(*len));
        }
    }

    /// The buffer into which a resident payload is written.
    fn payload(&mut self) -> &mut BitsWriter {
        match self {
            Output::Interleaved(out) => out,
            Output::Split(out) => &mut out.payloads,
        }
    }

    /// Collapse a pending left leaf with an incoming zero-delta right sibling.
    fn collapse_direct(&mut self, pending: &PendingPayload) {
        match pending {
            PendingPayload::Narrow { .. } => match self {
                Output::Interleaved(out) => out.truncate(out.len() - 1),
                Output::Split(out) => out.topology.truncate(out.topology.len() - 1),
            },
            PendingPayload::Wide { .. } => {
                self.ensure_split();
                let Output::Split(out) = self else {
                    unreachable!()
                };
                out.topology.truncate(out.topology.len() - 2);
                out.topology.push(true);
            }
        }
    }

    /// Remove an encoded left sibling and make its payload pending.
    fn take_left(&mut self, code_len: u64) -> PendingPayload {
        if code_len > NARROW_CODE_BITS {
            self.ensure_split();
            let Output::Split(out) = self else {
                unreachable!()
            };
            out.topology.truncate(out.topology.len() - 2);
            out.topology.push(true);
            return PendingPayload::Wide { len: code_len };
        }

        let len = code_len as u32;
        match self {
            Output::Interleaved(out) => {
                let start = out.len() - code_len;
                let bits = out.read_word(start, len);
                out.truncate(start - 2);
                PendingPayload::Narrow {
                    bits,
                    len: len as u8,
                }
            }
            Output::Split(out) => {
                let start = out.payloads.len() - code_len;
                let bits = out.payloads.read_word(start, len);
                out.payloads.truncate(start);
                out.topology.truncate(out.topology.len() - 2);
                PendingPayload::Narrow {
                    bits,
                    len: len as u8,
                }
            }
        }
    }

    /// Append an already-canonical continuation.
    fn splice_continuation(&mut self, src: &Bits, start: u64, end: u64) {
        match self {
            Output::Interleaved(out) => out.splice(src, start, end),
            Output::Split(out) => out.splice_continuation(src, start, end),
        }
    }

    /// Finish the canonical interleaved stream.
    fn finish(self) -> BitsWriter {
        match self {
            Output::Interleaved(out) => out,
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
        let bits = std::mem::replace(out, BitsWriter::with_capacity(0));
        *self = Output::Split(SplitOutput::from_interleaved(bits));
    }
}

impl SplitOutput {
    /// Separate a complete interleaved Version prefix.
    fn from_interleaved(bits: BitsWriter) -> Self {
        let mut topology = BitsWriter::with_capacity(0);
        let mut payloads = BitsWriter::with_capacity(0);
        let mut cursor = bits.reader();
        while cursor.position() < bits.len() {
            let internal_nodes = cursor
                .read_unary()
                .expect("the partial output ends at a payload boundary");
            for _ in 0..internal_nodes {
                topology.push(false);
            }
            topology.push(true);
            let code_start = cursor.position();
            cursor
                .skip_gamma()
                .expect("the partial output contains complete payloads");
            payloads.splice_writer(&bits, code_start, cursor.position());
        }
        SplitOutput { topology, payloads }
    }

    /// Separate an interleaved continuation while copying it.
    ///
    /// `end` may stop immediately before the continuation's final leaf flag
    /// when that leaf has a narrow payload held outside the output.
    fn splice_continuation(&mut self, src: &Bits, start: u64, end: u64) {
        let mut cursor = BitsReader::at(src, start);
        while cursor.position() < end {
            let internal_nodes = cursor
                .read_unary()
                .expect("a canonical continuation has a complete next leaf");
            for _ in 0..internal_nodes {
                self.topology.push(false);
            }
            if cursor.position() > end {
                debug_assert_eq!(cursor.position() - 1, end);
                return;
            }
            self.topology.push(true);
            let code_start = cursor.position();
            cursor
                .skip_gamma()
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
    fn finish(self) -> BitsWriter {
        let topology = self.topology;
        let payloads = self.payloads;
        let mut topology_cursor = topology.reader();
        let mut payload_cursor = payloads.reader();
        let mut out = BitsWriter::with_capacity(topology.len() + payloads.len());
        while topology_cursor.position() < topology.len() {
            let leaf = topology_cursor
                .read_bit()
                .expect("the built topology is complete");
            out.push(leaf);
            if leaf {
                let code_start = payload_cursor.position();
                payload_cursor
                    .skip_gamma()
                    .expect("every built leaf has a complete payload");
                out.splice_writer(&payloads, code_start, payload_cursor.position());
            }
        }
        debug_assert_eq!(payload_cursor.position(), payloads.len());
        out
    }
}

/// Writes one leaf payload into a Version builder.
///
/// The first 63 bits remain in a word. Writing another bit spills that word
/// directly into the Version output and sends every remaining bit there.
struct PayloadWriter<'a> {
    /// The Version writer receiving the completed payload.
    version: &'a mut VersionWriter,
    /// The leaf's depth.
    depth: u64,
    /// Bits staged before the payload becomes wide.
    staged: u64,
    /// Number of bits in `staged`.
    staged_len: u32,
    /// Total payload bits after spilling, or zero before spilling.
    wide_len: u64,
}

impl PayloadWriter<'_> {
    /// Write an absolute height through its Elias-gamma code.
    fn write_gamma(&mut self, value: &BigUint) {
        let mantissa = value + 1u32;
        self.push_zeros(mantissa.bits() - 1);
        self.push_magnitude(&mantissa);
    }

    /// Write a signed height change through its zigzag-gamma code.
    fn write_change(&mut self, change: &BigInt) {
        self.write_zigzag(change.sign(), change.magnitude());
    }

    /// Write a nonnegative height change without allocating a signed value.
    fn write_increase(&mut self, amount: &BigUint) {
        self.write_zigzag(Sign::Plus, amount);
    }

    /// Write a negative height change without allocating a signed value.
    fn write_decrease(&mut self, amount: &BigUint) {
        debug_assert_ne!(*amount, BigUint::ZERO, "negative zero has no encoding");
        self.write_zigzag(Sign::Minus, amount);
    }

    /// Write one sign and magnitude through the Version payload mapping.
    fn write_zigzag(&mut self, sign: Sign, magnitude: &BigUint) {
        debug_assert!(sign != Sign::Minus || *magnitude != BigUint::ZERO);
        self.push_zeros(magnitude.bits());
        self.push_magnitude(magnitude);
        self.push_bit(sign != Sign::Minus);
    }

    /// Append `len` zero bits to this payload.
    fn push_zeros(&mut self, mut len: u64) {
        while len >= u64::from(u64::BITS) {
            self.push_bits(0, u64::BITS);
            len -= u64::from(u64::BITS);
        }
        // The loop leaves fewer than 64 bits, so the narrowing is exact.
        self.push_bits(0, len as u32);
    }

    /// Append a magnitude's binary digits, most-significant first.
    fn push_magnitude(&mut self, value: &BigUint) {
        let bits = value.bits();
        if bits == 0 {
            return;
        }
        let mut words = value.iter_u64_digits().rev();
        let top = words.next().expect("a nonzero magnitude has one word");
        let top_len = ((bits - 1) % u64::from(u64::BITS) + 1) as u32;
        self.push_bits(top, top_len);
        for word in words {
            self.push_bits(word, u64::BITS);
        }
    }

    /// Copy one complete payload code from an existing stream.
    pub fn splice(&mut self, range: PayloadRange<'_>) {
        let (bits, range) = range.storage();
        let start = range.start;
        let end = range.end;
        assert!(
            start < end && end <= bits.reader().len(),
            "a copied payload is a nonempty range within its source"
        );
        let len = end - start;
        if self.wide_len == 0 && u64::from(self.staged_len) + len <= NARROW_CODE_BITS {
            let mut reader = BitsReader::at(bits, start);
            let word = reader
                .read_word(len as u32)
                .expect("the delimited payload is complete");
            self.push_bits(word, len as u32);
            return;
        }
        self.spill();
        self.version.out.payload().splice(bits, start, end);
        self.wide_len += len;
    }

    /// Append one payload bit.
    pub fn push_bit(&mut self, bit: bool) {
        self.push_bits(u64::from(bit), 1);
    }

    /// Append the low `len <= 64` payload bits of `value`.
    pub fn push_bits(&mut self, value: u64, len: u32) {
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
        self.version.out.payload().push_bits(value, len);
        self.wide_len += u64::from(len);
    }

    /// Spill the staged prefix once when the payload becomes wide.
    fn spill(&mut self) {
        if self.wide_len != 0 {
            return;
        }
        self.version.begin_wide(self.depth);
        self.version
            .out
            .payload()
            .push_bits(self.staged, self.staged_len);
        self.wide_len = u64::from(self.staged_len);
    }

    /// Install the completed narrow or wide payload as the newest leaf.
    fn finish(self) {
        if self.wide_len == 0 {
            assert!(self.staged_len > 0, "a leaf payload code is never empty");
            self.version
                .accept_narrow(self.depth, self.staged, self.staged_len as u8);
        } else {
            self.version.pending = Some(PendingPayload::Wide { len: self.wide_len });
        }
    }
}

/// A canonical Version-stream builder driven by the output leaf sequence.
pub struct VersionWriter {
    /// The output, split only if a wide payload reaches a collapse.
    out: Output,
    /// The newest leaf, omitted from the output only when narrow.
    pending: Option<PendingPayload>,
    /// Root-to-pending-leaf branch directions: `false` for left, `true` for
    /// right.
    path: BitStack,
    /// Parallel to `path`: whether a right branch's completed left sibling is
    /// a single leaf.
    ///
    /// A left branch and a level introduced by [`copy_subtree_remainder`](Self::copy_subtree_remainder)
    /// both store `false` because neither can participate in a collapse.
    left_leaf: BitStack,
    /// Payload lengths of possible left-sibling leaves, deepest last.
    lens: PackedU64Stack,
}

impl VersionWriter {
    /// Create a builder with room for `capacity` output bits.
    pub fn with_capacity(capacity: u64) -> Self {
        VersionWriter {
            out: Output::with_capacity(capacity),
            pending: None,
            path: BitStack::new(),
            left_leaf: BitStack::new(),
            lens: PackedU64Stack::new(),
        }
    }

    /// Append the next leaf at `depth`, writing its complete payload through
    /// `write`.
    ///
    /// The leaves must describe one tree from left to right. Each new depth
    /// must be reachable from the previous leaf by closing right edges, taking
    /// one right edge, and descending left; malformed sequences panic.
    fn payload(&mut self, depth: u64, write: impl FnOnce(&mut PayloadWriter<'_>)) {
        let mut payload = PayloadWriter {
            version: self,
            depth,
            staged: 0,
            staged_len: 0,
            wide_len: 0,
        };
        write(&mut payload);
        payload.finish();
    }

    /// Append an absolute leaf height.
    pub fn height(&mut self, depth: u64, height: &BigUint) {
        self.payload(depth, |out| out.write_gamma(height));
    }

    /// Append a signed height change from the preceding leaf.
    pub fn change(&mut self, depth: u64, change: &BigInt) {
        self.payload(depth, |out| out.write_change(change));
    }

    /// Append a positive height change.
    pub fn increase(&mut self, depth: u64, amount: &BigUint) {
        self.payload(depth, |out| out.write_increase(amount));
    }

    /// Append a negative height change.
    pub fn decrease(&mut self, depth: u64, amount: &BigUint) {
        self.payload(depth, |out| out.write_decrease(amount));
    }

    /// Append a zero height change.
    pub fn unchanged(&mut self, depth: u64) {
        self.payload(depth, |out| out.write_increase(&BigUint::ZERO));
    }

    /// Copy one complete payload code from an existing Version.
    pub fn copy_payload(&mut self, depth: u64, payload: PayloadRange<'_>) {
        self.payload(depth, |out| out.splice(payload));
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
    /// through one ordinary height or change call instead.
    #[allow(clippy::too_many_arguments)] // (src, start, end) is one logical range argument
    pub fn copy_subtree_remainder(
        &mut self,
        src: &Version,
        start: u64,
        end: u64,
        root_depth: u64,
        first_rel_depth: u64,
        last_rel_depth: u64,
        last_code_len: u64,
    ) {
        let bits = &src.0;
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
        let mut flag_reader = BitsReader::at(bits, last_flag);
        let last_is_leaf = flag_reader
            .read_bit()
            .expect("the continuation flag is live");
        debug_assert!(last_is_leaf, "the continuation ends with a leaf");

        let first = self
            .pending
            .take()
            .expect("the subtree's first leaf was supplied");
        self.out.flush(&first);
        if last_code_len <= NARROW_CODE_BITS {
            self.out.splice_continuation(bits, start, last_flag);
            let mut payload = BitsReader::at(bits, last_flag + 1);
            self.pending = Some(PendingPayload::Narrow {
                bits: payload
                    .read_word(last_code_len as u32)
                    .expect("the final payload is complete"),
                len: last_code_len as u8,
            });
        } else {
            self.out.splice_continuation(bits, start, end);
            self.pending = Some(PendingPayload::Wide { len: last_code_len });
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

    /// Finish the canonical version.
    ///
    /// # Panics
    ///
    /// Panics if no leaf was appended or the leaves do not complete one tree.
    pub fn finish(mut self) -> Version {
        let pending = self
            .pending
            .take()
            .expect("a Version stream has at least one leaf");
        self.out.flush(&pending);
        assert!(
            self.path.all_set(),
            "the final leaf closes every open ancestor from the right"
        );
        Version::from_canonical(self.out.finish().finalize())
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
        self.pending = Some(PendingPayload::Narrow { bits, len });
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
        while self.pending.as_ref().map(PendingPayload::len) == Some(ZERO_DELTA_CODE_BITS)
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
