# Partition benches-envelope: Criterion benches and their support, the window trade-off example, the envelope simulator

## Partition summary

This partition is the crate's measurement apparatus. Five Criterion targets sit under `benches/`: `branch_hash.rs` re-measures the feeding-strategy claim behind `Hash::branch`'s one-shot preimage; `in_memory.rs` sweeps the public single-set surface (batch insert, iteration, redaction, causal-delta ranges, both observer faces, point lookup) at three set sizes; `gossip_grid.rs` and `gossip_fixed.rs` reconcile diverged peers through `Rumors::gossip` over a persistent in-memory link, the first across a divergence cube and the second at fixed `N` with a link-latency axis; and `window_wallclock.rs` re-runs three cells of the window suites' virtual-time model on a running clock. Three `#[path]`-included support modules serve them: `support/grid.rs` (the divergence cube and its fixture builder), `support/wire.rs` (a pollster-driven `MemoryLink` pair and the bootstrap fork), and `support/latency.rs` (a delayed-pipe `Link` whose paused-clock runtime turns wire delay into an exact, load-independent hop count; seven `tests/` suites include it too). The two examples are `window_tradeoff.rs`, twelve lines that print `rumors::testing::window_tradeoff_table()` for the rustdoc table the gate byte-compares, and `envelope_sim.rs`, a 1,586-line port of the Python analysis behind the session window's occupancy envelopes, which `window.rs` cites as the certificate that its integer quantiles dominate the exact Chernoff tails.

All 3,337 lines are dev-side code: every file in the partition is bench, support, or example code, held to the test standard; none is production. I read every file with line numbers and corroborated the external anchors the findings rest on (`window.rs`, `hash.rs`, `budget/tests.rs`, `batch.rs`, `peer.rs`, `rumors.rs`, `snapshot.rs`, the pinned criterion 0.5.1 sources, the include sites, the recorded rationales in `.agent-notes/`, and the git provenance of the expiry commits).

The benches and their support are in good shape. `latency.rs` argues its measurement model from mechanism rather than asserting it, refuses to report a virtual figure on a wall-clock wire, and fails loudly off the delay lattice; `grid.rs` states its throughput denominator and enforces the redaction precondition at the one site that consumes it; every Criterion group keeps fixture construction untimed and warms lazy memos explicitly; `branch_hash.rs` prices the digest through the public `MERKLE_HASH_LEN` so the widening moved it automatically; and the trade-off table is a derived artifact with a `just` recipe, a temp-file-then-move write, and a byte-compare in the gate. The residue is small and concentrated in prose that outlived the code it described: a `join` bench deleted in June is still cited, a fixture-discipline section justifies a rebuild policy the benches do not follow with a fork hazard none of them can trigger, and a `Batch` contract is stated backwards. The structural findings are module-layout nits (wire nested under grid, two dead-code-allow conventions, duplicated helpers) and two denominator slips in `in_memory.rs`.

The envelope simulator carries the partition's substantive problems. Its constants say they mirror the crate but encode the 16-byte Merkle hash and the pre-CBOR target message size, so every byte-denominated table it prints describes a wire that does not ship. It pins a flat-solve baseline, labeled "landed" and "default 16 GiB", whose only referent is a branch that never survived. Its depth-0 prefix shift overflows a `u64`. And the dominance sweep that `window.rs` names as its certificate compares the simulator's private copy of the integer envelopes against the simulator's own oracle, never touches the shipped functions, never covers the pair-product generalization, and is run by no recipe, workflow, or test. The math family it certifies is sound and the transcription matches today; what is missing is any mechanical tie between the certificate and the code that claims it.

## Findings

### benches-envelope-1: branch_hash restates Hash::branch's preimage by hand, and nothing ties the copy to the shipped function
- Where: benches/branch_hash.rs:14-18 (related: benches/branch_hash.rs:57-73, src/tree/typed/hash.rs:75-79, src/tree/typed/hash.rs:162-213, src/tree/typed/hash.rs:167, Cargo.toml:145)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (field-by-field comparison of `contiguous` against `Hash::branch`: `BRANCH_TAG = 1`, one-byte prefix length, backfilled big-endian `u16` count, `1 + MERKLE_HASH_LEN` child records, `with_capacity(4 + …)`; `grep -rn branch_hash src tests` finds only the hash.rs:167 comment; `src/testing.rs` exposes no branch hash)
- Seen by: correctness, perfapi; refutation: confirmed; history: deliberate-and-holds (3e912ce8, for review ruling R30; the stated constraint "not public API" is true, but `rumors::testing` existed then and benches build with `test-internals`)
- Owner-gated: no (a `#[doc(hidden)]`, feature-gated `testing` addition is not public API)

hash.rs:167 cites this bench as the measurement behind the one-shot form, and the bench reproduces the preimage layout by hand. The two agree today, but a layout change in `Hash::branch` (count width, tag, field order) leaves the bench measuring a stale preimage while the production comment keeps citing it as evidence. Principle 8: a "measured by" claim is only as good as the instrument's fidelity, and fidelity that depends on two files being edited together is a convention, not a check. The `MERKLE_HASH_LEN` import protected the bench through the 16-to-24 widening; the other fields have no such protection.

Evidence:

    14	//! The layout is restated locally because the tree's hashing internals are
    15	//! not public API; it mirrors the preimage documented at `Hash::branch`,
    16	//! which the hash tests pin byte-for-byte. `contiguous` reproduces the
    17	//! shipped form including its per-call buffer allocation, so the measured
    18	//! difference is the end-to-end cost a caller sees, not the hash core alone.

Resolution: Expose the shipped assembly through `rumors::testing` (a `branch_hash(prefix, children) -> [u8; MERKLE_HASH_LEN]` shim over `Hash::branch`) and make `contiguous` call it, keeping `streamed` local as the alternative under test (it restates the layout by nature); or add a test in `tests/` that `#[path]`-includes the bench (the `latency_link.rs` pattern) and asserts `contiguous(prefix, kids)[..MERKLE_HASH_LEN]` equals the shipped digest for each `FANOUTS` entry. Rewrite lines 14-18 to match. Acceptance: a committed test fails when `contiguous` and `Hash::branch` disagree on any preimage byte for the swept fan-outs, or `contiguous` is the shipped code.
Construction: Change `BRANCH_TAG` in hash.rs to 2 and run the gate: the hash tests fail on the pinned layout, but nothing points at the bench, which keeps hashing tag 1 and keeps being cited at hash.rs:167.

### benches-envelope-2: The `V2` series label distinguishes one dialect, and two latency group names are never emitted
- Where: benches/gossip_fixed.rs:9-10 (related: benches/gossip_fixed.rs:107-114, benches/gossip_fixed.rs:154, benches/gossip_fixed.rs:168-171, benches/gossip_fixed.rs:190, src/protocol.rs:15-19)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 368da2a5 -- benches/gossip_fixed.rs` replaces `BenchmarkId::new(format!("{protocol:?}"), param)` over a `PROTOCOLS` array with the literal and adds the parenthetical in the same hunk; protocol.rs:15-19 has one variant; `sweeps` at 168-171 holds only the two `Bidir` scenarios; `grep -c gossip tools/benchjudge-expected.json` is 0)
- Seen by: structure, prose, perfapi; refutation: confirmed (owner-gated added); history: no-rationale-found for the label (planned at v1-retirement/README.md:240 without a reason); the two unreachable arms predate the retirement (818a8707 wrote the four-arm match and the two-element sweep together), so only the label is retirement residue
- Owner-gated: yes (Criterion series ids name any locally saved baselines; renaming breaks comparison against them)

With `Protocol` a single-variant enum, the `V2` segment in every series id partitions nothing, and the doc's parenthetical describes a comparison that cannot be made. Separately, `Scenario::latency_group_name` has arms for `gossip_latency_unilateral_insertions` and `gossip_latency_unilateral_redactions`, but `bench_gossip_latency` sweeps only the two bidirectional scenarios and the module doc lists only those two groups, so the arms produce names no group is registered under. Principle 3: machinery outlives the constraint that justified it; a match arm whose output no path produces is prose about a sweep that does not exist.

Evidence:

     9	//! The four Criterion groups measure the wire protocol on the same
    10	//! fixtures (each series is labeled `V2`, the dialect it measures):

   107	    fn latency_group_name(self) -> &'static str {
   108	        match self {
   109	            Scenario::BidirInsertions => "gossip_latency_bidir_insertions",
   110	            Scenario::BidirRedactions => "gossip_latency_bidir_redactions",
   111	            Scenario::UnilateralInsertions => "gossip_latency_unilateral_insertions",
   112	            Scenario::UnilateralRedactions => "gossip_latency_unilateral_redactions",
   113	        }
   114	    }

Resolution: Use `BenchmarkId::from_parameter(param)` at 154 and `BenchmarkId::new(format!("divergence={param}"), latency_ms)` at 190, drop the parenthetical at 10, and note the baseline discontinuity in the commit; if the slot is deliberately reserved for a future dialect, say so in one sentence instead. Make the latency sweep table carry its group name directly (`(Scenario, &str, &[usize])`) and delete `latency_group_name`, or sweep the unilateral scenarios under latency and list them in the module doc. Acceptance: `grep -n '"V2' benches/` returns nothing (or the doc states the reservation); no function in the file returns a group name that no group is registered under.

### benches-envelope-3: The identical corner reports a zero-element throughput
- Where: benches/gossip_fixed.rs:152-153 (related: benches/gossip_grid.rs:60-72)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (criterion 0.5.1 `measurement.rs:152-153` computes `elems * (1e9 / typical)` with no zero guard and renders " elem/s")
- Seen by: correctness; refutation: confirmed (version corrected from 0.8.2 to the pinned 0.5.1); history: no-rationale-found (present since 5a60fb3e3a; `gossip_grid` split the corner into a latency-only group from birth)
- Owner-gated: no

The sweep starts at `param = 0` and sets `Throughput::Elements(0)`, so the identical corner prints "0 elem/s" on the group's throughput plots: a datum that measures nothing. `gossip_grid.rs:60-72` handles the same corner by reporting latency only. Denominate precisely: the two benches disagree on how the corner is denominated.

Evidence:

   152	        for param in (0..=scenario.max_param()).step_by(scenario.step()) {
   153	            group.throughput(Throughput::Elements(param as u64));

Resolution: Skip `group.throughput` when `param == 0`, or split the corner into its own latency-only group as `gossip_grid` does. Acceptance: no Criterion group in the partition reports a zero-element throughput.

### benches-envelope-4: `window` carries three senses inside gossip_fixed.rs
- Where: benches/gossip_fixed.rs:175-180 (related: benches/gossip_fixed.rs:36-39, benches/gossip_fixed.rs:81-87, benches/support/latency.rs:18-20, benches/support/latency.rs:58-62)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -n -i window` over both files)
- Seen by: prose; refutation: reframed (the "receive window" sense at latency.rs:58-59 is an established networking term; the problem is the three senses meeting inside one file); history: deliberate-and-holds for "receive window" (latency.rs:58-59 and the sync-budget design record use the term)
- Owner-gated: no

