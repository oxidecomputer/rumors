//! CBOR tag numbers identifying the crate's opaque atoms on its
//! serialized surfaces.
//!
//! The wire protocol and the stored bookmark spell party and version
//! atoms as CBOR byte strings wrapping their canonical bit-level
//! codings. Each such byte string is preceded by one of the tags
//! below, so the atom's identity travels with it: a generic CBOR tool
//! holding nothing but this table can pick the atoms out of a capture,
//! a bookmark, or a pasted snippet, with no knowledge of where in the
//! protocol they appeared.
//!
//! Two rules govern the tags:
//!
//! - **They are protocol vocabulary, written and read only by the
//!   transport and bookmark codecs.** The serde implementations of the
//!   underlying types stay untagged and format-agnostic: an
//!   application payload containing a version serializes identically
//!   to JSON, CBOR, or any other backend, and never carries a
//!   CBOR-specific concept.
//! - **The numbers are provisional, pending IANA registration.** They
//!   are drawn from the first-come-first-served range (32768 and up
//!   per RFC 8949 §9.2) and based at `0xD255` — the ASCII bytes `RU`
//!   with the range's high bit set — in currently unassigned space.
//!   Should registration assign a different block, the constants here
//!   move in a deliberate, versioned format change; nothing else in
//!   the crate hard-codes them.

/// Tags a byte string holding a party atom's canonical encoding.
pub const PARTY_TAG: u64 = 0xD255;

/// Tags a byte string holding a version atom's canonical encoding.
pub const VERSION_TAG: u64 = 0xD256;

/// Tags a byte string holding a clock's canonical encoding: a party
/// atom's bytes immediately followed by a version atom's bytes, as the
/// bookmark stores them.
pub const CLOCK_TAG: u64 = 0xD257;
