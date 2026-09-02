# Sweep deps: Dependency, feature, and build-script audit

## Method and coverage

This is the verification pass over the deps sweep's sixteen findings, at
commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 on a clean tree. For each
finding I opened the cited sites with line numbers, grepped use sites, and
checked the recorded rationales: `git log`/`git show` on the relevant
commits, `.agent-notes/2026-08-13-before-fuelscape-rustdoc/` (the build.rs
design), `.agent-notes/2026-08-04-perf-probe/` (the probe examples),
`crates/before/AGENTS.md`, and the deny.toml, justfile, and
rust-toolchain.toml headers. Mechanical checks run: `git ls-files --
'*Cargo.lock'` (six lockfiles), per-lock grep of `wasmtime`, `dashu-int`,
`borsh`, `bytes`, `thiserror` versions, the RustSec advisory files for
RUSTSEC-2026-0268/0269 in `~/.cargo/advisory-db`, a scan of the root
Cargo.lock's dependency arrays for what reaches rand 0.9 and thiserror 1,
grep counts of `suanpan::UBig` and `dashu_int::UBig` spellings, grep of
`(PROG|COV)-[0-9]+` across the tree including `.agent-notes/`, grep of
`derive(Serialize|Deserialize)` across before, `git ls-files` sizes of
`results/`, `reference/`, `scripts/`, and the dashu-int 0.5.0 feature table
from the registry source. Two external documents were fetched to settle
mechanism claims: cargo-deny's bans configuration page (the
`multiple-versions-include-dev` default) and the rustc book's check-cfg
page (whether command-line `--cfg` values are checked). CI history was read
with `gh run list`/`gh run view` for the last six `ci` runs on main and the
failed job's log.

Not run: cargo, just, cargo-audit, cargo-deny, or any build; the brief
forbade them, and no finding needed a test to settle. The two constructions
below that would settle by running (the build.rs rerun gap and the
`expect("validated")` panic) require editing files, so they stay unexecuted
and are stated as constructions. What this pass cannot see: whether
`cargo audit` would flag anything beyond the wasmtime entry in the omitted
lock, and whether `cargo deny` with dev duplicates included would report
anything beyond the four crate pairs found by lock inspection.

All sixteen findings survive; three are reframed (deps-4 becomes an
owner-gated design question against a recorded decision, deps-6's
operational edge is settled by CI history, deps-7 is marked owner-gated).
One out-of-scope observation from the CI log is recorded under open
questions with an explicit disposition.

## Findings

