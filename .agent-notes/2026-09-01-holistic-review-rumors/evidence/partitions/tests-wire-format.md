# Partition tests-wire-format: Integration tests: wire snapshots, CBOR evolution, dispute wire, wire legibility, snapshot liveness, payload depth, future size, send bounds

## Partition summary

This partition is the wire-format and payload-contract instrument layer of the integration suite. `tests/gossip_snapshot.rs` stages twelve gossip scenarios over the recording link in `tests/common/gossip_snapshot.rs` and pins every wire byte with insta, staging hash-dependent tree shapes deterministically through `tests/common/shape.rs` (send a pool, search the created versions for the required path shape, redact the rest). `tests/wire_legibility.rs` states the CBOR-sequence promise as a proptest over random gossip, bootstrap, and retire sessions, checked by a walker that knows only `ciborium::Value`. `tests/snapshot_liveness.rs` reverses insta's path resolution to convict any committed `.snap` no live test generates. `tests/dispute_wire.rs` pins the affine per-message wire law at three record sizes with exact integer quotients over seeded corpora, guarded by a negative control. `tests/cbor_evolution.rs` and `tests/payload_depth.rs` exercise the payload contract peer to peer: name-keyed decoding, clean failure on undecodable payloads, the default depth boundary for two type shapes, the `Some(None)` faithfulness case, and the placement of the depth-mismatch abort. `tests/future_size.rs` and `tests/api_send_bounds.rs` are static guards on the public futures' size and `Send`-ness. All eight files are test code; I read 2454 lines across them, plus the harness modules they drive (`tests/common/{wire,gossip_snapshot,shape,mod}.rs`), the gate recipes, and the src sites the prose cites.

The suite is strong where it matters most. Every shape-staged fixture asserts the tree shape it landed before the byte comparison, so a pin cannot degrade into a weaker wire form unnoticed; two fixtures add in-test liveness floors on the frame sequence; the calibration suite proves its counter alive and bounds the truncation remainder before pinning exact figures; the snapshot harness renders from the public observer hook while holding every hook item to the transport capture as a totality oracle; and sessions run under the closed-world quiescence poller almost everywhere, so a protocol stall fails at its source rather than at nextest's timeout.

The dominant defects are of two kinds. First, one instrument is dead: `tests/future_size.rs` is compiled out under `debug_assertions`, and no gate leg, CI job, coverage run, or mutants campaign runs rumors tests at a profile where they are off, so its three budget tests have never executed under any committed check; its module doc meanwhile names types and an erasure site that no longer exist. Second, a cluster of prose has outlived the code it describes and was not re-read at the streaming swap, the CBOR respelling, or the V1 retirement: V1 phase names in `gossip_snapshot.rs`, a 25-byte preamble the same test's snapshot renders as 30 bytes, a test doc claiming the crate documents evolution rules the owner removed from `lib.rs` by hand, a design-cell doc contradicting `window.rs` about what derives from `DISPUTE_WIRE_BYTES`, and an `api_send_bounds.rs` module doc whose "every" the API outgrew in June. The remainder is harness duplication (`seeded` in ten suites, a cross-typed bootstrap hand-rolled five times, three observer recorders, two `FixtureTree`s), a handful of under-specified assertions, and idiom nits.

## Findings

### tests-wire-format-1: Two gossip_snapshot testdocs state something other than what their snapshots pin
- Where: tests/gossip_snapshot.rs:12-14 (related: tests/gossip_snapshot.rs:629-638, tests/gossip_snapshot.rs:620, tests/snapshots/gossip_snapshot__same_live_content_divergent_versions.snap:16-43)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read line 620 `let a: Rumors<String> = seeded();`; read the same_live_content snapshot: greetings of 132 and 131 bytes, one `Responder stream 0 (height 31)` carrying `Match(End)` then `End(Stream)`, no supplies)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: no rationale found (line 12 over-claimed from birth, written in the same commit as `string_payload`; the disjunction dates to the commit that also accepted the snapshot answering it)
- Owner-gated: no

The module doc says the payload type is `u64` throughout, while `string_payload` uses `Rumors<String>`; and `same_live_content_divergent_versions`'s doc poses a disjunction ("pins whether X or whether Y") instead of stating the invariant the pinned snapshot already answers. AGENTS.md requires every test's doc comment to state the behavior it protects, and review holds it to that standard; a disjunction states none.

Evidence:

    12	//! The payload type is `u64` throughout: a small integer is one CBOR byte
    13	//! (`01`, `02`, …), which keeps the dumps short and lets distinct payloads
    14	//! be spotted directly in the hex.

    634	/// root hashes are therefore equal while their versions are not — so this pins
    635	/// whether the protocol short-circuits on the matching live hash or whether the
    636	/// version dominance (the same signal redaction propagation rides on) drives a
    637	/// reconciliation pass. There are no deletion markers in the protocol; the only
    638	/// trace of the redacted `2` is the advanced version.

Resolution: Line 12: "`u64` except where a scenario says otherwise" (bootstrap_snapshot.rs:19-21 already phrases this correctly). Lines 634-637: state the pinned outcome: unequal versions open the descent (no greeting short-circuit), the root's children match at the first stage, and no leaf crosses; the only wire trace of the redacted `2` is the advanced version. Acceptance: the doc names the observed shape (descent opens; one `Match(End)` at height 31; zero supplies), and the module doc no longer says "throughout".