Within forty lines the bare word names the crate's session window (38, the thing the slope-reading instructions are about), the link's in-flight byte capacity (81, 85), and Criterion's warm-up and measurement times (177); latency.rs:20's "window stalls" is unqualified. `window` is a load-bearing crate term (`WindowConfig`, `sync_memory_budget`, the window suites), and this partition exists partly to measure it. "Receive window" stays: it is the established term for the link capacity.

Evidence:

   175	        // The virtual component is deterministic and the wall component is
   176	        // the same magnitude the fixed groups already sample heavily, so a
   177	        // small sample count and short windows suffice. The windows bound
   178	        // *reported* (largely virtual) time, while the real wall cost is
   179	        // the untimed fixture rebuild every iteration pays — keeping them
   180	        // tight is what keeps this group's wall time in check.

Resolution: At 177 write "short warm-up and measurement times suffice. Those times bound…"; at 81 "Per-stream in-flight capacity (the link's receive window)"; at latency.rs:20 "plus any stalls on the pipe capacity". Acceptance: the bare word `window` in gossip_fixed.rs and latency.rs means the session window, or is qualified as "receive window".

### benches-envelope-5: seeded_with_messages is seeded_with_versions minus the harvest
- Where: benches/gossip_fixed.rs:282-297 (related: benches/gossip_fixed.rs:215, benches/gossip_fixed.rs:237)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read both bodies; they differ only by line 295)
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (the two were distinct fixtures while `seeded_with_keys` returned batch-minted keys; 9c73d7b463 replaced that with a snapshot harvest and left both)
- Owner-gated: no

Two names for one fixture; the harvest is cheap at `N = 10_000`.

Evidence:

   282	fn seeded_with_messages(n: usize, seed: u64) -> Rumors<u8> {
   283	    let rumors = production_seed();
   284	    rumors
   285	        .send_all(random_bytes(n, seed))
   286	        .expect("flat test payloads are within any depth limit");
   287	    rumors
   288	}
   289	
   290	fn seeded_with_versions(n: usize, seed: u64) -> (Rumors<u8>, Vec<Version>) {
   291	    let rumors = production_seed();
   292	    rumors
   293	        .send_all(random_bytes(n, seed))
   294	        .expect("flat test payloads are within any depth limit");
   295	    let versions = rumors.snapshot().iter().map(|(v, _)| v.clone()).collect();
   296	    (rumors, versions)
   297	}

Resolution: Keep `seeded_with_versions` and have the insertion builders take `.0`, or keep `seeded_with_messages` and add a `versions_of(&Rumors<u8>) -> Vec<Version>` helper that the redaction builders call. Acceptance: one seeding helper in the file.

### benches-envelope-6: warm_caches is doc(hidden) but unconditionally public on Rumors, Peer, and Snapshot; its only callers are benches and tests
- Where: benches/gossip_fixed.rs:299-303 (related: src/rumors.rs:413-414, src/peer.rs:713-714, src/snapshot.rs:166-167, src/tree.rs:306-307, src/peer.rs:480-483, src/rumors.rs:420-422, Cargo.toml:145)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -rn -B2 'pub fn warm_caches' src` shows `#[doc(hidden)]` with no `cfg` at all four sites; `sync_window_floor` and `dangerously_alias_party` carry `#[cfg(any(test, feature = "test-internals"))]`; `grep -rn warm_caches` finds callers only in benches/ and src/tree/tests.rs; Cargo.toml:145 gives benches the feature)
- Seen by: perfapi; refutation: confirmed; history: deliberate-but-expired (611b325d added it when no `test-internals` feature existed; the feature arrived at 9eadfc680 and the gating convention at db2718d46, and `warm_caches` was not revisited)
- Owner-gated: yes (narrows the public surface)

The bench helper calls a method that is documented "For benchmark and test calibration only" yet ships unconditionally on the production handles. Its neighbours with the same purpose are feature-gated. Hidden items are still semver surface and still reachable; the crate's own convention for calibration-only methods is the feature gate, and this one is the odd one out. Benches already build with `test-internals`, so gating costs them nothing.

Evidence:

   299	fn warmed((left, right): (Rumors<u8>, Rumors<u8>)) -> (Rumors<u8>, Rumors<u8>) {
   300	    left.warm_caches();
   301	    right.warm_caches();
   302	    (left, right)
   303	}

Resolution: Add `#[cfg(any(test, feature = "test-internals"))]` to the three public `warm_caches` methods (and `Tree::warm_caches`, whose module is private), matching `sync_window_floor`. Acceptance: `cargo doc` and `cargo check` without features show no `warm_caches` on the public types; `cargo check --all-targets` still builds the benches.

### benches-envelope-7: A stray one-word doc line
- Where: benches/gossip_grid.rs:14-16
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (a partial re-wrap in the WIP commit 83edcd9441)
- Owner-gated: no

The word "The" sits alone between two wrapped lines. It renders fine and reads as a typo in source.

Evidence:

    14	//! A naive harness would allocate a fresh transport per Criterion iteration.
    15	//! The
    16	//! gossip exchange chain is statically bounded and self-delimiting (it closes

Resolution: Re-wrap the paragraph. Acceptance: no doc line in the file consists of a single word.

### benches-envelope-8: gossip_grid cites an in_memory.rs `join` bench that does not exist
- Where: benches/gossip_grid.rs:33-38 (related: benches/support/grid.rs:86-95, benches/in_memory.rs:1-5)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (I read all of in_memory.rs: no `join`; `git show ac68e8121^:benches/in_memory.rs` lists `join_grid` / `join_identical`; ac68e8121's message says the file "loses its join benches")
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (accurate at db5a3bbb80; ac68e8121 on 2026-06-10 removed the join benches without touching this sentence, and the later ghost-reference sweeps missed it)
- Owner-gated: no

The doc anchors its grid and throughput accounting to a bench that has not existed since June. A reader sent to compare the two finds nothing. AGENTS.md hard rule: nothing in the codebase refers to code that no longer exists. The accounting the sentence wants to name is stated in full at `grid.rs:86-95` (`Cell::divergence`), so the pointer to `[grid]` already carries it.

Evidence:

    33	/// `gossip` across the divergence grid.
    34	///
    35	/// The grid and throughput accounting mirror `in_memory.rs`'s `join` bench
    36	/// exactly (see [`grid`]); the only difference is that each pair reconciles
    37	/// over the wire via [`Wire::round_trip`] rather than in-process. The identical
    38	/// corner reports latency in a separate `gossip_identical` group.

Resolution: Re-state against what exists: "Throughput is charged against each cell's divergence and the sample count follows its build magnitude (see [`grid`]); the identical corner reconciles nothing and reports latency in a separate `gossip_identical` group." Acceptance: `grep -n join benches/gossip_grid.rs` returns nothing; the doc names only benches and helpers that exist.

### benches-envelope-9: "batches commit on drop" states the Batch contract backwards
- Where: benches/in_memory.rs:10-12 (related: src/batch.rs:13-18)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (batch.rs:15-17: "the batch commits — atomically, as one commit — exactly when the closure returns `Ok`. Any other exit (a returned `Err`, a panic) commits nothing"; `grep -n 'impl.*Drop\|fn commit' src/batch.rs` shows no `Drop` impl and a `pub(crate) fn commit(self)`; the file never calls `batch(`)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (true at c5f1210a39 when `Batch` committed in `Drop`; ce27df86 deleted that impl and made the lifecycle commit-on-`Ok`; 212c6914 then moved the benches to `send_all`/`redact_all`)
- Owner-gated: no

The parenthetical inverts a public atomicity contract the user-facing docs state carefully: dropping is the path that commits nothing. The benches also never use `batch`; they call `send_all` and `redact_all`, which are single synchronous commits in their own right.

Evidence:

    10	//! The handles here are the asynchronous [`rumors::Rumors`] and its message
    11	//! observers: every operation measured is synchronous on that surface
    12	//! (batches commit on drop), so no runtime is involved.

Resolution: Replace the parenthetical with what the benches do ("`send_all` and `redact_all` are synchronous single commits") or drop it; the sentence's point is only that no runtime is needed. Acceptance: the sentence no longer mentions drop and names the operations the benches invoke.

### benches-envelope-10: The "Fixture discipline" section describes a hazard the binary cannot trigger and a rebuild policy it does not follow
- Where: benches/in_memory.rs:14-21 (related: benches/in_memory.rs:86-90, benches/in_memory.rs:97-101, benches/in_memory.rs:117, benches/in_memory.rs:144, benches/in_memory.rs:184, benches/in_memory.rs:213, benches/in_memory.rs:239, benches/in_memory.rs:277, benches/in_memory.rs:301, benches/in_memory.rs:325, benches/support/wire.rs:39-56)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (the only `fork` in the file is doc line 17; no `wire::`, `bootstrap_fork`, or `.fork(`; bench bodies read: `batch_insert` seeds inside the timed closure at 97-101, `iter`/`observer_replay`/`causal_replay`/`get` build once per size at 117/213/277/325, the three `*_delta` groups once per `(n, delta)` at 184/239/301, and only `redact` rebuilds per iteration at 144)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (10f1e5166c wrote "rebuilt from a fresh seed in untimed setup and forked at most once" for the join benches, which forked a party per fixture; ac68e8121 deleted the join benches and the "forked at most once" clause but kept the fork hazard as the premise)
- Owner-gated: no

The section justifies fresh-seed fixtures by `before`'s fork-depth hazard, but nothing in this binary forks a party, so the hazard cannot arise (Principle 3: a rationale that names no failure it prevents is the circular-justification tell). It then asserts "every fixture is rebuilt from a fresh `Peer::seed` in untimed setup", which eight of nine benches contradict: seven build once per size and share the set across iterations, `batch_insert` seeds inside the timed body, and only `redact` rebuilds per iteration. The per-bench docs already state each fixture's real lifecycle, so the section competes with accurate prose.

Evidence:

    14	//! # Fixture discipline
    15	//!
    16	//! Inserting a message ticks an Interval Tree Clock party, and `before`
    17	//! documents that repeatedly [`fork`](before::Party::fork)ing *the same*
    18	//! party deepens its id tree linearly (worse memory and per-op cost). To
    19	//! keep that out of the measurements, every fixture is rebuilt from a fresh
    20	//! [`Peer::seed`](rumors::Peer::seed) in untimed setup: no party
    21	//! accumulates depth across Criterion iterations.

Resolution: Delete the section (the per-bench docs at 86-90, 107-111, 133-136, 156-158 carry the true fixture story), or replace it with what is: read-only benches build one set per size in untimed setup and share it; the consuming `redact` rebuilds per iteration with `BatchSize::PerIteration`; `batch_insert` builds inside its timed body. If the fork-depth note is wanted anywhere, it belongs beside `bootstrap_fork` in `support/wire.rs`, the one place a fork happens. Acceptance: no sentence in the module doc claims a rebuild policy a bench body contradicts; `grep -n fork benches/in_memory.rs` returns nothing or points at a real fork site.

### benches-envelope-11: in_memory.rs re-declares grid::send_units, twins drain for two observer types, qualifies an imported name, and harvests versions for callers that discard them
- Where: benches/in_memory.rs:59-64 (related: benches/support/grid.rs:56-63, benches/in_memory.rs:54, benches/in_memory.rs:68-73, benches/in_memory.rs:77-84, benches/in_memory.rs:256-263, benches/in_memory.rs:159, benches/in_memory.rs:46, src/rumors/unordered.rs:191, src/rumors/causal.rs:151)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both `send_units` bodies read; both drain bodies read; `impl<T: Send + Sync + 'static> Stream for` at unordered.rs:191 and causal.rs:151; line 46 imports `Version`, line 159 writes `rumors::Version`; `build` is called at 117, 144, 213, 277, 325 and its versions are read only at 144 and 325)
- Seen by: structure, prose, perfapi; refutation: confirmed; history: no-rationale-found for the `send_units` copy (ac68e8121 dropped a `use grid::{…, units}` import for a local helper); the twin drains are deliberate-but-expired (both called the type-specific lending `borrow_next()` until cd7c09db33 made `next()` the one engine)
- Owner-gated: no

