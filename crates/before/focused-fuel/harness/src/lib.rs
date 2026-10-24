//! Judges `before`'s focused cost claims in Wasmtime fuel.
//!
//! The amplification board scales each committed family's operands and
//! judges every operation's growth against them. Some cost claims cannot be
//! stated that way: that an operation's cost does not depend on a parameter
//! (a clone fast path at any operand size, an early exit at any tail length),
//! that one entry point costs another plus a fixed amount, that a fused walk
//! costs less than the composition it replaces, or that a cost grows along an
//! axis no board family scales. This crate states each such claim as a
//! relation between fuel readings of the same guest, so no fuel number is
//! committed anywhere, and a toolchain or Wasmtime change cannot move a
//! claim's verdict unless it changes how the measured operations compare.
//!
//! [`measure`] runs one [`Probe`] in a fresh instance of the compiled guest
//! (`focused-fuel-guest`), under the fuel board's operator prices, and
//! returns the fuel its measured window consumed. The guest checks every
//! operation's answer outside that window: against an independent value
//! where the probe has one (a documented relation, a closed form, or the
//! answer of a different path), and otherwise against the same operation's
//! public answer taken before the window, which shows only that the measured
//! call agrees with it. Each probe's doc says which. The suites under
//! `tests/` state the claims; `just focused-fuel` builds the guest and runs
//! them.
//!
//! # Forms
//!
//! Every claim takes one of these forms, which its test's doc comment names
//! by the label in parentheses:
//!
//! - *Equal fuel* (F-eq): the readings are identical at every value of a
//!   parameter the operation's cost must not depend on ([`assert_equal`]).
//! - *Equal difference* (F-diff): one operation's reading minus another's is
//!   identical at every size, so the first costs the second plus an amount
//!   independent of size ([`assert_equal_difference`]).
//! - *Strict saving* (F-lt): one operation reads strictly less than another
//!   on the same operands ([`assert_less`]), and where the claim is that the
//!   saved work scales with the operands, the saving grows with them
//!   ([`assert_saving_grows`]).
//! - *Flat per unit* (F-lin): fuel per unit of work at a larger size is at
//!   most 1.25 times fuel per unit at the smaller size ([`assert_flat`]). A
//!   claim whose own band differs states it, with its rationale, beside the
//!   test ([`assert_flat_within`]).
//! - *No product term* (F-mix): over a two-by-two grid of two parameters,
//!   the mixed second difference is at most a tenth of the top cell, which
//!   any cost additive in the parameters meets and a product of them does not
//!   ([`assert_no_product`]). Where a product would hide under that tenth,
//!   the claim derives a bound on the mixed difference from the mechanism
//!   instead and states it beside the test ([`assert_mixed_below`]).
//! - *Growth floor* (F-grow): fuel grows by at least a stated factor across a
//!   stated growth of the input ([`assert_grows`]).
//! - *Different paths* (F-ne): two readings differ, showing that two calls
//!   take different paths, where the direction of the difference is not part
//!   of the claim ([`assert_differ`]).
//! - *Derived bound* (F-bound): a reading stays within a bound derived from
//!   the operation's mechanism, stated beside the test; where the bound is
//!   another operation's reading, [`assert_at_most`] checks it.
//!
//! Every checker prints its readings on a `MEASURED` line, so a failure and a
//! thin margin can be read from the test's output.

use std::path::PathBuf;
use std::sync::OnceLock;

use board_fuel_harness::costs::operator_cost;
pub use focused_fuel_protocol::Probe;
use focused_fuel_protocol::{MEASURED, UNKNOWN_PROBE};
use wasmtime::{Caller, Config, Engine, Instance, Linker, Module, Store};

/// The environment variable naming the compiled guest module; the
/// `focused-fuel` recipe sets it.
pub const GUEST_ENV: &str = "FOCUSED_FUEL_GUEST_WASM";

/// Fuel loaded into each store: far above any probe's consumption, so the
/// counter never runs dry and `FUEL_TANK` minus the remaining fuel is the
/// instance's consumption so far.
const FUEL_TANK: u64 = u64::MAX / 2;

/// The flat-per-unit bound: fuel per unit may grow by at most this numerator
/// over [`FLAT_DEN`] across a doubling, the ×1.25 convention of `before`'s
/// focused resource checks.
const FLAT_NUM: u128 = 5;

/// The flat-per-unit bound's denominator; see [`FLAT_NUM`].
const FLAT_DEN: u128 = 4;

