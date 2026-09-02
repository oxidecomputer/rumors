# Performance

This document collects every finding of the holistic review of `before` and `suanpan` (repository `/Users/oxide/src/rumors` at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764) whose primary class is *performance*: places where the crates do avoidable work, denominated per input byte, per node, per boundary, or per test run, and how sure the review is of each. There are 24 such findings: one medium, fifteen low, eight nits. No benchmark was run for any of them. Every entry below is assessed by reading or verified by re-derivation, with two exceptions the entries themselves state: `suite-economics-2` rests on two filtered `cargo nextest` runs, and several entries cite witness constructions that ran for neighboring *claim*-class findings (`rank-33`, `party-22`, `party-25`, `skyline-coding-9`, `skyline-coding-29`, `skyline-sweep-place-masked-5`). Each entry therefore ends with a line, added in synthesis, stating the finding's sign (a *fixed-sign* change strictly deletes work at every input, so the doctrine says construct and measure; a *workload-dependent* change trades one resource for another and must be measured first) and naming the meter, bench, or fuel reading that would settle it. Ids are `<partition or sweep key>-<n>`; the full record of each, with the lens reports it was merged from and the refutation and history passes it survived, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. Severity is the review's four-step scale as the finalizers applied it: *high* for a breached contract or a wrong answer reachable from input, *medium* for a published cost bound not honored on a reachable input or an inversion of a fast path's stated purpose, *low* for avoidable work with a fixed sign and a named acceptance, *nit* for a constant-factor or build-time cost whose fix costs a re-pin. Provenance: *demonstrated* means a constructed test ran; *executed* means a run settled it; *verified* means mechanically checked or re-derived (grep, git, a hand trace of the code); *assessed* means read. Anchors were re-read at the reviewed commit while this document was written; the working tree's HEAD had advanced by two commits confined to `.agent-notes/`, and every cited line matched.

## Highest-value items

1. Every production `a <= b` in `causally` and `span.rs` runs `causal_cmp` with the two-direction exit, so it sweeps both streams to exhaustion whenever `a > b` strictly, while `sweep::le` already implements the one-direction exit behind a test-only cfg gate; the coincident fast rung in `Span::dominance` is thereby slower than the fused walk it replaces on a reachable input (`span-causally-26`). The README's owner-decision item 71 consolidates this entry with the simplification document's `skyline-sweep-place-masked-35` (narrow `sweep::le` and `concurrent` to `#[cfg(test)]`) and recommends the narrowing now, with the lift proposed here deferred as a separate performance proposal carrying its own relational pin; this item therefore waits on that proposal, not on a ruling about the meter surface, and the two acceptance rows in the entry are the proposal's instrument.
2. The serde `Serialize` impls copy the canonical bytes into a fresh `Vec` that `serialize_bytes` could borrow, and `Deserialize` copies serde's `Vec` a second time through `decode`'s `read_to_end` before adopting it; the borsh impls pay neither copy (`crate-root-35`).
3. `rank_cmp` keeps only the sign of `Integrator::finish`, whose last step is `sign_magnitude()`: an O(|total|) digit read plus a same-order `UBig` allocation, where `Accumulator::sign` is amortized O(1); `Ranked`'s public sort key pays it on every comparison (`skyline-query-3`).
4. `IdIndex`'s table search runs `partition_point` over the whole remaining table at every both-present node, log₂(table) probes each, where carrying the enclosing subtree's end bound makes it log₂(subtree) and drops the complete-skeleton total from about B·log B to about 2B probes (`party-26`).
5. `IdIndex::is_disjoint` queues 24 bytes per pending pair and `build` one byte per open frame while every sibling walk prices the same depth in bits; the table guard already proves both stored words fit `u32` (`party-24`).
6. `Rank::encode` clones the whole numerator before a by-value shift that a borrowing `Shr` would avoid, and `plus_one`'s at-ceiling arm keeps the original, the limb vector, and the byte image alive together at exactly the 32-bit capacity boundary where `from_limbs`' own comment says one extra live copy decides between fitting and exhaustion (`rank-16`).
7. Every `Rank` `Sum` readout materializes the result as digits, then limbs, then little-endian bytes, then a `UBig`, where `UBig::from_words` takes the limb vector directly on 64-bit (`rank-29`).
8. `hull` calls `sign_magnitude()` once per emission at every switch boundary although `follow_max` and `follow_min` switch together, so one O(|diff|) read and allocation is paid twice per boundary on the span path (`skyline-coding-15`).
9. On the same-side path `delta_code` rebuilds a code bit for bit from the decoded `Int` although the output equals the followed input's code bit for bit and `Code::from_range` would copy the range; fixed sign for wide codes, to be measured for word-scale ones (`skyline-coding-18`).
10. `PackedBuilder::extract_code` copies wide codes one bit per `BitsBuf::push` on the join/meet cascade, against its own module doc's promise of word operations per primitive, and returns an out-of-contract `Code::Small { len: 0 }` on an empty range (`codec-bits-13`).
11. The gate's board stream runs `amp-board-acceptance` and then `worst-cases-pin`, which spawns and merges the identical grid at the identical two scales to fold a map that is a pure function of results the acceptance run already holds (`board-ops-render-19`).
12. `version_triple_laws` alone is the test suite's critical path, 10.9 s on its own under load 3.8 to 7.5 and the last of 949 tests to finish, because `VERSION_TRIPLE` bundles twelve cheap lattice laws with eighteen span and query laws that re-encode and decode per probe (`suite-economics-2`).

## Crate-wide patterns

- **Peek, then decode again.** `IdReader` has no advance-past-tag primitive, so a peeked 2-bit tag is decoded a second time by the `read` or `skip` that follows it, at four shortcut sites in `fill.rs` (`skyline-fill-grow-11`) and at every node of `sum`, `sum_split`, and `copy_reader` (`party-29`). Both entries also record a metering skew: the scan currency charges 4 bits where the sibling walks charge 2 for the same read, so identical work is priced differently across kernels. One crate-private primitive fixes both, and both fixes re-pin scan columns.
- **Materialize, then discard.** A readout path that computes more than its caller keeps recurs across the arithmetic layer: `rank_cmp` materializes the numerator whose sign it wants (`skyline-query-3`); `hull` materializes `sign_magnitude` twice for one boundary (`skyline-coding-15`); `Rank::encode` clones the numerator to shift it (`rank-16`); a `Sum` readout passes through digits, limbs, bytes, and `UBig` (`rank-29`); `magnitude_from_digits` collects bytes through an unsized `flat_map` (`suanpan-25`). Each is a fixed-sign deletion; the shared instrument is the heap column of the rank rows and the touch column where an accumulator read is involved.
- **Per-element loops where batched primitives exist.** `extract_code` pushes one bit at a time (`codec-bits-13`); `LeafCursor` pushes and pops path bits one at a time while `BitStack::push_bits`/`pop_bits` sit private (`skyline-sweep-place-masked-10`); `ByteWords::gather` takes four bounds-tested byte reads per word (`codec-bits-18`); `emit_offset` takes `Signed` by value and forces a clone at both consumed-site callers (`clippy-pedantic-3`). These are invisible to the scan, limb, touch, and heap meters by construction (the work moves, the counts do not), so the only instruments that can see them are the bench judge's wall-time cells and the fuzz-fit fuel readings.
- **Capacity hints that undercount.** `Clock::encode` grows an empty `Vec` twice (`clock-10`); the splice builder's hint omits the two count-carrying codes (`skyline-fill-grow-36`); `shl` rebuilds into a buffer grown one position at a time (`suanpan-14`); the byte image in `magnitude_from_digits` is unsized (`suanpan-25`). Each is one exact `with_capacity` away from a single allocation; the heap column and an allocation counter (`stats_alloc` is already a workspace dependency) are the instruments.
- **The instruments' own cost.** Three entries are about verification doing avoidable work rather than the library: a duplicated board sweep in the gate (`board-ops-render-19`), a workspace-root release profile that compiles wasmtime and cranelift single-unit for a measurement only the guest needs (`fuzzfit-strategies-1`), and one proptest that defines the suite's wall time (`suite-economics-2`). The section *The cost of verification* below carries the timing evidence and its load disclosure.
- **Fixes that move pins.** Most entries here are fixed-sign deletions whose acceptance is a committed reading dropping: scan bits (`skyline-fill-grow-11`, `party-29`, `party-26`), touches (`skyline-coding-15`, `suanpan-17`), heap (`rank-16`, `rank-29`, `clock-10`, `clippy-pedantic-3`), limb ops (`skyline-coding-18`). The doctrine's attribution rule applies to each: measure at the parent, land the deletion with its re-pin in one commit, and name the movement. Two are owner-gated because the pinned number is a public contract or a gate recipe: `suanpan-17` (exact touch counts are declared a breaking-change surface at `crates/suanpan/src/lib.rs:283-286`) and `board-ops-render-19`.
- **Where the review found no avoidable work.** The comparison layer's early exits (`eq_exit`, the `ptr_eq` and `canonical_eq` rungs), the fused walks priced relationally against their compositions, the `Out` output mode that does zero output work on an unchanged tick, and `Bits::freeze` and `from_canonical` as the only storage boundaries all read as the performance design intended, and the *Positives* section names them. The superlinear findings on production paths (`skyline-coding-9`, `skyline-coding-29`, `skyline-sweep-place-masked-5`, `span-causally-24`, `span-causally-36`, `rank-33`, `party-25`, `codec-bits-29`) are contract breaches and are filed under *claim*; this document cross-references them where a performance entry touches the same code.

## Crate root and public types

### Clock

### clock-10: `Clock::encode` grows an empty `Vec` through `encode_to` where a two-slice concatenation is total and single-allocation
- Where: crates/before/src/clock.rs:735-740 (related: crates/before/src/clock.rs:733, 756-763; crates/before/src/party.rs:554-556, 572-574; crates/before/src/version.rs:1027-1029, 1045-1047; crates/before/src/meter/board/ops.rs:1556-1570)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (sibling `encode`s read as `as_bytes().to_vec()`; sibling `encode_to`s read as one `write_all`; board floor `heap_materializes(n)` read at ops.rs:1562 and floors.rs:615-620); executed: no
- Seen by: structure (idiom), claims (performance); refutation: confirmed; history: deliberate-but-expired (the delegating shape was justified at a3cd36f9 by a bit-boundary-merging writer that 32a655438 and 69f288f3 dissolved; those commits rewrote the sibling `encode`s but not this one)
- Owner-gated: no

`encode` starts from `Vec::new()`, so the first `write_all` allocates `max(8, |party|)` and the second reallocates to hold the version and copies the party bytes again: two allocations and one redundant copy for every clock past eight bytes, plus an `expect` discharging an `io::Result` that a total spelling never produces. The framing the doc example at 733 already states, `[party.encode(), version.encode()].concat()`, sizes the allocation exactly. Fixed-sign deletion of redundant work; the reason for the delegating shape left with the bit writer.

