<!-- CAVEAT LECTOR: review packet for lane p1-swarm, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p1-swarm

## Goal

Ruling T27 deletes the swarm example: `examples/swarm.rs`,
`examples/swarm/tests.rs`, the `[[example]]` entry in `Cargo.toml`, and
every recipe, workflow step, and prose reference to it. Every
`swarm-example-*` finding is disposed by the deletion rather than by its
own resolution, and the measurement the example offered is not replaced.
This is the one lane Finch reviews for what it removes: the invariant is
that the tree contains no reference to code that no longer exists and
that the gate is green without the example.

## Rulings landed

- T27: `examples/swarm.rs` and `examples/swarm/tests.rs` are deleted, with
  the `[[example]]` entry in `Cargo.toml` and every recipe, workflow step,
  and prose reference to the example; the crate carries no showcase
  example of that shape; the measurement it offered is not replaced by
  this ruling; owner decision 79 is moot.

## Stack position

- Base: `0926fe32`
- Parent: `main`
- Children: none

## Acceptance table

Every row is one the coordinator ran itself against the worktree at the
named commit; an entry whose acceptance does not hold is not in this
table. The thirty-one `swarm-example-*` ids share one commit and one
acceptance (the deletion and the reference sweep); the table carries them
as one row unless a report separates them.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| `swarm-example-1` through `-31` | `b8b97a38` | `git grep -n -i swarm -- ':!examples/swarm*' ':!.agent-notes' ':!crates/before-viz/www'` | eight lines, all in `tests/window_census.rs` (the fleet-shaped local the brief keeps); no other hit |
| (same) | `b8b97a38` | `git ls-files examples/` | `examples/envelope_sim.rs`, `examples/window_tradeoff.rs` |
| (same) | `e628ab2f` (identical code tree; the amend changed one annotation row) | `just gate` (lane log `p1-swarm/gate.log`) | `gate: clean in 356s`, `gate exit=0`, all eight legs ok |

## Fresh-eyes rounds

One round (surface correctness with operational validity folded in, since
the change is a deletion). The reviewer independently re-swept every
reference to the example, verified by its own grep that no workspace
target declares or imports `clap`, `arc-swap`, or `ratatui` (the detached
`crates/before-fuelscape` workspace pins its own `clap`), and checked the
lockfile as a strict subset of the base (no version bumped).

- Repaired: the commit message's and one annotation row's list of the
  duplicate crate versions the example's tree held omitted `hashbrown`
  (0.16.1, held by ratatui's layout solver). Both statements now carry
  the exact list from a name/version pair diff of the two lockfiles.
  Landed by amending the lane's single commit (`e628ab2f` to
  `b8b97a38`); the code tree is byte-identical.
- Reviewer notes not acted on: `clap` survives in `Cargo.lock` as a
  transitive dependency of `criterion`; informational, nothing declares
  it. Round 2 was not run: the round's findings were one prose nit and
  one scope observation (stop 1 below), which is past the "you might
  consider" threshold.

## Stops

1. **Dependency removal beyond the brief's letter.** T27 names the two
   files, the `[[example]]` entry, and every recipe, workflow step, and
   prose reference; it does not name dependencies. The lane also removed
   `clap`, `arc-swap`, and `ratatui` from `[dev-dependencies]` and
   `[workspace.dependencies]` and pruned `Cargo.lock` (338 to 219
   packages, eleven duplicate versions collapsed, nothing bumped). Both
   the lane agent and the fresh-eyes reviewer verified by grep that the
   example was the only user of the three. Recommendation: accept; three
   declared dev-dependencies with no consumer would be the worse
   artifact, and the gate is clean either way. If you decline, the six
   `Cargo.toml` lines and the lockfile revert in one commit and the
   entries land unchanged.
   **Ruled: accepted (T135).** The lane is offered for merge.

## Reading order

### tests and prose

