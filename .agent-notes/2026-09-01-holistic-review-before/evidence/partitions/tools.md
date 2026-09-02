# Partition tools: The workspace verification tools: benchjudge, citecheck, covcheck, digestshare, doclint, fuelscape-claims, manifestlint, memwatch, mutantcheck, readme, testdoc, workflowlint, and their expected-value rosters

## Partition summary

The `tools/` directory holds the gate's build-free checkers and three committed expectation rosters. Five of the tools are judges over captured artifacts: benchjudge fits wall-time exponents over criterion medians and enforces `tools/benchjudge-expected.json`; citecheck resolves the coverage roster's citations against a nextest listing; covcheck holds the skyline kernel's lcov residue to `tools/covcheck-expected.json`; mutantcheck holds `.cargo/mutants.toml`'s exclusion patterns to the counts in `tools/mutantcheck-expected.json`; workflowlint holds every `uses:` and every fetch pipeline in `.github/` to committed or digest-pinned code. Four are lints over the tree (doclint, testdoc, manifestlint, readme), two are meters (digestshare over the wire-capture corpus, fuelscape-claims over the widget datasets), and memwatch is a shell watchdog the codegen-running recipes wrap. The justfile's `gate-lints` line and `ci` line are the rosters that decide which of these run where.

The tools share one architecture that the owner's doctrine asks for and that is unusual to see executed this consistently: a single `run()` judgment over raw inputs, liveness floors on every extraction and haystack, spelling-totality guards that turn an unreadable entry into a red rather than a skip, input errors that exit 2 rather than scoring, thresholds derived at their constants, and a `--self-test` at the head of each recipe that pins the tool's own red paths. benchjudge's self-test asserts `main`'s exit codes end to end, so a judge that stops failing cannot pass its own self-test; its ceiling class rides the bench sidecar and is asserted at every sidecar write, so no roster edit can move a ceiling. citecheck's fabricated-citation tripwire and spelling-totality denominator close the partial-rot hole that floors alone cannot see. mutantcheck refuses colored and polluted captures, pins the tool version, and never writes its own pin. These are done well and are recorded under Positives.

The dominant issues are three. First, two instruments still carry a vocabulary for accepting known failures: covcheck's `remediation` disposition and benchjudge's open `red` class, both empty of library entries today, both used as buffers in their history, and both surviving the owner's 2026-08-07 ruling (920bfabb2) that no such mechanism may exist even empty, because that ruling's excision inventory was board-scoped. Second, several floors are one coarse premise that cannot see partial darkness: covcheck judges only the files the lcov names (twelve kernel files have no entry and would vanish silently), mutantcheck never asks whether a mutant missing from the filtered listing is claimed by any pinned pattern (a file-level `exclude_globs` shrinks the campaign with every pinned count unchanged), benchjudge lets every unrostered board cell drift into SKIP, and digestshare's floor is conditioned on having seen a file at all (an empty or missing corpus exits 0). Third, the bench judge's stated unique failure class ("work no counter column can see") is also claimed by the fuzz-fit row of the validation index, and the committed tripwire is a plain machine-word quadratic that wasmtime fuel would also read red; the leg's residual class (cost that is not instructions) is neither named nor demonstrated. Smaller items are a correctness hole in workflowlint's interpreter recognizer (the most common real installer spelling, `| sudo -E bash -`, passes), two hand-rolled file walkers that read generated code and a foreign worktree into gate legs, a `ci` roster that omits manifestlint, and prose that cites code no longer in the tree.

I read all fifteen partition files in full (4845 lines: benchjudge 999, citecheck 856, workflowlint 558, doclint 434, mutantcheck 406, covcheck 335, readme 259, covcheck-expected.json 227, memwatch 219, manifestlint 168, testdoc 151, digestshare 108, mutantcheck-expected.json 61, fuelscape-claims 49, benchjudge-expected.json 15) plus roughly 700 lines of related sites (justfile, ci.yml, crates/before/tests/bench_judge_roster.rs, benches/common/sidecar.rs, benches/board.rs, benches/tripwire.rs, src/testing/validation_index.rs, .cargo/mutants.toml, tests/seed_liveness.rs, the fuzzfit harness docs). No partition file is a test file; each tool carries its self-test inline, and bench_judge_roster.rs is the one test file consulted. HEAD is 7440d1a3, two commits past the briefed 9e5784fb; `git diff --stat 9e5784fb HEAD -- tools/ justfile .cargo .github crates/before` is empty, so every file read is byte-identical at both. Procedural disclosures: I ran the Python tools on synthetic input under my scratch directory and one shell function in isolation (no cargo, just, build, test, or bench); those module loads wrote `tools/__pycache__/benchjudgecpython-314.pyc` into the untracked `tools/__pycache__/` directory another reviewer had already created, which I left in place.

## Findings

### tools-1: ci.yml restates memwatch's per-process cap as a number that has rotted
- Where: .github/workflows/ci.yml:26-28 (related: tools/memwatch:46)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites; `git log -1 a1febcce` is "memwatch: raise the per-process cap to 32 GiB", 2026-08-04); executed: no
- Seen by: scaffolding [13], adequacy [26], instrument-correctness [54]; refutation: confirmed; history: deliberate-but-expired (the comment was written at 8 GiB in c440730b, 2026-06-19; the default was raised to 12, 16, and 32 without touching ci.yml)
- Owner-gated: no

The workflow comment hand-copies the default of `PROC_LIMIT_GB`, and the copy is stale. Principle 5: a number restated away from its declaration rots silently; the argument ("sits well above anything a normal Linux build reaches") survives without the literal.

Evidence:

    26	# memwatch's swap backstop is sysctl-based and degrades to a no-op off macOS, so
    27	# wrapping the test/doctest/bench recipes is harmless here; its 8 GiB per-process
    28	# cap sits well above anything a normal Linux build reaches.

    (tools/memwatch)
    46	PROC_LIMIT_GB="${PROC_LIMIT_GB:-32}"

Resolution: drop the figure and name the variable ("its per-process cap, `PROC_LIMIT_GB` in tools/memwatch, sits well above ..."). Acceptance: `grep -n GiB .github/workflows/ci.yml` is empty.

