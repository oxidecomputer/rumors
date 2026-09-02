# What rumors relies on from before

This document is the dependence ledger of the `before` review: every guarantee that `rumors` (the consumer at the workspace root) relies on from `before`, checked against `before`'s public contract and its committed tests, followed by the findings whose primary class is dependence, then the positives and the open questions the sweep produced. It also carries the one intra-crate dependence finding of the review (the paper oracle's import of a production cost type). Findings of other classes that bear on a ledger row are cross-referenced by id and never restated; their full records live in the class documents beside this one. Ids are `<partition or sweep key>-<n>`; the full record of each finding lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file, and the ledger's own record is the sweep's `ledger.md` (in the review scratchpad at `sweeps/rumors-dependence/ledger.md`, reproduced in the tables below), re-checked by the sweep's finalization pass, whose record is `evidence/sweeps/rumors-dependence.md`. Severity runs high, medium, low, nit. Provenance: *demonstrated* means a constructed test ran; *executed* means a run settled it; *verified* means mechanically checked or re-derived; *assessed* means read.

What this pass verified itself, at the briefed commit `9e5784fb4dce977cfbdfd1619886d1482b5ce764` through `git show`: every `laws.rs` line the ledger cites (73 line numbers) resolves to the law it names; the `before`-side documentation anchors of the ledger rows I sampled (about fifty lines across `party.rs`, `version.rs`, `version/own.rs`, `clock.rs`, `serde_impls.rs`, `auto_traits.rs`, `causally.rs`, `causally/query.rs`, `span.rs`, `version/ticks.rs`, `version/ranked.rs`, and `lib.rs`) and the `rumors`-side use sites of every finding (about forty lines) hold as quoted. The working tree's HEAD is `7440d1a3`, two commits above the briefed commit; `git diff --stat` between the two over every in-scope path is empty (both commits touch only `.agent-notes/`), so an anchor read at HEAD is an anchor at `9e5784fb`. Every other anchor in the ledger is the sweep's reading, re-checked by the sweep finalizer at each finding's sites. This pass ran no cargo, just, or test command.

## Highest-value items

1. rumors derives leaf identity from version bytes and panics on version reuse; the premise (a tick changes the version only inside the ticking party's region, so ticks by disjoint parties never coincide) is pinned by three laws and stated in no public rustdoc of `Version::tick`, `Party::tick`, or `Clock::tick`. (rumors-dependence-1)
2. The bytes rumors hashes into leaf paths and ships on the wire are certified canonical by `party_codec_roundtrip` and `version_codec_roundtrip`; the `as_bytes == encode` laws and tests that the surface roster's `CODEC_PINS` cites compare one expression with its own copy, since `encode` is `as_bytes().to_vec()`. Classed verification-gap. (rumors-dependence-3)
3. rumors' bookmark reclaim gate, `clock.own_version() <= *version`, and its suppression token, `v / p == version / p`, route through the masked comparison kernel (`masked::causal_cmp`), whose `block_skip` re-reads a stationary cursor's trailing run every round: a quadratic term against the linear guarantee, invisible to every committed meter. Classed claim, severity high. (skyline-sweep-place-masked-5)
4. `Version::join` is `Θ(depth × width)` on a flat wide leaf joined against a spine of right-child pairs, against the published `O(|self| + |other|)`; rumors' reconciliation joins at every insert (`src/tree/traverse/act.rs:148`) and its window pricing rest on the linear contract. Classed claim, severity high. (skyline-coding-9)
5. rumors charges a cross-side bound the sum of two exchanged bounds, citing before's "pinned join- and meet-subadditivity lemmas"; the lemma is derived and pinned inside `meter/tier2/tests.rs` and stated in no public contract of `join` or `meet`. (rumors-dependence-2; the same lemma from the fold side is version-core-5)
6. rumors' `meter` feature reads `Party::encoded_bits`, `meter::span_traffic`, `limb_ops`, `scan_bits`, `touch_ops`, `registry::Shape`, and `Packed::version`; six partition reports ask independently whether the meter-gated surface is covered by before's stability rule, and meter-core-3 proposes splitting `Packed`. The answer decides how exposed rumors' metering tests are. (crate-wide pattern; meter-core-3)
7. before's serde `Deserialize` impls request a `seq` while its `Serialize` impls emit `bytes`; rumors' only serde path, `Version::deserialize` under ciborium (`src/remote/codec/frame.rs:364-365`), works because ciborium bridges a CBOR byte string into a seq. Classed correctness. (crate-root-34)
8. `dangerously_alias`'s Warning says the second copy "must be dropped without further use"; rumors' bookmark compares, encodes, persists, and after a crash decodes and joins aliases, inside the hazard's purpose and outside its letter, as do before's own examples and laws. Classed documentation. (rumors-dependence-4)
9. rumors presents `Span::dominance`'s coincident `O(1)` rung as before's cost contract; before states it only in code comments and a private test, and no meter on either side counts it. (rumors-dependence-5)
10. `Query::coverage`'s `Partial` refinement sweeps the clamped endpoint once per live hole, `Θ(k·|hi|)` against a published `O(|self| + |span|)`; rumors prunes and promotes subtrees through `Query::coverage(node.span())` with single-hole queries today. Classed claim, severity high. (span-causally-36)
11. `Version::tick` can panic when the fused walk's memo ledger passes `2^32` links, a bound reachable by input scale and undisclosed at the public `tick`; rumors ticks once per action. Classed correctness. (skyline-fill-grow-23; the same cap is inventory-1 and recursion-5)
12. The paper oracle imports production's route-cost saturation (`RouteCost::deepen`, `RouteCost::INFEASIBLE`) for no observable payload; replacing `deepen` with `component + 1` passes every test. (oracle-laws-4)

## Crate-wide patterns

**Contracts stated in tests, not at the method.** The ledger's three undocumented rows share one form: the guarantee is pinned by a law or a proptest and derived in test-module prose, while the public method doc stops short of it. Tick region-locality lives in `laws.rs:2460-2469` and the laws module header; join/meet subadditivity lives in the rustdoc of a test constant, `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS` (`meter/tier2/tests.rs:326-346`); the coincident span's single-buffer routing lives in code comments in `span.rs` and the private `is_coincident`. rumors, unable to cite a public statement, cites test names and "pinned lemmas" (`src/tree/mirror/streaming/window.rs:326-329`, `src/tree/traverse/unknown/tests.rs:29-31`). The version-core partition found the same lemma unstated from the fold's side (version-core-5), and the surface roster's `CODEC_PINS` cites a test that certifies nothing (rumors-dependence-3), so a consumer reading before's pins as its contract can be misled in both directions.

**The meter-gated instrument surface is rumors' second API, with an unsettled stability status.** rumors' dev-dependency enables `before/meter` (`Cargo.toml:149`) and its own `meter` feature implies `before/limb-meter` and `before/scan-meter` (`Cargo.toml:122`); under it rumors reads `Party::encoded_bits` (`tests/party_conservation.rs:314,332,378`), `meter::span_traffic` (`src/tree/tests.rs:1398-1417`), `limb_ops`, `scan_bits`, `touch_ops`, `registry::Shape`, and `Packed::version` (`src/tree/typed/untyped/tests.rs:644-684`). before's Cargo comment names external instrument crates as the intended reader (`crates/before/Cargo.toml:60-67`), and the meter-core report confirms `touch_ops`/`reset_touch_ops` exist for exactly this consumer. Whether that surface falls under `AGENTS.md`'s stability rule is asked, independently, by the board-families-floors-judge (question 8), board-frame (question 4), clippy-pedantic (question 1), codec-base-text-tree (question 4), module-graph (question 2), and skyline-sweep-place-masked (question 1) reports; the recursion sweep marks recursion-1 owner-gated because dissolving the segments currency removes `meter::stack_segments`. Findings that would move rumors' tests if landed: meter-core-3 (split `Packed` into id and event types, changing `Packed::version`), module-graph-3 (move `hull_traffic`/`web_traffic` under a counter feature; rumors' `meter` already enables `scan-meter`). rumors does not read the segments column.

**The ledger records values; rumors also relies on costs.** The sweep's columns ask what a guarantee returns, and every value row is documented-and-pinned. The high-severity findings that touch rumors are all cost claims on operations the ledger lists as sound: join (skyline-coding-9), masked comparison behind `OwnVersion` (skyline-sweep-place-masked-5), `Query::coverage` with holes (span-causally-24, span-causally-36), and the log-factor clause of `rank`'s `O(M(|v|) · log |v|)` with no committed instrument (skyline-query-9; rumors materializes a rank per ingested leaf at `src/rumors/causal.rs:101`). rumors' own pricing (`src/tree/mirror/streaming/window.rs:320-358`, `src/tree/typed/untyped.rs:547-554`) and its bookmark gate assume the published bounds. The table *Costs rumors relies on* at the end of the ledger carries that column as far as this sweep recorded the sites.

