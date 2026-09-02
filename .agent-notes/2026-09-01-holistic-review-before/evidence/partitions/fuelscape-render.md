# Partition fuelscape-render: The fuelscape rendering and dataset pipeline: render, compact, dump, the two binaries, the rustdoc widget script and stylesheet, and build.rs

## Partition summary

This partition is the audit-view half of the population atlas. `render.rs` turns an operation's measured samples into a `HeatGrid` (log-log bin geometry over fuel and input size, one histogram per size column) and draws one SVG per operation plus a gallery page with plotters. `dump.rs` persists each operation's raw samples, overlay points, provenance, and its grid as one JSON document per operation with an index, and reads such a dump back strictly, recomputing the grid and refusing disagreement. `compact.rs` derives the committed widget datasets (`crates/before/fuelscape/*.json`: per-column log-fuel histograms at `RES` octaves per bin, stamped with the roster row's contract and claim strings) from a dump, and reads them back with the same strictness. `bin/fuelscape.rs` is the clap-driven runner with three modes (measure, replay a dump, compact a dump); `bin/spanbands.rs` is a one-off native-counter CSV emitter. `build.rs` in `before` re-reads the committed compact documents, formats them into single-line `<details>` islands for the `# Complexity` sections, pins the derived rustdoc header to the stylesheet-plus-script concatenation, and derives the theme-reactive space-consumption figure. `fuelscape.js` and `fuelscape.css` are the dependency-free browser widget: expression grammar, acceptance rule, density rendering, quantile probe, guides, keyboard access, and a client-side typesetting pass over complexity code spans.

The load-bearing instruments are sound and, in places, exemplary. The dump reader recomputes every stored grid from the raw samples and refuses a mismatch, with a committed tamper demonstration; replay from a dump is pinned byte-identical to a direct render across the whole roster; the compact tamper closure constructs seven known-bad documents and asserts the loader names each check; the header is held to exact equality with its sources (verified fresh at HEAD with `cmp`); the widget refuses a dataset without a positive `res`; and `tools/fuelscape-claims` parses every committed claim with the widget's own exported grammar rather than a reimplementation. Read-only measurement of the committed artifacts confirms the design holds in the data: 104 operation documents in both the dump and the compact dataset, 100 stamped f77011e3 and 4 stamped 46eb64f9, both ancestors of HEAD, no plain `atlas.json` shadowing the committed `.gz`, no zero-fuel sample or zero overlay point anywhere.

The findings cluster around seams the pins do not reach. Six are medium: the measurement commit is stamped from `git rev-parse HEAD` with no dirty-tree guard, falls back to the literal `untracked`, and no reader checks its shape; the dump format was redesigned to accrete but the only writer truncates the index, so the one accretion on record was a hand merge; the client-side typesetting pass runs on every workspace crate's rustdoc pages while the justfile says the script is inert off `.fuelscape` elements; the cross-platform grid pin's docstring names a sensitivity mechanism it does not have (by replicating its fixture, every hashed value except one `log2` is exact on any libm), and no known-bad demonstration exists; a scoped-thread panel pool with two mutexes and an atomic cursor serves a constant pinned to 1 fourteen minutes after it landed at 4; and both `FORMAT_VERSION` docs narrate the layout they replaced, which the root hard rules forbid. The lows are real but bounded: a CLI value reaches an assert documented as programmer error, the positivity of log-scaled quantities is enforced at three different places three different ways, `compact_dump` never runs `validate` on what it writes, `build.rs` omits two of its inputs from `rerun-if-changed` and validates untyped JSON values, a five-column smoothing kernel produces NaN for one- or two-column datasets, and several enumerated strictness rejections have no committed known-bad case. The rest are vocabulary, duplication, and prose nits, batched.

I read all eleven partition files in full with line numbers: 4,585 lines (`render.rs` 641, `render/tests.rs` 195, `compact.rs` 412, `compact/tests.rs` 275, `dump.rs` 306, `dump/tests.rs` 200, `bin/fuelscape.rs` 317, `bin/spanbands.rs` 170, `fuelscape.js` 1580, `fuelscape.css` 155, `build.rs` 334). The three `tests.rs` files are the test surfaces; `render/tests.rs` and `dump/tests.rs` drive the fuzz-fit guest. I also read the justfile's fuelscape and docs recipes, `crates/before-fuelscape/Cargo.toml`, `tools/fuelscape-claims`, the crate's `lib.rs`, the relevant excerpts of `plan.rs`, `ops.rs`, and `sample.rs`, the design note under `.agent-notes/2026-08-13-before-fuelscape-rustdoc/`, the typst consumer in `.agent-notes/2026-07-27-skyline-exposition/07-machine.typ`, the CI workflow's tier comments, plotters-svg 0.3.7 and rand_core 0.6.4 sources from the cargo registry, and the eleven commits the history pass cites (read-only git only). Programs run: `node` on the extracted `smoothSeries`, python over the committed datasets, the dump, and the platform pin's fixture generator, `wc -c` and `cmp` on the header. No cargo, just, build, or test command was run; nothing under the repository was modified.

## Findings

### fuelscape-render-1: Vocabulary: "guest-minted", "two-ways seam", "Honesty rule"/"honest", "backstop"
- Where: crates/before-fuelscape/src/render.rs:133-136 (related: crates/before-fuelscape/src/plan.rs:19, crates/before-fuelscape/src/plan.rs:168, crates/before-fuelscape/src/lib.rs:19, crates/before-fuelscape/src/ops.rs:105, :185, :800, :2448, :2507; crates/before-fuelscape/src/dump.rs:40-44; crates/before-fuelscape/src/dump/tests.rs:54; crates/before/build.rs:5; crates/before/docs/fuelscape.js:7-10, :553, :873-874, :1517)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep -rn -i mint over crates/before-fuelscape/src; grep two-ways over crates/ tools/ justfile; grep honest/backstop in fuelscape.js); executed: no
- Seen by: structure-prose [45], scaffolding [17]; refutation: confirmed; history: contradicts writing-style.md (:170 bans "mint"; :306-311 names "the seam" as metaphor promotion; :326-330 moralized code; :393 translates "backstop")
- Owner-gated: no

The brief's vocabulary rule and writing-style.md forbid "mint" for constructing a value outright; "two-ways seam"/"two-ways pin" compresses the owner's doctrine into a phrase no identifier anchors and is used as if the reader already shares it; "Honesty rule", "honest linear density", and "honest clicks" describe code by a moral rather than by the property that holds; "backstop" is a register transplant where no adversary exists.

Evidence:

       133	        // One-operand rows take the whole column size (the party-fold
       134	        // row's single party included: its shares are guest-minted, not
       135	        // input bytes); everything else plots a total (the stamp carries
       136	        // the row's exact measure declaration).

    dump.rs:
        40	//! recomputed from the raw samples by [`aggregate`]. The grid check is
        41	//! the two-ways seam: the grid is derivable data persisted for

    fuelscape.js:
         7	// Honesty rule: instruction counts are WASM operations metered in a
       553	    // capture dropped) must still end the drag: blur is the backstop.

Resolution: "guest-minted" to "produced in the guest" (and the other mint forms in plan.rs, ops.rs, lib.rs to construct/produce/derive); "the two-ways seam" to a sentence that defines the check ("the grid is recomputed from the samples and must equal the stored one"), and "two-ways pin" likewise where it appears; "Honesty rule" to "Presentation rule", "honest linear density" to "linear density", "most honest clicks" to "most ordinary clicks"; "blur is the backstop" to "blur ends any drag the browser never releases". Acceptance: no "mint" in before-fuelscape prose; "two-ways" appears only in a sentence that defines it; fuelscape.js contains neither "honest" nor "backstop".

### fuelscape-render-2: nit: long qualified paths at use sites, and a `use` block split by the allocator static
- Where: crates/before-fuelscape/src/render.rs:137-138 (related: crates/before-fuelscape/src/compact.rs:247; crates/before-fuelscape/src/render/tests.rs:54; crates/before-fuelscape/src/bin/fuelscape.rs:46-67, :131, :239-245; crates/before-fuelscape/src/bin/spanbands.rs:137-139)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [14], structure-prose [44]; refutation: confirmed; history: no rationale (blame shows the split `use` block is an insertion artifact of cd024f37/7744a717)
- Owner-gated: no

The doctrine prefers imports over long qualified paths except where the qualification informs; none of these sites disambiguates anything, and in `bin/fuelscape.rs` the `use` items sit at 46-47 and 62-67 around the `#[global_allocator]` static, which rustfmt never reorders.

Evidence:

       137	        let unary = matches!(atlas.op.inputs, crate::ops::Inputs::Packed(operands) if operands.len() == 1)
       138	            || matches!(atlas.op.inputs, crate::ops::Inputs::PartyShares);

    bin/fuelscape.rs:
       131	        let ops = before_fuelscape::compact::compact_dump(&dump_path, &out)
       239	    let writer = std::sync::Mutex::new(writer);
       241	    let cursor = std::sync::atomic::AtomicUsize::new(0);

Resolution: import `Inputs`, `ROSTER`, `compact_dump`, `Mutex`, `AtomicUsize`, `Ordering`, and `cmp::Ordering` at the top of each file; move the allocator static below one contiguous `use` block in `bin/fuelscape.rs`. Acceptance: no `crate::ops::`, `std::sync::`, or `core::cmp::` paths in function bodies of the listed files; one contiguous `use` block in `bin/fuelscape.rs`.

### fuelscape-render-3: `aggregate`'s "programmer error" panic is reachable from `--max-bytes`
- Where: crates/before-fuelscape/src/render.rs:211-222 (related: crates/before-fuelscape/src/plan.rs:51-59, :376-387; crates/before-fuelscape/src/bin/fuelscape.rs:95-97, :253-262; crates/before-fuelscape/src/ops.rs:118-128, :1718-1725; crates/before-fuelscape/src/render/tests.rs:16-17)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read plan.rs, ops.rs, the runner loop); executed: no
- Seen by: structure-prose [38], instrument-correctness [53]; refutation: confirmed; history: no rationale found
- Owner-gated: no

Panics are for programmer error only, and every `# Panics` premise must be a one-line proof. `Plan::columns(min_bytes)` returns an empty list whenever `max_bytes < min_bytes` (plan.rs:53-57 loops `while n <= self.max_bytes` from `min_bytes`), `run_op_with_progress` then yields zero samples, and the panel loop calls `render_op` unconditionally (bin/fuelscape.rs:261-262), so a CLI value trips the assert whose doc says every roster row has at least one column; nothing validates `--max-bytes` against `Inputs::min_bytes` (4 for the four-operand rows such as `own_span_contains`, ops.rs:1718-1725).