### tools-2: The validation index omits citecheck, covcheck, and mutantcheck
- Where: crates/before/src/testing/validation_index.rs:1-3 (related: validation_index.rs:109-119; tools/citecheck:8-34; tools/covcheck:8-22; tools/mutantcheck:9-36)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'citecheck\|covcheck\|mutantcheck\|tools/'` over the index matches only the bench-judge row at 109-119); executed: no
- Seen by: scaffolding [16]; refutation: confirmed (and notes the omission set is wider: surfacecheck and the wasm32 pins also have no row); history: no-rationale-found (chronology: the index landed 07-28 and was last edited 08-07; the three tools landed 08-11 to 08-13 without touching it)
- Owner-gated: no

The index promises a row for every instrument guarding the crate and the class each alone catches. Three gate or CI instruments that guard `before` (uncollected citations; unexercised kernel arms by name; exclusion patterns drifting from the live mutant inventory) have no row, so a maintainer orienting cold never learns they exist.

Evidence:

     1	//! The validation index: every instrument that guards this crate, what
     2	//! failure class each one catches that the others cannot, and where it
     3	//! lives.

Resolution: add one row each under the semantic instruments (citecheck, covcheck, mutantcheck), naming the recipe and the roster file; consider rows for surfacecheck and the wasm32 pins in the same pass. Acceptance: `grep -n 'citecheck\|covcheck\|mutantcheck' crates/before/src/testing/validation_index.rs` returns one row each.

### tools-3: digestshare's gate role is already carried by the byte-pinned snapshots, and its ratio has no in-tree consumer
- Where: justfile:212-216 (related: tools/digestshare:2-13; justfile:403, 1000; tests/gossip_snapshot.rs)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: verified (`ls design/` lists only rumors-frame-fuzz.md; grep for `digestshare` outside tools/ hits only justfile:211-220, 403, 1000; `git show -s 5f140251` describes the number as the "parent baseline" for the hash-width decision, which has since landed as SHA3-256 in 4f18c347); executed: no
- Seen by: scaffolding [7]; refutation: confirmed; history: deliberate-and-holds (an owner-gated recommendation of the CBOR wire review, implemented in fd997888 with the rationale at the recipe; the review weighed "fires only by hand" against "stays a manual aid" but did not weigh the snapshot suite's overlap, and the ratio consumer it named, design/cbor-legible-wire.md, has since moved to .agent-notes)
- Owner-gated: yes (gate policy; reopens a recorded ruling with new evidence: the consumer is gone and the decision it served has landed)

The recipe says the leg checks the renderer-vocabulary contract, never the ratio. The insta snapshot suite already pins the renderer's output byte-for-byte (root AGENTS.md hard rules), so a vocabulary move fails there first. What the leg adds is keeping a by-hand tool's regexes matching, for a number nothing committed reads. Principle 3: a thing earns its existence by what it serves outside itself.

Evidence:

   212	# vs non-digest bytes. As a gate leg it checks the renderer-vocabulary
   213	# contract, not a threshold: the tool exits nonzero when the corpus's
   214	# byte-count headers or digest annotations stop matching its patterns (the
   215	# renderer's vocabulary moved out from under the meter), never on the
   216	# measured ratio. Build-free, so it rides the lint tier.

Resolution: owner call between (a) moving `digestshare` out of `gate-lints` and `ci` into the conveniences section, keeping the tool (with tools-19's floor) for interactive runs, and (b) committing a consumer of the TOTAL line (a rustdoc figure or a pinned expectation) so the leg guards a claim rather than itself. Acceptance: either gate-lints no longer lists digestshare, or a committed artifact cites the ratio and the gate checks it.

### tools-4: `ci` omits manifestlint, so the lint tier has two hand-maintained rosters and GitHub CI never runs it
- Where: justfile:1000 (related: justfile:403; .github/workflows/ci.yml:14-15, 107-108)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read lines 403 and 1000 side by side; ci.yml:107-108 runs only `just ci`; `git log -1 84044759` shows manifestlint landing into gate-lints on 2026-08-31); executed: no
- Seen by: scaffolding [1], adequacy [25]; refutation: confirmed (one correction: ci.yml:14-15's "exactly one definition" sentence is about the workflow re-listing justfile steps, which holds; the drift is between two lines inside the justfile); history: already-known (recorded as verification-infra-7 in the rumors review, no-rationale-found; the same omission happened once for digestshare and was patched by de9e0bdf adding the leg to `ci`, which supports the structural fix below)
- Owner-gated: no

`gate-lints` lists eight build-free legs; `ci` restates seven of them and omits `manifestlint`, so a member manifest that restates a version or names a `path`/`git` source passes GitHub CI. Two hand-maintained lists of the same legs drift by exactly this mechanism, and it has now happened twice.

Evidence:

   403	gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check

  1000	ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

Resolution: make `ci` depend on `gate-lints` instead of re-listing its legs (`ci: gate-lints fuelscape-claims clippy clippy-default ...`), so the lint tier has one definition; ci.yml:83-86 already installs cargo-mutants and cargo-rdme, so nothing new is required of the runner. Acceptance: `just --show ci` names `gate-lints` as a dependency; a member manifest carrying `path = "../suanpan"` beside `workspace = true` fails `just ci`.
Construction: add `path = "../suanpan"` beside `workspace = true` on any member dependency; `just gate-lints` fails at manifestlint and `just ci` passes.

### tools-5: The bench judge's stated unique class is also priced by fuzz-fit fuel, and its committed tripwire is a fuel-visible quadratic
- Where: tools/benchjudge:5-7 (related: crates/before/src/testing/validation_index.rs:12-13, 113-116, 125-128; crates/before/benches/tripwire.rs:4-12, 42-52; crates/before/fuzzfit/harness/src/lib.rs:4-13; crates/before/fuzzfit/harness/src/ops.rs:17-19; crates/before/fuzzfit/harness/tests/enforce.rs:25, 263; justfile:467, 1003; .github/workflows/ci.yml:114-120)
- Class / severity / confidence: scaffolding / medium / medium
- Provenance: assessed (cross-read the sites named above); executed: no
- Seen by: scaffolding [2]; refutation: reframed (the two index rows do overlap, and the tripwire's kernel is pure machine-word arithmetic that fuel counts; but the premise that fuel prices every time-visible class is wrong: wasmtime charges fuel per executed wasm instruction, so bulk-memory instructions cost one unit regardless of bytes moved, and native codegen and memory-hierarchy effects are wall-time-only, assessed from wasmtime's documented fuel semantics and not verified here); history: deliberate-but-expired (the judge's "only witness" rationale, 57fe866d on 07-24, predates fuzzfit, 8cfd3c92 on 07-26, whose design note claims the judge's class explicitly; the index, written after both, re-states both rows without a separating input; no recorded decision revisits keeping the judge after fuel)
- Owner-gated: yes (restating or retiring an instrument; the leg was created by owner decision 54b68b4f)

The index's bar for an instrument is "a failure class no row below already catches, named as a constructible input" (validation_index.rs:12-13). The judge's row names "backend multiplication below the limb shim, container bookkeeping between metered primitives"; the fuzz-fit row names "total-cost drift that escapes the metered currencies while still costing instructions". Fuel counts both. The committed live demonstration (benches/tripwire.rs) says it "catches what no deterministic meter can", but its kernel is a machine-word double loop that fuel reads quadratic too, so nothing committed demonstrates a class only wall time sees. Fuzz-fit runs in the gate (justfile:467); the judge runs only at `just all` (justfile:1003), on the one nondeterministic currency, behind a quiet-machine precondition CI cannot meet (ci.yml:114-116). Principle 3: two rows claiming one class is the circular-justification tell.

Evidence:

     5	counters and liveness floors only, so its output is byte-identical under
     6	any machine load. The implementation-agnostic witness for *time* — work in
     7	plain machine words that no counter column can see — is judged here

    (crates/before/src/testing/validation_index.rs)
   113	//! pinned by `tests/bench_judge_roster.rs`). What it alone catches:
   114	//! **work invisible to every deterministic counter** — cost in layers
   115	//! the meters do not instrument (backend multiplication below the limb
   116	//! shim, container bookkeeping between metered primitives). It is the
   ...
   127	//! chosen-family instrument above — and total-cost drift that escapes
   128	//! the metered currencies while still costing instructions. Its bands

    (crates/before/benches/tripwire.rs)
     4	//! One bench, `tripwire_unmetered_quadratic/quadratic`: a quadratic pass of
     5	//! plain machine-word arithmetic — no allocation, no recursion, no
     6	//! big-integer ops, no stream reads — so every counter column the
   ...
    12	//! judge's leg catches what no deterministic meter can. The same shape is

Resolution: owner decision, two exits. (a) Keep the leg: restate its unique class as non-instruction cost at the board's families and acceptance scale (bulk-memory operations that fuel prices as one instruction, native codegen, memory hierarchy), and commit a tripwire that demonstrates it, reading RED under the judge and GREEN under fuzz-fit's bands; if no such kernel can be built at the board's byte scales, that failure is the evidence for (b). (b) Retire the leg after committing a fuzz-fit demonstration that tripwire.rs's shape reads above-band under fuel, then remove benchjudge, its roster, tests/bench_judge_roster.rs, tripwire.rs, the sidecar's `Ceiling` machinery, and the two recipes, and let the fuzz-fit row absorb the class. Either way the index's two rows stop claiming one class, and tripwire.rs:12 stops saying "what no deterministic meter can". Acceptance: either a committed time-only tripwire fails `just bench-judge-tripwire` and is banded green by `just fuzzfit`, with validation_index.rs stating that class; or the leg is gone and `just fuzzfit` demonstrably reads the retired tripwire's shape red.
Construction: for (b), transplant `unmetered_quadratic` (tripwire.rs:42-52) into the fuzz-fit guest as a banded kernel and run `just fuzzfit`: it reads above-band, so the judge catches nothing the bands do not. For (a), build a kernel whose wasm lowering is a single `memory.copy`/`memory.fill` per step over a growing buffer (instruction count linear in steps, bytes moved quadratic) and show RED under the judge and GREEN under fuel.

### tools-6: The roster's `red` class is documented as a buffer for owned reds awaiting cures, and the mechanism accepts any cell
- Where: tools/benchjudge:65-69 (related: benchjudge:74-80, 476-480, 528-536; tools/benchjudge-expected.json:2; crates/before/tests/bench_judge_roster.rs:7, 45-63; crates/before/benches/common/sidecar.rs:36-60)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read `load_roster` 476-480 and `roster_violations` 528-536: `red` is any list of strings, and any rostered cell that reads RED satisfies it; `git log -1` for 162290ab "bench judge: the hugeleaf display pair stays rostered red - the realization run's verdict" (07-27), a066a8e9 "declared models instead of standing reds" (07-28), and 920bfabb2 "board: excise the expected-reds triage buffer; any red of record fails outright" (08-07); `git log -S'owned reds' -- tools/benchjudge` returns only b4942461); executed: no
- Seen by: scaffolding [6], adequacy [21]; refutation: confirmed; history: deliberate-but-expired (the framing was written for a transitional roster during the kernel flip and was accurate then; the red set collapsed to the tripwire two days later; the owner ratified red = untriaged and moved the last library reds to declared models, rewriting the JSON notes but not the docstring; the 08-07 ruling's excision inventory lists no tools/ file)
- Owner-gated: yes (a roster's vocabulary is gate policy; the recorded ruling is board-scoped in letter, general in principle)

Three sites describe the class two ways: the docstring says roster mode keeps `just all` meaningful "while owned reds await their cures", while the JSON notes and the roster test say the set is exactly the permanent tripwire. The mechanism matches the docstring: a library cell added to `red` plus a one-line edit to `roster_red_membership_is_pinned` converts a standing regression into a pass, and history shows the path was used (162290ab). Doctrine under Principle 2: no mechanism for accepting known failures may exist, even as an empty buffer; a known-bad artifact required red is legitimate adequacy machinery, and the class name does not distinguish the two.

Evidence:

    65	Roster mode (`--roster FILE`): the committed expected-verdict roster —
    66	the board-side sibling of the gate's expected-failure test roster, same
    67	membership-by-name philosophy — makes the judge enforce a *fixed* verdict
    68	map instead of all-green, so `just all` stays meaningful while owned reds
    69	await their cures. The file pins the configuration it was recorded under

    (crates/before/tests/bench_judge_roster.rs)
    57	/// Removing an owned red silences a standing judgment and adding one
    58	/// launders a new regression as expected, so both directions must show
    59	/// up as a diff of this pin.

    (git show -s 920bfabb2)
    Owner ruling (2026-08-07, superseding the empty-buffer allowance): no
    expected-reds/accepted-reds mechanism may exist at all.

Resolution: bind the class to bench code the way `Ceiling` already is: the sidecar declares a tripwire flag at the cell's definition (only `display_schoolbook/hugeleaf` and tripwire.rs's cell qualify), `load_cells` cross-checks the two sidecars agree, and `roster_violations` requires roster `red` to equal the sidecar's tripwire set exactly, so a library cell can never be rostered and an owned regression must become a cure or a declared model. Rename the key (`tripwire`) so an awaiting-cure entry needs a schema change that `roster_schema_carries_expectations_only` and `load_roster` both refuse. Reword benchjudge:65-80, the JSON notes, and bench_judge_roster.rs:7 and 57-59 (drop "owned red"). Acceptance: self-test cases: a roster naming a non-tripwire cell is an input error (exit 2); a tripwire cell missing from the roster is an input error; the existing laundering pins still hold; the three descriptions agree.
Construction: add `"version_rank/harmonic"` to `red` in tools/benchjudge-expected.json and to the expected array at bench_judge_roster.rs:62. If that cell ever reads RED, `just bench-judge` exits 0 and the gate's test suite is green: a regression accepted by two one-line edits, with no cure and no declared model.

### tools-7: Unrostered cells drift GREEN and SKIP freely, so a board cell whose bench body goes dark passes the judge
- Where: tools/benchjudge:82-84 (related: benchjudge:537-547; crates/before/benches/board.rs:166-171, 242-245; crates/before/tests/bench_judge_roster.rs:50-52)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read `roster_violations` 513-547: it flags a rostered red reading GREEN or SKIP and an unrostered RED, and nothing else; board.rs:166-171 is one shared closure for every board cell, while the tripwire cell at 242-245 is a separate closure); executed: no
- Seen by: adequacy [20]; refutation: confirmed (the construction traces to SKIP on every board cell, RED on the tripwire, no violation, exit 0); history: deliberate-and-holds for near-floor cells only (the rationale at 82-84 and in b4942461 is machine noise across the judgment floor; nothing addresses a cell far above the floor going dark)
- Owner-gated: yes (the drift is declared as design in the docstring)

The only liveness signal in roster mode is the single rostered red, produced by `schoolbook_decimal` in bench code. Every library cell may read GREEN or SKIP with no expectation attached, so a cell whose timed body degenerates to trivial work is never flagged; the tripwire proves the judge's arithmetic and the criterion pipeline are alive, not that any library bench measures real work. bench_judge_roster.rs:50-52 claims the schoolbook red proves "the judge's time leg" alive, which overstates what that red covers. Doctrine: every ceiling needs a liveness floor so it cannot pass vacuously when the measured quantity goes dark.

Evidence:

    82	Any unrostered cell reading RED is a violation (exit 1). Unrostered cells
    83	may drift between GREEN and SKIP freely — both are non-red, and
    84	near-floor cells cross the judgment floor with machine noise. A roster

    (crates/before/benches/board.rs)
   166	            group.bench_function(cell.family, |b| {
   167	                b.iter_batched(
   168	                    || cell.body(),
   169	                    |body| black_box(body()),
   170	                    BatchSize::LargeInput,
   171	                );

Resolution: add a judged-set expectation: cells whose hi median sat well above the floor at pinning (a 10x margin, so noise cannot cross it) must read GREEN or RED, never SKIP, and such a cell reading SKIP is a violation. Declare it where the ceiling class is declared (the sidecar, a per-cell `judged` flag asserted against a pinned set), so the roster still carries expectations only and `roster_schema_carries_expectations_only` stays as is. Acceptance: a self-test case in which a judged-declared cell with a sub-floor pair returns exit 1 in roster mode; the construction below returns 1 instead of 0.
Construction: at board.rs:169 replace `|body| black_box(body())` with `|body| black_box(&body)` and run `just bench-judge`: every board cell reads SKIP (hi median below 10 us) or a flat GREEN, `display_schoolbook/hugeleaf` still reads RED, `roster_violations` finds nothing, exit 0.

### tools-8: Default-dialect vocabulary: "honest" as an algorithm class and a fixture label, "mint" for constructing a value
- Where: tools/benchjudge:126-127 (related: benchjudge:129, 153, 447, 859, 862-863; tools/memwatch:70, 94; tools/citecheck:532; tools/covcheck:254; tools/mutantcheck:263; tools/workflowlint:330; tools/doclint:295; crates/before/benches/common/sidecar.rs:42-43)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n '\bmint' tools/*` returns exactly benchjudge:447, benchjudge:859, memwatch:70, memwatch:94; `grep -ci honest` per tool: benchjudge 10, citecheck 2, covcheck 2, mutantcheck 2, workflowlint 1, doclint 1); executed: no
- Seen by: structure-prose [46], [47]; refutation: confirmed (and notes "honest" also pervades .cargo/mutants.toml:93 and :158, so this is a house-dialect ruling rather than a tool-local slip); history: contradicts the owner's writing doctrine (~/.claude/writing-style.md:170 bans "mint"; :326-330 and :409 ask for the property where "honest" beckons), both recorded after the code was written; the one "mint" purge (2c73d032) was scoped to the rumors crate
- Owner-gated: no

