//! Runs the amplification board in wasm guests under Wasmtime fuel and judges
//! the readings with the board's own code.
//!
//! [`FuelBoard`] measures one sampling scale of the board by instantiating
//! the guest afresh for every cell of the operation × family grid, on a work
//! queue across threads. A fuel reading counts the allocator's instructions
//! too, so a cell measured after other cells in the same instance would read
//! what they left behind. Fresh instances make every reading a function of
//! its cell alone, whatever the thread count.
//!
//! [`provenance`] names what a fuel reading depends on beyond the source tree
//! and that the harness can read back: the compiler that built the guest, the
//! Wasmtime version, and the cost table ([`costs`]). The board's fuel ceilings
//! record the provenance they were calibrated under, and [`check_guest`]
//! refuses a guest built under any other, or one that embeds a host-specific
//! path. The guest's build profile and source-path mappings move fuel too;
//! the `amp-board-fuel-build` recipe holds them fixed, and no check reads
//! them back.

pub mod costs;

use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use before::testing::meter::board;
use wasmtime::{Caller, Config, Engine, Instance, Linker, Module, Store};

/// Fuel loaded into each store: far above any cell's consumption, so the
/// counter never runs dry and `FUEL_TANK` minus the remaining fuel is the
/// instance's consumption so far.
const FUEL_TANK: u64 = u64::MAX / 2;

/// The wasm stack ceiling handed to Wasmtime: a trap limit, not a
/// reservation.
///
/// Slack, not a derived bound: `before`'s traversals keep their depth on
/// explicit stacks, and the ceiling sits far above the stack any board cell
/// has needed. A cell that overran it would trap and fail the run, never
/// read as a measurement.
const MAX_WASM_STACK: usize = 48 * 1024 * 1024;

/// The stack of every host thread that calls into a guest.
///
/// Wasm frames run on the calling thread's stack, so it must exceed
/// [`MAX_WASM_STACK`].
const HOST_STACK: usize = 64 * 1024 * 1024;

/// Describes what the readings of guest `wasm` depend on beyond the source
/// tree and can be read back: the rustc that built it (from the module's own
/// `producers` section), the Wasmtime version, and the cost table.
///
/// # Errors
///
/// Returns a message if the module records no rustc in its `producers`
/// section.
pub fn provenance(wasm: &[u8]) -> Result<String, String> {
    Ok(format!(
        "rustc {rustc}; wasmtime {wasmtime}; costs: {costs}",
        rustc = guest_rustc(wasm)?,
        wasmtime = env!("BOARD_FUEL_WASMTIME_VERSION"),
        costs = costs::render_costs(&costs::operator_cost()),
    ))
}

/// Returns the rustc version that the `producers` custom section of `wasm`
/// records as having processed it.
fn guest_rustc(wasm: &[u8]) -> Result<String, String> {
    use wasmtime::wasmparser::{KnownCustom, Parser, Payload};
    for payload in Parser::new(0).parse_all(wasm) {
        let payload = payload.map_err(|error| format!("the guest does not parse: {error}"))?;
        let Payload::CustomSection(section) = payload else {
            continue;
        };
        let KnownCustom::Producers(fields) = section.as_known() else {
            continue;
        };
        for field in fields {
            let field = field.map_err(|error| format!("malformed producers section: {error}"))?;
            if field.name != "processed-by" {
                continue;
            }
            for value in field.values {
                let value =
                    value.map_err(|error| format!("malformed producers section: {error}"))?;
                if value.name == "rustc" {
                    return Ok(value.version.to_owned());
                }
            }
        }
    }
    Err("the guest's producers section records no rustc".to_owned())
}

