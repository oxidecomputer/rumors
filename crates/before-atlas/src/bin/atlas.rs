//! The atlas runner: sample every roster operation over the size grid,
//! measure fuel in the fuzz-fit guest, render the per-op heatmaps and the
//! gallery.
//!
//! Flags (all optional): `--samples <n>` per column (default 300),
//! `--max-bytes <n>` grid top (default 256), `--seed <u64>` base seed
//! (default 0xa71a5), `--out <dir>` output directory (default
//! `target/atlas` under the current directory). Provenance: the `ATLAS_TIP`
//! environment variable (the recipe passes `git rev-parse HEAD`) is
//! stamped into every render; runs without it stamp `untracked`.
//!
//! The run is a pure function of (guest wasm, plan): every cell's RNG is
//! seeded from its coordinates, so two runs of the same plan on the same
//! guest emit byte-identical measurements in any execution order. Wall
//! times printed per operation are information for the operator, never an
//! input to anything. So are the count-table build's progress lines: each
//! table's entry counts are deterministic, but the two tables build
//! concurrently, so their lines interleave in scheduler order.

use std::path::PathBuf;
use std::time::Instant;

use before_atlas::ops::ROSTER;
use before_atlas::plan::{run_op, Plan, Samplers};
use before_atlas::render::{render_gallery, render_op, RenderMeta};

fn main() {
    let mut plan = Plan {
        base_seed: 0xa71a5,
        samples_per_column: 300,
        max_bytes: 256,
    };
    let mut out = PathBuf::from("target/atlas");
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .unwrap_or_else(|| panic!("flag {flag} needs a value"))
        };
        match flag.as_str() {
            "--samples" => plan.samples_per_column = value().parse().expect("--samples <count>"),
            "--max-bytes" => plan.max_bytes = value().parse().expect("--max-bytes <bytes>"),
            "--seed" => plan.base_seed = value().parse().expect("--seed <u64>"),
            "--out" => out = PathBuf::from(value()),
            other => panic!("unknown flag {other} (see the module doc for the flag list)"),
        }
    }
    let meta = RenderMeta {
        commit: std::env::var("ATLAS_TIP").unwrap_or_else(|_| "untracked".into()),
        base_seed: plan.base_seed,
        samples_per_column: plan.samples_per_column,
    };
    std::fs::create_dir_all(&out).expect("output directory must be creatable");

    let t0 = Instant::now();
    // One progress line per sixteenth of each table: legible over a
    // multi-minute large-span build, bounded noise at the default span.
    // Entry counts are deterministic; elapsed is operator information.
    let step = (8 * plan.max_bytes + 1).div_ceil(16);
    let samplers = Samplers::build_with_progress(&plan, |table, done, total| {
        if done % step == 0 && done < total {
            println!(
                "  {table} table: {done}/{total} entries, {:.1?}",
                t0.elapsed()
            );
        }
    });
    println!(
        "count tables to {} bytes: {:.1?}",
        plan.max_bytes,
        t0.elapsed()
    );

    let mut rendered = Vec::new();
    for op in ROSTER {
        let t0 = Instant::now();
        let atlas = run_op(&plan, &samplers, op);
        let measured = t0.elapsed();
        let rejected: u64 = atlas.samples.iter().map(|s| s.rejected).sum();
        let accepted = atlas.samples.len() as u64;
        let path = render_op(&atlas, &meta, &out).expect("render must succeed");
        println!(
            "{}: {} samples / {} columns, {} overlay points, acceptance {:.1}%, {:.1?} → {}",
            op.name,
            atlas.samples.len(),
            plan.columns(op.operands.len()).len(),
            atlas.overlay.len(),
            100.0 * accepted as f64 / (accepted + rejected) as f64,
            measured,
            path.display()
        );
        rendered.push((op.name.to_string(), path));
    }
    let gallery = render_gallery(&rendered, &meta, &out).expect("gallery must render");
    println!("gallery → {}", gallery.display());
}
