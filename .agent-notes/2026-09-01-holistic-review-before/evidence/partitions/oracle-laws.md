# Partition oracle-laws: The paper-shaped reference oracle and the named algebraic and representational laws

## Partition summary

This partition is instrument code, compiled only under `cfg(test)` or the `oracle`/`laws` features (`lib.rs:435-445`). The oracle (`oracle.rs` and its three type files, 916 lines) transcribes the ITC paper's recursive trees: `Party` and `Version` are the trees, every paper operation is a method, children sit behind `Arc` so the derived `Clone` is a refcount bump, and the module doc states an operating envelope (small scope, bounded depth, literal ticks) that every harness cap I checked honors (`MAX_TRACE_OPS = 30`, `ARB_DEPTH`, the iterated-tick law's `0..=3`). Its property suite (`oracle/tests.rs`, 829 lines) is the ground-truth gate: lattice and order laws over op-trace populations, the paper's worked examples with section citations, and a grow-optimality suite pinned against a brute-force enumeration that shares no arithmetic with the DP. The laws module (`laws.rs`, 3,493 lines) is one macro-registered roster of `(name, fn) -> bool` predicates grouped by input signature; `laws!` makes registration structural (a law cannot exist unregistered or under a foreign name), `for_each_law_group!` gives every consumer (two proptest drivers, the organic drive list, the fuzz target, the surface-coverage citation haystack) compile-time refusal of a novel signature, and `laws/tests.rs` (135 lines) pins the collection's own invariants. I read all 5,373 partition lines with line numbers, plus `fold.rs`, the production `join_all` contracts, the `join_all` differentials in `party/tests.rs` and `clock/tests.rs`, the surface roster and its citation checks, the generators, the law drivers, the fuzz target, `grow.rs`, `grow_brute_force.rs`, and the git history the history pass cited. The test files are `oracle/tests.rs` and `laws/tests.rs`; `laws.rs` is itself an instrument.

The quality is high. Paper transcription is exact at every case I traced (`node`/`is_normal` implement the §5.2 normal form; `leq`, `join_off`, `fill`, `split`, `sum` match the reference); fallible operations are quantified over outcomes, arm and payload; conditional laws construct their antecedents where the module policy says to, with `projection_additive_over_carved_regions` falling back to a fork-half decomposition so no population leaves it vacuous; the conservation laws state at the law why they quantify over region unions rather than byte identity, and I traced the deterministic witness `[a, alias(a), b, alias(b), c]` through the counter and confirmed the hand-back really is a coalesced group. The arity sweep is derived from the counter's structural boundaries, not observed values.

The dominant issue is at the oracle's edge. `Party::join_all` and `Clock::join_all` are not paper definitions but two verbatim copies of production's `fold::balanced_try_fold`, and the differentials that consume them pin the hand-back vector element-wise, contents and order, although the public contract now declares which inputs are absorbed versus handed back unspecified. The history pass shows this was deliberate when the contract documented the discipline (ace236595) and that the premise expired in the owner's docs pass (b3f09baa0), which also deleted the oracle-side docs that stated the purpose. The oracle module doc still claims obvious-correctness and paper transcription for a file that carries forty lines of stack discipline. Everything else is smaller: a medium doc/body mismatch in two oracle tests, a false one-to-one mirror claim over a dead `has_seen` and a renamed `receive`, a production-cost import whose stated rationale names no constructible input, a totality pin that scans source text where a compile-time tie exists, and a batch of prose and idiom nits (the banned "mint", an undefined "door" used 41 times, `unwrap` inside law bodies, `% n` pool indexing).

## Findings

### oracle-laws-1: The oracle Clock's one-to-one mirror claim is false: `has_seen` has no caller, the observer trio and `receive` mirror deleted or renamed production methods, and the module doc claims a single omission
- Where: crates/before/src/oracle/clock.rs:9-11 (related: crates/before/src/oracle.rs:9-12, crates/before/src/oracle/clock.rs:123-143, crates/before/src/clock/tests.rs:172-174 and 618, crates/before/src/testing/optrace.rs:93 and 140, crates/before/src/oracle/version.rs:99)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn 'has_seen\|happens_before\|concurrent_with\|\.receive('` over `crates/before`: `has_seen` has only its definition and two comment mentions; production `clock.rs` has no `has_seen`, `happens_before`, `concurrent_with`, or `receive`; `git show a8c4d395f` removes all three observers and the `ia.has_seen(&msg)` call; `git show dc88e755` renames `receive` to `recv`); executed: no
- Seen by: scaffolding [5], adequacy [15], structure-prose [23], refutation (new item 2); refutation: reframed (the `Default for oracle::Version` sub-claim is refuted: clippy's `new_without_default` under the gate's `-D warnings` requires it, and no `allow` exists); history: deliberate-but-expired (the mirror was true at b3320b385; a8c4d395f and dc88e755 changed production; b3f09baa0 reflowed the sentence without re-truing it)
- Owner-gated: no

Both mirror sentences state something the file below them contradicts (Principle 5: prose speaks in the present tense and says what is). `oracle::Clock` carries `has_seen` with no caller anywhere, `happens_before`/`concurrent_with` whose production counterparts were deleted, and `receive` where production has `recv`, while production's `ticks`, `forks`, `absorb`, `recv_all`, `absorb_all`, and `sync_all` have no oracle counterpart; `optrace::run` and `step_impl` already translate between the two spellings. The module-level sentence says the oracle omits only the byte codec, but it also omits `Span`, `Ranked`, the `causally` queries, `distance`, and `lag`. The observer differential's doc in `clock/tests.rs` names `has_seen` and `happens_before` although its body calls neither.

Evidence:

         9	/// The reference [`Clock`](crate::Clock): the paper's recursive trees,
        10	/// mirroring the optimized type's API one-to-one so the differential tests can
        11	/// drive both with the same script.

         9	//! can serve as differential ground truth. It mirrors the target's **semantic**
        10	//! surface (construction, operations, ordering, operators) and omits the one
        11	//! purely *representational* concern that carries no semantics: the byte codec
        12	//! (`encode`/`decode`).

       123	    pub fn has_seen(&self, msg: &Version) -> bool {
       124	        msg.leq(&Base::ZERO, &self.version, &Base::ZERO)
       125	    }
       140	    pub fn receive(&mut self, msg: Version) {

    clock/tests.rs (the consumer whose doc names methods its body never calls):
       172	    /// The clock observers match the oracle's: `has_seen` is `msg <= version`,
       173	    /// `happens_before` is the strict causal order, and `concurrent_with` is
       174	    /// incomparability.
       186	        prop_assert_eq!(ia.version() >= msg, oa.version() >= msg_oracle);

Resolution: Delete `has_seen` (then `Version::leq` at oracle/version.rs:99 can drop from `pub(super)` to private; its only other caller is `PartialOrd for Version`). Either rename `receive` to `recv` and replace `happens_before`/`concurrent_with` at their four call sites (oracle/tests.rs:386-387, 401; clock/tests.rs:188) with `partial_cmp`-based spellings, or keep them and rewrite oracle/clock.rs:9-11 to say the oracle mirrors the paper's operations under its own names, listing the divergences (owned-`Version` messages, `receive` for `recv`, no n-ary or absorb entries). Rewrite oracle.rs:9-12 to name what the oracle mirrors (the paper's operations) rather than claim one omission. Reword clock/tests.rs:172-174 to name the comparisons the body performs (`>=`, `<`, `concurrent`); that site belongs to the clock partition and should be carried there. `Default for oracle::Version` stays. Acceptance: `grep -rn has_seen crates/before` returns nothing; both mirror sentences are true of the code beneath them; `just gate` clean.

### oracle-laws-2: The oracle's `join_all` transcribes production's balanced fold, twice, and the differential pins a hand-back shape the contract declares unspecified while the module doc claims paper transcription
- Where: crates/before/src/oracle/clock.rs:66-108 (related: crates/before/src/oracle/party.rs:101-143, crates/before/src/fold.rs:41-81, crates/before/src/party.rs:308-372, crates/before/src/clock.rs:229-240, crates/before/src/party/tests.rs:65-170 and 182-290 and 292-310, crates/before/src/clock/tests.rs:24-139, crates/before/src/laws.rs:2330-2437 and 3282-3388, crates/before/src/testing/validation_index.rs:172-173, crates/before/src/oracle/version.rs:377-379, crates/before/src/oracle.rs:6-9)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (read `oracle/clock.rs:66-108`, `oracle/party.rs:101-143`, and `fold.rs:47-80` side by side: identical control flow, identical retention policy, identical `expect` strings; read both production contracts; read both differential harnesses and the known-bad variant; `git show -s ace236595` and the b3f09baa0 hunks the history pass cited); executed: no
- Seen by: scaffolding [0], adequacy [12], structure-prose [24], instrument-correctness [36]; refutation: confirmed (all four), with the caveat that the copy check is live against the conditional-drop class via committed seed `cc 5d25e701...` and is blind only to a defect mirrored into both spellings; history: deliberate-but-expired (ace236595 transcribed "the exact fixed-accumulator up-front test, binary-counter grouping, and overlap-at-merge hand-back order the production contract documents"; b9f6af2d7 retired the counter-sharing `fold_oracle` because "only one algorithm was left live on both sides"; b3f09baa0, the owner's docs pass, rewrote both contracts to "unspecified" and deleted the oracle-side `join_all` docs that stated the transcription's purpose, leaving the pin without its definition-site rationale)
- Owner-gated: yes (a recorded design decision, ace236595 and party/tests.rs:65-75; the resolution changes what the differential pins)

The oracle module doc promises "no second representation to keep in sync" and a reference that is "obviously correct"; `join_all` on both oracle types is instead a token-level copy of `fold::balanced_try_fold`, spelled a third and fourth time in `party/tests.rs` (the known-bad variant) and `fold.rs` (Principle 3: machinery outlives the constraint that justified it; the tell is that a contract-conforming reshaping of the production fold can only be kept green by editing the oracle to match, which validation_index.rs:172-173 forbids). What the element-wise differential pins beyond the laws is the hand-back grouping and order, which `party.rs:314-315` and `clock.rs:236-237` leave unspecified; everything the contract does promise (acceptance iff pairwise disjoint, accepted fold equals sequential joins, nothing dropped, region conservation) is already stated oracle-free by `PARTY_AND_LIST` and `CLOCK_AND_LIST`. The oracle's own `Version::meet_all` shows the paper-faithful shape: a plain `reduce`.

Evidence:

    oracle/clock.rs:
        75	            let mut weight = 0u32;
        76	            while stack.last().is_some_and(|(_, w)| *w == weight) {
        77	                let (mut top, _) = stack.pop().expect("the loop condition saw a top entry");
        78	                match top.join(merged.take().expect("the operand is held while merging up")) {

    fold.rs:
        54	        let mut weight = 0u32;
        55	        while stack.last().is_some_and(|(_, w)| *w == weight) {
        56	            let (top, _) = stack.pop().expect("the loop condition saw a top entry");

    oracle.rs:
         6	//! `Party` and `Version` *are* the trees; every operation is a method, so there
         7	//! is no second representation to keep in sync. Deliberately simple,
         8	//! suboptimal, and recursive: its only job is to be obviously correct, so it
         9	//! can serve as differential ground truth. It mirrors the target's **semantic**

    party.rs (the production contract):
       314	    /// handed back. In case of partial error, the set of parties which are
       315	    /// absorbed vs. handed back is unspecified.

    clock/tests.rs (what the differential compares):
        98	/// Identical outcomes: the same `Ok`/`Err` verdict — the returned version
        99	/// lowering to the oracle accumulator's — the same hand-back vector (contents
       100	/// *and* order, element-wise over `to_oracle_clock`), and accumulators (party
       101	/// and version both) lowering to the same oracle trees.

    party/tests.rs (the surviving rationale):
        71	// The recursive oracle's `join_all` (`oracle::Party`) is that discipline's
        72	// reference spelling, and these differentials pin production against it across

    oracle/version.rs (the paper-faithful shape the oracle already uses for the meet):
       377	    pub fn meet_all(iter: impl IntoIterator<Item = Version>) -> Option<Version> {
       378	        iter.into_iter().reduce(|acc, v| acc & v)
       379	    }

Resolution: Decide first whether the fold's retention and drain discipline is a pinned behavior. If not: replace both oracle `join_all` bodies with the sequential reference (test each input against the fixed accumulator; refused inputs hand back individually in feed order; `for other in inputs { if let Err(back) = self.join(other) { overlapping.push(back) } }`), and widen the `Err` arm of both `assert_join_all_matches_recursive_oracle` harnesses to the contract: same verdict, same final accumulator, and equal region union (party) or region union plus history join (clock) of the hand-backs, not element-wise order. That comparison still pins the `IdIndex` accept seam (a wrongly accepted or refused input moves the accumulator and the union) and still convicts the dropped-group variant (on `[a, b, alias(a), c, d, e]` the variant returns `Ok` and loses `c`, so verdict and union both differ); restate `join_all_differential_convicts_the_dropped_group_oracle`'s doc accordingly. If the discipline is wanted pinned: say so in both `# Errors` sections and pin it once at `fold.rs` over a plain payload type (see oracle-laws-26), then retire the oracle copies per the retirement discipline. Do not route the oracle through `crate::fold`: b9f6af2d7 retired exactly that shape. In either branch, amend oracle.rs:6-9 so it no longer claims paper transcription for `join_all`. Acceptance: no weight-stack loop remains outside `fold.rs` and the known-bad test variant; the surface roster rows for `Party::join_all` and `Clock::join_all` still cite live bindings (citecheck green); `just gate` clean.

