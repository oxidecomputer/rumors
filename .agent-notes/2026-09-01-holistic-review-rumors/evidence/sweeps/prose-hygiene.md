# Sweep prose-hygiene: Ghost references, temporal language, dialect tells

## Method and coverage

This is the verification pass over the prose-hygiene sweep at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (verified: `git rev-parse HEAD`
matches, `git status --porcelain` is empty). Every one of the sweep's twelve
findings was disputed by opening the cited lines with line numbers (`sed -n`
piped through `cat -n`), re-running the greps that produced the counts, and
checking history with `git show --stat`, `git log -S`, and `git log
--format=%ci` where a finding rested on a deletion or an ordering of commits.

Mechanical checks run for this pass:

- Deleted-identifier ghost hunt: extracted every `fn|struct|trait|enum|type|
  mod|const|macro_rules!` name on the deletion side of `git show 368da2a5`
  (the `Protocol::V1` retirement; 110 files, 7611 deletions), kept the
  CamelCase and underscore names (202), dropped those with a surviving
  definition anywhere in `src`, `tests`, `benches`, `examples`, or `crates`,
  and grepped the rest across the crate's prose. Surviving ghosts:
  `DecodeNode`, `serialize_to`, `crate::tree::wire` (src/tree/typed/node.rs),
  `Levels` (src/tree/traverse.rs and tests/future_size.rs), `Below`
  (tests/future_size.rs). No `V1`, `LEGACY_MAGIC`, `Alternating`, or
  `capture_*_v1` reference survives.
- Roster-tag anchoring: for each letter-number tag cited from code (`B5`,
  `F4`, `T3`, `D5`, `d5`, `d6`, `finding #6`, `finding #7`, `Bridge 1/2/3`,
  `charter`) grepped `formal/lean/**/*.lean`, `formal/MODEL.md`,
  `formal/PROGRESS.md`, `formal/PLAN.md`, and `.agent-notes/` for a home.
- Lean names cited from code (`wc_impossibility`, `viewEnc`, `LocalEq`,
  `wedge`, `asmResList`, `fan`, `capLevel`): each resolves to a definition,
  theorem, or structure field under `formal/lean`.
- Em-dash census by register: 1394 lines in `///` or `//!` rustdoc, 169 in
  plain `//` comments, 0 trailing after code, 4 inside string literals
  (assert messages), 95 in `#` comment lines across the justfile (57),
  .cargo/mutants.toml (20), .github/workflows/ci.yml (9), Cargo.toml (5),
  .config/nextest.toml (3), .github/workflows/pages.yml (1).
- "seam": 66 lines, 47 of them in rustdoc; no gloss or definition anywhere.
- Snapshot format: `grep -lE '^[0-9a-f]{20,}$'` over every `.snap` finds a
  standalone hex line only in the two bookmark pins; none of the 22 files
  under tests/snapshots has one.
- Display-string pin check for the `VersionMismatch` message: `find tests src
  -name '*.snap' | xargs grep -l 'speaks rumors protocol'` and a grep over
  `tests/` return nothing, so rewording it moves no snapshot.
- Sweep positives spot-checked: `blake` 0 hits, `mint` (word) 0, `TODO|FIXME|
  HACK` 0; adapter/tests/malformed.rs:190's "All eight leaf-query paths" is
  pinned by `assert_eq!(checked, 8)` at :282.

No test invocation was run: none of the findings is a correctness claim.
Not seen: the ~1394 rustdoc em-dashes were counted, not read; the
"silently" and "genuine(ly)" families the sweep left open were not read
per site here either.

## Findings

