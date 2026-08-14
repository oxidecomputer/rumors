# Fuelscape widgets in the `before` rustdoc

**Goal.** Every public operation's `# Complexity` section carries an
interactive fuelscape: a `<details>` expander whose summary states the
measured instruction-count claim, and whose body is the hypothesis-testing
widget (density columns, quantile probe, compensation guides). The data of
record is a committed, compact per-operation dataset derived from a
fuelscape atlas dump; the build is pure Rust (`build.rs`), with no Python
anywhere in the pipeline. The widget's chrome matches rustdoc's visual
language and theming.

**Mechanism beside the goal, throughout:** where a rule below conflicts
with this goal, the goal wins and the conflict is a finding.

---

## 1. Pipeline architecture

```
just fuelscape                    (existing; hours in the fuzzfit guest)
  └─> raw atlas dump              fuelscape-op-atlas JSON, ~2.4 MB/op, ~242 MB/run
        │
        ▼
fuelscape compact  (NEW subcommand in crates/before-fuelscape)
  └─> crates/before/fuelscape/    fuelscape-widget-data JSON, ~3 KB/op, ~320 KB total
        │                         COMMITTED — the data of record for the docs
        ▼
crates/before/build.rs  (NEW)     pure formatter: JSON -> island HTML
  └─> $OUT_DIR/fuelscapes/<op>.html
        │
        ▼
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/<op>.html"))]
                                  one per operation, in its # Complexity section
        │
        ▼
crates/before/docs/fuelscape.{css,js}        widget source, committed
crates/before/docs/fuelscape-header.html     derived concatenation, committed,
                                             freshness-checked by build.rs
  └─> rustdoc page <head> via --html-in-header (justfile recipes + docs.rs metadata)
```

Measuring and rendering stay decoupled (the dump module's existing
argument): a re-measure is a deliberate re-pin, a widget or style change
replays from committed data in seconds.

### Why the committed form is the compact one, not the raw dump

This is forced, not stylistic:

- crates.io caps packages at 10 MB, and docs.rs builds from the published
  package. `build.rs` therefore may only depend on files that ship inside
  `crates/before`. The raw dump is ~242 MB; the compact form measures
  ~3 KB/op (clock_join binned at RES 0.05: 3,056 bytes), ~320 KB for the
  full roster. Only the compact form can live in the crate.
- The compact form is sufficient: the widget consumes per-column log₂-fuel
  histograms, never raw samples. Quantiles, the worst-case constant, and
  all rendering derive from the histograms client-side.

The raw dump's disposition is an open decision (§8, Q1).

## 2. The compact format: `fuelscape-widget-data` v1

One JSON document per operation in `crates/before/fuelscape/`, plus an
`index.json`, following the dump module's idiom exactly (format banner,
version, meta repeated in every file so each stands alone, atomic writes,
strict reading with `deny_unknown_fields`):

```jsonc
{
  "format": "fuelscape-widget-data",
  "version": 1,
  "meta": { "commit": "…", "base_seed": …, "samples_per_column": … },
  "op": {
    "op_name": "clock_join",
    "size_measure": "…",            // stamped from the OpSpec row; drives the x-axis caption
    "contract": "O(|self| + |party|)",  // the rustdoc contract, display text; see §3
    "claim": "n",                    // widget grammar, bytes-denominated; see §3
    "res": 0.05,                     // octaves per bin; the compactor owns this constant
    "sizes": [3, 6, 12, …],          // total packed input bytes per column
    "cols": [ { "k0": …, "c": [ … ] }, … ],   // per-column log2-fuel histogram
    "overlay": [ { "family": "…", "size": …, "fuel": … }, … ]
  }
}
```

Notes:

- `res` rides in the data; the widget reads it and has **no fallback**
  (the prototype's `RESFALLBACK` is a silent-liveness hole: a dataset
  missing `res` would render at the wrong bin height forever; hydration
  refuses instead).
- `overlay` carries the committed adversarial-family points already in the
  dump. The first widget release need not draw them; carrying them costs
  ~1 KB/op and lets a later revision add family marks without a format
  bump or re-compaction.
- The compactor bins from raw samples the way the prototype does
  (`floor(log2(fuel) / res)` per column); the binning constant and code
  live in `before-fuelscape` (beside `aggregate`, the existing binning),
  never in `build.rs`.

### Strictness and adequacy

- The compactor's reader (used by its own round-trip tests) rejects:
  banner/version mismatch, meta differing between index and op files,
  op-name/index disagreement, unsorted or duplicate `sizes`, empty `cols`,
  a histogram column whose counts don't sum to `samples_per_column`
  (modulo rejection accounting), an op present in the dump but absent
  from the ops roster or vice versa.
