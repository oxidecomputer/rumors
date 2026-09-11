use std::fmt::Debug;
use std::sync::LazyLock;

use sha3::{Digest, Sha3_256};

/// Width in bytes of the subtree comparison digests exchanged during gossip
/// and returned by [`Snapshot::hash`](crate::Snapshot::hash).
pub const MERKLE_HASH_LEN: usize = 24;

/// Bytes in a leaf address: the full SHA3-256 output, one byte per tree level.
pub const PATH_LEN: usize = 32;

/// A subtree comparison digest: the leading 24 bytes of SHA3-256.
///
/// Peers compare these digests at the same trie position. Leaf hashes depend
/// on version-derived paths; branch hashes combine their prefix and ordered
/// child hashes. Message payloads do not enter either calculation.
///
/// Each listed child carries a digest, so using 24 bytes instead of 32 saves
/// eight bytes per entry. Under the crate's uniform-hash model, two unequal
/// inputs match with probability 2⁻¹⁹² per comparison. Only corresponding
/// subtrees are compared, so the risk grows with those comparisons, rather
/// than with every possible pair of nodes in the tree. The margin matters:
/// a false match skips data exchange, and advancing the shared causal history
/// can then cause missing messages to be treated as redacted.
///
/// Leaf paths instead use the full 32-byte [`PathHash`]. A path is a storage
/// address: a collision between any two versions would make them occupy the
/// same slot. Keeping all 256 bits makes that risk negligible across the
/// whole population of versions, whose number of possible pairs grows
/// quadratically.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
#[repr(transparent)]
pub struct Hash(pub [u8; MERKLE_HASH_LEN]);

/// Format a comparison digest as hexadecimal.
impl Debug for Hash {
    /// Display the raw digest bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

/// Domain tag distinguishing a leaf's hash input from a branch's.
const LEAF_TAG: u8 = 0;

/// Domain tag distinguishing a branch's hash input from a leaf's.
const BRANCH_TAG: u8 = 1;

/// Bytes a single child contributes to a branch preimage: its radix byte
/// followed by its [`MERKLE_HASH_LEN`]-byte hash.
const CHILD_RECORD_LEN: usize = 1 + MERKLE_HASH_LEN;

/// Hash canonical node layouts and expose their comparison bytes.
impl Hash {
    /// One-shot Merkle hash of a contiguous byte slice: the leading
    /// [`MERKLE_HASH_LEN`] bytes of the full-width hash of the same bytes.
    pub fn of(bytes: &[u8]) -> Self {
        PathHash::of(bytes).truncate()
    }

    /// Hash `LEAF_TAG ‖ suffix_len ‖ suffix`, truncated to the Merkle width.
    ///
    /// The suffix runs from shallowest to deepest path byte, with its length
    /// encoded in one byte. Typed-tree suffixes span at most the 32-byte path.
    ///
    /// [`Path::for_leaf`](super::Path::for_leaf) derives the path from the
    /// version. Neither the version's encoding nor the payload is repeated
    /// here. Comparisons at the same trie position share the preceding path,
    /// so the suffix supplies the remaining bytes needed to identify the leaf.
    ///
    /// # Panics
    ///
    /// Panics if `suffix` exceeds 255 bytes. Unreachable through the typed
    /// tree, whose height cap bounds compressed spans at the 32-byte path.
    pub fn leaf(suffix: &[u8]) -> Self {
        let suffix_len =
            u8::try_from(suffix.len()).expect("a compressed span fits in one length byte");
        // Successive updates hash the concatenated input without allocating
        // a temporary buffer for the header and suffix.
        let mut hash = Sha3_256::new();
        hash.update([LEAF_TAG, suffix_len]);
        hash.update(suffix);
        PathHash(hash.finalize().into()).truncate()
    }

