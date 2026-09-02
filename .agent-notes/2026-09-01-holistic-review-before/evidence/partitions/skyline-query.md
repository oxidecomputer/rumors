# Partition skyline-query: The linear functionals and projection: query, integral, web, and the query test suite

## Partition summary

This partition is the query surface of the skyline codec: `query.rs` (653 lines) holds the five folds (`rank`, `distance`, `lag`, `rank_cmp`, `min_ticks`, `project`) and the shared pair co-sweep `pair_fold`; `query/integral.rs` (1164 lines) is the anchored-segment integral every rank-family fold runs on (the `h* = B + P + L` height split, the freeze trigger, the promotion ledger, the mass-balanced product-tree settle, and the funding certificate); `query/web.rs` (422 lines) is the min_ticks fold's bookkeeping (reign records over the shared `watermark::MinWeb`, the epoch ledger settled by summation by parts, and the word-scale `mul_into` settle move); `query/tests.rs` (2704 lines) is the sibling test suite: differential pins against the recursive tree oracle, the composed kernels, and the semantic Riemann sum, plus a `limb-meter`-gated `adequacy` module of eight committed known-bad kernels rostered by `tests/superlinear_tripwires.rs`. Total read: 4943 lines across the four files, plus the supporting ranges the findings cite (`codec/base.rs`, `codec/bits.rs`, `codec/buf.rs`, `overlay.rs`, `signed.rs`, `rank.rs`, `int.rs`, `testing/generators.rs`, suanpan's `accumulator.rs`, `tools/covcheck-expected.json`, the justfile's coverage section, `tests/meter.rs`, the fuzzfit bands, the board family constants, `.cargo/mutants.toml`, and dashu-int 0.5.0's threshold constants). `tests.rs` is the only test file.

The production code is in good shape. One `Integrator` serves rank, distance, lag, and rank_cmp through a single `orientation: impl Fn(Ordering) -> i8` closure, each shipped closure a three-line transcription of one row of the module doc's sigma table; the settle tree runs on an explicit `Step::{Open, Merge}` stack and states why it is not routed through `crate::fold`; `web.rs` reuses `MinWeb<P>` at payload `Reign` rather than re-implementing the anchored-minimum web; every public fold carries a uniform `# Panics` section; every charge in both funding certificates names its deposit; the cfg-gated meter taps compile to nothing without `limb-meter` and are called unconditionally. I traced every `expect`, `unreachable!`, index, and integer conversion and every fold against the callee contracts; nothing panics on decoded input except the one `u32` epoch narrowing reported below, and no library walk recurses on depth.

The dominant issue is in the test suite: the adequacy module hand-copies the shipped `rank` loop four times, the `pair_fold` loop twice, `Integrator::finish` twice, and the `settle_armings` reduction twice, each documented as the shipped body "verbatim", and three commits on one day changed the shipped bodies without mirroring the copies, so every "verbatim" claim is false today. The remaining findings are smaller: a public `O(M(|v|) · log |v|)` clause with no committed instrument at the tier where it applies (disclosed in the doc itself), a `u32` epoch narrowing reachable by input size, several comments whose premise the codec now denies or whose rationale expired (the storage cap, the `scale` alias, the `arb_base` ceiling, the `one` field), a test whose doc claims a regime the body does not observe, and a set of vocabulary and hand-count items, several of which are slices of crate-wide patterns and belong to one crate-level ruling rather than to this partition's worklist.

## Findings

### skyline-query-1: Unanchored crate-dialect terms "seam" and "genre" in this partition's prose
- Where: crates/before/src/version/skyline/query.rs:121-123 (related: query.rs:139, 506, 603; query/integral.rs:327, 362, 754, 1024; query/tests.rs:266, 500, 524, 644, 790, 892, 893, 994, 1074, 1137, 2229, 2424)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep of both words over the four files and over crates/before/src); executed: no
- Seen by: prose; refutation: reframed (crate-wide dialect, not a partition coinage); history: no-rationale-found (never defined by contrast; `genre` is also used by the campaign configuration of record)
- Owner-gated: yes: a crate-level vocabulary ruling (176 word-matches of "seam" and 143 of "genre" in crates/before/src)

"seam" is used for a cfg switch, a cluster boundary, a product-tree node split, the internal-entry test level, and a backend dispatch threshold; "genre" is an undefined synonym for a class of shapes beside the anchored `FamilyId`. The review's vocabulary rule asks that a coined term be anchored to an identifier or defined once by contrast; neither word is, anywhere in the crate.

Evidence:

    121  //! dense-suffix bands hold the many-freezes and many-armings genres flat, and
    122  //! the `ledger_wide_arming` and `answer_embedded_product` bands hold the wide ×
    123  //! dense genres flat per byte in the fold's own traffic.

    (integral.rs:327)  /// The cluster seam of the settle products: within a cluster the digits densify
    (tests.rs:892)     /// product past the backend's simple→Karatsuba dispatch seam, let alone

Resolution: Rule once at crate scope. If the words stay, define each once by contrast at one home (the crate's vocabulary section or the first use) and link to it; if they go, substitute the specific noun at each site ("cfg switch", "cluster boundary", "node split", "kernel-level", "dispatch threshold"; "family" where a registered family is meant, "case" or "class" otherwise). Acceptance: either `grep -n -E '\bseams?\b|\bgenres?\b'` over the partition returns nothing, or every remaining use is the one sense a single definition anchors.

### skyline-query-2: `let scale = max_depth` aliases carry a comment justifying a conversion that no longer exists
- Where: crates/before/src/version/skyline/query.rs:203-205 (related: query.rs:334-337, 399-400; rank.rs:510)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`git show 05d87e1b` shows `let scale = max_depth as u64;` becoming `let scale = max_depth;`; `Rank::from_raw(num: Base, exp: u64)` at rank.rs:510; `fn max_depth(..) -> u64` at query.rs:636); executed: no
- Seen by: structure, prose; refutation: confirmed; history: deliberate-but-expired (6323d667 wrote the comment for a `u32 -> u64` conversion; 05d87e1b removed the conversion and left the alias and comment)
- Owner-gated: no

The comment argues that depth fits the `u64` rank exponent, but `max_depth` already returns `u64` and `Rank::from_raw` takes `u64`, so the comment documents what the types show and the alias names nothing (Principle 5: no dated rationale at a declaration site).

Evidence:

    203      // Depth counts levels of a stream held in memory, so it always fits the u64
    204      // rank exponent.
    205      let scale = max_depth;

Resolution: Pass `max_depth` (and `overlay_depth` in `pair_fold`) straight to `Rank::from_raw` and the return tuple; delete both aliases and both comments. Acceptance: `grep -n 'let scale' crates/before/src/version/skyline/query.rs` is empty.

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

### skyline-query-4: Em-dashes in `//` comments and in one assert message
- Where: crates/before/src/version/skyline/query.rs:388-390 (related: query.rs 169, 170, 349, 369, 425, 434, 507, 526, 527, 575, 604; query/integral.rs 464, 465, 862, 870, 871, 906, 1071, 1073, 1082, 1093; query/web.rs 131, 138, 255, 305; query/tests.rs 258, 755, 758, 1166, 1517, 1520, 1856, 1858, 2220, 2223, 2227, 2432, 2434, 2435, 2442, and the string at 2423)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -E '^\s*//[^/!].*—'` over the four files: 41 lines; over crates/before/src: 374 lines); executed: no
- Seen by: prose; refutation: reframed (the crate's house style, 374 lines in 76 files); history: no-rationale-found (nine of the partition's lines postdate the 2026-08-10 doctrine line; nothing mechanical checks it)
- Owner-gated: yes: a crate-level style ruling

The owner's register rule asks for colons or semicolons over em-dashes in comments and log messages, and spaced double-hyphens in code comments. The partition has 41 such `//` lines and one assert string (tests.rs:2423) that reaches the terminal.

Evidence:

    388          // a boundary with no step is programmer error — and a fabricated
    389          // funded width would silently misprice the trigger, so the violation
    390          // fails loudly instead.

    (tests.rs:2423)               through the settle — in both cases the dense-suffix flatness \

Resolution: Rule once at crate scope; if the doctrine applies, sweep `//` comments to ` -- ` or restructure with a colon, and use a colon or semicolon in the tests.rs:2423 string. Acceptance: the grep returns nothing over the partition and no string literal passed to `assert!`/`expect`/`eprintln!` contains an em-dash.

