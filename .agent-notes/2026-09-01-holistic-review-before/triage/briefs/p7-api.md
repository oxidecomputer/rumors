<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P7 lane: the API block

## Goal

The public surface of `before` and `suanpan` changes exactly as rulings
81 to 87 direct and in no other way. `crates/before/AGENTS.md` declares
the API stable, so every change here is adopted on Finch's explicit
word, recorded in those rulings; a lane agent adds, removes, or reshapes
nothing the rulings do not name. Signature changes (`forks(k: usize)`,
the fork iterator type names, the `suanpan` shift deletions) are each
named in their commit as an owner-directed change on the stable surface.
Additive changes (`must_use`, `Hash`, `#[source]`, the `Ticks` and
`Limbs` impls, `coverage`, `const fn`, `non_exhaustive` on `Decode`) land
as such. The text forms and the human-readable serde branch (ruling 85)
are new surface, not wire: no wire snapshot moves, and anything that
would move one is a stop.

Ruling 82 supersedes ruling 81's item (4) and restates ruling 35 at
`usize::MAX`; ruling 85 completes ruling 84's item (12). Three entries
under decision 17 are held (listed at the end) until pins this lane
lands have been read.

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `59998ad0` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `59998ad0`, fast-forward; if it has diverged, stop and report. Never call
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

## Ordering

- This lane edits rustdoc that `p3-vocabulary`, `p4-ghosts`, and
  `p4-rosters` also touch (the coinage sweep, the ghost-reference
  rewrites, the `# Errors`/`# Panics` pass). It runs after those lanes
  have landed on main, or rebases onto them before its final gate run;
  the coordinator names the base SHA at launch.
- The `forks(k: usize)` change (ruling 82: clock-17, party-13,
  api-audit-10) is this lane's and supersedes the entries for those ids
  and for clock-3, tests-other-17, and party-14 (ruling 35) in
  `p2-widths.md`, which now defer to this brief. If `p2-widths` has
  already landed a `u64` version of ruling 35, this lane rebases and
  changes the type; `tests/forks_max.rs` is re-pinned at `usize::MAX`.
- Ruling 8 applies: any change to the instrument surface (`pub mod
  meter`, `Base` in signatures, the test-local `Ticks` helper's
  neighbors) lands in the same commit as the `rumors` test update it
  entails. A change that would require editing `rumors`' public rustdoc
  is a stop.
- Order inside the lane: (1) the additive attributes and impls
  (rulings 81 items 2 and 5, 83, 84 item 10, 86 items 14 to 17's
  additions); (2) the type renames and the `usize` signature (81 item 3,
  82); (3) the `Parse` precedence rule and ASCII trim with its pins (84
  item 11); (4) `serde_bytes` deserialization, the one macro, the
  `serde_test` byte pins (84 item 13); (5) the text forms and the
  human-readable branch with its string pins (85), then `Rank`'s
  `f.pad` (86 item 14); (6) the rumors-relied contracts (87 item 18),
  the subadditivity proptests extended and read clean before the
  sentence is written; (7) the `suanpan` API items (86 item 17), the
  swap pin red-first; (8) packaging (87 item 20).
- `citecheck`'s rumors root (ruling 87, item 19) is `p5-scanners`'
  once the typed checker exists; this lane only reports any `rumors`
  public rustdoc that cites a `before` law name as a rumors-ledger
  finding (never edit `rumors` prose for it).
- Ruling 94's suanpan API change lands in this lane after `p8-performance`'s rank-22 (see the section below Hazards).
- This lane owns: `src/{clock,party}.rs` and `src/{clock,party}/forks.rs`,
  `src/iter.rs`, `src/version/{ticks,rank,ranked}.rs`, `src/span.rs` and
  `span/wire.rs`, `src/shape.rs`, `src/error.rs`, `src/serde_impls.rs`
  (or wherever the serde impls live), `src/codec/text.rs`'s trim
  predicate and the three text entries, `src/causally/**`'s `Floor`,
  `Ceiling`, and `Query` docs, `src/laws.rs` (the pair law),
  `tests/forks_max.rs`, `crates/suanpan/src/{limbs,accumulator}.rs`,
  and `crates/before/Cargo.toml`.

## suanpan drops dashu (ruling 94; no finding id)

Finch's words: "Does suanpan need dashu? I am wondering if we can simplify things here along the way."

Inside suanpan `UBig` is only a boundary type, never arithmetic: inputs arrive as a word view (`Limbs::new` reads `as_words`; `Magnitude::to_word` is the word-fit dispatch) and readouts leave as a `UBig` built from bytes (`sign_magnitude`, `sign_magnitude_shl`, `magnitude_from_digits`). Ruling 94: suanpan's public API takes limb slices and returns limb vectors (or a small magnitude newtype); the `Magnitude` trait stays generic and `before` implements it for `Base`; `before` converts at the seam at the linear cost it pays today (its `Signed::from_sign_magnitude` callers build the `UBig` from the limbs); `dashu-int` leaves suanpan's manifest; the `UBig` re-export and the `Magnitude for UBig` rows leave suanpan's claims roster. `before` keeps dashu for multiplication in the rank integrator, `Base`'s decimal conversion, and the codecs' bit operations.

An owner-directed suanpan public API change, named as such in its commit. Order: after ruling 90's limbs-only `Num` (`p8-performance`, rank-22) so `Num` never round-trips through `UBig`; before `p6-suanpan`. Acceptance: `grep -rn dashu crates/suanpan` returns nothing; suanpan's tests and `before`'s pass unchanged; every touch pin holds (ceilings, ruling 88).

## Hazards and stops

- A public signature not named in rulings 81 to 87 that the work seems
  to require is a stop.
- A wire snapshot (`tests/gossip_snapshot.rs` in rumors, the `insta`
  snapshots, the bookmark pins) or a `before` encoding pin that moves is
  a stop; the text forms are `Display`/`FromStr` and the human-readable
  serde branch only.
- `Rank`'s `Display` change moves its fuelscape include's class; the
  re-pin is measured at the parent and named. If the fuelscape
  regeneration exceeds this lane's one-run cap, report the local reading
  and leave the regeneration to the coordinator.
