use std::fmt::Debug;
use std::sync::LazyLock;

/// Width in bytes of the tree's Merkle hashes.
///
/// The subtree-comparison digests that gossip exchanges, surfaced as
/// [`Snapshot::hash`](crate::Snapshot::hash). Narrower than the 32-byte
/// version-derived leaf path; the width argument is in [the reconciliation
/// docs](crate::reconciliation).
pub const MERKLE_HASH_LEN: usize = 24;

/// A 24-byte Merkle hash.
///
/// A newtype over a fixed-size byte array; on the wire it travels as its
/// raw bytes, never length-prefixed (the width is pinned by the type).
///
/// The underlying primitive is [`blake3`], truncated to its leading
/// [`MERKLE_HASH_LEN`] bytes — BLAKE3 is an extendable-output function, so
/// prefix truncation is the sanctioned narrow form, with collision resistance
/// 2⁹⁶ and preimage resistance 2¹⁹². Callers use [`Hash::of`] (or
/// [`ContentHash`] for the full width) and never touch the `blake3` types
/// directly.
///
/// # Why 24 bytes here, and 32 for content
///
/// A Merkle hash is only ever an equality probe between two peers' subtrees at
/// the same prefix, so a false-equal costs what the causal sieve makes of it:
/// the divergent messages under a landed false-equal are eventually read as
/// seen-but-absent and deleted fleet-wide (the full argument is in [the
/// reconciliation docs](crate::reconciliation)). The width prices that event
/// in-model:
///
/// - **Accident.** The hash at prefix `P` is only ever compared against
///   the counterparty's hash at the same `P`, so a false-equal is a
///   per-interior-comparison event at 2⁻¹⁹² — pairwise, never
///   birthday-amplified across the tree's population.
/// - **Grinding.** For an author of message *content* who is not a peer —
///   the one adjacent actor the trust model admits — the offline birthday
///   floor for assembling any colliding content pair is 2⁹⁶ hash
///   evaluations, which closes that vector unconditionally, with no
///   premise about what an attempt would cost the attacker.
///
/// Hostile *peers* are off-model: peers in a universe trust one another
/// ([the crate docs](crate) make a compromised member's powers explicit),
/// so no width buys anything against a member, and none is priced here.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
#[repr(transparent)]
pub struct Hash(pub [u8; MERKLE_HASH_LEN]);

impl Debug for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

/// Domain-separation tag leading a leaf's hash preimage.
///
/// Leaves are version-addressed (the path is the full-width hash of the
/// leaf's version; see [`Path::for_leaf`](super::Path::for_leaf)), so a
/// leaf's preimage commits its compressed suffix — path bytes — and its
/// version's canonical encoding, never its message bytes: every compared
/// digest in the tree is a pure function of the version set.
const LEAF_TAG: u8 = 0;

/// Domain-separation tag leading a branch's hash preimage.
///
/// The kind byte makes leaf/branch separation checkable from the first
/// byte alone. The nearest pair of shapes — a leaf with an *empty*
/// suffix (its parent sits at depth 31) and the empty root — would
/// otherwise differ only in preimage length (two bytes against four),
/// so the tag is the stated separator and the length difference the
/// backstop.
const BRANCH_TAG: u8 = 1;

/// Bytes a single child contributes to a branch preimage: its radix byte
/// followed by its [`MERKLE_HASH_LEN`]-byte hash.
const CHILD_RECORD_LEN: usize = 1 + MERKLE_HASH_LEN;

impl Hash {
    /// One-shot Merkle hash of a contiguous byte slice: the leading
    /// [`MERKLE_HASH_LEN`] bytes of the full-width hash of the same bytes.
    pub fn of(bytes: &[u8]) -> Self {
        ContentHash::of(bytes).truncate()
    }

