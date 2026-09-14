//! Serde adapters shared by the bookmark envelope and its records.

use std::{fmt, marker::PhantomData};

use ::serde::de::{Error as _, IgnoredAny, SeqAccess, Visitor, value::SeqAccessDeserializer};
use ::serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serialize a borrowed slice as a byte string.
pub(super) struct Bytes<'a>(
    /// The opaque contents to encode.
    pub &'a [u8],
);

/// Keep opaque bytes in CBOR's byte-string representation.
impl Serialize for Bytes<'_> {
    /// Serialize the borrowed contents without copying them.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.0)
    }
}

/// Deserialize a tuple and require its array to end after the expected fields.
pub(super) struct Complete<T>(
    /// The tuple whose closing boundary has been consumed.
    pub T,
);

/// Consume the tuple's end, including a break for an indefinite CBOR array.
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Complete<T> {
    /// Let Serde decode the fields, then reject any extra element.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Decode a complete tuple of the caller's field types.
        struct Fields<T>(
            /// The tuple type to deserialize.
            PhantomData<T>,
        );

        /// Check the array boundary after the tuple consumes its fields.
        impl<'de, T: Deserialize<'de>> Visitor<'de> for Fields<T> {
            /// The decoded tuple with its closing boundary consumed.
            type Value = Complete<T>;

            /// Describe the expected shape for decoding errors.
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a tuple with exactly its expected fields")
            }

            /// Decode fields and consume the end of a definite or indefinite array.
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let value = T::deserialize(SeqAccessDeserializer::new(&mut seq))?;
                // Ciborium leaves tuple-end checking to the visitor. Without
                // this read, an indefinite array's break could end its parent.
                if seq.next_element::<IgnoredAny>()?.is_some() {
                    return Err(A::Error::custom("extra tuple field"));
                }
                Ok(Complete(value))
            }
        }

        deserializer.deserialize_seq(Fields(PhantomData))
    }
}
