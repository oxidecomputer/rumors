#!/usr/bin/env bash
# Applies the grown-input probe (never committed): VersionCompare with a == 0
# compares A = node(leaf(H), leaf(H + 1)) against B = leaf(H), H = 2^w - 1,
# w = 2^31, so A's right leaf flag sits at bit 2^32 + 3 and decides A > B.
# Also traps if any reader opens at a position past 2^32 (site 1).
set -euo pipefail
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-wasm32-compare-pin-reach
W=/Users/oxide/src/rumors-slot-42/crates/before
DIR="${1:-apply}"
swap() { if [ "$DIR" = apply ]; then python3 -I $S/swap.py "$1" "$2" "$3"; else python3 -I $S/swap.py "$1" "$3" "$2"; fi; }
swap $W/wasm32-pins/guest/src/checks.rs \
'fn version_compare(size: u64) -> Result<(), Failure> {
    let large' \
'fn version_compare(size: u64) -> Result<(), Failure> {
    if size == 0 {
        let (a, b) = synthesis::probe_pair()?;
        let a = decoded_version(&a)?;
        let b = decoded_version(&b)?;
        if a.encoded_bits() != (1u64 << 32) + 7 || b.encoded_bits() != (1u64 << 32) + 2 {
            return Err(Failure::WrongLength);
        }
        if a.partial_cmp(&b) != Some(Ordering::Greater)
            || b.partial_cmp(&a) != Some(Ordering::Less)
        {
            return Err(Failure::WrongValue);
        }
        return Ok(());
    }
    let large'
swap $W/wasm32-pins/guest/src/synthesis.rs \
'/// Encodes `(2^(exp-65) + 1) / 2^exp` in canonical `Rank` form.' \
'/// Probe: `node(leaf(2^w - 1), leaf(2^w))` and `leaf(2^w - 1)`, w = 2^31.
pub fn probe_pair() -> Result<(Vec<u8>, Vec<u8>), Failure> {
    let w: u64 = 1 << 31;
    let a_live = 2 * w + 7;
    let mut a = Stream::zeroed((a_live + 1).div_ceil(8))?;
    a.set(1)?; // left leaf after the root branch flag
    a.set(w + 2)?; // left height mantissa lead
    a.set(2 * w + 3)?; // right leaf flag, past 2^32
    a.set(2 * w + 5)?; // delta magnitude 1
    a.set(2 * w + 6)?; // positive sign
    a.set(a_live)?; // marker
    let b_live = 2 * w + 2;
    let mut b = Stream::zeroed((b_live + 1).div_ceil(8))?;
    b.set(0)?;
    b.set(w + 1)?;
    b.set(b_live)?;
    Ok((a.finish(), b.finish()))
}

/// Encodes `(2^(exp-65) + 1) / 2^exp` in canonical `Rank` form.'
swap $W/wasm32-pins/harness/tests/pins.rs \
'/// Join constructs the closest' \
'/// Probe only.
#[test]
fn version_compare_probe_grown() {
    assert_passes(Check::VersionCompare, 0, 0);
}

/// Join constructs the closest'
swap $W/src/bits/reader.rs \
'        assert!(position <= len, "reader opened past the stream'"'"'s end");' \
'        assert!(position <= len, "reader opened past the stream'"'"'s end");
        assert!(position < 1 << 32, "probe: reader opened at or past 2^32");'