Evidence:

       211	/// # Panics
       212	///
       213	/// Panics if the atlas has no samples: every roster row has at least
       214	/// one column, and the dump loader rejects empty sample lists, so an
       215	/// empty atlas here is a programmer error.
       216	pub fn aggregate(data: &AtlasData) -> HeatGrid {
       ...
       222	    assert!(!sizes.is_empty(), "an atlas without samples cannot render");

Resolution: after `select` in `main`, reject a plan whose `max_bytes` is below the largest `min_bytes` among the selected rows with a message naming the row and its minimum (or make `Plan` construction fallible), so the `# Panics` premise becomes true by construction. Acceptance: `just fuelscape --max-bytes 3 own_span_contains` exits nonzero naming the 4-byte minimum instead of panicking in `aggregate`.
Construction: `own_span_contains` is `Inputs::Packed` with four operands, so `min_bytes()` is 4; run `FUZZFIT_GUEST_WASM=... cargo run --release --bin fuelscape -- --max-bytes 3 own_span_contains` in crates/before-fuelscape and observe the panic "an atlas without samples cannot render". `--max-bytes 0` reproduces for every row.

### fuelscape-render-4: nit: dead defaults and clamps in `aggregate` and the reference curves, palette hex repeated in the gallery CSS, and load-bearing drops with no comment
- Where: crates/before-fuelscape/src/render.rs:226-239 (related: crates/before-fuelscape/src/render.rs:222, :258, :281-285, :440-459, :586-591, :618-622; plotters-svg-0.3.7/src/svg.rs:131-133, :538-545; plotters-0.3.7/src/drawing/area.rs:119-121)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read the plotters-svg and plotters sources from the cargo registry for the Drop/borrow analysis; arithmetic on the bin index); executed: no
- Seen by: scaffolding [15], adequacy [30], structure-prose [47], instrument-correctness [63], refutation (new nit on line 258); refutation: reframed (the drops are required, not no-ops); history: no rationale (defaults and assert are from the same initial commit)
- Owner-gated: no

Finished code should be obviously correct: after the assert at 222 the sample iterator is nonempty, so `.unwrap_or(1)`/`.unwrap_or(2)` never take their defaults; `counts[idx.min(FUEL_BINS - 1)]` at 258 cannot clamp because `y_hi` sits 0.7 octaves above the largest fuel (`idx = floor(56 - 39.2/range) <= 55` for every range); at 458 `x0 >= 1` (anchor column has `size >= 2`) and steps start at `x0`, so `x0.max(1.0)` and `.max(0.0)` are dead; `render_gallery` repeats `#fcfcfb`/`#0b0b0b`/`#52514e` as literals beside the constants that name them. The four `drop`s at 587-590 are not dead: `SVGBackend<'a>` borrows `path` (`Target::File(String::default(), path.as_ref())`) and implements `Drop`, and `DrawingArea` and `ChartContext` hold `Rc<RefCell<DB>>` clones, so `Ok(path)` cannot move `path` until every holder is dropped; nothing at the site says so.

Evidence:

       226	    let fuel_lo = data
       ...
       231	        .min()
       232	        .unwrap_or(1);
       ...
       258	                counts[idx.min(FUEL_BINS - 1)] += 1;
       ...
       586	    root.present().map_err(draw_err)?;
       587	    drop(chart);
       588	    drop(caption_area);
       589	    drop(chart_area);
       590	    drop(root);
       591	    Ok(path)

Resolution: compute `(fuel_lo, fuel_hi)` in one fold over the asserted-nonempty iterator with no defaults; drop the clamp at 258 and the two `max` calls at 458 (or state at each why it is a guard and against what); format the gallery `<style>` from `SURFACE`, `INK`, `INK_SOFT`; add one comment above the drops ("the SVG backend borrows `path` and its holders have drop glue; end the borrows before moving `path` out"). Acceptance: no unreachable default in `aggregate`; gallery colors derive from the constants; the drops carry their reason; `font_scale_changes_the_svg_and_rendering_is_deterministic` and the dump byte-identity pin still pass.

### fuelscape-render-5: Unanchored design-system vocabulary in the palette constants
- Where: crates/before-fuelscape/src/render.rs:280-292 (related: crates/before-fuelscape/src/render.rs:424-425)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for "reference palette", "categorical slot", "fills-need-spacers" over crates/, tools/, justfile hits only these three sites); executed: no
- Seen by: scaffolding [9]; refutation: confirmed; history: the phrases and hex values are transcribed from the bundled `dataviz` skill's reference documents (`palette.md`, `marks-and-anatomy.md`), which are not in the repository
- Owner-gated: no

Every coined term must be anchored to an identifier or defined at its site; "the reference palette", "categorical slot 2; validated against the ramp's blue", and "the fills-need-spacers rule" name a palette, a validator, and a rule that exist nowhere in the tree, so a reader cannot check what was validated or what the rule requires.

Evidence:

       280	/// Chart surface (light): the reference palette's chart surface.
       ...
       290	/// The adversarial overlay accent (categorical slot 2; validated against
       291	/// the ramp's blue).
       292	const ACCENT: RGBColor = RGBColor(0xeb, 0x68, 0x34);
       ...
       424	                        // A slight vertical inset keeps a visible gap between
       425	                        // occupied bins (the fills-need-spacers rule).

Resolution: state the checkable property in plain terms ("orange, chosen to stay distinguishable from the ramp's blue under common color-vision deficiencies"; "a 6% inset leaves a visible gap between adjacent occupied bins so cells stay countable"), or drop the qualifiers. Acceptance: the three phrases are gone and each color constant's doc names a checkable property or none.

### fuelscape-render-6: Positivity of log-scaled quantities is enforced three ways for fuel and not at all for the size axis
- Where: crates/before-fuelscape/src/render.rs:309-319 (related: crates/before-fuelscape/src/compact.rs:179-188, :226, :387-395, :404-410; crates/before-fuelscape/src/dump.rs:234-243; crates/before/build.rs:297-306; crates/before/docs/fuelscape.js:381; crates/before-fuelscape/src/render/tests.rs:39-43; crates/before-fuelscape/src/plan.rs:53)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read all three readers and the widget); executed: no (a read-only scan confirmed the committed dump and dataset hold no zero fuel or zero size, so every case is latent)
- Seen by: scaffolding [11], instrument-correctness [60], adequacy [23] (overlay half), refutation (new item: zero size axis); refutation: confirmed; history: each treatment individually deliberate, no record chooses a gate
- Owner-gated: no

Correct for all inputs means one policy per condition. A zero fuel reading is a measurement bug (`compact` says so and refuses it), yet `dump::read` accepts it and `lg` floors it to `log2(1)` so the SVG plots the bug at fuel 1 with no signal; `compact::validate` rejects a zero size or fuel on overlay points only, never on the `sizes` axis, and `build.rs` likewise requires the axis to be ascending but not positive, so a document with `sizes: [0, ...]` loads through both readers and the widget computes `Math.log2(0)` for its X domain. Only `Plan::columns` (`min_bytes.max(1)`) keeps measured sizes positive.

Evidence:

       309	/// `log2` with a floor of 1 so a degenerate zero reading cannot produce
       310	/// an infinite coordinate.
       ...
       317	fn lg(v: u64) -> f64 {
       318	    libm::log2(v.max(1) as f64)
       319	}

    compact.rs:
       387	    if op.sizes.is_empty() {
       388	        return reject("the size axis is empty");
       389	    }
       390	    if !op.sizes.windows(2).all(|w| w[0] < w[1]) {
       391	        return reject("the size axis must be strictly ascending");
       392	    }

Resolution: reject zero fuel (samples and overlay points) and zero size once at the format's strict gate, `dump::read` (and `DumpWriter::append`), and add a positivity check on `sizes[0]` to `compact::validate` and `build.rs`'s validator; then either drop the floor in `lg` or document it as unreachable given the gate, and extend the smoke assertion at render/tests.rs:39-43 to overlay points. Acceptance: a dump tamper case setting a sample's fuel to 0 is refused by `read` naming the check; a compact tamper case setting `sizes[0] = 0` is refused; `compact`'s own zero check is then a second line and says so, or goes.
Construction: build an `AtlasData` with one `fuel: 0` sample, `DumpWriter::append` it, `dump::read` it back (accepted), `render_op` it (renders, the point at `log2(1) = 0`), then `compact` it (refused). Separately, in `compact/tests.rs`'s tamper closure set `doc["op"]["sizes"][0] = 0`: `read` accepts today.

