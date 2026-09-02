# Partition fuzz-guests-pins: The libFuzzer targets, the fuzz-fit wasm guest, and the 32-bit boundary pins (guest and harness)

## Partition summary

This partition holds three detached instruments that execute `before` where the ordinary test suite cannot. The five libFuzzer targets in `crates/before/fuzz` feed arbitrary bytes and text to the public decode and parse doors under a shared peak-heap cap: `fuzz_decode` asserts byte identity on accept, `fuzz_decode_differential` holds the fused, borsh, and postcard paths to agreement on accept, value, and rejection genre, `fuzz_decode_ops` drives the clock op set on decoded trees, `fuzz_laws` expands the law-group roster over decoded values, and `fuzz_parse` round-trips the display notation. The fuzz-fit guest (`crates/before/fuzzfit/guest`) is a C-ABI register machine over the public API, compiled to wasm32 so the harness can meter each public operation in wasmtime fuel; it is the only route to fuel readings and ships a quadratic self-test burner as the meter's adequacy check. The wasm32-pins workspace is the tree's one place 32-bit code executes: the guest synthesizes canonical streams of hundreds of megabytes in-guest and drives the doors, walks, emitters, and rank arithmetic at the 2^29-bit, 2^29-byte, backend-capacity, and exponent-gap coordinates, and the harness pins each coordinate's outcome with adjacency witnesses on both sides.

The construction is strong where it matters most. The seed corpus is derived from the live API in one place and held byte-identical by a gate test; the differential target models rejection genres and stages explicitly and spells its one allowed divergence at the arm that admits it; the law target expands from `before::for_each_law_group!`, so a new law group is fuzzed with no wiring; the fuzz-fit guest keeps each measured window to exactly one public operation and pre-reserves its register file; the wasm32 guest documents every synthesized layout bit by bit with the strict decoder as the synthesizer's oracle, and the release profile keeps overflow checks on.

The dominant issues are instruments whose passing set is wider than the failure class they name. The fuzz heap cap is a flat 1 GiB against 4096-byte inputs, so any amplification below 2^18 times the input (a unit-constant quadratic included) passes, and nothing committed demonstrates the cap fires. The three memory-terminal pins assert `Trap::UnreachableCodeReached`, which every guest abort produces, so an overflow-check panic (the class the suite audits) or a backend-capacity panic at those coordinates reads identically to the modeled allocation failure; those same pins carry a red-first label while describing the terminal as intended, so their genre is unsettled. The rank pins observe only coarse order, which a decoder that drops the seam bit satisfies. The overflow-checks premise has no liveness self-test. The fuzz targets' own oracles execute only at `just all` cadence, `wasm32-pins/Cargo.lock` sits outside the supply-chain audit, and the wasm32-pins leg has no CI counterpart and no declared exclusion. The rest is prose and small structure: opaque roster IDs and a ghost test name in the fuzz manifest and README, a vestigial `BUILD_CAP_BYTES` name for a cap the tree no longer has, a harness fallback path that resolves outside the repository, hand-expanded accessor and verdict-table patterns in the fuzz-fit guest, and vocabulary tells.

Lines read: the sixteen partition files total 4808 lines (fuzz/Cargo.toml 98, fuzz/README.md 80, fuzz/src/lib.rs 48, the five targets 872, fuzzfit/guest/src/lib.rs 2038, fuzzfit/guest/Cargo.toml 15, wasm32-pins/guest/src/lib.rs 726, wasm32-pins/harness/src/lib.rs 115, wasm32-pins/harness/tests/pins.rs 743, the three wasm32-pins manifests 73), all read in full with line numbers, plus the cited regions of the justfile, `tests/support/fuzz_seed_set.rs`, `tests/fuzz_seeds.rs`, the fuzz-fit harness (`wasm.rs`, `ops.rs`, `bands.rs`, `tests/enforce.rs`, `tests/main.rs`), `clock.rs`, `clock/tests.rs`, `laws.rs`, `testing/algebraic_laws/tests.rs`, `borsh_impls.rs`, `span/wire.rs`, `version/rank/num.rs`, `validation_index.rs`, `tests/meter.rs`, `.github/workflows/ci.yml`, the sibling `.cargo/config.toml` files, `tools/doclint`, the resource-amplification agent note, and the `peak_alloc` 0.3.0 and `wasmtime-environ` 47.0.3 sources from the registry. `pins.rs` is the partition's only test file; the fuzz targets are instrument binaries and the two guests are instrument code. No cargo, just, build, or test command was run; every claim below rests on reading, grep, git history, path normalization, or arithmetic, and each entry says which.

## Findings

### fuzz-guests-pins-1: The wasm32-pins leg runs nowhere in CI and its exclusion is undeclared
- Where: .github/workflows/ci.yml:110-120 (related: justfile:985-986, justfile:1000, justfile:467, justfile:613-614, justfile:633-635)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the `ci` recipe line, the gate's stream roster, and both comments that enumerate the local-only legs); executed: no
- Seen by: refutation pass (raised as new); refutation: new; history: not examined
- Owner-gated: yes: gate and CI policy

The `ci` recipe (justfile:1000) omits `wasm32-pins` (and `fuzzfit`), and the two comments whose stated purpose is to name which instrument legs stay local and why (ci.yml:110-120, justfile:985-986) name only the wall-time judge and "the wasmtime fuel tier (the fuzzfit bands and the fuelscape pins)". wasm32-pins is deterministic and is not a fuel tier, and the justfile calls it "the tree's one place 32-bit code *executes*" (613-614), so the gate's wasm stream has a leg with no CI counterpart and no stated reason. Doctrine: every status board an acceptance concept exists for is wired into the required checks, or its exclusion is stated positively where the roster lives.

Evidence:

    110	  # The gate's deterministic instrument legs, re-run on the runner. Only
    111	  # counter-based legs ride here: their readings are byte-identical under
    112	  # any machine load, so a shared runner cannot flake them. What stays
    113	  # local, and why:
    114	  #   - the judged wall-time legs (bench-judge and its tripwire): criterion
    115	  #     exponent fits need the quiet-machine regime a shared runner cannot
    116	  #     promise;
    117	  #   - the wasmtime fuel tier (the fuzzfit bands and the fuelscape pins):
    118	  #     deterministic, but it brings a wasm32 guest build plus wasmtime to
    119	  #     police asymptotics the board legs below already judge at the scales
    120	  #     of record — the gate keeps the second jaw.

    (justfile:467)     start_stream wasm         10 fuzzfit fuelscape-test wasm32-pins
    (justfile:1000) ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

Resolution: Owner call. Either add `wasm32-pins` to the `instruments` job (the recipe prices it at minutes and a few GiB across nextest workers; it builds `before` for wasm32 plus wasmtime), or add a bullet to both comments declaring it local and why. Acceptance: every leg in the gate's stream roster (justfile:464-471) either appears in `ci` or the `instruments` job, or is named in the local-only list with its reason.
Construction: Change one expected value in pins.rs (`Outcome::Value(0)` to `Outcome::Value(1)` at line 34) and push: `just ci` and the `instruments` job stay green; only a local `just gate` reddens.

