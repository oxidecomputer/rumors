# Partition skyline-sweep-place-masked: Comparison kernels: sweep, place (and its filter), masked, overlay, signed

## Partition summary

This partition is the comparison layer of `before`'s skyline encoding. `overlay.rs` supplies the cursor vocabulary (`PlateauCursor`, `LeafCursor` over a skyline stream, `IdLeafCursor` over a packed id stream), the overlay-advance law in a binary form (`advance`) and an N-ary form (`advance_set` over a `CursorSet`), and the pair-difference algebra (`OpenedPair`, `Side`, `fold`, `advance_diff`). `sweep.rs` folds `sign(height_a - height_b)` once per elementary interval into a `Directions` pair and asks four questions of it through one generic loop. `masked.rs` generalizes that to up to four streams (two event streams and two optional id masks) with per-side height integrators and a block skip over unowned runs. `place.rs` fuses one probe against a span's two bounds with per-question verdict hooks, and `place/filter.rs` fuses one or two probes against any number of demand-carrying bounds for `causally`'s membership and coverage verdicts. `signed.rs` is the sign-magnitude substrate: the zigzag maps, the signed folds and sums, the fused gamma coder, and the signed comparisons.

The four `tests.rs` files (`sweep/tests.rs`, `place/tests.rs`, `masked/tests.rs`, `signed/tests.rs`) are test code. I read all ten partition files in full (4077 lines) and, to settle cross-file claims, `shape.rs`, `admit.rs`, `walk.rs`, `query.rs`, `codec/stack.rs`, `codec/base.rs`, `version/own.rs`, `version.rs` (the `PartialOrd` impls), `causally/query.rs`, `causally/conjunction.rs`, `causally/polarity.rs`, `causally/tests.rs`, `laws.rs`, `party/ops/diff.rs`, `party.rs`, `party/forks.rs`, `idbits.rs`, `src/meter.rs`, `tests/meter.rs` (the placement, early-exit, and masked-hole rows), `.cargo/mutants.toml`, and suanpan's `accumulator.rs` at the cited ranges. No cargo, just, or test command was run; every finding below is settled by reading, grep, git history, or a hand trace, and each says which.

The kernel layer is correct as far as every lens and this pass can trace it. `overlay.rs` states the advance law's correctness argument (nesting, the flip-level tie test, exhaustion by the all-right path) once, beside the mechanism, with a debug assertion at every tie; every `expect` and `unreachable!` in the partition is a one-line proof drawn from that argument; the verdict hooks act only on permanent refutations so every early exit is sound by construction; the fused gamma coder is pinned against the unfused composition at the `2^31` seam. What the review finds is mostly at the verification and claim layer. The one high finding is an asymptotic violation on a production path: `masked::Walk::block_skip` evaluates `LeafCursor::peek_flip` on every round for an unowned masked side even when that side is not the deepest slot and will not move, and `peek_flip` costs one word read per 64 bits of the path's trailing right-branch run, so a stationary deep cursor overlaid by a deep other-side subtree pays a quadratic term that no committed meter counts. Two medium claim and instrument gaps sit beside it: the filter walks' `O(|v| + Σ|bound|)` omits a factor of the bound count that the code plainly pays, and the coverage walk's three documented early exits, the production `order_exit`, and the placement walks' write-sequence identity each have no committed instrument that fails when the mechanism is removed. A medium prose defect is the ghost of a retired oracle: `sweep.rs` names `Version`'s `PartialOrd` as the verdict oracle, but that has been the sweep itself since the flag-day commit.

The remaining findings are the duplication the brief asked about (the pair seeding spelled at four production sites while `OpenedPair` claims one home; the arity-N advance law restated in `shape.rs`; two same-named `IdLeafCursor` structs) and a batch of nits on idiom, vocabulary, and copy-pasted rationale paragraphs. Several are claims that were true the day they were written and expired within days without re-denomination; the history pass documents that pattern and it is worth Finch's attention as a process observation.

## Findings

### skyline-sweep-place-masked-1: Rustdoc link syntax inside a `//` comment, and a ragged module-doc wrap
- Where: crates/before/src/version/skyline/masked.rs:110-111 (related: crates/before/src/version/skyline/masked.rs:38-40)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (a736ef14 wrote the link into a plain comment, where rustdoc never resolves it)
- Owner-gated: no

A plain `//` comment carries intra-doc link brackets that rustdoc never renders, and the module doc's paragraph at 38-46 wraps as a full line, a nine-word fragment, and a full line: both read as paste and rewrap artifacts.

Evidence:

       110	    // At exhaustion every surviving combination is a verdict
       111	    // ([`Directions::relation`]'s map).

        38	//! The single-owner reads are the comparison trichotomy's (`<`/`=`/`>`)
        39	//! zero-check: distinguishing `=`
        40	//! from `<` requires knowing whether the *other* operand has positive height

Resolution: write `// (Directions::relation's map).` and rewrap lines 38-46 to the module's width. Acceptance: no `[`...`]` link syntax inside a `//` comment in masked.rs; the paragraph wraps evenly.

### skyline-sweep-place-masked-2: `masked::Walk` carries two correlated `Option`s re-derived by `expect`, and its A/B arms are mirror copies
- Where: crates/before/src/version/skyline/masked.rs:158-164 (related: crates/before/src/version/skyline/masked.rs:195-206, 244-259, 317-345, 394-432)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (the Option-ness is stated inline; the two-parallel-Options shape is 6c88c2ad's original, never restructured)
- Owner-gated: no

`height_a` is `Some` exactly when `b_mask` is `Some` (constructed so at 197-201) and dually for `height_b`/`a_mask`; the correlation is carried as a doc sentence plus two runtime `expect`s (247, 256). Types-first doctrine: an invariant two fields share is a struct. Separately `block_skip` (319-342) and `CursorSet::step` (396-422) are A/B mirror images differing only in field names and `add_accum`/`sub_accum`.

Evidence:

       158	    /// `D = h_a − h_b`, the both-owned intervals' sign source.
       159	    diff: Accumulator,
       160	    /// `h_a`, maintained only when `b` is masked (the only case that reads it:
       161	    /// `a` owned alone compares `h_a` against zero).
       162	    height_a: Option<Accumulator>,
       163	    /// `h_b`, maintained only when `a` is masked, dually.
       164	    height_b: Option<Accumulator>,

Resolution: introduce `struct Mask<'a> { id: IdLeafCursor<'a>, other_height: Accumulator }` and store `a_mask: Option<Mask>`, `b_mask: Option<Mask>`; the single-owner arms then read the height through the mask they already matched on, deleting both `expect`s and the doc sentence. Optionally factor `block_skip`'s two arms into one per-side helper over disjoint borrows. Acceptance: no `expect("a masked ...")` in masked.rs; the `masked_cmp_*` envelopes and the projection laws are unchanged.

### skyline-sweep-place-masked-3: The masked height debug-asserts panic in debug builds on input the Panics contract says sweeps silently
- Where: crates/before/src/version/skyline/masked.rs:244-250 (related: crates/before/src/version/skyline/masked.rs:252-260, 96-103; crates/before/src/version/skyline/masked/tests.rs:15-72; crates/before/src/version/skyline/sweep.rs:87-93)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (hand trace of the walk state against the code, not run); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (independent trace); history: no rationale found (the asserts are 6c88c2ad's; 8b1db79d transplanted the sweep's Panics sentence two weeks later and pinned only the collapsible-pair half; neither commit reconciles them)
- Owner-gated: no

The Panics section promises that a delta driving the running height negative "sweep[s] silently"; under `debug_assertions` the two single-owner arms assert the height sign is not `Less` and panic on exactly that input whenever a single-owner interval reads the integrator. The same sentence in sweep.rs is true (the pair sweep has no height assert); masked's copy is false in debug and test builds. Statement faithfulness is breached, and the guard names no failure the committed suites miss: on canonical input a negative height needs a fold-orientation bug, which misreads the interval's sign and separates from the projection laws.

Evidence:

       244	                    let height_sign = self
       245	                        .height_a
       246	                        .as_mut()
       247	                        .expect("a masked `b` maintains h_a")
       248	                        .sign();
       249	                    debug_assert_ne!(height_sign, Ordering::Less, "heights are nonnegative");
       250	                    height_sign

       100	/// operands canonical packed ids. The violations the walk structurally notices
       101	/// (truncation, malformation) panic; the rest (a collapsible sibling pair, a
       102	/// delta driving the running height negative) sweep silently, and the verdict
       103	/// is then unspecified.

Resolution: delete the two `debug_assert_ne!` lines (249 and 258) and keep the contract as written. If the owner prefers the guard, amend the Panics section to say a negative running height trips a debug assertion, and either way extend `collapsible_sibling_pair_sweeps_without_panicking` (or add a sibling) with the negative-height witness so the sentence's second case is pinned. Acceptance: a debug-profile test feeding the witness below through `masked::causal_cmp` and `masked::eq`, in both operand positions, returns without panicking.

Construction: `a` = bits `0` (internal root), `1` + gamma(5) (left leaf, absolute 5), `1` + gamma(11) (right leaf; `unzigzag(11)` is odd, so `(Negative, 11/2 + 1 = 6)`, height -1 on [1/2, 1)); `b` = `1` + gamma(0) (single leaf 0); `b_mask` = packed id `10 00` (left child present and terminal, right absent: owned on [0, 1/2) only). `validate_bits` rejects `a` (skyline.rs:74-75, 251-252), so the witness sits outside the precondition exactly as the collapsible pair does. Trace: `Walk::open` seeds `diff = 5`, `height_a = Some(5)`; round 1 reads (owned, owned) and `Greater`; `advance_set` under priority `[B_MASK, B, A_MASK, A]` steps `B_MASK` (depth 1, flip 1, right child absent so `owned = false`) and ties `A` (depth 1 >= 1), decoding -6 into `diff` (-1) and `height_a` (-1); round 2 is the `(true, false)` arm, `height_a.sign() == Less`, and line 249 fires. In release the call returns `None`.

### skyline-sweep-place-masked-4: The masked-hole band's flatness rests on zero deltas in the skipped run; the docs attribute it to "accumulator work"
- Where: crates/before/src/version/skyline/masked.rs:313-316 (related: crates/before/src/version/skyline/overlay.rs:377-382; crates/before/src/meter.rs:146-156, 3506-3515; crates/before/tests/meter.rs:7515-7544; crates/suanpan/src/accumulator.rs:219-226, 718-722)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read `skip_deeper`, `ev_spine`, and suanpan's `add_u64`/`sign`); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (a6385d5f's generator doc states the depth-independence; the zero-delta premise is stated nowhere)
- Owner-gated: no

The block skip removes per-interval sign reads, not folds: `skip_deeper` (overlay.rs:377-382) folds every skipped delta into `net`, and suanpan's `add_u64` returns before touching only when the delta is zero. The band reads flat across a spine doubling because `ev_spine`'s skipped right siblings are `ev_leaf(bits, 0)`; on a spine with nonzero deltas the touch reading grows with depth with the skip engaged. A maintainer reading "accumulator work is a function of the mask depth alone" takes the skip for an asymptotic mechanism; it is a constant-factor one, and the premise is stated nowhere near the claim.