`send_units` (59-64) is byte-identical to `grid::send_units` while line 54 already imports from `grid`; the "one CBOR null byte" rationale is stated at in_memory.rs:6-8 and grid.rs:56-58 both. `drain` and `drain_causal` have identical bodies over two types that both implement `Stream`. `rumors::Version` at 159 is spelled out although `Version` is imported. `build` collects every live version for five callers, of which only `redact` and `get` read them (a one-million-element collect at the top size, untimed but pointless). Doctrine: one definition per helper; imports over qualified paths; no work no caller consumes.

Evidence:

    59	/// Commit `n` unit payloads to `rumors` as one batch.
    60	fn send_units(rumors: &Rumors<()>, n: usize) {
    61	    rumors
    62	        .send_all(iter::repeat_n((), n))
    63	        .expect("flat test payloads are within any depth limit");
    64	}

Resolution: `use grid::{SIZES, sample_size_for, send_units};` and delete the local copy and the `std::iter` import, keeping the CBOR-null rationale once at grid.rs:56-58; replace the two drains with one `fn drain<S: Stream + Unpin>(observer: &mut S) -> usize`; write `Version` at 159; split `build` into `build(n) -> Rumors<()>` and `versions_of(&Rumors<()>) -> Vec<Version>` so only `redact` and `get` pay the harvest. Acceptance: `grep -c 'fn send_units\|fn drain' benches/in_memory.rs` is 1; no `rumors::Version` remains in the file; `build` returns no value its caller drops unread.

### benches-envelope-12: batch_insert's timed body includes dropping the N-node tree and an OsRng draw
- Where: benches/in_memory.rs:88-101 (related: benches/in_memory.rs:143-150, src/peer.rs:206-208, src/peer.rs:212-213)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (Cargo.lock pins criterion 0.5.1; its `bencher.rs:80-92` `iter` runs `black_box(routine())` in the loop and calls `measurement.end` afterwards, so the returned `Rumors` drops inside the timed interval; `iter_batched` at 247-257 with batch size 1 calls `end` before `drop(black_box(output))`, so `redact` excludes its drop; `iter_with_large_drop` at 185-190 is `iter_batched(…, SmallInput)`; peer.rs:206-208 `seed()` is `Self::seed_rng(&mut OsRng)`)
- Seen by: correctness, perfapi (the OsRng half); refutation: confirmed (criterion version corrected; the perfapi magnitude estimate dropped); history: deliberate-and-holds for the `b.iter` shape (10f1e5166c chose it for peak memory), which never addressed the denominator
- Owner-gated: no

`b.iter` returns the built set, so each iteration's deallocation of the tree (one million `Arc` nodes at the top size) is inside the measured interval, while `redact` uses `iter_batched` and excludes its drop; the two groups are denominated differently while both docs present a per-element cost of one operation. The doc names the seed as the only extra cost and omits the larger one. The seed also draws sixteen bytes from `OsRng` per iteration, a syscall the doc calls negligible without a bound. Denominate precisely.

Evidence:

    88	/// `b.iter` builds and drops one set per iteration, so peak memory stays at a
    89	/// single tree even at N = 1M. The trivial `seed().into_rumors()` is inside
    90	/// the timed body, but its cost is negligible against N inserts.

    97	            b.iter(|| {
    98	                let rumors: Rumors<()> = Peer::seed().into_rumors();
    99	                send_units(&rumors, black_box(n));
   100	                rumors
   101	            })

Resolution: `b.iter_batched(|| Peer::seed().into_rumors(), |rumors| { send_units(&rumors, black_box(n)); rumors }, BatchSize::PerIteration)`: one tree alive at a time (the doc's stated constraint), the seed and its syscall untimed, and the drop after `end`. Do not use `iter_with_large_drop`, whose `SmallInput` batching keeps several trees alive. State in the doc what the timed body contains. Acceptance: the `batch_insert` timed body ends before the tree is dropped and contains no `OsRng` draw; the doc names the denominator.

### benches-envelope-13: The version-bounds pruning the benches advertise is enforced by no committed instrument
- Where: benches/in_memory.rs:170-174 (related: benches/in_memory.rs:225-229, benches/in_memory.rs:289-291, src/tree/typed/untyped/iter.rs:221-228, src/tree/tests.rs:872-884, src/tree.rs:616-622, src/tree/typed/untyped.rs:39-50)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (absence: `grep -n 'cfg(test)' src/tree/typed/untyped/iter.rs` is empty; `grep -rln 'visited\|nodes_touched\|descents\|census' src tests benches` hits only the node-handle residency census (untyped.rs:39-50, tests/window_census.rs:1-16), which counts live handles, not walk visits; tree/tests.rs:880 says the prune/promote shortcuts "are pure optimization" and the proptest compares yield only; the meter pattern exists at tree.rs:616)
- Seen by: correctness; refutation: confirmed (adds that the `Throughput::Elements(delta)` denominator is exact because `causally::since` excludes its argument); history: no-rationale-found (ac68e8121 placed the claim as a bench measurement; no note considers a committed visit counter)
- Owner-gated: no (a test-only meter); the resolution lands in `src/tree`, another partition's files

`range_delta`, `observer_delta`, and `causal_delta` state that a small delta against a large snapshot costs "the delta plus the pruning frontier, not the tree", the same property iter.rs:224-228 documents. The only committed check of the range walk is a differential proptest that compares yield against the naive filter; a `range` degraded to a full scan with a per-leaf filter passes every test, and only a human reading Criterion output would notice. Principle 2: a claimed performance property must move a committed number; an oracle that agrees with the wrong implementation is a blind spot.

Evidence:

   170	/// Throughput is charged against the delta, not the set size: the
   171	/// memoized version bounds let the walk prune everything the checkpoint
   172	/// dominates, so a small delta against a large snapshot should cost the
   173	/// delta plus the pruning frontier, not the tree. Comparing one column
   174	/// (fixed delta) across set sizes is exactly that claim under measurement.

Resolution: Add a `cfg(test)` visit counter to the query walk in `src/tree/typed/untyped/iter.rs` (the root-hash meter at tree.rs:616 is the pattern), and a committed test on a constructed shape (large `N`, `D` much smaller than `N`, a checkpoint dominating all but the last `D` sends) asserting nodes visited is at most `c · (D + frontier)` with a liveness floor (at least `D` leaves visited). Then the bench docs cite the test as the enforced claim. Acceptance: a committed test fails when the coverage prune in the walk is disabled and passes at HEAD; the three bench docs point at it.
Construction: In iter.rs, force the query walk's `coverage` verdict to "descend" (never prune) and run the tree and integration suites: every committed test passes; only the `range_delta` and `observer_delta` Criterion columns move.

### benches-envelope-14: "the generator's latency-only regime" has no referent
- Where: benches/window_wallclock.rs:27-29 (related: benches/window_wallclock.rs:3-5, tests/window_knee.rs:53-55, tests/window_knee.rs:257)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -c generator` is 0 in tests/window_knee.rs, tests/window_operator.rs, and tests/common/window.rs; window_knee.rs:55 `const LINK_CAPACITY: usize = 8 * 1024 * 1024;` "far above this test's transfers"; :257 "latency-only link (roomy pipe)")
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired ("the generator" was `examples/window_tradeoff.rs` at 1988de6d, which ran real sessions at this capacity; 4d0b3db22 made it pure arithmetic and re-pointed this bench's module doc at the knee and operator suites without touching line 27)
- Owner-gated: no

The comment justifies the constant by a "generator" the suites named at lines 3-5 do not contain; the crate's test generator (`tests/common/window.rs`) generates per-peer window choices, not link capacities. The regime the constant matches is the knee suite's own `LINK_CAPACITY`. Expand references, not vocabulary: a compressed pointer to context the reader does not hold is a comment that cannot be followed.

Evidence:

    27	/// Per-stream pipe buffering: roomy, matching the generator's
    28	/// latency-only regime.
    29	const LINK_CAPACITY: usize = 8 * 1024 * 1024;

Resolution: "Per-stream pipe buffering: roomy enough that the pipe never binds, the latency-only regime `tests/window_knee.rs` measures in (its `LINK_CAPACITY`)." Acceptance: the comment names a file and constant that exist.

### benches-envelope-15: window_wallclock builds the tokio runtime and link pair inside the timed routine
- Where: benches/window_wallclock.rs:50-56 (related: benches/support/latency.rs:369-373, benches/support/latency.rs:405-418, benches/gossip_fixed.rs:187)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (criterion 0.5.1 `bencher.rs:251-253` brackets only `routine(input)`; `with_clock` at latency.rs:405-418 builds a current-thread runtime with a time driver plus two pipes and two channels; gossip_fixed.rs:187 hoists its `DelayedWire` outside `bench_function`)
- Seen by: correctness, perfapi; refutation: confirmed; history: no-rationale-found (1988de6d; the sibling sweep hoisted from birth)
- Owner-gated: no

Each sample includes runtime construction and teardown that the virtual model it cross-checks does not contain. Tens of microseconds against cells of tens of milliseconds sits inside the "few percent" the module doc tolerates, but the bench exists to agree with a model, and `DelayedWire`'s own doc describes "one link pair reused at clean session boundaries".

Evidence:

    50	            bencher.iter_batched(
    51	                || diverged(budget, divergence),
    52	                |(left, right)| {
    53	                    let mut wire = latency::DelayedWire::new_wall_clock(LINK_CAPACITY, DELAY);
    54	                    wire.round_trip(left, right)
    55	                },
    56	                criterion::BatchSize::PerIteration,

Resolution: Hoist `let mut wire = latency::DelayedWire::new_wall_clock(LINK_CAPACITY, DELAY);` above `group.bench_function` (one per cell) and write the routine as `|(left, right)| wire.round_trip(left, right)`, as gossip_fixed does. Acceptance: no runtime construction inside the routine closure.

### benches-envelope-16: grid.rs owns wire.rs, so one bench re-implements the bootstrap fork and another includes the grid under a stale comment
- Where: benches/support/grid.rs:34-35 (related: benches/gossip_fixed.rs:50-54, benches/gossip_fixed.rs:141, benches/gossip_fixed.rs:216, benches/gossip_fixed.rs:238, benches/gossip_fixed.rs:254, benches/gossip_fixed.rs:268, benches/window_wallclock.rs:63-86, benches/support/wire.rs:39-56)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grid.rs:34-35 is the only `#[path]` include of wire.rs; gossip_fixed.rs reaches `grid::wire::Wire::new()` at 141 and `grid::wire::bootstrap_fork` at 216, 238, 254, 268; window_wallclock.rs:69-81 is wire.rs:44-55's `tokio::join!` bootstrap shape plus `.sync_memory_budget(budget)`)
- Seen by: structure, prose (the comment half), perfapi (the duplicate fork); refutation: confirmed; history: no-rationale-found (the nesting arrived in the WIP commit 83edcd9441 with no layout discussion; the gossip_fixed comment was accurate at 5a60fb3e3a and expired in the same commit)
- Owner-gated: no

`wire.rs` is reachable only as `grid::wire`. `window_wallclock.rs` needs a budget-configured bootstrap fork and no grid, so `diverged` (63-86) re-implements `wire::bootstrap_fork` inline, differing only in the `.sync_memory_budget(budget)` call. `gossip_fixed.rs` includes the grid under a comment saying it "only needs its sample-size policy", then reaches the wire harness through it at five sites. Fixtures and harness are separate concerns nested by accident of `#[path]` inclusion, and the cost is duplicated logic plus a comment contradicted by the code beneath it.

Evidence:

    34	#[path = "wire.rs"]
    35	pub mod wire;

    50	// The shared grid module exposes a superset of helpers; this bench only needs
    51	// its sample-size policy so fixed-N runs line up with the existing benches.

Resolution: Add `benches/support/mod.rs` declaring `pub mod grid; pub mod wire; pub mod latency;`, have each bench write `#[allow(dead_code)] mod support;` (a bench file is a crate root, so no `#[path]` is needed), and let `grid.rs` use `super::wire`. Have `bootstrap_fork` return the joined `Peer<T>` (callers finish with `.into_rumors()` or `.sync_memory_budget(b).into_rumors()`) and replace `window_wallclock::diverged`'s inline block with it. Fix the gossip_fixed comment to the siblings' generic wording. The tests' `#[path = "../benches/support/latency.rs"]` keep working because latency.rs imports no sibling. Acceptance: no bench contains a `tokio::join!(… .gossip(…), Peer::<_>::bootstrap().join(…))` block outside `support/wire.rs`; `gossip_fixed.rs` has no comment claiming to need only the sample-size policy.

### benches-envelope-17: Hand-maintained counts beside the arrays they count, one of them wrong
- Where: benches/support/grid.rs:37-40 (related: benches/window_wallclock.rs:34, benches/gossip_fixed.rs:9, benches/gossip_fixed.rs:67-69)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read; 100 to 1,000,000 spans four decades)
- Seen by: prose; refutation: confirmed; history: no-rationale-found ("three cells over two budgets" transcribed review ruling R77's own count; the others have no discussion)
- Owner-gated: no

`SIZES = {100, 10_000, 1_000_000}` is described as "spanning three orders of magnitude"; 10² to 10⁶ is four. The tallies at window_wallclock.rs:34 ("three cells over two budgets"), gossip_fixed.rs:9 ("The four Criterion groups"), and gossip_fixed.rs:68-69 ("three nonzero decades") are correct today and rot the moment a cell is added. Doctrine: state the structure, not the tally.

Evidence:

    37	/// Live message counts for the single-set benchmarks (`batch_insert`,
    38	/// `iter`, `redact`, `range_delta`, …), spanning three orders of magnitude.
    39	#[allow(unused)]
    40	pub const SIZES: &[usize] = &[100, 10_000, 1_000_000];

Resolution: "sizes two decades apart, 10² to 10⁶"; "one cell per (budget, divergence) pair"; "One Criterion group per scenario"; "the nonzero decades pin the slope". (The `#[allow(unused)]` at 39 is addressed in benches-envelope-22.) Acceptance: no numeral in these comments restates the length of the array beneath it.

### benches-envelope-18: grid::build harvests the shared prefix's versions for every cell though only redaction cells consume them
- Where: benches/support/grid.rs:156-172 (related: benches/support/grid.rs:42-44, benches/support/grid.rs:54, benches/support/grid.rs:120-137)
- Class / severity / confidence: performance / low / high
- Provenance: verified (`shared` is collected at 159 unconditionally and read only at 170-171 under `if redacted > 0` at 165; `REDACTED[0] == 0` at 54; `cells()` admits `redacted = 0` for every `(common, differing)`; the module doc at 42-44 names fixture building as the wall-time bottleneck)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the collect predates 9c73d7b463, which turned each element from a `Copy` key into a `Version` clone plus `Arc` traffic without moving it under the branch)
- Owner-gated: no