- The subadditivity sentence (rumors-dependence-2, version-core-5) is
  written only after the extended proptests read clean; a counterexample
  is a stop and a finding.
- `must_use` on `Party` and `Clock` may surface a `rumors` site under
  `-D warnings`; that is a stop with the site named, not a `rumors` edit.
- The `p2-widths.md` forks entries: do not land them from that brief;
  they are superseded here (see Ordering).

## Members

### api-audit-1 (medium, api): ruling 81

Party and Clock carry no #[must_use]; a discarded fork or seed leaks id space silently

Resolution: `#[must_use = "..."]` on `pub struct Party` (party.rs:66) and `pub struct Clock` (clock.rs:52), which covers every by-value producer including the `(Party, Version)` tuple from `into_parts` (rustc checks tuple elements); add a separate `#[must_use]` on `Party::without`, because `Option<T>` is not inspected for a must_use payload. Acceptance: `let mut p = Party::seed(); p.fork();` and `p.without(&q);` each produce `unused_must_use`; the crate's doctests, tests, and the rumors consumer compile warning-free.

Ruled (81, decision 2): `#[must_use = "..."]` on `pub struct Party` and `pub struct Clock`, the reason naming the loss (a discarded share leaks id space), plus method-level `#[must_use]` on `Version::join`, `join_all`, `meet`, `meet_all`, `Rank::checked_sub`, `saturating_sub`, and `Party::without`. Acceptance as quoted, and the rumors consumer compiles warning-free (check it in the same commit; a rumors site that trips is a stop, not a rumors edit).

### api-audit-13 (low, api): ruling 81

Ticks out-conversions and subtraction are narrower than its in-conversions and than Rank's

Resolution: `TryFrom<Ticks>` by value and `TryFrom<&Ticks>` for u128/usize/u32; `checked_sub`/`saturating_sub` mirroring `Rank`'s. Acceptance: `u64::try_from(Ticks::from(1u8))` compiles; `Ticks::checked_sub` exists with a doc example.

Ruled (81, decision 5): adopt: `TryFrom<Ticks>` by value, the `u128`/`usize`/`u32` duals of the existing `From` impls, and `checked_sub`/`saturating_sub` mirroring `Rank`, all additive. `From<suanpan::UBig> for Ticks` is declined (the string entry point keeps suanpan's type off before's stable surface).

### api-audit-6 (low, api): ruling 81

The fork iterators are defined as `Forks` but reachable only as `iter::Clock`/`iter::Party`; rustdoc shows a name the user cannot import

Resolution: owner's choice between (a) naming each struct as it is exported (define `ClockForks`/`PartyForks`, or define both in `iter.rs`), which fixes the rendered signature, or (b) keeping the names and rewriting the two "see [`Forks`]" sentences (clock.rs:178, party.rs:258) to the public path `[`iter::Clock`](crate::iter::Clock)` / `[`iter::Party`](crate::iter::Party)`, which fixes the prose only. Acceptance: (a) the rendered return type is a path a user can `use`; (b) at minimum the prose names the public path.

Ruled (81, decision 3). Finch's words: "I like the public names. Please have those be the only visible ones." The fork iterator types are renamed so `iter::Clock` and `iter::Party` are their defining names; `Forks` appears nowhere in rendered rustdoc. The public path is unchanged, so this is not a break; no rename is queued for later. The Resolution's prose-only fix is subsumed.

### clippy-pedantic-4 (low, api): ruling 81

The linear types `Party` and `Clock` and the pure `Version`/`Rank` combinators carry no `#[must_use]`

Resolution (suggestion): a type-level `#[must_use = "..."]` on `Party` and `Clock` naming the loss, and method-level `#[must_use]` on `Version::join`, `join_all`, `meet`, `meet_all` and `Rank::saturating_sub`. `Accumulator::merge_into_wider` (suanpan accumulator.rs:1169) returns a spare buffer whose drop is legitimate; leave it, or annotate with a reason string saying so. Acceptance: `just clippy` clean with the attributes in place, and a doctest or unit test that `p.fork();` warns is unnecessary (the attribute is the mechanism).

Ruled (81, decision 2): lands with api-audit-1's attributes; the same commit.

### envelopes-b-19 (low, simplification): ruling 81

The two-scale flatness assertion, the counter readers, the clock-history fixture, `tick_run`, and the `UBig`-to-`Ticks` conversion are hand-copied across modules

Resolution: hoist to file level one `Slack { num, den }` (or named `Ratio` constants: `FLAT = 5/4`, `LIMB_LEVEL = 3/2`, `WIDTH = 23/20`, `GROWTH = 5/2`), one `assert_flat_per_unit(name, small: (counter, unit), large, slack)` that also prints the MEASURED line (the fold modules pass `unit = bytes × levels`), one `Reading` struct, one `scanned`/`limbed`/`touched` trio, one `History` builder returning the snapshots each section needs, one `tick_run(ev, id) -> Run` with the shared floor, `fn redecoded(v: &Version) -> Version`, `fn ticks(n: &UBig) -> Ticks`, `Ticks::from(1u128 << 80)` for the constant, `party_of` at the decode sites, and `rank_kernel_envelope(name, shape, row)`/`masked_triple_envelope` row helpers so each `#[test]` is a one-line call under its doc. Acceptance: one definition each of `assert_model_flat`, `scanned`, `fixture`, `tick_run`; `grep -c 'fn scanned' crates/before/tests/meter.rs` reads 1; `grep -c 1208925819614629174706176 crates/before/tests/meter.rs` reads 0; `grep -c 'parse::<before::Ticks>' crates/before/tests/meter.rs` reads at most 1; readings and verdicts unchanged.

Ruled (81, decision 5): as meter-core-5; one test-local helper, not a public impl.

### fresh-eyes-9 (low, api): ruling 81

The fork iterator's documented name is not importable

Resolution: Either (owner-gated) re-export under the declared names, for example `iter::ClockForks` and `iter::PartyForks`, so the signature, the prose, and the import agree, or add one sentence to `Clock::forks` and `Party::forks` giving the importable spelling (`before::iter::Clock`, `before::iter::Party`). Acceptance: the return type a reader copies from the rendered signature is a path `use before::...` accepts, or the method docs state the path that is.