Evidence:

       313	    /// The `masked_cmp_hole` envelope and its depth band (`tests/meter.rs`)
       314	    /// pin the skip engaging: on the masked-hole triple the comparison's
       315	    /// accumulator work is a function of the mask depth alone, flat across
       316	    /// a spine-depth doubling.

    src/meter.rs
       153	    for _ in 0..d - 1 {
       154	        ev_leaf(bits, 0); // each ancestor's right sibling
       155	    }

Resolution: state the pinned mechanism as "no per-interval sign read inside an unowned run" at masked.rs:313-316 and at the generator (src/meter.rs:3506-3515) and band (tests/meter.rs:7515-7517), and name the zero-delta premise at the generator. Optionally add a nonzero-delta spine variant whose reading is expected linear, as the fold floor. Acceptance: the prose names sign reads and the zero-delta premise; the band is unchanged.

Construction: replace the spine's right-sibling leaves with alternating 0/1 heights (still canonical); under the current code the touch reading grows with `d` at both band points with the skip engaged, so the flat ceiling fails though the skip works: the band pins sign reads, not folds.

### skyline-sweep-place-masked-5: `block_skip` re-peeks a stationary cursor's trailing run every round: Θ(L·r/64) word reads on Θ(r + L) input
- Where: crates/before/src/version/skyline/masked.rs:317-345 (related: crates/before/src/version/skyline/overlay.rs:347-354; crates/before/src/codec/stack.rs:104-123; crates/before/src/version/skyline/masked.rs:48-59; crates/before/src/version/own.rs:113-121; crates/before/src/lib.rs:350-358; crates/before/tests/meter.rs:7434-7544)
- Class / severity / confidence: claim / high / high
- Provenance: assessed (hand trace against the code and `BitStack::trailing_ones`, not run); executed: no
- Seen by: claims; refutation: confirmed (independent trace); history: no rationale found (stack.rs:106-107 prices each peek per call and admits the no-pop case; no record analyzes a stationary re-peeked cursor)
- Owner-gated: no

The masked cost claim (masked.rs:54-56: scan, decode, stack, and fold work all linear in the operand streams' bits) is a hard guarantee under lib.rs:350-358. On every round, for a masked side whose current region is unowned, `block_skip` evaluates `self.a.peek_flip()` (and the `b` twin) before comparing it to the other slots' depth. `peek_flip` is `len - trailing_ones()`, and `BitStack::trailing_ones` walks one word per 64 bits of the trailing right-branch run. When that side is not the deepest slot it does not step, so the same run is re-read on every round while the other side consumes its subtree. The extra work is a read of the path stack, so it is invisible to every committed deterministic meter (touch, heap, segments, limb, scan), and it is on the production path: `OwnVersion`'s comparisons route through `masked::causal_cmp` (own.rs:113-121).

Evidence:

       319	            let a_bound = self.others_deepest(Self::A);
       320	            if self.a_mask.as_ref().is_some_and(|mask| !mask.owned())
       321	                && self.a.peek_flip() > a_bound
       322	            {

    overlay.rs
       352	    pub(super) fn peek_flip(&self) -> u64 {
       353	        self.path.len() - self.path.trailing_ones()
       354	    }

    codec/stack.rs
       106	    /// One word read per 64 bits of the run: the cost is the run the caller is
       107	    /// about to pop (or has decided not to), never the whole stack. `u64`,
       ...
       115	        for &word in self.words.iter().rev() {
       116	            let w = word.trailing_ones();
       117	            run += u64::from(w);
       118	            if w < 64 {
       119	                break;
       120	            }
       121	        }

Resolution: guard each peek with the depth test the skip already implies: `self.a.depth() > a_bound && self.a.peek_flip() > a_bound` (and the `b` twin). Since `peek_flip() <= depth()`, the guard is a necessary condition and changes no step; when it holds, `a` is the unique deepest slot, so if the skip does not engage `advance_set` steps `a` this same round and pops exactly the run the peek read, and every peek is then amortized against an immediate pop. (Caching the flip level in `LeafCursor` at open and step time has the same effect.) State the amortization premise the caller must keep in `peek_flip`'s doc (overlay.rs:347-354). Acceptance: a committed two-scale band on the family below (fuel per packed byte under the deterministic wasm fuel meter, or the bench judge's wall exponent) reads flat across a doubling of `(r, L)`: red at HEAD, green after the guard; the existing `masked_cmp_*` rows and `masked_cmp_hole_depth_band` unmoved.

Construction: `a` is a skyline whose preorder reaches a leaf at path `[left, right × r]` (the last leaf of the left half): root internal, left child internal, each spine node with a left leaf sibling and a right internal child, heights alternating 0/1 so no sibling pair collapses (about 5r bits). `a_mask` is the party owning only the right half, so [0, 1/2) is one unowned region at depth 1. `b` is unmasked, with the same left-spine skeleton to depth r + 1 and then a dense subtree of L leaves inside `a`'s last left-half leaf [1/2 - 2^-(r+1), 1/2) (about 5r + 5L bits). Once the sweep enters `a`'s leaf, `a.path = [0, 1 × r]` (depth r + 1, trailing run r, `peek_flip() == 1`). For each of `b`'s L boundaries there, `block_skip` computes `a_bound = depth(b) > depth(a)`, the mask is unowned so `peek_flip()` runs and reads ceil(r/64) words, the compare fails, `b`'s own check is skipped (`b_mask` is `None`), and `advance_set` steps `b`. Total about L·r/64 word reads on about 10r + 5L bits: with r = L = n/15, Θ(n²/14400). With the guard the same family does Θ(n) work.

### skyline-sweep-place-masked-6: `unreachable!` messages that are slot counts, not proofs
- Where: crates/before/src/version/skyline/masked.rs:384-384 (related: crates/before/src/version/skyline/masked.rs:430; crates/before/src/version/skyline/place.rs:544, 562)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for the four sites); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (e30d659d introduced the slot constants and both messages together)
- Owner-gated: no

The crate standard is that every `expect`/`unreachable!` message is a one-line proof (overlay.rs:448, 595, 605 are the model). "four cursor slots" and "three cursor slots" restate a hand-maintained count; the proof of unreachability is that slot indices come only from `priority()`, which names exactly the constants matched above.

Evidence:

       384	            _ => unreachable!("four cursor slots"),

    place.rs
       544	            _ => unreachable!("three cursor slots"),

Resolution: at all four sites, "slot indices come only from `priority`, which names the constants above". Acceptance: no `unreachable!` message in the partition is a bare count.

### skyline-sweep-place-masked-7: The arity-N advance law is stated twice: `advance_set` and `shape::advance_refinement`
- Where: crates/before/src/version/skyline/overlay.rs:12-17 (related: crates/before/src/version/skyline/shape.rs:102-116, 176-199; crates/before/src/version/skyline/overlay.rs:268-286; crates/before/src/version/skyline/admit.rs:340-395)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (grep of the tie-assert message `tied boundaries close to one shared flip level` hits overlay.rs, shape.rs, and admit.rs; the two bodies read side by side); executed: no
- Seen by: structure; refutation: confirmed (with the implementation note that `advance_set(set: &mut impl CursorSet)` has an implicit `Sized` bound, so a slice impl needs `?Sized` or a thin wrapper); history: deliberate but expired (true at c6ba2208/e30d659d; shape.rs's restatement landed twelve days later in 46eb64f9 with no reason for not reusing `CursorSet`)
- Owner-gated: no

The module doc says the law lives in exactly two generic faces, but `shape::advance_refinement` (shape.rs:176-199) is a full second statement of the arity-N body: the same strict first-maximum pick with `is_none_or(|(_, max)| depth > max)`, the same tied-step loop, the same `debug_assert_eq!`. Its trait `Refine` (`depth`, `done`, `advance -> flip`) is `CursorSet`'s per-slot vocabulary under another name, and shape.rs:171-175 says its `done()` guards are never load-bearing for the pick or the tie, so nothing in the body needs to differ. (`admit.rs`'s binary copy carries its own inline reason: fallible steps.) A duplicated advance discipline is the brief's named cost, and the "exactly two" claim is inaccurate as written.

Evidence:

        12	//! boundary carrying the cursor's own crossing payload. The overlay-advance law
        13	//! is stated (and debug-asserted) in exactly two generic faces — the binary
        14	//! [`advance`], which hands each crossing to the caller's fold, and the N-ary
        15	//! [`advance_set`] over a walk's whole [`CursorSet`], which folds crossings
        16	//! inside each slot's step; the boundary bookkeeping below is their shared
        17	//! correctness argument. Above them sit the two cursor instances.

    shape.rs
       191	    let flip = walks[deepest].advance();
       192	    for (slot, walk) in walks.iter_mut().enumerate() {
       193	        if slot != deepest && !walk.done() && walk.depth() >= flip {
       194	            let tied = walk.advance();
       195	            debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
       196	        }
       197	    }

Resolution: implement `CursorSet` for a slice of `Refine` walks (priority `0..len`, `depth(slot) = self[slot].depth()`, `step(slot) = self[slot].advance()`; give `advance_set` a `?Sized` bound or wrap the slice) and reduce `advance_refinement` to the all-done check followed by `advance_set`; or dissolve `Refine` into `CursorSet` outright. Then either make "exactly two" true by construction or drop the count and name the faces, and say that `admit.rs` restates the binary law for fallibility. Acceptance: `grep -rn 'tied boundaries close to one shared flip level' crates/before/src` hits overlay.rs and admit.rs only; the `shape` snapshots and `combine` tests are unchanged.

