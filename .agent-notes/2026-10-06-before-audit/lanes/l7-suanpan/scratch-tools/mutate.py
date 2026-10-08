#!/usr/bin/env python3
"""Apply or revert one exact-string mutant; refuse unless the match is unique."""
import sys
root = "/Users/oxide/src/rumors-audit-l7-suanpan/crates/suanpan/src/accumulator/"
MUTANTS = {
 "M1": ("digits/sign.rs", "const COMPARISON_DECIDED: i128 = 3;", "const COMPARISON_DECIDED: i128 = 2;"),
 "M2": ("digits/zero_ranges.rs", "                if hi <= from {", "                if hi <= to {"),
 "M3": ("digits/zero_ranges.rs", "    pub fn rebuild(&mut self, digits: &[i64]) {\n        self.clear();", "    pub fn rebuild(&mut self, digits: &[i64]) {\n        // mutant: no clear"),
 "M4": ("digits/read.rs", "let mut complement_carry = 1u64;", "let mut complement_carry = 0u64;"),
 "M5": ("conversions.rs", "if magnitude == 1u128 << 127 {", "if magnitude == 1u128 << 126 {"),
 "M6": ("../accumulator.rs", "let threshold = 3u128.checked_shl(threshold_bits)?;", "let threshold = 2u128.checked_shl(threshold_bits)?;"),
 "M7": ("digits/sign.rs", "index >= adjustment_high.saturating_add(2)", "index >= adjustment_high.saturating_add(1)"),
 "M8": ("digits.rs", "const RECENTER_BIAS: i128 = 1 << (DIGIT_BITS - 1);", "const RECENTER_BIAS: i128 = 0;"),
 "M9": ("digits/sign.rs", "            self.digits[index] = 0;\n            touch(1);\n            self.highest_nonzero = index;", "            touch(1);\n            self.highest_nonzero = index;"),
 "M12": ("digits/zero_ranges.rs", "Ranges::One(lo, hi) if *lo < above && *hi >= above => {", "Ranges::One(lo, hi) if *lo < above && *hi > above => {"),
 "M13": ("digits.rs", "                Some(lo) => lo,", "                Some(lo) => lo + 1,"),
}

MUTANTS.update({
 "M14": ("digits/zero_ranges.rs", "self.insert(lo, from);\n            }\n            if hi > to + 1 {\n                self.insert(to, hi);\n            }\n            return;", "self.insert(lo, from);\n            }\n            return;"),
 "M15": ("digits/zero_ranges.rs", "                if hi > to + 1 {\n                    ranges.insert(to, hi);\n                }", "                // mutant: upper remnant dropped"),
 "M16": ("digits/zero_ranges.rs", "Ranges::Empty;\n            if from > lo + 1 {\n                self.insert(lo, from);\n            }\n            if hi > to + 1 {\n                self.insert(to, hi);", "Ranges::Empty;\n            if hi > to + 1 {\n                self.insert(to, hi);"),
})

MUTANTS.update({
 "Z1": ("digits/zero_ranges.rs", "Ranges::One(lo, hi) if *lo < above && *hi >= above => {", "Ranges::One(lo, hi) if *lo <= above && *hi >= above => {"),
 "Z2": ("digits/zero_ranges.rs", "let (&lo, &hi) = ranges.range(..above).next_back()?;", "let (&lo, &hi) = ranges.range(..=above).next_back()?;"),
 "Z3": ("digits/zero_ranges.rs", "Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),", "Ranges::Many(_ranges) if true => Some(Ranges::Empty),"),
 "Z4": ("digits/zero_ranges.rs", "Ranges::Many(ranges) if ranges.len() == 1 => {", "Ranges::Many(ranges) if true => {"),
 "Z5": ("digits/zero_ranges.rs", "Ranges::Many(ranges) if ranges.len() == 1 => {", "Ranges::Many(ranges) if ranges.len() != 1 => {"),
})

MUTANTS.update({
 "N1": ("digits/normalize.rs", "self.digits[highest] = (polarity * remainder) as i64;", "self.digits[highest] = (polarity / remainder) as i64;"),
 "N2": ("digits/normalize.rs", "while highest > 0 && self.digits[highest] == 0 {", "while highest == 0 && self.digits[highest] == 0 {"),
 "N3": ("digits/normalize.rs", "            touch(1);\n            highest -= 1;", "            touch(1);\n            highest += 1;"),
})
MUTANTS.update({
 "WDEF": ("../../Cargo.toml", "touch-meter = []\n", "default = [\"l7-proto-w\"]\ntouch-meter = []\n"),
 "K1": ("digits/zero_ranges.rs", "        if let Ranges::Many(ranges) = &mut self.ranges {\n            while let Some((&lo, &hi)) = ranges.range(..to).next_back() {", "        if let Ranges::Many(ranges) = &mut self.ranges {\n            core::hint::black_box(ranges.range(..to).next_back());\n            while let Some((&lo, &hi)) = ranges.range(..to).next_back() {"),
 "L3S": ("operand.rs", "self.digits.deposit_value(operand_value, shift);", "self.digits.deposit_value(operand_value, shift as usize as u64);"),
 "L3D": ("operand.rs", ".add_digits(other.digits.stored_digits(), shift, update);", ".add_digits(other.digits.stored_digits(), shift as usize as u64, update);"),
 "L3W": ("digits/add.rs", "digit_index(u128::from(digit_shift) + offset as u128),", "(digit_shift as usize).wrapping_add(offset),"),
 "U1Fa": ("../accumulator.rs", "let adjustment_high = usize::try_from(adjustment_digits - 1).ok()?;", "let adjustment_high = adjustment_digits - 1;"),
 "U1Fb": ("digits/sign.rs", "pub fn cmp_zero_stable_above(&mut self, adjustment_high: usize) -> Option<Ordering> {", "pub fn cmp_zero_stable_above(&mut self, adjustment_high: u64) -> Option<Ordering> {"),
 "U1Fc": ("digits/sign.rs", "index >= adjustment_high.saturating_add(2))", "index as u64 >= adjustment_high.saturating_add(2))"),
})
MUTANTS.update({
 "L3H": ("operators.rs", "                self.start_digits();\n                self.digits.deposit_value(value, shift);", "                self.start_digits();\n                self.digits.deposit_value(value, shift as usize as u64);"),
})
name, mode = sys.argv[1], sys.argv[2]
path, a, b = MUTANTS[name]
path = root + path
s = open(path).read()
src, dst = (a, b) if mode == "apply" else (b, a)
n = s.count(src)
if n != 1:
    sys.exit(f"{name} {mode}: expected 1 match of source text, found {n}")
open(path, "w").write(s.replace(src, dst))
print(f"{name} {mode}d in {path}")
