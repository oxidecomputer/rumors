<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T82, T124, T125, T126, T128, T129, T130, T131, and T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: residue of the V1 retirement and the four wire respellings

## Goal

No prose describes the retired V1 protocol or the wire before its
four respellings (the CBOR respelling, the two-item frame opener,
version addressing, the suffix-only leaf preimage): nothing "selects" a
`Protocol` (T82, which also deletes the vestigial `#[repr(u16)]` and
the `Default` derive), no doc names `Levels`, the zipper, the typed
tower, or the node codec, no testdoc speaks V1 phase vocabulary, and
every wire figure and shape in prose is today's (a 30-byte preamble, a
one-item greeting, a two- or three-item frame, a version-addressed
tree, a suffix-only preimage). T82 disposes module-graph-4,
prose-hygiene-2, and mirror-common-12 as well as its four rows; the
ledger cites T132 for those three, and this brief reports the mismatch
rather than resolving it. Effort: medium (prose; two derive deletions).

## Ground rules

These apply to every P4 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `<base sha>` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `<base sha>`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, a `--self-test` case, or a reversible mutation whose observed
  failure the commit message records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under `<scratchpad>/p4-v1-residue/`,
  polled with short foreground checks (the foreground command cap is ten
  minutes). Keep every working file under that directory. `just all` and
  `just ci` are run once each at the end, not per commit.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.
- **Prose.** `PROSE.md` in this directory binds this lane (ruling T141): every
  paragraph you touch passes its three tests (altitude, concision,
  legibility), and the diff is net shorter in prose unless your report says
  what the added sentences buy.
- **Machine.** The illumos box (`ox-east-1`, per the `building-on-illumos` skill) is
  where a lane builds, tests, and gates. The wrapper syncs the Mac
  worktree to `~/src/<worktree basename>` on the box and runs one command
  there with its own target directory, so lanes do not collide; cargo
  runs `--locked` there; nothing is edited or committed on the box. A
  clean gate on the box is the gate of record for a commit; the Mac runs
  no gate (Finch's ruling). The box gate is `on-illumos.sh <worktree> 'just gate'`. One leg is
  expected red there and counts as clean when it is the only failure:
  `fuzz`, because libFuzzer has no illumos port (`FuzzerPlatform.h`
  refuses the target); a lane quotes that line and runs no fuzz build
  elsewhere (Finch's ruling: fuzzing is CI's). clippy's
  `missing_const_for_thread_local` misfires on illumos, where
  `thread_local!` expands through the OS-keyed path; `before` carries an
  illumos-scoped crate-level allow, so a lane based before that landed
  either rebases or passes `RUSTFLAGS="-A clippy::missing_const_for_thread_local"`
  in the remote command for that one run. Two legs that
  pin toolchain-derived numbers may fire on the box if its toolchains
  differ from the pinned ones; a lane reports such a leg with both numbers
  rather than re-pinning anything. `tools/memwatch` is deleted by the
  `p1-memwatch` lane, which runs first and unstacked; `p1-gate` rebases
  onto it. Benchmarks whose committed baselines are the Mac's run on the
  Mac, once, on a quiet machine. Clock guard, checked before every box
  run: rsync preserves mtimes and cargo's rebuild detection is
  mtime-based, so a box clock ahead of the Mac by more than a couple of
  seconds means a green build of stale code; on skew, a lane either runs
  with a fresh target directory on the box (a cold build, no stale
  artifact to trust) or waits, and says which. Stepping the box's clock is
  admin work on a shared machine and is Finch's, never a lane's. The
  builder cap counts Mac builders only; on the box (96 cores, 1 TiB) there
  is no lane cap, only the load: hold a launch while the one-minute load
  average sits above about 150 on 192 threads, and keep wall-time
  measurements under `pset-run` to one at a time, announced in the merge
  queue first.
- **Out of scope.** The formal tier (`lean`, `eventdag`, `muxprobe`, everything under
  `formal/`) and `before`'s bench judge (`bench-judge`,
  `bench-judge-tripwire`) are never run or edited by a rumors lane; a
  recipe that composes them (`all`) is exercised by its other legs
  individually. A rustdoc on a Rust-side literal derived from the Lean
  artifact is Rust prose and may be edited where a ruling names it.

## Mechanism, in order

1. **The T82 family, one commit**: the six prose sites, the two
   `Display` strings (byte-identical to each other; no snapshot pins
   them, which `just test` confirms), `#[repr(u16)]`, `Default`, and
   `tests/handshake.rs:59`.
