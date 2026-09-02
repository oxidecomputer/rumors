# Partition version-core: Version: the event tree type, owned versions, ticks, hull traffic, and the version test suite

## Partition summary

`Version` is a single `codec::Bits` (a refcounted, marker-padded canonical skyline stream) and every operation on it is a one-line door into `skyline::*`: `tick`/`ticks` into `fill`, `rank`/`distance`/`lag`/`min_ticks` into `query`, comparison into `sweep::causal_cmp`, equality into `codec::canonical_eq`. The lattice kernels sit in `version.rs` as five short-circuit ladders (`join_view`/`join_refs`, `meet_view`/`meet_refs`, `span_refs`) over `skyline::emit`; the n-ary folds (`join_all`, `meet_all`, `span_all`, `Sum`, `FromIterator`) run `crate::fold::balanced_reduce` with a `DedupRuns` adapter that collapses adjacent clone runs by pointer identity. The operator matrices (`|`, `&`, `^`, `/`, comparison) are macro-generated from one source each. `own.rs` holds `OwnVersion`, the lazy projection view whose comparison matrix routes every cell through the fused masked co-walk; `ticks.rs` holds `Ticks` (an unbounded count over `codec::Base`) and its `Limbs` iterator; `hull_traffic.rs` holds the feature-gated per-rung counters for the span ladder. The test files are `version/tests.rs` (2402 lines), `version/own/tests.rs` (144), and `version/ticks/tests.rs` (122); the production files total 2404 lines. All seven files were read in full with line numbers: 5072 lines.

The production code is clean under every lens: no recursion, no unsafe, every `expect` guarded by the receiver seed, the one `unreachable!` a genuine one-line proof resting on canonicality with the premise pinned (`eq_matches_causal_walk`), and both n-ary folds keeping their combine matches total rather than asserting on the arm the counter's weight discipline excludes. The maintainer comments at the two places a reader stumbles (`Version::new`'s static-not-const, `DedupRuns` holding a clone rather than an address) state the why. No correctness defect survives; the four lens reports, the refutation pass, and the history pass agree on that.

The dominant issue is machinery that outlived the `Batch` type (removed 2026-07-27): `join_view`/`meet_view` still take a foreign `&codec::Bits`, but every remaining caller passes a `Version`'s `.view()`; their bodies duplicate the `_refs` ladders rung for rung, held in step only by prose, and that duality in turn justifies the three-strategy `binop_matrix!`, the `view` parameter on `balanced_fold`, and the `lo_view`/`hi_view` fields in span algebra. Beside it, `span_all` copies `balanced_fold`'s four-arm dispatch (a third copy lives in span algebra), and the join/meet encoded-size subadditivity lemma that the folds' auxiliary-space bounds and rumors's window budget both rest on is derived only in test prose. The remainder is documentation and test hygiene: a stale module doc over a 2402-line test file hosting the `Rank`/`Ranked` suites, one duplicated proptest, five orphaned proptest seeds, an assert message pointing at prose a later commit excised, hand-transcribed measurements in a test doc, and a handful of public-doc slips (a wrong intra-doc link, a ghost example name, a sentence copied from `meet_all` without adaptation, two owner-authored paragraphs with syntax and likelihood problems).

Vocabulary and register items (`mints`, `honest`/`historical`, unanchored `door`, em-dashes in `//` comments) are real but crate-wide: the governing rules landed on 2026-08-10, after most of this prose; they are listed once with the partition's sites for a single sweep rather than as partition defects.

## Findings

### version-core-1: `Version::new`'s cross-call clone-identity claim has no pin
- Where: crates/before/src/version.rs:124-128 (related: crates/before/src/party.rs:122-129; crates/before/tests/meter.rs:10746-10797; crates/before/src/codec/tests.rs:183-188)
- Class / severity / confidence: claim / nit / high
- Provenance: verified (grep for `ptr_eq`/`from_static` over version/tests.rs, codec/tests.rs, clock/tests.rs, party/tests.rs, tests/meter.rs: no test compares two `Version::new()` or two `Party::seed()` results by pointer); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale for the missing pin (the static choice itself is deliberate, 1bb610d19)
- Owner-gated: no

The comment stakes a constant-factor claim (two independent `new()` calls answer `ptr_eq`) on `static` versus `const` promotion semantics, and nothing committed observes it. The standard for asymptotic and constant claims asks for a committed instrument; `codec/tests.rs:183-188` pins that two independently frozen *empty* `Bits` alias, a different mechanism (the zero-byte dangling pointer, not this one-byte static), and the identity fast-path meter rows drive `Version::new()` against a value, never two empties against each other.

Evidence:

       124	        // `0b1110_0000`: construction allocates nothing, and every empty
       125	        // version shares the one static buffer (clone identity holds
       126	        // even across separate `new()` calls). A `static`, not a
       127	        // `const`: a const's promoted allocation has no guaranteed
       128	        // unique address, and the cross-call sharing claim rests on one.

Resolution: one test-only assertion, `assert!(Version::new().view().ptr_eq(Version::new().view()))`, with the `Party::seed()` twin; or drop the parenthetical and let the `static` speak for itself. Acceptance: a committed test reads red when `EMPTY_STREAM` becomes a `const`, or the comment no longer claims cross-call sharing.

Construction: change `static EMPTY_STREAM` to `const EMPTY_STREAM`; every committed test stays green whether or not the promoted allocation happens to be shared.

### version-core-2: The `M` caption is pasted five times with inconsistent wrapping
- Where: crates/before/src/version.rs:293-293 (related: crates/before/src/version.rs:364-366, 411-413, 1058, 1081; crates/before/fuelscape/version_rank.json)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grep `Typical inputs run far below` in version.rs: five hits, three one-line and two wrapped; `fuelscape/version_rank.json`'s contract reads `O(M(|self|) · log |self|)` with `M` undefined there); executed: no
- Seen by: prose; refutation: confirmed; history: accretion (b5a81583, 2efff149), no placement decision
- Owner-gated: no

The sentence explaining `M` in the rank-family fuelscapes is a caption for the generated island, not a per-method contract, and five hand-synchronized copies already differ in layout. Superfluity competes with the contract, and copies drift.

Evidence:

       293	    /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).

Resolution: emit the caption from the fuelscape island renderer for every island whose contract uses `M`, or hoist it to one `# Complexity` note on `Rank` that the five sites link to; failing that, wrap all five identically. Acceptance: the sentence appears once in the tree (generator or `Rank` doc) and the five methods carry only the island include.

### version-core-3: Small typographic and consistency slips across the partition
- Where: crates/before/src/version.rs:320-321 (related: crates/before/src/version.rs:914, 121-130, 153-157, 1192, 1203; crates/before/src/version/own.rs:12, 19-20; crates/before/src/version/hull_traffic.rs:16-17, 77, 92, 108; crates/before/src/version/tests.rs:89)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (each site read; own.rs:12 checked against the `Div` impl at version.rs:1703, which exists only for `&'a Version`; the `codec::scan` link at hull_traffic.rs:17 targets the module `crate::codec`, and `codec::scan` exists at codec.rs:31 per the refutation pass); executed: no
- Seen by: prose (bundle), correctness (own.rs:12); refutation: confirmed; history: slips, except (e), whose header is expired (a242c56e5's "event mutation" section once held tick/fill/grow tests that migrated to `crate::laws`)
- Owner-gated: no

Eight thirty-second fixes that a careful reader notices together: (a) two consecutive empty `///` lines before `# Complexity`; (b) `a ∧ a == a` at 914 where 866, 891, 936 write `= a`; (c) own.rs:19-20 "so is kept explicit" has no subject; (d) own.rs:12 spells the projection `v / &p` while the module doc (own.rs:1), the section comment (version.rs:1662), and the only `Div` impl (`&'a Version`) spell it `&v / &p`; (e) tests.rs:89's "event mutation" header heads join-identity, subadditivity, operator-matrix, and byte-parity tests, no tick test; (f) 121-130 and 153-157 both derive the 2-bit empty stream in full; (g) `from_bits`/`from_frozen` (1192, 1203) and hull_traffic.rs:77, 92, 108 open in the imperative ("Freeze", "Adopt", "Count", "Reset") where every other doc in the partition is descriptive; (h) hull_traffic.rs:16-17's link text names `codec::scan` but targets the `crate::codec` module.

Evidence:

       320	    ///
       321	    ///
       322	    /// # Complexity

        12	/// The projection of a [`Version`] by a [`Party`]: `v / &p`.

        89	// ───────────────────────────── event mutation ─────────────────────────────

Resolution: (a) delete one blank; (b) `= a`; (c) "so materialization is kept explicit"; (d) `&v / &p`; (e) rename the section "join and meet identities" or move the header; (f) keep the derivation at `new()` and let `is_empty` say "the 2-bit stream `new()` documents"; (g) "Freezes", "Adopts", "Counts", "Resets"; (h) link to `crate::codec::scan` or write the plain path. Acceptance: each listed site reads as its neighbors do.

### version-core-4: Rustdoc link syntax inside `//` comments is inert
- Where: crates/before/src/version.rs:385-386 (related: crates/before/src/version.rs:429-431, 1349; crates/before/src/version/own.rs:77-78)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read each site; the refutation pass found the fourth at 1349, a `//` block carrying `[`join_all`](Version::join_all)`); executed: no
- Seen by: prose; refutation: confirmed, plus the 1349 site; history: the citations themselves are deliberate (1bb610d19: "every fast path cites its law at the code site"), the markdown form carries no rationale
- Owner-gated: no

Rustdoc never renders `[`text`](path)` in non-doc comments, so the syntax is noise a terminal reader must parse around; the plain path says the same thing and serves the stated intent (cite the law at the code site) equally.

Evidence:

       385	        // Equal operands sit at distance zero — the
       386	        // `distance_to_self_is_zero` law in [`laws`](crate::laws) —

Resolution: write the plain path: "the `distance_to_self_is_zero` law in `crate::laws`", at all four sites. Acceptance: no `](` sequence appears on a `//` (non-doc) line in the partition.

