use std::fmt::Debug;
use std::sync::LazyLock;

use borsh::{BorshDeserialize, BorshSerialize};

/// Width in bytes of the tree's Merkle hashes.
///
/// The subtree-comparison digests gossip exchanges, surfaced as
/// [`Snapshot::hash`](crate::Snapshot::hash). Half the width of a
/// [`Key`](crate::Key).
pub const MERKLE_HASH_LEN: usize = 16;

/// 16-byte Merkle hash newtype. Wraps a fixed-size byte array so borsh can be
/// derived without a length prefix and so the rest of the crate does not depend
/// on the underlying hash crate.
///
/// The underlying primitive is [`blake3`], truncated to its leading
/// [`MERKLE_HASH_LEN`] bytes — BLAKE3 is an extendable-output function, so
/// prefix truncation is the sanctioned narrow form, with collision resistance
/// 2⁶⁴ and preimage resistance 2¹²⁸. Callers use [`Hash::of`] (or
/// [`ContentHash`] for the full width) and never touch the `blake3` types
/// directly.
///
/// # Why 16 bytes here, and 32 for content
///
/// A Merkle hash is only ever an equality probe between two peers' subtrees at
/// the same prefix. The width is sized against both failure sources, derived
/// from the comparison structure and the trust model:
///
/// - **Accident.** The hash at prefix `P` is only ever compared against
///   the counterparty's hash at the same `P`, so a false-equal is a
///   per-comparison event at 2⁻¹²⁸ — pairwise, never birthday-amplified
///   across the tree's population.
/// - **Attack.** Peers in a universe trust one another ([the crate
///   docs](crate) make a compromised member's powers explicit), and the
///   mirror protocol inserts provided subtrees without re-hashing, so a
///   member who could grind the 2⁶⁴ collision floor already desyncs peers
///   for free, at any width.
#[derive(
    BorshSerialize, BorshDeserialize, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default,
)]
#[repr(transparent)]
pub struct Hash(pub [u8; MERKLE_HASH_LEN]);

impl Debug for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

/// Domain-separation tag leading a leaf's hash preimage.
///
/// Leaves are content-addressed (the path is the leaf's content hash; see
/// [`Path::for_leaf`](super::Path::for_leaf)), so a leaf's preimage commits
/// its compressed suffix — path bytes — and nothing else.
const LEAF_TAG: u8 = 0;

/// Domain-separation tag leading a branch's hash preimage.
///
/// Both tags are load-bearing, not legacy: a leaf may carry an *empty*
/// suffix (its parent sits at depth 31), and the kind byte is what
/// separates its preimage from the empty root's, rather than an argument
/// about field lengths.
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
    /// `blake3(LEAF_TAG ‖ suffix_len ‖ suffix)`.
    ///
    /// `suffix` is the leaf's path-compressed span in **path order** —
    /// shallowest byte first, as the node serializer emits it — and
    /// `suffix_len` is one byte (a compressed span never exceeds the 32-byte
    /// path). A leaf commits only its own path bytes: message and version
    /// are already committed by *where* the leaf sits (leaves are
    /// content-addressed; see [`Path::for_leaf`](super::Path::for_leaf)),
    /// and each parent commits its child's radix byte, so a root-to-leaf
    /// chain of preimages commits the full 32-byte path.
    ///
    /// # Panics
    ///
    /// Panics if `suffix` exceeds 255 bytes. Unreachable through the typed
    /// tree, whose height cap bounds compressed spans at the 32-byte path.
    pub fn leaf(suffix: &[u8]) -> Self {
        let suffix_len =
            u8::try_from(suffix.len()).expect("a compressed span fits in one length byte");
        let mut buf = Vec::with_capacity(2 + suffix.len());
        buf.push(LEAF_TAG);
        buf.push(suffix_len);
        buf.extend_from_slice(suffix);
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
    /// - Each child is a fixed 17-byte `radix ‖ hash` record. Empty slots
    ///   are *omitted*, not zero-filled.
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
        // and compresses block-by-block. For a saturated 256-child branch the
        // contiguous form is ~2x faster; for the hot small nodes (short
        // prefix, small fan) the whole preimage fits one 64-byte block, so a
        // node costs a single compression. `size_hint` sizes the buffer
        // exactly for the `OrdMap`/array/empty callers (all exact).
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
            // The convention requires ascending radix order, but only the
            // `OrdMap` caller guarantees it structurally: trip at the
            // violation site rather than as a cross-peer hash desync.
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

/// Full-width 32-byte BLAKE3 hash: the content-addressing primitive.
///
/// This is the width that carries identity. A leaf's path *is* a hash of this
/// width over its `(version, value)` (see
/// [`Path::for_leaf`](super::Path::for_leaf)), and
/// [`join`](crate::tree::traverse::join) resolves identical paths as identical
/// contents, so a collision here would be permanent, undetectable divergence —
/// full width is load-bearing, and every hash that feeds a path must use it (a
/// single Merkle-width component would cap the whole path's collision
/// resistance at 2⁶⁴). A `ContentHash` is never stored in a branch and never
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

/// Streaming full-width hasher: equivalent to feeding the concatenation of
/// every `update` chunk through [`ContentHash::of`], without allocating an
/// intermediate buffer.
#[derive(Default)]
pub struct Hasher(blake3::Hasher);

impl Hasher {
    /// Construct a fresh hasher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append `bytes` to the hash input.
    pub fn update(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.update(bytes);
        self
    }

    /// Finalize the hash and consume the hasher.
    pub fn finalize(self) -> ContentHash {
        ContentHash(*self.0.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests;
