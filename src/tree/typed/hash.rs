use std::fmt::Debug;
use std::sync::LazyLock;

use borsh::{BorshDeserialize, BorshSerialize};

/// 32-byte hash newtype. Wraps a fixed-size byte array so borsh can be
/// derived without a length prefix and so the rest of the crate does not
/// depend on the underlying hash crate.
///
/// The underlying primitive is [`blake3`], but that is an implementation
/// detail: callers use [`Hash::of`] or [`Hasher`] and never touch the
/// `blake3` types directly.
#[derive(
    BorshSerialize, BorshDeserialize, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default,
)]
#[repr(transparent)]
pub struct Hash(pub [u8; 32]);

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
/// followed by its 32-byte hash.
const CHILD_RECORD_LEN: usize = 1 + 32;

impl Hash {
    /// One-shot hash of a contiguous byte slice.
    pub fn of(bytes: &[u8]) -> Self {
        Hash(*blake3::hash(bytes).as_bytes())
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

    /// Reference to the raw 32 bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
}

impl From<Hash> for [u8; 32] {
    fn from(hash: Hash) -> Self {
        hash.0
    }
}

/// Streaming hasher: equivalent to feeding the concatenation of every
/// `update` chunk through [`Hash::of`], without allocating an intermediate
/// buffer.
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
    pub fn finalize(self) -> Hash {
        Hash(*self.0.finalize().as_bytes())
    }
}

#[cfg(test)]
mod test;