### version-core-5: The join/meet subadditivity lemma is derived only in test prose, though the folds' auxiliary-space bounds and a downstream budget rest on it
- Where: crates/before/src/version.rs:438-445 (related: crates/before/src/version.rs:475, 498-505, 539, 610, 1361, 1374, 1387, 1400; crates/before/src/version/tests.rs:110-119, 137-146, 481-528; crates/before/src/meter/tier2/tests.rs:328-346; crates/before/src/fold.rs:1-16; src/tree/mirror/streaming/window.rs:326-329; crates/before/fuelscape/version_join.json)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (`grep -rn -i subadditiv` over crates/before/src and src excluding test files: the only production hits are a code comment at grow.rs:511 and the rumors consumer at window.rs:328; the derivation at tier2/tests.rs:328-346 and the lemma statement at tests.rs:110-119 read in full; the join island contract reads `O(|self| + |other|)` time only); executed: no
- Seen by: correctness ([41], from the fold's space bound), claims ([48], from the join/meet contract); refutation: both confirmed; history: already-known (the amplification note records the lemma of record, `size(c) ≤ size(a) + size(b) − 2`, its tier2 pins, and an owner ruling of 2026-07-23 that the bound stays unless falsified; no ruling addresses where a production-side statement lives)
- Owner-gated: yes (adds a sentence to the public contract of API-stable doors)

Two consequences of one gap. First, the n-ary folds promise `O(|self| + |iter|)` auxiliary space, and the balanced counter (fold.rs:41-80) holds up to `log k` merged groups over disjoint input subsets, so the bound holds only if each merged group's encoding is at most the sum of its inputs' encodings, which is exactly the subadditivity lemma; that argument appears nowhere a reader of the fold can find it, only in test doc comments ("Callers that track version-size maxima rely on this"). Second, rumors's mirror window budgets on the byte-level corollary ("before's pinned join- and meet-subadditivity lemmas"), yet the public rustdoc of `join`/`meet` and the operator matrices state time complexity only, so under the crate's own framing (documented claims are hard guarantees) the property a consumer relies on is promised by no door. The standard asks that each hard-guarantee claim carry (a) an argument, (b) a matching implementation, and (c) a committed instrument; (a) is missing for the space bound, and the lemma itself has (b) and (c) without a public (a).

Evidence:

       438	    /// The join (least upper bound) of this [`Version`] and `other`: their
       439	    /// combined causal history.
       440	    ///
       441	    /// Identical to the operator form `self | other`.
       442	    ///
       443	    /// # Complexity
       444	    ///
       445	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join.html"))]

       475	    /// Auxiliary space is `O(|self| + |iter|)`.

Resolution: state the lemma once in production prose, positively, with the derivation's one-line shape (output boundaries lie in the union of input boundaries; pointwise max/min is 1-Lipschitz in each operand; zigzag-gamma length depends only on magnitude; the shared root and unmatched first-leaf code give the `−2`): in the `# Complexity` sections of `Version::join` and `Version::meet` (the operator matrices' `$opdoc` can cite them), and cited from `balanced_fold`'s doc or the `crate::fold` module doc as the reason a merged group cannot outgrow its inputs. Point the four `*_encoding_is_subadditive*` tests and the tier2 pins at the stated lemma by name. Acceptance: `grep -rn -i subadditiv crates/before/src/version.rs` matches the public rustdoc of `join` and `meet`; the aux-space lines read as consequences of a stated lemma; the tier2 pins are unchanged; `just readme` regenerates cleanly.

Construction: no committed artifact fails today, which is the gap. Before writing the lemma down, search for a counterexample: extend `join_encoding_is_subadditive_arbitrary` and its meet dual with a deeper generator than `arb_oracle_version`'s depth and with the meter families' deep spines and comb parties, asserting `encode(a|b).len() <= encode(a).len() + encode(b).len()`. If none appears, the fix is prose; if one appears, the aux-space claim needs a corrected bound.

### version-core-6: Ghost name `alice` in the `span` example
- Where: crates/before/src/version.rs:581-581
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -i alice` in version.rs: this one line; the example binds `a`, `b`, `va`, `va2`, `vb`); executed: no
- Seen by: prose, correctness; refutation: confirmed; history: slip from origin (2d3c232f0 wrote `a`/`b` bindings and the `alice` comment together)
- Owner-gated: no

No ghost references: the comment names a binding that does not exist in the snippet or the file, so the reader hunts for it.

Evidence:

       581	    /// let vb = b.tick().clone(); // concurrent to alice's line

Resolution: `// concurrent to a's line`. Acceptance: every name in the example's comments is bound in the example.

