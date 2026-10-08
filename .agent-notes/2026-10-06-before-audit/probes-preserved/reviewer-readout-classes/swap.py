#!/usr/bin/env python3
"""Reviewer's reversible string swaps on suanpan's readout (calibration only).

usage: swap.py <name> <apply|revert>

`schema` installs env-selected mutants (REVIEW_MUTANT) in read.rs:
  touch:<c>:<lowzero>, value:<c>:<lowzero>  (the builder's schema, reproduced)
  low-first   zero-low predicate reads only M's lowest digit
  low-last    zero-low predicate reads only M's top digit
  carry-once  complement carry is not propagated past digit 0
  parity-touch high-part touch charged only when the span length is even
`widths` changes the table's widths (the candidate repair).
"""
import sys

READ = "/Users/oxide/src/rumors-slot-25/crates/suanpan/src/accumulator/digits/read.rs"
METERED = "/Users/oxide/src/rumors-slot-25/crates/suanpan/src/accumulator/tests/metered.rs"

NEG_PUSH = """            if high > 0 {
                touch(1);
                collected.push(high);
            }
            // |carry|"""
NONNEG_PUSH = """            if high > 0 {
                touch(1);
                collected.push(high);
            }
            let ordering"""


def hooked(push):
    return push.replace(
        "            if high > 0 {\n                touch(1);\n",
        "            let high = review_high(carry, &collected, high);\n"
        "            if high > 0 {\n"
        "                if !(review_mutant(\"parity-touch\") && collected.len() % 2 == 1) {\n"
        "                    touch(1);\n"
        "                }\n",
    )


SCHEMA_FN = '''
/// REVIEW CALIBRATION SCHEMA: whether REVIEW_MUTANT names `name`.
fn review_mutant(name: &str) -> bool {
    std::env::var("REVIEW_MUTANT").is_ok_and(|spec| spec == name)
}

/// REVIEW CALIBRATION SCHEMA: perturb exactly the class REVIEW_MUTANT names.
fn review_high(carry: i128, collected: &[u32], high: u32) -> u32 {
    let low_zero = collected.iter().all(|&digit| digit == 0);
    let class = format!("{carry}:{low_zero}");
    if review_mutant(&format!("touch:{class}")) {
        touch(1);
        high
    } else if review_mutant(&format!("value:{class}")) {
        high + 1
    } else {
        high
    }
}
'''

LOW = "            let low_nonzero = collected.iter().any(|&digit| digit != 0);\n"
LOW_M = """            let low_nonzero = if review_mutant("low-first") {
                collected[0] != 0
            } else if review_mutant("low-last") {
                collected.last().is_some_and(|&digit| digit != 0)
            } else {
                collected.iter().any(|&digit| digit != 0)
            };
"""
CARRY = "                    complement_carry = complemented >> DIGIT_BITS;\n"
CARRY_M = """                    complement_carry = if review_mutant("carry-once") {
                        0
                    } else {
                        complemented >> DIGIT_BITS
                    };
"""

SWAPS = {
    "schema": (READ, [
        (NEG_PUSH, hooked(NEG_PUSH)),
        (NONNEG_PUSH, hooked(NONNEG_PUSH)),
        (LOW, LOW_M),
        (CARRY, CARRY_M),
        ("\n/// Pack unsigned base-2^32 digits", SCHEMA_FN + "\n/// Pack unsigned base-2^32 digits"),
    ]),
    "widths": (METERED, [
        ("    for digits in [64u32, 128] {\n        let held_digits = u64::from(digits);\n        let power",
         "    for digits in [64u32, 129] {\n        let held_digits = u64::from(digits);\n        let power"),
    ]),
    "prose": (METERED, [
        ("""/// Every row runs at 2,048 and 4,096 bits, through the unscaled and the scaled
/// readout; digit zero is written, so both read the same `d` digits. Exact
/// equality in every class leaves no slack for a missing or repeated touch, or
/// a wrong high part, confined to that class.
""",
         """/// Every row runs at an even and an odd digit count, through the unscaled and
/// the scaled readout; digit zero is written, so both read the same `d`
/// digits. Two widths separate the per-digit cost from the constant one, and
/// their opposite parities expose a cost that depends on pairing digits into
/// 64-bit limbs. Exact equality in every class leaves no slack for a missing
/// or repeated touch, or a wrong high part, confined to that class.
"""),
        ("""            // Digits 0, −(B − 1), …, −(B − 1): the low part `B` has a zero
            // lowest digit.
""",
         """            // Digits 0, −(B − 1), …, −(B − 1): the low part `B` has a zero
            // lowest digit, so deciding that it is nonzero, and complementing
            // it, must look past digit 0.
"""),
    ]),
    "row": (METERED, [
        ("""            // Every digit −(B − 1).
            build: |d| -added(&[all_ones(d)]),
            value: |d| -IBig::from(all_ones(d)),
""",
         """            // Digits 0, −(B − 1), …, −(B − 1): the low part `B` has a zero
            // lowest digit.
            build: |d| {
                let mut acc = added(&[all_ones(d)]);
                acc -= u64::from(u32::MAX);
                -acc
            },
            value: |d| -IBig::from(all_ones(d) - u32::MAX),
"""),
    ]),
}


def main():
    name, mode = sys.argv[1], sys.argv[2]
    path, swaps = SWAPS[name]
    text = open(path).read()
    for old, new in swaps:
        src, dst = (old, new) if mode == "apply" else (new, old)
        count = text.count(src)
        if count != 1:
            sys.exit(f"{name} {mode}: expected one match, found {count}: {src[:70]!r}")
        text = text.replace(src, dst)
    open(path, "w").write(text)
    print(f"{name} {mode}: ok")


main()
