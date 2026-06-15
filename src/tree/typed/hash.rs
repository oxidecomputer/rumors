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
/// the same prefix (the mirror protocol's
/// [`uncertain`](crate::tree::mirror::message::Exchange::uncertain) channel).
/// It is never an identity: a false-equal prunes one divergent subtree as
/// already-matching, and heals on the next mutation beneath that prefix, which
/// perturbs every branch hash above it and forces a re-compare. Nothing is
/// dropped; the failure is delayed propagation, not corruption. Content
/// integrity rides on the leaf's *path*, the full-width [`ContentHash`] of
/// `(version, value)` (see [`Path::for_leaf`](super::Path::for_leaf)), where a
/// collision would be permanent, silent split-brain. The comparison signal can
/// afford to lose the bits, halving the protocol's dominant hash traffic, and
/// the identity cannot.
///
/// The width is sized against both failure sources, derived (not
/// measured) from the comparison structure and the trust model:
///
/// - **Accident.** The hash at prefix `P` is only ever compared against
///   the counterparty's hash at the same `P`, so a false-equal is a
///   per-comparison event at 2⁻¹²⁸ — pairwise, never birthday-amplified
///   across the tree's population. A fleet running a million
///   divergent-subtree comparisons every second for a century accumulates
///   ≈2⁻⁷⁶; machine failure modes dominate long before the hash does.
/// - **Attack.** Peers in a universe trust one another ([the crate
///   docs](crate) make a compromised member's powers explicit), and the
///   mirror protocol inserts provided subtrees without re-hashing, so a
///   member who could grind the 2⁶⁴ collision floor already desyncs peers
///   for free, at any width. A non-member cannot grind at all: branch
///   preimages inherit child hashes from what honest peers actually hold,
///   so each attempt costs an honest insertion rather than an offline
///   hash, and the prize is the transient, self-healing false-equal
///   above, not corruption.
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

/// Domain-separation tag prefixed to a leaf's hash preimage. Leaves are
/// content-addressed (the path is the leaf's content hash; see
/// [`Path::for_leaf`](super::Path::for_leaf)), so a leaf carries no
/// hash-distinguishing content and commits to nothing but this tag.
const LEAF_TAG: u8 = 0;

/// Domain-separation tag prefixed to a branch's hash preimage, distinguishing
/// it from a leaf so the two can never collide regardless of children.
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

    /// The hash of a leaf node: `blake3(LEAF_TAG)`, a constant.
    pub fn leaf() -> Self {
        // A compile-time constant; compute the BLAKE3 once and reuse it rather
        // than re-hashing the single tag byte on every leaf.
        static LEAF: LazyLock<Hash> = LazyLock::new(|| Hash::of(&[LEAF_TAG]));
        *LEAF
    }

    /// The hash of a branch over `children`, given as `(radix, child hash)`
    /// pairs in ascending radix order: `blake3(BRANCH_TAG ‖ r₀ ‖ h₀ ‖ …)`.
    ///
    /// This single rule applies at every level of the tree, whether the branch
    /// is a fully-materialized multi-child node or a single-child virtual level
    /// collapsed into a compressed prefix. Because a one-child branch hashes
    /// identically whether it is materialized or path-compressed, hashing is
    /// compression-invariant by construction. Empty slots are *omitted*, not
    /// zero-filled; the empty iterator yields the [empty-root](Hash::empty_root)
    /// hash.
    pub fn branch(children: impl IntoIterator<Item = (u8, Hash)>) -> Self {
        // Assemble the whole preimage contiguously, then hash it in one shot.
        // Handing BLAKE3 a single large slice lets it engage its multi-block
        // SIMD compression; streaming a tiny `update` per radix/hash defeats
        // that and compresses block-by-block. For a saturated 256-child branch
        // the contiguous form is ~2x faster. Fan-out is bounded at 256, so the
        // buffer never exceeds `1 + CHILD_RECORD_LEN * 256` bytes; `size_hint`
        // sizes it exactly for the `OrdMap`/array/empty callers (all exact).
        let children = children.into_iter();
        let mut buf = Vec::with_capacity(1 + CHILD_RECORD_LEN * children.size_hint().0);
        buf.push(BRANCH_TAG);
        for (radix, child) in children {
            buf.push(radix);
            buf.extend_from_slice(child.as_bytes());
        }
        Hash::of(&buf)
    }

    /// The hash of the empty tree: a branch with no children, `blake3(BRANCH_TAG)`.
    pub fn empty_root() -> Self {
        // A compile-time constant, like [`leaf`](Self::leaf): memoize it.
        static EMPTY_ROOT: LazyLock<Hash> = LazyLock::new(|| Hash::branch(std::iter::empty()));
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