### version-core-7: `span_all` advises against an operation that does not exist and links `span` to `Version::meet`
- Where: crates/before/src/version.rs:603-604 (related: crates/before/src/version.rs:526-527, 638-651)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'Version::meet)ing'` matches only 603; `git blame -L 603,604` attributes both lines to b3f09baa0, the docs-pass commit; 526-527 is the `meet_all` sentence it was copied from); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed; history: slip (copy of `meet_all`'s sentence with the verb swapped and the link target left)
- Owner-gated: no

`span` returns a `Span`, not a `Version`, so there is no "iteratively spanning versions one-at-a-time" for a caller to prefer this over, and the link text `span` lands on `Version::meet`. Public rustdoc serves the library user; a comparison target that cannot be written plus a wrong link cost the reader the contract they came for.

Evidence:

       603	    /// Prefer this to iteratively [`span`](Version::meet)ing [`Version`]s
       604	    /// one-at-a-time, as it is more efficient.

Resolution: name the true alternative, which 638-651 already describes: "Prefer this to computing [`meet_all`](Self::meet_all) and [`join_all`](Self::join_all) separately: one balanced fold carries both endpoints and reads each input once." Drop the `Version::meet` link. Acceptance: the sentence names an operation a caller could write instead, and every intra-doc link in the `span_all` doc resolves to the item its text names.

### version-core-8: `span_all` re-implements `balanced_fold`'s counter dispatch; a third copy lives in span algebra
- Where: crates/before/src/version.rs:652-698 (related: crates/before/src/version.rs:806-827, 1230-1270, 1310-1331; crates/before/src/span/algebra.rs:369-424, 430-443; crates/before/src/fold.rs:1-3, 91-104)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (all three copies read: version.rs:675-679 and 817-821 are word-for-word identical; span/algebra.rs:400-404 differs only in "every leg kernel"; both `DedupRuns::new` call sites pass a `Borrow<Version>` projection, 805 `Borrow::borrow` and 652 `FoldInput::version`, whose `Borrow` impl at 1312-1316 is the same function); executed: no
- Seen by: structure; refutation: confirmed, severity medium to low (the span-algebra copy carries a `points` fast path at 375-377 that a shared fold must accommodate, so "one copy in the crate" over-reaches); history: the two-sided accumulator is deliberate (2d3c232f0), but no commit weighs duplicating the dispatch rather than generalizing the fold; the fold-unification survey records the n-ary reduction as "already owned", the standard these copies fall short of
- Owner-gated: no

fold.rs names itself "the one home for the counter discipline, so a hardening of the fold shape reaches every fold at once", yet the input-versus-merged dispatch that sits on top of it is written three times, so a change to the lone-input or weight-0 reasoning must be made in three places. `DedupRuns<I, F>`'s projection `F` duplicates the `Borrow<Version>` bound both call sites already satisfy. This composes with version-core-11: once the `_view` forms collapse, `balanced_fold` loses its `view` parameter and generalizing it over the accumulator is a smaller step.

Evidence:

       652	        let inputs = DedupRuns::new(self.with_items(iter), FoldInput::version).map(Hull::Input);
       ...
       675	                // Unreachable through the counter's weight discipline (a
       676	                // weight-0 lone input never sits below a merged group in the
       677	                // closing drain), but the match stays total rather than
       678	                // asserting: both sides' combiners are commutative, so folding
       679	                // the raw input into the owned hull is value-identical.

Resolution: generalize `balanced_fold` over the accumulator `M` with a small ops record (`lone: fn(&Version) -> M`, `leaf: fn(&Version, &Version) -> M`, `absorb: fn(&mut M, &Version)`, `merge: fn(&mut M, M)`), instantiated with `M = Version` by `join_all`/`meet_all`/`Sum` and with the `(lo, hi)` pair by `span_all`; `Hull` dissolves into `Group<B, M>`; `DedupRuns<I>` drops `F` for `I::Item: Borrow<Version>`. Span algebra can adopt the same fold with a `points` pre-check hook. Acceptance: the weight-discipline comment appears once in version.rs; `fold_clone_collapse_is_value_invisible`, `boundary_arity_fan_folds_match_the_sequential_fold`, and the `VERSION_LIST` fold laws stay green; `DedupRuns` has no function-typed field.

### version-core-9: `shape`'s closing paragraph argues from "realistically reachable" versions, which the same file's doctest refutes
- Where: crates/before/src/version.rs:742-748 (related: crates/before/src/version.rs:205-208; crates/before/src/version/tests.rs:441)
- Class / severity / confidence: claim / low / high
- Provenance: verified (version.rs:205-208 parses `"100000000000000000000000000"` into a `Ticks` and ticks a version by it in the public doctest; tests.rs:441 parses a leaf above `u64::MAX`); executed: no
- Seen by: prose, claims; refutation: confirmed; history: deliberate-and-holds as owner-authored prose (a5f06cad, 2026-08-19, replaced "Typical shapes sit far from that bound." with the current text)
- Owner-gated: yes (owner-authored substance; the likelihood framing is his call)

The paragraph prices the caller's own `Ticks` arithmetic (fine, in one sentence), then dismisses the quadratic worst case on the ground that reachable versions have machine-word `Ticks`; the crate's own `ticks` example builds a version whose one rise is 87 bits wide from the public API, and the crate's contract disclaims likelihood arguments ("the likelihood of an input carries no weight"). The grammatical subject of "are therefore effectively constant-time" is `[Version]s`, not the additions.

Evidence:

       742	    /// Arithmetic *you* do with the [`Ticks`] is priced separately. [`Ticks`]
       743	    /// addition costs the operands' widths, so folding the rises into a
       744	    /// running absolute height can cost each step the running value's full width,
       745	    /// inherently quadratic over the drain in the worst case. Typically, this is
       746	    /// not an issue, however, because realistically reachable [`Version`]s have
       747	    /// [`Ticks`] which are bounded by a machine word, and are therefore effectively
       748	    /// constant-time.

Resolution: state the price as a function of the input and stop: "Arithmetic you do with the yielded [`Ticks`] is not included: summing rises into a running height costs the running value's width per step, `O(1)` while every height fits a machine word and up to quadratic in the encoded size when heights are wide; the walk itself stays linear regardless." Acceptance: the paragraph names no likelihood, its subject agrees with its predicate, and the island's `O(|self|) to drain` contract is unchanged.

Construction: `let mut v = Version::new(); v.ticks(&Party::seed(), "100000000000000000000000000".parse::<Ticks>().unwrap());` yields a version whose one rise is 87 bits wide, refuting "bounded by a machine word" from the public API alone.

### version-core-10: `join_view`'s doc calls the byte-compare rung `O(1)`; the codec's own ladder prices it by the shared prefix
- Where: crates/before/src/version.rs:856-858 (related: crates/before/src/version.rs:385-390, 908-911; crates/before/src/codec/bits.rs:22-26, 414-420)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (bits.rs:419 is `a.ptr_eq(b) || a.as_raw_slice() == b.as_raw_slice()`; bits.rs:22-24 prices a miss as "an early-exiting byte compare over the operands' shared prefix"; version.rs:390 prices the same rung correctly for `distance`); executed: no
- Seen by: prose, claims; refutation: confirmed, with the refinement that slice `==` compares lengths first, so a miss on different-length streams is `O(1)` and the `O(min(|a|,|b|))` case is equal-length streams sharing a long prefix; history: the sentence entered in 58a37d80d when the doors became private methods; an inaccuracy, not a decision
- Owner-gated: no

A private complexity statement that contradicts the module it delegates to is a maintainer trap in a crate that treats complexity claims as hard guarantees; the `distance` comment twenty lines away already has it right, so the two should agree.

Evidence:

       856	    /// Before the merge sweep, two `O(1)` short-circuits settle the cases
       857	    /// canonical form makes immediate: trivial equality (`a ∨ a = a`, a no-op,
       858	    /// decided by a byte compare of the two unique streams) and the lattice

Resolution: "two short-circuits: canonical equality (clone identity in `O(1)`, else one early-exiting byte compare bounded by the shorter operand and absorbed by the sweep it precedes) and the `O(1)` lattice identity `0 ∨ v = v`"; `meet_view` (908-911) inherits the wording through "The dual short-circuits apply". Acceptance: the `join_view`/`meet_view` docs no longer call the byte compare `O(1)`, and their pricing matches bits.rs and the `distance`/`lag` comments.

### version-core-11: The `_view` join/meet doors, the three-strategy operator macro, and `balanced_fold`'s `view` parameter are Batch-era machinery: every caller passes a `Version`
- Where: crates/before/src/version.rs:864-900 (related: crates/before/src/version.rs:784-791, 796-832, 902-945, 1513-1616; crates/before/src/span/algebra.rs:383-410, 430-443, 481-514, 575-633; crates/before/src/causally/conjunction.rs:41-98; crates/before/tests/meter.rs:10757-10776; crates/before/src/meter/board/coverage.rs:8-10)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (grep of `join_view|meet_view|join_refs|meet_refs|span_refs` over crates/before/{src,tests,benches,examples,fuzz,fuzzfit,wasm32-pins,surfacecheck} and src: every `_view` call passes a `Version`'s `.view()`, in version.rs at 494, 558, 660-683, 810-823, 1364, 1377, and the `binop_matrix!` cells via `r.view()`, and in span/algebra.rs via `b.lo().view()`/`b.hi().view()` where `Span::lo`/`hi` return `&Version` (span.rs:522, 542); `causally/conjunction.rs` uses only the `_refs` forms; the four ladder bodies read side by side); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (58a37d80d removed `Batch`, whose `Batch::join_view(&mut self, incoming: &Bits)` needed a non-`Version` stream, and kept the doors as private methods; 9a6bb5fd7 then added `join_refs`/`meet_refs` as "arm-for-arm mirrors" and re-justified the pair at 784-791 as "the two forms ownership demands")
- Owner-gated: no (private surface; the roster edits it forces are named below)

`join_view`/`meet_view` take `&codec::Bits` so a foreign handle's stream could be folded in, and no such handle exists any more. Their bodies duplicate the `_refs` ladders rung for rung (the general paths are the same emitter call, 878 versus 899), aligned only by prose stated three times ("keep the two in lockstep" at 885-886, 933; "keep the three in lockstep" at 951-952, which version-core-12 shows is already false). What the in-place form buys over `*self = Self::join_refs(self, other)` is one refcount increment/decrement pair on the three trivial rungs; the stated "ownership demand" is `O(1)` refcount traffic, not stream bytes, and nothing committed pins it (the scan pins at tests/meter.rs:10757-10776 measure rung liveness, which the same rung code keeps). That duality then justifies the `own`/`clone`/`assign` `@cell` arms of `binop_matrix!` (1536-1584), the `view: fn(&mut Version, &codec::Bits)` parameter of `balanced_fold` (799), and `SpanFoldOps::{lo_view, hi_view}` (span/algebra.rs:436-438). Principle 3: machinery outlives the constraint that justified it, and a convention held in prose is not a check.

Evidence:

       864	    pub(crate) fn join_view(&mut self, incoming: &codec::Bits) {
       ...
       878	        *self = Version::from_bits(skyline::emit::join(self.0.live(), incoming.live()));
       ...
       884	    /// [`join_view`](Self::join_view) for the case where neither operand is
       885	    /// owned — the same short-circuits in the same order (keep the two in
       886	    /// lockstep), with each hand-back arm cloning the operand that is itself
       887	    /// the answer.
       ...
       899	        Version::from_bits(skyline::emit::join(a.0.live(), b.0.live()))

Resolution: make `join_view`/`meet_view` take `&Version` and be one-liners (`*self = Self::join_refs(self, other)`), or delete them and write the assignment at the call sites. Collapse `binop_matrix!` to `span_matrix!`'s shape: one arm for the four value cells using `Borrow::borrow` plus one `*Assign` arm. Drop the `view` parameter from `balanced_fold` (the `Merged × Input` arm becomes `a = refs(&a, b.borrow())`). In span/algebra.rs, `SpanFoldOps` loses `lo_view`/`hi_view` and the `_core` kernels call the `_refs` forms. Delete the three "lockstep" sentences and re-word 784-791. Rename the `join_view_*`/`meet_view_*` rows at tests/meter.rs:10757-10776 (they keep pinning the same rungs through `|=`/`&=`) and re-state coverage.rs:8-10's "the same `join_view`/`meet_view` emitters". Acceptance: `just gate` green; `empty_operands_answer_without_a_walk`'s scan pins unchanged; `grep -rn lockstep crates/before/src` returns nothing; `binop_matrix!` has one value-cell arm and one assign arm.

### version-core-12: `span_refs`'s doc claims its rungs match `join_refs`'s order; `join_refs` tests the empty rungs the other way round
- Where: crates/before/src/version.rs:950-952 (related: crates/before/src/version.rs:868-871, 889-899, 916-919, 934-945, 968-983)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the five ladders: `join_refs` tests `b` empty at 893 before `a` at 896; `meet_refs` at 938/941 and `span_refs` at 974/979 test `a` first; `join_view` at 868/871 mirrors `join_refs` and `meet_view` at 916/919 mirrors `meet_refs`); executed: no
- Seen by: prose; refutation: confirmed (value-irrelevant: after the equal rung at most one operand is empty); history: the claim was false from its first commit (70eb67ab1); the join/meet asymmetry has a readable logic (each `_view` tests its no-op rung first, and each `_refs` mirrors its own `_view`), so aligning `join_refs` would break the join_view/join_refs lockstep
- Owner-gated: no

A maintenance directive that the code already violates teaches the next reader to distrust the others; the fix that keeps every stated invariant is to restate the doc, not to reorder the rungs. (If version-core-11 lands, the `_view` forms vanish and the three `_refs` ladders can simply share one order.)

Evidence:

       950	    /// The first three rungs are [`meet_refs`](Self::meet_refs) and
       951	    /// [`join_refs`](Self::join_refs)'s in the same order — keep the three in
       952	    /// lockstep — each settling both endpoints at once, and the equal rung's

Resolution: "the same three rungs; after the equal rung at most one operand is empty, so the two empty rungs' order is free", and drop "keep the three in lockstep". Acceptance: no doc in the file claims an order the three `_refs` functions do not share.

### version-core-13: The `debug_assert!` on `hull.relation` is the only production reader of `emit::Hull.relation`, and the differential it re-runs is committed
- Where: crates/before/src/version.rs:1001-1007 (related: crates/before/src/version/skyline/emit.rs:146-160, 177-184, 265-270; crates/before/src/version/skyline/emit/tests.rs:65, 296, 464)
- Class / severity / confidence: vestigial / low / medium
- Provenance: verified (`grep -rn '\.relation\b'` over crates/before/src: production readers of `Hull.relation` are version.rs:1005 and its construction at emit.rs:267; the other hits are `fill.rs`'s `Relation` enum, a different type; emit/tests.rs pins `hulled.relation` against the oracle at three sites); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-and-holds (e4d4817f1 added field and assert together, rationale stated at both sites); the field's other stated purpose, "later comparable-pair fast paths and equality dedup" (70eb67ab1), never landed because the ladder classifies before emitting
- Owner-gated: yes (removal of an instrument)

The rationale is stated at the site, but under the doctrine's own test it does not hold: an assert must name a concrete, constructible failure the committed tests cannot catch, and "for deterministic pure functions, runtime recompute-and-compare asserts are not defense-in-depth" once differential coverage exists. `causal_cmp` and `emit::hull`'s folded relation are both deterministic functions of the two streams, each differentially pinned against the oracle (the verdict matrix; emit/tests.rs on worked, exhaustive, and proptest populations). The cost is not the assert (compiled away in release) but the surviving-directions fold inside every hull emission, whose only consumer is this debug-build assert: emit.rs:183-184 names the assert as the field's purpose, and the assert checks the field, which is the circular-justification tell.

Evidence:

      1001	        // The fused walk folds the pair relation beside its emissions (an O(1)
      1002	        // flag pair riding sign reads the walk performs anyway), so the
      1003	        // ladder's classification is cross-checked at the only door that emits.
      1004	        debug_assert!(
      1005	            hull.relation.is_none(),
      1006	            "the comparison rung admits only concurrent pairs to the emitting walk"
      1007	        );

Resolution: drop the `debug_assert!` and its comment. Then decide (skyline partition) whether `Hull.relation` and the directions fold in `emit::hull` keep a consumer; if the comparable-pair fast path is not planned, remove the field, the fold, and the three `hulled.relation` assertions in emit/tests.rs, and re-state the emit doc. If a consumer is planned, replace the assert with that consumer. Acceptance: `grep -rn 'hull.relation\|hulled.relation' crates/before/src` returns only emit-internal hits, or none; `identity_fast_paths_agree_across_buffer_identity` and the `span_is_the_pair_hull` law stay green.

Construction: to confirm redundancy rather than a defect, mutate `emit::hull` to return `relation: Some(Ordering::Equal)` unconditionally: emit/tests.rs fails on the oracle differential before any debug build of `span_refs` runs.

### version-core-14: `Version::decode` and the text/literal constructors have no `# Errors` section; `Rank::decode` and `Ranked::decode` carry itemized ones
- Where: crates/before/src/version.rs:1095-1110 (related: crates/before/src/version.rs:1440-1459, 1461-1478, 1480-1511; crates/before/src/version/ticks.rs:166-190, 199-213; crates/before/src/version/rank.rs:435-445; crates/before/src/version/ranked.rs:241; crates/before/src/party.rs:605-623, out of partition)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '# Errors' crates/before/src`: 13 sites in clock.rs, span.rs, party.rs, span/wire.rs, ranked.rs, rank.rs, admit.rs, none in version.rs, ticks.rs, or own.rs; rank.rs:435-445 itemizes `Truncated`/`TrailingBits`/`NotCanonical`/`Io` with triggers; literal.rs:18 `leaf` returns `BitsBuf`, so `TryFrom<u64>` at 1473-1478 never fails); executed: no
- Seen by: prose; refutation: confirmed; history: accretion across two authoring eras (the section entered with the rank wire form, f0f3a2aed; version.rs never carried one)
- Owner-gated: no

Documentation altitude: hazards belong under uniform `# Panics`/`# Errors` sections so a user finds every return arm; the crate adopted that convention for the rank doors, so the version doors' omission reads as a gap. `FromStr for Version`, `TryFrom<(u64, T, S)>`, `FromStr for Ticks`, and `TryFrom<&Ticks> for u64` state rejections in running prose or not at all, and `TryFrom<u64> for Version` is spelled fallible while never failing, with no note saying so.