**Cross-crate citations run unchecked in both directions.** `just citecheck` runs `tools/citecheck --root crates/before` over `-p before`'s test inventory (`justfile:291-292`), so rumors' prose citing before law names is never resolved (`src/tree/traverse/unknown/tests.rs:29-31`), and suanpan's claims roster cites `../before/tests/meter.rs` by relative path (`crates/suanpan/src/claims.rs:99`). In the reverse direction, before's `borsh_impls/tests.rs:225` links `crate::bookmark`, a rumors module (crate-root-10), and `version/tests.rs:115-117` describes "Callers that track version-size maxima" without naming them.

## The ledger

Column key: **Guarantee** is stated in before's terms; **Doc** is before's public rustdoc site; **Pins** are the laws (by name, with their `crates/before/src/laws.rs` line), tests, and differentials that would fail on loss; **rumors uses** and **rumors tests** are the consumer's sites and the suites that would notice a change in before; **Status** is one of documented-and-pinned, documented-unpinned, undocumented, outside-contract. before paths are relative to `crates/before/src/` unless spelled out; rumors paths (`src/`, `tests/`, `Cargo.toml`) are relative to the workspace root. Every law line below was re-resolved by this pass; a dagger (†) marks an anchor whose cited lines, or the opening lines of a cited range, this pass re-read at the briefed commit; the rest are the sweep's, re-checked by its finalizer.

### Feature and instrument reaches

| Reach | Where | Standing |
|---|---|---|
| `before = { features = ["serde"] }` (production) | `Cargo.toml:125`† | `src/remote/codec/frame.rs:364-365` decodes `Version` through ciborium; `serde_impls.rs:39-44`† routes to `Version::decode`; serde bytes equal canonical bytes per `serde_impls.rs:3-7`†, pinned by `clock/tests.rs:1095 serde_bytes_pin_the_canonical_encoding` and `:1122 serde_rejects_non_canonical`. See crate-root-34 for the `bytes`/`seq` mismatch ciborium bridges. |
| `before = { features = ["serde", "meter"] }` (dev) | `Cargo.toml:149`† | `tests/party_conservation.rs:314,332,378` read `Party::encoded_bits` (meter-gated, `party.rs:576-600`†). |
| rumors feature `meter = ["before/limb-meter", "before/scan-meter"]` | `Cargo.toml:122`†; both imply `meter` (`crates/before/Cargo.toml:84,95`) | `src/tree/tests.rs:1398-1417`† (`meter::span_traffic`), `src/tree/typed/untyped/tests.rs:644-684` (`limb_ops`, `scan_bits`, `touch_ops`, `registry::Shape`, `Packed::version`), `src/tree/traverse/unknown/tests.rs:5-7`; all under `#[cfg(feature = "meter")]`. Stability status unsettled (crate-wide pattern above). |
| `before::laws` | none in code; name citations at `src/tree/traverse/unknown/tests.rs:29-31`† | Prose citation of test-only law names, not checked by `tools/citecheck`. |
| `before::oracle`, `before::surface` | none | Not reached. |
| `pub use ::before;` and `pub use before::{Ticks, Version, causally};` | `src/lib.rs:328-330`† | rumors re-exports the whole crate: before's API stability is rumors' API stability. `Ticks` appears in `Error::NetworkMismatch` (`src/error.rs:84-99`†), `Span` in the public `Backend::Node::span` (`src/tree/mirror/streaming/backend.rs:252`). |

### Party

| Guarantee | Doc | Pins | rumors uses | rumors tests | Status |
|---|---|---|---|---|---|
| `Party::seed()`: one seed per interacting system; descendants pairwise disjoint | `lib.rs:224-229`† (Causal Singularity), `party.rs:102-110`† | `seed_covers_every_party`, `is_seed_iff_equals_seed` 2137; `party_join_all_reunites_forks_at_any_width` 2358 | `src/peer.rs:219`† (once per universe) | `tests/party_conservation.rs:83-96` (fold-join equals seed), `tests/common/sim.rs:1017-1055` | documented-and-pinned; rumors' `Network` id (`src/peer.rs:39-41`, `src/peer/gossip.rs:1157-1163`†) is its mechanism for never mixing universes |
| `Party::fork`: halves disjoint; parent covers both; join restores | `party.rs:210-237`† ("Splits off a new disjoint Party") | `fork_halves_disjoint` 2033, `fork_join_roundtrip` 2023, `fork_halves_covered_by_parent` 2044; differential `d_fork_join_roundtrip` (`party/tests.rs:323`) | `src/peer/gossip.rs:705`† (donate a fork on bootstrap serve) | `tests/party_conservation.rs:237-262` (donated exactly once), `:308-337` (returns to baseline, `encoded_bits` size bound) | documented-and-pinned; the fork is taken at the snapshot version inside one critical section (`src/peer/gossip.rs:645-665`†) |
| `Party::join`: `Ok` iff disjoint; `Err(other)` leaves `self` unmodified; result covers both; result bytes canonical | `party.rs:278-306`† (`# Errors`) | `join_defined_iff_disjoint` 2196, `join_commutative_outcomes` 2203, `join_covers_both_and_without_undoes` 2217, `join_overlap_hands_back` 2073, `party_codec_roundtrip` 2143 (the live pin for canonical padding after join; see rumors-dependence-3) | `src/peer/gossip.rs:820`†, `:1394` (absorb retiree; guard recovery), `src/bookmark.rs:452`† (reclaim) | `tests/bookmark_causality.rs:1093-1094 retire_into_rebooted_absorber_absorbs_cleanly` (`as_bytes` decodable after join), `tests/party_conservation.rs:264-291` | documented-and-pinned; `src/peer/gossip.rs:872-874` turns `Err` into `Error::PartyOverlap`, a conformance detector consistent with the model of record |
| `Party::join_all`: fold succeeds on a pairwise-disjoint family; equals seed | `party.rs:308-324`† | `party_join_all_accepts_iff_family_pairwise_disjoint` 2330, `party_join_all_reunites_forks_at_any_width` 2358 | `tests/party_conservation.rs:89` (accounting fold over aliases) | self | documented-and-pinned for the `Ok` arm rumors uses; the `Err` hand-back prose is disputed by party-8 and crate-root-18 (coalesced groups), which rumors does not read |
| `Party::is_disjoint`: exact disjointness of regions; symmetric | `party.rs:374-382`† | `disjoint_symmetric` 2184, `join_defined_iff_disjoint` 2196; differential `indexed_disjointness_matches_the_cursor_walk` (`party/tests.rs:384,408`) | `tests/party_conservation.rs:73`, `tests/common/sim.rs:648,1034`, `src/tree/arb.rs:692` | `src/tree/arb.rs:685-698 distinct_indices_are_pairwise_disjoint` | documented-and-pinned |
| `Party::covers`: `self ⊇ other` | `party.rs:405-407`† | `covers_reflexive` 2083, `covers_antisymmetric` 2179, `covers_transitive_constructed` 2090, `covers_transitive_incidental` 2266, `disjoint_excludes_covering` 2190 | `src/bookmark.rs:463`† | `tests/bookmark_causality.rs` (`store_parties` 216-222, coverage invariants 900-960) | documented-and-pinned |
| `Party::without`: `None` iff other covers self; else remainder ⊆ self, disjoint from other; a disjoint other is a no-op | `party.rs:431-435`† | `without_characterization` 2231, `without_inverts_fork` 2119, `without_disjoint_is_noop` 2240, `join_covers_both_and_without_undoes` 2217 | `src/bookmark.rs:378` (slice the donated region out of the record) | `tests/bookmark_causality.rs` coverage and leak checks | documented-and-pinned |
| `Party::dangerously_alias`: alias is byte-identical and overlaps the original; caller keeps at most one copy live | `party.rs:504-536`† ("The caller must ensure that at most one of the two copies is ever treated as live; the other must be dropped without further use") | `alias_is_byte_identical_overlap` 2130, `never_disjoint_from_self` 2106 | `src/peer.rs:733`†, `src/bookmark.rs:421,468†,478`, `src/rumors.rs:422` | `tests/bookmark_causality.rs` (whole suite), `tests/party_conservation.rs` | outside-contract (letter), inside its purpose: rumors compares (`src/bookmark.rs:296`†), encodes and persists (`src/bookmark/format.rs:396`) the alias, and after a crash decodes and joins it into a new live party; before's own examples use aliases read-only (`party.rs:419,452-453`). rumors-dependence-4 |
| `Party::as_bytes`: bytes are the canonical encoding, decodable by `Party::decode` | `party.rs:678-681`† | `party_codec_roundtrip` 2143 (independent pin through strict decode); `party_as_bytes_matches_encode` 2156 is tautological (rumors-dependence-3) | `src/tree/mirror/party.rs:69`† (wire hand-off) | `retire_into_rebooted_absorber_absorbs_cleanly`; `tests/gossip_snapshot.rs` pins hand-off bytes where a party crosses | documented-and-pinned |
| `Party::decode`: strict, canonical-only, typed `Decode` errors, iterative | `party.rs:602-641`†, `error.rs:56-92` | `codec/tests.rs` `reject_noncanonical_id` 874, `reject_trailing_bits` 1209, `reject_truncated` 1195, `bit_flip_rejects_or_decodes_canonically` 1312, `decode_never_yields_anonymous_party` 1165, `padding_perturbation_rejects` 1438; `clock/tests.rs:565 deep_tree_stack_safety` | `src/tree/mirror/party.rs:132`† (untrusted wire bytes) | `src/tree/mirror/party/tests.rs` | documented-and-pinned; rumors maps `Decode::Io` separately and wraps the rest (`src/tree/mirror/party.rs:131-138`†). The missing `# Errors` section is fresh-eyes-2 and api-audit-8 |
| `Party::is_seed`: equals the undivided seed | `party.rs:136-139` | `is_seed_iff_equals_seed` 2137 | `src/peer/gossip.rs:374` | `tests/bookmark_attach.rs` (a pristine seed persists nothing) | documented-and-pinned |
| `Party::encoded_bits`: exact live bit length (meter surface) | `party.rs:576-585`† | `party_encoded_bits_matches_encode_len` 2161 | `tests/party_conservation.rs:314,332,378` | self | documented-and-pinned (meter-gated instrument, dev-dependency only) |
| `Party` literal and text constructors | not used: rumors builds parties only by `Party::seed` plus `fork` chains (`src/tree/arb.rs:20-27`) | n/a | n/a | n/a | not relied on |

