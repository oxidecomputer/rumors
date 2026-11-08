use std::fmt::{self, Debug};

use hmac::{Hmac, KeyInit, Mac};
use sha2::{Sha256, digest::Output};

/// Width in bytes of the subtree digests exchanged during gossip.
///
/// [`struct@Hash`] gives the margin calculation that justifies this width.
pub(crate) const MERKLE_HASH_LEN: usize = 16;

/// Bytes in a leaf address: the full HMAC-SHA-256 output, one byte per tree
/// level.
pub const PATH_LEN: usize = 32;

/// A subtree's digest: HMAC-SHA-256 under the network's hash key
/// ([`HashKey`]) over the subtree's canonical layout, truncated to its
/// leading 16 bytes.
///
/// Each node caches its digest, and listings carry it: replicas compare a
/// listed digest with their own subtree's at the same trie position, so
/// equal version sets have equal digests and listing a node costs no
/// hashing. A leaf's digest commits the bytes of its path below its trie
/// position; a branch's commits its prefix and its children's radixes and
/// digests. Message payloads enter neither. Every preimage starts with a
/// kind tag and the height of the trie position the digest describes
/// ([`HashKey::leaf`], [`HashKey::branch`]), so leaves and branches, and
/// nodes at different heights, hash in separate domains.
///
/// The key is derived from the network's secret
/// ([`NetworkKeys`](crate::network::NetworkKeys)), so every digest anyone
/// sees, on the wire or in memory, is a MAC that nobody outside the network
/// can compute. Leaf paths are keyed too ([`PathKey`]), so the inputs of
/// leaf digests are hidden as well.
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
/// HMAC-SHA-256 is a pseudorandom function of its input under a secret
/// key, so for anyone without the key, the digests of two differing
/// subtrees agree with probability 2⁻⁸ʷ for a width of `w` bytes,
/// independently per comparison: 2⁻¹²⁸ at 16 bytes. Only corresponding
/// subtrees are compared, so the risk grows with the number of comparisons,
/// not with the number of pairs of nodes in either tree. Take a deliberately
/// absurd lifetime: 10⁶ peers, each running 10⁴ sessions a day of 10³
/// comparisons each, for 36,500 days. That is about 3.7 × 10¹⁷ comparisons,
/// or 2⁵⁸, so the chance of any false match over that whole lifetime is at
/// most about 2⁵⁸ × 2⁻¹²⁸ = 2⁻⁷⁰.
///
/// That bound also holds for inputs an adversary outside the peers steers.
/// Versions are deterministic, so such an adversary can infer them, and it can
/// choose when peers insert and gossip. Were the digest an unkeyed hash, it
/// could search offline for two version sets whose digests collide at one trie
/// position, a birthday search of about 2⁶⁴ evaluations at this width, and then
/// make those versions arise. Without the key it can evaluate no digest
/// offline. It can only collect the digests listings carry, and a collision
/// among those at one position takes about 2⁶⁴ distinct states of one node,
/// each made by insertions or redactions and listed in a session. Whoever holds
/// the secret can evaluate every digest, so a leaked secret permits the offline
/// search for the network's whole life, since the secret cannot be rotated
/// without moving every path (see `NetworkSecret`). This design should be
/// considered defense-in-depth against a highly improbable adversary model:
/// engineering such a collision would require substantial, though not
/// unthinkable, resources, perfect awareness by the attacker of the contents of
/// the gossip network, and for the gossip network to remain entirely static and
/// predictable while the offline attack is carried out. Consequently, it is an
/// intentional design choice to keep the Merkle digest width 16 bytes rather
/// than the 32 bytes which would decisively prevent such attacks even without a
/// keyed hash: we near-halve the metadata bandwidth required for
/// synchronization in so doing, and the keyed hash is a patch to raise the bar
/// back up to "you must compromise a peer to compromise the rumor set".
///
/// Leaf paths instead keep the full 32-byte [`PathHash`]. A path is a storage
/// address: a collision between *any* two versions would make them occupy
/// the same slot, so that risk grows with the number of pairs of versions,
/// quadratically in the number of versions. Keeping all 256 bits makes it
/// negligible across the whole population of versions.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
#[repr(transparent)]
pub struct Hash([u8; MERKLE_HASH_LEN]);