Evidence:

      1095	    /// Decodes a [`Version`] from a reader of canonical bytes.
      1096	    ///
      1097	    /// # Complexity
      1098	    ///
      1099	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_decode.html"))]
      1100	    ///
      1101	    /// Strict validation is one pass over the stream, and the result reuses the read buffer.

Resolution: add `# Errors` to `Version::decode` mirroring `Rank::decode`'s shape (`Decode::Truncated`, `Decode::NotCanonical`, `Decode::TrailingBits`, `Decode::Io`, each with its trigger drawn from `validate_prefix` and `require_marker_padding`); `# Errors` naming `Parse::Syntax`/`Parse::NotCanonical` on the two `FromStr` impls and the node literal; on `TryFrom<u64>`, "Never fails; the fallible spelling lets leaves compose with node literals, whose `TryFrom<T, Error = Parse>` bound it satisfies." Acceptance: every fallible public entry in version.rs, own.rs, and ticks.rs has an `# Errors` section naming each variant it can return, and the infallible `TryFrom<u64>` says so.

### version-core-15: Stored versions retain their build buffer's pre-size capacity; the tick and hull paths have no resident reading and no committed test pins retained bytes against encoded bytes
- Where: crates/before/src/version.rs:1192-1201 (related: crates/before/src/codec/bits.rs:107-125; crates/before/src/codec/buf.rs:121-125; crates/before/src/codec/build.rs:66-68, 233-240; crates/before/src/version/skyline/emit.rs:223, 228, 303; crates/before/src/version/skyline/query.rs:517, 603-607; crates/before/src/version/skyline/grow.rs:513; crates/before/src/version/skyline/fill.rs:251; crates/before/benches/presize.rs:1-14, 154-204; justfile:792-809; crates/before/tests/meter.rs:360-372; crates/before/src/version/tests.rs:2185-2201)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified for the absence (grep of `presize.rs` rows: `projection`, `projection_decoded`, `projection_outgrow`, `merge`, `display`, `parse`; no tick or hull row; `bench-alloc-ab` arms are `shipped|projection_growth|projection_shrink|display_growth`); assessed for the mechanism (read: `SkylineBuilder::with_capacity` reaches `Vec::with_capacity(cap / 8 + 1)` at build.rs:68; `finish` adopts the bytes without shrinking at 233-240; `Bits::freeze` is `Bytes::from(buf.into_bytes())`; bytes 1.11.1, the pinned version, `From<Vec<u8>>` at src/bytes.rs:960-977 boxes the slice only when `len == cap` and otherwise keeps `cap` in `Shared`; `fill.rs:251` pre-sizes a tick to exactly `event.len()`, so a tick that spills into a new byte pays `Vec` doubling); executed: no
- Seen by: claims; refutation: reframed (the "invisible to every committed meter" claim is refuted: query.rs:603-607 states the stranded-slack trade, `benches/presize.rs` is a committed allocation-strategy record with a `resident_report`, and the peak envelopes keep the result alive at measurement), severity medium to low; history: pre-sizing is owner-ratified (2026-07-27, "RATIFIED as the stated-band residual", for the projection's doubling chain); nothing in any note addresses at-rest retention on the tick or hull paths
- Owner-gated: no (measure first; a bench row or test is additive)

A stored `Version`'s heap footprint is its build buffer's final capacity, not its encoding: a meet of disjoint supports retains `|a| + |b|` bits behind a one-byte value, a join of comparable operands retains the dominated operand's size, and a tick that spills a byte past `event.len()` retains roughly twice its encoding. The crate knows this and records it at bench level for projection, merge, display, and parse, but the two paths a long-lived consumer (rumors's tree memos, mostly tick outputs and bounds hulls) exercises most have no resident row, and `at_rest_size_is_one_container_per_stream` pins only the 32-byte handle. Under Principle 2 this is a measure-first item: the sign on residency is fixed (shrinking can only reduce it), the sign on time is workload-dependent (one memcpy at freeze), so a reading precedes any cure.

Evidence:

      1192	    /// Freeze a normal-form skyline bit stream as a `Version`, canonicalizing
      1193	    /// its storage. The single build-side gate every built/parsed `Version`
      1194	    /// passes through.
      ...
      1199	    pub(crate) fn from_bits(bits: codec::BitsBuf) -> Self {
      1200	        Version(codec::Bits::freeze(bits))
      1201	    }

Resolution: add `tick` (dense version, one tick that spills a byte) and `hull` (meet of disjoint supports; join of comparable operands) rows to `presize.rs`'s `resident_report`, and one committed test that reads live allocator bytes after the operation returns minus `as_bytes().len()` on those constructed shapes, pinning a ceiling with slack and a floor at the encoding. If the reading is material for the consumer's workload, shrink at the one gate (`Bits::freeze`: `into_boxed_slice` before `Bytes::from`, so `len == cap` takes bytes' no-`Shared` path) or size the emitters to the subadditivity bound minus the known collapse. Acceptance: a committed reading exists for the tick and hull paths; if the cure lands, retained bytes equal the encoded length plus the fixed `Bytes` header on every shape, and the peak-heap envelopes move only where the memcpy adds to peak.

Construction: under the meter suite's counting allocator (`HEAP.current_usage()`), build `a` as a deep left spine and `b` as a deep right spine of about N bytes each (disjoint supports), take `let m = &a & &b;` (`m.as_bytes().len() == 1`), and read live bytes after the call returns: expect about `(|a| + |b|) / 8` retained. Dual: `let mut v = dense(N); v.tick(&Party::seed());` and compare live bytes to `v.as_bytes().len()`; a doubling past the `event.len()` pre-size reads about 2x.

### version-core-16: The tuple-literal `TryFrom` states `O(m)` but each nesting level re-scans its children
- Where: crates/before/src/version.rs:1486-1488 (related: crates/before/src/version/skyline/literal.rs:32-34; crates/before/src/version/tests.rs:394-400)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read literal.rs:33-34: `scan(left)` and `scan(right)` collect every leaf height at each level, and the children are re-emitted; the test doc at tests.rs:396-397 describes the re-derivation); executed: no
- Seen by: claims; refutation: confirmed; history: the `O(m)` line entered in 3bba6cbbc with no pricing of the depth factor
- Owner-gated: no (a doc correction toward the code)

A literal nested `d` levels deep scans the innermost stream `d` times, so the cost is `O(m · d)`; `d` is a compile-time property of the tuple type, so no caller input reaches it, but a public `# Complexity` section is a contract and this one hides the factor the test doc describes.

Evidence:

      1486	    /// # Complexity
      1487	    ///
      1488	    /// `O(m)`, with `m` the built version's size in bytes.

Resolution: state `O(m · d)` with `d` the literal's nesting depth (each level re-derives its children's absolute heights from their streams), or restructure `literal::node` to compose without re-scanning. Acceptance: the doc names the depth factor, or `literal::node` no longer re-scans and `descending_literals_build_the_oracle_tree` stays green.

Construction: a 20-deep right-nested literal of `u64` leaves scans the innermost leaf's stream 20 times; under the scan meter the reading is about the sum of subtree sizes rather than `m`.

### version-core-17: Register and vocabulary rules that postdate the prose: `mints`, moralized and dated adjectives, unanchored `door`, em-dashes in `//` comments
- Where: crates/before/src/version.rs:1624-1625 (related: `door`: version.rs:1003, hull_traffic.rs:13, tests.rs:1353, 1357, 1516, 1590, 1641, 1644, 1669, 1681; `honest`/`historical`/`truthful`/`real`: tests.rs:1107, 1349, 1355, 1392-1393, ticks/tests.rs:84; em-dashes on `//` lines: version.rs 25, 123, 156, 385, 386, 429, 430, 640, 646, 1122, 1351, 1622, 1717; own.rs 77, 78; own/tests.rs 40, 62; tests.rs 454, 455, 536, 788, 991, 1005, 1157, 1165, 1204, 1349, 1350, 1355, 1651, 1669, 1671, 1799, 1869, 1902)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep per file: `mint` once in the partition (version.rs:1624) and 43 times in crates/before/src; `honest`/`historical`/`truthful` 4 lines in the partition and `honest` 146 crate-wide; `door(s)` 10 lines in the partition and 208 crate-wide, with no definition in lib.rs, any AGENTS.md, or validation_index.rs; em-dashes on `//` (non-doc) lines 35 in the partition and 374 across 76 files crate-wide; no assert, expect, or panic message in the partition carries an em-dash); executed: no
- Seen by: prose ([17], [29], [30], [31]); refutation: [17], [29], [31] confirmed, [30] reframed as crate-wide; history: each rule entered the owner's chezmoi-managed doctrine on 2026-08-10 (55281a3), after most of this prose; `door` is listed in the writing-style lexicon as the plain-term case "entry point"
- Owner-gated: yes (a crate-wide sweep, and `door` needs a ruling: define once or replace)

Four tells the review standard names, all real in the partition, none partition-local: "mint" for constructing a value is banned outright; "honestly reachable", "honest coordinate", "real capacity", "real groups" moralize code and "the historical numerator path" dates it (the file's own present-tense name is "backend-arm", 1362, 1414); "door" is a metaphor promoted to jargon with no definition site; and the owner's comment-register rule asks for colons or spaced double-hyphens in `//` comments, with true em-dashes reserved for rendered prose. Partition-local fixes would leave the crate inconsistent, so the disposition is one mechanical sweep plus one ruling on `door`.

Evidence:

      1622	// operand type — a `Span`, not a `Version` — so the family has no assigning
      1623	// form (nothing of the receiver's type to assign back) and no owned-operand
      1624	// strategy: every cell reads both operands in place and mints the endpoints
      1625	// owned, exactly as the named method does.

      1349	// The wide arm is honestly reachable only past the backend's capacity —

Resolution: (1) "returns the endpoints owned"; (2) "reachable only past the backend's capacity", "the production coordinate is 2⁶⁴ − 64 bits", "whose capacity is astronomically higher", "the backend-arm path", "the exact-size length stays exact", "half a GiB of groups"; (3) define `door` once (lib.rs or AGENTS.md: a public method or trait impl through which a value enters or leaves the crate) or replace per site with "public method" / "codec entry" / "fold"; (4) on `//` lines only, replace ` — ` with `: ` or `; ` where it introduces an apposition and with a parenthetical or a new sentence where it brackets one. Book all four as one crate-wide pass. Acceptance: `grep -n -i 'mint\|honest\|historical\|truthful'` over the partition returns nothing; a single definition site for `door` exists or the term is gone; `grep` for `—` on lines matching `^\s*//[^/!]` over the partition returns nothing.

### version-core-18: "so it is not monotone under `<=`" reads as a claim about projection, which the same paragraph calls a lattice homomorphism
- Where: crates/before/src/version.rs:1675-1676 (related: crates/before/src/version.rs:1669-1674; crates/before/src/version/tests.rs:2161-2166)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites; 1673-1674 state projection is "a homomorphism of both join and meet", hence monotone); executed: no
- Seen by: prose; refutation: confirmed; history: both sentences written in 472d646e2 with the same construction; a grammatical slip
- Owner-gated: no

