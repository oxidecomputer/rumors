//! Runs programs in the compiled guest, each in a fresh instance under fuel.
//!
//! The engine prices operators with the fuel board's table
//! (`board_fuel_harness::costs`), so a byte moved by `memory.copy` or
//! `memory.fill` costs one, as an element moved by a loop does. Fuel is a
//! deterministic function of the guest module, the cost table, and the
//! program; every program runs in a new instance, so no state another run
//! left reaches it.

use std::path::PathBuf;
use std::sync::OnceLock;

use board_fuel_harness::costs::operator_cost;
use suanpan_fuel_protocol::Program;
use wasmtime::{Caller, Config, Engine, Instance, Linker, Module, Store};

/// The environment variable naming the compiled guest module; the
/// `suanpan-fuel` recipe sets it.
pub const GUEST_ENV: &str = "SUANPAN_FUEL_GUEST_WASM";

/// Fuel loaded into each store: far above any program's consumption, so the
/// counter never runs dry and `FUEL_TANK` minus the remaining fuel is the
/// instance's consumption so far.
const FUEL_TANK: u64 = u64::MAX / 2;

/// One program's run: the fuel readings and the observation words.
#[derive(Clone, Debug)]
pub struct Run {
    /// The instance's consumed fuel before the first step and after each
    /// step instance: one more reading than the program has step instances.
    pub fuel: Vec<u64>,
    /// One observation word per step instance.
    pub observations: Vec<u64>,
    /// The bytes the guest's reallocations had copied, before the first step
    /// and after each step instance, as for `fuel`.
    pub copied: Vec<u64>,
}

impl Run {
    /// Returns the fuel spent by the first `count` step instances.
    pub fn prefix(&self, count: usize) -> u64 {
        self.fuel[count] - self.fuel[0]
    }

    /// Returns the bytes the first `count` step instances' reallocations
    /// copied, which cost one fuel each.
    pub fn copied_prefix(&self, count: usize) -> u64 {
        self.copied[count] - self.copied[0]
    }

    /// Returns the number of step instances the run holds.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Returns whether the run holds no step instance.
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }
}

/// The engine and the compiled guest, shared by every run in the process.
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
                panic!("{GUEST_ENV} must name the compiled guest; run `just suanpan-fuel`")
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

/// Runs `program` in a fresh guest instance.
///
/// # Panics
///
/// Panics if the guest traps, naming the panic message it recorded, or if it
/// refuses the program as malformed.
pub fn run(program: &Program) -> Run {
    let Compiled { engine, module } = compiled();
    let mut store = Store::new(engine, ());
    store.set_fuel(FUEL_TANK).expect("fuel is enabled");
    let mut linker = Linker::new(engine);
    linker
        .func_wrap(
            "suanpan_fuel",
            "fuel_used",
            |caller: Caller<'_, ()>| -> wasmtime::Result<u64> {
                Ok(FUEL_TANK - caller.get_fuel()?)
            },
        )
        .expect("the fuel import is defined once");
    let instance = linker
        .instantiate(&mut store, module)
        .expect("the guest instantiates");

    let bytes = program.encode();
    let length = u32::try_from(bytes.len()).expect("an encoded program fits a wasm32 buffer");
    let address = call::<u32, u32>(&mut store, &instance, "input_buffer", length);
    memory(&mut store, &instance)
        .write(&mut store, address as usize, &bytes)
        .expect("the input buffer holds the program");

    let status = instance
        .get_typed_func::<(), u32>(&mut store, "run")
        .expect("the guest exports `run`")
        .call(&mut store, ());
    match status {
        Ok(0) => {}
        Ok(status) => panic!("the guest refused the program (status {status}): {program:?}"),
        Err(trap) => panic!(
            "the guest trapped ({trap}) and {}",
            panic_message(&mut store, &instance)
        ),
    }

    let address = call::<(), u32>(&mut store, &instance, "output_ptr", ());
    let length = call::<(), u32>(&mut store, &instance, "output_len", ()) as usize;
    let mut bytes = vec![0u8; 8 * length];
    memory(&mut store, &instance)
        .read(&store, address as usize, &mut bytes)
        .expect("the output lies in guest memory");
    let mut output: Vec<u64> = bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("chunks of eight bytes")))
        .collect();
    let steps = program.steps.len() * program.repetitions as usize;
    assert_eq!(
        output.len(),
        3 * steps + 2,
        "the guest returns a reading and a copy count per step instance plus one, and an \
         observation per step instance"
    );
    let copied = output.split_off(2 * steps + 1);
    let observations = output.split_off(steps + 1);
    Run {
        fuel: output,
        observations,
        copied,
    }
}

/// Calls the guest export `name`.
fn call<P: wasmtime::WasmParams, R: wasmtime::WasmResults>(
    store: &mut Store<()>,
    instance: &Instance,
    name: &str,
    params: P,
) -> R {
    instance
        .get_typed_func::<P, R>(&mut *store, name)
        .and_then(|func| func.call(&mut *store, params))
        .unwrap_or_else(|error| panic!("calling the guest's `{name}`: {error}"))
}

/// Returns the guest's linear memory.
fn memory(store: &mut Store<()>, instance: &Instance) -> wasmtime::Memory {
    instance
        .get_memory(store, "memory")
        .expect("the guest exports its memory")
}

/// Describes what the guest's panic record holds after a trap.
fn panic_message(store: &mut Store<()>, instance: &Instance) -> String {
    if call::<(), u32>(store, instance, "panic_recorded", ()) == 0 {
        return "recorded no panic, so it likely ran out of memory".to_owned();
    }
    let address = call::<(), u32>(store, instance, "panic_message_ptr", ());
    let length = call::<(), u32>(store, instance, "panic_message_len", ());
    let mut bytes = vec![0u8; length as usize];
    memory(store, instance)
        .read(&*store, address as usize, &mut bytes)
        .expect("the panic record lies in guest memory");
    format!("panicked: {}", String::from_utf8_lossy(&bytes))
}