    /// The hash of a leaf observed from the top of its compressed `suffix`:
    /// `blake3(LEAF_TAG ‖ suffix_len ‖ suffix ‖ version)`.
    ///
    /// `suffix` is the leaf's path-compressed span in **path order** —
    /// shallowest byte first, as the node serializer emits it — and
    /// `suffix_len` is one byte (a compressed span never exceeds the
    /// 32-byte path). `version` is the leaf's canonical encoding:
    /// self-delimiting, so the preimage stays injective with the suffix
    /// length-tagged and the version last.
    ///
    /// A leaf commits its path bytes and its version, never its message
    /// bytes: every compared digest is a pure function of the version set,
    /// and a content author contributes no bit to any compared quantity.
    /// The path already commits the version through its hash
    /// ([`Path::for_leaf`](super::Path::for_leaf)); committing the raw
    /// version bytes too makes two *distinct* versions that collided into
    /// one path (off-model) digest-unequal, so the merge walk surfaces
    /// that impossibility as a local violation instead of keeping a side.
    ///
    /// # Panics
    ///
    /// Panics if `suffix` exceeds 255 bytes. Unreachable through the typed
    /// tree, whose height cap bounds compressed spans at the 32-byte path.
    pub fn leaf(suffix: &[u8], version: &crate::Version) -> Self {
        let version = version.as_bytes();
        let suffix_len =
            u8::try_from(suffix.len()).expect("a compressed span fits in one length byte");
        let mut buf = Vec::with_capacity(2 + suffix.len() + version.len());
        buf.push(LEAF_TAG);
        buf.push(suffix_len);
        buf.extend_from_slice(suffix);
        buf.extend_from_slice(version);
        Hash::of(&buf)
    }

    /// The hash of a branch observed from the top of its compressed
    /// `prefix`:
    /// `blake3(BRANCH_TAG ‖ prefix_len ‖ prefix ‖ child_count ‖ r₀ ‖ h₀ ‖ …)`.
    ///
    /// One preimage per node, `children` given as `(radix, child hash)`
    /// pairs in ascending radix order, every variable-width field
    /// length-tagged:
    ///
    /// - `prefix` is the branch's path-compressed span in **path order** —
    ///   shallowest byte first, as the node serializer emits it —
    ///   and `prefix_len` is one byte.
    /// - `child_count` is a big-endian `u16`: the count ranges over
    ///   {0} ∪ \[2, 256\] — zero only for the [empty root](Hash::empty_root),
    ///   and never one, by the path-compression invariant (see below) —
    ///   which overflows a biased byte.
    /// - Each child is a fixed-width `radix ‖ hash` record
    ///   ([`CHILD_RECORD_LEN`] bytes). Empty slots are *omitted*, not
    ///   zero-filled.
    ///
    /// The explicit lengths make preimage injectivity *locally* checkable:
    /// no two distinct `(kind, prefix, children)` triples encode to the same
    /// byte string, by inspection of the fields alone.
    ///
    /// # Canonicity
    ///
    /// The convention defines no hash for a one-child branch: such a node
    /// is unrepresentable — the tree's constructors collapse a singleton
    /// branch into its child's compressed prefix, so every materialized
    /// branch carries at least two children and maximal prefixes, and the
    /// tree's shape is a pure function of its content. Equal content
    /// therefore yields equal shape, hence equal `(prefix, children)`
    /// fields, hence equal hashes, however two peers compressed or arrived
    /// at that content. This is the same ≥ 2-children maximal-compression
    /// invariant the node serializer's shape-discriminated decoding already
    /// rests on (see
    /// [`Node::serialize_to`](super::untyped::Node::serialize_to)); the
    /// canonicity proptests pin it so any future relaxation breaks loudly
    /// rather than desynchronizing hashes silently. In debug builds this
    /// function trips on a one-child fan and on out-of-order radixes at the
    /// call site.
    ///
    /// # Panics
    ///
    /// Panics if `prefix` exceeds 255 bytes. Unreachable through the typed
    /// tree, whose height cap bounds compressed spans at the 32-byte path.
    pub fn branch(prefix: &[u8], children: impl IntoIterator<Item = (u8, Hash)>) -> Self {
        // Assemble the whole preimage contiguously, then hash it in one shot.
        // Handing BLAKE3 a single large slice lets it engage its multi-block
        // SIMD compression; streaming a tiny `update` per field defeats that
        // and compresses block-by-block. Measured by `benches/branch_hash.rs`
        // over this preimage layout: ~2x faster for a saturated 256-child
        // branch, and never slower at the hot small nodes (short prefix,
        // small fan), whose whole preimage fits one 64-byte block and costs
        // a single compression. `size_hint` sizes the buffer exactly for
        // the fan/array/empty callers (all exact).
        let prefix_len =
            u8::try_from(prefix.len()).expect("a compressed span fits in one length byte");
        let children = children.into_iter();
        let mut buf =
            Vec::with_capacity(4 + prefix.len() + CHILD_RECORD_LEN * children.size_hint().0);
        buf.push(BRANCH_TAG);
        buf.push(prefix_len);
        buf.extend_from_slice(prefix);
        // The count is not known until the iterator is drained: reserve its
        // slot and backfill once the records are in.
        let count_at = buf.len();
        buf.extend_from_slice(&[0, 0]);
        let mut count: u16 = 0;
        let mut previous: Option<u8> = None;
        for (radix, child) in children {
            count = count
                .checked_add(1)
                .expect("branch fan-out is bounded by the 256-way radix");
            // The convention requires ascending radix order. The fan caller
            // guarantees it structurally (the fan's sorted invariant is
            // private, and every constructor preserves it); for direct and
            // test callers, trip at the violation site rather than as a
            // cross-peer hash desync.
            debug_assert!(
                previous.is_none_or(|previous| previous < radix),
                "branch children must arrive in strictly ascending radix order",
            );
            previous = Some(radix);
            buf.push(radix);
            buf.extend_from_slice(child.as_bytes());
        }
        // The convention defines no hash for a one-child branch (see
        // `# Canonicity`); computing one here would surface three layers
        // away as a silent cross-peer desync.
        debug_assert!(
            count != 1,
            "a one-child branch is unrepresentable under the canonical-shape invariant",
        );
        buf[count_at..count_at + 2].copy_from_slice(&count.to_be_bytes());
        Hash::of(&buf)
    }

