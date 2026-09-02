<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: core (crate root, clock, party, version core, rank, span and causally, cross-cutting)

## Goal

The per-module entries no pattern or owner decision grouped, for the public types and the crate root: each lands per its stated Resolution and Acceptance inside an approved roster, under rulings 44 (elide ungrounded allocation sentences), 52 (no coinages), 64 (guards deleted, covering test named only in the commit message), and 88 (ceilings only).

## Rulings on this lane's mediums

Every medium in this lane is ruled (rulings 93 to 103): clock-22, clock-28, deps-3, module-graph-2. The decisions stand beside each entry under Members.

## Roster summary

5 ruled (1 medium, 4 low); 4 medium ruled (93 to 103); 99 roster members approved (ruling 104) (49 low, 50 nit).

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `5328537c` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `5328537c`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 is the one place this brief set says
  otherwise, and it says so at the entry.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.
- **No lower bound on performance is pinned anywhere (ruling 88).** Every
  touch, scan, limb, or heap pin is a ceiling: a reading over it fails; a
  reading under it is an improvement that lands by tightening the ceiling
  with its attribution in the commit. The only floors are liveness floors
  derived from a mechanism's irreducible work, never from a measured
  reading. An entry whose Resolution asks for an exact pin or a measured
  floor is read as a ceiling plus any mechanism-derived floor it names.

