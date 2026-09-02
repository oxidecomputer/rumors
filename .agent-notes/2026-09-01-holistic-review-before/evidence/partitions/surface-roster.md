# Partition surface-roster: The public-surface roster, the surface-coverage suite, surfacecheck, and surface-scan

## Partition summary

This partition is the machinery that holds `before`'s public surface total against its differential coverage. `crates/before/src/surface.rs` is the roster: `METHOD_SURFACE` (one row per inherent `pub fn`, three leg dispositions each), `FAMILY_SURFACE` (rows by operator or trait family), and the typed `Exclusion` vocabulary whose variants each defend one exclusion argument once. `crates/before/src/testing/surface_coverage.rs` and its `tests.rs` enforce the roster in-tree: a rustfmt-shape line scan over the hand-named `SURFACE_SOURCES` list is held equal to `METHOD_SURFACE` both ways, every cited name is resolved against a `#[test]`-attribute scan plus the law and descriptor registries, exclusion payloads resolve the same way, same-named tests are rostered, and the fold seeds are pinned committed. `crates/before/surfacecheck` is the second, stronger extractor: a detached binary that walks nightly rustdoc JSON, holds every function-like item to `METHOD_SURFACE` or a module exception, and reconciles every trait impl and non-function item two ways against the pinned censuses in `census.rs`. `crates/surface-scan` is the crate-agnostic line scanner shared with `suanpan`; `tools/citecheck` resolves every citation against the runner's own `cargo nextest list` inventory in the gate.

The instrument is well built where it is built. Every reconciliation is two-way, so no jaw can pass by a counter going dark; `reconcile_with` is a pure function with a committed red demonstration per finding category; the rustdoc `format_version` gate runs before any schema-typed parse and names both numbers; the extractors panic rather than under-report on the shapes they recognize; and the `Exclusion` enum turns exclusions into defended families with resolvable payloads and an inhabitation census over its own vocabulary. Citation integrity is closed from more sides than most rosters bother with (bare names, payloads, duplicate names across files, binding kinds shadowing each other, and collection by the runner).

The dominant issues are seams between what the prose promises and what the code enforces, plus machinery that outlived the constraint that justified it. Three prose sites say the surface-totality gate fails until a `FAMILY_SURFACE` row is added; surfacecheck never reads `FAMILY_SURFACE`, and today the census already holds impls (`Default for Version`, the `Cow<Version>` conversions, `error::Overlap`/`TooWide`) that no family row dispositions. The `Party::tick` row cites three tests that never call `Party::tick` while the law that does is cited by nothing. Five writer-sink rows carry no resolvable name on any leg, and the only evidence behind `Span::encode_to` is a doctest whose endpoints coincide. The line-scan extractor's justification dissolved when surfacecheck landed, and the tree now carries two extractors, three `#[test]` scanners with two rule sets, an empty per-item exception list with full plumbing, and a dated-ruling convention already reported to the owner as an open design item. A handful of Principle 5 defects (a deleted API name in a doc example, a pointer to a note that does not exist, temporal phrasing at a declaration site) round it out.

Lines read: the eleven partition files in full (4,037 lines; the test files are `surface_coverage/tests.rs`, `surfacecheck/src/check/tests.rs`, and `surface-scan/src/tests.rs`), plus the neighbors every finding rests on (the justfile's pin, citecheck, gate-stream, and surface-totality sections; `.github/workflows/ci.yml`; `tools/citecheck`'s header and inventory filter; `laws.rs` around the tick law; `codec/tests.rs`, `span/wire.rs`, `rank.rs`, `ranked.rs`, `version.rs`, `serde_impls.rs`, `borsh_impls.rs` and its tests for the writer-sink doors; `auto_traits.rs`; `shape.rs`; `meter/registry.rs` and its tests for the dated-ruling precedent; `validation_index.rs`; `tests/doc_hidden.rs`; `tests/foreign_reexport.rs`; `tests/amp_board_smoke.rs`; `suanpan/src/claims/tests.rs`; `meter/board/coverage/tests.rs`; `causally.rs`). Programs run: a Python count over `census.rs` and `surface.rs` (437 impl rows, 373 distinct impl strings, 64 doubled, 17 `StructuralPartialEq`; 40 `Law`-plus-twin-exclusion rows) and `rustfmt --check` on scratch copies of `surface.rs`. No cargo, just, or test command was run; every construction below is by reading unless marked executed.

## Findings

### surface-roster-1: The CI instruments job installs a floating nightly while the recipe it runs invokes the dated pin, and its comment describes the coupling the pin exists to remove
- Where: .github/workflows/ci.yml:142-145 (related: .github/workflows/ci.yml:128-133, justfile:25-40, justfile:935-945, .github/workflows/ci.yml:64-67, .github/workflows/ci.yml:206-210)
- Class / severity / confidence: correctness / low / medium
- Provenance: verified (read ci.yml:118-190, justfile:20-42 and 935-945; `grep 2026-06-30 .github/workflows/ci.yml` returns nothing); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-but-expired (the comment was accurate under a floating `nightly_toolchain`; e7a4b7b0 dated the pin and touched the justfile, AGENTS.md, rust-toolchain.toml, and the bands, not ci.yml)
- Owner-gated: no

The `instruments` job installs `toolchain: nightly` and runs `just surface-totality`, whose `surface-json` prerequisite invokes `cargo +{{ nightly_toolchain }}` = `+nightly-2026-06-30`. The job comment says the job "tracks nightly, so a format bump upstream can turn the leg red on an untouched tree", which is the failure the justfile's dated pin (justfile:25-38) exists to prevent and which the recipe as written cannot exhibit. Whether the job is green depends on the runner's rustup auto-installing the dated toolchain on first use; either way the two committed descriptions of what CI runs disagree (Principle 4: the goal stands beside the mechanism, and the two must agree; measurements bind to their run).

Evidence:

       128	  # Toolchain coupling: surface-totality parses nightly rustdoc JSON
       129	  # through a `rustdoc-types` pin matched to the installed nightly's
       130	  # format_version. This job tracks nightly, so a format bump upstream can
       131	  # turn the leg red on an untouched tree — the checker refuses loudly,
    ...
       142	      - name: Install nightly toolchain (surface-totality rustdoc JSON)
       143	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
       144	        with:
       145	          toolchain: nightly

    justfile:
        40	nightly_toolchain := "nightly-2026-06-30"
    ...
       937	    cargo +{{ nightly_toolchain }} rustdoc -p before --lib --all-features --target-dir target/surface-json -- -Z unstable-options --output-format json

Resolution: install the dated toolchain in CI from the justfile's single pin (a step that writes `just --evaluate nightly_toolchain` to `GITHUB_OUTPUT`, consumed by the `toolchain:` input), and rewrite ci.yml:128-133 to say the job runs the pinned nightly. The `ci` and `coverage` jobs' `toolchain: nightly` steps (64-67, 206-210) feed `doctest`, `fuzz-build`, and the branch-coverage leg, all spelled `+{{ nightly_toolchain }}` too; same fix, outside this partition. Acceptance: ci.yml derives `nightly-2026-06-30` wherever a `+{{ nightly_toolchain }}` recipe runs; the comment matches the mechanism; bumping one side alone fails the job with surfacecheck's format_version message, not a rustup error.
Construction: on a machine with rustup auto-install disabled (`RUSTUP_AUTO_INSTALL=0`, rustup >= 1.28.1) and only the floating `nightly` installed, `just surface-totality` fails at `surface-json` with a toolchain-not-installed error before surfacecheck runs.

### surface-roster-2: The roster's module doc states the meter-feature fact three times, the surface-totality paragraph recurs at four sites, and two counts are hand-maintained
- Where: crates/before/src/surface.rs:1-22 (related: crates/before/src/testing/surface_coverage.rs:22-30, crates/before/src/testing/surface_coverage.rs:62-64, crates/before/surfacecheck/src/main.rs:5-22, crates/before/surfacecheck/src/check.rs:11-13, justfile:906-918)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read all sites; `Exclusion::FAMILIES` has seven entries at surface.rs:157-165; `ANCHORS` has two at check.rs:96); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (surface.rs:3-5 was prepended in ebe66966 beside the original 16-17; d60b7570 rewrote the totality story in parallel at four sites, which is the drift vector that produced surface-roster-7)
- Owner-gated: no

Lines 3-5, 11-15, and 16-17 each say the module is public under `meter` so instrument crates bind to the same roster. The surface-totality mechanism is described in full at surface.rs:1010-1015, surface_coverage.rs:22-30, main.rs:5-22, and justfile:906-918; two of the four copies carry the family-row overclaim (surface-roster-7). surface_coverage.rs:63 counts "seven families" by hand (restating `Exclusion::FAMILIES.len()`), and check.rs:12-13 says "two known-public items must be present by name" (restating `ANCHORS.len()`). Documentation altitude: every sentence competes with the contract the reader came for; Principle 5: no hand-maintained counts.

Evidence:

         3	//! Public under the `meter` feature (with the other instrument-facing data) so
         4	//! external instrument crates can bind their coverage tables to the same
         5	//! roster.
    ...
        16	//! Public under the `meter` feature (the instrument crates' feature) and never
        17	//! part of a production build.

    surface_coverage.rs:
        62	//! An excluded leg carries a variant of the typed [`crate::surface::Exclusion`]
        63	//! vocabulary — seven families, each variant's documentation defending its

    check.rs:
        12	//! that returns nothing (or the wrong tree) cannot pass, because two
        13	//! known-public items must be present by name; the censuses need no

Resolution: keep one meter sentence in surface.rs (the 11-15 sentence carries the argument), delete 3-5 and 16-17; let surface_coverage.rs own the enforcement story and main.rs own the gate's, with the other sites pointing by name; replace "seven families" with "the families in [`Exclusion::FAMILIES`]" and "two known-public items" with "the anchors". Acceptance: the meter sentence appears once in surface.rs; the totality mechanism is described in full at one site and referenced by name elsewhere; no numeral restates an enumerable list in the partition's docs.