### skyline-sweep-place-masked-8: Colon-fronted fragments open body paragraphs
- Where: crates/before/src/version/skyline/overlay.rs:71-71 (related: crates/before/src/version/skyline/overlay.rs:99, 360, 651; crates/before/src/version/skyline/sweep.rs:109, 159; crates/before/src/version/skyline/signed.rs:49)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: reframed (none of the sites is a doc comment's first sentence, so no module listing is affected; the style tell stands); history: no rationale found
- Owner-gated: no

Seven body paragraphs open with a label and a colon where a sentence in default word order would read better: "Derived:" (the Cost section), "What every overlay walk rests on:", "The ownership-gated walks' block consume:", "The one shared algebra over [`advance`]:", "The sweep form of equality: stops at", "The single-direction fold behind causal containment checks: stops at", "Two-valued on purpose:". The brief names colon-fronted fragments as a default-dialect tell.

Evidence:

        71	//! Derived: a cursor only moves forward, so every topology bit of a stream is

        99	/// What every overlay walk rests on: a *plateau* is one maximal constant run of

    signed.rs
        49	/// Two-valued on purpose: zero travels as a zero magnitude under

Resolution: rewrite as sentences ("The bounds are derived rather than measured: ..."; "A *plateau* is one maximal constant run ..."; "The sign is two-valued because zero travels as a zero magnitude under `Positive` ..."; "Equality's sweep form stops at the first elementary interval whose height difference is nonzero."). Acceptance: the listed sites open with complete sentences.

### skyline-sweep-place-masked-9: One Panics paragraph copied seven times in overlay.rs, each copy saying it is stated once elsewhere
- Where: crates/before/src/version/skyline/overlay.rs:331-336 (related: crates/before/src/version/skyline/overlay.rs:371-376, 390-395, 433-441, 494-501, 524-530, 575-582)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n 'stated once there' overlay.rs` returns lines 336, 376, 395, 438, 501, 530, 581); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate and holds for the decision (a736ef14 and 0a534fe0 gave each method the precise contract with a pointer); the verbatim four-line form versus a one-line citation was not itself ruled, so a compression keeps the decision
- Owner-gated: no

Seven `pub(super)`/private methods carry the same four-line Panics paragraph (modulo id-vs-skyline wording), each ending "stated once there". For maintainer docs every sentence competes with the per-method content (flip-level semantics, the block-consume rationale) the reader came for, and the crate's own convention is one statement cited by link.

Evidence:

       331	    /// # Panics
       332	    ///
       333	    /// The stream must be canonical. The violations this walk structurally
       334	    /// notices — truncation, malformation — panic; the rest walk silently with
       335	    /// an unspecified result (the contract of
       336	    /// [`causal_cmp`](super::sweep::causal_cmp), stated once there).

Resolution: state the contract once on each cursor struct's doc (`LeafCursor` at 302, `IdLeafCursor` at 460) and reduce every method's section to one line citing it, keeping any method-specific clause (the "Never called on a final leaf" sentences). Acceptance: `grep -c 'stated once there' overlay.rs` drops to at most two; each method's Panics section is a one-line citation plus its own clause.

### skyline-sweep-place-masked-10: `LeafCursor` pushes and pops path bits one at a time where `BitStack` has a batched form
- Where: crates/before/src/version/skyline/overlay.rs:400-403 (related: crates/before/src/version/skyline/overlay.rs:442-457; crates/before/src/codec/stack.rs:47-102)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read `BitStack::push`, the private `push_bits`/`pop_bits`, and both loops); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (d07fc20e added `push_bits`/`pop_bits` for `PopStack` and left them private; the per-bit loops were not revisited)
- Owner-gated: no

`descend` pushes `internal_nodes` zeros in a loop and `step` pops the trailing right-run one bit at a time; `BitStack::push_bits` and `pop_bits` batch up to 63 bits per call but are private. On the dense spine the opening descent is one push per level. Constant factor only, invisible to every deterministic meter, fixed sign (strict deletion of per-bit loop overhead), so measure-first per doctrine.

Evidence:

       400	        let internal_nodes = self.cursor.read_unary().expect("canonical skyline bits");
       401	        for _ in 0..internal_nodes {
       402	            self.path.push(false);
       403	        }

Resolution: expose a `BitStack::push_zeros(n)` (loops of `push_bits(0, 63)` plus the remainder) and use `trailing_ones` + `pop_bits` in `step`; measure on the bench judge's dense comparison cell or a fuelscape reading before landing. Acceptance: the reading improves or holds; all envelope rows unchanged (scan bits, heap, and touches are identical by construction).

### skyline-sweep-place-masked-11: `IdLeafCursor::open`'s empty-stream arm is reachable through none of its callers and has no test
- Where: crates/before/src/version/skyline/overlay.rs:488-514 (related: crates/before/src/version/skyline/masked.rs:209-211; crates/before/src/version/skyline/shape.rs:83-87; crates/before/src/version/skyline/query.rs:499-502; crates/before/src/party.rs:791-799; crates/before/src/party/forks.rs:105-119; crates/before/src/idbits.rs:10)
- Class / severity / confidence: verification-gap / nit / medium
- Provenance: verified (grep: `IdLeafCursor::open` is constructed at masked.rs:209/211, shape.rs:85, query.rs:502; `anonymous()` appears only at party.rs:652 (definition, `pub(crate)`) and forks.rs:113 (transient); `finish_id` rejects the empty id); executed: no
- Seen by: correctness; refutation: reframed (the caller census was one site short; the conclusion holds); history: deliberate and holds (the arm mirrors the packed coding, where the empty stream is the empty id; the rationale is stated at the site)
- Owner-gated: no

Every construction site feeds a `Party`'s bits, and a `Party` is never empty: `finish_id` rejects the anonymous id and the one `Party::anonymous()` is a transient inside `Forks::new`. The arm is the cursor's contract generality for the canonical empty packed id (idbits.rs:10), unreachable through every current caller and defended by no test, so a regression in it would be invisible. The generality is deliberate and documented at the site; what is missing is the one test that makes it checkable.

Evidence:

       488	    /// Open a packed id stream at its first constant region.
       489	    ///
       490	    /// The empty stream is the empty id — one unowned region over the whole
       491	    /// interval — mirroring the packed coding, where absence *is* the empty
       492	    /// region.
       ...
       510	        if !bits.is_empty() {
       511	            this.descend();
       512	        }

Resolution: add a one-line unit test (an `overlay/tests.rs`) opening `IdLeafCursor` on the empty view and asserting `depth() == 0`, `done()`, and `!owned()`; or, if the owner prefers, drop the guard and state that masks are `Party` streams, which are never empty. Acceptance: the arm has one committed test, or is gone with the doc matching.

Construction: delete the `if !bits.is_empty()` guard so `descend` runs on empty bits; `read_bit().expect("canonical id bits")` panics on the first read, and no committed test fails, because no test opens the cursor on an empty view.

### skyline-sweep-place-masked-12: Two structs named `IdLeafCursor` share a step body: overlay.rs and party/ops/diff.rs
- Where: crates/before/src/version/skyline/overlay.rs:583-614 (related: crates/before/src/party/ops/diff.rs:245-268, 381-432; crates/before/src/version/skyline/overlay.rs:460-485, 531-552)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'struct IdLeafCursor' crates/before/src` returns overlay.rs:471 and diff.rs:245; both `impl PlateauCursor` read); executed: no
- Seen by: structure; refutation: confirmed; history: already known (the fold-unification survey scheduled the merge as optional Phase C with a drop condition; it never ran; the rename has no record)
- Owner-gated: no

`diff.rs:245` declares a private `struct IdLeafCursor<'a>` implementing `PlateauCursor` with `Crossing = bool`; its `step` (diff.rs:414-432) is the same pop-flip loop as overlay's (same match arms, near-identical `unreachable!` text, the same `lefts`/`open_lefts -= 1`, push-true, read the right-present flag), diverging only in the settle semantics and the stack type (`BitsBuf` vs `BitStack`). `grep IdLeafCursor` yields two definitions with different fields, and a fix to the id-side flip bookkeeping lands in one.