### skyline-query-5: The freeze trigger predicate is spelled twice, in `min_ticks` and `Integrator::boundary`
- Where: crates/before/src/version/skyline/query.rs:456-458 (related: query.rs:180; query/integral.rs:81-98, 264-274, 846-850; query/tests.rs:1443, 1587, 1924)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read both predicates; `FREEZE_ALLOWANCE_DIGITS` is imported into query.rs at line 180 and used only at 456); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (both spellings landed together in 24a448cf; the mutants roster records a performance-genre disposition for `Integrator::boundary`'s `>` leg only, with no twin entry for query.rs:456)
- Owner-gated: no

The integral module doc describes one freeze trigger and calls its engagement boundary "a tuning choice inside the deliberate cost allowance"; the predicate is implemented independently in `min_ticks` and `Integrator::boundary`, so a tuning can move one and not the other, and nothing pins them equal. The mutants roster already treats the two copies asymmetrically.

Evidence:

    456          if live.digit_count() > int_digits(&step.magnitude) + FREEZE_ALLOWANCE_DIGITS {
    457              ledger.freeze(&mut live);
    458          }

    (integral.rs:847)          if self.live.digit_count() > funded_digits + FREEZE_ALLOWANCE_DIGITS {

Resolution: Add `pub(super) fn freeze_due(live: &Accumulator, funded_digits: usize) -> bool` beside the constant in integral.rs; call it from `Integrator::boundary`, `min_ticks`, and the three test kernels; stop exporting `FREEZE_ALLOWANCE_DIGITS` to query.rs. Acceptance: `grep -n 'FREEZE_ALLOWANCE_DIGITS' crates/before/src/version/skyline/query.rs` is empty and the predicate expression appears once in production code.

### skyline-query-6: `ReignWeb::leaf` relies on a two-call protocol at each call site that the callee could own
- Where: crates/before/src/version/skyline/query.rs:463-470 (related: query.rs:438-439; query/web.rs:282-296, 374-377)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read both leaf sites and `ReignWeb::leaf`; the `epoch` argument is always the ledger's current epoch and `leaf` already receives `&mut ledger`); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (the pairing dates to 24a448cf and survived the c29bd2b3 rebuild without comment)
- Owner-gated: no

Both leaf sites call `ledger.leaf_ref()` immediately before `web.leaf(...)` and pass an `epoch` that is always `ledger.epoch()` while also passing `&mut ledger`. The invariant "every leaf counts one reference in the epoch it is recorded under" is held by pairing two calls at each site instead of inside the one method that has both operands.

Evidence:

    463          ledger.leaf_ref();
    464          web.leaf(
    465              leaf_sign,
    466              &leaf_offset,
    467              ledger.epoch(),
    468              &mut total,
    469              &mut ledger,
    470          );

Resolution: Change `ReignWeb::leaf` to `(&mut self, sign, offset, total, ledger)`; inside, `ledger.leaf_ref(); let epoch = ledger.epoch();` before the closures capture the ledger; make `EpochLedger::leaf_ref` private; the first-leaf call becomes `web.leaf(Sign::Positive, &Base::ZERO, &mut total, &mut ledger)`. Acceptance: `grep -n 'leaf_ref' crates/before/src/version/skyline/query.rs` is empty; `assert_single`'s min_ticks legs stay green.

### skyline-query-7: Long qualified paths where the sibling items are imported; a one-line `version_of` wrapper
- Where: crates/before/src/version/skyline/query.rs:547 (related: query.rs:185, 499, 599; query/tests.rs:35-38, 46, 303, 320-321, 345-346, 1409, 1527, 1699, 1749, 1865, 2063, 2113, 2234-2236, 2370, 2603)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep counts in tests.rs: `crate::codec::built_view` 40, `version_of(` 43, `crate::codec::Base` 9; `super::super::integral::int_digits` spelled in full at six sites while imported at 1409; party/ops/sum_split.rs:1 imports `built_view`); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: no-rationale-found (5d167a63's mechanical migration introduced all 40 `built_view` spellings; `version_of` reduced to `p.version()` at faf3cd0a and the wrapper stayed)
- Owner-gated: no

`project` spells `super::signed::gamma_code_signed_int` twice while query.rs:185 imports six items from `super::signed`, and takes `&crate::Party` beside `use crate::Rank`. In tests.rs the `crate::codec::built_view` prefix is the dominant token on most assertion lines, `version_of` wraps `Packed::version()` one-to-one, `crate::codec::Base` appears in signatures whose bodies then `use crate::codec::Base;`, and the `adequacy` module places `use` lines mid-module by section. Doctrine: imports over long qualified paths except where the qualification informs.

Evidence:

    547                      super::signed::gamma_code_signed_int(Sign::Positive, &Int::ZERO),

    (query.rs:185)  use super::signed::{fold_signed, fold_signed_int, gamma_code_int, signed_sum_int, Sign, Signed};
    (tests.rs:36-38)
      36  fn version_of(p: &Packed) -> Version {
      37      p.version()
      38  }

Resolution: Add `gamma_code_signed_int` to the `super::signed` import and `Party` to `use crate::{Party, Rank}`; in tests.rs hoist `use crate::codec::{built_view, Base, BitsBuf};` and the adequacy module's imports to the module heads, replace `version_of(&x)` with `x.version()`, and use the imported `int_digits`. Acceptance: `grep -c 'crate::codec::built_view' tests.rs` is 0 or 1 (the import), `grep -c 'version_of(' tests.rs` is 0, and query.rs has no `super::signed::` or `crate::Party` at use sites; `just fmt` and `just clippy` clean.

### skyline-query-8: `pub(crate) mod integral` is wider than any use
- Where: crates/before/src/version/skyline/query.rs:649-650
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`grep -rn 'query::integral\|query::web' crates/before --include='*.rs'` outside src/version/skyline/query/ returns nothing); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-and-holds (a31062ae demoted the then-`pub` machinery modules to `pub(crate)` in one sweep; the rule it states is "pub exactly where code outside the crate path-names it", so narrowing further is consistent with the rule, not required by it)
- Owner-gated: no

Nothing outside the `query` directory names `query::integral`; the test module is a child of `query`, so a private `mod integral;` still exposes its `pub(super)` items to it. The sibling `mod web;` is private on the next line, and the asymmetry invites a reader to look for a crate-level consumer that does not exist.

Evidence:

    649  pub(crate) mod integral;
    650  mod web;

Resolution: `mod integral;`. Acceptance: the grep stays empty and the crate builds with `--all-features`.

### skyline-query-9: The log-factor clause of the public `O(M(|v|) · log |v|)` contract has no committed instrument at the tier where it applies
- Where: crates/before/src/version/skyline/query/integral.rs:220-229 (related: query/integral.rs:243-254; before-fuelscape/src/ops.rs:360, 602, 616; before/src/version.rs:289-293; tests/meter.rs:5220; fuzzfit/harness/src/bands.rs:892-903; src/meter/board/family.rs:285-300)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read the fuelscape contracts, `WIDE_ARMING_SMALL = 500`, `ff_version_rank`'s `max_denom: 8767`, and the board's ~13 KiB wide-arming probe); executed: no
- Seen by: claims; refutation: reframed (fuel counts the backend's instructions, so "no deterministic counter can see it" is false; a wall-time two-scale pair cannot resolve a log factor); history: already-known (the gap is declared at integral.rs:223-225 and in the adversarial note; "not commissioned" there refers to removing the log, not witnessing it)
- Owner-gated: yes: whether a declared, argued, uninstrumented public clause is acceptable, and whether a bench-only or CI-only witness enters gate policy

The public `# Complexity` for `Version::rank`, `distance`, and `lag` is `O(M(|self|) · log |self|)`. The argument is in this doc; every committed two-scale instrument sits below the 4,000-word tier where the log factor applies (meter bands at WA(500..1000), fuzzfit to 8.7 KiB, the board probe at ~13 KiB), and the doc says so. Under the crate docs' "any violation is a bug" standard an asymptotic claim needs an argument, a matching implementation, and a committed instrument that fails when it is false; the third is absent for exactly the clause that distinguishes the contract from `O(M)`. The gap is disclosed, which is why this is low rather than higher.

Evidence:

    220  //! The shipped backend dispatches power-law tiers up to 4,000-word operand
    221  //! sides (~32 KiB parked sums per side). Past that its quasilinear tier's
    222  //! per-level costs stop telescoping, and the settle pays at most one extra
    223  //! tree-depth factor, `O(M(|v|) · log |v|)` — and the log factor is tight there
    224  //! [derived; a committed witness at this scale would need 65 KiB+ packed
    225  //! operands]:

    (ops.rs:360)          contract: "`O(M(|self|) · log |self|)` time, `O(|self|)` space",

Resolution: Either instrument or narrow. To instrument: a multi-scale fuel fit (the fuzzfit harness already counts wasm fuel for `ff_version_rank`) over the doc's tight construction at three or more scales past 65 KiB, judged against the `M(n) · log n` model with `M ≈ n log n`; or a deterministic check that records `meter_product`'s operand widths per tree level on that construction and holds the per-level product widths to the model's telescoping. To narrow: state in the public contract only what committed instruments pin and move the quasilinear-tier remark to a decision record. Acceptance: either a committed cell or test whose operands' parked sums exceed 4,000 words at every scale, named from integral.rs in place of the "[derived; ...]" bracket and rostered, or the public `# Complexity` no longer carries the unwitnessed clause.
Construction: Build the doc's own worst case: `Θ(log |v|)` armings whose parked widths grow as `4,000 · 2^i` words, each banked ahead of a trailing window of span `Θ(|v|)`, at |v| of about 65 KiB, 130 KiB, and 260 KiB packed; run `Version::rank` under the fuzzfit fuel harness; fit the exponent across the three scales against `n log^2 n`. A settle that re-ran an NTT-tier product once per tree level without telescoping reads the extra `log` here and in no committed counter.

### skyline-query-10: `mass_split`'s doc overstates the right half's bound in the clamped case
- Where: crates/before/src/version/skyline/query/integral.rs:292-294 (related: query/integral.rs:301-304; query/tests.rs:1334-1335)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (hand computation of the construction against integral.rs:302-303); executed: no
- Seen by: correctness; refutation: confirmed, and adds that tests.rs:1334-1335 repeats the clause while the test asserts only nonempty halves and the depth bound; history: no-rationale-found (982bd260 derived "right half <= floor(M/2)" without the clamp case)
- Owner-gated: no

The doc says "the right half is at most half the node's mass", but when no prefix inside `(lo, hi)` reaches the target the `.min(hi - 1)` clamp makes the right half a single heavy leaf exceeding half. The pinned depth bound survives (that half is one leaf and the recursion ends there), and the preceding clause ("within half the node's plus one leaf's") is accurate; only this lemma is false, and a maintainer re-deriving the bound from the doc would reach a false step (Principle 5).

Evidence:

    292  /// internal node's range; the returned `mid` satisfies `lo < mid < hi`. Each
    293  /// half's mass stays within half the node's plus one leaf's: the right half is
    294  /// at most half the node's mass, and the left half exceeds half only by mass

    (tests.rs:1334-1335)
    1334      /// naive recursive reference expanding the same rule: the right half
    1335      /// never exceeds half the node's mass, and the left half exceeds it only

Resolution: Amend both sentences: "the right half is at most half the node's mass unless the clamp made it a single leaf, in which case it is that leaf and the recursion ends there". Acceptance: the doc's statement holds on masses `[1, 1, 100]`.
Construction: `mass_split(&[0, 1, 2, 102], 0, 3)`: `target = (0 + 102).div_ceil(2) = 51`; `prefix[1..3] = [1, 2]`, `partition_point(p < 51) = 2`; `lo + 1 + 2 = 3`, clamped by `min(hi - 1 = 2)` to `2`; the right half `[2, 3)` has mass 100 > 51.

### skyline-query-11: `clusters`' "ascending" precondition must be strict; the gap subtraction underflows on a repeated index
- Where: crates/before/src/version/skyline/query/integral.rs:343 (related: query/integral.rs:324-325, 441, 504-507, 603, 699-711)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read `clusters`, the three doc sites, `charge_digits`' debug_assert, and `WindowMass::combine`, whose per-position merge emits at most one entry per index); executed: no
- Seen by: correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`rest[end].0 - rest[end - 1].0 - 1` underflows when two entries share an index (a debug panic; in release it wraps to `u64::MAX`, exceeds any `gap_limit`, and splits, which stays value-correct). Production never repeats an index because `combine` emits one entry per position, so the panic is programmer-error-only, but the precondition that makes it so is stated as "ascending" without "strictly" at three sites, and `charge_digits`' debug_assert checks digit range, not ascent. Tests already hand-build digit runs.

Evidence:

    343          while end < rest.len() && rest[end].0 - rest[end - 1].0 - 1 <= gap_limit {

    (integral.rs:603)  /// Each entry is `(digit index, digit)` with `0 < |digit| ≤ 2^31`, ascending;

Resolution: Say "strictly ascending" at 324-325, 441, and 603, and extend `charge_digits`' debug_assert loop to check each index exceeds its predecessor. Acceptance: `charge_digits(&mut acc, Sign::Positive, &factor, &[(3, 1), (3, 1)])` fails the debug assertion with a message naming strict ascent rather than an overflow panic inside `clusters`.
Construction: `clusters(&[(3, 1), (3, 1)], 5)` in a debug build: `rest[1].0 - rest[0].0 - 1 = 3 - 3 - 1` underflows and panics "attempt to subtract with overflow".

### skyline-query-12: Moralized and significance wording: "honest", "real", "is the point"
- Where: crates/before/src/version/skyline/query/integral.rs:356-359 (related: query/integral.rs:39, 142, 272, 399; query/tests.rs:671, 994, 1317, 1394, 2379)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep of `\b(honest|real)\b` and `is the point` over the four files; `honest` has 101 word-matches in crates/before/src); executed: no
- Seen by: prose, correctness; refutation: reframed ("exactly" is mostly precise usage and needs only a light pass); history: no-rationale-found (all sites predate the 2026-08-19 rule; the owner's own rulings use "honest", so the sweep should be owner-confirmed)
- Owner-gated: yes: "honest" is the owner's own idiom in recorded rulings, so a crate-wide sweep needs his confirmation

The mechanism is called "honest" at three sites and work or a regime "real" at four; "X is the point" recurs three times. The vocabulary rule asks for the property in place of the moral: "when the settle delegates each cluster whole", "span-scale work", "value-exact against the shipped rank". integral.rs:272's "than any real stream holds" is also a likelihood argument where the doctrine wants the work bound (reaching 8 digits of slack from unit codes takes about 2^224 folds).

Evidence:

    356  /// shim — the `parse_decimal` convention — so the counters price the traffic
    357  /// the fold moves (operand reads, the product's width) and stay linear when the
    358  /// mechanism is honest: a settle that multiplied too often, or densified across
    359  /// an unfunded gap, would push this very tap superlinear.

    (integral.rs:39)   //! The anchoring is the point. A freeze must not settle evicted drift against
    (tests.rs:1394)    /// shipped rank, so the demonstrator is a real implementation, not a strawman.

Resolution: Name the property at each site; drop "is the point" and state the claim; restate integral.rs:272 as the work bound. Acceptance: `grep -n -E '\b(honest|real)\b|is the point'` over the partition returns nothing.

### skyline-query-13: Shift panic-freedom argued from a storage cap the codec denies
- Where: crates/before/src/version/skyline/query/integral.rs:464-471 (related: query/web.rs:136-141; codec/bits.rs:117-119; codec/buf.rs:25-31; overlay.rs:114-116)
- Class / severity / confidence: claim / low / high
- Provenance: verified (`grep -rn "caps below 2^32" crates/before/src` finds only these two comments); assessed (read the three codec statements); executed: no
- Seen by: correctness; refutation: confirmed; history: deliberate-but-expired (7ea3df58 wrote both comments while the borrowed view's 32-bit length encoding still capped walks; 5d167a63 and 83e61b4d removed the cap the same day and 05d87e1b states the new fact, without re-denominating these two comments)
- Owner-gated: no

Both comments prove the accumulator shifts cannot reach the documented `usize` panic on the premise "the storage caps below 2^32", and the codec states the opposite: `Bits::freeze` "imposes no bound of its own", `BitsBuf` says "allocatable memory is the only bound anywhere on the build path", and `PlateauCursor::depth` says live lengths outgrow a 32-bit `usize` from 512 MiB. The conclusion survives on the real bound (on a 32-bit target allocatable memory keeps the stream under 2^35 bits, so digit positions stay under 2^30 against the accumulator's 2^32-digit `usize` bound, a margin of two binary orders, not "multiple"; on 64-bit `usize` is 64 bits wide and the panic is unreachable outright). A panic-freedom argument is a one-line proof (Principle 1), and this one's premise is false.

Evidence:

    464      // Every accumulator shift in this module — the `32 * index` digit
    465      // routings here, the interval weights and segment scales below — is
    466      // bounded by the walked stream's own content: digit indexes by a
    467      // value's width over 32, weights by the tree's depth, both under the
    468      // stored stream's bit length, which the storage caps below 2^32. The
    469      // shifted entry points' documented panic (a digit position past
    470      // `usize`, from shift 2^37 on a 32-bit target) therefore sits multiple
    471      // binary orders of magnitude beyond anything this fold can feed it.

    (bits.rs:117-119)
    117      /// Exact at every size on every target: lengths and positions are `u64`
    118      /// on both sides of this seam, so an emission is storable whenever its
    119      /// buffer is allocatable — the door imposes no bound of its own.

Resolution: Restate the premise at both sites in the codec's terms: the shift is bounded by the stream's bit length, itself bounded by allocatable memory, under 2^35 bits on a 32-bit target, so a digit position stays under 2^30; on 64-bit targets the documented panic is unreachable. Drop "multiple". Acceptance: the grep returns nothing and both comments name the bound bits.rs states.
Construction: Textual: the premise is contradicted by bits.rs:117-119 and buf.rs:25-31 as written. For the runtime side, on a 64-bit target a right spine of 2^31 unit leaves (about 1.6 GiB of stream) passes `Version::from_bits` and every shift in `rank` and `min_ticks` still fits, which is the correct argument the comment should carry.

### skyline-query-14: Encoding conventions in the settle kernel that a named form would make evidently right
- Where: crates/before/src/version/skyline/query/integral.rs:501-525 (related: query/integral.rs:665-720; query/tests.rs:859-868)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read `charge_digits` and `WindowMass::combine`); executed: no
- Seen by: structure; refutation: confirmed (correct, pinned, taste-level); history: no-rationale-found
- Owner-gated: no

`charge_digits` densifies into `parts: [(Vec<u8>, bool); 2]` indexed by `usize::from(digit.is_negative())` and later tests `sign.is_negative() == (side == 1)`, so the reader carries "index 1 means negative" across 25 lines; the differential oracle in tests.rs:859-860 spells the same two images as `positive` and `negative`. `WindowMass::combine` picks the next merged index with a `u64::MAX` sentinel and three conditional mins and breaks on the sentinel, where an `Option` min over the three candidates says the same thing without a magic value. Both are correct; the owner's bar for finished kernel code is that it read as evidently right.

Evidence:

    501          let mut parts = [(vec![0u8; span * 4], false), (vec![0u8; span * 4], false)];
    509              let (image, live) = &mut parts[usize::from(digit.is_negative())];
    520              if sign.is_negative() == (side == 1) {

    672              let mut index = u64::MAX;
    682              if index == u64::MAX {
    683                  break;
    684              }

Resolution: Name the two images (two locals with a `live` flag each, routed by `match digit.is_negative()`) and iterate `[(Sign::Positive, &positive), (Sign::Negative, &negative)]` so the add/sub test reads `sign == image_sign`; in `combine`, compute the next index as the `min` over `[carry.then_some(carry_index), old.peek().map(..), new.peek().map(..)].into_iter().flatten()` and `let Some(index) = .. else { break }`. Acceptance: the clustered-charge proptest, the tier-boundary test, and `densify_tap_prices_the_cluster_span` stay green; no `u64::MAX` sentinel or `side == 1` comparison remains.

### skyline-query-15: `Integrator.one: Base` stands in for suanpan's `add_u64_shl`, which `before` never calls
- Where: crates/before/src/version/skyline/query/integral.rs:578-581 (related: query/integral.rs:771-783, 806; suanpan/src/accumulator.rs:418-424, 1194-1196, 1493; codec/base.rs:42-44, 275-277; query/tests.rs:1431, 1437, 1544, 1568, 1880, 1905)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn 'add_u64_shl\|sub_u64_shl' crates/before` returns nothing); assessed (read the dispatch: `add_magnitude_shl` maps `to_word() == Some(word)` to `add_shifted_word(word, false, shift)`, the same call `add_u64_shl` makes; `Base::to_word` is `to_u64`, which records no limb count); executed: no
- Seen by: structure, claims; refutation: confirmed; history: deliberate-but-expired (the field predates `add_u64_shl` by eight days; a736ef14 wrote its current doc after the entry point existed without weighing it)
- Owner-gated: no

The field exists so `interval` can borrow a ready `&Base` for the unit deposit. suanpan's `Accumulator::add_u64_shl(1, weight_shift)` is value- and touch-identical (same `add_shifted_word` path, unmetered `to_word`), so the field, its doc, and its initializer vanish; with `one` gone every field has a `Default` and `Integrator::new` can derive. The three test integrators copy the same field.

Evidence:

    578      /// The unit mass every interval deposits at its own scale; a constant,
    579      /// held on the struct so the per-interval deposit borrows a ready
    580      /// `&Base` instead of building one per interval.
    581      one: Base,

    806              self.segment_mass.add_magnitude_shl(&self.one, weight_shift);

Resolution: Replace line 806 with `self.segment_mass.add_u64_shl(1, weight_shift);`, delete the field and its initializer, `#[derive(Default)]` the struct (keep `new()` as `Self::default()` or drop it), and apply the same to the test kernels' `position`/`segment_mass` deposits. Acceptance: `grep -rn 'one: Base' crates/before/src/version/skyline/query*` is empty; `just test-all` green with every `skyline_rank_*`, `DISTANCE_*`, and `LAG_*` envelope row unchanged.

### skyline-query-16: `Arming` (a ledger entry) collides with the watermark web's "arming" of a range, with no contrast drawn
- Where: crates/before/src/version/skyline/query/integral.rs:584-587 (related: query.rs:108, 131; query/web.rs:30, 302-303; watermark.rs:105)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (grep: watermark.rs:105 is the section header "# The arming paths"; `struct Arming` at integral.rs:587); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the web sense predates; `struct Arming` arrived in ceb9f330; the lexicon re-ruling in a736ef14 did not notice the collision)
- Owner-gated: no

In `integral.rs` and the `query.rs` module doc an "arming" is one promotion recorded in the ledger; in `web.rs` and `watermark.rs` "arming" is pushing a pending range's boundary onto the web. The two senses live under one parent module and neither definition site mentions the other, so a reader who learns the ledger sense reads `web.rs`'s "range arms above it" with the wrong referent. The vocabulary rule asks for a definition by contrast where a neighbor shares the word.

Evidence:

    584  /// One promotion, recorded at its freeze and settled once at the sweep's close:
    585  /// the promoted parked component and the window of interval mass that separates
    586  /// it from the previous promotion.
    587  pub(super) struct Arming {

    (web.rs:30)  //! range arms above it — rides the interrupting boundary as its payload and

Resolution: Either one sentence of contrast at `Arming`'s definition ("an arming here is a ledger entry; the watermark web's arming of a pending range is unrelated") and the mirror sentence in web.rs's module doc, or rename the struct to `Promotion` (its own doc's noun; the field is already `promotions: Vec<Arming>`), leaving the registry family names untouched. Acceptance: each module's first use of "arming" is unambiguous to a reader of that module alone.

### skyline-query-17: The jump term's hard assert has no committed demonstration that it fires
- Where: crates/before/src/version/skyline/query/integral.rs:831-838 (related: query.rs:308-331)
- Class / severity / confidence: verification-gap / nit / medium
- Provenance: verified (grep for `should_panic` and "change term is a debit" under the partition finds only the assert itself); executed: no
- Seen by: correctness; refutation: confirmed (the downgrade alternative is off the table after f77011e3); history: no-rationale-found (f77011e3 promoted the assert under an owner direction and added no driver; the mutants roster has no entry for `jump`)
- Owner-gated: no

The assert is the only observer of `pair_fold`'s closure contract (every shipped closure is monotone, so no committed test reaches the failure), and it is a release-mode panic by owner ruling. Principle 6 asks for a committed demonstration that a known-bad mechanism fails each criterion; the cheap one is a `#[should_panic]` test driving a non-monotone closure. `pair_fold` is private to `query` and the test module is its child, so the driver needs no new surface.

Evidence:

    831          // A hard assert, not a debug one: a non-monotone closure would
    832          // otherwise fold the term in the wrong direction silently, and the
    833          // check is one word compare per orientation change.
    834          assert_eq!(
    835              coefficient < 0,
    836              sign == Ordering::Less,
    837              "a monotone orientation's change term is a debit"
    838          );

Resolution: Add `#[should_panic(expected = "a monotone orientation's change term is a debit")]` in tests.rs driving `super::pair_fold` on any pair whose difference changes sign with the anti-monotone closure `|s| match s { Ordering::Greater => -1, Ordering::Less => 1, Ordering::Equal => 0 }`. Acceptance: the test exists and passes.
Construction: With the anti-monotone closure, any orientation change with nonzero `D'` fires the assert: `D` from positive to negative gives `coefficient = +2` while `diff.sign_magnitude()` reads `Less`, so `coefficient < 0` (false) differs from `sign == Ordering::Less` (true). `Shape::ConcurrentPair.version_pair(2)`, whose difference flips at every overlay boundary, is one such pair.

### skyline-query-18: The "read an accumulator as (Sign, Base), skipping zero" idiom is hand-spelled at seven production sites
- Where: crates/before/src/version/skyline/query/integral.rs:921-934 (related: query/integral.rs:757-763, 859-867, 946-966, 988-999; query.rs:459-461; query/web.rs:387-392; signed.rs:66-72, 105-113)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `from_is_negative(.*== Ordering::Less)` over crates/before/src lists the sites); executed: no
- Seen by: structure; refutation: confirmed, severity low -> nit; history: no-rationale-found (4ac8fd70 placed the Ordering-to-Sign move at `Signed::from_sign_magnitude`, which yields `Int`, so the `Base` consumers cannot reuse it; a `Base`-valued twin was never proposed)
- Owner-gated: no

`sign_magnitude()`, compare to `UBig::ZERO`, `Sign::from_is_negative(sign == Ordering::Less)`, `Base::from(magnitude)` recurs in `Aggregate::merge`, `Integrator::freeze`, `settle_segment`, `settle`, `promote`, `min_ticks`, and `EpochLedger::freeze`. `signed.rs` declares itself the one home for the sign vocabulary, yet the read is re-derived at every settle; the `is_literally_zero` pre-check ahead of the read is present at 921 and 946 and absent at 988.

Evidence:

    921          if self.parked.is_literally_zero() {
    922              return;
    923          }
    924          let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
    925          if parked_magnitude == UBig::ZERO {
    926              return;
    927          }
    928          charge_segment(
    929              &mut self.total,
    930              Sign::from_is_negative(parked_sign == Ordering::Less),
    931              &Base::from(parked_magnitude),

Resolution: Add `pub(super) fn signed_base(acc: &Accumulator) -> Option<(Sign, Base)>` (None when the magnitude is zero) to signed.rs, and optionally `Sign::from_ordering`; rewrite each site as one `if let Some((sign, parked)) = signed_base(&self.parked)`. Acceptance: the grep returns at most the helper's own definition; `just test-all` green.

### skyline-query-19: "sound" used loosely for "correct only when" and "holds"
- Where: crates/before/src/version/skyline/query/integral.rs:980-981 (related: query/tests.rs:1418, 1524)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep of `\bsound\b` over the four files: integral.rs:143 is the proper use for an argument; the other three are loose); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

A method is correct under a precondition and an identity holds; "sound" belongs to arguments. The vocabulary rule flags loose "sound"/"invariant".

Evidence:

    980      /// Sound only immediately after [`settle_segment`](Self::settle_segment):
    981      /// the segment credit covered `P` up to the current position — which the

    (tests.rs:1418)      /// F_final·2^S − Σ_freezes drift·position` is sound — and superlinear

Resolution: integral.rs:980 "Correct only immediately after ..."; tests.rs:1418 and 1524 "the identity ... holds". Acceptance: the grep matches only integral.rs:143.

### skyline-query-20: Comment says the emptiness check is not re-taken here, directly above a debug_assert that re-takes it
- Where: crates/before/src/version/skyline/query/integral.rs:1044-1051
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read the comment and the assertion beneath it); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (9c7b6999 replaced an early return with the debug_assert and wrote both sentences in one hunk; its message says the precondition is "stated instead")
- Owner-gated: no

The comment and the assertion beneath it contradict each other; the comment also carries the hand count "The one caller" (finding 22).

Evidence:

    1044          // The ledger must hold armings: an empty one would still push the
    1045          // virtual closing entry and charge the final window against nobody's
    1046          // debt. The one caller tests this immediately above the call, so the
    1047          // check lives there rather than being re-taken here.
    1048          debug_assert!(
    1049              !self.promotions.is_empty(),
    1050              "the ledger settles only behind a non-empty check"
    1051          );

Resolution: "The caller gates the call on a non-empty ledger; this restates the precondition in debug builds." Acceptance: comment and code agree on whether the check is taken here.

### skyline-query-21: `mul_into` carries a limb-metered zero guard its sibling refuses, a shift parameter that is zero at every production call, a collected `Vec<u32>`, and a `bool` whose keep ruling lives only in history
- Where: crates/before/src/version/skyline/query/web.rs:121-135 (related: query/web.rs:93-102, 104-120, 208-214, 409-415; query/integral.rs:472-478; codec/base.rs:33-35, 248-252, 432-436; query/tests.rs:1447, 1627, 1645, 1660, 1965, 1983, 2560)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep of `mul_into(` call sites: both production callers pass `0` as `shift`; `git show aa7c96a0` and `git log -1 4ac8fd70` messages read); assessed (read `impl PartialEq for Base`, which calls `meter_limbs2`, and `Base::bits`, which is O(1) and unmetered); executed: no
- Seen by: structure, correctness, claims; refutation: confirmed; history: three verdicts: keeping `mul_into` as a distinct word-scale kernel is deliberate and stated inline (web.rs:115-120); `subtract: bool` is an explicit owner keep recorded only in 4ac8fd70 ("Deliberate keep-as-bool ... operation selector"); the factor zero guard was kept on purpose by aa7c96a0, which in the same commit dissolved `charge_digits`' guard on the ground that `Base` equality is metered width-scale work, without weighing that cost here; the `shift` parameter's caller moved to `charge_segment` in 016b91c4 and the parameter stayed
- Owner-gated: yes: the guard and the bool are recorded owner rulings

Four items on one function. (1) `if *factor == Base::ZERO` runs through `Base::eq`, which records both operands' limbs, so every reign settle and epoch settle records a factor-width compare into the limb column the min_ticks bands judge; the sibling kernel's comment at integral.rs:475-478 declines the identical guard for exactly that reason. `Base::bits() == 0` answers in O(1), unmetered. (2) Both production callers pass `shift` 0; only the test kernels pass a scale, and the doc's "a segment mass parked deep in the stream" describes those kernels, not any production charge. (3) `u32_digits` collects a `Vec<u32>` the loop then only iterates; the `Limbs` `flat_map` iterator suffices. (4) `subtract: bool` is an owner-ruled operation selector, but the ruling lives only in a commit message, so the site reads as a `Sign` that was not converted.

Evidence:

    121  pub(super) fn mul_into(
    122      total: &mut Accumulator,
    123      factor: &Base,
    124      digits: &Base,
    125      shift: u64,
    126      subtract: bool,
    127  ) {
    133      if *factor == Base::ZERO {
    134          return;
    135      }

    (integral.rs:475-478)
    475      // zero-valued factors at the sign reads that price them. A guard here
    476      // would itself be metered width-scale work: `Base` equality records its
    477      // operands' limbs, so a per-charge zero test taxes every settle by the
    478      // factor's width.
    (base.rs:248-252)
    248  impl PartialEq for Base {
    249      fn eq(&self, other: &Self) -> bool {
    250          meter_limbs2(self, other);
    251          self.0 == other.0

Resolution: `if factor.bits() == 0 { return; }` (or drop the guard: a zero factor's per-digit `*=` is word-scale and the adds are no-ops); iterate `Limbs::new(&digits.0).flat_map(..)` directly; either drop `shift` (the adequacy kernels pre-shift through a test-local wrapper) or reword the doc to say the scale serves the committed known-bad kernels while every production charge is at scale zero; add one comment line stating that `subtract` is an operation selector, not a quantity sign. Acceptance: `grep -n '\*factor == Base::ZERO' web.rs` is empty; the min_ticks limb columns drop by one factor-width record per settle (re-pin attributed to this change) with value pins unchanged; either no production call site names a shift or the doc's description matches its callers.

### skyline-query-22: Hand-maintained caller and client counts, one already false in the test build
- Where: crates/before/src/version/skyline/query/web.rs:128-129 (related: query/web.rs:20; query/integral.rs:473-474, 901, 1046; query/tests.rs:1447, 1627, 1645, 1660, 1965, 1983, 2560)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `both callers|the one caller|both its clients|both consumers` over the four files; grep of `mul_into(` shows two production callers and seven in tests.rs); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed; history: no-rationale-found (web.rs:128 and integral.rs:473 were written in aa7c96a0 two days after the doctrine line naming "both callers" entered the dotfiles; 1046 in 9c7b6999 the same day; 901 and web.rs:20 predate the rule)
- Owner-gated: no

Doctrine names "both callers" as the forbidden example of a hand-maintained count. `mul_into`'s "Both callers hand in nonzero counts" is already false: the adequacy kernels call it with segment and position masses (tests.rs:1645-1651 passes a possibly-zero segment) and rely on the zero-operand fall-through.

Evidence:

    128      // Both callers hand in nonzero counts (a settled reign counts at least
    129      // its one close; the ledger settle skips zero suffixes), and a zero

    (integral.rs:473-474)
    473      // clusters, so the loop is the no-op it should be, and both callers
    474      // (the segment settles and the aggregate merges) already skip
    (integral.rs:901)      /// both consumers, priced by the segment's depth variation.
    (integral.rs:1046)     // debt. The one caller tests this immediately above the call, so the
    (web.rs:20)            //! held once for both its clients; this module drives it through [`ReignWeb`]

Resolution: State the contract, not the tally: "Callers pass nonzero counts; a zero `digits` operand is a no-op by the empty digit walk" (web.rs:128); "callers skip zero-valued factors at the sign reads that price them" (integral.rs:473); "The caller gates the call" (1046); "held once for its clients" (web.rs:20); "one watermark read serving the settle and the banked window" (901). Acceptance: the grep returns nothing over the partition.

### skyline-query-23: `Reign::mint` and "mint" prose for constructing a value
- Where: crates/before/src/version/skyline/query/web.rs:190 (related: query.rs:56; query/web.rs:64, 66, 176, 300, 306, 311, 330)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep of `\bmint(s|ed|ing)?\b` over the four files: nine sites; 59 word-matches across crates/before/src and crates/suanpan/src); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed, severity medium -> low; history: deliberate-but-expired (the identifier landed in c29bd2b3 when "mint" was the restructure campaign's own word; the writing-style ban postdates it by about thirteen days)
- Owner-gated: no

The constructor is named `mint`, so the word propagates into four doc sentences that name the operation, and query.rs:56 says the integral submodule "mints" the height-split components. The review's vocabulary rule: never write "mint" for constructing a value. `Reign::mint` is the only identifier crate-wide; the prose part is one slice of a crate-level sweep.

Evidence:

    188  impl Reign {
    189      /// A fresh record at a leaf's value, no closes counted yet.
    190      fn mint(sign: Sign, offset: &Base, epoch: u32) -> Reign {

    (query.rs:56)  //! whose components the [`integral`] submodule mints and derives along with the
    (web.rs:64)    //! - a reign record's mint and its one death settle: the code funding

Resolution: Rename to `Reign::new` (or `Reign::at_leaf`); reword query.rs:56 "defines and derives", web.rs:64 "a reign record's creation", 66 "between creation and death", 176 "since the record was created", 306 "created or moved". Acceptance: the grep over the partition returns nothing.

### skyline-query-24: `EpochLedger::epoch` narrows the freeze count to `u32` behind an expect that asserts rather than argues, and its doc miscounts
- Where: crates/before/src/version/skyline/query/web.rs:369-372 (related: query/web.rs:177-186, 384-396; query.rs:456-458, 467; codec/bits.rs:117-119)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read `epoch`, `freeze`, the `Reign` struct, and the per-leaf call at query.rs:467; the freeze-cost arithmetic re-derived by hand from the trigger at query.rs:456); executed: no
- Seen by: correctness, claims (the narrowing), prose (the doc); refutation: confirmed; history: no-rationale-found (the bare expect dates to 56a08f90; two later width audits, 6323d667 and 7ea3df58, passed over it; d3a029d4 reworded `freeze`'s doc to name the discard arm and left `epoch`'s doc untouched)
- Owner-gated: no

`epoch()` runs once per leaf from `min_ticks` and converts `drifts.len() - 1` with `expect("freeze count fits u32")`. The message states the conclusion, not the premise, and the bound is reachable from a canonical stream on a 64-bit target: a `+2^288` delta followed by a `+1` trips the trigger (live at 10 digits against 1 funded plus 8 allowed) with nonzero drift, so one freeze costs about 580 code bits and 2^32 freezes need a stream of about 2^41 bits (roughly 311 GB), which the codec admits (bits.rs:117-119). Principle 1: panics only for programmer error, every expect message a one-line proof, and input size carries no weight; the one tolerated corner is infeasible work, which 2^41 bits is not. Separately, the doc says "the freezes so far" but `freeze` discards a redundantly spelled zero without pushing, so the epoch counts parked drifts, and the next method's own doc ("or discard a redundantly spelled zero, keeping the epoch") contradicts this one.

Evidence:

    369      /// The current epoch: the freezes so far.
    370      pub(super) fn epoch(&self) -> u32 {
    371          u32::try_from(self.drifts.len() - 1).expect("freeze count fits u32")
    372      }

    (web.rs:388-393)
    388          if drift != UBig::ZERO {
    389              self.drifts.push((
    390                  Sign::from_is_negative(sign == Ordering::Less),
    391                  Base::from(drift),
    392              ));
    393              self.refs.push(0);

Resolution: Make `Reign::epoch` and `epoch()` `usize` (or `u64`), drop the `try_from`/`expect`, and index `refs` with it directly (`Reign` is a heap-owned per-boundary payload, so the width change costs at most a few bytes of alignment per stacked boundary). If the owner instead rules a 2^41-bit operand infeasible, rewrite the message as the argument ("a freeze costs at least 2^9 stream bits, so 2^32 freezes need a 2^41-bit stream"). Either way, reword the doc: "The current epoch: the drifts parked so far (a freeze that finds no drift keeps the epoch)." Acceptance: no `u32` conversion remains on the epoch path, or the expect reads as a proof from a stated bound; the two method docs agree; min_ticks value pins unchanged.
Construction: A right spine of 2^32 + 1 two-leaf blocks whose heights step by `+2^288` then `+1`: after the `+2^288` fold `live.digit_count() = 10` and `int_digits = 10` (no trigger); after the `+1` fold `10 > 1 + 8` fires `EpochLedger::freeze` with nonzero drift, pushing one epoch per block. At the 2^32-th block's leaf, `ledger.epoch()` at query.rs:467 fails the `u32::try_from`. Stream size about 2^32 · 580 bits; construct through `Version::decode` on a 64-bit machine with enough memory rather than running it at test scale.

### skyline-query-25: The promoting-pool rationale cites an `arb_base` ceiling the generator no longer has
- Where: crates/before/src/version/skyline/query/tests.rs:237-242 (related: query/tests.rs:644-645; testing/generators.rs:322-332)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`git log -1 28f6981e` message: "previously every arm topped out near 2^129"; the widened arm at generators.rs:330 read); executed: no
- Seen by: correctness; refutation: confirmed (promotion reachability from the arbitrary sweep was not sampled either way); history: deliberate-but-expired (e695d5cf wrote both docs against the then-true ceiling; 28f6981e widened the generator and updated neither)
- Owner-gated: no

The doc rests on "`arb_base` tops out near 2^128, under half of that". generators.rs:330 now has an arm `(2j + 1) << k` for `k in 0..512`, so `arb_base` reaches about 2^514 (17 digits), and a wide node base over two small leaves fires the freeze trigger inside `arbitrary_trees_agree`. The pool's unique contribution (promotion, and the arming trains' repeated and mixed-sign armings) is real but is not what the doc says; a test doc's incorrectness is a bug in the test (root AGENTS.md).

Evidence:

    237  /// The other pools stay under the freeze allowance almost everywhere: a
    238  /// unit-funded fold freezes only past 9 digits (288 bits) of live drift, and
    239  /// `arb_base` tops out near 2^128, under half of that — so the promotion ledger
    240  /// and its product-tree settle would run differentially unwitnessed without
    241  /// this pool: these shapes are the only ones that arm it, and the arming trains
    242  /// are the only ones that arm it more than once per sweep or with mixed signs.

    (generators.rs:330)          1 => (0u64..4, 0u32..512).prop_map(|(j, k)| codec::Base::from(2 * j + 1) << k),

Resolution: Restate the rationale against generators.rs as it is: freezes are in-support for arbitrary trees; what the pool uniquely supplies is promotion (a parked component at least 10 digits wide against a wide-spelled narrow drift, or an 18-digit parked sum) and the multi-arming trains, which the arbitrary sweep reaches rarely if at all. Delete both 2^128 figures (237-242 and 644-645). If the exclusivity sentence ("these shapes are the only ones that arm it") is to stay, back it with a promotion tap over `family_pool` and the arbitrary sweep; otherwise drop it. Acceptance: `grep -n '2^128\|128-bit' tests.rs` returns nothing and the stated premise matches the widest arm in generators.rs:322-332.
Construction: Not a runtime construction; the falsity is textual. For the freeze claim: `spine_of(&[Base::from(1u8), (Base::from(1u8) << 500) + Base::from(1u8), (Base::from(1u8) << 500) + Base::from(3u8)])` folds deltas of about 2^500 then +2 and trips the trigger (16 live digits against 1 funded + 8), a shape `arb_oracle_version` generates whenever a wide-arm node base sits over two small leaves.

### skyline-query-26: A fullwidth left parenthesis (U+FF08) in the `zero_drift_heights` doc
- Where: crates/before/src/version/skyline/query/tests.rs:341
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for the UTF-8 sequence `EF BC 88` matches line 341); executed: no
- Seen by: refutation (new); refutation: n/a; history: n/a
- Owner-gated: no

A typo: the opening parenthesis of the first delta is the fullwidth form while its closing parenthesis is ASCII.

Evidence:

    341  /// Deltas `+（2^(32p) + d)`, `−(2^32 − 1)·2^(32(p−1))`, `−2^(32(p−1))`, then

Resolution: Replace `（` with `(`. Acceptance: `grep -c $'\xef\xbc\x88' tests.rs` is 0.

### skyline-query-27: `zero_drift_freezes_keep_the_totals_exact` claims a tripped trigger and an empty freeze that nothing in the test observes
- Where: crates/before/src/version/skyline/query/tests.rs:397-406 (related: query/integral.rs:276-285, 858-875; query/web.rs:384-396; query/tests.rs:358-375; tools/covcheck-expected.json:203-209; justfile:1005-1026)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read the test, `Integrator::freeze` (the zero arm returns before the `FREEZE_HITS` bump at 875), `EpochLedger::freeze` (no tap), and the coverage pin, which lists one `integral.rs` anchor and no `web.rs` entry); executed: no
- Seen by: prose, correctness; refutation: confirmed, severity medium -> low; history: deliberate-and-holds for the instrument choice (f77011e3 designed this family as coverage-driven, with the CI coverage legs as the committed liveness instrument: an undriven integral.rs:864-865 fails the line leg by name and a never-false web.rs:388 fails the branch leg), so the finding converts to "state the instrument at the test", with two residuals
- Owner-gated: no

The doc says the width trigger trips on a zero-valued live component, that the rank integral parks nothing, and that min_ticks keeps its epoch; the body asserts value equality only. The module's own standard (integral.rs:281-283) is that a freeze-regime witness must prove its inputs park; this test carries no floor and could not use `FREEZE_HITS`, which is bumped after the zero arm returns. The committed liveness instrument is the coverage pin, which runs in CI only ("the gate never runs them") and is not named at the test. The arm is value-neutral (deleting both zero arms leaves every total exact: a zero drift parks a zero, and every settle skips a zero parked magnitude or zero factor), so the doc's "freezes nothing" and "keeps its epoch" are cost claims of the kind the mutants roster classes as performance-genre for the neighbouring trigger legs.

Evidence:

    397      /// The zero-drift family: a width trigger tripped by a live component
    398      /// whose value is exactly zero (spelled wide) freezes nothing.
    399      ///
    400      /// The rank integral parks no drift and min_ticks keeps its epoch, and
    401      /// both folds stay exact against the tree oracle through the empty
    402      /// freeze.
    403      #[test]
    404      fn zero_drift_freezes_keep_the_totals_exact(p in 9u32..=13, d in 1u64..=6) {
    405          assert_single(&spine_of(&zero_drift_heights(p, d)));
    406      }

    (integral.rs:860-866, 875)
    860          if drift == UBig::ZERO {
    864              self.live.reset();
    865              return;
    866          }
    875          FREEZE_HITS.with(|hits| hits.set(hits.get() + 1));

Resolution: Either (a) add a `#[cfg(test)]` tap on the zero-drift arm of `Integrator::freeze` and on the discard arm of `EpochLedger::freeze` and floor both here (at least one per fold per case), so the gate itself sees the regime; or (b) keep the coverage legs as the instrument and say so in the doc: "value-exact on the zero-drift schedule; that the arm is driven is pinned by the CI coverage legs (tools/covcheck-expected.json lists no entry for either arm), not by this test". Acceptance: under (a), deleting the `if drift == UBig::ZERO` arm at integral.rs:860-866 or the `if drift != UBig::ZERO` guard at web.rs:388 turns this test red; under (b), the doc and body agree.
Construction: Change the strategy to `p in 1u32..=2` (or raise `FREEZE_ALLOWANCE_DIGITS` to 16): the live spelling never exceeds the allowance, no trigger fires, and the test still passes; independently, delete both zero arms and the test still passes, because parking a zero is value-identical to not parking it.

### skyline-query-28: Backend multiplication-tier thresholds restated as literals with no anchor to the dependency pin
- Where: crates/before/src/version/skyline/query/tests.rs:944-948 (related: query/tests.rs:1142-1143; query/integral.rs:220-221; suanpan/src/lib.rs:296-297)
- Class / severity / confidence: claim / nit / medium
- Provenance: assessed (read dashu-int 0.5.0's `mul/mod.rs:16, 23` (`THRESHOLD_SIMPLE_DEFAULT = 24`, `THRESHOLD_KARATSUBA_DEFAULT = 96`) and `mul/ntt/mod.rs:29` (`THRESHOLD_NTT = 4_000`) in the cargo registry; Cargo.lock holds dashu-int 0.5.0; suanpan's doc pins "dashu-int 0.5" and calls a bump a breaking change); executed: no
- Seen by: claims, correctness (open question); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The three dispatch thresholds appear as literals here, at tests.rs:1142-1143, and in the integral doc's "4,000-word operand sides". They are correct today, but nothing ties them to the pin: after a dashu bump that moves a threshold, both tier-boundary tests stay green (they are value tests) while no longer straddling any dispatch boundary, and integral.rs:220 becomes false with no red anywhere. Principle 5's hand-maintained-count rule applied to a dependency's internals.

Evidence:

    945      // dashu 0.5 dispatches on the smaller side in 64-bit words: simple ≤ 24,
    946      // Karatsuba ≤ 96, Toom-3 ≤ 4,000, NTT above. One width at each threshold
    947      // and one past it, in base-2^32 digits.
    948      for words in [24usize, 25, 96, 97, 4_000, 4_001] {

Resolution: Name them once as a test-module constant (for example `DASHU_05_MUL_TIER_WORDS: [usize; 3] = [24, 96, 4_000]`) with a comment tying them to suanpan's dashu 0.5 pin and the bump procedure; have `dense_factor_tier_legs` and the tier-boundary loop derive their widths from it, and have the integral doc cite the constant by name. Acceptance: one definition site for the thresholds.
Construction: Bump dashu-int past a release that moves its Toom-3 threshold: `clustered_charge_agrees_at_backend_tier_boundaries` and `dense_factors_agree_through_the_public_fold_at_tier_boundaries` still pass while straddling no dispatch boundary.

### skyline-query-29: Test comment attributes production stack safety to `crate::recurse::descend!`, which production code does not use
- Where: crates/before/src/version/skyline/query/tests.rs:1165-1168 (related: crates/before/AGENTS.md:31-37; query.rs:212-221, 440-471)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'descend!' crates/before/src` excluding tests.rs files, src/testing/, and src/recurse.rs returns nothing); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (inaccurate when written in 013334f2, not expired)
- Owner-gated: no

The production folds are iterative (`rank` and `min_ticks` are `loop`/`while` walks; the settle tree runs on an explicit stack), and AGENTS.md states that `descend!` guards only test surfaces. A reader following this parenthetical looks for a guard in `query.rs` that is not there (Principle 5).

Evidence:

    1165      // The recursive oracle and its bridge are test-only plain recursion on tree
    1166      // depth, and the dense masses here run the spine thousands of levels deep —
    1167      // the production folds are stack-safe (`crate::recurse::descend!`), so the
    1168      // headroom is for the witnesses, not the code under test.

Resolution: "the production folds are iterative (the crate's recursion rule: depth lives on explicit stacks), so the headroom is for the witnesses, not the code under test." Acceptance: the comment names no `descend!` and the grep stays empty.

### skyline-query-30: Eight adequacy test docs carry bracketed measured readings the sibling envelope suite keeps in pin commits
- Where: crates/before/src/version/skyline/query/tests.rs:1491-1493 (related: query/tests.rs:1802-1804, 1829-1832, 2165-2167, 2192-2195, 2404-2407, 2636-2639, 2675-2678; tests/meter.rs:5222-5225)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `measured in the dev profile` finds the eight sites; tests/meter.rs:5222-5225 read); executed: no
- Seen by: correctness, claims; refutation: confirmed, severity low -> nit; history: deliberate-but-expired (d2a9d04e kept the brackets and stripped their dates; 500d4d09 then ruled that readings live in pin commits and swept the meter surface, but its sweep did not reach query/tests.rs)
- Owner-gated: no

Each tripwire doc records touch or limb counts and byte sizes from one profile run to justify its floor as "midway between linear and the measured growth". The floors are enforced; the tallies are prose restatements that rot with any generator or accumulator change, while the `eprintln!("MEASURED ...")` lines already print the live reading. The crate's own convention of record for the same kind of number (tests/meter.rs:5222-5225) is "the record and every re-pin's movement live in the pin commits" (Principle 5: dated measurement reports are not exempt).

Evidence:

    1491      /// [measured in the dev profile, exact counters: touches 124,368 -> 372,859
    1492      /// across FP(1,000) -> FP(2,000), packed 73,328B -> 146,579B: per-byte
    1493      /// growth x1.50.]

    (meter.rs:5223-5224)
    5223      /// wide-arming family, measured ×1.25 (the record and every
    5224      /// re-pin's movement live in the pin commits).

Resolution: Move the eight bracketed records to the pin commits and keep the derivation sentence ("the floor sits between linear and the measured growth; the record lives in the pin commit"). Acceptance: `grep -n 'measured in the dev profile' tests.rs` returns nothing and each floor's doc still says what the floor sits between.

### skyline-query-31: The adequacy kernels hand-copy the shipped driver loops, `finish`, and the settle reduction under "verbatim" claims that have drifted
- Where: crates/before/src/version/skyline/query/tests.rs:1684-1755 (related: query/tests.rs:1577-1583, 1914-1920, 2048-2119, 2263-2352, 2354-2373, 2482-2606; query.rs:201-225, 327-401 (351-357, 391-396); query/integral.rs:825-841, 1043-1130, 1142-1163)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`git show aa7c96a0 -- integral.rs` removes `if final_window_magnitude != UBig::ZERO {`, `if segment_magnitude != UBig::ZERO {`, and `let negative = (coefficient < 0) != (sign == Ordering::Less);`; `git show f77011e3 -- query.rs` replaces `.unwrap_or(1);` with `.expect("the advance law steps at least one side per boundary");`; `git show 9c7b6999 -- integral.rs` replaces `if self.promotions.is_empty() { return; }` with the debug_assert; the copies read side by side with the shipped bodies); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed (adds the `is_empty` early return and the dropped opening-sign step), severity medium -> low; history: deliberate-but-expired (the copies were verbatim when written and the lockstep was an explicit hand-maintained convention: 14259c1f re-synced them "so their 'reduction verbatim' claims stay true", and 982bd260 shared `mass_split` after a copy of the shipped rule masked a genuine mutation through the full suite; the 2026-08-12 trio then changed the shipped bodies without mirroring)
- Owner-gated: no

The `adequacy` module reproduces the shipped `rank` loop four times (1686, 2050, 2357, 2590), the `pair_fold` loop twice (1706, 2070), `Integrator::finish` twice (2334, 2555), and the `settle_armings` reduction twice (2266, 2485), each documented as the shipped body "verbatim" with one move swapped. Every such claim is false today: the settle copies open with `if integ.promotions.is_empty() { return; }` where the shipped code debug-asserts non-emptiness; they and the `finish` copies guard `final_window_magnitude != UBig::ZERO` and `segment_magnitude != UBig::ZERO` where the shipped code is unconditional; the pair copies end their funded-width fold in `.unwrap_or(1)` where the shipped loop uses `.expect(...)` and omit the shipped opening-sign step (query.rs:351-357); the two standalone integrators keep the pre-aa7c96a0 conditional `jump`. Every divergence is value-neutral on reachable inputs and each kernel is value-pinned against the shipped fold on every run, so no tripwire is measuring a hybrid; the cost is false prose at ten sites (Principle 5) and a hand-maintained sync that has failed three times in one day and that the crate already knows can mask a mutation (982bd260). That the hazard has bitten is why this stays medium rather than low.

Evidence:

    1684      /// The rank fold on the span-reading integrator: the shipped
    1685      /// [`rank`](super::super::rank) loop verbatim, integrator swapped.
    1746              let funded = da
    1747                  .iter()
    1748                  .chain(db.iter())
    1749                  .map(|step| super::super::integral::int_digits(&step.magnitude))
    1750                  .max()
    1751                  .unwrap_or(1);

    (query.rs:396)              .expect("the advance law steps at least one side per boundary");

    2263      /// The shipped ledger settle with the per-digit absorb: the
    2264      /// mass-balanced product-tree reduction verbatim, every window
    2265      /// merge routed through [`merge_per_digit`].
    2282          if final_window_magnitude != UBig::ZERO {
    2283              windows.merge(&final_window_magnitude, final_window_shift);
    2284          }

    (integral.rs:1075-1076)
    1075          let mut windows = WindowMass::new();
    1076          windows.merge(&final_window_magnitude, final_window_shift);

Resolution: Test-local refactor, no production change (the precedent 982bd260 set for `mass_split`): one small trait (`open`, `interval`, `jump`, `boundary`, `live(&mut)`, `finish`) implemented by a thin wrapper over the shipped `Integrator` and by each known-bad integrator; one `fold_single` and one `fold_pair` driver in the test module; one `settle_with(integ, merge)` reducer shared by the per-digit and schoolbook kernels; one shared `finish_with(integ, close)`. The kernels then differ from the shipped code only in the component they refute, and "verbatim" becomes true by construction where it is still claimed. If the owner prefers to keep the copies independent, strike "verbatim" from all ten docs and re-sync the copies to the current shipped bodies (drop the zero guards and the `is_empty` return, adopt the unconditional `jump`, match `.expect`, add the opening-sign step). Acceptance: no doc in tests.rs says "verbatim" about a body that is not a shared function; `grep -n 'final_window_magnitude != \|segment_magnitude != \|unwrap_or(1)' tests.rs` is empty; every `_reads_superlinear` test keeps its rostered name and still reads red at its floor and value-exact against the shipped fold under `just test-all`.

### skyline-query-32: "retired" and "fell into" narrate history where the siblings state the present
- Where: crates/before/src/version/skyline/query/tests.rs:2446-2447 (related: query/tests.rs:2433, 1388, 1517, 1854, 2007, 2220; query/integral.rs:144-146)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `\bretired\b|fell into` over the four files; the sibling demonstrators say "refuted" at the related sites); executed: no
- Seen by: prose; refutation: confirmed (`Close::Retired` is live domain vocabulary in a different sense and stays); history: contradicts-hard-rule for "retired" (root AGENTS.md's ghost-reference rule of 2026-07-21 predates 016b91c4's "The retired per-digit charge"; the kernel was the shipped settle move until that commit, so "retired" does the work of "superseded"); "fell into" narrates a past incident but the composed form still exists as the test oracle, so that half is a wording fix
- Owner-gated: no

"Retired" says this kernel was formerly the shipped path, which is provenance that lives in git; the sibling demonstrators use the present-tense "refuted". This is a breach of the ghost-reference hard rule by its letter (no "formerly"/"superseded"); its purpose (no dangling references to deleted code) is not harmed because the kernel exists as the demonstrator, which is why the severity is low rather than high. integral.rs:144 narrates the composed form's failure in the past tense.

Evidence:

    2446      /// The retired per-digit charge: one `parked`-wide product per
    2447      /// balanced digit of the mass.

    (tests.rs:2433)      // `integral` module doc's settle bound). This kernel keeps the retired
    (integral.rs:144-146)
    144  //! operand that did not deposit — the hole the composed form fell into, where
    145  //! the meet's emission re-coded one operand's width into switch jumps that
    146  //! the integral then evicted at the other operand's cheap codes, priced by a

Resolution: tests.rs:2433 and 2446: "refuted". integral.rs:144: "the hole a composed form falls into: the meet's emission re-codes one operand's width into switch jumps that the integral then evicts at the other operand's cheap codes". Acceptance: `grep -n -E '\bretired\b|fell into'` over the partition matches only the `Close::Retired` variant.

## Positives

- `pair_fold`'s `orientation: impl Fn(Ordering) -> i8` is the right amount of abstraction: one merge walk, three measures, each closure a three-line transcription of one row of the module doc's sigma table (query.rs:76-80), monomorphized with no trait or enum machinery. The sigma table turns "orientation" into a checkable contract: `pair_fold`'s doc states its two clauses (query.rs:314-322) and `Integrator::jump`'s doc derives the always-debit property from them (integral.rs:818-824), so the assert message reads as a one-line proof.
- One `Integrator` serves rank, distance, lag, and rank_cmp; the single-stream rank is the two-ledger integral with `Φ_b` empty, so the shipped code has no per-measure kernel duplication.
- `settle_armings` satisfies the no-depth-recursion rule with an explicit `Step::{Open, Merge}` control stack that still reads like the recursion it replaces (integral.rs:1096-1129), and the doc says why it is not routed through `crate::fold` (an online entry-count balancer versus an offline mass balancer with a side-effecting combiner).
- `web.rs` reuses `watermark::MinWeb<P>` generically at payload `Reign` instead of re-implementing the anchored-minimum web; `ReignWeb` adds only the fold's own semantics, and `EpochLedger` isolates the frozen component's summation by parts in about 70 lines.
- The `frozen` gate on the segment and window feeds is derived on the field it gates (integral.rs:550-562) rather than asserted, and it is pinned from both sides by the `LoneFreeze` family (tests.rs:256-263), so the never-freezing regime pays nothing toward the settle machinery.
- The funding certificates are complete in the sense the doctrine asks: every charge in the integral (integral.rs:151-182) and the min_ticks web (web.rs:54-77) names its deposit, and the arity paragraph (integral.rs:137-149) states why a two-ledger potential is required for two-stream operations, naming the rejected composed form with its failure mode.
- Every public fold carries a uniform `# Panics` section deferring to `validate` for untrusted bytes; `project`'s omits the id operand, which the `Party` type already guarantees canonical.
- The test suite is unusually strong: three independent witnesses per fold (tree oracle, composed kernels, function-space Riemann sum), both operand orders, exhaustive small scope for singles and ordered pairs, constructed cancellation and zero-drift schedules with a `FREEZE_HITS` liveness floor on the cancellation family, eight value-exact known-bad kernels rostered by name in `tests/superlinear_tripwires.rs`, and a multiplication floor that is a reduction from arbitrary integer multiplication with the stored size pinned linear inside the same proptest (tests.rs:754-769).
- The two internal-entry pins (`settle_product_tap_is_alive_on_the_wide_arming_close`, `densify_tap_prices_the_cluster_span`) open with "Deliberate internal-entry pin, decided here" and derive their floors from a universal per-boundary premise (tests.rs:1001-1017); the floor arithmetic in prose matches the code, and each carries a value leg so a tap that recorded the right number while computing the wrong integer proves nothing.
- The scaled-read hazard from suanpan (a collapsing sign read lowering the watermark shift) is cited at both consuming sites by witness name (integral.rs:903-907, 953-956), the right altitude for a cross-crate invariant.
- `mass_split`'s contract is pinned size-generically against a naive recursive reference cross-checked against the shipped explicit-stack expansion (tests.rs:1327-1380), and the exponential/doubled/uniform points (tests.rs:1296-1325) document why the bound is denominated in mass; the stated masses and entry counts check arithmetically.
- The cfg-gated taps (`meter_product`, `meter_window_digits`, `meter_densified_image`) compile to nothing without `limb-meter` and are called unconditionally, so the algorithm has no feature forks; each tap's doc names the concrete dark-tap artifact it excludes and the committed pin that would fire.

## Open questions for Finch

1. Adequacy kernels (finding 31): share the drivers test-locally (a small trait plus `fold_single`/`fold_pair`/`settle_with`/`finish_with` in tests.rs) or keep independent copies and re-sync them by hand, striking "verbatim"? Recommendation: the test-local shared driver; it is the precedent 982bd260 set for `mass_split` after a copy masked a mutation, and it avoids adding production hooks whose only second implementor is a test.
2. `mul_into`'s zero guard (finding 21): replace the metered `*factor == Base::ZERO` with an O(1) `factor.bits() == 0`, drop it as `charge_digits` did, or keep it as ruled in aa7c96a0 and write the cost trade at the site? Recommendation: `bits() == 0`, then re-pin the min_ticks limb rows attributed to this change. Separately, the `subtract: bool` keep from 4ac8fd70 should be stated in one comment line at the signature.
3. The allocation A/B seam in `project` (query.rs:506-517, 603-613) and its `display_growth` twin: the arms are deliberate, registered under `deny(unexpected_cfgs)`, and reasoned inline, but no committed number records the pre-size ruling the shipped arm embodies (benches/presize.rs saves baselines locally). Recommendation: run the record protocol once, write the verdict (resident bytes and wall deltas) into the bench doc or an agent note, and either delete the alternative arms or add one sentence at each seam saying they are retained for re-measurement by design.
4. Crate-wide vocabulary and register (findings 1, 4, 12, 23): "seam" (176), "genre" (143), "honest" (101), "mint" (59), and em-dashes in `//` comments (374 lines) are crate dialect, and "honest" appears in your own recorded rulings. Recommendation: one crate-level pass with an owner-confirmed word list, rather than per-partition edits that leave the crate inconsistent; this partition's sites are listed under the findings.
5. The `O(M(|v|) · log |v|)` clause (finding 9): instrument it (a multi-scale fuel fit past 65 KiB, or a `meter_product` operand-width check on the doc's tight construction) or narrow the public contract to what the committed counters pin and move the quasilinear-tier remark to a decision record? Recommendation: the fuel fit, since the fuzzfit harness already prices `ff_version_rank` in the currency that counts the backend's work; failing that, narrow.
6. `EpochLedger::epoch`'s `u32` (finding 24): widen to `usize`, or rule a 2^41-bit operand infeasible and write the argument into the expect message? Recommendation: widen; the compactness saving on `Reign` is a few bytes per stacked boundary.
7. `promoting_pool`'s exclusivity claim ("these shapes are the only ones that arm it", tests.rs:241): unverified either way now that `arb_base` reaches 2^514. Recommendation: a `#[cfg(test)]` promotion tap over `family_pool` and one run of the arbitrary sweep to settle it, then either keep the sentence with the tap as its witness or drop it.
8. Measured brackets in the adequacy docs (finding 30): confirm that 500d4d09's pin-commit convention applies to query/tests.rs as it does to the meter surface. Recommendation: yes, move them.
9. dashu thresholds (finding 28): one named constant with the bump note, or leave the literals? Recommendation: the constant; a dashu bump is already declared a breaking change in suanpan's docs, so one re-verification site is the cheap discipline.

## Dropped

- Hard assert in `Integrator::jump` justified as catching a failure the differential suite already catches [27]: refuted; f77011e3 records the owner's direction that impossible states panic, the three shipped closures are all monotone so no committed test exercises a violation, and the assert is the only observer of the private closure contract; its adequacy gap survives as finding 17.
- query.rs module doc restates integral.rs's height-split paragraph and tests.rs's testing map "nearly verbatim" [22]: below the bar; the `# Testing` section is owner-kept (14259c1f, plan D6) and style-prescribed, and only one derivation sentence is shared between query.rs:50-52 and integral.rs:34-36.
- Bench-only allocation A/B arms compiled into `project` [10]: deliberate and documented inline (query.rs:506-512, Cargo.toml:98-108, justfile:792-806); the missing recorded verdict is open question 3.
- Folding `mul_into` into `charge_segment` (the maximum of [1]): deliberate and documented at web.rs:115-120 (a word-scale kernel for O(1)-dense operands); the remaining sub-points are finding 21.
- Adequacy demonstrators "verbatim" while diverging [26], [38], [45]: duplicates of finding 31.
- `Integrator::one` workaround for `add_u64_shl` [48]: duplicate of finding 15.
- "mint" [13], [35] (mint half): duplicates of finding 23.
- `let scale = max_depth` alias [29]: duplicate of finding 2.
- Hand-maintained counts [16], [36]: duplicates of finding 22.
- Long qualified paths [30], [41], [53]: duplicates of finding 7.
- Zero-drift freeze arm liveness [34]: duplicate of finding 27.
- `EpochLedger::epoch()` panics on an input-size bound [49]: duplicate of finding 24.
- Measured records in prose [50]: duplicate of finding 30.
- `mul_into`'s limb-metered zero guard [52] and its zero shift parameter [40]: merged into finding 21.
- "genre" as an undefined synonym [19]: merged into finding 1.
- Moralized "honest"/"real" [35] (that half): merged into finding 12.
