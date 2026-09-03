<!-- CAVEAT LECTOR: review packet for lane p1-renderer, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p1-renderer

## Goal

The wire snapshots under `tests/snapshots/` pin the protocol's bytes
through a rendering of each captured item, so the rendering must be
injective on wire bytes. The review found the hand-written renderer
elided map keys to an ellipsis, so two different byte strings could
render identically, and asked for the elision fixed and injectivity
pinned. Finch instead ruled the hand renderer dissolved: item bodies
render through the `cbor-diag` crate (already the workspace's renderer
in `rumors-tracing`), whose extended diagnostic notation with encoding
indicators carries the injectivity, and the snapshots are re-accepted
once under that ruling. The harness's own framing stays; the semantic
glosses go.

## Rulings landed

- T5: the renderer-vocabulary re-accept class folds into the
  format-change class; AGENTS.md carries one class naming both.
- T9, T138, T140: the capture renderer delegates to `cbor-diag`, pinned
  by revision; injectivity is the tool's, stated in the module doc; no
  round-trip pin, no re-indenting of the tool's output; NaN payload
  bits documented as unrepresentable under the payload contract; all
  wire snapshots re-accepted in one commit naming T138.
- T141: the prose standard, applied to every touched paragraph.

## Stack position

- Base: `b8401660` (main; the lane rebased there for its gate of record)
- Parent: `main`
- Children: none. `p2-walk` (running) leaves the capture module's files
  to this lane.

## Acceptance table

Every row is one the coordinator's verification runner ran on the illumos
box against the lane at `c3c16f34` and `c54314bf`, then a detached
scratch worktree at `fff43de9`, with whole logs kept.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| rev pin (T138) | `040629df` | `grep -n 'cbor-diag' Cargo.toml` | `rev = "a6a713671f92526b921bc926173266257379d609"`, no `branch`; optional under `test-internals` |
| `prose-hygiene-6`, `verification-infra-15` | `815c22a2` | `grep -niE 'hexdump\|renderer-vocabulary\|hex-line' AGENTS.md`; `grep -n 'capture renderer' AGENTS.md` | empty; line 152 names "the wire format or the capture renderer's vocabulary" |
| `remote-capture-atlas-13`, `-17` | `c3c16f34` | `git diff --stat 0926fe32...c3c16f34 -- tests/snapshots`; same for `src` filtered to snapshots | `22 files changed`; `0` under `src` |
| (same) | `c3c16f34` | `git show c3c16f34 --stat \| grep -E 'roundtrip\|inverse\|proptest-regressions'` | empty: no round-trip module, no seeds (T140) |
| (same) | `c3c16f34` | `grep -n -iE 'inject\|NaN\|encoding indicator' capture.rs` | module doc: encoding indicators, "injective on wire bytes", NaN payload bits written as `NaN`, the `Eq` contract |
| (same) | `c3c16f34` | `grep -c '"…"' capture.rs`; em-dash count in `capture.rs` and its tests | `0`; `0` |
| (same) | `c3c16f34` | `cargo nextest run -p rumors --all-features --locked -E 'test(capture::tests) \| binary(gossip_snapshot) \| binary(bootstrap_snapshot) \| binary(retire_snapshot) \| binary(target_message_size) \| binary(opening_supply)'` (box) | `41 tests run: 41 passed`; a second `gossip_snapshot` run writes no `.snap.new` |
| consumers restated | `c3c16f34` | `./tools/digestshare` | `TOTAL: 9260 wire B, 1824 digest B, 76 digests at 24 B each (19.7%)`, equal to the old tool over the old corpus |
| (negative control) | | the tool's byte-string prefix broken (`h'` to `x'`), `./tools/digestshare`, restored | `LIVENESS FAILURE: ... 9260 wire B / 0 digest B were parsed; the renderer's vocabulary has moved out from under this tool.`; exit 1 |
| all | `c3c16f34` | `just gate` on the box (lane log `gate-illumos3.log`) | seven streams ok; `fuzz` failed on `FuzzerPlatform.h:72:2: error: #error "Support for your platform has not been implemented"` (the accepted illumos leg); `test-all` `1810 tests run: 1810 passed` |
| repair `c54314bf` (canonical walk) | `c54314bf` | `git show --stat c54314bf` over `tests/snapshots` | no snapshot moved by the repair; the 22 re-accepted files are unchanged since `339ac043` |
| (same) | `c54314bf` | the seven affected binaries plus `snapshot_liveness` (box) | `47 tests run: 47 passed` |
| (negative control, width) | | the walk's shortest-width comparison made always true | only `non_shortest_length_heads_fall_back` FAILS (the ill-formed simple is refused by the re-encode comparison) |
| (negative control, control characters) | | the control-character predicate made always false | only `text_with_control_characters_falls_back` FAILS |
| `digestshare` | `c54314bf` | `./tools/digestshare --self-test`; `./tools/digestshare` | `self-test ok`; `TOTAL: 9260 wire B, 1824 digest B, 76 digests at 24 B each (19.7%)` |
| (negative control, scope) | | every frame header opens a listing section | `self-test FAILED: measured (180, 93, 4), expected (180, 70, 3)`: the Supply payload's digest-shaped entry now counts |
| all | `c54314bf` | `just gate` on the box (lane log `box-round1.log`) | seven streams ok; `fuzz` the accepted illumos leg |
| repair `fff43de9` (round 2) | `fff43de9` | the seven affected binaries plus `snapshot_liveness` (box, scratch worktree) | `49 tests run: 49 passed`; the repair touches no snapshot |
| (negative control, NaN) | | the NaN guard made always false | only `nans_fall_back` FAILS |
| (negative control, simples) | | the `Simple(24..=31)` arm disabled | only `simple_values_without_a_well_formed_spelling_fall_back` FAILS |
| `digestshare` | `fff43de9` | `--self-test`; the corpus; `body.split("\n")` reverted to `splitlines()` | `self-test ok`; `TOTAL: 9260 wire B, 1824 digest B, 76 digests`; reverted: `self-test FAILED: measured (180, 93, 4), expected (180, 70, 3)` (the U+2028 fixture opens a section) |
| all | `fff43de9` | `just gate` on the box (lane log `box-round2.log`) | seven streams ok; `fuzz` the accepted illumos leg |

## Fresh-eyes rounds

**Round 1** (surface correctness with operational validity), against
`c3c16f34`, by sha, with the fork's parser and printer read in full.
One defect: the module doc's injectivity claim was an absolute the tool
does not deliver, since cbor-diag spells width indicators only for
integers, tags, and floats and accepts an ill-formed two-byte simple
value, so non-canonical items collide (`82 40 58 00` and `82 58 00 40`
both render `[h'', h'']`); nothing pinned is affected because the codec
writes shortest heads and ingress rejects the rest. Three smaller items:
text strings with control characters could forge a harness header line
(cbor-diag escapes only quotes and backslashes); the listing-entry
patterns in `digestshare` and the snapshot test matched only the
one-entry-per-line layout cbor-diag happens to choose at the current
digest width; and the digest scan could count a payload map with
integer keys. Repairs landed: the harness now holds
every item to canonical form before delegating (shortest heads,
definite lengths, an exact re-encode, text free of control characters,
embedded CBOR held to the same rules), sending the rest to the hex
fallback, with point tests for both collision pairs, the ill-formed
simple, an embedded non-canonical item, and a newline payload; the
three consumers state the header rule as the renderer's guarantee; the
listing-entry patterns are layout-independent with a one-line fixture
in `digestshare`'s self-test; the digest scan is bounded to the greeting
and Query frames; the byte-count and NaN sentences are exact and the
crate-doc citation is a rustdoc link.

**Round 2** (operational validity and tree interaction), against
`c54314bf`, with the fork's parser, encoder, and printer read in full:
the reviewer enumerated every head kind the printer spells without a
width and every value it collapses, checked the walk's depth accounting
against the printer's (both stop unfolding at the same item), confirmed
every item the codec legitimately emits is canonical (no pin moves) and
scanned all 22 snapshots for fallback lines, indefinite forms, floats,
and control or separator characters (none). One gap in the repair: NaN
sign and payload bits round-trip bit-exactly through the re-encode
check, so two NaN spellings still collided while the doc claimed a
fallback; the final round rejects every NaN in the walk. Three small
items landed with it: `digestshare` splits on the renderer's own `\n`
rather than Python's `splitlines` (which also breaks on U+2028/2029),
the two-byte spelling of simple values 24 through 31 is rejected so the
doc's "ill-formed simple" sentence is exact, `canonical`'s doc states
one criterion, and a ghost `│` skip in `digestshare` is deleted.
The rounds stopped here.

Reviewer notes not acted on: Cf characters (bidi overrides, zero-width)
print raw and can reorder a snapshot visually without breaking
injectivity; the walk mirrors two printer internals a fork bump could
change, held by the rev pin and one point test per collapsed class;
the fork's unmarked embedded-depth stop
(recorded in `triage/cbor-diag-fork-items.md`, with the parser's
trailing-comma asymmetry, the control-character escaping, the missing
width indicators, and NaN); `nom`'s preallocation behavior on a frame
claiming a huge array length was not confirmed (harness-only
exposure).

## Stops

<!-- STOPS -->

## Reading order

### new tests and negative controls

- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:213` ([hunk](#hunk-16))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:241` ([hunk](#hunk-17))

### production edits

- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture.rs:1` ([hunk](#hunk-9))
- remote-capture-atlas-17 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:15` ([hunk](#hunk-9))
- remote-capture-atlas-17 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:18` ([hunk](#hunk-9))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture.rs:30` ([hunk](#hunk-9))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture.rs:51` ([hunk](#hunk-9))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:13` ([hunk](#hunk-9))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:23` ([hunk](#hunk-9))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:28` ([hunk](#hunk-9))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:14` ([hunk](#hunk-9))
- remote-capture-atlas-13 (T141) at `src/tree/mirror/streaming/remote/codec/capture.rs:154` ([hunk](#hunk-10))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture.rs:177` ([hunk](#hunk-11))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture.rs:199` ([hunk](#hunk-12))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture.rs:235` ([hunk](#hunk-13))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:272` ([hunk](#hunk-14))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture.rs:415` ([hunk](#hunk-14))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:277` ([hunk](#hunk-14))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:300` ([hunk](#hunk-14))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:364` ([hunk](#hunk-14))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:278` ([hunk](#hunk-14))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:289` ([hunk](#hunk-14))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:350` ([hunk](#hunk-14))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture.rs:356` ([hunk](#hunk-14))

### tests and prose

- prose-hygiene-6 (T5) at `AGENTS.md:152` ([hunk](#hunk-2))
- prose-hygiene-6 (T5) at `AGENTS.md:156` ([hunk](#hunk-2))
- verification-infra-15 (T5) at `AGENTS.md:156` ([hunk](#hunk-2))
- remote-capture-atlas-13 (T138) at `Cargo.lock:0` ([hunk](#hunk-3))
- remote-capture-atlas-13 (T138) at `Cargo.lock:0` ([hunk](#hunk-4))
- remote-capture-atlas-13 (T138) at `Cargo.toml:29` ([hunk](#hunk-5))
- remote-capture-atlas-13 (T138) at `Cargo.toml:38` ([hunk](#hunk-5))
- remote-capture-atlas-13 (T138) at `Cargo.toml:115` ([hunk](#hunk-6))
- remote-capture-atlas-13 (T138) at `Cargo.toml:145` ([hunk](#hunk-7))
- remote-capture-atlas-13 (T138) at `justfile:214` ([hunk](#hunk-8))
- remote-capture-atlas-13 (T140) at `justfile:220` ([hunk](#hunk-8))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:1` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:40` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:61` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:171` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:91` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:112` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:154` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:127` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T140) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:140` ([hunk](#hunk-15))
- remote-capture-atlas-13 (T138) at `src/tree/mirror/streaming/remote/codec/capture/tests.rs:277` ([hunk](#hunk-18))
- remote-capture-atlas-13 (T138) at `tests/gossip_snapshot.rs:13` ([hunk](#hunk-19))
- remote-capture-atlas-13 (T138) at `tests/gossip_snapshot.rs:166` ([hunk](#hunk-20))
- remote-capture-atlas-13 (T140) at `tests/gossip_snapshot.rs:166` ([hunk](#hunk-20))
- remote-capture-atlas-13 (T138) at `tests/gossip_snapshot.rs:183` ([hunk](#hunk-21))
- remote-capture-atlas-13 (T140) at `tests/gossip_snapshot.rs:211` ([hunk](#hunk-21))
- remote-capture-atlas-13 (T138) at `tests/gossip_snapshot.rs:221` ([hunk](#hunk-21))
- remote-capture-atlas-13 (T140) at `tests/gossip_snapshot.rs:223` ([hunk](#hunk-21))
- remote-capture-atlas-13 (T138) at `tests/gossip_snapshot.rs:259` ([hunk](#hunk-22))
- remote-capture-atlas-13 (T138) at `tests/opening_supply.rs:48` ([hunk](#hunk-23))
- remote-capture-atlas-13 (T138) at `tests/snapshots/bootstrap_snapshot__empty_provider.snap:0` ([hunk](#hunk-24))
- remote-capture-atlas-13 (T138) at `tests/snapshots/bootstrap_snapshot__mutual_bootstrap_bails.snap:0` ([hunk](#hunk-25))
- remote-capture-atlas-13 (T138) at `tests/snapshots/bootstrap_snapshot__populated_provider.snap:0` ([hunk](#hunk-26))
- remote-capture-atlas-13 (T138) at `tests/snapshots/bootstrap_snapshot__string_payload.snap:0` ([hunk](#hunk-27))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__asymmetric_message_targets_unbatch_the_run.snap:0` ([hunk](#hunk-28))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__batched_supply_run.snap:0` ([hunk](#hunk-29))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__both_redact_the_same_message.snap:0` ([hunk](#hunk-30))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__bulk_initiator_ships_opening_supplies.snap:0` ([hunk](#hunk-31))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__converged_forks_noop.snap:0` ([hunk](#hunk-32))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__deep_trie_divergence.snap:0` ([hunk](#hunk-33))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__early_supplies_honor_redactions.snap:0` ([hunk](#hunk-34))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__empty_pair_converges_immediately.snap:0` ([hunk](#hunk-35))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__fork_insert_redact.snap:0` ([hunk](#hunk-36))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__one_sided_transfer.snap:5` ([hunk](#hunk-37))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__one_sided_transfer.snap:0` ([hunk](#hunk-37))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__redaction_only.snap:0` ([hunk](#hunk-38))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__same_live_content_divergent_versions.snap:0` ([hunk](#hunk-39))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__shared_subtree_dispute_pins_a_nonempty_query.snap:0` ([hunk](#hunk-40))
- remote-capture-atlas-13 (T138) at `tests/snapshots/gossip_snapshot__string_payload.snap:0` ([hunk](#hunk-41))
- remote-capture-atlas-13 (T138) at `tests/snapshots/retire_snapshot__divergent_retire.snap:0` ([hunk](#hunk-42))
- remote-capture-atlas-13 (T138) at `tests/snapshots/retire_snapshot__empty_retire.snap:0` ([hunk](#hunk-43))
- remote-capture-atlas-13 (T138) at `tests/snapshots/retire_snapshot__mutual_retire_declines.snap:0` ([hunk](#hunk-44))
- remote-capture-atlas-13 (T138) at `tests/snapshots/retire_snapshot__retire_into_bootstrapper.snap:0` ([hunk](#hunk-45))
- remote-capture-atlas-13 (T138) at `tests/target_message_size.rs:127` ([hunk](#hunk-46))
- remote-capture-atlas-13 (T138) at `tools/digestshare:4` ([hunk](#hunk-47))
- remote-capture-atlas-13 (T138) at `tools/digestshare:20` ([hunk](#hunk-48))
- remote-capture-atlas-13 (T138) at `tools/digestshare:48` ([hunk](#hunk-49))
- remote-capture-atlas-13 (T140) at `tools/digestshare:48` ([hunk](#hunk-49))
- remote-capture-atlas-13 (T140) at `tools/digestshare:50` ([hunk](#hunk-49))
- remote-capture-atlas-13 (T140) at `tools/digestshare:72` ([hunk](#hunk-49))
- remote-capture-atlas-13 (T140) at `tools/digestshare:57` ([hunk](#hunk-49))
- remote-capture-atlas-13 (T140) at `tools/digestshare:114` ([hunk](#hunk-50))
- remote-capture-atlas-13 (T140) at `tools/digestshare:109` ([hunk](#hunk-50))
- remote-capture-atlas-13 (T138) at `tools/digestshare:147` ([hunk](#hunk-51))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p1-renderer.tsv `@@ -0,0 +1,88 @@`

```diff
@@ -0,0 +1,88 @@
+# p1-renderer lane annotations: path, line (final tree), entry id, ruling, note.
+Cargo.toml	29	remote-capture-atlas-13	T138	T138 pins the fork by revision so a fork change cannot reflow the wire snapshots without a commit here; the manifest comment states that argument beside the existing depth-limit rationale.
+Cargo.toml	38	remote-capture-atlas-13	T138	The rev is the commit Cargo.lock already resolved under the branch, so the lock's source string changes and nothing else; verified with an offline metadata run.
+Cargo.toml	145	remote-capture-atlas-13	T138	rumors gains cbor-diag as an optional dependency: the capture renderer is compiled under `test` or the `test-internals` feature, so the crate must be available under that feature, not only as a dev-dependency.
+Cargo.toml	115	remote-capture-atlas-13	T138	The feature enables the optional dependency; an application never enables test-internals (the feature comment says so), so cbor-diag never reaches a production build.
+AGENTS.md	152	prose-hygiene-6	T5	The one surviving re-accept class names both cases it governs: a deliberate, owner-ruled change to the wire format or to the capture renderer's vocabulary, named in the re-accepting commit. Without the second case a renderer change that moves snapshots (T138's own) would have no sanctioned path.
+AGENTS.md	156	prose-hygiene-6	T5	Deletion annotated at the line that follows it: the renderer-vocabulary re-accept class and its hexdump-line witness are removed, exactly the sentences from "One further sanctioned re-accept class" through "explicitly."; the witness named hexdump lines the corpus does not have. The bookmark fixture re-pin class and the rest of the paragraph are untouched.
+AGENTS.md	156	verification-infra-15	T5	Same deletion, same line: this entry asked for the witness restated plus a mechanical `snapwitness` tool; T5 replaces both resolutions with removing the class, so no tool is written and the paragraph reads as one rule.
+src/tree/mirror/streaming/remote/codec/capture.rs	1	remote-capture-atlas-13	T138	T138: item bodies render through cbor-diag; the hand renderer (its node tree, parser, listing and tag arms, glosses, and depth budget) is gone. The module doc is restated for that design and, per T141, cut to what a maintainer here needs: it no longer restates cbor-diag's own unfold and depth behavior, which the tests below pin.
+src/tree/mirror/streaming/remote/codec/capture.rs	15	remote-capture-atlas-17	T140	T140: the injectivity claim is delegated to cbor-diag's encoding indicators and stated here; no round-trip pin, generators, or inverse parser land. The sentence that this module adds nothing that could collapse the rendering is what the verbatim output below guarantees.
+src/tree/mirror/streaming/remote/codec/capture.rs	18	remote-capture-atlas-17	T140	T140: NaN payload bits are documented as unrepresentable under the payload contract (the `Eq` bound excludes float fields; a hand-written `Eq` admitting NaN has declared its NaNs equal); the protocol itself emits no floats.
+src/tree/mirror/streaming/remote/codec/capture.rs	30	remote-capture-atlas-13	T138	The harness-grammar sentence is restated to what remains asserted (a control item's opening head, the frame opener, the frame-to-stream match, the labels); body canonicality is no longer a panic, since a non-shortest head renders with its encoding indicator. Prose pass: the paragraph's em-dash and restatements are gone.
+src/tree/mirror/streaming/remote/codec/capture.rs	51	remote-capture-atlas-13	T138	Imports reduced to what the framing still reads: the head grammar for labels, control-item shapes, and frame openers.
+src/tree/mirror/streaming/remote/codec/capture.rs	154	remote-capture-atlas-13	T141	Nit swept in a file this lane edits: em-dashes replaced by parentheses; wording otherwise unchanged.
+src/tree/mirror/streaming/remote/codec/capture.rs	235	remote-capture-atlas-13	T138	Judgment call on where the protocol-phase label lives for a frame: the header line now carries the decoded signal (`frame N (B bytes) / Supply(End) /`), since the opener items are inside the one diagnostic item and cannot be annotated individually without a hand walk. The frame-grammar assertions stay.
+src/tree/mirror/streaming/remote/codec/capture.rs	272	remote-capture-atlas-13	T140	T140: the body renderer emits cbor-diag's pretty output verbatim under the header, with no re-indenting or re-splitting; parse failure (malformed, trailing bytes, too deep) goes to the explicit fallback with the parser's error.
+src/tree/mirror/streaming/remote/codec/capture.rs	415	remote-capture-atlas-13	T138	The fallback keeps the two-line form (failure line with the reason, then the exact hex) so a fallback is unmistakable in a diff.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	1	remote-capture-atlas-13	T138	The test module doc states the renderer's commitments as they now stand. Tests asserting the hand renderer's forms (field localization, listing annotations, order verdict, version gloss, hand depth budget) are removed with the code they pinned.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	40	remote-capture-atlas-13	T138	Point test for the frame header (index, exact byte count, decoded signal) and that the body is diagnostic notation with the supply run unfolded to the payload.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	61	remote-capture-atlas-13	T138	The explicit-fallback commitment on a truncated array; the reason is cbor-diag's own error text.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	171	remote-capture-atlas-13	T138	Trailing bytes after an item are not one item: cbor-diag's parse_bytes rejects them (verified in its source), so the whole buffer falls back rather than the tail being dropped.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	241	remote-capture-atlas-13	T138	Nit swept: the control-item naming test's doc claimed the unknown-shape panic without exercising it; the claim is now its own should_panic test.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	277	remote-capture-atlas-13	T138	The depth stress tests are kept and re-denominated against cbor-diag's DEFAULT_DEPTH_LIMIT: an embedded chain ten times deeper renders with bounded recursion through the item path and the frame path, and structure past the limit falls back explicitly. The no-unbounded-recursion property stays committed as a stress test.
+tools/digestshare	48	remote-capture-atlas-13	T138	Consequence T138 did not name: this gate leg counted digests by the `/ digest /` gloss the ruling drops, and its liveness floor would have failed the gate. It now recognizes a listing entry by shape (an unsigned-integer key with a byte-string value, the wire's only such map entry) and anchors the stream header to its full form; totals over the new corpus equal the old tool's over the old one (9260 wire B, 1824 digest B, 76 digests).
+justfile	214	remote-capture-atlas-13	T138	The recipe comment names listing entries instead of digest annotations, matching the tool.
+tests/target_message_size.rs	127	remote-capture-atlas-13	T138	Consequence T138 did not name: this suite counted supply frames by the hand renderer's annotated state line. It now counts `frame N (B bytes) / Supply... /` header lines, the label's home. Prose finding, not edited: paragraphs elsewhere in this file carry em-dashes (lines 69, 161, 203, 217, 318), outside what this commit touches.
+tests/gossip_snapshot.rs	166	remote-capture-atlas-13	T138	Consequence T138 did not name: three scenarios in this suite assert on the frame sequence by scanning the hand renderer's annotated state lines and listing headers. The scan now reads the frame header's label; with bodies verbatim at column zero, the doc states the oracle assumption plainly (no rendered item starts a line with `frame `) instead of claiming an absolute distinction.
+tests/gossip_snapshot.rs	183	remote-capture-atlas-13	T138	The query-listing child count is counted from the listing's entries between a Query frame header and the next frame or capture header, since the `/ listing: N child(ren) /` gloss is dropped and bodies at column zero can no longer end a section by indentation.
+tests/gossip_snapshot.rs	211	remote-capture-atlas-13	T140	Section boundaries for the two scans, now that item bodies stand at column zero: a capture header is one of the harness's fixed column-zero forms.
+tests/gossip_snapshot.rs	221	remote-capture-atlas-13	T138	The same listing-entry shape tools/digestshare recognizes, stated once here for the suite.
+tests/gossip_snapshot.rs	13	remote-capture-atlas-13	T138	Prose kept true to the rendering: payloads are spotted as decimal integers in diagnostic notation, no longer as hex bytes.
+tests/snapshots/gossip_snapshot__one_sided_transfer.snap	5	remote-capture-atlas-13	T138	All 22 wire snapshots under tests/snapshots move, re-accepted with `cargo insta accept` in this commit, the sanctioned case T138 names (this row stands for the set; the commit message lists every file). Framing lines are unchanged; every item body is now cbor-diag's pretty notation verbatim, so a re-accept diff shows the same values in the notation's spelling (encoding indicators, `<<...>>` unfolds) and without the dropped glosses. No snapshot outside tests/snapshots moved.
+tests/opening_supply.rs	48	remote-capture-atlas-13	T138	Consequence T138 did not name, found by the gate: this suite counted frames by the hand renderer's annotated state line. It now reads the frame header's label, like the other two suites; the doc is shortened to the header's form.
+src/tree/mirror/streaming/remote/codec/capture.rs	13	remote-capture-atlas-13	T140	Fresh-eyes repair (item 1): the injectivity argument was wrong as stated, since cbor-diag spells width indicators only for integers, negatives, tags, and floats, and its binary parser accepts the ill-formed two-byte simple form; equal-length collisions existed (`82 40 58 00` vs `82 58 00 40`, `82 f4 f8 14` vs `82 f8 14 f4`). The doc now states the design that makes the claim true: canonical form is checked first, notation is exact on canonical items, everything else is hex.
+src/tree/mirror/streaming/remote/codec/capture.rs	23	remote-capture-atlas-13	T140	Fresh-eyes repair (item 5): a NaN loses its sign bit as well as its payload; and with the re-encoding check such a float now falls back to hex rather than rendering as `NaN`. The payload-contract sentence stays, with the crate docs cited by rustdoc link instead of by string.
+src/tree/mirror/streaming/remote/codec/capture.rs	28	remote-capture-atlas-13	T140	Fresh-eyes repair (item 5): only the stream count is transport-sourced; a control item's or frame's count is the hook item's length, and their equality is the totality witness's claim, now said so.
+src/tree/mirror/streaming/remote/codec/capture.rs	277	remote-capture-atlas-13	T140	Fresh-eyes repair (item 1): re-encoding equal to the bytes, checked before the walk; this is what rejects the ill-formed simple (re-encoding uses the one-byte form) and any NaN bits beyond the canonical NaN.
+src/tree/mirror/streaming/remote/codec/capture.rs	300	remote-capture-atlas-13	T140	Fresh-eyes repair (item 1): the canonical walk over the parsed item, holding every head the printer spells without a width to its shortest width, containers to definite lengths, text to no control characters (item 2), and embedded content (tags 24 and 63) to the same rules plus exact re-encoding, because the printer unfolds it and a wide head inside it would collide the same way. The walk mirrors the printer's depth budget so recursion is bounded by the parser's limit; a level past the budget is accepted because the printer shows it as hex. Floats and simple values are exempt: their indicator or number is spelled, and the re-encoding check covers the rest.
+src/tree/mirror/streaming/remote/codec/capture.rs	364	remote-capture-atlas-13	T140	Embedded content that does not parse is left alone: the printer shows it as a byte string, which is hex and injective. Content that parses must re-encode exactly (a shorter ill-formed spelling inside would collide with a longer well-formed one at equal outer length) and be canonical.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	91	remote-capture-atlas-13	T140	Fresh-eyes repair (item 1): the first collision pair as a point test. Both wide variants carry the non-shortest head, so both fall back (each to its own hex); the canonical array alone renders in notation; the embedded case is included.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	112	remote-capture-atlas-13	T140	Fresh-eyes repair (item 1): the second collision pair and the ill-formed simple, caught by the re-encoding check.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	154	remote-capture-atlas-13	T140	Fresh-eyes repair (item 2): the header-forgery case. A string payload carrying a newline and a frame header's text renders as hex under its real header, so exactly one line begins with `frame `. Durable fix recorded for the owner: the fork should escape control characters in text (T140's fork list); until then the walk sends such text to hex.
+tests/gossip_snapshot.rs	166	remote-capture-atlas-13	T140	Fresh-eyes repair (item 2): the doc now states the renderer's guarantee (no body line begins with `frame `) instead of an oracle assumption about the corpus; same for is_capture_header and the target_message_size and opening_supply scanners.
+tests/gossip_snapshot.rs	223	remote-capture-atlas-13	T140	Fresh-eyes repair (item 3): the entry scan was layout-dependent (one entry per line, which cbor-diag chooses only because the 24-byte digest sits just above its trivial-layout threshold). Entries are now counted wherever they sit on a line, preceded by a line start, brace, or comma; the same rule tools/digestshare applies.
+tools/digestshare	48	remote-capture-atlas-13	T140	Fresh-eyes repair (item 3): all entries on a line are counted, so a listing printed on one line inside the frame array no longer undercounts silently.
+tools/digestshare	50	remote-capture-atlas-13	T140	Fresh-eyes repair (item 4): the shape rule alone was unsound for a payload map of uint keys and byte-string values inside a Supply frame; the scan is now bounded to the greeting control item and Query frame bodies, tracked by the same headers the snapshot suite reads, and the tool's doc states that rule.
+tools/digestshare	114	remote-capture-atlas-13	T140	Fresh-eyes repair (item 3): a self-test over a hand-written capture holds the patterns to a one-line Query listing of two entries, the greeting's listing, and a Supply payload of the digest shape that must not count; the recipe runs it before the corpus measurement.
+justfile	220	remote-capture-atlas-13	T140	The recipe runs the tool's self-test first, as the other lint-tier tools do.
+src/tree/mirror/streaming/remote/codec/capture.rs	14	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (items 1, 3, 4): the doc claimed the re-encoding check rejected NaN bits and ill-formed simples; cbor-diag's float path is bit-exact and simple values 24..=31 re-encode verbatim, so neither was true. The canonical-form list now names what the walk actually holds: every head shortest, definite lengths, well-formed simple values, no NaN, no control characters, exact re-encoding.
+src/tree/mirror/streaming/remote/codec/capture.rs	278	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 1): the reason string no longer names NaN bits or simple values, which this check does not catch; it names only what it detects.
+src/tree/mirror/streaming/remote/codec/capture.rs	289	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 4): one criterion stated once, every head shortest, with the note that only string and container heads need it for injectivity since the printer spells the other widths.
+src/tree/mirror/streaming/remote/codec/capture.rs	350	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 1): every NaN is rejected by value. The parser reads floats by from_bits and the encoder writes to_bits, so a NaN's sign and payload survive re-encoding while the printer writes a bare `NaN`; `fb 7ff8…` and `fb fff8…` both rendered `NaN_3`. Rejecting by value also drops any dependence on half's f16/f32 conversion path.
+src/tree/mirror/streaming/remote/codec/capture.rs	356	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 3): simple values 24 through 31 have no well-formed spelling, and their two-byte form re-encodes verbatim, so the walk rejects them by value; no collision resulted, but the doc's claim now holds.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	127	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 1): the sign pair as a point test, each side to its own hex, beside a finite float that renders in notation.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	140	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 3): `f8 18` falls back; `f8 20` (32) still renders.
+tools/digestshare	72	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 2): splitlines() also breaks on U+2028 and U+2029, which are not control characters, so the walk passes them and cbor-diag prints them raw; the Rust scanners split on the line feed alone, and the tool now does too, so the renderer's header guarantee holds for it.
+tools/digestshare	109	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 2): the self-test carries a Supply payload text with U+2028-separated header-shaped and listing-shaped tails, which must neither open a Query section nor count a digest.
+tools/digestshare	57	remote-capture-atlas-13	T140	Fresh-eyes repair, round 2 (item 4): the `│` skip referred to a side-by-side render form no snapshot has carried since the base, and a payload containing `│` would have dropped a whole capture from the meter silently; the skip and its docstring paragraph are deleted, and the liveness wording no longer says V2.
+tests/snapshots/bootstrap_snapshot__empty_provider.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/bootstrap_snapshot__mutual_bootstrap_bails.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/bootstrap_snapshot__populated_provider.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/bootstrap_snapshot__string_payload.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__asymmetric_message_targets_unbatch_the_run.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__batched_supply_run.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__both_redact_the_same_message.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__bulk_initiator_ships_opening_supplies.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__converged_forks_noop.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__deep_trie_divergence.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__early_supplies_honor_redactions.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__empty_pair_converges_immediately.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__fork_insert_redact.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__one_sided_transfer.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__redaction_only.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__same_live_content_divergent_versions.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__shared_subtree_dispute_pins_a_nonempty_query.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/gossip_snapshot__string_payload.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/retire_snapshot__divergent_retire.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/retire_snapshot__empty_retire.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/retire_snapshot__mutual_retire_declines.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+tests/snapshots/retire_snapshot__retire_into_bootstrapper.snap	0	remote-capture-atlas-13	T138	T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.
+Cargo.lock	0	remote-capture-atlas-13	T138	Lock consequences of the manifest: the cbor-diag entry's source string changes from the branch to the pinned rev (same commit, so no package moves), and rumors gains the cbor-diag edge for its optional dependency. Refreshed offline; nothing else in the lock changes.
+src/tree/mirror/streaming/remote/codec/capture.rs	177	remote-capture-atlas-13	T138	Call-site consequence of T140's verbatim output: control-item bodies no longer take an indent argument.
+src/tree/mirror/streaming/remote/codec/capture.rs	199	remote-capture-atlas-13	T138	The frame header line moved into render_frame (it now carries the decoded signal, so it needs the opener), and the loop passes the frame index through; the byte count on the header is unchanged.
+src/tree/mirror/streaming/remote/codec/capture/tests.rs	213	remote-capture-atlas-13	T138	Nit: the control-item naming test's doc claimed the unknown-shape panic without exercising it; the claim moved to its own should_panic test below, and this doc states only what this body checks.
+tests/gossip_snapshot.rs	259	remote-capture-atlas-13	T138	stream_frames ended a stream's section at any column-zero line, which held while bodies were indented; with bodies verbatim at column zero it ends at the next capture header instead, the same boundary the query-listing scan uses.
+tools/digestshare	4	remote-capture-atlas-13	T138	Docstring restated for the diagnostic-notation corpus and this lane's scan rules: what a listing entry looks like, that the scan is bounded to the greeting and Query bodies, and that no side-by-side render form exists to skip.
+tools/digestshare	20	remote-capture-atlas-13	T138	Docstring: the wire-byte header forms are unchanged; the liveness paragraph names the self-test's fixtures (one-line listing, Supply payload of digest shape, U+2028 in payload text) so a reader knows what the floor is held to.
+tools/digestshare	147	remote-capture-atlas-13	T138	The liveness floor's counter and message no longer say V2, since no other render form exists in the corpus; the floor itself (zero wire bytes or zero digests fails) is unchanged.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### AGENTS.md `@@ -148,17 +148,11 @@ You can leave durable notes and other artifacts of exploration and ideation in`

```diff
@@ -148,17 +148,11 @@ You can leave durable notes and other artifacts of exploration and ideation in
 - `tests/gossip_snapshot.rs` and the `insta` snapshots pin the wire format
   byte-for-byte; re-accept them only after a deliberate protocol change,
   never as an accommodation of drift. Pre-release (no shipped version
-  exists to hold compatible), that means a deliberate, owner-ruled format
-  change, named explicitly in the re-accepting commit. Once the first
+  exists to hold compatible), that means a deliberate, owner-ruled change
+  to the wire format or to the capture renderer's vocabulary, named
+  explicitly in the re-accepting commit. Once the first
   release ships, a format change means a new protocol version, never a
   mutation of a released one.
-  One further sanctioned re-accept class: a renderer-vocabulary change
-  (the capture renderer's decoded annotations gained or reworded, the
-  wire untouched), permitted only with the hex-line-preservation
-  witness — every hexdump line sequence identical to the parent commit,
-  the diff pure annotation additions or rewordings — and the re-accepting
-  commit stating that witness and the renderer-change attribution
-  explicitly.
   To re-accept deliberately: `just test-all`, then `cargo insta review`
   (install: `cargo install cargo-insta`), then commit the updated
   `tests/snapshots/*.snap`. For the bookmark pins (`src/bookmark/format/`),
```

<!-- annotation -->
> **prose-hygiene-6** (T5), line 152:
>
> The one surviving re-accept class names both cases it governs: a deliberate, owner-ruled change to the wire format or to the capture renderer's vocabulary, named in the re-accepting commit. Without the second case a renderer change that moves snapshots (T138's own) would have no sanctioned path.

<!-- annotation -->
> **prose-hygiene-6** (T5), line 156:
>
> Deletion annotated at the line that follows it: the renderer-vocabulary re-accept class and its hexdump-line witness are removed, exactly the sentences from "One further sanctioned re-accept class" through "explicitly."; the witness named hexdump lines the corpus does not have. The bookmark fixture re-pin class and the rest of the paragraph are untouched.

<!-- annotation -->
> **verification-infra-15** (T5), line 156:
>
> Same deletion, same line: this entry asked for the witness restated plus a mechanical `snapwitness` tool; T5 replaces both resolutions with removing the class, so no tool is written and the paragraph reads as one rule.

<a id="hunk-3"></a>
### Cargo.lock `@@ -190,7 +190,7 @@ checksum = "37b2a672a2cb129a2e41c10b1224bb368f9f37a2b16b612598138befd7b37eb5"`

```diff
@@ -190,7 +190,7 @@ checksum = "37b2a672a2cb129a2e41c10b1224bb368f9f37a2b16b612598138befd7b37eb5"
 [[package]]
 name = "cbor-diag"
 version = "0.1.12"
-source = "git+https://github.com/oxidecomputer/cbor-diag-rs?branch=depth-limit#a6a713671f92526b921bc926173266257379d609"
+source = "git+https://github.com/oxidecomputer/cbor-diag-rs?rev=a6a713671f92526b921bc926173266257379d609#a6a713671f92526b921bc926173266257379d609"
 dependencies = [
  "bs58",
  "chrono",
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> Lock consequences of the manifest: the cbor-diag entry's source string changes from the branch to the pinned rev (same commit, so no package moves), and rumors gains the cbor-diag edge for its optional dependency. Refreshed offline; nothing else in the lock changes.

<a id="hunk-4"></a>
### Cargo.lock `@@ -1256,6 +1256,7 @@ dependencies = [`

```diff
@@ -1256,6 +1256,7 @@ dependencies = [
  "async-stream",
  "before",
  "bytes",
+ "cbor-diag",
  "ciborium",
  "criterion",
  "futures",
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> Lock consequences of the manifest: the cbor-diag entry's source string changes from the branch to the pinned rev (same commit, so no package moves), and rumors gains the cbor-diag edge for its optional dependency. Refreshed offline; nothing else in the lock changes.

<a id="hunk-5"></a>
### Cargo.toml `@@ -25,13 +25,16 @@ bytes = "1"`

```diff
@@ -25,13 +25,16 @@ bytes = "1"
 # Oxide's fork of cbor-diag: nesting depth is bounded by default in every
 # input-driven parse and print (upstream recurses on input-controlled
 # depth), so wire-derived bytes can be rendered without a guard in front.
+# Pinned by revision, not branch: the wire snapshots under `tests/snapshots`
+# are rendered through this crate, so a change on the fork's branch must
+# not reflow them without a commit here that names the new revision.
 # Its `url` dependency reaches idna, whose unicode backend is selected by
 # `idna_adapter`: the lock pins idna_adapter to 1.1 (the unicode-rs
 # backend), because the icu backend behind later versions carries derive
 # macros on a second `syn` major, which the bans check rejects. If a
 # `cargo update` resurfaces a duplicate `syn`, re-pin with
 # `cargo update -p idna_adapter --precise 1.1.0`.
-cbor-diag = { git = "https://github.com/oxidecomputer/cbor-diag-rs", branch = "depth-limit" }
+cbor-diag = { git = "https://github.com/oxidecomputer/cbor-diag-rs", rev = "a6a713671f92526b921bc926173266257379d609" }
 # Pinned exactly: depth admission and wire ingress run the same compiled
 # deserializer, so decode-recursion accounting is symmetric within a binary,
 # and across a mixed-version fleet it holds only if every build shares one
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 29:
>
> T138 pins the fork by revision so a fork change cannot reflow the wire snapshots without a commit here; the manifest comment states that argument beside the existing depth-limit rationale.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 38:
>
> The rev is the commit Cargo.lock already resolved under the branch, so the lock's source string changes and nothing else; verified with an offline metadata run.

<a id="hunk-6"></a>
### Cargo.toml `@@ -106,7 +109,7 @@ bench = false`

```diff
@@ -106,7 +109,7 @@ bench = false
 # Test-only introspection and deterministic scheduling infrastructure.
 # Enabled for this crate's own tests via the self-referential dev-dependency
 # below; never enable it in an application.
-test-internals = ["tokio/rt"]
+test-internals = ["tokio/rt", "dep:cbor-diag"]
 # The public conformance suite for caller-built `Link` implementations:
 # enable from a dev-dependency to validate a custom transport against the
 # link contract. Documented; safe (though pointless) in an application.
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 115:
>
> The feature enables the optional dependency; an application never enables test-internals (the feature comment says so), so cbor-diag never reaches a production build.

<a id="hunk-7"></a>
### Cargo.toml `@@ -137,6 +140,9 @@ futures = { workspace = true }`

```diff
@@ -137,6 +140,9 @@ futures = { workspace = true }
 futures-util = { workspace = true }
 async-stream = { workspace = true }
 rand = { workspace = true }
+# The wire-capture renderer behind `test-internals` renders every captured
+# item in diagnostic notation through this crate.
+cbor-diag = { workspace = true, optional = true }
 
 [dev-dependencies]
 rumors = { workspace = true, features = ["test-internals", "conformance"] }
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 145:
>
> rumors gains cbor-diag as an optional dependency: the capture renderer is compiled under `test` or the `test-internals` feature, so the crate must be available under that feature, not only as a dev-dependency.

<a id="hunk-8"></a>
### justfile `@@ -211,12 +211,13 @@ manifestlint:`

```diff
@@ -211,12 +211,13 @@ manifestlint:
 # tools/digestshare reads the committed V2 wire captures and totals digest
 # vs non-digest bytes. As a gate leg it checks the renderer-vocabulary
 # contract, not a threshold: the tool exits nonzero when the corpus's
-# byte-count headers or digest annotations stop matching its patterns (the
+# byte-count headers or listing entries stop matching its patterns (the
 # renderer's vocabulary moved out from under the meter), never on the
 # measured ratio. Build-free, so it rides the lint tier.
 
 # Check the wire-capture renderer vocabulary via the digest-share meter.
 digestshare:
+    ./tools/digestshare --self-test
     ./tools/digestshare
 
 # tools/readme mirrors each crate's crate-level rustdoc into its README via
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 214:
>
> The recipe comment names listing entries instead of digest annotations, matching the tool.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 220:
>
> The recipe runs the tool's self-test first, as the other lint-tier tools do.

<a id="hunk-9"></a>
### src/tree/mirror/streaming/remote/codec/capture.rs `@@ -1,64 +1,58 @@`

```diff
@@ -1,64 +1,58 @@
-//! CBOR reflection rendering of captured V2 traffic.
+//! Rendering of captured V2 traffic in CBOR diagnostic notation.
 //!
 //! The snapshot suites pin every wire byte of a captured session, and
-//! this module is the form that pin takes: each observed item — one
-//! CBOR item per line of the hook's contract — renders as a fully
-//! unfolded value tree in extended-diagnostic-style notation, with a
-//! rumors naming layer as `/ comment /` annotations (signal names,
-//! listing children, tagged-atom meanings). A reviewer reading a
-//! re-accept diff sees the semantic field that moved; a generic CBOR
-//! reader sees plain diagnostic notation.
+//! this module is the form that pin takes: the harness's framing (each
+//! direction, each control item and data frame with its index, exact
+//! byte count, and protocol-phase label) over each item rendered by
+//! `cbor-diag` in extended diagnostic notation with encoding
+//! indicators, embedded CBOR (tags 24 and 63) unfolded, verbatim under
+//! its header.
 //!
 //! # Why a rendering with no hexdump is still a byte pin
 //!
-//! The wire is deterministic-encoding CBOR as a stated contract: one
-//! spelling per value, shortest-form heads only. The renderer walks
-//! each item with the codec's own canonical head grammar
-//! ([`cbor::read_head`]) and shows the item's *complete* content —
-//! every integer exactly, every byte string as full hex, every text
-//! string escaped, every tag number, and structure in wire order. Under
-//! the determinism contract a complete value tree has exactly one
-//! encoding, so the rendering is injective on wire bytes: two different
-//! byte streams cannot render identically. Wherever the walk cannot
-//! vouch for that inversion — non-canonical heads, invalid UTF-8,
-//! embedded content that does not fill its byte string, or nesting past
-//! the renderer's depth bound (one budget spanning the whole walk:
-//! structural descent and embedded-byte-string unfolds draw it down
-//! together) — the subtree falls back to an explicit failure line above
-//! its exact bytes as hex, which is injective trivially. Byte counts on
-//! item and stream headers come from the transport capture, so totals
-//! stay exact.
+//! The harness holds every item to canonical form before rendering it:
+//! every head at its shortest width, definite lengths, well-formed
+//! simple values, no NaN, text free of control characters, and a
+//! re-encoding equal to the bytes, checked down through embedded CBOR
+//! to the printer's depth limit. On a canonical item, diagnostic
+//! notation with encoding indicators spells the value exactly, so two
+//! different canonical byte streams cannot render identically; any other
+//! item renders as an explicit failure line carrying the reason above
+//! its exact hex. The rendering is therefore injective on wire bytes. A
+//! NaN's sign and payload bits are what the notation cannot spell, which
+//! is why every NaN is rejected; none occurs on the wire, since the
+//! protocol emits no floats and the payload contract's `Eq` bound
+//! excludes float fields (see
+//! [choosing a payload type](crate#choosing-a-payload-type)); a
+//! hand-written `Eq` admitting NaN has declared its NaNs equal. A control
+//! item's or frame's byte count is the observed item's length and a
+//! stream's is the transport's; the totality witness
+//! ([`assert_items_account_for`]) holds the two accounts equal.
 //!
 //! # Where the bytes come from
 //!
-//! Capture enters through the public observation hook
-//! ([`crate::observe`]): the harness records each directed stream's
-//! items and hands them here as a [`HookCapture`]. The transport-level
-//! byte capture ([`LinkCapture`]) remains the totality oracle: the
-//! harness asserts, per directed stream, that the stream's on-wire open
-//! label followed by the concatenated observed items reproduces the
-//! transport bytes exactly ([`assert_items_account_for`],
-//! [`stream_label`]) — that assertion is what licenses a rendering of
-//! *items* as a pin of *wire bytes*. Structural violations of the
-//! capture itself (an item that is not one canonical CBOR item where
-//! the wire grammar requires one, a frame contradicting its stream, a
-//! label that does not parse) are panics: they mean the capture
-//! harness, not the peer, is broken. Application payload bytes are the
-//! application's own CBOR and only ever fall back explicitly.
+//! Capture enters through the observation hook ([`crate::observe`]):
+//! the harness records each directed stream's items as a
+//! [`HookCapture`]. The transport-level bytes ([`LinkCapture`]) are the
+//! totality oracle: per directed stream, the on-wire open label followed
+//! by the observed items must reproduce the transport bytes exactly
+//! ([`assert_items_account_for`], [`stream_label`]), which is what lets
+//! a rendering of items pin wire bytes. Structural violations of the
+//! capture itself (a control item whose opening head is not canonical,
+//! a frame whose opener is not the codec's or contradicts its stream, a
+//! label that does not parse) are panics: the harness, not the peer, is
+//! broken.
 
 use std::{collections::BTreeMap, fmt::Write as _};
 
-use crate::Version;
 use crate::observe::Role;
+use cbor_diag::{DataItem, IntegerWidth, Simple};
+
 use crate::tree::mirror::cbor::{
-    self, MAJOR_ARRAY, MAJOR_BSTR, MAJOR_MAP, MAJOR_TAG, MAJOR_TEXT, MAJOR_UINT, TAG_CBOR_SEQUENCE,
-    TAG_EMBEDDED_ITEM,
+    self, MAJOR_ARRAY, MAJOR_TAG, MAJOR_TEXT, MAJOR_UINT, TAG_CBOR_SEQUENCE, TAG_EMBEDDED_ITEM,
 };
 
-use super::{
-    Speaker, Stream,
-    signal::{Signal, WireSignal},
-};
+use super::{Speaker, Stream, signal::WireSignal};
 
 #[cfg(test)]
 mod tests;
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 1:
>
> T138: item bodies render through cbor-diag; the hand renderer (its node tree, parser, listing and tag arms, glosses, and depth budget) is gone. The module doc is restated for that design and, per T141, cut to what a maintainer here needs: it no longer restates cbor-diag's own unfold and depth behavior, which the tests below pin.

<!-- annotation -->
> **remote-capture-atlas-17** (T140), line 15:
>
> T140: the injectivity claim is delegated to cbor-diag's encoding indicators and stated here; no round-trip pin, generators, or inverse parser land. The sentence that this module adds nothing that could collapse the rendering is what the verbatim output below guarantees.

<!-- annotation -->
> **remote-capture-atlas-17** (T140), line 18:
>
> T140: NaN payload bits are documented as unrepresentable under the payload contract (the `Eq` bound excludes float fields; a hand-written `Eq` admitting NaN has declared its NaNs equal); the protocol itself emits no floats.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 30:
>
> The harness-grammar sentence is restated to what remains asserted (a control item's opening head, the frame opener, the frame-to-stream match, the labels); body canonicality is no longer a panic, since a non-shortest head renders with its encoding indicator. Prose pass: the paragraph's em-dash and restatements are gone.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 51:
>
> Imports reduced to what the framing still reads: the head grammar for labels, control-item shapes, and frame openers.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 13:
>
> Fresh-eyes repair (item 1): the injectivity argument was wrong as stated, since cbor-diag spells width indicators only for integers, negatives, tags, and floats, and its binary parser accepts the ill-formed two-byte simple form; equal-length collisions existed (`82 40 58 00` vs `82 58 00 40`, `82 f4 f8 14` vs `82 f8 14 f4`). The doc now states the design that makes the claim true: canonical form is checked first, notation is exact on canonical items, everything else is hex.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 23:
>
> Fresh-eyes repair (item 5): a NaN loses its sign bit as well as its payload; and with the re-encoding check such a float now falls back to hex rather than rendering as `NaN`. The payload-contract sentence stays, with the crate docs cited by rustdoc link instead of by string.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 28:
>
> Fresh-eyes repair (item 5): only the stream count is transport-sourced; a control item's or frame's count is the hook item's length, and their equality is the totality witness's claim, now said so.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 14:
>
> Fresh-eyes repair, round 2 (items 1, 3, 4): the doc claimed the re-encoding check rejected NaN bits and ill-formed simples; cbor-diag's float path is bit-exact and simple values 24..=31 re-encode verbatim, so neither was true. The canonical-form list now names what the walk actually holds: every head shortest, definite lengths, well-formed simple values, no NaN, no control characters, exact re-encoding.

<a id="hunk-10"></a>
### src/tree/mirror/streaming/remote/codec/capture.rs `@@ -157,8 +151,8 @@ pub fn assert_items_account_for(items: &[Vec<u8>], wire: &[u8]) {`

```diff
@@ -157,8 +151,8 @@ pub fn assert_items_account_for(items: &[Vec<u8>], wire: &[u8]) {
 /// Render both endpoints' hook captures without retaining cross-stream
 /// order.
 ///
-/// Data streams are keyed by their labeled stream index — exact items
-/// and order within each stream, stream groups sorted — discarding the
+/// Data streams are keyed by their labeled stream index (exact items
+/// and order within each stream, stream groups sorted), discarding the
 /// incidental order in which independent streams were opened.
 pub fn render_hook_capture(a: &HookCapture, b: &HookCapture) -> String {
     let mut rendered = String::new();
```

<!-- annotation -->
> **remote-capture-atlas-13** (T141), line 154:
>
> Nit swept in a file this lane edits: em-dashes replaced by parentheses; wording otherwise unchanged.

<a id="hunk-11"></a>
### src/tree/mirror/streaming/remote/codec/capture.rs `@@ -183,7 +177,7 @@ fn render_direction(label: &str, capture: &HookCapture, out: &mut String) {`

```diff
@@ -183,7 +177,7 @@ fn render_direction(label: &str, capture: &HookCapture, out: &mut String) {
             item.len()
         )
         .unwrap();
-        render_item(item, "  ", out);
+        render_item(item, out);
     }
 
     let mut streams = BTreeMap::new();
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 177:
>
> Call-site consequence of T140's verbatim output: control-item bodies no longer take an indent argument.

<a id="hunk-12"></a>
### src/tree/mirror/streaming/remote/codec/capture.rs `@@ -205,8 +199,7 @@ fn render_direction(label: &str, capture: &HookCapture, out: &mut String) {`

```diff
@@ -205,8 +199,7 @@ fn render_direction(label: &str, capture: &HookCapture, out: &mut String) {
         )
         .unwrap();
         for (index, item) in stream.items.iter().enumerate() {
-            writeln!(out, "  frame {index} ({} bytes)", item.len()).unwrap();
-            render_frame(speaker, wire_stream, item, out);
+            render_frame(speaker, wire_stream, index, item, out);
         }
     }
 }
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 199:
>
> The frame header line moved into render_frame (it now carries the decoded signal, so it needs the opener), and the loop passes the frame index through; the byte count on the header is unchanged.

<a id="hunk-13"></a>
### src/tree/mirror/streaming/remote/codec/capture.rs `@@ -239,21 +232,19 @@ fn control_item_name(item: &[u8]) -> &'static str {`

```diff
@@ -239,21 +232,19 @@ fn control_item_name(item: &[u8]) -> &'static str {
     }
 }
 
-/// Render one data frame.
+/// Render one data frame: a header line with the frame's index, exact
+/// byte count, and the signal its opener names, then the frame item in
+/// diagnostic notation.
 ///
-/// The codec's frame grammar (the array head and the opener's stream and
-/// state items) is held to panics — a violation means the capture is
-/// broken — while the body renders through the generic walk, falling
-/// back explicitly where it cannot vouch for inversion. The opener's two
-/// items render one per line, the stream index annotated `stream` and
-/// the state code annotated with the signal it names.
-fn render_frame(speaker: Speaker, stream: Stream, item: &[u8], out: &mut String) {
+/// The frame grammar (the array head, the opener's stream and state
+/// items) is held to panics: a violation means the capture is broken.
+fn render_frame(speaker: Speaker, stream: Stream, index: usize, item: &[u8], out: &mut String) {
     let mut probe = item;
     let head = cbor::read_head(&mut probe).expect("captured frame head is canonical");
     assert_eq!(head.major, MAJOR_ARRAY, "captured frame is an array");
-    let index = cbor::read_head(&mut probe).expect("captured stream item is canonical");
+    let stream_item = cbor::read_head(&mut probe).expect("captured stream item is canonical");
     assert_eq!(
-        index.major, MAJOR_UINT,
+        stream_item.major, MAJOR_UINT,
         "captured stream item is an unsigned int"
     );
     let state = cbor::read_head(&mut probe).expect("captured state item is canonical");
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 235:
>
> Judgment call on where the protocol-phase label lives for a frame: the header line now carries the decoded signal (`frame N (B bytes) / Supply(End) /`), since the opener items are inside the one diagnostic item and cannot be annotated individually without a hand walk. The frame-grammar assertions stay.

<a id="hunk-14"></a>
### src/tree/mirror/streaming/remote/codec/capture.rs `@@ -261,471 +252,171 @@ fn render_frame(speaker: Speaker, stream: Stream, item: &[u8], out: &mut String)`

```diff
@@ -261,471 +252,171 @@ fn render_frame(speaker: Speaker, stream: Stream, item: &[u8], out: &mut String)
         state.major, MAJOR_UINT,
         "captured state item is an unsigned int"
     );
-    let (framed, semantic) = WireSignal::decode(speaker, index.value, state.value)
+    let (framed, semantic) = WireSignal::decode(speaker, stream_item.value, state.value)
         .expect("captured frame opener is valid")
         .into_parts();
     assert_eq!(framed, stream, "captured frame contradicts its label");
 
-    writeln!(out, "    [").unwrap();
-    writeln!(out, "      {} / stream /", framed.index()).unwrap();
-    writeln!(out, "      {} / {semantic:?} /", semantic.state()).unwrap();
-    let naming = match semantic {
-        Signal::Query(_) => Naming::Listing,
-        Signal::Supply(_) => Naming::Run,
-        _ => Naming::Plain,
-    };
-    let mut rest = probe;
-    while !rest.is_empty() {
-        let remaining = rest;
-        match parse_node(&mut rest, 0) {
-            Ok(node) => render_node(&node, naming, "      ", 0, out),
-            Err(reason) => {
-                fallback(remaining, &reason, "      ", out);
-                rest = &[];
-            }
-        }
-    }
-    writeln!(out, "    ]").unwrap();
+    writeln!(
+        out,
+        "  frame {index} ({} bytes) / {semantic:?} /",
+        item.len()
+    )
+    .unwrap();
+    render_item(item, out);
 }
 
-/// Render one whole captured item (a control item) as a value tree.
-fn render_item(item: &[u8], indent: &str, out: &mut String) {
-    let mut rest = item;
-    match parse_node(&mut rest, 0) {
-        Ok(node) if rest.is_empty() => render_node(&node, Naming::Plain, indent, 0, out),
-        Ok(_) => panic!("captured control item carries trailing bytes"),
-        Err(reason) => panic!("captured control item is not canonical CBOR: {reason}"),
+/// Render one captured item under its header: cbor-diag's pretty
+/// diagnostic notation, verbatim, when the item is canonical, or the
+/// explicit fallback with the reason it is not.
+fn render_item(item: &[u8], out: &mut String) {
+    let parsed = match cbor_diag::parse_bytes(item) {
+        Ok(parsed) => parsed,
+        Err(error) => return fallback(item, &error.to_string(), out),
+    };
+    if parsed.to_bytes() != item {
+        return fallback(item, "re-encodes differently", out);
+    }
+    match canonical(&parsed, cbor_diag::DEFAULT_DEPTH_LIMIT) {
+        Ok(()) => writeln!(out, "{}", parsed.to_diag_pretty()).unwrap(),
+        Err(reason) => fallback(item, &reason, out),
     }
 }
 
-/// The naming context a subtree renders under.
-///
-/// `Listing` annotates a map as a `{radix => digest}` listing (hex
-/// radix keys, `/ digest /` value comments, an order check in the
-/// block comment); `Run` names a supply body's embedded sequence a
-/// *supply run* and `Record` names the run's items *records*, so a
-/// re-accept diff speaks the protocol's own vocabulary.
-#[derive(Clone, Copy, PartialEq, Eq)]
-enum Naming {
-    Plain,
-    Listing,
-    Run,
-    Record,
-}
-
-/// One parsed CBOR value, canonical-head-checked, structure preserved
-/// in wire order.
-#[derive(Debug)]
-enum Node {
-    Uint(u64),
-    /// A major-1 negative integer holding `n`, meaning `-(n + 1)`.
-    Nint(u64),
-    Bytes(Vec<u8>),
-    Text(String),
-    Array(Vec<Node>),
-    Map(Vec<(Node, Node)>),
-    Tag(u64, Box<Node>),
-    /// A major-7 simple value.
-    Simple(u8),
-    /// A major-7 float: its width byte (25, 26, or 27) and raw bits.
-    Float(u8, u64),
-}
-
-/// Nesting past this bound falls back to exact hex: the walk never
-/// recurses on unbounded input-controlled depth.
+/// Check that a parsed item is canonical wherever the notation would
+/// not show a difference.
 ///
-/// One budget spans the whole walk — an embedded byte string's content
-/// re-parses at the depth already consumed above it, never at a fresh
-/// zero — so structural descent and embedded unfolds are bounded
-/// together.
-const MAX_DEPTH: usize = 64;
-
-/// Parse one canonical item off the front of `input`.
+/// Every head must be at its shortest width (only string and container
+/// heads need it for injectivity, since the printer spells the others
+/// with a width indicator), every container definite, every simple
+/// value well-formed, no float a NaN, text free of control characters,
+/// and every embedded item (tags 24 and 63) canonical and re-encoding
+/// to its bytes.
 ///
-/// Head canonicality comes from the codec's own grammar; major-7 items
-/// are handled here because float widths are semantic, not
-/// shortest-form arithmetic. Any violation is a typed reason for the
-/// caller's explicit fallback.
-fn parse_node(input: &mut &[u8], depth: usize) -> Result<Node, String> {
-    if depth >= MAX_DEPTH {
-        return Err(format!("nested deeper than {MAX_DEPTH}"));
-    }
-    let Some(&initial) = input.first() else {
-        return Err("input ends before an item".into());
+/// The walk mirrors the printer's depth budget: `remaining` counts down
+/// one per level, embedded content is parsed with what is left, and a
+/// level past the budget is accepted unchecked because the printer shows
+/// it as hex. Recursion is therefore bounded by the parser's depth limit.
+fn canonical(item: &DataItem, remaining: usize) -> Result<(), String> {
+    let Some(remaining) = remaining.checked_sub(1) else {
+        return Ok(());
     };
-    if initial >> 5 == 7 {
-        return parse_major_seven(input);
-    }
-    let head = cbor::read_head(input).map_err(|e| e.to_string())?;
-    match head.major {
-        MAJOR_UINT => Ok(Node::Uint(head.value)),
-        1 => Ok(Node::Nint(head.value)),
-        MAJOR_BSTR => {
-            let bytes = take(input, head.value)?;
-            Ok(Node::Bytes(bytes.to_vec()))
+    match item {
+        DataItem::Integer { value, bitwidth } | DataItem::Negative { value, bitwidth } => {
+            shortest(*value, *bitwidth, "integer")
         }
-        MAJOR_TEXT => {
-            let bytes = take(input, head.value)?;
-            let text = std::str::from_utf8(bytes).map_err(|_| "invalid UTF-8".to_string())?;
-            Ok(Node::Text(text.to_string()))
+        DataItem::ByteString(string) => {
+            shortest(string.data.len() as u64, string.bitwidth, "byte string")
         }
-        MAJOR_ARRAY => {
-            let mut items = Vec::new();
-            for _ in 0..head.value {
-                items.push(parse_node(input, depth + 1)?);
+        DataItem::TextString(string) => {
+            shortest(string.data.len() as u64, string.bitwidth, "text string")?;
+            if string.data.chars().any(char::is_control) {
+                return Err("control character in a text string".into());
             }
-            Ok(Node::Array(items))
+            Ok(())
         }
-        MAJOR_MAP => {
-            let mut entries = Vec::new();
-            for _ in 0..head.value {
-                let key = parse_node(input, depth + 1)?;
-                let value = parse_node(input, depth + 1)?;
-                entries.push((key, value));
-            }
-            Ok(Node::Map(entries))
-        }
-        MAJOR_TAG => Ok(Node::Tag(
-            head.value,
-            Box::new(parse_node(input, depth + 1)?),
-        )),
-        _ => unreachable!("majors 0 through 6 handled; 7 split off above"),
-    }
-}
-
-/// Parse one major-7 item: simple values inline, one-byte simples with
-/// their canonical floor, floats by width with exact bits.
-fn parse_major_seven(input: &mut &[u8]) -> Result<Node, String> {
-    let (&initial, rest) = input.split_first().expect("caller peeked the initial byte");
-    let info = initial & 0x1f;
-    match info {
-        0..=23 => {
-            *input = rest;
-            Ok(Node::Simple(info))
-        }
-        24 => {
-            let (&value, rest) = rest
-                .split_first()
-                .ok_or("input ends inside a simple value")?;
-            if value < 32 {
-                return Err("one-byte simple value below 32 is not canonical".into());
-            }
-            *input = rest;
-            Ok(Node::Simple(value))
+        DataItem::IndefiniteByteString(_) | DataItem::IndefiniteTextString(_) => {
+            Err("indefinite-length string".into())
         }
-        25..=27 => {
-            let width = 1usize << (info - 24);
-            if rest.len() < width {
-                return Err("input ends inside a float".into());
-            }
-            let (bytes, rest) = rest.split_at(width);
-            let mut bits = 0u64;
-            for &byte in bytes {
-                bits = bits << 8 | u64::from(byte);
-            }
-            *input = rest;
-            Ok(Node::Float(info, bits))
+        DataItem::Array { data, bitwidth } => {
+            definite(*bitwidth, data.len(), "array")?;
+            data.iter().try_for_each(|item| canonical(item, remaining))
         }
-        28..=30 => Err("reserved additional-information value".into()),
-        _ => Err("indefinite-length CBOR is not canonical".into()),
-    }
-}
-
-/// Split `len` payload bytes off `input`.
-fn take<'a>(input: &mut &'a [u8], len: u64) -> Result<&'a [u8], String> {
-    let len = usize::try_from(len).map_err(|_| "length exceeds memory".to_string())?;
-    if input.len() < len {
-        return Err("input ends inside a string".into());
-    }
-    let (bytes, rest) = input.split_at(len);
-    *input = rest;
-    Ok(bytes)
-}
-
-/// Render one node at `indent`, one line per scalar or bracket.
-///
-/// `depth` is the walk's one nesting budget, shared with
-/// [`parse_node`]: it counts structural levels descended since the
-/// walk's entry point, and an embedded byte string's content re-parses
-/// at the depth already consumed above it, so structural descent and
-/// embedded unfolds are bounded by [`MAX_DEPTH`] together. Invariant
-/// every `render_*` call site preserves: the `depth` passed is no
-/// greater than the depth its node was parsed at — so a node in hand
-/// always fits the remaining budget, and only [`parse_node`] need
-/// check the bound.
-fn render_node(node: &Node, naming: Naming, indent: &str, depth: usize, out: &mut String) {
-    match node {
-        Node::Map(entries) if naming == Naming::Listing => {
-            render_listing(entries, indent, depth, out);
+        DataItem::Map { data, bitwidth } => {
+            definite(*bitwidth, data.len(), "map")?;
+            data.iter().try_for_each(|(key, value)| {
+                canonical(key, remaining)?;
+                canonical(value, remaining)
+            })
         }
-        Node::Map(entries) => {
-            writeln!(out, "{indent}{{").unwrap();
-            for (key, value) in entries {
-                // The one context-sensitive key: a map value under the
-                // text key "listing" is a `{radix => digest}` listing.
-                let value_naming = match key {
-                    Node::Text(text) if text == "listing" => Naming::Listing,
-                    _ => Naming::Plain,
-                };
-                let key = scalar(key).unwrap_or_else(|| "…".into());
-                match scalar(value) {
-                    Some(value) => writeln!(out, "{indent}  {key} => {value}").unwrap(),
-                    None => {
-                        writeln!(out, "{indent}  {key} =>").unwrap();
-                        let deeper = format!("{indent}    ");
-                        render_node(value, value_naming, &deeper, depth + 1, out);
-                    }
+        DataItem::Tag {
+            tag,
+            bitwidth,
+            value,
+        } => {
+            shortest(tag.0, *bitwidth, "tag")?;
+            if let DataItem::ByteString(string) = &**value {
+                match tag.0 {
+                    TAG_EMBEDDED_ITEM => embedded_item(&string.data, remaining)?,
+                    TAG_CBOR_SEQUENCE => embedded_sequence(&string.data, remaining)?,
+                    _ => {}
                 }
             }
-            writeln!(out, "{indent}}}").unwrap();
+            canonical(value, remaining)
         }
-        Node::Array(items) => {
-            writeln!(out, "{indent}[").unwrap();
-            let deeper = format!("{indent}  ");
-            for item in items {
-                render_node(item, Naming::Plain, &deeper, depth + 1, out);
-            }
-            writeln!(out, "{indent}]").unwrap();
-        }
-        Node::Tag(number, content) => render_tag(*number, content, naming, indent, depth + 1, out),
-        scalar_node => {
-            let text = scalar(scalar_node).expect("non-container nodes render inline");
-            writeln!(out, "{indent}{text}").unwrap();
+        // The parser and encoder are bit-exact on floats, so a NaN's
+        // sign and payload bits survive re-encoding while the printer
+        // writes a bare `NaN`; every NaN is rejected outright.
+        DataItem::Float { value, .. } if value.is_nan() => {
+            Err("NaN, whose sign and payload bits the notation cannot spell".into())
         }
+        DataItem::Float { .. } => Ok(()),
+        // Simple values 24 through 31 have no well-formed spelling; the
+        // parser accepts their two-byte form and re-encodes it verbatim.
+        DataItem::Simple(Simple(24..=31)) => Err("ill-formed simple value".into()),
+        DataItem::Simple(_) => Ok(()),
     }
 }
 
-/// Render a `{radix => digest}` listing map: hex radix keys, digest
-/// annotations, and an explicit order verdict when the wire's
-/// strictly-ascending canonical form is violated.
-fn render_listing(entries: &[(Node, Node)], indent: &str, depth: usize, out: &mut String) {
-    let ascending = entries
-        .windows(2)
-        .all(|pair| match (&pair[0].0, &pair[1].0) {
-            (Node::Uint(a), Node::Uint(b)) => a < b,
-            _ => false,
-        })
-        || entries.len() < 2;
-    let order = if ascending {
-        ""
-    } else {
-        ", NON-CANONICAL ORDER"
-    };
-    writeln!(
-        out,
-        "{indent}{{ / listing: {} child(ren){order} /",
-        entries.len()
-    )
-    .unwrap();
-    for (key, value) in entries {
-        let key = match key {
-            Node::Uint(radix) => format!("0x{radix:x}"),
-            other => scalar(other).unwrap_or_else(|| "…".into()),
-        };
-        match value {
-            Node::Bytes(bytes) => {
-                writeln!(
-                    out,
-                    "{indent}  {key} => h'{}' / digest /",
-                    hex::encode(bytes)
-                )
-                .unwrap();
-            }
-            other => match scalar(other) {
-                Some(text) => writeln!(out, "{indent}  {key} => {text}").unwrap(),
-                None => {
-                    writeln!(out, "{indent}  {key} =>").unwrap();
-                    let deeper = format!("{indent}    ");
-                    render_node(other, Naming::Plain, &deeper, depth + 1, out);
-                }
-            },
+/// The embedded item the printer unfolds under tag 24 must be canonical
+/// and fill its byte string exactly; content that does not parse stays a
+/// byte string, which the printer shows as hex.
+fn embedded_item(data: &[u8], remaining: usize) -> Result<(), String> {
+    match cbor_diag::parse_bytes_with_limit(data, remaining) {
+        Ok(item) if item.to_bytes() != data => Err("embedded item re-encodes differently".into()),
+        Ok(item) => {
+            canonical(&item, remaining).map_err(|why| format!("in an embedded item, {why}"))
         }
+        Err(_) => Ok(()),
     }
-    writeln!(out, "{indent}}}").unwrap();
 }
 
-/// Render one tagged node, unfolding embedded byte strings and
-/// annotating the tags the protocol names.
-///
-/// `depth` is the tag's *content* depth — the caller already counted
-/// the tag's own structural level — and passes through unchanged.
-fn render_tag(
-    number: u64,
-    content: &Node,
-    naming: Naming,
-    indent: &str,
-    depth: usize,
-    out: &mut String,
-) {
-    match (number, content) {
-        (TAG_CBOR_SEQUENCE, Node::Bytes(bytes)) => {
-            let (name, inner) = match naming {
-                Naming::Run => ("supply run", Naming::Record),
-                Naming::Record => ("record", Naming::Plain),
-                _ => ("embedded sequence", Naming::Plain),
-            };
-            render_embedded_as(number, name, inner, bytes, indent, depth, out);
-        }
-        (TAG_EMBEDDED_ITEM, Node::Bytes(bytes)) => {
-            render_embedded(number, "embedded item", bytes, indent, depth, out);
-        }
-        (crate::tags::VERSION_TAG, Node::Bytes(bytes)) => {
-            let meaning = match Version::decode(&bytes[..]) {
-                // The rendering is the version's whole ITC event tree in
-                // paper notation, never a scalar: a flat tree renders as
-                // its single uniform height (e.g. `3`), a forked one as
-                // the nested `(n, e1, e2)` form.
-                Ok(version) => format!("causal version, event tree: {version}"),
-                Err(e) => format!("causal version undecodable: {e}"),
-            };
-            writeln!(
-                out,
-                "{indent}{number}(h'{}') / {meaning} /",
-                hex::encode(bytes)
-            )
-            .unwrap();
-        }
-        (crate::tags::PARTY_TAG, Node::Bytes(bytes)) => {
-            writeln!(out, "{indent}{number}(h'{}') / party /", hex::encode(bytes)).unwrap();
-        }
-        (crate::tags::CLOCK_TAG, Node::Bytes(bytes)) => {
-            writeln!(out, "{indent}{number}(h'{}') / clock /", hex::encode(bytes)).unwrap();
-        }
-        (cbor::TAG_SELF_DESCRIBED, _) => {
-            writeln!(out, "{indent}{number}( / self-described CBOR /").unwrap();
-            let deeper = format!("{indent}  ");
-            render_node(content, naming, &deeper, depth, out);
-            writeln!(out, "{indent})").unwrap();
-        }
-        (_, scalar_content) if scalar(scalar_content).is_some() => {
-            let text = scalar(scalar_content).expect("checked by the guard");
-            writeln!(out, "{indent}{number}({text})").unwrap();
-        }
-        _ => {
-            writeln!(out, "{indent}{number}(").unwrap();
-            let deeper = format!("{indent}  ");
-            render_node(content, naming, &deeper, depth, out);
-            writeln!(out, "{indent})").unwrap();
+/// Every item the printer unfolds from a tag-63 sequence must be
+/// canonical and re-encode to its span; the unparsed remainder, if any,
+/// stays a byte string.
+fn embedded_sequence(data: &[u8], remaining: usize) -> Result<(), String> {
+    let mut rest = data;
+    while let Ok(Some((item, len))) = cbor_diag::parse_bytes_partial_with_limit(rest, remaining) {
+        if item.to_bytes() != rest[..len] {
+            return Err("an embedded sequence item re-encodes differently".into());
         }
+        canonical(&item, remaining).map_err(|why| format!("in an embedded sequence, {why}"))?;
+        rest = &rest[len..];
     }
+    Ok(())
 }
 
-/// Unfold one embedded byte string (tag 24 or 63) as its parsed
-/// item sequence, falling back to exact hex when the content is not
-/// wholly canonical CBOR or when the walk's depth budget is spent.
-fn render_embedded(
-    number: u64,
-    name: &str,
-    bytes: &[u8],
-    indent: &str,
-    depth: usize,
-    out: &mut String,
-) {
-    render_embedded_as(number, name, Naming::Plain, bytes, indent, depth, out);
-}
-
-/// [`render_embedded`], with the naming context the unfolded items
-/// render under (a supply run's items are records).
-///
-/// The content re-parses at `depth` — the budget already consumed
-/// above this byte string — so a chain of embedded byte strings draws
-/// down the same [`MAX_DEPTH`] bound as structural nesting, and spends
-/// it here as the too-deep fallback.
-fn render_embedded_as(
-    number: u64,
-    name: &str,
-    inner: Naming,
-    bytes: &[u8],
-    indent: &str,
-    depth: usize,
-    out: &mut String,
-) {
-    let mut items = Vec::new();
-    let mut rest = bytes;
-    let mut failure = None;
-    while !rest.is_empty() {
-        match parse_node(&mut rest, depth) {
-            Ok(node) => items.push(node),
-            Err(reason) => {
-                failure = Some(reason);
-                break;
-            }
-        }
-    }
-    if let Some(reason) = failure {
-        writeln!(out, "{indent}{number}( / {name}, {} bytes /", bytes.len()).unwrap();
-        fallback(bytes, &reason, &format!("{indent}  "), out);
-        writeln!(out, "{indent})").unwrap();
-        return;
-    }
-    if number == TAG_EMBEDDED_ITEM && items.len() != 1 {
-        writeln!(out, "{indent}{number}( / {name}, {} bytes /", bytes.len()).unwrap();
-        fallback(
-            bytes,
-            &format!("embedded item holds {} items", items.len()),
-            &format!("{indent}  "),
-            out,
-        );
-        writeln!(out, "{indent})").unwrap();
-        return;
-    }
-    let count = match (number, inner) {
-        (TAG_CBOR_SEQUENCE, Naming::Record) => format!(", {} record(s)", items.len()),
-        (TAG_CBOR_SEQUENCE, _) => format!(", {} item(s)", items.len()),
-        _ => String::new(),
+/// The recorded width of a head must be the shortest that holds `value`.
+fn shortest(value: u64, width: IntegerWidth, what: &str) -> Result<(), String> {
+    let expected = match value {
+        0..=23 => IntegerWidth::Zero,
+        24..=0xff => IntegerWidth::Eight,
+        0x100..=0xffff => IntegerWidth::Sixteen,
+        0x1_0000..=0xffff_ffff => IntegerWidth::ThirtyTwo,
+        _ => IntegerWidth::SixtyFour,
     };
-    writeln!(
-        out,
-        "{indent}{number}(<< / {name}{count}, {} bytes /",
-        bytes.len()
-    )
-    .unwrap();
-    let deeper = format!("{indent}  ");
-    for item in &items {
-        render_node(item, inner, &deeper, depth, out);
+    if width == expected {
+        Ok(())
+    } else {
+        Err(format!("non-shortest {what} head"))
     }
-    writeln!(out, "{indent}>>)").unwrap();
 }
 
-/// Render one scalar node inline, or `None` for containers.
-fn scalar(node: &Node) -> Option<String> {
-    Some(match node {
-        Node::Uint(value) => format!("{value}"),
-        Node::Nint(value) => format!("-{}", u128::from(*value) + 1),
-        Node::Bytes(bytes) => format!("h'{}'", hex::encode(bytes)),
-        Node::Text(text) => format!("{text:?}"),
-        Node::Simple(20) => "false".into(),
-        Node::Simple(21) => "true".into(),
-        Node::Simple(22) => "null".into(),
-        Node::Simple(23) => "undefined".into(),
-        Node::Simple(value) => format!("simple({value})"),
-        Node::Float(25, bits) => format!("float16'{bits:04x}'"),
-        Node::Float(26, bits) => format!("float32'{bits:08x}'"),
-        Node::Float(_, bits) => format!("float64'{bits:016x}'"),
-        Node::Array(_) | Node::Map(_) => return None,
-        // Tags the protocol names always render through the block path,
-        // so their annotations cannot be skipped by an inline rendering.
-        Node::Tag(
-            TAG_CBOR_SEQUENCE
-            | TAG_EMBEDDED_ITEM
-            | cbor::TAG_SELF_DESCRIBED
-            | crate::tags::PARTY_TAG
-            | crate::tags::VERSION_TAG
-            | crate::tags::CLOCK_TAG,
-            _,
-        ) => return None,
-        Node::Tag(number, content) => format!("{number}({})", scalar(content)?),
-    })
+/// A container must be definite-length, with its count at the shortest
+/// width.
+fn definite(width: Option<IntegerWidth>, len: usize, what: &str) -> Result<(), String> {
+    match width {
+        Some(width) => shortest(len as u64, width, what),
+        None => Err(format!("indefinite-length {what}")),
+    }
 }
 
-/// Render an explicit walk failure above the exact bytes it convicts:
-/// the fallback that keeps the rendering injective where the generic
-/// walk cannot vouch for inversion.
-fn fallback(bytes: &[u8], reason: &str, indent: &str, out: &mut String) {
+/// Render a failure and its reason above the exact bytes.
+fn fallback(bytes: &[u8], reason: &str, out: &mut String) {
     writeln!(
         out,
-        "{indent}!! not rendered as CBOR ({reason}); the exact bytes stand here:"
+        "!! not rendered as CBOR ({reason}); the exact bytes stand here:"
     )
     .unwrap();
-    writeln!(out, "{indent}h'{}'", hex::encode(bytes)).unwrap();
+    writeln!(out, "h'{}'", hex::encode(bytes)).unwrap();
 }
```

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 272:
>
> T140: the body renderer emits cbor-diag's pretty output verbatim under the header, with no re-indenting or re-splitting; parse failure (malformed, trailing bytes, too deep) goes to the explicit fallback with the parser's error.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 415:
>
> The fallback keeps the two-line form (failure line with the reason, then the exact hex) so a fallback is unmistakable in a diff.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 277:
>
> Fresh-eyes repair (item 1): re-encoding equal to the bytes, checked before the walk; this is what rejects the ill-formed simple (re-encoding uses the one-byte form) and any NaN bits beyond the canonical NaN.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 300:
>
> Fresh-eyes repair (item 1): the canonical walk over the parsed item, holding every head the printer spells without a width to its shortest width, containers to definite lengths, text to no control characters (item 2), and embedded content (tags 24 and 63) to the same rules plus exact re-encoding, because the printer unfolds it and a wide head inside it would collide the same way. The walk mirrors the printer's depth budget so recursion is bounded by the parser's limit; a level past the budget is accepted because the printer shows it as hex. Floats and simple values are exempt: their indicator or number is spelled, and the re-encoding check covers the rest.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 364:
>
> Embedded content that does not parse is left alone: the printer shows it as a byte string, which is hex and injective. Content that parses must re-encode exactly (a shorter ill-formed spelling inside would collide with a longer well-formed one at equal outer length) and be canonical.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 278:
>
> Fresh-eyes repair, round 2 (item 1): the reason string no longer names NaN bits or simple values, which this check does not catch; it names only what it detects.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 289:
>
> Fresh-eyes repair, round 2 (item 4): one criterion stated once, every head shortest, with the note that only string and container heads need it for injectivity since the printer spells the other widths.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 350:
>
> Fresh-eyes repair, round 2 (item 1): every NaN is rejected by value. The parser reads floats by from_bits and the encoder writes to_bits, so a NaN's sign and payload survive re-encoding while the printer writes a bare `NaN`; `fb 7ff8…` and `fb fff8…` both rendered `NaN_3`. Rejecting by value also drops any dependence on half's f16/f32 conversion path.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 356:
>
> Fresh-eyes repair, round 2 (item 3): simple values 24 through 31 have no well-formed spelling, and their two-byte form re-encodes verbatim, so the walk rejects them by value; no collision resulted, but the doc's claim now holds.

<a id="hunk-15"></a>
### src/tree/mirror/streaming/remote/codec/capture/tests.rs `@@ -1,201 +1,179 @@`

```diff
@@ -1,201 +1,179 @@
-//! The reflection renderer's pins.
+//! The capture renderer's pins.
 //!
-//! Three commitments: the rendered value tree localizes the semantic
-//! field a snapshot re-accept moved to exactly one rendered line
-//! carrying the exact value (its surrounding vocabulary is the wire
-//! snapshots' to pin), bytes the walk cannot vouch for render as
-//! an explicit failure above their exact hex (never as a silently pretty
-//! tree, never as silent omission), and the totality witness
-//! ([`assert_items_account_for`]) refuses any gap between observed items
-//! and wire bytes.
+//! The framing the harness owns (item index, exact byte count,
+//! protocol-phase label) is stated on each header line; every item body
+//! is cbor-diag's diagnostic notation, verbatim, when the item is
+//! canonical, and otherwise an explicit failure above its exact hex, so
+//! two different byte strings never share a rendering and no body line
+//! begins with `frame `; and the totality witness ([`assert_items_account_for`]) refuses
+//! any gap between observed items and wire bytes.
 
 use super::*;
 
+use crate::Version;
 use crate::message::{Message, PayloadDepthLimit};
-use crate::tree::typed::Hash;
-use crate::tree::typed::hash::MERKLE_HASH_LEN;
+use crate::tree::mirror::cbor::MAJOR_BSTR;
 
 use super::super::encode::encode;
-use super::super::frame::{Frame, LeafRun, Reaction, write_listing};
+use super::super::frame::{Frame, LeafRun, Reaction};
 use super::super::signal::Flow;
 
-/// Render one embedded run (the supply body's tag-63 content) to lines.
-fn run_lines(run: &LeafRun) -> Vec<String> {
-    let mut out = String::new();
-    render_embedded(
-        TAG_CBOR_SEQUENCE,
-        "embedded sequence",
-        run.as_bytes(),
-        "",
-        0,
-        &mut out,
-    );
-    out.lines().map(str::to_string).collect()
+/// Encode one supply frame carrying `payload` as a single record on
+/// stream 0, the harness-reachable shape a captured frame arrives in.
+fn supply_frame(payload: &Message) -> (Stream, Vec<u8>) {
+    let mut run = LeafRun::new();
+    run.push(&Version::new(), payload)
+        .expect("one record fits a fresh run");
+    let stream = Stream::new(0).expect("stream 0 names a stream");
+    let frame = (stream, Frame::Reaction(Reaction::Supply(run), Flow::End));
+    let mut bytes = Vec::new();
+    encode(Speaker::Initiator, &frame, &mut bytes).expect("a supply frame encodes");
+    (stream, bytes)
 }
 
-/// Two supply runs differing only in one record's version render line
-/// sets that differ in exactly one line, and that line carries the
-/// version's exact rendering: the field-level account an insta
-/// re-accept diff shows.
+/// A frame renders under a header line naming its index, its exact byte
+/// count, and the signal its opener decodes to, with the whole frame as
+/// one diagnostic-notation item beneath.
 ///
-/// Containment, not equality: the record's version-addressed hash
-/// moves on the same line. The annotation's surrounding vocabulary is
-/// deliberately not asserted here; the wire snapshots pin it, and a
-/// reviewer judges its changes at re-accept.
+/// The supply run unfolds through its embedded-sequence tag down to the
+/// record's payload.
 #[test]
-fn supply_reflection_localizes_the_field_that_moved() {
-    let mut party = before::Party::seed();
-    let other = party.fork();
-    let mut low = Version::new();
-    low.tick(&party);
-    low.tick(&other);
-    low.tick(&other);
-    let mut high = low.clone();
-    high.tick(&other);
-
-    let render = |version: &Version| {
-        let mut run = LeafRun::new();
-        run.push(version, &Message::new(7_u64))
-            .expect("one small record fits any run");
-        run_lines(&run)
-    };
-    let a = render(&low);
-    let b = render(&high);
-    assert_eq!(a.len(), b.len(), "one field moved, no line appeared");
-    let diffs: Vec<_> = a.iter().zip(&b).filter(|(a, b)| a != b).collect();
-    assert_eq!(diffs.len(), 1, "exactly one rendered line moved: {diffs:?}");
-    let (a_line, b_line) = diffs[0];
-    // The unbalanced ticks across the fork keep the event tree
-    // non-flat, so its rendering carries punctuation neither hex nor a
-    // tag digit can spell: containment cannot match vacuously inside
-    // the line's other tokens.
-    let low_text = low.to_string();
-    let high_text = high.to_string();
-    assert!(
-        low_text.contains('('),
-        "the fixture is non-flat: {low_text}"
-    );
-    assert!(
-        a_line.contains(&low_text),
-        "the moved line carries the exact version rendering: {a_line}"
+fn frames_render_their_label_and_diagnostic_body() {
+    let (stream, bytes) = supply_frame(&Message::new(7_u64));
+    let mut out = String::new();
+    render_frame(Speaker::Initiator, stream, 3, &bytes, &mut out);
+    let header = out.lines().next().unwrap_or_default();
+    assert_eq!(
+        header,
+        format!("  frame 3 ({} bytes) / Supply(End) /", bytes.len()),
+        "{out}"
     );
     assert!(
-        b_line.contains(&high_text),
-        "the moved line carries the exact version rendering: {b_line}"
+        out.contains("(h'e0'), 7>>"),
+        "the run unfolds to the record's version atom and payload: {out}"
     );
-    // The identical payload renders identically as its own line: the
-    // small u64 is a bare CBOR int, visible directly.
-    assert!(a.iter().any(|line| line.trim() == "7"), "{a:?}");
 }
 
-/// Embedded content the walk cannot vouch for renders as an explicit
-/// failure line above the exact bytes, never as silence or a partial
-/// tree presented as whole.
+/// Bytes that are not one parseable CBOR item render as an explicit
+/// failure line carrying the parse error above the exact bytes.
+///
+/// The specimen is an array head promising more elements than follow.
 #[test]
-fn undecodable_embedded_content_falls_back_explicitly_to_hex() {
-    // 0xf8 0x05: a one-byte simple value below 32 is not canonical.
-    let garbage = [0xf8, 0x05];
+fn unparseable_items_fall_back_explicitly_to_hex() {
+    let truncated = [0x82, 0x01];
     let mut out = String::new();
-    render_embedded(
-        TAG_CBOR_SEQUENCE,
-        "embedded sequence",
-        &garbage,
-        "",
-        0,
-        &mut out,
-    );
+    render_item(&truncated, &mut out);
     assert!(
-        out.contains("!! not rendered as CBOR"),
+        out.starts_with("!! not rendered as CBOR ("),
         "the failure is explicit: {out}"
     );
     assert!(
-        out.contains(&format!("h'{}'", hex::encode(garbage))),
+        out.contains(&format!("h'{}'", hex::encode(truncated))),
         "the exact bytes stand: {out}"
     );
 }
 
-/// A tag-24 embedded item holding anything but exactly one item falls
-/// back explicitly: an embedded *item* is one item by definition.
-#[test]
-fn embedded_item_with_two_items_falls_back() {
-    let mut bytes = Vec::new();
-    cbor::write_head(&mut bytes, MAJOR_UINT, 1);
-    cbor::write_head(&mut bytes, MAJOR_UINT, 2);
+/// Render `bytes` as one item, returning the rendering's lines.
+fn rendered(bytes: &[u8]) -> Vec<String> {
     let mut out = String::new();
-    render_embedded(TAG_EMBEDDED_ITEM, "embedded item", &bytes, "", 0, &mut out);
-    assert!(out.contains("holds 2 items"), "{out}");
-    assert!(out.contains("!! not rendered as CBOR"), "{out}");
+    render_item(bytes, &mut out);
+    out.lines().map(str::to_string).collect()
 }
 
-/// A listing map renders each child as a hex radix and an annotated
-/// digest, and the block comment carries the child count.
+/// A length head wider than its value needs falls back to hex, so two
+/// arrays differing only in which empty string has the wide head render
+/// distinctly.
+///
+/// The notation spells a byte string's length without a width
+/// indicator, which is why the canonical `[h'', h'']` alone renders in
+/// notation. The same holds inside an embedded item.
 #[test]
-fn listing_renders_children_with_digest_annotations() {
-    let mut bytes = Vec::new();
-    write_listing(
-        &mut bytes,
-        &[
-            (0x3_u8, Hash([0xab; MERKLE_HASH_LEN])),
-            (0xc_u8, Hash([0x01; MERKLE_HASH_LEN])),
-        ],
-    );
-    let mut input = bytes.as_slice();
-    let node = parse_node(&mut input, 0).expect("the codec writes canonical listings");
-    let mut out = String::new();
-    render_node(&node, Naming::Listing, "", 0, &mut out);
-    assert!(out.contains("/ listing: 2 child(ren) /"), "{out}");
-    assert!(
-        out.contains(&format!(
-            "0x3 => h'{}' / digest /",
-            "ab".repeat(MERKLE_HASH_LEN)
-        )),
-        "{out}"
-    );
+fn non_shortest_length_heads_fall_back() {
+    let canonical = rendered(&[0x82, 0x40, 0x40]);
+    let wide_second = rendered(&[0x82, 0x40, 0x58, 0x00]);
+    let wide_first = rendered(&[0x82, 0x58, 0x00, 0x40]);
+    assert_eq!(canonical, ["[h'', h'']"]);
     assert!(
-        out.contains(&format!(
-            "0xc => h'{}' / digest /",
-            "01".repeat(MERKLE_HASH_LEN)
-        )),
-        "{out}"
+        wide_second[0].contains("non-shortest byte string head"),
+        "{wide_second:?}"
     );
-    assert!(!out.contains("NON-CANONICAL"), "{out}");
+    assert_eq!(wide_second[1], "h'82405800'");
+    assert_eq!(wide_first[1], "h'82580040'");
+    let embedded = rendered(&[0xd8, 0x18, 0x44, 0x82, 0x40, 0x58, 0x00]);
+    assert!(embedded[0].contains("in an embedded item"), "{embedded:?}");
 }
 
-/// A listing whose radixes are not strictly ascending renders with an
-/// explicit order verdict: the violation is visible in the transcript,
-/// and the entries still render completely, in wire order.
+/// The ill-formed two-byte spelling of a simple value (`f8 14` for
+/// `false`) falls back to hex, so arrays differing only in which `false`
+/// is ill-formed render distinctly.
+///
+/// The re-encoding check catches it: re-encoding uses the one-byte form.
 #[test]
-fn descending_listing_renders_an_order_verdict() {
-    let mut bytes = Vec::new();
-    cbor::write_head(&mut bytes, MAJOR_MAP, 2);
-    for radix in [0xf_u8, 0x0] {
-        cbor::write_head(&mut bytes, MAJOR_UINT, u64::from(radix));
-        cbor::write_head(&mut bytes, MAJOR_BSTR, MERKLE_HASH_LEN as u64);
-        bytes.extend_from_slice(&[radix; MERKLE_HASH_LEN]);
-    }
-    let mut input = bytes.as_slice();
-    let node = parse_node(&mut input, 0).expect("heads are canonical; order is not");
+fn ill_formed_simple_values_fall_back() {
+    assert_eq!(rendered(&[0x82, 0xf4, 0xf4]), ["[false, false]"]);
+    let second = rendered(&[0x82, 0xf4, 0xf8, 0x14]);
+    let first = rendered(&[0x82, 0xf8, 0x14, 0xf4]);
+    assert!(second[0].contains("re-encodes differently"), "{second:?}");
+    assert_eq!(second[1], "h'82f4f814'");
+    assert_eq!(first[1], "h'82f814f4'");
+}
+
+/// Every NaN falls back to hex, so two NaNs differing only in the sign
+/// bit render distinctly, while a finite float renders in notation.
+///
+/// The parser and encoder are bit-exact on floats, so the re-encoding
+/// check alone would pass both.
+#[test]
+fn nans_fall_back() {
+    let positive = rendered(&[0xfb, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0]);
+    let negative = rendered(&[0xfb, 0xff, 0xf8, 0, 0, 0, 0, 0, 0]);
+    assert!(positive[0].contains("NaN"), "{positive:?}");
+    assert_eq!(positive[1], "h'fb7ff8000000000000'");
+    assert_eq!(negative[1], "h'fbfff8000000000000'");
+    assert_eq!(rendered(&[0xf9, 0x3c, 0x00]), ["1.0_1"]);
+}
+
+/// The two-byte spelling of a simple value in 24 through 31 (`f8 18`)
+/// is not well-formed CBOR and falls back to hex; the parser accepts it
+/// and re-encodes it verbatim, so the walk rejects it by value.
+#[test]
+fn simple_values_without_a_well_formed_spelling_fall_back() {
+    let out = rendered(&[0xf8, 0x18]);
+    assert!(out[0].contains("ill-formed simple value"), "{out:?}");
+    assert_eq!(out[1], "h'f818'");
+    assert_eq!(rendered(&[0xf8, 0x20]), ["simple(32)"]);
+}
+
+/// A text string holding a control character falls back to hex, so a
+/// payload cannot forge a header.
+///
+/// A string carrying a newline followed by a frame header's text renders
+/// under its real header as hex, and the only line beginning with
+/// `frame ` is the header.
+#[test]
+fn text_with_control_characters_falls_back() {
+    let forged = "a\nframe 9 (1 bytes) / Supply(End) /";
+    let (stream, bytes) = supply_frame(&Message::new(forged.to_string()));
     let mut out = String::new();
-    render_node(&node, Naming::Listing, "", 0, &mut out);
-    assert!(out.contains("NON-CANONICAL ORDER"), "{out}");
-    assert!(out.contains("0xf =>"), "first entry renders: {out}");
-    assert!(out.contains("0x0 =>"), "second entry renders: {out}");
+    render_frame(Speaker::Initiator, stream, 0, &bytes, &mut out);
+    let headers: Vec<_> = out
+        .lines()
+        .filter(|line| line.trim_start().starts_with("frame "))
+        .collect();
+    assert_eq!(headers.len(), 1, "{out}");
+    assert!(out.contains("control character in a text string"), "{out}");
 }
 
-/// A version-tagged byte string whose bytes are no version encoding
-/// annotates the failure rather than inventing a meaning.
+/// An item followed by trailing bytes is not one item: the whole buffer
+/// falls back explicitly rather than rendering the item and dropping the
+/// tail.
 #[test]
-fn garbage_version_atom_annotates_undecodable() {
-    let mut bytes = Vec::new();
-    cbor::write_tag(&mut bytes, crate::tags::VERSION_TAG);
-    cbor::write_head(&mut bytes, MAJOR_BSTR, 3);
-    bytes.extend_from_slice(&[0xff, 0xff, 0xff]);
-    let mut input = bytes.as_slice();
-    let node = parse_node(&mut input, 0).expect("the wrapper is canonical");
+fn trailing_bytes_fall_back_explicitly() {
+    let two_items = [0x01, 0x02];
     let mut out = String::new();
-    render_node(&node, Naming::Plain, "", 0, &mut out);
-    assert!(out.contains("causal version undecodable"), "{out}");
-    assert!(out.contains("h'ffffff'"), "the atom bytes stand: {out}");
+    render_item(&two_items, &mut out);
+    assert!(out.starts_with("!! not rendered as CBOR ("), "{out}");
+    assert!(out.contains("h'0102'"), "{out}");
 }
 
 /// The totality witness accepts exactly the wire it was given, split at
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 1:
>
> The test module doc states the renderer's commitments as they now stand. Tests asserting the hand renderer's forms (field localization, listing annotations, order verdict, version gloss, hand depth budget) are removed with the code they pinned.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 40:
>
> Point test for the frame header (index, exact byte count, decoded signal) and that the body is diagnostic notation with the supply run unfolded to the payload.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 61:
>
> The explicit-fallback commitment on a truncated array; the reason is cbor-diag's own error text.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 171:
>
> Trailing bytes after an item are not one item: cbor-diag's parse_bytes rejects them (verified in its source), so the whole buffer falls back rather than the tail being dropped.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 91:
>
> Fresh-eyes repair (item 1): the first collision pair as a point test. Both wide variants carry the non-shortest head, so both fall back (each to its own hex); the canonical array alone renders in notation; the embedded case is included.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 112:
>
> Fresh-eyes repair (item 1): the second collision pair and the ill-formed simple, caught by the re-encoding check.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 154:
>
> Fresh-eyes repair (item 2): the header-forgery case. A string payload carrying a newline and a frame header's text renders as hex under its real header, so exactly one line begins with `frame `. Durable fix recorded for the owner: the fork should escape control characters in text (T140's fork list); until then the walk sends such text to hex.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 127:
>
> Fresh-eyes repair, round 2 (item 1): the sign pair as a point test, each side to its own hex, beside a finite float that renders in notation.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 140:
>
> Fresh-eyes repair, round 2 (item 3): `f8 18` falls back; `f8 20` (32) still renders.

<a id="hunk-16"></a>
### src/tree/mirror/streaming/remote/codec/capture/tests.rs `@@ -235,8 +213,7 @@ fn stream_label_parses_epoch_and_index() {`

```diff
@@ -235,8 +213,7 @@ fn stream_label_parses_epoch_and_index() {
     assert_eq!(len, 3, "one short head and one byte-argument head");
 }
 
-/// Control items are named by their shape; an unknown shape is a broken
-/// capture, not a renderable one.
+/// Control items are named by their shape.
 #[test]
 fn control_items_are_named_by_shape() {
     let mut preamble = Vec::new();
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 213:
>
> Nit: the control-item naming test's doc claimed the unknown-shape panic without exercising it; the claim moved to its own should_panic test below, and this doc states only what this body checks.

<a id="hunk-17"></a>
### src/tree/mirror/streaming/remote/codec/capture/tests.rs `@@ -257,23 +234,19 @@ fn control_items_are_named_by_shape() {`

```diff
@@ -257,23 +234,19 @@ fn control_items_are_named_by_shape() {
     assert_eq!(control_item_name(&epilogue), "epilogue");
 }
 
-/// Nesting past the walk's depth bound falls back explicitly instead of
-/// recursing without bound on input-controlled depth.
+/// A control item of no known shape is a broken capture, not a
+/// renderable one.
 #[test]
-fn nesting_past_the_depth_bound_falls_back() {
-    let mut bytes = Vec::new();
-    for _ in 0..=MAX_DEPTH {
-        cbor::write_head(&mut bytes, MAJOR_ARRAY, 1);
-    }
-    cbor::write_head(&mut bytes, MAJOR_UINT, 0);
-    let mut input = bytes.as_slice();
-    let error = parse_node(&mut input, 0).expect_err("too deep to vouch for");
-    assert!(error.contains("deeper than"), "{error}");
+#[should_panic(expected = "no known shape")]
+fn unknown_control_item_shapes_are_refused() {
+    let mut bare = Vec::new();
+    cbor::write_head(&mut bare, MAJOR_UINT, 0);
+    control_item_name(&bare);
 }
 
-/// Build a chain of `levels` nested embedded byte strings — each level
-/// one tag-24 item wrapping the next level's encoding as a byte
-/// string — bottoming out at a single `0x00` uint.
+/// Build a chain of `levels` nested embedded byte strings, each level
+/// one tag-24 item wrapping the next level's encoding as a byte string,
+/// bottoming out at a single `0x00` uint.
 ///
 /// Written outside-in: encoded lengths follow the recurrence
 /// `len[0] = 1` (the innermost uint) and
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 241:
>
> Nit swept: the control-item naming test's doc claimed the unknown-shape panic without exercising it; the claim is now its own should_panic test.

<a id="hunk-18"></a>
### src/tree/mirror/streaming/remote/codec/capture/tests.rs `@@ -295,60 +268,59 @@ fn embedded_chain(levels: usize) -> Vec<u8> {`

```diff
@@ -295,60 +268,59 @@ fn embedded_chain(levels: usize) -> Vec<u8> {
     bytes
 }
 
-/// The rendered output carries the depth fallback: the explicit
-/// too-deep failure line, with the convicted bytes standing as hex on
-/// the line below it.
-fn assert_depth_fallback(out: &str) {
-    assert!(
-        out.contains(&format!("nested deeper than {MAX_DEPTH}")),
-        "the depth fallback is explicit: {out}"
-    );
-    let fallback_hex = out
-        .lines()
-        .skip_while(|line| !line.contains("nested deeper than"))
-        .nth(1)
-        .unwrap_or_default();
-    assert!(
-        fallback_hex.trim_start().starts_with("h'"),
-        "the exact bytes stand under the failure line: {fallback_hex:?}"
-    );
-}
-
-/// A chain of embedded byte strings nested far past the depth bound
-/// renders to the explicit depth fallback instead of overflowing the
-/// stack.
+/// A chain of embedded byte strings nested far past the depth limit
+/// renders with bounded recursion.
 ///
-/// The walk's one depth budget spans embedded-byte-string re-parses,
-/// so no input-controlled nesting recurses without bound.
+/// Unfolding stops at the limit and the remainder stands as a plain byte
+/// string; the walk returns rather than overflowing the stack.
 #[test]
-fn deep_embedded_chain_falls_back_instead_of_recursing() {
-    let bytes = embedded_chain(10 * MAX_DEPTH);
+fn deep_embedded_chain_stops_unfolding_instead_of_recursing() {
+    let bytes = embedded_chain(10 * cbor_diag::DEFAULT_DEPTH_LIMIT);
     let mut out = String::new();
-    render_item(&bytes, "", &mut out);
-    assert_depth_fallback(&out);
+    render_item(&bytes, &mut out);
+    assert!(
+        out.starts_with("24_0(<<"),
+        "the chain unfolds from the top: {out:.80}"
+    );
+    assert!(out.contains("24_0(h'"), "and stops at the limit: {out:.80}");
 }
 
-/// The same nesting arriving as a supply record's payload — the
-/// harness-reachable path, a captured frame rendered whole — hits the
-/// same depth fallback.
+/// The same nesting arriving as a supply record's payload (a captured
+/// frame rendered whole) renders with the same bounded unfold.
 ///
 /// An application payload of legal CBOR can nest arbitrarily, and the
 /// frame walk must return, never overflow.
 #[test]
-fn deep_payload_through_the_frame_path_falls_back() {
+fn deep_payload_through_the_frame_path_renders() {
     let payload = Message::from_slice::<ciborium::Value>(
-        &embedded_chain(10 * MAX_DEPTH),
+        &embedded_chain(10 * cbor_diag::DEFAULT_DEPTH_LIMIT),
         PayloadDepthLimit::default(),
     )
     .expect("the chain is exactly one CBOR item");
-    let mut run = LeafRun::new();
-    run.push(&Version::new(), &payload)
-        .expect("one record fits a fresh run");
-    let stream = Stream::new(0).expect("stream 0 names a stream");
-    let frame = (stream, Frame::Reaction(Reaction::Supply(run), Flow::End));
+    let (stream, bytes) = supply_frame(&payload);
+    let mut out = String::new();
+    render_frame(Speaker::Initiator, stream, 0, &bytes, &mut out);
+    assert!(
+        out.contains("24_0(h'"),
+        "unfolding stopped at the limit: {out:.120}"
+    );
+}
+
+/// Structure nested past cbor-diag's depth limit is not one parseable
+/// item: the whole buffer falls back explicitly, naming the depth, rather
+/// than recursing without bound on input-controlled depth.
+#[test]
+fn structure_past_the_depth_limit_falls_back_explicitly() {
     let mut bytes = Vec::new();
-    encode(Speaker::Initiator, &frame, &mut bytes).expect("a supply frame encodes");
+    for _ in 0..=cbor_diag::DEFAULT_DEPTH_LIMIT {
+        cbor::write_head(&mut bytes, MAJOR_ARRAY, 1);
+    }
+    cbor::write_head(&mut bytes, MAJOR_UINT, 0);
     let mut out = String::new();
-    render_frame(Speaker::Initiator, stream, &bytes, &mut out);
-    assert_depth_fallback(&out);
+    render_item(&bytes, &mut out);
+    assert!(out.starts_with("!! not rendered as CBOR ("), "{out:.120}");
+    assert!(
+        out.contains(&format!("h'{}'", hex::encode(&bytes))),
+        "the exact bytes stand"
+    );
 }
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 277:
>
> The depth stress tests are kept and re-denominated against cbor-diag's DEFAULT_DEPTH_LIMIT: an embedded chain ten times deeper renders with bounded recursion through the item path and the frame path, and structure past the limit falls back explicitly. The no-unbounded-recursion property stays committed as a stress test.

<a id="hunk-19"></a>
### tests/gossip_snapshot.rs `@@ -10,8 +10,8 @@`

```diff
@@ -10,8 +10,8 @@
 //! its procedure (`cargo insta review`) are in `AGENTS.md`.
 //!
 //! The payload type is `u64` throughout: a small integer is one CBOR byte
-//! (`01`, `02`, …), which keeps the dumps short and lets distinct payloads
-//! be spotted directly in the hex.
+//! and renders as itself (`1`, `2`, …), which keeps the captures short and
+//! lets distinct payloads be spotted directly in the rendering.
 
 mod common;
 
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 13:
>
> Prose kept true to the rendering: payloads are spotted as decimal integers in diagnostic notation, no longer as hex bytes.

<a id="hunk-20"></a>
### tests/gossip_snapshot.rs `@@ -158,17 +158,14 @@ fn asymmetric_message_targets_unbatch_the_run() {`

```diff
@@ -158,17 +158,14 @@ fn asymmetric_message_targets_unbatch_the_run() {
     insta::assert_snapshot!(capture_gossip(a, b));
 }
 
-/// Extract one rendered signal line's semantic.
+/// Extract one frame header's semantic.
 ///
-/// A signal line has the form `<state code> / <Semantic> /`. The
-/// bare-digit code distinguishes it from every other annotated line
-/// (tagged atoms carry parentheses, listings carry `=>`, payloads carry
-/// no comment) except the frame's stream line, `<index> / stream /`,
-/// whose lowercase comment tells it apart from the capitalized
-/// semantics, so the extraction cannot misfire inside a frame.
+/// A frame header has the form `frame <n> (<b> bytes) / <Semantic> /`.
+/// The renderer guarantees no body line begins with `frame ` (text with
+/// a control character renders as hex), so the prefix identifies it.
 fn signal_semantic(line: &str) -> Option<&str> {
-    let (code, rest) = line.trim_start().split_once(" / ")?;
-    if code.is_empty() || !code.bytes().all(|b| b.is_ascii_digit()) {
+    let (head, rest) = line.trim_start().split_once(" / ")?;
+    if !head.starts_with("frame ") {
         return None;
     }
     let semantic = rest.strip_suffix(" /")?;
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 166:
>
> Consequence T138 did not name: three scenarios in this suite assert on the frame sequence by scanning the hand renderer's annotated state lines and listing headers. The scan now reads the frame header's label; with bodies verbatim at column zero, the doc states the oracle assumption plainly (no rendered item starts a line with `frame `) instead of claiming an absolute distinction.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 166:
>
> Fresh-eyes repair (item 2): the doc now states the renderer's guarantee (no body line begins with `frame `) instead of an oracle assumption about the corpus; same for is_capture_header and the target_message_size and opening_supply scanners.

<a id="hunk-21"></a>
### tests/gossip_snapshot.rs `@@ -179,39 +176,81 @@ fn signal_semantic(line: &str) -> Option<&str> {`

```diff
@@ -179,39 +176,81 @@ fn signal_semantic(line: &str) -> Option<&str> {
 
 /// The child count of every nonempty-Query frame body in a capture.
 ///
-/// A `Query(…)` signal line is followed by its frame's listing body,
-/// which opens with `{ / listing: <n> child(ren) /`. Greeting listings
-/// render the same annotation, so the scan keys on the Query signal
-/// and reads only until the next signal or column-zero header.
+/// A `Query(…)` frame's listing is a map from radix to digest. The scan
+/// counts its entries, however the layout places them, from the Query
+/// header to the next frame or capture header, so the greeting's listing
+/// (under a control item) is never counted.
 fn nonempty_query_listings(capture: &str) -> Vec<usize> {
     let mut counts = Vec::new();
-    let mut in_query = false;
+    let mut in_query: Option<usize> = None;
     for line in capture.lines() {
         if let Some(semantic) = signal_semantic(line) {
-            in_query = semantic.starts_with("Query(");
-        } else if !line.starts_with(char::is_whitespace) {
-            in_query = false;
-        } else if in_query
-            && let Some(rest) = line.trim_start().strip_prefix("{ / listing: ")
-            && let Some(n) = rest.split_whitespace().next().and_then(|n| n.parse().ok())
-        {
-            counts.push(n);
-            in_query = false;
+            if let Some(n) = in_query.take().filter(|n| *n > 0) {
+                counts.push(n);
+            }
+            in_query = semantic.starts_with("Query(").then_some(0);
+        } else if is_capture_header(line) {
+            if let Some(n) = in_query.take().filter(|n| *n > 0) {
+                counts.push(n);
+            }
+        } else if let Some(n) = in_query.as_mut() {
+            *n += listing_entries(line);
         }
     }
+    if let Some(n) = in_query.filter(|n| *n > 0) {
+        counts.push(n);
+    }
     counts
 }
 
+/// Whether one rendered line is a capture header: a direction, role,
+/// control-item, or stream header.
+///
+/// The capture writes these at column zero in a fixed vocabulary, and a
+/// rendered item cannot start a line with any of it: its text is quoted
+/// and holds no control character.
+fn is_capture_header(line: &str) -> bool {
+    line.starts_with("direction ")
+        || line.starts_with("role: ")
+        || line.starts_with("control item ")
+        || (line.contains(" stream ") && line.ends_with(" wire bytes"))
+}
+
+/// The listing entries on one rendered line, wherever the layout puts
+/// them: each an unsigned-integer key with an optional encoding
+/// indicator, a colon, and a byte-string value, preceded by a line
+/// start, a brace, or a comma.
+fn listing_entries(line: &str) -> usize {
+    let bytes = line.as_bytes();
+    line.match_indices(": h'")
+        .filter(|(at, _)| {
+            let mut key = *at;
+            if let Some(mark) = line[..key].rfind('_')
+                && line[mark + 1..key].len() == 1
+                && bytes[mark + 1].is_ascii_digit()
+            {
+                key = mark;
+            }
+            let digits = line[..key]
+                .bytes()
+                .rev()
+                .take_while(u8::is_ascii_digit)
+                .count();
+            let before = line[..key - digits].trim_end();
+            digits > 0 && (before.is_empty() || before.ends_with(['{', ',']))
+        })
+        .count()
+}
+
 /// Count the frames rendered under one stream header of a wire capture.
 ///
 /// Returns `None` when the header never appears; the header must be a
 /// prefix of the capture's
 /// `"{Speaker} stream {index} (height {height}), epoch {e}, {n} wire bytes"`
-/// header line. A stream's body lines — frame headers, rendered value
-/// trees — are all indented, so the section ends at the next column-zero
-/// line (the following stream or direction header, or a control-item
-/// label); within the section each frame contributes exactly one signal
-/// line, whose comment is the frame's semantic.
+/// header line. The section ends at the next capture header (the
+/// following stream or direction header, or a control-item label);
+/// within the section each frame contributes exactly one header line,
+/// whose comment is the frame's semantic.
 fn stream_frames(capture: &str, header: &str) -> Option<Vec<String>> {
     let mut frames = None;
     for line in capture.lines() {
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 183:
>
> The query-listing child count is counted from the listing's entries between a Query frame header and the next frame or capture header, since the `/ listing: N child(ren) /` gloss is dropped and bodies at column zero can no longer end a section by indentation.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 211:
>
> Section boundaries for the two scans, now that item bodies stand at column zero: a capture header is one of the harness's fixed column-zero forms.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 221:
>
> The same listing-entry shape tools/digestshare recognizes, stated once here for the suite.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 223:
>
> Fresh-eyes repair (item 3): the entry scan was layout-dependent (one entry per line, which cbor-diag chooses only because the 24-byte digest sits just above its trivial-layout threshold). Entries are now counted wherever they sit on a line, preceded by a line start, brace, or comma; the same rule tools/digestshare applies.

<a id="hunk-22"></a>
### tests/gossip_snapshot.rs `@@ -220,7 +259,7 @@ fn stream_frames(capture: &str, header: &str) -> Option<Vec<String>> {`

```diff
@@ -220,7 +259,7 @@ fn stream_frames(capture: &str, header: &str) -> Option<Vec<String>> {
         } else if let Some(frames) = frames.as_mut() {
             if let Some(semantic) = signal_semantic(line) {
                 frames.push(semantic.to_string());
-            } else if !line.starts_with(char::is_whitespace) {
+            } else if is_capture_header(line) {
                 break;
             }
         }
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 259:
>
> stream_frames ended a stream's section at any column-zero line, which held while bodies were indented; with bodies verbatim at column zero it ends at the next capture header instead, the same boundary the query-listing scan uses.

<a id="hunk-23"></a>
### tests/opening_supply.rs `@@ -42,19 +42,16 @@ const BALLAST_POOL: (u64, u64) = (10_000, 16);`

```diff
@@ -42,19 +42,16 @@ const BALLAST_POOL: (u64, u64) = (10_000, 16);
 /// Count the frames whose semantic label starts with `label` in a rendered
 /// wire capture, across both directions.
 ///
-/// A frame's semantic is the comment on its state line
-/// (`<state code> / <semantic> /`); the bare-digit code and the label
-/// prefix distinguish it from every other annotated line in the
-/// rendering, the frame's `<index> / stream /` line included.
+/// A frame's semantic is the comment on its header line
+/// (`frame <n> (<b> bytes) / <semantic> /`); the renderer guarantees no
+/// body line begins with `frame `, so the prefix identifies the header.
 fn frames_labeled(capture: &str, label: &str) -> usize {
     capture
         .lines()
         .filter_map(|line| {
-            let (code, rest) = line.trim_start().split_once(" / ")?;
-            if code.is_empty() || !code.bytes().all(|b| b.is_ascii_digit()) {
-                return None;
-            }
-            rest.strip_suffix(" /")
+            let (head, rest) = line.trim_start().split_once(" / ")?;
+            head.starts_with("frame ")
+                .then(|| rest.strip_suffix(" /"))?
         })
         .filter(|semantic| semantic.starts_with(label))
         .count()
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 48:
>
> Consequence T138 did not name, found by the gate: this suite counted frames by the hand renderer's annotated state line. It now reads the frame header's label, like the other two suites; the doc is shortened to the header's form.

<a id="hunk-24"></a>
### tests/snapshots/bootstrap_snapshot__empty_provider.snap `@@ -4,56 +4,42 @@ expression: capture_bootstrap(provider)`

```diff
@@ -4,56 +4,42 @@ expression: capture_bootstrap(provider)
 ---
 direction A -> B
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (5 bytes) / party hand-off /
-  53845(h'48') / party /
+53845_1(h'48')
 control item 3 (2 bytes) / epilogue /
-  "."
+"."
 
 direction B -> A
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'00000000000000000000000000000000'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'00000000000000000000000000000000',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-25"></a>
### tests/snapshots/bootstrap_snapshot__mutual_bootstrap_bails.snap `@@ -4,54 +4,40 @@ expression: capture`

```diff
@@ -4,54 +4,40 @@ expression: capture
 ---
 direction A -> B
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'00000000000000000000000000000000'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'00000000000000000000000000000000',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 
 direction B -> A
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'00000000000000000000000000000000'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'00000000000000000000000000000000',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-26"></a>
### tests/snapshots/bootstrap_snapshot__populated_provider.snap `@@ -5,99 +5,56 @@ expression: capture_bootstrap(provider)`

```diff
@@ -5,99 +5,56 @@ expression: capture_bootstrap(provider)
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (187 bytes) / greeting /
-  24(<< / embedded item, 183 bytes /
-    {
-      "listing" =>
-        { / listing: 3 child(ren) /
-          0x1e => h'873255e39c6ccc1019916b8d5c5eb211a991ea504acd727d' / digest /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-          0x8e => h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32' / digest /
-        }
-      "set_len" => 3
-      "version" =>
-        53846(h'92') / causal version, event tree: 3 /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        30_0: h'873255e39c6ccc1019916b8d5c5eb211a991ea504acd727d',
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+        142_0: h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32',
+    },
+    "set_len": 3,
+    "version": 53846_1(h'92'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (5 bytes) / party hand-off /
-  53845(h'48') / party /
+53845_1(h'48')
 control item 3 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 50 wire bytes
-  frame 0 (15 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 9 bytes /
-        63(<< / record, 2 item(s), 6 bytes /
-          53846(h'92') / causal version, event tree: 3 /
-          3
-        >>)
-      >>)
-    ]
-  frame 1 (15 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 9 bytes /
-        63(<< / record, 2 item(s), 6 bytes /
-          53846(h'b8') / causal version, event tree: 2 /
-          2
-        >>)
-      >>)
-    ]
-  frame 2 (15 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 9 bytes /
-        63(<< / record, 2 item(s), 6 bytes /
-          53846(h'a8') / causal version, event tree: 1 /
-          1
-        >>)
-      >>)
-    ]
-  frame 3 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (15 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'92'), 3>>)>>)]
+  frame 1 (15 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'b8'), 2>>)>>)]
+  frame 2 (15 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'a8'), 1>>)>>)]
+  frame 3 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'00000000000000000000000000000000'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'00000000000000000000000000000000',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-27"></a>
### tests/snapshots/bootstrap_snapshot__string_payload.snap `@@ -5,87 +5,53 @@ expression: capture_bootstrap(provider)`

```diff
@@ -5,87 +5,53 @@ expression: capture_bootstrap(provider)
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (159 bytes) / greeting /
-  24(<< / embedded item, 155 bytes /
-    {
-      "listing" =>
-        { / listing: 2 child(ren) /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-          0x8e => h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'b8') / causal version, event tree: 2 /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+        142_0: h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'b8'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (5 bytes) / party hand-off /
-  53845(h'48') / party /
+53845_1(h'48')
 control item 3 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 45 wire bytes
-  frame 0 (20 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 14 bytes /
-        63(<< / record, 2 item(s), 11 bytes /
-          53846(h'b8') / causal version, event tree: 2 /
-          "world"
-        >>)
-      >>)
-    ]
-  frame 1 (20 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 14 bytes /
-        63(<< / record, 2 item(s), 11 bytes /
-          53846(h'a8') / causal version, event tree: 1 /
-          "hello"
-        >>)
-      >>)
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (20 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'b8'), "world">>)>>)]
+  frame 1 (20 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'a8'), "hello">>)>>)]
+  frame 2 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'00000000000000000000000000000000'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'00000000000000000000000000000000',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-28"></a>
### tests/snapshots/gossip_snapshot__asymmetric_message_targets_unbatch_the_run.snap `@@ -5,84 +5,50 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,84 +5,50 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (136 bytes) / greeting /
-  24(<< / embedded item, 132 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x90 => h'5b9b4e07e489554d42eef4b5c4a7ae87f5483a12455d0b4f' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'400fff001ff9') / causal version, event tree: (0, 2046, 0) /
-      "max_version_bytes" => 6
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        144_0: h'5b9b4e07e489554d42eef4b5c4a7ae87f5483a12455d0b4f',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'400fff001ff9'),
+    "max_version_bytes": 6,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 47 wire bytes
-  frame 0 (22 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 16 bytes /
-        63(<< / record, 2 item(s), 13 bytes /
-          53846(h'40120c009010') / causal version, event tree: (0, 576, 0) /
-          575
-        >>)
-      >>)
-    ]
-  frame 1 (20 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 14 bytes /
-        63(<< / record, 2 item(s), 11 bytes /
-          53846(h'4042c02110') / causal version, event tree: (0, 132, 0) /
-          131
-        >>)
-      >>)
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (22 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'40120c009010'), 575_1>>)>>)]
+  frame 1 (20 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'4042c02110'), 131_0>>)>>)]
+  frame 2 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (99 bytes) / greeting /
-  24(<< / embedded item, 95 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 0
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 0,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-29"></a>
### tests/snapshots/gossip_snapshot__batched_supply_run.snap `@@ -5,77 +5,55 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,77 +5,55 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (136 bytes) / greeting /
-  24(<< / embedded item, 132 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x90 => h'5b9b4e07e489554d42eef4b5c4a7ae87f5483a12455d0b4f' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'400fff001ff9') / causal version, event tree: (0, 2046, 0) /
-      "max_version_bytes" => 6
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        144_0: h'5b9b4e07e489554d42eef4b5c4a7ae87f5483a12455d0b4f',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'400fff001ff9'),
+    "max_version_bytes": 6,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 42 wire bytes
-  frame 0 (37 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 2 record(s), 30 bytes /
-        63(<< / record, 2 item(s), 13 bytes /
-          53846(h'40120c009010') / causal version, event tree: (0, 576, 0) /
-          575
-        >>)
-        63(<< / record, 2 item(s), 11 bytes /
-          53846(h'4042c02110') / causal version, event tree: (0, 132, 0) /
-          131
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (37 bytes) / Supply(End) /
+[
+    0,
+    7,
+    63_0(<<
+        63_0(<<53846_1(h'40120c009010'), 575_1>>),
+        63_0(<<53846_1(h'4042c02110'), 131_0>>),
+    >>),
+]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-30"></a>
### tests/snapshots/gossip_snapshot__both_redact_the_same_message.snap `@@ -5,68 +5,50 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,68 +5,50 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (132 bytes) / greeting /
-  24(<< / embedded item, 128 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'4950') / causal version, event tree: (2, 1, 0) /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'4950'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 8 wire bytes
-  frame 0 (3 bytes)
-    [
-      0 / stream /
-      1 / Match(End) /
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / Match(End) /
+[0, 1]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (132 bytes) / greeting /
-  24(<< / embedded item, 128 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'5dc0') / causal version, event tree: (2, 0, 1) /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'5dc0'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-31"></a>
### tests/snapshots/gossip_snapshot__bulk_initiator_ships_opening_supplies.snap `@@ -5,135 +5,72 @@ expression: capture`

```diff
@@ -5,135 +5,72 @@ expression: capture
 direction A -> B
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (134 bytes) / greeting /
-  24(<< / embedded item, 130 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x9a => h'fff22c04c986261bb486eb580d11a38f011b4c6b326f7a13' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'40ff01f9') / causal version, event tree: (0, 126, 0) /
-      "max_version_bytes" => 3
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        154_0: h'fff22c04c986261bb486eb580d11a38f011b4c6b326f7a13',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'40ff01f9'),
+    "max_version_bytes": 3,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Initiator stream 0 (height 31), epoch 0, 33 wire bytes
-  frame 0 (28 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 2 record(s), 22 bytes /
-        63(<< / record, 2 item(s), 9 bytes /
-          53846(h'437069') / causal version, event tree: (0, 26, 0) /
-          25
-        >>)
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'4934') / causal version, event tree: (0, 3, 0) /
-          2
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (28 bytes) / Supply(End) /
+[
+    0,
+    7,
+    63_0(<<63_0(<<53846_1(h'437069'), 25_0>>), 63_0(<<53846_1(h'4934'), 2>>)>>),
+]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 Initiator stream 1 (height 30), epoch 0, 8 wire bytes
-  frame 0 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 1 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 1 (3 bytes) / End(Stream) /
+[1, 9]
 
 direction B -> A
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (188 bytes) / greeting /
-  24(<< / embedded item, 184 bytes /
-    {
-      "listing" =>
-        { / listing: 3 child(ren) /
-          0xad => h'c427db6fe92e923509619eb51d8a601e31b985ac24a3baa2' / digest /
-          0xe8 => h'a82ff6a5e3f64d4901f633ca61b549cfa54be033845641db' / digest /
-          0xf1 => h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2' / digest /
-        }
-      "set_len" => 3
-      "version" =>
-        53846(h'7077') / causal version, event tree: (0, 0, 29) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        173_0: h'c427db6fe92e923509619eb51d8a601e31b985ac24a3baa2',
+        232_0: h'a82ff6a5e3f64d4901f633ca61b549cfa54be033845641db',
+        241_0: h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2',
+    },
+    "set_len": 3,
+    "version": 53846_1(h'7077'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 61 wire bytes
-  frame 0 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 1 (18 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 12 bytes /
-        63(<< / record, 2 item(s), 9 bytes /
-          53846(h'72c0') / causal version, event tree: (0, 0, 2) /
-          10001
-        >>)
-      >>)
-    ]
-  frame 2 (18 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 12 bytes /
-        63(<< / record, 2 item(s), 9 bytes /
-          53846(h'73c0') / causal version, event tree: (0, 0, 3) /
-          10002
-        >>)
-      >>)
-    ]
-  frame 3 (17 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'77') / causal version, event tree: (0, 0, 1) /
-          10000
-        >>)
-      >>)
-    ]
-  frame 4 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 1 (18 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'72c0'), 10001_1>>)>>)]
+  frame 2 (18 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'73c0'), 10002_1>>)>>)]
+  frame 3 (17 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'77'), 10000_1>>)>>)]
+  frame 4 (3 bytes) / End(Stream) /
+[0, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-32"></a>
### tests/snapshots/gossip_snapshot__converged_forks_noop.snap `@@ -4,58 +4,46 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -4,58 +4,46 @@ expression: "capture_gossip(a, b)"
 ---
 direction A -> B
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (159 bytes) / greeting /
-  24(<< / embedded item, 155 bytes /
-    {
-      "listing" =>
-        { / listing: 2 child(ren) /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-          0x8e => h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'b8') / causal version, event tree: 2 /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+        142_0: h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'b8'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 
 direction B -> A
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (159 bytes) / greeting /
-  24(<< / embedded item, 155 bytes /
-    {
-      "listing" =>
-        { / listing: 2 child(ren) /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-          0x8e => h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'b8') / causal version, event tree: 2 /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+        142_0: h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'b8'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-33"></a>
### tests/snapshots/gossip_snapshot__deep_trie_divergence.snap `@@ -5,611 +5,224 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,611 +5,224 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (552 bytes) / greeting /
-  24(<< / embedded item, 547 bytes /
-    {
-      "listing" =>
-        { / listing: 16 child(ren) /
-          0x9 => h'7fcfe5ddc79994d48121e59df4fa76b2abc10dae127871c8' / digest /
-          0x12 => h'2faefee85382ec00ec6e46bee4d4e65a8e5704c704171c27' / digest /
-          0x19 => h'033a5a5bdafdf660c15507531d90f304348728b13a6989e8' / digest /
-          0x2f => h'f2455eb6489c2ee4b1992d50127ce15df9fe1116ae9aa81c' / digest /
-          0x37 => h'8aa4c74db538a6640e6d368e2519a78db03eeac36619a96d' / digest /
-          0x52 => h'b0425f1120b0c131ab72af780c67564c0340de7049ea3580' / digest /
-          0x63 => h'4ab319add5bfe22094db37633d4d7ab3aa5d3002f8f3471f' / digest /
-          0x72 => h'affc07ce6c37c4fc0442c25b05048afcf175156ace6bcead' / digest /
-          0x79 => h'9602973f461003f36f18b1ccb468d352b1f1a711efc1e963' / digest /
-          0x94 => h'fdf2e4f46197ca0b24b8461d8d83e63deedddbd3af0b6886' / digest /
-          0x9a => h'5bd84f5aa8c02d35d684a052efc14fbf01aa946496120b18' / digest /
-          0xa6 => h'd7cc20cdaabd6f00dcd5eea4d9674d4e71eefadc66bded88' / digest /
-          0xb2 => h'3fd1810476df8f9e3bce3224af62ac29973ae1154e0f794c' / digest /
-          0xbb => h'bc14df3d4ab8b50233aa6d692ed9cd05e44953c11310f7b1' / digest /
-          0xc9 => h'6519a416494a98ae06ff1f9ae592d7538f994d4afe68cfbd' / digest /
-          0xd4 => h'e57fcaad6bbe82b187d1b2f96b4cd384abe9243e017b9d89' / digest /
-        }
-      "set_len" => 16
-      "version" =>
-        53846(h'423041') / causal version, event tree: (0, 16, 0) /
-      "max_version_bytes" => 3
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        9: h'7fcfe5ddc79994d48121e59df4fa76b2abc10dae127871c8',
+        18: h'2faefee85382ec00ec6e46bee4d4e65a8e5704c704171c27',
+        25_0: h'033a5a5bdafdf660c15507531d90f304348728b13a6989e8',
+        47_0: h'f2455eb6489c2ee4b1992d50127ce15df9fe1116ae9aa81c',
+        55_0: h'8aa4c74db538a6640e6d368e2519a78db03eeac36619a96d',
+        82_0: h'b0425f1120b0c131ab72af780c67564c0340de7049ea3580',
+        99_0: h'4ab319add5bfe22094db37633d4d7ab3aa5d3002f8f3471f',
+        114_0: h'affc07ce6c37c4fc0442c25b05048afcf175156ace6bcead',
+        121_0: h'9602973f461003f36f18b1ccb468d352b1f1a711efc1e963',
+        148_0: h'fdf2e4f46197ca0b24b8461d8d83e63deedddbd3af0b6886',
+        154_0: h'5bd84f5aa8c02d35d684a052efc14fbf01aa946496120b18',
+        166_0: h'd7cc20cdaabd6f00dcd5eea4d9674d4e71eefadc66bded88',
+        178_0: h'3fd1810476df8f9e3bce3224af62ac29973ae1154e0f794c',
+        187_0: h'bc14df3d4ab8b50233aa6d692ed9cd05e44953c11310f7b1',
+        201_0: h'6519a416494a98ae06ff1f9ae592d7538f994d4afe68cfbd',
+        212_0: h'e57fcaad6bbe82b187d1b2f96b4cd384abe9243e017b9d89',
+    },
+    "set_len": 16,
+    "version": 53846_1(h'423041'),
+    "max_version_bytes": 3,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 340 wire bytes
-  frame 0 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 3 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'4642d0') / causal version, event tree: (0, 11, 0) /
-          10
-        >>)
-      >>)
-    ]
-  frame 4 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 5 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'4f19') / causal version, event tree: (0, 6, 0) /
-          5
-        >>)
-      >>)
-    ]
-  frame 6 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'454250') / causal version, event tree: (0, 9, 0) /
-          8
-        >>)
-      >>)
-    ]
-  frame 7 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 8 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'474350') / causal version, event tree: (0, 13, 0) /
-          12
-        >>)
-      >>)
-    ]
-  frame 9 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 10 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'44c210') / causal version, event tree: (0, 8, 0) /
-          7
-        >>)
-      >>)
-    ]
-  frame 11 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 12 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'5c90') / causal version, event tree: (0, 2, 0) /
-          1
-        >>)
-      >>)
-    ]
-  frame 13 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 14 (32 bytes)
-    [
-      0 / stream /
-      4 / Query(Continue) /
-      { / listing: 1 child(ren) /
-        0xaf => h'72dbeb62425da61f8402d848065fe9940504e4ec4ece6cb4' / digest /
-      }
-    ]
-  frame 15 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'4b11') / causal version, event tree: (0, 4, 0) /
-          3
-        >>)
-      >>)
-    ]
-  frame 16 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'5540') / causal version, event tree: (0, 1, 0) /
-          0
-        >>)
-      >>)
-    ]
-  frame 17 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'423041') / causal version, event tree: (0, 16, 0) /
-          15
-        >>)
-      >>)
-    ]
-  frame 18 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'4934') / causal version, event tree: (0, 3, 0) /
-          2
-        >>)
-      >>)
-    ]
-  frame 19 (32 bytes)
-    [
-      0 / stream /
-      4 / Query(Continue) /
-      { / listing: 1 child(ren) /
-        0xdd => h'e9a4403e50a62d20972cea377193ae1b569d5d1f352125af' / digest /
-      }
-    ]
-  frame 20 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 21 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'4210f4') / causal version, event tree: (0, 15, 0) /
-          14
-        >>)
-      >>)
-    ]
-  frame 22 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'4d15') / causal version, event tree: (0, 5, 0) /
-          4
-        >>)
-      >>)
-    ]
-  frame 23 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 24 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'45c290') / causal version, event tree: (0, 10, 0) /
-          9
-        >>)
-      >>)
-    ]
-  frame 25 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 26 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'46c310') / causal version, event tree: (0, 12, 0) /
-          11
-        >>)
-      >>)
-    ]
-  frame 27 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 28 (3 bytes)
-    [
-      0 / stream /
-      3 / QueryEmpty(End) /
-    ]
-  frame 29 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 1 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 2 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 3 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'4642d0'), 10>>)>>)]
+  frame 4 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 5 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'4f19'), 5>>)>>)]
+  frame 6 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'454250'), 8>>)>>)]
+  frame 7 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 8 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'474350'), 12>>)>>)]
+  frame 9 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 10 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'44c210'), 7>>)>>)]
+  frame 11 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 12 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'5c90'), 1>>)>>)]
+  frame 13 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 14 (32 bytes) / Query(Continue) /
+[
+    0,
+    4,
+    {
+        175_0: h'72dbeb62425da61f8402d848065fe9940504e4ec4ece6cb4',
+    },
+]
+  frame 15 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'4b11'), 3>>)>>)]
+  frame 16 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'5540'), 0>>)>>)]
+  frame 17 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'423041'), 15>>)>>)]
+  frame 18 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'4934'), 2>>)>>)]
+  frame 19 (32 bytes) / Query(Continue) /
+[
+    0,
+    4,
+    {
+        221_0: h'e9a4403e50a62d20972cea377193ae1b569d5d1f352125af',
+    },
+]
+  frame 20 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 21 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'4210f4'), 14>>)>>)]
+  frame 22 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'4d15'), 4>>)>>)]
+  frame 23 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 24 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'45c290'), 9>>)>>)]
+  frame 25 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 26 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'46c310'), 11>>)>>)]
+  frame 27 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 28 (3 bytes) / QueryEmpty(End) /
+[0, 3]
+  frame 29 (3 bytes) / End(Stream) /
+[0, 9]
 Responder stream 1 (height 29), epoch 0, 39 wire bytes
-  frame 0 (17 bytes)
-    [
-      1 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'444740') / causal version, event tree: (0, 7, 0) /
-          6
-        >>)
-      >>)
-    ]
-  frame 1 (17 bytes)
-    [
-      1 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'47c390') / causal version, event tree: (0, 14, 0) /
-          13
-        >>)
-      >>)
-    ]
-  frame 2 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (17 bytes) / Supply(End) /
+[1, 7, 63_0(<<63_0(<<53846_1(h'444740'), 6>>)>>)]
+  frame 1 (17 bytes) / Supply(End) /
+[1, 7, 63_0(<<63_0(<<53846_1(h'47c390'), 13>>)>>)]
+  frame 2 (3 bytes) / End(Stream) /
+[1, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (521 bytes) / greeting /
-  24(<< / embedded item, 516 bytes /
-    {
-      "listing" =>
-        { / listing: 15 child(ren) /
-          0x2 => h'98ce6b5c36fe7a0218a33c8325d312de40ad00956663fcef' / digest /
-          0x3 => h'0e16e3ba46d66cc627ebbf7e4c53edc3252b612f19451269' / digest /
-          0x6 => h'e62c1736212d7818ba1ef6e117236c79c3c52ba6756262ae' / digest /
-          0x10 => h'76673b9254ccdafd7fe01a59a0702a0108c6ea0c201c017f' / digest /
-          0x2a => h'373cb2db2e49895b3f53deed27c14c831b1a41e2c2ee9be8' / digest /
-          0x36 => h'baa315cab6fa94b0dd4c6d19e249f3bf33dba55c43593cc9' / digest /
-          0x39 => h'131331730f28a4b4ba24c81f2e246f846a3d437f526c68ba' / digest /
-          0x62 => h'73d8e46b8bdf6fe2d231fe4eb520cba7fe545ff1f2bb25c7' / digest /
-          0x63 => h'd37d5a66a342164998f89098b66e1014ee757e14c9d7a41c' / digest /
-          0xa6 => h'cb9d7d244a3720ea5ae6d7984b229a6b1e95c6b708e2c5a0' / digest /
-          0xad => h'c427db6fe92e923509619eb51d8a601e31b985ac24a3baa2' / digest /
-          0xc7 => h'075fc1551304ec1fcd0a5039357ff30878164317327f1906' / digest /
-          0xca => h'1950ba7afb798fce15b3607511216a7ba874ade0e9f1d719' / digest /
-          0xe8 => h'a82ff6a5e3f64d4901f633ca61b549cfa54be033845641db' / digest /
-          0xf1 => h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2' / digest /
-        }
-      "set_len" => 16
-      "version" =>
-        53846(h'7043') / causal version, event tree: (0, 0, 16) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        2: h'98ce6b5c36fe7a0218a33c8325d312de40ad00956663fcef',
+        3: h'0e16e3ba46d66cc627ebbf7e4c53edc3252b612f19451269',
+        6: h'e62c1736212d7818ba1ef6e117236c79c3c52ba6756262ae',
+        16: h'76673b9254ccdafd7fe01a59a0702a0108c6ea0c201c017f',
+        42_0: h'373cb2db2e49895b3f53deed27c14c831b1a41e2c2ee9be8',
+        54_0: h'baa315cab6fa94b0dd4c6d19e249f3bf33dba55c43593cc9',
+        57_0: h'131331730f28a4b4ba24c81f2e246f846a3d437f526c68ba',
+        98_0: h'73d8e46b8bdf6fe2d231fe4eb520cba7fe545ff1f2bb25c7',
+        99_0: h'd37d5a66a342164998f89098b66e1014ee757e14c9d7a41c',
+        166_0: h'cb9d7d244a3720ea5ae6d7984b229a6b1e95c6b708e2c5a0',
+        173_0: h'c427db6fe92e923509619eb51d8a601e31b985ac24a3baa2',
+        199_0: h'075fc1551304ec1fcd0a5039357ff30878164317327f1906',
+        202_0: h'1950ba7afb798fce15b3607511216a7ba874ade0e9f1d719',
+        232_0: h'a82ff6a5e3f64d4901f633ca61b549cfa54be033845641db',
+        241_0: h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2',
+    },
+    "set_len": 16,
+    "version": 53846_1(h'7043'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Initiator stream 0 (height 31), epoch 0, 229 wire bytes
-  frame 0 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'70cc') / causal version, event tree: (0, 0, 12) /
-          27
-        >>)
-      >>)
-    ]
-  frame 1 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'70ec') / causal version, event tree: (0, 0, 14) /
-          29
-        >>)
-      >>)
-    ]
-  frame 2 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'7130') / causal version, event tree: (0, 0, 4) /
-          19
-        >>)
-      >>)
-    ]
-  frame 3 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'708c') / causal version, event tree: (0, 0, 8) /
-          23
-        >>)
-      >>)
-    ]
-  frame 4 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'70ac') / causal version, event tree: (0, 0, 10) /
-          25
-        >>)
-      >>)
-    ]
-  frame 5 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'7170') / causal version, event tree: (0, 0, 5) /
-          20
-        >>)
-      >>)
-    ]
-  frame 6 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'70dc') / causal version, event tree: (0, 0, 13) /
-          28
-        >>)
-      >>)
-    ]
-  frame 7 (28 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 2 record(s), 22 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'70fc') / causal version, event tree: (0, 0, 15) /
-          30
-        >>)
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'709c') / causal version, event tree: (0, 0, 9) /
-          24
-        >>)
-      >>)
-    ]
-  frame 8 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'72c0') / causal version, event tree: (0, 0, 2) /
-          17
-        >>)
-      >>)
-    ]
-  frame 9 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'71b0') / causal version, event tree: (0, 0, 6) /
-          21
-        >>)
-      >>)
-    ]
-  frame 10 (17 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'70bc') / causal version, event tree: (0, 0, 11) /
-          26
-        >>)
-      >>)
-    ]
-  frame 11 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'73c0') / causal version, event tree: (0, 0, 3) /
-          18
-        >>)
-      >>)
-    ]
-  frame 12 (15 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 9 bytes /
-        63(<< / record, 2 item(s), 6 bytes /
-          53846(h'77') / causal version, event tree: (0, 0, 1) /
-          16
-        >>)
-      >>)
-    ]
-  frame 13 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'70cc'), 27_0>>)>>)]
+  frame 1 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'70ec'), 29_0>>)>>)]
+  frame 2 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'7130'), 19>>)>>)]
+  frame 3 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'708c'), 23>>)>>)]
+  frame 4 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'70ac'), 25_0>>)>>)]
+  frame 5 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'7170'), 20>>)>>)]
+  frame 6 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'70dc'), 28_0>>)>>)]
+  frame 7 (28 bytes) / Supply(Continue) /
+[
+    0,
+    6,
+    63_0(<<63_0(<<53846_1(h'70fc'), 30_0>>), 63_0(<<53846_1(h'709c'), 24_0>>)>>),
+]
+  frame 8 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'72c0'), 17>>)>>)]
+  frame 9 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'71b0'), 21>>)>>)]
+  frame 10 (17 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'70bc'), 26_0>>)>>)]
+  frame 11 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'73c0'), 18>>)>>)]
+  frame 12 (15 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'77'), 16>>)>>)]
+  frame 13 (3 bytes) / End(Stream) /
+[0, 9]
 Initiator stream 1 (height 30), epoch 0, 83 wire bytes
