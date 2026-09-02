# Partition testing-oracles: The test-only harness core: bridge, semantic oracle, exhaustive enumeration, algebraic-law binding, validation index

## Partition summary

This partition is the test-only core of before's differential architecture. `testing.rs` is the front door that declares the scaffolding and suite modules. `bridge.rs` converts between the recursive paper oracle's trees and the impl's values by emitting and reading the impl's stored bits directly (never the public codec), so structural agreement is decoupled from codec correctness. `semantic_oracle.rs` is the third reference: the paper's section 4 function-space construction realized as closures over dyadic points, with random section-4-valid `fork` and `event` policies, a derived grid ceiling `GRID_N`, and resolution probes; its `tests.rs` replays one single-seed op trace against all three references (`replay_matches_across_references`), holds two known-bad references convicted, and pins the grid derivation's premises. `exhaustive.rs` enumerates every canonical id tree and event tree under small depth bounds and its `tests.rs` runs every public operation over every tree and ordered pair against the oracle, plus five hand-spelled symmetry tests and a closed-form corpus totality pin. `algebraic_laws.rs` is a thin binding whose `tests.rs` expands both driver genres from `crate::for_each_law_group!`. `validation_index.rs` is a documentation-only map of the crate's instruments. All nine files compile only under `cfg(test)` (`lib.rs:453-454` gates `mod testing`), so nothing here ships; the three `tests.rs` files are test modules and the other six are test scaffolding. I read all 3168 lines of the partition with line numbers, plus the supporting files each finding cites (`recurse.rs`, `oracle.rs`, `oracle/version.rs`, `oracle/party.rs`, `party.rs`, `version.rs`, `laws.rs`, `optrace.rs`, `generators.rs`, `grow_brute_force.rs`, `grow/tests.rs`, `party/tests.rs`, `surface.rs`, `surface_coverage.rs`, `encode.rs`, the justfile, `.config/nextest.toml`, `tools/citecheck`, before's `AGENTS.md`, and the 2026-07-22 decision record). I ran no cargo, just, or test command; every "verified" below means read, grepped, or hand-derived, never executed.

The adequacy discipline is the partition's strength and is done unusually well. The two known-bad references (the cell-dropping Riemann sum, the mirrored embedding) are committed behind inverted assertions, the mirrored-embedding test also demonstrates the pointwise differential's blindness that makes the absolute-geometry anchor necessary, and both are rostered by name in `surface_coverage::TRIPWIRES` and resolved against nextest's live inventory from outside the crate. `corpus_counts_are_exact` pins the id corpus with an in-test closed form (`2^(2^d)`) and the event corpus with denotation distinctness through an independent path-sum walk. `GRID_N` is derived from `optrace::MAX_TRACE_OPS`, `fork` asserts the ceiling instead of clamping, and `fork_chain_raises_resolution_one_level_per_fork` meets the derivation's rate premise with equality on the worst in-support schedule. The law drivers expand from the single roster so a novel signature refuses to compile. I found no harness bug that masks a production failure.

The dominant issues are residue of three arcs the code has already moved past, each leaving prose or machinery that describes the earlier state. The semantic-oracle grid machinery went from a hand-picked clamping `GRID_N` to a derived, asserted one (08f7ccab, 7487be16, 28f6981e, f55c8276), and the test-side clamps, the "two levels per fork" rationale, the oracle-depth legs of the sampled sweep, and `Dyadic`'s dead `Ord` impl with its `GRID_N` premise all remain. The bridge gained `descend!` on its version walks only (faf3cd0a), while `recurse.rs` and before's `AGENTS.md` describe the whole bridge as guarded and the literal depth `0` defeats the guard's documented amortization. The exhaustive suite gained an exact-count totality pin (ebec26fe) without retiring the `> 20` floors it supersedes, and its grow-minimality pin still has no liveness floor. The validation index, which `AGENTS.md` names as the orientation map, opens with a totality claim that the tree does not meet. Two owner rulings (#53 on `exhaustive_deep`, #75 on the exhaustive point laws) settle questions the lenses raised, but neither rationale is stated at the code site.

## Findings