/// Format a subtree digest as hexadecimal.
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
/// domain tag, the height, the prefix's length byte, and the child count, a
/// big-endian `u16`.
const BRANCH_HEADER_LEN: usize = 3 * size_of::<u8>() + size_of::<u16>();

/// The most bytes [`HashKey::branch`] allocates for one preimage in the
/// typed tree: the header, a compressed prefix of at most [`PATH_LEN`]
/// bytes, and a record for each child of a full fan.
pub(crate) const MAX_BRANCH_PREIMAGE_LEN: usize =
    BRANCH_HEADER_LEN + PATH_LEN + CHILD_RECORD_LEN * (u8::MAX as usize + 1);

/// The most bytes the typed tree's hashing holds at once on one thread.
///
/// Computing a branch's hash allocates its preimage, a record per child,
/// and frees it before the hash returns; the node keeps only the hash.
/// Hashing never yields, so a thread holds the preimages of one hashing
/// descent at a time, which nests one preimage per branch whose hash it
/// computes along a path: at most one per path byte, since each branch
/// below another sits at least one byte deeper. Each preimage holds at
/// most [`MAX_BRANCH_PREIMAGE_LEN`] bytes.
pub(crate) const HASHING_SCRATCH_BYTES: usize = PATH_LEN * MAX_BRANCH_PREIMAGE_LEN;

/// Expose a digest's comparison bytes.
impl Hash {
    /// Reference to the raw [`MERKLE_HASH_LEN`] bytes.
    pub fn as_bytes(&self) -> &[u8; MERKLE_HASH_LEN] {
        &self.0
    }
}

/// Wrap digest bytes read off a listing, without hashing them again.
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

/// A full-width, 32-byte HMAC-SHA-256 output: a leaf path.
///
/// [`Path::for_leaf`](super::Path::for_leaf) hashes a version's canonical
/// encoding at this width, keyed by [`PathKey`]. Any two versions sharing an
/// address would compete for one leaf, so paths retain all 256 bits. The
/// shorter [`struct@Hash`] compares corresponding subtrees rather than
/// assigning storage addresses.
pub struct PathHash([u8; PATH_LEN]);

/// Format a leaf address digest as hexadecimal.
impl Debug for PathHash {
    /// Display the raw digest bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

/// Expose a leaf address's bytes.
impl PathHash {
    /// Reference to the raw 32 bytes.
    pub fn as_bytes(&self) -> &[u8; PATH_LEN] {
        &self.0
    }
}

/// Bytes of each HMAC key the network's key schedule derives: one SHA-256
/// output, HMAC-SHA-256's full-strength key.
pub(crate) const HMAC_KEY_LEN: usize = size_of::<Output<Sha256>>();

/// The keyed hash that derives a network's leaf paths: HMAC-SHA-256 under the
/// path key, which the network's key schedule derives from its secret
/// ([`NetworkKeys`](crate::network::NetworkKeys)).
///
/// Keying the paths makes where a version lands unpredictable to anyone
/// without the secret, so no one outside the network can choose versions
/// that crowd one subtree, and independently seeded networks place even
/// identical versions at unrelated addresses. It also hides the inputs of
/// leaf digests ([`HashKey::leaf`]), which hash a path's bytes below the
/// leaf's place in the trie: a descent puts a path's bytes on the wire, as
/// listing radixes, only down to the depth where it parts from every other
/// path either replica holds.
///
/// HMAC rather than a keyed prefix on a
/// plain SHA-256 input rules out length extension, for about one more
/// SHA-256 compression per path. Constructing a key absorbs the key's
/// two padded key blocks once, so each path then costs the two compressions
/// of a short message: one inner, one outer.
///
/// The keyed state is a function of the secret, so it is cleared when a key
/// drops, through the `zeroize` support of the `hmac` and `sha2` crates, as
/// the same best-effort hygiene as `NetworkSecret`'s own.
#[derive(Clone)]
pub struct PathKey(Hmac<Sha256>);

/// Derive leaf paths under one network's secret.
impl PathKey {
    /// Key the path hash with `key`, the path key the network's key
    /// schedule derives.
    ///
    /// [`NetworkKeys`](crate::network::NetworkKeys) derives each network's
    /// key once; everything else borrows it from there.
    pub(crate) fn new(key: &[u8; HMAC_KEY_LEN]) -> Self {
        Self(Hmac::new_from_slice(key).expect("HMAC accepts a key of any length"))
    }