/// Checks guest `wasm` before it is judged: it was built under the
/// provenance the board's fuel ceilings were calibrated under, and it embeds
/// no path under `home`, the home directory of the host that built it.
///
/// A rustc, Wasmtime, or cost table other than the calibration's makes the
/// readings incommensurable with the ceilings. A host path in the guest
/// means a source-path remapping failed (the `amp-board-fuel-build` recipe
/// maps the checkout, CARGO_HOME, and the toolchain's rust-src); its length
/// shifts the guest's memory layout, and so its fuel, from host to host.
///
/// # Errors
///
/// Returns a message naming what differs.
pub fn check_guest(wasm: &[u8], home: &str) -> Result<(), String> {
    let calibrated = board::cell_ceilings(board::Currency::Fuel, "wasm32")
        .and_then(|table| table.provenance)
        .unwrap_or("no calibrated table");
    let building = provenance(wasm)?;
    if calibrated != building {
        return Err(format!(
            "the fuel ceilings were calibrated under\n  {calibrated}\nbut this guest was \
             built under\n  {building}\nrecalibrate fuel with `just amp-board-calibrate-fuel`"
        ));
    }
    check_paths(wasm, home)
}

/// Checks that guest `wasm` embeds no path under `home`, the home directory
/// of the host that built it: [`check_guest`]'s second check, which a
/// calibration needs as well.
///
/// # Errors
///
/// Returns a message naming `home` if the guest embeds a path under it.
pub fn check_paths(wasm: &[u8], home: &str) -> Result<(), String> {
    let home = format!("{}/", home.trim_end_matches('/'));
    if home.len() > 1
        && wasm
            .windows(home.len())
            .any(|window| window == home.as_bytes())
    {
        return Err(format!(
            "the guest embeds a path under {home}, so its fuel depends on where it was built; \
             build it with `just amp-board-fuel-build`"
        ));
    }
    Ok(())
}

/// The compiled guest and the engine that meters it.
pub struct FuelBoard {
    /// The engine, with fuel metering and the explicit cost table.
    engine: Engine,
    /// The compiled guest module.
    module: Module,
    /// The number of worker threads.
    threads: usize,
}

impl FuelBoard {
    /// Compiles the guest `wasm` for a board that runs on `threads` workers.
    ///
    /// # Errors
    ///
    /// Returns an error if the module does not compile.
    ///
    /// # Panics
    ///
    /// Panics if `threads` is zero.
    pub fn new(wasm: &[u8], threads: usize) -> io::Result<FuelBoard> {
        assert!(threads > 0, "board-fuel: at least one worker thread");
        let mut config = Config::new();
        config.consume_fuel(true);
        config.operator_cost(costs::operator_cost());
        config.max_wasm_stack(MAX_WASM_STACK);
        // Never used, since no call is async, but Wasmtime requires it to
        // exceed the wasm stack ceiling.
        config.async_stack_size(MAX_WASM_STACK + 512 * 1024);
        let engine = Engine::new(&config).map_err(io::Error::other)?;
        let module = Module::new(&engine, wasm).map_err(io::Error::other)?;
        Ok(FuelBoard {
            engine,
            module,
            threads,
        })
    }

    /// Measures every cell of the board at `scale`, each in a fresh guest
    /// instance, and returns the shard captures in shard order: the
    /// [`board::ShardSpawner`] a fuel ladder sweeps with.
    ///
    /// The deal is one grid position per shard ([`board::max_useful_shards`]
    /// shards); a position no cell occupies yields an empty capture.
    ///
    /// # Errors
    ///
    /// Returns the first guest failure: a trap, with the panic message the
    /// guest recorded if it recorded one.
    pub fn spawn(&self, scale: f64) -> io::Result<Vec<Vec<u8>>> {
        let count = board::max_useful_shards();
        let next = AtomicUsize::new(0);
        let failed = AtomicBool::new(false);
        let captures: Mutex<Vec<Option<io::Result<Vec<u8>>>>> =
            Mutex::new((0..count).map(|_| None).collect());
        std::thread::scope(|scope| {
            for _ in 0..self.threads {
                std::thread::Builder::new()
                    .stack_size(HOST_STACK)
                    .spawn_scoped(scope, || {
                        let linker = self.linker();
                        while !failed.load(Ordering::Relaxed) {
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            if index >= count {
                                return;
                            }
                            let capture = self.run_shard(&linker, scale, index, count);
                            failed.fetch_or(capture.is_err(), Ordering::Relaxed);
                            captures
                                .lock()
                                .expect("no worker panics while holding the captures lock")
                                [index] = Some(capture);
                        }
                    })
                    .expect("a worker thread spawns");
            }
        });
        // The queue hands out shards in increasing order and stops only after
        // a failure, so every unrun shard follows a failed one, and an
        // in-order scan meets the failure first.
        captures
            .into_inner()
            .expect("no worker panics while holding the captures lock")
            .into_iter()
            .map(|capture| capture.expect("an unrun shard follows a failed one"))
            .collect()
    }