/// The engine and the compiled guest, shared by every measurement in the
/// process.
struct Compiled {
    /// The fuel-metering engine.
    engine: Engine,
    /// The guest module.
    module: Module,
}

/// Returns the process's engine and compiled guest, compiling on first use.
///
/// # Panics
///
/// Panics if [`GUEST_ENV`] names no readable module or the module does not
/// compile.
fn compiled() -> &'static Compiled {
    static COMPILED: OnceLock<Compiled> = OnceLock::new();
    COMPILED.get_or_init(|| {
        let path = std::env::var_os(GUEST_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                panic!("{GUEST_ENV} must name the compiled guest; run `just focused-fuel`")
            });
        let wasm = std::fs::read(&path).unwrap_or_else(|error| panic!("reading {path:?}: {error}"));
        let mut config = Config::new();
        config.consume_fuel(true);
        config.operator_cost(operator_cost());
        let engine = Engine::new(&config).expect("a fuel-metering engine builds");
        let module = Module::new(&engine, wasm).expect("the guest module compiles");
        Compiled { engine, module }
    })
}

/// Measures `probe` with parameters `a` and `b` in a fresh guest instance and
/// returns the fuel its measured window consumed.
///
/// A fresh instance keeps every state an earlier measurement left, such as
/// grown linear memory or a lazily initialized static, out of the reading.
///
/// # Panics
///
/// Panics if the guest traps, naming the panic message it recorded: a wrong
/// answer from the measured operation, or parameters the probe cannot use.
/// Also panics if the guest does not know the probe.
pub fn measure(probe: Probe, a: u64, b: u64) -> u64 {
    run(probe, a, b).0
}

/// Measures `probe` as [`measure`] does and returns its fuel with the units
/// of work the probe states, such as its operands' encoded bytes.
///
/// # Panics
///
/// Panics as [`measure`] does, or if the probe states no units.
pub fn measure_per(probe: Probe, a: u64, b: u64) -> (u64, u64) {
    let (fuel, units) = run(probe, a, b);
    assert!(units > 0, "{probe:?} states no units of work");
    (fuel, units)
}

/// Runs `probe` in a fresh guest instance and returns its fuel and the units
/// it states, zero if none.
fn run(probe: Probe, a: u64, b: u64) -> (u64, u64) {
    let Compiled { engine, module } = compiled();
    let mut store = Store::new(engine, ());
    store.set_fuel(FUEL_TANK).expect("fuel is enabled");
    let mut linker = Linker::new(engine);
    linker
        .func_wrap(
            "focused_fuel",
            "fuel_used",
            |caller: Caller<'_, ()>| -> wasmtime::Result<u64> {
                Ok(FUEL_TANK - caller.get_fuel()?)
            },
        )
        .expect("the fuel import is defined once");
    let instance = linker
        .instantiate(&mut store, module)
        .expect("the guest instantiates");
    let status = instance
        .get_typed_func::<(u32, u64, u64), u32>(&mut store, "measure")
        .expect("the guest exports `measure`")
        .call(&mut store, (u32::from(probe), a, b));
    match status {
        Ok(MEASURED) => (
            call(&mut store, &instance, "reading"),
            call(&mut store, &instance, "units"),
        ),
        Ok(UNKNOWN_PROBE) => panic!("the guest does not know {probe:?}; rebuild it"),
        Ok(status) => panic!("the guest returned unknown status {status} for {probe:?}"),
        Err(trap) => panic!(
            "{probe:?}({a}, {b}) trapped ({trap}) and {}",
            panic_message(&mut store, &instance)
        ),
    }
}

/// Measures `probe` at each value of `a`, with `b` zero.
pub fn measure_at(probe: Probe, sizes: &[u64]) -> Vec<u64> {
    sizes.iter().map(|&a| measure(probe, a, 0)).collect()
}

/// Asserts the *equal fuel* form: `probe` reads the same fuel at every value
/// of `a` in `sizes`, with `b` zero, and returns that reading.
///
/// # Panics
///
/// Panics if two readings differ, naming every size and its reading.
pub fn assert_equal(probe: Probe, sizes: &[u64]) -> u64 {
    let readings = measure_at(probe, sizes);
    let pairs: Vec<(u64, u64)> = sizes
        .iter()
        .copied()
        .zip(readings.iter().copied())
        .collect();
    eprintln!("MEASURED {probe:?}: (size, fuel) {pairs:?}");
    assert!(
        readings.windows(2).all(|pair| pair[0] == pair[1]),
        "{probe:?} must cost the same at every size, but read (size, fuel) {pairs:?}"
    );
    readings[0]
}

