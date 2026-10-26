//! Runs the amplification board in wasm guests under Wasmtime fuel and judges
//! the readings with the board's own code.
//!
//! [`FuelBoard`] measures the board at every sampling of the ladder by
//! instantiating the guest afresh for every cell of the operation × family
//! grid, on one work queue across threads. The guest gives each measured body a fresh heap, so
//! a reading does not depend on what the cell's preparation left in the
//! heap; fresh instances keep any other state an earlier cell left, such as a
//! lazily initialized static, out of it too. Every reading is therefore a
//! function of its cell alone, whatever the thread count.
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

#[cfg(test)]
mod tests;

use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use before::testing::meter::board::{self, Sampling};
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
/// maps CARGO_HOME and the toolchain's rust-src; paths inside the workspace
/// are already relative); its length shifts the guest's memory layout, and
/// so its fuel, from host to host.
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

    /// Measures every cell of the board at each of `samplings`, each in a
    /// fresh guest instance, and returns each sampling's shard captures in
    /// shard order: the [`board::LadderSpawner`] a fuel ladder sweeps with.
    ///
    /// The deal is one grid position per shard ([`board::max_useful_shards`]
    /// shards); a position no cell occupies yields an empty capture. Every
    /// sampling's shards share one work queue, the costliest expected first,
    /// so no worker idles while another sampling still has shards to run.
    /// Every cell runs in its own instance, so the order moves no reading.
    ///
    /// # Errors
    ///
    /// Returns a guest failure, the first in queue order: a trap, with the
    /// panic message the guest recorded if it recorded one.
    pub fn spawn(&self, samplings: &[Sampling]) -> io::Result<Vec<Vec<Vec<u8>>>> {
        let count = board::max_useful_shards();
        let queue = queue(samplings, count);
        let results = run_queue(
            &queue,
            self.threads,
            || self.linker(),
            |linker, &(sampling, index)| {
                self.run_shard(linker, samplings[sampling], index, count, 0)
            },
        )?;
        let mut captures = vec![vec![Vec::new(); count]; samplings.len()];
        // The queue holds each pair of sampling and shard exactly once, so
        // this fills every slot exactly once.
        for (&(sampling, index), capture) in queue.iter().zip(results) {
            captures[sampling][index] = capture;
        }
        Ok(captures)
    }

    /// Measures the cell that shard `index` of the one-cell deal owns (see
    /// [`board::cell_shard`]), at `sampling`, in a fresh instance whose measured
    /// windows each hold a spacer of `spacer` bytes, and returns the shard's
    /// capture.
    ///
    /// The guest allocates the spacer from its ordinary heap just before each
    /// window opens and frees it just after the window closes, so a spacer
    /// changes the heap state the cell's preparation leaves and nothing the
    /// body does. With the guest's window allocator, a capture under any
    /// spacer equals the capture under none, byte for byte.
    ///
    /// # Errors
    ///
    /// Returns the guest's failure, as [`spawn`](Self::spawn) does.
    pub fn spaced_shard(
        &self,
        sampling: Sampling,
        index: usize,
        spacer: u32,
    ) -> io::Result<Vec<u8>> {
        let count = board::max_useful_shards();
        self.run_shard(&self.linker(), sampling, index, count, spacer)
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

    /// Runs shard `index` of `count` at `sampling` in a fresh instance whose
    /// measured windows each hold a spacer of `spacer` bytes (none if zero),
    /// and returns its capture.
    fn run_shard(
        &self,
        linker: &Linker<()>,
        sampling: Sampling,
        index: usize,
        count: usize,
        spacer: u32,
    ) -> io::Result<Vec<u8>> {
        let wire = |value: usize| u32::try_from(value).map_err(io::Error::other);
        let mut store = Store::new(&self.engine, ());
        store.set_fuel(FUEL_TANK).map_err(io::Error::other)?;
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(io::Error::other)?;
        instance
            .get_typed_func::<u32, ()>(&mut store, "set_spacer")
            .and_then(|set_spacer| set_spacer.call(&mut store, spacer))
            .map_err(io::Error::other)?;
        let shard = instance
            .get_typed_func::<(u32, u64, u32, u32), u32>(&mut store, "shard")
            .map_err(io::Error::other)?;
        let (tag, word) = sampling.to_words();
        match shard.call(&mut store, (tag, word, wire(index)?, wire(count)?)) {
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
                    "board-fuel: shard {index}/{count} at sampling {sampling} trapped ({trap}) and \
                     {message}"
                )))
            }
        }
    }
}

