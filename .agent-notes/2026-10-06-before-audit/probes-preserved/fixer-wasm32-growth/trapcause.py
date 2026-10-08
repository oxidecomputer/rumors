import sys
root='/Users/oxide/src/rumors-slot-14/crates/before/wasm32-pins/'
lib_a="""    match checks::run(check, a, b) {"""
lib_b="""    // SCRATCH: divert capacity-overflow panics into a stack overflow trap.
    std::panic::set_hook(Box::new(|info| {
        if info.to_string().contains("capacity overflow") {
            scratch_exhaust(0);
        }
    }));
    match checks::run(check, a, b) {"""
lib_tail="""
// SCRATCH-BEGIN
fn scratch_exhaust(depth: u64) -> u64 {
    let local = core::hint::black_box([depth; 16]);
    let r = scratch_exhaust(local[0] + 1);
    core::hint::black_box(&local);
    r
}
"""
pins_tail="""
// SCRATCH-BEGIN
#[test]
fn scratch_cause_rank_past() { eprintln!("CAUSE rank past: {:?}", run(Check::RankDecodeGroups, DOUBLING_LIMIT + 1, 0)); }
#[test]
fn scratch_cause_borsh_past() { eprintln!("CAUSE borsh past: {:?}", run(Check::VersionBorshWideLeaf, WIDE_LEAF_PAST_LIMIT, 0)); }
#[test]
fn scratch_cause_other_panic() { eprintln!("CAUSE deliberate panic: {:?}", run(Check::HarnessTrap, 0, 0)); }
"""
rm='--remove' in sys.argv
p=root+'guest/src/lib.rs'; s=open(p).read()
if rm:
    s=s.replace(lib_b,lib_a); s=s[:s.index('\n// SCRATCH-BEGIN\n')]+'\n'; s=s.rstrip('\n')+'\n'
else:
    assert s.count(lib_a)==1; s=s.replace(lib_a,lib_b)+lib_tail
open(p,'w').write(s)
p=root+'harness/tests/pins.rs'; s=open(p).read()
if rm:
    s=s[:s.index('\n// SCRATCH-BEGIN\n')].rstrip('\n')+'\n'
else:
    s=s+pins_tail
open(p,'w').write(s)
print('removed' if rm else 'applied')
