//! The wasmtime driver: loads the compiled guest, instantiates it fresh per
//! case, and measures fuel per exported-kernel call.
//!
//! Determinism model: fuel is a pure function of (guest wasm bytes, call
//! sequence, payloads). A fresh [`Guest`] per program makes every replay
//! start from identical allocator and register-file state, so a proptest
//! shrink re-executes bit-identically. The engine and compiled module are
//! process-wide (compilation is the expensive part); stores are per-case.

use std::path::PathBuf;
use std::sync::OnceLock;

use wasmtime::{Config, Engine, Instance, Memory, Module, Store, TypedFunc, Val};

use crate::strategies::{BUDGET, ESCALATION_BUDGET};

/// Fuel loaded into the store before each measured call. Large enough that
/// no legitimate kernel can exhaust it; exhaustion therefore traps and is
/// reported as a harness failure rather than wrapping.
const FUEL_TANK: u64 = u64::MAX / 2;

/// Register slots pre-reserved in every fresh guest.
///
/// Sized above the worst register appetite a budgeted program can have,
/// so the file never reallocates during a measured call. The bound:
/// every op allocates at most two registers (`into_parts`), except
/// `Party::forks(n)` allocates `n` in one op — and a program's total
/// share count is capped by its fork budget — so no program allocates
/// more than `2 · max_ops + max_forks` slots (20,048 under the larger,
/// escalation budget).
const REGS_RESERVE: u32 = 32 * 1024;

// A budget raise past the reserve must be a compile error, never a
// silent reallocation inside some measured call's fuel window.
const _: () = {
    assert!(REGS_RESERVE as usize >= 2 * BUDGET.max_ops + BUDGET.max_forks as usize);
    assert!(
        REGS_RESERVE as usize
            >= 2 * ESCALATION_BUDGET.max_ops + ESCALATION_BUDGET.max_forks as usize
    );
};

/// The wasm stack ceiling handed to wasmtime.
///
/// Slack, not a derived budget: every library walk in the guest is
/// iterative (depth lives on explicit heap and bit stacks in linear
/// memory, never the call stack), so guest stack consumption is bounded
/// by the deepest static call chain and does not scale with input. The
/// ceiling is a trap limit, not an allocation — no async calls are made,
/// so no stack of this size is ever reserved — and the generous value
/// keeps a stack trap from ever masquerading as a kernel divergence.
const MAX_WASM_STACK: usize = 48 * 1024 * 1024;

/// Locate the compiled guest module.
///
/// Precedence: the `FUZZFIT_GUEST_WASM` environment variable (explicit
/// provenance), then `CARGO_TARGET_DIR`, then the workspace's configured
/// target dir (`.cargo/config.toml` points it at the repo root's
/// `target/fuzzfit`). The `just fuzzfit-*` recipes build the guest before
/// anything queries this.
pub fn guest_wasm_path() -> PathBuf {
    if let Ok(path) = std::env::var("FUZZFIT_GUEST_WASM") {
        return PathBuf::from(path);
    }
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(target).join("wasm32-unknown-unknown/release/fuzzfit_guest.wasm");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../target/fuzzfit/wasm32-unknown-unknown/release/fuzzfit_guest.wasm")
}

/// The process-wide engine and compiled module.
fn engine_and_module() -> &'static (Engine, Module) {
    static SHARED: OnceLock<(Engine, Module)> = OnceLock::new();
    SHARED.get_or_init(|| {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.max_wasm_stack(MAX_WASM_STACK);
        // Never used (no async calls are made), but wasmtime validates
        // async_stack_size >= max_wasm_stack even so.
        config.async_stack_size(MAX_WASM_STACK + 512 * 1024);
        let engine = Engine::new(&config).expect("wasmtime engine construction cannot fail here");
        let path = guest_wasm_path();
        let bytes = std::fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "fuzzfit guest wasm not found at {} ({e}); build it first: `just fuzzfit-build`",
                path.display()
            )
        });
        let module = Module::new(&engine, bytes).expect("guest wasm must validate");
        (engine, module)
    })
}