Ruled (81, decision 3): as api-audit-6; the rename makes every "see" sentence name the type's own path.

### fresh-eyes-14 (nit, api): ruling 81

Ticks converts out to u64 only by reference

Resolution: Owner-gated: add `impl TryFrom<Ticks> for u64` delegating to the reference form, and consider `u128`/`usize` duals for the `From` impls that exist. Acceptance: `u64::try_from(Ticks::from(1u64))` compiles.

Ruled (81, decision 5): lands with api-audit-13's additions.

### meter-core-5 (nit, simplification): ruling 81

Idiom inconsistencies: `debug_assert!` guards in three helpers whose siblings `assert!`, `Base::from(0u8)` beside `Base::ZERO`, a `saturating_sub` after an `assert!`, two `Ordering` paths, thirty qualified `suanpan::UBig`s, a rustfmt-folded comment, two undocumented recursive helpers, one caller named in `bitlen`'s doc, and a repeated `Ticks` string round-trip

Where: `crates/before/src/meter.rs:728-730`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `assert!` in the three helpers; `Base::ZERO`; one `Ordering` import; `use suanpan::UBig`

Ruled (81, decision 5): `From<suanpan::UBig> for Ticks` is declined; the envelope-suite sites get one test-local helper (`fn ticks(u: &UBig) -> Ticks` through the string entry point, defined once in `tests/meter.rs`'s support module after `p1-harness` lands).

### clock-17 (medium, correctness): ruling 82

`Forks::len()` panics on 32-bit targets for counts past `usize` (an `ExactSizeIterator` over a `u64` count)

Resolution: Owner's call among (a) dropping `ExactSizeIterator` from `Forks`/`party::Forks`/`Split` and keeping the accurate `size_hint` (`len()` disappears; iter.rs:16's doc example uses it); (b) taking the count as `usize`; (c) documenting under `# Panics` that `len()` panics past `usize` on narrow targets, or clamping the reserved count at construction and documenting the saturation beside the existing `u64::MAX` one. Whichever is chosen, add a red-first pin to `wasm32-pins` calling `.len()` and `size_hint()` on `forks(1u64 << 32)`. Acceptance: a committed wasm32 pin exercises the boundary under the chosen contract; the rustdoc of `Clock::forks`, `Forks`, `Party::forks`, and `party::Forks` states what happens past `usize`. Construction: on any 32-bit target (the pinned wasm32 guest): `let mut c = before::Clock::seed(); let it = c.forks(1u64 << 32); let _ = it.len();`. `usize::try_from(4294967297).ok()` is `None`, so `size_hint` is `(usize::MAX, None)` and the default `len` panics on `assert_eq!(None, Some(usize::MAX))`. On 64-bit the same call returns `4294967296`.

Ruled (82, decision 4; supersedes the pending note in `p2-widths.md`). Finch's words: "can we have it take a usize as an argument?" `Clock::forks` and `Party::forks` take the child count as `usize`; `ExactSizeIterator` is implemented unconditionally, `len()` and the count sharing one type on every target, so no trap exists to document or gate. The wasm32 pin's role becomes: `forks(usize::MAX)` on the guest yields `usize::MAX` shares' `len()` without a trap (the boundary, not a panic). Ruling 35's boundary reads with `usize::MAX` for `u64::MAX`: exactly `k` children for every `k`, pinned by `tests/forks_max.rs`. The `size_hint` nit (one conversion) is moot: no conversion remains. The commit names this as an owner-directed signature change on the stable surface.

Ledger note: forks takes usize

### party-13 (medium, correctness): ruling 82

`Forks`/`Split` implement `ExactSizeIterator`, whose default `len()` panics for `k >= usize::MAX` shares on 32-bit targets

Resolution: Owner's choice among: document a `# Panics` on `Forks`/`clock::Forks` for `len()` past `usize::MAX` shares on 32-bit targets; override `len()` to saturate at `usize::MAX` (documented as the one place the count is inexact); or stop implementing `ExactSizeIterator` (an API removal, least attractive). Whichever is chosen, add a wasm32 pin. Nit alongside: `size_hint` converts `remaining` twice; compute `usize::try_from(self.remaining)` once. Acceptance: a committed wasm32-pins test exercises `len()` on `forks(u64::from(u32::MAX) + 1)` under the chosen contract; tests/forks_max.rs loses its `64-bit test host` caveat or states why it remains. Construction: on any 32-bit target: `let mut p = Party::seed(); let it = p.forks(u64::from(u32::MAX) + 1); let _ = it.len();`. After the residual is drawn, `remaining` is `2^32 + 1`, `size_hint` is `(usize::MAX, None)`, and the default `len()`'s `assert_eq!(upper, Some(lower))` fails.

Ruled (82, decision 4): the `Party` twin of clock-17; same change, same commit.

Ledger note: forks takes usize

### api-audit-10 (low, documentation): ruling 82

forks(u64::MAX) yields one share fewer than asked; the public docs say "exactly `k`" and the test calls this "documented"

Resolution: one sentence in `Party::forks`, `Clock::forks`, and both `Forks` type docs ("`k == u64::MAX` saturates: `u64::MAX - 1` shares are yielded, the residual taking the last slot"), after which tests/forks_max.rs's "documented behavior" becomes true; or count in `u128` internally so `k` shares are always yielded. Acceptance: the public `forks` docs state the corner, or `forks(u64::MAX).len()` equals `u64::MAX` on 64-bit.

Ruled (82, restating 35): the count is `usize`; `forks(usize::MAX).len()` equals `usize::MAX` on every target; no saturation sentence is written. The `p2-widths.md` entry for this id is superseded by this one.

Ledger note: forks takes usize

### api-audit-11 (low, api): ruling 83

Span (and shape::Plateau/Rise/Region) lack Hash while every other value type in the crate has it

Resolution: derive `Hash` on `Span`, `Plateau`, `Rise`, and `Region`; add a law that `a == b` implies equal hashes for `Span`. Acceptance: `HashSet<Span<'static>>` compiles and the law holds under proptest.

