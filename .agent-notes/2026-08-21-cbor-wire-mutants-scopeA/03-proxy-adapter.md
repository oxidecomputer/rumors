# Proxy/adapter cluster: 8 survivors

Modules touched, and their roles in the streaming (V2) stack:

- `remote/streams.rs`: binds logical streams to transport streams. Every
  incoming data stream opens with a two-item label (session epoch, stream
  index), read by `label_item`; `AcceptDriver` validates labels and routes
  streams to their claimants.
- `remote/adapter/{encode,decode}.rs`: the leaf conversion boundary between
  the erased protocol vocabulary (`Reply`/`Reaction`) and wire frames.
  `encode.rs` renders one reply as frames and derives the questions it
  acknowledges; `decode.rs` reconstructs replies, streaming supplied leaves
  through the backend; `read_early` reads the opening-supply reply
  (supplies only, exactly one reply).
- `remote/proxy/work/encode.rs`: the per-stage outbound encoder tasks;
  `terminal` is the leaf-height encoder (initiator side passes
  `questions: None`, responder side `Some`).
- `remote/proxy/work/pump.rs`: the per-stage decode loops, plus `Early` —
  the cursor pairing the responder's root-level requests with the
  initiator's opening-supply groups in ascending radix order.

All eight survivors are in conformance-violation detection or
trust-boundary machinery, or in the terminal stage that committed suites
only ever run empty. None is a bug in live behavior; every one is a
"you couldn't tell if it were wrong" gap (the repairable kind). Verified
by reading the sites and the sibling test files named below; the
"why missed" claims about e2e suites are inferred from the honest-peer
session model plus the geometry argument, not from re-running anything.

## streams.rs:762 `label_item`: guard `head.major == cbor::MAJOR_UINT` → `true`

**Site** (verified): `label_item` classifies one label item:
canonical uint head → its value; well-formed non-uint head →
`AcceptError::Label("label item is not an unsigned int")`; malformed head
→ `Label("label head is not canonical")`; EOF → supply failure. The
mutant accepts any well-formed head as a label, reading e.g. a text or
tag head's value field as an epoch or stream index.

**Reachable?** Only from a nonconformant peer: honest sessions label with
uints always. On-model this is a conformance bug detector, and its
survival is a diagnostics gap: a mislabeling peer would be mis-diagnosed
(likely as `Epoch`/`UnknownStream`, from a garbage value) instead of
`Label`.

**Why missed**: `streams/tests.rs` exercises epoch mismatch, unknown
index, duplicate, unclaimed delivery, and malformed *frames*, but every
label it writes is a conformant uint; no test presents a well-formed
non-uint head as a label item.

**Disposition — killing family (rung 4)**: an exhaustive initial-byte
matrix over `label_item`, the same shape as
`ingress_paths_agree_on_every_initial_byte` in `cbor/tests.rs`: for each
of the 256 initial bytes (argument bytes padded so no path truncates),
drive `label_item` and compare against the class `cbor::read_head`
assigns the same bytes — `Ok(uint head)` must yield the head's value,
`Ok(non-uint head)` must yield the not-an-unsigned-int `Label` rejection,
`Err(malformed)` must yield the not-canonical `Label` rejection. This is
total (all 256 classes), differential (the slice grammar is the oracle),
and kills the guard mutant along with any future drift between the label
classifier and the head grammar. Add one EOF witness (empty stream →
supply failure) beside it.

## adapter/decode.rs:188 `read_early`: guard `!any` → `true`

**Site** (verified): the opening-supply grammar accepts either a bare
`End(Reply)` (empty reply) or supply reactions whose last frame carries
`Flow::End`. The `!any` guard makes a bare `End(Reply)` *after* a supply
reaction the violation `BareEndAfterReaction`; the mutant accepts the
sequence `[Reaction(Supply, Continue), End(Reply)]` as a clean end.

**Reachable?** Nonconformant peer or in-process frame construction only
(the frame stream arrives through the codec, which honest peers drive
conformantly). Diagnostics gap.

**Why missed**: `adapter/tests/opening.rs` hand-builds frame vectors and
covers the empty reply, the second-reply rejection
(`ExtraOpeningReply`), positional reactions, and ledger overdraw — but no
vector puts a bare reply-end behind a supply.