/// One fresh guest instance: a store, an instance, and its exported memory.
pub struct Guest {
    store: Store<()>,
    instance: Instance,
    memory: Memory,
}

/// A measured call's outcome: the kernel's return value and the fuel it
/// consumed (wasm instructions executed, per wasmtime's fuel schedule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measured {
    /// The kernel's i32/i64 return value (see the guest ABI's return codes).
    pub ret: i64,
    /// Fuel consumed by exactly this call.
    pub fuel: u64,
}

impl Guest {
    /// Instantiate a fresh guest (identical initial state every time).
    pub fn new() -> Guest {
        let (engine, module) = engine_and_module();
        let mut store = Store::new(engine, ());
        store.set_fuel(FUEL_TANK).expect("fuel is enabled");
        let instance =
            Instance::new(&mut store, module, &[]).expect("guest instantiates with no imports");
        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("cdylib guests export linear memory");
        let mut guest = Guest {
            store,
            instance,
            memory,
        };
        // Pre-reserve the register file to the program budget so no
        // measured kernel ever pays the file's reallocation inside its
        // fuel window (see the guest's `ff_regs_reserve` doc).
        let reserved = guest.call("ff_regs_reserve", &[REGS_RESERVE]);
        assert_eq!(reserved.ret, 0, "ff_regs_reserve failed");
        guest
    }

    /// Call exported kernel `name` with u32 `args`, measuring fuel.
    pub fn call(&mut self, name: &str, args: &[u32]) -> Measured {
        let func = self
            .instance
            .get_func(&mut self.store, name)
            .unwrap_or_else(|| panic!("guest does not export {name}"));
        let params: Vec<Val> = args.iter().map(|&a| Val::I32(a as i32)).collect();
        let mut results = [Val::I32(0)];
        self.store.set_fuel(FUEL_TANK).expect("fuel is enabled");
        func.call(&mut self.store, &params, &mut results)
            .unwrap_or_else(|e| panic!("guest call {name} trapped: {e}"));
        let remaining = self.store.get_fuel().expect("fuel is enabled");
        let ret = match results[0] {
            Val::I32(v) => v as i64,
            Val::I64(v) => v,
            ref other => panic!("guest kernel {name} returned unexpected type {other:?}"),
        };
        Measured {
            ret,
            fuel: FUEL_TANK - remaining,
        }
    }

    /// Write `bytes` into the staging buffer (unmeasured: the guest only
    /// resizes; the payload copy is a host-side memory write).
    pub fn stage_write(&mut self, bytes: &[u8]) {
        let ptr = self.call("ff_stage_prepare", &[bytes.len() as u32]).ret as u32;
        self.memory
            .write(&mut self.store, ptr as usize, bytes)
            .expect("stage pointer in bounds");
    }

    /// Read the staging buffer (unmeasured host-side memory read).
    pub fn stage_read(&mut self) -> Vec<u8> {
        let len = self.call("ff_stage_len", &[]).ret as usize;
        let ptr = self.call("ff_stage_ptr", &[]).ret as usize;
        let mut out = vec![0u8; len];
        self.memory
            .read(&self.store, ptr, &mut out)
            .expect("stage pointer in bounds");
        out
    }

    /// Typed convenience for i64-returning kernels (`ff_version_min_ticks`).
    pub fn call_i64(&mut self, name: &str, args: &[u32]) -> Measured {
        let func: TypedFunc<u32, i64> = self
            .instance
            .get_typed_func(&mut self.store, name)
            .unwrap_or_else(|e| panic!("guest kernel {name}: {e}"));
        self.store.set_fuel(FUEL_TANK).expect("fuel is enabled");
        let ret = func
            .call(&mut self.store, args[0])
            .unwrap_or_else(|e| panic!("guest call {name} trapped: {e}"));
        let remaining = self.store.get_fuel().expect("fuel is enabled");
        Measured {
            ret,
            fuel: FUEL_TANK - remaining,
        }
    }
}

impl Default for Guest {
    fn default() -> Self {
        Guest::new()
    }
}