## Ordering

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p2-widths`, `p2-cures`, `p4-ghosts`, `p4-structure`, `p4-rosters`, and `p7-api` (the same files). rank-22 is `p8-performance`'s and rank-16 lands before it. Owns `src/lib.rs`, `src/{clock,party,version}.rs` and their submodules, `src/version/{rank,ranked,ticks}.rs`, `src/span.rs`, `src/causally/**`, `src/{fold,shape,recurse}.rs`, the serde and borsh impls.

## Members

### rank-32 (medium, claims): ruling 90

Ranked::encode_rank is documented as a fused, more efficient emission; it is Rank::encode by another name, and has been since the commit that introduced it

- Owner-gated: no

Resolution: Make `Ranked::encode_rank` `self.version.rank().encode()` and `encode_rank_to` likewise; drop the `encode_parts` import from ranked.rs and make `encode_parts` private to rank.rs (delete its `pub(crate)` rationale); re-justify `raw_parts` by its remaining callers (the oracles' raw form and the board's limb denomination), or replace it with a `#[cfg(any(test, feature = "meter"))]` accessor beside `content_bits`. Rewrite ranked.rs:46-49 and 192-196 to "equivalent to `v.rank().encode()`" with no efficiency ranking; version.rs:1051 and 1073 drop "but more efficient" (keep "more succinct"); rank.rs:486-491 becomes "The stored parts, the raw normalized form the reference computations and the meter denominators read"; the fuelscape `size_measure` reads "one rank fold and its emission" (regenerate the JSON); version/tests.rs:1640-1641 drops the parenthetical. If an actual fusion is wanted, note that constructing the `Rank` after normalization is free, so there is no cheaper fold-side path to build; the true statement is that `encode_rank` exists as a spelling convenience on a view. Acceptance: `grep -rn 'fused emission\|fused encode\|more efficient\|less efficient\|without materializing the intermediate' crates/before/src crates/before-fuelscape/src` returns nothing about `encode_rank`; `encode_parts` has no `pub(crate)`; `ranked_carries_own_rank` (laws.rs:388-389, `ranked.encode_rank() == a.rank().encode()`) still passes; the board cells `rank_encode` and `ranked_encode_rank` read identical limb and touch counts for the same version, as they must already.

Ruled (90): `Ranked::encode_rank` is `self.version.rank().encode()` (and `encode_rank_to` likewise); `encode_parts` private to rank.rs; `raw_parts` re-justified by its real callers or replaced by a meter-gated accessor; every fused-emission sentence becomes "equivalent to `v.rank().encode()`"; the fuelscape `size_measure` JSON regenerated (report if the regeneration exceeds this lane's cap).

### rank-16 (low, performance): ruling 92

The encode path clones the whole numerator, and plus_one's at-ceiling arm keeps the base value alive across from_limbs

- Owner-gated: no

Resolution: Add a metered `impl Shr<u64> for &Base` (keeping `meter_limbs1`, so the board's `rank_encode` limb floor still reads the shift) and a `Num::shr_ref(&self, n)` reusing `Wide::shr_limbs`, so `encode_parts` reads `num.shr_ref(exp).plus_one()`; in `plus_one`'s at-ceiling arm bind `let limbs: Vec<u64> = Limbs::new(&base.0).collect(); drop(base);` before `Num::from_limbs(increment(limbs))`. Add a wasm32 pin `pin_rank_integral_roundtrip(k)` at `k = 2^32 - 32` (decode, re-encode, byte-equal); if the harness's memory cap makes it terminal today, commit it in the `*_memory_terminal_traps` style and flip it after the deletions. Acceptance: the `rank_encode` heap readings drop (re-pinned with attribution at the parent); the `rank_encode` limb floor still reads; the new wasm32 pin passes; `rank_wide_arm_codec_roundtrips_canonically` and num/tests.rs unchanged.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane. Lands ahead of rank-22 (`p8-performance`).

### rank-18 (low, simplification): ruling 92

Four byte-source adaptors wrap decode_stream, and two decodes copy their whole input first

- Owner-gated: yes (the borsh transport's error genre on a short read would change from `Decode::Io(UnexpectedEof)` to `Decode::Truncated` unless mapped deliberately)

Resolution: `pub(crate) fn decode_stream<R: Read>(reader: &mut R) -> Result<Rank, Decode>` with `BitSource<R>` reading one byte via `read_exact`; `Rank::decode` becomes `decode_stream` then a one-byte EOF probe (a byte read is `TrailingBits`); `Ranked::decode` becomes `decode_stream` then `Version::decode(reader)`; the two borsh impls call it directly; delete `decode_bytes` and the `decode_rank_stream` alias. The owner rules how a short read maps (`Truncated` is arguably the correct genre; `Io` preserves today's borsh behavior). Acceptance: one `decode_stream` definition and name, no closure adaptors; `rank_decoding_rejects_each_genre`, `rank_encoding_exhaustive_small_scope` (both rejection genres still fire), `ranked_decode_rejects_each_genre`, and the borsh round-trip suites pass; `Rank::decode` on a slice performs no `read_to_end` allocation.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

### span-causally-35 (low, verification): ruling 92

`Query::coverage`'s clone-identity rung has no agreement test across buffer identity

- Owner-gated: no

Resolution: a proptest beside `coverage_matches_membership_on_points` (or in causally/tests.rs) over `neutral_queries`/`down_queries`/`up_queries` asserting `q.coverage(Span::new(&v, &redecoded).unwrap()) == q.coverage(Span::at(&v))` with `redecoded = Version::decode(&v.encode()[..]).unwrap()`, mirroring `span_contains_matches_place`'s redecoded-argument leg. Acceptance: the property runs in `just test-all`; deleting the `ptr_eq` rung leaves it green (equivalence); forcing `refine_partial` to return `Partial` on a coincident clamp makes it red.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

### version-core-36 (low, simplification): ruling 92

Five committed proptest seeds describe parameter shapes of properties that no longer exist in their files

- Owner-gated: yes (the doctrine forbids stripping seeds casually; this is a deliberate pruning that needs a ruling)

Resolution: for each orphaned seed, move it to the file of the property that now owns the invariant if one exists, else remove it in a commit naming the dissolved property; and consider extending `tests/seed_liveness.rs` to check each `# shrinks to` parameter list against the sibling file's live `proptest!` signatures, so the class is caught mechanically. Acceptance: every `cc … # shrinks to …` comment in the two files names only parameters of a `proptest!` signature in the sibling test file.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

## Mediums ruled 93 to 103

Each medium below now carries its ruling and any amendment beside its quoted Resolution; land per the ruling.

### clock-22 (medium, verification): ruling 100

The depth-100k proof says "every public op" but drives a listed subset over one left-only spine family

- Owner-gated: no

Resolution: Extend rather than narrow: add flat-loop builders beside `deep_left_spine_party` for a right spine (`01` × depth, then `00`) and a both-present comb (`11 00` × depth, ending in a unary node so no node has two terminal children), and at depth 100k over each family drive `ticks(1 << 40)`, `forks(3).collect()`, `let [a, b]: [Clock; 2] = clock.into()`, `join_all`/`sync_all` over the forks, `shape().count()`, and `own_version() == other.own_version()`; then make the doc enumerate exactly what is driven. If the owner prefers the narrow fix, reword 551-553 to list the ops exercised. Acceptance: committed depth-100k tests over left-spine, right-spine, and both-present families for the listed ops, each with a doc naming the walk it proves iterative; the headline no longer says "every public op" unless the roster is total over `surface::METHOD_SURFACE`'s `Clock::` rows. Construction: `let mut b = BitsBuf::new(); for _ in 0..100_000 { b.push(true); b.push(true); b.push(false); b.push(false); } b.push(true); b.push(false); b.push(false); b.push(false); Party::from_bits(b)` (a 100k chain of both-present nodes whose left child is terminal, ending in a unary node); run `Clock::from_parts(comb, Version::new())` through `ticks(1u64 << 40)`, `forks(3)`, `join_all`, `sync_all`, `shape().count()`, and `Clock::decode(&encode())`; mirror with `01` tags for the right spine. Any per-frame or per-right-descent recursion overflows the 2 MiB test-thread stack at this depth.

Ruled (100): Extend: flat-loop builders for a right spine and a both-present comb, the same operation set driven at depth 100k over each family, the doc enumerating exactly what is driven. The narrow-the-doc alternative is struck. See ../rulings.md.

### clock-28 (medium, verification): ruling 100

The static-orbit pin misstates its mechanism, its exchange schedule degenerates into four fixed partner pairs, and its collision branch is dead

- Owner-gated: no

Resolution: Pick an exchange schedule whose `(sender, receiver)` orbit covers every ordered pair (e.g. `s = r % N; t = (s + 1 + (r / N) % (N − 1)) % N`), re-measure the octave array, restate the mechanism from the coding (which delta codes grow, at 2 bits per doubling each), and either delete the static orbit's collision guard or assert the schedule's pair coverage as the topology's liveness floor; leave the churn orbit's guard (1358-1360) in place, since it fires. Acceptance: the mechanism sentence is derivable by hand from skyline.rs:16-24 and gamma.rs:28 and names the number of growing codes; a committed assertion (or a fixed-period schedule with a stated proof) shows every ordered peer pair exchanges; no unreachable branch remains in the static orbit's body.

Ruled (100): A schedule whose orbit covers every ordered pair, the octave array re-measured, the mechanism restated from the coding, the dead branch deleted or the pair coverage asserted as the liveness floor. See ../rulings.md.

### deps-3 (medium, verification): ruling 101

build.rs's figure-freshness check cannot fire on an incremental build; its two inputs are absent from rerun-if-changed

- Owner-gated: no

Resolution: add `cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg` and `cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg` beside the existing four, or list the `docs/` directory as the design note specifies (directory entries are scanned recursively by mtime). Acceptance: the construction below fails at the second build.

Ruled (101): `rerun-if-changed` is derived from the one enumeration of inputs the script reads, so the two lists cannot diverge. See ../rulings.md.

### module-graph-2 (medium, verification): ruling 101

`clippy-default` never lints the default-feature `before` library

- Owner-gated: no

Resolution: Add `cargo clippy -p before --lib -- -D warnings` to `clippy-default`, and re-state the 140-143 comment's mechanism as feature unification through the self-dev-dependency (the lib target is compiled without `cfg(test)` either way). Acceptance: an ungated `pub(crate) fn` used only from a `cfg(any(test, feature = "meter"))` site fails `just clippy-default` with `dead_code`. Construction: Run `cargo clippy -p before --lib -- -D warnings` at HEAD. Whether or not it is clean today, add the ungated helper described above and observe `just clippy` and the current `just clippy-default` stay green while the bare-lib command reports `dead_code`.

Ruled (101): Lint the bare library (default features and none) under `-D warnings` with no test targets; rewrite the recipe comment to name feature unification as the mechanism. See ../rulings.md.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### api-audit-15 (low, documentation): roster: approved (ruling 104)

Party::decode's Warning is Clock::decode's text and never names Party

- Owner-gated: no

Resolution: reword in terms of `Party` (the module doc at party.rs:10-17 already carries the right sentence). Acceptance: the warning names `Party`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-12 (low, simplification): roster: approved (ruling 104)

`Clock::decode`'s prefix-split block is duplicated verbatim in `Span::decode`, and two sites reach two layers down instead of through a component door

- Owner-gated: no

Resolution: One `pub(crate)` helper in `codec` beside `require_marker_padding` (respecting 61d00223's decision that the interior guard lives at the doors, not inside the validator), e.g. `fn padded_prefix_len(buf: &[u8], end: u64) -> Result<usize, Decode>`, owning the comment, the `div_ceil(8)`, the truncation check, the narrowing (as a `<= buf.len()` guard, dissolving the `expect`), and the padding check; both decoders call it. Add `pub(crate) fn Version::validate_canonical(bytes: &[u8]) -> Result<(), Decode>` for the two lines `Version::decode` and `Clock::decode` share, and `pub(crate) fn Party::index(&self) -> IdIndex<'_>` for the two `join_all`s. Acceptance: clock.rs contains no `crate::party::ops` or `crate::version::skyline` path; `git grep -n 'div_ceil(8)' crates/before/src` shows the boundary computation only in `codec`; `just test-all` green including borsh_impls's truncation-genre suite and `decode_never_panics`; wire snapshots untouched.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-19 (low, verification): roster: approved (ruling 104)

Fold-differential docs assert concrete hand-back outcomes the bodies check only by oracle agreement

- Owner-gated: no

Resolution: Either add the direct assertions (aliased case: `back.len() == 1` and `back[0].party()` covers exactly the aliased region; coalesced case: one hand-back whose party covers alias∪c∪d∪e and `acc.party()` covers the seed's residual plus a∪b), or phrase the docs as "agree with the oracle, whose discipline hands back ...". Acceptance: each doc sentence naming an outcome corresponds to an assertion in the body or is phrased as the oracle's decision.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-8 (low, documentation): roster: approved (ruling 104)

`sync_all` claims "maximally balanced, and therefore minimally large" with no derivation

- Owner-gated: no

Resolution: State what is derived: "Prefer this to iterated `sync`: one balanced fold does the joins, and the union is re-split into shares of minimal depth (`⌈log₂ k⌉`) rather than the linear spine iterated `sync` builds." Acceptance: the sentence states the minimal-depth property and no size superlative.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-11 (low, verification): roster: approved (ruling 104)

Five differential proptests share one body

- Owner-gated: no

Resolution: A helper `fn assert_wire_matches_reference<T: PartialEq + Debug>(stream: &[u8], subject: impl FnOnce(&mut &[u8]) -> io::Result<T>, oracle: impl FnOnce(&mut &[u8]) -> Result<T, Decode>) -> Result<Option<(T, usize)>, TestCaseError>` returning the accepted value and consumed length, so the span test can add its re-encode checks; the five bodies become one call each. Acceptance: five one-line proptest bodies; the helper's doc states the three agreements it asserts.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-3 (low, simplification): roster: approved (ruling 104)

serde `derive` feature enabled with no derive in the crate

- Owner-gated: no

Resolution: `features = ["alloc"]`. Acceptance: `cargo check -p before --features serde` and the serde test legs build clean.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-33 (low, documentation): roster: approved (ruling 104)

`RED_ZONE`'s derivation reasons about a release-profile measurement for a constant that compiles only under `cfg(test)`

- Owner-gated: no

Resolution: Restate the premise for the profile the guard runs in, or replace the derivation with the enforcement: "`STRIDE` × the largest test frame must stay under `RED_ZONE`; `meter::tests::stack_segment_meter_counts_deterministically_and_resets` (depth 200 000) and `clock::tests::deep_tree_stack_safety` are the committed proofs that the pair holds on every target the gate runs." Acceptance: the comment names the profile it reasons about and the committed tests that hold the constant to it.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-4 (low, documentation): roster: approved (ruling 104)

build.rs's module doc and the manifest comment describe only the fuelscape job; the "holds no constants" clause is contradicted by the v3 banner literal

- Owner-gated: no

Resolution: Add the figure job to Inputs (the results/ SVG and the committed README SVG) and Outputs (`$OUT_DIR/space_consumption.svg`; the opt-in source-tree write); mirror it in Cargo.toml:16-19; widen `theme_svg`'s first sentence to both targets. Name the banner version once (`const WIDGET_DATA_VERSION: u64 = 3;` used in the tuple and interpolated into the message) and amend lines 18-19 to say the format banner is the one deliberately shared constant, the consumer's pin on the compactor's number. Acceptance: every path `main` reads or writes appears in the module doc; `grep -n 'Some(3)\|v3' crates/before/build.rs` finds only the constant's definition and its interpolation.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-5 (low, verification): roster: approved (ruling 104)

build.rs declares `rerun-if-changed` for the fuelscape inputs only, so the README-figure freshness check does not rerun on the files it compares

- Owner-gated: no

Resolution: Add `println!("cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg");` and `println!("cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg");` beside the existing four. Acceptance: after a green `cargo build -p before`, appending a byte to `docs/itc_space_consumption_readme.svg` and rebuilding reruns build.rs and fails with the "is stale relative to results/space_consumption" message. Construction: With a warm build dir, append a whitespace byte to `crates/before/docs/itc_space_consumption_readme.svg` and run `cargo build -p before`: the script does not rerun and the build stays green; `touch crates/before/build.rs && cargo build -p before` then fires the check.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-9 (low, simplification): roster: approved (ruling 104)

borsh_impls.rs: `Ranked`'s decoder re-inlines `Rank`'s, the one-byte read is spelled three times, and full paths sit beside their imports

- Owner-gated: no

Resolution: Replace 242-247 with `let rank = Rank::deserialize_reader(reader)?;`; add `fn read_byte<R: Read>(reader: &mut R) -> Result<u8, Decode>` used by `read_bit` and `Rank::deserialize_reader`; import `validate_from`, `validate_dominating_from`, and `Admission` from `version::skyline` at the top; drop the `)` at 74; at 8 write "while preserving their canonical bytes". Acceptance: the differential suite, the pair matrix, and the genre pins in borsh_impls/tests.rs stay green; no `crate::version::skyline::` path remains in the file.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### module-graph-8 (low, verification): roster: approved (ruling 104)

The fuzz workspace's gate leg formats but never lints, unlike its four detached siblings

- Owner-gated: no

Resolution: Add `cargo +{{ nightly_toolchain }} clippy --all-targets -- -D warnings` to `fuzz-build` (the targets are ordinary `[[bin]]`s over `libfuzzer-sys`; clippy is check-only and the recipe already has the nightly toolchain), and extend the 363-364 comment to cover both lines. Cost: one clippy pass over 914 lines plus a check-only build of `before` with `laws,serde,borsh` in the detached target dir. Acceptance: a clippy warning introduced in `fuzz/src/lib.rs` fails `just gate`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### paper-fidelity-10 (low, verification): roster: approved (ruling 104)

the paper's system-level freshness clause (e′ ≰ any other live x) is not pinned

- Owner-gated: no

Resolution: add a population law beside `disjointness_invariant` (oracle/tests.rs:238) and an impl-side trace test: for every live clock `i` in a world, after `cs[i].tick()`, `!(cs[i].version() <= cs[j].version())` for all `j ≠ i`; equivalently the ownership invariant `cs[j].version() / cs[i].party() <= cs[i].version() / cs[i].party()`. Both ride the existing `world_strategy` populations. Acceptance: a committed test fails when `tick` is replaced by an inflation that another clock could already hold (for example, a tick that raises the whole version to the join of all live versions).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### paper-fidelity-11 (low, documentation): roster: approved (ruling 104)

the paper's peek and anonymous stamp have no named counterpart in the public docs

- Owner-gated: no

Resolution: one sentence at `Clock::version` or in the crate docs' "Replicating clocks between processes": a bare `Version` is the paper's anonymous stamp `(0, e)`; `version()` is `peek`, `send` is event-then-peek, `absorb`/`|=` is the anonymous join, `recv` is join-then-event; no anonymous `Party` exists, which makes `event`'s `i ≠ 0` precondition structural. Acceptance: `grep -n peek crates/before/src/lib.rs crates/before/src/clock.rs` hits public rustdoc.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-10 (low, documentation): roster: approved (ruling 104)

Maintainer-facing "single gate"/"single point" overclaims and small slips in `party.rs`, `sum.rs`, and `compare.rs`

- Owner-gated: no

Resolution: Extract `fn nonempty(bits: codec::BitsBuf) -> Option<Party>` (the emptiness gate plus `from_bits`); `finish_id` becomes `nonempty(bits).ok_or(Parse::Anonymous)` and `without` becomes `nonempty(self.view().diff(other.view()))`; describe it as the gate for every top-level `Party` built from possibly-empty bits (the kernels that prove non-emptiness structurally freeze through `from_bits`). sum.rs:11: "the point of overlap detection for `join`: a successful `sum` is the disjointness proof (`sum_split` detects it the same way on the spine)". party.rs:646 `mem::replace`; party.rs:45 `q` in both columns; party.rs:453 `assert_eq!(p.without(&q).unwrap(), keep);`; compare.rs:11 "The cursor form of `oracle::Party::is_disjoint` (the paper's disjointness invariant `i1 · i2 = 0`, as a test)". Acceptance: `grep -n 'id_is_empty' crates/before/src/party.rs` shows one site; `grep -n 'single gate\|single point'` over party.rs and sum.rs returns claims true of the code; every "form of" line in party/ops names the oracle function it mirrors.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-15 (low, documentation): roster: approved (ruling 104)

`Forks`' Complexity section promises a per-step cost that is never stated

- Owner-gated: no

Resolution: State the per-step bound the code has: each `next` performs at most `⌈log₂ k⌉` forks along the spine of one pending region, each `O(|p| + log k)` bits, so a step is `O(|p| log k)` worst case and `O(|p| + log k)` amortized over a full drain; verify the constants against `Split::next` (forks.rs:48-63) before landing. Acceptance: a reader following the pointer from `Party::forks` or `Clock::forks` finds a per-step bound in `Forks`' Complexity section.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-16 (low, simplification): roster: approved (ruling 104)

`IdBuilder` serves two clients with disjoint method sets and documents only one discipline

- Owner-gated: no

Resolution: Either rewrite the type doc to name both disciplines and which kernel uses which (final tags at descent plus fixed-width collapse for `sum`; reserve/patch/close for the leaf-driven builder), or move `Open`, `open`, `close_node`, and the reserve into `IdSkylineBuilder`, their only client, keeping `IdBuilder` as the shared core. Acceptance: `IdBuilder`'s doc names no method that only one of its two clients uses without saying so; `sum`/`diff` differentials unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-17 (low, documentation): roster: approved (ruling 104)

`Open` token doc claims an unclosed node "cannot compile"; `#[must_use]` is a warn-level lint on discarded expressions, and the module destructures the token itself

- Owner-gated: no

Resolution: Rewrite to what holds: "The token is `!Clone` and `#[must_use]`, so a discarded `open()` result warns and a token cannot be closed twice by accident; `IdSkylineBuilder` stores the position on `PosStack` and reconstructs the token at close, so the pairing there is kept by `close_up`'s stack discipline, not by the type." Alternatively have `PosStack` hand back `Open` tokens behind a method so the reconstruction is confined to one place. Acceptance: the `Open` doc makes no claim of a compile error; a reader can find where the token discipline is bypassed from the doc alone.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-20 (low, documentation): roster: approved (ruling 104)

Two `IdLeafCursor` types walk the same id coding; the decision to keep them separate is recorded only in an agent note

- Owner-gated: no

Resolution: At diff.rs:229-244 add one paragraph: "A second id cursor beside [`overlay::IdLeafCursor`], deliberately: that cursor settles eagerly inside its step (its plateaus are the stored regions), this one defers settlement so the covered-block scans can skip what the sweep never visits; the shared flip bookkeeping is a dozen lines, and a merge would wrap an eager adapter around the settle-driven cursor or import block machinery into the shape and masked walks." At overlay.rs:17 "Above them sit this module's two cursor instances" (or name the third with its home). Acceptance: a reader of either cursor's doc can find the other and the reason they are two.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-27 (low, documentation): roster: approved (ruling 104)

`split`'s spine walk and output copies sit outside the scan meter; the exemption lives only in a `tests/meter.rs` comment, and `ops.rs` points at a rationale `build_split` does not carry

- Owner-gated: no (routing the walk through the meter would be, and is left as an open question)

Resolution: State the exemption where it lives: at `build_split` ("the halves are verbatim slices of already-normal ranges plus one retagged node, so no tag is reserved, patched, or collapsed and the builder's placeholder discipline buys nothing; the spine read and the copies are deliberately outside the scan meter, whose fork envelope pins the raw path's near-zero reading"), at `sum_split::half`/`splice`, and as an explicit bullet in scan.rs:9-11's coverage list naming the two kernels whose reads and writes it does not count. Acceptance: `grep -n 'outside the scan' crates/before/src/party/ops/split.rs crates/before/src/codec/scan.rs` hits; ops.rs:40's pointer lands on the answer it promises.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-28 (low, simplification): roster: approved (ruling 104)

`split` and `sum_split` rest on a stream-suffix precondition the type does not carry: a dead `start` parameter, a redundant right-child scan asserted equal to `bits.len()`, and an unasserted twin in `branch_children`

- Owner-gated: no

Resolution: Make `split` and `sum_split` whole-stream operations in name and doc (they are today in every caller): drop `start` (`build_split(bits)` from 0), use `bits.len()` for `branch_end` and the capacity hint, keep the relation as `debug_assert_eq!(subtree_end(bits, right_child), bits.len(), ..)` so debug builds still check the root-entry precondition while release builds do no scan, add the same debug assert in `branch_children`, and state the precondition in both method docs ("the reader must be at a stream root"). If the owner prefers structural enforcement, type both entries on a root `BitsView` instead of a positioned reader. Acceptance: fork of `node(Some(&full()), Some(&leftmost(k)))` records a constant number of scan bits at any `k` instead of `4 + 2k`; the board's `party_fork` scan readings on right-heavy families move down and are re-pinned as a deliberate event; `d_fork_join_roundtrip`, `split_arbitrary`, and `sum_split_is_sum_then_split` stay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-30 (low, simplification): roster: approved (ruling 104)

`sum_split` re-accumulates the union spine that is already present verbatim in either operand

- Owner-gated: no

Resolution: Record `(self.bits(), self.pos())` before the loop and `self.pos()` after it (before the delegated `self.sum(other)` at 104 consumes `self`); `half` and `splice` take `(BitsView, Range<u64>)` and `extend_from_view` the range; delete `spine`; note in the method doc that the operand prefix is the union spine, which the spine argument already proves. Acceptance: `sum_split_is_sum_then_split`, `sum_split_collapsed_union_matches_terminal_split`, `sum_split_constructed::*`, and `sum_split_scan_never_exceeds_the_composition` pass with unchanged readings (spine pushes and `extend_from_view` are both unmetered, so `fused` does not move).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-31 (low, verification): roster: approved (ruling 104)

`join_all_hands_back_aliased_inputs` builds its alias by seeding two extra universes instead of `dangerously_alias`

- Owner-gated: no

Resolution: `let duplicate = shares[0].dangerously_alias(); let expected_back = shares[0].dangerously_alias();` and delete the two extra seeds. Acceptance: the test constructs one universe; its assertions are unchanged and still pass.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-32 (low, verification): roster: approved (ruling 104)

`parse_bare_notation` claims the `TryFrom` literals agree with the string parser but never compares them

- Owner-gated: no

Resolution: Assert `Party::try_from(lit).unwrap() == text.parse::<Party>().unwrap()` for each literal/text pair (`(1, 0)` and `"(1, 0)"`, `((0, 1), (1, (1, 0)))` and `"((0, 1), (1, (1, 0)))"`) and rename to `literal_doors_agree_with_the_parser`; or narrow the doc to what the body checks. Add `use crate::codec::built_view;` at the top of the file. Acceptance: the doc and the body state the same invariant; a literal door producing a different tree from the parser fails the test; `grep -c 'crate::codec::built_view' crates/before/src/party/tests.rs` is 0.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-33 (low, verification): roster: approved (ruling 104)

The deep `is_disjoint` differential compares production against production, and `covers` has no deep differential at all

- Owner-gated: no

Resolution: In the deep test add `prop_assert_eq!(walk, to_oracle_party(x).is_disjoint(&to_oracle_party(y)))` and a parallel `covers` leg in both roles over the same shape triples, keeping the scale under `ORACLE_SCALE_MAX`. Acceptance: the deep test's doc names the oracle as the reference; a `covers` verdict is checked against the oracle at scale at least 64 in both operand orders. Construction: the shape pairs already built there suffice (`(&a, &a)` gives the overlapping verdict, the skip-stress pair the disjoint one); for `covers`, add `(shape_party(shape, scale), shape_party(shape, scale / 2))` and a `node(Some(&a), None)` wrapper so the `true` arm is reached. The test today would not fail if `IdIndex` and `IdReader` shared a wrong verdict on a deep pair.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-34 (low, verification): roster: approved (ruling 104)

`sum_split_scan_never_exceeds_the_composition` compares mixed currencies: the composed side counts `sum`'s builder writes, the fused side counts reads only

- Owner-gated: no

Resolution: Either meter the fused side's writes and the spine read (the re-pin route of party-27, which also moves `ID_FORK`'s scan column and the `spliced == 8` constant) so both sides count reads plus writes, or keep the raw path and compare like with like (subtract `sum`'s write bits from `composed`, or compare against a reads-only composition) and correct the doc to name exactly which regressions the assertion convicts. Acceptance: the construction below fails the test; the doc's stated conviction matches what the assertion can refute. Construction: on the committed "adjacent k" regime (`a = leftmost(k)`, `b = spine(k - 1, true, node(None, Some(&full())))`), inject the regression the doc names: in `branch_children` (sum_split.rs:175-179) add a second `IdReader::at(bits, start).skip()` after `probe.skip()`. `fused` rises by 2 bits and `fused <= composed` still holds at both scales; only a re-walk of at least about `2k` bits (re-scanning the spine) trips the assertion today.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-12 (low, documentation): roster: approved (ruling 104)

from_num's doc names the decoder as a producer it does not serve

- Owner-gated: no

Resolution: "The shared normalization the folds and the accumulator readout land through, which also re-dispatches the stripped numerator onto its canonical arm. The decoder is the one producer that bypasses it: strict minimal packing already guarantees an odd numerator whenever `exp > 0` (its debug_assert states the premise), so it constructs the normalized value directly." Acceptance: the doc's producer list equals the call graph and the decoder's bypass is stated at `from_num` or at line 817.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-19 (low, documentation): roster: approved (ruling 104)

Public rustdoc names internals and privately defined terms

- Owner-gated: no

Resolution: 879 "Ranks whose integer parts of log2 differ settle in O(1):"; 1064-1069 "Superlinear, subquadratic in the rank's width (decimal conversion); on 32-bit targets, numerators above ~2^32 bits, reachable only from hundreds of megabytes of decoded input, render quadratically."; ranked.rs:376 "One walk over both versions, no Rank materialized:" (see rank-33 for the bound itself); ranked.rs:243 "Each component's own variants". Acceptance: no public doc in the partition uses "backend", "magnitude class", "co-sweep", "signed", or "genre" without defining it in the same public doc.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-25 (low, simplification): roster: approved (ruling 104)

Num::msb_cmp's (Base, Base) arm is a no-op distinction and Base::msb_cmp is a one-caller wrapper

- Owner-gated: no

Resolution: Delete `Base::msb_cmp`; spell the four arms uniformly and rewrite the doc: "every pairing streams both arms' MSB windows through msb_cmp_windows; the arms differ only in their limb source". Optionally collapse to one arm with a small two-variant `Iterator` over the limb source so `Num::msb_windows` has one return type. Acceptance: `grep -rn 'Base::msb_cmp' crates/before/src` is empty; `msb_cmp_matches_the_aligned_oracle` and `rank_wide_arm_cmp_agrees_with_the_alignment_oracle_on_10k_pairs` pass unchanged; the limb-meter reading of `Rank::cmp` on any base pair is unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-27 (low, simplification): roster: approved (ruling 104)

Two sub-limb right-shift kernels in num.rs

- Owner-gated: no

Resolution: One `fn shr_in_place(limbs: &mut [u64], bits: u32)` documented for `1..64` (caller guards zero): `materialize_be` calls it when `pad > 0`; `shr_limbs` becomes `let mut tail = self.limbs[whole..].to_vec(); if bit != 0 { shr_in_place(&mut tail, bit) }; tail` and lets `from_limbs` trim the top. Optionally one `fn le_bytes_minimal(limbs: &[u64]) -> Vec<u8>` for the three flattenings. Acceptance: one occurrence of the `<< (64 - ` combine pattern in num.rs; `materialize_is_exact_and_canonical` (all pads) and `shr_matches_the_oracle_and_redispatches` (amounts 0..512) pass unchanged; the wasm32 fraction pins pass unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-30 (low, verification): roster: approved (ruling 104)

num/tests.rs's testdoc claims hashing coverage the body lacks, and its module doc overstates "every wide-arm operation"

- Owner-gated: no

Resolution: Add a hash-coherence clause (`a == b` implies equal `DefaultHasher` outputs for `na` and `nb`) or drop "and hashing" from the doc and cite `rank_cross_path_normalization`. Narrow the module doc to the operations pinned here (dispatch, assembly, shifts, bias steps, windows, rendering, equality) and point at the version/tests.rs wide-regime suites and the `RANK_TRIPLE` law run for `to_be_bytes`, `from_base`, `fold_into`, and `Hash`; or add oracle tests for those four. Acceptance: every `pub(crate)` fn on `Num` and every trait impl with a `Wide` arm either has a test in this file whose doc matches its body, or is named in the module doc with the covering suite cited.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-5 (low, documentation): roster: approved (ruling 104)

The exp field doc states the bound for one construction path

- Owner-gated: no

Resolution: On the field: "Bounded by bits already resident: a version-derived exponent by its tree's stored bit length (each level halves the interval), a decoded exponent by the fraction bits actually read, and a sum by its operands' maximum; under 2^35 on a 32-bit target, which keeps every usize-indexed digit position (suanpan's documented panic) unreachable." Reduce 579-582 and 1023-1027 to a citation of the field's bound. Acceptance: the two-case bound appears once, on `exp`; the two consumers cite it.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-8 (low, simplification): roster: approved (ruling 104)

The two-route dispatch is written twice, threaded by bool parameters

- Owner-gated: no

Resolution: One local `enum Op { Add, Sub }` with `fn headroom(self) -> u64` and `fn backend(self, a: Base, b: &Base) -> Base`; one `fn combine(lhs: &Rank, rhs: &Rank, op: Op) -> Rank` holding the routing and its comment once; `Add::add` becomes `combine(self, rhs, Op::Add)` and the Greater arm `Some(combine(self, other, Op::Sub))`; `accumulate` and `Num::fold_into` take `Op` instead of `bool`. Acceptance: one call site of `backend_alignment_fits`; no `bool` parameter on `accumulate` or `fold_into`; the `RANK_TRIPLE` laws, `rank_wide_arm_arithmetic_matches_the_backend_oracle`, the wasm32 `pin_rank_add`/`pin_rank_checked_sub`, and the `RANK_PAIR_MISMATCH`/`RANK_SUM_MIXED` envelopes read identically (pure refactor).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-9 (low, documentation): roster: approved (ruling 104)

Rank::decode's "# Decoded size" section describes the encoded size and cites text that lives on encode

- Owner-gated: no

Resolution: Move the bound onto `Rank::encode` after its suffix-safety paragraph as "# Encoded size", or into the type-level `# Complexity` beside the `‖r‖` definition; delete the section from `decode`. Acceptance: `decode`'s doc has no size section; the 9/8 bound appears once, following the suffix-safety text it references.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-11 (low, simplification): roster: approved (ruling 104)

`intersect_points` is the swapped hull: one `span_refs` call replaces the equality compare plus two emission walks, and its comment names a rescue that cannot happen

- Owner-gated: no

Resolution: `fn intersect_points(a: &Version, b: &Version) -> (Version, Version) { let (lo, hi) = Version::span_refs(a, b); (hi, lo) }`, with the doc stating the swap and replacing the parenthetical with the monotonicity argument ("a crossed pair stays crossed under further join-`lo`/meet-`hi` legs, so the closing `partial_cmp` decides for the whole family"), cited from `intersect_all`. Note `span_refs` records `hull_traffic` rungs; no committed intersect snapshot exists, so nothing committed moves. Acceptance: a case in the pointwise laws asserting `intersect_points(a, b) == { let (l, h) = span_refs(a, b); (h, l) }` over arbitrary pairs; `nary_doors_match_sequential_folds_on_a_mixed_family` and the intersect laws stay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-12 (low, simplification): roster: approved (ruling 104)

The span operators are encoded twice: `*_core` kernels re-implement the `SpanFoldOps` table with inline fast paths, and two ordering matches restate `Span::new`

- Owner-gated: no

Resolution: one private `fn combine(a: &Span<'_>, b: &Span<'_>, ops: &SpanFoldOps) -> (Version, Version)` that does `match (a.point(), b.point()) { (Some(va), Some(vb)) => (ops.points)(va, vb), _ => ((ops.lo_refs)(a.lo(), b.lo()), (ops.hi_refs)(a.hi(), b.hi())) }` (lifting `Group::point` for `Input` to a `Span::point` helper). `union_core`/`join_core`/`meet_core` become `Span::owned(combine(a, b, &OPS))`; `intersect_core` keeps its point fast path and otherwise becomes `let (lo, hi) = combine(a, b, &INTERSECT_OPS); Span::new(lo, hi).ok()`; `intersect_all`'s closing match becomes `Span::new(lo, hi).ok()`; `fold_endpoints`'s `(Input, Input)` arm calls the same `combine`. Acceptance: `Version::join_refs`/`meet_refs`/`span_refs` each appear once in algebra.rs (in the `*_OPS` constants or `*_points`); the two hand-written ordering matches are gone; the span algebra laws and `nary_doors_match_sequential_folds_on_a_mixed_family` stay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-13 (low, documentation): roster: approved (ruling 104)

Maintainer docs in `algebra.rs` give the wrong reason for the missing identities and a clone-discipline sentence the code contradicts

- Owner-gated: no

Resolution: union: "union has no identity span: the identity would be the empty set of versions, and every [`Span`] is nonempty (`lo <= hi`)". Intersection: "intersection has no identity: the identity would be the span covering every span, `[bottom, top]`, and the version lattice has no top." Fold doc: "inputs are borrowed into the counter; the dedup filter holds one refcount pair, and every clone of a stored version is a refcount bump, never a byte copy." Acceptance: the three sentences read true against the code; "covered by" at 1119 becomes "covers".

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-15 (low, simplification): roster: approved (ruling 104)

`OwnSpan` hand-writes the nine-way and three-way verdict tables that `place.rs` and the `verdict.rs` docs already state

- Owner-gated: no

Resolution: `pub(crate) fn Placement::from_relations(vs_lo: Option<Ordering>, vs_hi: Option<Ordering>) -> Placement` (and `Dominance::from_relations`, `Precedence::from_relations`) in verdict.rs, stated once beside the variant docs, with `(None, None) => Concurrent(Both)` as the total definition (what own.rs returns today at line 127); `OwnSpan::{place,dominance,precedence}` call them; `Span::place`'s coincident rung calls `Placement::from_relations(r, r)`; `place::span`'s closure calls it after its `debug_assert`. Acceptance: one nine-arm `Placement` table in production code; `own_span_place_reaches_every_concurrent_corner`, `own_span_matches_the_projected_span`, `span_place_places_every_witness`, and `coincident_span_rungs_agree_across_buffer_identity` stay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-3 (low, simplification): roster: approved (ruling 104)

The clone-identity coincidence certificate is spelled inline at five production sites while `Span::is_coincident` names it

- Owner-gated: no

Resolution: make `is_coincident` `pub(crate)` and call it at span.rs:255, 315, 380, 459 and query.rs:129. For the version-pair sites (algebra.rs:361, 562; version.rs:1262) add a `pub(crate) fn Version::shares_buffer(&self, other: &Version) -> bool` so `is_coincident` is `self.lo.shares_buffer(&self.hi)` and `DedupRuns` reads `prev.shares_buffer(version)`. Acceptance: `grep -rn '\.view()\.ptr_eq(' crates/before/src --include='*.rs' | grep -v tests` returns only the helper bodies.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-31 (low, simplification): roster: approved (ruling 104)

`Query` is built by sixteen struct literals across four files; the one-hole forms and the `Conjoin` lifts restate constructors that exist

- Owner-gated: no

Resolution: private constructors in query.rs beside `unbounded()`: `fn from_hole(hole: Hole<'a>) -> Self` and `fn bounded(floor, ceiling) -> Self`; `or_concurrent` = `Query::from_hole(Hole { at: self.at, strict: true })`, `Not` = `Query::from_hole(Hole { at: self.at, strict: false })`, the strict forms set the bound on the result, and `Conjoin::lift` for `Floor`/`Ceiling` = `Query::<Neutral>::from(self).adopt()`. Acceptance: the `PhantomData` count drops to the actual constructors (unbounded, from_hole/bounded, clone, into_owned, adopt, and); `forms_keep_their_relations`, `conjunction_normalizes`, `debug_renders_expressions`, and the conjunction laws stay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-4 (low, documentation): roster: approved (ruling 104)

`Span::dominance`/`precedence` comments explain the fast path in the consumer's vocabulary ("compressed-subtree classification")

- Owner-gated: no

Resolution: delete both sentences, or restate in before's terms ("a caller holding many coincident spans classifies each against one stream"). Acceptance: the grep returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-8 (low, verification): roster: approved (ruling 104)

The span folds' time and space claims are priced only by proxy `version_*_all` cells that never execute `fold_endpoints`

- Owner-gated: no

Resolution: state the space argument once at `fold_endpoints`'s doc. Add a board cell (heap and scan currencies) over a mixed point/wide family so both combine arms run, or a `scan-meter` two-scale row in tests/meter.rs, and map the four roster rows to it instead of the `version_*_all` proxies. If span-causally-9 unifies the folds first, the proxy becomes sound and only the argument is owed. Acceptance: rewriting `fold_endpoints` as a sequential left fold (or making the dedup filter copy bytes) fails the new cell at two scales; the current code passes.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-12 (low, documentation): roster: approved (ruling 104)

`span_refs`'s doc claims its rungs match `join_refs`'s order; `join_refs` tests the empty rungs the other way round

- Owner-gated: no

Resolution: "the same three rungs; after the equal rung at most one operand is empty, so the two empty rungs' order is free", and drop "keep the three in lockstep". Acceptance: no doc in the file claims an order the three `_refs` functions do not share.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-18 (low, documentation): roster: approved (ruling 104)

"so it is not monotone under `<=`" reads as a claim about projection, which the same paragraph calls a lattice homomorphism

- Owner-gated: no

Resolution: name the subject: "Projection can still raise `min_ticks` (carving one broad tick into disjoint peaks): `min_ticks` is not monotone under `<=`, though projection itself is." Same edit at tests.rs:2161. Acceptance: neither sentence can be read as "projection is not monotone".

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-22 (low, documentation): roster: approved (ruling 104)

`Ticks` type doc: unclosed parenthesis, a semicolon splice with a verb-agreement slip, the text-I/O bound stated twice, and "then ascending"

- Owner-gated: no (syntax slips; the owner authored them, the fix is uncontroversial)

Resolution: (a) close the parenthesis after "unsigned machine integers)" and start a new sentence for the conversions out; (b) "produced by `min_ticks` and consumed by `Version::ticks`, `Party::ticks`, and `Clock::ticks`, each of which takes `impl Into<Ticks>`"; (c) drop the text-I/O clause from 41-43 and keep 45-48; (d) "least significant first". Acceptance: the doc parses as English with balanced parentheses and states each bound once.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-25 (low, documentation): roster: approved (ruling 104)

The `From<u8..u128> for Ticks` impls carry no rustdoc; the doc sits on the macro, and `From<usize>` alone is documented

- Owner-gated: no

Resolution: move the doc inside the expansion (`$( #[doc = "A count from a machine integer: total, `O(1)`."] impl From<$t> for Ticks { ... } )*`), or fold `usize` into the macro so all six are documented identically. Acceptance: every `From<_> for Ticks` impl shows the same one-line doc in rendered rustdoc.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-28 (low, simplification): roster: approved (ruling 104)

Two proptests assert the same `|=` cells on the same population

- Owner-gated: no

Resolution: delete `version_assign_join_matches_oracle` and fold its motivation ("neither of which the by-value `|` differential reaches") into `join_assign_matrix_matches_oracle`'s doc, whose name mirrors the meet dual. Acceptance: one `|=` differential remains beside one `&=` differential; the seed file needs no change (both draw `(ops, i, j)` from `world_strategy`).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-32 (low, verification): roster: approved (ruling 104)

The provenance-linearity test quotes measurements its body never produces, and its single-scale ceiling cannot see a change of order

- Owner-gated: no

Resolution: measure each depth-parameterized family at two depths (`d` and `2d`) and assert the encoded-rank-bits to version-bits ratio does not grow beyond a stated slack; either assert the per-family ratios as a band (turning the slack into a tightened pin) or delete the five figures from this doc and from rank.rs:36-38, leaving the enforced 1.0 bound as the only number in prose; fix the stray `\[`/`\]` escapes either way. Acceptance: every numeric ratio in the doc is asserted by the body or absent; the test fails when the rank encoder is replaced by one whose output grows as `version_bits · log(depth)`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-33 (low, verification): roster: approved (ruling 104)

`div_decomposes_along_fork`'s doc claims a disjoint-party check its body does not perform

- Owner-gated: no

Resolution: drop the final clause (the next test owns it), or add the disjoint-party assertion here and keep the sentence. Acceptance: every clause of the doc corresponds to an assertion in the body.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-7 (low, documentation): roster: approved (ruling 104)

`span_all` advises against an operation that does not exist and links `span` to `Version::meet`

- Owner-gated: no

Resolution: name the true alternative, which 638-651 already describes: "Prefer this to computing [`meet_all`](Self::meet_all) and [`join_all`](Self::join_all) separately: one balanced fold carries both endpoints and reads each input once." Drop the `Version::meet` link. Acceptance: the sentence names an operation a caller could write instead, and every intra-doc link in the `span_all` doc resolves to the item its text names.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### api-audit-17 (nit, documentation): roster: approved (ruling 104)

Doc-link and spelling slips in public rustdoc

Where: `crates/before/src/version.rs:603-604`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): fix the link target to `Version::span` and the five typos

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### api-audit-20 (nit, documentation): roster: approved (ruling 104)

The error module's first sentence is a rhetorical question

Where: `crates/before/src/error.rs:1-1`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): a descriptive sentence, e.g. "The error types: decode, parse, overlap, crossed-span ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clippy-pedantic-5 (nit, documentation): roster: approved (ruling 104)

87 doc links spelled `[`name`]`(args)`` render as two adjacent code spans with the link on only the first

Where: `crates/before/src/span.rs:36-37`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): mechanical pass: `[`Span::new(lo, hi)`](Span::new)`,

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-13 (nit, simplification): roster: approved (ruling 104)

The manual `Debug` for `Clock` is exactly the derive's expansion

Where: `crates/before/src/clock.rs:916-923`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Derive `Debug`; delete the impl

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-15 (nit, documentation): roster: approved (ruling 104)

The shared operator doc says "in either operand order" on the `\

Where: `crates/before/src/clock.rs:1049-1058`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): =` cells, where order is fixed

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-16 (nit, simplification): roster: approved (ruling 104)

A one-caller helper and a redundant reborrow line in `Forks::new`/`next`

Where: `crates/before/src/clock/forks.rs:31-51`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Inline `clock`; delete the reborrow line

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-18 (nit, documentation): roster: approved (ruling 104)

Pointer comment claims "every feed order"; the law drivers sample pool-indexed picks

Where: `crates/before/src/clock/tests.rs:16-22`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "driven at boundary-band arities with pool-indexed picks (repeats and arbitrary orders arise from the draws)" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-23 (nit, documentation): roster: approved (ruling 104)

Test doc cites AGENTS.md (a reverse citation)

Where: `crates/before/src/clock/tests.rs:555-557`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the parenthetical (naturally done while rewriting the headline under clock-22)

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-5 (nit, documentation): roster: approved (ruling 104)

The `result_large_err` allows are live at exactly the threshold, and only one of the two sites says why

Where: `crates/before/src/clock.rs:257`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Move the rationale to `join_all` (the site sync_all cites), stating the mechanism in one line: "the combine closure's `Err` is two `Clock`s ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-6 (nit, documentation): roster: approved (ruling 104)

`join_all`'s maintainer comment re-derives `Party::join_all`'s argument instead of stating the delta

Where: `crates/before/src/clock.rs:262-274`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Reduce to the delta: `Party::join_all`'s fold carrying versions; the accept test indexes `self`'s party; the combine is `Clock::join` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clock-7 (nit, simplification): roster: approved (ruling 104)

The join-as-fallible-combiner adapter closure is spelled three times

Where: `crates/before/src/clock.rs:280-283`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A private `join_group` adapter per type; propose a `FnMut(&mut T, T) -> Result<(), T>` combiner shape for `fold`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-12 (nit, simplification): roster: approved (ruling 104)

borsh_impls/tests.rs: qualified paths beside their imports, hand counts, a likelihood argument, and formatting slips

Where: `crates/before/src/borsh_impls/tests.rs:829-831`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Use the imported names; name the prefix; drop the tallies; rewrap

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-13 (nit, documentation): roster: approved (ruling 104)

`error` module's listing sentence is a joke, and three error types derive `Default` nothing defaults

Where: `crates/before/src/error.rs:1-1`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A first sentence such as "The error types every fallible operation returns.", with the joke kept as a second line if wanted ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-19 (nit, simplification): roster: approved (ruling 104)

`Vec::pop_if` removes both `expect`s and the `Option` juggling in the counter loop

Where: `crates/before/src/fold.rs:53-78`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `Vec::pop_if` and a labeled `continue`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-23 (nit, documentation): roster: approved (ruling 104)

Unclosed backtick in the `Clock::forks` intra-doc link

Where: `crates/before/src/iter.rs:9-10`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `//! [`Clock::forks`](crate::Clock::forks).` Acceptance: the rendered `before::iter` page shows both links as code spans.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-30 (nit, documentation): roster: approved (ruling 104)

Register and redundancy nits in the crate-level docs and neighbors

Where: `crates/before/src/lib.rs:350-358`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): 350 "The operations hold their bounds on every input shape."; 357 "with the bound holding on the rest"; 381 "Consequently" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-36 (nit, verification): roster: approved (ruling 104)

serde testdocs: "Both deserialization paths" lists three formats, "every rejection genre" covers three, "The new impls" is dated.

Where: `crates/before/src/serde_impls/tests.rs:26-30`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Reword to what each body drives; drop "new".

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-39 (nit, simplification): roster: approved (ruling 104)

Four shape iterators carry an identical `finished` flag and byte-identical `size_hint` bodies

Where: `crates/before/src/shape.rs:180-186`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `open_hint` helper, or route through `advance_refinement`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### crate-root-8 (nit, simplification): roster: approved (ruling 104)

Em-dashes in `//` comments across the partition

Where: `crates/before/src/borsh_impls.rs:89-91`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Colons or semicolons at the listed sites; one dash form in the Quickstart

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### deps-12 (nit, documentation): roster: approved (ruling 104)

docs.rs metadata enables no feature, so the serde and borsh impls the crate docs advertise never render there

Where: `crates/before/Cargo.toml:13-14`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): add `features = ["serde", "borsh"]` to the docs.rs table (not

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### deps-15 (nit, documentation): roster: approved (ruling 104)

manifest feature summaries lag their module docs: limb-meter lights two counters, touch-meter counts more than the manifest says

Where: `crates/before/Cargo.toml:74-84`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): one clause each ("...and a second column counting

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fresh-eyes-12 (nit, documentation): roster: approved (ruling 104)

Party::decode's warning is written about Clock

Where: `crates/before/src/party.rs:605-610`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Restate in terms of `Party`: "Serializing a [`Party`] circumvents its otherwise compiler-enforced `!Clone` linearity ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### inventory-10 (nit, documentation): roster: approved (ruling 104)

Two doc slips: `span_all` links "span" to `Version::meet`; "seach" typo

Where: `crates/before/src/version.rs:603-604`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): link to `Version::span`; "seach" to "each" and "endpoint" to

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-18 (nit, simplification): roster: approved (ruling 104)

`#[allow(clippy::wrong_self_convention)]` on `covers` guards nothing

Where: `crates/before/src/party/ops/compare.rs:32-34`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the attribute

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### party-6 (nit, simplification): roster: approved (ruling 104)

Idiom nits: `from_frozen` bypassed by `decode`/`seed`, qualified `core::fmt`/`crate::`/`std::io` paths beside a once-used import, `then(\

Where: `crates/before/src/party.rs:19-19`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): \

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### prose-hygiene-13 (nit, documentation): roster: approved (ruling 104)

Contract paragraph intensifiers and a significance adverb in public rustdoc

Where: `crates/before/src/lib.rs:350-357`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "The operations in this crate are hardened against pathological

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-11 (nit, documentation): roster: approved (ruling 104)

The same shouted warning appears verbatim on raw_parts and from_raw; the invariant it guards is never stated positively

Where: `crates/before/src/version/rank.rs:493-509`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): State once, on `from_raw`: "Crate-private, and must stay so together with `raw_parts`: every public construction path bounds `exp` by bits already res ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-14 (nit, simplification): roster: approved (ruling 104)

suanpan's private digit width is hardcoded as 32 in before

Where: `crates/before/src/version/rank.rs:596-599`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Export `DIGIT_BITS` or add `reserve_bits` in suanpan

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-17 (nit, simplification): roster: approved (ruling 104)

Idiom nits: ilog2, a redundant rename, an expect that is not a proof, qualified paths, asymmetric twins, a re-export rename

Where: `crates/before/src/version/rank.rs:629`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Apply as listed; `unwrap_or(0)` or a premise-stating expect

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-28 (nit, simplification): roster: approved (ruling 104)

Wide's documented invariant is violated by from_limbs's transient, which is the only reason Wide::bits has a zero arm

Where: `crates/before/src/version/rank/num.rs:372-377`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A `limb_bits` helper decides the arm; `Wide::bits` drops its zero arm

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-3 (nit, documentation): roster: approved (ruling 104)

Copyedit slips in the public Rank and Ranked type docs

Where: `crates/before/src/version/rank.rs:152-166`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "strictly monotone in the causal order"; "so any tiebreak between equal ranks extends the causal order to a total one" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### rank-7 (nit, verification): roster: approved (ruling 104)

A `None` or zero result allocates nothing (rank.rs:287, 343) has no allocation-counting instrument; the pair envelope meters `cmp`, `checked_sub`, and `+` together and the board's heap floor is NA.

Where: `crates/before/src/version/rank.rs:287`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A counting-allocator row for `checked_sub` alone on wide operands asserting zero heap delta.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### recursion-7 (nit, documentation): roster: approved (ruling 104)

Doc wording inverts iterative and recursive at three sites

Where: `crates/before/src/party/ops/split.rs:19-19`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): split.rs:19 -> "The cursor form of `oracle::Party::split`";

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-14 (nit, documentation): roster: approved (ruling 104)

`OwnSpan`'s composed two-comparison verdicts are a deliberate design whose rationale lives only in history

Where: `crates/before/src/span/own.rs:88-90`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): one sentence on the type or the `place` doc: the verdicts compose two masked comparisons (no fused masked placement walk exists ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-19 (nit, documentation): roster: approved (ruling 104)

`Dominance` and `Precedence` docs drift in vocabulary between mirrored variants

Where: `crates/before/src/span/verdict.rs:69-85`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): one noun ("version", matching the parameter name) and one relation phrase ("concurrent to", the crate's public term) in both enums

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-20 (nit, documentation): roster: approved (ruling 104)

`wire.rs` calls the endpoints "the meet" and "the join" where the rest of `Span` says `lo`/`hi` and `meet`/`join` name the operators

Where: `crates/before/src/span/wire.rs:40-41`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `lo`/`hi` (or "the lower endpoint") in wire.rs and in the span/tests.rs genre strings ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-21 (nit, simplification): roster: approved (ruling 104)

`Span::encode` builds the composite asymmetrically and reallocates once

Where: `crates/before/src/span/wire.rs:44-48`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `[lo.as_bytes(), hi.as_bytes()].concat()`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-23 (nit, simplification): roster: approved (ruling 104)

The wire decode tests `Admission::Refuted` twice: an early return, then an `unreachable!` arm for the same variant

Where: `crates/before/src/span/wire.rs:156-172`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A total match on `Admission` at the decision point

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-29 (nit, documentation): roster: approved (ruling 104)

`conjoin!` gives every `&` cell one doc line and the hole-bearing island, including the seven hole-free atom cells

Where: `crates/before/src/causally/conjunction.rs:150-154`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): let the macro take a per-group doc string (atom x atom, atom x polar, polar x polar) so each cell states its output polarity ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-30 (nit, documentation): roster: approved (ruling 104)

`Floor`/`Ceiling` public docs state their predicate with the private field name `at`

Where: `crates/before/src/causally/forms.rs:19-32`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `s <= v` / `v <= e` (matching `after(s)` / `before(e)`), or "bound <= v"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-32 (nit, simplification): roster: approved (ruling 104)

`Hole.strict` and the `hole_demand`/`hole_name` dispatch take a bare `bool` for a two-valued domain concept

Where: `crates/before/src/causally/polarity.rs:19-22`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `enum Bound { Inclusive, Strict }`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### span-causally-6 (nit, simplification): roster: approved (ruling 104)

The `Cow<Version>` `From` impls live in `span.rs` and claim a span-only purpose, but `causally::forms` depends on them

Where: `crates/before/src/span.rs:605-627`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Move both impls to version.rs; reword the doc

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-19 (nit, simplification): roster: approved (ruling 104)

`Div<&Party> for &Version` lives in version.rs and constructs `OwnVersion` through `pub(crate)` fields

Where: `crates/before/src/version.rs:1703-1711`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Move the `Div` impl into own.rs, or state the grouping

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-2 (nit, simplification): roster: approved (ruling 104)

The `M` caption is pasted five times with inconsistent wrapping

Where: `crates/before/src/version.rs:293-293`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Emit the caption from the island renderer, or hoist it to one `Rank` note

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-20 (nit, simplification): roster: approved (ruling 104)

`causal_cmp_impls!` has one invocation and is the fixed-body twin of `view_cmp_impls!`

Where: `crates/before/src/version.rs:1721-1760`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One body-parameterized fan-out macro

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-21 (nit, verification): roster: approved (ruling 104)

`from_impl_is_to_version`'s doc says "without re-projecting" while `From` and `to_version` each project.

Where: `crates/before/src/version/own/tests.rs:45-55`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Reword to what the asserts show (the `Copy` view stays usable).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-29 (nit, verification): roster: approved (ruling 104)

`trace_ticks`'s doc enumerates the ops but omits `Op::Ticks`, which the body charges `k`.

Where: `crates/before/src/version/tests.rs:656-672`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Add the arm to the enumeration or restate structurally.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-31 (nit, simplification): roster: approved (ruling 104)

The two rank-order sweeps duplicate the perturbed-pair construction verbatim

Where: `crates/before/src/version/tests.rs:857-885`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `adversarial_pair` helper

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-34 (nit, verification): roster: approved (ruling 104)

The at-rest size pin hard-codes `32`/`64` beside the invariant it states; on a 32-bit host it would fail on a number, not the invariant.

Where: `crates/before/src/version/tests.rs:2195-2200`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Derive from `size_of::<usize>()` and `size_of::<Party>() + size_of::<Version>()`, or cfg-gate.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### version-core-4 (nit, simplification): roster: approved (ruling 104)

Rustdoc link syntax inside `//` comments is inert

Where: `crates/before/src/version.rs:385-386`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Plain paths in `//` comments

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

