//! Hostile text-parse fuzzing of the display notation.
//!
//! Feed arbitrary UTF-8 to every public `FromStr` — the paper-notation
//! parsers for `Party`, `Version`, and `Clock`, and the decimal parser
//! for `Ticks`. The contracts under test:
//!
//!   1. `parse` never panics on any input (it returns `Ok` or `Err`),
//!      including arbitrarily deep nesting — the parsers are iterative,
//!      never call-stack recursive.
//!   2. Any accepted value round-trips through its display: `Display` is
//!      the notation `FromStr` parses, and it emits the canonical
//!      spelling, so an accepted value's rendering parses back to the
//!      same value. Spelling is deliberately flexible on the way in
//!      (whitespace and leading zeros normalize; the text cursor and
//!      `parse_base` document both) — what the parsers strictly reject
//!      is non-normal-form *values*, and that structural strictness is
//!      the `FromStr` contract, not spelling strictness.
//!   3. No input's computation exceeds the harness heap cap
//!      (`before_fuzz::under_heap_cap`).

#![no_main]

use libfuzzer_sys::fuzz_target;

use before::{Clock, Party, Ticks, Version};

fuzz_target!(|data: &[u8]| {
    before_fuzz::under_heap_cap(|| run(data));
});

/// One input's body: parse every `FromStr` type and check the display round-trip.
fn run(data: &[u8]) {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    if let Ok(party) = text.parse::<Party>() {
        let again: Party = party
            .to_string()
            .parse()
            .expect("a displayed party re-parses");
        assert_eq!(again, party, "party display round-trip changed the value");
    }
    if let Ok(version) = text.parse::<Version>() {
        let again: Version = version
            .to_string()
            .parse()
            .expect("a displayed version re-parses");
        assert_eq!(
            again, version,
            "version display round-trip changed the value"
        );
    }
    if let Ok(clock) = text.parse::<Clock>() {
        let again: Clock = clock
            .to_string()
            .parse()
            .expect("a displayed clock re-parses");
        assert_eq!(
            again.encode(),
            clock.encode(),
            "clock display round-trip changed the value"
        );
    }
    if let Ok(ticks) = text.parse::<Ticks>() {
        let again: Ticks = ticks
            .to_string()
            .parse()
            .expect("a displayed tick count re-parses");
        assert_eq!(
            again, ticks,
            "tick-count display round-trip changed the value"
        );
    }
}
