//! Temporary builder probe: memory high-water per `RankArithmetic` case. Not committed.

use std::time::Instant;
use wasmtime::{Engine, Instance, Module, Store};

/// Prints outcome code, final memory pages, and elapsed seconds per case.
#[test]
fn zz_scratch_probe_rank_arithmetic_pages() {
    let path = std::env::var("WASM32_PINS_GUEST_WASM").expect("guest path");
    let engine = Engine::default();
    let module = Module::from_file(&engine, &path).expect("module");
    let only: Option<u64> = std::env::var("PROBE_CASE")
        .ok()
        .and_then(|c| c.parse().ok());
    for (check, case) in [(1u32, 0u64), (8, 1), (7, 7), (7, 8)] {
        if only.is_some_and(|c| c != case) {
            continue;
        }
        let start = Instant::now();
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).expect("instance");
        let f = instance
            .get_typed_func::<(u32, u64, u64), i32>(&mut store, "check")
            .expect("check");
        let result = f.call(&mut store, (check, case, 0));
        let pages = instance
            .get_memory(&mut store, "memory")
            .map_or(0, |m| m.size(&store));
        let panicked = instance
            .get_typed_func::<(), u32>(&mut store, "zz_panicked")
            .ok()
            .and_then(|g| g.call(&mut store, ()).ok());
        eprintln!("PROBE check {check} case {case}: panicked={panicked:?}");
        eprintln!(
            "PROBE check {check} case {case}: {:?} pages={pages} GiB={:.3} secs={:.1}",
            result.map_err(|e| e.to_string()),
            pages as f64 * 65536.0 / (1u64 << 30) as f64,
            start.elapsed().as_secs_f64()
        );
    }
}
