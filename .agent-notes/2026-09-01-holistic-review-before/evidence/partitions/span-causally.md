# Partition span-causally: Span (algebra, owned spans, verdicts, wire) and the causally query language

## Partition summary

`Span` is an ordered pair of `Version`s (`lo <= hi`) stored as two `Cow<Version>`
endpoints. `span.rs` carries the type, the validating constructor, the four
verdict methods (`place`, `dominance`, `precedence`, `contains`), and the
clone-identity fast paths that answer a coincident span (both endpoints one
shared buffer) with one pair sweep instead of the fused three-stream walk in
`version/skyline/place.rs`. `span/algebra.rs` gives the four binary operators
(`+` union, `*` intersect, `|` pointwise join, `&` pointwise meet) as borrowed
kernels plus a receiver-seeded n-ary fold over `fold::balanced_reduce`, and
generates the owned/borrowed operator matrices by macro. `span/own.rs` is the
lazy per-party projection view, answering verdicts by two masked comparisons.
`span/verdict.rs` is the verdict vocabulary. `span/wire.rs` is the concatenated
canonical encoding and a strict decode whose admission walk parses the second
component while proving dominance. `causally.rs` and its submodules are the
polarity-restricted query language: `Query<P>` is an optional floor and
ceiling plus a hole antichain, with `Down`/`Up`/`Neutral` dispatch sealed in
`polarity.rs`, construction in `forms.rs` and `convert.rs`, the same-polarity
merge in `conjunction.rs`, and the two verdicts (`contains`, `coverage`) in
`query.rs` compiling bounds into `filter::admits`/`filter::coverage` demands,
with a clamp refinement behind the `Partial` arm.

I read all thirteen partition files in full (5213 lines; `span/tests.rs` and
`causally/tests.rs` are the test files) and the out-of-partition code every
finding leans on: `version.rs` (the version folds, `DedupRuns`, `span_refs`,
`encode`, the `Div` comment), `version/skyline/sweep.rs`, `place.rs`,
`place/filter.rs`, `fold.rs`, `laws.rs`, `codec/bits.rs`, `codec/scan.rs`,
`error.rs`, the board tiling and cells, the fuelscape roster, the meter's
placement module, `tests/coincident_span.rs`, and the git history the history
pass cited. I ran no cargo, just, test, or bench command; every anchor below is
by reading, and every count is by grep.

The kernels are correct on every arm I traced, and the verification around
them is unusually strong: the wire decode is a strict door with a single
allocation and genre precedence pinned on multiply-defective composites; the
coincident fast paths derive only what `Bits::ptr_eq` licenses and are held to
the fused walk both for verdict agreement and for liveness; the two-party grid
census makes `Coverage` exact rather than merely sound; and the polarity
dispatch concentrates the whole `Down`/`Up` difference in one sealed table
whose `Neutral` `unreachable!` arms carry a proof a reader can check inside the
module. The dominant issue is a cost claim: `Query::coverage`'s `Partial`
refinement sweeps the clamped endpoint once per surviving hole, so a query
with k pairwise-concurrent holes against a wide span costs Θ(k·|span|) against
a published `O(|self| + |span|)`, and no committed instrument builds a query
with more than one hole. Two structural issues follow it: the span operator
table is transcribed twice (binary kernels and fold constants), and the
two-sided receiver-seeded fold is written in both `algebra.rs` and
`version.rs`. The remainder is prose: a cluster of dangling pointers and
inverted arguments that the history pass traced to four terse owner hand-edit
commits (a6dcfbb4, b3f09baa, 20c0515a, bbb9f802), banned and undefined
vocabulary, and typos in public rustdoc.

## Findings

### span-causally-1: `Span` derives `Eq` but not `Hash`, unlike `Version` and every verdict type beside it
- Where: crates/before/src/span.rs:104-108 (related: crates/before/src/span/verdict.rs:7, 24, 56, 74; crates/before/src/causally/query.rs:51; crates/before/src/version.rs:97-106; crates/before/surfacecheck/src/census.rs:156-171)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: verified (read the derive lists; grep of the surfacecheck census shows `Eq`/`PartialEq` rows for `Span` and no `Hash` row); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (db9dfa3e's absent-`Hash` decision concerned `Query`, not `Span`)
- Owner-gated: yes: an additive public API change on a stable API, and a census change

`Span`'s equality is byte equality of two canonical streams, and `Version: Hash` hashes exactly those bytes (`canonical_hash`), so `#[derive(Hash)]` over the two `Cow<Version>` fields would be consistent with `Eq` by construction. `Placement`, `Endpoint`, `Dominance`, `Precedence`, and `Coverage` all derive `Hash`; a consumer can key a map by version but not by span. Suggestion only: before's API is stable.

Evidence:

       104	#[derive(Debug, Clone, PartialEq, Eq)]
       105	pub struct Span<'a> {
       106	    lo: Cow<'a, Version>,
       107	    hi: Cow<'a, Version>,
       108	}

Resolution: if the owner agrees, add `Hash` to the derive, add the census row, and extend a byte-equality law (the `version_eq_iff_bytes_eq` family) to spans so equal spans hash equal. Acceptance: `HashSet<Span<'static>>` compiles; the law is green; the census diff shows one added row.

### span-causally-2: Em-dashes inside `//` comments at fifteen partition sites
- Where: crates/before/src/span.rs:176-179 (related: crates/before/src/span.rs:248-249, 457; crates/before/src/span/wire.rs:126, 136; crates/before/src/span/tests.rs:334, 337, 474, 492, 897, 1006; crates/before/src/causally/tests.rs:200-201; crates/before/src/causally/query.rs:214)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nE '^\s*//[^/!].*—'` over the thirteen files returns exactly these fifteen lines); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (crate-wide practice: 374 such lines in 76 files)
- Owner-gated: no

The owner's doctrine prefers colons or semicolons over em-dashes in comments; rendered rustdoc (`///`, `//!`) may keep them, `//` comments may not. The partition has fifteen such lines.

Evidence:

       176	        // A borrowed endpoint is lent twice; an owned one moves in
       177	        // and its buffer-sharing clone fills the second slot — either
       178	        // way the pair reads one shared buffer, the O(1) coincidence
       179	        // certificate every fast path reads.

Resolution: recast each with a colon, semicolon, or parentheses. The crate-wide sweep (374 lines) is a separate prose-pass decision; see the open questions. Acceptance: the grep above returns nothing for the thirteen partition files.

### span-causally-3: The clone-identity coincidence certificate is spelled inline at five production sites while `Span::is_coincident` names it
- Where: crates/before/src/span.rs:255 (related: crates/before/src/span.rs:315, 380, 459, 553-555; crates/before/src/causally/query.rs:129; crates/before/src/span/algebra.rs:361, 562; crates/before/src/version.rs:1262)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep for `is_coincident\|\.ptr_eq(` over non-test production sources enumerates exactly the sites listed); executed: no
- Seen by: structure; refutation: confirmed (adds that `is_coincident` is private to `span.rs`, so `query.rs` cannot call it today); history: no rationale found (the inline sites predate the helper; ed1b3c8b added it for the algebra only)
- Owner-gated: no

The certificate is the pivot of every fast path in the partition, and every doc comment calls it one thing ("clone identity", "the coincident span's `O(1)` certificate"), yet `place`, `dominance`, `precedence`, the receiver test in `contains`, and `Query::coverage` each write `lo.view().ptr_eq(hi.view())` rather than calling the helper that exists for it. A reader hunting for "where does coincidence dispatch?" cannot grep for it by name. Legibility, and the small-scale form of the doubled-table problem (span-causally-12).