### Version

| Guarantee | Doc | Pins | rumors uses | rumors tests | Status |
|---|---|---|---|---|---|
| `Version::new`/`default`: the empty version; lattice bottom; join identity | `version.rs:109-120`†, `:1333-1344` | `new_is_the_bottom` 296, `merge_new_is_identity` 301, `is_empty_iff_new` 311 | `src/tree.rs:125`, `src/peer.rs:686,700`, `src/tree/traverse/act.rs:140,156`, tests | wire snapshots pin the empty version's bytes (`tests/gossip_snapshot.rs`) | documented-and-pinned |
| `Version::tick(&party)` strictly advances: `v < v.tick(p)` | `version.rs:161-166`† says "Advances this version by one event"; strictness is public by example at `version.rs:178`†, `party.rs:178`, `clock.rs:100` | `tick_strictly_advances` 2454 (both law populations and the fuzz target) | `src/tree.rs:440`† (one tick per action) | `tests/redaction.rs`, `tests/single_peer.rs`, `tests/causal.rs`; `src/tree/traverse/act.rs:169-176`† asserts on reuse | documented-and-pinned (by example) |
| `Version::tick(&party)` changes the version only within `p`'s region, hence stamps are unique across disjoint parties (rumors' identity premise, `src/tree.rs:83-89`†, `src/tree/typed/path.rs:24-27`†, `src/tree/traverse/act.rs:30-38`†) | no public rustdoc; region-locality appears only in `laws.rs:2445-2446,2460-2469` (2460-2463 re-read†) and the paper | `tick_only_inflates_the_region` 2463, `tick_advances_within_the_region` 2474; no direct cross-party distinctness law | as above | as above | undocumented (pinned). rumors-dependence-1 |
| `Version::ticks(&party, n)` equals `n` sequential ticks | `version.rs:184-187`† | `ticks_agrees_with_iterated_ticks` 2510, `ticks_composes` 2636, `ticks_line_realizes_min_ticks` 2537 | `src/tree/arb.rs:48,112`, `src/tree/tests.rs:78` (fixtures) | fixtures only | documented-and-pinned |
| `PartialOrd` (`<`, `<=`, `partial_cmp`): causal containment; antisymmetric; `Eq` iff `Some(Equal)`; dual; concurrency is `None` | `version.rs:41-60`†, `lib.rs:275-279` | `order_reflexive` 291, `order_antisymmetric` 508, `eq_iff_cmp_equal` 518, `partial_cmp_is_dual` 524, `concurrent_iff_incomparable` 529, `order_transitive_constructed` 896, `order_transitive_incidental` 904; differential `compare_matrix_matches_oracle` (`version/tests.rs:73`) | `src/tree/traverse/act.rs:152`†, `src/peer/gossip.rs:865-868`, `src/bookmark.rs:450`†, `src/tree/traverse/unknown.rs:84` (through `causally::before`), `tests/bookmark_causality.rs:206` | `tests/causal.rs` (causal delivery), `tests/redaction.rs` (deletion honoring) | documented-and-pinned |
| `Eq`/`==`/`!=`: byte equality is value equality | `version.rs:62-66`† ("byte equality is exactly causal equality"), `lib.rs:70-74` | `version_eq_iff_bytes_eq` 581, `eq_iff_cmp_equal` 518, `version_eq_implies_hash_eq` 601; `codec/tests.rs:842 canonical_encoding_is_injective`, `version/tests.rs:2212 byte_equality_matches_bit_equality` | `src/tree/traverse/act.rs:171`†, `src/rumors/changes.rs:91,148`, `src/peer/gossip.rs:1007` | `tests/gossip_when.rs` (suppression token equality), snapshots | documented-and-pinned |
| `join` (the `\|` and `\|=` operators): least upper bound; commutative, associative, idempotent | `version.rs:438-461`†, operator table `:48-56` | `merge_is_least_upper_bound` 864, `merge_commutative` 448, `merge_associative` 852, `merge_idempotent` 280; differential `join_matrix_matches_oracle` (`version/tests.rs:199`) | `src/tree.rs:527,600`, `src/tree/traverse/act.rs:148`†, `src/rumors/causal.rs:103`†, `src/tree/arb.rs:114` | `tests/multi_peer.rs`, `tests/pairwise.rs` (convergence) | documented-and-pinned for the value; the linear cost contract is disputed by skyline-coding-9 |
| `join`/`meet` encoding subadditivity: `size(c) <= size(a) + size(b) - 2` for `c` either lattice operation | none: `Version::join`, `Version::meet`, the operator table, and the Space Efficiency section state no size relation | `join_encoding_is_subadditive`, `meet_encoding_is_subadditive` (`version/tests.rs:109-161`), the tier2 grid across four emitters (`meter/tier2/tests.rs:326-346†, 417-453, 496-560`) | `src/tree/mirror/streaming/message.rs:79-88`, `src/tree/mirror/streaming/window.rs:320-337†, 349-358` (window pricing) | none directly; the mirror suites would catch a wrong bound only as a window overrun | undocumented (pinned). rumors-dependence-2; version-core-5 |
| `meet` (the `&` operator): greatest lower bound | `version.rs:498-521`† | `meet_is_greatest_lower_bound` 873 | only through `Span` folds and `src/iter.rs`' clamp | through bounds memos | documented-and-pinned |
| `Version::span_all`: tightest span `[meet_all, join_all]`; ordered by construction | `version.rs:598-604`† | `span_all_is_the_family_hull` 1843, `span_all_is_rotation_invariant` 1862 | `src/tree/typed/untyped.rs:596`† (fringe bounds memo) | `src/tree/typed/untyped/tests.rs:739-760` (memo equals sequential join/meet, metered) | documented-and-pinned |
| `Version::as_bytes`: canonical, injective (equal iff equal versions); lexicographic order arbitrary but deterministic; identical on both ends after strict decode | `version.rs:1155-1160`† | `version_eq_iff_bytes_eq` 581, `version_encoding_is_prefix_free` 594, `version_codec_roundtrip` 407 (independent pin); `version_as_bytes_matches_encode` 420 is tautological (rumors-dependence-3) | `src/tree/typed/path.rs:38`† (leaf path is SHA3 of canonical bytes), `src/tree/mirror/streaming/message.rs:150` (role-election tiebreak), `src/remote/codec/frame.rs:183`, `src/remote/codec/greeting.rs:70` (wire), `src/tree/typed/untyped.rs:424,435-436` (size aggregate), `tests/common/oracle.rs:69-71` | `tests/gossip_snapshot.rs` pins every wire byte including version atoms; `tests/disruption.rs:196,336` decodes keys back; `tests/causal.rs:605-621` checkpoint decode | documented-and-pinned; `message.rs:150-153`'s `unreachable!` on `Equal` is guarded by the caller's equality short-circuit and by `eq_iff_cmp_equal` |
| `Version::decode`: strict canonical-only; typed `Decode`; iterative | `version.rs:1095-1101`†, `serde_impls.rs:1-7`† | `codec/tests.rs` `reject_noncanonical_event` 1014, `reject_trailing_bits`, `reject_truncated`, `bit_flip_rejects_or_decodes_canonically`, `truncation_rejects_or_decodes_canonically` 1347; `clock/tests.rs:1122 serde_rejects_non_canonical`, `:773 decode_never_panics`, `:565 deep_tree_stack_safety` | `src/remote/codec/greeting.rs:178`, `src/remote/codec/capture.rs:573`, tests `tests/disruption.rs:196`, `tests/causal.rs:606,621`; serde path `src/remote/codec/frame.rs:364-365` | `src/remote/codec/decode/tests.rs`, `src/remote/codec/tests/error_atlas.rs` snapshot, `tests/wire_legibility.rs` | documented-and-pinned; the version atom's CBOR byte-string head is accepted by ciborium without spelling judgment (`frame.rs:82-84`, rumors-side, carried to the rumors review). Missing `# Errors`: fresh-eyes-2 |
| `Version::rank()`: `Rank: Ord` total; strictly monotone in the causal order; `(rank, bytes)` order equals `Ranked`'s | `version.rs:282-288`†, `lib.rs:291-300`, `ranked.rs:24-31`† | `rank_strictly_monotone` 542, `ranked_orders_by_rank_then_bytes` 615 (exactly the tuple rumors builds), `RANK_TRIPLE` group 2842 | `src/rumors/causal.rs:101`† (BTreeMap key `(Rank, as_bytes)`) | `tests/causal.rs` (`assert_causal`) | documented-and-pinned for the order; the log-factor clause of rank's cost has no committed instrument (skyline-query-9) |
| `Version::min_ticks()`: exact lower bound on ticks; `Ticks: Ord` numeric, unbounded | `version.rs:240-254`†, `ticks.rs:15-33`† | `min_ticks_zero_iff_empty` 358, `version/tests.rs:693 min_ticks_floors_every_history`, `ticks/tests.rs:44 wide_counts_round_trip_and_order`, `:70 addition_behaves_like_the_naturals` | `src/peer/gossip.rs:721,1160†,1271` (`Error::NetworkMismatch`, `BootstrapHistoryConflict`) | `tests/bookmark_causality.rs:537-554` (the `(min_ticks, network)` rule), `tests/membership.rs` | documented-and-pinned |
| `Version::is_empty`: equals `Version::new()` | `version.rs:137`† | `is_empty_iff_new` 311 | `src/peer/gossip.rs:374,1267` | `tests/bootstrap.rs` | documented-and-pinned |
| `Version: Clone` is `O(1)` and buffer-sharing; clone identity feeds before's fast paths | `version.rs:84-93` (comment), derive at `:94` | `identity_fast_paths_agree_across_buffer_identity` (`version/tests.rs:2243`), `fold_clone_collapse_is_value_invisible` 2268 | everywhere (`Arc`-shared trees, snapshots) | n/a | documented-and-pinned; retained build capacity is unpinned (version-core-15) |
| `Version: Send + Sync + Unpin` | `auto_traits.rs:6`† (compile-time pin) | compile-time | watch channels, tokio | compile | documented-and-pinned |
| `Version: Serialize/Deserialize`: serde bytes equal canonical `encode`; deserialize is strict `decode` | `serde_impls.rs:1-12`†, `lib.rs:392-393` | `clock/tests.rs:1095,1122` | `src/remote/codec/frame.rs:364` (decode side only; the encode side is hand-written at `frame.rs:183`) | `tests/gossip_snapshot.rs` | documented-and-pinned under ciborium; the `bytes`-versus-`seq` mismatch is crate-root-34 |
| `Version::try_from(u64)`: literal leaf constructor (fresh universe) | `version.rs:1461-1478`† | `version/tests.rs:380-440` | `src/remote/adapter/tests.rs:71`, `tests/fan_occupancy.rs:46` (tests only; never mixed with a seeded universe) | n/a | documented-and-pinned |