The grammatical subject of "it is not monotone" is "Projection", so the sentence asserts a falsehood two lines after the mathematics that refutes it; the intended claim is that `min_ticks` is not monotone under `<=` (a sub-version can have a larger floor). The test doc at tests.rs:2161 ("Projection can *raise* `min_ticks`: it is not monotone under `<=`.") has the same construction, and test docs are held to "must be accurate".

Evidence:

      1675	// Projection can still raise `min_ticks` (carving one broad tick into
      1676	// disjoint peaks), so it is not monotone under `<=`.

Resolution: name the subject: "Projection can still raise `min_ticks` (carving one broad tick into disjoint peaks): `min_ticks` is not monotone under `<=`, though projection itself is." Same edit at tests.rs:2161. Acceptance: neither sentence can be read as "projection is not monotone".

### version-core-19: `Div<&Party> for &Version` lives in version.rs and constructs `OwnVersion` through `pub(crate)` fields
- Where: crates/before/src/version.rs:1703-1711 (related: crates/before/src/version.rs:1660-1676; crates/before/src/version/own.rs:1, 47-53)
- Class / severity / confidence: modularity / nit / medium
- Provenance: verified (`grep -rn 'OwnVersion {' crates/before/src`: exactly one construction, version.rs:1706); executed: no
- Seen by: structure; refutation: confirmed; history: 6c88c2ad0 placed the `Div` impl beside the other operator matrices from the start; the bottom of version.rs groups the matrices (`|`/`&` at 1513, `^` at 1618, `/` at 1660, comparison at 1713), a visible organizing principle no comment states
- Owner-gated: no

own.rs opens with "the lazy projection view `&v / &p`" and holds the view's whole comparison matrix, but the `/` impl that builds the view sits in version.rs, which is why `OwnVersion::{party, version}` are `pub(crate)` rather than private: visibility wider than use, for one constructor that belongs beside the type. The competing principle (all operator matrices in one place) is real but unstated.

Evidence:

      1703	impl<'a> Div<&'a Party> for &'a Version {
      1704	    type Output = OwnVersion<'a>;
      1705	    fn div(self, party: &'a Party) -> OwnVersion<'a> {
      1706	        OwnVersion {
      1707	            party,
      1708	            version: self,
      1709	        }
      1710	    }
      1711	}

Resolution: move the `Div` impl and its section comment (1660-1676) into own.rs and make the fields private; or keep the grouping and state it once at the head of the matrices section so the `pub(crate)` fields have a stated reason. Acceptance: `OwnVersion` has no `pub(crate)` fields, or the matrices section states why the constructor lives there; `Version::project` still reads `self / party`.

### version-core-20: `causal_cmp_impls!` has one invocation and is the fixed-body twin of `view_cmp_impls!`
- Where: crates/before/src/version.rs:1721-1760 (related: crates/before/src/version/own.rs:189-230)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified (read both macros: the same six-impl fan-out, one with fixed bodies and one invocation, the other parameterized by `$eq`/`$cmp` and a lifetime list); executed: no
- Seen by: structure; refutation: confirmed (`impl<>` is valid Rust, so the empty-lifetime case needs no new arm; the shared macro must sit textually before `mod own;` at version.rs:18); history: `causal_cmp_impls!` predates the crate rename; `view_cmp_impls!` arrived in 6c88c2ad0 as a parameterized copy; no commit weighs unifying them
- Owner-gated: no

Two copies of one fan-out shape: a change to which reference forms the comparison matrix covers must be made twice.

Evidence:

      1758	causal_cmp_impls! {
      1759	    Version, Version;
      1760	}

Resolution: define the body-parameterized fan-out once in version.rs before `mod own;` and invoke it for `Version, Version` with `codec::canonical_eq`/`sweep::causal_cmp` bodies and an empty lifetime list; delete `causal_cmp_impls!`. Acceptance: one comparison fan-out macro in the crate; `compare_matrix_matches_oracle` and `mirror_cells_agree_on_arbitrary_triples` stay green.

### version-core-21: `from_impl_is_to_version`'s doc says "without re-projecting" while the body re-projects
- Where: crates/before/src/version/own/tests.rs:45-55 (related: crates/before/src/version/own.rs:76-89)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (own.rs:82-88 materializes on every call unless the party is the seed; the body calls `Version::from(view)` and `view.to_version()`, two projections); executed: no
- Seen by: prose; refutation: confirmed; history: written in 6c88c2ad0 with the body it still has; no cache was ever present
- Owner-gated: no

Test docs must be accurate; "without re-projecting" promises a cache the code does not have. What the two asserts show is that the `Copy` view remains usable after `From` consumed a copy of it.

Evidence:

        45	/// The `From` impl is `to_version`, and the view is `Copy`: one view can be
        46	/// compared and materialized repeatedly without re-projecting.

Resolution: "and the view is `Copy`: it stays usable after `From` consumes a copy of it." Acceptance: the doc's claim matches what the two asserts demonstrate.

### version-core-22: `Ticks` type doc: unclosed parenthesis, a semicolon splice with a verb-agreement slip, the text-I/O bound stated twice, and "then ascending"
- Where: crates/before/src/version/ticks.rs:18-30 (related: crates/before/src/version/ticks.rs:41-48, 91-92)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the sites); executed: no
- Seen by: structure ([9]), prose ([27]); refutation: confirmed; history: (a) and (d) are the owner's hand edit 925b9973 (2026-08-19), which replaced a balanced parenthesis with the current unclosed one and wrote "then ascending"; (c) is accretion across 56a08f90, 3bba6cbb, and the same hand edit; none deliberate
- Owner-gated: no (syntax slips; the owner authored them, the fix is uncontroversial)

Four prose defects in the first thing a user of `Ticks` reads: (a) line 19 opens "total (" and never closes it, running three clauses together; (b) 26-29 splice with "; and consumed by" and mismatch "each of which take"; (c) 41-43 and 45-48 state the same text-I/O bound with the same reason, the second paragraph alone carrying the space term; (d) 91-92 "least significant first, then ascending" says one thing twice.

Evidence:

        18	/// Event counts have no ceiling, so the count is unbounded rather than any
        19	/// fixed-width integer: every conversion *into* it is total ([`From`] on
        20	/// unsigned machine integers, and every conversion *out* is explicit
        21	/// about width: `TryFrom<&Ticks> for u64` answers the machine-range case
        ...
        26	/// This type is produced by [`Version::min_ticks`](crate::Version::min_ticks);
        27	/// and consumed by [`Version::ticks`](crate::Version::ticks),
        28	/// [`Party::ticks`](crate::Party::ticks), and
        29	/// [`Clock::ticks`](crate::Clock::ticks), each of which take `impl
        ...
        41	/// Construction is `O(1)`; comparison and hashing `O(‖n‖)`; addition `O(‖a‖ +
        42	/// ‖b‖)`, `Sum` `O(N)`; text I/O is superlinear but subquadratic in the count's
        43	/// width (because it requires decimal conversion).
        ...
        91	    /// The count's base-2^64 "digits", i.e. its *limbs*, least significant
        92	    /// first, then ascending.

Resolution: (a) close the parenthesis after "unsigned machine integers)" and start a new sentence for the conversions out; (b) "produced by `min_ticks` and consumed by `Version::ticks`, `Party::ticks`, and `Clock::ticks`, each of which takes `impl Into<Ticks>`"; (c) drop the text-I/O clause from 41-43 and keep 45-48; (d) "least significant first". Acceptance: the doc parses as English with balanced parentheses and states each bound once.

### version-core-23: `Ticks`' `Sum` bound omits the per-summand term and states no amortization argument
- Where: crates/before/src/version/ticks.rs:41-42 (related: crates/before/src/version/ticks.rs:37-39, 275-293)
- Class / severity / confidence: claim / low / high
- Provenance: assessed (ticks.rs:37-39 defines `N` as the summands' total bit width, zero for any number of `Ticks::ZERO`; `Sum` at 276-282 is a fold of `+=` over the iterator, `Θ(k)` iterations; the per-add cost and the carry amortization are the claims lens's reading of dashu-int's `add_large`, not re-read here); executed: no
- Seen by: correctness ([44]), claims ([59]); refutation: both confirmed; history: the clause entered with the Tier-3 complexity lines (669cf3103) from a uniform template; no argument recorded
- Owner-gated: no (a doc correction toward the code)

Every asymptotic claim is a hard guarantee for all inputs; a bound that reads `O(0)` on a nonempty iterator is wrong for the zero-width family, however cheap the miss (the missing term is exactly the per-item constant). And the `O(N)` half does need an argument: a single `+=` can carry through the whole accumulator, so only the binary-counter potential (each carry clears a bit an earlier summand set) brings the total to `O(N + k)`; the other bounds on the line read straight off the implementation and need none.

Evidence:

        41	/// Construction is `O(1)`; comparison and hashing `O(‖n‖)`; addition `O(‖a‖ +
        42	/// ‖b‖)`, `Sum` `O(N)`; text I/O is superlinear but subquadratic in the count's

Resolution: write `Sum` as `O(N + k)` for `k` summands, "amortized: each carry clears bits an earlier summand set", or define `N` as `Σ(1 + ‖nᵢ‖)`. Acceptance: the stated `Sum` bound is nonzero for every nonempty iterator and names its amortization.

Construction: `(0..k).map(|_| Ticks::ZERO).sum::<Ticks>()` has `N = 0` and performs `Θ(k)` iterations.

### version-core-24: `Ticks::limbs` re-derives an exact size and reaches through two newtypes because `suanpan::Limbs` exposes neither `ExactSizeIterator` nor a `Base` accessor
- Where: crates/before/src/version/ticks.rs:112-118 (related: crates/before/src/version/ticks.rs:125-149; crates/suanpan/src/limbs.rs:43-72; crates/before/src/codec/base.rs:26-35; crates/before/src/version/ticks/tests.rs:81-109)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read crates/suanpan/src/limbs.rs in full: `Limbs` wraps `core::slice::Chunks<'a, Word>` and implements `Iterator` and `DoubleEndedIterator` only; `Base` is `pub struct Base(pub(crate) UBig)` at base.rs:26); executed: no
- Seen by: structure; refutation: confirmed; history: the shadow counter arrived with the shape surface (46eb64f9); no note explains recomputing the count over delegating `size_hint`; suanpan's API is unpublished "until a second consumer stabilizes the API"
- Owner-gated: yes (additive trait impls on suanpan's public `Limbs`)

`core::slice::Chunks` is `ExactSizeIterator` and `FusedIterator`, so the chunk count is available for free; before's `Limbs` instead recomputes it as `bits().div_ceil(64)` through a `usize::try_from(..).expect(..)`, decrements a shadow `remaining` in `next`, and implements the two traits over the shadow. Two spellings of one quantity are held equal by a test (`limbs_respell_the_count`, per step) rather than by construction, and `&self.0 .0` bypasses the `Base` newtype at a use site.

Evidence:

       112	    pub fn limbs(&self) -> Limbs<'_> {
       113	        Limbs {
       114	            limbs: suanpan::Limbs::new(&self.0 .0),
       115	            remaining: usize::try_from(self.0.bits().div_ceil(64))
       116	                .expect("a stored count's limb count fits usize"),
       117	        }
       118	    }