### surface-roster-3: `Exclusion::FAMILIES` is a hand-maintained twin of the enum, and a variant missing from it escapes the inhabitation census
- Where: crates/before/src/surface.rs:152-178 (related: crates/before/src/testing/surface_coverage/tests.rs:250-268)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read surface.rs:152-178 and tests.rs:250-268: the census iterates `FAMILIES` only; `family()` is exhaustive over the enum; the enum is `pub`, so an unconstructed variant draws no dead-code warning); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (ebe66966 introduced both without weighing a derived list)
- Owner-gated: no (a `macro_rules!` that emits the enum and the list together is dependency-free; `strum` would be a new dependency and is the owner's call)

`every_exclusion_family_is_inhabited` iterates `FAMILIES`; `family()`'s exhaustive match forces an arm for a new variant, but nothing forces a `FAMILIES` entry, so a variant added to the enum and to `family()` but not to the list is exactly the dead category the census exists to catch, and it is never checked. The doc's "keeps the two in one diff" describes reviewer attention, not a check (Principle 5: no hand-maintained enumerations of facts the code can change; prefer a dependency over hand-rolling).

Evidence:

       153	    /// Every family name, for the inhabitation census (an empty family is
       154	    /// a dead category); [`family`](Exclusion::family)'s exhaustive match
       155	    /// beside this list keeps the two in one diff when the vocabulary
       156	    /// widens.
       157	    pub const FAMILIES: &'static [&'static str] = &[

Resolution: derive the list from the enum. Dependency-free: a small `macro_rules!` that takes the variant list once and emits the `enum`, `FAMILIES`, and `family()`. With a dependency (owner's call): `strum::VariantNames` for `FAMILIES` and `strum::IntoStaticStr` for `family()`, `strum` optional under `meter`. Acceptance: adding a variant to `Exclusion` without an inhabitant fails `every_exclusion_family_is_inhabited` (or fails to compile).
Construction: add `Probe { pins: &'static [&'static str] }` to `Exclusion`, add `Exclusion::Probe { .. } => "Probe"` to `family()`, leave `FAMILIES` unchanged, use the variant in no row; `cargo nextest run -p before every_exclusion_family_is_inhabited` stays green.

### surface-roster-4: The `Party::tick` row cites tests that never exercise `Party::tick`; the reduction law that does is cited by nothing
- Where: crates/before/src/surface.rs:340-345 (related: crates/before/src/laws.rs:2480-2488, crates/before/src/surface.rs:346-351, crates/before/src/testing/surface_coverage.rs:53-55)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn '\.tick(&mut\|Party::tick'` over crates/before/{src,tests,benches,examples,fuzz}: the only non-doctest call of `Party::tick` is laws.rs:2486 inside `party_tick_matches_version_tick`; `grep -rn party_tick_matches_version_tick crates/before tools` hits only its definition at laws.rs:2482); executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (the row's citations are Version::tick anchors from the roster's first commit; the law landed the next day and was never wired in, while the sibling `Party::ticks` row cites `party_ticks_matches_version_ticks` on all three legs)
- Owner-gated: no

All three legs are `Trans` citations to `version_tick_matches_the_oracle`, `event_dominates_local_and_advances`, and `replay_matches_across_references`, none of which calls `Party::tick`. The `Trans` contract (surface_coverage.rs:53-55: "the named test anchors the reduction") is unmet: the cited tests anchor `Version::tick`, not the `Party::tick -> Version::tick` reduction. The law that pins that reduction, `party_tick_matches_version_tick`, is cited by no roster row, so deleting it turns nothing red (Principle 6: the cheapest passing artifact; a `Party::tick` that ticks the wrong operand is caught only by a doctest and an uncited law).

Evidence:

       340	    SurfaceRow {
       341	        op: "Party::tick",
       342	        prod_tree: Leg::Trans("version_tick_matches_the_oracle"),
       343	        prod_fs: Leg::Trans("event_dominates_local_and_advances"),
       344	        tree_fs: Leg::Trans("replay_matches_across_references"),
       345	    },

    laws.rs:
      2480	    /// The two `tick` entry points agree: `version.tick(&party)` and
      2481	    /// `party.tick(&mut version)` produce the same advance.
      2482	    fn party_tick_matches_version_tick {

Resolution: cite `party_tick_matches_version_tick` on the three `Trans` legs of the `Party::tick` row, mirroring `Party::ticks`. Acceptance: `every_cited_binding_test_exists` resolves the new citation via `laws::registered_names()`; deleting the law from laws.rs turns the roster red.
Construction: replace the body of `Party::tick` (party.rs:180) with `let _ = version;`; `roster_is_total_over_the_public_fn_surface`, `every_cited_binding_test_exists`, and `exclusion_payload_citations_resolve` stay green, and only the doctests and the uncited law fail. Then delete `party_tick_matches_version_tick`: the roster suite is still green.

### surface-roster-5: Forty rows spell one exclusion twice; a `Copy` derive and a `law_row` helper state the disposition once per row
- Where: crates/before/src/surface.rs:929-940 (related: crates/before/src/surface.rs:26-27, crates/before/src/surface.rs:72-73, crates/before/src/surface.rs:210-217, crates/before/src/surface.rs:312-317, tools/citecheck:122-145)
- Class / severity / confidence: simplification / low / high
- Provenance: verified; executed: yes (a Python pass over surface.rs parsed 132 explicit `SurfaceRow` literals, 40 of which have `Leg::Law` on prod_tree and identical empty-pin exclusions on both fs legs; `grep -c 'pins: &\[\]'` = 143)
- Seen by: scaffolding; refutation: reframed (the count holds; the proposed acceptance was wrong because citecheck's spelling-totality guard classifies a `Leg::Law(` occurrence only as a quoted literal or a match-arm binding, so a helper body `Leg::Law(law)` would be reported and the call sites would leave its extraction); history: no-rationale-found (ebe66966 introduced helpers for its own shapes and left this one)
- Owner-gated: yes (`Copy` on `Leg` and `Exclusion` widens the meter-public API; the refactor needs a paired, deliberate extension of `tools/citecheck`'s `extract_legs`)

Forty rows (the `Span`/`OwnSpan` accessors and algebra, `Version::ranked`, the `Ranked` rows, most `FAMILY_SURFACE` span rows) share the shape "Law on prod_tree, the same empty-payload exclusion on prod_fs and tree_fs", each spelling `Leg::Excluded(Exclusion::X { pins: &[] })` twice. `Leg` and `Exclusion` hold only `&'static` data, so `Copy` is derivable, after which a `const fn law_row(op, law, fs: Exclusion)` states the fs disposition once, exactly as `codec_row`, `causally_row`, `span_row`, and `HANDBACK` already do for their shapes. Legibility: a reviewer scanning forty six-line blocks to confirm two lines are identical is doing work the type could do.

Evidence:

       929	    SurfaceRow {
       930	        op: "OwnSpan::lo",
       931	        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
       932	        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
       933	        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
       934	    },
       935	    SurfaceRow {
       936	        op: "OwnSpan::hi",
       937	        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
       938	        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
       939	        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
       940	    },

Resolution: `#[derive(Debug, Clone, Copy)]` on `Leg` and `Exclusion`; `const fn law_row(op: &'static str, law: &'static str, fs: Exclusion) -> SurfaceRow`; named consts for the recurring empty exclusions (`DEFINITIONAL`, `LINEARITY`, `NO_WIRE_FORMAT`); rewrite the forty rows as one-liners. In the same change, extend `tools/citecheck`'s `extract_legs` to classify the helper's `Leg::Law(law)` body as a binding and to extract the law literal from `law_row("op", "law", ..)` call sites, with a `--self-test` fixture for each. Acceptance: every surface_coverage test and `just citecheck` green, with citecheck's leg count unchanged (40 law citations still extracted); the file shrinks by roughly 120 lines.

### surface-roster-6: Five writer-sink rows carry no resolvable name on any leg, and the only evidence behind `Span::encode_to` cannot see endpoint order
- Where: crates/before/src/surface.rs:987-992 (related: crates/before/src/surface.rs:506-511, crates/before/src/surface.rs:745-750, crates/before/src/surface.rs:785-790, crates/before/src/surface.rs:797-802, crates/before/src/surface.rs:60, crates/before/src/surface.rs:74-85, crates/before/src/span/wire.rs:62-74, crates/before/src/codec/tests.rs:805-834, crates/before/src/borsh_impls.rs:262-266)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the five rows; `grep -rn 'encode_to(\|encode_rank_to('` over every tests.rs under crates/before/src and over crates/before/tests and benches hits only codec/tests.rs:821, 826, 831, the Party/Version/Clock cases of `encode_to_matches_encode`; read span/wire.rs:44-74: `encode` and `encode_to` have independent bodies and the doctest builds `Span::new(&v, &v)`; read borsh_impls.rs:262-266 (`BorshSerialize for Span` calls `encode_to`) and every `borsh::to_vec`/`serialize` call in borsh_impls/tests.rs, none of which serializes a `Span`; serde `Serialize for Span` uses `encode()` at serde_impls.rs:100); executed: no
- Seen by: structure-prose, instrument-correctness; refutation: confirmed at medium (the doors are small delegations and a swap fails at the consumer's strict decode, but the roster's contract is breached and no floor catches an all-empty row); history: deliberate-and-holds for the disposition (94880a47: "the writer-sink door (encode_rank_to) stays in the owner-ratified doctest-pinned exclusion family"; 8350c556 gave Party/Version/Clock `encode_to` a named pin and left the others), with the Span doctest's coinciding endpoints anticipated by no record
- Owner-gated: no (the ratified disposition stands; the fix strengthens its premise and adds a floor)

The rows for `Span::encode_to`, `Rank::encode_to`, `Ranked::encode_to`, `Ranked::encode_rank_to`, and `Version::encode_rank_to` carry `pins: &[]` on all three excluded legs. The `Exclusion` doc names "for each writer-sink door, its doctest pinning byte identity with the buffer door" as the pin, but a doctest has no citable name, so these five rows are mechanically indistinguishable from unbound rows and `exclusion_payload_citations_resolve` has nothing to resolve. Three of the five doors are `write_all(&self.encode())`-style delegations (ranked.rs:185-187, 234-236; version.rs:1091-1093); two have independent bodies: `Rank::encode_to` (rank.rs:421-423, `encode_parts` directly) and `Span::encode_to` (wire.rs:71-74, two `encode_to` writes against `encode`'s `encode()` plus `as_bytes()`). The `Span` doctest uses `Span::new(&v, &v)`, so swapping the two writes leaves it green, and no test, serde path, or borsh test round-trips a `Span` through `encode_to` with distinct endpoints. Principle 6 (the cheapest passing artifact) and surface.rs:60 ("never a bare opt-out"), whose enforcement clause (67-71) has no name to act on here.

Evidence:

       987	    SurfaceRow {
       988	        op: "Span::encode_to",
       989	        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       990	        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       991	        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       992	    },

    span/wire.rs:
        66	    /// let span = Span::new(&v, &v).unwrap();
    ...
        71	    pub fn encode_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        72	        self.lo.encode_to(writer)?;
        73	        self.hi.encode_to(writer)
        74	    }

Resolution: extend `encode_to_matches_encode` (codec/tests.rs:812) to `Rank`, `Ranked` (both doors), `Span` with distinct endpoints, and `Version::encode_rank_to`, and cite it from the five rows (through `encode_to_row`, or a sibling helper for the rank doors). Then add a roster-level floor in surface_coverage/tests.rs: every row carries at least one resolvable name across its legs and payloads (a citation, a `pins` element, a `license`, a `guard`, or a `bound_at`), so a zero-binding row reads red by construction. Acceptance: the construction below turns the extended proptest red; the new floor fails on the current roster (five rows) and passes once the citations are added.
Construction: edit crates/before/src/span/wire.rs:72-73 to `self.hi.encode_to(writer)?; self.lo.encode_to(writer)`. Run `just test-all` and the doctest leg: every roster and coverage test, the doctest at wire.rs:62-70 (lo == hi), and the serde and borsh suites stay green.

### surface-roster-7: Three prose sites say the surface-totality gate fails until a `FAMILY_SURFACE` row is added; surfacecheck never reads `FAMILY_SURFACE`, and impls already sit behind no row
- Where: crates/before/src/surface.rs:1010-1015 (related: crates/before/src/testing/surface_coverage.rs:25-30, crates/before/surfacecheck/src/main.rs:17-20, crates/before/surfacecheck/src/main.rs:100-103, crates/before/surfacecheck/src/check.rs:159-165, crates/before/surfacecheck/src/census.rs:4-10, crates/before/src/testing/surface_coverage/tests.rs:334-341, crates/before/surfacecheck/src/census.rs:251, crates/before/surfacecheck/src/census.rs:256, crates/before/surfacecheck/src/census.rs:262, crates/before/surfacecheck/src/census.rs:401-429)
- Class / severity / confidence: claim / high / high
- Provenance: verified (read every surfacecheck source: the only roster read is main.rs:100 `before::surface::METHOD_SURFACE`; `FAMILY_SURFACE` appears in surfacecheck only in prose at main.rs:20 and census.rs:9 and in the guidance string at check.rs:162; a case-sensitive grep of surface.rs for `Default`, `Cow`, `Overlap`, `TooWide` returns nothing while census.rs pins `Default for Version` (262), the two `Cow<Version>` conversions (251, 256), and full `error::Overlap`/`error::TooWide` rows (401-429); the "error verdict types" row at 1241 names only Decode / Parse / Crossed); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (d60b7570 replaced the accurate "Totality here is by review of this file" with the overclaim; its message describes only the pin mechanism; 9aa8c9ac states "Trait-impl methods stay FAMILY_SURFACE's review-governed domain"; tests.rs:334-336 still says totality "is by review of the file")
- Owner-gated: no for the prose correction; binding the two layers is a design change for the owner

The roster doc, the suite's module doc, and surfacecheck's module doc claim a new operator impl "fails that gate until its pin — and, for a new family, a family row here — is added". The gate reconciles trait impls against `census::TRAIT_IMPLS` only; nothing maps a census pin to a family row in either direction. A maintainer who adds an impl, sees the census go red, and pins the row is green everywhere with no family disposition recorded, and the census already holds `Default for Version` (a second spelling of `Version::new`), the `From<&Version>`/`From<Version> for Cow<Version>` conversions, and `error::Overlap`/`error::TooWide` behind no row. Principle 8 (verified vs told): a reader trusting the roster believes every operator family has a recorded leg disposition when only the impl's existence is pinned; Principle 2: a board nothing enforces is decoration. The severity follows the brief's rubric for a claim contradicted by the code; the minimum fix is three sentences.

Evidence:

      1010	/// Rows here carry the leg dispositions by family; the concrete
      1011	/// impl inventory behind them is held mechanically total by the
      1012	/// surface-totality gate (`crates/before/surfacecheck`), which pins every
      1013	/// reachable trait impl by name against nightly rustdoc JSON. A new
      1014	/// operator impl is a deliberate API event: it fails that gate until its
      1015	/// pin — and, for a new family, a family row here — is added.

    surface_coverage/tests.rs:
       334	/// The family roster's rows are unique by op description (totality over
       335	/// the operator/trait surface is by review of the file; this pins the
       336	/// table's internal hygiene).

    surfacecheck/src/main.rs:
       100	    let rostered: BTreeSet<&str> = before::surface::METHOD_SURFACE
       101	        .iter()
       102	        .map(|row| row.op)
       103	        .collect();

Resolution: minimum, now: restate surface.rs:1013-1015, surface_coverage.rs:28-30, and main.rs:17-20 so the pin is mechanical and the family row is review-maintained, matching tests.rs:334-336; add family rows (or extend existing ones) for `Default` on `Version`/`Rank`/`Ticks`, the `Cow<Version>` conversions, and `error::Overlap`/`TooWide`. Better, as a design round: give each `FAMILY_SURFACE` row a machine-checkable membership (an `impls: &'static [&'static str]` of census-row prefixes or exact rows) and reconcile in surfacecheck both ways, with the three non-impl rows ("unbounded depth", "meter instrumentation plumbing", "error verdict types") excepted by name; `TRAIT_IMPLS` then becomes derived data. Acceptance: no prose attributes the family-row obligation to the gate, and the orphans above carry a disposition; or, under the binding, a synthetic census pin with no covering family row reads red in check/tests.rs and the construction below reads red in `just surface-totality`.
Construction: add `impl core::ops::Neg for Version { type Output = Version; fn neg(self) -> Version { self } }` in src/version.rs and the line `"Version: impl core::ops::arith::Neg for Version",` to `TRAIT_IMPLS`, with no `FAMILY_SURFACE` row. `just surface-totality` and `just test-all` are both green.

### surface-roster-8: Long `op` literals keep rustfmt from formatting `FAMILY_SURFACE`, leaving hand-formatted one-line rows beside vertical ones
- Where: crates/before/src/surface.rs:1128-1129 (related: crates/before/src/surface.rs:1134-1135, crates/before/src/surface.rs:1140-1141, crates/before/src/surface.rs:1173, crates/before/src/surface.rs:1179, crates/before/src/surface.rs:1204, justfile:157)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified; executed: yes (`rustfmt 1.9.0-stable --check --edition 2021` on a scratch copy of the verbatim file exits 0; on a copy with every `op` literal over 60 characters shortened it reports 8 hunks, the first reflowing 1128-1129 into vertical `Leg::Excluded(Exclusion::RepresentationMechanics {\n    license: ..,\n})` blocks; 26 lines exceed 100 columns, 1179 is 253; no rustfmt.toml or `rustfmt::skip` exists)
- Seen by: structure-prose; refutation: confirmed (its own rustfmt run agrees); history: no-rationale-found (the one-liners are residue of the typed-vocabulary conversion in ebe66966)
- Owner-gated: no

String literals cannot be broken, so rustfmt leaves the enclosing expression as written whenever a row's `op` exceeds the width: rows 1128-1129, 1134-1135, 1140-1141, 1173, 1179, and 1204 are one-line struct literals up to 253 columns while every other row is vertical, and `cargo fmt --check` in the gate accepts the file because rustfmt declines to format the constant. A roster the owner reviews row by row should not need horizontal scrolling, and a constant rustfmt cannot touch is hand-formatted forever.

Evidence:

      1128	        prod_fs: Leg::Excluded(Exclusion::RepresentationMechanics { license: "byte_equality_matches_bit_equality" }),
      1129	        tree_fs: Leg::Excluded(Exclusion::RepresentationMechanics { license: "byte_equality_matches_bit_equality" }),

Resolution: keep `op` to the identity (the parentheticals such as "owned and borrowed — the containment join" move to a `//` comment above the row, or into a `note: &'static str` field if they must stay machine-readable, which is owner-gated as a public field under `meter`) so every line fits and rustfmt formats the constant. Acceptance: `awk 'length($0) > 100' crates/before/src/surface.rs` lists only the box-drawing section comments; `cargo fmt --check` passes with every row vertical.

### surface-roster-9: Two extractors of the same `pub fn` surface: the line-scan's justifying constraint dissolved when surfacecheck landed
- Where: crates/before/src/testing/surface_coverage.rs:164-167 (related: crates/before/src/testing/surface_coverage.rs:208-217, crates/before/src/testing/surface_coverage.rs:263-273, crates/before/src/testing/surface_coverage/tests.rs:70-94, crates/before/surfacecheck/src/main.rs:5-8, justfile:917-918, justfile:464, justfile:469, justfile:1000, crates/before/src/meter/board/coverage/tests.rs:18-28, crates/before/tests/doc_hidden.rs:4-8, crates/before/tests/foreign_reexport.rs:6-12)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (read both extractors and their consumers; `extract_public_fns` is called from the roster test and from `meter/board/coverage/tests.rs:21` only; justfile:464 and 469 put `test-all` and `surface-totality` in different gate streams, and `just ci` (1000) omits `surface-totality`; the two forks entries extract nothing: `grep -n 'pub fn'` on `src/party/forks.rs` and `src/clock/forks.rs` finds only `pub(crate) fn new`, and `git log -S'pub fn'` on both files is empty); executed: no
- Seen by: scaffolding; refutation: confirmed, with one residual (surface-roster-21); history: deliberate-but-expired (67956d1f 2026-07-26 landed the scan as the only extractor; 9aa8c9ac 2026-07-30 landed surfacecheck, whose justfile comment calls the scan "a hand-named source-file list" and itself "the other jaw of the pincer"; no commit or note states what the scan catches that rustdoc JSON does not)
- Owner-gated: yes (retiring an instrument)

`extract_public_fns` (a rustfmt-shape line scan over the seventeen-entry `SURFACE_SOURCES` list) and surfacecheck's rustdoc-JSON walk both reconcile `METHOD_SURFACE` two ways. The JSON walk catches strictly more (any file, feature-gated trees, trait-declared methods, consts, macros, every trait impl) and has run in the gate since 2026-07-30; the scan's remaining payload is feedback in `cargo nextest run -p before` on the stable toolchain, and everything else it carries is cascade: the file list, `type_overrides`, `module_prefix`, the rustfmt-shape panics (surface-roster-28, -29), two forks entries that have never extracted anything, the board coverage test's dependence on the extractor rather than the roster, and the "two jaws" prose at four sites. Principle 3: infrastructure that reimplements a capability a mature tool provides (the compiler's own account of the surface) and machinery that outlives the constraint that justified it. The doctrine's retirement bar is met: surfacecheck's `unrostered`/`orphaned` categories are the same two directions, demonstrated red in check/tests.rs:60-79.

Evidence:

       164	/// The public-API source files of record. A new public module with
       165	/// inherent methods must be added here (and the roster test's coverage
       166	/// note updated), which is itself a reviewed diff.
       167	pub(crate) const SURFACE_SOURCES: &[SourceSpec] = &[
    ...
       271	pub(crate) fn extract_public_fns() -> BTreeSet<String> {
       272	    ::surface_scan::extract_public_fns(&crate_root(), SURFACE_SOURCES)
       273	}

    justfile:
       917	# in-tree roster test scans a hand-named source-file list; this leg is
       918	# the other jaw of the pincer, with no file list to forget. The checker

Resolution: retire before's line-scan extractor: delete `SURFACE_SOURCES`, `extract_public_fns`, and `roster_is_total_over_the_public_fn_surface`; have `meter/board/coverage/tests.rs::public_surface` read `METHOD_SURFACE` op names (it is held equal to the extraction anyway); re-word the "two jaws" prose in tests/doc_hidden.rs, tests/foreign_reexport.rs, the surface_coverage.rs module doc (drop "# Tamper-evident totality"; totality is surfacecheck's), and the justfile to name one totality check. `surface-scan` stays for suanpan's claims roster. If the owner wants the stable-toolchain inner-loop check kept, the honest alternative is to keep the scan retitled as a convenience and delete the two forks entries. Acceptance: with a `pub fn` added to any public type and no roster row, `just surface-totality` reads red naming it; with a `METHOD_SURFACE` row removed it reads red as orphaned; `cargo nextest run -p before --all-features` is green with `SURFACE_SOURCES` gone; no prose under crates/before mentions a second extractor or a pincer.

### surface-roster-10: Three `#[test]`-attribute scanners with two rule sets, in a workspace that created `surface-scan` to have one
- Where: crates/before/src/testing/surface_coverage.rs:310-365 (related: crates/before/src/testing/surface_coverage.rs:159-162, crates/surface-scan/src/lib.rs:174-196, crates/before/tests/amp_board_smoke.rs:314-340, crates/suanpan/src/claims/tests.rs:283, tools/citecheck:332-350)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read all three scanners; before imports only `fn_name` and `SourceSpec` from surface-scan (surface_coverage.rs:162) and suanpan is `test_fns`'s only consumer; an awk over every crates/**/*.rs found no `#[test]` line followed by a comment line, so the divergence is latent); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (the comment at 159-162 is imprecise about what is shared rather than asserting the witness scan is); history: no-rationale-found (the scanners were born separately on 2026-07-28/29 and the extraction commit's claim that surface_coverage.rs became a thin wrapper holds only for `extract_public_fns`)
- Owner-gated: no (consolidation; dropping the in-tree witness scan in favor of citecheck alone would be the owner's call)

`declared_test_names_by_file` (here), `surface_scan::test_fns` (lib.rs:174-196), and `band_test_names` (amp_board_smoke.rs:314-340) each re-implement "the names of `#[test]`-attributed fns" with divergent rules: this one matches `#[test]` by prefix and tolerates `///` and `//` lines between the attribute and the `fn`; the other two require `t == "#[test]"` and disarm on any non-attribute line. Every fix to the predicate (surface-roster-11's `#[ignore]` handling included) must land twice or thrice, and the comment at 159-162 says the extractor "and its line discipline" are the shared scanners while the witness scan stays local. Principle 3: duplicated capability generating its own maintenance cascade.

Evidence:

       331	                    let trimmed = line.trim_start();
       332	                    if trimmed.starts_with("#[test]") {
       333	                        test_pending = true;
       334	                        continue;
       335	                    }
       336	                    // Other attributes, doc comments, and comments sit
       337	                    // between `#[test]` and its `fn` without detaching it.
       338	                    if trimmed.starts_with("#[")
       339	                        || trimmed.starts_with("///")
       340	                        || trimmed.starts_with("//")
       341	                        || trimmed.is_empty()

    surface-scan/src/lib.rs:
       179	        if t == "#[test]" {
    ...
       183	        if t.starts_with("#[") || t.is_empty() {

Resolution: make `surface_scan::test_fns` the one scanner, adopting this copy's richer rule (comments between attribute and `fn` keep the arming; `fn ` after a qualifier is found via `fn_name` with a word-boundary check) and adding a per-file entry point; rewrite `declared_test_names_by_file` as a directory walk calling it per file and `band_test_names` as `test_fns(source).into_iter().filter(..)`; move the before-side fixture behaviors (doc comment between; `proptest!` property) into surface-scan's tests; reword 159-162 to what is shared. Acceptance: `grep -rn 'starts_with("#\[test\]")\|== "#\[test\]"' crates tools` matches only crates/surface-scan/src/lib.rs; before's coverage tests, suanpan's claims tests, and amp_board_smoke's parity test pass unchanged.

### surface-roster-11: `#[ignore]`d tests satisfy the in-tree citation checks, including the `GridCap` guard that "must run"; citecheck closes the hole for before, not for suanpan
- Where: crates/before/src/testing/surface_coverage.rs:336-344 (related: crates/before/src/testing/surface_coverage.rs:292-298, crates/before/src/testing/surface_coverage/tests.rs:183-185, crates/before/src/testing/surface_coverage/tests.rs:224-231, crates/surface-scan/src/lib.rs:183, crates/surface-scan/src/tests.rs:85-88, tools/citecheck:332-344, justfile:292)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the scan: any `#[` line between `#[test]` and `fn` is skipped with `test_pending` still set; `grep -rn '#\[ignore' crates/before/src` finds two ignored tests, codec/tests.rs:1969 and testing/exhaustive/tests.rs:445, neither cited; tools/citecheck:342-343 skips `case.get("ignored")` and runs in the gate's workspace stream and in `just ci`; the citecheck recipe is `--root crates/before` only, and suanpan uses `test_fns`, which has the same rule and whose fixture asserts the ignored `attr_between` admitted); executed: no
- Seen by: adequacy, instrument-correctness; refutation: reframed (true for the in-tree suite and `test_fns`, false for the gate); history: already-known (ffb92968's citecheck records and closes the collection seam, ignored tests explicitly; the in-tree prose and the surface-scan fixture were not updated)
- Owner-gated: no

The module doc says "a citation is satisfiable only by an item the test runner actually executes" and the `GridCap` check's doc says "a premise guard must run, not merely resolve"; both are false for an `#[ignore]`d test, which the scan records like any other. For before the gate's citecheck leg catches it (the nextest inventory excludes ignored tests), so the defect is a doc overclaim in the in-tree suite plus a fixture in surface-scan that pins the admission as correct. For suanpan, whose witness pairs resolve through `test_fns` and which has no citecheck leg, the seam is unguarded (no ignored test exists there today). Principle 2: a guard that resolves but never runs is a dark counter.

Evidence:

       296	/// kernels, and test-support plumbing never enter the haystack, so a
       297	/// citation is satisfiable only by an item the test runner actually
       298	/// executes.
    ...
       338	                    if trimmed.starts_with("#[")
       339	                        || trimmed.starts_with("///")
       340	                        || trimmed.starts_with("//")
       341	                        || trimmed.is_empty()
       342	                    {
       343	                        continue;
       344	                    }

    surface-scan/src/tests.rs:
        85	         #[test]\n#[ignore]\nfn attr_between() {}\n\
    ...
        88	    let want: Vec<&str> = vec!["attr_between", "cfg_before", "plain"];

Resolution: in the (ideally single, per surface-roster-10) witness scanner, note `#[ignore` while armed and either drop the name or return ignored names separately; have the citation tests reject a cited ignored name; change the surface-scan fixture so `attr_between` is asserted ignored rather than admitted; reword surface_coverage.rs:296-298 and tests.rs:183-185 to what the in-tree scan attests (attribution) and name citecheck as the collection authority for before. Acceptance: the construction below reads red in `exclusion_payload_citations_resolve` (or a new `no_cited_test_is_ignored`); the surface-scan fixture change is red-then-green.
Construction: insert `#[ignore = "probe"]` between `#[test]` (semantic_oracle/tests.rs:550) and `fn grid_cap_is_never_reached()` (551). `cargo nextest run -p before` (the in-tree suite alone) stays green while the guard never runs; only `just citecheck` reads red.

### surface-roster-12: Two of the three negative probes in the citation-haystack test cannot fail, and its doc cites a scan that no longer exists
- Where: crates/before/src/testing/surface_coverage/tests.rs:130-149 (related: crates/surface-scan/src/lib.rs:137, crates/surface-scan/src/lib.rs:162, crates/before/src/testing/surface_coverage.rs:162, crates/before/src/testing/surface_coverage.rs:310-313)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -rn 'fn parse_impl_self_type\|fn fn_name' crates/before/src` returns nothing; both are declared at crates/surface-scan/src/lib.rs:137 and 162; the scan walks before's `src/` only, so no scanner rule could admit them); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed at low (the positive direction and one negative probe are live); history: deliberate-but-expired (both helpers lived in before's src/ at a6746a65 and moved to the shared crate in 97d8f2b1 the next day; "the old bare-name scan" was written in 4b136e2b, the commit that replaced that scan)
- Owner-gated: no

The negative direction asserts that `parse_impl_self_type` and `fn_name` are absent from the haystack as proof that helpers cannot satisfy citations; both live in crates/surface-scan and are declared nowhere under crates/before/src, so the scan could admit every `fn` in the tree and these two assertions would still pass. Only `declared_test_names` is a live negative probe. Instruments before cures: a check whose negative half cannot fail is decoration. The doc's "the old bare-name scan" is a ghost reference (listed under surface-roster-20).

Evidence:

       133	/// Two directions. Negative: named non-test helpers — declared `fn`s the
       134	/// old bare-name scan accepted — are absent from
       135	/// [`declared_test_names`], so a roster row whose cited differential test
    ...
       143	    for helper in ["declared_test_names", "parse_impl_self_type", "fn_name"] {

Resolution: probe helpers declared under `src/` that an attribute-blind scan would admit (`declared_test_names_by_file`, `crate_root`, `extract_public_fns` from this module, or a helper `fn` adjacent to a `proptest!` block) and rewrite the doc positively. Acceptance: every name in the negative loop is found by `grep -rn 'fn <name>' crates/before/src`; temporarily removing the `if test_pending` guard at surface_coverage.rs:345 makes every negative probe fail; restored, the test is green.

### surface-roster-13: Two test doc comments are narrower or broader than their bodies
- Where: crates/before/src/testing/surface_coverage/tests.rs:181-185 (related: crates/before/src/testing/surface_coverage/tests.rs:195-199, crates/before/surfacecheck/src/check/tests.rs:244-260)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read both tests; `exclusion_payload_citations_resolve` extends `resolvable` with `diff_ops::registered_names()` at 195-199; `render_names_the_findings` populates four of the ten `Findings` fields at 248-253); executed: no
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (18c56984 widened the body to descriptors and updated the sibling test's doc but not this one; d60b7570 added six `Findings` categories and extended the render test to two of them)
- Owner-gated: no

The payload test's doc says a pin must be a `#[test]` item or a registered law, but the body also admits registered descriptor names. `render_names_the_findings` says "The render names every non-empty category" and exercises four of ten. Every test's doc comment states its invariant in English and must be accurate; the gate's `testdoc` checks presence, review holds it to accuracy.

Evidence:

       181	/// cited only in a payload would rot silently. Every `pins` element and
       182	/// `license` must be a `#[test]`-attributed item under `src/` or a
       183	/// registered law name; a `GridCap` guard must be an executable test

    check/tests.rs:
       244	/// The render names every non-empty category, so a red run always says
       245	/// what to do next.

Resolution: add "or a registered descriptor name" to the first doc; populate all ten `Findings` fields with one name each in the render test and assert each appears, or narrow its doc to the categories exercised. Acceptance: each doc describes exactly what its body checks.

### surface-roster-14: `tripwires_are_labeled` tests the shape of prose nothing consumes
- Where: crates/before/src/testing/surface_coverage/tests.rs:343-353 (related: crates/before/src/testing/surface_coverage.rs:285, tools/citecheck:275-286)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (`cited_test_names` maps `(_, test)` at surface_coverage.rs:285 and citecheck's `extract_tripwires` captures only the second string, so the labels are read by no code); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (byte-identical since 67956d1f; no commit names a failure it caught)
- Owner-gated: no

The test asserts the `TRIPWIRES` labels are unique and non-empty; the labels are human-facing strings consumed by nothing, so its failure would signal nothing about coverage. Principle 3: an assert names a concrete failure the other instruments miss; "the list stays legible" is the list justifying its own formatting.

Evidence:

       343	/// Every tripwire leg label is nonempty and unique — the tripwire list
       344	/// stays a legible per-leg inventory, not a grab bag.
       345	#[test]
       346	fn tripwires_are_labeled() {

Resolution: delete the test; label legibility is review's job. Acceptance: test removed; `TRIPWIRES` still cited by `cited_test_names` and extracted by citecheck.

### surface-roster-15: `d1_seeds_stay_committed` carries an opaque prefix defined nowhere
- Where: crates/before/src/testing/surface_coverage/tests.rs:355-359 (related: crates/before/src/testing/surface_coverage.rs:77, crates/before/src/testing/surface_coverage.rs:115)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '\bd1_\|\bD1\b' crates/before/src` hits only the test name and its two prose mentions); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (the name arrived in 67956d1f with no definition of D1 in that diff or any note)
- Owner-gated: no

The test and its two mentions use `d1` for the prod↔tree leg; every other site names the leg in words, and `D1` is defined nowhere in the crate. Doctrine under Principle 5: no opaque roster IDs in code or prose; a tag that outlives its roster is self-invented jargon.

Evidence:

       355	/// The prod↔tree adequacy seeds stay committed: the two fold-mutation
       356	/// witnesses replay through the `join_all` differentials from these
       357	/// files on every run, and this pin makes stripping them a red diff.
       358	#[test]
       359	fn d1_seeds_stay_committed() {

Resolution: rename to `fold_seeds_stay_committed` and update surface_coverage.rs:77 and 115. Acceptance: `grep -rn 'd1_' crates/before/src` returns nothing; the coverage tests pass.

### surface-roster-16: The validation index promises every guarding instrument and omits surfacecheck, citecheck, and the two hidden-surface pins
- Where: crates/before/src/testing/validation_index.rs:1-32 (related: crates/before/surfacecheck/src/main.rs:1-28, tools/citecheck:1-34, crates/before/tests/doc_hidden.rs:1-11, crates/before/tests/foreign_reexport.rs:1-21)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'surfacecheck\|rustdoc JSON\|citecheck\|doc_hidden\|foreign_reexport\|totality'` on the 182-line index returns nothing; the index does cover external instruments, e.g. before-fuelscape at 132); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (the index predates all four instruments and was edited eight times afterwards without gaining entries)
- Owner-gated: no

The index describes itself as "every instrument that guards this crate, what failure class each one catches that the others cannot" and gives the roster one entry, but has no entry for surfacecheck (items in unlisted files, feature-gated trees, trait impls), citecheck (cited tests the runner never collects), tests/doc_hidden.rs (hidden items rustdoc JSON omits), or tests/foreign_reexport.rs (a dependency's surface published through `pub use`). A maintainer orienting cold would not learn that the roster's totality has a second, stronger enforcement or that citation liveness is judged outside the crate.

Evidence:

         1	//! The validation index: every instrument that guards this crate, what
         2	//! failure class each one catches that the others cannot, and where it
         3	//! lives.

Resolution: one entry each, stating what it alone catches as a constructible input. If the line scan is retired (surface-roster-9), restate the roster entry's "held equal, name for name, to the `pub fn` surface extracted from source" against rustdoc JSON. Acceptance: `grep -n 'surfacecheck\|citecheck\|doc_hidden\|foreign_reexport' crates/before/src/testing/validation_index.rs` returns one entry each.

### surface-roster-17: Exception rulings carry enforced dates at the declaration site, validated by one of two divergent validators, under a name defined nowhere
- Where: crates/before/surfacecheck/src/check.rs:33-34 (related: crates/before/surfacecheck/src/check.rs:7-8, crates/before/surfacecheck/src/check.rs:259-277, crates/before/surfacecheck/src/check/tests.rs:262-326, crates/before/src/meter/registry.rs:1075-1103, crates/before/src/meter/registry/tests.rs:207-225)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: verified (`grep -rn exemption crates/before --include='*.rs'` hits the phrase only in surfacecheck; the convention's referent exists: registry.rs:1079 and 1095 declare `decided`, 1103 defines `REGISTRY_RATIFIED = "2026-07-29"`, and registry/tests.rs:215 and 221 validate it with `len() == 10 && two dashes`, the shape check.rs's `misdashed` fixture (`"20-26-07xx"`) is written to defeat); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: reframed (a repo-wide owner convention in tension with Principle 5, not a local slip; the phrase is unanchored); history: already-known (d2a9d04e's message reports "the enforced dated-exception machinery (surfacecheck Exception.decided + its YYYY-MM-DD validation and tests, registry Coverage/Bands decided fields + REGISTRY_RATIFIED ...) requires field and test changes to dissolve — a design round"; no ruling recorded since)
- Owner-gated: yes (a repo-wide convention; any change should be uniform across surfacecheck and the meter registry)

Every `Exception` carries a `decided` date; `reconcile_with` validates its `YYYY-MM-DD` shape (positions, digits, month and day ranges) and a `reason.len() < 20` threshold, and two tests with four fixtures pin the validator. The meter registry keeps the same field through one named constant and validates it with a weaker check, so two validators of different strength guard one convention. The module doc attributes the discipline to "the registry exemption-list discipline", a phrase that appears in no other file. Principle 5 as written: dated rationale at a declaration site belongs in git; the `reason` is the positively stated model. This is the open item d2a9d04e reported, restated with the divergence b1403c59 later created.

Evidence:

         7	//! [`crate::census`]. Every exception is named, dated, and reasoned —
         8	//! the registry exemption-list discipline — and every exception and
    ...
        33	    /// The date of the ruling of record (YYYY-MM-DD).
        34	    pub decided: &'static str,

    registry/tests.rs:
       215	                decided.len() == 10 && decided.chars().filter(|&c| c == '-').count() == 2,

Resolution: owner's ruling, applied uniformly. Preferred: drop `decided` from `Exception` and from the registry's `EnvelopeOnly`/`Unbanded`, delete the `dated` closure and the `undated`/`misdashed`/`unmonthed` fixtures, and let `git log -L` carry the when. If dated rulings stay: record the convention once in the project instructions as a deliberate exception to Principle 5, host one `is_yyyy_mm_dd` in surface-scan used by both sites, and replace "the registry exemption-list discipline" with a pointer to that site. Either way, name the `20` (surface-roster-26). Acceptance: `grep -rn decided crates/before/surfacecheck crates/before/src/meter` is empty, or exactly one date validator exists and the convention is recorded where the phrase is defined.

### surface-roster-18: `ITEM_EXCEPTIONS` is an empty per-item exception list with a full category, render block, `--list` disposition, census count, and tests behind it
- Where: crates/before/surfacecheck/src/check.rs:37-40 (related: crates/before/surfacecheck/src/check.rs:114-116, crates/before/surfacecheck/src/check.rs:185-189, crates/before/surfacecheck/src/check.rs:228-238, crates/before/surfacecheck/src/check.rs:305-310, crates/before/surfacecheck/src/main.rs:125, crates/before/surfacecheck/src/main.rs:152-153, crates/before/surfacecheck/src/check/tests.rs:129-201)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: verified (read every site; the list is `&[]`); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: reframed (the "empty buffer for known failures" doctrine misapplies, since a reasoned exception is the sanctioned declared-model route; the applicable lens is Principle 3, uninhabited machinery, and the proposed roster-row substitute reaches only fns in `SURFACE_SOURCES` files); history: no-rationale-found (empty since 9aa8c9ac's first run, "zero item exceptions", and never justified)
- Owner-gated: yes (dissolving an instrument's category)

The per-item list has no inhabitant at this or any commit, yet it owns the `item_exceptions` parameter and `excepted_items` set in `reconcile_with`, the `dead_item_exceptions` category and its render block, the `excepted (item)` branch in `render_list`, a slot in the clean-sweep census line, and the item halves of four tests. Its doc says "none at this tip", a statement about the current moment rather than a rule (Principle 5). Principle 3: machinery earns its place by what it serves; this serves nothing yet. Module exceptions differ: each is an inhabited, positively stated model of a whole instrument tree, and stays.

Evidence:

        37	/// Per-item exceptions: none at this tip. An entry here is a deliberate,
        38	/// owner-reviewable ruling that one public function-like item stays off
        39	/// the roster.
        40	pub(crate) const ITEM_EXCEPTIONS: &[Exception] = &[];

Resolution: delete `ITEM_EXCEPTIONS` and its plumbing (the parameter, `excepted_items`, `Findings::dead_item_exceptions` and its render block, the `--list` branch, the census-line count, and the item halves of `exceptions_excuse_their_scope`, `dead_and_shadowing_exceptions_read_red`, `malformed_exceptions_read_red`, `committed_exceptions_are_well_formed`); re-adding the mechanism with its first inhabitant is the reviewed event. If kept, reword the doc without the temporal clause. Acceptance: `grep -rn ITEM_EXCEPTIONS crates/before/surfacecheck` is empty and `reconcile_with` has six parameters; or the doc states the rule with no "at this tip".

### surface-roster-19: Em-dashes in `//` comments at six sites
- Where: crates/before/surfacecheck/src/check.rs:259-261 (related: crates/before/src/testing/surface_coverage.rs:101-102, crates/before/surfacecheck/src/main.rs:69, crates/before/surfacecheck/src/extract.rs:195)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (an awk over `^\s*//[^/!]` lines in every partition file lists exactly these six); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (owner doctrine, unpoliced by any lint)
- Owner-gated: no

Owner doctrine under Code Organization: "Prefer colons (or semicolons) over em-dashes in log messages and comments alike"; rendered rustdoc is exempt, `//` comments are not.

Evidence:

       259	    // A decision date is a real `YYYY-MM-DD` shape — digits in the digit
       260	    // positions, dashes at positions 4 and 7, month 01-12, day 01-31 —
       261	    // never merely ten characters holding two dashes somewhere.

Resolution: rewrite with colons or semicolons at check.rs:259-260, main.rs:69, extract.rs:195, surface_coverage.rs:101-102. Acceptance: the awk finds no `—` in a `//` comment in the partition.

### surface-roster-20: Ghost and temporal references: a deleted API name in a doc example, a pointer to a note that does not exist, a scan that no longer exists, and "none at this tip"
- Where: crates/before/surfacecheck/src/extract.rs:7-8 (related: crates/before/src/testing/surface_coverage.rs:164-166, crates/before/src/testing/surface_coverage/tests.rs:133-134, crates/before/surfacecheck/src/check.rs:37, crates/before/src/causally/forms.rs:172)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (`grep -rn 'causally::Range\|struct Range\|enum Range' crates/before/src crates/before/surfacecheck/src` hits only extract.rs:8; db9dfa3e "before: replace causally::Range with the polar Query algebra" touched census.rs and auto_traits.rs and not extract.rs; `pub fn since` is a free function at causally/forms.rs:172 and the roster row is `causally::since`; `grep -rn 'coverage note' crates/before/src` hits only meter/board/coverage.rs:21, which concerns error-path dispositions, and tests.rs:70-94 carries no such note; `grep -rn 'bare-name scan'` hits only tests.rs:134); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: contradicts-hard-rule for the Range example (live when written, a ghost since db9dfa3e); the coverage-note pointer has pointed at nothing since 67956d1f; "old bare-name scan" was a ghost from birth (4b136e2b); "none at this tip" survived the dated-notes sweep because d2a9d04e did not touch check.rs
- Owner-gated: no

The root AGENTS.md hard rule: "Nothing in the codebase refers to code that no longer exists — no 'formerly', 'superseded', 'was removed', no deleted API names in any prose." `causally::Range::since` names a type the crate no longer has, in the sentence meant to teach the naming scheme. The other three sites are Principle 5 defects of the same family: `SURFACE_SOURCES`'s doc tells the maintainer to update "the roster test's coverage note", which does not exist; the haystack test's doc explains its probes by "the old bare-name scan"; `ITEM_EXCEPTIONS`'s doc dates its emptiness to the present. The severity follows the rubric for a hard-rule breach; each fix is a phrase.

Evidence:

         7	//! which is the roster's own naming (`Party::seed` for a root re-export,
         8	//! `causally::Range::since` inside a public module). The `paths` table's

    surface_coverage.rs:
       164	/// The public-API source files of record. A new public module with
       165	/// inherent methods must be added here (and the roster test's coverage
       166	/// note updated), which is itself a reviewed diff.

    surface_coverage/tests.rs:
       133	/// Two directions. Negative: named non-test helpers — declared `fn`s the
       134	/// old bare-name scan accepted — are absent from

    check.rs:
        37	/// Per-item exceptions: none at this tip. An entry here is a deliberate,

Resolution: extract.rs:8: `causally::since` for a public-module free function (and `causally::Query::contains` for a method on a type inside a public module). surface_coverage.rs:165-166: delete the parenthetical. tests.rs:133-134: "named non-test helpers declared under `src/` are absent from ...". check.rs:37: state the rule without the temporal clause, or dissolve with surface-roster-18. Acceptance: `grep -rn 'causally::Range' crates` is empty; `grep -rn 'coverage note\|bare-name scan' crates/before/src/testing` is empty; `grep -n 'at this tip' crates/before/surfacecheck` is empty.

### surface-roster-21: The two totality jaws contradict each other on a `#[doc(hidden)]` inherent `pub fn` in a scanned file
- Where: crates/before/surfacecheck/src/extract.rs:31-32 (related: crates/surface-scan/src/lib.rs:111-121, crates/before/surfacecheck/src/check.rs:296-300, crates/before/tests/doc_hidden.rs:16-21)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (the line scan at lib.rs:111 strips `    pub fn ` with no attribute check, so a hidden inherent method in a `SURFACE_SOURCES` file is extracted and `roster_is_total_over_the_public_fn_surface` demands a row; extract.rs:31-32 states hidden items are absent from the JSON, and check.rs:296-300 reports every rostered op absent from the extraction as orphaned, with no exception path for orphans; tests/doc_hidden.rs:21 pins today's only hidden items as the sealed `PartyLiteral` trait and its method, which the line scan never sees); executed: no
- Seen by: none (raised by the refutation pass); refutation: new; history: not examined
- Owner-gated: no

A `#[doc(hidden)] pub fn` added to an inherent impl in a listed file would put the tree in a state no edit can make green: the attribute-blind line scan demands a `METHOD_SURFACE` row, and surfacecheck then reports that row orphaned because hidden items never reach rustdoc JSON. Today the interaction is moot, but nothing documents the consequence (hidden inherent methods are forbidden by construction, which may be desirable), and a maintainer meeting it sees two contradictory red diffs. If the line scan is retired (surface-roster-9) the contradiction dissolves; if it is kept, the consequence belongs in prose at tests/doc_hidden.rs or surface_coverage.rs.

Evidence:

        31	//! `#[doc(hidden)]` items never appear in the JSON at all, so the ground
        32	//! truth here is the documented public surface.

    surface-scan/src/lib.rs:
       111	            if let Some(rest) = line.strip_prefix("    pub fn ") {

    check.rs:
       296	        orphaned: rostered
       297	            .iter()
       298	            .filter(|op| !extracted.contains(**op))

Resolution: retire the line scan (surface-roster-9), or state at tests/doc_hidden.rs (beside the roster) that a hidden inherent `pub fn` in a `SURFACE_SOURCES` file cannot satisfy both totality checks and is therefore not a shape the crate admits. Acceptance: the prose exists, or only one extractor remains.
Construction: add `#[doc(hidden)] pub fn probe(&self) {}` inside `impl Party` in src/party.rs. `roster_is_total_over_the_public_fn_surface` fails naming `Party::probe` as unrostered; add the row, and `just surface-totality` fails naming `Party::probe` as orphaned.

### surface-roster-22: The item-grammar catch-all is silent where the type grammar is fail-loud, main.rs overclaims "every public item", and the skip's rationale is inaccurate
- Where: crates/before/surfacecheck/src/extract.rs:174-178 (related: crates/before/surfacecheck/src/main.rs:1-12, crates/before/surfacecheck/src/main.rs:13-22, crates/before/surfacecheck/src/extract.rs:228-261, crates/before/surfacecheck/src/extract.rs:352, crates/before/surfacecheck/src/extract.rs:361, crates/before/src/shape.rs:95-137, justfile:917-918)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`walk_named`'s `_ => {}` swallows `StructField`, `Variant`, `TypeAlias`, and every other `ItemEnum` variant, while `render_type` and `render_args` panic on unhandled shapes; `walk_type` never looks at fields or variants; `grep -nE '^\s+pub [a-z_]+: ' crates/before/src/shape.rs` finds six public fields at 95, 97, 120, 122, 130, 137; the three `pub type` aliases in the tree are all under module-excepted `laws`/`meter`); executed: no
- Seen by: adequacy, instrument-correctness; refutation: reframed (the gate's stated categories at main.rs:13-22 exclude fields, variants, and aliases by design; the overclaim at main.rs:1-3 and 11-12 and the rationale at extract.rs:175 are the concrete defects; censusing shape is an owner-gated extension); history: deliberate-and-holds for the scope, with the premise "aliases of already-walked types" unenforced
- Owner-gated: no for the enumerate-and-panic and the prose; widening the census to fields and variants is the owner's call

main.rs opens with "every public item of `before` is rostered, pinned, or excepted" and "holds every public item to exactly one disposition", then enumerates three categories that exclude struct fields, enum variants, and type aliases. The skip comment says fields and variants are "reached through their types' rows", but a type's rows pin its trait impls and methods, not its shape: adding a `pub` field to `shape::Plateau` or a variant to `shape::Rise` is a breaking API change the gate does not see. The `_ => {}` arm also swallows any `ItemEnum` variant a future rustdoc adds, unlike the rest of the module's fail-loud posture; the justfile sells this jaw as having "no file list to forget", and a silent catch-all reintroduces a forgettable category.

Evidence:

       174	        // Not reachable surface in this crate's grammar: enum variants
       175	        // and struct fields (reached through their types' rows), type
       176	        // aliases (aliases of already-walked types), and the rest of
       177	        // the item grammar.
       178	        _ => {}

    main.rs:
         1	//! Surface totality against rustdoc JSON: every public item of `before`
         2	//! is rostered, pinned, or excepted, checked from the compiler's own
         3	//! account of the public surface.

Resolution: enumerate the deliberately ignored variants explicitly (`Variant`, `StructField`, `TypeAlias`, `Primitive`, `ExternCrate`, ...) and panic on the remainder, matching `render_type`; correct main.rs:1-3 and 11-12 to name the three categories the gate covers; replace the rationale at 175 with the true one (shape is outside the roster's operation vocabulary). Owner's call: record public fields as `Type::field` and variants as `Type::Variant` item rows pinned in `census::ITEMS`, so a shape change reads red. Acceptance: an unhandled `ItemEnum` variant panics naming it in a unit test over a synthetic `Crate`; main.rs claims what the gate covers; or, under the widening, adding `pub extra: u8` to `shape::Plateau` turns `just surface-totality` red until pinned.
Construction: add `pub extra: u8` to `pub struct Plateau` (shape.rs:88) and initialize it where `Plateau` is built. `just surface-totality` and `roster_is_total_over_the_public_fn_surface` stay green.

### surface-roster-23: Auto-trait impls are excluded from the census on the premise that `auto_traits.rs` covers every public type; it misses thirteen
- Where: crates/before/surfacecheck/src/extract.rs:267-270 (related: crates/before/surfacecheck/src/extract.rs:21-24, crates/before/surfacecheck/src/extract.rs:279-281, crates/before/src/auto_traits.rs:5-32, crates/before/surfacecheck/src/census.rs:60, crates/before/surfacecheck/src/census.rs:332-335, crates/before/surfacecheck/src/census.rs:349-352, crates/before/surfacecheck/src/census.rs:384-387, crates/before/surfacecheck/src/census.rs:420-429, crates/before/surfacecheck/src/census.rs:435-463)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read auto_traits.rs in full, 26 instantiations; the census owners it does not cover are `Limbs`, `error::TooWide`, `shape::{Cell, Cells, Overlay, Plateau, Plateaus, Region, Regions, Rise}`, and `causally::{Down, Up, Neutral}`, thirteen types); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the premise was true at d60b7570, which listed every then-public type; db9dfa3e added the polarity markers and pinned `Query<Down>`/`Query<Up>` rather than the markers; 46eb64f9 introduced the shape types, `Limbs`, and `TooWide` without touching auto_traits.rs)
- Owner-gated: no

`record_impl` drops `is_synthetic` impls because `auto_traits.rs` "asserts `Send + Sync + Unpin` on every public API type at compile time". The list is a hand-maintained enumeration of public types (the exact rot this partition's rustdoc walk exists to prevent), nothing holds it total, and it has already drifted by thirteen types. For those, no check pins `Send`/`Sync`/`Unpin` anywhere: a shape iterator that gains an `Rc`- or `Cell`-bearing field loses `Send` with no red. Principle 3: an exclusion earns its place by naming what covers the excluded space, and the named cover is partial; the polarity markers are uninhabited enums and trivially auto-trait, but the shape types, `Limbs`, and `TooWide` are not.

Evidence:

       267	/// Compiler-synthesized auto-trait impls are excluded — `before` pins
       268	/// `Send`/`Sync`/`Unpin` on every public API type at compile time in
       269	/// `src/auto_traits.rs`, which is where that guarantee is reviewed —

Resolution: either (a) stop excluding synthetic impls and pin the `Send`/`Sync`/`Unpin` rows in the census for every reachable type, dissolving the hand list (the JSON already carries them, so the pin becomes mechanical and total); or (b) keep auto_traits.rs, extend it with the thirteen missing types, and have surfacecheck hold it total by asserting every reachable struct and enum has a synthetic impl for each of the three traits. Acceptance: a public type losing `Send` makes `just surface-totality` (a) or `cargo check` (b) red.
Construction: add `_marker: core::marker::PhantomData<*const ()>` to `shape::Plateaus` and initialize it. The crate compiles, `Plateaus` is no longer `Send` or `Sync`, and `just gate` is green.

### surface-roster-24: Census rows key trait impls by private definition paths and generic parameter names, record cross-type impls under both owners, and pin the compiler-internal `StructuralPartialEq`
- Where: crates/before/surfacecheck/src/extract.rs:293-306 (related: crates/before/surfacecheck/src/extract.rs:38-43, crates/before/surfacecheck/src/extract.rs:279-281, crates/before/surfacecheck/src/census.rs:4-6, crates/before/surfacecheck/src/census.rs:37, crates/before/surfacecheck/src/census.rs:39, crates/before/surfacecheck/src/census.rs:43-45, crates/before/surfacecheck/src/census.rs:64, crates/before/surfacecheck/src/census.rs:164, crates/before/surfacecheck/src/census.rs:296, crates/before/surfacecheck/src/census.rs:332, crates/before/src/causally.rs:146, crates/before/src/causally.rs:156)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified; executed: yes (a Python pass over census.rs: 437 impl rows, 373 distinct `impl ... for ...` strings, 64 strings under two owners occupying 128 rows, 17 `StructuralPartialEq` rows; e.g. `impl core::ops::bit::BitOr<Clock> for Version` at rows 39 and 296, `impl core::convert::From<OwnSpan> for Span` at 64 and 164; `mod polarity;` is private at causally.rs:146 and re-exported at 156, yet six rows spell `before::causally::polarity::Polarity`)
- Seen by: scaffolding, structure-prose (plus the refutation pass's observation on extract.rs:38-43); refutation: confirmed (with the nuance that `StructuralPartialEq` distinguishes a derived from a hand-written `PartialEq`, which is user-observable in const patterns); history: deliberate-and-holds for one-row-per-spelling and definition-path rendering (both stated in the code and d60b7570's message); the private-path cost is weighed nowhere and `StructuralPartialEq` was never named
- Owner-gated: no (internal to surfacecheck; one re-pin of `TRAIT_IMPLS`, attributed in its commit)

`render_trait` spells the trait by its definition path from rustdoc's `paths` table, which embeds private module structure: before's own `causally::polarity` (public path `causally::Polarity`), libcore's internal layout (`core::ops::bit::`, `core::iter::traits::accum::`, `core::str::traits::`), and serde's `serde_core::` split. census.rs:4-6 promises pins move only when an impl is added, removed, or reshaped, but renaming `src/causally/polarity.rs`'s module or a dependency reorganizing internally orphans rows with zero API change, in one mechanical diff where a real impl change is hardest to see. Rows also carry generic parameter names (`Add<T>`, `Cell<N>`, `Query<P>`), so a parameter rename is a pin event. Separately, 64 impls are recorded under both participating types' pages; extract.rs:38-43 defends this as "two public spellings are two rows of surface", which describes a type reachable at two paths and does not cover the mechanism that produces these rows (rustdoc lists a cross-type impl on every type that appears in it). And 17 rows pin `core::marker::StructuralPartialEq`, a perma-unstable marker that tracks `derive(PartialEq)`; it carries a small signal (derived vs hand-written) at the cost of orphaning 17 rows together if a toolchain stops emitting it. Principle 3: the tamper-evidence value of a pin list degrades in proportion to how often it moves for non-API reasons.

Evidence:

       293	/// Render a trait reference for a census row: the definition path from
       294	/// the `paths` table (unambiguous across same-named traits), plus any
       295	/// generic arguments.
       296	fn render_trait(krate: &Crate, trait_: &Path) -> String {
       297	    let mut out = krate
       298	        .paths
       299	        .get(&trait_.id)
       300	        .map(|summary| summary.path.join("::"))
       301	        .unwrap_or_else(|| trait_.path.clone());

    census.rs:
       332	    "causally::Down: impl before::causally::polarity::Polarity for Down",

Resolution: render the trait as `<crate>::<TraitName><Args>` (first and last `paths` segments), which still disambiguates same-named traits across crates; render generic parameters positionally (`_`); dedupe in `record_impl` by impl `Id` so a cross-type impl is one row (disambiguating same-named for-types only on collision), and rewrite extract.rs:38-43 to name the actual mechanism if double rows are kept; decide `StructuralPartialEq` explicitly (skip beside `is_synthetic` with a one-line reason, or keep with the derived-vs-manual rationale stated). One re-pin of `TRAIT_IMPLS`, attributed to the renderer change. Acceptance: a unit test renders a synthetic `Path` whose `paths` entry is `before::causally::polarity::Polarity` as `before::Polarity`; after re-pinning, the gate is green and renaming the private `polarity` module moves no pin.

### surface-roster-25: Metaphors promoted to jargon without an anchor: "jaw"/"pincer", "honest reading", "earns", "the seal", "keystone", "a real roster row"
- Where: crates/before/surfacecheck/src/main.rs:19-20 (related: crates/before/surfacecheck/src/extract.rs:42, crates/before/surfacecheck/src/check.rs:31, crates/before/src/testing/surface_coverage/tests.rs:138, crates/before/src/testing/surface_coverage.rs:47-48, crates/before/src/testing/surface_coverage.rs:67, justfile:918, crates/before/tests/doc_hidden.rs:2, crates/before/tests/foreign_reexport.rs:3)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for each term across the partition and the two hidden-surface pins; none is anchored to an identifier or defined by contrast); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed (the TRIPWIRES two-genre widening is defined at its site and is dropped here as taste); history: no-rationale-found (each term first appears in a prose rewrite with no definition)
- Owner-gated: no

The vocabulary rule: a coined term is anchored to an identifier or defined once by contrast; a metaphor that exists only as texture fails. "Pincer" in particular names the two-extractor design surface-roster-9 recommends dissolving; "honest reading" moralizes a design choice; "earns" is an economy register where none exists; "a real roster row" contrasts with nothing.

Evidence:

        19	//!   new impl and a vanished pin both read red — the mechanical jaw
        20	//!   behind `FAMILY_SURFACE`'s per-family dispositions;

    extract.rs:
        42	/// path as an unrostered finding to triage, which is the honest reading

Resolution: rewrite as mechanism: "the census that pins each impl `FAMILY_SURFACE` disposes"; "two public spellings are two rows of surface" (drop "honest"); "Why this surface has no roster row"; "so the check cannot pass by scanning nothing"; "the replay differential (`replay_matches_across_references`)"; "a roster row, test, or law"; and, when surface-roster-9 lands, no "jaws" or "pincer" anywhere. Acceptance: none of the terms appears in the partition's prose without a definition.

### surface-roster-26: Idiom nits: a mutation inside `Option::inspect`, leading `::` on an external crate path, a fully qualified `Range`, `push_str(&format!(..))`, an order-sensitive roster comparison, and an unnamed `20` at four sites
- Where: crates/before/surfacecheck/src/main.rs:44-46 (related: crates/before/surfacecheck/src/main.rs:104, crates/before/src/testing/surface_coverage.rs:162, crates/before/src/testing/surface_coverage.rs:272, crates/before/surfacecheck/src/check.rs:264, crates/before/surfacecheck/src/check.rs:277, crates/before/surfacecheck/src/check.rs:136-145, crates/before/src/testing/surface_coverage/tests.rs:283-302, crates/before/src/meter/board/coverage/tests.rs:69, crates/suanpan/src/claims/tests.rs:300, crates/suanpan/src/claims/tests.rs:385)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read each site; `grep -rn 'len() < 20\|len() >= 20' crates` lists exactly the four threshold sites; crates/before is edition 2021 with no local `surface_scan` module); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (the leading `::` plausibly expired when the same-named sibling module was deleted in 0a5bdaeb)
- Owner-gated: no

`--list` is detected with `position(..).inspect(|&i| { args.remove(i); })`, a mutation hidden in an inspection combinator whose result is then tested with `is_some()`; `if let Some(i) = .. { args.remove(i); }` says what happens. `::surface_scan::` uses the leading-`::` disambiguator where no local module of that name exists. `std::ops::Range<usize>` is spelled by path in a closure signature. `Findings::render` builds with `push_str(&format!(..))` where `writeln!` via `std::fmt::Write` reads directly. `duplicate_test_names_are_rostered` compares two `Vec<(String, Vec<String>)>`, so a correct roster listed out of sorted order fails with a full-dump diff rather than a named discrepancy. The reason-length floor `20` is an unnamed magic number at check.rs:277, meter/board/coverage/tests.rs:69, and suanpan/src/claims/tests.rs:300 and 385. Legibility: finished code should be obviously correct at a glance; named constants over magic numbers; imports over long qualified paths.

Evidence:

        44	    let list = args.iter().position(|a| a == "--list").inspect(|&i| {
        45	        args.remove(i);
        46	    });

    check.rs:
       277	        .filter(|e| !dated(e) || e.reason.len() < 20)

Resolution: `let list = match args.iter().position(|a| a == "--list") { Some(i) => { args.remove(i); true } None => false };`; drop the leading `::`; `use std::ops::Range`; `use std::fmt::Write; writeln!(out, ..)`; compare the duplicate roster as `BTreeMap<String, BTreeSet<String>>` and report the symmetric difference by name; `pub const MIN_REASON_LEN: usize = 20;` in surface-scan, used from all four sites. Acceptance: each site reads as described; clippy clean; `20` appears once, named.

### surface-roster-27: The clean-sweep census line mixes item counts with exception-entry counts, so its arithmetic identity does not hold
- Where: crates/before/surfacecheck/src/main.rs:118-133 (related: crates/before/surfacecheck/src/main.rs:109-117)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read main.rs:109-133: "excepted" is `surface.functions.len() - rostered.len()`, a count of items, while the parenthetical reports `ITEM_EXCEPTIONS.len()` and `MODULE_EXCEPTIONS.len()`, counts of exception entries); executed: no
- Seen by: none (raised by the refutation pass); refutation: new; history: not examined
- Owner-gated: no

The line reads as "N = R rostered + E excepted (a item exceptions, b module-scope)", which a reader takes as `E = a + b`; with four module entries covering roughly a hundred items, it does not. The `outside` closure at 109-117 already computes the item-count form for impls and items.

Evidence:

       119	            "surface totality: {} public function-like items = {} rostered + {} \
       120	             excepted ({} item exceptions, {} module-scope); {} trait impls = {} \
       121	             pinned + {} module-excepted; {} items = {} pinned + {} module-excepted",
       122	            surface.functions.len(),
       123	            rostered.len(),
       124	            surface.functions.len() - rostered.len(),
       125	            check::ITEM_EXCEPTIONS.len(),
       126	            check::MODULE_EXCEPTIONS.len(),

Resolution: report the parenthetical as item counts (items under item exceptions, items under module exceptions, via the `outside` closure) so the identity holds, or label them as entry counts ("under 4 module exceptions"). Acceptance: the printed numbers satisfy the identity the line's shape implies.

### surface-roster-28: The shared extractor silently drops `pub const fn` and any `pub fn` at an unexpected indent, contradicting its stated never-under-report contract
- Where: crates/surface-scan/src/lib.rs:111-129 (related: crates/surface-scan/src/lib.rs:19-25, crates/surface-scan/src/lib.rs:69-72, crates/surface-scan/src/lib.rs:104-110, crates/before/src/testing/surface_coverage.rs:267-270, crates/suanpan/src/claims/tests.rs:172-195, crates/surface-scan/src/tests.rs:56-75)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read lib.rs:73-133: exactly two positive shapes, `    pub fn ` and `pub fn `, and no negative catch-all; the nested-impl panic at 104-110 fires only inside a `pub mod`; `grep -nE '^\s*pub (const|async|unsafe|extern) fn|^(  |      |        +)pub fn '` over the seventeen `SURFACE_SOURCES` files returns nothing, so the hole is latent for before; suanpan's only totality check is `claims_are_total_over_the_public_surface` over this extractor and it has no rustdoc-JSON leg); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: no-rationale-found (the qualifier and nested-indent cases were never considered when the contract was written)
- Owner-gated: no (a catch-all panic; replacing the line scan with `syn` parsing would be a design decision)

The module doc's one invariant, "A `pub fn` at an unexpected position panics rather than silently vanishing — the extractor must never under-report the surface it exists to pin", is false for constructible shapes: `    pub const fn`, `    pub async fn`, `    pub unsafe fn`, and any `pub fn` at an indent other than 0 or 4 (an inherent impl inside a private `mod` block, still public API on a public type) match neither positive arm and produce no row and no panic. Making a constructor `const` is a routine change. before is rescued by surfacecheck; suanpan's claims roster has no second jaw. Correct for all inputs: an instrument that pins the public surface must not have silent-drop shapes, and its doc must not claim a discipline it lacks.

Evidence:

        23	//! methods and module-block functions at one indent. A `pub fn` at an
        24	//! unexpected position panics rather than silently vanishing — the
        25	//! extractor must never under-report the surface it exists to pin.
    ...
       111	            if let Some(rest) = line.strip_prefix("    pub fn ") {
    ...
       123	            if let Some(rest) = line.strip_prefix("pub fn ") {

Resolution: after the two positive arms, a negative catch-all: any line whose trimmed form starts with `pub` and contains ` fn ` that was not classified panics naming file and line ("beyond the line discipline"). Accept `pub const fn` positively at both indents (strip an optional `const ` after `pub `), since it is public surface with the same naming. Fixtures in surface-scan/src/tests.rs: `pub const fn` in an inherent impl is named; an 8-indent `pub fn` inside `mod x { impl T { .. } }` panics. Acceptance: the new fixtures are red on the current extractor and green after; before's and suanpan's totality tests stay green on the tree.
Construction: fixture `impl Thing {\n    pub const fn zero() -> u8 {\n        0\n    }\n}` with `spec(None)`: `extract_public_fns` returns an empty set and does not panic. In suanpan, adding `pub const fn probe() -> u8 { 0 }` inside `impl Accumulator` leaves `claims_are_total_over_the_public_surface` green with no `Accumulator::probe` claim row.

### surface-roster-29: `parse_impl_self_type` treats the `>` of a `->` return arrow as a closing angle bracket; a header in the scanned tree already trips it
- Where: crates/surface-scan/src/lib.rs:139-153 (related: crates/surface-scan/src/lib.rs:135-137, crates/before/src/version/rank.rs:693-703)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (`grep -nE '^impl.*->'` over the `SURFACE_SOURCES` files hits exactly rank.rs:693 `impl<F: FnMut() -> Result<u8, Decode>> BitSource<F> {`; hand trace of 139-153: `<` sets depth 1, the `>` of `->` sets depth 0 and breaks, and the remainder ` Result<u8, Decode>> BitSource<F> {` yields `Result`, so `current_type` is `Result`; the block holds only the private `fn bit`, so nothing is misnamed today); executed: no
- Seen by: instrument-correctness; refutation: confirmed by trace; history: no-rationale-found (the parser predates the header that trips it)
- Owner-gated: no

The function documents "skip a balanced generics list, then read the first identifier" and mis-skips a header the tree already contains. A `pub fn` added to that block would be extracted as `Result::..`, failing the roster loudly but under a phantom type name. Correct for all inputs: the failure is loud, which keeps the severity low, but the mismatch message would misdirect the maintainer.

Evidence:

       139	    if chars.peek() == Some(&'<') {
       140	        let mut depth = 0usize;
       141	        for c in chars.by_ref() {
       142	            match c {
       143	                '<' => depth += 1,
       144	                '>' => {
       145	                    depth -= 1;
       146	                    if depth == 0 {
       147	                        break;
       148	                    }
       149	                }

Resolution: track the previous character and do not count a `>` preceded by `-` as a close (or skip `->` as a unit); add a fixture `impl<F: FnMut() -> u8> Thing<F> {\n    pub fn poke(&self) {}\n}` extracting as `Thing::poke`. Acceptance: the fixture is red on the current parser (it yields `u8::poke`) and green after.
Construction: the fixture above with `spec(None)`: `extract_public_fns` returns `{"u8::poke"}`.

### surface-roster-30: surface-scan fixtures hand-roll unique temp dirs and never remove them
- Where: crates/surface-scan/src/tests.rs:14-25 (related: crates/surface-scan/Cargo.toml:12)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read the fixture; `tempfile` appears in no workspace manifest; surface-scan declares no dependencies at all); executed: no
- Seen by: adequacy, structure-prose; refutation: confirmed; history: no-rationale-found (c665f153 fixed a cross-checkout race with pid keying and did not weigh tempfile)
- Owner-gated: no

Each fixture creates `surface-scan-fixture-<name>-<pid>` under `std::env::temp_dir()` and leaves it behind, accumulating one directory per test per run, and the comment has to carry the race reasoning that `tempfile::TempDir` removes along with the leak. Prefer a dependency over hand-rolling.

Evidence:

        18	    let dir = std::env::temp_dir().join(format!(
        19	        "surface-scan-fixture-{name}-{}",
        20	        std::process::id()
        21	    ));

Resolution: add `tempfile` as a dev-dependency of surface-scan; `fixture` returns the `TempDir` (callers keep it alive for the scan) and writes `lib.rs` into it. Acceptance: no `surface-scan-fixture-*` directories remain after `cargo nextest run -p surface-scan`.

### surface-roster-31: citecheck re-parses the typed roster lexically with its own extraction guards, and its rejected-alternative note omits the external-binary route surfacecheck already demonstrates
- Where: tools/citecheck:59-68 (related: tools/citecheck:122-289, tools/citecheck:438-807, crates/before/surfacecheck/Cargo.toml:21, crates/before/src/lib.rs:441-454, crates/before/src/testing/surface_coverage.rs:116)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (read the header and the extraction functions at 122-289; `wc -l` = 856; surfacecheck links `before` with `meter` at Cargo.toml:21; `TRIPWIRES` is `pub(crate)` under `#[cfg(test)] mod testing` (lib.rs:453-454) and `laws` is behind its feature (lib.rs:444-445)); executed: no
- Seen by: structure-prose; refutation: reframed (viable, but a design change: `TRIPWIRES` and `DIFF_BESPOKE` must leave the cfg(test) tree first); history: deliberate-and-holds for the tools/ route (the rejected alternative is recorded and its reason still holds); the third option was not weighed although 9aa8c9ac had established the detached-binary idiom two weeks earlier
- Owner-gated: yes (moving test-tree tables under `meter`; retiring an instrument)

citecheck extracts `Leg::Bound/Law/Trans` citations, exclusion payloads, `TRIPWIRES`, `DIFF_BESPOKE`, and law and descriptor names from Rust source with Python regexes, then defends that lexical scan with extraction floors, a spelling-totality guard, a shadow guard, a fabricated-citation tripwire, and a long `--self-test`. Its rejected-alternative note considers only an in-crate `#[cfg(test)]` pin; a Rust binary in a detached workspace that links `before` with `meter`, iterates the typed roster, and reads the nextest inventory file as an input artifact keeps the layering the note wants while dissolving the regex extractors and the guards that exist only because the roster is read as text (Principle 3). The retirement bar applies: the replacement must reproduce citecheck's self-test red paths before citecheck is deleted.

Evidence:

        59	Rejected alternative, for the record: an in-crate `#[cfg(test)]` pin
        60	iterating the live `SURFACE` arrays against an inventory of collected test
        61	names. The tables are data and the crate already resolves them against its
        62	registries, but only the runner knows what it collects, and shelling out to
        63	`cargo nextest list` from inside a test binary that same runner is
        64	executing is not house style and inverts the layering; the crate cannot
        65	attest its own collection from within. The `tools/` linter route keeps the
        66	runner's inventory an input artifact, at the cost of reading the law and
        67	descriptor tables lexically — a cost the floors, the spelling totality,
        68	and the shadow guard price in.

Resolution: a second surfacecheck subcommand (or sibling binary) that iterates `METHOD_SURFACE`/`FAMILY_SURFACE` and `laws::registered_names`, reads the `cargo nextest list --message-format json` artifact the recipe hands it, and keeps the fabricated-citation tripwire and the shadow guard; move `TRIPWIRES` and `DIFF_BESPOKE` (or their name lists) under `meter`; reproduce citecheck's self-test red paths as unit tests; then retire `tools/citecheck`. If the owner keeps the tools/ route, add the omitted option to the note with the reason it was not taken. Acceptance: the justfile's `citecheck` recipe invokes the Rust checker and `tools/citecheck` is deleted; or the note records the third option.

## Positives

- surfacecheck refuses a rustdoc JSON `format_version` mismatch before any schema-typed parse (main.rs:67-85), names both numbers, and the bump procedure is written at the exact `rustdoc-types = "=0.59.0"` pin (Cargo.toml:22-29) and in the justfile recipe. A nightly bump can fail the leg but cannot make it silently wrong.
- Every reconciliation in the partition is two-way: roster against extraction (tests.rs:77-94), census pins against reachable impls and items (check.rs:243-258), duplicate names against their roster (282-303), binding kinds against each other (314-332), dead and shadowing exceptions (check.rs:305-321). Nothing here can go green by a counter going dark, and check.rs:12-15 states correctly why the censuses need no separate anchor.
- `reconcile_with` (check.rs:228-329) is a pure function over explicit tables, and check/tests.rs demonstrates every finding category red on synthetic input, including the empty extraction tripping the anchors and the module-prefix `::` non-leak.
- The refusal-over-under-report discipline is applied where the extractors recognize a shape: surface-scan panics on an unnamed `pub fn` and a nested impl (lib.rs:104-127); the rustdoc walk panics on dangling ids, unexpected item kinds, and unhandled type or generic-args shapes (extract.rs:68-73, 239, 277, 352, 361); a malformed `use` target asserts rather than shrinking the surface (197-204).
- The typed `Exclusion` vocabulary (surface.rs:57-150) makes an exclusion a defended family with a resolvable payload rather than free text; per-family structural obligations are enforced (a `GridCap` guard must be a `#[test]`, a `NotAPaperObject` `bound_at` must resolve to a row, test, or law), and `every_exclusion_family_is_inhabited` applies the scaffolding audit to the enum itself.
- One roster, several consumers, no copy: surfacecheck, the in-tree suite, before-fuelscape, and the board coverage tiling all bind to `before::surface::METHOD_SURFACE` under `meter`, so the enumeration the reviewer edits is the one every instrument enforces.
- Citation integrity is closed from more sides than most rosters bother with: bare names, payloads and `bound_at`, a positive-and-negative haystack liveness test, same-named tests across files held equal both ways, kinds that shadow each other, and collection by the runner (tools/citecheck, whose fabricated-citation tripwire runs on every live run and whose header records the rejected alternative).
- `tests/doc_hidden.rs` and `tests/foreign_reexport.rs` close the two channels both extractors structurally miss, and extract.rs:31-32 states the dependence explicitly. Together with surfacecheck the three are total over documented, hidden, and re-exported surface.
- `--list` (main.rs:141-176) renders every extracted row with its disposition, which is exactly the triage view a reviewer needs when a totality run goes red.

## Open questions for Finch

1. Retire the line-scan extractor (surface-roster-9), or keep it as a stated stable-toolchain inner-loop convenience? `just ci` runs `test-all` but not `surface-totality` (that runs in the `instruments` CI job and the gate's `surface` stream), so retirement moves the only totality feedback to the gate. Recommendation: retire; the JSON walk catches strictly more, the retirement bar is met, and the doc-hidden contradiction (surface-roster-21) dissolves with it.
2. For `FAMILY_SURFACE` (surface-roster-7): correct the three prose sites now and treat binding census pins to family rows as a design round, or build the binding directly? Recommendation: prose fix plus rows for the orphans now; the binding as a follow-on, since it reshapes the meter-public `SurfaceRow` and the three non-impl rows need a disposition.
3. The dated `decided` convention (surface-roster-17) is repo-wide (surfacecheck and the meter registry) and was reported to you as an open design item in d2a9d04e. Recommendation: drop the field at both sites and let git carry the when; if kept, record the exception to Principle 5 once in the project instructions and unify the validator.
4. Should the rustdoc census cover public struct fields, enum variants, and type aliases (surface-roster-22)? Recommendation: enumerate-and-panic and the prose fix now; extend the census only if you want shape changes diff-visible in the gate, since the roster's vocabulary is operations.
5. Per-suite citations versus `DUPLICATE_TEST_NAMES`: a cited-name uniqueness rule would let the roster go, but the roster's file pinning is what the seed tripwire composes with (moving `join_all_matches_the_recursive_oracle` to another file breaks `d1_seeds_stay_committed` through the file list). Recommendation: keep the roster; it is a deliberate, inline-defended choice.
6. `StructuralPartialEq` rows (surface-roster-24): keep for the derived-vs-hand-written signal, or skip beside `is_synthetic` to remove a perma-unstable marker from the pin list? Recommendation: skip, with the reason stated at the skip; a derive-to-manual change is visible in the diff that makes it.
7. "door" is used as crate-wide vocabulary in the roster (surface.rs:78, 103, 111, 235) with no defining site I could find. Is it defined somewhere outside this partition, or does it need one anchor?

## Dropped

- [3] DUPLICATE_TEST_NAMES dissolution: deliberate and documented in code (tests.rs:15-23, 1e6af44f); the proposed uniqueness rule drops the file pinning the seed tripwire composes with; carried as open question 5.
- [8] Vestigial forks entries: prophylactic mappings with no current payload, folded into surface-roster-9; the "coverage note" ghost is a site of surface-roster-20.
- [19], [32], [53]: duplicates of surface-roster-7.
- [2], [28], [38], [58]: duplicates merged into surface-roster-17 (dated rulings); [22], [33], [61]: merged into surface-roster-18 (ITEM_EXCEPTIONS).
- [21], [31], [57]: duplicates of surface-roster-10.
- [30]: duplicate of surface-roster-12.
- [25], [40], and the `d1_` item of [62]: duplicates of surface-roster-15; the `d_fork_join_roundtrip` item of [62] is outside this partition (party/tests.rs).
- [27], [37], [60]: duplicates of surface-roster-3; the `20`-threshold item of [27] is in surface-roster-26.
- [26], [42], [43], [59]: duplicates merged into surface-roster-2 (repetition and counts) and surface-roster-20 (ghost and temporal references).
- [45], and the idiom items of [62]: merged into surface-roster-26.
- [52]: duplicate of surface-roster-11.
- [20]: duplicate of surface-roster-28.
- [48]: duplicate of surface-roster-30.
- [35]: duplicate of surface-roster-6.
- [6], [39]: merged into surface-roster-20.
- [9], [34], [47]: merged into surface-roster-24.
- [24], [54]: merged into surface-roster-22.
- [15], the metaphor items of [41] and [62]: merged into surface-roster-25.
- [41] TRIPWIRES rename: the two-genre sense is defined at its site by contrast (surface_coverage.rs:71-92, 112-113); a vocabulary preference, not a defect.
