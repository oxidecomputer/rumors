//! The fuelscape runner: sample every roster operation over the size grid,
//! measure fuel in the fuzz-fit guest, render the per-op heatmaps and the
//! gallery — or replay rendering from a persisted dump without measuring.
//!
//! Positional arguments filter the roster the way `cargo bench` filters
//! benchmarks: each argument selects every operation whose name contains
//! it as a substring, and the run surveys the union (roster order, no
//! duplicates). No arguments surveys the whole roster. A filter that
//! matches no operation is an error naming the available operations —
//! never a silent empty run ([`before_fuelscape::select`] owns and pins
//! these semantics).
//!
//! The flags, their defaults, and which combine with which are
//! documented by the command itself: `--help` (the [`Args`] struct is
//! their single source). Three modes share the binary: a measuring
//! survey (the default), `--render-from <dump>` (replay a dump's SVGs
//! and gallery, no guest), and `--compact-from <dump>` (derive the
//! compact widget dataset `before`'s doc build consumes, no guest).
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
//! anything. The count tables build concurrently, one progress bar
//! each; their entry counts are deterministic.
//!
//! Panels sample [`CONCURRENT_PANELS`] at a time, each with a progress
//! sub-bar under a run-total bar (tty only; a redirected run keeps the
//! plain lines). Completion lines print in completion order — panel
//! wall times overlap — while the gallery and the dump index keep
//! roster order, and the measurements themselves stay order-free.

use std::path::{Path, PathBuf};
use std::time::Instant;

use before_fuelscape::dump::{self, DumpWriter};
use clap::Parser;

/// The sampler is allocator-bound under the system allocator;
/// mimalloc's per-thread heaps keep the workers on the cores.
///
/// The storm is a fresh wasmtime store per sample and big-integer
/// temporaries per rejection draw, across every worker, against one
/// global malloc mutex. Fuel readings are indifferent: the meter
/// counts guest instructions. mimalloc's C runtime faults on illumos
/// (startup segfault), so the gate matches the manifest's: an illumos
/// launcher preloads libumem for the same relief.
#[cfg(not(target_os = "illumos"))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

use before_fuelscape::ops::ROSTER;
use before_fuelscape::plan::{run_op_with_progress, Plan, Samplers};
use before_fuelscape::render::{render_gallery, render_op, AtlasData, RenderMeta, RunParams};
use before_fuelscape::select::{listing, select};
use fuzzfit_harness::wasm::Guest;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Panels sampled concurrently.
///
/// Each panel's own samples already fan out on the shared rayon pool,
/// so this bounds only how many panels' serial phases (overlay points,
/// render, dump append) overlap — and how many sub-bars the progress
/// display carries at once.
const CONCURRENT_PANELS: usize = 1;

/// Survey the roster's fuel landscape, or replay a recorded dump.
///
/// The mode exclusions (a replay or a compaction takes no measuring
/// flags) are declared per argument, so clap reports a conflicting
/// invocation instead of a mode silently ignoring a flag.
#[derive(Parser)]
struct Args {
    /// Substring filters selecting roster operations, `cargo bench`
    /// style (union, roster order); none selects the whole roster
    #[arg(conflicts_with_all = ["render_from", "compact_from"])]
    filters: Vec<String>,
    /// Print the selected operations' names, one per line, without
    /// measuring
    #[arg(long, conflicts_with_all = ["render_from", "compact_from"])]
    list: bool,
    /// Samples per size column, on average
    #[arg(long, default_value_t = 300, conflicts_with_all = ["render_from", "compact_from"])]
    samples: usize,
    /// Top of the size grid, in packed input bytes
    #[arg(long, default_value_t = 256, conflicts_with_all = ["render_from", "compact_from"])]
    max_bytes: usize,
    /// Base seed of the deterministic per-cell RNG streams
    #[arg(long, default_value_t = 0xa71a5, conflicts_with_all = ["render_from", "compact_from"])]
    seed: u64,
    /// Output directory
    #[arg(long, default_value = "target/fuelscape")]
    out: PathBuf,
    /// Persist every selected operation's raw atlas as JSON beside the
    /// SVGs (the dump layout `--render-from` and `--compact-from` read)
    #[arg(long, conflicts_with_all = ["render_from", "compact_from"])]
    dump: bool,
    /// Skip measurement: load a dump (its atlas.json or the directory
    /// holding it) and render its SVGs and gallery into --out
    #[arg(long, value_name = "DUMP")]
    render_from: Option<PathBuf>,
    /// Skip measurement: load a dump and write the compact widget
    /// dataset (the layout before's doc build consumes) into --out
    #[arg(long, value_name = "DUMP", conflicts_with_all = ["render_from", "font_scale"])]
    compact_from: Option<PathBuf>,
    /// Scale all SVG text, for print
    #[arg(long, default_value_t = 1.0)]
    font_scale: f64,
}