- The known-bad artifacts (wrong banner; meta mismatch; unsorted sizes;
  untight histograms; empty claims) are constructed by JSON tampering
  inside the compactor's tests — the dump suite's established idiom for
  the same adequacy discipline — and each rejection must keep firing.
- `build.rs` re-validates structurally at doc build (banner, meta
  uniformity, sorted sizes, non-empty cols) and panics naming the file
  and check. Build-time validation of committed repo data is programmer
  error by construction, so a panic is the correct register here.

## 3. Complexity claims: where they live and what they mean

Each operation needs a claimed bound in the widget's grammar (`n`,
`n log n`, …), denominated in **total packed input bytes** — the
fuelscape's x-axis. This is a *different statement* from the rustdoc's
`# Complexity` contract (`O(|self| + |party|)`, multi-variable, in
abstract structure sizes). Both appear; neither impersonates the other.

- **Both statements live in the `OpSpec` row**
  (`crates/before-fuelscape/src/ops.rs`), as two new non-optional fields:
  `contract: &'static str` (the rustdoc contract, display text, e.g.
  `O(|self| + |party|)`) and `claim: &'static str` (widget grammar,
  bytes-denominated). Rationale: the ops table is already the single
  roster (one row per measured operation, parity-tested against
  `before::surface`), and non-optional fields make totality a compile
  error — the strongest possible check. Moving the contract out of ~100
  doc comments into one roster is the "load-bearing number lives in a
  mechanically-enforced place" shape; the rendered `# Complexity`
  sections still state it, so nothing the asymptotics pins describe
  changes in the docs a reader sees. The compactor stamps both fields
  into each compact file; a dump/roster mismatch at compaction time is
  an error, forcing deliberate handling when the roster has moved since
  the measuring run.