### deps-1: supply-chain leg audits five of six lockfiles; the omitted wasm32-pins lock carries advisory-listed wasmtime 47.0.3
- Where: justfile:328-352 (related: justfile:390, deny.toml:4, rust-toolchain.toml:2-4, crates/before/wasm32-pins/Cargo.lock:952-953, crates/before/wasm32-pins/harness/Cargo.toml:14-17)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (`git ls-files -- '*Cargo.lock'` lists six locks; grep of the wasm32-pins lock; the two advisory files' `patched` ranges; `git log --diff-filter=A` dates; `git show 4e64a4fb`); executed: no (cargo audit not permitted; the version-to-range comparison is mechanical)
- Verification: confirmed; history: deliberate-but-expired (the recipe's enumeration was complete when written on 2026-07-31 in 4da4779f; wasm32-pins landed 2026-08-18 in eb6ba627 and no enumeration moved)
- Owner-gated: no

The recipe promises an audit of every lockfile but names the detached
workspaces by hand and stops at four, so `crates/before/wasm32-pins/Cargo.lock`
is never audited. That lock resolves wasmtime 47.0.3, which RUSTSEC-2026-0268
and RUSTSEC-2026-0269 both list with `patched = [..., ">= 47.0.4"]`; the
2026-08-31 advisory bump (4e64a4fb) reached fuzzfit and fuelscape only and
its message counts "all five lockfiles". The same four-workspace roster is
restated at justfile:330, justfile:390, deny.toml:4, and
rust-toolchain.toml:3.

Evidence:

       329	# cargo-audit sweeps every lockfile in the repository — the root workspace
       330	# and each detached workspace (fuzz, fuzzfit, fuelscape, surfacecheck) —
       ...
       347	    cargo audit
       348	    cargo audit --file crates/before/fuzz/Cargo.lock
       349	    cargo audit --file crates/before/fuzzfit/Cargo.lock
       350	    cargo audit --file crates/before-fuelscape/Cargo.lock
       351	    cargo audit --file crates/before/surfacecheck/Cargo.lock
       352	    cargo deny --workspace check bans

    crates/before/wasm32-pins/Cargo.lock
       952	name = "wasmtime"
       953	version = "47.0.3"

    ~/.cargo/advisory-db/crates/wasmtime/RUSTSEC-2026-0269.md
    patched = [
        ">= 24.0.13, < 25.0.0",
        ">= 36.0.14, < 37.0.0",
        ">= 46.0.3, < 47.0.0",
        ">= 47.0.4",
    ]

    commit 4e64a4fb (2026-08-31): "just supply-chain clean across
    all five lockfiles with no warnings."

    deny.toml
         4	# detached workspaces (fuzz, fuzzfit, fuelscape, surfacecheck) are
    rust-toolchain.toml
         3	# directory, so the fuzz, fuzzfit, fuelscape, and surfacecheck

Reachability, stated plainly: the harness takes wasmtime with
`default-features = false, features = ["cranelift", "runtime"]`
(harness/Cargo.toml:14-17) and no WASI, and both advisories concern WASI
paths, so the vulnerable code is most likely not compiled. cargo-audit judges
by version, which is the rule the gate adopted at justfile:332 ("a
vulnerability anywhere fails the gate"), and by that rule the gate is green
over a lock it should refuse. The principle breached is the no-hand-maintained
enumeration rule: five prose and recipe restatements of one roster all
drifted silently when a fifth detached workspace landed.

Resolution: enumerate lockfiles mechanically in the recipe body (a loop over
`git ls-files -z -- '*Cargo.lock'` feeding `cargo audit --file`), so a new
detached workspace is audited the commit its lock lands; re-denominate the
four prose enumerations to "every committed Cargo.lock"; bump wasm32-pins to
wasmtime 47.0.4 (`cargo update -p wasmtime` inside the workspace) and run
`just wasm32-pins` to confirm every pin's outcome is unchanged. Acceptance:
`just supply-chain` at HEAD fails naming RUSTSEC-2026-0269 in
crates/before/wasm32-pins/Cargo.lock; after the bump it passes; a seventh
lockfile added anywhere in the tree is audited with no recipe edit.

### deps-2: the deny leg's stated scope (one version per crate over the whole graph) exceeds what it enforces; dev-only duplicates pass by cargo-deny's default
- Where: justfile:334-339 (related: deny.toml:1-24, Cargo.lock:1864-1876 and 1885-1896 and 2515-2525, Cargo.toml:142 and 150 and 168)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (lock inspection with a scan of dependency arrays; cargo-deny bans documentation fetched; `git show 4e64a4fb` attests the check passed with these entries present); executed: no (cargo deny not permitted)
- Verification: confirmed; history: no-rationale-found (deny.toml's header states the whole resolved graph; no comment names a dev exemption)
- Owner-gated: no (the policy choice is the owner's, but prose and mechanism must agree either way)

The root lock carries rand 0.8.6 and 0.9.4, rand_chacha 0.3.1 and 0.9.0,
rand_core 0.6.4 and 0.9.5, and thiserror 1.0.69 and 2.0.18. The 0.9 rand
line enters only through proptest 1.11 (a dev-dependency of every member);
thiserror 1 enters only through rumors' dev-dependency ratatui (via termwiz,
filedescriptor, and the wezterm crates). cargo-deny's documented default
excludes crates reachable only over dev edges from duplicate detection, so
`just supply-chain` passes while the header and recipe comment promise one
version per crate across the full closure and a roster of every tolerated
duplicate.

Evidence:

    justfile
       334	# for triage without failing. cargo-deny holds the root workspace's
       335	# resolved graph (all members, all features, all targets) to one version
       336	# per crate; deny.toml is the roster of record, every tolerated duplicate
    deny.toml
         2	# which runs `cargo deny check bans`). Scope: the root workspace's full
         3	# feature-and-target closure — exactly the graph Cargo.lock records. The
        24	skip = []
    Cargo.lock
      1864	name = "rand"
      1865	version = "0.8.6"
      1875	name = "rand"
      1876	version = "0.9.4"
      2515	name = "thiserror"
      2516	version = "1.0.69"
      2524	name = "thiserror"
      2525	version = "2.0.18"
    Cargo.toml (rumors)
       142	rand = { workspace = true }
       150	proptest = { workspace = true }
       168	ratatui = { workspace = true }

    cargo-deny bans configuration, `multiple-versions-include-dev`:
    "If `true`, `dev-dependencies` are included when checking for multiple
    versions of crates. By default this is false, and any crates that are
    only reached via dev dependency edges are ignored when checking for
    multiple versions."

The header's own cost argument ("A crate resolved at two versions compiles
twice, doubles its audit surface") is paid by every gate test build for
these four pairs, and a reader trusting the comment believes rand and
thiserror are converged. Principle 8 (verified vs told) and the
no-unrostered-exemption rule are what this breaches.

Resolution: choose and make the text match. Either (a) set
`multiple-versions-include-dev = true` under `[bans]` and roster the
holdouts with their reasons (rand 0.8 held by the workspace pin against
proptest's 0.9; thiserror 1 through ratatui's termwiz chain), which restores
the header's promise and lets the roster shrink visibly as they converge; or
(b) state the dev-edge exemption in both comments. The rand convergence path
(workspace `rand`/`rand_chacha` to 0.9) touches rumors' normal dependency at
Cargo.toml:142 and reseeds every corpus drawn through `gen_range`, so it is
a separate, owner-ruled decision and is listed under open questions rather
than proposed here. Acceptance: with option (a), `cargo deny --workspace
check bans` at HEAD reports rand, rand_chacha, rand_core, and thiserror and
passes once the skip entries name them; with option (b), the deny.toml and
justfile comments both say dev-only duplicates are out of scope.

Construction: run `cargo deny --workspace check bans` at HEAD (passes), then
again with `multiple-versions-include-dev = true` added under `[bans]`: it
reports the four crates above as duplicates.

### deps-3: build.rs's figure-freshness check cannot fire on an incremental build; its two inputs are absent from rerun-if-changed
- Where: crates/before/build.rs:27-30 (related: crates/before/build.rs:93-100, crates/before/build.rs:183-197, .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:159)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read build.rs in full; `git log -S` dates the rerun list to 0a8884ee on 2026-08-13 and both figure reads to 2efff149 on 2026-08-14); executed: no (a build is not permitted)
- Verification: confirmed; history: no-rationale-found (the design note specifies `cargo:rerun-if-changed` on `fuelscape/` and `docs/`; the implementation lists three files under docs/, and the figure check landed a day later without extending the list)
- Owner-gated: no

Once a build script prints any `cargo:rerun-if-changed`, cargo reruns it only
when a listed path (or a `rerun-if-env-changed` variable, or the script
itself) changes. The list names the fuelscape directory and three files
under docs/. `main` also reads `results/space_consumption/itc_space_consumption.svg`
and `check_readme_figure_fresh` reads `docs/itc_space_consumption_readme.svg`;
neither is listed, so editing either after a warm build neither reruns the
script nor refreshes `$OUT_DIR/space_consumption.svg`, and the staleness the
check exists to catch is caught only by a clean build.

Evidence:

        27	    println!("cargo:rerun-if-changed=fuelscape");
        28	    println!("cargo:rerun-if-changed=docs/fuelscape.css");
        29	    println!("cargo:rerun-if-changed=docs/fuelscape.js");
        30	    println!("cargo:rerun-if-changed=docs/fuelscape-header.html");
        ...
        93	    let figure = std::fs::read_to_string("results/space_consumption/itc_space_consumption.svg")
        ...
       178	/// The README references the figure by URL, so it must be a committed
       179	/// file; committed-but-derived means it can rot, and this check is what
       180	/// prevents that — the header-freshness idiom. Regenerate with
        ...
       184	    println!("cargo:rerun-if-env-changed=BEFORE_REGEN_DOC_FIGURE");
       185	    let path = "docs/itc_space_consumption_readme.svg";
        ...
       191	    let have = std::fs::read_to_string(path).unwrap_or_default();

    .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md
       159	- `cargo:rerun-if-changed` on `fuelscape/` and `docs/`.

A freshness check is a meter, and this one has a liveness hole exactly on the
edit it guards (instruments before cures: every meter needs a liveness
floor). The doc comment at 178-180 says the check "is what prevents" rot; on
the developer's incremental loop it prevents nothing until an unrelated
listed file changes.

Resolution: add `cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg`
and `cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg` beside the
existing four, or list the `docs/` directory as the design note specifies
(directory entries are scanned recursively by mtime). Acceptance: the
construction below fails at the second build.

Construction: `cargo build -p before` (warm), change one color literal in
results/space_consumption/itc_space_consumption.svg, build again: at HEAD the
script does not rerun and no error appears; with the fix, the second build
fails with "docs/itc_space_consumption_readme.svg is stale relative to
results/space_consumption: run `just doc-figure`".

### deps-4: build.rs is a docs-only formatter every consumer compiles; a design question against a recorded decision
- Where: crates/before/build.rs:1-21 (related: crates/before/Cargo.toml:16-21, justfile:680-715, crates/before/src/testing/fuelscape_islands.rs:29, .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:46-54 and 171-174)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read build.rs, Cargo.toml, the justfile recipes, the design note; `du -sh crates/before/fuelscape` = 1.5M, 105 files; grep counts 155 `fuelscapes/` include sites in src); executed: no
- Verification: reframed: the sweep filed this as a medium-severity dissolution candidate; the design note records the per-consumer cost as weighed and accepted, so this is an owner-gated design question, not a defect; history: deliberate-and-holds (the note's §4 accepts the cost at "~320 KB ... negligible"; the dataset is now 1.3 MB committed JSON, still small)
- Owner-gated: yes (the direction of a documentation pipeline the owner designed)

The script renders 105 committed JSON files into rustdoc `<details>` islands
included at 155 `#[doc = include_str!(concat!(env!("OUT_DIR"), ...))]` sites
and themes one SVG. Every consumer of `before` (rumors, before-viz, the four
detached workspaces, any external user) compiles a serde_json
build-dependency and parses the dataset per clean build, and a malformed
committed dataset is a compile failure of every downstream crate. The
repository already uses the alternative idiom for the header and the README
figure (a committed derived file held fresh by a check), and the script has
accreted wiring that serves its own placement: the rerun list (deps-3), a
source-tree write behind an env opt-in, and a dotted `.open.html` name to
stay outside the totality scan's charset.

Evidence:

    crates/before/Cargo.toml
        16	# build.rs is a pure formatter: it renders the committed fuelscape
        17	# widget datasets (fuelscape/) into the doc islands the # Complexity
        18	# sections include from $OUT_DIR. serde_json only reads the committed
        19	# JSON; nothing is measured or computed at build time.
        20	[build-dependencies]
        21	serde_json = { workspace = true }
    crates/before/build.rs
        78	        // include (the dot keeps it outside the totality scan's
        79	        // island-name charset — the op's own doc site still owes the
        80	        // closed island).
        ...
       187	    if std::env::var_os("BEFORE_REGEN_DOC_FIGURE").is_some() {
       188	        std::fs::write(path, &want).expect("the README figure is writable");

    .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md
       171	- Build-deps: `serde_json` only. The script runs for every consumer build
       172	  (including the detached fuzz/fuzzfit workspaces and wasm targets — build
       173	  scripts execute on the host, so this is safe); it is a file transform
       174	  over ~320 KB and stays negligible.

The decision is recorded and the cost it accepted is still small; what has
changed since is the dataset size (4x), the two freshness and message
defects found in the script by this sweep (deps-3, deps-5), and the
carve-outs above, which are the "maintenance cascade" signal of Principle 3.
The note does not record the committed-islands alternative as considered.

Resolution: a design proposal for the owner, costs named. Have the compactor
(crates/before-fuelscape, which owns the format) emit the islands, the
`.open` variant, and the two themed SVGs as committed derived files under
crates/before/docs/, included by path from the doc sites; hold them fresh by
`fuelscape-verify`'s existing diff plus an in-crate test that re-derives the
header and README figure from CARGO_MANIFEST_DIR; point the totality test
(fuelscape_islands.rs:29, which reads `$OUT_DIR/fuelscapes/index`) at the
committed directory listing. Then build.rs and the serde_json
build-dependency dissolve, `just doc-figure` becomes a compactor mode, and a
dataset defect fails a test instead of every consumer's compile. Cost:
roughly 1.5 to 3 MB more committed derived HTML (the islands embed the JSON
payload), or the standalone JSON goes if tools/fuelscape-claims learns to read
the islands. Acceptance: if taken, `crates/before` has no build.rs and no
`[build-dependencies]`, `just docs` renders every island, and
`fuelscape-verify` fails on a hand-edited island. If declined, the deps-3 and
deps-5 fixes stand on their own and this entry closes as a recorded
decision.

### deps-5: build.rs `expect("validated")` on `meta.base_seed` names a proof that does not exist
- Where: crates/before/build.rs:216-216 (related: crates/before/build.rs:58-67, crates/before/build.rs:283-316, crates/before/build.rs:19-21)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (traced every check applied to `meta`: lines 58-63 compare doc and index values for equality, 64-67 check `commit` is a string, `validate` at 283-316 checks `op` fields only); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The only check on `meta.base_seed` is equality with the index's value, which
two string-typed or two absent values also satisfy. The `.as_u64().expect("validated")`
therefore panics with the word "validated" on a dataset whose seed is a
string or missing in both files, instead of the file-and-check message the
module doc promises. The sibling expects at 202-203 are honest: `validate`
checks `contract` and `claim` as non-empty strings via `as_str()`.

Evidence:

        19	//! hold. Every failure here is a defect in the committed repository
        20	//! (malformed data, a stale derived header), so it panics naming the
        21	//! file and check rather than reporting errors to a caller.
        ...
        58	        for param in ["base_seed", "samples_per_column"] {
        59	            assert_eq!(
        60	                doc["meta"][param], index["meta"][param],
        61	                "{file}: run parameter {param} differs from the index's"
        62	            );
        63	        }
        ...
       216	        "seed": format!("{:#x}", meta["base_seed"].as_u64().expect("validated")),
        ...
       285	        let s = op[key].as_str().unwrap_or("");

The doctrine is that every expect message is a one-line proof; this one
borrows its neighbors' wording without their backing.

Resolution: validate the index's `meta` once (`base_seed` and
`samples_per_column` as u64, `commit` as a non-empty string) in a
`validate_meta(file, meta)` beside `validate`, after which the expect message
is true; or replace the expect with `unwrap_or_else(|| panic!("{file}:
meta.base_seed must be an integer"))`. Acceptance: the construction below
panics naming fuelscape/index.json and the field.

Construction: set `"base_seed": "0x1"` (a string) in fuelscape/index.json and
in every fuelscape/<op>.json, then `cargo build -p before`: at HEAD the panic
message is `validated`, naming neither file nor field.

### deps-6: ci.yml installs and documents a floating `nightly` (and a floating stable) that no recipe invokes; its comments describe the regime the justfile pinned away
- Where: .github/workflows/ci.yml:54-74 (related: .github/workflows/ci.yml:17-25, .github/workflows/ci.yml:128-145, .github/workflows/ci.yml:202-216, justfile:23-40, justfile:1041, rust-toolchain.toml:20-31)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read ci.yml, justfile, rust-toolchain.toml; `git log -S'nightly-2026-06-30' -- justfile` dates the pin to e7a4b7b0c on 2026-08-04; `git log -- .github/workflows/ci.yml` since then shows one hand edit, e4d92ae4 on 2026-08-17 adding cargo-mutants to the install list, and otherwise dependabot bumps; `gh run view` on the last successful main run 33560347645 shows the coverage job, including `coverage-kernel-branch`, succeeded); executed: no
- Verification: reframed: the textual claims are confirmed; the sweep's "operational edge" about llvm-tools on the wrong nightly is settled by CI history (the branch leg runs green today, so rustup installs the dated nightly on demand and cargo-llvm-cov provisions its component there), and the sweep's "moved only by dependabot" is corrected to one hand edit that did not touch toolchain steps; history: deliberate-but-expired (the comments were true until e7a4b7b0c)
- Owner-gated: no

Every nightly recipe invokes `cargo +nightly-2026-06-30` and
rust-toolchain.toml pins stable at 1.97.1 with clippy, rustfmt, and the
wasm32 target; rustup resolves both regardless of what the workflow installs.
The workflow's toolchain steps install floating `nightly` and `stable` in all
three jobs, and its comments say the recipes "invoke `cargo +nightly`", that
the instruments job "tracks nightly, so a format bump upstream can turn the
leg red on an untouched tree", and that the runner needs "a current stable
toolchain (edition 2024 needs 1.85+)". Each is contradicted by the pins: the
installed floating toolchains are dead weight and the described failure mode
(a nightly format bump reddening an untouched tree) is exactly what the pin
removed.

Evidence:

    .github/workflows/ci.yml
        18	#   - a current stable toolchain (edition 2024 needs 1.85+), with clippy + rustfmt
        ...
        56	      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
        57	      # needs to exist, not be the default — whereas the gate's bare
        58	      # `cargo fmt`/`clippy`/`check` must hit stable.
        ...
        64	      - name: Install nightly toolchain (merged doctests and fuzz build)
        65	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
        66	        with:
        67	          toolchain: nightly
        ...
       130	  # format_version. This job tracks nightly, so a format bump upstream can
       131	  # turn the leg red on an untouched tree — the checker refuses loudly,
        ...
       206	      - name: Install nightly toolchain (branch coverage)
       207	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
       208	        with:
       209	          toolchain: nightly
       210	          components: llvm-tools
    justfile
        40	nightly_toolchain := "nightly-2026-06-30"
      1041	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} llvm-cov nextest --branch --workspace --all-features --lcov --output-path target/llvm-cov/workspace-branch.lcov

Prose speaks in the present tense: the workflow describes a floating regime
the tree retired a month ago, and three jobs each download a nightly they
never use. workflowlint pins action SHAs but nothing holds the workflow's
toolchain inputs to the justfile's pin, which is how the two drifted in
opposite directions.

Resolution: install the pinned toolchains from one source: read
`just --evaluate nightly_toolchain` in a step and pass it as `toolchain:`
(with `llvm-tools` in the coverage job), drop the floating nightly installs;
drop the stable install steps or re-denominate their comments to
"rust-toolchain.toml provisions 1.97.1 with clippy, rustfmt, and wasm32";
rewrite lines 17-25, 54-58, 128-133, 139-141, and 202-205 to the pinned
regime. Optionally extend tools/workflowlint to require every dtolnay
`toolchain:` input to equal the justfile pin or the rust-toolchain.toml
channel. Acceptance: no `toolchain: nightly` or `toolchain: stable` remains
in ci.yml; every comment naming a toolchain names the pinned one; the three
jobs stay green.

### deps-7: the bitvec dev-dependency and two one-off probe examples outlive the investigation that justified them
- Where: crates/before/Cargo.toml:33-36 (related: crates/before/examples/emit_probe.rs:1-16 and 50-52, crates/before/examples/perf_probe.rs:1-9, .agent-notes/2026-08-04-perf-probe/README.md:15-17)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for emit_probe/perf_probe across justfile, tools/, .github/, and both AGENTS.md files: the only reference is the Cargo.toml comment; grep of `before::` in emit_probe.rs: none; `git log` on both files and on the BitsBuf landing); executed: no
- Verification: confirmed, with one sharpening: emit_probe.rs uses nothing from `before` at all (it reproduces a word writer standalone "so the comparison needs no crate internals"), so it is a self-contained microbenchmark of hand-rolled writers against bitvec living in before's examples; history: already-known (the agent note states where the files remain; it records no reason to keep them)
- Owner-gated: yes (whether a profiling harness stays at hand is the owner's call; the emit_probe half, whose question BitsBuf answered, is not)

`bitvec` is a dev-dependency only so emit_probe.rs can time an external
baseline. Both probes date from the 2026-08-04 perf investigation
(7d101fe7), perf_probe calls itself "One-off", neither appears in any recipe,
and the decision they informed landed on 2026-08-18 (83e61b4d, "replace the
bitvec build buffer with the crate-owned BitsBuf"). Cost: bitvec and its four
transitive crates compile into every `cargo test -p before`, `clippy
--all-targets`, and `check --all-targets`, and 527 lines of probe code are
linted, doclinted, and rustdoc-built on every gate for no committed verdict.

Evidence:

    crates/before/Cargo.toml
        34	# The emit_probe example's external comparison baseline: the production
        35	# build buffer is the crate-owned BitsBuf, and nothing shipped links bitvec.
        36	bitvec = { workspace = true }
    crates/before/examples/emit_probe.rs
        50	/// A minimal word-buffered MSB-first bit writer: the staging discipline
        51	/// the crate's own `PackedBuilder` ships, reproduced standalone so the
        52	/// comparison needs no crate internals.
    crates/before/examples/perf_probe.rs
         1	//! One-off profiling harness for the perf-probe investigation.
    .agent-notes/2026-08-04-perf-probe/README.md
        15	The instruments it names — `perf_probe.rs` (op loops) and `emit_probe.rs`
        16	(output-primitive microbenchmarks) — are cargo examples and remain at
        17	`crates/before/examples/`, where they can still be run.

Principle 3: machinery outlives the constraint that justified it. The
criterion benches against `before::oracle` are the retained instrument for
the question perf_probe asked.

Resolution: retire examples/emit_probe.rs and the `bitvec` dev-dependency
(its question is closed and it exercises no crate code); decide separately
whether perf_probe.rs stays as a profiling harness, and if it goes, re-aim
the agent note's sentence at git history (the note is exempt from the ghost
rule; the Cargo.toml comment is not). Acceptance: `bitvec` absent from
crates/before/Cargo.toml and the root lock; `just gate` clean.

### deps-8: before's `serde` feature selects `derive`, which nothing in the crate uses
- Where: crates/before/Cargo.toml:30-30 (related: Cargo.toml:68, crates/before/src/serde_impls.rs:14-15 and 28)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep for `derive(...Serialize|Deserialize)` and `serde_derive` over crates/before/{src,tests,benches,examples}: nothing; read serde_impls.rs:1-40); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes (the selection is part of the published feature surface, though no consumer in this workspace observes a difference)