### fuzz-guests-pins-2: Opaque roster IDs and a ghost test name in the fuzz manifest and README
- Where: crates/before/fuzz/Cargo.toml:1-1 (related: crates/before/fuzz/Cargo.toml:48, crates/before/fuzz/README.md:1, crates/before/fuzz/README.md:24-25, crates/before/src/clock/tests.rs:766-773)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rnE 'PROG-[0-9]|COV-[0-9]'` over crates/, justfile, tools/ hits exactly the three sites; `grep -rn h34` over crates/before hits only README.md:25; the live test is `fn decode_never_panics` at clock/tests.rs:773, whose doc at 766-767 states the `decode(b) == Ok(x) ⟹ is_normal(x)` implication the README describes); executed: no
- Seen by: scaffolding [2],[3]; adequacy [27]; structure-prose [48],[49]; instrument-correctness [65],[66]; refutation: confirmed; history: contradicts-hard-rule (the tags were born with the itc-era plan in 5a2679c9a, the plan retired in 7384d0450, and the test-name scrub 90f903c33 never touched fuzz/)
- Owner-gated: no

`PROG-5` and `COV-7` are plan tags defined nowhere in the tree, and `clock::tests::h34_decode_never_panics` names a test that does not exist; the live test is `clock::tests::decode_never_panics`. Hard rules: no opaque roster IDs in code or prose; nothing refers to code that no longer exists.

Evidence:

    (Cargo.toml:1)  # Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
    (Cargo.toml:48) # keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes
    (README.md:1)   # `before` fuzz targets (PROG-5 / COV-7)
    (README.md:24-25)
      structural `is_normal`-on-accept form of the same invariant is checked by
      the in-tree proptest `clock::tests::h34_decode_never_panics`.

Resolution: Delete the three parenthetical tags (the surrounding sentences already name the invariant in plain words) and cite `clock::tests::decode_never_panics`. Acceptance: `grep -rnE 'PROG-[0-9]|COV-[0-9]|h34_' crates/before` is empty and the cited test name resolves to a `fn`.

### fuzz-guests-pins-3: The fuzz run commands are spelled three ways, two have drifted, and the README misstates the gate's toolchain
- Where: crates/before/fuzz/Cargo.toml:3-9 (related: crates/before/fuzz/README.md:6-9, crates/before/fuzz/README.md:54-60, crates/before/fuzz/README.md:72-74, justfile:42-54, justfile:366-368, justfile:468, justfile:555-559)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (compared the three command lists; read the gate stream roster and the `fuzz-build` recipe); executed: no
- Seen by: scaffolding [4]; adequacy [27]; structure-prose [50]; refutation: confirmed, and raised the README:8-9 claim as new; history: deliberate-but-expired (the manifest header was accurate before fd5c92bf5 made seeding a named-directory contract and c6fac5c7d added `--target` to the justfile alone)
- Owner-gated: no

The manifest header's commands pass no seed directory, so following them runs unseeded (README.md:72-74 says seeds load only when named); neither the manifest nor the README passes `--target`, which justfile:42-47 explains a prebuilt cargo-fuzz needs; and README.md:8-9 says the gate does not need nightly or libFuzzer, which is false: `fuzz-build` is a gate leg (justfile:468) and runs `cargo +nightly fuzz build` (justfile:368). AGENTS.md names the justfile as the source of truth for verification; the duplicates are where drift lives, and justfile:51-52 still cross-refers to the manifest header for the smoke duration.

Evidence:

    3	# tries to build it (it needs nightly + libFuzzer). Build/run with cargo-fuzz only:
    4	#   cargo +nightly fuzz build
    5	#   cargo +nightly fuzz run fuzz_decode -- -max_total_time=20
    (README.md:8-9)
    gate never tries to build it. Fuzzing needs a nightly toolchain and libFuzzer; the gate
    does not.
    (justfile:368)     {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

Resolution: Replace Cargo.toml:3-9 with one pointer to `just fuzz-build` / `just fuzz`; in the README keep only the crash-reproduction line the recipe does not cover and point at the recipe for the rest; correct README.md:8-9 to say the gate builds the targets on nightly and only the smoke runs at `just all` cadence; point justfile:51-52 at the README or drop the cross-reference. Acceptance: one seeded, `--target`-bearing spelling of the invocation remains (the justfile); the README's gate sentence agrees with justfile:468.

### fuzz-guests-pins-4: `fuzz_decode`'s assertions are implied by `fuzz_decode_differential`'s borsh arm
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode.rs:14-17 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:59-64, crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:254-306, crates/before/tests/support/fuzz_seed_set.rs:29-35)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: assessed (read `borsh_vs_raw`'s three branches against `fuzz_decode::run`); executed: no
- Seen by: scaffolding [5]; refutation: confirmed; history: no-rationale-found (778c89f50 introduced the differential target without discussing retention)
- Owner-gated: yes: removal of an instrument

For each of the six wire types, `borsh_vs_raw` runs the raw whole-slice `decode(data)` in every branch: when borsh accepts the whole slice it asserts `encode(&raw) == encode(&value) == consumed == data` (fuzz_decode's byte identity); when borsh accepts a proper prefix or rejects, it asserts the raw decode rejects, so fuzz_decode's `if let Ok` cannot fire. The re-decode and re-encode-stability checks follow from decode determinism on identical bytes. fuzz_decode therefore catches nothing the differential target does not on the same input distribution; the plausible retention reasons (executions per second on the raw doors, a coverage map without borsh, independence from the transport features) are real but unstated. Circular justification is the tell: an instrument names what it alone catches.

Evidence:

    14	//! The roster is every public wire type: `Party`, `Version`, `Clock`, `Rank`,
    15	//! `Ranked`, and `Span`. The composite decodes (`Ranked`, `Span`) are additionally
    16	//! held to their composed counterparts — on rejection genre, not just accept — by
    17	//! the sibling `fuzz_decode_differential` target.

Resolution: Either keep the target with one sentence in its module doc naming the payload it alone provides, or dissolve it into the differential target, re-homing its 16 committed seeds (including the two non-derivable frontier witnesses) under `seeds/fuzz_decode_differential` and updating `fuzz_seed_set.rs`. Acceptance: the target's doc names its unique payload, or the target is gone with `tests/fuzz_seeds.rs` green.

### fuzz-guests-pins-5: The six-type wire roster is spelled three times across the decode targets
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode.rs:31-86 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:59-71)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [18]; structure-prose [52]; refutation: confirmed (whether a macro reads better is owner taste); history: no-rationale-found
- Owner-gated: no

`fuzz_decode::run` is six nine-line blocks differing only in type and noun, and `fuzz_decode_differential::run` lists the six types twice (borsh, postcard). A seventh wire type is a three-site edit with no check that all three were touched. Legibility: the module doc's three-assertion contract should read once in code.

Evidence:

    31	    if let Ok(p) = Party::decode(data) {
    32	        let bytes = p.encode();
    33	        assert_eq!(
    34	            bytes, data,
    35	            "accepted party bytes were not canonical as fed"
    36	        );
    37	        let again = Party::decode(&bytes[..]).expect("re-decode of an accepted party is canonical");
    38	        assert_eq!(again, p, "accepted party did not round-trip");
    39	        assert_eq!(again.encode(), bytes, "party re-encode is not stable");
    40	    }

Resolution: One `macro_rules! round_trip { ($ty:ty, $noun:literal) => ... }` invoked six times in `fuzz_decode` (a generic fn is blocked by the absence of a shared decode/encode trait), and one roster list both `borsh_vs_raw` and `postcard_vs_composed` expand from. Acceptance: adding a type name to one list adds it to every arm; the assertions and messages are unchanged.

### fuzz-guests-pins-6: The differential's composite allowance admits every `(NotCanonical, TrailingBits)` pair on `Ranked` and `Span`
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:293-300 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:130-139, crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:237-245, crates/before/src/borsh_impls.rs:275-287, crates/before/tests/support/fuzz_seed_set.rs:195-203)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read the arm, the borsh `Span` reader's cursor-then-verdict order, and the `span_crossed_padding` seed); executed: no
- Seen by: adequacy [31]; refutation: confirmed, narrower than stated (a wrong pair verdict on an input with no trailing bytes is still caught by the Err-arm expect); history: deliberate-and-holds (the allowance and its scope are documented at 237-245; conditioning it on the stage was never considered)
- Owner-gated: no

The arm admits `(NotCanonical, TrailingBits)` whenever `composite` is true, regardless of whether borsh actually rejected at its composite check. A composite-local defect in the borsh reader that reports `NotCanonical` for a structural fault the raw door calls `TrailingBits` is admitted by the same arm, and `span_differential` decodes `hi` raw (line 134), so it never sees the borsh path. The cheapest passing artifact: an allowance conditioned on a type admits every divergence of that genre pair on that type, not only the documented one.

Evidence:

    293	            let agreed = match (borsh, raw) {
    294	                (b, r) if b == r => true,
    295	                // A prefix-scoped composite check rejects where the
    296	                // whole-slice parse still sees unconsumed input — possible
    297	                // only on the types that run one.
    298	                (Genre::NotCanonical, Genre::TrailingBits) => composite,
    299	                _ => false,
    300	            };

Resolution: Condition the allowance on evidence that the composite check was reached: parse both components with the borsh prefix readers, run the pair check, and allow `(NotCanonical, TrailingBits)` only when that composed spelling rejects at `Stage::Pair` with a nonempty remainder; otherwise require exact genre agreement. Acceptance: a borsh `Span` reader mutated to wrap `hi`'s `TrailingBits` as `NotCanonical` diverges on the committed `span_crossed_padding` seed; the committed reader agrees on every seed.
Construction: In `borsh_impls.rs`'s `Span::deserialize_reader`, map the cursor's `Decode::TrailingBits` (line 285) to `NotCanonical` before the pair check, then run `fuzz_decode_differential` on `seeds/fuzz_decode_differential/span_crossed_padding`: raw `Span::decode` says `TrailingBits`, borsh says `NotCanonical`, `composite = true` admits it.

### fuzz-guests-pins-7: Em-dashes inside `//` comments at six sites
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:296-296 (related: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:193, crates/before/fuzzfit/guest/src/lib.rs:534, crates/before/fuzzfit/guest/src/lib.rs:1355, crates/before/fuzzfit/guest/src/lib.rs:1840, crates/before/wasm32-pins/guest/src/lib.rs:167)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nE '^\s*//[^/!].*—'` over the partition's ten .rs files returns exactly these six lines; the same grep over crates/before/src returns 374 lines); executed: no
- Seen by: structure-prose [55]; refutation: confirmed; history: contradicts-hard-rule, but the practice is crate-wide, so a partition-local fix would be inconsistent
- Owner-gated: no

Doctrine prefers colons or semicolons over em-dashes in code comments (terminal compatibility); rustdoc keeps the em-dash. Six line comments in the partition use it, against 374 under crates/before/src.

Evidence:

    296	                // whole-slice parse still sees unconsumed input — possible

Resolution: Batch into a crate-wide sweep rather than fixing these six alone; per site, a colon, semicolon, or parenthetical. Acceptance: the grep above returns nothing, crate-wide.

### fuzz-guests-pins-8: `fuzz_decode_ops`'s module doc and flavour-1 comment describe only flavour 0's framing
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:11-15 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:48-60, crates/before/fuzz/Cargo.toml:68-70, crates/before/fuzz/README.md:36-39, crates/before/tests/support/fuzz_seed_set.rs:29-35, crates/before/tests/support/fuzz_seed_set.rs:290-299)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the two arms against `fuzz_seed_set.rs`'s `clock_then_msg` spelling: `[1u8, len] ++ clock_bytes ++ sibling.version().encode()`); executed: no
- Seen by: scaffolding [17]; structure-prose [46]; instrument-correctness [72]; adequacy [27] (the Cargo.toml:68 ride-along); refutation: confirmed; history: wrong from birth (5a2679c9a), kept through two later doc passes
- Owner-gated: no

The module doc says the remainder after the value chunk "is the op script (one op per byte)" and calls the framing a wire contract with the seed set; in flavour 1 the arm decodes a `Clock` from the chunk and the whole remainder as a `Version`, and its own comment at line 48 says it decodes "a Version (message)" where line 50 decodes a `Clock`. Cargo.toml:68 ("a `Clock` (or `Version`)"), README.md:36-37, and `fuzz_seed_set.rs:33-35` repeat the one-flavour description. A doc that declares itself a wire contract must state both flavours; a maintainer regenerating seeds from it would spell flavour 1 wrong.

Evidence:

    11	//! The first byte selects the value flavour, the next length-prefixed chunk is
    12	//! the value's bytes, and the remainder is the op script (one op per byte).
    13	//! This framing and the op table below are a wire contract with the committed
    14	//! seed corpus: `tests/support/fuzz_seed_set.rs` spells seeds in exactly this
    15	//! shape, so a change here means regenerating the seeds with it.
    48	        // Decode a Version (message) and exercise the version-facing ops.
    49	        _ => {
    50	            let Ok(mut clock) = Clock::decode(value_bytes) else {

Resolution: Module doc: "flavour 0: the remainder is an op script, one op per byte, over the decoded clock; flavour 1: the remainder is a `Version` message the decoded clock compares against and receives". Line 48: "Decode a Clock, then compare against and receive a Version decoded from the remainder." Mirror the two-flavour sentence in `fuzz_seed_set.rs:33-35`, Cargo.toml:68, and README.md:36-37. Acceptance: every description of the framing names both flavours and matches both `match` arms.

### fuzz-guests-pins-9: `drive_clock` swallows `join`/`sync` errors its own construction proves impossible
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:77-91 (related: crates/before/src/clock.rs:199-202, crates/before/src/clock.rs:322-328, crates/before/src/laws.rs:2029-2040, crates/before/tests/support/fuzz_seed_set.rs:279-288)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `Clock::join`'s `# Errors` and `Clock::sync`'s `sum_split` arm: overlap is the only error path); executed: no
- Seen by: structure-prose [47]; refutation: confirmed, severity low (the single-fork case is already caught by `fork_halves_disjoint` in `fuzz_laws`; the residual is multi-op sequences); history: no-rationale-found (the hedge is verbatim from the target's first commit)
- Owner-gated: no

Every stash entry is `clock.fork()` or a child `sync` re-split from `clock`, so `join` and `sync` can fail here only if `Party::fork` or `sum_split` produced overlapping parties on a decoded tree, which is exactly the bug class this target exists to reach; the code pushes the error back or discards it under "on the off chance it errors". The cheapest passing artifact: a target whose contract is "no panic" should panic on the one error it can prove impossible, or the fuzzer minimizes nothing when a decoded party forks into overlapping halves after prior ops.

Evidence:

    78	                if let Some(child) = stash.pop() {
    79	                    // Re-join a disjoint fork; on the off chance it errors, keep the
    80	                    // returned clock so we never lose the share.
    81	                    if let Err(returned) = clock.join(child) {
    82	                        stash.push(returned);
    83	                    }
    84	                }
    ...
    88	                    let _ = clock.sync(&mut child);

Resolution: `.expect("a forked or re-split child is disjoint from its origin")` on both, with the comment rewritten as the one-line proof (every stash entry is a fork or sync re-split of `clock`, so overlap here is a `Party::fork`/`sum_split` defect). Acceptance: a `Party::fork` deliberately returning an aliased party crashes `cargo +nightly fuzz run fuzz_decode_ops seeds/fuzz_decode_ops` on the `clock_then_ops` seed (ops `[0, 1, 3, 5, 2, 4, 6, 7]` drive fork, sync, then join).
Construction: Locally make `Party::fork` return `self.dangerously_alias()` and replay the seed: today the run stays green because `join`'s `Err` is pushed back and `sync`'s `Err(Overlap)` is dropped.

### fuzz-guests-pins-10: The fuzz framing is a prose wire contract duplicated across the detached boundary
- Where: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:26-29 (related: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:46-55, crates/before/fuzz/fuzz_targets/fuzz_laws.rs:65, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:13-15, crates/before/tests/fuzz_seeds.rs:290-299, crates/before/tests/fuzz_seeds.rs:308, crates/before/tests/support/fuzz_seed_set.rs:263-268, crates/before/tests/support/fuzz_seed_set.rs:301-309)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`ARITY_SPAN = 18` at fuzz_laws.rs:65 and again at tests/fuzz_seeds.rs:308; `chunk` at fuzz_laws.rs:46-55 and `laws_chunk` at tests/fuzz_seeds.rs:290-299 are textually identical bodies); executed: no
- Seen by: adequacy [26]; refutation: confirmed; history: no-rationale-found (fd5c92bf5 chose the cross-pointer convention without discussing a shared definition)
- Owner-gated: no

`ARITY_SPAN`, the chunk carve, and the decode-ops flavour/length framing are re-spelled in `tests/fuzz_seeds.rs` and `tests/support/fuzz_seed_set.rs`, held together only by "a change here means regenerating the seeds with it". The seed test's in-band assertion reads its own copy, so a narrowed band in the target folds seed arities silently while the test stays green. Every hole found becomes a committed check, never a convention held in memory.

Evidence:

    26	//! feeding the variadic law groups. This framing is a wire contract with
    27	//! the committed seed corpus: `tests/support/fuzz_seed_set.rs` spells seeds
    28	//! in exactly this shape, so a change here means regenerating the seeds
    29	//! with it.
    (tests/fuzz_seeds.rs:308)     const ARITY_SPAN: usize = 18; // the target's arity band, per its framing

Resolution: Extract the framing (`ARITY_SPAN`, `chunk`, `byte`/`picks`, the decode-ops flavour/length carve) into one file `#[path]`-included by the targets, `tests/fuzz_seeds.rs`, and `tests/support/fuzz_seed_set.rs` (the seed set already shares by `#[path]` between the example and the test). Acceptance: `ARITY_SPAN` has one definition; the seed test's in-band assertion reads the constant the target folds with.
Construction: Set `ARITY_SPAN` to 16 in fuzz_laws.rs only: the seed test still passes (its copy asserts `< 18`), while the target now folds the seed's arity-17 script to 1 and never crosses the second octave it was written to cross.

