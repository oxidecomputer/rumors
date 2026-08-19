//! The wasmtime driver for the 32-bit boundary pins: loads the compiled
//! guest, instantiates it fresh per call, and reports each export's outcome
//! as value-or-trap.
//!
//! A fresh instance per call keeps the pins independent: wasm linear memory
//! only ever grows, and one pin's multi-gigabyte peak must not become the
//! next pin's baseline. The engine and compiled module are process-wide
//! (compilation is the expensive part). No fuel metering and no pooling
//! allocator: the pins assert outcomes, not instruction counts, and each
//! wants the full 4 GiB 32-bit address space a pooled slot would cap.

use std::path::PathBuf;
use std::sync::OnceLock;

// `Trap` is re-exported so the pin tests can name the exact trap they
// assert without a direct wasmtime dependency edge of their own.
pub use wasmtime::Trap;
use wasmtime::{Engine, Instance, Module, Store};

/// One pin call's outcome: the export's return value, or the trap that
/// aborted it.
///
/// A trap is a first-class outcome, not a driver failure: a guest panic
/// surfaces as the `unreachable` trap under `panic = abort`, and a boundary
/// found panicking is pinned as exactly that trap until its cure lands, so
/// the pins assert on this axis directly.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The export returned: nonnegative is its observation, negative names
    /// the first failed in-guest check (the guest's doc comments key them).
    Value(i64),
    /// The export trapped; `Trap::UnreachableCodeReached` is a guest panic.
    Trapped(Trap),
}

/// Locate the compiled guest module.
///
/// Precedence: the `WASM32_PINS_GUEST_WASM` environment variable (explicit
/// provenance, what the `just wasm32-pins` recipe passes), then the
/// workspace-relative target dir the recipe builds into.
pub fn guest_wasm_path() -> PathBuf {
    if let Ok(path) = std::env::var("WASM32_PINS_GUEST_WASM") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../../../../target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm",
    )
}

/// The process-wide engine and compiled module.
fn engine_and_module() -> &'static (Engine, Module) {
    static SHARED: OnceLock<(Engine, Module)> = OnceLock::new();
    SHARED.get_or_init(|| {
        let engine = Engine::default();
        let path = guest_wasm_path();
        let module = Module::from_file(&engine, &path).unwrap_or_else(|error| {
            panic!(
                "wasm32-pins guest not loadable from {} (build it first: `just wasm32-pins-build`): {error}",
                path.display()
            )
        });
        (engine, module)
    })
}

/// Call a nullary pin export in a fresh instance.
pub fn call0(export: &str) -> Outcome {
    call(export, &[])
}

/// Call a one-argument pin export in a fresh instance.
pub fn call1(export: &str, arg: u64) -> Outcome {
    call(export, &[arg])
}

/// Call a two-argument pin export in a fresh instance.
pub fn call2(export: &str, a: u64, b: u64) -> Outcome {
    call(export, &[a, b])
}

fn call(export: &str, args: &[u64]) -> Outcome {
    let (engine, module) = engine_and_module();
    let mut store = Store::new(engine, ());
    let instance =
        Instance::new(&mut store, module, &[]).expect("the guest instantiates without imports");
    let result = match *args {
        [] => {
            let func = instance
                .get_typed_func::<(), i64>(&mut store, export)
                .expect("the export exists with the pinned signature");
            func.call(&mut store, ())
        }
        [arg] => {
            let func = instance
                .get_typed_func::<u64, i64>(&mut store, export)
                .expect("the export exists with the pinned signature");
            func.call(&mut store, arg)
        }
        [a, b] => {
            let func = instance
                .get_typed_func::<(u64, u64), i64>(&mut store, export)
                .expect("the export exists with the pinned signature");
            func.call(&mut store, (a, b))
        }
        _ => unreachable!("pin exports take at most two arguments"),
    };
    match result {
        Ok(value) => Outcome::Value(value),
        Err(error) => match error.downcast_ref::<Trap>() {
            Some(&trap) => Outcome::Trapped(trap),
            None => panic!("pin export {export} failed outside wasm: {error}"),
        },
    }
}
