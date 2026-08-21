# Cluster: stats shutdown, alternating remote EOF arm, alternating partition

Four survivors from the scope-A run. Code read at branch tip 62447263
(`/Users/oxide/src/rumors-mutants`); none of these four files changed
between the run base 02560b1f and the tip except `stats.rs` (whose change
does not touch `CountedWrite`; the mutant re-locates to tip lines
278–280). Killer attributions below are verified against the run's
per-mutant logs on ox-east-1
(`~/build/rumors-mutants/scopeA/mutants.out/log/`), not inferred.

## `src/tree/mirror/streaming/stats.rs:279:9`: replace `CountedWrite::poll_shutdown` with `Poll::from(Ok(()))`

**What the mutation does.** `CountedWrite` is the byte-counting write
proxy between the frame codec and the transport. Its `poll_shutdown`
delegates to the inner half; the mutant reports shutdown complete without
ever shutting the inner half down.

**Reachability: dormant, verified.** No code in the crate calls
`shutdown`/`poll_shutdown` on any mirror-side write half — a crate-wide
grep finds only the two delegation definitions themselves (this one and
`link/erased.rs`), no invocations. The link doctrine is drop-is-shutdown
(`link/routed.rs`: "dropping it is the shutdown"), and `CountedWrite` is
not public API (`mod tree` is private at the crate root). The method
exists because `AsyncWrite` requires it, so the mutable codepoint cannot
be refactored out of structural existence.

**Why the suites miss it.** The behavioral suites tear sessions down by
dropping halves, so `poll_shutdown` is never polled in any test: the
mutant is invisible end-to-end, not merely under-asserted.

**Severity.** Dormant, not bug-shaped today: no current caller means no
current behavioral divergence. But the failure it would cause — a
transport whose half-close is silently skipped — is the quiet kind, so
leaving the method permanently unobserved is a test blind spot of the
"not wrong, but you couldn't tell if it were" class.

**Disposition: killing test (ladder step 4).** Not roster material: the
mutation is not equivalent (any future caller observes the difference),
and there is no impossibility to assert. The most general form is a
*proxy-transparency family* over both counting wrappers, stated once:

> `CountedWrite` and `CountedRead` are transparent proxies whose only
> side effect is counting: for any scripted inner half and any operation
> sequence, every trait method forwards its call and returns the inner
> result verbatim, and the counters advance by exactly the accepted
> (resp. delivered) bytes.

Generator: a scripted mock `AsyncWrite`/`AsyncRead` whose per-call
results are an arbitrary `Vec` of `Poll` outcomes (`Pending`,
`Ready(Ok(n))`, `Ready(Err)`), plus an arbitrary operation sequence
(write/flush/shutdown with arbitrary buffers). Oracle: the wrapper's
observable call log and returned polls equal the script's, and the
counter deltas equal the sum of accepted bytes. This kills the
`poll_shutdown` replacement (the mock's shutdown log stays empty), and
extends the existing `counted_read_counts_any_chunking` family — which
covers counting only, on the read side only — to the full trait surface
of both wrappers. Home: `src/tree/mirror/streaming/stats/tests.rs`.

## `src/tree/mirror/alternating/backend/remote.rs:188:13`: delete match arm `UnexpectedEof` in `recv_msg_with`

**What the mutation does.** The arm rewraps a frame-read EOF as an
`UnexpectedEof` carrying "peer closed before sending expected message".
Deleting it lets the raw EOF through: same `ErrorKind`, same `Error::Io`
variant, original message text. Only the diagnosis is lost.

**Reachability: yes, on-model.** An honest peer that crashes or closes
the stream before a payload-bearing message lands here; environmental
failure, not programmer error.

**Why the suites miss it, and the duplication tell.** The arm is a
verbatim duplicate of `recv_msg`'s (line 163), whose deletion the run
*caught* — `close_before_a_message_is_a_typed_eof`
(`remote/tests.rs:79`) asserts the diagnosis text, but only through the
`recv_msg` entry. The payload-codec twin has no analogous drive. One
codepoint killed, its copy missed: the classic duplication survivor.

**Severity.** Diagnostics-only: nothing matches on the message text; a
desync would still surface as a typed EOF, just less legibly.

**Disposition: refactor (ladder step 1).** Extract the EOF-diagnosis
rewrap into one shared helper (e.g. `fn diagnose_boundary_close(e:
io::Error) -> io::Error`) used by both `recv_msg` and `recv_msg_with`;
the surviving duplicate dissolves into the codepoint the existing test
already kills. Worth doing alongside: generalize
`close_before_a_message_is_a_typed_eof` into a small family over both
ingress entries × close positions (empty stream; every strict prefix of
the length header; mid-body) so the diagnosis contract holds through
both entries whatever the code shape — the header-prefix and
over-declared cases already exist as separate point tests in the same
file and would fold in naturally.

## `src/tree/mirror/alternating/backend/local/partition.rs:408:61`: replace `<` with `>` in `partition_leaf_uncertain`

**What the mutation does.** Line 408 is the carry-over scan: while
draining the counterparty's uncertain leaf-listings (grouped by
leaf-parent prefix, ascending) against our own ascending frontier of
leaf-parents, frontier entries strictly *below* the current group's
parent are ours alone — the counterparty never mentioned them — and are
kept untouched. With `>`, that drain never fires for a below-neighbor;
the group's own frontier match (line 413) then peeks the wrong entry and
falls into the else arm, whose `debug_assert!(false)` panics under the
campaign's dev profile. So every discriminating input kills the mutant
*loudly*; survival means no test ever presents the discriminating shape.