/// Asserts the *equal difference* form: `probe` minus `reference`, each a
/// probe with its parameter `b`, both at the same `a`, is the same at every
/// value of `a` in `sizes`, and returns that difference, which may be
/// negative.
///
/// # Panics
///
/// Panics if two differences differ.
pub fn assert_equal_difference(
    probe: (Probe, u64),
    reference: (Probe, u64),
    sizes: &[u64],
) -> i128 {
    let ((probe, b), (reference, reference_b)) = (probe, reference);
    let rows: Vec<(u64, u64, u64)> = sizes
        .iter()
        .map(|&a| (a, measure(probe, a, b), measure(reference, a, reference_b)))
        .collect();
    eprintln!(
        "MEASURED {probe:?}({b:#x}) - {reference:?}({reference_b:#x}): (size, probe, reference) \
         {rows:?}"
    );
    let differences: Vec<i128> = rows
        .iter()
        .map(|&(_, measured, base)| i128::from(measured) - i128::from(base))
        .collect();
    assert!(
        differences.windows(2).all(|pair| pair[0] == pair[1]),
        "{probe:?} must cost {reference:?} plus a fixed amount, but the differences at \
         sizes {sizes:?} are {differences:?}"
    );
    differences[0]
}

/// Asserts the *strict saving* form: `smaller` is strictly less than
/// `larger`, with `name` describing the comparison.
///
/// # Panics
///
/// Panics unless `smaller < larger`.
pub fn assert_less(name: &str, smaller: u64, larger: u64) {
    eprintln!(
        "MEASURED {name}: {smaller} < {larger} (ratio {:.3})",
        smaller as f64 / larger as f64
    );
    assert!(
        smaller < larger,
        "{name}: {smaller} must be strictly less than {larger}"
    );
}

/// Asserts the *derived bound* form for a bound that is another reading:
/// `reading` is at most `bound`, with `name` describing the comparison.
///
/// # Panics
///
/// Panics unless `reading <= bound`.
pub fn assert_at_most(name: &str, reading: u64, bound: u64) {
    eprintln!(
        "MEASURED {name}: {reading} <= {bound} (ratio {:.3})",
        reading as f64 / bound as f64
    );
    assert!(
        reading <= bound,
        "{name}: {reading} must be at most {bound}"
    );
}

/// Asserts the *strict saving* form across sizes: at every size, the first
/// reading of the row is strictly less than the second, and the saving, the
/// second minus the first, grows strictly from each size to the next.
///
/// Each row is `(size, smaller, larger)`, in increasing size. A growing
/// saving shows that the work saved scales with the operands, as a stream
/// decoded once instead of twice does, rather than being a fixed cost.
///
/// # Panics
///
/// Panics if either bound fails.
pub fn assert_saving_grows(name: &str, rows: &[(u64, u64, u64)]) {
    let mut previous: Option<(u64, u64)> = None;
    for &(size, smaller, larger) in rows {
        assert_less(&format!("{name} at size {size}"), smaller, larger);
        let saving = larger - smaller;
        if let Some((earlier, earlier_saving)) = previous {
            assert!(
                earlier_saving < saving,
                "{name}: the saving must grow with the operands, but it read \
                 {earlier_saving} at size {earlier} and {saving} at size {size}"
            );
        }
        previous = Some((size, saving));
    }
}

/// Asserts the *flat per unit* form: `large`'s fuel per unit is at most 1.25
/// times `small`'s, where each argument is `(fuel, units)`.
///
/// # Panics
///
/// Panics if the bound fails or either unit count is zero.
pub fn assert_flat(name: &str, small: (u64, u64), large: (u64, u64)) {
    assert_flat_within(name, small, large, (FLAT_NUM, FLAT_DEN));
}

