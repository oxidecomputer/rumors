# Partition crate-root: Crate root and cross-cutting: lib docs, error, fold, iter, auto traits, shape, recurse, serde and borsh impls, build.rs, Cargo.toml

## Partition summary

The crate root is where `before` states its contract and hosts the machinery every other module shares. `lib.rs` carries the crate-level docs (the model, the safety rules, the space and asymptotic promises, the feature list) and the module tree; `error.rs` the four error types; `fold.rs` the balanced binary-counter reduction that every n-ary operation runs on; `shape.rs` the public step-function vocabulary (`Plateau`, `Region`, `Cell`, and their iterators); `recurse.rs` the test-only stack-growth guard and its segment counter; `serde_impls.rs` and `borsh_impls.rs` the two serialization doors over the one canonical wire form; `auto_traits.rs` the compile-time `Send + Sync + Unpin` pins; `build.rs` a pure formatter that renders the committed fuelscape datasets into rustdoc islands and derives the README figure from the measurement SVG; and `Cargo.toml` the features, the `unexpected_cfgs` roster, and the `required-features` guard on the board example. I read all fourteen partition files in full with line numbers (3838 lines) plus the cross-file anchors each finding rests on (meter.rs and the board modules, tests/meter.rs, party.rs, clock.rs, version.rs, ticks.rs, skyline.rs and its shape and overlay modules, codec/stack.rs, laws.rs, surface.rs, surfacecheck, the fuelscape datasets, results/space_consumption, and the serde_core, serde_bytes, ciborium, and thiserror-impl sources in the local registry). The test files are `shape/tests.rs`, `serde_impls/tests.rs`, and `borsh_impls/tests.rs`. No cargo, just, or build command was run; every "verified" below means read or mechanically checked (grep, git, a python parse of committed data), never executed, except where a finding says otherwise.

The code itself is in good shape. Nothing in the partition is unsafe, nothing recurses on input depth in a library file, and the only panic sites in production code (fold.rs's two `expect`s, borsh_impls.rs's buffer index and shifts) are programmer-error-only with messages that argue them. `ReaderCursor` in borsh_impls.rs is the model wire door: one byte pulled per demanded bit, the word window provably unable to touch a byte the reader has not yielded, the read buffer adopted as storage without a copy, and a per-bit reference cursor that asserts byte consumption on accepts and rejects alike. `fold.rs` is one home for the counter discipline with the quadratic left-fold failure named beside it. `build.rs` panics on repository defects and computes nothing. `Cargo.toml` closes two quiet-failure paths mechanically (`unexpected_cfgs = deny` and `required-features` on `amp_board`).

The dominant issues are in prose and instruments, not code. First, the crate-level docs make three quantitative or asymptotic promises that either have no committed instrument or contradict the crate's own per-operation contracts: "approximately 100× more space-efficient" (no measurement exists), "asymptotically linear" (rank's committed contract is `O(M(|self|) · log |self|)`), and "100 parties and 1,000,000 events" (the committed artifact runs 4..128 parties at 100k/25k iterations). Second, the stack-segment counter that the board and the meter envelopes judge has no writer in any binary that judges it: its one `fetch_add` is `#[cfg(test)]`, while the readers compile under `any(test, feature = "meter")`, so the "measured fact" recurse.rs describes is a compile-time zero. Third, the serde impls serialize as the data model's `bytes` type and deserialize by requesting a `seq`; the three tested formats bridge the two, but a conforming strict Deserializer rejects what `before` itself wrote. The remainder is a scatter of hand-maintained rosters that drifted (auto_traits, fold's caller list, the `Parse` and `Overlap` producer lists), ghost references (the crates.io description, AGENTS.md, a `crate::bookmark` link into rumors), a public definition of *plateau* the code contradicts, and idiom nits in the test files.

## Findings

### crate-root-1: AGENTS.md sends readers to a "Law of Disjointness" and an `implementation` module that do not exist
- Where: crates/before/AGENTS.md:5-6 (related: crates/before/examples/code_study.rs:6, crates/before/src/lib.rs:224-231, crates/before/src/lib.rs:415-454)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `implementation\b` and `Law of Disjointness` over crates/before; `ls crates/before/src` shows no implementation.rs; lib.rs module list read at 415-454); executed: no
- Seen by: structure, prose; refutation: confirmed; history: contradicts-hard-rule (the rule was renamed at a431eaf1d, the module retired at 22cdfbe1; AGENTS.md last touched 04b4d1b7, between the two)
- Owner-gated: no

The guidepost every agent reads first points at a rule name lib.rs no longer uses (the safety rules are "Causal Singularity" and "Identity Linearity", lib.rs:224 and 231) and at a public module that no longer exists; `examples/code_study.rs:6` links `before::implementation` as well. The root AGENTS.md hard rule forbids references to code that no longer exists.

Evidence:

     5  crate docs for the model (`Party`/`Version`/`Clock`, the Law of
     6  Disjointness) and the public `implementation` module for the design essay,

    code_study.rs:
     6  //! [`implementation`](before::implementation) essay's "Small values over

Resolution: Point at what exists: the crate docs' "Safety rules" section, and `version/skyline.rs` (which line 7 already names) for the coding and kernels; fix or remove the `before::implementation` link in code_study.rs, naming the module doc that now carries the "Small values over large" trade. Acceptance: every module and rule name AGENTS.md cites resolves in src; `grep -rn 'before::implementation\|Law of Disjointness' crates/before` is empty.

### crate-root-2: Cargo description names a "transient fixed-width working form" the crate no longer has
- Where: crates/before/Cargo.toml:5-5 (related: crates/before/results/benchmarks/README.md:4, crates/before/src/lib.rs:333-340)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep -rni 'working form' over crates/before: Cargo.toml:5, results/benchmarks/README.md:4, and benches/board.rs:185 in an unrelated sense; nothing in src); executed: no
- Seen by: structure, prose, claims; refutation: confirmed; history: contradicts-hard-rule (faf3cd0a6 deleted `version/working.rs`, whose module doc was "The transient fixed-width working form for event mutation"; 1af119c1f re-touched the line and left the phrase)
- Owner-gated: yes (crates.io-facing wording)

The one line every crates.io visitor reads describes a design element that was deleted; the crate docs now describe fused single-pass kernels over the packed form (lib.rs:336-338). `results/benchmarks/README.md:4` carries the same ghost outside this partition.

Evidence:

     5  description = "Interval Tree Clocks (Almeida, Baquero & Fonte, 2008): packed bit-stream storage, transient fixed-width working form, linear-typed API."

Resolution: Restate against today's design, for example "Interval Tree Clocks (Almeida, Baquero & Fonte, 2008): canonical packed bit-stream storage, fused streaming kernels, linear-typed API"; fix results/benchmarks/README.md:4 in the same pass. Acceptance: `grep -rni 'working form' crates/before --include='*.toml' --include='*.md'` is empty.

### crate-root-3: serde `derive` feature enabled with no derive in the crate
- Where: crates/before/Cargo.toml:30-30 (related: Cargo.toml:68 at the workspace root)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for `derive(.*Serialize`, `derive(.*Deserialize`, `#[serde` over crates/before/**/*.rs: no hits; every impl in serde_impls.rs is hand-written); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (Phase 0 boilerplate carried through the rename; no derive ever existed)
- Owner-gated: no

The `derive` feature pulls `serde_derive` (a proc-macro crate) into every consumer's build that enables `before/serde`, and nothing in the crate uses it. `alloc` is needed (the workspace serde is `default-features = false` and the `Vec<u8>` impls live under `alloc`); `derive` guards nothing.

Evidence:

    30  serde = { workspace = true, optional = true, features = ["derive", "alloc"] }

Resolution: `features = ["alloc"]`. Acceptance: `cargo check -p before --features serde` and the serde test legs build clean.

### crate-root-4: build.rs's module doc and the manifest comment describe only the fuelscape job; the "holds no constants" clause is contradicted by the v3 banner literal
- Where: crates/before/build.rs:1-22 (related: crates/before/Cargo.toml:16-19, crates/before/build.rs:93-100, crates/before/build.rs:114-115, crates/before/build.rs:183-197, crates/before/build.rs:272-278, crates/before-fuelscape/src/compact.rs:82)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (build.rs read in full; `grep -n FORMAT_VERSION crates/before-fuelscape/src/compact.rs` gives line 82 `const FORMAT_VERSION: u32 = 3;`); executed: no
- Seen by: structure, prose; refutation: confirmed; history: deliberate-but-expired (the doc and Cargo.toml:16-19 are from 0a8884ee4 and were accurate then; 2efff1498 added the figure job without amending either; 6f63edb7 bumped the banner from v2 to v3 in both the tuple and the message)
- Owner-gated: no

The doc's Inputs and Outputs (3-15) name `fuelscape/` and `docs/` and `$OUT_DIR/fuelscapes/...`, but `main` also reads `results/space_consumption/itc_space_consumption.svg` (93), writes `$OUT_DIR/space_consumption.svg` (95-99), and under `BEFORE_REGEN_DOC_FIGURE` writes into the source tree (187-189); Cargo.toml:16-19 has the same blind spot. Line 18-19 says the script "holds no constants the widget or compactor also hold", yet `check_banner` holds the format version `3` twice (as a value and inside the message string), a number the compactor owns as `FORMAT_VERSION`. `theme_svg`'s first sentence (114-115) names only the rustdoc target though it also derives the README copy.

Evidence:

     9  //! Outputs: `$OUT_DIR/fuelscapes/<op>.html` — one single-line
    10  //! `<details>` island per operation, pulled into a `# Complexity`
    ...
    17  //! This script is a pure formatter: it re-bins nothing, computes no
    18  //! statistics, and holds no constants the widget or compactor also
    19  //! hold. Every failure here is a defect in the committed repository
    ...
   273      assert_eq!(
   274          (doc["format"].as_str(), doc["version"].as_u64()),
   275          (Some(expected), Some(3)),
   276          "{file}: not a {expected} v3 document"
   277      );

Resolution: Add the figure job to Inputs (the results/ SVG and the committed README SVG) and Outputs (`$OUT_DIR/space_consumption.svg`; the opt-in source-tree write); mirror it in Cargo.toml:16-19; widen `theme_svg`'s first sentence to both targets. Name the banner version once (`const WIDGET_DATA_VERSION: u64 = 3;` used in the tuple and interpolated into the message) and amend lines 18-19 to say the format banner is the one deliberately shared constant, the consumer's pin on the compactor's number. Acceptance: every path `main` reads or writes appears in the module doc; `grep -n 'Some(3)\|v3' crates/before/build.rs` finds only the constant's definition and its interpolation.

### crate-root-5: build.rs declares `rerun-if-changed` for the fuelscape inputs only, so the README-figure freshness check does not rerun on the files it compares
- Where: crates/before/build.rs:27-30 (related: crates/before/build.rs:93, crates/before/build.rs:175-197, justfile:710-711)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (the four `rerun-if-changed` lines and the one `rerun-if-env-changed` at 184 are the only triggers; the figure is read at 93 and the README copy at 191); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed, severity lowered to low (`just doc-figure` flips the env trigger and so reruns the script; only a hand edit of either SVG, or a results/ regeneration without `just doc-figure`, escapes, and only in the local loop; a clean CI build still fails); history: no-rationale-found (2efff1498 added the figure job and its env trigger without path triggers)
- Owner-gated: no

Once a build script prints any `rerun-if-changed`, cargo reruns it only when a listed path or a `rerun-if-env-changed` variable changes. The two SVGs the freshness check reads are not listed, so on an incremental build an edit to either does not run the check the doc at 175-182 calls "what prevents" the derived figure from rotting.

Evidence:

    27      println!("cargo:rerun-if-changed=fuelscape");
    28      println!("cargo:rerun-if-changed=docs/fuelscape.css");
    29      println!("cargo:rerun-if-changed=docs/fuelscape.js");
    30      println!("cargo:rerun-if-changed=docs/fuelscape-header.html");
    ...
    93      let figure = std::fs::read_to_string("results/space_consumption/itc_space_consumption.svg")
    ...
   185      let path = "docs/itc_space_consumption_readme.svg";

Resolution: Add `println!("cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg");` and `println!("cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg");` beside the existing four. Acceptance: after a green `cargo build -p before`, appending a byte to `docs/itc_space_consumption_readme.svg` and rebuilding reruns build.rs and fails with the "is stale relative to results/space_consumption" message.
Construction: With a warm build dir, append a whitespace byte to `crates/before/docs/itc_space_consumption_readme.svg` and run `cargo build -p before`: the script does not rerun and the build stays green; `touch crates/before/build.rs && cargo build -p before` then fires the check.

