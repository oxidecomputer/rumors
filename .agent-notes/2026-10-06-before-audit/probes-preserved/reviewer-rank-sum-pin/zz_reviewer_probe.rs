//! Reviewer's temporary probe: outcome and memory high-water per `RankArithmetic` case. Never committed.

use std::time::Instant;
use wasmtime::{Engine, Instance, Module, Store};

/// Prints each case's outcome, final memory pages, and elapsed seconds.
#[test]
fn zz_reviewer_probe_rank_pages() {
    let path = std::env::var("WASM32_PINS_GUEST_WASM").expect("guest path");
    let engine = Engine::default();
    let module = Module::from_file(&engine, &path).expect("module");
    for case in [4u64, 5, 6, 7] {
        let start = Instant::now();
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).expect("instance");
        let f = instance
            .get_typed_func::<(u32, u64, u64), i32>(&mut store, "check")
            .expect("check");
        let result = f.call(&mut store, (7, case, 0));
        let pages = instance
            .get_memory(&mut store, "memory")
            .map_or(0, |m| m.size(&store));
        eprintln!(
            "PROBE case {case}: {:?} pages={pages} GiB={:.3} secs={:.1}",
            result.map_err(|e| format!("{e:#}").lines().next().unwrap_or("").to_owned()),
            pages as f64 * 65536.0 / (1u64 << 30) as f64,
            start.elapsed().as_secs_f64()
        );
    }
}
