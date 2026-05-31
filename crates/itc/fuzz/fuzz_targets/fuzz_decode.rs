//! PROG-5(a) / COV-7 — hostile decode fuzzing of the byte codec.
//!
//! Feed arbitrary bytes to every top-level `decode`. The contracts under test:
//!   1. `decode` never panics on any input (it returns `Ok` or `Err`).
//!   2. Any accepted value is canonical: re-encoding it then decoding again yields the
//!      same value and the same bytes (the keystone byte-equality invariant that
//!      `Eq`/`Hash` rely on).
//!
//! `is_normal` (the structural-lowering form of the same invariant) is asserted in the
//! in-tree proptests (`clock::tests::h34_decode_never_panics`), which can reach the
//! `oracle` lowering; this target guards the byte path with no nightly-only deps.

#![no_main]

use libfuzzer_sys::fuzz_target;

use itc::{Clock, Party, Version};

fuzz_target!(|data: &[u8]| {
    if let Ok(p) = Party::decode(data) {
        let bytes = p.encode();
        let again = Party::decode(&bytes).expect("re-decode of an accepted party is canonical");
        assert_eq!(again, p, "accepted party did not round-trip");
        assert_eq!(again.encode(), bytes, "party re-encode is not stable");
    }
    if let Ok(v) = Version::decode(data) {
        let bytes = v.encode();
        let again = Version::decode(&bytes).expect("re-decode of an accepted version is canonical");
        assert_eq!(again, v, "accepted version did not round-trip");
        assert_eq!(again.encode(), bytes, "version re-encode is not stable");
    }
    if let Ok(c) = Clock::decode(data) {
        let bytes = c.encode();
        let again = Clock::decode(&bytes).expect("re-decode of an accepted clock is canonical");
        assert_eq!(again.encode(), bytes, "clock re-encode is not stable");
    }
});