    /// The key of trees tests build directly: the path hash under
    /// [`NetworkSecret::FIXTURE`](crate::network::NetworkSecret::FIXTURE),
    /// which `Tree::default` uses.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn fixture() -> Self {
        crate::network::NetworkKeys::fixture().path().clone()
    }

    /// The keyed full-width hash of `bytes`.
    pub fn hash(&self, bytes: &[u8]) -> PathHash {
        let mut mac = self.0.clone();
        mac.update(bytes);
        PathHash(mac.finalize().into_bytes().into())
    }
}

/// Redacts the key: its state is a function of the network's secret.
impl Debug for PathKey {
    /// Show only the type's name.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PathKey(..)")
    }
}

/// The keyed hash of a network's content tree: HMAC-SHA-256 under the
/// network's hash key, truncated to [`MERKLE_HASH_LEN`] bytes.
///
/// The network's key schedule derives the key from its secret
/// ([`NetworkKeys`](crate::network::NetworkKeys)).
///
/// It is the tree's only digest function: every node digest
/// ([`struct@Hash`]) is one evaluation of it over the node's canonical
/// layout, which [`leaf`](Self::leaf) and [`branch`](Self::branch) build.
/// [`struct@Hash`] says why the digests are keyed, and the key schedule's
/// docs say why the hash key, the path key and the public network
/// identifier reveal nothing about one another.
///
/// Constructing a key absorbs its two padded key blocks once, so each
/// digest then costs the compressions of its preimage, plus one outer
/// compression over the 32-byte inner hash.
///
/// The keyed state is a function of the secret, so it is cleared when a key
/// drops, as [`PathKey`]'s is.
#[derive(Clone)]
pub struct HashKey(Hmac<Sha256>);

/// Hash canonical node layouts under one network's secret.
impl HashKey {
    /// Key the tree hash with `key`, the hash key the network's key
    /// schedule derives.
    ///
    /// [`NetworkKeys`](crate::network::NetworkKeys) derives each network's
    /// key once; everything else borrows it from there.
    pub(crate) fn new(key: &[u8; HMAC_KEY_LEN]) -> Self {
        Self(Hmac::new_from_slice(key).expect("HMAC accepts a key of any length"))
    }

    /// The key of trees tests build directly: the tree hash under
    /// [`NetworkSecret::FIXTURE`](crate::network::NetworkSecret::FIXTURE),
    /// which `Tree::default` uses.
    #[cfg(test)]
    pub(crate) fn fixture() -> Self {
        crate::network::NetworkKeys::fixture().hash().clone()
    }

    /// The keyed hash of a contiguous byte slice: HMAC-SHA-256 of `bytes`
    /// under this key, truncated to its leading [`MERKLE_HASH_LEN`] bytes.
    ///
    /// [`leaf`](Self::leaf) and [`branch`](Self::branch) are this function
    /// over their layouts; tests use it to check those layouts byte for
    /// byte.
    pub fn of(&self, bytes: &[u8]) -> Hash {
        let mut mac = self.0.clone();
        mac.update(bytes);
        Self::truncated(mac)
    }

    /// Finish `mac` and keep its leading [`MERKLE_HASH_LEN`] bytes, the
    /// truncation SP 800-107 approves for HMAC outputs.
    fn truncated(mac: Hmac<Sha256>) -> Hash {
        let full = mac.finalize().into_bytes();
        let mut out = [0u8; MERKLE_HASH_LEN];
        out.copy_from_slice(&full[..MERKLE_HASH_LEN]);
        Hash(out)
    }

