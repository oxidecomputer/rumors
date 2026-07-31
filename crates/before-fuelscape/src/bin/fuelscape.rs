//! The fuelscape runner: sample every roster operation over the size grid,
//! measure fuel in the fuzz-fit guest, render the per-op heatmaps and the
//! gallery — or replay rendering from a persisted dump without measuring.
//!
//! Flags (all optional): `--samples <n>` per column (default 300),
//! `--max-bytes <n>` grid top (default 256), `--seed <u64>` base seed
//! (default 0xa71a5), `--out <dir>` output directory (default
//! `target/fuelscape` under the current directory), `--dump` (persist
//! every operation's raw atlas as JSON beside the SVGs, in the
//! `before_fuelscape::dump` layout), `--render-from <dump>` (skip
//! measurement: load a dump — its `atlas.json` or the directory holding
//! it — and render its SVGs and gallery into `--out`; the measuring
//! flags `--samples`/`--max-bytes`/`--seed` and `--dump` do not combine
//! with it), and `--font-scale <f64>` (scale all SVG text for print,
//! default 1.0; usable in both modes).
//!
//! Provenance: the `FUELSCAPE_TIP` environment variable (the recipe
//! passes `git rev-parse HEAD`) is stamped into every measuring run's
//! render; runs without it stamp `untracked`. A replay stamps the meta
//! recorded in the dump, never the current environment: the figures
//! describe the run that measured, whatever commit re-renders them.
//!
//! The run is a pure function of (guest wasm, plan): every cell's RNG is
//! seeded from its coordinates, so two runs of the same plan on the same
//! guest emit byte-identical measurements in any execution order — and
//! the dump, being those measurements verbatim, inherits the same
//! guarantee. Rendering is a pure function of (measured data, font
//! scale), so a replay from a dump is byte-identical to the measuring
//! run's own renders at the same font scale. Wall times printed per
//! operation are information for the operator, never an input to
//! anything. So are the count-table build's progress lines: each table's
//! entry counts are deterministic, but the two tables build
//! concurrently, so their lines interleave in scheduler order.

use std::path::{Path, PathBuf};
use std::time::Instant;

use before_fuelscape::dump::{self, DumpWriter};
use before_fuelscape::ops::ROSTER;
use before_fuelscape::plan::{run_op, Plan, Samplers};
use before_fuelscape::render::{render_gallery, render_op, AtlasData, RenderMeta};
use fuzzfit_harness::wasm::Guest;

fn main() {
    let mut plan = Plan {
        base_seed: 0xa71a5,
        samples_per_column: 300,
        max_bytes: 256,
    };
    let mut out = PathBuf::from("target/fuelscape");
    let mut dump_measurements = false;
    let mut render_from: Option<PathBuf> = None;
    let mut font_scale = 1.0f64;
    let mut measuring_flags = false;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .unwrap_or_else(|| panic!("flag {flag} needs a value"))
        };
        match flag.as_str() {
            "--samples" => {
                plan.samples_per_column = value().parse().expect("--samples <count>");
                measuring_flags = true;
            }
            "--max-bytes" => {
                plan.max_bytes = value().parse().expect("--max-bytes <bytes>");
                measuring_flags = true;
            }
            "--seed" => {
                plan.base_seed = value().parse().expect("--seed <u64>");
                measuring_flags = true;
            }
            "--out" => out = PathBuf::from(value()),
            "--dump" => dump_measurements = true,
            "--render-from" => render_from = Some(PathBuf::from(value())),
            "--font-scale" => font_scale = value().parse().expect("--font-scale <f64>"),
            other => panic!("unknown flag {other} (see the module doc for the flag list)"),
        }
    }

    if let Some(dump_path) = render_from {
        assert!(
            !measuring_flags && !dump_measurements,
            "--render-from replays a recorded dump; the measuring flags \
             (--samples/--max-bytes/--seed) and --dump configure a measuring run"
        );
        render_from_dump(&dump_path, &out, font_scale);
        return;
    }

    let meta = RenderMeta {
        commit: std::env::var("FUELSCAPE_TIP").unwrap_or_else(|_| "untracked".into()),
        base_seed: plan.base_seed,
        samples_per_column: plan.samples_per_column,
    };
    std::fs::create_dir_all(&out).expect("output directory must be creatable");
    let mut writer = dump_measurements
        .then(|| DumpWriter::new(&out, meta.clone()).expect("dump index must be writable"));

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

    // Compile the guest module now (a process-wide, one-time cost inside
    // the first guest construction) rather than lazily under the first
    // operation's parallel workers, so every printed per-op wall is a
    // sampling number.
    let t0 = Instant::now();
    drop(Guest::new());
    println!("guest module compiled: {:.1?}", t0.elapsed());

    let mut rendered = Vec::new();
    for op in ROSTER {
        let t0 = Instant::now();
        let atlas = run_op(&plan, &samplers, op);
        let measured = t0.elapsed();
        let rejected: u64 = atlas.samples.iter().map(|s| s.rejected).sum();
        let accepted = atlas.samples.len() as u64;
        let data = AtlasData::from_atlas(&atlas);
        let path = render_op(&data, &meta, &out, font_scale).expect("render must succeed");
        if let Some(writer) = writer.as_mut() {
            writer.append(&data).expect("dump must be writable");
        }
        println!(
            "{}: {} samples / {} columns, {} overlay points, acceptance {:.1}%, {:.1?} → {}",
            op.name,
            atlas.samples.len(),
            plan.columns(op.inputs.min_bytes()).len(),
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

/// Replay rendering from a dump: no guest, no count tables, no sampling.
fn render_from_dump(dump_path: &Path, out: &Path, font_scale: f64) {
    let (meta, atlases) = dump::read(dump_path).expect("dump must load");
    std::fs::create_dir_all(out).expect("output directory must be creatable");
    let mut rendered = Vec::new();
    for data in &atlases {
        let path = render_op(data, &meta, out, font_scale).expect("render must succeed");
        println!(
            "{}: {} samples, {} overlay points → {}",
            data.op_name,
            data.samples.len(),
            data.overlay.len(),
            path.display()
        );
        rendered.push((data.op_name.clone(), path));
    }
    let gallery = render_gallery(&rendered, &meta, out).expect("gallery must render");
    println!("gallery → {}", gallery.display());
}