### crate-root-6: Island summaries denominate every bound "in total input bytes" while the datasets carry a per-operation `size_measure`
- Where: crates/before/build.rs:227-235 (related: crates/before/build.rs:212-222, crates/before/docs/fuelscape.js:758, crates/before/docs/fuelscape.js:819, crates/before/fuelscape/rank_add.json, crates/before/fuelscape/party_fromstr.json)
- Class / severity / confidence: claim / low / high
- Provenance: verified (python3 read of `op.size_measure` in rank_add.json: "total packed bytes of the two versions whose ranks are added, split uniform (ranks derived by Version::rank in preparation)"; party_fromstr.json: "packed bytes of the sampled value, rendered to text by Display (the value measure pushed through rendering — not uniform ..."; fuelscape.js:758 shows `size_measure` only in the provenance tooltip and :819 hardcodes the x-axis caption); executed: no
- Seen by: claims; refutation: confirmed; history: already-known (the fuelscape note records the widget's uniform bytes denomination as a decision and, as open item 4, that `size_measure` should drive the x-axis caption, "currently hardcoded"; that item is unimplemented at fuelscape.js:819, and the summary and noscript strings here are two more sites of the same convention)
- Owner-gated: no

The visible summary and the no-JavaScript fallback both say "in total input bytes", but for the Rank rows the x-axis is the packed bytes of the versions the ranks were derived from (the inputs are ranks), and for the FromStr rows it is the packed bytes of the value the text renders (the input is text). The doctrine asks that every amplification claim state its denominator; the datasets do, and the two readers without the widget see a different one.

Evidence:

   227      format!(
   228          "<details class=\"toggle fs-details\"><summary>{variant}\
   229           <span class=\"fs-claim\"><code>O({claim_html})</code> \
   230           in total input bytes</span>; {contract}</summary>\
   231           <div class=\"fuelscape\"><script type=\"application/json\">{data}</script></div>\
   232           <noscript><p>The interactive chart requires JavaScript; the bound \
   233           is O({claim_html}) in total input bytes.</p></noscript>\
   234           </details>\n"
   235      )

Resolution: Give the widget data a short denominator phrase beside `size_measure` (or derive one from its first clause) and interpolate it into the summary and noscript strings; this is the same change the note's open item 4 wants for the x-axis caption, done once for all three readers. Acceptance: the rendered summary for `rank_add` names the versions' packed bytes and for `party_fromstr` the value's packed bytes; the constant phrase no longer appears in build.rs.
Construction: Open the rendered docs for `Rank::add` with JavaScript disabled: the noscript text reads "in total input bytes" while the dataset's x-axis is the packed bytes of two versions the ranks were derived from.

### crate-root-7: auto_traits.rs claims every public API type but omits `Limbs`, `TooWide`, and the whole `shape` module, and surfacecheck defers totality to it
- Where: crates/before/src/auto_traits.rs:1-32 (related: crates/before/src/lib.rs:431, crates/before/src/error.rs:52-54, crates/before/src/shape.rs:88-345, crates/before/surfacecheck/src/extract.rs:21-24, crates/before/surfacecheck/src/extract.rs:267-269)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (the 25 pins read against `pub struct`/`pub enum` in shape.rs at 88, 108, 118, 128, 146, 197, 255, 345, `pub use version::{Limbs, ...}` at lib.rs:431, and `TooWide` at error.rs:54; extract.rs:21-24 and 267-269 read; the only other pins in src are the not-impl pins in clock.rs and party.rs); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: no-rationale-found for the drift (the paired design was deliberate at d60b7570 with a scope doc a431eaf1d trimmed to the one-liner; 46eb64f9 added the shape items, Limbs, and TooWide and updated surfacecheck's census but not this file). The history pass disputes the lenses' inclusion of `causally::{Down, Up, Neutral}`: `Polarity: sealed::Sealed + Send + Sync + 'static` (polarity.rs:225) plus the pinned `Query<'static, Down>`/`Query<'static, Up>` already hold them, and I agree, so they are dropped from the list.
- Owner-gated: no

The roster is a hand-maintained enumeration of a fact the code changes without touching it, and the census tool that walks the rustdoc JSON item list excludes auto-trait rows on the strength of this file's totality, so nothing checks the roster against the public surface. The omitted types are `Send + Sync + Unpin` today (borrowed walks over cursors, `Ticks` payloads, `slice::Chunks`), so this is a gap in the pin, not a shipped defect.

Evidence:

     1  //! Compile-time pins on the auto traits of every public API type.
    ...
    26  assert_impl_all!(crate::error::Crossed: Send, Sync, Unpin);
    27  assert_impl_all!(crate::error::Decode: Send, Sync, Unpin);
    28  assert_impl_all!(crate::error::Overlap: Send, Sync, Unpin);
    29  assert_impl_all!(crate::error::Parse: Send, Sync, Unpin);

    extract.rs:
   267  /// Compiler-synthesized auto-trait impls are excluded — `before` pins
   268  /// `Send`/`Sync`/`Unpin` on every public API type at compile time in
   269  /// `src/auto_traits.rs`, which is where that guarantee is reviewed —

Resolution: Add pins for `crate::Limbs<'static>`, `crate::error::TooWide`, and each `crate::shape` type (`Plateau`, `Rise`, `Region`, `Cell<1>`, `Plateaus<'static>`, `Regions<'static>`, `Overlay<'static>`, `Cells<'static, 1>`). Then close the drift path mechanically: have surfacecheck's census compare the set of public struct and enum paths it already extracts against the names pinned in this file (a text scan suffices), so an unpinned public type fails `just surface-totality`; the exclusion rationale at extract.rs:21-26 and 267-269 then names that check. Acceptance: deleting one `assert_impl_all!` line fails a gate leg naming the type; the added pins compile.
Construction: On a branch, add a `PhantomData<*const ()>` field to `shape::Plateaus` and run `just gate`: no pin names `Plateaus` and surfacecheck skips auto-trait rows by design, so a `!Send` public type ships with every instrument green.

### crate-root-8: Em-dashes in `//` comments across the partition
- Where: crates/before/src/borsh_impls.rs:89-91 (related: crates/before/src/borsh_impls.rs:113, crates/before/src/borsh_impls.rs:161, crates/before/src/borsh_impls.rs:290-291, crates/before/src/borsh_impls/tests.rs:296-297, crates/before/build.rs:79, crates/before/build.rs:132, crates/before/Cargo.toml:90-91, crates/before/Cargo.toml:100, crates/before/src/lib.rs:60, crates/before/src/lib.rs:66-67, crates/before/src/lib.rs:133, crates/before/src/lib.rs:208)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the partition files and `grep -n '—' Cargo.toml`; lib.rs doctest comments grepped for both dash forms); executed: no
- Seen by: prose; refutation: confirmed; history: contradicts-hard-rule (the owner's comment rule entered the dotfiles 2026-08-10; most sites predate it, build.rs:79 and :132 postdate it)
- Owner-gated: no

The owner's rule prefers colons or semicolons over em-dashes in `//` comments and log messages; rendered rustdoc is exempt. lib.rs's doctest comments mix em-dashes (60, 66-67, 133) with `--` (208) in one Quickstart.

Evidence:

    89      // The rich error type, not `cursor::Truncated`: this is the boundary
    90      // where `Decode::Io` enters, and it is constructed only when a read
    91      // actually fails — never on the per-bit success path.

Resolution: Rewrite the listed `//` sites with colons or semicolons; pick one convention for the lib.rs doctest comments. Acceptance: the grep above returns nothing over the partition; lib.rs doctest comments use one dash form.

### crate-root-9: borsh_impls.rs: `Ranked`'s decoder re-inlines `Rank`'s, the one-byte read is spelled three times, and full paths sit beside their imports
- Where: crates/before/src/borsh_impls.rs:242-247 (related: crates/before/src/borsh_impls.rs:94-99, crates/before/src/borsh_impls.rs:211-220, crates/before/src/borsh_impls.rs:141, crates/before/src/borsh_impls.rs:277-281, crates/before/src/borsh_impls.rs:74, crates/before/src/borsh_impls.rs:8)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (lines 242-247 read against 213-218; `BorshDeserialize` is in scope at line 14; the `read_exact` idiom at 96-97, 214-215, 243-244; full `crate::version::skyline::` paths at 141, 277, 281 while tests.rs:16 imports the same items); executed: no
- Seen by: structure, prose; refutation: confirmed; history: no-rationale-found (8519dd4789 inlined the closure instead of calling `Rank::deserialize_reader`; the stray parenthesis is from 5d167a636). The prose lens also flagged the sentence "Borsh is a transport for the one wire form, never a second format" repeated at 199, 225, 259; the history pass disputes that half, since `borsh_impls` is a private module whose doc never renders while each impl doc renders on the public type's page, and I agree, so that half is dropped.
- Owner-gated: no

The composite decoder should read as "a rank, then a version, then the cross-check"; instead its first six lines are byte-identical to `Rank::deserialize_reader`'s body. Two prose typos ride along: line 74 closes a parenthesis never opened, and line 8's "in-memory wire form" contradicts itself.

Evidence:

   242          let rank = decode_rank_stream(|| {
   243              let mut byte = [0];
   244              reader.read_exact(&mut byte).map_err(Decode::Io)?;
   245              Ok(byte[0])
   246          })
   247          .map_err(decode_error)?;
    ...
    74      /// storage form, adopted without a copy).
    ...
     8  //! borsh stream while preserving their in-memory wire form.

Resolution: Replace 242-247 with `let rank = Rank::deserialize_reader(reader)?;`; add `fn read_byte<R: Read>(reader: &mut R) -> Result<u8, Decode>` used by `read_bit` and `Rank::deserialize_reader`; import `validate_from`, `validate_dominating_from`, and `Admission` from `version::skyline` at the top; drop the `)` at 74; at 8 write "while preserving their canonical bytes". Acceptance: the differential suite, the pair matrix, and the genre pins in borsh_impls/tests.rs stay green; no `crate::version::skyline::` path remains in the file.

### crate-root-10: before's borsh tests cite a rumors module (`crate::bookmark`) and rumors' protocol
- Where: crates/before/src/borsh_impls/tests.rs:224-226 (related: crates/before/src/borsh_impls/tests.rs:261-263)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'bookmark\|gossip protocol' crates/before/src` returns only lines 225 and 263; no bookmark module exists under crates/before/src); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed; history: no-rationale-found (written into this file at b7fb409f0, a rumors-feature commit, when `bookmark` was already the root crate's module; cross-crate from birth)
- Owner-gated: no

`crate::bookmark` resolves to nothing in `before` (rustdoc never compiles `cfg(test)` module docs, so no gate catches the dangling link), and the test's motivation is stated in a consumer's vocabulary. The invariant the test protects stands on its own in before's terms (lines 228-234: a normalizing `join` sheds bits the freeze must seal behind canonical padding).

Evidence:

   224  /// Regression: a `Party` grown by [`join`](Party::join) — the operation
   225  /// [`reclaim`](crate::bookmark) drives on a reboot — must survive the borsh
   226  /// wire round-trip.
    ...
   263      /// through borsh — the on-wire form the gossip protocol ships.

Resolution: Drop the `reclaim`/`bookmark` clause and the "gossip protocol" clause; keep the mechanism sentences. Acceptance: `grep -rn 'bookmark\|gossip protocol' crates/before/src` is empty.

### crate-root-11: Five differential proptests share one body
- Where: crates/before/src/borsh_impls/tests.rs:434-451 (related: crates/before/src/borsh_impls/tests.rs:461-480, crates/before/src/borsh_impls/tests.rs:549-574, crates/before/src/borsh_impls/tests.rs:589-608, crates/before/src/borsh_impls/tests.rs:693-728)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the five bodies read; each opens two `&[u8]` readers, asserts equal remaining length, and matches `(Ok, Ok)`/`(Err, Err)`/diverged; only the span test adds re-encode checks in its `Ok` arm at 713-723; both sides are already `io::Result<T>` via `decode_error`); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (accreted per type)
- Owner-gated: no

The differential discipline (consumption asserted unconditionally, error genre compared by discriminant) is the valuable part and should have one spelling, so a tightening reaches all five legs at once, the same argument fold.rs:1-3 makes for the fold.

Evidence:

   441          prop_assert_eq!(
   442              subject_reader.len(),
   443              oracle_reader.len(),
   444              "byte consumption diverged",
   445          );
   446          match (subject, oracle) {
   447              (Ok(s), Ok(o)) => prop_assert_eq!(s, o),
   448              (Err(s), Err(o)) => assert_same_error(&s, &o)?,
   449              (s, o) => prop_assert!(false, "accept/reject diverged: {:?} vs {:?}", s, o),
   450          }

Resolution: A helper `fn assert_wire_matches_reference<T: PartialEq + Debug>(stream: &[u8], subject: impl FnOnce(&mut &[u8]) -> io::Result<T>, oracle: impl FnOnce(&mut &[u8]) -> Result<T, Decode>) -> Result<Option<(T, usize)>, TestCaseError>` returning the accepted value and consumed length, so the span test can add its re-encode checks; the five bodies become one call each. Acceptance: five one-line proptest bodies; the helper's doc states the three agreements it asserts.

### crate-root-12: borsh_impls/tests.rs: qualified paths beside their imports, hand counts, a likelihood argument, and formatting slips
- Where: crates/before/src/borsh_impls/tests.rs:829-831 (related: crates/before/src/borsh_impls/tests.rs:358, 836, 846, 876, 882, 887, 895, 911, 919, 921, 930, 934, 938, 959, 960, 976, 978, 979, 1013, 1099, 1233; 776-777; 1074-1077; 1157; 685; 1065; 1198; 1213; crates/before/src/shape/tests.rs:58-60)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (all sites read with line numbers; line 17 imports `Clock, Party, Rank, Ranked, Version` and line 7 `codec`; line 16 imports `validate_dominating_from, Admission` while 358 spells `crate::version::skyline::validate_from`); executed: no
- Seen by: structure, prose; refutation: confirmed; history: no-rationale-found (the counts predate the no-hand-counts rule; the misindent and long line are inside `proptest!` bodies rustfmt does not reach)
- Owner-gated: no

With the imports in scope the file still writes `crate::Rank`, `crate::Ranked`, `crate::Clock::seed()`, and `crate::version::Rank::from_raw(crate::codec::Base::from(..))` (the same type as `Rank`, spelled two ways); `shape/tests.rs:58-60` spells `core::cmp::Ordering::{Equal, Less, Greater}` three times unimported. The pair-matrix doc carries hand counts ("six wire types", "36 ordered pairs", "6x6") for a matrix the macro at 1158-1173 defines structurally; line 685 runs far past the wrap width; line 1065 is indented 12 spaces where its siblings use 8; borsh's u32 length prefix is a bare `4` at 1198 and 1213. Line 776 argues from likelihood ("far deeper than any real id or event tree") where the doctrine wants the mechanism, which lines 785-787 already state.

Evidence:

   829          crate::Rank::ZERO,
   830          Version::try_from(7).unwrap().rank(),
   831          crate::version::Rank::from_raw(crate::codec::Base::from(1u8), 40),
    ...
   776  /// Trees far deeper than any real id or event tree round-trip on the wire
   777  /// path with the parse frames grown on the heap.
    ...
  1074  /// Every ordered pair of the six wire types composes adjacently in one
  1075  /// borsh stream with exact parse boundaries.
  1076  ///
  1077  /// Self-delimitation totality across types: for each of the 36 ordered

Resolution: Use the imported names; import `validate_from` at 16 and `Ordering` in shape/tests.rs; name the prefix (`const BORSH_LEN_PREFIX: usize = 4;`); state the matrix as "every ordered pair of the borsh-implementing types" without the tally; rewrap 685; fix the indent at 1065; reword 776-777 to the mechanism ("at DEPTH 48 the parser's explicit frame stack regrows several times mid-parse ..."). Acceptance: `grep -n 'crate::Rank\|crate::Ranked\|crate::Clock\|crate::version::Rank\|crate::codec::Base' crates/before/src/borsh_impls/tests.rs` is empty; no numeral type count in the testdocs; no likelihood claim in the deep-tree testdoc.

### crate-root-13: `error` module's listing sentence is a joke, and three error types derive `Default` nothing defaults
- Where: crates/before/src/error.rs:1-1 (related: crates/before/src/error.rs:15, crates/before/src/error.rs:33, crates/before/src/error.rs:52)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep for `Overlap::default|Crossed::default|TooWide::default|unwrap_or_default` over crates/before/src and src: the only `unwrap_or_default` hits are on `Option<Version>` at version.rs:1364,1377 and unrelated rumors sites); executed: no
- Seen by: structure, prose; refutation: confirmed; history: the summary is the owner's own hand (a431eaf1d), so deliberate taste; the derives have no rationale (46184a6a6e, 0068220ae1, 46eb64f9 copying the list)
- Owner-gated: yes (the summary is the owner's wording; removing a derive from a public type is an API change)

A doc comment's first sentence stands alone in the crate's module listing, and this one orients nobody. Separately, `Overlap`, `Crossed`, and `TooWide` carry a `Default` impl the crate now maintains as public API with no caller.

Evidence:

     1  //! What could possibly go wrong?
    ...
    15  #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, thiserror::Error)]

Resolution: A first sentence such as "The error types every fallible operation returns.", with the joke kept as a second line if wanted; consider dropping `Default` from the three unit structs if no consumer relies on it. Acceptance: the module listing shows an orienting summary.

### crate-root-14: `Overlap`'s first sentence names only `Clock::sync`; `sync_all` also returns it
- Where: crates/before/src/error.rs:5-5 (related: crates/before/src/clock.rs:377, crates/before/src/clock.rs:408, crates/before/src/clock.rs:413)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (clock.rs:377 `pub fn sync_all<'a, I>(&mut self, iter: I) -> Result<&Version, Overlap>` with `return Err(Overlap)` at 408 and 413); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (written at c51dd3f2bc when `sync` was the only producer; `sync_all` landed at a6dcfbb4f)
- Owner-gated: no

A one-method enumeration in the first sentence rots when a second producer exists.

Evidence:

     5  /// Two parties were not disjoint during [`Clock::sync`](crate::Clock::sync).

Resolution: "Two parties were not disjoint when synchronizing clocks ([`Clock::sync`], [`Clock::sync_all`])." Acceptance: the doc names every public producer of `Overlap` or none by method.

### crate-root-15: `Decode::Io` carries its `io::Error` only in the message, not as `source()`
- Where: crates/before/src/error.rs:89-91 (related: crates/before/src/borsh_impls.rs:145-150, crates/before/src/testing/snapshots.rs:276-280)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (thiserror-impl-2.0.20 selects a source field only when named `source` (prop.rs:105) or attributed `#[source]`/`#[from]` (attr.rs:82); the field here is tuple position 0 with no attribute; borsh_impls.rs:147 unwraps the cause by pattern match); executed: no
- Seen by: structure, correctness; refutation: confirmed, low and owner-gated; history: no-rationale-found (the variant switched from `ErrorKind` to `io::Error` at dc88e755b with an empty commit body; no `#[source]`/`#[from]` ever existed)
- Owner-gated: yes (`source()` moving from `None` to `Some` is an observable change on a stable public error type, though a non-breaking one)

`std::error::Error::source()` returns `None` for `Decode::Io`, so chain-walking reporters (and any `Box<dyn Error>` consumer, including serde's `D::Error::custom`) reach the underlying io error only through the Display text. The crate wants explicit construction (seven `map_err(Decode::Io)` sites), so `#[source]` rather than `#[from]` is the fit; the Display string can stay.

Evidence:

    89      /// The underlying reader failed.
    90      #[error("read error: {0}")]
    91      Io(io::Error),

Resolution: `Io(#[source] io::Error)`; keep the Display string so the snapshot at testing/snapshots.rs:276 is unchanged. Acceptance: a test asserts `std::error::Error::source(&Decode::Io(io::Error::from(io::ErrorKind::UnexpectedEof))).is_some()`.

### crate-root-16: `Parse`'s doc omits `Ticks` and asserts paper notation for a decimal parse
- Where: crates/before/src/error.rs:94-98 (related: crates/before/src/version/ticks.rs:205-212, crates/before/src/error.rs:47-51)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (ticks.rs:205-206 `impl FromStr for Ticks { type Err = Parse;` parsing ASCII digits; error.rs:49 itself parses a `Ticks` in the `TooWide` example); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (the producer list is from dc88e755be; `Ticks` gained `FromStr` at 56a08f907; ebcf0087f7 re-edited the second sentence afterwards without adding it)
- Owner-gated: no

Public error docs are where a user learns which operations can produce the error; the list misses one producer, and "the original paper's notation" is false for the decimal count.

Evidence:

    94  /// Why a string or Rust literal failed to parse into a [`Party`](crate::Party),
    95  /// [`Version`](crate::Version), or [`Clock`](crate::Clock).
    96  ///
    97  /// Parsing uses the original paper's notation and strictly rejects
    98  /// non-canonical input.

Resolution: Add [`Ticks`](crate::Ticks) and qualify: "Party, Version, and Clock parse the paper's notation; Ticks parses a decimal count. Every parser strictly rejects non-canonical input." (Or drop the list: "into one of the crate's `FromStr` types".) Acceptance: the doc names every `FromStr` impl whose `Err = Parse`, or names none by type.

### crate-root-17: fold.rs states operand-size balance the counter does not provide
- Where: crates/before/src/fold.rs:5-8 (related: crates/before/src/fold.rs:55, crates/before/src/meter/board/ceilings.rs:247-250)
- Class / severity / confidence: claim / low / high
- Provenance: verified (the counter merges on `*w == weight`, equal input counts, at line 55; nothing bounds packed-size ratios; ceilings.rs:247-250 states the argument the `O(D log k)` bound actually rests on: "every input passes through `O(log k)` joins, and each join level re-scans the operands it merges"); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (lines 5-8 are from the b3f09baa0 squash, not the fold's author commit)
- Owner-gated: no

Every bound needs an argument the code implements. The sentence offers one (bounded operand-size ratio) the code does not implement, while the correct one (each counter level's groups partition the inputs, so per-level work is `O(D)`) is what the board's fold ceiling and the `*_log_factor_is_alive` pins rest on. A maintainer reading fold.rs alone would defend the wrong invariant.

Evidence:

     5  //! An incoming operand merges upward while the top stack entry holds as many
     6  //! inputs as it does, so every input passes through `O(log k)` combines against
     7  //! similarly sized partners and no combine's operand is more than a bounded
     8  //! factor larger than its partner. A sequential left fold instead combines

Resolution: "so each input passes through `O(log k)` combines, each pairing two groups holding equally many inputs; because the groups at any counter level partition the inputs, one level's combines cost `O(D)` in total packed size and the whole fold `O(D log k)`." Acceptance: the cost argument mentions input-count balance and per-level partition only; no claim about operand packed-size ratio remains.
Construction: Two inputs, a one-leaf version and `Shape::Dense.packed1(125_000)`: `balanced_reduce` performs exactly one combine whose operands differ in packed size by about five orders of magnitude, contradicting the "bounded factor" clause while the `O(D log k)` bound holds trivially at `k = 2`.

### crate-root-18: The closing drain hands back coalesced groups, and the public `# Errors` prose promises each input is merged or handed back
- Where: crates/before/src/fold.rs:38-40 (related: crates/before/src/party.rs:312-315, crates/before/src/party.rs:353-366, crates/before/src/clock.rs:234-237, crates/before/src/laws.rs:2406-2410)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read; the construction below was traced by hand against party.rs:353-366, not run); executed: no
- Seen by: correctness; refutation: reframed from correctness to documentation precision (no region is lost; the clause already says the absorbed/handed-back split is unspecified; reachable only through aliased input); history: deliberate-and-holds for the mechanism (fold.rs:33-36 documents the over-full slot; laws.rs:2406-2410 pins conservation over region unions and states that "an element-wise identity clause would reject correct behavior"), so the finding narrows to the prose
- Owner-gated: no

On aliased input a refused group is a union of several inputs, and `Party::join_all`/`Clock::join_all` push that union to the returned `Vec`. The `# Errors` sections say "every input [`Party`] is either merged into `self` or handed back", which the fused element does not satisfy element-wise; the laws state the correct contract (the union of handed-back regions equals the union of the unmerged inputs' regions). Judge by the clause, not the likelihood of aliasing.

Evidence:

    38  /// The returned groups are in stack order, oldest (heaviest) first, for
    39  /// the caller's closing drain: a left-to-right fold over them keeps
    40  /// every combine's left operand the older group.

    party.rs:
   312      /// Returns the parties which *overlapped* and so could not be folded in,
   313      /// dropping nothing: every input [`Party`] is either merged into `self` or
   314      /// handed back. In case of partial error, the set of parties which are
   315      /// absorbed vs. handed back is unspecified.

Resolution: Amend both `# Errors` sections to state the union contract ("a handed-back element may be the union of several inputs whose regions could not be folded in; the handed-back regions together are exactly the unmerged inputs' regions"), and add one sentence to fold.rs's drain paragraph saying the drain may refuse a coalesced group. Acceptance: a committed unit test constructs the case below and asserts `err.len() == 1` with `err[0]` equal to the union `c | d`; the public prose matches the test.
Construction: `let mut s = Party::seed(); let mut x = s.fork(); let mut b = x.fork(); let d = b.fork(); let a = x; let c = a.dangerously_alias();` then `s.join_all([a, b, c, d]).unwrap_err()`: `a` and `b` coalesce at weight 1; `c` waits at weight 0 and coalesces with `d`; `combine(ab, cd)` overlaps on `(0, (1, 0))` so both stay on the stack; the drain joins `ab` into `s` and refuses `cd`, so the error is `vec![cd]` with `cd == "(0, (1, (0, 1)))"`, a party that was never an input.

### crate-root-19: `Vec::pop_if` removes both `expect`s and the `Option` juggling in the counter loop
- Where: crates/before/src/fold.rs:53-78 (related: rust-toolchain.toml:29)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (read; `Vec::pop_if` is stable since 1.86.0 and rust-toolchain.toml:29 pins `channel = "1.97.1"`; the rewrite's borrow-flow was checked by reasoning, not compiled); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (peek-then-pop is habit, not a compatibility choice; no `pop_if` is used anywhere in crates/before/src)
- Owner-gated: no

Two proof-carrying `expect`s and an `Option` exist only to satisfy the borrow checker; `pop_if` is the peek-and-pop, and a labeled `continue 'outer` in the `Err` arm lets `merged: T` be a plain value.

Evidence:

    55          while stack.last().is_some_and(|(_, w)| *w == weight) {
    56              let (top, _) = stack.pop().expect("the loop condition saw a top entry");
    57              match combine(
    58                  top,
    59                  merged.take().expect("the operand is held while merging up"),

Resolution: `'outer: for item in iter { if !accept(&item) { rejected.push(item); continue; } let mut merged = item; let mut weight = 0u32; while let Some((top, _)) = stack.pop_if(|(_, w)| *w == weight) { match combine(top, merged) { Ok(group) => { merged = group; weight += 1; } Err((top, back)) => { stack.push((top, weight)); if weight == 0 { rejected.push(back) } else { stack.push((back, weight)) } continue 'outer; } } } stack.push((merged, weight)); }`. Acceptance: `tests/fold_skeleton.rs` and the fold users' suites stay green; no `expect` remains in fold.rs.

### crate-root-20: fold.rs's hand-maintained caller list omits `Span`'s fold
- Where: crates/before/src/fold.rs:86-90 (related: crates/before/src/span/algebra.rs:339-341, crates/before/src/span/algebra.rs:369)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `balanced_reduce` outside fold.rs: version.rs:653, :806, and span/algebra.rs:369 inside `fold_endpoints`, the four span operators' shared fold); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (b3f09baa0 both wrote the list and added the span caller, so it was incomplete from birth)
- Owner-gated: no

No hand-maintained enumerations of callers; the list has already drifted from the code.

Evidence:

    86  /// [`balanced_try_fold`] with every input accepted and the closing
    87  /// drain folded in: the receiver-seeded version folds
    88  /// (`Version::join_all`, `Version::meet_all`, `Version::span_all`)
    89  /// never see the `None`, and the seedless `Sum`/`FromIterator` doors
    90  /// restore the join's identity (the empty version) over it.

Resolution: State the two caller shapes without naming them: "Receiver-seeded callers never see `None`; seedless callers restore their operator's identity over it." Acceptance: fold.rs names no specific caller, or `git grep balanced_reduce` matches the list exactly.

### crate-root-21: "door" is crate-wide jargon that no site defines
- Where: crates/before/src/fold.rs:89-89 (related: crates/before/src/serde_impls.rs:11, crates/before/src/shape/tests.rs:1, crates/before/src/borsh_impls/tests.rs:115-116, crates/before/src/party.rs:14, crates/before/src/surface.rs)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -rni '\bdoors\?\b' crates/before/{src,tests}` counts 224 lines; a grep for definitional phrasings finds usages only); executed: no
- Seen by: prose; refutation: confirmed, nit; history: no-rationale-found (crate vocabulary since the itc-to-before rename; the owner's lexicon lists "door | entry point" as default-dialect but "not banned as such")
- Owner-gated: no

The vocabulary rule wants every coined term anchored to an identifier or defined once by contrast. "door" (a public entry through which a value enters or leaves the process: decode/encode, FromStr/Display, the serde and borsh impls) is met as established vocabulary everywhere and defined nowhere; "genre" (a `Decode`-variant class) in the serde and borsh tests is similar. It reaches public rustdoc only through the feature-gated `laws` and `meter` modules.

Evidence:

    89  /// never see the `None`, and the seedless `Sum`/`FromIterator` doors

Resolution: Define once where the entries live (codec.rs's or lib.rs's private docs): "A *door* is a public entry through which a value enters or leaves the process: `decode`/`encode`, `FromStr`/`Display`, and the serde and borsh impls." Or replace with "entry point" at the public-adjacent sites. Acceptance: a grep for "door" finds one definition-by-contrast the other uses read against, or the word is gone from maintainer prose.

### crate-root-22: `balanced_reduce`'s `debug_assert` checks a property its own closures make impossible
- Where: crates/before/src/fold.rs:97-100 (related: crates/before/src/fold.rs:41-81)
- Class / severity / confidence: scaffolding / nit / high
- Provenance: verified (read: with `accept = |_| true` and `combine = |a, b| Ok(combine(a, b))`, `balanced_try_fold` has no path that pushes to `rejected`); executed: no
- Seen by: correctness; refutation: confirmed (debug-only and O(1), so a taste call resting on redundancy alone); history: no-rationale-found (added by b4093db21 two days after the campaign's assertions doctrine, which nothing exempts it from)
- Owner-gated: no

A guard earns its place by naming a failure it catches that the tests cannot; this one restates the two literal closures above it.

Evidence:

    97      debug_assert!(
    98          rejected.is_empty(),
    99          "an infallible combiner rejects nothing"
   100      );

Resolution: Delete the assert; if the invariant is wanted in types, split the infallible path so no `rejected` Vec exists to inspect. Acceptance: fold.rs has no recompute-and-compare assert; `just gate` green.

### crate-root-23: Unclosed backtick in the `Clock::forks` intra-doc link
- Where: crates/before/src/iter.rs:9-10
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read; line 9's `Party::forks` link has both backticks, line 10's lacks the closing one; the target resolves, so rustdoc's link lints stay quiet); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: no-rationale-found (present since the file's creation at a431eaf1d)
- Owner-gated: no

The link text opens a code span it never closes, so the rendered module intro shows a stray backtick.

Evidence:

     9  //! See [`Party::forks`](crate::Party::forks) and
    10  //! [`Clock::forks](crate::Clock::forks).

Resolution: `//! [`Clock::forks`](crate::Clock::forks).` Acceptance: the rendered `before::iter` page shows both links as code spans.

### crate-root-24: "Approximately 100× more space-efficient than a naïve transcription" has no committed measurement
- Where: crates/before/src/lib.rs:3-4 (related: crates/before/results/space_consumption/README.md:27-41, crates/before/examples/space_consumption.rs, crates/before/src/oracle.rs)
- Class / severity / confidence: claim / medium / medium
- Provenance: verified (grep for `100×|100x|naïve transcription|naive transcription` over crates/before/{src,examples,results,benches,tests}: only lib.rs matches; results/space_consumption/README.md:29-34 compares against the paper's Appendix A encoding and reports parity to modest wins); executed: no
- Seen by: prose, claims; refutation: confirmed; history: no-rationale-found (a431eaf1d, the owner's edit, replaced 67970b75c's provenance-tagged "between 2–20× faster (measured: the workspace bench suite ...)" with this figure and no artifact)
- Owner-gated: yes (the owner's own headline; a measurement may exist off-tree)

A number you were handed is a hypothesis: this is the first number a reader sees, and nothing in the tree produces it. The referent is also undefined: if "naïve transcription" means the `oracle` module's boxed trees, no program measures their footprint; if it means the paper's own Appendix A encoding, the committed table says parity, not two orders of magnitude.

Evidence:

     3  //! using a compact representation which is approximately 100× more
     4  //! space-efficient than a naïve transcription of the original paper, while

    results/space_consumption/README.md:
    31  | Data, 100k iters   |        128 |                ~3262 B  | "< 2900 B" (chart ~3000–4000) |

Resolution: Either commit the measurement (extend `examples/space_consumption.rs` to record the oracle values' in-memory or boxed-node encoded size beside `encode().len()` at each checkpoint, and quote the observed ratio with its scenario) or drop the multiplier and say what the README supports. Acceptance: the figure is reproduced by a committed example or CSV the docs name, or is absent; `just readme` regenerated.
Construction: Compute both sizes for one population: `Clock::encode().len()` against the oracle tree's size under any naive spelling (boxed nodes with u64 counts, or the paper's Appendix A bits). No committed program does this, so the ratio is unchecked; if it is not about 100 at the paper's parameters the sentence is false as written.

### crate-root-25: The headline promises "asymptotically linear" performance; the crate's own contracts are superlinear for rank, text I/O, the render merge, and the folds
- Where: crates/before/src/lib.rs:5-6 (related: crates/before/src/lib.rs:350-358, crates/before/src/version.rs:293, crates/before/src/version.rs:1058, crates/before/src/version/ticks.rs:41-48, crates/before/fuelscape/version_rank.json, crates/before/src/testing/asymptotics.rs:1-12)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (python3 read of fuelscape/version_rank.json: `op.claim` = "n (log n)^2", `op.contract` = "`O(M(|self|) · log |self|)` time, `O(|self|)` space"; version.rs:293 and :1058 "`M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation)"; ticks.rs:42-43 "text I/O is superlinear but subquadratic"; asymptotics.rs:5-7 names the log factor, the render merge's superlinear growth, and the settle's multiplication-bound case as live pins); executed: no
- Seen by: prose, claims; refutation: confirmed; history: no-rationale-found (the headline is a431eaf1d and the hard-guarantee paragraph bbb9f8023, both the owner's hand; the superlinear contracts were already in the tree)
- Owner-gated: yes (the owner's own headline; the intended meaning, probably "linear for the core operations", is not stated)

Lines 351-353 make every asymptotic claim a hard guarantee, so the headline is itself a guarantee, and it is one the crate's asymptotics suite holds alive counterexamples to. The careful statement already exists in the per-operation `# Complexity` sections; the opening sentence overstates it.

Evidence:

     5  //! maintaining asymptotically linear and practically quick performance even
     6  //! over the most adversarially pessimal inputs.
    ...
   351  //! pathological input shapes. Any asymptotic claim is a hard guarantee that the
   352  //! operation will perform in time proportionate to that bound, for all input
   353  //! sizes, no matter how unlikely and contorted the shape of the input.

    version.rs:
   293      /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).

Resolution: Reword to the bound the crate guarantees, for example "while every operation carries a documented, guaranteed time bound: linear in the encoded input for the core operations (tick, fork, join, comparison, the codecs), near-linear for the n-ary folds, and multiplication-bound only where the answer itself is a wide integer (rank and its relatives)"; then `just readme`. Acceptance: the opening paragraph names no bound any `# Complexity` section exceeds; the words "asymptotically linear" do not stand as a crate-wide promise while `fuelscape/version_rank.json` carries contract `O(M(|self|) · log |self|)`.
Construction: Textual: the rendered docs for `Version::rank` state `O(n (log n)^2)` in total input bytes on the same page set whose front page states "asymptotically linear"; the instruments that would fail a literal linear claim are already committed and green (`render_merge_superlinearity_is_alive`, `version_join_all_log_factor_is_alive`).

### crate-root-26: "mint" for constructing a value, in the Quickstart and crate-wide
- Where: crates/before/src/lib.rs:49-49 (related: crates/before/src/serde_impls/tests.rs:111, crates/before/src/party.rs:15, crates/before/src/party.rs:872, crates/before/src/version/skyline.rs:7, and roughly twenty further sites listed by `grep -rni '\bmint' crates/before/src`)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep; in-partition sites lib.rs:49 and serde_impls/tests.rs:111; crate-wide hits in laws.rs, meter.rs, version.rs, causally/conjunction.rs, party.rs, skyline.rs and its submodules, meter/board/*, including a function named `Reign::mint` in skyline/query/web.rs); executed: no
- Seen by: prose, structure, correctness; refutation: confirmed (a crate-wide sweep, not a partition fix); history: contradicts-hard-rule (the ban is in the owner's live writing-style.md; every site predates its adoption)
- Owner-gated: no

The owner's vocabulary rule: never write "mint" for constructing a value. The Quickstart site is the crate's first code sample and flows into the derived README.

Evidence:

    49  //! // New participants fork off a live clock, never mint themselves.

    serde_impls/tests.rs:
   110  /// the self-describing (number-array) paths, for every rejection genre the raw
   111  /// decodes mint — trailing bytes on each type, the rank-mismatch composite

Resolution: lib.rs:49 "// New participants fork off a live clock; nothing creates a second seed."; serde_impls/tests.rs:111 "every rejection genre the raw decodes produce"; sweep the remaining sites (party.rs:15 "which create a second holder", :872 "Creates identity exactly as ..."; rename `Reign::mint` if the owner wants the rule to reach identifiers). Acceptance: `grep -rni '\bmint' crates/before/src` is empty, or lists only identifiers the owner exempts.

### crate-root-27: Safety rule 2 says bytes are "exactly one hole" in linearity; party.rs names three doors
- Where: crates/before/src/lib.rs:236-238 (related: crates/before/src/party.rs:13-17, crates/before/src/party.rs:534, crates/before/src/party.rs:784, crates/before/src/clock.rs:890, crates/before/src/clock.rs:942)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`pub fn dangerously_alias` at party.rs:534 and clock.rs:890; `impl FromStr for` Party at party.rs:784 and Clock at clock.rs:942; party.rs:13-17 read); executed: no
- Seen by: prose, correctness; refutation: confirmed, low (the bytes hole is the one a user trips unknowingly; the other two are labeled dangerous or as fresh-universe doors at their sites, so the omission misdirects an auditor rather than a user); history: no-rationale-found (the owner's a431eaf1d sentence postdates both other doors and party.rs's three-door enumeration)
- Owner-gated: no

The crate-level rule is the hazard map a user reads first; a reviewer auditing an application for linearity on its strength checks only decode sites. party.rs's own module doc lists the serialization door, the text/literal door, and `dangerously_alias`. Hand-maintained counts ("exactly one") rot.

Evidence:

   236  //!    [`!Clone`](Clone). This leaves exactly one hole: bytes. A
   237  //!    serialized state sidesteps the type system, and
   238  //!    [`decode`](Clock::decode) cannot tell the latest state from a stale

    party.rs:
    13  //! merely (mutably) borrows. The type system enforces that linearity up to the
    14  //! documented escape hatches: the serialization and text/literal doors, which
    15  //! mint a second holder from bytes or notation, and
    16  //! [`dangerously_alias`](Party::dangerously_alias), the deliberate in-memory
    17  //! duplication.

Resolution: State the three doors as party.rs does (bytes via `decode`, serde, and borsh; text notation via `FromStr` and literals; `dangerously_alias`), each creating a second holder of an identity, and keep the bytes paragraph as the one that is easy to trip unknowingly; `just readme`. Acceptance: rule 2 names every public constructor that yields a `Party`/`Clock` sharing identity with an existing handle; no sentence asserts a count of holes.

### crate-root-28: Crate-docs accuracy slips: `PartialEq` where `PartialOrd` is meant, the `'static`-only `Deserialize`, and the unmentioned `surface` module
- Where: crates/before/src/lib.rs:277-277 (related: crates/before/src/lib.rs:30, crates/before/src/lib.rs:392-393, crates/before/src/lib.rs:398-403, crates/before/src/lib.rs:441-442, crates/before/src/serde_impls.rs:89, crates/before/src/serde_impls.rs:109, crates/before/src/borsh_impls.rs:240, crates/before/src/borsh_impls.rs:275)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lib.rs:30's types table names `PartialOrd` for `<`, `<=`, `concurrent`; serde_impls.rs:89 `impl<'de> Deserialize<'de> for Ranked<'static>` and :109 likewise for `Span<'static>`, borsh at 240 and 275; `pub mod surface` gated by `meter` at lib.rs:441-442); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed, low; history: no-rationale-found (line 277 was introduced whole by a431eaf1d, a slip against the line-30 table; the instrument bullet predates `pub mod surface`)
- Owner-gated: no

Three sentences a library user reads to choose a trait bound, a feature, or a field type are wrong or incomplete: `PartialEq` on `Version` is byte equality and the causal ordering belongs to `PartialOrd`; the serde bullet does not say `Deserialize` yields only the owned `Ranked<'static>`/`Span<'static>` (a user deriving `Deserialize` on a struct holding `Span<'a>` hits this without warning); the `meter` bullet omits the `surface` roster the same feature gates.

Evidence:

   277  //! [`Version`]'s [`PartialEq`] describes a causal ordering: `a <= b` tests
    ...
   392  //! - **`serde`:** `Serialize`/`Deserialize` for [`Party`], [`Version`],
   393  //!   [`Clock`], [`Rank`], [`Ranked`], and [`Span`].

Resolution: `[`PartialOrd`]` at 277 (add "its `PartialEq` is byte equality on the canonical encoding" if wanted); append "`Deserialize` yields the owned forms `Ranked<'static>` and `Span<'static>`" to the serde bullet; add the operation roster (`surface`) to the instrument bullet; `just readme`. Acceptance: 277 names `PartialOrd`; the feature list names every `pub mod` a feature enables and states the lifetime of the deserialized view types.

### crate-root-29: The space-efficiency figures name parameters no committed measurement uses
- Where: crates/before/src/lib.rs:312-319 (related: crates/before/results/space_consumption/README.md:8-34, crates/before/results/space_consumption/space.csv, crates/before/examples/space_consumption.rs:9-22, crates/before/build.rs:93-99)
- Class / severity / confidence: claim / medium / high
- Provenance: verified; executed: yes (python3 csv parse of results/space_consumption/space.csv: 444 rows; columns `scenario, entities, iteration, mean_bits, std_bits, mean_bytes, std_bytes, runs`; scenarios {data, process}; entities {4, 8, 16, 32, 64, 128}; max iteration 100000 for data and 25000 for process; no row at 100 entities or 10^6 events, and no party/version split)
- Seen by: claims, prose; refutation: confirmed; history: no-rationale-found (67970b75c introduced the figures tagged "(measured: the space-consumption experiment that draws the figure below)", which even then ran the paper's parameters; a431eaf1d removed the tag and added "steady-state")
- Owner-gated: yes (the owner's paragraph; a run at these parameters may exist off-tree)

The prose quotes sizes at 100 parties and 1,000,000 events (Party about 3 B, Version about 100 B) and under churn (about 50 B and 2,000 B, "linearly in N" and "roughly N²"), directly above a figure derived from an artifact that runs populations 4..128 at 100k/25k iterations and reports whole-stamp sizes (about 146 B static and 3262 B dynamic at 128). The N² reading is plausible (2,000 × (128/100)² is about 3,277 against the measured 3,262) but is an inference the reader cannot check, and the 10^6-event denominator has no committed source.

Evidence:

   312  //! At 100 parties and 1,000,000 events, the expected size of a [`Party`] is
   313  //! about 3 bytes and the expected size of a [`Version`] is about 100 bytes.
    ...
   316  //! bounds. Under sustained random membership churn, those same 100 parties will
   317  //! each stabilize at around 50 bytes (growing linearly in the steady-state
   318  //! number of parties `N`) and their corresponding versions at around 2,000
   319  //! bytes (roughly `N²` in the steady-state number of parties `N`).

    results/space_consumption/README.md:
     8  - `space.csv` — raw measurements (100 runs, paper parameters). Columns:

Resolution: Re-denominate the paragraph to the committed artifact's parameters and quantities (stamp bytes at 128 entities after 100k/25k iterations, static versus dynamic), or commit the run that produces the 100-party / 10^6-event figures (the example takes the population and iteration budget) and cite it; state the N and N² fits as slopes over the committed columns with their band. Acceptance: every number in lib.rs:312-319 is readable from results/space_consumption/space.csv (or a newly committed CSV) at the parameters the prose names, and the growth laws are stated as fits over committed columns.

### crate-root-30: Register and redundancy nits in the crate-level docs and neighbors
- Where: crates/before/src/lib.rs:350-358 (related: crates/before/src/lib.rs:88-90, crates/before/src/lib.rs:244-248, crates/before/src/lib.rs:288-290, crates/before/src/lib.rs:301-308, crates/before/src/lib.rs:381, crates/before/src/recurse.rs:64, crates/before/src/shape.rs:16-17, crates/before/src/shape.rs:65-70)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (each phrase read at its line); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (most lines are the owner's direct edits with one-line commit messages; recurse.rs:64 and shape.rs:17 are not)
- Owner-gated: yes (the owner's own wording)

Every sentence in public rustdoc competes with the contract. "very carefully hardened" (350) and "backstop" (357) are register transplants with a significance adverb; "Consequentially" (381) should be "Consequently"; 244-248 states the cross-seed point twice; the ordering list introduces `Span` twice (288-290 and 301-308); 88-90 repeats "mixing"; recurse.rs:64 "the honest signal" moralizes a counter; shape.rs:17 "actually draw" and :70 "independently-checked witness" put testing apparatus into user docs.

Evidence:

   350  //! The operations in this crate have been very carefully hardened against
   351  //! pathological input shapes. Any asymptotic claim is a hard guarantee that the
    ...
   356  //! organically reachable (i.e. non-adversarial) inputs, with the asymptotic
   357  //! guarantee acting as a backstop in case of pathologically unlikely shapes.
    ...
   381  //! absolute instruction counts in native builds. Consequentially, the cost axis

Resolution: 350 "The operations hold their bounds on every input shape."; 357 "with the bound holding on the rest"; 381 "Consequently"; 244-248 keep one sentence; merge the `Span` bullet into the Filtering bullet's last sentence or move that sentence out; recurse.rs:64 "is the one signal available"; shape.rs:70 drop or move to the tests' module doc. Acceptance: none of the listed phrases remain; `just readme` regenerated.

### crate-root-31: "Every operation is verified differentially against" both references overstates the roster
- Where: crates/before/src/lib.rs:407-410 (related: crates/before/src/surface.rs:72-115, crates/before/src/surface.rs Leg::Excluded rows)
- Class / severity / confidence: claim / low / high
- Provenance: verified (`grep -c 'Leg::Excluded' crates/before/src/surface.rs` = 193; the `Exclusion` enum at 72-115 names the families: no wire format in the references, definitional combinators, n-ary not in the references, linearity/borrowing mechanics); executed: no
- Seen by: claims; refutation: confirmed; history: deliberate-but-expired (the paragraph is from dee3cd61aa, before the surface roster existed; the roster later made it checkably false)
- Owner-gated: no

The roster is the crate's own record of which doors carry which legs; the crate docs should not claim more legs than it records. The honest statement is stronger: differential where a reference exists, law- and battery-pinned where none can.

Evidence:

   407  //! Every operation is verified differentially against the paper's naive
   408  //! recursive implementation as well as a nondeterministic function-space
   409  //! semantics, alongside exhaustive small-scope enumeration of clock shapes,
   410  //! algebraic-law property suites, and fuzzed codecs.

Resolution: "Every operation with a counterpart in the paper is verified differentially against its naive recursive implementation and a nondeterministic function-space semantics; operations with no reference (the codecs, text, the n-ary folds, borrowing mechanics) are pinned on production by algebraic laws, round-trip and strict-rejection batteries, and format goldens; the `surface` roster records which leg each door carries." Acceptance: the Testing paragraph's quantifier matches the `Leg` variants the roster assigns; `just readme`.
Construction: Textual: list the surface.rs rows whose `prod_tree`/`prod_fs` legs are `Leg::Excluded(_)`; each is an operation the sentence claims is differentially verified against both references and is not.

### crate-root-32: The stack-segment counter has no writer in any binary that judges it
- Where: crates/before/src/recurse.rs:16-20 (related: crates/before/src/recurse.rs:37, crates/before/src/recurse.rs:68-86, crates/before/src/recurse.rs:100-109, crates/before/src/recurse.rs:118-129, crates/before/Cargo.toml:33-44, crates/before/src/meter.rs:3552-3558, crates/before/src/meter/board/measure.rs:84-92, crates/before/src/meter/board/ceilings.rs:80-83, crates/before/src/meter/board/judge.rs:60-64, crates/before/src/meter/board/currency.rs:138-140, crates/before/src/meter/board/floors.rs:627-636, crates/before/src/meter/tests.rs:392-432, crates/before/tests/meter.rs:22-26, crates/before/tests/meter.rs:364-391, crates/before/tests/meter.rs:263, crates/before/tests/meter.rs:441, crates/before/tests/amp_board_smoke.rs:96, justfile:903-904)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (the cfg attributes read: the static at 68 and its readers at 77 and 83 compile under `any(test, feature = "meter")`; `grow` at 100 and `descend!` at 118 under `cfg(test)`; the sole `fetch_add` is line 106 inside `grow`; `stacker` appears only under `[dev-dependencies]` at Cargo.toml:44; `grep -rn 'descend!\|recurse::grow'` outside recurse.rs hits only testing/bridge.rs, skyline/grow/tests.rs, and meter/tests.rs; the board reads the counter at measure.rs:84 and :92 and tests/meter.rs at 364 and 371; `just amp-board-acceptance` (justfile:903-904) runs the example binary with `--features limb-meter,scan-meter`; tests/amp_board_smoke.rs:96 calls `board::run` from an integration-test binary; a python parse of tests/meter.rs found 46 `envelope(` rows, every one with `0` in the segments column); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate-but-expired (05bd2b16d made the last library walk iterative, put `grow`/`descend!` under `cfg(test)`, and wrote lines 16-20 in the same diff, reasoning about the lib unit-test binary only; 1ddb5a483 moved `stacker` to dev-dependencies and recorded a keep for the guard; neither commit, the amplification note, nor meter/tests.rs:399-403 addresses that tests/meter.rs and examples/amp_board.rs link a library without `cfg(test)`)
- Owner-gated: yes (removal of an instrument, and a board policy)

Integration tests and examples compile the library without `cfg(test)`, so in `tests/meter.rs` and in the `amp_board` example the counter is a constant zero: every `segments <= env.segments` assertion and the board's `MAX_GROWN_STACK_SEGMENTS = 1` ceiling judge a signal that cannot move, and the liveness dive (`stack_segment_meter_counts_deterministically_and_resets`) runs in a different binary. Independently of which binary, no library kernel can reach `descend!` in any build, so the zero "over the library kernels" is a compile-time fact, not a measurement; a kernel that regressed into stack recursion would crash `clock::tests::deep_tree_stack_safety`, never bump this counter. The prose at 74-76 ("the counter is always written") is false in exactly the builds that read it; `judge.rs:60-64` carries a floor-trip message "so a future segments floor binds without a code change", the empty buffer the doctrine forbids, and no floor can ever bind on a counter nothing writes. Two smaller ghosts ride along: line 37's "`depth % STRIDE`" describes code that reads `is_multiple_of(STRIDE)` (92), and tests/meter.rs:441 ("heap stays flat, segments do not") beside `CMP_DENSE = envelope(30_720, 0, 0, 0)` at :263 is a remnant of the recursive-comparison era.

Evidence:

    16  //! The paper-shaped oracle is clearest written recursively, and the guard is
    17  //! what lets it meet deep inputs safely. The segment counter below stays
    18  //! compiled for the meters: it is the deterministic stand-in for
    19  //! recursion-driven stack consumption, and its zero reading over the library
    20  //! kernels is the measured fact the boards' segments column pins.
    ...
    68  #[cfg(any(test, feature = "meter"))]
    69  static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);
    ...
    74  /// Compiled only for the meter surface: the counter is always written (the bump
    75  /// is inseparable from the growth arm), but nothing outside the meters ever
    76  /// reads it.
    ...
   100  #[cfg(test)]
   101  #[inline]
   102  pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
   103      if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
   104          f()
   105      } else {
   106          SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
   107          stacker::grow(STACK_GROWTH, f)
   108      }
   109  }
    ...
   118  #[cfg(test)]
   119  macro_rules! descend {

    Cargo.toml:
    33  [dev-dependencies]
    ...
    44  stacker = { workspace = true }

    tests/meter.rs:
   364      meter::reset_stack_segments();
    ...
   371      let segments = meter::stack_segments();
    ...
   387      assert!(
   388          segments <= env.segments,

    meter/board/judge.rs:
    60  /// The segments column's floor-trip message (unreachable while segments is
    61  /// ceiling-only by policy; the judgment loop still carries it so a future
    62  /// segments floor binds without a code change).

Resolution: The owner's call between two honest shapes. (a) Dissolve, my recommendation: remove the segments currency from the board (`Currency::Segments`, `seg_ceiling_only()` on every cell, `MAX_GROWN_STACK_SEGMENTS`, `SEG_FLOOR_TRIP`, the render column) and `Envelope.segments` with every `segments:` pin in tests/meter.rs, and `meter::{stack_segments, reset_stack_segments}`; confine `SEGMENTS_GROWN` and its readers to `cfg(test)` beside their one live client, the determinism dive; name `clock::tests::deep_tree_stack_safety` (depth 100k) plus the structural fact that `descend!` is `cfg(test)` and `stacker` a dev-dependency as the instruments against reintroduced depth recursion. (b) Keep and make it live: `stacker` becomes an optional dependency enabled by `meter`, `grow`/`descend!` compile under `any(test, feature = "meter")`, and a guarded 200k-deep descent in tests/meter.rs and in the board's self-check reads `stack_segments() > 0` in each enforcing binary before the kernels' zero is asserted. Either way, restate recurse.rs:16-20 and 74-76 as what IS (the guard and its counter exist for the test-only oracle bridge and its witnesses; in non-test meter builds the counter has no writer), fix line 37, and excise tests/meter.rs:441's clause. Acceptance: (a) `grep -rn 'stack_segments\|SEGMENTS_GROWN\|Currency::Segments' crates/before` finds only the `cfg(test)` determinism witness, and `just gate` is green; (b) a test in crates/before/tests/ built with `--features meter` asserts `before::meter::stack_segments() > 0` after a guarded deep descent. In both, no prose calls the segments zero a measured fact.
Construction: Read-only: in a build of the library with `--features meter,limb-meter,scan-meter` and without `cfg(test)` (what tests/meter.rs and examples/amp_board.rs link), no expression writes `SEGMENTS_GROWN`; the sole `fetch_add` is inside `#[cfg(test)] fn grow`. Runtime: add a temporary scenario to tests/meter.rs whose body recurses 10^6 frames through `stacker::grow` directly; `meter::stack_segments()` still reads 0 and every `segments: 0` envelope passes. Conversely, delete the body of `stack_segment_meter_counts_deterministically_and_resets` and run the meter suite and `just amp-board-acceptance`: every segments ceiling stays green, because nothing in those binaries could have moved the counter before the change either.

### crate-root-33: `RED_ZONE`'s derivation reasons about a release-profile measurement for a constant that compiles only under `cfg(test)`
- Where: crates/before/src/recurse.rs:43-49 (related: crates/before/src/recurse.rs:38-39, crates/before/src/recurse.rs:50-51, crates/before/src/recurse.rs:100, justfile:109, justfile:114, crates/before/src/meter/tests.rs:392-432, crates/before/src/clock/tests.rs deep_tree_stack_safety)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read; `STRIDE`, `RED_ZONE`, and `grow` are `#[cfg(test)]` at 38, 50, 100; the gate's test recipes run `cargo nextest run --workspace` without `--release`, so the committed exercises of the guard run under the dev profile the derivation does not describe); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (dfb247c36 set the constant from release frame sizes when the guard ran in production traversals; 05bd2b16d moved it under `cfg(test)`; b3f09baa0 re-wrapped the comment without re-denominating)
- Owner-gated: no

An approximation survives in prose only with its validity band stated; this band is stated for a profile the constant never runs under (dev-profile frames are considerably larger), so the "8x cushion" is not the cushion the code runs with. The operative proofs are the depth-200k dive and the depth-100k clock test, which the comment does not cite.

Evidence:

    43  /// Sized from a frame-size measurement (aarch64 release): the heaviest
    44  /// traversal frame is roughly 0.5 KiB/level — established by per-level
    45  /// stack-pointer deltas and cross-checked against each recursive function's
    46  /// prologue `sub sp`. With [`STRIDE`] = 64 the inter-probe burst is therefore
    47  /// well under 32 KiB, so 256 KiB leaves roughly an 8x cushion — ample headroom

Resolution: Restate the premise for the profile the guard runs in, or replace the derivation with the enforcement: "`STRIDE` × the largest test frame must stay under `RED_ZONE`; `meter::tests::stack_segment_meter_counts_deterministically_and_resets` (depth 200 000) and `clock::tests::deep_tree_stack_safety` are the committed proofs that the pair holds on every target the gate runs." Acceptance: the comment names the profile it reasons about and the committed tests that hold the constant to it.

### crate-root-34: serde impls serialize as `bytes` but deserialize by requesting a `seq`
- Where: crates/before/src/serde_impls.rs:20-31 (related: crates/before/src/serde_impls.rs:33-57, crates/before/src/serde_impls.rs:64-75, crates/before/src/serde_impls.rs:80-94, crates/before/src/serde_impls.rs:98-114, crates/before/src/serde_impls/tests.rs:23-105)
- Class / severity / confidence: correctness / medium / medium
- Provenance: verified (serde_core-1.0.229 src/de/impls.rs: `VecVisitor` at 1143 implements only `visit_seq` (1157) and `Vec<T>::deserialize` calls `deserializer.deserialize_seq(visitor)` (1175); ciborium-0.2.2 src/de/mod.rs:425-435 answers `deserialize_seq` on `Header::Bytes` by synthesizing a `BytesAccess` and calling `visit_seq`; serde_bytes-0.11.19 src/de.rs:132-174 accepts `visit_borrowed_bytes`, `visit_bytes`, `visit_byte_buf`, and `visit_seq`; `grep -c serde_bytes Cargo.lock` = 0. Which third-party formats fail is assessed, not run); executed: no
- Seen by: structure, correctness; refutation: confirmed, with one correction to the correctness lens (serde_bytes is not in the workspace's dependency graph, so it would be a new optional dependency); history: no-rationale-found (the `<Vec<u8>>::deserialize` shape is from a519aa88e with only the wire-form intent recorded; ada0db0e extended it and tested three formats' leniency without reconsidering the visitor)
- Owner-gated: no (a new optional dependency is a design choice; see the open question)

Every `Serialize` here calls `serialize_bytes`; every `Deserialize` goes through `<Vec<u8>>::deserialize`, whose visitor implements only `visit_seq`. In serde's data model `bytes` and `seq` are distinct types, and a Deserializer that answers `deserialize_seq` on a typed byte string by calling `visit_bytes` (which the model permits, and which serde's own `serde_test` does) rejects what `before` itself serialized with "invalid type: byte array, expected a sequence". The suite cannot see this: serde_json has no typed bytes, postcard frames `serialize_bytes` and `deserialize_seq` identically, and ciborium explicitly bridges a CBOR byte string into a seq. The module doc's contract ("the serialized form is exactly the wire form") should hold for every conforming format, not the three the tests drive. Prefer a dependency over hand-rolling: `serde_bytes::ByteBuf`'s visitor is strictly more accepting and makes both directions `bytes`.

Evidence:

    20  impl Serialize for Party {
    21      fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    22          s.serialize_bytes(&self.encode())
    23      }
    24  }
    25  
    26  impl<'de> Deserialize<'de> for Party {
    27      fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    28          let bytes = <Vec<u8>>::deserialize(d)?;
    29          Party::decode(&bytes[..]).map_err(D::Error::custom)
    30      }
    31  }

Resolution: Deserialize through `serde_bytes::ByteBuf::deserialize(d)?` (add `serde_bytes` as an optional dependency enabled by the `serde` feature) or a local ~20-line `Visitor` implementing `visit_bytes`, `visit_byte_buf`, and `visit_seq`, driven by `deserialize_bytes`. While touching all twelve impls, fold the six identical pairs into one macro taking a per-type doc attribute (`version.rs`'s `causal_cmp_impls!` is precedent) so the door has one body; the `Rank`/`Ranked`/`Span` impl docs carrying contract survive as doc arguments. Pairs naturally with crate-root-35 (the owned Vec the visitor yields can become the storage). Acceptance: a committed test drives a strict-typed deserializer, `serde_test::assert_tokens(&value, &[Token::Bytes(&value.encode())])` for each of the six types (dev-dependency `serde_test`), or a minimal local `Deserializer` whose `deserialize_seq` forwards a bytes payload to `visit_bytes`; the existing json/postcard/ciborium legs and the canonical-bytes pin stay green.
Construction: Add `serde_test` as a dev-dependency and write `serde_test::assert_tokens(&Party::seed(), &[serde_test::Token::Bytes(Party::seed().as_bytes())])`: the serialize leg passes (`serialize_bytes` emits `Token::Bytes`); the deserialize leg fails, because `serde_test`'s `deserialize_seq` forwards a non-seq token to `deserialize_any`, which calls `visit_bytes`, which `VecVisitor` does not implement.

### crate-root-35: serde impls copy every payload twice on both sides where the borsh impls copy once
- Where: crates/before/src/serde_impls.rs:20-31 (related: crates/before/src/serde_impls.rs:33-57, crates/before/src/version.rs:1027-1029, crates/before/src/version.rs:1110-1127, crates/before/src/borsh_impls.rs:152-181)
- Class / severity / confidence: performance / low / high
- Provenance: verified (version.rs:1027-1029 `pub fn encode(&self) -> Vec<u8> { self.as_bytes().to_vec() }`; version.rs:1110-1126 `decode` reads the whole reader into a fresh `Vec` and then adopts that buffer without copying; borsh_impls.rs:154 and :172 write `self.as_bytes()` directly and :164-166 and :179 adopt the read buffer); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (the zero-copy decode design is deliberate at e4de158ce and 1af119c1f, but the serde handoff of serde's `Vec` as a `&[u8]` reader predates it and was never revisited)
- Owner-gated: no

Strict deletion of redundant work has a fixed sign. `Serialize` calls `encode()`, which allocates and copies a buffer that `serialize_bytes` could take by borrow for `Party` and `Version`; `Deserialize` takes an owned `Vec<u8>` from serde and hands it to `decode(&bytes[..])`, whose `read_to_end` copies it into a second `Vec` before adopting that one as storage. The borsh door pays neither copy. (Clock, Rank, Ranked, and Span genuinely need a Vec on the serialize side; their deserialize sides do not.)

Evidence:

    22          s.serialize_bytes(&self.encode())
    ...
    28          let bytes = <Vec<u8>>::deserialize(d)?;
    29          Party::decode(&bytes[..]).map_err(D::Error::custom)

    version.rs:
  1027      pub fn encode(&self) -> Vec<u8> {
  1028          self.as_bytes().to_vec()
  1029      }
    ...
  1110      pub fn decode<R: Read>(mut reader: R) -> Result<Self, Decode> {
  1111          let mut buf = Vec::new();
  1112          reader.read_to_end(&mut buf).map_err(Decode::Io)?;

Resolution: Serialize `Party` and `Version` from `as_bytes()`. Add a crate-internal owned-bytes door per type (the tail of `decode` after `read_to_end`: validate the slice, then `from_frozen(Bits::from_canonical(buf.into()))`) and route every serde `deserialize` through it, so the Vec the visitor yields becomes the storage; Clock, Ranked, and Span can adopt sub-slices of the one buffer as `Clock::decode` already does. Acceptance: a heap-metered serde round-trip scenario in tests/meter.rs (serde is in `--all-features`) reads peak at most one payload copy plus the fixed allowance for Party and Version; the canonical-bytes pins stay byte-identical.

### crate-root-36: serde testdocs: "Both" paths lists three formats, "every rejection genre" covers three, "The new impls" is dated
- Where: crates/before/src/serde_impls/tests.rs:26-30 (related: crates/before/src/serde_impls/tests.rs:107-112, crates/before/src/serde_impls/tests.rs:153-154)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read; the rejection test's body at 114-151 drives trailing bytes, the rank mismatch, and the crossed pair only; `Truncated` and `Io` are not driven); executed: no
- Seen by: prose, structure; refutation: confirmed; history: deliberate-but-expired for "new" (true at ada0db0e); no history for the other two
- Owner-gated: no

A testdoc must be accurate to its body. "Both deserialization paths" is defensible as two visitor paths but reads as a miscount over the three formats it then enumerates; "every rejection genre the raw decodes mint" overreaches; "The new impls" is relative to a time, not the code.

Evidence:

    26      /// Both deserialization paths are driven: the self-describing number-array
    27      /// (`serde_json`), the non-self-describing length-prefixed bytes
    28      /// (`postcard`), and CBOR's *typed* byte string (`ciborium`, major type 2)
    ...
   153  /// The new impls compose inside a larger serde value: a `(Span, Rank, Ranked)`

Resolution: 26 "Both deserialization paths are driven across three formats: ..."; 110-111 "for the rejection genres the raw decodes produce that a wrapper could mask: ..." (or drive `Truncated` too); 153 "The impls compose inside a larger serde value". Acceptance: each testdoc's claim matches the set of cases its body drives; no testdoc uses "new".

### crate-root-37: Shape walks allocate an O(depth)-bit path stack past 64 levels; `Party::shape` says nothing allocates
- Where: crates/before/src/shape.rs:22-25 (related: crates/before/src/party.rs:480-481, crates/before/src/version.rs:738-740, crates/before/src/version/skyline/overlay.rs:317-324, crates/before/src/version/skyline/overlay.rs:471-478, crates/before/src/codec/stack.rs:48-56)
- Class / severity / confidence: claim / low / high
- Provenance: verified (overlay.rs:317-320 `LeafCursor` holds `path: BitStack` and 471-477 `IdLeafCursor` two `BitStack`s; codec/stack.rs:48-53 `push` spills `top` into `words: Vec<u64>` when `top_len == 64`, the first heap allocation of a `Vec::new()`, on the 65th live bit; `grep shape crates/before/tests/meter.rs` finds no shape-drain scenario; the depth-65 construction is assessed, not run); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (party.rs:480-481 and shape.rs:22-25 are from 46eb64f9; the path stacks and the spill predate the shape module; no commit prices the walk's auxiliary stack)
- Owner-gated: no

Space claims are hard guarantees per lib.rs:333-340, and the doctrine judges by the clause, not the likelihood of a 65-deep party. The walk's auxiliary space is a bounded `O(depth)` bits (within "a small constant multiple of the input"), so the contract holds; the false clause is "nothing allocates", and no heap-metered row pins any shape drain.

Evidence:

    22  //! Every walk borrows its value and streams in place: nothing is
    23  //! materialized up front, draining is linear in the value's encoded
    24  //! size, and an item allocates only when its rise magnitude exceeds two
    25  //! machine words.

    party.rs:
   480      /// Draining the iterator is linear in the party's encoded size: each
   481      /// region costs `O(1)`, and nothing allocates.

    codec/stack.rs:
    48      pub(crate) fn push(&mut self, bit: bool) {
    49          if self.top_len == 64 {
    50              self.words.push(self.top);

Resolution: State the walk's auxiliary space in the module doc (one path stack per input, `O(depth)` bits, heap-resident past 64 levels, freed at drop) and amend party.rs:481 to "allocates nothing per item" (likewise version.rs:738-740 and the clock door if they carry the clause). Add one heap-metered scenario per shape door to tests/meter.rs on the deep spine families so the auxiliary-space claim has a pin. Acceptance: under `PeakAlloc`, draining `deep_left_spine_party(64).shape()` reads zero heap delta and `deep_left_spine_party(65).shape()` a nonzero one; the prose states the `O(depth)`-bit path stack; a committed envelope row bands the deep-spine drains.
Construction: In a `PeakAlloc`-instrumented test: `let p = deep_left_spine_party(65); reset peak; p.shape().count(); assert_eq!(peak_delta, 0)` fails: `IdLeafCursor::open` pushes 65 path bits and `BitStack::push` allocates its first `words` entry at the 65th push.

### crate-root-38: A plateau is documented as a "maximal constant run" but the walk yields canonical leaves, which can be adjacent and equal
- Where: crates/before/src/shape.rs:83-94 (related: crates/before/src/shape.rs:11-15, crates/before/src/shape.rs:115-116, crates/before/src/version/skyline.rs:4-7, crates/before/src/version/skyline.rs:69-73, crates/before/src/version/skyline/shape.rs:33-36, crates/before/src/party.rs:473)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (skyline.rs:69-73 defines canonical form as forbidding only equal *sibling* leaves and calls a zero delta between non-sibling leaves "two plateaus of equal height separated by a subtree boundary"; skyline/shape.rs:33-36 returns `None` for a zero delta, which arises exactly between adjacent equal-height leaves; the constructed example below was checked by hand against the normalization rule, not run); executed: no
- Seen by: prose; refutation: confirmed (and the same conflation stands at skyline.rs:5, overlay.rs:99, and party.rs:473 outside this partition); history: no-rationale-found (46eb64f9 wrote "maximal constant run"; the owner's same-day 98e5b3b4 removed Region's copy of the definition but left Plateau's and the module bullets; the file contradicts itself at 93-94)
- Owner-gated: no

The module doc (11-15) and `Plateau`'s doc (83-84) define an item as one maximal constant run of the step function; `Plateau::rise`'s doc (93-94) then admits that two equal-height plateaus can be adjacent. Both cannot hold. The items are the leaves of the canonical coding, and canonical form forbids only equal sibling leaves, so a maximal constant run can span several items. A renderer that trusts the definition draws a boundary the function does not have; a consumer counting runs gets the wrong count. The same holds for `Region` (115-116) and `Party::shape`.

Evidence:

    83  /// A *plateau* is one maximal constant run of the version's step
    84  /// function. A shape walk yields plateaus left to right; see the
    ...
    93      /// left edge. `None` occurs mid-stream too: two equal-height
    94      /// plateaus separated by a subtree boundary are a real shape.

    version/skyline.rs:
    71  //!   what collapse removes. A zero delta between *non-sibling* consecutive
    72  //!   leaves is a real, canonical shape: two plateaus of equal height
    73  //!   separated by a subtree boundary.

Resolution: Define the item by the coding: "A *plateau* is one leaf of the version's canonical coding: a dyadic interval on which the step function is constant. Canonical form merges equal sibling leaves, so adjacent plateaus differ in height except across a subtree boundary, where `rise: None` marks a level step." Apply the same correction to the `Region` sentences (14-15, 115-116), the module bullets (11-12), and, outside this partition, skyline.rs:4-5, overlay.rs:99, and party.rs:473. Keep "equal iff plateau sequences equal" (it holds for the leaf sequence). Acceptance: no sentence in shape.rs calls an item a maximal constant run; a doctest parses `(0, (0, 0, 1), (0, 1, 0))` and asserts four plateaus with the third's `rise == None`.
Construction: `let v: Version = "(0, (0, 0, 1), (0, 1, 0))".parse().unwrap(); let p: Vec<_> = v.shape().collect();` The tree is canonical (no equal sibling leaves; no liftable minimum), the step function is 0, 1, 1, 0 over quarters (three maximal runs), and the walk yields four `Plateau { depth: 2 }` items, the third with `rise: None`. For parties, `((0, 1), (1, 0))` yields four regions with the middle two both `owned: true`.

### crate-root-39: Four shape iterators carry an identical `finished` flag and byte-identical `size_hint` bodies
- Where: crates/before/src/shape.rs:180-186 (related: crates/before/src/shape.rs:164-178, crates/before/src/shape.rs:215-237, crates/before/src/shape.rs:291-297, crates/before/src/shape.rs:363-369, crates/before/src/version/skyline/shape.rs:176-199)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified (the four `size_hint` bodies read; `Plateaus::next` and `Regions::next` repeat the done/advance step that `advance_refinement` encodes for a single walk, so routing them through it is behavior-preserving); executed: no
- Seen by: structure; refutation: confirmed (four small copies are also legible as-is); history: no-rationale-found (all four arrived together at 46eb64f9)
- Owner-gated: no

Duplication within one module; the cost is low because each copy is small and the four public types are distinct.

Evidence:

   180      fn size_hint(&self) -> (usize, Option<usize>) {
   181          if self.finished {
   182              (0, Some(0))
   183          } else {
   184              (1, None)
   185          }
   186      }

Resolution: A private `fn open_hint(finished: bool) -> (usize, Option<usize>)` called from all four, or route `Plateaus`/`Regions` through `advance_refinement(&mut [&mut walk])` so all four `next` bodies share one shape and the flag. Acceptance: `shape_walks_fuse` and `trivial_values_walk_whole` stay green; one `size_hint` body in the file.

### crate-root-40: The meter suite's header says the implementation is "far from" the contract the crate docs state as a hard guarantee
- Where: crates/before/tests/meter.rs:7-9 (related: crates/before/src/lib.rs:350-358, crates/before/src/meter/board/ceilings.rs:64-73)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (both texts read; ceilings.rs:64-69 judges every cell at exponent at most 1.15 and :71-73 at 16 heap bytes per input byte, the crate docs' position; which side is true is not settled here, since the board was not run); executed: no
- Seen by: claims (and flagged by correctness for the envelopes reviewer); refutation: confirmed; history: deliberate-but-expired (the header is from 4348636693, 2026-07-22, while the amplification campaign's V1-V6 were open; the campaign cured them (C2, P4.2, P5) and the dated-notes excision swept dates but left this sentence; the owner's bbb9f8023 then wrote the hard-guarantee paragraph)
- Owner-gated: no

Two present-tense statements of the crate's central promise cannot both be true. This file sits outside the listed partition files but is the counterpart of lib.rs:350-358, which is in it; per the history, the meter header is the stale side.

Evidence:

     7  //! on value magnitude, tree depth, or encoded size. Today's implementation
     8  //! is far from that — several operations amplify their input by large
     9  //! constants or worse — so every scenario here pins the *current* measured

    lib.rs:
   351  //! pathological input shapes. Any asymptotic claim is a hard guarantee that the
   352  //! operation will perform in time proportionate to that bound, for all input
   353  //! sizes, no matter how unlikely and contorted the shape of the input.

Resolution: Name the exceptions rather than pick a winner. Five operations the claims document demonstrates over their documented bounds (`Version::join`, skyline-coding-9; `Ranked::cmp`, rank-33; the masked comparison, skyline-sweep-place-masked-5; `Query::coverage`, span-causally-36; `tick` on the memo families, skyline-fill-grow-2) make the hard-guarantee sentence false as written and the header's "far from" true of them. Have `lib.rs`'s Asymptotic Complexity section name them as exceptions (or land the fixes), and restate the header as regression pins on measured constant factors under the documented bounds, with the same five named there or linked and the recursion-and-transcode narrative dropped. Acceptance: neither file contradicts the other; whichever file names the exceptions, the list is exactly the set of the claims document's demonstrated rows not yet fixed or declared, and the meter suite's module doc attributes every gap it names to a listed operation.
Construction: Textual; `just amp-board-acceptance` (the exponent and heap legs) is the committed instrument that settles which side is true: a red cell sides with the meter header.

Synthesis note (reconciliation): the body's "per the history, the meter header is the stale side" is the history pass's reading; the claims document's constructed rows overturn it for five operations (skyline-coding-9, rank-33, skyline-sweep-place-masked-5, span-causally-36, skyline-fill-grow-2), which are over their documented bounds today. The Resolution above is rewritten accordingly. The Construction line is kept as the finalizer wrote it; the board it names cannot settle the question, because its committed families are the shapes on which the claims hold, so a green board is consistent with the five breaches (claims document, crate-wide patterns).

## Positives

- borsh_impls.rs `ReaderCursor` (24-127): one byte pulled per demanded bit, the invariant `position <= 8 * bytes.len()` stated on the field that carries it, the `u64` position justified by the 512 MiB 32-bit seam, and the word-window fast path argued to be incapable of speculative reads ("the window's proven bits end at the buffer's end"); `finish` hands the buffered bytes over as storage without a copy. This is reviewably-correct performance code.
- borsh_impls/tests.rs: the per-bit `BitwiseReaderCursor` reference with byte consumption asserted on accepts and rejects alike (434-451 and siblings) over canonical, bit-flipped, truncated, noisy, and junk-tailed streams; the full ordered-pair composition matrix with re-serialization to each segment (1084-1174); the `Vec<Version>` element-N genre test; `coincident_span_keeps_borsh_container_framing` (1263-1299), whose doc names the exact failure every lone-value test would miss; and deterministic tripwires for the arms uniform sampling never reaches (negative-height join, collapsible join, tampered coincident padding). The strongest wire-door suite in the crate.
- One grammar body per tree: `validate_prefix` is `validate_from` over a slice cursor and `parse_id_core` is shared by the slice and reader doors, so the two doors can diverge only in cursor mechanics, which is exactly what the differential pins. The anonymous id is structurally unspellable on the wire (a zero presence bit, no bits of its own), so `i != 0` needs no runtime check at either door.
- fold.rs's module doc (1-16) states the goal beside the mechanism: why the balanced counter over a left fold, with the concrete lattice shapes (interleaved single-tick joins; one deep version among dominating meet operands) that make the left fold quadratic, and the combiner precondition (associative and commutative) that makes regrouping value-identical. Both `expect` messages are one-line proofs from the loop condition.
- error.rs draws the `Truncated`/`TrailingBits` boundary exactly at the flush-byte edge (69-85), the one place a reader would otherwise guess wrong, and the borsh suite holds both doors to that genre split (`flush_cut_*`). Every error type has a doctest that constructs it.
- shape.rs leads with which-of-these-do-I-want, explains why heights travel as rises (unbounded counts keep the walk linear in encoded size), reconstructs absolute heights in its example, handles and tests the `N = 0` refinement, and documents the iterators as fused but not exact-size. `advance_refinement` states the overlay-advance law once over `Refine` for any arity and both walk kinds.
- build.rs is a genuinely pure formatter: it recomputes nothing, checks each dataset's run parameters against the index (58-63), pairs each derived committed artifact with a freshness assert whose failure message names the regenerating recipe, and gates the one source-tree write behind an explicit env opt-in with the rationale at the check.
- Cargo.toml's feature comments (56-95) say what each counter sees that no other meter can; the `unexpected_cfgs = deny` roster closes the drift path for the alloc A/B cfg while its comment names the one hole it cannot see; `required-features` on `amp_board` makes an unfeatured board a cargo error rather than a green-looking judgment.
- recurse.rs's `descend!` design note (22-28): guarding the descent rather than the body, with the frame-cost argument for why, is a clear maintainer-facing explanation of a non-obvious choice.

## Open questions for Finch

1. Segments column (crate-root-32): dissolve, or keep and make live? Library kernels cannot reach `grow` in any build, so the column can only ever read zero for them; the depth-100k clock test and the `cfg(test)` gate are the proofs that hold. My recommendation is (a), dissolution, with `deep_tree_stack_safety` named as the instrument against reintroduced recursion; (b) costs an optional `stacker` dependency under `meter` and a liveness witness per enforcing binary.
2. The 100× figure (crate-root-24): is it backed by an off-tree measurement (for instance the oracle tree's boxed footprint against the packed bytes)? If so, commit that measurement beside results/space_consumption and cite it; if not, drop the number. Recommendation: extend `examples/space_consumption.rs` to emit the oracle size and state the observed ratio with its scenario.
3. The space figures (crate-root-29): were the 100-party / 10^6-event numbers produced by a run of `examples/space_consumption.rs` at non-paper parameters? If so, committing that CSV makes the paragraph checkable without rewording; otherwise re-denominate to the committed run. Recommendation: re-denominate; the paper-parameter artifact is what the figure below the paragraph draws.
4. The headline (crate-root-25): is the intended claim "linear for the core operations"? Recommendation: say exactly that and leave the exact bound to each `# Complexity` section.
5. serde (crate-root-34): a new optional `serde_bytes` dependency under the `serde` feature, or a local two-way visitor to keep the dependency surface fixed? Recommendation: `serde_bytes` (mature, tiny, and exactly this problem); either way, `serde_test` as a dev-dependency gives the strict-format witness without inventing a Deserializer.
6. auto_traits totality (crate-root-7): should surfacecheck own the totality check (it already walks the rustdoc JSON) with auto_traits.rs kept as the compile-time early warning, or should it assert the synthetic auto-trait impls directly and retire the roster? Recommendation: the first; the compile-time pin is the cheaper signal and the census makes its totality mechanical.
7. Plateau vocabulary (crate-root-38): is "plateau" intended to mean one leaf of the canonical coding (what the code yields), or a maximal constant run (what a renderer might expect)? The code cannot yield the latter without merging across subtree boundaries. Recommendation: define it as the leaf and say so once; a merging adapter is a consumer's one-liner.
8. Packaging (raised by the refutation pass, not a finding): crates/before/Cargo.toml has no `include`/`exclude`, so `cargo package` ships results/ (about 2.0M including the PNG and CSV nothing in the package reads), reference/ (the paper transcription, about 492K), fuelscape/ (about 1.5M), and docs/ (about 340K). build.rs needs fuelscape/, docs/, and results/space_consumption/itc_space_consumption.svg inside the package, so any include list must keep those; the license status of shipping the transcription under MPL-2.0 is the owner's call. Recommendation: an explicit `include` list before the first release.
9. Meter header (crate-root-40): is any operation today still over its documented bound? The board's green exponent gate says none; a ruling settles which prose is stale and whether the meter suite's header should be rewritten as regression pins under guaranteed bounds.

## Dropped

- [24], [45], [57] Segments counter (prose, correctness, claims lenses): duplicates of crate-root-32; their extra evidence (all 46 envelope rows at 0; tests/meter.rs:441; judge.rs's floor-trip scaffold) is folded in.
- [25], [49], [66] build.rs rerun triggers: duplicates of crate-root-5.
- [47] serde bytes/seq (correctness lens): duplicate of crate-root-34; its claim that serde_bytes is already a transitive dependency was refuted (Cargo.lock has no serde_bytes).
- [26], [46], [64] auto_traits totality: duplicates of crate-root-7; the `causally::{Down, Up, Neutral}` half is dropped per the history pass's dispute (`Polarity: Send + Sync + 'static` and the pinned `Query<'static, Down/Up>` already hold them).
- [30], [68] Cargo description: duplicates of crate-root-2.
- [32] AGENTS.md ghosts: duplicate of crate-root-1.
- [53] Decode::Io source: duplicate of crate-root-15.
- [31], [52] borsh tests cite rumors: duplicates of crate-root-10.
- [43] borsh test hand counts and formatting: merged into crate-root-12.
- [36] borsh test likelihood argument: merged into crate-root-12.
- [19], [50] PartialEq/PartialOrd: duplicates of crate-root-28; [41] ('static and surface) merged into it.
- [34], [55], [70] iter.rs backtick: duplicates of crate-root-23.
- [33], [56] "mint": duplicates of crate-root-26; [18]'s "new impls" half merged into crate-root-36.
- [59] headline "asymptotically linear": duplicate of crate-root-25.
- [61] 100× figure: duplicate of crate-root-24.
- [51] "exactly one hole": duplicate of crate-root-27.
- [38] borsh_impls.rs prose nits: the stray parenthesis and "in-memory wire form" merged into crate-root-9; the thrice-repeated one-wire-form sentence dropped per the history pass's dispute (borsh_impls is a private module whose doc never renders, while each impl doc renders on the public type's page, so the per-impl sentences are the only rendered statement of the contract).
- [44] build.rs version literal: merged into crate-root-4.
- [48] balanced-fold hand-back as a correctness bug: reframed to crate-root-18 (documentation) per the refutation pass; the mechanism is a decided, law-pinned design (laws.rs:2406-2410), so only the public `# Errors` prose is at issue.
- [65]'s status: kept as crate-root-6 despite the history pass's already-known verdict, because the recorded item is an open TODO for the x-axis caption, not a rationale for the current state, and the summary and noscript strings are two more sites of the same gap.
- [12]-adjacent: no separate finding on `merged.take()` idiom; it is the same simplification as crate-root-19.
- Version::shape's likelihood argument (version.rs:745-748, "realistically reachable Versions"), raised as an open question by the prose lens: out of this partition; belongs to the version-core review.
- The `testing/shape_rows.rs` oracle enumerations recursing without `descend!`, raised by the correctness lens: out of this partition; belongs to the testing-oracles review.