### fuzz-guests-pins-11: One-byte length prefixes cap every decoded law-target operand at 255 bytes
- Where: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:46-55 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:32-38, crates/before/tests/support/fuzz_seed_set.rs:277, crates/before/tests/support/fuzz_seed_set.rs:313-318)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read both carves and the seed set's `u8::try_from(...).expect(...)`); executed: no
- Seen by: adequacy [32]; refutation: reframed (the cap binds every `fuzz_laws` operand; in `fuzz_decode_ops` it binds only the seed value, since the script's fork/join/ticks ops grow the driven clock without bound); history: no-rationale-found
- Owner-gated: no

`fuzz_laws` decodes every operand from a `u8`-length chunk, so no law is ever driven on a decoded tree over 255 bytes, and any input past about 1.5 KB spends its excess on the list scripts; the deep and wide regime where the fold shapes and the heap cap have work to do is unreachable for this target. The sizing choice is stated nowhere. In `fuzz_decode_ops` the same prefix bounds only the seed value. Does the sweep's population reach the worst case the crate docs admit exists.

Evidence:

    46	fn chunk<'d>(data: &mut &'d [u8]) -> &'d [u8] {
    47	    let Some((&len, rest)) = data.split_first() else {
    48	        *data = &[];
    49	        return &[];
    50	    };
    51	    let split = (len as usize).min(rest.len());

Resolution: Widen the prefix to `u16` (little-endian, saturated at the remainder) in both targets and regenerate the seeds, or state at the framing why 255 bytes is the intended operand ceiling for the law target. Acceptance: a regenerated seed carrying a canonical clock over 255 bytes decodes in `fuzz_laws` and drives every group; the seed set's `u8::try_from` becomes `u16`.

### fuzz-guests-pins-12: `drive_groups!` duplicates the in-tree `organic_drive!` selection macro arm for arm
- Where: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:122-180 (related: crates/before/src/testing/algebraic_laws/tests.rs:336-394, crates/before/src/laws.rs:109-134)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read both macros against the roster: eighteen arms each over the same signatures, differing in field name `k` versus `c` and pool indices `p[0]` versus `p[1]`/`p[2]`, `v[0]` versus `v[1]` for `(clock, version)`); executed: no
- Seen by: structure-prose [54]; refutation: confirmed; history: no-rationale-found (86dd53a71 landed both without explaining two selection macros)
- Owner-gated: yes: adds an exported macro to the `laws` instrument feature

The signature-to-inputs selection is a property of the roster, not of either driver, yet both must be extended in lockstep whenever `for_each_law_group!` gains a signature, and the compile-time totality each claims is enforced twice. The two copies already differ in pool indices; the difference does not look load-bearing.

Evidence:

    122	macro_rules! drive_groups {
    123	    (args: ($env:expr); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
    124	        $( drive_groups!(@one $env, $group, $shape); )*
    125	    };
    126	    (@one $env:expr, $group:ident, (version)) => {
    127	        drive!(before::laws::$group, $env.v[0]);
    128	    };

Resolution: Move the eighteen selection arms into `laws.rs` beside `for_each_law_group!` as a macro taking the environment expression and the assertion macro (`assert!` here, `assert_laws!` in the tests), and have both consumers invoke it; fix the pool-index choice once. Acceptance: one spelling of the eighteen arms in the tree; a signature added to `for_each_law_group!` breaks the build in exactly one place.

### fuzz-guests-pins-13: `fuzz_parse` compares the `Clock` round-trip by `encode()` while its siblings compare by `==`
- Where: crates/before/fuzz/fuzz_targets/fuzz_parse.rs:53-63 (related: crates/before/src/clock.rs:51)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`#[derive(PartialEq, Eq, Hash)]` at clock.rs:51); executed: no
- Seen by: structure-prose [53]; refutation: confirmed; history: no-rationale-found (`Clock` already derived `PartialEq` when the target was written)
- Owner-gated: no

`Clock` derives `PartialEq`, so the byte comparison is an unexplained inconsistency a reader will misread as "`Clock` lacks `Eq`". Comments state what the code cannot show; here the code shows a different comparison with no reason.

Evidence:

    58	        assert_eq!(
    59	            again.encode(),
    60	            clock.encode(),
    61	            "clock display round-trip changed the value"
    62	        );

Resolution: `assert_eq!(again, clock, ...)` like the three siblings, or a one-line comment if the byte comparison is deliberate. Acceptance: all four blocks compare the same way, or the odd one says why.

### fuzz-guests-pins-14: The flat 1 GiB heap cap cannot see sub-2^18x amplification at fuzz input sizes, and nothing committed shows it fires
- Where: crates/before/fuzz/src/lib.rs:10-17 (related: crates/before/fuzz/src/lib.rs:28-29, crates/before/fuzz/src/lib.rs:37-47, crates/before/fuzz/Cargo.toml:23-28, crates/before/src/lib.rs:333-340, crates/before/tests/meter.rs:1-11, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:1533-1538)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (arithmetic: 2^30 / 4096 = 2^18; 4096^2 = 2^24 = 16 MiB, 64 times under the cap; committed seeds are at most 130 bytes by `wc -c`; no `#[test]` anywhere in the fuzz workspace by grep; `git show -s 48965ff3` records the trip as a hand run with a temporarily lowered ceiling; the agent note lists the proportional cap as an open closeout obligation); executed: no
- Seen by: scaffolding [0]; adequacy [24]; instrument-correctness [60]; structure-prose [51]; refutation: confirmed, with two corrections carried here (the ratio is 64 times, six binary orders, not "four orders"; the doc's "a trip means a new class" is the forward implication and true as written); history: already-known (the note records "Land the proportional ceiling, or the owner ratifies the flat cap with the harness doc re-derived to the ratified shape"; the committed trip demonstration is the unrecorded half)
- Owner-gated: yes: the note frames flat-versus-proportional as an owner ruling

At libFuzzer's default `-max_len` (4096; the committed seeds are at most 130 bytes, so the corpus does not raise the guess), the cap fires only on amplification of at least 2^18 times the input: a quadratic amplifier with unit constant peaks at 16 MiB and passes every run, although the crate docs make "auxiliary space at most a small constant multiple of the input size" a hard guarantee and this harness is the only space instrument over unchosen input shapes (`tests/meter.rs` pins chosen families). libFuzzer's own `-malloc_limit_mb` (default equal to `-rss_limit_mb`, 2048) already kills anything past about 2 GiB, so the cap's exclusive window is survivable peaks in [1 GiB, 2 GiB). No committed check demonstrates the cap fires (the bins are `test = false`; the lib has no test), so a broken `reset_peak_usage`/`peak_usage` pairing would read green forever. "not yet proportional" is dated rationale at a declaration site. Instruments before cures: every ceiling needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

    10	//! The ceiling is generous and absolute: a flat [`PEAK_HEAP_CAP_BYTES`],
    11	//! not yet proportional to input size. The known amplifiers are linear in
    12	//! the input with constants in the hundreds, and libFuzzer's default 4096-byte
    13	//! inputs keep them megabytes below the cap, so a trip means a new class of
    14	//! blowup, not a bigger constant. The peak is read after the body returns
    15	//! rather than enforced inside the allocator, so a spike that outruns the
    16	//! process before returning is stopped by libFuzzer's RSS limit instead; the
    17	//! cap's job is the far more common survivable amplification.
    29	pub const PEAK_HEAP_CAP_BYTES: usize = 1 << 30;

Resolution: Owner rules the shape (the note's two branches). If proportional: `cap = max(FLOOR_BYTES, PER_INPUT_BYTE * data.len())`, threading `data.len()` into `under_heap_cap`, with `PER_INPUT_BYTE` measured over the seeds and a smoke run first and committed with slack, so the doc's "constants in the hundreds" becomes an enforced number. If ratified flat: restate the doc positively without "not yet" and name the class the flat cap is for. Either way, add a `#[test]` to the fuzz lib (plain `cargo test --lib` in the workspace, runnable from `fuzz-build`) that `under_heap_cap` panics on a synthetic over-cap allocation and passes on a linear one. Acceptance: the lib test is green in the gate; under the proportional rule, `let _sink = vec![0u8; data.len() * data.len()];` planted in a target's `run` crashes on fuzzer-generated inputs near `-max_len` (not on the tiny seeds); "not yet" is gone.
Construction: Add `let _sink = vec![0u8; data.len() * data.len()];` to `fuzz_decode::run` and run `just fuzz`: peak 16 MiB at 4096 bytes, `peak <= 1 << 30` holds, no finding. For the liveness half: make `under_heap_cap` skip its assert; nothing committed reddens.

### fuzz-guests-pins-15: The cap asserts on absolute peak heap, not the "peak transient heap" its doc names
- Where: crates/before/fuzz/src/lib.rs:28-40 (related: crates/before/fuzz/Cargo.toml:23-28)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read peak_alloc 0.3.0 `src/lib.rs:107-110`: `reset_peak_usage` stores `CURRENT` into `PEAK`, and `peak_usage` returns the absolute `PEAK`); executed: no
- Seen by: scaffolding [1]; refutation: confirmed; history: no-rationale-found (the doc and 48965ff3 both say "transient"; nothing acknowledges the baseline)
- Owner-gated: no

After the body, `peak_usage()` is the larger of the heap live at reset and the in-body peak: libFuzzer's resident corpus and every process-lifetime allocation sit inside the asserted number, so the effective headroom shifts over a long run, and under a proportional cap (finding 14) the baseline would dominate small inputs outright. The doc names the quantity "one input's peak transient heap". Distinguish what is measured from what is claimed.

Evidence:

    28	/// Hard ceiling on one input's peak transient heap: 1 GiB.
    29	pub const PEAK_HEAP_CAP_BYTES: usize = 1 << 30;
    ...
    38	    HEAP.reset_peak_usage();
    39	    let r = body();
    40	    let peak = HEAP.peak_usage();

Resolution: Read `let base = HEAP.current_usage();` before the reset and assert on `HEAP.peak_usage().saturating_sub(base)`, the transient quantity the doc names. Acceptance: the asserted quantity is zero for an empty body regardless of process baseline; the doc sentence and the arithmetic agree.
Construction: Hold 900 MiB before the fuzz loop (or grow the live corpus to that size), then run an input whose body allocates 200 MiB: the cap trips with no amplification present.

### fuzz-guests-pins-16: The guest's contract says every nonzero return is a harness bug; the harness prices `ERR_OP` as an outcome
- Where: crates/before/fuzzfit/guest/src/lib.rs:21-24 (related: crates/before/fuzzfit/guest/src/lib.rs:533-536, crates/before/fuzzfit/guest/src/lib.rs:1354-1358, crates/before/fuzzfit/harness/src/ops.rs:278-287, crates/before/fuzzfit/harness/src/bands.rs:10-16)
- Class / severity / confidence: claim / low / high
- Provenance: verified (ops.rs:285-287 `pub fn rejected(&self) -> bool { self.expect == ERR_OP }`; bands.rs carries `rejected: true` bands for `ff_clock_join`, `ff_clock_sync`, `ff_party_join`, `ff_party_without`, `ff_rank_checked_sub`); executed: no
- Seen by: adequacy [29]; refutation: confirmed; history: deliberate-but-expired (true at the guest's birth 8cfd3c929; f66d7c172 the same day made `ERR_OP` a predicted, separately priced outcome without touching the guest doc)
- Owner-gated: no

The module doc is the contract a maintainer reads before touching a kernel, and it says the rejection arms are unmeasured error paths that abort the case; the harness predicts `ERR_OP` per step and bands it as its own mechanism (kernel by outcome). The per-kernel comments at 533-536 and 1354-1358 repeat the overstatement.

Evidence:

    21	//! - Every export returns `0` for success and a negative code for a misuse
    22	//!   (missing register, wrong type, operation error). The harness treats any
    23	//!   nonzero return as a harness bug and aborts the case: its generators
    24	//!   construct programs that are valid by construction.

Resolution: State the three codes' roles: `ERR_REG` and `ERR_CODEC` are harness bugs (abort the case); `ERR_OP` is a predicted outcome on the kernels with a rejection arm and is priced as a separate band; `ERR_OP` elsewhere is a harness bug. Trim the per-kernel repeats to point at the module doc. Acceptance: the module doc names `ERR_OP` as a priced outcome for exactly the kernels `bands.rs` carries `rejected: true` bands for.
Construction: Textual: compare guest lines 21-24 with ops.rs:285-287 and the five `rejected: true` bands.

### fuzz-guests-pins-17: Moralized qualifiers ("honest", "real") at ten sites
- Where: crates/before/fuzzfit/guest/src/lib.rs:84-87 (related: crates/before/fuzzfit/guest/src/lib.rs:129, crates/before/fuzzfit/guest/src/lib.rs:2025, crates/before/wasm32-pins/guest/src/lib.rs:131, crates/before/wasm32-pins/guest/src/lib.rs:186, crates/before/wasm32-pins/guest/src/lib.rs:565, crates/before/wasm32-pins/guest/src/lib.rs:600, crates/before/wasm32-pins/harness/tests/pins.rs:162, crates/before/wasm32-pins/harness/tests/pins.rs:215, crates/before/wasm32-pins/harness/tests/pins.rs:248)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nE '\bhonest|\breal\b'` over the partition's ten .rs files: "honest" at eight sites, "real" at two; `grep -rn -i '\bhonest'` over crates/before/src returns 146 lines); executed: no
- Seen by: structure-prose [56]; instrument-correctness [71]; refutation: confirmed (eight "honest" sites, not nine); history: contradicts-hard-rule, but crate-wide, so "smallest honest trigger" reads as a crate-wide term of art the owner may keep or rename in one sweep
- Owner-gated: no

The writing-style rule asks for the property where "real", "genuine", or "honest" beckons. Each site has a plain mechanism: "smallest honest trigger" is "the smallest input that reaches the seam" (the depth is counted from bits read, never a header claim); "keep the file honest" is "leave the register file well-typed"; "`-D warnings` honest" is "`-D warnings` clean"; "the real backend" is "the production backend"; "the real kernels" is "the measured kernels".

Evidence:

    84	// clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
    85	// fallback-TLS lowering (illumos among the gate's targets) and denies
    86	// initializers that already sit in `const` blocks; the allow keeps
    87	// `-D warnings` honest on every platform the gate runs.
    (wasm32 guest:131) /// smallest honest trigger for any `exp`-boundary behavior: no crafted

Resolution: Batch with the crate-wide prose pass; substitute the property at each site. Acceptance: the grep returns nothing, crate-wide.

### fuzz-guests-pins-18: The fuzz-fit guest hand-expands its register-file accessors, split borrows, and verdict tables, and the copies already drift
- Where: crates/before/fuzzfit/guest/src/lib.rs:123-245 (related: crates/before/fuzzfit/guest/src/lib.rs:369-378, crates/before/fuzzfit/guest/src/lib.rs:440-450, crates/before/fuzzfit/guest/src/lib.rs:478-524, crates/before/fuzzfit/guest/src/lib.rs:582-587, crates/before/fuzzfit/guest/src/lib.rs:685-706, crates/before/fuzzfit/guest/src/lib.rs:934-960, crates/before/fuzzfit/guest/src/lib.rs:1238-1281, crates/before/fuzzfit/guest/src/lib.rs:1288-1291, crates/before/fuzzfit/guest/src/lib.rs:1305-1308, crates/before/fuzzfit/guest/src/lib.rs:1497-1507, crates/before/fuzzfit/guest/src/lib.rs:1758-1768, crates/before/fuzzfit/harness/src/ops.rs:640-652, crates/before/fuzzfit/harness/src/ops.rs:910-920)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `split_at_mut` four times; `Some(Some(Val::C(c)))` at 234 and 242 in the helpers and re-inlined at 371, 442, 1289, 1306, while `with_c` is called once, at 776; `Placement::After => 8` twice; the FNV offset and prime twice each at 686/701 and 689/706; the sign mask four times at 623, 691, 738, 1086; `hit as i32` at 1560 and 1830 against `i32::from` elsewhere); executed: no
- Seen by: scaffolding [14]; adequacy [35]; structure-prose [40],[41]; refutation: confirmed, reframed to legibility and maintenance (a divergent verdict copy fails loudly against the harness's native `expect`, ops.rs:644-649, so no masked differential); history: no-rationale-found (`with_c` arrived with the shape walk in 46eb64f9 and the four older clock kernels were never migrated)
- Owner-gated: no

Four `take_*` and eight `with_*` accessors are one pattern per slot type; the two-index `split_at_mut` borrow is copied four times; the `Option<Ordering>`, `Ordering`, `Placement`, `Dominance`, and `Precedence` return-code tables are each spelled two or three times; `decimal_digest` is `ShapeDigest` over a string's bytes with the same two literal constants (and the harness holds a third copy it must keep in step); and `ff_clock_encode`, `ff_clock_display`, `ff_clock_own_version`, and `ff_clock_version` re-inline the match `with_c` performs. Every new kernel is written by copying a neighbor, and the `with_c` bypasses show the copies already disagree on which spelling is current. Legibility of a 2000-line instrument.

Evidence:

    369	pub extern "C" fn ff_clock_encode(src: u32) -> i32 {
    370	    REGS.with_borrow(|regs| match regs.get(src as usize) {
    371	        Some(Some(Val::C(c))) => {
    ...
    685	fn decimal_digest(text: &str) -> i64 {
    686	    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    ...
    700	    fn new() -> Self {
    701	        ShapeDigest(0xcbf2_9ce4_8422_2325)

Resolution: A `Slot` trait (`from_val`, `as_ref`, `as_mut`) implemented for the six slot types gives one generic `take::<T>`, `with::<T>`, `with_mut::<T>`, `take_range::<T>`, and `with_two_mut::<A, B>`; name the return-code tables as functions (`ordering_code`, `placement_code`, `dominance_code`, `precedence_code`) with the encoding in their doc; delete `decimal_digest` in favor of `ShapeDigest` fed the text's bytes, and name `FNV_OFFSET`, `FNV_PRIME`, `NONNEGATIVE_MASK`; route the four clock kernels through `with_c`. Acceptance: `split_at_mut` appears once; `Some(Some(Val::C(c)))` appears only in the helpers; each verdict encoding is spelled once; `cbf2_9ce4` appears once in the guest; `just fuzzfit` bands unchanged, since nothing measured changes.

### fuzz-guests-pins-19: `ff_reset` has no caller
- Where: crates/before/fuzzfit/guest/src/lib.rs:261-267 (related: crates/before/fuzzfit/harness/src/wasm.rs:1-11)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn ff_reset crates/` hits only the definition; wasm.rs:4-6 states a fresh `Guest` per program as the determinism model); executed: no
- Seen by: scaffolding [15]; refutation: confirmed; history: no-rationale-found (born with the guest in 8cfd3c929, never named by any consumer in any revision)
- Owner-gated: no

The harness's determinism model is a fresh guest per program, which makes an in-place reset redundant; an export nothing drives exists only for itself and suggests a reuse mode the harness deliberately does not have.

Evidence:

    261	/// Clear the register file and the staging buffer.
    262	#[no_mangle]
    263	pub extern "C" fn ff_reset() -> i32 {
    264	    REGS.with_borrow_mut(Vec::clear);
    265	    STAGE.with_borrow_mut(Vec::clear);
    266	    OK
    267	}

Resolution: Delete it. Acceptance: `grep -rn ff_reset crates` returns nothing.

### fuzz-guests-pins-20: `ff_regs_reserve`'s doc narrates an incident and cites a seed path that does not exist
- Where: crates/before/fuzzfit/guest/src/lib.rs:272-279 (related: crates/before/fuzzfit/harness/tests/main.rs:1-7, crates/before/fuzzfit/harness/proptest-regressions/enforce.txt, crates/before/fuzzfit/harness/src/wasm.rs:28-47)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`ls crates/before/fuzzfit/harness/tests` shows enforce.rs, main.rs, sanity.rs; `find crates/before/fuzzfit -name '*proptest-regressions*'` returns only `harness/proptest-regressions`, holding enforce.txt); executed: no
- Seen by: scaffolding [16]; adequacy [28]; structure-prose [43]; instrument-correctness [67]; refutation: confirmed; history: deliberate-but-expired (ce3664dd9 centralized seed persistence and did not update this doc)
- Owner-gated: no

