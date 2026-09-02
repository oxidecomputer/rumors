# Documentation and comments

This document collects every finding of the review whose primary class is documentation: prose that is inaccurate against the code, pitched at the wrong altitude for its reader, longer or shorter than its contract needs, or written in the default dialect the writing doctrine names. It answers one question per finding: where the prose is wrong and what it should say instead. Three audiences are kept apart because the fixes differ. Public rustdoc is what a library user reads on docs.rs; it may name nothing the API does not reach, and a false sentence there is a contract defect. Maintainer prose (private rustdoc and `//` comments) states invariants and the reasons behind branches; a false sentence there costs the next maintainer a wrong repair. Test doc comments are required by the gate's `testdoc` check and held by AGENTS.md to the standard that an inaccurate one is a bug in the test. Each finding names which audience it concerns in its claim.

Ids are `<partition or sweep key>-<n>`. The full record for each, including the lens reports and refutation notes it was distilled from, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`, which sit beside this file in the committed note. Severity is the finalizers' assignment on their four-step scale (high, medium, low, nit); no documentation finding was rated high, and in this class the medium entries turn out to be those where a public or maintainer-facing statement is false against the code, or where a maintainer following the prose would act on a wrong premise. Provenance is stated per finding: *verified* means the finalizer ran a command or made a mechanical check (a grep, a git query, a byte count, a line-by-line comparison of prose with code); *assessed* means the claim rests on reading; *demonstrated* means a constructed test exhibits the defect, which no documentation finding needed. Every entry keeps the finalizer's anchor, evidence, verdict, severity, and provenance unchanged. Where this document adds a judgment of its own it is marked "Synthesis note". During synthesis, forty anchors from the highest-value entries were re-read at 9e5784fb and every quoted line matched; the anchor pass that followed found two runs off by one (remote-codec-1, remote-proxy-1) and corrected them in place.

The class holds 313 entries (44 medium, 146 low, 123 nit). Medium and low entries appear below in the finalizers' full template; nit entries appear per module in a compact table (id, anchor, one-line claim, resolution), each pointing at the evidence file for its full record. Findings of other classes that bear on a documentation finding are cross-referenced by id and not reproduced. Sections follow the crate's own order; two sections not in the standard roster were added because entries anchored there: the streaming protocol's own test suite (`src/tree/mirror/streaming/tests`), placed after the materialized walk it exercises, and the verification recipes and configuration (justfile, `.config`, `.github`, `tools`), placed last. One entry anchored under `proptest-regressions/` (tests-observation-38) is filed with the integration tests, whose seeds those files are.

## Highest-value items

1. The public reconciliation page derives the stream bound as "16 data streams plus a control stream" while `STREAM_COUNT`'s own doc defines 17 data streams per direction beside the control stream, so a transport author sizing a pool from that page under-provisions by one stream per direction, and the page disagrees with itself forty lines apart (fresh-eyes-1, session-bookmark-35).
2. Five public sites and the runtime `Display` string of `Error::VersionMismatch` still describe a `Protocol` the user "selected", after the V1 retirement removed every setter; the error table's remedy row prescribes an action the API does not afford (api-core-4, with fresh-eyes-2, api-audit-5, module-graph-4, prose-hygiene-2, mirror-common-12 on the same residue).
3. `Bookmark::load`'s public contract promises implementors one call per `Peer`; the record driver re-reads storage after any failed `store`, so an implementor who trusts "once" and hands out a one-shot reader persists a record without the stranded clocks the bookmark exists to keep (session-bookmark-22).
4. `Link`'s retire cancellation carve-out says a dropped `retire` future loses the identity "recoverable only through an attached bookmark"; once `bookmark_donate` has persisted, the party is already sliced out of the record and a drop loses it exactly as `Retire::Uncertain` does, and no committed test cancels `retire` mid-flight (async-hazards-2).
5. `Peer::sync_memory_budget`'s public rustdoc names four test files, a private test function, a `cfg(test)` constant, and "the storage backend's own cost function", none reachable from docs.rs, inside a 160-line sizing guide on a setter; whether the guide moves to a docs-only page is an owner decision, the citations are not (fresh-eyes-3, api-audit-8, api-core-15, api-audit-9).
6. Three module docs under `src/tree/` say `join` and the wire mirror delegate deletion honoring to one filter, and `tree.rs` derives the crate's testing strategy from that premise; the streaming mirror has its own pruner and the identity is pinned differentially, so a maintainer changing `traverse::unknown` would wrongly believe the wire follows (tree-core-1).
7. `Tree::act`'s leaf-level contract prose does not match `Act for Z` or the two committed pins that contradict it, and `react`'s panic-atomicity comment rests its destructor argument on a "wire-apply path" that does not exist (tree-core-11, tree-core-16).
8. The public error taxonomy misdescribes itself in four places: `RemoteError` variant summaries name outcomes their wrapped enums never produce, `MaterializedViolation` speaks of reaction kinds no public item defines, `FrameShape` states an arity the decoder rejects, and no `missing_docs` lint guards the many undocumented public variants and fields (remote-proxy-1, materialized-18, remote-codec-20, remote-capture-atlas-21, api-audit-7).
9. AGENTS.md's renderer-vocabulary re-accept class demands a "hex-line-preservation witness" over snapshots that have carried no hexdump line since the CBOR reflection renderer landed; the hard rule's witness cannot be computed as worded, and the policy behind it is owner-ruled (prose-hygiene-6, verification-infra-15).
10. The `Bootstrap` page enumerates three settings of the builder's four and gives two contradictory answers to which settings affect the join session; `convert.rs`'s module doc describes a dormant cross-backend converter while the wire adapter runs the module on every supplied node (api-core-18, streaming-backend-window-19).
11. Numbers maintained by hand have rotted under the code: the geometry fixture's "attempt 1581", measured under BLAKE3 and not re-derived at the SHA3 swap; the window suites' `28 + m` where the crate ships 43; parking.rs's megabyte figures two format changes behind their own pin; the nextest profile's wall-clock bound a commit removed; and dispute_wire's design-cell doc contradicting `window.rs` about what derives from `DISPUTE_WIRE_BYTES` (tree-core-24, suite-economics-3, tests-resource-link-window-29, remote-adapter-tests-17, tests-resource-link-window-1, tests-wire-format-16).
12. Code cites roster tags that resolve only in deleted plans, agent notes, or `formal/MODEL.md`: "finding #6" and "#7", `T3`, `F4`, "Bridge 1/2/3", and fourteen `§6.n` prefixes opening the testdocs of tests/listen.rs, every one a citation AGENTS.md forbids from code (materialized-20, streaming-tests-8, prose-hygiene-4, prose-hygiene-3, tests-observation-15).

## Crate-wide patterns

Each pattern below is stated once with the union of the sites its member entries name, so that whoever fixes it sweeps the whole list rather than one partition. The member entries keep their own text in the per-module sections (medium and low in full, nits in the tables) because each carries its own evidence and acceptance test; a reader repairing a single module can work from the entry, and a reader repairing the crate can work from here.

### Residue of the V1 retirement (368da2a5)

The retirement's own prose pass held almost completely (the prose-hygiene sweep's deleted-identifier grep found no `V1`, `LEGACY_MAGIC`, or `Alternating` in prose), and what survived is small and specific.

- The "selected `Protocol`" vocabulary, six entries on one residue: api-core-4, fresh-eyes-2, api-audit-5, module-graph-4, prose-hygiene-2, mirror-common-12. Sites: src/protocol.rs:1 ("Selectable"); src/error.rs:14 (the remedy row), :76 (the `Display` string), :179, :201; src/peer.rs:512, 603-604; src/tree/mirror/handshake.rs:180 (the twin `Display` string), :204; src/peer/gossip.rs:723-729 ("Both branches", "neither concrete protocol state machine"). api-audit-5 adds the vestigial `#[repr(u16)]` at src/protocol.rs:12 and tests/handshake.rs:59. No snapshot pins either `Display` string, so rewording moves no pin.
- "Two deliberate boundaries" over one bullet on the public `SessionStats` doc, the second bullet having been the V1 one: fresh-eyes-5, mirror-common-35, api-audit-17 (src/tree/mirror/streaming/stats.rs:33-38).
- "through both protocol implementations" (tests-disruption-handshake-9, tests/gossip_pipelining.rs:5-8); "the protocols" (tests-resource-link-window-9, tests/latency_link.rs:5); "V2" qualifiers that distinguish nothing (tests-common-7, tests/common/gossip_snapshot.rs:389-399, tests/common/sim.rs:422).
- The shared-filter premise of the V1 mirror (tree-core-1: src/tree.rs:58-61, src/tree/traverse/join.rs:8-11, src/tree/traverse/unknown.rs:9-11, src/tree/mirror/streaming/materialized/unknown.rs:4-5, src/tree/mirror/streaming/tests.rs:154-156) and the ghosts of `Levels`, the zipper, and the typed tower (tree-core-26 and module-graph-16 at src/tree/traverse.rs:7-19 and src/tree/typed.rs:19-22; tests-wire-format-25 at tests/future_size.rs:3-9, 37-39; materialized-25 at materialized/unknown.rs:19-23; mirror-common-23 at erased.rs:29-30).
- V1 phase vocabulary (`Exchange`, `Opening`, `Closing`, `Complete`, "to Done") in gossip_snapshot testdocs (tests-wire-format-6, tests/gossip_snapshot.rs:447-453, 484-499) and in src/tree/mirror/streaming/tests.rs:179-180 (streaming-tests-5, whose history pass shows that site is streaming's own retired vocabulary rather than V1's).

### Residue of the wire respellings

Four changes to the wire and the tree's addressing each left prose one step behind: the CBOR respelling (4dd2053c: 30-byte preamble, one-item greeting, two-byte epilogue marker), the two-item frame opener (3327a92b), version addressing (961f63c6), and the suffix-only leaf preimage (f3fef7bc).

- A 25-byte preamble with fields that no longer exist: tests-wire-format-3 (tests/gossip_snapshot.rs:51), testing-infra-16 (src/tests.rs:25-28), module-graph-5 (the same lines, plus src/peer/gossip.rs:4-6 and tests/handshake.rs:1).
- A two-frame greeting: remote-proxy-tests-1 (five test names at src/tree/mirror/streaming/remote/proxy/start/tests.rs:73-169; src/tests.rs:262-264), remote-proxy-1 ("greeting frames" at proxy/error.rs:19, 25), testing-infra-16 (src/tests.rs:262-264).
- A "one- or two-element" frame where the decoder requires two or three: remote-codec-20 and remote-capture-atlas-21 (codec/error.rs:131-133); a "one-byte count-minus-one" query listing the encoder does not write: remote-capture-atlas-2 (remote.rs:30-31); "pre-batching" (remote-codec-4, codec/budget.rs:108-110); parking.rs's megabyte figures (remote-adapter-tests-17).
- Content addressing where the tree is version-addressed: tree-core-23 (src/tree/arb.rs:235-236, 301-302, 327-329; out of partition src/tree/mirror/streaming/stats.rs:45 and streaming/tests/fixtures.rs:322), tree-core-18 (src/tree/tests.rs:29-33), tests-resource-link-window-4 ("byte-for-byte" from the hash, tests/async_wire.rs:26-27).
- A leaf preimage said to commit the version's encoding: tree-typed-5 and inventory-9 (src/tree/typed/hash.rs:60-64), with the drifted 24-byte argument at hash.rs:39-43 (tree-typed-3).
- The pre-erasure `Reply<B, H>` boundary and `Convert::assemble` (remote-adapter-streams-1, adapter.rs:4-14, 52, 56-58; remote.rs:59); `convert.rs`'s dormant converter (streaming-backend-window-19).

### Public rustdoc that names what the API does not reach

Documentation altitude: a public page names only concepts reachable from the API the reader holds.

- Test files, a private test function, and a `cfg(test)` constant in `Peer::sync_memory_budget` and `DEFAULT_SYNC_MEMORY_BUDGET` (fresh-eyes-3, api-audit-8, api-core-15, streaming-backend-window-27: src/peer.rs:313, 318, 393-394, 417, 428, 444-445, 457, 527; src/tree/mirror/streaming/window.rs:272-274); `Tree::join` on the public reconciliation page (session-bookmark-36, src/reconciliation.rs:246-249).
- Private items in public error docs: `SUPPLY_FRAME_OVERHEAD` and `LeafRun::records` (remote-codec-19, api-audit-8: codec/error.rs:97-102, 150-153); reaction kinds `Match`/`Query`/`Supply` in `MaterializedViolation` (materialized-18); the coherence argument on `MirrorError`'s `From` impl (mirror-common-1, src/tree/mirror.rs:44-49, 66-75); `[u8; 6]` whose named constant is private (api-audit-7, api-core-6).
- Maintainer altitude on public pages: the sizing guide (api-audit-9), the backend suite's visibility paragraph in the `conformance` module doc (conformance-3), the `Backend` family addressed to an implementer the crate cannot admit (streaming-backend-window-1).
- File-path citations in prose, which a rename orphans: testing-infra-1 (src/testing.rs:69, 81, 105, 116, 136, 145, 161, 178, 188, 203; src/testing/transport.rs:667-669), streaming-backend-window-24 (window.rs:110-121), tests-wire-format-15 (tests/dispute_wire.rs:111-112), prose-hygiene-5 (justfile:731 citing formal/PROGRESS.md), tests-disruption-handshake-20 (tests/handshake.rs:1 citing a module that does not exist).

### Hand-maintained counts, tallies, and measurements

Principle 5: state the structure, not the tally; a number that matters lives in a mechanically enforced place that prose cites by name. The finalizers found the crate's convention (`Stream::COUNT`, `VALID_PLACEMENTS`, `DISPUTED_REPLY_TRANSIENT_CEILING`, `assert_eq!(checked, 8)`) applied at many sites and missed at these.

- Counts that have already rotted: "Two deliberate boundaries" (fresh-eyes-5, mirror-common-35, api-audit-17); "four invariants" over six (tests-disruption-handshake-4, tests/disruption.rs:571-573); "two corners" over three tests (tests-bookmark-1, tests/bookmark_attach.rs:4-8); "three orders of magnitude" for 10^2 to 10^6 (benches-envelope-17, benches/support/grid.rs:37-40); one caller where three exist (tests-resource-link-window-2, benches/support/latency.rs:398-400); module maps missing members (streaming-tests-1, remote-adapter-tests-1, tests-common-9, testing-infra-15, tests-lifecycle-30, tests-observation-1, remote-proxy-tests-13).
- Counts correct today that rot on the next change: "four `Retire` outcomes" (api-core-14); "Seven checks:" (materialized-19); "Both tests" (prose-hygiene-12); "one of 17", "one of ten", "162"/"163" placements (remote-codec-1, remote-capture-atlas-1, remote-adapter-streams-17: codec.rs:18-27, remote.rs:8, 12-18, streams.rs:3, signal/tests.rs:9-10, 146); "17 of the fixed item's 30 bytes", "a 33-arm match", caller rosters (mirror-common-2); "32 KiB is four times", "the fixture's fifth open", "a four-point `debug_assert`" (conformance-5); "16 data streams plus a control stream" (fresh-eyes-1, session-bookmark-35); "Version 5", "0x18 0x05" (session-bookmark-28).
- Measurements nothing re-derives: "attempt 1581" (tree-core-24, suite-economics-3); "2-3x" (tree-core-10); `28 + m` and "~36 B" (tests-resource-link-window-29); ≈ 1.1 MB / ≈ 2.2 MB (remote-adapter-tests-17); "~0.5 s under a 5 s budget", "around 6 seconds", "today" (tests-resource-link-window-1, suite-economics-6, verification-infra-18); "+0.7 GiB of rustc peak memory" (conformance-5; an open question in the link report); 172 B and 64 B literals (tests-wire-format-16); recorded hop counts beside bands (tests-resource-link-window-24); "8 GiB" memwatch cap (suite-economics-9); "8 KiB" per stream (link-1).

### Em-dashes in line comments and assert strings

The doctrine assigns the spaced double-hyphen to chat and code comments and the em-dash to rendered prose (`///`, `//!`). The prose-hygiene sweep's census: 169 `//` comment lines in `.rs` files (led by src/peer/gossip.rs 20, src/tree/mirror/streaming/window.rs 12, examples/swarm.rs 7, tests/gossip_when.rs 6, tests/causal.rs 6, src/tree/tests.rs 6, src/tests.rs 6, src/tree.rs 5), 95 `#` comment lines in non-`.rs` files (justfile 57, .cargo/mutants.toml 20, .github/workflows/ci.yml 9, Cargo.toml 5, .config/nextest.toml 3, .github/workflows/pages.yml 1), 4 inside assert strings (prose-hygiene-8: tests/future_size.rs:53, tests/bookmark_causality.rs:149, tests/party_conservation.rs:93-94), and 1394 in rustdoc, where they are permitted. Two `//` comments already use ` -- ` (src/tree/mirror/handshake/tests.rs:286, justfile:951). Member entries: prose-hygiene-9 (the master census and the regenerating greps), session-bookmark-6, conformance-19, tree-core-15, streaming-backend-window-7, materialized-38, remote-adapter-streams-7, remote-adapter-tests-18 (in part), remote-proxy-tests-19, swarm-example-11, tests-common-24, prose-hygiene-8. Twelve partition reports independently recommend one mechanical sweep and a `tools/` lint wired into the gate rather than per-partition edits; the open questions carry that decision.

### Metaphors promoted to jargon

The writing doctrine admits a metaphor only where it names an artifact and can be rewritten as mechanism without loss.

- "seam": 66 lines, 47 in rustdoc, defined nowhere (prose-hygiene-10's census). Public sites: src/tree/mirror/streaming/stats.rs:6, 29, 104, 122 (`SessionStats` and two field docs); src/peer/gossip.rs:177-178 (`Gossiped::stats`); src/link/routed.rs:214 (`Dial`); src/conformance/backend.rs:17. Private sites the entries name: src/tree/mirror/streaming/erased.rs:1, 12; streaming.rs:19; framing.rs:42; framing/tests.rs:18; conformance/backend.rs:253, 493, 518-521, 627, 643, 698, 728; conformance/backend/tests.rs:229-231, 430-444, 452-517; src/tree/arb.rs:524; backend.rs:183, 251; backend/local/tests.rs:57, 75; src/testing/memnet.rs:10; tests/routed_link.rs:10, 258; tests/bookmark_when.rs:1. Member entries: prose-hygiene-10, mirror-common-34, conformance-22, session-bookmark-5, fresh-eyes-11, link-13, streaming-backend-window-3, tree-core-12, tests-bookmark-22, tests-resource-link-window-6, testing-infra-6. Three finalizers note that the height-erasure sense at erased.rs:1 is the one use anchored to an artifact, and two withdrew "seam" candidates on that ground (materialized-23, remote-proxy-17), so the ruling should say whether that one sense survives.
- "knob": 46 sites under src/; public at src/lib.rs:263 (mirrored into README.md:267) and src/link/routed/endpoint.rs:30 (prose-hygiene-11); private at src/peer/gossip.rs:1201, src/tree/mirror/streaming/window.rs:14, materialized/work/queues.rs:308, src/tree/mirror/streaming/tests/fixtures.rs:31, and five tests/target_message_size.rs sites (session-bookmark-5, streaming-backend-window-3, materialized-23, streaming-tests-8, tests-resource-link-window-6, link-13).
- "door" for a constructor, `before`'s coinage (tree-typed-28: src/tree/typed/untyped.rs:520-525, 548-554, untyped/tests.rs:282; tree-core-20: src/tree/tests.rs:1385-1397 with "rung", "pair hull", "fringe"); "the walk" reused by the capture renderer for its own traversal (remote-capture-atlas-7, eight sites in capture.rs and capture/tests.rs); "story" (prose-hygiene-11, streaming-backend-window-3: src/tutorial.rs:299, backend.rs:273, backend/local.rs:199); "load-bearing" at 16 sites (prose-hygiene-11); "chokepoint" (materialized-23); "register" for a queue (remote-proxy-17); "compatibility door" and "tripwire" in the routed link (link-13); "Guardrail that" (prose-hygiene-11, tests/future_size.rs:1); "the Law of Disjointness" (tests-observation-31).

### Moralized vocabulary colliding with the model of record

AGENTS.md names the model "authenticated-honest-peer", so "honest" is a term of art for the trust premise. The finalizers found it borrowed for unrelated predicates, and its adversarial cousins ("lie", "deceived", "malicious", "trust boundary") used where the same hard rule puts hostile peers off-model. The prose-hygiene sweep counts "genuine(ly)" at 90 sites and "silently" at 81, most carrying a contrast or a mechanism; the entries below list the ones that do not.

- "honest" for something other than the trust premise: an EOF versus a bad byte (session-bookmark-16, gossip.rs:1282-1284; tests-common-23, `is_honest_error` at tests/common/sim.rs:359-385 and six binaries); a self-reporting cost function (`Dishonest`, `Knob.honest`, conformance-32); trees, ticks, an iterator, a simulation (tree-core-12); a lint kept "honest" in nine copies (remote-proxy-22, remote-adapter-streams-11, tree-core-12, streaming-backend-window-3, tests-common-23); fully delivered frames (`HONEST_LEN`, tests-resource-link-window-6); "honest divergence" (testing-infra-6); "honest sessions" (tests-observation-19); "honestly sized frame" (mirror-common-36); "the honest floor" (tests-wire-format-15); "honest encoder" and "trust boundary" (remote-codec-2, codec.rs:58-61, encode.rs:82, frame.rs:515, async_io.rs:434-436); "panicking is honest" (swarm-example-4); "integer-honest" (benches-envelope-26); "address seam honest" (testing-infra-6, prose-hygiene-11).
- "lie", "lied", "deceived", `GreetingLie`: remote-proxy-tests-14 (declarations.rs and start/tests.rs, plus 43 sites in ten files outside that partition); the recorded term is "misdeclared" (408ede87). "a buggy or malicious peer": testing-infra-6 (src/tests.rs:86-87).
- "genuine(ly)", "real", "really", "silently", "loudly", "sound" (loose), "provably", "faithful", "teeth": conformance-4, link-13, tree-core-12, mirror-common-36, streaming-backend-window-3, materialized-23, remote-capture-atlas-18, remote-adapter-tests-8, testing-infra-6, tests-disruption-handshake-1, tests-common-27, tests-bookmark-22, tests-resource-link-window-6, tests-wire-format-15, swarm-example-4, benches-envelope-26, prose-hygiene-11, streaming-tests-8. Several finalizers list the uses that carry information and should stay ("real sockets", "real TCP", "a real accept").

### Roster tags and process narrative cited from code

AGENTS.md forbids citing `formal/MODEL.md`, `formal/PROGRESS.md`, and design documents from code, and permits Lean theorem or definition names with the invariant restated inline. The prose-hygiene sweep checked every tag: `B5` is a Lean axiom and "charter" a Lean-anchored term, so those citations stand; `F4` resolves nowhere in the tree, "finding #6" and "#7" only in MODEL.md's dated finding log and an agent note, `T3` only in a Lean docstring beside `wc_impossibility`, "Bridge 1/2/3" only in these three module docs, and `§6.1` through `§6.12` only in a plan deleted from the tree. Sites: src/tree/mirror/streaming/materialized/progress.rs:82, 93, 114, 199, 208; progress/tests.rs:62, 80, 102, 121; materialized/transcript.rs:10-11; src/tree/mirror/streaming/tests/wedge.rs:1, 4, 9, 118; local_eq.rs:1, 12-13, 142, 259; announced.rs:1-2, 41; skeleton.rs:18-19, 21, 506; capacity.rs:244, 280; tests/listen.rs:73, 109, 137, 164, 215, 241, 276, 298, 330, 357, 380, 464, 555, 612. Member entries: materialized-20, streaming-tests-8, prose-hygiene-4, prose-hygiene-3, tests-observation-15, materialized-21 ("the weave", `formal/MODEL.md:37`), materialized-19 (`d6` glossed two ways).

### Incident narrative and dated rationale at the declaration site

Prose speaks in the present tense; provenance lives in git. The entries: generator docs narrating the streaming-deadlock incident (tree-core-21, src/tree/arb.rs:174-175, 185, 300-302); the overlap harness motivated by a defect and by imbl's 16-entry node (tests-common-11, tests/common/overlap.rs:9-11, 321-323, 414-416, 425-427; tests/session_overlap.rs:13, 73, 148); "still fails ... now that", "The discovering incident", "Historical" (prose-hygiene-7, tests/payload_depth.rs:325-327, tests/session_overlap.rs:73-76, tests/disruption.rs:589, tests/bookmark_causality.rs:1263); the `Party::join` defect narrative and "the fix" (tests-bookmark-16); past-tense counterexample headers and cut geometry nothing maintains (tests-disruption-handshake-5); "observed while building this bridge" (streaming-tests-9); seed paths the gate forbids inside a design-history paragraph (streaming-tests-27); "have been *observed* live" (link-2); "a fleet upgrading together" with no prior release (session-bookmark-45); "keeps today's streamless opening" (remote-proxy-16); "exactly as before", "exactly as the typed tower did" (mirror-common-23, materialized-25); "the two-pass shape it replaces", "now rejects" (tree-core-35); "the split folds this replaces", scare-quoted "frozen" (tree-typed-32); a verification claimed done in a retired note (benches-envelope-30); "awaits owner disposition" and "minted" in seed files (tests-observation-38); "existing" as a dated qualifier (remote-adapter-streams-1).

### Rewrap residue from the first-sentence split

Commit dfd19c44 inserted a blank `///` after each doc's first sentence to satisfy `tools/doclint`'s summary rule and did not re-wrap the paragraph that follows, leaving fragment lines of one to five words above full-width continuations. The split is correct; the wrap is unfinished, and rustfmt does not reflow comments. Sites: src/error.rs:6; src/peer.rs:332, 334; src/lib.rs:187; src/tutorial.rs:112; src/rumors/changes.rs:18; src/peer/bootstrap/tests.rs:6; src/rumors.rs:255; src/batch.rs:49, 55-56 (api-core-3); src/peer/gossip.rs:68-69; gossip/tests.rs:156-157; src/bookmark.rs:72-73, 118-119; src/reconciliation.rs:55 (session-bookmark-14, session-bookmark-33); tests/bookmark_attach.rs:75-77, 127; tests/bookmark_causality.rs:212, 310, 472, 492, 647, 1047, 1218; tests/bookmark_when.rs:215, 296, 389 (tests-bookmark-5); tests/handshake_liveness.rs:41-44, 56-59, 81-82, 290-295, 316-318; tests/gossip_when.rs:153-157, 359-364, 472-475, 540-542, 784-787; tests/disruption.rs:80-83, 575-580; tests/hop_trace.rs:632-634 (tests-disruption-handshake-24); tests/causal.rs:184-189, 214-222, 454-459; tests/changes.rs:165-170; tests/listen.rs:164-169, 464-473, 612-620; tests/observe.rs:406-411; tests/party_conservation.rs:124-129, 153-158, 200-205, 213-219, 297-306, 339-346 (tests-observation-4); examples/swarm.rs:282-283, 330-331, 355-356, 515-516, 626-627, 1155-1156, and mildly 399-400, 877-878, 1197-1198 (swarm-example-7); benches/gossip_grid.rs:14-16 (benches-envelope-7); tests/gossip_snapshot.rs:51 (tests-wire-format-3).

### Hazard sections named or placed inconsistently

Hazards get uniform named sections. `# Cancellation` at src/rumors.rs:563 against `# Cancel safety` at eight other sites (api-core-28, api-audit-16); `# Errors` absent from `send`, `send_all`, `gossip`, `gossip_when`, and `Bootstrap::join` while `Peer::bookmark`, `Endpoint::new`, `Endpoint::link`, `Connector::connect`, and `Acceptor::accept` carry one (api-audit-16); `# Cancel safety` absent from `Connector::connect` and `Endpoint::link` while their accept-side duals have it (link-4); cancel safety stated as a trailing sentence at src/tree/mirror/cbor.rs:219-224 and in prose at handshake.rs:244, 264 (mirror-common-4); `# Panics` absent from `from_sorted_leaves` while every other hazard in its partition has one (tree-typed-27).

### Missing module docs and undocumented public items

Twenty-seven non-test files open without a `//!` (module-graph-15; the multi-item ones are src/rumors.rs, adapter/decode.rs, adapter/encode.rs, typed/node.rs, typed/untyped.rs, protocol/peer.rs, proxy/work/progress/trace.rs, materialized/common.rs, backend/local.rs), and the map-carrying docs of `streaming/tests.rs`, `adapter/tests.rs`, `tests/common/mod.rs`, `work.rs`, and `src/tests.rs` omit members (streaming-tests-1, remote-adapter-tests-1, tests-common-9, materialized-29, testing-infra-15). Undocumented items: the phase-schedule traits and `Greeting::version` (mirror-common-30); `Resolver`'s methods (materialized-29); `RoleStats`'s fields (streaming-backend-window-17); `Kind`'s variants and two functions in the proxy trace (remote-proxy-18); many public variants, fields, and methods under `rumors::error` (api-audit-7, remote-codec-19, api-core-6, materialized-18). No `missing_docs` lint exists anywhere in the tree (api-audit-7), which is why none of these fails the gate.

### The illumos lint-allow rationale in nine copies

One comment, copied to nine in-scope sites (src/tree.rs:632-635, 672-675; src/tree/mirror/streaming/backend/local/adversarial.rs:22-25; channel/instrumented.rs:212-215; materialized/transcript.rs:59-62; materialized/progress.rs:391-394; remote/adapter/decode.rs:565-568; remote/proxy/work/progress/trace.rs:146-149; tests/common/wire.rs:22-25), names illumos as one of "the gate's targets" and says the allow "keeps `-D warnings` honest". Nothing in the justfile, workflows, `rust-toolchain.toml`, or AGENTS.md names illumos; the practice is a hand run on ox-east-1 recorded in commit eb4e0e1ba. Member entries: remote-proxy-22, remote-adapter-streams-11; the "honest" half also in tree-core-12, streaming-backend-window-3, tests-common-23.

### One argument in two homes that have drifted

Duplicated rationale drifts; these are the realized instances. The 24-byte width argument (tree-typed-3: hash.rs:39-43 against reconciliation.rs:164-172); the `PhantomData<fn() -> H>` argument and the typed wrappers' restated docs (tree-typed-13); the frame grammar restated in `remote.rs` beside `codec.rs`, which is where remote-capture-atlas-2's drift arose (remote-capture-atlas-1); parking.rs against message.rs and its own pin (remote-adapter-tests-17); the full-fan and one-slot capacity arguments (materialized-32); the version-bound rationale (remote-adapter-streams-8); the underpriced-node hazard three times (streaming-backend-window-4); the session promise in full on two pages (api-core-27); typestate impl docs against `Work` (remote-proxy-10); the greeting derivation in `connect` and `accept` (materialized-11, an idiom-class entry); the observer-replay rationale at four sites (swarm-example-17); dispute_wire's design cell against `window.rs` (tests-wire-format-16); the stream count on one page (fresh-eyes-1, session-bookmark-35).

## Findings by module

Sections follow the crate's own order. Within a section, medium entries come first, then low, each ordered by anchor (with `tests/common` before the suites in the integration-test section); the nits close the section as a table. Entries reproduce the finalizers' template, lightly edited for style; a "Synthesis note" line, where present, is this document's addition.

## Crate root and public surface (lib, peer, rumors, batch, snapshot, network, tags, protocol, error, tutorial; README, Cargo.toml, AGENTS.md)

### prose-hygiene-6: AGENTS.md re-accept rule names a hexdump witness the capture renderer no longer produces
- Where: AGENTS.md:155-161 (related: src/tree/mirror/streaming/remote/codec/capture.rs:12; src/bookmark/format/tests.rs:465-473)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -lE '^[0-9a-f]{20,}$'` over every `.snap` matches only the two bookmark pins; tests/snapshots/gossip_snapshot__one_sided_transfer.snap:1-10 is CBOR diagnostic notation; capture.rs:1-40 read; `git log --format='%h %ci' -S'hex-line-preservation' -- AGENTS.md` gives c6333e9e 2026-08-17; `git log --format='%h %ci' -1 ef6569c4` gives 2026-08-20, and that commit's stat touches no AGENTS.md)
- Verification: confirmed; history: deliberate-but-expired (the rule was codified three days before the renderer change and not updated with it)
- Owner-gated: yes: the re-accept policy is owner-ruled; the factual correction of its witness is not

The renderer-vocabulary re-accept class demands "every hexdump line
sequence identical to the parent commit", but the 22 snapshots under
tests/snapshots are CBOR reflection renders with no hex line, and
capture.rs:12 is headed "Why a rendering with no hexdump is still a byte
pin". The witness the rule demands cannot be produced for the snapshots the
bullet is about.

Evidence:

    AGENTS.md
    155	  One further sanctioned re-accept class: a renderer-vocabulary change
    156	  (the capture renderer's decoded annotations gained or reworded, the
    157	  wire untouched), permitted only with the hex-line-preservation
    158	  witness — every hexdump line sequence identical to the parent commit,

    src/tree/mirror/streaming/remote/codec/capture.rs
    12	//! # Why a rendering with no hexdump is still a byte pin

Resolution: Restate the witness in terms of what the render carries: the
diff confined to `/ comment /` annotation text, with every item and stream
byte count and every `h'...'` byte string, integer, and tag number identical
to the parent commit (capture.rs's injectivity argument is the
justification). The bookmark pins' hex-line sentence at
src/bookmark/format/tests.rs:465-473 is accurate and stays. Acceptance: a
reviewer can check the stated witness against a real re-accept diff of
tests/snapshots.

Synthesis note: Same clause as verification-infra-15, which proposes the mechanical form of the restated witness; open question 7 carries the policy half.

### api-core-4: Prose describes selecting a `Protocol`, an operation the API no longer offers
- Where: src/error.rs:14 (related: src/error.rs:76, src/error.rs:179, src/error.rs:201, src/protocol.rs:1, src/peer.rs:512, src/peer.rs:604; out of partition: src/tree/mirror/handshake.rs:180, src/tree/mirror/handshake.rs:204, src/peer/gossip.rs:723)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep: no `fn protocol` or `.protocol(` anywhere in src, tests, examples, or benches; `Protocol` has one variant at protocol.rs:15-18; the only public carriers are `Error::VersionMismatch.local_protocol` and `observe::SessionInfo.protocol`)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: deliberate-but-expired (every site was accurate under the `.protocol()` builders; 368da2a5 removed them and re-touched peer.rs:604 without rewording, and its prose sweep pattern did not include "select")
- Owner-gated: no (the enum itself stays, per the retirement note's decision 2)

Seven sites in the partition describe the protocol as something the user selects: the module doc, the error table's remedy row, the `VersionMismatch` display string (the text a user reads at runtime), two `Error` variant docs, `target_message_size`'s "default protocol", and `payload_depth_limit`'s analogy. No selector exists, so a user following the remedy row has nothing to do. AGENTS.md's hard rule is that nothing in the tree refers to code that no longer exists.

Evidence:

    14	//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |

    76	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]

    1	//! Selectable wire reconciliation protocols.

    603	    /// is therefore a fleet-coordinated configuration event, like
    604	    /// changing the selected [`Protocol`](crate::Protocol), never a
    605	    /// per-peer tuning parameter.

Resolution: protocol.rs:1 becomes "The wire protocol version a session speaks."; error.rs:14 becomes "the two releases speak different wire versions: align crate versions"; error.rs:76 becomes "peer speaks rumors protocol version {remote_version}; this release speaks {local_protocol:?}"; error.rs:179 and 201 say "this release's dialect"; peer.rs:512 says "When a session supplies a subtree the counterparty lacks"; peer.rs:604 says "like upgrading to a release that speaks a new [`Protocol`] version". Apply the same wording to the out-of-partition twins at handshake.rs:180 and 204 and gossip.rs:723. Acceptance: `grep -rn -i select src/protocol.rs src/error.rs src/peer.rs` returns no line pairing selection with `Protocol` or a dialect, and no doc or message implies a protocol choice the API does not offer.

Synthesis note: One of six entries on one residue (fresh-eyes-2, api-audit-5, module-graph-4, prose-hygiene-2, mirror-common-12); the union of their sites is in the patterns section. Their proposed wordings differ only in phrasing, and any one of them applied at every site satisfies all six.

### fresh-eyes-2: Docs tell users to "select" a Protocol, but no API selects one
- Where: src/error.rs:14 (related: src/error.rs:76, src/error.rs:179, src/error.rs:201, src/protocol.rs:1, src/peer.rs:603-604, src/tree/mirror/handshake.rs:180, src/tree/mirror/handshake.rs:204, src/peer/gossip.rs:723-729)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (the sweep's probe `check-1.log:88`: `error[E0599]: no method named `protocol` found for struct `Peer<T, B>``; `git show 368da2a5` deletes three `pub fn protocol(mut self, protocol: Protocol) -> Self` setters from src/peer.rs, src/peer/bootstrap.rs and src/rumors.rs; `grep -rn Protocol src` finds no setter, and src/tree/mirror/handshake.rs:102, 125, 127 hard-code `Protocol::V2`; `Reconciliation` at gossip.rs:1080 has one method, `reconcile`)
- Verification: confirmed and widened (two more public sites at error.rs:179 and :201, and a private comment at gossip.rs:723-729 that still describes two protocol branches); history: deliberate-but-expired (the setter was removed by ruling: `.agent-notes/2026-09-01-v1-retirement/README.md` decision 2 and commit 368da2a5, whose message reads "a knob with one position; it returns with a V3"; the vocabulary survived the ruling. The one-variant `Protocol` enum itself is deliberate by that same message, so the fix is wording, not the type)
- Owner-gated: no

Four public pages and one Display message describe a protocol the user
"selected", and the `VersionMismatch` remedy is to "select the same
`Protocol` at both ends". Nothing selects one: the setters are gone and the
handshake hard-codes `Protocol::V2`. A user handed the error is given an
action the API does not afford, and a maintainer reading gossip.rs:723-729
is told about two branches that no longer exist.

Evidence:

    src/error.rs
    14	//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |
    76	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    179	    /// The peer opened as a rumors stream of the selected dialect, but a
    201	        /// The selected dialect's full preamble width.

    src/protocol.rs
    1	//! Selectable wire reconciliation protocols.

    src/peer.rs
    603	    /// is therefore a fleet-coordinated configuration event, like
    604	    /// changing the selected [`Protocol`](crate::Protocol), never a

    src/peer/gossip.rs
    723	        // Reconcile using this peer's selected protocol. Both branches meet at
    724	        // the lifecycle boundary the surrounding transaction needs: a local
    725	        // root plus raw transport halves positioned after reconciliation.
    726	        // The protocol bodies live behind the non-generic [`Reconciliation`],
    727	        // whose methods return their futures boxed: neither concrete
    728	        // protocol state machine becomes part of this outer session future,
    729	        // or of the consumer crate that instantiates it.

    .agent-notes/2026-09-01-v1-retirement/README.md (the ruling)
    decision 2 — remove the `.protocol()` builders (done; ...)
    ... (rewrite as the design rationale it is: the level-synchronous shape described as
    the naive alternative, not as a shipped selectable dialect)

Resolution: error.rs:14: keep only the second clause ("the two ends run
releases whose wire versions differ: align crate versions"). error.rs:76 and
handshake.rs:180: name the local wire version ("we speak
{local_protocol:?}"). error.rs:179, :201 and handshake.rs:204: "the
dialect" or "the wire's". protocol.rs:1: drop "Selectable" ("The wire
reconciliation protocol."). peer.rs:603-604: compare to a different
fleet-coordinated event, or cut the comparison. gossip.rs:723-729: state the
one body ("`Reconciliation::reconcile` returns its future boxed so the
protocol state machine stays in this crate's object code") and delete "Both
branches" and "neither concrete protocol state machine". Acceptance: `grep
-rn "select" src/protocol.rs src/error.rs src/peer.rs src/peer/gossip.rs
src/tree/mirror/handshake.rs` matches only settings setters (the
`payload_depth_limit` line at error.rs:109 and the builder docs), never
`Protocol`.

Synthesis note: Same residue as api-core-4; this entry adds the gossip.rs:723-729 comment and the sweep's compiler probe. See the patterns section for the union of sites.

### fresh-eyes-3: sync_memory_budget rustdoc cites test files, a test fn, and internal concepts
- Where: src/peer.rs:393-457 (related: src/peer.rs:313, src/peer.rs:318, src/tree/mirror/streaming/window.rs:273-274, src/reconciliation.rs:248-249, src/snapshot.rs:4)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (the four cited files exist under tests/; `default_crossover_matches_the_solve` is at src/tree/mirror/streaming/window/tests.rs:278; `SCOPE_ENVELOPE_BYTES` is `pub(crate)` at window.rs:265 while `DEFAULT_SYNC_MEMORY_BUDGET` is public via peer.rs:20 and lib.rs:341; `mod tree` is private at lib.rs:322, so `Tree::join` is unreachable from the API. The altitude judgment is assessed by reading)
- Verification: confirmed; history: the trade-off table and closed form are deliberate (`.agent-notes/2026-07-22-sync-budget/sync-budget.md:150-156` records the table as "compiled into `sync_memory_budget`'s rustdoc ... the figure of record"); the test-file citations have no-rationale-found
- Owner-gated: no

The public rustdoc of one setter names four test files, one test function,
and "the storage backend's own cost function"; the public
`DEFAULT_SYNC_MEMORY_BUDGET` names a `pub(crate)` constant; the public
reconciliation page names the private `Tree::join`. None is reachable from
docs.rs or the API. They are provenance for maintainers inside a contract the
user reads for one number, and they lengthen a ~160-line doc.

Evidence:

    src/peer.rs
    313	    /// priced by the storage backend's own cost function — and
    318	    /// record per reply stream — ~0.2 MB under the in-memory backend, a
    393	    /// (calibrated by deterministic byte counts,
    394	    /// `tests/dispute_wire.rs`).
    417	    /// (`tests/tradeoff_probe.rs`).
    428	    ///   and pinned by `default_crossover_matches_the_solve`;
    444	    ///   solve). `tests/window_operator.rs` holds the wave model
    445	    ///   against measured sessions on a bandwidth-limited link.
    457	    /// `tests/window_knee.rs`, `tests/window_operator.rs`). One

    src/tree/mirror/streaming/window.rs
    273	/// decomposition behind the accuracy band is recorded beside the pinned
    274	/// per-scope envelope (`SCOPE_ENVELOPE_BYTES`).

    src/reconciliation.rs
    248	//! behavioral oracle in the test suite is the in-memory merge
    249	//! (`Tree::join`), which honors deletions through the same filter.

Resolution: Keep the contract, the closed form with its accuracy band, the
worked answers, and the table. Replace each "(`tests/x.rs`)" with the claim's
status alone ("measured", "pinned by test") and move the file names to the
tests' own doc comments or to a maintainer comment on the constant each
pins. Replace "the storage backend's own cost function" and "under the
in-memory backend" with what the user can see (per disputed subtree in
flight, at the default) or cut. window.rs:273-274: drop the parenthetical
naming `SCOPE_ENVELOPE_BYTES` from the public constant's doc (a `//`
maintainer comment beside the constant may keep it). reconciliation.rs:248-
249: cut the oracle sentence from the public page (it is a test-suite fact).
Acceptance: `grep -n "tests/\|_matches_the_solve\|SCOPE_ENVELOPE_BYTES\|Tree::join" src/peer.rs src/tree/mirror/streaming/window.rs src/reconciliation.rs` matches only lines that are not `///` or `//!` docs of public items.

Synthesis note: Overlaps api-audit-8 and api-core-15 on the sync_memory_budget citations; api-audit-9 holds the altitude question (open question 1). One edit of peer.rs:308-465 resolves all three.

### api-core-18: The `Bootstrap` page enumerates a three-setting builder that has four
- Where: src/peer/bootstrap.rs:33-38 (related: src/peer/bootstrap.rs:141-142, src/peer/bootstrap.rs:161-162, src/peer/bootstrap.rs:171-172, src/peer/bootstrap.rs:269, src/peer.rs:236-240)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read the four setting methods at 133, 153, 166, 180 and `bookmark` at 212; `git merge-base --is-ancestor` per the prose lens confirms the roster predates `observe`)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (the roster was accurate at c5f1210a3; 40b1e96a added `observe` without touching any roster line; 4356e197 maintained the roster for `payload_depth_limit` but nothing else; 55e347382's "rather than a fourth setting" was arithmetic on three)
- Owner-gated: no

Three statements on the public page are stale against the builder's current methods. The type doc enumerates three settings and omits `observe`; `target_message_size` is called "the one setting with immediate effect on the bootstrap session" while `payload_depth_limit` ("The join session decodes the provider's supplied records before a [`Peer`] exists, so the bound is selected here") and `observe` ("starting with the bootstrap session itself") both say they take effect there too; and `BookmarkedBootstrap` is justified as "A distinct type, rather than a fourth setting" when a fourth setting already exists. `Peer::bootstrap` (peer.rs:236-237) carries the same partial list. A user reading 141-142 and then 161-162 gets two contradictory answers to which settings affect the join session.

Evidence:

    33	/// Every setting here is the new peer's own, selected one session
    34	/// early: [`sync_memory_budget`](Self::sync_memory_budget),
    35	/// [`target_message_size`](Self::target_message_size), and
    36	/// [`payload_depth_limit`](Self::payload_depth_limit) each state what they
    37	/// change about the bootstrap session itself, and the joined peer keeps

    141	    /// This is the one setting with immediate effect on the bootstrap
    142	    /// session, the session that transfers the provider's entire set as

    269	/// type, rather than a fourth setting, lets each state's `join` declare only

    236	    /// builder's settings ([`Bootstrap::sync_memory_budget`],
    237	    /// [`Bootstrap::target_message_size`]) are the peer-to-be's own,

Resolution: Rewrite 33-38 without the roster ("Every session setting here is the new peer's own, selected one session early; each method states what it changes about the bootstrap session itself"). Replace 141-142 with the structural fact: the settings with a wire or handshake effect (run sizing, the greeting's depth-limit field, observation) reach the join session; `sync_memory_budget` alone has nothing to bound there. Replace "rather than a fourth setting" with "rather than another setting". Trim peer.rs:236-240 to point at the builder's page. Acceptance: no ordinal or method roster remains on the `Bootstrap`/`BookmarkedBootstrap` pages or in `Peer::bootstrap`; the `target_message_size`, `payload_depth_limit`, and `observe` docs agree about which settings affect the join session.

### verification-infra-15: AGENTS.md's snapshot re-accept witness is stated in terms of hexdump lines the corpus no longer has
- Where: AGENTS.md:155-161 (related: tests/snapshots/*.snap, src/tree/mirror/streaming/remote/codec/capture.rs:12, tools/digestshare:15-18)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the clause; every snapshot is a CBOR reflection render since ef6569c4, 2026-08-19; a grep for hexdump-shaped lines over `tests/snapshots/*.snap` matches nothing; the clause was written 2026-08-13 in c6333e9e; ef6569c4's message states the render change "does not qualify under, and does not use, the hex-line-preservation class")
- Verification: reframed: the sweep said the witness is a convention with no mechanical check; the prior problem is that the witness cannot be computed as worded, because the corpus has no hexdump lines. History: deliberate-but-expired
- Owner-gated: yes: re-defining a sanctioned re-accept class is policy

AGENTS.md sanctions a renderer-vocabulary re-accept only with "the
hex-line-preservation witness — every hexdump line sequence identical to
the parent commit". The snapshots are now fully unfolded CBOR value trees
whose byte pin rests on injectivity plus the exact byte-count headers
(`control item N (B bytes)`, `..., B wire bytes`) and `h'…'` literals;
there is no hexdump line to preserve. The clause is a ghost reference, and
the class it defines has no computable witness and no mechanical check.

Evidence:

    AGENTS.md
       155	  One further sanctioned re-accept class: a renderer-vocabulary change
       156	  (the capture renderer's decoded annotations gained or reworded, the
       157	  wire untouched), permitted only with the hex-line-preservation
       158	  witness — every hexdump line sequence identical to the parent commit,
       159	  the diff pure annotation additions or rewordings — and the re-accepting

    src/tree/mirror/streaming/remote/codec/capture.rs
        12	//! # Why a rendering with no hexdump is still a byte pin

    git show -s ef6569c4 (excerpt)
        snapshot extractor consumer, 2026-08-19); it does not qualify under,
        and does not use, the hex-line-preservation class.

Resolution: re-state the witness in today's terms (every byte-count header
and every `h'…'` literal identical to the parent commit, the diff pure
annotation additions or rewordings), and give it a mechanical form (a
small `tools/snapwitness` or a `snap-witness` recipe that extracts those
tokens from `git show <parent>:tests/snapshots/x.snap` and the working copy
and reports identical/moved per file), cited from AGENTS.md and run at
re-accept time. Acceptance: AGENTS.md names no hexdump, and the tool fails
on a scratch snapshot whose byte count moved under an annotation-only diff.

Synthesis note: Same clause as prose-hygiene-6.

### prose-hygiene-2: Prose still describes a protocol selection the V1 retirement removed
- Where: src/error.rs:14 (related: src/error.rs:76; src/tree/mirror/handshake.rs:180; src/protocol.rs:1; src/peer.rs:603-604; src/peer/gossip.rs:723-729)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rnE 'fn (protocol|with_protocol)\b' src` finds no setter; src/protocol.rs read in full: one variant `V2 = 2`, `#[default]`, `#[non_exhaustive]`; src/peer/gossip.rs:715-735 read; the Display string is not pinned by any `.snap` or test)
- Verification: confirmed; one site added: src/tree/mirror/handshake.rs:180 carries the identical "we selected" Display string; history: deliberate-but-expired (368da2a5 removed the setter)
- Owner-gated: no

The user-facing recovery table advises "select the same Protocol at both
ends", but there is nothing to select: `Protocol` has one variant and no
setter reaches it. The same framing survives in protocol.rs's first sentence,
two Display strings, `Peer::payload_depth_limit`'s docs, and a gossip.rs
comment whose "Both branches" and "neither concrete protocol state machine"
describe the removed second arm.

Evidence:

    src/error.rs
    14	//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |
    76	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]

    src/tree/mirror/handshake.rs
    180	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]

    src/protocol.rs
    1	//! Selectable wire reconciliation protocols.

    src/peer.rs
    603	    /// is therefore a fleet-coordinated configuration event, like
    604	    /// changing the selected [`Protocol`](crate::Protocol), never a

    src/peer/gossip.rs
    723	        // Reconcile using this peer's selected protocol. Both branches meet at
    ...
    727	        // whose methods return their futures boxed: neither concrete
    728	        // protocol state machine becomes part of this outer session future,

Resolution: error.rs:14: "the two ends speak different wire versions: align
crate versions". error.rs:76 and handshake.rs:180: drop "we selected" ("peer
speaks rumors protocol version {remote_version}, this side speaks
{local_protocol:?}"); no snapshot pins either string. protocol.rs:1: "The
wire reconciliation protocol, versioned." peer.rs:603-604: cite an event that
exists (a crate upgrade that bumps the protocol version). gossip.rs:723-729:
rewrite for one protocol. Keeping `Protocol` as a `#[non_exhaustive]`
versioned enum is outside this finding. Acceptance: `grep -rniE 'select(ed|able)?
.*protocol|protocol.*select' src` returns nothing.

Synthesis note: Same residue as api-core-4; this entry confirms that no snapshot pins either `Display` string.

### api-core-13: `Peer::bookmark`'s first sentence promises a persist that a pristine seed skips
- Where: src/peer.rs:245-246 (related: src/peer.rs:260-263, src/peer/gossip.rs:372-378)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `bookmark_inner`: the `pristine` check returns before `bookmark_record()` runs)
- Seen by: prose; refutation: confirmed; history: no rationale found (the pristine skip is deliberate and stated at 260-263 and gossip.rs:368-371; the summary line and its exception paragraph were added in one hunk and never reconciled)
- Owner-gated: no

The summary line, the one that appears in the module listing, says attaching persists the identity before returning; the fourth paragraph and the code say a pristine seed is attached without touching storage.

Evidence:

    245	    /// Attach `bookmark` to this [`Peer`], persisting its identity before
    246	    /// returning.

    260	    /// A pristine [`seed`](Peer::seed), with nothing sent and no identity yet
    261	    /// donated or absorbed, has nothing worth persisting, so this touches
    262	    /// storage only once the peer *knows* something: any content, or any
    263	    /// identity beyond the undivided seed.

    376	        if pristine {
    377	            return Ok(peer);
    378	        }

Resolution: First sentence: "Attach `bookmark` to this [`Peer`], persisting its identity before returning unless the peer is a pristine seed with nothing yet to record." Keep 260-263 as the explanation. Acceptance: the summary line and paragraph 260-263 agree; `Bootstrap::bookmark` (bootstrap.rs:194-196) already states the joined-peer case correctly and needs no change.

### api-audit-9: `Peer::sync_memory_budget` carries a 160-line operator sizing guide on a setter
- Where: src/peer.rs:308-470 (related: src/peer.rs:371-373, src/peer/bootstrap.rs:120-136, src/reconciliation.rs:216-235, src/tree/mirror/streaming/window.rs:267-275, src/tree/mirror/streaming/window/tradeoff.md)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read peer.rs:308-470; the contract paragraphs run 308-369, "# Choosing a budget" runs 371-464, and the included table is 11 lines)
- Verification: confirmed; history: deliberate-but-expired: the placement was chosen with the setting (1988de6da) and the sync-budget note's 2026-07-23 amendment records "worked in `Peer::sync_memory_budget`'s docs"; the crate has since grown explanation modules (`reconciliation`, `tutorial`) that give the guide a natural home it lacked then
- Owner-gated: yes: moves public documentation the owner placed

The doc comment on one builder method runs from line 308 to an included
trade-off table at 465: the contract (what is bounded, that any budget is
deadlock-free, per session and not wire-visible, follows the peer) occupies
the first sixty lines, and the remaining hundred plus the table are a
sizing methodology with a closed form, an accuracy band, worked figures,
and measurement provenance. A reader who came for "what does this setting do"
scrolls past the derivation; a reader who came for the derivation cannot
link to it except through the method. `Bootstrap::sync_memory_budget`
already models the alternative by pointing at the contract.

Evidence:

    src/peer.rs
    308	    /// Bound the memory a synchronization may spend on pipelining.
    ...
    371	    /// # Choosing a budget
    372	    ///
    373	    /// The intuition: the budget buys parallelism on the wire. A
    ...
    465	    #[doc = include_str!("tree/mirror/streaming/window/tradeoff.md")]
    466	    #[must_use]
    467	    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {

Resolution: move "# Choosing a budget" and the table into a public
explanation module beside `reconciliation` (or a section of it), leave the
method doc with the contract plus one pointer, and have
`DEFAULT_SYNC_MEMORY_BUDGET` and reconciliation.rs:233-235 point at the new
page. Acceptance: the method doc states the contract and links to the
sizing page; the sizing page renders the table.

### api-core-15: `sync_memory_budget`'s public doc cites test files and internals, and states the wire-buffer bound twice
- Where: src/peer.rs:308-465 (related: src/peer.rs:313, 318, 320-326, 353-357, 394, 417, 428, 445, 457, 527; src/lib.rs:322; src/tree/mirror/streaming/window/tests.rs:278; src/conformance.rs:10-12)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: `default_crossover_matches_the_solve` is a private test fn at src/tree/mirror/streaming/window/tests.rs:278; `mod tree` is private at lib.rs:322; the STREAM_COUNT x target_message_size bound appears at 320-326 and again at 353-357)
- Seen by: prose; refutation: confirmed; history: already-known for the derivation's placement (execution ledger 3a, ruled 2026-07-23, and option (c), ruled 2026-07-24: the closed form and the tradeoff table live in this rustdoc); the three sub-items below are not covered by either ruling
- Owner-gated: no for the three sub-items; relocating the sizing guide would reopen the rulings and is listed under open questions instead

The contract portion of this doc is right; three things in it are not at public altitude. (a) It prices memory "by the storage backend's own cost function" and "under the in-memory backend", and `target_message_size` repeats "storage backend" at 527, though `mod tree` is private and src/conformance.rs:10-12 itself says the storage boundary is crate-internal. (b) It cites `tests/dispute_wire.rs`, `tests/tradeoff_probe.rs`, `tests/window_operator.rs`, `tests/window_knee.rs`, and the private test fn `default_crossover_matches_the_solve` by name: repo-internal navigation a docs.rs reader cannot follow. (c) The encoded-wire-buffer bound is stated in full in the opening paragraph and again under "# What this does not bound".

Evidence:

    312	    /// what costs memory — kilobytes per disputed subtree in flight,
    313	    /// priced by the storage backend's own cost function — and

    393	    /// (calibrated by deterministic byte counts,
    394	    /// `tests/dispute_wire.rs`).

    428	    ///   and pinned by `default_crossover_matches_the_solve`;

    320	    /// This setting does not govern encoded wire messages in hand: the
    321	    /// wire schedule bounds those, at most one run per stream per
    322	    /// direction, so up to

    353	    /// - **Encoded wire messages in hand**: the run buffers stated
    354	    ///   above, priced by

Resolution: Replace "storage backend" and "in-memory backend" with "per disputed subtree in flight" (here and at 527); replace each test-file and test-fn citation with the stable claim ("the crate's tests pin the envelope and the crossover") or drop it; state the wire-buffer bound once, in the "# What this does not bound" list, and have the opening paragraph point there. Acceptance: no `tests/*.rs` path or test fn name appears in public rustdoc in the partition; "backend" appears in no public doc; the bound is stated once.

Synthesis note: Overlaps fresh-eyes-3 and api-audit-8; this entry adds the twice-stated wire-buffer bound.

### api-audit-8: Public rustdoc cites test files and a `cfg(test)` constant a library user cannot see
- Where: src/peer.rs:391-394 (related: src/peer.rs:414-417, src/peer.rs:428-429, src/peer.rs:444-445, src/peer.rs:456-457, src/tree/mirror/streaming/window.rs:264-265, src/tree/mirror/streaming/window.rs:272-274, src/tree/mirror/streaming/remote/codec/error.rs:150-153, src/tree/mirror/streaming/remote/codec/budget.rs:86)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn -E '^\s*(///|//!).*tests/' src`; `SCOPE_ENVELOPE_BYTES` is `#[cfg(any(test, feature = "test-internals"))] pub(crate)` at window.rs:264-265; `SUPPLY_FRAME_OVERHEAD` is `pub const` in the private `codec::budget` module and is not re-exported)
- Verification: reframed: the sweep also listed link.rs:49-50, link.rs:167-168, and the adapter `DecodeError` docs naming `max_version_bytes` and `set_len`; the first two already state provenance without a path ("the crate's own tests", "pinned ... by test"), which is the form the resolution asks for, and the last two are the greeting's wire field names (greeting.rs:16-19), observable by any wire observer, so all four are dropped from the list; history: no-rationale-found
- Owner-gated: no

The public docs of `Peer::sync_memory_budget` name five test artifacts
(`tests/dispute_wire.rs`, `tests/tradeoff_probe.rs`,
`default_crossover_matches_the_solve`, `tests/window_operator.rs`,
`tests/window_knee.rs`); `DEFAULT_SYNC_MEMORY_BUDGET`'s doc cites the
`cfg(test)` constant `SCOPE_ENVELOPE_BYTES`; `CodecDecodeErrorKind::
OverbatchedRun`'s field doc cites the private `SUPPLY_FRAME_OVERHEAD`.
None is reachable from the API, and nothing checks the test paths when a
suite is renamed (the gate's `citecheck` covers `before`'s rosters, not
doc-to-test paths).

Evidence:

    src/peer.rs
    391	    /// a 5431 B envelope (recomputed exactly by test), and each disputed
    392	    /// message costs 43 B of wire overhead on top of its record
    393	    /// (calibrated by deterministic byte counts,
    394	    /// `tests/dispute_wire.rs`).

    src/tree/mirror/streaming/window.rs
    264	#[cfg(any(test, feature = "test-internals"))]
    265	pub(crate) const SCOPE_ENVELOPE_BYTES: usize = 5_431;
    ...
    272	/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget); the
    273	/// decomposition behind the accuracy band is recorded beside the pinned
    274	/// per-scope envelope (`SCOPE_ENVELOPE_BYTES`).

    src/tree/mirror/streaming/remote/codec/error.rs
    150	        /// The frame's charged wire size — its run body plus the
    151	        /// `SUPPLY_FRAME_OVERHEAD` envelope at its widest — which may

Resolution: in the public docs, state the provenance without the path
("measured; pinned by test", as link.rs already does) and keep the numbers
with their validity bands; move the measurement narrative to maintainer
docs (a private module doc or the note under
`.agent-notes/2026-07-22-sync-budget/`). Replace the private-constant
names with the quantities they denote. Acceptance: `grep -rn 'tests/' src
--include='*.rs' | grep -E '^[^:]+:[0-9]+:\s*(///|//!)'` returns hits only
in private docs (`src/testing.rs`, test-module docs), and no public doc
names an item `cargo doc` does not render.

Synthesis note: Overlaps fresh-eyes-3 and api-core-15; this entry adds the codec error.rs site.

### api-audit-5: Prose still speaks of 'selecting' a Protocol that no API selects, and the enum carries a vestigial repr
- Where: src/protocol.rs:1-19 (related: src/error.rs:14, src/error.rs:76, src/error.rs:179, src/error.rs:201, src/peer.rs:603-605, src/tree/mirror/handshake.rs:102, src/tree/mirror/handshake.rs:125, tests/handshake.rs:59)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'select' src/protocol.rs src/error.rs src/peer.rs`; no public function takes or sets a `Protocol`; `git log -S'repr(u16)'` attributes the repr to 83edcd944, a WIP commit; the only casts are `Protocol::V2 as u64` on the wire and `as u8`/`as u16` in tests)
- Verification: confirmed; history: deliberate-and-holds for the enum itself (the V1 retirement note's decision 2 keeps `Protocol` public and `#[non_exhaustive]` as wire vocabulary), no-rationale-found for the surviving "select" prose and the repr
- Owner-gated: no

After the V1 removal (368da2a5) `Protocol` has one variant and no public
function selects one, yet the module doc calls the protocols "Selectable",
the error table tells the caller to "select the same Protocol at both
ends", the `VersionMismatch` display says "we selected", two `Error` docs
speak of "the selected dialect", and `Peer::payload_depth_limit` compares
its setting to "changing the selected Protocol". A reader goes looking for a
`.protocol(..)` builder that does not exist. The enum also carries
`#[repr(u16)]` while the wire writes the discriminant as a CBOR uint and
`VersionMismatch` reports `remote_version: u64`; nothing in the crate
depends on the repr.

Evidence:

    src/protocol.rs
    1	//! Selectable wire reconciliation protocols.
    ...
    12	#[repr(u16)]

    src/error.rs
    14	//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |
    ...
    76	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    ...
    179	    /// The peer opened as a rumors stream of the selected dialect, but a
    ...
    201	        /// The selected dialect's full preamble width.

    src/peer.rs
    603	    /// is therefore a fleet-coordinated configuration event, like
    604	    /// changing the selected [`Protocol`](crate::Protocol), never a
    605	    /// per-peer tuning parameter.

    src/tree/mirror/handshake.rs
    102	        cbor::write_head(&mut bytes, MAJOR_UINT, Protocol::V2 as u64);

Resolution: rewrite protocol.rs:1 as the dialect the crate speaks (one
today; a wire change adds a variant), the error.rs:14 row as "the peers run
releases speaking different wire versions: align crate versions", the
display at error.rs:76 as "we speak", error.rs:179 and 201 as "the rumors
dialect", and peer.rs:603-605 as "like a wire-version upgrade"; drop
`#[repr(u16)]` (or state at the enum why a repr matters when the wire
carries a CBOR uint), adjusting tests/handshake.rs:59 accordingly.
Acceptance: `grep -rn -i 'select' src/protocol.rs src/error.rs
src/peer.rs` finds no protocol-selection prose; the `Protocol` doc states
that the crate speaks one dialect.

Synthesis note: Same residue as api-core-4; this entry adds the `#[repr(u16)]` question, which open question 8 carries.

### module-graph-4: `Protocol` prose still describes selecting a dialect that no API offers
- Where: src/protocol.rs:1-11 (related: src/error.rs:14, :76; src/peer.rs:604; src/tree/mirror/handshake.rs:180, :203-205)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '\bProtocol\b' src tests benches examples`: every non-doc use is `Protocol::V2 as ...`, the `local_protocol` field, or `SessionInfo.protocol`; no builder takes one)
- Verification: confirmed, narrowed to prose; history: deliberate-and-holds for the type (`.agent-notes/2026-09-01-v1-retirement/README.md`, decision 2: "The `Protocol` enum itself stays public and `#[non_exhaustive]` (it is wire vocabulary ...)"), no-rationale-found for the prose (the re-denomination commit `c13c21b4` lists neither these sites nor a decision to keep them)
- Owner-gated: no (the type's placement and publicity are already ruled; only prose moves)

After the V1 retirement, `Protocol` has one variant and is produced only as a wire constant. The V1 plan ruled that the enum stays public as wire vocabulary, so the type is not the defect; the module doc, the error table, `peer.rs`'s rustdoc, two `Display` strings, and `handshake.rs`'s `PreambleDefect` doc still speak of selecting one, an affordance the `.protocol()` builders' removal took away.

Evidence:

    src/protocol.rs:
         1	//! Selectable wire reconciliation protocols.
    ...
         5	/// Both endpoints of a session must speak the same dialect; the preamble
         6	/// enforces this, diagnosing a skewed pairing as

    src/error.rs:
        14	//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |
    ...
        76	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]

    src/peer.rs:
       604	    /// changing the selected [`Protocol`](crate::Protocol), never a

    src/tree/mirror/handshake.rs:
       180	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    ...
       203	/// [`Error::PreambleMalformed`](crate::Error::PreambleMalformed): the
       204	/// peer opened as a rumors stream of the selected dialect, but one

Resolution: Rewrite `protocol.rs:1` as the wire-version vocabulary one release speaks (the enum's own doc at `:7-11` already says the right thing about frozen wire formats and new variants); re-word `error.rs:14` and `peer.rs:604` to "both ends must run releases speaking the same protocol version"; change the two `Display` strings to "we speak {local_protocol:?}"; drop "selected" at `handshake.rs:204`. Acceptance: `grep -rn -i 'select' src/protocol.rs src/error.rs src/peer.rs src/tree/mirror/handshake.rs` returns only `Peer::payload_depth_limit`'s "select the same" (a configuration setting that exists) or nothing.

Synthesis note: Same residue as api-core-4; see the patterns section.

### api-core-24: Terminology drifts in public prose, including a ghost of the retired `Broadcast` type
- Where: src/rumors.rs:93 (related: src/rumors.rs:33, src/rumors.rs:358-359, 369, 379, 389, src/rumors/unordered.rs:9, src/rumors/unordered.rs:12-14, src/rumors/causal.rs:15, src/peer.rs:303, src/rumors.rs:345-346, src/snapshot.rs:34-35)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S'pub struct Broadcast' -- src` ends at cb69fc951, the commit that renamed the type away; 7be20b412 swept the phrase from `Extant`'s doc and missed `Rumors::new`'s; unordered.rs:12-14 read as cited)
- Seen by: prose; refutation: confirmed; history: no rationale found; the `Broadcast` ghost edges into AGENTS.md's rule against deleted API names in prose
- Owner-gated: no

(a) `Rumors::new`'s doc says "a fresh broadcast generation" where rumors.rs:33 says "a [`Rumors`] generation"; `Broadcast` was the type `Rumors` replaced. (b) The observer accessors say "every message sent to this [`Rumors`]" (358, 369, 379, 389; also unordered.rs:9 and causal.rs:15), though gossip-learned messages are observed too and only unordered.rs:12-14 says so. (c) The three `network()` accessors open with three different sentences for one fact (peer.rs:303 "The globally unique identifier for this network of gossiping [`Peer`]s."; rumors.rs:345-346 and snapshot.rs:34-35 "The identifier shared by every peer that descends from the same [`seed`]"); the doctrine's parallel-prose rule wants siblings to share one skeleton.

Evidence:

    93	    /// Assemble the first handle of a fresh broadcast generation around `peer`,

    33	/// One handle's share of a [`Rumors`] generation's existence.

    358	    /// Monitor every message sent to this [`Rumors`], in arbitrary
    359	    /// (*non-causal*) order.

    12	/// This enumerates every message not causally contained in the starting
    13	/// checkpoint, then every message learned afterwards: by local
    14	/// [`send`](crate::Rumors::send), by gossip, through any handle. Once the

Resolution: (a) "a fresh [`Rumors`] generation"; (b) "Monitor every message live in this [`Rumors`], however it arrived" at the four accessors and the two observer type docs; (c) one sentence for all three `network()` accessors. Acceptance: `grep -rn broadcast src` is empty; the six observer sentences and the three accessors use the crate's established terms.

### api-core-26: The `_since` observer constructors state no same-universe precondition on `since`
- Where: src/rumors.rs:369-377 (related: src/rumors.rs:389-398, src/rumors/unordered.rs:111-113, src/rumors/causal.rs:106-108, src/network.rs:14-19)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read; a `Version` carries no `Network`, so nothing can check it)
- Seen by: correctness; refutation: confirmed; history: no rationale found (the same-network clause has lived only on `checkpoint()` since the `listen_from` era)
- Owner-gated: no

A checkpoint `Version` from another seed makes the containment filter meaningless and silently skips messages; network.rs:16-19 already records that independent universes can be "coincidentally and transiently compatible", which is exactly how a foreign checkpoint would filter live messages out. The precondition appears only on the producer side (`checkpoint()`: "another replica of the same network") and not where the value is consumed. AGENTS.md: never let two independently seeded universes interact; where the type system cannot enforce that, the contract at the consuming call is the only guard.

Evidence:

   369	    /// Monitor every message sent to this [`Rumors`] which is not already
   370	    /// causally contained in `since`, then everything learned afterwards, in
   371	    /// arbitrary (*non-causal*) order.
   372	    pub fn unordered_messages_since(&self, since: Version) -> UnorderedMessages<T>

Resolution: Add one sentence to both `_since` docs: `since` must be a checkpoint or frontier observed in this same [`Network`] (compare [`Rumors::network`]); a version from another universe is undetectable here and resumes incorrectly. Acceptance: both `_since` methods name the precondition and point at `network()`.

### api-core-37: `Snapshot` docs advise sorting by `Version`, which has no `Ord`
- Where: src/snapshot.rs:101-104 (related: src/snapshot.rs:129-132, crates/before/src/version.rs:1729-1760, crates/before/src/version/ranked.rs:372-383, src/rumors/causal.rs:54-63, tests/single_peer.rs:85-87)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `before` for `impl Ord for`: only `Ranked<'_>`, `Rank`, `Base`, and test oracles; `causal_cmp_impls!` at version.rs:1758 gives `Version` only `PartialOrd` via the causal partial order)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (inaccurate from birth: `Version` has never had `Ord`)
- Owner-gated: no

Both `iter` and `range` tell the reader to "Sort by the yielded [`Version`]s if your application needs an ordering consistent with causality." `Version` implements only the causal partial order, so `versions.sort()` does not compile and `sort_by(|a, b| a.partial_cmp(b).unwrap())` panics on any two concurrent versions (tests/single_peer.rs:85-87 gets away with it only for a lone peer's versions). The total order the crate itself uses is `Version::ranked()` (`before::Ranked: Ord`), and neither site names it.

Evidence:

   101	    /// Order is unspecified, and in particular does *not* follow the causal
   102	    /// order: a message may be yielded before another that causally precedes
   103	    /// it. Sort by the yielded [`Version`]s if your application needs an
   104	    /// ordering consistent with causality.

Resolution: Name the order at both sites: "sort by [`Version::ranked`] (a total order extending causality: the order [`CausalMessages`] delivers in)", with a two-line example (`items.sort_by(|a, b| a.0.ranked().cmp(&b.0.ranked()))`). Acceptance: a doctest at the `iter` site sorts a three-message snapshot containing two concurrent sends and asserts the causal predecessor comes first.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| deps-11 | Cargo.toml:117-118 | The `meter` comment names one suite as if it were the roster of three. | Name the class, not a suite. |
| fresh-eyes-10 | src/error.rs:3-6 | The routing sentence says everything else on the page reaches the user through `Error::Mirror`; `PreambleDefect` and `HandOffDefect` arrive through two other variants. | "through `Error`'s variants (chiefly `Error::Mirror`)"; rewrap line 6. |
| api-core-3 | src/error.rs:6 and nine related lines | Rewrap residue: one over-long or stub-short line mid-paragraph at ten sites. | Rewrap each paragraph to the surrounding width. |
| api-core-6 | src/error.rs:73-80, 233 | `MagicMismatch`, `VersionMismatch`, and `IntentInvalid` carry undocumented fields while every sibling documents each field. | One-line field docs. |
| api-core-8 | src/lib.rs:240-242 | "all demanded once, at peer construction" is true of the serde and `Eq` bounds only; `Send + Sync + 'static` recur on every typed method. | State which bounds recur. |
| prose-hygiene-11 | src/lib.rs:263 and the sites listed | Dialect tells: "knob" in the crate docs and `Config` docs, "load-bearing" (16 sites), "story", "Guardrail that", "earn a checkpoint", moralized "honest". | Reword per site; `just readme`. |
| deps-10 | src/lib.rs:274-282 (README.md:278-286 derived) | The feature list opens "Every feature is off by default" and omits `meter`. | Add a `meter` bullet; run `just readme`. |
| api-core-14 | src/peer.rs:285; src/peer/bootstrap.rs:334-335 | "the four `Retire` outcomes" and "the four ways" count enums defined in other files. | "each `Retire` outcome"; "Each way that can end is a `Joined` variant". |
| api-core-17 | src/peer.rs:710-712 and the first sentences listed | First sentences mix the imperative and the third person; the doctrine fixes third person, so the imperative majority is the deviation. | Owner ruling, then one sweep; at minimum make peer.rs:710, rumors.rs:410, and snapshot.rs:163 identical (owner-gated). |
| api-core-27 | src/rumors.rs:451-481 | The session promise with its three exceptions is stated in full on both `Rumors::gossip` and `Link`. | Cut `Rumors::gossip` to a summary linking `Link#what-a-session-promises`. |
| api-audit-16 | src/rumors.rs:563 (and the `# Errors`-less methods listed) | Hazard headings are inconsistent: `# Cancellation` beside `# Cancel safety`, and `send`, `send_all`, `gossip`, `gossip_when`, and `Bootstrap::join` carry error contracts with no `# Errors` heading. | Rename the heading; add `# Errors` above the existing prose; consider `clippy::missing_errors_doc`. |
| api-core-28 | src/rumors.rs:563 | `# Cancellation` where the crate's nine other hazard sections say `# Cancel safety`. | Rename. |
| api-audit-18 | src/snapshot.rs:140-143; src/rumors/unordered.rs:139, 156 | Two doc examples call `send` as a statement and discard its `Result`, teaching the pattern the admission contract exists to prevent. | Append `?` with a `Result` return; add `#![doc(test(attr(deny(unused_must_use))))]` to lib.rs. |

## Session and bookmark (peer/gossip, bookmark, reconciliation, observe, message)

### session-bookmark-22: `Bookmark::load` is documented as called once per `Peer`, but a failed store makes the crate call it again
- Where: src/bookmark.rs:96-102 (related: src/bookmark.rs:166-168, 308-313, 354-358, 244-246, 337-339; tests/bookmark_when.rs:9-11, 35-38)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read the path: `write`'s `Err` arm sets `self.inner = None` at 355; `ensure_loaded` calls `self.persist.read()` whenever `inner` is `None` at 309-311; `Persist::read` calls `Bookmark::load` at 166 and maps `Ok(None)` to `BTreeMap::new()` at 167-168)
- Seen by: prose, perfapi; refutation: confirmed (both); history: no rationale found (the "once" sentence and the failed-write reset were written in the same commit, b7fb409f; the contradiction is original)
- Owner-gated: no (the doc-correction policy sanctions fixing prose toward the code)

The public trait doc promises implementors that `load` runs once per `Peer`. After any failed `store` the record driver discards its in-memory record and the next `ensure_loaded` re-reads storage, so `load` runs again; a `load` future dropped mid-read leaves `inner` as `None` too. The private docs in the same file (244-246, 337-339) and `tests/bookmark_when.rs:35-38` state the re-read. An implementor that takes "once" literally, handing out a one-shot reader and returning `Ok(None)` afterwards, is read on the reload as "nothing has ever been written" (99), so the next `reclaim` or `record` pushes only the live alias and the next successful write persists a record without the stranded clocks: the identity loss the bookmark exists to prevent. A public contract clause the implementation contradicts is a correctness hazard for the implementor (Principles 4 and 5).

Evidence:

    96	    /// Open the stored record for reading, or `Ok(None)` if nothing is stored.
    97	    ///
    98	    /// Called once per [`Peer`](crate::Peer), lazily, before the first write.

    354	            Err(_) => {
    355	                self.inner = None;

Resolution: Rewrite the sentence to the actual schedule: "Called lazily before the first store, and again whenever the crate discards its in-memory record (after a failed `store`, or after a `load` that did not complete); `load` must be repeatable, and each call must return the current stored bytes." Add a test in tests/bookmark_when.rs (or tests/common/flaky.rs) that injects one store failure and asserts a second `Io::Read` on the next session. Acceptance: the `load` rustdoc states repeatability; the test pins the second read.

Construction: Implement `Bookmark` with a `Mutex<Option<Vec<u8>>>` whose `load` does `take()` (one-shot) and whose `store` fails once via an injected fault. Seed a peer, bookmark it with a record already holding a stranded clock for the same network, run one session with the fault armed, then one clean session; decode the stored frame and assert the stranded clock is still present. Under the current driver it is gone.

### fresh-eyes-1: reconciliation.rs and link.rs give different stream counts
- Where: src/reconciliation.rs:201-205 (related: src/link.rs:161-169, src/link.rs:93-97, src/reconciliation.rs:184, src/observe.rs:182-184, src/tree/mirror/streaming/remote/codec/signal.rs:31-32)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read `pub const STREAM_COUNT: usize = 17;` at link.rs:169 and `pub const COUNT: u8 = 17;` on the codec's `Stream` newtype at signal.rs:32, whose indices `0..COUNT` are the data streams the session opens through `Connector`/`Acceptor`; the control stream is the separate `Link` halves; the pin test `stream_count_matches_the_codec` in src/link/tests.rs asserts the two constants equal; read, not run)
- Verification: confirmed; history: no-rationale-found (reconciliation.rs:184 already states the bound as `STREAM_COUNT` per direction; the paragraph at 201-205 is a second, unpinned derivation that arrives at a different total)
- Owner-gated: no

The reconciliation page says a side needs at most 17 streams in total, 16
data plus one control; `STREAM_COUNT` and the codec say 17 data streams per
direction beside the control stream. A transport author sizing a pool from
the reconciliation page under-provisions by one stream per direction, and
the `+ 1` in link.rs's pool formula appears to double count under the
reconciliation page's account.

Evidence:

    src/reconciliation.rs
    201	//! In *theory*, the maximum number of streams needed on either side of the link
    202	//! is 17, though in practice, far fewer will ever be needed. Why 17? A 32-byte
    203	//! key gives the descent 32 levels; the schedule of traversal asks each side to
    204	//! hop down the tree by 2 levels at a time, so at most 16 data streams plus a
    205	//! control stream are ever needed.

    src/link.rs
    161	/// Logical data streams a session may open in one direction.
    162	///
    163	/// The protocol never opens more, and instantiations must admit this many
    164	/// concurrently (per direction, plus the control stream). The value is the
    165	/// protocol's own, fixed by its wire schedule (the descent's 32 tree
    166	/// heights at a two-height stride per stream, plus the shared opening
    167	/// stream: `ceil(32 / 2) + 1 = 17`) and pinned against the wire codec by
    168	/// test, so it cannot drift silently.
    169	pub const STREAM_COUNT: usize = 17;

    src/link.rs
    93	//! - Size the pool to at least **([`STREAM_COUNT`] + 1) × B** per
    94	//!   direction, where B is the per-stream buffering the transport grants:
    95	//!   every data stream plus the control stream (the +1), each sitting
    96	//!   full at the same moment.

    src/tree/mirror/streaming/remote/codec/signal.rs
    31	    /// Logical streams multiplexed into each transport direction.
    32	    pub const COUNT: u8 = 17;

Resolution: Rewrite reconciliation.rs:201-205 to state that a session opens
at most [`STREAM_COUNT`] data streams per direction beside the persistent
control stream, linking the constant for the derivation, and delete the
"16 data streams plus a control stream" arithmetic; one derivation, at the
constant. Acceptance: `grep -n "16 data\|is 17" src/reconciliation.rs` is
empty and the page states the bound only through `STREAM_COUNT`.

Synthesis note: Same site as session-bookmark-35, whose record carries the git provenance (77334965, a hand edit that replaced a derivation matching link.rs) and notes the wording is Finch's own; both stand, and one rewrite satisfies both.

### session-bookmark-35: Stream-count arithmetic contradicts `STREAM_COUNT`'s own doc and the same page
- Where: src/reconciliation.rs:201-205 (related: src/reconciliation.rs:181-184, src/link.rs:161-169)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read src/link.rs:161-169: `STREAM_COUNT: usize = 17` is "Logical data streams a session may open in one direction ... (per direction, plus the control stream)", derived as "the descent's 32 tree heights at a two-height stride per stream, plus the shared opening stream: `ceil(32 / 2) + 1 = 17`"; `git show 77334965 -- src/reconciliation.rs` shows this text replacing a derivation that matched link.rs)
- Seen by: prose; refutation: confirmed; history: no rationale found (the wrong derivation is Finch's own hand edit, 77334965 "Manual doc edits", replacing a correct one; no ruling accompanies it, so the evidence points to a misderivation)
- Owner-gated: no (a factual correction toward the code, sanctioned by the doc-correction policy; the wording being replaced is Finch's, so it is reported here rather than assumed)

The page says the maximum is 17 streams made of "16 data streams plus a control stream". `STREAM_COUNT` defines 17 as data streams per direction with the control stream additional; the +1 is the shared opening data stream, not the control stream. The same page at 183-184 says data streams are "at most [`STREAM_COUNT`] per direction", so the page disagrees with itself. A transport implementor sizing a pool from 201-205 under-provisions by one data stream per direction. The literals 16 and 17 are also hand-maintained restatements of a constant the prose can cite by name (Principle 5).

Evidence:

    201	//! In *theory*, the maximum number of streams needed on either side of the link
    202	//! is 17, though in practice, far fewer will ever be needed. Why 17? A 32-byte
    203	//! key gives the descent 32 levels; the schedule of traversal asks each side to
    204	//! hop down the tree by 2 levels at a time, so at most 16 data streams plus a
    205	//! control stream are ever needed.

Resolution: "A 32-byte address gives the descent 32 heights; each reply phase descends two, and the opening question rides its own stream, so a side needs at most [`STREAM_COUNT`] data streams per direction, plus the persistent control stream; in practice far fewer are opened." Drop the literal 16 and 17. Acceptance: the page's two statements agree with each other and with link.rs:161-169; no literal stream count remains in the prose.

Synthesis note: Same site as fresh-eyes-1; both stand.

### async-hazards-1: Bookmark gate is documented as preceding all wire traffic, but the preamble exchange precedes it
- Where: src/bookmark.rs:51-53 (related: src/rumors.rs:486-488, src/peer/gossip.rs:667-668, src/peer/gossip.rs:625-629, src/peer/gossip.rs:677, src/tree/mirror/handshake.rs:3-10)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (traced the await order in `gossip_inner`: `handshake::preamble(..).await` at 626 completes before `self.bookmark.lock().await` at 677 and `bookmark.write().await` at 713; checked `git show 3f33afabf:src/peer/gossip.rs`, the commit that introduced both the gate and the comment: the preamble was already at its line 582, the lock at 637)
- Verification: confirmed; history: no-rationale-found (the sentence was inaccurate when written, not drifted into)
- Owner-gated: no

Three sites say a bookmarked peer's sessions queue at the bookmark lock
"before any wire traffic". `gossip_inner` exchanges the fixed 30-byte
preamble with the peer first and takes the bookmark lock afterwards, so a slow
or failing store is observable on the wire: the peer's preamble has been
consumed and answered, and the peer waits for our greeting. The ordering is
deliberate (handshake.rs:9-10: the provider must learn whether the peer is
bootstrapping before it snapshots and forks), so the prose is what moves. The
other "before any wire traffic" phrases in the tree (error.rs:170,
gossip.rs:434, src/tests.rs:447 and 474) describe the `LinkPoisoned`
fail-fast, which does run before any byte, and are accurate.

Evidence:

    src/bookmark.rs
    51	/// A slow store delays session *starts* (sessions queue at the peer's
    52	/// bookmark lock before any wire traffic), never a `send` and never a
    53	/// session's in-flight wire progress.

    src/rumors.rs
    486	    /// which the `&mut` borrow enforces; a bookmarked peer's sessions also
    487	    /// queue at the bookmark lock before any wire traffic
    488	    /// ([`Bookmark`]).

    src/peer/gossip.rs
    625	        let remote =
    626	            match handshake::preamble(self.network, intent, staged, read, write, &observe).await {
    ...
    667	        // The lock order is bookmark-then-`watch`, as everywhere. A failed
    668	        // record write aborts the session before any wire traffic: dropping
    ...
    677	            let mut bookmark = self.bookmark.lock().await;

    src/tree/mirror/handshake.rs
    3	//! Every wire session first exchanges one fixed-size [`Preamble`] carrying
    4	//! the wire dialect's version, the network, and the session intent. Only
    5	//! after it succeeds does the mirror exchange its greeting, which
    ...
    9	//! Keeping these phases separate permits a provider to learn that its peer is
    10	//! bootstrapping before it atomically snapshots the tree and forks its party.

Resolution: At the two public sites (bookmark.rs:51-53, rumors.rs:486-488)
state the true position: sessions queue at the bookmark lock after the fixed
preamble exchange and before the greeting, so a slow store delays the peer's
greeting and a failed store fails a session the peer has already entered (the
peer abandons it by its own timeout, link poisoned). At gossip.rs:668 replace
"before any wire traffic" with "before the greeting". Acceptance: `grep -rn
'before any wire traffic' src/` returns only the `LinkPoisoned` fail-fast
sites (error.rs, gossip.rs:434, src/tests.rs), and the two public sentences
name the preamble.

### fresh-eyes-8: No worked Bookmark implementation is visible to users
- Where: src/bookmark.rs:104-124 (related: src/bookmark.rs:192-218, src/tutorial.rs:321, tests/bookmark_when.rs:103, tests/bookmark_transmit_window.rs:115, tests/common/flaky.rs:195)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -rn "Bookmark for" src tests examples` lists `NoBookmark` plus three test impls; `grep -n "# Example" src/bookmark.rs` is empty; the tutorial names `Bookmark` once, at line 321, without an impl; the sweep's file-backed impl at `scratch-app/src/main.rs:53-87` compiled on its first check and its record survived the simulated crash, `run.log` lines 4, 36, 38)
- Verification: confirmed; class reframed from feature-gap to documentation, since the resolution is an example and a shipped impl is a separate decision; history: no-rationale-found
- Owner-gated: no for the example; yes for shipping an implementation

The only `Bookmark` impl a user can see is `NoBookmark`. `store` takes a lent
boxed-future writer (`Serialized<'a>`) and carries an obligation the crate
cannot check, with "unspecified corruption" as the cost of getting it wrong,
and no example shows the temp-and-rename shape that satisfies it. The
sweep's impl compiled first try, so the docs are sufficient; the example is
what makes the cheapest artifact the intended one.

Evidence:

    src/bookmark.rs
    104	    /// Atomically replace the stored record.
    105	    ///
    106	    /// The crate serializes the framed record by calling `write` with a lent
    107	    /// writer. The implementor **must commit the written bytes atomically iff
    108	    /// `write` returns `Ok`** and must report an error rather than leave a
    109	    /// partial frame where the next [`load`](Self::load) could read it.
    110	    ///
    111	    /// Atomicity here is a safety obligation the crate cannot check, not
    112	    /// storage hygiene: a torn or reordered store whose next `load` yields
    113	    /// *valid but stale* bytes is indistinguishable from a record that never
    114	    /// covered the session, and its consequence is unspecified corruption:

Resolution: Add an `# Examples` block on `Bookmark` (or on `store`) with a
minimal file-backed impl: `load` reads the file into a `Cursor<Vec<u8>>`
(`Ok(None)` when absent), `store` buffers through the lent writer into a
`Vec<u8>`, writes a sibling temp file, and `rename`s over the target. Make
it a compiled doctest so it cannot rot. Owner option: ship that impl behind
a feature so the three test bookmarks and every user's copy collapse into
one. Acceptance: `cargo test --doc` compiles the example; the example is the
one the tutorial points to.

### session-bookmark-26: `is_current`'s doc says "exactly" while the code compares own-party projections, and its inline comment states only half the recording condition
- Where: src/bookmark.rs:282-298 (related: src/peer/gossip.rs:528-532, src/peer/gossip.rs:650-656, crates/before/src/version.rs:1703-1711, tests/bookmark_when.rs:12-28)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `impl<'a> Div<&'a Party> for &'a Version` at crates/before/src/version.rs:1703-1711, whose output is `OwnVersion`, the own-party projection; compared the doc text with the expression)
- Seen by: structure, prose; refutation: reframed (the comment is incomplete, omitting the party-changed arm, not inverted; severity lowered because tests/bookmark_when.rs:12-28 states the predicate correctly one grep away); history: deliberate but expired (the summary was accurate when the predicate was `v == version`; Finch's 9cf90dfb changed it to projections and added the inline comment the design rests on without touching the summary; gossip.rs:528-532 predates the change)
- Owner-gated: no

The doc says the token matches when `(party, version)` is "exactly what the last update persisted"; the code compares `v / p == version / p`, the versions' projections onto the recorded party, so an advance confined to other parties' identity space counts as current. That is the design decision the durability guarantee rests on (persist before gossiping own events only) and it lives in a code comment inside the closure rather than in the contract. The inline comment states the recording condition as "party is the same ... and the projections are not equal", which is the version-advance half only; the code records when the party differs or the projections differ. `bookmark_update`'s doc (gossip.rs:528-532) names both halves but leaves "the version advancing on new content" unqualified by own-party. This predicate gates the durability guarantee the bookmark exists for (gossip.rs:650-656 argues safety from it), so a maintainer needs the precise statement at the method.

Evidence:

    282	    /// Whether `(party, version)` is exactly what the last update persisted, so
    283	    /// re-recording it would be a no-op. The suppression test for
    284	    /// [`update`](crate::Peer::bookmark_update).

    287	            // We only need to record the bookmark when our party is the same as
    288	            // the last time we recorded, and the two versions *quotiented by
    289	            // our current party* are not equal, because we're trying to ensure

    296	            p == party && v / p == version / p

Resolution: State it positively at the method: "Current when the party is unchanged and the version's projection onto that party is unchanged. Only own events must be durably covered before they cross the wire; an advance confined to other parties' identity space changes nothing the record must dominate, so it does not defeat suppression." Rewrite the inline comment to describe what returns true (or delete it as redundant with the doc), and qualify "the version advancing on new content" in `bookmark_update`'s doc with "in this party's own identity space". Acceptance: doc and comment read as the same boolean the expression computes; a reader of the rustdoc alone can predict the answer for a version that advanced only in a foreign party's space.

### inventory-8: `ensure_loaded`'s doc promises a return value the signature lacks
- Where: src/bookmark.rs:302-308 (related: src/bookmark.rs:372, src/bookmark.rs:419, src/bookmark.rs:437)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read lines 301-313 and the three `expect("loaded before mutation")` sites; blame at 302 is b7fb409f, 2026-06-15)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The rustdoc says the function reads the record "returning it for mutation"; the signature returns `Result<(), BookmarkIo<B::Error>>`, and `slice`, `record`, and `reclaim` each re-fetch the record with an `expect`. Returning `&mut` from `ensure_loaded` would fight the `&mut self` methods that follow it, so the sentence, not the signature, is the thing to fix.

Evidence:

    302	    /// Read the stored record on first use, returning it for mutation. A no-op
    303	    /// once loaded; the mutex serializes access, and no mutation precedes a
    304	    /// load, so the read is the record's first content.
    ...
    308	    pub(crate) async fn ensure_loaded(&mut self) -> Result<(), BookmarkIo<B::Error>> {

    372	        let inner = self.inner.as_mut().expect("loaded before mutation");

Resolution: Drop "returning it for mutation" and state what the function does: loads the record into `inner` on first use so the mutators that follow find it present. Acceptance: the doc sentence and the signature agree.

### session-bookmark-27: `Bookmarked::slice` and the struct doc claim a `watch` critical section its only caller deliberately omits
- Where: src/bookmark.rs:363-370 (related: src/bookmark.rs:228-231, src/peer/gossip.rs:561-571)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '\.slice(' src/`, excluding `as_slice`, returns exactly one call site, gossip.rs:569, inside `bookmark_donate`, whose body holds only the bookmark mutex)
- Seen by: prose; refutation: confirmed, severity lowered (private prose, single caller, correct statement already at that caller, no lock-order hazard in code); history: no rationale found (the slice doc and the contradicting caller comment were written in the same commit; the false statement is original)
- Owner-gated: no

`slice`'s doc says it runs "inside the caller's `watch` critical section (so it moves with the party leaving `Inner`)", and the struct doc groups `reclaim` and `slice` under "a brief `watch` critical section nested inside the mutex". The single caller takes no `watch` section and says why: the party has already left `Inner`. Lock-discipline prose is where a false statement costs the most (Principle 5).

Evidence:

    366	    /// The synchronous half of donation, run inside the caller's `watch`
    367	    /// critical section (so it moves with the party leaving `Inner`); the

    562	    /// and persist. The party has already left `Inner` (forked off or taken
    563	    /// whole), so this needs no `watch` critical section.

Resolution: Re-state `slice`'s placement: "Runs under the bookmark mutex, after the donated party has already been removed from `Inner` (in the session's speculative critical section) and before it is sent; the caller writes afterwards." Amend the struct doc at 228-231 so only `reclaim` is described as nested inside a `watch` section. Acceptance: no doc claims `slice` runs under `watch`.

### session-bookmark-43: `Message`'s `# Panics` lists two serializing constructors; `try_new` serializes too
- Where: src/message.rs:36-43 (related: src/message.rs:357-364, 375)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: `try_new` at 359-364 calls `try_from_arc`, which calls `to_vec` at 375, which panics on a `Serialize` error at 285-290; `try_new`'s own doc at 357-358 defers to this section)
- Seen by: prose; refutation: confirmed; history: no rationale found (`try_new` was added before the prose-trimming pass that kept the two-item list)
- Owner-gated: no

The type-level hazard section is the contract of record for the serialize panic, and `try_new`'s doc points the reader here; the enumeration omits it. A hand-maintained list already out of date.

Evidence:

    38	/// Every payload value must serialize: methods that serialize
    39	/// ([`new`](Self::new), [`from_arc`](Self::from_arc)) panic if the

Resolution: Drop the enumeration: "every constructor that serializes panics if ...". Acceptance: every method whose body reaches `to_vec` is covered by the section's wording.

### session-bookmark-37: `Observer::session`'s "before the session's first byte crosses the wire" is not met for remote-led `gossip_when` sessions
- Where: src/observe.rs:71-73 (related: src/peer/gossip.rs:961-970, src/peer/gossip.rs:622, src/tree/mirror/handshake.rs:319-335)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: the driver's idle select reads the remote preamble into the staging buffer at gossip.rs:967, `drive.staged.fill(&mut *drive.read)`, before `gossip_inner` calls `self.observe.begin(kind)` at 622; delivery is intact because `handshake::preamble` reports the validated item from `staged.received()` at handshake.rs:334, after `begin`)
- Seen by: correctness; refutation: confirmed; history: no rationale found (written with the hook, after the driver already staged remote bytes before entering a session)
- Owner-gated: no

For a remote-led session, up to the whole preamble has been read from the wire before the observer is asked. The item-level coverage contract holds (the validated preamble is delivered whole to the control-received handler), so the cost is a misleading clause, not lost data; the clause is inherently about this side's writes.

Evidence:

    71	    /// Called once per session, before the session's first byte
    72	    /// crosses the wire. `session` identifies it; the returned
    73	    /// handler's lifetime is the session's.

Resolution: "Called once per session, before this side writes its first byte and before any item of the session is delivered to a handler." Acceptance: the clause reads true for one-shot and driver-led sessions; `tests/observe.rs`'s byte-for-byte mirror property passes unchanged.

### session-bookmark-4: `Gossiped::converged` doc overstates what both sides hold at commit
- Where: src/peer/gossip.rs:168-171 (related: src/peer/gossip.rs:581-583, src/peer/gossip.rs:810-812, src/peer/gossip.rs:869)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: correctness; refutation: confirmed; history: no rationale found (original wording from the driver commit; the private doc was later made precise without updating the public field)
- Owner-gated: no

The public field doc says both replicas "held exactly this version" at commit. The commit joins the reconciled tree into a tree that may have advanced during the session (`inner.tree.join(merged)` at 869), so a side that ran a `send` or `redact` concurrently holds a frontier strictly above `converged`. The private doc at 581-583 ("before any commits that ran concurrently with the session") and the comment at 810-812 state the accurate rule; the public contract disagrees with them.

Evidence:

    168	    /// The causal frontier the two replicas converged on.
    169	    ///
    170	    /// At the instant the session committed, both held exactly this version.
    171	    pub converged: Version,

Resolution: "The reconciled frontier both replicas absorbed at commit; a side that committed local work during the session holds a frontier above it." Acceptance: the field doc agrees with `gossip_inner`'s doc at 581-583.

### session-bookmark-9: `gossip_inner`'s return contract under-states the Retire-with-error arm
- Where: src/peer/gossip.rs:575-579 (related: src/peer/gossip.rs:782-802, src/peer/gossip.rs:894-899, src/peer/gossip.rs:126-129)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the doc named the only such path when written; the epilogue commit added the post-send epilogue failure without touching the doc)
- Owner-gated: no

The doc says `Intent::Retire` arrives with an error "when sending the party itself fails". The body also returns `(outcome, Err(..))` with `outcome == Intent::Retire` when the epilogue fails after a successful send (897-899, explained at 894-896). A reader of the doc alone would believe a post-hand-off epilogue failure maps to `Remain`, the duplication hazard the code guards against. The public `Retire::Uncertain` doc (126-129) covers both cases; only this private contract is incomplete.

Evidence:

    577	    /// off to the counterparty via retirement. `Intent::Retire` can arrive
    578	    /// *with* an error: when sending the party itself fails, we cannot know
    579	    /// whether the remote received it, so we must assume it might have.

Resolution: "`Intent::Retire` can arrive with an error whenever the party may already be held by the peer: the send itself failed, or the send succeeded and the epilogue after it failed." Acceptance: the doc names both post-hand-off failure paths.

### session-bookmark-16: "Honest", "genuine", and "real" as moralized adjectives where the crate's trust model owns the word
- Where: src/peer/gossip.rs:1282-1284 (related: src/peer/gossip.rs:383; src/peer/gossip/tests.rs:10, 84, 112, 155, 158, 318, 345, 349, 375, 377, 383; src/bookmark/format.rs:48; src/bookmark/format/tests.rs:19; tests/common/sim.rs:359-395)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep -n -i for `honest|genuine|real` across the partition; AGENTS.md's model-of-record rule uses "authenticated-honest-peer"; tests/common/sim.rs:359-395 uses "honesty of failures" and `is_honest_error` for EOF-versus-corruption)
- Seen by: prose; refutation: confirmed; history: no rationale found for most sites; one collision noted: "honest wire cut" borrows the sim harness's failure-honesty vocabulary (Finch's b7fb409f), which predates the AGENTS.md trust-model sense (a59dc786), so two in-crate senses of "honest" collide
- Owner-gated: no

"Honest wire cut" (a plain EOF), "honest bootstrap" / "honest newborns" / "genuinely newborn" (a claimant with an empty greeting version), "genuinely untouched", "real semantics", "a genuine `events`-tick version". The cost is specific: the model of record is "authenticated-honest-peer", so "honest" is a term of art for the trust assumption, and using it for "an EOF rather than a bad byte" or "a newborn rather than a misdeclaring claimant" blurs the term at exactly the sites (bootstrap history conflict) that are conformance-bug detectors, not trust boundaries. The writing-style rule asks for the property instead of the adjective.

Evidence:

    1282	/// resolves, so the exchange cannot deadlock. Failure is [`Error::Epilogue`]:
    1283	/// post-commit by construction, with a non-marker byte surfaced as an
    1284	/// invalid-data protocol violation rather than an honest wire cut.

    318	/// encounter were two honest newborns.

Resolution: "a plain wire cut" / "an EOF"; "a newborn claimant (empty greeting version)"; "a bootstrap whose greeting version is empty"; "untouched"; drop "real" and "genuine" where the noun carries the meaning. Decide separately whether the sim harness keeps its "honesty of failures" vocabulary or renames toward "cut versus corruption", so the crate has one sense of the word. Acceptance: `grep -n -i 'honest\|genuine' src/peer/gossip.rs src/peer/gossip/tests.rs src/bookmark/format/tests.rs` returns nothing; "honest" appears in the crate only in its trust-model sense.

### session-bookmark-33: The greeting field list omits the payload depth limit
- Where: src/reconciliation.rs:53-57 (related: src/tree/mirror/streaming/message.rs:62-118, src/peer/gossip.rs:1419-1429)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `pub` fields in `Greeting`: `version`, `set_len`, `max_version_bytes`, `target_message_size`, `payload_depth_limit` (line 115), `listing`)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the list was complete when written; the commit that added the field, 71de90c1, updated the codec and its tests but not this page)
- Owner-gated: no

The page lists five greeting fields; the struct carries six. The omitted one is user-visible: `Peer::payload_depth_limit` is a public setting and `Error::PayloadDepthMismatch` a public error whose cause is the greeting comparison, so the list that should explain that error omits its field. Line 55 also runs far past the paragraph's measure.

Evidence:

    53	//! After a fixed transport preamble, a session opens with a *greeting*:
    54	//! each side sends its version, its live-message count, its version-size
    55	//! bound, its message-size target, and its root's child listing. Equal versions mean identical

Resolution: Add "its payload depth limit" with one clause ("the two limits must match, or the session fails before any descent") and re-wrap. Acceptance: the list names every field of `Greeting`; no line exceeds the surrounding measure.

### session-bookmark-34: The digest-width section carries an adversary work-factor sentence that lost its "off-model" label
- Where: src/reconciliation.rs:164-177 (related: src/reconciliation.rs:157-162; AGENTS.md hard rule on adversary economics)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git show 2d1e6ea5 -- src/reconciliation.rs` introduced the sentence under an "Off-model note:" heading, and its commit message records "adversary-economics arguments are demoted to explicit off-model notes per the model of record"; 9c73d7b4 replaced that text with the current unlabeled form)
- Seen by: prose; refutation: reframed (the acceptance at 157-162 is on-model; the supplementary sentence is what the rule forbids resting on); history: already known (owner ruling to keep it as an explicit off-model note; what drifted is the label and the framing)
- Owner-gated: no for restoring the recorded framing; deleting the sentence would reverse the ruling and is the owner's call

The page prices the width on the on-model accident bound (157-162), then adds that "against any such actor, the 24-byte width keeps the offline birthday floor at 2⁹⁶ evaluations". The recorded ruling was to keep that as an explicit off-model note; the current text presents it as part of the acceptance argument, and AGENTS.md's hard rule says no pricing argument may rest on adversary economics. A reader meeting the paragraph reasonably infers the width was partly chosen for collision resistance against an actor.

Evidence:

    170	//! versions get created (an actor steering gossip schedules steers the
    171	//! version set); against any such actor, the 24-byte width keeps the
    172	//! offline birthday floor at 2⁹⁶ evaluations, an unconditional bound that
    173	//! rests on no premise about capabilities. Hostile *peers* remain

Resolution: Restore the ruling's framing: keep the structural claim (message bytes contribute zero bits to any compared digest, a property of the construction) and mark the birthday-floor sentence explicitly as an off-model aside that is not part of the acceptance ("Off-model note: ..."), or move it after the hostile-peers sentence so the paragraph's argument visibly ends at the accident bound. Acceptance: the section's acceptance rests on the per-comparison accident bound alone, and any adversary sentence is labeled as outside the model.

### session-bookmark-36: Public explanation names `Tree::join`, unreachable from the API
- Where: src/reconciliation.rs:246-249 (related: src/lib.rs:322, 348-349; AGENTS.md orientation)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: lib.rs:322 `mod tree;` is private and lib.rs:348-349 re-export only `MERKLE_HASH_LEN` and `SessionStats` from it; lib.rs:315 `pub mod reconciliation`)
- Seen by: prose; refutation: confirmed (a plain code span, not a broken link, but a user cannot find the item); history: no rationale found (fresh authorship by 368da2a5 beyond what the retirement plan asked for; AGENTS.md already places the oracle fact at the streaming module doc)
- Owner-gated: no

`Tree` is not exported, so a user of this page cannot find `Tree::join`. The sentence is maintainer content (the test oracle) and already lives where the maintainer looks. Public rustdoc names nothing the API does not reach.

Evidence:

    246	//! case. [`Protocol::V2`](crate::Protocol::V2) instead runs the descent
    247	//! using the bounded-memory streaming approach described above; its
    248	//! behavioral oracle in the test suite is the in-memory merge
    249	//! (`Tree::join`), which honors deletions through the same filter.

Resolution: End the paragraph at "described above"; leave the oracle statement to the streaming module doc and AGENTS.md. Acceptance: `reconciliation.rs` names no item absent from the public surface.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| session-bookmark-28 | src/bookmark/format.rs:61-69; format/tests.rs:171-173, 190-191 | "Version 5 is ..." and "0x18 0x05" hand-maintain the format version; "the earlier frame shapes" refers to formats no longer in the tree. | "The current version ..."; "0x18 <version>"; "no decoder exists for any lower version number". |
| session-bookmark-45 | src/message.rs:53-59 | The default depth limit is justified by a fleet upgrade with no prior release to upgrade from. | State the fact positively: at the default, admission enforces nothing the decoder does not. |
| api-audit-10 | src/message.rs:116-117 (and the `#[non_exhaustive]` sites listed) | The rule for which enums are open lives only in commit 1e458d69, so nothing distinguishes the split from accident. | State the rule in the `error` module doc; name `EncodeError`'s class at its definition. |
| clippy-pedantic-11 | src/observe.rs:182 (three test-side sites of a different shape) | A code span split around an intra-doc link renders as two fragments. | `` [`0..STREAM_COUNT`](crate::link::STREAM_COUNT) ``; the test sites take their own rewrite. |
| session-bookmark-5 | src/peer/gossip.rs:177-178, 1201 | "seam" in the public `Gossiped::stats` doc and "knob" in a private comment. | "knob" to "setting" now; "seam" is a crate-wide ruling (owner-gated). |
| session-bookmark-6 | src/peer/gossip.rs:256-257 (23 sites in the partition) | Em-dashes in `//` comments. | Crate-wide pattern. |
| session-bookmark-13 | src/peer/gossip.rs:825-828 | "Unreachable in practice" where the one-line proof (a live `Peer` always holds its party) is available. | Name the invariant, not its rarity. |
| prose-hygiene-9 | src/peer/gossip.rs:879 (169 `//` lines and 95 `#` lines crate-wide) | Em-dashes in line comments, the register the doctrine assigns the spaced double-hyphen. | Crate-wide pattern; see the patterns section. |
| session-bookmark-14 | src/peer/gossip.rs:901-904 and eight related sites | Prose mechanics: a misplaced comment, a mid-doc link definition, ragged wraps, "self-inverse" for invertible, a stale cross-reference, two grammar slips. | Fix each as the entry lists. |

## Link

### async-hazards-2: The retire cancellation carve-out promises bookmark recovery in the window where none exists
- Where: src/link.rs:313-316 (related: src/link.rs:318-321, src/peer/gossip.rs:773-782, src/bookmark.rs:363-364, src/peer/gossip.rs:108-110 and 121-124, src/peer/gossip.rs:1384-1404, tests/retire.rs, src/tests.rs:47-56)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read)
- Verification: reframed: the sentence is accurate over the window where `Err` would return `Retire::Recovered` and inaccurate over the window where `Err` would return `Retire::Uncertain`, and it does not scope itself; history: no-rationale-found (`git log -S'recoverable only through'` finds no commit touching the phrase under the paths searched, and the link-transport review packet's carve-out remarks concern the epilogue confirmation, R54)
- Owner-gated: no

The carve-out says a dropped `retire` future "loses the identity (recoverable
only through an attached bookmark)". `bookmark_donate` slices the whole party
out of the record and persists (gossip.rs:774, bookmark.rs:363-364) before
`party::send` runs (gossip.rs:782). A drop landing after that persist commits
(the tail of `bookmark_donate`, `party::send`, or the epilogue wait) destroys
the `Peer` with the party recorded in no bookmark on our side; it survives
only if the counterparty received and committed it. That is the same loss
`Retire::Uncertain` documents for `Err` in the same window, so the sentence's
contrast ("where `retire`'s `Err` would have handed the peer back") also holds
only for the `Recovered` window. The doc immediately below tells callers to
wrap sessions in a timeout and treat expiry as cancellation; a timeout fires
most plausibly while awaiting the stalled peer's epilogue, which is inside the
misdescribed window. No test cancels a `retire` mid-flight: `tests/retire.rs`
has no drop, cancel, or bounded-poll site (grep for
`drop|cancel|abort|poll|mid` is empty), and every `retire` in `src/tests.rs`
runs to completion under `tokio::join!`.

Evidence:

    src/link.rs
    313	///   carve-out: a `retire` future owns its consumed [`Peer`](crate::Peer),
    314	///   so dropping it destroys the peer and loses the identity (recoverable
    315	///   only through an attached bookmark), where `retire`'s `Err` would have
    316	///   handed the peer back through [`Retire`](crate::Retire)'s variants.
    ...
    318	/// No session imposes its own deadline: against a stalled peer a session
    319	/// waits forever, so the *caller* owns the timeout. Wrap sessions in your
    320	/// runtime's timeout and treat expiry as any other cancellation: replica
    321	/// intact or fully committed, link poisoned, reconnect.

    src/peer/gossip.rs
    773	            let donated = guarded.party.as_ref().expect("is_some");
    774	            if let Err(e) = self.bookmark_donate(donated).await {
    775	                return (Intent::Remain, Err(Error::Bookmark(e)));
    776	            }
    ...
    781	            let donated = guarded.party.take().expect("is_some");
    782	            match party::send(donated, write, &observe).await {

    src/bookmark.rs
    363	    /// Slice the donated `party` out of the record, since it has now left for
    364	    /// the network.

    src/peer/gossip.rs (Retire variants)
    108	    /// **Recovered, unchanged.** The session failed *before* our identity
    109	    /// ever crossed the wire; the replica is handed back intact, to try
    110	    /// retiring elsewhere.
    ...
    121	    /// **Uncertain.** The session failed while our identity itself was in
    122	    /// flight: the peer may or may not hold it, so our peer is consumed
    123	    /// rather than risk the same identity living twice. The link is
    124	    /// poisoned; discard it.

Resolution: Reword the carve-out to scope itself by the `Retire` variant the
same failure would have produced: where `Err` would return `Recovered`, a drop
instead destroys the peer and the identity survives only in an attached
bookmark; where `Err` would return `Uncertain` (the donation persisted, the
party frame or the epilogue in flight), a drop loses the identity exactly as
`Uncertain` does, and the local bookmark no longer records it. Add a test that
drives `retire` against a counterparty with the bounded-poll harness
`tests/lifecycle.rs` already uses for mid-descent cancellation, cuts at each
of the three await points (before `bookmark_donate`, after it, during
`party::send`), and asserts the record's contents and the counterparty's party
at each cut. Acceptance: the sentence names both windows, and a committed
test under `tests/` cancels `retire` after the donation persist and asserts
the bookmark record no longer contains the party.
Construction: attach an in-memory `Bookmark` to a peer, start `retire` against
a counterparty over `link::memory`, poll both manually until the retiree's
`bookmark_donate` write has completed but `party::send` has not (stall the
retiree's control write), drop the retire future, then load the bookmark
bytes: the retiree's party is absent (sliced) and no live handle holds it.

### fresh-eyes-7: The BufReader advice states the mechanism without its reason, and the routed module inherits nothing
- Where: src/link.rs:76-78 (related: src/link/routed.rs:86-103, src/link/routed.rs:193-208, src/link/routed/endpoint.rs:20-25, src/tree/mirror/framing.rs:18-24)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -rn "BufReader\|BufWriter" src` finds only the two doc mentions, no code; framing.rs:18-24 and the cbor-wire review's N1 note read; the sweep's TCP session over a bare `TcpStream` completed, `run.log` line 43: `tcp: alice gained 0 shed 0 bytes 69/0; bob2 gained 1 shed 0 bytes 0/69`)
- Verification: reframed: the advice is a syscall-count mitigation (the codec reads CBOR heads byte-wise and digests in 32-byte `read_exact`s; payload chunks bypass an 8 KiB buffer), which the private framing.rs doc and the review packet both say and the public sentence does not; history: already-known (`.agent-notes/2026-08-20-cbor-wire-review/REVIEW.md` N1 placed the sentence in link.rs verbatim and recorded "If left undone, no invariant is at risk"; the routed module predates that ruling and was not considered by it)
- Owner-gated: no for the wording; yes for having the adapter wrap its own read halves

The public transport contract instructs "wrap the read half in
`tokio::io::BufReader`" without saying it is a throughput measure, so a
`Link` author cannot tell whether omitting it is a correctness risk. The
routed adapter builds `RoutedLink` from raw `ReadHalf<D::Conn>` halves, its
TCP example hands over a bare `TcpStream`, and its "What the transport must
provide" section is silent, so a routed-TCP user cannot tell whether the
adapter applied the advice, whether their `Conn` should, or whether it
matters. (tokio's `BufReader<T>` forwards `AsyncWrite` to `T` unbuffered, so
`Conn = BufReader<TcpStream>` satisfies the no-hidden-write-buffering rule;
assessed from tokio's API, not run.)

Evidence:

    src/link.rs
    76	//! Reads are exact and item-granular; on an unbuffered transport, wrap the
    77	//! read half in `tokio::io::BufReader` — caller-owned buffering outlives a
    78	//! session and is safe across session boundaries.

    src/tree/mirror/framing.rs (private module doc)
    //! The price is read batching: capacity-bounded payload reads instead of
    //! one large buffered read. A caller wanting fewer reads on a raw socket
    //! can wrap it in [`tokio::io::BufReader`] sized above
    //! [`PAYLOAD_CHUNK_LEN`] — at the default 8 KiB capacity nearly every
    //! payload read outsizes the buffer and bypasses it.

    src/link/routed/endpoint.rs
    20	pub type RoutedLink<D> = Link<
    21	    ReadHalf<<D as Dial>::Conn>,
    22	    WriteHalf<<D as Dial>::Conn>,

Resolution: At link.rs:76-78, add the reason in one clause ("a throughput
measure: the codec reads frame heads and digests item by item, so an
unbuffered socket pays a read syscall per item; correctness does not depend
on it"). In routed.rs's "What the transport must provide" section, one
sentence: the adapter does not buffer; a `Dial` whose `Conn` is a raw socket
may hand over `BufReader<TcpStream>` (write-through, so the `Conn` rule
holds). Owner option: have the adapter wrap its own read halves, since it
owns the split. Acceptance: both pages state whether the wrapper is
optional and why; the routed example either wraps or says why it does not.

### link-4: `Connector::connect` and `Endpoint::link` have no `# Cancel safety` section; their accept-side duals do
- Where: src/link.rs:220-227 (related: src/link.rs:252-257, src/link.rs:64-68, src/link/routed/endpoint.rs:246-252, src/link/routed/endpoint.rs:295-298, src/link/routed/router.rs:269-277, src/link/routed/tests.rs:412-461, src/tree/mirror/streaming/remote/streams.rs:159-168)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read both trait docs, both endpoint methods, and the router's ACK path)
- Seen by: perfapi (41), correctness (29a); refutation: confirmed both; history: no rationale (0eff1021 scoped the cancellation clause to accept drops; `Incoming::accept` got its section in b16a800b while `link` did not)
- Owner-gated: no

Hazards get uniform named sections. `Acceptor::accept` and `Incoming::accept` each carry `# Cancel safety`; `Connector::connect` and `Endpoint::link` are silent, which a reader takes as "nothing to say". Both have something to say. For `connect`: the session retains a pending open until it resolves and drops one only at teardown, an implementation must leave the link usable after a drop at any stage, and a completed-then-dropped open surfaces to the peer as an empty stream (the routed test `cancelled_opens_leave_the_link_usable` asserts exactly this, with no clause to cite). For `Endpoint::link`: dropping the future after the peer's router has written ACK and delivered through `slot.send` leaves the peer's application holding a link whose control connection closed at the drop, so its first session fails as transport failure; nothing leaks (the dialer's `Registration` drops with the future), and "nothing needs cleaning up" at endpoint.rs:251 is true of the `Err` arms only.

Evidence:

    220	    /// Open one outgoing unidirectional stream, paired with where the
    221	    /// half goes at its clean end.
    222	    ///
    223	    /// # Errors
    224	    ///
    225	    /// Fails only for transport reasons (the link is gone); the session
    226	    /// treats any error as fatal to the session, never retries.
    227	    fn connect(&self) -> impl Future<Output = io::Result<(Self::Tx, Done<Self::Tx>)>> + Send;

    (endpoint.rs)
    251	    /// Either way no link exists and nothing needs cleaning up; retry
    252	    /// policy is the caller's.

Resolution: add `# Cancel safety` to `Connector::connect` stating the retention discipline and the empty-stream residue, and mirror it in the contract's cancellation clause (link.rs:64-68), which today speaks only of `accept`. Add `# Cancel safety` to `Endpoint::link` stating the peer-side residue (a delivered link whose control stream is already closed; the peer's first session fails as transport failure and it re-links). The routed test's doc can then cite the clause it exercises. Acceptance: both sections present; `just doclint` clean.

Synthesis note: api-audit-16 lists the same inconsistency from the public-surface side.

### link-7: `Link::new` states its precondition as "at the same time"; the invariant is "before any session"
- Where: src/link.rs:418-420 (related: src/link.rs:339-340, src/link.rs:427-430)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); the history claim verified via `git show c5f1210a -- src/link.rs`
- Seen by: prose (21); refutation: confirmed; history: no rationale, and the founding wording (b3b877d9) carried the correct explanation ("the epoch counters start at zero on both sides and advance in lockstep"), which c5f1210a replaced with a pointer
- Owner-gated: no

What `SessionState`'s lockstep needs is that both ends wrap the same transport with no session yet run on it, so both start at epoch zero with the latch clear. Simultaneity is neither necessary nor what any code checks; a public precondition stating a different condition than the one relied on misleads the implementer.

Evidence:

    418	    /// Both ends of a connection must construct their links around the
    419	    /// same fresh transport at the same time; the bookkeeping the two ends
    420	    /// then keep in step is [`SessionState`]'s.

Resolution: "Both ends wrap the same transport before any session runs on it, so both start at epoch zero with the latch clear; the bookkeeping the two ends then keep in step is [`SessionState`]'s." Acceptance: the doc states the fresh-transport condition and not simultaneity.

### api-audit-6: The routed-link example names a `peer.rumors()` method that does not exist
- Where: src/link/routed.rs:159-160 (related: src/peer.rs:616-625)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'fn rumors\b' src tests benches examples` returns nothing; the only conversion is `Peer::into_rumors` at peer.rs:623; `git log -S'peer.rumors()'` attributes the line to b16a800b5)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The TCP instantiation example closes by telling the reader to run sessions
with `peer.rumors().gossip(&mut link_at_b)`. `Peer` has no `rumors()`; the
conversion is `into_rumors`. The call sits in a comment, so the doctest
passes and nothing catches it: the one place a transport author reads
first carries a ghost method.

Evidence:

    src/link/routed.rs
    159	//! // Each side now runs sessions on its link:
    160	//! // `peer.rumors().gossip(&mut link_at_b)`, etc.
    161	//! # let _ = (&mut link_at_a, &mut link_at_b);

Resolution: write `rumors.gossip(&mut link_at_b)` with `let rumors =
Peer::<String>::seed().into_rumors();` introduced earlier in the example,
or make the trailing line live code (for example a closure over
`rumors::Rumors<u64>` that calls `gossip`). Acceptance: the example's
session line is compiled code or names `into_rumors`.

Synthesis note: Same site as fresh-eyes-6, which adds that the comment was wrong when written rather than orphaned by a rename.

### fresh-eyes-6: Routed TCP example comment names a nonexistent `peer.rumors()`
- Where: src/link/routed.rs:159-160 (related: src/peer.rs:623)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn "fn rumors(" src tests examples` is empty; `pub fn into_rumors(self) -> Rumors<T, B>` at src/peer.rs:623; `git show b16a800b5:src/peer.rs`, the commit that wrote the example, already has `into_rumors` and no `rumors()`, so the comment was wrong when written, not orphaned by a rename)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The doctest's closing hint names a method that has never existed; being a
comment, `cargo test` cannot catch it, and it is the last line a first-time
TCP user reads before writing their own code.

Evidence:

    src/link/routed.rs
    159	//! // Each side now runs sessions on its link:
    160	//! // `peer.rumors().gossip(&mut link_at_b)`, etc.

Resolution: `//! // `peer.into_rumors().gossip(&mut link_at_b)`, etc.` (or
show `let rumors = peer.into_rumors();` first). Acceptance: every method
named in the routed example resolves (`grep -rn "fn into_rumors" src`
non-empty; `grep -rn "\.rumors()" src` empty).

Synthesis note: Same site as api-audit-6.

### link-18: `LinkError::Rejected` and `Endpoint::link`'s `# Errors` misstate the causes: the router never "answers" a rejection, and the address-decode mismatch is unnamed
- Where: src/link/routed/endpoint.rs:117-121 (related: src/link/routed/endpoint.rs:246-252, src/link/routed/endpoint.rs:262-270, src/link/routed/endpoint.rs:285-286, src/link/routed/router.rs:159-163, src/link/routed/router.rs:252-257, src/link/routed/router.rs:266-268, src/link/routed/header.rs:312-313, src/link/routed/tests.rs:320-346, src/link/routed/tests.rs:378-389)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (traced every path that ends in `Rejected`)
- Seen by: prose (12), correctness (29b), perfapi (38); refutation: confirmed all three, merged; history: the EOF-to-`Rejected` mapping is deliberate (design record §5 "EOF or a non-ACK byte is a crisp rejection"), the two-cause doc was imprecise from b16a800b, distinguishing busy from foreign was never considered
- Owner-gated: no for the doc fix; the NACK-byte option is a wire decision and is in open questions

The variant says the peer's router "answered but did not accept", yet the router never writes a non-ACK byte in any rejection path: it drops the connection on a full backlog (router.rs:266-268), a dropped `Incoming` (same `try_reserve`), a duplicate token (router.rs:252-257), a pending-header eviction of the `LINK` connection (router.rs:159-163), and any header it cannot parse, including an advertised name the peer's `Addr::decode` returns `None` for (header.rs:312-313). That last cause is what a deployer sees when two endpoints disagree on their `Addr` type or encoding, and neither doc names it. Separately, a full backlog (transient; retry) and a foreign listener (permanent; fix the deployment) reach the dialer identically as `UnexpectedEof`, so the "retry policy is the caller's" it hands over cannot be informed. Public `# Errors` prose must state every arm accurately against the code.

Evidence:

    117	    /// The peer's router answered but did not accept the link: its
    118	    /// application is not accepting links, or the listener is not a
    119	    /// routed-link router at all.
    120	    #[error("the peer's router rejected the link")]
    121	    Rejected,

    248	    /// [`LinkError::Io`] for transport failure, [`LinkError::Rejected`]
    249	    /// when the peer answered without acknowledging (its application's
    250	    /// backlog is full, or the listener does not speak this wire).

    264	            // A clean close or a non-acknowledgement byte is the
    265	            // peer's router declining; transport trouble stays Io.
    266	            Ok(_) => return Err(LinkError::Rejected),
    267	            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
    268	                return Err(LinkError::Rejected);
    269	            }

    (router.rs)
    266	            let Ok(slot) = incoming.try_reserve() else {
    267	                return Ok(());
    268	            };

Resolution: rewrite the variant doc and `Endpoint::link`'s `# Errors` around what the dialer observes (the peer closed the connection, or wrote a byte other than the acknowledgement, before acknowledging) and name the causes the router has: its application's backlog is full or `Incoming` was dropped, the advertised name did not decode at the peer (the two endpoints' `Addr` types or encodings disagree), the router was at its pending-header bound, or the listener is not a routed-link router. Drop "answered". State that the busy and foreign cases are indistinguishable by design unless the owner takes the NACK-byte option. Acceptance: the doc lists no cause the code does not produce and omits none it does; the address-decode cause appears by name; `just doclint` clean.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| link-1 | src/link.rs:19-22, 298-299, 547-549, 376-377 | An expired "cheap", a "stated where they arise" clause that precedes the full list, a hand-maintained "8 KiB", a missing blank line. | Four edits as the entry lists. |
| link-2 | src/link.rs:107-113 | The pooled-flow-control paragraph narrates a past observation that a committed test now pins. | Present-tense restatement naming the pin (owner-gated: owner phrasing). |
| link-13 | src/link/routed.rs:68-70 and the sites listed | "honest"/"misbehavior" where "conforming" is the claim; "real"/"genuine"; "seam", "knobs", "compatibility door", "tripwire". | Reword per site; owner vocabulary, so batch for a prose pass. |
| link-15 | src/link/routed.rs:252-259 | `Dial::recycle` obligates pooling dials around a router-written byte whose value the public contract never states. | State that the value is unspecified and must be consumed; list the router-to-dialer bytes in the layout block. |
| link-23 | src/link/routed/header.rs:186-187; routed/tests.rs:663-664 | Encoding is attributed to the router; `Endpoint::new` encodes. | "(the one place the adapter encodes: `Endpoint::new`)". |

## Conformance

### conformance-2: Suite summaries name the link contract's clauses in words the contract does not use
- Where: src/conformance.rs:5-7 (related: src/conformance/link.rs:4-5, src/conformance/link/tests.rs:155-156, src/conformance/link/tests.rs:1026-1027, src/link.rs:55-63, src/link.rs:107-113)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'half-close\|half close\|measured\|router' src/link.rs` returns only line 149, the routed adapter's "per-process router"; the clause at src/link.rs:55 is `**Completion.**`; src/link.rs:108-109 read "have been *observed* live" and "That is observed behavior"; `router.helper` appears crate-wide only at link/tests.rs:155)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the clause was named **Half-close** until 4be4b830 renamed it **Completion**; "measured" matched link.rs's wording until c5f1210a rewrote it to "observed"; "router-helper" cited a design-doc section a3da46c4 excised from code)
- Owner-gated: no

Three cross-references into the link contract use terms the contract no longer uses, so a reader following the link has to translate: "half-close" for the **Completion** clause (both module docs), "the link docs' measured-tolerance sentence" for a sentence that says *observed*, and "the router-helper shape" for a shape the cancellation clause describes without naming a router. Each was accurate when written and lost its referent to a later commit; the rule is established terms of art only, and a summary's anchor is the cited text's own vocabulary.

Evidence:

         5	//! [`Link`](crate::link::Link) transport delivers the stream independence,
         6	//! flow control, half-close, and cancellation tolerance the [link
         7	//! module](crate::link) requires of every implementation. Available from a

    src/conformance/link.rs:
         4	//! contract](crate::link): a full-duplex control stream, independent
         5	//! receiver-paced streams, half-close, and accept-cancellation tolerance.

    src/conformance/link/tests.rs:
       155	/// This is the router-helper shape the cancellation clause exists to
       156	/// exclude: if the `accept` future is dropped between the internal

    src/conformance/link/tests.rs:
      1026	/// The sessions run at the serialization floor, the shape the link docs'
      1027	/// measured-tolerance sentence is denominated in: a sub-bound pool couples

Resolution: Write "completion" (or "stream completion") at conformance.rs:6 and link.rs:5; write "observed-tolerance sentence" or quote its key phrase at tests.rs:1027; drop "router-helper" at tests.rs:155 (the next clause already states the shape: dequeue, then await). Acceptance: every clause name the conformance docs use appears verbatim as a clause heading in src/link.rs's contract section.

### prose-hygiene-10: "Seam" used 66 times as undefined jargon for a trait or interface boundary, reaching public rustdoc
- Where: src/conformance/backend.rs:17 (related: src/tree/mirror/streaming/stats.rs:6, 29; src/peer/gossip.rs:177; src/tree/mirror/streaming/erased.rs:1; the remaining 61 sites the sweep listed, regenerable with `grep -rniE '\bseam(s)?\b'` over the scope)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (66 lines, 47 in `///`/`//!`; no gloss found by `grep -rniE '\bseam\b.*(\bis\b|means|:)'`; `pub mod conformance` at src/lib.rs:307; `pub use tree::mirror::streaming::stats::SessionStats` at src/lib.rs:349; `pub stats: SessionStats` on `Gossiped` at src/peer/gossip.rs:179)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

"Seam" stands in for "the trait boundary at which a check or count is
taken" and is defined nowhere. It reaches the public `SessionStats` type
docs, the `Gossiped::stats` field doc, and the `conformance` feature's
backend module doc, where a library user must infer the meaning from
repetition.

Evidence:

    src/conformance/backend.rs
    17	//! - **Bulk seams**: the backend's own [`leaves`](Backend::leaves) and

    src/tree/mirror/streaming/stats.rs
    29	/// Every count is taken locally, at the seam named in its field docs, while

    src/peer/gossip.rs
    177	    /// See [`SessionStats`] for each field's mechanism and the seam it is

Resolution: In public rustdoc name the boundary ("**Bulk methods**: the
backend's own `leaves` and `assemble` overrides"; "at the point named in its
field docs"; "the mechanism and the point it is counted at"). In private
comments and tests, "boundary", "interface", or the method name is a
one-word substitution. Acceptance: `grep -rniE '\bseam(s)?\b' src` returns
nothing in `///` or `//!` lines; private uses at the owner's discretion.

### conformance-29: `run`'s doc says it returns the peak above the resting corpora; the ledger's peak is absolute
- Where: src/conformance/backend.rs:671-672 (related: src/conformance/backend.rs:114-121, src/conformance/backend.rs:663, src/conformance/backend.rs:708-715)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`reset_peak` stores `LIVE` into `PEAK` at 115-117; `run` calls it after the corpora exist at 708 and returns `ledger::peak()` at 715 and 725; the subtraction is `check`'s at 663)
- Seen by: prose; refutation: confirmed; history: no rationale found (doc and arithmetic born together in 922db57a)
- Owner-gated: no

`run` returns the high-water mark of absolute live bytes since `reset_peak` seeded it with the resting corpora included; the differencing that yields bytes above the corpora happens in `check`, between two absolute peaks. A maintainer reasoning from this sentence expects `run` to return a small number and misreads `check`'s `saturating_sub`.

Evidence:

       671	/// One controlled-divergence reconciliation; returns the ledger's peak
       672	/// measured bytes above the resting corpora.

Resolution: "One controlled-divergence reconciliation; returns the ledger's peak live bytes during the session, the resting corpora included. `check` differences two such peaks to isolate the window's own admittance." Acceptance: the doc names what the function returns and attributes the differencing to `check`.

### conformance-5: Hand-maintained tallies and ratios in prose, one of them a compile-time relation the compiler could hold
- Where: src/conformance/link.rs:87-88 (related: src/link.rs:532, src/conformance/link/tests.rs:1074-1075, src/conformance/link/tests.rs:630, src/conformance/backend.rs:580-581, src/tree/mirror/streaming/window.rs:361, src/conformance/backend/tests.rs:66-68, src/conformance/backend/tests.rs:1-2)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (src/link.rs:532 `const MEMORY_STREAM_CAPACITY: usize = 8 * 1024;` is private; window.rs:361 reads `for window in [0usize, 1, 16, FAN].windows(2)`; tests.rs:630 `const CAPPED_STREAMS: usize = 4;`; tests.rs:129-132 runs `Local` as the only crate backend)
- Seen by: prose, perfapi; refutation: confirmed; history: no rationale found for four sites; the +0.7 GiB figure was placed inline deliberately by b204fe56 under the design-doc-citation ban, but the height-erasure note records that erasure "shrank every tower from the inside", so the measurement predates the code it now describes
- Owner-gated: no

Five docs restate enumerable facts the code can change without touching the prose: (a) "32 KiB is four times the in-memory reference's buffer" is a ratio to the private `MEMORY_STREAM_CAPACITY`, and if that constant grows past 32 KiB the duplex probe against `memory()` stops overrunning the buffer while `memory_link_conforms` keeps passing; (b) "the fixture's fifth open" is `CAPPED_STREAMS + 1`; (c) "a four-point `debug_assert`" counts an array literal in window.rs; (d) "+0.7 GiB of rustc peak memory per additional instantiation" is a dated measurement at a declaration site whose rationale (one type, so the tower instantiates once) stands without the number; (e) "this crate's backends" is plural for one. Principle 5: state the structure, not the tally; a number that matters lives in a mechanically enforced place.

Evidence:

        87	/// this much hidden buffering (see the module docs). 32 KiB is four times
        88	/// the in-memory reference's buffer and past common transport defaults,

    src/conformance/backend.rs:
       580	/// The derivation's own check is a four-point `debug_assert`, compiled
       581	/// out of release, so the suite sweeps the grid: every adjacent fan pair

    src/conformance/backend/tests.rs:
        66	/// backend type instantiates the whole height-indexed protocol tower
        67	/// (measured at +0.7 GiB of rustc peak memory per additional
        68	/// instantiation), so the honest and lying variants must share one type.

Resolution: (a) make `MEMORY_STREAM_CAPACITY` `pub(crate)` and pin the relation, `const _: () = assert!(CONTROL_DUPLEX_FILL > crate::link::MEMORY_STREAM_CAPACITY, "the duplex probe must overrun the reference link's control buffer");`, then let the prose state the inequality, not the ratio; (b) "the open past [`CAPPED_STREAMS`]"; (c) "a sparse `debug_assert` over a handful of fans"; (d) keep the mechanism ("a large compile-time cost per instantiation") and drop the figure; (e) "this crate's backend". Acceptance: no doc in the partition restates a count, ratio, or measurement a sibling constant or array literal owns, and a const assertion ties `CONTROL_DUPLEX_FILL` to `MEMORY_STREAM_CAPACITY`.

### conformance-12: The concurrency check's docs assert backpressured elders that exist only below the probe size
- Where: src/conformance/link.rs:700-708 (related: src/conformance/link.rs:47-49, src/conformance/link.rs:731-734, src/conformance/link.rs:751-754, src/link.rs:532, src/link.rs:550-551)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`PROBE` is 24 bytes by `printf | wc -c`; `memory()` builds every stream as `tokio::io::duplex(MEMORY_STREAM_CAPACITY)` with the constant 8 KiB at src/link.rs:532; each elder writes one index byte then `PROBE` at 745 and 756)
- Seen by: prose; refutation: confirmed; history: no rationale found (32813f55 wrote the unqualified doc and the qualified inline comment in the same commit)
- Owner-gated: no

The public `check_concurrency` doc, the private `probe_concurrency` doc, and the module-doc bullet all state that the youngest stream drains past elders sitting backpressured mid-write. Under `memory()` every elder's 24-byte write completes into an 8 KiB buffer and nothing sits mid-write; only `memory_with_capacity(1)` produces the described state. The inline comment at 751-754 carries the correct qualifier ("at small windows"); the docs above it claim a stronger probe than the one run at the default capacity, in a module whose "What the suite cannot see" section is careful about exactly this.

Evidence:

       700	/// This validates the concurrency clause's quantitative bound: all
       701	/// [`STREAM_COUNT`] streams held open at once, with the last-opened
       702	/// stream's bytes flowing to completion past its still-open,
       703	/// backpressured elders: the progress-beside-siblings the session's

       751	        // Every stream then writes its payload concurrently. The receiver
       752	        // drains the last-opened stream first, so at small windows the
       753	        // elder writers sit backpressured, on their own streams only,
       754	        // while the youngest completes.

Resolution: Qualify the three docs the way the inline comment does ("past its still-open elders, which sit backpressured where the window is smaller than the probe"), or strengthen the probe so elders always backpressure (write `STALL_FILL`-sized payloads on the elders) and keep the docs; the latter changes the public suite's behavior and is an owner call. Acceptance: the docs describe the probe at both tested capacities, or the probe backpressures elders at every capacity.

### conformance-17: The session-check comment claims many streams per side; the constant it sizes says one or two
- Where: src/conformance/link.rs:1045-1045 (related: src/conformance/link.rs:139-145, src/conformance/link.rs:1070-1078)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites and the final assertion `opened >= 1` per side at 1070-1078)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the comment predates 32813f55's measurement of "two on side a, one on side b", which restated the constant doc "at that strength" and left the comment)
- Owner-gated: no

Two statements of one fact disagree, and the constant doc is the one the final assertion is calibrated to. The reader is left to reconcile what the code cannot settle for them.

Evidence:

      1045	    // Divergence wide and deep enough to exercise many streams per side.

       139	/// Stream count follows the reconciled tree's depth, not the payload
       140	/// count: hashed leaf paths keep a corpus this size one or two levels
       141	/// deep, opening one or two streams per direction. The check's final
       142	/// assertion pins exactly what the sizing buys (every direction opened at
       143	/// least one data stream in-session), so it cannot rot silently; the
       144	/// many-streams regime is [`check_concurrency`]'s job.

Resolution: `// Divergence wide enough to open data streams in each direction; the many-streams regime is check_concurrency's job (see SESSION_PAYLOADS).` Acceptance: the inline comment and the `SESSION_PAYLOADS` doc make the same claim.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| conformance-3 | src/conformance.rs:10-19 | The backend suite's visibility rationale is stated three times, once in public rustdoc, and omits why the items are `pub(crate)`. | One statement of record in backend.rs; drop the public paragraph. |
| conformance-22 | src/conformance/backend.rs:17-20 and the sites listed | "seam" for boundaries that are each a named trait method. | Name the method; "seam" is a crate-wide ruling. |
| conformance-23 | src/conformance/backend.rs:35-36 | The process-global ledger states its consequence (serialize tests) but not its cause (`Leaf::leaf` receives no backend handle). | One sentence naming the no-handle signatures. |
| conformance-26 | src/conformance/backend.rs:552-554 | `BOUND_SWEEP_CEILING`'s doc names a largest bound the grid exceeds by one. | "each power of two up to 1 MiB, with both neighbors". |
| conformance-32 | src/conformance/backend/tests.rs:62-72 | "honest", "lying", and `Dishonest` collide with the model's term of art for the trust premise. | Rename to accuracy terms (`accurate`, `Skewed`); owner-gated. |
| conformance-34 | src/conformance/backend/tests.rs:417-422 | The testdoc places detection "the moment" a node assembles; the report lands at run end and the leaf check records the same lie first. | State recording versus reporting. |
| conformance-4 | src/conformance/link.rs:40-46 and the sites listed | "genuinely" (12), "silently" (9), "loudly", "teeth", and loose "sound" without their mechanism. | Per-site pass; keep the "real sockets" contrasts. |
| conformance-8 | src/conformance/link.rs:186-190 | `check_control`'s first sentence claims independence, which is the next check's clause. | "The control halves deliver each direction's bytes, in order, to the peer." |
| conformance-10 | src/conformance/link.rs:632-634, 1060-1069 | The `contract:` prefix decorates a structural `unreachable!` and is missing from a sibling assertion. | Reword the message; prefix both convergence assertions alike. |
| conformance-19 | src/conformance/link/tests.rs:66-68 (nine partition sites) | Em-dashes in `//` comments. | Crate-wide pattern (owner-gated ruling plus a lint). |

## Tree core

### tree-core-1: Three module docs say the wire mirror delegates deletion honoring to `traverse::unknown`; the mirror has its own filter, pinned differentially
- Where: src/tree.rs:58-61 (related: src/tree/traverse/join.rs:8-11, src/tree/traverse/unknown.rs:9-11, src/tree/mirror/streaming/materialized/unknown.rs:4-5 and 43-45, src/tree/mirror/streaming/materialized/unknown/tests.rs:15 and 76, src/tree/mirror/streaming/tests.rs:154-156)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep for `traverse::unknown|unknown::Unknown|Unknown::unknown` outside `src/tree/traverse/` returns only rustdoc links and the differential test's import and call at `materialized/unknown/tests.rs:15,76`; read `materialized/unknown.rs:1-45`, which defines its own `known` predicate and dominance classifier)
- Seen by: correctness; refutation: confirmed; history: deliberate-but-expired (true while the V1 mirror called `traverse::unknown::Unknown`; 368da2a50 deleted V1 on 2026-09-01, and the retirement plan's lattice item 6 scheduled `unknown.rs`'s doc restatement, which did not land)
- Owner-gated: no

Three module docs state that `join` and the wire mirror share one deletion filter, and `tree.rs` draws the crate's testing strategy from that premise ("every convergence property can be tested in-memory and trusted on the wire"). The streaming mirror does not call `traverse::unknown`: `materialized/unknown.rs` is a separate, erased, backend-generic implementation, and the only consumer of `Unknown::unknown` outside `traverse` is the differential test that uses it as an oracle. The observational identity is established by `agrees_with_materialized_oracle` and `streaming_matches_join_oracle`, not by shared code, so a maintainer changing `traverse::unknown` would wrongly believe the wire follows. Prose speaks in the present tense and states what IS; the stated basis of the test strategy must be the actual one.

Evidence:

    src/tree.rs
    58    //! [`mirror`] reconciles two trees over a wire. `join` and `mirror` are
    59    //! observationally identical — both delegate deletion honoring to the same
    60    //! filter — so every convergence property can be tested in-memory and
    61    //! trusted on the wire.

    src/tree/traverse/join.rs
    8     //! merged union once. It is observationally identical to mirroring two local
    9     //! trees, producing the same merged [`Root`](crate::tree::Root), because it
    10    //! delegates all version filtering to the same [`Unknown`] traversal the
    11    //! mirror uses.

    src/tree/traverse/unknown.rs
    9     //! Both the in-memory [`join`](mod@super::join) and the wire
    10    //! [`mirror`](super::mirror) delegate their version filtering here, which
    11    //! is what makes them observationally identical.

    src/tree/mirror/streaming/materialized/unknown.rs
    4     //! This is the streaming counterpart of
    5     //! [`traverse::unknown`](crate::tree::traverse::unknown): it prunes a single

Resolution: Reword the three sites to state the actual relationship: `join` and the streaming mirror implement the same deletion-honoring predicate (a subtree causally at or before the counterparty's version drops out) in two walks, and their agreement is pinned differentially (`agrees_with_materialized_oracle` against `traverse::unknown`; `streaming_matches_join_oracle` against `Tree::join`). In `unknown.rs`, state the module's present role: the in-memory filter `join` calls and the oracle the materialized pruner is checked against. Route the same rewording to `streaming/tests.rs:154-156` (out of partition). Acceptance: no prose under `src/tree/` says the mirror delegates to or uses `traverse::unknown`; each site names the differential test that carries the identity.

### tree-core-11: The leaf-level contract prose does not match `Act for Z`: the "morally associative" version clause, the observer's "once per effectual action", and the `# Panics` reach
- Where: src/tree.rs:392-397 (related: src/tree/traverse/act.rs:17-22, 30-38, 139-190; src/tree/tests.rs:512-548)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (traced `Act for Z`, act.rs:139-192, against the three prose sites; the two committed pins at tests.rs:530 and 547 agree with the trace and contradict the paragraph at 392-397)
- Seen by: prose (12), correctness (37); refutation: confirmed (37 states the exact rule; the refutation's new item on the `# Panics` reach is folded in here); history: deliberate-but-expired (1f83e9c74 wrote the paragraph when forgets did not tick, per 114cc9998; fc1ea02d4 and 9ebd1b8d2 amended around it without re-deriving; the observer wording at act.rs:19-22 is from 262568f9e while the once-per-key observation with `greatest_version` is 052d1f95b, never reconciled)
- Owner-gated: no

`Z::act` joins every action's version at a key into `greatest_version` (line 148, before the skip), and fires `on_action` once per key group, with that join, iff the leaf existed before or exists after (186-190). Three prose sites describe something else. (a) `Tree::act` says the version "is incremented once per changed key, regardless of how many actions pertain to it": `[Insert, Forget(same path)]` in one batch leaves the ceiling untouched (no observation when the net effect is nil; pinned at tests.rs:547 `Version::new()`), while the same pair across two calls advances it by both ticks (tests.rs:530 `version_for(&party, 2)`), and `[Insert, Forget(absent), Insert]` leaves it three ticks along with two changed keys. The actual rule: the ceiling absorbs the join of each observed group, that is, the tick of the last observed action; ticks of trailing unobserved actions are reissued by the next batch. (b) `traverse::act` says `on_action` "fires once per *effectual* action ... with that action's version": it fires per key group, with the joined version, and it fires for a group whose every action was skipped as causally prior and for an identical re-insert (neither effectual; tree.rs:409-417 acknowledges the first as the flag's conservative case), while it does not fire for an effectual Insert+Forget pair on a fresh key. (c) The `# Panics` section says an insert landing on a live leaf "disagreeing with it on version or payload" panics, but the causal skip at 152-159 runs before the identity check at 169-176, so an insert whose version is strictly prior to the resident leaf's is dropped, never asserted; `act_destructor_unwind_leaves_tree_byte_identical` (tests.rs:1683-1720) relies on exactly that. The observer is the sole source of the changed flag and of ceiling movement, so its stated semantics are what `Batch::commit`'s wakeup rests on; and a maintainer reading the doc and the two pins today gets two answers.

Evidence:

    src/tree.rs
    392        /// This function is "morally associative": partitioning a sequence of
    393        /// actions across multiple `act` calls produces the same tree as a
    394        /// single `act` over their concatenation, except possibly for the tree's
    395        /// version when several actions address the same key. In that case the
    396        /// version is incremented once per changed key, regardless of how many
    397        /// actions pertain to it.

    src/tree/traverse/act.rs
    19    /// `on_action` fires once per *effectual* action — a leaf inserted, replaced,
    20    /// or removed — with that action's version. A forget of a leaf that never
    21    /// existed observes nothing, which is what lets the caller join versions only
    22    /// for actions that changed the tree.
    ...
    32    /// Panics if an insert lands on a live leaf disagreeing with it on
    33    /// version or payload: version reuse. No input reaches that state —
    ...
    148            greatest_version |= &version;
    ...
    152            if version
    153                < node
    154                    .as_ref()
    155                    .map(|n| n.ceiling())
    156                    .unwrap_or(&Version::default())
    157            {
    158                continue;
    159            }
    ...
    185        // Observe the action, provided that the net action wasn't nil
    186        match (existed_before, &node) {
    187            // The node stayed empty
    188            (false, None) => {}
    189            _ => on_action(&greatest_version),
    190        }

Resolution: Restate tree.rs:392-397 as the implemented rule: the ceiling advances to the version of the last action whose key was observed (a key whose leaf existed before or exists after the batch); a batch whose trailing actions touch only never-present keys leaves those ticks unabsorbed, so the next batch reissues them (harmless: nothing carried them). Drop the scare-quoted coinage or define it in the sentence. Restate act.rs:19-22 as: fires once per key the batch touches whose leaf existed before or exists after, with the join of that key's action versions. Qualify act.rs:32-33: the panic covers an insert not causally prior to the resident leaf; a strictly prior insert is skipped. Add a unit test for `[Insert, Forget(absent path), Insert]` pinning `latest()` at three ticks. Acceptance: the paragraph at 392-397 predicts both `insert_then_delete_is_empty` and `insert_and_delete_same_batch_is_empty` without an exception clause; the observer doc names the key-group rule; the `# Panics` section names the skip's precedence; the new test is committed.

### tree-core-16: `react`'s atomicity comment attributes its mid-walk destructor source to a "wire-apply path" that does not exist
- Where: src/tree.rs:507-513 (related: src/tree/tests.rs:669-672 and 1689-1691; src/tree/traverse/act.rs:36-37; src/peer/gossip.rs:869; src/batch.rs:143)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn wire-apply src/` returns tree.rs:511 and tests.rs:1690 only; `grep -rn '\.react('` shows `act` as `Tree::react`'s only production caller; the wire path installs merged trees through `inner.tree.join(merged)` at gossip.rs:869 and local batches through `inner.tree.act(party, actions)` at batch.rs:143; act.rs:36-37 states "no wire-derived leaf passes through this walk")
- Seen by: prose (11); refutation: confirmed (at 2d3a86d3f, the commit that introduced the phrase, `act` was already the sole caller, so the sentence was inaccurate when written); history: deliberate-but-expired (`react` was the foreign-apply entry in April, 47c9c9013; that role ended with `Tree::join`, 329d891bf, and `react` went private in 8e348feff; f3fef7bc1 the next day stated the opposite premise in the same walk)
- Owner-gated: no

The commit-section comment justifies the mid-walk `T`-destructor hazard by "the wire-apply path" where "those messages are freshly deserialized". No wire leaf passes through `react`. The mechanism itself is real on the `act` path (a `Batch` holds the only handles to its `Message`s, so a displaced insert or causally-skipped action dropped mid-walk is the last handle), so the conclusion stands while its stated premise is a ghost that contradicts act.rs:36-37 fourteen lines into the same walk. The same ghost appears in the destructor pin's doc ("exactly the wire-apply shape") and in `react_idempotent`'s rationale (re-delivery "in the face of retries or out-of-order transport"). A comment describing a code path the code does not have is a ghost reference, and here it misstates the premise a correctness argument rests on.

Evidence:

    507            // Panic atomicity: nothing of `self` mutates until the commit point
    508            // below, whatever the unwind's origin — a user type's destructor or
    509            // our own bug. Unwind sources survive inside this walk: the leaf
    510            // level drops causally-skipped action messages and batch-internal
    511            // displaced inserts mid-walk, and on the wire-apply path those
    512            // messages are freshly deserialized, so the drop is the last handle
    513            // and runs `T`'s destructor.

    src/tree/tests.rs
    1689    /// skips it and drops the action's message mid-walk — and that message is
    1690    /// the payload's last handle, exactly the wire-apply shape, where every
    1691    /// incoming message is freshly deserialized. The caught panic must be the

    src/tree/traverse/act.rs
    36    /// party linearity keeps regions disjoint), and no wire-derived leaf
    37    /// passes through this walk — so the panic marks a bug in this crate,

Resolution: Re-state the unwind-source premise in terms of the path that exists: `act` moves the batch's `Message`s into the walk (a `Batch` holds the only handles), so a displaced insert or a causally-skipped action dropped mid-walk is the last handle and runs `T`'s destructor. Delete the wire-apply clause at tree.rs:511-512 and tests.rs:1689-1691; reword tests.rs:669-672 so `react_idempotent` speaks of re-applying a versioned batch, not of transport retries. Leave act.rs:36-37 as is. Acceptance: `grep -rn 'wire-apply' src/` returns nothing; the react comment and both test docs name only the local batch path as the destructor source; act.rs's `# Panics` premise and the react comment agree.

### tree-core-23: `arb.rs` describes paths as content-addressed and as functions of `(version, payload)`; the tree is version-addressed
- Where: src/tree/arb.rs:235-236 (related: src/tree/arb.rs:301-302, 327-329; src/tree/typed/path.rs:35-38; src/tree.rs:11-12; out of partition: src/tree/mirror/streaming/stats.rs:45, src/tree/mirror/streaming/tests/fixtures.rs:322)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`Path::for_leaf` is `PathHash::of(version.as_bytes())`, path.rs:35-38; tree.rs:11-12 says "Message bytes enter no path and no digest"; `git show 4f18c347 -- src/tree/arb.rs` is a one-line hunk renaming the hash at line 235 while keeping the noun)
- Seen by: prose (13); refutation: confirmed; history: deliberate-but-expired (both sentences were accurate when written, b3b877d9b and 48d5253d9; 961f63c6 derived leaf identity from the version alone and edited `arb.rs` without them; 4f18c347 then edited line 235 and kept the wrong noun)
- Owner-gated: no

The search comment at 327 states "Paths are functions of (version, payload) and payloads are unit", the very function the search simulates, and it is wrong: the path is a function of the version alone. "Content-addressed generators" (235) and "Content addressing means the shape cannot be dictated" (301-302) carry the same expired design. AGENTS.md's hard rule: nothing in the codebase refers to a design the code no longer has.

Evidence:

    235    /// Content-addressed generators cannot produce this shape — SHA3-256 scatters
    236    /// their keys at the root fan, so a merge's divergent descent below the
    ...
    301    /// permanent at the tier that should have owned it. Content
    302    /// addressing means the shape cannot be dictated, so it is *searched*: insert
    ...
    327        // Paths are functions of (version, payload) and payloads are unit, so a
    328        // candidate pair is fully determined by where each side's version chain
    329        // *starts*: `Tree::act` ticks from the root ceiling, so seeding a built

    src/tree/typed/path.rs
    35        pub fn for_leaf(version: &Version) -> Self {
    36            Self {
    37                height: PhantomData,
    38                hash: PathHash::of(version.as_bytes()).into(),

Resolution: Line 327: "Paths are functions of the version alone, so a candidate pair is fully determined by where each side's version chain starts" (the "payloads are unit" clause then goes). Lines 235 and 301: "Version-addressed generators" / "Version addressing". Route the two out-of-partition sites to the streaming partition. Acceptance: `grep -n -i 'content-address\|content addressing\|(version, payload)' src/tree/arb.rs` returns nothing; every statement of the path function in `arb.rs` agrees with `Path::for_leaf`.

Synthesis note: The out-of-partition sites (streaming/stats.rs:45, streaming/tests/fixtures.rs:322) were not filed by the streaming partitions; they should ride with this entry's fix.

### tree-core-24: `ATTEMPTS`'s doc hand-maintains "the winning window is attempt 1581", which the SHA3 swap did not re-derive, and calls the `unreachable!` an assert
- Where: src/tree/arb.rs:318-325 (related: src/tree/arb.rs:413)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log -S'1581' -- src/tree/arb.rs` yields only d800957e8, 2026-08-06, whose message records "winning attempt 1581, budget 2048" and whose diff replaced the earlier "attempt 622, so 1024"; `git show 4f18c347 --stat -- src/tree/arb.rs` is one insertion and one deletion at line 235, so lines 318-325 were untouched while every `Path::for_leaf` output moved; line 413 is `unreachable!`. Whether 1581 still wins under SHA3-256 was not run)
- Seen by: structure (0), prose (14), perfapi (52); refutation: confirmed; history: deliberate-but-expired (the number was part of an explicit re-derivation discipline that d800957e8 followed for the previous leaf-hash change and 4f18c347 did not; "the assert below" has been wrong since b3b877d9b wrote it beside an `unreachable!`)
- Owner-gated: no

The doc asserts a specific search outcome and calls 2048 "exact headroom, not a guess". The figure was measured under BLAKE3; the swap to SHA3-256 re-derived every leaf path and changed only the hash's name in this file, so the sentence now describes a search the code no longer performs. Nothing checks which attempt wins (exhaustion is an `unreachable!`, not "the assert below"), so the claim is unverifiable in the tree and has already been hand-updated once. No hand-maintained counts: a number that matters lives in a mechanically enforced place that prose may cite by name.

Evidence:

    318        /// Attempt budget; the assert below turns exhaustion into a loud failure.
    319        ///
    320        /// The precompute below is proportional to this bound, so it directly
    321        /// prices the fixture. Hashing is deterministic and the winning window
    322        /// is attempt 1581, so 2048 is exact headroom, not a guess; if hashing
    323        /// or the leaf encoding ever changes, the search either finds another
    324        /// window within the budget or fails loudly here.
    325        const ATTEMPTS: usize = 2048;
    ...
    413        unreachable!("the deterministic geometry search must terminate");

Resolution: Delete "and the winning window is attempt 1581, so 2048 is exact headroom, not a guess" and keep the structural statement (a deterministic search under a budget whose exhaustion panics). If the headroom matters, make it mechanical: have the fixture return the winning attempt (or log it with the crate's `MEASURED` idiom) and assert a margin in a test the doc cites by name, which restates the discipline d800957e8 followed instead of relying on memory. Change "the assert below" to name the `unreachable!`, or make the guard an `assert!` whose message prices the budget. Acceptance: no literal attempt index in `arb.rs` prose, or a committed test asserts the winning attempt and the doc cites it; the guard's description matches its spelling.

Synthesis note: Same lines as suite-economics-3.

### suite-economics-3: "the winning window is attempt 1581" is a hand-maintained number the SHA3 swap left behind
- Where: src/tree/arb.rs:321-325 (related: src/tree/typed/hash.rs (`PathHash::of`), src/tree/typed/path.rs:35-40 (`Path::for_leaf`))
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read plus git history: d800957e8 on 2026-08-06 changed the number from "attempt 622, so 1024" to "attempt 1581, so 2048" when the version encoding changed; 4f18c347 on 2026-09-01 replaced `blake3::hash` with `Sha3_256::digest` in `PathHash::of`, which `Path::for_leaf` calls, and touched arb.rs only at a doc comment on line 235; arb.rs has no commit since). The current winning attempt itself is unobservable without a probe test, so the exact new value is not verified; that the scanned sequence changed is.
- Verification: new finding, surfaced while disputing suite-economics-2; history: the number has been maintained by hand once already (the 2026-08-06 edit), which is the mechanism by which it now drifts.
- Owner-gated: no

The search predicate reads the first byte of `PathHash::of(version bytes)`
for each candidate leaf; the hash function changed, so the sequence of
first bytes and the first satisfying window changed with it. The suite
passes, so some window under 2048 satisfies the geometry, but the comment's
specific number and the sentence built on it ("2048 is exact headroom, not a
guess") are now unverifiable prose. Principle 5 (no hand-maintained counts:
a number that matters lives where something checks it) and Principle 8 (a
number nothing re-measures is a hypothesis).

Evidence:

    321	    /// prices the fixture. Hashing is deterministic and the winning window
    322	    /// is attempt 1581, so 2048 is exact headroom, not a guess; if hashing
    323	    /// or the leaf encoding ever changes, the search either finds another
    324	    /// window within the budget or fails loudly here.
    325	    const ATTEMPTS: usize = 2048;

    -        PathHash(*blake3::hash(bytes).as_bytes())
    +        PathHash(Sha3_256::digest(bytes).into())

    35	    pub fn for_leaf(version: &Version) -> Self {
    36	        Self {
    37	            height: PhantomData,
    38	            hash: PathHash::of(version.as_bytes()).into(),
    39	        }
    40	    }

Resolution: the same change as suite-economics-2's hint constant, measured
once after the SHA3 swap and checked by code (the search asserts the hint
satisfies the predicate, or a dedicated test fails with "update
HINT_ATTEMPT"). If the hint is not wanted, delete the number and state the
structure instead: the winning window lies inside the budget and exhaustion
fails loudly. Acceptance: no attempt number remains in prose that code does
not check.

Synthesis note: Same lines as tree-core-24, which is the fuller record; this entry adds the hint-constant resolution shared with suite-economics-2 (a verification-class entry).

### tree-core-10: `act`'s rustdoc states a measured "2-3x" that no committed bench produces
- Where: src/tree.rs:387-390 (related: benches/in_memory.rs:23-39 and 91-92)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '2-3x' src/ benches/ README.md` finds tree.rs:389 only; `benches/in_memory.rs` has a `batch_insert` group and no one-at-a-time counterpart; no bench file calls `.act(`)
- Seen by: prose (22), correctness (42), perfapi (51); refutation: confirmed; history: deliberate-but-expired (8231541a0 landed the figure with a bench comparing `act/insert_batch_into_empty` against `act/insert_one_by_one`; 94da12b59 deleted that bench and the figure survived two rewordings)
- Owner-gated: no

The batching speedup is given a number that nothing in the tree can re-derive; it once had a bench and no longer does, so it rots as the fan, hashing, or allocator change (and tree-core-27's sort deletion would move it). A number you were handed is a hypothesis; an approximation survives in prose only with its measurement named.

Evidence:

    387        /// A batch is applied to the tree in a single traversal, which is more
    388        /// efficient than applying its actions one at a time: in theory an
    389        /// O(log n) speedup over one-by-one insertion, in practice about 2-3x
    390        /// since the log base is 256.

Resolution: Either drop the figure and keep the qualitative claim (one traversal instead of one per action, shared spine work amortized), or add a `single_insert` column beside `batch_insert` in `benches/in_memory.rs` and cite it by name. Acceptance: the doc either states no figure or names a committed bench that produces it.

### tree-core-13: `act`'s body comments are fragmented and partly redundant, and both `Action::Insert` docs misstate where the version comes from
- Where: src/tree.rs:424-446 (related: src/tree.rs:161, 380; src/tree/traverse/act.rs:11)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: prose (23); refutation: confirmed; history: no-rationale-found (the blocks accreted across 114cc9998, fc1ea02d4, and 262568f9e; the `Insert` clause "tagged at the current version" was accurate when a batch shared one version and expired at 114cc9998)
- Owner-gated: no

Four comment blocks say overlapping things: 424-430 (tick per action, forgets above inserts, the deletion-honoring rationale) runs without a blank line into 431-434 (running version, lazy reactions); 437-439 restates uniqueness with a different reason ("wrongly early-aborts when versions compare equal" is the mirror's equal-ceiling short circuit, not per-action uniqueness); 443-446 carries rustdoc link syntax inside a `//` comment where it cannot resolve; and the rustdoc at 380 defers to "the body comment" for a reason 376-379 already gives. Separately, tree.rs:161 says an insert is "tagged at the current version by your own party" (it is tagged at the post-tick version, and the second person is odd in a maintainer doc), and act.rs:11 says "tagged by a version at a party" though the version rides in the action tuple, not the `Message`. Comments state what the code cannot show, once, at the branch that needs it.

Evidence:

    161        /// Insert some value, tagged at the current version by your own party.
    ...
    431            // The running version, advanced in place per action; each action
    432            // clones the post-tick value as the committed version that keys
    433            // its leaf. The reactions flow into `react` lazily; the whole
    434            // chain materializes only once, at the traversal's radix sort.
    ...
    437                // Advance the version. It must be unique for every action
    438                // applied to the tree; otherwise the mirror protocol
    439                // wrongly early-aborts when versions compare equal.
    ...
    443                // Convert unversioned, unlocalized actions into reactions
    444                // independent of our party and current version. The path is
    445                // derived from the post-tick version, which is unique per
    446                // insert (see [`typed::Path::for_leaf`]).

Resolution: Merge 424-446 into one comment: one paragraph on why every action ticks (a fresh path per insert; forgets strictly above any prior insert so the ceiling moves and equal-ceiling early completion cannot hide a redaction), one line on the lazy chain materializing at the walk's sort; drop the link syntax. At 380, delete "see the body comment". Reword 161 to "Insert a message; `act` stamps it with the version the batch ticks to" and act.rs:11 to "Insert the message; the version rides alongside in the action tuple". Acceptance: one comment block precedes `self.react(`; no `[`...`]` link syntax inside `//` comments in `tree.rs`; both `Insert` variant docs describe where the version comes from.

### tree-core-21: Generator docs in `arb.rs` narrate the streaming-deadlock incident instead of stating the geometry
- Where: src/tree/arb.rs:174-175 (related: src/tree/arb.rs:185, 293, 300-302; the incident record at .agent-notes/2026-07-17-streaming-wire-deadlock/)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the sites; the geometry itself is stated present-tense at 295-298 and 367-371)
- Seen by: prose (19); refutation: confirmed; history: no-rationale-found (b3b877d9b, the deadlock fix, wrote the narrative; the link-transport review's R36 later flagged line 185 for a design-doc section citation and 023546e4a produced the current "closes the proxy tier's generator gap" wording, so that ruling addressed citation form, not incident framing, and this finding does not reopen it)
- Owner-gated: no

`arb_wide_divergent_pair` and `early_first_child_dispute_pair` are documented as incident artifacts ("the streaming wire deadlock's trigger geometry"; "closes the proxy tier's generator gap"; "the streaming wire deadlock's counterexample skeleton, made permanent at the tier that should have owned it"). What a reader of a generator needs is the shape it produces and the property it stresses, which lines 295-298 and 367-371 already state well. History, blame, and gap-closing narratives belong in git and the decision record, not at the declaration site.

Evidence:

    174    /// [`arb_divergent_pair`] at a budget wide enough to reach the streaming
    175    /// wire deadlock's trigger geometry.
    ...
    185    /// This strategy closes the proxy tier's generator gap on *budget* only,
    ...
    300    /// This is the streaming wire deadlock's counterexample skeleton, made
    301    /// permanent at the tier that should have owned it. Content
    302    /// addressing means the shape cannot be dictated, so it is *searched*: insert

Resolution: At 174-175 and 185-192, describe the budget and what the wide pairs reach (multi-level disputes mixed with provisions in the opening reply) without "deadlock" or "gap". At 300-302: "The shape stresses whole-subtree provisions queued behind a dispute on one reply stream. Version addressing means it cannot be dictated, so it is searched: ...". Acceptance: `arb.rs` contains no incident nouns ("deadlock", "gap", "should have"); each generator doc states shape and stressed property in the present tense.

### tree-core-18: Two `tests.rs` helper docs state preconditions that are false: `distinct_bytes` buys no path distinctness, and `idx` handles no generated strings
- Where: src/tree/tests.rs:29-33 (related: src/tree/tests.rs:56-65, 265-271, 714-718; src/tree/typed/path.rs:35-38; src/tree.rs:11-12)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`Path::for_leaf` hashes `version.as_bytes()` alone, path.rs:35-38; every `party_of(` argument in tests.rs is a literal, a `&party` bound to a literal, `[b'a' + (i % 5) as u8]` at 1037, or `label` from a literal array at 1460; `idx` is first byte lowercased minus `b'a'` mod 16)
- Seen by: prose (20, 21); refutation: confirmed; history: deliberate-but-expired for `distinct_bytes` (at c6a448972 `leaf_path(party, scalar, &value)` hashed the payload; 961f63c6 removed payload bytes from paths without this doc); no-rationale-found for `idx` (the "proptest-generated strings" clause described no caller at birth, 1af0ab9aa)
- Owner-gated: no

`distinct_bytes`'s doc says deduplication makes "every element map to a unique leaf path" and that properties need "no two inserts collide by path". Paths derive from versions, never bytes, so distinct bytes buy no placement; two inserts at one version collide whatever their bytes (and trip the version-reuse assert when the bytes differ, tests.rs:1877). Distinctness is still needed, for a different reason: the payload-keyed maps `index_of` (265-271) and `meta_by_value` (714-718) must be injective. `idx`'s doc says distinct labels "or proptest-generated strings" map to distinct indices; no caller passes a generated string, and the first-byte-mod-16 mapping sends "ab"/"ac" or "a"/"q" to one index, so the claim would be false if exercised. A helper's stated precondition must be one the code enforces or the callers respect.

Evidence:

    29    /// Generate a vector of distinct `Bytes`, deduplicated so every element maps
    30    /// to a unique leaf path when inserted under the same party and version.
    31    ///
    32    /// Many of the hash-invariance properties below are only meaningful when no two
    33    /// inserts collide by path; collision semantics are exercised separately.
    ...
    56    /// Map a human-readable party label to a small disjoint-party index.
    57    ///
    58    /// The distinct labels the tests use ("A"/"B"/"C"/"P", or proptest-generated
    59    /// strings) map to distinct indices, so [`party_of`] yields mutually
    60    /// disjoint parties.
    61    fn idx(label: impl AsRef<[u8]>) -> usize {
    62        label.as_ref().first().map_or(0, |b| {
    63            (b.to_ascii_lowercase().wrapping_sub(b'a') as usize) % 16
    64        })
    65    }

Resolution: `distinct_bytes`: "Generate a vector of distinct `Bytes`. Paths derive from versions, so distinctness buys nothing for placement; it keeps the payload-keyed maps the shuffle properties build (`index_of`, `meta_by_value`) injective." `idx`: drop "or proptest-generated strings" and state the rule: "Labels are single letters; the index is the letter's alphabet position mod 16, so the letters one test mixes must be distinct mod 16." Acceptance: both docs state the reason the helper is actually needed and no claim about paths or generated strings.

### tree-core-26: Ghosts of the retired V1 mirror in `traverse.rs`, `join.rs`, and `tree.rs`: `Levels`, the zipper, a "free function" claim `unknown` does not meet, and a trio that names `mirror`
- Where: src/tree/traverse.rs:7-19 (related: src/tree/traverse.rs:1-5; src/tree.rs:53-58; src/tree/traverse/join.rs:4-8 and 121; src/tree/traverse/unknown.rs:20-27; src/tree/traverse/act.rs:5; src/tree/traverse/join.rs:39; src/tree/traverse/unknown.rs:17; src/tree/mirror/streaming/materialized/unknown/tests.rs:2 and 15)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '\bLevels\b' src/` returns traverse.rs:10 only; `grep -rni zipper src/` returns join.rs:6 only; grep for `traverse::act::` or `act::Act` outside `traverse/` returns nothing, while `unknown::Unknown` is imported and linked by `materialized/unknown/tests.rs:2,15`; `unknown.rs` exports only the `Unknown` trait and join.rs:121 calls `Unknown::unknown` directly; `use super::*;` at traverse.rs:7 is consumed by nothing in that file)
- Seen by: structure (6), prose (17, 18); refutation: confirmed (with provenance: `Levels` lived in the removed `traverse/mirror/local.rs`, "Each side keeps a [`Levels`] zipper"); history: deliberate-but-expired (ba8da96c3 widened `act` and `unknown` to `pub(crate)` for links from `typed/levels.rs`; 368da2a50 deleted `levels.rs` and the last `act::Act` linker; "run a zipper" and the act/join/mirror trio described the V1 mirror, which lived under `traverse/` until a4df5b2f6)
- Owner-gated: no

The visibility comment justifies `pub(crate)` on `act` and `unknown` by "rustdoc elsewhere (e.g. the `Levels` docs)". No `Levels` item exists, and nothing outside `traverse` links into `act` (every user goes through the `pub use act::{Action, act}` facade); `unknown` does have a live linker (the materialized pruner's differential test), so that half is real but misattributed. The module doc says "Each traversal is exposed as a free function", but `unknown` exposes only the trait. `tree.rs:55-58` lists the `traverse` trio as act, join, mirror, while `traverse` holds act, join, unknown and `mirror` is a sibling module whose streaming walk is erased, not height-inductive. `join.rs:6` says the mirror must "run a zipper", the V1 mechanism. The glob `use super::*;` exists only so the children can write `use super::typed::*;` and link `super::mirror`, which suggests that `typed` and `mirror` are children of `traverse`. No ghost references; a visibility rationale that names a nonexistent dependent is circular justification for the visibility it explains.

Evidence:

    src/tree/traverse.rs
    3     //! Each traversal is exposed as a free function so callers need not import a
    4     //! trait, though under the hood all are implemented by polymorphic recursion
    5     //! through traits.
    6
    7     use super::*;
    8
    9     // `act` and `unknown` are `pub(crate)` so rustdoc elsewhere (e.g. the
    10    // `Levels` docs) can link to the traversal traits inside them: a private
    11    // `mod` is unnameable from outside `traverse`, so the links would not
    12    // resolve. The free-function facade below remains the API.
    13    pub(crate) mod act;
    14    pub use act::{Action, act};
    15
    16    pub(crate) mod unknown;

    src/tree.rs
    55    //! All mutation and reconciliation is three inductive traversals over the
    56    //! same structure ([`traverse`]): [`act`](Tree::act) applies a local batch
    57    //! in one pass; [`join`](Tree::join) merges two in-memory trees;
    58    //! [`mirror`] reconciles two trees over a wire. `join` and `mirror` are

    src/tree/traverse/join.rs
    5     //! protocol: where the mirror reconciles two replicas by exchanging messages
    6     //! (and so must serialize, run a zipper, and build the union on both sides),

Resolution: `mod act;` (private) with the facade unchanged; re-state the comment to the actual dependent ("`unknown` is `pub(crate)` because the materialized pruner's tests import the `Unknown` trait as their oracle and link it from rustdoc"). Rewrite traverse.rs:3-5: "`act` and `join` are free functions over their per-height traits; `Unknown` is used as a trait by `join` and by the materialized pruner's tests." Drop `use super::*;` and have the children import `crate::tree::typed::*` (as `arb.rs` and `unknown/tests.rs` already do) and link `crate::tree::mirror`. At tree.rs:55-58, list the `traverse` trio as act/join/unknown and introduce `mirror` as the wire counterpart of `join`. At join.rs:6: "(over the wire, pairing queries with replies and building the union on both sides)". Acceptance: `cargo doc` (the gate's `doclint`) resolves every intra-doc link with `mod act` private; `grep -rn '\bLevels\b' src/` and `grep -rni zipper src/` return nothing; `traverse.rs` has no glob import; tree.rs's trio matches `traverse.rs`'s contents.

Synthesis note: Overlaps module-graph-16 on the traverse.rs comment.

### module-graph-16: Both recorded visibility rationales have drifted from the code
- Where: src/tree/traverse.rs:9-13 (related: src/tree/typed.rs:19-22; src/tree/mirror/streaming/backend/local.rs:61, :116; src/testing.rs:54, :60; src/tree/typed/node.rs:401-406; src/tree.rs:475)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '\bLevels\b' src` returns only the comment; `grep -rn 'traverse::act' src` outside `traverse` returns only links to the re-exported *function*, `[`traverse::act`](fn@traverse::act)` at `tree.rs:475`; `grep -rn 'untyped::Range' src` returns hits only inside `typed` (`node.rs`), where a private module is nameable anyway; `grep -rn 'typed::untyped::' src` outside `typed` returns code uses in `local.rs:61,116` and `testing.rs:54,60`)
- Verification: new finding, not in the sweep; it reframes the sweep's positive "Visibility decisions are recorded at the declaration where they are unusual"; history: `Levels` was deleted by `368da2a5` ("Node::levels, the typed::levels zipper stack")
- Owner-gated: no

`traverse.rs` keeps `act` at `pub(crate)` "so rustdoc elsewhere (e.g. the `Levels` docs) can link to the traversal traits inside them"; `Levels` no longer exists, and the only outside links to `act` are to the function the facade re-exports at `:14`, so the module can be private. `typed.rs` keeps `untyped` at `pub(crate)` "so rustdoc elsewhere can link to `typed::untyped::Range`"; every `untyped::Range` link is inside `typed`, while the visibility is in fact held by code outside `typed` (`impl ErasedNode for typed::untyped::Node` in `local.rs`, `untyped::census` in `testing.rs`), which the comment does not say. Both comments state a reason that is not the reason, the failure the crate's own no-ghost-reference rule exists to prevent.

Evidence:

    src/tree/traverse.rs:
         9	// `act` and `unknown` are `pub(crate)` so rustdoc elsewhere (e.g. the
        10	// `Levels` docs) can link to the traversal traits inside them: a private
        11	// `mod` is unnameable from outside `traverse`, so the links would not
        12	// resolve. The free-function facade below remains the API.
        13	pub(crate) mod act;
        14	pub use act::{Action, act};
    ...
        16	pub(crate) mod unknown;

    src/tree/typed.rs:
        19	// `pub(crate)` so rustdoc elsewhere can link to `typed::untyped::Range`: a
        20	// private `mod` is unnameable from outside `typed`, so the links would not
        21	// resolve. The items below are still re-exported as the canonical paths.
        22	pub(crate) mod untyped;

    src/tree/mirror/streaming/backend/local.rs (what actually holds `untyped` open):
        61	impl ErasedNode for typed::untyped::Node {
    ...
       116	    type Erased = typed::untyped::Node;

Resolution: In `traverse.rs`, make `act` private (`mod act;`) and re-word the comment to cover `unknown` alone, naming the live link sites (`tree.rs:49`, `typed/untyped/iter.rs:172`, `materialized/unknown.rs:5`). In `typed.rs`, state the real reason: the erased node type and the census are used by the streaming backend and the test facade. Acceptance: `just docs-internal` stays clean (it documents private items with `-D warnings`, so a link the narrowing breaks fails there), and neither comment names an item that does not exist.

Synthesis note: Same traverse.rs comment as tree-core-26, which carries the fuller resolution (private `mod act`, the glob import, the trio at tree.rs:55-58); this entry adds the typed.rs half.

### tree-core-32: Collision detection is attributed to "`react`'s occupied-path arms" and called "ingestion"; the check lives in `Act for Z`, and "ingestion" already means the wire
- Where: src/tree/traverse/join.rs:224-229 (related: src/tree/traverse/act.rs:161-176; src/tree/tests.rs:1301, 1800-1804, 1830-1832; out of partition: src/tree/typed/hash.rs:102-103)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn occupied src/tree/` shows no such arms in `Tree::react`, tree.rs:477-541; the identity assert is `Act for Z`, act.rs:169-176; `hash.rs:103` even links `[react](crate::tree::Tree::react)`; tests.rs:1301 uses "ingestion" for session ingestion)
- Seen by: prose (16); refutation: confirmed; history: no-rationale-found (f3fef7bc1, the commit that dissolved the collision machinery, deleted react's occupied-path doc clause in the same diff that wrote "`react`'s occupied-path arms" into join.rs, so the pointer was imprecise from birth, and its message uses "ingestion" for the apply walk, colliding with the session-ingestion sense from 0116a0817)
- Owner-gated: no

A pointer to the wrong site costs the maintainer a search, and a term with two meanings (wire ingestion versus local apply) costs a misreading. The same pointer recurs in two test docs and in `hash.rs`.

Evidence:

    224                // Two leaves at one position share the path, and a leaf digest
    225                // is a pure function of its path (`Hash::leaf`), so the pair
    226                // hashes equal and the level above carries it over verbatim
    227                // without recursing. Collision detection is ingestion's job
    228                // (`react`'s occupied-path arms), where both leaves are in
    229                // hand; the merge walk trusts path derivation.

Resolution: "Collision detection is the apply walk's job (`Act for Z`'s insert-on-live-leaf assert), where both leaves are in hand"; the same wording at tests.rs:1800-1804 and 1830-1832, and at `hash.rs:102-103` when that file is touched. Acceptance: `grep -rn "occupied-path\|ingestion's job" src/tree` returns nothing; each pointer names `Act for Z` or the act.rs assert.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| tree-core-9 | src/tree.rs:370-371 (nine sites) | `Root::ceiling` carries three names: "version vector", "causal ceiling", "version". | Use "ceiling" at the nine sites. |
| tree-core-12 | src/tree.rs:412-413 and the sites listed | "honest" for trees, ticks, an iterator, a simulation, and a lint; intensifier "real"/"genuinely"; "seam", "cashed out", "priced". | Reword per site. |
| tree-core-15 | src/tree.rs:487 (21 partition sites) | Em-dashes in `//` comments. | Crate-wide pattern. |
| tree-core-17 | src/tree.rs:660-661; src/tree/tests.rs:1641-1643 | "proves the defense total" attributes to a fuse what the commit-point structure provides. | "demonstrates the defense at an arbitrary fire point; totality is the commit-point structure". |
| tree-core-20 | src/tree/tests.rs:1385-1397 | `span_door_traffic`'s first sentence leans on `before`-internal metaphors (door, rung, pair hull, fringe) with no definition or link. | Open with the mechanism in plain terms; link each borrowed term. |
| tree-core-35 | src/tree/traverse/unknown/tests.rs:1-3, 24-25, 127-129; src/tree/tests.rs:1342 | The live cost oracle is framed as "replaced"; the meter doc states a ratio the assert does not pin; one dated "now". | Present-tense restatements; make the stated pin match the `assert!`. |

## Tree typed

### tree-typed-3: `Hash`'s copy of the 24-byte width argument has drifted from the doc of record and prices an actor the same docstring says contributes nothing
- Where: src/tree/typed/hash.rs:39-43 (related: src/tree/typed/hash.rs:9-11, src/tree/typed/hash.rs:26-47, src/tree/typed/hash.rs:100-102, src/reconciliation.rs:164-172)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read reconciliation.rs:138-177 clause by clause against hash.rs:26-47; `git log -S'Why 24 bytes here'` names 2d1e6ea51 alone, whose body says "reconciliation.rs and hash.rs carry the re-derived argument"; `git show 9c73d7b46 -- src/tree/typed/hash.rs` touched only hunks at lines 7-11 and 92-110, leaving the Grinding bullet as written)
- Seen by: structure, prose; refutation: confirmed; history: two homes were owner intent at 2d1e6ea51 (deliberate-and-holds for the shape), while the content expired at 9c73d7b46, which re-derived reconciliation.rs only
- Owner-gated: yes for the dedupe (the two-home decision is recorded in 2d1e6ea51's message); no for the minimum fix (re-sync the bullet)

The `# Why 24 bytes here, and 32 for content` section restates the argument that `MERKLE_HASH_LEN`'s doc (lines 9-11) and the section itself (lines 31-32) already delegate to `crate::reconciliation`. Its Grinding bullet prices a content author's "colliding content pair" at 2⁹⁶, but the same docstring says at lines 100-102 that a content author contributes no bit to any compared quantity, and reconciliation.rs:166-169 says the content-grinding route is "structurally gone, not merely priced" and that the 2⁹⁶ floor prices influence over which versions are created. One argument in two homes has drifted, and the crate-private copy now misidentifies the actor it prices (Principle 5, one statement of record; a pricing argument that names the wrong actor is a prose correctness defect).

Evidence:

        39	/// - **Grinding.** For an author of message *content* who is not a peer —
        40	///   the one adjacent actor the trust model admits — the offline birthday
        41	///   floor for assembling any colliding content pair is 2⁹⁶ hash
        42	///   evaluations, which closes that vector unconditionally, with no
        43	///   premise about what an attempt would cost the attacker.

       100	    /// leaf commits no message bytes: a content author contributes no bit
       101	    /// to any compared quantity — digests are content-blind by design, a
       102	    /// modeled trade.

    src/reconciliation.rs:
       166	//! children, and message bytes appear nowhere. An author of message
       167	//! content therefore contributes zero bits to any compared quantity — the
       168	//! offline content-grinding route to a collision is structurally gone, not
       169	//! merely priced. What could still contribute bits is influence over which
       170	//! versions get created (an actor steering gossip schedules steers the
       171	//! version set); against any such actor, the 24-byte width keeps the
       172	//! offline birthday floor at 2⁹⁶ evaluations, an unconditional bound that

Resolution: preferred: cut lines 26-47 down to what is local to the type (a Merkle hash is an equality probe between subtrees at one prefix; a false-equal's cost and the width that prices it live in `crate::reconciliation#twenty-four-byte-digests`), keeping the SP 800-107 sentence at 19-24 and the link. Minimum: rewrite the Grinding bullet to match reconciliation.rs: content contributes zero bits by construction, and the 2⁹⁶ floor is the unconditional bound against an actor steering which versions are created. Acceptance: the crate states the 24-byte pricing derivation once, and `Hash`'s doc asserts nothing about content authors that lines 100-102 contradict.

### tree-typed-5: `LEAF_TAG`'s doc says the leaf preimage commits the version's encoding; `Hash::leaf` commits the suffix alone
- Where: src/tree/typed/hash.rs:60-64 (related: src/tree/typed/hash.rs:88-89, src/tree/typed/hash.rs:110-118, src/tree/typed/hash/tests.rs:30-32)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show 961f63c6:src/tree/typed/hash.rs` line 116 reads `pub fn leaf(suffix: &[u8], version: &crate::Version)`; f3fef7bc is "tree: leaf digests commit the suffix alone"; `git blame -L 60,64` attributes all five lines to 961f63c6, unchanged since; `Hash::leaf`'s body read at 110-118)
- Seen by: prose, correctness; refutation: confirmed; history: deliberate-but-expired (accurate when written; the owner-ruled f3fef7bc updated `Hash::leaf`'s doc and the layout test but not this tag's)
- Owner-gated: no

The tag's docstring lists two committed fields, suffix and version encoding. The code hashes `LEAF_TAG ‖ suffix_len ‖ suffix` and nothing else, `Hash::leaf`'s own doc says so at line 89, and `leaf_preimage_layout` states the opposite of this doc ("never any version or message bytes"). Two docs in one module disagree about what a wire-visible digest commits (statement faithfulness; the AGENTS.md hard rule on prose describing removed behavior).

Evidence:

        60	/// Leaves are version-addressed (the path is the full-width hash of the
        61	/// leaf's version; see [`Path::for_leaf`](super::Path::for_leaf)), so a
        62	/// leaf's preimage commits its compressed suffix — path bytes — and its
        63	/// version's canonical encoding, never its message bytes: every compared
        64	/// digest in the tree is a pure function of the version set.

        88	    /// The hash of a leaf observed from the top of its compressed `suffix`:
        89	    /// `sha3_256(LEAF_TAG ‖ suffix_len ‖ suffix)`.

    src/tree/typed/hash/tests.rs:
        30	/// A leaf commits to exactly `LEAF_TAG ‖ suffix_len ‖ suffix` — its
        31	/// compressed suffix, length-tagged, and never any version or message
        32	/// bytes.

Resolution: restate: the preimage commits the compressed suffix alone; because a leaf's path is the full-width hash of its version, the suffix (with the prefix above it) commits the version transitively, and no version or message bytes enter. Acceptance: `LEAF_TAG`'s doc, `Hash::leaf`'s doc, and `leaf_preimage_layout`'s doc name the same field list.

Synthesis note: Same lines as inventory-9, which sharpens the claim: a leaf preimage alone binds neither the full path nor the version; the root digest does, through the branch preimages.

### inventory-9: `LEAF_TAG`'s doc lists the version's encoding as a preimage component
- Where: src/tree/typed/hash.rs:58-65 (related: src/tree/typed/hash.rs:88-99, src/tree/typed/hash.rs:110-118)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (compared lines 60-64 against the body at 110-118 and the formula at 88-89; the `2026-07-18-node-hash-preimage` note states the same formula)
- Verification: confirmed and sharpened: a leaf's compressed suffix is only the path bytes below its branch point, so a leaf preimage alone binds neither the full path nor the version; the root digest does, through the branch preimages; history: deliberate-and-holds for the formula (the note), no-rationale-found for the sentence
- Owner-gated: no

The `LEAF_TAG` comment says a leaf's preimage commits "its compressed suffix — path bytes — and its version's canonical encoding". `Hash::leaf` hashes `LEAF_TAG ‖ suffix_len ‖ suffix` and nothing else; its own doc at 88-89 says exactly that, and 96-99 explain that the version is committed transitively because the path is the version's hash. A reader auditing the digest algebra against the `LEAF_TAG` sentence would look for version bytes that are not there.

Evidence:

    60	/// Leaves are version-addressed (the path is the full-width hash of the
    61	/// leaf's version; see [`Path::for_leaf`](super::Path::for_leaf)), so a
    62	/// leaf's preimage commits its compressed suffix — path bytes — and its
    63	/// version's canonical encoding, never its message bytes: every compared
    64	/// digest in the tree is a pure function of the version set.
    65	const LEAF_TAG: u8 = 0;

    88	    /// The hash of a leaf observed from the top of its compressed `suffix`:
    89	    /// `sha3_256(LEAF_TAG ‖ suffix_len ‖ suffix)`.

Resolution: Re-state: the leaf preimage carries the compressed suffix alone; the version enters only through the path being its full-width hash, and message bytes enter nowhere. Or cut the sentence and point at `Hash::leaf`, whose doc is exact. Acceptance: the `LEAF_TAG` comment matches the bytes `Hash::leaf` hashes.

Synthesis note: Same lines as tree-typed-5.

### tree-typed-13: The `PhantomData<fn() -> H>` argument is written out twice, and the typed wrappers restate the untyped docs they delegate to
- Where: src/tree/typed/node.rs:116-123 (related: src/tree/typed/height.rs:6-12, src/tree/typed/path.rs:9-13, src/tree/typed/prefix.rs:13-16, src/tree/typed/node.rs:184-195, 202-212, 225-234, 244-258, src/tree/typed/untyped.rs:481-515)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); the history verified by `git log -L` (all three phantom paragraphs written at edea11aca; 8f87ddd01 re-justified the node.rs copy "on its own terms")
- Seen by: prose; refutation: confirmed; history: no-rationale-found, noting the node.rs copy is the owner-touched one
- Owner-gated: no

The auto-trait argument appears in full on `S` (height.rs:6-12) and again on `Node` (node.rs:116-123), while `Path` and `Prefix` point at `Node` rather than at `S`, where the chain originates. Likewise the one-line delegating wrappers `dominance`, `version_bytes`, `is_leaf`, and `hash` on `Node<H>` re-explain the untyped semantics instead of stating what the height tag adds and linking the method of record, and untyped.rs:481-515 carries near-verbatim twin paragraphs for `ceiling` and `floor`. Duplicated rationale drifts (finding 3 is one realized instance); Principle 5: one statement of record per argument.

Evidence:

       116	/// The height marker is held as `PhantomData<fn() -> H>` rather than
       117	/// `PhantomData<H>`. Function pointers are unconditionally `Send + Sync`,
       118	/// so any auto-trait obligation on `Node` discharges without descending
       119	/// the `S<S<S<...S<Z>...>>>` peano-style height chain: a bare
       120	/// `PhantomData<H>` would send the trait solver walking 32 levels of
       121	/// `S<…>: Sync` on every `Send`/`Sync` check, even though the type
       122	/// variable `H` is purely phantom and never constructs anything that
       123	/// could fail to be `Send`/`Sync`.

    src/tree/typed/height.rs:
         6	/// The inner marker is `PhantomData<fn() -> T>` rather than `T` (or
         7	/// `PhantomData<T>`) so the auto-trait check on `S<T>` does not descend
         8	/// into `T`. Without this, proving `S<S<…S<Z>…>>: Sync` recurses 32
         9	/// levels deep every time a downstream crate asks an auto-trait question
        10	/// about a type that names `Root`. Function pointers are unconditionally
        11	/// `Send + Sync` regardless of their return type, so this marker
        12	/// short-circuits the recursion without unsafe `Send` / `Sync` impls.

Resolution: keep the full argument once, on `S` in height.rs, and have `Node`, `Path`, and `Prefix` say "`PhantomData<fn() -> H>`; see [`S`]" (the owner last re-justified the node.rs copy at 8f87ddd01; if that wording is preferred, move it to `S`). For each typed wrapper, one sentence stating the typed contract plus a link to the untyped method of record; make `floor`'s doc "The dual of [`ceiling`]" and keep the memo paragraph once. Acceptance: "Function pointers are unconditionally `Send + Sync`" occurs once under src/tree/typed/; each typed wrapper doc is a few lines and links its untyped counterpart.

### tree-typed-14: `compressed_prefix_len`: the typed wrapper has no caller, and both docs say a leaf's count is zero when compressed leaves are the common stored shape
- Where: src/tree/typed/node.rs:236-242 (related: src/tree/typed/untyped.rs:645-651, 292-301, 659-664; src/tree/typed/untyped/tests.rs:334, 343, 590-594)
- Class / severity / confidence: documentation / low / high
- Provenance: verified for the dead wrapper (grep: the only callers, untyped/tests.rs:334 and 343, take an untyped `Node` from `arb_tree`; `git log -S'.map(|n| n.compressed_prefix_len())'` shows the wrapper's only caller removed at fceb55f98); assessed (read) for the doc: `beneath` pushes onto any node, `from_sorted_leaves` extends a lone leaf's whole remaining spine, and `node_hash_preimage_is_in_path_order` hashes a leaf carrying `[0xAA, 0xBB]`
- Seen by: structure ([12]), prose ([24]); refutation: confirmed (both); history: [12] deliberate-but-expired (added for the borsh round-trip tests, dead since fceb55f98); [24] no-rationale-found
- Owner-gated: no

The `#[cfg(test)]` typed wrapper delegates to the untyped method but every caller invokes the untyped one directly, so it is dead test surface. Both docstrings say "Zero for a leaf or a non-compressed branch", which is false for every leaf under a collapsed spine, the shape `nested_singleton_wraps_extend_the_committed_prefix` drives through this very accessor; the module elsewhere (`is_leaf`: "regardless of any path-compressed prefix above it") already says leaves can be compressed.

Evidence:

       236	    /// Number of path-compressed prefix bytes on this node — i.e., the
       237	    /// count of singleton virtual-branch levels collapsed above the node's
       238	    /// actual content. Zero for a leaf or a non-compressed branch.
       239	    #[cfg(test)]
       240	    pub fn compressed_prefix_len(&self) -> usize {
       241	        self.inner.compressed_prefix_len()
       242	    }

    src/tree/typed/untyped/tests.rs:
       593	    let wrapped = leaf.beneath(0xAA).beneath(0xBB);
       594	    assert_eq!(wrapped.hash(), super::Hash::of(&[LEAF_TAG, 2, 0xBB, 0xAA]));

Resolution: delete node.rs:236-242; at untyped.rs:645-647 replace the last sentence with "Zero for an uncompressed node of either kind." Acceptance: `grep -rn compressed_prefix_len src` finds only the untyped definition and its two test callers; no docstring asserts that leaves have a zero prefix length.

### tree-typed-22: Two `expect` messages do not prove the impossibility they guard, and one inverts the geometry
- Where: src/tree/typed/prefix.rs:188-191 (related: src/tree/typed/prefix.rs:88-92, src/tree/typed/node.rs:361-366)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read against the impl bounds: `pop` at 184-187 requires `S<H>: Height`, so `H` is strictly below `Root`; `ErasedPrefix::pop` at 92 has the direction right; `git log -L188,191` shows the line survived the 9ebd1b8d2 accuracy sweep)
- Seen by: structure ([10]), prose ([28]), correctness ([40]); refutation: reframed (untyped.rs:237's `expect("non-empty prefix")` sits directly under its guard and is not a defect); history: no-rationale-found
- Owner-gated: no

`Prefix<H>::pop` says "a prefix above height Root has at least one byte to pop"; nothing is above `Root`, and the bound places `H` below it. The sibling at line 92 states it correctly. `Node<Z>::message` says "typed leaf failed to be a leaf", which names the failure rather than why it cannot happen (compare iter.rs:341 "a Leaf wraps a leaf node, by construction"). The codebase's standard: every `expect` message is a one-line proof; an inverted direction passes for true and is worse than none.

Evidence:

       188	        let byte = self
       189	            .hash
       190	            .pop()
       191	            .expect("a prefix above height Root has at least one byte to pop");

        92	            .expect("a prefix below the root has at least one byte to pop");

    src/tree/typed/node.rs:
       363	        self.inner
       364	            .as_leaf()
       365	            .expect("typed leaf failed to be a leaf")

Resolution: prefix.rs:191: "a prefix below the root has at least one byte to pop" (matching line 92). node.rs:365: "a `Node<Z>` wraps a leaf: the height-zero constructors admit nothing else". Acceptance: both messages state the reason the branch is unreachable; the two `pop` messages agree on direction.

### tree-typed-26: `Node::branch`'s `None` arm and singleton collapse are undocumented at both layers
- Where: src/tree/typed/untyped.rs:201-211 (related: src/tree/typed/node.rs:317-324, src/tree/traverse/unknown.rs:65-73, src/tree/traverse/act.rs:129, src/tree/traverse/join.rs:195, src/tree/typed/untyped/tests.rs:150-160)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: the body returns `None` for an empty fan and `beneath` for one child; unknown.rs:65-73, act.rs:129, and join.rs:195 rely on an emptied subtree vanishing)
- Seen by: prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Both `branch` docs say only "Construct a new branch node from ... children" over an `Option<Self>` return. The `None` arm is what makes redaction pruning delete empty subtrees, and the one-child arm is the canonical-shape collapse the hash convention rests on; a maintainer must read the body to learn either (documentation altitude: state every return arm).

Evidence:

       201	    /// Construct a new branch node from a list of children with distinct
       202	    /// indices (inverse to [`Node::into_children`]).
       203	    pub fn branch(children: Fan) -> Option<Self> {
       204	        match children.len() {
       205	            0 => None,
       206	            1 => {
    ...
       210	                Some(node.beneath(index))

Resolution: add to both docs: "`None` when `children` is empty: an empty subtree has no node. A single child is not materialized as a branch; it is returned collapsed into its compressed prefix (see [`beneath`]), so every materialized branch has at least two children." Acceptance: both `branch` docstrings name the empty and singleton arms; `empty_branch_is_none`'s testdoc can cite the contract rather than restate it.

### tree-typed-27: `from_sorted_leaves` panics on an empty run and on a consumed slot but has no `# Panics` section
- Where: src/tree/typed/untyped.rs:260-279 (related: src/tree/typed/untyped.rs:285-287, 308-312; src/tree/typed/node.rs:284-294; src/tree/mirror/streaming/remote/adapter/decode.rs:504-528; src/tree/mirror/streaming/backend/local.rs:201-220)
- Class / severity / confidence: documentation / low / high
- Provenance: verified for reachability (decode.rs:508-527 rejects out-of-scope and non-ascending leaves as `LeafOutsideScope`/`LeafOrder` before assembly; `Local::assemble` at local.rs:201-220 builds only non-empty runs); assessed for the doc gap
- Seen by: prose; refutation: confirmed, plus its new item (the release-mode duplicate-path behavior lands on the `expect` at 312 because the ascending check at 280 is debug-only); history: no-rationale-found
- Owner-gated: no

The preconditions (non-empty, strictly ascending, bare leaves, each consumed once) are stated in running prose while the body `expect`s at 287 (consumed slot) and 308-309 (empty run), and in a release build a run with duplicate paths reaches `expect("distinct 32-byte paths diverge before the bottom")` at 312 rather than the stated precondition. Every other hazard in the partition (`Hash::leaf`/`branch`, `ErasedPrefix::push`/`pop`, `Leaf::value`) has a named `# Panics` section, and all of these panics are programmer-error only: the decoder and `Local::assemble` uphold the preconditions at the trust boundary.

Evidence:

       262	    /// `leaves` pairs each full 32-byte path with its bare (prefix-free)
       263	    /// leaf node, **strictly ascending by path**, every path sharing its
       264	    /// first `depth` bytes; the run must be non-empty, and each node is
       265	    /// consumed exactly once (the `Option` lets the recursion move nodes
       266	    /// out of a shared slice).

       308	        let first = leaves.first().expect("a leaf run is non-empty").0;
    ...
       312	            .expect("distinct 32-byte paths diverge before the bottom");

Resolution: add `# Panics` to both docstrings: on an empty run, on a `None` slot, and (release) on duplicate paths, each with the one-line reason the wire path cannot produce it (the decoder rejects non-ascending and out-of-scope leaves; `assemble` builds non-empty runs); note that ordering and bareness are debug-asserted. Acceptance: both `from_sorted_leaves` docstrings carry a `# Panics` section matching the `expect` sites.

### tree-typed-32: Local prose slips in the maintainer docs
- Where: src/tree/typed/untyped/iter.rs:1-6 (related: src/tree/typed/untyped/iter.rs:270, 287-288, 376-378; src/tree/typed/hash.rs:218-219; src/tree/typed/prefix.rs:23-24; src/tree/typed/untyped.rs:568-569; src/tree/typed/node.rs:385)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log --follow --name-status -- src/tree/typed/untyped/iter.rs` shows `R100 src/tree/typed/untyped/node/iter.rs -> src/tree/typed/untyped/iter.rs`; untyped.rs:11 `mod iter;`; `git log -S'this replaces decoded'` names eb21c4d2d; `git log -S'struct Frozen'` names ef4239f87, renamed at 933505cde; `git show 9c73d7b46 -- src/tree/typed/untyped/iter.rs` shows the "32-byte" doubling arriving with the `Key` retirement's rewrap; the rest read)
- Seen by: structure ([9]), prose ([29]), correctness ([40]), perfapi ([54]); refutation: confirmed; history: iter.rs:5 deliberate-but-expired (true before the 48dc4323e move), untyped.rs:568-569 contradicts-hard-rule, the rest no-rationale-found; the "undercounts its walks" clause is disputed (the doc counts shells of the shared frontier engine, and `RangeOwned` has its own spine engine; omitting it is a completeness item)
- Owner-gated: no

Each is a small factual slip a reader trips on: the module doc names its parent `node` (the parent is `untyped`; `typed::node` is a different module) and omits `RangeOwned` from its listing sentence; `within`'s doc doubles "32-byte" across a line break; `RangeOwned`'s doc lacks a paragraph break at 287-288 so two paragraphs render as one; hash.rs:218 calls a `LazyLock` value "A compile-time constant"; prefix.rs:24 says an erased prefix's length "*is* the height" and then gives the complement; untyped.rs:568-569 is ungrammatical and refers to a replaced implementation ("the split folds this replaces"), which the hard rule sends to git; the scare-quoted "frozen" (iter.rs:270) and "Freeze" (node.rs:385) are the retired `Frozen` type's name surviving as vocabulary where "owned" already says it.

Evidence:

         1	//! Leaf iterators over the untyped tree: a shared frontier walk and its two
         2	//! shells, [`Iter`] (the unfiltered, exact-size walk) and [`Range`]
         3	//! (the walk filtered to a causal [`causally::Query`]).
         4	//!
         5	//! A child module of [`node`](super) so the walk can match on the parent's
         6	//! private [`Children`] variants and path-compression internals directly.

       376	    /// each leaf still reconstructs a full 32-byte
       377	    /// 32-byte path. `path.len()` plus the height of

    src/tree/typed/hash.rs:
       218	        // A compile-time constant: memoize it rather than re-hashing the
       219	        // four fixed bytes on every empty-root read.

    src/tree/typed/prefix.rs:
        23	/// A prefix with its height tag forgotten: the same accumulated path
        24	/// bytes, whose length *is* the height (`32 - height` bytes at `height`).

    src/tree/typed/untyped.rs:
       568	    ///   join legs sharing every operand decode — where the split folds
       569	    ///   this replaces decoded each version once per lattice direction.

Resolution: iter.rs:1-6: "Leaf walks over the untyped tree: a shared borrowing frontier engine beneath [`Iter`] and [`Range`], and the owned spine walk [`RangeOwned`]. A child module of [`untyped`](super) so the walks can match the parent's private `Children` variants directly." Delete one "32-byte" at 376-377; insert the blank `///` at 287-288; hash.rs:218: "A fixed value: compute it once, on first read, rather than re-hashing the four bytes on every empty-root read"; prefix.rs:24: "whose length determines the height (`32 - height` bytes at `height`)"; untyped.rs:568-569: a present-tense statement of the fused hull's cost (one decode per operand serving both lattice directions); drop the quoted "frozen" and "Freeze" for "owned". Acceptance: each cited line reads correctly; `grep -rn 'this replaces\|"frozen"' src/tree/typed` is empty.

### tree-typed-33: `Iter`'s doc names `unknown` and `Tree::join` as callback consumers of its order; neither takes a callback or uses `Iter`
- Where: src/tree/typed/untyped/iter.rs:171-174 (related: src/tree.rs:567, src/tree/traverse/unknown.rs:26, src/tree/traverse/join.rs:154-159, src/tree/traverse/act.rs:42)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (tree.rs:567 `pub fn join(&mut self, other: Tree<T>) -> bool`; unknown.rs:26 takes no callback; join.rs:154-159 walks `Children::iter` fans in lockstep and never names `Iter`; `git log -S'deterministic callback delivery'` names f7257af49, when `Tree::join` took `on_recv`/`on_send`; 262568f9e "retire the observation callbacks")
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

The parenthetical describes a dependency that was retired with the observation callbacks; it sends a maintainer looking for a callback-delivering join that does not exist (Principle 5).

Evidence:

       171	/// it. (The public observers on [`Rumors`](crate::Rumors) still promise
       172	/// nothing about order, but [`unknown`](crate::tree::traverse::unknown)
       173	/// and `Tree::join` lean on the ascending forward order for their own
       174	/// deterministic callback delivery.)

Resolution: delete the parenthetical, or replace it with the consumer that actually relies on `Iter`'s order, if one does. Acceptance: every consumer the `Iter` doc names calls `Tree::iter` or `Iter::root`, or is removed from the sentence.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| tree-typed-2 | src/tree/typed/hash.rs:6-12 | The partition's only public rustdoc opens its second paragraph with a verbless fragment. | Proposed sentence in the entry; owner-gated as user-facing prose. |
| tree-typed-28 | src/tree/typed/untyped.rs:520-525, 548-554; untyped/tests.rs:282 | "door" is `before`'s maintainer coinage for a constructor, undefined in rumors. | Name `Span::at` and `Span::dominance`. |

## Mirror common

### mirror-common-1: Public `MirrorError` docs speak at maintainer altitude
- Where: src/tree/mirror.rs:44-49 (related: src/tree/mirror.rs:66-75, src/error.rs:52-53, src/error.rs:283-289, src/tree/mirror/streaming.rs:211-215)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: no rationale found (the coherence argument is deliberate and recorded in 9c8800a6e; its placement on public rustdoc was never ruled)
- Owner-gated: no

`mirror::Error<C, S>` reaches users as `MirrorError` (error.rs:53) and `Error::Mirror` (error.rs:289). Its variant docs say "The protocol participant supplied in the client position failed", but a library user supplies no participants and is not told that in `MirrorError` the client position is always the local replica's walk and the server position the wire proxy (the alias fixes both). The `From<C>` impl's rustdoc is entirely maintainer material (coherence overlap, frame-relative instantiation, the party boundary), rendered on a public item. Documentation altitude: public rustdoc names only what the API reaches; the why-only-one-`From` argument belongs to the maintainer.

Evidence:

    44	    /// The protocol participant supplied in the client position failed.
    45	    #[error("mirror client failed")]
    46	    Client(#[source] C),
    47	    /// The protocol participant supplied in the server position failed.
    48	    #[error("mirror server failed")]
    49	    Server(#[source] S),
    ...
    68	/// Only the first position can have this impl: its second-position mirror
    69	/// would overlap with it when `C = S`, and coherence permits one. This
    70	/// asymmetry shapes how the streaming driver uses the sum — each party runs
    71	/// its session at the *frame-relative* instantiation with its own error
    72	/// first, so `?` lifts either party's backend errors through this one impl,

Resolution: Variant docs: "The client-position participant failed; in a gossip session ([`MirrorError`](crate::MirrorError)) this is the local replica's walk." and "... the wire proxy for the peer." Move the coherence paragraph to a `//` comment above the impl and leave its rustdoc at "Lift a client-position error into the sum." Acceptance: the public docs on `Error<C, S>` and its `From` impl name only what a `MirrorError` holder can observe; the coherence argument survives as a maintainer comment.

### mirror-common-4: Cancel-safety hazards stated inline instead of under the crate's `# Cancel safety` section
- Where: src/tree/mirror/cbor.rs:219-224 (related: src/tree/mirror/handshake.rs:244, src/tree/mirror/handshake.rs:264-282, src/link.rs:252, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:108)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: eight sites in the crate use a `# Cancel safety` section; cbor.rs:223-224 states the hazard as a trailing sentence; handshake.rs:244 and :264 state cancel safety in prose without a section)
- Seen by: prose; refutation: reframed (`Staged` does state its cancel safety, at :244 and :264; only the section form and `fill`'s return-arm inventory are missing); history: no rationale found (the section convention predates cbor.rs's inline sentence)
- Owner-gated: no

`read_head_async` ends its summary paragraph with "Not cancel safe: a dropped future may have consumed part of the head." The crate spells this hazard as a `# Cancel safety` section elsewhere, so a reader scanning for the section misses it here. `Staged::fill`, whose reason to exist is cancel safety, states it only in prose and does not document its three outcomes (`Filled`; `Closed` only when no byte has arrived; `Truncated` on a close mid-frame). Hazards get uniform named sections.

Evidence:

    221	/// A clean end-of-stream *before the first byte* returns `Ok(None)`; an
    222	/// end-of-stream inside the head is an
    223	/// [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) I/O error. Not
    224	/// cancel safe: a dropped future may have consumed part of the head.
    ...
    264	    /// Continue receiving the fixed frame without losing cancelled progress.
    265	    pub(crate) async fn fill<R>(&mut self, reader: &mut R) -> Result<Fill, Error>

Resolution: cbor.rs: move the sentence under `# Cancel safety`. handshake.rs:264: add `# Cancel safety` ("Cancel safe: bytes received before a drop stay in `self`, and the next call resumes from them.") and state the return arms. Acceptance: every async reader in the partition that is not cancel safe, and `Staged::fill`, carries a `# Cancel safety` section.

### mirror-common-9: "Handshake" names both the preamble module and the greeting exchange, against `message.rs`'s own definitions
- Where: src/tree/mirror/handshake.rs:1-8 (related: src/tree/mirror/streaming/message.rs:34-36, src/tree/mirror/streaming.rs:30, src/tree/mirror/streaming.rs:35-36, src/tree/mirror/streaming.rs:155, src/tree/mirror/streaming.rs:165, src/error.rs:21, src/error.rs:195)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the cited lines; grep for `mid-handshake` in error.rs finds :21 and :195, both about the preamble truncation)
- Seen by: prose; refutation: confirmed; history: no rationale found (the definition arrived in c5f1210a3 while editing this module's own doc and leaving line 1)
- Owner-gated: no

`message.rs` defines three terms as three things: the preamble is the fixed transport bytes, the handshake is the act of exchanging greetings, a `Greeting` is the message. The module that implements the preamble is named `handshake` and opens "The transport handshake opening every mirror session"; `streaming.rs`'s module doc uses both senses in one paragraph (`[`handshake`]` the greeting-exchange function at :30, `[`super::handshake`]` the preamble module at :36); and `handshake()` binds the received `Greeting` to `our_handshake` under a summary that says "Exchange versions". A term defined in the codebase must be used as defined everywhere else in it, or the definition is a trap.

Evidence:

    1	//! The transport handshake opening every mirror session.

    message.rs:
    34	/// Three terms, three things: the *preamble* is the fixed transport bytes
    35	/// that precede any message, the *handshake* is the act of exchanging
    36	/// greetings, and a `Greeting` is the message each side contributes to it.

Resolution: Adopt `message.rs`'s definitions: rename `pub(crate) mod handshake` to `preamble` (no public path moves; `PreambleDefect` is re-exported from error.rs by path) and open it "The fixed preamble opening every mirror session."; rewrite streaming.rs:35-36 as "first exchanges the fixed [`super::preamble`]"; rename `our_handshake` to `our_greeting` and make :155 "Exchange greetings and return both connected protocol states." In error.rs (another partition), "mid-handshake" → "mid-preamble" at :21 and :195. Acceptance: `grep -rn -i handshake src/tree/mirror` returns only the greeting-exchange sense.

### mirror-common-12: `VersionMismatch` says "we selected" a protocol nothing selects any more
- Where: src/tree/mirror/handshake.rs:180-184 (related: src/tree/mirror/handshake.rs:204, src/error.rs:14, src/error.rs:76, src/error.rs:179, src/error.rs:201, src/protocol.rs:1, .agent-notes/2026-09-01-v1-retirement/README.md:153-160)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: no `fn protocol(` exists in src; `Protocol` has the single variant `V2 = 2`; handshake.rs:127 hard-codes `local_protocol: Protocol::V2`; the wording survives at the seven listed sites)
- Seen by: structure, prose; refutation: confirmed; history: deliberate but expired (accurate under 83edcd944's two-variant, builder-selectable `Protocol`; 368da2a5 removed the builders and swept five other "selected" lines from handshake.rs, leaving :180 and :204; the retirement note rules that `local_protocol` stays as wire vocabulary)
- Owner-gated: no

With the `.protocol()` builders gone and `Protocol` down to one variant, no code path selects a protocol. The Display text "we selected {local_protocol:?}" (mirrored at error.rs:76, the string users see) and "the selected dialect" (handshake.rs:204, error.rs:179, :201) describe a removed affordance, and error.rs:14's remedy column ("select the same [`Protocol`] at both ends") invites a repair that no longer exists when the only remedy is aligning crate versions. No ghost references: prose, Display text included, may not describe code that no longer exists.

Evidence:

    180	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    181	    VersionMismatch {
    182	        local_protocol: Protocol,
    183	        remote_version: u64,
    184	    },
    ...
    204	/// peer opened as a rumors stream of the selected dialect, but one

Resolution: "peer speaks rumors protocol version {remote_version}; this build speaks {local_protocol:?}" here and, byte-identical, at error.rs:76; handshake.rs:204 "opened as a rumors stream of this build's dialect"; sweep error.rs:14, :179, :201 and protocol.rs:1 ("Selectable") in their own partitions. Dropping `local_protocol` from the public variant is the further step the retirement note already declined. Acceptance: `grep -rn -i 'we selected\|selected dialect\|selectable' src` is empty; the two `VersionMismatch` strings are identical; tests/handshake.rs still passes.

Synthesis note: Same residue as api-core-4, anchored at the `Display` string's twin in handshake.rs.

### mirror-common-23: `erased.rs` module doc compares today's code to a prior design state ("exactly as before")
- Where: src/tree/mirror/streaming/erased.rs:29-30 (related: AGENTS.md:128-130)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read)
- Seen by: prose; refutation: confirmed; history: contradicts the hard rule (the clause compares to the pre-erasure design of d1b7f00e; AGENTS.md's first hard rule forbids prose referring to code that no longer exists)
- Owner-gated: no

"is a compile error, exactly as before" compares the present design to one that no longer exists. The sentence is complete and true without the clause. Prose speaks in the present tense; provenance lives in git.

Evidence:

    29	//! Outside the walk, pairing a height-5 payload with a height-6 consumer
    30	//! is a compile error, exactly as before: the schedule's typestates and

Resolution: Delete ", exactly as before". Acceptance: `grep -n 'as before' src/tree/mirror/streaming/erased.rs` is empty.

Synthesis note: Sibling of materialized-25.

### mirror-common-27: `Greeting::payload_depth_limit` is documented "in scopes"; the type's unit is decode recursion steps, and "scope" is a different crate term
- Where: src/tree/mirror/streaming/message.rs:104-106 (related: src/message.rs:61-62, src/message.rs:81-83, src/tree/mirror/streaming/stats.rs:45-47)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: src/message.rs:61-62 "counted in the CBOR decode engine's recursion steps", :81-83 `get` "in decode recursion steps"; stats.rs:45-47 defines scope publicly as one prefix of the tree)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (71de90c1 documented both sites "in scopes"; 6e4b6eea re-denominated src/message.rs to recursion steps and did not touch this file)
- Owner-gated: no

The wire field's stated unit disagrees with the unit of the value it carries, and the word it uses means the subtree one question names everywhere else in the mirror. One term, one referent; a unit stated at the wire field must match the type's.

Evidence:

    104	    /// The sender's configured payload nesting-depth limit
    105	    /// ([`Peer::payload_depth_limit`](crate::Peer::payload_depth_limit)),
    106	    /// in scopes.

Resolution: "in decode recursion steps". Acceptance: the field doc's unit matches `PayloadDepthLimit::get`'s.

### mirror-common-30: The phase schedule's spine is half undocumented
- Where: src/tree/mirror/streaming/protocol.rs:24-31 (related: src/tree/mirror/streaming/protocol.rs:56, src/tree/mirror/streaming/protocol.rs:65, src/tree/mirror/streaming/protocol.rs:77, src/tree/mirror/streaming/protocol.rs:123, src/tree/mirror/streaming/protocol.rs:163, src/tree/mirror/streaming/protocol/peer.rs:60, src/tree/mirror/streaming/protocol/peer.rs:78, src/tree/mirror/streaming/protocol/peer.rs:92, src/tree/mirror/streaming.rs:80, src/tree/mirror/streaming.rs:101, src/tree/mirror/streaming/message.rs:63)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read every cited line; `grep -rn missing_docs src Cargo.toml .cargo justfile` is empty, and tools/testdoc checks test functions only)
- Seen by: prose; refutation: confirmed; history: no rationale found (`pub trait Protocol` was undocumented at birth, b6625f5fa, and stayed so)
- Owner-gated: no

The traits a maintainer must read to follow the type-level schedule carry no doc comment: `Protocol`, `Connect`, `Accept`, `CompleteConnect`, `Responder`, `Reply`, and the macro-generated `Peer`/`Server`/`Client`; `Handshaken` and its `peer()` likewise; and `Greeting::version` is the one undocumented field of an otherwise fully documented struct, the field whose equality ends a session before the descent. Private docs serve the maintainer: a phase trait's contract (which height it sits at, what it consumes, what it yields, which phase follows) is what the reader cannot recover from bounds alone.

Evidence:

    24	pub trait Protocol: Send {
    25	    type Height: Height;
    26	    // `Send + 'static` because these traits' outgoing streams carry it, both
    27	    // bare and inside an `OutputError`, and the driver moves it into its
    28	    // error slot.
    29	    type Error: Send + 'static;
    30	    type Output: Send;
    31	}

Resolution: One sentence apiece stating height, input, output, and successor, e.g. `Connect`: "The client's opening phase at the root: produce our greeting and the state that awaits the peer's."; `Reply`: "One descent step at `Self::Height`: consume the peer's replies at this height and yield ours one height down."; `Peer`/`Client`/`Server`: "The full chain from a connected state to a terminal, spelled out so the compiler holds `mirror_connected`'s schedule to it."; `Handshaken`: "Both participants past the greeting exchange, plus the election keys."; `Greeting::version`: "The sender's set version; equal versions end the session at the greeting." Acceptance: every `pub` or `pub(crate)` trait, struct, and field in protocol.rs, protocol/peer.rs, streaming.rs, and message.rs has a doc comment whose first sentence stands alone (a review check: `missing_docs` ignores private items).

### mirror-common-33: The `define_peer!` comment locates `mirror_connected` in the wrong file and states as a manual obligation what the compiler enforces
- Where: src/tree/mirror/streaming/protocol/peer.rs:108-111 (related: src/tree/mirror/streaming/driver.rs:152-172, src/tree/mirror/streaming.rs:74, src/tree/mirror/streaming/protocol/peer.rs:60-67)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: `pub(super) async fn mirror_connected` is defined at driver.rs:152 only; streaming.rs:74 imports it; the compile-time coupling is assessed by reading the nested `Next` bounds against the driver's 15 loop `reply`s plus one)
- Seen by: structure, prose, correctness, perfapi; refutation: reframed (the round arithmetic is consistent as written: 15 `_` yield 16 nested `Reply` bounds matching the driver's fifteen loop rounds plus the final `i.reply`; only the location and the manual-coupling framing are wrong); history: deliberate but expired (8037ad045 wrote the comment while the function lived at streaming.rs:83; cbfe1aff3 moved it to driver.rs and left the comment)
- Owner-gated: no

The function lives in driver.rs, and a driver whose round count disagrees with the chains fails to type-check (after the wrong number of `reply` calls the state is still a `Reply` phase, which does not implement `CompleteInitiator`, or a `Reply` bound is missing for the extra call), so the coupling is compiler-held, not hand-held. A comment naming the wrong file sends the maintainer to the wrong place, and it undersells the guarantee the types give.

Evidence:

    108	// One `_` per exchange round: the initiator descends heights 31 → 1 in
    109	// fifteen rounds of two heights each, the responder 30 → 2 in fourteen.
    110	// `mirror_connected` in streaming.rs drives this same schedule; the counts
    111	// must move together.

Resolution: "`driver::mirror_connected` drives this same schedule; a count that disagrees with its loop fails to compile against the chains' terminal bounds, which is what holds the two in step (`seq!` and macro repetition cannot share a named constant)." The larger redesign (height-indexed chain traits deriving the depth from `Root`, dissolving the `_` roster) is a design proposal nobody has compiled; see the open questions. Acceptance: the comment names driver.rs and states the compile-time coupling.

Synthesis note: Same comment as module-graph-5's first site.

### module-graph-5: Stale location and layout comments
- Where: src/tree/mirror/streaming/protocol/peer.rs:108-111 (related: src/tree/mirror/streaming/driver.rs:152, :164-168; src/tree/mirror/streaming/protocol.rs:163-168, :198; src/tests.rs:25-28; src/tree/mirror/handshake.rs:14-16, :32, :45-48, :53; src/peer/gossip.rs:4-6, :54; tests/handshake.rs:1)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (each site read; `V2_PREAMBLE_LEN` computed from its definition as 11 + 1 + 17 + 1 = 30; `grep -n -E '^\s*(pub(\([^)]*\))?\s+)?(const|static)\s' src/peer/gossip.rs` returns only line 54; `grep -rn 'mirror::remote' src tests` returns only `tests/handshake.rs:1`; `grep -rn 'fn preamble' src` returns only `handshake.rs:298`)
- Verification: reframed on the round count: `define_peer!` builds a type-level chain whose every `Reply::Next` has `Height = <Self::Height as ReplyHeight>::Next` and whose terminals require `Height = Z`, so a wrong `_` count fails to compile against `mirror_connected`; the compiler already ties the counts, and the sweep's const-assert is unnecessary. One site added: `tests/handshake.rs:1`; history: no-rationale-found
- Owner-gated: no

Four comments describe code that is elsewhere or no longer shaped that way: `protocol/peer.rs` places `mirror_connected` in `streaming.rs` (it is `driver.rs:152`) and presents the round count as hand-coupled when the type checker enforces it; `src/tests.rs` glosses `PREAMBLE_LEN` with a field layout summing to 25 where `handshake.rs` defines and asserts a 30-byte item; `gossip.rs`'s module doc claims the preamble constants when its only constant is the epilogue marker; and `tests/handshake.rs` names a `mirror::remote::preamble` that does not exist (the function is `tree::mirror::handshake::preamble`).

Evidence:

    src/tree/mirror/streaming/protocol/peer.rs:
       108	// One `_` per exchange round: the initiator descends heights 31 → 1 in
       109	// fifteen rounds of two heights each, the responder 30 → 2 in fourteen.
       110	// `mirror_connected` in streaming.rs drives this same schedule; the counts
       111	// must move together.

    src/tree/mirror/streaming/driver.rs:
       152	pub(super) async fn mirror_connected<B, I, R>(
    ...
       164	        for _ in 0..15 {

    src/tree/mirror/streaming/protocol.rs (the compiler-side tie):
       163	pub trait Reply<B: Backend<Node<Z>: Leaf>>: Protocol<Height: ReplyHeight> + Sized {
       164	    type Next: Protocol<
       165	            Height = <Self::Height as ReplyHeight>::Next,
    ...
       198	pub trait CompleteInitiator<B: Backend<Node<Z>: Leaf>>: Protocol<Height = Z> + Sized {

    src/tests.rs:
        25	/// The preamble's wire length: magic(6) + proto_version(2) + network(16) +
        26	/// intent(1). The fault-injection budgets
        27	/// below land cuts on exact protocol boundaries relative to this.
        28	const PREAMBLE_LEN: usize = crate::tree::mirror::handshake::V2_PREAMBLE_LEN;

    src/tree/mirror/handshake.rs:
        15	//! Every field's head is one byte at the values the dialect admits, so
        16	//! the item is 30 bytes, fixed; that width is part of the dialect, so
    ...
        53	pub(crate) const V2_PREAMBLE_LEN: usize = V2_PREFIX.len() + 1 + (1 + NETWORK_LEN) + 1;

    src/peer/gossip.rs:
         4	//! Also here: the preamble constants every session leads with, and the
         5	//! [`PartyGuard`] that snaps a speculatively donated party back in place
    ...
        54	const EPILOGUE_MARKER: [u8; 2] = [0x61, b'.'];

    tests/handshake.rs:
         1	//! Protocol preamble exchange (`mirror::remote::preamble`).

Resolution: Fix each comment toward the code: `driver.rs`, with "the compiler holds the two counts together through the type-level height descent" in place of "the counts must move together"; the 30-byte `55799(["rumors", version, network, intent])` layout; `handshake.rs` owns the preamble constants (drop the clause from `gossip.rs:4-5`); `tree::mirror::handshake::preamble`. Acceptance: the four comments name the file and layout that exist; no new constant or assert is added.

Synthesis note: Overlaps mirror-common-33 (the `define_peer!` comment), testing-infra-16 (src/tests.rs:25-28), and tests-disruption-handshake-20 (tests/handshake.rs:1); each carries its own resolution and they agree.

### mirror-common-34: "Seam" is an undefined house metaphor used for three different boundaries, including in public `SessionStats` docs
- Where: src/tree/mirror/streaming/stats.rs:29-29 (related: src/tree/mirror/streaming/stats.rs:6, src/tree/mirror/streaming/stats.rs:104, src/tree/mirror/streaming/stats.rs:122, src/tree/mirror/streaming/erased.rs:1, src/tree/mirror/streaming/erased.rs:12, src/tree/mirror/streaming.rs:19, src/tree/mirror/framing.rs:42, src/tree/mirror/framing/tests.rs:18)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: nine sites in the partition, 48 crate-wide, none defining the word)
- Seen by: prose; refutation: reframed (one metaphor, a junction between layers, applied to three different junctions; the doctrinal point stands and three of the sites are public rustdoc); history: no rationale found (house dialect from 487e17ea3 and d1b7f00e)
- Owner-gated: no

In stats.rs the word means the code location where a count is taken; in erased.rs and streaming.rs the typed/erased boundary; in framing.rs a chunk boundary in a byte stream. A library user reading `SessionStats` meets "the seam named in its field docs" with no anchor. Metaphors only where they can be rewritten as mechanism without loss; each site here can be.

Evidence:

    29	/// Every count is taken locally, at the seam named in its field docs, while

Resolution: stats.rs: "at the point named in its field docs" / "taken exactly at that boundary, between the codec and the transport stream" / "Counted at the same codec boundary as"; erased.rs and streaming.rs: "the height-erased boundary"; framing.rs:42 "at the chunk boundaries"; framing/tests.rs:18 "every partial-read boundary". Acceptance: `grep -rn -i seam` over the nine files is empty.

### fresh-eyes-5: SessionStats announces "Two deliberate boundaries" and lists one
- Where: src/tree/mirror/streaming/stats.rs:33-38 (related: src/peer/gossip.rs:174-179)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read stats.rs:26-41; `git log -S"Two deliberate boundaries"` finds the block's birth in 487e17ea3 with a second bullet, "[`Protocol::V1`](crate::Protocol::V1) sessions report zero in every field", which commit 368da2a5 deleted)
- Verification: confirmed, cause identified; history: deliberate-but-expired (the second bullet was the V1 one; the retirement removed it and left the count)
- Owner-gated: no

The public type's doc promises two bullets and delivers one. This is the
rot a hand-maintained count invites: the V1 bullet was deleted with V1, and
"Two" stayed.

Evidence:

    src/tree/mirror/streaming/stats.rs
    33	/// Two deliberate boundaries:
    34	///
    35	/// - **No duration field.** The caller owns the clock: wrap the `gossip`
    36	///   call (or the `gossip_when` stream's polls) in whatever timing
    37	///   instrument the application already uses. A duration measured inside
    38	///   the crate would bake in one notion of time and satisfy nobody's.
    39	#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]

Resolution: Drop the count ("One deliberate boundary:" or fold the bullet
into a sentence), or restore a second item if one is intended (the "every
count is local; the two ends report their own numbers" point at
gossip.rs:174-175 is the natural candidate). Acceptance: the heading's count,
if any, equals the bullets beneath it.

Synthesis note: Same site as mirror-common-35 and api-audit-17; the three agree on cause and differ only on whether to restore a second bullet.

### mirror-common-35: Public `SessionStats` doc announces "Two deliberate boundaries" and lists one
- Where: src/tree/mirror/streaming/stats.rs:33-38 (related: src/tree/mirror/streaming/stats.rs:143-145)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (git: `git show 487e17ea3:src/tree/mirror/streaming/stats.rs` shows the second bullet, "[`Protocol::V1`](crate::Protocol::V1) sessions report zero in every field."; 368da2a5 removes exactly those lines and leaves the heading)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: deliberate but expired (the count was correct when written; the V1 retirement deleted the bullet and not the count)
- Owner-gated: no

The public rustdoc on `SessionStats` (re-exported at the crate root) opens a two-item list and delivers one bullet. Prose speaks in the present tense and carries no hand-maintained counts; this one rotted under the code and lands on the library user.

Evidence:

    33	/// Two deliberate boundaries:
    34	///
    35	/// - **No duration field.** The caller owns the clock: wrap the `gossip`
    36	///   call (or the `gossip_when` stream's polls) in whatever timing
    37	///   instrument the application already uses. A duration measured inside
    38	///   the crate would bake in one notion of time and satisfy nobody's.

Resolution: Fold the bullet into a sentence ("There is deliberately no duration field: the caller owns the clock, ..."), dropping the enumerated form; or, if a second boundary is real today (`window_granted` at :143-145 already says it "is a summary, not the whole vector"), state it as the second bullet. Do not restore the V1 bullet. Acceptance: the heading's count matches the list beneath it, or there is no count.

Synthesis note: Same site as fresh-eyes-5 and api-audit-17.

### mirror-common-37: `tasks::complete` is documented as a race; its body is a fail-fast join
- Where: src/tree/mirror/streaming/tasks.rs:19-37
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: after either `select!` branch resolves `Ok`, the body awaits the other to completion, `finish.await` at :29 and `tasks.await?` at :33, so `Ok` requires both and the first `Err` short-circuits, which is `future::try_join`'s contract)
- Seen by: prose; refutation: confirmed; history: no rationale found (doc and body are their birth forms from cbfe1aff3; the mismatch is original)
- Owner-gated: no

"Race" tells the caller the loser is dropped; the body never drops a branch that has not finished. A doc's first sentence stands alone in a listing and must be accurate; race and join are opposite contracts for the caller's cancellation expectations.

Evidence:

    19	/// Race registered work against its terminal operation, failing on either.
    20	pub async fn complete<O, E>(
    ...
    31	        output = &mut finish => {
    32	            let output = output?;
    33	            tasks.await?;
    34	            Ok(output)
    35	        }

Resolution: Either rewrite the summary ("Run registered work alongside its terminal operation: both must complete, and the first error cancels whatever remains.") or replace the body with `future::try_join(try_run_all(tasks), finish).await.map(|((), output)| output)`, which has the same semantics (unbiased `select!` imposes no ordering) and deletes the hand-rolled select. Acceptance: the summary names the semantics the body has; if the body changes, the streaming suites still pass.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| mirror-common-2 | src/tree/mirror/cbor.rs:59-62 and the sites listed | Derived arithmetic ("17 of the fixed item's 30 bytes", "a 33-arm match", MB figures) and caller rosters written as literals beside the constants that derive them. | Cite the constants and the pin by name; state the property instead of the roster. |
| mirror-common-13 | src/tree/mirror/handshake.rs:259-262; streaming.rs:32-33 | `Staged::is_empty` is documented by its caller's question; the tree-less proxy is called "backed by trees". | Two one-sentence rewrites. |
| mirror-common-21 | src/tree/mirror/streaming/driver.rs:103-114 | Why every `mirror!` party ident carries a `(state, route)` tuple is recorded only in commit history. | State the macro-hygiene rationale in the macro doc. |
| mirror-common-26 | src/tree/mirror/streaming/message.rs:80-82 (three more sites) | "the pinned pairwise lemmas" cites `before`'s subadditivity proptests without naming them. | Name `join_encoding_is_subadditive` and `meet_encoding_is_subadditive`. |
| fresh-eyes-11 | stats.rs:29, 104, 122; src/peer/gossip.rs:177-178; src/link/routed.rs:214 | "seam" in the rustdoc of five public items. | "boundary" at the five public sites. |
| api-audit-17 | src/tree/mirror/streaming/stats.rs:33-38 | "Two deliberate boundaries" lists one; the second was the V1 bullet. | Fix the count or restore a second bullet (same site as fresh-eyes-5 and mirror-common-35). |
| mirror-common-36 | stats.rs:42-43; message.rs:59-60, 171-189; party/tests.rs:186, 247; framing/tests.rs:9-10 | "genuine", loose "sound", "honestly", "real"; `Reaction`'s variant docs narrate in the first person. | Reword; recast `Reaction` in `Greeting`'s third-person voice. |

## Streaming backend and window

### streaming-backend-window-19: convert.rs's module doc describes a dormant cross-backend converter; the wire adapter runs the module on every supplied node
- Where: src/tree/mirror/streaming/convert.rs:1-8 (related: src/tree/mirror/streaming.rs:22; src/tree/mirror/streaming/remote/adapter.rs:42-67; src/tree/mirror/streaming/remote/adapter/encode.rs:185; src/tree/mirror/streaming/remote/adapter/decode.rs:88, 408; src/tree/mirror/streaming/erased.rs:299-334)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep: erased.rs:308 `backend.leaves::<H>(...)` and :330 `.assemble::<H>(leaves)` dispatch to the trait methods whose defaults are `Convert::explode`/`Convert::assemble`; encode.rs:185 and decode.rs:88, 408 run them on every supplied node; `grep -rn Converted src` is empty; adapter.rs:44-58 documents exactly this path)
- Seen by: structure, prose; refutation: confirmed; history: deliberate-but-expired (the paragraph was written for `convert::Converted` at 01643fa5, deleted at 7c32f0e2; the adapter became the consumer at cbfe1aff3)
- Owner-gated: no

The doc frames `Convert` as what lets "a heterogeneous pair meet" and says "a homogeneous session pays nothing" and "the protocol itself converts nowhere". Today the wire carries supplies as leaf records, so every supplied node is exploded on encode (`Backend::leaves`) and reassembled on decode (`Backend::assemble`), within one backend, on every session; the supply-decode memory charge rides on this fold. A maintainer looking for where leaf runs become nodes again would not recognize the module from its doc. AGENTS.md hard rule: nothing in the tree describes code that no longer exists; the parent's one-line summary (streaming.rs:22, "the leaf conversion boundary between backends") is already closer to the truth.

Evidence:

         1	//! Re-represent nodes from one backend in the node types of another.
         2	//!
         3	//! A node converts by exploding to leaves in the source backend and
         4	//! reassembling in the target.
         5	//!
         6	//! The protocol itself converts nowhere: both parties of a session name one
         7	//! backend, and a homogeneous session pays nothing. This module is what lets a
         8	//! heterogeneous pair meet, by re-representing each node-carrying message.

Resolution: Rewrite around what the module does: explode a height-`H` node stream to the prefix-ordered leaf stream beneath it and reassemble height-`H` nodes from one, level by level through `Backend::children`/`Backend::parent`; the wire carries leaves, never nodes, so the encoder runs `Backend::leaves` and the decoder `Backend::assemble` on every supplied node, and a backend may override both in bulk. One sentence may note that leaf records are backend-neutral, so two peers' backends need not coincide. Renaming the module is optional taste. Acceptance: the module doc names the encode/decode call sites as consumers and no longer states that a homogeneous session pays nothing.

### streaming-backend-window-1: `Backend` family documented for an implementer the crate cannot admit
- Where: src/tree/mirror/streaming/backend.rs:1-20 (related: src/tree/mirror/streaming/backend.rs:158-162, 195-197; src/lib.rs:322, 328-345; src/conformance.rs:16-19; src/conformance/backend.rs:41-47)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: `mod tree;` is private at lib.rs:322; lib.rs:328-345 re-exports no member of the `Backend`/`Node`/`Leaf`/`ErasedNode`/`Root`/`Convert` family; `conformance::backend` is `#[cfg(test)] pub(crate)` at conformance.rs:18-19)
- Seen by: prose, perfapi; refutation: confirmed; history: already-known (the sync-budget record, `.agent-notes/2026-07-22-sync-budget/sync-budget.md:577-581`, states the trait is crate-internal today and goes public with the conformance suite)
- Owner-gated: yes: publishing the boundary reopens a recorded deferral; the one-sentence status fix does not

The module doc and the trait docs address a third-party storage implementer ("a storage engine of its own"; the conformance suite "is how an implementation proves its account"), yet no member of the family is reachable outside the crate, and only `conformance/backend.rs` says so. Documentation altitude: prose written for an audience the API does not reach, in the one module a maintainer adding a backend reads first.

Evidence:

         3	//! A [`Backend`] decides what a tree node physically is — a value carried
         4	//! in the node's handle, or a reference into storage the backend owns —
         5	//! and the streaming protocol is generic over that decision: an
         6	//! implementation may hold its tree entirely in memory or in a storage
         7	//! engine of its own, and the session schedule is identical either way.
    ...
        18	//! backend conformance suite (`crate::conformance::backend`, compiled as
        19	//! this crate's own test gate; see [`crate::conformance`]) is how an
        20	//! implementation proves its account.

Resolution: Add one sentence to the module doc stating the boundary is crate-internal and `Local` is the sole production implementation (the wording at conformance/backend.rs:43-47 already exists). Keep the implementer guidance that serves the maintainer adding a backend. When the boundary is published, do the trait-shape pass then: `parent` takes an owned `Vec`, `assemble`'s signature names the `pub(crate)` alias `BoxNodeStream`, and `Error: Send + 'static` carries no `Debug` or `std::error::Error` bound. Acceptance: backend.rs states the boundary's status in its module doc; no sentence in the family addresses an external implementer without that framing.

### streaming-backend-window-6: `Root<B>::max_version_bytes` narrates the in-memory backend's memoization at a backend-generic accessor
- Where: src/tree/mirror/streaming/backend.rs:396-407 (related: src/tree/typed/untyped.rs:400-421; src/tree/mirror/streaming/backend.rs:262-263)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read untyped.rs:400-421, which already carries the memo paragraph at the typed node)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (written at 206c288a; the same mechanism is documented on the typed node)
- Owner-gated: no

The accessor is generic over `B`, but its doc describes `Local`'s lazy memo ("every branch's bounds memo is forced tree-wide ... shared through the node handles across snapshots"). For a backend keeping `version_bytes` as a stored field, which `Node::version_bytes`'s doc at 262-263 invites, the paragraph is false. Documentation respects the abstraction boundary of the item it documents.

Evidence:

       400	    /// The root node's [`version_bytes`](Node::version_bytes) aggregate
       401	    /// — leaf versions and every interior ceiling and floor — or zero
       402	    /// when empty. The first read materializes it: every branch's
       403	    /// bounds memo is forced tree-wide, `O(#branches)` bound
       404	    /// folds, once per tree lineage — the memos are shared through the
       405	    /// node handles across snapshots, and a mutation invalidates only
       406	    /// its own spine. A fully converged pair pays this once, at
       407	    /// greeting time.

Resolution: Cut to the first sentence plus what the greeting carries; if the greeting-time cost matters to the maintainer, one clause pointing at `typed::Node::version_bytes`. Acceptance: the doc makes no claim that depends on `B = Local`.

### streaming-backend-window-10: `Local::assemble`'s comment asserts the memory story is unchanged without pricing the run-proportional transient it introduces
- Where: src/tree/mirror/streaming/backend/local.rs:195-200 (related: src/tree/mirror/streaming/remote/adapter.rs:56-67; src/tree/mirror/streaming/remote/adapter/decode.rs:356-358; src/peer.rs:351-369; src/tree/typed/node.rs:301-308)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: a run is one supplied node's whole leaf set per adapter.rs:44-54 and encode.rs:185; each buffered entry is a 34-byte prefix padded beside an 8-byte handle, then re-collected at node.rs:301-308 into 40-byte entries; peer.rs:358-360 excludes the replica from the budget)
- Seen by: correctness (as a medium correctness finding); refutation: reframed to low (no committed promise is falsified: the budget's documented scope excludes replica storage, and the transient is a bounded fraction of the replica growth the same run causes); history: deliberate-and-holds for the buffering (88a76a71), but the memory clause was never measured, and the cbor review's memory check (`.agent-notes/2026-08-20-cbor-wire-review/REVIEW.md:976-979`) overlooked this buffer
- Owner-gated: yes: choosing between pricing the transient in prose and rebuilding the bulk path incrementally is the owner's call

The default `Convert::assemble` holds one open radix group per level. The override accumulates every leaf of a maximal same-prefix run before building, so for an early supply or a root-child answer the transient is proportional to the supplied subtree, about 88 bytes of bookkeeping per leaf beyond the handles, reclaimed when the run's node is built. The comment says the memory story is "unchanged" and the decoder's comment says "never a decoded vector of leaves"; both are true at the wire layer and neither quantifies this buffer, and nothing prices it (the supply-decode envelope prices `FAN + 1` leaves per stream). A maintainer reading either comment would conclude no run-proportional state exists.

Evidence:

       195	        // The bulk counterpart of `leaves`: buffer each maximal
       196	        // same-prefix run and build its subtree in one pass, rather than
       197	        // folding it up one virtual level at a time. The buffered run is
       198	        // transient state for a subtree this in-memory backend is about to
       199	        // hold whole anyway, so the streaming session's memory story is
       200	        // unchanged.

Resolution: Either (a) restate the comment with the quantity: the override trades the default chain's fan-per-level bookkeeping for one proportional to the run (state the per-leaf figure as a `size_of` expression, and that it is reclaimed at run end and falls under the budget's "replica itself" exclusion), and add one clause to decode.rs:356-358 saying the backend's own run buffer is the backend's custody; or (b) make the bulk build incremental with a radix-stack builder that emits each compressed subtree when its prefix closes. Acceptance: for (a), both comments describe the run buffer truthfully; for (b), a census-ledger measurement of one large supply run shows peak transient bounded by fan times depth rather than growing with run length.

### streaming-backend-window-24: The flushed-question derivation cites proxy internals by file path from a module that cannot link them
- Where: src/tree/mirror/streaming/window.rs:110-121 (related: window.rs:78-109; src/tree/mirror/streaming/remote/proxy/work/queues.rs:25-42)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read queues.rs:34-36, which defers to the window docs as "the canonical derivation"; `encode.rs` and `pump.rs` are siblings of queues.rs, so from there both premises link as `super::encode`/`super::pump`)
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds for the placement (b76a31f3 wrote the derivation into window.rs as the argument that the edge's window-wide capacity is necessary, and reconciled queues.rs to defer); what remains is the citation form
- Owner-gated: yes: moving the section reverses a recorded placement

The proof of the `ProxyLocalQuestions` occupancy bound names its premises by file path and explains inside the doc why intra-doc links cannot resolve. Cite by stable name, never file path: refactors orphan paths, and a doc that must explain its own citation mechanics is placed one module away from its subject.

Evidence:

       112	//! - the encoder flushes one complete wire reply, then publishes that
       113	//!   reply's entire question batch, before dequeuing its next scope
       114	//!   (`remote/proxy/work/encode.rs`; file paths, not intra-doc links,
       115	//!   because `proxy` is private to `remote` and unresolvable from here);
       116	//! - the decoder dequeues question-first and retires exactly one entry
       117	//!   per decoded reply, in wire order (`remote/proxy/work/pump.rs`);

Resolution: Move the "Sizing the flushed-question edge" section onto `queues::local_questions`, where the premises become resolving links and `Scope` is in scope, and reduce window.rs to one sentence pointing there; invert the pointer at queues.rs:34-36. Acceptance: no file path in window.rs prose; the derivation lives beside `local_questions`.

### streaming-backend-window-27: The public constant's first sentence overstates the budget's scope and names a cfg-gated private constant
- Where: src/tree/mirror/streaming/window.rs:267-275 (related: window.rs:264-265; src/peer.rs:20, 308, 351-369; src/lib.rs:341)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: the re-export chain peer.rs:20 and lib.rs:341 makes `DEFAULT_SYNC_MEMORY_BUDGET` public API; `SCOPE_ENVELOPE_BYTES` is `#[cfg(any(test, feature = "test-internals"))] pub(crate)` at 264-265; `Peer::sync_memory_budget`'s own summary at peer.rs:308 is "Bound the memory a synchronization may spend on pipelining", with exclusions at 351-369)
- Seen by: prose; refutation: confirmed, severity down to low; history: no-rationale-found (written in the 2026-07-24 docs pass; the prior review verified the constant's arithmetic, not its public wording)
- Owner-gated: yes: public rustdoc

The first sentence, "Worst-case memory one synchronization may spend by default", drops the qualifier the method's own summary carries ("on pipelining") and contradicts the method's "What this does not bound" list (wire messages in hand, the replica, observers, other sessions). The doc also points the library user at `SCOPE_ENVELOPE_BYTES`, which the API does not reach. Public rustdoc names nothing the API does not reach, and a first sentence stands alone in a module listing.

Evidence:

       267	/// Worst-case memory one synchronization may spend by default: 512 MiB.
    ...
       272	/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget); the
       273	/// decomposition behind the accuracy band is recorded beside the pinned
       274	/// per-scope envelope (`SCOPE_ENVELOPE_BYTES`).
       275	pub const DEFAULT_SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

Resolution: First sentence: "The default budget for the memory a synchronization may spend on pipelining: 512 MiB." Keep the link to `Peer::sync_memory_budget`. Move the `SCOPE_ENVELOPE_BYTES` pointer into a `//` maintainer comment or drop it (its own doc carries the decomposition). Acceptance: the public first sentence is consistent with the method's exclusions and no cfg-gated or crate-private item is named.

Synthesis note: The `SCOPE_ENVELOPE_BYTES` pointer is also fresh-eyes-3's and api-audit-8's window.rs site.

### streaming-backend-window-33: `pow256`'s safety argument rests on a premise its call sites falsify
- Where: src/tree/mirror/streaming/window.rs:580-584 (related: window.rs:674, 698, 700, 714, 730-731)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: the saturated value is a divisor at 698 `pair / pow256(j)` and 714 `n / pow256(j)`, and a multiplicand at 730 `pow256(j).saturating_mul(fan)` before dividing at 731; the `min` uses at 674 and 700 are the only comparisons)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the premise was already false at d27cb5aa)
- Owner-gated: no

The doc says the saturated value is "only ever compared against" bounded values. The conclusion holds for a different reason: division by the saturated value floors to zero and every caller pads `+ 1`, and `min` with it is the identity, so saturation only narrows an envelope. A safety argument whose premise is false trains the reader to distrust the file's other arguments, which are careful.

Evidence:

       580	/// `256^j`, saturating at `u128::MAX` (only ever compared against values
       581	/// bounded by `u64` inputs, so saturation is always on the safe side).
       582	fn pow256(j: usize) -> u128 {

Resolution: State the operative argument: saturating for `j >= 16`; as a `min` operand saturation is the identity, and as a divisor it floors the mean to zero, which every caller pads with `+ 1`, so saturation only ever narrows an envelope. Acceptance: the premise is true of every use in the file.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| streaming-backend-window-3 | src/tree/mirror/streaming/backend.rs:67 and the sites listed | "seam", "knob", "story"; moralized "honest"/"genuine"/"real"; nine "X, not Y:" openers in window.rs alone. | Reword; keep at most two antithesis openers per module. |
| streaming-backend-window-4 | src/tree/mirror/streaming/backend.rs:115-117 and the sites listed | One hazard stated three times, two phrases twice; a `///` doc on an anonymous `const`. | One home for each phrase; `//` on the const. |
| streaming-backend-window-7 | src/tree/mirror/streaming/backend/local.rs:44 (fourteen partition lines) | Em-dashes in `//` comments. | Crate-wide pattern. |
| streaming-backend-window-17 | src/tree/mirror/streaming/channel/instrumented.rs:16-25 | No module doc; `RoleStats`'s fields, the vocabulary of the capacity assertions, are undocumented. | One line per field and a two-sentence `//!`. |
| streaming-backend-window-21 | src/tree/mirror/streaming/testing.rs:3-4 | The summary describes only `Faulting`'s reply-corruption face, not `Fault::Greeting`. | Cover both variants in the sentence. |
| streaming-backend-window-22 | src/tree/mirror/streaming/testing/failing.rs:198-206 | `Failing<B>` deliberately keeps the default `leaves`/`assemble` chain so injection sees every level, and says so nowhere. | Two-line comment stating the intent. |
| streaming-backend-window-35 | src/tree/mirror/streaming/window.rs:721-727 | `child_slots_quantile`'s doc attributes the evaluation-point slack to the `+ 1` when the two named losses can exceed one. | Match each loss to the term that covers it (the Bernstein slack). |

## Materialized

### materialized-18: The public `Violation`/`Error` rustdoc names reaction kinds the user cannot reach and leaves `Error`'s variants undocumented
- Where: src/tree/mirror/streaming/materialized/error.rs:10-39 (related: src/tree/mirror/streaming/materialized/error.rs:3-7; src/error.rs:41-43; src/tree/mirror/streaming.rs:49, 51)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (src/error.rs:41-43 re-exports both enums; `Reaction` lives in `pub(crate) mod message` and private `mod erased`, streaming.rs:49/51; grep of lib.rs and Cargo.toml finds no `missing_docs` lint; `Error::Backend` and `Error::Violation` carry no doc comment)
- Seen by: prose; refutation: confirmed; history: no rationale found (the variants have been undocumented since 7126e489; the style pass dfd19c44 rewrote `Violation`'s summary but left the `Match`/`Query`/`Supply` vocabulary)
- Owner-gated: no

This file is user-facing: a user arriving from `Error::Mirror` to file a bug report (error.rs's own module doc promises the taxonomy is for matching and bug reports) meets `Match`, `Query`, `Supply`, and "positional", the vocabulary of a `pub(crate)` type defined nowhere in the public docs, and an enum that speaks as "we"/"our". Documentation altitude for public rustdoc: state the contract, name nothing the API does not reach.

Evidence:

    3	pub enum Error<E> {
    4	    #[error(transparent)]
    5	    Backend(#[from] E),
    6	    #[error(transparent)]
    7	    Violation(Violation),

    10	/// The ways a counterparty can misbehave: exactly the semantic faults
    11	/// only this side can detect, because they depend on what we hold (our
    12	/// questions, our tree, and the greeting the peer declared to us).

    28	    /// A positional `Match` after every held child has been answered.
    29	    #[error("reply attempted to match unknown child")]
    30	    UnexpectedMatch,

Resolution: Give `Error::Backend` and `Error::Violation` one-line docs (noting that in `MirrorError` the backend error is `Infallible`). In `Violation`'s enum doc, define the three answer kinds once in plain language (a reply answers each child the question listed, in order, with an agreement, a sub-question, or, for a child the asker lacks, a supplied subtree) and rewrite the variants against those words; replace "we"/"our" with "the receiving replica". Acceptance: `cargo doc` for `rumors::error::MaterializedViolation` reads without a backtick identifier that does not resolve to a public item; every variant of both enums has a doc comment.

### materialized-20: Agent-note roster IDs and process history cited from code: "finding #6", "finding #7", "adjudicated", "the mux adjudication"
- Where: src/tree/mirror/streaming/materialized/progress.rs:82-98 (related: src/tree/mirror/streaming/materialized/progress.rs:93, 114, 208; progress/tests.rs:62, 80, 102, 121; src/tree/mirror/streaming/materialized/transcript.rs:10-11; outside the partition: src/tree/mirror/streaming/tests/capacity.rs:244, 280; tests/local_eq.rs:142, 259; tests/wedge.rs:9)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep `finding #|adjudicat` across src/ and tests/ lists every site; the roster resolves only to `formal/MODEL.md:35-36`, `formal/PROGRESS.md`, and `.agent-notes/2026-07-18-parent-placement/`, none of which code may cite; `B5` is a named axiom, `formal/lean/StreamingMirror/Mux/Causal.lean:20`)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed (correcting the correctness lens: `B5` does resolve, as a Lean axiom, and may be cited by name); history: contradicts the hard rule (the IDs entered with 77674c9c and 4407590b; the 2026-07-23 citation sweeps 07f33b0c and ccdd4c5d removed `MODEL.md` path citations but left the IDs and the adjudication narrative)
- Owner-gated: no

The recorder's docs, its tests, and the transcript module tag their invariants with numbered findings that index the formal campaign's roster, and narrate how a decision was reached ("adjudicated", "promoted to a proptest bridge by the mux adjudication", "Retained as the design-space record"). AGENTS.md's hard rule forbids citing `MODEL.md`/`PROGRESS.md` from code, and an ID that resolves only there is such a citation by proxy; Principle 5 forbids opaque roster IDs and dated rationale at the declaration site. The text beside each tag already names the invariant in plain words (wire contiguity, parent placement, payload independence) and the Lean theorem names beside them (`Control.jam_not_deadlockFree`, `Sched.deadlock_free`, `Sched.deadlock_free_d5`, axiom `B5`) are the sanctioned anchors.

Evidence:

    81	    /// checks and deadlock. Wire contiguity is its wire-stream twin
    82	    /// (finding #6): without it, a wire stream that runs ahead of an

    93	    /// reorder within a channel. Parent placement (finding #7) is the

    208	    /// the adjudicated design decision, with the capacity-universal

    10	//! payloads — promoted to a proptest bridge by the mux adjudication
    11	//! (bridge B5): the announced dispute skeleton must be reconstructible from

Resolution: Delete every "(finding #N)" parenthetical. Rewrite progress.rs:206-212 to state the decision positively ("the walk publishes a scope's parent resolution last, trading any-capacity deadlock freedom for pipelining under the assembler's fan floor; `Sched.deadlock_free_d5` is the theorem for the other placement") without "adjudicated" or "Retained as the design-space record". In transcript.rs:7-13 keep the premise and the axiom name `B5`, drop "promoted to a proptest bridge by the mux adjudication". Sweep the sites outside the partition in the same pass. Acceptance: `grep -rn 'finding #\|adjudicat' src tests` returns nothing; the rewritten paragraphs read as present-tense statements of the discipline and the theorem that backs it.

Synthesis note: Overlaps streaming-tests-8 and prose-hygiene-4 on the out-of-partition sites; this entry's treatment of `B5` (keep, as a Lean axiom) agrees with prose-hygiene-4.

### async-hazards-4: The first greeting on a cold tree hashes and bounds the whole tree inside one poll, stated only in a crate-private doc
- Where: src/tree/mirror/streaming/materialized.rs:513-527 (related: src/tree/mirror/streaming/materialized.rs:569-583, src/tree/mirror/streaming/materialized.rs:501-505, src/tree/mirror/streaming/backend.rs:400-407, src/tree/mirror/streaming/backend/local.rs:136-141, src/peer.rs:710-714, src/rumors.rs:410-416, src/lib.rs:231-236, src/link/routed.rs:37)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `greeting_fan` -> `children_of` -> `Local::children`, which is `stream::iter` over the root's children with no yield point; `fan_listing` calls `node.hash()` per root child and `Root::max_version_bytes` forces the bounds memo; `grep -rn yield_now src/` finds nothing)
- Verification: reframed: the sweep's companion claim about `CausalMessages` is dropped (see Dropped); the greeting half stands; history: deliberate-and-holds for the hiding of `warm_caches` (all four sites say "For benchmark and test calibration only"), no-rationale-found for the absence of any public statement of the greeting cost
- Owner-gated: no for the doc sentence; yes for exposing `warm_caches` (API addition)

Both `connect` and `accept` build the greeting from `fan_listing(&fan)` and
`self.root.max_version_bytes()`. On a tree whose nodes carry empty memos (a
fresh process that rebuilt the set, or a replica that just bootstrapped) the
first of these hashes every subtree and the second folds every branch's
bounds, synchronously, inside the session future's poll: the local backend's
`children` is `stream::iter`, so `greeting_fan(..).await` never yields. The
only statement of this cost is the `pub(crate)` doc on `max_version_bytes`
(backend.rs:402-407). The public contract tells the caller they own the
executor (lib.rs:233-236) and, for routed links, to spawn or select over the
router future (routed.rs:37); on a current-thread executor that router, and
every other task, stalls for the duration of the first greeting's hash of a
large set. `warm_caches`, which pre-pays exactly this, is `#[doc(hidden)]` at
`Peer`, `Rumors`, `Snapshot`, and `Tree`, scoped to benchmarks by its docs.

Evidence:

    src/tree/mirror/streaming/materialized.rs
    513	        let fan = greeting_fan(&self.backend, self.root.root.clone())
    514	            .await
    515	            .map_err(Error::Backend)?;
    ...
    521	            max_version_bytes: self.root.max_version_bytes(),
    ...
    527	            listing: fan_listing(&fan),

    src/tree/mirror/streaming/backend.rs
    402	    /// when empty. The first read materializes it: every branch's
    403	    /// bounds memo is forced tree-wide, `O(#branches)` bound
    404	    /// folds, once per tree lineage — the memos are shared through the
    405	    /// node handles across snapshots, and a mutation invalidates only
    406	    /// its own spine. A fully converged pair pays this once, at
    407	    /// greeting time.

    src/tree/mirror/streaming/backend/local.rs
    136	        let children = stream::iter(
    137	            parent
    138	                .into_children()
    139	                .into_iter()
    140	                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
    141	        );

    src/peer.rs
    710	    /// Force this set's tree to compute its lazy structural memos (observable
    711	    /// hash and ceiling/floor version bounds), so a subsequent operation is
    712	    /// timed against its own work. For benchmark and test calibration only.
    713	    #[doc(hidden)]
    714	    pub fn warm_caches(&self) {

    src/lib.rs
    233	//! Sessions and observers are plain futures and streams, driven entirely by
    234	//! the caller. The I/O traits are Tokio's runtime-independent
    235	//! [`AsyncRead`](tokio::io::AsyncRead) and [`AsyncWrite`](tokio::io::AsyncWrite);
    236	//! no Tokio runtime, spawning, sockets, or timers are required by this crate.

Resolution: Add one sentence to the runtime-independence section (or to
`Rumors::gossip`): the first session on a freshly built replica computes the
tree's hashes and version bounds synchronously inside the greeting, once per
tree lineage and proportional to the set's size, so a large set's first
session occupies its executor thread for that long. Owner call, separately:
whether to make `warm_caches` (or a named equivalent) public so an
application can pre-pay off the latency path; if it stays hidden, the doc
sentence should say the cost is paid at the first session. Acceptance: a
public doc sentence names the first-greeting materialization; the
`warm_caches` decision is recorded either as a public method with user-facing
docs or as an explicit "hidden stays" ruling.

### materialized-21: "The encoder" in the progress recorder means the walk, colliding with the crate's use of the word for the wire codec; "the weave" is model vocabulary never defined in the crate
- Where: src/tree/mirror/streaming/materialized/progress.rs:97 (related: src/tree/mirror/streaming/materialized/progress.rs:122, 127, 196-197, 203, 211-212; progress/tests.rs:96-109; src/tree/mirror/streaming/materialized.rs:450; src/tree/mirror/streaming/window.rs:82, 105, 112; src/tree/mirror/streaming/remote.rs:37; src/tree/mirror/streaming.rs:5-6)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (grep `encoder` across `streaming/`: the partition's ten sites all denote the traced walk; window.rs, remote.rs, backend.rs:163, message.rs:99, and materialized.rs:450 all denote the wire codec's encoder; streaming.rs:5-6 defines "the walk" as the in-process participant; `weave` is defined only in `formal/MODEL.md:37` and `EventDag.lean`)
- Seen by: prose, structure; refutation: confirmed; history: no rationale found (the term is the formal campaign's name for the walk, adopted by 77674c9c/4407590b; nobody weighed it against the codec's "encoder")
- Owner-gated: no

One term, one meaning per crate: a maintainer reading "the encoder's traces" one file away from "the wire encoder enforces each by panic" (backend.rs:163) must resolve the word per sentence. The crate already has a name for the traced participant. "The weave" costs the reader a lookup in a different artifact.

Evidence:

    97	    /// so the encoder's traces pin exactly the discipline the proof

    196	    /// weave's placement) — deliberately NOT wired into `assert_valid`:
    197	    /// the encoder does not and should not satisfy it.

    450	    /// session's encoders on both ends run at the minimum of the two

Resolution: Replace "the encoder" with "the walk" throughout progress.rs and progress/tests.rs (rename the test to `walk_order_violates_parent_early_discipline` and update the citation at progress.rs:212). Replace "the weave's parent-early discipline" with "the parent-early discipline, `d5` in the formal model" (or cite the Lean definition `weaveScope` by name if the owner wants the model term). Acceptance: `grep -n encoder` in the two progress files returns nothing; `weave` is defined at first use or absent.

### materialized-24: "the materialized filter/prune/oracle" names the in-memory oracle with the word this module uses for itself
- Where: src/tree/mirror/streaming/materialized/unknown.rs:11-12 (related: src/tree/mirror/streaming/materialized/unknown/tests.rs:1-3, 67-70; src/tree/mirror/streaming.rs:13; src/tree/traverse/unknown.rs:9)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep `materialized` outside this module: every use denotes the streaming walk or a materialized node; traverse/unknown.rs:9 calls the oracle "the in-memory join")
- Seen by: prose; refutation: confirmed, severity down to low (two files, local context disambiguates on a careful read); history: deliberate but expired (bbe89b94 wrote "Unlike the materialized filter" when no `materialized` module existed; 61ddf3a5 created `streaming/materialized.rs`, inverting the word's referent in this file)
- Owner-gated: no

Inside `streaming::materialized::unknown`, "the materialized filter" means `traverse::unknown`, the in-memory traversal; everywhere else "materialized" is this walk. A term of art must mean one thing in one crate, and the crate already has the unambiguous name.

Evidence:

    11	//! Unlike the materialized filter, which walks one owned subtree, this version
    12	//! is generic over any [`Backend`].

    1	//! The streaming [`unknown`] prune must agree, node for node,
    2	//! with the materialized [`Unknown`](crate::tree::traverse::unknown::Unknown)
    3	//! oracle it mirrors.

Resolution: "Unlike the in-memory filter ([`traverse::unknown`](crate::tree::traverse::unknown)) ..." at unknown.rs:11; "the in-memory [`Unknown`] oracle" at tests.rs:2; "the in-memory prune" at tests.rs:67; rename `agrees_with_materialized_oracle` to `agrees_with_in_memory_oracle`. Acceptance: `grep -n materialized` in the two files returns no line using the word for the `traverse` oracle.

### materialized-25: Ghost reference to the removed height-typed recursion: "exactly as the typed tower did"
- Where: src/tree/mirror/streaming/materialized/unknown.rs:19-23 (related: src/tree/mirror/streaming/erased.rs:30; src/tree/mirror/streaming/window.rs:137; src/tree/typed/height.rs:166)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S"typed tower" -- src/` returns only bf1a5b4b, "streaming: erase the materialized walk's workers", whose message reads "boxed per step exactly as the typed tower was"; grep finds no other occurrence in src/)
- Seen by: structure, prose, perfapi; refutation: confirmed; history: contradicts the hard rule (a ghost at birth: written by the commit that deleted the tower, after the 44724ad0 prose sweep, so no sweep has caught it)
- Owner-gated: no

The module doc justifies boxing each recursive future by comparison with an implementation that no longer exists, in the past tense. The same sentence hand-writes the depth bound ("at most 32") where a named constant exists (`KEY_DEPTH` in window.rs:137; `Root::HEIGHT` pinned at height.rs:166) and closes with a moralizer. The sibling "exactly as before" at erased.rs:30 is the same tell from the same commit, outside this partition.

Evidence:

    19	//! would instantiate one per level. Each recursive call boxes its future
    20	//! ([`BoxFuture`]) exactly as the typed tower did — the type stays flat —
    21	//! and the depth is bounded by the prefix's remaining height, at most 32,
    22	//! so the recursion is stack-safe by construction rather than by input
    23	//! goodwill.

Resolution: "Each recursive call boxes its future ([`BoxFuture`]) so the future type stays finite, and the depth is bounded by the prefix's remaining height (the key depth), so the recursion is stack-safe by construction." Fix erased.rs:30 in the same pass. Acceptance: `git grep 'typed tower\|as before' src/tree/mirror/streaming/` returns nothing; the paragraph describes what is.

Synthesis note: Sibling of mirror-common-23 ("exactly as before"), from the same commit; one pass fixes both.

### materialized-29: Four production files lack module docs; `work.rs` names two of its five children; `Resolver`'s methods are undocumented
- Where: src/tree/mirror/streaming/materialized/work.rs:3-5 (related: src/tree/mirror/streaming/materialized/work.rs:20-24; work/answer.rs:1; work/resolver.rs:1, 45, 111, 115, 119; src/tree/mirror/streaming/materialized/common.rs:1; src/tree/mirror/streaming/materialized/error.rs:1)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read each file's opening: answer.rs, resolver.rs, and common.rs open with `use`; error.rs opens with an item doc; work.rs:3-5 links `levels` and `assembly` while declaring five children at 20-24; `Resolver::new`, `ready`, `pending`, and `finish` carry no doc comment)
- Seen by: prose; refutation: confirmed; history: no rationale found (unchanged since 7126e489; the work.rs map dates from the cbfe1aff split when `answer` and `resolver` already existed)
- Owner-gated: no

A module doc's first sentence is what a maintainer sees in the listing; `work.rs`, the natural entry point, does not map `answer`, `queues`, or `resolver`. `finish` is where `UnfinishedReply` is raised and says nothing.

Evidence:

    3	//! [`Work`] owns every independently runnable pump while the type-level walk
    4	//! advances. [`levels`] contains the phase-specific walks, while [`assembly`]
    5	//! reconstructs their resolved scopes upward.

    1	use itertools::{EitherOrBoth, Itertools};

    119	    pub fn finish(mut self) -> Result<Resolution<B::Erased>, Error<B::Error>> {

Resolution: Add one-sentence `//!` docs to answer.rs ("The per-question answers: merge-joins of both listings at internal, leaf-parent, and leaf heights, where disputes and shed leaves are counted"), resolver.rs, and error.rs (common.rs dissolves under materialized-16; if it stays, document it and the cfg split's reason: the test receiver is itself a `Stream`, channel.rs:3-6). Extend work.rs:3-7 to name all five children. Doc `Resolver::new`, `ready`, `pending`, and `finish` (with the `UnfinishedReply` condition). Acceptance: every non-test `.rs` in the partition opens with a `//!` block; work.rs links all five submodules; every `pub fn` in resolver.rs has a doc comment.

### materialized-32: The full-fan and one-slot capacity arguments are each stated in more than one place
- Where: src/tree/mirror/streaming/materialized/work/assembly.rs:29-31 (related: src/tree/mirror/streaming/materialized.rs:79-87; work/queues.rs:3-5, 41-45, 60-83; work.rs:130-133)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read all five sites)
- Seen by: prose; refutation: confirmed; history: no rationale found (accretion across 7126e489, c9b8f38b, 4d55d484, cbfe1aff)
- Owner-gated: no

The full-fan argument appears in the module doc, the constructor doc, and `Work::assemble`'s doc; the one-buffered-response argument appears at `pump` and at `outgoing_responses`. `queues.rs:3-5` declares the constructors the home of capacity reasoning. Three copies of one argument drift independently. The module-doc copy is arguably deliberate (the deadlock argument's home, and the fan queue is its one exception), so it stays as a summary with a link.

Evidence:

    29	    /// A full fan lets every lower scope enqueue before the parent resolution
    30	    /// containing its [`Resolve::Pending`] slots is published, without relying
    31	    /// on blocked sender futures remaining independently runnable.

    3	//! Each function names one edge in the protocol dataflow. Keeping capacity
    4	//! choices here makes them reviewable alongside the exact item type and keeps
    5	//! queue arithmetic out of the walk itself.

Resolution: Keep the full arguments at `assembly_level_returns` and `outgoing_responses`; reduce assembly.rs:29-31 and work.rs:130-133 to one sentence each ending in a link to the constructor; end the module-doc paragraph at materialized.rs:79-87 with the same link. Acceptance: each mechanism ("full fan", "one buffered response") is spelled out at exactly one constructor; the other sites link there.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| materialized-5 | src/tree/mirror/streaming/materialized.rs:253-255 | `Query`'s summary sentence describes the queue, not the item. | "A pending question, resolved by one remote reply: one item of the pairing queue ...". |
| materialized-7 | src/tree/mirror/streaming/materialized.rs:358-359 | `Descending`'s doc links the erased `Reply` under the typed spelling `Reply<B, H>`. | Match the link text to the erased type's arity. |
| module-graph-15 | src/tree/mirror/streaming/materialized/common.rs:1 (27 files) | Twenty-seven non-test files open without a `//!` module doc. | One-sentence `//!` for the multi-item files; leave the single-type files. |
| materialized-19 | src/tree/mirror/streaming/materialized/progress.rs:58, 94, 117, 122-124; progress/tests.rs:23 | "Seven checks:", a one-off name for a named invariant, two glosses for `d6`, and a check cited by quoting another file's comment. | Four edits as the entry lists. |
| materialized-23 | src/tree/mirror/streaming/materialized/tests.rs:1-8 and the sites listed | "chokepoint", moralizers, a jokey banner, a next-line narration, a compressed cross-reference. | Reword; delete the banner and the narration. |
| async-hazards-5 | src/tree/mirror/streaming/materialized/work/levels.rs:399, 447-448 | Dropped early hand-off senders are read as empty without the argument that makes that safe. | One comment at the first site; the second points to it. |
| materialized-38 | work/resolver.rs:86; unknown.rs:112; work/tests/violations.rs:185-186 | Em-dashes in `//` comments. | Crate-wide pattern. |

## Streaming tests (src/tree/mirror/streaming/tests)

### streaming-tests-5: Two doc comments in tests.rs describe code that no longer exists
- Where: src/tree/mirror/streaming/tests.rs:99-105 (related: src/tree/mirror/streaming/tests.rs:176-180)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log -L99,105` lists 97dfbcdf (doc and `<T: Send + Sync + 'static>` added together), b524e406, 48bc31df (signature erased, doc unchanged); `git log -L176,180` lists only a8b130e6; `git grep -w 'Uncertain\|Closing' a8b130e6 -- src/tree/mirror/streaming` hits `convert.rs:18,230,235,241`; a word-boundary grep for `uncertain\|Closing` over streaming production code at HEAD returns nothing)
- Seen by: structure-prose ([0], [1]), api-economics ([39]); refutation: confirmed both; history: deliberate-but-expired for both. Attribution correction from the history pass: the `uncertain`/`Closing`/`Complete` vocabulary at 179-180 was streaming's own message vocabulary when written (a8b130e6, 2026-07-09) and was collapsed to `Reply` by 7126e489 (2026-07-14), seven weeks before the alternating protocol retired; it is not V1 leakage
- Owner-gated: no

Two paragraphs describe mechanisms the code no longer has. The doc on `transcribed_mirror_sides` (99-105) justifies a type parameter the function lost in the payload-erasure commit, and misleads about why payload twins work (they work because `Root` carries erased payloads, not because of generics). The doc on `converges_on_leaf_parent_dispute` (176-180) explains the leaf-parent merge in terms of `uncertain`, `Closing`, and `Complete`, none of which streaming code speaks today. AGENTS.md's hard rule: nothing refers to code that no longer exists; a testdoc held to accuracy is a bug when inaccurate. Two prose sweeps whose stated goal covered the second site (44724ad0 and the V1-retirement re-denomination) missed it.

Evidence:

       102	/// The skeleton-bridge harness ([`skeleton`]): generic over the payload type
       103	/// so payload-perturbation twins (same paths, different contents) can run
       104	/// through the identical machinery.
       105	fn transcribed_mirror_sides(a: Root, b: Root) -> (Root, Root, Trace, Transcript) {

       179	/// The responder's closing `uncertain` lists its leaves, and the leaf-height
       180	/// `Closing`/`Complete` words carry the difference in both directions.

Resolution: 102-104: "The skeleton-bridge harness ([`skeleton`]). Payload twins ([`announced`]) differ only in leaf contents, which the erased `Root` carries opaquely, so both run through identical machinery." 179-180: delete (the invariant sentence at 176-177 stands alone) or restate against today's code: the leaf-parent answerer merge-joins both leaf listings, supplying its exclusive leaves and issuing a leaf request for each it lacks. Acceptance: `grep -n 'generic over the payload\|uncertain\|Closing' src/tree/mirror/streaming/tests.rs` is empty.

Synthesis note: The second site (tests.rs:179-180) is also named by tests-wire-format-6 as its out-of-partition sibling.

### streaming-tests-8: Campaign roster IDs and default-dialect vocabulary in the bridge and probe docs
- Where: src/tree/mirror/streaming/tests/announced.rs:1-2 (related: announced.rs:41; skeleton.rs:18-19, 21, 506; wedge.rs:1, 4, 9, 118; local_eq.rs:1, 12-13, 142, 259; capacity.rs:244, 280; fixtures.rs:31; faults.rs:61; outside the partition: materialized/transcript.rs:10-11, materialized/progress.rs:82, 93, 114, 208, materialized/progress/tests.rs:62, 80, 102, 121)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep enumerates every site; `grep -rnE '^(theorem|def|lemma|abbrev)\s+(B5|T3|F4)\b' formal/lean` returns nothing, so none is a Lean declaration name; `finding #7` resolves to formal/MODEL.md:36, 41, 507, 636 and formal/PROGRESS.md:981; `F4` resolves only to `.agent-notes/`; `B5` is defined in formal/doc/exposition.typ:1132 ("the fifth of the campaign's numbered Rust bridge tests") and named in Lean docstrings)
- Seen by: structure-prose ([3]), blind-spots ([31]), api-economics ([37], [51]); refutation: confirmed; history: no rationale found (ad89a63e removed the document pointers and kept the tags, introducing "charter" and "adjudicated" as the restatement; b204fe56 and ccdd4c5d normalized the Lean citations in local_eq.rs and left the tags)
- Owner-gated: no

The bridge suites identify what they test by tags from the mux campaign roster: `B5`, `T3`, `F4`, `finding #7`, the ordinals `Bridge 1/2/3`, and the process words `adjudicated`/`adjudication` and `charter`. None is a Lean theorem or definition name (the sanctioned citation form), `finding #7` resolves only in formal/MODEL.md (which AGENTS.md forbids citing from code) and an agent note, and `F4` resolves nowhere in the tree. The Lean names beside them (`wc_impossibility`, `Mux.wedge`, `viewEnc`, `LocalEq`) already carry the meaning. The same files carry the dialect tells the owner names: `knobs` (fixtures.rs:31), `genuine`/`genuinely` (faults.rs:61, local_eq.rs:12-13), and the colon-fronted fragment "Deviations from the Lean, recorded:" (skeleton.rs:21).

Evidence:

    announced.rs:
         1	//! Bridge 3: announced-skeleton reconstruction — the payload-independence
         2	//! bridge B5.

    wedge.rs:
         4	//! The mux impossibility theorem T3 (`wc_impossibility`) quantifies over one
         9	//! trees whose dispute skeleton IS the wedge (adjudication repair F4: for
        10	//! impossibilities, realizability flows from Rust to the model).

    capacity.rs:
       244	/// Probe (model finding #7): a lone parent scope stalls the real encoder

    skeleton.rs:
        18	//!   reconstruction bridge B5 asks for: payload-erased frame contents
        19	//!   determine the announced skeleton, the fact charter locality rests on.

Resolution: Name the thing at each site and keep the Lean names: "the payload-independence bridge (announced-skeleton reconstruction)" for B5; "the mux impossibility theorem `wc_impossibility`" for T3; drop "adjudication repair F4" and say "for an impossibility, realizability flows from Rust to the model"; "Parent-placement probe" for "finding #7"; drop the `Bridge N` ordinals (each file's first sentence already names its bridge); "the fact the locality theorems rest on" for "charter locality"; "the model's" for "adjudicated"; "parameters" for "knobs"; "A malformed reply" for "A genuine malformed reply"; "held"/"absent" for "genuinely held"/"genuinely absent". Sweep the out-of-partition sites in the same pass so production and tests keep one vocabulary. Acceptance: `grep -rnE '\b(B5|T3|F4)\b|Bridge [0-9]|finding #[0-9]|adjudicat|charter|knob|genuine' src/tree/mirror/streaming` is empty; every formal citation is a Lean theorem or definition name.

Synthesis note: prose-hygiene-4 narrows this entry: `B5` is a Lean axiom (`axiom B5`, Mux/Causal.lean:20) and "charter" a Lean-anchored term (Charters.lean, `c1_charter`), so the substitutions for "bridge B5" and "charter locality" are optional; every other substitution here stands, and materialized-20 already keeps `B5`.

### streaming-tests-27: The wedge.rs module doc cites seed-file paths that do not exist, inside a decision-history paragraph
- Where: src/tree/mirror/streaming/tests/wedge.rs:12-18 (related: formal/lean/StreamingMirror/Mux/Instances.lean:49-55 (outside the partition) repeats the narrative)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git ls-files | grep proptest-regressions` lists `proptest-regressions/pairwise.txt` and `proptest-regressions/shadow_validity.txt` and no `tests/*.proptest-regressions`; `ls tests/*.proptest-regressions` matches nothing; `git ls-tree 97dfbcdf tests/` shows both cited paths existed when the doc was written)
- Seen by: structure-prose ([2]), blind-spots ([29]), api-economics ([38]); refutation: confirmed; history: deliberate-but-expired (the paths were real at 97dfbcdf; ce3664dd, 2026-09-01, centralized every sibling seed under `proptest-regressions/`, and its sweep list did not include wedge.rs; the paragraph's presence was deliberately reworded in 2921784c, so deleting the whole paragraph revisits an LLM-authored prose choice, not an owner ruling)
- Owner-gated: no

The paragraph names `tests/pairwise.proptest-regressions` and `tests/shadow_validity.proptest-regressions`. Neither is tracked; the seeds live at `proptest-regressions/pairwise.txt` and `proptest-regressions/shadow_validity.txt`, and AGENTS.md says `tests/seed_liveness.rs` fails exactly that sibling layout, so the doc names a layout the gate forbids, written the same day the guidance landed. The rest of the paragraph is a design-history argument ("This bridge therefore constructs...") rather than a statement of what the module is.

Evidence:

        12	//! On the committed seeds: `tests/pairwise.proptest-regressions` and
        13	//! `tests/shadow_validity.proptest-regressions` are integration-level
        14	//! seeds (three-peer networks, version-addressed leaves, whole-`Rumors`
        15	//! action lists) that realize the wedge's *jam mechanism*, not its
        16	//! byte-exact shape; a structural equality pin needs hand-placed paths.
        17	//! This bridge therefore constructs the pair deterministically and pins
        18	//! the decoded skeleton to the literal.

Resolution: Replace 12-18 with one present-tense sentence without paths: "The pair is hand-placed: a structural-equality pin needs exact paths, which content-addressed generators cannot supply." If the seed observation must survive, it belongs in `.agent-notes/`. Instances.lean:49-55 carries the same narrative and should be trimmed in the same pass (outside this partition). Acceptance: `grep -rn 'proptest-regressions' src/tree/mirror/streaming/tests/wedge.rs` is empty.

### streaming-tests-1: The tests.rs module doc maps three of its eight submodules
- Where: src/tree/mirror/streaming/tests.rs:1-5 (related: src/tree/mirror/streaming/tests.rs:27-34)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); the history pass verified via `git log -L1,5` that the doc dates from ddc9a2d8, when the three named modules were the only submodules
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (complete when written; five modules added later in 97dfbcdf and 7626543e without updating it)
- Owner-gated: no

The module doc enumerates `capacity`, `faults`, and `fixtures` and omits `announced`, `local_eq`, `skeleton`, `stats`, and `wedge`. A hand-maintained map the code can change without touching is the enumeration Principle 5 forbids; here it is already stale, and the five omitted modules are the ones a newcomer most needs orientation for.

Evidence:

         3	//! Capacity/scheduling stress lives in [`capacity`], connected abort and
         4	//! lifecycle checks in [`faults`], and deterministic tree builders in
         5	//! [`fixtures`].
        27	mod announced;
        28	mod capacity;
        29	mod faults;
        30	mod fixtures;
        31	mod local_eq;
        32	mod skeleton;
        33	mod stats;
        34	mod wedge;

Resolution: Either complete the map with one clause per module (`stats` pins the counters against a dispute oracle; `skeleton`, `wedge`, `local_eq`, and `announced` bridge sessions to the Lean model) or drop the enumeration and let each submodule's own first sentence serve, since the module listing already shows them. Acceptance: every `mod` at tests.rs:27-34 is named in the doc, or the doc enumerates none.

### streaming-tests-9: The announced.rs module doc records a build-time observation as narrative
- Where: src/tree/mirror/streaming/tests/announced.rs:26-32 (related: skeleton.rs:641-648; src/tree/mirror/streaming/materialized.rs:841-850)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (cat of `proptest-regressions/tree/mirror/streaming/tests/announced.txt` shows two `cc` lines; read materialized.rs:841-850: `tokio::select!` with no `biased;`)
- Seen by: structure-prose ([15]), api-economics ([49]); refutation: confirmed; history: deliberate-and-holds for the mechanism (97dfbcdf recorded the finding here on purpose, and the unbiased select is still true); the parenthetical is a trim compatible with that decision
- Owner-gated: no

The scoping rationale ends with a diary entry ("observed while building this bridge: the committed regression seed reorders..."). The durable fact is one sentence: the claim is per channel because `complete_initiator`'s terminal `tokio::select!` is unbiased, so the global order varies run to run. The history of how that was found belongs in git (where 97dfbcdf's message already records it), "the committed regression seed" (singular) disagrees with the two seeds in announced.txt, and skeleton.rs:641-648 explains the same mechanism a second time, so the two copies can drift.

Evidence:

        26	//! adversarially, and the real global publication order is not even a
        27	//! function of the trees — `complete_initiator`'s terminal `tokio::select!`
        28	//! is unbiased, so its branch order draws tokio's thread-local RNG and
        29	//! reorders the tail of otherwise identical back-to-back runs (observed
        30	//! while building this bridge: the committed regression seed reorders the
        31	//! final absorb-side events between two runs of the SAME trees; the
        32	//! per-channel projections are unaffected).

Resolution: Cut from "(observed" through "unaffected)". Keep the mechanism once, at `trace_channels` (skeleton.rs:641-648), and have announced.rs say "the claim is per channel, not the global interleaving ([`trace_channels`] explains why)". Acceptance: no "observed while" phrasing; one explanation of the unbiased `select!` in the partition.

### streaming-tests-13: Two testdocs understate or mislabel what their bodies check
- Where: src/tree/mirror/streaming/tests/capacity.rs:113-115 (related: capacity.rs:154-168, 363-366, 375-383; src/tree/mirror/streaming/channel.rs:31-33)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read the two bodies; `QueueKind::ALL` is documented "Every materialized semantic edge, for its coverage assertions." at channel.rs:31 and excludes the proxy kinds)
- Seen by: structure-prose ([21]); refutation: confirmed; history: no rationale found (both docs are verbatim from ddc9a2d8; the oracle comparison at 383 existed from the start)
- Owner-gated: no

`capacity_stress_covers_every_queue_role` says "Every named, height-carrying queue" while the body iterates `QueueKind::ALL` (every materialized semantic edge, proxy kinds excluded) and additionally asserts that typed heights survive observation (154-162) and that some sender saw backpressure (163-168), neither mentioned. `scheduled_structured_disputes_match_oracle`'s doc says the disputes "terminate" while the body asserts equality with `join_oracle` (the stronger claim its name states). A doc narrower than its body hides coverage a future edit may drop; AGENTS.md holds the testdoc to the behavior and invariant the test protects.

Evidence:

       113	/// Every named, height-carrying queue is exercised at its documented capacity.

       363	    /// Structured disputes terminate under independently shrinkable channel
       364	    /// and Local-backend poll schedules.
       383	        prop_assert_eq!(actual, expected);

Resolution: 113: "Every materialized queue role is constructed, carries traffic, and runs at its documented capacity; recursive roles keep their typed heights, and the scheduled run applies backpressure." 363-364: "Structured disputes converge to the join oracle under independently shrinkable channel and Local-backend poll schedules." Acceptance: each sentence in the two docs corresponds to an assertion in the body and vice versa.

### prose-hygiene-4: Formal-effort roster tags with no Lean home cited from streaming tests and the progress checker
- Where: src/tree/mirror/streaming/tests/wedge.rs:1-10 (related: src/tree/mirror/streaming/tests/wedge.rs:118; src/tree/mirror/streaming/tests/local_eq.rs:1; src/tree/mirror/streaming/tests/announced.rs:1; src/tree/mirror/streaming/tests/capacity.rs:244, 280; src/tree/mirror/streaming/materialized/progress.rs:82, 93, 114, 199; src/tree/mirror/streaming/materialized/progress/tests.rs:62, 80, 102, 121)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (each site read with line numbers; `grep -rnwE 'F4' formal` returns nothing; `Bridge [123]` matches nothing in formal/; `T3` appears in Lean only inside docstrings beside `wc_impossibility`; `D5` appears as an axiom label in formal/MODEL.md:504 and as the field `d5` in formal/lean/EventDag.lean:293-299; "finding #6/#7" appear in formal/MODEL.md:35-36 as dated finding-log entries)
- Verification: reframed and narrowed: `B5` is a Lean axiom (`axiom B5` at formal/lean/StreamingMirror/Mux/Causal.lean:20, "bridge axiom B5" at :162) and "charter" is a Lean-anchored term (formal/lean/StreamingMirror/Mux/Charters.lean; `c1_charter` and "charter-local" in Statement.lean:24-25), so the announced.rs:2, skeleton.rs:18-19, skeleton.rs:506, and transcript.rs:11 citations are in the permitted form and drop out; what remains is F4, T3, D5, "finding #6/#7", and "Bridge 1/2/3"; history: no-rationale-found
- Owner-gated: no

The remaining tags resolve to nothing code may cite: `F4` has no home
anywhere under formal/; "Bridge 1/2/3" is a numbering that exists only in
these three module docs; "finding #6" and "finding #7" index MODEL.md's dated
finding log; `T3` is a docstring label whose Lean name (`wc_impossibility`)
already rides beside it; "D5 as stated" names MODEL.md's axiom label where
the Lean field is `d5`.

Evidence:

    src/tree/mirror/streaming/tests/wedge.rs
    1	//! Bridge 1: wedge realizability — real trees produce the Lean witness's
    4	//! The mux impossibility theorem T3 (`wc_impossibility`) quantifies over one
    9	//! trees whose dispute skeleton IS the wedge (adjudication repair F4: for

    src/tree/mirror/streaming/materialized/progress.rs
    82	    /// (finding #6): without it, a wire stream that runs ahead of an
    114	    /// The parent-placement check (finding #7): a parent resolution is
    199	    /// D5 as stated: once the resolution of a scope's last disputed child

    src/tree/mirror/streaming/tests/capacity.rs
    244	/// Probe (model finding #7): a lone parent scope stalls the real encoder

Resolution: Drop `T3` and keep `wc_impossibility`; drop "adjudication repair
F4" and keep the clause it introduces ("for impossibilities, realizability
flows from Rust to the model"); "finding #6" and "finding #7" become "sibling
contiguity" and "parent placement", which the surrounding text already
calls them; "Bridge 1/2/3" module docs become titles ("Wedge realizability",
"`LocalEq` soundness", "Announced-skeleton reconstruction"); "D5 as stated"
becomes "the `d5` placement as the model states it". Acceptance: `grep -rnE
'\bF4\b|\bT3\b|\bD5\b|finding #[0-9]|Bridge [0-9]' src` returns nothing.

Synthesis note: Narrower than streaming-tests-8 and materialized-20 on the same sites; where they disagree (`B5`, "charter"), this entry's grep of `formal/lean` is the evidence.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| streaming-tests-10 | src/tree/mirror/streaming/tests/announced.rs:81-85 and the sites listed | The reply capture at `Work::respond` is called a "wire transcript"; on a link, frame counts depend on payload size. | "reply transcript"; note that link-level framing is out of the bridge's scope. |

## Remote codec

### remote-codec-1: Hand-maintained counts and rosters restate constants and tests
- Where: src/tree/mirror/streaming/remote/codec.rs:18-27 (related: src/tree/mirror/streaming/remote/codec/signal/tests.rs:9-10 and :146; src/tree/mirror/streaming/remote/codec/budget/tests.rs:3-4; src/tree/mirror/streaming/remote/codec/frame.rs:424-426; src/tree/mirror/streaming/remote/codec/decode/tests.rs:798-806; outside the partition, src/tree/mirror/streaming/remote.rs:13 and :18)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep `162\b|163\b` over src hits only remote.rs:18, codec.rs:25-26, and `VALID_PLACEMENTS` at signal/tests.rs:7; every cited line read)
- Seen by: structure (15), prose (18, 31); refutation: confirmed; history: contradicts CLAUDE.md Principle 5 doctrine; the module-doc counts were written by 3327a92b and are currently accurate
- Owner-gated: no

The module doc states the stream count, the state count, and the two per-speaker placement totals as literals; two testdocs restate the state and stream counts and one the radix fan; `ListingBuilder`'s doc enumerates its three callers; and the doc of `supply_truncation_at_chunk_boundaries_is_typed` re-lists the cut offsets that `chunk_boundary_cuts` centralizes "so the boundary roster cannot drift". Each is a copy of a fact the code can change without touching the prose, and each already has a mechanically enforced home: `Stream::COUNT`, `Signal::STATE_COUNT`, `FAN`, `VALID_PLACEMENTS` asserted by `placements_match_the_phase_schedule_exhaustively`, and `chunk_boundary_cuts` itself.

Evidence:

    18	//! A frame opens with two unsigned ints: the index of the logical stream
    19	//! it rides (one of 17) and its signal's state code (one of ten frame
    20	//! states — four reaction forms, each continuing or ending its reply,
    ...
    25	//! select a phase-specific subset of states: the initiator admits 162
    26	//! placements and the responder 163, rejecting the rest before their
    27	//! frame body is read.

    signal/tests.rs:9	/// The state roster is a bijection between the ten signals and the state
    signal/tests.rs:10	/// codes 0 through 9; every other code is reserved.
    signal/tests.rs:146	/// Both elected speakers map their 17 stream indices bijectively to schedule heights.
    frame.rs:425	/// greeting's root-fan listing — and whichever reader drives it (the
    frame.rs:426	/// async decoder, the sync oracle, or the slice parser). The map's
    decode/tests.rs:801	/// The seeded offsets are one byte short of, exactly on, and one byte
    decode/tests.rs:802	/// past each payload chunk boundary, plus the zero-byte, one-byte, and
    decode/tests.rs:803	/// one-short-of-total cuts. The chunked body read preserves the typed

Resolution: In `codec.rs:18-27`, drop "(one of 17)" and "one of ten" (the structure is already stated) and replace "the initiator admits 162 placements and the responder 163" with "each speaker admits a phase-specific subset, pinned by `placements_match_the_phase_schedule_exhaustively` and `invalid_placement_snapshot`". In `signal/tests.rs:9-10` and `:146` cite `Signal::STATE_COUNT` and `Stream::COUNT`; in `budget/tests.rs:3` write "`FAN` full-fan query frames"; in `frame.rs:424-426` say "whichever reader drives it" without the roster; in `decode/tests.rs:798-806` say "every cut in `chunk_boundary_cuts`'s roster" and repeat none of the offsets (the first sentence, "cuts at every seeded offset all classify", also wants a rewrite). Apply the same edit at `remote.rs:13` and `:18` (outside this partition). Acceptance: `grep -n "162\|163" src/tree/mirror/streaming/remote` hits only `VALID_PLACEMENTS`; no literal 17, ten, or 256 remains in codec prose outside the enforcing constant or test; the truncation testdoc names the helper and none of its offsets.

Synthesis note: Overlaps remote-capture-atlas-1 and remote-adapter-streams-17 on the literal counts.

### remote-codec-2: "trust boundary" and "honest encoder" in an honest-peer model, plus smaller register tells
- Where: src/tree/mirror/streaming/remote/codec.rs:58-61 (related: src/tree/mirror/streaming/remote/codec/encode.rs:82; src/tree/mirror/streaming/remote/codec/frame.rs:515; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:434-436; codec.rs:29; src/tree/mirror/streaming/remote/codec/budget/tests.rs:74 and :89; src/tree/mirror/streaming/remote/codec/decode/tests.rs:804)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep "trust boundary|honest encoder" over src returns exactly the four codec sites; the smaller tells located by reading)
- Seen by: prose (29); refutation: confirmed; history: contradicts the AGENTS.md hard rule (a59dc786); codec.rs:60 predates the rule, the other three sites postdate it, and ed0f1775's own message uses the calibrated phrase "conformance-bug detector"
- Owner-gated: no

Three docs call decoding "the trust boundary" (and the encoder "not a trust boundary") and one comment reasons about "an honest encoder", in a crate whose model of record is authenticated honest peers and whose fail-fast machinery is a conformance-bug detector, not a security boundary. `budget.rs:28` already has the calibrated wording ("a conformance-buggy peer"). Smaller tells: "orthogonal" for independent, "real bound" and "genuinely binds" as moralizers, "at every seam" as a promoted metaphor.

Evidence:

    58	//! Encoding trusts the protocol and adapter to produce phase-correct,
    59	//! canonically ordered frames; it performs no redundant semantic validation.
    60	//! Decoding is the trust boundary and validates every peer-controlled signal,

    encode.rs:82	/// The encoder is not a trust boundary: phase placement, query ordering, and
    frame.rs:515	/// The encoder is not a trust boundary — callers guarantee canonical
    async_io.rs:434	            // The frame outsizes the budget the peer's encoder flushes
    async_io.rs:435	            // within, so the one shape an honest encoder can still have
    budget/tests.rs:74	/// maximum is refused, so the saturated budget is a real bound, not a
    budget/tests.rs:89	    // Negative control: the ceiling genuinely binds.

Resolution: "Decoding is where conformance is checked" / "The encoder performs no conformance checks" / "a conforming encoder"; "independent" for "orthogonal"; drop "real" and "genuinely"; "at every chunk boundary" for "at every seam". Acceptance: `grep -rn "trust boundary\|honest encoder" src/tree/mirror/streaming/remote/codec` returns nothing; the other sites read in plain terms.

### remote-codec-4: "pre-batching" names a wire state that no longer exists
- Where: src/tree/mirror/streaming/remote/codec/budget.rs:108-110 (related: none)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: the partition's only occurrence; `git log -S"pre-batching"` on the file names only f94f2056, the commit that introduced batching)
- Seen by: prose (19); refutation: confirmed; history: contradicts the AGENTS.md hard rule; the phrase predates the rule (a59dc786) and no sweep caught it
- Owner-gated: no

`RunBudget`'s doc describes a zero budget by comparison to the wire before batching landed. The present-tense fact is one leaf per frame; the comparison is history the tree should not carry.

Evidence:

    108	/// [`admits`](Self::admits). Any value, including zero, is safe: the
    109	/// minimum-one-record rule keeps every leaf shippable, degrading a zero
    110	/// budget to the pre-batching one-leaf-per-frame wire traffic, and the

Resolution: "degrading a zero budget to one leaf per frame, and the". Acceptance: `grep -rn pre-batching src` returns nothing.

### api-audit-7: The public error taxonomy has undocumented variants, fields, and methods, and no lint guards against it
- Where: src/tree/mirror/streaming/remote/codec/error.rs:40-53 (related: src/tree/mirror/streaming/remote/codec/error.rs:12-27, src/tree/mirror/streaming/remote/codec/error.rs:56-61, src/tree/mirror/streaming/remote/codec/error.rs:63-76, src/tree/mirror/streaming/remote/codec/error.rs:78-85, src/tree/mirror/streaming/remote/codec/error.rs:103-109, src/tree/mirror/streaming/remote/codec/error.rs:111-159, src/tree/mirror/streaming/remote/codec/error.rs:161-168, src/tree/mirror/streaming/remote/codec/signal.rs:105-117, src/tree/mirror/streaming/remote/codec/signal.rs:137-154, src/tree/mirror/streaming/remote/codec/signal.rs:396-397, src/tree/mirror/streaming/materialized/error.rs:1-8, src/tree/mirror/streaming/remote/streams.rs:237-257, src/tree/mirror/streaming/remote/streams.rs:264-295, src/tree/mirror/streaming/remote/streams.rs:805-826, src/error.rs:72-80, src/error.rs:231-233, src/tree/mirror/handshake.rs:37, src/tree/mirror/handshake.rs:178, src/lib.rs:295-302, Cargo.toml:83-107)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the listed files; lib.rs:295-302 carries only `forbid(unsafe_code)`, `doc_auto_cfg`, and `deny(clippy::large_futures)`; `grep -rn 'missing_docs\|missing_debug' src Cargo.toml justfile .cargo .config .github` returns nothing; `tools/doclint` checks summary length and `include_str!` separation only)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Items rendered under `rumors::error` lack doc comments on many public
members: every `FramePart` variant, the `Origin::Stream` fields,
`QueryOrderError`'s fields, `EncodeErrorKind::{Write, Flush,
SupplyTooLarge}`, `CodecEncodeError`'s fields, `DecodeLeafError::{Version,
Message}`, the `Read`, `InvalidSignal`, `Truncated`, `QueryOutOfOrder`,
`InvalidRun`, `OverbatchedRun`, and `TrailingBytes` variants of
`CodecDecodeErrorKind` and most of their fields, `CodecDecodeError`'s
fields, `MaterializedError::{Backend, Violation}`, `Speaker::{Initiator,
Responder}`, four of five `StreamClass` variants,
`DecodeSignalError::Placement`, the `origin`/`source` fields throughout
`SendError`, `StreamError`, and `AcceptError`, and at the top level
`Error::MagicMismatch { remote_magic: [u8; 6] }`, `VersionMismatch`'s two
fields, and `IntentInvalid { byte }`. The `6` is a magic number whose named
constant (`MISMATCH_PREVIEW_LEN`) is private, so the field's meaning is
unrecoverable from the public page. No `missing_docs` lint exists anywhere
in the tree, so the gate cannot notice.

Evidence:

    src/tree/mirror/streaming/remote/codec/error.rs
    40	/// The absent or malformed component of a frame.
    41	#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
    42	pub enum FramePart {
    43	    #[error("frame head")]
    44	    FrameHead,
    45	    #[error("signal")]
    46	    Signal,
    47	    #[error("query child listing")]
    48	    QueryChildren,
    49	    #[error("supply run head")]
    50	    SupplyLength,
    51	    #[error("supply run")]
    52	    SupplyRun,
    53	}

    src/error.rs
    72	    #[error("peer is not a rumors stream (leading bytes: {remote_magic:x?})")]
    73	    MagicMismatch { remote_magic: [u8; 6] },

    src/tree/mirror/handshake.rs
    37	const MISMATCH_PREVIEW_LEN: usize = 6;
    ...
    178	    MagicMismatch { remote_magic: [u8; 6] },

Resolution: add `#![warn(missing_docs)]` to lib.rs (the clippy leg's `-D
warnings` then enforces it) and write the missing sentences, or make the
item `pub(crate)` where the sentence would be "internal" (api-audit-14
covers the constructors). Replace both `[u8; 6]` with `[u8;
MISMATCH_PREVIEW_LEN]` and either export the constant or document the
field's width in words. Acceptance: `cargo clippy -p rumors --lib
--all-features -- -D warnings` passes with `#![warn(missing_docs)]` in
lib.rs.

### remote-codec-19: Public error docs: uneven variant coverage, undocumented public constructors, and two docs naming internals
- Where: src/tree/mirror/streaming/remote/codec/error.rs:97-102 (related: error.rs:19-26, :40-53, :63-76, :111-159, :150-154; src/tree/mirror/streaming/remote/codec/signal.rs:112-117, :137-154, :396-397; src/tree/mirror/streaming/remote/codec.rs:75-76; src/tree/mirror/streaming/remote/error.rs:11-16)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified for the two internals (read: `LeafRun` is absent from the export list at remote/error.rs:11-16, and `SUPPLY_FRAME_OVERHEAD` is re-exported only under `#[cfg(test)]` at codec.rs:75-76); the coverage tally assessed by reading error.rs and signal.rs
- Seen by: prose (25, 26); refutation: confirmed both; history: the `declared` field doc is a verbatim transcription of REVIEW.md seed 3's prescribed wording, whose purpose survives the rewording; `Origin::direction`/`stream` have been public and undocumented since e41e7069; no `missing_docs` lint has ever been enabled
- Owner-gated: yes for narrowing the public constructors; no for the prose

Within `DecodeErrorKind`, three variants carry docs and seven do not; `EncodeErrorKind`, `FramePart`, and `Speaker` variants have none; `Origin::direction` and `Origin::stream` are undocumented `pub fn`s; `StreamClass` documents one variant of five; `DecodeSignalError::Placement` is undocumented. `src/error.rs:3-6` tells users this taxonomy is "for matching and bug reports", so the distinction a matcher needs (`Read`: the transport failed; `Truncated`: a clean close mid-frame; `Malformed`: present but not canonical) belongs at the variants. Two docs also explain themselves in terms of names the API does not reach: `DecodeLeafError` via `LeafRun::records` and "the incoming adapter", and `OverbatchedRun::declared` via `SUPPLY_FRAME_OVERHEAD`.

Evidence:

    97	/// A decode or canonicality failure in a supplied leaf record.
    98	///
    99	/// Produced by the run's record iterator (`LeafRun::records`), which the
    100	/// incoming adapter drives record by record; run *structure* is instead
    ...
    115	    #[error("could not read the frame's {part}")]
    116	    Read {
    ...
    150	        /// The frame's charged wire size — its run body plus the
    151	        /// `SUPPLY_FRAME_OVERHEAD` envelope at its widest — which may

    19	impl Origin {
    20	    pub fn direction(speaker: Speaker) -> Self {

Resolution: One line per variant stating what separates it from its neighbors (`Read`: "The transport failed while the frame's `part` was being read."; `Truncated`: "The transport closed cleanly before the frame's `missing` component arrived."; `TrailingBytes`: "Bytes followed the frame where the input was required to end."; likewise for `EncodeErrorKind`, `FramePart`, `Speaker`, `StreamClass`, `DecodeSignalError::Placement`). Rewrite error.rs:99-100 as "Produced when a supplied record is decoded, after the run's structure has already been validated at the wire ([`LeafRunError`])." and error.rs:150-153 as "The frame's charged wire size: its run body plus a fixed frame-head envelope, which may exceed the frame's actual size by a few bytes of head slack." Document or narrow to `pub(crate)` the `Origin` constructors. Acceptance: every public variant and public method in error.rs and signal.rs has a doc comment; no public doc in error.rs names a type, function, or constant that `rumors::error` does not export.

### remote-capture-atlas-21: `FrameShape`'s doc says one- or two-element array; the decoder requires two or three
- Where: src/tree/mirror/streaming/remote/codec/error.rs:131-133 (related: src/tree/mirror/streaming/remote/codec/decode.rs:217-231, src/tree/mirror/streaming/remote/codec/decode/tests.rs:180-181)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (decode.rs:225 `if !(2..=3).contains(&head.value) {` against error.rs:131; decode/tests.rs:180-181's testdoc already states the correct arity)
- Seen by: correctness; refutation: confirmed; history: deliberate but expired (accurate under the single-int opener; 3327a92b moved the opener to two items and the range to `2..=3` and missed this variant doc)
- Owner-gated: no

Adjacent to the partition (the atlas pins this error): the variant doc names an arity the decoder rejects. Prose contradicts code.

Evidence:

    131	    /// The frame item is not a one- or two-element CBOR array.
    132	    #[error("frame is not a CBOR reaction array: {detail}")]
    133	    FrameShape { detail: &'static str },

    decode.rs:
    225	    if !(2..=3).contains(&head.value) {

Resolution: "The frame item is not a two- or three-element CBOR array." Acceptance: the variant doc agrees with `frame_arity`. Dedupe against the codec-core partition if it reports the same line.

Synthesis note: Same line as remote-codec-20, which carries the fuller resolution.

### remote-codec-20: The public `FrameShape` variant doc states the wrong array arity
- Where: src/tree/mirror/streaming/remote/codec/error.rs:131-133 (related: src/tree/mirror/streaming/remote/codec/decode.rs:217-231; src/tree/mirror/streaming/remote/codec.rs:3-5)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S"one- or two-element"` names only 4dd2053c, where decode.rs:227 read `if !(1..=2).contains(&head.value)`; `git log -S"(2..=3).contains"` names 3327a92b, whose stat touches no codec/error.rs)
- Seen by: structure (5), prose (17), correctness (40), perfapi (43); refutation: confirmed; history: deliberate-but-expired (the two-item opener widened the decoder without touching this line)
- Owner-gated: no

The doc says a frame is "a one- or two-element CBOR array"; every frame now opens with two items plus an optional body, `frame_arity` enforces `(2..=3)`, and the module doc describes `[stream, state]` and `[stream, state, body]`. The variant is reachable as `rumors::error::CodecDecodeErrorKind::FrameShape`, so a user reads a contract the code contradicts. The Display text "frame is not a CBOR reaction array" also misnames bare `End` frames, which are not reactions.

Evidence:

    131	    /// The frame item is not a one- or two-element CBOR array.
    132	    #[error("frame is not a CBOR reaction array: {detail}")]
    133	    FrameShape { detail: &'static str },

    decode.rs:225	    if !(2..=3).contains(&head.value) {
    decode.rs:227	            detail: "frame array is not two or three items",

Resolution: "The frame item is not a two- or three-item CBOR array (the opener's stream and state, then a body when the state takes one)."; consider "frame is not a CBOR array of two or three items" for the Display text (a wire-neutral message change, but the atlas snapshot pins it, so re-accept deliberately). Acceptance: the variant doc and Display agree with `frame_arity`'s accepted range and detail strings; no prose in the partition says "one- or two".

Synthesis note: Same line as remote-capture-atlas-21; this entry adds the `Display` text and notes the atlas snapshot pins it.

### remote-capture-atlas-22: `record_slices`'s visibility rationale names a capture-renderer use that does not exist
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:246-248 (related: src/tree/mirror/streaming/remote/codec/frame.rs:232, src/tree/mirror/streaming/remote/codec/frame.rs:240, src/tree/mirror/streaming/remote/codec/capture.rs:561-567, src/tree/mirror/streaming/remote/codec/capture.rs:634-688)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn record_slices src` returns frame.rs:232, 240, 248 only; capture.rs parses run bytes through `parse_node` in `render_embedded_as`)
- Seen by: correctness; refutation: confirmed; history: deliberate but expired (true when written in 74ebba4d; ef6569c4 rewrote the renderer to walk run bytes generically and dropped every `record_slices` call)
- Owner-gated: no

Adjacent to the partition (the rationale is about this partition's module): the `pub(super)` is justified by a caller in capture.rs that no longer exists; the only users are `record_count` and `records` in the same file. No ghost references: a rationale naming a phantom caller is dated reasoning at a declaration site.

Evidence:

    246	    /// `pub(super)` for the capture renderer, which decodes each
    247	    /// record's version structurally without knowing the leaf type.
    248	    pub(super) fn record_slices(&self) -> RecordSlices<'_> {

Resolution: Make `record_slices` private and delete the rationale (and narrow `RecordSlices`'s visibility if nothing else needs it). Acceptance: `grep -rn record_slices src` shows only frame.rs; no prose names a nonexistent caller.

### remote-codec-23: `checked_run_len`'s doc calls the encoder-side check redundant; it is the only bound on the accumulated body
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:300-306 (related: frame.rs:177-182; src/tree/mirror/streaming/remote/codec/encode.rs:117-122; src/tree/mirror/streaming/remote/codec/budget.rs:147-149; src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:92-98)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read: `push` calls `checked_run_len(item)` on the single record item; `FrameEncoding::new` calls it on `run.encoded_len()`, the accumulated body; `LeafRun` itself never consults `RunBudget::admits`)
- Seen by: prose (28); refutation: reframed (the call at encode.rs:118 is also the `usize` to `u64` conversion the byte-string head needs, typed as the cap check, so it is not standalone guard machinery; the doc sentence is the fix); history: the doc is 4dd2053c prose and the only rationale on record; the atlas exempts `SupplyTooLarge` as "resource exhaustion by construction", consistent with a programmer-error path, not with "belt to that suspender"
- Owner-gated: no for the doc; the typed-error-versus-assert question is the owner's (see open questions)

The doc says a run the cap rejects "was necessarily a single record" that `push` already rejected, and calls the encoder-side call "the belt to that suspender", whose mechanism reading is "a redundant second check". It is not redundant: `push` bounds each record item, never the cumulative body, so a caller that pushes past the cap without consulting `RunBudget::admits` is caught here and nowhere else. A guard is justified by naming the constructible failure it catches; this doc names the wrong justification and omits the right one.

Evidence:

    300	/// Check a run body length against the wire's run byte cap.
    301	///
    302	/// The encoder's boundary: a run the cap rejects was necessarily a single
    303	/// record (the budget saturates below the cap, so a multi-record run never
    304	/// grows here), and [`LeafRun::push`] already rejected any such record —
    305	/// this check is the belt to that suspender, priced identically.

    179	        let item = RECORD_TAG_LEN
    ...
    182	        checked_run_len(item)?;
    encode.rs:118	                let len = super::frame::checked_run_len(run.encoded_len())?;

Resolution: Restate: "The run byte cap as a checked conversion to the u64 the run head carries. `push` bounds each record item; the encoder applies this to the accumulated body, which only a caller pushing past the cap without consulting `RunBudget::admits` can exceed (the budget saturates below the cap). Reaching it there is a programmer error in the accumulator, surfaced typed as `EncodeErrorKind::SupplyTooLarge`." Acceptance: the doc names the constructible failure; no metaphor remains.

### remote-codec-29: `read_greeting`'s doc describes an error arm the signature does not have
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:229-234 (related: greeting.rs:235 and :282-294)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); the history pass verified with `git show 4dd2053c` that the phrase and the `Result<Greeting, ReadGreetingError>` signature landed together
- Seen by: structure (16), prose (20); refutation: confirmed; history: opaque from the start; 0e2e85c6 rewrote the following clause and left the phrase
- Owner-gated: no

The doc says transport failures "pass through as `Err(Ok-side io)`", a phrase describing no arm of `Result<Greeting, ReadGreetingError>`; the reader must scroll to the enum to learn the three arms are `Io`, `Decode`, and `Listing`. The sentence that should name them misdirects instead.

Evidence:

    229	/// Read one complete greeting item from the control stream.
    230	///
    231	/// Transport failures pass through as `Err(Ok-side io)`; a malformed or
    232	/// non-canonical greeting is a typed [`GreetingError`], except a
    233	/// non-canonical listing order, surfaced separately so the handshake can
    234	/// report it as the codec's own violation class.

Resolution: "Transport failures surface as [`ReadGreetingError::Io`]; a malformed or non-canonical greeting as [`ReadGreetingError::Decode`], except a non-canonical listing order, which [`ReadGreetingError::Listing`] carries separately so the handshake can report it as the codec's own violation class." Adjust if remote-codec-28 lands. Acceptance: the doc names each variant of `ReadGreetingError` by intra-doc link and contains no `Ok-side` phrase.

### remote-codec-33: Two arms of the phase schedule carry no rule
- Where: src/tree/mirror/streaming/remote/codec/signal.rs:326-341 (related: signal.rs:137-154)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read)
- Seen by: prose (27); refutation: confirmed; history: the arms and the `VALID_PLACEMENTS` pin date from 3dac1b85 (no body); the structural reason exists only in `formal/MODEL.md` (`LeafRequests`, `TerminalLeafResolutions`), which AGENTS.md forbids citing from code, so it must be restated inline
- Owner-gated: no

`validate` explains why the initiator's opening stream admits only supplies and ends, but says nothing at the `LeafParentReplies` arm (why a leaf-parent reply never carries a nonempty `Query`) or the `TerminalLeafReplies` arm (why the responder's last stream admits only a reply-ending `Supply`). The snapshot pins the table; nothing in code states the rule it encodes, and the `StreamClass` variant docs are labels without rules. A table with one arm explained and two not invites the next editor to widen an arm without knowing what it protects; the schedule is wire format.

Evidence:

    329	            // One supplies-only reply (empty when pruning left nothing),
    330	            // then the stream end: the opening carries answers the
    331	            // responder is about to ask for, never questions of its own.
    332	            StreamClass::OpeningSupplies => {
    333	                matches!(self.signal, Signal::Supply(_) | Signal::End(_))
    334	            }
    335	            StreamClass::OpeningReply => true,
    336	            StreamClass::InteriorReplies => true,
    337	            StreamClass::LeafParentReplies => !matches!(self.signal, Signal::Query(_)),
    338	            StreamClass::TerminalLeafReplies => {
    339	                matches!(self.signal, Signal::Supply(Flow::End) | Signal::End(_))
    340	            }

Resolution: One comment per restricted arm, in the owner's words; my reading of the schedule: for `LeafParentReplies`, "A leaf-parent's children are leaves, which are supplied whole, never listed: a nonempty query has nothing to name."; for `TerminalLeafReplies`, "The responder's final stream answers only the initiator's empty leaf-parent queries, each with exactly one reply-ending supply." Say the same in one line at the two `StreamClass` variants. Acceptance: each arm of `validate` whose admitted set is a proper subset carries a comment stating its rule.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| remote-codec-25 | src/tree/mirror/streaming/remote/codec/frame.rs:370-371 | The per-record payload copy is the custody hand-off that keeps per-leaf pricing exact; the site does not say so. | One sentence at the call. |
| remote-codec-31 | src/tree/mirror/streaming/remote/codec/signal.rs:67-70, 203-204, 228 | `at_height` reuses a phase remainder as the index offset without the derivation; `STATE_STRIDE` is used once, for another meaning. | A named offset with its derivation; retire `STATE_STRIDE`. |
| remote-codec-32 | src/tree/mirror/streaming/remote/codec/signal.rs:112-135 | `Speaker` and `observe::Role` name one election; the reason (the hook is rumors-blind) lives only in a commit message. | One sentence at `Speaker`; unification is owner-gated. |

## Remote capture and codec tests

### remote-capture-atlas-2: The module doc describes a query-listing spelling the wire no longer has
- Where: src/tree/mirror/streaming/remote.rs:30-31 (related: src/tree/mirror/streaming/remote/codec.rs:37-41, src/tree/mirror/streaming/remote/codec/frame.rs:517-518, src/tree/mirror/streaming/remote/codec/encode.rs:112-115)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`write_listing` at frame.rs:517-518 writes `cbor::write_head(out, MAJOR_MAP, children.len() as u64)`, the count carried directly in the map head; `git blame -L30,31` dates the lines to e41e7069 and c5f1210a; `git show c5f1210a:src/tree/mirror/streaming/remote/codec/encode.rs` carries `QUERY_COUNT_BIAS` and a `count: [u8; 1]` body field at lines 17, 54, 71; `git show 4dd2053c -- remote.rs` touched only the `mod codec` visibility and one re-export line)
- Seen by: prose, correctness; refutation: confirmed; history: deliberate but expired (accurate for the byte-coded wire; orphaned by 4dd2053c, which rewrote codec.rs's doc and left this paragraph)
- Owner-gated: no

The doc of record for the proxy says a nonempty query's body is a "one-byte count-minus-one". The encoder writes a `{radix: hash}` CBOR map whose head carries the count directly (one byte below 24, two or three bytes above), and codec.rs:37-38 documents it that way. A reader implementing or debugging against this paragraph goes looking for a count byte that is not on the wire. This breaches the AGENTS.md hard rule that nothing in the codebase refers to code that no longer exists.

Evidence:

    30	//! An empty query occupies its signal alone; a nonempty query's one-byte
    31	//! count-minus-one admits every fan from 1 through 256.

    frame.rs:
    517	pub(crate) fn write_listing(out: &mut Vec<u8>, children: &[(u8, Hash)]) {
    518	    cbor::write_head(out, MAJOR_MAP, children.len() as u64);

Resolution: Delete the sentence and let codec.rs own the grammar (finding 1), or restate it: "An empty query occupies its signal alone; a nonempty query's body is a `{radix: hash}` map of one to 256 children, its map head carrying the count." While there, lines 33-41 omit the record's tag-63 wrapper and the version atom's own tag that codec.rs:44-46 states; acceptable at this altitude if deliberate, but decide. Acceptance: `grep -rn count-minus-one src` is empty and any remaining sentence matches `write_listing`.

### remote-capture-atlas-1: The root module doc restates its submodules' contracts, literal tallies included
- Where: src/tree/mirror/streaming/remote.rs:12-41 (related: src/tree/mirror/streaming/remote.rs:56-67, src/tree/mirror/streaming/remote.rs:8, src/tree/mirror/streaming/remote/codec.rs:18-56, src/tree/mirror/streaming/remote/streams.rs:3, src/tree/mirror/streaming/remote/codec/signal/tests.rs:7)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (side-by-side read of remote.rs against codec.rs:18-56 and streams.rs:1-10; `grep -rnE '\b(162|163|340)\b' src` returns remote.rs:18, codec.rs:25-26, codec/tests.rs:246, and the enforced home signal/tests.rs:7; `git show --stat 3327a92b` shows the last opener change edited both remote.rs and codec.rs)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: no rationale found (the doc was deliberately reshaped in c5f1210a and maintained in lockstep, but nothing records why the root should carry the grammar)
- Owner-gated: no

After the one-paragraph role map (lines 3-6), the root doc re-describes the frame grammar with its counts (17 streams, ten states, 162/163 placements), the reply/stream end separation, the query and run spellings, scope retention, and stream binding, each of which the owning submodule states in nearly the same words. The literal tallies therefore live in two prose homes beside their enforced homes (`Stream::COUNT`, `Signal::STATE_COUNT`, `VALID_PLACEMENTS`), and the copy drifts independently: finding 2 is the drift this duplication has already cost. Principle: documentation altitude (a module root orients and points; it does not restate a child's contract) and no hand-maintained counts.

Evidence:

    12	//! [`codec`] defines the common frame grammar: a frame opens with its
    13	//! stream's index (one of 17) and its signal's state code (one of ten:
    ...
    17	//! reserved. The phase schedule narrows that product: the initiator
    18	//! admits 162 placements and the responder 163, rejecting the rest

    codec.rs:
    18	//! A frame opens with two unsigned ints: the index of the logical stream
    19	//! it rides (one of 17) and its signal's state code (one of ten frame
    ...
    25	//! select a phase-specific subset of states: the initiator admits 162
    26	//! placements and the responder 163, rejecting the rest before their

    streams.rs:
    3	//! This layer binds the protocol's 17-per-direction logical streams onto a

Resolution: Trim remote.rs to the role map (3-6), the transport sentence (8-10, citing `Stream::COUNT` by name rather than "17"), and the cross-cutting account only this level can tell (the opening question riding the greeting and the early-supply stream, 43-54), with intra-doc links to `codec`, `adapter`, and `streams` for their own contracts. Where a count survives anywhere in prose (codec.rs:25-26, streams.rs:3), name the enforced place (`VALID_PLACEMENTS`, `Stream::COUNT`) rather than the literal. Acceptance: `grep -nE '162|163|one of 17|one of ten' src/tree/mirror/streaming/remote.rs` is empty; no sentence in remote.rs's module doc duplicates one in codec.rs, adapter.rs, or streams.rs.

Synthesis note: The literal counts recur in remote-codec-1 (codec.rs) and remote-adapter-streams-17 (streams.rs); the patterns section lists every site.

### remote-capture-atlas-10: `MAX_DEPTH` states no derivation and no relation to the payload depth limit
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:334-341 (related: src/message.rs:59, src/tree/mirror/streaming/remote/codec/capture.rs:280, src/tree/mirror/streaming/remote/codec/capture.rs:490, src/tree/mirror/streaming/remote/codec/capture.rs:567, src/tree/mirror/streaming/remote/codec/capture.rs:647, src/tree/mirror/streaming/remote/codec/capture.rs:685, src/tree/mirror/streaming/remote/codec/capture/tests.rs:3-5)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (traced depth through `render_frame` (parse at 0) -> `render_node` Tag arm (`depth + 1`) -> `render_tag` -> `render_embedded_as` (re-parse at 1) -> record Tag (`depth + 1`) -> `render_embedded_as` (re-parse at 2): a supply record's payload root parses at depth 2, so `depth >= MAX_DEPTH` at line 350 trips once the payload nests about 62 levels; message.rs:59 admits 256)
- Seen by: prose, correctness; refutation: confirmed; history: no rationale found (the constant arrived at 64 with no derivation; the prior review named both numbers without relating them)
- Owner-gated: no

The constant's doc explains that the budget is shared, not why 64. A payload legal under `DEFAULT_PAYLOAD_DEPTH_LIMIT` (256) but nested past about 62 levels renders as the explicit hex fallback, which keeps the pin injective but forfeits the field-localization commitment (capture/tests.rs:3-5) for that payload. That is the next question a maintainer asks, and the declaration does not answer it. Named constants over magic numbers: the name is there, the derivation is not.

Evidence:

    334	/// Nesting past this bound falls back to exact hex: the walk never
    335	/// recurses on unbounded input-controlled depth.
    ...
    341	const MAX_DEPTH: usize = 64;

    message.rs:
    59	pub const DEFAULT_PAYLOAD_DEPTH_LIMIT: PayloadDepthLimit = PayloadDepthLimit(256);

Resolution: Add one sentence at the declaration: the bound is a legibility bound chosen below `DEFAULT_PAYLOAD_DEPTH_LIMIT`; a fixture nested that deep pins as the explicit hex fallback rather than a value tree; embedded-byte-string chains (which the payload limit does not bound) are why the bound must exist at all. Deriving the bound from the payload limit instead would deepen `render_node`'s recursion to 256-plus frames, so documenting is the safer option. Acceptance: the doc states why 64 and names the relationship to the payload depth limit.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| remote-capture-atlas-6 | src/tree/mirror/streaming/remote/codec/capture.rs:4-5, 42-47, 347-349 | Three sentences misstate the hook contract, the panic boundary, and the error type (`Result<Node, String>` called "a typed reason"). | Three rewrites as the entry lists. |
| remote-capture-atlas-7 | src/tree/mirror/streaming/remote/codec/capture.rs:22 (eight renderer sites) | "the walk" in the renderer collides with the anchored term for the in-process participant. | "the renderer" or "the traversal". |
| remote-capture-atlas-12 | src/tree/mirror/streaming/remote/codec/capture.rs:464-469 | The `"listing"` key rule fires at every depth, so application payload maps acquire listing annotations. | Scope the rule to the greeting, or state the context-free choice at the comment. |
| remote-capture-atlas-18 | capture/tests.rs:7-8; codec/tests/error_atlas.rs:133-134 | A doubled "silently" naming no mechanism; "genuinely absent" where "absent" says it. | Reword both. |

## Remote adapter and streams

### remote-adapter-tests-17: parking.rs module doc states megabyte figures that contradict its own pin and `message.rs`
- Where: src/tree/mirror/streaming/remote/adapter/tests/parking.rs:13-20 (related: src/tree/mirror/streaming/remote/adapter/tests/parking.rs:51-59, src/tree/mirror/streaming/message.rs:14-17)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log -L13,20` on parking.rs shows only 3a5ba643 and c20b9cf4 touched the figure lines; `git show 2d1e6ea5 -- parking.rs` and `git show 4dd2053c -- parking.rs` each change only the constant line, 2_300_000 to 3_380_000 to 3_570_000, while message.rs:15-16 was re-derived at both to ≈ 1.8 MB / ≈ 3.5 MB; `size_of::<(u8, Hash)>()` with `MERKLE_HASH_LEN = 24` is 25, so the decoded half alone is 65,536 × 25 = 1,638,400 bytes, above the prose's 1.1 MB)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale found; the owner ruling recorded in 2d1e6ea5's message ("Prose figures that were hand-synced to measurements are excised rather than re-synced where they are not load-bearing ... cite the pinned constant or recomputation instead") prescribes the shape of the fix
- Owner-gated: no

The module doc says the disputed-reply skeleton is "≈ 1.1 MB encoded, ≈ 2.2 MB while the encoded and decoded forms coexist"; the pin beside it is 3,570,000 with about 3% headroom, and `message.rs`, which this doc says states the same figure, reads ≈ 1.8 MB / ≈ 3.5 MB. The constant's own comment names preventing exactly this staleness as its purpose; the figure went stale twice anyway because only the constant is enforced. Three sites disagree and a re-baseliner cannot tell which is authoritative.

Evidence:

    13	//! node, and a maximally disputed reply retains a pure skeleton of at most
    14	//! fan² `(radix, hash)` entries: ≈ 1.1 MB encoded, ≈ 2.2 MB while the
    15	//! encoded and decoded forms coexist mid-decode. That coexistence
    16	//! transient is the figure the session's memory model charges per parked
    17	//! reply (`streaming/message.rs`), and it is pinned here as a sum of the

    55	/// Chosen tight so growth in either half — a wider hash, a larger fan,
    56	/// heavier framing — fails the pin and forces the module doc's charged
    57	/// figure (and `streaming/message.rs`, which states it) to be
    58	/// re-derived rather than silently going stale.
    59	const DISPUTED_REPLY_TRANSIENT_CEILING: usize = 3_570_000;

    15	//! reactions × a 256-entry listing ≈ fan² hashes ≈ 1.8 MB encoded
    16	//! (≈ 3.5 MB while an encoded and a decoded copy coexist), transient, at

Resolution: apply the standing ruling: excise the megabyte figures from the module doc and cite the mechanism and the enforced constant by name ("the encoded reply plus the decoded fan² skeleton, pinned by `DISPUTED_REPLY_TRANSIENT_CEILING`"); have message.rs name the same constant so there is one number of record, or give the crate one visible constant both cite. Acceptance: `grep -n MB` over parking.rs returns no figure disagreeing with the pin; the module doc, the constant doc, and message.rs:14-17 agree or defer to the named constant.

### remote-adapter-streams-1: Adapter module doc describes the pre-erasure `Reply<B, H>` boundary and names `Convert::assemble` where `Backend::assemble` runs
- Where: src/tree/mirror/streaming/remote/adapter.rs:4-14 (related: src/tree/mirror/streaming/remote/adapter.rs:52, src/tree/mirror/streaming/remote/adapter.rs:56-58, src/tree/mirror/streaming/remote.rs:59, src/tree/mirror/streaming/remote/adapter/scope.rs:5-7, src/tree/mirror/streaming/remote/adapter/decode.rs:14, src/tree/mirror/streaming/remote/adapter/decode.rs:88, src/tree/mirror/streaming/remote/adapter/encode.rs:84, src/tree/mirror/streaming/erased.rs:319-334, src/tree/mirror/streaming/backend.rs:198-203, src/tree/mirror/streaming/backend/local.rs:191)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `Reply<B` across remote/; read the entry-point signatures, `erased::ops::assemble`, `Backend::assemble`'s default body, and `Local`'s override)
- Seen by: structure ([13]), prose ([18], [19]); refutation: confirmed (all three); history: deliberate-but-expired (d8bef16b's diffstat excludes adapter.rs; 48bc31df edited the diagram line, dropped `T`, kept `H`; 88a76a71 added `Backend::assemble` over the `Convert::assemble` the doc still names)
- Owner-gated: no

The module doc links `Reply` to the height-typed `message::Reply`, draws the boundary as `Reply<B, H>`, speaks of grouping leaves by "their height-`H` prefix", and says records flow "into the existing `Convert::assemble` fold". Every adapter entry point takes or returns the erased form (`decode.rs:14` imports `erased::{Reaction as ProtocolReaction, Reply, ops}`; `encode_reply` takes `reply: Reply<B::Erased>` at encode.rs:84; `decode_reply` returns `Decoded<B::Erased, Vec<Scope>>` at decode.rs:210), the height is a runtime witness recovered from the scope's prefix length (which scope.rs:5-7 states), and the decoder calls `ops::assemble` (decode.rs:88, 408), which dispatches to `Backend::assemble` (erased.rs:327-331); `Convert::assemble` is only that method's default body (backend.rs:202) and `Local` overrides it (local.rs:191), so under the in-memory backend the named fold never runs. Principle 5: a document reads as if written today against today's code; "existing" is a dated qualifier in one word.

Evidence:

         4	//! levels. In memory, one [`Reply`](super::super::message::Reply) contains
        ...
        11	//! ```text
        12	//! Reply<B, H> -- encode + explode --> Frame leaves
        13	//! Reply<B, H> <-- decode + assemble -- Frame leaves
        14	//! ```
        ...
        52	//! height-`H` prefix. Strict path and run ordering make those group
        ...
        56	//! The decoder yields each record as it is decoded through a fan-bounded
        57	//! channel into the existing
        58	//! [`Convert::assemble`](super::super::convert::Convert::assemble) fold,

Resolution: Link `Reply` to `erased::Reply`, draw the diagram as `Reply<E>` (or `erased::Reply<B::Erased>`) with one clause saying the height rides as the retained scope's prefix length; reword line 52 to "the prefix at the scope's children height"; rewrite 56-58 as "through a fan-bounded channel into [`Backend::assemble`] (dispatched at the scope's children height by `erased::ops::assemble`)", dropping "existing", and apply the same fix to remote.rs:59 ("the backend's existing conversion fold"). Acceptance: no `Reply<B, H>` or bare `H` parameter remains in adapter.rs prose; the `Reply` intra-doc link resolves to the erased type the signatures use; adapter.rs names `Backend::assemble` for the decode side as it names `Backend::leaves` (line 44) for the encode side; `grep -n existing` over adapter.rs and remote.rs is empty.

### remote-adapter-streams-11: The clippy-allow rationale names a gate platform the verification recipes do not name, and moralizes the lint flag
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:565-568 (related: rust-toolchain.toml:31, tests/common/wire.rs:22-25, and eight further copies of the same comment across the workspace per `grep -rn 'illumos among' src/ tests/ crates/`)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -rn illumos justfile .github/ rust-toolchain.toml Cargo.toml .cargo/ .config/` is empty; rust-toolchain.toml:31 is `targets = ["wasm32-unknown-unknown"]`; the comment recurs at ten sites)
- Seen by: prose ([21]); refutation: confirmed (eb4e0e1ba's message shows the claim is operationally true as a manual re-check on ox-east-1); history: deliberate-and-holds for the rationale (stated in eb4e0e1b), but propagated to ten copies (f924463a), and "honest" has no recorded rationale
- Owner-gated: no for the wording; the roster question (encode the illumos run in the recipes, or drop the platform claim) is an owner call (see open questions)

The comment justifies the allow by naming illumos as one of "the gate's targets". Nothing in the tree says so: the justfile, CI workflows, and the toolchain pin name only `wasm32-unknown-unknown`. The gate is run on illumos by hand, which is operational provenance a reader cannot corroborate from the tree (Principle 8). "keeps `-D warnings` honest" is moralized code for "clean". The fix is workspace-wide (ten copies), so a partition-local rewording would leave nine divergent copies.

Evidence:

       565	    // clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
       566	    // fallback-TLS lowering (illumos among the gate's targets) and denies
       567	    // initializers that already sit in `const` blocks; the allow keeps
       568	    // `-D warnings` honest on every platform the gate runs.

Resolution: State the platform-independent mechanism without the roster, at all ten sites: "clippy's `missing_const_for_thread_local` fires on `const {}` initializers when `thread_local!` lowers to fallback TLS; the allow keeps `-D warnings` clean under either lowering." If illumos is meant to be a gate platform, make it one in the justfile or CI so the claim becomes checkable. Deduplicating the ten copies (a shared `thread_local!` helper or a crate-level allow) is a separate question. Acceptance: the comment names no platform the verification recipes do not name, or the recipes name it; "honest" is gone from every copy.

Synthesis note: Same comment as remote-proxy-22.

### remote-adapter-streams-15: Mux-era vocabulary survives the mux's deletion: "demultiplexer" and "reintroduce"
- Where: src/tree/mirror/streaming/remote/adapter/error.rs:69-71 (related: src/tree/mirror/streaming/remote/streams.rs:4, src/tree/mirror/streaming/remote/streams.rs:640, src/tree/mirror/streaming/remote/streams.rs:478-482; outside the partition: src/tree/mirror/streaming/remote/codec.rs:32, src/tree/mirror/streaming/remote/codec/signal.rs:31)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'multiplex\|reintroduc'` over the partition and codec files: error.rs:69, streams.rs:4 ("Nothing multiplexes"), streams.rs:640, codec.rs:32, signal.rs:31)
- Seen by: prose ([22]); refutation: confirmed; history: deliberate-but-expired (error.rs:69 written in 24b187a8 when the mux/demux layer existed; b3b877d9 deleted it "outright"; the sweeps 1a07a090 and 339561aa fixed sibling sites and missed this one; streams.rs:640 was written in b3b877d9 naming the design it deleted)
- Owner-gated: no

`UnexpectedStreamEnd`'s doc says control "leaked through the demultiplexer", while the streams module doc opens "Nothing multiplexes" (streams.rs:4). `AcceptDriver`'s doc says a shared reader "would reintroduce" head-of-line coupling, a verb that parses only if the reader knows a mux once existed. Both sit under AGENTS.md's hard rule that nothing in the codebase refers to code that no longer exists. Since `StreamReceiver` consumes `End::Stream` (streams.rs:478-482), the variant is in fact reachable only from in-process frame streams, which the doc could say, as decode.rs:179-181 does for its sibling case.

Evidence:

        69	    /// Transport control leaked through the demultiplexer into reply decoding.
        70	    #[error("a stream-end control reached the protocol reply decoder")]
        71	    UnexpectedStreamEnd,

    streams.rs:
       640	/// shared reader would reintroduce is structurally absent. The driver runs

Resolution: error.rs:69: "A stream-end control reached reply decoding; [`StreamReceiver`] consumes it on the wire path, so this is reachable only from frame streams constructed in process." streams.rs:640: "would introduce". For the codec partition's owner: codec.rs:32 ("The session demultiplexer") and codec/signal.rs:31 ("Logical streams multiplexed into each transport direction"). Acceptance: `grep -n 'multiplex\|reintroduc'` over adapter/ and streams.rs matches only streams.rs:4.

### remote-adapter-tests-1: tests.rs module map omits `backend_errors`
- Where: src/tree/mirror/streaming/remote/adapter/tests.rs:1-11 (related: src/tree/mirror/streaming/remote/adapter/tests.rs:23)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read lines 1-29; `git log -L1,11` shows f94f2056, 3a5ba643, and d6537a7b each appended their module to the map and no commit added `backend_errors`, which 07504b2e declared)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (completeness is the evident intent; this is the one omission)
- Owner-gated: no

The module doc maps six of the seven submodules by name and role and omits `backend_errors`, declared at line 23. A reader using the map to find where injected backend failures are covered will not find it. A module map that enumerates its members is a hand-maintained list, and this one has already rotted once.

Evidence:

    3	//! [`properties`] states the adapter's laws and sweeps the complete type-level
    4	//! height ladder. [`malformed`] pins the smaller set of wire shapes which must
    5	//! be rejected before those laws can apply. [`opening`] covers the one
    6	//! deliberately exceptional reply in the protocol. [`runs`] states the
    7	//! supply-run batching contract the byte budget imposes on the encoder.
    8	//! [`parking`] pins the memory accounting that makes a parked decoded reply
    9	//! O(fan) handles rather than a subtree. [`fan_occupancy`] pins the
    10	//! reader/assembler channel's occupancy ceiling — the supply-decode
    11	//! envelope's charge premise.
    ...
    23	mod backend_errors;

Resolution: add one sentence for `backend_errors` (its role is source-error propagation and atomicity across the backend operations the adapter reaches), or restate the map as structure rather than roster. Acceptance: every `mod` declared in tests.rs is named in its module doc.

### remote-adapter-streams-17: Stream count hand-written as "17" in prose where a named constant exists
- Where: src/tree/mirror/streaming/remote/streams.rs:3 (related: src/tree/mirror/streaming/remote/streams.rs:70-71, src/link.rs:161-169, src/tree/mirror/streaming/remote/codec/signal.rs:31-32; outside the partition: src/tree/mirror/streaming/remote.rs:8, src/tree/mirror/streaming/remote.rs:13, src/tree/mirror/streaming/remote/codec.rs:19)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read link.rs:161-169 with its `ceil(32 / 2) + 1 = 17` derivation and signal.rs:32 `pub const COUNT: u8 = 17;`; streams.rs:70-71 defines a private `STREAM_COUNT` from `Stream::COUNT`; grep for the literal in remote.rs and codec.rs)
- Seen by: structure ([14]), prose ([16]); refutation: confirmed (both); history: no-rationale-found (b3b877d9 wrote "17-per-direction" in the same commit that defined `STREAM_COUNT = 17`; 3327a92b writes "one of 17" fresh today, so the fix is a cross-file decision)
- Owner-gated: no

The module doc restates the per-direction stream count as a literal. The number is defined and derived in `link::STREAM_COUNT` (link.rs:165-169) and `Stream::COUNT` (signal.rs:32), pinned equal by test, and this file already names it two screens down. No hand-maintained counts: a number the code owns is cited by name from prose, never retyped where a wire-schedule change would orphan it. The same literal appears in remote.rs:8, 13 and codec.rs:19 (other partitions).

Evidence:

         3	//! This layer binds the protocol's 17-per-direction logical streams onto a

        70	/// Number of logical streams per direction, as an array dimension.
        71	const STREAM_COUNT: usize = Stream::COUNT as usize;

Resolution: "binds the protocol's [`Stream::COUNT`] logical streams per direction onto ..." (or link `crate::link::STREAM_COUNT`); likewise at the remote.rs and codec.rs sites. Acceptance: the literal 17 appears in prose only at its derivation site in link.rs; the intra-doc link resolves under `just docs`.

Synthesis note: Overlaps remote-codec-1 and remote-capture-atlas-1.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| remote-adapter-streams-7 | adapter/decode.rs:283-284; streams.rs:445 | Em-dashes in `//` comments. | Crate-wide pattern. |
| remote-adapter-streams-8 | adapter/decode.rs:492-496, 460-461; adapter/error.rs:84-98 | The version-bound rationale is stated three times. | Keep it on the variant; a one-line pointer at the check. |
| remote-adapter-streams-13 | src/tree/mirror/streaming/remote/adapter/encode.rs:183; streams.rs:522, 555 | A `debug_assert!` with no message and two `expect`s that name the lock rather than why poisoning cannot occur. | Messages stating why each cannot fire. |
| remote-adapter-tests-3 | adapter/tests.rs:65-73 | `LeafCase` claims distinct cases produce distinct versions, but `wrapping_shl(8)` drops `value`'s top byte. | `value << 8`; qualify the doc. |
| remote-adapter-tests-8 | adapter/tests/fan_occupancy.rs:12-16 and the sites listed | "real" as a quality marker, "silently" without mechanism, "provably", "RAM-sound". | Rewrite to the property that holds. |
| remote-adapter-tests-16 | adapter/tests/parking.rs:5-7 | The one-slot response relay is attributed to `proxy/work/queues.rs`; `Work::respond` constructs it. | Cite `Work::respond`. |
| remote-adapter-tests-18 | adapter/tests/parking.rs:66-74, 136, 198-201 | A runtime size assertion is called "compiler-checked"; two em-dashes; the pin's `eprintln!` readout is unexplained. | `const` assert or "pinned"; ` -- `; state the readout's purpose. |
| remote-adapter-streams-23 | src/tree/mirror/streaming/remote/streams.rs:286-289 | `SupplyClosed.source`'s doc says the session "reports" it; `ErrorRoute::report` is the path that never carries it. | "surfaces"; point at the deposit attachment site. |

## Remote proxy

### remote-proxy-1: Public RemoteError variant docs misdescribe four variants
- Where: src/tree/mirror/streaming/remote/proxy/error.rs:19-27 (related: src/tree/mirror/streaming/remote/proxy/error.rs:57-62, src/tree/mirror/streaming/remote/codec/greeting.rs:3, src/tree/mirror/streaming/remote/streams.rs:698-707, src/tree/mirror/streaming/remote/streams.rs:793, src/error.rs:44-50)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep `greeting frames` across src: only these two lines; `git blame` puts both on 0aa29ed94, whose message describes a two-frame greeting; read `codec/greeting.rs:3` and `start.rs:280-281` for the current one-item spelling; read `AcceptDriver::run` and both wrapped enums' variant lists)
- Seen by: prose (two candidates); refutation: confirmed both, each downgraded to low; history: deliberate-but-expired (0aa29ed94 wrote "frames" when the greeting was two frames; 4dd2053c9 respelled it as one item without touching error.rs) and no-rationale-found for the `Stream`/`Accept` summaries (loose from birth in b3b877d9b)
- Owner-gated: no

`RemoteError` is public rustdoc (re-exported at `src/error.rs:44-50` and embedded in `MirrorError`), and four of its variant summaries do not match the code. `HandshakeRead`/`HandshakeWrite` speak of "greeting frames", but the greeting is one self-delimiting control-stream item. `Accept` claims a transport stream "could not be accepted", but an acceptor failure is never returned as `Accept`: `AcceptDriver::run` deposits it and parks, and the consumer surfaces `Stream(SupplyClosed)`; `AcceptError`'s variants are all stream-discipline violations. `Stream`'s summary covers `Decode` and `Truncated` but not `Mislabeled` or `SupplyClosed`, the variant the enum-level paragraph is about. Principle: prose speaks in the present tense (the plural describes a wire shape that no longer exists), and public rustdoc must state every return arm accurately.

Evidence:

    19	    /// Reading one of the peer's greeting frames failed.
    25	    /// Writing and flushing the local greeting frames failed.
    57	    /// An incoming logical stream failed to decode or ended prematurely.
    60	    /// An incoming transport stream could not be accepted or routed.

    greeting.rs:3	//! One control-stream item: an embedded-CBOR-item tag (24) wrapping a
    streams.rs:699	                Err(AcceptFate::SupplyFailed(source)) => {
    streams.rs:700	                    self.route.supply_failed(source);
    streams.rs:701	                    drop(self.slots);
    streams.rs:702	                    cancelled().await
    streams.rs:793	/// An incoming transport stream violated the session's stream discipline.

Resolution: reword each summary from its source's contract, keeping to one line each (the owner's PR #38 directive on this enum, reported by the history pass, asks for parsimony): "Reading the peer's greeting item failed." / "Writing and flushing the local greeting item failed." / `Stream`: "An incoming logical stream failed, or its stream supply closed before it arrived." / `Accept`: "An incoming transport stream violated the session's stream discipline." Acceptance: `grep -rn "greeting frames" src/` is empty; the `Accept` summary no longer claims acceptor failure; each summary names only outcomes its wrapped enum produces, checked against `StreamError` (streams.rs:265-298) and `AcceptError` (streams.rs:796-826).

Synthesis note: The "greeting frames" ghost is the same residue as remote-proxy-tests-1's test names and testing-infra-16's `greeting_frame_len` doc.

### remote-proxy-tests-1: Five greeting-ingress test names speak the retired two-frame greeting
- Where: src/tree/mirror/streaming/remote/proxy/start/tests.rs:73-74 (related: start/tests.rs:92, 114, 151, 169; src/tests.rs:262-264)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git show a72fa0395:src/tree/mirror/streaming/remote/proxy/start.rs` lines 199-200 read "Send one greeting: an exactly bounded causal-version frame, then the root-fan listing frame"; `git show 4dd2053c -- start/tests.rs` renamed two sibling tests to the one-item vocabulary and left these five; the module doc at lines 3-5 today says the greeting is "one peer-controlled item")
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired at 4dd2053c
- Owner-gated: no

The names `truncated_version_header_...`, `over_declared_version_frame_...`, `empty_version_frame_...`, `trailing_version_bytes_...`, and `missing_listing_frame_...` describe a wire layout with a separate causal-version frame and root-fan listing frame that no longer exists; `missing_listing_frame_is_a_typed_read_error` cuts the last byte of the single item and involves no listing. AGENTS.md's hard rule forbids prose that refers to code or wire shapes that no longer exist, and a test name is prose a maintainer reads in `nextest` output. The same ghost survives outside the partition at src/tests.rs:262-264, whose doc says "the causal-version frame plus the root-fan listing frame" while its own body comment (272-273) says "measure its one wire item".

Evidence:

        73	#[pollster::test]
        74	async fn truncated_version_header_is_a_typed_read_error() {

        92	async fn over_declared_version_frame_is_a_typed_read_error() {
       114	async fn empty_version_frame_is_a_typed_decode_error() {
       151	async fn trailing_version_bytes_are_rejected() {
       169	async fn missing_listing_frame_is_a_typed_read_error() {

Resolution: Rename to what each test does to the one item: `truncated_item_head_is_a_typed_read_error`, `over_declared_item_length_is_a_typed_read_error`, `empty_item_is_a_typed_decode_error`, `trailing_item_bytes_are_rejected`, `cut_item_content_is_a_typed_read_error`. Sweep src/tests.rs:262-264 in the same commit. Acceptance: `grep -rn -i 'version frame\|listing frame\|version header\|version_frame\|listing_frame\|version_header' src` returns nothing.

Synthesis note: Same residue as remote-proxy-1 and testing-infra-16's second site.

### remote-proxy-8: Session method docs name a `height` parameter that does not exist; `outgoing` takes `&mut self` needlessly; one impl doc calls a fixed terminal "role-specific"
- Where: src/tree/mirror/streaming/remote/proxy/state.rs:60-61 (related: src/tree/mirror/streaming/remote/proxy/state.rs:74-85, src/tree/mirror/streaming/remote/proxy/state.rs:368-370, src/link.rs:216)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the signatures; `outgoing`'s body reads `self.remote`/`self.epoch` and clones `connector`/`stats`/`observe`, and `Connector: Clone` at link.rs:216; `Reply for Descending<S<Z>>` is reached only on the initiator-role chain 31, 29, ..., 1 and its `Next` is `Completing`)
- Seen by: structure, prose; refutation: confirmed, merged; history: no-rationale-found (loose from birth in cbfe1aff3)
- Owner-gated: no

Both method docs say "at `height`" but the height arrives as the type parameter `H`. `outgoing` takes `&mut self` though it only clones fields. The `Descending<S<Z>>` impl says it proxies into "the role-specific terminal", but height 1 is reachable only on the initiator-role chain and its `Next` is fixed to `Completing`; the responder's terminal is reached from `Descending<Z>` via `complete_responder`. A doc naming a parameter the signature lacks sends the reader looking for it.

Evidence:

    60	    /// Bind the incoming logical stream spoken by the remote at `height`.
    61	    fn incoming<H: Height>(&mut self) -> StreamReceiver<A::Rx> {
    74	    /// Bind the outgoing logical stream spoken locally at `height`.
    75	    fn outgoing<H: Height>(&mut self) -> StreamSender<C> {
    368	    type Next = Completing<B, R, W, C, A>;
    369	
    370	    /// Proxy the leaf-parent transition into the role-specific terminal.

Resolution: "...at height `H`." on both; `fn outgoing<H: Height>(&self)`; "Proxy the leaf-parent transition into the initiator's terminal." Acceptance: no doc in state.rs names an identifier absent from its item's signature; `outgoing` takes `&self`.

### remote-proxy-10: Typestate impl docs duplicate the Work method docs they delegate to; the codec doc is written four times with "peer" meaning the local Peer
- Where: src/tree/mirror/streaming/remote/proxy/state.rs:264-270 (related: src/tree/mirror/streaming/remote/proxy/state.rs:296-304, src/tree/mirror/streaming/remote/proxy/work/pump.rs:66-74, src/tree/mirror/streaming/remote/proxy/work/encode.rs:105-124, src/tree/mirror/streaming/remote/proxy/start.rs:53-55, src/tree/mirror/streaming/remote/proxy/start.rs:65-68, src/tree/mirror/streaming/remote/proxy/work.rs:61-64, src/tree/mirror/streaming/remote/proxy/work/pump.rs:428-430)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read all sites; `use crate::message::PayloadCodec;` at work.rs:8 makes the explicit link definition at work.rs:64 redundant; the codec is constructed by the local `Peer`, per 4356e1973's subject, while error.rs:31 and start.rs:306-312 use "peer" for the counterparty)
- Seen by: prose, perfapi; refutation: confirmed both; history: no-rationale-found for the duplication; deliberate-and-holds for "peer" meaning the `Peer` type (the rationale lives only in the commit subject)
- Owner-gated: no

`Initiator::initiator`'s doc and `Work::initiator`'s doc explain the same greeting-borne opening and lazily claimed early stream; `Responder::responder`'s doc repeats what `encode::opening` and `Work::opening_responder` document. The typestate layer's job is wiring; the mechanism lives in `Work`. The codec field doc is written at four sites, each saying "The peer's payload codec" while the same paragraphs use "peer" for the remote counterparty; the codec is this replica's `Peer`'s. Principle: every sentence competes with the contract the reader came for, and two homes for one explanation drift.

Evidence:

    264	    /// Replay the remote initiator's opening question from its greeting.
    265	    ///
    266	    /// The opening question's content already crossed inside the greeting's
    267	    /// listing, so no frame is read here. The initiator-direction opening
    268	    /// stream carries the remote's early supplies instead: its receiver is
    269	    /// bound now and handed to the next stage, which reads (and thereby
    270	    /// claims) it only when a root-level request needs an opening supply.

    pump.rs:68	    /// The question's content — the remote's root-fan listing — already
    pump.rs:69	    /// crossed inside the greeting, so no wire frame exists at this stage:

    work.rs:61	    /// The peer's payload codec: the typed ingress every supplied
    work.rs:62	    /// leaf record decodes through (see [`PayloadCodec`]).
    work.rs:63	    ///
    work.rs:64	    /// [`PayloadCodec`]: crate::message::PayloadCodec

Resolution: keep the first sentence of each typestate impl doc and replace the body with what the impl alone decides (which streams are bound; that `early` is armed here) plus a link to the `Work` method that carries the mechanism. Collapse the codec doc to one sentence at `Work::codec` reading "this replica's `Peer` payload codec" (or code-font `Peer`), drop the explicit link line, and make the other three sites one-line pointers. Acceptance: each mechanism paragraph has one home in the partition; `grep -rn "peer's payload codec" src` is empty; `rg -n 'PayloadCodec\]: crate::message' src` is empty.

### remote-proxy-tests-6: The reordering helper docs describe an effect the property proves unreachable, and its case count is undefended
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:113-122 (related: tests.rs:502-565, 434-443; src/testing/transport.rs:775-781; src/conformance/link/tests.rs:50)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both tests and the helper; the `reorder_accepts` doc in `testing/transport.rs` sanctions "zero as a tripwire where it provably cannot")
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed (deliberate design, owner-gated); history: deliberate-and-holds (cbc4a0aa states the principle: prove the adversity fires, or say it can't and pin that with a tripwire; 59827c7a moved the count onto `CASES`; the link-transport review's R13 cites the tripwire approvingly)
- Owner-gated: yes: deleting the test or restructuring the driver reopens cbc4a0aa; reducing `CASES` changes a decided parameter

The zero-inversion tripwire is a recorded decision and the test's own doc (502-521) is candid that "as exercised this property pins no more than `reconcile_symmetric_accepts`". What the decision does not cover: the `REORDER_BATCH` doc says it is "deep enough to invert most bursts" and the helper doc promises "worst-case-legal stream reordering on both ends", both describing an effect the test proves never happens in this driver, so a reader of the helpers alone is misled; the doc mixes modalities ("provably" at 504, "verified empirically" at 510-511) without saying which is the argument of record; and the 48 cases were chosen as "fewer than the default", not as the minimum that keeps the tripwire alive, although the argument is structural (any session opening two or more streams would show an inversion if the topology admitted one), so one deterministic case suffices to detect a topology change.

Evidence:

       113	/// Arrivals held and released newest-first by the reordering acceptor: deep
       114	/// enough to invert most bursts, small enough that batching never starves a
       115	/// stream.
       116	const REORDER_BATCH: usize = 3;
       117	
       118	/// [`reconcile_symmetric_accepts`] with both acceptors delivering arrivals
       119	/// in reversed batches: worst-case-legal stream reordering on both ends.

       502	/// Wide-budget divergence still matches the materialized oracle with both
       503	/// acceptors decorated for reversed-batch delivery at one-byte windows —
       504	/// with the honest caveat that the reordering provably never fires here.

Resolution: Ungated: rewrite the `REORDER_BATCH` and helper docs to state that no inversion is reachable under the joined-endpoint driver and that the counter's zero is the assertion; settle "provably" versus "verified empirically" by stating the mechanism argument once and calling the zero an observation that pins it. Gated, the owner's choice: keep the property as is; or keep the tripwire as one deterministic case over `early_first_child_dispute_pair` asserting `reordered == 0`, relying on `wide_symmetric_accepts_match_local` for the wide property and the conformance suite's `ReversingAcceptor` for the decorator; or restructure the driver so accepts can batch and flip the tripwire to `> 0`. Acceptance: the helper docs and the test doc agree that no inversion is reachable and say why; either the case count is one with the reason stated, or the current count is defended in the `CASES` doc.

### remote-proxy-tests-11: Test docs narrower than their bodies, one hand-maintained count, and one mux-era clause
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:388-405 (related: tests.rs:316, 343-344; tests/failures.rs:158-166, 275-277, 327; start/tests.rs:277-278, 288, 296-300; tests/greeting.rs:174-178)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read each doc against its body; production call sites of `streaming::handshake` are gossip.rs:1152 and 1210)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: no rationale (`payload_depth_limit: 256` was added by 71de90c1 with no assertion; "close every unused logical stream" dates from cbfe1aff's mux era and expired at b3b877d9, when streams became lazily opened; 6410212a relaxed the counterparty clause for the generated property only)
- Owner-gated: no

`wire_reconciliation_matches_local`'s doc claims observational identity only, but the body also asserts trace validity and one-slot channel bounds. `every_transport_fault_surface_is_reachable` asserts `outcome.right.is_err()` while its doc says nothing about the counterparty and the neighbouring property's doc explicitly permits the counterparty to complete; the mechanism (the fault fires at `after: 0`, before any exchange, so the counterparty cannot have completed) is unstated. `empty_listing_greeting_decodes` says "the decoded handshake carries the sent fields" but checks four of the five it sets, omitting `payload_depth_limit`. `symmetric_accept_handshakes_are_live` says "used by both public API endpoints", a hand count. `equal_versions_return_both_roots` says equal versions "close every unused logical stream", a clause from the multiplexed era; today no stream exists to be closed, and greeting.rs:176-177 asserts `connects == 0` and `accepts == 0`. AGENTS.md: an inaccurate testdoc is a bug in the test; doctrine: no hand-maintained counts.

Evidence:

       388	    /// For arbitrary valid divergence, crossing the codec and per-stream
       389	    /// transport is observationally identical to the in-process protocol.
        ...
       402	        trace.assert_valid();
       403	        assert_proxy_channels_are_bounded(&channels);

    start/tests.rs:
       288	        payload_depth_limit: 256,
        ...
       296	    assert_eq!(handshake.version, version);
       297	    assert_eq!(handshake.set_len, 7);
       298	    assert_eq!(handshake.max_version_bytes, 512);
       299	    assert_eq!(handshake.target_message_size, 1 << 16);
       300	    assert!(handshake.listing.is_empty());

    failures.rs:
       327	        assert!(outcome.right.is_err());

    tests.rs:
       343	/// The same client/proxy pairing used by both public API endpoints remains
       344	/// live under deterministic closed-world polling.

Resolution: Add the asserted clauses to each doc (trace validity and channel bounds; counterparty failure under an immediate fault, with the `after: 0` argument); assert `handshake.payload_depth_limit == 256`; replace "both public API endpoints" with "the pairing the session drivers use"; narrow `equal_versions_return_both_roots`'s doc to root equality with no stream opened. Acceptance: each doc lists every property its body asserts; the greeting test asserts all five sent fields.

### remote-proxy-tests-13: Module docs under-describe their files; `work/tests.rs` has none
- Where: src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:4-5 (related: declarations.rs:87-91, 94, 148; tests/harness.rs:1, 116-282, 284-446, 521-534; work/tests.rs:1)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the three files in full)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale (declarations.rs's doc named the two words that existed at 739e4d1f; the `target_message_size` tests arrived with ed0f1775 without extending it; harness.rs's doc is from 77674c9c and predates the rewriter and the election predicate; work/tests.rs has opened with a `use` since cbfe1aff)
- Owner-gated: no

declarations.rs's module doc lists the size words as `set_len` and `max_version_bytes`, but two tests rewrite `target_message_size`, and the doc at 87-91 itself says that "complet[es] the declaration matrix beside `set_len` and `max_version_bytes`". harness.rs:1 says "for transport-adversity properties" while the file also provides frame scripting, greeting rewriting, and the election predicate used by three sibling suites. work/tests.rs is the only partition file with no `//!` line. A module doc's first sentence stands alone in a listing; a doc naming two of three inputs misleads the reader who came for the third.

Evidence:

    declarations.rs:
         4	//! The greeting's size words — `set_len`, `max_version_bytes` — are
         5	//! peer-declared inputs to the window solve and the role election. These

    harness.rs:
         1	//! Reusable two-proxy session harness for transport-adversity properties.

    work/tests.rs:
         1	use crate::message::{PayloadCodec, PayloadDepthLimit};

Resolution: declarations.rs: name `target_message_size` (the run budget's input) beside the other two. harness.rs: "Two-proxy session harness: link decorators (I/O plans, frame scripts, greeting rewrites), the shared driver, and the role-election predicate." work/tests.rs: a `//!` line naming the `Work` executor's error-selection properties. Acceptance: each doc names what its file contains; `grep -L '^//!' -r --include='*.rs' src/tree/mirror/streaming/remote/proxy` lists no test file.

### remote-proxy-tests-14: Adversary register ("lie", "lied", "deceived") in a suite whose own module doc names the regime buggy-peer
- Where: src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:8-11 (related: declarations.rs:84, 87, 97, 113, 121, 143, 146, 219, 227, 249, 257, 289, 321, 329, 373; start/tests.rs:12, 88, 188-189; 43 further sites in ten files outside the partition, including the `GreetingLie` type in src/tree/mirror/streaming/testing/faulting.rs re-exported at streaming.rs:68)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (grep of `lie|lied|lies|deceived|deceive` as whole words across src and tests; malformed.rs:192 "where they lie" is the verb of position and is excluded)
- Seen by: structure-prose; refutation: confirmed (a prose-vocabulary call); history: no rationale for the survivors (408ede87 chose "mis-declared" as the crate's register for this concept and applied only its victim-to-receiver half to declarations.rs)
- Owner-gated: yes: a crate-wide vocabulary sweep that includes a test-scaffold type name is the owner's prose call

The module doc frames the sessions as "the buggy-peer regime ... (an authorized peer already holds write authority, so none of this is a security boundary)" and in the next sentence says "what a lied declaration costs"; the tests continue with "the deceived side", "undetected set_len lie" panic messages, and "the understated lie". AGENTS.md's model of record puts hostile-peer regimes off-model and makes the violation machinery a conformance-bug detector; lie and deceive presuppose intent the model excludes, so the vocabulary contradicts the paragraph it sits in, and "a lied declaration" is not idiomatic English. The recorded term already exists: 408ede87 established "mis-declared". The same vocabulary appears crate-wide, so a partition-local fix would leave the crate split.

Evidence:

         8	//! declaration the peer's actual traffic does not honor: the buggy-peer
         9	//! regime, exercised as a conformance tripwire (an authorized peer already
        10	//! holds write authority, so none of this is a security boundary). Each
        11	//! test pins what a lied declaration costs the receiving side.

Resolution: Sweep to the recorded term: "misdeclared"/"under-declared"/"a declaration its traffic does not honor"; "the side hearing the misdeclaration" for "the deceived side"; panic messages "undetected set_len misdeclaration: ...". Do it crate-wide in one prose commit, renaming `GreetingLie` in the same pass; drop the secondary moralizers "genuinely batched" (declarations.rs:70) and "genuinely pristine" (greeting.rs:181) while there. Acceptance: `grep -rn -i -w 'lie\|lied\|lies\|deceived\|deceive' src tests` returns only positional uses of the verb.

### remote-proxy-15: Three module-doc sentences misplace where a mechanism lives
- Where: src/tree/mirror/streaming/remote/proxy/work/encode.rs:6-10 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:3-6, src/tree/mirror/streaming/remote/proxy/work/pump.rs:23-25, src/tree/mirror/streaming/remote/proxy/work/pump.rs:98-99, src/tree/mirror/streaming/remote/proxy/work.rs:133-135, src/tree/mirror/streaming/remote/proxy/work.rs:147-150, src/tree/mirror/streaming/remote/proxy/work/queues.rs:8-10)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: the erase-and-box calls are in `Work` methods in pump.rs and `Work::spawn` spawns; `encode::opening/replies/terminal/publish` and `proxy::send_or_cancel` all take `Sender<Scope>`; the third edge's rationale is at `Work::respond`, as queues.rs:8-10 says)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired for pump.rs:24-25 (true when queues.rs held all three constructors; d8bef16b9 moved the relay into `Work::respond` and updated queues.rs but not pump.rs); the other two were loose from birth
- Owner-gated: no

encode.rs says each encoder's request stream is "erased and boxed by the proxy state that spawns it", but the erasure happens in the `Work` methods and `Work::spawn` spawns; state.rs only forwards. pump.rs says "No state outside this module handles an internal sender", which is true only if "state" means the typestate. pump.rs says "Each edge's capacity rationale lives at its constructor in `queues`", but the third edge's rationale is at `Work::respond`. Module docs are the reader's map; a wrong pointer costs a file read per error.

Evidence:

    7	//! Every encoder here consumes the erased vocabulary — its typed request
    8	//! stream is erased and boxed by the proxy state that spawns it — so each

    pump.rs:5	//! receiver-side stream or next-phase scope queue fed by that task. No state
    pump.rs:6	//! outside this module handles an internal sender.
    pump.rs:24	//! precedes its dependent scopes. Each edge's capacity rationale lives at
    pump.rs:25	//! its constructor in [`queues`].

Resolution: encode.rs:7-8: "erased and boxed by the [`Work`] method that spawns it"; pump.rs:6: "The typestates in `state` never hold an internal sender."; pump.rs:24-25: "The two scope edges' capacity rationales live at their constructors in [`queues`]; the response relay's lives at [`Work::respond`]." Acceptance: each sentence names the item that holds the mechanism, checked against pump.rs:98, work.rs:147-150, and the `Sender<Scope>` parameters in encode.rs.

### remote-proxy-16: Temporal "today's" in `opening`'s doc describes a history instead of the behavior
- Where: src/tree/mirror/streaming/remote/proxy/work/encode.rs:123-124 (related: src/tree/mirror/streaming/remote/proxy/work/encode.rs:153-167)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn today` over the partition excluding tests returns only this line)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (55d76d5cf wrote it as a contrast with the pre-change behavior)
- Owner-gated: no

"keeps today's streamless opening" contrasts the present with an implied other state. The present-tense fact is that such a session opens no initiator-direction stream, which the `early` branch at 153-167 shows. Principle: prose speaks in the present tense; dated rationale at a declaration site is the ghost-reference failure in disguise.

Evidence:

    123	/// never opens, and a session without initiator exclusives keeps today's
    124	/// streamless opening.

Resolution: "never opens, and a session without initiator exclusives opens no initiator-direction stream at all." Acceptance: the grep is empty and the sentence states the behavior without temporal contrast.

### remote-proxy-18: trace.rs lacks a module doc and the variant and function docs its sibling in the walk has; "scopes" is used for the questions ledger
- Where: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:8-15 (related: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:1, src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:68-69, src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:180, src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:188, src/tree/mirror/streaming/remote/proxy/work/progress/trace/tests.rs:99-100, src/tree/mirror/streaming/materialized/progress.rs:1-31)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: trace.rs line 1 is `use std::{`, no `//!`; `Kind`'s variants, `new_work`, and `record` carry no docs; materialized/progress.rs opens with a module doc and documents every variant; the height-1 ledger at trace.rs:84-86 counts `LocalQuestion` events)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the walk's sibling was documented at birth; this one never was)
- Owner-gated: no

`Kind`'s payload counts (how many publications must follow) are inferable only from `assert_valid`; the file has no module doc; `new_work` and `record` are undocumented. The bullet at 68-69 (and the mirrored testdoc at trace/tests.rs:99-100) calls the height-1 ledger entries "scopes", but that ledger counts `LocalQuestion` events; in this file's vocabulary "scopes" are `NextScope` events. Maintainer docs state invariants; an instrument whose event vocabulary is undocumented cannot be audited, and the sibling shows the bar.

Evidence:

    8	/// One progress-critical proxy publication.
    9	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
    10	pub enum Kind {
    11	    WireReply { questions: usize },
    12	    LocalQuestion,
    13	    DecodedReply { scopes: usize },
    14	    NextScope,
    15	}
    68	    /// - leaf-height decodes drain both the last internal stage's height-1
    69	    ///   scopes and the terminal height-0 leaf questions, so their bound is

    materialized/progress.rs:1	//! Test-only trace of the walk's progress-critical publications.

Resolution: add a one-line `//!` ("Test-only trace of the proxy's progress-critical publications."), document each `Kind` variant (e.g. `WireReply { questions }`: "one complete wire reply flushed; `questions` publications must follow before the next at this height"), document `new_work`/`record`, and change "height-1 scopes" to "height-1 questions" at trace.rs:68 and trace/tests.rs:99. Acceptance: trace.rs opens with a module doc; every `Kind` variant and pub fn has a doc comment; "scopes" in this file refers only to `NextScope` publications.

### remote-proxy-22: The lint-allow rationale asserts an illumos gate target the tree does not declare, in nine identical copies
- Where: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:146-149 (related: src/tree.rs:632-635, src/tree.rs:672-675, src/tree/mirror/streaming/backend/local/adversarial.rs:22-25, src/tree/mirror/streaming/channel/instrumented.rs:212-215, src/tree/mirror/streaming/materialized/transcript.rs:59-62, src/tree/mirror/streaming/materialized/progress.rs:391-394, src/tree/mirror/streaming/remote/adapter/decode.rs:565-568)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -rni illumos justfile tools/ .github/ .cargo/ Cargo.toml .config/ rust-toolchain.toml AGENTS.md README.md` returns nothing; `grep -rn "illumos among the gate"` returns eight in-scope sites plus one in crates/before, out of scope)
- Seen by: prose; refutation: confirmed (copy count corrected upward); history: deliberate-and-holds (eb4e0e1ba's message describes an illumos gate run on ox-east-1; the practice is real but recorded only in history)
- Owner-gated: no

The comment says the lint misfires under the fallback-TLS lowering "(illumos among the gate's targets)" and that the allow "keeps `-D warnings` honest". Nothing in the justfile, workflows, `.cargo`, `.config`, `rust-toolchain.toml`, or AGENTS.md names illumos; the only declared extra target is `wasm32-unknown-unknown`. A reader cannot check the claim from the tree, and "honest" moralizes a lint. Principle: comments state what the code cannot show and must be checkable against today's tree; a deliberate practice that lives only in history needs an in-tree anchor.

Evidence:

    146	// clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
    147	// fallback-TLS lowering (illumos among the gate's targets) and denies
    148	// initializers that already sit in `const` blocks; the allow keeps
    149	// `-D warnings` honest on every platform the gate runs.

Resolution: either record the illumos gate run where a reader can find it (a justfile recipe or an AGENTS.md line) and cite it, or restate the comment platform-free ("clippy's `missing_const_for_thread_local` denies `const`-block initializers on targets that lower `thread_local!` through fallback TLS; the allow keeps `-D warnings` clean there."). Apply the same text at all eight in-scope sites, or hoist to one note the others point to. Acceptance: the comment cites an in-tree location or makes no platform claim; all sites read identically.

Synthesis note: Same comment as remote-adapter-streams-11, in nine copies; the patterns section lists them all. Open question 11 carries the AGENTS.md half.

### remote-proxy-32: queues.rs points readers to the window docs for "the slack", which the window docs no longer derive
- Where: src/tree/mirror/streaming/remote/proxy/work/queues.rs:31-36 (related: src/tree/mirror/streaming/window.rs:78-126)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i "slack\|undercount\|dequeued\|flushing batch" src/tree/mirror/streaming/window.rs` matches only line 726, "the Bernstein slack", in the statistical envelope section; `git show b76a31f38:...window.rs` carries the derivation at lines 91-95 and `git show d27cb5aa5:...window.rs` does not)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (b76a31f38 wrote both the pointer and the derivation; d27cb5aa5, the same day, rewrote window.rs and dropped the bullet)
- Owner-gated: no

The doc promises that the occupancy bound, its reachability, and the slack are all derived in the window module docs. The window docs carry the bound and the reachability argument; the slack statement exists only here. A cross-reference is a promise; the reader makes the round trip and comes back to the only statement there is, which has the form of a summary of something more rigorous.

Evidence:

    31	/// a full round trip later. Tracks, not equals: occupancy undercounts the
    32	/// wire by a bounded slack (a flushing batch rides the wire before
    33	/// publication; the decoder holds one dequeued entry while its reply
    34	/// decodes). The canonical derivation — the occupancy bound, its
    35	/// reachability, and the slack — is in the
    36	/// [`window`](crate::tree::mirror::streaming::window) module docs.

Resolution: drop "and the slack" from the pointer so this doc is the slack's home ("The occupancy bound and its reachability are derived in the window module docs."), or restore the slack bullet to window.rs's flushed-question section (out of partition). Acceptance: every item the pointer names is present at its target.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| remote-proxy-6 | proxy/start.rs:185-195 | The handshake impls switch from the trait's frame (`theirs`) to the process frame (`local`) with nothing marking the switch. | Rename the parameters, or a one-line comment at the `connected(...)` call. |
| remote-proxy-9 | proxy/state.rs:88-91 | `stream_at`'s expect states the conclusion, not the parity argument that makes `None` unreachable. | Name the parity argument. |
| remote-proxy-tests-19 | proxy/tests/failures.rs:290-292; tests/greeting.rs:238; work/tests.rs:303 | Em-dashes in `//` comments. | Crate-wide pattern. |
| remote-proxy-13 | src/tree/mirror/streaming/remote/proxy/work.rs:221-225 | The `Ok \| Accept` arm comment explains only the `Accept` pattern. | Cover both patterns. |
| remote-proxy-17 | proxy/work/encode.rs:175; proxy/work/queues.rs:17 | "acknowledged questions" where nothing is acknowledged; "decode-side register" for a scope queue. | "flushed questions"; "decode-side scope queue". |
| remote-proxy-26 | proxy/work/pump.rs:193-196 | The comment says the pairing reply "arrives empty"; the code splices without checking, and a duplicate is caught one layer down. | Reword to what the code does, or add the check. |

## Test scaffolding (src/testing, src/tests)

### testing-infra-16: `PREAMBLE_LEN` and `greeting_frame_len` docs describe wire shapes that no longer exist
- Where: src/tests.rs:25-28 (related: src/tests.rs:262-264, 271-272; src/tree/mirror/handshake.rs:12-17, 45-53; src/tree/mirror/streaming/remote/codec/greeting.rs:1-8)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (handshake.rs:45 `const V2_PREFIX: [u8; 11]`, :53 `V2_PREAMBLE_LEN = V2_PREFIX.len() + 1 + (1 + NETWORK_LEN) + 1` with `NETWORK_LEN = 16`, i.e. 30; handshake.rs:16 "the item is 30 bytes, fixed"; `git blame -L 25,28` dates lines 25-26 to f6cf2579 (2026-07-16) and line 28 to 4dd2053c (the CBOR wire); codec/greeting.rs:3 "One control-stream item")
- Seen by: structure-prose [0]; blind-spots [21]; api-economics [33]; refutation: confirmed; history: deliberate-but-expired (the breakdown was maintained through two earlier layout changes; 4dd2053c changed only the constant line in that hunk and left both docs as context)
- Owner-gated: no

The doc sums the preamble as magic(6) + proto_version(2) + network(16) + intent(1) = 25, but the constant it aliases is the 30-byte self-described CBOR item, and none of the four named fields exists in that shape; a reader hand-checking the three fuse budgets gets a wrong number. `greeting_frame_len`'s doc describes "the causal-version frame plus the root-fan listing frame" while the greeting is one tag-24 item and the helper's own comment at 272 says "measure its one wire item". AGENTS.md hard rule and Principle 5: nothing refers to code or layouts that no longer exist; no hand-maintained arithmetic restates what a constant computes.

Evidence:

    25	/// The preamble's wire length: magic(6) + proto_version(2) + network(16) +
    26	/// intent(1). The fault-injection budgets
    27	/// below land cuts on exact protocol boundaries relative to this.
    28	const PREAMBLE_LEN: usize = crate::tree::mirror::handshake::V2_PREAMBLE_LEN;

    262	/// The wire length of `retiree`'s complete greeting — the causal-version
    263	/// frame plus the root-fan listing frame — so a [`Fuse`] budget can land on
    264	/// an exact protocol boundary.

Resolution: Rewrite 25-27 as "The preamble's fixed wire length, the handshake's own constant, so the fuse budgets below land on exact protocol boundaries." (or drop the alias and import `V2_PREAMBLE_LEN`, whose doc at handshake.rs:50-52 carries the correct decomposition). Rewrite 262-264 as "The wire length of `retiree`'s greeting item, the one control-stream item after the preamble, so a fuse budget can land on an exact protocol boundary." Acceptance: neither doc contains a byte breakdown or the word "frame" for the greeting; handshake.rs:16 remains the single place the width is stated.

Synthesis note: Overlaps module-graph-5 on src/tests.rs:25-28 and remote-proxy-tests-1 on src/tests.rs:262-264.

### testing-infra-6: Prose tells across the partition, including an off-model "malicious" and a `Side` doc bound to one retired topology
- Where: src/testing/memnet.rs:9-10 (related: src/tests.rs:86-87, 142, 414, 497, 543-548, 558, 564, 672; src/testing/transport.rs:21-24, 638, 654, 660-665, 675, 688, 701, 744, 749; tests/routed_link.rs:10 (outside this partition, same phrase))
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'genuine\|seam\|honest\|malicious\|silently\|load-bearing'` and `grep -n -w sound` over the four files; each hit read in context)
- Seen by: structure-prose [13]; blind-spots [27]; api-economics [41]; refutation: confirmed ("honest peer" at transport.rs:64 and 616 is the model-of-record term and stays); history: `malicious` predates and contradicts AGENTS.md's model-of-record hard rule (a59dc786d); the `Side` docs date from when the module served only the proxy harness
- Owner-gated: no

One sentence breaches a hard rule: tests.rs:86-87 models the forged retiree as "a buggy or malicious peer", while AGENTS.md puts hostile-peer regimes off-model and makes the overlap check a conformance-bug detector. The rest is default dialect: "seam honest" (memnet.rs:10), "genuine"/"genuinely" seven times in transport.rs and twice in tests.rs, "load-bearing" (662), "silently" without its mechanism (664, 749), "sound" for "correct here" (transport.rs:701, tests.rs:414), "honest divergence" (tests.rs:558, 672), and "parity leg", "tripwires", "seam", "tier" stacked at tests.rs:543-548. `Side`'s variant docs call the endpoints "proxy endpoint[s] in the test harness", but `IoSide::Left/Right` label peers' links in tests/lifecycle.rs:70-71. Writing-style rule: describe code by the property that holds; metaphors only where they rewrite as mechanism.

Evidence:

    9	//! single-poll executor, and names are plain strings, which keeps the
    10	//! address seam honest: nothing here resembles an IP address.

    86	/// we forge it with [`Party::dangerously_alias`] — a copy of the absorber's
    87	/// *exact* region — to model a buggy or malicious peer. The overlap is detected

    21	    /// The first proxy endpoint in the test harness.
    22	    Left,
    23	    /// The second proxy endpoint in the test harness.
    24	    Right,

    662	/// disposition instead of assuming it. The genuine wait is load-bearing: a
    663	/// decorator that only drains arrivals already `Ready` never sees a second
    664	/// arrival under the deterministic scheduler and silently degenerates to
    665	/// pass-through — which is exactly what the asserted counter makes loud.

Resolution: tests.rs:87: "a buggy or nonconforming peer". memnet.rs:10: "names are plain strings, so nothing here can be mistaken for an IP address". transport.rs:21-24: "The endpoint the report attributes operations to; which is which is the test's choice." Define "inversion" once (a released batch of two or more) and drop the "genuine" qualifiers; "waits, yielding, for a further arrival" for "genuinely waits"; "degenerates to pass-through with no failing assertion" for "silently degenerates"; "divergence on both sides" for "honest divergence"; "correct" for "sound"; rewrite tests.rs:543-548 as "the same rejection the mirror suites pin in process and over their wires, observed here through `Rumors::gossip`; the poisoned store is built by a local `Tree::join`, which no session check guards". Acceptance: the grep above returns only "honest peer" at transport.rs:64 and 616; "proxy endpoint" is gone from transport.rs.

### testing-infra-15: The `src/tests.rs` module doc describes only the party tests
- Where: src/tests.rs:1-6 (related: src/tests.rs:446-471, 536-618, 620-624)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (file read in full; the tests at 449, 476, 549, 625, 644, 667 are not about parties)
- Seen by: structure-prose [14]; api-economics [37] (module-doc half); refutation: confirmed; history: deliberate-but-expired (true at bb3c4f2a9; expired across 61bd5de70, 7c9175a0b, 9d35b663c, none of which touched it)
- Owner-gated: no

The charter scopes the file to "party mechanics" reachable only through a forged `Peer` or a party read. The file also holds link-poisoning tests, the uncontained-supply test (which needs `tree::arb::poisoned_root`), and three root-hash meter pins (which need `tree::meter`), and line 623 positions the pins by count ("the two commit-path pins below"). A module doc's first sentence must be true of the module; hand-maintained positional counts rot.

Evidence:

    1	//! Crate-level unit tests for party mechanics that the public integration tests
    2	//! can't reach.
    3	//!
    4	//! They need either a *forged* `Peer` (private fields) or to read a `Peer`'s
    5	//! [`Party`] and compare it to [`Party::seed`]. Both require in-crate access,
    6	//! so they live here rather than in `tests/`.

    623	/// The liveness leg for the two commit-path pins below — a ceiling asserted

Resolution: Restate: "Crate-level tests that need in-crate access: party linearity across bootstrap and retire, retire sessions severed at exact frame boundaries, link poisoning, the containment violation at the API tier, and the root-hash read meter." At 623, name the pins instead of counting them. Acceptance: the module doc names every family of test in the file; no "the N ... below" phrasing.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| testing-infra-1 | src/testing.rs:69 (ten sites); src/testing/transport.rs:667-669 | Rustdoc cites consumers by file path; two divergent acceptors are called "duplicated". | Name consumers by role; state the divergence. |
| testing-infra-7 | src/testing/memnet.rs:116-121, 138 | Diagnostics misname their cause (`Full` and `Closed` share one message); `LISTEN_BACKLOG` has no witness. | Two messages; test the constant with its rationale or delete it. |

## Integration tests (tests/common, then the suites)

### tests-common-11: overlap.rs motivates its harness by an incident and by a data structure the tree no longer has
- Where: tests/common/overlap.rs:9-11 (related: tests/common/overlap.rs:298-302, tests/common/overlap.rs:321-323, tests/common/overlap.rs:414-416, tests/common/overlap.rs:425-427; outside the partition tests/session_overlap.rs:13, 73, 148)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show -s 0b353ffc` names "the imbl OrdMap::diff defect"; `git show -s 8f87ddd0`, nineteen minutes later, removes imbl; `grep -c 'name = "imbl"' Cargo.lock` is 0; `FAN_INLINE = 2` at src/tree/typed/untyped/fan.rs:44; `grep -rn chunk src/tree` finds no fan chunking)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (the "16-entry chunk" is imbl's `OrdMap` node; the dependency left the tree the day the comment was written)
- Owner-gated: no

The module doc and three comments motivate the harness by the narrative of a past defect ("where a real defect lived", "the discovering incident", "the one that found a real bug", "the known defect reproduces") rather than by the failure class it detects, and the preamble-size comment anchors its `12..=48` range to "the root fan to cross one 16-entry chunk". Today's radix fan has no 16-entry chunking and no chunk concept; the 16 was a node size of a dependency the tree no longer has. A maintainer cannot re-derive the range from what is, and the AGENTS.md hard rule (nothing refers to code that no longer exists) is what the parenthetical breaches. "Silently lost" names no mechanism.

Evidence:

         9	//! sessions) in between. That gap is where a real defect lived — its
        10	//! downstream symptom was an innocent leaf silently lost under exactly
        11	//! such an overlap — so overlap is a first-class, deterministically
    ...
       321	            // Preamble size: enough base content to span multiple
       322	            // radix-fan chunks (the discovering defect needed the root
       323	            // fan to cross one 16-entry chunk), sometimes much more.
    ...
       414	/// One deliberate overlap pincer, spliced into the generated soup: the
       415	/// motif distilled from the discovering incident, as choices the shadow
       416	/// processes like any others.

Resolution: restate the motivation forward: the failure class is an install that re-joins a session's fork-time state and must not drop a leaf a concurrent install added, and the pincer is the minimal schedule that puts a mutation between one session's fork and its install. Re-derive the preamble range from today's tree (for example: enough leaves that the root fan has several children, so an install touches a branch node) or drop the parenthetical. Apply the same treatment at 425-427 and at tests/session_overlap.rs:13, 73, 148. Acceptance: no sentence in overlap.rs refers to a past defect, incident, or the number 16; the preamble comment names a property of today's tree or names none.

Synthesis note: prose-hygiene-7 names the same overlap.rs lines among its five sites; this entry carries the imbl provenance and the preamble-range re-derivation.

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

### tests-disruption-handshake-4: The inter-process testdoc counts "four invariants" against a six-item list, and the leg never redacts
- Where: tests/disruption.rs:571-573 (related: tests/disruption.rs:57-74, 512-517, 839-864)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read 57-68, the six numbered invariants; read run_proc_plan 703-865: it asserts the dishonest log empty, probed disjointness, convergence, per-child content survival, and assert_party_invariants, and never calls assert_deletion_honored or assert_value_oracle)
- Seen by: structure-prose [2], api-economics [44]; refutation: confirmed; history: deliberate but expired (accurate at 9eadfc68; 919132dc added invariants 5 and 6 to the intra-process doc without touching this one)
- Owner-gated: no

The doc points at the intra-process list and counts four; the list has six. A reader checking the count finds six and cannot tell which two are meant. The two absent are the ledger checks, and they are absent for a structural reason the doc should state: ChildPlan carries only `n_sends`, so children never redact and the TCP leg exercises neither deletion honoring nor the value ledger. AGENTS.md: no hand-maintained counts; an inaccurate testdoc is a bug in the test.

Evidence:

       571	    /// The same four invariants as the intra-process simulation, with the
       572	    /// fleet split across OS processes gossiping over real TCP sockets
       573	    /// severed at arbitrary byte offsets.

       512	struct ChildPlan {
       513	    n_sends: usize,
       514	    boot: FaultPlan,
       515	    sessions: Vec<FaultPlan>,
       516	    retire: FaultPlan,
       517	}

Resolution: Name the asserted properties instead of counting them: honest errors only, probed disjointness, survivor convergence, and seed fold-join when no hand-off was lost; state that the ledger checks do not run because children keep no ledger and never redact. Separately (owner's call, see open questions) give ChildPlan a redaction step so the TCP leg covers deletion honoring too. Acceptance: no numeral summarizes the list; the doc says why the ledger checks are absent, or they are asserted.

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

### tests-disruption-handshake-9: Ghost reference to "both protocol implementations" after the V1 retirement
- Where: tests/gossip_pipelining.rs:5-8 (related: src/protocol.rs:15-19)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep 'both protocol' tests: only this line; src/protocol.rs declares the single variant `V2 = 2`; git show 368da2a5 -- tests/gossip_pipelining.rs shows that commit edited this file, removing the `Protocol` import and `.protocol(Protocol::V2)`, and left the sentence)
- Seen by: structure-prose [1], api-economics [41]; refutation: confirmed; history: contradicts the AGENTS.md hard rule (accurate from 818a8707, expired at 368da2a5)
- Owner-gated: no

The module doc says the test asserts the window "through both protocol implementations". Since 368da2a5 there is one; the sentence names code that no longer exists and misdescribes what the test exercises. AGENTS.md hard rule: nothing in the codebase refers to code that no longer exists.

Evidence:

         5	//! instead of tree depth. The window (set through
         6	//! [`Peer::sync_memory_budget`]) is the fix, and this test asserts it
         7	//! end-to-end — from the public knob, through both protocol implementations, to
         8	//! the channels — by gossiping over a delayed-pipe link on a paused-clock

Resolution: Rewrite as "from the public knob, through the streaming mirror, to the channels" (or drop the middle clause). Acceptance: `grep -rn 'both protocol' tests` returns nothing.

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

Synthesis note: The sibling ghost at src/tree/mirror/streaming/tests.rs:180 is streaming-tests-5's second site.

### prose-hygiene-3: tests/listen.rs testdocs open with §6.N section numbers of a plan deleted from the tree
- Where: tests/listen.rs:73 (related: tests/listen.rs:109, 137, 164, 215, 241, 276, 298, 330, 357, 380, 464, 555, 612)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn '§' tests` lists the fourteen sites; `git log --all --oneline -S'Genesis replay'` gives 480d232af "broadcast-listen plan: version-cursor architecture with listen_from", 040a0d04 "tests/listen.rs: the Messages observer contract (plan §6)", and 12f9b85e9 "Remove old and intermediate artifacts"; `grep -rlE 'Genesis replay|observer contract' design .agent-notes formal` returns nothing)
- Verification: confirmed; the plan's provenance is now known: it existed as a design doc (480d232af) and was deleted in 12f9b85e9, so the prefixes are ghosts as well as opaque; history: deliberate-but-expired (the plan was deleted deliberately; the tags were not re-denominated)
- Owner-gated: no

Fourteen test doc comments begin with "§6.1" through "§6.12", indexing a
plan that was deleted from the tree. As the first sentence of each testdoc
they also fail the standalone-first-sentence rule for module listings.

Evidence:

    tests/listen.rs
    73	/// §6.1 Genesis replay: a from-genesis observer on a populated set yields
    109	/// §6.2 Arbitrary start: `unordered_messages_since(v_mid)` observes exactly
    555	    /// §6.5 Exactly-once under interleaving: across an arbitrary

Resolution: Strip the "§6.N " prefix from each of the fourteen testdocs,
keeping the title phrase as the first sentence. If the coverage map matters,
list the observer-contract clauses in the module doc in English, without
numbers. Acceptance: `grep -c '§' tests/listen.rs` is 0.

Synthesis note: Same fourteen sites as tests-observation-15, which adds the plan's provenance (plans/broadcast-listen.md, deleted in 12f9b85e9).

### tests-observation-15: Fourteen testdocs in listen.rs open with `§6.n` tags from a plan file deleted from the tree
- Where: tests/listen.rs:73-75 (related: tests/listen.rs:109, 137, 164, 215, 241, 276, 298, 330, 357, 380, 464, 555, 612)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -n '§' tests/listen.rs` returns exactly these fourteen lines; `git log -S'§6.1'` gives 040a0d042 "tests/listen.rs: the Messages observer contract (plan §6)", whose body cites plans/broadcast-listen.md; `git log --diff-filter=D -- plans/broadcast-listen.md` gives 12f9b85e9 "Remove old and intermediate artifacts")
- Seen by: structure-prose [0], api-economics [46]; refutation: confirmed (fourteen sites, not thirteen); history: contradicts hard rule (one correction: the repeated 6.6 and 6.9 tags mirror the plan's own "Negative control" and "Variant" sub-items, so they are orphaned, not disordered)
- Owner-gated: no

The tags are section numbers of `plans/broadcast-listen.md`, which the suite's own commit cites and which was deleted two days later. Nothing in the tree resolves them; they are opaque roster IDs to every reader today, and they break the doc's first sentence in a module listing. Hard rules: no design-document citations from code; no opaque roster IDs. The English titles that follow each tag already carry the meaning.

Evidence:

    73	/// §6.1 Genesis replay: a from-genesis observer on a populated set yields
    74	/// exactly the live set, each message once, then goes quiet; after the
    75	/// completed pass its checkpoint dominates every observed version.

Resolution: Delete the `§6.n ` prefix at each of the fourteen doc comments, keeping the English title; the parentheticals "(retire variant)" and "(negative control)" may stay as plain words ("Retire variant of termination: ...", "Negative control: ..."). Acceptance: `grep -c '§' tests/listen.rs` prints 0 and every affected doc still opens with its English title.

Synthesis note: Same fourteen sites as prose-hygiene-3.

### tests-observation-18: Ghost vocabulary from the dissolved lending forms: "borrowed faces", "lent out", `lent_borrows_do_not_block_senders`
- Where: tests/listen.rs:330-334 (related: tests/listen.rs:28, tests/listen.rs:339-340, tests/listen.rs:352-353, tests/causal.rs:26, src/rumors/unordered.rs:192, src/rumors/causal.rs:152)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show cd7c09db` "observers: dissolve the lending forms" removed `borrow_next` and the lending `TryNext`; both observers' `Stream::Item` is the owned `(Version, Arc<T>)`; `grep -n 'lent\|borrow' tests/listen.rs tests/causal.rs` hits only these sites plus the unrelated `redactions_are_honored_silently`)
- Seen by: structure-prose [1]; refutation: confirmed; history: contradicts hard rule (cd7c09db rewrote the `borrow_next` mentions and one inline comment in these suites but left the `Step` docs, the test name, its doc sentence, the `lent`/`lent_value` bindings, and the assert message: an incomplete sweep inside the removing commit)
- Owner-gated: no

Nothing in the API these tests exercise borrows or lends: `step` copies out of an owned item, and the test holds an owned `Arc`. The `Step` doc in both suites still says "with the borrowed faces cloned out", and this test is named for lent borrows with a doc about an item "still lent out". A reader looks for the lending API to understand what "lent out" means and finds none. Hard rule: nothing refers to code that no longer exists.

Evidence:

    28	/// One observer step, with the borrowed faces cloned out.

    330	/// §6.12 Non-blocking observer: an observer mid-pass — its most recent item
    331	/// still lent out — holds no lock, so sends on the set proceed and the
    332	/// observer sees their effects on its next passes.
    333	#[test]
    334	fn lent_borrows_do_not_block_senders() {

Resolution: Rename to `mid_pass_observer_does_not_block_senders`; reword the doc to "an observer mid-pass, one of the pass's items handed out and still held, holds no lock..."; rename `lent`/`lent_value` to `held`/`held_value` and the assert message to match. In both suites change the `Step` doc (causal.rs:26, listen.rs:28) to "One observer step, with the item's payload copied out of its `Arc`." (moot if tests-observation-2 lands). Acceptance: `grep -n 'lent\|borrow' tests/listen.rs tests/causal.rs` matches only `redactions_are_honored_silently`.

### tests-lifecycle-19: partition.rs's module doc names an `on_message` callback and a `key` that exist nowhere in the crate
- Where: tests/partition.rs:9-19 (related: tests/shadow_validity.rs:15-16, tests/common/peer.rs:6-14, tests/common/schedule/executor.rs:95-100, tests/common/schedule/executor.rs:212-222)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn on_message src tests examples benches README.md AGENTS.md` hits only tests/partition.rs:12 and tests/shadow_validity.rs:16; `grep -rn 'pub struct Key\b\|pub type Key\b\|enum Key\b' src/` is empty; 9c73d7b463 is "api: retire Key; a message's public identity is its Version"; observation is pull-based through `Peer::drain` and the executor gates redacts on `peer.observations` at executor.rs:214)
- Seen by: structure-prose, blind-spots, api-economics (on_message); the `key` ghost is new in this pass; refutation: confirmed; history: contradicts-hard-rule (the callback API left the crate at db32b94d9 on 2026-06-10; 9c73d7b463 re-wrapped the sentence for the Key retirement on 2026-08-18 and kept both ghosts)
- Owner-gated: no

The argument for self-consistency over twin comparison is sound and worth keeping, but its referents are stale twice over: a redact is gated on the peer having "received the targeted message via an `on_message` callback", and a partitioned schedule may suppress redacts "because the targeted peer hasn't observed the key yet". Neither `on_message` nor `Key` exists; the harness observes by pull (`Peer::drain`) and identity is the `Version`. The executor's own doc (executor.rs:95-100) states the rule correctly. AGENTS.md's hard rule: nothing in the codebase refers to code that no longer exists. The paragraph also opens in the first person plural, which no other module doc in the partition does.

Evidence:

         9	//! We deliberately do *not* compare against an unrestricted run of
        10	//! the same schedule. Doing so would assume order-independence of
        11	//! redactions, but a redact event can only happen at peer `P` once
        12	//! `P` has already received the targeted message via an `on_message`
        13	//! callback —
        14	//! which is a function of the gossip schedule. A partitioned schedule
        15	//! may legitimately suppress some redacts (because the targeted peer
        16	//! hasn't observed the key yet), so the two schedules can converge

Resolution: Rewrite in terms of what is: "a redact event can only fire at peer `P` once the targeted message appears in `P`'s observation log (`Peer::drain`), which is a function of the gossip schedule. A partitioned schedule may legitimately suppress some redacts (the redacting peer has not yet observed the version), so ..." and drop "We". Fix the same `on_message` ghost at tests/shadow_validity.rs:16 in the same pass. Acceptance: `grep -rn on_message tests/ src/` returns nothing; no partition file uses "key" for a message identity.

Synthesis note: The `on_message` ghost recurs at tests/shadow_validity.rs:15-17 (tests-observation-27); one pass fixes both.

### tests-lifecycle-22: retire.rs prose describes a domination precondition and declines the `Retire` contract does not have
- Where: tests/retire.rs:11-12 (related: tests/retire.rs:114-115, tests/retire.rs:143-145, tests/retire.rs:155, tests/retire.rs:159-161, tests/retire_snapshot.rs:73-74, src/peer/gossip.rs:101-103, src/peer/gossip.rs:453, src/peer.rs:285-287)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -n dominat src/peer/gossip.rs` hits only two bookmark lines, 519 and 650; `Retire::Declined` is produced solely by `(Intent::Remain, Ok(_))` at gossip.rs:453 and documented as "The peer was itself retiring" at 101-102; peer.rs:285-287 states reconcile-then-absorb; fedb3ecb2 (2026-06-09) is "Retire now reconciles before relinquishing the party")
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (143-145 were true when written on 2026-06-08; fedb3ecb2 removed the precondition the next day; the header's "remain" and 159-161's "is not declined" were written after the removal as contrast prose against the old design)
- Owner-gated: no

Five passages describe retirement as gated on the absorber causally dominating the retiree, with divergence "not declined": the header's "Declines remain only" (negative space against a design that declined more), "equal versions, so it reflexively dominates" (114-115), "the `<=` domination precondition" (143-145), "equal versions dominate reflexively, so retire commits" (155), "A retiree whose peer does *not* dominate it ... is not declined" (159-161), and retire_snapshot.rs:73-74's "the absorber dominates reflexively". The public contract has no such precondition: a retire session reconciles exactly as gossip would and then the peer absorbs the identity; `Declined` means only that the counterparty was itself retiring. Prose speaks in the present tense: a testdoc states today's invariant, not what the test would have caught under a design the tree no longer has. (Line 6's "comes to causally dominate the retiree before the party changes hands" is a fine mechanism description and needs no change.)

Evidence:

        11	//! version are untouched). Declines remain only for a counterparty that is
        12	//! itself retiring; a bootstrapping counterparty *absorbs* the retiree —

       143	/// Equal versions satisfy the `<=` domination precondition reflexively: a
       144	/// fresh, empty bootstrap fork can retire into the peer it forked from with
       145	/// no prior gossip.

Resolution: Restate positively: the header says a retire reconciles first and then absorbs, and declines only a retiring counterparty (drop "remain"); `empty_equal_version_retire_succeeds` becomes "an empty fork retiring into its parent is the minimal session: no content moves, the party is absorbed"; `divergent_retiree_reconciles_then_retires` drops "is not declined" and "dominate"; retire_snapshot.rs:73-74 drops "dominates reflexively". Acceptance: `grep -n 'dominat\|not declined\|remain only' tests/retire.rs tests/retire_snapshot.rs` returns only mechanism descriptions of the in-session reconciliation, none naming a precondition or a decline the enum does not have.

### tests-resource-link-window-29: window_operator's module doc states the wire-law intercept as 28 where the crate ships 43; window_knee carries the matching stale ~36 B arithmetic
- Where: tests/window_operator.rs:3-10 (related: tests/window_knee.rs:314-317, src/tree/mirror/streaming/window.rs:221, src/peer.rs:400, src/peer.rs:455, tests/dispute_wire.rs:105, tests/dispute_wire.rs:115, tests/tradeoff_probe.rs:137-139)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git grep -n '28 + m'` finds only tests/window_operator.rs:4 and :9 in the tree outside .agent-notes; window.rs:221 `pub(crate) const DISPUTE_OVERHEAD_BYTES: usize = 43;`; peer.rs:400, 403, 431, 455 quote `(43 + m)`; `git show --stat 4dd2053c` touches peer.rs and window.rs and no tests/window_*.rs; `git blame` dates window_operator.rs:4 and :9 to 2026-07-23/24; window_knee.rs:315 dates to 90a309e0b1, 2026-07-23)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (28 was the constant when written; it moved 28 to 34 to 35 to 43 across 2d1e6ea51, ba8045c5, 4dd2053c; e59b2292 swept the identical drift out of tradeoff_probe by reading the accessor and did not reach these two files)
- Owner-gated: no

The module doc quotes the closed form and the per-message wire law with the constant 28, twice; the crate's calibrated intercept is 43 and `Peer::sync_memory_budget`'s rustdoc says so. The test body is unaffected (it reads `window_capacities` and `supply_decode_envelope_bytes` at runtime), which is exactly why the drift went unnoticed. window_knee.rs:315 reasons from "~36 B each" (the borsh-era 28 + 8) to "~57 messages" per 2 KiB; today's minimal cell reads 43 + 9 - 1 = 51 B (`U64_ENCODED_BYTES` 9 and `MINIMAL_CELL_RESIDUAL` 1 in tests/dispute_wire.rs), about 40 messages. Hand-maintained numbers rot silently (Principle 5); the crate's own convention at tradeoff_probe.rs:137-139 ("The shipped intercept, never a transcribed copy") is the ruled pattern.

Evidence:

     3	//! `sync_memory_budget`'s docs publish the closed-form estimate,
     4	//! `slowdown(budget, m) ≈ max(1, BDP × envelope / (budget × (28 + m)))`:

     9	//! `BDP_messages = BDP / (28 + m)`, the calibrated per-message wire law
    10	//! `tests/dispute_wire.rs` pins. The `K` substitution overstates the

    window_knee.rs:
   314	    // 2 KiB in flight at 10 ms one-way models ~200 KB/s per stream: a
   315	    // BDP of ~2 KiB — ~57 messages at this corpus's measured ~36 B each
   316	    // (tests/dispute_wire.rs pins the affine per-message cost) — below
   317	    // the binding capacity, so bandwidth binds before the window does.

Resolution: At window_operator.rs:4 and :9, write the law in terms of the named constant (`DISPUTE_OVERHEAD_BYTES + m`, as window.rs:233-234 does) or as "the intercept `dispute_overhead_bytes()` exposes", and point at `Peer::sync_memory_budget` for the published form. At window_knee.rs:314-317, either excise the worked figures and keep the conclusion ("a BDP in messages below the binding capacity, per the intercept `tests/dispute_wire.rs` pins") or compute the BDP in messages from `dispute_overhead_bytes()` in code and assert it is below `capacity`. Acceptance: `grep -rn '28 + m\|~36 B' tests/` returns nothing; every wire-law figure in test prose is stated by constant name or derived from a `testing` accessor.

### tests-common-9: the composition map omits `overlap` and `shape`
- Where: tests/common/mod.rs:6-29 (related: tests/common/mod.rs:40, tests/common/mod.rs:44)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (compared the twelve linked modules in 6-29 against the fourteen `pub mod` lines at 35-48)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale (each module landed as a one-line `pub mod` addition; the header rewrite at 0d48b153 postdates `overlap` and did not add it)
- Owner-gated: no

The "How the pieces compose" list is a hand-maintained enumeration of module contents, the kind the doctrine warns rots silently, and it has: `overlap` (the only harness that samples chosen interleavings) and `shape` (fixture staging for the wire pins) are absent. A reader using the map as the tour misses two of fourteen modules.

Evidence:

        28	//! - [`gossip_snapshot`] captures a session's exact bytes for the `insta`
        29	//!   pins.

        40	pub mod overlap;
    ...
        44	pub mod shape;

Resolution: add a bullet for each (`overlap`: a session parked at a chosen poll prefix while other events run, with its own valid-by-construction generator; `shape`: deterministic tree-shape staging for the pin fixtures). Alternatively restructure the map to state composition relations only and let each module's first doc sentence carry its role, so the list cannot rot. Acceptance: every `pub mod` in mod.rs appears in the map, or the map no longer enumerates modules.

### tests-common-13: `Step { polls }` and `SESSION_POLL_BOUND`'s panic count alternation rounds, not polls
- Where: tests/common/overlap.rs:181-182 (related: tests/common/overlap.rs:59-60, tests/common/overlap.rs:120-127, tests/common/overlap.rs:153-159, tests/common/overlap.rs:494-497)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `Session::step`: `n` rounds of one poll per unfinished side, so up to `2n` polls)
- Seen by: api-economics; refutation: confirmed; history: no rationale (both docs unchanged since 0b353ffc)
- Owner-gated: no

The event doc says the session is polled "at most `polls` times"; `step(n)` runs `n` alternation rounds of one poll per unfinished side. `SESSION_POLL_BOUND` is documented as "Alternation rounds" but `finish`'s panic message reports it as "polls". A counterexample reading `Step { polls: 3 }` should mean what the executor does.

Evidence:

       181	    /// Poll the session in `slot` at most `polls` times.
       182	    Step { slot: usize, polls: usize },

       120	    /// Drive at most `n` alternation rounds — one poll of each unfinished
       121	    /// side per round — returning `true` once both sides have completed.

       156	            "overlapped session did not complete within {SESSION_POLL_BOUND} polls: \
       157	             a protocol deadlock"

Resolution: rename the field to `rounds` (and `Pincer.park`'s doc accordingly) or reword the doc to "at most `polls` alternation rounds (one poll of each unfinished side per round)"; say "rounds" in the panic message. Acceptance: the field doc, `Session::step`'s doc, `SESSION_POLL_BOUND`'s doc, and the panic message use one unit.

### tests-common-23: "honest" already names the model of record; here it names a different predicate
- Where: tests/common/sim.rs:359-385 (related: tests/common/sim.rs:188, tests/common/sim.rs:390-399, tests/common/sim.rs:412, tests/common/sim.rs:422-423, tests/common/sim.rs:453, tests/common/sim.rs:663, tests/common/sim.rs:942, tests/common/wire.rs:25, tests/seed_liveness.rs:347; callers in tests/disruption.rs)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rniw 'honest\|dishonest\|honesty'` over the partition lists the sites; the word appears in 39 files under src/ and six other test binaries)
- Seen by: structure-prose; refutation: confirmed, and noted the vocabulary is crate-wide; history: no rationale (dates to the engine's first commit)
- Owner-gated: no, but the vocabulary should be ruled crate-wide

AGENTS.md names the model of record "authenticated-honest-peer": there, honest means non-adversarial, and the word is a term of art the crate uses widely in that sense. sim.rs's `is_honest_error` decides something else: whether an error is attributable to the injected cut (a truncation or a severed-transport kind) rather than a decode or protocol failure. The same word carries a second meaning in the one place the first meaning matters most (the engine that must stay on-model). The writing rule says to name the property; here the property is "attributable to the cut". Two further sites use the word for unrelated things: wire.rs:25 ("keeps `-D warnings` honest") and seed_liveness.rs:347 ("the honest seed stays").

Evidence:

       359	// ---- honesty of failures ---------------------------------------------------
    ...
       361	/// Assert `e` is an injected I/O fault that *truncated* a frame: the only
       362	/// error an honest, single-universe simulation can surface.
    ...
       385	pub fn is_honest_error(error: &Error) -> bool {

Resolution: rename to the mechanism (`is_injected_cut`, `assert_injected_cut`, `cut_io`, `cut_remote`, `assert_session_cut_or_ok`) and write "attributable to the cut" / "not attributable to a cut (a decode or protocol failure)" in prose; update the callers in tests/disruption.rs. Rule the vocabulary once for the crate (src/peer/gossip/tests.rs and six binaries share it) rather than renaming here alone. Rewrite wire.rs:25 ("keeps `-D warnings` clean") and seed_liveness.rs:347 ("the `.txt` seed stays"). Acceptance: no identifier in tests/common uses honest/dishonest/honesty; prose uses the word only in the model-of-record sense.

### tests-common-28: "the *asynchronous* gossip path" qualifies against a twin that no longer exists
- Where: tests/common/wire.rs:1-6 (related: tests/async_wire.rs:1)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S'asynchronous' -- tests/common/wire.rs tests/simulation/wire.rs` bottoms out at 691909e79 "Split wire-equivalence tests into async and sync binaries"; `ls tests | grep -i sync` returns only async_wire.rs)
- Seen by: api-economics; refutation: confirmed; history: deliberate-but-expired (the sync twin was deleted at 83edcd94 and its retirement recorded at 9d9eaac0; neither header was re-denominated)
- Owner-gated: no

The emphasized qualifier distinguished this module from a synchronous wire binary that was deleted; no sync binary and no synchronous gossip path exist, so it sends a reader looking for the other path.

Evidence:

         1	//! Wire helpers for the *asynchronous* gossip path.
         2	//!
         3	//! These drive `rumors::Rumors::gossip` over an in-memory [`rumors::link`]
         4	//! pair with both peers polled concurrently via `tokio::join!`. The two
         5	//! tasks progress directly against each other through the link's streams;
         6	//! no runtime is required unless a caller explicitly spawns a task.

Resolution: retitle ("In-memory gossip and bootstrap drivers under the closed-world poller") and drop the qualifier from tests/async_wire.rs:1 (that binary's overlap with tests/pairwise.rs is a finding for the partition owning the test binaries). Acceptance: `grep -rn asynchronous tests/` returns nothing about a gossip path.

### tests-observation-38: Seed files carry a parked "awaits owner disposition" note and past-tense "minted" provenance comments
- Where: proptest-regressions/shadow_validity.txt:9-14 (related: proptest-regressions/session_stats.txt:7, proptest-regressions/tree/mirror/streaming/tests/stats.txt:7, proptest-regressions/tree/mirror/streaming/tests/faults.txt:11, proptest-regressions/tree/mirror/streaming/tests/faults.txt:13, proptest-regressions/tree/mirror/streaming/materialized/work/tests/violations.txt:7, proptest-regressions/tree/mirror/streaming/remote/codec/decode/tests.txt:7)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both partition seed files; `grep -rn minted proptest-regressions/` lists six files; `git log -S'awaits owner disposition'` gives 390160a76 on 2026-08-13; `git show -s 2c73d032` records the "mint" purge's carve-out)
- Seen by: blind-spots [33] [35], api-economics [49]; refutation: confirmed, with two corrections carried here (the disposition note is from 390160a76, not ce3664dd, which moved the file untouched; 2c73d032 deliberately exempted the session_stats comment as "a committed seed artifact that stays byte-stable"); history: already-known on both halves (a ruling pending since 2026-08-13; a ruling that undercounted six occurrences as one and whose byte-stability premise concerns stripping seeds, which editing a `#` comment does not do)
- Owner-gated: yes: both halves are recorded rulings

The third `cc` line in shadow_validity.txt carries a note saying it predates the current `Schedule` shape, "no longer replays" the failure it was written for, and "awaits owner disposition". Proptest reads only the hash, so the line still seeds one deterministic case and is not dead; but the tree holds a dated, undisposed note, and "awaiting" is not a state the tree may hold: every contradiction resolves to a fix or a declared model. The session_stats seed's comment explains the seed's origin in past tense with "minted", vocabulary the crate purged; the purge exempted it on a premise (seed byte-stability) that AGENTS.md attaches to `cc` lines, not comments, and five sibling seed files carry the same pattern.

Evidence:

    9	# The next entry predates the current Schedule strategy shape (its shrink
    10	# note lacks fork_parents), so it no longer replays the failure it was
    11	# written for. It awaits owner disposition; do not strip it without a
    12	# ruling. Proptest reads only the hash before the first '#' on a cc line,
    13	# so this comment and the stale shrink note cost nothing at replay.

    (proptest-regressions/session_stats.txt)
    7	# The case below was minted against a deliberately unwired recorder during red-first verification; it passes on real code.

Resolution: Rule now. Recommended: keep the `cc 86723fa8...` hash as a valid deterministic replay case under the current strategy, delete its stale shrink note after the `#` and the five-line comment, recording the ruling in the commit message; for the six "minted" comments, either delete them (git has the provenance) or restate each in the present tense ("# A deterministic minimal replay case; passes on the current code."). Acceptance: no comment in any seed file speaks of pending disposition; `grep -rn minted proptest-regressions/` is empty; `tests/seed_liveness.rs` still passes.

### tests-resource-link-window-4: "byte-for-byte" attributed to a hash that is blind to message bytes
- Where: tests/async_wire.rs:26-27 (related: tests/async_wire.rs:39-41, src/tree.rs:20-22)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (src/tree.rs:20-22 reads "a pure function of the version set, blind to message bytes"; `git log -S'blind to message' -- src/tree.rs` gives 961f63c6, 2026-08-18, after the helper's doc was written)
- Seen by: blind-spots; refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

The helper's doc says equal hashes mean the pair "agrees byte-for-byte"; since 961f63c6 the Merkle hash is a function of the version set alone. The byte-level agreement is what the `readout` equalities above establish; the hash pins the version set. An inaccurate testdoc is a bug in the test. Moot if the file is deleted per finding 3, but the same sentence should not migrate.

Evidence:

    26	/// The converged pair agrees byte-for-byte and causally: equal observable
    27	/// hashes and equal `latest` versions.

Resolution: State what the two checks pin: equal version sets (`hash`) and equal causal frontier (`latest`); fix the echo at lines 39-41 ("byte-identical (`hash`)"). Acceptance: no doc in the file claims byte identity from the hash.

### tests-bookmark-1: Module doc counts "two corners" over three tests
- Where: tests/bookmark_attach.rs:4-8 (related: tests/bookmark_attach.rs:51, 78, 139-140)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (counted the `#[test]` functions at L51, L78, L139; `git log -S'two corners'` resolves to 624917eb, whose message names all three tests)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (the count was incomplete at birth, not rotted)
- Owner-gated: no

The module doc names two corners of the attach contract, but the file holds three tests, and the third, `failed_attach_does_not_reclaim_into_an_unbookmarked_peer`, pins the reclaim-at-attach hazard that is neither named corner. A hand-maintained count in the reader's map of the file omits its most intricate test (Principle 5: no hand-maintained counts).

Evidence:

         4	//! The property suite in `bookmark_causality.rs` exercises the eager-persist
         5	//! and fault paths in aggregate; these are point assertions on the two corners
         6	//! of the attach contract that suite never names directly: that a pristine seed
         7	//! is persisted lazily (no write at attach time), and that a failed persist
         8	//! hands the peer back intact for a retry rather than stranding its identity.

Resolution: drop the count and name all three claims (lazy persist for a pristine seed; a failed persist returns the peer intact; a failed attach reclaims nothing into the returned peer), or state that these are point tests on the attach contract's corners and let the testdocs enumerate. Acceptance: the module doc's list matches the `#[test]` functions in the file.

### tests-bookmark-7: Module doc claims wire faults interrupt hand-offs; every hand-off runs on a clean link
- Where: tests/bookmark_causality.rs:10-11 (related: tests/bookmark_causality.rs:514, 518, 563, 581, 673, 721)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'fault::faulty'` in the file returns L514 and L518 only, both inside `gossip`; the bootstrap link at L581 and the retire link at L673 are bare `memory_with_capacity`)
- Seen by: blind-spots; refutation: confirmed; history: no rationale found (inaccurate since birth)
- Owner-gated: no

The doc says wire faults sever sessions "so messages are lost and hand-offs are interrupted", but only `World::gossip` wraps its link in `fault::faulty`; `bootstrap_into` (whose own doc at L563 says "The bootstrap's wire is clean") and `retire` use unfaulted links, and a mismatched gossip resolves through `bootstrap_into`. No party hand-off is ever wire-interrupted in this suite; hand-offs fail only through bookmark faults. The doc credits the `Retire::Uncertain` arm at L721 with coverage that does not exist.

Evidence:

        10	//! - **wire faults** — sessions severed at arbitrary byte offsets (reusing
        11	//!   [`common::fault`]), so messages are lost and hand-offs are interrupted; and
        ...
       673	            let (ret_side, abs_side) = rumors::link::memory_with_capacity(LINK_BUF);

Resolution: either reword the doc to say only plain gossip sessions are wire-faulted and hand-offs are interrupted solely by bookmark faults, or wrap the retire link in `fault::faulty` with a generated `FaultPlan` (as `tests/common/sim.rs::arb_retire` does) while keeping the reliable variant clean so `assert_no_leak` stays sound. Acceptance: the module doc's fault taxonomy matches the link wrapping at L514-518, L581, and L673.

### tests-bookmark-16: Test prose narrates the bugs it once caught instead of stating the invariant
- Where: tests/bookmark_causality.rs:1081-1087 (related: tests/bookmark_causality.rs:696-705, 1156-1157, 1263-1267, 1269-1270, 1290-1292; tests/bookmark_transmit_window.rs:313-320; crates/before/src/lib.rs:417)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `fixed|motivated|Historical|Reconstructed|the fix|Diagnostic` resolves to the cited lines; `mod codec;` is private at crates/before/src/lib.rs:417)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: contradicts AGENTS.md hard rule 1 on its purpose clause ("provenance lives in git history"); the letter enumerates other words, so the owner may read it as Principle 5; "Reconstructed" is 2c73d032's mechanical rename of "Re-minted", not an endorsement of the wording
- Owner-gated: no

The regression test's doc narrates the `Party::join` defect it caught ("left stale bits", "With the `join` normalization fixed") rather than the invariant and the shape that triggers it, and cites `before::codec`, a private module of a sibling crate. The same pattern recurs: "exactly what hid the codec leak this test was written to catch" and "the non-canonical party that motivated this check" (L696-705), "Diagnostic helper" on a fixture (L1157), "Historical shrunk counterexamples" (L1263), "Reconstructed counterexample" (L1269, L1290), and "the in-flight-window fix" (transmit L319-320). Prose speaks in the present tense: state what must hold and what shape exercises it; provenance lives in git.

Evidence:

      1081	/// Both peers reclaiming once grew the retiree's donated party via
      1082	/// [`Party::join`](before::Party), which left stale bits in its `as_bytes`
      1083	/// encoding; the absorber's session then aborted decoding it (`before::codec`
      1084	/// `TrailingBits`) while the retiree reported [`Retire::Retired`], having
      1085	/// already shipped and sliced away its party — so the donated region was held
      1086	/// by no one: a leak. With the `join` normalization fixed, the absorption
      1087	/// lands.
        ...
      1263	// Historical shrunk counterexamples for the property above, preserved as
      1264	// explicit constructions: their committed seeds regenerate gossip fault
        ...
       704	        // fully-received frame was malformed — a protocol/codec bug like the
       705	        // non-canonical party that motivated this check — so surface it loudly.

Resolution: rewrite L1075-1092 as the invariant (retiring into an absorber that has itself reclaimed from a bookmark absorbs cleanly and leaves the absorber holding `Party::seed()`), the trigger (both peers have reclaimed; one side alone does not exercise it), and why it lives at the library boundary (no harness). Reword L696-705 to state the classification rule without the incident. Replace L1263-1267 with the live mechanism ("these plans pin the shapes below explicitly; the committed seeds regenerate through the strategy's cut range, so a range change re-maps them") and open L1269 and L1290 with the invariant. Drop "Diagnostic helper" and "the in-flight-window fix" (name the schedule the test does not cover instead). Acceptance: `grep -n 'fixed\|motivated\|Historical\|Reconstructed\|the fix\|Diagnostic helper' tests/bookmark_*.rs` returns nothing.

Synthesis note: Lines 1263-1267 are also prose-hygiene-7's fifth site.

### tests-bookmark-19: Testdoc says "cut in both directions" but both cuts sit on the node-1-to-node-2 direction
- Where: tests/bookmark_causality.rs:1290-1292 (related: tests/bookmark_causality.rs:1304-1311; tests/common/fault.rs:42-47; proptest-regressions/bookmark_causality.txt:8)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `FaultPlan`'s field docs and compared them with the plan literal and the committed seed's plan, which match)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (6d48d8dc's wording)
- Owner-gated: no

The plan gives node 1 `write_cut: Some(1324)` and node 2 `read_cut: Some(134)`. Per `FaultPlan`, node 1's writes and node 2's reads are the same traffic direction; the node-2-to-node-1 direction is uncut. The doc misdescribes the fixture.

Evidence:

      1290	/// Reconstructed counterexample: a send, one gossip cut in both directions
      1291	/// mid-frame, then a retirement, under bookmark read/write fail
      1292	/// schedules on every node.
        ...
      1304	                FaultPlan {
      1305	                    write_cut: Some(1324),
      1306	                    read_cut: None,
      1307	                },
      1308	                FaultPlan {
      1309	                    write_cut: None,
      1310	                    read_cut: Some(134),
      1311	                },

    tests/common/fault.rs:
        43	    /// Bytes this endpoint may write before its writers fail.
        44	    pub write_cut: Option<usize>,
        45	    /// Bytes this endpoint may read before its readers fail.
        46	    pub read_cut: Option<usize>,

Resolution: reword to "one gossip whose node-1-to-node-2 direction is cut at both ends (the writer after 1324 bytes, the reader after 134), then a retirement, ...". Acceptance: the doc names the direction and both offsets consistently with `FaultPlan`'s field semantics.

### tests-bookmark-24: Rule (2) in the module doc disagrees with `Model::serve_bootstrap` on the session after a donation
- Where: tests/bookmark_when.rs:13-20 (related: tests/bookmark_when.rs:40-55, 341-350; src/bookmark.rs:387-396; src/peer/gossip.rs:564-571)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read the rule, the model transition, `Bookmarked::slice`'s token clearing with its stated hazard, and `bookmark_donate`)
- Seen by: api-economics; refutation: confirmed; history: the implementation policy is deliberate and its rationale lives at src/bookmark.rs:387-394; the finding converts to stating it in rule (2)
- Owner-gated: no for the doc fix; the production alternative reopens ccd88401's design and is listed under open questions

Rule (2) says a session writes only when the peer has unpersisted local work "since its last persist". The donation's own slice-and-write is a persist, after which nothing local has happened, so rule (2) predicts zero writes for the next plain gossip. `Model::serve_bootstrap` sets `pending = true` after the donation write and predicts one write for that session, and the proptest asserts the model. The model is right about the implementation (`slice` clears the token so `bookmark_update` re-records), but that extra write is a policy chosen for a specific hazard (fork-then-absorb returning the party to its pre-donation value at the same version), not a consequence of "what the operation is" as the model section (L40-55) claims. The doc overstates the model's independence and misstates the observable contract.

Evidence:

        13	//! 2. **Write on local work, never on hearsay.** A session persists the record
        14	//!    *before* it transmits, but only when the peer has unpersisted *local*
        15	//!    identity work to checkpoint: it has never persisted, or has emitted a local
        16	//!    change (a [`send`](rumors::Rumors::send) or
        17	//!    [`redact`](rumors::Rumors::redact)) or moved a party (donated a fork,
        18	//!    absorbed a retiree) since its last persist. Merely *incorporating*
        19	//!    content learned over gossip advances only other parties' identities,
        20	//!    never the peer's own, so it triggers no write at all.
        ...
       344	    fn serve_bootstrap(&mut self) -> Delta {
       345	        let reads = self.read_on_first_use();
       346	        let writes = 1 + usize::from(self.pending);
       347	        self.loaded = true;
       348	        self.pending = true;
       349	        Delta { reads, writes }
       350	    }

    src/bookmark.rs:
       387	        // Donating shrinks our live identity, so the suppression token is now
       388	        // stale: clear it. Leaving it would let a later update wrongly suppress
       389	        // if the party happened to return to its pre-donation value at the same
       390	        // version (e.g. forking for a bootstrap, then absorbing that peer's

Resolution: state the post-donation re-record explicitly in rule (2) as policy ("a donation persists the sliced record but leaves no suppression token, so the session after a donation re-records the live identity once"), and in `Model::serve_bootstrap`'s doc name the hazard the policy guards. Acceptance: rule (2), the model-section prose, and `Model::serve_bootstrap` agree on the post-donation session; a reader can predict the `Delta` from the doc alone.

### tests-lifecycle-2: Three partition-local 64 KiB `LINK_BUF` constants with rationales the harness does not support
- Where: tests/bootstrap.rs:24-27 (related: tests/retire.rs:31-33, tests/reuse.rs:28-31, tests/party_conservation.rs:48-51, tests/common/wire.rs:61-63, tests/common/wire.rs:248, tests/common/schedule/executor.rs:258)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'const LINK_BUF' tests/` lists ten definitions, four at 8 KiB including `common::wire::LINK_BUF`, six at 64 KiB; wire.rs:248 serves every harness bootstrap at 8 KiB; executor.rs:258 runs every executor retire session at 8 KiB)
- Seen by: structure-prose, api-economics; refutation: reframed (bootstrap.rs does not claim 8 KiB fails; its choice is unmotivated, and retire.rs and party_conservation.rs justify 64 KiB by pointing at each other); history: deliberate-but-expired (the 64 KiB constants and their prose were written for the single duplex pipe under the mux; b3b877d9bf renamed `DUPLEX_BUF` to `LINK_BUF` and carried the sentences across unchanged)
- Owner-gated: no

The bootstrap.rs constant is justified by keeping "the bootstrap descent's largest frames" clear of backpressure, yet `bootstrap_fork` serves every harness bootstrap over the 8 KiB `common::wire::LINK_BUF`, whose own doc states the opposite policy ("A modest buffer is sufficient and naturally exercises per-stream backpressure"). retire.rs:31-33 keeps 64 KiB for "the other wire tests' headroom" while party_conservation.rs:48-51 keeps it to "keep `retire.rs`'s headroom": a circular chain, and the executor runs the same retire sessions at 8 KiB. Only reuse.rs:28-31 states a reason local to its test shape (an eager side must write its next preamble without waiting on the laggard). Prose must describe today's code; a rationale the tree elsewhere falsifies misleads the next person sizing a buffer.

Evidence:

        24	/// Capacity for each in-memory link stream. Roomy enough that the bootstrap
        25	/// descent's largest frames fit without the test depending on backpressure
        26	/// subtleties.
        27	const LINK_BUF: usize = 64 * 1024;

        31	/// Capacity for each in-memory link stream. A divergent retiree's session moves
        32	/// content through the gossip round, so keep the other wire tests' headroom.
        33	const LINK_BUF: usize = 64 * 1024;

Resolution: Import `crate::common::wire::LINK_BUF` in bootstrap.rs and retire.rs (and party_conservation.rs, whose comment dangles once retire.rs changes). Keep reuse.rs's constant with its shape-specific rationale, or promote it to wire.rs under a name that carries that rationale. Acceptance: `grep -rn 'const LINK_BUF' tests/` returns wire.rs plus only definitions whose comment names a test shape a smaller buffer would change; bootstrap, retire, and party_conservation pass unchanged.

### tests-observation-1: causal.rs's module doc scopes out the unordered face that two of its tests exercise
- Where: tests/causal.rs:1-3 (related: tests/causal.rs:89-99, tests/causal.rs:570-580, tests/causal.rs:620-632, tests/causal.rs:657-667)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: structure-prose [11]; refutation: confirmed; history: deliberate-and-holds for the placement (the crash boundary was found on the causal face with the unordered face as control, 6269aab3f; the module doc was never updated)
- Owner-gated: no

The module doc says this file covers `CausalMessages` "on top of everything `UnorderedMessages` already promises (exercised in `tests/listen.rs`)", yet `restart_replays_every_unhandled_message` and `final_pop_checkpoint_still_replays_the_last_message` each run an `UnorderedMessages` half, and `drain_unordered` exists only to serve them. A reader hunting for the unordered face's crash-boundary pin looks in listen.rs and does not find it. A module doc's first sentence stands alone in a listing and tells the reader what lives here; the map is wrong.

Evidence:

    1	//! The [`CausalMessages`] observer: the causal-delivery contract on top of
    2	//! everything [`UnorderedMessages`](rumors::UnorderedMessages) already
    3	//! promises (exercised in `tests/listen.rs`).

    91	fn drain_unordered(obs: &mut rumors::UnorderedMessages<u64>) -> Vec<(Version, u64)> {

Resolution: Keep the two-face tests together (they pin that both faces hold the same checkpoint boundary, which causal.rs:645-649 states) and restate the module doc: "...and the checkpoint-resume boundary both observer faces share, pinned here against both." `drain_unordered` dissolves with tests-observation-2. Acceptance: the module doc names the unordered-face tests present in the file.

### tests-disruption-handshake-5: The reconstructed-counterexample section speaks in the past tense and states cut geometry nothing maintains
- Where: tests/disruption.rs:589-594 (related: tests/disruption.rs:604-607, 621-624, 672-674)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (git log -S: the tests were introduced at 6d48d8dc, 2026-08-13, and renamed at 2c73d032; the wire was respelled afterwards at 4dd2053c, 3327a92b, and 368da2a5, none of which touched tests/disruption.rs)
- Seen by: structure-prose [19], blind-spots [27]; refutation: confirmed (27's dating corrected: introduced at 6d48d8dc); history: 19 no rationale found, 27 deliberate but expired
- Owner-gated: no

The header frames the constructions as "Historical" seeds that "no longer" replay, which is provenance prose in the tree; the mechanism sentence (a committed seed regenerates through the strategy's cut range, so a range change re-maps its offsets) is present-tense and worth keeping. The four test docs then describe where their byte-offset cuts land ("dies mid-transfer", "cut in both directions", "late in their byte streams", "near the deep end of the fault range the case was found under"). A byte offset is a property of the wire bytes, and the frame layout has changed since the plans were recorded, so those positions are claims nothing maintains: the plans are still deterministic fault plans worth running, but the docs promise a session position they cannot keep. Prose speaks in the present tense; a testdoc must be accurate.

Evidence:

       589	// Historical shrunk counterexamples, preserved as explicit constructions:
       590	// their committed seeds regenerate through the fault strategy's cut range,
       591	// so a range change re-maps the offsets and the seed no longer replays the
       592	// case it pinned. Each test runs the exact plan its seed's shrink recorded,
       593	// under the same invariants as the proptest above. The seed files stay
       594	// committed; these constructions carry the counterexamples themselves.

       672	/// Reconstructed counterexample: three children whose cuts sit near the deep
       673	/// end of the fault range the case was found under, severing sessions and
       674	/// retirements late in their byte streams.

Resolution: Rewrite the header in the present tense ("Shrunk counterexamples as explicit constructions. A committed seed regenerates through the fault strategy's cut range, so a range change re-maps its offsets and the seed replays a different plan; these constructions carry the plans themselves.") and re-denominate the four docs to what is stable: fixed fault plans from shrunk seeds, run under the full invariant battery, with no claim about where the cuts fall. If the original geometry matters, derive the cut from a metered clean run of the same plan (a fraction of the measured extent) so it survives wire changes. Acceptance: no "Historical", "no longer", or "was found" in the section; no doc states a session position a wire-format change could falsify.

Synthesis note: The "Historical" header is also prose-hygiene-7's fourth site.

### prose-hygiene-8: Em-dashes inside assert message strings
- Where: tests/future_size.rs:53 (related: tests/bookmark_causality.rs:149; tests/party_conservation.rs:93-94)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rnI --include='*.rs' '—' src tests benches examples | grep -vE '^[^:]+:[0-9]+:\s*(///|//!|//)'` returns exactly these four lines; each read in context as an assert!/assert_eq! message)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Four assertion messages carry a true em-dash. These strings print to
terminals on failure, where the doctrine prefers colons or semicolons.

Evidence:

    tests/future_size.rs
    53	         indirection, restore it — otherwise downstream crates will hit \

    tests/party_conservation.rs
    93	        "the join of all live parties must be invariant — exactly the seed's \
    94	         whole interval — after every step; got {whole:?}"

Resolution: Replace with a colon or semicolon in each of the four strings.
Acceptance: the grep above returns nothing.

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

### tests-disruption-handshake-20: The handshake suite's first line cites a module path that does not exist
- Where: tests/handshake.rs:1-1 (related: src/tree/mirror/handshake.rs:298; src/tree/mirror/streaming.rs:53)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep -rn 'remote::preamble|mod remote|fn preamble' src tests: `pub mod remote` only at src/tree/mirror/streaming.rs:53, the proxy; the function at src/tree/mirror/handshake.rs:298)
- Seen by: structure-prose [8], blind-spots [34], api-economics [45]; refutation: confirmed (a true ghost of moved code: remote/preamble.rs existed at 2b6618ae and was merged into the shared handshake at f6cf2579); history: contradicts the AGENTS.md hard rule
- Owner-gated: no

The preamble exchange is `tree::mirror::handshake::preamble`; nothing named `mirror::remote::preamble` exists, and `remote` is the streaming proxy. A first sentence stands alone in a listing and here points the maintainer at the wrong module.

Evidence:

         1	//! Protocol preamble exchange (`mirror::remote::preamble`).

Resolution: Cite `tree::mirror::handshake::preamble`, or drop the parenthetical (the next sentence already says what is driven). Acceptance: the cited path resolves under src/.

Synthesis note: Same line as module-graph-5's last site.

### tests-resource-link-window-9: two module-doc claims are false today: latency_link's plural "protocols" and tcp_link's "one Link instantiation over real sockets"
- Where: tests/latency_link.rs:3-6 (related: tests/tcp_link.rs:3-4, tests/routed_link.rs:1-12, src/protocol.rs:15-19, src/link/routed/endpoint.rs:20-25, tests/common/tcp.rs:18-21)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (src/protocol.rs:15-19 has `V2` as the sole variant; routed_link.rs runs `RoutedLink<TcpDial>` over loopback sockets through the same conformance suite; tcp.rs:18-21 already names the distinguishing property)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (latency_link.rs:5 predates the V1 retirement; tcp_link.rs:3 predates routed_link)
- Owner-gated: no

Both sentences were true when written and neither was swept by the commit that falsified it. "the protocols" is V1-era residue; "the workspace's one Link instantiation over real sockets" is a uniqueness claim routed_link.rs contradicts. Prose states what is.

Evidence:

     3	//! `benches/support/latency.rs` builds the delayed-pipe link the latency
     4	//! benchmarks sweep; these tests run it through the public
     5	//! [`rumors::conformance::link`] suite so the sweep measures the protocols, not
     6	//! an accidentally nonconforming transport. Delays live in virtual time

    tcp_link.rs:
     3	//! `common::tcp` is the workspace's one [`Link`](rumors::link::Link)
     4	//! instantiation over real sockets, and `tests/disruption.rs` trusts it

Resolution: latency_link.rs:5: "measures the protocol". tcp_link.rs:3-4: "the simulations' direct, one-connection-per-stream Link over real sockets" (the property tcp.rs:18-21 explains). Acceptance: both sentences are true of today's tree.

### tests-lifecycle-17: The union oracle's stated soundness premise omits the redaction exclusion, and the pairwise laws are pinned only on disjoint-content populations
- Where: tests/pairwise.rs:212-218 (related: tests/pairwise.rs:55-57, tests/pairwise.rs:72-74, tests/async_wire.rs:7-11, tests/common/action.rs:79-83, src/rumors.rs:451-452, tests/common/schedule/arb.rs:315-336, tests/common/schedule/arb.rs:395-403, tests/retire.rs:369-371, tests/retire.rs:412-414)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (action.rs:79-83: `Redact` only targets `versions[..]` the same peer inserted; every pairwise peer is `build_local(dup(&seed), ..)` over an empty seed; async_wire.rs:9-11 states the extra premise this doc omits; arb.rs:395-403 redacts from the observed log, which includes gossip-learned messages, so the schedule engines do reach the shared-redaction shape)
- Seen by: blind-spots; refutation: reframed (the shape is covered generatively by multi_peer's oracle under arbitrary gossip orders plus the fixed-point test; what remains is a doc inaccuracy and a narrower population for one file's laws) and downgraded to low; history: no-rationale-found
- Owner-gated: no

The doc justifies `BTreeMap::extend` as a union oracle by disjoint parties alone. That is necessary but not sufficient: the oracle is also sound only because `build_local` redacts only a peer's own pre-session sends, so no peer ever holds a message its counterparty redacted. Under the documented session promise (rumors.rs:451-452: both replicas hold every message either held "and neither had deleted"), the constructed scenario "seed sends X; a and b fork; a redacts X; gossip" makes the union oracle predict X live at both sides while the contract requires it absent. async_wire.rs:9-11 states the premise correctly. The same population shape means `gossip_converges`, `gossip_side_symmetric`, `gossip_idempotent`, and `gossip_order_independent` never see a redaction of shared content; the schedule engines cover that family, but this file's header does not say the division is deliberate. The doc also names `alice` / `bob` while the body's variables are `a` / `b`.

Evidence:

       212	    /// One session unions live content: after gossip, each side's readout
       213	    /// equals the union of the two pre-session readouts.
       214	    ///
       215	    /// The "union of readouts" is computed by `BTreeMap::extend`,
       216	    /// which is sound here only because readout keys are the leaf
       217	    /// versions' canonical bytes and `alice` / `bob` tick disjoint
       218	    /// parties, so they can't create the same version.

Resolution: State the full premise in the doc (disjoint parties, and redactions only of one's own pre-session sends), fix the names, and either state in the module header that shared-then-redacted content is the schedule engines' domain, or extend the population: draw `seed_actions` applied before the forks and let a post-fork redact phase target inherited versions, replacing the oracle with union minus every version either side redacted. Acceptance: the doc's stated reasons are sufficient for the oracle's soundness; the division of coverage between pairwise.rs and the engines is written where a reader of either will see it.

### prose-hygiene-7: Dated rationale and incident narrative in test docs
- Where: tests/payload_depth.rs:325-327 (related: tests/session_overlap.rs:73-76; tests/common/overlap.rs:9-11; tests/disruption.rs:589; tests/bookmark_causality.rs:1263)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (each site read with line numbers)
- Verification: confirmed, with the two "Historical" comments relocated to their actual lines (disruption.rs:589 and bookmark_causality.rs:1263, not :588 and :1262); history: no-rationale-found
- Owner-gated: no

Three testdocs explain the present by reference to a past change or
incident: "still fails ... now that admission runs the receiving decode",
"The discovering incident's symptom ... at 2 of the 25 sweep positions",
and "where a real defect lived". Two softer comments label reconstructed
counterexamples "Historical"; the rest of those comments state a live
mechanism and are fine.

Evidence:

    tests/payload_depth.rs
    325	/// decodes payloads as `String`), the one shape that still fails at
    326	/// ingress between equal limits now that admission runs the receiving
    327	/// decode. The receiver's own exit is the typed decode error.

    tests/session_overlap.rs
    73	/// The discovering incident's symptom was precisely an innocent leaf
    74	/// silently deleted under this overlap, at 2 of the 25 sweep positions.

    tests/common/overlap.rs
    9	//! sessions) in between. That gap is where a real defect lived — its
    10	//! downstream symptom was an innocent leaf silently lost under exactly

    tests/disruption.rs
    589	// Historical shrunk counterexamples, preserved as explicit constructions:

Resolution: payload_depth.rs:325-327: "the one shape that fails at ingress
between equal limits, because admission runs the receiving decode."
session_overlap.rs:73-76: "An innocent leaf deleted under this overlap at
any sweep position fails here; the sweep is total, so any regression with
that symptom fails whichever layer produces it." common/overlap.rs:9-11:
"That gap is where an overlap defect hides: an innocent leaf lost under
exactly such an overlap." Optionally "Reconstructed" for "Historical" at
disruption.rs:589 and bookmark_causality.rs:1263. Acceptance: the five sites
read as present-tense statements of mechanism with no tally or incident.

Synthesis note: Overlaps tests-common-11 (overlap.rs:9-11), tests-bookmark-16 (bookmark_causality.rs:1263), tests-disruption-handshake-5 (disruption.rs:589), and tests-wire-format-15 (payload_depth.rs:325-327).

### tests-observation-27: Ghost reference: the shadow-validity doc names an `on_message` callback the harness does not have
- Where: tests/shadow_validity.rs:15-17 (related: tests/partition.rs:12, tests/common/peer.rs:65-75)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn on_message tests/ src/` matches only the two doc comments; tests/common/peer.rs records observations via `drain` over `snapshot.range(causally::since(&self.checkpoint))`)
- Seen by: blind-spots [34]; refutation: confirmed; history: deliberate-but-expired (`on_message` was the callback of the original listen API, retired into the pull-based observers in db32b94d9 on 2026-06-10; both docs are older and were never re-denominated)
- Owner-gated: no

The module doc describes `observed_log` as predicting what each peer's `on_message` callback would fire; no such callback exists anywhere in tests/ or src/. The harness records observations by draining `Snapshot::range` into `Peer::observations`. Hard rule: no prose names code that does not exist.

Evidence:

    15	//! * `observed_log` — the set of `EventIdx`s the shadow predicts
    16	//!   each peer's `on_message` callback would have fired for must
    17	//!   match the set the live executor actually fired (for a retired

Resolution: Rewrite as "the set of `EventIdx`s the shadow predicts each peer's drain would record must match the set the executor's `observations` log holds"; fix tests/partition.rs:12 in the same pass. Acceptance: `grep -rn on_message tests/ src/` returns nothing.

Synthesis note: Same ghost as tests-lifecycle-19.

### tests-lifecycle-30: single_peer.rs's module doc covers half the file
- Where: tests/single_peer.rs:1-6 (related: tests/single_peer.rs:29-140, tests/single_peer.rs:171-462)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: the doc names fan-out, version distinctness, and monotonicity, which is the proptest block at 29-140; lines 171-462 pin commit-iff-Ok, cancel on panic, on `?`-propagated depth error, and on user `Err`, `send_all` all-or-nothing, `redact_all` skip-and-one-tick, the locally handled admitted prefix, and inner-before-outer nesting)
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (the doc's last edit coincides with the first batch-lifecycle additions and predates the second batch)
- Owner-gated: no

A module doc is the file's map; when it omits half the contents, the next batch-lifecycle test lands in the wrong file or gets duplicated elsewhere.

Evidence:

         1	//! Single-peer correctness for a lone rumor set, with no gossip.
         2	//!
         3	//! Exercises the surface area of [`Batch`](rumors::Batch) commits:
         4	//! live-leaf fan-out, distinctness of the [`Version`](rumors::Version)s
         5	//! created within a batch, and strict monotonicity of the local party's
         6	//! component of each created version.

Resolution: Extend the doc with one sentence per group: the commit lifecycle (commit on `Ok`, cancel on `Err` or panic), the bulk variants' all-or-nothing and skip semantics, and nesting. Acceptance: every test in the file is covered by a sentence in the module doc.

### tests-lifecycle-32: A testdoc cites a batch-docs promise of strictly increasing per-action versions that the public docs do not make
- Where: tests/single_peer.rs:66-69 (related: src/batch.rs:10-26, src/tree.rs:370-382, tests/single_peer.rs:84-87)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'increasing\|ascending\|monoton\|totally ordered\|strictly' src/batch.rs src/peer.rs src/lib.rs src/rumors.rs` returns nothing relevant; src/batch.rs read in full, it mentions versions only for redaction; the only statement is the private `Tree::act` doc at tree.rs:373-376; `git show 8dc0596ed -- src/batch.rs` deletes the sentence "increasing versions (content-identical messages get distinct keys)" that the testdoc cited)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: deliberate-but-expired (the promise existed in `Batch`'s public docs the day the testdoc was written, 2026-06-10, and 8dc0596ed removed it the next day)
- Owner-gated: no (the reword needs no API decision; restoring the clause would, and is listed as an open question)

The property is sound (a lone party's versions form a chain), but its doc attributes the guarantee to "the batch docs", which no longer state it; a reader looks for the promise, fails to find it, and cannot tell whether the test or the docs is wrong. The test also cannot check action order: `batch_send` recovers versions in tree-iteration order and the test sorts them (84-87).

Evidence:

        66	    /// Every `Version` created by a lone peer is totally ordered against
        67	    /// every other — both within a single batch (the batch docs promise
        68	    /// strictly increasing versions per action) and across successive
        69	    /// batches.

Resolution: Reword the parenthetical to state the invariant in its own terms ("every action ticks the one party, so a lone peer's versions are a chain") or cite `Tree::act` as the maintainer contract; alternatively, if Finch wants users to rely on per-action ordering within a batch, restore the clause to `Batch`'s docs and keep the citation. Acceptance: the doc names a contract that exists where it says it exists.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| tests-common-7 | tests/common/gossip_snapshot.rs:389-399; tests/common/sim.rs:422 | "V2" qualifiers where one protocol exists. | Drop the qualifier. |
| tests-common-24 | tests/common/sim.rs:390, 759; tests/common/flaky.rs:212 | Em-dashes in `//` comments. | Crate-wide pattern. |
| tests-common-27 | tests/common/sim.rs:1007-1011 and the sites listed | "genuine(ly)", loose "sound", "conviction" for a `verdict`, and a "silent divergence" inside an assertion. | Reword per site. |
| prose-hygiene-12 | tests/async_wire.rs:13 | "Both tests" enumerates the module's contents by count. | "The tests share ...". |
| tests-bookmark-5 | tests/bookmark_attach.rs:75-77 (twelve sites) | Doc paragraphs left as a fragment line after the first-sentence split. | Re-wrap. |
| tests-bookmark-22 | tests/bookmark_when.rs:1 and the sites listed | "at the seam", "faithful", "genuine", "honest disruption". | Mechanism words at each site; route "seam" to the crate-wide sweep. |
| tests-observation-4 | tests/causal.rs:184-189 (fourteen sites) | Ragged wraps left by the first-sentence split. | Re-wrap. |
| tests-resource-link-window-6 | tests/decode_alloc.rs:31-41 and the sites listed | "honest" (two constant names), "genuinely", "knob", "seam", "law", "reds" as a verb. | Rename `HONEST_LEN` to `FULL_LEN`; reword per site. |
| tests-wire-format-15 | tests/dispute_wire.rs:8-12 and the sites listed | "silently", "loudly", "defuse", "close the loop", a dated "now that", and a file-path citation of a constant. | Rewrite each as mechanism; cite `DISPUTE_OVERHEAD_BYTES` by name. |
| tests-disruption-handshake-1 | tests/disruption.rs:8-9 and the sites listed | "genuine(ly)", "silently", and "real" as a synonym for actual, without a contrast. | Delete, or name the counterpart contrasted. |
| tests-disruption-handshake-24 | tests/handshake_liveness.rs:56-59 (thirteen sites) | Ragged doc paragraphs after the first-sentence split. | Reflow, keeping the blank `///`. |
| tests-disruption-handshake-32 | tests/hop_trace.rs:529-530, 585-586 | `transfer_pair`'s comments misattribute its determinism to `seed_rng` and overstate what the self-checks catch. | Use `Peer::seed()`; reword both comments. |
| tests-observation-19 | tests/listen.rs:357-359, 376, 380, 391; tests/session_overlap.rs:61-62 | "earned"/"earns" for checkpoints; "two honest sessions". | "yields an equal checkpoint"; drop "honest". |
| tests-lifecycle-16 | tests/pairwise.rs:113-118 | The doc claims "the union of all three"; the body asserts only `a1 == a2`. | Assert the union, or trim the doc. |
| tests-observation-31 | tests/party_conservation.rs:16-18, 66, 200 | "The Law of Disjointness" is a capitalized coinage with no anchor in `before`. | "party disjointness", or cite `fork_halves_disjoint`. |
| tests-observation-26 | tests/stale_floor.rs:27-31 | The testdoc quantifies over a family ("no matter how far") that the body samples at one point. | Reword to the point pinned, or lift `16` to a proptest range. |
| tests-resource-link-window-24 | tests/window_corners.rs:93-97 and the sites listed | Measured hop counts recorded in comments beside bands no assertion enforces. | Pin exactly, or drop the measurement (owner-gated). |

## Benches and examples

### benches-envelope-8: gossip_grid cites an in_memory.rs `join` bench that does not exist
- Where: benches/gossip_grid.rs:33-38 (related: benches/support/grid.rs:86-95, benches/in_memory.rs:1-5)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (I read all of in_memory.rs: no `join`; `git show ac68e8121^:benches/in_memory.rs` lists `join_grid` / `join_identical`; ac68e8121's message says the file "loses its join benches")
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (accurate at db5a3bbb80; ac68e8121 on 2026-06-10 removed the join benches without touching this sentence, and the later ghost-reference sweeps missed it)
- Owner-gated: no

The doc anchors its grid and throughput accounting to a bench that has not existed since June. A reader sent to compare the two finds nothing. AGENTS.md hard rule: nothing in the codebase refers to code that no longer exists. The accounting the sentence wants to name is stated in full at `grid.rs:86-95` (`Cell::divergence`), so the pointer to `[grid]` already carries it.

Evidence:

    33	/// `gossip` across the divergence grid.
    34	///
    35	/// The grid and throughput accounting mirror `in_memory.rs`'s `join` bench
    36	/// exactly (see [`grid`]); the only difference is that each pair reconciles
    37	/// over the wire via [`Wire::round_trip`] rather than in-process. The identical
    38	/// corner reports latency in a separate `gossip_identical` group.

Resolution: Re-state against what exists: "Throughput is charged against each cell's divergence and the sample count follows its build magnitude (see [`grid`]); the identical corner reconciles nothing and reports latency in a separate `gossip_identical` group." Acceptance: `grep -n join benches/gossip_grid.rs` returns nothing; the doc names only benches and helpers that exist.

### benches-envelope-10: The "Fixture discipline" section describes a hazard the binary cannot trigger and a rebuild policy it does not follow
- Where: benches/in_memory.rs:14-21 (related: benches/in_memory.rs:86-90, benches/in_memory.rs:97-101, benches/in_memory.rs:117, benches/in_memory.rs:144, benches/in_memory.rs:184, benches/in_memory.rs:213, benches/in_memory.rs:239, benches/in_memory.rs:277, benches/in_memory.rs:301, benches/in_memory.rs:325, benches/support/wire.rs:39-56)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (the only `fork` in the file is doc line 17; no `wire::`, `bootstrap_fork`, or `.fork(`; bench bodies read: `batch_insert` seeds inside the timed closure at 97-101, `iter`/`observer_replay`/`causal_replay`/`get` build once per size at 117/213/277/325, the three `*_delta` groups once per `(n, delta)` at 184/239/301, and only `redact` rebuilds per iteration at 144)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (10f1e5166c wrote "rebuilt from a fresh seed in untimed setup and forked at most once" for the join benches, which forked a party per fixture; ac68e8121 deleted the join benches and the "forked at most once" clause but kept the fork hazard as the premise)
- Owner-gated: no

The section justifies fresh-seed fixtures by `before`'s fork-depth hazard, but nothing in this binary forks a party, so the hazard cannot arise (Principle 3: a rationale that names no failure it prevents is the circular-justification tell). It then asserts "every fixture is rebuilt from a fresh `Peer::seed` in untimed setup", which eight of nine benches contradict: seven build once per size and share the set across iterations, `batch_insert` seeds inside the timed body, and only `redact` rebuilds per iteration. The per-bench docs already state each fixture's real lifecycle, so the section competes with accurate prose.

Evidence:

    14	//! # Fixture discipline
    15	//!
    16	//! Inserting a message ticks an Interval Tree Clock party, and `before`
    17	//! documents that repeatedly [`fork`](before::Party::fork)ing *the same*
    18	//! party deepens its id tree linearly (worse memory and per-op cost). To
    19	//! keep that out of the measurements, every fixture is rebuilt from a fresh
    20	//! [`Peer::seed`](rumors::Peer::seed) in untimed setup: no party
    21	//! accumulates depth across Criterion iterations.

Resolution: Delete the section (the per-bench docs at 86-90, 107-111, 133-136, 156-158 carry the true fixture story), or replace it with what is: read-only benches build one set per size in untimed setup and share it; the consuming `redact` rebuilds per iteration with `BatchSize::PerIteration`; `batch_insert` builds inside its timed body. If the fork-depth note is wanted anywhere, it belongs beside `bootstrap_fork` in `support/wire.rs`, the one place a fork happens. Acceptance: no sentence in the module doc claims a rebuild policy a bench body contradicts; `grep -n fork benches/in_memory.rs` returns nothing or points at a real fork site.

### benches-envelope-9: "batches commit on drop" states the Batch contract backwards
- Where: benches/in_memory.rs:10-12 (related: src/batch.rs:13-18)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (batch.rs:15-17: "the batch commits — atomically, as one commit — exactly when the closure returns `Ok`. Any other exit (a returned `Err`, a panic) commits nothing"; `grep -n 'impl.*Drop\|fn commit' src/batch.rs` shows no `Drop` impl and a `pub(crate) fn commit(self)`; the file never calls `batch(`)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (true at c5f1210a39 when `Batch` committed in `Drop`; ce27df86 deleted that impl and made the lifecycle commit-on-`Ok`; 212c6914 then moved the benches to `send_all`/`redact_all`)
- Owner-gated: no

The parenthetical inverts a public atomicity contract the user-facing docs state carefully: dropping is the path that commits nothing. The benches also never use `batch`; they call `send_all` and `redact_all`, which are single synchronous commits in their own right.

Evidence:

    10	//! The handles here are the asynchronous [`rumors::Rumors`] and its message
    11	//! observers: every operation measured is synchronous on that surface
    12	//! (batches commit on drop), so no runtime is involved.

Resolution: Replace the parenthetical with what the benches do ("`send_all` and `redact_all` are synchronous single commits") or drop it; the sentence's point is only that no runtime is needed. Acceptance: the sentence no longer mentions drop and names the operations the benches invoke.

### benches-envelope-14: "the generator's latency-only regime" has no referent
- Where: benches/window_wallclock.rs:27-29 (related: benches/window_wallclock.rs:3-5, tests/window_knee.rs:53-55, tests/window_knee.rs:257)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -c generator` is 0 in tests/window_knee.rs, tests/window_operator.rs, and tests/common/window.rs; window_knee.rs:55 `const LINK_CAPACITY: usize = 8 * 1024 * 1024;` "far above this test's transfers"; :257 "latency-only link (roomy pipe)")
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired ("the generator" was `examples/window_tradeoff.rs` at 1988de6d, which ran real sessions at this capacity; 4d0b3db22 made it pure arithmetic and re-pointed this bench's module doc at the knee and operator suites without touching line 27)
- Owner-gated: no

The comment justifies the constant by a "generator" the suites named at lines 3-5 do not contain; the crate's test generator (`tests/common/window.rs`) generates per-peer window choices, not link capacities. The regime the constant matches is the knee suite's own `LINK_CAPACITY`. Expand references, not vocabulary: a compressed pointer to context the reader does not hold is a comment that cannot be followed.

Evidence:

    27	/// Per-stream pipe buffering: roomy, matching the generator's
    28	/// latency-only regime.
    29	const LINK_CAPACITY: usize = 8 * 1024 * 1024;

Resolution: "Per-stream pipe buffering: roomy enough that the pipe never binds, the latency-only regime `tests/window_knee.rs` measures in (its `LINK_CAPACITY`)." Acceptance: the comment names a file and constant that exist.

### swarm-example-1: Module doc omits the run command and two quit keys, and its step numbering disagrees with `run_party`
- Where: examples/swarm.rs:13-20 (related: examples/swarm.rs:114-123, examples/swarm.rs:550-611, examples/swarm.rs:1558-1561, examples/swarm.rs:1760-1761)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: no rationale found (all four gaps original to e8442e6a)
- Owner-gated: no

The module doc is the essay a reader uses to run and interpret the example, and three details drift from the code: `# What it does` numbers the party loop 1-3 (serve, initiate, controller) while `run_party` numbers it 1-4 with membership commands as step 2, so cross-referencing the numbers misleads; `# Controls` and the footer list only `q` for quit while `ui_loop` also quits on `Esc` and `Ctrl-C`; and the doc never says how to run the example (`cargo run --release --example swarm -- [flags]`), where whether `--release` matters for a throughput readout is the first question a reader has. (The readout bullet at 109-110, "on the initiator's I/O", is resolved by swarm-example-16.)

Evidence:

    13	//! 1. **Serve** any inbound sync requests waiting in its inbox (it is the
    17	//! 3. Otherwise, run the **steady-state controller**: compare the number of
    116	//! `↑`/`↓` select a parameter, `←`/`→` adjust it (`Shift` for a coarse step),
    117	//! `space` pauses all churn, `q` quits.
    566	        // 2. Obey membership commands from the coordinator, between sessions.
    1558	                KeyCode::Char('q') | KeyCode::Esc => break,
    1559	                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {

Resolution: Add "obey membership commands from the coordinator" to the overview list (or drop its numbers); write "`q`, `Esc`, or `Ctrl-C` quits" at 117 and in the footer at 1760-1761 (or remove the extra keys from the handler); add a two-line `# Running` section with the `cargo run --release --example swarm -- [flags]` invocation and the `--headless-secs` variant. Acceptance: every key the loop handles is listed; the overview's steps map one-to-one onto `run_party`'s numbered comments; a reader can run the example from the module doc alone.

### swarm-example-2: "version vector" collides with the distributed-systems term of art
- Where: examples/swarm.rs:27-28 (related: none)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep finds this single occurrence; `git blame -L 27,28` attributes it to 9c73d7b4, the `Key` retirement, which mechanically renamed "key vector")
- Seen by: prose; refutation: confirmed; history: no rationale found (rename residue)
- Owner-gated: no

The module doc calls each party's `Vec<Version>` redaction pool "the version vector". In a crate built on Interval Tree Clocks, "version vector" names the causality structure that ITCs generalize, not a list of message versions; the rest of the file says "pool" or "version pool". Established terms of art are used only in their established sense.

Evidence:

    27	//! the next sync. The version vector is per-thread: no shared rumor-set
    28	//! state, no lock contention on the hot path.

Resolution: Write "The pool is per-thread" (or "version pool"), matching `Donation`'s and `run_party`'s vocabulary. Acceptance: `grep -n 'version vector' examples/swarm.rs` returns nothing.

### swarm-example-6: Private docs and comments describe state the code does not have
- Where: examples/swarm.rs:258-260 (related: examples/swarm.rs:264, examples/swarm.rs:291-294, examples/swarm.rs:171, examples/swarm.rs:1023, examples/swarm.rs:1399-1410, examples/swarm.rs:355-358, examples/swarm.rs:363, examples/swarm.rs:372, examples/swarm.rs:377-378, examples/swarm.rs:575, examples/swarm.rs:661-663, examples/swarm.rs:678, examples/swarm.rs:709, examples/swarm.rs:717, examples/swarm.rs:907-908)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (every `compare_exchange` in the file is at 646, 675, 695 by grep; `inflight` is absent from `Snapshot::take` at 1399-1410 and moves both ways at 342/349; `bootstrap_fork` and `shrink` build raw `memory_with_capacity` pairs at 171 and 1023; the remaining items by reading)
- Seen by: prose (four candidates merged here); refutation: all four confirmed, two lowered to nit; history: no rationale found for any (the `Metrics` claim was false when d987a2be wrote it; the `SwarmPeer` list omitted `id` and `control` from e8442e6a; the `try_initiate` arms were added by d987a2be without a doc update; the "claim race" comment never had a referent)
- Owner-gated: no

Five sites state something the code does not do, the Principle 5 failure in its plainest form. `Metrics`' doc says the counters are "sampled by the UI" and "All monotonic since start except `sync_nanos_best`", but `inflight` goes up and down and is read by shutdown, not the UI; `wire_bytes` says "every wire" while the bootstrap and retire links are undecorated, so only sync-session wires are tallied (the readout stays self-consistent because `wire_direction_nanos` covers the same sessions; the doc's denominator is what is wrong). `SwarmPeer`'s doc enumerates "only" three of its five fields. `Command`'s doc says the reply is "for the party to hand its [`Rumors`] back" while `Fork` hands back a child's. `try_initiate`'s doc names two `false` arms of five (already engaged, shutdown begun, and peer thread gone are omitted). The coordinator's balanced-branch comment names "the loser of a claim race" as a trigger, but the coordinator performs no claims and in that branch only the `parties` knob can change either count.

Evidence:

    258	/// Process-wide counters, sampled by the UI. All monotonic since start except
    259	/// `sync_nanos_best`; the UI differences successive snapshots to get windowed
    260	/// rates.
    264	    /// Total bytes written to every wire — control stream and every data
    355	/// Holds no rumor-set
    356	/// data — only the inbox to deliver session endpoints, the engaged flag that
    357	/// serializes each party into one session at a time, and a gauge of its
    358	/// current live-message count for the UI.
    377	/// A membership command sent by the coordinator to a single party. Each carries
    378	/// a one-shot reply channel for the party to hand its [`Rumors`] back.
    661	/// Attempt to initiate a sync with a random other party. Returns `true` if a
    662	/// session actually ran (both claims succeeded), `false` if the peer was busy
    663	/// or there was no one to pick.
    907	            // Balanced: nothing to do until the knob or the loser of a claim
    908	            // race changes things. Poll at a human-noticeable cadence.

Resolution: `Metrics`: "Process-wide counters. The rate counters are monotonic since start and the UI differences successive snapshots; `sync_nanos_best` is a windowed minimum and `inflight` a gauge that shutdown drains." `wire_bytes`: "Total bytes written on every sync session's wire (the bootstrap and retire links are not decorated) ...". `SwarmPeer`: state the role without the list. `Command`: "a [`Rumors`] back: its own on wind-down, a fresh child's on fork." `try_initiate`: "Returns `true` iff a session ran; `false` when this party is already engaged, no other party is live, the peer is engaged, shutdown has begun, or the peer's thread has exited." Coordinator: "Balanced: only the parties knob can change this. Poll at a human-noticeable cadence." Acceptance: each doc names only properties its fields have; each `return false` in `try_initiate` has a clause; no reference to claims remains in `run_coordinator`.

### swarm-example-21: "key pool" is a ghost of the retired `Key` vocabulary
- Where: examples/swarm.rs:1014-1015 (related: examples/swarm.rs:387-389, examples/swarm.rs:529, examples/swarm.rs:572)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git blame -L 1014,1015` attributes the comment to 45835294 (2026-06-10); 9c73d7b4 (2026-08-18, "api: retire Key; a message's public identity is its Version") swept "key pool" at four other sites in this file and missed this one; grep finds no other pool-sense "key" in the file)
- Seen by: prose; refutation: confirmed; history: contradicts the hard rule
- Owner-gated: no

The shrink comment calls the redaction pool the "key pool". The crate retired `Key`; every other site says "version pool", "redaction pool", or "pool". AGENTS.md's first hard rule: nothing in the codebase refers to code that no longer exists.

Evidence:

    1014	    // Merge: retire b into a over an in-memory wire. The survivor's key pool
    1015	    // is rebuilt by observer replay in its new thread, so nothing but the

Resolution: "The survivor's version pool is rebuilt by observer replay ...". Acceptance: `grep -n 'key pool' examples/` returns nothing.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| benches-envelope-4 | benches/gossip_fixed.rs:175-180, 36-39, 81-87; latency.rs:20 | `window` carries three senses inside one file. | Qualify each sense; keep "receive window". |
| benches-envelope-7 | benches/gossip_grid.rs:14-16 | A stray one-word doc line. | Re-wrap. |
| benches-envelope-17 | benches/support/grid.rs:37-40; window_wallclock.rs:34; gossip_fixed.rs:9, 67-69 | Hand-maintained counts beside the arrays they count, one wrong ("three orders of magnitude" for 10^2 to 10^6). | State the structure, not the tally. |
| benches-envelope-21 | benches/support/latency.rs:32-34 | The wall component is described as CPU time and measured as elapsed time. | "real elapsed time spent computing". |
| tests-resource-link-window-2 | benches/support/latency.rs:398-400 (three sibling comments) | The dead_code comment names one caller where the partition has three. | Drop the caller clause; keep every `#[allow(dead_code)]`. |
| benches-envelope-24 | benches/support/latency.rs:552 | `expect("bounded hop count")` names no bound. | State why the conversion cannot fail. |
| benches-envelope-26 | examples/envelope_sim.rs:7, 17, 392, 422, 539; benches/support/grid.rs:142 | "integer-honest", "keeps `min` honest", "genuine", "hp". | Plain terms; keep the model-of-record "honest peer". |
| benches-envelope-27 | examples/envelope_sim.rs:36 | The usage block advertises `--fast` as a standalone mode; it applies only under `--full`. | Fix the usage line, or make `--fast` imply `--full`. |
| benches-envelope-30 | examples/envelope_sim.rs:304-308 | A verification is claimed done whose artifact is not in the tree. | Drop the parenthetical, or name the `--manifest` affordance. |
| swarm-example-4 | examples/swarm.rs:155, 570, 737, 767, 1026 | "genuine", "honest", "a real application". | Delete or restate as mechanism. |
| swarm-example-7 | examples/swarm.rs:282-283 (nine sites) | Doc paragraphs left ragged by the summary split. | Re-wrap, keeping the blank `///`. |
| swarm-example-10 | examples/swarm.rs:431, 178-181, 839; examples/swarm/tests.rs:65-66, 92 | `expect` messages are labels rather than proofs, and one claims "any depth limit" though `PayloadDepthLimit::new(0)` is constructible. | State each proof; "inside the default depth limit". |
| swarm-example-11 | examples/swarm.rs:487 (seven partition sites) | Em-dashes in `//` comments. | Crate-wide pattern. |
| swarm-example-17 | examples/swarm.rs:730-734 and the sites listed | The same rationale stated twice at three points. | One full statement each; pointers elsewhere. |

## Verification recipes and configuration (justfile, .config, .github, tools)

### tests-resource-link-window-1: nextest profile comment describes a wall-clock upper bound window_corners no longer asserts
- Where: .config/nextest.toml:32-35 (related: tests/window_corners.rs:206-212)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show f0762b8b -- tests/window_corners.rs` removes `catch_up < Duration::from_secs(5)` and adds `/// Both legs assert floors only`; `git log -1 -- .config/nextest.toml` gives a4d2e546, 2026-07-24, before f0762b8b, 2026-07-31; `grep -n 'from_secs' tests/window_corners.rs` finds no wall-clock ceiling)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

The profile comment justifies running the window suites without scheduling isolation by pointing at one load-sensitive assertion with 8x headroom. That assertion was removed by f0762b8b, whose message states the stronger argument ("a threshold over a noisy quantity relocates the flakiness to the threshold"); the config comment now describes a bound that does not exist and leaves the correct rationale, that every wall-clock assertion is a floor, unstated. This breaches the no-ghost-references rule.

Evidence:

    32	# of the suite like any other test. The one wall-clock upper bound among
    33	# them (window_corners' real-clock promptness cross-check) holds a
    34	# ~0.5 s measured catch-up under a 5 s budget: that ~8x headroom is what
    35	# absorbs load there, where the virtual pins need none.

    206	/// Both legs assert floors only, because only the never-undercharge
    207	/// direction is load-independent on a wall clock: machine load inflates real
    208	/// elapsed time and can never compress it, so these assertions read the
    209	/// same under any suite parallelism.

Resolution: Rewrite lines 32-35 to state what is: the window suites assert on exact virtual time, and window_corners' real-clock leg asserts lower bounds only, which load can inflate but never falsify, so none needs isolation. Drop the 0.5 s and 5 s figures. Acceptance: the comment names no upper bound and no budget figure and agrees with window_corners.rs:206-212.

Synthesis note: Same lines as suite-economics-6, which adds the header paragraph's drifted figures; verification-infra-18 covers that header too. One rewrite of the file resolves all three.

### suite-economics-6: nextest.toml describes a wall-clock upper bound no test asserts and cites drifted measurements
- Where: .config/nextest.toml:28-35 (related: .config/nextest.toml:7-12, tests/window_corners.rs:206-212)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both files in full; no wall-clock `<=` assertion exists in window_corners.rs; git: f0762b8b2 on 2026-07-31 made the real-clock corners floors-only, after the nextest.toml sentence was written in a4d2e5460 on 2026-07-24; run-log times for the inter-process test 0.664 s, the fixture search about 1.6 s per call)
- Verification: confirmed; history: the 2026-07-23 review packet (`review-link-transport-branch.md:642` and `:661`) flagged two other sentences in this file, since fixed; the current drift postdates those fixes.
- Owner-gated: no

The trailing paragraph says the window_corners real-clock cross-check
"holds a ~0.5 s measured catch-up under a 5 s budget" and that "~8x
headroom is what absorbs load there"; `real_clock_corners_match_the_
virtual_model` asserts floors only, by its own docstring, and no 5 s budget
exists anywhere in the file. The header paragraph anchors the 180 s
terminate budget on "the inter-process disruption tests" and a fixture
search "around 6 seconds"; the inter-process test now takes 0.66 s, the
fixture search about 1.6 s per call, and the slowest tests are the capacity
witness (19.9 s), the bookmark plans (11.7 s, 10.1 s) and the codec manifest
(11.2 s). Principle 5: prose at a declaration site speaks in the present
tense and carries no hand-maintained measurements; a justification citing a
check that does not exist misleads whoever next tunes the timeout.

Evidence:

    7	# The slowest honest tests today are the proptest suites and the
    8	# inter-process disruption tests, all finishing well under 60 seconds
    9	# (the deterministic fixture search in src/tree/arb.rs is around 6
    10	# seconds), so three 60-second periods —

    32	# of the suite like any other test. The one wall-clock upper bound among
    33	# them (window_corners' real-clock promptness cross-check) holds a
    34	# ~0.5 s measured catch-up under a 5 s budget: that ~8x headroom is what
    35	# absorbs load there, where the virtual pins need none.

    206	/// Both legs assert floors only, because only the never-undercharge
    207	/// direction is load-independent on a wall clock: machine load inflates real
    208	/// elapsed time and can never compress it, so these assertions read the
    209	/// same under any suite parallelism.

Resolution: delete the wall-clock-upper-bound sentence or re-anchor it to
the floor-only claim (a floor cannot flake under load, which is the point
worth stating). Restate the budget's premise without numbers: the
deterministic pollers turn stalls into errors, every committed test finishes
far inside one period, and the terminate budget is a last-resort bound for escaped
hangs rather than a per-test bound. Acceptance: every sentence in the file
describes a check or mechanism that exists in the tree, and no timing
number remains that a reader would need to re-measure to trust.

Synthesis note: Overlaps tests-resource-link-window-1 (lines 32-35) and verification-infra-18 (lines 7-10).

### deps-7: ci.yml installs toolchains the recipes no longer invoke, and its comment cites a `cargo +nightly` call the justfile does not make
- Where: .github/workflows/ci.yml:54-74 (related: .github/workflows/ci.yml:17-20, justfile:23-40, justfile:123, justfile:368, justfile:555-559, rust-toolchain.toml:28-31)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read ci.yml:1-110, justfile:20-42, rust-toolchain.toml; grep of `nightly` over the justfile; `git log -p -S 'nightly_toolchain :='` shows the variable born as `"nightly"` in 788b5366f (2026-07-15) and pinned to `"nightly-2026-06-30"` in e7a4b7b0 (2026-08-04), whose stat touches AGENTS.md, the fuzzfit bands, the justfile, and rust-toolchain.toml but not ci.yml; ci.yml's only later commits are Dependabot SHA bumps)
- Verification: raised in this verification pass, not in the sweep's report; history: deliberate-but-expired (the comment was accurate when the variable was `"nightly"`; both pins landed afterward without touching ci.yml)
- Owner-gated: no

The workflow installs a floating `nightly` (line 67) with a comment saying "The doctest and fuzz recipes invoke `cargo +nightly`", and installs `stable` (line 72) so that "the gate's bare `cargo fmt`/`clippy`/`check` must hit stable". Neither is what runs: every nightly recipe invokes `cargo +{{ nightly_toolchain }}` with the variable pinned to `nightly-2026-06-30`, and `rust-toolchain.toml` pins `1.97.1`, which rustup resolves for every bare `cargo` regardless of the default. Both pinned toolchains reach the runner only through rustup's implicit auto-install on first use, which the workflow neither states nor controls, so CI's toolchain provenance is the opposite of what its comments claim and rests on a rustup default the tree does not pin. The justfile's own comment (23-33) gives the argument against a floating nightly that this workflow step contradicts.

Evidence:

    .github/workflows/ci.yml
        54	      # Install nightly *first* so the stable install below wins the default:
        55	      # each dtolnay/rust-toolchain step runs `rustup default`, last one sticks.
        56	      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
        57	      # needs to exist, not be the default — whereas the gate's bare
        58	      # `cargo fmt`/`clippy`/`check` must hit stable.
        64	      - name: Install nightly toolchain (merged doctests and fuzz build)
        65	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
        66	        with:
        67	          toolchain: nightly
        69	      - name: Install stable toolchain (default)
        70	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
        71	        with:
        72	          toolchain: stable

    justfile
        40	nightly_toolchain := "nightly-2026-06-30"
       368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

    rust-toolchain.toml
        28	[toolchain]
        29	channel = "1.97.1"

Resolution: Install the pinned toolchains by name in the workflow (the stable step names `1.97.1`, or is dropped if the action honors `rust-toolchain.toml` without a `toolchain:` input; the nightly step names `nightly-2026-06-30`, ideally read from the justfile so the two cannot diverge, or the justfile's pin comment names ci.yml as the second site to move), and rewrite the comments at 17-20 and 54-58 to describe the pins. Acceptance: the workflow names no floating channel; `grep -n 'cargo +nightly' .github/workflows/ci.yml` is empty; the nightly pin appears in exactly one place or its two sites name each other.

### verification-infra-12: ci.yml comments describe toolchains and a bench compile the tree no longer has
- Where: .github/workflows/ci.yml:54-67 (related: .github/workflows/ci.yml:17-19, 122-133, 202-216; justfile:40; rust-toolchain.toml:29; Cargo.toml:96-106)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the files; no bare `+nightly` exists in the justfile; the CI log at HEAD shows `info: syncing channel updates for nightly-2026-06-30-x86_64-unknown-linux-gnu` at the doctest step and `the toolchain '1.97.1-x86_64-unknown-linux-gnu' is currently in use (overridden by '/home/runner/work/rumors/rumors/rust-toolchain.toml')` at the stable install step; history: the nightly comment dates from 788b5366f when `nightly_toolchain := "nightly"`, pinned dated in e7a4b7b0c; the lib-as-bench comment (89369ed47) predates `bench = false` (67ac9846e))
- Verification: confirmed; history: deliberate-but-expired on all four points
- Owner-gated: no

Four claims in the workflow's comments are contradicted by the justfile,
Cargo.toml, and the runner's own log: (a) "The doctest and fuzz recipes
invoke `cargo +nightly`": every nightly leg invokes `+nightly-2026-06-30`,
which rustup provisions itself at first use, so the installed floating
`nightly` is unused and the instruments job's "tracks nightly, so a format
bump upstream can turn the leg red" cannot happen under the pin; (b) "a
current stable toolchain (edition 2024 needs 1.85+)": rust-toolchain.toml
pins 1.97.1 and overrides the installed stable; (c) "the rumors
bench/release tier, whose lib-as-bench compile is known to exceed 16 GiB"
names a compile that `[lib] bench = false` removed; (d) the coverage job
installs `llvm-tools` on floating nightly, not the pinned nightly the
branch leg runs (cargo-llvm-cov provisions the component itself, so the
step is inert rather than wrong).

Evidence:

    .github/workflows/ci.yml
        56	      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
        57	      # needs to exist, not be the default — whereas the gate's bare
        58	      # `cargo fmt`/`clippy`/`check` must hit stable.
       124	  # never the rumors bench/release tier, whose lib-as-bench compile is
       125	  # known to exceed 16 GiB; that build lives in `ci`'s bench-build leg,
       130	  # format_version. This job tracks nightly, so a format bump upstream can
       131	  # turn the leg red on an untouched tree — the checker refuses loudly,

    justfile
        40	nightly_toolchain := "nightly-2026-06-30"

    rust-toolchain.toml
        29	channel = "1.97.1"

    Cargo.toml
       106	bench = false

Resolution: install the pinned nightly by name (read `nightly_toolchain`
from the justfile in a step, or drop the nightly install and rely on
rustup's provisioning as AGENTS.md describes), rewrite the four comments to
today's mechanism, and either install `llvm-tools` on the pinned nightly or
state that cargo-llvm-cov provisions it. Acceptance: no comment in ci.yml
names a floating `nightly` as the toolchain a recipe uses, or a lib bench
compile.

### verification-infra-13: `just --list` shows mid-sentence fragments for eight recipes
- Where: justfile:111-114 (related: justfile:287-289, 314-321, 363-366, 588-594, 633-641, 705-710, 792-807)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read the comment layout above each recipe; just takes the comment line immediately preceding a recipe, or its attributes, as the listing description; I did not run `just --list`, and the sweep's reported output is unverified corroboration)
- Verification: confirmed from layout; history: no-rationale-found
- Owner-gated: no

Where a multi-line explanatory block abuts the recipe with no blank line
and no one-line summary, the tour shows the block's last line. The eight
recipes are `test-all` ("module build only here)."), `citecheck`
("collected test inventory."), `mutants-list` ("capture, and this flag is
what keeps that refusal from ever firing."), `fuzz-build` ("workspace's
formatting leg: the root `cargo fmt --all` cannot reach it."), `fuzzfit`
("surfacecheck recipes carry the same discipline)."), `wasm32-pins`
("gates — the fuzzfit recipes carry the same discipline)."), `doc-figure`
("that lets the build write into the source tree."), and `bench-alloc-ab`
("(never quoted)."). The header calls `just --list` "the tour".

Evidence:

    justfile
       111	# The gate's test run: every feature (the meter suites and the conformance
       112	# module build only here).
       113	test-all *args:
       805	# Reduced-sampling smoke: append `--sample-size 10 --measurement-time 1`
       806	# (never quoted).
       807	bench-alloc-ab target arm="shipped" *filter:

Resolution: for each of the eight recipes, end the explanatory block with a
blank line and add a one-line summary comment directly above the recipe
(or its attribute line), the pattern the other recipes already follow.
Acceptance: every line of `just --list --unsorted` is a complete
sentence.

### prose-hygiene-5: justfile cites formal/PROGRESS.md by section from the build surface
- Where: justfile:731
- Class / severity / confidence: documentation / low / high
- Provenance: verified (justfile:728-734 read; `grep -rnE 'PROGRESS\.md|MODEL\.md'` over the scope finds this line and AGENTS.md:135-136, which states the rule)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The formal-tier recipe comment cites `formal/PROGRESS.md §3/§5`. AGENTS.md's
hard rule says the progress notes are never cited from code, and the
owner's doctrine extends "design docs may cite code, never the reverse" to
the build surface.

Evidence:

    justfile
    730	# invariant preservation); `eventdag` is the progress-lemma oracle and
    731	# schedule-candidate gate (formal/PROGRESS.md §3/§5): DAG acyclicity, totals
    732	# cross-checks, greedy/candidate linearization, replay of the candidate as a

Resolution: Delete the parenthetical; the comment already lists what
`eventdag` checks. Acceptance: `grep -c 'PROGRESS.md' justfile` is 0.

**Nits.** One row per entry; the full record (evidence, provenance, acceptance) is in the evidence file named by the id's key.

| Id | Where | Claim | Resolution |
|---|---|---|---|
| suite-economics-9 | .github/workflows/ci.yml:26-28 | The header quotes an 8 GiB memwatch cap; the tool's default is 32. | Drop the figure, or name `PROC_LIMIT_GB` as the owner of the number. |
| verification-infra-18 | justfile:272-273; .config/nextest.toml:7-10 | A comment path that resolves to nothing (`src/testing/diff_ops.rs`); a dated "today" and 6-second figure. | Fix the path; drop the dated figures. |

## Positives

The finalizers' reports agree that the crate's documentation is, taken whole, unusually good, and the findings above are the residue of a few mechanical passes that each stopped one step short. The items below are drawn from the reports' positives and deduplicated; each names the site so the pattern can be copied.

- The `error` module opens with a per-variant table of replica state and the caller's next action (src/error.rs:10-28), exactly the altitude a caller matching on `Error` needs, and its "counterparty bug: report it" rows keep the conformance-detector framing of the model of record. `Error::Epilogue`'s doc (124-159) is a clear statement of the two-generals residue.
- `Link`'s contract section (src/link.rs:24-68) states, for every clause, the mechanism, the failure it precludes, and the concrete carrier shape that would violate it, and "What a session promises" (279-321) states the `Ok`/`Err`/cancellation contract with its exceptions and puts the timeout with the caller. Every clause has a conformance check with a committed negative control.
- The deadlock-freedom argument in `materialized.rs:26-87` is a model maintainer document: two ordering invariants, why one slot suffices, where the independence premise is supplied and where it is refuted, and the one exception with its reason. `yield_resolve_query!` keeps the test-only trace hooks inside the expansion so the instrument cannot drift from the production order; `queues.rs` gives every channel constructor its own capacity argument.
- `cbor.rs` argues its rejected alternative as mechanism and names the round-trip tests as what holds writers and readers together; `framing.rs` states its memory policy as a contract and turns it into a session-boundary argument; `capture.rs` argues that a rendering with no hexdump is still a byte pin and states its depth-budget invariant once, at `render_node`.
- `join.rs`'s module doc gives the four-case analysis and a candid complexity note; `unknown.rs`'s fused-classification comment explains each verdict as a prune decision; the commit sections in `Tree::react` and `Tree::join` state the panic-atomicity hazard concretely ("byte-for-byte the shape of 'everything was redacted'") and every test they name exists.
- `traverse::act`'s `# Panics` section is close to the one-line proof the doctrine asks of an assert, and the monomorphization boundary is a stated decision at the code that enforces it (act.rs:24-28). `Reconciliation::reconcile`'s doc explains why both the boxed coercion and `inline(never)` are needed.
- Every `must_use` names the concrete loss (`Bootstrap` does nothing until `join`; `Joined` carries a peer or the bookmark; `Retire` leaks the identity; `gossip_when` does nothing until polled). `changes.rs:36-43` names the deadlock a naive `loop { changes.next(); gossip() }` produces and routes the reader to `gossip_when`.
- `expect` and `unreachable!` messages carry one-line proofs across the link layer ("length checked above", "the loop ends at the first vacancy"), the codec (frame.rs:265, async_io.rs:376, codec.rs:116), mirror-common, and the tree (act.rs:169-176, join.rs:230-232); the findings above are the exceptions.
- Hazard sections are complete where they exist: `Endpoint::new`'s `# Errors` names every arm with its trigger; `Listen`'s `# Cancel safety` matches the router's `select!` re-creation exactly; `StreamSender::frame`'s `# Cancel safety` states the exact peer-side consequence and what the caller must do; `Leaf::leaf` carries uniform `# Errors` and `# Cancel safety` sections; `Backend::assume` follows its unspecified-behavior clause with a one-line proof.
- `streaming.rs:1-40` is an orientation map: each child module is named with the one reason it is separate, and it says where to start reading. `SessionStats`'s field docs follow one shape (mechanism, where the count is taken, when it is zero). `Greeting`'s field docs each say why the field rides the greeting.
- The tutorial is a sequence of complete programs with their exact output, the final program is the cumulative one, and step 5's "one driver per end" is called out as the thing to remember; the `Peer` lifecycle example covers every `Retire` outcome in one runnable block; `Rumors::batch` pins both escape routes with `compile_fail` doctests.
- Every test in the crate carries a doc comment, and the finalizers who traced testdocs against bodies found them overwhelmingly accurate; the conformance suite's "What the suite cannot see" section (link.rs:28-55) and `dispute_wire.rs`'s module doc (1-36) state what their checks establish and what they do not.
- `benches/support/latency.rs` states its measurement model as mechanism (which component is deterministic and why, which moves with load, which figure assertions may rest on), and its `# Panics` sections are one-line proofs. `.config/nextest.toml:14-24` names the one collision its timeout budget accepts and the recovery procedure. `Cargo.toml`'s comments on the ciborium pin, the cbor-diag fork, and `[lib] bench = false` state the invariant and the bump procedure.
- Constants carry the failure they prevent, not just their value: `STALLED_TAG`, `CANCEL_DROP_PATIENCE`, `SESSION_PAYLOADS`, `CONTROL_DUPLEX_FILL` in the conformance suite; `ERROR_ROUTE_CAPACITY` in `streams.rs`; `REFERENCE_SLOT_BYTES` and `FAN_SLOT_BYTES` derived from `size_of` with the rationale inline.
- Maintainer comments state the why at branches rather than the next line: the pressure loop's yield (conformance/link.rs:495-500), the cancellation signal's order (861-866), the `Option` in `ChargedNode` (backend.rs:151-155), `PartyGuard`'s unconditional recovery join (gossip.rs:1389-1392), `Early`'s pairing invariant (pump.rs:404-412), `payload_depth_limits_match`'s rejected alternative (start.rs:248-257).
- The V1 retirement's prose pass held almost completely (no `V1`, `LEGACY_MAGIC`, `Alternating`, or `capture_*_v1` survives in prose); the BLAKE3 swap left zero `blake` hits; "mint" is fully purged; there is no TODO, FIXME, or HACK in scope; no src/ or tests/ prose cites `.agent-notes/`, `design/`, or the formal model's documents except the one justfile line (prose-hygiene-5); every Lean citation checked resolves under `formal/lean` with its invariant restated inline.
- `src/reconciliation.rs:237-249` keeps the rejected level-at-a-time wire shape as design rationale without naming the retired protocol: a model for retaining a rejected alternative with no ghost reference. `arb.rs` justifies every generator by the shape it reaches and why version addressing cannot reach it otherwise.
- Deliberate omissions are documented where a reader would ask: `SessionInfo`'s doc explains why there is no session number; `rumors.rs:196-207` explains why `send` returns no `Version`; `SessionStats`'s no-duration boundary says who owns the clock; the two "Defensive-variant exemption" comments say plainly what is untested and why.

## Open questions for Finch

Decisions only the owner can make, deduplicated across the partition and sweep reports, each with the recommendation the finalizers converged on.

1. **Where does the sizing guide live?** `Peer::sync_memory_budget`'s "# Choosing a budget" and its included table run a hundred lines past the method's contract; the 2026-07-23 and 2026-07-24 rulings placed the derivation in this rustdoc, before the crate had docs-only pages (`reconciliation`, `tutorial`). Recommendation: move the guide and the table to a public explanation module beside `reconciliation`, leave the method with its contract and one link, and fix the test-file citations either way (api-audit-9, api-core-15, fresh-eyes-3, api-audit-8).
2. **"seam" and "knob" crate-wide.** "seam" appears on 66 lines (47 rustdoc, four public items) with no definition; "knob" on 46. Recommendation: replace both in one sweep, following the `mint` purge precedent, keeping at most the height-erasure sense of "seam" at `erased.rs:1` if a single anchored use is wanted; in public rustdoc write "boundary" and "setting" now regardless (prose-hygiene-10, mirror-common-34, conformance-22, session-bookmark-5, fresh-eyes-11).
3. **Em-dashes in `//` and `#` comments.** 169 `.rs` comment lines and 95 config-file comment lines carry a true em-dash against the doctrine's spaced double-hyphen; no lint checks it, and twelve reports ask for one decision rather than per-partition edits. Recommendation: rule once, sweep mechanically with the greps in prose-hygiene-9, and add a `tools/` lint on U+2014 in non-doc comment lines wired into `just gate`, so the rule is a check rather than a convention (prose-hygiene-9, prose-hygiene-8, session-bookmark-6, conformance-19).
4. **"honest", "lie", and their cousins.** "honest" is the model of record's term for the trust premise and is borrowed for six other predicates, including two constant names (`HONEST_LEN`, `Knob.honest`), a type (`Dishonest`), and a function (`is_honest_error`); "lie"/"deceived" and the `GreetingLie` type presuppose intent the model excludes, where 408ede87 already chose "misdeclared". Recommendation: one crate-wide prose commit reserving "honest" for the trust sense and renaming the fixtures to accuracy terms (`FULL_LEN`, `Skewed`, `is_injected_cut`, `GreetingMisdeclaration`) (conformance-32, remote-proxy-tests-14, tests-common-23, session-bookmark-16, tests-resource-link-window-6).
5. **First-sentence mood.** `Batch`, `Snapshot`, and `Network` open in the third person, which the doctrine fixes; `Peer`, `Rumors`, `Bootstrap`, and the observers open in the imperative, and one sentence appears in both moods. Recommendation: rule for the third person and sweep the imperative sites, or amend the doctrine; at minimum unify the three duplicated sentences (api-core-17).
6. **A `missing_docs` lint.** Many public variants, fields, and methods under `rumors::error` carry no doc, and nothing in the tree notices. Recommendation: `#![warn(missing_docs)]` in lib.rs, which the clippy leg's `-D warnings` then enforces, with `pub(crate)` narrowing where the sentence would be "internal"; consider `unnameable_types` alongside (api-audit-7, remote-codec-19, api-core-6).
7. **The renderer-vocabulary re-accept witness.** AGENTS.md's class demands a hexdump witness the snapshots cannot produce. Recommendation: restate the witness in the render's own terms (byte-count headers and `h'…'` literals identical, the diff confined to `/ comment /` annotations) and give it a mechanical form (a `snap-witness` recipe); whether the class should survive at all now that annotations are a `/ comment /` layer is your call (prose-hygiene-6, verification-infra-15).
8. **`Error::VersionMismatch.local_protocol`.** Decision 2 of the V1 note keeps `Protocol` public as wire vocabulary. Does that ruling also keep the field as the enum, so the six "select" findings are purely a prose rewrite, or should the variant carry the local wire version as a number like `remote_version`? The `#[repr(u16)]` on the enum has no in-tree consumer either way (api-audit-5, api-core-4).
9. **The CBOR evolution rules.** `tests/cbor_evolution.rs` says the crate documents unknown-field skipping and `#[serde(default)]` semantics; your hand edit 3d16765f9 removed both from lib.rs's compatibility paragraph the day after they were added. Recommendation: re-add them, since "may I add a field?" is the first evolution question a user asks and the tests already pin the answer; if the removal was a ruling not to promise serde's field semantics, reword the test docs instead (tests-wire-format-9).
10. **The off-model note on digest width.** 2d1e6ea5 kept the adversary birthday-floor sentence in `reconciliation.rs` as an explicit "Off-model note"; 9c73d7b4 dropped the label, so the sentence now reads as part of the acceptance. Recommendation: restore the label, or move the sentence after the hostile-peers clause so the argument visibly ends at the accident bound (session-bookmark-34).
11. **The illumos gate run.** Nine copies of one lint-allow comment name illumos among "the gate's targets"; the run is real but recorded only in commit eb4e0e1ba. Recommendation: one line in AGENTS.md's Commands section naming the hand run on ox-east-1, and a platform-free restatement of the comment at all nine sites (remote-proxy-22, remote-adapter-streams-11).
12. **A worked `Bookmark` implementation.** The only visible impl is `NoBookmark`; `store`'s atomicity obligation has "unspecified corruption" as its cost and no example shows the temp-and-rename shape. Recommendation: a compiled `# Examples` block with a minimal file-backed impl; shipping that impl behind a feature is a separate API decision (fresh-eyes-8).
13. **The first greeting's cost, and `warm_caches`.** On a freshly built replica the first greeting hashes and bounds the whole tree inside one poll, stated only in a `pub(crate)` doc; `warm_caches`, which pre-pays it, is `doc(hidden)`. Recommendation: one public sentence in the runtime-independence section either way; making `warm_caches` public is an API addition for you to rule on (async-hazards-4).
14. **One home for the 24-byte argument.** 2d1e6ea5 deliberately kept the width derivation in both `hash.rs` and `reconciliation.rs`; the copies have since drifted and the crate-private one names the wrong actor. Recommendation: cut `Hash`'s copy to what is local to the type and link the reconciliation section; the minimum fix is re-syncing the Grinding bullet (tree-typed-3).
15. **Seed-file comments.** `proptest-regressions/shadow_validity.txt` carries a note that "awaits owner disposition" since 2026-08-13, and six seed files carry past-tense "minted" provenance the `mint` purge exempted on a byte-stability premise that concerns `cc` lines, not comments. Recommendation: keep the hash as a valid replay case, delete its stale shrink note and the five-line comment, and delete or restate the six comments in the present tense (tests-observation-38).
16. **`MERKLE_HASH_LEN`'s public wording.** The constant's second paragraph is a verbless fragment; user-facing prose gets your final say. Recommendation: the sentence proposed in tree-typed-2, linking `crate::reconciliation#twenty-four-byte-digests`.
17. **Wider dialect sweeps.** "genuine(ly)" appears at about 90 sites and rustdoc carries 1394 em-dashes, which the doctrine permits "sparingly"; neither pass read every site. Recommendation: a dedicated prose pass for the pure-intensifier subset of "genuine(ly)" (the partition entries list their own sites) and a taste ruling on em-dash density in rendered prose (prose-hygiene open questions 2 and 3).
18. **Exact hop pins or bands.** The window suites keep headroom bands by 814f07ad's design and record the current measured hop counts in comments nothing enforces. Recommendation: keep the bands, drop the recorded measurements, and let the `eprintln!` lines be the record (tests-resource-link-window-24).
19. **`Speaker` and `observe::Role`.** Two public enums name one elected role because the observation hook is rumors-blind; the reason lives in a commit message. Recommendation: one sentence at `Speaker` stating it; unify only if the separation is no longer wanted (remote-codec-32).
20. **Owner phrasing in `link.rs`.** The pooled-flow-control paragraph (link.rs:107-113) is your wording and narrates a past observation a test now pins; `Dial::recycle` obligates pooling dials around a byte whose value the contract never states. Recommendation: present-tense restatement naming the pin; state that the ready byte's value is unspecified rather than exposing `READY` (link-2, link-15).
21. **"V2" in prose that describes behavior.** The variant is deliberately named `V2`; "V2 protocol sessions" and "the V2 protocol's ordering" in test harness docs distinguish nothing today. The wire-format finalizer judged "a V2 session" a present-tense fact; the tests-common finalizer flagged the qualifiers. Recommendation: keep `V2` where the wire number is the subject and drop it where behavior is (tests-common-7; the session-bookmark report's open question on gossip.rs:51, 1276).
22. **Test-only inline `mod tests {}`.** Several test-only modules carry inline test blocks against the sibling-file convention. Recommendation: exempt test-only modules explicitly in AGENTS.md rather than adding ceremony (raised by the streaming-backend-window report; the entries are of other classes).
23. **The reordering tripwire's docs and case count.** `REORDER_BATCH` and the reordering helper are documented as inverting "most bursts" and delivering "worst-case-legal stream reordering", while the property they serve proves no inversion is reachable under the joined-endpoint driver (cbc4a0aa's deliberate zero-inversion tripwire), and its 48 cases were chosen as "fewer than the default" rather than as the minimum that keeps the tripwire alive. Recommendation: rewrite the helper docs to say no inversion is reachable and why (ungated), and reduce the property to one deterministic case over `early_first_child_dispute_pair` asserting `reordered == 0`, leaving the decorator's coverage to the conformance suite's `ReversingAcceptor` tests (remote-proxy-tests-6).
24. **The `Backend` family's audience.** `backend.rs` addresses a third-party storage implementer the crate cannot admit; the sync-budget record defers publishing the boundary. Recommendation: add the one-sentence "crate-internal today; `Local` is the sole production implementation" status to the module doc now, and schedule the trait-shape pass (an owned `Vec` in `parent`, the `pub(crate)` alias in `assemble`'s signature, the `Error` bounds) for the commit that publishes the boundary (streaming-backend-window-1).
25. **Window-module prose placement and one public first sentence.** Three window-side items reverse or touch recorded placements: moving the flushed-question derivation from `window.rs` onto `queues::local_questions`, where its premises become resolving links (reverses b76a31f3's placement); stating the run-proportional transient `Local::assemble`'s buffer introduces, either in prose as a `size_of` expression under the budget's "replica itself" exclusion or by rebuilding incrementally; and rewording `DEFAULT_SYNC_MEMORY_BUDGET`'s public first sentence, which drops the "on pipelining" qualifier the method carries and names a `cfg(test)` constant. Recommendation: move the derivation; price the buffer in prose now and build the incremental builder only if a census shows it matters; take the proposed first sentence (streaming-backend-window-24, streaming-backend-window-10, streaming-backend-window-27).

## Counts

Entries of class documentation at commit 9e5784fb, as finalized on 2026-09-01.

| Module | Medium | Low | Nit | Total |
|---|---:|---:|---:|---:|
| Crate root and public surface | 5 | 11 | 13 | 29 |
| Session and bookmark | 3 | 13 | 9 | 25 |
| Link | 1 | 6 | 5 | 12 |
| Conformance | 0 | 6 | 10 | 16 |
| Tree core | 6 | 7 | 6 | 19 |
| Tree typed | 2 | 8 | 2 | 12 |
| Mirror common | 0 | 13 | 7 | 20 |
| Streaming backend and window | 1 | 6 | 7 | 14 |
| Materialized | 2 | 6 | 7 | 15 |
| Streaming tests | 3 | 4 | 1 | 8 |
| Remote codec | 0 | 11 | 3 | 14 |
| Remote capture and codec tests | 1 | 2 | 4 | 7 |
| Remote adapter and streams | 1 | 5 | 8 | 14 |
| Remote proxy | 1 | 12 | 6 | 19 |
| Test scaffolding | 1 | 2 | 2 | 5 |
| Integration tests | 14 | 23 | 17 | 54 |
| Benches and examples | 2 | 6 | 14 | 22 |
| Verification recipes and configuration | 1 | 5 | 2 | 8 |
| **All modules** | **44** | **146** | **123** | **313** |

No entry of this class was rated high. By provenance, 258 of the 313 entries are marked verified (a grep, a git query, or a mechanical comparison of prose with code) and 55 assessed (read); none is demonstrated. Twenty-two entries are marked owner-gated in whole or in part, and each of those appears in the open questions above.
