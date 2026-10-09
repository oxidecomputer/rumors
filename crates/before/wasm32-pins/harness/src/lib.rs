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
    /// The guest trapped.
    ///
    /// A panic and an allocation failure both abort the guest with
    /// [`Trap::UnreachableCodeReached`], so the trap alone cannot tell them
    /// apart. `panic` separates them in one direction only.
    ///
    /// - `Some` always means the guest panicked. It holds the panic's
    ///   message; a message longer than 1,024 bytes keeps its longest prefix
    ///   that ends on a character boundary within that limit.
    /// - `None` means no panic reached the guest's panic hook. Usually the
    ///   guest ran out of memory. A panic raised after memory is exhausted
    ///   also reads `None` when its message has runtime arguments, as every
    ///   `expect` message and `Result::unwrap`'s do (`Option::unwrap`'s does
    ///   not), because std allocates to format such a message before it runs
    ///   the hook.
    ///
    /// A pin that expects a panic can therefore fail spuriously once memory
    /// is exhausted, but it never passes when the guest did not panic.
    Trapped {
        /// The wasm trap that ended the check.
        trap: Trap,
        /// The message of the panic that caused the trap, if one reached the
        /// guest's panic hook.
        panic: Option<String>,
    },
}

/// Calls `check` in a fresh guest instance.
pub fn run(check: Check, a: u64, b: u64) -> Outcome {
    run_raw(u32::from(check), a, b)
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
            Some(&trap) => Outcome::Trapped {
                trap,
                panic: recorded_panic(&instance, &mut store),
            },
            None => panic!("guest check failed outside wasm: {error}"),
        },
    }
}

/// Reads the panic message the guest recorded before it trapped, if any.
///
/// Wasmtime permits calls into an instance after one of its calls trapped,
/// and the guest keeps the record in static memory, so it survives the trap.
fn recorded_panic(instance: &Instance, store: &mut Store<()>) -> Option<String> {
    let mut export = |name| {
        instance
            .get_typed_func::<(), u32>(&mut *store, name)
            .and_then(|function| function.call(&mut *store, ()))
            .unwrap_or_else(|error| panic!("the guest's `{name}` export fails: {error}"))
    };
    if export("panic_recorded") == 0 {
        return None;
    }
    let address = export("panic_message_ptr");
    let len = export("panic_message_len");

    let memory = instance
        .get_memory(&mut *store, "memory")
        .expect("the guest exports its linear memory");
    let mut message = vec![0; len as usize];
    memory
        .read(&*store, address as usize, &mut message)
        .expect("the recorded message lies inside the guest's memory");
    Some(String::from_utf8(message).expect("the guest records UTF-8 cut at a character boundary"))
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