The doc tells the story of a false above-band flag and points at `harness/tests/enforce.proptest-regressions`; the committed seed lives at `harness/proptest-regressions/enforce.txt`. Prose speaks in the present tense: the invariant (no reallocation inside a measured window; at most one fresh slot per `put`) is the useful sentence, the story is git's, and the path is a ghost reference.

Evidence:

    272	/// Without the reservation, a measured kernel whose `put` lands on a `Vec`
    273	/// doubling boundary pays an O(file) reallocation inside its fuel window —
    274	/// register-machine bookkeeping billed to a public operation. The
    275	/// enforcement suite caught exactly that as a false above-band flag on
    276	/// `ff_party_seed` (the committed seed in
    277	/// `harness/tests/enforce.proptest-regressions` replays it); with the file
    278	/// pre-reserved to the program budget, `put` fills at most one fresh slot
    279	/// per call, O(1) forever.

Resolution: Keep the mechanism sentences (272-274 and 277-279 without the parenthetical); delete the "caught exactly that" sentence and the path; if a pointer is wanted, cite the harness constant that enforces the reserve (`REGS_RESERVE` and its const assert in `harness/src/wasm.rs`). Acceptance: the doc names no path and no past event.

### fuzz-guests-pins-21: "mint" for constructing values at five sites
- Where: crates/before/fuzzfit/guest/src/lib.rs:454-456 (related: crates/before/fuzzfit/guest/src/lib.rs:881, crates/before/fuzzfit/guest/src/lib.rs:899, crates/before/fuzzfit/guest/src/lib.rs:1592, crates/before/fuzzfit/guest/src/lib.rs:1657)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -niE '\bmint'` over the partition's ten .rs files hits only the fuzz-fit guest at 454, 455, 881, 899, 1592, 1657); executed: no
- Seen by: structure-prose [42]; instrument-correctness [71]; refutation: confirmed; history: contradicts-hard-rule (writing-style: never write "mint" for constructing a value)
- Owner-gated: no

The one vocabulary rule the brief states as an outright "never". At 881, 899, 1592, and 1657 the intended content is "the operands are borrowed and the endpoints are freshly allocated (owned)", which the plain words say better.

Evidence:

    454	/// The text door mints the clock's party from the literal; the atlas
    455	/// only replays text a staged clock rendered, so no minted party ever
    456	/// meets a live handle.

Resolution: 454-456: "The text door constructs the clock's party from the literal; ... so no such party ever meets a live handle." 881, 899, 1592, 1657: "the operands are read in place and the endpoints are freshly allocated (owned)". Acceptance: `grep -in '\bmint' crates/before/fuzzfit/guest/src/lib.rs` returns nothing.

### fuzz-guests-pins-22: `COMBINE_ARITY_CAP` and the `dispatch!` arm list are hand-parallel
- Where: crates/before/fuzzfit/guest/src/lib.rs:791-831
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read: the cap is 16 at 791, the guard `n > COMBINE_ARITY_CAP` returns -1 at 808, the list at 831 enumerates 0..=16, and the fallthrough at 827 is `unreachable!`); executed: no
- Seen by: scaffolding [19]; structure-prose [45]; refutation: confirmed; history: no-rationale-found (both spellings landed in 46eb64f9)
- Owner-gated: no

Raising the cap without extending the list reaches `unreachable!("arity is capped above")` at runtime rather than failing to compile. No hand-maintained counts: a number that matters lives in one mechanically enforced place.

Evidence:

    791	const COMBINE_ARITY_CAP: u32 = 16;
    ...
    827	                    _ => unreachable!("arity is capped above"),
    ...
    831	        dispatch!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)

Resolution: Derive one from the other: generate the arms from the cap with `seq_macro::seq!(N in 0..=16 { ... })` (a dependency over hand-rolling), or define the cap as the list's last literal inside the macro, or tie them with a `const _: () = assert!(...)`. Acceptance: changing one spelling without the other is a compile error.

### fuzz-guests-pins-23: `ff_party_forks`'s doc says the kernel "replaces `src`"; it mutates `src` in place
- Where: crates/before/fuzzfit/guest/src/lib.rs:987-988 (related: crates/before/fuzzfit/guest/src/lib.rs:991, crates/before/fuzzfit/guest/src/lib.rs:1122-1123)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read the doc against the body `with_p_mut(src, |p| p.forks(u64::from(n)).collect::<Vec<_>>())`); executed: no
- Seen by: structure-prose [44]; refutation: confirmed; history: no-rationale-found (unchanged since 8cfd3c929; the phrase plausibly means "the party in `src` is replaced in place by its remainder", so this may be phrasing rather than contradiction)
- Owner-gated: no

The parenthetical reads as self-contradictory (replaces, yet borrows and keeps); the sibling `ff_clock_forks` doc states the mechanism plainly and sends readers here for the discipline.

Evidence:

    987	/// `Party::forks(n)`: balanced shares into `dst..dst + n` (replaces `src`:
    988	/// the iterator borrows the source, which keeps its remainder).

Resolution: "(the source in `src` keeps its remainder share; the iterator borrows it)". Acceptance: the two `*_forks` docs describe the same mechanism in the same words.

### fuzz-guests-pins-24: The validation index has no row for the fuzz targets, the heap cap, or the wasm32 pins
- Where: crates/before/src/testing/validation_index.rs:1-3 (related: crates/before/src/testing/validation_index.rs:55, crates/before/src/testing/validation_index.rs:121, crates/before/src/testing/validation_index.rs:169, justfile:464-471)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'fuzz|wasm32|32-bit|heap cap|libfuzzer'` over the index hits only line 55 (laws "shared with the fuzz targets"), 121 (the fuzz-fit bands row), and 169 ("fuzz seeds" under the wire-format pins)); executed: no
- Seen by: scaffolding [11]; refutation: confirmed; history: no-rationale-found (the index was created after the fuzz targets and never gained a row; wasm32-pins landed three weeks later)
- Owner-gated: no

