# Partition rank: Rank and Ranked: the total order extending causality, and its numeric core

## Partition summary

The partition is the causal rank kernel: `Rank` (crates/before/src/version/rank.rs, 1096 lines) is an exact dyadic rational `num · 2^-exp` with normalized storage (odd numerator whenever `exp > 0`, zero pinned to exponent zero) so that derived `Eq`/`Hash` are value equality; it carries `Ord` (an O(1) magnitude-class test, then an MSB-window stream on class ties), `Add`/`checked_sub`/`Sum` (a backend route when both aligned operands fit the big-integer backend, a suanpan `Accumulator` route past that), and a prefix-ascending wire form whose byte order equals `Ord` (an inverted-polarity Elias delta integral part, then continuation-framed eight-bit fraction groups, then a close bit). `Num` (rank/num.rs, 650 lines) is the two-arm numerator: a `Base` (dashu `UBig`) below the backend's capacity ceiling and a raw `Vec<u64>` limb vector above it, existing so that on 32-bit targets a decoded numerator past `2^32 - 32` bits is a value rather than a backend panic; a test-only thread-local override lowers the ceiling so both arms and their seam are reachable at host sizes. `Ranked` (ranked.rs, 389 lines) is the total-order view over a `Version`: rank first via a fused signed co-sweep, canonical version bytes on rank ties, with a composite key whose byte order is `Ord` on the views. rank/num/tests.rs (191 lines) is the partition's one test file: differential unit suites for the wide arm against `UBig` as oracle under a 96-bit ceiling. The partition's behavioral suites live in version/tests.rs (the rank, wire-form, wide-arm, ranked, and composite-key sections, roughly lines 753-2115) and in the `RANK_TRIPLE`, `VERSION_SOLO`, and `VERSION_PAIR` law groups in laws.rs; I read those sections, the skyline query kernel (`rank`, `rank_cmp`, `pair_fold`, `Integrator::finish`), suanpan's `shl` and `sign_limbs`, the meter and board pins for rank rows, the fuelscape roster and JSON, the borsh boundary, the wasm32 pins, and the pinned dashu-int 0.5.0 source to settle anchors. Total: 2326 partition lines read in full plus roughly 900 lines of anchors elsewhere.

The mechanisms are correct and total as far as reading can establish, and the argumentation is unusually complete: the wire-form module doc is a checkable proof sketch with every rejected alternative named and the concrete reason it fails, the decoder allocates only from bits actually read, `Rank::cmp` is obviously right and cheap, and the two-arm dispatch invariant is stated once and held to the real backend on the load-bearing side by the wasm32 pins. No correctness defect in the code was found by any lens or by me.

The dominant issues are claims the code does not honor, and one instrument aimed at the wrong adversary. `Ranked`'s public `# Complexity` promises `O(|self| + |other|)` while the kernel it calls runs the same mass-balanced settle as `distance`/`lag` (the crate's own doc stated the M-bound at commit 3bba6cbb and lost it when contracts moved into the fuelscape roster). `Ranked::encode_rank` is documented at four sites as a fused emission cheaper than `rank().encode()`, and has been `rank().encode()` spelled in three lines since the commit that introduced it, so the `raw_parts`/`encode_parts` seam and two public efficiency sentences describe nothing. The bold public size claim "never is larger than the version it measures" fails at small scale (a one-byte version whose rank encodes to two bytes). `sum_ranks`' amortization argument is false for ascending exponent order, and both committed pins fix the benign order while documenting it as the adversary. Below those: the wide arm's sixteen meter hooks feed a counter nothing observes, a handful of local dedupes (two routing copies, two shift kernels, a one-caller wrapper, four byte-source adaptors), a test doc that claims hashing coverage the body lacks, and prose habits (dated "historical" rationale, moralized "honest", hand-maintained literals, a sentence hand-copied eleven times) that the crate shares beyond this partition.

## Findings

### rank-1: Hand-maintained literals and counts in prose that nothing enforces
- Where: crates/before/src/version/rank.rs:36-39 (related: rank.rs:543, rank.rs:920-921, rank.rs:623, rank.rs:799-804, crates/before/src/version/rank/num.rs:12, crates/before/src/version/rank/num/tests.rs:189, crates/before/src/version/tests.rs:1187-1190)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for each literal; hand arithmetic for the assert message: `vec![1, 0, 5, 0]` strips to three limbs with top limb 5, three bits wide, so 2·64 + 3 = 131 bits; the only enforced number is the `encoded_bits <= input_bits` assert at version/tests.rs:1240-1241); executed: no
- Seen by: prose, correctness, claims, structure; refutation: confirmed (the `dsi-bitstream 0.10` item dropped: it names the minor line evaluated); history: no rationale found (0.56 entered in 02f6180f as a design record quoting the measurement)
- Owner-gated: no

The module doc argues a design choice from a measured constant (0.56) that no test enforces (the pin is 1.0), the provenance test's doc lists five measured ratios its body never checks, `Three clauses` introduces a predicate with two clauses per operand, `The four reference forms` counts `impl Add` blocks by hand, `~604 MB` is written at four sites with its derivation (`9/64 · exp` bytes) only in the wasm32 guest, and a test assert message states a false width. Principle 5: no hand-maintained counts or dated measurement literals; a number that matters lives in a mechanically enforced place the prose cites by name.

Evidence:

        36	//!    width *again* and drive the worst committed provenance family
        37	//!    (the lone wide counter, measured at 0.56 encoded bits per
        38	//!    packed input bit) up against the 1.0-per-family
        39	//!    provenance-linearity pin; delta's `N + O(log N)` is what keeps

       543	/// Three clauses, all width facts: each exponent gap must fit the

       920	// what makes [`Version::distance`](crate::Version::distance) a metric. The four
       921	// reference forms mirror [`Base`]'s own `Add` matrix so callers need not place

    (num/tests.rs)
       189	    assert!(wide.is_wide(), "129 bits exceeds the 96-bit test ceiling");

Resolution: Either pin each family's ratio in `rank_encoding_size_is_provenance_linear` as a ceiling with slack (wide counter at or under 0.7, say) and have rank.rs:37-38 and version/tests.rs:1187-1190 cite the pin by name, or delete the raw figures and keep only the enforced 1.0 statement. Rewrite 543 as "Two clauses per operand, both width facts"; 920-921 as "The reference forms mirror Base's own Add matrix". Derive ~604 MB once in num.rs's module doc ("9/64 · 2^32 bytes") and refer to "the fraction-form capacity crossing" at the other three sites. Replace the assert message with one computed from the constants: `format!("131 bits exceeds the {TEST_CEILING_BITS}-bit test ceiling")`, or assert `wide.bits() > TEST_CEILING_BITS`. Acceptance: no numeric literal in the partition's prose duplicates an enforced constant or an unenforced measurement; `python3 -c 'print(((5<<128)+1).bit_length())'` prints 131 and matches the message.

### rank-2: Rank and Ranked tests live in the parent module's test file
- Where: crates/before/src/version/rank.rs:144-147 (related: crates/before/src/version/tests.rs:753, 948, 1347, 1741, 1865)
- Class / severity / confidence: modularity / nit / high
- Provenance: verified (`ls crates/before/src/version/rank/` shows `num` and `num.rs` only; the section headers in version/tests.rs are at the lines cited); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (legacy placement from 18206f21, when `area.rs` appended its tests to `version/tests.rs`; every later suite followed)
- Owner-gated: no

The crate applies the sibling `tests.rs` convention everywhere else (num.rs included), but roughly 1350 lines of rank, wire-form, wide-arm, ranked, and composite-key suites sit in version/tests.rs, and the `#[cfg(test)]` re-export below exists only to let them reach `num`'s ceiling override from there.

Evidence:

       144	mod num;
       145	use num::{arm_ceiling_bits, Num};
       146	#[cfg(test)]
       147	pub(crate) use num::{ceiling as arm_ceiling, BACKEND_CAPACITY_BITS};

Resolution: Move the five sections into `version/rank/tests.rs` and `version/ranked/tests.rs` (everything they reach is already `pub(crate)`), and drop the re-export. Acceptance: rank.rs and ranked.rs each declare `mod tests;`; version/tests.rs holds no Rank- or Ranked-specific suites.

