//! Hostile decode fuzzing of the byte codec.
//!
//! Feed arbitrary bytes to every top-level `decode`. The contracts under test:
//!
//!   1. `decode` never panics on any input (it returns `Ok` or `Err`).
//!   2. Any accepted value is canonical: re-encoding it then decoding again yields the
//!      same value and the same bytes (the keystone byte-equality invariant that
//!      `Eq`/`Hash` rely on).
//!   3. No input's computation exceeds the harness heap cap (`before_fuzz::under_heap_cap`),
//!      so a resource amplifier is a crash finding, not a latent one.
//!
//! The roster is every public wire type: `Party`, `Version`, `Clock`, `Rank`,
//! `Ranked`, and `Span`. The composite decodes (`Ranked`, `Span`) are additionally
//! held to their composed counterparts — on rejection genre, not just accept — by
//! the sibling `fuzz_decode_differential` target.

#![no_main]

use libfuzzer_sys::fuzz_target;

use before::{causally::Span, Clock, Party, Rank, Ranked, Version};

fuzz_target!(|data: &[u8]| {
    before_fuzz::under_heap_cap(|| run(data));
});

/// One input's body: decode every top-level type and check the round-trip contract.
fn run(data: &[u8]) {
    if let Ok(p) = Party::decode(data) {
        let bytes = p.encode();
        let again = Party::decode(&bytes[..]).expect("re-decode of an accepted party is canonical");
        assert_eq!(again, p, "accepted party did not round-trip");
        assert_eq!(again.encode(), bytes, "party re-encode is not stable");
    }
    if let Ok(v) = Version::decode(data) {
        let bytes = v.encode();
        let again =
            Version::decode(&bytes[..]).expect("re-decode of an accepted version is canonical");
        assert_eq!(again, v, "accepted version did not round-trip");
        assert_eq!(again.encode(), bytes, "version re-encode is not stable");
    }
    if let Ok(c) = Clock::decode(data) {
        let bytes = c.encode();
        let again = Clock::decode(&bytes[..]).expect("re-decode of an accepted clock is canonical");
        assert_eq!(again.encode(), bytes, "clock re-encode is not stable");
    }
    if let Ok(r) = Rank::decode(data) {
        let bytes = r.encode();
        let again = Rank::decode(&bytes[..]).expect("re-decode of an accepted rank is canonical");
        assert_eq!(again, r, "accepted rank did not round-trip");
        assert_eq!(again.encode(), bytes, "rank re-encode is not stable");
    }
    if let Ok(k) = Ranked::decode(data) {
        let bytes = k.encode();
        let again =
            Ranked::decode(&bytes[..]).expect("re-decode of an accepted ranked key is canonical");
        assert_eq!(again, k, "accepted ranked key did not round-trip");
        assert_eq!(again.encode(), bytes, "ranked-key re-encode is not stable");
    }
    if let Ok(s) = Span::decode(data) {
        let bytes = s.encode();
        let again = Span::decode(&bytes[..]).expect("re-decode of an accepted span is canonical");
        assert_eq!(again, s, "accepted span did not round-trip");
        assert_eq!(again.encode(), bytes, "span re-encode is not stable");
    }
}