-  frame 0 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 1 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 2 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 3 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 4 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 5 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 6 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 7 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 8 (17 bytes)
-    [
-      1 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 11 bytes /
-        63(<< / record, 2 item(s), 8 bytes /
-          53846(h'7043') / causal version, event tree: (0, 0, 16) /
-          31
-        >>)
-      >>)
-    ]
-  frame 9 (3 bytes)
-    [
-      1 / stream /
-      3 / QueryEmpty(End) /
-    ]
-  frame 10 (16 bytes)
-    [
-      1 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'71f0') / causal version, event tree: (0, 0, 7) /
-          22
-        >>)
-      >>)
-    ]
-  frame 11 (3 bytes)
-    [
-      1 / stream /
-      3 / QueryEmpty(End) /
-    ]
-  frame 12 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 13 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 14 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 15 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 16 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 17 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 1 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 2 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 3 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 4 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 5 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 6 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 7 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 8 (17 bytes) / Supply(Continue) /
+[1, 6, 63_0(<<63_0(<<53846_1(h'7043'), 31_0>>)>>)]
+  frame 9 (3 bytes) / QueryEmpty(End) /
+[1, 3]
+  frame 10 (16 bytes) / Supply(Continue) /
+[1, 6, 63_0(<<63_0(<<53846_1(h'71f0'), 22>>)>>)]
+  frame 11 (3 bytes) / QueryEmpty(End) /
+[1, 3]
+  frame 12 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 13 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 14 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 15 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 16 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 17 (3 bytes) / End(Stream) /
+[1, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-34"></a>
### tests/snapshots/gossip_snapshot__early_supplies_honor_redactions.snap `@@ -5,131 +5,68 @@ expression: capture`

```diff
@@ -5,131 +5,68 @@ expression: capture
 direction A -> B
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (137 bytes) / greeting /
-  24(<< / embedded item, 133 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x8e => h'3ddb8ec8897a61fed8827fda50bea5f4ec959f2ab9cf4826' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'4002003000fff4') / causal version, event tree: (1, 4095, 0) /
-      "max_version_bytes" => 5
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        142_0: h'3ddb8ec8897a61fed8827fda50bea5f4ec959f2ab9cf4826',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'4002003000fff4'),
+    "max_version_bytes": 5,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Initiator stream 0 (height 31), epoch 0, 25 wire bytes
-  frame 0 (20 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 14 bytes /
-        63(<< / record, 2 item(s), 11 bytes /
-          53846(h'4073403910') / causal version, event tree: (1, 228, 0) /
-          229
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (20 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'4073403910'), 229_0>>)>>)]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 Initiator stream 1 (height 30), epoch 0, 8 wire bytes
-  frame 0 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 1 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 1 (3 bytes) / End(Stream) /
+[1, 9]
 
 direction B -> A
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (189 bytes) / greeting /
-  24(<< / embedded item, 185 bytes /
-    {
-      "listing" =>
-        { / listing: 3 child(ren) /
-          0x1e => h'ea962c263b019bd428a49dbac092f24ffb46a14a869077e0' / digest /
-          0x21 => h'4e79398dd559d56b1b8353d7724c44d431918c96b4876923' / digest /
-          0x8f => h'26ae24b07cd93ba2329a78559e9b77e820f8a3e9b99d928a' / digest /
-        }
-      "set_len" => 3
-      "version" =>
-        53846(h'541ec0') / causal version, event tree: (1, 0, 30) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        30_0: h'ea962c263b019bd428a49dbac092f24ffb46a14a869077e0',
+        33_0: h'4e79398dd559d56b1b8353d7724c44d431918c96b4876923',
+        143_0: h'26ae24b07cd93ba2329a78559e9b77e820f8a3e9b99d928a',
+    },
+    "set_len": 3,
+    "version": 53846_1(h'541ec0'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 62 wire bytes
-  frame 0 (18 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 12 bytes /
-        63(<< / record, 2 item(s), 9 bytes /
-          53846(h'54b0') / causal version, event tree: (1, 0, 2) /
-          10000
-        >>)
-      >>)
-    ]
-  frame 1 (18 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 12 bytes /
-        63(<< / record, 2 item(s), 9 bytes /
-          53846(h'54f0') / causal version, event tree: (1, 0, 3) /
-          10001
-        >>)
-      >>)
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 3 (18 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 12 bytes /
-        63(<< / record, 2 item(s), 9 bytes /
-          53846(h'544c') / causal version, event tree: (1, 0, 4) /
-          10002
-        >>)
-      >>)
-    ]
-  frame 4 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (18 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'54b0'), 10000_1>>)>>)]
+  frame 1 (18 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'54f0'), 10001_1>>)>>)]
+  frame 2 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 3 (18 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'544c'), 10002_1>>)>>)]
+  frame 4 (3 bytes) / End(Stream) /
+[0, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-35"></a>
### tests/snapshots/gossip_snapshot__empty_pair_converges_immediately.snap `@@ -4,54 +4,40 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -4,54 +4,40 @@ expression: "capture_gossip(a, b)"
 ---
 direction A -> B
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 
 direction B -> A
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-36"></a>
### tests/snapshots/gossip_snapshot__fork_insert_redact.snap `@@ -5,119 +5,68 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,119 +5,68 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (159 bytes) / greeting /
-  24(<< / embedded item, 155 bytes /
-    {
-      "listing" =>
-        { / listing: 2 child(ren) /
-          0x8 => h'711c107a02ffed39979bcdd86654134ee8bb931687a03a29' / digest /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'4b24') / causal version, event tree: (2, 2, 0) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        8: h'711c107a02ffed39979bcdd86654134ee8bb931687a03a29',
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'4b24'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 27 wire bytes
-  frame 0 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'4950') / causal version, event tree: (2, 1, 0) /
-          3
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      3 / QueryEmpty(End) /
-    ]
-  frame 3 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'4950'), 3>>)>>)]
+  frame 1 (3 bytes) / QueryEmpty(Continue) /
+[0, 2]
+  frame 2 (3 bytes) / QueryEmpty(End) /
+[0, 3]
+  frame 3 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (160 bytes) / greeting /
-  24(<< / embedded item, 156 bytes /
-    {
-      "listing" =>
-        { / listing: 2 child(ren) /
-          0x8e => h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32' / digest /
-          0x98 => h'387ecab186fe71c89d5894b895f5917b7549e591ce2f098c' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'5cb0') / causal version, event tree: (2, 0, 2) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        142_0: h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32',
+        152_0: h'387ecab186fe71c89d5894b895f5917b7549e591ce2f098c',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'5cb0'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Initiator stream 0 (height 31), epoch 0, 21 wire bytes
-  frame 0 (16 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'5dc0') / causal version, event tree: (2, 0, 1) /
-          4
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (16 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'5dc0'), 4>>)>>)]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 Initiator stream 1 (height 30), epoch 0, 11 wire bytes
-  frame 0 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 1 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 2 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 1 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 2 (3 bytes) / End(Stream) /
+[1, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-37"></a>
### tests/snapshots/gossip_snapshot__one_sided_transfer.snap `@@ -5,85 +5,51 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,85 +5,51 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (160 bytes) / greeting /
-  24(<< / embedded item, 156 bytes /
-    {
-      "listing" =>
-        { / listing: 2 child(ren) /
-          0x52 => h'b0425f1120b0c131ab72af780c67564c0340de7049ea3580' / digest /
-          0x79 => h'9602973f461003f36f18b1ccb468d352b1f1a711efc1e963' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'5c90') / causal version, event tree: (0, 2, 0) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        82_0: h'b0425f1120b0c131ab72af780c67564c0340de7049ea3580',
+        121_0: h'9602973f461003f36f18b1ccb468d352b1f1a711efc1e963',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'5c90'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 37 wire bytes
-  frame 0 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'5c90') / causal version, event tree: (0, 2, 0) /
-          2
-        >>)
-      >>)
-    ]
-  frame 1 (16 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'5540') / causal version, event tree: (0, 1, 0) /
-          1
-        >>)
-      >>)
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'5c90'), 2>>)>>)]
+  frame 1 (16 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'5540'), 1>>)>>)]
+  frame 2 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 5:
>
> All 22 wire snapshots under tests/snapshots move, re-accepted with `cargo insta accept` in this commit, the sanctioned case T138 names (this row stands for the set; the commit message lists every file). Framing lines are unchanged; every item body is now cbor-diag's pretty notation verbatim, so a re-accept diff shows the same values in the notation's spelling (encoding indicators, `<<...>>` unfolds) and without the dropped glosses. No snapshot outside tests/snapshots moved.

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-38"></a>
### tests/snapshots/gossip_snapshot__redaction_only.snap `@@ -5,69 +5,51 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,69 +5,51 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (132 bytes) / greeting /
-  24(<< / embedded item, 128 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'4950') / causal version, event tree: (2, 1, 0) /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'4950'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 
 direction B -> A
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (159 bytes) / greeting /
-  24(<< / embedded item, 155 bytes /
-    {
-      "listing" =>
-        { / listing: 2 child(ren) /
-          0x82 => h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637' / digest /
-          0x8e => h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'b8') / causal version, event tree: 2 /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        130_0: h'ad151bc8bfcf8e311bf94802769422676fec3e83d495d637',
+        142_0: h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'b8'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 8 wire bytes
-  frame 0 (3 bytes)
-    [
-      0 / stream /
-      1 / Match(End) /
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / Match(End) /
+[0, 1]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-39"></a>
### tests/snapshots/gossip_snapshot__same_live_content_divergent_versions.snap `@@ -5,68 +5,50 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,68 +5,50 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (132 bytes) / greeting /
-  24(<< / embedded item, 128 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x8e => h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'4924') / causal version, event tree: (1, 2, 0) /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        142_0: h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'4924'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 8 wire bytes
-  frame 0 (3 bytes)
-    [
-      0 / stream /
-      1 / Match(End) /
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / Match(End) /
+[0, 1]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (131 bytes) / greeting /
-  24(<< / embedded item, 127 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x8e => h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'a8') / causal version, event tree: 1 /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        142_0: h'1b5c7a2a860aa40597f61b26a1d3d54b48591068e45aed32',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'a8'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-40"></a>
### tests/snapshots/gossip_snapshot__shared_subtree_dispute_pins_a_nonempty_query.snap `@@ -5,111 +5,72 @@ expression: capture`

```diff
@@ -5,111 +5,72 @@ expression: capture
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (137 bytes) / greeting /
-  24(<< / embedded item, 133 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0xb8 => h'78de0c248cc1ab42a1fe3e70e760f0ea747f209e524f32c7' / digest /
-        }
-      "set_len" => 3
-      "version" =>
-        53846(h'40020fd000fff4') / causal version, event tree: (126, 4095, 0) /
-      "max_version_bytes" => 5
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        184_0: h'78de0c248cc1ab42a1fe3e70e760f0ea747f209e524f32c7',
+    },
+    "set_len": 3,
+    "version": 53846_1(h'40020fd000fff4'),
+    "max_version_bytes": 5,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 93 wire bytes
-  frame 0 (88 bytes)
-    [
-      0 / stream /
-      5 / Query(End) /
-      { / listing: 3 child(ren) /
-        0x56 => h'1faf8fb07589a94097c9fa09c5bebb255e91d14a1abbf9df' / digest /
-        0x93 => h'038cec0d20f5d63d0164683a443bf231d8c9bf9c3c22178a' / digest /
-        0x9b => h'b8ff09282ace2f376e1bb6c511455a2161893a7dc46345ea' / digest /
-      }
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (88 bytes) / Query(End) /
+[
+    0,
+    5,
+    {
+        86_0: h'1faf8fb07589a94097c9fa09c5bebb255e91d14a1abbf9df',
+        147_0: h'038cec0d20f5d63d0164683a443bf231d8c9bf9c3c22178a',
+        155_0: h'b8ff09282ace2f376e1bb6c511455a2161893a7dc46345ea',
+    },
+]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 Responder stream 1 (height 29), epoch 0, 26 wire bytes
-  frame 0 (21 bytes)
-    [
-      1 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 15 bytes /
-        63(<< / record, 2 item(s), 12 bytes /
-          53846(h'4038d0051d') / causal version, event tree: (126, 327, 0) /
-          426
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (21 bytes) / Supply(End) /
+[1, 7, 63_0(<<63_0(<<53846_1(h'4038d0051d'), 426_1>>)>>)]
+  frame 1 (3 bytes) / End(Stream) /
+[1, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (132 bytes) / greeting /
-  24(<< / embedded item, 128 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0xb8 => h'637f3315fe244bda1d9b0c308c4a16ea93e7df24699aa713' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'81fe') / causal version, event tree: 126 /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        184_0: h'637f3315fe244bda1d9b0c308c4a16ea93e7df24699aa713',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'81fe'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Initiator stream 1 (height 30), epoch 0, 14 wire bytes
-  frame 0 (3 bytes)
-    [
-      1 / stream /
-      0 / Match(Continue) /
-    ]
-  frame 1 (3 bytes)
-    [
-      1 / stream /
-      2 / QueryEmpty(Continue) /
-    ]
-  frame 2 (3 bytes)
-    [
-      1 / stream /
-      1 / Match(End) /
-    ]
-  frame 3 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / Match(Continue) /
+[1, 0]
+  frame 1 (3 bytes) / QueryEmpty(Continue) /
+[1, 2]
+  frame 2 (3 bytes) / Match(End) /
+[1, 1]
+  frame 3 (3 bytes) / End(Stream) /
+[1, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-41"></a>
### tests/snapshots/gossip_snapshot__string_payload.snap `@@ -5,107 +5,62 @@ expression: "capture_gossip(a, b)"`

```diff
@@ -5,107 +5,62 @@ expression: "capture_gossip(a, b)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (132 bytes) / greeting /
-  24(<< / embedded item, 128 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x79 => h'9602973f461003f36f18b1ccb468d352b1f1a711efc1e963' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'5540') / causal version, event tree: (0, 1, 0) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        121_0: h'9602973f461003f36f18b1ccb468d352b1f1a711efc1e963',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'5540'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 29 wire bytes
-  frame 0 (21 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 15 bytes /
-        63(<< / record, 2 item(s), 12 bytes /
-          53846(h'5540') / causal version, event tree: (0, 1, 0) /
-          "hello"
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      3 / QueryEmpty(End) /
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (21 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'5540'), "hello">>)>>)]
+  frame 1 (3 bytes) / QueryEmpty(End) /
+[0, 3]
+  frame 2 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (131 bytes) / greeting /
-  24(<< / embedded item, 127 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0xf1 => h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'77') / causal version, event tree: (0, 0, 1) /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        241_0: h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'77'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Initiator stream 0 (height 31), epoch 0, 25 wire bytes
-  frame 0 (20 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 14 bytes /
-        63(<< / record, 2 item(s), 11 bytes /
-          53846(h'77') / causal version, event tree: (0, 0, 1) /
-          "world"
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (20 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'77'), "world">>)>>)]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 Initiator stream 1 (height 30), epoch 0, 8 wire bytes
-  frame 0 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 1 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 1 (3 bytes) / End(Stream) /
+[1, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-42"></a>
### tests/snapshots/retire_snapshot__divergent_retire.snap `@@ -5,109 +5,64 @@ expression: "capture_retire(seed, retiree)"`

```diff
@@ -5,109 +5,64 @@ expression: "capture_retire(seed, retiree)"
 direction A -> B
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (132 bytes) / greeting /
-  24(<< / embedded item, 128 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0x79 => h'9602973f461003f36f18b1ccb468d352b1f1a711efc1e963' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'5540') / causal version, event tree: (0, 1, 0) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        121_0: h'9602973f461003f36f18b1ccb468d352b1f1a711efc1e963',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'5540'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 24 wire bytes
-  frame 0 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'5540') / causal version, event tree: (0, 1, 0) /
-          2
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      3 / QueryEmpty(End) /
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'5540'), 2>>)>>)]
+  frame 1 (3 bytes) / QueryEmpty(End) /
+[0, 3]
+  frame 2 (3 bytes) / End(Stream) /
+[0, 9]
 
 direction B -> A
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      1
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    1,
+])
 control item 1 (131 bytes) / greeting /
-  24(<< / embedded item, 127 bytes /
-    {
-      "listing" =>
-        { / listing: 1 child(ren) /
-          0xf1 => h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2' / digest /
-        }
-      "set_len" => 1
-      "version" =>
-        53846(h'77') / causal version, event tree: (0, 0, 1) /
-      "max_version_bytes" => 1
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        241_0: h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2',
+    },
+    "set_len": 1,
+    "version": 53846_1(h'77'),
+    "max_version_bytes": 1,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (5 bytes) / party hand-off /
-  53845(h'48') / party /
+53845_1(h'48')
 control item 3 (2 bytes) / epilogue /
-  "."
+"."
 Initiator stream 0 (height 31), epoch 0, 20 wire bytes
-  frame 0 (15 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 9 bytes /
-        63(<< / record, 2 item(s), 6 bytes /
-          53846(h'77') / causal version, event tree: (0, 0, 1) /
-          1
-        >>)
-      >>)
-    ]
-  frame 1 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (15 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'77'), 1>>)>>)]
+  frame 1 (3 bytes) / End(Stream) /
+[0, 9]
 Initiator stream 1 (height 30), epoch 0, 8 wire bytes
-  frame 0 (3 bytes)
-    [
-      1 / stream /
-      8 / End(Reply) /
-    ]
-  frame 1 (3 bytes)
-    [
-      1 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (3 bytes) / End(Reply) /
+[1, 8]
+  frame 1 (3 bytes) / End(Stream) /
+[1, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-43"></a>
### tests/snapshots/retire_snapshot__empty_retire.snap `@@ -4,56 +4,42 @@ expression: "capture_retire(seed, retiree)"`

```diff
@@ -4,56 +4,42 @@ expression: "capture_retire(seed, retiree)"
 ---
 direction A -> B
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 
 direction B -> A
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      1
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    1,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (5 bytes) / party hand-off /
-  53845(h'48') / party /
+53845_1(h'48')
 control item 3 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-44"></a>
### tests/snapshots/retire_snapshot__mutual_retire_declines.snap `@@ -4,26 +4,22 @@ expression: capture`

```diff
@@ -4,26 +4,22 @@ expression: capture
 ---
 direction A -> B
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      1
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    1,
+])
 control item 1 (2 bytes) / epilogue /
-  "."
+"."
 
 direction B -> A
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      1
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    1,
+])
 control item 1 (2 bytes) / epilogue /
-  "."
+"."
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-45"></a>
### tests/snapshots/retire_snapshot__retire_into_bootstrapper.snap `@@ -5,87 +5,53 @@ expression: capture`

```diff
@@ -5,87 +5,53 @@ expression: capture
 direction A -> B
 role: Initiator
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'00000000000000000000000000000000'
-      0
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'00000000000000000000000000000000',
+    0,
+])
 control item 1 (103 bytes) / greeting /
-  24(<< / embedded item, 99 bytes /
-    {
-      "listing" =>
-        { / listing: 0 child(ren) /
-        }
-      "set_len" => 0
-      "version" =>
-        53846(h'e0') / causal version, event tree: 0 /
-      "max_version_bytes" => 0
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {},
+    "set_len": 0,
+    "version": 53846_1(h'e0'),
+    "max_version_bytes": 0,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (2 bytes) / epilogue /
-  "."
+"."
 
 direction B -> A
 role: Responder
 control item 0 (30 bytes) / preamble /
-  55799( / self-described CBOR /
-    [
-      "rumors"
-      2
-      h'8c189668c9e4837237bf31e02d7f6b70'
-      1
-    ]
-  )
+55799_1([
+    "rumors",
+    2,
+    h'8c189668c9e4837237bf31e02d7f6b70',
+    1,
+])
 control item 1 (160 bytes) / greeting /
-  24(<< / embedded item, 156 bytes /
-    {
-      "listing" =>
-        { / listing: 2 child(ren) /
-          0xad => h'c427db6fe92e923509619eb51d8a601e31b985ac24a3baa2' / digest /
-          0xf1 => h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2' / digest /
-        }
-      "set_len" => 2
-      "version" =>
-        53846(h'72c0') / causal version, event tree: (0, 0, 2) /
-      "max_version_bytes" => 2
-      "payload_depth_limit" => 256
-      "target_message_size" => 1830400
-    }
-  >>)
+24_0(<<{
+    "listing": {
+        173_0: h'c427db6fe92e923509619eb51d8a601e31b985ac24a3baa2',
+        241_0: h'871321571b8582637fc4d52dc6df1c0f865df999eaab42f2',
+    },
+    "set_len": 2,
+    "version": 53846_1(h'72c0'),
+    "max_version_bytes": 2,
+    "payload_depth_limit": 256_1,
+    "target_message_size": 1830400_2,
+}>>)
 control item 2 (5 bytes) / party hand-off /
-  53845(h'48') / party /
+53845_1(h'48')
 control item 3 (2 bytes) / epilogue /
-  "."
+"."
 Responder stream 0 (height 31), epoch 0, 36 wire bytes
-  frame 0 (16 bytes)
-    [
-      0 / stream /
-      6 / Supply(Continue) /
-      63(<< / supply run, 1 record(s), 10 bytes /
-        63(<< / record, 2 item(s), 7 bytes /
-          53846(h'72c0') / causal version, event tree: (0, 0, 2) /
-          2
-        >>)
-      >>)
-    ]
-  frame 1 (15 bytes)
-    [
-      0 / stream /
-      7 / Supply(End) /
-      63(<< / supply run, 1 record(s), 9 bytes /
-        63(<< / record, 2 item(s), 6 bytes /
-          53846(h'77') / causal version, event tree: (0, 0, 1) /
-          1
-        >>)
-      >>)
-    ]
-  frame 2 (3 bytes)
-    [
-      0 / stream /
-      9 / End(Stream) /
-    ]
+  frame 0 (16 bytes) / Supply(Continue) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'72c0'), 2>>)>>)]
+  frame 1 (15 bytes) / Supply(End) /
+[0, 7, 63_0(<<63_0(<<53846_1(h'77'), 1>>)>>)]
+  frame 2 (3 bytes) / End(Stream) /
+[0, 9]
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 0:
>
> T138 re-accept: this file's diff is the whole rendering, since every item body now stands in cbor-diag's diagnostic notation verbatim under its header and the dropped glosses (decoded version and party, listing child counts, embedded byte counts) no longer appear. Framing lines (direction, role, control-item and frame headers with their byte counts, stream headers) are unchanged, so the same wire bytes are pinned in the notation's spelling. Re-accepted with `cargo insta accept` in one commit naming T138.