benchjudge calls divide-and-conquer conversion "the honest class" and every self-test in the partition calls its green baseline "the honest fixture"; four sites use "mint" for creating a record or a class. Each has a precise name (the divide-and-conquer class; the baseline fixture; construct, insert, or forge).

Evidence:

   126	# parsing alike — is honestly superlinear (divide-and-conquer conversion
   127	# is ~O(n^1.5)), so the general ceiling would read the honest class red.
   447	    error — a roster cannot mint a class, and in particular cannot mint

    (tools/memwatch)
    70	# sequence) inside one would mint synthetic records in the forensic log.

Resolution: "the divide-and-conquer class" and "the divide-and-conquer render" in benchjudge and sidecar.rs; "the baseline fixture" in the self-tests; "a roster cannot declare a class"; memwatch: "would forge a synthetic record". Acceptance: `grep -n '\bmint' tools/*` is empty and `grep -ci honest tools/benchjudge` is 0.

### tools-9: benchjudge cites code that no longer exists and narrates its own migration
- Where: tools/benchjudge:128-130 (related: benchjudge:23, 29-30, 65-67, 857-858, 864; tools/benchjudge-expected.json:2; tools/memwatch:4-8)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep over the tree excluding .agent-notes, target, and .claude: `expected-failure` appears only at benchjudge:66 and the JSON's line 2, `wall-ratio` only at benchjudge:130, `moved here with the leg` only at benchjudge:30; `git log -1 4451386a` is "Retire the display canary: its judgment lives in the bench judge", 2026-07-24); executed: no
- Seen by: scaffolding [5], structure-prose [39], instrument-correctness [53], adequacy [31]; refutation: confirmed (the sibling-roster phrase had no tree referent even at its authoring commit b4942461; the compile-fail suite that may have been meant was retired in d74c7f7f); history: contradicts-hard-rule (root AGENTS.md: nothing refers to code that no longer exists; the dated-notes sweep d2a9d04e edited these paragraphs and left the phrases)
- Owner-gated: no

The text-ceiling derivation names a "retired wall-ratio canary" that appears nowhere in the tree; the roster paragraph names "the gate's expected-failure test roster", which does not exist and would breach the no-known-failures rule if it did; the fit paragraph says the ceiling "moved here with the leg"; two self-test pins are labeled by the review that produced them rather than the attack they close. memwatch:4-8 opens with an incident narrative at a declaration site (doctrine rather than the hard rule: its pointer resolves to a live comment in src/tree/traverse/act.rs).

Evidence:

   128	# This ceiling separates
   129	# the honest class from the schoolbook (quadratic) class it replaces the
   130	# retired wall-ratio canary in catching. Placement is derived from the
    29	ratio over log of the denominator-bytes ratio) at the general ceiling
    30	that moved here with the leg — the same convention, not a new one. A
    66	the board-side sibling of the gate's expected-failure test roster, same
   857	    # LAUNDERING PIN (the ceiling-class review's demonstrated attack):
   864	    # LAUNDERING PIN (the review's second attack): the schoolbook tripwire

    (tools/memwatch)
     4	# Why this exists: a monomorphization bomb (see the comment in
     5	# src/tree/traverse/act.rs) once made every leaf-crate rustc invocation
     6	# consume 25+ GiB at codegen, outrunning jetsam and wedging the machine into

Resolution: benchjudge:128-130: "This ceiling separates the divide-and-conquer class from the schoolbook (quadratic) class." Delete the sibling clause at 65-67 and in the JSON notes; at 29-30 write "at the general ceiling — the same convention"; at 857 and 864 name the attack ("a roster class cannot select a ceiling"; "a rostered red is still judged at its own ceiling"); reflow the orphaned short lines at 23 and 128. memwatch:4-8: state the invariant ("a codegen runaway fails the build with the crate named instead of wedging the machine") and leave the incident to git. Acceptance: `grep -rn 'wall-ratio\|expected-failure\|moved here\|review.s .*attack\|once made' tools/` is empty.