Resolution: in suanpan, `impl ExactSizeIterator for Limbs<'_>` (delegating `size_hint` to `self.chunks.size_hint()`) and `impl FusedIterator for Limbs<'_>`; in before, `Limbs { limbs }` forwards `size_hint` and the `remaining` field and `expect` go; add a crate-private `Base::limbs(&self) -> suanpan::Limbs<'_>` so `Ticks::limbs` reads `self.0.limbs()`. Acceptance: `limbs_respell_the_count` stays green; `grep -n remaining crates/before/src/version/ticks.rs` is empty; no `.0 .0` in ticks.rs.

### version-core-25: The `From<u8..u128> for Ticks` impls carry no rustdoc; the doc sits on the macro, and `From<usize>` alone is documented
- Where: crates/before/src/version/ticks.rs:151-164 (related: crates/before/src/version/ticks.rs:192-197)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (the `///` line precedes `macro_rules!`; the expansion at 155-159 carries no `#[doc]`; rustdoc attaches a doc comment to the item it precedes, here the private macro); executed: no (a docs build was not permitted)
- Seen by: prose; refutation: confirmed; history: the macro and its doc line date to 56a08f90; no commit addresses where the doc lands
- Owner-gated: no

The type doc promises `From` on unsigned machine integers is total and `O(1)`, so each impl should say so where the user sees it, as the `usize` one already does.