Evidence:

       735	    pub fn encode(&self) -> Vec<u8> {
       736	        let mut bytes = Vec::new();
       737	        self.encode_to(&mut bytes)
       738	            .expect("writing to a Vec is infallible");
       739	        bytes
       740	    }

       733	    /// assert_eq!(bytes, [clock.party().encode(), clock.version().encode()].concat());

Resolution: `pub fn encode(&self) -> Vec<u8> { [self.party.as_bytes(), self.version.as_bytes()].concat() }`, keeping `encode_to` for the writer path. The board's `clock_encode` row has a heap floor of `heap_materializes(n)` and no per-op ceiling in ceilings.rs, so one exact-size allocation reads at the floor and needs no re-pin (assessed, not run). Acceptance: `encode_frames_party_then_version`, `clock_codec_roundtrip`, and the wire snapshots unchanged and green; the `clock_encode` heap currency reads one allocation of `n` bytes.

Sign and instrument: fixed sign (one allocation and one copy deleted per encode past eight bytes), denominated per clock byte. Instrument: the board's `clock_encode` heap cell, whose floor `heap_materializes(n)` the exact-size allocation reads at; the wire snapshots are the value leg.

### Party

### party-24: `IdIndex::is_disjoint` spends 24 bytes per queued pair and `build` one byte per open frame, where every sibling walk spends bits
- Where: crates/before/src/party/ops/index.rs:185-189 (related: crates/before/src/party/ops/index.rs:73-75, crates/before/src/party/ops/index.rs:96, crates/before/src/party/ops/compare.rs:113-115, crates/before/src/party/ops/sum.rs:135-137, crates/before/src/party/ops/build.rs:328-335, crates/before/src/party/ops.rs:6-8)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read: `Vec<(Option<u64>, usize)>` is 24 bytes per entry on 64-bit; the table exists only when `bits.len() <= u32::MAX` so both stored words fit `u32`); executed: no
- Seen by: claims (filed), structure (asked as an open question); refutation: confirmed; history: no rationale found (a99e8c8f7 as written)
- Owner-gated: no

On a both-present chain the pending stack is about 32× the input operand's bytes, while `Lockstep` (2 bits), `Frames` (2-3 bits), and the diff cursor price the same depth in bits, and ops.rs:7-8 states the discipline "a deep operand costs bits, not stack frames or grown segments". The table guard already proves positions and entry indexes fit `u32`, so `(u32, u32)` with a sentinel is an 8-byte entry with no algorithmic change; two delta-coded `PopStack`s would match the siblings' bit pricing. `awaiting_left: Vec<bool>` in `build` is the same shape at one byte per frame.

Evidence:

       185	        // Queued right pairs, innermost last: the indexed side's position
       186	        // (`None` = absent child) and its entry bound. `other`'s right children
       187	        // need no bookkeeping — its cursor reaches each one in stream order,
       188	        // exactly as in the cursor walk.
       189	        let mut pending: Vec<(Option<u64>, usize)> = Vec::new();

Resolution: Store `(u32, u32)` with `u32::MAX` for the absent side (or two `PopStack`s of deltas), make `awaiting_left` a `BitStack`, and state the per-level transient in the method doc as the other walks do. Acceptance: peak heap of `IdIndex::build(acc).is_disjoint(input)` on a both-present chain at depth `d` falls from about `24d` to at most `8d` bytes under the `PeakAlloc` meter; `indexed_disjointness_matches_the_cursor_walk[_deep]` and `unindexed_fallback_*` stay green.

Sign and instrument: fixed sign (auxiliary space only; the walk and its probe count are unchanged), denominated per both-present node of the indexed operand. Instrument: the `PeakAlloc` heap column on a both-present chain, and the board's `party_join_all` heap cell. Cross-references: `party-22` (*claim*, demonstrated: the `u32` table itself is up to 8× the operand, not "strictly smaller"), `party-23` (*claim*: past 2^32 packed bits the index falls back to the quadratic discipline), `party-25` (*claim*, demonstrated: the stated `B log n` term does not describe the code).

### party-26: Table searches scan to the table's end instead of the enclosing subtree's entry bound
- Where: crates/before/src/party/ops/index.rs:226-229 (related: crates/before/src/party/ops/index.rs:279-291, crates/before/src/party/tests.rs:1139-1147, crates/before/src/meter/board/ops.rs:1373-1377)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read: the left subtree's entries follow `entry` with targets below `rights[entry]`, the right subtree's entries have larger targets and end at the parent's bound, so restricting the slice to `[entry + 1, end)` preserves the partition point and can only reduce probes); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (29d3c8f27 priced probes at `log2(table + 1)` per node, matching the whole-suffix search)
- Owner-gated: no

Each both-present node's search runs `partition_point` over `rights[entry + 1..]`, the whole remaining table, so every search is `log₂(table)` probes; carrying an end bound on the pending stack makes each search `log₂(subtree entries)`, and on a complete skeleton the total falls from about `B·log B` to about `2B` probes. The module doc calls the search term "a price this module pays knowingly"; this reduces the price without changing the mechanism. Construct and measure: the parity-halves floor (tests.rs:1147, a measured ×0.75) would trip, and the board's `probes_per_node` model (a ceiling) stays valid; see the open question on that floor.

Evidence:

       226	                                let target = rights[entry];
       227	                                let after_left = entry
       228	                                    + 1
       229	                                    + metered_partition_point(&rights[entry + 1..], target);

Resolution: Carry `end` beside `entry` (the left child's entries are `[left_entry, after_left)`, the right child's `[right_entry, parent_end)`), search `&rights[entry + 1..end]`, and re-derive the parity-halves floor and the board's `with_fold_search` allowance from the tighter per-node model. Acceptance: scan bits on `parity_halves(10)` drop by roughly 4× under the same verdict; the re-derived floor still sits an order above the cursor co-walk's reading; `party_join_all` board cells read at or under their declared model.

Sign and instrument: fixed sign in probes (the bounded slice is a subset of the suffix, so no search does more work), denominated per both-present node; the review's confidence is medium because the 4× figure is a derivation, not a reading. Instrument: the `scan-meter` column on `parity_halves(10)` (each 32-bit probe records 32 scan bits) and the board's `party_join_all` cells. The measured ×0.75 floor `SEARCH_SCAN_FLOOR_BITS` at party/tests.rs:1147 will trip on this improvement, which is the party partition's open question 1 and is carried under *Open questions* below.