Every serde impl in before is hand-written and imports only the traits; the
Deserialize impls go through `<Vec<u8>>::deserialize`, which serde's `alloc`
feature supplies. The workspace table takes serde with default features off,
so before's selection is the whole selection, and `derive` pulls serde_derive,
syn, quote, and proc-macro2 into the graph of any external consumer that
enables `before/serde` without otherwise deriving. rumors and before-viz
already select `derive` themselves, so the cost is external-consumer only.

Evidence:

    crates/before/Cargo.toml
        30	serde = { workspace = true, optional = true, features = ["derive", "alloc"] }
    Cargo.toml
        68	serde = { version = "1", default-features = false }
    crates/before/src/serde_impls.rs
        14	use serde::de::Error as _;
        15	use serde::{Deserialize, Deserializer, Serialize, Serializer};
        ...
        28	        let bytes = <Vec<u8>>::deserialize(d)?;

Resolution: `serde = { workspace = true, optional = true, features = ["alloc"] }`.
Acceptance: `just gate` clean; `cargo tree -p before --features serde -e
normal` shows no serde_derive.

### deps-9: detached-workspace lockfiles resolve shared crates at different versions than the root, with no stated convergence policy
- Where: crates/before/wasm32-pins/Cargo.lock:73-74 (related: crates/before/fuzz/Cargo.lock:39-40 and 94-95, crates/before/surfacecheck/Cargo.lock:37-38, crates/before-fuelscape/Cargo.lock:484-485, tools/manifestlint:20-22)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (grep of every committed lock for dashu-int, borsh, bytes, thiserror, wasmtime); executed: no
- Verification: confirmed; history: no-rationale-found (grep of `cargo update` across .agent-notes, justfile, and AGENTS.md finds no stated policy)
- Owner-gated: no