- swarm-example-* (T27) at `Cargo.lock:11` ([hunk](#hunk-2))
- swarm-example-* (T27) at `Cargo.lock:38` ([hunk](#hunk-3))
- swarm-example-* (T27) at `Cargo.lock:57` ([hunk](#hunk-4))
- swarm-example-* (T27) at `Cargo.lock:98` ([hunk](#hunk-5))
- swarm-example-* (T27) at `Cargo.lock:114` ([hunk](#hunk-6))
- swarm-example-* (T27) at `Cargo.lock:147` ([hunk](#hunk-7))
- swarm-example-* (T27) at `Cargo.lock:172` ([hunk](#hunk-8))
- swarm-example-* (T27) at `Cargo.lock:187` ([hunk](#hunk-9))
- swarm-example-* (T27) at `Cargo.lock:270` ([hunk](#hunk-10))
- swarm-example-* (T27) at `Cargo.lock:278` ([hunk](#hunk-11))
- swarm-example-* (T27) at `Cargo.lock:294` ([hunk](#hunk-12))
- swarm-example-* (T27) at `Cargo.lock:325` ([hunk](#hunk-13))
- swarm-example-* (T27) at `Cargo.lock:370` ([hunk](#hunk-14))
- swarm-example-* (T27) at `Cargo.lock:395` ([hunk](#hunk-15))
- swarm-example-* (T27) at `Cargo.lock:410` ([hunk](#hunk-16))
- swarm-example-* (T27) at `Cargo.lock:435` ([hunk](#hunk-17))
- swarm-example-* (T27) at `Cargo.lock:442` ([hunk](#hunk-18))
- swarm-example-* (T27) at `Cargo.lock:494` ([hunk](#hunk-19))
- swarm-example-* (T27) at `Cargo.lock:518` ([hunk](#hunk-20))
- swarm-example-* (T27) at `Cargo.lock:577` ([hunk](#hunk-21))
- swarm-example-* (T27) at `Cargo.lock:609` ([hunk](#hunk-22))
- swarm-example-* (T27) at `Cargo.lock:662` ([hunk](#hunk-23))
- swarm-example-* (T27) at `Cargo.lock:670` ([hunk](#hunk-24))
- swarm-example-* (T27) at `Cargo.lock:704` ([hunk](#hunk-25))
- swarm-example-* (T27) at `Cargo.lock:760` ([hunk](#hunk-26))
- swarm-example-* (T27) at `Cargo.lock:772` ([hunk](#hunk-27))
- swarm-example-* (T27) at `Cargo.lock:783` ([hunk](#hunk-28))
- swarm-example-* (T27) at `Cargo.lock:819` ([hunk](#hunk-29))
- swarm-example-* (T27) at `Cargo.lock:826` ([hunk](#hunk-30))
- swarm-example-* (T27) at `Cargo.lock:841` ([hunk](#hunk-31))
- swarm-example-* (T27) at `Cargo.lock:872` ([hunk](#hunk-32))
- swarm-example-* (T27) at `Cargo.lock:896` ([hunk](#hunk-33))
- swarm-example-* (T27) at `Cargo.lock:937` ([hunk](#hunk-34))
- swarm-example-* (T27) at `Cargo.lock:952` ([hunk](#hunk-35))
- swarm-example-* (T27) at `Cargo.lock:970` ([hunk](#hunk-36))
- swarm-example-* (T27) at `Cargo.lock:1021` ([hunk](#hunk-37))
- swarm-example-* (T27) at `Cargo.lock:1040` ([hunk](#hunk-38))
- swarm-example-* (T27) at `Cargo.lock:1058` ([hunk](#hunk-39))
- swarm-example-* (T27) at `Cargo.lock:1076` ([hunk](#hunk-40))
- swarm-example-* (T27) at `Cargo.lock:1200` ([hunk](#hunk-41))
- swarm-example-* (T27) at `Cargo.lock:1220` ([hunk](#hunk-42))
- swarm-example-* (T27) at `Cargo.lock:1253` ([hunk](#hunk-43))
- swarm-example-* (T27) at `Cargo.lock:1266` ([hunk](#hunk-44))
- swarm-example-* (T27) at `Cargo.lock:1273` ([hunk](#hunk-45))
- swarm-example-* (T27) at `Cargo.lock:1291` ([hunk](#hunk-46))
- swarm-example-* (T27) at `Cargo.lock:1322` ([hunk](#hunk-47))
- swarm-example-* (T27) at `Cargo.lock:1331` ([hunk](#hunk-48))
- swarm-example-* (T27) at `Cargo.lock:1376` ([hunk](#hunk-49))
- swarm-example-* (T27) at `Cargo.lock:1392` ([hunk](#hunk-50))
- swarm-example-* (T27) at `Cargo.lock:1409` ([hunk](#hunk-51))
- swarm-example-* (T27) at `Cargo.lock:1468` ([hunk](#hunk-52))
- swarm-example-* (T27) at `Cargo.lock:1481` ([hunk](#hunk-53))
- swarm-example-* (T27) at `Cargo.lock:1511` ([hunk](#hunk-54))
- swarm-example-* (T27) at `Cargo.lock:1528` ([hunk](#hunk-55))
- swarm-example-* (T27) at `Cargo.lock:1579` ([hunk](#hunk-56))
- swarm-example-* (T27) at `Cargo.lock:1612` ([hunk](#hunk-57))
- swarm-example-* (T27) at `Cargo.lock:1630` ([hunk](#hunk-58))
- swarm-example-* (T27) at `Cargo.lock:1663` ([hunk](#hunk-59))
- swarm-example-* (T27) at `Cargo.lock:1698` ([hunk](#hunk-60))
- swarm-example-* (T27) at `Cargo.lock:1779` ([hunk](#hunk-61))
- swarm-example-* (T27) at `Cargo.lock:1820` ([hunk](#hunk-62))
- swarm-example-* (T27) at `Cargo.lock:1846` ([hunk](#hunk-63))
- swarm-example-* (T27) at `Cargo.lock:1855` ([hunk](#hunk-64))
- swarm-example-* (T27) at `Cargo.lock:1906` ([hunk](#hunk-65))
- swarm-example-* (T27) at `Cargo.lock:1922` ([hunk](#hunk-66))
- swarm-example-* (T27) at `Cargo.lock:1934` ([hunk](#hunk-67))
- swarm-example-* (T27) at `Cargo.lock:1990` ([hunk](#hunk-68))
- swarm-example-* (T27) at `Cargo.toml:18` ([hunk](#hunk-69))
- swarm-example-* (T27) at `Cargo.toml:46` ([hunk](#hunk-70))
- swarm-example-* (T27) at `Cargo.toml:62` ([hunk](#hunk-71))
- swarm-example-* (T27) at `Cargo.toml:161` ([hunk](#hunk-72))
- swarm-example-* (T27) at `Cargo.toml:162` ([hunk](#hunk-72))
- swarm-example-* (T27) at `Cargo.toml:163` ([hunk](#hunk-72))
- swarm-example-* (T27) at `Cargo.toml:188` ([hunk](#hunk-73))
- swarm-example-* (T27) at `examples/swarm.rs:0` ([hunk](#hunk-74))
- swarm-example-* (T27) at `examples/swarm/tests.rs:0` ([hunk](#hunk-75))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p1-swarm.tsv `@@ -0,0 +1,80 @@`

```diff
@@ -0,0 +1,80 @@
+# Lane p1-swarm, ruling T27: the swarm example is deleted. The entry id `swarm-example-*` stands for all
+# 31 ledger entries swarm-example-1 through swarm-example-31; the deletion resolves each of them and lands
+# none of their individual resolutions. Line numbers are new-side lines of `git diff 0926fe32...HEAD`;
+# a wholly deleted file is annotated at line 0 (its hunk header is `+0,0`).
+examples/swarm.rs	0	swarm-example-*	T27	Ruling T27 deletes the example; this file is the whole of it: the module doc, the party threads, the steady-state controller, the memory-link connector shim, the ratatui terminal UI, and the headless mode. Every swarm-example-* entry described a defect in this file or its test; none of their resolutions is landed, because the deletion supersedes them.\nHow I searched for references: `git grep -n -i swarm` over the whole tree minus the example itself, .agent-notes, and the before-viz www bundle (hits: the [[example]] block, and a fleet-shaped local variable in tests/window_census.rs that is not this example and stays); a whole-word grep for `example`/`examples` outside src/, crates/, formal/ (only unrelated hits: the Peer docs' lifecycle example, before's amp_board recipes, the envelope_sim and window_tradeoff recipes, fixture URLs in tools/); greps for the example's other possible names (steady-state controller, showcase, TUI, terminal UI, sparkline, ratatui, crossterm, arc-swap, clap); and a read of the justfile, .github/workflows, .cargo/mutants.toml, .config/nextest.toml, deny.toml, tools/*, README.md, AGENTS.md, design/, results/, .gitignore, proptest-regressions/, and tests/seed_liveness.rs. No file outside the example imported from it or depended on its behavior, and no design document cites it.
+examples/swarm/tests.rs	0	swarm-example-*	T27	The example's single unit test (the controller's convergence trajectory; swarm-example-29, -30, and -31 all name it) goes with the example. It ran as a test target only because the [[example]] block set `test = true`; no other target shares it, it used no proptest, and no seed file under proptest-regressions/ anchors to it.
+Cargo.toml	188	swarm-example-*	T27	The [[example]] block: with the files gone this is the last pointer at them, and its comment named the controller tests that no longer exist. The block's trailing blank line goes with it so [profile.bench] and [[bench]] keep one blank line between them.
+Cargo.toml	161	swarm-example-*	T27	Judgment call beyond the brief's letter: `clap`, `arc-swap`, and `ratatui` were declared in [dev-dependencies] for this example alone (verified by whole-word grep of every other target: benches, tests, the other two examples, src, every crate under crates/; before-fuelscape pins its own `clap` in a detached workspace). A dev-dependency that exists only to build deleted code is part of the example's footprint, so it goes: leaving it would keep a terminal-UI stack in the lock for nothing. `rand`'s `small_rng` feature and `pollster` stay: two benches and envelope_sim use SmallRng, and the conformance and bench suites use pollster. This line: `clap`.
+Cargo.toml	162	swarm-example-*	T27	Judgment call beyond the brief's letter: `clap`, `arc-swap`, and `ratatui` were declared in [dev-dependencies] for this example alone (verified by whole-word grep of every other target: benches, tests, the other two examples, src, every crate under crates/; before-fuelscape pins its own `clap` in a detached workspace). A dev-dependency that exists only to build deleted code is part of the example's footprint, so it goes: leaving it would keep a terminal-UI stack in the lock for nothing. `rand`'s `small_rng` feature and `pollster` stay: two benches and envelope_sim use SmallRng, and the conformance and bench suites use pollster. This line: `arc-swap`.
+Cargo.toml	163	swarm-example-*	T27	Judgment call beyond the brief's letter: `clap`, `arc-swap`, and `ratatui` were declared in [dev-dependencies] for this example alone (verified by whole-word grep of every other target: benches, tests, the other two examples, src, every crate under crates/; before-fuelscape pins its own `clap` in a detached workspace). A dev-dependency that exists only to build deleted code is part of the example's footprint, so it goes: leaving it would keep a terminal-UI stack in the lock for nothing. `rand`'s `small_rng` feature and `pollster` stay: two benches and envelope_sim use SmallRng, and the conformance and bench suites use pollster. This line: `ratatui`.
+Cargo.toml	18	swarm-example-*	T27	Same judgment call as the [dev-dependencies] rows: [workspace.dependencies] is the version of record, and with the member declaration removed this entry had no inheritor anywhere (grep of every member manifest and the detached workspaces). Removing it keeps the table's own premise true: every entry has a member that inherits it. This line: `arc-swap`.
+Cargo.toml	46	swarm-example-*	T27	Same judgment call as the [dev-dependencies] rows: [workspace.dependencies] is the version of record, and with the member declaration removed this entry had no inheritor anywhere (grep of every member manifest and the detached workspaces). Removing it keeps the table's own premise true: every entry has a member that inherits it. This line: `clap`.
+Cargo.toml	62	swarm-example-*	T27	Same judgment call as the [dev-dependencies] rows: [workspace.dependencies] is the version of record, and with the member declaration removed this entry had no inheritor anywhere (grep of every member manifest and the detached workspaces). Removing it keeps the table's own premise true: every entry has a member that inherits it. This line: `ratatui`.
+Cargo.lock	11	swarm-example-*	T27	Regenerated with `cargo update --workspace`, which drops packages no longer reachable and bumps nothing (cargo's own note: 78 unchanged dependencies behind latest). The lock shrinks from 338 to 219 packages. Measured by diffing name/version pairs between the base lock and this one: 119 pairs dropped, none added. Eleven of the dropped pairs are versions of crates the lock still carries at another version, so the example's TUI dependency tree was their only holder: syn 1.0.109, thiserror 1.0.69, thiserror-impl 1.0.69, bitflags 1.3.2, bit-set 0.5.3, bit-vec 0.6.3, cpufeatures 0.2.17, crypto-common 0.1.7, digest 0.10.7, foldhash 0.2.0, and hashbrown 0.16.1 (held by kasuari, ratatui's layout solver). The first ten now resolve to one version, which is why the added lines exist: cargo drops the version suffix from a dependency name once it no longer disambiguates. hashbrown goes from three versions to two (0.15.5 and 0.17.1 remain). Nine duplicate-version crates remain (embedded-io, getrandom, hashbrown, itertools, r-efi, rand, rand_chacha, rand_core, wit-bindgen); apart from hashbrown's third version, none was pulled by the example. One row per hunk below because the packet tool anchors per hunk; every hunk is the same regeneration.
+Cargo.lock	38	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	57	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	98	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	114	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	147	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	172	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	187	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	270	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	278	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	294	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	325	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	370	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	395	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	410	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	435	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	442	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	494	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	518	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	577	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	609	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	662	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	670	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	704	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	760	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	772	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	783	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	819	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	826	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	841	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	872	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	896	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	937	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	952	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	970	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1021	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1040	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1058	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1076	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1200	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1220	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1253	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1266	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1273	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1291	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1322	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1331	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1376	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1392	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1409	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1468	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1481	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1511	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1528	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1579	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1612	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1630	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1663	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1698	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1779	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1820	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1846	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1855	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1906	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1922	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1934	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
+Cargo.lock	1990	swarm-example-*	T27	Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### Cargo.lock `@@ -11,83 +11,24 @@ dependencies = [`

```diff
@@ -11,83 +11,24 @@ dependencies = [
  "memchr",
 ]
 
-[[package]]
-name = "allocator-api2"
-version = "0.2.21"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "683d7910e743518b0e34f1186f92494becacb047c7b6bf616c96772180fef923"
-
 [[package]]
 name = "anes"
 version = "0.1.6"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "4b46cbb362ab8752921c97e041f5e366ee6297bd428a31275b9fcf1e380f7299"
 
-[[package]]
-name = "anstream"
-version = "1.0.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "824a212faf96e9acacdbd09febd34438f8f711fb84e09a8916013cd7815ca28d"
-dependencies = [
- "anstyle",
- "anstyle-parse",
- "anstyle-query",
- "anstyle-wincon",
- "colorchoice",
- "is_terminal_polyfill",
- "utf8parse",
-]
-
 [[package]]
 name = "anstyle"
 version = "1.0.14"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "940b3a0ca603d1eade50a4846a2afffd5ef57a9feac2c0e2ec2e14f9ead76000"
 
-[[package]]
-name = "anstyle-parse"
-version = "1.0.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "52ce7f38b242319f7cabaa6813055467063ecdc9d355bbb4ce0c68908cd8130e"
-dependencies = [
- "utf8parse",
-]
-
-[[package]]
-name = "anstyle-query"
-version = "1.1.5"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "40c48f72fd53cd289104fc64099abca73db4166ad86ea0b4341abe65af83dadc"
-dependencies = [
- "windows-sys",
-]
-
-[[package]]
-name = "anstyle-wincon"
-version = "3.0.11"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "291e6a250ff86cd4a820112fb8898808a366d8f9f58ce16d1f538353ad55747d"
-dependencies = [
- "anstyle",
- "once_cell_polyfill",
- "windows-sys",
-]
-
 [[package]]
 name = "anyhow"
 version = "1.0.104"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "330a5ed07fa54e4702c9d6c4174f74427fc0ef6e214bbd677ae50a5099946470"
 
-[[package]]
-name = "approx"
-version = "0.5.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "cab112f0a86d568ea0e627cc1d6be74a1e9cd55214684db5561995f6dad897c6"
-dependencies = [
- "num-traits",
-]
-
 [[package]]
 name = "ar_archive_writer"
 version = "0.5.1"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 11:
>
> Regenerated with `cargo update --workspace`, which drops packages no longer reachable and bumps nothing (cargo's own note: 78 unchanged dependencies behind latest). The lock shrinks from 338 to 219 packages. Measured by diffing name/version pairs between the base lock and this one: 119 pairs dropped, none added. Eleven of the dropped pairs are versions of crates the lock still carries at another version, so the example's TUI dependency tree was their only holder: syn 1.0.109, thiserror 1.0.69, thiserror-impl 1.0.69, bitflags 1.3.2, bit-set 0.5.3, bit-vec 0.6.3, cpufeatures 0.2.17, crypto-common 0.1.7, digest 0.10.7, foldhash 0.2.0, and hashbrown 0.16.1 (held by kasuari, ratatui's layout solver). The first ten now resolve to one version, which is why the added lines exist: cargo drops the version suffix from a dependency name once it no longer disambiguates. hashbrown goes from three versions to two (0.15.5 and 0.17.1 remain). Nine duplicate-version crates remain (embedded-io, getrandom, hashbrown, itertools, r-efi, rand, rand_chacha, rand_core, wit-bindgen); apart from hashbrown's third version, none was pulled by the example. One row per hunk below because the packet tool anchors per hunk; every hunk is the same regeneration.

<a id="hunk-3"></a>
### Cargo.lock `@@ -97,15 +38,6 @@ dependencies = [`

```diff
@@ -97,15 +38,6 @@ dependencies = [
  "object",
 ]
 
-[[package]]
-name = "arc-swap"
-version = "1.9.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "6a3a1fd6f75306b68087b831f025c712524bcb19aad54e557b1129cfa0a2b207"
-dependencies = [
- "rustversion",
-]
-
 [[package]]
 name = "async-stream"
 version = "0.3.6"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 38:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-4"></a>
### Cargo.lock `@@ -125,16 +57,7 @@ checksum = "c7c24de15d275a1ecfd47a380fb4d5ec9bfe0933f309ed5e705b775596a3574d"`

```diff
@@ -125,16 +57,7 @@ checksum = "c7c24de15d275a1ecfd47a380fb4d5ec9bfe0933f309ed5e705b775596a3574d"
 dependencies = [
  "proc-macro2",
  "quote",
- "syn 2.0.117",
-]
-
-[[package]]
-name = "atomic"
-version = "0.6.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "a89cbf775b137e9b968e67227ef7f775587cde3fd31b0d8599dbd0f598a48340"
-dependencies = [
- "bytemuck",
+ "syn",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 57:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-5"></a>
### Cargo.lock `@@ -175,7 +98,7 @@ dependencies = [`

```diff
@@ -175,7 +98,7 @@ dependencies = [
  "static_assertions",
  "suanpan",
  "surface-scan",
- "thiserror 2.0.18",
+ "thiserror",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 98:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-6"></a>
### Cargo.lock `@@ -191,42 +114,21 @@ dependencies = [`

```diff
@@ -191,42 +114,21 @@ dependencies = [
  "wasm-bindgen",
 ]
 
-[[package]]
-name = "bit-set"
-version = "0.5.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "0700ddab506f33b20a03b13996eccd309a48e5ff77d0d95926aa0210fb4e95f1"
-dependencies = [
- "bit-vec 0.6.3",
-]
-
 [[package]]
 name = "bit-set"
 version = "0.8.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "08807e080ed7f9d5433fa9b275196cfc35414f66a0c79d864dc51a0d825231a3"
 dependencies = [
- "bit-vec 0.8.0",
+ "bit-vec",
 ]
 
-[[package]]
-name = "bit-vec"
-version = "0.6.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "349f9b6a179ed607305526ca489b34ad0a41aed5f7980fa90eb03160b69598fb"
-
 [[package]]
 name = "bit-vec"
 version = "0.8.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "5e764a1d40d510daf35e07be9eb06e75770908c27d411ee6c92109c9840eaaf7"
 
-[[package]]
-name = "bitflags"
-version = "1.3.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "bef38d45163c2f1dde094a7dfd33ccf595c92905c8f8f4fdc18d06fb1037718a"
-
 [[package]]
 name = "bitflags"
 version = "2.12.1"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 114:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-7"></a>
### Cargo.lock `@@ -245,15 +147,6 @@ dependencies = [`

```diff
@@ -245,15 +147,6 @@ dependencies = [
  "wyz",
 ]
 
-[[package]]
-name = "block-buffer"
-version = "0.10.4"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71"
-dependencies = [
- "generic-array",
-]
-
 [[package]]
 name = "borsh"
 version = "1.6.1"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 147:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-8"></a>
### Cargo.lock `@@ -279,18 +172,6 @@ version = "3.20.3"`

```diff
@@ -279,18 +172,6 @@ version = "3.20.3"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "72f5acc6cb2ba439de613abc23857ec3d78374d8ed5ac84e9d11336e87da8649"
 
-[[package]]
-name = "by_address"
-version = "1.2.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "64fa3c856b712db6612c019f14756e64e4bcea13337a6b33b696333a9eaa2d06"
-
-[[package]]
-name = "bytemuck"
-version = "1.25.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "c8efb64bd706a16a1bdde310ae86b351e4d21550d98d056f22f8a7f7a2183fec"
-
 [[package]]
 name = "bytes"
 version = "1.11.1"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 172:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-9"></a>
### Cargo.lock `@@ -306,15 +187,6 @@ version = "0.3.0"`

```diff
@@ -306,15 +187,6 @@ version = "0.3.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "37b2a672a2cb129a2e41c10b1224bb368f9f37a2b16b612598138befd7b37eb5"
 
-[[package]]
-name = "castaway"
-version = "0.2.4"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "dec551ab6e7578819132c713a93c022a05d60159dc86e7a7050223577484c55a"
-dependencies = [
- "rustversion",
-]
-
 [[package]]
 name = "cbor-diag"
 version = "0.1.12"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 187:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-10"></a>
### Cargo.lock `@@ -398,7 +270,6 @@ source = "registry+https://github.com/rust-lang/crates.io-index"`

```diff
@@ -398,7 +270,6 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "1ddb117e43bbf7dacf0a4190fef4d345b9bad68dfc649cb349e7d17d28428e51"
 dependencies = [
  "clap_builder",
- "clap_derive",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 270:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-11"></a>
### Cargo.lock `@@ -407,22 +278,8 @@ version = "4.6.0"`

```diff
@@ -407,22 +278,8 @@ version = "4.6.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "714a53001bf66416adb0e2ef5ac857140e7dc3a0c48fb28b2f10762fc4b5069f"
 dependencies = [
- "anstream",
  "anstyle",
  "clap_lex",
- "strsim",
-]
-
-[[package]]
-name = "clap_derive"
-version = "4.6.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "f2ce8604710f6733aa641a2b3731eaa1e8b3d9973d5e3565da11800813f997a9"
-dependencies = [
- "heck",
- "proc-macro2",
- "quote",
- "syn 2.0.117",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 278:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-12"></a>
### Cargo.lock `@@ -437,27 +294,7 @@ version = "0.3.0"`

```diff
@@ -437,27 +294,7 @@ version = "0.3.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "0fa961b519f0b462e3a3b4a34b64d119eeaca1d59af726fe450bbba07a9fc0a1"
 dependencies = [
- "thiserror 2.0.18",
-]
-
-[[package]]
-name = "colorchoice"
-version = "1.0.5"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "1d07550c9036bf2ae0c684c4297d503f838287c83c53686d05370d0e139ae570"
-
-[[package]]
-name = "compact_str"
-version = "0.9.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "9dfdd1c2274d9aa354115b09dc9a901d6c5576818cdf70d14cae2bdb47df00ab"
-dependencies = [
- "castaway",
- "cfg-if",
- "itoa",
- "rustversion",
- "ryu",
- "static_assertions",
+ "thiserror",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 294:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-13"></a>
### Cargo.lock `@@ -488,24 +325,6 @@ version = "0.10.2"`

```diff
@@ -488,24 +325,6 @@ version = "0.10.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "a6ef517f0926dd24a1582492c791b6a4818a4d94e789a334894aa15b0d12f55c"
 
-[[package]]
-name = "convert_case"
-version = "0.10.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "633458d4ef8c78b72454de2d54fd6ab2e60f9e02be22f3c6104cdc8a4e0fceb9"
-dependencies = [
- "unicode-segmentation",
-]
-
-[[package]]
-name = "cpufeatures"
-version = "0.2.17"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "59ed5838eebb26a2bb2e58f6d5b5316989ae9d08bab10e0e6d103e656d1b0280"
-dependencies = [
- "libc",
-]
-
 [[package]]
 name = "cpufeatures"
 version = "0.3.1"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 325:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-14"></a>
### Cargo.lock `@@ -551,12 +370,6 @@ dependencies = [`

```diff
@@ -551,12 +370,6 @@ dependencies = [
  "itertools 0.10.5",
 ]
 
-[[package]]
-name = "critical-section"
-version = "1.2.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "790eea4361631c5e7d22598ecd5723ff611904e3344ce8720784c93e3d83d40b"
-
 [[package]]
 name = "crossbeam-deque"
 version = "0.8.6"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 370:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-15"></a>
### Cargo.lock `@@ -582,49 +395,12 @@ version = "0.8.21"`

```diff
@@ -582,49 +395,12 @@ version = "0.8.21"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "d0a5c400df2834b80a4c3327b3aad3a4c4cd4de0629063962b03235697506a28"
 
-[[package]]
-name = "crossterm"
-version = "0.29.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d8b9f2e4c67f833b660cdb0a3523065869fb35570177239812ed4c905aeff87b"
-dependencies = [
- "bitflags 2.12.1",
- "crossterm_winapi",
- "derive_more",
- "document-features",
- "mio",
- "parking_lot",
- "rustix",
- "signal-hook",
- "signal-hook-mio",
- "winapi",
-]
-
-[[package]]
-name = "crossterm_winapi"
-version = "0.9.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "acdd7c62a3665c7f6830a51635d9ac9b23ed385797f70a83bb8bafe9c572ab2b"
-dependencies = [
- "winapi",
-]
-
 [[package]]
 name = "crunchy"
 version = "0.2.4"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "460fbee9c2c2f33933d720630a6a0bac33ba7053db5344fac858d4b8952d77d5"
 
-[[package]]
-name = "crypto-common"
-version = "0.1.7"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "78c8292055d1c1df0cce5d180393dc8cce0abec0a7102adb6c7b1eef6016d60a"
-dependencies = [
- "generic-array",
- "typenum",
-]
-
 [[package]]
 name = "crypto-common"
 version = "0.2.2"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 395:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-16"></a>
### Cargo.lock `@@ -634,50 +410,6 @@ dependencies = [`

```diff
@@ -634,50 +410,6 @@ dependencies = [
  "hybrid-array",
 ]
 
-[[package]]
-name = "csscolorparser"
-version = "0.6.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "eb2a7d3066da2de787b7f032c736763eb7ae5d355f81a68bab2675a96008b0bf"
-dependencies = [
- "lab",
- "phf",
-]
-
-[[package]]
-name = "darling"
-version = "0.23.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "25ae13da2f202d56bd7f91c25fba009e7717a1e4a1cc98a76d844b65ae912e9d"
-dependencies = [
- "darling_core",
- "darling_macro",
-]
-
-[[package]]
-name = "darling_core"
-version = "0.23.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "9865a50f7c335f53564bb694ef660825eb8610e0a53d3e11bf1b0d3df31e03b0"
-dependencies = [
- "ident_case",
- "proc-macro2",
- "quote",
- "strsim",
- "syn 2.0.117",
-]
-
-[[package]]
-name = "darling_macro"
-version = "0.23.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "ac3984ec7bd6cfa798e62b4a642426a5be0e68f9401cfc2a01e3fa9ea2fcdb8d"
-dependencies = [
- "darling_core",
- "quote",
- "syn 2.0.117",
-]
-
 [[package]]
 name = "dashu-base"
 version = "0.5.0"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 410:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-17"></a>
### Cargo.lock `@@ -703,53 +435,6 @@ version = "2.11.1"`

```diff
@@ -703,53 +435,6 @@ version = "2.11.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "4583a4551df46e2792f82ceeac45e850d2e2d5debba0b91f102385cda5b11f06"
 
-[[package]]
-name = "deltae"
-version = "0.3.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "5729f5117e208430e437df2f4843f5e5952997175992d1414f94c57d61e270b4"
-
-[[package]]
-name = "deranged"
-version = "0.5.8"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "7cd812cc2bc1d69d4764bd80df88b4317eaef9e773c75226407d9bc0876b211c"
-dependencies = [
- "powerfmt",
-]
-
-[[package]]
-name = "derive_more"
-version = "2.1.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d751e9e49156b02b44f9c1815bcb94b984cdcc4396ecc32521c739452808b134"
-dependencies = [
- "derive_more-impl",
-]
-
-[[package]]
-name = "derive_more-impl"
-version = "2.1.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "799a97264921d8623a957f6c3b9011f3b5492f557bbb7a5a19b7fa6d06ba8dcb"
-dependencies = [
- "convert_case",
- "proc-macro2",
- "quote",
- "rustc_version",
- "syn 2.0.117",
-]
-
-[[package]]
-name = "digest"
-version = "0.10.7"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "9ed9a281f7bc9b7576e61468ba615a66a5c8cfdff42420a70aa82701a3b1e292"
-dependencies = [
- "block-buffer",
- "crypto-common 0.1.7",
-]
-
 [[package]]
 name = "digest"
 version = "0.11.3"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 435:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-18"></a>
### Cargo.lock `@@ -757,16 +442,7 @@ source = "registry+https://github.com/rust-lang/crates.io-index"`

```diff
@@ -757,16 +442,7 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "f1dd6dbb5841937940781866fa1281a1ff7bd3bf827091440879f9994983d5c2"
 dependencies = [
  "const-oid",
- "crypto-common 0.2.2",
-]
-
-[[package]]
-name = "document-features"
-version = "0.2.12"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d4b8a88685455ed29a21542a33abd9cb6510b6b129abadabdcef0f4c55bc8f61"
-dependencies = [
- "litrs",
+ "crypto-common",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 442:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-19"></a>
### Cargo.lock `@@ -818,66 +494,18 @@ dependencies = [`

```diff
@@ -818,66 +494,18 @@ dependencies = [
  "windows-sys",
 ]
 
-[[package]]
-name = "euclid"
-version = "0.22.14"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "f1a05365e3b1c6d1650318537c7460c6923f1abdd272ad6842baa2b509957a06"
-dependencies = [
- "num-traits",
-]
-
-[[package]]
-name = "fancy-regex"
-version = "0.11.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "b95f7c0680e4142284cf8b22c14a476e87d61b004a3a0861872b32ef7ead40a2"
-dependencies = [
- "bit-set 0.5.3",
- "regex",
-]
-
-[[package]]
-name = "fast-srgb8"
-version = "1.0.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "dd2e7510819d6fbf51a5545c8f922716ecfb14df168a3242f7d33e0239efe6a1"
-
 [[package]]
 name = "fastrand"
 version = "2.4.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "9f1f227452a390804cdb637b74a86990f2a7d7ba4b7d5693aac9b4dd6defd8d6"
 
-[[package]]
-name = "filedescriptor"
-version = "0.8.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "e40758ed24c9b2eeb76c35fb0aebc66c626084edd827e07e1552279814c6682d"
-dependencies = [
- "libc",
- "thiserror 1.0.69",
- "winapi",
-]
-
 [[package]]
 name = "find-msvc-tools"
 version = "0.1.9"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "5baebc0774151f905a1a2cc41989300b1e6fbb29aff0ceffa1064fdd3088d582"
 
-[[package]]
-name = "finl_unicode"
-version = "1.4.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "9844ddc3a6e533d62bba727eb6c28b5d360921d5175e9ff0f1e621a5c590a4d5"
-
-[[package]]
-name = "fixedbitset"
-version = "0.4.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "0ce7134b9999ecaf8bcd65542e436736ef32ddca1b3e06094cb6ec5755203b80"
-
 [[package]]
 name = "fnv"
 version = "1.0.7"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 494:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-20"></a>
### Cargo.lock `@@ -890,12 +518,6 @@ version = "0.1.5"`

```diff
@@ -890,12 +518,6 @@ version = "0.1.5"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "d9c4f5dac5e15c24eb999c26181a6ca40b39fe946cbe4c263c7209467bc83af2"
 
-[[package]]
-name = "foldhash"
-version = "0.2.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "77ce24cb58228fbb8aa041425bb1050850ac19177686ea6e0f41a70416f56fdb"
-
 [[package]]
 name = "form_urlencoded"
 version = "1.2.2"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 518:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-21"></a>
### Cargo.lock `@@ -955,7 +577,7 @@ checksum = "e835b70203e41293343137df5c0664546da5745f82ec9b84d40be8336958447b"`

```diff
@@ -955,7 +577,7 @@ checksum = "e835b70203e41293343137df5c0664546da5745f82ec9b84d40be8336958447b"
 dependencies = [
  "proc-macro2",
  "quote",
- "syn 2.0.117",
+ "syn",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 577:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-22"></a>
### Cargo.lock `@@ -987,16 +609,6 @@ dependencies = [`

```diff
@@ -987,16 +609,6 @@ dependencies = [
  "slab",
 ]
 
-[[package]]
-name = "generic-array"
-version = "0.14.7"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "85649ca51fd72272d7821adaf274ad91c288277713d9c18820d8499a7ff69e9a"
-dependencies = [
- "typenum",
- "version_check",
-]
-
 [[package]]
 name = "getrandom"
 version = "0.2.17"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 609:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-23"></a>
### Cargo.lock `@@ -1050,18 +662,7 @@ version = "0.15.5"`

```diff
@@ -1050,18 +662,7 @@ version = "0.15.5"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "9229cfe53dfd69f0609a49f65461bd93001ea1ef889cd5529dd176593f5338a1"
 dependencies = [
- "foldhash 0.1.5",
-]
-
-[[package]]
-name = "hashbrown"
-version = "0.16.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "841d1cc9bed7f9236f321df977030373f4a4163ae1a7dbfe1a51a2c1a51d9100"
-dependencies = [
- "allocator-api2",
- "equivalent",
- "foldhash 0.2.0",
+ "foldhash",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 662:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-24"></a>
### Cargo.lock `@@ -1069,11 +670,6 @@ name = "hashbrown"`

```diff
@@ -1069,11 +670,6 @@ name = "hashbrown"
 version = "0.17.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "ed5909b6e89a2db4456e54cd5f673791d7eca6732202bbf2a9cc504fe2f9b84a"
-dependencies = [
- "allocator-api2",
- "equivalent",
- "foldhash 0.2.0",
-]
 
 [[package]]
 name = "heck"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 670:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-25"></a>
### Cargo.lock `@@ -1108,12 +704,6 @@ version = "2.3.0"`

```diff
@@ -1108,12 +704,6 @@ version = "2.3.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "3d3067d79b975e8844ca9eb072e16b31c3c1c36928edf9c6789548c524d0d954"
 
-[[package]]
-name = "ident_case"
-version = "1.0.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "b9e0384b61958566e926dc50660321d12159025e767c18e043daf26b70104c39"
-
 [[package]]
 name = "idna"
 version = "1.1.0"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 704:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-26"></a>
### Cargo.lock `@@ -1170,15 +760,6 @@ dependencies = [`

```diff
@@ -1170,15 +760,6 @@ dependencies = [
  "web-time",
 ]
 
-[[package]]
-name = "indoc"
-version = "2.0.7"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "79cf5c93f93228cf8efb3ba362535fb11199ac548a09ce117c9b1adc3030d706"
-dependencies = [
- "rustversion",
-]
-
 [[package]]
 name = "insta"
 version = "1.47.2"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 760:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-27"></a>
### Cargo.lock `@@ -1191,19 +772,6 @@ dependencies = [`

```diff
@@ -1191,19 +772,6 @@ dependencies = [
  "tempfile",
 ]
 
-[[package]]
-name = "instability"
-version = "0.3.12"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "5eb2d60ef19920a3a9193c3e371f726ec1dafc045dac788d0fb3704272458971"
-dependencies = [
- "darling",
- "indoc",
- "proc-macro2",
- "quote",
- "syn 2.0.117",
-]
-
 [[package]]
 name = "is-terminal"
 version = "0.4.17"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 772:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-28"></a>
### Cargo.lock `@@ -1215,12 +783,6 @@ dependencies = [`

```diff
@@ -1215,12 +783,6 @@ dependencies = [
  "windows-sys",
 ]
 
-[[package]]
-name = "is_terminal_polyfill"
-version = "1.70.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "a6cb138bb79a146c1bd460005623e142ef0181e3d0219cb493e02f7d08a35695"
-
 [[package]]
 name = "itertools"
 version = "0.10.5"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 783:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-29"></a>
### Cargo.lock `@@ -1257,17 +819,6 @@ dependencies = [`

```diff
@@ -1257,17 +819,6 @@ dependencies = [
  "wasm-bindgen",
 ]
 
-[[package]]
-name = "kasuari"
-version = "0.4.12"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "bde5057d6143cc94e861d90f591b9303d6716c6b9602309150bd068853c10899"
-dependencies = [
- "hashbrown 0.16.1",
- "portable-atomic",
- "thiserror 2.0.18",
-]
-
 [[package]]
 name = "keccak"
 version = "0.2.2"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 819:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-30"></a>
### Cargo.lock `@@ -1275,21 +826,9 @@ source = "registry+https://github.com/rust-lang/crates.io-index"`

```diff
@@ -1275,21 +826,9 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "d8f198d1db720e4940b5a493201d199d9f24f568f8f746bd13706243a2f71598"
 dependencies = [
  "cfg-if",
- "cpufeatures 0.3.1",
+ "cpufeatures",
 ]
 
-[[package]]
-name = "lab"
-version = "0.11.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "bf36173d4167ed999940f804952e6b08197cae5ad5d572eb4db150ce8ad5d58f"
-
-[[package]]
-name = "lazy_static"
-version = "1.5.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "bbd2bcb4c963f2ddae06a2efc7e9f3591312473c50c6685e1f298068316e66fe"
-
 [[package]]
 name = "leb128fmt"
 version = "0.1.0"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 826:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-31"></a>
### Cargo.lock `@@ -1302,88 +841,24 @@ version = "0.2.186"`

```diff
@@ -1302,88 +841,24 @@ version = "0.2.186"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "68ab91017fe16c622486840e4c83c9a37afeff978bd239b5293d61ece587de66"
 
-[[package]]
-name = "libm"
-version = "0.2.16"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "b6d2cec3eae94f9f509c767b45932f1ada8350c4bdb85af2fcab4a3c14807981"
-
-[[package]]
-name = "line-clipping"
-version = "0.3.7"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "3f50e8f47623268b5407192d26876c4d7f89d686ca130fdc53bced4814cd29f8"
-dependencies = [
- "bitflags 2.12.1",
-]
-
 [[package]]
 name = "linux-raw-sys"
 version = "0.12.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "32a66949e030da00e8c7d4434b251670a91556f4144941d37452769c25d58a53"
 
-[[package]]
-name = "litrs"
-version = "1.0.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "11d3d7f243d5c5a8b9bb5d6dd2b1602c0cb0b9db1621bafc7ed66e35ff9fe092"
-
-[[package]]
-name = "lock_api"
-version = "0.4.14"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "224399e74b87b5f3557511d98dff8b14089b3dadafcab6bb93eab67d3aace965"
-dependencies = [
- "scopeguard",
-]
-
 [[package]]
 name = "log"
 version = "0.4.30"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "616ec5685824bcc94416c6d4a7a446eea774a31efd7062c8480ba6fd06d7a6e5"
 
-[[package]]
-name = "lru"
-version = "0.18.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "5d2f2f9b4ba7e6b24d95e7e899329d35be83bcded72c8540cdd5368932d1d90a"
-dependencies = [
- "hashbrown 0.17.1",
-]
-
-[[package]]
-name = "mac_address"
-version = "1.1.8"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "c0aeb26bf5e836cc1c341c8106051b573f1766dfa05aa87f0b98be5e51b02303"
-dependencies = [
- "nix",
- "winapi",
-]
-
 [[package]]
 name = "memchr"
 version = "2.8.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "6b947ae49db0d222b1dbc6b113ce7248a3fc3a6ca21b696717bfc000ba4484d8"
 
-[[package]]
-name = "memmem"
-version = "0.1.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "a64a92489e2744ce060c349162be1c5f33c6969234104dbd99ddb5feb08b8c15"
-
-[[package]]
-name = "memoffset"
-version = "0.9.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "488016bfae457b036d996092f6cb448677611ce4449e970ceaf42695203f218a"
-dependencies = [
- "autocfg",
-]
-
 [[package]]
 name = "minimal-lexical"
 version = "0.2.1"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 841:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-32"></a>
### Cargo.lock `@@ -1397,24 +872,10 @@ source = "registry+https://github.com/rust-lang/crates.io-index"`

```diff
@@ -1397,24 +872,10 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "50b7e5b27aa02a74bac8c3f23f448f8d87ff11f92d3aac1a6ed369ee08cc56c1"
 dependencies = [
  "libc",
- "log",
  "wasi",
  "windows-sys",
 ]
 
-[[package]]
-name = "nix"
-version = "0.29.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "71e2746dc3a24dd78b3cfcb7be93368c6de9963d30f43a6a73998a9cf4b17b46"
-dependencies = [
- "bitflags 2.12.1",
- "cfg-if",
- "cfg_aliases",
- "libc",
- "memoffset",
-]
-
 [[package]]
 name = "nom"
 version = "7.1.3"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 872:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-33"></a>
### Cargo.lock `@@ -1435,23 +896,6 @@ dependencies = [`

```diff
@@ -1435,23 +896,6 @@ dependencies = [
  "num-traits",
 ]
 
-[[package]]
-name = "num-conv"
-version = "0.2.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "521739c6d2bac4aa25192232afe6841231376b2b26d4d9fae5ecf8ca5772e441"
-
-[[package]]
-name = "num-derive"
-version = "0.4.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "ed3955f1a9c7c0c15e092f9c887db08b1fc683305fdf6eb6684f22555355e202"
-dependencies = [
- "proc-macro2",
- "quote",
- "syn 2.0.117",
-]
-
 [[package]]
 name = "num-integer"
 version = "0.1.47"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 896:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-34"></a>
### Cargo.lock `@@ -1493,15 +937,6 @@ dependencies = [`

```diff
@@ -1493,15 +937,6 @@ dependencies = [
  "autocfg",
 ]
 
-[[package]]
-name = "num_threads"
-version = "0.1.7"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "5c7398b9c8b70908f6371f47ed36737907c87c52af34c268fed0bf0ceb92ead9"
-dependencies = [
- "libc",
-]
-
 [[package]]
 name = "object"
 version = "0.37.3"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 937:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-35"></a>
### Cargo.lock `@@ -1517,74 +952,12 @@ version = "1.21.4"`

```diff
@@ -1517,74 +952,12 @@ version = "1.21.4"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "9f7c3e4beb33f85d45ae3e3a1792185706c8e16d043238c593331cc7cd313b50"
 
-[[package]]
-name = "once_cell_polyfill"
-version = "1.70.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "384b8ab6d37215f3c5301a95a4accb5d64aa607f1fcb26a11b5303878451b4fe"
-
 [[package]]
 name = "oorandom"
 version = "11.1.5"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "d6790f58c7ff633d8771f42965289203411a5e5c68388703c06e14f24770b41e"
 
-[[package]]
-name = "ordered-float"
-version = "4.6.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "7bb71e1b3fa6ca1c61f383464aaf2bb0e2f8e772a1f01d486832464de363b951"
-dependencies = [
- "num-traits",
-]
-
-[[package]]
-name = "palette"
-version = "0.7.6"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "4cbf71184cc5ecc2e4e1baccdb21026c20e5fc3dcf63028a086131b3ab00b6e6"
-dependencies = [
- "approx",
- "fast-srgb8",
- "libm",
- "palette_derive",
-]
-
-[[package]]
-name = "palette_derive"
-version = "0.7.6"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "f5030daf005bface118c096f510ffb781fc28f9ab6a32ab224d8631be6851d30"
-dependencies = [
- "by_address",
- "proc-macro2",
- "quote",
- "syn 2.0.117",
-]
-
-[[package]]
-name = "parking_lot"
-version = "0.12.5"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "93857453250e3077bd71ff98b6a65ea6621a19bb0f559a85248955ac12c45a1a"
-dependencies = [
- "lock_api",
- "parking_lot_core",
-]
-
-[[package]]
-name = "parking_lot_core"
-version = "0.9.12"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "2621685985a2ebf1c516881c026032ac7deafcda1a2c9b7850dc81e3dfcb64c1"
-dependencies = [
- "cfg-if",
- "libc",
- "redox_syscall",
- "smallvec",
- "windows-link",
-]
-
 [[package]]
 name = "peak_alloc"
 version = "0.3.0"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 952:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-36"></a>
### Cargo.lock `@@ -1597,101 +970,6 @@ version = "2.3.2"`

```diff
@@ -1597,101 +970,6 @@ version = "2.3.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "9b4f627cb1b25917193a259e49bdad08f671f8d9708acfd5fe0a8c1455d87220"
 
-[[package]]
-name = "pest"
-version = "2.8.6"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "e0848c601009d37dfa3430c4666e147e49cdcf1b92ecd3e63657d8a5f19da662"
-dependencies = [
- "memchr",
- "ucd-trie",
-]
-
-[[package]]
-name = "pest_derive"
-version = "2.8.6"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "11f486f1ea21e6c10ed15d5a7c77165d0ee443402f0780849d1768e7d9d6fe77"
-dependencies = [
- "pest",
- "pest_generator",
-]
-
-[[package]]
-name = "pest_generator"
-version = "2.8.6"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "8040c4647b13b210a963c1ed407c1ff4fdfa01c31d6d2a098218702e6664f94f"
-dependencies = [
- "pest",
- "pest_meta",
- "proc-macro2",
- "quote",
- "syn 2.0.117",
-]
-
-[[package]]
-name = "pest_meta"
-version = "2.8.6"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "89815c69d36021a140146f26659a81d6c2afa33d216d736dd4be5381a7362220"
-dependencies = [
- "pest",
- "sha2",
-]
-
-[[package]]
-name = "phf"
-version = "0.11.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "1fd6780a80ae0c52cc120a26a1a42c1ae51b247a253e4e06113d23d2c2edd078"
-dependencies = [
- "phf_macros",
- "phf_shared",
-]
-
-[[package]]
-name = "phf_codegen"
-version = "0.11.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "aef8048c789fa5e851558d709946d6d79a8ff88c0440c587967f8e94bfb1216a"
-dependencies = [
- "phf_generator",
- "phf_shared",
-]
-
-[[package]]
-name = "phf_generator"
-version = "0.11.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "3c80231409c20246a13fddb31776fb942c38553c51e871f8cbd687a4cfb5843d"
-dependencies = [
- "phf_shared",
- "rand 0.8.6",
-]
-
-[[package]]
-name = "phf_macros"
-version = "0.11.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "f84ac04429c13a7ff43785d75ad27569f2951ce0ffd30a3321230db2fc727216"
-dependencies = [
- "phf_generator",
- "phf_shared",
- "proc-macro2",
- "quote",
- "syn 2.0.117",
-]
-
-[[package]]
-name = "phf_shared"
-version = "0.11.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "67eabc2ef2a60eb7faa00097bd1ffdb5bd28e62bf39990626a582201b7a754e5"
-dependencies = [
- "siphasher",
-]
-
 [[package]]
 name = "pin-project-lite"
 version = "0.2.17"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 970:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-37"></a>
### Cargo.lock `@@ -1743,7 +1021,7 @@ checksum = "ac5da421106a50887c5b51d20806867db377fbb86bacf478ee0500a912e0c113"`

```diff
@@ -1743,7 +1021,7 @@ checksum = "ac5da421106a50887c5b51d20806867db377fbb86bacf478ee0500a912e0c113"
 dependencies = [
  "proc-macro2",
  "quote",
- "syn 2.0.117",
+ "syn",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1021:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-38"></a>
### Cargo.lock `@@ -1762,13 +1040,7 @@ dependencies = [`

```diff
@@ -1762,13 +1040,7 @@ dependencies = [
  "embedded-io 0.4.0",
  "embedded-io 0.6.1",
  "serde",
-]
-
-[[package]]
-name = "powerfmt"
-version = "0.2.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "439ee305def115ba05938db6eb1644ff94165c5ab5e9420d1c1bcedbba909391"
+]
 
 [[package]]
 name = "ppv-lite86"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1040:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-39"></a>
### Cargo.lock `@@ -1786,7 +1058,7 @@ source = "registry+https://github.com/rust-lang/crates.io-index"`

```diff
@@ -1786,7 +1058,7 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "479ca8adacdd7ce8f1fb39ce9ecccbfe93a3f1344b3d0d97f20bc0196208f62b"
 dependencies = [
  "proc-macro2",
- "syn 2.0.117",
+ "syn",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1058:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-40"></a>
### Cargo.lock `@@ -1804,9 +1076,9 @@ version = "1.11.0"`

```diff
@@ -1804,9 +1076,9 @@ version = "1.11.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "4b45fcc2344c680f5025fe57779faef368840d0bd1f42f216291f0dc4ace4744"
 dependencies = [
- "bit-set 0.8.0",
- "bit-vec 0.8.0",
- "bitflags 2.12.1",
+ "bit-set",
+ "bit-vec",
+ "bitflags",
  "num-traits",
  "rand 0.9.4",
  "rand_chacha 0.9.0",
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1076:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-41"></a>
### Cargo.lock `@@ -1928,96 +1200,6 @@ dependencies = [`

```diff
@@ -1928,96 +1200,6 @@ dependencies = [
  "rand_core 0.9.5",
 ]
 
-[[package]]
-name = "ratatui"
-version = "0.30.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "1695748e3a735b34968c887ceea5a380b43545903868ae8f5b666593100f6b68"
-dependencies = [
- "instability",
- "ratatui-core",
- "ratatui-crossterm",
- "ratatui-macros",
- "ratatui-termwiz",
- "ratatui-widgets",
- "serde",
-]
-
-[[package]]
-name = "ratatui-core"
-version = "0.1.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "42d3603f354bba8c595fa47860e60142d7372b7210c27044c6a7d0e1a4336b44"
-dependencies = [
- "bitflags 2.12.1",
- "compact_str",
- "critical-section",
- "hashbrown 0.17.1",
- "indoc",
- "itertools 0.14.0",
- "kasuari",
- "lru",
- "palette",
- "serde",
- "strum",
- "thiserror 2.0.18",
- "unicode-segmentation",
- "unicode-truncate",
- "unicode-width",
-]
-
-[[package]]
-name = "ratatui-crossterm"
-version = "0.1.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "2b2867bedcbd6a690ca4f8672a687b730ec07660c79844517b084311b529980c"
-dependencies = [
- "cfg-if",
- "crossterm",
- "instability",
- "ratatui-core",
-]
-
-[[package]]
-name = "ratatui-macros"
-version = "0.7.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "80fac59720679490d89d200df411faa249be728681adcabed3d047ae72c48f1d"
-dependencies = [
- "ratatui-core",
- "ratatui-widgets",
-]
-
-[[package]]
-name = "ratatui-termwiz"
-version = "0.1.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "386b8ff8f74ed749509391c56d549761a2fcdb408e1f42e467286bcb7dac8967"
-dependencies = [
- "ratatui-core",
- "termwiz",
-]
-
-[[package]]
-name = "ratatui-widgets"
-version = "0.3.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "7ef4f17dd7ac3abf5adc2b920a03c61eee4bfe6a88fa5191936895525371d79c"
-dependencies = [
- "bitflags 2.12.1",
- "hashbrown 0.17.1",
- "indoc",
- "instability",
- "itertools 0.14.0",
- "line-clipping",
- "ratatui-core",
- "serde",
- "strum",
- "time",
- "unicode-segmentation",
- "unicode-width",
-]
-
 [[package]]
 name = "rayon"
 version = "1.12.0"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1200:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-42"></a>
### Cargo.lock `@@ -2038,15 +1220,6 @@ dependencies = [`

```diff
@@ -2038,15 +1220,6 @@ dependencies = [
  "crossbeam-utils",
 ]
 
-[[package]]
-name = "redox_syscall"
-version = "0.5.18"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "ed2bf2547551a7053d6fdfafda3f938979645c44812fbfcda098faae3f1a362d"
-dependencies = [
- "bitflags 2.12.1",
-]
-
 [[package]]
 name = "regex"
 version = "1.12.3"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1220:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-43"></a>
### Cargo.lock `@@ -2080,12 +1253,10 @@ checksum = "dc897dd8d9e8bd1ed8cdad82b5966c3e0ecae09fb1907d58efaa013543185d0a"`

```diff
@@ -2080,12 +1253,10 @@ checksum = "dc897dd8d9e8bd1ed8cdad82b5966c3e0ecae09fb1907d58efaa013543185d0a"
 name = "rumors"
 version = "0.1.0"
 dependencies = [
- "arc-swap",
  "async-stream",
  "before",
  "bytes",
  "ciborium",
- "clap",
  "criterion",
  "futures",
  "futures-util",
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1253:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-44"></a>
### Cargo.lock `@@ -2095,7 +1266,6 @@ dependencies = [`

```diff
@@ -2095,7 +1266,6 @@ dependencies = [
  "pollster",
  "proptest",
  "rand 0.8.6",
- "ratatui",
  "rumors",
  "seq-macro",
  "serde",
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1266:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-45"></a>
### Cargo.lock `@@ -2103,7 +1273,7 @@ dependencies = [`

```diff
@@ -2103,7 +1273,7 @@ dependencies = [
  "smallvec",
  "static_assertions",
  "stats_alloc",
- "thiserror 2.0.18",
+ "thiserror",
  "tinyvec",
  "tokio",
  "tokio-stream",
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1273:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-46"></a>
### Cargo.lock `@@ -2121,22 +1291,13 @@ dependencies = [`

```diff
@@ -2121,22 +1291,13 @@ dependencies = [
  "tracing",
 ]
 
-[[package]]
-name = "rustc_version"
-version = "0.4.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "cfcb3a22ef46e85b45de6ee7e79d063319ebb6594faafcf1c225ea92ab6e9b92"
-dependencies = [
- "semver",
-]
-
 [[package]]
 name = "rustix"
 version = "1.1.4"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "b6fe4565b9518b83ef4f91bb47ce29620ca828bd32cb7e408f0062e9930ba190"
 dependencies = [
- "bitflags 2.12.1",
+ "bitflags",
  "errno",
  "libc",
  "linux-raw-sys",
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1291:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-47"></a>
### Cargo.lock `@@ -2161,12 +1322,6 @@ dependencies = [`

```diff
@@ -2161,12 +1322,6 @@ dependencies = [
  "wait-timeout",
 ]
 
-[[package]]
-name = "ryu"
-version = "1.0.23"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "9774ba4a74de5f7b1c1451ed6cd5285a32eddb5cccb8cc655a4e50009e06477f"
-
 [[package]]
 name = "same-file"
 version = "1.0.6"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1322:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-48"></a>
### Cargo.lock `@@ -2176,12 +1331,6 @@ dependencies = [`

```diff
@@ -2176,12 +1331,6 @@ dependencies = [
  "winapi-util",
 ]
 
-[[package]]
-name = "scopeguard"
-version = "1.2.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "94143f37725109f92c262ed2cf5e59bce7498c01bcc1502d7b9afe439a4e9f49"
-
 [[package]]
 name = "semver"
 version = "1.0.28"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1331:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-49"></a>
### Cargo.lock `@@ -2227,7 +1376,7 @@ checksum = "d540f220d3187173da220f885ab66608367b6574e925011a9353e4badda91d79"`

```diff
@@ -2227,7 +1376,7 @@ checksum = "d540f220d3187173da220f885ab66608367b6574e925011a9353e4badda91d79"
 dependencies = [
  "proc-macro2",
  "quote",
- "syn 2.0.117",
+ "syn",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1376:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-50"></a>
### Cargo.lock `@@ -2243,24 +1392,13 @@ dependencies = [`

```diff
@@ -2243,24 +1392,13 @@ dependencies = [
  "zmij",
 ]
 
-[[package]]
-name = "sha2"
-version = "0.10.9"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "a7507d819769d01a365ab707794a4084392c824f54a7a6a7862f8c3d0892b283"
-dependencies = [
- "cfg-if",
- "cpufeatures 0.2.17",
- "digest 0.10.7",
-]
-
 [[package]]
 name = "sha3"
 version = "0.12.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "bc9bad02c26382724b2d2692c6f179285e4b54eeecd7968f52a50059c3c11759"
 dependencies = [
- "digest 0.11.3",
+ "digest",
  "keccak",
  "sponge-cursor",
 ]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1392:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-51"></a>
### Cargo.lock `@@ -2271,49 +1409,12 @@ version = "1.3.0"`

```diff
@@ -2271,49 +1409,12 @@ version = "1.3.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "0fda2ff0d084019ba4d7c6f371c95d8fd75ce3524c3cb8fb653a3023f6323e64"
 
-[[package]]
-name = "signal-hook"
-version = "0.3.18"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d881a16cf4426aa584979d30bd82cb33429027e42122b169753d6ef1085ed6e2"
-dependencies = [
- "libc",
- "signal-hook-registry",
-]
-
-[[package]]
-name = "signal-hook-mio"
-version = "0.2.5"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "b75a19a7a740b25bc7944bdee6172368f988763b744e3d4dfe753f6b4ece40cc"
-dependencies = [
- "libc",
- "mio",
- "signal-hook",
-]
-
-[[package]]
-name = "signal-hook-registry"
-version = "1.4.8"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "c4db69cba1110affc0e9f7bcd48bbf87b3f4fc7c61fc9155afd4c469eb3d6c1b"
-dependencies = [
- "errno",
- "libc",
-]
-
 [[package]]
 name = "similar"
 version = "2.7.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "bbbb5d9659141646ae647b42fe094daf6c6192d1620870b449d9557f748b2daa"
 
-[[package]]
-name = "siphasher"
-version = "1.0.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "8ee5873ec9cce0195efcb7a4e9507a04cd49aec9c83d0389df45b1ef7ba2e649"
-
 [[package]]
 name = "slab"
 version = "0.4.12"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1409:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-52"></a>
### Cargo.lock `@@ -2367,33 +1468,6 @@ version = "0.1.10"`

```diff
@@ -2367,33 +1468,6 @@ version = "0.1.10"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "5c0e04424e733e69714ca1bbb9204c1a57f09f5493439520f9f68c132ad25eec"
 
-[[package]]
-name = "strsim"
-version = "0.11.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "7da8b5736845d9f2fcb837ea5d9e2628564b3b043a70948a3f0b778838c5fb4f"
-
-[[package]]
-name = "strum"
-version = "0.28.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "9628de9b8791db39ceda2b119bbe13134770b56c138ec1d3af810d045c04f9bd"
-dependencies = [
- "strum_macros",
-]
-
-[[package]]
-name = "strum_macros"
-version = "0.28.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "ab85eea0270ee17587ed4156089e10b9e6880ee688791d45a905f5b1ca36f664"
-dependencies = [
- "heck",
- "proc-macro2",
- "quote",
- "syn 2.0.117",
-]
-
 [[package]]
 name = "suanpan"
 version = "0.1.0"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1468:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-53"></a>
### Cargo.lock `@@ -2407,17 +1481,6 @@ dependencies = [`

```diff
@@ -2407,17 +1481,6 @@ dependencies = [
 name = "surface-scan"
 version = "0.1.0"
 
-[[package]]
-name = "syn"
-version = "1.0.109"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "72b64191b275b66ffe2469e8af2c1cfe3bafa67b529ead792a6d0160888b4237"
-dependencies = [
- "proc-macro2",
- "quote",
- "unicode-ident",
-]
-
 [[package]]
 name = "syn"
 version = "2.0.117"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1481:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-54"></a>
### Cargo.lock `@@ -2448,96 +1511,13 @@ dependencies = [`

```diff
@@ -2448,96 +1511,13 @@ dependencies = [
  "windows-sys",
 ]
 
-[[package]]
-name = "terminfo"
-version = "0.9.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d4ea810f0692f9f51b382fff5893887bb4580f5fa246fde546e0b13e7fcee662"
-dependencies = [
- "fnv",
- "nom",
- "phf",
- "phf_codegen",
-]
-
-[[package]]
-name = "termios"
-version = "0.3.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "411c5bf740737c7918b8b1fe232dca4dc9f8e754b8ad5e20966814001ed0ac6b"
-dependencies = [
- "libc",
-]
-
-[[package]]
-name = "termwiz"
-version = "0.23.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "4676b37242ccbd1aabf56edb093a4827dc49086c0ffd764a5705899e0f35f8f7"
-dependencies = [
- "anyhow",
- "base64",
- "bitflags 2.12.1",
- "fancy-regex",
- "filedescriptor",
- "finl_unicode",
- "fixedbitset",
- "hex",
- "lazy_static",
- "libc",
- "log",
- "memmem",
- "nix",
- "num-derive",
- "num-traits",
- "ordered-float",
- "pest",
- "pest_derive",
- "phf",
- "sha2",
- "signal-hook",
- "siphasher",
- "terminfo",
- "termios",
- "thiserror 1.0.69",
- "ucd-trie",
- "unicode-segmentation",
- "vtparse",
- "wezterm-bidi",
- "wezterm-blob-leases",
- "wezterm-color-types",
- "wezterm-dynamic",
- "wezterm-input-types",
- "winapi",
-]
-
-[[package]]
-name = "thiserror"
-version = "1.0.69"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "b6aaf5339b578ea85b50e080feb250a3e8ae8cfcdff9a461c9ec2904bc923f52"
-dependencies = [
- "thiserror-impl 1.0.69",
-]
-
 [[package]]
 name = "thiserror"
 version = "2.0.18"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "4288b5bcbc7920c07a1149a35cf9590a2aa808e0bc1eafaade0b80947865fbc4"
 dependencies = [
- "thiserror-impl 2.0.18",
-]
-
-[[package]]
-name = "thiserror-impl"
-version = "1.0.69"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "4fee6c4efc90059e10f81e6d42c60a18f76588c3d74cb83a0b242a2b6c7504c1"
-dependencies = [
- "proc-macro2",
- "quote",
- "syn 2.0.117",
+ "thiserror-impl",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1511:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-55"></a>
### Cargo.lock `@@ -2548,30 +1528,9 @@ checksum = "ebc4ee7f67670e9b64d05fa4253e753e016c6c95ff35b89b7941d6b856dec1d5"`

```diff
@@ -2548,30 +1528,9 @@ checksum = "ebc4ee7f67670e9b64d05fa4253e753e016c6c95ff35b89b7941d6b856dec1d5"
 dependencies = [
  "proc-macro2",
  "quote",
- "syn 2.0.117",
-]
-
-[[package]]
-name = "time"
-version = "0.3.47"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "743bd48c283afc0388f9b8827b976905fb217ad9e647fae3a379a9283c4def2c"
-dependencies = [
- "deranged",
- "libc",
- "num-conv",
- "num_threads",
- "powerfmt",
- "serde_core",
- "time-core",
+ "syn",
 ]
 
-[[package]]
-name = "time-core"
-version = "0.1.8"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "7694e1cfe791f8d31026952abf09c69ca6f6fa4e1a1229e18988f06a04a12dca"
-
 [[package]]
 name = "tinytemplate"
 version = "1.2.1"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1528:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-56"></a>
### Cargo.lock `@@ -2620,7 +1579,7 @@ checksum = "385a6cb71ab9ab790c5fe8d67f1645e6c450a7ce006a33de03daa956cf70a496"`

```diff
@@ -2620,7 +1579,7 @@ checksum = "385a6cb71ab9ab790c5fe8d67f1645e6c450a7ce006a33de03daa956cf70a496"
 dependencies = [
  "proc-macro2",
  "quote",
- "syn 2.0.117",
+ "syn",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1579:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-57"></a>
### Cargo.lock `@@ -2653,7 +1612,7 @@ checksum = "7490cfa5ec963746568740651ac6781f701c9c5ea257c58e057f3ba8cf69e8da"`

```diff
@@ -2653,7 +1612,7 @@ checksum = "7490cfa5ec963746568740651ac6781f701c9c5ea257c58e057f3ba8cf69e8da"
 dependencies = [
  "proc-macro2",
  "quote",
- "syn 2.0.117",
+ "syn",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1612:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-58"></a>
### Cargo.lock `@@ -2671,12 +1630,6 @@ version = "1.20.1"`

```diff
@@ -2671,12 +1630,6 @@ version = "1.20.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "b6f5e870be6c3b371b77fe0ee0bafb859fa4964b4404c27de1d380043c4dda20"
 
-[[package]]
-name = "ucd-trie"
-version = "0.1.7"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "2896d95c02a80c6d6a5d6e953d479f5ddf2dfdb6a244441010e373ac0fb88971"
-
 [[package]]
 name = "unarray"
 version = "0.1.4"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1630:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-59"></a>
### Cargo.lock `@@ -2710,23 +1663,6 @@ dependencies = [`

```diff
@@ -2710,23 +1663,6 @@ dependencies = [
  "tinyvec",
 ]
 
-[[package]]
-name = "unicode-segmentation"
-version = "1.13.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "c6f5d3c3b1bf09027a88a6bc961fc00497d651009560b5463668dc81b0fa87a8"
-
-[[package]]
-name = "unicode-truncate"
-version = "2.0.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "16b380a1238663e5f8a691f9039c73e1cdae598a30e9855f541d29b08b53e9a5"
-dependencies = [
- "itertools 0.14.0",
- "unicode-segmentation",
- "unicode-width",
-]
-
 [[package]]
 name = "unicode-width"
 version = "0.2.2"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1663:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-60"></a>
### Cargo.lock `@@ -2762,38 +1698,11 @@ version = "1.0.4"`

```diff
@@ -2762,38 +1698,11 @@ version = "1.0.4"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "b6c140620e7ffbb22c2dee59cafe6084a59b5ffc27a8859a5f0d494b5d52b6be"
 
-[[package]]
-name = "utf8parse"
-version = "0.2.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "06abde3611657adf66d383f00b093d7faecc7fa57071cce2578660c9f1010821"
-
 [[package]]
 name = "uuid"
 version = "1.23.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "d258b83ceec21034727ecee8c382cfa6c3e133699b0742c64571814fb420c9f7"
-dependencies = [
- "atomic",
- "getrandom 0.4.2",
- "js-sys",
- "wasm-bindgen",
-]
-
-[[package]]
-name = "version_check"
-version = "0.9.5"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "0b928f33d975fc6ad9f86c8f283853ad26bdd5b10b7f1542aa2fa15e2289105a"
-
-[[package]]
-name = "vtparse"
-version = "0.6.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "6d9b2acfb050df409c972a37d3b8e08cdea3bddb0c09db9d53137e504cfabed0"
-dependencies = [
- "utf8parse",
-]
 
 [[package]]
 name = "wait-timeout"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1698:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-61"></a>
### Cargo.lock `@@ -2870,7 +1779,7 @@ dependencies = [`

```diff
@@ -2870,7 +1779,7 @@ dependencies = [
  "bumpalo",
  "proc-macro2",
  "quote",
- "syn 2.0.117",
+ "syn",
  "wasm-bindgen-shared",
 ]
 
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1779:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-62"></a>
### Cargo.lock `@@ -2911,7 +1820,7 @@ version = "0.244.0"`

```diff
@@ -2911,7 +1820,7 @@ version = "0.244.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "47b807c72e1bac69382b3a6fb3dbe8ea4c0ed87ff5629b8685ae6b9a611028fe"
 dependencies = [
- "bitflags 2.12.1",
+ "bitflags",
  "hashbrown 0.15.5",
  "indexmap",
  "semver",
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1820:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-63"></a>
### Cargo.lock `@@ -2937,94 +1846,6 @@ dependencies = [`

```diff
@@ -2937,94 +1846,6 @@ dependencies = [
  "wasm-bindgen",
 ]
 
-[[package]]
-name = "wezterm-bidi"
-version = "0.2.3"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "0c0a6e355560527dd2d1cf7890652f4f09bb3433b6aadade4c9b5ed76de5f3ec"
-dependencies = [
- "log",
- "wezterm-dynamic",
-]
-
-[[package]]
-name = "wezterm-blob-leases"
-version = "0.1.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "692daff6d93d94e29e4114544ef6d5c942a7ed998b37abdc19b17136ea428eb7"
-dependencies = [
- "getrandom 0.3.4",
- "mac_address",
- "sha2",
- "thiserror 1.0.69",
- "uuid",
-]
-
-[[package]]
-name = "wezterm-color-types"
-version = "0.3.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "7de81ef35c9010270d63772bebef2f2d6d1f2d20a983d27505ac850b8c4b4296"
-dependencies = [
- "csscolorparser",
- "deltae",
- "lazy_static",
- "wezterm-dynamic",
-]
-
-[[package]]
-name = "wezterm-dynamic"
-version = "0.2.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "5f2ab60e120fd6eaa68d9567f3226e876684639d22a4219b313ff69ec0ccd5ac"
-dependencies = [
- "log",
- "ordered-float",
- "strsim",
- "thiserror 1.0.69",
- "wezterm-dynamic-derive",
-]
-
-[[package]]
-name = "wezterm-dynamic-derive"
-version = "0.1.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "46c0cf2d539c645b448eaffec9ec494b8b19bd5077d9e58cb1ae7efece8d575b"
-dependencies = [
- "proc-macro2",
- "quote",
- "syn 1.0.109",
-]
-
-[[package]]
-name = "wezterm-input-types"
-version = "0.1.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "7012add459f951456ec9d6c7e6fc340b1ce15d6fc9629f8c42853412c029e57e"
-dependencies = [
- "bitflags 1.3.2",
- "euclid",
- "lazy_static",
- "serde",
- "wezterm-dynamic",
-]
-
-[[package]]
-name = "winapi"
-version = "0.3.9"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "5c839a674fcd7a98952e593242ea400abe93992746761e38641405d28b00f419"
-dependencies = [
- "winapi-i686-pc-windows-gnu",
- "winapi-x86_64-pc-windows-gnu",
-]
-
-[[package]]
-name = "winapi-i686-pc-windows-gnu"
-version = "0.4.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "ac3b87c63620426dd9b991e5ce0329eff545bccbbb34f3be09ff6fb6ab51b7b6"
-
 [[package]]
 name = "winapi-util"
 version = "0.1.11"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1846:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-64"></a>
### Cargo.lock `@@ -3034,12 +1855,6 @@ dependencies = [`

```diff
@@ -3034,12 +1855,6 @@ dependencies = [
  "windows-sys",
 ]
 
-[[package]]
-name = "winapi-x86_64-pc-windows-gnu"
-version = "0.4.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "712e227841d057c1ee1cd2fb22fa7e5a5461ae8e48fa2ca79ec42cfc1931183f"
-
 [[package]]
 name = "windows-link"
 version = "0.2.1"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1855:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-65"></a>
### Cargo.lock `@@ -3091,7 +1906,7 @@ dependencies = [`

```diff
@@ -3091,7 +1906,7 @@ dependencies = [
  "heck",
  "indexmap",
  "prettyplease",
- "syn 2.0.117",
+ "syn",
  "wasm-metadata",
  "wit-bindgen-core",
  "wit-component",
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1906:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-66"></a>
### Cargo.lock `@@ -3107,7 +1922,7 @@ dependencies = [`

```diff
@@ -3107,7 +1922,7 @@ dependencies = [
  "prettyplease",
  "proc-macro2",
  "quote",
- "syn 2.0.117",
+ "syn",
  "wit-bindgen-core",
  "wit-bindgen-rust",
 ]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1922:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-67"></a>
### Cargo.lock `@@ -3119,7 +1934,7 @@ source = "registry+https://github.com/rust-lang/crates.io-index"`

```diff
@@ -3119,7 +1934,7 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "9d66ea20e9553b30172b5e831994e35fbde2d165325bec84fc43dbf6f4eb9cb2"
 dependencies = [
  "anyhow",
- "bitflags 2.12.1",
+ "bitflags",
  "indexmap",
  "log",
  "serde",
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1934:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-68"></a>
### Cargo.lock `@@ -3175,7 +1990,7 @@ checksum = "8fd425244944f4ab65ccff928e7323354c5a018c75838362fdce749dfad2ee1e"`

```diff
@@ -3175,7 +1990,7 @@ checksum = "8fd425244944f4ab65ccff928e7323354c5a018c75838362fdce749dfad2ee1e"
 dependencies = [
  "proc-macro2",
  "quote",
- "syn 2.0.117",
+ "syn",
 ]
 
 [[package]]
```

<!-- annotation -->
> **swarm-example-*** (T27), line 1990:
>
> Cargo.lock: the same `cargo update --workspace` regeneration as the first Cargo.lock row; dropped packages and un-suffixed names only, no version bumps.

<a id="hunk-69"></a>
### Cargo.toml `@@ -15,7 +15,6 @@ members = [`

```diff
@@ -15,7 +15,6 @@ members = [
 # a version, path, or other source of its own; the gate's manifestlint leg
 # holds every member manifest to this.
 [workspace.dependencies]
-arc-swap = "1"
 async-stream = "0.3"
 base64 = "0.22"
 before = { path = "crates/before" }
```

<!-- annotation -->
> **swarm-example-*** (T27), line 18:
>
> Same judgment call as the [dev-dependencies] rows: [workspace.dependencies] is the version of record, and with the member declaration removed this entry had no inheritor anywhere (grep of every member manifest and the detached workspaces). Removing it keeps the table's own premise true: every entry has a member that inherits it. This line: `arc-swap`.

<a id="hunk-70"></a>
### Cargo.toml `@@ -44,7 +43,6 @@ cbor-diag = { git = "https://github.com/oxidecomputer/cbor-diag-rs", branch = "d`

```diff
@@ -44,7 +43,6 @@ cbor-diag = { git = "https://github.com/oxidecomputer/cbor-diag-rs", branch = "d
 # the handshake; and a verdict-strictening bump additionally requires
 # re-validating stored payloads before gossiping.
 ciborium = "=0.2.2"
-clap = { version = "4", features = ["derive"] }
 console_error_panic_hook = "0.1"
 criterion = "0.5"
 dashu-int = { version = "0.5", default-features = false, features = ["std"] }
```

<!-- annotation -->
> **swarm-example-*** (T27), line 46:
>
> Same judgment call as the [dev-dependencies] rows: [workspace.dependencies] is the version of record, and with the member declaration removed this entry had no inheritor anywhere (grep of every member manifest and the detached workspaces). Removing it keeps the table's own premise true: every entry has a member that inherits it. This line: `clap`.

<a id="hunk-71"></a>
### Cargo.toml `@@ -61,7 +59,6 @@ postcard = { version = "1", default-features = false, features = ["use-std"] }`

```diff
@@ -61,7 +59,6 @@ postcard = { version = "1", default-features = false, features = ["use-std"] }
 proptest = "1"
 rand = "0.8"
 rand_chacha = "0.3"
-ratatui = "0.30"
 rayon = "1"
 rumors = { path = "." }
 seq-macro = "0.3"
```

<!-- annotation -->
> **swarm-example-*** (T27), line 62:
>
> Same judgment call as the [dev-dependencies] rows: [workspace.dependencies] is the version of record, and with the member declaration removed this entry had no inheritor anywhere (grep of every member manifest and the detached workspaces). Removing it keeps the table's own premise true: every entry has a member that inherits it. This line: `ratatui`.

<a id="hunk-72"></a>
### Cargo.toml `@@ -161,11 +158,8 @@ tokio = { workspace = true, default-features = true, features = [`

```diff
@@ -161,11 +158,8 @@ tokio = { workspace = true, default-features = true, features = [
     # delay to virtual time so sweeps cost wall-clock compute only.
     "test-util",
 ] }
-clap = { workspace = true }
 rand = { workspace = true, features = ["small_rng"] }
-arc-swap = { workspace = true }
 pollster = { workspace = true }
-ratatui = { workspace = true }
 # Counting global allocator for the decoder allocation meter
 # (`tests/decode_alloc.rs`): prices decodes in bytes requested from the
 # allocator, so a declared-length pre-allocation is observable.
```

<!-- annotation -->
> **swarm-example-*** (T27), line 161:
>
> Judgment call beyond the brief's letter: `clap`, `arc-swap`, and `ratatui` were declared in [dev-dependencies] for this example alone (verified by whole-word grep of every other target: benches, tests, the other two examples, src, every crate under crates/; before-fuelscape pins its own `clap` in a detached workspace). A dev-dependency that exists only to build deleted code is part of the example's footprint, so it goes: leaving it would keep a terminal-UI stack in the lock for nothing. `rand`'s `small_rng` feature and `pollster` stay: two benches and envelope_sim use SmallRng, and the conformance and bench suites use pollster. This line: `clap`.

<!-- annotation -->
> **swarm-example-*** (T27), line 162:
>
> Judgment call beyond the brief's letter: `clap`, `arc-swap`, and `ratatui` were declared in [dev-dependencies] for this example alone (verified by whole-word grep of every other target: benches, tests, the other two examples, src, every crate under crates/; before-fuelscape pins its own `clap` in a detached workspace). A dev-dependency that exists only to build deleted code is part of the example's footprint, so it goes: leaving it would keep a terminal-UI stack in the lock for nothing. `rand`'s `small_rng` feature and `pollster` stay: two benches and envelope_sim use SmallRng, and the conformance and bench suites use pollster. This line: `arc-swap`.

<!-- annotation -->
> **swarm-example-*** (T27), line 163:
>
> Judgment call beyond the brief's letter: `clap`, `arc-swap`, and `ratatui` were declared in [dev-dependencies] for this example alone (verified by whole-word grep of every other target: benches, tests, the other two examples, src, every crate under crates/; before-fuelscape pins its own `clap` in a detached workspace). A dev-dependency that exists only to build deleted code is part of the example's footprint, so it goes: leaving it would keep a terminal-UI stack in the lock for nothing. `rand`'s `small_rng` feature and `pollster` stay: two benches and envelope_sim use SmallRng, and the conformance and bench suites use pollster. This line: `ratatui`.

<a id="hunk-73"></a>
### Cargo.toml `@@ -191,12 +185,6 @@ opt-level = 2`

```diff
@@ -191,12 +185,6 @@ opt-level = 2
 [profile.bench]
 debug = "line-tables-only"
 
-[[example]]
-name = "swarm"
-path = "examples/swarm.rs"
-# The steady-state controller's convergence tests live in the example.
-test = true
-
 [[bench]]
 name = "branch_hash"
 harness = false
```

<!-- annotation -->
> **swarm-example-*** (T27), line 188:
>
> The [[example]] block: with the files gone this is the last pointer at them, and its comment named the controller tests that no longer exist. The block's trailing blank line goes with it so [profile.bench] and [[bench]] keep one blank line between them.

<a id="hunk-74"></a>
### examples/swarm.rs `@@ -1,1800 +0,0 @@`

```diff
@@ -1,1800 +0,0 @@
-//! An interactive gossip swarm: a virtual network of parties churning messages
-//! and reconciling over the real wire protocol, steered live from a terminal
-//! UI.
-//!
-//! # What it does
-//!
-//! `seed_messages` random `Vec<u8>` payloads are inserted into one seed
-//! [`Rumors`] *before any fork*, so every party starts from the same shared
-//! observations. The seed is then forked (via `bootstrap_fork`) into `parties`
-//! disjoint peers, one per OS thread, and the party count is itself tunable
-//! live (see *Dynamic membership* below). Each party thread runs a tight loop:
-//!
-//! 1. **Serve** any inbound sync requests waiting in its inbox (it is the
-//!    *responder* for a peer that chose it).
-//! 2. If its Poisson sync timer has fired, **initiate** a sync with a random
-//!    other party (it is the *initiator*).
-//! 3. Otherwise, run the **steady-state controller**: compare the number of
-//!    messages it currently knows about to the target and, with a probability
-//!    derived from that gap, either inject a fresh random message or redact a
-//!    message it already knows about.
-//!
-//! Each party keeps its *own* `Vec<Version>` of every message it has
-//! observed — fed by an [`UnorderedMessages`] observer that replays its
-//! rumor set from genesis and then yields its own inserts and everything
-//! learned over the wire alike — so redactions may evict messages
-//! originally published by *other* parties, and the contagion spreads on
-//! the next sync. The version vector is per-thread: no shared rumor-set
-//! state, no lock contention on the hot path.
-//!
-//! # Steady-state controller
-//!
-//! Left to a fair coin, the live-message count random-walks. Instead each node
-//! sets its probability of *adding* (versus redacting) from its current live
-//! count `L` against the target `T`:
-//!
-//! ```text
-//! p_add = T / (T + L)
-//! ```
-//!
-//! At `L = 0` it always adds; at `L = T` the odds are even; as `L` grows past
-//! `T` adding becomes rare. The fixed point is `L = T`, so each node's live
-//! count is driven toward the target, tunable live from the UI. A redact op
-//! always removes a message that is still live, discarding pool entries
-//! other parties already redacted as they surface; [`steady_state_op`] explains
-//! why the fixed point depends on that, and `swarm/tests.rs` pins
-//! convergence through retargeting. Because redactions and inserts both
-//! propagate, every node's `L` tracks the global live count as views
-//! converge; while churn and syncs race each other the network rides above
-//! the target by the messages still in flight between views.
-//!
-//! # Synchronization uses the wire protocol
-//!
-//! Syncs go through [`Rumors::gossip`] over an in-memory [`rumors::link`] pair
-//! — the *same* bytes-on-the-wire path a QUIC peer would drive. Both ends of a
-//! session run `gossip` concurrently on their own thread's current-thread
-//! runtime, exactly as two networked peers would.
-//!
-//! # Rendezvous without deadlock
-//!
-//! Two parties that pick each other at the same instant must not both block
-//! waiting for the other to respond. A single [`AtomicBool`] per party — its
-//! *engaged* flag — makes each party a participant in at most one session at a
-//! time. An initiator claims itself and its peer with compare-and-swap before
-//! sending; a party that is already engaged (claimed as a responder, or
-//! initiating elsewhere) cannot be claimed, so the initiator simply backs off
-//! and does local work instead. No party is ever simultaneously a blocked
-//! initiator *and* an owed responder, so the wait-for graph has no cycle.
-//!
-//! # Dynamic membership
-//!
-//! The party count is live-tunable, and growing or shrinking it exercises the
-//! bootstrap/[`retire`](Peer::retire) algebra directly. A single coordinator
-//! thread — the only writer of the peer directory — watches the desired count
-//! against the live one and reconciles a step at a time:
-//!
-//! - **Grow:** pick a random live party, hand it a *fork* command. It creates a
-//!   disjoint child of its own [`Rumors`] (via `bootstrap_fork`), ships the child
-//!   back to the coordinator, and keeps running; the coordinator spawns a fresh
-//!   thread for the child. The parent and child are disjoint sub-parties, so the
-//!   directory stays a partition of the seed's party space.
-//! - **Shrink:** pick two random parties, hand each a *wind-down* command.
-//!   Each finishes any owed session, locks itself out of new ones, ships its
-//!   [`Rumors`] back, and exits. The coordinator [`retire`](Peer::retire)s
-//!   one into the other over an in-memory wire — the session's gossip round
-//!   carries any divergent content across, and the survivor absorbs the
-//!   retiree's party region, so the id-space is reclaimed rather than leaked
-//!   — and starts the survivor in a new thread, for a net loss of one.
-//!
-//! Because every live party is a disjoint fork of the common seed, any two can
-//! always reconcile, so shrink never fails. The directory itself is an
-//! [`ArcSwap`], so the sync hot path reads it without locking; only the
-//! coordinator ever swaps it, one membership change at a time. The floor is two
-//! parties — there is no one to gossip with below that.
-//!
-//! # The readout
-//!
-//! The UI reports, as windowed rates over each refresh interval:
-//!
-//! - **local ops/s** — inserts + redactions, per party and in aggregate;
-//! - **wire bandwidth** — bytes/s per direction, averaged across every wire
-//!   and both directions, auto-scaled to B/KiB/MiB/…;
-//! - **sync latency** — mean wall-clock duration of one end-to-end gossip
-//!   session, measured by the initiator from the instant the exchange begins
-//!   to the instant it returns (never derived from the Poisson schedule).
-//!   Wall clock includes every OS scheduling delay between protocol hops,
-//!   and those dominate on a loaded machine, so the readout pairs the mean
-//!   with the window's *best* session — a floor for what the protocol
-//!   itself costs;
-//! - **roundtrips/sync** — mean number of request→response turns per session,
-//!   counted from write→read direction flips on the initiator's I/O;
-//! - **live messages/node** — the current rumor-set size, charted against the
-//!   target so you can watch the controller converge.
-//!
-//! # Controls
-//!
-//! `↑`/`↓` select a parameter, `←`/`→` adjust it (`Shift` for a coarse step),
-//! `space` pauses all churn, `q` quits.
-//!
-//! # Headless mode
-//!
-//! `--headless-secs N` runs the same swarm without the UI for `N` seconds
-//! and prints the readout's statistics to stdout — the scripted way to
-//! measure the swarm itself.
-
-use std::collections::VecDeque;
-use std::io;
-use std::pin::Pin;
-use std::sync::Arc;
-use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
-use std::sync::mpsc::{Receiver, Sender, channel};
-use std::task::{Context, Poll};
-use std::thread::{self, JoinHandle};
-use std::time::{Duration, Instant};
-
-use arc_swap::ArcSwap;
-use clap::Parser;
-use futures::{FutureExt, StreamExt};
-use rand::rngs::SmallRng;
-use rand::{Rng, RngCore, SeedableRng};
-use ratatui::Terminal;
-use ratatui::backend::CrosstermBackend;
-use ratatui::crossterm::{
-    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
-    execute,
-    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
-};
-use ratatui::layout::{Constraint, Layout, Rect};
-use ratatui::style::{Color, Modifier, Style};
-use ratatui::text::{Line, Span};
-use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};
-use rumors::link::{Connector, Done, Link, LinkParts, MemoryAcceptor, MemoryConnector, MemoryLink};
-use rumors::{Peer, Retire, Rumors, UnorderedMessages, Version};
-use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};
-
-/// Create a genuine party-disjoint peer that inherits `parent`'s content.
-///
-/// Every party in the swarm — which independently `send`s, `redact`s, and
-/// `gossip`s — needs its own disjoint Interval Tree Clock region. We create one
-/// by serving a bootstrap from `parent` over an in-memory link: the
-/// newcomer pulls `parent`'s whole tree through the ordinary mirror descent
-/// and is handed a fresh disjoint party, forked in the same critical section
-/// that snapshots the served tree. Both halves run concurrently on
-/// `runtime`'s single current thread via [`tokio::join!`]. `duplex_capacity`
-/// sizes each of the link's in-memory streams, same as every other link in
-/// the swarm.
-fn bootstrap_fork(
-    runtime: &tokio::runtime::Runtime,
-    parent: &Rumors<Payload>,
-    duplex_capacity: usize,
-) -> Rumors<Payload> {
-    let (mut parent_link, mut newcomer_link) = rumors::link::memory_with_capacity(duplex_capacity);
-    let (served, newcomer) = runtime.block_on(async {
-        tokio::join!(
-            parent.gossip(&mut parent_link),
-            Peer::<Payload>::bootstrap().join(&mut newcomer_link),
-        )
-    });
-    served.expect("serve bootstrap");
-    newcomer
-        .expect("bootstrap newcomer")
-        .expect("provider served bootstrap")
-        .into_rumors()
-}
-
-/// Message payload type: opaque, randomized bytes. CBOR encodes `Vec<u8>`
-/// as an integer array, so the wire cost tracks the message size (within
-/// a small per-element constant).
-type Payload = Vec<u8>;
-
-/// One endpoint of a sync session, handed from an initiator to the responder
-/// it claimed. The responder decorates it for byte accounting and drives
-/// `gossip` on its own thread.
-type SessionEnd = MemoryLink;
-
-/// An interactive gossip swarm with a live throughput readout.
-#[derive(Parser, Debug)]
-#[command(about, long_about = None)]
-struct Args {
-    /// Initial number of parties (one OS thread each). Must be at least 2.
-    /// Adjustable live: growing forks a new party, shrinking joins two.
-    #[arg(long, default_value_t = 8)]
-    parties: usize,
-
-    /// Initial expected seconds between a party's syncs. Inter-sync gaps are
-    /// drawn from an exponential distribution with this mean, so syncs form a
-    /// Poisson process per party. Adjustable live.
-    #[arg(long, default_value_t = 0.25)]
-    sync_interval: f64,
-
-    /// Messages inserted into the shared seed before the first fork.
-    #[arg(long, default_value_t = 100)]
-    seed_messages: usize,
-
-    /// Initial steady-state target for the network's live message count.
-    /// Adjustable live.
-    #[arg(long, default_value_t = 100)]
-    target: u64,
-
-    /// Initial size in bytes of each randomized message payload. Adjustable
-    /// live.
-    #[arg(long, default_value_t = 256)]
-    message_size: usize,
-
-    /// Capacity in bytes of each in-memory link stream (control and every data
-    /// stream alike). Smaller values exercise more backpressure; larger values
-    /// fewer roundtrips.
-    #[arg(long, default_value_t = 16 * 1024)]
-    duplex_capacity: usize,
-
-    /// UI refresh interval in milliseconds.
-    #[arg(long, default_value_t = 200)]
-    refresh_ms: u64,
-
-    /// Run without the UI for this many seconds, then print the readout's
-    /// windowed statistics to stdout and exit. Zero runs the interactive UI.
-    /// Useful for scripted measurements of the swarm itself.
-    #[arg(long, default_value_t = 0)]
-    headless_secs: u64,
-}
-
-/// Live-tunable knobs shared with every party thread. Read on the hot path, so
-/// every field is a lock-free atomic.
-struct Controls {
-    /// Mean microseconds between a party's syncs (exponential inter-arrival).
-    sync_interval_us: AtomicU64,
-    /// Per-node target live-message count for the steady-state controller.
-    target: AtomicU64,
-    /// Size in bytes of freshly injected messages.
-    message_size: AtomicU64,
-    /// Desired number of parties. The coordinator reconciles the live party
-    /// count toward this by forking (to grow) or joining (to shrink).
-    parties: AtomicU64,
-    /// While true, parties stop churning and initiating (but still serve
-    /// in-flight syncs).
-    paused: AtomicBool,
-}
-
-/// Process-wide counters, sampled by the UI. All monotonic since start except
-/// `sync_nanos_best`; the UI differences successive snapshots to get windowed
-/// rates.
-struct Metrics {
-    /// Local inserts + redactions completed across all parties.
-    local_ops: AtomicU64,
-    /// Total bytes written to every wire — control stream and every data
-    /// stream, both directions (each byte counted once, on the writer that
-    /// produced it).
-    ///
-    /// Shared into the link-decorating writers, which outlive
-    /// the borrow of `Metrics`, so it is an [`Arc`] rather than a bare field.
-    wire_bytes: Arc<AtomicU64>,
-    /// Sum over completed sessions of `2 * duration_nanos`: one direction-span
-    /// per direction. Pairs with `wire_bytes` to give a time-weighted mean
-    /// per-direction bandwidth.
-    wire_direction_nanos: AtomicU64,
-    /// Number of completed sync sessions (counted once each, by the initiator).
-    syncs: AtomicU64,
-    /// Sum of session wall-clock durations in nanos (end-to-end latency).
-    sync_nanos: AtomicU64,
-    /// Fastest session in the current sampling window, in nanos —
-    /// `u64::MAX` when the window has seen none.
-    ///
-    /// The sampler swaps the
-    /// sentinel back in each time it reads, so the value is windowed where
-    /// every other counter is monotonic. Wall-clock means include every
-    /// OS scheduling delay a loaded machine inserts between protocol
-    /// hops; the window's fastest session is a floor for what the
-    /// protocol itself costs, so the readout shows both.
-    sync_nanos_best: AtomicU64,
-    /// Sum of request→response roundtrips across all sessions.
-    roundtrips: AtomicU64,
-    /// Sessions still in flight. Drains to zero during shutdown before any
-    /// thread is allowed to exit, so no party is left blocked on a peer that
-    /// already quit.
-    inflight: AtomicU64,
-}
-
-impl Default for Metrics {
-    fn default() -> Self {
-        Metrics {
-            local_ops: AtomicU64::new(0),
-            wire_bytes: Arc::new(AtomicU64::new(0)),
-            wire_direction_nanos: AtomicU64::new(0),
-            syncs: AtomicU64::new(0),
-            sync_nanos: AtomicU64::new(0),
-            // The windowed minimum starts at its no-sessions sentinel.
-            sync_nanos_best: AtomicU64::new(u64::MAX),
-            roundtrips: AtomicU64::new(0),
-            inflight: AtomicU64::new(0),
-        }
-    }
-}
-
-impl Metrics {
-    /// Record one completed session from the initiator's vantage: its
-    /// wall-clock `duration` and the `roundtrips` observed on its I/O.
-    fn record_sync(&self, duration: Duration, roundtrips: u64) {
-        let nanos = duration.as_nanos() as u64;
-        self.syncs.fetch_add(1, Ordering::Relaxed);
-        self.sync_nanos.fetch_add(nanos, Ordering::Relaxed);
-        self.sync_nanos_best.fetch_min(nanos, Ordering::Relaxed);
-        // Two directions, each spanning the whole session.
-        self.wire_direction_nanos
-            .fetch_add(nanos.saturating_mul(2), Ordering::Relaxed);
-        self.roundtrips.fetch_add(roundtrips, Ordering::Relaxed);
-    }
-}
-
-/// Holds one in-flight session slot; releases it on drop.
-///
-/// Shutdown drains
-/// the in-flight counter to zero before letting threads exit, so the slot
-/// must be released on every exit from a session — including a panic
-/// unwinding out of `gossip` — or shutdown spins forever on a slot no one
-/// holds. RAII makes the release unconditional.
-struct InflightGuard<'a>(&'a Metrics);
-
-impl<'a> InflightGuard<'a> {
-    /// Reserve a slot. Callers reserve *before* their final `running` check:
-    /// shutdown first clears `running`, then waits for this counter, so the
-    /// ordering guarantees no session slips between the check and the drain.
-    fn reserve(metrics: &'a Metrics) -> Self {
-        metrics.inflight.fetch_add(1, Ordering::SeqCst);
-        InflightGuard(metrics)
-    }
-}
-
-impl Drop for InflightGuard<'_> {
-    fn drop(&mut self) {
-        self.0.inflight.fetch_sub(1, Ordering::SeqCst);
-    }
-}
-
-/// Per-party coordination state, shared across all threads.
-///
-/// Holds no rumor-set
-/// data — only the inbox to deliver session endpoints, the engaged flag that
-/// serializes each party into one session at a time, and a gauge of its
-/// current live-message count for the UI.
-struct SwarmPeer {
-    /// Stable identity for this party, unique across the whole run (never
-    /// reused, even after a party retires). Used to exclude self when picking a
-    /// peer and to splice the directory on membership changes.
-    id: u64,
-    /// Set while this party is a participant in a session (initiator or
-    /// responder). A party can be claimed only when this is `false`. A retiring
-    /// party also sets it, permanently, to lock out new claims before it exits.
-    engaged: AtomicBool,
-    /// Inbound session endpoints, pushed by initiators that claimed this party.
-    inbox: Sender<SessionEnd>,
-    /// Membership commands from the coordinator (fork to grow, retire to
-    /// shrink). Served at the top of the party loop, between sessions.
-    control: Sender<Command>,
-    /// This party's current live-message count, republished every loop.
-    live: AtomicU64,
-}
-
-/// A membership command sent by the coordinator to a single party. Each carries
-/// a one-shot reply channel for the party to hand its [`Rumors`] back.
-enum Command {
-    /// Fork off a child party and reply with it; the recipient keeps running.
-    Fork { reply: Sender<Donation> },
-    /// Finish any owed session, lock out new ones, reply with this party's own
-    /// state, and exit the thread.
-    WindDown { reply: Sender<Donation> },
-}
-
-/// A party's [`Rumors`] handed back to the coordinator. The version pool
-/// is not carried along: the receiving thread rebuilds it by replaying the
-/// set through a fresh [`UnorderedMessages`] observer.
-struct Donation {
-    rumors: Rumors<Payload>,
-}
-
-/// Everything the party threads share: the peer directory, live controls,
-/// run/stop flags, and the metrics counters.
-struct Net {
-    /// The live peer directory.
-    ///
-    /// Read locklessly on the sync hot path; swapped
-    /// only by the coordinator, one membership change at a time. Each entry is
-    /// an `Arc` so a party claimed for a session survives its removal from the
-    /// directory.
-    peers: ArcSwap<Vec<Arc<SwarmPeer>>>,
-    controls: Controls,
-    metrics: Metrics,
-    /// Capacity of each in-memory link stream. Immutable for the run.
-    duplex_capacity: usize,
-    /// While true, parties may initiate new syncs. Cleared first at shutdown.
-    running: AtomicBool,
-    /// While false, parties keep looping; set once all in-flight sessions have
-    /// drained, after which every thread breaks.
-    shutdown: AtomicBool,
-}
-
-fn main() -> io::Result<()> {
-    let args = Args::parse();
-    assert!(args.parties >= 2, "need at least 2 parties to gossip");
-    assert!(args.message_size > 0, "message size must be positive");
-
-    // Seed the shared rumor set, then fork one disjoint party per thread.
-    // The seed's messages are shared by every party, so any party may redact
-    // them (each thread learns them by replaying its set through an
-    // observer).
-    let seed_runtime = tokio::runtime::Builder::new_current_thread()
-        .build()
-        .expect("build seed runtime");
-    let seed: Rumors<Payload> = Peer::seed().into_rumors();
-    {
-        let mut rng = SmallRng::from_entropy();
-        seed.send_all((0..args.seed_messages).map(|_| random_message(&mut rng, args.message_size)))
-            .expect("flat test payloads are within any depth limit");
-    }
-    // Every party starts as a disjoint fork of the seed: same observations,
-    // its own party region. The seed party itself only serves the initial
-    // bootstraps; its forks do all the gossiping.
-    let initial: Vec<Donation> = (0..args.parties)
-        .map(|_| Donation {
-            rumors: bootstrap_fork(&seed_runtime, &seed, args.duplex_capacity),
-        })
-        .collect();
-    drop(seed);
-
-    // The directory starts empty; the coordinator populates it as it launches
-    // the initial parties, then keeps it reconciled with the desired count.
-    let net = Arc::new(Net {
-        peers: ArcSwap::from_pointee(Vec::new()),
-        controls: Controls {
-            sync_interval_us: AtomicU64::new((args.sync_interval * 1e6) as u64),
-            target: AtomicU64::new(args.target),
-            message_size: AtomicU64::new(args.message_size as u64),
-            parties: AtomicU64::new(args.parties as u64),
-            paused: AtomicBool::new(false),
-        },
-        metrics: Metrics::default(),
-        duplex_capacity: args.duplex_capacity,
-        running: AtomicBool::new(true),
-        shutdown: AtomicBool::new(false),
-    });
-
-    // The coordinator owns every party thread's lifecycle: it launches the
-    // initial set, forks/joins to track the desired count, and hands back all
-    // outstanding join handles once `running` is cleared.
-    let coordinator = {
-        let net = Arc::clone(&net);
-        thread::Builder::new()
-            .name("coordinator".to_string())
-            .spawn(move || run_coordinator(net, initial))
-            .expect("spawn coordinator thread")
-    };
-
-    // Run the interactive UI on the main thread until the user quits (or, in
-    // headless mode, sample the same statistics for the requested duration).
-    // The UI always restores the terminal, even on error or panic.
-    let result = if args.headless_secs > 0 {
-        run_headless(&net, &args);
-        Ok(())
-    } else {
-        run_ui(&net, &args)
-    };
-
-    // Shutdown: stop new syncs and membership changes. Clearing `running`
-    // ends the coordinator's reconcile loop; it hands back every party's join
-    // handle. Then wait for in-flight sessions to drain so no party is blocked
-    // on a peer that already quit, and finally release the threads. The drain
-    // is bounded: in-flight slots release even across panics (see
-    // [`InflightGuard`]), so a drain that outlives the deadline means a
-    // session is wedged, and joining its thread would hang forever — report
-    // and let process exit reap the threads instead.
-    net.running.store(false, Ordering::SeqCst);
-    let handles = coordinator.join().expect("join coordinator thread");
-    let drain_deadline = Instant::now() + SHUTDOWN_DRAIN_DEADLINE;
-    while net.metrics.inflight.load(Ordering::SeqCst) > 0 {
-        if Instant::now() >= drain_deadline {
-            eprintln!(
-                "shutdown: {} session(s) still in flight after {:?}: \
-                 exiting without joining party threads",
-                net.metrics.inflight.load(Ordering::SeqCst),
-                SHUTDOWN_DRAIN_DEADLINE,
-            );
-            return result;
-        }
-        thread::sleep(Duration::from_millis(1));
-    }
-    net.shutdown.store(true, Ordering::SeqCst);
-    for handle in handles {
-        handle.join().expect("join party thread");
-    }
-
-    result
-}
-
-/// How long shutdown waits for in-flight sessions to drain before concluding
-/// one is wedged and exiting without the party-thread joins.
-///
-/// In-memory
-/// sessions complete in milliseconds even on a heavily loaded machine, so
-/// five seconds is generous.
-const SHUTDOWN_DRAIN_DEADLINE: Duration = Duration::from_secs(5);
-
-// --- party engine ----------------------------------------------------------
-
-/// One party's main loop: serve inbound syncs, obey membership commands, fire
-/// scheduled syncs, and otherwise churn the local rumor set under the
-/// steady-state controller, until wound down or shut down.
-///
-/// `me` is this party's own directory entry, held directly so the hot path
-/// never has to look itself up.
-///
-/// The redaction pool is fed by an [`UnorderedMessages`] observer from
-/// genesis: the initial drain replays everything the party inherited (the
-/// seed content, or a fork parent's whole set), and each loop's drain picks
-/// up its own inserts and everything learned over the wire, exactly once
-/// each.
-fn run_party(
-    net: Arc<Net>,
-    me: Arc<SwarmPeer>,
-    rumors: Rumors<Payload>,
-    inbox: Receiver<SessionEnd>,
-    control: Receiver<Command>,
-) {
-    let runtime = tokio::runtime::Builder::new_current_thread()
-        .build()
-        .expect("build party runtime");
-    let mut rng = SmallRng::from_entropy();
-    let mut next_sync = Instant::now() + exponential(&mut rng, &net.controls);
-    let mut observer = rumors.unordered_messages();
-    let mut pool: Vec<Version> = Vec::new();
-
-    loop {
-        // 1. Serve every inbound session. Our engaged flag was set true by the
-        //    initiator that claimed us; clear it once we have reconciled.
-        while let Ok(end) = inbox.try_recv() {
-            serve_sync(&runtime, &net, &rumors, end);
-            me.engaged.store(false, Ordering::Release);
-        }
-
-        // Catch the redaction pool up with everything observed since the
-        // last turn — the sessions just served included — and republish our
-        // live count for the UI gauge. One snapshot serves the rest of the
-        // iteration: taking it after the serves keeps the controller's odds
-        // and the pool's liveness checks current with what just arrived.
-        drain_versions(&mut observer, &mut pool);
-        let snap = rumors.snapshot();
-        me.live.store(snap.len() as u64, Ordering::Relaxed);
-
-        // 2. Obey membership commands from the coordinator, between sessions.
-        while let Ok(cmd) = control.try_recv() {
-            match cmd {
-                Command::Fork { reply } => {
-                    // Create a genuine disjoint child that inherits our content,
-                    // so it can independently churn and gossip; its thread
-                    // rebuilds the version pool by observer replay. We keep
-                    // running unchanged.
-                    let child = bootstrap_fork(&runtime, &rumors, net.duplex_capacity);
-                    let _ = reply.send(Donation { rumors: child });
-                }
-                Command::WindDown { reply } => {
-                    // Finish anything owed, lock ourselves out of new claims,
-                    // then hand our whole state over and exit.
-                    let donation = wind_down(&runtime, &net, &me, &inbox, rumors);
-                    let _ = reply.send(donation);
-                    return;
-                }
-            }
-        }
-
-        if net.shutdown.load(Ordering::SeqCst) {
-            break;
-        }
-
-        // While paused, keep serving inbound (above) but generate nothing.
-        if net.controls.paused.load(Ordering::SeqCst) {
-            thread::sleep(Duration::from_millis(1));
-            continue;
-        }
-
-        // 3. Time to initiate a sync? Only while still running, and only if we
-        //    can claim both ourselves and a peer. When the claim fails (peer
-        //    busy), fall through to local work and retry on the next
-        //    iteration — next_sync stays in the past — so a due initiator
-        //    keeps churning instead of spinning on claim attempts.
-        if net.running.load(Ordering::SeqCst)
-            && Instant::now() >= next_sync
-            && try_initiate(&runtime, &net, &me, &mut rng, &rumors)
-        {
-            next_sync = Instant::now() + exponential(&mut rng, &net.controls);
-            continue;
-        }
-
-        // 4. Local churn under the steady-state controller.
-        local_op(&net, &mut rng, &rumors, &snap, &mut pool);
-    }
-}
-
-/// Pull every message the observer has pending — without blocking — and push
-/// its version into the pool. Each message is yielded exactly once across
-/// the party's lifetime, so the pool never holds duplicates.
-fn drain_versions(observer: &mut UnorderedMessages<Payload>, pool: &mut Vec<Version>) {
-    while let Some(Some((version, _))) = observer.next().now_or_never() {
-        pool.push(version.clone());
-    }
-}
-
-/// Wind a party down so the coordinator can absorb it.
-///
-/// Serves any owed session,
-/// then claims our own engaged flag — permanently — so no initiator can open a
-/// new session with us. Because an initiator sets a peer's flag *before*
-/// delivering the session, a successful claim here proves nothing is owed; if
-/// the claim loses to an initiator, we serve that session and retry. Returns
-/// our state for the coordinator to fork from or retire.
-fn wind_down(
-    runtime: &tokio::runtime::Runtime,
-    net: &Net,
-    me: &SwarmPeer,
-    inbox: &Receiver<SessionEnd>,
-    rumors: Rumors<Payload>,
-) -> Donation {
-    loop {
-        while let Ok(end) = inbox.try_recv() {
-            serve_sync(runtime, net, &rumors, end);
-            me.engaged.store(false, Ordering::Release);
-        }
-        if me
-            .engaged
-            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
-            .is_ok()
-        {
-            break;
-        }
-        // An initiator holds us; its session is arriving. Yield and serve it.
-        thread::yield_now();
-    }
-    // Locked: nothing new can arrive. Drain any straggler for good measure.
-    while let Ok(end) = inbox.try_recv() {
-        serve_sync(runtime, net, &rumors, end);
-    }
-    Donation { rumors }
-}
-
-/// Attempt to initiate a sync with a random other party. Returns `true` if a
-/// session actually ran (both claims succeeded), `false` if the peer was busy
-/// or there was no one to pick.
-fn try_initiate(
-    runtime: &tokio::runtime::Runtime,
-    net: &Net,
-    me: &Arc<SwarmPeer>,
-    rng: &mut SmallRng,
-    rumors: &Rumors<Payload>,
-) -> bool {
-    // Claim ourselves first. If we are already engaged (a responder claimed us
-    // between the inbox drain and now), back off.
-    if me
-        .engaged
-        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
-        .is_err()
-    {
-        return false;
-    }
-
-    // Snapshot the directory and pick a peer that is not us. Cloning the chosen
-    // `Arc` lets us drop the snapshot immediately; the peer stays alive for the
-    // whole session even if the coordinator removes it from the directory.
-    let peer = match pick_peer(rng, &net.peers.load(), me.id) {
-        Some(peer) => peer,
-        None => {
-            me.engaged.store(false, Ordering::Release);
-            return false;
-        }
-    };
-
-    // Claim the peer. If it is engaged, release ourselves and back off.
-    if peer
-        .engaged
-        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
-        .is_err()
-    {
-        me.engaged.store(false, Ordering::Release);
-        return false;
-    }
-
-    // Both claimed. Reserve the in-flight slot (see `InflightGuard::reserve`
-    // for the ordering against shutdown); the guard releases it on every
-    // exit from this function, panics included.
-    let _inflight = InflightGuard::reserve(&net.metrics);
-    if !net.running.load(Ordering::SeqCst) {
-        peer.engaged.store(false, Ordering::Release);
-        me.engaged.store(false, Ordering::Release);
-        return false;
-    }
-
-    // Hand the peer one end of a fresh link and gossip the other.
-    let (mine, theirs) = rumors::link::memory_with_capacity(net.duplex_capacity);
-    if peer.inbox.send(theirs).is_err() {
-        peer.engaged.store(false, Ordering::Release);
-        me.engaged.store(false, Ordering::Release);
-        return false;
-    }
-
-    // Roundtrips are write→read flips on the *control* stream — the session's
-    // request→response turns — so only its halves carry the `Rounds`; the
-    // data streams tally bytes but never roundtrips.
-    let rounds = Arc::new(Rounds::default());
-    let mut link = initiator_link(
-        mine,
-        Arc::clone(&net.metrics.wire_bytes),
-        Arc::clone(&rounds),
-    );
-
-    // Latency is the wall-clock span of the gossip exchange itself: `start` is
-    // taken immediately before the protocol runs and `elapsed` immediately
-    // after it returns. It is never derived from the Poisson schedule.
-    // (Learned messages surface through the party's observer on its next
-    // drain.)
-    let start = Instant::now();
-    // The swarm's links are in-process and its parties one universe, so a
-    // failed session is a bug and panicking is honest. A real application
-    // matches on the error instead: `Error::LinkPoisoned` and
-    // `Error::Epilogue` call for a reconnect (the replica is intact — or,
-    // for `Epilogue`, already fully committed), `Error::NetworkMismatch`
-    // for a partition-merge decision, and I/O errors for discarding the
-    // link and retrying later.
-    runtime
-        .block_on(rumors.gossip(&mut link))
-        .expect("initiator gossip");
-    let elapsed = start.elapsed();
-
-    net.metrics.record_sync(elapsed, rounds.roundtrips());
-    me.engaged.store(false, Ordering::Release);
-    true
-}
-
-/// Drive the responder side of a session that some initiator opened with us.
-/// (Learned messages surface through the party's observer on its next
-/// drain.)
-fn serve_sync(
-    runtime: &tokio::runtime::Runtime,
-    net: &Net,
-    rumors: &Rumors<Payload>,
-    end: SessionEnd,
-) {
-    // The responder counts only the bytes it writes (its outbound direction on
-    // the control stream and every data stream it opens); the initiator counts
-    // the other direction. Roundtrips are tallied by the initiator alone, so
-    // the responder attaches no `Rounds`.
-    let mut link = responder_link(end, Arc::clone(&net.metrics.wire_bytes));
-    // See `try_initiate`: a real application matches on the session error
-    // instead of panicking.
-    runtime
-        .block_on(rumors.gossip(&mut link))
-        .expect("responder gossip");
-}
-
-/// Perform one unit of local churn under the steady-state controller, against
-/// `snap`, the caller's current snapshot, and bump the ops counter.
-fn local_op(
-    net: &Net,
-    rng: &mut SmallRng,
-    rumors: &Rumors<Payload>,
-    snap: &rumors::Snapshot<Payload>,
-    pool: &mut Vec<Version>,
-) {
-    let target = net.controls.target.load(Ordering::Relaxed);
-    let size = net.controls.message_size.load(Ordering::Relaxed) as usize;
-    steady_state_op(rng, rumors, snap, pool, target, size);
-    net.metrics.local_ops.fetch_add(1, Ordering::Relaxed);
-}
-
-/// One steady-state controller op: insert with probability
-/// `target / (target + live)` (1.0 when empty, 0.5 at target, → 0 far over),
-/// otherwise redact a **live** message, so the live set is driven toward
-/// `target`.
-///
-/// Falls back to an insert when no live message is in the pool.
-///
-/// The redact arm draws until it finds a message still present in `snap`,
-/// discarding stale entries — messages other parties already redacted — as
-/// they surface, without spending the op on them. The discard is what keeps
-/// the fixed point at `live == target` for any swarm size: every party's
-/// pool takes in every party's inserts but drains only by its own draws, so
-/// counting a stale draw as the op's redaction would let the stale backlog
-/// grow with the swarm and starve the downward pressure (live counts then
-/// stall near `parties × target` instead).
-fn steady_state_op(
-    rng: &mut SmallRng,
-    rumors: &Rumors<Payload>,
-    snap: &rumors::Snapshot<Payload>,
-    pool: &mut Vec<Version>,
-    target: u64,
-    message_size: usize,
-) {
-    let target = target as f64;
-    let live = snap.len() as f64;
-    let p_add = if target <= 0.0 {
-        0.0
-    } else {
-        target / (target + live)
-    };
-
-    if !rng.gen_bool(p_add.clamp(0.0, 1.0)) {
-        // Swap-remove random pool entries until one is still live, and
-        // redact it: the message leaves our local view and the redaction
-        // propagates on our next sync. Stale entries leave the vector as
-        // they are drawn, so a redaction burst elsewhere costs at most one
-        // pass over the pool here.
-        while !pool.is_empty() {
-            let idx = rng.gen_range(0..pool.len());
-            let version = pool.swap_remove(idx);
-            if snap.get(&version).is_some() {
-                rumors.redact(&version);
-                return;
-            }
-        }
-    }
-    // The add arm — or a pool with no live message left in it. The send's
-    // version reaches the pool through the observer's next drain.
-    rumors
-        .send(random_message(rng, message_size))
-        .expect("flat payload");
-}
-
-/// Draw an exponential inter-arrival time with the current mean, so successive
-/// syncs form a Poisson process. `1 - u` keeps the log argument in `(0, 1]`,
-/// avoiding `ln(0)`.
-fn exponential(rng: &mut SmallRng, controls: &Controls) -> Duration {
-    let mean_secs = controls.sync_interval_us.load(Ordering::Relaxed) as f64 / 1e6;
-    let u: f64 = rng.r#gen();
-    let secs = -mean_secs * (1.0 - u).ln();
-    Duration::from_secs_f64(secs.max(0.0))
-}
-
-/// A fresh random payload of `size` bytes.
-fn random_message(rng: &mut SmallRng, size: usize) -> Payload {
-    let mut buf = vec![0u8; size];
-    rng.fill_bytes(&mut buf);
-    buf
-}
-
-/// Uniformly pick a directory entry whose `id` is not `me`, cloning its `Arc`.
-/// Returns `None` when there is no other party to pick (fewer than two live).
-fn pick_peer(rng: &mut SmallRng, dir: &[Arc<SwarmPeer>], me: u64) -> Option<Arc<SwarmPeer>> {
-    if dir.len() < 2 {
-        return None;
-    }
-    loop {
-        let peer = &dir[rng.gen_range(0..dir.len())];
-        if peer.id != me {
-            return Some(Arc::clone(peer));
-        }
-    }
-}
-
-// --- membership coordinator ------------------------------------------------
-
-/// The coordinator thread.
-///
-/// The sole writer of the peer directory: it launches
-/// the initial parties, then reconciles the live party count toward the desired
-/// one by forking (to grow) or retiring one party into another (to shrink),
-/// one step per iteration.
-///
-/// Returns every outstanding party join handle once `running` is cleared, so
-/// the main thread can join them after the in-flight sessions drain.
-fn run_coordinator(net: Arc<Net>, initial: Vec<Donation>) -> Vec<JoinHandle<()>> {
-    let runtime = tokio::runtime::Builder::new_current_thread()
-        .build()
-        .expect("build coordinator runtime");
-    let mut next_id: u64 = 0;
-    let mut handles = Vec::with_capacity(initial.len());
-    let mut peers = Vec::with_capacity(initial.len());
-    for donation in initial {
-        let (peer, handle) = launch_party(&net, &mut next_id, donation);
-        peers.push(peer);
-        handles.push(handle);
-    }
-    net.peers.store(Arc::new(peers));
-
-    let mut rng = SmallRng::from_entropy();
-    while net.running.load(Ordering::SeqCst) {
-        let desired = net.controls.parties.load(Ordering::Relaxed) as usize;
-        let current = net.peers.load().len();
-        if desired > current {
-            grow(&net, &mut rng, &mut next_id, &mut handles);
-        } else if desired < current {
-            shrink(&runtime, &net, &mut rng, &mut next_id, &mut handles);
-        } else {
-            // Balanced: nothing to do until the knob or the loser of a claim
-            // race changes things. Poll at a human-noticeable cadence.
-            thread::sleep(Duration::from_millis(20));
-        }
-    }
-    handles
-}
-
-/// Build a party's directory entry and channels, spawn its thread, and return
-/// the shared [`SwarmPeer`] alongside the join handle. Assigns the next unique
-/// id.
-fn launch_party(
-    net: &Arc<Net>,
-    next_id: &mut u64,
-    donation: Donation,
-) -> (Arc<SwarmPeer>, JoinHandle<()>) {
-    let id = *next_id;
-    *next_id += 1;
-    let (inbox_tx, inbox_rx) = channel::<SessionEnd>();
-    let (control_tx, control_rx) = channel::<Command>();
-    let peer = Arc::new(SwarmPeer {
-        id,
-        engaged: AtomicBool::new(false),
-        inbox: inbox_tx,
-        control: control_tx,
-        live: AtomicU64::new(donation.rumors.snapshot().len() as u64),
-    });
-    let handle = {
-        let net = Arc::clone(net);
-        let peer = Arc::clone(&peer);
-        let Donation { rumors } = donation;
-        thread::Builder::new()
-            .name(format!("party-{id}"))
-            .spawn(move || run_party(net, peer, rumors, inbox_rx, control_rx))
-            .expect("spawn party thread")
-    };
-    (peer, handle)
-}
-
-/// Grow by one: fork a random live party's [`Rumors`] and run the child in a
-/// new thread. The child and parent are disjoint sub-parties, so the directory
-/// stays a valid partition of the seed's party space.
-fn grow(net: &Arc<Net>, rng: &mut SmallRng, next_id: &mut u64, handles: &mut Vec<JoinHandle<()>>) {
-    let dir = net.peers.load_full();
-    if dir.is_empty() {
-        return;
-    }
-    let parent = &dir[rng.gen_range(0..dir.len())];
-    let (reply_tx, reply_rx) = channel::<Donation>();
-    if parent
-        .control
-        .send(Command::Fork { reply: reply_tx })
-        .is_err()
-    {
-        return; // parent already gone; try again next tick
-    }
-    let Ok(child) = reply_rx.recv() else {
-        return;
-    };
-    let (peer, handle) = launch_party(net, next_id, child);
-    handles.push(handle);
-
-    let mut peers = (*dir).clone();
-    peers.push(peer);
-    net.peers.store(Arc::new(peers));
-}
-
-/// Shrink by one: wind down two random parties, [`retire`](Peer::retire)
-/// one into the other over an in-memory wire, and run the survivor in a new
-/// thread.
-///
-/// The retire session's gossip round carries any divergent content
-/// across before the party hand-off, and the survivor absorbs the retiree's
-/// id-region — the merge is leak-free. Any two live parties are disjoint
-/// forks of the common seed, so the session always commits.
-fn shrink(
-    runtime: &tokio::runtime::Runtime,
-    net: &Arc<Net>,
-    rng: &mut SmallRng,
-    next_id: &mut u64,
-    handles: &mut Vec<JoinHandle<()>>,
-) {
-    let dir = net.peers.load_full();
-    if dir.len() <= 2 {
-        return; // keep at least two parties: there must be someone to gossip with
-    }
-    // Pick two distinct entries.
-    let i = rng.gen_range(0..dir.len());
-    let mut j = rng.gen_range(0..dir.len() - 1);
-    if j >= i {
-        j += 1;
-    }
-    let (a, b) = (Arc::clone(&dir[i]), Arc::clone(&dir[j]));
-
-    // Wind both down. Sending both commands before awaiting either reply lets
-    // the two parties finish their owed sessions concurrently.
-    let (a_tx, a_rx) = channel::<Donation>();
-    let (b_tx, b_rx) = channel::<Donation>();
-    if a.control.send(Command::WindDown { reply: a_tx }).is_err()
-        || b.control.send(Command::WindDown { reply: b_tx }).is_err()
-    {
-        return; // a party already gone; try again next tick
-    }
-    let (Ok(da), Ok(db)) = (a_rx.recv(), b_rx.recv()) else {
-        return;
-    };
-
-    // Merge: retire b into a over an in-memory wire. The survivor's key pool
-    // is rebuilt by observer replay in its new thread, so nothing but the
-    // `Rumors` needs to move. Retiring requires the unique [`Peer`] handle;
-    // the wound-down party's `Rumors` is the last one standing, so reclaiming
-    // it resolves immediately.
-    let rumors = da.rumors;
-    let retiree = runtime
-        .block_on(db.rumors.try_into_peer())
-        .expect("a wound-down party's handle is unique");
-    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(net.duplex_capacity);
-    let (retired, survived) = runtime
-        .block_on(async { tokio::join!(retiree.retire(&mut b_link), rumors.gossip(&mut a_link)) });
-    // See `try_initiate`: a real application matches on the session error
-    // instead of panicking.
-    survived.expect("survivor gossip");
-    assert!(
-        matches!(retired, Retire::Retired),
-        "two live swarm parties always reconcile, got {retired:?}"
-    );
-
-    let (peer, handle) = launch_party(net, next_id, Donation { rumors });
-    handles.push(handle);
-
-    // Swap in a directory without the two retired parties, plus the merged one.
-    let mut peers: Vec<Arc<SwarmPeer>> = dir
-        .iter()
-        .filter(|p| p.id != a.id && p.id != b.id)
-        .cloned()
-        .collect();
-    peers.push(peer);
-    net.peers.store(Arc::new(peers));
-}
-
-// --- byte- and roundtrip-counting I/O wrappers -----------------------------
-
-/// Direction-flip counter shared between an initiator's reader and writer. A
-/// write→read flip marks one completed request→response roundtrip.
-#[derive(Default)]
-struct Rounds {
-    inner: std::sync::Mutex<RoundState>,
-}
-
-#[derive(Default)]
-struct RoundState {
-    /// Whether the last I/O on this session was a write.
-    last_was_write: bool,
-    /// Completed write→read roundtrips.
-    roundtrips: u64,
-}
-
-impl Rounds {
-    fn on_write(&self) {
-        self.inner.lock().unwrap().last_was_write = true;
-    }
-
-    fn on_read(&self) {
-        let mut s = self.inner.lock().unwrap();
-        if s.last_was_write {
-            s.roundtrips += 1;
-            s.last_was_write = false;
-        }
-    }
-
-    fn roundtrips(&self) -> u64 {
-        self.inner.lock().unwrap().roundtrips
-    }
-}
-
-/// `AsyncWrite` wrapper that tallies bytes into a shared counter and, when a
-/// `Rounds` is attached, records the write phase for roundtrip counting.
-///
-/// The counter is owned as an [`Arc`] rather than borrowed so this wrapper
-/// can back a [`Connector::Tx`], whose `'static` bound outlives any borrow of
-/// the metrics.
-struct CountWrite<W> {
-    inner: W,
-    wire_bytes: Arc<AtomicU64>,
-    rounds: Option<Arc<Rounds>>,
-}
-
-impl<W: AsyncWrite + Unpin> AsyncWrite for CountWrite<W> {
-    fn poll_write(
-        self: Pin<&mut Self>,
-        cx: &mut Context<'_>,
-        buf: &[u8],
-    ) -> Poll<io::Result<usize>> {
-        let this = self.get_mut();
-        match Pin::new(&mut this.inner).poll_write(cx, buf) {
-            Poll::Ready(Ok(n)) => {
-                this.wire_bytes.fetch_add(n as u64, Ordering::Relaxed);
-                if let Some(rounds) = &this.rounds {
-                    rounds.on_write();
-                }
-                Poll::Ready(Ok(n))
-            }
-            other => other,
-        }
-    }
-
-    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
-        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
-    }
-
-    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
-        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
-    }
-}
-
-/// `AsyncRead` wrapper that records the read phase for roundtrip counting. It
-/// does not tally bytes: each byte is counted once, on the writer that sent it.
-struct CountRead<R> {
-    inner: R,
-    rounds: Option<Arc<Rounds>>,
-}
-
-impl<R: AsyncRead + Unpin> AsyncRead for CountRead<R> {
-    fn poll_read(
-        self: Pin<&mut Self>,
-        cx: &mut Context<'_>,
-        buf: &mut ReadBuf<'_>,
-    ) -> Poll<io::Result<()>> {
-        let this = self.get_mut();
-        let before = buf.filled().len();
-        match Pin::new(&mut this.inner).poll_read(cx, buf) {
-            Poll::Ready(Ok(())) => {
-                if buf.filled().len() > before
-                    && let Some(rounds) = &this.rounds
-                {
-                    rounds.on_read();
-                }
-                Poll::Ready(Ok(()))
-            }
-            other => other,
-        }
-    }
-}
-
-/// A [`Connector`] that wraps each opened data stream's writer in a
-/// [`CountWrite`], so the bytes a session pushes down its data streams join
-/// the same tally as its control-stream bytes.
-///
-/// Data-stream writes never carry
-/// a `Rounds`: roundtrips are a property of the bidirectional control stream.
-#[derive(Clone)]
-struct CountConnector {
-    inner: MemoryConnector,
-    wire_bytes: Arc<AtomicU64>,
-}
-
-impl Connector for CountConnector {
-    type Tx = CountWrite<DuplexStream>;
-
-    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
-        let (tx, _) = self.inner.connect().await?;
-        Ok((
-            CountWrite {
-                inner: tx,
-                wire_bytes: Arc::clone(&self.wire_bytes),
-                rounds: None,
-            },
-            Done::discard(),
-        ))
-    }
-}
-
-/// The initiator's decorated link: both control halves count (writes tally
-/// bytes and roundtrips, reads tally roundtrips) and each opened data stream
-/// tallies its bytes.
-///
-/// Incoming data streams are accepted unwrapped — their
-/// bytes are counted by the peer that wrote them.
-type InitiatorLink =
-    Link<CountRead<DuplexStream>, CountWrite<DuplexStream>, CountConnector, MemoryAcceptor>;
-
-/// The responder's decorated link: it counts only the bytes it writes (its
-/// control write half and each data stream it opens), so its control read half
-/// is left bare and no `Rounds` is attached anywhere.
-type ResponderLink = Link<DuplexStream, CountWrite<DuplexStream>, CountConnector, MemoryAcceptor>;
-
-/// Decorate an in-memory link end for the initiator's accounting: byte and
-/// roundtrip counting on the control stream, byte counting on opened data
-/// streams.
-///
-/// The [`SessionState`](rumors::link::SessionState) is carried
-/// through unchanged, so decoration neither resets the session counter that
-/// keeps the two ends in lockstep nor clears a poison latch.
-fn initiator_link(
-    link: MemoryLink,
-    wire_bytes: Arc<AtomicU64>,
-    rounds: Arc<Rounds>,
-) -> InitiatorLink {
-    let parts = link.into_parts();
-    LinkParts {
-        control_read: CountRead {
-            inner: parts.control_read,
-            rounds: Some(Arc::clone(&rounds)),
-        },
-        control_write: CountWrite {
-            inner: parts.control_write,
-            wire_bytes: Arc::clone(&wire_bytes),
-            rounds: Some(rounds),
-        },
-        connector: CountConnector {
-            inner: parts.connector,
-            wire_bytes,
-        },
-        acceptor: parts.acceptor,
-        session: parts.session,
-    }
-    .into_link()
-}
-
-/// Decorate an in-memory link end for the responder's accounting: byte
-/// counting on every stream it writes, nothing on the streams it reads.
-fn responder_link(link: MemoryLink, wire_bytes: Arc<AtomicU64>) -> ResponderLink {
-    let parts = link.into_parts();
-    LinkParts {
-        control_read: parts.control_read,
-        control_write: CountWrite {
-            inner: parts.control_write,
-            wire_bytes: Arc::clone(&wire_bytes),
-            rounds: None,
-        },
-        connector: CountConnector {
-            inner: parts.connector,
-            wire_bytes,
-        },
-        acceptor: parts.acceptor,
-        session: parts.session,
-    }
-    .into_link()
-}
-
-// --- headless measurement --------------------------------------------------
-
-/// Sample the same windowed statistics the UI renders, for `headless_secs`,
-/// then print one summary line per readout row to stdout. The first sampling
-/// window is discarded as warmup so the summary reflects steady state.
-fn run_headless(net: &Net, args: &Args) {
-    let sample_interval = Duration::from_millis(args.refresh_ms.max(1));
-    let windows = (args.headless_secs * 1000 / args.refresh_ms.max(1)).max(2) as usize;
-    let mut prev = Snapshot::take(net);
-    let mut samples: Vec<Stats> = Vec::with_capacity(windows);
-    for i in 0..windows {
-        thread::sleep(sample_interval);
-        let now = Snapshot::take(net);
-        let stats = compute(net, &prev, &now);
-        prev = now;
-        if i > 0 {
-            samples.push(stats);
-        }
-    }
-    let n = samples.len().max(1) as f64;
-    let mean = |f: fn(&Stats) -> f64| samples.iter().map(f).sum::<f64>() / n;
-    let ops_total = mean(|s| s.ops_total);
-    let ops_per_party = mean(|s| s.ops_per_party);
-    let bandwidth = mean(|s| s.bandwidth);
-    let sync_rate = mean(|s| s.sync_rate);
-    let avg_live = mean(|s| s.avg_live);
-    // Latency and roundtrips are formatted strings in `Stats`; recompute the
-    // means from the raw counters across the whole measured span instead. The
-    // best is the fastest session seen in any window.
-    let best = samples
-        .iter()
-        .map(|s| s.latency_best_nanos)
-        .min()
-        .unwrap_or(u64::MAX);
-    let end = Snapshot::take(net);
-    let syncs = end.syncs.max(1);
-    let latency = Duration::from_nanos(end.sync_nanos / syncs);
-    let roundtrips = end.roundtrips as f64 / syncs as f64;
-    println!("parties          {}", net.peers.load().len());
-    println!("local ops/s      {ops_total:.0} total, {ops_per_party:.0} per party");
-    println!("wire bandwidth   {} per direction", format_rate(bandwidth));
-    println!(
-        "sync latency     {} mean, {} best, over {} sessions",
-        format_duration(latency),
-        if best == u64::MAX {
-            "--".to_string()
-        } else {
-            format_duration(Duration::from_nanos(best))
-        },
-        end.syncs
-    );
-    println!("sync rate        {sync_rate:.1}/s");
-    println!("roundtrips/sync  {roundtrips:.1}");
-    println!(
-        "live messages    {avg_live:.0}/node (target {})",
-        net.controls.target.load(Ordering::Relaxed)
-    );
-}
-
-// --- interactive UI --------------------------------------------------------
-
-/// Largest party count the UI will dial up to. A guard against accidentally
-/// spawning an unreasonable number of OS threads, not a protocol limit.
-const MAX_PARTIES: u64 = 64;
-
-/// The live-adjustable parameters, in selection order.
-#[derive(Copy, Clone)]
-enum Field {
-    Parties,
-    SyncInterval,
-    Target,
-    MessageSize,
-}
-
-impl Field {
-    const ALL: [Field; 4] = [
-        Field::Parties,
-        Field::SyncInterval,
-        Field::Target,
-        Field::MessageSize,
-    ];
-
-    fn label(self) -> &'static str {
-        match self {
-            Field::Parties => "parties",
-            Field::SyncInterval => "sync interval",
-            Field::Target => "target msgs",
-            Field::MessageSize => "message size",
-        }
-    }
-
-    /// The current value of this field, formatted for display.
-    fn value(self, controls: &Controls) -> String {
-        match self {
-            Field::Parties => format!("{}", controls.parties.load(Ordering::Relaxed)),
-            Field::SyncInterval => {
-                let ms = controls.sync_interval_us.load(Ordering::Relaxed) as f64 / 1000.0;
-                format!("{ms:.0} ms")
-            }
-            Field::Target => format!("{}", controls.target.load(Ordering::Relaxed)),
-            Field::MessageSize => format!("{} B", controls.message_size.load(Ordering::Relaxed)),
-        }
-    }
-
-    /// Nudge this field. `increase` chooses direction; `coarse` chooses a
-    /// larger (roughly 8–10×) step. Values are clamped to sane bounds.
-    fn adjust(self, controls: &Controls, increase: bool, coarse: bool) {
-        match self {
-            Field::Parties => {
-                let step = if coarse { 8 } else { 1 };
-                // Floor of two: a lone party has no one to gossip with.
-                bump(&controls.parties, increase, step, 2, MAX_PARTIES);
-            }
-            Field::SyncInterval => {
-                let step = if coarse { 50_000 } else { 5_000 }; // microseconds
-                bump(&controls.sync_interval_us, increase, step, 1_000, 5_000_000);
-            }
-            Field::Target => {
-                let step = if coarse { 100 } else { 10 };
-                bump(&controls.target, increase, step, 0, 1_000_000);
-            }
-            Field::MessageSize => {
-                let step = if coarse { 256 } else { 32 };
-                bump(&controls.message_size, increase, step, 1, 65_536);
-            }
-        }
-    }
-}
-
-/// Add or subtract `step` from `value`, clamped to `[min, max]`.
-fn bump(value: &AtomicU64, increase: bool, step: u64, min: u64, max: u64) {
-    let cur = value.load(Ordering::Relaxed);
-    let next = if increase {
-        cur.saturating_add(step).min(max)
-    } else {
-        cur.saturating_sub(step).max(min)
-    };
-    value.store(next, Ordering::Relaxed);
-}
-
-/// A point-in-time read of the counters plus the instant it was taken.
-struct Snapshot {
-    at: Instant,
-    local_ops: u64,
-    wire_bytes: u64,
-    wire_direction_nanos: u64,
-    syncs: u64,
-    sync_nanos: u64,
-    roundtrips: u64,
-}
-
-impl Snapshot {
-    fn take(net: &Net) -> Self {
-        let m = &net.metrics;
-        Snapshot {
-            at: Instant::now(),
-            local_ops: m.local_ops.load(Ordering::Relaxed),
-            wire_bytes: m.wire_bytes.load(Ordering::Relaxed),
-            wire_direction_nanos: m.wire_direction_nanos.load(Ordering::Relaxed),
-            syncs: m.syncs.load(Ordering::Relaxed),
-            sync_nanos: m.sync_nanos.load(Ordering::Relaxed),
-            roundtrips: m.roundtrips.load(Ordering::Relaxed),
-        }
-    }
-}
-
-/// Windowed statistics derived from two snapshots and the live gauges.
-struct Stats {
-    ops_per_party: f64,
-    ops_total: f64,
-    bandwidth: f64,
-    latency: String,
-    /// Fastest session in the window, nanos; `u64::MAX` when none completed.
-    latency_best_nanos: u64,
-    sync_rate: f64,
-    roundtrips: String,
-    avg_live: f64,
-    syncs_total: u64,
-}
-
-/// Compute windowed rates between two snapshots.
-fn compute(net: &Net, prev: &Snapshot, now: &Snapshot) -> Stats {
-    let dir = net.peers.load();
-    let parties = (dir.len() as f64).max(1.0);
-    let dt = now
-        .at
-        .duration_since(prev.at)
-        .as_secs_f64()
-        .max(f64::MIN_POSITIVE);
-
-    let d_ops = now.local_ops - prev.local_ops;
-    let ops_total = d_ops as f64 / dt;
-
-    let d_bytes = now.wire_bytes - prev.wire_bytes;
-    let d_dir_nanos = now.wire_direction_nanos - prev.wire_direction_nanos;
-    let bandwidth = if d_dir_nanos > 0 {
-        d_bytes as f64 / (d_dir_nanos as f64 / 1e9)
-    } else {
-        0.0
-    };
-
-    let d_syncs = now.syncs - prev.syncs;
-    let (latency, roundtrips, sync_rate) =
-        if let Some(lat_nanos) = (now.sync_nanos - prev.sync_nanos).checked_div(d_syncs) {
-            let lat = Duration::from_nanos(lat_nanos);
-            let rt = (now.roundtrips - prev.roundtrips) as f64 / d_syncs as f64;
-            (
-                format_duration(lat),
-                format!("{rt:.1}"),
-                d_syncs as f64 / dt,
-            )
-        } else {
-            ("--".to_string(), "--".to_string(), 0.0)
-        };
-
-    let live_total: u64 = dir.iter().map(|p| p.live.load(Ordering::Relaxed)).sum();
-
-    // Consume the window's fastest session and re-arm the sentinel for the
-    // next window.
-    let latency_best_nanos = net
-        .metrics
-        .sync_nanos_best
-        .swap(u64::MAX, Ordering::Relaxed);
-
-    Stats {
-        ops_per_party: ops_total / parties,
-        ops_total,
-        bandwidth,
-        latency,
-        latency_best_nanos,
-        sync_rate,
-        roundtrips,
-        avg_live: live_total as f64 / parties,
-        syncs_total: now.syncs,
-    }
-}
-
-/// Bounded history rings for the charts.
-struct History {
-    live: VecDeque<u64>,
-    ops: VecDeque<u64>,
-    cap: usize,
-}
-
-impl History {
-    fn new(cap: usize) -> Self {
-        History {
-            live: VecDeque::with_capacity(cap),
-            ops: VecDeque::with_capacity(cap),
-            cap,
-        }
-    }
-
-    fn push(&mut self, live: u64, ops: u64) {
-        for (ring, v) in [(&mut self.live, live), (&mut self.ops, ops)] {
-            if ring.len() == self.cap {
-                ring.pop_front();
-            }
-            ring.push_back(v);
-        }
-    }
-}
-
-/// Run the terminal UI until the user quits. Sets up and tears down raw mode
-/// and the alternate screen, restoring the terminal on any exit path.
-fn run_ui(net: &Net, args: &Args) -> io::Result<()> {
-    // Restore the terminal even if a party thread (or this one) panics.
-    let default_hook = std::panic::take_hook();
-    std::panic::set_hook(Box::new(move |info| {
-        let _ = disable_raw_mode();
-        let _ = execute!(io::stdout(), LeaveAlternateScreen);
-        default_hook(info);
-    }));
-
-    enable_raw_mode()?;
-    let mut stdout = io::stdout();
-    execute!(stdout, EnterAlternateScreen)?;
-    let backend = CrosstermBackend::new(stdout);
-    let mut terminal = Terminal::new(backend)?;
-
-    let result = ui_loop(&mut terminal, net, args);
-
-    disable_raw_mode()?;
-    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
-    terminal.show_cursor()?;
-    result
-}
-
-/// The event/render loop. Renders every wake; resamples the windowed stats on
-/// a steady cadence so event-driven redraws (e.g. a key press) don't compute
-/// rates over a vanishingly small window.
-fn ui_loop(
-    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
-    net: &Net,
-    args: &Args,
-) -> io::Result<()> {
-    let sample_interval = Duration::from_millis(args.refresh_ms);
-    let mut selected = 0usize;
-    let mut history = History::new(160);
-    let mut prev = Snapshot::take(net);
-    let mut stats = compute(net, &prev, &Snapshot::take(net));
-    let mut last_sample = Instant::now();
-    let started = Instant::now();
-
-    loop {
-        if event::poll(sample_interval)?
-            && let Event::Key(key) = event::read()?
-            && key.kind == KeyEventKind::Press
-        {
-            let coarse = key.modifiers.contains(KeyModifiers::SHIFT);
-            match key.code {
-                KeyCode::Char('q') | KeyCode::Esc => break,
-                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
-                    break;
-                }
-                KeyCode::Up => selected = selected.saturating_sub(1),
-                KeyCode::Down => selected = (selected + 1).min(Field::ALL.len() - 1),
-                KeyCode::Left => Field::ALL[selected].adjust(&net.controls, false, coarse),
-                KeyCode::Right => Field::ALL[selected].adjust(&net.controls, true, coarse),
-                KeyCode::Char(' ') => {
-                    let p = &net.controls.paused;
-                    p.store(!p.load(Ordering::SeqCst), Ordering::SeqCst);
-                }
-                _ => {}
-            }
-        }
-
-        if last_sample.elapsed() >= sample_interval {
-            let now = Snapshot::take(net);
-            stats = compute(net, &prev, &now);
-            history.push(
-                stats.avg_live.round() as u64,
-                stats.ops_total.round() as u64,
-            );
-            prev = now;
-            last_sample = Instant::now();
-        }
-
-        terminal.draw(|frame| {
-            draw(frame, net, selected, &stats, &history, started.elapsed());
-        })?;
-    }
-    Ok(())
-}
-
-/// Paint one frame.
-fn draw(
-    frame: &mut ratatui::Frame,
-    net: &Net,
-    selected: usize,
-    stats: &Stats,
-    history: &History,
-    elapsed: Duration,
-) {
-    let rows = Layout::vertical([
-        Constraint::Length(3), // header
-        Constraint::Length(7), // params | stats
-        Constraint::Min(7),    // charts
-        Constraint::Length(1), // footer
-    ])
-    .split(frame.area());
-
-    draw_header(frame, rows[0], net, elapsed);
-
-    let mid =
-        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).split(rows[1]);
-    draw_params(frame, mid[0], net, selected);
-    draw_stats(frame, mid[1], stats);
-
-    draw_charts(frame, rows[2], net, stats, history);
-    draw_footer(frame, rows[3]);
-}
-
-fn draw_header(frame: &mut ratatui::Frame, area: Rect, net: &Net, elapsed: Duration) {
-    let paused = net.controls.paused.load(Ordering::SeqCst);
-    let (status, status_style) = if paused {
-        ("PAUSED", Style::default().fg(Color::Yellow))
-    } else {
-        ("running", Style::default().fg(Color::Green))
-    };
-    let line = Line::from(vec![
-        Span::styled(
-            " rumors swarm ",
-            Style::default()
-                .fg(Color::Cyan)
-                .add_modifier(Modifier::BOLD),
-        ),
-        Span::raw(format!("  {} parties   ", net.peers.load().len())),
-        Span::styled(status, status_style),
-        Span::raw(format!("   {:.0}s", elapsed.as_secs_f64())),
-    ]);
-    frame.render_widget(
-        Paragraph::new(line).block(Block::default().borders(Borders::ALL)),
-        area,
-    );
-}
-
-fn draw_params(frame: &mut ratatui::Frame, area: Rect, net: &Net, selected: usize) {
-    let lines: Vec<Line> = Field::ALL
-        .iter()
-        .enumerate()
-        .map(|(i, field)| {
-            let value = field.value(&net.controls);
-            let selected = i == selected;
-            let marker = if selected { "▸ " } else { "  " };
-            let style = if selected {
-                Style::default()
-                    .fg(Color::Black)
-                    .bg(Color::Cyan)
-                    .add_modifier(Modifier::BOLD)
-            } else {
-                Style::default()
-            };
-            Line::from(Span::styled(
-                format!("{marker}{:<14}{:>10}", field.label(), value),
-                style,
-            ))
-        })
-        .collect();
-    frame.render_widget(
-        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" parameters ")),
-        area,
-    );
-}
-
-fn draw_stats(frame: &mut ratatui::Frame, area: Rect, stats: &Stats) {
-    let kv = |k: &str, v: String| {
-        Line::from(vec![
-            Span::styled(format!("{k:<16}"), Style::default().fg(Color::DarkGray)),
-            Span::styled(v, Style::default().fg(Color::White)),
-        ])
-    };
-    let target_hint = format!("{:.0}/node", stats.avg_live);
-    let lines = vec![
-        kv(
-            "local ops/s",
-            format!(
-                "{:.0} /party  ({:.0} total)",
-                stats.ops_per_party, stats.ops_total
-            ),
-        ),
-        kv(
-            "wire bandwidth",
-            format!("{} /dir", format_rate(stats.bandwidth)),
-        ),
-        kv(
-            "sync latency",
-            format!(
-                "{} (best {})  ({:.1}/s)",
-                stats.latency,
-                if stats.latency_best_nanos == u64::MAX {
-                    "--".to_string()
-                } else {
-                    format_duration(Duration::from_nanos(stats.latency_best_nanos))
-                },
-                stats.sync_rate,
-            ),
-        ),
-        kv("roundtrips/sync", stats.roundtrips.clone()),
-        kv("live messages", target_hint),
-        kv("syncs total", format!("{}", stats.syncs_total)),
-    ];
-    frame.render_widget(
-        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" live stats ")),
-        area,
-    );
-}
-
-fn draw_charts(
-    frame: &mut ratatui::Frame,
-    area: Rect,
-    net: &Net,
-    stats: &Stats,
-    history: &History,
-) {
-    let halves =
-        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
-
-    let target = net.controls.target.load(Ordering::Relaxed);
-    let live: Vec<u64> = history.live.iter().copied().collect();
-    frame.render_widget(
-        Sparkline::default()
-            .block(Block::default().borders(Borders::ALL).title(format!(
-                " live messages/node: {:.0}   (target {target}) ",
-                stats.avg_live
-            )))
-            .data(&live)
-            .style(Style::default().fg(Color::Cyan)),
-        halves[0],
-    );
-
-    let ops: Vec<u64> = history.ops.iter().copied().collect();
-    frame.render_widget(
-        Sparkline::default()
-            .block(
-                Block::default()
-                    .borders(Borders::ALL)
-                    .title(format!(" local ops/s: {:.0} ", stats.ops_total)),
-            )
-            .data(&ops)
-            .style(Style::default().fg(Color::Magenta)),
-        halves[1],
-    );
-}
-
-fn draw_footer(frame: &mut ratatui::Frame, area: Rect) {
-    let hint = Line::from(vec![
-        Span::styled("  ↑/↓ ", Style::default().fg(Color::Cyan)),
-        Span::raw("select   "),
-        Span::styled("←/→ ", Style::default().fg(Color::Cyan)),
-        Span::raw("adjust (Shift = coarse)   "),
-        Span::styled("space ", Style::default().fg(Color::Cyan)),
-        Span::raw("pause   "),
-        Span::styled("q ", Style::default().fg(Color::Cyan)),
-        Span::raw("quit"),
-    ]);
-    frame.render_widget(Paragraph::new(hint), area);
-}
-
-/// Format a byte-rate with a binary unit prefix.
-fn format_rate(bytes_per_sec: f64) -> String {
-    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
-    if !bytes_per_sec.is_finite() || bytes_per_sec <= 0.0 {
-        return "0 B/s".to_string();
-    }
-    let mut value = bytes_per_sec;
-    let mut unit = 0;
-    while value >= 1024.0 && unit < UNITS.len() - 1 {
-        value /= 1024.0;
-        unit += 1;
-    }
-    format!("{value:.1} {}/s", UNITS[unit])
-}
-
-// The example file is its own crate root, so the module path is stated
-// explicitly to keep the tests beside it in `examples/swarm/` rather than
-// loose in `examples/` where cargo would take them for another example.
-#[cfg(test)]
-#[path = "swarm/tests.rs"]
-mod tests;
-
-/// Format a short duration with an adaptive unit (ns / µs / ms / s).
-fn format_duration(d: Duration) -> String {
-    let nanos = d.as_nanos();
-    if nanos < 1_000 {
-        format!("{nanos} ns")
-    } else if nanos < 1_000_000 {
-        format!("{:.1} µs", nanos as f64 / 1e3)
-    } else if nanos < 1_000_000_000 {
-        format!("{:.2} ms", nanos as f64 / 1e6)
-    } else {
-        format!("{:.2} s", nanos as f64 / 1e9)
-    }
-}
```

<!-- annotation -->
> **swarm-example-*** (T27), line 0:
>
> Ruling T27 deletes the example; this file is the whole of it: the module doc, the party threads, the steady-state controller, the memory-link connector shim, the ratatui terminal UI, and the headless mode. Every swarm-example-* entry described a defect in this file or its test; none of their resolutions is landed, because the deletion supersedes them.
> How I searched for references: `git grep -n -i swarm` over the whole tree minus the example itself, .agent-notes, and the before-viz www bundle (hits: the [[example]] block, and a fleet-shaped local variable in tests/window_census.rs that is not this example and stays); a whole-word grep for `example`/`examples` outside src/, crates/, formal/ (only unrelated hits: the Peer docs' lifecycle example, before's amp_board recipes, the envelope_sim and window_tradeoff recipes, fixture URLs in tools/); greps for the example's other possible names (steady-state controller, showcase, TUI, terminal UI, sparkline, ratatui, crossterm, arc-swap, clap); and a read of the justfile, .github/workflows, .cargo/mutants.toml, .config/nextest.toml, deny.toml, tools/*, README.md, AGENTS.md, design/, results/, .gitignore, proptest-regressions/, and tests/seed_liveness.rs. No file outside the example imported from it or depended on its behavior, and no design document cites it.

<a id="hunk-75"></a>
### examples/swarm/tests.rs `@@ -1,149 +0,0 @@`

```diff
@@ -1,149 +0,0 @@
-//! Deterministic tests for the swarm's steady-state controller.
-//!
-//! The controller's contract is that each node's live-message count
-//! converges onto the target — including after retargeting, and including
-//! when other parties' redactions have strewn stale entries through the
-//! local pool. Everything here is single-threaded and seeded, so a failure
-//! reproduces exactly.
-
-use super::*;
-
-/// Message size for controller tests: small, so runs stay fast — the
-/// controller's arithmetic never depends on payload size.
-const TEST_MESSAGE_SIZE: usize = 32;
-
-/// In-memory stream capacity for the test links, matching the swarm default.
-const TEST_DUPLEX_CAPACITY: usize = 16 * 1024;
-
-/// One test party: its rumor set, observer-fed version pool, and seeded
-/// rng — the same per-thread state `run_party` keeps, minus the threads.
-struct Party {
-    rumors: Rumors<Payload>,
-    observer: UnorderedMessages<Payload>,
-    pool: Vec<Version>,
-    rng: SmallRng,
-}
-
-impl Party {
-    fn new(rumors: Rumors<Payload>, seed: u64) -> Self {
-        let observer = rumors.unordered_messages();
-        Party {
-            rumors,
-            observer,
-            pool: Vec::new(),
-            rng: SmallRng::seed_from_u64(seed),
-        }
-    }
-
-    /// Run `ops` controller operations at `target`, draining the observer
-    /// and snapshotting before each op exactly as the party loop does.
-    fn churn(&mut self, target: u64, ops: usize) {
-        for _ in 0..ops {
-            drain_versions(&mut self.observer, &mut self.pool);
-            let snap = self.rumors.snapshot();
-            steady_state_op(
-                &mut self.rng,
-                &self.rumors,
-                &snap,
-                &mut self.pool,
-                target,
-                TEST_MESSAGE_SIZE,
-            );
-        }
-    }
-
-    fn live(&self) -> u64 {
-        self.rumors.snapshot().len() as u64
-    }
-}
-
-/// Reconcile two parties over an in-memory link, both ends on one
-/// current-thread runtime, as `bootstrap_fork` runs its halves.
-fn reconcile(runtime: &tokio::runtime::Runtime, a: &Rumors<Payload>, b: &Rumors<Payload>) {
-    let (mut la, mut lb) = rumors::link::memory_with_capacity(TEST_DUPLEX_CAPACITY);
-    let (ra, rb) = runtime.block_on(async { tokio::join!(a.gossip(&mut la), b.gossip(&mut lb)) });
-    ra.expect("gossip a");
-    rb.expect("gossip b");
-}
-
-/// The controller's fixed point is the target, and it survives retargeting.
-///
-/// Driving three gossiping parties through a target drop and a target raise
-/// must land each party's live count within half-to-double of every phase's
-/// target, even though every phase's redaction bursts fill each party's
-/// pool with entries the others already redacted. Three parties is the
-/// smallest swarm where those stale entries outpace the pool's drain (each
-/// party's draws must keep up with everyone's inserts), so a controller
-/// that burns its redact ops on stale entries stalls far above a lowered
-/// target here, while a two-party run would sit at the balance boundary
-/// and hide the defect.
-#[test]
-fn controller_converges_through_retargeting() {
-    let runtime = tokio::runtime::Builder::new_current_thread()
-        .build()
-        .expect("build test runtime");
-
-    // Seed shared content, then fork three disjoint parties from it, exactly
-    // as the swarm boots.
-    let seed: Rumors<Payload> = Peer::seed().into_rumors();
-    {
-        let mut rng = SmallRng::seed_from_u64(0x5eed);
-        seed.send_all((0..100).map(|_| random_message(&mut rng, TEST_MESSAGE_SIZE)))
-            .expect("flat test payloads are within any depth limit");
-    }
-    let mut parties = [
-        Party::new(
-            bootstrap_fork(&runtime, &seed, TEST_DUPLEX_CAPACITY),
-            0xa11ce,
-        ),
-        Party::new(bootstrap_fork(&runtime, &seed, TEST_DUPLEX_CAPACITY), 0xb0b),
-        Party::new(
-            bootstrap_fork(&runtime, &seed, TEST_DUPLEX_CAPACITY),
-            0xca201,
-        ),
-    ];
-    drop(seed);
-
-    // Three phases: settle at the seed-sized target, drop hard, raise hard.
-    // Each phase interleaves bursts of local churn with ring reconciliation
-    // every third round, the swarm's own rhythm at test scale; the judgment
-    // runs after the phase's full budget.
-    for (target, rounds) in [(100u64, 12), (20, 24), (200, 24)] {
-        for round in 0..rounds {
-            for party in &mut parties {
-                party.churn(target, 25);
-            }
-            if round % 3 == 0 {
-                reconcile(&runtime, &parties[0].rumors, &parties[1].rumors);
-                reconcile(&runtime, &parties[1].rumors, &parties[2].rumors);
-            }
-        }
-        // Judge the converged equilibrium, not one draw of it: the count
-        // oscillates round to round (churn bursts between syncs are large
-        // relative to a small target), so a single post-ring sample puts
-        // the band's edge inside the oscillation. Each sample is one
-        // churn round settled by a full ring pass — so every party holds
-        // the reconciled surviving set when read — and each party's mean
-        // over the samples is what must sit in band.
-        const SAMPLES: usize = 4;
-        let mut settled = [0u64; 3];
-        for _ in 0..SAMPLES {
-            for party in &mut parties {
-                party.churn(target, 25);
-            }
-            reconcile(&runtime, &parties[0].rumors, &parties[1].rumors);
-            reconcile(&runtime, &parties[1].rumors, &parties[2].rumors);
-            reconcile(&runtime, &parties[2].rumors, &parties[0].rumors);
-            for (sum, party) in settled.iter_mut().zip(&parties) {
-                *sum += party.live();
-            }
-        }
-        for sum in settled {
-            let live = sum / SAMPLES as u64;
-            assert!(
-                live >= target / 2 && live <= target * 2,
-                "controller failed to converge: mean live {live} vs target {target}",
-            );
-        }
-    }
-}
```

<!-- annotation -->
> **swarm-example-*** (T27), line 0:
>
> The example's single unit test (the controller's convergence trajectory; swarm-example-29, -30, and -31 all name it) goes with the example. It ran as a test target only because the [[example]] block set `test = true`; no other target shares it, it used no proptest, and no seed file under proptest-regressions/ anchors to it.