fn main() {
    let args = Args::parse();
    let plan = Plan {
        base_seed: args.seed,
        samples_per_column: args.samples,
        max_bytes: args.max_bytes,
    };
    let out = args.out;

    if let Some(dump_path) = args.compact_from {
        let ops = before_fuelscape::compact::compact_dump(&dump_path, &out)
            .expect("compaction must succeed");
        println!("{} operations → {}", ops.len(), out.display());
        return;
    }

    if let Some(dump_path) = args.render_from {
        render_from_dump(&dump_path, &out, args.font_scale);
        return;
    }
    let font_scale = args.font_scale;
    let dump_measurements = args.dump;

    // A filter that matches nothing must never become a silent empty
    // survey; `select` errors with the roster's names instead.
    let selected = select(ROSTER, &args.filters).unwrap_or_else(|no_match| {
        eprintln!("{no_match}");
        std::process::exit(1);
    });
    if args.list {
        print!("{}", listing(&selected));
        return;
    }

    let meta = RenderMeta {
        commit: std::env::var("FUELSCAPE_TIP").unwrap_or_else(|_| "untracked".into()),
        base_seed: plan.base_seed,
        samples_per_column: plan.samples_per_column,
    };
    std::fs::create_dir_all(&out).expect("output directory must be creatable");
    let writer = dump_measurements
        .then(|| DumpWriter::new(&out, meta.clone()).expect("dump index must be writable"));

    let bars = MultiProgress::new();
    let bar_style = ProgressStyle::with_template("{msg:24} {wide_bar} {pos}/{len}")
        .expect("the progress template is well-formed");

    // The two count tables build concurrently, one bar each; lengths
    // arrive with the first callback. Entry counts are deterministic;
    // elapsed is operator information.
    let t0 = Instant::now();
    let table_bar = |name: &str| {
        let bar = bars.add(ProgressBar::new(0));
        bar.set_style(bar_style.clone());
        bar.set_message(format!("{name} table"));
        bar
    };
    let version_table = table_bar("version");
    let party_table = table_bar("party");
    let samplers = Samplers::build_with_progress(&plan, |table, done, total| {
        let bar = match table {
            "version" => &version_table,
            _ => &party_table,
        };
        bar.set_length(total as u64);
        bar.set_position(done as u64);
    });
    version_table.finish_and_clear();
    party_table.finish_and_clear();
    bars.suspend(|| {
        println!(
            "count tables to {} bytes: {:.1?}",
            plan.max_bytes,
            t0.elapsed()
        )
    });

    // Compile the guest module now (a process-wide, one-time cost inside
    // the first guest construction) rather than lazily under the first
    // operation's parallel workers, so every printed per-op wall is a
    // sampling number.
    let t0 = Instant::now();
    drop(Guest::new());
    bars.suspend(|| println!("guest module compiled: {:.1?}", t0.elapsed()));

    // The panel pool: CONCURRENT_PANELS workers pull selected rows off a
    // shared cursor; each panel's samples fan out on the global rayon
    // pool underneath. One sub-bar per in-flight panel, one total bar
    // over every bulk sample in the run; completion lines print above
    // the bars in completion order (the gallery keeps roster order).
    // On a non-tty the bars draw nothing and the lines remain.
    // Progress is denominated in predicted work — column bytes per
    // sample, summed — not sample counts: per-sample cost is ~linear
    // in column bytes across the roster, so a byte-weighted bar moves
    // at a roughly stationary rate and its ETA holds steady, where a
    // sample-counting bar reads optimistic through every panel's small
    // columns and re-learns the ramp on the large ones.
    let work_units: Vec<u64> = selected
        .iter()
        .map(|op| {
            let min = op.inputs.min_bytes();
            plan.columns(min)
                .into_iter()
                .map(|size| (plan.samples_for(size, min) * size) as u64)
                .sum()
        })
        .collect();
    let total = bars.add(ProgressBar::new(work_units.iter().sum()));
    total.set_style(
        ProgressStyle::with_template(
            "{msg:24} {wide_bar} {percent}%  {elapsed_precise} (eta {eta})",
        )
        .expect("the progress template is well-formed"),
    );
    total.set_message(format!("total ({} panels)", selected.len()));
    let panel_style = ProgressStyle::with_template("{msg:24} {wide_bar} {percent}%")
        .expect("the progress template is well-formed");

    let writer = std::sync::Mutex::new(writer);
    let rendered = std::sync::Mutex::new(vec![None; selected.len()]);
    let cursor = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..CONCURRENT_PANELS.min(selected.len()) {
            scope.spawn(|| loop {
                let i = cursor.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(op) = selected.get(i).copied() else {
                    break;
                };
                let bar = bars.insert_before(&total, ProgressBar::new(work_units[i]));
                bar.set_style(panel_style.clone());
                bar.set_message(op.name);
                let t0 = Instant::now();
                let atlas = run_op_with_progress(&plan, &samplers, op, |size| {
                    bar.inc(size as u64);
                    total.inc(size as u64);
                });
                let measured = t0.elapsed();
                let rejected: u64 = atlas.samples.iter().map(|s| s.rejected).sum();
                let accepted = atlas.samples.len() as u64;
                let data = AtlasData::from_atlas(&atlas);
                let path =
                    render_op(&data, &meta, &out, font_scale).expect("render must succeed");
                if let Some(writer) = writer.lock().expect("no panicked holder").as_mut() {
                    writer.append(&data).expect("dump must be writable");
                }
                // suspend, not MultiProgress::println: println writes
                // to the draw target, which a redirected run hides —
                // the lines must survive in a nohup log.
                bars.suspend(|| {
                    println!(
                        "{}: {} samples / {} columns, {} overlay points, acceptance {:.1}%, {:.1?} → {}",
                        op.name,
                        atlas.samples.len(),
                        plan.columns(op.inputs.min_bytes()).len(),
                        atlas.overlay.len(),
                        100.0 * accepted as f64 / (accepted + rejected) as f64,
                        measured,
                        path.display()
                    )
                });
                bar.finish_and_clear();
                rendered.lock().expect("no panicked holder")[i] =
                    Some((op.name.to_string(), path));
            });
        }
    });
    total.finish_and_clear();
    let rendered: Vec<(String, PathBuf)> = rendered
        .into_inner()
        .expect("no panicked holder")
        .into_iter()
        .map(|slot| slot.expect("every selected row ran"))
        .collect();
    let gallery =
        render_gallery(&rendered, &RunParams::from(&meta), &out).expect("gallery must render");
    println!("gallery → {}", gallery.display());
}

/// Replay rendering from a dump: no guest, no count tables, no sampling.
fn render_from_dump(dump_path: &Path, out: &Path, font_scale: f64) {
    let (params, atlases) = dump::read(dump_path).expect("dump must load");
    std::fs::create_dir_all(out).expect("output directory must be creatable");
    let mut rendered = Vec::new();
    for (meta, data) in &atlases {
        let path = render_op(data, meta, out, font_scale).expect("render must succeed");
        println!(
            "{}: {} samples, {} overlay points → {}",
            data.op_name,
            data.samples.len(),
            data.overlay.len(),
            path.display()
        );
        rendered.push((data.op_name.clone(), path));
    }
    let gallery = render_gallery(&rendered, &params, out).expect("gallery must render");
    println!("gallery → {}", gallery.display());
}