Evidence:

       583	    fn step(&mut self) -> (u64, ()) {
       584	        loop {
       585	            match self.path.pop() {
       586	                Some(true) => {
       587	                    self.right_present.pop();
       588	                    continue;
       589	                }
       ...
       593	                Some(false) => break,
       594	                None => unreachable!(
       595	                    "the advanced cursor is never at its final region: an all-right path means the stream is consumed"
       596	                ),
       597	            }
       598	        }
       599	        self.lefts -= 1;
       600	        self.path.push(true);
       601	        let flip = self.path.len();

    diff.rs
       414	    fn step(&mut self) -> (u64, bool) {
       415	        loop {
       416	            match self.path.pop() {
       417	                Some(true) => continue, // this ancestor closed with the item
       418	                Some(false) => break,   // the flip level: its right slot is next
       419	                None => unreachable!(
       420	                    "the advanced cursor is never at its final item: an all-right path means the tiling is consumed"
       421	                ),
       422	            }
       423	        }
       424	        self.open_lefts -= 1;
       425	        self.path.push(true);
       426	        let flip = self.depth();

Resolution: at minimum rename diff.rs's cursor (for example `SpliceCursor`) so `IdLeafCursor` names one thing, and have overlay.rs's doc name it as the splicing sibling; the composition (diff's cursor over overlay's, adding the `Item`/unsettled layer) is a cross-partition proposal for the party sweep. Acceptance: `grep -rn 'struct IdLeafCursor' crates/before/src` returns one definition; party diff tests and the `party::ops` meter rows unchanged.

### skyline-sweep-place-masked-13: `OpenedPair::open` and `BoundSide::open` overstate their Panics relative to the walk they wrap
- Where: crates/before/src/version/skyline/overlay.rs:697-699 (related: crates/before/src/version/skyline/place.rs:147-149; crates/before/src/version/skyline/overlay.rs:331-336; crates/before/src/version/skyline/masked/tests.rs:25-72)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the short form was the original contract wording; a736ef14 and 0a534fe0 replaced it with the precise noticed/silent split on the cursor methods and missed these two constructors)
- Owner-gated: no

Both constructors say they panic if a stream is not a canonical skyline encoding. They call `LeafCursor::open`, whose Panics section says only structurally noticed violations panic and the rest walk silently, and `collapsible_sibling_pair_sweeps_without_panicking` drives a non-canonical stream through `OpenedPair::open` (via `Walk::open`) without a panic. The module now carries two incompatible spellings of one contract.

Evidence:

       697	    /// # Panics
       698	    ///
       699	    /// Panics if either stream is not a canonical skyline encoding.

    place.rs
       147	    /// # Panics
       148	    ///
       149	    /// Panics if the stream is not a canonical skyline encoding.

Resolution: replace both with a citation of the one statement: "[`LeafCursor::open`]'s canonical-stream contract, on each stream." Acceptance: neither constructor claims an unconditional panic on non-canonical input.

### skyline-sweep-place-masked-14: A constant written inside big-O in the placement cost comparison
- Where: crates/before/src/version/skyline/place.rs:91-92 (related: crates/before/tests/meter.rs:9762-9766)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found
- Owner-gated: no

`O(2|v| + |s| + |e|)` equals `O(|v| + |s| + |e|)`, so the comparison is vacuous as written; the intended statement is a scan-bit count, which the meter row `span_place_scans_each_stream_once` states correctly as `fused + cmp_vv / 2 == cmp_vs + cmp_ve`.

Evidence:

        91	//! linear in the streams' bits — `O(|v| + |s| + |e|)` against the
        92	//! two-walk composition's `O(2|v| + |s| + |e|)` — and the per-interval sign

Resolution: write the comparison in bits scanned ("|v| + |s| + |e| bits against the composition's 2|v| + |s| + |e|") and keep one O() for the order. Acceptance: no O() expression in the partition carries a numeric constant.

Construction: not applicable; a notation defect settled by reading.

### skyline-sweep-place-masked-15: Pair-tracking state and its seeding are spelled at four production sites while `OpenedPair` claims one home
- Where: crates/before/src/version/skyline/place.rs:131-170 (related: crates/before/src/version/skyline/place/filter.rs:91-117; crates/before/src/version/skyline/overlay.rs:670-713; crates/before/src/version/skyline/admit.rs:292-298)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (all four seeding sites read: overlay.rs:703-705, place.rs:152-154, filter.rs:99-101, admit.rs:296-298; `read`/`relation` bodies in place.rs:163-170 and filter.rs:109-116 are byte-identical); executed: no
- Seen by: structure, correctness; refutation: confirmed; history: deliberate but expired (040fdfff's "one home" was true at 17:26 on 2026-07-29 and false by 19:32 when 0cbb9dc7 landed place.rs; the sibling `Directions` got its one home the next day for the same pattern)
- Owner-gated: no

`BoundSide` in place.rs carries `{diff, directions}` with `read` and `relation`; filter.rs's `Pair` is the identical pair of fields with identical bodies. The seed move (`Accumulator::new()`, fold `+a_first`, fold `-b_first`) is written out at overlay.rs:703-705, place.rs:152-154, filter.rs:99-101, and admit.rs:296-298, though overlay.rs:674-676 says the orientation "has one home". The seeding orientation is exactly the invariant the touch-meter identities rest on, so it should have one spelling.

Evidence:

       150	    fn open(bits: BitsView<'a>, probe_first: &Int) -> BoundSide<'a> {
       151	        let (cursor, first) = LeafCursor::open(bits);
       152	        let mut diff = Accumulator::new();
       153	        super::signed::fold_signed_int(&mut diff, Sign::Positive, probe_first);
       154	        super::signed::fold_signed_int(&mut diff, Sign::Negative, &first);
       ...
       162	    /// Fold this interval's sign into the surviving directions.
       163	    fn read(&mut self) {
       164	        self.directions.fold(self.diff.sign());
       165	    }
       166	
       167	    /// The relation the completed sweep decided, as the causal order.
       168	    fn relation(&self) -> Option<Ordering> {
       169	        self.directions.relation()
       170	    }

    overlay.rs
       674	/// The shared opening move of every two-skyline walk, stated once so the
       675	/// seeding's orientation — `a` positive, `b` negative, the orientation [`fold`]
       676	/// applies to every later crossing — has one home. The opening heights ride

Resolution: hoist filter's `Pair` to sit beside `Directions` in sweep.rs (or beside `OpenedPair` in overlay.rs) with `Pair::open(a_first, b_first)`, `read`, `relation`; `OpenedPair::open` seeds through the same constructor (or a shared `seed_diff`), place's `BoundSide` becomes `{cursor, pair}`, and admit's seed calls the shared function. Optionally `sweep::sweep` and `masked::Walk::run` hold a `Pair` instead of parallel `diff`/`directions` locals. Acceptance: one production site folds `Sign::Negative` into a fresh difference; the placement identity rows in tests/meter.rs are unchanged (the write sequence is identical).

### skyline-sweep-place-masked-16: Em-dashes in `//` comments at 25 sites
- Where: crates/before/src/version/skyline/place.rs:258-260 (related: crates/before/src/version/skyline/sweep.rs:68, 69, 97; crates/before/src/version/skyline/overlay.rs:591; crates/before/src/version/skyline/place.rs:213, 219, 269, 279, 322, 331, 342, 386, 403, 459, 462; crates/before/src/version/skyline/place/filter.rs:191, 369, 370, 378, 407, 408, 480; crates/before/src/version/skyline/place/tests.rs:245; crates/before/src/version/skyline/masked/tests.rs:27)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the ten partition files returns exactly the 25 sites listed; no assert, expect, or unreachable string literal contains one); executed: no
- Seen by: prose; refutation: confirmed; history: the rule lives in the owner's global doctrine (spaced double-hyphens in code comments), not in either AGENTS.md, and no gate tool checks it
- Owner-gated: no

The owner's doctrine reserves true em-dashes for rendered prose and asks for colons, semicolons, or spaced double-hyphens in code comments (terminal compatibility and sentence flow). Twenty-five non-doc `//` comments in the partition carry one.

Evidence:

       258	        // `lo <= probe` refuted is the whole verdict — the probe dominates not
       259	        // even the start, whatever the end relation: the family's earliest
       260	        // bail.

Resolution: rewrite the 25 sites with colons, semicolons, parentheses, or spaced double-hyphens; the grep above enumerates them. Acceptance: the grep returns nothing over the partition files.

### skyline-sweep-place-masked-17: `walk`'s `finish` takes `Option<Option<Ordering>>` that every arm flattens; `Some(None)` is unreachable
- Where: crates/before/src/version/skyline/place.rs:437-450 (related: crates/before/src/version/skyline/place.rs:224, 291, 295, 354, 358, 411-412, 489-492)
- Class / severity / confidence: idiom / low / high
- Provenance: assessed (read every hook and finish arm); executed: no
- Seen by: structure; refutation: confirmed (hook analysis: `span`'s `on_side` drops or breaks on a double refutation; dominance, precedence, and contains break or drop the moment their single watched direction is refuted, so a surviving side always has `le || ge`); history: deliberate but expired (e2f4e2a5 kept the arm total for non-canonical input; aac1bc04 and b0194c6a established that the hooks rule the corner out by control flow on any input; the obligation paragraph was added by 248d5539 to justify the flattening)
- Owner-gated: no

`finish` receives `Option<Option<Ordering>>` per side (`None` dropped, `Some(None)` swept to concurrent). Every finish arm calls `.flatten()` on both sides before reading, and the doc itself argues `Some(None)` is deliverable only by hooks that leave a refutation standing, which no entry point's hooks do. The nested type then needs an "Obligation on every caller" paragraph to make the flattening sound: a type wider than its reachable states, with seven flatten calls and a paragraph compensating.

Evidence:

       437	/// Obligation on every caller: a `finish` arm that reads a side through
       438	/// `flatten` merges "dropped" with "swept to concurrent", so the side's drop
       439	/// condition must agree with that arm's reading — the hook may drop a side only
       440	/// when the direction the finish arm tests is already refuted, making the
       441	/// flattened `None` and the decided relation give the same answer. Each entry
       442	/// point carries the per-verdict argument at its closures.
       ...
       449	    finish: impl FnOnce(Option<Option<Ordering>>, Option<Option<Ordering>>) -> V,

Resolution: change the signature to `finish: impl FnOnce(Option<Ordering>, Option<Ordering>) -> V`, pass `set.start.as_ref().and_then(BoundSide::relation)` (and the end twin), delete the `.flatten()` calls (the existing `debug_assert!`s work unchanged on the flattened values), and replace the obligation paragraph with one sentence: a dropped side reads `None`, the same as a swept concurrency, and hooks drop only once the direction their finish arm tests is refuted. Acceptance: `grep -c '\.flatten()' place.rs` drops to 1 (the iterator flatten at 555); `span_walks_match_the_composed_sweeps` and the witness tests pass.

### skyline-sweep-place-masked-18: Verdict hooks take a `bool` that three of four callers ignore
- Where: crates/before/src/version/skyline/place.rs:447-448 (related: crates/before/src/version/skyline/place.rs:198-206, 261, 271, 324, 334, 387, 395)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (the `other_live: bool` dates to e2f4e2a5's `Mode` trait; 2339d8dc kept the shape when converting to closures)
- Owner-gated: no

`on_start`/`on_end` receive `(Directions, bool)`, the bool meaning "the other side still sweeps"; only `span`'s `on_side` reads it, while `dominance`, `precedence`, and `contains` bind it `_`. An unnamed boolean in a closure signature is read at the call site only by convention.

Evidence:

       447	    on_start: impl Fn(Directions, bool) -> ControlFlow<V, Fate>,
       448	    on_end: impl Fn(Directions, bool) -> ControlFlow<V, Fate>,

Resolution: name it (a two-variant enum such as `OtherSide::Live | OtherSide::Dropped`, or a small `HookView { directions, other_live }`), or, since only `span` needs it, let the other three hooks be `Fn(Directions) -> ControlFlow<V, Fate>` through a thin adapter. Acceptance: no bare `bool` in `walk`'s hook signature; witness and proptest suites pass.

### skyline-sweep-place-masked-19: The placement write-sequence identity is claimed pinned by touch readings, but the placement rows read no touch meter
- Where: crates/before/src/version/skyline/place.rs:527-531 (related: crates/before/src/version/skyline/place/filter.rs:249-256; crates/before/src/version/skyline/overlay.rs:233-236; crates/before/tests/meter.rs:9472-10084; crates/before/src/meter.rs:3561-3624; crates/before/src/codec/base.rs:274-282)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of tests/meter.rs lines 9463-10090 for `touch_meter|touches|limb_ops|scan_bits`: only `reset_scan_bits`/`scan_bits` at 9480-9482 and `reset_limb_ops`/`limb_ops` at 9707-9709; `Base`'s `Magnitude` impl at base.rs:274-282 records nothing; `limb_ops` counts `Base` operations and wide-gamma decodes, `touch_ops` is the separate suanpan counter); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (the placement rows have pinned scan and limb identities only since 0cbb9dc7/e2f4e2a5; the generic "committed touch-meter readings pin each walk's sequence" sentence generalizes from the masked walk, whose rows do pin touches)
- Owner-gated: no

place.rs, filter.rs, and overlay.rs state that the accumulator write-sequence identity between each fused walk and its pair sweep is pinned by the placement rows in tests/meter.rs. Those rows read scan bits (order-independent: both orders read the same bits) and limb ops (blind to accumulator writes: `Base`'s `Magnitude` impl records nothing, and word-scale folds ride the quick register). A priority reorder that changes carry work would pass every placement row, so the instrument the docs name pins something weaker than the docs claim (Principle 8, and the adequacy rule).

Evidence:

       527	/// Priority `[PROBE, START, END]`: the probe steps first on every tie — it is
       528	/// every pair's first operand, and the binary law's equal-depth arm steps its
       529	/// first operand first — keeping each accumulator's write sequence identical
       530	/// to its pair sweep's (the placement identity rows in `tests/meter.rs` pin
       531	/// each fused walk against its composed sweeps). The start/end order among

    overlay.rs
       233	    /// which no current client does. The order is contract, not convenience: a
       234	    /// walk whose slots share an accumulator commits its digit writes in step
       235	    /// order, so the committed touch-meter readings pin each walk's sequence
       236	    /// (each impl documents which identities pin its own). The iterator is

Resolution: add `touch_ops` identity legs beside the scan and limb legs in the placement module (for the single exhaustion-confirmed bound, `touches(contains) == touches(partial_cmp)`; for `span_place_scans_each_stream_once`, a relational touch identity against the composed sweeps), on a fixture whose deltas cross a carry boundary (the CliffComb family) so step order moves the reading; then point the three docs at the touch legs by name. Acceptance: with the new legs, reordering `Cursors::priority` to `[START, END, PROBE]` (verdicts, scan, and limb rows unchanged) turns the touch legs red on the carry-boundary fixture; restoring the order turns them green.

Construction: swap `PROBE` to last in place.rs:535. Every verdict suite passes (folds are commutative sums); `span_place_scans_each_stream_once` and `query_single_bound_matches_the_pair_sweep_limbs` pass (scan bits and `Base` ops are order-independent). Only a touch identity on a fixture where the a-then-b and b-then-a write orders commit different carry work separates; on a quick-register-resident fixture both orders touch identically, so the fixture must be a carry-boundary comb.

### skyline-sweep-place-masked-20: The coverage walk's three documented early exits have no resource pin and no deterministic witness
- Where: crates/before/src/version/skyline/place/filter.rs:26-35 (related: crates/before/src/version/skyline/place/filter.rs:427-440; crates/before/src/version/skyline/place/tests.rs:330-379; crates/before/tests/meter.rs:9472-10084)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n '\.coverage(' tests/meter.rs` returns nothing; `grep -in coverage tests/meter.rs` returns only two unrelated doc lines, 4029 and 6060; the placement module measures contains, place, dominance, and precedence); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (db9dfa3e landed four membership rows and none for coverage; no coverage row has ever existed)
- Owner-gated: no

The module doc promises three coverage exits: a hole refuted at both endpoints drops its stream, a probe endpoint whose every pair is settled stops being scanned, and a walk holding only settled holes returns `Full` without exhausting anything. The membership, span, and dominance walks each carry a scan-bit row stated relationally against the pair sweep; coverage has none. `filter_coverage_organic_witnesses`' `Full` case carries `After` and `Before` demands, which never drop (`After` settles only `lo`, `Before` only `hi`), so the `live == 0` return and both endpoint drops are unreached there; only the proptest can reach them, by chance. A `coverage` with the three drop paths deleted passes every committed test, because a settled pair's stale directions give `finish` the same answer.

Evidence:

        26	//! - [`coverage`]: a floor refuting `floor <= hi` (or a ceiling
        27	//!   refuting `lo <= ceiling`) proves no covered version is admitted —
        28	//!   [`Coverage::Empty`] at the refuting interval, the verdict a
        29	//!   pruning tree walk consumes. A hole whose subtraction is refuted at
        30	//!   both endpoints is settled and drops its stream, a probe endpoint
        31	//!   whose every pair is settled drops its own cursor, and a walk left
        32	//!   holding only settled holes returns [`Coverage::Full`] without
        33	//!   exhausting anything. `Partial` alone always confirms at

       427	            if !side.lo.live && !side.hi.live {
       428	                *slot = None;
       429	                live -= 1;
       430	            }
       ...
       439	        walk.lo_live = walk.lo_live && walk.sides.iter().flatten().any(|side| side.lo.live);
       440	        walk.hi_live = walk.hi_live && walk.sides.iter().flatten().any(|side| side.hi.live);

Resolution: add two scan-bit rows to the placement meter module, stated relationally like the others: (1) a hole-only query concurrent to both endpoints must read strictly under the composed four `partial_cmp`s and stop at the second deciding interval (the early `Full`); (2) a required floor plus a hole whose `lo` pair settles must show the `lo` stream's scan stopping (compare against the same query with the hole removed). Add the two deterministic verdict witnesses (all-holes-settled `Full`; endpoint drop then `Partial`) to `filter_coverage_organic_witnesses`. Acceptance: the new rows read green at HEAD and red when lines 427-430 and 439-440 are removed, while every verdict test stays green.

Construction: delete lines 427-430 and 439-440. Every verdict suite (place/tests.rs proptests, `coverage_is_exact_on_the_two_party_grid`, the laws, the verdict matrix) passes because the exits are value-preserving; no meter row fails because none measures `coverage`.

### skyline-sweep-place-masked-21: The filter walks' stated `O(|v| + Σ|bound|)` omits the per-interval factor k
- Where: crates/before/src/version/skyline/place/filter.rs:37-48 (related: crates/before/src/version/skyline/place/filter.rs:183-207, 359-367, 580-613; crates/before/src/version/skyline/overlay.rs:268-286; crates/before/src/causally/query.rs:91-96, 114-117; crates/before/src/causally/conjunction.rs:47-61; crates/before/src/causally/polarity.rs:98-107, 156-163; crates/before/tests/meter.rs:9472-10084)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed for the derivation (read loops and `advance_set`; suanpan's `sign()` touches once on the quick register); verified for the instrument census (grep: the placement rows use at most two bounds, tests/meter.rs has no coverage call, and `Query::holes` is an uncapped `Vec` kept by `Query::and` for every pairwise-unabsorbed hole); executed: no
- Seen by: correctness, claims; refutation: confirmed; history: no rationale found (a736ef14 #17 added the sentence acknowledging O(#bounds) per-interval bookkeeping and left the total unchanged; no record treats the bound count as an input axis)
- Owner-gated: no

The read loop performs one `sign()` read per live pair per elementary interval of the (k+1)-stream overlay, the probe's step folds into every live pair, and `advance_set` makes two O(k) passes per boundary. The overlay has Θ(|v| + Σ|b_i|) elementary intervals, so the total is Θ(k · (|v| + Σ|b_i|)), not O(|v| + Σ|b_i|); the composed pair sweeps read Σ_i (|v| + |b_i|) = k|v| + Σ|b_i|, so on many large holes against a small probe the fusion is asymptotically worse on the bounds' term than the composition it is stated "against". The bound count is caller-controlled: `Query::and` keeps every pairwise-unabsorbed hole and pairwise-concurrent hole versions never absorb. The crate docs make every asymptotic claim a hard guarantee for all input sizes; the public `Query::contains` doc (query.rs:95-96, "One traversal of `version` and the stored bounds") inherits the denomination.

Evidence:

        43	//! and the per-interval sign reads ride the accumulator's amortized-O(1)
        44	//! collapse. The coverage walk additionally recomputes its endpoint-liveness
        45	//! flags and sweeps the settled flags once per interval — O(#bounds)
        46	//! bookkeeping absorbed by the same per-interval read loop. `O(|v| + Σ|bound|)`
        47	//! for membership, `O(|lo| + |hi| + Σ|bound|)` for coverage, against the
        48	//! composed sweeps' one probe decode per bound.

       184	        // One read per live bound per elementary interval, in demand order.
       185	        for slot in &mut walk.sides {
       186	            let Some(side) = slot else { continue };
       187	            side.pair.read();

Resolution: either (a) restate the bound honestly at filter.rs:37-48 and the public docs it feeds (scan bits O(|v| + Σ|b_i|); folds O(k|v| + Σ|b_i|); sign reads and advance bookkeeping O(k · (|v| + Σ|b_i|))), with the honest comparison (the fusion saves k-1 probe decodes and pays a k factor on the bounds' boundaries), and pin the exponent in k with a two-scale touch row; or (b) restructure: read only pairs whose cursor stepped this round (a bound step changes one pair; a probe step changes all k, which the composition also pays; `Directions::fold` is idempotent on an unchanged sign), and pick the deepest slot with a heap keyed by depth, reaching Θ(k|v| + Σ|b_i| · log k); the single-bound identity rows are unmoved because every interval of a two-stream overlay is dirty. Acceptance: a committed tests/meter.rs row builds k and 2k `Demand::NotBefore` holes with pairwise-disjoint leaf-boundary sets against the probe `Version::new()` (below every hole, so nothing exits early), measures `touch_ops` over `filter::admits`, and asserts the ratio sits under the stated exponent, with a liveness floor of k touches (one read per bound on the first interval).

Construction: k holes, each an oracle tree with a B-leaf shape under the i-th depth-ceil(log2 k) dyadic prefix and zero leaves elsewhere (Σ|h_i| about kB; the joint overlay has about kB elementary intervals); probe `Version::new()`; demands all `NotBefore`. Run `filter::admits` at k and 2k with B fixed and read `touch_ops`. The fused walk reads k live pairs per interval, about k · kB touches, quadrupling on the doubling; the composed baseline Σ_i `sweep::causal_cmp(v, h_i)` reads about kB, doubling. A fused ratio near 4 refutes `O(|v| + Σ|bound|)`.

### skyline-sweep-place-masked-22: `fold_signed_int` is called by qualified path beside imports from the same module
- Where: crates/before/src/version/skyline/place/filter.rs:98-101 (related: crates/before/src/version/skyline/overlay.rs:94, 380, 455, 645, 704-705; crates/before/src/version/skyline/place.rs:120, 153-154; crates/before/src/version/skyline/masked.rs:87, 199, 204, 402, 419; crates/before/src/version/skyline/walk.rs:32)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `super::signed::` and `super::super::signed::` across the four production files); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (a migration artifact of edd03f15's re-pointing; 4ac8fd70 later added `use ...signed::Sign` beside the still-qualified calls)
- Owner-gated: no

`Sign` is imported from `signed` in all four files, but `fold_signed_int` (and `unzigzag` at overlay.rs:455) is spelled `super::signed::...` or `super::super::signed::...` at every call; walk.rs in the same directory imports the names directly. The qualification disambiguates nothing (overlay's local function is `fold`, a different name), and `super::super::` is the crate's least legible spelling of the module.

Evidence:

        98	    fn open(probe_first: &Int, bound_first: &Int) -> Pair {
        99	        let mut diff = Accumulator::new();
       100	        super::super::signed::fold_signed_int(&mut diff, Sign::Positive, probe_first);
       101	        super::super::signed::fold_signed_int(&mut diff, Sign::Negative, bound_first);

Resolution: `use super::signed::{fold_signed_int, Sign};` (and `unzigzag` in overlay.rs); in filter.rs prefer `use crate::version::skyline::{overlay, signed, sweep}` over `super::super::`. Most sites dissolve if finding 15 lands. Acceptance: the grep shows only `use` lines.

### skyline-sweep-place-masked-23: `filter::BoundSide` sits one module below `place::BoundSide` with a different shape
- Where: crates/before/src/version/skyline/place/filter.rs:140-147 (related: crates/before/src/version/skyline/place.rs:133-141; crates/before/src/version/skyline/place/filter.rs:300-311)
- Class / severity / confidence: modularity / nit / high
- Provenance: assessed (read both definitions); executed: no
- Seen by: structure; refutation: confirmed (a child module, so no actual shadowing; two same-named private shapes one module apart); history: no rationale found
- Owner-gated: no

place.rs declares `struct BoundSide<'a> { cursor, diff, directions }` and its child filter.rs declares `struct BoundSide<'a> { cursor, pair, demand }`. A reader grepping `BoundSide` lands on two shapes. The membership/coverage banners mark two responsibilities sharing one 614-line file; the banner convention itself is house style.

Evidence:

       140	// ───────────────────────────── membership ─────────────────────────────
       141	
       142	/// One bound's side of the membership walk.
       143	struct BoundSide<'a> {
       144	    cursor: LeafCursor<'a>,
       145	    pair: Pair,
       146	    demand: Demand,
       147	}

Resolution: rename filter's to `DemandSide` (it carries a `Demand`); optionally split into `filter/admits.rs` and `filter/coverage.rs` with `Demand`, `Pair`, and `GatedPair` in `filter.rs`. Acceptance: `grep -rn 'struct BoundSide' crates/before/src` returns one definition.

### skyline-sweep-place-masked-24: Read-order prose names a write-sequence effect that independent accumulators cannot have
- Where: crates/before/src/version/skyline/place/filter.rs:152-154 (related: crates/before/src/version/skyline/place/filter.rs:193-194, 254-256, 318-319; crates/before/src/version/skyline/place.rs:459-463)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: reframed (the order is not vacuous: an early `return false` at 193-194, or a `Break` at place.rs:466-478, ends the read loop mid-pass, so which pairs get their one `sign()` touch on the exit interval depends on the order; the sentences name the wrong mechanism rather than protecting nothing); history: no rationale found
- Owner-gated: no

`sign()` touches only its own accumulator and no two pairs share one, so the order in which pairs are read per interval cannot change any accumulator's write sequence; filter.rs:254-256 says exactly this three paragraphs later. What a fixed order does fix is which pairs are read before an early return on the exit interval, which is what makes the touch readings deterministic. The three sites should state that reason or drop the justification.

Evidence:

       152	/// An empty demand list is vacuously `true` at zero cost. The demand list's
       153	/// order is the read order per elementary interval, which fixes the accumulator
       154	/// write sequence; callers supply a deterministic order.

       254	/// identity rows in `tests/meter.rs` pin the single-bound identity). The
       255	/// bounds' order among themselves moves no committed reading: no two bounds
       256	/// share an accumulator.

Resolution: at filter.rs:152-154, :318-319, and place.rs:459-463, replace "fixes the accumulator write sequence" / "so every question's accumulator traffic is identical" with the true reason: a fixed read order makes the exit-interval reads, and so the committed touch readings, deterministic. Acceptance: the sentences state the exit-interval reason or are removed; filter.rs:254-256 stands.

### skyline-sweep-place-masked-25: `coverage`'s `finish` doc states a discipline its hole emptiness arms do not keep
- Where: crates/before/src/version/skyline/place/filter.rs:463-476 (related: crates/before/src/version/skyline/place/filter.rs:410-425, 482-492)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read the walk arms and `finish`); executed: no
- Seen by: correctness; refutation: confirmed; history: already known (ed77a9f7 recorded the decision: "finish's emptiness arms legitimately read settled pairs' relations", choosing the comment over making `Pair::relation` refuse when not live; aa7c96a0's doc rewrite narrowed the paragraph to required pairs and dropped that clause, producing the mismatch)
- Owner-gated: no

The doc says only a pair alive at exhaustion answers by its decided relation and that the stale-direction agreement "is not part of the contract". For `NotBefore`/`NotStrictlyBefore` the walk settles `hi` alone when `hi <= bound` is refuted (411-413) while `lo` stays live, so the side survives to `finish`, whose emptiness arms read `side.hi.pair.relation()` unguarded (482, 487-488). The answer is right because refutation is permanent, which is precisely the argument the doc disclaims. The recorded decision is that the emptiness arms read settled pairs by design, so the fix is to restore that clause, not to add guards.

Evidence:

       465	/// Division of labor with the walk: a settled *required* pair's refutation
       466	/// already lives in `full_possible`, so the `!live` guards on the two
       467	/// required arms keep `finish` from consulting a settled pair's stale
       468	/// `directions`. The stale directions would happen to agree (a
       469	/// settle-direction refutation is permanent), but that agreement is not part
       470	/// of the contract: only a pair alive at exhaustion answers by its decided
       471	/// relation. A hole needs no guard on its fullness endpoint: a surviving

       482	        let (lo, hi) = (side.lo.pair.relation(), side.hi.pair.relation());
       ...
       487	            Demand::NotBefore => matches!(hi, Some(Ordering::Less | Ordering::Equal)),
       488	            Demand::NotStrictlyBefore => hi == Some(Ordering::Less),

Resolution: rewrite the paragraph to say that the hole emptiness arms read their emptiness endpoint whether or not it is settled, relying on the permanence of refutation (a settled emptiness endpoint has its emptying direction refuted, so `false` is the decided answer), and that the `!live` guards on the two required arms exist because those arms' refutations already live in `full_possible`. Acceptance: the doc and the six emptiness arms agree on the argument each uses; `filter_coverage_matches_the_composed_sweeps` and `filter_coverage_organic_witnesses` stay green.

### skyline-sweep-place-masked-26: place/tests.rs has no module doc, and two tests share an indistinguishable first sentence
- Where: crates/before/src/version/skyline/place/tests.rs:1-1 (related: crates/before/src/version/skyline/place/tests.rs:258-260, 294-298; crates/before/src/version/skyline/sweep/tests.rs:1-10; crates/before/src/version/skyline/masked/tests.rs:1-7; crates/before/src/version/skyline/signed/tests.rs:1-17)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (line 1 is the first `use`; `grep -n 'Every membership-walk hook path'` returns lines 258 and 294); executed: no
- Seen by: prose, correctness; refutation: reframed (the third sub-claim, that "conservative Partial" misdescribes an exact verdict, is refuted: `Query::coverage` passes filter's `Partial` through `refine_partial`, "the exact emptiness decision the fused endpoint fold cannot reach", so at the stream layer `Partial` is conservative by design); history: no rationale found
- Owner-gated: no

The three sibling test files open by naming their oracle; place/tests.rs opens with `use proptest::prelude::*;` and never states at module level that the composed pair sweeps (`composed_span`, `demand_admits`, `composed_coverage`) are its oracle. `contains_walk_verdicts_organic_witnesses` (258) and `filter_admits_organic_witnesses` (294) both open "Every membership-walk hook path on an organic witness set", so the first sentence does not identify the test in a listing.

Evidence:

         1	use proptest::prelude::*;

       258	/// Every membership-walk hook path on an organic witness set: the start-side

       294	/// Every membership-walk hook path on an organic witness set.

Resolution: add a `//!` module doc naming the composed pair-sweep spellings as the oracle and the organic witness sets as the deterministic hook-path coverage; reword line 294 to "Every filter-membership hook path on an organic witness set." Acceptance: place/tests.rs begins with a module doc; the duplicated first sentence occurs once.

### skyline-sweep-place-masked-27: Test idiom: redundant parentheses (41 sites) and an unimported qualified helper (17 sites)
- Where: crates/before/src/version/skyline/place/tests.rs:14-15 (related: crates/before/src/version/skyline/sweep/tests.rs:34-39, 44-81, 202-224, 260-279)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -o '([a-z_]*\.view())\.live()' place/tests.rs | wc -l` is 41; `grep -c 'crate::codec::built_view' sweep/tests.rs` is 17); executed: no
- Seen by: structure; refutation: confirmed (the reader's 24 undercounted); history: no rationale found (5d167a63's mechanical rewrite when `Bits`'s `Deref` was deleted; 83e61b4d's `BitsBuf` migration)
- Owner-gated: no

`(x.view()).live()` appears 41 times in place/tests.rs, with parentheses that look like they guard a precedence that does not exist; `crate::codec::built_view(...)` is spelled with its full path 17 times in sweep/tests.rs instead of being imported once.

Evidence:

        14	    let lo_rel = sweep::causal_cmp((probe.view()).live(), (lo.view()).live());
        15	    let hi_rel = sweep::causal_cmp((probe.view()).live(), (hi.view()).live());

Resolution: `probe.view().live()`; `use crate::codec::built_view;` in sweep/tests.rs. Acceptance: the first grep returns 0; the second returns 0 or 1 (the import).

### skyline-sweep-place-masked-28: `dominance` is the one placement entry point without an organic-witness test
- Where: crates/before/src/version/skyline/place/tests.rs:218-224 (related: crates/before/src/version/skyline/place/tests.rs:177, 262, 300, 334; crates/before/tests/meter.rs:9889)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (`grep -n 'fn .*organic_witnesses' place/tests.rs` lists span, precedence, contains, filter_admits, filter_coverage and no dominance); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (no dominance organic-witness test has ever existed at the stream layer)
- Owner-gated: no

`span`, `precedence`, `contains`, `filter::admits`, and `filter::coverage` each have a deterministic organic-witness test naming every hook path; `dominance`'s paths are reached deterministically only through the public `span/tests.rs` and the meter row `dominance_bails_at_the_refuted_start`. The module doc presents `precedence` as `dominance` mirrored; the deterministic coverage is asymmetric.

Evidence:

       218	/// Every precedence-walk hook path on an organic witness set.
       219	///
       220	/// The end refutation's early bail, the start refutation's drop (the verdict
       221	/// riding the end relation alone), and the exhaustion confirmations for both
       222	/// surviving directions.
       223	#[test]
       224	fn precedence_walk_verdicts_organic_witnesses() {

Resolution: add `dominance_walk_verdicts_organic_witnesses` mirroring the precedence test (Before bail on a concurrent start and on a dominating start; end drop then `Between` at exhaustion; `After` at exhaustion). Acceptance: four organic-witness tests, one per placement closure, each with a distinct first sentence; `just testdoc` clean.

### skyline-sweep-place-masked-29: Texture coinages used as jargon: currency, face, genre, "block consume", "real", point "tripwire"
- Where: crates/before/src/version/skyline/signed.rs:1-3 (related: crates/before/src/version/skyline/signed.rs:191; crates/before/src/version/skyline/overlay.rs:13, 263, 265, 351, 360; crates/before/src/version/skyline/place.rs:243, 304, 367; crates/before/src/version/skyline/masked.rs:286; crates/before/src/version/skyline/sweep.rs:61; crates/before/src/version/skyline/sweep/tests.rs:189; crates/before/src/version/skyline/place/tests.rs:259; crates/before/src/version/skyline/signed/tests.rs:88; crates/before/src/meter/board.rs:292)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (word greps over the partition; `Currency` is a live identifier at meter/board.rs:292 with a different meaning); executed: no
- Seen by: prose; refutation: confirmed with caveats ("bail" is conventional programming usage and is dropped from this list; the signed/tests.rs:88 "tripwire" names the known-bad mechanism it fails and is borderline); history: no rationale found (a736ef14's per-term counterweight dissolved "both width currencies" and minted "improvement tripwire" as a meter-genre term, but "sign-magnitude currency", face, genre, "block consume", and "real" were never submitted; the "point tripwire" postdates the mint and uses the minted word off-genre)
- Owner-gated: yes (vocabulary rulings are the owner's per-term counterweight)

"currency" (signed.rs:1, 191) is a monetary metaphor for "representation" that collides with the board's `Currency` identifier, which means a deterministic meter. "face" for variant or entry point (overlay.rs:13, 263, 265; place.rs:243, 304, 367), "genre" for kind (sweep.rs:61; sweep/tests.rs:189; place/tests.rs:259), "block consume" as a noun (overlay.rs:360, masked.rs:286) beside the identifiers `block_skip`/`skip_deeper`, "every real flip level" (overlay.rs:351) moralizing the non-sentinel value, and "point tripwire" (signed/tests.rs:88) for a deterministic point test, where the crate reserves the word for a meter-genre adequacy pin. Every coined term is anchored to an identifier or defined once by contrast; these are texture.

Evidence:

         1	//! The sign-magnitude currency: the zigzag maps, the signed folds and sums, and
         2	//! the gamma codes of signed deltas — one home for the vocabulary every skyline
         3	//! walk exchanges heights in.

    overlay.rs
       351	    /// remains — every real flip level is at least one.

    signed/tests.rs
        88	/// The point tripwire riding beside the generalized family below: a flipped

Resolution: currency to "representation" or "exchange form" (signed.rs already anchors "exchange pair/shape" to `Signed`); face to "form" or "entry point"; genre to "kind"; "block consume" to "block skip" matching the identifiers; "every real flip level" to "every flip level a step returns"; "point tripwire" to "deterministic point pin". Acceptance: no "currency" in signed.rs; "block consume", "real", and the point-test "tripwire" gone; face and genre replaced or each defined once at first use.

### skyline-sweep-place-masked-30: signed.rs says "the tests pin the bijection", but the bijection test lives in skyline/tests.rs
- Where: crates/before/src/version/skyline/signed.rs:12-14 (related: crates/before/src/version/skyline/tests.rs:33, 300-331; crates/before/src/version/skyline/signed/tests.rs:1-26)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep: `zigzag_is_a_bijection_without_negative_zero` is at skyline/tests.rs:307 importing `super::signed::{unzigzag, zigzag}` at :33; signed/tests.rs mentions zigzag only as the reference coder); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate but expired (bb5c2c64 placed the test beside zigzag's then home in the skyline root; edd03f15 moved the code into signed.rs and the sentence with it, leaving the test behind)
- Owner-gated: no

The zigzag section closes with "the tests pin the bijection exhaustively at small scope"; signed/tests.rs holds only the fused-coder differentials and the signed-comparison pins. AGENTS.md places unit tests in the sibling tests.rs of the code they exercise; a reader following that convention does not find the test.

Evidence:

        12	//! spelling cannot be written at all. The root module doc's canonical-form
        13	//! argument leans on exactly this; the tests pin the bijection exhaustively at
        14	//! small scope.

Resolution: move `zigzag_is_a_bijection_without_negative_zero` (skyline/tests.rs:300-331) into signed/tests.rs, or, if it stays because it justifies the reject corpus there, name its home in this sentence. Acceptance: signed/tests.rs contains the bijection test, or signed.rs:13-14 names the test by name and file.

### skyline-sweep-place-masked-31: `gamma_code_signed_int` duplicates `gamma_code_signed`'s fused fast-path body
- Where: crates/before/src/version/skyline/signed.rs:259-276 (related: crates/before/src/version/skyline/signed.rs:236-255; crates/before/src/version/skyline/signed/tests.rs:1-11)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (read both bodies); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (the `Int` twin's existence is deliberate, e4c9b083; a736ef14 named the bound in "both twins" without merging the bodies)
- Owner-gated: no

The `Int::Small(mag) if *mag < GAMMA_SMALL_MAG_BOUND` arm repeats lines 241-252 verbatim: the negative-zero `debug_assert!`, `let m = 2 * mag + u64::from(sign == Sign::Positive)`, the `k` from `leading_zeros`, and the `Code::Small` construction. The fused mantissa expression is the one the differential tests exist to guard; two copies let a future off-by-one be fixed in one and not the other.

Evidence:

       261	        Int::Small(mag) if *mag < GAMMA_SMALL_MAG_BOUND => {
       262	            debug_assert!(
       263	                !sign.is_negative() || *mag != 0,
       264	                "a negative delta has a nonzero magnitude"
       265	            );
       266	            let m = 2 * mag + u64::from(sign == Sign::Positive);
       267	            let k = (u64::BITS - 1 - m.leading_zeros()) as usize;
       268	            Code::Small {
       269	                bits: m,
       270	                len: (2 * k + 1) as u8,
       271	            }
       272	        }

Resolution: extract `fn small_signed_code(sign: Sign, mag: u64) -> Code` holding the assert and the mantissa arithmetic; `gamma_code_signed` calls it under its `to_u64()`/bound check, `gamma_code_signed_int` in the `Small` arm. Acceptance: one occurrence of `2 * k + 1` in signed.rs; `fused_coders_match_at_the_fast_path_seam` and the proptest pass.

### skyline-sweep-place-masked-32: The early-exit discipline is pinned only for the test-gated `sweep::eq`; production `causal_cmp` and both masked exits are unpinned
- Where: crates/before/src/version/skyline/sweep.rs:37-52 (related: crates/before/src/version/skyline/masked.rs:61-65; crates/before/src/version/skyline/sweep.rs:233-242; crates/before/tests/meter.rs:4983-5100, 9637-9649, 9906-9912; .cargo/mutants.toml:48-51)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of tests/meter.rs: the only early-exit pin is `mod eq_early_exit` (4999, under `limb-meter`) calling `meter::skyline::sweep::eq` at 5045; the masked rows assert `Some(Less)` with "no early exit" at 4360-4364 and 7483-7487; the concurrent references at 9639-9648 and 9906-9912 are bounds the fused walk must sit under, which only loosen if `order_exit` sweeps to exhaustion; mutants.toml:48-51 names only `eq_exit`'s guard as instrument-killed); executed: no
- Seen by: claims; refutation: confirmed (every measured `partial_cmp(...).is_none()` site checked); history: no rationale found (bff7b04c chose `eq()` and says nothing about the strict-mix exit)
- Owner-gated: no

The sweep doc's early-exit clause ("a decided sweep reads no more of either stream") is a measured number only for `eq`, which is `#[cfg(any(test, feature = "meter"))]`. The production exit predicate `order_exit` (the strict-mix `None` behind `Version`'s `PartialOrd`) and masked.rs's two exits have no work pin, and every masked row runs to exhaustion. An `order_exit` that always returns `Continue` is value-equivalent (`Directions::relation` at exhaustion still yields `None`), so every verdict suite stays green, and every relational row that uses a concurrent `partial_cmp` as its reference only loosens. Every criterion needs a committed demonstration that the known-bad mechanism fails it.

Evidence:

        44	//! is (any `D > 0`). A refuted direction stays refuted, so breaking at the
        45	//! question's resolution never moves a verdict the completed sweep would have
        46	//! reached, and a decided sweep reads no more of either stream. That last
        47	//! clause is a measured number, not just a claim: the `eq_early_exit` row of
        48	//! the resource-envelope suite (`tests/meter.rs`) pins the sweep's touch and

       121	#[cfg(any(test, feature = "meter"))]
       122	pub fn eq(a: BitsView<'_>, b: BitsView<'_>) -> bool {

    tests/meter.rs
      4996	// stay green. This is the one committed row where the early-exit prose
      4997	// is a measured number rather than a claim.
      4998	#[cfg(feature = "limb-meter")]
      4999	mod eq_early_exit {

Resolution: add `cmp_early_exit` rows in the `eq_early_exit` idiom: a pair decided concurrent at its second elementary interval with a scale-varying tail, absolute two-scale touch and scan pins on `Version::partial_cmp`, plus the masked twins (`(v / p).partial_cmp(&w)` with the deciding intervals inside owned regions, and a masked `eq`). Update sweep.rs:46-52 and masked.rs:61-65 to name the rows. Acceptance: with `order_exit` returning `Continue` unconditionally (and masked's `run` ignoring `Break`), all verdict suites remain green and the new rows read tail-linear and fail at both scales; restored, the rows read identical at both scales.

Construction: `a` = left half at height 1, right half a CliffComb tail of t teeth; `b` = left half at height 0, right half a single plateau above every tooth. Interval 1 has D > 0 (refutes `le`), interval 2 has D < 0 (refutes `ge`), so `order_exit` breaks at the second fold before either cursor enters the tail. Pin touches and scan bits at t and 2t with fixed tooth magnitude, as `eq_early_exit` does at tests/meter.rs:5004-5089.

### skyline-sweep-place-masked-33: sweep.rs names `Version`'s `PartialOrd` as the verdict oracle, but that is the sweep itself
- Where: crates/before/src/version/skyline/sweep.rs:56-58 (related: crates/before/src/version/skyline/sweep.rs:84-85; crates/before/src/version.rs:1730-1731; crates/before/src/version/skyline/sweep/tests.rs:1-10, 45)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep: `stored-form comparison` occurs only at sweep.rs:56 and :84; version.rs:1730-1731 (and the `&` variants at 1740-1751) implement `partial_cmp` as `skyline::sweep::causal_cmp`; `git show 66b5dc661:crates/before/src/version.rs` has `partial_cmp` calling `self.view().causal_cmp(o.view())` at lines 770-771; sweep/tests.rs:45 computes `want` from `to_oracle_version(a).partial_cmp(&to_oracle_version(b))`); executed: no
- Seen by: prose, claims; refutation: confirmed; history: deliberate but expired (true at 66b5dc66; faf3cd0a made `partial_cmp` the sweep two days later; 73624b27 re-anchored the tests to the recursive oracle the same day but touched sweep/tests.rs only)
- Owner-gated: no

The Testing section and `causal_cmp`'s doc name "the stored-form comparison ([`Version`]'s `PartialOrd`)" as the differential oracle. Today `Version`'s `partial_cmp` is `skyline::sweep::causal_cmp`, so the sentence says the function is tested against itself; the tests use the recursive oracle through the bridge. The phrase names an implementation since retired, breaching the hard rule that nothing refers to code that no longer exists, and it hides the strongest fact about this suite: the oracle shares no cursor, delta, or accumulator with the sweep.

Evidence:

        56	//! The stored-form comparison ([`Version`](crate::Version)'s `PartialOrd`) is
        57	//! the verdict oracle: differential tests pin all four entry points against it

        84	/// verdict matches the stored-form comparison exactly (the module doc's
        85	/// differential suite pins all four outcomes).

    version.rs
      1730	                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
      1731	                    skyline::sweep::causal_cmp(self.view().live(), o.view().live())

Resolution: re-denominate both sentences against what is: the recursive oracle (`oracle::Version`, the paper transcription, reached through `testing::bridge::to_oracle_version`) is the verdict witness, over the exhaustive small scope, the generator families, and the organic histories, as sweep/tests.rs:1-10 already says. Drop "stored-form comparison" from sweep.rs. Acceptance: `grep -n 'stored-form comparison' sweep.rs` returns nothing; the Testing section names the oracle sweep/tests.rs calls.

### skyline-sweep-place-masked-34: The debug-assert rationale paragraph is copied six times across sweep, masked, and place
- Where: crates/before/src/version/skyline/sweep.rs:123-127 (related: crates/before/src/version/skyline/masked.rs:133-137; crates/before/src/version/skyline/place.rs:220-223, 282-285, 345-348, 405-408, 431-435)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n 'argument loud'` returns sweep.rs:125, masked.rs:135, place.rs:220, 283, 346, 405); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate and holds for the asserts (aac1bc04, faa31262, b0194c6a record owner rulings to fail loudly and to propagate the `contains` precedent, comment included, to every twin)
- Owner-gated: yes (compressing the paragraphs is a style change to an owner-ruled precedent; the asserts themselves are not in question)

The same five-line rationale ("reaching the finish arm IS the verdict. The assertion keeps that control-flow argument loud: ... fails debug builds at the seam instead of silently re-deriving (or corrupting) the verdict") appears at six sites, three shouting "IS" in capitals, while `walk`'s doc (place.rs:431-435) already states the pattern once and each assert's message is already the one-line proof. Comments state what the code cannot show, once; per-site comments should carry only the site-specific fact (which hook breaks before the exhaustion check).

Evidence:

       123	    // Surviving to exhaustion is equality: `eq_exit` breaks on any refutation
       124	    // before the exhaustion check runs, so reaching the finish arm IS the
       125	    // verdict. The assertion keeps that control-flow argument loud: an exit or
       126	    // loop change that ever admits a refuted sweep here fails debug builds at
       127	    // the seam instead of silently re-deriving (or corrupting) the verdict.

Resolution: keep every assert and each site's first sentence (the site-specific control-flow fact); delete the "keeps that argument loud ... silently re-deriving" sentences, letting `walk`'s doc and the assert messages carry the rationale; lowercase "IS". Acceptance: `grep -c 'argument loud'` returns 0 or 1 per file; no capitalized IS in `//` comments.

### skyline-sweep-place-masked-35: `sweep::le` and `sweep::concurrent` have no caller outside their own tests, and `le`'s opener names wiring that does not exist
- Where: crates/before/src/version/skyline/sweep.rs:151-186 (related: crates/before/src/version/skyline/sweep.rs:7, 41-44, 57; crates/before/src/version/skyline/sweep/tests.rs:8-10, 67-81; crates/before/tests/meter.rs:5045; crates/before/src/version/skyline.rs:191; crates/before/src/version.rs:26-27; crates/before/src/version.rs:236)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep over crates/before/{src,tests,benches,examples,fuzz,fuzzfit,wasm32-pins,surfacecheck}, crates/before-fuelscape, and the rumors crate: `sweep::le`/`sweep::concurrent` appear only in sweep/tests.rs; the only non-test caller of a gated entry is `meter::skyline::sweep::eq` at tests/meter.rs:5045; `git log -S'sweep::le('` and `-S'sweep::concurrent('` return nothing; skyline.rs:191 `pub mod sweep;` under version.rs:26-27 `#[cfg(any(test, feature = "meter"))] pub mod skyline;`); executed: no
- Seen by: structure, correctness; refutation: confirmed; history: deliberate and holds for the four-entry-point differential design (sweep/tests.rs:8-10 states the rationale: "Every assertion runs all four entry points, so a bookkeeping error that misreads a direction ... has four chances to separate from the oracle"); the `meter` half of the cfg is 30759af0's blanket gating with no consumer for these two, and `le`'s opener is unexamined
- Owner-gated: yes (the items sit on the `meter` feature's public surface, and deleting them reverses a recorded differential design)

`concurrent` is a one-line wrapper on `causal_cmp` (it adds no chance of separating from the oracle beyond `causal_cmp` itself); `le` is a distinct sweep whose only exerciser is the test proving it correct. Both are `pub fn` gated `#[cfg(any(test, feature = "meter"))]`, so under `--features meter` they are public instrument surface with no consumer (Principle 3). `le`'s doc calls it "the fold behind causal containment checks", but no containment check calls it: `Version::concurrent` (version.rs:236) and every containment door go through `PartialOrd` and `causal_cmp`, as the same doc's last paragraph admits.

Evidence:

       151	#[cfg(any(test, feature = "meter"))]
       152	pub fn concurrent(a: BitsView<'_>, b: BitsView<'_>) -> bool {
       153	    causal_cmp(a, b).is_none()
       154	}
       ...
       159	/// The single-direction fold behind causal containment checks: stops at the
       160	/// first elementary interval where `a`'s height exceeds `b`'s.
       ...
       169	#[cfg(any(test, feature = "meter"))]
       170	pub fn le(a: BitsView<'_>, b: BitsView<'_>) -> bool {

Resolution: at minimum narrow `le` and `concurrent` to `#[cfg(test)]` and reword `le`'s opener to name it as the test-only single-direction exit predicate the differential suite exercises. If the owner prefers, delete both, drop their assertions from `assert_verdicts`, `exhaustive_small_scope_agrees`, and `organic_histories_agree`, and restate the module doc's entry-point list for `causal_cmp` and `eq` (`eq` stays: tests/meter.rs:5045 is its caller). Acceptance: `grep -rn 'sweep::le\|sweep::concurrent' tests benches examples` is empty and the two items are `#[cfg(test)]` or gone; `le`'s doc describes what it is.

### skyline-sweep-place-masked-36: Hand-maintained caller rosters and counts that have already drifted
- Where: crates/before/src/version/skyline/sweep.rs:191-193 (related: crates/before/src/version/skyline/overlay.rs:230-233; crates/before/src/version/skyline/masked.rs:19-20, 323, 335; crates/before/src/version/skyline/emit.rs:211; crates/before/src/version/skyline/place/filter.rs:91-94)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: emit.rs:211 `let mut directions = Directions::new();` and filter.rs:93 `directions: Directions,` are clients the roster omits; masked.rs:323 and :335 `let mut net = Accumulator::new();`); executed: no
- Seen by: prose; refutation: confirmed (the masked count is defensible as "at most three persistent", which is exactly the rot the doctrine forbids); history: deliberate but expired (bb6f083d's roster was accurate for a day; emit.rs adopted `Directions` in e4d4817f the next day and filter's `Pair` in db9dfa3e; c6ba2208 already set the house precedent of trimming client inventories: "each client module names what it consumes")
- Owner-gated: no

`Directions`' doc enumerates its clients as "this module's sweep, the masked co-walk, the placement walk's bound sides"; emit.rs and filter.rs's `Pair` are a fourth and fifth client. overlay.rs:233 asserts "which no current client does", a census of callers. masked.rs:19-20 counts the transient state as "three accumulators" while `block_skip` allocates a fourth (`net`) per block. Principle 5: no hand-maintained counts or caller rosters; two of these three have already rotted.

Evidence:

       191	/// Every comparison walk — this module's sweep, the masked co-walk, the
       192	/// placement walk's bound sides — folds one sign per elementary interval into
       193	/// this pair and asks its question of the survivors.

    overlay.rs
       230	    /// Semantically the choice is free only because every client algebra folds
       231	    /// commutative sums — any order yields the same fold values; a client with
       232	    /// a non-commutative fold would make the tie-break part of its answer,
       233	    /// which no current client does. The order is contract, not convenience: a

    masked.rs
        19	//! slot's step folds). Nothing recurses, and the transient state is the
        20	//! cursors' path bits plus three accumulators.

Resolution: sweep.rs:191-193, "Every comparison walk folds one sign per elementary interval into this pair ..." with no list; overlay.rs:233, replace the census with the requirement ("so a client's fold must be commutative"); masked.rs:20, "plus a constant number of accumulators" or name them structurally (one difference, at most one height integrator per masked side, a per-block net during a skip). Acceptance: no client list on `Directions`; the priority doc states the commutativity requirement; the masked module doc states no number that `block_skip`'s `net` falsifies.

## Positives

- overlay.rs:27-67 is a complete correctness argument stated once: the three dyadic facts (nesting, the flip-level tie test, the all-right path as exhaustion) are exactly what `advance`, `advance_set`, `LeafCursor::step`, and `IdLeafCursor::step` implement, the mixed done/live, tied, and dropped-slot cases included, and every `expect` and `unreachable!` in the partition (overlay.rs:448, 595, 605 are the models) is a one-line proof drawn from it. The tie debug-asserts are a legitimate O(1) probe of a structural property no differential test observes directly.
- `sweep::sweep` (254-288) carries each question as an exit predicate whose `Break` payload is the verdict, so three questions share one loop and no stale direction is handed back; `Directions::relation`, `order_exit`, and `eq_exit` are reused by masked.rs and emit.rs rather than re-spelled.
- The pair sweep's cost story is fully instrumented: `skyline_cmp_*` envelopes with zero-segment pins, two flatness bands with per-delta liveness floors, and `eq_early_exit` as an absolute two-scale pin with a stated mutation that fails it. `masked_cmp_hole_depth_band`'s `assert_eq!(lo, hi)` across a spine-depth doubling, with a shared ceiling and a ×0.75 liveness floor, is an instrument that pins the order, not just an envelope. The placement rows are stated relationally against the pair sweep, so no measured constant can rot.
- `advance`'s dual channel (fold callback in step order, positional return for re-coding) documents why both exist (overlay.rs:153-162), and every `CursorSet::priority` doc says which committed identity pins its order and which reorders move nothing.
- place.rs's `Fate` is a two-variant enum where a bool would have been the lazy choice; every finish arm debug-asserts the control-flow argument its hooks prove instead of re-deciding the verdict; place.rs:76-82 records the rejected alternative (a stream-level verdict vocabulary) with its reason.
- filter.rs:395-409 (the settle-order argument) explains not only why each guard exists but which agreement is coincidental, and the per-interval `debug_assert!` at 441-450 carries the argument as its message. `block_skip`'s justification (masked.rs:296-311) is stated in full and is correct.
- The Panics contracts are uniform across the partition and cite one home (`causal_cmp`); masked/tests.rs pins the contract's negative space (a validator-rejected witness sweeps without panicking) rather than an unspecified verdict.
- signed.rs derives `GAMMA_SMALL_MAG_BOUND` inline (212-214), keeps `Sign` two-valued with the zero convention stated once, and signed/tests.rs is a model differential: the fused coder against the unfused composition, a deterministic seam sweep at `2^31 ± 4` beside the proptest, and `signed_le`/`signed_max` against an exact `IBig` value order over both `Int` spellings and negative zeros. `fold_signed_int` dispatches word-scale deltas straight to `add_u64`/`sub_u64`.
- Every test in the partition has a doc comment stating its invariant, and the ones checked against their bodies (the flush-right tie geometry, the seam-magnitude strategy, the small signed grid, the filter witness sets) are accurate; `demand_lists` enumerates the demand kinds so every `Demand` arm is reached rather than sampled.
- Every meter row, law, and cross-module name the docs cite exists, and the committed signed seed shrinks to exactly `2^31`, the fast-path bound the tests claim to hunt.

## Open questions for Finch

1. Is the `meter`-feature `pub mod skyline` surface (and so `sweep::le`/`sweep::concurrent`) part of the stable public API? Finding 35 is owner-gated on that basis. Recommendation: treat it as instrument surface, narrow the two to `#[cfg(test)]`, and keep `eq` because a meter row calls it.
2. For the filter cost claim (finding 21): restate the bound with a k-scaling row, or restructure the read loop and the deepest-slot pick? Recommendation: restate now and add the two-scale k row; restructure only if rumors' classifiers are expected to conjoin many concurrent holes, since the dirty-flag gate plus a depth heap is real machinery the current small-k callers never exercise.
3. For the masked height debug-asserts (finding 3): delete them, or amend the Panics contract? Recommendation: delete; the asserts check an input property the validator owns, and the differential laws separate any fold-orientation bug.
4. For the masked-hole band (finding 4): is a nonzero-delta spine variant wanted as the fold floor? Recommendation: state the zero-delta premise now; add the variant only if the fold cost, not just the sign reads, is meant to be pinned.
5. The `Step` vs `Signed` split (overlay.rs:288-300): I read it as carrying a real invariant (a `Step` from `unzigzag` is always normalized; a `Signed` may carry a negative zero by documented slack, signed.rs:98-99), and a736ef14 #11 ruled on the distinction. Recommendation: leave it.
6. The claims lens notes that query.rs:533-549 (the skyline-query partition) uses the same peek-then-break shape inside the projection walk. If the outer loop advances the id cursor repeatedly while the event cursor stays put, finding 5's construction applies there too. Recommendation: hand to the query reviewer.

## Dropped

- LeafCursor and walk::LeafWalk share a descend/backtrack skeleton (structure [4]): deliberate and documented; 248d5539 ruled "no structural unification", the rationale is stated at walk.rs:8-12 and overlay.rs:313-316, and the finding brought no new evidence.
- The coverage emptiness/fullness conditions are tested only against transcriptions of themselves (correctness [29]): refuted; `coverage_is_exact_on_the_two_party_grid` (causally/tests.rs:222-229) is a two-sided brute-force membership census over every version, segment, hole spelling, and conjunction of the two-party grid, and `coverage_bounds_membership` (laws.rs:1029-1039) is the one-sided soundness law; the seeded mutation also separates from `composed_coverage`'s `!lt(lo)` at `lo == bound`.
- The pair-difference seeding re-spelled in place.rs and filter.rs (correctness [30]): duplicate of finding 15.
- sweep::le's opener overstates its role; le and concurrent are test-only (correctness [32]): duplicate of finding 35.
- filter cost claim drops the per-interval factor (claims [35]): duplicate of finding 21.
- sweep.rs names its own PartialOrd as the oracle (claims [38]): duplicate of finding 33.
- masked Panics contract vs debug_assert (prose [13], claims [39]): duplicates of finding 3.
- "Partial is called conservative" (part of prose [22]): refuted; `Query::coverage` routes filter's `Partial` through `refine_partial` ("the exact emptiness decision the fused endpoint fold cannot reach", causally/query.rs:143-144), so at the stream layer the verdict is conservative by design.
- Duplicated first sentence (part of correctness [31]): merged into finding 26.
- laws.rs:1036-1038's reference to a `Coverage` precision contract that the `Coverage` rustdoc does not carry (raised as new by the refutation pass): out of scope for this partition; hand to the causally/laws sweep.