### tests-wire-format-2: Fixture helpers the harness should own are re-spelled per suite: seeded, version_for, the leaf-prefix collector
- Where: tests/gossip_snapshot.rs:28-47 (related: tests/gossip_snapshot.rs:109-116, 276-283, 562-571; tests/bootstrap_snapshot.rs:33-43; tests/retire_snapshot.rs:40-44; tests/opening_supply.rs:25-28, 88; tests/network.rs:18-22; tests/observe.rs:258-261; tests/hop_trace.rs:546; tests/target_message_size.rs:85, 272; tests/listen.rs:484; tests/retire.rs:197; tests/changes.rs:49; tests/common/shape.rs:17-24)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn -E 'fn seeded|seed_rng\(' tests/` lists ten test files; `grep -rn -E 'fn version_for|then_some\(v\.clone\(\)\)' tests/` lists four; `grep -rn -F '[path[0], path[1]]' tests/` lists exactly the three gossip_snapshot sites)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (bootstrap_snapshot.rs's copy has said "Mirrors `gossip_snapshot::seeded`" since 2026-06-19; `common::shape` was extracted on 2026-08-18 and left the three inline prefix collectors in place)
- Owner-gated: no

Three fixture helpers are duplicated across suites rather than living in `tests/common`: the fixed-RNG floor-window peer constructor (ten copies, two of which already differ: opening_supply.rs omits `sync_window_floor()`, network.rs parametrizes the seed), the read-path version lookup (one named function plus three inline `find_map` copies that drop its uniqueness precondition and named panic), and the two-byte leaf-prefix collector with its hand-written pair-shape self-check (three copies in this file, beside a `common::shape` module that owns `leaf_path`, `path_radix`, and `shaped_pair`). Duplicated helpers drift independently; `tests/common` exists to own exactly this.

Evidence:

    31	fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>()
    32	-> Rumors<T> {
    33	    Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
    34	        .sync_window_floor()
    35	        .into_rumors()
    36	}

    41	fn version_for(rumors: &Rumors<u64>, value: u64) -> Version {
    42	    rumors
    43	        .snapshot()
    44	        .iter()
    45	        .find_map(|(v, m)| (*m == value).then_some(v.clone()))
    46	        .unwrap_or_else(|| panic!("no live message holds {value}"))
    47	}

    109	    let prefixes: Vec<[u8; 2]> = a
    110	        .snapshot()
    111	        .iter()
    112	        .map(|(v, _)| {
    113	            let path = leaf_path(v);
    114	            [path[0], path[1]]
    115	        })
    116	        .collect();

Resolution: Add `common::wire::seeded<T>(stream: u64) -> Peer<T>` (returning the `Peer` so callers may add knobs before `into_rumors()`), with the determinism rationale stated once; add `common::peer::version_of<T: PartialEq>(rumors, payload) -> Version` with the uniqueness precondition in its doc; add `common::shape::leaf_prefix` and an `assert_shaped` that re-verifies a landed pair. Replace every site. Acceptance: `grep -rn 'seed_rng(' tests/` and `grep -rn 'then_some(v.clone())' tests/` each hit only `tests/common`; `grep -c '\[path\[0\], path\[1\]\]' tests/gossip_snapshot.rs` is 0.

### tests-wire-format-3: The empty-pair testdoc states a 25-byte preamble; the test's own snapshot pins 30
- Where: tests/gossip_snapshot.rs:49-54 (related: src/tree/mirror/handshake.rs:12-17, tests/snapshots/gossip_snapshot__empty_pair_converges_immediately.snap:6)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read handshake.rs:16 `the item is 30 bytes, fixed`; read the snapshot's line 6 `control item 0 (30 bytes) / preamble /`; `grep -rn '25-byte' src tests` hits only this line)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate but expired (correct at f6cf2579 on 2026-07-16; the CBOR respelling 4dd2053c on 2026-08-19 made the preamble a 30-byte item and swept `25-byte` from src without touching this doc; the line has carried 8, 24, 8, 25, 29, and 25 across six commits)
- Owner-gated: no

The doc names a preamble width that contradicts both the handshake module and the snapshot this very test generates. AGENTS.md treats an inaccurate testdoc as a bug in the test, and the owner's doctrine forbids hand-maintained counts: this number has been rewritten five times and has rotted a sixth. Line 51 is also a stranded fragment left by the last reflow.

Evidence:

    49	/// Two empty peers: the minimal session.
    50	///
    51	/// After the 25-byte preamble
    52	/// the two sides exchange greetings, find their versions equal, and converge
    53	/// immediately with no content transfer: the protocol's shortest possible
    54	/// conversation.

Resolution: Drop the number ("After the fixed-width preamble the two sides exchange greetings ...") and reflow the paragraph; the snapshot's header is the pin of record for the width. Acceptance: the doc carries no byte count, and `grep -n '25-byte' tests/gossip_snapshot.rs` returns nothing.

### tests-wire-format-4: asymmetric_message_targets pins one direction of a symmetric negotiation
- Where: tests/gossip_snapshot.rs:139-147 (related: tests/gossip_snapshot.rs:150-157, src/tree/mirror/streaming/remote/proxy/start.rs:275-278)
- Class / severity / confidence: verification-gap / nit / medium
- Provenance: assessed (read `run_budget` at start.rs:275-278, which takes `ours.target_message_size.min(theirs.target_message_size)`, and the fixture, which sets `target_message_size(0)` only on the bootstrapped receiving side)
- Seen by: blind-spots; refutation: confirmed; history: no rationale found (the receiver-declares-zero direction was chosen at bdf74d45 as the semantic demonstration; the dual is not mentioned)
- Owner-gated: no

The doc claims the session honors the smaller of the two exchanged targets, but only the receiving peer declares zero; the dual, where the populated sender declares zero and the receiver keeps its default, is not pinned. The code's `min` is symmetric, so the risk is low, but the claim as written covers both directions and the test covers one.

Evidence:

    139	/// The session honors the smaller of the two exchanged message targets.
    140	///
    141	/// The same two-leaves-one-subtree fixture as [`batched_supply_run`],
    142	/// except the *receiving* (empty) peer declares a zero target. The

Resolution: Add the dual fixture (populated side at `target_message_size(0)`), or narrow the doc to "a receiver's smaller target is honored by its peer". Acceptance: both directions are pinned, or the doc claims one.
Construction: Copy `asymmetric_message_targets_unbatch_the_run`, apply `target_message_size(0)` to `a` (the populated side) instead of `b`, and snapshot; the pinned bytes should show two single-record Supply frames as in the existing snapshot.

### tests-wire-format-5: Snapshot tests whose docs promise a converged live set assert nothing about it
- Where: tests/gossip_snapshot.rs:418-421 (related: tests/gossip_snapshot.rs:62-68, 465-466, 611-616, 654-662; tests/common/gossip_snapshot.rs:476-516)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read `capture_gossip` and `capture_gossip_returning`: the drivers `expect("gossip A")` / `expect("gossip B")` and the harness asserts the control drain; nothing inspects the live sets; only `early_supplies_honor_redactions` at 381-404 takes the handles back and checks them)
- Seen by: blind-spots; refutation: confirmed; history: no rationale found (`capture_gossip_returning` arrived on 2026-08-19 for one test; no note records a byte-pins-only division of labor)
- Owner-gated: no

`fork_insert_redact` ("must converge both peers on the live set `{3, 4}`"), `redaction_only` ("converge on `{2}`"), `both_redact_the_same_message` ("converges idempotently on `{2}`"), `string_payload` ("converge on both"), and `one_sided_transfer` state post-session outcomes that only the human who accepted the snapshot ever judged. A snapshot re-accept is exactly the moment the semantic claim is unguarded: a re-accept under a wrong implementation would pin wrong bytes with no mechanical objection. The doc claims a stronger property than the test checks.

Evidence:

    418	/// Reconciliation must converge both peers on the live set `{3, 4}`: the two
    419	/// redactions are contagious and cross the wire alongside the two novel
    420	/// inserts, so the capture pins inserts, fork divergence, bidirectional
    421	/// transfer, and redaction propagation all at once.

Resolution: Use `capture_gossip_returning` in these scenarios and assert the live sets the docs name (a sorted-payloads `assert_eq!` per side), or reword the docs to claim only the pinned wire form. Acceptance: each doc's stated live set is asserted after the capture, or the doc no longer states one.
Construction: In `fork_insert_redact`, replace `capture_gossip(a, b)` with `capture_gossip_returning`, then assert both snapshots' payloads equal `[3, 4]`; the assertion passes today and would fail under a redaction-propagation regression that the snapshot alone would only catch as a byte diff a reviewer could re-accept.

### tests-wire-format-6: V1-era protocol vocabulary survives in gossip_snapshot testdocs
- Where: tests/gossip_snapshot.rs:484-499 (related: tests/gossip_snapshot.rs:447-453, src/tree/mirror/streaming.rs:106-109, src/tree/mirror/streaming/tests.rs:179-180, tests/snapshots/gossip_snapshot__deep_trie_divergence.snap:48,289,360,513)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn -E '\b(Exchange|Opening|Closing)\b' src/` hits only prose verbs at handshake.rs:297, streaming.rs:155, start.rs:209, gossip.rs:1276, transport.rs:36; `grep -l -E 'Opening|Closing|Complete|Exchange' tests/snapshots/*.snap` returns nothing; the only `Done` in src is `link::Done`; the pinned deep_trie headers are heights 31, 29, 31, 30)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: contradicts AGENTS.md hard rule 1 (the names were accurate against the alternating mirror on 2026-06-05; they expired at the streaming swap b3b877d9 and definitively at the V1 retirement 368da2a5, which edited this file's module doc and imports but left these lines)
- Owner-gated: no

Three testdocs describe the pinned sessions in terms of a protocol that no longer exists: `DEEP_TRIE_PER_SIDE` speaks of a "recursive `Exchange` descent" with `Opening`/`Closing`/`Complete` phases, `deep_trie_divergence` of "the protocol's recursive core", and `converged_forks_noop` of short-circuiting "to Done". No such type, variant, or state exists in `src/`; today's protocol is the fixed per-height stage schedule under `streaming::protocol`, descending two heights per stream, with equal versions resolving both sides "without opening the descent" (streaming.rs:108-109). AGENTS.md: nothing in the codebase refers to code that no longer exists. A reader calibrating what `deep_trie_divergence` protects learns the wrong mechanism, and "recursive" misdescribes a protocol whose design point is a non-recursive fixed-depth schedule. The same ghost sits at src/tree/mirror/streaming/tests.rs:180 (`Closing`/`Complete` words), outside this partition.

Evidence:

    486	/// Chosen so the two sides' leaves are numerous enough to collide in their
    487	/// leading hash byte, branching the trie past its root and so driving the
    488	/// recursive `Exchange` descent (and the `Opening`/`Closing`/`Complete` phases
    489	/// at more than one level) that the small scenarios never reach.

    449	/// Both peers carry identical content *and* identical version vectors, so the
    450	/// version exchange short-circuits the session to Done before any content is
    451	/// examined — zero transfer despite non-empty trees. The non-empty companion

    497	/// reconciliation must branch the prefix-trie and recurse down it, exercising
    498	/// the protocol's recursive core that the handful-of-messages scenarios leave
    499	/// untouched.

Resolution: Restate against the pinned snapshot: the disjoint sets collide in their leading path byte, so stages open below the root (the pin shows Responder streams at heights 31 and 29 and Initiator streams at heights 31 and 30) that the two-or-three-message scenarios never reach; drop "recursive". For `converged_forks_noop`: "the equal-versions resolution completes both sides without opening the descent". Fix the sibling ghost at streaming/tests.rs:180 in the same pass. Acceptance: `grep -n -E 'Exchange|Opening|Closing|to Done|recursive' tests/gossip_snapshot.rs` returns nothing; the docs name heights or stages the snapshot shows.

### tests-wire-format-7: No snapshot pins a data stream with index 2 or higher, and deep_trie_divergence has no depth floor
- Where: tests/gossip_snapshot.rs:492-515 (related: tests/gossip_snapshot.rs:4-5, 599-607; tests/snapshots/gossip_snapshot__deep_trie_divergence.snap:289; src/tree/mirror/streaming/remote/codec/signal.rs:17-24,55-73; src/link.rs:161-169)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -h -o -E '(Initiator|Responder) stream [0-9]+ \(height [0-9]+\)' tests/snapshots/*.snap | sort | uniq -c` yields exactly four header kinds: Initiator 0 (h31), Initiator 1 (h30), Responder 0 (h31), Responder 1 (h29); `STREAM_COUNT = 17` at link.rs:169; `deep_trie_divergence` at 500-515 asserts nothing before `insta::assert_snapshot!`)
- Seen by: blind-spots; refutation: confirmed, with a mechanism correction adopted below; history: no rationale found (`DEEP_TRIE_PER_SIDE = 16` was chosen on 2026-06-05 to collide in one leading byte and never revisited; REVIEW.md item 14 catalogued corpus gaps and named only the nonempty Query frame)
- Owner-gated: no

The module doc says the corpus "pins every wire byte", but the deepest fixture reaches `Responder stream 1 (height 29)`; the stream-label encoding and per-height frame forms for the remaining fifteen streams the schedule can open are pinned nowhere in `tests/snapshots`. By the stride at signal.rs:17-24 and 55-73 (stream 0 is height 31 for both speakers; Initiator stream k carries height 32-2k, Responder stream k height 31-2k), a stream index of 2 needs a dispute surviving at a height-29 node, which requires leaves under the same three-byte path prefix on both sides; `deep_trie_divergence`'s one-byte collisions reach exactly index 1. The same fixture carries no liveness floor for its depth claim, unlike `shared_subtree_dispute_pins_a_nonempty_query` (599-607), so a corpus change that flattened it would re-accept with no mechanical objection. White-box worst-case construction asks that the constructed shapes be diffed against the committed roster; the roster stops two levels below the root.

Evidence:

    500	#[test]
    501	fn deep_trie_divergence() {
    502	    let (a, b) = block_on(async {
    503	        let a: Rumors<u64> = seeded();
    504	        let b = bootstrap_fork_async(&a).await;
    505	        {
    506	            a.send_all(0..DEEP_TRIE_PER_SIDE).unwrap();
    507	        }
    508	        {
    509	            b.send_all(DEEP_TRIE_PER_SIDE..2 * DEEP_TRIE_PER_SIDE)
    510	                .unwrap();
    511	        }
    512	        (a, b)
    513	    });
    514	    insta::assert_snapshot!(capture_gossip(a, b));
    515	}

Resolution: Add a depth floor to `deep_trie_divergence` (`stream_frames(&capture, "Responder stream 1 (height 29)")` must be `Some`), so its stated purpose is tamper-evident. For the deeper labels, either stage one fixture with both peers holding leaves under a shared three-byte prefix (a shared pair at `shaped_pair(pool, 3, true)` staged before the fork, plus a divergent third leaf under the same prefix, as `shared_subtree_dispute_pins_a_nonempty_query` does at one byte; a birthday pool on the order of 2^12 sends), floored on a `stream 2` header; or, if the staging cost is judged too high, add a codec-level unit pin of the stream-label encoding for indexes 2..16 and say so in this module doc. Acceptance: `deep_trie_divergence` fails before the snapshot comparison if its depth degrades; either a committed snapshot contains a `stream 2` header or the module doc states where the deeper labels are pinned.
Construction: Stage `send_pool(&a, 0, 4096)`, `shaped_pair(&pool(&a, 0, 4096), 3, true)`, `keep_only`, fork `b`, then land a third leaf under the same three-byte prefix on `a` via a targeted pool search on `leaf_path(v)[..3]`; capture and check for a `stream 2` header.

### tests-wire-format-8: Braces left behind by the send_all conversion
- Where: tests/gossip_snapshot.rs:505-511 (related: none)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`git show 212c6914 -- tests/gossip_snapshot.rs` replaces `a.batch(|batch| { ... })` closures inside the retained braces with single `send_all` statements)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: deliberate but expired (the blocks scoped batch closures that 212c6914 on 2026-09-01 replaced)
- Owner-gated: no

Two bare `{ ... }` blocks each wrap a single statement; they scoped the batch closures the `send_all` commit removed. Scaffolding that outlived the constraint that justified it.

Evidence:

    505	        {
    506	            a.send_all(0..DEEP_TRIE_PER_SIDE).unwrap();
    507	        }
    508	        {
    509	            b.send_all(DEEP_TRIE_PER_SIDE..2 * DEEP_TRIE_PER_SIDE)
    510	                .unwrap();
    511	        }

Resolution: Remove the braces. Acceptance: two plain statements.

### tests-wire-format-9: cbor_evolution attributes evolution rules to crate docs that no longer state them
- Where: tests/cbor_evolution.rs:12-14 (related: tests/cbor_evolution.rs:216-217, src/lib.rs:266-272, README.md:271)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read src/lib.rs:266-272: the compatibility paragraph documents only that reordering is safe and renaming breaks; `grep -rn -i -E 'serde\(default\)|missing field|unknown field' src/ README.md` returns nothing; `git show 3d16765f9 -- src/lib.rs` shows the deleted sentence `peers skip fields they don't know, and a missing field is an error unless the type supplies `#[serde(default)]``)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: deliberate but expired (acb556fc on 2026-08-18 added the two rules to lib.rs; f2b74a97 the same day wrote this test doc citing them; 3d16765f9 on 2026-08-19, author finch, message "Update lib.rs", rewrote the paragraph and dropped both rules)
- Owner-gated: yes: either resolution touches the public compatibility paragraph, and one of them re-adds prose the owner removed by hand

The module doc says "the evolution rules the crate documents" include unknown-field skipping and `#[serde(default)]` semantics, and the last test's doc calls the missing-field boundary "the documented boundary". The crate's compatibility paragraph documents neither. A testdoc that cites documentation must cite documentation that exists; otherwise the test pins a contract the library user was never told. The history matters for the decision: the attribution was true for one day, and it was Finch's own edit that removed the rules, so re-adding them reverses that edit rather than restoring an accident.

Evidence:

    12	//! The evolution rules the crate documents ride the same mechanism and are
    13	//! pinned beside it: unknown fields are skipped, and missing fields error
    14	//! unless the field carries `#[serde(default)]`.

    216	/// A missing field errors without `#[serde(default)]` and fills with it:
    217	/// the documented boundary between tolerated and rejected evolution.

Resolution: Owner's call. (a) Add the two rules back to lib.rs's compatibility paragraph (they answer the first evolution question a user asks, "may I add a field?", and the tests already pin them), keeping the test docs. (b) If the crate does not want to promise serde's field semantics, reword the module doc to "the rules serde's name-keyed decoding gives" and drop "documented" from line 217. Whichever way, see tests-wire-format-14 for the test body. Acceptance: either lib.rs names both rules, or no doc in the file says the crate documents them.

### tests-wire-format-10: A trailing serde import glued to the first item, at 22 sites across the tree
- Where: tests/cbor_evolution.rs:24-29 (related: tests/dispute_wire.rs:54-58; tests/common/wire.rs:20-22; tests/common/gossip_snapshot.rs:53-55; tests/bootstrap_snapshot.rs:31-33; src/message.rs:11-12; src/peer.rs:27-28; src/peer/bootstrap.rs:22-23; src/peer/gossip.rs:44-45; and 13 more)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn -A1 -E '^use serde::(de::DeserializeOwned|Serialize);$' src/ tests/ benches/ examples/`, filtered to cases where the following line is an item, attribute, or doc comment: 22 sites)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed, count corrected to 22; history: no rationale found (residue of c6fe4018's self-described "mechanized sweep" moving serde trait names into `use` lines)
- Owner-gated: no

A second import group sits after a blank line below the main imports and runs straight into the next item's doc comment or attribute with no separating blank line; in cbor_evolution.rs `serde` is imported in two places (24 and 28). The import block is the file's table of contents, and a glued item hides its boundary; rustfmt does not merge groups, so the split persists until someone does it by hand.

Evidence:

    24	use serde::{Deserialize, Serialize};
    25	
    26	use rumors::Peer;
    27	
    28	use serde::de::DeserializeOwned;
    29	/// A struct payload in one field order.

Resolution: Fold the stray line into the existing serde import (`use serde::{Deserialize, Serialize, de::DeserializeOwned};`) and restore the blank line, in one sweep over the grep above. Acceptance: no `use` line is immediately followed by a `///`, `//`, `#[`, or item line without a blank line between.

### tests-wire-format-11: cbor_evolution runs sessions on a real tokio runtime and hand-rolls the cross-typed bootstrap that payload_depth also hand-rolls
- Where: tests/cbor_evolution.rs:65-87 (related: tests/cbor_evolution.rs:154-170, 178-214; tests/payload_depth.rs:140-156, 265-281, 331-343; tests/gossip_snapshot.rs:150-157; tests/wire_legibility.rs:161-172; tests/common/wire.rs:34-59, 244-264; .config/nextest.toml:25-26)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read `bootstrap_fork_configured` at wire.rs:244-264: private, single-typed, consumes the `Peer` via `into_rumors()`, and calls `assert_control_drained`, which none of the inline copies do; read wire.rs:44-48, whose doc reserves the tokio path for tests that "explicitly need Tokio facilities"; `grep -n -E 'tokio::test|tokio::spawn'` in the partition hits only cbor_evolution.rs)
- Seen by: structure-prose, api-economics; refutation: confirmed, severity lowered to low (the tests are correct today; the cost is diagnosis time and harness inconsistency); history: no rationale found (`bootstrap_fork_configured` landed same-typed on 2026-08-13; the later suites inlined their own joins)
- Owner-gated: no

Every other session-driving file in the partition runs under `common::wire::block_on` (`run_to_quiescence`), so a wire stall is reported at its source; payload_depth.rs:349-350 states that payoff for exactly the shape cbor_evolution uses. cbor_evolution instead uses `#[tokio::test]` with `tokio::spawn` and relies on `drop(near)` to unhang the server, so a stall here surfaces only through nextest's 180-second kill. The cross-typed bootstrap (`a.gossip` joined with `Peer::<B>::bootstrap().join`) is spelled inline five times across cbor_evolution and payload_depth because `bootstrap_fork` is same-typed and has no builder hook, and any knob set after a fork pays an async `try_into_peer` reclaim (payload_depth.rs:265-281 does three). The two failure tests also assert less than their comment: "its session then fails too" (166-167) is checked only as `let _ = server.await.expect(...)`, which discards the `Result`.

Evidence:

    73	    let (mut near, mut far) = rumors::link::memory();
    74	    let serve = sender.clone();
    75	    let server = tokio::spawn(async move { serve.gossip(&mut far).await.unwrap() });
    76	    let receiver = Peer::<B>::bootstrap()
    77	        .join(&mut near)
    78	        .await
    79	        .expect("the bootstrap session succeeds")
    80	        .expect("the sender is established")
    81	        .into_rumors();
    82	    server.await.expect("the serving task");

    165	    // Drop the failed side's link so the donor sees the transport close
    166	    // (a peer that errored out of a session hangs up); its session then
    167	    // fails too — but only ever as an error, never a panic.
    168	    drop(near);
    169	    let _ = server.await.expect("the serving task must not panic");

Resolution: Generalize `bootstrap_fork_configured` over two payload types with a builder hook (`bootstrap_fork_with<T, U>(parent: &Rumors<T>, configure: impl FnOnce(Bootstrap<U>) -> Bootstrap<U>) -> Peer<U>`, async core plus sync wrapper, draining the control stream as the existing form does), expose a variant returning the raw `Joined` so the failure tests can assert `Err`, and have `bootstrap_fork` delegate to it. Use it from cbor_evolution.rs (adding `mod common;`) and payload_depth.rs under `block_on` + `tokio::join!`; assert the server side's `Err` in the two failure tests. Acceptance: no `#[tokio::test]` or `tokio::spawn` in cbor_evolution.rs; `grep -rn '::bootstrap()' tests/cbor_evolution.rs tests/payload_depth.rs` returns nothing; the failure tests assert `server` completed with `Err`.

### tests-wire-format-12: Error-path tests accept any error of the outer shape, not the decode failure their docs name
- Where: tests/cbor_evolution.rs:162-163 (related: tests/cbor_evolution.rs:16-18, 204-205; tests/payload_depth.rs:325-327, 361-365; src/error.rs:52-53; src/tree/mirror/streaming/remote/proxy/error.rs:18-81; src/tree/mirror/streaming/remote/codec/error.rs:104-109; src/message.rs:300-315)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read `MirrorError = mirror::Error<MaterializedError<Infallible>, RemoteError<Infallible>>` at error.rs:53 and the `RemoteError` enum at proxy/error.rs:18-81, which alone has some twenty variants; read `decode_exact` at message.rs:300-315 producing `PayloadDecodeError::Io`, which the codec surfaces as `DecodeLeafError::Message(io::Error)`)
- Seen by: blind-spots; refutation: confirmed, with the note that the exact leaf must be read off the ingress path rather than guessed; history: no rationale found (a division-of-labor argument is available but unstated: the codec-level tests and the error atlas pin `DecodeLeafError::Message`)
- Owner-gated: no

cbor_evolution's module doc promises "a decode error at the receiver's wire ingress", and payload_depth's doc promises "the typed decode error", but the assertions are `is_err()` (cbor_evolution 163, 205) and `matches!(b_err, rumors::Error::Mirror(_))` (payload_depth 363), which any of roughly twenty unrelated leaves (an accept error, a framing violation, `UnaskedReply`) would also satisfy. The public `rumors::error` module re-exports the whole taxonomy so callers and tests can name the leaf; a regression that failed these sessions for an unrelated reason passes.

Evidence:

    162	    let joined = Peer::<u64>::bootstrap().join(&mut near).await;
    163	    assert!(joined.is_err(), "a String payload must not decode as u64");

    362	    assert!(
    363	        matches!(b_err, rumors::Error::Mirror(_)),
    364	        "the receiver's exit is the typed decode failure: {b_err:?}"
    365	    );

Resolution: Trace the ingress path once and match the leaf that carries the payload `io::Error` (the `DecodeLeafError::Message` route through `RemoteError`, whichever variant it surfaces in) at all three sites; in cbor_evolution's bootstrap case assert the same on the newcomer's error. Acceptance: each of the three assertions names the decode leaf, and the assertion messages match what they check.
Construction: In `a_sender_exits_typed_when_its_counterparty_aborts_on_decode`, print `b_err` with `{:?}` once to learn the exact variant chain, then replace `Error::Mirror(_)` with that pattern; the same pattern applies at cbor_evolution 163 and 205.

### tests-wire-format-13: Em-dashes in a // comment and an assert message
- Where: tests/cbor_evolution.rs:165-167 (related: tests/future_size.rs:49-55)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep of the eight files for em-dashes on lines not beginning with `///` or `//!` returns exactly cbor_evolution.rs:167 and future_size.rs:53)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

The owner's convention is colons or semicolons in log messages and code comments, with em-dashes reserved for rendered prose. Every other `//` comment and assert message in the partition follows it.

Evidence:

    167	    // fails too — but only ever as an error, never a panic.

    53	         indirection, restore it — otherwise downstream crates will hit \

Resolution: Replace with a colon or semicolon at both sites. Acceptance: the grep above returns nothing.

### tests-wire-format-14: missing_fields_error_absent_a_default tests ciborium in isolation, against the module's own stated method
- Where: tests/cbor_evolution.rs:216-251 (related: tests/cbor_evolution.rs:5-8, 150-170; src/message.rs:300-310)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (the only `ciborium::` calls in the file are at 237, 240, and 243; the module doc at 7-8 says the property is pinned "through the crate's own encode and decode paths, not against a serializer in isolation"; the crate's ingress decode is `ciborium::de::from_reader_with_recursion_limit` via `decode_exact`, a different entry from `from_reader`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale found (the direct calls are present in the file's first commit alongside the doc that forbids them; the only plausible constraint, no session-shaped clean-failure route until 0ac1872c two days later, has lapsed)
- Owner-gated: no

The module doc states the file's method: every rule is pinned through the crate's own paths, never against a serializer alone. This test calls `ciborium::ser::into_writer` and `ciborium::de::from_reader` directly and constructs no `Peer`, so a change to the crate's payload codec (a wrapper deserializer, a positional encoding, a different engine) would leave it green while the contract it claims to pin breaks. Differential and contract suites exercise the public API; here the entry is not even the crate's. The test is also the file's only non-async test, a shape that stands out for the wrong reason.

Evidence:

    236	    let mut narrow = Vec::new();
    237	    ciborium::ser::into_writer(&Narrow { id: 5 }, &mut narrow).unwrap();
    238	
    239	    // Without a default, the absent field is an error, not a guess.
    240	    assert!(ciborium::de::from_reader::<Wide, _>(narrow.as_slice()).is_err());

Resolution: Route the tolerated case through `exchanged::<Narrow, WideDefaulted>` (the filled default must arrive over a session) and the rejected case through the failing-bootstrap shape `undecodable_payload_fails_bootstrap_cleanly` already uses (a `Peer<Wide>` bootstrapping from a `Narrow` donor returns an error, never a panic, and moves nothing). If the owner chooses option (b) of tests-wire-format-9 and wants to keep a serializer-level pin, say so at the test with a comment naming it as the one deliberate exception. Acceptance: the test contains no direct `ciborium::` call and both branches run through `Peer`/`Rumors`, or the exception is stated at the site.

### tests-wire-format-15: Default-dialect tells and a dated rationale in test prose
- Where: tests/dispute_wire.rs:8-12 (related: tests/dispute_wire.rs:22, 59, 111-114, 297-298; tests/cbor_evolution.rs:10; tests/snapshot_liveness.rs:4, 34, 222; tests/future_size.rs:6; tests/payload_depth.rs:8, 141, 325-327; tests/gossip_snapshot.rs:73, 430, 528, 536, 590; tests/wire_legibility.rs:11)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -E '\b(genuine|honest|silently|loudly|defuse|now that|real)\b'` over the eight files; `grep -n 'window.rs' tests/dispute_wire.rs` hits line 112)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed, noting that some `real` uses carry a named contrast and are defensible; history: no rationale found (the 2026-07-24 style round left these; the "now that" is a dated rationale from payload_depth's own creation commit)
- Owner-gated: no

The tells the brief asks to flag recur: menace adverbs without mechanism ("silently letting the documented law go stale", "silently dead", "orphaning them silently"; snapshot_liveness.rs:4-7 is the one site that gives the mechanism), "loudly" as an intensifier (dispute_wire 22, 297; cbor_evolution 10; snapshot_liveness 34), moralized code ("the honest floor", dispute_wire 298; "genuine" at gossip_snapshot 528 and in the assert message at 590, where no contrast is named; at 73 and 430 "genuine" does name a contrast, a party-disjoint fork versus a `Rumors::clone`, and "party-disjoint" is the better word), register transplants ("close the loop", "defuse"), a dated rationale ("now that admission runs the receiving decode", payload_depth 326-327), and a file-path citation of a constant that a module move orphans where the name alone would not (dispute_wire 111-112).

Evidence:

    8	//! self-calibrates its link rate). These pins close the loop with pure
    9	//! byte counts — every write on the control stream and on every data
    10	//! stream of an in-memory session is tallied, no timing anywhere — so a
    11	//! wire-format change that moves the real cost of a disputed message
    12	//! fails here instead of silently letting the documented law go stale.

    111	/// The mechanism is stated at `DISPUTE_OVERHEAD_BYTES` in
    112	/// `src/tree/mirror/streaming/window.rs`. A measured value moving off

    326	/// ingress between equal limits now that admission runs the receiving
    327	/// decode. The receiver's own exit is the typed decode error.

Resolution: Rewrite each site as mechanism ("a wire-format change that moves the per-message cost fails the pin; nothing else in the suite compares the law to bytes"), delete "loudly", "honest", "defuse", and the contrast-free "genuine"s, replace "genuine disjoint fork" with "party-disjoint fork", state the payload_depth fact in the present tense, and cite `DISPUTE_OVERHEAD_BYTES` by name only. Acceptance: the grep above returns only uses where the word carries a named contrast; no file path appears in a doc comment in the partition.

### tests-wire-format-16: dispute_wire restates the design record size by hand and its design-cell doc contradicts window.rs about what derives from DISPUTE_WIRE_BYTES
- Where: tests/dispute_wire.rs:82-100 (related: tests/dispute_wire.rs:29-31, 72-80, 254-284, 332-339, 343-352, 274-277, 306, 333, 360; src/tree/mirror/streaming/window.rs:221-238; src/testing.rs:63-86; tests/tradeoff_probe.rs:136)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read window.rs:227 `DESIGN_RECORD_BYTES: usize = 172`, 232 `An anchor, not an input: nothing derives from it`, 238 `DISPUTE_WIRE_BYTES = DISPUTE_OVERHEAD_BYTES + DESIGN_RECORD_BYTES`; `grep -rn DISPUTE_WIRE_BYTES src/` shows its only reader is `envelope_and_wire_bytes` at testing.rs:73; `rumors::testing` exposes no record-size accessor; tradeoff_probe.rs:136 discards the wire half of the tuple and dispute_wire.rs:273 discards the envelope half)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate but expired for the doc (true at 90a309e0 on 2026-07-23, when `DISPUTE_WIRE_BYTES` did derive the default budget; 4d0b3db2 the same day retired that derivation, rewrote window.rs and testing.rs, and left this test doc); no rationale found for the local constant (4d0b3db2 de-localized the intercept "so the pins hold the shipped number, not a test-local copy" and minted `DESIGN_RECORD_BYTES` in window.rs in the same commit, yet left `DESIGN_PAYLOAD_LEN` local); the `eprintln!`s are deliberate per ce8baf44 ("it also prints the measured figure") with that purpose recorded nowhere in the file
- Owner-gated: no

Four related defects in one file, all resolvable in one pass against 4d0b3db2's message. (a) The design cell's doc says `DISPUTE_WIRE_BYTES` is "the constant that denominates the default budget and both operator equations"; window.rs:232 and testing.rs:67-68 say the opposite, and grep confirms nothing derives from it. Two docs in the tree disagree about one constant's role. (b) `DESIGN_PAYLOAD_LEN = 170` plus `CBOR_BSTR_HEADER_BYTES = 2` restates `DESIGN_RECORD_BYTES = 172` with no assertion tying them, against the file's own rule at 75-77 that the cells pin "the constant the closed form quotes, not a test-local copy"; if the design point moved in src, the cell would fail blaming the intercept ("re-derive the constant", 280-282) when the fixture is what went stale. The docs also restate derived totals as literals ("172 B" at 83, "64 B" at 89 and 318), and the mid cell spells `CBOR_BSTR_HEADER_BYTES + MID_PAYLOAD_LEN` inline (332, 339) where the design cell has a named constant. (c) `fixed_overhead_bytes()` (78-80) is a one-line rename of `dispute_overhead_bytes()`, and `envelope_and_wire_bytes()` bundles two constants whose every consumer wants one half. (d) The claim that the per-message division "truncates only the session-fixed greeting and epilogue" (29-31, 348-352) is loose: `session_wire_bytes` tallies the descent over shared structure too, which is neither per-message nor bounded by the converged control; the cells are exact because the corpus is deterministic. The four `eprintln!`s (274-277, 306, 333, 360) are the only success-path readout of the calibration figures; that purpose is stated only in git.

Evidence:

    82	/// The `Bytes` payload length whose CBOR encoding (a 2-byte byte-string
    83	/// header plus the bytes, 172 B) prices a disputed message at exactly
    84	/// `DISPUTE_WIRE_BYTES` under the current format.
    85	///
    86	/// This is the record size the design-point constant is denominated in.
    87	const DESIGN_PAYLOAD_LEN: usize = 170;

    257	/// The invariant: total session wire bytes over a known mutual
    258	/// divergence of [`DESIGN_PAYLOAD_LEN`]-byte payloads, divided by the
    259	/// messages that crossed, equals the constant exactly — so the
    260	/// constant that denominates the default budget and both operator
    261	/// equations is tied to the wire format by deterministic byte counts.

    78	fn fixed_overhead_bytes() -> usize {
    79	    dispute_overhead_bytes()
    80	}

Resolution: Expose `testing::design_record_bytes()` beside `dispute_overhead_bytes()` and derive the design payload length from it (`design_record_bytes() - CBOR_BSTR_HEADER_BYTES`), or assert `DESIGN_ENCODED_PAYLOAD_BYTES == design_record_bytes()` up front with its own message naming the record-size coupling; reword the design cell's doc to "the anchor the budget docs quote"; split `envelope_and_wire_bytes` into `scope_envelope_bytes()` and `dispute_wire_bytes()`; delete `fixed_overhead_bytes` and call `dispute_overhead_bytes()` directly; add `MID_ENCODED_PAYLOAD_BYTES` and drop the literal totals from the docs; reword 29-31 and 348-352 to "the control bounds the greeting-and-epilogue share; exactness rests on the seeded corpus and in-memory link"; state the `eprintln!`s' purpose in one comment or delete them. Acceptance: changing `DESIGN_RECORD_BYTES` in window.rs fails dispute_wire with a message naming the record size; the two docs agree on what derives from `DISPUTE_WIRE_BYTES`; no literal 170, 172, or 64 appears in the file except as a constant's single definition; no `(_, x) = envelope_and_wire_bytes()` pattern remains in tests.

### tests-wire-format-17: wire_legibility loads payloads one send at a time and folds seed and fork into one Option-flagged helper
- Where: tests/wire_legibility.rs:91-102 (related: tests/wire_legibility.rs:130-134, 157-159, 184-188; src/rumors.rs `send_all`)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read the three loops; `Rumors::send_all` landed in 212c6914, whose sweep targeted batch-closure helpers and did not touch this file)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: deliberate but expired (the loops were the only shape available when the file was written on 2026-08-19)
- Owner-gated: no

`loaded(None, ..)` seeds and `loaded(Some(&a), ..)` forks, and each test then loops `send` over a third corpus; with `send_all` in the API, each load is one call and the mode flag is unnecessary. Separately, `arbitrary_bootstrap_sessions_are_cbor_sequences` generates a three-tuple corpus and discards two of the three per case.

Evidence:

    93	fn loaded(parent: Option<&Rumors<Vec<u8>>>, payloads: &[Vec<u8>]) -> Rumors<Vec<u8>> {
    94	    let peer = match parent {
    95	        Some(parent) => block_on(bootstrap_fork_async(parent)),
    96	        None => Peer::seed().sync_window_floor().into_rumors(),
    97	    };
    98	    for payload in payloads {
    99	        peer.send(payload.clone()).unwrap();
    100	    }
    101	    peer
    102	}

Resolution: Use `send_all(payloads.iter().cloned())` at all three sites, inline the two construction shapes (seed versus `bootstrap_fork`), and give the bootstrap property its own single-corpus strategy. Acceptance: no per-payload `send` loop in the file; no `(shared, _, _)` destructure.

### tests-wire-format-18: The legibility walker has no committed negative case, and the corpora never open a stream index of 2 or higher
- Where: tests/wire_legibility.rs:109-116 (related: tests/wire_legibility.rs:37-79, 118-122; src/tree/mirror/streaming/remote/codec/signal.rs:17-24, 55-73)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read the file: its only tests are the three proptests at 127, 157, 181, and nothing feeds `walk_sequence`/`walk_value` a rejected input; read the stride at signal.rs, from which a stream index of 2 requires a dispute at a height-29 node, i.e. leaves under the same three-byte path prefix on both sides)
- Seen by: blind-spots; refutation: reframed (the mechanism for the rarity was corrected from "two-byte shared prefix" to "three-byte shared prefix on both sides", which makes the region rarer still); history: no rationale found (the legible-wire design note and REVIEW.md specify only the positive property)
- Owner-gated: no

The walker is the whole oracle, and nothing committed shows it rejecting a non-CBOR byte, residue behind a tag-24 item, or a tag 63 wrapping a non-bstr; a walker bug that returned `Ok` on everything would pass all 72 cases vacuously. Every criterion needs a committed demonstration that a known-bad artifact fails it. And with at most eleven payloads per bucket over uniform paths, a shared three-byte prefix present on both sides essentially never arises, so the property is checked on streams 0 and 1 only; together with tests-wire-format-7, the deeper stream labels have neither a byte pin nor a property behind them.

Evidence:

    109	fn corpora() -> impl Strategy<Value = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    110	    let payload = vec(any::<u8>(), 0..48);
    111	    (
    112	        vec(payload.clone(), 0..12),
    113	        vec(payload.clone(), 0..12),
    114	        vec(payload, 0..12),
    115	    )
    116	}

Resolution: Add three unit checks on the walker (a `[0xff]` sequence; `24(<< 0x00 0x00 >>)` with residue; `63(0)`) asserting `Err`. Add one deterministic case staged through `common::shape` (a shared pair at three bytes on both sides plus a divergent leaf) so at least one capture per run has a stream index of 2 or more, asserting `streams.len() >= 3` on that capture. Acceptance: negative walker tests committed and red on the bad inputs; one legibility run includes a capture with three or more data streams.
Construction: `assert!(walk_sequence(&[0xff], "bad").is_err())`; build tag 24 around the two-byte string `[0x00, 0x00]` via `ciborium::ser::into_writer(&Value::Tag(24, Box::new(Value::Bytes(vec![0, 0]))), ..)` and assert `Err` mentioning "residue"; build `Value::Tag(63, Box::new(Value::Integer(0.into())))` and assert `Err` mentioning "byte string".

### tests-wire-format-19: The snapshot sweep judges only tests/snapshots, though insta writes beside the asserting file
- Where: tests/snapshot_liveness.rs:105-109 (related: tests/snapshot_liveness.rs:132-146, 319-351)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read `scan_tests_side`, which reads exactly `root/tests/snapshots`, against `scan_src_side`, which recurses for any `snapshots` directory; `find . -type d -name snapshots` shows no `tests/common/snapshots` today)
- Seen by: api-economics; refutation: confirmed; history: no rationale found (REVIEW.md item 13's resolution said "Walk `tests/snapshots/*.snap`" and the implementation transcribed it; the gap was never considered)
- Owner-gated: no

insta places a snapshot in a `snapshots` directory beside the asserting source file, so an `assert_snapshot!` added to `tests/common/*.rs` (or any nested test module) would land at `tests/common/snapshots/`, outside both scans; an orphan there would never be convicted. The module doc claims every committed snapshot resolves to a live generator, and the src side already applies the recursive discipline; the tests side does not. No such directory exists today, so this is a blind spot rather than a live orphan.

Evidence:

    105	/// Judge every file under `<root>/tests/snapshots`: the stem's prefix
    106	/// before the first `__` names the suite binary, the rest the
    107	/// snapshot.
    108	fn scan_tests_side(root: &Path, out: &mut Vec<Snap>) {
    109	    let dir = root.join("tests").join("snapshots");

Resolution: Recurse under `tests/` the way `scan_src_side` does, convicting any `snapshots` directory other than `tests/snapshots` outright (the module doc's "extending the rule is then a reviewed decision, not a silent skip" already sets the policy), and add a fixture line `tests/common/snapshots/x.snap` to `suite_snapshots_resolve_to_their_binary` asserting the conviction. Acceptance: a fixture tree containing `tests/common/snapshots/anything.snap` is convicted by the sweep.
Construction: In `suite_snapshots_resolve_to_their_binary`, add `.file("tests/common/snapshots/stray.snap", "")` and assert the sweep reports five verdicts with that path convicted; today the sweep reports four and never sees the file.

### tests-wire-format-20: The snapshot_liveness fixture tests demonstrate three of nine conviction branches
- Where: tests/snapshot_liveness.rs:187-213 (related: tests/snapshot_liveness.rs:66-81, 86-102, 119-122, 162-169, 319-351, 357-390)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (counted the `Err(` sites: `contained` two, `snap_stem` two, `scan_tests_side`'s no-`__` branch one, `judge_snapshots_dir`'s directory branch one, `judge_src_stem` three; the two fixture tests produce only "does not exist", "contains neither", and "unaccepted snapshot")
- Seen by: structure-prose; refutation: confirmed, count corrected from four to three of nine; history: no rationale found (all nine branches existed at the sweep's creation; the fixture scope was transcribed from REVIEW.md item 13's acceptance, which named one stray file)
- Owner-gated: no

`judge_src_stem`'s three convictions (missing `rumors__` prefix, too-short module path, directory segments not spelling the location), `snap_stem`'s stray-file conviction, `scan_tests_side`'s no-`__` conviction, and the nested-directory conviction are never exercised; a typo in any of them (a wrong `split_at`, an inverted comparison) would pass. Every criterion needs a committed demonstration that a known-bad artifact fails it. The two fixture tests also differ in style: the first indexes `verdicts[0..3]` by sort position, the second builds a by-path map.

Evidence:

    188	    let Some(rest) = stem.strip_prefix("rumors__") else {
    189	        return Err("the stem does not open with this crate's `rumors__` prefix".to_owned());
    190	    };
    191	    let segments: Vec<&str> = rest.split("__").collect();
    192	    if segments.len() < components.len() + 2 {

Resolution: Extend the module fixture with `other__foo__tests__x.snap`, `rumors__foo__tests.snap`, `rumors__wrong__tests__x.snap`, a `notes.txt`, a nested directory, and a tests-side `nosep.snap`, asserting each conviction's message; use the by-path map in both fixture tests. Acceptance: each `Err(...)` literal in the sweep has a fixture line that produces it.
Construction: Add `.file("src/foo/snapshots/other__foo__tests__x.snap", "")` to `module_snapshots_resolve_through_the_module_path` and assert its verdict contains "rumors__ prefix"; repeat for the other five.

### tests-wire-format-21: FixtureTree hand-rolls a temp directory and is duplicated between the two provenance sweeps
- Where: tests/snapshot_liveness.rs:258-309 (related: tests/seed_liveness.rs:244-290; Cargo.toml:144-172; Cargo.lock)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -c 'name = "tempfile"' Cargo.lock` is 1, a transitive dependency; Cargo.toml's dev-dependencies do not list it; read both `FixtureTree` definitions, identical save `file`'s signature and the sweep body; neither sweep declares `mod common;`)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (seed_liveness.rs's copy is the original from 2026-08-13; snapshot_liveness.rs copied it "modeled on the seed-liveness sweep"; `tempfile` has never been a rumors dependency and no note weighs it)
- Owner-gated: yes: adds a dev-dependency

`FixtureTree::new` builds a pid-tagged path under `std::env::temp_dir()`, pre-removes a stale one, and removes on drop; `tempfile::tempdir()` does exactly this with a unique name, and the crate is already in the lockfile, so adding it as a dev-dependency introduces no new crate to the supply chain. The stale-leftover dance exists only because the name is not unique. The struct is also defined twice, in two sweeps that deliberately do not compile the 4.4k-line harness, so the ordinary "move it to tests/common" fix is the wrong trade.

Evidence:

    264	    fn new(tag: &str) -> Self {
    265	        let dir =
    266	            std::env::temp_dir().join(format!("snapshot-liveness-{tag}-{}", std::process::id()));
    267	        // A stale run's leftovers must not leak into this one.
    268	        let _ = std::fs::remove_dir_all(&dir);

Resolution: Add `tempfile` to dev-dependencies and hold a `tempfile::TempDir` in `FixtureTree`, dropping the constructor's cleanup and the `Drop` impl; share the remaining `file`/`sweep` shell between the two sweeps through a small path-included module (`#[path = "support/fixture_tree.rs"] mod fixture_tree;`) so neither binary compiles `tests/common`. Acceptance: one definition of `FixtureTree` holding a `TempDir`; no `remove_dir_all` in either sweep.

### tests-wire-format-22: The zero and u64::MAX corners of PayloadDepthLimit have no committed pin
- Where: tests/payload_depth.rs:132-134 (related: src/message.rs:74-94; src/message/tests.rs:191-200)
- Class / severity / confidence: verification-gap / nit / medium
- Provenance: verified (`grep -rn -E 'PayloadDepthLimit::new\((0|u64::MAX)\)' src tests` returns nothing; `PayloadDepthLimit::new` accepts any `u64` and `recursion_limit` saturates at message.rs:91-93)
- Seen by: blind-spots; refutation: reframed (the raised boundary itself is pinned at unit level in src/message/tests.rs:191-200; only the zero and saturation corners remain); history: no rationale found
- Owner-gated: no

The knob's contract is "exactly `steps` decode recursion steps" over any `u64`, yet the lower corner is never touched (under ciborium's accounting a limit of 0 admits scalar payloads and rejects every container) and `new(u64::MAX)` never exercises the documented `usize` saturation. Boundaries untouched: zero and capacity. These are cheap point pins.

Evidence:

    132	fn equal_raised_limits_gossip_deep_content_clean() {
    133	    let raised = PayloadDepthLimit::new(DEFAULT_PAYLOAD_DEPTH_LIMIT.get() + 64);
    134	    let deep = nested(DEFAULT_PAYLOAD_DEPTH_LIMIT.get() + 32);

Resolution: Add a point test that limit 0 admits `7u64` and rejects `Arr(vec![])`, and that `new(u64::MAX)` admits a deep payload end to end. Acceptance: both corners have committed pins.
Construction: `Peer::<u64>::seed().payload_depth_limit(PayloadDepthLimit::new(0)).into_rumors().send(7)` must be `Ok`; the same with `Rumors<Arr>` and `nested(1)` must be `Err(EncodeError::Depth { .. })`.

### tests-wire-format-23: A third observer recorder where the harness already holds two
- Where: tests/payload_depth.rs:202-244 (related: tests/common/gossip_snapshot.rs:258-316; tests/observe.rs:39-105)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read the three: `SessionShape`/`ShapeObserver`/`ShapeSession` retains an elected flag and a data-stream count; `HookRecorder` retains the elected role and every sent stream by `StreamId`; `Recording` retains sessions, roles, every stream with its `StreamInfo`, and items)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (both existing recorders predate this one)
- Owner-gated: no

The payload_depth pin needs only "did an election happen" and "how many data streams opened", which both existing recorders already hold. Three `impl Observer` recorders in `tests/` is duplicated harness; `Recording` in observe.rs is the superset and belongs in `tests/common`, queried for the shape each suite asserts.

Evidence:

    209	#[derive(Default)]
    210	struct SessionShape {
    211	    elected: std::sync::Mutex<bool>,
    212	    streams: std::sync::Mutex<usize>,
    213	}
    214	
    215	struct ShapeObserver(std::sync::Arc<SessionShape>);

Resolution: Move `Recording` (or `HookRecorder`) into `tests/common`, give it `elected()` and `data_streams()` accessors, and delete `SessionShape`/`ShapeObserver`/`ShapeSession`. Acceptance: one `impl Observer` recorder type exists under `tests/`, in `tests/common`.

### tests-wire-format-24: Long qualified paths where imports exist or belong, Mutex for a counter, and misnamed link ends
- Where: tests/payload_depth.rs:209-243 (related: tests/payload_depth.rs:21, 142, 258, 265-283, 331, 351; tests/dispute_wire.rs:170-178; tests/wire_legibility.rs:200; tests/future_size.rs:42, 62, 80; tests/api_send_bounds.rs:32-38, 44-48, 54-57, 64-67, 77-78, 90-91)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (payload_depth.rs spells `crate::common::wire::block_on` at 142, 265, 272, 277, 283, 331, 351 while importing from the same module at 21; `std::sync::` and `rumors::observe::` qualified throughout 209-243 and 270-279; a fn-local `use rumors::Error;` at 258; dispute_wire.rs:173-178 qualifies `tokio::io::DuplexStream`, `rumors::link::MemoryConnector`, `rumors::link::MemoryAcceptor`; future_size.rs and api_send_bounds.rs bind the discarded far link end as `peer`/`_peer`; api_send_bounds.rs has a mid-sentence hard break at 34-35 and `drop(fut)` after each by-reference `require_send(&fut)`)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found
- Owner-gated: no

Imports over long qualified paths except where the qualification informs; none of these does (the file already imports two names from `crate::common::wire`). `SessionShape` holds `Mutex<bool>` and `Mutex<usize>` where `AtomicBool`/`AtomicUsize` fit, as dispute_wire.rs:120 uses in the same partition. A `MemoryLink` end bound as `peer` is not a peer. The explicit `drop(fut)` calls restate what `require_send`'s doc already explains.

Evidence:

    219	impl rumors::observe::Observer for ShapeObserver {
    220	    fn session(
    221	        &self,
    222	        _session: &rumors::observe::SessionInfo,
    223	    ) -> Option<Box<dyn rumors::observe::SessionObserver>> {

    42	    let (mut link, peer) = rumors::link::memory();
    43	    drop(peer);

Resolution: Import `block_on`, `Arc`, `AtomicBool`, `AtomicUsize`, `Error`, and the `observe` items at the top of payload_depth.rs; switch the two counters to atomics; import the link types in dispute_wire.rs; rename the far link end `_far`; reflow the api_send_bounds comment and drop the `drop(fut)` lines. Acceptance: `grep -c 'crate::common::wire::\|std::sync::\|rumors::observe::' tests/payload_depth.rs` is 0 outside the `use` block; no binding named `peer` for a `MemoryLink`.

### tests-wire-format-25: future_size.rs documents a type chain and an erasure boundary that do not exist
- Where: tests/future_size.rs:3-9 (related: tests/future_size.rs:37-39, 51-54; src/tree/traverse.rs:9-12; src/tree/mirror/streaming.rs:110-134, 142-153; src/peer/gossip.rs:1118-1128, 942-946)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn -E '\bLevels\b|\bBelow\b' src/` hits only the comment at traverse.rs:10; `grep -rn -E 'fn mirror\b' src/` hits only streaming.rs:143, which is `#[cfg(test)] pub(crate) async fn mirror` with no `Box::pin` in its body; the production boxing is `Handshaken::reconcile` at streaming.rs:110-116 and the `#[inline(never)]` `Reconciliation::reconcile` at gossip.rs:1126-1128, whose doc at 1118-1125 explains the coercion; no `Box::pin` exists under src/tree/traverse/)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate but expired (every named item was real at bdf10d4c on 2026-05-27: `mirror()` boxed its body under a recursion-limit comment, `Levels` was a zipper trait, `Below<H, A>` a struct in `typed::levels`; the premise moved at the streaming swap, again when 442174f0 sealed the funnel at `Reconciliation::reconcile`, and `typed::levels` was deleted at 368da2a5; the doc was reflowed twice without re-denomination)
- Owner-gated: no

The module doc attributes the deep chain to `Levels<Below<..., Below<..., ...>>>` and says erasure happens "inside the protocol and `tree::traverse::act`"; the first test's doc says the erasure is "`mirror()`'s internal `Pin<Box<dyn Future>>`". None of that is current. A maintainer who trips this guard is sent by its doc and assert message to a function that neither boxes nor ships. The same expiry orphans the `Levels` comment at traverse.rs:9-12.

Evidence:

    3	//! The mirror protocol's `Levels<Below<…, Below<…, …>>>` chain is ~30 deep,
    4	//! enough that any layout query that traverses it inline blows past the
    5	//! default `recursion_limit = 128` and forces downstream crates to bump
    6	//! their own limit. We defuse that by type-erasing inside the protocol and
    7	//! `tree::traverse::act`, which leaves the public futures (`Rumors::gossip`,
    8	//! `Peer::retire`, `Bootstrap::join`) holding nothing more than a
    9	//! `Pin<Box<dyn Future>>` plus a few locals.

    37	/// The erasure is `mirror()`'s internal `Pin<Box<dyn
    38	/// Future>>`, so the protocol's `Levels` chain doesn't appear in the
    39	/// caller's layout query.

Resolution: Restate the mechanism against today's code: the deep type is the typed phase schedule under `streaming::protocol`; the erasure boundaries are `Reconciliation::reconcile`'s boxed, `inline(never)` future (with `Handshaken::reconcile` below it) and `gossip_when`'s boxed unfold; point the assert messages at `Reconciliation::reconcile` by name; fix traverse.rs:9-12 in the same pass. Acceptance: `grep -n -E 'Levels|Below<|mirror\(\)' tests/future_size.rs src/tree/traverse.rs` returns nothing; the module doc names `Reconciliation::reconcile`.

### tests-wire-format-26: future_size.rs never runs: its crate-level cfg excludes it from every committed check
- Where: tests/future_size.rs:17-20 (related: tests/future_size.rs:26-32; justfile:107-114, 597, 644, 1000; Cargo.toml:174-192; .config/nextest.toml:25-26; .cargo/mutants.toml:26-37; .github/workflows/ci.yml:107-108; src/peer/gossip.rs:1126-1128, 942-946)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (read justfile:108-114: `test` and `test-all` are `cargo nextest run --workspace [--all-features]` with no profile; the only `--cargo-profile release` nextest legs at justfile:597 and 644 sit inside the `crates/before` fuzzfit and wasm32-pins harnesses; Cargo.toml:174-192 defines only `[profile.dev]` and `[profile.bench]` and its comment at 180 says `debug-assertions` stays on; ci.yml runs `just ci`, whose test leg is `test-all`; .cargo/mutants.toml:26-37 states the campaign runs under the dev/test profile; `grep -n -E 'future_size' justfile .github tools .cargo .config` returns nothing; `git show bf91938a` shows the 2026-08-12 budget bump 1024 to 2048, author finch, touching only this file)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed, adding the mutants observer; history: no rationale found for the missing leg, and the cfg's original premise has expired (bdf10d4c's doc said debug layouts differ because the traverse trait dispatch "is itself boxed under `cfg(debug_assertions)` for stack safety"; that boxing left with the streaming swap, the sentence was generalized to "debug layouts carry additional state", and today the only `cfg(debug_assertions)` in src is a monotonicity `debug_assert` at window.rs:360, unrelated to layout)
- Owner-gated: no

The whole binary is gated `#![cfg(not(debug_assertions))]`, and every rumors test run the repository configures (the gate, CI, coverage, the mutants campaign) is a dev-profile run with debug assertions on. The three budget tests therefore compile to an empty binary and have never executed under any committed check since the file was created; the guard against the downstream `recursion_limit` regression its module doc promises to catch "before downstream crates discover" it is decoration, and its 2048-byte budget has no committed evidence behind it (the doc at 28 still says "measured sizes (a few hundred bytes)" after a bump past 1024). A meter nothing runs is a ceiling with no liveness: every status board an acceptance concept exists for must be wired into the required checks. The by-hand release run the budget bump implies is exactly the convention held in memory the doctrine forbids.

Evidence:

    17	//! The budget is enforced only in release builds: debug layouts carry
    18	//! additional state, and they are not what users ship.
    19	
    20	#![cfg(not(debug_assertions))]

    108	test *args:
    109	    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace {{ args }}
    113	test-all *args:
    114	    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace --all-features {{ args }}

    180	# `debug-assertions` stays on, as the dev profile grants: the envelope

Resolution: Lift the cfg and pin a budget that holds in both profiles: the claim is that the public futures hold only a `Pin<Box<dyn Future>>` plus locals, which is a layout fact in either profile; measure the three sizes once in dev and release (a handed number is a hypothesis), set `PUBLIC_FUTURE_BUDGET` with headroom, and state the measured band in its doc, replacing "a few hundred bytes". This keeps the gate's build set unchanged and keeps the guard inside the mutants observer, which a release-only leg would not. Add `gossip_when`'s stream (boxed at gossip.rs:946, unmeasured) to the measured surface. Add an adequacy demonstration: the commit landing the fix records that removing the `Box::pin` in `Reconciliation::reconcile` trips the budget. If the owner prefers a release leg instead, add `cargo nextest run -p rumors --test future_size --cargo-profile release` to `gate-streams` and `just ci` with a recipe comment stating why this one binary runs at release. Acceptance: `cargo nextest list -p rumors --test future_size` under the gate's invocation lists the three (or four) tests; `just gate` runs and passes them; a deliberate removal of the `Box::pin` at gossip.rs:1128 fails them.
Construction: `cargo nextest list -p rumors --test future_size` today lists no tests; `cargo nextest list -p rumors --test future_size --cargo-profile release` lists three. Remove the `Box::pin` at src/peer/gossip.rs:1128 and confirm only the release invocation notices.

### tests-wire-format-27: api_send_bounds says "every async public method" but omits gossip_when, Peer::bookmark, BookmarkedBootstrap::join, and two of the three observer streams
- Where: tests/api_send_bounds.rs:1-3 (related: tests/api_send_bounds.rs:19-22, 27-38; src/rumors.rs:437, 489, 617-632; src/peer.rs:272, 290; src/peer/bootstrap.rs:247, 337; src/rumors/causal.rs:151; src/rumors/changes.rs:127; src/rumors/unordered.rs:191; Cargo.toml:72, 128)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n -E 'pub (async )?fn (try_into_peer|gossip|gossip_when|bookmark|retire|join)\b'` over src/rumors.rs, src/peer.rs, src/peer/bootstrap.rs, plus `grep -rn -E 'impl.*Stream for' src/rumors/*.rs`, enumerate the async surface; `gossip_when` returns `impl Stream<Item = ...> + Unpin + 'a` with `S: Stream + 'a` and no `Send` in its signature; the suite checks `gossip`, `Bootstrap::join`, `retire`, `try_into_peer`, and `UnorderedMessages::next` only; `static_assertions` is a regular dependency)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate but expired ("every" was exact on 2026-05-27 for `Local::{message,process,gossip}`; `gossip_when` landed 2026-06-12 and `Peer::bookmark` 2026-06-17 with no change to this file; later touches generalized the wording to "handle types" without re-auditing)
- Owner-gated: no

The public async surface is `Rumors::{try_into_peer, gossip, gossip_when}`, `Peer::{bookmark, retire}`, `Bootstrap::join`, `BookmarkedBootstrap::join`, and the `Stream` faces of `UnorderedMessages`, `CausalMessages`, and `Changes`. Five of those are unchecked, so "every" over-claims and a `!Send` regression in any of them (an `Rc` captured in the bookmark path, a non-`Send` cue stream bound) compiles. `gossip_when` is the method most likely to be `tokio::spawn`ed, exactly the motivating use the doc names. A guard whose doc says "every" invites a maintainer to assume a new async method is covered when it is not. The hand-rolled `require_send_sync`/`require_send_type` also duplicate `static_assertions::assert_impl_all!`, already a dependency.

Evidence:

    1	//! Static assertions that every async public method on the `rumors`
    2	//! handle types returns a `Send` future, and that the handle types
    3	//! themselves are `Send + Sync`.

Resolution: Add `require_send` checks for `Peer::bookmark` (with `NoBookmark` or a trivial `Bookmark`), `BookmarkedBootstrap::join`, `gossip_when`'s stream (with `futures::stream::pending()` as the cue), and `CausalMessages::next` / `Changes::next`; replace lines 19-22 and 27-38 with item-level `static_assertions::assert_impl_all!(Peer<String>, Rumors<String>, Snapshot<String>: Send, Sync); assert_impl_all!(UnorderedMessages<String>: Send);`. Or narrow the module doc to the enumerated set. Acceptance: every `pub async fn` and every `impl Stream`-returning method on `Peer`, `Rumors`, `Bootstrap`, `BookmarkedBootstrap`, and every observer's `Stream` impl has a compile-time `Send` check, and the doc's "every" is true.
Construction: Add `fn gossip_when_stream_is_send() { let s = rumors.gossip_when(futures::stream::pending::<Gossip>(), &mut link); require_send(&s); }`; today this is the only way to learn whether it compiles.

### tests-wire-format-28: Unused Protocol import in the shared wire harness, masked by a blanket unused_imports allow
- Where: tests/common/wire.rs:14 (related: tests/common/mod.rs:31-33)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -n Protocol tests/common/wire.rs` returns only line 14; tests/common/mod.rs:33 is `#![allow(dead_code, unused_imports)]` with the rationale "Not every binary uses every module" at 31-32)
- Seen by: api-economics; refutation: confirmed; history: deliberate but expired (368da2a5 removed every use of `Protocol` in wire.rs and left the import; the allow dates to 2026-05-27)
- Owner-gated: no

`Protocol` is imported and never used; its last uses left with the V1 retirement. It survives because of `#![allow(dead_code, unused_imports)]`, whose stated rationale (per-binary variance) justifies `dead_code` only: an import referenced by any item, live or dead, is a used import, so `unused_imports` in the allow does no work for that rationale and only hides rot like this. This file sits in `tests/common`, the harness every partition file drives; if another partition owns `tests/common`, merge there.

Evidence:

    14	use rumors::{Peer, Protocol, Rumors, testing::run_to_quiescence};

    31	//! Not every binary uses every module; suppress unused-code warnings here
    32	//! rather than peppering allows across modules.
    33	#![allow(dead_code, unused_imports)]

Resolution: Remove `Protocol` from the import; drop `unused_imports` from the allow at mod.rs:33 and fix whatever else `just clippy` then reports. Acceptance: `just clippy` is clean with `#![allow(dead_code)]` alone on tests/common/mod.rs.

## Positives

- gossip_snapshot.rs's shape-staged fixtures (`colliding_pair` 107-122, `bulk_initiator_ships_opening_supplies` 275-302, `early_supplies_honor_redactions` 361-379, `shared_subtree_dispute_pins_a_nonempty_query` 560-596) assert the tree shape they landed before the byte comparison, and two of them add in-test liveness floors on the frame sequence (305-319, 382-388) or the nonempty listing (599-607): the cheapest passing artifact (a re-accepted degraded capture) is excluded by construction, not by convention. `early_supplies_honor_redactions` also checks post-state on both replicas (389-404); it is the pattern the rest of the roster should follow (tests-wire-format-5).
- tests/common/shape.rs stages hash-derived shapes deterministically (pool, search, redact) with no hand-picked versions and no mocked hash, and states plainly why its searches cannot flake; its panics say "enlarge the pool".
- tests/common/gossip_snapshot.rs renders from the public observation hook while holding every hook item to the transport capture with `assert_items_account_for` and asserting the control drain in both directions (459-471): the instrument enters through the public door and the byte-pin claim keeps a totality oracle.
- dispute_wire.rs's negative control (343-371) is a textbook liveness floor: it proves the counter alive on a session that disputes nothing and bounds the fixed overhead below one byte per message, which is what licenses exact integer pins; the three-cell affine design with a separately named residual makes framing drift visible at every record size.
- wire_legibility.rs states its claim as a family (a proptest over sessions), keeps the walker to `ciborium::Value` and standard tag semantics only, and uses `#[allow(clippy::type_complexity)]` on `corpora()` rather than coining a type name, as the owner's doctrine asks. No proptest seed belongs to it: it has never failed.
- snapshot_liveness.rs guards its own vacuity (230-237), refuses to skip anything under a snapshots directory (a pending `.snap.new` convicts), and documents its substring-match residual plainly (37-42).
- payload_depth.rs's `mismatched_limits_abort_both_sides_at_the_handshake` pins the abort's placement (no election, no data stream, on a converged pair) rather than only the error variant, and `a_sender_exits_typed_when_its_counterparty_aborts_on_decode` uses the closed-world poller so a hung sender is a failure, not a parked process; cbor_evolution.rs's failure-path tests assert "error, never a panic, nothing moved" on both sides.
- The closed-world `block_on` (`run_to_quiescence`) is used consistently in six of the seven session-driving files, so protocol stalls fail at their source instead of at nextest's 180-second timeout.

## Open questions for Finch

- future_size.rs (tests-wire-format-26): lift the cfg and pin a budget measured in both profiles, or add a release-profile leg for this one binary? Recommendation: lift the cfg. The cfg's original premise (debug-only boxing in the traverse trait dispatch) left the tree in June; a dev-profile budget keeps the gate's build set unchanged and keeps the guard inside the mutants observer, which runs dev/test. Either way the number needs a fresh measurement before it lands.
- cbor_evolution.rs (tests-wire-format-9): re-add the unknown-field and `#[serde(default)]` rules to lib.rs's compatibility paragraph, or drop the test doc's attribution? Recommendation: re-add them, because "may I add a field to my type?" is the first evolution question a user asks and the tests already pin the answer; but this reverses your own edit in 3d16765f9 ("Update lib.rs"), so if that removal was a ruling not to promise serde's field semantics, choose the other branch and the test doc follows.
- Liveness floors in gossip_snapshot.rs parse the renderer's text (161-229), as REVIEW.md item 14 specified. The parsers have been re-shaped three times for renderer changes, and AGENTS.md sanctions renderer-vocabulary re-accepts independent of the wire. Would you want a `rumors::testing` accessor exposing decoded frame signals so the floors read structure instead of text? Recommendation: leave as specified unless a renderer-vocabulary re-accept actually lands; it is a design proposal, not a defect.
- `Peer::seed_rng` is `#[doc(hidden)]` (src/peer.rs:212-213), yet it is the precondition for every wire pin in this partition, and a downstream crate writing its own insta pins over rumors sessions has the same need with no documented entry. Against documenting it: a caller-supplied RNG lets two processes mint the same `Network` each holding `Party::seed()`, which is exactly the two-universes hazard AGENTS.md forbids. Recommendation: keep it hidden and, if the need is real, offer a documented constructor that takes a `Network` value rather than an RNG, with the safety rule restated there.
- tempfile as a dev-dependency (tests-wire-format-21): it is already in the lockfile transitively. Recommendation: yes.
- A stream-index-2 snapshot fixture (tests-wire-format-7) costs a birthday pool on the order of 2^12 sends per run. Recommendation: add the cheap depth floor to `deep_trie_divergence` now; for the deeper labels, a codec-level unit pin of the stream-label encoding for indexes 2..16 is the lower-cost route if one does not already exist, with the module doc saying where the pin lives.

## Dropped

- [22] 'V2' qualifiers on the only protocol: refuted. `Protocol::V2 = 2` is public API, the literal `2` crosses the wire in the preamble (snapshot line 10), and the crate docs make per-version pinning first-class; "a V2 session" is a present-tense fact, not V1 residue.
- [12] Liveness floors re-parse rendered text: deliberate and specified in REVIEW.md item 14 (2026-08-20), implemented verbatim; the finding brings coupling cost but no new evidence, so it moves to open questions as a design proposal.
- [36] snapshot_liveness does not name the cfg'd-out generator as a residual: below the bar; the doc's closing clause ("pairs names with sources, not assertions with snapshots") already generalizes the residual, and no such generator has snapshots today.
- [45] Every byte-level suite depends on the doc(hidden) `Peer::seed_rng`: an owner decision about public API in src/peer.rs, not a defect in this partition; moved to open questions with the two-universes hazard as the case against documenting.
- [34] Depth-limit boundaries beyond the default unexercised: partly refuted (the raised boundary is pinned at src/message/tests.rs:191-200); the surviving zero and u64::MAX corners are tests-wire-format-22.
- [16] eprintln! diagnostics and rename-only wrapper: reframed by the refutation pass (the prints are the only success-path readout of a calibration suite; the wrapper is a pass-through); both folded into tests-wire-format-16.
- [0], [24], [38]: duplicates of tests-wire-format-26.
- [3], [25], [39]: duplicates of tests-wire-format-25.
- [2], [40]: duplicates of tests-wire-format-3.
- [1], [26], [41]: duplicates of tests-wire-format-6 (the 25-byte and 'u64 throughout' sub-claims of [26] and [41] live in tests-wire-format-3 and tests-wire-format-1; [26]'s 'now that' lives in tests-wire-format-15).
- [7]: folded into tests-wire-format-1.
- [4], [42]: duplicates of tests-wire-format-9; [42]'s serializer half and [5], [31] are tests-wire-format-14.
- [6], [33], [49]: duplicates of tests-wire-format-27.
- [8], [43], [47]: merged into tests-wire-format-11.
- [9], [44], [46], [10]: merged into tests-wire-format-2.
- [55]: split among tests-wire-format-8 (braces), tests-wire-format-2 (prefix collector), and tests-wire-format-17 (discarded corpora); [17]: duplicate of tests-wire-format-8; [18]: duplicate of tests-wire-format-17.
- [15], [32], [48]: merged into tests-wire-format-16.
- [20], [35], [53]: duplicates of tests-wire-format-10 (count corrected to 22 sites).
- [21], [52]: merged into tests-wire-format-24.
- [23], [54]: merged into tests-wire-format-15.
- [11]: tests-wire-format-23. [13]: tests-wire-format-20 (count corrected to three of nine). [14], [56]: merged into tests-wire-format-21. [19]: tests-wire-format-13. [27]: tests-wire-format-5. [28]: tests-wire-format-7 (mechanism corrected to a three-byte shared prefix on both sides). [29]: tests-wire-format-18. [30]: tests-wire-format-12. [37]: tests-wire-format-4. [50]: tests-wire-format-19. [51]: tests-wire-format-28.
