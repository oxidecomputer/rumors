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
//!   on the same operands ([`assert_less`]).
//! - *Flat per unit* (F-lin): fuel per unit of work at a doubled size is at
//!   most 1.25 times fuel per unit at the smaller size ([`assert_flat`]).
//! - *No product term* (F-mix): over a two-by-two grid of two parameters,
//!   the mixed second difference is at most a tenth of the top cell, which
//!   any cost additive in the parameters meets and a product of them does not
//!   ([`assert_no_product`]).
//! - *Growth floor* (F-grow): fuel grows by at least a stated factor across a
//!   stated growth of the input ([`assert_grows`]).
//! - *Derived bound* (F-bound): a reading stays within a bound derived from
//!   the operation's mechanism, stated beside the test.
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
        Ok(MEASURED) => call(&mut store, &instance, "reading"),
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

/// Asserts the *equal difference* form: `probe` minus `reference`, both at
/// the same `a` with `b` zero, is the same at every size in `sizes`, and
/// returns that difference.
///
/// # Panics
///
/// Panics if `probe` ever reads less than `reference`, or if two differences
/// differ.
pub fn assert_equal_difference(probe: Probe, reference: Probe, sizes: &[u64]) -> u64 {
    let rows: Vec<(u64, u64, u64)> = sizes
        .iter()
        .map(|&a| (a, measure(probe, a, 0), measure(reference, a, 0)))
        .collect();
    eprintln!("MEASURED {probe:?} - {reference:?}: (size, probe, reference) {rows:?}");
    let differences: Vec<u64> = rows
        .iter()
        .map(|&(a, measured, base)| {
            measured.checked_sub(base).unwrap_or_else(|| {
                panic!("{probe:?} read {measured} below {reference:?}'s {base} at size {a}")
            })
        })
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

/// Asserts the *flat per unit* form: `large`'s fuel per unit is at most 1.25
/// times `small`'s, where each argument is `(fuel, units)`.
///
/// # Panics
///
/// Panics if the bound fails or either unit count is zero.
pub fn assert_flat(name: &str, small: (u64, u64), large: (u64, u64)) {
    let ((fuel1, units1), (fuel2, units2)) = (small, large);
    assert!(units1 > 0 && units2 > 0, "{name}: unit counts are positive");
    eprintln!(
        "MEASURED {name}: small={fuel1}/{units1} large={fuel2}/{units2} growth per unit {:.3}",
        (fuel2 as f64 / units2 as f64) / (fuel1 as f64 / units1 as f64)
    );
    assert!(
        u128::from(fuel2) * u128::from(units1) * FLAT_DEN
            <= u128::from(fuel1) * u128::from(units2) * FLAT_NUM,
        "{name}: fuel per unit grew more than ×1.25: {fuel1}/{units1} -> {fuel2}/{units2}"
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
