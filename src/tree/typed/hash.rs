use borsh::{BorshDeserialize, BorshSerialize};

/// 32-byte hash newtype. Wraps a fixed-size byte array so we can derive borsh
/// without a length prefix and so the rest of the crate doesn't depend on the
/// underlying hash crate.
///
/// The underlying primitive is [`blake3`], but this is an implementation detail:
/// callers should reach for [`Hash::hash`] or [`Hasher`] and never touch the
/// `blake3` types directly.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Default,
)]
#[repr(transparent)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// One-shot hash of a contiguous byte slice.
    pub fn hash(bytes: &[u8]) -> Self {
        Hash(*blake3::hash(bytes).as_bytes())
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

/// Streaming hasher: equivalent to feeding the concatenation of every `update`
/// chunk through [`Hash::hash`], without allocating an intermediate buffer.
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