Ruled (83, decision 6): derive `Hash` on `Span`, `shape::Plateau`, `Rise`, and `Region`; additive.

### api-audit-2 (low, correctness): ruling 83

Decode::Io interpolates the io::Error into Display and returns None from Error::source()

Resolution: `#[error("read error")] Io(#[source] io::Error)`, or `#[from]`, which also yields `From<io::Error>` and lets each `.map_err(Decode::Io)?` become `?` (party.rs:625, clock.rs:789, version.rs:1112, rank.rs:463, ranked.rs:273). Acceptance: a unit test constructs `Decode::Io(io::Error::new(ErrorKind::Other, "x"))` and asserts `std::error::Error::source(&e).is_some()`; `Display` no longer embeds the inner message. Construction: in `crates/before/src/error/tests.rs` (or the existing error tests), `assert!(std::error::Error::source(&Decode::Io(io::Error::other("x"))).is_some())` fails at this commit and passes after the attribute lands.

Ruled (83, decision 7): as fresh-eyes-5.

### crate-root-15 (low, simplification): ruling 83

`Decode::Io` carries its `io::Error` only in the message, not as `source()`

Resolution: `Io(#[source] io::Error)`; keep the Display string so the snapshot at testing/snapshots.rs:276 is unchanged. Acceptance: a test asserts `std::error::Error::source(&Decode::Io(io::Error::from(io::ErrorKind::UnexpectedEof))).is_some()`.

Ruled (83, decision 7): as fresh-eyes-5.

### fresh-eyes-5 (low, api): ruling 83

Decode::Io drops the io::Error from the error source chain

Resolution: Annotate the field `#[source]` (or `#[from]`, which also gives callers `?` from `io::Error`); either keep `{0}` in the message and accept the duplicated text, or change the message to `"read error"` and let the chain carry the cause. Acceptance: a test that decodes from a failing reader asserts `err.source().is_some()`. Construction: A `Read` impl whose `read` returns `Err(io::Error::other("boom"))`; `Version::decode(reader)` yields `Decode::Io`, and `std::error::Error::source(&err)` is `None` today.

Ruled (83, decision 7): annotate `Decode::Io`'s field `#[source]`; `source()` moving from `None` to `Some` is observable but not breaking.

### fresh-eyes-6 (low, api): ruling 83

The causally atoms expose contains but not coverage

Resolution: Either add `coverage` to `Floor` and `Ceiling` (owner-gated), or add one sentence to `after`, `before`, `Floor`, and `Ceiling` saying a bare atom converts into a neutral `Query` with `Query::from` (or `.into()`) for `coverage`. Acceptance: a doctest on `after` calls `coverage` on the atom, directly or through the documented conversion.

Ruled (83, decision 9): the one-sentence pointer to `Query::from` on `Floor` and `Ceiling` now, and a delegating `coverage` method on each as an additive change, in the same commit.

### fresh-eyes-15 (nit, api): ruling 83

Clock::join hands back a Clock in Err, which does not compose with `?`

Resolution: Add to `join`/`join_all`'s `# Errors` one sentence with the propagation idiom (`.map_err(|_| Overlap)?` when the handed-back clock is not wanted), or (owner-gated) offer an `Overlap`-returning variant beside the hand-back form. Acceptance: the `join` docs show how to propagate the error as an `Error`.

Ruled (83, decision 8): model. `Clock::join` keeps `Err(Clock)`; its rustdoc states, in one present-tense sentence, that the share comes back so it cannot be lost. Finch's words: "don't document the idiom, it's an anti-pattern": no `map_err(|_| Overlap)` example appears anywhere in the crate, and any existing one is removed. Nothing else from the Resolution lands.

### span-causally-1 (nit, api): ruling 83

`Span` derives `Eq` but not `Hash`, unlike `Version` and every verdict type beside it

Resolution: if the owner agrees, add `Hash` to the derive, add the census row, and extend a byte-equality law (the `version_eq_iff_bytes_eq` family) to spans so equal spans hash equal. Acceptance: `HashSet<Span<'static>>` compiles; the law is green; the census diff shows one added row.

Ruled (83, decision 6): lands with api-audit-11.

### crate-root-34 (medium, correctness): ruling 84

serde impls serialize as `bytes` but deserialize by requesting a `seq`

Resolution: Deserialize through `serde_bytes::ByteBuf::deserialize(d)?` (add `serde_bytes` as an optional dependency enabled by the `serde` feature) or a local ~20-line `Visitor` implementing `visit_bytes`, `visit_byte_buf`, and `visit_seq`, driven by `deserialize_bytes`. While touching all twelve impls, fold the six identical pairs into one macro taking a per-type doc attribute (`version.rs`'s `causal_cmp_impls!` is precedent) so the door has one body; the `Rank`/`Ranked`/`Span` impl docs carrying contract survive as doc arguments. Pairs naturally with crate-root-35 (the owned Vec the visitor yields can become the storage). Acceptance: a committed test drives a strict-typed deserializer, `serde_test::assert_tokens(&value, &[Token::Bytes(&value.encode())])` for each of the six types (dev-dependency `serde_test`), or a minimal local `Deserializer` whose `deserialize_seq` forwards a bytes payload to `visit_bytes`; the existing json/postcard/ciborium legs and the canonical-bytes pin stay green. Construction: Add `serde_test` as a dev-dependency and write `serde_test::assert_tokens(&Party::seed(), &[serde_test::Token::Bytes(Party::seed().as_bytes())])`: the serialize leg passes (`serialize_bytes` emits `Token::Bytes`); the deserialize leg fails, because `serde_test`'s `deserialize_seq` forwards a non-seq token to `deserialize_any`, which calls `visit_bytes`, which `VecVisitor` does not implement.

Ruled (84, decision 13): deserialize through `serde_bytes` (optional dependency enabled by the `serde` feature); `serde_test` as a dev-dependency pinning each of the six types with `assert_tokens` against `Token::Bytes(&value.encode())`; the six identical impl pairs folded into one macro taking a per-type doc attribute. The Resolution's local-visitor alternative is struck. The human-readable branch (ruling 85, below) is built in the same macro.

### api-audit-14 (low, api): ruling 84

Rank::decode labels its representation bound Decode::NotCanonical, and neither error enum is #[non_exhaustive]

Resolution: either a dedicated variant now, or `#[non_exhaustive]` on `Decode` and `Parse` before the first release so one can land later; at minimum extend `NotCanonical`'s variant doc to name the representation-bound case. Acceptance: a decode that fails the representation bound is distinguishable by variant, or the variant doc names both cases.

Ruled (84, decision 10): `#[non_exhaustive]` on `Decode` only; `Parse` stays closed by the grammar. `NotCanonical`'s variant doc names the representation-bound case (a mantissa width the backend cannot hold on this target; ruling 41's threshold). rank-10's rewording of the same clause (`p4-ghosts`) composes with this; coordinate so the variant doc is written once.

