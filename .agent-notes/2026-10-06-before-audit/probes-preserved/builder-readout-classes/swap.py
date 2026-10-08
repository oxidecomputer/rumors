#!/usr/bin/env python3
"""Apply or revert reversible string swaps to suanpan's readout (calibration only).

usage: swap.py <schema|negative-minus-one|push-two-twice> <apply|revert>
"""
import sys

PATH = "/Users/oxide/src/rumors-slot-25/crates/suanpan/src/accumulator/digits/read.rs"

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
SCHEMA_FN = '''
/// CALIBRATION MUTANT SCHEMA: perturb exactly the class READOUT_MUTANT names.
fn mutant_high(carry: i128, collected: &[u32], high: u32) -> u32 {
    let low_zero = collected.iter().all(|&digit| digit == 0);
    let class = format!("{carry}:{low_zero}");
    match std::env::var("READOUT_MUTANT").ok() {
        Some(spec) if spec == format!("touch:{class}") => {
            touch(1);
            high
        }
        Some(spec) if spec == format!("value:{class}") => high + 1,
        _ => high,
    }
}
'''
HOOK = "            let high = mutant_high(carry, &collected, high);\n"

SWAPS = {
    "schema": [
        (NEG_PUSH, HOOK + NEG_PUSH),
        (NONNEG_PUSH, HOOK + NONNEG_PUSH),
        ("\n/// Pack unsigned base-2^32 digits", SCHEMA_FN + "\n/// Pack unsigned base-2^32 digits"),
    ],
    "negative-minus-one": [
        ("-carry - i128::from(low_nonzero)", "-carry - 1"),
    ],
    "push-two-twice": [
        (NONNEG_PUSH, NONNEG_PUSH.replace("            let ordering", """            if high == 2 {
                touch(1);
                collected.push(high);
            }
            let ordering""")),
    ],
}


def main():
    name, mode = sys.argv[1], sys.argv[2]
    text = open(PATH).read()
    for old, new in SWAPS[name]:
        src, dst = (old, new) if mode == "apply" else (new, old)
        count = text.count(src)
        if count != 1:
            sys.exit(f"{name} {mode}: expected one match, found {count}: {src[:60]!r}")
        text = text.replace(src, dst)
    open(PATH, "w").write(text)
    print(f"{name} {mode}: ok")


main()