The `before` the fuzz targets, the 32-bit pins, the surface check, and the
fuelscape exercise links dependency versions the gate's native suites never
build: dashu-int 0.5.1 (fuzz, wasm32-pins, surfacecheck, fuelscape) against
0.5.0 (root, fuzzfit); borsh 1.8.0 (fuzz, wasm32-pins) against 1.6.1 (root);
bytes 1.12.1 (all five detached) against 1.11.1 (root); wasmtime 47.0.3
(wasm32-pins) against 47.0.4 (fuzzfit, fuelscape). The differential fuzz
target compares borsh 1.8.0's `deserialize_reader` against raw decodes while
the shipped graph carries 1.6.1; the 32-bit pins exercise the big-integer
backend's word cap against dashu-int 0.5.1 while production builds 0.5.0.
manifestlint stops at the root workspace by design.

Evidence:

    grep of `name = "dashu-int"` / `version` across committed locks:
      Cargo.lock: "0.5.0"
      crates/before-fuelscape/Cargo.lock: "0.5.1"
      crates/before/fuzz/Cargo.lock: "0.5.1"
      crates/before/fuzzfit/Cargo.lock: "0.5.0"
      crates/before/surfacecheck/Cargo.lock: "0.5.1"
      crates/before/wasm32-pins/Cargo.lock: "0.5.1"
    borsh: Cargo.lock "1.6.1"; fuzz and wasm32-pins "1.8.0"
    tools/manifestlint
        20	# library's TOML parser. Detached workspaces (the fuzz targets, with their
        21	# own Cargo.lock) are separate workspaces with their own tables, outside
        22	# this tool's scope.