    /// Hash `BRANCH_TAG ‖ prefix_len ‖ prefix ‖ child_count ‖ (radix ‖ hash)*`,
    /// truncated to the Merkle width.
    ///
    /// The prefix runs from shallowest to deepest path byte. Its length is
    /// one byte; the child count is a big-endian `u16` so it can represent
    /// all 256 children. Each child contributes one radix byte and its hash.
    /// Empty slots contribute nothing. The lengths and fixed record width
    /// keep different node layouts from producing the same hash input.
    ///
    /// # Canonicity
    ///
    /// Children must have distinct, ascending radixes. A branch has at least
    /// two children, except for the [empty root](Self::empty_root). Tree
    /// constructors collapse single-child branches and maximize compression,
    /// so equal version sets have the same layout and hash. Debug assertions
    /// catch single-child branches and unordered radixes here.
    ///
    /// # Panics
    ///
    /// Panics if `prefix` exceeds 255 bytes. Unreachable through the typed
    /// tree, whose height cap bounds compressed spans at the 32-byte path.
    pub fn branch(prefix: &[u8], children: impl IntoIterator<Item = (u8, Hash)>) -> Self {
        // A contiguous input lets SHA3 absorb complete blocks directly.
        // `benches/branch_hash.rs` compares this with updates for each field.
        // The tree's child iterators have exact size hints, so one allocation
        // holds the prefix and every child record.
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
            // Tree fans preserve this order; catch other callers that would
            // make a node's hash depend on child enumeration order.
            debug_assert!(
                previous.is_none_or(|previous| previous < radix),
                "branch children must arrive in strictly ascending radix order",
            );
            previous = Some(radix);
            buf.push(radix);
            buf.extend_from_slice(child.as_bytes());
        }
        // A singleton must be compressed into its child before hashing.
        debug_assert!(
            count != 1,
            "a one-child branch is unrepresentable under the canonical-shape invariant",
        );
        buf[count_at..count_at + 2].copy_from_slice(&count.to_be_bytes());
        Hash::of(&buf)
    }

    /// The hash of the empty tree: a prefixless branch with no children,
    /// `sha3_256(BRANCH_TAG ‖ 0 ‖ 0u16)`.
    pub fn empty_root() -> Self {
        /// The fixed empty-root digest, computed on first use.
        static EMPTY_ROOT: LazyLock<Hash> = LazyLock::new(|| Hash::branch(&[], []));
        *EMPTY_ROOT
    }

    /// Reference to the raw [`MERKLE_HASH_LEN`] bytes.
    pub fn as_bytes(&self) -> &[u8; MERKLE_HASH_LEN] {
        &self.0
    }
}

/// Wrap comparison bytes without hashing them again.
impl From<[u8; MERKLE_HASH_LEN]> for Hash {
    /// Construct a digest from its raw bytes.
    fn from(bytes: [u8; MERKLE_HASH_LEN]) -> Self {
        Hash(bytes)
    }
}

/// Recover the comparison bytes.
impl From<Hash> for [u8; MERKLE_HASH_LEN] {
    /// Unwrap the digest.
    fn from(hash: Hash) -> Self {
        hash.0
    }
}

/// A full-width, 32-byte SHA3-256 digest used to derive leaf paths.
///
/// [`Path::for_leaf`](super::Path::for_leaf) hashes a version's canonical
/// encoding at this width. Any two versions sharing an address would compete
/// for one leaf, so paths retain all 256 bits. The shorter [`struct@Hash`]
/// compares corresponding subtrees rather than assigning storage addresses.
pub struct PathHash([u8; PATH_LEN]);

/// Compute full-width digests and truncate them for subtree comparisons.
impl PathHash {
    /// One-shot full-width hash of a contiguous byte slice.
    pub fn of(bytes: &[u8]) -> Self {
        PathHash(Sha3_256::digest(bytes).into())
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
    pub fn as_bytes(&self) -> &[u8; PATH_LEN] {
        &self.0
    }
}

/// Recover the full-width digest bytes.
impl From<PathHash> for [u8; PATH_LEN] {
    /// Unwrap the digest.
    fn from(hash: PathHash) -> Self {
        hash.0
    }
}

#[cfg(test)]
mod tests;
