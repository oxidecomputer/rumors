"""Apply calibration batch 2 (regime-targeted mutations) to the L3 explore tree."""
import sys
ROOT = '/Users/oxide/src/rumors-audit-l3-events/crates/before/src/'

def sub(path, old, new):
    p = ROOT + path
    s = open(p).read()
    n = s.count(old)
    assert n == 1, (path, old, n)
    open(p, 'w').write(s.replace(old, new))

# The switch itself.
sub('lib.rs', "pub(crate) mod accumulator;\n", '''pub(crate) mod accumulator;

/// Explore-branch calibration switch: whether mutation `k` is enabled.
pub(crate) fn l3_mut(k: u32) -> bool {
    #[cfg(test)]
    {
        static M: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
        *M.get_or_init(|| {
            std::env::var("L3_MUT")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0)
        }) == k
    }
    #[cfg(not(test))]
    {
        let _ = k;
        false
    }
}
''')
# M15: begin_scan clears only the first used block.
sub('version/tick/memo.rs',
    "        for block in &mut self.blocks[..self.len.div_ceil(BLOCK_SLOTS)] {",
    "        let used = if crate::l3_mut(15) { self.len.div_ceil(BLOCK_SLOTS).min(1) } else { self.len.div_ceil(BLOCK_SLOTS) };\n        for block in &mut self.blocks[..used] {")
# M21: begin_scan forgets to reset the consumption cursor.
sub('version/tick/memo.rs',
    "        self.len = 0;\n        self.cursor = 0;",
    "        self.len = 0;\n        if !crate::l3_mut(21) {\n            self.cursor = 0;\n        }")
# M19: the block for slots 64..127 is allocated one reservation late.
sub('version/tick/memo.rs',
    "        if slot / BLOCK_SLOTS == self.blocks.len() {",
    "        if slot / BLOCK_SLOTS >= self.blocks.len() && !(crate::l3_mut(19) && slot == BLOCK_SLOTS) {")
# M16: resolve_inner restores the reference level by decrementing it.
sub('version/tick/prescan.rs',
    "        self.reference_level = outer.level;",
    "        self.reference_level = if crate::l3_mut(16) { self.reference_level - 1 } else { outer.level };")
print("batch 2 applied")
# M22 / M23: report one more / one less than the computed floor (definition-level errors
# that bypass the settle() debug assertion, to calibrate the floor and tightness probes).
sub('version/measure/min_ticks.rs',
    "        Count(magnitude)\n    }",
    "        if crate::l3_mut(22) && magnitude != num_bigint::BigUint::ZERO {\n            return Count(magnitude + 1u8);\n        }\n        if crate::l3_mut(23) && magnitude != num_bigint::BigUint::ZERO {\n            return Count(magnitude - 1u8);\n        }\n        Count(magnitude)\n    }")
print("batch 2 extended")