### party-29: peek-then-read decodes every tag twice in `sum`, `sum_split`, and `copy_reader`, and counts it twice in the scan currency
- Where: crates/before/src/party/ops/sum.rs:38-39 (related: crates/before/src/party/ops/sum.rs:67-68, crates/before/src/party/ops/sum_split.rs:80, crates/before/src/party/ops/sum_split.rs:110-111, crates/before/src/party/ops/sum_split.rs:123-124, crates/before/src/party/ops/build.rs:89-91, crates/before/src/idbits.rs:126-140, crates/before/src/party/tests.rs:788-792)
- Class / severity / confidence: performance / nit / high
- Provenance: verified (read the peek and read sites; `peek` and `read` each record 2 bits at idbits.rs:118 and :136; `skip` re-decodes the top tag through `skip_subtree`'s header probe; tests.rs:788-791 spells the double count: "peeked, then read"); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (0077affbe introduced `peek` for the copy side; 24dff4f58 pinned "peeked, then read" = 8 as observed)
- Owner-gated: no

`sum` peeks both nodes, then in the both-internal arm reads each, re-decoding the tag it just decoded and recording another 2 bits; `sum_split` does the same at every spine node, and `copy_reader` peeks then `skip`s the same top tag. Fixed-sign deletion of one decode per node, and it removes a metering skew: a `sum` operand tag records 4 scan bits where `is_disjoint` records 2 for the same read, so the currency prices identical work differently across walks.

Evidence:

        38	            let a_node = if a_on { self.peek() } else { IdNode::Empty };
        39	            let b_node = if b_on { other.peek() } else { IdNode::Empty };

        67	                    self.read();
        68	                    other.read();

Resolution: Add `IdReader::advance_past_tag(&mut self)` (a 2-bit position advance, no decode, no record) and use it after every peek that is followed by a read of the same node; re-pin the exact scan witnesses (`sum_split_scan_never_exceeds_the_composition`'s 8-bit splice constant becomes 4) as a deliberate event. Acceptance: the splice witness reads 4 bits (one peek per operand); the sum_split scan test and the id differentials stay green.

Sign and instrument: fixed sign (one 2-bit decode deleted per node), denominated per operand node. Instrument: the `scan-meter` column; the `ID_JOIN`/`ID_SYNC` envelope rows and the `sum_split_scan_never_exceeds_the_composition` witness move and are re-pinned in the same commit. Cross-references: `skyline-fill-grow-11` (the same missing primitive on the event side; one `IdReader` method serves both), `party-34` (*verification-gap*: the `sum_split` scan comparison mixes currencies, which the re-pin here should not paper over).

### Rank

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

Sign and instrument: fixed sign for both deletions (one numerator copy per encode; one live copy fewer at the ceiling crossing), denominated per numerator bit. Instruments: the board's `rank_encode` heap cell (must drop) and limb floor (must still read the shift), and the proposed wasm32 pin at `k = 2^32 - 32`, which is the only place the at-ceiling arm's peak is observable. Cross-references: `rank-32` (*claim*, demonstrated: `Ranked::encode_rank` is `rank().encode()` by another name, so it pays this same clone), `rank-22` (*simplification*: the single-limbs-arm proposal that would dissolve the two-arm boundary this entry tightens), `rank-14` (*idiom*: the hardcoded digit width `32`).

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

Sign and instrument: fixed sign (one byte image per readout deleted; the digit and limb stages stay), denominated per result limb; confidence is medium because the `from_words` route depends on dashu's word width per target. Instruments: `RANK_SUM_MIXED`'s peak-heap column in `tests/meter.rs` and the board's `rank_sum` heap cell; the touch and limb columns are the control (they must not move). Cross-references: `suanpan-25` (the same byte detour inside `magnitude_from_digits`, kept there for a stated one-code-path reason), `rank-20` (*claim*: `sum_ranks`' amortization argument is false for ascending exponent order, so the readout's per-`Sum` cost is not the only issue on this path).

### Span and causally

### span-causally-26: Production `<=` checks sweep through `partial_cmp`, losing the one-direction early exit `sweep::le` already implements; the coincident fast rungs can read more than the walk they replace
- Where: crates/before/src/causally.rs:161-168 (related: crates/before/src/span.rs:315-326, 380-391, 453-476; crates/before/src/causally/polarity.rs:69-107, 127-163; crates/before/src/causally/forms.rs:283-285, 307-309; crates/before/src/causally/query.rs:161; crates/before/src/version/skyline/sweep.rs:156-186, 233-242; crates/before/src/version/skyline/place.rs:253-267; crates/before/src/version/skyline/place/filter.rs:189-194)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (grep: `sweep::le` has no caller outside sweep.rs; read sweep.rs:169-170, the `#[cfg(any(test, feature = "meter"))]` gate on `le`, and sweep.rs:236-242, `order_exit`, which breaks only when both directions are refuted; read place.rs:261-267, where `dominance` breaks `Before` at the first interval refuting `lo <= probe`); executed: no
- Seen by: claims; refutation: confirmed (adds two corollaries: `Floor::contains`/`Ceiling::contains` are slower than `Query::from(after(s)).contains(v)` on a refuted relation, since `filter::admits` returns at the refuting interval; and causally.rs:106's "stops as soon as its verdict is decided" is breached by `le`/`lt` on their own); history: no rationale found (30759af0 gated `le` as dead-code cleanup because nothing in production called it; "production ordering goes through the `PartialOrd` surface" describes that state, not a design decision)
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

Sign and instrument: fixed sign (a `le`-exit sweep never reads more than `causal_cmp` and reads strictly less whenever the watched direction is refuted before exhaustion), denominated per elementary interval, hence per stream bit scanned. Instrument: the two relational `scan-meter` rows the resolution specifies; relational rows carry no constant and so need no re-pin. Lifting `sweep::le`'s cfg gate touches the `meter`-feature `pub mod skyline` surface, whose stability status is an owner question (see *Open questions*, item 2); `skyline-sweep-place-masked-35` (*vestigial*) proposes the opposite disposition for `le` and `concurrent` (narrow to `#[cfg(test)]`), and the two resolve together; README owner-decision item 71 recommends the narrowing now and defers this lift as a separate performance proposal with its own relational pin, so the two acceptance rows above are that proposal's instrument rather than a change to land with the narrowing. Cross-references: `span-causally-24` and `span-causally-36` (*claim*, both demonstrated: the multi-hole query walks cost Θ(k·|span|) against a published linear bound; a fused `refine_partial` is a sibling deletion on the same code), `skyline-sweep-place-masked-21` (*claim*: the filter walks' bound omits the hole count).

## The skyline coding

### The coding: emit

### skyline-coding-15: `hull` reads `sign_magnitude` twice per switch boundary; both emissions switch together
- Where: crates/before/src/version/skyline/emit.rs:251-262 (related: crates/before/src/version/skyline/emit.rs:96-112, 166-172, 333-352, 356-370; crates/suanpan/src/accumulator.rs:950-968)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read: `sign_magnitude` is `O(|self|)` digit touches plus a same-order allocation per its doc); executed: no
- Seen by: claims; refutation: confirmed (with the precision that after an opening tie only one emission moves at the first departure; every later switch moves both); history: no rationale (70eb67ab1's accounting of what hull doubles omits it)
- Owner-gated: no

Fixed-sign optimization (strict deletion of redundant work): `follow_max` and `follow_min` are mirrors, so once seated on opposite sides both change side at the same boundary, and `delta_code` → `switch_delta` → `diff.sign_magnitude()` runs twice for one `(sign, magnitude)` pair on the per-boundary hot path; the hull doc's "only the per-interval side selections and the two output builders are doubled" undercounts it.

Evidence:

    emit.rs
    251	        for emission in &mut outputs {
    252	            let new_side = (emission.pick)(sign, emission.side);
    253	            let code = delta_code(
    254	                &diff,
    255	                emission.side,
    256	                new_side,
    257	                step_a.as_ref(),
    258	                step_b.as_ref(),
    259	            );
    260	            emission.side = new_side;
    261	            emission.out.leaf(depth, code);
    262	        }

    357	    let (diff_sign, magnitude) = diff.sign_magnitude();

Resolution: in hull's loop compute the magnitude lazily once per boundary (only when either emission's `new_side` differs) and pass it to both `switch_delta` calls; keep `emit`'s single-output path unchanged. Acceptance: the hull differential in emit/tests.rs stays byte-identical; the `version_span` board cells' touch readings on switch-heavy pairs fall and are re-pinned under attribution; no other column rises.

Sign and instrument: fixed sign (one O(|diff|) digit read and one allocation deleted per switch boundary on the span path), denominated per switch boundary. Instruments: the touch column of the `version_span` board cells and the span rows of `tests/meter.rs` (the heap column should fall with it); the emit differential is the value leg. Cross-references: `version-core-13` (*vestigial*: `Hull.relation`'s only production reader is a `debug_assert!`, so the hull loop carries a third fold with no consumer), `skyline-coding-9` (*claim*, demonstrated: the builder's re-anchor cascade makes `join` Θ(depth × width) on a flat wide leaf against a spine of pairs; the same `emit` loop feeds that builder).

### skyline-coding-18: same-side wide steps are decoded then re-gamma-coded instead of spliced from the source
- Where: crates/before/src/version/skyline/emit.rs:344-348 (related: crates/before/src/codec/code.rs:42-59; crates/before/src/version/skyline/build.rs:268-269; crates/before/tests/meter.rs:1884)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale (147b55f6c describes the intent as "same-side deltas pass through" while the implementation re-encodes)
- Owner-gated: no

On the same-side path the output code equals the followed side's own input code bit for bit, yet `delta_code` rebuilds it with `gamma_code_signed_int` from the decoded `Int` (a fresh `BitsBuf` built bit by bit for `Int::Wide`) where `Code::from_range` would copy the source range; the fill/grow path already takes the copy route through `continue_verbatim`. The decode is needed for the accumulator fold; the re-encode is redundant. Fixed sign for `Wide` codes, workload-dependent for word-scale ones, so measure first.

Evidence:

    emit.rs
    344	    if new_side == side {
    345	        return match step {
    346	            Some(step) => gamma_code_signed_int(step.sign, &step.magnitude),
    347	            None => gamma_code_signed_int(Sign::Positive, &Int::ZERO),
    348	        };

Resolution: have `LeafCursor::step` also report the payload code's bit range and let `delta_code` return `Code::from_range(src, start, end)` on the same-side path when the step is `Some`; keep the re-encode only for switches. Measure at the parent on `SKYLINE_JOIN_WIDE_TOOTH` and `SKYLINE_JOIN_DENSE` before adopting. Acceptance: the emit differential is unchanged; `SKYLINE_JOIN_WIDE_TOOTH` limb ops fall measurably with the row re-pinned under attribution, and `SKYLINE_JOIN_DENSE` does not rise.

Sign and instrument: mixed. For `Int::Wide` steps the change strictly deletes a bit-by-bit re-encode (fixed sign, denominated per wide-code bit); for word-scale steps a range copy and a word-scale gamma encode are both a few word operations, so the sign there is workload-dependent and the entry says measure first. Instruments: the limb column of `SKYLINE_JOIN_WIDE_TOOTH` (must fall) and `SKYLINE_JOIN_DENSE` (must not rise) in `tests/meter.rs`; the bench judge's join cells for the word-scale case. Cross-references: `codec-bits-13` (the `extract_code` per-bit loop on the same cascade; `Code::from_range` is the shared fix), `codec-bits-12` (*simplification*: `PackedBuilder`'s staging register, whose consolidation the `from_range` route depends on).

### Fill and grow

### skyline-fill-grow-11: Every shortcut arm reads its full child's 2-bit tag twice (peek, then skip)
- Where: crates/before/src/version/skyline/fill.rs:486-495 (related: crates/before/src/version/skyline/fill.rs:584-591, crates/before/src/version/skyline/fill/prescan.rs:193-199, crates/before/src/version/skyline/fill/prescan.rs:255-259, crates/before/src/idbits.rs:132-156)
- Class / severity / confidence: performance / nit / high
- Provenance: verified (read `IdReader::peek`, which records 2 bits, and `IdReader::skip`, whose `skip_subtree` header probe reads and records the Full node's own tag once more); executed: no
- Seen by: claims (45); refutation: confirmed; history: no-rationale-found (`IdReader` has no advance-past-terminal method)
- Owner-gated: no

`id.peek()` records and reads the tag; the following `id.skip()` on the known-Full node runs `skip_subtree`, whose header probe reads and records the same tag again to learn it has no children. Four scan-meter bits per shortcut site where two suffice, at four sites. Fixed-sign deletion on the hot path; acting re-pins every tick row's scan column, which is why this is a nit and not a recommendation to act now.

Evidence:

       486	                if left && matches!(id.peek(), IdNode::Full) {
       495	                    id.skip();
    (idbits.rs)
       136	                crate::codec::scan::record_bits(2); // one 2-bit tag scanned
       151	                crate::codec::scan::record_bits(2);

Resolution: A crate-private `IdReader::skip_terminal()` (advance by 2, no read) for the case where the caller has just peeked `Full`, used at the four sites; re-pin the affected scan columns with attribution in the same commit. Acceptance: tick-row scan readings drop by exactly 2 bits per shortcut site, stated in the re-pin.

Sign and instrument: fixed sign (2 scan bits and one tag decode deleted per shortcut site), denominated per full-child shortcut. Instrument: the `scan-meter` column of every tick row in `tests/meter.rs` and the board's tick group. Cross-references: `party-29` (the id-side twin; one `IdReader` primitive serves both, and the two re-pins belong in one commit).

### skyline-fill-grow-36: The splice builder's capacity hint omits the count's width
- Where: crates/before/src/version/skyline/grow.rs:511-513 (related: crates/before/src/version/skyline/grow.rs:644-668; crates/before/src/codec/build.rs:60-72; crates/before/tests/meter.rs:834)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read `PackedBuilder::with_capacity`, which sizes the `Vec` to `capacity / 8 + 1`; read the two count-carrying codes at 645/654 and 662; `TICKS_WIDE_COUNT_BITS = 8_192`); executed: no
- Seen by: claims (44); refutation: confirmed (not measured); history: deliberate-but-expired (the hint and comment were written for the +1 splice in 183e3cab; 4cc9b995's +k generalization changed exactly the two codes to carry `±k` and did not revisit them)
- Owner-gated: no

The hint is input plus `64`, argued as "a few bits per id level". The output also carries two count-coded deltas (`+k` at the grown leaf and `−k` at its successor, or the chain's `±k` fresh leaves), each about `2·bits(k) + 1` bits, so at the wide-count flatness pin's 8,192-bit counts the output exceeds the hint by about `4·bits(k)` and the byte `Vec` reallocates once, doubling at peak. Fixed-sign correction; the comment's bound also omits a term the code reaches.

Evidence:

       511	    // Subadditivity of the coding bounds the output by the input plus the
       512	    // expansion chain's fresh codes, each a few bits per id level.
       513	    let mut out = SkylineBuilder::with_capacity(event_bits.len() + id_bits.len() + 64);

Resolution: Widen the hint by `4 * events.bits() + 4` and extend the comment: input, plus a few bits per id level of the chain, plus the two count-carrying codes. Acceptance: on the `ticks_wide_count_flatness` cases the builder's byte `Vec` does not reallocate after the initial reservation (an unchanged peak-heap reading with the doubling gone, or a `capacity()` probe before and after in a debug check).

Sign and instrument: fixed sign (one reallocation and its doubled peak deleted when the count is wide; the hint is a few bytes larger otherwise), denominated per count bit. Instrument: the peak-heap column of the `ticks_wide_count_flatness` band in `tests/meter.rs`, or a `capacity()` probe in a debug check. Cross-references: `version-core-15` (*verification-gap*: stored versions retain their build buffer's pre-size capacity, so a widened hint also raises the resident slack this entry's sibling worries about; the two should be weighed together).

### clippy-pedantic-3: `emit_offset` takes `Signed` by value but only borrows it, forcing a clone at both consumed-site callers
- Where: crates/before/src/version/skyline/fill.rs:892-894 (related: crates/before/src/version/skyline/fill.rs:731, crates/before/src/version/skyline/fill.rs:790, crates/before/src/version/skyline/fill.rs:463, crates/before/src/version/skyline/fill.rs:504, crates/before/src/version/skyline/fill.rs:596, crates/before/src/version/skyline/fill/prescan.rs:439, crates/before/src/version/skyline/signed.rs:96-103, crates/before/src/codec/int.rs:18-26)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read fill.rs:455-470, 498-510, 588-602, 670-760, 780-800, 884-935, signed.rs:55-62 and 90-115, codec/int.rs:1-60; grepped every `emit_offset` caller); executed: no
- Verification: reframed: `Signed::magnitude` is an `Int`, which is `Small(u64) | Wide(Base)`, so the clone allocates only when the magnitude is wide; for word-sized magnitudes it is a small copy. The redundancy stands regardless; history: no-rationale-found (the sibling `emit_offset` in prescan.rs:439 already takes `&Signed`)
- Owner-gated: no (private)

`emit_offset` reads `offset` only through `&offset`, `offset.is_zero()`,
`.sum(&offset)`, `offset.sign` (a `Copy` enum), and `&offset.magnitude`
(lines 893, 894, 920, 929, 934), yet takes it by value. Both consumed-site
callers hold `above: &Signed` and must write `above.clone()`. Changing the
parameter to `&Signed` deletes both clones; the three owned-argument callers
(463, 504, 596) pass a reference. Strict deletion of redundant work has a fixed
sign (doctrine under Principle 4): construct and measure, no gating.

Evidence:

       892	    fn emit_offset(&mut self, depth: u64, offset: Signed) {
       893	        self.web.emit_offset(&offset);
       894	        if self.out.is_verbatim() && self.range_is_leaf && offset.is_zero() {

       731	                    self.emit_offset(depth + 1, above.clone());
       790	            self.emit_offset(depth + 1, above.clone());

    crates/before/src/version/skyline/fill/prescan.rs (the sibling already by reference)
       439	    fn emit_offset(&mut self, offset: &Signed) {

    crates/before/src/codec/int.rs
        18	#[derive(Clone, Debug, PartialEq, Eq)]
        19	pub(crate) enum Int {
        20	    /// A value within the machine-word range.
        21	    Small(u64),
       ...
        25	    Wide(Base),

Resolution: `fn emit_offset(&mut self, depth: u64, offset: &Signed)`; pass
`above` at 731 and 790, `&above` at 463 and 596, `&value_offset` at 504.
Re-run the board's fill-family heap cells at the parent and at the change;
tighten any pin that moves. Acceptance: `just gate` clean and no `.clone()`
argument to `emit_offset` remains; any moved heap pin is re-committed with the
attribution.
Construction: under the `limb-meter` build, a tick whose consumed sites carry
wide `above` magnitudes shows two fewer big-integer allocations per consumed
site after the signature change.

Sign and instrument: fixed sign (two clones deleted per consumed site; a big-integer allocation each only when the magnitude is wide, a word copy otherwise), denominated per consumed shortcut site. Instruments: the heap column of the board's fill-family cells and the tick rows in `tests/meter.rs`; the word-scale case is invisible to every deterministic meter and shows only in the bench judge's tick cells. Synthesis note: this is a sweep finding (clippy-pedantic) whose anchor is `fill.rs`; it is placed here with the fill and grow partition's entries because the fix and the re-pin are the same as `skyline-fill-grow-11`'s.

### The comparison kernels

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

Sign and instrument: fixed sign (per-bit loop overhead deleted; the number of bits pushed and popped is unchanged), denominated per path level. No deterministic meter can see it; the instruments are the bench judge's dense comparison cell (`version/partial_cmp` on the dense spine) and the fuzz-fit fuel reading for `ff_version_cmp`. Cross-references: `skyline-sweep-place-masked-5` (*claim*, demonstrated: `block_skip` re-peeks a stationary cursor's trailing run every round, Θ(L·r/64) word reads on Θ(r + L) input; the `trailing_ones` loop this entry batches is the same loop that finding counts), `codec-bits-29` (*claim*, demonstrated: `peek_flip` re-scans the trailing run on every call), `codec-bits-28` (*vestigial*: the dead `width == 64` arm in `push_bits`/`pop_bits`, which a total `push_zeros` would meet).

### Query

### skyline-query-3: `rank_cmp` materializes the full numerator it discards
- Where: crates/before/src/version/skyline/query.rs:286-290 (related: query/integral.rs:1142-1163; suanpan/src/accumulator.rs:704-725, 950-968)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read `Integrator::finish` and suanpan's `sign` and `sign_magnitude` docs); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (c8ea49f4 stated the sign-only intent while reusing `finish`'s single return shape)
- Owner-gated: no

`rank_cmp` keeps `.0` of `pair_fold`, whose `Integrator::finish` ends in `self.total.sign_magnitude()`: `O(|self|)` digit touches and a same-order `UBig` allocation. `Accumulator::sign` is amortized `O(1)`. The module doc promises "keeping only the exact total's sign"; the code reads and allocates the whole total. Fixed-sign deletion of redundant work: a constant factor over a sweep that already touched the total, but `Ranked`'s ordering is a public sort key.

Evidence:

    286  pub fn rank_cmp(a: BitsView<'_>, b: BitsView<'_>) -> Ordering {
    287      // `∫ D`, signed: σ is constantly `+1`, the total is
    288      // `rank(a) − rank(b)`, and only its sign is kept.
    289      pair_fold(a, b, |_| 1).0
    290  }

    (integral.rs:1162)      self.total.sign_magnitude()

Resolution: Split `Integrator::finish` into a `close(&mut self, closing_shift)` that performs the settle, ledger, and base steps, and let callers read the total: `rank` and `pair_integral` take `sign_magnitude()`, `rank_cmp` takes `sign()`. Acceptance: add a `ranked_cmp` row to `query_env` in tests/meter.rs over a deep family (none exists today; the board has the cell at board/ops.rs:668) and pin the touch column; the pinned reading drops on the change while `rank_cmp_agrees_with_the_oracle_in_the_freeze_regime` and `arbitrary_mirrored_arming_trains_cancel_to_equal` stay green.

Sign and instrument: fixed sign (one O(|total|) digit read and one `UBig` allocation deleted per comparison; the settle before it is untouched), denominated per digit of the closed total. Instruments: the touch and heap columns of the proposed `ranked_cmp` row in `query_env`, and the board's `ranked_cmp` cell at board/ops.rs:668. What this deletion does not remove: `rank-33` (*claim*, demonstrated) shows by run that `Ranked::cmp` costs 1.08× `rank`'s wall time and 0.77× its limb ops at every scale, with a wall exponent of 1.14 to 1.28 over stored bits while `partial_cmp` grows at 1.00, because `finish` runs the mass-balanced settle's multiplication before the readout this entry deletes; the M-bound is the settle's, and the two findings resolve separately. That witness also records an instrument limitation worth carrying: the limb meter prices a big multiplication by its operand limb counts, so it is structurally blind to the `M(n)` term and read exponent 0.989 on the same construction. Cross-references: `skyline-query-9` (*verification-gap*: the public `O(M(|v|) · log |v|)` clause has no committed instrument at its tier), `rank-33`.

## The codec

### Bits

### codec-bits-13: extract_code copies wide codes one bit at a time, and has no guard for an empty range
- Where: crates/before/src/codec/build.rs:93-107 (related: crates/before/src/codec/build.rs:24-33; crates/before/src/codec/code.rs:16, 42-59; crates/before/src/version/skyline/build.rs:333-334)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read `extract_code`, `Code::from_range`, and the cascade caller); executed: no
- Seen by: claims [36] (the per-bit wide arm); refutation: confirmed, and raised the `n == 0` corner as new; history: no-rationale-found (the per-bit arm is unchanged since 525e7324; code.rs frames wide codes as the cold path but the module doc promises word ops for every primitive)
- Owner-gated: no

For `n > SMALL_CODE_BITS` the read-back is `out.push(self.bit_at(i))` per bit, each a committed/staged branch plus a `BitsBuf::push`, against the module doc's "bit-granular work costs word ops, not one buffer operation per bit" (26-30); it runs on the join/meet cascade (skyline/build.rs:334) whenever the kept left leaf's code is wide. Linear, so a fixed-sign constant-factor deletion, not an asymptotic breach. Separately, `n == 0` (`start == self.len()`) returns `Code::Small { bits: 0, len: 0 }`, outside `Code::Small`'s documented `1..=63` (code.rs:16) and the precondition `Code::from_range` debug-asserts (`start < end`, code.rs:43); unreachable today (`lens.pop()` is a code length of at least one bit) but the two constructors disagree on the empty-range contract.

Evidence:

        93	    pub(crate) fn extract_code(&self, start: u64) -> Code {
        94	        let n = self.len() - start;
        95	        super::scan::record_bits_u64(n);
        96	        if n <= SMALL_CODE_BITS {
        97	            return Code::Small {
        98	                bits: self.read_bits(start, n as u32),
        99	                len: n as u8,
        100	            };
        101	        }
        102	        let mut out = BitsBuf::with_capacity(n);
        103	        for i in start..start + n {
        104	            out.push(self.bit_at(i));
        105	        }
        106	        Code::Wide(out)

Resolution: Read in `<= 63`-bit chunks with `read_bits` and append with `BitsBuf::push_bits`; or, after codec-bits-12, `Code::from_range(built_view(&self.out), start, self.len())`. Add `debug_assert!(n > 0, "a payload code is never empty")` matching `from_range`. Acceptance: no per-bit loop remains in `extract_code`; the wide-payload envelopes in `tests/meter.rs` read unchanged or lower on heap with scan bits identical.

Sign and instrument: fixed sign (a per-bit branch-and-push loop becomes a per-word copy; the scan record `record_bits_u64(n)` is unchanged by construction), denominated per wide-code bit on the join/meet cascade. Instruments: the heap column of the wide-payload envelopes (`SKYLINE_JOIN_WIDE_TOOTH` and siblings) and the bench judge's join cells, since the scan column cannot move. Cross-references: `codec-bits-12` (*simplification*: `PackedBuilder` reimplements `BitsBuf`'s substrate behind a staging register whose saving is unmeasured; its consolidation gives `extract_code` the `from_range` spelling for free), `codec-bits-15` (*verification-gap*, demonstrated: `PackedBuilder` has no direct model test, which any rewrite of `extract_code` should land first), `skyline-coding-9` (*claim*, demonstrated: the cascade this runs on is Θ(depth × width) on the constructed family; a faster copy lowers the constant, not the class).

### codec-bits-18: ByteWords::gather takes four bounds-tested byte reads per refill, and the u32 word width is unargued
- Where: crates/before/src/codec/dsi.rs:338-345 (related: crates/before/src/codec/dsi.rs:292-302, 325-333, 360-380; dsi-bitstream-0.10.1 src/impls/buf_bit_reader.rs:18, 179-215)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read `gather`, `byte_at`, and the `BufBitReader` refill paths); executed: no
- Seen by: claims [37]; refutation: reframed (the word-width half misread the dependency: `read_bits_refill_word64` exists to spare 64-bit-word readers the cross-half shifts of their 128-bit buffer; `u32` words give a native `u64` buffer and never pay that cost); history: no-rationale-found (3e5b95df23 records "BufBitReader (BE, u32 words)" as a fact)
- Owner-gated: no

Every word refill under every skyline walk calls `byte_at` four times, two comparisons each; when `next + 4 <= body.len()` (every word but the last) one `u32::from_ne_bytes(body[next..next + 4].try_into())` suffices, a fixed-sign deletion of branches on the hottest read path, unmeasured. The `ByteWords` doc states why the tail zero-fills but not why the word is `u32` (a `u64` word would make the reader's buffer `u128`), so the next reader re-derives the choice.

Evidence:

       338	    fn gather(&self) -> u32 {
       339	        u32::from_ne_bytes([
       340	            self.byte_at(self.next),
       341	            self.byte_at(self.next + 1),
       342	            self.byte_at(self.next + 2),
       343	            self.byte_at(self.next + 3),
       344	        ])
       345	    }

Resolution: Add the aligned fast path in `gather` and state the `u32` rationale in the `ByteWords` doc (one sentence). Acceptance: the `dsi/tests.rs` differential suite and scan-bit envelopes unchanged; the doc names the word width's reason.

Sign and instrument: fixed sign (eight comparisons become one per word refill; every read of every skyline walk passes through it), denominated per 32-bit word of stream read. No deterministic meter sees it; the bench judge's dense comparison and decode cells and the fuzz-fit `ff_version_decode`/`ff_version_cmp` fuel readings are the instruments. The refutation's reframe stands: the `u32` word width is the right choice for this reader, and the only remaining ask is the one-sentence rationale.

### Base, text, and tree

### codec-base-text-tree-15: `parse_base` re-validates UTF-8 over a digit run it has already classified byte by byte
- Where: crates/before/src/codec/text.rs:64-66 (related: crates/before/src/codec/text.rs:10-13)
- Class / severity / confidence: performance / nit / high
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (`Cur` has held `&[u8]` since f3da6377 introduced the slice-and-delegate form)
- Owner-gated: no

The loop at 58-60 has already proved every byte in `start..pos` is an ASCII digit; `core::str::from_utf8` walks them again. Keeping the `&str` in `Cur` and slicing it (both ends are on ASCII digits, hence char boundaries) removes the pass and the `expect`. Fixed-sign deletion, denominated in digit-run bytes.

Evidence:

        64	    let digits = core::str::from_utf8(&cur.bytes[start..cur.pos])
        65	        .expect("an ASCII digit run is valid UTF-8");
        66	    Ok(Base::parse_decimal(digits))

Resolution: Store `s: &'a str` in `Cur`, scan via `s.as_bytes()`, and slice the run as `&cur.s[start..cur.pos]`. Acceptance: no `from_utf8` in `parse_base`; the codec and skyline text suites unchanged.

Sign and instrument: fixed sign (one linear pass and one `expect` deleted per decimal run), denominated per digit byte; the backend's radix conversion that follows is the dominant cost, so the constant is small. No deterministic meter reads the UTF-8 pass (the limb meter prices `parse_decimal` by its result width); the bench judge's text-class parse cells are the only instrument. Cross-references: `codec-base-text-tree-13` (*claim*, demonstrated: the `Party` literal entry point's `O(n)` is quadratic on a nested spine because each level copies and re-validates the subtree below it; the id-side parser is the larger text-path cost), `codec-base-text-tree-19` (the two-pass stamp split with its depth counter, filed under *simplification* in that partition).

## Cross-cutting

### The serde impls

### crate-root-35: serde impls copy every payload twice on both sides where the borsh impls copy once
- Where: crates/before/src/serde_impls.rs:20-31 (related: crates/before/src/serde_impls.rs:33-57, crates/before/src/version.rs:1027-1029, crates/before/src/version.rs:1110-1127, crates/before/src/borsh_impls.rs:152-181)
- Class / severity / confidence: performance / low / high
- Provenance: verified (version.rs:1027-1029 `pub fn encode(&self) -> Vec<u8> { self.as_bytes().to_vec() }`; version.rs:1110-1126 `decode` reads the whole reader into a fresh `Vec` and then adopts that buffer without copying; borsh_impls.rs:154 and :172 write `self.as_bytes()` directly and :164-166 and :179 adopt the read buffer); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (the zero-copy decode design is deliberate at e4de158ce and 1af119c1f, but the serde handoff of serde's `Vec` as a `&[u8]` reader predates it and was never revisited)
- Owner-gated: no

Strict deletion of redundant work has a fixed sign. `Serialize` calls `encode()`, which allocates and copies a buffer that `serialize_bytes` could take by borrow for `Party` and `Version`; `Deserialize` takes an owned `Vec<u8>` from serde and hands it to `decode(&bytes[..])`, whose `read_to_end` copies it into a second `Vec` before adopting that one as storage. The borsh door pays neither copy. (Clock, Rank, Ranked, and Span need a Vec on the serialize side; their deserialize sides do not.)

Evidence:

    22          s.serialize_bytes(&self.encode())
    ...
    28          let bytes = <Vec<u8>>::deserialize(d)?;
    29          Party::decode(&bytes[..]).map_err(D::Error::custom)

    version.rs:
  1027      pub fn encode(&self) -> Vec<u8> {
  1028          self.as_bytes().to_vec()
  1029      }
    ...
  1110      pub fn decode<R: Read>(mut reader: R) -> Result<Self, Decode> {
  1111          let mut buf = Vec::new();
  1112          reader.read_to_end(&mut buf).map_err(Decode::Io)?;

Resolution: Serialize `Party` and `Version` from `as_bytes()`. Add a crate-internal owned-bytes door per type (the tail of `decode` after `read_to_end`: validate the slice, then `from_frozen(Bits::from_canonical(buf.into()))`) and route every serde `deserialize` through it, so the Vec the visitor yields becomes the storage; Clock, Ranked, and Span can adopt sub-slices of the one buffer as `Clock::decode` already does. Acceptance: a heap-metered serde round-trip scenario in tests/meter.rs (serde is in `--all-features`) reads peak at most one payload copy plus the fixed allowance for Party and Version; the canonical-bytes pins stay byte-identical.

Sign and instrument: fixed sign, denominated per payload byte on both sides (two copies deleted per direction for `Party` and `Version`, one per direction for the composite types). No committed meter reads the serde path today; the proposed heap-metered round-trip row in `tests/meter.rs` is the instrument, and the `serde_impls/tests.rs` format matrix is the value leg. Cross-references: `crate-root-34` (*correctness*: the same impls serialize as `bytes` and deserialize by requesting a `seq`; the owned-bytes entry point proposed here is where a `serde_bytes` visitor would land too), `rumors-dependence` (rumors deserializes `Version` through this path).

## suanpan

### suanpan-14: `shl` on a digit-engine value rebuilds into a fresh buffer grown one position at a time
- Where: crates/suanpan/src/accumulator.rs:626-627 (related: 557, 587-590, 681-702, 1283-1292, 1370-1372)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read: `mem::take` leaves `digits: Vec::new()`; the fold's spill reserves exactly one (1290); each new position `resize`s by one (1371), so the buffer reaches its width through Vec doubling with the transient overshoot `reserve_digits`' own doc names (684-694)); executed: no
- Seen by: claims; refutation: confirmed (fixed sign, small); history: no rationale found (the rebuild dates from ee894835; 22cb962d made "may release the buffer" the contract)
- Owner-gated: no for the pre-reserve (moves no touch); an in-place variant for `shift % 32 == 0` would change counts and is owner-gated

Evidence:

       626	        let held = core::mem::take(self);
       627	        self.add_accum_shl(&held, shift);

Resolution: before the fold, `self.reserve_digits(held.top + 1 + digit_shift + 2)` (the `+2` covers a carry out of the top digit; `digit_shift` via suanpan-12's helper). Acceptance: one allocation for `shl` on a 64-digit value at shift 32_000 (stats_alloc is a workspace dependency); `held_width_rows_cost_the_held_digits` still reads 2d.

Sign and instrument: fixed sign for the pre-reserve (log₂(width) reallocations become one; no touch moves), denominated per digit of the shifted value. Instrument: an allocation count under `stats_alloc`, with the exact-touch pin `held_width_rows_cost_the_held_digits` as the control that must not move. The in-place variant for whole-digit shifts would change touch counts and therefore falls under the exact-touch contract (*Open questions*, item 1). Cross-references: `suanpan-12` (*simplification*: the shift split and its `expect` duplicated at four sites; its helper is the `digit_shift` the resolution names).

### suanpan-17: A collapse whose re-deposit recenters back into the digit it just zeroed is a fixed point: every sign read costs 6 touches
- Where: crates/suanpan/src/accumulator.rs:869-877 (related: 1342-1396; crates/suanpan/src/lib.rs:115-123; crates/suanpan/src/accumulator/tests/metered.rs:786-795)
- Class / severity / confidence: performance / low / high
- Provenance: verified (hand trace of `fold_and_collapse` and `add_at` on `add_wide(&(UBig::from(2u8) << 128usize))`: digit 4 = 2, run (0, 4). Each `sign()`: read digit 4 (=2), zero it, read digit 3 (=0) so partial `2^33` decides, zero the floor, `add_at(3, 2^33)` recenters with carry 2 and remainder 0 and rewrites digit 4 = 2: six touches, and the state after the first read (run cropped to (0, 3)) is a fixed point); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found
- Owner-gated: yes (touch counts are a public exact contract, lib.rs:283-286; any fix re-derives pins and is a named contract change)

When the top digit is +/-1 or +/-2 over a digit in `[-2^31, 2^31)`, the fold cannot decide at the top, descends one digit, decides, and re-deposits a partial that recenters into exactly the spelling it started from. Amortized O(1) holds (constant 6) and lib.rs:119-121 stays literally true (the re-deposited digit is the next fold's first step), but four of the six touches are churn that leave the representation unchanged, on a shape before's many production sign reads can sit on. metered.rs:786-795 observes the shape only as `> 1`.

Evidence:

       869	        if index < start_top {
       870	            // Collapse: the descent zeroed everything above; zero the
       871	            // floor digit too and re-deposit the exact partial there.
       872	            self.digits[index] = 0;
       873	            touch(1);
       874	            self.top = index;
       875	            if partial != 0 {
       876	                self.add_at(index, partial);
       877	            } else {

Resolution: owner decision. Either skip the collapse when it cannot make progress (`index == start_top - 1` and `partial.abs() >= LAZY_LIMIT`: the re-deposit would carry straight back), or write `(carry, remainder)` directly at `(start_top, index)` without the recenter pass; either re-derives the affected pins and names the touch-count change. Acceptance: a metered pin: after `acc.add_wide(&(UBig::from(2u8) << 128usize)); acc.sign();`, 1,000 further `sign()` reads cost 1,000 touches (6,000 today by trace).

Sign and instrument: fixed sign in touches (six per sign read become one on the fixed-point shape; no other shape's count rises under either resolution), denominated per sign read. Instrument: the `touch-meter` pin the resolution specifies in `accumulator/tests/metered.rs`. The change is owner-gated because `crates/suanpan/src/lib.rs:283-286` declares every touch count a public exact contract; the suanpan-tests partition's open question 3 asks the same question from the test side and recommends pinning the 6 exactly first. Synthesis note: the tree's line 877 reads `            } else {`; the excerpt above trims the `else` branch's opener, which changes nothing about the trace. Cross-references: `suanpan-tests-13` (*test-quality*: three inequality pins in a module whose doc says every pin is exact; metered.rs:786-795 is one), `suanpan-10` (*api-surprise*: a zero wide operand spills the register, another representation change with a touch-count face).

### suanpan-25: `magnitude_from_digits` collects bytes through an unsized `flat_map`
- Where: crates/suanpan/src/accumulator.rs:1505-1511 (related: 1499-1504)
- Class / severity / confidence: performance / nit / high
- Provenance: assessed (read: `FlatMap`'s `size_hint` lower bound counts only active inner iterators, so `collect` starts small and grows by doubling); executed: no
- Seen by: claims; refutation: confirmed; history: deliberate-and-holds for the byte route (1499-1504 states the one-code-path rationale); no rationale for the missing presize
- Owner-gated: no

Evidence:

      1505	fn magnitude_from_digits(digits: Vec<u32>) -> UBig {
      1506	    let bytes: Vec<u8> = digits
      1507	        .iter()
      1508	        .flat_map(|digit| digit.to_le_bytes())
      1509	        .collect();
      1510	    drop(digits);
      1511	    UBig::from_le_bytes(&bytes)

Resolution: `Vec::with_capacity(4 * digits.len())` plus `extend`; no touch or behavior change. (`UBig::from_words` would remove the byte pass but trades the stated one-code-path property; leave it unless the readout shows in before's bench-judge.) Acceptance: one allocation for the byte image.

Sign and instrument: fixed sign for the presize (log₂(bytes) reallocations become one; the byte pass itself stays by the stated one-code-path decision), denominated per result digit. Instrument: an allocation count under `stats_alloc`; touch pins are the control. Cross-references: `rank-29` (the same byte detour one layer up, in `Num::from_limbs`, where `UBig::from_words` is recommended because no one-code-path argument protects it there).

## The instruments

The board, the fuzz-fit harness, and the test suite carry three performance findings of their own. The board's is a duplicated sweep in the gate; the harness's is a build-profile scoping; the suite's is its critical path, and that one sits inside the section on the cost of verification, which also carries the sweep's timing evidence and its load disclosure.

### The board

### board-ops-render-19: `worst-cases-pin` re-sweeps the identical grid at the identical two scales that `amp-board-acceptance` just measured
- Where: crates/before/src/meter/board/shard.rs:645-651 (related: shard.rs:15-18, 570-612; worst.rs:67, 609-611; justfile:466; .github/workflows/ci.yml:177-181; examples/amp_board.rs:162-188)
- Class / severity / confidence: performance / low / high
- Provenance: verified (read shard.rs:575-576 and 645-651, worst.rs:67 and 609-611, justfile:466, ci.yml:177-181; the fold is a pure function of `CellResult`s (worst.rs:174); the 66.7 s pin-leg timing one lens quoted is not in the tree and is dropped); executed: no
- Seen by: scaffolding [3], adequacy [26]; refutation: confirmed (severity lowered: the board stream is niced beside the workspace stream, which the justfile names as the gate's critical path, so the redundant sweeps cost CPU rather than gate wall time); history: no-rationale-found (the pin was wired as its own gate leg at 493ef543; 289e14a4 retired the previous double-measurement pattern on the same principle and left this one; 9e36dd280 unified the acceptance sweeps and touched worst.rs only for vocabulary)
- Owner-gated: yes (gate recipe and CI step changes)

The gate's board stream runs `amp-board-acceptance` then `worst-cases-pin`. `run_acceptance` merges every cell at `DEFAULT_SCALE` and `LADDER_TOP_SCALE`; `check_worst_map` then calls `check_with` with a closure that spawns and merges anew at each `WORST_MAP_SCALES` entry, `[("default", 1.0), ("acceptance", LADDER_TOP_SCALE)]`: the identical grid at the identical scales. The map is a pure fold over `CellResult`s that `run_acceptance` already holds, every judged quantity is a deterministic counter, so the second pair of sweeps produces no new number. Principle 3: the second sweep exists because the pin is a separate binary mode, not because the fold needs fresh readings. The two entry points also label the same scales differently ("ladder base/top" at shard.rs:596-598 versus "default/acceptance" at worst.rs:67).

Evidence:

       645	pub fn check_worst_map(
       646	    shards: usize,
       647	    spawn: ShardSpawner<'_>,
       648	    out: &mut dyn Write,
       649	) -> io::Result<bool> {
       650	    check_with(&mut |scale| Ok(merge(scale, shards, &spawn(scale)?)), out)
       651	}

    worst.rs:
       609	    for (label, scale) in WORST_MAP_SCALES {
       610	        let results = sweeps(scale)?;
       611	        let map = fold(&results);

    justfile:
       466	    start_stream board        10 amp-board-acceptance worst-cases-pin

Resolution: Let the acceptance mode also fold and check the worst map from its two merged sample sets (evaluate each window with `evaluate` and feed `check_with` a closure that returns the already-merged results by scale), make `worst-cases-pin` an alias or a view recipe, and collapse the gate line and the CI steps to one invocation; unify the scale labels. Acceptance: one `cargo run ... acceptance` produces both matrices, both map tables, and the pin verdict; the gate's board stream invokes the example once; the rendered map tables are byte-identical to the current `just worst-cases` output at both scales.

Sign and instrument: fixed sign (two of four board sweeps per gate run deleted; every judged quantity is a deterministic counter, so the folded map is byte-identical by construction), denominated per gate run. Instrument: the board stream's own completion line (`gate: ok board (Ns)` in the gate-streams recipe) before and after, on a quiet machine; because the stream runs at `nice -n 10` beside the workspace stream, the saving is CPU time the other niced streams can use, not gate wall time, which is why the refutation lowered the severity. Owner-gated on the gate recipe and the CI `instruments` steps. Cross-references: `board-ops-render-15` (*verification-gap*, demonstrated: the `segments` column both sweeps judge is a compile-time zero in the binary of record), `board-frame-25` (*scaffolding*: the hand-maintained bench rider list that the same acceptance run would derive).

### Fuzzfit

### fuzzfit-strategies-1: The release profile's guest-only justification governs the harness build too
- Where: crates/before/fuzzfit/Cargo.toml:17-25 (related: justfile:575-577, justfile:597)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: instrument-correctness; refutation: confirmed (nit stands; `panic = "abort"` is ignored for test targets, so the cost is build time only); history: no-rationale-found (the block is the 8cfd3c92 original)
- Owner-gated: yes: the profile is part of the pin's provenance and moving it touches the justfile's artifact path and bands.rs's provenance note

The comment justifies `[profile.release]` as guest codegen provenance, but the table sits at the workspace root and also governs `cargo build -p fuzzfit-harness --tests --release`, so wasmtime, cranelift, and proptest compile with one codegen unit for no measurement benefit (Principle 3: a setting whose stated beneficiary is the guest should be scoped to the guest).

Evidence:

        17	# The guest's profile is part of the measurement's provenance: fuel counts
        18	# instructions of THIS codegen. codegen-units = 1 keeps function layout
        19	# independent of build parallelism; panic = "abort" keeps unwinding tables
        20	# (and their instruction overhead) out of the kernels. Debug assertions stay
        21	# off so fuel prices the production work alone (the amp-board's release-only
        22	# rule, applied to instruction counts).
        23	[profile.release]
        24	codegen-units = 1
        25	panic = "abort"

    justfile:577	    {{ justfile_directory() }}/tools/memwatch cargo build -p fuzzfit-harness --tests --release

Resolution: introduce a guest-only profile (`[profile.guest] inherits = "release"` carrying both settings, built with `--profile guest`) or a per-package override for the heavy tool dependencies; move `fuzzfit_guest_wasm` in the justfile and the provenance note in bands.rs with it; measure `just fuzzfit-build` wall time before and after on a quiet machine. Acceptance: the guest wasm is byte-identical under the new profile and the harness build no longer compiles wasmtime single-unit.

Sign and instrument: build time only, and its sign is fixed only for a cold or invalidated build (a warm `kache` hit pays nothing either way), denominated per harness build of the gate's wasm stream. Instrument: `just fuzzfit-build` wall time on a quiet machine, with the guest wasm's byte identity as the value leg; the fuel bands must not move, which is what makes the profile part of the pin's provenance and the change owner-gated. Cross-references: `fuzzfit-bands-6` (*verification-gap*: the wasmtime pin convention is unenforced; the same provenance note is where a wasmtime version assertion would live), `fuzzfit-strategies` open question 2 (whether the wasm gate stream's clippy leg is clean at HEAD, which the same build would answer).

### The cost of verification (suite-economics sweep)

The suite-economics sweep is the only place in this review where wall time was measured, and the measurements carry two caveats the reader must keep. First, the machine was never quiet: the brief warned that another review workflow might be building in the same workspace, the sweep's own `build.log` opens with `Blocking waiting for file lock on build directory` (another cargo process held the lock), and the recorded load average never fell below 3.8 on 16 logical cores across any run. Second, every number is from the dev profile, which is the profile `just test-all` runs, not a release build. The numbers are therefore indicative of shape and ranking, not of quiet-machine cost.

What ran (all under `cargo nextest run ... --all-features`, logs under the scratchpad's `sweeps/suite-economics/` and `final-sweep:suite-economics/`):

| Run | What | Wall | Load before, after | Note |
|---|---|---|---|---|
| sweep | `-p before -p suanpan -p surface-scan`, 949 tests in 17 binaries | 19.656 s | 5.19, 19.23 | all passing, 2 skipped (`#[ignore]`), 1 flagged leaky, no retries configured |
| run1 | `version_triple_laws` alone | 10.910 s | 3.77, 7.50 | `kache` recompiled `before` for about 30 s first, so the build was not fully warm |
| run2 | `exhaustive_small`, the `amp_board_smoke` binary, suanpan's `ledger_invariants_hold_exhaustively` | 12.915 s | 5.33, 19.86 | `exhaustive_small` took 12.914 s on its own rayon pool; the smoke tests 0.007 to 0.948 s; the ledger 2.545 s |

Where the suite's time goes, from the sweep's run (summed test time per binary, tests per binary): the `before` library binary 157.39 s over 670 tests; `before::meter` 15.01 s over 178; `before::verdict_matrix` 10.33 s over 7; `suanpan` 5.25 s over 59; `before::answer_embedded` 3.71 s over 2; `before::amp_board_smoke` 3.62 s over 5; every other binary under 0.1 s. The slowest ten tests under that load: `version_triple_laws` 18.311 s; `codec::tests::bit_flip_rejects_or_decodes_canonically` 10.888 s; `exhaustive_small` 8.372 s; `version_and_list_laws` 5.267 s; `ranked_composite_bit_flip_rejects_or_decodes_canonically` 5.198 s; `meter::tier2::tests::comb_pairs_hold_subadditivity` 4.618 s; `query::tests::clustered_charge_agrees_at_backend_tier_boundaries` 4.151 s; `answer_embedded::measure_folds_are_flat_per_byte_on_wide_ladder` 3.658 s; `generators::tests::generator_classes_stay_under_mass` 3.428 s; and the three `verdict_matrix` sweeps at 3.33 to 3.41 s each. The load inflation is visible in the one test measured twice: 18.3 s in the full run against 10.9 s alone, about 1.7×.

The gate's structure bears on how these numbers are paid (`justfile:458-471`, read at the reviewed commit): `gate-streams` runs the workspace stream (`clippy clippy-default docs test-all citecheck`) at niceness 0 as the declared critical path and every other stream (`doctest`, `board`, `wasm`, `fuzz`, `surface`, `internal-docs`, `audit`) at niceness 10, so the test suite has first call on the cores and the shorter streams fill what it leaves idle. A commit message on main (4e64a4fb, quoted by the fuzzfit-strategies partition and not re-measured here) reports `just gate clean (398s, all legs)`. Two consequences for the entries in this document: the board's duplicated sweep (`board-ops-render-19`) costs CPU that the niced streams share rather than gate wall time, and the harness's single-unit build (`fuzzfit-strategies-1`) sits on the wasm stream, which finishes before the workspace stream does.

### suite-economics-2: version_triple_laws alone defines the suite's critical path; the VERSION_TRIPLE group bundles two cost classes
- Where: crates/before/src/laws.rs:848 (related: laws.rs:94-102 and 109-134 (the roster and its zero-wiring argument); laws.rs:852-946 (twelve lattice, order, and metric laws); laws.rs:962-1605 (eighteen span and query laws); laws.rs:1663-1714 (the span helpers); crates/before/src/testing/algebraic_laws/tests.rs:81-94 and 346; crates/before/fuzz/fuzz_targets/fuzz_laws.rs:132; crates/before/src/laws/tests.rs:36-51; crates/before/src/span/tests.rs:442-443; crates/before/src/version/tests.rs:26-31)
- Class / severity / confidence: performance / low / high
- Provenance: verified (run1.log: `PASS [ 10.910s] (1/1) before testing::algebraic_laws::tests::version_triple_laws`, alone, load 3.8 to 7.5; the sweep's run.log: 18.311 s as the last of 949 to finish, the next-slowest done at 10.888 s; law count by reading laws.rs:848-1662); executed: yes (run1)
- Verification: confirmed with the absolute number corrected: 10.9 s alone, not 18.3 s (the sweep's figure carried about 1.7x load inflation), and the test remains the longest by a wide margin and the last to finish; history: deliberate-and-holds for the roster design (laws.rs:94-98 anticipates several groups per signature), no-rationale-found for bundling both cost classes in one group
- Owner-gated: no (`before::laws` is exposed only under the `laws` feature, which Cargo.toml:69-73 declares test/fuzz-only)

The suite's wall time is one test. VERSION_TRIPLE holds 30 laws: twelve cheap lattice, order, and metric laws (`merge_associative` through `lag_antitone_in_the_receiver`) and eighteen span and query laws (`conjunction_is_intersection` through `span_meet_associative`) whose bodies loop over `span_candidates` and `operand_spans` products and re-encode and decode per probe (laws.rs:1218-1219), all serialized in one proptest of 256 cases. The cost per law is intrinsic to the claims; the serialization is not. The roster keys every consumer on the input signature, so a second group with the `(version, version, version)` header is driven by construction in the proptest drivers, the organic drive list, and the fuzz target, and the totality pin picks up any new `pub static`.

Evidence:

       848	    pub static VERSION_TRIPLE: (a: &Version, b: &Version, c: &Version);

        94	/// A group added here is therefore *executed by construction* in every
        95	/// consumer: each consumer keys its expansion arms on the input
        96	/// signature, so a new group with a known signature is driven with no
        97	/// further wiring, and one with a novel signature refuses to compile
        98	/// until every consumer says how to feed it.

      1218	                let redecoded =
      1219	                    Version::decode(&probe.encode()[..]).expect("a stored stream re-decodes");

Resolution: split VERSION_TRIPLE into two `pub static` groups with the same header, the lattice/order/metric laws (852-946) staying and the span and query laws (962-1605) moving to a group such as SPAN_TRIPLE, registered in `for_each_law_group!` with its own driver name; no consumer arm changes. Re-point the two comments that name the group (span/tests.rs:442-443; version/tests.rs:26-31). Measure each half once on a quiet machine before choosing the cut. Acceptance: same predicates, same `arb_oracle_version` generator, 256 cases per group; `every_law_group_is_registered` and `law_names_are_unique_across_groups` pass unchanged; in a full `just test-all` the last test to finish is no longer alone by several seconds.

Sign and instrument: the total work is unchanged; the change is to parallelism, so the wall-time sign is fixed (two proptests of 256 cases each run on two nextest processes instead of one) while summed CPU time is the same, denominated per full `just test-all`. Instrument: the per-test time of the two halves and the suite's last-to-finish gap in a `cargo nextest run` on a quiet machine, measured before and after; the value leg is the unchanged law set and case count. Synthesis note: because the machine was loaded on every measurement, the 10.9 s figure is an upper bound on the quiet cost, and the ranking (longest by about 8 s under load, by the same order alone) is the durable fact.

Bearing on the cost of verification but filed under other classes: `suite-economics-1` (*verification-gap*, medium: process-per-test isolation is the premise of every metered reading and nothing checks it; the witness pass did not construct the shared-process case, so whether readings flip under `cargo test` is unobserved); `suite-economics-5` (*documentation*: `exhaustive.rs` calls the small cross-product "well under a second" where run2 measured 12.9 s in the dev profile on a 16-thread rayon pool); `suite-economics-6` (*verification-gap*: two `#[ignore]`d tests, the 2 GiB paren witness and `exhaustive_deep`, are run by no recipe); `gate-legs-1` (*correctness*: the CI coverage job was red at the briefed commit because `masked_cmp_hole_envelope` read 1156 B of peak heap under `cargo llvm-cov nextest` against a 480 B pin, while the same test read 384 B under plain `cargo nextest run` in the gate-legs sweep's one permitted execution; the heap meter is not deterministic under instrumentation, and what allocates the extra bytes is open); `gate-legs-6` (*verification-gap*: no recipe runs the mutation campaign, whose 54,918 listed mutants cannot be a per-commit leg); `deps-4` (*simplification*: `build.rs` is a docs-only formatter every consumer compiles, a recorded decision worth revisiting); `fuelscape-render` open question 4 (`fuelscape-verify`, about a minute of strict dump reading, is `ci`-tier while `build.rs` consumes the dataset on every build); `benches-examples` open question 7 (`just all` calls `bench-judge` in quick mode while `benches/board.rs:31-35` says full sampling is required for any quoted number, and the judge's results depend on a quiet machine the workflow does not attest).

## Positives

Performance design the review found sound, deduplicated across the partition and sweep reports; each item names the partition that verified it by reading (or, where stated, by run).

- Byte equality is semantic equality on both codings, so `Eq` is one `memcmp` and `Hash` one pass over canonical bytes (`skyline.rs:87`, `bits.rs:8-31`); every comparison entry point opens with a `ptr_eq` rung whose contract states exactly what it may derive (equality, never provenance) and names the seed that constrains it (codec-bits, version-core). The coincident-span rungs are held live by scan parity in `tests/coincident_span.rs`, which asserts "a zero is a dead meter" before every comparison (tests-other).
- The storage boundary is two functions: `Bits::freeze` seals the build form and `from_canonical` adopts a validated buffer, so decode adopts its read buffer without a copy (`Version::decode`), the borsh `ReaderCursor` pulls one byte per demanded bit and hands the buffer over as storage, `Clock::decode` adopts two slices of one buffer, and `Span::decode` adopts two slices of a single allocation while proving dominance in the same walk (crate-root, clock, span-causally, codec-bits).
- Every deep walk keeps its transient on bit stacks and states the per-level price at the declaration: `LeafCursor` and `IdLeafCursor` as `BitStack`s, the `sum` frames at two or three bits, `write_id` at one to two bits per open node, `PopStack` at `2·w` bits per entry with its cost model written beside it; `IdIndex` names the one trade it makes and the instruments that price it (recursion sweep, party, codec-bits). `party-24` is the one walk that departs from this discipline.
- Fused walks are priced relationally, not by constant: the `placement`, `span`, `span_codec`, and `identity_fast_paths` modules of `tests/meter.rs` state each fusion's cost as an exact identity against its composition on the same operands (`fused + cmp_ss / 2 == cmp_sv + cmp_se`; `fused == decode_lo + cmp`), each with a nonzero liveness read and a walking control, so there is no number to re-pin and a dead meter cannot pass (envelopes-b, span-causally).
- The changed flag of the fused tick is an output mode (`Out::Unstarted`/`Verbatim`/`Built`): the unchanged branch does zero output work and the first divergence materializes the prefix once (skyline-fill-grow). The builder's absorb face is amortized O(1), argued at the code and pinned per collapse genre (skyline-coding). `Version::fork` and `Clock::fork` clone the version as a refcount bump (map, clock).
- The meter shims compile to nothing without their feature and are called unconditionally, so production kernels carry no cfg forks: `codec::scan::record_bits`, `hull_traffic`, `web_traffic`, `pool_traffic`, and suanpan's `touch` (module-graph sweep); no `debug_assert!` body in suanpan meters, so the exact-touch contract is independent of the debug-assertions setting (suanpan); width tests are kept off the limb meter with the reason at the site so dev and release readings agree (`fill.rs:248-254`, `grow.rs:503-508`; skyline-fill-grow); `literal.rs:11-14` refuses a fuller debug assert because dev builds would then meter a different program than the release board of record (inventory sweep).
- The arithmetic layer's cheap paths are cheap by construction: `Rank::cmp` settles zero first, then an O(1) magnitude-class test, then MSB-aligned windows through one shared kernel with no allocation (rank); `MsbWindows` streams a comparison with one register of carry and stops at the first differing window, so its cost is O(shared-prefix limbs) (codec-base-text-tree); `fold_signed_int` dispatches word-scale deltas straight to `add_u64`/`sub_u64` (skyline-sweep-place-masked); the settle's mass-balanced reduction pins its depth bound `2 * total.ilog2() + 2` against a recursive reference (recursion sweep).
- The instruments that judge cost are built to resist the cheapest passing artifact: the bench judge's IDs are the board's own cell names with a stamped sidecar the judge refuses to cross (benches-examples); two committed known-bad kernels must read red every sweep and `--self-test` pins their shapes; `MIN_JUDGED_MEDIAN_NANOS` is derived from timer error, not calibrated (meter-adequacy sweep); `touch_pair_fold` derives a liveness floor per boundary, as a max not a sum, with zero deltas excluded and the mechanism stated (board-families-floors-judge); suanpan's exact-count pins at two scales make the liveness floor and the flatness witness one assertion, with the collapse-less fold committed and red (suanpan).
- The gate spends its cores deliberately: the workspace stream is the declared critical path at niceness 0 and every other stream is niced, with a liveness floor on stream completion so an OOM-killed stream reads as a failure (gate-legs sweep, read here at `justfile:405-481`). The suite is fast for its size and cannot hide flakiness: 949 tests in 19.7 s wall under load, no `retries`, no slow-timeout flags, and one identified tail (suite-economics sweep).
- Fuel windows are kept pure: `ff_regs_reserve` keeps `Vec` doubling out of every measured window, and `REGS_RESERVE` is tied to both budgets by a `const` assertion so a budget raise past it is a compile error rather than a reallocation inside a measurement (fuzz-guests-pins, fuzzfit-bands). `PAR_SPLIT_THRESHOLD` in the fuelscape counting tables carries bracketing measurements on both sides of the cut and says the exact value is low-stakes (fuelscape-pipeline).

## Open questions for Finch

Deduplicated across the reports, restricted to questions a performance change turns on; each with a recommendation.

1. **Does suanpan's exact-touch contract freeze constant-factor improvements?** `crates/suanpan/src/lib.rs:283-286` declares every touch count a public contract and any change a breaking change; that blocks `suanpan-17` (six touches per sign read on a fixed-point shape), the whole-digit in-place `shl` variant of `suanpan-14`, and the fused per-limb `apply_limbs` pass the suanpan partition raised as a hypothesis. Recommendation: restate the contract as "deterministic and pinned; a count change is a versioned change named in its commit", which keeps the pins as enforcement without forbidding improvement, and pin the 6 exactly first so the fix has a number to move.
2. **Is the `meter`-feature surface part of the stable API?** `span-causally-26` lifts the cfg gate on `sweep::le`; `skyline-sweep-place-masked-35` proposes narrowing `le` and `concurrent` to `#[cfg(test)]`; the board-families, clippy-pedantic, codec-base-text-tree, and skyline-sweep-place-masked partitions each asked the same question about `pub mod skyline`, the counter readers, and `Base` in a meter signature. Recommendation: rule once that the meter surface is instrument surface and record it where surfacecheck reads it. On `sweep::le` itself, the README's owner-decision item 71 consolidates this question with the simplification document's open question 22 and recommends the other disposition: narrow `le` and `concurrent` to `#[cfg(test)]` now (`skyline-sweep-place-masked-35`), and treat `span-causally-26`'s lift as a separate performance proposal with its own pin; under that recommendation `le`/`lt` do not become `pub(crate)` production entries until the proposal is taken up, and this document's earlier recommendation to lift now is superseded by the consolidated one.
3. **How should fixed-sign deletions that move pinned readings land?** Eleven entries here re-pin a scan, touch, heap, or limb column (`skyline-fill-grow-11`, `party-29`, `party-26`, `skyline-coding-15`, `skyline-coding-18`, `rank-16`, `rank-29`, `clock-10`, `clippy-pedantic-3`, `suanpan-17`, `codec-bits-13`). Recommendation: one commit per deletion, measured at the parent, with the re-pin and its attribution in the same commit; group only the two `IdReader` primitives (`skyline-fill-grow-11` and `party-29`), which share a fix and a currency.
4. **The parity-halves search floor is a measured reading × 0.75** (`party/tests.rs:1139-1147`, `SEARCH_SCAN_FLOOR_BITS = 101_397`), so `party-26`'s bounded search, a strict improvement, would trip it, as would a galloping search. Recommendation: derive the floor from the mechanism (every skeleton node both-present, so at least `2^d − 1` searches of at least one 32-bit probe: about 38,876 bits at `d = 10` against the 6,140-bit unmetered reading), and decide separately whether the × 0.75 convention should stay confined to `tests/meter.rs`.
5. **Is `PackedBuilder`'s staging register a measurable saving?** `codec-bits-12`'s resolution is construct and measure; `codec-bits-13`'s `from_range` spelling and `skyline-coding-18`'s same-side copy both simplify if the register goes. Recommendation: build `PackedBuilder { out: BitsBuf }`, run the bench judge at parent and change on a quiet machine, and take the consolidation unless a cell leaves its band; if the register wins, move it into `BitsBuf` so one implementation remains.
6. **Should `Boundary::Wide(Accumulator)` be boxed in `MinWeb` entries?** Every `Entry<P>` is on the order of 100 bytes by field layout and every `mem::replace`, `lease`, `retire`, and push or pop copies that much; the sign is workload-dependent and nothing measured it (skyline-watermark open question 4). Recommendation: print `size_of::<Entry<Reign>>()` in a test, prototype `Box<Accumulator>`, and judge it on the ascend row's heap ceiling and the tick/min_ticks bench cells on a quiet machine; land nothing on anticipated benefit.
7. **`ticks`' changed branch runs a second fused walk over the filled output to record the grow route.** The nested and mirror-wide rows show the second walk's touch cost is small on those shapes; a route-only pass is a workload-dependent trade (skyline-fill-grow open question 10). Recommendation: no action unless a shape is constructed where the second walk's accumulator work is not small.
8. **Do stored versions pay retained-capacity slack at a material rate in rumors?** `version-core-15` finds no resident reading on the tick and hull paths; `skyline-fill-grow-36` widens a hint that would raise the same slack. Recommendation: one resident-bytes row each for the tick and hull outputs in `tests/meter.rs`, and a `shrink_to_fit` decision made on that number; a dependence-sweep item for rumors' long-lived storage.
9. **Should the bench judge's quick mode be the mode `just all` runs, and where is the quiet-machine attestation?** `benches/board.rs:31-35` says quick mode is for agent iteration and full sampling is required for any quoted number, yet `just all` calls `bench-judge` bare; the meter-adequacy sweep carries the question of a committed "last judged at tip" record. Recommendation: leave the recipe, and have the judge stamp the load average into its sidecar so a quoted number carries its own disclosure; every wall-time instrument this document names for a fixed-sign deletion depends on that discipline.
10. **How should `VERSION_TRIPLE` be cut?** Between the lattice/metric laws and the span/query laws on cost (`suite-economics-2`), or by moving the span laws to a `SPAN_TRIPLE` group as a semantic split independent of timing. Recommendation: the semantic split, since it coincides with the cost cut and is a design split in its own right; measure each half once on a quiet machine.
11. **Should the acceptance run compute the worst-case map, and does the pin stay a gate leg?** `board-ops-render-19` removes the duplicate sweeps; the board-ops-render partition's open question 2 asks whether the pin should retire to an audit view. Recommendation: keep the pin (f34f4b34 records a catch the envelope suite would not have made) and compute it from the acceptance sweep's results.
12. **Should the fuzz-fit release profile become a guest-only profile?** `fuzzfit-strategies-1` is owner-gated because the profile is the fuel pin's provenance. Recommendation: `[profile.guest] inherits = "release"` with both settings, the justfile's artifact path and `bands.rs`'s provenance note moved with it, accepted only on a byte-identical guest wasm.
13. **Is the presize allocation-strategy record still live?** `benches/presize.rs` keeps A/B arms compiled into `project` and `Display` with no recorded verdict (`benches-examples-12`, and the skyline-query and skyline-coding partitions' open questions). Recommendation: run the record protocol once, write the verdict into a note and the closing commit, and dissolve the arms the way the stacks leg was closed at cd171c29.
14. **What allocates the extra 772 bytes under coverage instrumentation?** `gate-legs-1`: `masked_cmp_hole_envelope` read 1156 B under `cargo llvm-cov nextest` in CI and 384 B under `cargo nextest run`; the previous coverage run on an identical `before` tree passed. Recommendation: reproduce once under `just coverage-kernel` and then either exclude the allocator's instrumentation-time allocations from the peak or drop the envelope suite from the coverage recipes, since a heap pin that depends on the profiler is not a pin.
15. **Where does the `NEXTEST_EXECUTION_MODE` guard live?** `suite-economics-1`: every metered reading assumes process-per-test isolation and nothing checks it. Recommendation: one function in `before::meter` behind the `meter` feature, called from every metering helper in `tests/meter.rs`, the satellite binaries, and suanpan's touch-meter tests.
16. **Is a memory-terminal wasm32 pin acceptable for the at-capacity integral rank?** `rank-16`'s acceptance adds `pin_rank_integral_roundtrip(k)` at `k = 2^32 - 32`, which may be terminal under the harness's memory cap before the deletions land (rank open question 9). Recommendation: commit it in the `*_memory_terminal_traps` style, flip it after the deletions, and add the origin discriminator the fuzz-guests-pins partition asks for so the trap is attributable.

## Counts

By severity:

| Severity | Count |
|---|---|
| high | 0 |
| medium | 1 |
| low | 15 |
| nit | 8 |
| total | 24 |

By module (the sections above), with severities:

| Module | Entries | medium | low | nit |
|---|---|---|---|---|
| Crate root and public types (clock, party, rank, span and causally) | 7 | 1 | 5 | 1 |
| The skyline coding (emit, fill and grow, comparison kernels, query) | 7 | 0 | 4 | 3 |
| The codec (bits; base, text, and tree) | 3 | 0 | 1 | 2 |
| Cross-cutting (the serde impls) | 1 | 0 | 1 | 0 |
| suanpan | 3 | 0 | 2 | 1 |
| The instruments (the board, fuzzfit, the cost of verification) | 3 | 0 | 2 | 1 |
| total | 24 | 1 | 15 | 8 |

By provenance: 9 verified (one of them, `suite-economics-2`, also executed by two filtered nextest runs), 15 assessed; no performance entry was demonstrated by a witness construction, though six cross-referenced *claim*-class findings were (`rank-33`, `party-22`, `party-25`, `skyline-coding-9`, `skyline-coding-29`, `skyline-sweep-place-masked-5`). By sign: 21 fixed, 2 mixed or workload-dependent in part (`skyline-coding-18` on word-scale codes, `fuzzfit-strategies-1` on a warm build), 1 a parallelism change with unchanged total work (`suite-economics-2`). Owner-gated: 4 (`board-ops-render-19`, `fuzzfit-strategies-1`, `suanpan-17`, and `suanpan-14`'s in-place variant).