### fuelscape-render-7: The cross-platform grid pin's stated sensitivity mechanism is not how it is sensitive, and no known-bad demonstration exists
- Where: crates/before-fuelscape/src/render/tests.rs:131-138 (related: crates/before-fuelscape/src/render/tests.rs:141-173, :182-194; crates/before-fuelscape/src/render.rs:223-224, :240-241, :257-258, :317-319; crates/before-fuelscape/src/dump.rs:237-243; justfile:660-663, :700-703)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (replicated the test's SplitMix64 stream in python: 5,500 samples, `fuel_lo` is 1 with 153 samples reading 1, `fuel_hi` is 140523145363644, not a power of two, range 48.10 octaves, bin height 0.859); executed: yes (the python replication settles the fixture's extremes; the missing demonstration is by reading)
- Seen by: instrument-correctness [61]; refutation: reframed (severity low to medium: the hash's platform sensitivity rides on one `log2` evaluation); history: no rationale found; commit 84a97a39 records the observed mechanism ("the ulp never moved a drawn pixel; only the committed f64 grid bits carried the platform") and names the illumos gate run as the second-architecture check, neither of which the docstring states
- Owner-gated: no

Every criterion needs a committed demonstration that a known-bad mechanism fails it, and a test's doc must be accurate. The docstring says a platform `log2` divergence "anywhere in the log2 range flips at least one bin count and the hash". With `fuel_lo = 1`, `y_lo = -0.4` is exact on every libm; `x_lo`/`x_hi` are `log2` of powers of two (exact); medians are integer arithmetic; a bin count flips only when a sample's `log2` sits within an ulp of a bin edge (about 5,500 chances at roughly 2^-52 per octave, negligible). The only platform-sensitive value in the hashed serialization is `y_hi = lg(140523145363644) + 0.7`, so the pin is one coin flip per host, and no committed test substitutes `f64::log2` for `lg` to show the hash moves. The pin runs only in the local gate (`fuelscape-test`; CI runs `just ci`, which omits it), so cross-architecture agreement is exercised by the illumos gate host, as the commit says and the test does not. The stronger real-data witness is `fuelscape-verify` on ubuntu CI, which recomputes 104 grids measured on illumos and refuses any drift.

Evidence:

       131	/// A dump commits its `HeatGrid`, and the loader re-derives that grid
       132	/// bit-for-bit on whatever machine opens the dump — so the bin geometry
       133	/// must not lean on the platform math library, whose `log2` differs by
       134	/// an ulp across libms exactly at bin boundaries. The sample set below
       135	/// spreads fuel values across ~48 octaves so a platform divergence
       136	/// anywhere in the log2 range flips at least one bin count and the
       137	/// hash. The committed constant was produced by this test's own first
       138	/// run; its value carries no meaning beyond cross-host agreement.

Resolution: restate the mechanism (the serialized f64 domain edges, chiefly `y_hi`, carry the platform; bin flips do not); make the fixture carry more sensitive values (a non-power-of-two smallest fuel and smallest size, so `y_lo`, `x_lo`, `x_hi` are all live `log2` evaluations) and state that the cross-host check of record is the illumos gate run plus `fuelscape-verify` on CI; add the known-bad demonstration where feasible: a test-only shadow of `aggregate` using `f64::log2`, asserting its hash differs from the committed one on at least one documented host, or an explicit statement that no single host can witness the swap. Acceptance: the docstring's mechanism matches the arithmetic; the fixture has no exact-power-of-two extremes; either a committed test shows the `f64::log2` variant hashes differently somewhere, or the docstring says plainly that sensitivity is established by the two-architecture gate run and `fuelscape-verify`, not by this host alone.
Construction: change `lg` to `f64::log2(v.max(1) as f64)` and run `aggregate_bins_identically_on_every_platform` on the development host; if it passes there, the docstring's sensitivity claim is refuted on that host and only the illumos run and `fuelscape-verify` stand between a libm regression and the committed dump.

### fuelscape-render-8: `compact_dump` never runs `validate` on what it writes, and the Strictness paragraph names the wrong consumer of `read`
- Where: crates/before-fuelscape/src/compact.rs:45-53 (related: crates/before-fuelscape/src/compact.rs:226, :243-279, :371, :404-410; crates/before/build.rs:280-282; crates/before-fuelscape/src/bin/fuelscape.rs:131)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (grep for `compact::read` or a `read` import outside compact/tests.rs finds nothing; bin/fuelscape.rs:131 calls only `compact_dump`); executed: no
- Seen by: adequacy [23]; refutation: confirmed; history: the doc-build attribution was a known imprecision (design note §2:104 says the reader is "used by its own round-trip tests"; §2:114-117 says build.rs re-validates); the writer/reader asymmetry has no record
- Owner-gated: no

The cheapest passing artifact must be the intended one: `compact()` copies `data.overlay` through unchecked (226), `compact_dump` calls `dump::read`, `compact`, and `write` and never `validate` (243-279), while `validate` rejects a zero-size or zero-fuel overlay point on read (404-410), so the compactor can write a dataset its own reader refuses and the production path (`fuelscape-verify` diffs bytes only) would not notice. The paragraph's rationale for `read`'s strictness (environmental input to `before`'s doc build) is false as stated: the doc build's reader is `build.rs`, which panics by design; `read`'s consumers are the round-trip and tamper tests.

Evidence:

        45	//! # Strictness
        46	//!
        47	//! A compact dataset is environmental input to `before`'s doc build, so
        48	//! [`read`] rejects malformed data as errors, never panics: unknown
        ...
       226	        overlay: data.overlay.clone(),
       ...
       277	    write(out, &params, &ops)?;

Resolution: call `validate(&path, &op)` on every `WidgetOp` inside `write` (or in `compact_dump` before writing), so the writer's output is by construction what the reader accepts; reword the Strictness paragraph to name `read`'s actual consumers and state that `build.rs` re-checks independently because the detached workspace cannot share the reader. Acceptance: a `compact_dump` test whose dump carries a zero-fuel overlay point fails at compaction naming the overlay check; the module doc no longer attributes `read` to the doc build.
Construction: write a dump via `DumpWriter::append` with `overlay: vec![OverlayData { fuel: 0, size: 2, family: "f".into() }]`; `dump::read` accepts; `compact_dump` succeeds; `compact::read` on the output errors at line 405.

### fuelscape-render-9: Format-version docs narrate the layout they replaced
- Where: crates/before-fuelscape/src/compact.rs:76-82 (related: crates/before-fuelscape/src/dump.rs:65-70; crates/before-fuelscape/src/render.rs:54-61)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [8], adequacy [28], structure-prose [35]; refutation: confirmed; history: contradicts the root hard rule (both docs written by 6f63edb7 as the bump rationale; the commit body records the move fully)
- Owner-gated: no

The root hard rules say nothing in the codebase refers to code that no longer exists, and Principle 5 says dated rationale at a declaration site is the same failure. "Version N moved the measurement commit from the index" narrates the deleted `commit` field of `IndexDoc`; the present-tense invariant already lives at `RunParams` (render.rs:54-61) and on the `IndexDoc` meta fields. Rated medium rather than high: a hard-rule breach by the letter, but the remedy is two sentences and nothing downstream depends on them.

Evidence:

        76	/// The compact format version both banners carry.
        77	///
        78	/// Version 3 moved the measurement commit from the index into each
        79	/// operation document, mirroring the dump format: a dataset accretes
        80	/// across measuring runs, so the index holds only the run parameters
        81	/// every document must share.
        82	const FORMAT_VERSION: u32 = 3;

    dump.rs:
        67	/// Version 2 moved the measurement commit from the index into each
        68	/// operation document: a dataset accretes across measuring runs, so the
        69	/// index holds only the run parameters every document must share.
        70	const FORMAT_VERSION: u32 = 2;

Resolution: keep the first sentence and, if a second is wanted, state the invariant positively without "moved" or "Version N": "each operation document carries its own measurement commit; the index carries only the run parameters every document shares". Acceptance: neither constant's doc names a prior layout.

### fuelscape-render-10: `overlay` is compacted, validated, and committed for a widget revision that does not exist, at a sixth of the dataset's bytes
- Where: crates/before-fuelscape/src/compact.rs:146-148 (related: crates/before-fuelscape/src/compact.rs:21-22, :226, :404-410; crates/before-fuelscape/src/compact/tests.rs:183; crates/before/build.rs:212-222; .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:93-96, :445-446)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (python over the 104 committed documents: overlay arrays are 216,579 of 1,334,551 bytes compact-serialized, 16.2%, about 2.1 KB per operation; grep finds no `overlay` in fuelscape.js and build.rs's island payload omits it); executed: yes (the byte share)
- Seen by: scaffolding [5], structure-prose [49]; refutation: confirmed; history: already-known (a recorded decision in the design note, whose "~1 KB/op" estimate is off by about 2x)
- Owner-gated: yes (a documented design decision in the note)

Circular justification is the tell: the field's stated purpose is to let a later revision draw family marks without a format bump or re-compaction, but re-compaction is the designed one-minute pure derivation (`just fuelscape-compact`) and a bump is a constant increment; meanwhile the field costs a validation branch, a tamper case, and 16% of the committed bytes, and its doc speaks of "later widget revisions" and "the first widget release" instead of the present fact.

Evidence:

       146	    /// The committed adversarial-family points, carried through for
       147	    /// later widget revisions (the first widget release draws none).
       148	    pub overlay: Vec<OverlayData>,

Resolution: either drop `overlay` from `WidgetOp` (with its validate branch and tamper case), bump `FORMAT_VERSION`, and re-derive with `just fuelscape-compact`; or keep it and restate the doc in the present tense with the measured cost ("the widget does not draw these; carrying them, about 2 KB per operation, lets a widget that does read the same document"). Acceptance: `du` of crates/before/fuelscape drops by roughly 215 KB and `just fuelscape-verify` is clean; or the field doc states the present fact and the measured share.

### fuelscape-render-11: nit: `expect` messages that name a hope, not the proof
- Where: crates/before-fuelscape/src/compact.rs:208-212
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Seen by: structure-prose [49]; refutation: confirmed; history: no rationale
- Owner-gated: no

Every `expect` message is a one-line proof; "bin span fits" and "bin offset fits" state the desired outcome, where the proof is `hi >= k0` (max and min over one nonempty list) and `k >= k0`.

Evidence:

       208	            let k0 = *ks.iter().min().expect("every column has a sample");
       209	            let hi = *ks.iter().max().expect("every column has a sample");
       210	            let mut c = vec![0u32; usize::try_from(hi - k0 + 1).expect("bin span fits")];
       211	            for k in ks {
       212	                c[usize::try_from(k - k0).expect("bin offset fits")] += 1;

Resolution: "hi >= k0: max and min of the same nonempty list" and "k >= k0 by construction". Acceptance: both messages state their bound.

### fuelscape-render-12: `compact.rs` and `dump.rs` carry one reader and one writer twice
- Where: crates/before-fuelscape/src/compact.rs:320-375 (related: crates/before-fuelscape/src/compact.rs:92-120, :283-307; crates/before-fuelscape/src/dump.rs:72-103, :152-165, :186-247)
- Class / severity / confidence: modularity / low / high
- Provenance: assessed (read both readers side by side; the string "run parameters differ from the index's" appears at compact.rs:359 and dump.rs:222); executed: no
- Seen by: structure-prose [34]; refutation: confirmed; history: mirroring is deliberate at the format level (design note §2:63-66, "following the dump module's idiom exactly"); code sharing is undiscussed, and 6f63edb7 edited both readers in lockstep
- Owner-gated: no

The two `read` functions are the same body modulo constants and the final per-document check (index-path resolution, `parse`, `check_banner`, empty-ops rejection, per-op parse, banner, `RunParams` uniformity, name agreement), and the `IndexDoc`/`OpDoc` pairs and writers are likewise parallel; a strictness rule added to one (rejecting duplicate op names in the index, say) must be mirrored by hand in the other, and the version-bump history shows they already evolve together. `compact.rs` already imports `dump::{parse, check_banner, malformed, write_atomic}`, so the seam for a shared generic exists.

Evidence:

       356	        if RunParams::from(&doc.meta) != index.meta {
       357	            return Err(dump::malformed(
       358	                &op_path,
       359	                "run parameters differ from the index's",
       360	            ));
       361	        }

    dump.rs:
       219	        if RunParams::from(&doc.meta) != index.meta {
       220	            return Err(malformed(
       221	                &op_path,
       222	                "run parameters differ from the index's",
       223	            ));
       224	        }

Resolution: extract one generic dataset layer in `dump.rs` (or a sibling module): a `read` parameterized by the banner constants and the payload type with a per-document `validate` callback, and the matching writer; `dump` passes the grid check, `compact` passes `validate`; both keep their own constants and payload types. Acceptance: one reader body in the crate; `compact::read` and `dump::read` are thin calls; every existing compact and dump test passes unchanged.

### fuelscape-render-13: Test fixtures duplicated verbatim, and hand-rolled temp dirs that leak on failure
- Where: crates/before-fuelscape/src/compact/tests.rs:9-50 (related: crates/before-fuelscape/src/dump/tests.rs:11-52; crates/before-fuelscape/src/render/tests.rs:28-29, :64, :104-107, :124; crates/before-fuelscape/src/compact/tests.rs:119, :185, :213, :272-274; crates/before-fuelscape/src/dump/tests.rs:129, :160, :199; crates/before-fuelscape/Cargo.toml:82-83)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (compared both `synthetic_atlas` bodies line by line: identical; `temp_dir` differs only in the "before-compact-"/"before-fuelscape-" prefix; every site cleans up only via a trailing `remove_dir_all`); executed: no
- Seen by: scaffolding [13], adequacy [32], structure-prose [36]; refutation: confirmed; history: no rationale (compact/tests.rs copied dump/tests.rs's fixture; `tempfile` is not a dev-dependency)
- Owner-gated: no

Two copies of one fixture drift silently (a change to the roster glyphs the fixture exercises must land twice), and prefer a dependency over hand-rolling: `tempfile::TempDir` cleans up on drop, including on a failing assertion, where the trailing `remove_dir_all` leaves every failed test's directory behind in the system temp dir.

Evidence:

        45	/// A per-test temporary directory, cleaned up by the caller.
        46	fn temp_dir(name: &str) -> std::path::PathBuf {
        47	    let dir = std::env::temp_dir().join(format!("before-compact-{name}-{}", std::process::id()));
        48	    std::fs::create_dir_all(&dir).expect("temp output dir");
        49	    dir
        50	}

Resolution: move `synthetic_atlas` (and a JSON `tamper` helper, see finding 14) into one `#[cfg(test)]` module both suites import; add `tempfile` as a dev-dependency and replace the four temp-dir idioms with `TempDir::new()`, dropping the trailing `remove_dir_all` calls. Acceptance: one `synthetic_atlas` definition; no `std::env::temp_dir()` in the crate's tests; a deliberately failing assertion leaves nothing behind.

### fuelscape-render-14: The tamper test's doc undercounts its cases, and several enumerated rejections in both readers have no known-bad demonstration
- Where: crates/before-fuelscape/src/compact/tests.rs:137-141 (related: crates/before-fuelscape/src/compact/tests.rs:168-183; crates/before-fuelscape/src/compact.rs:45-53, :338-343, :362-370, :387-399; crates/before-fuelscape/src/dump/tests.rs:163-200; crates/before-fuelscape/src/dump.rs:33-44, :197-236)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (counted seven `tamper(...)` calls at 168-183 against the doc's five; `grep -rn unknown crates/before-fuelscape/src/*/tests.rs` is empty); executed: no
- Seen by: scaffolding [10], adequacy [26], structure-prose [37], instrument-correctness [65]; refutation: confirmed; history: the doc's five items are the design note's five (§2:110-111); the body already had seven at introduction (c6d8106a), so the undercount is drift from birth
- Owner-gated: no

Every test's doc must be accurate and every criterion needs a committed demonstration that a known-bad artifact fails it. The doc lists five rejections; the body tampers seven. The module doc promises rejections with no tamper case anywhere: unknown fields (`deny_unknown_fields`, untested in both modules), `op_name` disagreeing with the index, `sizes.len() != cols.len()`, an empty size axis, an empty histogram, and an empty `ops` index. `dump/tests.rs` demonstrates only the grid check, leaving banner, run-parameter drift, name disagreement, empty samples, and unknown fields undemonstrated; any of those branches can be deleted or inverted with the gate green.

Evidence:

       137	/// Tamper with one committed document field and the loader refuses it,
       138	/// naming the file and check: the histogram-tightness, size-order,
       139	/// banner, meta-uniformity, and empty-claim rejections are each alive.
       140	#[test]
       141	fn read_rejects_each_tampered_document() {

Resolution: rewrite the doc as the family ("each structural rejection the module doc enumerates that a single-field edit can reach is alive") and let the `tamper` calls be the list; add the missing cases (`doc["op"]["extra"] = 1` expecting "unknown field"; `doc["op"]["op_name"] = "other"` expecting "index claims"; `doc["op"]["cols"] = []` expecting "one histogram per size column"; `doc["op"]["sizes"] = []` expecting "empty"; `doc["op"]["cols"][0]["c"] = []` expecting "empty"; through the index file, `ops: []`); lift the closure into dump/tests.rs for the dump reader's banner, run-parameter, name, empty-samples, and unknown-field checks. Acceptance: every `malformed(...)`/`reject(...)` site in `dump::read` and `compact::validate`/`read` is reached by a tamper case whose assertion names it; commenting out any one rejection branch turns a test red; neither test doc enumerates by hand.
Construction: comment out dump.rs:225-233 (the op-name check) and run the fuelscape suite: nothing fails today; the proposed `doc["op"]["op_name"] = "other"` case would.

### fuelscape-render-15: The dump format accretes, but the only writer truncates the index, and the one accretion on record was a hand merge
- Where: crates/before-fuelscape/src/dump.rs:118-129 (related: crates/before-fuelscape/src/dump.rs:255-261; crates/before-fuelscape/src/bin/fuelscape.rs:161-162; justfile:680-691; crates/before-fuelscape/src/render.rs:54-61; crates/before-fuelscape/src/lib.rs:47-53; crates/before-fuelscape/src/compact.rs:79-81)
- Class / severity / confidence: feature-gap / medium / high
- Provenance: verified (grep finds no open/append entry in dump.rs or bin/fuelscape.rs; `git show --stat 3c64f08a` touches only data files and doc-attachment lines, and its body says "The dump merge appends the four op documents and their index entries"; a read-only script over the committed dump: 104 ops, 100 stamped f77011e3 and 4 stamped 46eb64f9, no plain `atlas.json` present); executed: no (the central claim, no append path, is by reading)
- Seen by: scaffolding [1], adequacy [22]; refutation: confirmed (and raised [22] to medium to agree); history: no rationale found (6f63edb7's owner ruling covers provenance shape only; the justfile's re-pin text predates it and documents only the full re-measure)
- Owner-gated: no

Every hole found becomes a committed check, never a convention held in memory. The format's own docs describe accretion as the workflow (render.rs:56-61, lib.rs:47-53, compact.rs:79-81) and the dataset did accrete, but `DumpWriter::new` always writes an empty index and `append` extends only its in-memory list, so a filtered `--dump` run into the committed dump directory replaces the index with one naming only the new ops; `parse` prefers a plain `atlas.json` over the committed `atlas.json.gz`, so that truncated index then shadows the committed one on every later read, and the failure surfaces only as 100 "Only in" lines from `fuelscape-verify`'s `diff -r`. The workflow that produced 3c64f08a lives only in that commit's body.

Evidence:

       118	    /// Open a dump in `dir` (created if absent) and write the empty
       119	    /// index.
       120	    pub fn new(dir: &Path, meta: RenderMeta) -> io::Result<DumpWriter> {
       121	        std::fs::create_dir_all(dir)?;
       122	        let writer = DumpWriter {
       123	            dir: dir.to_path_buf(),
       124	            meta,
       125	            ops: Vec::new(),
       126	        };
       127	        writer.write_index()?;
       128	        Ok(writer)
       129	    }
       ...
       258	        Err(e) if e.kind() == io::ErrorKind::NotFound => {
       259	            let mut gz = path.as_os_str().to_owned();
       260	            gz.push(".gz");

Resolution: add `DumpWriter::open(dir, meta)` that loads an existing index with `read`-grade strictness (gz-aware via `parse`), refuses a `RunParams` mismatch and a duplicate op name, and appends; make `new` refuse a directory already holding `atlas.json` or `atlas.json.gz`; have `parse` refuse when both the plain file and its `.gz` sibling exist; wire the open path as `--append-to <dump>` (or make `--dump` open-or-create) and document the accretion recipe (measure alone, gzip, append, `fuelscape-compact`) beside the full re-measure at justfile:680-691. Acceptance: a dump test appends a synthetic op to an existing dump and `read` returns the union in index order with the original documents byte-unchanged; a params-mismatch test and a plain-beside-gz test are refused naming the file; `DumpWriter::new` on a directory holding an index errors.
Construction: in crates/before-fuelscape, `cargo run --bin fuelscape -- version_tick --dump --samples 2 --max-bytes 4 --out dump`, then read `dump/atlas.json`: `ops` is `["version_tick"]` while 104 `.json.gz` documents sit unindexed, and `dump::read("dump")` now sees one op.

### fuelscape-render-16: nit: `write_atomic`'s crash claim outruns its mechanism
- Where: crates/before-fuelscape/src/dump.rs:168-175 (related: crates/before-fuelscape/src/dump.rs:16-18)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [16], adequacy [27], instrument-correctness [62]; refutation: confirmed; history: deliberate-and-holds (the design target is a dying process, stated at dump.rs:16-18 and in eda7cccd; only the word "crash" reads broader)
- Owner-gated: no

A claim should match its mechanism. Write-then-rename is atomic against a process death, which is the stated target; without `sync_all` on the temporary before the rename, a power loss on a filesystem that persists the rename before the data can leave the name pointing at an empty file, which is the torn document the sentence says can never stand there.

Evidence:

       168	/// Write `bytes` to `path` atomically: a sibling temporary file, then a
       169	/// rename, so a crash never leaves a torn document where a whole one
       170	/// stood.
       171	pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
       172	    let tmp = path.with_extension("json.tmp");
       173	    std::fs::write(&tmp, bytes)?;
       174	    std::fs::rename(&tmp, path)
       175	}

Resolution: narrow the doc to "a dying process" (matching dump.rs:16-18), or add `File::create` + `write_all` + `sync_all` before the rename (or use `tempfile::NamedTempFile::persist`) and keep the broader claim. Acceptance: the doc's failure model matches the code.

### fuelscape-render-17: The stored `HeatGrid` has no regrid path, so any change to the aggregation constants makes the committed dump unreadable
- Where: crates/before-fuelscape/src/dump.rs:237-243 (related: crates/before-fuelscape/src/dump.rs:5-8, :25-27; crates/before-fuelscape/src/render.rs:223-224, :240-241, :297, :364; .agent-notes/2026-07-27-skyline-exposition/07-machine.typ:193-194)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (grep for regrid/rebin across the crate and justfile hits only prose at dump.rs:27 and compact.rs:88; `render_op` recomputes `aggregate(data)` at render.rs:364 and never reads the stored grid; the only reader of `atlas.grid` outside the loader is the typst chapter, which reads a frozen v1 snapshot under `.agent-notes/.../data/fuelscape-8k-2000`, 64 files, `"version":1`, commit 2db1a20e); executed: no
- Seen by: scaffolding [0]; refutation: reframed (severity medium to low: the refusal is loud, the raw samples are intact, and the libm routing is required independently by `fuelscape-verify`); history: deliberate-and-holds for persisting and checking the grid (dump.rs:19-27, eda7cccd), with no ruling for or against a regrid path
- Owner-gated: yes (persisting the grid is a documented design decision)

The module promises replay "any font scale, any future styling" with no guest, but `read` refuses any document whose stored grid is not bit-equal to today's `aggregate`, which `FUEL_BINS` and the 0.55/0.4/0.7 axis margins shape; a change to any of them refuses every committed op document, and no tool path rewrites the grids from the samples the dump still holds. For the committed v2 dump the grid's only reader is the loader's own equality check.

Evidence:

       237	        if doc.grid != aggregate(&doc.op) {
       238	            return Err(malformed(
       239	                &op_path,
       240	                "stored grid does not match the grid recomputed from its samples \
       241	                 (the dump was altered, or it predates a change to the aggregation)",
       242	            ));
       243	        }

Resolution: add a `--regrid <dump>` mode (or a `DumpWriter` entry) that rewrites each op document's grid from its samples, so an aggregation change is a recipe beside `fuelscape-compact` rather than a hand migration; or move the grid out of the dump into a derived sidecar the way `compact.rs` already derives the widget form; in either case narrow the module doc's styling promise to what it excludes. Acceptance: changing `FUEL_BINS` and running `--regrid dump` then `--render-from dump` succeeds without hand-editing a committed document, and the dump tests still pin raw-sample round-trip and replay byte-identity.
Construction: change `FUEL_BINS` from 56 to 64 and run `cargo run --bin fuelscape -- --render-from dump --out target/x` in crates/before-fuelscape: every op document is refused with "stored grid does not match" and nothing in the tool repairs it.

### fuelscape-render-18: A panel pool of width 1: scoped threads, an atomic cursor, and two mutexes wrap a sequential loop
- Where: crates/before-fuelscape/src/bin/fuelscape.rs:69-75 (related: crates/before-fuelscape/src/bin/fuelscape.rs:37-41, :206-217, :239-293)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`git show 7744a717` introduces `const CONCURRENT_PANELS: usize = 4;` with a stated purpose; `git show 6d84c8fa`, fourteen minutes later, is a one-line diff to `1` whose whole message is "Fuelscape only samples one panel at a time."); executed: no
- Seen by: scaffolding [3], adequacy [29], structure-prose [33], instrument-correctness [58]; refutation: confirmed; history: no rationale found (no note or later commit says why 1)
- Owner-gated: no (a dev-tool binary; git keeps the pool)

Machinery outlives the constraint that justified it, and legibility matters almost as much as correctness. With the width at 1 the loop over `selected` is sequential in roster order, yet the reader must still verify a `Mutex<Option<DumpWriter>>`, a `Mutex<Vec<Option<_>>>` of result slots, an `AtomicUsize` cursor, per-panel `insert_before` sub-bars, and four `expect` proofs about panicked holders and filled slots; the module doc (37-41) and the loop comment (206-211) describe completion-order printing and overlapping panel wall times that a width of 1 makes impossible, so the prose is not in the present tense about what the code does.

Evidence:

        69	/// Panels sampled concurrently.
        70	///
        71	/// Each panel's own samples already fan out on the shared rayon pool,
        72	/// so this bounds only how many panels' serial phases (overlay points,
        73	/// render, dump append) overlap — and how many sub-bars the progress
        74	/// display carries at once.
        75	const CONCURRENT_PANELS: usize = 1;
       ...
        39	//! plain lines). Completion lines print in completion order — panel
        40	//! wall times overlap — while the gallery and the dump index keep
        41	//! roster order, and the measurements themselves stay order-free.

Resolution: replace the pool with `for (i, op) in selected.iter().enumerate()` holding `writer` and `rendered` directly (no mutexes, no slots, no cursor), delete the constant, and rewrite lines 37-41 and 206-211 for sequential panels whose samples fan out on rayon; or, if 1 is a measured tuning outcome the owner wants to keep as a knob, record the measured reason at the constant. Acceptance: either `std::thread::scope`, `AtomicUsize`, and both `Mutex`es are gone from `main` with the smoke and dump pins unchanged and the gallery in roster order, or the constant's doc names why 1 and the module doc no longer describes overlap that cannot occur.

### fuelscape-render-19: Measurement provenance is stamped unchecked and accepted as any string
- Where: crates/before-fuelscape/src/bin/fuelscape.rs:155-159 (related: crates/before-fuelscape/src/bin/fuelscape.rs:20-24; justfile:678; crates/before/build.rs:64-67; crates/before-fuelscape/src/compact.rs:379-412; crates/before/docs/fuelscape.js:756-758; crates/before-fuelscape/src/render.rs:44-45)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read the stamp, the recipe, and both readers; a read-only scan shows today's committed documents carry two 40-hex commits, both ancestors of HEAD, so the gap is in the checks, not the data); executed: no
- Seen by: adequacy [19]; refutation: confirmed; history: the `untracked` fallback is deliberate for ad-hoc runs (bin/fuelscape.rs:20-24); no record considers a dirty-tree guard or a shape check on the committed path
- Owner-gated: no

Measurements bind to their run. The commit stamp is the only thing binding a measuring run, every committed dataset document, and every rustdoc island footer to the code that was measured, and the accretion design rests on it being meaningful per operation. The recipe stamps `git rev-parse HEAD` with no dirty-tree check while the guest wasm is built from the working tree; the binary falls back to the literal `untracked`; and the only check anywhere is `build.rs`'s `is_string()`, which `untracked` satisfies, so a dataset whose documents all say `"commit": "untracked"`, or name a commit whose tree is not what was measured, passes compaction, `fuelscape-verify`, `build.rs`, and renders in the docs.

Evidence:

       155	    let meta = RenderMeta {
       156	        commit: std::env::var("FUELSCAPE_TIP").unwrap_or_else(|_| "untracked".into()),
       157	        base_seed: plan.base_seed,
       158	        samples_per_column: plan.samples_per_column,
       159	    };

    justfile:
       678	    FUELSCAPE_TIP=$(git rev-parse HEAD) FUZZFIT_GUEST_WASM=... cargo run --release --bin fuelscape -- --out ... {{ args }}

    build.rs:
        64	        assert!(
        65	            doc["meta"]["commit"].is_string(),
        66	            "{file}: the measurement commit is missing"
        67	        );

Resolution: (1) in the `fuelscape` recipe, refuse to run a `--dump` survey when `git diff --quiet HEAD -- crates/before crates/suanpan` fails (or stamp `git describe --always --dirty --abbrev=40` so a dirty tree is visible); (2) make `--dump` refuse to run without `FUELSCAPE_TIP` instead of writing `untracked` (ad-hoc renders may keep the fallback); (3) require the commit to be exactly 40 lowercase hex characters in `compact::validate` and in `build.rs`'s loop, so neither `untracked` nor `<sha>-dirty` can reach the committed dataset. Acceptance: a `--dump` run without `FUELSCAPE_TIP` exits nonzero naming the variable; a tamper line in compact/tests.rs setting `doc["meta"]["commit"] = "untracked"` is rejected naming the check; the same edit to a committed `crates/before/fuelscape/<op>.json` fails `cargo build -p before` naming the file.
Construction: `FUZZFIT_GUEST_WASM=... cargo run --bin fuelscape -- --dump --max-bytes 2 --samples 1 --out <tmp> version_tick` without `FUELSCAPE_TIP`, then `--compact-from <tmp> --out <tmp2>`: the written `version_tick.json` carries `"commit":"untracked"` and passes build.rs's checks verbatim. Or edit one committed document's `meta.commit` to `"untracked"` and run `cargo build -p before`: it succeeds.

### fuelscape-render-20: nit: stringly-typed table callback with a catch-all arm; allocator rationale duplicated with the manifest
- Where: crates/before-fuelscape/src/bin/fuelscape.rs:180-187 (related: crates/before-fuelscape/src/plan.rs:141-158; crates/before-fuelscape/src/bin/fuelscape.rs:49-57; crates/before-fuelscape/Cargo.toml:85-96)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: structure-prose [51]; refutation: confirmed; history: no rationale
- Owner-gated: no

Types first: `Samplers::build_with_progress` passes `&'static str` table names and the binary matches with `_ => &party_table`, so a third table or a typo routes silently to the party bar; the `#[global_allocator]` doc restates Cargo.toml:85-96 nearly verbatim, so one rationale lives in two places.

Evidence:

       180	    let samplers = Samplers::build_with_progress(&plan, |table, done, total| {
       181	        let bar = match table {
       182	            "version" => &version_table,
       183	            _ => &party_table,
       184	        };

Resolution: introduce `pub enum Table { Version, Party }` in plan.rs for the callback so the match is exhaustive; shorten the allocator doc to one sentence pointing at the manifest's dependency comment. Acceptance: no `_ =>` in the table match; the illumos/libumem explanation appears once.

### fuelscape-render-21: `spanbands` is referenced by nothing, records no result, hand-rolls its CLI against the manifest's clap rationale, and alone justifies two `before` features and the `suanpan` dependency
- Where: crates/before-fuelscape/src/bin/spanbands.rs:1-6 (related: crates/before-fuelscape/src/bin/spanbands.rs:20-23, :65-83; crates/before-fuelscape/Cargo.toml:16-26, :36-40)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for "spanbands" across *.rs, *.md, *.toml, *.yml, *.py, *.typ, and the justfile, excluding target and .claude/worktrees, hits only Cargo.toml:19 and the binary; `git log -- crates/before-fuelscape/src/bin/spanbands.rs` shows one commit, 5b2ae58c, whose message states the question; no `.agent-notes` entry records an answer); executed: no
- Seen by: scaffolding [4], adequacy [21], structure-prose [41], [42], instrument-correctness [59]; refutation: confirmed (and verified that `scan-meter`/`limb-meter` and `suanpan/touch-meter` serve only this binary); history: no rationale found; the clap port (c6d8106a, "The runner's flag parsing moves to clap") postdates the binary and wrote the manifest's "runner binaries" rationale without touching it
- Owner-gated: yes (retiring an instrument)

An instrument earns its place by naming what it serves outside itself: a one-off analysis binary with no recipe, test, CI leg, note, or recorded outcome is scaffolding, and its dependency footprint (Cargo.toml:16-26) exists only for it. If it stays, it should meet the crate's own standard: it parses flags by hand with `panic!`/`expect` and keeps a hand-maintained flag list in its module doc while Cargo.toml:36-40 justifies clap for "the runner binaries" precisely against hand-rolled asserts.

Evidence:

         1	//! The span-band discriminator: per-pair native measurements of the
         2	//! pair operations over the `version_span` input space, as raw CSV.
         3	//!
         4	//! The atlas's `version_span` heatmap shows banded conditional work
         5	//! distributions, but a heatmap cannot say *which coordinate of the
         6	//! pair* separates the bands — it plots work against total size alone.
       ...
        81	            other => panic!("unknown flag {other} (see the module doc for the flag list)"),

    Cargo.toml:
        36	# Argument parsing for the runner binaries: per-flag help and defaults
        37	# beside their declarations, and the mode exclusions (a replay or a
        38	# compaction takes no measuring flags) enforced declaratively instead of
        39	# by hand-rolled asserts.

Resolution: owner's call: (a) record the discriminator's result (which pair coordinate separates the `version_span` bands, and what it decided about the classify-first versus emit-always trade) in an agent note or the span kernel's docs, and delete the binary together with the `scan-meter`/`limb-meter` features and the `suanpan` dependency in this manifest; or (b) keep it, port the flags to a clap `Args` struct, add a `just spanbands` recipe with the question stated, and give it a smoke test. Acceptance: either the binary and its three manifest lines are gone and a note holds its result, or `just --list` names it, `--help` derives the flag docs, and `--sizes 64,abc` exits with a clap error rather than a panic.

### fuelscape-render-22: `smoothSeries` indexes past the array for one- or two-column datasets, producing a NaN probe path
- Where: crates/before/docs/fuelscape.js:325-331 (related: crates/before/docs/fuelscape.js:472-476, :1344-1360; crates/before-fuelscape/src/compact.rs:387-395; crates/before-fuelscape/src/plan.rs:51-59)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (ran the function body copied from lines 325-342 under node); executed: yes (`smoothSeries([5],2,0.9)` gives `[NaN]`, `smoothSeries([1,2],2,0.9)` gives `[NaN,NaN]`, a three-element input is finite; python over the committed datasets shows 11 to 13 columns per document, so the case is latent today)
- Seen by: adequacy [25], instrument-correctness [57]; refutation: confirmed (same node result); history: no rationale (the mirror extension arrived with the widget port and has no bounds comment)
- Owner-gated: no

Correct at all scales, for all inputs: with radius 2 the mirror reads `vals[-j]` and `vals[2*(n-1)-j]`, which fall outside the array whenever `n <= radius`; `undefined` propagates as NaN through the weighted sum, `monotonePath` emits NaN coordinates, and the probe trace silently fails to draw. `compact::validate` admits a single-column dataset (`sizes` need only be nonempty and ascending), and `Plan::columns` yields one column whenever `min_bytes <= max_bytes < 2*min_bytes`, so the reader and the consumer disagree on the admissible input.

Evidence:

       325	function smoothSeries(vals, radius, sigma) {
       326	  const n = vals.length;
       327	  const at = j => {
       328	    if (j >= 0 && j < n) return vals[j];
       329	    if (j < 0) return 2 * vals[0] - vals[-j];
       330	    return 2 * vals[n - 1] - vals[2 * (n - 1) - j];
       331	  };

Resolution: clamp the mirror index (`vals[Math.min(-j, n - 1)]` and `vals[Math.max(2 * (n - 1) - j, 0)]`), or return `vals.slice()` when `n <= radius`, matching the raw fallback `traceValues` already takes at the extreme quantiles. Acceptance: a dataset with `sizes: [8]` and one column hydrates with a finite probe path (`d` contains no `NaN`); a node check of the extracted function gives finite output for arrays of length 1 and 2.

### fuelscape-render-23: The probe trace draws a five-column smooth of the quantile while its label, handle, readout, and guide anchor use the raw value
- Where: crates/before/docs/fuelscape.js:472-476 (related: crates/before/docs/fuelscape.js:322-324, :491-500, :1165-1173, :1356, :1379)
- Class / severity / confidence: correctness / low / medium
- Provenance: assessed (read `traceValues`, the trace path from `tv.sm`, the guide crossing from `tv.raw[aI]`, the handle from `tv.raw[li]`); executed: no
- Seen by: instrument-correctness [56]; refutation: confirmed (the geometric inconsistency is a fact; whether smoothing removes signal at 4096 samples per column is a judgment); history: deliberate-but-expired (the smoothing's premise at 322-324 is that the anchor is an endpoint; 2efff149 added interior click-to-anchor without revisiting it)
- Owner-gated: yes (a presentation choice)

The widget's own rule is to present shapes faithfully. For interior quantiles the drawn trace is `smoothSeries(raw, 2, 0.9)` (center weight about 0.44, neighbors 0.24, second neighbors 0.04), while the guide crossing, the slider handle, and the readout use the raw per-column quantile; the mirror extension preserves endpoints, so the default rightmost anchor agrees, but once a reader locks an interior column the handle and the guide crossing sit off the drawn line by the smoothing residual, and a line labeled "median" or "pNN" is not the per-column quantile there. The comment at 1165-1167 promises guides intersect the probe at the anchor column; with an interior anchor they intersect the raw value, not the drawn trace.

Evidence:

       472	  traceValues(q) {
       473	    const raw = this.data.cols.map(col => this.quantAt(col, q));
       474	    const sm = (q <= 0 || q >= 1) ? raw.slice() : smoothSeries(raw, 2, 0.9);
       475	    return { raw, sm };
       476	  }

Resolution: draw the raw quantiles (the natural choice at 4096 samples per column), or keep the smoothing and anchor guides and the handle to `tv.sm`, disclosing it in the y-label or probe tooltip ("median, smoothed across neighboring columns"). Acceptance: with an interior column locked, the slider handle, the active guide's crossing, and the drawn trace coincide at that column, and the label names what is drawn.
Construction: open any island (`version_tick`), select the `n` hypothesis, click the size-64 column to lock it, and compare the handle's y (raw median) with the trace's y at the same x (smoothed); they differ by the five-point residual, largest where the median's slope changes between columns.

### fuelscape-render-24: nit: the pointer-to-viewBox conversion, the quantile-step handler, and the guide-tip scan are each written twice
- Where: crates/before/docs/fuelscape.js:568-569 (related: crates/before/docs/fuelscape.js:596-597, :617-618, :1010-1011, :527-534, :927-936, :1196-1207, :307-308, :343-345)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grep: the `px` conversion `rect.width || W` appears at 568, 596, 1010; the `py` conversion at 569, 597, 618, 1011; the step arithmetic `Math.round((this.q` at 529, 530, 929, 930); executed: no
- Seen by: structure-prose [48]; refutation: confirmed with the count correction; history: no rationale (`quantKey` was written as the shared handler; the slider kept its own copy)
- Owner-gated: no

A change to the viewBox mapping (a device-pixel correction, say) must land in four places; the slider's `keydown` re-implements the ArrowUp/Down stepping `quantKey` exists to share; the tip loop at 1203-1207 recomputes the values the loop at 1196-1202 just stored in `vals`; blank-line runs at 307-308 and 343-345.

Evidence:

       568	      const px = (e.clientX - rect.left) * (W / (rect.width || W));
       569	      const py = (e.clientY - rect.top) * (H / (rect.height || H));

Resolution: add `svgPoint(e)` returning `{px, py}` and use it at the four sites; have the slider handler call `this.quantKey(ev)` first and handle only Left/Right/Home/End/m itself; derive `tip` as `vals.find(([, v]) => v >= ylo + 0.1 && v <= yhi - 0.1)`. Acceptance: one occurrence of `rect.width || W`; the step arithmetic appears only inside `quantKey`.

### fuelscape-render-25: nit: literal `\u2014` escapes inside JavaScript comments, and em-dashes in `//` comments across the partition
- Where: crates/before/docs/fuelscape.js:1055 (related: crates/before/docs/fuelscape.js:1538, :1540, and 22 `//` lines with an em-dash; crates/before-fuelscape/src/render/tests.rs:141; crates/before-fuelscape/src/bin/fuelscape.rs:212-213, :267; crates/before/build.rs:79, :132; crates/before/docs/fuelscape.css:1)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep -n '\\u2014' fuelscape.js: 185 and 717 are inside string literals, where the escape is correct; 1055, 1538, 1540 are inside comments; grep for em-dashes in non-doc `//` comments on the listed files); executed: no
- Seen by: structure-prose [46]; refutation: confirmed; history: contradicts Finch's global doctrine (prefer colons or semicolons over em-dashes in comments; the spaced double-hyphen is the dash of code comments)
- Owner-gated: no

The three `\u2014` sequences are a text defect: a comment never interprets the escape, so the reader sees six characters where a dash was meant. The em-dashes in `//` comments are a register rule from the owner's doctrine; rustdoc `///`/`//!` prose is exempt.

Evidence:

      1054	    // Ratios only: the column's min-to-max spread, and where the median
      1055	    // sits above the column minimum \u2014 both platform-transferable.
       ...
      1538	  // thousands of SVG nodes \u2014 building them all at page load would tax
       ...
      1540	  // near the viewport (even while closed \u2014 an SVG builds fine inside a

Resolution: replace `\u2014` with a real dash or a colon at 1055, 1538, 1540; sweep the listed `//` lines to colons, semicolons, or spaced double-hyphens. Acceptance: grep for `\\u2014` in fuelscape.js comments returns nothing; the listed `//` lines carry no em-dash.

### fuelscape-render-26: `__FS_NO_ANIM` and the `Fuelscape.parse` export have no consumer
- Where: crates/before/docs/fuelscape.js:1105 (related: crates/before/docs/fuelscape.js:1533-1535; tools/fuelscape-claims:25-39; .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:316-317, :449)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for `__FS_NO_ANIM` over rs/js/md/py/justfile, excluding target and the derived header, hits only fuelscape.js:1105 and the design note; tools/fuelscape-claims calls only `Fuelscape.accepts`); executed: no
- Seen by: structure-prose [40]; refutation: confirmed; history: deliberate-but-expired (the tool called `parse` at 61f05692 and 2efff149 switched it to `accepts`; the note keeps the hook as a "test hook" while ruling screenshot pinning a non-goal, and nothing ever set it)
- Owner-gated: no

A hook earns its place by naming the test that flips it, and an export by its caller; the test this hook anticipated was declared a non-goal, and the tool that used `parse` now uses `accepts`.

Evidence:

      1105	    const noAnim = typeof window !== "undefined" && window.__FS_NO_ANIM;
       ...
      1533	const Fuelscape = {
      1534	  parse: parseBound,
      1535	  accepts: acceptBound,

Resolution: delete the `noAnim` branch (keeping the `prefers-reduced-motion` path) and the `parse` export; or land the DOM or node test that uses them and cite it at the hook. Acceptance: grep for `__FS_NO_ANIM` across the tree returns nothing or returns a test; `Fuelscape`'s exports are exactly what tools/fuelscape-claims and the page use.

### fuelscape-render-27: The typesetting pass rewrites code spans on every workspace crate's rustdoc pages, while the justfile says the script activates only on `.fuelscape` elements
- Where: crates/before/docs/fuelscape.js:1504-1511 (related: crates/before/docs/fuelscape.js:1468, :711, :1544-1545, :1575-1579; justfile:249-258; crates/before/Cargo.toml:9-14; src/link.rs:190)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified by reading and grep (hydrate calls `typesetDocMath(scope)` unconditionally and runs at load on every page; `MATH_SHAPED` matches a lone lowercase letter; the header is injected via workspace-wide `RUSTDOCFLAGS`; a census of doc lines carrying a lone-lowercase-letter code span: 79 in rumors `src/`, 15 in `crates/suanpan/src`, 5 in `crates/before-viz/src`; rustdoc emits `data-current-crate` in target/doc/rumors/index.html and target/doc/before/index.html); executed: no (not rendered in a browser)
- Seen by: instrument-correctness [52]; refutation: confirmed; history: deliberate-but-expired (61f05692 wrote the justfile claim when the script was inert off before's pages; 2efff149 added the pass to reach before's own doc prose and no record considers other crates' pages)
- Owner-gated: no

Distinguish what the instrument does from what its documentation says it does: `typesetDocMath` replaces every `.docblock code` element whose whole text is math-shaped (a lone lowercase letter included) with an `<i>` inside `<span class="fs-math">`, on every page the header reaches, so rumors', suanpan's, before-viz's, and rumors-tracing's rendered docs have code identifiers such as `f`, `k`, or `b` re-set as italic math variables with no visible cause, while the justfile tells a reader the script is inert there. docs.rs builds per crate and is unaffected.

Evidence:

      1504	function typesetDocMath(scope) {
      1505	  (scope || document).querySelectorAll(".docblock code").forEach(code => {
      1506	    if (code.closest("pre") || !MATH_SHAPED.test(code.textContent)) return;
      1507	    const span = document.createElement("span");
      1508	    span.className = "fs-math";
      1509	    typesetInto(span, code.textContent);
      1510	    code.replaceWith(span);
      1511	  });
      1512	}

    justfile:
       252	# per-crate rustdocflags — so non-before pages carry ~40 KB of inert
       253	# head weight; the script activates only on .fuelscape elements.

Resolution: gate the pass on before's own pages: the script already reads `meta[name="rustdoc-vars"]` at line 711, and rustdoc stamps `data-current-crate`, so `hydrate` can run `typesetDocMath` only when `vars.dataset.currentCrate === "before"`. If the owner wants the typesetting workspace-wide instead, correct justfile:253 and the Cargo.toml comment at 9-12 to say so and confirm the other crates' authors accept the face change. Acceptance: after `just docs`, rumors' `Link` doc ("Hand the completed half to `f`.", src/link.rs:190) renders `<code>f</code>` intact while before's `# Complexity` sections are still typeset; the justfile comment matches the behavior either way.
Construction: after `just docs`, open the rendered page for `src/link.rs`'s completed-half method: the `f` code span is an `<i>f</i>` inside `<span class="fs-math">`. Or in node with a stub DOM containing `<div class="docblock"><code>k</code></div>`, call `Fuelscape.hydrate()`: the `<code>` is gone.

### fuelscape-render-28: nit: one sans-serif font stack spelled five times where the stylesheet already uses a token
- Where: crates/before/docs/fuelscape.css:54 (related: crates/before/docs/fuelscape.css:8, :85, :118, :142, :146)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep "Fira Sans": lines 54, 85, 118, 142, 146; `--fs-mono` at line 8); executed: no
- Seen by: scaffolding [17]; refutation: confirmed; history: no rationale
- Owner-gated: no

Named constants over magic values applies to CSS tokens: `--fs-mono` shows the idiom; the sans stack is repeated verbatim five times.

Evidence:

        54	.fs-hyplabel { font-size: 11pt; font-weight: 600; font-family: "Fira Sans", Arial, NanumBarunGothic, sans-serif;

Resolution: add `--fs-sans` beside `--fs-mono` at `:root` and use it at the five sites. Acceptance: one definition of the sans stack.

### fuelscape-render-29: `build.rs`'s module doc describes one of its two jobs and denies a duplication the script deliberately carries
- Where: crates/before/build.rs:1-21 (related: crates/before/build.rs:38, :54, :93-100, :103-197, :272-278, :280-316; crates/before-fuelscape/src/compact.rs:71-82, :404-410; .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:50-52, :97-100)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [2], [7], adequacy [31], structure-prose [43], instrument-correctness [64]; refutation: confirmed, severity medium to low (the duplication is forced by the detached workspace and today's data passes both readers); history: deliberate-and-holds (the design note §1 forbids build.rs depending on anything outside the package, so a shared crate would itself have to be published; the sentence was written with §2's binning-constant meaning)
- Owner-gated: no (the leaf-crate alternative would be)

A module doc's first sentence stands alone in a listing, and prose states what is. The doc enumerates inputs and outputs for the islands only, while lines 93-197 (`theme_svg`, `Ink`, `check_readme_figure_fresh`) read `results/space_consumption/itc_space_consumption.svg` and write `$OUT_DIR/space_consumption.svg` and `docs/itc_space_consumption_readme.svg`, none of which the doc names; and the "holds no constants the widget or compactor also hold" sentence is contradicted by `check_banner`'s literal `3` and `v3` and the banner strings at 38 and 54, which restate compact.rs's `FORMAT_VERSION`, `INDEX_FORMAT`, and `OP_FORMAT`. The two validators have also drifted: build.rs checks the commit is a string and skips the overlay check; `compact::validate` checks overlay points and nothing about the commit.

Evidence:

         1	//! Formats the committed fuelscape widget datasets into rustdoc islands.
       ...
        17	//! This script is a pure formatter: it re-bins nothing, computes no
        18	//! statistics, and holds no constants the widget or compactor also
        19	//! hold. Every failure here is a defect in the committed repository
       ...
       274	        (doc["format"].as_str(), doc["version"].as_u64()),
       275	        (Some(expected), Some(3)),
       276	        "{file}: not a {expected} v3 document"

Resolution: open the doc with both responsibilities and add a second inputs/outputs paragraph naming the three figure paths (or split the figure job into `mod figure;` with its own doc); reword the sentence to "holds no binning or statistical constant; the format banners and version are the one deliberate duplication, spelled on both sides of the package boundary"; hoist `const FORMAT_VERSION: u64 = 3;` and the two banner strings so the check and its message read one constant; and either add build.rs's overlay-positivity check or state at 280-282 which of the compactor's checks are deliberately not repeated. Acceptance: the module doc names every file build.rs reads and writes; no bare `3`/"v3" in `check_banner`; the doc names the duplicated identifiers.

### fuelscape-render-30: `build.rs` reads two figure files it does not declare in `rerun-if-changed`, so the README-figure freshness check is dormant under incremental builds
- Where: crates/before/build.rs:27-30 (related: crates/before/build.rs:93-94, :175-197; justfile:710-711)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; cargo's rule that once any `rerun-if-changed` is emitted only listed paths and env vars retrigger the script is documented behavior); executed: no
- Seen by: scaffolding [6], adequacy [20], structure-prose [39], instrument-correctness [54]; refutation: confirmed, severity medium to low (every clean build, CI included, still runs the check; the blind spot is the warm local gate); history: no rationale found (the four lines are from 0a8884ee with one job; 2efff149 added the figure job without extending them; the design note §4 had planned directory-level `docs/`)
- Owner-gated: no

A freshness check whose trigger set excludes the files it guards passes vacuously in exactly the loop where the rot is introduced. The script lists `fuelscape`, the three widget files, and `BEFORE_REGEN_DOC_FIGURE`, but also reads the measurement figure (93) and the committed README copy (185), so editing either leaves `$OUT_DIR/space_consumption.svg` stale and `check_readme_figure_fresh` unrun until some listed path changes, while its doc (175-182) says the check "is what prevents" the rot.

Evidence:

        27	    println!("cargo:rerun-if-changed=fuelscape");
        28	    println!("cargo:rerun-if-changed=docs/fuelscape.css");
        29	    println!("cargo:rerun-if-changed=docs/fuelscape.js");
        30	    println!("cargo:rerun-if-changed=docs/fuelscape-header.html");
       ...
        93	    let figure = std::fs::read_to_string("results/space_consumption/itc_space_consumption.svg")

Resolution: add `println!("cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg");` and `println!("cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg");` beside the existing four (the second may sit next to the env-changed line in `check_readme_figure_fresh` for locality). Acceptance: after a warm `cargo build -p before`, append a byte to `docs/itc_space_consumption_readme.svg` and build again: the script reruns (visible with `-vv`) and fails with "is stale relative to results/space_consumption".
Construction: warm build of `before`; edit a color in `results/space_consumption/itc_space_consumption.svg` (in a scratch copy of the repo); `cargo build -p before -vv` shows `Fresh before` with no build-script run and no staleness panic; `cargo clean -p before` then fails with the stale-figure message.

### fuelscape-render-31: `build.rs`'s re-validation compares untyped JSON values, so non-integer sizes and counts pass, and an `expect("validated")` names a check that never ran
- Where: crates/before/build.rs:303-315 (related: crates/before/build.rs:58-63, :216, :280-282; crates/before-fuelscape/src/compact.rs:379-412)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read; `Option<u64>` orders `None` below every `Some`, so `w[0].as_u64() < w[1].as_u64()` is true when the first size is not an integer; `v.as_u64() != Some(0)` is true for any non-integer end bin; lines 58-63 compare `Value == Value`, so `Null == Null` passes when both files lack a parameter, and line 216 then panics with "validated"); executed: no
- Seen by: adequacy [24], instrument-correctness [55]; refutation: confirmed; history: no rationale found (design note §4:171 fixes "Build-deps: serde_json only", which explains the absence of derive typing but not the `Option` comparisons)
- Owner-gated: no

A check that passes a malformed artifact is decoration, and every `expect` message is a one-line proof. The validator's stated reason to exist (280-282) is that it cannot rely on the compactor's reader, yet it is weaker than that reader on exactly the shapes it names; the malformed values then ride verbatim into the island JSON, where the widget computes `Math.log2(null)` silently. Reachable only through a hand-edited committed file at gate tier (`fuelscape-verify` in `ci` would catch the byte difference), hence low.

Evidence:

       303	    assert!(
       304	        sizes.windows(2).all(|w| w[0].as_u64() < w[1].as_u64()),
       305	        "{file}: the size axis must be strictly ascending"
       306	    );
       307	    for col in cols {
       308	        let c = col["c"].as_array().expect("histogram counts are a list");
       309	        let ends_nonzero = c.first().is_some_and(|v| v.as_u64() != Some(0))
       310	            && c.last().is_some_and(|v| v.as_u64() != Some(0));
       ...
       216	        "seed": format!("{:#x}", meta["base_seed"].as_u64().expect("validated")),

Resolution: demand the types before comparing: collect `sizes` and each `c` into `Vec<u64>`, panicking with the file name on any non-integer entry, then compare plain integers; validate `base_seed` and `samples_per_column` as `u64` in the meta loop so the `expect` at 216 becomes true or dissolves into a checked value (or deserialize `meta`, `sizes`, `cols` into small typed structs with `deny_unknown_fields`, mirroring the compactor's). Acceptance: a committed document with `"sizes":[null,2,...]` or `"c":["x",1]` fails `cargo build -p before` naming the file and check; `"base_seed":"7"` in both files fails naming the parameter rather than panicking with `validated`.
Construction: in a scratch copy of `crates/before/fuelscape/`, set `op.sizes[0]` to `null` in one document: line 304's `all(...)` returns true (`None < Some(2)`) and the emitted island carries the null. Delete `meta.base_seed` from both `index.json` and that document: lines 58-63 pass (`Null == Null`), then line 216 panics with `validated`.

### fuelscape-render-32: nit: `fuelscape-claims` describes an acceptance rule the widget does not implement
- Where: tools/fuelscape-claims:36-38 (related: tools/fuelscape-claims:6-14; crates/before/docs/fuelscape.js:246-295)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read `acceptBound`: it finds the smallest constant `c <= 65536` such that `g(n + c) >= 1` at every measured size and errors only when none exists, so `log n` over sizes starting at 1 is accepted with lift 1); executed: no
- Seen by: instrument-correctness [66]; refutation: confirmed; history: deliberate-but-expired (the "clamp" wording is the round-2 decision as first recorded in the design note §8:384-388; 2efff149's code and message implement the lift rule)
- Owner-gated: no

Prose speaks in the present tense: a reader deciding whether a new claim is admissible is told "positive and finite at the anchor (a zero at a small column is fine — the widget clamps it)" and, at lines 9-12, that `log n` at n=1 renders "a silently unselected hypothesis"; neither matches `acceptBound`. The check itself is correct because it calls the export.

Evidence:

        36	  // The widget's own acceptance rule, verbatim: parse, no throws, never
        37	  // negative, positive and finite at the anchor (a zero at a small
        38	  // column is fine — the widget clamps it).

Resolution: replace both passages with the rule by name: "the rule is `Fuelscape.accepts`: every measured size must give >= 1 after the smallest constant argument lift; see `acceptBound` in fuelscape.js"; restate the `log n` example as the lift case. Acceptance: the tool's comments describe `acceptBound`'s rule and cite it by name.

### fuelscape-render-33: nit: the justfile's head-weight figure for the injected header has drifted
- Where: justfile:251-253 (related: crates/before/docs/fuelscape-header.html; .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:260)
- Class / severity / confidence: claim / nit / high
- Provenance: verified (`wc -c crates/before/docs/fuelscape-header.html` = 77988); executed: yes
- Seen by: scaffolding [18]; refutation: confirmed; history: no rationale (written at 61f05692 as "~40 KB", the design note the same day as "~30 KB"; the widget grew through four later commits with neither updated)
- Owner-gated: no

No hand-maintained counts: the comment says non-before pages carry "~40 KB of inert head weight"; the committed header is 77,988 bytes. (The same comment's "activates only on .fuelscape elements" is finding 27.)

Evidence:

       251	# every page's head. RUSTDOCFLAGS is workspace-wide — cargo has no
       252	# per-crate rustdocflags — so non-before pages carry ~40 KB of inert
       253	# head weight; the script activates only on .fuelscape elements.

Resolution: drop the figure ("carry the header's inert weight") or tie it to the file ("the size of docs/fuelscape-header.html"). Acceptance: the comment carries no byte figure, or the figure is derived.

## Positives

- `dump::read` recomputes every stored `HeatGrid` from the raw samples and refuses disagreement (dump.rs:237-243), and `read_rejects_a_dump_whose_grid_disagrees_with_its_samples` (dump/tests.rs:170-200) is the committed known-bad demonstration that the check fires: a real two-derivation check, not decoration.
- `dump_and_rerender_matches_direct_render_byte_for_byte` (dump/tests.rs:63-130) measures once, renders directly and from the persisted dump, and asserts byte-identical SVGs and gallery across the whole roster: exactly the shape the doctrine asks for when a quantity is computable two ways, and what makes `--render-from` a faithful substitute for re-measuring.
- The compact tamper closure (compact/tests.rs:148-183) constructs each known-bad document by JSON edit and asserts the loader names the check; the idiom is table-shaped, so the missing cases in finding 14 are one line each.
- `tools/fuelscape-claims` loads `docs/fuelscape.js` under node and asks the widget's own exported `Fuelscape.accepts` (lines 23-39), so the claim check and the grammar are one truth rather than a Rust reimplementation.
- The widget refuses a dataset without a positive `res` (fuelscape.js:374-377) and the compactor stamps `RES` into every document (compact.rs:223), closing the wrong-bin-height-forever hole the design note named; `quantAt` and `cdfAt` are exact inverses.
- `check_header_fresh` holds the derived header to exact equality with `<style>{css}</style>\n<script>{js}</script>\n` (build.rs:323-334); I verified with `cmp` that the committed header is fresh at HEAD. `build.rs` also escapes `</` inside the island JSON (223-225) and emits single-line islands so rustdoc's Markdown pass cannot re-enter.
- The libm routing is argued precisely where it matters (render.rs:312-316, compact.rs:200-203), and the accretion design holds in the data: 104 operation documents in both the dump and the compact dataset, 100 stamped f77011e3 and 4 stamped 46eb64f9, both ancestors of HEAD, the dump index naming exactly the files present.
- The runner's clap `Args` (bin/fuelscape.rs:82-119) makes the three modes' flag exclusions declarative with per-flag docs, and the module doc states the purity contract (same plan, same guest, byte-identical measurements in any order).
- `dump::parse` (dump.rs:255-279) reports the plain path's absence with "(nor a .gz sibling)", so strictness errors name the file the caller asked for while the gzip storage stays an implementation detail.
- fuelscape.js's comments consistently state what the code cannot show: the rAF negative-t clamp (53-57), the exact plot-box clip for guides (787-793), hover on the svg rather than the tiles (1004-1007), the tooltip-accumulation guard (944-946), the chip reappend flicker (1436-1439), and `armSummaryClicks`'s account of rustdoc's `preventDefault` (1514-1520). fuelscape.css documents its one specificity fight with rustdoc by naming the rule it out-specifies (26-30).

## Open questions for Finch

1. Was the `spanbands` discriminator's question (which pair coordinate separates the `version_span` bands; classify-first versus emit-always) answered on the span branch? Nothing in the tree records a result. Recommendation: record the answer in an agent note and retire the binary with its two `before` features and the `suanpan` dependency (finding 21, option a); keep it only if the question is still live.
2. Is `CONCURRENT_PANELS = 1` a measured tuning outcome (memory, ETA stability, bar legibility) you want to keep as a knob? Recommendation: dissolve the pool to a plain loop (finding 18); git keeps it if per-panel concurrency ever pays.
3. Should dump accretion be tooled (`DumpWriter::open`, `--append-to`), or is measure-alone-then-hand-merge the intended workflow to be documented in the justfile? Recommendation: tool it (finding 15); the hand merge has no check for run-parameter agreement or duplicate names until `fuelscape-verify` diffs.
4. `fuelscape-verify` is `ci`-tier (about a minute of strict dump reading per the justfile). Given that `build.rs` consumes the committed dataset on every build and the byte-derivation is the only thing binding it to the dump, should it be gate-tier? Recommendation: measure the minute on the gate host; if it fits the wasm stream's budget, promote it.
5. The cross-platform hash pin (finding 7) runs only in the local gate; the commit that introduced it names the illumos gate run as the second-architecture witness. Recommendation: state that in the docstring, strengthen the fixture so more than one hashed value is libm-sensitive, and treat `fuelscape-verify` on ubuntu CI as the real-data cross-host witness it already is.
6. Is the probe-trace smoothing (finding 23) a presentation choice to keep? Recommendation: draw raw quantiles; at 4096 samples per column the smoothing removes column-to-column structure rather than noise, and interior anchoring exposes the inconsistency.
7. Is `typesetDocMath` on every workspace crate's pages intended (finding 27)? Recommendation: gate on `data-current-crate === "before"`; the other crates' authors did not opt into italic math for their lone-letter code spans.
8. Outside this partition: `crates/before/src/testing/fuelscape_islands.rs:22` declares `const EXEMPTIONS: &[(&str, &str)] = &[];`. The doctrine forbids a mechanism for accepting known failures "even as an empty buffer"; the design note frames it as a modeled-exemption list for derived impls with no doc site. For the testing-oracles reviewer or you: is an empty, reason-carrying exemption table a sanctioned model declaration here?
9. The design note §2:104-109 promises reader rejections that were never implemented (a histogram whose counts do not sum to `samples_per_column`, and roster presence checked in the reader rather than only in `compact_dump`). If the note is treated as a spec, it and the code should be reconciled; if not, no action.

## Dropped

- [12] The widget's x-axis caption says "total input size" for every operation: deliberate and recorded (design note §8 Q2 fixes one denominator for the whole widget; build.rs:230 emits "in total input bytes" for every island; the tooltip carries `size_measure`); the SVG renderer's unary/total split is a separate audit view and also omits the party for `version_tick`.
- [50] Hand-rolled SplitMix64 contradicts the manifest's rand_chacha rationale: refuted; `cell_rng` uses `ChaCha12Rng::from_seed` over a 32-byte key (the documented-portable path the manifest describes), and the test declines to ride rand's minor-bump value-stability policy for a committed hash; rand_core 0.6.4's docs (lib.rs:303-307, 333-334) frame these as different axes, not a contradiction.
- [20], [39], [54]: duplicates of finding 30 (rerun-if-changed).
- [22]: duplicate of finding 15 (accretion writer).
- [31], [43], [64], [7]: merged into finding 29 (build.rs module doc).
- [29], [33], [58]: duplicates of finding 18 (panel pool).
- [21], [41], [42], [59]: merged into finding 21 (spanbands).
- [28], [35]: duplicates of finding 9 (version-history docs).
- [26], [37], [65]: merged into finding 14 (tamper coverage).
- [60], [23]'s overlay half, and the refutation's new zero-size item: merged into findings 6 and 8 (positivity policy; writer never validates).
- [32], [36]: merged into finding 13 (fixtures and temp dirs).
- [44]: merged into finding 2 (qualified paths).
- [30], [47], [63], and the refutation's new clamp nit: merged into finding 4 (render.rs vestigial), with the drop claim corrected (the drops are load-bearing).
- [27], [62]: duplicates of finding 16 (write_atomic).
- [45]: merged into finding 1 (vocabulary); the CSS font stack half of [17] is finding 28.
- [55]: duplicate of finding 31 (untyped validation).
- [57]: duplicate of finding 22 (smoothSeries NaN).
- [53]: duplicate of finding 3 (max-bytes panic).
- [49]: split into finding 10 (overlay doc tense) and finding 11 (expect messages).
