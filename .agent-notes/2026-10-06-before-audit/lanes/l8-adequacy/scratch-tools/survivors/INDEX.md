# Mutation survivors, classified (rounds 1 and 2)

Campaign: cargo-mutants 27.1.0 over the full `before` or `suanpan` nextest
suite (`--all-features`), `PROPTEST_RNG_SEED=8008`, excluding `src/testing/`
and test files (`l8/mutants-common.sh`). Exact diffs, self-contained for
handover: `diffs/<group>.md` (each entry gives the location, the
cargo-mutants name, and the diff). Raw cargo-mutants files: `raw-<group>/`.

Classes:

- **G**: a reachable value change no test detects. Briefed.
- **T\***: an unverified trait behavior that callers rely on (`Hash`). Briefed.
- **T**: an unverified trait spelling (`Debug` text, a delegating operator
  form, an advisory `size_hint`). Listed.
- **C**: value-neutral, changes work only. Listed with the meter that sees it.
- **E**: equivalent on every input. One-line argument.
- **U**: unreachable (dead code, `unreachable!` arms, a 32-bit-only path the
  64-bit suite cannot run). One-line argument.
- **I**: instrument or test-support code compiled into the crate.
- **?**: undecided; evidence given.

## Campaign table

| group | mutants | caught | missed | unviable | status |
|---|---:|---:|---:|---:|---|
| count.rs (pilot) | 57 | 41 | 3 | 13 | done |
| version | 1,596 | 1,301 | 79 | 216 | done |
| rank (rank, ranked, accumulator) | 341 | 271 | 47 | 23 | done |
| span (span, causally) | 306 | 103 | 29 | 174 | done |
| rest (borsh, serde, shape, fold, recurse, text) | 113 | 45 | 36 | 32 | done |
| suanpan | 411 | 355 | 30 | 26 | done |
| bits | 628 | 586 | 32 | 10 | done |
| party (party, clock) | 467 | 339 | 26 | 102 | done |
| **total** | **3,919** | **3,041** | **282** | **596** | |

## Class counts

| group | G | T\* | T | C | E | U | I | ? |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| count.rs | | | 3 | | | | | |
| version | 2 | 1 | | 49 | 21 | 2 | 3 | 1 |
| rank | 11 | | 4 | 9 | 9 | 14 | | |
| span | 1 | | 2 | 6 | | 20 | | |
| rest | | | 25 | 3 | 2 | | 6 | |
| suanpan | 6 | | 3 | 9 | 12 | | | |
| bits | | 1 | 2 | 5 | 18 | 5 | 1 | |
| party | 1 | 1 | 2 | 15 | 7 | | | |
| **total (282 survivors)** | **21** | **3** | **41** | **96** | **69** | **41** | **10** | **1** |

## G (briefed)

| survivor | brief |
|---|---|
| range_minima/boundary.rs:106, :116 (`-=` to `+=`) | briefs/machinery-range-minima-near-boundaries.md (witnesses verified) |
| rank.rs:378, 387 (to `true`), 392 x3, 414 x3, 426 x3 | briefs/machinery-rank-decode-reader.md, round-2 amendment |
| span/wire.rs:136, party/io.rs:29 (`>` to `>=`) | briefs/test-span-lone-endpoint-padding.md (both witnesses verified) |
| suanpan normalize.rs:74, 81 x3, 83 x2 | briefs/machinery-suanpan-normalize-bound.md (witnesses verified) |

## T\* (briefed)

`Hash for Version` (version.rs:162), `Hash for Party` (party.rs:150), and
`Hash for Bits` (bits/storage.rs:156), each replaced by `()`:
briefs/machinery-trait-impl-coherence.md, round-2 amendment (narrowed to
`Hash`). `Bits` is crate-private and both public impls delegate to it, so
the `Version` and `Party` content tests also kill the `Bits` mutant. The branch `audit/trait-coherence-and-hole-subtracts` (in review) carries those laws; see the status section at the end.

## ? (question, not briefed)

- version/io/writer.rs:255:34 `>` to `==` in `SplitOutput::splice_continuation`.
  The mutant breaks the early return for "`end` immediately before the
  continuation's final leaf flag" (doc at writer.rs:242-245), which
  `copy_subtree_remainder` requests whenever a copied subtree's last leaf has
  a narrow code (writer.rs:569-570). Coverage never executes the return; the
  suite reaches split-output copies through 16 tests, always with a wide last
  leaf; 600,000 random ticks from two generators (one aimed at a wide left
  subtree beside a narrow, at-least-two-level right subtree) never reached
  it. Either a construction exists and the branch is an untested G, or tick
  never splits before such a copy and the doc and branch describe a dead
  case. Route to the events lane. `>` to `>=` (writer.rs:255) is E: the
  cursor cannot stop exactly at `end` after a leaf flag.

