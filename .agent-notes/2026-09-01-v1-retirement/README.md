# Retiring Protocol::V1 (the alternating mirror): plan

Status: in progress. Base surveyed at 4e64a4fb. Rulings (2026-09-01):
decision 1 — drop the legacy-magic recognition; decision 2 — remove the
`.protocol()` builders; decision 3 — the scoped mutants A/B is deferred
(the seed-corpus replay check still applies). Premise confirmed by Finch:
the crate is pre-release and nothing uses V1.

## Goal

Remove the V1 (alternating) protocol from the crate entirely: the
`Protocol::V1` variant, the `protocol-v1` cargo feature, the
`tree::mirror::alternating` tower (~4,500 lines plus its wire snapshots),
the V1-only codec (`tree::wire`, 267 lines, and the typed tree's gated
ingress entries), the V1 branches in the shared handshake/party/framing
layers, and every V1 driver, test, and snapshot — then simplify everything
whose shape existed only to accommodate two dialects.

Why now: persistent storage is coming, and V1 cannot in principle support
it; V2 is trusted on its own evidence; V1's remaining role is as the
streaming protocol's differential oracle, and that role can be handed to a
simpler oracle without relaxing verification (next section).

The crate is pre-release: no shipped version holds wire compatibility, so
no fleet speaks V1 and no compatibility obligation survives. That premise
underlies decisions 1 and 2 below; if it is wrong, the plan changes.

## The verification lattice: what V1 anchors, and what replaces it

V1 is load-bearing as an oracle in exactly three committed suites. The
governing rule is that retiring an instrument follows the same discipline
as landing one: each replacement lands and demonstrates coverage *before*
the alternating oracle is deleted.

1. **`streaming/tests.rs::streaming_matches_alternating_oracle`, and three
   expected-value uses in `streaming/tests/capacity.rs`.** Replacement:
   **`Tree::join` as the oracle** (driven at the `Root` level, the way
   `traverse/join/tests.rs::mirror_merge` drives the mirror today, but
   synchronous and wire-free). The crate's own documented invariant is
   that `join` and `mirror` are observationally identical — both delegate
   deletion honoring to the same filter — so the property becomes: both
   streaming sides equal `join(a, b)` (and hence each other), in both
   orientations. Strictly simpler oracle: no async, no wire, no protocol
   state machine; independently anchored (see 3).

2. **`traverse/join/tests.rs::join_matches_mirror`** (join is checked
   *against* the alternating mirror). This edge does not disappear — it
   reverses direction: the new streaming-vs-join differential is the same
   edge of the lattice, now checking the wire implementation against the
   in-memory one instead of vice versa. `join_matches_mirror` is deleted.

3. **What anchors `join` once the mirror no longer does:** its existing
   independent suites — the route-equivalence property in
   `tree/tests.rs` (direct construction ≡ act-detour ≡ join of disjoint
   halves), idempotence/commutativity/associativity, and the redaction
   suites. The lattice stays connected: `act` ↔ `join` (route
   equivalence), `join` ↔ streaming (new differential), streaming ↔ the
   byte-pinned wire snapshots. Every edge V1 carried is still present.

Two smaller instruments also lean on V1 machinery and get retargeted, not
deleted:

4. **`tests/decode_alloc.rs`'s framing-layer meter** prices "memory tracks
   receipt, never declared length" through `testing::read_framed_payload`,
   which wraps V1's `FrameRead`. The property belongs to `read_payload`/
   `resume_payload`, which survive (they are the V2 party hand-off's and
   the streaming codec's payload policy). Retarget the `testing` shim to
   drive `read_payload` with a caller-declared length — the exact function
   the V2 hand-off ingress trusts — and keep both meter legs and their
   ceilings unchanged. The streaming supply-frame leg is untouched.

5. **`tree/tests.rs` route equivalence** currently compares routes by
   serializing each root through `tree::wire::to_vec` (the V1 codec).
   Replace byte-equality-of-encodings with structural equality on `Root`
   (which the join suites already use); the hash-equality assertions
   stay. Equal trees imply equal encodings under any codec, so nothing
   weakens.

6. **`traverse::unknown`** loses its only production consumer (the
   alternating local backend) but remains the committed differential
   oracle for streaming's `materialized::Unknown`. It stays, re-gated to
   `cfg(test)` if nothing else needs it, with its module doc restated
   positively as the materialized pruner's in-memory oracle.

Deleted without replacement, deliberately: every test whose *subject* is
V1 itself — the alternating unit/protocol suites and wire snapshots, the
V1 cases in `tests/{gossip,bootstrap,retire}_snapshot.rs` (and their
`.snap` files), `tests/handshake_liveness.rs` V1 instantiations, the
`tests/session_stats.rs` V1-zeros test, `tests/bootstrap.rs` explicit-V1
selection, `tests/handshake.rs` V1 arms, and `peer/gossip/tests.rs`'s V1
twins. These verified the thing being removed, not the thing that remains.
One exception to rework rather than delete: `peer/bootstrap/tests.rs` uses
`.protocol(V1)` to prove builder-config *propagation*; retarget those
assertions to a surviving non-default knob (e.g. window or budget) so
propagation coverage survives.

## Decision queue

1. **Legacy-magic recognition: drop it (recommended) or keep it.** A V2
   endpoint today recognizes the legacy `b"RUMORS"` preamble and diagnoses
   `VersionMismatch { remote_version: 1 }` instead of `MagicMismatch`
   (`handshake.rs`: `decode_v2`'s sniff, plus the early-close diagnosis in
   `Staged::fill`). With V1 fully retired and never shipped, this code
   diagnoses a peer that cannot exist; keeping it is a ghost reference in
   executable form. Dropping it deletes `LEGACY_MAGIC`, the legacy layout
   constants, and the `fill` special case; a hypothetical ancient peer
   then reports as `MagicMismatch`, whose hex `remote_magic` field still
   shows `52554d4f5253` ("RUMORS") to a human debugging. This is an
   outward-facing behavior change (error variant selection), hence your
   call. The `VersionMismatch` machinery itself stays regardless — it is
   the live V2-vs-future-version diagnostic.

2. **The `.protocol()` builders: remove them (recommended) or keep them.**
   `Peer`'s and `Bootstrap`'s `protocol()` become one-position knobs — a
   setter whose only accepted value is the default. The `Protocol` enum
   itself stays public and `#[non_exhaustive]` (it is wire vocabulary:
   `SessionInfo.protocol`, `VersionMismatch.local_protocol`, and the
   versioning story persistent storage will lean on), but a knob with one
   position is decoration. Pre-release, removing it is cheap and it
   returns naturally with a V3. Public API removal, hence your call.
   Ripple either way: three sibling knobs' docs cite `protocol()` as the
   canonical description of config propagation; if it goes, that
   description moves to one of the survivors.

3. **Adequacy check for the oracle swap: scoped mutants A/B
   (recommended) or skip.** After commit 1 (both oracles coexisting) and
   again after commit 2, run `cargo mutants` scoped to
   `src/tree/mirror/streaming` and `src/tree/traverse/join.rs`, and
   compare survivor rosters: the join oracle must kill what the
   alternating oracle killed. Costs a few CPU-hours; results reported in
   this note's follow-up, not committed as a baseline. The cheap
   always-do check regardless: the committed seed corpora for
   `streaming/tests.txt` and `capacity.txt` are per-file and keep
   replaying through the renamed differential, so the shrunk regressions
   of record continue to run.

## Commit sequence

Worktree off main at the briefed SHA; `just gate` fully clean before each
commit; commits ordered so every intermediate tree is gate-clean.

**Commit 1 — oracle handoff (adds only).** Land the join-based streaming
differential *beside* the alternating one (both run); retarget the
`testing` shim and `tests/decode_alloc.rs` framing leg to `read_payload`;
swap `tree/tests.rs` route equivalence to structural equality. Run the
decision-3 mutants baseline here if approved.

**Commit 2 — the removal (atomic).** Everything whose absence breaks
compilation or the gate must move together:

- `src/protocol.rs`: delete `V1`.
- `Cargo.toml`: delete the `protocol-v1` feature. `justfile`: delete the
  `cargo check -p rumors --features protocol-v1` leg; update the
  inner-loop and test-all comments that explain the V1 towers.
- `src/tree/mirror/alternating{.rs,/}`: delete, including
  `wire_snapshot.rs` and the `snapshots/` directory, and
  `proptest-regressions/tree/mirror/alternating/tests.txt` (an orphaned
  seed fails `tests/seed_liveness.rs`; history preserves it).
- `src/tree/wire.rs`: delete. `src/tree/typed/{node,prefix,untyped}.rs`:
  delete the gated codec/ingress items and their malformed-input tests;
  `typed/tests.rs` wire round-trips go with them.
- `src/message.rs`: delete `from_reader` and its `message/tests.rs`
  round-trip tests (its only consumers are the deleted codec).
- `src/tree/mirror/framing.rs`: delete `FrameRead`, `FrameWrite`,
  `length_header`, `LENGTH_HEADER_LEN`; keep `read_payload`,
  `resume_payload`, `PAYLOAD_CHUNK_LEN`, `LengthOverflow` (the streaming
  codec's `SupplyTooLarge` wraps it); rewrite the module doc.
- `src/tree/mirror/handshake.rs`: delete the legacy dialect encode/decode
  and (per decision 1) the legacy recognition; `preamble_len` and
  `PREAMBLE_MAX` dissolve into `V2_PREAMBLE_LEN`; `Staged` loses its
  dual-width machinery. Tests: drop V1 loop arms; `legacy_peer_is_a_
  version_mismatch` per decision 1.
- `src/tree/mirror/party.rs`: drop the V1 branches, `decode_party_v1`,
  and the now-unused `protocol` parameter on `send`/`receive`; call sites
  simplify.
- `src/peer/gossip.rs`: delete `PROTOCOL_MAGIC`, `bootstrap_v1`,
  `Reconciliation::v1`, `alternating_error`; the protocol matches
  collapse to direct calls; the epilogue conditionals become
  unconditional (session `Ok` now uniformly certifies both sides —
  simplify `link.rs`'s "what a session promises" and `rumors.rs`'s gossip
  docs accordingly, here, since they carry doc links).
- `src/peer.rs` / `src/lib.rs`: drop the `PROTOCOL_MAGIC` re-export; per
  decision 2, the `protocol()` builders; the crate docs' `protocol-v1`
  feature bullet. Run `just readme` (READMEs are derived).
- `src/observe.rs`: drop the dialect guard in `Attachment::begin` —
  observability is total — and the V1 exclusion prose.
- `src/tree.rs` / `src/tree/mirror.rs` / `src/tree/typed.rs`: drop the
  gated `mod wire`/`mod alternating` declarations; rewrite the module
  docs (the mirror doc's compile-time rationale for the feature gate
  dissolves).
- `src/tree/traverse/join/tests.rs`: delete `join_matches_mirror`;
  delete the alternating differential from `streaming/tests.rs` and its
  `capacity.rs` uses (the join oracle from commit 1 remains);
  `traverse::unknown` re-gated per lattice item 6.
- Integration suites: delete the V1 cases and `cfg(feature =
  "protocol-v1")` gates listed above; delete
  `tests/snapshots/*__v1_*.snap`; in `tests/common/gossip_snapshot.rs`
  delete `capture_session_v1`/`render_v1` (the shared `Log` stays — the
  drain assertion uses it) and rewrite the module doc.
- `benches/gossip_fixed.rs`: `PROTOCOLS` collapses to `[V2]`; doc sweep.
- Error/doc surfaces whose links break: `error.rs` ("Reachable only for
  V2" clauses become vacuous and dissolve; `MagicMismatch` doc loses the
  legacy clause), `reconciliation.rs`'s "Two protocols" section (rewrite
  as the design rationale it is: the level-synchronous shape described as
  the naive alternative, not as a shipped selectable dialect),
  `peer.rs`/`peer/bootstrap.rs` knob docs' V1-ignores clauses.

Snapshot witness for the commit message: the V2 `.snap` files are
byte-identical to the parent — the diff under `tests/snapshots/` and the
bookmark pins is deletions of `*__v1_*` files only. This is a deletion of
V1 cases, not a re-accept; no wire byte moves.

**Commit 3 — prose re-denomination (fresh-eyes pass).** Sweep every
surviving mention so each document reads as written today against today's
code: `AGENTS.md`'s mirror-protocols bullet, `observe.rs`, `link.rs`,
`streaming/erased.rs`'s behavioral-pins list,
`streaming/remote/proxy/tests/transport.rs`'s doc comment,
`formal/doc/exposition.typ` ("ships that protocol too … as the behavioral
oracle" — restate the request/response-per-level shape as the design
alternative without claiming the crate ships it),
`design/rumors-frame-fuzz.md` (its V1-exclusion sections re-anchor to "the
crate has one protocol"), `results/mirror-complexity.tex` if it names the
dialects. Then the mechanical sweep: `grep -ri 'alternating\|protocol-v1\|
\bv1\b'` across `src tests benches formal design justfile Cargo.toml`,
disposing of every hit.

## What is genuinely lost

Honest accounting: V1 was a second, independent, hand-verifiably-simple
implementation of the same wire reconciliation — N-version redundancy on
the deletion-honoring filter and the descent semantics. The join oracle
preserves the semantic edge but shares `traverse` machinery with `act`,
so the implementations under differential test are less disjoint than
before. Compensations already committed: the streaming wire is pinned
byte-for-byte by snapshots, every ingress has a malformed-input suite,
and the capacity/violation/liveness suites exercise the wire mechanics
V1 never shared anyway. And V1's oracle value was already decaying: it
cannot express bookmarks, budgets, stats, epilogues — or persistence.

Also lost, cheerfully: the entire `#[cfg(any(test, feature =
"protocol-v1"))]` lattice (the crate's own tests always built the
alternating monomorphization tower — every `just test` gets faster), the
dual-dialect handshake, and the default-vs-all-features gap narrows to
`conformance`/`test-internals`/`meter`.

## Sweep false positives — leave these alone

English "alternating" and user-payload "V1" that name no protocol:
`tests/listen.rs` (op-index alternation), `tests/common/overlap.rs`
(alternating polls), `tests/common/shape.rs` (`v1` loop variable),
`tests/cbor_evolution.rs` (`WideV1`/`WideV2`, a user-type evolution
example), `src/lib.rs`'s payload-versioning advice ("an outer enum … one
variant, `V1`"), and the formal artifacts' "alternating mixtures" /
parity-alternation phrasing (`EventDag.lean`, `production.qnt`,
`PROGRESS.md`, `narrative.typ`'s message-index alternation).

`.agent-notes/` history is exempt by charter and is not swept.
`.cargo/mutants.toml` carries no alternating/V1 entries (verified);
confirm again after commit 2 that no exclusion pattern has gone
pattern-dead.
