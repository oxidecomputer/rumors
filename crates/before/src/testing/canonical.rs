//! The types with a canonical byte encoding, read and written through their
//! public codec.
//!
//! [`Canonical`] lists them once, for the properties that drive every reader
//! `decode` through the scripted reader. It needs no codec feature, so those
//! properties run in every test build. The codec-feature properties extend it
//! with each type's serde and borsh entry points.

use std::fmt::Debug;
use std::io::Read;

use crate::error::Decode;
use crate::{Clock, Party, Rank, Ranked, Span, Version};

/// A type with a canonical byte encoding and the public entry points that
/// read and write it.
pub(crate) trait Canonical: Sized + PartialEq + Debug {
    /// Decodes one value from a reader, through the type's public `decode`.
    fn decode_reader<R: Read>(reader: R) -> Result<Self, Decode>;

    /// The value's canonical bytes, through the type's public `encode`.
    fn encode_canonical(&self) -> Vec<u8>;
}

/// A version's codec.
impl Canonical for Version {
    fn decode_reader<R: Read>(reader: R) -> Result<Self, Decode> {
        Version::decode(reader)
    }

    fn encode_canonical(&self) -> Vec<u8> {
        self.encode()
    }
}

/// A party's codec.
impl Canonical for Party {
    fn decode_reader<R: Read>(reader: R) -> Result<Self, Decode> {
        Party::decode(reader)
    }

    fn encode_canonical(&self) -> Vec<u8> {
        self.encode()
    }
}

/// A rank's codec.
impl Canonical for Rank {
    fn decode_reader<R: Read>(reader: R) -> Result<Self, Decode> {
        Rank::decode(reader)
    }

    fn encode_canonical(&self) -> Vec<u8> {
        self.encode()
    }
}

/// A clock's codec.
impl Canonical for Clock {
    fn decode_reader<R: Read>(reader: R) -> Result<Self, Decode> {
        Clock::decode(reader)
    }

    fn encode_canonical(&self) -> Vec<u8> {
        self.encode()
    }
}

/// A span's codec.
impl Canonical for Span<'static> {
    fn decode_reader<R: Read>(reader: R) -> Result<Self, Decode> {
        Span::decode(reader)
    }

    fn encode_canonical(&self) -> Vec<u8> {
        self.encode()
    }
}

/// A ranked key's codec.
impl Canonical for Ranked<'static> {
    fn decode_reader<R: Read>(reader: R) -> Result<Self, Decode> {
        Ranked::decode(reader)
    }

    fn encode_canonical(&self) -> Vec<u8> {
        self.encode()
    }
}