Construction: In `party.rs:362` reverse the closing drain (`for group in groups.into_iter().rev()`), a contract-conforming change since the absorbed set is unspecified. On the committed feed `[a, b, alias(a), c, d, e]` (party/tests.rs:105-115) the stack is `[a∪b, alias∪c∪d∪e]`; production now absorbs `alias∪c∪d∪e` and hands back `a∪b`, the oracle absorbs `a∪b` and hands back `alias∪c∪d∪e`. `join_all_agrees_with_oracle_on_aliased_coalesced_group` goes red. Every `PARTY_AND_LIST` law stays green by reading: the acceptance law's `Err` arm sees `!pairwise_disjoint`, a nonempty hand-back, and `acc.covers(p)`; the conservation law sees the same region union; the best-effort and reunion laws feed families the drain order cannot affect (their aliases are rejected up front, and their fork shares are pairwise disjoint). The only edit that returns the differential to green is reversing the drain at oracle/party.rs:133 as well.

### oracle-laws-3: `unreachable!("party overlap")` is a label, not a proof, and a denormal literal reaches the arm
- Where: crates/before/src/oracle/party.rs:82-82 (related: crates/before/src/oracle/party.rs:74-84, 92-98, 145-151; crates/before/src/oracle/tests.rs:250, 323, 501, 670, 683)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (traced `is_disjoint(Leaf(true), Node(Leaf(false), Leaf(false)))` through arm 148 to `true`, then `sum` through arms 76-81 to the fallthrough; `git show b3f09baa0 -- crates/before/src/oracle/party.rs` shows the arm replaced `_ => Party::Leaf(true), // overlap: unreachable (callers check disjointness)`); executed: no
- Seen by: scaffolding [10], adequacy [18], structure-prose [32], instrument-correctness [42]; refutation: confirmed (42 is the precise statement); history: no-rationale-found (owner-authored in b3f09baa0, replacing a silent wrong fallback whose comment carried the proof)
- Owner-gated: no

Every `expect`/`unreachable!` message is a one-line proof. The message names the condition, not why callers cannot reach it, and the premise it depends on (normal-form operands) is unstated: `is_disjoint` treats a denormal `Node(Leaf(false), Leaf(false))` as empty, after which `sum` pairs a full leaf with a node and panics. The oracle is deliberately unhardened (oracle.rs:14-32), so the message is the fix, not a guard.

Evidence:

        82	            _ => unreachable!("party overlap"),

       148	            (Party::Leaf(true), x) | (x, Party::Leaf(true)) => x.is_empty(),

Resolution: State the premise in the message, for example `unreachable!("sum runs only on normal-form operands that passed is_disjoint: join checks it, and the test populations are seed-derived")`, or make `sum` total by adding `(a, b) if b.is_empty() => a` and `(a, b) if a.is_empty() => b` arms ahead of the fallthrough. Acceptance: the arm's message names the premise and where it is established, or the arm is gone.