Measurements bind to their run: a fuzz or pin verdict is evidence about the
shipped `before` to the extent the linked closure matches. Patch-level skew
is most likely benign here, but the property is unstated and unchecked, so a
real divergence (a dashu-int behavior fix landing in one lock) would be
invisible.

Resolution: state the policy once, at the mechanical lockfile enumeration the
deps-1 fix introduces: either a convention ("every detached lock is updated
in the same commit as a root `cargo update`") or a cheap cross-lock diff in
that leg (a small tool over `git ls-files -- '*Cargo.lock'` reporting any
crate resolved at differing versions across locks; crates absent from the
root, such as wasmtime, need no entry). Acceptance: the policy sentence
exists in the recipe comment, and if the tool is taken, it fails at HEAD on
dashu-int, borsh, and bytes and passes after one `cargo update` sweep.

### deps-10: the alloc-A/B arm roster lives in three hand-synced places, and the check-cfg comment's "cannot drift" holds in one direction only
- Where: crates/before/Cargo.toml:97-108 (related: justfile:792-809, crates/before/benches/common/mod.rs:259-281, crates/before/src/version/skyline/query.rs:513-515 and 608, crates/before/src/version/skyline/text.rs:351-353)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read all four sites; grep of `before_alloc_ab` in src and benches; rustc book check-cfg page fetched: "The command line `--cfg` arguments are currently NOT checked but may very well be checked in the future."); executed: no
- Verification: confirmed; history: no-rationale-found (the Cargo.toml comment acknowledges the hole and delegates it to the recipe's hand-written list)
- Owner-gated: no

`unexpected_cfgs = deny` fails a seam or bench that queries a value missing
from the check-cfg roster (roster too small is loud). A roster value no seam
queries, or a recipe arm no roster lists, is silent, because rustc does not
check command-line `--cfg` values. The recipe closes that hole with a
hand-written case list that duplicates the Cargo.toml values, which duplicate
the seams; `alloc_arms()` in the bench is a fourth spelling. An arm added to
the recipe's list without a seam passes the case check, sets a cfg nothing
queries, and saves a baseline labeled `<target>-<arm>` over shipped code
while `alloc_arms()` prints "shipped": the mislabel the comment names.

Evidence:

    crates/before/Cargo.toml
       102	# the build of any seam or bench that queries an arm missing from this
       103	# roster, so the roster cannot drift from the code. (It cannot see a
       104	# mistyped RUSTFLAGS value nothing queries; the `bench-alloc-ab` recipe
       105	# validates the arm name at invocation for exactly that hole.)
       106	unexpected_cfgs = { level = "deny", check-cfg = [
       107	    'cfg(before_alloc_ab, values("projection_growth", "projection_shrink", "display_growth"))',
       108	] }
    justfile
       808	    @case "{{ arm }}" in (shipped|projection_growth|projection_shrink|display_growth) ;; (*) echo 'bench-alloc-ab: unknown arm "{{ arm }}"' >&2; exit 2;; esac
    crates/before/benches/common/mod.rs
       268	    if cfg!(before_alloc_ab = "projection_growth") {
       269	        arms.push("projection_growth");
       270	    }
       271	    if cfg!(before_alloc_ab = "projection_shrink") {
       272	        arms.push("projection_shrink");
       273	    }
       274	    if cfg!(before_alloc_ab = "display_growth") {
       275	        arms.push("display_growth");
       276	    }

No hand-maintained enumerations: four restatements of one roster, with the
silent direction unguarded. The seams are the ground truth.

Resolution: replace the recipe's case list with a run-time provenance
assertion in the bench: the recipe exports `BEFORE_ALLOC_AB_ARM={{ arm }}`
beside RUSTFLAGS, and a small wrapper called at bench start panics when the
requested arm is not the compiled one (`alloc_arms()` stays as the label
printer). check-cfg then remains the sole roster, kept in sync with the seams
by `deny`. Acceptance: `just bench-alloc-ab presize nosuch_arm` fails before
any measurement, naming the compiled arm; adding a fourth arm requires
touching only a seam and the check-cfg roster.

### deps-11: the fuzz workspace manifest and README carry opaque roster IDs (PROG-5 / COV-7) and restate build commands the justfile supersedes
- Where: crates/before/fuzz/Cargo.toml:1-9 (related: crates/before/fuzz/Cargo.toml:47-48, crates/before/fuzz/README.md:1 and 14 and 55-60, justfile:42-54 and 368 and 555-559)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `(PROG|COV)-[0-9]+` across the tree including .agent-notes: exactly three sites, all in the fuzz workspace, and no definition anywhere; read fuzz/Cargo.toml and fuzz/README.md in full); executed: no
- Verification: confirmed; history: no-rationale-found (the IDs resolve to nothing, including the agent notes)
- Owner-gated: no