**Why the suites miss it: verified by the killer logs.** The `<=` and
`==` legs at the same site, and the peek-match `==→!=` at 413, all died
to exactly one test: `converges_on_leaf_parent_dispute`
(`alternating/tests.rs:141`), which drives the protocol end-to-end over
`tree::arb::leaf_parent_dispute_pair()` — a forged-geometry constructor
building two trees that share one `S<Z>` (31-byte) prefix with different
leaf sets. That pair puts exactly one leaf-parent in the frontier, so
the carry-over scan never executes and `<` vs `>` is indistinguishable.

**Reachability in principle.** The discriminating class is: at
leaf-parent height, an only-ours leaf-parent sorting below some disputed
group's parent. Through the public API this needs two leaves sharing a
31-byte path prefix *plus* another deep-disputed path — 248-bit blake3
prefix collisions. This is squarely the "bottom-level protocol walks"
class the collision-schedule note
(`.agent-notes/2026-08-21-collision-schedule-test-mode/`) names as
geometry-shadow; forged-geometry constructors are the sanctioned
targeted instrument, the collision-schedule mode the broad one.

**Disposition: generalize the forged-geometry family (ladder step 4).**
Promote the two deterministic pairs (`leaf_parent_dispute_pair`,
`leaf_parent_redaction_pair`) into a proptest *leaf-parent scenario
family* in `tree::arb`:

> For any disputed leaf-parent whose two leaf sets stand in an arbitrary
> relation (disjoint / overlapping / subset / superset), flanked by
> arbitrary neighbor leaf-parents (only-ours, only-theirs,
> shared-and-equal) on either side of the disputed prefix in sort
> order, and with the two replicas at distinct versions: both driver
> arrangements converge, and both sides land on the union tree.

The oracle already exists (the union tree, as in the deterministic
pins, which stay as witnesses). The below-neighbor draw is what kills
this mutant — the mutated run panics in the else arm. The family also
hardens the whole `partition_leaf_uncertain`/`close` bottom edge rather
than one operator. Longer term, the collision-schedule mode runs the
public-API algebraic suites (`idempotent`, `commutative`, `absorptive`,
`associative`) under collision-rich geometry and covers this site with
no per-site foresight; per that note, keep both instruments.

## `src/tree/mirror/alternating/backend/local/partition.rs:545:56`: replace `|` with `&` in `close`

**What the mutation does.** `close` is the leaf-height closing round;
its `Step::Done` exit assembles the reconciled `tree::Root` with
`ceiling: our_version | their_version`. `Version` implements both ops in
`before` (join and meet), so the mutant compiles to the causal *meet*:
the reconciled ceiling under-joins whenever the two replicas' versions
differ.

**Severity: bug-shaped, the worst of the four.** The ceiling is the
deletion-honoring boundary — `Root`'s own doc says it is "exactly what
deletion honoring compares against", and redaction has no tombstones,
only version ceilings. An under-joined ceiling on a reconciled tree can
let a later session fail to honor a redaction. `Root::PartialEq`
compares the ceiling, so any test reaching the mutated exit with
differing versions catches it.

**Why the suites miss it: verified.** The branch-height twin —
`reply`'s identical join at 652:56 — died to the algebraic proptests
(`absorptive`, `associative`, `equivalent_to_cross_react`), so ordinary
sessions exercise the join through `reply`'s exits. What no test
reaches is `close`'s *Done* exit with differing versions: it requires a
session still disputed at leaf height whose closing response requests
nothing, i.e. one side's leaf sets a superset under every disputed
parent. Both existing leaf-parent pins are symmetric-difference-shaped
(each side requests the other's leaf), so they exit through `Continue`
and never evaluate this expression.

**Reachability in principle.** Same 248-bit-collision shadow as the
mutant above: leaf-height dispute is unreachable under blake3-derived
paths through the public API.

**Disposition: the same leaf-parent scenario family.** The
subset/superset draw drives `close`'s Done exit; distinct versions make
join and meet differ; the union oracle's ceiling then convicts the
meet. Two sharpenings worth writing into the family: (a) assert the
reconciled ceiling equals the join on *every* scenario, not only via
whole-`Root` equality, so the claim is stated where it lives; (b) the
superset-plus-redaction draw (extending what
`leaf_parent_redaction_pair` pins) doubles as the end-to-end witness
that an under-joined ceiling breaks deletion honoring, tying the site
to the invariant it serves.

## Cross-cutting

- The two partition survivors dissolve under one new generator family in
  `tree::arb` plus one proptest in `alternating/tests.rs`; no roster
  entries, no asserts. Both are also exactly the mutants the
  collision-schedule mode's mutation-campaign integration bullet is
  about ("the campaign's test command must include the collision leg").
- The remote.rs survivor is an adoptable-now refactor (shared helper),
  behavior-preserving.
- The stats survivor wants a small scripted-mock transparency family;
  cheap, unit-level, and it closes the whole wrapper surface rather than
  the one method.