### tools-10: Duplicated computation: the denominator pair is validated twice per judged cell, and mutantcheck computes each pattern's counts twice
- Where: tools/benchjudge:244-247 (related: benchjudge:268-270, 286; tools/mutantcheck:184-185, 232-233)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (`judge_cells` at 286 is `fit_exponent`'s only caller and calls `checked_denominators` at 270 before the floor test, so the call at 246 can never raise; mutantcheck's `run` and `observed_table` carry the same two expressions); executed: no
- Seen by: structure-prose [42], instrument-correctness [59]; refutation: confirmed; history: no-rationale-found (651eb92a extracted the helper and added the pre-floor call without removing the original; mutantcheck's `observed_table` repeated the expression from its landing)
- Owner-gated: no

One definition per quantity: the observed table printed for re-pinning must be the same arithmetic the judgment used, and today that is true only by copy; the second `checked_denominators` call reads as if it were doing work.

Evidence:

   244	def fit_exponent(median_lo, median_hi, denom_lo, denom_hi, name):
   245	    """The board's scaling-exponent fit over one cell's two medians."""
   246	    checked_denominators(name, denom_lo, denom_hi)
   247	    return math.log(median_hi / median_lo) / math.log(denom_hi / denom_lo)
   268	        median_lo = checked_median(name, "lo", median_lo)
   269	        median_hi = checked_median(name, "hi", median_hi)
   270	        checked_denominators(name, denom_lo, denom_hi)

    (tools/mutantcheck)
   184	        listed = sum(1 for name in raw if rx.search(name))
   185	        suppressed = listed - sum(1 for name in filtered if rx.search(name))

Resolution: drop the call at benchjudge:246 and let `fit_exponent`'s docstring say its caller has validated the pair (the early call is the one whose ordering the docstring at 255-259 explains); mutantcheck: a `counts(rx, raw, filtered)` helper used by `run` and `observed_table`. Acceptance: one `checked_denominators` call per judged cell; the count expression appears once in mutantcheck; both self-tests pass.

### tools-11: benchjudge's self-test is fifty bare asserts that `python3 -O` strips
- Where: tools/benchjudge:596-598 (related: benchjudge:586-923; tools/citecheck:805)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified; executed: yes (`PYTHONOPTIMIZE=1 python3 tools/benchjudge --self-test` printed nothing and exited 0; `grep -cE '^\s*assert '`: benchjudge 50, citecheck 1, every other tool 0)
- Seen by: adequacy [29]; refutation: confirmed; history: no-rationale-found (facc7550 closed this hole for workflowlint alone: "self-test failures raise explicitly, surviving PYTHONOPTIMIZE"; benchjudge's asserts were never revisited)
- Owner-gated: no

Under `PYTHONOPTIMIZE=1` every `assert` in `self_test` is removed and `--self-test` returns 0 having checked nothing, while the bench-judge recipes run `--self-test` first precisely so the judge cannot go dark unnoticed. Every other tool raises `AssertionError` explicitly (citecheck has one bare assert at 805). Principle 1: the likelihood of the environment carries no weight.

Evidence:

   596	    assert verdicts([("op/linear", 1e6, 4e6, 1000, 4000, "general")]) == [
   597	        ("GREEN", "op/linear")
   598	    ]

Resolution: open `self_test` with `if sys.flags.optimize: raise SystemExit("benchjudge --self-test needs asserts; run without -O")`, or convert the asserts to explicit `if not ...: raise AssertionError(...)` as the sibling tools do; same one-liner for citecheck:805. Acceptance: `PYTHONOPTIMIZE=1 python3 tools/benchjudge --self-test` exits nonzero.

### tools-12: The roster's notes duplicate measured exponents held in sidecar.rs and enumerate the text cells by hand
- Where: tools/benchjudge-expected.json:2 (related: crates/before/benches/common/sidecar.rs:72-80, 86-94)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (both sites read: "measured e 1.39/1.42" and "1.28/1.33/1.30" appear in the JSON notes and at sidecar.rs:73-78; "0.93–1.18" and "1.464/2.001" live only in the notes; the notes name six of TEXT_CEILING_CELLS' seven members); executed: no
- Seen by: scaffolding [19], adequacy [31], structure-prose [40]; refutation: confirmed (correcting [31]: six of seven cells are named, not seven); history: deliberate-and-holds for the figures themselves (the owner's dated-notes sweep d2a9d04e edited this line and kept the undated figures as measurement provenance), no-rationale-found for the duplication
- Owner-gated: no

Per the owner's applied rule, undated measured figures stay; the finding is the duplication and the hand-maintained enumeration. Two hand-copied sites for one measurement drift independently, the one-line JSON string is unreviewable as a diff, and the cell list restates a constant the sidecar asserts.

Evidence:

    (excerpt of line 2)
    ... the display pair (`version_display/hugeleaf`, `clock_display/hugeleaf`; measured e 1.39/1.42) outbound and the parse trio (`version_parse_trailing/hugeleaf`, `version_parse_noncanon/hugeleaf`, `clock_parse_trailing/hugeleaf`; measured e 1.28/1.33/1.30) inbound ...

    (crates/before/benches/common/sidecar.rs)
    73	/// (`version_display`, `clock_display`) renders binary→decimal —
    74	/// measured exponents 1.39/1.42. Inbound, the hugeleaf parse trio
    77	/// decimal→binary on the way to the placed defect — measured exponents
    78	/// 1.28/1.33/1.30.

Resolution: trim `notes` to the roster's contract (what the file is, what `red` means, that membership is pinned by tests/bench_judge_roster.rs, that ceilings ride the sidecar), point at `TEXT_CEILING_CELLS` for the text set, and keep each measurement at one site (sidecar.rs already carries two; move the riders' and record-mode figures beside them or drop them). Acceptance: 1.39/1.42 and 1.28/1.33/1.30 appear at one site; the notes name no cell that TEXT_CEILING_CELLS does not.

### tools-13: citecheck carries three string-literal-aware scanners with three escape conventions
- Where: tools/citecheck:88-108 (related: citecheck:205-235, 289-329)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (line 101 uses a lookbehind on the previous character, line 222 skips one character on a backslash, line 310 skips two; only `macro_block_fn_names` handles char literals and block comments); executed: no
- Seen by: structure-prose [41]; refutation: confirmed; history: no-rationale-found (two commits wrote the three lexers; 11363f89 fixed a crash in one without unifying them)
- Owner-gated: no

Three lexers means three places a Rust lexical corner (a `'"'` char literal, an escaped quote) is handled differently, and three places to test. Legibility and one definition per capability.

Evidence:

   101	            if c == '"' and (i == 0 or line[i - 1] != "\\"):
   102	                in_string = not in_string
   103	            elif not in_string and c == "/" and line[i : i + 2] == "//":
   222	            if c == "\\":
   223	                i += 1
   310	                    i += 2 if text[i] == "\\" else 1

Resolution: one `code_spans(text)` generator that skips string and char literals and both comment forms and yields code text with offsets; `strip_line_comments`, `top_level_entries`, and the `macro_block_fn_names` brace walk consume it. Acceptance: one lexer function; the existing fixtures (the quoted-brace descriptor at 481-486 included) still pass.

### tools-14: covcheck's `remediation` disposition is a mechanism for accepting known failures, empty today
- Where: tools/covcheck:11-14 (related: covcheck:77, 264, 272, 282, 289; justfile:1006-1009, 1024-1026; tools/covcheck-expected.json)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (`run()` over an expectation entry `{"anchor": "covered();", "disposition": "remediation", "why": "later"}` anchored on an uncovered reachable line returned no problems; the committed file tallies line 16 panic-arm / 8 unreachable, branch 10 panic-arm, 0 remediation)
- Seen by: scaffolding [3], adequacy [22], structure-prose [33]; refutation: confirmed; history: deliberate-but-expired (designed as the landing ratchet's named-gap category holding 31 entries at 8a9583c5 on 08-11, drained to zero by f77011e3 on 08-12; the owner ruled four days before covcheck landed, for the sibling board instrument in 920bfabb2, that no accepted-reds mechanism may exist even empty; the letter of that ruling is board-scoped, its principle general)
- Owner-gated: yes (the justfile designs the category)

A `remediation` entry passes the coverage pin while stating that the arm is reachable and unexercised: an accepted known failure by construction, and git shows CI read green while seven such entries stood (aa7c96a0). Doctrine under Principle 2: every contradiction resolves to a fix or a model the owner declares; a reachable uncovered kernel line is a test gap, and until the test lands the leg is red.

Evidence:

    11	and the checker rejects any other: "panic-arm" (the untaken code is a
    12	panic only programmer error can trigger), "unreachable" (genuinely
    13	unexercisable, with a reconstructible argument naming the pinned
    14	invariant it rests on), and "remediation" (reachable but unexercised: a
    77	    dispositions = {"panic-arm", "unreachable", "remediation"}

    (justfile)
  1008	# panic-arm or an unreachable arm, its argument stated at the entry) or a
  1009	# named remediation item, and any NEW hole fails by name. MECHANISM:

Resolution: delete `remediation` from `dispositions` and the docstring; move the self-test fixtures at 264, 272, 282, 289 to `unreachable` or `panic-arm`; restate justfile:1006-1009 as "curated (panic-arm or unreachable, its argument at the entry) or a failure until a directed test lands" and 1024-1026 accordingly. If the owner keeps the category, record the ruling positively at line 77 as a deliberate exception. Acceptance: an entry carrying `"disposition": "remediation"` fails with the "never invent a category" message; `covcheck --self-test` passes with two dispositions; `just coverage-kernel` still passes on the committed roster.

### tools-15: covcheck validates only the disposition: a `why`-less or extra-keyed entry passes and a missing `anchor` tracebacks
- Where: tools/covcheck:87-95 (related: covcheck:8-10; tools/mutantcheck:204-219)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified; executed: yes (an entry `{"anchor": "None => unreachable!(", "disposition": "unreachable", "bogus": 1}` with no `why` passed `run()` with no problems; an entry lacking `anchor` raised `KeyError: 'anchor'` at line 95; every committed entry carries a nonempty `why` and no keys outside {anchor, disposition, why, offset})
- Seen by: instrument-correctness [55]; refutation: confirmed; history: no-rationale-found (the landing self-test pins the invented-category path only; the hardening round 11363f89 that made sibling checkers total on malformed input did not touch covcheck)
- Owner-gated: no

The docstring promises "each with a curated disposition and the argument for it"; the checker never requires the argument. The cheapest artifact that passes the pin is an `unreachable` entry with no `why`, exactly the category the docstring says must carry "a reconstructible argument". A missing `anchor` discards every other finding as a traceback, against the discipline mutantcheck's `compile_pattern` applies.

Evidence:

    87	        for entry in file_entries:
    88	            if entry.get("disposition") not in dispositions:
    89	                problems.append(
    90	                    f"{relpath}: entry {entry.get('anchor')!r} carries disposition "
    91	                    f"{entry.get('disposition')!r}; only "
    92	                    f"{sorted(dispositions)} exist — re-curate, never invent a category"
    93	                )
    94	                continue
    95	            anchor = entry["anchor"]

Resolution: validate each entry's key set as exactly {anchor, disposition, why} plus optional {offset}, with anchor and why nonempty strings and offset an int, reporting violations as problems; add self-test cases for a missing why, an extra key, and a missing anchor. Acceptance: `{"anchor": "...", "disposition": "unreachable"}` fails by name; the committed file still passes.

### tools-16: covcheck has no per-file presence floor: a scope file absent from the lcov is invisible unless it already carries an entry
- Where: tools/covcheck:150-154 (related: covcheck:53-56, 123-126, 170-173; tools/covcheck-expected.json:202; justfile:1006)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (a second file under scope with no SF record and no expectation entry produced no problem from `run()`; enumerating crates/before/src/version/skyline/ gives 39 .rs files, 27 non-test and 12 tests.rs siblings; 12 of the 27 non-test files have no entry in either map: decode, emit, encode, fill/memo, literal, pool_traffic, query, query/web, shape, validate, walk, web_traffic)
- Seen by: adequacy [23], instrument-correctness [51]; refutation: confirmed (correcting [51]'s file counts); history: no-rationale-found (8a9583c5 enumerates the two tamper directions and the whole-report floor; a file dropping out of the lcov was not considered; the only presence pin that exists is the accidental empty key at covcheck-expected.json:202)
- Owner-gated: no

`parse_lcov` collects whatever SF records contain the scope prefix, and both checks iterate that set plus the expectation's files; nothing enumerates the scope directory itself. A kernel file with no entry can drop out of the report (cfg'd out, excluded by an ignore regex, moved) and covcheck reads green; the floor at 150-154 fires only when the whole scope is dark. Doctrine: a liveness floor asserts irreducible work per boundary, and the boundary here is the file. The justfile's GOAL (1006) is per arm.

Evidence:

    53	        if raw.startswith("SF:"):
    54	            path = raw[3:]
    55	            at = path.find(scope)
    56	            current = files.setdefault(path[at:], {"da": {}, "brda": {}}) if at >= 0 else None
   150	    if not any(hits > 0 for data in files.values() for hits in data["da"].values()):
   151	        problems.append(
   152	            "liveness: no covered kernel line in the lcov report; "
   153	            "the coverage run did not exercise the kernel at all"
   154	        )

Resolution: enumerate `Path(root, scope).rglob("*.rs")` at check time and fail by name for every file with no SF record, in both modes; state the presence rule in the docstring; decide deliberately at the scope declaration whether tests.rs siblings are in or out; add a self-test case with a second scope file the lcov omits; then delete the `place/filter.rs: []` entry (tools-18), whose only role this replaces. Acceptance: a coverage run with `--ignore-filename-regex 'skyline/walk\.rs'` makes `just coverage-kernel` fail naming walk.rs; the committed roster passes on a fresh lcov; the self-test case is committed.
Construction: gate `mod walk;` in crates/before/src/version/skyline.rs behind an unlit cfg, or pass cargo-llvm-cov `--ignore-filename-regex 'skyline/walk\.rs'`: the lcov carries no SF record for walk.rs, `files` and `resolved` have no key for it, the floor sees covered lines elsewhere, and covcheck exits 0.

### tools-17: covcheck returns early on an out-of-scope entry, hiding every other finding, against its own stated policy two lines later
- Where: tools/covcheck:212-218
- Class / severity / confidence: correctness / low / high
- Provenance: verified; executed: yes (an expectation carrying `"other/x.rs": []` beside a report with two new uncovered lines returned only `["other/x.rs: expected entry outside the pinned scope 'crates/k/src/'"]`)
- Seen by: none of the four lenses; raised as new by the refutation pass; history: not examined
- Owner-gated: no

The scope check `return`s with a single problem, so a mis-scoped entry hides every hole and every anchor finding, while the comment immediately below says resolution problems must report side by side with the holes "rather than hiding one class behind the other".

Evidence:

   212	    for relpath in entries:
   213	        if not relpath.startswith(scope):
   214	            return [f"{relpath}: expected entry outside the pinned scope {scope!r}"]
   215	    # Resolution problems do not stop the judgment: the comparison still
   216	    # runs over whatever resolved, so a drifted pin reports the anchor
   217	    # findings AND the holes side by side rather than hiding one class
   218	    # behind the other.

Resolution: accumulate the scope problems into `problems`, drop the offending keys from `entries`, and continue to `resolve` and `check`; add a self-test case pairing an out-of-scope entry with a new hole and expecting both messages. Acceptance: the case passes; the comment at 215-218 is true of the scope check too.

### tools-18: covcheck-expected.json carries an empty `place/filter.rs` branch key, the residue of a cured remediation entry
- Where: tools/covcheck-expected.json:202 (related: tools/covcheck:116, 170-173)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git log -S'place/filter.rs' -- tools/covcheck-expected.json` lists only 8a9583c5, where the key held a `remediation` entry anchored `let (flip, step) = self.probe.step();`; the JSON tally shows it is the only empty list in the file); executed: no
- Seen by: scaffolding [12], structure-prose [45]; refutation: confirmed; history: deliberate-but-expired (the entry was removed by aa7c96a0 and the empty list left behind)
- Owner-gated: no

Through `resolve()` (line 116 records empty entry lists) and `check_branches()` (170-173), the key asserts only that the file appears in the branch lcov, which no other file gets and which no `why` states; a reader cannot distinguish it from intent.

Evidence:

   202	  "crates/before/src/version/skyline/place/filter.rs": [],

Resolution: delete the key; if file presence is the intended pin, it belongs in covcheck as the scope-wide rule of tools-16, not in one empty row. Acceptance: the branch map has no empty lists and `just coverage-kernel-branch` is unaffected.

### tools-19: digestshare passes a vacant, missing, or partially drifted corpus, keeps a dead V1 branch, and tracebacks on a headerless file
- Where: tools/digestshare:89-96 (related: digestshare:20-23, 43-49, 71-78, 81; justfile:211-220)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (`python3 tools/digestshare <empty dir>` and `python3 tools/digestshare <nonexistent dir>` both printed nothing and exited 0; a `.snap` without insta's two-`---` header raised `IndexError` at line 47; `grep -l '│' tests/snapshots/*.snap | wc -l` is 0 of 22)
- Seen by: scaffolding [4], adequacy [27], [28], [57], structure-prose [37]; refutation: confirmed (all five; it also verified that every committed capture has at least four header lines, so a per-file `total > 0` floor is a live universal premise); history: no-rationale-found for the vacuity path (the missing-root discipline of 651eb92a predates digestshare's landing 5f140251 and was not adopted); already-known for the V1 half (verification-infra-11 in the rumors review; correct at landing, expired at the V1 retirement 368da2a5, whose re-denomination did not reach tools/)
- Owner-gated: no

The floor fires only when `v2_files` is nonzero, so a snapshot directory that is missing, empty, or renamed yields no output and exit 0; every other lint-tier tool treats a vacant scan as a usage error and says why at the site (doclint:387-394, testdoc:119-126, workflowlint:521-531). The V1 skip (20-23, 48-49, 76-78) guards a render form the tree no longer produces and is the only way a file becomes "not V2", so it doubles as a silent-skip path for any future render containing `│`. The floor is also corpus-wide: one capture whose headers stop matching contributes zero to the totals while its neighbours keep them nonzero, although the recipe's stated purpose (justfile:212-216) is exactly that vocabulary contract. The header split at 47 is a traceback on any file without two `---`.

Evidence:

    47	    body = text.split("---", 2)[2]
    48	    if "│" in body:
    49	        return None
    71	    dirs = [Path(arg) for arg in sys.argv[1:]] or [root / "tests" / "snapshots"]
    89	    if v2_files and (not grand_total or not grand_digest):
    90	        print(
    91	            "digestshare: LIVENESS FAILURE: the corpus has V2 captures but "
    92	            f"{grand_total} wire B / {grand_digest} digest B were parsed; "
    93	            "the renderer's vocabulary has moved out from under this tool.",
    94	            file=sys.stderr,
    95	        )
    96	        return 1

Resolution: refuse a nonexistent directory (exit 2, named); make the floor unconditional (zero files, zero wire bytes, or zero digests exits 1 naming the directory); delete the V1 handling and its prose so every `.snap` is a capture; require `total > 0` per file, failing by name (digests per file stay corpus-wide, since a capture with no listing legitimately has none); turn a file without two `---` into a named problem. Acceptance: `./tools/digestshare <empty dir>` and `<missing dir>` exit nonzero; rewriting one committed capture's `control item N (B bytes)` lines to another spelling fails naming that file; a headerless file is reported, not a traceback; `grep -c V1 tools/digestshare` is 0; the committed corpus still prints its table and TOTAL line.

### tools-20: doclint's summary-length rule reimplements clippy's `too_long_first_doc_paragraph`
- Where: tools/doclint:66-70 (related: doclint:7-25, 116-167; justfile clippy legs)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: assessed (read the rule; the refutation pass reports that `clippy-driver -W help` on an installed toolchain lists `clippy::too-long-first-doc-paragraph` under the nursery group; I did not run the toolchain); executed: no
- Seen by: scaffolding [14]; refutation: confirmed; history: no-rationale-found (`grep -rn too_long_first_doc` over the tree and the agent notes is empty; neither landing nor rewrite commit names the compiler-side lint)
- Owner-gated: yes (removal of an instrument)

Rule 1 (first doc paragraph over a character budget) is what clippy's lint checks on items listed in module pages, with the crate root exempt. The in-house version differs in its 220-character budget and in reaching files the gate's clippy invocations do not compile; the fence tracking, the nbsp rule, the crate-root exemption, and the calibration constant are a maintenance cascade the compiler-side lint carries. Rule 2 (include_str! separation) has no external equivalent and stays. Principle 3: could a maintained tool produce this behavior?

Evidence:

    66	# The longest a summary may render before it stops reading as a one-liner.
    67	# Calibrated to the workspace: the median summary renders to ~180 characters
    68	# (a single sentence over two or three wrapped lines) and stays; the threshold
    69	# catches the multi-sentence paragraphs above it.
    70	MAX_SUMMARY_CHARS = 220

Resolution: owner check: enable `-W clippy::too_long_first_doc_paragraph` on the root and detached-workspace clippy legs, confirm it fires on doclint's summary fixtures, then drop rule 1 (keeping rule 2 and its self-test). If the lint's nursery grade, its compiled-cfg coverage, or its threshold is judged insufficient, record that at MAX_SUMMARY_CHARS as the reason the in-house rule exists. Acceptance: either `just clippy` fails on a 250-character first paragraph and doclint no longer measures summaries, or the constant's comment states why clippy's lint does not suffice.

### tools-21: doclint pins the summary rule through two overlapping fixture suites
- Where: tools/doclint:279-282 (related: doclint:265-278, 294-351)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (`summary_cases`, 3-tuples, and `cases`, named 4-tuples, both drive `long_summaries`; the crate-root exemption is pinned at 276-277 and again at 305-306); executed: no
- Seen by: structure-prose [43]; refutation: confirmed; history: no-rationale-found (bbb9f802 added the 3-tuple suite; d838845c added the named suite while re-pinning the crate-root boundary)
- Owner-gated: no

Two fixture shapes for one function is maintenance weight without added coverage; the named suite is the better shape.

Evidence:

   276	        (f"//! {overlong}\n///\n/// Body.", "lib.rs", []),
   279	    for source, filename, expected in summary_cases:
   280	        actual = [line for line, _ in long_summaries(source.splitlines(), filename)]
   305	        ("crate-root lede exempt", "lib.rs",
   306	         f"{doc(over, '//!')}\nfn f() {{}}\n", []),

Resolution: fold `summary_cases` into `cases` with names, one loop. Acceptance: one fixture table for the summary rule with every existing case present by name.

### tools-22: Two hand-rolled Rust-file walkers read build output and a foreign worktree into the gate
- Where: tools/doclint:370-373 (related: tools/testdoc:19, 66-78; justfile:181, 186; tests/seed_liveness.rs:35; .github/workflows/ci.yml:103-105)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified; executed: yes (reproducing doclint's traversal, `Path(r).rglob("*.rs")` over benches, crates, examples, src, tests, lists 547 files, 2 of them under crates/before/fuzz/target/{aarch64-apple-darwin/release,debug}/build/thiserror-*/out/private.rs; reproducing testdoc's walk over `.` lists 1088 files, 543 under .claude/, which git excludes via .git/info/exclude:7; CACHEDIR.TAG is present in target/, crates/before/fuzz/target, crates/before/surfacecheck/target, and crates/before/wasm32-pins/target)
- Seen by: scaffolding [8], adequacy [24], structure-prose [34]; refutation: confirmed; history: no-rationale-found (the walkers were written independently in 432ac34d and 358c6b1a with no shared policy; tests/seed_liveness.rs:35 carries a third copy with `.claude` added, so the divergence was noticed once and fixed locally; the testdoc half is recorded as verification-infra-9 in the rumors review, whose proposed fix, explicit roots for testdoc, would leave doclint's descent in place)
- Owner-gated: no

doclint takes hand-named roots and walks them with no exclusions, so `crates` includes thiserror's generated code under the fuzz workspace's target, present only on machines that have built fuzz; testdoc excludes `target` by basename but not hidden directories, so `just testdoc .` reads another agent's uncommitted worktree. A gate verdict must be a function of the committed tree (Principle 6). Two sibling lints with two discovery routines, and a third copy in a test, is the duplicated-capability pattern that produces the divergence; `git ls-files` already knows the tracked set. ci.yml:103-105 restores the fuzz workspace's target from cache, so the runner is not necessarily exempt (an inference from the workflow, not verified).

Evidence:

   370	def rust_files(root):
   371	    if root.is_file():
   372	        return [root] if root.suffix == ".rs" else []
   373	    return sorted(root.rglob("*.rs"))

    (tools/testdoc)
    19	IGNORED_DIRECTORIES = {".git", "node_modules", "target"}

    (tests/seed_liveness.rs)
    35	const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".claude"];

Resolution: one discovery routine shared by doclint and testdoc (a small helper module in tools/, offered to seed_liveness.rs as well): `git ls-files -z --cached --others --exclude-standard -- '*.rs'` restricted to the given roots (tracked plus untracked-not-ignored, so pre-add work is still linted while target/, .claude/worktrees, and other excluded trees are invisible), or, if git is not wanted in the lint tier, prune directories carrying CACHEDIR.TAG and hidden directories. Keep each tool's missing-root guard; pin the routine in both self-tests with a fixture tree containing `target/x.rs` beside a CACHEDIR.TAG and `.hidden/y.rs`. Acceptance: doclint and testdoc report the same file set on the same tree; with the two generated files and the `.claude/worktrees` files present, neither tool visits them.
Construction: write a `///` paragraph of 300 characters into a scratch `.rs` under crates/before/fuzz/target/ and run `./tools/doclint crates`: it fails; on a fresh checkout the same commit passes. Write an undocumented `#[test]` into `.claude/worktrees/w/src/t.rs` and run `./tools/testdoc .`: it reports it.

### tools-23: fuelscape-claims has no liveness floor and no self-test
- Where: tools/fuelscape-claims:30-49 (related: justfile:175-176, 700-703, 723-726)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified; executed: yes (a scratch copy of the tool and bundle with `index.json` set to `{"ops": []}` printed "fuelscape-claims: 0 claims accepted by the widget's own rule" and exited 0; the committed index carries 104 ops)
- Seen by: scaffolding [10], structure-prose [36], instrument-correctness [56]; refutation: confirmed (noting `fuelscape-verify` byte-diffs the committed directory against regeneration in ci, which bounds an emptied index landing silently but demonstrates nothing about `accepts` refusing a bad claim); history: no-rationale-found (61f05692 recorded one live catch and no floor; every other tool gained a self-test at or after landing under the recipe convention at justfile:175-176)
- Owner-gated: no

The loop over `index.ops` is the only failure source; a zero-length list passes as a clean sweep, and nothing committed demonstrates that a known-bad claim (the header's own example, `log n` over a dataset whose smallest column is n=1) is refused. Principle 2: a meter over a counter passes vacuously when the counter goes dark; Principle 6: every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

    33	for (const op of index.ops) {
    48	if (failures) process.exit(1);
    49	console.log(`fuelscape-claims: ${index.ops.length} claims accepted by the widget's own rule`);

Resolution: fail when `index.ops.length === 0` or when any `doc.op.claim`/`sizes` is missing, naming the file; add `--self-test` asserting `Fuelscape.accepts` refuses a syntax error and the `log n` at n=1 case and accepts a linear claim; run the self-test first in the recipe like the other tools. Acceptance: `{"ops": []}` exits 1 naming the floor; `tools/fuelscape-claims --self-test` fails when `accepts` is stubbed to `() => ({})`; the committed index still passes with its count printed.

### tools-24: manifestlint's self-test normalizes one fixture spelled as a bare tuple
- Where: tools/manifestlint:140-143 (related: manifestlint:149-150)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (the target-table case is the only expectation that is a bare tuple; lines 149-150 exist for it alone); executed: no
- Seen by: scaffolding [18], structure-prose [44]; refutation: confirmed; history: no-rationale-found (both born in 84044759)
- Owner-gated: no

A fixture table should have one shape; the normalization branch is code that serves an inconsistency in its own data.

Evidence:

   140	        (
   141	            "[target.'cfg(unix)'.dependencies]\na = \"1\"",
   142	            ("target.cfg(unix).dependencies", "a"),
   143	        ),
   149	        if isinstance(expected, tuple):
   150	            expected = [expected]

Resolution: write the expectation as `[("target.cfg(unix).dependencies", "a")]` and delete the `isinstance` branch. Acceptance: no `isinstance` in `self_test`; `./tools/manifestlint --self-test` passes.

### tools-25: memwatch's per-process kill for test and bench binaries keys on a `target/` path component that cargo's build.build-dir removes
- Where: tools/memwatch:82-87 (related: memwatch:11-13; justfile:101-103)
- Class / severity / confidence: correctness / low / high
- Provenance: verified; executed: yes (the `is_build_proc` case statement, run in isolation, returns 1 for `/Volumes/forge/build/70/874132ca3784ca/debug/deps/before-1234 --exact` and 0 for the same path under `target/`; `~/.cargo/config.toml` sets `build-dir = "/Volumes/forge/build/{workspace-path-hash}"`; the repository's `.cargo/` holds only mutants.toml; the only `target/*/deps` directories in the tree belong to the detached fuzz workspace, and the root `target/debug` has no `deps/`)
- Seen by: instrument-correctness [52]; refutation: confirmed (resolved the build directory for this workspace via read-only `cargo metadata`); history: deliberate-but-expired (the glob was written under cargo's default layout in 90df4227; the build-dir setting lives outside version control and cannot be dated)
- Owner-gated: no

Under `build.build-dir`, set on the development machine these limits are sized against, test and bench executables live at `<build-dir>/<profile>/deps/<bin>` with no `target` component, so the branch never fires for them and only the swap backstop remains; justfile:101-103 promises "a runaway test fails the build with the offender named". rustc and clippy-driver still match by name, so the monomorphization case the tool was built for is unaffected. Principle 1: "test binaries under target/" is a path-shape premise the environment falsifies.

Evidence:

    82	is_build_proc() {
    83	    case "$1" in
    84	        *rustc*|*clippy-driver*|*build-script-build*|*/target/*/deps/*) return 0 ;;
    85	        *) return 1 ;;
    86	    esac
    87	}
    11	#   1. Per-process: any build-related process (rustc, clippy-driver, build
    12	#      scripts, test binaries under target/) whose resident size exceeds

Resolution: match on `*/deps/*` regardless of the directory above it (test and bench binaries are only ever emitted into a `deps/` directory), or resolve `build_directory` once via `cargo metadata --no-deps` and match on that prefix; state the premise at the case arm and at line 12. Acceptance: on a machine with build.build-dir set, a test binary that allocates past PROC_LIMIT_GB is killed and logged with its pid and command.

### tools-26: memwatch's kill path has no committed demonstration
- Where: tools/memwatch:100-115 (related: memwatch:82-87, 131-158; justfile:101-105)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (grep for any invocation of memwatch other than as a recipe wrapper finds none; the script has no `--self-test`); executed: no
- Seen by: adequacy [30]; refutation: confirmed (a self-test is feasible: `PROC_LIMIT_GB=0` makes every process a candidate and `is_build_proc` is name-matched, so a fake `*rustc*`-named child that allocates demonstrates the path); history: no-rationale-found (the kill and freeze paths were demonstrated by hand and the demonstrations live only in commit messages 38aaad3a and ccbcede8)
- Owner-gated: no

The watchdog wraps the codegen-running recipes, and its targeting (the awk parent-chain walk, the per-pid re-check, the build-proc filter) is the most intricate code in tools/, yet nothing committed ever drives a candidate over the limit. A rot in the `ps` column parse or the chain walk reads as a green gate until the next runaway. Principle 6: every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

   100	own_procs() {
   101	    ps -axo pid=,ppid=,rss= | awk -v root="$ROOT" '
   109	                    if (a == root) { print p, rss[p]; break }

Resolution: add a `--self-test` that runs memwatch with a tiny `PROC_LIMIT_GB` around a child whose command matches the filter (a copied `python3` named `rustc-selftest`, or a script) that allocates past the limit, asserting the `KILL pid` note and a nonzero exit; and a second case where the child is named outside the filter and survives; run it at the head of one recipe; degrade explicitly (skip with a message) where `ps -o rss` differs. Acceptance: both cases pass on macOS; the construction below fails the self-test.
Construction: swap the fields at line 109 to `print rss[p], p`: every recipe still passes, and pid numbers are compared against `lim_kib`, so the next runaway is never killed.

### tools-27: Prose restates the pinned cargo-mutants release, and edits left orphaned lines
- Where: tools/mutantcheck:33-36 (related: mutantcheck:18-20; .cargo/mutants.toml:75-76; tools/mutantcheck-expected.json:2; tools/benchjudge:23, 128; justfile:308-309)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn '27\.1\.0' tools .cargo justfile` hits mutantcheck:33, the JSON pin at line 2, and mutants.toml:76); executed: no
- Seen by: scaffolding [17], instrument-correctness [58]; refutation: confirmed; history: no-rationale-found (11363f89 removed the hand-copied version from the justfile, "the pin of record is mutantcheck-expected.json", and left these two; the orphaned lines are residue of specific edits that did not reflow)
- Owner-gated: no

The version of record is `tool` in tools/mutantcheck-expected.json; two prose sites restate it and go stale on the very bump the pin exists to force. Several docstrings carry short lines left mid-clause by edits.

Evidence:

    33	    tool declines to apply pins suppressed 0 (cargo-mutants 27.1.0
    34	    applies name filters to no delete-field mutant, so a delete-field
    18	else holds the surviving regexes to the live mutant inventory: a
    19	pattern whose site is
    20	refactored away silently excludes nothing, and a function-scoped pattern

    (.cargo/mutants.toml)
    75	# measured margins) — and unexcludable regardless, since cargo-mutants
    76	# 27.1.0 applies name filters to no delete-field mutant.

Resolution: write "the pinned cargo-mutants release (tools/mutantcheck-expected.json, `tool`)" at both sites, or keep the sentence only in the roster header; reflow mutantcheck:19-20, benchjudge:23 and :128, justfile:308-309. Acceptance: `grep -n '27\.1\.0' tools/mutantcheck .cargo/mutants.toml` is empty; no docstring line ends mid-clause at fewer than about forty columns.

### tools-28: mutantcheck never accounts for mutants missing from the filtered listing, and reads only `exclude_re`
- Where: tools/mutantcheck:156-161 (related: mutantcheck:38-41, 47-55, 377-378; .cargo/mutants.toml:78-82; tools/mutantcheck-expected.json)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (`run()` over raw {a, b, d, e}, filtered {e}, two patterns claiming a and b with pins listed = suppressed = 1, returned no problems although d vanished from the filtered listing with no pattern claiming it; every committed entry has listed == suppressed, 14 of 14, so a widened exclusion cannot move any pinned number; the config today carries only additional_cargo_args, exclude_re, test_tool, test_workspace)
- Seen by: instrument-correctness [50]; refutation: confirmed; history: no-rationale-found (the docstring's model of record names exactly one accepted residual, the count-preserving swap; 3ed79544, which added the executable campaign keys to the config, changed only the docstring)
- Owner-gated: no

The checker asserts filtered is a subset of raw but never that every mutant in raw minus filtered is claimed by a pinned pattern, and it reads only `exclude_re`. A file- or crate-level restriction added via `exclude_globs`, `examine_globs`, or `examine_re` shrinks the campaign of record while every pinned listed/suppressed count stays exactly where it was (listed is counted over the `--no-config` raw listing, and suppressed already equals listed for every entry), so the gate's mutants-list leg passes. Principle 6, against the docstring's promise at 38-41 that the judgment is tamper-evident in both directions.

Evidence:

   156	    extra = filtered - raw
   157	    if extra:
   158	        problems.append(
   159	            f"{len(extra)} filtered-listing mutant(s) absent from the raw "
   160	            "listing; the two captures are not from the same tree"
   161	        )
   377	    config = tomllib.loads(Path(args.config).read_text(encoding="utf-8"))
   378	    patterns = config.get("exclude_re", [])

Resolution: after parsing both captures, compute the unclaimed set (mutants in raw minus filtered matched by no pinned pattern) and fail by name for each; refuse any filtering key in the config beyond `exclude_re` (`exclude_globs`, `examine_globs`, `examine_re`) as an unpinned campaign restriction; add a self-test case: raw {a, b, c, d}, filtered {c}, d matched by no pattern, must red with "suppressed by no pinned pattern". Acceptance: with `exclude_globs = ["**/watermark.rs"]` appended to .cargo/mutants.toml and the captures regenerated, `just mutants-list` fails naming the watermark mutants or the key; the current tree stays green; the new case is committed.

### tools-29: Em-dashes in comments and in nine printed diagnostics
- Where: tools/mutantcheck:195-197 (related: mutantcheck:190; tools/covcheck:92; tools/citecheck:411, 427, 844; tools/workflowlint:275, 285, 545)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -c '—'` per tool: benchjudge 38, citecheck 16, mutantcheck 16, doclint 10, workflowlint 10, memwatch 5, covcheck 4, fuelscape-claims 4, manifestlint 4, readme 2, testdoc 0, digestshare 0; the nine printed-string sites listed under related); executed: no
- Seen by: structure-prose [48]; refutation: confirmed; history: already-known (a pending owner ruling, decision 3 of the rumors review, recommends ruling once and sweeping mechanically with a tools/ check; an earlier review named the assert-message case a house rule)
- Owner-gated: yes (pending ruling)

Doctrine under Principle 5: colons or semicolons over em-dashes in log messages and comments alike, for terminal compatibility and sentence flow. What this partition adds to the pending ruling is the Python-docstring register and the nine strings that land in gate and CI logs.

Evidence:

   195	                f"counts moved for {pattern}: listed {listed} (pinned "
   196	                f"{want.get('listed')}), suppressed {suppressed} (pinned "
   197	                f"{want.get('suppressed')}) — a swallowed new mutant, a "

Resolution: apply the ruling to tools/: colons or semicolons in `#` comments, docstrings, and every printed string; the self-test needles match substrings, so most edits are needle-safe (check citecheck:844 and workflowlint:545, whose needles lie elsewhere). Acceptance: `grep -c '—' tools/*` is 0 for every tool, or the owner records a docstring exception.

### tools-30: readme's crate roster is hand-maintained with no totality check, and `image_refs` is never exercised by the self-test
- Where: tools/readme:78-108 (related: readme:144-154, 184-203, 238; tools/manifestlint:83-90; Cargo.toml:2-8)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rl 'cargo-rdme start' --include='*.md'` returns exactly README.md, crates/before/README.md, crates/suanpan/README.md, crates/rumors-tracing/README.md, matching the four CRATES entries; the workspace lists five members plus the root package; crates/before-viz/README.md exists without markers; every `self_test` case at 238 passes `{}` for image_refs); executed: no
- Seen by: scaffolding [11], structure-prose [35]; refutation: confirmed; history: no-rationale-found (the roster began as two crates in 2a3a752c and grew by hand; the stripper self-test landed with `{}` in every case)
- Owner-gated: no

Nothing checks that the set of marker-bearing READMEs equals CRATES, so a new crate or a README that gains markers is silently never derived or checked; manifestlint in the same directory already shows the idiom (ask `cargo metadata` for the members). The comment-wrapped image-reference unwrap at 146-151 is untested although the docstring (17) says the self-test pins the forms rewritten.

Evidence:

    78	CRATES = [
    79	    Crate(name="rumors", readme="README.md"),
    80	    Crate(
    81	        name="rumors-tracing",
    82	        readme="crates/rumors-tracing/README.md",
    83	        rdme_args=["-w", "rumors-tracing"],
    84	    ),
   238	        actual = strip_intra_doc_links(source, {})

Resolution: derive the crate set from `cargo metadata --no-deps` packages whose README contains the markers, keeping only `image_refs` as per-crate data; or, minimally, assert in `check()` that the marker-bearing README set equals CRATES and fail by name. Add a self-test case for the comment-wrapped image reference. Acceptance: adding the markers to crates/before-viz/README.md with no roster change makes `just readme-check` derive it or fail naming it; `tools/readme self-test` has an `image_refs` case.

### tools-31: House-style inconsistencies across the tool set
- Where: tools/testdoc:111-113 (related: tools/covcheck:312-314; tools/benchjudge:807, 927-929; tools/doclint:87-96, 370; tools/workflowlint:503; tools/readme:172, 188; tools/digestshare:45)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n 'self-test ok' tools/*` lists citecheck:825, doclint:379, manifestlint:154, mutantcheck:368, workflowlint:514 only; `read_text()` without `encoding=` at digestshare:45, readme:172, readme:188, and a `write_text` without it at benchjudge:807); executed: yes (`python3 tools/benchjudge --self-test` printed nothing and exited 0)
- Seen by: structure-prose [49]; refutation: confirmed (correcting: argv is hand-parsed in six tools, not five); history: no-rationale-found (each convention arrived with its tool's landing commit)
- Owner-gated: no

Self-test success is announced by five tools and silent in three (testdoc, covcheck, benchjudge), so a recipe log cannot show the self-test ran; `doc_marker` is documented by a `#` comment and `rust_files`/`yaml_files` have no docstring while every other function does; readme and digestshare read text without an encoding (locale-dependent before Python 3.15, and readme's derivation is byte-pinned); argv is parsed with argparse in four tools and by hand in six.

Evidence:

   111	    if argv[1:] == ["--self-test"]:
   112	        self_test()
   113	        return 0

    (tools/readme)
   172	    rendered = readme_path.read_text()

Resolution: one convention per axis: every self-test prints `<tool>: self-test ok`; every text read and write passes `encoding="utf-8"`; a docstring on every def; one argv style. Acceptance: `grep -c 'self-test ok' tools/*` is 1 for every tool with a self-test; `grep -n 'read_text()\|write_text(' tools/*` shows an encoding on every call.

### tools-32: workflowlint spells the interpreter roster twice
- Where: tools/workflowlint:86-94 (related: workflowlint:146-161, 164-174)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (`INTERPRETERS` and the alternation inside `SUBST_FETCH` repeat the same nine names plus the python form; the pipeline check at 170 and the substitution check at 174 consult different spellings); executed: no
- Seen by: structure-prose [38]; refutation: confirmed; history: no-rationale-found (both spellings born in facc7550)
- Owner-gated: no

Adding an interpreter to one and not the other makes the two checks disagree: add `pwsh` to `INTERPRETERS` and `curl ... | pwsh` is caught while `pwsh <(curl ...)` is not. Named constants over restated literals.

Evidence:

    86	INTERPRETERS = {"sh", "bash", "dash", "zsh", "fish", "ksh", "node", "perl", "ruby"}
    87	PYTHON = re.compile(r"python[0-9.]*$")
    90	SUBST_FETCH = re.compile(
    91	    r"(?:(?:/[\w./-]+/)?(?:sh|bash|dash|zsh|fish|ksh|node|perl|ruby"
    92	    r"|python[0-9.]*)|\$SHELL|\$\{SHELL\})\b[^|;&]*"

Resolution: build the alternation from the set (`"|".join(map(re.escape, sorted(INTERPRETERS)))` joined with the python form) and compile `SUBST_FETCH` from it. Acceptance: each interpreter name appears once in the file; a self-test case adds a name to the set and shows both the pipeline and substitution forms red.

### tools-33: workflowlint's interpreter recognizer stops at the first non-prefix token, so `| sudo -E bash -` and `| env -i sh` pass as fetch-without-execute
- Where: tools/workflowlint:151-160 (related: workflowlint:34-37, 40-43, 45-46, 405-473; justfile:188-196)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (`fetch_feeds_interpreter` returned False for `curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -`, `curl https://x/i.sh | sudo -u runner sh`, and `curl https://x/i.sh | env -i sh`, and True for `| sudo bash` and `| bash -s -- --yes`; `run()` over a workflow fixture carrying the nodesource line returned no problems)
- Seen by: scaffolding [0]; refutation: confirmed; history: no-rationale-found (facc7550's fixtures at 426 and 444 are bare-prefix only; the docstring's own over-matching policy at 40-43 argues for the fix)
- Owner-gated: no

`invokes_interpreter` returns on the first token that is neither a listed prefix word nor a `VAR=value` assignment, so any flag on `sudo` or `env` makes `base` the flag and the segment reads as a non-interpreter. The docstring enumerates `| env sh` and `| sudo bash` as caught, and the flag-bearing spellings are the ones real installers use; they sit inside the tool's stated one-pipeline boundary (45-46), not the documented two-step exception. Principle 6: the cheapest passing artifact must be the intended one.

Evidence:

   151	    for token in segment.strip().split():
   152	        word = token.strip("\"'")
   153	        if word in ("sudo", "env", "exec", "command", "nice", "time"):
   154	            continue
   155	        if "=" in word and not word.startswith(("<", "$")):
   156	            continue  # an env-style VAR=value prefix
   157	        if word in ("$SHELL", "${SHELL}"):
   158	            return True
   159	        base = word.rsplit("/", 1)[-1]
   160	        return base in INTERPRETERS or bool(PYTHON.fullmatch(base))

Resolution: adopt the tool's own over-matching policy: after a prefix word, skip flag tokens (those starting with `-`) and their arguments, or treat a post-fetch segment as fetch-execute when an interpreter token appears anywhere in it after stripping prefix words, flags, and assignments. Add `curl ... | sudo -E bash -`, `curl ... | sudo -u runner sh`, and `curl ... | env -i sh` to the self-test's red fixtures. Acceptance: `./tools/workflowlint --self-test` fails on the current recognizer with the three new fixtures and passes after the fix; the fetch-to-a-file green fixture stays green.

## Positives

- benchjudge's self-test drives `main` end to end on synthetic criterion trees and asserts exit codes 0/1/2 (lines 702-923), so the `return 0 on red` mutation and the dark-tripwire path are both convicted in `--self-test`; both ceiling-laundering attacks are pinned as constructed inputs (857-884). The ceiling class rides the sidecar and is asserted at every sidecar write (sidecar.rs:175-180); the derived constants check out (I recomputed `fit_noise_band(1.7)` = 0.0876 and `fit_noise_band(1.3)` = 0.0523, as the comments state).
- citecheck's four layers (extraction floors, spelling totality with an entry-count denominator, the shadow guard, and a fabricated citation pushed through the live resolver on every run) are a model of a checker that cannot go dark quietly, and the rejected alternative is recorded with its reason (59-68).
- mutantcheck refuses colored and polluted captures by name before structural parsing, pins the cargo-mutants version, never writes its own pin, prints the observed table for a reviewed re-pin, and states its accepted residual (47-55) and its regex dialect boundary (57-65) with the reason a full-name pin was rejected.
- covcheck anchors entries on source text with offsets, fails closed on vanished, ambiguous, or out-of-file anchors, refuses invented categories, and defers DA=0 lines to the line pin in branch mode; the scaffolding lens reports every one of the 34 committed anchors resolves exactly once against today's sources.
- workflowlint's `USES_ANYWHERE` totality turns every unreadable `uses:` shape into a loud failure, its logical-line joining scans split pipelines whole, and its `swap` helper fails on fixture rot (322-325).
- doclint's include_str! rule carries a genuine rendering derivation (the `</p>` landing mid-SVG) that no external tool checks, with fixtures for every shape of invisible separator; doclint, testdoc, and workflowlint refuse a missing root and say why at the site, and doclint's self-test pins that red path too (353-367).
- readme's `RDME_VERSION` pin (2.1.0) and ci.yml:86's `cargo-rdme@2.1.0` agree, and the tool refuses a mismatch naming both versions and the install command; `check` never mutates the committed file.
- memwatch's process snapshot is numeric-only with a per-pid re-check of size and command before any signal, defeating argv injection and pid recycling, with the residual kill-by-pid race stated exactly (132-139); the swap-abort path freezes the tree before killing so no compile survives orphaned.
- manifestlint asks cargo for member discovery instead of walking directories, the idiom tools-30 asks readme to copy.
- The justfile's gate-streams verdict requires every launched stream to record ok or failed (409-413, 474-481), so a stream killed from outside fails the gate rather than vanishing.

## Open questions for Finch

1. The bench judge (tools-5): do you regard cost that is not instructions (bulk-memory operations fuel prices as one unit, native codegen, memory hierarchy) as a failure class worth an instrument at the board's byte scales? Recommendation: decide this first, because tools-6, tools-7, and tools-12 and the roster test all go away under exit (b); if the answer is yes, exit (a)'s tripwire is the acceptance artifact.
2. covcheck's `remediation` disposition (tools-14): excise, or keep with a positive ruling recorded at the declaration? Recommendation: excise; the roster is empty and the 08-07 ruling's principle is general.
3. The bench roster's `red` class (tools-6): bind it to a sidecar-declared tripwire set? Recommendation: yes, and rename the key so an awaiting-cure entry needs a schema change.
4. digestshare (tools-3, tools-19): the tool measures rumors' wire corpus and lives in tools/. Recommendation: fix the floor regardless (tools-19); move the leg to conveniences unless the ratio gets a committed consumer.
5. covcheck's scope includes the twelve tests.rs siblings under skyline/, so uncovered test-helper lines are held to the same pin as kernel lines. Recommendation: state the decision at the scope declaration when landing tools-16; keeping them in is fail-closed.
6. Three mutant exclusions pin file:line:col (mutantcheck-expected.json:12, 28, 56; mutants.toml:117, 142, 150) and re-redded three times on 2026-08-18. The header at mutants.toml:39-46 accepts the fail-open drift knowingly, so this is not filed as a finding. Recommendation: where extracting the equivalent occurrence into a named helper is cheap, do it so the pattern can anchor on `in <fn>`; otherwise leave the ruling as is.
7. Should tools/ grow a small shared helper module (file enumeration for tools-22, self-test scaffolding)? Recommendation: yes for file enumeration; otherwise each tool stays self-contained.
8. The vocabulary rulings pending in the rumors review ("honest", em-dashes) should apply to tools/ in the same sweep (tools-8, tools-29).
9. Procedural: my module loads wrote `tools/__pycache__/benchjudgecpython-314.pyc` into the untracked `tools/__pycache__/` directory another reviewer had created. I left it in place; the directory is a deletable byproduct.

## Dropped

- Lens [15] (tests/bench_judge_roster.rs mirrors two constants): refuted. The roster test is the doctrine's prescribed tamper-evidence form and the only gate-cadence confrontation of a `just all`-cadence artifact (the JSON is read only by `just bench-judge`, TEXT_CEILING_CELLS is asserted only when a bench runs); moot only if the judge is retired under tools-5.
- Lens [32] (the tripwire recipe never hands `--tip` to the judge): refuted. justfile:857 ends with `--tip $(git rev-parse HEAD)`; the lens's quote was truncated. The secondary observation (a dirty-tree run is stamped with the clean commit) is below the bar.
- Lens [9] (three line:col-pinned exclusion patterns re-red the gate on line drift): deliberate and documented at .cargo/mutants.toml:39-46, which accepts the fail-open drift and puts refactoring first in its own ladder; moved to open question 6.
- Lens [1]/[25]: duplicate of tools-4. Lens [3]/[22]/[33]: duplicate of tools-14. Lens [4]/[27]/[37]/[28]/[57]: duplicate of tools-19. Lens [5]/[39]/[53] and the ghost half of [31]: duplicate of tools-9. Lens [6]/[21]: duplicate of tools-6. Lens [8]/[24]/[34]: duplicate of tools-22. Lens [10]/[36]/[56]: duplicate of tools-23. Lens [11]/[35]: duplicate of tools-30. Lens [12]/[45]: duplicate of tools-18. Lens [13]/[26]/[54]: duplicate of tools-1. Lens [17]/[58]: duplicate of tools-27. Lens [18]/[44]: duplicate of tools-24. Lens [19]/[31]/[40]: duplicate of tools-12 (reframed to deduplication per d2a9d04e). Lens [23]/[51]: duplicate of tools-16. Lens [42]/[59]: duplicate of tools-10. Lens [46]/[47]: duplicate of tools-8.