Evidence:

       151	/// A count from a machine integer: total, `O(1)`.
       152	macro_rules! ticks_from_unsigned {
       153	    ($($t:ty),*) => {
       154	        $(
       155	            impl From<$t> for Ticks {
       ...
       192	/// A count from a machine size: total, `O(1)`.
       193	impl From<usize> for Ticks {

Resolution: move the doc inside the expansion (`$( #[doc = "A count from a machine integer: total, `O(1)`."] impl From<$t> for Ticks { ... } )*`), or fold `usize` into the macro so all six are documented identically. Acceptance: every `From<_> for Ticks` impl shows the same one-line doc in rendered rustdoc.

### version-core-26: `hull_traffic`'s `snapshot` and `reset` enumerate the `Rung` variants by hand; `web_traffic` is a shape-for-shape copy
- Where: crates/before/src/version/hull_traffic.rs:92-102 (related: crates/before/src/version/hull_traffic.rs:67-75, 83-90; crates/before/src/version/skyline/web_traffic.rs:58-98)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both modules: `cell()` is an exhaustive match, `snapshot` and `reset` are hand-listed; web_traffic.rs:58-98 has the identical construction over `Decision`); executed: no
- Seen by: structure; refutation: confirmed, adding that `snapshot` (84-89) shares the hand enumeration; history: hull_traffic from e4d4817f1, web_traffic from 76579a3c, whose message calls the copy "the hull_traffic idiom"; no rationale for the array literal
- Owner-gated: no

Adding a variant makes `cell()` fail to compile but leaves `reset()` compiling and silently skipping the new counter (so it never resets between scenarios) and `snapshot()` compiling without a field for it (so it goes uncounted). Principle 5's "no hand-maintained counts" applied to code: an enumeration the compiler does not check rots silently, and here the rot is a meter that stops resetting. The duplication with `web_traffic` is the crate reimplementing its own small capability twice.

Evidence:

        92	    /// Reset every rung counter to zero.
        93	    pub(crate) fn reset() {
        94	        for rung in [
        95	            super::Rung::Equal,
        96	            super::Rung::Empty,
        97	            super::Rung::Comparable,
        98	            super::Rung::Concurrent,
        99	        ] {
       100	            cell(rung).store(0, Ordering::Relaxed);
       101	        }
       102	    }

Resolution: store the cells as `static CELLS: [AtomicU64; N]` indexed by a `Rung::index()` (or `#[repr(usize)]`), so `reset` is a loop over `&CELLS` and `snapshot` reads by index; or lift a small `Tally<const N: usize>` (record/snapshot/reset over an atomic array) into `codec`/`meter` and have both classified counters use it. Apply the same to web_traffic.rs. Acceptance: no per-variant array literal in either `reset`; adding a `Rung` variant requires touching exactly the enum and the snapshot struct, with a compile error naming the second.

### version-core-27: `version/tests.rs`'s module doc omits half the file, and the `Rank`/`Ranked` suites live two modules away from the types they test
- Where: crates/before/src/version/tests.rs:1-5 (related: crates/before/src/version/tests.rs:344-347, 753, 948, 1347, 1667, 1741, 1865, 2185, 2375; crates/before/src/version/rank.rs; crates/before/src/version/ranked.rs)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -n 'mod tests'` over rank.rs, ranked.rs, rank/*.rs: only rank/num.rs:650; `ls crates/before/src/version/rank/` holds `num` and `num.rs`, no `ranked/` directory exists; `grep -c version/tests.rs tools/covcheck-expected.json` is 0; tools/citecheck resolves `surface.rs` pins by the final `::` segment of a collected test name, so a module move costs nothing there); executed: no
- Seen by: structure ([2]), prose ([24]); refutation: both confirmed (one sub-claim softened: 283-342 are identity/absorbing laws in byte-parity form, so "lattice laws" is partly accurate); history: legacy placement (the rank type was born in version/tests.rs on 2026-06-10; the sibling-file convention entered root AGENTS.md on 2026-07-24); no decision recorded
- Owner-gated: no

The module doc lists five topics; the file's own section headers add `rank` (753), `the rank wire form` (948), `the numerator's wide arm` (1347), `the join fold` (1667), `ranked` (1741), `the composite ranked key` (1865), plus the at-rest size pin and the deep-spine grammar pin. Roughly lines 753-1665 and 1741-2114 are the `Rank` and `Ranked` unit suites, reaching their subjects as `super::Rank` and `super::rank::arm_ceiling::force`; `rank.rs` and `ranked.rs` declare no `mod tests`. The crate convention places unit tests in a sibling of the source they test, and a module doc that omits half the file is stale prose: a reviewer of `rank.rs` has no signpost to where its tests are.

Evidence:

         1	//! Version tests.
         2	//!
         3	//! The causal order and its comparison matrix, the join/meet operator matrices
         4	//! and lattice laws, grow optimality against the brute-force reference,
         5	//! `min_ticks`, and projection (`/`).

Resolution: minimum, rewrite the module doc to map every `// ───` section in the file. Better, move the rank sections to `crates/before/src/version/rank/tests.rs` (the directory exists) with `mod tests;` in rank.rs, and the `Ranked`/composite-key sections to `version/ranked/tests.rs`, taking `stream_rank`/`seeded_rank`/`rank_parts`/`stairs` with their consumers; the `pub(crate)` items they reach (`from_raw`, `raw_parts`, `numerator_is_wide`, `content_bits`, `arm_ceiling`, `BACKEND_CAPACITY_BITS`) need no visibility change. The move crosses into the rank partition's ownership; this note is the pointer. Acceptance: `rank.rs` and `ranked.rs` each have a sibling `tests.rs`, `version/tests.rs`'s module doc names exactly its remaining sections, and `just gate` is green with no new seed files (a moved proptest that had a seed replays from its new path, per `tests/seed_liveness.rs`, or the seed moves with it).

### version-core-28: Two proptests assert the same `|=` cells on the same population
- Where: crates/before/src/version/tests.rs:163-189 (related: crates/before/src/version/tests.rs:215-231, 257-274)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (both bodies read: same generators `world_strategy`, `i`, `j` in `0..64`, same oracle expectation, same two cells `Version |= Version` and `Version |= &Version`; `grep -rn version_assign_join_matches_oracle` over crates/before and tools returns only the test file); executed: no
- Seen by: structure ([3]), prose ([22]), correctness ([47]); refutation: confirmed; history: deliberate-but-expired (at 58a37d80d^ the first test's second block was `Batch |= &Version` and the second covered the `{Version, Batch}²` matrix; 58a37d80d re-scoped both to the surviving cells, leaving them identical)
- Owner-gated: no

A test earns its place by what it catches that nothing else does; two properties over one population asserting the same cells double the run time and the reading load for no coverage. The meet side has only the matrix form, so the pair is asymmetric with its dual as well as redundant.

Evidence:

       171	    fn version_assign_join_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
       ...
       180	        let mut assign = a.clone();
       181	        assign |= b.clone();
       182	        prop_assert!(assign == expected);
       ...
       219	    fn join_assign_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
       ...
       228	        { let mut x = a.clone(); x |= b.clone(); prop_assert!(x == expected); }
       229	        { let mut x = a.clone(); x |= &b; prop_assert!(x == expected); }

Resolution: delete `version_assign_join_matches_oracle` and fold its motivation ("neither of which the by-value `|` differential reaches") into `join_assign_matrix_matches_oracle`'s doc, whose name mirrors the meet dual. Acceptance: one `|=` differential remains beside one `&=` differential; the seed file needs no change (both draw `(ops, i, j)` from `world_strategy`).

### version-core-29: `trace_ticks`'s doc enumerates the ops but omits `Op::Ticks`, which the body counts
- Where: crates/before/src/version/tests.rs:656-672 (related: crates/before/src/testing/optrace.rs:124-132)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read 659-667); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: 8f253e4ef added the arm and left the enumeration untouched
- Owner-gated: no

The doc lists `Tick`, `Send`, `Fork`, `Sync`, `Join` and says "this count is exact — it mirrors `step_impl`", but the match also charges `Op::Ticks(_, k)` its `k`, the op that makes the count non-unit; `min_ticks_floors_every_history` rests on this helper being exact, and a hand-maintained roster that the code has outgrown is the failure in miniature.

Evidence:

       659	/// `Tick` advances once; `Send` advances twice (the sender `tick`s, the
       660	/// receiver `recv`s = join-then-`tick`); `Fork`, `Sync`, and `Join` never
       661	/// `tick`. Each `Tick`/`Send` always executes fully (no index guard can skip
       662	/// it), so this count is exact — it mirrors `step_impl`.
       ...
       667	            Op::Ticks(_, k) => u64::from(*k),

Resolution: add "`Ticks(_, k)` advances `k` times" to the enumeration, or restate structurally ("one unit per `tick` the impl applier performs, mirroring `step_impl` arm for arm") so it cannot rot when an op is added. Acceptance: the doc names every `Op` variant the match charges, or names none individually.

### version-core-30: Hand-maintained pair counts in test names, docs, and a const doc
- Where: crates/before/src/version/tests.rs:840-855 (related: crates/before/src/version/tests.rs:1410-1424; crates/before/src/surface.rs:718, 730, 1173)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (25,000 appears at 841, 845, 853, 855; 10,000 at 1412, 1420, 1424; `surface.rs` cites `rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs` at three sites and citecheck resolves pins by exact final-segment name, so a rename touches surface.rs); executed: no
- Seen by: prose; refutation: confirmed (its roster grep missed surface.rs); history: no discussion of the spelling
- Owner-gated: no

The sweep size is spelled four ways for one sweep and three for the other; changing a bound leaves stale spellings, and a count in a test name is the hardest spelling to keep in step.

Evidence:

       840	/// The order-agreement sweep's fixed PRNG seed: every run replays the
       841	/// same 25,000-pair corpus.
       842	const RANK_CMP_SWEEP_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
       843	
       844	/// The class-first streamed `Rank` order agrees with the alignment oracle on
       845	/// 25,000 adversarial pairs.
       ...
       853	fn rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs() {
       854	    let mut next = crate::testing::rng::word_stream(RANK_CMP_SWEEP_SEED);
       855	    for case in 0..25_000u32 {

Resolution: name the bounds (`const RANK_CMP_SWEEP_PAIRS: u32 = 25_000;` and its wide-arm twin), cite them by name in the docs, and drop the numbers from the test names (`rank_cmp_agrees_with_the_alignment_oracle`, `rank_wide_arm_cmp_agrees_with_the_alignment_oracle`), updating the three `surface.rs` pins. Acceptance: the loop bounds are named constants, no digit string for them appears in a doc or identifier, and `just citecheck` is green.

### version-core-31: The two rank-order sweeps duplicate the perturbed-pair construction verbatim
- Where: crates/before/src/version/tests.rs:857-885 (related: crates/before/src/version/tests.rs:1426-1448)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read both blocks: the same four-arm `match next() % 4`, differing only in the surrounding `assert_rank_canonical` calls); executed: no
- Seen by: structure; refutation: confirmed; history: 79a944ab's message acknowledges "the same generator and oracle as the backend-arm sweep above" without a reason to copy the partner construction
- Owner-gated: no

Duplicated test logic drifts: a new adversarial partner class added to one sweep is silently absent from the other.

Evidence:

       857	        let b = match next() % 4 {
       858	            // An unrelated rank: usually a mismatched class.
       859	            0 => stream_rank(&mut next),
       860	            // An exact duplicate: the equality path.
       861	            1 => a.clone(),

Resolution: extract `fn adversarial_pair(next: &mut impl FnMut() -> u64) -> (Rank, Rank)` beside `stream_rank` and call it from both sweeps. Acceptance: one construction of the four partner classes in the file.

### version-core-32: The provenance-linearity test quotes measurements its body never produces, and its single-scale ceiling cannot see a change of order
- Where: crates/before/src/version/tests.rs:1187-1192 (related: crates/before/src/version/tests.rs:1237-1245; crates/before/src/version/rank.rs:36-38)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the body 1237-1245: one `assert!(encoded_bits <= input_bits)` per family, no print, no ratio; rank.rs:36-38 repeats "measured at 0.56 encoded bits per packed input bit" as design rationale); executed: no
- Seen by: structure ([10]), prose ([25]), correctness ([42]), claims ([53]); refutation: all confirmed; history: at the test's origin (f0f3a2aed) the body already asserted only the 1.0 ceiling and printed nothing; the ratios were a development-time measurement recorded in the commit message and transcribed into the doc, so "by this test's own instrumentation" was never true of the committed body
- Owner-gated: no

Two defects in one test. The doc reports five ratios "measured by this test's own instrumentation" that nothing computes, prints, or asserts: a hand-maintained measurement (Principle 5) that will keep asserting itself after the encoder moves, repeated in `rank.rs` as rationale. And the claim in the doc's first sentence, "encodes linearly in the version's packed bytes", is pinned by a ratio ceiling of 1.0 at one depth per family; with the worst family at 0.56 that is a constant-factor envelope, not an order pin: an encoder whose output grew as `version_bits · log(depth)` would sit under 1.0 at depth 800 and pass. The standard distinguishes an envelope with slack from an instrument that pins the order (a two-scale ratio or fitted band), and a linearity claim needs the latter.

Evidence:

      1186	/// holds each family's encoded size at or under 1.0 bit per packed input bit.
      1187	/// Measured \[by this test's own instrumentation\]: wide counter 0.56 (the
      1188	/// worst — a lone counter's version pays gamma's doubled width where the
      1189	/// encoding pays the width once), deep spine 0.38, dense staircase 0.38, deep
      1190	/// wide counter 0.27, plateau puncture 0.18; the 1.0 pin leaves headroom for
      1191	/// packing drift while sitting an order under the exponential blowup arbitrary
      1192	/// in-memory ranks can reach.

Resolution: measure each depth-parameterized family at two depths (`d` and `2d`) and assert the encoded-rank-bits to version-bits ratio does not grow beyond a stated slack; either assert the per-family ratios as a band (turning the slack into a tightened pin) or delete the five figures from this doc and from rank.rs:36-38, leaving the enforced 1.0 bound as the only number in prose; fix the stray `\[`/`\]` escapes either way. Acceptance: every numeric ratio in the doc is asserted by the body or absent; the test fails when the rank encoder is replaced by one whose output grows as `version_bits · log(depth)`.

Construction: for each family build versions at `d` and `2d` (`spine(800)`/`spine(1600)`, `staircase(800)`/`staircase(1600)`, `deep_counter(400, wide)`/`deep_counter(800, wide)`); compute `r(d) = rank().encode().len() * 8 / encoded_bits()`; assert `r(2d) <= r(d) + slack`. In a scratch build, append a depth-proportional header to the rank encoder: today's `encoded_bits <= input_bits` stays green at 800 while the two-scale check fails.

### version-core-33: `div_decomposes_along_fork`'s doc claims a disjoint-party check its body does not perform
- Where: crates/before/src/version/tests.rs:2118-2141 (related: crates/before/src/version/tests.rs:2143-2159)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the body at 2137-2140 asserts sub-version, rejoin, empty meet of the two projections, and seed identity; `div_view_matches_materialization` at 2157-2158 owns the disjoint-party assertion); executed: no
- Seen by: prose; refutation: confirmed; history: the overclaim is original (472d646e2), not drift
- Owner-gated: no

The crate's testing rules make a test's doc comment the statement of the invariant it protects and hold its accuracy to the standard of a bug; a doc that claims an assertion the body lacks misreports coverage.

Evidence:

      2120	/// Each half's contribution is a sub-version, the two rejoin to the whole, and
      2121	/// their supports are disjoint (so their meet is empty). The whole-interval
      2122	/// seed party is the identity, and projecting onto a *disjoint* party keeps
      2123	/// nothing.

Resolution: drop the final clause (the next test owns it), or add the disjoint-party assertion here and keep the sentence. Acceptance: every clause of the doc corresponds to an assertion in the body.

### version-core-34: The at-rest size pin hard-codes 64-bit literals beside the invariant it already states
- Where: crates/before/src/version/tests.rs:2195-2200 (related: justfile:525-526; crates/before/wasm32-pins)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (justfile:525-526 `wasm-check` is `cargo check -p before-viz --target wasm32-unknown-unknown`; wasm32-pins is a separate guest workspace with no `size_of` pin; so this unit suite never runs on a 32-bit target and the literals are latent); executed: no
- Seen by: structure ([13]), prose ([40]), correctness ([46]), claims ([54]); refutation: confirmed; history: deliberate-and-holds as the storage-shrink commits' witness (24ff18b93 pinned 24/48, d800957e8 re-pinned 32/64, each naming the drop in its message); the rationale lives only in history
- Owner-gated: no

The first assertion is the invariant (no field beside the container); the `32` and `64` literals restate it for one pointer width, and on a 32-bit host the suite would fail on a number, not on the invariant. The literals exist as a witness of the byte drop each storage change delivered, which is a real purpose the site does not state; a derived spelling keeps the witness and removes the host dependence.

Evidence:

      2195	    assert_eq!(
      2196	        core::mem::size_of::<Version>(),
      2197	        core::mem::size_of::<bytes::Bytes>()
      2198	    );
      2199	    assert_eq!(core::mem::size_of::<Version>(), 32);
      2200	    assert_eq!(core::mem::size_of::<crate::Clock>(), 64);

Resolution: replace the literals with `4 * size_of::<usize>()` and `size_of::<Party>() + size_of::<Version>()` (or cfg-gate the literal asserts with `target_pointer_width = "64"`), and state at the site that the pin witnesses the container-only storage (the doc's "32 bytes on 64-bit" can stay as illustration). Acceptance: the test passes unmodified on a 32-bit target and still fails if a field is added beside the container.

Construction: run the unit suite on any 32-bit target (i686, or wasm32 via a test runner): `size_of::<bytes::Bytes>()` is 16 there, so the `== 32` assert fails with `16 != 32`.

### version-core-35: The deep-spine pin's assert message points at "the query depth-guard size prose", which no longer exists
- Where: crates/before/src/version/tests.rs:2375-2401 (related: crates/before/src/version/skyline/query/integral.rs:172-174)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `depth-guard`, `depth guard`, `bits per level`, `depth-derived` over crates/before/src and tests outside this file: no such prose; the nearest live consumer is integral.rs:172-174, "each digit position of a window is a depth the stream's topology paid at least one bit for"; `git show --stat 6323d6677` touched query.rs and tests.rs); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (6ea3a18a2 added the pin to guard a `# Panics` sentence in query.rs; 6323d6677 widened the exponent to u64, excised that prose "everywhere it propagated", and re-denominated the doc comment as the grammar's exchange rate, but missed the assert message)
- Owner-gated: no

An assert message is prose the maintainer reads at the moment of failure; a pointer to an argument that cannot be found leaves them without the derivation they need (no ghost references). The pin protects a real premise (integral.rs prices each depth at "at least one bit", which the 3-bit rate over-satisfies) but names it by a name that does not exist.

Evidence:

      2381	/// the grammar ever admits a cheaper per-level spelling, this pin moves and any
      2382	/// prose pricing depth in input bytes must be re-derived with it.
      ...
      2399	        "the deep spine's marginal level cost moved off 3 bits: re-derive \
      2400	         the query depth-guard size prose from the new grammar"

Resolution: name the actual consumer in both the doc and the message: the `query::integral` funding argument's premise that every depth is paid at least one stored bit (which a 3-bit rate implies); or, if no prose depends on the 3-bit figure, restate the pin as pinning the grammar's per-level cost and drop the re-derive instruction. Acceptance: the assert message and doc cite a prose location that exists, by module and premise, or the pin's purpose is restated without a pointer.

### version-core-36: Five committed proptest seeds describe parameter shapes of properties that no longer exist in their files
- Where: crates/before/proptest-regressions/version/tests.txt:7-8 (related: crates/before/proptest-regressions/version/own/tests.txt:7-9; crates/before/src/version/own/tests.rs:127-144; tests/seed_liveness.rs:1-30)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (seed files read; grep over `proptest!` signatures in version/tests.rs finds no `scale` parameter and the only `k in` is `descending_literals_build_the_oracle_tree`'s `k in 1u64..=1u64 << 40`, a different shape from `(ops, i, j, k)`; own/tests.rs's only property is `mirror_cells_agree_on_arbitrary_triples(ow, ov, op)`; `tests/seed_liveness.rs` checks only that seed paths resolve, not that shrunk parameters match a live signature; the git attribution of each orphan to its dissolving commit is the history pass's, not re-run here); executed: no
- Seen by: structure; refutation: confirmed; history: each orphan traced (tests.txt:7 from 472d646e2, its `k`-taking properties dissolved by f3e715d0; tests.txt:8 from 782064269, its `scale` properties retired by c5b8d5ec9; own/tests.txt:7-9 from a7b6955e3, dissolved by f149d76b); no dissolving commit ruled on the seeds, and the root AGENTS.md "never strip" rule governs seeds a failure produces, not pruning orphans
- Owner-gated: yes (the doctrine forbids stripping seeds casually; this is a deliberate pruning that needs a ruling)

Proptest replays every seed in a file for every property in that file, so these still run as RNG seeds, but they no longer reproduce the failure they record and their comments name dissolved tests: a ghost reference (Principle 5) whose replay cost is paid on every run for a failure no live property can re-observe. No instrument catches this class: `seed_liveness` checks paths, not signatures.

Evidence:

         7	cc b187e79143e8821fb0e3c7c5d070a90ef9d77161138576d8c601af8c35253c26 # shrinks to ops = [Fork(0), Send(5, 1), Fork(1), Fork(0), Join(0, 3), Fork(1), Join(1, 2), Send(4, 0)], i = 0, j = 0, k = 46
         8	cc f2e1622bbf94d90894d55b51d4dda12cae2f3f7d6d0fd55126c22d635387dfa5 # shrinks to scale = 125

Resolution: for each orphaned seed, move it to the file of the property that now owns the invariant if one exists, else remove it in a commit naming the dissolved property; and consider extending `tests/seed_liveness.rs` to check each `# shrinks to` parameter list against the sibling file's live `proptest!` signatures, so the class is caught mechanically. Acceptance: every `cc … # shrinks to …` comment in the two files names only parameters of a `proptest!` signature in the sibling test file.

Construction: list each seed's `# shrinks to` parameter names and diff against the `proptest!` signatures in the sibling tests.rs; the five lines above have no match. To confirm they are inert as regressions: no live assertion can fail on them, because the property they shrank does not exist.

## Positives

- `DedupRuns` holds a clone rather than a raw address, with the reason stated (version.rs:1227-1229): a freed buffer reused at the same address mid-iteration cannot masquerade as a duplicate. Exactly the invisible-state hazard a comment should carry. (Verified by reading.)
- `Version::new`'s static-versus-const comment (version.rs:121-131) names the observable it protects and the laws that pin the constant; the twin at `Party::seed` (party.rs:122-129) matches word for word. (Verified.)
- The one `unreachable!` in the partition (version.rs:994-996) is a one-line proof resting on canonical uniqueness, and its premise is pinned by `eq_matches_causal_walk` (tests.rs:459-479); the refutation pass correctly held it against the mutants roster's sanctioned disposition for structurally-required unreachable arms. No assert, expect, or panic message in the partition carries an em-dash. (Verified.)
- Both n-ary folds keep their combine matches total on the `(Input, Merged)` arm with the commutativity argument stated (version.rs:675-679, 817-821), and the argument is correct against `fold.rs:41-80`: a weight-0 input is only ever the newest item in the counter or the last, right-hand group in the drain. The duplication is the finding, not the decision. (Verified against fold.rs.)
- `assert_cmp_cell` (tests.rs:33-61) is generic over `(L, R)` so each call resolves to exactly one impl cell, and its doc names the tell: a delegation cycle overflows the stack in the test rather than diverging in production. (Verified.)
- `own/tests.rs:1-13` maps the negative space: it says where the coherence and differential properties live so a reader knows where not to look here. (Verified.)
- `own.rs:133-141` explains why the mirror orientation drives its own co-walk rather than reversing the first (totality over every mask arrangement from the public surface), with the antisymmetry pinned by a named differential. (Verified.)
- `limbs_respell_the_count` (ticks/tests.rs:81-109) checks the exact-size contract at every step of the drain and the fused-past-the-end contract, not only at construction. (Verified.)
- `hull_traffic::record` compiles to nothing without the feature and is callable unconditionally (hull_traffic.rs:108-118), so the production ladder carries no cfg noise; the module doc earns the counters' existence by naming what no per-operation envelope can see (the caller's mix). The lenses report a per-rung liveness test at meter/tests.rs:565-622 and a consumer in rumors's tree tests; not re-read here.
- `deep_spine_marginal_cost_is_three_bits_per_level` (tests.rs:2375-2402) is a closed-form two-scale witness, the instrument shape the standard asks for; `boundary_arity_fan_folds_match_the_sequential_fold` (2315-2373) deliberately salts the fan with an adjacent clone run and an empty operand so three mechanisms meet in one deterministic construction. (Verified.)
- The join/meet subadditivity lemma is derived term by term in the tree (meter/tier2/tests.rs:328-346) with the tight constant identified and its equality case named; only its promotion to production prose is missing (version-core-5). (Verified.)
- The `Version` type doc's operation table and explicit "Comparison is **partial**" paragraph (version.rs:48-60), and `tick`'s which-one-do-I-use contrast with `Clock` (161-166), are the right altitude for a library user. (Verified.)
- The claims lens reports the `ticks` zero-count early return (version.rs:214-216) is load-bearing, not an optimization: `grow.rs:505-508` debug-asserts a nonzero splice. Not re-read here.

## Open questions for Finch

1. Was the `own`-strategy cell's saved refcount clone/drop pair ever measured to matter, or is anything pinning refcount traffic? Nothing I found does (the scan pins measure rungs). Recommendation: treat version-core-11's collapse as free and land it; if a reading is wanted, take it before, not after.
2. Does `emit::Hull.relation` have a planned consumer (the comparable-pair fast path 70eb67ab1 named)? Recommendation: if not, remove the field, the directions fold, the three emit-test assertions, and the `debug_assert!` together (version-core-13); if so, replace the assert with that consumer when it lands.
3. Moving the `Rank`/`Ranked` suites out of `version/tests.rs` (version-core-27) crosses into the rank partition. Recommendation: this note carries the module-doc fix; the move is a coordinated change for whoever owns rank.rs, with this finding as the pointer; citecheck's bare-name matching makes it roster-free.
4. Orphaned-seed disposition (version-core-36): prune, or retain as an RNG corpus? Recommendation: prune with the dissolved property named in the commit, and extend `tests/seed_liveness.rs` to match shrunk parameter names against live signatures so the class cannot recur; the "never strip" rule is about seeds a failure produced, which these no longer are.
5. `# Errors` uniformity (version-core-14): `Party::decode` shares `Version::decode`'s omission while the rank doors carry itemized sections. Recommendation: one crate-wide pass, and consider having `doclint` require the section on every `pub fn` returning `Result`.
6. Is `door` intended crate vocabulary (208 uses, no definition)? Recommendation: define it once in lib.rs if kept; otherwise the sweep replaces it per site (version-core-17).
7. ticks.rs:167 links `[`From<u64>`](Ticks#impl-From<u64>-for-Ticks)`; whether that fragment matches rustdoc's generated anchor cannot be checked without a docs build, which this review could not run. Recommendation: check once under `cargo doc`, or link `[`From`]` and let the sentence name `u64`.
8. `Version::concurrent` is generic over `V: PartialOrd<Version>` and so accepts an `OwnVersion`, but no law, differential row, or test instantiates it that way. Delegation is trivially correct; the question is whether the surface roster should record the view instantiation as a reachable cell.
9. Does rumors's long-lived version storage pay the retained-capacity slack (version-core-15) at a material rate? The tick and hull resident rows would answer it; if the stored versions are mostly tick outputs, a spilling tick's `Vec` doubling puts the resident cost near 2x the encoding.
10. From the dropped observation on the span ladder's order: the rung mix from rumors's reconciliation workload is already readable through `meter::span_traffic` in the tree tests. Recommendation: record one reading in a decision record so the comparable-first order rests on a number; a dependence-sweep item, not a partition change.

## Dropped

- `span_refs`'s `Some(Equal)` arm should hand back a value instead of panicking ([57]): refuted. The arm is the mutants roster's sanctioned shape for a structurally required unreachable branch, `causal_cmp` returns `Equal` only on byte-equal streams (pinned by `eq_matches_causal_walk` and the verdict matrix), and the crate's own rule wants a canonicality bug surfaced, not masked.
- Concurrent hulls pay a classifying sweep then the emitting walk ([56]): reframed to an observation. The design is documented at version.rs:956-967 and metered by `hull_traffic`; the reader's own resolution is "no code change". Carried as open question 10.
- `Ticks` text-I/O bound stated twice ([9]): merged into version-core-22.
- Wrong `span_all` link ([8], [43] part 1): merged into version-core-7.
- Ghost `alice` ([43] part 2): merged into version-core-6; `v / &p` spelling ([43] part 3): merged into version-core-3.
- Duplicate `|=` tests ([22], [47]): merged into version-core-28.
- Module doc omits half the file ([24]): merged into version-core-27.
- Provenance ratios in prose ([25], [53]) and the single-scale ceiling ([42]): merged into version-core-32.
- `trace_ticks` doc omits `Ticks` ([38], [45], [55]): merged into version-core-29.
- 64-bit literals ([40], [46], [54]): merged into version-core-34.
- Byte-compare rung labelled `O(1)` ([50]): merged into version-core-10.
- `shape`'s likelihood paragraph ([51]): merged into version-core-9.
- Subadditivity lemma unstated at the fold ([41]) and unpromised at the door ([48]): merged into version-core-5.
- `mints` ([17]), moralized adjectives ([29]), em-dashes ([30]), `door` ([31]): merged into version-core-17 as one crate-wide sweep item.
- Typographic bundle ([39]): version-core-3, with the own.rs:12 spelling and the hull_traffic.rs link added.
- Inert rustdoc links ([33]): version-core-4, with the version.rs:1349 site added.
- Hand-listed `reset` variants ([6]): version-core-26, with `snapshot` added.
- `Sum O(N)` per-summand term ([44]) and missing amortization ([59]): merged into version-core-23.