    /// The linker that supplies the guest's one import: its consumed fuel.
    fn linker(&self) -> Linker<()> {
        let mut linker = Linker::new(&self.engine);
        linker
            .func_wrap(
                "board_fuel",
                "fuel_used",
                |caller: Caller<'_, ()>| -> wasmtime::Result<u64> {
                    Ok(FUEL_TANK - caller.get_fuel()?)
                },
            )
            .expect("the fuel import is defined once");
        linker
    }

    /// Runs shard `index` of `count` at `scale` in a fresh instance and
    /// returns its capture.
    fn run_shard(
        &self,
        linker: &Linker<()>,
        scale: f64,
        index: usize,
        count: usize,
    ) -> io::Result<Vec<u8>> {
        let wire = |value: usize| u32::try_from(value).map_err(io::Error::other);
        let mut store = Store::new(&self.engine, ());
        store.set_fuel(FUEL_TANK).map_err(io::Error::other)?;
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(io::Error::other)?;
        let shard = instance
            .get_typed_func::<(u64, u32, u32), u32>(&mut store, "shard")
            .map_err(io::Error::other)?;
        match shard.call(&mut store, (scale.to_bits(), wire(index)?, wire(count)?)) {
            Ok(0) => read(&mut store, &instance, "output_ptr", "output_len"),
            Ok(status) => Err(io::Error::other(format!(
                "board-fuel: shard {index}/{count} returned status {status}"
            ))),
            Err(trap) => {
                let panicked = call::<u32>(&mut store, &instance, "panic_recorded")? != 0;
                let message = if panicked {
                    let bytes = read(
                        &mut store,
                        &instance,
                        "panic_message_ptr",
                        "panic_message_len",
                    )?;
                    format!("panicked: {}", String::from_utf8_lossy(&bytes))
                } else {
                    "recorded no panic, so it likely ran out of memory".to_owned()
                };
                Err(io::Error::other(format!(
                    "board-fuel: shard {index}/{count} at scale {scale} trapped ({trap}) and \
                     {message}"
                )))
            }
        }
    }
}

/// Calls the guest's nullary export `name`.
fn call<R: wasmtime::WasmResults>(
    store: &mut Store<()>,
    instance: &Instance,
    name: &str,
) -> io::Result<R> {
    instance
        .get_typed_func::<(), R>(&mut *store, name)
        .and_then(|func| func.call(&mut *store, ()))
        .map_err(io::Error::other)
}

/// Copies the guest bytes whose address and length the exports `ptr` and
/// `len` return.
fn read(store: &mut Store<()>, instance: &Instance, ptr: &str, len: &str) -> io::Result<Vec<u8>> {
    let address = call::<u32>(store, instance, ptr)?;
    let length = call::<u32>(store, instance, len)?;
    let memory = instance
        .get_memory(&mut *store, "memory")
        .ok_or_else(|| io::Error::other("board-fuel: the guest exports no memory"))?;
    let mut bytes = vec![0; usize::try_from(length).map_err(io::Error::other)?];
    memory
        .read(
            &*store,
            usize::try_from(address).map_err(io::Error::other)?,
            &mut bytes,
        )
        .map_err(io::Error::other)?;
    Ok(bytes)
}