## T (listed, not briefed)

- count.rs:303 `Debug`, :324 `Add<&Count> for Count`, :360 `Sum<&Count>`.
- rank.rs:983 `Add<Rank> for &Rank`, :989 `AddAssign<&Rank>`, :995
  `AddAssign<Rank>`: delegations to the tested `&Rank + &Rank`.
- ranked.rs:328 `Debug`; causally/forms.rs:402, :408 `Debug`; serde_impls.rs:287
  `expecting` text.
- suanpan accumulator.rs:432, zero_ranges.rs:51, digits.rs:123 debug output.
- bits/reader/words.rs:58 (`Display for OutOfBytes` -> `Ok(Default::default())`): the text of a crate-private error that no path formats.
- bits/writer.rs:379 (`Debug for BitsWriter` -> `Ok(Default::default())`): the test suites' failure-message spelling.
- shape.rs `size_hint` for `Plateaus`, `Regions`, `Overlay`, and `Cells`
  (six constant replacements each): advisory under `Iterator`'s contract.
- party/forks.rs:139 x2 (`Remaining::advance`, `Near` arm, `-` to `+` or
  `/`): a count between `usize::MAX + 2` and `2 * usize::MAX` never turns
  exact, so `PartyForks::size_hint` stays `(usize::MAX, None)` where it
  could report `(usize::MAX, Some(usize::MAX))`. That is still sound and
  matches the public doc ("wider counts report a sound lower bound and no
  upper bound"). Distinguishing input: `PartyForks::new(&mut Party::seed(),
  usize::MAX + 2)` after two `next` calls (the existing
  `adjacent_wide_count_becomes_exact` covers only `Near(1)`). Exhaustion
  also changes, but only after more than `usize::MAX` shares, which is
  infeasible work.

## C (cost only; for the instrument survey)

"Board" means the amplification board's acceptance run (`just
amp-board-acceptance`), which cargo-mutants does not run. "Heap", "scan",
"touch" name the column that would move.

| survivor(s) | what changes | meter that would see it |
|---|---|---|
| fold.rs:27, :32 (`balanced_try_fold`) | balanced fold becomes a linear left fold: 14x scan bits at arity 1,024, quadratic growth (measured) | board scan ceiling and trend (`*_join_all`, `meet_all`, `span_all`); the asymptotics pins are floors and pass |
| version/projection.rs:110 | capacity hint `+` to `*` | board heap |
| version/projection.rs:355, :375 | `block_skip` shortcut disabled | board scan (projection rows) |
| version/tick.rs:831 x3; tick/prescan.rs:414 x3, :446 x3, :480 x3 | leaf-by-leaf vs block-summary threshold | board scan/touch (tick rows) |
| version/io/regions.rs:122 x2 (`peek_flip` constant) | bulk unowned-run shortcuts disabled | board scan (projection, own-version rows) |
| version/io/writer.rs:155 | 63-bit code takes the wide representation | board heap (tick rows), marginal |
| version/measure/integral.rs:230 x2, :253; min_ticks.rs:69 x2 | freeze/defer schedule (values schedule-independent; lane L4 checked all three schedules) | board touch (rank, distance, lag, min_ticks rows) |
| version/place/filter.rs:185 x2, :260, :360 x2, :364 x2, :603 x2, :613, :614 | private difference dropped and recomputed; early exits lost; settled endpoints kept scanning | board scan/touch (query, span rows) |
| version/range_minima/anchor.rs:165; emission.rs:64 | gap-dominance fast path disabled | board touch (tick rows) |
| version/range_minima/boundary.rs:46 | 2-digit boundary stored wide | board heap |
| version/range_minima/boundary.rs:113 x2 | second domination fast path toggled; fallback subtracts the same | board touch |
| version/tick/memo.rs:120 (`begin_scan` -> `()`) | memo never reused: storage grows across pre-scans | board heap (memo families) |
| version/tick/memo.rs:136 | extra memo blocks allocated early | board heap |
| version/tick/prescan.rs:279 | level-0 `latest_from_first` accumulates dead sums | board touch |
| version/tick/probe.rs:31 (`kill` -> `()`) | route probing continues after the result is decided | board scan/touch (tick rows) |
| version/measure/integral/deferred.rs:47, :110 x2; integral/width.rs:39, :90 x2 | empty settle runs; reduction-tree balancing weights | board touch (rank rows) |
| rank: accumulator.rs:99-101 (3 arms), rank.rs:387 (`false`), :396, :806, sum_iter :1052, :1058, :1059 | general conversion path; one extra read; flush per byte; wider intermediate scale | board heap/touch (rank rows) |
| span: causally/polarity.rs:90, :145, :148, :159, :161; span/algebra.rs:410 | redundant holes retained; duplicate inputs not deduplicated | board scan (query rows, span folds) |
| rest: borsh_impls.rs:129 | gamma window disabled; slow path runs | board scan (borsh rows) |
| party: fork.rs:101, :119 (`+` to `*`); io/reader.rs:48 x3, :73 x3 (`PartyPath`/`PartySubtree::stored_len`) | output capacity hints in `Party::fork_tree` (the readers' `stored_len` feeds only those hints) | board heap (fork rows) |
| party: io/reader.rs:331 x3 (`PartyReader::stored_len`), :220 x2 (`source_len`) | output capacity hints for join, `without`, fork removal, projection, and tick's raise | board heap (join, without, fork, projection, tick rows) |
| party: io/writer/positions.rs:46 (`len > 0` to `len < 0` or `len == 0`) | every adjacent tag push stores a record (one bit plus a packed distance) instead of counting the run; positions are restored exactly | board heap at depth (party writer and fork removal), unverified; #75's deep tests check values only |
| bits: stack/packed_u64.rs:63 (`quick < 62` to `>`) | every pop decodes its width bit by bit | none: bit-stack pops carry no scan charge, so only wall-time benches see it |
| bits: writer.rs:363 (`whole > 0` to `<` or `==`) | `splice_storage` copies whole bytes bit by bit instead of by `memcpy` | none: the scan charge is the range length, recorded up front, so only wall-time benches see it |
| bits: reader/words.rs:78 x2 (`read_word_opt` -> `None`, `>=` to `<`) | dsi-bitstream's optional buffer top-up is skipped, or adds a zero word past the end; `read_word` refills the same words later | none: refill counts carry no meter |
| suanpan: read.rs:90, :99; operators.rs:382; zero_ranges.rs:141, :150, :170, :195 (`true`), :196 (`true`, `!=`) | extra zero digits; `shl(0)` general path; skip ranges dropped or kept as a tree | suanpan touch (metered tests); lane L7 should check the "scans proportional to prior writes" claim against the range-dropping ones |

## E (equivalent), one line each

- version.rs:776, :777 are U (below).
- overlay.rs:142 `>`→`>=`: all four `CursorSet::step`s fold commutative sums; tied order cannot matter.
- range_minima.rs:319: `push_equal(0)` adds zero.
- shape.rs:174: equally deep walks containing the same point share one interval and flip level; walks are independent.
- io/writer.rs:255 `>=`: see the ? entry.
- io/writer.rs:410, contributions.rs:89, :164, rank.rs:647, :667, :893, accumulator.rs:101 `|`→`^`, suanpan conversions.rs:46, read.rs:115: OR of disjoint bit fields.
- place/filter.rs:689, :692 `delete !`: a required side's endpoint pair goes dead only together with `full_possible = false`; while live, its relation is already `>=` or `<=`.
- range_minima/boundaries.rs:59 x2; tick/prescan.rs:233; prescan/suspended.rs:73, :111; suanpan zero_ranges.rs:185 x3; suanpan digits.rs:247, :260: read only inside debug assertions.
- tick/memo.rs:110 x4 (`check_position`): a debug-only checksum folded the same way on both sides.
- measure/integral/pair.rs:71: inside `if current_orientation != 0` with orientation in {-1, 0, 1}, `> 0` equals `>= 0`.
- min_ticks/minima/heights.rs:117: the flipped sign arises only with count 0, which returns early; :131: multiplying by zero equals the early return.
- rank.rs:497 x5 (`alignment_fits`): on a 64-bit host `usize::try_from` of any `u64` succeeds, so the predicate is constant `true`; on wasm32 both routes are correct (pin calibration).
- borsh_impls.rs:116 x2 (`StreamBitsReader::position`): no borsh path calls it (`whole` and `prefix` take an in-memory reader).
- bits/reader.rs:182: both callers read one word from a fresh reader and drop it; `position` is never read again.
- party/compare.rs:149 `||`→`&&`: the branch is reached only when neither side has a right child, so both have left children.
- bits/reader/gamma.rs:68, gamma/window.rs:47:25, stack/bit.rs:46, :58, :64, :93, :152, :155, stack/packed_u64.rs:80 `|`→`^`: OR of disjoint bit fields (a shifted-in value is narrower than the shift, under each function's asserted precondition or by construction).
- bits/stack/packed_u64.rs:38 `&`→`|` (`value >> 63 & 1 == 1`): the arm runs only for width 64, where `value >> 63` is 1 and both forms give 1.
- bits/writer.rs:363 `>`→`>=`: with `whole == 0`, `extend_bytes` of an empty slice does nothing.
- bits/writer.rs:328 (`tail_is_zeroed` -> `true`); party/io/reader.rs:341 (`at_end` -> `true`): read only inside debug assertions.
- party/io/writer/positions.rs:46 `>`→`>=`: lets the first push onto an empty stack count as adjacent to position 0; `top` is 0 whenever the stack is empty, so pops restore the same positions.
- party/io/writer/positions.rs:62 x2 (`len -= 1` to `/=` or `+=`): `len` feeds only that empty-stack test (value-neutral, as above) and `pop`'s underflow assertion, which fires only on a programmer error.
- party/forks.rs:61 `&&`→`||` (`SharePath::next_decision`): `SharePath::next` returns `None` before calling it once `decision == depth + forks_again`, so it sees `decision == depth` only with `forks_again` set, where both forms agree.
- party/io/reader.rs:83 (`is_branch` -> `true`); bits/storage.rs:82 (`has_canonical_padding` -> `true`), :83, :84 (arm deletions): read only inside debug assertions.
- bits/writer.rs:194, :244 `|`→`^`: OR of disjoint bit fields (`value < 2^len` is `push_bits`'s asserted precondition; `value << 1` has a clear low bit).
- suanpan zero_ranges.rs:194 (`compact_storage` -> `()`), :195 (`false`), :196 (`false`): an empty or one-entry tree behaves as `Empty`/`One`; suanpan accumulator.rs:197, digits.rs:106: capacity reservations.

## U (unreachable), one line each

- version.rs:776, :777 (`span_all`): the balanced counter only combines equal weights (both raw or both merged), and the closing reduce starts from the heaviest group, which is merged whenever two groups exist.
- causally/polarity.rs `hole_subtracts` (14 mutants across `Down`, `Up`, `Neutral`): no caller anywhere (briefs/simplification-hole-subtracts.md deletes it).
- causally/polarity.rs `Neutral` impls (6): bodies are `unreachable!("a neutral query holds no holes")`.
- rank.rs `accumulate` (14): runs only when an exponent gap exceeds `usize`, i.e. only on 32-bit targets; the wasm32 pins are its instrument (they caught an injected sign error). Seven of the fourteen change values (506, 518 x2, 520 x2, 522 x2); seven change only a capacity hint (508, 511 x2, 515 x4).
- bits/reader/gamma/window.rs:47 x3 (`>>` to `<<`, `-` to `+`, `-` to `/` in `load`'s ninth-byte merge): value-changing inside the window (verified: 39, 26, and 20 of 42 unaligned 59- to 63-bit codes decode wrong; base 42 of 42 right; `witness/window.log`, `l8/window_witness.rs`), but its one production caller, borsh's `StreamBitsReader`, refills one byte at a time inside `read_bit`, so at most 7 buffered bits ever lie past `position` and the window proves codes of at most 7 bits, which never reach the merged low bits. The other caller is the test-only `ReferenceBitsReader`. The window's own tests pin a 63-bit code only at position 0.
- bits/reader.rs:201, :218: the reader's own truncation checks; decoding readers span the whole padded buffer, so the marker ends any run inside the live range, and exact-length readers read only validated storage.

## I (instrument or test support)

- version/instrument.rs:33, :57, :63: `meter`-feature entry points the board measures.
- recurse.rs:45 x2, :49 x2: test-only stack-growth constants.
- borsh_impls.rs:131 x2: a scan-meter count only.
- bits/reader.rs:298 (`k + 1` to `k * 1` in `read_unary`'s `scan::record_bits_u64`): a scan-meter count only; the position update on the next line is separate.

## Status on unlanded branches (checked against their tips)

- `simplify/codec-cleanups` (#67, tip `81f3d682`):
  - It merges the two decoder G sites into `Bits::padded_len`'s guard `len <= bytes.len()` and adds no test of it.
  - The decoder brief's two tests pass on that tip and both fail with `<=` changed to `<` (`witness/padded-len-67.log`), so the brief applies unchanged.
  - It also deletes the `?` survivor's branch (writer.rs:255), arguing that no caller places `end` on a leaf flag after internal-node flags. That agrees with the coverage and hunt evidence above (not independently verified), and the `?` becomes U, with the `>=` E going with it, when #67 lands.
- `fix/before-fork-size-hint` (#43): it deletes `Remaining`, so both party/forks.rs:139 T survivors lose their code. Its tests assert the hint is exact once the remainder fits `usize`, which is what those mutants break.
- `audit/trait-coherence-and-hole-subtracts` (in review):
  - `version_hash_is_the_byte_hash` and `party_hash_is_the_byte_hash` kill all three T\* survivors, `Bits` included through delegation (by reasoning: a `()` hash cannot equal the byte view's hash; not run).
  - `debug_is_display` kills count.rs:303 (`Debug for Count`).
  - Deleting `hole_subtracts` removes 14 U survivors.
- `audit/deep-identity-probe` (#75) adds value checks at depth 2^18; it does not change any classification here.
- `simplify/clock-sync-all-via-join-all` (#76) touches no survivor site.