The manifest header and README title name `PROG-5 / COV-7`, and Cargo.toml
calls the round-trip property "the keystone invariant (COV-7)"; the tags
resolve to nothing in the tree or the notes. The same header and the README's
Run section give `cargo +nightly fuzz build` / `fuzz run` commands that omit
the `--target {{ host_triple }}` the justfile explains is required with a
prebuilt cargo-fuzz and use floating `nightly` where the recipes use the
dated pin; justfile:51-52 in turn hand-syncs the 20-second smoke default
"with the guidance in crates/before/fuzz/Cargo.toml".

Evidence:

    crates/before/fuzz/Cargo.toml
         1	# Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
         ...
         4	#   cargo +nightly fuzz build
         5	#   cargo +nightly fuzz run fuzz_decode -- -max_total_time=20
         ...
        47	# Decode-only target: feed arbitrary bytes to every top-level `decode`. Asserts the
        48	# keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes
    crates/before/fuzz/README.md
         1	# `before` fuzz targets (PROG-5 / COV-7)
        14	rustup toolchain install nightly
        55	cargo +nightly fuzz build   # build all targets
    justfile
        51	# Default fuzz smoke duration per target, in seconds (matches the guidance in
        52	# crates/before/fuzz/Cargo.toml).
       368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

No opaque roster IDs (they outlive their roster; these already have) and no
restated enumerable facts: the invocation of record is `just fuzz-build` /
`just fuzz`, and both prose copies have diverged from it.

Resolution: drop the IDs and name the property ("the decode round-trip
invariant: an accepted value re-encodes stably and decodes back to itself");
replace the command listings in Cargo.toml:3-9 and README:50-64 with a
pointer to the two recipes; let the README's prerequisites name the pinned
nightly via the justfile rather than `rustup toolchain install nightly`; drop
the justfile:51-52 cross-reference once the manifest no longer carries the
number. Acceptance: grep of `(PROG|COV)-[0-9]+` over the tree is empty; the
fuzz workspace's prose names no cargo-fuzz command line.

### deps-12: docs.rs metadata enables no feature, so the serde and borsh impls the crate docs advertise never render there
- Where: crates/before/Cargo.toml:13-14 (related: crates/before/src/lib.rs:392-397 and 448 and 451)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read Cargo.toml and lib.rs:388-403; grep of docsrs/doc_cfg in lib.rs: none; lib.rs:448 and 451 declare `mod serde_impls;` and `mod borsh_impls;` behind their features); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes (published-docs configuration)

`[package.metadata.docs.rs]` passes only the header flag; with default
features docs.rs omits the impl blocks lib.rs:392-397 describes. rumors' own
metadata uses `all-features = true`; for before that would also publish the
test-only instrument modules, so a narrower selection is right.

Evidence:

    crates/before/Cargo.toml
        13	[package.metadata.docs.rs]
        14	rustdoc-args = ["--html-in-header", "docs/fuelscape-header.html"]
    crates/before/src/lib.rs
       392	//! - **`serde`:** `Serialize`/`Deserialize` for [`Party`], [`Version`],
       393	//!   [`Clock`], [`Rank`], [`Ranked`], and [`Span`].
       394	//! - **`borsh`:** `BorshSerialize`/`BorshDeserialize`, likewise as the

Resolution: add `features = ["serde", "borsh"]` to the docs.rs table (not
`all-features`). Acceptance: a `cargo doc -p before --features serde,borsh`
render shows the impl blocks on the six types.

### deps-13: before names one type two ways: `dashu_int::UBig` in codec, `suanpan::UBig` in meter and the query tests
- Where: crates/before/src/meter.rs:1660-1663 (related: crates/before/src/version/skyline/query/tests.rs (17 sites), crates/before/src/codec/int.rs:83, crates/before/src/meter/tests.rs:8, crates/before/src/meter/registry.rs:80, crates/before/src/codec/base.rs:7, crates/suanpan/src/lib.rs:358)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep counts: `suanpan::UBig` 29 times in meter.rs, 17 in query/tests.rs, once in codec/int.rs, plus `use suanpan::UBig;` in meter/tests.rs and meter/registry.rs; `use dashu_int::UBig;` in codec/dsi.rs, codec/base.rs, codec/gamma.rs and the codec tests); executed: no
- Verification: confirmed, with a sharper site list than the sweep's; history: no-rationale-found
- Owner-gated: no