### codec-base-text-tree-19 (low, simplification): ruling 84

`parse_clock_str` pre-scans for the top-level comma with an `i64` depth counter, work the id parser on a cursor already does

Resolution: Minimal shape: keep the outer-paren strip; then `let mut cur = Cur::new(inner); let mut bits = BitsBuf::new(); parse_id_tree(&mut cur, &mut bits)?; if cur.bump() != Some(b',') { return Err(Parse::Syntax); } Ok((bits, cur.rest()))` with a `Cur::rest(&self) -> &'a str` accessor (the position after an ASCII `,` is a char boundary). Deeper shape: add a cursor-taking entry to `skyline::text` (its body already runs on `Cur` and its trailing check is one `peek`) and make the stamp one cursor pass returning both bit streams. Either way delete the depth loop, its comment, and `clock_text_split_survives_two_gib_of_parens`; keep `clock_text_deep_nesting_never_panics`, which exercises the whole `Clock::from_str`. Add a point pin for the precedence corner so the change is visible. Acceptance: `grep -n 'depth: i64' src/codec/text.rs` is empty; the 2 GiB ignored test is gone; the clock text pins and the never-panics proptest stay green; the clock `FromStr` fuel band in fuzzfit is unchanged or lower, measured at the parent.

Ruled (84, decision 11): one whole-pass precedence rule documented on `Parse`; the cursor's ASCII whitespace predicate at all three text entries; pinned beside `id_text_parser_error_precedence_pins`.

### codec-base-text-tree-20 (low, correctness): ruling 84

The clock text door trims Unicode whitespace; every other door and the cursor's own contract are ASCII-only

Resolution: Trim with the cursor's own predicate, `s.trim_matches(|c: char| c.is_ascii_whitespace())`, or let the cursor-based split of finding codec-base-text-tree-19 do the skipping through `Cur` so the stamp door has no whitespace rule of its own. Add a point pin beside `id_text_parser_error_precedence_pins` asserting the three `FromStr` doors agree on a leading U+000B and U+00A0 (all `Err(Parse::Syntax)`). Acceptance: `"\u{0B}(1, 0)".parse::<Clock>()` and `"\u{A0}(1, 0)".parse::<Clock>()` return `Err(Parse::Syntax)`, matching `"\u{0B}1".parse::<Party>()`; the pin and `clock_text_deep_nesting_never_panics` stay green.

Ruled (84, decision 11): as codec-base-text-tree-19; ASCII trim only, Unicode whitespace is not trimmed at any entry.

### codec-base-text-tree-18 (nit, api): ruling 84

The id and version text doors disagree on whether trailing junk outranks `NotCanonical`

Resolution: Owner ruling on which rule wins, then align: either defer `NotCanonical` in `parse_id_tree` (a `canonical` flag reported after the trailing check, as the skyline parser does, mirrored in `ref_parse_id_node`), or document the per-node rule on `Parse` and in the skyline parser. Add one cross-door pin with the same trailing-junk-plus-collapsible text through both doors. Acceptance: `"(1, 1) x".parse::<Party>()` and `"(0, 5, 5) x".parse::<Version>()` return the same variant, and `Parse`'s rustdoc states the precedence.

Ruled (84, decision 11): lands with the precedence rule.

### fresh-eyes-3 (low, documentation): ruling 85

The serde representation is undocumented (JSON emits a numeric byte array)

Resolution: State in the feature docs that every type serializes as the bytes of its canonical encoding via `serialize_bytes`, so binary formats carry the wire bytes and human-readable formats carry a sequence of integers; then `just readme`. Whether to branch on `Serializer::is_human_readable()` and emit the paper notation is an owner decision (see Open questions). Acceptance: the `serde` bullet names the representation.