2. **Ghost machinery**, one commit: tree-core-26, module-graph-16,
   prose-hygiene-1, tree-typed-19, tree-core-1 (with its
   `streaming/tests.rs` site), tree-core-23 and its two out-of-partition
   sites. `mod act;` goes private; `just docs-internal` (private items
   under `-D warnings`) is the standing check for every intra-doc link
   the narrowing moves.
3. **Wire respellings**, one commit per site family: the preamble
   (testing-infra-16, module-graph-5), the greeting (remote-proxy-tests-1,
   remote-proxy-1), the frame (remote-codec-20, remote-capture-atlas-21,
   remote-capture-atlas-2, remote-codec-4), addressing (tree-core-18,
   tests-resource-link-window-4, tree-typed-5), the adapter
   (streaming-backend-window-19, remote-adapter-tests-17).
4. **Phase vocabulary and qualifiers**: tests-wire-format-6,
   tests-wire-format-3, streaming-tests-5, tests-disruption-handshake-9,
   tests-resource-link-window-9, the `SessionStats` bullet is
   `p4-tallies`' (fresh-eyes-5).
5. **Closing commit**: the union grep, empty.

Oracle (empty output is the acceptance; no standing check exists for a
ghost, so this is a one-time pass with its site list in the report):

    git grep -n -i -E 'select(ed|able)? .*protocol|protocol.*select|we selected|selectable|both protocol|the protocols\b|\bLevels\b|zipper|typed tower|25-byte|version frame|listing frame|version header|greeting frames|one- or two|count-minus-one|pre-batching|content-address|content addressing|\(version, payload\)|Reply<B, H>|Convert::assemble|tree::wire|serialize_to|DecodeNode|node serializer|never length-prefixed' -- src tests benches
    git grep -n -E 'Exchange|Opening|Closing|to Done|recursive' -- tests/gossip_snapshot.rs
    git grep -n -E 'generic over the payload|uncertain|Closing' -- src/tree/mirror/streaming/tests.rs

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **api-audit-5** (low; T82). `src/protocol.rs:1-19`. Resolution: rewrite protocol.rs:1 as the dialect the crate speaks (one today; a wire change adds a variant), the error.rs:14 row as "the peers run releases speaking different wire versions: align crate versions", the display at error.rs:76 as "we speak", error.rs:179 and 201 as "the rumors dialect", and peer.rs:603-605 as "like a wire-version upgrade"; drop `#[repr(u16)]` (or state at the enum why a repr matters when the wire carries a CBOR uint), adjusting tests/handshake.rs:59 accordingly. Acceptance: `grep -rn -i 'select' src/protocol.rs src/error.rs src/peer.rs` finds no protocol-selection prose; the `Protocol` doc states that the crate speaks one dialect.
- **api-core-21** (nit; T82). ``src/protocol.rs:13-18``: `Protocol` derives `Default` for a call site that no longer exists Resolution: Remove `Default` from the derive list and the `#[default]` attribute
- **api-core-4** (medium; T82). `src/error.rs:14`. Resolution: protocol.rs:1 becomes "The wire protocol version a session speaks."; error.rs:14 becomes "the two releases speak different wire versions: align crate versions"; error.rs:76 becomes "peer speaks rumors protocol version {remote_version}; this release speaks {local_protocol:?}"; error.rs:179 and 201 say "this release's dialect"; peer.rs:512 says "When a session supplies a subtree the counterparty lacks"; peer.rs:604 says "like upgrading to a release that speaks a new [`Protocol`] version". Apply the same wording to the out-of-partition twins at handshake.rs:180 and 204 and gossip.rs:723. Acceptance: `grep -rn -i select src/protocol.rs src/error.rs src/peer.rs` returns no line pairing selection with `Protocol` or a dialect, and no doc or message implies a protocol choice the API does not offer.
- **fresh-eyes-2** (medium; T82). `src/error.rs:14`. Resolution: error.rs:14: keep only the second clause ("the two ends run releases whose wire versions differ: align crate versions"). error.rs:76 and handshake.rs:180: name the local wire version ("we speak {local_protocol:?}"). error.rs:179, :201 and handshake.rs:204: "the dialect" or "the wire's". protocol.rs:1: drop "Selectable" ("The wire reconciliation protocol."). peer.rs:603-604: compare to a different fleet-coordinated event, or cut the comparison. gossip.rs:723-729: state the one body ("`Reconciliation::reconcile` returns its future boxed so the protocol state machine stays in this crate's object code") and delete "Both branches" and "neither concrete protocol state machine". Acceptance: `grep -rn "select" src/protocol.rs src/error.rs src/peer.rs src/peer/gossip.rs src/tree/mirror/handshake.rs` matches only settings setters (the `payload_depth_limit` line at error.rs:109 and the builder docs), never `Protocol`.
- **tree-core-1** (medium; T124). `src/tree.rs:58-61`. Resolution: Reword the three sites to state the actual relationship: `join` and the streaming mirror implement the same deletion-honoring predicate (a subtree causally at or before the counterparty's version drops out) in two walks, and their agreement is pinned differentially (`agrees_with_materialized_oracle` against `traverse::unknown`; `streaming_matches_join_oracle` against `Tree::join`). In `unknown.rs`, state the module's present role: the in-memory filter `join` calls and the oracle the materialized pruner is checked against. Route the same rewording to `streaming/tests.rs:154-156` (out of partition). Acceptance: no prose under `src/tree/` says the mirror delegates to or uses `traverse::unknown`; each site names the differential test that carries the identity.
- **tree-core-23** (medium; T124). `src/tree/arb.rs:235-236`. Resolution: Line 327: "Paths are functions of the version alone, so a candidate pair is fully determined by where each side's version chain starts" (the "payloads are unit" clause then goes). Lines 235 and 301: "Version-addressed generators" / "Version addressing". Route the two out-of-partition sites to the streaming partition. Acceptance: `grep -n -i 'content-address\|content addressing\|(version, payload)' src/tree/arb.rs` returns nothing; every statement of the path function in `arb.rs` agrees with `Path::for_leaf`.
- **prose-hygiene-1** (medium; T125). `src/tree/typed/node.rs:428-447`. Resolution: Delete node.rs:428-447; if the path-compression sentence at 444-446 is wanted, restate it against `Node::beneath` without the wire framing (the V2 node vocabulary lives in src/tree/mirror/streaming/message.rs and the codec under remote/codec/). In traverse.rs:9-12 replace the `Levels` example with a link that exists today (src/tree.rs:49 and :475, src/tree/typed/untyped/iter.rs:172, src/tree/mirror/streaming/materialized/ unknown.rs:5 all link into `traverse::act` or `traverse::unknown`). In tests/future_size.rs re-denominate against the streaming protocol's type-level phase schedule (the `protocol` module's stage traits) and name the boxing sites that exist: `Handshaken::reconcile` and `Reconciliation`. Acceptance: `grep -rnwE 'Levels|Below|DecodeNode|serialize_to|tree::wire' src tests` returns nothing, and future_size.rs's module doc names only items that resolve.
- **tree-typed-19** (medium; T125). `src/tree/typed/node.rs:428-447`. Resolution: delete node.rs:428-447 (the file then ends after the `PartialEq` impl; the one surviving fact, that singletons never materialize and reconstruct through `beneath`, already lives at `Hash::branch`'s canonicity section and `untyped::Node::branch`). At hash.rs:16-17 either drop the wire sentence (the codec module owns framing) or restate it: the digest travels as a 24-byte CBOR byte string whose head the decoder checks against `MERKLE_HASH_LEN`. At hash.rs:92 and 129 cut "as the node serializer emits it"; the sentence already says "shallowest byte first". Hand tree.rs:36 to the tree-root partition. Acceptance: `grep -rn 'tree::wire\|serialize_to\|DecodeNode\|count_minus_two\|Read::chain\|node serializer\|never length-prefixed' src` returns nothing; `just doclint` clean.
- **tree-typed-5** (medium; T125). `src/tree/typed/hash.rs:60-64`. Resolution: restate: the preimage commits the compressed suffix alone; because a leaf's path is the full-width hash of its version, the suffix (with the prefix above it) commits the version transitively, and no version or message bytes enter. Acceptance: `LEAF_TAG`'s doc, `Hash::leaf`'s doc, and `leaf_preimage_layout`'s doc name the same field list.
- **remote-adapter-tests-17** (medium; T126). `src/tree/mirror/streaming/remote/adapter/tests/parking.rs:13-20`. Resolution: apply the standing ruling: excise the megabyte figures from the module doc and cite the mechanism and the enforced constant by name ("the encoded reply plus the decoded fan² skeleton, pinned by `DISPUTED_REPLY_TRANSIENT_CEILING`"); have message.rs name the same constant so there is one number of record, or give the crate one visible constant both cite. Acceptance: `grep -n MB` over parking.rs returns no figure disagreeing with the pin; the module doc, the constant doc, and message.rs:14-17 agree or defer to the named constant.
- **remote-capture-atlas-2** (medium; T126). `src/tree/mirror/streaming/remote.rs:30-31`. Resolution: Delete the sentence and let codec.rs own the grammar (finding 1), or restate it: "An empty query occupies its signal alone; a nonempty query's body is a `{radix: hash}` map of one to 256 children, its map head carrying the count." While there, lines 33-41 omit the record's tag-63 wrapper and the version atom's own tag that codec.rs:44-46 states; acceptable at this altitude if deliberate, but decide. Acceptance: `grep -rn count-minus-one src` is empty and any remaining sentence matches `write_listing`.
- **remote-proxy-1** (medium; T126). `src/tree/mirror/streaming/remote/proxy/error.rs:19-27`. Resolution: reword each summary from its source's contract, keeping to one line each (the owner's PR #38 directive on this enum, reported by the history pass, asks for parsimony): "Reading the peer's greeting item failed." / "Writing and flushing the local greeting item failed." / `Stream`: "An incoming logical stream failed, or its stream supply closed before it arrived." / `Accept`: "An incoming transport stream violated the session's stream discipline." Acceptance: `grep -rn "greeting frames" src/` is empty; the `Accept` summary no longer claims acceptor failure; each summary names only outcomes its wrapped enum produces, checked against `StreamError` (streams.rs:265-298) and `AcceptError` (streams.rs:796-826).
- **streaming-backend-window-19** (medium; T128). `src/tree/mirror/streaming/convert.rs:1-8`. Resolution: Rewrite around what the module does: explode a height-`H` node stream to the prefix-ordered leaf stream beneath it and reassemble height-`H` nodes from one, level by level through `Backend::children`/`Backend::parent`; the wire carries leaves, never nodes, so the encoder runs `Backend::leaves` and the decoder `Backend::assemble` on every supplied node, and a backend may override both in bulk. One sentence may note that leaf records are backend-neutral, so two peers' backends need not coincide. Renaming the module is optional taste. Acceptance: the module doc names the encode/decode call sites as consumers and no longer states that a homogeneous session pays nothing.
- **streaming-tests-5** (medium; T128). `src/tree/mirror/streaming/tests.rs:99-105`. Resolution: 102-104: "The skeleton-bridge harness ([`skeleton`]). Payload twins ([`announced`]) differ only in leaf contents, which the erased `Root` carries opaquely, so both run through identical machinery." 179-180: delete (the invariant sentence at 176-177 stands alone) or restate against today's code: the leaf-parent answerer merge-joins both leaf listings, supplying its exclusive leaves and issuing a leaf request for each it lacks. Acceptance: `grep -n 'generic over the payload\|uncertain\|Closing' src/tree/mirror/streaming/tests.rs` is empty.
- **testing-infra-16** (medium; T129). `src/tests.rs:25-28`. Resolution: Rewrite 25-27 as "The preamble's fixed wire length, the handshake's own constant, so the fuse budgets below land on exact protocol boundaries." (or drop the alias and import `V2_PREAMBLE_LEN`, whose doc at handshake.rs:50-52 carries the correct decomposition). Rewrite 262-264 as "The wire length of `retiree`'s greeting item, the one control-stream item after the preamble, so a fuse budget can land on an exact protocol boundary." Acceptance: neither doc contains a byte breakdown or the word "frame" for the greeting; handshake.rs:16 remains the single place the width is stated.
- **tests-disruption-handshake-9** (medium; T130). `tests/gossip_pipelining.rs:5-8`. Resolution: Rewrite as "from the public knob, through the streaming mirror, to the channels" (or drop the middle clause). Acceptance: `grep -rn 'both protocol' tests` returns nothing.
- **tests-wire-format-3** (medium; T131). `tests/gossip_snapshot.rs:49-54`. Resolution: Drop the number ("After the fixed-width preamble the two sides exchange greetings ...") and reflow the paragraph; the snapshot's header is the pin of record for the width. Acceptance: the doc carries no byte count, and `grep -n '25-byte' tests/gossip_snapshot.rs` returns nothing.
- **tests-wire-format-6** (medium; T131). `tests/gossip_snapshot.rs:484-499`. Resolution: Restate against the pinned snapshot: the disjoint sets collide in their leading path byte, so stages open below the root (the pin shows Responder streams at heights 31 and 29 and Initiator streams at heights 31 and 30) that the two-or-three-message scenarios never reach; drop "recursive". For `converged_forks_noop`: "the equal-versions resolution completes both sides without opening the descent". Fix the sibling ghost at streaming/tests.rs:180 in the same pass. Acceptance: `grep -n -E 'Exchange|Opening|Closing|to Done|recursive' tests/gossip_snapshot.rs` returns nothing; the docs name heights or stages the snapshot shows.
- **mirror-common-12** (low; T132; disposed by T82 as well; the ledger cites T132). `src/tree/mirror/handshake.rs:180-184`. Resolution: "peer speaks rumors protocol version {remote_version}; this build speaks {local_protocol:?}" here and, byte-identical, at error.rs:76; handshake.rs:204 "opened as a rumors stream of this build's dialect"; sweep error.rs:14, :179, :201 and protocol.rs:1 ("Selectable") in their own partitions. Dropping `local_protocol` from the public variant is the further step the retirement note already declined. Acceptance: `grep -rn -i 'we selected\|selected dialect\|selectable' src` is empty; the two `VersionMismatch` strings are identical; tests/handshake.rs still passes.
- **module-graph-16** (low; T132). `src/tree/traverse.rs:9-13`. Resolution: In `traverse.rs`, make `act` private (`mod act;`) and re-word the comment to cover `unknown` alone, naming the live link sites (`tree.rs:49`, `typed/untyped/iter.rs:172`, `materialized/unknown.rs:5`). In `typed.rs`, state the real reason: the erased node type and the census are used by the streaming backend and the test facade. Acceptance: `just docs-internal` stays clean (it documents private items with `-D warnings`, so a link the narrowing breaks fails there), and neither comment names an item that does not exist.
- **module-graph-4** (low; T132; disposed by T82 as well; the ledger cites T132). `src/protocol.rs:1-11`. Resolution: Rewrite `protocol.rs:1` as the wire-version vocabulary one release speaks (the enum's own doc at `:7-11` already says the right thing about frozen wire formats and new variants); re-word `error.rs:14` and `peer.rs:604` to "both ends must run releases speaking the same protocol version"; change the two `Display` strings to "we speak {local_protocol:?}"; drop "selected" at `handshake.rs:204`. Acceptance: `grep -rn -i 'select' src/protocol.rs src/error.rs src/peer.rs src/tree/mirror/handshake.rs` returns only `Peer::payload_depth_limit`'s "select the same" (a configuration setting that exists) or nothing.
- **module-graph-5** (low; T132). `src/tree/mirror/streaming/protocol/peer.rs:108-111`. Resolution: Fix each comment toward the code: `driver.rs`, with "the compiler holds the two counts together through the type-level height descent" in place of "the counts must move together"; the 30-byte `55799(["rumors", version, network, intent])` layout; `handshake.rs` owns the preamble constants (drop the clause from `gossip.rs:4-5`); `tree::mirror::handshake::preamble`. Acceptance: the four comments name the file and layout that exist; no new constant or assert is added.
- **prose-hygiene-2** (low; T132; disposed by T82 as well; the ledger cites T132). `src/error.rs:14`. Resolution: error.rs:14: "the two ends speak different wire versions: align crate versions". error.rs:76 and handshake.rs:180: drop "we selected" ("peer speaks rumors protocol version {remote_version}, this side speaks {local_protocol:?}"); no snapshot pins either string. protocol.rs:1: "The wire reconciliation protocol, versioned." peer.rs:603-604: cite an event that exists (a crate upgrade that bumps the protocol version). gossip.rs:723-729: rewrite for one protocol. Keeping `Protocol` as a `#[non_exhaustive]` versioned enum is outside this finding. Acceptance: `grep -rniE 'select(ed|able)? .*protocol|protocol.*select' src` returns nothing.
- **remote-capture-atlas-21** (low; T132). `src/tree/mirror/streaming/remote/codec/error.rs:131-133`. Resolution: "The frame item is not a two- or three-element CBOR array." Acceptance: the variant doc agrees with `frame_arity`. Dedupe against the codec-core partition if it reports the same line.
- **remote-codec-20** (low; T132). `src/tree/mirror/streaming/remote/codec/error.rs:131-133`. Resolution: "The frame item is not a two- or three-item CBOR array (the opener's stream and state, then a body when the state takes one)."; consider "frame is not a CBOR array of two or three items" for the Display text (a wire-neutral message change, but the atlas snapshot pins it, so re-accept deliberately). Acceptance: the variant doc and Display agree with `frame_arity`'s accepted range and detail strings; no prose in the partition says "one- or two".
- **remote-codec-4** (low; T132). `src/tree/mirror/streaming/remote/codec/budget.rs:108-110`. Resolution: "degrading a zero budget to one leaf per frame, and the". Acceptance: `grep -rn pre-batching src` returns nothing.
- **remote-proxy-tests-1** (low; T132). `src/tree/mirror/streaming/remote/proxy/start/tests.rs:73-74`. Resolution: Rename to what each test does to the one item: `truncated_item_head_is_a_typed_read_error`, `over_declared_item_length_is_a_typed_read_error`, `empty_item_is_a_typed_decode_error`, `trailing_item_bytes_are_rejected`, `cut_item_content_is_a_typed_read_error`. Sweep src/tests.rs:262-264 in the same commit. Acceptance: `grep -rn -i 'version frame\|listing frame\|version header\|version_frame\|listing_frame\|version_header' src` returns nothing.
- **tests-resource-link-window-4** (low; T132; same file as prose-hygiene-12: land only if present at base). `tests/async_wire.rs:26-27`. Resolution: State what the two checks pin: equal version sets (`hash`) and equal causal frontier (`latest`); fix the echo at lines 39-41 ("byte-identical (`hash`)"). Acceptance: no doc in the file claims byte identity from the hash.
- **tests-resource-link-window-9** (low; T132). `tests/latency_link.rs:3-6`. Resolution: latency_link.rs:5: "measures the protocol". tcp_link.rs:3-4: "the simulations' direct, one-connection-per-stream Link over real sockets" (the property tcp.rs:18-21 explains). Acceptance: both sentences are true of today's tree.
- **tree-core-18** (low; T132). `src/tree/tests.rs:29-33`. Resolution: `distinct_bytes`: "Generate a vector of distinct `Bytes`. Paths derive from versions, so distinctness buys nothing for placement; it keeps the payload-keyed maps the shuffle properties build (`index_of`, `meta_by_value`) injective." `idx`: drop "or proptest-generated strings" and state the rule: "Labels are single letters; the index is the letter's alphabet position mod 16, so the letters one test mixes must be distinct mod 16." Acceptance: both docs state the reason the helper is actually needed and no claim about paths or generated strings.
- **tree-core-26** (low; T132). `src/tree/traverse.rs:7-19`. Resolution: `mod act;` (private) with the facade unchanged; re-state the comment to the actual dependent ("`unknown` is `pub(crate)` because the materialized pruner's tests import the `Unknown` trait as their oracle and link it from rustdoc"). Rewrite traverse.rs:3-5: "`act` and `join` are free functions over their per-height traits; `Unknown` is used as a trait by `join` and by the materialized pruner's tests." Drop `use super::*;` and have the children import `crate::tree::typed::*` (as `arb.rs` and `unknown/tests.rs` already do) and link `crate::tree::mirror`. At tree.rs:55-58, list the `traverse` trio as act/join/unknown and introduce `mirror` as the wire counterpart of `join`. At join.rs:6: "(over the wire, pairing queries with replies and building the union on both sides)". Acceptance: `cargo doc` (the gate's `doclint`) resolves every intra-doc link with `mod act` private; `grep -rn '\bLevels\b' src/` and `grep -rni zipper src/` return nothing; `traverse.rs` has no glob import; tree.rs's trio matches `traverse.rs`'s contents.

## Hazards and stops

- `p2-commit-path` (unmerged) rewrites `src/tree.rs`, `traverse/*`,
  `typed/node.rs`, and `arb.rs`: tree-core-1, -23, -26, module-graph-16,
  prose-hygiene-1, and tree-typed-19 all sit there. Launch after it
  merges and re-anchor from the quoted evidence.
- T82 deletes one public derive and one attribute; any further change
  to `Protocol`'s public shape is a stop. Nothing here may move a
  snapshot (tests-wire-format-6 restates against the pinned one).
- `README.md` is derived from `src/lib.rs`: `just readme`.
