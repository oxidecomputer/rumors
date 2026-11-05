use std::fmt::{self, Debug};
use std::sync::LazyLock;

use sha2::{Digest, Sha256};

/// Width in bytes of the subtree comparison digests exchanged during gossip.
///
/// [`struct@Hash`] gives the margin calculation that justifies this width.
pub(crate) const MERKLE_HASH_LEN: usize = 16;

/// Bytes in a leaf address: the full SHA-256 output, one byte per tree level.
pub const PATH_LEN: usize = 32;

/// A subtree comparison digest: the leading 16 bytes of SHA-256.
///
/// Peers compare these digests at the same trie position. Leaf hashes depend
/// on version-derived paths; branch hashes combine their prefix and ordered
/// child hashes. Message payloads do not enter either calculation.
///
/// Both these digests and leaf paths use SHA-256 because current processors
/// compute it in hardware: the SHA extensions on x86 and the SHA-2
/// extensions on Arm. `sha2` detects them at run time on x86, and on Arm
/// under Linux, Android, and Apple's systems; elsewhere it uses them only
/// when the build enables the target feature.
///
/// # Width
///
/// Each listed child carries a digest, so the width is the largest part of
/// a listing's cost on the wire, and we choose it as narrow as the failure
/// it risks allows. That failure is a *false match*: two differing subtrees
/// whose digests agree. A false match is silent loss. The descent settles
/// that comparison and skips the data exchange beneath it, so neither side
/// learns of the messages the other holds there, and once the shared causal
/// history advances past them, the causal sieve treats the missing messages
/// as redacted.
///
/// Under the crate's uniform-hash model, a comparison between corresponding
/// subtrees falsely matches with probability 2⁻⁸ʷ for a width of `w` bytes,
/// independently per comparison: 2⁻¹²⁸ at 16 bytes. Only corresponding
/// subtrees are compared, so the risk grows with the number of comparisons,
/// not with the number of pairs of nodes in either tree. Take a deliberately
/// absurd lifetime: 10⁶ peers, each running 10⁴ sessions a day of 10³
/// comparisons each, for 36,500 days. That is about 3.7 × 10¹⁷ comparisons,
/// or 2⁵⁸, so the chance of any false match over that whole lifetime is at
/// most about 2⁵⁸ × 2⁻¹²⁸ = 2⁻⁷⁰.
///
/// The threat model does not change this calculation. Hostile peers are
/// outside it, only authorized peers' clocks produce versions, and message
/// payloads enter no digest.
///
/// Leaf paths instead keep the full 32-byte [`PathHash`]. A path is a storage
/// address: a collision between *any* two versions would make them occupy
/// the same slot, so that risk grows with the number of pairs of versions,
/// quadratically in the number of versions. Keeping all 256 bits makes it
/// negligible across the whole population of versions.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
#[repr(transparent)]
pub struct Hash([u8; MERKLE_HASH_LEN]);

/// Format a comparison digest as hexadecimal.
impl Debug for Hash {
    /// Display the raw digest bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// Bytes a branch preimage holds before its prefix and child records: the
/// domain tag, the prefix's length byte, and the child count, a big-endian
/// `u16`.
const BRANCH_HEADER_LEN: usize = 2 * size_of::<u8>() + size_of::<u16>();

/// The most bytes [`Hash::branch`] allocates for one preimage in the typed
/// tree: the header, a compressed prefix of at most [`PATH_LEN`] bytes, and
/// a record for each child of a full fan.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const MAX_BRANCH_PREIMAGE_LEN: usize =
    BRANCH_HEADER_LEN + PATH_LEN + CHILD_RECORD_LEN * (u8::MAX as usize + 1);

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
        let mut hash = Sha256::new();
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
        // A contiguous input lets SHA-256 compress complete blocks directly
        // from it, rather than copying each field into its block buffer.
        // `benches/branch_hash.rs` compares this with updates for each field.
        // The tree's child iterators have exact size hints, so one allocation
        // holds the prefix and every child record.
        let prefix_len =
            u8::try_from(prefix.len()).expect("a compressed span fits in one length byte");
        let children = children.into_iter();
        let mut buf = Vec::with_capacity(
            BRANCH_HEADER_LEN + prefix.len() + CHILD_RECORD_LEN * children.size_hint().0,
        );
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
    /// `sha256(BRANCH_TAG ‖ 0 ‖ 0u16)`, truncated to the Merkle width.
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

/// A full-width, 32-byte SHA-256 digest used to derive leaf paths.
///
/// [`Path::for_leaf`](super::Path::for_leaf) hashes a version's canonical
/// encoding at this width. Any two versions sharing an address would compete
/// for one leaf, so paths retain all 256 bits. The shorter [`struct@Hash`]
/// compares corresponding subtrees rather than assigning storage addresses.
pub struct PathHash([u8; PATH_LEN]);

/// Format a leaf address digest as hexadecimal.
impl Debug for PathHash {
    /// Display the raw digest bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

/// Compute full-width digests and truncate them for subtree comparisons.
impl PathHash {
    /// One-shot full-width hash of a contiguous byte slice.
    pub fn of(bytes: &[u8]) -> Self {
        PathHash(Sha256::digest(bytes).into())
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