Ruled (85, decision 12; completes 84's item 12). Build the human-readable branch: under `is_human_readable()` every type serializes as its `Display` string and deserializes through `FromStr`; the byte form stays accepted by every deserializer; binary formats are unchanged; both branches pinned with `serde_test` (`Token::Str` and `Token::Bytes`). Text forms: `Rank` in binary with a binary point (below); `Ranked` renders and parses as its version's text, `FromStr` deriving the rank; `Span` renders as `lo <= hi` and parses through `Span::new`, so an inverted or incomparable pair reports `Crossed`. `FromStr` is added for `Rank`, `Ranked`, and `Span`; `Display` for `Ranked` and `Span`. The new parsers follow the whole-pass precedence rule and ASCII whitespace predicate of ruling 84. Not a wire change: no snapshot moves; if one would, stop.

Ledger note: human-readable branch built; forms per ruling 85

### fresh-eyes-16 (nit, api): ruling 85

Rank renders to text but does not parse

Resolution: Owner-gated: add `FromStr for Rank` over the Display grammar with the same strict rejection of non-normal spellings, or state on the `Display` impl that the text form is one-way. Acceptance: either `"19/2^4".parse::<Rank>()` round-trips, or the `Display` docs say the form does not parse.

Ruled (85, decision 12). Finch's words on the decimal `n/2^k` form: "we cannot allow this, because it produces an exponential blowup if parsed from this representation and serialized to binary. We should change the representation so that it's proportionate in size to the binary one." `Rank` renders and parses in binary with a binary point: the integer part with no leading zeros (a lone `0` for zero); a fraction present only when the exponent is nonzero; never a trailing zero digit (the numerator is odd, so the form is canonical by construction); `5` is `101`, one half is `0.1`, three halves is `1.1`. The decimal rendering goes; `Debug` stays equal to `Display`; `FromStr for Rank` accepts exactly this grammar and rejects a non-canonical spelling as a `Parse` error. Conversion is linear both ways, so `Display`'s `# Complexity` prose and its fuelscape include (`rank_display`) are re-pinned to the linear class, measured at the parent, the movement named in the commit (the fuelscape regeneration is the coordinator's run if it exceeds this lane's caps; report the reading you can take and stop there).

Ledger note: human-readable branch built; forms per ruling 85

### suanpan-tests-16 (medium, verification): ruling 86

merge_into_wider's swap, the min in its cost row, has no touch pin in the direction that exercises it

Resolution: add the mirrored leg inside the same `held_bits` loop: `receiver = narrow()` (2 digits), operand holding `(1 << held_bits) - 1` (64 then 128 digits); `touch_meter::reset(); let spare = receiver.merge_into_wider(operand); assert_eq!(touches, 4)`; assert the sum landed in `receiver`; state in the doc that without the swap this leg reads `2 · held_digits` (one read per operand digit plus one deposit each, no carries since every digit stays under 2^33). Acceptance: a metered leg in which `other.digit_count() > self.digit_count()` pins 4 touches at two receiver widths; a build with the swap removed fails that leg by name. Construction: `let mut receiver = narrow(); let mut operand = Accumulator::new(); operand.add_wide(&((UBig::from(1u8) << 2_048usize) - 1u8)); touch_meter::reset(); let spare = receiver.merge_into_wider(operand); assert_eq!(touch_meter::touches(), 4);` With lines 1170-1172 deleted, `fold_accum` iterates 64 digits: 64 reads + 64 deposits = 128.

Ruled (86, decision 17): the mirrored `merge_into_wider` leg lands red-first (four touches at two receiver widths; the doc states the no-swap reading). The `Drained` newtype (suanpan-tests-8) waits on this pin's reading.

### api-audit-12 (low, simplification): ruling 86

suanpan::Limbs withholds the exact size Chunks already knows, so before's Ticks::limbs re-derives it by hand

Resolution: in suanpan, add `size_hint` delegating to `self.chunks.size_hint()`, `impl ExactSizeIterator for Limbs<'_> {}`, `impl FusedIterator for Limbs<'_> {}`, and update the `FAMILY_SURFACE` row "Limbs iteration (Iterator / DoubleEndedIterator)"; then let before's `Limbs` delegate and drop `remaining`. Acceptance: `suanpan::Limbs::new(&x).len()` equals the yielded count under proptest; before's `Limbs` has no `remaining` field.

Ruled (86, decision 17): as version-core-24.

### api-audit-9 (low, documentation): ruling 86

Public docs point at arguments that live nowhere public (OwnSpan monotonicity; Query's missing Eq)

Resolution: state once, in `OwnVersion`'s public docs, that projection is a homomorphism of join and meet and therefore order-preserving (`a <= b` implies `a/p <= b/p`), and point `to_span` there; reword version.rs:1675-1676 so `min_ticks` is the stated subject ("`min_ticks` is not monotone under projection"); add the reason `Query` has no `PartialEq` to `Query`'s type docs and fix the query.rs pointer. Acceptance: each pointer resolves to a paragraph that states the argument.

Ruled (86, decision 15): `Query` keeps no `PartialEq`; one present-tense sentence on the type states that equality is not offered and why. The deleted rationale sentence stays deleted (ruling 66); this is new prose, not a restoration.

### fresh-eyes-8 (low, documentation): ruling 86

Query has no equality; the public docs do not say so and the private pointer to the rationale dangles

Resolution: Add one sentence to the `Query` type docs stating that queries carry no equality (two structurally different queries can denote one predicate, so `==` would not mean what a reader expects) and that `Debug` renders the normal form, and either state the rationale at query.rs:212 inline or point the comment at the sentence that now holds it. Acceptance: `grep -n 'see the module docs' crates/before/src/causally/query.rs` either returns nothing or the module doc it names contains the word `Eq`.

Ruled (86, decision 15): as api-audit-9.

### span-causally-38 (low, documentation): ruling 86

"there is deliberately no `Eq`; see the module docs" points at module docs that no longer discuss `Eq`

Resolution: record the decision once, either restored to causally.rs's module doc (where both pointers say it is) or on `Query`'s type doc with the pointers repointed; state the mechanism (non-unique normal forms under conjunction order and inert degenerate holes). Acceptance: `grep -n 'Eq' crates/before/src/causally.rs` (or query.rs's type doc) finds the decision the two comments cite.

Ruled (86, decision 15; the pointer half is ruling 66 in `p4-ghosts`): the dangling "see the module docs" pointer goes, and the one-sentence statement on `Query` is written here. Coordinate with `p4-ghosts` so the site is edited once.

### suanpan-20 (low, simplification): ruling 86

`add_u64_shl` / `sub_u64_shl` have never had a caller outside suanpan's own tests

Resolution: owner decision: retire both (their claims rows, the table row at lib.rs:213, and the second loop of `alternating_shifted_writes_cost_the_operand_not_the_gap`), or route before's shifted word-scale folds through them so the entry has the caller its introducing commit described. Acceptance: either no `add_u64_shl`/`sub_u64_shl` remain and `claims_are_total_over_the_public_surface` passes, or a production caller in before invokes them.

Ruled (86, decision 17): `add_u64_shl` and `sub_u64_shl` are deleted now, with their tests, as suanpan public API with no caller; an owner-directed removal named in its commit.

### version-core-24 (low, simplification): ruling 86

`Ticks::limbs` re-derives an exact size and reaches through two newtypes because `suanpan::Limbs` exposes neither `ExactSizeIterator` nor a `Base` accessor

Resolution: in suanpan, `impl ExactSizeIterator for Limbs<'_>` (delegating `size_hint` to `self.chunks.size_hint()`) and `impl FusedIterator for Limbs<'_>`; in before, `Limbs { limbs }` forwards `size_hint` and the `remaining` field and `expect` go; add a crate-private `Base::limbs(&self) -> suanpan::Limbs<'_>` so `Ticks::limbs` reads `self.0.limbs()`. Acceptance: `limbs_respell_the_count` stays green; `grep -n remaining crates/before/src/version/ticks.rs` is empty; no `.0 .0` in ticks.rs.

Ruled (86, decision 17): `ExactSizeIterator` and a `Base` accessor on `Limbs`, additive; `Ticks::limbs` stops re-deriving its exact size.

### rank-21 (nit, api): ruling 86

Display honors formatter flags only for integral ranks

Resolution: Render all three forms to a `String` and route through one `f.pad_integral(true, "", &text)` (or `f.pad`), or state in the `Display` doc that formatter flags are ignored. Acceptance: a unit test formatting `{:>6}` over `ZERO`, an integral rank, and a fractional rank asserts the same padding behavior for all three, or the doc states the contract.

Ruled (86, decision 14): `Display` pads uniformly through `f.pad` on the whole rendered string; the doc states that sign and zero flags do not apply (a rank is nonnegative and the form has no fixed width). Lands with the ruling 85 rewrite of `Display`.

### rumors-dependence-1 (medium, dependence): ruling 87

Tick's event contract (strict advance, region-locality) is pinned by laws but stated in no public rustdoc of `tick`

Resolution: state the event contract on `Version::tick` and mirror it on `Party::tick` and `Clock::tick`: the result strictly dominates the input; projected onto any region disjoint from `party`, the result equals the input; hence ticks by disjoint parties from one base always differ. Optionally add the consequence as a `VERSION_PARTY_PAIR` law (for example `ticks_by_disjoint_parties_differ`: `!p.is_disjoint(q) || { let mut a = v.clone(); a.tick(p); let mut b = v.clone(); b.tick(q); a != b && (&b / p) == (v / p) }`); the one-world population inhabits the antecedent (`laws.rs:2782-2783`), and the roster and fuzz driver pick a new group member up by construction (`laws.rs:94-97`). Acceptance: the rustdoc of the three `tick` entries states both clauses; a reader of before's public docs can derive stamp uniqueness without the paper or the laws module.

Ruled (87, decision 18): state tick's event contract on `Version::tick`, `Party::tick`, and `Clock::tick` (the result strictly dominates the input; projected onto any region disjoint from the ticking party it equals the input; hence ticks by disjoint parties from one base always differ), and add the consequence as a committed pair law in `laws.rs` as the Resolution sketches.

### version-core-5 (medium, claims): ruling 87

The join/meet subadditivity lemma is derived only in test prose, though the folds' auxiliary-space bounds and a downstream budget rest on it

Resolution: state the lemma once in production prose, positively, with the derivation's one-line shape (output boundaries lie in the union of input boundaries; pointwise max/min is 1-Lipschitz in each operand; zigzag-gamma length depends only on magnitude; the shared root and unmatched first-leaf code give the `−2`): in the `# Complexity` sections of `Version::join` and `Version::meet` (the operator matrices' `$opdoc` can cite them), and cited from `balanced_fold`'s doc or the `crate::fold` module doc as the reason a merged group cannot outgrow its inputs. Point the four `*_encoding_is_subadditive*` tests and the tier2 pins at the stated lemma by name. Acceptance: `grep -rn -i subadditiv crates/before/src/version.rs` matches the public rustdoc of `join` and `meet`; the aux-space lines read as consequences of a stated lemma; the tier2 pins are unchanged; `just readme` regenerates cleanly.

Ruled (87, decision 18): as rumors-dependence-2; the proptest extension is this entry's first step.

### crate-root-2 (low, documentation): ruling 87

Cargo description names a "transient fixed-width working form" the crate no longer has

Resolution: Restate against today's design, for example "Interval Tree Clocks (Almeida, Baquero & Fonte, 2008): canonical packed bit-stream storage, fused streaming kernels, linear-typed API"; fix results/benchmarks/README.md:4 in the same pass. Acceptance: `grep -rni 'working form' crates/before --include='*.toml' --include='*.md'` is empty.

Ruled (87, decision 20): the crates.io description is corrected toward the code (no "transient fixed-width working form").

### deps-8 (low, simplification): ruling 87

before's `serde` feature selects `derive`, which nothing in the crate uses

Resolution: `serde = { workspace = true, optional = true, features = ["alloc"] }`. Acceptance: `just gate` clean; `cargo tree -p before --features serde -e normal` shows no serde_derive.

Ruled (87, decision 20): `derive` dropped from the `serde` feature.

### rumors-dependence-2 (low, dependence): ruling 87

Join/meet encoding subadditivity is derived and pinned inside before's test tree and priced against by rumors, but stated in no public contract

Resolution: add one sentence to `Version::join` and `Version::meet` (or one paragraph to the Space Efficiency section, linked from both): the canonical encoding of a join or meet is never longer than the sum of its operands' encodings. Optionally promote `join_encoding_is_subadditive` and `meet_encoding_is_subadditive` to `VERSION_PAIR` laws so the fuzz law target also drives them over decoded values. Acceptance: the statement appears in public rustdoc; rumors' `message.rs` and `window.rs` comments can cite the documented contract rather than "pinned lemmas".

Ruled (87, decision 18): join and meet encoding subadditivity is stated in their public `# Complexity` with the derivation at `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS` and linked from `fold.rs`, but only after the subadditivity proptests are extended to deeper generators and the meter families and read clean; a counterexample is a stop (the sentence is not written and the finding goes to Finch).

### deps-16 (nit, simplification): ruling 87

no package include/exclude: measurement artifacts, the paper transcription, and plotting scripts would ship in the crate tarball

Where: `crates/before/Cargo.toml:1-7`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Nothing today; pre-release, `cargo package` would ship results, the paper transcription, and plotting scripts. An `include` (or `exclude`) list before the first release.

Ruled (87, decision 20): an explicit `include` list (`src`, `docs`, the README, `build.rs` while it exists); `cargo package --list` in the commit message shows neither `reference/` nor `scripts/` (`results/benchmarks` is deleted by `p3-vocabulary`, ruling 57; rebase).

### rumors-dependence-5 (nit, dependence): ruling 87

rumors cites `Span::dominance`'s coincident fast path as a cost contract; before documents it only in code comments and a private test, and no meter on either side counts it

Resolution: either state under `Span::at`'s `# Complexity` that the coincident span shares one stored buffer, so `place`, `dominance`, `precedence`, and `contains` against it cost a single pairwise comparison (before-side, owner-gated), or have the rumors review soften `untyped.rs:548-554` to describe the routing without presenting it as before's contract. Acceptance: rumors' comment cites a public statement or makes none.

Ruled (87, decision 18): the coincident span's single-comparison cost stated at `Span::at` and the verdict methods.

## Roster members approved (ruling 104)

Nits approved as this lane's roster by ruling 104, placed here because they touch the API surface or its docs. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### api-audit-5 (nit, documentation): roster: approved (ruling 104)

iter module doc has an unclosed code span; the Clock::forks link text carries a literal backtick

Where: `crates/before/src/iter.rs:9-10`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): add the missing backtick

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or touches a public signature the rulings did not name.

### codec-bits-11 (nit, api): roster: approved (ruling 104)

BitsBuf::get panics where BitsView::get returns Option

Resolution: Rename `BitsBuf::get` to `bit` (callers: literal.rs:44, buf.rs internals, test files). Acceptance: both storage forms spell the asserting read `bit` and the bounded read `get`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or touches a public signature the rulings did not name.

### fresh-eyes-11 (nit, documentation): roster: approved (ruling 104)

Typos and wrong link targets in public rustdoc

Where: `crates/before/src/version.rs:603-604`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): version.rs:603 link to `Version::span`; forms.rs:232 `t` for `e`, forms.rs:234 `after(s) & until(t)`; clock.rs:341 "iteratively" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or touches a public signature the rulings did not name.

