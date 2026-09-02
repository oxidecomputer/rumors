# Partition skyline-watermark: The watermark (min-ticks web) and the traffic modules

## Partition summary

`watermark.rs` holds `MinWeb<P>`, the anchored-minimum web both skyline sweeps share: one signed accumulator `gap = h − A`, an optional latent boundary `Λ = A − m`, a `Vec<Entry<P>>` difference stack with zero runs compressed and (per instantiation) word-scale boundaries compacted, two follower slots with one-bit anchor-relative tags, and an accumulator pool. The generic core (arming, close/park, undercut/propagate, latent resolution) is written once; `impl MinWeb<()>` adds the fill walk's emission vocabulary (`emit_here`, `emit_offset`, `compare_above`, the bridges, the follower slots), and the min-ticks client rides `P = Reign` through a lazy `FnOnce` payload constructor, an `FnMut` death hook, and the total `Close<P>` enum. `pool_traffic.rs` and `web_traffic.rs` are feature-gated relaxed-atomic counters: one counts pool misses (the one observable that separates a live recycle from a dead one), the other classifies the fill walk's post-sign domination decisions (the one observable that proves the dominated-undercut arm still fires). `watermark/tests.rs` is a declared internal-entry suite: four worked pins on the latent ladder's gates plus three proptest families.

The kernel is sound. Every arm's value flow was traced by the correctness and claims lenses against suanpan's `Accumulator` contract and I re-traced the arms the surviving findings rest on (the three arming paths into `push_boundary`, `close`/`park`, `undercuts_here`/`decide_undercut_through_latent`, `undercut`/`drop_below`/`propagate`, both `compare_above` readers, the `emit_offset` ladder). No traversal recurses; the difference stack is a `Vec`; every count is a `u64`; every `debug_assert`/`expect`/`unreachable!` message is a one-line proof of why its branch is programmer error; every cost claim in the module doc names a committed instrument, and each name resolves. The payload seam is a model of a zero-cost generic. The two counter modules justify their existence in writing by naming what no other meter can see.

The dominant issues are structural duplication inside `watermark.rs` and two inaccurate test-side claims. The latent-ladder decision is written three times, the undercut tail four times (once with `drop_below`'s follower loop hand-inlined at exactly the site where the residue-polarity bug of d7293057 lived), the first-arming preamble twice, and `propagate`'s domination guard twice with the operands swapped — the last costing a line-and-column-pinned mutant exclusion that git history shows re-pinned four times in eight days. On the verification side, the test module's doc says `drop_below`'s latent annihilation is unreachable from any packed stream, but the min-ticks client reaches it directly (`ReignWeb::leaf` calls `undercuts_here` then `undercut` with no `compare_above` in front), and a proptest's closing comment describes a follower and a parked boundary the test never builds while the one arm it names (`drop_below` with a live latent and a live follower) runs in no test. The rest are documentation-altitude and vocabulary items, most of which predate the rules they now breach, plus one owner decision about which feature gate the two watermark counters share.

Lines read: watermark.rs 1181, watermark/tests.rs 431 (the test file), pool_traffic.rs 60, web_traffic.rs 113 (1785 in the partition), plus the cross-referenced ranges of `tests/meter.rs`, `src/meter.rs`, `fill.rs`, `fill/prescan.rs`, `fill/tests.rs`, `query.rs`, `query/web.rs`, `signed.rs`, `codec/base.rs`, `hull_traffic.rs`, `codec/scan.rs`, `crates/suanpan/src/accumulator.rs`, `.cargo/mutants.toml`, `tools/covcheck-expected.json`, `tools/mutantcheck-expected.json`, `Cargo.toml`, `rust-toolchain.toml`, and the commit messages and blame the history pass cites. No cargo, just, test, or state-changing git command was run; every "verified" below means read, grepped, or traced by hand against the source at 9e5784fb.

## Findings

### skyline-watermark-1: Vocabulary sweep: 'mint' for constructing a value, moralized 'honest', shouted 'MOVES', 'genuinely'
- Where: crates/before/src/version/skyline/watermark.rs:44-1144 (related: watermark.rs:66, 129, 303, 350, 361, 553, 628 for 'mint'; 488, 764, 768, 1144 for 'honest'; 44, 303 for 'MOVES'; watermark/tests.rs:221 for 'genuinely')
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n '\bmint'` and `grep -n -i 'honest\|genuinely\|MOVES'` over the four partition files return exactly these sites); executed: no
- Seen by: prose, claims; refutation: confirmed (line 303, not 304); history: contradicts-hard-rule, every site predating the rule (writing-style.md landed 2026-08-19; the sites date from 2026-08-07 to 08-18)
- Owner-gated: no

The review standard bans 'mint' for constructing a value and flags moralized code and significance adverbs. `watermark.rs` uses 'mint' in two senses (construct a payload or latent; define a convention at line 66), calls a collapsed top digit 'honest' where the mechanism is "a sign read already ran, so `digit_count` is tight", and shouts 'MOVES' in capitals where italics carry the move-versus-fold distinction.

Evidence:

        66	//!   module's Cost section mints) and
       129	//! web: a pushed-above arm stacks the payload its caller mints lazily beside
       303	    /// boundary MOVES into the latent register (minting it, or dying by
       488	        // Collapse for an honest top before the domination reads: a sign
       764	                    // Tops are honest: a pushed difference had its sign read at

Resolution: 66 "defines"; 129, 553, 628 "constructs"; 303 "creating it"; 350 "the fresh-latent move"; 361 "a fresh latent finds them `m`-exact"; 488/1144 "Collapse so the top is tight before ..."; 764 "Tops are tight:"; 768 "an undecided read"; 44/303 "*moves* into" (italics, the emphasis is semantic); tests.rs:221 "a different certification test that the same input must answer the same way". Note the cascade: `.cargo/mutants.toml:117` pins `watermark.rs:790:43`, so edits above line 790 need a re-pin in the same commit. Acceptance: `grep -n -i '\bmint\|honest\|genuinely\|MOVES'` over the partition returns nothing.

### skyline-watermark-2: The emission bullet of the cost discipline is one sixteen-line sentence with rewrap residue
- Where: crates/before/src/version/skyline/watermark.rs:55-70 (related: none)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read; `git blame -L 55,70` shows 57-61 from a736ef14 and 64-66 from 248d5539 spliced into c29bd2b3's sentence, none touched by the b03267f0 rewrap); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (edit residue)
- Owner-gated: no

The argument of record for the emission cost is the hardest passage in the module to parse: one sentence from 55 to 70, with 'it,' alone on line 61 and 'module's Cost section mints) and' on line 66. Every other bullet in the section is one or two sentences.

Evidence:

        59	//!   and only comparable scales
        60	//!   fold — a narrower-into-wider collapse whose near-cancellation funds
        61	//!   it,
        62	//!   after which re-widening the latent costs the input a fresh climb.

Resolution: split into three sentences (the amortized sign read against the anchor; the O(1) latent decision by top-index domination with comparable scales folding and funding; the fold-the-priced-side-and-restore rule with the wide-`gap` exception) and re-wrap. Acceptance: no doc line is a dangling fragment; the bullet reads as separate claims.

### skyline-watermark-3: The pool-recycle claim is stated for every client but pinned only through min_ticks
- Where: crates/before/src/version/skyline/watermark.rs:82-87 (related: crates/before/tests/meter.rs:9381-9461; fill.rs:313-314, 527-528, 695-822, 980-987, 1091-1123; fill/prescan.rs:335-408, 555-584)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn pool_misses crates/before` finds readers only at tests/meter.rs:9418-9420, inside `#[cfg(feature = "limb-meter")] mod pool_recycle`, which drives `Version::min_ticks` over `Shape::SeamStop`; `MinWeb` is instantiated at fill.rs:303, prescan.rs:125, query/web.rs:231); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (d0501cd7 placed the row on SeamStop because that family was already being built)
- Owner-gated: no

Instruments before cures: the module doc claims steady-state churn allocates nothing and cites one row; that row runs the min-ticks client only. The fill walk and its pre-scan are the churn-heaviest clients, leasing and retiring buffers at their own call sites that no meter observes, and `pool_traffic.rs:8-13`'s own argument (heap and touch meters are blind to a dead recycle) applies equally to a client-side leak: an accumulator dropped instead of retired in fill.rs would read proportional misses on a tick path while every committed envelope stayed green.

Evidence:

        82	//! - Dying accumulators return to a pool and are re-armed cleared, so
        83	//!   range churn allocates nothing in steady state: misses (leases the
        84	//!   pool could not serve, counted by the [`pool_traffic`](super::pool_traffic)
        85	//!   meter) are bounded by the walk's peak simultaneous demand, never its
        86	//!   churn length — the seam-stop pool row of `tests/meter.rs` pins it,
        87	//!   the heap meter being structurally blind to a dead recycle.

Resolution: add a tick-path pool-miss row beside `pool_recycle` on a committed follower-churn family (`width_circulation_cost`'s or `memo_resolution_cost`'s shapes): reset misses, tick, assert `small >= 1`, `small == large` across the doubling, and `large <= WARMUP` with the warm-up derived from the walk's peak outstanding leases. Then cite both rows at 86, or scope the sentence to the min-ticks client. Acceptance: a `limb-meter` row whose MEASURED line reads equal misses at both scales on a tick family, and which turns red when one `self.web.retire(...)` in fill.rs becomes `drop(...)` under a local, reverted swap.
Construction: under a local reverted swap replace `self.web.retire(relation)` at fill.rs:808 with `drop(relation)`; run the existing suite — every envelope and differential stays green (peak heap falls; touches identical). Add the proposed row and observe misses proportional to `k`.