### OwnVersion (projection)

| Guarantee | Doc | Pins | rumors uses | rumors tests | Status |
|---|---|---|---|---|---|
| `v / p` is `v` masked to `p`'s region; comparisons decide as the materialized projection would; `v / p <= v` | `own.rs:12-20`†, `version.rs:1678-1680`, `clock.rs:644-650`† | `projection_is_sub_version` 2545, `own_version_cmp_matches_materialized` 2651, `own_version_pair_cmp_matches_materialized` 2823, `disjoint_projections_share_nothing` 2770, `tick_only_inflates_the_region` 2463; `own/tests.rs:134 mirror_cells_agree_on_arbitrary_triples`, `version/tests.rs:2147 div_view_matches_materialization` | `clock.own_version() <= *version` at `src/bookmark.rs:450`† (reclaim gate); `v / p == version / p` at `src/bookmark.rs:296`† (suppression token) | `tests/bookmark_causality.rs` (an early reclaim violates its causality assertions), `tests/bookmark_transmit_window.rs`, `tests/stale_floor.rs` | documented-and-pinned for the value; the masked kernel's linear cost is disputed by skyline-sweep-place-masked-5 |

### Clock

| Guarantee | Doc | Pins | rumors uses | rumors tests | Status |
|---|---|---|---|---|---|
| `Clock::from_parts`/`into_parts`: pairs a party with a version; inverse pair | `clock.rs:574-612`† | `CLOCK_SOLO` laws 2959; `tests/stale_state.rs:79-94` (`from_parts` over an earlier version is "Valid by the model") | `src/bookmark.rs:377-379,420,451-453†,467`, `tests/common/*` | `tests/bookmark_causality.rs` | documented-and-pinned; rumors never ticks the record's clocks, it joins their parties after the own-projection gate |
| `Clock::encode`/`Clock::decode`: canonical concatenation party‖version; strict decode | `clock.rs:717-734†,765-823†` | `codec/tests.rs:786 decode_encode_roundtrip`, `:1274 assert_clock_accept_canonical`, `clock/tests.rs:826 decode_preserves_component_canonicity`, `:773 decode_never_panics` | `src/bookmark/format.rs:396,465` (disk) | `src/bookmark/format/tests.rs:475,494` insta pins (`frame_non_trivial` carries nested version bytes), `:113 record_round_trips` | documented-and-pinned; disk bytes pass the frame's SHA3 integrity check before `Clock::decode` (`format.rs:370-377`) |
| `Clock::seed`, `fork`, `tick`, `sync` in fixtures: one universe per fixture | `lib.rs:36-77` | master differentials (`clock/tests.rs:204`) | `src/conformance/backend.rs:682-693`, `src/bookmark/format/tests.rs:26`, `src/remote/adapter/tests/parking.rs:76`, `src/remote/codec/capture/tests.rs:47`, `src/remote/proxy/tests.rs:361` | n/a | documented-and-pinned |
| `Clock::own_version` is `version / party` | `clock.rs:644-650`† | `project_is_the_operator_spelling` 2574 | `src/bookmark.rs:450`† | as above | documented-and-pinned |

### Span, Dominance, causally

| Guarantee | Doc | Pins | rumors uses | rumors tests | Status |
|---|---|---|---|---|---|
| `Span<'static>` as the branch bounds memo; `Span::at(version)` for leaves; `Span::union_all`; `Span::reborrow`: `lo <= hi` by construction; union endpoints `lo_a & lo_b`, `hi_a \| hi_b` | `span.rs:24-48†,50-78`, `span/algebra.rs:61-65,93-96` | `span_union_is_the_containment_join` 1309, `span_union_of_points_is_span_all` 1935, `span_folds_match_the_sequential_operators` 1887, `at_is_the_coincident_hull` 343, `span_gate_admits_exactly_the_ordered` 647 | `src/tree/typed/untyped.rs:147†,528†,531,600†` | `src/tree/typed/untyped/tests.rs:739` | documented-and-pinned |
| `Span::at` is the coincident span: one buffer shared, so `place`/`dominance`/`precedence`/`contains` cost a single pairwise comparison (`O(1)` coincidence rung) | code comments only (`span.rs:176-180,247-254,304-311†`); `Span::at`'s `# Complexity` (`span.rs:156-158`†) prices construction alone | `span/tests.rs:185 at_builds_the_coincident_span` (private `is_coincident`); `coincident_span_rungs_agree_across_buffer_identity` (`span/tests.rs:812-844`); `tests/coincident_span.rs` (scan parity) | `src/tree/typed/untyped.rs:547-556`† (cost reliance stated as before's contract) | `src/tree/tests.rs:1398-1470 span_door_traffic` reads `meter::span_traffic`, which counts pair-hull constructions, not this rung | undocumented (pinned, unmetered). rumors-dependence-5 |
| `Span::dominance`: `After` iff `hi <= probe`; `Before` iff not `lo <= probe`; `Between` otherwise | `span.rs:270-283`† | `span_dominance_coarsens_place` 1132, `span_place_matches_relations` 1108, `degenerate_span_place_is_partial_cmp` 660; `span/tests.rs:67` | `src/tree/typed/untyped.rs:556`†, `src/tree/traverse/unknown.rs:48`, `src/tree/mirror/streaming/materialized/unknown.rs:66` (deletion-honoring classifier) | `src/tree/traverse/unknown/tests.rs` (two-pass oracle, pruned trees equal), `tests/redaction.rs`, `tests/retire_redaction.rs` | documented-and-pinned |
| `causally::before(e)` keeps `v <= e`; `since(s)` is `!before(s)`; `all` admits everything | `causally.rs:13-39`†, `causally/forms.rs:34-200` | `atom_membership_matches_relations` 772, `query_shorthands_are_their_expressions` 810 | `src/tree/traverse/unknown.rs:84`, `src/tree/mirror/streaming/materialized/unknown.rs:44`; `src/rumors/unordered.rs:105`, `src/rumors/causal.rs:95`; `src/tree/typed/node.rs:273` | `tests/causal.rs`, `tests/redaction.rs` | documented-and-pinned |
| `Query::coverage(span)`: `Empty` means no covered version is admitted; `Full` means all are; exact on coincident spans | `causally/query.rs:50-59†,109-113†` | `coverage_bounds_membership` 1039, `coverage_matches_membership_on_points` 1077, `causally/tests.rs:229 coverage_is_exact_on_the_two_party_grid`, `:189 coverage_clamp_refinement_is_exact` | `src/tree/typed/untyped/iter.rs:112,442` (prune `Empty`, promote `Full`) | `tests/causal.rs`, `tests/single_peer.rs` (observer completeness) | documented-and-pinned for the verdict; the linear cost claim fails for `k` holes (span-causally-24, span-causally-36); rumors' queries at the listed sites carry at most one hole |
| `From<Span>`/`From<&Version> for Query`: span to `after(lo) & before(hi)`; version to a singleton | `causally/convert.rs:45-76` | `query_shorthands_are_their_expressions` 810 | public `Snapshot::range` accepts them (`src/snapshot.rs:116-121`) | doctests at `src/snapshot.rs:136-152` | documented-and-pinned |
| `Span::decode`, `Span::new`, `Crossed`, named in the public `Backend::Node::span` contract: decode validates ordering; `new` returns `Crossed` unless `lo <= hi` | `span.rs:111-149`†, `error.rs:19-35`, `serde_impls.rs:104-113` | `span_gate_admits_exactly_the_ordered` 647, `span_codec_roundtrip` 749, `span/tests.rs:250-660` decode genre suite | `src/tree/mirror/streaming/backend.rs:238-252` (handed to backend implementors; rumors itself never calls them) | `src/conformance` | documented-and-pinned |