### rank-3: Copyedit slips in the public Rank and Ranked type docs
- Where: crates/before/src/version/rank.rs:152-166 (related: rank.rs:172, rank.rs:216-221, rank.rs:361, crates/before/src/version/ranked.rs:29)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Seen by: prose, structure, correctness, claims; refutation: confirmed; history: no rationale found (b3f09baa and 8d8a06e2, the owner's docs pass; typos)
- Owner-gated: no

The first paragraph a library user reads about `Rank` doubles a connective ("This means that ... therefore"), carries a stray "in", and disagrees in number ("[`Version`]s it ranks"); the informal statement says "strictly monotone in ticks" while the displayed formal statement is over the causal order `<` (joins raise rank too). Further down: "a very deliberate property" (172), a former/latter chain whose referents switch mid-paragraph (216-221), "The reason this encoding is crafted this way is to support" (361), and "In this case, we say that" introducing a definition (ranked.rs:29).

Evidence:

       152	/// The [`Rank`] of a [`Version`] is **strictly monotone** in
       153	/// [`tick`](crate::Version::tick)s; that is, for every pair of versions `v` and
       154	/// `w`:
       155	///
       156	/// > if `v < w` then `v.rank() < w.rank()`.
       157	///
       158	/// Contrapositively, **equal ranks are never causally ordered** (they are the
       159	/// same version or concurrent). This means that any tiebreak between equal
       160	/// ranks therefore extends [`Rank`]'s induced causal order to a total one. This
       161	/// makes `Rank` well-fitted for sorted-container keys that must deliver causes
       162	/// before effects. Indeed, the [`Ranked`](crate::Ranked) view builds exactly
       163	/// such a total order in, with the version's own bytes as the tiebreak.
       164	///
       165	/// [`Rank`], and its companion view [`Ranked`], are both totally ordered
       166	/// ([`Ord`]), unlike the [`Version`]s it ranks.

Resolution: "strictly monotone in the causal order"; "so any tiebreak between equal ranks extends the causal order to a total one"; "builds exactly such a total order, with the version's own bytes as the tiebreak"; "unlike the Versions they rank"; drop "very deliberate"; at 216-221 name the two versions ("the two-tick version strictly out-ranks the one-tick version"); 361 "This encoding exists to support"; ranked.rs:29 "Rank-equal distinct versions are ordered by". Acceptance: the listed sentences read as amended and the prose agrees with the displayed formal statement.

### rank-4: The bold public size claim is false at small scale
- Where: crates/before/src/version/rank.rs:198-202 (related: crates/before/src/version/tests.rs:962-1014, version/tests.rs:1179-1246, crates/before/src/version/skyline.rs:11-24, crates/before/src/codec/tests.rs:78-80, crates/before/src/version.rs:1129-1131)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (hand arithmetic from the committed coding and goldens: skyline.rs:11-24 gives one preorder flag bit per node, the first leaf as `gamma(height)`, later leaves as `gamma(zigzag(delta))`; codec/tests.rs:78-80 pins `gamma(0)` at 1 bit and `gamma(2)` at 3 bits; version.rs:1131 gives `encode().len() == (encoded_bits() + 1).div_ceil(8)`; version/tests.rs:974 pins rank 1/2 at `[0x60, 0x00]`); executed: no
- Seen by: correctness; refutation: confirmed (re-derived independently); history: no rationale found (the sentence is a new `# Complexity` section from 8d8a06e2 with no instrument behind the universal)
- Owner-gated: no

The version `"(0, 1, 0)"` is root flag `0`, leaf `1` + `gamma(0)` = `1`, leaf `1` + `gamma(zigzag(+1) = 2)` = `011`: 7 live bits, one byte with its marker; its rank is 1/2, whose canonical bytes are the committed two-byte golden. `Version::try_from(1)` is `1` + `gamma(1)` = 4 bits while its rank golden `[0x80]` is 8 bits, so in the provenance pin's own denomination the inequality `encoded_bits <= input_bits` fails at that scale, and the pin's five fixed-scale families at ratio at or under 1.0 are an envelope, not a proof of "never". Statement faithfulness: never stronger than proven; the crate docs hold every space claim as a hard guarantee, and this one is in public rustdoc a KV-store user would size keys by. The paragraph that follows argues only a constant-factor bound.

Evidence:

       198	/// **A rank's representation never is larger than the version it measures, and
       199	/// often is exponentially smaller.** In-memory size and encoded size are both,
       200	/// within small constant factors, at most the length of the rank's binary
       201	/// expansion, which we'll write `‖r‖`. Notably, `‖r‖` is at most linear in the
       202	/// size of the originating version.

    (version/tests.rs)
       973	        // 1/2 = "0" ++ "1 10000000 0".
       974	        (rank_of("(0, 1, 0)"), vec![0x60, 0x00]),
      ...
       978	        // 1 = "1000" ++ "0": the first integral header step.
       979	        (int(1), vec![0x80]),

Resolution: Restate the bold sentence as the bound the paragraph argues (a small constant multiple of the version's packed size, and often exponentially smaller), and state the denominator the pin measures (encoded rank bits against packed version bits at the tested scales). If a byte-size bound is wanted as a contract, derive it (per level at least two topology/payload bits against 9/8 fraction bits; a b-bit counter costs 2b+1 gamma bits against b + 2 log b) with the small-scale exception stated, and pin it at two scales. Acceptance: the public doc no longer asserts an unqualified "never"; a committed test asserts the small-scale witness (`"(0, 1, 0)".parse::<Version>().unwrap().encode().len() == 1` alongside the existing 1/2 golden), and the provenance pin's doc names its denominator and scales.

Construction: `let v: Version = "(0, 1, 0)".parse().unwrap(); assert_eq!(v.encode().len(), 1); assert_eq!(v.rank().encode(), vec![0x60, 0x00]);` (rank bytes 2 > version bytes 1). Bit-denominated, the pin's own: `let one = Version::try_from(1).unwrap(); assert!(one.rank().encode().len() * 8 > one.encoded_bits() as usize);` (8 > 4). Both rank-side byte strings are already committed goldens at version/tests.rs:974 and 979.

### rank-5: The exp field doc states the bound for one construction path
- Where: crates/before/src/version/rank.rs:255-257 (related: rank.rs:577-582, rank.rs:1020-1028, rank.rs:788)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (rank.rs:788 `let exp = frac_len;` is the decoder's exponent, counted from groups read; the two-case bound is spelled at 579-582 and 1023-1027); executed: no
- Seen by: prose, correctness; refutation: confirmed; history: deliberate but expired (18206f21 wrote the field doc when the tree fold was the only producer; f0f3a2ae added the decoder and 6323d667 re-derived its bound in the commit message but left the field doc)
- Owner-gated: no

The field is the invariant's home, and both panic arguments (accumulate's and sum_ranks') rest on it, yet it states the bound for version-derived ranks only while a decoded rank's exponent is bounded by fraction bits read and `Add`/`Sum` carry the operands' maximum. The full two-case bound is then re-derived at two consumer sites instead.

Evidence:

       255	    /// The (binary) exponent of the denominator `2^exp`. Bounded by the
       256	    /// event tree's depth, since each level halves the interval width.
       257	    exp: u64,

       580	/// any honest exponent: a decoded exponent is counted from fraction bits
       581	/// actually read, under 2³⁵ from a whole 32-bit address space, and a
       582	/// version-derived exponent is bounded by its tree's stored bit length.

Resolution: On the field: "Bounded by bits already resident: a version-derived exponent by its tree's stored bit length (each level halves the interval), a decoded exponent by the fraction bits actually read, and a sum by its operands' maximum; under 2^35 on a 32-bit target, which keeps every usize-indexed digit position (suanpan's documented panic) unreachable." Reduce 579-582 and 1023-1027 to a citation of the field's bound. Acceptance: the two-case bound appears once, on `exp`; the two consumers cite it.

### rank-6: checked_sub and saturating_sub docs name a parameter the signatures do not have
- Where: crates/before/src/version/rank.rs:278-298 (related: rank.rs:332-337, rank.rs:354)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lines 278, 298, 332-333, 337, 354 read); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (14d36c62 named it `rhs`; 8d8a06e2 renamed the signature to `other` and left the docs)
- Owner-gated: no

Both public docs describe `self - rhs` while the parameter is `other`, and saturating_sub's doc says "handled arm by arm" where "arm" everywhere else in this module means a `Num` storage arm.

Evidence:

       278	    /// The difference `self - rhs`, or [`None`] when `rhs` exceeds `self`.
      ...
       298	    pub fn checked_sub(&self, other: &Rank) -> Option<Rank> {

       332	    /// The difference `self - rhs`, or [`Rank::ZERO`] when `rhs` exceeds
       333	    /// `self`.
      ...
       337	    /// remaining" rather than be handled arm by arm.

Resolution: Rename the parameter to `rhs` (matches `Add` and the docs; parameter names are not part of the API) or change the docs to `other`; replace "handled arm by arm" with "matched on". Acceptance: parameter names in the two signatures equal the names in their docs; "arm" in rank.rs refers only to `Num`'s arms.

### rank-7: "A None or zero result allocates nothing" has no allocation-counting instrument
- Where: crates/before/src/version/rank.rs:287 (related: rank.rs:343, crates/before/tests/meter.rs:1348-1353, crates/before/src/meter/board/ops.rs:490)
- Class / severity / confidence: verification-gap / nit / high
- Provenance: verified (tests/meter.rs:1349-1352 measures `cmp`, `checked_sub`, and `+` under one envelope; board/ops.rs:490 `heap: na(NA_HEAP_IN_PLACE)`); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (b5a81583)
- Owner-gated: no

The claim is true by reading (`Rank::cmp` uses stored widths and borrowed MSB windows, and the `None`/`Equal` arms return before any alignment), but no committed check fails if a future `cmp` allocates: the pair envelope's heap column covers three operations together and the board's heap floor is N/A. Every public claim wants a committed check that fails when it is false.

Evidence:

       287	    /// A `None` or zero result allocates nothing.

       343	    /// A floored result allocates nothing.

    (tests/meter.rs)
      1349	            let ord = a.cmp(&b);
      1350	            let diff = b.checked_sub(&a);
      1351	            let sum = &a + &b;

Resolution: Add a meter row (or a unit test under the counting allocator) that runs `three.checked_sub(&five)` and `five.checked_sub(&five)` on wide operands alone and asserts zero heap delta. Acceptance: the row is committed and reads 0 bytes; wrapping the class computation in a `Vec` allocation makes it fail.

Construction: Under tests/meter.rs's counting allocator, build `a = version_of(&Shape::Dense.packed1(RANK_PAIR_DEPTH)).rank()` and `b = Version::try_from(3).unwrap().rank()`; measure peak heap over `a.checked_sub(&b)` alone (the `None` arm) and over `a.checked_sub(&a)` (the zero arm); assert both are 0.

### rank-8: The two-route dispatch is written twice, threaded by bool parameters
- Where: crates/before/src/version/rank.rs:314-322 (related: rank.rs:949-957, rank.rs:587, crates/before/src/version/rank/num.rs:426, crates/before/src/version/skyline/signed.rs:58)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both blocks read; `accumulate(lhs, rhs, e, subtract_rhs: bool)` at 587 threads into `fold_into(&self, acc, shift, subtract: bool)` at num.rs:426; `Sign` is `pub(super)` at signed.rs:58 and not reachable here); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (both copies arrived in 79a944ab, whose message speaks of "a routing predicate" in the singular)
- Owner-gated: no

`checked_sub`'s Greater arm and `Add::add` spell the same routing, differing only in the headroom literal (0 vs 1), the backend operator (`-` vs `+`), and the bool passed to `accumulate`; the routing argument is restated in two comment blocks (299-313, 940-948). A change to the predicate's contract must land twice, and the call sites `accumulate(self, other, e, true)` / `accumulate(self, rhs, e, false)` do not say what the bool means.

Evidence:

       314	                let e = self.exp.max(other.exp);
       315	                if let (Num::Base(a), Num::Base(b)) = (&self.num, &other.num) {
       316	                    if backend_alignment_fits(a, self.exp, b, other.exp, e, 0) {
       317	                        let a = a.clone() << (e - self.exp);
       318	                        let b = b.clone() << (e - other.exp);
       319	                        return Some(Rank::from_raw(a - &b, e));
       320	                    }
       321	                }
       322	                let difference = accumulate(self, other, e, true);

       949	        let e = self.exp.max(rhs.exp);
       950	        if let (Num::Base(a), Num::Base(b)) = (&self.num, &rhs.num) {
       951	            if backend_alignment_fits(a, self.exp, b, rhs.exp, e, 1) {
       952	                let a = a.clone() << (e - self.exp);
       953	                let b = b.clone() << (e - rhs.exp);
       954	                return Rank::from_raw(a + &b, e);
       955	            }
       956	        }
       957	        accumulate(self, rhs, e, false)

Resolution: One local `enum Op { Add, Sub }` with `fn headroom(self) -> u64` and `fn backend(self, a: Base, b: &Base) -> Base`; one `fn combine(lhs: &Rank, rhs: &Rank, op: Op) -> Rank` holding the routing and its comment once; `Add::add` becomes `combine(self, rhs, Op::Add)` and the Greater arm `Some(combine(self, other, Op::Sub))`; `accumulate` and `Num::fold_into` take `Op` instead of `bool`. Acceptance: one call site of `backend_alignment_fits`; no `bool` parameter on `accumulate` or `fold_into`; the `RANK_TRIPLE` laws, `rank_wide_arm_arithmetic_matches_the_backend_oracle`, the wasm32 `pin_rank_add`/`pin_rank_checked_sub`, and the `RANK_PAIR_MISMATCH`/`RANK_SUM_MIXED` envelopes read identically (pure refactor).

### rank-9: Rank::decode's "# Decoded size" section describes the encoded size and cites text that lives on encode
- Where: crates/before/src/version/rank.rs:428-433 (related: rank.rs:372-376)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: the suffix-safety paragraph is at 372-376 on `encode`; nothing above 428 in `decode`'s doc mentions it); executed: no
- Seen by: prose, structure, correctness, claims; refutation: confirmed; history: no rationale found (added fresh on `decode` in 8d8a06e2, cross-reference dangling at birth)
- Owner-gated: no

A first-time reader of `decode` finds a heading about decoded size, a fact about encoded size, and a dangling "above". The 9/8 bound is a property of the encoding and belongs on `encode` or the type's `# Complexity`, which already introduces `‖r‖`.

Evidence:

       428	    /// # Decoded size
       429	    ///
       430	    /// The serialized representation of a [`Rank`] is at most `9⁄8 · ‖r‖ +
       431	    /// O(log ‖r‖)` bits: one bit per integral bit, nine bits per eight
       432	    /// fractional bits (this is required to keep distinct ranks' encodings
       433	    /// prefix-free, providing the above generalized suffix-safety).

Resolution: Move the bound onto `Rank::encode` after its suffix-safety paragraph as "# Encoded size", or into the type-level `# Complexity` beside the `‖r‖` definition; delete the section from `decode`. Acceptance: `decode`'s doc has no size section; the 9/8 bound appears once, following the suffix-safety text it references.

### rank-10: Rank::decode's # Errors says NotCanonical needs 2 EiB of input; a nine-byte header reaches it
- Where: crates/before/src/version/rank.rs:442-444 (related: rank.rs:727-731, rank.rs:124-131, crates/before/src/version/tests.rs:1164-1171)
- Class / severity / confidence: claim / low / high
- Provenance: verified (the check at 727-731 fires after a unary run of 64 ones; the committed genre witness at version/tests.rs:1167-1170 feeds `[0xFF; 8] ++ [0x00]` and asserts `NotCanonical`); executed: no
- Seen by: correctness, claims; refutation: confirmed; history: no rationale found (8d8a06e2 transcribed the code comment's size argument as the trigger; the nine-byte witness predates it, 6ae76895)
- Owner-gated: no

The `# Errors` section is the contract a caller decides handling by, and it calls this arm effectively unreachable; the crate's own test reaches it with nine bytes. What needs 2 EiB is a canonical stream of that width, not the rejection; the sentence conflates the error's trigger with the bound's provenance. The module doc at 128-131 already has the accurate framing.

Evidence:

       442	    /// - [`Decode::NotCanonical`] when otherwise valid content exceeds the type's
       443	    ///   representation bound (an integral mantissa of `2⁶⁴` or more bits, effectively
       444	    ///   unreachable, since it can only be hit by reading inputs of 2 EiB or more);

       727	    if rho >= 64 {
       728	        // The format bound: an integral width of 2⁶⁴ or more bits exceeds both
       729	        // the numerator this crate can hold and any input under 2 EiB (the
       730	        // mantissa alone would need 2⁶⁴ − 1 bits).
       731	        return Err(Decode::NotCanonical);

Resolution: Reword: "[`Decode::NotCanonical`] when the integral header claims a mantissa of 2^64 or more bits (a unary run of 64 or more ones: eight leading 0xFF bytes); no canonical encoding carries such a header, since its mantissa alone would exceed 2 EiB, so the genre only ever names a forged stream." Acceptance: the clause names the trigger and separates it from the size argument; the genre test passes unchanged.

Construction: `assert!(matches!(Rank::decode(&[0xFF; 8][..].iter().copied().chain([0x00]).collect::<Vec<u8>>()[..]), Err(Decode::NotCanonical)))`; this is the committed case at version/tests.rs:1167-1170.

### rank-11: The same shouted warning appears verbatim on raw_parts and from_raw; the invariant it guards is never stated positively
- Where: crates/before/src/version/rank.rs:493-509
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (both paragraphs read); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate and holds (owner-authored in 8d8a06e2; the purpose, a public `(num, exp)` constructor is a size bomb, is stated at both sites)
- Owner-gated: no

The paragraph is duplicated and shouts; a stated invariant does the work the emphasis is trying to do, once: every public construction path bounds `exp` by bits already resident, which is what keeps `encode`'s output linear in memory held.

Evidence:

       493	    /// It is **VERY IMPORTANT** that these not be exposed together, with the
       494	    /// `from_raw` constructor, as this creates an affordance for constructing
       495	    /// exponential serialization-size bombs.
      ...
       507	    /// It is **VERY IMPORTANT** that these not be exposed, together with the
       508	    /// `raw_parts` destructor, as this creates an affordance for constructing
       509	    /// exponential serialization-size bombs.

Resolution: State once, on `from_raw`: "Crate-private, and must stay so together with `raw_parts`: every public construction path bounds `exp` by bits already resident (tree bits or bits read), which keeps `encode`'s output linear in memory held; a public `(num, exp)` constructor would let a 16-byte value encode to `2^exp / 8` bytes." On `raw_parts`: "See `from_raw` for why the pair stays crate-private." Acceptance: `grep -n 'VERY IMPORTANT' crates/before/src/version/rank.rs` is empty and the exp-bound invariant is stated at `from_raw`.

### rank-12: from_num's doc names the decoder as a producer it does not serve
- Where: crates/before/src/version/rank.rs:514-519 (related: rank.rs:813-817)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'from_num(' crates/before/src` lists exactly rank.rs:511, 609, 1049 as callers; the decoder constructs `Rank { num, exp }` directly at 817 behind the debug_assert at 813-816); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (inaccurate from birth: the decoder already bypassed normalization one commit before 79a944ab wrote this doc)
- Owner-gated: no

A reader auditing the canonical-form invariant (structural `Eq`/`Hash` rest on it) is told there is one normalization funnel when the decoder is a second, independent guarantor of the same invariant, relying on strict minimal packing. That bypass is deliberate and deserves to be named rather than hidden by an inaccurate list.

Evidence:

       516	    /// The shared normalization every raw `(numerator, exponent)`
       517	    /// producer — the folds, the decoder, the accumulator readout — lands
       518	    /// through, which also re-dispatches the stripped numerator onto its
       519	    /// canonical arm.

       817	    Ok(Rank { num, exp })

Resolution: "The shared normalization the folds and the accumulator readout land through, which also re-dispatches the stripped numerator onto its canonical arm. The decoder is the one producer that bypasses it: strict minimal packing already guarantees an odd numerator whenever `exp > 0` (its debug_assert states the premise), so it constructs the normalized value directly." Acceptance: the doc's producer list equals the call graph and the decoder's bypass is stated at `from_num` or at line 817.

### rank-13: Register and vocabulary: "honest", "loud", "rent", "sliver", "door", "two-ways pin", "rank-class"
- Where: crates/before/src/version/rank.rs:549-552 (related: rank.rs:580, rank.rs:1068, rank.rs:101, crates/before/src/version/rank/num.rs:9-19, num.rs:33, num.rs:91, num.rs:104-105, num.rs:383, num.rs:502, crates/before/src/version/rank/num/tests.rs:10, crates/before/src/version/ranked.rs:46, ranked.rs:284)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -i honest` over the four files: rank.rs:550, 580, 1068; num.rs:11, 104, 383, 502; `door` has no definition in lib.rs and 208 uses under crates/before/src; the other sites read); executed: no
- Seen by: prose, correctness, claims, structure; refutation: confirmed for "honest"; reframed for "door" (crate-wide, no definition site) and "class" (two files, both qualified; a nit at most); history: no rationale found ("honest" is a crate-wide idiom with a consistent sense, 127 uses; "rent" is 30a50f65; "sliver"/"loud" are 79a944ab)
- Owner-gated: no

Moralized or economic modifiers stand in for a stated mechanism: "honest exponent" (an exponent a version or decoded stream can carry), "honestly outgrow"/"honestly reachable" (reachable through `Rank::decode`), "honest exhaustion" (allocation failure), "honest price" (quadratic cost), "loud backend panics", "the minimum rent for in-band delimitation", "the sliver it cannot". "door" is a coinage with no definition by contrast anywhere in the crate. "The two-ways pin on the wire" compresses a doctrine phrase into a code comment whose next clause states the mechanism anyway. At rank.rs:549-550 the modifier also understates the argument: on 64-bit targets the gap clause holds for every `u64` exponent (`usize::try_from(u64)` never fails), not merely "below any honest exponent". The vocabulary rule: every coined term is anchored or defined once; moralized code names no mechanism. "honest", "door", and the em-dash habit (rank-15) are crate-wide, so partition-local edits would make these files the inconsistent ones; the partition follows a crate-level ruling.

Evidence:

       548	/// cost, never results. On 64-bit targets the capacity clause is
       549	/// unreachable below allocatable memory and the gap clause below any
       550	/// honest exponent, so every rank that exists routes to the backend there;
       551	/// the accumulator route is live exactly where 32-bit targets need it, and
       552	/// under the test ceiling.

    (num.rs)
        11	//! space — and the rank wire door can honestly outgrow it: the fraction
        12	//! form reaches a wider numerator from ~604 MB of input, and the integral
        13	//! form from ~512 MiB, both loud backend panics rather than values without

    (ranked.rs)
       284	        // The two-ways pin on the wire: the key's rank component must

Resolution: Replace each modifier with the mechanism it abbreviates: rank.rs:550 "and the gap clause holds for every u64 exponent on a 64-bit target"; rank.rs:580 "above any exponent a constructor can produce"; num.rs:11 "the rank wire form can outgrow it through Rank::decode"; num.rs:13 "both backend panics rather than values"; num.rs:104 "reachable only past ~2^32 bits on a 32-bit target"; num.rs:383 "and allocation failure"; rank.rs:1068 and num.rs:502 "quadratic in the width"; rank.rs:101 "the minimum overhead of in-band delimitation"; num.rs:16 "the values it cannot"; ranked.rs:284 "The key's rank component must be the rank the version measures (the two are computable independently, so the wire pins their agreement)"; ranked.rs:46 "group by rank alone". "door": one definition by contrast in lib.rs, or "public entry point" at the five partition sites, per the crate-level decision. Acceptance: `grep -n -i 'honest\|\bdoor' ` over the four files returns nothing, or `door` is defined in lib.rs.

### rank-14: suanpan's private digit width is hardcoded as 32 in before
- Where: crates/before/src/version/rank.rs:596-599 (related: rank.rs:1020-1022, crates/suanpan/src/accumulator.rs:28-29, crates/suanpan/src/lib.rs:225-227)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn DIGIT_BITS crates/suanpan/src` shows only the private `const DIGIT_BITS: u32 = 32;` at accumulator.rs:29; suanpan's public crate doc at lib.rs:226-227 states memory as "O(shift / 32)"); executed: no
- Seen by: prose, structure; refutation: confirmed; history: no rationale found (79a944ab; the drift risk is bounded because suanpan documents the base publicly)
- Owner-gated: yes (a suanpan API addition)

`accumulate` reserves `widest / 32 + 2` digits and `sum_ranks`' comment reasons about `shift / 32`, where 32 is a constant suanpan does not export. Named constants over magic numbers; a cross-crate coupling to a private constant is invisible state. Cost today is legibility only, since suanpan's public docs pin the base at `2^32` and its touch counts are declared exact.

Evidence:

       596	    let widest = aligned_bits(lhs).max(aligned_bits(rhs)).saturating_add(1);
       597	    if let Ok(digits) = usize::try_from(widest / 32 + 2) {
       598	        acc.reserve_digits(digits);
       599	    }

    (suanpan accumulator.rs)
        28	/// Bits per digit: the digit base is `2^32`.
        29	const DIGIT_BITS: u32 = 32;

Resolution: In suanpan, export the width (`Accumulator::DIGIT_BITS`) or add `reserve_bits(&mut self, bits: u64)` that does the division itself; use it at rank.rs:597 and cite the name at 1021-1022. Failing that, a named constant in rank.rs whose comment says it mirrors suanpan's documented digit base. Acceptance: `grep -n '/ 32' crates/before/src/version/rank.rs` returns nothing.

### rank-15: Em-dashes in // comments (19 lines); none in assert or expect messages
- Where: crates/before/src/version/rank.rs:621 (related: rank.rs:634, 771, 786, 795, 885, 891, 917, 918, 943, 944, 1024, 1025, 1029; crates/before/src/version/rank/num.rs:209; crates/before/src/version/ranked.rs:335, 337, 351, 354)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n '—'` filtered to lines without `///` or `//!` lists exactly these 19; a second grep over `assert!`/`expect(`/`unreachable!` messages returned nothing); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (crate-wide house style: 375 such lines across crates/before/src; no repo rule; the rule is the owner's global doctrine)
- Owner-gated: no

The doctrine prefers colons or semicolons over em-dashes in comments; the partition's `//` comments use true em-dashes on nineteen lines. Crate-wide, so a partition-local fix makes these files the inconsistent ones; this wants the same crate-level ruling as rank-13.

Evidence:

       621	    // exponent — both numerator arms clamp a shift past their width — so

Resolution: Replace with colons, semicolons, or parentheses at the listed lines, as part of a crate-wide pass. Acceptance: the filtered grep returns nothing.

### rank-16: The encode path clones the whole numerator, and plus_one's at-ceiling arm keeps the base value alive across from_limbs
- Where: crates/before/src/version/rank.rs:627 (related: crates/before/src/version/rank/num.rs:232-248, num.rs:372-400, crates/before/src/codec/base.rs:457-501, crates/before/wasm32-pins/guest/src/lib.rs:603-643)
- Class / severity / confidence: performance / low / high
- Provenance: verified (base.rs:457-501 shows only by-value `Shr` impls on `Base`; dashu-int 0.5.0 shift_ops.rs:107 has `impl Shr<usize> for &UBig`; `Wide::shr_limbs` at num.rs:571 already borrows; the wasm32 guest's `pin_rank_integral_decode` decodes, compares, and clones but never encodes, and `pin_rank_roundtrip` re-encodes a fraction-heavy rank); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found; constraint from cfa7c7ed: the shift's width-scale limb record feeds the board's `rank_encode` limb floor, so a borrowing shift must still emit it
- Owner-gated: no

`encode_parts` takes the integral part as `num.clone().shr(exp).plus_one()`: one O(bits(num)) copy of the whole numerator that a borrowing shift would avoid on both arms. In `plus_one`'s at-ceiling arm the moved-in `base` stays alive for the whole `Num::from_limbs(increment(Limbs::new(&base.0).collect()))` expression, during which `from_limbs` builds a byte image and then the `UBig`, so the original, the clone, the limb vector, and the byte image coexist at exactly the 32-bit seam where `from_limbs`' own comment (378-383) says one extra live copy is decisive. Both deletions have fixed sign (strictly less work and memory, same value); no wasm32 pin encodes an at-capacity integral rank.

Evidence:

       627	    let biased = num.clone().shr(exp).plus_one();

    (num.rs)
       235	            Num::Base(base) => {
       236	                // At the ceiling exactly: the backend may not survive the
       237	                // carry, so the increment runs in limb space and
       238	                // re-dispatches (an all-ones value grows one bit, past the
       239	                // ceiling; anything else stays base).
       240	                meter_wide(base.bits().div_ceil(64).max(1));
       241	                Num::from_limbs(increment(Limbs::new(&base.0).collect()))
       242	            }

       378	            // Exact-capacity byte image, and the limb vector dropped
       379	            // before the backend materializes: at the seam's widest
       380	            // crossings (a borrow falling back from one bit past a 32-bit
       381	            // target's capacity) the value is ~512 MiB, so an amortized
       382	            // growth double or one extra live copy is the difference
       383	            // between fitting the address space and an honest exhaustion.

Resolution: Add a metered `impl Shr<u64> for &Base` (keeping `meter_limbs1`, so the board's `rank_encode` limb floor still reads the shift) and a `Num::shr_ref(&self, n)` reusing `Wide::shr_limbs`, so `encode_parts` reads `num.shr_ref(exp).plus_one()`; in `plus_one`'s at-ceiling arm bind `let limbs: Vec<u64> = Limbs::new(&base.0).collect(); drop(base);` before `Num::from_limbs(increment(limbs))`. Add a wasm32 pin `pin_rank_integral_roundtrip(k)` at `k = 2^32 - 32` (decode, re-encode, byte-equal); if the harness's memory cap makes it terminal today, commit it in the `*_memory_terminal_traps` style and flip it after the deletions. Acceptance: the `rank_encode` heap readings drop (re-pinned with attribution at the parent); the `rank_encode` limb floor still reads; the new wasm32 pin passes; `rank_wide_arm_codec_roundtrips_canonically` and num/tests.rs unchanged.

### rank-17: Idiom nits: ilog2, a redundant rename, an expect that is not a proof, qualified paths, asymmetric twins, a re-export rename
- Where: crates/before/src/version/rank.rs:629 (related: rank.rs:788, rank.rs:838-843, rank.rs:1019, rank.rs:147, crates/before/src/version/rank/num.rs:505-506, num.rs:514-515, num.rs:259-271, num.rs:636-647, crates/before/src/version/ranked.rs:320-321, ranked.rs:342, ranked.rs:366-367, ranked.rs:385-389)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (each cited line read; rank.rs:134 imports `core::fmt::{self, Debug, Display}` while num.rs and ranked.rs spell `core::fmt::Display`/`core::hash::Hash` inline; `w = biased.bits()` is at least 1 because `biased` is at least 1); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: no rationale found (`expect("output fits")` dates from f0f3a2ae when `exp` was `u32`; 6323d667 widened it to `u64` without revisiting)
- Owner-gated: no

(1) `u64::from(63 - w.leading_zeros())` is `u64::from(w.ilog2())`. (2) `let exp = frac_len;` renames a value that could be bound as `exp` in the match at 773. (3) `.expect("output fits")` on a capacity hint states a conclusion, not the premise (exp is bounded by resident bits, so the 9/8 output width fits `usize` wherever the rank did); since it is only a reservation, `unwrap_or(0)` removes the panic path and is the more benign spelling at a tolerated corner. (4) `core::borrow::Borrow`, `core::fmt::Display`, `core::hash::Hash`, `crate::codec::canonical_eq` spelled inline where imports exist or belong. (5) `increment` is a free fn used twice while `minus_one` inlines its borrow loop; a `decrement` twin reads symmetric. (6) `PartialOrd for Ranked` calls `total_cmp` directly where `Some(self.cmp(other))` mirrors rank.rs:909-913. (7) `pub(crate) use num::{ceiling as arm_ceiling, ...}` gives one module two names.

Evidence:

       629	    let rho = u64::from(63 - w.leading_zeros());

       840	            bytes: Vec::with_capacity(usize::try_from(bits.div_ceil(8)).expect("output fits")),

      1019	fn sum_ranks<T: core::borrow::Borrow<Rank>, I: Iterator<Item = T>>(iter: I) -> Rank {

    (num.rs)
       505	impl core::fmt::Display for Num {

    (ranked.rs)
       342	    if crate::codec::canonical_eq(a.version.view(), b.version.view()) {

Resolution: Apply as listed; for (3) either `unwrap_or(0)` with the exact-length comment kept as documentation of the hint, or an expect whose message states the premise. Acceptance: clippy clean; the `RANK_TRIPLE` laws and rank envelopes unchanged; no `core::`/`crate::` path at a use site in the four files beyond the `<Self as Display>::fmt` disambiguation rank.rs needs.

### rank-18: Four byte-source adaptors wrap decode_stream, and two decodes copy their whole input first
- Where: crates/before/src/version/rank.rs:670-682 (related: rank.rs:461-465, rank.rs:684-716, crates/before/src/version/ranked.rs:271-290, crates/before/src/borsh_impls.rs:211-220, borsh_impls.rs:240-254, crates/before/src/version.rs:33)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (grep lists the adaptors at rank.rs:675, ranked.rs:276-280, borsh_impls.rs:213-217 and 242-246, plus a test use at borsh_impls/tests.rs:501; the second name `decode_rank_stream` at version.rs:33; `read_to_end` at rank.rs:462-463 and ranked.rs:272-273); executed: no
- Seen by: structure; refutation: confirmed (with the borsh genre change flagged); history: no rationale found for the `FnMut` seam (58c85989); the `read_to_end` shape is deliberate precedent (the family's whole-input strict decode)
- Owner-gated: yes (the borsh transport's error genre on a short read would change from `Decode::Io(UnexpectedEof)` to `Decode::Truncated` unless mapped deliberately)

`decode_stream` takes `FnMut() -> Result<u8, Decode>` and every caller writes its own adaptor: an iterator closure, an index closure, and two identical `read_exact` closures. `Rank::decode` and `Ranked::decode` additionally `read_to_end` into a `Vec` before parsing, so `Rank::decode(&key[..])` copies the key to decode it. The stream is self-delimiting by design (the module doc's point at 107-113), so a `Read`-based parser is the natural spelling; the byte-source generic is machinery serving only the adaptors. Steelman: the closure form keeps `decode_stream` io-agnostic and the cost is small.

Evidence:

       673	fn decode_bytes(bytes: &[u8]) -> Result<Rank, Decode> {
       674	    let mut iter = bytes.iter();
       675	    let rank = decode_stream(|| iter.next().copied().ok_or(Decode::Truncated))?;
       676	    if iter.next().is_some() {
       677	        // `decode` handed over the whole input, so bytes past the
       678	        // self-delimited stream are non-minimal packing.
       679	        return Err(Decode::TrailingBits);
       680	    }
       681	    Ok(rank)
       682	}

    (borsh_impls.rs)
       213	        decode_rank_stream(|| {
       214	            let mut byte = [0];
       215	            reader.read_exact(&mut byte).map_err(Decode::Io)?;
       216	            Ok(byte[0])
       217	        })

Resolution: `pub(crate) fn decode_stream<R: Read>(reader: &mut R) -> Result<Rank, Decode>` with `BitSource<R>` reading one byte via `read_exact`; `Rank::decode` becomes `decode_stream` then a one-byte EOF probe (a byte read is `TrailingBits`); `Ranked::decode` becomes `decode_stream` then `Version::decode(reader)`; the two borsh impls call it directly; delete `decode_bytes` and the `decode_rank_stream` alias. The owner rules how a short read maps (`Truncated` is arguably the correct genre; `Io` preserves today's borsh behavior). Acceptance: one `decode_stream` definition and name, no closure adaptors; `rank_decoding_rejects_each_genre`, `rank_encoding_exhaustive_small_scope` (both rejection genres still fire), `ranked_decode_rejects_each_genre`, and the borsh round-trip suites pass; `Rank::decode` on a slice performs no `read_to_end` allocation.

### rank-19: Public rustdoc names internals and privately defined terms
- Where: crates/before/src/version/rank.rs:879 (related: rank.rs:884, rank.rs:1064-1069, crates/before/src/version/ranked.rs:243, ranked.rs:376)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read; "magnitude class" is defined only in the `//` comment at 884, invisible to rustdoc readers of 879; "genre" has no definition in error.rs per the history pass's grep); executed: no
- Seen by: prose; refutation: confirmed ("schoolbook long division" dropped as an established term); history: no rationale found (879 and 376 are owner-authored, 2efff149 and b5a81583)
- Owner-gated: no

Public `# Complexity` and `# Errors` sections use maintainer vocabulary: "magnitude classes" (defined in a private comment), "the big-integer backend's capacity", "One fused signed rank co-sweep", "Each component's own genres". AGENTS.md: public rustdoc must not refer to functionality invisible to someone not reading the source. The user-relevant facts are plain: comparison is O(1) when the two ranks' `floor(log2)` differ; decimal rendering is superlinear and, on 32-bit targets past ~2^32-bit numerators, quadratic; `Ranked` comparison is one linear walk with no `Rank` built; the error variants are those of `Rank::decode` and `Version::decode`.

Evidence:

       879	/// Unequal magnitude classes settle in `O(1)`:

      1064	/// Superlinear, subquadratic in the rank's width: decimal conversion. (A
      1065	/// numerator wider than the big-integer backend's capacity — reachable
      1066	/// only on 32-bit targets, from hundreds of megabytes of decoded input —
      1067	/// renders by schoolbook long division instead, quadratic in the width:

    (ranked.rs)
       243	    /// Each component's own genres ([`Rank::decode`]'s and
      ...
       376	/// One fused signed rank co-sweep over the two viewed versions:

Resolution: 879 "Ranks whose integer parts of log2 differ settle in O(1):"; 1064-1069 "Superlinear, subquadratic in the rank's width (decimal conversion); on 32-bit targets, numerators above ~2^32 bits, reachable only from hundreds of megabytes of decoded input, render quadratically."; ranked.rs:376 "One walk over both versions, no Rank materialized:" (see rank-33 for the bound itself); ranked.rs:243 "Each component's own variants". Acceptance: no public doc in the partition uses "backend", "magnitude class", "co-sweep", "signed", or "genre" without defining it in the same public doc.

### rank-20: sum_ranks' amortization claim is false for ascending exponent order, and both committed pins fix the benign order while naming it the adversary
- Where: crates/before/src/version/rank.rs:1011-1018 (related: rank.rs:994-1006, rank.rs:1032-1049, crates/suanpan/src/accumulator.rs:592-595, accumulator.rs:626-627, crates/before/tests/meter.rs:1364-1396, crates/before/src/meter/board/ops.rs:509-559)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (suanpan's `shl` doc at accumulator.rs:594-595 states `O(|self|)` digit touches per call and its body at 626-627 is `core::mem::take(self)` then `add_accum_shl(&held, shift)`; rank.rs:1036-1039 calls it on every new exponent maximum; tests/meter.rs:1379-1388 and board/ops.rs:522-530 build the summands high-first); executed: no
- Seen by: claims; refutation: confirmed (no public cost sentence is falsified; the type doc's `O(‖a‖ + ‖b‖)` at rank.rs:211 covers `Add`, not `Sum`); history: no rationale found (ee894835 states the amortization and pins high-first as the adversary of the pre-cure per-element fold; no derivation and no consideration of ascending order anywhere)
- Owner-gated: no

The doc says a summand raising the maximum exponent rescales the accumulator "O(held digits) — paid by the exponent the summand itself carries". The rescale's cost is the held width, not the summand's exponent: a W-bit integer rank followed by 1/2, 1/4, ..., 1/2^n in ascending order pays n rescales of about W/32 digit touches each (plus n buffer rebuilds) against W + n(n+1)/2 content bits, so for W much larger than n^2 the touches per content bit grow with n. Both committed pins sum high-first and document that order as the worst case, which was true of the per-element-normalizing fold ee894835 replaced and is not true of the fold in the tree; a regression on the ascending order would pass every committed check. The `impl Sum` blocks carry no `# Complexity` at all. Instruments before cures: the failing row lands first. A linear algorithm exists (geometric headroom, below), so the bound the doc claims is achievable.

Evidence:

      1011	/// The accumulator holds the running numerator at the largest exponent seen so
      1012	/// far: a summand at a smaller exponent is digit-routed in at the exponent gap
      1013	/// (O(its own limbs), independent of the gap), and a summand raising the
      1014	/// maximum rescales the accumulator once, O(held digits) — paid by the exponent
      1015	/// the summand itself carries. Nothing renormalizes per element, so a
      1016	/// high-exponent summand costs its own width once instead of once per later
      1017	/// element, and the result is the identical [`Rank`] the pairwise fold produces
      1018	/// (one exact value, one shared normalization at the end).

      1036	        if rank.exp > exp {
      1037	            acc.shl(rank.exp - exp);
      1038	            exp = rank.exp;
      1039	        }

    (suanpan accumulator.rs)
       594	    /// `O(|self|)` digit touches, independent of the shift; the digit
       595	    /// buffer covers the shifted positions.
      ...
       626	        let held = core::mem::take(self);
       627	        self.add_accum_shl(&held, shift);

    (tests/meter.rs)
      1372	/// High-first ordering was the adversarial arm of the fold's
      1373	/// order-dependence (`Sum` accepts arbitrary order, so the worst order is
      1374	/// the honest pin); under the raw accumulator it is the order that makes
      1375	/// every later add a shifted word, which is why the pin stays the
      1376	/// scenario of record.

    (board/ops.rs)
       516	                // both sides of the value content scale together. High-first
       517	                // is the committed adversarial order: `Sum` accepts arbitrary
       518	                // order, and under a fold that re-normalizes per element it
       519	                // is the order that makes every later add a full-width
       520	                // operation. The denominator is the summands' total value

Resolution: Instrument first: a two-scale touch row (W fixed at, say, a 2^20-bit counter rank; n in {64, 128} spine ranks 1/2^k summed in ascending k) asserting touches per content bit flat across the two scales; it reads red today (touches double with n while content barely moves). Then cure with geometric headroom: when a summand exceeds the held exponent by `gap`, shift by `max(gap, held_bits)` and carry the surplus as exponent headroom, so the held width at least doubles per rescale and total rescale touches telescope to O(final held width); `from_num`'s `trailing_zeros`/`shr` already strip the headroom at the end (every summand entered at a shift at least the headroom). Rewrite 1011-1018 to the true bound, add `# Complexity` to both `impl Sum` blocks, and correct the two pins' adversarial-order prose. Acceptance: the new row is committed red then green with touches at most c · Σ‖r_i‖ at both scales; `RANK_SUM_MIXED` unchanged or re-pinned with attribution; `rank_sum_equals_the_pairwise_fold` and `rank_cross_path_normalization` still hold.

Construction: `let wide = Rank of a single-leaf version whose counter is 2^(2^20) - 1` (num.bits() about 2^20, exp 0); `let steps: Vec<Rank> = (1..=n).map(|k| spine(k).rank()).collect()` using version/tests.rs's `spine` helper (rank 1/2^k); reset the touch meter; `once(wide).chain(steps).sum::<Rank>()`. Each step raises `exp` by one and triggers `acc.shl(1)` over about 2^20/32 = 32768 held digits: n = 64 gives about 2.1M touches, n = 128 about 4.2M, while content bits move from about 1.05M to 1.06M. The reverse order (steps descending, then `wide`) stays near n + W/32.

### rank-21: Display honors formatter flags only for integral ranks
- Where: crates/before/src/version/rank.rs:1083-1087 (related: crates/before/src/version/rank/num.rs:505-512, crates/before/src/codec/base.rs:297-301)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (the `exp == 0` arm hands `f` to `Num`'s `Display`, which for the base arm delegates to `UBig`'s and for the wide arm uses `f.pad_integral`; the `exp >= 1` arms use `write!(f, "{}/2...", ...)`, whose inner placeholders take fresh default formatters); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (the `write!` arms date to 18206f21; 79a944ab added flag handling to `Num` alone; no test uses width or fill)
- Owner-gated: yes (which contract to fix to)

`format!("{:>6}", five)` pads while `format!("{:>6}", half)` does not; the doc ("Renders as the exact rational") states neither contract. A caller aligning a column of ranks will hit the inconsistency.

Evidence:

      1083	        match self.exp {
      1084	            0 => Display::fmt(&self.num, f),
      1085	            1 => write!(f, "{}/2", self.num),
      1086	            exp => write!(f, "{}/2^{}", self.num, exp),
      1087	        }

Resolution: Render all three forms to a `String` and route through one `f.pad_integral(true, "", &text)` (or `f.pad`), or state in the `Display` doc that formatter flags are ignored. Acceptance: a unit test formatting `{:>6}` over `ZERO`, an integral rank, and a fractional rank asserts the same padding behavior for all three, or the doc states the contract.

### rank-22: The two-arm numerator carries a maintenance cascade a single limbs arm would not
- Where: crates/before/src/version/rank/num.rs:4-48 (related: num.rs:86-139, num.rs:402-417, crates/before/src/version/rank.rs:146-147, rank.rs:537-571, rank.rs:314-322, rank.rs:949-957, crates/before/src/codec/base.rs:97-146, crates/before/src/version/tests.rs:1347-1665, crates/before/src/version/skyline/query/integral.rs:1142-1163, crates/before/tests/meter.rs:1197-1198)
- Class / severity / confidence: simplification / medium / medium
- Provenance: assessed (read: the override module at num.rs:113-139, the `#[cfg(test)]` read inside the production routing predicate at 93-99, the dispatch invariant every constructor re-establishes at 372-400 and 409-417, the rank-only `Base` shims at base.rs:97-146 whose callers are rank.rs:807 and num.rs:287, 299, 337 only, the ~320-line wide-regime suite at version/tests.rs:1347-1665, and `Integrator::finish` reading out via `sign_magnitude` at integral.rs:1162 on an `Accumulator` that also offers `sign_limbs`); executed: no
- Seen by: structure, correctness (as an open question); refutation: confirmed as a coherent design proposal, not a defect; history: deliberate and holds (79a944ab and num.rs:4-19 record the rationale: keep the backend as the arithmetic engine wherever it can represent the value, and keep base-arm limb records byte-identical); caveat from cfa7c7ed: base-arm limb records are load-bearing for the board's `rank_encode` limb floor (a liveness floor), so any redesign must keep width records
- Owner-gated: yes (a documented design decision)

Totality past the backend's 32-bit cap is mandatory, so some mechanism must exist; the question is its shape. The two-arm form carries: a canonical-dispatch invariant every constructor re-establishes, a test-only thread-local read inside the production routing predicate, a routing predicate with dual code paths in `Add` and `checked_sub`, rank-only shims on `Base` (`msb_cmp`, `msb_windows`, `to_be_bytes`, `from_be_bytes`, `to_bytes_le`), cross-arm `PartialEq`/`Hash` metering conventions, and a wide-regime copy of the rank test suite whose only job is to drive the seam. The module doc's own justification for the base arm's shape ("meters exactly as it did when Base was the numerator's only storage") is a stabilization convention: code shaped so pinned readings do not move (Principle 3). The wide arm already implements every operation `Rank` needs on a `Vec<u64>`, and the rank fold's readout can be `sign_limbs()` instead of `sign_magnitude()`. Steelman for the current form: dashu keeps values up to two words inline (small ranks never allocate and run at dashu's speed), `Display` below the ceiling uses dashu's subquadratic radix conversion, and the existing limb envelopes stay byte-identical.

Evidence:

        14	//! this module. [`Num`] closes that gap: the [`Base`] arm keeps the
        15	//! backend as the arithmetic engine of record everywhere it can represent
        16	//! the value, and the [`Wide`] arm stores the sliver it cannot — bounded
        17	//! only by memory — implementing exactly the operation set rank arithmetic
        18	//! needs (byte assembly, bit reads, right shifts, ±1, MSB-window
        19	//! comparison, and limb streaming into the accumulator).
      ...
        42	//! stores the value. Base-arm operations delegate to [`Base`]'s already
        43	//! metered methods, so a value below the ceiling meters exactly as it did
        44	//! when [`Base`] was the numerator's only storage. Work routed through the

        93	pub(crate) fn arm_ceiling_bits() -> u64 {
        94	    #[cfg(test)]
        95	    if let Some(bits) = ceiling::override_bits() {
        96	        return bits;
        97	    }
        98	    BACKEND_CAPACITY_BITS
        99	}

Resolution: Owner decision. If pursued: `Num` becomes a limbs-only struct (`SmallVec<[u64; 2]>` if allocation-free small ranks are wanted, else `Vec<u64>`) whose methods are the current `Wide` impl plus width-scale limb records; `Add`/`checked_sub`/`Sum` all go through `accumulate` (delete `backend_alignment_fits` and both backend routes); `Display` converts to a transient `UBig` via `UBig::from_words` when the width fits the backend and falls back to the existing long division above it; the fold reads out `sign_limbs()`; delete `ceiling`, `arm_ceiling_bits`, `from_base`, `is_wide`, `numerator_is_wide`, the rank-only `Base` shims, and fold the wide-regime suites into the base suites (their generators already produce every width). Re-pin `RANK_PAIR_MISMATCH` and `RANK_SUM_MIXED` at the parent commit first (the limb column moves to the touch column). The wasm32 pins stay unchanged: they assert values. Acceptance: num.rs is the limbs implementation plus `Display`/`Hash`/`Eq`; rank.rs has one arithmetic route; `grep -rn 'arm_ceiling\|is_wide\|numerator_is_wide\|from_base\|backend_alignment_fits' crates/before/src` is empty; num/tests.rs's differential suites pass with the ceiling parameter removed; all `RANK_TRIPLE` laws, the alignment sweeps, `rank_encoding_exhaustive_small_scope`, the board's `rank_encode` limb floor, and the wasm32 rank pins pass unchanged; the two re-pinned envelopes carry the movement's attribution.

### rank-23: The module doc attributes arm placement to pins that assert values only
- Where: crates/before/src/version/rank/num.rs:28-34 (related: crates/before/wasm32-pins/harness/tests/pins.rs:165-171, crates/before/wasm32-pins/guest/src/lib.rs:603-643)
- Class / severity / confidence: claim / nit / medium
- Provenance: verified (`grep -rn 'is_wide\|numerator_is_wide' crates/before/wasm32-pins/` returns nothing; pins.rs:167-170 asserts `Outcome::Value(0)`; the guest entries decode, compare, clone, and re-encode); executed: no
- Seen by: history pass (cross-cutting observation 8); refutation: not examined; history: no rationale found
- Owner-gated: no

The parenthetical states which arm each pin decodes on; no pin observes the arm. What the pins do hold is the load-bearing direction the constant's own doc names (num.rs:77-82): a past-capacity decode that misrouted to the `Base` arm would panic inside the backend, so the past-capacity pins' success bounds the ceiling from above. The below/at-capacity pins would pass on either arm. The sentence should claim the side the pins hold.

Evidence:

        28	//! routing: both arms denote the same integers exactly, so a misplaced
        29	//! ceiling could misroute cost, never value. The production ceiling is the
        30	//! backend capacity itself, held to the real backend by the wasm32
        31	//! boundary pins (the below/at-capacity decode pins fill the backend's
        32	//! last word on the [`Base`] arm; the past-capacity pins decode on the
        33	//! [`Wide`] arm); tests may lower it (the test-only `ceiling` module) so
        34	//! every public door drives both arms and the seam between them at

    (pins.rs)
       167	    assert_eq!(
       168	        call1("pin_rank_decode", (1u64 << 32) - 32),
       169	        Outcome::Value(0),
       170	    );

Resolution: "held to the real backend from above by the wasm32 boundary pins: a decode one fraction group past the capacity succeeds, which it could not if the ceiling routed that width to the backend (the pins assert values, not arms; the lower side is not load-bearing)". Acceptance: the sentence claims only what a pin observes.

### rank-24: "historical" as dated rationale at five declaration sites, once explaining a metered no-op shift whose live rationale exists only in git
- Where: crates/before/src/version/rank/num.rs:206-222 (related: num.rs:42-44, num.rs:281, num.rs:319-320, num.rs:405-407, crates/before/src/version/rank.rs:946-947, crates/before/src/version/tests.rs:1393)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n historical` over the partition and version/tests.rs: rank.rs:946; num.rs:210, 281, 319, 406; tests.rs:1393; num.rs:42-44 is the same rationale without the word); executed: no
- Seen by: structure, prose, claims; refutation: confirmed; history: deliberate and holds for the mechanism, dated for the prose (cfa7c7ed restored the whole `(num >> exp) + 1` spelling because a width-guard early exit skipped the shift's width-scale limb record and read the board's `rank_encode` limb floor from below; the floor is a liveness floor asserting the encode walk reads every limb)
- Owner-gated: no

A reader who never saw the single-arm numerator gets no information from "historical" (Principle 5: dated rationale at a declaration site is a ghost reference; provenance lives in git). At 208-212 the word also carries a real, live rationale that is stated nowhere in the tree: the shift-by-zero is executed so the shift's limb record is emitted, because the board's `rank_encode` limb floor asserts that the encode walk reads the numerator and the record rides the shift. That converts to "state the rationale at the site"; the option of adding an early return and re-pinning would re-open the verdict cfa7c7ed cured.

Evidence:

       208	            // The base arm can only shrink, so it stays canonical with no
       209	            // re-dispatch — including the shift-by-zero spelling, which
       210	            // keeps this arm's cost and metering exactly the historical
       211	            // numerator path's.
       212	            Num::Base(base) => Num::Base(base >> n),

    (rank.rs)
       946	        // historical cost, and a 32-bit target's wide sums (a gap at or

    (cfa7c7ed, commit message)
    one that skipped the shift's width-scale limb record, reading the
    amp board's rank_encode limb floors from below (the floor was right:
    the walk genuinely reads the numerator; the record rides the shift).

Resolution: Restate each site positively and undated: num.rs:208-211 "The base arm can only shrink, so it stays canonical without re-dispatch. The shift runs even at zero so its width-scale limb record is emitted: the board's rank_encode limb floor asserts that the encode walk reads every limb of the numerator, and that record rides the shift."; num.rs:42-44 "Base-arm operations are Base's own metered methods"; num.rs:281 see rank-25; num.rs:319-320 "Below the ceiling this is Base::from_be_bytes then the metered sub-byte shift"; num.rs:405-407 "so the conversion never fires and every Base numerator passes through unchanged, unmetered"; rank.rs:946-947 "at Base's shift-and-add cost"; version/tests.rs:1393 likewise. Acceptance: `grep -n historical` over the partition and version/tests.rs is empty, and the shift-by-zero's rationale is stated at num.rs:206-212.

### rank-25: Num::msb_cmp's (Base, Base) arm is a no-op distinction and Base::msb_cmp is a one-caller wrapper
- Where: crates/before/src/version/rank/num.rs:277-292 (related: crates/before/src/codec/base.rs:97-112)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (base.rs:104-106 is exactly `msb_cmp_windows(a.msb_windows(), b.msb_windows())`; `grep -rn msb_cmp crates/before/src` shows `Base::msb_cmp`'s only caller is num.rs:287); executed: no
- Seen by: structure, claims; refutation: confirmed; history: no rationale found (79a944ab extracted the kernel from `Base::msb_cmp` and kept the wrapper without saying why)
- Owner-gated: no

The doc tells the reader two paths exist ("the historical path, metering included" versus "the same shared kernel") where all four arms are the identical expression with identical metering; a reader must open base.rs to learn there is no difference.

Evidence:

       281	    /// Same-arm base pairs take [`Base::msb_cmp`] — the historical path,
       282	    /// metering included; every other pairing streams both arms' windows
       283	    /// through the same shared kernel, so the tail rule and the per-window
       284	    /// metering are one implementation across arms.
       285	    pub(crate) fn msb_cmp(a: &Num, b: &Num) -> Ordering {
       286	        match (a, b) {
       287	            (Num::Base(x), Num::Base(y)) => Base::msb_cmp(x, y),
       288	            (Num::Base(x), Num::Wide(y)) => msb_cmp_windows(x.msb_windows(), y.msb_windows()),

    (base.rs)
       104	    pub(crate) fn msb_cmp(a: &Base, b: &Base) -> Ordering {
       105	        msb_cmp_windows(a.msb_windows(), b.msb_windows())
       106	    }

Resolution: Delete `Base::msb_cmp`; spell the four arms uniformly and rewrite the doc: "every pairing streams both arms' MSB windows through msb_cmp_windows; the arms differ only in their limb source". Optionally collapse to one arm with a small two-variant `Iterator` over the limb source so `Num::msb_windows` has one return type. Acceptance: `grep -rn 'Base::msb_cmp' crates/before/src` is empty; `msb_cmp_matches_the_aligned_oracle` and `rank_wide_arm_cmp_agrees_with_the_alignment_oracle_on_10k_pairs` pass unchanged; the limb-meter reading of `Rank::cmp` on any base pair is unchanged.

### rank-26: Wide-arm limb metering has no observer: sixteen meter_wide hooks feed a counter no committed check reads while the arm is engaged
- Where: crates/before/src/version/rank/num.rs:319-324 (related: num.rs:37-48, num.rs:58-69, crates/before/src/codec/base/limb_meter.rs, .cargo/mutants.toml:80)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (`grep -rln 'ceiling::force\|arm_ceiling::force' crates/before/` returns only version/tests.rs and num/tests.rs; `grep -n 'limb_ops\|limb_meter'` on those two files returns nothing; the `limb_ops()` readers are meter.rs, tests/meter.rs, meter/tests.rs, board/*, query/tests.rs, testing/asymptotics.rs, and two integration tests, none of which forces the ceiling; the override module is `#[cfg(test)]` (num.rs:113), so integration tests cannot reach it; the wasm32 pins contain no `limb_ops` read); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (79a944ab extends the meter convention to the wide arm but pins no floor; .cargo/mutants.toml carries no num.rs entry)
- Owner-gated: no

`meter_wide` records wide-arm work at sixteen sites, justified by a liveness principle, but the wide arm engages in production only on 32-bit targets (no envelope suite runs there) and on the host only under the test ceiling, which only suites that never read the limb meter use. A wide decode recording nothing would fail no test; deleting every `meter_wide` call would fail no test. Principle 3: an instrument hook earns its place by naming a committed check that reads it; a meter needs a liveness floor or it is decoration.

Evidence:

       319	    /// alignment shift. Below the ceiling this is exactly the historical
       320	    /// spelling ([`Base::from_be_bytes`] then the metered sub-byte shift);
       321	    /// above it the limbs are assembled directly — no backend value ever
       322	    /// exists — with the materialization metered on the value's width, the
       323	    /// wide-decode convention: a meter that missed it would let a decoder
       324	    /// build arbitrarily wide values while recording nothing.

        64	fn meter_wide(limbs: u64) {
        65	    #[cfg(feature = "limb-meter")]
        66	    crate::codec::base::limb_meter::record(limbs);

Resolution: Either make the hooks live: one `#[cfg(all(test, feature = "limb-meter"))]` unit pin under `ceiling::force(TEST_CEILING_BITS)` that resets the meter, decodes a stream whose numerator crosses the ceiling, and asserts `limb_ops() >= bits.div_ceil(64)` (the floor derived from irreducible work: every limb of the value is materialized), plus the same for one `+` on the accumulate route; or remove `meter_wide` and its call sites and state in the Metering section that wide-arm cost is priced by memory and suanpan's digit-touch meter. Acceptance: a committed test fails when any `meter_wide` call is deleted, or `meter_wide` is gone and the module doc's Metering section describes what remains.

Construction: Make `meter_wide` a no-op (or delete its sixteen calls) and run `just test-all`; nothing fails today. The proposed pin: `let _g = ceiling::force(96); limb_meter::reset(); let r = Rank::decode(&bytes_of_a_200_bit_rank[..]).unwrap(); assert!(limb_meter::limb_ops() >= 200u64.div_ceil(64));`.

### rank-27: Two sub-limb right-shift kernels in num.rs
- Where: crates/before/src/version/rank/num.rs:355-360 (related: num.rs:569-593, num.rs:297-311, num.rs:384-395, num.rs:449-462)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both loops read: 358 `limbs[i] = (limbs[i] >> pad) | (high << (64 - pad));` and 588 `self.limbs.get(i + 1).copied().unwrap_or(0) << (64 - bit)`); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (both in 79a944ab)
- Owner-gated: no

`materialize_be` shifts an assembled limb vector right by `pad` bits in place; `Wide::shr_limbs` shifts by `n % 64` while copying the tail. Both are `limbs[i] = (limbs[i] >> k) | (limbs[i+1] << (64 - k))` with the same top-limb handling: one cursor discipline written twice, so a fix to one (an off-by-one at the top limb, a `k == 0` guard) must be found in the other. Alongside: the limb-to-byte flatten-and-strip is spelled three times (302-307, 384-395, 453-460).

Evidence:

       355	        if pad > 0 {
       356	            for i in 0..limbs.len() {
       357	                let high = limbs.get(i + 1).copied().unwrap_or(0);
       358	                limbs[i] = (limbs[i] >> pad) | (high << (64 - pad));
       359	            }
       360	        }

       583	        for i in whole..self.limbs.len() {
       584	            let low = self.limbs[i] >> bit;
       585	            let high = if bit == 0 {
       586	                0
       587	            } else {
       588	                self.limbs.get(i + 1).copied().unwrap_or(0) << (64 - bit)
       589	            };
       590	            limbs.push(low | high);
       591	        }

Resolution: One `fn shr_in_place(limbs: &mut [u64], bits: u32)` documented for `1..64` (caller guards zero): `materialize_be` calls it when `pad > 0`; `shr_limbs` becomes `let mut tail = self.limbs[whole..].to_vec(); if bit != 0 { shr_in_place(&mut tail, bit) }; tail` and lets `from_limbs` trim the top. Optionally one `fn le_bytes_minimal(limbs: &[u64]) -> Vec<u8>` for the three flattenings. Acceptance: one occurrence of the `<< (64 - ` combine pattern in num.rs; `materialize_is_exact_and_canonical` (all pads) and `shr_matches_the_oracle_and_redispatches` (amounts 0..512) pass unchanged; the wasm32 fraction pins pass unchanged.

### rank-28: Wide's documented invariant is violated by from_limbs's transient, which is the only reason Wide::bits has a zero arm
- Where: crates/before/src/version/rank/num.rs:372-377 (related: num.rs:156-166, num.rs:526-537, num.rs:557-567, crates/before/src/version/rank/num/tests.rs:183)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (num/tests.rs:183 tests `Num::from_limbs(vec![])` equals `ZERO`, so the transient `Wide { limbs: vec![] }` is constructed; `Wide::bits` carries `None => 0` at 531 while `Wide::trailing_zeros` ends in `unreachable!` at 566 on the same invariant); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

The type doc says "the top limb is nonzero ... Never zero", yet `from_limbs` constructs `Wide { limbs }` before knowing the arm, including for empty input, and `Wide::bits` carries a zero case the doc says cannot happen; a reader must trace `from_limbs` to learn why.

Evidence:

       159	/// Invariants: the top limb is nonzero (minimal spelling — what makes the
       160	/// derived equality value equality), and the bit width exceeds the arm
       161	/// ceiling in force (canonical dispatch; [`Num`]'s constructors enforce
       162	/// it). Never zero.

       376	        let wide = Wide { limbs };
       377	        if wide.bits() <= arm_ceiling_bits() {

       530	        match self.limbs.last() {
       531	            None => 0,

Resolution: A free `fn limb_bits(limbs: &[u64]) -> u64` (top limb nonzero by the caller's stripping; empty is 0) used by `from_limbs` to decide the arm, constructing `Wide { limbs }` only in the wide branch; `Wide::bits` delegates with a nonempty assertion and drops the zero arm. Acceptance: `Wide::bits` has no zero arm; `from_limbs_normalizes_and_dispatches` and `materialize_is_exact_and_canonical` pass unchanged.

### rank-29: Accumulator readouts materialize the result four times (digits, limbs, LE bytes, UBig) on every Sum
- Where: crates/before/src/version/rank/num.rs:384-396 (related: crates/before/src/version/rank.rs:602-609, rank.rs:1042-1049, crates/suanpan/src/accumulator.rs:1027-1052, accumulator.rs:1505-1512)
- Class / severity / confidence: performance / low / medium
- Provenance: verified (suanpan's `sign_limbs` at accumulator.rs:1037-1041 reads digits then builds a limb `Vec`; `from_limbs` then builds a byte image and calls `UBig::from_le_bytes`; dashu-int 0.5.0 exposes `UBig::from_words(&[Word])` at ubig.rs:167, with `Word = u64` on 64-bit and `u32` on wasm32; suanpan's `sign_magnitude` also goes through bytes at 1505-1511, so it is not a shortcut); executed: no
- Seen by: claims; refutation: confirmed (and raised the count from three to four: `sign_limbs` itself materializes once); history: no rationale found (79a944ab's comment justifies dropping the limb vector before the backend materializes, not the byte detour)
- Owner-gated: no

On the path every `Sum` takes on every target, the result is materialized as digits, then limbs, then little-endian bytes, then a `UBig`. Fixed-sign constant-factor work on a hot readout; doctrine says construct and measure such deletions rather than argue. Denominator: result width in limbs; only the heap column moves.

Evidence:

       384	            let mut bytes: Vec<u8> = Vec::with_capacity(wide.limbs.len() * 8);
       385	            for limb in &wide.limbs {
       386	                bytes.extend_from_slice(&limb.to_le_bytes());
       387	            }
       388	            drop(wide);
      ...
       396	            return Num::Base(Base::from(UBig::from_le_bytes(&bytes)));

    (suanpan accumulator.rs)
      1037	        let (sign, digits) = self.read_digits(0);
      1038	        let mut limbs: Vec<u64> = digits
      1039	            .chunks(2)
      1040	            .map(|pair| u64::from(pair[0]) | (pair.get(1).copied().map_or(0, u64::from) << 32))
      1041	            .collect();

Resolution: In `from_limbs`' under-ceiling arm build the `UBig` with `UBig::from_words` from the limb vector on 64-bit (cfg on `dashu_int::Word::BITS`), splitting limbs into `u32` words on 32-bit; measure with `RANK_SUM_MIXED`'s peak-heap column and the board's `rank_sum` heap cell at the parent and after, re-pinning with attribution. Acceptance: the heap readings drop by roughly the result's byte width with no change to touch or limb readings; `from_limbs_normalizes_and_dispatches` and the wide-arm proptests unchanged.

### rank-30: num/tests.rs's testdoc claims hashing coverage the body lacks, and its module doc overstates "every wide-arm operation"
- Where: crates/before/src/version/rank/num/tests.rs:131-141 (related: num/tests.rs:1-2, crates/before/src/laws.rs:2936-2942, crates/before/src/version/tests.rs:1485-1490)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -n -i hash` on num/tests.rs returns only the doc line 131; no hasher is constructed anywhere in the file; `Num::to_be_bytes` on the wide arm, `from_base`, `fold_into`, and the `Hash` impl have no test in the file; wide-arm `Hash`/`Eq` coherence is exercised elsewhere: `rank_cross_path_normalization` at laws.rs:2942 asserts `hash_of(&via_add) == hash_of(&via_sum)` and `rank_wide_arm_triple_laws_on_seeded_ranks` runs `RANK_TRIPLE` under the lowered ceiling); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (79a944ab), with the coverage picture corrected as above
- Owner-gated: no

Doctrine: every test's doc comment states its invariant and its incorrectness is a bug in the test. The doc says hashing is checked; the body asserts only `na == nb` against the oracle and clone-reflexivity. The module doc tells a maintainer this file is the coverage of record for wide-arm mechanics when four operations are exercised only through version/tests.rs and laws.rs. Because the hash property is pinned in the law group, the fix is doc-side or a small hasher clause, not a coverage emergency.

Evidence:

         1	//! Unit suites for the numerator's two-arm storage: every wide-arm
         2	//! operation differentially against the backend as oracle.

       131	    /// Structural equality and hashing are value equality across the
       132	    /// dispatch: equal values are one arm and equal, unequal values are
       133	    /// unequal whatever their arms.
       134	    #[test]
       135	    fn equality_is_value_equality(a in arb_value(), b in arb_value()) {
       136	        let _guard = ceiling::force(TEST_CEILING_BITS);
       137	        let na = num_from_oracle(&a);
       138	        let nb = num_from_oracle(&b);
       139	        prop_assert_eq!(na == nb, a == b);
       140	        prop_assert_eq!(&na, &na.clone());
       141	    }

Resolution: Add a hash-coherence clause (`a == b` implies equal `DefaultHasher` outputs for `na` and `nb`) or drop "and hashing" from the doc and cite `rank_cross_path_normalization`. Narrow the module doc to the operations pinned here (dispatch, assembly, shifts, bias steps, windows, rendering, equality) and point at the version/tests.rs wide-regime suites and the `RANK_TRIPLE` law run for `to_be_bytes`, `from_base`, `fold_into`, and `Hash`; or add oracle tests for those four. Acceptance: every `pub(crate)` fn on `Num` and every trait impl with a `Wide` arm either has a test in this file whose doc matches its body, or is named in the module doc with the covering suite cited.

Construction: Under `ceiling::force(96)`, build two `Num`s of equal value, feed each to `std::hash::DefaultHasher`, and compare; nothing in this file does so today, so a `Hash` impl that hashed the arm tag would pass every test in the file (and be caught only by the law group).

### rank-31: The M-definition sentence is hand-copied six times in ranked.rs (eleven crate-wide)
- Where: crates/before/src/version/ranked.rs:101 (related: ranked.rs:148, 174, 202, 223, 253; crates/before/src/version.rs:293, 365, 412, 1058, 1081)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn 'unbounded-integer multiplication' crates/before/src crates/before/build.rs` lists exactly these eleven `///` lines and nothing emitted by build.rs); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed (the sentence's "about O(n log n)" is accurate as an asymptotic statement: dashu-int 0.5.0 gates NTT out only for 16-bit targets, arch/mod.rs:9 and mul/mod.rs:27, and wasm32 selects generic_32_bit, which ships its own ntt.rs; the "Toom-3 on wasm32" finding is dropped); history: no rationale found (b5a81583 added six copies; 2efff149's own message states a single-source principle for contract prose)
- Owner-gated: no

A notation defined at every use site is the hand-maintained-count failure in prose form: one wording change means eleven edits, and copies drift. The "Typical inputs run far below the worst case" clause carries no contract.

Evidence:

       101	    /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).

Resolution: Define `M` once in the crate docs' complexity notation (lib.rs already hosts the asymptotic-guarantee section) and have the fuelscape include emit a link to it, or a one-clause "M is unbounded-integer multiplication" with the link; drop the typical-inputs clause. Acceptance: `grep -rc 'unbounded-integer multiplication' crates/before/src` totals 1.

### rank-32: Ranked::encode_rank is documented as a fused, more efficient emission; it is Rank::encode by another name, and has been since the commit that introduced it
- Where: crates/before/src/version/ranked.rs:211-215 (related: ranked.rs:46-49, ranked.rs:189-196, crates/before/src/version/rank.rs:395-397, rank.rs:486-498, rank.rs:612-618, crates/before/src/version.rs:1049-1052, version.rs:1071-1075, crates/before-fuelscape/src/ops.rs:1312-1314, crates/before/src/version/tests.rs:1639-1641, crates/before/src/version/skyline/query.rs:201-225)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (both bodies read; `git show c8ea49f4:crates/before/src/version/ranked.rs` lines 171-175 show the same three-line body as `Ranked::encode` at the commit titled "fused encode", and 8519dd47 renamed it unchanged; `skyline::query::rank` returns a `Rank` via `Rank::from_raw` at query.rs:224, so there is no fold-side `(num, exp)` to hand off; `grep -rn 'encode_parts\|raw_parts' crates/before/src` shows `encode_parts`' only caller outside rank.rs is ranked.rs:214 and `raw_parts`' remaining callers are version/tests.rs:804, 1372 and meter/board/ops.rs:2277); executed: no
- Seen by: structure, prose, correctness, claims (four independent reports); refutation: confirmed; history: no rationale found (the private "hand-off" prose was written in c8ea49f4 and never matched the code; the public efficiency sentences were added later in 8d8a06e2 and c8fd70e6)
- Owner-gated: no

The body materializes the `Rank` (`self.version.rank()`), destructures it, and calls the same `encode_parts` that `Rank::encode` calls on the same normalized parts: identical work, byte for byte and cost for cost. Yet three public sentences claim efficiency or non-materialization, the `pub(crate)` visibility of `encode_parts` is justified by "the ranked view's fused emission", `raw_parts` is described as "The fused encode's hand-off", the fuelscape `size_measure` says "one fused rank fold and emission", and a test doc says "the fused emission from the fold's raw parts". Public rustdoc states a cost contract the code does not deliver (the crate makes cost claims guarantees), and the two crate-private entries justify their existence by a mechanism that does not exist: the circular-justification tell.

Evidence:

       211	    pub fn encode_rank(&self) -> Vec<u8> {
       212	        let rank = self.version.rank();
       213	        let (num, exp) = rank.raw_parts();
       214	        encode_parts(num, exp)
       215	    }

        48	/// choosing; [`encode_rank`](Self::encode_rank) emits exactly the corresponding
        49	/// [`Rank`]'s encoded bytes without materializing the intermediate [`Rank`].

       194	    /// - `v.ranked().encode_rank()`
       195	    /// - `v.rank().encode()` (this one is less efficient)

    (rank.rs)
       395	    pub fn encode(&self) -> Vec<u8> {
       396	        encode_parts(&self.num, self.exp)
       397	    }

       488	    /// The fused encode's hand-off from a rank fold's output to the canonical
       489	    /// emission ([`encode_parts`]), and the raw normalized form the reference

       615	/// `pub(crate)` alongside [`Rank::encode`] so the ranked view's fused emission
       616	/// can emit straight from its rank fold's `(numerator, exponent)` output, with
       617	/// no walk beyond the fold's own.

    (version.rs)
      1051	    /// Equivalent to `self.rank().encode()`, but more efficient. Exactly
      ...
      1073	    /// Equivalent to `self.rank().encode_to(writer)`, but more efficient.

    (before-fuelscape/src/ops.rs)
      1312	        size_measure: "packed bytes of the viewed version (the composite key's \
      1313	             rank component alone: one fused rank fold and emission; view \

Resolution: Make `Ranked::encode_rank` `self.version.rank().encode()` and `encode_rank_to` likewise; drop the `encode_parts` import from ranked.rs and make `encode_parts` private to rank.rs (delete its `pub(crate)` rationale); re-justify `raw_parts` by its remaining callers (the oracles' raw form and the board's limb denomination), or replace it with a `#[cfg(any(test, feature = "meter"))]` accessor beside `content_bits`. Rewrite ranked.rs:46-49 and 192-196 to "equivalent to `v.rank().encode()`" with no efficiency ranking; version.rs:1051 and 1073 drop "but more efficient" (keep "more succinct"); rank.rs:486-491 becomes "The stored parts, the raw normalized form the reference computations and the meter denominators read"; the fuelscape `size_measure` reads "one rank fold and its emission" (regenerate the JSON); version/tests.rs:1640-1641 drops the parenthetical. If a genuine fusion is wanted, note that constructing the `Rank` after normalization is free, so there is no cheaper fold-side path to build; the true statement is that `encode_rank` exists as a spelling convenience on a view. Acceptance: `grep -rn 'fused emission\|fused encode\|more efficient\|less efficient\|without materializing the intermediate' crates/before/src crates/before-fuelscape/src` returns nothing about `encode_rank`; `encode_parts` has no `pub(crate)`; `ranked_carries_own_rank` (laws.rs:388-389, `ranked.encode_rank() == a.rank().encode()`) still passes; the board cells `rank_encode` and `ranked_encode_rank` read identical limb and touch counts for the same version, as they must already.

Construction: Read the two bodies side by side (ranked.rs:211-215 and rank.rs:395-397): both reduce to `encode_parts(num, exp)` on the output of `skyline::query::rank`. For a mechanical witness under `--features limb-meter`: reset, call `v.rank().encode()`, read `limb_ops()`; reset, call `Ranked::from(&v).encode_rank()`, read again; the counts are equal for every `v`. Alternatively wrap `Rank::from_raw` in a thread-local call counter under test and assert each spelling increments it exactly once.

### rank-33: Ranked::cmp promises O(|self| + |other|) but runs the multiplication-bound rank settle
- Where: crates/before/src/version/ranked.rs:372-378 (related: ranked.rs:19-22, ranked.rs:328-347, crates/before-fuelscape/src/ops.rs:1339-1357, crates/before/fuelscape/ranked_cmp.json, crates/before/src/version/skyline/query.rs:271-290, query.rs:390-401, crates/before/src/version/skyline/query/integral.rs:1142-1163, integral.rs:205-229, crates/before/src/meter/board/ops.rs:667-688)
- Class / severity / confidence: claim / high / high
- Provenance: verified (query.rs:286-290 `rank_cmp` is `pair_fold(a, b, |_| 1).0`; `pair_fold` ends at query.rs:399 with `integral.finish(overlay_depth)`; `Integrator::finish` at integral.rs:1142-1143 runs `self.settle()`, the mass-balanced product tree the module doc prices at `O(M(|v|))` and, past the backend's power-law tiers, `O(M(|v|) · log |v|)` (integral.rs:213-229); the total is computed and discarded. The committed contract for `ranked_cmp` (ops.rs:1350, fuelscape/ranked_cmp.json) is `O(|self| + |other|)` with claim `n`, while `version_distance.json`, the same kernel with a different orientation, is `O(M(|self|) · log |self|)`. The `ranked_cmp` island's overlay roster is jump_pair, tooth_tail, concurrent_pair, dense × self, hugeleaf × self; `version_rank`'s includes wide_arming and plateau_puncture, the two families whose settle products fire, and neither appears on the comparison island); executed: no
- Seen by: claims; refutation: confirmed, with the roster corroboration added; history: no rationale found; a regression, not a decision: at 3bba6cbb the type doc stated "time `O(M(n) · log n)` worst case" and that "On answer-embedding pairs the shipped co-sweep provably pays the backend's multiplication cost"; c6d8106a wrote the linear roster string, b5a81583 deleted the type-doc paragraph because the roster became the single source, and 2efff149 re-denominated the string
- Owner-gated: no
- Witness (witness/results.md lines 148-257): demonstrated for the claim's core; the order stands by reading. At every scale `Ranked::cmp` does the rank fold's work, 0.77× `rank`'s limb operations (281/364, 561/726, 1113/1440) and about 1.08× its wall time in the same run, while the `partial_cmp` control does a fraction of either; the structural reading (`rank_cmp` = `pair_fold(a, b, |_| 1).0`, `integral.finish`, `self.settle()`) was confirmed at the cited lines. The deterministic limb meter read 0.989, linear, because meter.rs:3565-3566 prices a multiplication by its operand limbs, so no limb-metered row can pin the `M(n)` class. The wall exponents the construction also printed (1.14, 1.20, 1.28, 1.25 across four doublings, min of three in a dev build) carry no load disclosure on a machine the review records at load averages 3.8 to 19.9, so they are wall time under undisclosed load, not a measurement of the order.

The crate docs make every asymptotic claim a hard guarantee ("any violation is a bug"), and this public `# Complexity` states a class the implementation does not meet: the sign-only fold still settles the exact signed difference, whose value embeds an input-funded product, so its cost is the pair measures' bound with the promotion ledger and settle tree allocated, not a linear sweep. The only instruments pricing this entry are a uniform fuzz-fit band fitted on a roster that omits exactly the superlinear families the same kernel's other island exhibits, and a board cell that takes the distance/lag touch floors (an adequacy floor, not an order pin). The type doc's "cheaper than two rank folds and a compare" (19-22) is a constant-factor statement that remains true; the class claim is the defect.

Evidence:

       372	/// The total order: rank first, canonical bytes on rank ties.
       373	///
       374	/// # Complexity
       375	///
       376	/// One fused signed rank co-sweep over the two viewed versions:
       377	///
       378	#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_cmp.html"))]

    (query.rs)
       286	pub fn rank_cmp(a: BitsView<'_>, b: BitsView<'_>) -> Ordering {
       287	    // `∫ D`, signed: σ is constantly `+1`, the total is
       288	    // `rank(a) − rank(b)`, and only its sign is kept.
       289	    pair_fold(a, b, |_| 1).0
       290	}
      ...
       399	    let (sign, total) = integral.finish(overlay_depth);

    (integral.rs)
      1142	    pub(super) fn finish(mut self, closing_shift: u64) -> (Ordering, UBig) {
      1143	        self.settle();

    (before-fuelscape/src/ops.rs)
      1350	        contract: "`O(|self| + |other|)`",
      1351	        claim: "n",

    (3bba6cbb, ranked.rs)
        81	/// With `n = |a| + |b|`, the compared views' versions' total size in
        82	/// bytes: `O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)`
        83	/// with width-bounded parked drifts.

Resolution: Restore the contract at ops.rs:1350 to the pair measures' form (`O(M(|self| + |other|)) · log(|self| + |other|))` time, `O(|self| + |other|)` space, mirroring `version_distance`'s committed contract) and regenerate fuelscape/ranked_cmp.json through the compactor so the island and fuelscape-verify agree; add the plateau_puncture and wide_arming pair families to the `ranked_cmp` OpSpec so the fit itself confronts the superlinear shapes; rewrite ranked.rs:19-22 to what is true (one co-sweep instead of two folds and a compare, a constant, with the integrator's ledger and settle tree as its transient). If a cheaper order kernel is wanted, note that a sign-only fold cannot be linear on exact ties (the answer-embedded product must be resolved), so a domination-certificate early exit can only improve the non-tie case; the public worst case stays M-bound either way. Acceptance: the island text reads the M-bound contract and fuelscape-verify passes on the regenerated JSON; a committed two-scale meter row `ranked_cmp × PlateauPuncture` (`Shape::PlateauPuncture.packed2(w, d)` against `Version::new()`, at (w, d) and (2w, 2d)) whose limb and touch readings track `a.rank()`'s rather than a flat per-byte line.

Construction: `let a = Shape::PlateauPuncture.packed2(64, 48).version(); let b = Version::new();` reset the limb and touch meters; `Ranked::from(&a).cmp(&Ranked::from(&b));` read the counters and compare with `a.rank()`'s under the same meters: the co-sweep's settle performs the same product tree (`rank_cmp` discards `total` after computing it), so both readings share the multiplication term; doubling (w, d) grows that term at the backend's power-law exponent, not linearly in packed bytes.

## Positives

- The wire-form module doc (rank.rs:8-131) is a complete, checkable argument: the inverted-polarity delta header is shown bijective (so no rejection genre is needed for non-minimal headers), the fraction's in-band framing is justified against a length header by the concrete 1/2-versus-7/16 counterexample, prefix-freeness is derived from the close bit, and every rejected alternative (gamma/omega/varints, dsi-bitstream's codes, ordered-varint, the FoundationDB tuple form) is named with the mechanism that rules it out. The decoder's comments (722-787) mirror the argument clause for clause. All four lenses independently singled this out; I confirm it by reading.
- The decoder's allocation discipline is stated and kept: every allocation is fed by bits actually read (`BitSink::new` grows while the encoder's `with_capacity_bits` preallocates, 827-843; the fraction's depth is counted from consumed bits, 784-788), the numerator is assembled by byte concatenation rather than a value-width shift a 32-bit `usize` cannot hold (792-811), and the exhaustive 0-2 byte sweep asserts both rejection genres fire and acceptance is live (version/tests.rs:1087-1095), so strictness cannot pass vacuously.
- `Rank::cmp` (882-906) is obviously right and cheap: zero settled first (the one even-numerator form whose class would collide with `(0, 1]`), an O(1) class test on `bits(num) - exp`, then MSB-aligned streaming windows through one shared kernel with zero allocation and a tail rule whose premise (odd numerators) is stated at the site; the alignment-oracle sweeps and the `RANK_TRIPLE` law group pin it against a definitionally correct oracle.
- num/tests.rs turns the backend into the exact oracle for the arm that exists because the backend cannot hold the value: under a lowered ceiling every wide-arm value is also a `UBig`, and each `Wide` operation is checked one at a time (materialize at every pad, `shr` with redispatch, the bias steps as inverses, window comparison against materialized shifts, decimal rendering). The 96-bit ceiling is deliberately not a multiple of 64, so partial-limb windows and sub-limb shifts are exercised, and the seam corners (all-ones carry across the ceiling, power-of-two borrow back) have exact unit witnesses.
- The test-only ceiling override (num.rs:101-139) is thread-local, scoped by a `Drop` guard that restores the previous value so scopes nest, and documented as routing-only with the caveat that values must not outlive their guard.
- `BACKEND_CAPACITY_BITS`'s stated derivation (`usize::MAX / word-bits` words) matches dashu-int 0.5.0's `Buffer::MAX_CAPACITY` (two lenses verified this in the registry source; I read the crate's `Word` and NTT dispatch but did not re-read `Buffer`).
- `Ranked`'s `Eq`/`Ord`/`Hash` coherence is stated once (rank then bytes; `Equal` is version identity) with the O(1) canonical-equality rung before the co-sweep, pinned by `ranked_orders_by_rank_then_bytes` and `ranked_encoding_orders_like_ord`; the composite decode carries the two-ways check (the rank stream must be the rank the version measures), witnessed from both sides.
- `sum_ranks`' value identity with the pairwise fold is pinned by `rank_sum_equals_the_pairwise_fold` over arbitrary multisets and orders, and `rank_cross_path_normalization` pins `Eq` and `Hash` agreement across add, sum, and add-then-subtract paths, under the lowered ceiling too.
- No em-dash appears in any `assert!`, `expect(`, or `unreachable!(` message in the partition; the messages that exist ("the top limb is nonzero", "a wide value is nonzero by invariant", "the Greater pre-check promises a strictly positive difference") state their premise.

## Open questions for Finch

1. Do you want the single-limbs-arm numerator (rank-22)? It is a design proposal, not a defect: the two-arm form is correct and instrumented. The trade is a large deletion of seam machinery (override module, routing predicate, dual routes, `Base` shims, the wide-regime suite copy) against re-pinning two envelopes, a possible `SmallVec` dependency to keep small ranks allocation-free, and keeping the encode walk's width record for the board's `rank_encode` limb floor. My recommendation: pursue it after rank-32 and rank-16 land, since both shrink the seam's surface first and make the remaining machinery easier to weigh.
2. For `Ranked::cmp` (rank-33): restore the M-bound contract now (a roster string and a regenerated JSON) and treat a sign-only kernel with a domination-certificate early exit as separate future work? I recommend restoring first; the linear class was never true and a cheaper kernel cannot make exact ties linear.
3. Should `Sum` carry a public `# Complexity` island? No fuelscape op exists for it; the board's `rank_sum` cell is high-first only. The crate's "every asymptotic claim is a guarantee" is silent where no claim is made, which is itself a gap for a public operation. I recommend adding the island after rank-20's instrument and cure land, so the stated bound is the achieved one.
4. Crate-wide vocabulary rulings the partition inherits: "honest" (127 uses), "door" (208 uses, no definition in lib.rs), and em-dashes in `//` comments (375 lines). I recommend one crate-level decision for each (define "door" by contrast once in lib.rs or retire it; replace "honest" with mechanisms in a crate-wide pass; settle the dash rule) and have the partition follow, rather than editing these three files alone.
5. Once `Ranked::encode_rank` collapses (rank-32), do you want `raw_parts` retained as the board's limb-denomination accessor (meter/board/ops.rs:2277) or replaced by a `#[cfg(any(test, feature = "meter"))]` accessor beside `content_bits`? I recommend the gated accessor: the remaining callers are tests and the meter board, and the `VERY IMPORTANT` warning (rank-11) then has one home.
6. Should suanpan export its digit width or a `reserve_bits(u64)` entry (rank-14)? I recommend `reserve_bits`: it keeps the digit base private and removes the only cross-crate literal.
7. The refutation pass inferred, without running it, that under `.cargo/mutants.toml`'s `--all-features` campaign the sixteen `meter_wide` call deletions (rank-26) have no killer on record. Worth a `cargo mutants --list -f crates/before/src/version/rank/num.rs` by whoever may run it; the outcome decides between the two resolutions in rank-26.
8. `Ranked::decode` and the borsh `Ranked` impl recompute `version.rank()` to verify the wire prefix, so decoding a composite key costs a full rank fold. This is a trust-boundary check and I judge it justified; if an O(|key|) decode is ever wanted, the alternative is to trust the prefix under the model of record (authenticated honest peers) and document the key as producer-authenticated. I recommend keeping the check.
9. The 32-bit seam's memory peaks (encode of an at-capacity integral rank; the `plus_one`/`minus_one` crossings) are unpinned on wasm32 (rank-16). Is a memory-terminal pin in the existing `*_memory_terminal_traps` style acceptable there, or does the working-set ceiling need the copy deletions first?
10. Where should `M` be defined once (rank-31): a crate-level complexity-notation section the fuelscape include links, or emitted by build.rs into the include itself? I recommend the crate-level section, since `M` appears in prose outside the islands too.

## Dropped

- "M is about O(n log n)" holds only on 64-bit targets (structure [10]): refuted. dashu-int 0.5.0 gates NTT out only under `force_bits = "16"` or `target_pointer_width = "16"` (mul/mod.rs:27, 31-33; arch/mod.rs:9); wasm32 selects generic_32_bit, which ships its own ntt.rs with `Word = u32` and the same 4000-word threshold; dashu's source comments saying "16/32-bit" contradict its cfg, and the cfg is what compiles. Verified in the registry source.
- M is Karatsuba/Toom-3 at typical rank widths, so "about O(n log n)" is misleading (claims [46], one clause): not a defect. The sentence is an asymptotic statement and NTT is the asymptotic tier on every target but 16-bit; the power-law tiers below 4000 words are already the subject of integral.rs's module doc.
- Docs describe a fused rank-only encode (prose [11]), Ranked::encode_rank is Rank::encode by another name (correctness [29]), encode_rank documented as fused (claims [40]): duplicates of rank-32; [40]'s fuelscape `size_measure` site is folded in.
- "historical" comparisons (prose [14]) and dated/moralized rationale (claims [44]): duplicates of rank-24 and rank-13; [44]'s `Base::msb_cmp` note duplicates rank-25.
- Register tells (correctness [33]) and moralized "honest" (prose [15]): merged into rank-13.
- Grammar slips, misplaced size section, repeated sentence (structure [9]), decode's misnamed section (prose [19]), copyedit slips (prose [21]), wording slips (correctness [34]), prose nits (claims [46]): merged into rank-3, rank-9, rank-31, and rank-1.
- Hand-maintained measured ratios (correctness [32]): merged into rank-1. The `dsi-bitstream 0.10` item from [22] is dropped: it names the minor line the claim was evaluated against, which is provenance, not rot.
- expect("output fits") (prose [26], correctness [35]) and idiom nits (structure [8]): merged into rank-17; [8]'s digit-width item is rank-14.
- Long qualified paths (correctness [36], claims [48]): merged into rank-17.
- NotCanonical 2 EiB (claims [41]): duplicate of rank-10.
- Vocabulary collisions (prose [25]): the "class" collision spans two files with distinct qualifiers and is below the bar on its own; "door" and "two-ways pin" are folded into rank-13.
- "schoolbook long division" as internal vocabulary (part of prose [20]): dropped; it is an established term. The rest of [20] is rank-19.
- Structure [7]'s option "add an early return at the shift-by-zero and re-pin": struck. cfa7c7ed shows the shift's limb record feeds the board's `rank_encode` limb floor; rank-24 states the rationale at the site instead.
- Structure [1]'s implication that the wasm32 pins hold arm dispatch as a whole: reframed into rank-23 (the pins hold the load-bearing upper side; the parenthetical over-describes).