### oracle-laws-4: The oracle imports production's route-cost saturation for no observable payload, and the stated rationale names no constructible input
- Where: crates/before/src/oracle/version.rs:8-22 (related: crates/before/src/oracle/version.rs:287-289 and 299-311, crates/before/src/version/skyline/grow.rs:116-149, crates/before/src/testing/grow_brute_force.rs:31-38, crates/before/src/version/skyline/grow/tests.rs:63 and 319, crates/before/src/oracle/tests.rs:548-643)
- Class / severity / confidence: dependence / low / high
- Provenance: verified (grep for `grow_for_test`: the two production-side callers discard the cost with `let (raw, _) = ...`; the only cost consumers are oracle/tests.rs:560 and 579, which compare against `grow_brute_force`; read the `grow` arms: the `(Party::Leaf(false), _)` arm is reached only with an empty id at the top or a denormal `Node(empty, empty)`, since the `Node`/`Node` arm guards `il.is_empty()`/`ir.is_empty()` and every other arm recurses on nonempty ids; `git show fc06c5e8` shows the import and six `+ 1` rewrites landing together); executed: no
- Seen by: scaffolding [1], refutation (supporting item 3); refutation: confirmed; history: deliberate-and-holds (fc06c5e8's rationale is stated at oracle/version.rs:14-19 and grow.rs:137-142)
- Owner-gated: yes (a documented design decision with its rationale in code; this finding disputes the rationale)

The oracle module doc promises no second representation to keep in sync and ranks transcription fidelity above robustness; the `RouteCost` import is a sync point with production (fc06c5e8 changed 33 lines of the oracle alongside `grow.rs`) whose recorded rationale, "a debug overflow past the old bound" in the oracle's `+ 1`, names no input any harness constructs: `INFEASIBLE` originates only in an arm no normal nonempty id reaches, and a `u32` `+ 1` overflow needs `2^32` levels of an oracle tree that would overflow the stack long before (Principle 3: a coupling earns its place by naming what it serves outside itself). The brute force in the same suite made the opposite call and says why.

Evidence:

         8	use crate::version::skyline::grow::Cost as RouteCost;

        14	/// One more path level for a cost component, exactly the route DP's step
        15	/// ([`RouteCost::deepen`] at the production ceiling).
        16	///
        17	/// Infeasibility propagates, and feasible components saturate strictly below
        18	/// the [`RouteCost::INFEASIBLE`] sentinel, so a feasible chain of any depth
        19	/// stays feasible in every implementation of the fold at once.
        20	fn deepen(component: u64) -> u64 {
        21	    RouteCost::deepen(component, RouteCost::CEILING)
        22	}

       287	            (Party::Leaf(false), _) => {
       288	                (self.clone(), (RouteCost::INFEASIBLE, RouteCost::INFEASIBLE))
       289	            }

    grow/tests.rs:
        63	            let (raw, _) = to_oracle_version(v).grow_for_test(&to_oracle_party(p));

    grow_brute_force.rs:
        34	/// Deliberately unchecked exact arithmetic, no saturation and no infeasible
        35	/// sentinel: infeasibility is structural here (an empty enumeration /
        36	/// [`None`]), and depths are bounded by the enumerated test trees, so the
        37	/// brute force stays an independent witness of the DPs' saturating folds.

Resolution: Either give the oracle the brute force's idiom (`Option<Cost>` with `None` for an id that owns nothing here and plain `+ 1` deepening; the top-level empty-id arm returns `None`), drop the import, and remove "the recursive oracle" from grow.rs:137-140's client list; or keep the coupling and replace the doc at 14-19 with a rationale that names a constructible input (there is none under the harness caps, which is the point). Acceptance: `grep -rn RouteCost crates/before/src/oracle` is empty and the four grow-optimality proptests pass unchanged, or the doc at 14-19 states a reachable trigger.

Construction: Replace `deepen` with `component + 1` and the sentinel arm with `unreachable!`; every test in the crate passes (nothing observes the oracle's cost except the brute-force comparison, whose inputs are nonempty normal-form ids), which demonstrates the coupling carries no payload.

### oracle-laws-5: Three spellings of the grow-cost tuple with a hand-maintained "matches" comment
- Where: crates/before/src/oracle/version.rs:12-12 (related: crates/before/src/oracle/version.rs:355, crates/before/src/testing/grow_brute_force.rs:31-38)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read all three sites; `git show fc06c5e8` moved all three from `u32` to `u64` in one commit); executed: no
- Seen by: scaffolding [9], structure-prose [25, cost half]; refutation: confirmed (25's shim half refuted: `#[cfg(test)] pub(crate)` is the only way to scope crate visibility to test builds, so the wrappers are a taste trade, not a free deletion); history: no-rationale-found
- Owner-gated: no

The oracle's private `Cost`, `grow_for_test`'s literal `(Version, (u64, u64))`, and the brute force's `GrowCost` name one type three ways, held together by a sentence asserting they match (Principle 5: no hand-maintained restatements of facts the compiler can hold).

Evidence:

        12	type Cost = (u64, u64); // (#expansions, depth), lexicographic

       355	    pub(crate) fn grow_for_test(&self, id: &Party) -> (Version, (u64, u64)) {

    grow_brute_force.rs:
        33	/// Matches the oracle's `Cost` and the impl's `grow::Cost` component width.
        38	pub(crate) type GrowCost = (u64, u64);

Resolution: Make the oracle's `Cost` `pub(crate)`, return it from `grow_for_test`, use it in `grow_brute_force.rs` (dropping `GrowCost` and the "Matches" sentence). Folds naturally into oracle-laws-4's rework. Acceptance: one definition of the tuple type; no prose asserts the correspondence.

### oracle-laws-6: `join_off` and `meet_off` are one recursion differing only in the leaf combiner
- Where: crates/before/src/oracle/version.rs:114-154 (related: none)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: structure-prose [34]; refutation: confirmed (taste); history: no-rationale-found (c1e60b90's doc acknowledged the copy; b3f09baa0 deleted that sentence)
- Owner-gated: no

The two offset-threaded lattice recursions share every line except `.max` versus `.min`; twenty duplicated lines that must stay in step, in a file whose job is to be obviously correct. The paper does write join and meet as separate definitions, so this is a taste call with a named cost.

Evidence:

       117	            return Version::Leaf((so + sn).max(oo + on));
       138	            return Version::Leaf((so + sn).min(oo + on));

Resolution: Factor the shared recursion into one private helper taking the leaf combiner (`fn lattice_off(&self, so, other, oo, pick: fn(Base, Base) -> Base)`), keeping `join_off`/`meet_off` as one-line wrappers so the paper's names survive. Acceptance: one recursion body; `lattice`, `meet_semilattice`, and `lattice_absorption` pass unchanged.

### oracle-laws-7: Qualified `crate::` paths where an import would do, `std`/`core` mixing, the legacy `DefaultHasher` path, and a mid-clause comment wrap
- Where: crates/before/src/oracle/version.rs:211-212 (related: crates/before/src/oracle/version.rs:226-228 and 393, crates/before/src/oracle/tests.rs:808-813, crates/before/src/laws.rs:153, 263, 2058, 2998, 1213-1217)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `crate::Ticks\|crate::Rank` in `oracle/`; grep for `std::iter\|core::iter\|DefaultHasher` in `laws.rs`; `party/tests.rs:1026`, `codec/tests.rs:309`, and `version/tests.rs:2224` use `std::hash::DefaultHasher`); executed: no
- Seen by: structure-prose [30]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Imports over long qualified paths except where the qualification informs; here it does not (the oracle's own `Party`/`Version` do not collide with `Ticks`/`Rank`). `laws.rs` imports from `core` and spells `core::iter::once` at twelve sites but `std::iter::once`/`empty` at three, and its `hash_of` uses `std::collections::hash_map::DefaultHasher` where the crate's other tests use `std::hash::DefaultHasher`.

Evidence:

       211	    pub fn min_ticks(&self) -> crate::Ticks {
       212	        crate::Ticks(self.base_total())
       213	    }

    laws.rs:
       153	            std::iter::empty()
       263	    let mut hasher = std::collections::hash_map::DefaultHasher::new();
      1213	                // The membership argument shapes: owned version, coincident
      1214	                // span, and
      1215	                // the same endpoints re-decoded into distinct buffers (which

Resolution: Add `use crate::{Rank, Ticks};` to oracle/version.rs and oracle/tests.rs and drop the prefixes; pick one of `core::`/`std::` for `iter::once`/`iter::empty` in laws.rs; use `std::hash::DefaultHasher`; rewrap the comment at 1213-1217. Acceptance: no `crate::Ticks`/`crate::Rank` in `oracle/`; one spelling of `iter::once` in laws.rs; the comment reads as sentences.

### oracle-laws-8: Em-dashes in `//` line comments at ten sites
- Where: crates/before/src/oracle/version.rs:449-453 (related: crates/before/src/oracle/version.rs:469-471, crates/before/src/oracle/tests.rs:314 and 544, crates/before/src/laws.rs:1895, 1957, 3367-3368)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the seven partition files returns exactly these ten lines); executed: no
- Seen by: structure-prose [29]; refutation: confirmed (the convention is breached crate-wide, so the unit of repair is a sweep); history: no-rationale-found
- Owner-gated: no

The owner's doctrine prefers colons or semicolons over em-dashes in code comments (rendered `///` prose is exempt).

Evidence:

       451	// safe meet — intersecting two small disjoint shares could synthesize an
       452	// ancestor they share with a third live party, violating disjoint linearity —

Resolution: Replace each em-dash in a `//` comment with a colon, semicolon, parenthetical, or ` -- `; batch with the crate-wide sweep. Acceptance: the grep returns nothing.

### oracle-laws-9: The oracle test module doc says trees are never fabricated directly; three suites draw generated or literal trees
- Where: crates/before/src/oracle/tests.rs:5-7 (related: crates/before/src/oracle/tests.rs:437-443, 469-488, 499-506, 518-519, 556-559, 575-578, 593-596, 625-628, 722, 742, 759)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the module doc against the draws and literals listed); executed: no
- Seen by: instrument-correctness [41]; refutation: confirmed; history: deliberate-but-expired (true at b3320b385 apart from the paper literals; e27835ac6 and f0b83733 added the generator-driven suites; b3f09baa0 re-touched the sentence without re-truing it)
- Owner-gated: no

Prose speaks in the present tense and describes what is: the grow-optimality suite draws `arb_oracle_party_nonempty`/`arb_oracle_version`, the `covers`/`without` suites draw `arb_oracle_party`, and the paper worked examples build `V::Node`/`Party::Node` literals with `Arc::new`; a reader auditing the suite's input space from this header gets a false map.

Evidence:

         5	//! Values are generated via operations from a seed (always valid, normal-form,
         6	//! and — for populations — pairwise party-disjoint), never by fabricating
         7	//! trees directly, which might violate normality.

       437	    let left = V::Node(2u64.into(), Arc::new(leaf(1)), Arc::new(leaf(0))); // (2,1,0), already normal

       557	        id in arb_oracle_party_nonempty(),
       558	        e in arb_oracle_version(),

Resolution: Restate the header: values come from seed-derived op traces (pairwise party-disjoint populations), from the normalizing arbitrary generators, or from paper literals whose normality the test asserts or which are already normal. Acceptance: the module doc names all three input sources the file uses.

### oracle-laws-10: The arbitrary generators' normal-form claim has no direct pin
- Where: crates/before/src/oracle/tests.rs:42-60 (related: crates/before/src/testing/generators.rs:334-366, crates/before/src/oracle/version.rs:331-339)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn 'is_normal()'` over `src/`, `tests/`, `fuzz/`: every pin is over op-trace outputs, decode results, the exhaustive enumeration, or an operation's output; none names `arb_oracle_party()`/`arb_oracle_version()` output, and `testing/generators/tests.rs` has no `is_normal` call); executed: no
- Seen by: instrument-correctness [38]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The generators' docs assert normal form by construction and the oracle's `event` relies on it (the `filled != *self` structural comparison at version.rs:333 is semantic equality only on normal forms), but no test asserts `is_normal()` on a generated tree; a regression in `node`'s normalization would surface as differential failures far from the cause rather than by name (Principle 8: a claim is verified by an instrument, not by the sentence stating it).

Evidence:

        43	    /// Every value any op produces is in normal form (parties and versions),
        44	    /// including the result of a join.
        45	    #[test]
        46	    fn normal_form(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {

    generators.rs:
       358	/// (including values near/beyond `u64::MAX`); every interior node goes through
       359	/// the oracle's normalizing `Version::node`, so the result is always in normal
       360	/// form (a zero-base child at every node, no collapsible `(n, m, m)`).

Resolution: Add one proptest beside `normal_form` (or in `testing/generators/tests.rs`) asserting `arb_oracle_party()` and `arb_oracle_version()` outputs satisfy `is_normal()`. Acceptance: removing the `debase` step from `Version::node` (version.rs:82-84) or the collapse arm from `Party::node` (party.rs:24-25) fails the new test by name.

Construction: Change `Version::node` to build `Version::Node(n, Arc::new(l), Arc::new(r))` unconditionally and run the oracle suite: `normal_form` and the paper examples fail through `join_off`, but no failure names the generator's output as the non-normal artifact.

### oracle-laws-11: Pool indices are drawn as `0..64` and reduced modulo the population at fourteen sites where the crate elsewhere uses `prop::sample::Index`
- Where: crates/before/src/oracle/tests.rs:46-55 (related: crates/before/src/testing/generators.rs:428-432, crates/before/src/clock/tests.rs:176-179)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (`grep -c '% n\]' crates/before/src/oracle/tests.rs` = 14; `sample::Index` is used at generators.rs:430-432, span/tests.rs:730, skyline/tests.rs:171 and 497, text/tests.rs:456-457, borsh_impls/tests.rs:407-408); executed: no
- Seen by: structure-prose [33]; refutation: confirmed (no coverage hole: `world_strategy` populations have at most 30 members, below 64); history: no-rationale-found (Phase-0 text; `sample::Index` entered the crate later)
- Owner-gated: no

Prefer the library's tool over hand-rolling, and one idiom across the crate: the modulo skews toward low indices when `n` does not divide 64, shrinks toward `i = 0` rather than toward a smaller population, and hides the intent ("pick a member") behind arithmetic.

Evidence:

        46	    fn normal_form(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        53	        let vs = versions(&cs);
        54	        let n = vs.len();
        55	        let joined = vs[i % n].clone() | vs[j % n].clone();

Resolution: Replace `i in 0usize..64` with `i in any::<prop::sample::Index>()` and `vs[i % n]` with `vs[i.index(n)]` at the fourteen sites (the same pattern at clock/tests.rs:176-179 belongs to the clock partition). Acceptance: the grep count is 0.

### oracle-laws-12: A dead `prop_assume!` where the sibling tests state the premise
- Where: crates/before/src/oracle/tests.rs:597-600 (related: crates/before/src/oracle/tests.rs:561 and 580, crates/before/src/testing/grow_brute_force.rs:44)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read: line 594 draws `arb_oracle_party_nonempty()`; grow_brute_force.rs:44 states `all_inflations` is empty iff the id owns nothing); executed: no
- Seen by: adequacy [19], instrument-correctness [43]; refutation: confirmed; history: no-rationale-found (dead from birth in e27835ac6)
- Owner-gated: no

A guard must name a constructible case it filters (Principle 3); this assume never rejects, misstates the population, and the two `unwrap`s hide the same premise the siblings spell as `.expect("non-empty id always has an inflation")`.

Evidence:

       597	        let cands = all_inflations(&id, &e);
       598	        prop_assume!(!cands.is_empty());
       599	        let min = cands.iter().map(|(_, c)| *c).min().unwrap();
       600	        let (best_tree, best_cost) = best_inflation(&id, &e).unwrap();

Resolution: Drop the assume and use the siblings' `.expect("non-empty id always has an inflation")` on both results. Acceptance: the three grow-optimality tests state the nonempty-id premise the same way.

### oracle-laws-13: Two oracle test doc comments claim invariants their bodies never assert
- Where: crates/before/src/oracle/tests.rs:795-829 (related: none)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read: the `version_min_ticks` body has no join of two versions; the `clock_own_version` body never calls `tick`; line 809 is implied by 808 for a natural count); executed: no
- Seen by: adequacy [13], structure-prose [21], instrument-correctness [37]; refutation: confirmed (severity argued down because production `min_ticks` is pinned elsewhere; I keep medium because the doc claims coverage of the oracle's own additivity, which nothing in the crate checks); history: no-rationale-found (doc and body landed mismatched in f0b83733; 56a08f90 re-typed the body only)
- Owner-gated: no

Every test's doc comment states its invariant in English and must be accurate; the crate's AGENTS.md says review holds it to that standard, and the doctrine calls an incorrect one a bug in the test. `version_min_ticks` promises additivity under a disjoint-support join and non-domination by either operand; `clock_own_version` promises that ticking raises the own version; neither body checks its promise, so a coverage audit reading these docs is misled.

Evidence:

       796	    /// `min_ticks` is the sum of every base, so it is `0` only for the zero
       797	    /// version, additive under a disjoint-support join (`(a|b)` over fork
       798	    /// halves sums their counts), and dominated by neither operand alone.

       808	        prop_assert_eq!(v.min_ticks() == crate::Ticks::ZERO, *v == Version::new()); // zero iff the zero version
       809	        prop_assert!(v.min_ticks() >= crate::Ticks::from(1u64) || *v == Version::new());
       810	        // The seed ticked once in a line costs exactly one.
       811	        let mut one = Version::new();
       812	        one.tick(&Party::seed());
       813	        prop_assert_eq!(one.min_ticks(), crate::Ticks::from(1u64));

       818	    /// `own_version` is exactly `version() / party()`: the clock's history within
       819	    /// the region it owns. It is a sub-version of the full version, and ticking
       820	    /// (which advances only the owned region) raises it.

       826	        prop_assert_eq!(c.own_version(), c.version() / c.party()); // the definition
       827	        prop_assert!(leq(&c.own_version(), &c.version())); // a sub-version

Resolution: Assert the claims or trim the docs. Additivity is constructible on every call: fork a trace member's party into `keep`/`give`, take `x = v / &keep` and `y = v / &give`, assert `(x | y).min_ticks() == x.min_ticks() + y.min_ticks()` and `(x | y).min_ticks() >= x.min_ticks().max(y.min_ticks())`. For `clock_own_version`, tick a clone and assert `own_version` strictly rises under `leq`. Delete the redundant line 809 either way. Acceptance: each sentence of both doc comments corresponds to an assertion in its body.

Construction: Read 796-798 against 808-813 and 818-820 against 826-827: no assertion involves a join of two versions or a `tick` of the clock, so the documented additivity and tick-raises clauses have no corresponding check.

### oracle-laws-14: Two incidental-only laws lack a constructed arm, and nothing measures antecedent liveness
- Where: crates/before/src/laws.rs:23-26 (related: crates/before/src/laws.rs:2770-2772, 601-603, 2250-2252, 3141-3143; crates/before/src/testing/algebraic_laws/tests.rs:105-115)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read each conditional law; confirmed the constructed twins the refutation named: `without_inverts_fork` 2119-2125, `fork_halves_covered_by_parent` 2044-2048, `meet_join_absorption` 474-476, `lag_is_the_rank_gap` 576-578 with `lag_zero_iff_dominated` 570-572; grep for antecedent/liveness/vacuous in the drivers returns nothing); executed: no
- Seen by: adequacy [14]; refutation: reframed (four of the six cited laws have constructed twins under other names; the party-group line numbers in the original were +200 off); history: no-rationale-found (the policy and the older implications date from the founding commit; the constructed-plus-incidental pattern was applied to newer laws in d4c7d827 and 8350c556 without revisiting these)
- Owner-gated: no

The module doc commits to constructing antecedent witnesses where possible. `disjoint_projections_share_nothing` waits for a disjoint party pair, where a fork half is constructible as `without_inverts_fork` does; the three `*_eq_implies_hash_eq` laws wait for `a == b`, where `decode(encode(a))` constructs the interesting case (equal value, distinct buffer) on every call. Under the arbitrary drivers, which draw pairs independently and exist to reach shapes the op pipeline never produces, these antecedents fire only by chance, and no floor in the suite would report a law that had gone vacuous.

Evidence:

        23	//! law holds unconditionally on the inputs its group admits (below);
        24	//! conditional laws are stated as implications, vacuously true when the
        25	//! antecedent fails, and where they can, they *construct* a witness for the
        26	//! antecedent instead of waiting for one.

      2770	    fn disjoint_projections_share_nothing {
      2771	        !p.is_disjoint(q) || ((v / p).to_version() & (v / q).to_version()).is_empty()
      2772	    }

       601	    fn version_eq_implies_hash_eq {
       602	        a != b || hash_of(a) == hash_of(b)
       603	    }

Resolution: Add a constructed arm to each (the `constructed && incidental` shape at 933-937): `disjoint_projections_share_nothing` on `(keep, give)` from `p.dangerously_alias().fork()`; the eq/hash trio on `(a, decode(encode(a)))`. Optionally one deterministic test asserting each incidental antecedent is satisfiable on a small fixed population, as a liveness pin. Acceptance: every conditional law either runs a constructed arm on every call or carries a comment naming why none is constructible; law names unchanged.

Construction: Locally count antecedent hits in the four predicates and run only the `version_pair_laws`, `party_pair_laws`, `clock_pair_laws`, and `version_party_pair_laws` drivers (organic driver disabled) at the default 256 cases; a zero count demonstrates that the arbitrary-regime driver asserts nothing for that law, and nothing committed would report it.

### oracle-laws-15: `le` is `le_by` at one type
- Where: crates/before/src/laws.rs:248-258 (related: crates/before/src/testing/surface_coverage/tests.rs:168-171)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read both bodies; the surface-coverage negative probe names `"le"`); executed: no
- Seen by: scaffolding [8], structure-prose [31]; refutation: confirmed; history: no-rationale-found (`le_by` was added the same day for view comparisons)
- Owner-gated: no

Two names for one predicate in a file whose job is naming predicates exactly; `le_by::<Version, Version>` is every `le` call.

Evidence:

       249	fn le(a: &Version, b: &Version) -> bool {
       250	    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
       251	}

       256	fn le_by<L: PartialOrd<R>, R>(a: &L, b: &R) -> bool {
       257	    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
       258	}

Resolution: Keep the generic body under the name `le` and delete the other; surface_coverage/tests.rs:169's `!laws.contains(&"le")` probe is unaffected if the surviving name is `le`. Acceptance: one `le`-family helper in laws.rs.

### oracle-laws-16: Bundled laws report one name for seven to ten independent clauses
- Where: crates/before/src/laws.rs:383-402 (related: crates/before/src/laws.rs:696-739, 1309-1359, 1369-1404, 1419-1472, 1486-1540, 2691-2717, 3401-3447, 75; crates/before/src/surface.rs `Leg::Law` citations of the bundled names)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read the bodies; `assert_laws!` and `drive!` report only the bundle name); executed: no
- Seen by: structure-prose [26]; refutation: confirmed (and owner-gated in practice); history: no-rationale-found
- Owner-gated: yes (`Law<F>` is a `pub type` under the `laws` feature; splitting moves roster citations)

The module's stated payoff is that a harness "reports the *name* of any law that fails". The bodies already bind sub-results to descriptive names (`definitional`, `accessors`, `reborrowed`, `settled`, `commutative`, ...) and then discard them in one `&&` chain, so a shrunk fuzz input arrives with a bundle name and the failing clause must be bisected by hand, at exactly the moment the instrument is supposed to help.

Evidence:

         7	//! Each law is a `(&str, fn(...) -> bool)` pair in a slice grouped by predicate
         8	//! signature, so a harness iterates a slice, feeds every law the same inputs,
         9	//! and reports the *name* of any law that fails. The crate's law proptests

       731	        definitional
       732	            && accessors
       733	            && reborrowed
       734	            && settled
       735	            && commutative
       736	            && flip_subsumed
       737	            && empty_edge
       738	            && unary_edge

        75	pub type Law<F> = (&'static str, F);

Resolution: Either split each bundle into one law per clause (the `surface.rs` citations of `ranked_carries_own_rank` and `span_is_the_pair_hull` move with them), or change the predicate to `fn(...) -> Result<(), &'static str>` returning the failing clause's name and have the three consumers report `law/clause`. The second keeps roster and citations stable but changes a feature-gated public type. Acceptance: a deliberately negated inner conjunct (say `commutative` in `span_is_the_pair_hull`, in a scratch build) produces a driver or fuzz report naming that clause.

Construction: Negate `commutative` in `span_is_the_pair_hull` and run the `version_pair_laws` driver: the report names `span_is_the_pair_hull` with the shrunk pair and nothing identifies which of the eight named booleans went false.

### oracle-laws-17: Law predicates that `unwrap` lose the failure's name
- Where: crates/before/src/laws.rs:699-699 (related: crates/before/src/laws.rs:712-714, 725-726, 1219, 1222, 1314, 1426, 1491, 1848, and the helper at 1705; crates/before/src/testing/algebraic_laws/tests.rs:38-44; crates/before/fuzz/fuzz_targets/fuzz_laws.rs:7-8 and 86-92)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep over laws.rs for `.unwrap()\|.expect(`; ten sites inside `laws!` bodies plus one in `operand_spans`; the let-else idiom is already used at 661-664, 1041-1044, 1111-1114, 2698-2700; the two `split_last().expect(...)` at 1822 and 1870 are infallible); executed: no
- Seen by: scaffolding [3], instrument-correctness [40]; refutation: confirmed; history: no-rationale-found (the let-else idiom predates the unwraps)
- Owner-gated: no

`assert_laws!` and `drive!` emit `law violated: {name}` only when the predicate returns `false`; a panic inside the predicate reports a file and line instead, breaking the fuzz target's stated contract ("A violated law is a panic naming the law"). The same antecedent (a meet/join pair is ordered) is handled two ways in one file.

Evidence:

       699	        let definitional = hull == Span::new(&meet, &join).unwrap();

       661	        let Ok(span) = Span::new(b, b) else {
       662	            // A version is always ordered with itself.
       663	            return false;
       664	        };

    algebraic_laws/tests.rs:
        41	            prop_assert!(law($($input),+), "law violated: {}", name);

Resolution: Use the let-else `return false` idiom at 699, 713, 725, 726, 1219, 1222, 1314, 1426, 1491, 1848 (a small `fn ordered(lo, hi) -> Option<Span<'_>>` beside `within` keeps the bodies short); `operand_spans` at 1705 may keep its `expect` or return `Option`. Acceptance: `grep -n 'unwrap()\|expect(' crates/before/src/laws.rs` returns only the two infallible `split_last` sites (and optionally 1705).

### oracle-laws-18: The least/greatest bound laws check only a join-built bound; the clause that means "least" is absent
- Where: crates/before/src/laws.rs:861-877 (related: crates/before/src/oracle/tests.rs:125-128 and 171-174, crates/before/src/laws.rs:536-538)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: structure-prose [35]; refutation: confirmed (the gap is in the named law's discriminating power, not coverage: `rank_is_a_valuation` fails for any join that over-approximates the LUB); history: no-rationale-found (Phase-0 construction transcribed into the laws)
- Owner-gated: no

Every clause is the upper-bound law on a pair built from the join, so the law named for least-ness adds no discriminating power over `merge_is_upper_bound`; least-ness is the implication `a <= c && b <= c ==> a | b <= c` for an arbitrary `c`, which the triple group has in hand and never states (dually for the meet).

Evidence:

       864	    fn merge_is_least_upper_bound {
       865	        let ab = a | b;
       866	        let upper = &ab | c;
       867	        le(a, &upper) && le(b, &upper) && le(&ab, &upper)
       868	    }

Resolution: Conjoin the incidental clause `!(le(a, c) && le(b, c)) || le(&ab, c)` (dually `!(le(c, a) && le(c, b)) || le(c, &ab)` for the meet), and the same at oracle/tests.rs:125-128 and 171-174. Acceptance: each "least"/"greatest" law contains a clause whose bound is not constructed from the operation under test.

### oracle-laws-19: "door" is an undefined coinage used 41 times in laws.rs
- Where: crates/before/src/laws.rs:1723-1724 (related: crates/before/src/laws.rs:341-342, 1782-1783, 2327, 3265; crates/before/src/oracle/version.rs:370 and 375; 237 uses across `crates/before/src`)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -c door` laws.rs = 41, lib.rs = 0, crate-wide 237; the owner's lexicon at `~/.claude/writing-style.md:383` lists `| door | entry point |`); executed: no
- Seen by: structure-prose [22]; refutation: confirmed (severity lowered: vocabulary only, crate-wide, owner's call); history: contradicts-hard-rule (writing-style.md:164-169, the anchoring rule)
- Owner-gated: no

Every coined term must be anchored to an identifier or defined once by contrast; "door" ("join doors", "n-ary door", "validating door", "materialization doors", "without doors") is neither an identifier nor defined anywhere in the crate, and a new maintainer must infer from context whether a door is a method, an operator impl, a trait impl, or a constructor. The lexicon lists it as a case where the plain term is strictly clearer.

Evidence:

      1723	    /// The seedless iterator join doors (`Sum` and `FromIterator`, owned and
      1724	    /// borrowed) against their sequential pair-operator oracle, and their

Resolution: Replace with the plain term at each site ("entry point", or the method or trait name: "the `Sum` and `FromIterator` impls", "the n-ary method"), or, if the word is kept, define it once in the crate-level docs and link that definition from laws.rs's module doc. The partition-local edit is mechanical; the crate-wide decision is Finch's (see open questions). Acceptance: `grep -c door crates/before/src/laws.rs` returns 0, or one definition site exists and laws.rs links it.

### oracle-laws-20: `span_all_is_the_family_hull`'s containment clause admits `Concurrent` placements its own argument rules out
- Where: crates/before/src/laws.rs:1850-1851 (related: crates/before/src/laws.rs:1834-1836, 1714-1716, 1108-1122)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read); executed: no
- Seen by: adequacy [16]; refutation: confirmed, severity lowered to nit (`span_place_matches_relations` already pins `place`, so the tightening catches nothing new); history: no-rationale-found (the clause predates the `within` helper)
- Owner-gated: no

The doc's argument (the meet bounds each input from below and the join from above) proves `At(_) | Between`, which is exactly `within`; the clause as written passes `Concurrent(_)`.

Evidence:

      1850	        let contained =
      1851	            family().all(|v| !matches!(hull.place(v), Placement::Before | Placement::After));

      1714	fn within(span: &Span<'_>, probe: &Version) -> bool {
      1715	    matches!(span.place(probe), Placement::At(_) | Placement::Between)
      1716	}

Resolution: Use `family().all(|v| within(&hull, v))` and restate the doc as "places at an endpoint or between them". Acceptance: the clause uses `within`; the law is green under all three drivers.

### oracle-laws-21: The acceptance laws' `Err` arms assert less than their docs say, and the clock group has no best-effort law
- Where: crates/before/src/laws.rs:2345-2345 (related: crates/before/src/laws.rs:2325-2329, 2390-2400, 3278-3281, 3302-3307)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read: the accumulator starts as an alias of the receiver and only grows through `join`, so `acc.covers(p)` holds for any fold that does not corrupt `self` on refusal; `CLOCK_AND_LIST` has six laws and none plants a mid-stream clash); executed: no
- Seen by: refutation (new item 1); refutation: raised; history: not examined
- Owner-gated: no

The docs gloss "a refused one still absorbed every region it could" as "the accumulator covers its original region", but covering the original region shows only that the accumulator never shrank. The absorb-around-the-clash claim is carried by `party_join_all_is_best_effort_at_any_width` alone, for parties only; the clock group has no twin, so its collection cannot on its own distinguish best-effort from fail-fast (the oracle differentials in `clock/tests.rs` can, which is one more reason oracle-laws-2 must keep an accumulator comparison).

Evidence:

      2325	    /// An accepted fold equals the sequential pair joins (the bound pair
      2326	    /// operation, never the n-ary door, so the two sides cannot share a
      2327	    /// broken arm); a refused one still absorbed every region it could —
      2328	    /// the accumulator covers its original region — and handed at least
      2345	            Err(returned) => !pairwise_disjoint && !returned.is_empty() && acc.covers(p),

      3305	                    && acc.party().covers(c.party())
      3306	                    && le(c.version(), acc.version())

Resolution: Weaken both docs to what the clause checks (the accumulator is never corrupted on refusal), and add `clock_join_all_is_best_effort_at_any_width` as the twin of the party law (fork `width` children, tick them apart, plant an alias of the keeper mid-stream, expect exactly the alias back and the keeper's party restored with the join of every line's version). Acceptance: each `Err`-arm sentence maps to a clause; a fail-fast `Clock::join_all` fails a `CLOCK_AND_LIST` law by name.

Construction: Make `Clock::join_all` fail-fast (on the first `accept` failure, push the remaining inputs to `overlapping` and stop). Every `CLOCK_AND_LIST` law stays green by reading: the acceptance law's `Err` arm sees `!pairwise_disjoint`, a nonempty hand-back, an accumulator covering its origin and dominating its original version; the conservation law sees every unabsorbed input handed back; the reunion, `sync_all`, `recv_all`, and `absorb_all` laws feed pairwise-disjoint families. Only `clock/tests.rs`'s oracle differentials convict it.

### oracle-laws-22: Prose tells: the banned "minted", "THE LAW", moralized "real"/"honest", significance adverbs, the undefined "under mass" and "faces", and temporal "still works"/"survive here"
- Where: crates/before/src/laws.rs:2956-2956 (related: crates/before/src/laws.rs:3236, 3473, 2913, 2472, 1036, 2354, 2318, 2781, 2309, 3258, 3352; crates/before/src/laws/tests.rs:69-74, 81, 107; crates/before/src/oracle/version.rs:28, 388-389, 472-473; crates/before/src/oracle/tests.rs:544; crates/before/src/oracle.rs:40)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep over the partition files for each word; `~/.claude/writing-style.md:170` bans "mint" outright, :326-330 asks for the property in place of "real"/"genuine"/"honest", :404-409 lists red/green, tripwire, and "keeps it honest"; "under mass" has seven uses crate-wide and no definition; `git log -S'impl DivAssign' -- version.rs` shows production's by-value `Div`/`DivAssign` removed in 6c88c2ad0 on the day oracle/version.rs:473 was written); executed: no
- Seen by: scaffolding [7], structure-prose [27, 28]; refutation: confirmed (one citation corrected: laws.rs:2382 reads "genuine shares", not "genuinely"); history: contradicts-hard-rule for "mint", "real"/"honest"/"genuinely", and "red"; the "survive here" clause is intelligible only relative to a production removal, so it refers to code that no longer exists
- Owner-gated: no

The vocabulary rule: never write "mint" for constructing a value (here stretched to recording a tick, where production's own docs say "without marking an event"); describe code by the property that holds rather than "real", "honest", "genuinely"; anchor every coinage ("under mass", "faces" for the two operand types of one law). "THE LAW" in capitals is emphasis without a semantic distinction. "still works" and "survive here" imply a history the present-tense rule excludes. On "tripwire" at laws/tests.rs:69: the crate defines the word broadly at surface_coverage.rs:71-92 to include liveness anchors, so the site matches the crate's definition and conflicts with the lexicon's ("a test a known-bad implementation fails"); the dispute is with the crate-local definition, and "deterministic witness" is right at this site either way.

Evidence:

      2956	    /// anonymous join with no event minted, `sync` reconciles a fork,
      3236	    /// `absorb` is the anonymous join with no event minted: the version
      2913	    /// THE LAW of the rank wire form: byte-wise lexicographic order on
      2472	    /// `tick`'s inflation is real *within* the region (§4: `f · i ⊐ 0`): the
      2318	    /// the refusal arm under mass, with two and more distinct

    oracle/version.rs:
        28	/// truncation point. Literal/`u64` construction still works via
       388	    /// [`Version::ticks`](crate::Version::ticks) is the definitionally
       389	    /// honest loop — `O(n · tree)`, fit for small `n` only (the module
       473	// by-value and assign forms survive here for the oracle's own ergonomics.

    laws/tests.rs:
        69	/// discipline-level drop would lose — the deterministic tripwire beside
        70	/// the pool-driven law family, red the day the fold misroutes that group
        71	/// (dropping it outright flips the verdict, which the acceptance laws
        72	/// police; dropping it only when the rejection channel is already

Resolution: "no event minted" -> "without marking an event" (production's phrasing at clock.rs:494) at 2956, 3236, 3473; "THE LAW of the rank wire form" -> "The rank wire form's defining law"; "is real *within*" -> "is strict within"; "the definitionally honest loop" -> "the literal loop"; drop "genuinely" at laws.rs:1036, 2354, laws/tests.rs:74, oracle/tests.rs:544; "under mass" -> "on most samples" or define it once in testing/generators.rs where it originates; "faces" -> "the party and clock instances"; "still works" -> "works"; "survive here for" -> "exist for"; "not real public API" (oracle.rs:40) -> "a test and bench reference, not supported API"; rewrite laws/tests.rs:67-76 as mechanism ("This feed order makes the closing drain hand back a coalesced group; the conservation laws hold on it, and a fold that dropped that group would fail them while passing the acceptance laws' `Err` clauses"). Acceptance: `grep -n -i 'minted\|THE LAW\|honest\|genuinely\|under mass\|survive here\|still works\|red the day\|police' crates/before/src/laws.rs crates/before/src/laws/tests.rs crates/before/src/oracle crates/before/src/oracle.rs` returns nothing.

### oracle-laws-23: `clock_ticks_matches_version_ticks` pins one hard-coded count where the version-level twin quantifies
- Where: crates/before/src/laws.rs:3033-3033 (related: crates/before/src/laws.rs:2523-2530 and 2537-2542)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read both laws); executed: no
- Seen by: adequacy [20]; refutation: confirmed; history: no-rationale-found (56a08f90 introduced the literal beside a sibling that draws `a.min_ticks()`)
- Owner-gated: no

Named constants over magic numbers, and a family stated as a point when the operand already supplies a quantified count: the clock-level law never exercises the fused path at width.

Evidence:

      3033	        let n = Ticks::from(3u64);

      2524	        let n = a.min_ticks();

Resolution: Use `c.version().min_ticks()` (or `min_ticks() + 1` to avoid the zero-count identity on fresh clocks) as the version-level law does. Acceptance: no literal count in the law; green under all drivers.

### oracle-laws-24: `law_names_are_unique_across_groups` restates a guarantee `laws!` gives at compile time, and its doc overstates its role
- Where: crates/before/src/laws/tests.rs:9-21 (related: crates/before/src/laws.rs:186-246, 198, 236, 244)
- Class / severity / confidence: scaffolding / nit / high
- Provenance: assessed (read: `laws!` emits every law as a module-scope `fn $law` and registers `stringify!($law)`, so two same-named laws anywhere in laws.rs are a duplicate-definition compile error); executed: no
- Seen by: adequacy [17]; refutation: confirmed; history: deliberate-but-expired (the test guarded a real door when groups were hand-written tables; b3f09baa0's `laws!` closed it mechanically without re-denominating the doc)
- Owner-gated: no

A check earns its place by naming what it catches that nothing else does (Principle 3); the compiler is the mature tool here. The only constructible failure is a hand-written group bypassing `laws!` that registers a foreign fn under another law's name, which the doc does not mention.

Evidence:

         9	/// Every law name is unique across all groups, so a failure anywhere names
        10	/// exactly one law — the property the whole collection exists to provide.
        11	#[test]
        12	fn law_names_are_unique_across_groups() {

    laws.rs:
       198	        $(#[$group_meta])* pub static $group: &[Law<fn($($ty),+) -> bool>] = &[$((stringify!($law), $law)),+];
       236	        fn $law($($param: $ty),+) -> bool $body

Resolution: Either dissolve the test, or keep it and re-document the door it guards (a hand-authored `pub static` group registering a name that is not its fn's). Acceptance: doc and test agree on what the test catches, or the test is gone and the totality pin remains.

### oracle-laws-25: The law-group totality pin is a source-text scan where a compile-time tie exists, and the scan is not total over the syntax `laws!` accepts
- Where: crates/before/src/laws/tests.rs:36-61 (related: crates/before/src/laws.rs:136-166, 186-206, 192; crates/before/src/testing/diff_ops.rs:954-964 and crates/before/src/testing/diff_ops/tests.rs:169, which carry the same pattern)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified for the gap (laws/tests.rs:42 matches only lines whose trimmed text begins `pub static `, while the matcher at laws.rs:192 accepts `$(#[$group_meta:meta])* pub static`; `grep -n '^\s*#\[.*pub static' crates/before/src/laws.rs` finds no attributed invocation header today, so the hole is latent); assessed for the replacement (not compiled); executed: no
- Seen by: scaffolding [4], instrument-correctness [39]; refutation: confirmed (4's compile-time tie dissolves 39); history: deliberate-and-holds for the scan's existence (b800537d chose it because the surface-totality extractor walks function-like items only; 5c990b90 retained it explicitly; b3f09baa0, the owner, added the one-line `pub static` convention to keep it honest), but no record weighs a compile-time reference, so this is a fresh proposal against an owner-authored mechanism
- Owner-gated: yes (replaces an instrument; the replacement must demonstrate it catches what the scan catches)

Infrastructure is suspect when it carries its own stabilization conventions (the one-line `pub static` spelling rule on the macro) and reimplements a capability the compiler provides. The reverse direction (a rostered phantom) is already a compile error because `registered_names` chains `$group.iter()`; only the unrostered-group direction needs a pin, and a name reference gives it without a file read, a parse, or a formatting rule. The scan's stated reason (surfacecheck cannot see statics) argues against relying on surfacecheck, not against a compile-time tie. The scan is also not total: `#[allow(dead_code)] pub static EXTRA: (a: &Version);` on one line compiles, registers, and escapes the prefix match.

Evidence:

        41	    for line in text.lines() {
        42	        if let Some(rest) = line.trim_start().strip_prefix("pub static ") {
        43	            let name: String = rest
        44	                .chars()
        45	                .take_while(|c| c.is_alphanumeric() || *c == '_')
        46	                .collect();

    laws.rs:
       187	    // In both the matcher and the transcriber, attributes and the declaration
       188	    // share a line: the totality pin's source scan reads any line starting `pub
       189	    // static` as a group declaration, and must see the invocations' headers
       190	    // only, never this definition.
       192	        $(#[$group_meta:meta])* pub static $group:ident: ($($param:ident: $ty:ty),+ $(,)?);

Resolution: Have `emit_registration` (already expanded from the roster, `#[cfg(test)]`) also emit `mod rostered { pub(super) use super::{VERSION_SOLO, ...}; }` from the roster, and have `laws!` append a `#[cfg(test)] fn` (or `const _`) whose body references `rostered::$group`; an unrostered group then fails `cargo check --tests` at its own declaration, the text scan and the convention comment at laws.rs:187-190 go, and `REGISTERED_GROUPS` dissolves or reduces to the alias module. `diff_ops.rs` carries the same scan pattern and would take the same tie. If the scan is kept instead, strip leading `#[...]` groups before matching (or match `pub static ` anywhere on lines without `$`) so it is total over the macro's syntax. Acceptance: laws/tests.rs has no `fs::read_to_string`; declaring `laws! { pub static PHANTOM: (a: &Version); fn x { true } }` without a roster entry fails to compile; or, in the fallback, the attributed-header case fails `every_law_group_is_registered`.

Construction: Insert `#[allow(dead_code)] pub static EXTRA: (a: &Version); fn extra_law { true }` as a `laws!` block without adding `EXTRA` to `for_each_law_group!`, run `laws::tests::every_law_group_is_registered`: it passes, while no driver, organic drive, or fuzz loop ever executes `extra_law`.

### oracle-laws-26: The fold's retention-arm witness lives beside the laws while `fold.rs` has no tests
- Where: crates/before/src/laws/tests.rs:63-135 (related: crates/before/src/laws/tests.rs:1-3; crates/before/src/fold.rs:18-81; crates/before/src/party/tests.rs:89-115 and 172-290; crates/before/src/clock/tests.rs:65-93; crates/before/src/laws.rs:2402-2437)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -n 'mod tests\|#\[cfg(test)\]' crates/before/src/fold.rs` returns nothing; read the three deterministic witnesses of the same arm); executed: no
- Seen by: scaffolding [6]; refutation: confirmed (also: no committed known-bad artifact fails the laws on this feed; the dropped-group variant is convicted only by the differential comparison); history: no-rationale-found (fold.rs was born without tests in b4093db21, "Pure refactor: all fold/join/meet/sync suites green"; f42ce3d4 placed this witness in laws/tests.rs without saying why)
- Owner-gated: no

A capability in the wrong layer generates per-consumer copies: the retention arm is one branch of `balanced_try_fold` (fold.rs:65-72) and needs one witness over a trivial combiner at its declaration, yet it is witnessed three times over parties and clocks, and `fold.rs`, whose module doc calls itself "the one home for the counter discipline", carries no test at all. This test also sits in a module whose doc scopes it to the collection's own invariants while it exercises production fold behavior.

Evidence:

         1	//! Guards on the law collection itself (the laws are *asserted* by the drivers
         2	//! in [`crate::testing`]'s algebraic-laws suite and by the fuzz workspace; here
         3	//! we pin the collection's own invariants).

        63	/// The conservation laws' deterministic witness: the feed order
        64	/// [a, alias(a), b, alias(b), c] holds both `join_all` conservation laws
        65	/// while its hand-back contains a *coalesced* group.

    fold.rs:
         1	//! The balanced binary-counter reduction every n-ary fold runs on: the one home
         2	//! for the counter discipline, so a hardening of the fold shape reaches every
         3	//! fold at once.

Resolution: Add `fold/tests.rs` with one witness of the retention arm over integers (a combiner that refuses a marked pair), plus the dropped-group known-bad variant from party/tests.rs:182-227 rewritten over the same combiner and held convicted there; then reduce this test to the one clause the laws module owns (the hand-back contains a coalesced group, the justification for stating conservation over unions), or move that clause beside the fold witness and delete this test; widen or honor the module doc at laws/tests.rs:1-3. This is the natural home for the pin oracle-laws-2's second branch asks for. Acceptance: `fold.rs` has `mod tests;` with the retention-arm witness and its known-bad conviction; laws/tests.rs contains only collection-level pins or its doc says otherwise.

## Positives

- The operating envelope (oracle.rs:14-32) states once where every input bound lives and why hardening the reference would be a fidelity loss, and the harness caps I checked honor it: `optrace::MAX_TRACE_OPS = 30`, the generators' `ARB_DEPTH`, the iterated-tick law's `0..=3` (laws.rs:2507-2519). Depth stress runs against the impl alone, as promised.
- `laws!` (laws.rs:186-246) makes registration structural, and every consumer expands from `for_each_law_group!` as the docs at laws.rs:84-92 claim: I verified `algebraic_laws/tests.rs:304` and `447`, `fuzz_laws.rs:225`, and `surface_coverage/tests.rs:110`. A novel signature refuses to compile in every consumer.
- Fallible operations are quantified over outcomes, arm and payload: `join_commutative_outcomes` (2203-2213) requires both orders to agree and to leave `self` unchanged on `Err`; `join_associative_outcomes` (2273-2290) and `span_intersect_associative_outcomes` (1570-1583) do the same over `Option`. The worst passing artifact of an `Ok`-only law is closed.
- Conditional laws construct their antecedents where the policy says to: `order_transitive_constructed` (896-900), the `lag_*` constructed-plus-incidental pairs (933-950), `projection_monotone_in_version` (2623-2628), and `projection_additive_over_carved_regions` (2787-2809), whose fork-half fallback through the same `without` entries guarantees the equation runs on every call.
- The conservation laws state at the law why they quantify over unions (2402-2416, 3352-3360), and the witness in laws/tests.rs:78-135 is genuine: tracing `[a, alias(a), b, alias(b), c]` through the counter yields hand-backs `alias(a)` and `alias(b) ∪ c`, the latter byte-distinct from every input, exactly as asserted.
- `arb_fold_arity` (generators.rs:370-406) derives its band from the counter's structural boundaries (first in-counter combine, first drain, first merged-merged carry, two octave crossings), not from observed values.
- The grow-optimality suite (oracle/tests.rs:537-643) is independent of the DP it checks: `grow_brute_force` enumerates the whole inflation space with exact `+ 1` arithmetic and no shared `deepen`, and its header states precisely what the impl-equals-oracle differential alone cannot catch; the §3 event-condition reading is stated with the counterexample that rules out the literal reading (610-623).
- `Arc` children on both oracle trees (party.rs:7-10, version.rs:32-35) are explained by the cost model they buy, and `Version::meet_all` (version.rs:366-379) is the paper-faithful `reduce` with the missing-top rationale stated.
- The paper's worked examples are transcribed with section citations (`event_normalization` §5.2, `split_equations` §5.3.2, `sum_and_join` §5.3.3, `event_fills_to_single_integer` §5.3.4, `worked_example` §5.1), so the oracle can be audited against the source directly.
- `#![allow(clippy::type_complexity)]` at laws.rs:60-63 carries its rationale inline, the doctrine's preferred shape; the `RANK_TRIPLE` `(a, b, _c)` renames make ignored inputs explicit and arity-checked.
- `join_all_differential_convicts_the_dropped_group_oracle` (party/tests.rs:266-290) proves the differential criterion can fail, at exactly the widths that reach the retention arm.

## Open questions for Finch

1. Is `join_all`'s hand-back grouping and drain order a pinned behavior? Today both `# Errors` sections say the absorbed set is unspecified, and the only pin is the oracle copy (oracle-laws-2). Recommendation: no; make the oracle sequential, compare verdict, accumulator, and region union, and let the laws carry the contract. If yes, say so in the contracts and pin it once at `fold.rs` (oracle-laws-26).
2. Should `Leg::Law` citations resolve against `laws::registered_names()` alone? Three roster rows (`surface.rs:761, 1127, 1133`) cite `#[test]` items with `Leg::Law`; two of those compare a public operation against an internal kernel entry. The leg's documented meaning ("on production alone, by test name") is satisfied, so the premise of the lens finding misreads the docs, and the site is outside this partition. Recommendation: route to the surface-roster partition as a question of whether the `Law` kind should mean "member of `crate::laws`".
3. Should the oracle `Clock` track production's method names (`recv`, and drop the deleted observer trio) so `optrace` stops translating, or be documented as a semantic mirror with named divergences (oracle-laws-1)? Recommendation: track the names; the translation layer is small and the false claim is the cost.
4. "door" has 237 uses across `crates/before/src` with no definition (oracle-laws-19). Recommendation: replace with the plain term crate-wide in one sweep, or define once in the crate docs; the partition-local edit alone would leave the word inconsistent.
5. For bundled laws (oracle-laws-16): change `Law<F>` to return the failing clause's name (a feature-gated public type change, roster stable) or split the bundles (roster citations move)? Recommendation: the `Result<(), &'static str>` form; it keeps the citation haystack stable.
6. Does cargo-mutants under `--all-features` mutate `src/laws.rs` and `src/oracle/`, which are gated `cfg(any(test, feature = ...))` rather than `cfg(test)`? If so, a law body mutated to `true` is a missed mutant by construction and the manual campaign carries that noise; if the campaign scope excludes instrument modules, where is that stated? Not verified; no tool was run. Recommendation: state the scope in `.cargo/mutants.toml`'s header.
7. `oracle::Version::grow`'s `(Party::Leaf(true), Version::Node)` arm (version.rs:272-286) extends the paper, whose `grow(1, e)` is defined only for a leaf `e`; the arm is unreachable through `event` (fill collapses the region first) and exists for the direct grow tests. Recommendation: one sentence at the arm naming it as the `1 ≡ (1, 1)` extension.

## Dropped

- [2] A `Leg::Law` citation may name any `#[test]`: out of scope (anchors in `surface.rs` and `surface_coverage/tests.rs`, the surface-roster partition), and the premise misreads the leg docs, which define `Law` as "on production alone, by test name" (history: 67956d1fc predates `crate::laws`). Carried as open question 2.
- [5, part] `Default for oracle::Version` has no caller: refuted; clippy's warn-by-default `new_without_default` under the gate's `cargo clippy ... -D warnings` requires it for a type with `pub fn new()`, and no `allow` exists anywhere in the crate.
- [11] The roster carries one consumer's driver names: deliberate and documented at laws.rs:88-89 ("into the per-group proptest drivers (by the driver names carried here)"); the one-sentence ask is already met.
- [25, shim half] Three `_for_test` visibility shims: refuted; `#[cfg(test)] pub(crate)` is the only way to scope crate visibility to test builds, so removing the wrappers is a taste trade, not a free simplification. The cost-pair half survives as oracle-laws-5.
- [12], [24], [36]: duplicates of oracle-laws-2 (merged; 36's `crate::fold` routing rejected per b9f6af2d7's recorded reason; 24's oracle-local helper is a sub-step of the design question, not an alternative).
- [15], [23]: duplicates of oracle-laws-1.
- [18], [32], [42]: duplicates of oracle-laws-3.
- [21], [37]: duplicates of oracle-laws-13.
- [31]: duplicate of oracle-laws-15.
- [40]: duplicate of oracle-laws-17.
- [43]: duplicate of oracle-laws-12.
- [27], [28]: duplicates of oracle-laws-22.
- [39]: merged into oracle-laws-25 (the compile-time tie dissolves it; the scan patch is the fallback).
- [9]: merged into oracle-laws-5.
- Refutation new item 2 (oracle.rs's single-omission claim): merged into oracle-laws-1.
- Refutation new item 3 (fc06c5e8's rationale names no constructible input): merged into oracle-laws-4.