/// Asserts the *flat per unit* form within `band`, a ratio `(numerator,
/// denominator)`: `large`'s fuel per unit is at most that ratio times
/// `small`'s, where each argument is `(fuel, units)`.
///
/// For a claim whose own band differs from the ×1.25 convention.
///
/// # Panics
///
/// Panics if the bound fails or either unit count is zero.
pub fn assert_flat_within(name: &str, small: (u64, u64), large: (u64, u64), band: (u128, u128)) {
    let ((fuel1, units1), (fuel2, units2)) = (small, large);
    let (numerator, denominator) = band;
    assert!(units1 > 0 && units2 > 0, "{name}: unit counts are positive");
    eprintln!(
        "MEASURED {name}: small={fuel1}/{units1} large={fuel2}/{units2} growth per unit {:.3} \
         (band {:.3})",
        (fuel2 as f64 / units2 as f64) / (fuel1 as f64 / units1 as f64),
        numerator as f64 / denominator as f64,
    );
    assert!(
        u128::from(fuel2) * u128::from(units1) * denominator
            <= u128::from(fuel1) * u128::from(units2) * numerator,
        "{name}: fuel per unit grew past the band: {fuel1}/{units1} -> {fuel2}/{units2}"
    );
}

/// Asserts the *no product term* form over `grid`, the readings at
/// `(x, y)`, `(2x, y)`, `(x, 2y)`, and `(2x, 2y)` in that order: the mixed
/// second difference is at most a tenth of the top cell.
///
/// A cost `p·x + q·y + r` has a mixed second difference of zero, and a cost
/// `s·x·y` one of a quarter of the top cell, so the bound tells them apart
/// with room for amortized costs that wobble between cells.
///
/// # Panics
///
/// Panics if the bound fails.
pub fn assert_no_product(name: &str, grid: [u64; 4]) {
    let [base, x2, y2, both] = grid.map(i128::from);
    let mixed = both - x2 - y2 + base;
    eprintln!("MEASURED {name}: grid {grid:?} mixed {mixed}");
    assert!(
        mixed.abs() * 10 <= both,
        "{name}: the mixed second difference {mixed} exceeds a tenth of the top cell \
         {both}: the cost has a product term"
    );
}

/// Asserts the *no product term* form with a derived bound: the mixed second
/// difference over `grid` is below `bound`.
///
/// `grid` holds the readings at `(x, y)`, `(2x, y)`, `(x, y')`, and
/// `(2x, y')` in that order, and the claim derives `bound` from the smallest
/// product term it rules out.
///
/// # Panics
///
/// Panics if the mixed difference reaches `bound`.
pub fn assert_mixed_below(name: &str, grid: [u64; 4], bound: i128) {
    let [base, x2, y2, both] = grid.map(i128::from);
    let mixed = both - x2 - y2 + base;
    eprintln!("MEASURED {name}: grid {grid:?} mixed {mixed} bound {bound}");
    assert!(
        mixed < bound,
        "{name}: the mixed second difference {mixed} reaches the bound {bound}: the cost has a \
         product term"
    );
}

/// Asserts the *different paths* form: `first` and `second` differ.
///
/// # Panics
///
/// Panics if the two readings are equal.
pub fn assert_differ(name: &str, first: u64, second: u64) {
    eprintln!("MEASURED {name}: {first} vs {second}");
    assert_ne!(
        first, second,
        "{name}: the two calls read the same fuel, so they take one path"
    );
}

/// Asserts the *growth floor* form: `large` is at least `factor` times
/// `small`.
///
/// # Panics
///
/// Panics if the bound fails.
pub fn assert_grows(name: &str, small: u64, large: u64, factor: f64) {
    let growth = large as f64 / small.max(1) as f64;
    eprintln!("MEASURED {name}: {small} -> {large} (x{growth:.3}, floor x{factor})");
    assert!(
        growth >= factor,
        "{name}: fuel grew only x{growth:.3} ({small} -> {large}), below x{factor}"
    );
}

/// Calls the guest's nullary export `name`.
fn call<R: wasmtime::WasmResults>(store: &mut Store<()>, instance: &Instance, name: &str) -> R {
    instance
        .get_typed_func::<(), R>(&mut *store, name)
        .and_then(|func| func.call(&mut *store, ()))
        .unwrap_or_else(|error| panic!("calling the guest's `{name}`: {error}"))
}

/// Describes what the guest's panic record holds after a trap.
fn panic_message(store: &mut Store<()>, instance: &Instance) -> String {
    if call::<u32>(store, instance, "panic_recorded") == 0 {
        return "recorded no panic, so it likely ran out of memory".to_owned();
    }
    let address = call::<u32>(store, instance, "panic_message_ptr");
    let length = call::<u32>(store, instance, "panic_message_len");
    let mut bytes = vec![0u8; length as usize];
    instance
        .get_memory(&mut *store, "memory")
        .expect("the guest exports its memory")
        .read(&*store, address as usize, &mut bytes)
        .expect("the panic record lies in guest memory");
    format!("panicked: {}", String::from_utf8_lossy(&bytes))
}