    /// Hash `LEAF_TAG ‖ height ‖ suffix_len ‖ suffix`.
    ///
    /// `height` is the height of the trie position the digest describes,
    /// one byte. The suffix runs from shallowest to deepest path byte,
    /// with its length encoded in one byte. A leaf sits at height zero, so
    /// the position's height is the suffix's length: both are committed,
    /// so every preimage carries its height in the same place.
    ///
    /// [`Path::for_leaf`](super::Path::for_leaf) derives the path from the
    /// version. Neither the version's encoding nor the payload is repeated
    /// here. Comparisons at the same trie position share the preceding path,
    /// so the suffix supplies the remaining bytes needed to identify the leaf.
    ///
    /// # Panics
    ///
    /// Panics if `height` or the suffix's length exceeds 255. Unreachable
    /// through the typed tree, whose heights and compressed spans are
    /// bounded by the 32-byte path. Debug builds also panic if `height`
    /// differs from the suffix's length.
    pub fn leaf(&self, height: usize, suffix: &[u8]) -> Hash {
        debug_assert_eq!(
            height,
            suffix.len(),
            "a leaf's position lies one level above it per suffix byte",
        );
        let height = u8::try_from(height).expect("a trie position's height fits in one byte");
        let suffix_len =
            u8::try_from(suffix.len()).expect("a compressed span fits in one length byte");
        // Successive updates hash the concatenated input without allocating
        // a temporary buffer for the header and suffix.
        let mut mac = self.0.clone();
        mac.update(&[LEAF_TAG, height, suffix_len]);
        mac.update(suffix);
        Self::truncated(mac)
    }

    /// Hash `BRANCH_TAG ‖ height ‖ prefix_len ‖ prefix ‖ child_count ‖
    /// (radix ‖ hash)*`.
    ///
    /// `height` is the height of the trie position the digest describes,
    /// the top of the prefix, one byte. The prefix runs from shallowest to
    /// deepest path byte. Its length is one byte; the child count is a
    /// big-endian `u16` so it can represent all 256 children. Each child
    /// contributes one radix byte and its hash. Empty slots contribute
    /// nothing. The lengths and fixed record width keep different node
    /// layouts from producing the same hash input, and with the height
    /// they fix the height of the branch beneath the prefix too.
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
    /// Panics if `height` or `prefix`'s length exceeds 255. Unreachable
    /// through the typed tree, whose heights and compressed spans are
    /// bounded by the 32-byte path.
    pub fn branch(
        &self,
        height: usize,
        prefix: &[u8],
        children: impl IntoIterator<Item = (u8, Hash)>,
    ) -> Hash {
        // A contiguous input lets SHA-256 compress complete blocks directly
        // from it, rather than copying each field into its block buffer.
        // `benches/branch_hash.rs` compares this with updates for each field.
        // The tree's child iterators have exact size hints, so one allocation
        // holds the prefix and every child record.
        let height = u8::try_from(height).expect("a trie position's height fits in one byte");
        let prefix_len =
            u8::try_from(prefix.len()).expect("a compressed span fits in one length byte");
        let children = children.into_iter();
        let mut buf = Vec::with_capacity(
            BRANCH_HEADER_LEN + prefix.len() + CHILD_RECORD_LEN * children.size_hint().0,
        );
        buf.extend_from_slice(&[BRANCH_TAG, height, prefix_len]);
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
        self.of(&buf)
    }

    /// The hash of the empty tree: a prefixless branch with no children at
    /// the root's height, `BRANCH_TAG ‖ 32 ‖ 0 ‖ 0u16`.
    #[cfg(any(test, feature = "test-internals"))]
    pub fn empty_root(&self) -> Hash {
        use super::height::{Height, Root};
        self.branch(Root::HEIGHT, &[], [])
    }
}

/// Redacts the key: its state is a function of the network's secret.
impl Debug for HashKey {
    /// Show only the type's name.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HashKey(..)")
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