### Rank, Ranked, Ticks

| Guarantee | Doc | Pins | rumors uses | rumors tests | Status |
|---|---|---|---|---|---|
| `Rank` as a BTreeMap key: total order, `Clone`, causally monotone | `version.rs:282-288`†, `rank.rs:244,882,909` | `RANK_TRIPLE` 2842, `rank_strictly_monotone` 542, `version/tests.rs:853 rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs` | `src/rumors/causal.rs:63†,101†` | `tests/causal.rs` | documented-and-pinned |
| `Ranked`'s order is `(rank, canonical bytes)`; rumors mirrors it rather than using the type | `ranked.rs:24-31`† | `ranked_orders_by_rank_then_bytes` 615 | `src/rumors/causal.rs:54-63`† | `tests/causal.rs` | documented-and-pinned |
| `Ticks`: total order at any magnitude; `Debug`, `Ord`, `Clone` | `ticks.rs:15-33`† | `ticks/tests.rs:44,70` | `src/error.rs:84-99`† (`Error::NetworkMismatch`) | `tests/bookmark_causality.rs:551-554` | documented-and-pinned |

### Safety-rule audit

- **Causal Singularity.** rumors never mixes universes: the `Network` check precedes any reconciliation (`src/peer/gossip.rs:1155-1163`†), a bootstrap claimant must be newborn (`:1266-1274`), and the bookmark record is keyed by `Network` (`src/bookmark.rs:149,439`). The residual is rumors' own: two independently seeded universes with equal 128-bit `Network` ids (`src/peer.rs:213-215`).
- **Identity Linearity and the bytes hole.** The bookmark is a deliberate use of the documented Warning at `clock.rs:768-772`†. rumors' obligations are stated at the sites: persist before any own event crosses the wire (`src/peer/gossip.rs:645-665`†); reclaim only when `own_version <= live version` (`src/bookmark.rs:444-455`†); a restart re-bootstraps rather than resurrecting (`src/bookmark.rs:55-66`); one bookmark per peer, stored atomically (`src/bookmark.rs:84-91,104-117`). Within the model as before documents it (`tests/stale_state.rs:79-94`). Stamp uniqueness after reclaim additionally needs tick region-locality (rumors-dependence-1).
- **`dangerously_alias`.** Only read-only uses: compare, encode, accounting folds on aliases; never ticked or forked as a second live actor (`src/peer.rs:718-725`†, `tests/party_conservation.rs:12-14`†). The letter-versus-purpose gap is before's wording (rumors-dependence-4).
- **Decode of untrusted bytes.** Every ingress routes through the strict decoders (`src/tree/mirror/party.rs:132`†, `src/remote/codec/greeting.rs:178`, `src/remote/codec/frame.rs:364`, `src/bookmark/format.rs:465`) with typed errors; no `unwrap` on wire bytes.
- **No recursion on version depth in rumors.** rumors never walks version structure; its depth-bounded recursion is over the 32-level path only.

### Rows not documented-and-pinned, and their findings

| Row | Status | Finding |
|---|---|---|
| `tick` region-locality and cross-party stamp distinctness | undocumented (pinned) | rumors-dependence-1 (below) |
| join/meet encoding subadditivity | undocumented (pinned) | rumors-dependence-2 (below); version-core-5 (claim) |
| `dangerously_alias` read-only uses | outside-contract (letter) | rumors-dependence-4 (documentation) |
| coincident span `O(1)` rung as a cost contract | undocumented (pinned, unmetered) | rumors-dependence-5 (below) |
| `as_bytes` canonical: the roster-cited pin is tautological | documented-and-pinned through the roundtrip laws | rumors-dependence-3 (verification-gap) |

### Costs rumors relies on

The ledger above records what each guarantee returns; this table records what each hot-path operation costs, because every high-severity finding that touches rumors is a cost claim. One row per `before` operation rumors calls on a hot path, as the dependence entries and the ledger name them. **Contract** is `before`'s published cost and where it is stated; **Exception** is the constructed breach in the claims document, if one exists; **rumors' budget** is the consumer's own pricing or budget site as far as the sweep recorded it, and says plainly where it recorded none. The rumors sites are the sweep's (its ledger and finding anchors), not re-derived by this pass; the empty budget cells are work for the rumors review.