    /// The hash of the empty tree: a prefixless branch with no children,
    /// `blake3(BRANCH_TAG ‖ 0 ‖ 0u16)`.
    pub fn empty_root() -> Self {
        // A compile-time constant: memoize it rather than re-hashing the
        // four fixed bytes on every empty-root read.
        static EMPTY_ROOT: LazyLock<Hash> = LazyLock::new(|| Hash::branch(&[], []));
        *EMPTY_ROOT
    }

    /// Reference to the raw [`MERKLE_HASH_LEN`] bytes.
    pub fn as_bytes(&self) -> &[u8; MERKLE_HASH_LEN] {
        &self.0
    }
}

impl From<[u8; MERKLE_HASH_LEN]> for Hash {
    fn from(bytes: [u8; MERKLE_HASH_LEN]) -> Self {
        Hash(bytes)
    }
}

impl From<Hash> for [u8; MERKLE_HASH_LEN] {
    fn from(hash: Hash) -> Self {
        hash.0
    }
}

/// Full-width 32-byte BLAKE3 hash: the identity primitive.
///
/// This is the width that carries identity. A leaf's path *is* a hash of this
/// width over its version's canonical bytes (see
/// [`Path::for_leaf`](super::Path::for_leaf)), and every ingestion site
/// treats one path as one identity, so a collision here would be permanent
/// split-brain — full width is load-bearing for the path even though the
/// comparison digests are narrower. A `ContentHash` is never stored in a
/// branch and never
/// travels as a hash on the wire; it reaches the protocol only as a leaf's path
/// bytes.
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// One-shot full-width hash of a contiguous byte slice.
    pub fn of(bytes: &[u8]) -> Self {
        ContentHash(*blake3::hash(bytes).as_bytes())
    }

    /// Truncate to the Merkle width: the leading [`MERKLE_HASH_LEN`] bytes.
    ///
    /// This is the *only* bridge between the two widths — a Merkle
    /// [`struct@Hash`] is, by definition, the prefix truncation of the
    /// full-width hash of the same preimage.
    pub fn truncate(self) -> Hash {
        let mut out = [0u8; MERKLE_HASH_LEN];
        out.copy_from_slice(&self.0[..MERKLE_HASH_LEN]);
        Hash(out)
    }

    /// Reference to the raw 32 bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<ContentHash> for [u8; 32] {
    fn from(hash: ContentHash) -> Self {
        hash.0
    }
}

#[cfg(test)]
mod tests;
