//! Canonical decoding from arbitrary bytes.
//!
//! This is the small, transport-independent target for high-throughput raw
//! decoder coverage. For every accepted value it checks byte identity, value
//! identity after a second decode, and stable re-encoding. Malformed bytes may
//! return any documented error but must never panic.

use core::fmt::Debug;

use before::{error::Decode, Clock, Party, Rank, Ranked, Span, Version};

use super::for_each_wire_type;

/// A value with one canonical raw encoding.
trait Wire: Debug + Eq + Sized {
    /// Decode a complete input.
    fn decode(data: &[u8]) -> Result<Self, Decode>;

    /// Return the value's canonical encoding.
    fn encode(&self) -> Vec<u8>;
}

/// Implement [`Wire`] for the public types that use inherent codecs.
macro_rules! impl_wire {
    ($type:ty, $name:literal, $kind:ident) => {
        impl Wire for $type {
            fn decode(data: &[u8]) -> Result<Self, Decode> {
                <$type>::decode(data)
            }

            fn encode(&self) -> Vec<u8> {
                <$type>::encode(self)
            }
        }
    };
}
for_each_wire_type!(impl_wire);

/// Run the canonical-decoding oracle over every raw wire type.
pub fn run(data: &[u8]) {
    /// Invoke [`round_trip`] for one entry in the shared wire-type list.
    macro_rules! check {
        ($type:ty, $name:literal, $kind:ident) => {
            round_trip::<$type>(data, $name);
        };
    }
    for_each_wire_type!(check);
}

/// Check the complete canonicality contract for one type.
fn round_trip<T: Wire>(data: &[u8], name: &str) {
    let Ok(value) = T::decode(data) else {
        return;
    };
    let encoded = value.encode();
    assert_eq!(encoded, data, "accepted {name} bytes are not canonical");
    let decoded = T::decode(&encoded)
        .unwrap_or_else(|error| panic!("an accepted {name} does not decode again: {error}"));
    assert_eq!(decoded, value, "accepted {name} changed after decoding");
    assert_eq!(decoded.encode(), encoded, "{name} re-encoding is unstable");
}