- **Enforcement stays where it is.** The fuzz-fit bands own asymptotic
  enforcement (per the justfile's own comment); the widget claim is the
  *presented default hypothesis*, and its correctness check is the reader
  dragging guides. We do not build a flatness gate over the widget data —
  that would be a second, noisier enforcement channel for a claim the
  bands already own.
- **Populating ~100 claims is review work, not transcription.** Never
  transcribe a proposed constant: each claim is drafted from the op's
  documented contract, then checked against its own fuelscape (does the
  claimed guide flatten the band?) before committing. Deliverable for
  review: a table of op → claim → verdict, plus the rendered gallery.

## 4. `build.rs`: a pure formatter

Much smaller than the prototype, because binning and claims moved into
the compactor:

- `cargo:rerun-if-changed` on `fuelscape/` and `docs/`.
- Read every `fuelscape/*.json`, validate (§2), emit
  `$OUT_DIR/fuelscapes/<op>.html` — one **single-line** island per op
  (a newline inside inline HTML would let rustdoc's Markdown re-enter),
  with `</` escaped to `<\/` inside the JSON script element, and a
  `<noscript>` fallback stating the claimed bound.
- Emit `$OUT_DIR/fuelscapes/index` (newline-separated op names) for the
  inclusion-totality test (§5).
- Header freshness check: `docs/fuelscape-header.html` must equal
  `<style>{fuelscape.css}</style>\n<script>{fuelscape.js}</script>\n`.
  The panic message points at `just fuelscape-header`, which regenerates
  it (recipe, not an inline shell incantation in a panic string).
- Build-deps: `serde_json` only. The script runs for every consumer build
  (including the detached fuzz/fuzzfit workspaces and wasm targets — build
  scripts execute on the host, so this is safe); it is a file transform
  over ~320 KB and stays negligible.

Island skeleton (Q2, §8: the contract is the clickable element):

```html
<details class="toggle fs-details"><summary>
  <code>O(|self| + |party|)</code> ·
  <span class="fs-claim">measured growth Θ(<code>n</code>) in total input
  bytes</span></summary>
  <div class="fuelscape"><script type="application/json">{data}</script></div>
  <noscript><p>The interactive fuelscape requires JavaScript; the claimed
  growth is Θ(n) in total input bytes.</p></noscript>
</details>
```

**No absolute numbers in the summary.** Fuel is the fuzzfit wasm guest's
deterministic instruction count: meaningful for growth shape and relative
comparison, meaningless as an absolute cost for a native build. The
prototype's hydrate-time "worst-case c ≈ …" constant is deleted outright
(the `.fs-c` slot and the code that fills it); the summary states only
the claimed growth class and its denominator.

## 5. Doc attachment: ~100 sites, two totality directions

At each operation's `# Complexity` section, the island *replaces* the
contract paragraph (the contract now renders inside the summary, sourced
from the roster):

```rust
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_tick.html"))]
pub fn tick(&mut self, party: &Party) { … }
```

- The sweep transcribes each site's contract string into its `OpSpec`
  row exactly as the doc source states it today; the sweep commit shows
  each prose deletion beside the roster line carrying the same string,
  so the move is diff-reviewable as a pure relocation. Where a
  `# Complexity` section carries prose beyond the claim (amortization
  notes, mechanism sentences), that prose stays in the doc source below
  the expander.
- Unmeasured items keep their prose contracts unchanged: the tree has
  161 `# Complexity` sections and ~100 measured ops, so the two idioms
  coexist deliberately — an expander wherever a fuelscape exists, plain
  prose where none does (mostly `O(1)` accessors). doclint/testdoc
  semantics are untouched either way (the island sits far below the
  summary paragraph).
- **Mapping op → site follows `OpSpec.covers`**, which already binds each
  op to its `before::surface` roster row — the mapping is not invented
  fresh.
- **Totality, both directions:**
  - *Dangling include → compile error* (a missing `$OUT_DIR` file fails
    `include_str!`); nothing more needed.
  - *Orphaned island*: a unit test in `before` reads
    `$OUT_DIR/fuelscapes/index` (via `env!("OUT_DIR")`), scans
    `src/**/*.rs` for `fuelscapes/<op>.html` occurrences, and asserts
    every op is included exactly once **or** appears in a committed
    `EXEMPTIONS`-style list with a one-line reason — the same
    reviewed-membership idiom the ops table already uses against the
    surface roster.
- Expected exemptions exist: ops that measure derived impls have no doc
  site to carry an island (`version_eq` measures `Eq`, which `Version`
  derives; a derived impl takes no doc attribute). Each such op gets a
  reasoned exemption row, not a silent skip.

The 100-site sweep lands as its own commit containing *only* the
attribute insertions — no opportunistic prose edits riding along in
heavily-reviewed doc comments.

## 6. Widget assets and rustdoc integration

Source of truth: `crates/before/docs/fuelscape.js` and `fuelscape.css`,
committed; `fuelscape-header.html` derived from them (`just
fuelscape-header`), committed because rustdoc flags cannot point into
`$OUT_DIR`, and freshness-checked by `build.rs` so it cannot rot.

Wiring:

- `[package.metadata.docs.rs] rustdoc-args = ["--html-in-header",
  "docs/fuelscape-header.html"]` (docs.rs resolves the path against the
  crate root; the file ships in the package).
- The `docs` and `docs-internal` justfile recipes add the same flag with
  an absolute path (`{{ justfile_directory() }}/crates/before/docs/…`)
  to `RUSTDOCFLAGS`. This injects the header into every workspace
  crate's pages; the script is inert without `.fuelscape` elements, so
  the cost is ~30 KB of dead weight on non-`before` pages — accepted,
  since cargo has no per-crate rustdocflags in a workspace doc build.
- `cargo package --list` is checked once during implementation to
  confirm `fuelscape/` and `docs/` ride in the package and the total
  stays far under the cap.

### Widget improvements (beyond the prototype)

1. **Scale-free presentation, "instructions" vocabulary kept.** The
   working assumption: instruction counts relate linearly across
   platforms, so an operation's growth *shape* transfers even though the
   wasm-guest absolute counts do not (and the graphs are not meant for
   cross-comparison or as benchmarks — real benchmarks run on your own
   platform). The widget therefore keeps the word "instructions" and
   removes every absolute-count surface instead:
   - y-axis `2^k` tick labels: gone (unlabeled gridlines still read as
     log-scale structure — their spacing ratio is constant);
   - the quantile slider readout keeps the quantile name ("med", "p95"),
     drops the value;
   - the per-column hover readout keeps `n` (input bytes — real) and
     drops the fuel values; if a number earns its place there, the
     min–max **spread as a ratio** ("×3.2"), which is platform-invariant
     under the linearity assumption;
   - the worst-case constant is already deleted (§4).
   The assumption is stated once, at zero reading cost: a `title=`
   tooltip on the y-axis caption or a clause in the provenance footer
   ("counted deterministically in a measurement guest; shapes transfer
   across platforms, absolute counts don't"). The prototype's title
   "Asymptotic Time Complexity" is still wrong twice (neither time nor a
   proof) — retitle to "Measured growth" or similar.
2. **Lazy hydration.** rustdoc renders *every* method's full docs on the
   type's page, so `Version`'s page would carry dozens of islands; the
   prototype hydrates all of them at `DOMContentLoaded` (each widget is
   thousands of SVG rects). Instead: build each widget's DOM on its
   details element's first `toggle`. Nothing is computed eagerly — the
   summary is static text (§4). The `toggle` event fires on programmatic
   `open` changes too, so rustdoc's expand-all control hydrates correctly
   (as a user-initiated burst, accepted).
3. **`res` required** (no `RESFALLBACK`), per §2.
4. **`size_measure` drives the x-axis caption** (currently hardcoded
   "total input size (bytes, log scale)"); the per-op string is already
   in the data.
5. **Export `globalThis.Fuelscape`** (not only `window.…`) so node can
   load the bundle for the CI checks (§7) without a DOM.
6. **Theme fidelity**: verify the `data-theme` observation against the
   pinned toolchain's actual rustdoc output (attribute name and values),
   in both themes and the "system" setting; the widget already listens
   via MutationObserver + `prefers-color-scheme`.
7. **Rustdoc visual language**: the islands take rustdoc's own
   `details.toggle` class (decision Q3: resolved) — native chevron,
   spacing, and participation in rustdoc's global expand/collapse-all —
   with our `fs-details` skin layered on top, and our own CSS still
   carrying a complete fallback look so a rustdoc.css class-convention
   change degrades gracefully rather than unstyling the islands. Keep
   rustdoc's variables and font stacks (already wired); drop the unused
   `--fs-orange` token.
8. Keep: keyboard a11y, `prefers-reduced-motion`, `aria` roles,
   `__FS_NO_ANIM` test hook, the `</`-escape.

## 7. Verification wiring

| Check | Where | Tier |
|---|---|---|
| Header freshness (css+js ↔ header) | `build.rs` panic | every build of `before` |
| Compact-data structural validity | `build.rs` panic | every build of `before` |
| Claim totality over ops | non-optional `OpSpec.claim` field | compile |
| Compactor strictness + round-trip + known-bad fixtures | `before-fuelscape` tests | `just fuelscape-test` (gate) |
| Island-inclusion totality (both directions, exemptions reviewed) | `before` unit test | `just test` (gate) |
| Islands render, links resolve, no doc warnings | existing `docs` / `docs-internal` recipes | gate |
| `node --check docs/fuelscape.js` + every committed claim parses in the widget grammar and is positive over its sizes | small node script | `just ci` (node already required there) |
| Raw-dump ↔ compact two-ways pin over the committed gzipped dump | `fuelscape-verify` recipe | `just ci` (reads ~242 MB of JSON; measure, and promote to gate only if it stays trivial) |

The claim-parse check is the liveness floor for claim strings: without
it, a typo'd claim renders a permanent silent error chip. It runs under
node rather than reimplementing the expression grammar in Rust — the
grammar's single truth stays `fuelscape.js`.

Manual acceptance for the visual work: `just docs`, then eyeball a
struct page and a standalone fn page in light, dark, and system themes,
plus one op end-to-end against the prototype gallery for parity.

## 8. Decisions

**Q1 — the raw dump's disposition: RESOLVED — committed, gzipped per
file, outside the crate** (e.g. `crates/before-fuelscape/dump/`,
`<op>.json.gz`). ~24 MB in tree and per re-measure in history; buys the
raw↔compact two-ways pin (`fuelscape-verify`, §7) and re-binning without
re-measuring — the decoupling that justified the dump format in the
first place. The verify leg either streams through `gzip -dc` or the
dump reader learns `.json.gz`; whichever the implementation picks, the
committed files stay byte-stable (deterministic gzip invocation, no
timestamps in the member header — `gzip -n` — so the pin compares clean).
Provenance: the dump's `meta.commit` is an ancestor of HEAD (verified).

**Q2 — the summary line: RESOLVED, in three parts.** (1) No absolute
instruction counts anywhere user-facing — not in the summary (the
"worst-case c ≈ …" headline is deleted) and not inside the widget
(§6.1): counts are WASM operations metered in a sandboxed build, and
only shapes and ratios transfer to native builds (the provenance
tooltip says exactly that). The vocabulary stays "instructions",
presented scale-free. (2) The contract itself is the clickable element,
and the contract string relocates from doc source into the `OpSpec`
roster (§3, §5) — a diff-reviewable pure move, with rendered docs
unchanged in what they state. (3) The claim is **contract-shaped**: the
pre-selected hypothesis is the contract's worst-case asymptote in total
input bytes, so the summary — "O(n) in total input
bytes; `O(|self| + |other|)`" — and the chart always assert the same
bound. An
early-exit operation therefore opens with a visibly falling band: the
uniform-sampling bulk beating the claimed worst case is the finding,
and the flatter guides are one click away. (The earlier bulk-shaped
defaults read as summary/chart mismatches and were replaced.)

**Q3 — rustdoc's `details.toggle` class: RESOLVED — take it.** Native
chevron and spacing; islands participate in expand/collapse-all (lazy
hydration handles the burst, §6.2); our CSS keeps a complete fallback
look so rustdoc.css internal changes degrade rather than break.

**Q4 — the claims themselves.** ~100 new public statements about
measured growth, each needing Finch's review (per §3's protocol). The
plan produces the draft table + gallery; the review is his.

**Round-2 review decisions** (from the first rendered pass):

- **Natural claim forms.** The widget accepts a hypothesis that is zero
  at a small column ("n log n" at n = 1) and clamps such values to 1
  for compensation; acceptance requires positivity only at the anchor.
  The rule is exported (`Fuelscape.accepts`) and the claim-check tool
  calls it verbatim, so the two cannot drift.
- **The painted band ends at the data.** Display smoothing renormalizes
  a truncated kernel at the support's edges instead of padding past
  them: a band overhanging the true min–max made the quantile slider
  read as stopping short.
- **Pre-hydration by scroll**: islands hydrate when their expander
  nears the viewport (closed included), with first-toggle as the
  fallback wake, so opening one is instant.
- **Interactions**: the probe trace and every hypothesis line are
  draggable (quantile inversion through the pointer's nearest column's
  own density); clicking a column locks the hypothesis/quantile anchor
  there, clicking again returns it to the largest n.
- **Chrome**: accent colors derive from the theme's own code-highlight
  palette (hypotheses = keyword color, quantile = string color); the
  "Hypothesis:"/"Quantile" labels are serif; the expander carries an
  explicit chevron; the footer is one provenance line (short commit
  hash) with the caveats in its tooltip; the plot fills the card width;
  the word "fuelscape" appears nowhere in the rendered chrome.
- **Secondary sites**: islands also attach at covered aliases and
  writer variants (`encode_to` and kin, the materializing `From` impls,
  the forks machinery, the tick doors) — attachment requires read-code
  evidence that the site delegates to the measured kernel, never
  name-resemblance.

## 9. Execution phases (dependency order)

0. **Spike (de-risks everything downstream):** hand-attach one island
   (`version_tick`) with a throwaway build.rs, run `just docs`, verify:
   inline HTML survives rustdoc's Markdown pass, `-D warnings` stays
   clean, the header flag works, theming and hydration behave on a real
   rustdoc page. *Nothing else lands until this passes.*
1. **Compact format + compactor** in `before-fuelscape` (`compact`
   subcommand, strict reader, round-trip tests, known-bad fixtures);
   `OpSpec.claim` field with draft claims; `just fuelscape-compact`
   recipe. (Q4 review gates the claim values, not the machinery.)
2. **Data landing:** run the compactor over the source dump; commit
   `crates/before/fuelscape/` and the gzipped raw dump (Q1). Directory
   names undated — provenance lives in `meta` and git history.
3. **Widget assets:** port `fuelscape.js`/`.css` into
   `crates/before/docs/` with §6's improvements; `just fuelscape-header`;
   node checks wired into `just ci`.
4. **`build.rs` + manifest:** the formatter, its checks, `serde_json`
   build-dep, docs.rs metadata, `cargo package --list` sanity.
5. **Attachment sweep:** at each covered site, replace the contract
   paragraph with the doc attribute (mapping via `OpSpec.covers`),
   transcribing the contract string into the roster verbatim; commit the
   exemptions list; land the inclusion-totality test. One mechanical
   commit, no prose changes beyond the relocation.
6. **Recipes + gate:** `docs`/`docs-internal` header flag,
   (`fuelscape-verify` per Q1); `just gate` and `just ci` fully clean;
   manual two-theme eyeball; retire the Downloads prototypes only after
   one op renders at parity with the prototype gallery.

## 10. Non-goals (stated so they don't creep back in)

- No flatness/asymptotics gate over widget data — the fuzz-fit bands own
  enforcement.
- No overlay-family rendering in the first release (the data carries
  overlays so a later revision can add marks without a format bump).
- No gallery in the crate-level docs for now (it would leak into the
  cargo-rdme README); the rustdoc pages are the gallery.
- No screenshot pinning of widget pixels.