| Operation (rumors' hot-path site) | before's cost contract | Demonstrated exception | rumors' budget or pricing site |
|---|---|---|---|
| `Version::join` (`\|`, `\|=`) at every insert: `src/tree/traverse/act.rs:148`; also `src/tree.rs:527,600`, `src/rumors/causal.rs:103` | `O(\|self\| + \|other\|)` (`version.rs:438-461`; the `version_join` island) | skyline-coding-9 (claim, high, demonstrated): Θ(depth × width) on a flat wide leaf against a spine of pairs; scan per input bit 43.8, 86.5, 171.8, 342.5 as depth doubles | The sweep recorded that rumors' window pricing rests on the linear contract (highest-value item 4) and that the mirror window is priced at `src/tree/mirror/streaming/window.rs:320-358` and `message.rs:79-88` on the join/meet size lemma (rumors-dependence-2); it recorded no rumors site that budgets join's time. |
| The masked comparison behind `OwnVersion`: the bookmark reclaim gate `clock.own_version() <= *version` at `src/bookmark.rs:450`, the suppression token `v / p == version / p` at `src/bookmark.rs:296` | linear in the streams (`masked.rs:52-56`: every path bit pushed and popped at most once) | skyline-sweep-place-masked-5 with codec-bits-29 (claim, high, demonstrated): `block_skip` re-reads a parked cursor's trailing run every round; spilled words read grow ×4.000 on a ×2.000 input; no committed meter counts a stack-word read | None recorded: the ledger's `OwnVersion` row notes the disputed cost and names no rumors budget for the gate, which runs once per reclaim decision (`src/bookmark.rs:444-455`). |
| `Query::coverage(node.span())` pruning and promotion: `src/tree/typed/untyped/iter.rs:112,442` | `O(\|self\| + \|span\|)` (`causally/query.rs:114-117`); "linear time" in the `causally` module summary | span-causally-36 with span-causally-24 (claim, high, demonstrated): Θ(k·\|hi\|) for k live holes; 56 added holes add 578,644 scanned bits | None recorded. The sweep assessed that rumors' queries at these sites carry at most one hole (open question 8), so the k factor is 1 on today's paths; no rumors site budgets coverage's cost. |
| `Version::rank()` per ingested leaf: `src/rumors/causal.rs:101` (the `(Rank, as_bytes)` key) | `O(M(\|self\|) · log \|self\|)` (the `version_rank` island) | None demonstrated; the `log` clause has no committed instrument at its tier (skyline-query-9, claim). `Ranked::cmp`'s false linear contract (rank-33) is the sibling operation and is off rumors' path: rumors mirrors the order in its own key (`src/rumors/causal.rs:54-63`) rather than calling `Ranked::cmp` | None recorded. |
| `Version::tick` once per action: `src/tree.rs:440` | auxiliary space at most a small constant multiple of the input (`lib.rs:333-340`); the board enforces 16 B/B on its families | skyline-fill-grow-2 (claim, high, demonstrated): 50 to 105 transient heap bytes per input byte on the distinct-minima memo families, flat across a doubling. The `u32` link cap on the same path is a panic, not a cost (inventory-1, skyline-fill-grow-23) | None recorded. |
| `Span::dominance` on the coincident span `Span::at(version)`: `src/tree/typed/untyped.rs:547-556` | stated in code comments only (`span.rs:176-180, 247-254, 304-311`): one pairwise comparison on the coincident rung; `Span::at`'s `# Complexity` (`span.rs:156-158`) prices construction alone | None demonstrated; the rung is unmetered on both sides (rumors-dependence-5, this document). span-causally-26 (performance) records that the rung's `partial_cmp` sweeps to exhaustion when `hi > probe` strictly | `src/tree/typed/untyped.rs:547-556` states the O(1) coincidence rung as before's cost contract (recorded). |
| join/meet output size, a size bound rumors prices against: `src/tree/mirror/streaming/window.rs:320-337, 349-358`, `message.rs:79-88` | `size(c) <= size(a) + size(b) - 2`, stated in no public contract (rumors-dependence-2; version-core-5) | None; the constructed run for version-core-5 found no counterexample on 128 adversarial pairs | `window.rs:320-358` and `message.rs:79-88` (recorded): a cross-side bound is charged the sum of two exchanged bounds. |
| `Party::fork` on the bootstrap serve path: `src/peer/gossip.rs:705` | no cost contract recorded by the sweep | None; the fuzz-fit `SMALL_BAND_KERNELS` omit fork on the premise that it is off rumors' bootstrap hot path (open question 7) | None recorded. |

Where this table says *None recorded*, the sweep read rumors' use sites and found no budget, pricing constant, or cost comment at them; it did not search rumors for one elsewhere, so absence here is absence from the sweep's record, not a finding that rumors has no budget.

## Crate root and public types

### Version core

Related findings classed elsewhere: version-core-5 (claim, medium: the subadditivity lemma derived only in test prose, with the folds' auxiliary-space bounds resting on it); rumors-dependence-3 (verification-gap, low: the tautological `as_bytes == encode` laws and the roster pin that cites them); skyline-coding-9 (claim, high: join's re-anchor cascade is `Θ(depth × width)`); skyline-sweep-place-masked-5 (claim, high: the masked comparison behind `OwnVersion` re-peeks a stationary cursor); skyline-fill-grow-23, inventory-1, recursion-5 (correctness: the memo ledger's `u32` link cap panics at the public `tick`); crate-root-34 (correctness, medium: serde `bytes` versus `seq`); version-core-15 (verification-gap, low: retained build capacity in stored versions); fresh-eyes-2 and api-audit-8 (documentation: `# Errors` sections missing on `Version::decode`, `Party::decode`, `Clock::decode`); party-8 and crate-root-18 (documentation: `join_all`'s `# Errors` hand-back prose).

### rumors-dependence-1: Tick's event contract (strict advance, region-locality) is pinned by laws but stated in no public rustdoc of `tick`
- Where: crates/before/src/version.rs:161-182 (related: crates/before/src/party.rs:161-182, crates/before/src/clock.rs:88-105, crates/before/src/laws.rs:15-19, crates/before/src/laws.rs:2453-2478, crates/before/src/lib.rs:218-256, src/tree.rs:83-89, src/tree/typed/path.rs:21-29, src/tree/traverse/act.rs:30-38, src/tree/traverse/act.rs:161-176, src/peer/gossip.rs:645-665, src/bookmark.rs:444-455)
- Class / severity / confidence: dependence / medium / high
- Provenance: assessed (read); executed: no
- Verification: reframed: strictness is already public by example (`version.rs:178`, `party.rs:178`, `clock.rs:100`, the tutorial at `lib.rs:195-198`); what the public contract lacks is region-locality and its consequence, distinct stamps across disjoint parties; history: no-rationale-found (the laws landed with 86dd53a71 and b3f09baa0; nothing records why the contract stayed out of `tick`'s rustdoc)
- Owner-gated: yes: public rustdoc on the stable API, and a new public law name under the `laws` feature

rumors derives leaf identity from version bytes and panics on version reuse, resting on two properties of `tick`: the result strictly dominates the input, and it differs from the input only inside the ticking party's region, so ticks by disjoint parties from any base never coincide. before pins both as laws (`tick_strictly_advances`, `tick_only_inflates_the_region`, `tick_advances_within_the_region`) driven over both law populations and the fuzz target, and states them in the laws module's header, but `Version::tick`, `Party::tick`, and `Clock::tick` say only that the version advances by one event; region-locality appears in no rustdoc outside the feature-gated laws module, and the safety-rules section connects identity disjointness to nothing about events.

Evidence:

       161	    /// Advances this version by one event for `party`.

    laws.rs:
        18	//! disjoint join with `fork` as its splitting inverse, events inflate strictly
        19	//! and only within the owned region, and `rank` is a strictly monotone

      2460	    /// `tick` inflates only within the party's region (§4: `e' = e + f·i`, zero
      2461	    /// outside `i`): projected onto the region's complement, the ticked version
      2462	    /// is unchanged. Vacuous only for the seed party, which has no complement.

    src/tree.rs:
        85	/// [`Path::for_leaf`](typed::Path::for_leaf)). Versions are unique per
        86	/// send — locally by tick, globally by party disjointness — so two
        87	/// content-identical messages sent at distinct moments occupy distinct
        88	/// leaves, and two leaves collide only when a version has been reused,
        89	/// which conforming peers cannot do.

    src/tree/traverse/act.rs:
        33	/// version or payload: version reuse. No input reaches that state —
        34	/// every production insert carries a freshly created version (a fresh
        35	/// tick strictly dominates the ceiling bounding every live leaf, and
        36	/// party linearity keeps regions disjoint), and no wire-derived leaf

Resolution: state the event contract on `Version::tick` and mirror it on `Party::tick` and `Clock::tick`: the result strictly dominates the input; projected onto any region disjoint from `party`, the result equals the input; hence ticks by disjoint parties from one base always differ. Optionally add the consequence as a `VERSION_PARTY_PAIR` law (for example `ticks_by_disjoint_parties_differ`: `!p.is_disjoint(q) || { let mut a = v.clone(); a.tick(p); let mut b = v.clone(); b.tick(q); a != b && (&b / p) == (v / p) }`); the one-world population inhabits the antecedent (`laws.rs:2782-2783`), and the roster and fuzz driver pick a new group member up by construction (`laws.rs:94-97`). Acceptance: the rustdoc of the three `tick` entries states both clauses; a reader of before's public docs can derive stamp uniqueness without the paper or the laws module.

### rumors-dependence-2: Join/meet encoding subadditivity is derived and pinned inside before's test tree and priced against by rumors, but stated in no public contract
- Where: crates/before/src/version.rs:438-461 (related: crates/before/src/version.rs:498-521, crates/before/src/version.rs:48-56, crates/before/src/lib.rs:310-340, crates/before/src/version/tests.rs:109-161, crates/before/src/version/tests.rs:481-528, crates/before/src/meter/tier2/tests.rs:326-346, crates/before/src/meter/tier2/tests.rs:417-453, crates/before/src/meter/tier2/tests.rs:496-560, src/tree/mirror/streaming/message.rs:79-88, src/tree/mirror/streaming/window.rs:320-337, src/tree/mirror/streaming/window.rs:349-358, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:359-373, :455-457)
- Class / severity / confidence: dependence / low / high
- Provenance: assessed (read); executed: no
- Verification: reframed: the sweep undersold before's side. The bound has a written derivation (the rustdoc of `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS`, a term-by-term argument with the tight constant of 2 bits and its equality witness) and two instruments (the churned and arbitrary proptests in `version/tests.rs`; the tier2 grid across four emitters, including the skyline kernel with its short-circuits stripped). The residual is only where the contract is stated; history: deliberate-and-holds for the pins (the agent note records the lemma of record and the 2026-07-23 ruling), no-rationale-found for keeping the statement out of the public rustdoc
- Owner-gated: yes: public rustdoc

rumors' window pricing charges a cross-side ceiling or floor the sum of the two exchanged bounds, citing before's "pinned join- and meet-subadditivity lemmas". before proves and pins the bound, derivation included, inside `meter/tier2/tests.rs` and its proptests, but `Version::join`, `Version::meet`, the operator table, and the Space Efficiency section state no size relation between a join or meet and its operands. A maintainer reading the join contract cannot learn that a consumer depends on the property, and rumors' comments can cite only test names.

Evidence:

    crates/before/src/version.rs:
       438	    /// The join (least upper bound) of this [`Version`] and `other`: their
       439	    /// combined causal history.
       440	    ///
       441	    /// Identical to the operator form `self | other`.

    crates/before/src/meter/tier2/tests.rs:
       331	/// For canonical `a`, `b` and `c` either their join (pointwise max) or meet
       332	/// (pointwise min), the Tier 2 sizes satisfy
       333	/// `size(c) <= size(a) + size(b) - 2`, term by term: the canonical output
       ...
       346	const JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS: u64 = 2;

    crates/before/src/version/tests.rs:
       115	    /// never invent structure beyond both inputs together. Callers that track
       116	    /// version-size maxima rely on this to charge a join of two bounded
       117	    /// versions the sum of their bounds. Probed here over churned

    src/tree/mirror/streaming/window.rs:
       326	    /// and a bound a session assembles across the two joins a ceiling —
       327	    /// or meets a floor — drawn from each side, encoding within the
       328	    /// pair's sum (`before`'s pinned join- and meet-subadditivity
       329	    /// lemmas); a node holds two bounds, hence the double. One priced

Resolution: add one sentence to `Version::join` and `Version::meet` (or one paragraph to the Space Efficiency section, linked from both): the canonical encoding of a join or meet is never longer than the sum of its operands' encodings. Optionally promote `join_encoding_is_subadditive` and `meet_encoding_is_subadditive` to `VERSION_PAIR` laws so the fuzz law target also drives them over decoded values. Acceptance: the statement appears in public rustdoc; rumors' `message.rs` and `window.rs` comments can cite the documented contract rather than "pinned lemmas".

### Span and causally

Related findings classed elsewhere: rumors-dependence-4 (documentation, nit: `dangerously_alias`'s "dropped without further use" forbids the read-only uses before's own examples, the laws module, and rumors' bookmark make; the row is in the Party table because the method is `Party`'s); span-causally-24 and span-causally-36 (claim: the fused query walks and `Query::coverage`'s `Partial` refinement pay a per-hole factor the public "linear time" contracts omit; rumors' queries at the ledger's sites carry at most one hole).

### rumors-dependence-5: rumors cites `Span::dominance`'s coincident fast path as a cost contract; before documents it only in code comments and a private test, and no meter on either side counts it
- Where: crates/before/src/span.rs:151-182 (related: crates/before/src/span.rs:303-332, crates/before/src/span.rs:546-555, crates/before/src/span/tests.rs:181-201, crates/before/src/meter.rs:3632-3646, src/tree/typed/untyped.rs:535-557, src/tree/tests.rs:1398-1447)
- Class / severity / confidence: dependence / nit / high
- Provenance: assessed (read); executed: no
- Verification: reframed: the sweep said rumors' `span_door_traffic` reads the rung meter to pin this traffic; `meter::span_traffic` counts pair-hull constructions (`Version::span`, `span_all` leaf combines, span-union point combines), not `dominance`'s `ptr_eq` rung, so the cost property rumors documents is unmetered on both sides; history: no-rationale-found
- Owner-gated: yes: public rustdoc (the alternative, a rumors-side rewording, belongs to the rumors review)

rumors' bounds-memo docs state that a leaf's `Span::at(version)` stores one version twice and that clone identity certifies the coincidence in `O(1)`, so classifying a leaf pays one decode of each stream, never two. `Span::at`'s public rustdoc gives `O(1)` for construction only; the coincident routing lives in code comments in `at`, `place`, and `dominance`, and in the private `is_coincident` and its private test. The documented `O(|self| + |version|)` bound on `dominance` covers the fast path, so nothing is wrong today; the finding is that a consumer presents as before's contract a property it can learn only from before's source.

Evidence:

    crates/before/src/span.rs:
       156	    /// # Complexity
       157	    ///
       158	    /// `O(1)`.
       ...
       309	        // Clone identity certifies the coincidence in `O(1)` so one
       310	        // single-bound placement (each stream decoded once) answers where the
       311	        // fused walk would read the shared buffer twice.
       ...
       553	    fn is_coincident(&self) -> bool {

    src/tree/typed/untyped.rs:
       551	    /// leaf's span stores its one version twice, and clone identity
       552	    /// certifies the coincidence in `O(1)`), so routing wholly through
       553	    /// [`span`](Self::span) pays a leaf one decode of each stream,
       554	    /// never two.

    crates/before/src/meter.rs:
      3640	/// kernel regime the consumer actually pays. Counts every pair-hull
      3641	/// construction: every [`Version::span`](crate::Version::span), every leaf
      3642	/// combine of `span_all`, and every point-combine of the span union doors
      3643	/// (`Span | Span` and [`Span::union_all`](crate::Span::union_all) on coincident
      3644	/// operands), which derive their hull through the same kernel. Process-global,

Resolution: either state under `Span::at`'s `# Complexity` that the coincident span shares one stored buffer, so `place`, `dominance`, `precedence`, and `contains` against it cost a single pairwise comparison (before-side, owner-gated), or have the rumors review soften `untyped.rs:548-554` to describe the routing without presenting it as before's contract. Acceptance: rumors' comment cites a public statement or makes none.

## The instruments

### Oracle and laws

Related findings classed elsewhere: rumors-dependence-6 (documentation, low: `crates/before/AGENTS.md:5-6`, `examples/code_study.rs:5-7`, and `version/skyline/build/tests.rs:394-395` cite the retired `implementation` module; found by the dependence sweep, classed documentation). The paper-fidelity sweep's question 4 asks the same question oracle-laws-4 answers below: whether the oracle's `RouteCost` coupling is acceptable given the oracle module's independence framing.

### oracle-laws-4: The oracle imports production's route-cost saturation for no observable payload, and the stated rationale names no constructible input
- Where: crates/before/src/oracle/version.rs:8-22 (related: crates/before/src/oracle/version.rs:287-289 and 299-311, crates/before/src/version/skyline/grow.rs:116-149, crates/before/src/testing/grow_brute_force.rs:31-38, crates/before/src/version/skyline/grow/tests.rs:63 and 319, crates/before/src/oracle/tests.rs:548-643)
- Class / severity / confidence: dependence / low / high
- Provenance: verified (grep for `grow_for_test`: the two production-side callers discard the cost with `let (raw, _) = ...`; the only cost consumers are oracle/tests.rs:560 and 579, which compare against `grow_brute_force`; read the `grow` arms: the `(Party::Leaf(false), _)` arm is reached only with an empty id at the top or a denormal `Node(empty, empty)`, since the `Node`/`Node` arm guards `il.is_empty()`/`ir.is_empty()` and every other arm recurses on nonempty ids; `git show fc06c5e8` shows the import and six `+ 1` rewrites landing together); executed: no
- Seen by: scaffolding [1], refutation (supporting item 3); refutation: confirmed; history: deliberate-and-holds (fc06c5e8's rationale is stated at oracle/version.rs:14-19 and grow.rs:137-142)
- Owner-gated: yes (a documented design decision with its rationale in code; this finding disputes the rationale)

The oracle module doc promises no second representation to keep in sync and ranks transcription fidelity above robustness; the `RouteCost` import is a sync point with production (fc06c5e8 changed 33 lines of the oracle alongside `grow.rs`) whose recorded rationale, "a debug overflow past the old bound" in the oracle's `+ 1`, names no input any harness constructs: `INFEASIBLE` originates only in an arm no normal nonempty id reaches, and a `u32` `+ 1` overflow needs `2^32` levels of an oracle tree that would overflow the stack long before (Principle 3: a coupling earns its place by naming what it serves outside itself). The brute force in the same suite made the opposite call and says why.

Evidence:

         8	use crate::version::skyline::grow::Cost as RouteCost;

        14	/// One more path level for a cost component, exactly the route DP's step
        15	/// ([`RouteCost::deepen`] at the production ceiling).
        16	///
        17	/// Infeasibility propagates, and feasible components saturate strictly below
        18	/// the [`RouteCost::INFEASIBLE`] sentinel, so a feasible chain of any depth
        19	/// stays feasible in every implementation of the fold at once.
        20	fn deepen(component: u64) -> u64 {
        21	    RouteCost::deepen(component, RouteCost::CEILING)
        22	}

       287	            (Party::Leaf(false), _) => {
       288	                (self.clone(), (RouteCost::INFEASIBLE, RouteCost::INFEASIBLE))
       289	            }

    grow/tests.rs:
        63	            let (raw, _) = to_oracle_version(v).grow_for_test(&to_oracle_party(p));

    grow_brute_force.rs:
        34	/// Deliberately unchecked exact arithmetic, no saturation and no infeasible
        35	/// sentinel: infeasibility is structural here (an empty enumeration /
        36	/// [`None`]), and depths are bounded by the enumerated test trees, so the
        37	/// brute force stays an independent witness of the DPs' saturating folds.

Resolution: Either give the oracle the brute force's idiom (`Option<Cost>` with `None` for an id that owns nothing here and plain `+ 1` deepening; the top-level empty-id arm returns `None`), drop the import, and remove "the recursive oracle" from grow.rs:137-140's client list; or keep the coupling and replace the doc at 14-19 with a rationale that names a constructible input (there is none under the harness caps, which is the point). Acceptance: `grep -rn RouteCost crates/before/src/oracle` is empty and the four grow-optimality proptests pass unchanged, or the doc at 14-19 states a reachable trigger.

Construction: Replace `deepen` with `component + 1` and the sentinel arm with `unreachable!`; every test in the crate passes (nothing observes the oracle's cost except the brute-force comparison, whose inputs are nonempty normal-form ids), which demonstrates the coupling carries no payload.

## Positives

Verified by the sweep's finalizer at the cited lines unless marked otherwise; this pass re-read the `†` anchors listed in the ledger.

- Every ingress of a before value into rumors routes through the strict decoders with typed errors and no `unwrap`: `src/tree/mirror/party.rs:131-138` maps `Decode::Io` separately and wraps every other defect as `HandOffMalformed`; serde deserialization of `Version` is `Version::decode` (`crates/before/src/serde_impls.rs:39-44`).
- rumors' mechanism for before's Causal Singularity rule is explicit: the `Network` check precedes any reconciliation (`src/peer/gossip.rs:1155-1163`), and the bookmark record is keyed by `Network` (`src/bookmark.rs:439`).
- The bookmark confronts the documented bytes hole with its obligations stated at the site: one critical section carries the persist-before-transmit and fork-at-snapshot obligations (`src/peer/gossip.rs:645-665`); reclaim gates on `clock.own_version() <= *version` with the reasoning written beside it (`src/bookmark.rs:444-455`); a restart re-bootstraps rather than resurrecting, and the reverted-bookmark hazard is named (`src/bookmark.rs:55-70`).
- `CausalMessages`' key `(Rank, Vec<u8>)` (`src/rumors/causal.rs:54-63`) is exactly the tuple before's law `ranked_orders_by_rank_then_bytes` (`crates/before/src/laws.rs:615-628`) pins equal to `Ranked`'s total order, so rumors' claim of sharing that order is pinned on before's side.
- `crates/before/src/auto_traits.rs` pins `Send + Sync + Unpin` for every public type at compile time; rumors' `Arc`-shared trees and watch channels depend on this without saying so, and the pin makes that safe.
- `encoding_views_agree_over_impl_history` (`crates/before/src/clock/tests.rs:296-341`) drives the implementation's own `fork`/`join`/`sync` and runs strict `decode` on `as_bytes`: the live before-side pin for the boundary rumors' `retire_into_rebooted_absorber_absorbs_cleanly` guards from outside.
- The subadditivity lemma comes with a tight constant and its extremal witness: `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS` (`crates/before/src/meter/tier2/tests.rs:328-346`) and `empty_pair_is_the_subadditivity_equality_case` (`:496-516`), checked across four emitters including the kernel without its short-circuits.
- The meter reach is gated on both sides: `pub mod meter` and `pub mod surface` sit under `#[cfg(any(test, feature = "meter"))]` (`crates/before/src/lib.rs:438-442`); rumors' `span_door_traffic` module is under `#[cfg(feature = "meter")]` (`src/tree/tests.rs:1398`).
- rumors states its alias discipline at the definition (`src/peer.rs:718-725`) and restates it at the head of the conservation suite (`tests/party_conservation.rs:12-14`).
- before's instrument surface was built for this consumer and says so: `Party::encoded_bits`'s doc names its instrument consumers by owner ruling 05d87e1b (clock report, dropped item 32; assessed from the report), the counter readers `touch_ops`/`reset_touch_ops` exist for rumors' `src/tree/typed/untyped/tests.rs:676` and `:682` (meter-core report, dropped item 8; assessed from the report), and the fuzz-fit `SMALL_BAND_KERNELS` price four kernels below 128 bits on the stated premise of rumors' bootstrap hot path (fuzzfit-bands report, question 8; assessed from the report).
- The clone-identity fast paths rumors' shared buffers ride on have a before-side liveness home: the fuzz-fit driver routes identity outcomes to before's `identity_fast_paths` meter pins, verified present at `crates/before/tests/meter.rs:10552` by the fuzzfit-bands finalizer (assessed from the report).

## Open questions for Finch

1. Does the event contract (strict advance, region-locality, hence distinct stamps across disjoint parties) belong in `tick`'s public rustdoc, given that rumors' identity model and bookmark reclaim rest on it? The laws exist; the question is only where the contract is stated (rumors-dependence-1). Recommendation: yes, on all three `tick` entries, with the optional pair law.
2. `just citecheck` runs `tools/citecheck --root crates/before` over `-p before`'s test list (`justfile:291-292`), so rumors' maintainer prose citing before law names (`span_place_matches_relations`, `span_dominance_coarsens_place` at `src/tree/traverse/unknown/tests.rs:29-31`) is never checked. Should citecheck's haystack include before's law names for cross-crate citations, or should rumors avoid citing before test and law names? Recommendation: once rumors-dependence-1, -2, and -5 land, rumors can cite public contracts and the question shrinks to the two law names; add a rumors root to citecheck with before's `laws::registered_names()` in its haystack rather than forbid the citations.
3. Where should the subadditivity derivation of record live: the rustdoc of a test-module constant (today), or the public contract of `join`/`meet` (rumors-dependence-2; version-core-5 asks the same from the fold's side)? Recommendation: the statement in the public contract, the derivation kept at the constant and linked from the fold's auxiliary-space bounds.
4. Is the meter-gated surface (`Party::encoded_bits`, `meter::span_traffic`, `limb_ops`, `scan_bits`, `touch_ops`, `registry::Shape`, `Packed::version`) covered by before's stability rule? Six partition reports ask this independently (crate-wide pattern above), and rumors' metering tests are the consumer. Recommendation: rule it an instrument surface outside the stability promise, with the rule that a change lands in the same commit as the rumors test update (both crates share the workspace); record the ruling once in `crates/before/AGENTS.md` and at the `pub mod meter` declaration.
5. Does rumors' long-lived version storage pay the retained-capacity slack version-core-15 describes (a spilling tick's `Vec` doubling can leave the resident cost near twice the encoding)? Recommendation: read the tick and hull resident rows once; if tick outputs dominate rumors' stored versions, decide whether `Bits::freeze` should trim, with the reading attributed at the parent (version-core report, question 9).
6. The version-core report's question 10: the span ladder's comparable-first rung order rests on no recorded number, and rumors' reconciliation workload's rung mix is already readable through `meter::span_traffic` in `src/tree/tests.rs:1398-1470`. Recommendation: record one reading in a decision record.
7. The fuzz-fit bootstrap family emits `ClockFork` below the small-band floor, and `SMALL_BAND_KERNELS` omits fork on the premise that it is off rumors' bootstrap hot path (fuzzfit-strategies report, question 4). The ledger shows `Party::fork` on the bootstrap serve path (`src/peer/gossip.rs:705`, one donation per served bootstrap). Recommendation: add fork to the roster and re-pin, or state at the constant why one fork per bootstrap does not count as hot.
8. `Query::coverage` and the fused query walks pay a per-hole factor (span-causally-24, span-causally-36); rumors' queries at the ledger's sites carry at most one hole (assessed from the sweep's site list, not re-derived here). Recommendation: restate before's contract with the hole count and pin the `k` axis; no rumors-side action unless a classifier starts conjoining `since` queries.
9. Should before's serde `Deserialize` accept `bytes` as well as `seq` (crate-root-34)? rumors decodes `Version` only through ciborium, which bridges, and hand-writes the encode side (`src/remote/codec/frame.rs:183`). Recommendation: the `serde_bytes` visitor on before's side; rumors is unaffected today and stays so.
10. The oracle's `RouteCost` coupling (oracle-laws-4; the paper-fidelity sweep's question 4): keep it and write a rationale naming a constructible trigger, or give the oracle the brute force's `Option<Cost>` idiom? Recommendation: the brute force's idiom; the construction in oracle-laws-4 shows the coupling carries no payload.
11. Carried from the sweep for the rumors review, not verified in this pass: `PartyGuard::drop` (`src/peer/gossip.rs:1384-1403`) recovering a speculative fork with only a `debug_assert!(false)` on the impossible failure; `parse_record` (`src/remote/codec/frame.rs:82-84`, `:364-365`) accepting the version atom's CBOR byte-string head without spelling judgment; and `src/tree.rs:83-89`'s collision premise also assuming SHA3-256 path collision freedom, which is rumors' own hashing assumption.

## Counts

Findings whose primary class is dependence: four. Cross-referenced findings of other classes are not counted here.

| Severity | Count | Ids |
|---|---|---|
| high | 0 | |
| medium | 1 | rumors-dependence-1 |
| low | 2 | rumors-dependence-2, oracle-laws-4 |
| nit | 1 | rumors-dependence-5 |

| Module | Count | Ids |
|---|---|---|
| Version core | 2 | rumors-dependence-1, rumors-dependence-2 |
| Span and causally | 1 | rumors-dependence-5 |
| Oracle and laws | 1 | oracle-laws-4 |

The rumors-dependence sweep's other three findings are classed elsewhere and appear in their class documents: rumors-dependence-3 (verification-gap, low), rumors-dependence-4 (documentation, nit), rumors-dependence-6 (documentation, low). Ledger rows: 47 guarantee rows across six tables (Party 13, Version 19, OwnVersion 1, Clock 4, Span and causally 7, Rank and Ticks 3): 42 documented-and-pinned, 3 undocumented (pinned), 1 outside-contract, 1 not relied on; plus six feature and instrument reaches and five safety-rule entries.