Evidence:

       255	        if self.lo.view().ptr_eq(self.hi.view()) {

       553	    fn is_coincident(&self) -> bool {
       554	        self.lo.view().ptr_eq(self.hi.view())
       555	    }

    (causally/query.rs)
       129	        if lo.view().ptr_eq(hi.view()) {

Resolution: make `is_coincident` `pub(crate)` and call it at span.rs:255, 315, 380, 459 and query.rs:129. For the version-pair sites (algebra.rs:361, 562; version.rs:1262) add a `pub(crate) fn Version::shares_buffer(&self, other: &Version) -> bool` so `is_coincident` is `self.lo.shares_buffer(&self.hi)` and `DedupRuns` reads `prev.shares_buffer(version)`. Acceptance: `grep -rn '\.view()\.ptr_eq(' crates/before/src --include='*.rs' | grep -v tests` returns only the helper bodies.

### span-causally-4: `Span::dominance`/`precedence` comments explain the fast path in the consumer's vocabulary ("compressed-subtree classification")
- Where: crates/before/src/span.rs:313-314 (related: crates/before/src/span.rs:377-379)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'compressed-subtree\|compressed subtree' crates/before/src` returns exactly these two sites; nothing in before names nodes or subtrees); executed: no
- Seen by: structure, prose; refutation: confirmed (corrects the cite: the abstraction-boundary rule is the root AGENTS.md "Writing style" section, not crates/before/AGENTS.md); history: no rationale found (4874f527 added the rung to serve rumors' classifier and wrote the motivation in rumors' terms)
- Owner-gated: no

Both coarsening fast paths close their rationale with a sentence about "compressed-subtree classification" and "a node whose version bounds coincide", which are the rumors tree's concepts. The root AGENTS.md asks documentation to respect abstraction boundaries; a maintainer of before has no node or subtree to map these to, and the paragraph above each sentence already states the in-crate mechanism (one single-bound placement instead of the fused walk reading one shared buffer twice).

Evidence:

       313	        // This is the compressed-subtree classification fast path: a node whose
       314	        // version bounds coincide is classified against one stream, not two.

       377	        // This is the compressed-subtree classification fast path, mirrored: a
       378	        // node whose version bounds coincide is classified against one stream,
       379	        // not two.

Resolution: delete both sentences, or restate in before's terms ("a caller holding many coincident spans classifies each against one stream"). Acceptance: the grep returns nothing.

### span-causally-5: Public rustdoc typos, a doubled word, a garbled sentence, and a ghost parameter name
- Where: crates/before/src/span.rs:413-415 (related: crates/before/src/span/algebra.rs:754-755, 785-786, 814-815, 899-902; crates/before/src/causally.rs:58-59, 81; crates/before/src/causally/conjunction.rs:5-6; crates/before/src/causally/forms.rs:231-234, 253)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (each excerpt read at the cited line; grep for `seach\|arbitary\|intractible\|of a \[`Span`\]s` and for line-final `taken as` locates exactly these sites; `toward`'s signature at forms.rs:253 is `toward(s, t)`); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: no rationale found (`after(p)` is a rename residue from a6dcfbb4, which renamed the parameter and rewrote the prose but left the equivalence line; the algebra.rs and span.rs items are from 22cdfbe1; "arbitary"/"intractible" from a6dcfbb4; "of a [`Span`]s" from bbb9f802)
- Owner-gated: no

Public rustdoc is the contract the reader came for, and these sit under `# Complexity` and on operator impls. Eight sites: span.rs:414 "the two `hi` endpoint, where seach comparison"; algebra.rs:754-755, 785-786, 814-815 "taken as / as its coincident point span" (the doubled word renders three times on the `Span` page); algebra.rs:900-902 "The smallest span containing [`Span`] two versions is"; causally.rs:59 "of a [`Span`]s"; causally.rs:81 "arbitary"; conjunction.rs:6 "intractible" (private module doc); forms.rs:234 "Equivalent to `after(p) & until(t)`" for a function whose parameters are `s` and `t` and whose prose uses `s` and `e`.

Evidence:

       413	    /// A [`Span`] requires two causal comparisons: one to compare the two `lo`
       414	    /// endpoints and a second to compare the two `hi` endpoint, where seach
       415	    /// comparison costs:

    (algebra.rs, likewise 785-786 and 814-815)
       754	    /// The right operand is anything [`Into`] a [`Span`]; a [`Version`] is taken as
       755	    /// as its coincident point span.

    (algebra.rs)
       900	    /// which a version-pair `+` would conceptually contradict. The smallest
       901	    /// span containing [`Span`] two versions is [`span`](Version::span) (`v ^
       902	    /// w`).

    (causally/forms.rs)
       231	/// Everything in the causal future of `s` (including `s` itself) but nothing in
       232	/// the causal future of `e` (including `e` itself).
       233	///
       234	/// Equivalent to `after(p) & until(t)`.
       253	pub fn toward<'a>(s: impl Into<Cow<'a, Version>>, t: impl Into<Cow<'a, Version>>) -> Query<'a, Up> {

Resolution: "the two `hi` endpoints, where each comparison costs:"; drop the duplicated "as" at three sites; "The smallest [`Span`] containing two versions is [`span`](Version::span) (`v ^ w`)."; "of [`Span`]s grant them"; "arbitrary"; "intractable"; make `toward`'s doc use `s` and `t` throughout ("nothing in the causal future of `t` (including `t` itself). Equivalent to `after(s) & until(t)`."). Acceptance: the greps return nothing and the doc letters equal the signature's.

### span-causally-6: The `Cow<Version>` `From` impls live in `span.rs` and claim a span-only purpose, but `causally::forms` depends on them
- Where: crates/before/src/span.rs:605-627 (related: crates/before/src/causally/forms.rs:55, 80)
- Class / severity / confidence: modularity / nit / high
- Provenance: verified (grep for `for Cow<` across crates/before/src finds only span.rs:611 and 623; forms.rs:55 and 80 take `impl Into<Cow<'a, Version>>`); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (ed840d93 added them for the span constructors; causally adopted the signature five days later)
- Owner-gated: no

`impl From<&Version> for Cow<Version>` and `impl From<Version> for Cow<Version>` are what let every `causally` constructor (`after`, `before`, `strictly_after`, ...) accept owned or borrowed versions, yet their rustdoc names only `Span` and they sit in `span.rs`. A reader of `causally::after`'s signature looking for the conversion finds it in an unrelated module with a doc that says it exists for something else.

Evidence:

       605	/// Lends this version to a [`Cow`]-accepting callsite, providing automatic
       606	/// reference lifting for methods on [`Span`]s which take [`Version`]s.
       607	///
       608	/// # Complexity
       609	///
       610	/// `O(1)`.
       611	impl<'a> From<&'a Version> for Cow<'a, Version> {

Resolution: move both impls to `version.rs` beside `Version`'s other conversions and reword the doc to name the `Cow`-accepting constructors generally. Acceptance: the impls and docs live in version.rs; nothing else changes.

### span-causally-7: The `v + w` absence argument is written out three times in `algebra.rs`
- Where: crates/before/src/span/algebra.rs:39-43 (related: crates/before/src/span/algebra.rs:22, 672-675, 899-902)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (all three copies landed in 22cdfbe1)
- Owner-gated: no

The module doc announces "Totality arguments, once:" (line 22) as the file's policy, then the `span_version_lhs_matrix!` macro doc and the public `v + s` cell each restate in full why `Version + Version` is absent. The three already differ ("contradict" versus "conceptually contradict"), and the third copy is where the garbled sentence of span-causally-5 lives.

Evidence:

        39	//! `v | s`, `v & s`) with the same meaning. The one deliberate hole in the
        40	//! symmetry is a version pair: `v | w` and `v & w` keep the version
        41	//! lattice's own meaning, and `v + w` stays absent because [`Sum`] for
        42	//! [`Version`] is the join fold, which a version-pair `+` would contradict
        43	//! (the hull of two versions is `v ^ w`). The

Resolution: keep the module-doc statement; reduce 672-675 to a pointer ("a version-pair `+` is deliberately absent; the module doc carries the argument") and 899-902 to the user-facing consequence ("There is no `Version + Version`; the span of two versions is [`span`](Version::span) (`v ^ w`)."). Acceptance: `grep -n 'join fold' crates/before/src/span/algebra.rs` returns one site.

### span-causally-8: The span folds' time and space claims are priced only by proxy `version_*_all` cells that never execute `fold_endpoints`
- Where: crates/before/src/span/algebra.rs:98-102 (related: crates/before/src/span/algebra.rs:171-175, 244-248, 310-314, 339-424; crates/before/src/meter/board/coverage.rs:4-5, 165-171; crates/before/src/fold.rs:1-16)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read coverage.rs:165-171: `("Span::union_all", &["version_span_all"])`, `("Span::intersect_all", &["version_join_all", "version_meet_all"])`, and the join/meet rows; grep of tests/meter.rs for `union_all\|intersect_all\|Span::union` finds no span-fold rows, only `Version::join_all`/`meet_all` rows; grep of fuzzfit/harness/src/bands.rs finds no span or query band); executed: no
- Seen by: claims; refutation: confirmed (adds that the fuelscape span islands are audit-only, justfile:675); history: deliberate-and-holds (the board's stated policy "rows price delegations at their shared mechanism"), but its premise holds only at fold.rs's counter: `fold_endpoints` is its own body (span-causally-9), which is exactly what makes a regression there invisible
- Owner-gated: no

Four public `# Complexity` sections state "Auxiliary space is `O(|self| + |iter|)`" and their islands state `O((|self| + |iter|) log k)`. The argument (the balanced counter holds at most log k merged groups, each group's two legs bounded by the packed size of the inputs it absorbed, since a join or meet of two skylines is O(|a| + |b|) bits) is stated nowhere in the module. The board prices these rows through `Version`'s own folds, which never run `fold_endpoints`'s dedup filter, two-leg groups, or point-combine. A regression confined to `fold_endpoints` (a left fold, or a byte-copying dedup filter) would move nothing enforced. The crate docs make asymptotic claims hard guarantees, and a claim needs an argument and an instrument that fails when it is false.

Evidence:

        98	    /// # Complexity
        99	    ///
       100	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html"))]
       101	    ///
       102	    /// Auxiliary space is `O(|self| + |iter|)`.

    (meter/board/coverage.rs)
       165	    ("Span::union_all", &["version_span_all"]),
       166	    (
       167	        "Span::intersect_all",
       168	        &["version_join_all", "version_meet_all"],
       169	    ),
       170	    ("Span::join_all", &["version_join_all"]),
       171	    ("Span::meet_all", &["version_meet_all"]),

Resolution: state the space argument once at `fold_endpoints`'s doc. Add a board cell (heap and scan currencies) over a mixed point/wide family so both combine arms run, or a `scan-meter` two-scale row in tests/meter.rs, and map the four roster rows to it instead of the `version_*_all` proxies. If span-causally-9 unifies the folds first, the proxy becomes sound and only the argument is owed. Acceptance: rewriting `fold_endpoints` as a sequential left fold (or making the dedup filter copy bytes) fails the new cell at two scales; the current code passes.

Construction: receiver `a_1.span(&a_2)`; items alternate `Span::at(&x_i)` points and wide `a_i.span(&b_i)` hulls over interleaved single-tick versions (the shape fold.rs:9-14 names as growing without coalescing); arity k in {64, 512}. Measure peak heap and scan bits around `receiver.union_all(&items)`; a left fold scales as k · Σ|items|, the balanced fold as Σ|items| · log k.

### span-causally-9: The receiver-seeded two-sided fold is written twice, with `FoldInput`, the group enum, and the adjacent-clone dedup re-declared in each file
- Where: crates/before/src/span/algebra.rs:349-424 (related: crates/before/src/span/algebra.rs:516-565; crates/before/src/version.rs:633-698, 796-843, 1215-1331; crates/before/src/fold.rs:1-3)
- Class / severity / confidence: modularity / medium / high
- Provenance: assessed (read both fold bodies side by side, both `FoldInput` enums, `Group`/`Hull`, and `DedupRuns`); executed: no
- Seen by: structure; refutation: confirmed (one correction: fold.rs's "one home" claim is about the counter, which both folds share; the duplicated layer is the group/receiver/dedup shape above it); history: no rationale found (`DedupRuns`, `FoldInput`, `Hull` landed 2026-07-30; `fold_endpoints` with its own filter and enums one day later without reusing them)
- Owner-gated: no

`fold_endpoints` and `Version::span_all` run the same balanced two-sided fold with the same four-arm match and the same commutativity comment; `algebra.rs` re-declares `FoldInput` and a two-sided `Group` that `version.rs` already has as `FoldInput` and `Hull`, and hand-rolls the adjacent-clone dedup filter that `version.rs` names, documents, and argues as `DedupRuns`. The load-bearing safety argument for `ptr_eq` dedup (the filter holds a clone, so a freed allocation cannot be reused at the same address and masquerade as a duplicate) is stated only in `version.rs`; `algebra.rs` relies on it silently. `Version::span_all`'s leaf combine `span_refs` is exactly `union_points`, so its body is `fold_endpoints` under `UNION_OPS` with point items. A change to the fold shape must now be made in three places.

Evidence:

       354	        // The dedup filter: one (lo, hi) buffer-identity pair of state.
       355	        let mut last: Option<(Version, Version)> = None;
       356	        let inputs = core::iter::once(FoldInput::Receiver(self))
       357	            .chain(iter.into_iter().map(FoldInput::Item))
       358	            .filter(move |input| {
       359	                let s = input.span();
       360	                let dup = last.as_ref().is_some_and(|(lo, hi)| {
       361	                    lo.view().ptr_eq(s.lo().view()) && hi.view().ptr_eq(s.hi().view())
       362	                });

       400	                    // Unreachable through the counter's weight discipline (a
       401	                    // weight-0 lone input never sits below a merged group in
       402	                    // the closing drain), but the match stays total rather than
       403	                    // asserting: every leg kernel is commutative, so folding
       404	                    // the raw input into the owned group is value-identical.

    (version.rs, the same comment)
       675	                // Unreachable through the counter's weight discipline (a
       676	                // weight-0 lone input never sits below a merged group in the
       677	                // closing drain), but the match stays total rather than
       678	                // asserting: both sides' combiners are commutative, so folding
       679	                // the raw input into the owned hull is value-identical.

Resolution: give the two-sided fold one home. One shape: a private `Endpoints` trait (`lo()`, `hi()`, `point()`) implemented for `Span<'_>` and for a `Version`-wrapping newtype (lo = hi = the version, `point()` always `Some`); a shared `FoldInput<'r, T>` and `DedupRuns` keyed on `(&Version, &Version)` (the one-sided folds pass `|v| (v, v)`); one `fold_endpoints<T: Endpoints>(receiver, items, &SpanFoldOps)`. `Version::span_all` becomes a call into it with items wrapped, keeping its stable `Borrow<Version>` item convention; `Hull` and the second `FoldInput`/dedup dissolve. Acceptance: `grep -rn 'weight-0 lone input never sits below' crates/before/src` returns one production site (or two, if `balanced_fold` keeps its one-sided copy); `enum FoldInput` and the dedup adapter are each declared once; `span_all_is_the_family_hull`, `span_union_of_points_is_span_all`, and the n-ary span laws stay green; the `span_all`/`join_all` envelopes in tests/meter.rs are re-measured at the parent and unchanged (the change deletes no work and adds none).

### span-causally-10: Inline qualified paths where imports exist, and a `core::` beside `std::` imports
- Where: crates/before/src/span/algebra.rs:356-369 (related: crates/before/src/causally/polarity.rs:30; crates/before/src/version.rs:842)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `core::` over the partition returns exactly algebra.rs:356 and polarity.rs:30; crate-wide, `use core::cmp::Ordering` appears in 32 files and `use std::cmp::Ordering` in 15; the crate is not `no_std`); executed: no
- Seen by: structure; refutation: reframed (the crate-wide majority spelling is `core::`, so the partition's `std::` imports are the minority; the convention is a crate-wide question); history: no rationale found
- Owner-gated: no

`algebra.rs` imports `std::iter::{Product, Sum}` at line 52 but writes `core::iter::once(...)` and `crate::fold::balanced_reduce(...)` inline; the owner's doctrine is imports over long qualified paths except where the qualification informs, and here it does not. The `core::`/`std::` mix is crate-wide (see the open questions), not a partition defect.

Evidence:

       356	        let inputs = core::iter::once(FoldInput::Receiver(self))

       369	        let group = crate::fold::balanced_reduce(inputs, |a, b| {

Resolution: `use std::iter::{once, Product, Sum};` and `use crate::fold::balanced_reduce;` in algebra.rs (or `core::iter::once` if the crate settles on `core::`). Acceptance: no inline `core::`/`crate::fold::` at use sites in the partition.

### span-causally-11: `intersect_points` is the swapped hull: one `span_refs` call replaces the equality compare plus two emission walks, and its comment names a rescue that cannot happen
- Where: crates/before/src/span/algebra.rs:452-463 (related: crates/before/src/span/algebra.rs:193-202, 445-450; crates/before/src/version.rs:968-1008)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (re-derived rung by rung against `span_refs` at version.rs:968-1008); executed: no
- Seen by: structure, claims; refutation: confirmed (re-derived the swap on every rung; notes no committed traffic snapshot surrounds an intersect call, so nothing moves except the unmeasured cost); history: no rationale found (only the equal-pair byte compare was deliberate)
- Owner-gated: no

Intersect's point-combine wants `(a ∨ b, a ∧ b)`; `Version::span_refs(a, b)` returns `(a ∧ b, a ∨ b)` through a five-rung ladder (equal, empty-`a`, empty-`b`, comparable, concurrent). Swapping its output is value-identical on every rung: equal gives `(a, a)`; empty `a` gives `(b, 0)`; empty `b` gives `(a, 0)`; `a < b` gives `(b, a)`; concurrent gives `(join, meet)` from one fused `emit::hull` where the current code runs `join_refs` and `meet_refs` as two full walks. The comparable rung answers with two `O(1)` clones after one sweep. This is a strict deletion of redundant work (fixed sign). Separately, the parenthetical "(or a later combine absorbs)" reads as if a crossed pair could later un-cross; under `INTERSECT_OPS` `lo` only joins upward and `hi` only meets downward, so `lo' <= hi'` would imply `lo <= lo' <= hi' <= hi`: a crossed pair stays crossed to the closing `partial_cmp` at 199-202, which is the actual soundness argument for deferring the verdict, and it is stated nowhere.

Evidence:

       452	/// Intersection's point-combine: two points share a version exactly when they
       453	/// are equal.
       454	///
       455	/// One byte compare answers the only nonempty case; an unequal pair pays the
       456	/// per-leg walks whose crossed output the operator's final validation rejects (or a
       457	/// later combine absorbs).
       458	fn intersect_points(a: &Version, b: &Version) -> (Version, Version) {
       459	    if codec::canonical_eq(a.view(), b.view()) {
       460	        return (a.clone(), a.clone());
       461	    }
       462	    (Version::join_refs(a, b), Version::meet_refs(a, b))
       463	}

Resolution: `fn intersect_points(a: &Version, b: &Version) -> (Version, Version) { let (lo, hi) = Version::span_refs(a, b); (hi, lo) }`, with the doc stating the swap and replacing the parenthetical with the monotonicity argument ("a crossed pair stays crossed under further join-`lo`/meet-`hi` legs, so the closing `partial_cmp` decides for the whole family"), cited from `intersect_all`. Note `span_refs` records `hull_traffic` rungs; no committed intersect snapshot exists, so nothing committed moves. Acceptance: a case in the pointwise laws asserting `intersect_points(a, b) == { let (l, h) = span_refs(a, b); (h, l) }` over arbitrary pairs; `nary_doors_match_sequential_folds_on_a_mixed_family` and the intersect laws stay green.

### span-causally-12: The span operators are encoded twice: `*_core` kernels re-implement the `SpanFoldOps` table with inline fast paths, and two ordering matches restate `Span::new`
- Where: crates/before/src/span/algebra.rs:568-635 (related: crates/before/src/span/algebra.rs:199-202, 375-382, 448-514; crates/before/src/span.rs:145-148; crates/before/src/version.rs:864-945)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read both encodings; `join_refs` and `clone()+join_view` have the same three short-circuits in the same order per version.rs:884-900, so the value is unchanged); executed: no
- Seen by: structure; refutation: reframed (the duplication is real and `Span::new(lo, hi).ok()` is a loss-free replacement for both ordering matches, but the intersect "divergence" is by design: the fold defers its verdict to the closing `partial_cmp`, so `intersect_points` must return a crossed pair where the binary kernel returns `None`; keep `intersect_core`'s point fast path); severity lowered to low; history: no rationale found (both transcriptions landed together in ed1b3c8b and were reworked together in 22cdfbe1)
- Owner-gated: no

The module doc states the operator table once as four leg assignments (lines 14-20); the code states it twice: `union_core`/`join_core`/`meet_core` hand-write the coincident fast path and the per-leg kernel pair that `UNION_OPS`/`JOIN_OPS`/`MEET_OPS` with `*_points` already encode for the fold, and `fold_endpoints`'s `(Group::Input, Group::Input)` arm plus the `point()` check is already the binary combine. The ordering match at 199-202 and 599-602 is `Span::new`'s body (span.rs:145-148) restated. A reader verifying `a + b == union_all([b])` must reconcile two spellings (`clone()+*_view` in the cores, `*_refs` in the fold) whose equivalence rests on version.rs's "keep in lockstep" ladders.

Evidence:

       568	fn union_core(a: &Span<'_>, b: &Span<'_>) -> Span<'static> {
       569	    if a.is_coincident() && b.is_coincident() {
       570	        // Two points' union is their hull: one fused pair walk (Version::span's
       571	        // ladder, fast paths and traffic accounting included) where the per-leg
       572	        // folds below would walk the same operand pair twice.
       573	        return a.lo().span(b.lo());
       574	    }
       575	    let mut lo = a.lo().clone(); // O(1): a stored version's clone shares its buffer
       576	    let mut hi = a.hi().clone();
       577	    lo.meet_view(b.lo().view());
       578	    hi.join_view(b.hi().view());

    (the fold's encoding of the same table)
       448	fn union_points(a: &Version, b: &Version) -> (Version, Version) {
       449	    Version::span_refs(a, b)
       450	}

    (the restated constructor, also at 599-602)
       199	        match lo.partial_cmp(&hi) {
       200	            Some(Ordering::Less | Ordering::Equal) => Some(Span::owned(lo, hi)),
       201	            Some(Ordering::Greater) | None => None,
       202	        }

Resolution: one private `fn combine(a: &Span<'_>, b: &Span<'_>, ops: &SpanFoldOps) -> (Version, Version)` that does `match (a.point(), b.point()) { (Some(va), Some(vb)) => (ops.points)(va, vb), _ => ((ops.lo_refs)(a.lo(), b.lo()), (ops.hi_refs)(a.hi(), b.hi())) }` (lifting `Group::point` for `Input` to a `Span::point` helper). `union_core`/`join_core`/`meet_core` become `Span::owned(combine(a, b, &OPS))`; `intersect_core` keeps its point fast path and otherwise becomes `let (lo, hi) = combine(a, b, &INTERSECT_OPS); Span::new(lo, hi).ok()`; `intersect_all`'s closing match becomes `Span::new(lo, hi).ok()`; `fold_endpoints`'s `(Input, Input)` arm calls the same `combine`. Acceptance: `Version::join_refs`/`meet_refs`/`span_refs` each appear once in algebra.rs (in the `*_OPS` constants or `*_points`); the two hand-written ordering matches are gone; the span algebra laws and `nary_doors_match_sequential_folds_on_a_mixed_family` stay green.

### span-causally-13: Maintainer docs in `algebra.rs` give the wrong reason for the missing identities and a clone-discipline sentence the code contradicts
- Where: crates/before/src/span/algebra.rs:1014-1016 (related: crates/before/src/span/algebra.rs:1118-1120, 1041-1043, 346-348, 364)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (re-derived both identities; read the fold doc against the filter body); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (severity lowered from medium: private macro docs; the public impl doc at 1041-1043 says only "(union has no identity span)", which is correct); history: no rationale found (both identity paragraphs are the owner's own text, 20c0515a; the clone sentence and the clone-on-entry filter landed together in ed1b3c8b)
- Owner-gated: no

Three maintainer sentences state an argument the code does not support. (1) The `span_union_fold!` doc says union has no identity because "the version lattice has no top": an identity `e` for `+` needs `lo_a & lo_e = lo_a` for all `a` (so `lo_e` is a top) and `hi_a | hi_e = hi_a` (so `hi_e` is the bottom), i.e. the crossed pair `(top, bottom)`, which is no `Span` whether or not a top exists; the missing top is not the operative reason. (2) The `span_intersect_fold!` doc says "no span is covered by every span": the intersection identity is the span that covers every span, `(bottom, top)`, so the direction is inverted, and there the missing top is the actual reason. (3) `fold_endpoints`'s doc says inputs "are cloned only at their first combine", but the dedup filter clones every admitted input's endpoints on entry (line 364, refcount bumps).

Evidence:

      1014	/// The receiver is [`Option`] because union has no identity: the version
      1015	/// lattice has no top, so an empty iterator has no non-empty hull. `None`
      1016	/// means exactly "no spans came", never an empty union. The item shapes

      1118	/// [`None`] covers both an empty iterator (intersection has no identity: the
      1119	/// version lattice has no top, so no span is covered by every span) and a
      1120	/// nonempty family sharing no version — the two ways there is no product.

       346	    /// so the fold is never empty. Inputs enter untouched and are cloned only
       347	    /// at their first combine, and every clone of a stored version is a
       348	    /// refcount bump, never a byte copy.
       364	                    last = Some((s.lo().clone(), s.hi().clone()));

Resolution: union: "union has no identity span: the identity would be the empty set of versions, and every [`Span`] is nonempty (`lo <= hi`)". Intersection: "intersection has no identity: the identity would be the span covering every span, `[bottom, top]`, and the version lattice has no top." Fold doc: "inputs are borrowed into the counter; the dedup filter holds one refcount pair, and every clone of a stored version is a refcount bump, never a byte copy." Acceptance: the three sentences read true against the code; "covered by" at 1119 becomes "covers".

### span-causally-14: `OwnSpan`'s composed two-comparison verdicts are a deliberate design whose rationale lives only in history
- Where: crates/before/src/span/own.rs:88-90 (related: crates/before/src/span/own.rs:112-131, 166-181, 217-232, 254-262; crates/before/src/span.rs:263-267, 327-331, 392-396, 462-466)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read the four `OwnSpan` verdicts against `Span`'s fused walks); executed: no
- Seen by: claims; refutation: confirmed (the islands price the composed cost, so no claim is false; a masked arity-three walk is a design proposal); history: deliberate-and-holds (ed1b3c8b: "placement and dominance answered from two masked co-walks without materializing"; the own-version-view note specifies only three- and four-stream comparison co-walks)
- Owner-gated: no for the sentence; the fused masked kernel itself is an owner design decision (see the open questions)

Each `OwnSpan` verdict runs two masked comparisons, decoding the probe and the party's id stream twice, where the unmasked `Span` verdicts fuse the same question into one three-stream walk precisely to save the second probe decode. That is the recorded design (a masked three-stream placement kernel was never planned), but the recorded reason is in a commit message and a design note, not at the site; a maintainer comparing `own.rs` to `span.rs` finds no statement that the composition is intended.

Evidence:

        88	    /// Compares `version` against this [`OwnSpan`] at full resolution,
        89	    /// rendering a nine-way [`Placement`] verdict: [`Span::place`], against the
        90	    /// projected endpoints, without materializing the projection.

Resolution: one sentence on the type or the `place` doc: the verdicts compose two masked comparisons (no fused masked placement walk exists; the islands price the composition). If the owner wants the fused kernel, that is a new `skyline::masked` arity-three walk routed under the four verdicts with `own_span_matches_the_projected_span` as the oracle. Acceptance: the design statement is at the site.

### span-causally-15: `OwnSpan` hand-writes the nine-way and three-way verdict tables that `place.rs` and the `verdict.rs` docs already state
- Where: crates/before/src/span/own.rs:112-131 (related: crates/before/src/span/own.rs:166-181, 217-232, 254-262; crates/before/src/span.rs:256-261; crates/before/src/version/skyline/place.rs:224-239; crates/before/src/span/verdict.rs:25-85; crates/before/src/laws.rs:1675-1693)
- Class / severity / confidence: modularity / low / medium
- Provenance: assessed (read all transcriptions); executed: no
- Seen by: structure; refutation: confirmed (adds laws.rs:1675-1693 `place_from_relations` as a test-side fourth transcription); history: no rationale found; the duplication has already cost once (4d9641bd: a mutation probe found the hand-written `Concurrent(Start)` arm unpinned and a dedicated witness had to be added)
- Owner-gated: no

The rule mapping two relations to a `Placement` lives in `own.rs` (nine arms), in `place::span`'s closing closure (the same table in another arm order), as `Span::place`'s coincident diagonal, and as prose on the `Placement` variants; `dominance`, `precedence`, and `contains` are the same two-comparison transcription of the rules the `Dominance`/`Precedence` variant docs spell. A constructor on the verdict type puts the table where its documentation already is and makes the four `OwnSpan` methods one-liners whose correctness is the constructor's, not a per-method re-derivation. The test-side copy in laws.rs is the differential oracle and may deliberately stay independent.

Evidence:

       112	    pub fn place(&self, version: &Version) -> Placement {
       113	        let (lo, hi) = (self.lo(), self.hi());
       114	        match version.partial_cmp(&lo) {
       115	            Some(Ordering::Less) => Placement::Before,
       116	            Some(Ordering::Equal) => match version.partial_cmp(&hi) {
       117	                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
       118	                _ => Placement::At(Endpoint::Start),
       119	            },

Resolution: `pub(crate) fn Placement::from_relations(vs_lo: Option<Ordering>, vs_hi: Option<Ordering>) -> Placement` (and `Dominance::from_relations`, `Precedence::from_relations`) in verdict.rs, stated once beside the variant docs, with `(None, None) => Concurrent(Both)` as the total definition (what own.rs returns today at line 127); `OwnSpan::{place,dominance,precedence}` call them; `Span::place`'s coincident rung calls `Placement::from_relations(r, r)`; `place::span`'s closure calls it after its `debug_assert`. Acceptance: one nine-arm `Placement` table in production code; `own_span_place_reaches_every_concurrent_corner`, `own_span_matches_the_projected_span`, `span_place_places_every_witness`, and `coincident_span_rungs_agree_across_buffer_identity` stay green.

### span-causally-16: `OwnSpan::to_span` cites a monotonicity argument "the type's docs carry" that no type's docs carry
- Where: crates/before/src/span/own.rs:264-270 (related: crates/before/src/version/own.rs:12-46; crates/before/src/version.rs:700-724, 1669-1676; crates/before/src/laws.rs:2620-2628, 2688-2690)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -ni monoton crates/before/src/version/own.rs` returns nothing; read `OwnVersion`'s type doc and `Version::project`'s doc, which state only that a projection is a sub-version; the argument lives at the `Div` impl comment, version.rs:1669-1674, and the `projection_monotone_in_version` law, laws.rs:2623); executed: no
- Seen by: prose, correctness; refutation: confirmed; history: no rationale found (born dangling in ed1b3c8b)
- Owner-gated: no

`to_span` builds through `Span::owned` with no validation, so monotonicity of projection is load-bearing for the `lo <= hi` invariant, and the doc points at an argument that is not where it says. The argument that exists (projection is a join and meet homomorphism, so `a <= b`, i.e. `a | b == b`, gives `a/p | b/p == b/p`, i.e. `a/p <= b/p`) lives at the `Div` impl comment and the law. One caution when repointing: version.rs:1675-1676 reads "Projection can still raise `min_ticks` ..., so it is not monotone under `<=`", whose "it" a reader takes as the projection; that sentence should say `min_ticks` is what fails to be monotone.

Evidence:

       264	    /// Materializes the projected span: the explicit, eager form of
       265	    /// this view.
       266	    ///
       267	    /// One [`OwnVersion::to_version`] per endpoint; the projection is
       268	    /// monotone (the type's docs carry the argument), so the
       269	    /// projected pair is ordered and the construction revalidates
       270	    /// nothing.

Resolution: state the argument inline ("projection is a join homomorphism, so `lo <= hi` gives `lo / p <= hi / p`; the `projection_monotone_in_version` law pins it") and optionally add the monotonicity sentence to `OwnVersion`'s docs; disambiguate version.rs:1675-1676. Acceptance: the pointer resolves (`grep -ni monoton` finds the argument at the cited site), and the `Div` comment's "it" names `min_ticks`.

### span-causally-17: "door" is used as jargon for constructors and entry points without a definition
- Where: crates/before/src/span/tests.rs:149-151 (related: crates/before/src/span/tests.rs:166, 203, 209, 231, 818, 904, 918, 936, 1045)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -nE '\bdoors?\b'` over the partition returns exactly the ten span/tests.rs sites; crate-wide the count is 208 with no definitional use in lib.rs or error.rs); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (owner vocabulary, undefined; 22cdfbe1 replaced every "door" in algebra.rs with "operator", so the partition's remaining uses are all in tests)
- Owner-gated: yes: a crate-wide vocabulary decision

Coined terms must be anchored to an identifier or defined once by contrast; "door" ("the validating door", "constructor doors", "n-ary doors") names no identifier and is defined nowhere. It is harmless where the reader can substitute "constructor", but it is default-dialect texture promoted to jargon, and the production trend (22cdfbe1) is already away from it.

Evidence:

       149	/// The validating door admits exactly the ordered pairs: `lo <= hi` composes
       150	/// (coincident included), while reversed and incomparable pairs are rejected
       151	/// with `Crossed`.

       166	/// The constructor doors accept any ownership mix per endpoint (owned,

Resolution: in this partition, the plain noun ("constructor", "entry point", "`span_all`"). Crate-wide, define "door" once in lib.rs or retire it (recommendation: retire; see the open questions). Acceptance: owner rules on the term; the ten sites read with the plain noun or the term is defined at the crate root.

### span-causally-18: A testdoc hardcodes "depth 2" for a constant defined elsewhere
- Where: crates/before/src/span/tests.rs:608-609 (related: crates/before/src/span/tests.rs:368-369; crates/before/src/testing/exhaustive.rs:83)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`pub(crate) const EV_SMALL_DEPTH: usize = 2;` at exhaustive.rs:83; the sibling test at 368-369 reads the constant by name); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

No hand-maintained counts: a number that matters lives in a mechanically-enforced place that prose cites by name. The prose rots silently if `EV_SMALL_DEPTH` moves.

Evidence:

       608	/// The small-scope sweep is exhaustive to depth 2; these constructed families
       609	/// sample the genres it cannot contain — 300-level spines (deep path stacks,

Resolution: "The small-scope sweep is exhaustive to `EV_SMALL_DEPTH`; ...". Acceptance: the testdoc cites the constant by name.

### span-causally-19: `Dominance` and `Precedence` docs drift in vocabulary between mirrored variants
- Where: crates/before/src/span/verdict.rs:69-85 (related: crates/before/src/span/verdict.rs:51-67)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read both enums); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (`Precedence` was added in b3f09baa with "probe"/"beside" where `Dominance` says "version"/"concurrent to")
- Owner-gated: no

The two enums are documented as each other's mirror, which is the reader's aid for learning the second from the first; `Dominance` speaks of "the version" and "above or concurrent to", `Precedence` of "the probe"/`p` and "below or beside" for the same roles.

Evidence:

        71	/// This is [`Placement`] coarsened to the precedence question, "is the probe
        72	/// causally at or before the span's content?" ([`Dominance`] renders the

        82	    /// The version does not precede even the end: `hi` is below or
        83	    /// beside the probe (and with it `lo`).

    (Dominance, the mirror)
        53	/// This is [`Placement`] coarsened to the dominance question, "is the version
        54	/// causally at or after the span's content?" ([`Precedence`] renders the
        64	    /// The version does not dominate even the start: `lo` is above or
        65	    /// concurrent to the version (and with it `hi` as well).

Resolution: one noun ("version", matching the parameter name) and one relation phrase ("concurrent to", the crate's public term) in both enums. Acceptance: `grep -n probe crates/before/src/span/verdict.rs` returns nothing; the variant docs differ only by direction words.

### span-causally-20: `wire.rs` calls the endpoints "the meet" and "the join" where the rest of `Span` says `lo`/`hi` and `meet`/`join` name the operators
- Where: crates/before/src/span/wire.rs:40-41 (related: crates/before/src/span/wire.rs:123-127, 136-140, 146, 148-149, 165-166; crates/before/src/span/tests.rs:290-294; crates/before/src/span.rs:75-78; crates/before/src/span/algebra.rs:205-301)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read the sites); executed: no
- Seen by: structure, prose, claims; refutation: confirmed (a vocabulary collision, not an error: `lo` is the meet of `{lo, hi}`); history: deliberate-but-expired (the accessors were named `meet()`/`join()` until b3f09baa renamed them to `lo()`/`hi()`; wire.rs, the doctest, and the test genre strings were not swept)
- Owner-gated: no

The `Span` docs define the wire form as "its `lo` [`Version`] followed by its `hi` [`Version`]", and on `Span`, `meet` and `join` are the pointwise operators two headings away. The public doctest comment and the private decode comments call the endpoints the meet and the join, so one word means two things on one page.

Evidence:

        40	    /// // The framing: the meet's bytes, then the join's.
        41	    /// assert_eq!(span.encode(), [older.encode(), newer.encode()].concat());

       123	        // The meet is the byte-aligned self-delimiting prefix: parse its tree
       124	        // to find the split and check its padding. The join's admission walk

Resolution: `lo`/`hi` (or "the lower endpoint") in wire.rs and in the span/tests.rs genre strings; reserve meet/join for the operators and for `Version::span`'s derivation. Acceptance: `grep -n 'the meet\|the join' crates/before/src/span/wire.rs` returns nothing.

### span-causally-21: `Span::encode` builds the composite asymmetrically and reallocates once
- Where: crates/before/src/span/wire.rs:44-48 (related: crates/before/src/version.rs:1027-1029; crates/before/src/meter/board/ops.rs:321-342)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read version.rs:1027-1029: `pub fn encode(&self) -> Vec<u8> { self.as_bytes().to_vec() }`); executed: no
- Seen by: structure, claims; refutation: confirmed (one fix for both reports); history: no rationale found (byte-identical since 6355adde)
- Owner-gated: no

`self.lo.encode()` is `as_bytes().to_vec()`, a `Vec` sized exactly to `lo`; `extend_from_slice(hi.as_bytes())` then grows it, copying `lo`'s bytes a second time, and one endpoint goes through a different door than the other. The doc says "the two concatenate with no length prefix"; `concat` says exactly that and sizes the allocation once.

Evidence:

        44	    pub fn encode(&self) -> Vec<u8> {
        45	        let mut bytes = self.lo.encode();
        46	        bytes.extend_from_slice(self.hi.as_bytes());
        47	        bytes
        48	    }

Resolution: `[self.lo.as_bytes(), self.hi.as_bytes()].concat()`. Acceptance: byte-identical output (the wire snapshot and `span_codec_roundtrip` law hold); the `span_encode` board cell's heap reading shows one allocation of the composite size.

### span-causally-22: "Structural genres" in `Span::decode`'s public `# Errors` is undefined at the user's altitude
- Where: crates/before/src/span/wire.rs:91-92 (related: crates/before/src/span/wire.rs:79-90; crates/before/src/error.rs:68-92; crates/before/src/span/tests.rs:448-463)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -ni genre crates/before/src/error.rs crates/before/src/lib.rs` returns nothing; `Decode`'s variant docs never use the word); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (the public sentence inherited the wire module's private vocabulary from 39f5d591's "structural genres win")
- Owner-gated: no

Public rustdoc names nothing the API does not reach: the user reaches `Decode::Truncated`, `Decode::TrailingBits`, and `Decode::NotCanonical`, not a "genre" (crate-internal dialect, 210 occurrences, defined only in a private codec doc). The precedence rule is real and pinned by `span_decode_structural_genres_outrank_the_pair_verdict`; it deserves stating in the user's vocabulary.

Evidence:

        91	    /// On an input defective several ways at once, the components'
        92	    /// structural genres win.

Resolution: "On an input defective several ways at once, a component's own [`Truncated`](Decode::Truncated) or [`TrailingBits`](Decode::TrailingBits) is reported before the pair's [`NotCanonical`](Decode::NotCanonical), exactly as decoding the two components separately would." Acceptance: the public doc names the variants; "genre" appears in wire.rs only in private comments or not at all.

### span-causally-23: The wire decode tests `Admission::Refuted` twice: an early return, then an `unreachable!` arm for the same variant
- Where: crates/before/src/span/wire.rs:156-172 (related: crates/before/src/version/skyline/admit.rs:230-239)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (read the block; `Admission` derives `PartialEq` at admit.rs:230); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (the shape dates from 1af119c1; the padding-before-verdict ORDER is deliberate and load-bearing, 39f5d591, and the rewrite preserves it)
- Owner-gated: no

The block returns `Err(Decode::NotCanonical)` on `admission == Refuted` and hands the full `Admission` out, so the later `match admission` needs a `Refuted => unreachable!(...)` arm whose correctness depends on a check twelve lines earlier. A total match at the decision point states the three verdicts once.

Evidence:

       156	            if admission == skyline::Admission::Refuted {
       157	                return Err(Decode::NotCanonical);
       158	            }
       159	            (lo_bytes, admission)

       172	            skyline::Admission::Refuted => unreachable!("refuted admissions rejected above"),

Resolution: inside the block, after the padding check: `let coincident = match admission { Admission::Equal => true, Admission::Dominates => false, Admission::Refuted => return Err(Decode::NotCanonical) };` returning `(lo_bytes, coincident)`; then `let hi = if coincident { lo.clone() } else { Version::from_frozen(...) };`. Acceptance: no `unreachable!` in wire.rs; `span_decode_rejects_each_genre` and both structural-genre tests stay green.

### span-causally-24: The fused query walks do k sign reads and k probe folds per elementary interval for k live holes; the public contracts and the polarity rationale say "linear time"
- Where: crates/before/src/causally.rs:85-86 (related: crates/before/src/causally/query.rs:26-30, 91-107; crates/before/src/causally/polarity.rs:209-212; crates/before/src/version/skyline/place/filter.rs:37-48, 183-207, 279-297; crates/before-fuelscape/src/ops.rs:1742-1743; crates/before/src/meter/board/ops.rs:1213-1238)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed (read `filter::admits`'s per-interval loop and `MemberCursors::step`; read filter.rs's Cost paragraph; read codec/scan.rs's header for what the scan meter counts); executed: no
- Seen by: claims; refutation: reframed (the k factor is real in time: every live side's pair is read per interval and every probe step folds into every live pair; but the bit-read cost is linear because each stream is decoded once, and `codec::scan` counts cursor advances and decodes, not accumulator reads or folds, so a scan-bits row would pass vacuously; the factor is visible only in the touch currency or wasm fuel); history: no rationale found (filter.rs's Cost paragraph names the per-interval k work and then states a bound without k; no note or declared-model cell records a hole-count model)
- Owner-gated: yes: the contract wording of public API docs, and possibly an algorithm decision

`Query::contains` says "One traversal of `version` and the stored bounds" (true), and the module, `Query`, and `Polarity` docs motivate the polarity restriction as guaranteeing "linear time" (causally.rs:85-86, query.rs:28-30, polarity.rs:211-212). With k holes that never drop (an `Up` hole `!after(h_i)` with `h_i <= v`, or a `Down` hole `since(h_i)` with `v <= h_i`), `filter::admits` reads every live pair's sign on every elementary interval (filter.rs:185-207) and folds every probe step into every live pair (filter.rs:283-285): Θ(k · #intervals) operations with #intervals up to leaves(v) + Σ leaves(bound). That is k times the input in operation count, and quadratic in k when holes dominate; the same holds for `filter::coverage`. filter.rs's own Cost paragraph acknowledges the k folds per probe delta and then states `O(|v| + Σ|bound|)`. The bits-decoded bound is true; the word "time" is not, for k >= 2. Every committed instrument has at most one hole (fuelscape ops.rs:1742-1743; the board's `query_contains` is `after(&lo) & before(&hi)`; the meter's contains rows are `since(&v) & before(&e)`), so nothing sees the k axis. The crate treats asymptotic claims as hard guarantees.

Evidence:

        85	//! Instead, we restrict queries to only those whose verdicts can assuredly be
        86	//! resolved in linear time: those with a uniform *polarity*. We say a [`Query`]

    (causally/query.rs)
        28	/// NP-complete (non-polynomial). The [`Polarity`] restriction enforced by the
        29	/// types of [`Query`] ensures that only linear-time decidable queries are
        30	/// expressible.

    (version/skyline/place/filter.rs, outside the partition)
        39	//! Derived, by the placement walk's argument stream by stream: every topology
        40	//! bit of every stream read at most once, every leaf payload decoded once and
        41	//! folded into at most one accumulator per pair it participates in — the
        42	//! probe's deltas into each live bound's pair, a bound's deltas into its own —
        46	//! bookkeeping absorbed by the same per-interval read loop. `O(|v| + Σ|bound|)`

Resolution: owner decision, then wording. (a) Restate the contracts with the hole-count factor: "linear in the bits decoded (each stream once); per-interval work proportional to the number of live holes, `O(k · (|probe| + |self|))` in operations", and replace "linear time" at causally.rs:85-86, query.rs:28-30, polarity.rs:211-212 with the actual payoff of the polarity restriction (a polynomial decision procedure with an exact verdict; the SAT reduction is span-causally-33). (b) Keep the linear promise, which needs an indexed per-hole design. In both cases add a k-scaling instrument in the touch (accumulator) currency or fuel, never scan bits. Also correct filter.rs:46-48 (outside this partition). Acceptance: a touch-currency row with probe `v` fixed and `Q_k = until(&h_1) & ... & until(&h_k)` at k in {8, 64}; under (a) it pins the ratio ~8 as the declared model; under (b) it asserts the difference is bounded by the added holes' bits and fails today. The public docs no longer say "linear time" without the hole-count clause.

Construction: fork k = 64 parties from one seed and tick each once, giving h_1..h_k pairwise concurrent. Let `v` be the join of all h_i followed by ~10^4 further received sends from fresh forks (n ~ 10^4 plateaus). Build `q = until(&h_1) & ... & until(&h_k)` (Up polarity, an antichain of k holes; every side's demand is `NotAfter` with `h_i <= v`, so filter.rs:201-204 never drops a side before exhaustion). `q.contains(&v)` is false and the walk runs ~n + k log k elementary intervals reading k pairs each: ~64 · 10^4 sign reads and probe folds against ~10^4 + 64 log 64 packed bits of input. Halving k halves the touch (accumulator digit) reading at fixed `v`, which `O(|self| + |version|)` forbids; a scan-bits reading is the same at both k.

### span-causally-25: The module-level `# Complexity` says every pass is linear; the conjoin island it links declares `O(|self| · |rhs|)`
- Where: crates/before/src/causally.rs:101-106 (related: crates/before/src/causally/conjunction.rs:38-68, 148-163; crates/before-fuelscape/src/ops.rs:2146-2147)
- Class / severity / confidence: claim / low / high
- Provenance: verified (read the committed island contract at before-fuelscape/src/ops.rs:2146: "linear, plus one comparison per opposite-side hole pair: `O(|self| · |rhs|)` at worst", claim "n^2"; read `and()`'s k survive checks and up to k·m absorption comparisons); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (db9dfa3e's Complexity section priced conjunction as "one lattice walk per floor/ceiling merge and one causal comparison per hole pair" and coverage's clamp walks separately; a6dcfbb4 and 3bba6cbb trimmed it to the blanket sentence)
- Owner-gated: no

Two statements of the same crate disagree about the same operation: the module summary tells the reader every pass and walk is linear, while the conjunction island rendered on every `&` impl declares a hole-pair product. The walk clauses of this paragraph are settled by span-causally-24 and span-causally-36; this finding is the conjunction clause and the paragraph's structure.

Evidence:

       101	//! # Complexity
       102	//!
       103	//! Atoms and named constructors are `O(1)`.
       104	//!
       105	//! Each pass and walk is linear in its operands' sizes in bytes and
       106	//! stops as soon as its verdict is decided.

    (before-fuelscape/src/ops.rs)
      2146	        contract: "linear, plus one comparison per opposite-side hole pair: `O(|self| · |rhs|)` at worst",
      2147	        claim: "n^2",

Resolution: rewrite the paragraph as three clauses once span-causally-24 and -36 are settled: atoms and named constructors `O(1)`; membership and coverage as fused walks over the probe(s) and every bound (with the hole-count factor the owner chooses); conjunction linear in the bounds plus one comparison per cross-side hole pair. Acceptance: the module summary, the conjoin island contract, and the `contains`/`coverage` islands agree on the hole-count dependence.

Construction: none needed beyond reading; the two doc strings contradict each other on their face.

### span-causally-26: Production `<=` checks sweep through `partial_cmp`, losing the one-direction early exit `sweep::le` already implements; the coincident fast rungs can read more than the walk they replace
- Where: crates/before/src/causally.rs:161-168 (related: crates/before/src/span.rs:315-326, 380-391, 453-476; crates/before/src/causally/polarity.rs:69-107, 127-163; crates/before/src/causally/forms.rs:283-285, 307-309; crates/before/src/causally/query.rs:161; crates/before/src/version/skyline/sweep.rs:156-186, 233-242; crates/before/src/version/skyline/place.rs:253-267; crates/before/src/version/skyline/place/filter.rs:189-194)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (grep: `sweep::le` has no caller outside sweep.rs; read sweep.rs:169-170, the `#[cfg(any(test, feature = "meter"))]` gate on `le`, and sweep.rs:236-242, `order_exit`, which breaks only when both directions are refuted; read place.rs:261-267, where `dominance` breaks `Before` at the first interval refuting `lo <= probe`); executed: no
- Seen by: claims; refutation: confirmed (adds two corollaries: `Floor::contains`/`Ceiling::contains` are slower than `Query::from(after(s)).contains(v)` on a refuted relation, since `filter::admits` returns at the refuting interval; and causally.rs:106's "stops as soon as its verdict is decided" is breached by `le`/`lt` on their own); history: no rationale found (30759af0 gated `le` as dead-code hygiene because nothing in production called it; "production ordering goes through the `PartialOrd` surface" describes that state, not a design decision)
- Owner-gated: no

`le`/`lt` are `partial_cmp` matches, i.e. `causal_cmp` with `order_exit`, which stops early only when both directions are refuted (concurrency). A one-direction question `a <= b` is decided the moment `le` is refuted, and `sweep::le` implements exactly that exit, but it is test/meter-only. Every production `<=` in this partition therefore sweeps to exhaustion whenever `a > b` strictly: `Floor::contains`/`Ceiling::contains`, `hole_subtracts`/`hole_survives`/`absorbs`, `refine_partial`'s clamp check, `Span::contains`'s span-argument arm, and the coincident rungs of `Span::dominance`/`precedence`. The last is an inversion: on `Span::at(&hi)` with `hi > probe` strictly, the `ptr_eq` rung runs `hi.partial_cmp(probe)` to exhaustion while `place::dominance` on the same operands in distinct buffers breaks at the first refuting interval; the fast path is slower than the walk it replaces on a reachable input, contradicting its stated purpose (span.rs:309-311). A `le`-exit sweep does at most `causal_cmp`'s work on every input and strictly less whenever the watched direction is refuted before exhaustion (fixed sign). A narrower corollary at span.rs:453-476: with a coincident receiver and a non-coincident span argument, `[v, v]` contains `[a, b]` only when `a == b == v`, so two `canonical_eq` byte compares answer where the two sweeps run to exhaustion (equality confirms only at exhaustion).

Evidence:

       161	/// `a <= b` under the causal order.
       162	fn le(a: &Version, b: &Version) -> bool {
       163	    matches!(a.partial_cmp(b), Some(Ordering::Less | Ordering::Equal))
       164	}
       165	
       166	/// `a < b` under the causal order.
       167	fn lt(a: &Version, b: &Version) -> bool {
       168	    a.partial_cmp(b) == Some(Ordering::Less)
       169	}

    (version/skyline/sweep.rs)
       167	/// Test- and meter-only: production ordering goes through the `PartialOrd`
       168	/// surface over [`causal_cmp`].
       169	#[cfg(any(test, feature = "meter"))]
       170	pub fn le(a: BitsView<'_>, b: BitsView<'_>) -> bool {

       236	pub(super) fn order_exit(directions: Directions) -> ControlFlow<Option<Ordering>> {
       237	    if !directions.le && !directions.ge {
       238	        ControlFlow::Break(None)
       239	    } else {
       240	        ControlFlow::Continue(())
       241	    }
       242	}

    (span.rs, the coincident dominance rung)
       318	            return if matches!(
       319	                self.hi().partial_cmp(version),
       320	                Some(Ordering::Less | Ordering::Equal)
       321	            ) {

Resolution: lift the cfg gate on `sweep::le` (adding the `ptr_eq` reflexivity rung `causal_cmp` has), add a sibling `lt` (same exit, finish `directions.le && !directions.ge`), expose `pub(crate) fn Version::le/lt` over `self.0.live()`, and route `causally::le`/`lt`, the two coincident rungs in span.rs, and `Span::contains`'s span arm through them; `absorbs` may keep `partial_cmp` where it needs the `Equal`/`Less` distinction. Optionally answer the coincident-receiver/span-argument case with two `canonical_eq` compares. Acceptance: two relational `scan-meter` rows in tests/meter.rs's `placement` module: (1) `Span::at(&hi).dominance(&probe)` with `probe < hi` strictly and `hi` extending far past the first refuting interval scans no more than `Span::new(&hi, &hi_redecoded).dominance(&probe)` (today it scans strictly more); (2) `after(&s).contains(&v)` with `v < s` scans strictly less than `s.partial_cmp(&v)` and equals the fused `filter::admits` reading on the same pair. No constants.

Construction: the meter fixture (tests/meter.rs:9492-9529): `s < v < e` with a 2^80-tick plateau between `s` and `v`. `Span::at(&e).dominance(&s)` (the `ptr_eq` rung) versus `Span::new(&e, &Version::decode(&e.encode()[..]).unwrap()).dominance(&s)` (the fused walk). Wrap each in `meter::reset_scan_bits()`/`meter::scan_bits()`. The rung's `causal_cmp(e, s)` refutes `le` at the plateau and never refutes `ge`, so it sweeps both streams to the end; the fused walk breaks `Dominance::Before` at the plateau.

### span-causally-27: Banned vocabulary: "mint"/"minting" for constructing a hole
- Where: crates/before/src/causally/conjunction.rs:29-37 (related: crates/before/src/party.rs:15, 872)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rni '\bmint'` over the thirteen partition files returns exactly conjunction.rs:30 and :37); executed: no
- Seen by: prose; refutation: confirmed; history: already-known (2c73d032 purged the word from the rumors crate as an owner ruling; the before crate was not swept, and these two sites are the owner's own text from 20c0515a, written before the ruling)
- Owner-gated: no

The review standard names "mint" as never to be written for constructing a value, and the ruling on record already removed it from the sibling crate.

Evidence:

        29	    /// Each operand's holes are already a pairwise-unabsorbed antichain
        30	    /// (constructors mint at most one hole; every multi-hole query came
        31	    /// through this merge), so same-side pairs are never compared — only the

        36	    /// fall into rides through inert, subtracting nothing on every path,
        37	    /// rather than minting a corner case here.

Resolution: line 30: "(constructors add at most one hole; ...)"; line 37: "rather than introducing a corner case here." Acceptance: `grep -rni '\bmint' crates/before/src/causally crates/before/src/span` returns nothing (party.rs is outside this partition).

### span-causally-28: No enforced instrument prices `Query::and` under a growing hole count; the rendered island's `n^2` claim describes a regime its one-hole operands never enter, and the survive filter re-reads the floor once per hole
- Where: crates/before/src/causally/conjunction.rs:38-68 (related: crates/before/src/causally/conjunction.rs:148-163; crates/before/src/causally/polarity.rs:81-107; crates/before-fuelscape/src/ops.rs:160-178, 2130-2157; justfile:675)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the panel spec: both operands `strictly_after(a) & before(c)`, one hole each, contract `O(|self| · |rhs|)`, claim `n^2`; read justfile:675 "audit view; not enforcement" and ops.rs:176-177 "The envelope suite and the fuzz-fit bands own worst-case enforcement"; grep of tests/meter.rs and fuzzfit bands finds no conjunction row); executed: no
- Seen by: claims; refutation: reframed (the fuelscape is not an enforcement instrument, so the gap is that nothing enforced prices `and()` under a growing hole count, and the island's contract describes a regime its operands cannot enter); history: deliberate-and-holds for the label (2efff149's rule: a claim is an O upper bound, "O already bounds above", so the `n^2` stamp over linear data is honest under that rule); the enforcement gap stands
- Owner-gated: yes: adding an enforced row is a gate-policy addition, and the island's shape is the owner's

`and()` performs k `hole_survives` checks (each a separate `le(floor, hole_i)` sweep re-reading `floor`: O(k · |floor|), where a fused per-bound walk would be O(|floor| + Σ|holes|)) plus up to k·m `absorbs` comparisons. The only cost content on the twenty-one `&` impls is an island whose operands carry one hole each, so the measured merge does exactly one cross-side comparison regardless of size, and no enforced instrument (tests/meter.rs, board, fuzzfit) prices the k·m term at all. An instrument that pins a claim must be able to fail when the claim is false.

Evidence:

        47	        let survives =
        48	            |hole: &Hole<'a>| P::hole_survives(hole, floor.as_deref(), ceiling.as_deref());
        49	        let mut kept: Vec<Hole<'a>> = self.holes.into_iter().filter(survives).collect();
        50	        let mut added: Vec<Hole<'a>> = Vec::new();
        51	        for hole in other.holes {
        52	            if !survives(&hole) {
        53	                continue;
        54	            }
        55	            if kept.iter().any(|held| P::absorbs(held, &hole)) {
        56	                continue;
        57	            }
        58	            kept.retain(|held| !P::absorbs(&hole, held));
        59	            added.push(hole);
        60	        }
        61	        kept.append(&mut added);

    (before-fuelscape/src/ops.rs)
      2139	        size_measure: "total packed bytes of the two strict-lower bounds and the \
      2140	             two ceilings, split uniform four ways (each operand composed in \
      2141	             unmeasured preparation as strictly_after(a) & before(c) — \

Resolution: add an enforced row (a `scan-meter` or touch row in tests/meter.rs) whose operands carry k pairwise-concurrent holes with k scaling, declaring the k·m model at the cell; either add a many-hole fuelscape variant or relabel the existing island's contract to the one-hole shape it samples. Optionally fuse the survive filter through `filter::admits` with per-bound verdicts. Acceptance: an enforced row exists whose reading quadruples when k doubles at fixed bound size, and the island's contract describes what its operands measure.

Construction: `A_k = since(&a_1) & ... & since(&a_k)` and `B_k = since(&b_1) & ... & since(&b_k)` over 2k pairwise-concurrent one-tick versions; `A_k & B_k` performs k·k `absorbs` comparisons (lines 55, 58) plus 2k survive checks. Doubling k quadruples the comparison count at fixed bound size; the committed panel never varies k.

### span-causally-29: `conjoin!` gives every `&` cell one doc line and the hole-bearing island, including the seven hole-free atom cells
- Where: crates/before/src/causally/conjunction.rs:150-154 (related: crates/before/src/causally/conjunction.rs:71-101, 165-192)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read the macro and its twenty-one rows); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds (20c0515a, the owner, replaced a longer generic doc with "Conjunction of [`Query`]s." plus one complexity sentence; 2efff149 swapped the sentence for the island); the rationale is not stated in code
- Owner-gated: yes: the owner's chosen doc shape

`Floor & Ceiling` (two atoms, no holes, output `Neutral`) shows a hole-cost chart and a doc that does not say what it produces, while `Floor & Floor` and `Ceiling & Ceiling` get precise docs and their own islands. A user hovering `Floor & Query<Down>` wants the output polarity and the rule ("the atom adopts the holed operand's polarity"), which the macro's comment lines already carry (166, 174-175, 183-184) but its emitted docs do not; for the hole-free cells the chart is an upper bound rather than their measurement.

Evidence:

       150	        #[doc = "Conjunction of [`Query`]s."]
       151	        #[doc = ""]
       152	        #[doc = "# Complexity"]
       153	        #[doc = ""]
       154	        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_conjoin_bounded_holes.html"))]

Resolution: let the macro take a per-group doc string (atom x atom, atom x polar, polar x polar) so each cell states its output polarity, and let the hole-free cells cite the `query_conjoin_floors`/`ceilings` class charts or say "hole-free: the bounds join and meet". Acceptance: `cargo doc` for `Floor`'s trait impls names the output polarity per cell.

### span-causally-30: `Floor`/`Ceiling` public docs state their predicate with the private field name `at`
- Where: crates/before/src/causally/forms.rs:19-32 (related: crates/before/src/causally/forms.rs:278, 302; crates/before/src/causally.rs:15-18)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read; `at` is `pub(super)` at forms.rs:25 and 31; the module table at causally.rs:15-18 writes `p <= v` and `v <= e`); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (type docs from a6dcfbb4, `contains` docs from db9dfa3e)
- Owner-gated: no

Public rustdoc names nothing the API does not reach; `at` is a field the user cannot see, and the constructors' docs and the module table already use `s`/`p` and `e`.

Evidence:

        19	/// Built by [`after`]: keeps the versions at or above its bound, `at <= v`.

        28	/// Built by [`before`]: keeps the versions at or below its bound, `v <= at`.

       278	    /// Whether `version` is at or above the bound: `at <= v`.
       302	    /// Whether `version` is at or below the bound: `v <= at`.

Resolution: `s <= v` / `v <= e` (matching `after(s)` / `before(e)`), or "bound <= v". Acceptance: no public doc in forms.rs mentions `at` outside code.

### span-causally-31: `Query` is built by sixteen struct literals across four files; the one-hole forms and the `Conjoin` lifts restate constructors that exist
- Where: crates/before/src/causally/forms.rs:288-298 (related: crates/before/src/causally/forms.rs:103-148, 312-358; crates/before/src/causally/conjunction.rs:108-128; crates/before/src/causally/convert.rs:13-34; crates/before/src/causally/query.rs:63-70, 200-208)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -c 'polarity: PhantomData'` gives forms 6, conjunction 3, convert 2, query 5: sixteen production sites); executed: no
- Seen by: structure; refutation: confirmed (corrects the count from fifteen to sixteen); history: no rationale found
- Owner-gated: no

The four one-hole constructors (`Floor::or_concurrent`, `Ceiling::or_concurrent`, `Not for Ceiling`, `Not for Floor`) write the identical eight-line literal differing only in `strict`; `strictly_after`/`strictly_before` add a bound to it; `Conjoin::lift` for `Floor`/`Ceiling` duplicates the `From<Floor>`/`From<Ceiling> for Query<Neutral>` impls, which `adopt()` already lifts to any polarity. The normal-form invariant "a neutral query holds no holes", which six `unreachable!` arms in polarity.rs rest on, is today enforced only by every literal happening to agree.

Evidence:

       288	    pub fn or_concurrent(self) -> Query<'a, Down> {
       289	        Query {
       290	            floor: None,
       291	            ceiling: None,
       292	            holes: vec![Hole {
       293	                at: self.at,
       294	                strict: true,
       295	            }],
       296	            polarity: PhantomData,
       297	        }
       298	    }

Resolution: private constructors in query.rs beside `unbounded()`: `fn from_hole(hole: Hole<'a>) -> Self` and `fn bounded(floor, ceiling) -> Self`; `or_concurrent` = `Query::from_hole(Hole { at: self.at, strict: true })`, `Not` = `Query::from_hole(Hole { at: self.at, strict: false })`, the strict forms set the bound on the result, and `Conjoin::lift` for `Floor`/`Ceiling` = `Query::<Neutral>::from(self).adopt()`. Acceptance: the `PhantomData` count drops to the genuine constructors (unbounded, from_hole/bounded, clone, into_owned, adopt, and); `forms_keep_their_relations`, `conjunction_normalizes`, `debug_renders_expressions`, and the conjunction laws stay green.

### span-causally-32: `Hole.strict` and the `hole_demand`/`hole_name` dispatch take a bare `bool` for a two-valued domain concept
- Where: crates/before/src/causally/polarity.rs:19-22 (related: crates/before/src/causally/polarity.rs:40, 57, 61-67, 70-74, 88-92, 119-125, 128-132, 146-150; crates/before/src/causally/forms.rs:105-108, 292-295, 316-319, 334-337, 352-355; crates/before/src/causally/query.rs:187-190)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

At eight construction sites a reader must know that `strict: false` means the hole subtracts the inclusive set and `strict: true` the strict one; six `if strict { .. } else { .. }` pairs in the `Down`/`Up` impls would become matches on a named variant. Types-first: `Hole { at, bound: Bound::Inclusive }` reads without the polarity table at hand.

Evidence:

        19	pub(super) struct Hole<'a> {
        20	    pub(super) at: Cow<'a, Version>,
        21	    pub(super) strict: bool,
        22	}

        40	        fn hole_demand(strict: bool) -> Demand;
        57	        fn hole_name(strict: bool) -> &'static str;

Resolution: `pub(super) enum Bound { Inclusive, Strict }` in polarity.rs; the `if strict` pairs become matches. Acceptance: no `bool` in `Hole` or the `Sealed` signatures; tests unchanged.

### span-causally-33: The SAT/NP-completeness motivation is stated three ways, once logically inverted, never argued
- Where: crates/before/src/causally/query.rs:26-30 (related: crates/before/src/causally.rs:78-83; crates/before/src/causally/polarity.rs:209-212)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read the three sites); executed: no
- Seen by: prose; refutation: confirmed (adds a correct sketch: monotone SAT reduces to mixed-polarity emptiness); history: no rationale found (the inverted direction and the "footgun"/"famously"/"non-polynomial" register are the owner's own words from a6dcfbb4; the text they replaced, db9dfa3e's "deciding whether it empties a span encodes satisfiability", had the correct direction)
- Owner-gated: no (doc accuracy), but the text is owner-authored

The public `Query` doc says deciding span overlap for arbitrary queries "reduces to the SAT problem, and is therefore NP-complete (non-polynomial)": a problem that reduces to SAT is in NP (an upper bound), not thereby NP-complete, and NP-complete does not mean non-polynomial. causally.rs:79-83 says "equivalent to", polarity.rs:209-212 "reduces to", and none of the three gives the reduction; three paraphrases that disagree in strength are the tell that none is the statement of record. A hardness claim in public docs is a claim like any other and needs its argument. The causally.rs site also carries register transplants ("powerful hidden footgun", "famously", "silently exponential") where no adversary exists, and the typo of span-causally-5.

Evidence:

        26	/// Not all queries may be combined, because exactly deciding [`Span`]-overlap
        27	/// for entirely arbitrary queries reduces to the SAT problem, and is therefore
        28	/// NP-complete (non-polynomial). The [`Polarity`] restriction enforced by the
        29	/// types of [`Query`] ensures that only linear-time decidable queries are
        30	/// expressible.

    (causally.rs)
        79	//! `&`, permitting this carries a powerful hidden footgun: exactly deciding the
        80	//! [`coverage`](Query::coverage) of a [`Span`] against a freely constructed
        81	//! [`Query`] with arbitary negation is equivalent to the famously NP-complete
        82	//! SAT problem: exposing this interface would make it easy to express silently
        83	//! exponential queries.

Resolution: state the claim once, in causally.rs's Polarity section, in the honest direction with a one-line sketch. The refutation pass's sketch: SAT (in its monotone form, every clause all-positive or all-negative, still NP-complete) reduces to mixed-polarity emptiness: take one region per variable and the span `[⊥, all ones]`; an all-positive clause is one `Down` hole `v <= h` with `h` zero exactly on the clause's regions (it subtracts the assignments violating the clause), an all-negative clause one `Up` hole `h' <= v` with `h'` one exactly on its regions; the clamp minus the holes is nonempty iff the formula is satisfiable, so exact `coverage` for a mixed query is at least as hard as SAT. Then make query.rs:26-30 and polarity.rs:209-212 one sentence each linking there; replace "footgun"/"famously"/"silently exponential" with the mechanism; align "linear time" with span-causally-24. Acceptance: exactly one site states the hardness claim with its direction and argument; `grep -rn 'NP-complete\|SAT problem' crates/before/src` returns one definitional hit plus links; "non-polynomial" is gone.

### span-causally-34: `Coverage`'s docs do not state exactness, and `laws.rs` cites a precision contract on them that does not exist and describes an incompleteness the code does not have
- Where: crates/before/src/causally/query.rs:50-59 (related: crates/before/src/causally/query.rs:143-171; crates/before/src/laws.rs:1029-1039; crates/before/src/causally/tests.rs:222-323)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `Coverage`'s docs in full, which state only the three verdicts' meanings; read `refine_partial`'s doc, "the exact emptiness decision"; read `coverage_is_exact_on_the_two_party_grid`, which asserts every verdict equals the brute-force census; `git show db9dfa3e:crates/before/src/causally/query.rs` carries "The verdict is **exact** for every constructible query" at its lines 61-62); executed: no
- Seen by: correctness, claims; refutation: confirmed (re-derived exactness: for `Down` the clamp is nonempty iff `clamped_lo <= clamped_hi`, and then `clamped_hi` is admitted iff no hole subtracts it, since every `v` in the clamp has `v <= clamped_hi <= hole.at`; dually for `Up`); history: no rationale found (a6dcfbb4 deleted the exactness sentence from `Coverage` and from `coverage`'s doc; b3f09baa introduced the laws.rs sentence the next day citing a precision contract that never existed anywhere in the tree and asserting an incompleteness the code has never had)
- Owner-gated: no

A user reading `Coverage`'s rustdoc cannot tell whether `Partial` means "genuinely mixed" or "undecided"; the code makes it the former (the clamp refinement exists to buy exactly that), and the docs are silent. The `coverage_bounds_membership` law's doc says "the [`Coverage`] docs carry the precision contract, including why `Empty` cannot be complete", which is a ghost reference in both halves: no such text exists, and `Empty` is complete. Prose speaks in the present tense.

Evidence:

        50	/// How much of a [`Span`]'s segment a [`Query`] admits.
        51	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        52	pub enum Coverage {
        53	    /// Every version the span covers is admitted by the query.
        54	    Full,
        55	    /// Some covered versions are admitted and some are not.
        56	    Partial,
        57	    /// No version the span covers is admitted by the query.
        58	    Empty,
        59	}

    (laws.rs)
      1036	    /// arms are exercised against genuinely interior points. `Partial` promises
      1037	    /// nothing pointwise: the [`Coverage`] docs carry the precision contract,
      1038	    /// including why `Empty` cannot be complete.

Resolution: restore one sentence on `Coverage` (db9dfa3e's "The verdict is exact for every constructible query" is a restoration candidate; today's wording: "Each verdict is exact: `Partial` means at least one covered version is admitted and at least one is not"); rewrite laws.rs:1036-1038 to say the law pins soundness and that completeness is pinned by `coverage_is_exact_on_the_two_party_grid`. Acceptance: `grep -rn 'cannot be complete' crates/before/src` returns nothing; `Coverage`'s rustdoc states exactness; both agree with `refine_partial`'s doc.

### span-causally-35: `Query::coverage`'s clone-identity rung has no agreement test across buffer identity
- Where: crates/before/src/causally/query.rs:129-135 (related: crates/before/src/span/tests.rs:812-844; crates/before/tests/coincident_span.rs:1-11; crates/before/src/laws.rs:1074-1099, 1663-1670; crates/before/src/causally/tests.rs:295-299; crates/before/src/version/skyline/place/tests.rs:475-506; crates/before/src/version/skyline/place/filter.rs:477-515)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (every coincident span reaching `Query::coverage` in committed tests shares one buffer: laws.rs:1665 builds the coincident candidate as `(meet.clone(), meet)`; laws.rs:1088 uses `Span::at(p)`; the two-party grid builds `Span::new(lo, hi)` from the same `&grid[i]`; tests/coincident_span.rs pins `Span::place`, `Span::dominance`, and the `contains` argument rung by scan parity and never calls `coverage`; `filter_coverage_matches_the_composed_sweeps` is stream-level and never reaches `refine_partial`); executed: no
- Seen by: correctness; refutation: confirmed (reading filter.rs:477-515 shows a byte-equal `(lo, hi)` can never yield `Partial`, so the rung is equivalent today and only the pin is missing); history: no rationale found (the span rungs' cross-buffer pin was added deliberately in 4874f527; db9dfa3e names no such pin for the query rung)
- Owner-gated: no

The rung answers `Full`/`Empty` from `contains(lo)`; a byte-equal coincident span in distinct buffers takes `filter::coverage` and possibly `refine_partial`. The span module pins its analogous rungs with `coincident_span_rungs_agree_across_buffer_identity` and the scan-parity tests; the query module has no twin, so the step "`refine_partial` is never reached on a coincident span" lives in the reader's head. A future change to `refine_partial` (including the fix for span-causally-36) that made a coincident distinct-buffer span read `Partial` would pass every current test.

Evidence:

       129	        if lo.view().ptr_eq(hi.view()) {
       130	            return if self.contains(lo) {
       131	                Coverage::Full
       132	            } else {
       133	                Coverage::Empty
       134	            };
       135	        }

Resolution: a proptest beside `coverage_matches_membership_on_points` (or in causally/tests.rs) over `neutral_queries`/`down_queries`/`up_queries` asserting `q.coverage(Span::new(&v, &redecoded).unwrap()) == q.coverage(Span::at(&v))` with `redecoded = Version::decode(&v.encode()[..]).unwrap()`, mirroring `span_contains_matches_place`'s redecoded-argument leg. Acceptance: the property runs in `just test-all`; deleting the `ptr_eq` rung leaves it green (equivalence); forcing `refine_partial` to return `Partial` on a coincident clamp makes it red.

Construction: for any `v` and query `q`: `let w = Version::decode(&v.encode()[..]).unwrap(); assert_eq!(q.coverage(Span::new(&v, &w).unwrap()), q.coverage(Span::at(&v)))`. Today this passes; it is the missing pin, not a failing case.

### span-causally-36: `Query::coverage`'s `Partial` refinement sweeps the clamped endpoint once per hole: Θ(k·|hi|) against a published `O(|self| + |span|)`, with no multi-hole instrument
- Where: crates/before/src/causally/query.rs:152-171 (related: crates/before/src/causally/query.rs:114-117; crates/before/src/causally.rs:105-106; crates/before/src/causally/polarity.rs:69-79, 127-137; crates/before/src/causally.rs:161-168; crates/before/src/version/skyline/sweep.rs:233-242; crates/before/src/version/skyline/place/filter.rs:477-515; crates/before-fuelscape/src/ops.rs:1740-1746, 1985-1993; crates/before/src/meter/board/ops.rs:1239-1266; justfile:675)
- Class / severity / confidence: claim / high / high
- Provenance: assessed for the cost trace (read `refine_partial`, `hole_covers` -> `hole_subtracts` -> `le`/`lt` -> `partial_cmp` -> `causal_cmp` with `order_exit`, and `filter::finish`); verified for the instrument census (fuelscape ops.rs:1742-1743 "with at most one hole" and every coverage panel one hole; board `query_coverage` is `delta(&v, &w)`; grep of tests/meter.rs for `.coverage(` finds no row; fuzzfit has no query band); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (traced the construction below; notes the fuelscape is audit-only, so the failing witness must be a tests/meter.rs row or board cell, and that a fused replacement is linear in bits decoded but keeps the per-interval hole factor of span-causally-24); the prose lens's construction was refuted (a version where every party ticked once normalizes to a single leaf) and is replaced by the claims lens's; history: no rationale found (the per-hole loop's cost was never priced: db9dfa3e's module doc priced coverage's extra work as "two further lattice walks", the clamps, never a per-hole comparison; a6dcfbb4 and 3bba6cbb trimmed that to a blanket linear sentence; `refine_partial`'s doc argues exactness, not cost)
- Owner-gated: no

The public `# Complexity` on `coverage` promises "At most two traversals of the span's endpoints and the stored bounds", the seven islands state `O(|self| + |span|)`, and the module doc says every pass is linear in operand bytes. `refine_partial` then evaluates `self.holes.iter().any(|hole| P::hole_covers(hole, ...))`, and each `hole_covers` is `hole_subtracts(hole, clamped_hi)` (`Down`) or `(hole, clamped_lo)` (`Up`): a full `partial_cmp` sweep of the clamped endpoint against that hole's bound, exiting early only on concurrency (`order_exit`). When the clamped endpoint strictly dominates every hole (the shape a `Partial` verdict with many satisfied holes produces), each sweep runs both streams to exhaustion, so the refinement re-decodes the clamped endpoint k times: Θ(k · |hi| + Σ|hole_i|) on the `Down` `Partial` path, dually for `Up`. Holes survive absorption whenever pairwise concurrent, and k is unbounded (an anti-entropy query conjoining `since(v_i)` for many concurrent peers is exactly this shape). The exactness argument at 143-151 is correct; only the cost is wrong. The crate docs make every asymptotic claim a hard guarantee, and a claim needs an argument, a matching implementation, and a committed instrument that fails when it is false; here the implementation does not match and no instrument can see it.

Evidence:

       114	    /// # Complexity
       115	    ///
       116	    /// At most two traversals of the span's endpoints and the stored
       117	    /// bounds (`|self|`, their total size); one chart per bounds shape:

       161	        if !le(&clamped_lo, &clamped_hi)
       162	            || self
       163	                .holes
       164	                .iter()
       165	                .any(|hole| P::hole_covers(hole, &clamped_lo, &clamped_hi))
       166	        {
       167	            Coverage::Empty
       168	        } else {
       169	            Coverage::Partial
       170	        }

    (polarity.rs)
        77	        fn hole_covers(hole: &Hole<'_>, _clamped_lo: &Version, clamped_hi: &Version) -> bool {
        78	            Self::hole_subtracts(hole, clamped_hi)
        79	        }

    (before-fuelscape/src/ops.rs)
      1742	    // One contains and one coverage panel per stored-bound shape a public
      1743	    // constructor can produce with at most one hole. The query is composed
      1992	        contract: "`O(|self| + |span|)`",

Resolution: instruments before cures. First land a deterministic `scan-meter` row in tests/meter.rs's `placement` module shaped as below, measured at (k, |hi|), (2k, |hi|), (k, 2|hi|), asserting the marginal cost of doubling |hi| is independent of k (within the crate's slack); commit it red. Then replace the per-hole loop with one fused membership walk over the polarity's deciding clamp end: for `Down`, `Empty <=> !filter::admits(clamped_hi.view().live(), self.holes.iter().map(|h| (h.at.view().live(), P::hole_demand(h.strict))))` (with the crossed-clamp test kept as `le(clamped_lo, clamped_hi)`, or reduced to `!le(floor, ceiling)` since `floor <= hi` and `lo <= ceiling` are established by the fused walk returning `Partial`); dually `clamped_lo` for `Up`; a new sealed method naming which clamp end decides replaces `hole_covers` (its only caller). `admits` returns false exactly when some hole's subtraction holds on the probe (filter.rs:221-230), which is the `any(hole_covers)` predicate. Then restate the `# Complexity` text to the traversals the code performs (the fused walk's own hole factor is span-causally-24). Add a many-hole fuelscape variant for the rendered island. Acceptance: the meter row goes green; the doc says what the code does; `coverage_is_exact_on_the_two_party_grid`, `coverage_clamp_refinement_is_exact`, and the new pin of span-causally-35 stay green.

Construction: fork k = 64 parties from `Clock::seed()` and tick each once: h_1..h_k are pairwise concurrent. `q = since(&h_1) & ... & since(&h_k)` (`Down`; `and()` keeps all k as an antichain since no pair compares; `rendered_holes` in causally/tests.rs reads k). `hi` = the join of all h_i followed by many more received sends on further forked parties (n >> k log k plateaus), so `hi > h_i` strictly for every i; `span = Span::new(&Version::new(), &hi)`. `filter::coverage` returns `Partial` (filter.rs:487: `hi`'s relation to each hole is `Greater`, so no hole empties; filter.rs:503: `lo`'s relation is `Less`, so `admits_all` is false). `refine_partial` then computes `le(hi, h_i)` for each i; `causal_cmp(hi, h_i)` refutes `le` early and never refutes `ge`, so `order_exit` never breaks and each sweep reads all of `hi` and all of `h_i`. Wrap `q.coverage(span.reborrow())` in `meter::reset_scan_bits()`/`meter::scan_bits()` at k = 8 and k = 64 with the same `hi`: the difference is ~56 · |hi| bits, where the contract predicts a difference bounded by the added holes' own bits. `q.contains(&hi)` on the same query is the linear control (one fused walk).

### span-causally-37: `Query::into_owned` is documented `O(1)` but rebuilds the hole `Vec`
- Where: crates/before/src/causally/query.rs:176-194 (related: crates/before/src/causally/query.rs:38; crates/before/src/causally/convert.rs:36)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read the body); executed: no
- Seen by: prose, claims; refutation: confirmed; history: no rationale found (db9dfa3e read "`O(1)` per stored bound"; a6dcfbb4 dropped "per stored bound")
- Owner-gated: no

The body allocates a fresh `Vec` and maps every hole into it: O(#holes) refcount bumps plus one allocation, and the hole count is unbounded. The `Clone` impl's doc (line 38, "`O(1)` per bound") and convert.rs:36 ("`O(1)` per stored bound") have the accurate denominator; `into_owned` dropped it. Asymptotic claims are hard guarantees in this crate.

Evidence:

       176	    /// # Complexity
       177	    ///
       178	    /// `O(1)`: owned versions move, borrowed ones clone by sharing their stored
       179	    /// buffers.

       184	            holes: self
       185	                .holes
       186	                .into_iter()
       187	                .map(|hole| Hole {
       188	                    at: Cow::Owned(hole.at.into_owned()),
       189	                    strict: hole.strict,
       190	                })
       191	                .collect(),

Resolution: "`O(1)` per stored bound (one allocation for the hole list): owned versions move, borrowed ones clone by sharing their stored buffers." Alternatively settle the holes in place (`for hole in &mut holes { hole.at = Cow::Owned(...) }`) and keep the per-bound wording. Acceptance: the complexity sentence names the per-bound denominator.

### span-causally-38: "there is deliberately no `Eq`; see the module docs" points at module docs that no longer discuss `Eq`
- Where: crates/before/src/causally/query.rs:211-212 (related: crates/before/src/laws.rs:807-809; crates/before/src/causally.rs:1-139; crates/before/src/causally/conjunction.rs:61)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'Eq\b'` over causally.rs and causally/*.rs, excluding `Ordering::Equal`, returns only the `Coverage` derive at query.rs:51 and this pointer; laws.rs:809 repeats the pointer; `git show db9dfa3e:crates/before/src/causally.rs` lines 60-80 carry the deleted "# Deliberately absent" section with the `Eq` bullet); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate-but-expired (the pointer was true when written; the owner's a6dcfbb4 deleted the section while re-wrapping and keeping the pointer)
- Owner-gated: no

A pointer to a decision recorded nowhere sends the maintainer on a search that ends nowhere, and a maintainer who wants to add `PartialEq` finds no stated objection. The reason is real and written nowhere: the hole antichain is stored in construction order (`kept.append(&mut added)` at conjunction.rs:61, so `a & b` and `b & a` differ structurally), and a degenerate hole rides inert beside `all()` (`degenerate_holes_are_inert`), so structurally different normal forms denote one predicate and structural equality would be neither semantic equality nor a useful approximation. The deleted text is a restoration candidate.

Evidence:

       211	// Debug renders the module's own expression vocabulary, the only structural
       212	// window into a query (there is deliberately no `Eq`; see the module docs), so

    (laws.rs)
       807	    /// query admitting exactly itself. Behavioral equations only: a query's
       808	    /// observation surface is membership, deliberately not identity
       809	    /// (`causally`'s module docs carry the no-`Eq` decision).

    (causally.rs at db9dfa3e, deleted by a6dcfbb4)
        73	//! - **`Eq`, `Hash`, and a wire form.** A query is an ephemeral
        74	//!   filter, not a value: two queries built differently may denote
        75	//!   the same predicate. Observe queries behaviorally, through
        76	//!   [`contains`](Query::contains) and [`coverage`](Query::coverage).

Resolution: record the decision once, either restored to causally.rs's module doc (where both pointers say it is) or on `Query`'s type doc with the pointers repointed; state the mechanism (non-unique normal forms under conjunction order and inert degenerate holes). Acceptance: `grep -n 'Eq' crates/before/src/causally.rs` (or query.rs's type doc) finds the decision the two comments cite.

## Positives

- `polarity.rs` concentrates the entire `Down`/`Up` behavioral difference in one sealed dispatch table (lines 36-58), so the two polarities differ in exactly six one-line methods, and the `Neutral` `unreachable!` arms (174-204) carry a one-line proof a reader can check inside the module: every `Query<Neutral>` construction site (convert.rs 15-20, 27-32; conjunction.rs 110-115, 121-126; query.rs 64-69, 202-207) writes `holes: Vec::new()`, and `and` never changes `P`.
- `Span::decode` (wire.rs 116-178) validates both components against the borrowed read buffer, adopts slices of that one allocation for both endpoints, dedups the coincident span's storage on `Admission::Equal`, and pronounces the pair verdict after the padding check so structural defects win; `span_decode_structural_genres_outrank_the_pair_verdict` and its coincident twin pin exactly that precedence with byte-level witnesses whose comments derive each byte, and `span_single_bit_mutations_never_alias` re-derives an accepted mutant through the oracle bridge into a fresh composite so the accept side cannot be gamed by an admission walk that accepts a non-canonical spelling.
- The coincident-span certificate is one idea applied uniformly and held live: `is_coincident`'s doc (span.rs 546-552) states what `ptr_eq` proves and its limit (byte-equal endpoints in distinct buffers take the general walk), `coincident_span_rungs_agree_across_buffer_identity` (span/tests.rs 812-844) holds every fast rung to the fused walk across buffer identity, and `tests/coincident_span.rs` pins the rungs' liveness by scan parity, so a lost rung and a dead meter both read red. Every rung derives only byte equality from `ptr_eq`, which is exactly what `Bits::ptr_eq`'s doc (bits.rs 205-216) licenses.
- `coverage_is_exact_on_the_two_party_grid` (causally/tests.rs 222-323) is a total oracle: the grid is the complete two-party interval, closed under join and meet, so the brute-force census over every ordered segment and a query family spanning both polarities, every hole spelling, and two-hole antichains leaves no corner; together with `refine_partial`'s one-polarity exactness argument (query.rs 143-151, correct and tight) it makes `Coverage` exact, not merely sound.
- `span/algebra.rs`'s module doc (lines 10-33) states the four operators as one four-row leg table with each totality argument written once beside it, and the refusal of `Into<Span>` for the partial operator (lines 43-48) names the silent-vanish failure it prevents; `span_version_lhs_matrix!`'s doc (665-675) gives the coherence reason its cells are concrete rather than generic.
- `fold_endpoints`'s total `(Input, Merged)` arm (algebra.rs 400-410) documents why an unreachable case stays total instead of asserting, and the claim checks out against fold.rs's weight discipline: the closing drain only ever combines `(Merged, Merged)` or `(Merged, Input)`.
- `Placement`'s variant docs (verdict.rs 26-48) derive the forced relation behind each `Concurrent` payload rather than naming it; `span_place_places_every_witness` hits all nine, and the three coarsening tests enumerate each fiber's members.
- The meter's `placement` module (tests/meter.rs 9463-9530) states every fusion's cost relationally against its own composition (fused = composed minus one probe scan), so there is no pinned constant to rot.
- `conjunction.rs`'s `and` doc (20-37) states the normal-form invariant (a pairwise-unabsorbed antichain), why only cross pairs are probed, and the deliberate non-decision (comparative pruning, never semantic emptiness), with its consequence tested in `degenerate_holes_are_inert`.
- Every test in both `tests.rs` files carries a doc comment stating its invariant, and I found none misstating its body; the point-witness tests name the quantified laws they instantiate (`span_gate_admits_exactly_the_ordered`, `degenerate_span_place_is_partial_cmp`, `own_span_matches_the_projected_span`, `span_encoding_is_prefix_free`), all of which exist in `laws.rs`.
- Every `expect`/`unreachable!` message in the partition is a one-line proof (wire.rs 146, 172; algebra.rs 415; polarity.rs 178-202).

## Open questions for Finch

1. Multi-hole cost contract (span-causally-24, -36). Do you want to keep `O(|self| + |span|)`/`O(|self| + |version|)` as the promise for multi-hole queries, or restate with the hole-count factor? Recommendation: fuse `refine_partial` regardless (a fixed-sign deletion of k-1 endpoint decodes), then restate "linear time" as "linear in bits decoded, with per-interval work proportional to live holes", since the polarity restriction's real payoff is a polynomial exact decision, not linear; pin the k axis in the touch currency, not scan bits.
2. `sweep::le` in production (span-causally-26). Was the cfg gate meant to keep one comparison entry point for the differential suite to pin? Recommendation: lift it and add `le`/`lt` to the pinned surface; the one-direction exit is what the fused walks already advertise.
3. `Version::span_all`'s `I::Item: Borrow<Version>` convention (span-causally-9). The wrapper-newtype route keeps that stable signature; changing the convention to `Into<Span>` (an API change) would make `span_all` `Span::at(self).union_all(iter)` outright. Recommendation: the newtype; no API change.
4. Owner-authored prose cluster. The history pass traces the no-`Eq` pointer, the deleted `Coverage` exactness sentence, the inverted SAT direction and its register, "arbitary"/"intractible", the dropped "per stored bound", `after(p)`, the identity rationales, "mint", the uniform `conjoin!` doc, and "of a [`Span`]s" to your hand-edit commits a6dcfbb4, b3f09baa, 20c0515a, and bbb9f802; the db9dfa3e originals are quoted in span-causally-34 and -38 as restoration candidates. Recommendation: re-rule on these as a batch rather than treating them as agent drift.
5. "door" (208 crate-wide uses, none definitional) and "genre" (210 uses, defined only in a private codec doc). Recommendation: retire "door" for the plain noun, as 22cdfbe1 already did in algebra.rs; keep "genre" out of public docs (span-causally-22) and either define it once in `error.rs` or leave it private.
6. `core::` versus `std::` (32 files import `core::cmp::Ordering`, 15 `std::cmp::Ordering`; the crate is not `no_std`). Recommendation: pick one crate-wide (std, since `core::` carries no meaning here) in a mechanical sweep.
7. Em-dashes in `//` comments (374 lines in 76 files crate-wide). Recommendation: a mechanical sweep to colons in a dedicated prose commit; the fifteen partition sites are listed in span-causally-2.
8. `Span: Hash` (span-causally-1). Recommendation: add it; it is additive, consistent with every verdict type, and rests on the same byte equality as `Eq`.
9. The board's proxy pricing of the span folds (span-causally-8). Was mapping `Span::union_all` to `version_span_all` a ruling that `fold_endpoints` adds nothing, or a tiling convenience? Recommendation: if span-causally-9 lands, the proxy becomes sound and only the space argument is owed; otherwise add a span fold cell.
10. `OwnSpan`'s composed verdicts (span-causally-14). Do you want a fused masked three-stream placement kernel? Recommendation: state the composition as the design for now; the kernel is a design proposal whose payoff is one probe-plus-id decode per verdict.

## Dropped

- [13] `Query`'s manual `Clone` could be a derive: refuted as a drop-in (`From<&Query<P>> for Query<P>` under `P: Polarity` alone calls `clone()`, so a derive needs `Polarity: Clone` as a supertrait, a public-trait change); the marker derives are vacuous on uninhabited enums and harmless; below the bar.
- [29] "Union: meets meet, joins join" trades legibility for a pun: taste without a named cost beyond the module-doc table already stating the legs plainly; below the bar.
- [16]'s construction (uniform-height `B`): refuted (a version where every party ticked once normalizes to a single leaf, so each `le(B, v_i)` is O(log k)); the finding itself is merged into span-causally-36 with [40]'s construction.
- [35], [40]: duplicates of span-causally-36.
- [8], [21] (`Eq` half), [36] (`Eq` half), [47]: duplicates of span-causally-38.
- [36] (second half), [48]: duplicates of span-causally-34.
- [21] (monotonicity half), [38] item 6: merged into span-causally-16.
- [10], [20], [26], [38] items 1-4, [51]: merged into span-causally-5.
- [9], [23]: merged into span-causally-4.
- [11], [34]: merged into span-causally-20.
- [7], [49]: merged into span-causally-21.
- [18], [38] item 5, [46] (clone-discipline half): merged into span-causally-13; [46] (intersect parenthetical): merged into span-causally-11.
- [22], [43] (`into_owned` half): span-causally-37; [43] (module summary half): span-causally-25.
- [44]: reframed into span-causally-28 (the fuelscape is audit-only; the enforcement gap is the finding).
- [50]: converted per the history verdict into span-causally-14 (deliberate design, rationale only in history).
- [15]: reframed into span-causally-10 (the `core::`/`std::` mix is crate-wide; the partition-local defect is the inline qualified paths).
- Refutation new item 1 (fuelscape audit-only): folded into the acceptance clauses of span-causally-8, -28, -36 rather than a finding of its own.
- Refutation new items 2 and 3 (coincident-receiver span-argument equality; causally.rs:106 "stops as soon as its verdict is decided"): folded into span-causally-26.