The index claims totality ("every instrument that guards this crate"), yet the five libFuzzer targets (hostile-byte inputs no generator produces; the heap cap as their resource side) and the wasm32 execution leg (seams that exist only under a 32-bit `usize`, which `wasm-check` compiles but never runs) have no row, although both run in the gate's stream roster.

Evidence:

    1	//! The validation index: every instrument that guards this crate, what
    2	//! failure class each one catches that the others cannot, and where it
    3	//! lives.

Resolution: Add two rows, each with its failure class as a constructible input and its recipe: the fuzz targets and their heap cap; the wasm32 pins. Acceptance: every leg in the gate's stream list (justfile:464-471) has a row naming what it alone catches.

### fuzz-guests-pins-25: wasm32-pins lacks the target-dir redirect its sibling workspaces carry against wasmtime-generated sources under `crates/`
- Where: crates/before/wasm32-pins/Cargo.toml:1-5 (related: crates/before/fuzzfit/.cargo/config.toml:1-7, crates/before-fuelscape/.cargo/config.toml:1-7, tools/doclint:370-373, justfile:179-181, justfile:629-631)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (the two sibling configs exist with the stated rationale; `crates/before/wasm32-pins/.cargo/config.toml` does not; the harness half builds with no `--target-dir` (justfile:631) into `crates/before/wasm32-pins/target`, present on disk; `doclint` walks `root.rglob("*.rs")` under `crates` with no exclusion). Unverified by anyone: that wasmtime's build scripts emit rustdoc'd `.rs` files (the claim rests on the sibling config's rationale); on this machine a global `build.build-dir` diverts intermediates, so the hazard is masked here, and CI never builds this workspace; executed: no
- Seen by: scaffolding [8]; refutation: confirmed; history: no-rationale-found (built "in the fuzzfit idiom" without the idiom's redirect)
- Owner-gated: no

Machinery justified once (the fuzzfit redirect) was not carried to the workspace built in its idiom; a contributor without a global build-dir override gets a red `doclint` after `just wasm32-pins`, attributed to nothing they wrote.

Evidence:

    (crates/before/fuzzfit/.cargo/config.toml:1-7)
    # Build into the repo root's target/ (its own subdirectory) instead of a
    # workspace-local target dir: the gate's doclint sweeps every .rs under
    # crates/, and wasmtime's build scripts generate rustdoc'd sources that
    # would otherwise land inside this tree and fail it. The repo root target/
    # is outside every gate sweep.
    [build]
    target-dir = "../../../target/fuzzfit"
    (crates/before/wasm32-pins/Cargo.toml:1-5)
    # Standalone 32-bit boundary-pin workspace. The empty `[workspace]` table
    # detaches it from the parent `rumors` workspace, so the ordinary gate never
    # builds it (the harness carries wasmtime, a heavy tool-side dependency that
    # must stay out of the production crates' graph; the guest builds only for
    # wasm32-unknown-unknown). Build/run through the `just wasm32-pins*` recipes.

Resolution: Add `crates/before/wasm32-pins/.cargo/config.toml` with `[build] target-dir = "../../../target/wasm32-pins"` (matching `wasm32pins_target`) and the same rationale; the harness fallback path (finding 32) then has one definition to agree with. Acceptance: on a host with no `~/.cargo/config.toml` build-dir override, `just wasm32-pins && just doclint` is green and `find crates/before/wasm32-pins -name '*.rs' -path '*/target/*'` is empty.
Construction: On a host without a build-dir override, `just wasm32-pins-build` then `./tools/doclint crates`: wasmtime's OUT_DIR sources under `crates/before/wasm32-pins/target` are linted.

### fuzz-guests-pins-26: The overflow-checks premise has no liveness self-test
- Where: crates/before/wasm32-pins/Cargo.toml:26-32 (related: crates/before/wasm32-pins/guest/src/lib.rs:21-24, crates/before/wasm32-pins/harness/tests/pins.rs:84-90, crates/before/fuzzfit/guest/src/lib.rs:2016-2037, crates/before/fuzzfit/harness/tests/enforce.rs:260-310)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read the profile, the guest's module doc, and every pin's expected value; `grep -n 'selftest|set_hook|PANICKED'` over the pins guest is empty while the fuzz-fit guest ships `ff_selftest_quadratic`); executed: no
- Seen by: adequacy [23]; refutation: confirmed (not run); history: no-rationale-found
- Owner-gated: no

The whole "a 32-bit wrap is an observable trap, never a silently wrong value" argument rests on `overflow-checks = true` in the guest's release profile, and nothing observes that the built guest has it: `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false` in the environment, a `[profile.release]` in a config file, or a future manifest edit silently restores wrapping, and no pin's expected value depends on a wrapped intermediate trapping (pins.rs:84-90 notes the 2^29-byte wrap is coincidentally correct), so every pin stays green. The fuzz-fit sibling carries the corresponding instrument self-test for its fuel path. Meters need liveness proof.

Evidence:

    26	# The pins run at release speed (the boundary cases walk hundreds of
    27	# megabytes), but with overflow checks kept ON: the 32-bit failure class
    28	# under audit includes silent release-mode wraps, and the checks turn
    29	# exactly those wraps into observable traps instead of wrong values the
    30	# guest would have to detect after the fact.
    31	[profile.release]
    32	overflow-checks = true

Resolution: Add `pin_selftest_overflow() -> i64` to the guest that computes `black_box(u32::MAX) + black_box(1u32)` (and a `usize` variant) and a harness test asserting `Outcome::Trapped(Trap::UnreachableCodeReached)` (with the panic-genre discriminator from finding 35, asserting the panic genre). Name it beside `version_small_roundtrips_and_rejects_typed` as the leg's second liveness pin. Acceptance: `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false just wasm32-pins` turns the self-test red; the default build keeps it green.
Construction: Run `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false just wasm32-pins`: every existing pin passes, because no pin's observation is a wrapped quantity and no export exercises a wrap on purpose.

### fuzz-guests-pins-27: `synth_version` does 32-bit `usize` arithmetic that wraps seven bytes above the terminal pin, and synthesis failures share the trap channel
- Where: crates/before/wasm32-pins/guest/src/lib.rs:47-58 (related: crates/before/wasm32-pins/guest/src/lib.rs:13-15, crates/before/wasm32-pins/guest/src/lib.rs:133-175, crates/before/wasm32-pins/guest/src/lib.rs:213-216, crates/before/wasm32-pins/guest/src/lib.rs:326-332, crates/before/wasm32-pins/harness/tests/pins.rs:150)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (arithmetic from the quoted line: `4 * n` for `n = 2^30 = 1_073_741_824` is 2^32, which does not fit a 32-bit `usize`; the pinned terminal is `1_073_741_817`, seven bytes under; every other synthesizer computes positions in `u64` and converts at indexing, and `synth_ranked:327` computes `4 * n as u64 - 5` in `u64` before calling `synth_version(n)`); executed: no
- Seen by: structure-prose [39]; instrument-correctness [62]; refutation: confirmed (seven bytes, not six); history: no-rationale-found (written in `usize` when the largest pinned size was 512 MiB; c75d5022 pushed the terminal to within 28 bits of the wrap while writing its own new synthesizers in `u64`)
- Owner-gated: no

With overflow checks on, `pin_version_decode(n)` for `n >= 2^30` traps inside the synthesizer, before `Version::decode` runs, and that trap is byte-identical to the one `version_decode_memory_terminal_traps` pins for a decode-side terminal. The guest's contract (lines 13-15) promises a negative code for the first failed observation; synthesis is exempt (its `expect("the stream is addressable")` and `vec!` allocation failures also trap), so the one outcome class the suite pins red is the one it cannot attribute. Correct at all scales applies to the instrument too, and the wrap is the very genre it audits, occurring in the instrument.

Evidence:

    47	fn synth_version(n: usize) -> Vec<u8> {
    48	    assert!(
    49	        n >= 18,
    50	        "the single-wide-leaf layout needs k = 4n - 5 >= 64"
    51	    );
    52	    let k = 4 * n - 5;
    53	    let mut bytes = vec![0u8; n];
    54	    bytes[0] |= 0x80; // the leaf flag
    55	    bytes[(k + 1) / 8] |= 0x80 >> ((k + 1) % 8); // the mantissa's leading 1
    56	    bytes[n - 1] |= 0x80; // the padding marker

Resolution: Take `n_bytes: u64`, compute `k` in `u64`, place the bits with the existing `set_bit` helper (lines 54-56 and `synth_rank:172` hand-roll the `0x80 >> (pos % 8)` it names; move `set_bit` and `fill_ones` above their first use), and convert to `usize` only for the allocation via `usize::try_from`. Make synthesis fallible in-band: `Vec::try_reserve_exact` and `checked_mul`/`checked_add` returning distinct negative codes, so a trap is a `before` trap by construction and the pins' "the probe backtrace attributes..." sentences become unnecessary. Acceptance: `call1("pin_version_decode", 1 << 30)` returns a negative synthesis code, never a synthesizer trap; `grep -c '0x80 >>' guest/src/lib.rs` is 1.
Construction: `call1("pin_version_decode", 1_073_741_824)`: `4 * n` overflows at line 52 under `overflow-checks = true`; the harness reports `Trapped(UnreachableCodeReached)`, indistinguishable from the pinned terminal.

### fuzz-guests-pins-28: Repeated prologue and epilogue fragments in the wasm32 guest; a slice-dispatch `unreachable!` in the harness
- Where: crates/before/wasm32-pins/guest/src/lib.rs:102-106 (related: crates/before/wasm32-pins/guest/src/lib.rs:342-346, crates/before/wasm32-pins/guest/src/lib.rs:365-369, crates/before/wasm32-pins/guest/src/lib.rs:394-398, crates/before/wasm32-pins/guest/src/lib.rs:427-431, crates/before/wasm32-pins/guest/src/lib.rs:459-463, crates/before/wasm32-pins/guest/src/lib.rs:578-587, crates/before/wasm32-pins/guest/src/lib.rs:610-619, crates/before/wasm32-pins/harness/src/lib.rs:66-106)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grep: six `usize::try_from(n_bytes)` prologues at 103, 343, 366, 395, 428, 460; three `r != r.clone()` epilogues at 206, 585, 617); executed: no
- Seen by: scaffolding [13]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Six exports repeat the `usize::try_from(n_bytes)` to `-100` prologue verbatim and three repeat the rank epilogue; in the harness, `call0`/`call1`/`call2` funnel into a slice match ending in `unreachable!`, where a single `fn call<P: wasmtime::WasmParams>(export, params: P)` typed on wasmtime's trait removes the runtime branch. Repeated fragments hide the one line per export that differs.

Evidence:

    103	    let n = match usize::try_from(n_bytes) {
    104	        Ok(n) => n,
    105	        Err(_) => return -100,
    106	    };
    (harness/src/lib.rs:105)         _ => unreachable!("pin exports take at most two arguments"),

Resolution: `fn addressable(n_bytes: u64) -> Result<usize, i64>` and `fn rank_observations(r: &Rank) -> i64` in the guest (the latter revised per finding 29); a generic `call<P: WasmParams>` in the harness. Acceptance: each export body is its synthesis plus the checks unique to it; no `unreachable!` in the harness driver.

### fuzz-guests-pins-29: The rank pins observe only `0 < r < 1` and `r == r.clone()`, which a decoder that drops the seam bit satisfies
- Where: crates/before/wasm32-pins/guest/src/lib.rs:189-210 (related: crates/before/wasm32-pins/guest/src/lib.rs:118-175, crates/before/wasm32-pins/guest/src/lib.rs:569-589, crates/before/wasm32-pins/guest/src/lib.rs:603-621, crates/before/wasm32-pins/guest/src/lib.rs:671-690, crates/before/wasm32-pins/guest/src/lib.rs:700-725, crates/before/wasm32-pins/harness/tests/pins.rs:155-302, crates/before/wasm32-pins/harness/tests/pins.rs:633-713, crates/before/src/version/rank.rs:244)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read `synth_rank`: it sets expansion bits 65 and `exp`, so the value is 2^-65 + 2^-exp; read every rank pin's checks; `Rank` derives `Clone, PartialEq, Eq` at rank.rs:244); executed: no
- Seen by: adequacy [22]; refutation: confirmed with a nuance carried here (the "never zeros" clause at pins.rs:180-181 is checked by `r > ZERO`; the overstated clauses are "orders exactly against reference ranks" at pins.rs:174, 205, 226, 244 and the "exact" arithmetic; deleting `r != r.clone()` would drop execution coverage of `Clone`/`Eq` on the limb arm, a trap channel, so replace rather than delete); history: no-rationale-found (the leg's original design, copied into later rank pins)
- Owner-gated: no

`pin_rank_decode`, `pin_rank_integral_decode`, `pin_version_rank`, `pin_rank_add`, and `pin_rank_checked_sub` observe the result only by coarse order against `Rank::ZERO` or the rank of the version `1`, and by `r != r.clone()` (a copy compared to itself, which no arm can fail). A decoder that drops the bit at expansion position `exp` (the seam bit the pins target) still yields a value strictly between zero and one; an adder that loses the 2^-(2^32) term still exceeds both summands. The natural known-bad mechanism passes every rank pin except `pin_rank_roundtrip`, which runs at one exponent. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

    196	    if r <= Rank::ZERO {
    197	        return -2;
    198	    }
    199	    let one = match Version::try_from(1) {
    200	        Ok(v) => v.rank(),
    201	        Err(_) => return -3,
    202	    };
    203	    if r >= one {
    204	        return -4;
    205	    }
    206	    if r != r.clone() {
    207	        return -5;
    208	    }
    (pins.rs:174) /// correctly on wasm32 and orders exactly against reference ranks.

Resolution: Give each rank pin a witness that sees the seam bit inside the memory budget: decode a second synthesized stream identical except for the deep bit (drop the first stream's bytes first) and assert strict order between the two; for `pin_rank_add` and `pin_rank_checked_sub` assert the algebraic inverse (`sum.checked_sub(&small) == Some(big)`, `diff + small == minuend`) or compare `sum.encode()` against a synthesized expected stream; for `pin_version_rank` bracket with tight bounds `rank(leaf(h)) < r < rank(leaf(h + d))` rather than `rank(1)`. Replace `r != r.clone()` with a check that exercises `Clone` and `Eq` on the limb arm and can fail (compare the clone's `encode()` to the original's). Acceptance: a guest whose `synth_rank` clears the bit at `exp` reads red on every rank decode pin; a `pin_rank_add` whose result lacks the 2^-(2^32) term reads red; the committed guest stays green within the 4 GiB budget.
Construction: In `synth_rank`, change `for e in [65, exp]` to `for e in [65]` (standing in for a decoder that drops the seam bit) and run `just wasm32-pins`: `rank_decode_below_backend_capacity`, `rank_decode_at_backend_byte_capacity`, `rank_decode_at_usize_exp_boundary`, `rank_decode_at_backend_bit_capacity`, `rank_decode_past_backend_bit_capacity`, the `rank_add_*` pins, and the `rank_checked_sub_*` pins all still return `Value(0)`; only `rank_roundtrip_past_backend_bit_capacity` notices.

### fuzz-guests-pins-30: `synth_rank_ladder`'s layout check is a `debug_assert` compiled out of the only profile built
- Where: crates/before/wasm32-pins/guest/src/lib.rs:552-552 (related: justfile:629-631, crates/before/wasm32-pins/Cargo.toml:31-32)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (justfile:630 builds the guest with `--release` only; the profile sets only `overflow-checks = true`, not `debug-assertions`); executed: no
- Seen by: structure-prose [58] (which cited line 626; the site is 552); instrument-correctness [68]; refutation: confirmed at 552; history: no-rationale-found
- Owner-gated: no

A guard compiled out of every build that exists catches nothing; the check is O(1) beside a megabyte fill, and the fold pin at `(2_684_354_496, 64)` rests on the ladder being laid out as documented.

Evidence:

    552	    debug_assert_eq!(p, live);

Resolution: `assert_eq!(p, live, "the ladder's live length matches its layout")`, or a negative code per finding 27. Acceptance: perturbing `p += 5` to `p += 4` makes `pin_version_rank` fail at the synthesizer rather than at the decode.

### fuzz-guests-pins-31: The wasm32 guest's failure codes are bare negative literals, and the harness says the guest's docs key them
- Where: crates/before/wasm32-pins/harness/src/lib.rs:29-30 (related: crates/before/wasm32-pins/guest/src/lib.rs:66-116, crates/before/wasm32-pins/guest/src/lib.rs:189-210, crates/before/wasm32-pins/guest/src/lib.rs:648-660, crates/before/fuzzfit/guest/src/lib.rs:98-105)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (no guest doc enumerates a code; 54 `return -N` sites by grep; the same code means different checks per export, so -2 is "bytes differ" at 112-113 and "r <= ZERO" at 196-197; the fuzz-fit guest names `OK`, `ERR_REG`, `ERR_OP`, `ERR_CODEC`); executed: no
- Seen by: scaffolding [12]; refutation: confirmed; history: no-rationale-found (false from the leg's first commit)
- Owner-gated: no

A failing pin reports `Value(-3)` and the reader must open the guest source to learn which check failed; the harness doc's claim is false as written. Named constants over magic numbers.

Evidence:

    29	    /// The export returned: nonnegative is its observation, negative names
    30	    /// the first failed in-guest check (the guest's doc comments key them).

Resolution: Introduce named codes in the guest (`DECODE_REJECTED`, `BYTES_DIFFER`, `LENGTH_UNADDRESSABLE`, and so on) shared across exports, and either re-export them for the pins to assert on or correct the harness doc to say the guest's code names key them. Acceptance: `grep -E 'return -[0-9]+' crates/before/wasm32-pins/guest/src/lib.rs` is empty; the harness doc sentence is true.

### fuzz-guests-pins-32: The harness's fallback guest path resolves one directory above the repository
- Where: crates/before/wasm32-pins/harness/src/lib.rs:45-47 (related: crates/before/wasm32-pins/harness/src/lib.rs:38-40, crates/before/fuzzfit/harness/src/wasm.rs:99-108, justfile:60-67, justfile:77-78, justfile:644)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (`os.path.normpath` of the manifest dir joined with five `..` yields `/Users/oxide/src/target/wasm32-pins/...`, which does not exist; four `..` yields `/Users/oxide/src/rumors/target/wasm32-pins/...`, which the recipe builds into; the fuzzfit twin at the same depth uses four); executed: no
- Seen by: scaffolding [6]; adequacy [30]; structure-prose [36]; instrument-correctness [64]; refutation: confirmed, severity low (the recipe always sets `WASM32_PINS_GUEST_WASM`, so no gate or CI path reaches the fallback); history: no-rationale-found (written in eb6ba627 together with the recipe's target dir; never correct)
- Owner-gated: no

The doc at 38-40 says the fallback is "the workspace-relative target dir the recipe builds into"; it is not. Anyone running `cargo nextest run` in the workspace by hand gets a "not loadable" panic naming a path nothing writes, the exact failure the justfile's own comment at 60-67 warns about. The fuzzfit twin also honors `CARGO_TARGET_DIR` (wasm.rs:103-105), an arm this harness lacks. Correct for all inputs: the unset-variable path is an input.

Evidence:

    45	    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
    46	        "../../../../../target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm",
    47	    )

Resolution: Either drop the fallback and require `WASM32_PINS_GUEST_WASM`, panicking with a message that names the variable and `just wasm32-pins-build`; or fix it to four `..` and mirror the fuzzfit precedence (`CARGO_TARGET_DIR` first), with finding 25's `.cargo/config.toml` as the one definition it mirrors. Acceptance: with the variable unset after `just wasm32-pins-build`, `cargo nextest run --cargo-profile release` in `crates/before/wasm32-pins` loads the guest (or fails naming the variable).
Construction: Unset `WASM32_PINS_GUEST_WASM`, build the guest, run the harness tests: every pin panics with `wasm32-pins guest not loadable from /Users/oxide/src/target/...`.

### fuzz-guests-pins-33: Three "PINNED AS FOUND" trap pins are labeled as red baselines awaiting a cure while their docs describe the terminal as intended
- Where: crates/before/wasm32-pins/harness/tests/pins.rs:4-10 (related: crates/before/wasm32-pins/harness/tests/pins.rs:130-153, crates/before/wasm32-pins/harness/tests/pins.rs:359-379, crates/before/wasm32-pins/harness/tests/pins.rs:715-742, crates/before/wasm32-pins/harness/src/lib.rs:23-26, crates/before/tests/meter.rs:1-11)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read the header against the three pins; `git show -s c75d5022` uses the same framing, "the terminal is pinned as found ... A leaner working set — not a wider denomination — is what would move these terminals outward"; `grep -rl 'wasm32-pins' .agent-notes` is empty, so no ruling exists); executed: no
- Seen by: scaffolding [10]; adequacy [21]; structure-prose [38]; instrument-correctness [63]; refutation: confirmed, and raised harness/src/lib.rs:23-26 as a fourth site carrying the same "until its cure lands" contract; history: no-rationale-found (the label is deliberate per c75d5022, which in the same breath declines a cure; the model-versus-defect ruling does not exist)
- Owner-gated: yes: a design decision the owner has not ruled on

The header contracts that a pinned trap "is never an accepted behavior: it is a committed bad baseline its cure must move". `version_decode_memory_terminal_traps`, `ranked_decode_memory_terminal_traps`, and `version_rank_memory_terminal_traps` pin allocation failure inside the 4 GiB address space at about 1 GiB of input, and each doc argues the trap is "the doors' one terminal", that the backend capacity "is unreachable through the doors on this target", and names only a hypothetical "leaner working set" with no cure tracked anywhere. No seam is pinned; what is pinned is a memory constant factor (the docs enumerate the working set qualitatively; no measured multiple exists), a quantity the envelope suite in `tests/meter.rs` owns. Doctrine: no mechanism for accepting known failures may exist; every contradiction resolves to a fix or a model the owner declares, stated positively at the declaration site. These three are in neither genre cleanly, and a fourth `Trapped(...)` assertion added tomorrow for a real defect would be indistinguishable in kind.

Evidence:

    4	//! Red-first discipline: a boundary found misbehaving is pinned AS FOUND —
    5	//! the assertion names the trap or wrong value, and the doc comment names
    6	//! the wrong behavior it stands for — and the commit that engineers the
    7	//! seam around flips the same test to the correct-value assertion. A pinned
    8	//! trap is therefore never an accepted behavior: it is a committed bad
    9	//! baseline its cure must move. Each pin's own history of red and green
    10	//! lives in this file's git log.
    130	/// PINNED AS FOUND: a valid ~1 GiB (1073741817-byte) version encoding
    131	/// aborts on allocation failure — the doors' one terminal here.
    ...
    145	/// `rank_decode_past_backend_bit_capacity`.) A leaner working set — not
    146	/// a wider denomination — is what would move this terminal outward.
    (harness/src/lib.rs:24-26)
    /// surfaces as the `unreachable` trap under `panic = abort`, and a boundary
    /// found panicking is pinned as exactly that trap until its cure lands, so
    /// the pins assert on this axis directly.

Resolution: Owner rules the genre. If the address-space bound is the model (the evidence reads that way): drop "PINNED AS FOUND" from the three, rename them to state the behavior positively (for example `version_decode_past_address_space_aborts_loudly`), state the working-set multiple they pin, narrow the header and harness/src/lib.rs:23-26 to distinguish red-first seam pins (none today) from declared terminals, and pair the restatement with the origin discriminator from finding 35 so the instrument, not the prose, establishes "allocation failure". If a leaner working set is a planned cure: say so at each pin and track it as open work with its target multiple, and consider whether a heap envelope in `tests/meter.rs` on decode's working set is the right home for the constant factor, leaving the wasm32 leg to seams. Acceptance: every trap pin in the file is either a seam with a tracked cure or a positively stated declared model, and the header's contract is true of every pin below it.
Construction: Textual, not runnable: compare lines 4-10 with 130-146, 359-372, and 715-735 in the same file, and harness/src/lib.rs:23-26.

### fuzz-guests-pins-34: `BUILD_CAP_BYTES` and the `*_build_cap` test-name family name a cap two commits removed; three pins spell its coordinate as literals
- Where: crates/before/wasm32-pins/harness/tests/pins.rs:14-25 (related: crates/before/wasm32-pins/harness/tests/pins.rs:44-79, the `*_build_cap` tests at pins.rs:45, 59, 74, 312, 328, 339, 388, 404, 415, 440, 451, 462, 485, 497, 507, 545, 562, 574, 591, 609, crates/before/Cargo.toml:33-36)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git log -S'BUILD_CAP_BYTES'` names 5d167a63 and 83e61b4d; `git show 83e61b4d^:.../pins.rs` lines 14-23 read "one byte under the 32-bit bit-vector length encoding's cap" and "the emitters' build buffer is the one surface still bounded by this encoding"; 83e61b4d, "replace the bitvec build buffer with the crate-owned BitsBuf", rewrote that doc to the hypothetical while keeping the identifier and the test names; `git blame` puts the literals at 47-48, 61-62, 76-77 in eb6ba627, before the constant existed; `grep -c 'fn .*_build_cap'` is 20; crates/before/Cargo.toml:34-35 confirms nothing shipped links bitvec); executed: no
- Seen by: scaffolding [9]; adequacy [34]; structure-prose [37],[57]; instrument-correctness [70]; refutation: confirmed, severity low (a rename with byte-identical assertions; the prose was already re-denominated); history: deliberate-but-expired (83e61b4d)
- Owner-gated: no

No cap exists (the doc's own second paragraph says "Every surface is exact across it"), yet the constant's name and twenty test names say `build_cap`, and the doc speaks in the present tense of "the cap a 32-bit bit-vector length encoding imposes". A reader of `version_decode_at_build_cap` looks for a build cap and finds none, and the constant is named for the last size below the coordinate, so the `*_at_build_cap` tests run at `BUILD_CAP_BYTES + 1`. Separately, the version-decode straddle spells `67_108_863`, `67_108_864`, `67_108_865` and `8 * 67_108_86x - 8` as literals while every later straddle uses the constant. Rider: five asserts (356, 427, 474, 523, 534) carry a trailing `Outcome::Value(0),)` the others do not. Prose speaks in the present tense; named constants over magic numbers.

Evidence:

    14	/// The largest buffer byte count whose whole-buffer bit count stays below
    15	/// 2^29 bits — `usize::MAX >> 3`, the cap a 32-bit bit-vector length
    16	/// encoding imposes — so 67108863 bytes.
    ...
    25	const BUILD_CAP_BYTES: u64 = 67_108_863;
    45	fn version_decode_below_build_cap() {
    46	    assert_eq!(
    47	        call1("pin_version_decode", 67_108_863),
    48	        Outcome::Value(8 * 67_108_863 - 8),

Resolution: Rename the constant for the coordinate it is (for example `STRADDLE_COORDINATE_BYTES` for 67_108_864, so `at` reads as the coordinate itself and `below`/`past` as `- 1`/`+ 1`), rename the test family `*_below_straddle`/`*_at_straddle`/`*_past_straddle`, restate lines 14-16 as a present-tense definition (the byte count at which a buffer's bit count reaches 2^29, where a `usize`-denominated 32-bit bit count would bind), use the constant at 47-48, 61-62, 76-77 with a `fn live_bits(n: u64) -> i64 { 8 * n - 8 }` helper, and drop the five trailing commas. Acceptance: `grep -rn -i 'build_cap\|build cap' crates/before/wasm32-pins` returns nothing; `grep -c '67_108_86' pins.rs` is 1; the pins' assertions are byte-identical before and after.

### fuzz-guests-pins-35: The memory-terminal pins assert a trap genre every guest abort produces
- Where: crates/before/wasm32-pins/harness/tests/pins.rs:147-153 (related: crates/before/wasm32-pins/harness/tests/pins.rs:373-379, crates/before/wasm32-pins/harness/tests/pins.rs:736-742, crates/before/wasm32-pins/harness/src/lib.rs:20-34, crates/before/wasm32-pins/guest/src/lib.rs:16-24, crates/before/wasm32-pins/Cargo.toml:26-32, crates/before/src/version/rank/num.rs:6-13)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read: wasmtime-environ 47.0.3 `trap_encoding.rs:138` defines `UnreachableCodeReached = "wasm \`unreachable\` instruction executed"`; on wasm32-unknown-unknown a panic aborts and abort lowers to `unreachable`, and allocation failure routes through `handle_alloc_error` to the same abort; the harness doc itself equates the trap with "a guest panic"; the at-capacity arithmetic for `version_rank_memory_terminal_traps` is (2^32 - 96) + 64 = 2^32 - 32 bits, exactly num.rs:10's backend cap); executed: no
- Seen by: adequacy [20]; instrument-correctness [61]; refutation: confirmed; history: no-rationale-found (c75d5022 attributes each terminal to allocation failure by probe backtraces read by hand; no discriminator was ever added)
- Owner-gated: no

The three terminal pins assert only `Outcome::Trapped(Trap::UnreachableCodeReached)`, but that trap is produced by every abort: allocation failure, an explicit `panic!` or `expect`, an overflow-check trap (the failure class this workspace exists to surface, per Cargo.toml:26-32), a big-integer backend capacity panic (num.rs:13 calls both overruns "loud backend panics"), or a synthesizer failure (finding 27). The only thing tying the trap to allocation failure is a probe backtrace described in prose. `version_rank_memory_terminal_traps` sits exactly at the backend capacity coordinate, so an arm-routing off-by-one that handed the `Base` arm a value it panics on reads green there. The cheapest passing artifact: a pin for "aborts on allocation failure" passes on any panic, so the suite's most sensitive coordinates read green on exactly the class it audits.

Evidence:

    147	#[test]
    148	fn version_decode_memory_terminal_traps() {
    149	    assert_eq!(
    150	        call1("pin_version_decode", 1_073_741_817),
    151	        Outcome::Trapped(Trap::UnreachableCodeReached),
    152	    );
    153	}
    (harness/src/lib.rs:32)     /// The export trapped; `Trap::UnreachableCodeReached` is a guest panic.

Resolution: Record the abort genre before the trap: install a `std::panic::set_hook` at each export's entry that sets a `static PANICKED: AtomicBool` (std's alloc-error path calls `abort` directly and never runs the panic hook, so the flag separates the two), expose it through `pin_panicked() -> i64` or a fixed linear-memory address the harness reads from the store after the trap, and widen `Outcome::Trapped` into panic-versus-abort so the terminal pins assert the allocation-abort genre and every other pin asserts no panic flag. A cheaper second discriminator: read `Memory::size(&store)` after the trap and assert linear memory grew to within a page budget of 4 GiB. Confirm wasmtime's behavior for calling exports in the same store after a trap before choosing. Acceptance: a guest whose `pin_version_decode` panics for `n_bytes >= 1 << 30`, or spells `Bits::len` as a `usize` product, turns all three terminal pins red; the committed guest keeps them green.
Construction: Add `assert!(n < 1_000_000_000);` at the top of `pin_version_decode` and run `just wasm32-pins`: `version_decode_memory_terminal_traps` still observes `Trapped(UnreachableCodeReached)` at 1_073_741_817 and passes. Equivalently, `call1("pin_version_decode", 1 << 30)` traps in the synthesizer's `4 * n - 5` (finding 27) with the same outcome.

### fuzz-guests-pins-36: One join-emit pin's doc gives a different size for the same operand than its sibling
- Where: crates/before/wasm32-pins/harness/tests/pins.rs:616-618 (related: crates/before/wasm32-pins/harness/tests/pins.rs:603, crates/before/wasm32-pins/guest/src/lib.rs:251-262, crates/before/wasm32-pins/guest/src/lib.rs:276-291)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (arithmetic from `synth_two_leaf_left`: live length `4k + 5` is 400_000_005 bits at k = 10^8, about 50 MB; the right operand at j = 2_047_483_647 is `2j + 5` = 4_094_967_299 bits, 512 MB, which is 488 MiB); executed: no
- Seen by: instrument-correctness [69]; refutation: confirmed; history: no-rationale-found (the two docs were written in different commits; "~25 MB" matches only the left leaf's gamma code, not the operand)
- Owner-gated: no

Line 603 says "~50 MB" for `synth_two_leaf_left(100_000_000)`; line 616 says "~25 MB" for the same operand, and "~488 MB" where the unit is MiB. The project holds test doc comments to correctness ("their incorrectness is a bug in the test").

Evidence:

    616	/// A join of two valid operands (~25 MB and ~488 MB) emits an output of
    617	/// 4294967299 live bits — 536870913 finished bytes, one byte past the
    618	/// 2^29-byte coordinate where a 32-bit `usize` runs out of bit positions.

Resolution: "~50 MB and ~512 MB" (or "~48 MiB and ~488 MiB"). Acceptance: the two join-emit docs quote the same size for the same operand in the same unit.

### fuzz-guests-pins-37: `wasm32-pins/Cargo.lock` is outside the supply-chain audit roster
- Where: justfile:345-351 (related: justfile:328-330, crates/before/wasm32-pins/Cargo.lock:952-953, crates/before/fuzzfit/Cargo.lock:1193-1194, crates/before-fuelscape/Cargo.lock:1577-1578, .github/workflows/ci.yml:169-172)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`git ls-files '*Cargo.lock'` lists six lockfiles; the recipe audits five; the comment at 329-330 hand-enumerates "(fuzz, fuzzfit, fuelscape, surfacecheck)"; wasm32-pins locks wasmtime 47.0.3 where fuzzfit and fuelscape lock 47.0.4, so it is a distinct resolution never swept; `git show eb6ba627 --stat` shows the lockfile and a justfile edit with no audit line; ci.yml:171-172 runs `just supply-chain`); executed: no
- Seen by: scaffolding [7]; refutation: confirmed; history: deliberate-but-expired (the roster was total when 4da4779fc landed it; eb6ba627 added the sixth lockfile and edited the justfile without extending the recipe)
- Owner-gated: no

The recipe's own comment claims "every lockfile in the repository"; a hand-maintained enumeration of workspaces rots exactly this way, and CI runs the recipe, so the gap is live. No hand-maintained rosters the code can change without touching the prose.

Evidence:

    345	# Audit advisories on every lockfile and hold the workspace to single crate versions.
    346	supply-chain:
    347	    cargo audit
    348	    cargo audit --file crates/before/fuzz/Cargo.lock
    349	    cargo audit --file crates/before/fuzzfit/Cargo.lock
    350	    cargo audit --file crates/before-fuelscape/Cargo.lock
    351	    cargo audit --file crates/before/surfacecheck/Cargo.lock

Resolution: Add the wasm32-pins lockfile, and derive the list mechanically so the next detached workspace cannot be missed: `git ls-files '*Cargo.lock' | xargs -n1 cargo audit --file` (or a `find` excluding `target/`), and drop the hand enumeration at 329-330. Consider aligning the three wasmtime pins while touching the locks. Acceptance: the recipe audits every committed lockfile without naming them; adding a lockfile to the tree changes nothing in the justfile.
Construction: Pin an advisory-bearing crate version only in `wasm32-pins/Cargo.lock` (or wait for a RustSec advisory covering wasmtime 47.0.3 alone): `just supply-chain` stays green.

### fuzz-guests-pins-38: The fuzz targets' own oracles never execute in the gate or CI
- Where: justfile:358-360 (related: justfile:548-559, justfile:1000-1003, .github/workflows/ci.yml:4-7, crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:164-179, crates/before/tests/support/fuzz_seed_set.rs:155-159)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read: `fuzz-build` compiles only; `ci` (justfile:1000) includes `fuzz-build` and not `fuzz`; only `all` (1003) runs the smoke; `tests/fuzz_seeds.rs` checks seed genres through the public API, never through the targets' `agreed` tables); executed: no
- Seen by: adequacy [25]; refutation: confirmed; history: deliberate-and-holds for the smoke's exclusion (justfile:359-360 and 04b4d1b7 price fuzzing minutes), but a seconds-long deterministic `-runs=0` seed replay was never considered
- Owner-gated: yes: gate policy

The agreement logic that lives in the targets themselves (the `agreed` tables in `span_differential` and `borsh_vs_raw`, the postcard framing arm, the law drive loop, the heap cap) executes only when someone runs the smoke. The seed corpus exists so that "a reintroduced genre-ordering defect ... crashes the very first smoke run" (fuzz_seed_set.rs:157-159), but that first run is at sweep cadence, and an inverted predicate or wrong allowance in a target is invisible to every gate and CI run. Could the instrument go dark while reading green: compiled but never executed.

Evidence:

    358	# `before` breaks a fuzz target in the same commit that lands it, and a
    359	# compile is seconds of gate time. Only the build: the libFuzzer smoke is
    360	# poor per-commit spend and runs at `just all` cadence.

Resolution: Add a gate leg beside `fuzz-build` that replays the committed seeds through each built target once: `cargo +nightly fuzz run --target <host> <target> seeds/<target> -- -runs=0` executes every corpus input and exits; deterministic, seconds, and it turns the seed corpus from a smoke-time asset into a per-commit oracle liveness check. Acceptance: inverting `(_, c, f) if c == f => true` in `span_differential` reddens the new leg on the committed `span_ordered` and `span_crossed` seeds; the committed targets pass it.
Construction: Flip `(_, c, f) if c == f => true` to `c != f` at fuzz_decode_differential.rs:168 and run `just gate`: `fuzz-build` compiles it green, `tests/fuzz_seeds.rs` is untouched, and nothing reddens until `just fuzz`.

### fuzz-guests-pins-39: `fuzz-build` lints the detached fuzz workspace with fmt only, unlike its siblings
- Where: justfile:365-368 (related: justfile:588-597, justfile:633-644)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read the three recipes: fuzzfit and wasm32-pins run `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` with the rationale at 636-639; fuzz-build runs fmt only). Not verified: that stable clippy compiles the fuzz workspace (no cargo command was permitted; libfuzzer-sys builds on stable via `cc`, and `cargo fmt --check` already runs there on the default toolchain, so this is likely but unrun); executed: no
- Seen by: adequacy [33]; refutation: confirmed (not run); history: no-rationale-found (1a827d68 gave fuzzfit fmt plus clippy and fuzz-build fmt alone with no stated reason; wasm32-pins later received both)
- Owner-gated: yes: gate policy

The justfile's own argument for the sibling legs ("without them its source rots invisibly through green gates") applies verbatim to the fuzz workspace; clippy never sees the five targets or the harness lib.

Evidence:

    365	[working-directory("crates/before/fuzz")]
    366	fuzz-build:
    367	    cargo fmt --check
    368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

Resolution: Add `cargo clippy --all-targets -- -D warnings` to `fuzz-build`, mirroring wasm32-pins:642-643; if stable clippy cannot build the `#![no_main]` bins, pin a clippy component on the nightly toolchain instead. Acceptance: a `clippy::needless_borrow` planted in a fuzz target reddens `just gate`.
Construction: Insert `let _ = &(&data);` in `fuzz_decode.rs::run`: `just gate` stays green; `cargo clippy --all-targets -- -D warnings` in `crates/before/fuzz` fails.

## Positives

- The seed corpus is derived from the live public API in one place (`tests/support/fuzz_seed_set.rs`) and held byte-identical and stray-free by `tests/fuzz_seeds.rs`, which also pins each rejection witness to the exact genre it was written for; the two non-derivable frontier witnesses carry their bit-level derivations inline, and the wide-gamma seeds close a tail (64-plus-zero unary prefixes) coverage-guided search would never reach. Seed rot is a red gate with a one-command fix.
- `fuzz_decode_differential`'s rejection-genre axis is the right complement to round-trip fuzzing: explicit `Genre` and `Stage` enums make the allowance table legible, the one allowed fused-versus-composed divergence is spelled at the arm that admits it (143-154, 169-172) and matches `Span::decode`'s public `# Errors` contract ("the components' structural genres win"), and a committed seed per genre (`span_negative_join`, `span_crossed_padding`) means a reintroduced ordering defect fails the first replay rather than waiting on random discovery.
- `fuzz_laws` expands its drive loop from `before::for_each_law_group!`, so a law group added to the roster is fuzzed with no wiring and a novel signature refuses to compile; the arity band `0..=17` is derived from the balanced counter's structure with the reasoning stated (57-65), and the `Env` field docs say which decoded value feeds which slot and why.
- `fuzz_decode` asserts byte identity between the accepted input and its re-encode, which refuses an accept-and-normalize decoder outright rather than merely checking a round trip.
- The heap cap takes `peak_alloc` as a dependency rather than hand-rolling a `GlobalAlloc`, in line with the zero-unsafe policy even inside a fuzz binary, and its module doc states its own limit precisely (a spike that outruns the process is libFuzzer's RSS limit's job).
- The fuzz-fit guest keeps a strict measured/unmeasured partition (staging, register reservation, and query construction outside the fuel window; exactly one public operation inside), mirrors the API's linearity in the register file so a replayed program cannot alias a `Party`, keeps happy paths move-only with no defensive clones, and uses `split_at_mut` rather than cloning for two-register mutations. `ff_regs_reserve` keeps `Vec` doubling out of every fuel window, and the harness binds the reserve to both budgets with a const assert.
- `ff_selftest_quadratic` is the adequacy demonstration the doctrine asks of a criterion: a `black_box`-pinned known-bad mechanism the enforcement suite drives through the real wasm, fuel, and band-judge path (enforce.rs:273-310), in both the Above and the Below (liveness) directions.
- The wasm32 guest synthesizes every input in-guest with the strict canonical decoder as the synthesizer's oracle, documents each layout bit by bit with the canonicality argument stated (31-46, 237-250, 507-524), and the harness gives each coordinate below/at/past adjacency witnesses so a failure attributes to its seam; the join-emit pins construct complementary two-leaf skylines whose join concatenates, exercising the emitter's output side at sizes larger than either operand. Each pin runs in a fresh instance so one pin's peak cannot become the next pin's baseline, and `version_small_roundtrips_and_rejects_typed` is an always-green liveness pin for the leg.
- The wasm32-pins workspace keeps `overflow-checks = true` in the release profile with the rationale beside it, and the harness manifest explains why fuel and pooling are absent (outcomes, not counts; a full 4 GiB address space per pin).
- The justfile names each guest wasm path once and passes it explicitly at producer and every consumer, with the ambient-`CARGO_TARGET_DIR` failure mode it prevents written beside the variable (56-78); the detached fuzz workspace is compiled in the gate so a rename in `before` breaks a target in the same commit, with the reasoning for keeping the smoke at `just all` stated.

## Open questions for Finch

1. Are the three memory-terminal pins (pins.rs:130-153, 359-379, 715-742) a declared model or a pending cure? The header and the pins' own docs say different things. Recommendation: declare the model (the address-space bound is the terminal, and it aborts loudly), restate the three pins positively, narrow the header to seam pins, and add the origin discriminator (finding 35) so the instrument rather than the prose establishes "allocation failure"; if a leaner working set is wanted, own that as a `tests/meter.rs` envelope on decode's working set rather than a wasm32 pin.
2. Does the fuzz heap cap become proportional to input length, or is the flat cap ratified? Recommendation: proportional, with `PER_INPUT_BYTE` measured first over the seeds and a smoke run and committed with slack; either branch owes a committed test that the cap fires.
3. Is a seeds-replay gate leg (`-runs=0` per target, seconds) acceptable gate spend, or do the targets' oracles stay `just all`-only by design? Recommendation: add it; it is the cheapest liveness check the seed corpus already makes possible.
4. Is `fuzz_decode` retained for raw-door throughput and transport-feature independence? Recommendation: keep it and say so in one sentence of its module doc.
5. Should `just fuzz` pass `-max_len` above libFuzzer's 4096 default so multi-kilobyte trees reach the heap cap and the ops target by mutation, not only through the seeds? Recommendation: raise it modestly (16 to 64 KiB) once the cap is proportional; before that a larger `-max_len` only widens the blind window.
6. Does the wasm32-pins leg belong in CI's `instruments` job, or is it declared local? Recommendation: declare it local in both comments for now (the leg needs a wasm32 target plus wasmtime and a few GiB), and revisit when the `instruments` job's runner fit is next reviewed.
7. Should the eighteen signature arms move into an exported `laws`-feature macro shared by `drive_groups!` and `organic_drive!` (finding 12)? Owner-gated as an instrument-surface addition; recommendation: yes, with the pool-index choice fixed once.
8. Does clippy's `missing_const_for_thread_local` still misfire on the fallback-TLS lowering on illumos (fuzzfit guest 84-87)? Nobody in this review ran clippy there; if the lint no longer fires, the two `#[allow]`s and their comment are vestigial. Recommendation: check on ox-east-1 when the guest is next touched.
9. The three wasmtime pins (wasm32-pins 47.0.3, fuzzfit and fuelscape 47.0.4) and the two near-identical `guest_wasm_path`/`engine_and_module` drivers are the kind of duplication a shared tool-side crate would remove; the differing engine configs (fuel and pooling versus neither) argue against. Recommendation: align the three pins when the locks are next touched; leave the drivers separate.

## Dropped

- Straddle literals beside the named constant (adequacy [34], structure-prose [57], instrument-correctness [70]): duplicate of fuzz-guests-pins-34.
- `BUILD_CAP_BYTES` names a removed cap (structure-prose [37]): duplicate of fuzz-guests-pins-34.
- Terminal pins labeled as pending cures while documented as a model (adequacy [21], structure-prose [38], instrument-correctness [63]): duplicate of fuzz-guests-pins-33.
- Terminal pins assert a trap kind any panic produces (instrument-correctness [61]): duplicate of fuzz-guests-pins-35.
- `synth_version` wraps six bytes above the terminal (instrument-correctness [62]): duplicate of fuzz-guests-pins-27, with the count corrected to seven.
- Harness fallback path (adequacy [30], structure-prose [36], instrument-correctness [64]): duplicate of fuzz-guests-pins-32; [36]'s medium lowered to low because no gate or CI path reaches the fallback.
- Flat heap cap (adequacy [24], instrument-correctness [60]) and "not yet proportional" (structure-prose [51]): duplicate of fuzz-guests-pins-14; [60]'s acceptance on "the first seed" corrected (the committed seeds are at most 130 bytes, so a quadratic body peaks at about 17 KB there; the construction needs fuzzer-generated inputs near `-max_len`).
- Roster IDs (structure-prose [49], instrument-correctness [66]) and ghost test name (structure-prose [48], instrument-correctness [65]), and the bundle (adequacy [27]): duplicates of fuzz-guests-pins-2 and fuzz-guests-pins-3; [27]'s Cargo.toml:68 ride-along lives in fuzz-guests-pins-8.
- Manifest run commands drifted (structure-prose [50]): duplicate of fuzz-guests-pins-3.
- `fuzz_decode_ops` framing doc (structure-prose [46], instrument-correctness [72]): duplicate of fuzz-guests-pins-8.
- `ff_regs_reserve` path (adequacy [28], structure-prose [43], instrument-correctness [67]): duplicate of fuzz-guests-pins-20.
- Clock kernels bypass `with_c` (adequacy [35]); FNV twice and the sign mask (structure-prose [41]); accessor duplication (structure-prose [40]): folded into fuzz-guests-pins-18, at low per the refutation's reframe (a divergent copy fails loudly against the harness's native expectation).
- `dispatch!` arity list (structure-prose [45]): duplicate of fuzz-guests-pins-22.
- `fuzz_decode::run` six copies (structure-prose [52]): duplicate of fuzz-guests-pins-5.
- `debug_assert_eq!` compiled out (instrument-correctness [68]): duplicate of fuzz-guests-pins-30; [58]'s cited line 626 corrected to 552.
- Vocabulary tells (instrument-correctness [71]): split between fuzz-guests-pins-21 ("mint") and fuzz-guests-pins-17 ("honest", "real").
- `synth_version` and `synth_rank` hand-roll `set_bit` (structure-prose [59]): folded into fuzz-guests-pins-27's resolution, since the `u64` rewrite of `synth_version` routes through `set_bit` anyway.
- Trailing commas in five asserts (structure-prose [57]): folded into fuzz-guests-pins-34 as a rider; below the bar on its own.
- Guest synthesizer failures share the trap channel (structure-prose [39]): folded into fuzz-guests-pins-27 (the in-band synthesis codes) and fuzz-guests-pins-35 (the discriminator).
- The refutation pass's evidence corrections (lines 626 versus 552, nine versus eight "honest" sites, six versus seven bytes): applied in place; not findings.