suanpan re-exports `dashu_int::UBig` for callers without a direct dashu-int
dependency; before depends on dashu-int directly and spells the type
`dashu_int::UBig` throughout codec, but `suanpan::UBig` fully qualified at
every use in meter.rs and query/tests.rs. Two spellings of one type across
one crate cost a reader a lookup to confirm they are the same. The one site
where the suanpan spelling arguably informs is `Int::from_ubig`
(codec/int.rs:83), which receives the accumulator's output; the meter and
test sites construct magnitudes and have no such reason.

Evidence:

    crates/before/src/meter.rs
      1660	    let wide = suanpan::UBig::ONE << FREEZE_POSITION_DROP_BITS;
      1661	    let unit = suanpan::UBig::ONE;
      1662	    let descent = (&wide + &unit) * suanpan::UBig::from(k as u64);
      1663	    let mut value = (suanpan::UBig::ONE << band) + descent;
    crates/before/src/codec/base.rs
         7	use dashu_int::UBig;
    crates/suanpan/src/lib.rs
       358	pub use dashu_int::UBig;

Resolution: `use dashu_int::UBig;` at the top of meter.rs and
query/tests.rs and drop the prefix at each site; leave `from_ubig` as is if
the boundary spelling is wanted. Acceptance: grep of `suanpan::UBig` in
crates/before/src returns only deliberate boundary sites.

### deps-14: before-fuelscape carries a second bignum (num-bigint + num-traits) beside the dashu-int already in its graph
- Where: crates/before-fuelscape/Cargo.toml:42-45 (related: crates/before-fuelscape/src/count.rs:43-44, crates/before-fuelscape/src/sample.rs:28-29, crates/before-fuelscape/src/count/tests.rs:2)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (grep of num_bigint/num_traits use sites: three files; dashu-int 0.5.0's `[features]` table read from the registry source: `rand = ["rand_v08"]`); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no (tool-side only)

The fuelscape uses num-bigint (with its `rand` feature) and num-traits in
count.rs and sample.rs for exact counting tables and uniform choice, while
dashu-int 0.5 already rides the graph through `before` and offers the same
capability behind its own `rand` feature, which targets the rand 0.8 this
workspace pins.

Evidence:

    crates/before-fuelscape/Cargo.toml
        42	# Exact-integer counting tables and proportional choice: uniform sampling
        43	# is arithmetic over exact counts, never floats.
        44	num-bigint = { version = "0.4", features = ["rand"] }
        45	num-traits = "0.2"
    crates/before-fuelscape/src/sample.rs
        28	use num_bigint::{BigUint, RandBigInt};
        29	use num_traits::Zero;
    dashu-int 0.5.0 Cargo.toml [features]
    rand = ["rand_v08"]

Resolution: enable dashu-int's `rand` feature in the fuelscape manifest and
port count.rs and sample.rs (`Zero`/`One` to `UBig::ZERO`/`UBig::ONE`;
`gen_biguint_below` to `rng.gen_range(UBig::ZERO..bound)` via dashu's
`SampleUniform`). Cost: a two-module rewrite plus its tests; benefit: two
crates fewer in the tool build and one integer type at the sampler/subject
boundary. Acceptance: `just fuelscape-verify` byte-identical to HEAD (the
sampler's draws must not move; if they do, that is a re-pin event and this
nit is not worth it).

### deps-15: manifest feature summaries lag their module docs: limb-meter lights two counters, touch-meter counts more than the manifest says
- Where: crates/before/Cargo.toml:74-84 (related: crates/suanpan/Cargo.toml:22-28, crates/before/src/codec/base/limb_meter.rs:21-27 and 54-69, crates/suanpan/src/touch_meter.rs:3-12)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read both manifests, limb_meter.rs:1-69, touch_meter.rs:1-20); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The `limb-meter` comment names operand limbs per `Base` operation plus one
width record per wide-gamma value, while the module also carries the
densified-image digit column, which the board reads as a separate currency.
suanpan's `touch-meter` comment names digit read-modify-writes plus one per
operand limb, while touch_meter.rs also counts top-settlement scan steps,
certified-run skips, and the quick register's one-touch operations. Both
manifests are summaries and the module docs are the contract, so this is
completeness, not contradiction.

Evidence:

    crates/before/Cargo.toml
        74	# Counts big-integer limb-scale work (operand limbs per `Base` operation:
        75	# arithmetic, comparison, equality, and hashing; plus one value-width record per
        76	# decoded wide-gamma value) into a process-global counter read via
    crates/before/src/codec/base/limb_meter.rs
        21	//! A second column ([`record_densified`]) counts the query folds' densified
        22	//! cluster images by their zero-filled capacity. That fill is width-scale
    crates/suanpan/Cargo.toml
        23	# Counts every digit read-modify-write (plus one per operand limb read by a
        24	# wide operation) into a process-global counter read via [`touch_meter`].
    crates/suanpan/src/touch_meter.rs
         6	//! digit its collapse zeroes; a top-settlement scan counts one per zero
         7	//! digit it steps past, and one — total — per certified zero run it
         8	//! skips whole; a wide operation adds one per operand limb read): the