<a id="hunk-46"></a>
### tests/target_message_size.rs `@@ -119,14 +119,16 @@ fn supply_frames(capture: &str) -> usize {`

```diff
@@ -119,14 +119,16 @@ fn supply_frames(capture: &str) -> usize {
         .count()
 }
 
-/// Whether one rendered line is a supply frame's state line: a bare
-/// state code followed by a `/ Supply… /` comment. The bare-digit code
-/// and the comment distinguish it from every other annotated line.
+/// Whether one rendered line is a supply frame's header: the
+/// `frame N (B bytes)` line whose label comment names a `Supply` signal.
+///
+/// The `frame` prefix and the comment distinguish it from every other
+/// line: the renderer guarantees no body line begins with `frame `.
 fn is_supply_signal(line: &str) -> bool {
-    let Some((code, rest)) = line.trim_start().split_once(" / ") else {
+    let Some((head, rest)) = line.trim_start().split_once(" / ") else {
         return false;
     };
-    !code.is_empty() && code.bytes().all(|b| b.is_ascii_digit()) && rest.starts_with("Supply")
+    head.starts_with("frame ") && rest.starts_with("Supply")
 }
 
 /// Count the supply frames in each direction of a rendered wire capture:
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 127:
>
> Consequence T138 did not name: this suite counted supply frames by the hand renderer's annotated state line. It now counts `frame N (B bytes) / Supply... /` header lines, the label's home. Prose finding, not edited: paragraphs elsewhere in this file carry em-dashes (lines 69, 161, 203, 217, 318), outside what this commit touches.

<a id="hunk-47"></a>
### tools/digestshare `@@ -4,9 +4,12 @@`

```diff
@@ -4,9 +4,12 @@
 Reads the V2 capture snapshots (`tests/snapshots/*.snap`) and totals, per
 file and overall: every captured wire byte (each direction's outgoing
 bytes, counted once, from the renderer's exact byte-count headers), the
-bytes spent on Merkle digests (the renderer annotates every listing and
-query child as `0x<radix> => h'<digest hex>' / digest /`, so the digest
-width is read from the corpus itself, never assumed), and their ratio.
+bytes spent on Merkle digests (a listing renders as a map from radix to
+digest, `<radix>: h'<digest hex>'`; the scan reads such entries only
+inside the greeting control item and Query frame bodies, the two places
+a listing appears and neither of which carries a payload, so a payload
+map of the same shape inside a Supply frame is never counted; the width
+is read from the corpus, never assumed), and their ratio.
 This is the denominator for any claim about what a digest width costs on
 the wire: dispute-listing metadata is digest-dominated, and this meter
 says by exactly how much, over the corpus the snapshot suite already pins
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 4:
>
> Docstring restated for the diagnostic-notation corpus and this lane's scan rules: what a listing entry looks like, that the scan is bounded to the greeting and Query bodies, and that no side-by-side render form exists to skip.

<a id="hunk-48"></a>
### tools/digestshare `@@ -17,16 +20,17 @@ is double-counted: `control item N (B bytes)` lines (one per control-`

```diff
@@ -17,16 +20,17 @@ is double-counted: `control item N (B bytes)` lines (one per control-
 stream item) and stream headers ending `..., B wire bytes` (whose total
 includes the stream's open label and every frame).
 
-V1 side-by-side timelines (renders containing the `│` column separator)
-are skipped and reported as such: their transcripts show each byte twice
-(sent and received), so totaling them would double-count; the V1 digest
-atom is the same `Hash` the V2 corpus measures.
-
-Liveness: a corpus with V2 captures but zero wire bytes or zero digests
+Liveness: a corpus with captures but zero wire bytes or zero digests
 means the renderer's vocabulary moved out from under these patterns; the
-tool exits nonzero rather than reporting a silent zero.
+tool exits nonzero rather than reporting a silent zero. The self-test
+holds the patterns to a listing laid out on one line, to a Supply
+payload that must not count, and to a payload text carrying U+2028 (a
+line separator the renderer prints raw; lines are split on the
+renderer's own terminator, the line feed, so such text cannot open a
+section or end one).
 
 Usage: tools/digestshare [snapshot-dir ...]
+       tools/digestshare --self-test
 Defaults to tests/snapshots relative to the repository root.
 """
 
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 20:
>
> Docstring: the wire-byte header forms are unchanged; the liveness paragraph names the self-test's fixtures (one-line listing, Supply payload of digest shape, U+2028 in payload text) so a reader knows what the floor is held to.

<a id="hunk-49"></a>
### tools/digestshare `@@ -35,22 +39,39 @@ import sys`

```diff
@@ -35,22 +39,39 @@ import sys
 from pathlib import Path
 
 CONTROL_ITEM = re.compile(r"^control item \d+ \((\d+) bytes\)")
-STREAM_HEADER = re.compile(r"^\S.*, (\d+) wire bytes$")
-DIGEST_LINE = re.compile(r"=> h'([0-9a-f]+)' / digest /$")
+STREAM_HEADER = re.compile(r"^\S+ stream \d+ \(height \d+\), epoch \d+, (\d+) wire bytes$")
+# A listing entry wherever the layout puts it: an unsigned-integer key
+# (its encoding indicator, if any, included) and a byte-string value,
+# preceded by the line start, a brace, or a comma.
+DIGEST_ENTRY = re.compile(r"(?:^|[{,])\s*\d+(?:_\d)?: h'([0-9a-f]+)'")
+# The two sections whose bodies hold listings.
+GREETING_HEADER = re.compile(r"^control item \d+ \(\d+ bytes\) / greeting /$")
+QUERY_HEADER = re.compile(r"^\s+frame \d+ \(\d+ bytes\) / Query\(")
+# Any header line: it ends the section before it.
+HEADER = re.compile(
+    r"^(?:direction |role: |control item |\S+ stream \d+ \(height |\s+frame \d+ \()"
+)
 
 
 def measure(path: Path):
-    """Return (total_bytes, digest_bytes, digest_count) for one V2 snap,
-    or None for a V1 side-by-side render."""
+    """Return (total_bytes, digest_bytes, digest_count) for one snapshot."""
     text = path.read_text()
     # Strip insta's YAML header: content starts after the second `---`.
     body = text.split("---", 2)[2]
-    if "│" in body:
-        return None
+    return measure_text(body)
+
+
+def measure_text(body: str):
+    """Total the wire bytes and the digests of one rendered capture."""
     total = 0
     digest_bytes = 0
     digests = 0
-    for line in body.splitlines():
+    in_listing_section = False
+    # The renderer's line terminator is the line feed alone; splitlines()
+    # would also break on U+2028 and U+2029, which a text string may hold.
+    for line in body.split("\n"):
+        if HEADER.match(line):
+            in_listing_section = bool(GREETING_HEADER.match(line) or QUERY_HEADER.match(line))
         control = CONTROL_ITEM.match(line)
         if control:
             total += int(control.group(1))
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 48:
>
> Consequence T138 did not name: this gate leg counted digests by the `/ digest /` gloss the ruling drops, and its liveness floor would have failed the gate. It now recognizes a listing entry by shape (an unsigned-integer key with a byte-string value, the wire's only such map entry) and anchors the stream header to its full form; totals over the new corpus equal the old tool's over the old one (9260 wire B, 1824 digest B, 76 digests).

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 48:
>
> Fresh-eyes repair (item 3): all entries on a line are counted, so a listing printed on one line inside the frame array no longer undercounts silently.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 50:
>
> Fresh-eyes repair (item 4): the shape rule alone was unsound for a payload map of uint keys and byte-string values inside a Supply frame; the scan is now bounded to the greeting control item and Query frame bodies, tracked by the same headers the snapshot suite reads, and the tool's doc states that rule.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 72:
>
> Fresh-eyes repair, round 2 (item 2): splitlines() also breaks on U+2028 and U+2029, which are not control characters, so the walk passes them and cbor-diag prints them raw; the Rust scanners split on the line feed alone, and the tool now does too, so the renderer's header guarantee holds for it.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 57:
>
> Fresh-eyes repair, round 2 (item 4): the `│` skip referred to a side-by-side render form no snapshot has carried since the base, and a payload containing `│` would have dropped a whole capture from the meter silently; the skip and its docstring paragraph are deleted, and the liveness wording no longer says V2.

<a id="hunk-50"></a>
### tools/digestshare `@@ -59,24 +80,64 @@ def measure(path: Path):`

```diff
@@ -59,24 +80,64 @@ def measure(path: Path):
         if stream:
             total += int(stream.group(1))
             continue
-        digest = DIGEST_LINE.search(line)
-        if digest:
-            digest_bytes += len(digest.group(1)) // 2
-            digests += 1
+        if in_listing_section:
+            for digest in DIGEST_ENTRY.findall(line):
+                digest_bytes += len(digest) // 2
+                digests += 1
     return total, digest_bytes, digests
 
 
+SELF_TEST_CAPTURE = """direction A -> B
+role: Responder
+control item 0 (30 bytes) / preamble /
+55799_1([
+    "rumors",
+    2,
+])
+control item 1 (60 bytes) / greeting /
+24_0(<<{
+    "listing": {
+        82_0: h'010203040506070809101112131415161718192021222324',
+    },
+    "set_len": 1,
+}>>)
+Responder stream 0 (height 31), epoch 0, 90 wire bytes
+  frame 0 (20 bytes) / Query(Continue) /
+[0, 5, {1: h'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 2: h'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'}]
+  frame 1 (40 bytes) / Supply(End) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'e0'), {7: h'cccccccccccccccccccccccccccccccccccccccccccccc'}>>)>>)]
+  frame 2 (50 bytes) / Supply(End) /
+[0, 6, 63_0(<<63_0(<<53846_1(h'e0'), "x\u2028  frame 3 (9 bytes) / Query(Continue) /\u2028{1: h'dddddddddddddddddddddddddddddddddddddddddddddd'}">>)>>)]
+"""
+
+
+def self_test() -> int:
+    """Hold the patterns to their claims on a hand-written capture."""
+    total, digest_bytes, digests = measure_text(SELF_TEST_CAPTURE)
+    expected = (30 + 60 + 90, 24 + 23 + 23, 3)
+    if (total, digest_bytes, digests) != expected:
+        print(
+            f"digestshare: self-test FAILED: measured {(total, digest_bytes, digests)}, "
+            f"expected {expected} (greeting listing, one-line Query listing of two, "
+            "no digest from the Supply payloads, and no section opened by the "
+            "U+2028-separated header text inside a payload)",
+            file=sys.stderr,
+        )
+        return 1
+    print("digestshare: self-test ok")
+    return 0
+
+
 def main() -> int:
+    if sys.argv[1:] == ["--self-test"]:
+        return self_test()
     root = Path(__file__).resolve().parent.parent
     dirs = [Path(arg) for arg in sys.argv[1:]] or [root / "tests" / "snapshots"]
-    grand_total = grand_digest = grand_count = v2_files = 0
+    grand_total = grand_digest = grand_count = files = 0
     for directory in dirs:
         for path in sorted(directory.glob("*.snap")):
             measured = measure(path)
-            if measured is None:
-                print(f"{path.name}: skipped (V1 side-by-side timeline)")
-                continue
-            v2_files += 1
+            files += 1
             total, digest, count = measured
             share = (100.0 * digest / total) if total else 0.0
             print(
```

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 114:
>
> Fresh-eyes repair (item 3): a self-test over a hand-written capture holds the patterns to a one-line Query listing of two entries, the greeting's listing, and a Supply payload of the digest shape that must not count; the recipe runs it before the corpus measurement.

<!-- annotation -->
> **remote-capture-atlas-13** (T140), line 109:
>
> Fresh-eyes repair, round 2 (item 2): the self-test carries a Supply payload text with U+2028-separated header-shaped and listing-shaped tails, which must neither open a Query section nor count a digest.

<a id="hunk-51"></a>
### tools/digestshare `@@ -86,9 +147,9 @@ def main() -> int:`

```diff
@@ -86,9 +147,9 @@ def main() -> int:
             grand_total += total
             grand_digest += digest
             grand_count += count
-    if v2_files and (not grand_total or not grand_digest):
+    if files and (not grand_total or not grand_digest):
         print(
-            "digestshare: LIVENESS FAILURE: the corpus has V2 captures but "
+            "digestshare: LIVENESS FAILURE: the corpus has captures but "
             f"{grand_total} wire B / {grand_digest} digest B were parsed; "
             "the renderer's vocabulary has moved out from under this tool.",
             file=sys.stderr,
```

<!-- annotation -->
> **remote-capture-atlas-13** (T138), line 147:
>
> The liveness floor's counter and message no longer say V2, since no other render form exists in the corpus; the floor itself (zero wire bytes or zero digests fails) is unchanged.

