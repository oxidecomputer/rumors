# Partition board-ops-render: The board operation rows, renderer, shards, worst-case families, and board tests

## Partition summary

This partition is the amplification board's instrument layer under `crates/before/src/meter/board/`: the operation row table (`ops.rs`, 2,278 lines), the per-cell measurement seam and the printed matrix (`render.rs`, 302), the child-process wire protocol and merge (`shard.rs`, 651), the argmax fold and its committed ranking pin (`worst.rs`, 670), and the board's sibling unit tests (`tests.rs`, 1,284, the one test file). Total read: 5,185 lines in the five files, plus the neighbours every finding leans on (judge.rs, ceilings.rs, floors.rs, family.rs, cell.rs, measure.rs, export.rs, coverage.rs, registry.rs, recurse.rs, place.rs, the smoke suite, the runner example, the justfile's board recipes, and the cited git history). No cargo, just, or build command was run; one Python snippet reproduced `mechanism()`'s substring tests over the judge's verbatim label strings.

The structure is sound and, in several places, exemplary. Each row in `ops.rs` prepares a cell from bundle slots through `FamilyData`'s accessors and commits a typed `Liveness` declaration per currency; `measure_cell` measures exactly the body at two sizes; a child emits raw `Sample`s as a stamped, tab-separated line with floats carried as IEEE-754 bit patterns and the parent refuses any header that is not byte-for-byte the one it commissioned; the merge's completeness refusal compares the union against the registry's declared reach, and the smoke suite commits a tampered capture per family that must be refused; `worst::rank` records exact ties whole so the pin cannot flap; and `tests.rs` pairs every judgment leg with a known-bad artifact that reads red and a correct one that reads green, through `evaluate` itself. Every `expect`/`unreachable!` message in the five files is a one-line proof.

The dominant issues are of two kinds. First, instrument liveness: the `segments` column has no writer in the binary of record (its only incrementer is `#[cfg(test)]`), so every cell's `seg[...]` reading is a compile-time zero presented as a measurement and the ladder-top rationale rests on an onset that cannot occur there; the ascend-cliff family-stated heap ceilings carry no under-side band or class pin, contrary to the board module doc's commitment; the placement rows declare the touch column not applicable while the placement kernel's own doc states the pair-fold premise the floor needs; the membership and covers rows measure their early-exit verdict on nearly every family; and the delegating-parser floors' separation from the bypass reading is remembered in prose rather than checked per run (git records it once failing silently). Second, prose residue from two dissolutions: the mechanism tag's justifying comment cites an excised triage buffer (and the tag mis-classifies every floor trip as `constant+floor`), a test doc carries a dated measurement and a design-doc citation, and the acceptance criterion's doc names a retired determinism tripwire. The remainder is low items and nits: duplicated scale guards, a two-thousand-line table function under a clippy allow, literals shadowing named constants, undemonstrated protocol refusals, and vocabulary.

## Findings

### board-ops-render-1: The row table's module doc states an absolute the declared-model attachments break
- Where: crates/before/src/meter/board/ops.rs:3-5 (related: ops.rs:391, 427, 800, 902, 1047, 1586, 1710, 1770; board.rs:25-29)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'f\.kind' ops.rs` returns exactly the eight sites); executed: no
- Seen by: scaffolding [1], adequacy [23], instrument-correctness [55]; refutation: reframed (the consults attach judgment models after the cell is prepared and never gate applicability, so the product argument holds; the defect is the sentence); history: deliberate-but-expired (true at 3eadcb107, falsified by 669cf3103 two days later without amendment)
- Owner-gated: no

The module doc's first sentence promises that a row prepares its cell "never from the shape's identity", but eight rows match on `f.kind` to attach owner-declared per-cell models (`with_declared_heap`, `with_capacity_model`, `with_declared_limb`). The attachments do not change which shapes a row reaches, so the product argument the sentence defends still holds; the sentence is what is wrong (Principle 5: prose states what is).

Evidence:

         3	//! Each row declares the bundle slots its signature consumes and prepares its
         4	//! cell from them alone — never from the shape's identity — so a row reaches
         5	//! every shape that supplies its operands.

       391	                    return Some(if matches!(f.kind, FamilyId::AscendCliff) {
       392	                        cell.with_declared_heap(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)

Resolution: Name the exception where the rule is stated: "...never from the shape's identity, except to attach an owner-declared per-cell judgment model (the `ceilings` module's declared-models section), which by construction is a statement about one named cell and never about reach." Apply the same amendment to board.rs:25-29. Relocating the declarations onto the bundle (a `declared` slot the family builder fills, as `output_dominated` is) is an optional design proposal; the declared constants differ per operation (the tick trio versus `version_min_ticks`), so a per-family slot is not obviously cleaner. Acceptance: the two docs and the code agree; a reader of ops.rs:3-5 is told where identity is consulted and why.

### board-ops-render-2: `designed()` is a shape-axis declaration living in the operation module, hand-mirroring the registry's envelope-only roster with no test binding the two
- Where: crates/before/src/meter/board/ops.rs:99-187 (related: registry.rs:569-571; family.rs:765-786; export.rs:144-149; benches/common/sidecar.rs:131-132; justfile:466, 1003)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: `designed(` has one caller, export.rs:147, inside the `BenchMode::Pinned` arm; `BenchMode::Pinned` is selected only in benches/common/sidecar.rs:131-132; the gate's board stream at justfile:466 runs acceptance and the pin only; `bench-judge` appears only in the `all` recipe at justfile:1003; tests.rs:1099 calls `bench_cells(0.02, BenchMode::Full)`, which short-circuits `designed`); executed: no
- Seen by: scaffolding [2], adequacy [24], structure-prose [33]; refutation: reframed (the registry's own checklist locates the arm in the board family module; family.rs:765-786 already enumerates the same envelope-only variants; the public `Coverage` route would expose `OpGroup`); history: no-rationale-found (designed() was written into the monolithic board.rs, dropped into ops.rs by the 89cf4c8d split, and left behind when cc84df7e moved the roster into the registry; the 19-name `unreachable!` arm is what cc84df7e added to keep the match exhaustive)
- Owner-gated: no (a move to family.rs is internal; moving the designation into the public `Coverage::Board` variant would be)

The designed pairings are "declared per shape, on the shape axis" but live in the operation module; the registry's adding-a-family checklist says the arm lives in "the board family module"; the `unreachable!` arm hand-lists nineteen envelope-only variants that family.rs:765-786 already lists for the bundle build; and nothing in `just test` exercises the arm, so a family promoted to `Coverage::Board` while still listed here panics only under `just bench-judge`. Principle 5 (no hand-maintained enumerations of a fact another table owns) and Principle 6 (every hole becomes a committed check).

Evidence:

       102	/// Declared per shape, on the shape axis, so the pinned bench subset is
       103	/// derived — a shape added to the axis must answer which groups it was
       104	/// built against (the exhaustive match), and the subset follows. The

       162	        // Envelope-only families never reach the board's product, so
       163	        // they have no designed diagonal.
       164	        FamilyId::WideToothComb
       ...
       182	        | FamilyId::LatentLadder => unreachable!(
       183	            "{kind:?} is envelope-only in the registry: the bench mirror derives its \
       184	             subset from the board roster alone"
       185	        ),

    registry.rs:
       569	/// Adding a family: the [`FamilyId::spec`] and [`FamilyId::index`] arms and —
       570	/// for a board column — the board family module's bundle-build and
       571	/// designed-diagonal match arms are compiler-forced from the variant. What the

Resolution: (1) Add a unit test in tests.rs: for every `FamilyId::ALL` variant and every `OpGroup`, `catch_unwind(|| designed(kind, group)).is_err()` equals `matches!(kind.spec().coverage, Coverage::EnvelopeOnly { .. })`, so the arm and the registry cannot disagree in either direction. (2) Move `designed` (and `OpGroup`, or a re-export) into family.rs beside the bundle-build match, where the registry doc already says it lives; the envelope-only arm then sits beside its twin. Acceptance: flipping one envelope-only variant's registry coverage to `Board { .. }` fails `just test`; `grep -n 'fn designed' ops.rs` is empty and registry.rs:569-571 reads true.

### board-ops-render-3: A dated rationale justifies the magnitude shapes' `designed` arm
- Where: crates/before/src/meter/board/ops.rs:113-115 (related: none)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); executed: no
- Seen by: structure-prose [32]; refutation: confirmed; history: contradicts-hard-rule (Part II under Principle 5: "Dated rationale at a declaration site is the same failure in disguise"; the comment is from 3eadcb107 and survived d2a9d04e's dated-notes sweep because it carries no calendar date)
- Owner-gated: no

The arm's comment explains the arm by chronology (which family came first) rather than by the present-tense fact it encodes (which groups those shapes stress).

Evidence:

       113	        // The magnitude shapes predate the rank rows' mismatch pair and
       114	        // were never its designed adversary.
       115	        FamilyId::Bigroot | FamilyId::Hugeleaf | FamilyId::Cliff => group != OpGroup::Rank,

Resolution: State the invariant positively, for example "The magnitude shapes stress every group but Rank: the rank rows' mismatch pair is built from the spine families, so these shapes are not its adversary." Acceptance: no "predate" or "were never" in the arm's comment.

### board-ops-render-4: The operation table is a two-thousand-line function under a clippy allow, with its row templates repeated verbatim
- Where: crates/before/src/meter/board/ops.rs:192-194 (related: ops.rs:1840-2153 (sixteen rejection rows), 1141-1212 (four placement rows), 1094-1112, 1517-1536, 1820-1838 (three hash rows), and the eight declared-model branches of finding 1; callers shard.rs:125, 147, 415, export.rs:142, tests.rs:1114, 1241)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read; closure-to-fn-pointer coercion in a `static` initializer under the pinned 1.97.1 toolchain was not compile-checked because builds were not permitted); executed: no
- Seen by: scaffolding [14], structure-prose [29], structure-prose [30]; refutation: confirmed (severity of the template finding lowered to low: the flat table's per-row greppability is a real counter-value and no contract is misstated); history: no-rationale-found (the `vec!` form and the allow are from the board's landing commit 7d81a248a)
- Owner-gated: no

`ops()` builds the whole row table as a `Vec<Op>` on every call under `#[allow(clippy::too_many_lines)]`, although `Op` is `&'static str` + `OpGroup` + a non-capturing `fn` pointer and the table is data. Inside it, the sixteen byte- and text-rejection rows share one body shape (`expect_err`, `assert!(matches!(err, ..), "the placed defect is ..., not {err:?}")`, `(err, fed)`), the four placement rows differ only in the method called, the three hash rows are identical up to the hashed type, and the declared-model attachment repeats at eight sites. Legibility (the bar for finished code is "obviously, reviewably correct"): a reader must check thirteen copies of the rejection contract to know it is stated once.

Evidence:

       192	#[allow(clippy::too_many_lines)]
       193	pub(super) fn ops() -> Vec<Op> {
       194	    vec![

      1848	                Some(Cell::new(n, floors, move || {
      1849	                    let err =
      1850	                        Version::decode(&fed[..]).expect_err("a truncated stream is rejected");
      1851	                    assert!(
      1852	                        matches!(err, Decode::Truncated),
      1853	                        "the placed defect is the cut, not {err:?}"
      1854	                    );
      1855	                    (err, fed)
      1856	                }))

Resolution: Two independent levers, either or both. (a) Make the table data: `pub(super) static OPS: &[Op] = &[ .. ]` (or split into `version_rows()`, `party_rows()`, `clock_rows()`, `rejection_rows()` at the existing section headers), removing the allow and the per-call construction. (b) Add row-builder helpers beside the table: a generic `decode_rejection(fed, floors, decode: fn(&[u8]) -> Result<R, Decode>, expected: fn(&Decode) -> bool, defect_name)` (Version, Span, Party, and Clock all decode `&[u8]` to `Result<_, Decode>`), a `placement_row(span, probe, method)`, a `hash_row(value)`, and a `declared_for(kind, cell)` step. Weigh the greppability trade the owner prefers. Acceptance: no `too_many_lines` allow in ops.rs; the `matches!(err, Decode::..)` assertion appears once per defect kind; the smoke suite's per-family cell counts and the rendered row names are unchanged.

### board-ops-render-5: Em-dashes in plain `//` comments at twenty sites
- Where: crates/before/src/meter/board/ops.rs:327-327 (related: ops.rs:350, 415, 569, 578, 701, 702, 732, 733, 947, 1134, 1139, 1246, 1688, 1982, 2179, 2180; shard.rs:502; tests.rs:946, 947)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -c -E '^\s*//[^/!].*—'` per file: ops.rs 17, shard.rs 1, tests.rs 2, render.rs 0, worst.rs 0); executed: no
- Seen by: structure-prose [41]; refutation: confirmed; history: contradicts-hard-rule (Part II: "Prefer colons (or semicolons) over em-dashes in log messages and comments alike"; rustdoc is rendered prose and is not counted)
- Owner-gated: no

Plain code comments (not `///` or `//!`) use true em-dashes at the twenty sites listed; the doctrine reserves them for rendered prose.

Evidence:

       327	                // endpoint — the codec emission genre, denominated by

Resolution: Replace with colons, semicolons, or spaced `--` in those twenty lines. Acceptance: the grep above returns zero for all five files.

### board-ops-render-6: The ascend-cliff family-stated heap ceilings have no under-side band or class pin
- Where: crates/before/src/meter/board/ops.rs:388-395 (related: ops.rs:425-431, 797-804, 1584-1590; judge.rs:339-345; ceilings.rs:231-242, 360-407, 424-426; board.rs:178-183; registry.rs:1459-1467; tests/meter.rs:35-37)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (judge.rs:341-345 read: `if let Some(declared) = s2.declared_heap { ceiling = declared; }` with no floor arm, unlike the capacity model's banded floor at judge.rs:331-336; ceilings.rs:360-407 read: no under-side clause, where the mirror-wide constant at 424-426 names its liveness pin; `grep -n -i 'ascend\|certificate\|reign' src/testing/asymptotics.rs` returns nothing; registry.rs:1463 `bands: Bands::Unbanded`); executed: no
- Seen by: adequacy [18]; refutation: confirmed; history: no-rationale-found (669cf3103 landed the ascend-cliff declarations beside the mirror-wide model; the mirror-wide doc names its under-side witness, the ascend-cliff docs say nothing about one; the board module doc and the LLM-written design note both assert every declared model is held on the under side)
- Owner-gated: yes (a documented design decision about a ratified declared model)

The tick trio and `version_min_ticks` on the ascend-cliff cross are judged at declared flat heap ceilings of 227 and 247 B/B in place of the global 16 B/B, but the judge substitutes the ceiling only. There is no banded floor (as the capacity model has) and no committed class-liveness pin (as the mirror-wide model has), and the family is `Bands::Unbanded` in the registry, so a certificate-memory cure that drops the reading to 10 B/B stays green under the stale 227 B/B declaration indefinitely. The board module doc commits every declared model to being "held honest on the under side"; the ceilings header says the same (231-242). This is the "mechanism for accepting known failures" the doctrine forbids, once the failure is cured.

Evidence:

       388	                    // The ascending cliff defeats certificate consumption,
       389	                    // so its tick cells carry the ratified family-stated
       390	                    // heap ceiling (the constant's derivation).
       391	                    return Some(if matches!(f.kind, FamilyId::AscendCliff) {
       392	                        cell.with_declared_heap(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)

    judge.rs:
       341	        if c == Currency::Heap {
       342	            if let Some(declared) = s2.declared_heap {
       343	                ceiling = declared;
       344	            }
       345	        }

    board.rs:
       180	//! declared-models section), and held honest on the under side — banded floors
       181	//! where the model predicts a quantity, committed liveness pins where it
       182	//! declares a class — so an improved kernel forces a deliberate re-declaration,

Resolution: Either band the family-stated heap ceilings as the capacity model is banded (a `DECLARED_HEAP_FLOOR` fraction of the declared constant, red as "heap family-stated floor (stale model)"), or commit a class-liveness pin for the certificate-memory mechanism (an envelope or asymptotics test asserting the ascend-cliff tick heap reads above the global 16 B/B, which reads red the day consumption is cured) and cite it from the two constants' docs as the mirror-wide constant cites `render_merge_superlinearity_is_alive`. Add the "improved" probe to tests.rs beside `declared_capacity_model_bands_the_projection_peak`. Acceptance: a synthetic `declared_heap: Some(227.0)` sample pair reading 10 B/B evaluates red on a stale-model leg, or a committed pin fails when the ascend-cliff tick heap drops under the global ceiling; ceilings.rs:360-407 name the under-side check; the release board stays green.

Construction: Through `evaluate`, build two `Sample`s with `declared_heap: Some(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)`, denominators 10_000 and 20_000, heap readings 100_000 and 200_000 (10 B/B, above the 8_192 B flat allowance so the exponent leg is judged and reads 1.00), all floors NA: `red` is empty. Compare the capacity model's "improved" probe at tests.rs:1074-1086, which reads red for the same shape of event.

### board-ops-render-7: Rows reach around `party_pair()` for one side's length or bytes at thirteen sites
- Where: crates/before/src/meter/board/ops.rs:449-450 (related: ops.rs:930, 961, 1292, 1308, 1314, 1432, 1450, 1522, 1672, 2006, 2024, 2255; family.rs:1036-1039, 1061-1062)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep `parties\.as_ref()\.map` over ops.rs and family.rs returns the fourteen sites listed); executed: no
- Seen by: scaffolding [10], structure-prose [31]; refutation: confirmed (the discarded decode runs at prepare, outside measurement, so the cost is bench setup, not a reading); history: no-rationale-found (accessor and raw reach are coeval at 7d81a248a)
- Owner-gated: no

`FamilyData::party_pair()` returns both parties decoded and only their combined length, so every row that prices one party alone decodes both, discards one, and re-reads the raw slot positionally for the length or bytes; `clock()` in family.rs does the same. Legibility: a two-line idiom repeated fourteen times is an accessor waiting to exist, and the positional raw-slot reach is a second way of reading a slot the accessor already owns.

Evidence:

       449	                let (a, _, _) = f.party_pair()?;
       450	                let n = f.parties.as_ref().map(|(a, _)| a.len())?;

    family.rs:
      1036	    pub(super) fn party_pair(&self) -> Option<(Party, Party, usize)> {
      1037	        let (a, b) = self.parties.as_ref()?;
      1038	        Some((decode_party(a), decode_party(b), a.len() + b.len()))

Resolution: Add per-side accessors on `FamilyData` (`party_a(&self) -> Option<(Party, usize)>`, `party_b`, and `party_a_bytes(&self) -> Option<&[u8]>` for the rejection rows), or have `party_pair` return per-side lengths; use them at every site, including `clock()`. Acceptance: `grep -c 'parties.as_ref().map' ops.rs family.rs` is zero outside the accessors.

### board-ops-render-8: Moralized and overloaded vocabulary: unanchored "honest", two meanings of "diagonal", three referents for "seam", and "mints"
- Where: crates/before/src/meter/board/ops.rs:701-702 (related: ops.rs:100, 163, 732, 1981; shard.rs:13, 65, 569, 601; worst.rs:12; tests.rs:423, 585)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep -o -i 'honest' per file: ops 11, render 1, shard 1, worst 1, tests 17; `mint` at ops.rs:1981; `seam` at shard.rs:13, 65, 569; `diagonal` at ops.rs:100 and 163 against eight parameter-space uses in family.rs); executed: no
- Seen by: structure-prose [43]; refutation: confirmed; history: contradicts-hard-rule (writing-style.md forbids "mint" for constructing a value and directs "honest" to be replaced by the property; caveat: the tree's own commit vocabulary used "mint" as late as 2026-08-10, and the date those lines entered writing-style.md was not established, so the rule may postdate the prose)
- Owner-gated: no

Most "honest" uses are anchored to `assert_honest_text` and "output honesty". The unanchored ones ("the honest, harder denominator", "honestly undeclared", "the honest count", "# Honest scope", "the honest linear witness") moralize what a mechanism sentence would state. "Diagonal" means designed pairings in ops.rs and parameter-space `(s, s)` points in family.rs; "seam" names the child/parent split, a private function's link, and the row-order boundary between merges; "mints" is the forbidden word.

Evidence:

       701	                // the honest, harder denominator — the codec rows'
       702	                // rule — and lets the flat-denominator shape's content

      1981	                // The genre the span decode mints: the reversed

Resolution: Replace unanchored "honest" with the mechanism ("the input-byte denominator, which the output-honesty assertion makes the smaller of the two"; "the touch column undeclared: the probe folds no accumulator"; "distinct cells, not the sum"; "# Scope"); write "designed pairings" for the ops.rs "diagonal"; write the concrete thing for each "seam"; "the rejection genre the span decode introduces" for "mints". Acceptance: `grep -c -i honest` across the five files counts only the `assert_honest_text`/"output honesty" sites; no "mint" in the partition.

### board-ops-render-9: `causally_contains` and `party_covers` measure only their early-exit verdict on nearly every family
- Where: crates/before/src/meter/board/ops.rs:1113-1126 (related: ops.rs:1397-1411; floors.rs:697-739; family.rs:679, 717, 751, 761, 797-803; causally/forms.rs:150-153; party.rs:405-406; coverage.rs:141-147; tests/meter.rs:6135-6144, 6354-6361; worst.rs:438, 450; board.rs:189-196)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read: `since` is documented as everything `s` does not already contain; the post-pass sets `w = v + Party::seed()` tick wherever a shape did not build its own `version2`, and only JumpPair, DenseSuffix, ConcurrentPair, and ToothTail do; `membership_floors` refuses (full walk) only on `v.partial_cmp(w) == Some(Greater)`; `party_pair` is documented as the disjoint pair; grep of tests/meter.rs shows both `covers` scenarios assert `!a.covers(b)`); executed: no
- Seen by: instrument-correctness [47]; refutation: confirmed (by reading, not constructed; the certifying walk is measured on at most three families, and the accepting `a ⊇ b` walk on none in the instrument corpus); history: no-rationale-found (7bd84b49 established that thirteen families answer the admitting verdict and dense-suffix alone forces certification, and chose to re-derive the floors on the verdict rather than re-orient the probe; no reason for leaving the certifying path unmeasured is recorded)
- Owner-gated: yes (re-orienting a probe changes what a board row of record measures and forces a `WORST_RANKINGS` re-pin; replacing versus adding a row is the owner's call)

The membership row asks `since(&v).contains(&w)` where the bundle post-pass makes `w = v + one seed tick` for every family that did not build its own pairing, so `w` is in `v`'s strict future, the query admits at one witness, and `membership_floors` commits the root-code scan floor with touch NA. The certifying full walk is measured only where a shape's own pairing satisfies `w < v` (dense-suffix's unit-base mate, consistent with dense-suffix holding the row's limb and scan argmax in the pin). `party_covers` asks `a.covers(&b)` of the disjoint pair, false for every family and decided at the root; the accepting walk is measured nowhere, and the envelope suite's two covers scenarios also assert the disjoint case. The coverage roster credits these rows with pricing `Query::contains` and `Party::covers`. The board's own rule ("the defect maximally deferred in every shape: an early-exit-only measurement would be the cheapest artifact that passes", board.rs:191-196) is stated for the rejection rows and not applied to these two verdicts.

Evidence:

      1120	                let (v, w, n) = f.version_pair()?;
      1121	                let floors = membership_floors(&v, &w, n);
      1122	                Some(Cell::new(n, floors, move || {
      1123	                    let hit = causally::since(&v).contains(&w);

      1401	                let (a, b, n) = f.party_pair()?;
      ...
      1406	                    scan: scan_touch(),
      ...
      1409	                Some(Cell::new(n, floors, move || (a.covers(&b), a, b)))

    family.rs:
       799	            if data.version2.is_none() {
       800	                let mut w = v.clone();
       801	                w.tick(&Party::seed());
       802	                data.version2 = Some(w.encode());

    floors.rs:
       728	    if v.partial_cmp(w) == Some(Ordering::Greater) {
       729	        walk_floors(packed_bytes, touch_pair_fold(v, w))
       730	    } else {

Resolution: Orient each probe so the certifying verdict is what the cell measures, from operands the bundle already supplies: for membership, `causally::since(&w).contains(&v)` (with `w = v + tick`, `v` is covered, so the query refuses and certifies every region; keep `membership_floors` deriving from the actual verdict so a concurrent pairing still floors honestly); for covers, probe `a.covers(&decode_party(&a_bytes))` (the buffer-distinct re-decode the placement rows use at ops.rs:1128-1135) or a prepared `join(a, b).covers(&b)`, floored at `scan_examines(n)`. If both verdicts are wanted, add the certifying rows (this moves every family's `Coverage::Board { cells }` count and the bench mirror). Acceptance: on the release board both rows render a full-examination scan floor on every family, their scan constants sit near a full walk's reading, and the two pin rows move with the movement annotated; the smoke suite's per-family cell counts are unchanged (replace) or re-stated (add).

Construction: At prepare, log the verdict per family: `causally::since(&v).contains(&w)` returns `true` on every family whose `version2` came from the post-pass, and `a.covers(&b)` returns `false` on every family. Compare the two rows' scan readings against `version_cmp` on the same family (a comparable pair that must certify reads ~8 bits per packed byte; these rows read root-code scans on the post-pass-paired families). To see the gap bite, plant a refusal walk that re-scans the probe once per plateau of the bound: the board stays green on every family except dense-suffix.

### board-ops-render-10: The placement rows declare the touch column not applicable although the placement kernel's own doc states the pair-fold premise the floor needs
- Where: crates/before/src/meter/board/ops.rs:1149-1151 (related: ops.rs:1128-1140, 1169, 1187, 1205, 1228, 1257; floors.rs:328-332, 458-492, 643-651; place.rs:22-31; worst.rs:439-444; tests/meter.rs:9463-9820)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read; grep for `touch` inside tests/meter.rs's placement module (lines 9463-9820) returns nothing, so the touch meter inside the placement kernel is unfloored in the envelope suite too; the live kernel was not run to confirm touches exceed the proposed floor); executed: no
- Seen by: adequacy [17]; refutation: confirmed (the rows already commit `scan_examines(n)` on the argument that full examination is forced; the NA reason argues from per-byte variability, which the pair floors already tolerate by taking a max); history: deliberate-and-holds for the NA's existence (declared with the rows at 363e96e0d, reason inline), but place.rs:22-31 (db9dfa3ed, 2026-08-05) postdates it and states the premise the floor rests on, which is new evidence
- Owner-gated: yes (the finding reopens a recorded NA declaration)

Six rows commit `walk_floors(n, na(NA_TOUCH_PLACEMENT))`, leaving the touch column unfloored while the placement walk maintains one `Accumulator` running difference per bound and folds its sign per elementary interval exactly as the pair sweep does; place.rs says each accumulator "sees exactly the write sequence the corresponding pair sweep would commit", which is `touch_pair_fold`'s one universal premise (every nonzero stored delta of either operand lands in the running difference, max not sum). The rows force the confirming full sweep by construction (ops.rs:1128-1140) and commit the full-examination scan floor on that argument, so the touch floor follows from the same premise at arity three. Principle 2: a ceiling over a counter passes vacuously when the counter goes dark, and a touch-meter bypass inside `place` reads green on six rows today; the pin shows the counter live and nonzero on every one of them.

Evidence:

      1149	                Some(Cell::new(
      1150	                    n,
      1151	                    walk_floors(n, na(NA_TOUCH_PLACEMENT)),

    floors.rs:
       329	pub(super) const NA_TOUCH_PLACEMENT: &str =
       330	    "the fused placement walk's delta-fold count varies with how the \
       331	     bounds partition the probe's intervals: no per-byte fold count is \
       332	     forced";

    place.rs:
        26	//! that orientation everywhere: the probe is every pair's `a` operand. A probe
        27	//! crossing folds into both differences; a bound crossing folds into its own.
        28	//! Each accumulator therefore sees exactly the write sequence the corresponding
        29	//! pair sweep would commit — the identity the resource pins in

Resolution: Add `touch_placement_fold(probe, lo, hi)` in floors.rs returning `Floor { min: max(nz(probe), nz(lo), nz(hi)), why }` (NA only when all three store no nonzero delta), state the arity-three premise beside `touch_pair_fold`, and use it on the five placement rows; decide `query_coverage` separately (its two-probe walk has clamp legs) and either floor it the same way or state positively why not. Retire `NA_TOUCH_PLACEMENT` if nothing else uses it. Acceptance: the five rows render a nonzero touch floor on every family with stored nonzero deltas; a synthetic span_place-shaped sample with `readings.touch = Some(0)` evaluates red on `TOUCH_FLOOR_TRIP` (a unit test beside the bypass-walk probe); the release board stays green.

Construction: Through `evaluate`, two `Sample`s for a span_place-shaped cell with the committed `walk_floors(n, na(NA_TOUCH_PLACEMENT))` and `readings.touch = Some(0)`: `red` is empty. Under the proposed floor the same samples read `[TOUCH_FLOOR_TRIP]`. To confirm the premise on the live kernel, reset `suanpan::touch_meter`, run `span.place(&probe)` on a staircase pair, and check touches are at least the maximum nonzero-delta count of the three streams.

### board-ops-render-11: `Summary.red`'s doc calls every red an amplification finding
- Where: crates/before/src/meter/board/render.rs:26-29 (related: judge.rs:335, 373-388; board.rs:92-95; render.rs:219-221)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [15]; refutation: confirmed; history: deliberate-but-expired (the doc is from the board's landing, before liveness floors (fbe82320c) and stale-model floors (a4cc1cf35) added red classes that are not amplification)
- Owner-gated: no

A red cell may be a liveness-floor trip (the meter is not watching) or a stale declared model, neither of which is amplification; `Summary` is the type the runner turns into the exit code, so its field doc is what a reader of the verdict sees.

Evidence:

        26	    /// Cells within every ceiling and exponent bound.
        27	    pub green: usize,
        28	    /// Cells over at least one bound, i.e. amplification findings.
        29	    pub red: usize,

Resolution: "Cells with at least one red leg: an exponent or constant over its bound, a reading outside a declared model's band, or a counter below its liveness floor." Acceptance: the doc names the floor-trip case.

### board-ops-render-12: `mechanism()` tags every liveness-floor trip as `constant+floor`, and its justifying comment cites the excised red-triage buffer
- Where: crates/before/src/meter/board/render.rs:48-62 (related: render.rs:98-104; judge.rs:56-73, 280-309, 335, 373-388; tests.rs:362, 480)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (a Python snippet reproduced the three substring tests at render.rs:50-60 over the verbatim label strings at judge.rs:58-73 and 283-335: every `*_FLOOR_TRIP` yields `constant+floor` because each contains "counter"; `heap capacity-model floor (stale model)` yields `floor`; `segments count` yields `constant`. `git show -s 920bfabb2`: "board: excise the expected-reds triage buffer; any red of record fails outright"; grep for `red-buffer`, `BOARD_EXPECTED_REDS`, `ExpectedRed` over src, tests, examples, benches, AGENTS.md, and the justfile finds only render.rs:99; `mech[` has no consumer outside render.rs); executed: yes (the Python evaluation over the exact labels)
- Seen by: scaffolding [0], structure-prose [27], instrument-correctness [48]; refutation: confirmed (severity of the mis-tag lowered because the `<- reasons` list beside the tag is correct); history: deliberate-but-expired (the tag was the class-binding seal's substrate (a4cc1cf35), whose `ExpectedRed` carried only `exponent` and `constant` booleans, so a floor mis-tag never reached a consumer; the seal was dissolved at 0a5bdaebd, which re-aimed the comment at the triage buffer; 920bfabb2 excised the buffer without touching render.rs; the `"count"`/`"counter"` collision is present in a4cc1cf35's own board.rs)
- Owner-gated: no

The red row's `mech[...]` tag is derived by substring search over the judge's human-readable labels. `"count"` targets the segments constant label (`"segments count"`) and also matches "counter" in every floor-trip message, so a cell red on a floor alone renders `mech[constant+floor]`: a liveness vacuity presented as also a constant regression, contradicting the function's own doc. The comment that justifies the tag names a mechanism the tree no longer has (root AGENTS.md hard rule: nothing refers to code that no longer exists), and nothing consumes the tag since the seal's dissolution. A stringly-typed classifier over another module's message text is the fragile mechanism; the labels are fixed `&'static str` constants, so the kind can be carried as data.

Evidence:

        53	    if red.iter().any(|label| {
        54	        label.contains("constant") || label.contains("count") || label.contains("ceiling")
        55	    }) {
        56	        kinds.push("constant");
        57	    }
        58	    if red.iter().any(|label| label.contains("floor")) {
        59	        kinds.push("floor");
        60	    }

        98	    // A red cell's mechanism tag: which judgment kinds put it on the red list,
        99	    // mirroring the tags a red-buffer triage entry commits.

    judge.rs:
        58	pub(super) const HEAP_FLOOR_TRIP: &str =
        59	    "heap floor: counter reads below floor: the meter is not watching this work";

Resolution: Delete the "mirroring the tags a red-buffer triage entry commits" clause. Then decide the tag's fate on present-tense grounds: either dissolve `mechanism()` and the `mech[...]` column (the `<- {reasons}` list already names every red leg), or keep it as a reader aid and have judge.rs carry the kind as data (a `RedKind { Exponent, Constant, Floor }` beside each label in `CellResult.red`, with `Display` producing today's text) so render classifies by type. The tests that match labels by string (`vec!["limb exponent"]`, `SCAN_FLOOR_TRIP`) compare variants or `to_string()` afterwards. Acceptance: `git grep -n -i 'red-buffer' -- crates` is empty; either `mechanism` is gone or a unit test asserts `mechanism(&[SCAN_FLOOR_TRIP]) == "floor"` and `mechanism(&["segments count"]) == "constant"`, with no `contains(` in the classifier.

### board-ops-render-13: render.rs idiom nits: guard-then-`expect` match arms, an inline `std::collections::` path, and an em-dash in one printed legend line
- Where: crates/before/src/meter/board/render.rs:118-126 (related: render.rs:231, 277, 249, 258; tests.rs:1099, 1242)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep `std::collections::` in the partition: render.rs:231, tests.rs:1099, 1242; grep `—` in render.rs: the string literal at 277, beside legend strings at 249 and 258 that use ` - `); executed: no
- Seen by: structure-prose [40]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The `decl` match uses `_ if x.is_some() =>` guards followed by `x.expect("just matched")` twice where `if let` binds the value; line 231 spells `std::collections::BTreeSet::new()` inline while the file imports `std::io::{self, Write}`; the legend line at 277 uses a true em-dash in terminal output where its sibling legend lines use ` - `.

Evidence:

       118	    let decl = match (r.s1.heap_model, r.s2.heap_model, r.s2.fold_arity) {
       119	        _ if r.s2.declared_heap.is_some() => {
       120	            let d = r.s2.declared_heap.expect("just matched");

Resolution: `if let Some(d) = r.s2.declared_heap { .. } else if let Some((e, k)) = r.s2.declared_limb { .. } else { match (..) { .. } }`; import `BTreeSet` (here and at the two tests.rs sites); replace the em-dash at 277 with ` - ` or a colon. Acceptance: no `expect("just matched")`; no inline `std::collections::` in the partition; no `—` inside string literals.

### board-ops-render-14: render.rs houses the measurement seam beside the renderer, and its "one scale guard" has two verbatim copies
- Where: crates/before/src/meter/board/render.rs:164-197 (related: render.rs:1-6; shard.rs:405-408; export.rs:134-137; measure.rs:1-2; shard.rs:78)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -rn 'scale > 0.0' src/meter/board/` returns render.rs:167, shard.rs:406 (identical predicate and message), export.rs:135 (identical predicate, message "bench cells: ..."); shard.rs:78 already imports `assert_scale` and calls it in `emit_shard`); executed: no
- Seen by: scaffolding [7], scaffolding [8], structure-prose [37]; refutation: confirmed; history: the guard's three copies are coeval at c4695a459 (the uniqueness claim was false at birth); the helpers' home in render.rs expired at 289e14a4, which moved the board driver into shard.rs and left `assert_scale`, `build_pair`, `measure_cell` behind
- Owner-gated: no

The module's first sentence names two responsibilities and the code bears it out: `assert_scale`, `build_pair`, and `measure_cell` are the per-cell measurement discipline every shard child drives, wrapping `measure` from measure.rs ("The measurement engine"), while the rest of the file is the matrix. `assert_scale` is documented as "the one scale guard, shared by every entry that measures", and `merge_samples` and `bench_cells` re-inline the identical predicate (the latter with a different message). Modules have a single responsibility; a doc claim of uniqueness the code does not have is a Principle 5 failure.

Evidence:

         1	//! The measurement discipline and the printed matrix: how one grid cell is
         2	//! measured and judged, and how a whole board's judged cells render.

       164	/// The one scale guard, shared by every entry that measures.
       165	pub(super) fn assert_scale(scale: f64) {
       166	    assert!(
       167	        scale > 0.0 && scale.is_finite(),
       168	        "amp-board: scale must be a positive finite number"
       169	    );
       170	}

    shard.rs:
       405	    assert!(
       406	        scale > 0.0 && scale.is_finite(),
       407	        "amp-board: scale must be a positive finite number"
       408	    );

Resolution: Call `assert_scale` from `merge_samples` and `bench_cells`; move `assert_scale`, `build_pair`, and `measure_cell` into measure.rs beside `measure` (shard.rs already imports from both modules), leaving render.rs with `Summary`, `row`, and `render_results`, and rewrite its first sentence to name the one responsibility. Acceptance: one `scale > 0.0 && scale.is_finite()` in the board; render.rs imports nothing from `measure`, `ops`, or `family`.

### board-ops-render-15: The `segments` column has no writer in the binary of record: a judged meter that is a compile-time zero, with prose and the ladder-top calibration resting on it
- Where: crates/before/src/meter/board/render.rs:212-221 (related: recurse.rs:17-20, 68-69, 74-76, 100-109, 118-129; Cargo.toml:44; meter.rs:3552-3558; ceilings.rs:453-467; floors.rs:627-636; ops.rs:205 and every `seg_ceiling_only()` site; worst.rs:63-66; meter/tests.rs:399-403; tests/meter.rs:22-26)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (recurse.rs read: `SEGMENTS_GROWN` is `#[cfg(any(test, feature = "meter"))]` at 68-69 and its only incrementer `grow` is `#[cfg(test)]` at 100-109, as are `should_grow` and the `descend!` macro; `grep -rn 'SEGMENTS_GROWN\|recurse::grow\|descend!'` over src, tests, benches, examples finds `descend!` users only in testing/bridge.rs, grow/tests.rs, and meter/tests.rs; meter.rs:3552-3558 shows `stack_segments()` reads `recurse::segments_grown()`; Cargo.toml:44 lists `stacker` under `[dev-dependencies]`; `git show -s 1ddb5a483`: "stacker becomes a dev-dependency: library walks are iterative, the guard is test-surface only"); executed: no
- Seen by: instrument-correctness [46]; refutation: confirmed (with git history: the column's judged role and the ×4 ladder top were set when library kernels still recursed (ce9e73b46); the kernels went iterative (da0f6a937, 05bd2b16d) and 1ddb5a483 gated `grow` behind `cfg(test)`; the sentence calling the zero "the measured fact the boards' segments column pins" is a later docs-pass addition (b3f09baa0)); history: deliberate-but-expired (1ddb5a483's keep decision is about the test-only guard, not the board column)
- Owner-gated: yes (removal of an instrument column; a wire-format bump of the shard `PROTOCOL`)

The board renders and judges `segments` as one of five deterministic meters (`seg[e .. ..]` on every row, `segments <= 1` in the legend, an exponent and constant leg in the judge), but the counter's only incrementer is the `#[cfg(test)]` growth arm of `recurse::grow`. The `amp_board` example (the release profile of record) and every integration-test binary compile the library without `cfg(test)`, so no code path in those binaries can write the counter: every cell reads zero by construction, the `MAX_GROWN_STACK_SEGMENTS` ceiling and the segments exponent can never fire there, and a kernel that started recursing would grow no segments through this counter (it would recurse on the native stack, which `deep_tree_stack_safety` catches). Three claims are contradicted: recurse.rs:19-20 calls the zero "the measured fact the boards' segments column pins"; recurse.rs:74-75 says "the counter is always written (the bump is inseparable from the growth arm)" of a build with no growth arm; and `LADDER_TOP_SCALE`'s doc derives ×4 entirely from segment-onset amplifiers that cannot occur in that binary. The unit test that keeps the column "honest" (meter/tests.rs:399-403) proves liveness in the lib's own test build, a different compilation. Principle 2: a ceiling over a counter that cannot count; Principle 3: machinery that outlived the constraint (recursive kernels) that justified it.

Evidence:

       212	        "green iff every meter's exponent <= {MAX_SCALING_EXPONENT}, constants within: \
       213	         heap <= {MAX_HEAP_BYTES_PER_INPUT_BYTE} B/B over {HEAP_FLAT_ALLOWANCE_BYTES} B flat, \
       214	         segments <= {MAX_GROWN_STACK_SEGMENTS}, \
       ...
       220	         the meter is not watching that work; segments is ceiling-only by policy, its honest \
       221	         floor is zero). exponent legs are fitted only where the denominator pair scales \

    recurse.rs:
        68	#[cfg(any(test, feature = "meter"))]
        69	static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);
       ...
       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);

        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.

    ceilings.rs:
       455	/// The base-scale sizes under-detect segment amplifiers: stacker grows a
       456	/// segment only past ~1 MiB of frames, so a recursion-frame amplifier whose
       457	/// onset sits above the base depths reads a false green there. ×4 is the
       458	/// witnessed calibration floor — the smallest sampling scale at which every
       459	/// segment-onset amplifier the suite has caught reads red — so the ladder

Resolution: Owner ruling. (a) Recommended: dissolve the segments currency from the board: drop `ByCurrency::segments`, every row's `seg_ceiling_only()` declaration, the `seg[...]` column, the legend clause, `SEG_FLOOR_TRIP`, and the judge's segments arm; bump the shard `PROTOCOL`; re-state `LADDER_TOP_SCALE`'s rationale on the onset effects that do occur at ×4 (worst.rs:65-66 names the doubling-chain steps); re-word recurse.rs:17-20 and 74-76 and meter/tests.rs:399-403 so the unit test's claim is about the counter mechanism in the test build, not the board; handle or explicitly defer tests/meter.rs's segments pins in the same change. (b) Minimum: disclose in the legend and in board.rs that the counter has no writer outside the lib's test build, so a reader does not take the column as a measurement. Acceptance for (a): `ByCurrency` has four fields; the example and smoke suite pass with no `segments` text on the board face; the acceptance and pin renders are byte-identical on the four remaining columns. For (b): the legend names the column as an inert pinned zero and cites the lib unit test as the counter's only live witness.

Construction: `grep -rn 'SEGMENTS_GROWN.fetch_add' crates/before/src` returns only recurse.rs:106, inside `#[cfg(test)] fn grow`. Add a temporary board row whose body routes through `crate::recurse::descend!`: it fails to compile in the example because the macro is `cfg(test)`. Contrast: `cargo nextest run -p before --lib stack_segment_meter` passes because the lib's own test build compiles `grow`. Any board render shows `seg[e 0.00    0]` on every row.

### board-ops-render-16: Six of the seven shard-merge refusals have no committed known-bad demonstration, and the round-trip test's doc claims coverage of guards it only exercises on the accept path
- Where: crates/before/src/meter/board/shard.rs:34-42 (related: shard.rs:426-496 (the asserts at 436-439, 445, 449-453, 460, 466, 474-475, 481, 488, 494); tests/amp_board_smoke.rs:131-165, 229-302)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep for `stamp mismatch`, `outside its slice`, `duplicate cell`, `emitted past its end`, `no end line`, `unknown operation`, `unknown family`, `trailing fields`, `unknown line`, `non-UTF-8` across crates/before/tests and board/tests.rs returns nothing; the smoke suite read in full holds four tests plus the band-parity survivor, and only `merge_refuses_a_silently_shrunk_grid_for_every_family` tampers a capture); executed: no
- Seen by: scaffolding [5], adequacy [21], instrument-correctness [50]; refutation: confirmed (the duplicate refusal is load-bearing: `BTreeMap::insert` overwrites, so without the assert at 486-489 a duplicated line plus a restated end count merges with per-family counts intact); history: no-rationale-found (the refusals landed under the honest round trip only; b58a80ef's tamper test was scoped to the completeness refusal it added)
- Owner-gated: no

The module doc lists the refusals the parent enforces; only the completeness refusal has a committed tampered capture. `shard_protocol_round_trips` (smoke 135-137) says it covers "the ownership and count guards", but an honest three-shard round trip exercises only their accept paths, so a guard that never fires passes it. Deleting the `owns` assert or the duplicate assert passes every committed test. Principle 6: every criterion needs a committed demonstration that a known-bad mechanism fails it; the tampering harness (honest capture, line surgery, `catch_unwind`, message match) already exists in the completeness test.

Evidence:

        34	//! guarding truncation. The parent refuses any mismatch — a header that is not
        35	//! byte-for-byte the one it commissioned, an unknown operation or family name,
        36	//! a cell outside the child's slice, a duplicate cell, a count that
        37	//! disagrees with the lines received, or a merged grid whose per-family

       483	            let duplicate = cells
       484	                .insert((op_position, family_position), (op, family, s1, s2))
       485	                .is_some();
       486	            assert!(
       487	                !duplicate,
       488	                "amp-board shard merge: duplicate cell {op} x {family}"
       489	            );

Resolution: Extend the smoke suite's tamper test into a table over the honest capture: flip one hex digit of the header's scale bits (stamp mismatch); rename one cell's op or family (unknown operation/family); in a two- or three-shard deal move one cell line into another shard's capture and restate both counts (outside its slice); duplicate a cell line and bump the end count (duplicate); drop the end line (truncated); leave the count unchanged after dropping a line (count disagreement); append a field (trailing fields). Assert each is refused with the documented message fragment. Re-word `shard_protocol_round_trips`'s doc to claim the accept path only. Acceptance: one test per refusal in the module doc's list, each failing if its assert is deleted.

Construction: Take the honest single-shard capture the completeness test already builds; duplicate `lines[1]` and set `end N+1`; today `merge_samples` panics on the duplicate assert; with that assert removed the merge succeeds and renders a board with the right per-family counts.

### board-ops-render-17: shard.rs `# Panics` sections are incomplete, the rustdoc allow is module-wide, the merge output is a positional four-tuple, and the bit-pattern parse repeats where a helper exists
- Where: crates/before/src/meter/board/shard.rs:136-138 (related: shard.rs:65-67, 196-201, 231, 235, 279-281, 322-329, 385, 395-399, 404, 532-535, 556, 567, 584; render.rs:165-170, 193; measure.rs:115)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the cited lines: `emit_shard`'s `# Panics` names index and scale only while `assert_unframed` (196-201, called at 231 and 235), `measure_cell`'s applicability expect (render.rs:193), and `assert_honest_text` in `measure` (measure.rs:115) are also reachable; `assert_scale` requires finiteness, which the "strictly positive" wording at 138, 398, and 534 omits; the `#![allow(rustdoc::private_intra_doc_links)]` at 67 is module-wide although only `run_acceptance`'s doc (556, 567) links private items; `merge_samples` returns `Vec<(&'static str, &'static str, Sample, Sample)>` destructured positionally at 385 and 584; `heap_model` and `declared_heap` parsing (322-329) repeat `(text != "-").then(|| from_bits(text, line))` beside the `opt_number` helper at 279-281); executed: no
- Seen by: structure-prose [44]; refutation: confirmed, adding the finiteness omission; history: no-rationale-found
- Owner-gated: no

`emit_shard`, `run`, and `merge_samples` are public and their uniform `# Panics` sections must be complete: each says "strictly positive" where the guard also requires finite (an infinite scale is strictly positive and panics), and `emit_shard` omits the framing-byte assertion on a rationale containing a tab or newline and the panics reachable through `measure_cell`. The lint allow at 67 keeps the detector off for the whole module although one item needs it. A named record beats a four-tuple read by position, and an `opt_bits` helper would parallel `opt_number`.

Evidence:

       136	/// # Panics
       137	///
       138	/// Panics unless `index < count` and `scale` is strictly positive.

       196	fn assert_unframed(text: &str) {
       197	    assert!(
       198	        !text.contains('\t') && !text.contains('\n'),

Resolution: Write "a strictly positive finite number" in the three `# Panics` sections; add the unframed-rationale clause to `emit_shard` (or move `assert_unframed` to a unit test over the floors constants, since every rationale is a `&'static str` constant); move the allow onto `run_acceptance` as an outer attribute; introduce `struct MeasuredCell { op, family, s1, s2 }`; add `fn opt_bits(text, line) -> Option<f64>`. Acceptance: each public `# Panics` lists every panic path; the file has no inner `#![allow]`; no four-tuple destructuring of merge output.

### board-ops-render-18: An unjudged exponent leg passes acceptance uncounted and undeclared
- Where: crates/before/src/meter/board/shard.rs:600-611 (related: render.rs:74-82, 221-225; judge.rs:120-152, 363; ceilings.rs:218-229; tests.rs:686-696)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; the refutation checked the arithmetic of the construction by hand against `MAX_LIMB_OPS_PER_INPUT_BYTE = 128` and `MIN_EXPONENT_DENOM_GROWTH = 1.5`); executed: no
- Seen by: adequacy [19]; refutation: reframed (the skip is the documented design: an unjudged cell rides its constants and floors, which bound single-size cost, and the validation index assigns constant-tight regressions to the envelope suite; what survives is a disclosure gap: no declaration of which cells are expected to take the skip and no count on the summary line); history: no-rationale-found (the guards are an owner-ratified ruling (f68802c33) that fixes the `-.--` rendering; neither it nor 9e36dd280 addresses counting or declaring unjudged legs)
- Owner-gated: yes (the exponent guards are an owner-ratified ruling)

When a cell's denominator pair fails the `MIN_EXPONENT_DENOM_GROWTH` guard, or its heap readings sit inside the flat allowance, the exponent leg is unjudged, the renderer prints `-.--`, and the cell reads green; `Summary` and the exit code carry no count of unjudged legs, and no per-cell declaration says which cells are expected to be unjudged (the ceilings doc names one: the benign rank pair). A generator that stops scaling an operand drops that cell's exponent leg silently on the board of record. The board solves this shape for floors with the typed `Liveness::NotApplicable { reason }` the compiler demands; the exponent leg has no such declaration.

Evidence:

       602	    let red: BTreeSet<(&'static str, &'static str)> = lo_cells
       603	        .iter()
       604	        .chain(hi_cells.iter())
       605	        .filter(|cell| !cell.red.is_empty())
       606	        .map(|cell| (cell.op, cell.family))
       607	        .collect();

    render.rs:
        74	    // An exponent the guards leave unjudged renders -.-- : printing the
        75	    // fitted digits would invite reading noise as a measurement.

Resolution: At minimum, count unjudged non-heap exponent legs (and heap legs whose readings clear the allowance) on the summary line so the number is diffable. Better, give the exponent leg the floors' totality: a per-cell declaration on the rows whose operands legitimately do not scale (the benign rank pair), rendered in the legend, with an undeclared unjudged leg counted red by `run_acceptance`. Acceptance: forged captures with one cell's denominators flat across the ladder and readings growing ×100 make `run_acceptance` exit nonzero; the benign rank pair's declared reason renders in the legend; the release board stays green.

Construction: Through `evaluate`, `sample(100, limb 10)` and `sample(120, limb 10_000)` with all-NA floors: per-unit 10_000/120 = 83 < 128, the pair grows less than ×1.5, so `red` is empty and `scores.limb.exp_judged` is false (the existing `exponent_guards_skip_noise_and_keep_real_amplifiers_red` sub-scaling probe already shows the not-red half). Board-level: take an honest in-process capture, restamp its header to the ladder scales' bit patterns, edit one cell's denominator fields to a constant and its limb readings to grow ×100 across the four points, and feed both to `run_acceptance`: `Summary.red == 0`.

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

### board-ops-render-20: Literals shadow named constants: `1.0` for `DEFAULT_SCALE`, `/ 64` for `OVERLAP_FOLD_INPUT_DIVISOR`, `0.02` for the smoke scale, and the seed and empty-version packed sizes as arithmetic
- Where: crates/before/src/meter/board/worst.rs:67-67 (related: ceilings.rs:451; shard.rs:73, 575, 596-598; tests.rs:520, 1099, 1113; ops.rs:2235; family.rs:170, 1059, 1063, 1084, 1091; ops.rs:400, 436, 453, 526, 923, 957, 993, 1440, 1665, 1676, 2183, 2204; tests/amp_board_smoke.rs:30)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (ceilings.rs:451 `pub const DEFAULT_SCALE: f64 = 1.0;`; worst.rs:54 imports only `HEAP_FLAT_ALLOWANCE_BYTES` and `LADDER_TOP_SCALE`; family.rs:170 `pub(super) const OVERLAP_FOLD_INPUT_DIVISOR: usize = 64;`; ops.rs:2235 uses it by name with a `.max(MIN_SIZE_PARAM)` clamp the test at tests.rs:520 omits; `SMOKE_SCALE` lives in the separate smoke test binary); executed: no
- Seen by: scaffolding [9], scaffolding [13], adequacy [25], structure-prose [38], instrument-correctness [53]; refutation: confirmed; history: deliberate-but-expired for the `1.0` (no base-scale constant existed at 493ef543; `DEFAULT_SCALE` arrived with 9e36dd280 and the site was missed); no-rationale-found for the rest (the divisor landed two minutes before the test that spells `/ 64`, in separate commits)
- Owner-gated: no

`WORST_MAP_SCALES` spells the ladder base as `1.0` while the acceptance ladder uses `DEFAULT_SCALE` for the same point, so a change to `DEFAULT_SCALE` would leave the pin checked at a scale the ladder no longer measures. `join_all_overlap_upfront_test_reads_flat` claims parity with the `party_join_all_overlap` row but spells the divisor as `64` and omits the row's `MIN_SIZE_PARAM` clamp. Two tests use `0.02` for the smoke scale a different binary names. ops.rs and family.rs encode the seed party's and empty version's packed sizes as `n + 1`, `n + 2`, and `2` at a dozen sites, and `i as u64 % 7 + 1` at 526 carries an unnamed modulus. Named constants over magic numbers.

Evidence:

    worst.rs:
        67	pub const WORST_MAP_SCALES: [(&str, f64); 2] = [("default", 1.0), ("acceptance", LADDER_TOP_SCALE)];

    tests.rs:
       520	        let count = a_bytes.len() / 64;

    ops.rs:
       400	                    n + 1,

Resolution: `("default", DEFAULT_SCALE)` with the import, and one label pair for both renderers; `(a_bytes.len() / OVERLAP_FOLD_INPUT_DIVISOR).max(MIN_SIZE_PARAM)` in the test; a lib-side `PROBE_SCALE` constant for the `0.02` sites; `SEED_PARTY_BYTES`/`EMPTY_VERSION_BYTES` constants in family.rs (or compute `Party::seed().encode().len()` once) used by both files; name the `% 7` modulus. Acceptance: no `"default", 1.0` in worst.rs; no `/ 64` in tests.rs; no bare `n + 1`/`n + 2` denominators in ops.rs.

### board-ops-render-21: Hand-rolled run grouping in `worst::fold` where `slice::chunk_by` expresses it
- Where: crates/before/src/meter/board/worst.rs:176-183 (related: rust-toolchain.toml:29)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (rust-toolchain.toml:29 `channel = "1.97.1"`; `chunk_by` is stable since 1.77); executed: no
- Seen by: structure-prose [39]; refutation: confirmed; history: no-rationale-found (the tree tracked floating stable until e7a4b7b0c pinned it, so no toolchain constraint ever forced the cursors)
- Owner-gated: no

`fold` groups consecutive results by `op` with two index cursors and an inner loop; the std adapter names the intent and removes the index arithmetic a reader must check.

Evidence:

       176	    let mut start = 0;
       177	    while start < results.len() {
       178	        let op = results[start].op;
       179	        let mut end = start;
       180	        while end < results.len() && results[end].op == op {
       181	            end += 1;
       182	        }
       183	        let row = &results[start..end];

Resolution: `results.chunk_by(|a, b| a.op == b.op).map(|row| { let op = row[0].op; .. }).collect()`. Acceptance: no index cursors in `fold`.

### board-ops-render-22: The two verdict-of-record entry points have no committed wrong-artifact demonstration
- Where: crates/before/src/meter/board/worst.rs:593-608 (related: shard.rs:570-612; tests.rs:770-855; examples/amp_board.rs:168-187)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (grep for `check_with`, `run_acceptance`, `check_worst_map` across board/tests.rs, tests/, and examples/ finds only the two callers in examples/amp_board.rs:169 and 179; tests.rs exercises `evaluate_acceptance` at 772, 813, 835 and neither caller); executed: no
- Seen by: adequacy [20]; refutation: confirmed; history: no-rationale-found (493ef543's test inventory never drives `check_with`'s drift path; 9e36dd280 tested `evaluate_acceptance` and left `run_acceptance`'s glue to the gate run)
- Owner-gated: no

`check_with` (the pin's drift detector) and `run_acceptance`'s glue (the two-sweep zip and the distinct-red union) are exercised only by the release gate run, whose only observed outcome is "clean"; a checker that never sets `clean = false`, or a summary that counted only one window's reds, would pass the gate with no test to catch it. Principle 6: a harness bug that masks a failure is the highest-value target for a known-bad demonstration, and `check_with` is injectable through its `sweeps` closure.

Evidence:

       593	pub(super) fn check_with(
       594	    sweeps: &mut dyn FnMut(f64) -> io::Result<Vec<CellResult>>,
       595	    out: &mut dyn Write,
       596	) -> io::Result<bool> {
       ...
       608	    let mut clean = true;

Resolution: In tests.rs, (a) feed `check_with` a one-operation synthetic sweep (built from `Sample`s through `evaluate`) whose argmax disagrees with `WORST_RANKINGS` on one currency and assert `Ok(false)` plus a drift line naming op, currency, and both worsts; and a sweep missing a pinned op, asserting the stale-entry line. (b) For `run_acceptance`, restamp an honest smoke capture to the two ladder scales, raise one cell's top-window heap reading over its ceiling, and assert `Summary.red == 1` counted once. Acceptance: both tests fail under a mutated `check_with` that never clears `clean` and under a `run_acceptance` that unions only `lo_cells`.

### board-ops-render-23: `check_with` re-validates the pin table's structure at runtime; the committed test already does
- Where: crates/before/src/meter/board/worst.rs:597-607 (related: worst.rs:587-592; shard.rs:638-644; tests.rs:1238-1284)
- Class / severity / confidence: vestigial / low / medium
- Provenance: verified (tests.rs:1250-1260 read: per scale, `assert_eq!(pinned, ops)` and the total-count assertion imply both a known scale label and no duplicate `(scale, op)` key); executed: no
- Seen by: structure-prose [42]; refutation: confirmed; history: no-rationale-found (the loop and `worst_rankings_pin_is_well_formed` landed together in 493ef543; nothing says whether the loop exists so `just worst-cases-pin` stands alone without the test binary)
- Owner-gated: no

The runtime loop asserts every `WORST_RANKINGS` entry names a known scale label and is not a duplicate; `worst_rankings_pin_is_well_formed` pins, per scale, that the entries equal the operation table exactly and that the total is ops × scales, which excludes both. A runtime recompute over deterministic committed data is not defense in depth once committed coverage exists (Principle 3), and the `# Panics` clauses in `check_with` and `check_worst_map` then document a case the gate already excludes.

Evidence:

       597	    let mut seen = BTreeSet::new();
       598	    for (scale, op, _) in WORST_RANKINGS {
       599	        assert!(
       600	            WORST_MAP_SCALES.iter().any(|(label, _)| label == scale),
       601	            "worst-case pin: unknown scale label {scale:?} on {op}"
       602	        );
       603	        assert!(
       604	            seen.insert((*scale, *op)),
       605	            "worst-case pin: duplicate entry for {op} at the {scale} scale"
       606	        );
       607	    }

Resolution: Drop the loop and the corresponding `# Panics` clauses, or keep it with a one-line comment naming the reason it must hold without the test suite (the pin recipe running standalone). Acceptance: either the loop is gone and the two `# Panics` sections no longer mention a malformed pin, or the loop carries its standalone justification.

### board-ops-render-24: tests.rs prose carries a partial hand-maintained inventory and two counts its bodies contradict
- Where: crates/before/src/meter/board/tests.rs:1-12 (related: tests.rs:104-105 versus 109-114; tests.rs:860-861 versus 910, 921, 932, 953, 964; measure.rs:114-116)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the whole file: the header names the κ tripwires, the `n_io` exponent tripwire, the floors tripwire, and the join_all pin, and omits the exponent-guard, acceptance-trend, fold-model, capacity-model, bench-rider, and worst-map tests at 650-1284; counted five `evaluate` calls in the fold-model test against "Three probes"; counted four hand-picked shapes against "Every family's"); executed: no
- Seen by: structure-prose [35], structure-prose [36]; refutation: confirmed; history: "Three probes" was accurate at a4cc1cf35 and 29d3c8f2 added the two search probes the same day without amending it; the header was already partial when 89cf4c8d created the file
- Owner-gated: no

Every test's doc comment must state its invariant accurately and no prose may hand-maintain a count or an inventory the code can change without touching it (Principle 5). The module doc enumerates roughly half the file's tests; `declared_fold_model_admits_the_log_factor_and_rejects_quadratic` says three probes go through `evaluate` where five do; `rendered_text_is_honest_and_padding_trips` says "Every family's rendered text" where the body checks four shapes (the every-family property is enforced per board text cell at measure.rs:114-116, not here).

Evidence:

       860	/// Three probes through [`evaluate`], all at the benign control's committed
       861	/// arity pair (k 256 -> 512 over a x2.19 denominator): the pre-declaration

       104	/// Every family's rendered text sits under the output-honesty ceiling, and the
       105	/// ceiling is tight enough that a text stream padded past

Resolution: Rewrite the header as the file's structure, not a tally: derivation pins (radix units, mandatory limbs, output honesty), known-bad probes through `evaluate`/`evaluate_acceptance` that must read red on exactly one leg, pins that tie tables to live axes (bench riders, the ranking pin), and the argmax kernel and near-tie rendering tests. Rewrite 860 to name the probes without a numeral; rewrite 104 as "Four representative shapes render under the ceiling", pointing at the per-cell assertion. Acceptance: no numeral count of probes in a test doc; the header names genres, not tests; adding a probe test requires no header edit.

### board-ops-render-25: An assertion message carries a run of ten spaces mid-sentence
- Where: crates/before/src/meter/board/tests.rs:73-73 (related: none)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read line 73); executed: no
- Seen by: scaffolding [13], structure-prose [45]; refutation: confirmed; history: no-rationale-found (a rewrap artifact from 93c07aa7a)
- Owner-gated: no

The plateau-versus-leaf assertion's message has an unintended whitespace run after the colon, which is the first text a maintainer reads on failure.

Evidence:

        73	        "the plateau's width is stored once but materialized per site:          stream {stream} must undercut tree {tree}"

Resolution: Collapse to a single space or break the literal with `\` continuation as the sibling messages do. Acceptance: no double space inside the message.

### board-ops-render-26: The delegating-parser pinned floors' separation from the bypass reading is prose, not a per-run check, and the table is a measured band labelled a liveness floor
- Where: crates/before/src/meter/board/tests.rs:128-143 (related: tests.rs:158-197; codec/base.rs:84-95; codec/base/limb_meter.rs:39-42; tests/meter.rs:35-53)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the test body 158-197: it computes `ops`, the derived floor, and the pinned floor, and never computes the radix site's contribution, so the doc's separation claim is unchecked per run; `git show -s bc222bd2`: "the delegating-parser bigroot floor separated nothing; re-pinned over the live bypass reading" with its body recording the bypass passing the guard by 46% and radix contributions of 250 of 752 (hugeleaf) and 4_127 of 24_399 (bigroot); base.rs:92-93 records once per parsed decimal value via `limb_meter::record_wide`, which records `bit_len.div_ceil(64).max(1)`); executed: no
- Seen by: scaffolding [11], adequacy [16], refutation's new item 2; refutation: confirmed (adding that the doc's "roughly half" clause disagrees with bc222bd2's own readings, 33% and 17%, and that the pinned values have since moved from 639/20_739 to 425/7_020, so the clause is a hand-maintained measurement statement of the genre 500d4d09 excised and is unverified against current readings); history: deliberate-but-expired (bc222bd2 knowingly left separation to prose, recording both readings and the 2.3% margin as its mitigation; 500d4d09 then excised measured readings from meter prose, so the mitigation no longer exists in the tree)
- Owner-gated: no

The pinned per-family floors are measured pipeline totals ×0.85 whose whole value rests on sitting above the pipeline-total-minus-radix bypass reading, but that separation is asserted only in the doc comment; nothing in the test computes the bypass reading, so the floor can degrade to decoration while reading green, which bc222bd2 records happening once. The doc also states a qualitative measurement ("the radix site alone contributes roughly half on both families") that its own re-pin commit contradicts, and the table is a measured band asserted with a liveness floor's message although tests/meter.rs:35-53 names the two genres apart (a measured band trips on an honest improvement; a derived floor never can). Principle 2: every criterion needs a committed demonstration that the known-bad mechanism (the radix site not recording) fails it, and the demonstration must live in the run, not in memory.

Evidence:

       132	/// The pipeline records from two sites — the delegated radix conversion (one
       133	/// width-proportional count per materialized value) and the gamma encoder's
       134	/// arithmetic — and the radix site alone contributes roughly half on both
       135	/// families, so a floor at ×0.85 of the pipeline total sits above what the
       ...
       138	/// encode-side arithmetic already covers them). The separation margin is
       139	/// measured, not structural — the floor sits over a bypass reading of
       140	/// pipeline-total-minus-radix — so a re-measure that moves the encoder site
       141	/// up must re-derive both readings before trusting the floor separates.
       142	#[cfg(feature = "limb-meter")]
       143	const DELEGATING_PARSE_LIMB_FLOORS: [(&str, u64); 2] = [("hugeleaf", 425), ("bigroot", 7_020)];

    base.rs:
        92	        #[cfg(feature = "limb-meter")]
        93	        limb_meter::record_wide(&value);

Resolution: Make the separation a per-run check inside `delegating_parser_stays_under_the_text_limb_ceiling`: compute the radix site's contribution outside measurement (the hypothesis is `stored_bases(&v).iter().map(|b| b.bits().div_ceil(64).max(1)).sum::<u64>()`, since `parse_decimal` records exactly that per spelled value; measure it rather than transcribe it), and assert `ops - radix < pinned_floor && pinned_floor <= ops`, so the floor is proven to sit strictly between the bypass reading and the live reading on every run. Delete the "roughly half" clause. Re-label the constant's doc to the genre it is (a measured tripwire with a per-run separation witness), or, structurally, give the radix delegation its own counter site and floor it at `mandatory_limbs_version(&v)`, which dissolves the table. Acceptance: setting a pinned floor to `radix - 1` fails the separation leg; the doc claims no separation it does not check and no reading it does not measure.

Construction: In the existing test, after `let ops = crate::meter::limb_ops();`, add the radix computation and `assert!(ops - radix < pinned_floor, ..)`. To demonstrate the gap today, set a pinned floor to `radix - 1`: every existing assertion still passes (`ops >= floor` trivially) while the floor would not trip a parser that deleted the radix recording.

### board-ops-render-27: Counter-reading tests omit the one-test-per-process premise their sibling suites state
- Where: crates/before/src/meter/board/tests.rs:175-177 (related: tests.rs:261-274, 434-436, 522-527, 566-568, 619-626; meter/tests.rs:21-25; tests/meter.rs:15-26; meter.rs:3549-3551)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (`grep -n -i 'nextest\|process-global\|per process\|ISOLATION' board/tests.rs` returns nothing; meter/tests.rs:24-25 defines `ISOLATION_NOTE` for exactly this); executed: no
- Seen by: instrument-correctness [51]; refutation: confirmed (a diagnosis-quality nit under the sanctioned runner, a flake under `cargo test`); history: no-rationale-found (`ISOLATION_NOTE` predates every counter-reading board test)
- Owner-gated: no

Six tests reset and read the process-global counters around a body. Under a shared-process `cargo test` they race with every concurrent test doing `Base` arithmetic or stream reads, and their failure messages would misdiagnose the pollution as a criterion breach ("width-scale work re-entered the parse path", "the criterion softened"). Principle 8: a failure that could be either a real breach or runner pollution must say so at the failure site, as `meter/tests.rs` does.

Evidence:

       175	        crate::meter::reset_limb_ops();
       176	        let parsed: Version = s.parse().expect("a displayed version parses back");
       177	        let ops = crate::meter::limb_ops();

    meter/tests.rs:
        24	const ISOLATION_NOTE: &str = "note: the counter is process-global and meaningful only one \
        25	     test per process: run under cargo nextest, not a shared-process cargo test";

Resolution: Hoist `ISOLATION_NOTE` to a shared meter test-support location (or re-declare it here) and append it to every assertion whose operand is a counter reading in these six tests; alternatively serialize the counter-reading tests behind one process-wide lock. Acceptance: every counter-comparison assertion in tests.rs carries the note or the tests hold a shared lock.

### board-ops-render-28: Six probe tests hand-build `Sample`s with per-test `PROBE_NA` constants and repeated in-function imports; the radix-work formula and a trivial wrapper are duplicated
- Where: crates/before/src/meter/board/tests.rs:305-308 (related: tests.rs:22-31, 35-38, 172-174, 314-316, 325-351, 408-457, 651-682, 771-802, 874-906, 1013-1044; measure.rs:117-118)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: full `Sample { .. }` literals at 326, 438, 657, 777, 881, 1019, each with a `PROBE_NA` at 325, 413, 655, 775, 879, 1017; in-function `use super::` blocks at 305-308, 408-411, 651-654, 771-774, 874-878, 1013-1016, of which the tests at 650, 770, and 1012 carry no cfg gate); executed: no
- Seen by: structure-prose [34]; refutation: confirmed; history: no-rationale-found (the probe tests accreted one at a time, each copying the previous one's closure and imports)
- Owner-gated: no

Each probe test defines its own `PROBE_NA` and a `sample` closure filling every `Sample` field by hand, so each tripwire's signal (two or three readings and one declaration) is buried under about twenty-five lines of defaults; the imports are repeated inside ungated test bodies although the file header explains only why the limb-meter names are gated; the `R` formula `n_io + radix_units_version + TEXT_PIPELINE_LIMB_OPS_PER_VALUE * stored_bases.len()` is copied from measure.rs at three sites; and `version_of` wraps `Packed::version` with no added meaning.

Evidence:

       305	    use super::floors::na;
       306	    use super::judge::evaluate;
       307	    use super::measure::Sample;
       308	    use super::{ByCurrency, Floors};

       325	        const PROBE_NA: &str = "probe: the limb exponent leg alone is under test";
       326	        Sample {

Resolution: One module-level `fn probe(reason: &'static str, denom: usize, readings: ByCurrency<Option<u64>>) -> Sample` with all-NA floors and defaults, and per-test one-line tweaks; hoist the shared imports to the top (keeping only the limb-meter names gated); replace `version_of(&x)` with `x.version()`; expose measure's radix-work computation as a small `pub(super) fn` or add one test-local helper. Acceptance: one `Sample {` literal in tests.rs; no `use super::` inside test bodies except cfg-gated ones; `version_of` gone.

### board-ops-render-29: A test doc quotes a dated measurement with its history and cites a design document from code
- Where: crates/before/src/meter/board/tests.rs:489-505 (related: tests.rs:862-863; ceilings.rs:54-62)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -n 'design doc\|§\|Measured\|landed' src/meter/board/*.rs` returns only tests.rs:490 and 505, so the module is clean after the deletion; `git log -S'33,036'` attributes the number to a99e8c8f7, the cure commit; ceilings.rs:56-62 read); executed: no
- Seen by: scaffolding [12], adequacy [22], structure-prose [28], instrument-correctness [52]; refutation: confirmed (adding tests.rs:862-863 "the readings that were red under the flat ceilings" as the same genre); history: contradicts-hard-rule (the global doctrine's "Never reference design documents in code comments or rustdoc" and Principle 5; the tree's own convention at ceilings.rs:56-62 set by 500d4d09, whose grep sweep of board/tests.rs missed the bracketed form; the cited document was moved to .agent-notes at 15bd905e0, so the pointer now resolves to an LLM-written note)
- Owner-gated: no

`join_all_overlap_upfront_test_reads_flat`'s doc records a past reading and the landing of a re-pin as history and cites "the design doc's §3 entry". The crate's own rule (ceilings.rs:56-62) is that readings live in pin commits, never in prose, because "a quoted reading would keep asserting itself as present-tense fact while headroom absorbed the drift"; the doctrine forbids design-doc citations from code. Lines 862-863 narrate history in the same vein.

Evidence:

       490	/// \[Measured ×2.00, 33,036 → 66,060 bits — the re-pin landed with the per-call
       491	/// index.\]

       504	/// board's `party_join_all_overlap` row carries the same reading at the scales
       505	/// of record; the cure's decision record lives in the design doc's §3 entry.

    ceilings.rs:
        57	// at the release profile of record. The readings themselves live in the pin
        58	// commits (`git log -S` the constant), never in this prose: a quoted reading

Resolution: Delete the bracketed measurement and the "§3 entry" sentence (the paragraph already states the mechanism: a linear discipline reads ×2.00 across the joint doubling, the ceiling adds rounding headroom, a per-input re-walk reads ~×4); re-word 862-863 to describe the probe's readings without "were red". Acceptance: `grep -n 'design doc\|§\|Measured\|landed\|were red' crates/before/src/meter/board/*.rs` returns nothing.

### board-ops-render-30: The acceptance criterion's doc and two kernel docs cite a retired determinism tripwire
- Where: crates/before/src/meter/board/ceilings.rs:460-462 (related: version/skyline/fill.rs:150; version/skyline/fill/tests.rs:14; shard.rs:570-612; board.rs:115-120)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'determinism tripwire' src` returns exactly ceilings.rs:462, fill.rs:150, fill/tests.rs:14; `git show -s 289e14a4`: "board: retire the determinism scaffolding and the serial reference path"; `run_acceptance` (shard.rs:570-612) read in full performs no determinism check); executed: no
- Seen by: instrument-correctness [49]; refutation: confirmed; history: contradicts-hard-rule (root AGENTS.md: nothing refers to code that no longer exists; the tripwire landed at c15daaab4/67b031aa9 and 289e14a45 retired both halves without sweeping these three sites)
- Owner-gated: no

Note: the anchors live outside this partition's five files; the claim concerns `run_acceptance`, which is in shard.rs. Deduplicate against the ceilings and skyline partitions at merge.

`LADDER_TOP_SCALE`'s doc states that campaign acceptance runs "under the determinism tripwire"; commit 289e14a4 removed both halves of that tripwire (the in-process double measurement and the gate's cross-process compare), and `run_acceptance` performs no determinism check. fill.rs and fill/tests.rs cite "the board's determinism tripwire" as a large-operand coverage instrument in the same way. board.rs:115-120 already states the determinism property as a design fact without the tripwire.

Evidence:

    ceilings.rs:
       460	/// tops out there. **Campaign acceptance is every cell green across the whole
       461	/// ladder, one acceptance invocation measuring all of it under the
       462	/// determinism tripwire**, with each exponent judged as one trend over the

    fill.rs:
       150	//! board's determinism tripwire.

Resolution: In ceilings.rs drop "under the determinism tripwire" (if the owner wants the property re-instrumented, the smoke suite's cross-shard byte-identity test is the live witness to cite). In fill.rs:150 and fill/tests.rs:14 re-denominate the coverage sentence onto what exists: the board's acceptance ladder and the envelope suite's pinned scales. Acceptance: `grep -rn 'determinism tripwire' crates/before/src` returns nothing; `just doclint` and `just citecheck` stay clean.

### board-ops-render-31: A zero denominator makes the exponent fit `NaN`, and the exponent leg then reads green on unbounded growth
- Where: crates/before/src/meter/board/judge.rs:41-44 (related: judge.rs:120-124, 363; cell.rs:204-308; measure.rs:78-123; defect.rs:40-51)
- Class / severity / confidence: correctness / nit / high
- Provenance: verified (arithmetic from judge.rs:37-54: `(0f64).ln()` is `-inf`, so `mean_x` is `-inf`, the first centered term is `NaN`, `sxx` is `NaN`, `sxx <= f64::EPSILON` is false, and the slope is `NaN`; at 363 `e > ceiling` is false for `NaN`; `spans` passes trivially; cell.rs:204-308 and measure.rs carry no positivity assert on `input_bytes`; `truncated_bytes` asserts `bytes.len() > cut`, every encoding is at least one marker byte, and text rows use `s.len()`, so no committed row can produce it); executed: no
- Seen by: instrument-correctness [54]; refutation: confirmed (a programmer-error class; the constant leg still fires on the construction); history: no-rationale-found
- Owner-gated: no

Note: the anchors live outside this partition's five files (judge.rs, cell.rs); deduplicate against the judge partition at merge.

`trend` takes `ln` of the denominator; a point with `n = 0` yields a `NaN` slope, and the exponent check `e > ceiling` is false for `NaN`, so the leg is judged and never red. No committed family produces a zero denominator, but `Cell::new`, `Cell::io`, and `Cell::text` accept it and nothing asserts otherwise. Correct for all inputs: the judge should fail loudly on a denominator it cannot fit rather than fold `NaN` into a green; the invariant belongs at construction.

Evidence:

    judge.rs:
        41	    let xy: Vec<(f64, f64)> = points
        42	        .iter()
        43	        .map(|&(n, m)| ((n as f64).ln(), (m.max(1) as f64).ln()))
        44	        .collect();

       363	        if s.exp_judged && s.exp.is_some_and(|e| e > *ceilings.get(c)) {

Resolution: `assert!(input_bytes > 0, "a board cell charges against at least one byte")` in the three `Cell` constructors (or in `measure` before the fit); optionally `debug_assert!(!slope.is_nan())` at the end of `trend`. Acceptance: a unit test constructing a `Sample` pair with `exp_denom_bytes: 0` through `evaluate` panics at the guard rather than returning a green cell.

Construction: `trend(&[(0, 1), (100, 1_000_000)])` returns `NaN`; two `Sample`s with `denom_bytes`/`exp_denom_bytes` 0 and 100 and limb readings 1 and 1_000_000 through `evaluate` yield `red` without "limb exponent" (the constant leg still fires).

## Positives

- shard.rs stages a child's whole emission in memory and writes it only after the last cell (149-154), with the pipe-stall failure it prevents stated at the site; the family-outer deal (44-63) commits its own counterexample (an op-outer deal aliasing whenever the roster length and the shard count share a factor) to prose beside the code.
- The completeness refusal in `merge_samples` (497-520) is exactly the doctrine's shape: a total check on the union that no per-capture check implies, with a committed known-bad artifact (`merge_refuses_a_silently_shrunk_grid_for_every_family`) swept over the axis the refusal discriminates on, firing at every scale including the acceptance ladder.
- tests.rs pairs every judgment leg with a probe that reads red under the leg and green without it, in both directions: the bypass walk for the floors, the chunked schoolbook for the `n_io` exponent leg, the lump ladder and quadratic ladder for the four-point trend, the ratified/regressed/improved trio for the capacity band, the straddling pair for the heap guard, the fold model's log factor against a quadratic and a fat constant. Each goes through `evaluate` itself, not a description.
- `worst::rank` records exact ties whole and name-sorted so the pin cannot flap, and the near-tie flag is kept out of the pin on a stated argument (73-79); `render.rs` prints `-.--` for an unjudged exponent rather than digits a reader would mistake for a measurement (74-81).
- ops.rs asserts output honesty at prepare on the I/O-denominated rank rows (`rank_encode` 582-586, `ranked_encode` 707-712, `ranked_encode_rank` 738-742) with the bound's derivation inline, so padding the output side of a denominator trips the run rather than greening a cell.
- Floats cross the shard wire as IEEE-754 bit patterns (184-188, 283-288) with the reason stated, and the merge refuses any header that is not byte-for-byte the commissioned one, so the byte-identity the smoke suite asserts across shard counts is exact by construction; `required-features` on the example makes an unfeatured board a cargo error rather than a matrix that looks judged.
- Every `expect`/`unreachable!` message in the five files is a one-line proof (shard.rs:193 "a cell's applicability depends on the family, never the size"; shard.rs:507 "FamilyId::board() filters on the Board coverage answer"; ops.rs:183-184 the envelope-only derivation).
- `measure()` meters exactly the body: counters reset, heap baseline taken after the reset, the boxed result kept alive until every meter is read, I/O denominators settled from the actual result.

## Open questions for Finch

1. The segments column (finding 15): dissolve it from the board, or keep it with a disclosure that the counter has no writer outside the lib's test build? Recommendation: dissolve, and re-state `LADDER_TOP_SCALE`'s rationale on the doubling-chain onset (worst.rs:65-66 already names it); handle tests/meter.rs's segments pins in the same change.
2. The worst-case ranking pin: keep it as a gate leg, computed from the acceptance sweep (finding 19), or retire it to an audit view like the fuelscape atlas? Recommendation: keep it. f34f4b34 records a catch (a heap movement on freeze-parade and bigroot) that the envelope suite's limb-only lower bounds would not have surfaced, so it is not circularly justified; the tied-set rows that enumerate the roster do carry information (a family leaving a saturated tie is a reading change), so leave them.
3. `designed()`'s destination (finding 2): family.rs beside the bundle-build match (internal, matches registry.rs:569-571), or a field on the public `Coverage::Board` (exposes `OpGroup` under the `meter` feature)? Recommendation: family.rs.
4. The shard codec: an optional `serde_json` dependency under the `meter` feature, or keep the hand-rolled positional codec and add the tamper table (finding 16)? Recommendation: keep the codec; its refusals are inseparable from its parsing and the protocol is runner-internal; the tamper table is the actionable gap.
5. Membership and covers rows (finding 9): replace the early-exit probes with certifying ones, or add certifying rows beside them? Recommendation: replace (the admit and disjoint verdicts are O(1)-class on most families and `party_disjoint` already prices the full-examination disjoint walk); re-pin the two `WORST_RANKINGS` rows with the movement annotated.
6. The ascend-cliff under-side (finding 6): a banded floor like the capacity model, or a class-liveness pin like the mirror-wide model? Recommendation: a class pin, since both constants are documented as "conditional on exactly this flat-constant profile" and a pin reads red the day the profile changes.
7. Should `query_coverage` take the placement touch floor (finding 10) or keep an NA with a positively stated reason? Its two-probe walk has clamp legs the placement premise does not cover; recommendation: derive separately and state whichever answer positively.

## Dropped

- [6] Hand-rolled TSV wire codec where a derive would do: refuted on the merits (refusals interleaved with parsing; a new library dependency for an internal protocol); moved to open question 4.
- [4] The WORST_RANKINGS pin as a dissolution candidate: refuted (f34f4b34 is a demonstrated catch outside the pin, so the justification is not circular; several counted "re-pins" add rows or families); the surviving sweep-sharing point is finding 19 and the retention question is open question 2.
- [23], [55]: duplicates of finding 1.
- [24], [33]: duplicates of finding 2.
- [29], [30]: merged into finding 4.
- [27], [48]: merged into finding 12.
- [8], [37]: merged into finding 14.
- [21], [50]: merged into finding 16.
- [26]: merged into finding 19; its "66.7 s pin leg" timing is not in the tree and is dropped as evidence.
- [13], [25], [38], [53]: merged into finding 20.
- [36]: merged into finding 24.
- [11]: merged into finding 26.
- [22], [28], [52]: merged into finding 29.
- Refutation new item 1 (registry.rs:569-571 doc/code mismatch): folded into finding 2 as its lever.
- Refutation new item 2 (the "roughly half" clause): folded into finding 26.
- Refutation new item 3 (recurse.rs:74-75 "always written"): folded into finding 15's evidence.
- Refutation new item 4 (`# Panics` finiteness omission): folded into finding 17.
