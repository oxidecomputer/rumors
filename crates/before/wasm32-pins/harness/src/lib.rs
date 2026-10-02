//! Runs checks inside a fresh instance of the 32-bit guest.
//!
//! Wasm linear memory grows but does not shrink. A fresh instance therefore
//! prevents one large check from becoming the next check's baseline. The
//! engine and compiled module are shared because compilation is expensive.

use std::path::PathBuf;
use std::sync::OnceLock;

pub use wasm32_pins_protocol::{Check, Failure};
pub use wasmtime::Trap;
use wasmtime::{Engine, Instance, Module, Store};

/// The observable result of one guest check.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The check established its invariant.
    Passed,
    /// The check completed but found the stated discrepancy.
    Failed(Failure),
    /// The guest trapped, normally because the exercised operation panicked.
    Trapped(Trap),
}

/// Calls `check` in a fresh guest instance.
pub fn run(check: Check, a: u64, b: u64) -> Outcome {
    run_raw(check as u32, a, b)
}

/// Calls the guest with an unchecked check number.
///
/// This is public only so the integration suite can prove that the protocol's
/// typed dispatch failure remains live.
pub fn run_raw(check: u32, a: u64, b: u64) -> Outcome {
    let (engine, module) = engine_and_module();
    let mut store = Store::new(engine, ());
    let instance =
        Instance::new(&mut store, module, &[]).expect("the guest instantiates without imports");
    let function = instance
        .get_typed_func::<(u32, u64, u64), i32>(&mut store, "check")
        .expect("the guest exports the check protocol");

    match function.call(&mut store, (check, a, b)) {
        Ok(wasm32_pins_protocol::PASS) => Outcome::Passed,
        Ok(code) => match Failure::from_code(code) {
            Some(failure) => Outcome::Failed(failure),
            None => panic!("guest returned unknown failure code {code}"),
        },
        Err(error) => match error.downcast_ref::<Trap>() {
            Some(&trap) => Outcome::Trapped(trap),
            None => panic!("guest check failed outside wasm: {error}"),
        },
    }
}

/// Returns the guest built beside this detached workspace.
fn guest_wasm_path() -> PathBuf {
    if let Ok(path) = std::env::var("WASM32_PINS_GUEST_WASM") {
        return PathBuf::from(path);
    }
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(target).join("wasm32-unknown-unknown/release/wasm32_pins_guest.wasm");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../../../target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm",
    )
}

/// Shares the expensive compilation while keeping stores and memories local.
fn engine_and_module() -> &'static (Engine, Module) {
    static SHARED: OnceLock<(Engine, Module)> = OnceLock::new();
    SHARED.get_or_init(|| {
        let engine = Engine::default();
        let path = guest_wasm_path();
        let module = Module::from_file(&engine, &path).unwrap_or_else(|error| {
            panic!(
                "wasm32 guest not loadable from {} (build it first): {error}",
                path.display()
            )
        });
        (engine, module)
    })
}