### prose-hygiene-1: Comments and a test module doc describe the V1 node wire encoding and `Levels` chain by names the retirement deleted
- Where: src/tree/typed/node.rs:428-447 (related: src/tree/traverse.rs:9-12; tests/future_size.rs:3, 7-9, 37-38)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`git show 368da2a5 --stat` lists `src/tree/wire.rs | 267 ---------`, `src/tree/typed/levels.rs | 189 ------`, `src/tree/typed/levels/level.rs | 152 -----`; `grep -rnE '(struct|trait|type|enum) +(Levels|Below)\b' src crates` returns nothing; `find src -name wire.rs` returns nothing; `grep -rnE 'serialize_to|DecodeNode|tree::wire' src tests` returns only the three prose lines in node.rs)
- Verification: confirmed and extended: the sweep's name-based grep missed tests/future_size.rs, whose module doc names the deleted `Levels<Below<…>>` chain and attributes the boxing to `mirror()`, which is `#[cfg(test)]` and unboxed (src/tree/mirror/streaming.rs:142-153); the `BoxFuture` lives in `Handshaken::reconcile` (streaming.rs:110-116) and `Reconciliation` (src/peer/gossip.rs:1128); history: deliberate-but-expired (368da2a5 deleted the identifiers; the prose survived the retirement's prose pass c13c21b4)
- Owner-gated: no

A twenty-line maintainer comment documents a node serialization through
`crate::tree::wire`, `untyped::Node::serialize_to`, and `DecodeNode`, none of
which exist. The `pub(crate)` rationale in traverse.rs names the deleted
`Levels` docs as its example linker. tests/future_size.rs explains its budget
by a `Levels<Below<…>>` chain that no longer exists and credits the erasure
to a test-only function that does not box.

Evidence:

    src/tree/typed/node.rs
    428	// Wire format (see [`crate::tree::wire`]). Serialization is
    429	// height-uniform: every typed `Node<H>` delegates to
    430	// [`untyped::Node::serialize_to`], which emits the in-memory
    ...
    436	// Deserialization at typed height `H` ([`DecodeNode`]) reads `prefix_len`, then either

    src/tree/traverse.rs
    9	// `act` and `unknown` are `pub(crate)` so rustdoc elsewhere (e.g. the
    10	// `Levels` docs) can link to the traversal traits inside them: a private

    tests/future_size.rs
    3	//! The mirror protocol's `Levels<Below<…, Below<…, …>>>` chain is ~30 deep,
    ...
    37	/// The erasure is `mirror()`'s internal `Pin<Box<dyn
    38	/// Future>>`, so the protocol's `Levels` chain doesn't appear in the

