#!/usr/bin/env python3
"""Add the enumerate-wrap landing case to the wasm32 guest and an explore test."""
root = "/Users/oxide/src/rumors-audit-l7-suanpan/crates/before/wasm32-pins/"
p = root + "guest/src/checks.rs"
s = open(p).read()
old = "        3 => accumulator.add_shifted_limbs(32 * max, [1]),\n"
new = old + (
    "        // L7 explore: the limb index past usize::MAX limbs. Indices 0..=2^32-1\n"
    "        // carry zero limbs; the `1` sits at index 2^32, digit position 2^33.\n"
    "        4 => accumulator.add_shifted_limbs(0, core::iter::repeat_n(0u64, usize::MAX).chain([0, 1])),\n"
    "        // L7 explore control: the same `1` at index 2^31 lands at 2^32 and must panic.\n"
    "        5 => accumulator.add_shifted_limbs(0, core::iter::repeat_n(0u64, 1usize << 31).chain([1])),\n"
)
assert s.count(old) == 1
s = s.replace(old, new)
open(p, "w").write(s)

p = root + "harness/tests/pins.rs"
s = open(p).read()
s += '''
/// L7 explore: a limb stream longer than `usize::MAX` must not wrap its
/// landing position (case 4); case 5 is the in-range control.
#[test]
fn zz_l7_suanpan_limb_index_past_usize_max() {
    let control = run(Check::SuanpanLanding, 5, 0);
    let wrapped = run(Check::SuanpanLanding, 4, 0);
    eprintln!("L7 case5 (index 2^31) outcome: {control:?}");
    eprintln!("L7 case4 (index 2^32) outcome: {wrapped:?}");
    assert_eq!(control, Outcome::Trapped(Trap::UnreachableCodeReached));
    assert_eq!(wrapped, Outcome::Trapped(Trap::UnreachableCodeReached), "a limb past 2^32 wrapped instead of panicking");
}
'''
open(p, "w").write(s)
print("patched")