/// Returns the work queue that measures `shards` shards at each of
/// `samplings`: every pair of a position in `samplings` and a shard index,
/// exactly once, the costliest expected first.
///
/// Starting the costliest work first (longest-processing-time scheduling)
/// keeps a long cell from starting late and running on alone after every
/// other worker has finished. The key only needs to rank roughly, and
/// [`knob_factor`] supplies it: a cell's operands grow with its family's
/// size knobs, and its work grows with its operands, so a cell costs more
/// at a sampling with larger knobs. Within one sampling the queue keeps
/// shard order. The knobs cannot rank cells there, because each family's
/// knobs count a different thing (tree depth, magnitude bits, teeth,
/// clocks), so they do not compare across families.
fn queue(samplings: &[Sampling], shards: usize) -> Vec<(usize, usize)> {
    let mut queue: Vec<(usize, usize)> = (0..samplings.len())
        .flat_map(|sampling| (0..shards).map(move |index| (sampling, index)))
        .collect();
    // The sort is stable, so samplings with equal factors keep their given
    // order, and each sampling's shards keep theirs.
    queue.sort_by(|&(a, _), &(b, _)| {
        knob_factor(samplings[b]).total_cmp(&knob_factor(samplings[a]))
    });
    queue
}

/// Returns how far `sampling` sets every family's size knobs above their
/// base sizes, as a factor: the work queue's cost key.
fn knob_factor(sampling: Sampling) -> f64 {
    match sampling {
        Sampling::Scaled(scale) => scale,
        // Each knob sits two past the power of two at or above its base
        // size: from one to about two times the base. Any factor strictly
        // between the ladder's base and top scales ranks the sampling
        // between them.
        Sampling::PastPowerOfTwo => 2.0,
    }
}

/// Runs `run` on every task of `queue` across `threads` worker threads,
/// each with its own state from `init`, and returns the results in queue
/// order.
///
/// Workers take tasks in queue order, and once a task fails, none takes
/// another. The tasks that ran are therefore a prefix of the queue, and it
/// holds the first failing task in queue order, whose error is returned.
///
/// # Errors
///
/// Returns the error of the first task in queue order that failed.
///
/// # Panics
///
/// Panics if `threads` is zero.
fn run_queue<T: Sync, W, R: Send>(
    queue: &[T],
    threads: usize,
    init: impl Fn() -> W + Sync,
    run: impl Fn(&W, &T) -> io::Result<R> + Sync,
) -> io::Result<Vec<R>> {
    assert!(threads > 0, "board-fuel: at least one worker thread");
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let results: Mutex<Vec<Option<io::Result<R>>>> =
        Mutex::new((0..queue.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            std::thread::Builder::new()
                .stack_size(HOST_STACK)
                .spawn_scoped(scope, || {
                    let state = init();
                    while !failed.load(Ordering::Relaxed) {
                        let position = next.fetch_add(1, Ordering::Relaxed);
                        let Some(task) = queue.get(position) else {
                            return;
                        };
                        let result = run(&state, task);
                        failed.fetch_or(result.is_err(), Ordering::Relaxed);
                        results
                            .lock()
                            .expect("no worker panics while holding the results lock")[position] =
                            Some(result);
                    }
                })
                .expect("a worker thread spawns");
        }
    });
    // Collecting stops at the first error, which precedes every task that
    // never ran.
    results
        .into_inner()
        .expect("no worker panics while holding the results lock")
        .into_iter()
        .map(|result| result.expect("a task that never ran follows a failed one"))
        .collect()
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