Resolution: Delete node.rs:428-447; if the path-compression sentence at
444-446 is wanted, restate it against `Node::beneath` without the wire
framing (the V2 node vocabulary lives in src/tree/mirror/streaming/message.rs
and the codec under remote/codec/). In traverse.rs:9-12 replace the `Levels`
example with a link that exists today (src/tree.rs:49 and :475,
src/tree/typed/untyped/iter.rs:172, src/tree/mirror/streaming/materialized/
unknown.rs:5 all link into `traverse::act` or `traverse::unknown`). In
tests/future_size.rs re-denominate against the streaming protocol's
type-level phase schedule (the `protocol` module's stage traits) and name the
boxing sites that exist: `Handshaken::reconcile` and `Reconciliation`.
Acceptance: `grep -rnwE 'Levels|Below|DecodeNode|serialize_to|tree::wire' src
tests` returns nothing, and future_size.rs's module doc names only items
that resolve.

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

### prose-hygiene-9: Em-dashes in line comments (`//` and `#`) across 264 sites
- Where: src/peer/gossip.rs:879 (related: 169 plain `//` comment lines in .rs files, led by src/peer/gossip.rs (20), src/tree/mirror/streaming/window.rs (12), examples/swarm.rs (7), tests/gossip_when.rs (6), tests/causal.rs (6), src/tree/tests.rs (6), src/tests.rs (6), src/tree.rs (5); 95 `#` comment lines: justfile (57), .cargo/mutants.toml (20), .github/workflows/ci.yml (9), Cargo.toml (5), .config/nextest.toml (3), .github/workflows/pages.yml (1))
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (counts reproduced: 169 lines matching `^\s*//([^/!]|$)` with an em-dash, 0 trailing after code, 1394 in `///`/`//!`; 95 `#`-comment lines in the non-.rs files, with per-file tallies above; the existing `--` convention confirmed at src/tree/mirror/handshake/tests.rs:286 and justfile:951)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Line comments are the register the doctrine assigns the spaced
double-hyphen to; rustdoc is rendered prose where the em-dash is permitted.
Two `//` comments already use ` -- `, so this is a normalization.

Evidence:

    src/peer/gossip.rs
    879	        // it down a crash here would strand it — held by no one, recorded

    src/message.rs
    377	        // — the same fn every receiver's wire ingress runs for this
    378	        // payload type — reads the just-serialized bytes back at the same

Resolution: A mechanical pass over the lines the census grep lists,
replacing ` — ` with ` -- ` (or a colon where the clause explains); skip
`///` and `//!`. Regenerate the site list with `grep -rnI --include='*.rs' '—'
src tests benches examples | grep -E '^[^:]+:[0-9]+:\s*//([^/!]|$)'` and
`grep -nHI '—' justfile Cargo.toml .cargo/mutants.toml .config/nextest.toml
.github/workflows/*.yml | grep -E ':[0-9]+:\s*#'`. Acceptance: both greps
return nothing.

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

### prose-hygiene-11: Other dialect tells: knob in public rustdoc, load-bearing, story, guardrail, earn, moralized honest
- Where: src/lib.rs:263 (related: README.md:267; src/link/routed/endpoint.rs:30; tests/future_size.rs:1; tests/listen.rs:357, 359, 376, 380, 391; "load-bearing" at 16 sites incl. src/link.rs:27, src/tree.rs:581, src/tree/typed/hash.rs:249; "story" at src/tutorial.rs:299, src/tree/mirror/streaming/backend.rs:273, src/tree/mirror/streaming/backend/local.rs:199; "`-D warnings` honest" at 9 sites incl. src/tree.rs:635, 675; "address seam honest" at src/testing/memnet.rs:10 and tests/routed_link.rs:10)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (per-term greps re-run; "load-bearing" is 16 sites, not the sweep's 13; each listed line read)
- Verification: confirmed with the load-bearing count corrected; history: no-rationale-found
- Owner-gated: no

Smaller register tells, each cheap to reword: "knob" for a setting reaches
the crate-level docs (mirrored into README.md) and the routed link's public
`Config` docs; "load-bearing" means "required"; "story" means "account";
"Guardrail that ..." opens a test module doc ungrammatically; "earn a
checkpoint" is a metaphor for "complete a pass and receive one"; "honest"
is moralized in a copy-pasted lint rationale and two address comments.

Evidence:

    src/lib.rs
    263	//!   an admitted payload is transferable everywhere; the knob's docs

    src/link/routed/endpoint.rs
    30	/// Capacity knobs of an endpoint's router; [`Config::default`] suits

    src/tree.rs
    635	    // `-D warnings` honest on every platform the gate runs.

    tests/future_size.rs
    1	//! Guardrail that the public futures stay type-erased.

Resolution: lib.rs:263 (then `just readme`): "the limit's docs carry the
full contract"; endpoint.rs:30: "Capacity settings of an endpoint's router";
"load-bearing for X" becomes "required for X"; "story" becomes "account" or
is cut; future_size.rs:1: "The public futures stay type-erased."; listen.rs:
"a checkpoint returned by a completed pass"; the lint rationale: "the allow
keeps the gate green under `-D warnings` on every platform"; "keep the
address seam honest" becomes "so nothing here resembles an IP address".
Acceptance: the listed lines reworded; per-term greps return only the
test-local `Knob` type and the honest-participant sense.

### prose-hygiene-12: "Both tests" enumerates the module's contents by count
- Where: tests/async_wire.rs:13
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -c '#\[test\]' tests/async_wire.rs` is 2, at lines 42 and 64)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The module doc says "Both tests share the `Insert`/`Redact` action shape";
a third test would make the sentence false without touching it.

Evidence:

    tests/async_wire.rs
    13	//! Both tests share the `Insert`/`Redact` action shape, so redactions cross

Resolution: "The tests share the `Insert`/`Redact` action shape, ...".
Acceptance: the line no longer states a count.

## Positives

- The V1 retirement's prose pass held almost completely: across the crate,
  the only surviving references to deleted identifiers are the three sites
  in finding 1 and the selection framing in finding 2 (verified by the
  deleted-identifier grep described under Method). No `V1`, `LEGACY_MAGIC`,
  `Alternating`, or `capture_*_v1` reference survives in prose.
- The BLAKE3 to SHA3 swap left zero `blake` hits; "mint" is fully purged;
  there is no TODO, FIXME, or HACK anywhere in scope (all verified).
- No src/ or tests/ prose cites `.agent-notes/`, `design/`, `formal/MODEL.md`,
  or `formal/PROGRESS.md`; the only pointers are AGENTS.md (the sanctioned
  guidepost) and the one justfile line in finding 5 (verified).
- Every Lean citation checked (`wc_impossibility`, `viewEnc`, `LocalEq`,
  `wedge`, `asmResList`, `fan`, `capLevel`, `B5`) resolves under formal/lean,
  and each rides with its invariant restated inline (verified).
- Hand-maintained counts are rare and disciplined: "All eight leaf-query
  paths" (adapter/tests/malformed.rs:190) is pinned by `assert_eq!(checked, 8)`
  at :282 (verified).
- src/reconciliation.rs:237-249 keeps the rejected level-at-a-time wire
  shape as design rationale without naming the retired protocol: a model for
  retaining a rejected alternative with no ghost reference (verified by
  reading).
- "Tripwire" is anchored at src/tree/mirror.rs:9-13 in terms consistent with
  the model of record (per the sweep; not re-read here).

## Open questions for Finch

1. A gitignored build directory sits inside the source tree at
   src/tree/mirror/streaming/target/doctest-nightly/ (verified present and
   ignored via `.gitignore:3:target/`). It is residue of running the nightly
   doctest recipe with the shell's cwd inside that module. Not tree content,
   so not a finding; delete at your discretion, and consider an absolute
   `--target-dir` in the `doctest` recipe.
2. Terms the sweep judged exempt but that sit on the brief's list: "the
   walk" (defined at src/tree/mirror/streaming.rs:4-7), "tripwire" (anchored
   at src/tree/mirror.rs:9-13), "dial" (the `Dial` trait), "silently" (81
   sites, about twenty read by the sweep with the mechanism beside the
   adverb), "genuine(ly)" (90 sites, mostly a meaningful contrast). Do you
   want the pure-intensifier subset of "genuine(ly)" swept regardless? It
   needs a per-site read neither pass did.
3. Rustdoc carries 1394 true em-dashes. The doctrine permits them in
   rendered prose "sparingly"; this pass did not treat volume as a finding,
   but the density is a taste question for a dedicated prose pass.
4. Finding 6's fix rewrites a hard rule's witness. The factual correction is
   plain; whether the renderer-vocabulary re-accept class should survive at
   all, now that the render is CBOR notation whose annotations are the
   `/ comment /` layer, is your call.
5. `Protocol` is a `#[non_exhaustive]` enum with one variant and a
   `#[default]`; finding 2 stands independently of whether you keep it.

## Dropped

- Sweep [3]'s `B5` sites (announced.rs:2, skeleton.rs:18 and :506,
  transcript.rs:11): `B5` is a named Lean axiom (formal/lean/StreamingMirror/
  Mux/Causal.lean:20 and :162), so the citation is in the permitted form.
- Sweep [3]'s "charter locality" (skeleton.rs:19): "charter" is a Lean-anchored
  term (Charters.lean; `c1_charter`, "charter-local" in Mux/Statement.lean),
  not a register transplant.
- No other finding was dropped; all twelve survive, with findings 1 and 2
  extended by one site each, finding 4 narrowed, and finding 7's line
  numbers corrected.