Resolution: one clause each ("...and a second column counting
densified-image digits"; "...plus top-settlement scan steps and
quick-register operations"), or point both summaries at the module doc as
the contract. Acceptance: each manifest comment names every column its
feature lights, or defers to the module.

### deps-16: no package include/exclude: measurement artifacts, the paper transcription, and plotting scripts would ship in the crate tarball
- Where: crates/before/Cargo.toml:1-7 (related: .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:50-52)
- Class / severity / confidence: scaffolding / nit / medium
- Provenance: verified (read Cargo.toml; `git ls-files` sizes: results/ 2.0 MB across 13 files, reference/ 445 KB across 15 files, scripts/ 15 KB); executed: no
- Verification: confirmed; history: already-known in part (the design note records the crates.io 10 MB cap and that build.rs may only read files that ship; it does not record the package's contents)
- Owner-gated: yes (pre-release publishing hygiene)

With no `include`/`exclude`, `cargo package` would carry results/benchmarks
(PNG, SVG, CSV), reference/ (including itc2008.pdf), scripts/, and the
space-consumption PNG alongside what build.rs needs (fuelscape/, docs/,
results/space_consumption/itc_space_consumption.svg). Sub-packages (fuzz,
fuzzfit, wasm32-pins, surfacecheck) are excluded by cargo automatically.
Pre-release, so forward-looking; the total stays well under the 10 MB cap.

Evidence:

    crates/before/Cargo.toml
         1	[package]
         2	name = "before"
         3	version = "0.1.0"
         4	edition = "2021"
         5	description = "Interval Tree Clocks (Almeida, Baquero & Fonte, 2008): packed bit-stream storage, transient fixed-width working form, linear-typed API."
         6	license = "MPL-2.0"
         7	readme = "README.md"
    (no include or exclude key anywhere in the file)

Resolution: when the first release approaches, add an `include` list (src,
build.rs, fuelscape, docs, results/space_consumption/itc_space_consumption.svg,
README.md, LICENSE) or an `exclude` of results/benchmarks, reference,
scripts, and the PNGs; if the deps-4 direction is taken first, the list
shrinks to src, docs, README. Acceptance: `cargo package --list -p before`
names no file under results/benchmarks, reference, or scripts.

## Positives

Verified by reading at the cited sites in this pass:

- dsi-bitstream is taken with `default-features = false, features = ["alloc"]`
  (Cargo.toml:51), and crates/before/src/codec/dsi.rs:1-30 states the exact
  trade: the production reader from the library, the writers in-house, and
  the reason the library's own `read_gamma` is not used (a `debug_assert`-guarded
  2^64 cap that would mis-decode in release). This is the model for how a
  dependency boundary should be documented.
- dashu-int is narrowed to `std` in the workspace table (Cargo.toml:50), and
  suanpan's crate docs (lib.rs:296-297) state that the re-exported `UBig`
  makes the dashu-int major a public-API fact and its bump a breaking change.
- borsh impls are hand-written against `borsh::io` with no `derive` feature
  (borsh_impls.rs:13-14); the fuzz workspace's manifest explains each
  transport dependency's role in one sentence (fuzz/Cargo.toml:29-35).
- stacker is test-only: recurse.rs gates `grow` and `descend!` behind
  `cfg(test)` (lines 100 and 118), matching crates/before/AGENTS.md:24-25.
- surfacecheck pins `rustdoc-types = "=0.59.0"` with the reason at the pin
  (Cargo.toml:23-29) and refuses a mismatched `format_version` naming both
  numbers and the bump procedure (main.rs:71-85); rust-toolchain.toml states
  why it pins and how to bump (lines 6-18).
- tools/manifestlint holds every member manifest to the workspace table with
  a self-test, and the root Cargo.toml states the inheritance convention
  beside the table (lines 10-16).
- The `required-features` entries on amp_board and code_study
  (crates/before/Cargo.toml:110-125) turn a mis-featured invocation into a
  cargo error with the reason stated; the self-dev-dependency excludes
  limb-meter and scan-meter so bench builds stay unmetered (lines 49 and
  80-83).
- suanpan's footprint is one normal dependency, two dev-dependencies, one
  feature, with claims.rs behind `cfg(test)` (lib.rs:364-365).
- The design note for the fuelscape islands records the per-consumer build
  cost of build.rs explicitly (§4) rather than leaving it implicit; that is
  what let this pass reframe deps-4 as a decision to revisit rather than an
  oversight.

Reported by the sweep and not re-read here: the fuzzfit bands' `PINNED_RUSTC`
and wasmtime lock-pin pairing (bands.rs:312-321), and the `oracle` feature's
bench/test-only claim across benches and the perf_probe example.

## Open questions for Finch

1. Dev-only duplicates in the deny leg (deps-2): include them (option a,
   roster rand 0.8/0.9 and thiserror 1/2 with holdouts) or state the
   exemption (option b)? If the rand convergence is wanted, it changes
   rumors' normal `rand` dependency (Cargo.toml:142) and reseeds every corpus
   drawn through `gen_range` (rand 0.9's Uniform integer sampling is
   documented as breaking value stability): which pinned artifacts (the
   fuzzfit fuel bands, benchjudge-expected rosters, any snapshot from
   `gen_range` draws) would re-pin? Recommendation: option (a) now, the rand
   convergence as its own owner-ruled commit.
2. wasm32-pins wasmtime bump (deps-1): the advisories concern WASI paths the
   harness does not compile, so the fix is a patch bump either way; confirm
   with `just wasm32-pins` that 47.0.4 leaves every pin's outcome unchanged.
3. build.rs direction (deps-4): committed derived islands emitted by the
   compactor and held fresh by `fuelscape-verify`, or the per-consumer
   build-time formatter as the accepted cost of a single JSON source? The
   deps-3 and deps-5 fixes stand regardless.
4. Out of this sweep's scope, recorded here because it was observed and
   disposed of nowhere else: the latest `ci` run on main at the briefed
   commit (run 33567211421, 2026-09-01) failed in the `coverage` job at
   `just coverage-kernel (line, stable)`, with `before::meter
   masked_cmp_hole_envelope` panicking at crates/before/tests/meter.rs:6929:
   "masked_cmp_hole: peak heap 1156 B exceeds the pinned envelope 480 B
   (input 755 B)" (MEASURED line: input_bytes=755 peak_heap=1156 segments=0
   limb_ops=0 touches=14 scan_bits=6028). The `ci` and `instruments` jobs
   passed, and the previous main run (33560347645) passed all three jobs, so
   the peak-heap envelope trips only under llvm-cov instrumentation and only
   on this run. Not investigated; it belongs to the metering partition. The
   captured log is at
   <session scratchpad>/final-sweep-deps/ci-failed.log.
5. indicatif for the space_consumption example: a progress bar for a
   minutes-long paper-reproduction run that produces a committed figure, at
   the cost of its transitive set in every before test build. A judgment
   call the sweep declined to make for you; I would keep it.

## Dropped

- Sweep open question "Does the CI coverage job's coverage-kernel-branch leg
  actually run today?": settled, not dropped as a finding but retired as a
  question. Run 33560347645 on main shows the coverage job green including
  the branch leg, so rustup provisions the dated nightly on demand and
  cargo-llvm-cov supplies its component there; folded into deps-6 as
  verified context.
- Sweep phrasing in finding 5 "ci.yml has since moved only by dependabot
  bumps": corrected in deps-6 (one hand edit, e4d92ae4 on 2026-08-17, touched
  the tool-install list only); the finding itself survives.
- No finding was dropped outright: every claim checked against the tree held
  at its cited lines.