**Disposition — killing family (rung 4)**: generalize those point
vectors into a grammar-recognizer differential family over `read_early`:
generate small arbitrary frame sequences (supplies with either flow,
bare ends, stream ends, positional reactions), define the accepted
language once as a ~10-line reference recognizer in the test ("zero or
more `Supply(_, Continue)` then one `Supply(_, End)`, or exactly one
bare `End(Reply)`; nothing after the reply"), and assert `read_early`
accepts exactly the language and rejects everything else with the typed
defect the recognizer predicts. The existing point tests dissolve into
witnesses of this family. Kills the `!any` guard (the family contains
the supply-then-bare-end word) and pins every other arm of the loop.

## adapter/encode.rs:250 `validate_leaf` → `()`

**Site** (verified): `validate_leaf` asserts, per leaf a backend
enumerates while encoding a supply, that the leaf's path sits under the
requested prefix and that enumeration order is strictly ascending. It is
pure assert machinery at the `Backend` trust boundary — `Backend` is a
public trait (`pub use backend::Backend`), so implementations are
caller-supplied and the harness cannot reach a misbehaving one through
the committed suites' `Local` backend.

**Reachable?** The asserts fire only on a buggy `Backend`
implementation. That is exactly the complement the assert doctrine
sanctions (a trust boundary, O(prefix) spot check), so the assert stays;
the gap is adequacy — nothing demonstrates the guard fires.

**Disposition — adequacy family (rung 4)**: a scripted misbehaving
backend double (wrap `Local`, permute or displace its `leaves()`
enumeration) and a proptest family: any enumeration violating either
clause — a leaf outside the requested prefix, or a non-ascending
adjacent pair — panics (catch_unwind or `#[should_panic]` per shrunk
witness), and every conforming enumeration encodes cleanly. State the
invariant as the enumeration contract, generate violations by choosing
an index to corrupt and a corruption kind, so the family covers both
assert clauses rather than one point each. This is the committed
demonstration that a known-bad mechanism fails (the meters' adequacy
rule applied to an assert).

## pump.rs:475 `Early::advance_to`: `>` → `>=` — proven equivalent; refactor it away

**Site** (verified): the cursor takes `lookahead`, returns the node on
`next == radix` (line 472), stashes-and-prunes on `next > radix`
(line 475), and fails `UnaskedReply` otherwise. Control reaches line 475
only with `next != radix`, so `>` and `>=` have an empty discriminating
class: the mutant is value-equivalent for all inputs.

**Disposition — refactor (rung 1)**: replace the if-ladder with
`match next.cmp(&radix) { Equal => .., Greater => .., Less => .. }`. The
comparison operator then does not structurally exist, and the tool's
match-arm deletions are unviable (the `Ordering` match becomes
non-exhaustive). Clearer at the site, no roster entry needed. Do not
roster-exclude: the ladder prefers dissolution when it is this cheap.

## pump.rs:512/516 `Early::finish` (body → `Ok(())`; delete `!` on `!self.exhausted`)

**Site** (verified): after the question loop, `finish` enforces "every
opening supply answered a root-level request": a stashed lookahead group
is `UnaskedReply` (line 512), and an armed, not-yet-exhausted supplies
stream that yields one more item is `UnaskedReply` (lines 515–519). The
body mutant drops both checks; the deleted `!` inverts the second so the
un-pulled-trailing-group case is only polled when the stream is already
exhausted, i.e. never detected.

**Reachable?** The discriminating input is an initiator that ships an
opening-supply group at a radix the responder never requests, positioned
after the last requested radix so it is never pulled into lookahead
(the deleted-`!` case) or stashed there (the body case's second
witness). Honest peers compute the early set from the same two listings
the responder recomputes it from, so this is conformance detection —
but the un-pulled case also silently skips driving `read_early` to its
trailing validation, so the mutant widens what a nonconformant peer can
leave undiagnosed.

**Why missed**: `proxy/tests/malformed.rs` crafts full-proxy violations
(`duplicated_reply_is_rejected_as_unasked` covers `UnaskedReply` at the
ordinary reply level), and `adapter/tests/opening.rs` covers the
opening-supply *decode* — but nothing exercises the pairing discipline
between requested radices and supplied groups that `Early` owns.

**Disposition — refactor + killing family (rung 1 then 4)**: `Early` is
private to `pump.rs`; move it to its own sibling module with tests, then
state the pairing property directly: for all pairs of ascending radix
sets (R = requested, S = supplied), driving `advance_to` over R against
a scripted supply stream of S and then `finish` yields — every r ∈ R
answered with its node iff r ∈ S, `None` iff r ∉ S, and the run ends in
`UnaskedReply` iff S ⊄ R (at the first offending group when it is
reached, at `finish` when it trails). Generator: arbitrary subsets of a
small radix universe; oracle: the subset check. This kills 512, 516, and
the body mutant in one family, and would have killed the equivalent
`advance_to` legs too had they not dissolved. Add one full-proxy wire
witness in `proxy/tests/malformed.rs` (an opening-supply group at an
unrequested radix → session fails `UnaskedReply`) so the module property
stays bound to the wire path.

## encode.rs:54/66 `terminal` (body → `Ok(())`; delete `!` on `!batch.is_empty()`) — the geometry shadow

**Site** (verified): `terminal` pairs each leaf scope with the local
walk's leaf reply, writes the reply's frames, and either publishes the
acknowledged questions (responder side, `questions: Some`) or requires
the batch empty (initiator side, `None` — a non-empty batch is
`Error::TerminalQuery`). After the loop, `finish` rejects unclaimed
local replies and closes the outgoing stream.

**Why these survived — and what it tells us**: the deleted `!` makes the
initiator-side encoder fail with `TerminalQuery` on every *empty* batch,
i.e. on every normal leaf reply. One executed loop iteration in any
committed suite would kill it; its survival is therefore evidence
(stronger than inference from any single test) that **no committed
session ever delivers a scope to the terminal stage**. That matches the
geometry argument: a leaf-height question needs a dispute at height 1 —
two peers listing nodes at the same 31-byte prefix with different
hashes — which requires leaves sharing a 31-byte blake3 path prefix, a
248-bit prefix collision. This is precisely the "bottom-level protocol
walks" entry in the collision-schedule test mode design
(.agent-notes/2026-08-21-collision-schedule-test-mode/README.md): the
stage loops run every session, but empty. With the loop never entered,
the body mutant is also invisible — its skipped `finish` misses only
checks that are vacuous on an empty stage, and the outgoing stream it
fails to close was never claimed, so the remote's `reject_extra` is
satisfied vacuously.

**Reachable in principle?** Yes — both legs guard live behavior once
leaf-height work exists. The `TerminalQuery` leg itself defends against
a local-walk conformance bug (the batch is derived from local replies,
not wire input), so it is in-process detection; the body deletion drops
real protocol work (leaf replies never cross), which under collision
geometry is a session-breaking bug, not a diagnostics gap.

**Disposition — killing family now, mode later (rung 4, twice)**:

1. A direct contract family over `encode::terminal` with scripted
   scopes/replies (the `proxy/work/tests.rs` harness already builds
   parked sessions; a lighter scripted-channel harness suffices here):
   for arbitrary small scripts of (scopes, matching leaf replies, an
   optional trailing unclaimed reply, questions arm chosen by the
   generator) — with `Some`, every derived question publishes, in wire
   order; with `None`, a reply deriving a question fails `TerminalQuery`
   and a question-free script completes; a trailing unclaimed local
   reply fails `UnaskedLocalReply`; a missing reply fails
   `UnansweredRemoteQuery`. The body mutant fails the unclaimed-reply
   and question-publication legs; the `!` mutant fails the plain
   empty-batch completion leg.
2. The collision-schedule mode is the general instrument that makes the
   terminal stage carry real work through the public API suite-wide;
   the campaign's test command must include that leg once it exists
   (the mode's design note already records this requirement). The unit
   family above is the fast deterministic pin that does not wait for it.

## Summary

| Rung | Mutants | Disposition |
|---|---|---|
| 1 (refactor) | pump.rs:475 `>`→`>=` | `match next.cmp(&radix)`; operator dissolves, arm deletions unviable |
| 4 (family) | streams.rs:762 | exhaustive label-byte matrix, differential against `cbor::read_head` |
| 4 (family) | decode.rs:188 | opening-supply grammar recognizer, differential over generated frame sequences |
| 4 (family) | encode.rs:250 | misbehaving-backend adequacy family; the assert itself stays (trust boundary) |
| 4 (family) | pump.rs:512, 516 | `Early` pairing property (S ⊆ R oracle) after module split; one full-proxy wire witness |
| 4 (family) | encode.rs:54, 66 | terminal-encoder contract family with scripted channels; geometry shadow — collision-schedule mode is the public-API instrument |

No roster candidates: nothing here is value-equivalent except the one
refactorable comparison, and nothing is work-only. Headline: this
cluster is uniformly "conformance detectors nobody misdrives" plus the
first hard confirmation (the `!batch.is_empty()` survivor) that the
terminal stage runs empty in every committed suite — worth citing in
the collision-schedule design's provenance when it lands.
