#!/usr/bin/env python3
"""Add L7 fuel probes to the wasm32-pins protocol, guest, and harness (explore only)."""
import sys
S = sys.argv[1]
root = "/Users/oxide/src/rumors-audit-l7-suanpan/crates/before/wasm32-pins/"

def edit(path, old, new, count=1):
    s = open(path).read()
    n = s.count(old)
    assert n == count, (path, old[:50], n)
    open(path, "w").write(s.replace(old, new))

# Protocol: one new check.
p = root + "protocol/src/lib.rs"
edit(p, "    SuanpanLanding = 8,\n}", "    SuanpanLanding = 8,\n    /// L7 explore: a fuel probe selected by `a`, at size `b`.\n    L7Fuel = 9,\n}")
edit(p, "            8 => Ok(Self::SuanpanLanding),\n", "            8 => Ok(Self::SuanpanLanding),\n            9 => Ok(Self::L7Fuel),\n")

# Guest: dispatch and probe bodies.
p = root + "guest/src/checks.rs"
edit(p, "        Check::SuanpanLanding => suanpan_landing(a),\n", "        Check::SuanpanLanding => suanpan_landing(a),\n        Check::L7Fuel => l7_fuel(a, b),\n")
s = open(p).read() + open(S + "/guest_fuel.rs").read()
open(p, "w").write(s)


# Harness: fuel-metered runs.
p = root + "harness/Cargo.toml"
s = open(p).read()
assert '"runtime",' in s
p2 = root + "harness/src/lib.rs"
s = open(p2).read()
s += '''
/// L7 explore: run a check under fuel metering, returning the outcome and the
/// fuel consumed. A separate engine is used because fuel instrumentation is
/// fixed when the module compiles.
pub fn run_fueled(check: Check, a: u64, b: u64) -> (Outcome, u64) {
    static SHARED: OnceLock<(Engine, Module)> = OnceLock::new();
    let (engine, module) = SHARED.get_or_init(|| {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).expect("a fuel-metering engine builds");
        let module = Module::from_file(&engine, guest_wasm_path()).expect("the guest loads");
        (engine, module)
    });
    let mut store = Store::new(engine, ());
    store.set_fuel(u64::MAX).expect("fuel is enabled");
    let instance = Instance::new(&mut store, module, &[]).expect("the guest instantiates");
    let function = instance
        .get_typed_func::<(u32, u64, u64), i32>(&mut store, "check")
        .expect("the guest exports the check protocol");
    let outcome = match function.call(&mut store, (check as u32, a, b)) {
        Ok(wasm32_pins_protocol::PASS) => Outcome::Passed,
        Ok(code) => Outcome::Failed(Failure::from_code(code).expect("known failure code")),
        Err(error) => Outcome::Trapped(*error.downcast_ref::<Trap>().expect("a trap")),
    };
    let used = u64::MAX - store.get_fuel().expect("fuel is enabled");
    (outcome, used)
}
'''
open(p2, "w").write(s)

p = root + "harness/tests/pins.rs"
s = open(p).read()
s += '''
/// L7 explore: fuel per operation as the sparse-range count grows.
///
/// Investigative; prints readings and asserts only that each probe passes.
#[test]
#[ignore]
fn zz_l7_fuel_readings() {
    use wasm32_pins_harness::run_fueled;
    let sizes: Vec<u64> = std::env::var("L7_SIZES")
        .unwrap_or_else(|_| "64,256,1024,4096,16384".into())
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    for &r in &sizes {
        let (o0, f0) = run_fueled(Check::L7Fuel, 0, r);
        let (o1, f1) = run_fueled(Check::L7Fuel, 1, r);
        assert_eq!((o0, o1), (Outcome::Passed, Outcome::Passed));
        let per_update = (f1 - f0) as f64 / f64::from(1u32 << 17);
        eprintln!("L7FUEL suanpan ranges {r}: {per_update:.2} fuel per word update");
    }
    for &r in &sizes {
        let (o2, f2) = run_fueled(Check::L7Fuel, 2, r);
        let (o3, f3) = run_fueled(Check::L7Fuel, 3, r);
        let (o4, f4) = run_fueled(Check::L7Fuel, 4, r);
        assert_eq!((o2, o3, o4), (Outcome::Passed, Outcome::Passed, Outcome::Passed));
        let leaves = (64 * r) as f64;
        eprintln!(
            "L7FUEL version ranges {r} leaves {leaves}: decode {:.2}/leaf, compare {:.2}/leaf",
            (f3 - f2) as f64 / leaves,
            (f4 - f2) as f64 / leaves
        );
    }
}
'''
open(p, "w").write(s)
print("patched")