### skyline-watermark-4: Hand-maintained 'two' restates FOLLOWER_SLOTS, once inaccurately
- Where: crates/before/src/version/skyline/watermark.rs:93-95 (related: watermark.rs:147-149, 236; fill.rs:182-188)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read; the slot-iterating sites are `park` 367-375, `resolve_latent` 418-426, `drop_below` 530-540, `push_boundary` 639-644, the dominated arm 975-977, and `close`'s assert 328 — `undercuts_here` 452-463 touches no slot); executed: no
- Seen by: prose, claims; refutation: confirmed (with the fuller site list); history: contradicts-hard-rule (CLAUDE.md "No hand-maintained counts", 2026-08-10; the prose is c29bd2b3's, `FOLLOWER_SLOTS` arrived the same day in a736ef14 without re-denominating it)
- Owner-gated: no

Doctrine: no hand-maintained counts; a number that matters lives in a mechanically enforced place the prose cites by name. `FOLLOWER_SLOTS` is that place, and the prose has already drifted: a min-ticks leaf that neither arms nor undercuts passes through no follower loop, so "two `None` checks per event" is false.

Evidence:

        93	//! the relation and never reads it. Only the fill walk installs any (two, for
        94	//! relations named in `fill.rs`; the min-ticks fold installs none and pays two
        95	//! `None` checks per event).
       147	/// Follower slots the web carries (the fill walk's two relations; a const
       148	/// assert beside the fill walk's slot constants binds the two rosters).
       236	            followers: [None, None],

Resolution: 93-95 "Only the fill walk installs any (`FOLLOWER_SLOTS` of them, for the relations named in `fill.rs`); the min-ticks fold installs none and pays one `Option` check per slot at each arm, undercut, park, and collapse." 147-148 "Follower slots the web carries; a const assert beside the fill walk's slot constants binds the two rosters." 236 `followers: [const { None }; FOLLOWER_SLOTS]` (stable on the pinned 1.97.1 toolchain). Acceptance: widening `FOLLOWER_SLOTS` needs no prose edit and no change at 236; the cost statement matches the code paths that iterate the slots.

### skyline-watermark-5: Two maintainer docs misattribute which operand a fold reads or a reader mutates
- Where: crates/before/src/version/skyline/watermark.rs:113-115 (related: watermark.rs:449-451, 584-586, 610-614, 1160-1162; crates/suanpan/src/accumulator.rs:561)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read: line 613 seats `below` as `self.gap`, line 614 is `offset.sub_accum(&self.gap)`, and suanpan's `fold_accum` at accumulator.rs:561 iterates `other.digits[..=other.top]` only, so the fold costs `|below|`, the surviving operand; `bridge_add_gap` at 1160-1162 is `delta.add_accum(&self.gap)` and mutates no web state); executed: no
- Seen by: prose ([18]), claims ([40b]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Comments state what the code cannot show, and must be right about it. (a) The module doc and `arm_below`'s doc charge the offset fold to "the narrow dying side" (`gap_old`, the receiver), but the fold walks `below`'s digits, the priced operand that survives as the new `gap`; a future amortization audit reasoning from the doc would price the wrong operand. (b) `undercuts_here`'s doc files `bridge_add_gap` among "the fold-and-restore readers", but it neither folds into nor restores the web; `compare_above_vs` is the genuine fold-and-restore.

Evidence:

       113	//! - [`arm_below`](MinWeb::arm_below): `v = h − below`, the accumulator
       114	//!   moving in as the new `gap`. Handles the first arming too; the offset
       115	//!   `gap_old − below` costs one fold of the narrow dying side.
       613	        let mut offset = core::mem::replace(&mut self.gap, below);
       614	        offset.sub_accum(&self.gap);
       449	    /// unchanged, its representation is not — unlike the fold-and-restore
       450	    /// readers ([`compare_above_vs`](Self::compare_above_vs),
       451	    /// [`bridge_add_gap`](Self::bridge_add_gap)), which restore exactly.

Resolution: 115 and 586 "the offset `gap_old − below` costs one fold of `below`'s width, which the caller priced and which survives as the new `gap`"; 449-451 "unlike the readers that leave the representation untouched ([`compare_above_vs`], which folds and restores exactly, and [`bridge_add_gap`], which only reads `gap`)". Acceptance: each doc names the operand the code actually reads.

### skyline-watermark-6: Strictly-positive counts held as plain u64
- Where: crates/before/src/version/skyline/watermark.rs:154-168 (related: watermark.rs:341-343, 652, 741, 819, 854-876)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (read: `push_zeros` returns on `count == 0` at 868-870, `close` re-pushes only `count > 1` at 341-343, and every `compact` call site hands a value whose sign was just read `Greater`); executed: no
- Seen by: structure; refutation: confirmed (noting `NonZeroU64::new` at `compact` would turn the debug assert at 857 into an `else` branch); history: no-rationale-found (in-crate precedent: fill/memo.rs's `Option<NonZeroU32>` niche assert)
- Owner-gated: no

Types-first: `Boundary::Word` is documented "strictly positive" and `Entry::ZeroRun` is never constructed with zero, so both could be `NonZeroU64`, moving the doc invariant into the type. The cost is `.get()` noise at the arithmetic sites (872, 342, 722), which is why this is a nit.

Evidence:

       154	enum Boundary {
       155	    /// A machine-word difference (strictly positive).
       156	    Word(u64),
       157	    /// A wide difference on its own accumulator (strictly positive).
       158	    Wide(Accumulator),
       159	}
       164	    ZeroRun(u64),

Resolution: if taken, `Word(NonZeroU64)` and `ZeroRun(NonZeroU64)`; `compact` constructs via `NonZeroU64::new(word)` with the `Greater` assert as its justification; `push_zeros` keeps its `u64` door and converts once. Acceptance: "strictly positive" disappears from `Boundary::Word`'s doc because the type states it.

### skyline-watermark-7: Follower slots as parallel arrays; the coupling invariant lives in asserts and an expect
- Where: crates/before/src/version/skyline/watermark.rs:194-210 (related: watermark.rs:331, 364-402, 414-429, 529-547, 639-644, 1120-1138)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the five index loops at 367-375, 387-391, 418-426, 530-540, 639-644, the `expect` at 422, and the vacuous tag reset at 331 after every follower is asserted `None`); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (the tick-cost-spec's "one-bit sigma tag" constrains the tag's cost, not its layout)
- Owner-gated: no

Types-first: "a set tag rides an active follower" is enforced by index loops, a `debug_assert!`, and an `.expect(...)` where a single `[Option<Follower>; FOLLOWER_SLOTS]` with `struct Follower { relation: Accumulator, anchor_relative: bool }` makes a tag without a follower unrepresentable, turns every loop into `for f in self.followers.iter_mut().flatten()`, and deletes the `expect`, the tag-existence asserts, and line 331. What remains to assert is only the tag-to-latent relation.

Evidence:

       194	    latent: Option<Accumulator>,
       195	    /// Per follower slot: whether the stored content is anchor-relative
       196	    /// (`f_true = f_stored − Λ`). Set only while the latent lives; a set tag
       197	    /// never outlives it.
       198	    anchor_relative: [bool; FOLLOWER_SLOTS],
       207	    /// Active followers (module doc), tracking `m − X` (anchor-relative while
       208	    /// the slot's tag is set). The fill walk installs them; the min-ticks
       209	    /// fold leaves both slots empty.
       210	    followers: [Option<Accumulator>; FOLLOWER_SLOTS],
       420	                self.followers[slot]
       421	                    .as_mut()
       422	                    .expect("a set tag rides an active follower")

Resolution: define the private `Follower` struct, change the field, delete `anchor_relative`, rewrite the five loops over the flattened iterator; `follower_set` becomes `Some(Follower { relation, anchor_relative: self.latent.is_some() })`, `follower_take` returns `.take().expect("the follower is active").relation`. Keep `park`'s and `drop_below`'s asserts relating tags to the latent. Acceptance: no `anchor_relative` array, no `expect("a set tag rides an active follower")`, no `0..self.followers.len()` loop; `watermark/tests.rs`, `fill/tests.rs`, and the fill touch envelopes unchanged (no arithmetic moves).

### skyline-watermark-8: compacting()'s doc carries measured counterfactual ratios and misstates the saving's mechanism
- Where: crates/before/src/version/skyline/watermark.rs:245-256 (related: watermark.rs:220-227; tests/meter.rs:7125-7140, 8363; .cargo/mutants.toml:70-76; crates/suanpan/src/accumulator.rs:103-155)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified for the ratios' provenance (`git show cedb6015`: "623,408 B heap (x1.41) and 32,027 touches (x2.0)" measured under a manual swap; `compacting()` is the only min-ticks constructor at query/web.rs:231, so nothing committed reproduces them) and for the roster cross-reference (.cargo/mutants.toml:74-75 "the compacting doc carries the measured margins"); assessed for the layout point (Accumulator's fields at accumulator.rs:103-155 are `Option<i128>`, `Vec<i64>`, two `usize`, `BTreeMap`; `Boundary` is a by-value enum inside `Entry<P>`, so both variants occupy the same inline size; `size_of` not measured); executed: no
- Seen by: prose ([19]), correctness ([29]), claims ([34]); refutation: confirmed; history: deliberate-and-holds for placement (cedb6015 put the ratios in; 9aec9aa2 adjudicated the delete-field mutant "in the doc's favor"; the mutants roster reads them), so the relocation is owner-gated
- Owner-gated: yes: the ratios are the recorded adequacy evidence for a mutant cargo-mutants cannot filter, and .cargo/mutants.toml:74-75 cites the doc as carrying them

Prose speaks in the present tense: the ×1.41 and ×2.0 figures are a dated measurement of an implementation that does not exist in the tree, quoted at a declaration site, and re-pinning `skyline_min_ticks_ascend` silently invalidates them. The mechanism sentence is imprecise in a way the next optimizer would trip on: `Word(u64)` and `Wide(Accumulator)` occupy the same inline footprint, so the saving is the retired accumulator's digit buffer circulating through the pool (and the O(1) word fold at propagate), not "an inline word ... instead of an accumulator entry". Separately, `new()`'s doc at 226-227 cites "the `width_circulation_cost` and memo modules"; the module is `memo_resolution_cost` (tests/meter.rs:8363).

Evidence:

       246	    /// boundaries stack, in two currencies: per-boundary transient storage
       247	    /// (an inline word per stacked difference instead of an accumulator
       248	    /// entry) and undercut propagation (a residue consumes each word
       253	    /// un-compacted storage reads ×1.41 that row's pinned peak heap and
       254	    /// ×2.0 its pinned touches, over both ceilings. Shapes whose stacked

Resolution: reword the mechanism as "compaction retires a word-scale boundary's accumulator to the pool at the push, so a stack of word-scale boundaries circulates one digit buffer instead of holding one per entry, and an undercut consumes each word boundary by one O(1) fold; the `skyline_min_ticks_ascend` row is the enforcing envelope, and deleting compaction trips both its heap and touch ceilings". Either drop the ratios (they live in cedb6015) and re-point .cargo/mutants.toml:74-75 and tests/meter.rs:7137-7139 at the row itself, or make the demonstration committed (a `#[cfg(test)]` constructor toggle and a red-first assertion that the un-compacted web exceeds the row's ceilings), at which point the numbers live in that test. Name `memo_resolution_cost` at 227. Acceptance: no measured ratio at the declaration site or a committed test producing it; the mechanism sentence names the digit buffer; every cited module name resolves by grep.

### skyline-watermark-9: fold_height's armed guard has its why only in .cargo/mutants.toml
- Where: crates/before/src/version/skyline/watermark.rs:282-290 (related: .cargo/mutants.toml:127-129; watermark.rs:184-186, 570, 605)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read both sites: the guard's doc explains only what a height step means; the roster entry carries "an unarmed range's gap is re-based on arming, so folding height into it early is a dead store"; the first-arming paths at 570 and 605 replace `gap` wholesale); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds (rationale exists, off-site)
- Owner-gated: no

Comments state what the code cannot show, and configuration files may cite code, never the reverse: the reason a branch exists belongs at the branch.

Evidence:

       284	    /// `h` moved while every `m` stayed: exactly the innermost range's
       285	    /// `gap` shifts; the differences and followers are height-free.
       286	    pub(super) fn fold_height(&mut self, sign: Sign, magnitude: &Int) {
       287	        if self.armed > 0 {

Resolution: add one clause to the doc: "An unarmed web skips the fold: its `gap` is replaced wholesale at the first arming, so the store would be dead." Acceptance: a reader at the guard learns why it exists without opening mutants.toml.

### skyline-watermark-10: Em-dashes in // line comments at eight sites
- Where: crates/before/src/version/skyline/watermark.rs:491-811 (related: watermark.rs:699, 704, 759, 767, 768, 811; watermark/tests.rs:146; .cargo/mutants.toml:117)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the four files returns exactly these eight lines; no assert, expect, or unreachable string contains one); executed: no
- Seen by: prose; refutation: confirmed; history: contradicts-hard-rule (CLAUDE.md "colons (or semicolons) over em-dashes in ... comments", 2026-08-10; several sites postdate it)
- Owner-gated: no

Owner doctrine reserves true em-dashes for rendered prose; line comments take colons, semicolons, or a sentence break.

Evidence:

       491	        // stale count's floor would refuse — suanpan's witness
       811	                    // — the dying side's digits within a constant, whichever

Resolution: replace each with a colon, semicolon, or sentence break. Seven of the eight sit above line 790, so the edit moves the line-pinned exclusion `watermark.rs:790:43`; re-measure with `cargo mutants --list` and re-pin in the same commit (as b44a8326 did), or land this together with finding 14, which removes the pin. Acceptance: the grep returns nothing; `just mutants-list` clean.

### skyline-watermark-11: A redundant latent sign read, and a comment that credits it with the floor's tightness
- Where: crates/before/src/version/skyline/watermark.rs:492-500 (related: crates/suanpan/src/accumulator.rs:805-828, 844-886; tests/meter.rs:3417-3427)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified (read: `sign_dominates_at` at accumulator.rs:820 runs `fold_and_collapse` before line 500 reads `latent.digit_count()`, so the explicit read at 495 collapses nothing the 497 read would not; under the test profile a `debug_assert_eq!(latent.sign(), ...)` still executes the read, so `LADDER_MARGINAL_TOUCH_FLOOR`'s premise at meter.rs:3421-3424, which counts the consumed step, the gap fold, and the total fold, is unaffected); executed: no
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (the read predates unification; suanpan's `sign_dominates_at` has collapsed in place since 10b86c036, so it was never load-bearing)
- Owner-gated: no

Legibility: a read whose only surviving purpose is a debug assertion should be spelled as one, and the comment should credit the collapse that actually makes `latent_floor` tight (the domination read at 497). As written, line 495 costs one amortized touch per decision on the path the latent-ladder band meters, for a value no release build reads.

Evidence:

       492	        // `sign_collapse_tightens_the_top_and_arms_domination`. Both floors
       493	        // here rest on that clause: gap's sign was read by the caller, the
       494	        // latent's is read now.
       495	        let _sign = latent.sign();
       496	        debug_assert_eq!(_sign, Ordering::Greater, "the latent is strictly positive");
       497	        if latent.sign_dominates_at(gap_floor).1 {
       498	            return false;
       499	        }
       500	        let latent_floor = latent.digit_count() - 1;

Resolution: `debug_assert_eq!(latent.sign(), Ordering::Greater, "the latent is strictly positive");` and re-state 492-494: the caller's sign read makes `gap_floor` tight, and the domination read at 497 collapses the latent before `latent_floor` is taken. Acceptance: no unconditional `sign()` precedes the domination read; the latent-ladder band stays green.

### skyline-watermark-12: arm_at_height and arm_below duplicate the first-arming preamble
- Where: crates/before/src/version/skyline/watermark.rs:562-574 (related: watermark.rs:598-609, 1019-1022, 593-596)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read: the two branches differ only in what seats the new `gap`, `self.lease()` versus `below`; `emit_below_accum`'s `debug_assert!(self.pending > 0, ...)` at 1020 restates `arm_below`'s at 593-596 with a different message); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (c29bd2b3 preserved each client's op sequence for byte-identity against twin webs deleted in the same commit)
- Owner-gated: no

Legibility: the first-arm invariants (`armed = pending`, `pending − 1` zeros, followers and latent absent) are stated twice and must agree; the module doc's "All three converge on the shared boundary bookkeeping" is true of the non-first path only.

Evidence:

       562	        if self.armed == 0 {
       563	            debug_assert!(
       564	                self.followers.iter().all(Option::is_none),
       565	                "followers attach after the first arming"
       566	            );
       567	            debug_assert!(self.latent.is_none(), "the latent dies with the web");
       568	            self.armed = pending;
       569	            let fresh = self.lease();
       570	            let old = core::mem::replace(&mut self.gap, fresh);
       571	            self.retire(old);
       572	            self.push_zeros(pending - 1);
       573	            return;
       574	        }

Resolution: extract `fn seat_first(&mut self, pending: u64, gap: Accumulator)` holding the two asserts and the seat/retire/push_zeros body, taking the already-leased or moved `gap` so the lease-before-retire order the pool row's warm-up derivation describes is preserved; call it from both entry points. Drop the duplicate assert in `emit_below_accum` or keep only the one whose message names the raise. Acceptance: one occurrence of "followers attach after the first arming"; `batch_armed_closes_consume_exactly_one_range_record` and the min_ticks differentials green.

### skyline-watermark-13: push_boundary's doc says only the pushed-above arm constructs the payload; the undercut arm does too
- Where: crates/before/src/version/skyline/watermark.rs:628-631 (related: watermark.rs:553-555, 663-668; query/web.rs:308-315)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: line 664 is `on_die(payload());`; `arm_at_height`'s doc at 553-555 names both constructing arms; the min-ticks payload closure at query/web.rs:309-313 swaps the reigning record, so the call is load-bearing); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (inaccurate from c29bd2b3)
- Owner-gated: no

Private rustdoc must be accurate against today's code: a maintainer trusting this sentence would take the `Less` arm's `payload()` for a redundant construction and remove it, leaving the dead record reigning in the min-ticks fold.

Evidence:

       628	    /// residue). Only the pushed-above arm mints the payload; an arming
       629	    /// undercut's payload dies by `on_die` before the residue drives outward,
       630	    /// and an exact meet touches no payload at all — the reigning state
       631	    /// continues.
       663	            Ordering::Less => {
       664	                on_die(payload());

Resolution: "Two arms construct the payload: the pushed-above arm stacks it beside the new difference, and an arming undercut hands it straight to `on_die` before the residue drives outward; an exact meet touches no payload at all — the reigning state continues." Acceptance: the `push_boundary` and `arm_at_height` docs agree and name the same two arms as lines 655 and 664.

### skyline-watermark-14: propagate's certificate ladder is cost-inert over the fold it guards, and its duplicated guard costs a line-pinned mutant exclusion
- Where: crates/before/src/version/skyline/watermark.rs:772-809 (related: watermark.rs:676-694, 747-771, 810-839; .cargo/mutants.toml:99-117; tools/covcheck-expected.json:146-157; tests/meter.rs:3301-3327; crates/suanpan/src/accumulator.rs:536-575, 718-725, 805-828, 1169-1175)
- Class / severity / confidence: simplification / medium / medium
- Provenance: assessed (read suanpan's `fold_accum`, which walks `other.digits[..=other.top]` only, so `x.sub_accum(&y)` costs O(|y|) whatever `x`'s width; `sign()` and `sign_dominates_at` share one `fold_and_collapse`; .cargo/mutants.toml:106-109 records the boundary-dominates guard's inversion as touch-identical; the cascade verified from `git show b44a8326` and the four re-pins in history); executed: no
- Seen by: structure ([2]), claims ([32]); refutation: confirmed, reframed (the seam bands stay as the width-conservation instrument; what dissolves is the certificates, the `+ 2` rule, the two `unreachable!` arms with their covcheck entries, and the line pin); history: already-known for the touch-identity (d0501cd7 kept the arm deliberately for "width conservation, arm coverage, and the closed-form value legs") — reopened here with new evidence, since no record considers the merge-direction alternative
- Owner-gated: yes: reopens d0501cd7's recorded decision to keep the boundary-dominates arm; removes two rostered panic arms and a rostered exclusion

Circular justification: a guard earns its place by naming a cost it saves. The boundary-dominates arm (790-809) does exactly the work of the comparable fold below it (one O(1) top read of `diff`, one O(|residue|) fold, retire); the residue-dominates arm (772-789) equals folding the narrower operand into the wider; and when a certificate is undecided at exactly two digits of clearance, the current code falls through to `diff.sub_accum(&residue)`, paying the wider side. `merge_into_wider`'s rule — receiver by `digit_count`, fold the narrower in, read the exact sign — reproduces all five outcomes at the same or lower touch cost with no clearance arithmetic, and dissolves the 20-line certificate comment, both `unreachable!` arms, their two covcheck roster entries, and the exclusion pinned at `790:43`, which four commits in eight days have re-measured after prose edits. If the owner keeps the ladder, the two mirror-image guards should at least be one helper, which by the roster's own disposition ladder (step 1: refactor so the mutated codepoint does not structurally exist) may retire the line pin.

Evidence:

       772	                    if residue.digit_count() >= diff.digit_count() + 2 {
       773	                        match residue.sign_dominates_at(diff.digit_count() - 1) {
       774	                            (Ordering::Greater, true) => {
       790	                    if diff.digit_count() >= residue.digit_count() + 2 {
       791	                        match diff.sign_dominates_at(residue.digit_count() - 1) {
       792	                            (Ordering::Greater, true) => {
       813	                    diff.sub_accum(&residue);
       814	                    self.retire(residue);
       815	                    match diff.sign() {

Resolution (primary): replace 772-839 with `let residue_wider = residue.digit_count() >= diff.digit_count(); let (mut wide, narrow) = if residue_wider { (residue, diff) } else { (diff, residue) }; wide.sub_accum(&narrow); self.retire(narrow);` then dispatch on `(wide.sign(), residue_wider)`: `(Greater, true)` the difference died, `on_die(payload); zeros += 1; residue = wide; continue`; `(Equal, _)` exact meet, `on_die(payload); self.retire(wide); zeros += 1; break`; `(Less, true)` the boundary survives as `diff − residue`: `wide.negate(); push Diff { compact(wide), payload }; break`; `(Greater, false)` push `Diff { compact(wide), payload }; break`; `(Less, false)` the difference died: `on_die(payload); wide.negate(); residue = wide; zeros += 1`. Rewrite 676-694 as "fold the narrower operand into the wider (`merge_into_wider`'s rule); the survivor is never read across its width", delete .cargo/mutants.toml:99-117 and the two covcheck entries, re-pin tools/mutantcheck-expected.json. Fallback: a `fn dwarfs(big: &mut Accumulator, small: &Accumulator) -> bool` written once (guard, read, one `unreachable!`), the arm becoming two `if dwarfs(...)` calls; then `cargo mutants --list` decides whether the single `>=` site still needs a pin, and a name pin replaces the line pin if so. Acceptance: `just test-all` green with the seam plunge/stop bands, the latent-ladder band, the ascend envelope, and the fill/min_ticks differentials unchanged; the seam MEASURED lines inside their bands (identical, or differing by a constant per hop); no line-and-column pin for `watermark.rs` in .cargo/mutants.toml; `just mutants-list` clean.
Construction: before changing code, delete lines 790-809 alone under a local reverted swap and run `skyline_min_ticks_seam_stop_*`: the mutants.toml rationale predicts byte-equal readings, which demonstrates that arm saves nothing. Then apply the primary replacement and capture the seam bands' MEASURED lines; equality or a per-hop constant delta inside the ×0.75/×1.25 bands settles cost-inertness.

### skyline-watermark-15: compact's 'anything wider can never fit' overstates suanpan's collapse bound
- Where: crates/before/src/version/skyline/watermark.rs:850-853 (related: crates/suanpan/src/accumulator.rs:50, 104-106, 785-800, 844-886, 942-948)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified by derivation from suanpan's source (`SIGN_DECIDED = 3` at accumulator.rs:50; digits lie in `|d| < 2^33`; `fold_and_collapse` breaks at the top digit when `|partial| >= 3` and rewrites nothing, so `[−(2^33−1), −(2^33−1), 3]` denotes `3·2^64 − (2^33−1)(2^32+1) = 2^64 − 2^32 + 1`, a three-digit collapsed spelling of a `u64`-fitting value that `compact` keeps `Wide`); executed: no
- Seen by: prose ([17]), correctness ([28]); refutation: confirmed (duplicates merged; [28]'s figure is the tight one); history: no-rationale-found
- Owner-gated: no

Statement faithfulness: the doc asserts an impossibility the substrate does not provide. The consequence is only a forfeited compaction on the band `[2^64 − 2^32 + 1, 2^64)` of digit-engine spellings (register-held values have an exact digit count), never a wrong value, because `u64::try_from` decides whenever the count test passes.

Evidence:

       850	    /// The width test reads the digit count alone — two digits cover a `u64` at
       851	    /// the accumulator's base-2^32 digit width, so anything wider can never fit
       852	    /// — and a wide difference is therefore never normalized just to learn it
       853	    /// would not fit.

Resolution: "The width test reads the digit count alone: a difference spelled in more than two digits after its sign read is at least `2^64 − 2^32` (the domination bound), so the count test forfeits compaction only on that sliver, and a wide difference is never normalized just to learn it would not fit." Acceptance: the doc states sufficiency and names the forfeited band; code unchanged.

### skyline-watermark-16: Accumulator::default() spelled once where the crate says Accumulator::new()
- Where: crates/before/src/version/skyline/watermark.rs:883-891 (related: watermark.rs:230; .cargo/mutants.toml:65-66; crates/suanpan/src/accumulator.rs:1493-1497)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn 'Accumulator::default()' crates/before/src crates/before/tests crates/before/benches` returns only line 888; `Default` forwards to `new`); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (residue of d0501cd7 unpacking `unwrap_or_default()` into a match)
- Owner-gated: no

One spelling per construction across a crate; the odd spelling reads as if it meant something, and the mutants header's "a bare `Default::default()`" beside it invites a reader to look for a distinction that does not exist.

Evidence:

       883	    pub(super) fn lease(&mut self) -> Accumulator {
       884	        match self.pool.pop() {
       885	            Some(cleared) => cleared,
       886	            None => {
       887	                super::pool_traffic::record_miss();
       888	                Accumulator::default()
       889	            }
       890	        }
       891	    }

Resolution: `Accumulator::new()` at 888. Acceptance: the grep returns nothing outside tests.

### skyline-watermark-17: Emission entry points do not assert the range bracket their siblings assert
- Where: crates/before/src/version/skyline/watermark.rs:947-958 (related: watermark.rs:184-186, 908-925, 1037, 1084, 1103-1104; fill.rs:310, 523; query.rs:437-439)
- Class / severity / confidence: verification-gap / nit / high
- Provenance: verified by trace (on a fresh web `emit_offset(&below(5))` records `Undecided`, folds `gap` to −5, takes the undercut tail, and re-seats `gap = +5` while `armed == 0`, contradicting the field doc at 184-186 until the first arming replaces `gap` wholesale; both clients open before emitting, so no live bug); executed: no
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (the asymmetry predates unification)
- Owner-gated: no

The module doc states the client contract ("a range is the client's own bracket"); `compare_above`, `compare_above_vs`, `arm_relative`, `emit_below_accum`, and `close` debug-assert their `armed`/`pending` preconditions, while `emit_here` and `emit_offset` accept an emission outside any range and heal silently at the next arming, which is exactly the constructible misuse a guard exists to name.

Evidence:

       947	    pub(super) fn emit_offset(&mut self, offset: &Signed) {
       948	        if offset.is_zero() {
       949	            self.emit_here();
       950	            return;
       951	        }
       952	        if self.pending > 0 {

Resolution: `debug_assert!(self.pending > 0 || self.armed > 0, "every emission lies inside an open range")` at the top of `emit_here` and `emit_offset`. Acceptance: the assert is present; both clients' suites green under debug assertions.
Construction: `let mut web: MinWeb<()> = MinWeb::new(); web.emit_offset(&below(5));` then `web.open(1); web.emit_here();` — today the first call leaves `gap = +5` unarmed and records an `Undecided` decision; after the fix it panics in debug builds.

### skyline-watermark-18: Four undercut tails, one with drop_below's follower loop hand-inlined where the polarity bug lived
- Where: crates/before/src/version/skyline/watermark.rs:969-982 (related: watermark.rs:514-519, 529-547, 919-924, 1003-1010; tests/meter.rs:9399-9411)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`git show d7293057 -- watermark.rs` shows the one-line fix `offset.sign` → `offset.sign.negate()` at the hand-written residue fold that is line 974 today; the inlined loop at 975-977 equals `drop_below` on a latent-free web because the guard at 962 makes `latent` `None` and the tag invariant then clears every tag; the lease-order difference between `undercut` (515, lease before `drop_below`) and the two `MinWeb<()>` tails (924, 1008, lease after) verified by reading, with the pool row's derivation at meter.rs:9402-9410 written for lease-first and its three assertions unable to trip under lease-after); executed: no
- Seen by: structure ([1]), prose ([21]), correctness ([27]), claims ([38]); refutation: confirmed (four duplicates merged); history: deliberate-but-expired (c29bd2b3's op-sequence preservation; fe39fca3 broke byte-identity deliberately the same day)
- Owner-gated: no

The sequence "take `gap`, negate into the residue, drive the drop outward, re-seat `gap`" is written in `undercut`, in `emit_here`, and twice in `emit_offset`; the dominated arm re-implements `drop_below`'s follower loop and derives the residue by a separate hand fold, and that hand fold is the one place the polarity was wrong (d7293057). Folding `+offset` into `gap` on the dominated arm and calling one shared tail makes the residue arithmetic a single code path and the lease order a single documented decision, which is the doctrine's "the cheapest passing artifact must be the intended one" applied to a kernel with a recorded polarity bug.

Evidence:

       969	                web_traffic::record(web_traffic::Decision::DominatedUndercut);
       970	                // gap wide-negative: v sits far below the minimum; the
       971	                // drop dwarfs the offset. Residue = m − v = −gap − offset.
       972	                let mut residue = core::mem::take(&mut self.gap);
       973	                residue.negate();
       974	                fold_signed_int(&mut residue, offset.sign.negate(), &offset.magnitude);
       975	                for follower in self.followers.iter_mut().flatten() {
       976	                    follower.sub_accum(&residue);
       977	                }
       978	                let mut gap = self.lease();
       979	                fold_signed_int(&mut gap, offset.sign.negate(), &offset.magnitude);
       980	                self.gap = gap;
       981	                self.propagate(residue, |()| ());
       982	                return;

Resolution: give `undercut` the lease-after order (`let mut residue = core::mem::take(&mut self.gap); residue.negate(); self.drop_below(residue, on_die); self.gap = self.lease();`) and state the order in its doc; replace `emit_here`'s tail (921-924) with `self.undercut(|()| ())`; in `emit_offset` make the dominated arm `fold_signed_int(&mut self.gap, offset.sign, &offset.magnitude);` and give both undercut arms the one shared tail `self.undercut(|()| ()); fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);`. The comment at 970-971 becomes "`gap` holds `v − A` with no latent, so `drop_below`'s residue is `m − v`". Acceptance: one undercut tail in the file; `a_dominated_undercut_subtracts_its_residue_from_live_followers`, `dominated_latent_annihilates_into_the_undercut_residue`, and the `fill/tests.rs` differentials green; under `--features limb-meter` the `dominated_undercut_cost` floor and ceiling hold and `seam_stop_pool_misses_stay_at_warmup_across_churn_doubling` reads equal misses (a lease moved after a retire can only lower them, never below the first arming's one).

### skyline-watermark-19: The latent-ladder decision is written three times
- Where: crates/before/src/version/skyline/watermark.rs:986-1002 (related: watermark.rs:452-463, 1045-1063; tests/meter.rs:3417-3427)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified by reading each copy arm by arm against the proposed helper (no latent: the plain sign; latent live: `Equal | Greater` → `Greater`, a refusal → `Greater`, a surviving dominated latent → `Less`, a collapse → a fresh sign read), including side effects (the collapse inside `decide_undercut_through_latent`, the restore on every exit in `emit_offset`); on the dominated path the helper returns `Less` without the second sign read at 462/998, so touch readings can only fall, and the latent-ladder floor's premise does not count that read; executed: no
- Seen by: structure; refutation: confirmed (acceptance corrected: no `decide_undercut_through_latent` call site outside the helper); history: deliberate-but-expired
- Owner-gated: no

Legibility and correctness surface: "does the `v − A` in `gap` lie strictly below `m`" is a four-arm ladder (sign read; if a latent lives, the domination decision; plain re-test after a possible collapse) hand-copied into `undercuts_here`, the fold path of `emit_offset` (each false exit restoring the priced fold separately), and the latent branch of `compare_above`. Three copies in a kernel whose history includes a polarity bug are three places a future ladder edit must be made in lockstep, and the restore-on-every-exit shape is where a missed restore would silently displace the web.

Evidence:

       987	        fold_signed_int(&mut self.gap, offset.sign, &offset.magnitude);
       988	        if self.gap.sign() != Ordering::Less {
       989	            // v at or above the anchor, hence at or above the minimum.
       990	            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
       991	            return;
       992	        }
       993	        // v < A: only a drop past the latent too is a true undercut.
       994	        if self.latent.is_some() && !self.decide_undercut_through_latent() {
       995	            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
       996	            return;
       997	        }
       998	        if self.gap.sign() != Ordering::Less {
       999	            // A collapse re-based the anchor to m and v is not below it.
      1000	            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
      1001	            return;
      1002	        }

Resolution: add a private `fn cmp_min(&mut self) -> Ordering`, documented as "the ordering of `v` against `m` for the `v − A` that `gap` currently holds; may retire the latent (a funded collapse)": read `gap.sign()`; with no latent return it; with a latent map `Equal | Greater => Greater`, and on `Less` return `Greater` if the ladder refuses, `Less` if the latent survived (dominated), else `gap.sign()`. Then `undercuts_here` is `self.cmp_min() == Ordering::Less`, `emit_offset`'s fold path is one fold, one `if self.cmp_min() != Ordering::Less { restore; return }`, one undercut tail (finding 18), and `compare_above`'s latent branch is `let sign = self.cmp_min();`. Update `undercuts_here`'s doc (the shared form is anchor-relative, not `v = h`). Acceptance: `watermark/tests.rs`, the latent-ladder differentials in `fill/tests.rs`, and `skyline_min_ticks_latent_ladder_is_flat_per_unit` pass unchanged; `decide_undercut_through_latent` has no call site outside `cmp_min`; `watermark.rs` is net smaller.

### skyline-watermark-20: materialize and the lease/retire pool are an allocator riding the watermark type
- Where: crates/before/src/version/skyline/watermark.rs:1140-1150 (related: watermark.rs:878-897; fill.rs:695, 769, 980, 986, 1091; fill/prescan.rs:351, 368, 389, 555)
- Class / severity / confidence: modularity / nit / medium
- Provenance: verified (grep of `.lease()`/`.retire(`/`.materialize(` across fill.rs, prescan.rs, query/web.rs: the fill walk and pre-scan lease and retire buffers that never enter the web); executed: no
- Seen by: structure; refutation: confirmed (taste-level); history: deliberate-and-holds for one shared pool (the tick-cost-spec's "the fused walk's new paths never bypass the pool"), compatible with the split
- Owner-gated: no

Single responsibility: `materialize` has no range-minimum content and lives on `MinWeb<()>` only because `retire` does; a private `Pool` type with `lease`, `retire`, and `materialize`, held by the web and exposed as `pool(&mut self)`, would name the allocator, keep the miss meter in `Pool::lease`, and leave `MinWeb`'s method list to the watermark discipline. Taste-level; the cost is ~25 mechanical renames.

Evidence:

      1143	    pub(super) fn materialize(&mut self, mut dying: Accumulator) -> Signed {
      1144	        // Collapse for an honest width before the read-out: `sign()` is
      1145	        // called for its compaction side effect, the value unread.
      1146	        let _sign = dying.sign();
      1147	        let (sign, magnitude) = dying.sign_magnitude();
      1148	        self.retire(dying);
      1149	        Signed::from_sign_magnitude(sign, magnitude)
      1150	    }

Resolution: if taken, `struct Pool { free: Vec<Accumulator> }` in `watermark.rs` (or a sibling `pool.rs`) with the three methods; `MinWeb.pool: Pool`; internal `self.lease()`/`self.retire()` become `self.pool.lease()`/`self.pool.retire()`; external callers `web.pool().lease()`. Acceptance: `MinWeb`'s impl blocks contain no `materialize`; the pool row unchanged.

### skyline-watermark-21: The test module's unreachability claim is false for min_ticks, which reaches drop_below's latent annihilation with no directed public-API witness
- Where: crates/before/src/version/skyline/watermark/tests.rs:1-18 (related: watermark.rs:452-463, 497-505, 514-519, 541-545; query/web.rs:322-334; query.rs:437-470; query/tests.rs; src/meter.rs:2897-2916; testing/exhaustive.rs:83, 104)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified by reading the structural path: `ReignWeb::leaf` (query/web.rs:322-333) calls `undercuts_here` then `undercut` with no `compare_above` in front; `decide_undercut_through_latent` returns `true` with the latent still live on its gap-dominates exit (501-502); `undercut` → `drop_below` then annihilates at 541-545. The driver opens ranges only per left branch (`web.open(cursor.depth() - flip)`, query.rs:452), so a right leaf under a range whose left child just closed with a park emits with no pending range against a live latent — unlike the fill walk, where every child (leaf included) gets its own `web.open(1)` (fill.rs:505, 535, 544, 602, 608; prescan.rs:267) and a post-park emission always arms. Absence of a min_ticks-side witness verified by grep (`latent` occurs under `query/` only in web.rs; in tests/meter.rs only for the ladder and width-circulation families; the exhaustive scope is depth 2 over bases {0,1,2}). The numeric construction and closed form were traced by the correctness lens and re-traced by the refutation pass, not executed; executed: no
- Seen by: correctness; refutation: confirmed (re-traced the construction and both answers); history: no-rationale-found (29ee22fb's recorded reasoning covers only the `compare_above` readers; its tripwire that "the committed wide-arming and query suites stay green" under a dropped annihilation shows no committed query test drives the arm, not that a stream cannot)
- Owner-gated: no

Test doc comments must be accurate (an incorrect one is a bug in the test), and differential suites exercise the public API with internal-entry checks only as documented decisions whose stated rationale holds. Here the rationale is false for one of the two named gates: `drop_below`'s annihilation is reachable from `Version::min_ticks`, its value flow decides which `Reign` record counts the outer closes (so a skipped annihilation changes the answer), and no committed public-API test drives it deterministically.

Evidence:

         1	//! Direct pins on the web's latent-ladder gates that no packed-stream walk
         2	//! reaches, and on the seam contracts no packed stream drives.
         8	//! raise to the tracked minimum's emission — so `emit_offset`'s post-collapse
         9	//! restore and the undercut's latent annihilation in `drop_below` execute on
        10	//! no input either walk can be handed. The third latent-ladder arm, a

Resolution: add a directed min_ticks witness beside the query differentials (a `Version` built through the oracle's normalizing constructor, asserted against `oracle::Version::min_ticks` and the closed form), and re-state the module doc: the annihilation is reachable from `Version::min_ticks` (a right leaf emitted under a range whose left child parked) and unreachable from the two fill-side walks because every fill-side child opens its own range, so a post-park emission always arms; only `emit_offset`'s post-collapse restore is `MinWeb<()>`-only. Optionally add a decision tap on `decide_undercut_through_latent`'s three exits (the `web_traffic` idiom) so the min_ticks populations' coverage of each arm is a readable floor. Acceptance: a committed test under `Version::min_ticks` whose input parks a word-scale latent and then emits a drop that dominates it with an outer boundary still stacked, asserting the exact closed form, which fails when `residue.sub_accum(&latent)` at watermark.rs:543 is deleted; the module doc no longer claims the arm is unreachable from packed streams.
Construction: leaf heights in stream order `w = 0` (depth 1), `a = 2^34` (depth 3), `b = 2^34 + 2` (depth 4), `c = 2^34 + 1` (depth 4), `z = 1` (depth 2); the tree `R(w, O(X(a, I(b, c)), z))` is normal (no equal sibling leaves). Under `min_ticks`: `w` arms `R` at 0; `a` opens `O`, `X` and arms them 2^34 above (`Word(2^34)`, then a zero run); `b` opens `I` and arms it 2 above; `c` undercuts `I` to 2^34 + 1 (boundary 1, anchor `A = 2^34 + 1`); at `z` the step closes `I` (parks `Λ = 1`) and `X` (zero run), leaving `R`, `O` armed with diffs `[Word(2^34)]`; `z` emits with no pending range: `gap = 1 − A = −2^34`, the latent cannot dominate a two-digit gap, `gap.sign_dominates_at(0)` certifies (2^34 ≥ 3·2^32), `undercuts_here` returns true with the latent live, `undercut` → `drop_below` annihilates (residue 2^34 − 1), which stops at `Word(2^34)` and leaves `O`'s boundary at 1. Expected `min_ticks = (3·2^34 + 4) − (0 + 1 + 2^34 + 2^34 + 1) = 2^34 + 2`; with the annihilation skipped the residue 2^34 meets the outer boundary exactly, settles `R`'s record early, and the answer reads 2^34 + 1.

### skyline-watermark-22: The wide() helper doc misstates what a wide Int spelling does at the accumulator seam
- Where: crates/before/src/version/skyline/watermark/tests.rs:46-53 (related: signed.rs:203-210; codec/base.rs:42-44, 274-282; crates/suanpan/src/accumulator.rs:253-256, 279-284)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read the dispatch: `fold_signed_int` routes `Int::Wide` to `add_magnitude`/`sub_magnitude`; `Base::to_word` is `to_u64`; `add_magnitude` takes `add_u64` on `Some`, landing in the register exactly as `Int::Small` would; `add_wide` calls `spill()` unconditionally); executed: no
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (9c7b6999)
- Owner-gated: no

Test prose must be accurate about the mechanism it claims to exercise: within the word range the wide spelling is a no-op at the accumulator, so for `b < 64` in `a_drop_inside_the_latent_never_moves_the_minimum` it probes nothing; the regime change the proptests rely on is the unconditional spill past `2^64` (values in `(2^64, 2^96]` forced into the digit engine).

Evidence:

        48	/// Past the word range this is the only spelling; within it, the same value
        49	/// spelled wide is the redundant spelling the accumulator's certification is
        50	/// sensitive to, which is what makes it worth constructing deliberately.

Resolution: re-state: within the word range `Magnitude::to_word` dispatches the value to the word path, so the spelling is a no-op; past it, `add_wide` spills the register even for values the register could hold, which is the regime the proptests cross. Acceptance: the doc names the `to_word` dispatch and the spill; no claim that a `u64`-fitting wide spelling changes certification.

### skyline-watermark-23: The three-probe minimum read is written seven times
- Where: crates/before/src/version/skyline/watermark/tests.rs:91-105 (related: tests.rs:149-163, 197-211, 242-256, 341-355, 366-380, 415-429)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read all seven blocks; the wide and small spellings are deliberate per the doc at 46-50, and the assertion messages differ slightly across sites, e.g. 143); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Test code is held to legibility too: a named probe makes each test body read as its scenario followed by one line stating where the minimum must be. Two thin helpers (small and wide spelling) preserve the deliberate spelling distinction.

Evidence:

        91	    assert_eq!(
        92	        web.compare_above(&below(1000)),
        93	        Ordering::Equal,
        94	        "the probe at the true minimum reads exact"
        95	    );

Resolution: `fn assert_minimum_at(web: &mut MinWeb<()>, h_minus_m: u64)` and `fn assert_minimum_at_wide(web, h_minus_m: &UBig)` (a `Result`-returning form for the `prop_assert_eq!` sites), replacing the seven blocks. Acceptance: each test ends with one or two helper calls; test count and names unchanged; `just test watermark` green.

### skyline-watermark-24: A proptest's closing comment describes a follower and a parked boundary it never builds, and drop_below's tagged-follower arm runs in no test
- Where: crates/before/src/version/skyline/watermark/tests.rs:356-364 (related: tests.rs:108-164, 289; watermark.rs:529-547)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified by trace (the file's only `follower_set` is line 289, in the other proptest; `open(2); emit_here()` leaves `[ZeroRun(1)]`; at the trailing undercut the latent, if it survived the refusal, is collapsed by `decide_undercut_through_latent` at comparable scales (`|gap| = Λ + 1` against `Λ`) before `drop_below`; the close at 364 pops `Close::ZeroRun`, so nothing parks and the three final probes read the re-seated zero `gap` plus 100 whatever the residue's value or sign); `drop_below`'s loop with a live latent (531-539) is reached by no test in the file because the dominated-undercut proptest takes the inlined arm at 975-977 and the direct annihilation pin installs no follower; executed: no
- Seen by: prose ([11]), correctness ([25]), claims ([37]); refutation: confirmed (with the corrected expected read-out below); history: no-rationale-found (9c7b6999's blob had no follower in this test either)
- Owner-gated: no

A test's English must state what it verifies: a reader would believe residue polarity into tagged followers and parked boundaries is pinned by this family when it is not, and the one arm the comment names — `drop_below` with a live latent and a live follower, whose value flow depends on the follower fold (530-540, subtracting the pre-annihilation `A − v`) running before the annihilation (541-545) — is exercised nowhere. "Not wrong, but you couldn't tell if it were" is repairable, and the repair is cheap.

Evidence:

       358	        web.fold_height(Sign::Negative, &wide(&(&height + UBig::from(1u8)))); // h = −1
       359	        web.emit_here(); // v = −1: past m = 0, a true undercut
       360	        // The undercut runs with the outer range still armed, so it propagates
       361	        // to a live follower. Close the dropped range and read the outer
       362	        // minimum back through the boundary that parked: the value survives
       363	        // only if the follower's residue moved at the right polarity.
       364	        web.close();

Resolution: rewrite 360-363 to what runs ("the refusal left the web able to fire a true undercut; its residue passes the outer pair's zero run, the close pops that run, and the outer range reads the undercut's value exactly"), optionally adding `prop_assert!(matches!(web.close(), Close::ZeroRun))`. Then add a directed pin for the missing arm in the `dominated_latent_annihilates_into_the_undercut_residue` scenario: install `follower_set(0, acc)` holding `start` after the first arming; after `emit_offset(&below(50 + E))` take and `materialize` it and expect `start + D − E` (the arms fold `+D` and `+50` into the follower via `push_boundary`, the park tags it, the undercut subtracts the pre-annihilation `E + 50`). Acceptance: the new pin fails when the annihilation block is moved above the follower loop (reads `start + D + 50 − E`) and when line 537's `sub_accum` becomes `add_accum` (reads `start + D + 50 + E + 50`); the rewritten comment names no follower and no parked boundary.
Construction: mutant 1 swaps the order of the `for slot in 0..self.followers.len()` loop and the `if let Some(latent) = self.latent.take()` block in `drop_below`; mutant 2 changes line 537 to `add_accum`. All seven tests in the file pass under both today.

### skyline-watermark-25: Vocabulary collisions: 're-arm' for lease and 'fill phase' beside the fill walk
- Where: crates/before/src/version/skyline/pool_traffic.rs:4-18 (related: watermark.rs:82; tests/meter.rs:9434, 9446, 9452; .cargo/mutants.toml:70; src/meter.rs:2824; src/meter.rs:1683-1802 for the third sense)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n 'fill phase\|re-arm'` over the partition, tests/meter.rs, and .cargo/mutants.toml); executed: no
- Seen by: prose ([20]), claims ([40c]); refutation: confirmed (the word is imported from suanpan's `reset` doc, accumulator.rs:652-654); history: no-rationale-found; the collision is three-way, since 're-arm' also names the promotion re-arm meter family
- Owner-gated: no

Each coined term anchors to one identifier: 'arm' is this module's defined operation on pending ranges (`arm_at_height`, `arm_below`, `arm_relative`), yet two docs use 're-arm' for leasing a pooled buffer; 'fill phase' for the pool's warm-up sits beside `fill.rs`, the walk every neighbour calls the fill walk, and the pinning test's own name already uses the unambiguous word ("warmup").

Evidence:

         5	//! (`MinWeb::retire`) and re-arms from it (`MinWeb::lease`), so range
        16	//! constant bounded by the walk's peak outstanding leases (the fill
        17	//! phase); churn-proportional misses mean the recycle is dead. The

Resolution: watermark.rs:82 "return to a pool and are leased again cleared"; pool_traffic.rs:5 "and leases from it (`MinWeb::lease`)"; pool_traffic.rs:16-17 "(the warm-up)"; the same 'fill phase' recurs at tests/meter.rs:9434, 9446, 9452, .cargo/mutants.toml:70, and src/meter.rs:2824 (outside this partition). Acceptance: within the partition 'arm' refers only to range arming and 'fill' only to the fill walk.

### skyline-watermark-26: Four hand-rolled counter modules of one shape
- Where: crates/before/src/version/skyline/pool_traffic.rs:27-60 (related: web_traffic.rs:57-113; src/version/hull_traffic.rs:58-118; src/codec/scan.rs:24-72)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (read all four modules in full; the two classified counters duplicate the `cell(enum) -> &'static AtomicU64` dispatch and a hand-listed `reset` loop); executed: no
- Seen by: structure; refutation: confirmed (at best parity: the snapshot structs carry per-field rustdoc and differ in arity); history: no-rationale-found (a recorded copy chain: scan → hull_traffic → web_traffic → pool_traffic, never questioned)
- Owner-gated: no

The idiom stated once over copies that drift (finding 27 is a drift instance: the fourth copy claims a sibling's idiom while adopting a different gate). Each module is 30-60 lines and its doc is the valuable part, so this is take-or-leave.

Evidence:

        27	#[cfg(feature = "limb-meter")]
        28	mod counter {
        29	    use core::sync::atomic::{AtomicU64, Ordering};
        30	
        31	    static MISSES: AtomicU64 = AtomicU64::new(0);
        56	#[inline(always)]
        57	pub(crate) fn record_miss() {
        58	    #[cfg(feature = "limb-meter")]
        59	    counter::record_miss();
        60	}

Resolution: if taken, a `macro_rules!` in `codec/scan.rs` expanding to the gated `counter` module, the `use`, and the shim, parameterized by feature and by a single cell or a classification enum; each module keeps its doc and becomes one invocation. If declined, no action beyond finding 27. Acceptance: one `mod counter` definition site; the four read surfaces in `meter.rs` unchanged.

### skyline-watermark-27: The two watermark counters sit under different gates, and web_traffic misnames the gate of the idiom it cites
- Where: crates/before/src/version/skyline/web_traffic.rs:19-24 (related: web_traffic.rs:46, 57, 100, 109; pool_traffic.rs:20-25, 27, 49, 58; Cargo.toml:49, 74-95; src/meter.rs:3668-3699; fill/tests.rs:459, 468, 597, 600; lib.rs:438-439; codec/scan.rs:24; hull_traffic.rs:16-17)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (web_traffic's `counter`, `use`, and `EmitTraffic` are `#[cfg(feature = "meter")]`; pool_traffic's are `limb-meter`; Cargo.toml:49's self-dev-dependency enables `meter` for every bench and test build; Cargo.toml:80-83 and 92-94 state that `limb-meter` and `scan-meter` exist to keep per-primitive relaxed-atomic bumps "out of the bench builds"; codec/scan.rs:24 is `scan-meter`, so both web_traffic.rs:19-20 and hull_traffic.rs:16-17 cite an idiom under the wrong gate name; `fill/tests.rs` reads `crate::meter::emit_traffic()` at 459-468 and 597-600 with no `#[cfg]` anywhere in that file); executed: no
- Seen by: structure ([5]), claims ([41]); refutation: [5] reframed (re-gating is not consumer-free: the two fill/tests.rs readers must move under the gate; hull_traffic is a same-shaped per-call counter under `meter`, so the frequency argument has an opposite precedent), [41] refuted (the gate names a frequency class, not a capability dependency); history: no-rationale-found (both the gate and the inaccurate citation were inherited by copy from hull_traffic in 76579a3c)
- Owner-gated: yes: which frequency class the `meter` feature admits is a gate-policy decision; both directions have in-tree precedent

Two counters on one struct under two gates with no stated reason, and a doc that names the wrong idiom: `web_traffic::record` fires on every latent-free word-scale priced emission in the tick kernel the bench judge times (`emit_offset` 966, 969, 984) under `meter`, which the self-dev-dependency puts in every bench build, while Cargo.toml's stated reason for `limb-meter` and `scan-meter` is to keep per-primitive bumps out of that profile; `pool_traffic::record_miss` fires only on a miss (bounded by peak demand) yet sits under `limb-meter`. Either the cost argument moves web_traffic under `limb-meter` (with the two `fill/tests.rs` readers cfg'd) or Cargo.toml states that `meter` admits decision-liveness counters (hull_traffic's per-call counter is the precedent); in both cases lines 19-20 should name the gate they actually share.

Evidence:

        19	//! The recording compiles to nothing without the `meter` feature — the
        20	//! [`codec::scan`](crate::codec) counter's idiom — and the readings are

Resolution: owner's choice of direction. (a) Gate `web_traffic`'s `counter`, `use`, and `EmitTraffic`, plus `meter::emit_traffic`/`reset_emit_traffic` and the re-export at meter.rs:81, on `limb-meter` exactly as `pool_misses` is, and put `#[cfg(feature = "limb-meter")]` on the two `fill/tests.rs` witnesses' counter asserts (or on the tests). (b) Add to Cargo.toml's `meter` comment that per-decision liveness counters (`hull_traffic`, `web_traffic`) are admitted, and state why a per-emission bump in the tick kernel is acceptable in bench builds. In either case reword web_traffic.rs:19-20 (and hull_traffic.rs:16-17, outside this partition) to name the actual gate of the cited idiom. Acceptance: one stated policy the two watermark counters both satisfy; `grep -rn "codec::scan.*idiom"` finds no citation under a gate `codec::scan` does not use; under (a), `cargo build --features meter` compiles `emit_offset` with no `web_traffic` statics and `cargo test -p before --all-features` runs `dominated_undercut_cost` and both fill witnesses green.

### skyline-watermark-28: Two of three emit-traffic counters are recorded but never read, and the snapshot doc overstates what they sum to
- Where: crates/before/src/version/skyline/web_traffic.rs:42-55 (related: watermark.rs:962-985, 1036-1043; tests/meter.rs:9250; fill/tests.rs:468, 600)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (`grep -rn -E '\.dominated_above|\.undecided|Decision::Undecided|Decision::DominatedAbove' crates/before --include='*.rs'` matches only the record sites at watermark.rs:966 and 984 and web_traffic's own dispatch; every reader reads `.dominated_undercut`; `compare_above`'s `sign_dominates_word` at 1039 records nothing); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (the three-cell shape and the "fields sum to" sentence were copied from hull_traffic's four-rung idiom; 76579a3c pinned only `dominated_undercut >= k`)
- Owner-gated: no

Circular justification: a counter earns its existence by naming what reads it. The module's own liveness argument (an undriven arm is where a polarity error waits) applies to the undercut arm, which has a floor; `DominatedAbove` and `Undecided` are decoration until a family pins them. The doc sentence is also inaccurate: `compare_above`'s domination read is unclassified, so the fields sum to the emission-path reads only.

Evidence:

        42	/// A snapshot of the three decision counters, in emissions.
        43	///
        44	/// Read through `meter::emit_traffic`; the fields sum to the domination
        45	/// reads performed since the last reset.

Resolution: either (a) pin `dominated_above` on a committed family that provably routes emissions through the return-early arm (the dominated-undercut family's own sites may yield a derivable count; state the derivation in the floor doc) and fold `Undecided` into a single fallback cell, or (b) reduce `EmitTraffic` to the one enforced cell. In both cases reword 44-45 to "the emission-path domination reads". Acceptance: every field of `EmitTraffic` is read by at least one committed floor or band, or the struct has one field; the doc names the emission path.
Construction: delete the `record(Decision::DominatedAbove)` call at watermark.rs:966 and run the full suite: nothing turns red.

## Positives

- The payload seam is a model of a zero-cost generic: `payload: impl FnOnce() -> P` constructs lazily only on the arms that store or kill a boundary (655, 664), `on_die: impl FnMut(P)` fires at exactly the difference's death (726, 732, 780, 826, 834), `Close<P>` makes the min-ticks dispatch total (query/web.rs:259-279 matches all three arms), and the fill client at `P = ()` spells `|| (), |()| ()` and pays nothing.
- Every cost claim in the module doc names a committed instrument and each name resolves: `skyline_min_ticks_latent_ladder_is_flat_per_unit` (tests/meter.rs:3449) for the O(1) latent decision, the `skyline_min_ticks_seam_*` bands for `propagate`'s wide hops, `pool_recycle` (9382) for the pool claim, `dominated_undercut_cost` (9209) for the arm-liveness floor. The suanpan witnesses are cited by stable test name with the assumed clause restated inline (488-494, 751-771), which is the citation discipline the hard rules ask for.
- The latent-ladder band (tests/meter.rs:3434-3490) is a genuine order pin, not an envelope: a per-decision `k`-marginal compared across a doubling of the parked latent's width in both directions, with a floor derived from three irreducible register folds.
- Every `debug_assert!`, `expect`, and `unreachable!` message in the partition is a one-line proof of why its branch is programmer error (318, 322, 326, 339, 370, 390, 422, 535, 785, 805, 857), and both `unreachable!` arms are rostered in tools/covcheck-expected.json (146-157) with the positivity argument rather than excluded.
- `propagate` states its loop invariant and the deferred-zeros flush in comments a reader can check against every `break`/`continue` (697-706), and `compact` refuses to normalize a wide difference just to learn it will not fit (855).
- `pool_traffic.rs:4-18` passes the circular-justification test in writing: it names the property no other meter can see (a dead recycle leaves peak heap and every touch reading byte-identical) and the row that pins both directions; the row's ceiling `SEAM_STOP_POOL_WARMUP = 2` is derived from peak simultaneous demand (tests/meter.rs:9399-9411), not measured, and .cargo/mutants.toml:65-70 records that the row, not an exclusion, kills the retire/lease deletion mutants.
- `watermark/tests.rs` declares itself an internal-entry suite at its head with a reachability argument, pairs each worked pin with the proptest family it is a point of (309-322), and each worked pin's doc states the exact displacement a polarity error would produce (72-73, 117-118, 177-178), so the three-point anchor probe is demonstrably sensitive in both directions.
- `MinWeb::new` and `MinWeb::compacting` each carry the basis for their compaction choice with the row named, so `compact_words` is a measured decision rather than a bare bool (finding 8 is about the form of that record, not its existence).

## Open questions for Finch

1. Fill-side unreachability of a live-latent undercut is prose only. tests.rs:4-10 argues it from "only behind `compare_above`"; what I could verify is a different, structural argument: every fill-side child, leaf included, gets its own `web.open(1)` (fill.rs:505, 535, 544, 602, 608; prescan.rs:267) and `copy_subtree`/`copy_range`'s per-leaf loops (fill.rs:1071-1073, prescan.rs:485-488) close nothing inside, so a post-park emission always arms and `push_boundary` consumes the latent by merge. I did not exhaust every emission site. Recommendation: when re-stating the module doc per finding 21, write the structural argument and, if you want it mechanical, a `debug_assert!(self.latent.is_none() || self.pending > 0)` at the top of `MinWeb<()>::emit_here`/`emit_offset`'s non-arming path would turn the claim into a checked one for the fill client (min_ticks does not go through `MinWeb<()>`).

2. The pool floor `small >= 1` (tests/meter.rs:9444-9448, "the fill phase always misses") rests on the first arming leasing before it retires the initial zero `gap` (569-571). On the seam-stop family at least one miss is irreducible regardless of order (gap plus one stacked boundary exceed the constructor's single buffer), so the floor's premise holds for that family, but it is not the premise the message states. Recommendation: reword the floor's message and doc to "the family's peak demand exceeds the constructor's one buffer", which survives the lease-order unification in finding 18 (envelopes partition).

3. `decide_undercut_through_latent` (485-506) reads domination with no clearance guard while `propagate` (772, 790) guards at `+ 2` digits; `a_drop_short_of_the_latent_minimum_refuses_the_undercut` (D = 2^36, drop 50) shows the unguarded read is load-bearing there (a register-held latent certifies at one digit of clearance). The two ladders are correctly different, but nothing at either site says why the other's guard would be wrong here. Recommendation: one cross-referencing sentence at 495-497; moot for `propagate` if finding 14's primary resolution lands.

4. By-value `Accumulator` moves: `Boundary::Wide(Accumulator)` is held by value, so every `Entry<P>` is on the order of 100 bytes and every `mem::replace`/`take` of `gap`, every `lease`/`retire`, and every `Entry` push/pop copies that much. Whether `Box<Accumulator>` (pointer-sized moves, one indirection per fold) wins is workload-dependent and unmeasured; `size_of::<Entry<Reign>>()` is estimated from field layout, not measured. Recommendation: not a finding; if you want to know, print the two sizes in a test and judge a prototype on the ascend row's heap ceiling and the tick/min_ticks bench cells on a quiet machine. Nothing lands on anticipated benefit.

5. The module doc and both constructors name the two clients by module and describe their behavior (8-9, 93-95, 207-209, 222-227, 245-256, 312-314). The module's thesis is that "each client contributes only its own semantics through the payload seam". Recommendation: keep the client names (a reader needs to know who drives the web) but move the client-specific measured bases to the clients' construction sites (fill.rs:303, query/web.rs:231), where the row a client's choice rests on is beside the choice; this also resolves finding 8's placement question.

6. Out of partition, with dispositions: hull_traffic.rs:16-17 carries the same wrong-gate idiom citation as web_traffic.rs:19-20 (whoever owns version/hull_traffic.rs); query/web.rs names a constructor `Reign::mint` and uses 'mint' eight times, inside writing-style.md:170's "in code" clause (skyline-query partition); 're-arm' is also the promotion re-arm meter family's name (src/meter.rs:1683-1802), a third sense colliding with finding 25.

## Dropped

- [21], [27], [38] (emit_here/emit_offset re-implement undercut): duplicates of skyline-watermark-18.
- [25], [37] (the proptest comment describes a follower and parked boundary): duplicates of skyline-watermark-24.
- [14], [36] (moralized 'honest', 'mint'): merged into skyline-watermark-1.
- [28] (compact's width claim, tight figure): merged into skyline-watermark-15 with its figure.
- [29], [34] (compacting ratios; mechanism): merged into skyline-watermark-8.
- [40] (hand-counted 'two'; arm_below fold attribution; 'fill phase'): split into skyline-watermark-4, -5, and -25.
- [2] (propagate's mirror-image guards): merged into skyline-watermark-14 as the fallback resolution.
- [41] (pool_traffic should be gated on `meter`): refuted; Cargo.toml:74-84 defines `limb-meter` by frequency class, not by a dependency on `suanpan/touch-meter`, so the "needs only `meter`" premise misreads the gate. Its capability point (a per-miss bump is rare) survives as one side of skyline-watermark-27's owner decision.
- [39] (Box<Accumulator>): below the bar as a finding; nothing verified and the sign is workload-dependent. Moved to open question 4.
- [19]'s second half (the `memo modules` citation): weak on its own (an informal descriptor, not a wrong identifier); folded into skyline-watermark-8 as a one-word correction.