### testing-oracles-1: Hand-maintained enumerations of suites and corpus importers have drifted
- Where: crates/before/src/testing.rs:10-20 (related: crates/before/src/testing.rs:46, crates/before/src/testing/exhaustive.rs:32-35, crates/before/src/span/tests.rs, crates/before/src/version/skyline/tests.rs)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (compared the doc's suite list against the `mod` declarations at testing.rs:41-50; `grep -rl 'all_normal_ids\|all_normal_events' crates/before/src` lists span/tests.rs and version/skyline/tests.rs beyond the six suites exhaustive.rs names; `git show --stat b5a81583` adds `mod fuelscape_islands;` without touching the doc); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the inventories were written at ea0b1a8c and drifted at b5a81583 and ed1b3c8b; version/skyline/tests.rs was already an importer when the list was written)
- Owner-gated: no

The module doc enumerates the cross-cutting suites and omits `fuelscape_islands`, which line 46 declares; exhaustive.rs enumerates the kernel suites that import the corpus and omits two importers. Principle 5 forbids hand-maintained enumerations of module contents or callers because they rot silently, and both have.

Evidence:

        10	//! The cross-cutting suites: exhaustive small-scope enumeration
        11	//! ([`exhaustive`]), the function-space semantic oracle ([`semantic_oracle`]),
        12	//! the algebraic-law harness ([`algebraic_laws`] — a thin binding; the named
        46	mod fuelscape_islands;

    exhaustive.rs:
        32	//! necessarily outside the small scope: the kernel test suites import the
        33	//! corpus and run their own operation-specific sweeps over it (the skyline
        34	//! query, sweep, emit, fill, grow, and text suites all do — e.g. the pair
        35	//! queries' `exhaustive_small_scope_pairs_agree`). Two variants:

Resolution: In testing.rs, either add `fuelscape_islands` with its one-clause purpose or replace the list with the structural statement (scaffolding modules are `pub(crate)`, suite modules are private, each module's own doc states what it catches) and let the `mod` list be the roster. In exhaustive.rs, drop the importer list and keep the sentence that kernel suites import the corpus, with the one example. Acceptance: every `mod` declared in testing.rs is named in its doc or the doc no longer enumerates; exhaustive.rs names no subset of the importers.

### testing-oracles-2: The validation index is never rendered, so its intra-doc links are unchecked and its `pub` is dead visibility
- Where: crates/before/src/testing.rs:50-50 (related: crates/before/src/lib.rs:453-454, justfile:256-270, crates/before/src/testing/validation_index.rs:1-13)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (lib.rs:453-454 reads `#[cfg(test)] mod testing;`; justfile:258 and :270 run `cargo doc ... --all-features --no-deps` and `--document-private-items` with no `--cfg test`; `grep -n 'cfg test' justfile` is empty); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (from birth at 669cf310)
- Owner-gated: yes (the page's charter: rendered map or source-read page)

`validation_index` is `pub mod` inside a module lib.rs gates with `#[cfg(test)]`, and no doc recipe passes `--cfg test`, so rustdoc never compiles the module and never resolves its `[`super::exhaustive`]`-style links. The page's link syntax implies a check that does not run, and the `pub` reaches no further than `pub(crate)`. Principle 3: link syntax whose only reader is a human opening the source file costs maintenance and buys no checking.

Evidence:

        50	pub mod validation_index;

    lib.rs:
       453	#[cfg(test)]
       454	mod testing;

Resolution: Owner's call between (a) rendering it: move the index out of `cfg(test)` (it holds no code) behind `#[cfg(doc)]` or a feature so `just docs-internal` checks its links, with the links pointed at renderable targets; or (b) keeping it source-read: change `pub mod` to `mod`, replace intra-doc link syntax with plain code spans, and say in the opening paragraph that it is read in source. Acceptance: either `just docs-internal` fails on a dead link in the index, or the file contains no intra-doc link syntax and its visibility matches its siblings.

### testing-oracles-3: Bridge id walks recurse bare while the version walks route through `descend!`; the literal depth `0` probes on every level
- Where: crates/before/src/testing/bridge.rs:29-45 (related: crates/before/src/testing/bridge.rs:56-57, crates/before/src/testing/bridge.rs:109-132, crates/before/src/testing/bridge.rs:150-151, crates/before/src/recurse.rs:3-4, crates/before/src/recurse.rs:9-14, crates/before/src/recurse.rs:22-28, crates/before/src/recurse.rs:88-93, crates/before/src/recurse.rs:111-117, crates/before/AGENTS.md:32-36, crates/before/src/party/tests.rs:830-848)
- Class / severity / confidence: idiom / medium / high
- Provenance: verified (`grep -rn 'descend!(' crates/before/src` shows the bridge's only sites are 56, 57, 150, 151, all with literal `0`, while grow/tests.rs:172-180 and meter/tests.rs:417 thread `depth + 1`; recurse.rs:91-93 is `depth.is_multiple_of(STRIDE)`; `git show faf3cd0a -- bridge.rs` adds `descend!` around `emit_ev`/`read_ev` only; `git log -S'descend!(0, read_id'` and `-S'descend!(0, emit_id'` are empty; party/tests.rs:834 `ORACLE_SCALE_MAX = 4096` with `to_oracle_party(&Party::from_bits(...))` at 847-848 drives the unguarded `read_id` to 4096 levels); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: contradicts-hard-rule (the bridge was never guarded before faf3cd0a, which guarded only the two walks it was rewriting; `descend!(0, …)` was the entry-call idiom of 1c4c9a2d applied to recursive calls)
- Owner-gated: no

`emit_id` and `read_id` recurse on tree depth with plain calls; `emit_ev` and `read_ev` route through `descend!(0, …)`. Before's `AGENTS.md` hard-rule paragraph and `recurse.rs` both name "the oracle bridge" as the place recursion routes through the guard, so the inventory is false for half the bridge, and the guarded half is the shallower side in practice (the id walks see 4096-level spines in party/tests.rs; the version walks see at most 128 in the callers traced). Separately, `should_grow(0)` is `0.is_multiple_of(STRIDE)`, which is true, so the guarded walks probe stack headroom at every level, forfeiting the once-per-`STRIDE` amortization recurse.rs:22-28 documents and contradicting the macro's prescribed usage `descend!(depth + 1, …)`. Correctness is not exposed today (the oracle's own recursive `Drop` binds first per `oracle.rs:16-21`, and 4096 frames fit the test stack), so this is the letter of the hard rule breached on a test surface plus two inaccurate inventory statements, not a reachable overflow; the fix is trivial either way.

Evidence:

        36	        oracle::Party::Node(l, r) => {
        37	            // 2-bit presence tag, then the present children (a `0` child emits
        38	            // nothing).
        39	            out.push(!id_is_zero(l)); // bit 0 = left present
        40	            out.push(!id_is_zero(r)); // bit 1 = right present
        41	            emit_id(out, l);
        42	            emit_id(out, r);
        43	        }
        56	            descend!(0, emit_ev(out, l));
        57	            descend!(0, emit_ev(out, r));

    recurse.rs:
        11	//! surface, where the remaining depth recursion lives: the differential oracle
        12	//! bridge (`testing::bridge`), whose walks mirror the paper's recursive trees,
        91	pub(crate) fn should_grow(depth: usize) -> bool {
        92	    depth.is_multiple_of(STRIDE)
        93	}
       116	/// each recursive call site: `descend!(depth + 1, self.rec(child_args, depth +
       117	/// 1))`.

    before/AGENTS.md:
        32	  `version/skyline/fill.rs`). A walk that must recurse routes each recursive
        33	  call through `crate::recurse::descend!`, which grows the stack onto the
        34	  heap before a deep input can overflow — today those are only test
        35	  surfaces: the oracle bridge and the test-local recursive witnesses beside
        36	  it (`recurse.rs`'s module doc holds the inventory and the keep decision).

Resolution: Thread a `depth: usize` through all four walks and route every recursive call through `descend!(depth + 1, …)` as grow/tests.rs and meter/tests.rs do; if the id side is instead meant to be exempt, state its bound at `emit_id`/`read_id` (the oracle envelope plus `ORACLE_SCALE_MAX`) and correct recurse.rs:11-14 and AGENTS.md:32-36 to say which bridge walks are guarded. Acceptance: `grep -n 'descend!(0' crates/before/src/testing/bridge.rs` is empty and every recursive call in bridge.rs is inside `descend!(depth + 1, …)`; or the exemption is stated at the site and recurse.rs and AGENTS.md describe the bridge accurately.

### testing-oracles-4: The oracle-to-impl doors state normal form as a fact but check nothing; a non-normal or empty oracle tree lowers to a wrong `Party`
- Where: crates/before/src/testing/bridge.rs:70-76 (related: crates/before/src/testing/bridge.rs:10, crates/before/src/testing/bridge.rs:23-24, crates/before/src/testing/bridge.rs:36-43, crates/before/src/testing/bridge.rs:83-89, crates/before/src/party.rs:712-722, crates/before/src/version.rs:1192-1201, crates/before/src/testing/grow_brute_force.rs:43-47, crates/before/src/testing/exhaustive/tests.rs:12-14)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (party.rs:716 and version.rs:1196 state caller guarantees and `from_bits` only freezes; `emit_id` on `Node(Leaf(false), Leaf(false))` pushes `false, false` at 39-40, the `Leaf(true)` terminal tag at 33-34, so the empty region lowers to the full one; `from_oracle_party(&Leaf(false))` emits no bits and freezes an anonymous `Party`; `all_inflations` returns raw trees by contract at grow_brute_force.rs:45-46; `oracle::Party` is a `pub enum` with `Node` public); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found ("only ever emits normalized" entered at 32a65543 as a statement about callers; the oracle envelope assigns input bounding to harnesses and says nothing about a normal-form check at the door)
- Owner-gated: no

`Party::from_bits` and `Version::from_bits` delegate the normal-form and nonempty obligation to their callers; the bridge passes it on silently to its own callers while its module doc asserts the outcome as a fact ("Both forms are normalized"). A non-normal oracle tree is constructible from committed test API: `all_inflations` returns raw trees, and `oracle::Party::Node` is public. Lowering `Node(Leaf(false), Leaf(false))` yields a `Party` byte-equal to the seed's full id; lowering `Leaf(false)` yields the anonymous `Party` that exhaustive/tests.rs:12-14 says never exists standalone; lowering a raw version yields a skyline with equal adjacent plateaus whose byte-`Eq` disagrees with every canonical value, a red that would be blamed on production. All present callers comply (the oracle's constructors normalize; the exhaustive suite normalizes candidates at tests.rs:326), so nothing is masked today; the doctrine's bar for a guard is met because the failure is constructible from test API, no committed test exercises it, and the check is O(n) on bounded test trees.

Evidence:

        10	//! Both forms are normalized, so structural `==` ⇔ semantic equality.
        23	/// Whether an oracle id subtree is the empty `0` region. In normal form that is
        24	/// exactly the `Leaf(false)`; the bridge only ever emits normalized oracle trees.
        72	pub(crate) fn from_oracle_party(t: &oracle::Party) -> Party {
        73	    let mut bits = BitsBuf::new();
        74	    emit_id(&mut bits, t);
        75	    Party::from_bits(bits)
        76	}

    party.rs:
       716	    /// Callers guarantee normal *tree* form (a nonempty, normalized id);

Resolution: Add `debug_assert!(t.is_normal(), …)` at `from_oracle_version` and `from_oracle_party`, plus `debug_assert!(!t.is_empty(), …)` on the party door (both oracle types expose `is_normal`; `oracle::Party::is_empty` exists at oracle/party.rs:30), and reword lines 10 and 23-24 to state the precondition as the caller's. Acceptance: a test lowering `oracle::Party::Node(Arc::new(Leaf(false)), Arc::new(Leaf(false)))` panics at the door instead of yielding a `Party` equal to the seed's.
Construction: In any unit test with `pub(crate)` access: `from_oracle_party(&oracle::Party::Node(Arc::new(oracle::Party::Leaf(false)), Arc::new(oracle::Party::Leaf(false))))` returns bits `00`, equal to `from_oracle_party(&oracle::Party::Leaf(true))`. For versions: take the left-descent candidate of `all_inflations(&oracle::Party::Leaf(true), &oracle::Version::node(0u64, oracle::Version::leaf(1u64), oracle::Version::leaf(2u64)))` (a non-normal `Node(0, Leaf 2, Leaf 2)`), lower it with `from_oracle_version`, and observe it is not equal to `from_oracle_version(&that.normalized_for_test())`.

### testing-oracles-5: Idiom residue across the partition: qualified paths beside imports, a re-spelled helper, an index before its `expect`, a `debug_assert!` in test-only code, a `Result<(), ()>`
- Where: crates/before/src/testing/bridge.rs:83-89 (related: crates/before/src/testing/bridge.rs:64-68, crates/before/src/testing/semantic_oracle.rs:324, crates/before/src/testing/semantic_oracle.rs:386, crates/before/src/testing/semantic_oracle.rs:391, crates/before/src/testing/semantic_oracle.rs:98-102, crates/before/src/testing/semantic_oracle.rs:519, crates/before/src/testing/semantic_oracle.rs:644-649, crates/before/src/testing/semantic_oracle.rs:711, crates/before/src/testing/algebraic_laws/tests.rs:27, crates/before/src/testing/algebraic_laws/tests.rs:34, crates/before/src/testing/exhaustive/tests.rs:581-582)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read each site; bridge.rs:16 imports `crate::codec::{self, BitsBuf}` so `codec::built_view` is in scope at 87; `packed_bits_of` at 64-68 is exactly the two lines at 84-85; exhaustive/tests.rs:582 imports `Base` one line below a signature that spells `crate::codec::Base`); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness (refutation reframed the `debug_assert!` item: no documented run compiles it out, since the `--release` prescription belongs to `exhaustive_deep`, whose suite never calls `fs_grid`; Cargo.toml keeps debug assertions on for the dev profile the tests run under); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Small legibility items the doctrine names (imports over long qualified paths except where the qualification informs; a helper its own module bypasses; an `expect` whose one-line proof cannot print). Sites: bridge.rs:84-87 re-spells `packed_bits_of` and writes `crate::version::skyline::encode_bits(crate::codec::built_view(&bits))` with `codec` imported; semantic_oracle.rs:324 and 391 `std::collections::HashMap`, 644 and 649 `crate::Rank`; semantic_oracle.rs:386 evaluates `owned[0]` before `owned.last().expect("fork of an empty id")`, so an empty id panics with a bounds message and the proof never prints; semantic_oracle.rs:98 `debug_assert!` in `cfg(test)`-only code whose doc (94-95) promises the population "fails loudly here" (stylistic, since no documented run removes it); semantic_oracle.rs:711 `sync` returns `Result<(), ()>` where `bool` says the same; semantic_oracle.rs:519 a free fn `descend` shares its name with the `recurse::descend!` macro the neighboring bridge imports for a different purpose; algebraic_laws/tests.rs:27 and 34 `crate::testing::bridge::from_oracle_*`; exhaustive/tests.rs:581 `Vec<crate::codec::Base>` directly above `use crate::codec::Base;`.

Evidence:

        83	pub(crate) fn from_oracle_version(t: &oracle::Version) -> Version {
        84	    let mut bits = BitsBuf::new();
        85	    emit_ev(&mut bits, t);
        86	    Version::from_bits(crate::version::skyline::encode_bits(
        87	        crate::codec::built_view(&bits),
        88	    ))
        89	}

    semantic_oracle.rs:
       324	    let bump: std::collections::HashMap<usize, u64> = owned
       386	    let (lo, hi) = (owned[0], *owned.last().expect("fork of an empty id"));

    exhaustive/tests.rs:
       581	    fn ev_vector(t: &oracle::Version, depth: usize) -> Vec<crate::codec::Base> {
       582	        use crate::codec::Base;

Resolution: `from_oracle_version` becomes `Version::from_bits(skyline::encode_bits(codec::built_view(&packed_bits_of(t))))` with `use crate::version::skyline;`; import `HashMap` and `Rank` in semantic_oracle.rs; `let (&lo, &hi) = (owned.first().expect(..), owned.last().expect(..))`; `assert!` in `fs_grid`; `fn sync(..) -> bool`; rename the free `descend` (e.g. `halve`); import the bridge functions in algebraic_laws/tests.rs; use the imported `Base` in `ev_vector`'s signature. Acceptance: no `std::collections::`, `crate::testing::bridge::`, or `crate::codec::` path at a use site where the module is imported; `emit_ev` is called from exactly one function in bridge.rs; `just fmt` and `just clippy` clean.

### testing-oracles-6: The bridge's impl-to-oracle section comment restates the module doc, repeats the recursion caveat four times, and coins "master harness"
- Where: crates/before/src/testing/bridge.rs:97-107 (related: crates/before/src/testing/bridge.rs:1-12, crates/before/src/testing/bridge.rs:70-71, crates/before/src/testing/bridge.rs:80-81, crates/before/src/clock/tests.rs:192)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read 1-12 against 97-107; the clause "(test-only; the impl's own traversals are iterative)" or its variant appears at 11-12, 70-71, 80-81, 105-106; `grep -rn 'master harness\|master differential' crates/before/src` hits bridge.rs:104 and a section banner "master differential harness" at clock/tests.rs:192 and nothing else); executed: no
- Seen by: structure-prose; refutation: confirmed (with the correction that the clock/tests.rs recurrence is the banner phrase "master differential harness", not "master harness"); history: no-rationale-found (arrived with the 01b2ee61 relocation)
- Owner-gated: no

The `//` block heading the impl-to-oracle half repeats the module doc nearly sentence for sentence; the recursion caveat is stated four times in 193 lines; "the master harness" names nothing and is anchored to no identifier or definition, failing the vocabulary rule that a coined term is an identifier or is defined once by contrast. Comments state what the code cannot show, once.

Evidence:

        99	// Structural lowering for differential agreement: rebuild the oracle's tree
       100	// shape from the impl's *internal* stored bits (the packed id bits; the
       101	// version's skyline stream), then compare with `==`. This is the inverse of
       102	// `from_oracle_*`. It walks the stored bits directly — the impl's at-rest
       103	// storage — rather than round-tripping the public `encode`/`decode`, so the
       104	// master harness checks algorithm correctness without sharing a failure mode
       105	// with the byte codec (which is exercised separately). Recursive over a
       106	// bounded tree (test-only; the impl's own traversals are iterative). Both
       107	// forms are normalized, so structural `==` ⇔ semantic equality.

Resolution: Reduce the section comment to the banner plus one sentence saying `to_oracle_*` is the inverse of `from_oracle_*`; state the recursion caveat once in the module doc and let per-fn docs say only what differs; replace "the master harness" with the thing meant (the per-module differential suites). Acceptance: "master harness" appears nowhere in bridge.rs; the recursion caveat appears once.

### testing-oracles-7: Em-dashes in `//` comments (a crate-wide pattern; four sites in this partition)
- Where: crates/before/src/testing/bridge.rs:102-103 (related: crates/before/src/testing/semantic_oracle/tests.rs:187, crates/before/src/testing/exhaustive.rs:122, crates/before/src/testing/exhaustive/tests.rs:462)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep of the em-dash character on lines that are neither `///` nor `//!` across the nine files gives exactly these four; the same grep across `crates/before/src` gives 375 lines); executed: no
- Seen by: structure-prose; refutation: confirmed; history: contradicts-hard-rule (the writing-style rule postdates every partition prose commit, so this is residue the rule now targets; the practice is crate-wide)
- Owner-gated: no

The doctrine prefers colons or spaced double-hyphens over em-dashes in code comments (em-dashes belong to rendered prose). The four partition sites are a sample of 375 crate-wide, so the remedy is a crate-wide sweep, not four edits here; this finding exists so the sweep has a named trigger.

Evidence:

       102	// `from_oracle_*`. It walks the stored bits directly — the impl's at-rest
       103	// storage — rather than round-tripping the public `encode`/`decode`, so the

Resolution: A crate-wide sweep replacing em-dashes in `//` (non-doc) comments with ` -- ` or a colon. Acceptance: the grep above over `crates/before/src` returns only `///` and `//!` lines.

### testing-oracles-8: `to_oracle_party`/`to_oracle_version` discard the end position, so trailing live bits lower silently
- Where: crates/before/src/testing/bridge.rs:174-188 (related: crates/before/src/testing/bridge.rs:131, crates/before/src/testing/bridge.rs:171, crates/before/src/version/skyline/encode.rs:54-58, crates/before/src/party/tests.rs:846-849)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read_id returns `next` at 131 and read_ev returns `after_n` at 171; both doors take `.0` at 179 and 186; encode.rs:54-58 asserts totality on the inverse door; `from_bits` validates nothing); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The walks return the position after the root subtree and the doors drop it, so a stored stream with extra live bits after the root lowers to the prefix's tree. The inverse door (`encode_bits`) asserts that a canonical walk consumes every input bit. In the exhaustive suite every structural leg is paired with a byte-equality leg, so nothing is masked there; a suite comparing only through `to_oracle_*` (party/tests.rs:847-849) would not see extra bits. Totality over spot checks is the doctrine's defense form.

Evidence:

       175	pub(crate) fn to_oracle_party(p: &Party) -> oracle::Party {
       176	    if p.as_bits().is_empty() {
       177	        return oracle::Party::Leaf(false); // the anonymous `0` id
       178	    }
       179	    read_id(p.as_bits(), 0).0
       180	}
       185	pub(crate) fn to_oracle_version(v: &Version) -> oracle::Version {
       186	    let raw = read_ev(v.as_bits(), 0, &mut None).0;
       187	    raw.normalized_for_test()
       188	}

    encode.rs:
        54	    assert_eq!(
        55	        pos,
        56	        bits.len(),
        57	        "a canonical packed walk consumes every input bit"
        58	    );

Resolution: Bind the returned position in both doors and `assert_eq!(after, bits.len(), "the stored stream is exactly one subtree")`. Acceptance: a test that pushes one extra `true` onto `Version::new()`'s packed bits, freezes it with `Version::from_bits`, and calls `to_oracle_version` panics at the assert.
Construction: With `pub(crate)` access, take the packed skyline bits of `Version::new()` (a single leaf), push one extra `true`, `Version::from_bits(bits)`, then `to_oracle_version(&v)` returns `Leaf(0)` unchanged today.

### testing-oracles-9: `Dyadic`'s hand-written `PartialEq`/`Eq`/`PartialOrd`/`Ord` are dead code, and the `Ord` doc's overflow premise names the wrong bound
- Where: crates/before/src/testing/semantic_oracle.rs:132-152 (related: crates/before/src/testing/semantic_oracle.rs:86-104, crates/before/src/testing/semantic_oracle.rs:123-129, crates/before/src/testing/semantic_oracle.rs:519-541)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn Dyadic crates/before/src crates/before/tests` outside the impl block shows only construction via `Dyadic::grid`/`Dyadic::center`, struct literals in `descend`, field reads in `cell_at`/`descend`, and `Fn(Dyadic)` parameter types; no `==`, `cmp`, `min`/`max`, sort, or ordered container over `Dyadic` anywhere); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (dead at birth in 08f7ccab; the `GRID_N` premise was true while every grid was capped at 10, and f55c8276's uncapped `fs_grid` moved the bound to 64 without revisiting the doc)
- Owner-gated: no

Nothing compares two `Dyadic` values, so the four trait impls exist only to be correct about themselves (Principle 3), and trait impls are never dead-code-warned. The `Ord` doc argues `u128` cannot overflow because exponents stay near `GRID_N`; the module's `fs_grid` deliberately does not cap at `GRID_N` and instead asserts `g < 64`, which with `center` adding one level is the operative bound (`num < 2^64` shifted by at most 64 fits `u128`). The conclusion holds; the stated reason sends an auditor to the wrong constant.

Evidence:

       143	impl Ord for Dyadic {
       144	    /// Compare `a/2^p` and `b/2^q` by cross-multiplication: `a·2^q` vs `b·2^p`
       145	    /// (exponents stay within a level of [`GRID_N`], so `u128` never
       146	    /// overflows).
       147	    fn cmp(&self, other: &Self) -> Ordering {

    fs_grid:
        86	/// Deliberately *not* capped at [`GRID_N`]: that ceiling is derived from
        98	    debug_assert!(
        99	        g < 64,

Resolution: Delete the four impls, keeping `#[derive(Clone, Copy, Debug)]`. If an ordering is wanted later, state the bound as `fs_grid`'s `g < 64` (exponent at most 64 after `center`). Acceptance: `grep -n 'impl .* for Dyadic' crates/before/src/testing/semantic_oracle.rs` is empty and the test target compiles.

### testing-oracles-10: Test-side grid caps at `GRID_N` cannot bind, contradict `fs_grid`'s stated policy, and are guarded by constant-only asserts
- Where: crates/before/src/testing/semantic_oracle/tests.rs:40-47 (related: crates/before/src/testing/semantic_oracle/tests.rs:185-194, crates/before/src/testing/semantic_oracle/tests.rs:288, crates/before/src/testing/semantic_oracle/tests.rs:603-609, crates/before/src/testing/semantic_oracle/tests.rs:614, crates/before/src/testing/semantic_oracle.rs:62-64, crates/before/src/testing/semantic_oracle.rs:80, crates/before/src/testing/semantic_oracle.rs:86-104, crates/before/src/testing/semantic_oracle.rs:375-384)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (generators.rs:298 `ARB_DEPTH: u32 = 4` bounds `arb_oracle_*` via `prop_recursive` at 341 and 363, so every `grid_for` input is at most 5, or 7 at the `+ 3` site, against `GRID_N = 32` from optrace.rs:56 `MAX_TRACE_OPS = 30` and semantic_oracle.rs:80; `fork` asserts `res < GRID_N` before `res + 1`, so every reachable id ceiling is at most `GRID_N`, `event`'s ceiling is `e.res_ceiling.max(id_res(i))`, and the keystone's `d.min(GRID_N)` is the identity; `grid_for`'s six call sites are 230, 244, 265, 303, 422, 511); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness (the refutation added the `k < GRID_N` assert at 614 and the `GRID_N` doc sentence); refutation: confirmed; history: deliberate-but-expired (the caps matched the clamping `fork` of 08f7ccab/7487be16; 28f6981e replaced the clamp with an assert and f55c8276 stated the opposite policy for `fs_grid`, neither touching these sites; the 606-609 assert was written at 28f6981e as a guard against redefining `GRID_N` independently of the op cap)
- Owner-gated: no

`grid_for` is `fs_grid` plus `.min(GRID_N)`; the keystone caps again at line 194 under a comment that credits `grid_cap_is_never_reached`, and `fork_partitions` caps at 288. The parent module's `fs_grid` computes the same `max + 1` and refuses to cap for the stated reason that a binding cap "could only alias a scan silently". None of the caps can bind on any reachable population, so their only possible effect is the silent-alias failure the module rejects, and two helpers with opposite policies for one computation is a legibility seam. The `GRID_N` doc at 62-64 ("this caps `g`") describes the clamp, not `fs_grid`'s assert. Beside them, `fork_chain_raises_resolution_one_level_per_fork` opens with `MAX_TRACE_OPS <= GRID_N` where `GRID_N` is defined as `MAX_TRACE_OPS as u32 + 2`, and asserts `k < GRID_N` with `k` at most 16: both are runtime checks of compile-time facts. The keystone's clamp matters for testing-oracles-17: a silent clamp is exactly what makes the sampled sweep load-bearing, and turning it into an assert is what would let the sweep retire.

Evidence:

        45	fn grid_for(parts: &[u32]) -> u32 {
        46	    (parts.iter().copied().max().unwrap_or(0) + 1).min(GRID_N)
        47	}
       190	        let g = se
       191	            .iter()
       192	            .map(|c| id_res(&c.id).max(ev_res(&c.ev)))
       193	            .max()
       194	            .map_or(0, |d| d.min(GRID_N));
       606	    assert!(
       607	        u32::try_from(MAX_TRACE_OPS).expect("small cap") <= GRID_N,
       608	        "the grid ceiling no longer clears the deepest in-support bisection"
       609	    );
       614	        assert!(k < GRID_N, "the chain itself must stay inside the grid");

    semantic_oracle.rs:
        80	pub(crate) const GRID_N: u32 = optrace::MAX_TRACE_OPS as u32 + 2;
        86	/// Deliberately *not* capped at [`GRID_N`]: that ceiling is derived from
        90	/// organic drivers), so a binding cap here could only alias a scan
        91	/// silently. What is asserted instead is the one hard bound the

Resolution: Delete `grid_for` and call `fs_grid` at its six sites; replace the keystone's `.map_or(0, |d| d.min(GRID_N))` with `fs_grid` or an `assert!(d <= GRID_N, …)` and reword the comment at 185-189 to say `fork`'s assertion is what keeps the probed grid inside the ceiling; use `fs_grid(&[id_depth(&p) + 1])` at 288 (see testing-oracles-16 for the rate); turn the 606-609 assert into a `const _: () = assert!(...)` (its purpose, guarding a future redefinition of `GRID_N`, survives) and delete 614; restate the `GRID_N` doc at 62-64 in terms of the assert. Acceptance: `grep -n 'min(GRID_N)\|fn grid_for' crates/before/src/testing/semantic_oracle/tests.rs` is empty; the suite is green.

### testing-oracles-11: `replay` carries a `seeds` parameter every caller fixes at 1 and re-spells the optrace steppers; `FunctionClock`'s `Err` arms are unreachable
- Where: crates/before/src/testing/semantic_oracle/tests.rs:55-63 (related: crates/before/src/testing/semantic_oracle/tests.rs:68-152, crates/before/src/testing/semantic_oracle/tests.rs:183, crates/before/src/testing/semantic_oracle/tests.rs:565, crates/before/src/testing/optrace.rs:72-120, crates/before/src/testing/optrace.rs:122-165, crates/before/src/testing/semantic_oracle.rs:697-724)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both callers pass `1` at 183 and 565; the `Join` arm's `i2 = if j < i { i - 1 } else { i }` and the `Sync` arm's `split_at_mut` appear in replay at 108-123 and 139-146 and in optrace.rs at 98-101, 109-110, 145-147, 156-157; with one seed all live ids are disjoint, so the `if d_im` at 117 and 139 always takes the true arm; grep of diff_ops.rs shows `FunctionClock` used only by construction and field reads, so `join`/`sync`'s `Err` arms at 704 and 722 are reached by no test); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: `seeds` deliberate-but-expired (load-bearing at 08f7ccab when the sweep ran `1usize..=4` seeds; single-seed since 7487be16); the lockstep inlining deliberate-and-holds (its purpose, asserting three-way disjointness agreement before every `Join`/`Sync`, is stated at 51-58 and is something neither optrace stepper can do)
- Owner-gated: no

A parameter with one value at every site is generality with no consumer, and the doc at 55 already says so. The impl and oracle arms are a third spelling of the trace semantics that `optrace::run` and `step_impl` each spell once "so traces line up"; a divergence in index arithmetic would misalign populations and read as a false differential failure. The lockstep design itself has a stated reason (the pre-op agreement assert) and stays; what can dissolve is the duplicated dispatch. `FunctionClock::join`/`sync` return `Result` to mirror the crate's API, but with one seed and a pre-check, their failure arms are dead in every harness.

Evidence:

        55	/// Every caller passes `seeds = 1`: the invariance the keystone asserts holds
        56	/// only for a proper single-seed system (see
        57	/// [`replay_matches_across_references`]). A `Join`/`Sync` on overlapping
        58	/// parties is a no-op in all three (disjointness is invariant).
        59	fn replay(
        60	    seeds: usize,
        61	    ops: &[Op],
        62	    rng: &mut StdRng,
        63	) -> (Vec<Clock>, Vec<oracle::Clock>, Vec<FunctionClock>) {
       139	                        if d_im {
       140	                            let i2 = if j < i { i - 1 } else { i };
       141	                            let v = im.remove(j);
       142	                            assert!(im[i2].join(v).is_ok());

    optrace.rs:
       109	                        let victim = cs.remove(j);
       110	                        let i2 = if j < i { i - 1 } else { i };
       122	/// Apply one op to an impl population, mirroring [`run`] for the oracle (same index
       123	/// arithmetic, so traces line up). Used by tests that drive the impl alone.

Resolution: Drop `seeds` and start each population from one seed. Extract the oracle arm of `optrace::run` into a `step_oracle(&mut Vec<oracle::Clock>, &Op)` so `run` folds over it, write a `step_fs(&mut Vec<FunctionClock>, &Op, &mut StdRng)` beside it, and reduce `replay` to the pre-op disjointness-agreement assert followed by three step calls. Make `FunctionClock::join`/`sync` infallible operations that assert disjointness, since no caller wants the `Err`. Acceptance: `replay` contains no `match *op` arm that mutates `im` or `or` directly; the `Join`/`Sync` index arithmetic exists in optrace only (plus the fs stepper); `replay(1, …)` call sites become `replay(…)`; `replay_matches_across_references` and the sweep pass unchanged.

### testing-oracles-12: "keystone" is used six times before any definition and as a first sentence that says nothing; "honest" and "genuine" moralize the correct reference
- Where: crates/before/src/testing/semantic_oracle/tests.rs:157-157 (related: crates/before/src/testing/semantic_oracle/tests.rs:55, crates/before/src/testing/semantic_oracle/tests.rs:530, crates/before/src/testing/semantic_oracle/tests.rs:544-549, crates/before/src/testing/semantic_oracle/tests.rs:356, crates/before/src/testing/semantic_oracle/tests.rs:410-427, crates/before/src/testing/semantic_oracle/tests.rs:457-514, crates/before/src/testing/algebraic_laws/tests.rs:188, crates/before/src/testing/algebraic_laws/tests.rs:315)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep over the nine files: "keystone" at 55, 157, 530, 544, 546, 549; "honest" at 457, 477, 479, 514; "genuine" at 356, 410, 414, 427 and algebraic_laws/tests.rs 188, 315); executed: no
- Seen by: structure-prose, adequacy; refutation: reframed ("canary" is established test jargon and exempt; "widened codec" at exhaustive/tests.rs:233 is anchored crate-wide at codec/int.rs and version/tests.rs, though not locally); history: contradicts-hard-rule (the writing-style rule postdates the prose, so this is residue the rule now targets)
- Owner-gated: no

The vocabulary rule anchors every coined term to an identifier or defines it once by contrast; "keystone" names `replay_matches_across_references` throughout the file but is introduced nowhere, and the test's own first sentence, which stands alone in a module listing, is "The keystone check." "Honest" and "genuine" distinguish the correct reference from the known-bad one where "the section-4 embedding" and "the known-bad variant" would, and "genuine functions" (356) and "genuine distance" (188, 315) carry no mechanism.

Evidence:

       157	    /// The keystone check.
        55	/// Every caller passes `seeds = 1`: the invariance the keystone asserts holds
       457	                    // The defect: the honest embedding descends right
       410	/// while the genuine sum agrees with the impl everywhere.

Resolution: Open the keystone doc with the invariant ("After one op trace, every ordered pair of final clocks has the same comparison descriptor under all three references.") and refer to the test by name elsewhere; write "the section-4 embedding" or "the reference" for honest/genuine, "actual functions" at 356, and "a distance" at 188 and 315. Acceptance: no "keystone", "honest", or "genuine" remains in the partition; every `#[test]` doc's first sentence states what is checked.

### testing-oracles-13: An empty "operation cross-checks" section banner survived the move of its tests to `diff_ops`
- Where: crates/before/src/testing/semantic_oracle/tests.rs:218-220 (related: none)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (two banners back to back at 218-220; `git show 223795c1 -- crates/before/src/testing/semantic_oracle/tests.rs` has the hunk `@@ -218,110 +217,6 @@` removing the six proptests beneath the first banner and keeping both banners); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (an oversight in 223795c1)
- Owner-gated: no

A heading for code that no longer exists in the file; a reader scanning section banners is told there is a cross-check section and finds none (Principle 5).

Evidence:

       218	// ───────────────────────────── operation cross-checks ─────────────────────────────
       219	
       220	// ───────────────────────────── law suite (the function-space model is a sound ITC) ─────────────────────────────

Resolution: Delete lines 218-219. Acceptance: every section banner in the file precedes at least one item.

### testing-oracles-14: `order_is_a_partial_order`'s transitivity arm is conditional on a premise nothing constructs
- Where: crates/before/src/testing/semantic_oracle/tests.rs:233-237 (related: crates/before/src/testing/semantic_oracle/tests.rs:251-255)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: assessed (read; the frequency of the premise over three independent `arb_oracle_version` draws is unmeasured, and measuring it would require running the test); executed: no
- Seen by: adequacy; refutation: reframed (the "mostly vacuous" claim is unquantified: `arb_base` puts real weight on small bases, so pointwise comparability is not rare; the construction is sound because `join_is_the_lub` at 251-255 already proves `a <= join(a, x)`); history: no-rationale-found (unchanged since 08f7ccab)
- Owner-gated: no

Transitivity is asserted only when `le(a, b) && le(b, c)` happens to hold for three independent draws; the arm executes on an unmeasured fraction of cases. The cheapest passing artifact under a conditional law is a population that never satisfies the premise; constructing the chain makes the arm total at no cost.

Evidence:

       233	        // transitive: a ≤ b ≤ c ⇒ a ≤ c
       234	        let le = |x: &Event, y: &Event| matches!(ev_order(x, y, g), Some(Ordering::Less | Ordering::Equal));
       235	        if le(&fa, &fb) && le(&fb, &fc) {
       236	            prop_assert!(le(&fa, &fc));
       237	        }

Resolution: Draw `a`, `x`, `y`; set `b = join(a, x)` and `c = join(b, y)` (upper bounds by `join_is_the_lub`); assert `le(&fa, &fc)` unconditionally, keeping the reflexivity arm as is. Acceptance: the transitivity assertion has no `if` guard and the test stays green.

### testing-oracles-15: The paper's worked-value fixture and its expected vector are spelled twice
- Where: crates/before/src/testing/semantic_oracle/tests.rs:484-504 (related: crates/before/src/testing/semantic_oracle/tests.rs:334-352)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read both sites: identical tree literal and identical `[3u64, 3, 3, 3, 2, 4, 1, 1]` vector; the second site's comment says it copies the first); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no-rationale-found (ce03111a introduced the copy with the acknowledging comment)
- Owner-gated: no

The conviction test's argument is that the mirrored lift of the anchor's fixture disagrees with the paper's samples, so the two tests must hold the same tree; two transcriptions can drift independently.

Evidence:

       484	    // The worked tree the committed anchor pins
       485	    // ([`embedding_matches_paper_worked_value`]'s fixture).
       501	    let want: Vec<Base> = [3u64, 3, 3, 3, 2, 4, 1, 1]
       502	        .into_iter()
       503	        .map(Base::from)
       504	        .collect();

Resolution: Extract `fn paper_worked_example() -> (oracle::Version, Vec<Base>)` and use it in both tests. Acceptance: the tree literal and the vector appear once in the file.

### testing-oracles-16: Two test docs say the random `fork` refines up to two levels per call; the code, the module doc, and the equality pin say one
- Where: crates/before/src/testing/semantic_oracle/tests.rs:533-536 (related: crates/before/src/testing/semantic_oracle/tests.rs:287-288, crates/before/src/testing/semantic_oracle.rs:360-362, crates/before/src/testing/semantic_oracle.rs:371-384, crates/before/src/testing/semantic_oracle/tests.rs:612-624)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`fork` at 375-384 sets `level = res` when the region has two or more pieces, else `res + 1`, and both children are built at `level`, so resolution grows at most one level per call; the module doc at 360-362 states exactly that and `fork_chain_raises_resolution_one_level_per_fork` asserts `id_res(&kept.id) == k` after `k` forks; `git show 7487be16:crates/itc/src/testing/semantic_oracle.rs` line 212 reads `(res + 1).min(GRID_N)` and that commit's tests.rs already carries both "two levels" phrases); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (wrong at introduction; never revisited)
- Owner-gated: no

AGENTS.md holds every test doc's invariant statement to accuracy ("their incorrectness is a bug in the test"). The `grid_cap_is_never_reached` doc makes a two-level rate the reason the function space's resolution "is the binding constraint", and `fork_partitions` scans at `id_depth + 3` under the same premise, one level finer than needed. Both contradict the module doc beside them and the deterministic pin whose whole purpose is the one-level rate.

Evidence:

       533	/// This covers *both* the oracle's tree depth and the function space's probed
       534	/// resolution — the random `fork` refines up to two levels per call (vs. the
       535	/// paper's one), so its resolution can run ahead of the oracle's, and it is the
       536	/// binding constraint.
       287	        // Children refine ≤ 2 levels below the id's depth; scan deep enough to resolve them.
       288	        let g = (id_depth(&p) + 3).min(GRID_N);

    semantic_oracle.rs:
       360	/// Dealing out *existing* pieces adds no new boundary, so resolution grows only
       361	/// on the bisection of an indivisible piece — exactly the paper's rate (≤ 1
       362	/// level per fork). That is the one concession to a *finite* comparison grid:
       375	    let level = if owned_cells(i, res).len() >= 2 {
       376	        res
       377	    } else {
       383	        res + 1
       384	    };

Resolution: Restate both as one level (children carry a ceiling at most one level below the id's resolution); at 287-288 use `fs_grid(&[id_depth(&p) + 1])` with the one-level reason; if testing-oracles-17 rewrites the sweep's doc, the 533-536 paragraph is replaced there. Acceptance: no "two levels" or "≤ 2 levels" remains in the file; the three statements of the rate agree; `fork_partitions` and the chain pin stay green.

### testing-oracles-17: `grid_cap_is_never_reached`'s oracle-depth legs feed no scan; its function-space legs are the only committed bound on event-side resolution, which its doc does not say
- Where: crates/before/src/testing/semantic_oracle/tests.rs:551-583 (related: crates/before/src/testing/semantic_oracle/tests.rs:529-549, crates/before/src/testing/semantic_oracle/tests.rs:190-194, crates/before/src/testing/semantic_oracle.rs:75-78, crates/before/src/testing/semantic_oracle.rs:317-346, crates/before/src/testing/semantic_oracle.rs:365-367, crates/before/src/testing/semantic_oracle.rs:378-383, crates/before/src/surface.rs:1222-1223, crates/before/src/testing/surface_coverage.rs:133-136)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (the keystone's grid at 190-194 derives from `se` only, so the `ev_depth`/`id_depth` maxima at 566-569 are read by no scan; `fork`'s assert at 378-383 bounds the id side at bisection only; the chain pin exercises forks only; `event` at 336 sets the ceiling to `e.res_ceiling.max(level)` with `level = id_res(i)`, so event-side resolution is bounded by an argument, not a committed check, and the sweep's `ev_res` legs at 572 are the only assertion on it; the two roster citations are surface.rs:1222-1223 and surface_coverage.rs:135); executed: no
- Seen by: scaffolding, structure-prose; refutation: reframed (retiring the test outright is refuted: a change to `event` bumping at `id_res(i) + 1` would grow `ev_res` per tick while `fork`'s assert and the chain pin stay green; only the oracle-depth legs are unconsumed); history: deliberate-but-expired (load-bearing at 08f7ccab when `GRID_N` was a hand-picked 10 and `fork` clamped; retained at 28f6981e under a restated "distributional" rationale after the clamp became an assert)
- Owner-gated: yes (removal or reshaping of a rostered instrument; two roster citations)

The sweep maxes four quantities per case: the oracle trees' depths (566-569), which no scan consumes, and the function space's probed resolutions (570-573). The doc frames it as a distributional companion to the structural derivation and names a false two-level rate as its reason (testing-oracles-16); the true reason it is not redundant today is that the keystone's silent clamp (testing-oracles-10) plus the absence of any event-side assert leave the `ev_res` legs as the sole guard against an aliased scan. A guard should name the constructible failure it alone catches; this one's doc names a different one.

Evidence:

       566	            for c in &or {
       567	                max_d.fetch_max(ev_depth(&c.version()), AOrd::Relaxed);
       568	                max_d.fetch_max(id_depth(c.party()), AOrd::Relaxed);
       569	            }
       570	            for c in &se {
       571	                max_d.fetch_max(id_res(&c.id), AOrd::Relaxed);
       572	                max_d.fetch_max(ev_res(&c.ev), AOrd::Relaxed);
       573	            }
       578	    assert!(
       579	        observed < GRID_N,
       580	        "op-trace reached resolution {observed} ≥ GRID_N {GRID_N}; raise GRID_N so the scans \
       581	         stay fully faithful",
       582	    );

    semantic_oracle.rs:
       336	    let ceiling = e.res_ceiling.max(level);
       378	        assert!(
       379	            res < GRID_N,

Resolution: Drop the oracle-depth legs (566-569) and rewrite the doc at 529-549 to name what the sweep guards: event-side resolution under `event`'s per-cell bumps and `join`, which `fork`'s assert and the chain pin do not cover. Then the owner's call: keep the trimmed sweep, or convert the keystone's clamp into an assert (`fs_grid` or `assert!(d <= GRID_N)` at 194) so every keystone case asserts both sides, retire the sweep, and re-point the two roster citations (surface.rs:1222-1223 `GridCap { guard }` and `surface_coverage::TRIPWIRES`) to `fork_chain_raises_resolution_one_level_per_fork`, with `citecheck` green. Acceptance: the sweep reads only function-space quantities or is gone; its doc, `GRID_N`'s doc (75-78), and `fork`'s doc (365-367) name the event side or the keystone assert as the guard; both roster citations resolve.

### testing-oracles-18: The event corpus is the normalization closure over `{0, 1, 2}`, not "normal-form trees with bases in `{0, 1, 2}`"
- Where: crates/before/src/testing/exhaustive.rs:6-9 (related: crates/before/src/testing/exhaustive.rs:150-151, crates/before/src/oracle/version.rs:80-88)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (hand derivation from `oracle::Version::node` at 80-88: `node(2, leaf 2, leaf 2)` lifts `m = 2`, both children debase to `Leaf(0)` and collapse, giving `Leaf(2 + 2 + 0) = Leaf(4)`; so the depth-1 corpus holds leaves `{0, 1, 2, 3, 4}` plus 14 nodes, matching the doc's 19); executed: no
- Seen by: instrument-correctness; refutation: confirmed (same derivation); history: no-rationale-found (wording from the enumerator's birth at 4eec16bb)
- Owner-gated: no

`all_normal_events` folds raw trees over `BASES` through the normalizing constructor, which lifts minima and collapses equal leaves, so the corpus contains bases outside the alphabet. Every normal-form tree with in-alphabet bases is reached (its children are in the pool and `node` returns it unchanged), so the exhaustiveness claim holds; the description of what the 19 and 691 count is wrong at the corpus's definition site and at the constructor.

Evidence:

         6	//! corner is hit. This module closes that gap by brute force: it **enumerates
         7	//! every distinct normal-form id tree** up to a depth bound, and **every
         8	//! distinct normal-form event tree with bases in `{0, 1, 2}`** up to the same
         9	//! bound, then runs every operation on every tree and every ordered pair,
       150	/// Every distinct **normal-form** event tree of depth `≤ depth` with every base
       151	/// in [`BASES`].

    oracle/version.rs:
        82	        let m = l.base().min(r.base()).clone();
        86	            (Version::Leaf(a), Version::Leaf(b)) if a == b => Version::Leaf(n + m + a),

Resolution: State it as "the normal forms of every raw tree over the alphabet `{0, 1, 2}` (lifting and collapse carry bases up to `2(d + 1)`)" at both sites, and describe 19 and 691 as counts of that closure. Acceptance: the module doc and `all_normal_events`'s doc describe the closure.

### testing-oracles-19: The deep-enumeration docs carry a run chronology and per-leg nanosecond pricing beyond the budget-and-machine annotation the owner ruled on
- Where: crates/before/src/testing/exhaustive.rs:53-59 (related: crates/before/src/testing/exhaustive/tests.rs:427-436, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:597-612)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both passages; `git show d2a9d04e` on both files removes only the calendar date from each; the decision record at 609-612 reads "its budget and machine annotation the contract for whoever runs it"; `grep -rn tiled crates/before/src` hits only tests.rs:429); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (severity lowered because the ruling retains budget and machine annotation); history: deliberate-and-holds for the annotation (ruling #53, preserved by the d2a9d04e sweep); the chronology and pricing exceed what the ruling names, and the reason for keeping them is stated nowhere in the tree; "cache-tiled" names an experimental run that never landed, so the no-ghost rule is not triggered
- Owner-gated: no

Principle 5 sends dated measurement reports and incident chronology to git and decision records. Ruling #53 keeps "budget and machine annotation" on the test; the two passages additionally recount two aborted runs (a 45-minute cap, a 31-minute profile), a cache-tiled variant not in the tree, and machine-bound ns/pair figures per leg, which is the story of the measurement rather than its result. The decision record already holds that story. The present-tense contract (hour-scale; run detached with `cargo test` because nextest terminates at its slow-timeout budget; verdict legs only at the deep bound because they allocate nothing; strided samples undershoot because the expensive pairs are structurally similar trees near the diagonal) is correct and should stay.

Evidence:

        53	//! structural compare — which prices at roughly seven eighths of the pair
        54	//! worker (measured on a stride-sampled quarter of the deep corpus,
        55	//! aarch64-apple-darwin, 16 cores, release: the difference leg
        56	//! ~66 and the join leg ~12 of a ~91 ns/pair wall worker), and at the full

    exhaustive/tests.rs:
       427	/// about a minute. Measured state (aarch64-apple-darwin, 16
       428	/// cores, release, quiet machine): two fully parallel runs were stopped at a
       429	/// 45-minute cap, one row-major and one cache-tiled, the row-major one
       430	/// profiled still inside the verdict pair product at 31 minutes — budget
       431	/// upwards of an hour and run it detached. Sampled-corpus extrapolation

    decision record:
       609	  robustness); every oracle-facing suite bounded. DECIDED (owner):
       610	  the hour-scale verdict-pair totality test stays, and stays out
       611	  of the gate — `#[ignore]`d, run detached on demand, its budget
       612	  and machine annotation the contract for whoever runs it.

Resolution: Trim both passages to the ruled contract: the budget (hour-scale, run detached), the machine and profile annotation, the structural reason for the leg split stated as a relative present-tense fact, and the reason strided extrapolation fails. Remove the aborted-run chronology, the cache-tiled mention, and the ns/pair figures; if a completed run exists, state its wall time, and if none does, say so plainly (the history pass reports no completed run on record). If the owner wants the chronology kept, state at the site why. Acceptance: neither file mentions "cache-tiled", "45-minute", "31 minutes", or ns/pair figures; budget and machine annotation remain; the decision record is unchanged.

### testing-oracles-20: Hand-rolled injective dedup keys where the oracle types already derive `Hash + Eq`
- Where: crates/before/src/testing/exhaustive.rs:120-125 (related: crates/before/src/testing/exhaustive.rs:156-182, crates/before/src/testing/exhaustive.rs:184-229, crates/before/src/oracle/party.rs:11, crates/before/src/oracle/version.rs:36, crates/before/src/testing/exhaustive/tests.rs:568-627)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (oracle/party.rs:11 and oracle/version.rs:36 both read `#[derive(Clone, PartialEq, Eq, Hash, Debug)]`; `id_key`/`ev_key` are used only at exhaustive.rs 128, 141, 164, 174; `seen` is never iterated, so `BTreeSet`'s ordering buys nothing); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (severity lowered: test scaffolding with no behavioral consequence, and `corpus_counts_are_exact` already pins the dedup); history: no-rationale-found (the derives already existed at the enumerator's birth in 4eec16bb; the one stated reason, "`Party` has no `Ord`", explains `BTreeSet`'s key requirement, not why a `BTreeSet` was wanted)
- Owner-gated: no

The enumerators dedup through `BTreeSet<Vec<u8>>` keyed by 45 lines of hand-written preorder encodings carrying their own correctness argument ("this never collides", conditioned on bases staying single-byte). Dedup needs `Hash + Eq`, which both oracle types derive; a `HashSet<oracle::Party>`/`HashSet<oracle::Version>` dissolves both encoders and the argument. Prefer the mature capability over hand-rolling.

Evidence:

       120	    // `pool` holds the deduped *canonical* trees of depth `≤ d`, keyed for
       121	    // de-dup by a cheap injective preorder encoding (`Party` has no `Ord`).
       125	    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
       217	                out.push(0xff); // base terminator (bases are small; this never collides)

    oracle/party.rs:
        11	#[derive(Clone, PartialEq, Eq, Hash, Debug)]

Resolution: Replace `seen` with `HashSet<oracle::Party>`/`HashSet<oracle::Version>` (`seen.insert(t.clone())`), delete `id_key` and `ev_key`, and rewrite the comment to state only the every-level dedup rationale. Acceptance: `id_key`/`ev_key` are gone; `corpus_counts_are_exact` still reports `2^(2^d)` ids and 691 events with full denotation distinctness.

### testing-oracles-21: "At the bottom of this file" is no longer true, and "180 seconds" recomputes a number the nextest profile owns
- Where: crates/before/src/testing/exhaustive/tests.rs:49-51 (related: crates/before/src/testing/exhaustive/tests.rs:442-443, crates/before/src/testing/exhaustive/tests.rs:542-627, .config/nextest.toml:26)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (the symmetry section at 459-540 is followed by the corpus totality pins at 542-627; .config/nextest.toml:26 is `slow-timeout = { period = "60s", terminate-after = 3 }`); executed: no
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (the position claim predates ebec26fe's appended section; the 180 figure is accurate today, derived in two places)
- Owner-gated: no

Positional and numeric facts the tree can change without touching the prose rot silently (Principle 5).

Evidence:

        49	//! NOT checked here; it is an intrinsic algebraic property of the impl, tested
        50	//! directly and oracle-independently in the "intrinsic symmetry laws" section
        51	//! at the bottom of this file.
       442	/// (`cargo test`, not nextest: the workspace's nextest profile terminates
       443	/// any test at 180 seconds, which this enumeration exceeds).

Resolution: "in the intrinsic symmetry laws section of this file"; "the workspace's nextest profile terminates slow tests at its `slow-timeout` budget, which this enumeration exceeds". Acceptance: neither the position word nor the seconds figure remains.

### testing-oracles-22: `check_tick`'s grow-minimality pin has no liveness floor: a fill that always changes the tree skips it on every pair while both entry points stay green
- Where: crates/before/src/testing/exhaustive/tests.rs:313-316 (related: crates/before/src/testing/exhaustive/tests.rs:317-333, crates/before/src/testing/exhaustive.rs:47-50, crates/before/src/oracle/version.rs:247-251, crates/before/src/oracle/version.rs:345-347, crates/before/src/version/skyline/grow/tests.rs:288-343, crates/before/src/version/tests.rs:560, crates/before/src/version/tests.rs:588)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read `check_tick`: nothing counts pairs reaching line 317; `fill_for_test` delegates to `fill` at 345-347; `fill` is the identity for any `Party::Node` on a `Version::Leaf` (251) and for `Party::Leaf(true)` on a leaf (250, `max_ev()` of a leaf is its value), so every leaf event with every nonempty id takes the grow branch; grow/tests.rs:304-343 pins a count but gates on the impl-side `assert_grow_depth_safe`, not on the oracle's `fill_for_test`, so it would not notice an oracle-side regression; version/tests.rs:560 and 588 `prop_assume!` on `fill_for_test` only signal if the identity case is never reached); executed: no
- Seen by: adequacy; refutation: confirmed (the always-differing `fill_for_test` construction passes `exhaustive_small` with `best_inflation` and `all_inflations` executing zero times); history: no-rationale-found for the missing floor; the pin's presence at the deep bound is owner-ruled (#53: "tick with the brute-force grow-minimality pin"), so dissolving the pin is off the table and only the floor is open
- Owner-gated: no

Instruments before cures: every criterion needs a liveness floor derived from irreducible work so it cannot pass vacuously when its gate goes dark. exhaustive.rs:47-50 names this pin as the deep run's "irreplaceable value", and the gate that admits pairs to it is an oracle-internal branch probe with no count. The floor's premise is universal and needs no observation: the oracle's `fill` returns every leaf event unchanged for every nonempty id, so the grow branch is taken on at least `ids.len() * (leaf events in evs)` pairs.

Evidence:

       313	            // Grow-branch only: pin the inflation to the global brute-force optimum.
       314	            if ov.fill_for_test(op) != *ov {
       315	                continue; // fill simplified the tree; grow was not taken
       316	            }
       317	            let (best_tree, _cost) = best_inflation(op, ov).expect("non-empty id inflates");

    exhaustive.rs:
        47	//! The deep variant runs the *verdict* pair legs (`is_disjoint`, `covers` —
        48	//! borrowed operands, no allocation) and `tick` with the brute-force
        49	//! grow-minimality pin (the pin's irreplaceable value: `grow`'s DP held to the
        50	//! global optimum over all 65536 deep ids). The *structural* pair legs

    oracle/version.rs:
       249	            (Party::Leaf(false), _) => self.clone(),
       250	            (Party::Leaf(true), _) => Version::Leaf(self.max_ev()),
       251	            (Party::Node(..), Version::Leaf(n)) => Version::Leaf(n.clone()),

Resolution: Count the pairs that reach line 317 (an `AtomicUsize` under the par iter, returned from `check_tick` or asserted inside it) and assert `grow_pairs >= ids.len() * evs.iter().filter(|v| matches!(v, oracle::Version::Leaf(_))).count()`, with the premise stated at the assertion site (fill is the identity on a leaf event for every nonempty id). Acceptance: temporarily making `fill_for_test` return `self.fill(id).tick(&Party::Leaf(true))` (always different from `self`) turns `exhaustive_small` red on the new floor rather than green; restored, it is green.
Construction: In crates/before/src/oracle/version.rs change `fill_for_test` to return `self.fill(id).tick(&Party::Leaf(true))`; run `cargo nextest run -p before exhaustive_small`: every pair hits `continue`, `best_inflation` and `all_inflations` execute zero times, and the test passes. With the floor added, the same mutation fails the floor's assertion.

### testing-oracles-23: `corpus_is_canonical`'s `> 20` non-triviality floors are superseded by the exact-count pin that names them inadequate
- Where: crates/before/src/testing/exhaustive/tests.rs:384-395 (related: crates/before/src/testing/exhaustive/tests.rs:550-552, crates/before/src/testing/exhaustive/tests.rs:568-627)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read both tests; `corpus_counts_are_exact` pins `2^(2^d)` for every `d` in `0..=ID_SMALL_DEPTH` and the 691 literal plus denotation distinctness, and its doc at 550-552 names the floor as admitting shrinks past half the corpus; ebec26fe's message says the replacement was demonstrated red under a constructed shrink); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (ebec26fe landed the replacement, met the retire-an-instrument bar, and left the floors in place with no stated reason)
- Owner-gated: no

Retiring an instrument requires the replacement to demonstrate it catches what the instrument caught; the replacement's own doc and commit make that demonstration, and the floors now catch nothing the exact pin misses. The `is_normal` sweep in the same test has no replacement and stays.

Evidence:

       384	    // The corpus is non-trivial (guards against an enumeration that silently produces
       385	    // nothing and makes every cross-product loop vacuous).
       386	    assert!(
       387	        ids.len() > 20,
       388	        "id corpus suspiciously small: {}",
       389	        ids.len()
       390	    );
       551	/// (`corpus_is_canonical`'s non-triviality floor admits shrinks of more
       552	/// than half the corpus). This pin closes that hole two ways:

Resolution: Delete lines 384-395 and the parenthetical at 551-552; keep `corpus_is_canonical` as the normality sweep and reword its doc accordingly. Acceptance: `corpus_is_canonical` asserts only `is_normal`; no `> 20` literal remains; a deliberate enumeration shrink (drop `P::Leaf(true)` from the seed loop) still fails `corpus_counts_are_exact`.

### testing-oracles-24: `exhaustive_deep` has no recipe and nothing runs it, while `generators.rs` defers deeper coverage to it
- Where: crates/before/src/testing/exhaustive/tests.rs:438-446 (related: crates/before/src/testing/generators.rs:293-298, crates/before/src/testing/exhaustive.rs:41-45, justfile:550, justfile:958, .github/workflows)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -n 'exhaustive\|--ignored' justfile` hits only an unrelated comment at 651; `grep -rn 'schedule\|cron\|--ignored\|exhaustive' .github/workflows/` is empty; the fuzz (justfile:550) and worst-cases (justfile:958) manual tiers have recipes; generators.rs:295-297 reads "deeper coverage is the job of the (ignored) exhaustive variant"); executed: no
- Seen by: adequacy, structure-prose; refutation: confirmed; history: already-known (ruling #53: the test "stays, and stays out of the gate — `#[ignore]`d, run detached on demand"; the ruling is silent on a recipe or cadence, so a recipe is additive; deleting the test would contradict the ruling)
- Owner-gated: yes (the ruling fixes the disposition; a recipe or cadence is a policy addition)

AGENTS.md makes the justfile the source of truth for verification, with a recipe per artifact. The deep enumeration's invocation lives only in a doc comment, no process executes it, and a documented coverage claim in generators.rs rests on it. A `just` recipe gives the run a name in `just --list`, a place for the nextest caveat, and a target for the deferral.

Evidence:

       438	/// ```text
       439	/// cargo test -p before --release --all-features -- --ignored exhaustive_deep
       440	/// ```
       444	#[test]
       445	#[ignore = "exhaustive deep enumeration: O(corpus^2) over 65536 ids; hour-scale, run detached"]
       446	fn exhaustive_deep() {

    generators.rs:
       295	/// Kept small so the default proptest run stays CI-cheap while still covering
       296	/// every arm; deeper coverage is the job of the (ignored) exhaustive variant
       297	/// and the deep-tree stack-safety test.

Resolution: Add a manual-tier `just exhaustive-deep` recipe wrapping the documented `cargo test` line, its recipe comment carrying the budget and the cargo-test-not-nextest reason; point this doc and generators.rs:295-297 at the recipe; optionally give it a scheduled CI job so the deferred coverage is exercised on a cadence. Acceptance: `just --list` shows the recipe; the doc names it instead of an inline command; generators.rs's deferral names something that runs.
Construction: `grep -n exhaustive justfile` and `grep -rn -- '--ignored' .github/workflows/` return no relevant hit.

### testing-oracles-25: The five "intrinsic symmetry laws" restate five `crate::laws` predicates one for one, and the ruling that keeps them outside the roster is stated only in the decision record
- Where: crates/before/src/testing/exhaustive/tests.rs:459-540 (related: crates/before/src/testing/exhaustive/tests.rs:47-51, crates/before/src/laws.rs:444-456, crates/before/src/laws.rs:522-526, crates/before/src/laws.rs:2183-2213, crates/before/src/testing/algebraic_laws.rs:11-12, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:717-729)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read the five bodies against laws.rs: `id_is_disjoint_is_symmetric` is `disjoint_symmetric` (2184-2186); `id_join_is_commutative` is a weaker `join_commutative_outcomes` (2203-2213, which also pins the `Err` hand-back); `event_partial_cmp_is_antisymmetric` is `partial_cmp_is_dual` (524-526), not `order_antisymmetric` (508); `event_merge_is_commutative` is `merge_commutative` (448-450); `event_meet_is_commutative` is `meet_commutative` (454-456); none of the five test names is cited by surface.rs, surface_coverage.rs, or diff_ops.rs; `VERSION_PAIR` holds 32 laws and `PARTY_PAIR` 10, counted by `fn` between the `pub static` headers); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed (severity lowered: nothing is caught or missed today; adds the cost caveat); history: already-known (ruling #75: "Bespoke law tests dissolved name-for-name; kept deliberately outside: the exhaustive small-scope point laws"; the rationale lives only in the record, and the section comment at 461-465 argues oracle-independence and totality without mentioning the roster)
- Owner-gated: yes (reopens a recorded ruling with a proposal it did not consider)

Two spellings of the same law surface: a law added to laws.rs gets sampled coverage only, and a symmetry retyped here drifts independently (the join copy already omits the `Err` hand-back the law pins). The section's stated payoff, oracle-independent and total, is exactly what iterating `laws::PARTY_PAIR` and `laws::VERSION_PAIR` over the corpus pairs would deliver for every pair law rather than five. Ruling #75 declined folding the point laws into the collection; driving the collection over the exhaustive corpus is a different proposal it did not rule on. Whatever the owner decides, the reason the five stay hand-spelled belongs at the site. One naming nit rides along: the test and the module doc (47-48) call the dual property "anti-symmetric", while laws.rs reserves `order_antisymmetric` for `le(a, b) && le(b, a) => a == b` and names this property `partial_cmp_is_dual`. Cost caveat for the total drive: 32 laws over 691^2 event pairs is roughly 15M law evaluations, so the gate-resident placement needs a measurement, following the leg-split logic exhaustive.rs already applies.

Evidence:

       461	// The op symmetries are intrinsic algebraic properties of the impl, so they are
       462	// tested DIRECTLY on the impl — no oracle, and not folded into the differential
       463	// checks above. Two payoffs: a symmetry bug the oracle happened to *share* is
       464	// still caught here, and (being deterministic + total over the small-scope
       465	// corpus) the guarantee is total, not sampled.
       509	        assert_eq!(
       510	            imp[i].partial_cmp(&imp[j]),
       511	            imp[j].partial_cmp(&imp[i]).map(Ordering::reverse),

    laws.rs:
       524	    fn partial_cmp_is_dual {
       525	        a.partial_cmp(b) == b.partial_cmp(a).map(Ordering::reverse)
       526	    }
      2184	    fn disjoint_symmetric {
      2185	        a.is_disjoint(b) == b.is_disjoint(a)
      2186	    }

    decision record:
       724	  `dangerously_alias`, confined to predicate scope. Bespoke law
       725	  tests dissolved name-for-name; kept deliberately outside: the
       726	  exhaustive small-scope point laws, the population/fold laws

Resolution: At minimum, add the ruling's rationale to the section comment (why these five stay outside the roster). Preferably, replace the five with two total drivers (`for (name, law) in laws::PARTY_PAIR` over `par_for_pairs` on `impl_ids`, and the `VERSION_PAIR` twin over `impl_events`, each asserting `law(&a, &b)` with the law name in the message), measure the wall time, and place them in `exhaustive_small` or the deep tier by that measurement; rename or re-doc the antisymmetry test if it stays. Acceptance: either the section comment states the ruling's reason, or the five tests are gone, every `PARTY_PAIR`/`VERSION_PAIR` law is asserted over the full small corpus, and temporarily inverting one predicate in laws.rs fails the driver naming that law.

### testing-oracles-26: The organic drive pairs versions with foreign clocks' regions without saying whether that is the intended regime
- Where: crates/before/src/testing/algebraic_laws/tests.rs:358-369 (related: crates/before/src/testing/algebraic_laws/tests.rs:310-328, crates/before/src/testing/algebraic_laws/tests.rs:413-418)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read the arms and the `Organic` construction: `v[k]` and `p[k]` come from the same picked clock at 414-418; the four version-with-party arms use `p[1]`/`p[2]` and never `p[0]`, so the owner pairing arises only when picks coincide, `i % n == j % n`); executed: no
- Seen by: structure-prose; refutation: reframed (coincident picks reach the owner pairing by chance, so "never" is too strong; the regime is undocumented and incidental); history: no-rationale-found (birth shape from 86dd53a7)
- Owner-gated: no

The index choice encodes a population regime (a version paired with a foreign live region versus its own owner) that is invisible without knowing how `Organic` is built; comments state what the code cannot show.

Evidence:

       358	    (@one $env:expr, $group:ident, (version, party)) => {
       359	        assert_laws!(laws::$group, $env.v[0], $env.p[1]);
       360	    };
       361	    (@one $env:expr, $group:ident, (version, version, party)) => {
       362	        assert_laws!(laws::$group, $env.v[0], $env.v[1], $env.p[2]);
       363	    };
       364	    (@one $env:expr, $group:ident, (version, party, party)) => {
       365	        assert_laws!(laws::$group, $env.v[0], $env.p[1], $env.p[2]);
       366	    };

Resolution: One comment above the arms (or on `Organic`) stating the regime, for example that versions meet foreign live regions here because the owner pairing is the op trace's own tick, already exercised by the replay; or, if the owner pairing is wanted under mass, use `p[0]` beside `v[0]` in one arm. Acceptance: the pairing regime is stated where the indices are chosen.

### testing-oracles-27: The organic population replays the trace twice and takes versions and parties from the oracle through the bridge, although the impl replay already holds them
- Where: crates/before/src/testing/algebraic_laws/tests.rs:411-436 (related: crates/before/src/testing/algebraic_laws/tests.rs:24-35, crates/before/src/testing/algebraic_laws.rs:20-22, crates/before/src/clock.rs:625, crates/before/src/clock.rs:640, crates/before/src/party.rs:534)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (read 411-436: `run(&ops)` for the oracle, `ver`/`party` through the bridge, then `step_impl` for clocks; `Clock::party() -> &Party` at clock.rs:625, `Clock::version() -> &Version` at 640, `Party::dangerously_alias` at party.rs:534 make impl-sourced values feasible with the aliasing the clock list already uses at 435); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (the shape 86dd53a7 landed with; `dangerously_alias` predates it by seven weeks)
- Owner-gated: no

The suite's claim is that the laws hold on "the value shapes real fork/tick/join/sync schedules produce"; sourcing versions and parties from the impl's own replay makes that literally about the impl's values, removes a bridge dependency from a suite whose module doc says the oracle is "only a source of bits", and halves the replay work. `Version` is `Clone` and parties alias through the documented escape hatch.

Evidence:

       411	        let cs = run(&ops);
       412	        let n = cs.len();
       413	        let picks = [i % n, j % n, k % n];
       414	        let (pa, va) = cs[picks[0]].trees();
       417	        let (ia, ib, ic) = (ver(va), ver(vb), ver(vc));
       418	        let (qa, qb, qc) = (party(pa), party(pb), party(pc));
       426	        // Clocks: replay the trace on the impl for real, reachable clocks.
       427	        let mut imp = vec![Clock::seed()];
       428	        for op in &ops {
       429	            step_impl(&mut imp, op);
       430	        }

Resolution: Replay once on the impl; derive `v` from `imp[..].version().clone()`, `p` from `imp[..].party().dangerously_alias()`, and the lists likewise; drop `run(&ops)` and the `ver`/`party` calls in this test. Acceptance: the organic test imports neither `run` nor the bridge; every law group still drives; the committed seeds in proptest-regressions/testing/algebraic_laws/tests.txt replay green.

### testing-oracles-28: The validation index claims to map every instrument but has no row for at least eight committed instruments, misdescribes the exhaustive corpus as "reachable states", and gives the replay keystone no row
- Where: crates/before/src/testing/validation_index.rs:1-3 (related: crates/before/src/testing/validation_index.rs:11-12, crates/before/src/testing/validation_index.rs:64-66, crates/before/src/testing/validation_index.rs:164-170, crates/before/AGENTS.md, justfile:321, justfile:366, justfile:641, justfile:853, justfile:941, crates/before/tests, crates/before/fuzz, crates/before/wasm32-pins, crates/before/surfacecheck, tools/covcheck, tools/mutantcheck, .cargo/mutants.toml)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (`grep -n -i 'wasm\|mutant\|covcheck\|surfacecheck\|verdict\|superlinear\|tamper\|fuzz\|tripwire'` over the index matches only the fuzz-fit bands, "shared with the fuzz targets", "fuzz seeds", and generic uses of "coverage"/"tripwires"; every omitted instrument exists on disk and has a justfile recipe (`mutants-list` 321, `fuzz-build` 366, `wasm32-pins` 641, `bench-judge-tripwire` 853, `surface-totality` 941) or a `tests/*.rs` binary; `tools/citecheck` scopes itself to surface.rs, diff_ops.rs, and surface_coverage.rs; the module is never rendered (testing-oracles-2), so no link check runs; the exhaustive enumerator produces canonical trees whether or not any op sequence reaches them, per exhaustive.rs:106-116 and tests.rs:252-253); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: no-rationale-found ("every instrument" is the recorded design intent and stood at birth in 669cf310; the omitted instruments were never rowed; no ruling narrows the page)
- Owner-gated: no

The page is the crate's designated orientation map (before's AGENTS.md routes a maintainer to the testing module docs, and the review brief names it the validation map). It opens with a totality claim and has no row for the fuzz targets, the wasm32 pins, the rustdoc-JSON surface totality check, the coverage pins, the mutants roster, the verdict matrix, the superlinear tripwires, the bench-judge tripwire, or the `tests/` tamper pins, so a maintainer asking "is there a fuzz target for non-canonical decode?" concludes there is none. Its own bar for a new instrument ("a failure class no row below already catches") cannot be applied against rows that are missing. Unlike the crate's other rosters (surface held to the `pub fn` scan, `for_each_law_group!` pinned, `TRIPWIRES` names checked live), nothing holds this page to the instruments that exist. Two rows misdescribe: "every reachable state" is a reachability claim the exhaustive enumerator does not make (it enumerates canonical normal-form trees, including shapes the tests doc says "the op pipeline never builds"), and the function-space replay, the one thing the semantic oracle alone catches (a bug both tree recursions share; dependence on the impl's particular fork/inflation policy), has no row of its own.

Evidence:

         1	//! The validation index: every instrument that guards this crate, what
         2	//! failure class each one catches that the others cannot, and where it
         3	//! lives.
        11	//! trips one instrument, this page says which neighbors to check; when a
        12	//! new instrument is proposed, the bar is a failure class no row below
        64	//! **Exhaustive small-scope enumeration** ([`super::exhaustive`]). Total
        65	//! enumeration of every reachable state and operation pairing inside
        66	//! small bounds, checked against the oracle. What it alone catches:

Resolution: Either add one row per omitted instrument at the same altitude (what it alone catches, where it lives, which recipe runs it), plus a row for the fs replay, or narrow the opening sentence to the classes the page covers and point at `just --list` and the justfile's recipe comments as the recipe-level inventory. Rewrite the exhaustive row as "every canonical normal-form id tree to `ID_SMALL_DEPTH` and event tree to `EV_SMALL_DEPTH`, every ordered pair, for the operations its check set names; kernel suites sweep further operations over the same corpus". Owner's call whether to pin the page mechanically (a test that every recipe under the gate/ci composition and every `tests/*.rs` binary is named in the index). Acceptance: every verification recipe the justfile's gate and ci compositions run for before, and every `crates/before/tests/*.rs` binary, is named in the index or the header states the page's scope and where the rest is indexed; the exhaustive row says "canonical normal-form trees"; `grep -n -i 'wasm32\|covcheck\|mutants\|surfacecheck\|verdict_matrix\|superlinear' validation_index.rs` finds each.
Construction: The grep in the acceptance clause returns nothing at this commit; `ls crates/before/{fuzz,wasm32-pins,surfacecheck} tools/{covcheck,mutantcheck} .cargo/mutants.toml crates/before/tests/{verdict_matrix,superlinear_tripwires}.rs` all exist. For the exhaustive row: `all_normal_events(2)` contains trees with base patterns no tick sequence over a seed-derived party produces at that depth, and they are checked, so reachability is not the enumerated property.

### testing-oracles-29: Validation index prose: a ghost reference to replaced bodies, "mint", "honest", a self-description hedge, and a hand count
- Where: crates/before/src/testing/validation_index.rs:42-45 (related: crates/before/src/testing/validation_index.rs:5-6, crates/before/src/testing/validation_index.rs:75, crates/before/src/testing/validation_index.rs:142, crates/before/src/testing/validation_index.rs:178, crates/before/src/testing/validation_index.rs:182)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read each site; the "four instruments" at 75 enumerates four enforcing mechanisms and the fifth bold entry, the atlas, is described at 139-142 as enforcing nothing, so the count is consistent today); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed (the count item reframed as a hand count that is defensible as written); history: contradicts-hard-rule for "replaced" (entered at 5a3f9cb1, after the d2a9d04e ghost sweep; the bodies were deleted the next day in 223795c1) and for "mint"/"honest" (the writing-style rule postdates the prose)
- Owner-gated: no

The root AGENTS.md hard rule forbids referring to code that no longer exists ("The hand-written bodies it replaced"); the vocabulary rule bans "mint" for constructing a value and moralized words like "honest" where no adversary exists; "in the spirit of a documentation-only module" hedges a description of a module that is one; "four instruments" is a hand count that rots on the next addition even though it is consistent today.

Evidence:

        42	//! op-trace values, or the reverse. The hand-written bodies it replaced
        43	//! could not prevent one: each chose its own population, so coverage was
        44	//! a product nobody enumerated. Here the population belongs to the driver
        45	//! and the operation to the descriptor, and the two meet by construction.
         5	//! This page is a map for a maintainer orienting cold, in the spirit of
         6	//! a documentation-only module: it holds no code. Two questions organize
        75	//! Cost claims are guarded by four instruments in a deliberate layering:
       142	//! read the roster but never mint a threshold.
       178	//!   or an honest input legitimately did less work than the floor's
       182	//!   change, whichever direction is honest.

Resolution: 42-45: state the property positively ("A body that chose its own population would make coverage a product nobody enumerated; here the population belongs to the driver and the operation to the descriptor, so the two meet by construction."). 5-6: "This module is documentation only: it holds no code." 75: "guarded by a deliberate layering of instruments". 142: "never set a threshold". 178: "a legitimate input did less work than the floor's premise". 182: "whichever direction the code supports". Acceptance: none of "replaced", "mint", "honest", "in the spirit of", or "four instruments" remains in the file.

## Positives

- The known-bad conviction pair (`rank_differential_convicts_the_cell_dropping_riemann_sum`, `worked_value_anchor_convicts_the_mirrored_embedding`; semantic_oracle/tests.rs:375-527) is the adequacy doctrine done right: each defective reference is committed behind an inverted assertion over a committed asymmetric family, and the mirrored-embedding test proves rather than argues that the pointwise differential is blind to a twin-substituted mirror, which is why the absolute-geometry anchor is necessary. Both are rostered by name in `surface_coverage::TRIPWIRES` (133-138) and resolved against nextest's live inventory by `tools/citecheck` from outside the crate.
- `corpus_counts_are_exact` (exhaustive/tests.rs:567-627) is the right shape of totality pin: the id count is derived in-test in closed form (`2^(2^d)`, the bijection with owned-cell subsets), and event distinctness is checked by an independent path-sum walk sharing no code with `node`, normalization, or the dedup key. Its doc names the exact shrink it catches.
- The grid discipline is spec-first: `GRID_N` is derived from `optrace::MAX_TRACE_OPS` (semantic_oracle.rs:80), `fork` asserts the ceiling instead of clamping (378-383) so an out-of-derivation trace fails loudly rather than aliasing, `fs_grid` argues explicitly against a silent cap while asserting the one representation bound (86-104), and `fork_chain_raises_resolution_one_level_per_fork` meets the derivation's rate premise with equality on the worst in-support schedule, deterministically.
- The law drivers expand both genres from the single `for_each_law_group!` roster (algebraic_laws/tests.rs:57-304, 336-394): a group with a known signature is driven with no wiring, a novel signature refuses to compile until an arm says how to feed it, and every assertion names the violated law. The roster's 18 signatures match both macros' arm sets exactly.
- The bridge lowers the impl's stored bits directly rather than through the public codec (bridge.rs:1-12), so structural agreement and codec correctness cannot share a failure mode; `read_ev` decodes the zigzag skyline stream independently of production decode and carries `Base`-typed heights so lowering is lossless for any magnitude; `to_oracle_version` normalizes once at the root.
- The semantic oracle's random section-4-valid `fork` and `event` policies (semantic_oracle.rs:303-407) test the one property no other instrument reaches, policy independence of the observable partial order, and the single-seed restriction is argued from the model (several seeds each owning `[0,1)` is not a valid configuration), not from convenience.
- The exhaustive suite drives every impl reading through public ops with the mutating and consuming contracts asserted as such: `check_id_pair_algebra` (172-229) pins the refused join's leave-self-unmodified and hand-back, and `without`'s `None` exactly when the remainder is empty; `check_ev_pairs` pairs every structural leg with a byte-equality leg so representation drift cannot hide behind structural agreement; the diagonal is included deliberately.
- The deep variant's leg split (verdict legs plus `tick` at the deep bound, structural legs at the small bound) is recorded with an owner ruling, drops the anonymous id once at lowering with the public-domain reason stated, and correctly names the nextest terminate budget as the reason to run under `cargo test`.

## Open questions for Finch

1. Stale-signature proptest seeds. `crates/before/proptest-regressions/testing/semantic_oracle/tests.txt` lines 8-9 record `seeds = 2, ops = [...]` for a keystone signature that no longer has a `seeds` input (the live property is `replay_matches_across_references(ops, seed)`), and line 7 records a single-parameter `p` shrink. They still replay as RNG seeds for every property in the file, so they hold nothing specific but cost nothing. The doctrine forbids stripping seeds; do you want stale-signature seeds pruned as a deliberate, named act, or left in place? Recommendation: leave them (the cost is nil and the rule is bright-line), but this is worth a one-line ruling so the next reader does not relitigate it.
2. The validation index's charter (testing-oracles-2, -28): a total map with a light liveness pin (a test that every gate/ci recipe and every `tests/*.rs` binary is named), rendered under `just docs-internal`; or a scoped, source-read page that names the justfile as the inventory of record? Recommendation: total and rendered, because before's AGENTS.md routes maintainers there and the rest of the crate holds every roster to the surface it enumerates.
3. `exhaustive_deep` (testing-oracles-24): has it completed since the #53 leg split, and do you want a `just exhaustive-deep` manual-tier recipe with a scheduled run? Recommendation: add the recipe, run it once detached, and record the completed wall time in the doc in place of the aborted-run chronology; if no completed run exists, the doc should say so.
4. The bridge's id walks (testing-oracles-3): guard all four walks uniformly with a threaded depth, or document the id side's exemption and correct recurse.rs and AGENTS.md? Recommendation: guard uniformly; the cost is nil and the inventory statements become true as written. Either way, is `descend!(0, …)` on recursive calls intended (probe every level) or a misreading of the macro's depth argument? Recommendation: thread depth as the macro doc prescribes.
5. The sampled grid sweep (testing-oracles-17, with -10): convert the keystone's silent clamp into an assert so every keystone case guards both sides, retire `grid_cap_is_never_reached`, and re-point its two roster citations to the chain pin; or keep the sweep trimmed to its function-space legs as the event-side guard? Recommendation: assert in the keystone and retire the sweep; a silent clamp is exactly the failure mode `fs_grid`'s doc refuses, and an every-case assert subsumes a 400-case sample of the same distribution.
6. The exhaustive point laws (testing-oracles-25): #75 kept them outside the roster; driving `PARTY_PAIR`/`VERSION_PAIR` totally over the small corpus is a different proposal. Do you want that measured, or the ruling's rationale stated at the site and the five kept? Recommendation: measure the total drive; if it fits the gate, adopt it and retire the five; if not, place the version half in the deep tier and state the reason at the site.

## Dropped

- [51] `Id`/`Event`, `id_res`/`ev_res`, `id_depth`/`ev_depth` differ only in codomain: taste; the doctrine prefers newtypes over synonyms, the depth walks traverse different oracle enums, and no cost was named.
- [46] "canary" sites: established test jargon, exempt under the vocabulary rule; "widened codec" at exhaustive/tests.rs:233 is anchored crate-wide (codec/int.rs, version/tests.rs), so no rename; the "keystone"/"honest"/"genuine" sites survive as testing-oracles-12.
- [15] as stated (`fs_grid`'s `debug_assert!` compiled out by the prescribed `--release` run): premise wrong, since the `--release` line belongs to `exhaustive_deep`, whose suite never calls `fs_grid`, and the dev profile keeps debug assertions on; the stylistic point survives inside testing-oracles-5.
- [44]'s "four instruments" hand count as a defect: the sentence enumerates four enforcing kinds and the atlas is described as enforcing nothing, so the count is consistent today; kept only as a hand-count mention inside testing-oracles-29.
- [1]'s proposal to retire `grid_cap_is_never_reached` outright: refuted by the refutation pass (the `ev_res` legs are the only committed event-side bound); reframed into testing-oracles-17.
- [37]'s tautological `MAX_TRACE_OPS <= GRID_N` assert as a standalone finding: history shows it was written as a guard against redefining `GRID_N`, and its arithmetic is stated inline; folded into testing-oracles-10 as "make it a const assertion".
- [18]'s alternative to dissolve the grow-minimality comparison in `check_tick`: contradicts ruling #53, which places the pin at the deep bound; only the floor survives, as testing-oracles-22.
- [5]'s framing of "cache-tiled" as a ghost reference to deleted code: history shows it names an experimental run that never landed; the chronology point survives as testing-oracles-19.
- [22]'s mapping of `event_partial_cmp_is_antisymmetric` to `order_antisymmetric`: the matching law is `partial_cmp_is_dual` (laws.rs:524-526); corrected inside testing-oracles-25, where the misnomer becomes a nit.
- [41]'s claim that "master harness" recurs at clock/tests.rs:562: the crate-wide grep finds the phrase only at bridge.rs:104 and the banner "master differential harness" at clock/tests.rs:192; corrected inside testing-oracles-6.
- Refutation's new observation on `event_dominates_local_and_advances`'s grid (`ev_depth + 1` but not `id_depth + 1` at tests.rs:303): correct as written and harmless; below the bar.
- Duplicates merged: [0]=[22] into -25; [1]=[37] into -17; [2]=[38] into -20; [3]=[19]=[55] into -28; [4]=[20]=[33]=[59] into -16; [5]=[29]=[32]=[61] into -19; [6]=[39]=[66] into -23; [7]=[31]=[35] into -11; [8]=[21]=[36]=[58] into -10; [10]=[23]=[40]=[62] into -1; [11]=[48] into -15; [12]=[34]=[65a] into -13; [13]=[17]=[27]=[44] into -29 (with -12 for the semantic-oracle sites); [14]=[24]=[43]=[56] into -3; [16]=[30]=[42]=[50]=[65] into -5; [25]=[53] into -24; [26]=[47]=[60] into -9.