### fuelscape-pipeline-14 (nit, api): roster: approved (ruling 104)

Preconditions unstated where the arithmetic relies on them: bit_window(0), sample_bytes past the table, ClockSlice at one byte

Resolution: `assert!(bytes >= 1, "a packed encoding has at least one byte")` at the head of `bit_window`; have `sample_bytes` return the draw directly with a `# Panics` section stating `1 <= bytes <= span` (or check the span and make that the `None`), removing the seven `.expect` sites in plan.rs; at plan.rs:316 either assert `size >= 2` naming the `ClockSlice` minimum or route the cap through `Inputs::min_bytes`. Also use the `RangeInclusive` import already at count.rs:41 in the return type. Acceptance: no `.expect` on `sample_bytes` remains in plan.rs; `bit_window(0, _)` panics with the named message in both profiles.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or touches a public signature the rulings did not name.

## Held members

Ruling 86 holds these three until the pins it lands have been read; they are listed so the roster is complete.

### suanpan-10 (low, api): held (ruling 86)

A zero wide operand spills the register and can flip a domination certificate

Resolution: owner decision: spill lazily (move `self.spill()` into `apply_limbs` ahead of the first `add_at`, and skip it in `fold_accum`'s digit path when `other.is_literally_zero()`) and re-derive the affected pins as a named touch-count change; or state at the wide entry points that a zero wide operand retires the register. Acceptance: a witness that `add_wide(&UBig::ZERO)` on a register value leaves `quick.is_some()` and records 0 touches, or the doc states the behavior. Construction (passes today): `let mut a = Accumulator::new(); a.add_u64(1 << 20); a.shl(30); a.shl(30); assert_eq!(a.sign_dominates_at(1), (Ordering::Greater, true)); a.add_wide(&UBig::ZERO); assert_eq!(a.sign_dominates_at(1), (Ordering::Greater, false));`.

Held: ruling 86 defers this entry until the `merge_into_wider` swap pin (suanpan-tests-16) and decision 25's zero-operand row have landed and been read. Do not land it; report the pin's reading beside it in your final report so Finch can rule.

Ledger note: held until the ruling 86 pins read

### suanpan-tests-4 (low, verification): held (ruling 86)

zero-valued wide operands are never drawn or witnessed, and they retire the register where the magnitude entries do not

Resolution: add a witness: register-held 5; `add_wide(&UBig::ZERO)`, `sub_wide_shl(&UBig::ZERO, 96)`, `add_limbs_shl(core::iter::empty(), 0)`; `assert_value` against 5 and pin the tier the owner intends (`acc.quick.is_some()` if the wide entries should short-circuit zero like `add_magnitude_shl`, the opposite if the spill is the contract). If short-circuiting is chosen, the wide entries gain an `if delta.is_zero() { return; }` before the spill (production; owner-gated). Acceptance: one committed test drives every wide entry with a zero operand on a register-held value and asserts both the value and the tier afterward. Construction: `let mut acc = Accumulator::new(); acc.add_small(5); acc.add_wide(&UBig::ZERO); assert!(acc.quick.is_some());` fails today, because `add_wide` spills unconditionally and the zero's limb stream is empty.

Held: ruling 86 defers this entry until the `merge_into_wider` swap pin (suanpan-tests-16) and decision 25's zero-operand row have landed and been read. Do not land it; report the pin's reading beside it in your final report so Finch can rule.

Ledger note: held until the ruling 86 pins read

### suanpan-tests-8 (nit, api): held (ruling 86)

merge_into_wider hands back an ordinary Accumulator whose value is unspecified; three test sites carry the reset obligation by convention

Resolution: owner's call: return a newtype that can only become an `Accumulator` by resetting (before's `retire` would need a second entry or a `From`, since it also takes live accumulators); the roster gains one trivially excluded row. Acceptance: a forgotten reset fails to compile; the drained-buffer prose in `merge_into_wider`'s rustdoc is gone.

Held: ruling 86 defers this entry until the `merge_into_wider` swap pin (suanpan-tests-16) and decision 25's zero-operand row have landed and been read. Do not land it; report the pin's reading beside it in your final report so Finch can rule.

Ledger note: held until the ruling 86 pins read