For every zero-redaction cell (the whole `REDACTED[0]` column plus every cell the `common < 2 * redacted` filter empties), the harvest is dead work: up to 100,000 `Version` clones, each paying the `Arc<dyn Any>` clone, downcast, and drop of `Snapshot::iter`, per fixture build, per Criterion iteration, in the stage the module calls its bottleneck. Untimed setup still costs wall time; deleting redundant work has a fixed sign.

Evidence:

   156	    // The shared prefix's versions, for carving the redaction blocks; order
   157	    // is immaterial (the blocks only need to be disjoint and deterministic,
   158	    // and the snapshot iterates in a stable order).
   159	    let shared: Vec<Version> = left.snapshot().iter().map(|(v, _)| v.clone()).collect();
   160	
   161	    let right = wire::bootstrap_fork(&left);
   162	    send_units(&left, differing);
   163	    send_units(&right, differing);
   164	
   165	    if redacted > 0 {

Resolution: `let shared = (redacted > 0).then(|| left.snapshot().iter().map(|(v, _)| v.clone()).collect::<Vec<Version>>());` at the same position (it must precede the post-fork sends at 162-163), and read it inside the branch. Acceptance: non-redaction cells perform no snapshot iteration in setup; cell ids and measured bodies unchanged.

### benches-envelope-19: Harvesting versions from a Snapshot pays an Arc clone, downcast, and drop per element; no versions-only enumeration exists
- Where: benches/support/grid.rs:159 (related: benches/in_memory.rs:71, benches/gossip_fixed.rs:295, src/rumors.rs:269-274, src/snapshot.rs:105-112, src/tree.rs:178-183, src/message.rs:513-518)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: verified (snapshot.rs's public methods are `network`, `latest`, `earliest`, `is_empty`, `len`, `hash`, `get`, `iter`, `range`, `warm_caches`; `iter` yields `(&Version, Arc<T>)` through tree.rs:182 `m.arc::<T>()`, which is message.rs:514-516 `self.message.clone().downcast::<T>()`; the inner `typed::Iter` yields `(&Version, &Message)`)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (9c73d7b463 retargeted the surface from `Key` to `Version` and replaced batch-minted key lists with snapshot walks in every fixture; no note considers a borrowed versions iterator)
- Owner-gated: yes (public API addition)

Three bench fixtures and the `redact_all` doc example at rumors.rs:269-274 enumerate versions with `snapshot().iter().map(|(v, _)| v.clone())`, and each item pays two atomic operations and a `TypeId` compare for a payload nobody reads. A user of a replicated set would expect membership and version enumeration without touching payloads, and the underlying iterator already has the borrowed form. The `iter` bench (in_memory.rs:107-131) currently measures this refcount traffic as "the walk itself".

Evidence:

   159	    let shared: Vec<Version> = left.snapshot().iter().map(|(v, _)| v.clone()).collect();

Resolution: Propose `Snapshot::versions(&self) -> impl DoubleEndedIterator<Item = &Version> + ExactSizeIterator` over `typed::Iter` (no `T` bound), and consider `Snapshot::contains(&Version) -> bool` beside `get`. Leave `iter()`'s owned-`Arc<T>` shape alone: it is a deliberate trade (the handle outlives the snapshot), and `&Arc<T>` is unavailable because storage is `Arc<dyn Any>`. Acceptance: the three bench sites and the doc example use `versions()`; if a `versions` bench group is added, its column against `iter` shows the payload-handle cost.

### benches-envelope-20: latency.rs bundles the pipe, the link adapter, and the measurement harness; its includers already name the seam
- Where: benches/support/latency.rs:1-8 (related: benches/support/latency.rs:88-288, benches/support/latency.rs:290-367, benches/support/latency.rs:369-553, tests/hop_trace.rs:26-28, tests/window_knee.rs:17-19)
- Class / severity / confidence: modularity / nit / medium
- Provenance: assessed (read; the three layers and the includers' comments)
- Seen by: structure; refutation: reframed (the claimed payoff, fewer dead-code allows, does not materialize because includers use different subsets within each layer; the payoff is legibility of each `#[path]` line); history: no-rationale-found (the module grew in place; no commit records considering a split)
- Owner-gated: no

The module holds the delayed pipe (88-288), the link adapter (290-367), and the measurement harness (369-553). `hop_trace.rs:26` says "Only the pipe layer is reused"; the window suites say "Only the delayed wire is exercised here". A two-file split along that seam would make each includer's intent legible from its `#[path]` line and retire those explanatory comments. Every includer would still carry a blanket `#[allow(dead_code)]`, so this is a design proposal whose payoff is legibility, not fewer attributes.

Evidence:

     1	//! A latency-injecting in-memory link: the wire-delay knob for benchmarks.
     2	//!
     3	//! [`delayed_pair`] mirrors the topology of [`rumors::link::memory`] — a
     4	//! bidirectional control stream plus announced unidirectional data streams —
     5	//! but builds every stream from a *delayed pipe*: bytes written at instant
     6	//! `t` become readable at `t + delay`, under a byte-bounded in-flight
     7	//! window. `delay` is the link's one-way latency, so a blocking
     8	//! request/response exchange pays `2 * delay`.

Resolution: If taken up, split into `support/delayed_pipe.rs` (pipe, adapter, `delayed_pair`) and `support/delayed_wire.rs` (`DelayedWire`, `session_hops`, `hops_on_lattice`, including the former), moving the measurement-model prose with the harness, and update the nine `#[path]` lines. Acceptance: `tests/hop_trace.rs` includes only the pipe file; the window suites include only the wire file; the per-includer "only the … is used here" comments are deleted.

### benches-envelope-21: The wall component is described as CPU time but measured as elapsed time
- Where: benches/support/latency.rs:32-34 (related: benches/support/latency.rs:436-446)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`round_trip` measures `wall_start.elapsed()` at 436-439)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (a wording slip in 814f07ad's rewrite)
- Owner-gated: no

`std::time::Instant::elapsed` is elapsed time; under load it exceeds CPU time, and the same bullet's "It moves with machine load" describes elapsed time, not CPU time. This is the one place the module's argument distinguishes the load-dependent figure from the load-independent one.

Evidence:

    32	//! - the *wall* component: real CPU time spent computing — both peers
    33	//!   serialized on one thread, the same convention as the zero-latency
    34	//!   harness in [`wire.rs`](wire.rs). It moves with machine load.

Resolution: "real elapsed time spent computing". Acceptance: the bullet's noun matches the quantity `round_trip` measures.

### benches-envelope-22: Two dead-code conventions for the shared bench modules, one of them redundant everywhere it appears
- Where: benches/support/latency.rs:398-400 (related: benches/support/latency.rs:463-466, benches/support/latency.rs:511-514, benches/support/latency.rs:534-537, benches/gossip_fixed.rs:56-57, benches/support/grid.rs:39, benches/window_wallclock.rs:12, tests/window_corners.rs:14, tests/tradeoff_probe.rs:30, tests/window_knee.rs:19, tests/hop_trace.rs:27, tests/gossip_pipelining.rs:13, tests/window_operator.rs:24, tests/latency_link.rs:13)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -rn -B3 'path = ".*latency.rs"' benches tests`: nine includers, eight with `#[allow(dead_code)]` on the `mod`, gossip_fixed.rs:56-57 alone without; `grep -rn -B3 'path = ".*grid.rs"' benches`: all three includers carry the module-level allow, so grid.rs:39's `#[allow(unused)]` is redundant)
- Seen by: structure, prose; refutation: confirmed (the structure lens's "nine of ten" corrected to eight of nine); history: no-rationale-found (the item-level allows accreted at 1988de6d and 814f07ad, each explaining the `#[path]` mechanism but never why the exception sits at the item rather than at the one includer lacking the module-level allow)
- Owner-gated: no

Four items in latency.rs carry their own `#[allow(dead_code)]` each preceded by the same three-line comment, and each exists only because one includer omits the module-level allow the other eight use. The comments also hand-maintain caller lists ("Used only by `window_wallclock`"). `grid.rs:39` carries `#[allow(unused)]` on `SIZES` although every includer already allows at the module level. One statement of a mechanism beats four, and a caller list in a comment rots (Principle 5).

Evidence:

   398	    // Used only by `window_wallclock`; the module is `#[path]`-included by
   399	    // several targets, each seeing its own copy's usage.
   400	    #[allow(dead_code)]

Resolution: Add `#[allow(dead_code)]` to `gossip_fixed.rs:56`'s `mod latency;` (as that file already does for `mod grid;` at 52), delete the four item-level allows and their comments in latency.rs, and delete `#[allow(unused)]` at grid.rs:39. Acceptance: `grep -c 'allow(dead_code)\|allow(unused)' benches/support/*.rs` is 0 and clippy stays clean across all include sites.

### benches-envelope-23: DelayedWire selects its cost model by a bool
- Where: benches/support/latency.rs:405 (related: benches/support/latency.rs:378-385, benches/support/latency.rs:438-446, benches/support/latency.rs:475-479)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (6c66b180 implemented review ruling R25's wording "store the paused flag and select" literally; neither considered the representation)
- Owner-gated: no

`with_clock(capacity, delay, paused: bool)` and the `paused: bool` field encode a two-mode choice (paused: partitioned wall plus virtual; running: wall only) whose meaning `round_trip` re-explains in a comment. Types-first: a bool parameter carries no name at the call site; the doc on the field is a two-variant enum's variant docs waiting to be written.

Evidence:

   405	    fn with_clock(capacity: usize, delay: Duration, paused: bool) -> Self {

Resolution: Introduce `enum Clock { Paused, Running }`, store it, and match on it in `round_trip` and `round_trip_virtual`'s assert. Acceptance: no `bool` parameter or field selects the clock mode in latency.rs.

### benches-envelope-24: `expect("bounded hop count")` names no bound
- Where: benches/support/latency.rs:552 (related: benches/support/latency.rs:524-537)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read)
- Seen by: none of the four lenses; raised by the refutation pass (its note 7) and confirmed by my reading
- Owner-gated: no

The message states a claim, not the one-line proof the doctrine asks of every `expect`. The ratio exceeds `u32::MAX` only past about 4.3 × 10⁹ serialized hops (49 days of virtual time at a 1 ms delay), and a session's hop count is bounded by the protocol's height structure and by `2·d / capacity(d)` under window stalls, so the bound holds with enormous margin; the message should say so.

Evidence:

   552	    u32::try_from(elapsed.as_nanos() / delay.as_nanos()).expect("bounded hop count")

Resolution: `.expect("a session's serialized hop count is bounded by its height structure and its window stalls, far below u32::MAX")`, or state the premise in the `# Panics` section above. Acceptance: the message names why the conversion cannot fail.

### benches-envelope-25: The harnesses restate serde bounds on round_trip that Rumors::gossip does not demand
- Where: benches/support/wire.rs:26-29 (related: benches/support/latency.rs:428-435, benches/support/latency.rs:467-474, benches/support/latency.rs:484-487, benches/support/latency.rs:515-518, benches/support/wire.rs:40-43, src/rumors.rs:92, src/rumors.rs:489-494, src/peer.rs:201-205, src/peer/bootstrap.rs:247-252)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`Rumors::gossip` at rumors.rs:494 bounds `T: Send + Sync + 'static` inside `impl<T, B: BookmarkError> Rumors<T, B>` at 92, which carries no serde bounds; peer.rs:201-205 states the design; `Bootstrap::join` at bootstrap.rs:252 requires `Serialize + DeserializeOwned + Eq + Send + Sync + 'static`, so `bootstrap_fork` legitimately keeps the full set)
- Seen by: perfapi; refutation: confirmed; history: deliberate-but-expired (the harness bounds mirrored `gossip`'s Borsh bounds at 818a8707; 48bc31df and ce27df86 localized serde at construction, and a5a16f43 swept `Eq` through the harness without narrowing the gossip-only sites)
- Owner-gated: no

`Wire::round_trip`, `DelayedWire::round_trip`, `round_trip_virtual`, `reconcile`, and `session_hops` require `T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static`, but the only thing they call is `Rumors::gossip`, whose bound is `T: Send + Sync + 'static`. `Peer::seed`'s doc states the design the library paid for: serde obligations live at construction "so neither the send paths nor the gossip entry points carry serde bounds of their own". The harness is the first generic caller and it shows future callers the wrong idiom; it also spells `serde::de::DeserializeOwned` inline at every site.

Evidence:

    26	    pub fn round_trip<T>(&mut self, a: Rumors<T>, b: Rumors<T>) -> (Rumors<T>, Rumors<T>)
    27	    where
    28	        T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
    29	    {

Resolution: Narrow the five gossip-only sites to `T: Send + Sync + 'static`; keep the full set on `bootstrap_fork` only, with `use serde::Serialize; use serde::de::DeserializeOwned;` at the top of wire.rs. Acceptance: `cargo check --all-targets` passes with the narrowed bounds; the only `DeserializeOwned` in benches/ is on `bootstrap_fork`.

### benches-envelope-26: Default-dialect tells: "integer-honest", "keeps min honest", "genuine", "hp"
- Where: examples/envelope_sim.rs:7 (related: examples/envelope_sim.rs:17, examples/envelope_sim.rs:392, examples/envelope_sim.rs:422, examples/envelope_sim.rs:539, benches/support/grid.rs:142)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n 'honest\|genuine'` over the partition; "hp bound" at 392 against "High-probability" spelled out at 274, 301)
- Seen by: prose; refutation: reframed (the em-dash-in-`//`-comment item is dropped here: `//` comments in src use em-dashes 116 times, so gossip_fixed.rs:179 follows the crate's convention and the conflict with the house rule is crate-wide; "knob" is crate vocabulary with 36 uses in src); history: no-rationale-found ("integer-honest" is inherited from the retired campaign note and its Python; "genuine" arrived with a de-minting pass)
- Owner-gated: no

Moralized code: "integer-honest" (7, 17, 539) and "saturation keeps `min` honest" (422) describe a property (every bound is computed in exact integers and dominates the exact form) that the `_int` suffix and the section comment at 539-544 already state; "honest peer"/"honest corpora" at 5 and 202 are the model-of-record term and stay. "a genuine disjoint peer" (grid.rs:142) adds nothing to "a disjoint peer". "hp bound" (392) abbreviates a term the file spells out elsewhere.

Evidence:

     7	//! integer-honest adoptable forms, and the proof obligations tying them

Resolution: "integer-honest" to "integer"; "keeps `min` honest" to "keeps `min` an upper bound"; "a genuine disjoint peer" to "a disjoint peer"; "hp bound" to "high-probability bound". Acceptance: `grep -n 'honest\|genuine' benches examples/envelope_sim.rs` matches only the model-of-record phrase.

### benches-envelope-27: The usage block advertises `--fast` as a standalone mode it is not
- Where: examples/envelope_sim.rs:36 (related: examples/envelope_sim.rs:1579-1584)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`main` reads `flag("--fast")` only inside `if flag("--full")` at 1580-1581; `--fast` alone runs `certification()` and prints the "--full" hint)
- Seen by: structure, perfapi; refutation: confirmed; history: no-rationale-found (port residue: the Python CLI had no `--full` gate, so its `--fast` line was accurate; 3824c7754 added the gate and kept the line)
- Owner-gated: no

A usage line is the CLI's contract; a documented flag that does nothing when passed as shown misleads.

Evidence:

    36	//!   cargo run --release --example envelope_sim -- --fast    # fewer seeds

Resolution: Write the line as `-- --full --fast   # Monte Carlo with fewer seeds`, or make `--fast` imply `--full` in `main`. Acceptance: every flag in the usage block changes the program's behaviour when passed as shown.

### benches-envelope-28: The "mirror the crate" constants are hand-copied literals that have rotted twice
- Where: examples/envelope_sim.rs:55-71 (related: examples/envelope_sim.rs:66, examples/envelope_sim.rs:105, examples/envelope_sim.rs:139-141, examples/envelope_sim.rs:164, examples/envelope_sim.rs:184-196, examples/envelope_sim.rs:1174, src/tree/typed/hash.rs:12, src/tree/typed/hash.rs:79, src/tree/mirror/streaming/remote/codec/budget/tests.rs:9, src/tree/mirror/streaming/window.rs:132, src/tree/mirror/streaming/window.rs:137, src/tree/mirror/streaming/window.rs:275, src/lib.rs:340-348, src/link.rs:169)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (hash.rs:12 `pub const MERKLE_HASH_LEN: usize = 24;` and hash.rs:79 `CHILD_RECORD_LEN = 1 + MERKLE_HASH_LEN`; budget/tests.rs:9 `assert_eq!(DEFAULT_TARGET_MESSAGE_SIZE, 1_830_400)`; window.rs:275 `DEFAULT_SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024`; `1_114_624 = 256 × (256 × 17 + 2)` by arithmetic; `git log -1 --date=short 2d1e6ea51` is 2026-08-18 and `git show --stat 2d1e6ea51 -- benches examples` is empty; the example was last touched by dfd19c447 on 2026-07-24; the Python source at envelope-sim.py:70 reads `Hash = [u8; 16]`; lib.rs:340-348 re-exports `DEFAULT_SYNC_MEMORY_BUDGET`, `DEFAULT_TARGET_MESSAGE_SIZE`, `MERKLE_HASH_LEN`; link.rs:169 `pub const STREAM_COUNT`; window.rs:132 `FAN` is `pub(crate)` and window.rs:137 `KEY_DEPTH` is private, and `src/testing.rs` exposes neither)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: deliberate-but-expired (correct mirrors at 3824c7754 on 2026-07-23; expired at 2d1e6ea51, which lists `DEFAULT_TARGET_MESSAGE_SIZE 1114624 -> 1638912` among its moved readings but names no example, and again at 4dd2053c9 for the CBOR wire)
- Owner-gated: no

The block says `LISTING_ENTRY_BYTES` mirrors the crate's 17-byte `(u8, Hash)` entry and `DECODE_SLACK_BYTES` mirrors `DEFAULT_TARGET_MESSAGE_SIZE`. Neither holds: the entry is 25 bytes since the digest widened to 24, and the target message size is pinned at 1,830,400; `1_114_624` factors as the 16-byte-entry, pre-CBOR full-fan frame. Every byte-denominated output (the `K_flat`/`K_sharp`/`K_int` tables, envelope-at-K, `L(N)`, heavy counts, the sensitivity section) is computed for a wire that does not ship, under a comment saying otherwise. Principle 5 and the no-hand-maintained-counts rule: four of the mirrored values are public re-exports the example could import, as `branch_hash.rs:29` already does for `MERKLE_HASH_LEN`. The count-level dominance sweep is constant-free and unaffected. Note the coupling: `check_landed_replication` (184-196) pins `k_flat`, which reads `LISTING_ENTRY_BYTES` through `PARKED_REPLY_SKELETON_BYTES` (66, 139-141) and `DECODE_SLACK_BYTES` through `STEADY` (105, 164), so importing the constants moves that pin (see benches-envelope-29).

Evidence:

    55	// ---------------------------------------------------------------------
    56	// Model constants. FAN/DEPTH/LISTING_ENTRY_BYTES/STAGES mirror the
    57	// crate (radix-256 tries over 32-byte content addresses, 17-byte
    58	// `(u8, Hash)` listing entries, `Stream::COUNT` = 17 streams);
    59	// DECODE_SLACK_BYTES mirrors `DEFAULT_TARGET_MESSAGE_SIZE`. The rest
    60	// parameterize the reference flat solve and the reply containers.
    61	// ---------------------------------------------------------------------
    62	
    63	const FAN: u128 = 256;
    64	const DEPTH: i32 = 32;
    65	const LISTING_ENTRY_BYTES: u128 = 17;
    66	const PARKED_REPLY_SKELETON_BYTES: u128 = FAN * FAN * LISTING_ENTRY_BYTES;
    67	const CHILD_CONTAINER_BYTES: u128 = 32;
    68	const STAGES: u128 = 17;
    69	const DECODE_SLACK_BYTES: u128 = 1_114_624;
    70	const DEFAULT_BUDGET: u128 = 16 * (1 << 30);
    71	const DEFAULT_N: u64 = 1 << 40;

Resolution: Derive every constant the crate exports: `const LISTING_ENTRY_BYTES: u128 = 1 + rumors::MERKLE_HASH_LEN as u128;`, `const STAGES: u128 = rumors::link::STREAM_COUNT as u128;`, `const DECODE_SLACK_BYTES: u128 = rumors::DEFAULT_TARGET_MESSAGE_SIZE as u128;`. `FAN` and `DEPTH` mirror `pub(crate)` items that `rumors::testing` does not expose, so either expose them there or state at the declaration that they are the simulator's own model parameters. Rewrite the banner to say which constants are imported and which are the simulator's own. Re-run the certification and re-pin or re-label `check_landed_replication` in the same change (benches-envelope-29). If benches-envelope-32 dissolves the example, this dissolves with it. Acceptance: `grep -n '= 17;\|1_114_624' examples/envelope_sim.rs` returns nothing; no numeric literal in the file duplicates a value the crate exports; the analytic certification passes.
Construction: Add at the top of `main`: `assert_eq!(LISTING_ENTRY_BYTES, 1 + rumors::MERKLE_HASH_LEN as u128); assert_eq!(DECODE_SLACK_BYTES, rumors::DEFAULT_TARGET_MESSAGE_SIZE as u128);`. Both fail today (17 against 25; 1,114,624 against 1,830,400).

### benches-envelope-29: The "landed" flat baseline, its "default 16 GiB" budget, and the L(N) derivation denominate against designs on no surviving branch
- Where: examples/envelope_sim.rs:181-196 (related: examples/envelope_sim.rs:10-13, examples/envelope_sim.rs:21-22, examples/envelope_sim.rs:70-76, examples/envelope_sim.rs:139-179, examples/envelope_sim.rs:728-787, examples/envelope_sim.rs:1171, examples/envelope_sim.rs:1174-1221, examples/envelope_sim.rs:1223-1251, examples/envelope_sim.rs:1522-1569, src/tree/mirror/streaming/window.rs:275, .agent-notes/2026-07-22-sync-budget/sync-budget.md:593-596, .agent-notes/2026-07-22-sync-budget/sync-budget.md:602-606, .agent-notes/2026-07-22-uniformity-envelope/README.md:16-19)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`4_644` and `NODE_BYTES`/`k_flat` appear nowhere in src, tests, or benches; line 1174 prints "at the default 16 GiB budget" while the shipped default is 512 MiB; sync-budget.md:593-596 records that the `NODE_BYTES` token survives only in the design documents and this simulator, "which speak about the rejected shape"; sync-budget.md:602-606 lists per-stream budget division (`L(N)`) under "Deliberately out of scope"; the uniformity-envelope README:16-17 states "Everything the body calls "landed" lives on the campaign branch, not on any branch that survives"; the Python header has `DEFAULT_BUDGET = 16 * (1 << 30)  # 16 GiB`)
- Seen by: structure, prose; refutation: confirmed; history: already-known (both notes acknowledge the residue as scoped out of their campaigns; neither is a ruling to keep it, so the owner decision the finding asks for has not been recorded)
- Owner-gated: yes (retiring or re-labeling a design comparison is the owner's call)

`check_landed_replication` pins "the reference figures" of a flat per-node solve (`K_flat(2^40, 16 GiB, 340) = 4 644`) that exists on no branch in this repository; `window.rs` prices nodes through the backend's `node_bytes`, not a flat constant. The report titles a section "at the default 16 GiB budget" when the crate's default is 512 MiB and was never 16 GiB on a surviving branch. Section 4 derives `L(N)` for a per-stream budget division the design record scopes out. "Landed" is dated provenance vocabulary (AGENTS.md hard rule: no references to code that no longer exists), and a pin whose only referent is the thing pinning it is the circular-justification tell. The sections that earn their place are the dominance sweep and the Monte Carlo tiers.

Evidence:

   181	/// Pins the flat replication: the reference figures (default window
   182	/// 4 644 at N = 256⁵; NODE_BYTES = 340 the unique consistent price;
   183	/// the ~3× widening at small declarations) must reproduce exactly.
   184	fn check_landed_replication() {
   185	    let consistent: Vec<u128> = (0..2048)
   186	        .filter(|&nb| k_flat(DEFAULT_N, DEFAULT_BUDGET, nb) == 4_644)
   187	        .collect();
   188	    assert_eq!(consistent, vec![340], "NODE_BYTES back-out failed");

Resolution: Owner decision between (a) retire the flat baseline and the `L(N)` section: delete `NODE_BYTES`, `per_scope_flat`, `charged_scopes`, `k_flat`, `check_landed_replication`, the `K_flat` columns, the `flat,` manifest rows, `l_of_n`, `heavy_count`, `stage_saturation_bytes`, and their report blocks, and rename `DEFAULT_BUDGET`/`DEFAULT_N` to `SIM_BUDGET`/`SIM_N` (or import `rumors::DEFAULT_SYNC_MEMORY_BUDGET` if the tables should be at the shipped default); or (b) keep the flat solve as a named comparison and re-state it positively ("a flat per-scope solve, the alternative the sharpened envelope is compared against, at a 16 GiB budget"), dropping "landed", "reference", "default", and the self-referential pins, and move the `L(N)` derivation to `.agent-notes/` beside the transport receive-window plan that earmarks it. Module-doc items 1, 4, and 10-13 and the printlns at 1171 and 1174 follow either way. If benches-envelope-32 dissolves the example, this is subsumed. Acceptance: `grep -n 'landed\|4_644\|4644\|default 16 GiB' examples/envelope_sim.rs` returns nothing; every remaining constant is imported from the crate or documented as the simulator's own parameter.

### benches-envelope-30: A verification is claimed done whose artifact is not in the tree
- Where: examples/envelope_sim.rs:304-308 (related: examples/envelope_sim.rs:44-46)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (no comparison harness in tests/ or tools/; the independent implementation lives at `.agent-notes/2026-07-22-uniformity-envelope/envelope-sim.py`; 3824c7754's message records the one-time comparison: "verified value-for-value: 5,076 manifest lines … byte-identical")
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds (the comparison happened, and its record lives only in a commit message), so the finding converts to: state the argument inline and drop the past-tense claim
- Owner-gated: no

The doc says the clamped-domain search was "verified value-for-value in the manifest comparison", while the module doc at 44-46 presents `--manifest` as an affordance an independent implementation can be diffed against. The reference implementation and the comparison live in a retired agent note and a commit message, so the claim is not reproducible from the tree. Principle 8: distinguish what the tree verifies from what was once verified elsewhere; the argument the sentence already gives is what belongs here.

Evidence:

   304	/// Past `u128` range the slot count is still an exact power of two in
   305	/// `f64`, the pair mean is sub-unit, and the search boundary sits at
   306	/// small values where every `f64` conversion is exact — so the
   307	/// clamped-domain search returns what the unbounded-integer search
   308	/// would (verified value-for-value in the manifest comparison).

Resolution: Drop the parenthetical, or replace it with the reproducible affordance: "(`--manifest` dumps these values for diffing against an arbitrary-precision implementation)". Acceptance: no doc in the file asserts a completed comparison the tree cannot rerun.

### benches-envelope-31: joint_int's expect argues a bound that fails for j = 16 at N ≥ 2⁶²
- Where: examples/envelope_sim.rs:607-608 (related: examples/envelope_sim.rs:128-133, examples/envelope_sim.rs:295, examples/envelope_sim.rs:579-591, examples/envelope_sim.rs:691-705)
- Class / severity / confidence: correctness / nit / high
- Provenance: verified (by arithmetic against 579-591: at `n = 2⁶²`, `nn = 2¹²⁴`, `num_bits = 125`, `den_exp = 128`; the first branch fails because `125 > 128 − 48 − 2 = 78`; `den_bits = 129 < num_bits + 5 = 130`, so `small_mean_quantile` returns `None`; `pow256(16)` is `None` at 128-133, and the `expect` fires. At `n = 2⁶¹` the second branch returns `Some`, so the threshold is exactly `n ≥ 2⁶²`)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (a port artifact; the comment transcribes the Python's informal premise)
- Owner-gated: no

The comment proves `256^j` fits `u128` from "8j is within nn's bit length", but the function's declared `u64` domain admits inputs where `small_mean_quantile` returns `None` and `pow256` is `None` together. It is unreachable only because the hard-coded sweep grids stop at `2⁵⁰` and `occ_hi`'s `assert!(n < (1 << 60))` at 295 runs earlier in the sweep, neither of which the message names. Doctrine: every `expect` message is a one-line proof of why it cannot fire; this one's premise is incomplete for the function's domain.

Evidence:

   607	    // Non-sub-unit mean ⇒ 8j is within nn's bit length ⇒ 256^j fits.
   608	    let slots = pow256(j).expect("bulk regime keeps 256^j within u128");

Resolution: Assert the `N < 2⁶⁰` premise at `joint_int`'s entry (matching `occ_hi`) and cite it in the message, or fall back to the corpus cap `u128::from(n)` when `pow256` is `None`, which is the value the deterministic cap gives anyway. Acceptance: the message (or a guarding assert) names a premise that covers every `u64` the function accepts.
Construction: Call `joint_int(1 << 62, 16)`: `small_mean_quantile(2¹²⁴, 128, 48)` returns `None`, `pow256(16)` is `None`, and the `expect` panics.

### benches-envelope-32: The dominance certificate window.rs cites compares the simulator's own copy of the integer envelopes against its own oracle, never covers the shipped pair-product forms, and is run by nothing
- Where: examples/envelope_sim.rs:690-726 (related: examples/envelope_sim.rs:28-31, examples/envelope_sim.rs:48-53, examples/envelope_sim.rs:538-682, src/tree/mirror/streaming/window.rs:570-578, src/tree/mirror/streaming/window.rs:689-701, src/tree/mirror/streaming/window/tests.rs:376-381, Cargo.toml:194-198, justfile:97-99)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (window.rs:572-575 says the dominance "is verified by `examples/envelope_sim.rs` over a dense sampled sweep"; the example's imports at 48-53 are `std` and `rand` only, with no `use rumors`; `check_integer_dominates` compares `occ_int`/`joint_int`/`q_leaves_int`/`q_slots_int`/`stage_pop_int` (562-661) against the example's own float oracle (206-407); `grep -rn envelope_sim justfile .github tools src tests Cargo.toml` returns only window.rs:573; justfile:97-99 `check` only type-checks examples; `window/tests.rs:376-381` `envelopes_are_consistent` checks internal relations only, and every bound-level call in that file (`jointly_occupied`/`stage_population` at 52, 57, 207, 210, 378, 380) passes `pair = n * n`; I compared the example's `_int` family to window.rs function by function and the formulas agree at `pair = n²` today, while the shipped `jointly_occupied(n, pair, j)` generalizes to `A·B` and takes the saturating `pow256` path where the example's `expect` at 608 sits)
- Seen by: structure, correctness, perfapi, prose (the "nothing runs it" half); refutation: confirmed, severity lowered from high to medium because window.rs:626-666 carries inline sufficiency proofs for `bernstein` and `small_mean_quantile`, making the sweep a secondary empirical check of an argued bound; history: deliberate-and-holds for the independent-replica design (ded24eb3a: "examples/envelope_sim.rs is the certifying tool of record"; 3824c7754 verified the port byte-identical against the Python; the example's header at 28-31 concedes the pair-product gap in its own words) and no-rationale-found for the gating gap (`git log -S envelope_sim -- justfile .github` is empty at every commit)
- Owner-gated: yes (reverses the recorded "tool of record" ruling)

A provenance claim in production code names a check that exercises a sibling implementation, in a place the gate never runs. The shipped `occupied`/`jointly_occupied`/`children_quantile`/`stage_population` are separate code with a divergent signature (the pair product), the pair-product adaptation is certified by nothing at any asymmetric `(A, B)`, and a refactor of `small_mean_quantile` or `bernstein` in window.rs today would leave the cited certificate untouched and green. Principle 6 (the cheapest artifact that satisfies the proxy must be the intended one) and Principle 8 (a provenance claim must name a check that exercises the code making the claim): "a board nothing enforces is decoration". The design value the ruling protected, an independent float oracle, survives the fix; what changes is what the oracle is pointed at.

Evidence:

   690	fn check_integer_dominates() {
   691	    let ns: [u64; 13] = [

   706	    for &n in &ns {
   707	        for j in 0..=DEPTH {
   708	            assert!(occ_int(n, j) >= occ_hi(n, j), "occ dominance at ({n}, {j})");
   709	            assert!(
   710	                joint_int(n, j) >= joint_hi(n, j),
   711	                "joint dominance at ({n}, {j})"
   712	            );

    28	//! The shipped window derivation (`src/tree/mirror/streaming/window.rs`)
    29	//! implements the pair-based `A·B` adaptation of the same integer
    30	//! family; this tool certifies the one-corpus `N` forms and the
    31	//! integer-over-exact dominance those adaptations rest on.

Resolution: Move the exact-Chernoff oracle (`p_occ`, `binom_tail_log`, `chernoff_quantile`, `occ_hi`, `joint_hi`, `occ_quantile`, `stage_pop`; about 120 lines) into `src/tree/mirror/streaming/window/tests.rs` as a differential test `integer_envelopes_dominate_exact_chernoff` that sweeps the same `(N, depth)` grid against the real `occupied`, `jointly_occupied`, `children_quantile`, and `stage_population`, extending the oracle to asymmetric `(A, B)` (joint per-slot probability `p_occ(A) · p_occ(B)`) so the shipped pair-product path is covered. Re-state window.rs:570-578 to name that test. Delete the example's integer copies (538-682) and `check_integer_dominates`; if the Monte Carlo tiers are worth keeping they become an `#[ignore]`-gated test beside it, and what remains of the example is design exploration that retires to `.agent-notes/`. The cheaper interim step is `test = true` on the example in Cargo.toml (the `swarm` precedent at 194-198) with the two certification calls under `#[test]`, which closes the gating gap but not the replica gap. Acceptance: `just gate` runs a test that fails when any shipped integer quantile is lowered below its exact-Chernoff counterpart at a sampled `(A, B, depth)` (demonstrate once by a deliberate `- 1` on `bernstein`'s return); window.rs no longer names `examples/envelope_sim.rs`; the example's integer copies are gone or the example itself is.
Construction: In window.rs, change `bernstein`'s return to `mean_hi + (2 * mean_hi * t).isqrt() + t - 1` and run `just gate`: every committed test passes, because nothing compares the shipped quantiles against an exact tail. Then run `cargo run --release --example envelope_sim` by hand: it also passes, because it never sees window.rs.

### benches-envelope-33: envelope_sim repeats itself: two copies of the sweep grid, the KL tail, both bisections, the per-parent min, the theta list, and the measured population
- Where: examples/envelope_sim.rs:691-705 (related: examples/envelope_sim.rs:1109-1123, examples/envelope_sim.rs:222-236, examples/envelope_sim.rs:336-349, examples/envelope_sim.rs:170-178, examples/envelope_sim.rs:523-531, examples/envelope_sim.rs:259-267, examples/envelope_sim.rs:354-361, examples/envelope_sim.rs:647-649, examples/envelope_sim.rs:669-671, examples/envelope_sim.rs:715, examples/envelope_sim.rs:1137, examples/envelope_sim.rs:1162, examples/envelope_sim.rs:1225, examples/envelope_sim.rs:1044-1052, examples/envelope_sim.rs:1483-1491, src/tree/mirror/streaming/window.rs:449-456)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (the two thirteen-element arrays are textually identical; the KL expression at 234 recurs at 347; the largest-fitting upper bisection at 170-178 recurs at 523-531 and at window.rs:449-456; the lower bisection at 259-267 recurs at 354-361; `c_q_int` at 647-649 is the `sub = FAN` case of the min spelled out at 669-671, 715, and 1137; the theta list appears at 1162 and 1225; the `s_meas` computation at 1044-1052 recurs at 1483-1491 as `p_up * p_up` against `.powi(2)`)
- Seen by: structure, perfapi; refutation: confirmed; history: no-rationale-found (port-introduced: the Python defines the sweep list once and has no `--manifest` mode; 3824c7754 added the mode and spelled the list a second time)
- Owner-gated: no

A 1,586-line numerical tool is reviewable only if each formula has one home; the second sweep array lets the manifest stop covering what the sweep certifies without a diff a reviewer would notice, and the doc at 687 hand-counts it ("13 corpus sizes"). Several of these vanish if benches-envelope-32 dissolves the integer copies; the rest are one-line hoists.

Evidence:

   691	    let ns: [u64; 13] = [
   692	        2,
   693	        10,
   694	        100,
   695	        10u64.pow(4),

  1109	    let sweep_ns: [u64; 13] = [
  1110	        2,
  1111	        10,
  1112	        100,
  1113	        10u64.pow(4),

Resolution: Hoist `SWEEP_N` and `THETAS` to `const`; make `binom_tail_log` take `(nf: f64, p: f64, af: f64)` and call it from `chernoff_quantile_huge`; extract `largest_fitting(lo, hi, fits)` and `smallest_certifying(lo, hi, certifies)`; generalize `c_q_int` to `q_int(n, j, sub)` and call it at all four sites; extract `measured_stage_population`. Acceptance: each listed expression has exactly one definition in the file; the "13 corpus sizes" count is gone from prose.

### benches-envelope-34: prefixes() shifts a u64 by 64 at depth 0: a debug-mode panic and a wrong release-mode row
- Where: examples/envelope_sim.rs:807-812 (related: examples/envelope_sim.rs:840, examples/envelope_sim.rs:887-896, examples/envelope_sim.rs:1414-1420, Cargo.toml:174-192)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read against the Rust reference: a shift by at least the operand's bit width is arithmetic overflow, a panic with overflow checks on and a masked shift with them off; the Python original at envelope-sim.py:718 evaluates the same shift under numpy, where an unsigned shift by the width yields 0, so the port introduced the bug; I could not run the example)
- Seen by: correctness; refutation: confirmed, severity lowered from medium to low (the vacated assertion at 1420 could not have failed at `j = 1`, where the envelope is the deterministic depth-1 slot cap of 256 and `listed` is structurally at most 256; what remains is a wrong printed row and a debug-mode panic in an example nothing runs); history: no-rationale-found (a port artifact; the port's byte-identical check covered the deterministic manifest, which never calls `prefixes`)
- Owner-gated: no

`prefixes(keys, 0)` computes `shift = 8 * (8 - 0) = 64` and evaluates `k >> 64` on a `u64`. `trie_stats` calls it for `j = 0` unconditionally (840; the loop at 842-866 then never reads the depth-0 entry), and `two_corpus_stats` calls `prefixes(&a, j - 1)` at `j = 1` (891). With debug assertions the documented `--full` run panics before the first Monte Carlo table prints; in release (Cargo.toml sets no `[profile.release]` override) the shift masks to zero, every key becomes its own "prefix", `a_up.binary_search(&(p >> 8))` searches random keys for small values, `listed` at `j = 1` reads 0 instead of about 256, and the assertion at 1420 passes vacuously. Correct at all scales, for all inputs: a panic reachable from an argument the code itself passes, and a release result that differs from the debug result, is incorrect behaviour, not a tolerated corner.

Evidence:

   807	fn prefixes(keys: &[u64], j: i32) -> Vec<u64> {
   808	    let shift = 8 * (8 - j as u32);
   809	    let mut v: Vec<u64> = keys.iter().map(|&k| k >> shift).collect();
   810	    v.dedup(); // input sorted ⇒ prefixes sorted
   811	    v
   812	}

Resolution: Make the depth-0 case explicit: `if j == 0 { return vec![0]; }` before the shift (or `k.checked_shr(shift).unwrap_or(0)`); start `trie_stats`'s range at 1 since nothing reads the depth-0 entry; keep the `two_corpus_stats` call at 891, which needs the depth-0 case handled. Then run `--full --fast` once in a debug profile and once in release and confirm the two-corpus table's `j = 1` "listed meas" column is about `256 · p_occ(N, 1)²`-scale, not 0. Acceptance: `cargo run --example envelope_sim -- --full --fast` completes in a debug profile; the `j = 1` row shows a non-zero `listed meas` consistent with `joint pred`; a unit assertion that `prefixes(keys, 0) == [0]` for a non-empty corpus is committed.
Construction: Debug profile: `cargo run --example envelope_sim -- --full --fast` panics at line 809 with "attempt to shift right with overflow" during the brute-force section. Release profile: insert `assert_eq!(prefixes(&keys, 0), vec![0]);` after `let keys = draw_keys(n_brute, &mut rng);` in `monte_carlo` and observe it fail, or read the printed `j = 1` "listed meas" (about 0.0) against the expected hundreds.

## Positives

- benches/support/latency.rs:10-62 states the measurement model as mechanism: which component is deterministic and why (the runtime's one thread, no library timers, deadlines on the delay lattice, OS descheduling unable to convince the runtime a pending task is idle), which component moves with load, and which figure assertions may rest on. `round_trip_virtual` refuses to report a virtual figure on a wall-clock wire (475-479) and `hops_on_lattice` fails loudly off the lattice instead of rounding (538-553). The `# Panics` sections are one-line proofs, not labels. I traced the reader and writer state machine (waker set and consumed under the same lock, the equal-deadline timer path, half-close via `writer_gone` with queue drain before EOF, `buffered <= capacity` by construction) and found no lost wakeup, underflow, or busy loop; `tests/latency_link.rs` pins the link against the public conformance suite at zero and nonzero delay.
- examples/window_tradeoff.rs is twelve lines over `rumors::testing::window_tradeoff_table()`, `just window-tradeoff` (justfile:773-775) writes through a temp file and `mv` so a failed build cannot truncate the tracked artifact, and `tradeoff_table_matches_the_derivation` (window/tests.rs:258-263) byte-compares the committed table with a message naming the recipe: a derived artifact with no hand-maintained copy and a closed drift path.
- benches/branch_hash.rs restates `Hash::branch`'s preimage faithfully (I checked every field against hash.rs:162-213), takes the hash width from the public `rumors::MERKLE_HASH_LEN` so the 16-to-24 widening moved it automatically, names the exact claim it keeps re-measurable, and hash.rs:167 cites it back with the measured magnitude.
- benches/support/grid.rs denominates throughput precisely (`Cell::divergence`, 86-95: per-side transfer, not the shared size), documents the disjoint-block redaction scheme and its `common >= 2 * redacted` precondition once in the module doc, enforces it in `cells()` (124), and restates it where the slices are taken (168-169). The corners-of-the-cube list (23-28) makes the classic divergence shapes fall out of the sweep instead of being enumerated.
- Every Criterion group keeps fixture construction untimed (`iter_batched` at `PerIteration`), warms lazy memos explicitly, and says what its throughput denominator means; `gossip_grid.rs:60-72` moves the identical corner to a latency-only group with the reason stated.
- benches/gossip_fixed.rs:27-39 tells the reader how to read the latency lines (intercept is compute, slope over delay is the serialized hop count) and which shape of slope is the regression the session window exists to prevent; the latency sweep's `iter_custom` (192-201) sums only the harness's own reported elapsed, keeping the per-iteration fixture rebuild out of the figure, and the comment at 175-180 explains the sample-size choice in terms of what is bounded.
- benches/in_memory.rs's per-bench docs each say what the column measures and which claim it puts under measurement (`range_delta` at 168-174 is the model), the right altitude for a bench.
- examples/envelope_sim.rs's integer arithmetic is disciplined: `pow256` is the single crossover guard, `bernstein`'s `x = ⌊√(2μT)⌋ + T` certifies `e⁻ᵀ` in the regime every caller is in, `small_mean_quantile`'s bit-length argument is sound, `binom_tail_log` handles the `p = 0` and `a = n` boundaries explicitly, and the `# Numeric representation` paragraph (39-46) says exactly where `u128` gives way to `f64` and why. The `--manifest` mode dumping every deterministic quantity for value-for-value diffing is the right shape for a numerical certificate. The file keeps inline tuple types with `#[allow(clippy::type_complexity)]` (1335, 1390) rather than coining names.

## Open questions for Finch

- Should `examples/envelope_sim.rs` remain the certifying tool of record (ded24eb3a), or should the dominance certificate move into `window/tests.rs` as a differential test over the shipped functions, extended to asymmetric `(A, B)`, with the example's integer copies dissolved? Recommendation: move it; the oracle's independence survives, the gate enforces it, and the pair-product path gains its first bound-level check. If the example stays, the minimal step is `test = true` in Cargo.toml so `test-all` runs the analytic tier.
- Is the flat-solve baseline (`NODE_BYTES`, `k_flat`, `check_landed_replication`) a standing design comparison worth keeping under a positive name, or history to retire? Recommendation: retire it; both agent notes already record the rejected shape, and the tables it feeds are computed for a wire that no longer ships. Same question for the `L(N)` section, which the sync-budget note earmarks for a future transport task: keep it in `.agent-notes/` beside that plan.
- Should the example's `FAN` and `KEY_DEPTH` mirrors reach the crate through `rumors::testing` (both are `pub(crate)` in window.rs), or be declared as the simulator's own model parameters? Recommendation: declare them as the simulator's own; they are structural constants of a radix-256 trie over 32-byte paths and will not move.
- Renaming the `V2` Criterion series ids breaks comparison against any saved local baselines; no committed roster depends on them. Is there bench history worth preserving, or is a clean rename fine pre-release? Recommendation: rename now, before a release makes baselines worth keeping.
- The benches are compiled by the gate (`bench-build`) but never run; a fixture `expect` that starts panicking surfaces only when someone runs `just bench`. Is a `--sample-size 10 --measurement-time 1` smoke of each bench binary worth a `just all` leg? Recommendation: yes, as a cheap liveness check; it would also have caught nothing here, since the benches are sound, so the case rests on the future.
- `Wire` uses 64 KiB pipes (wire.rs:10) while `DelayedWire` in the latency sweep uses 8 MiB (gossip_fixed.rs:87); the zero-latency intercept of the fixed groups and the `d = 0` point of the latency groups therefore measure different harness overheads. Intended (the memory link as a socket-buffer stand-in), or should they share a capacity so the intercepts compare? Recommendation: state the intent at `CAPACITY` either way.
- Crate-wide register: `//` line comments in src use em-dashes 116 times and "knob" appears 36 times, both contrary to the house rules (spaced double-hyphens in code comments; unanchored metaphor as jargon). Both are one decision and, once made, mechanically enforceable by the doc linter. Recommendation: rule once, then sweep; do not fix them piecemeal in this partition.
- `Gossiped.stats` (bytes sent and received, disputed scopes, window granted) is discarded by both wire harnesses. Reporting `Throughput::Bytes` alongside elements would make the gossip benches read as wire efficiency as well as compute. A bench-design choice, not a defect; recommendation: add it when the next bench pass happens.
- window_wallclock.rs:36-38 asserts the 256 KiB budget runs "below and above its serialization knee (1k pipelined, 10k serialized)"; the knee suite computes its own predicted knee from the binding capacity. Recommendation: have the bench doc cite that derivation rather than hard-code the two divergences as the two sides, so a window change cannot leave the claim stale.

## Dropped

- "envelope_sim certifies its own copy" as a high-severity simplification (structure [0]): merged into benches-envelope-32 at medium, per the refutation's reasoning that window.rs carries inline sufficiency proofs; the class is verification-gap, which is what the defect is.
- Correctness [31] and perfapi [38] (the certificate is never executed; the example imports nothing from the crate): duplicates of benches-envelope-32.
- Prose [18] (nothing runs the envelope certification): the enforcement half of benches-envelope-32, merged.
- Structure [1], prose [16], perfapi [39] (mirrored constants): duplicates of benches-envelope-28 (correctness [30] is the most complete report and is the base).
- Structure [2] (flat solve and L(N) model unshipped designs): merged into benches-envelope-29 with prose [17].
- Structure [3] (dead-code convention): merged into benches-envelope-22 with prose [23]; its "nine of ten includers" is corrected to eight of nine.
- Prose [19] (gossip_fixed's stale include comment): merged into benches-envelope-16 as the comment half of the wire-under-grid nesting.
- Perfapi [45] (duplicated helpers across binaries): split across benches-envelope-11 (send_units), benches-envelope-16 (bootstrap_fork), and benches-envelope-33 (the sweep grid).
- Prose [27] (send_units and its rationale stated three times): merged into benches-envelope-11.
- Prose [20] and perfapi [46] (V2 label, unreachable arms): duplicates of benches-envelope-2; the attribution of the unreachable arms to the V1 retirement is corrected, since 818a8707 wrote them with the two-element sweep.
- Perfapi [48] (`--fast` standalone): duplicate of benches-envelope-27.
- Perfapi [47] (branch_hash mirror): duplicate of benches-envelope-1.
- Perfapi [44] (runtime inside the timed body): duplicate of benches-envelope-15.
- Perfapi [49] (OsRng draw inside batch_insert's timed loop): merged into benches-envelope-12, whose `iter_batched` resolution removes both the drop and the syscall; its unmeasured "low-single-digit percent" magnitude is dropped.
- Structure [5]'s stated mechanism (a split removes the dead-code allows): refuted; every includer uses a different subset within each layer. Kept as benches-envelope-20, a nit design proposal whose payoff is legibility.
- Prose [21]'s proposal to rename "receive window": dropped; it is the established networking term and the design record's vocabulary. Narrowed to benches-envelope-4.
- Prose [25]'s em-dash-in-`//`-comment item: dropped as a partition finding; the crate uses em-dashes in line comments throughout (116 sites in src), so it is a crate-wide register decision, raised as an open question. Its "knob" item: crate vocabulary (window.rs:14 "The public knob", 36 uses in src), same disposition.
- Correctness [29]'s numpy claim about the retired Python: out of scope for the tree and unverifiable here (numpy is not installed); the port-bug attribution rests on reading the Python source, and the Rust defect stands on its own.
- The criterion 0.8.2 citations in correctness [34] and [37]: corrected to the pinned 0.5.1 (Cargo.lock:519-520); the mechanics are the same and the verdicts stand.
- `children()`'s `k.max(1)` in branch_hash.rs:50 as a dead guard: below the bar (the range is empty at `k = 0`, so the guard is never exercised and harmless).
- The persistent-wire stream-accounting question from the correctness lens (whether a session can leave an opened-but-unaccepted stream): not a finding; `link.rs` states the contract and `tests/reuse.rs` pins reuse, and no lens found evidence against it.
