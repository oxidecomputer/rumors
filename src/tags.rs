//! CBOR tag numbers identifying the crate's opaque atoms.
//!
//! The wire protocol and stored bookmark format represent the crate's internal
//! [`Version`](crate::Version)s, [`Party`](before::Party)s, and
//! [`Clock`](before::Clock)s as CBOR byte strings wrapping their canonical
//! byte-level encodings. Each such byte string is serialized using the tags
//! defined herein, so the atom's type can be inspected by a generic CBOR tool
//! using nothing but this table.
//!
//! - **These tags are internal protocol encoding, written and read only by the
//!   transport and bookmark codecs.** The [`serde`] implementations of the
//!   underlying types stay untagged and format-agnostic.
//! - **The tag numbers herein are provisional, pending possible future IANA
//!   registration.** They are drawn from currently unassigned space in the
//!   first-come-first-served range (32768 and up per [RFC 8949
//!   §9.2](https://www.rfc-editor.org/info/rfc8949/#section-9.2)) and based at
//!   `0xD255`, i.e. the ASCII bytes `RU` (for **`ru`**`mors`), with the range's
//!   high bit set. Should future IANA registration assign a different block,
//!   the constants here would move in a versioned format change.

///   Tags a byte string holding a [`Party`](before::Party)'s encoding.
pub const PARTY_TAG: u64 = 0xD255;

/// Tags a byte string holding a [`Version`](before::Version)'s encoding.
pub const VERSION_TAG: u64 = 0xD256;

/// Tags a byte string holding a [`Clock`](before::Clock)'s encoding.
pub const CLOCK_TAG: u64 = 0xD257;

/// Tags a byte string holding a network's secret sealed for a bootstrapping
/// peer: HPKE's encapsulated key, the sealed secret, and its authentication
/// tag.
///
/// Public, as every tag the wire carries is, so that a tool reading a capture
/// can name the item; hidden from the rendered docs, as [`BOOTSTRAP_KEY_TAG`]
/// is, because no caller handles a bootstrap's items.
#[doc(hidden)]
pub const NETWORK_SECRET_TAG: u64 = 0xD258;

/// Tags a byte string holding the public key a bootstrapping peer draws for one
/// session.
#[doc(hidden)]
pub const BOOTSTRAP_KEY_TAG: u64 = 0xD259;
