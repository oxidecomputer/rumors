<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# P2 lane: a session never parks on a peer that has died

## Goal

A session whose peer dies after a complete handshake, before opening a data
stream it owes, parks forever: the accept driver waits on a stream nobody will
open, the protocol waits on the claim it would fill, and the control stream,
already at end-of-stream, is polled by nothing. The invariant restored: a
session awaiting a data stream also observes the control stream, and a control
EOF or failure ends it with an error naming the peer's departure. What stays:
the caller owns the timeout for a live, silent peer, and the link contract's
deadlock argument is untouched.

Ruling T145, verbatim:

> The mirror protocol is fixed so that a session awaiting a data stream its peer owes also observes the control stream, and a control EOF (or any control-stream failure) ends the wait with an error naming the peer's departure; no session path parks indefinitely on a peer that has gone. With that in place, the intra-process disruption arm draws vanishes at every point (first connect, control-half byte offsets, and data streams mid-frame) and asserts precisely that: every survivor of a vanish ends its session with an error, never parking past the closed-world poller's verdict, and the six invariants hold; the ignored deterministic test is un-ignored and becomes a committed point of the same property. Under the link contract the caller still owns the timeout for a peer that is alive and silent; a peer whose control stream has closed is not that case.

## Ground rules

- **Base.** This lane stacks on `triage/p1-harness-tests`, which carries the
  vanish fault and the ignored test. Your worktree's HEAD must equal `<parent
  sha>` before you start. Run `git -C <worktree> rev-parse HEAD`. If HEAD is an
  ancestor of `<parent sha>`, fast-forward; if it has diverged, stop and report.
  Never call EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The rulings are the specification.** T145, and T143 (the finding and the
  instrument: `tests/common/fault.rs`'s `Vanish`, `drive`, `Driven`), in
  `../rulings.md`; the member entry states the resolution in terms of the code.
  Line anchors are at `main` as this brief was written; re-anchor from the named
  functions, never from the line numbers.
- **Goal beside mechanism.** Where the stated resolution and the goal above come
  apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything that
  moves an `insta` snapshot or a committed pin; any change to a public signature
  or public rustdoc contract beyond the one variant this brief proposes;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation from
  the stated resolution; anything this brief marks as a stop.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it (an asserted `Err`, or a
  reversible mutation whose observed failure the commit message records
  verbatim); Acceptance below names them.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never iterate
  on a timing measurement. The box (`ox-east-1`, per the `building-on-illumos`
  skill) is where the lane builds, tests, and gates: `on-illumos.sh <worktree>
  'just gate'`, one run per commit series, in the background redirected to a log
  under `<scratchpad>/p2-vanish-liveness/` and polled with short foreground
  checks (the foreground cap is ten minutes). The box gate is the gate of
  record; the Mac runs no gate. `fuzz` is expected red there (libFuzzer has no
  illumos port) and counts as clean when it is the only failure: quote that line
  and run no fuzz build elsewhere. Never run `just all` or `just ci`. Cargo runs
  `--locked` on the box; nothing is edited or committed there. Check clock skew
  before each box run (rsync preserves mtimes; a box clock ahead by seconds
  means a green build of stale code): on skew, use a fresh box target directory
  or wait, and say which. If clippy's `missing_const_for_thread_local` fires on
  illumos, pass `RUSTFLAGS="-A clippy::missing_const_for_thread_local"` for that
  run. Hold a launch while the box's one-minute load sits above about 150.
- **Commits.** One commit per logical unit, its message describing the change
  and naming the entry id and ruling. Commit every proptest seed file that
  appears. Prose speaks in the present tense: no reference to code that no
  longer exists, no dated rationale at a declaration site. Comments use spaced
  double-hyphens, never em-dashes; every test has a doc comment stating its
  invariant. Never delete anything outside your worktree; if the disk fills,
  stop and report.
- **Self-retirement.** After the final commit, from inside the worktree: `cargo
  metadata --no-deps --format-version 1 | jq -r .build_directory`, then delete
  that directory and the worktree's `target/`. Leave the worktree in place.
- **Report.** For the entry: landed, stopped, or open; the commit sha(s); the
  acceptance evidence (the command and its decisive output, verbatim); then
  anything left open and why. Your report is data: the coordinator verifies the
  Acceptance against the tree at the reported sha before the ledger records it.
  Report what you could not do rather than working around it.

## Members

### vanish-liveness: ruling T145

**Where the session parks.** Every session that opens streams (gossip,
bootstrap, retire; the equal-version short-circuit is exempt) runs its descent
through `Work::execute` (`src/tree/mirror/streaming/remote/proxy/work.rs:197`),
a biased `tokio::select!` over the protocol, the incoming-stream error route,
and the accept driver, `AcceptDriver::run`
(`src/tree/mirror/streaming/remote/streams.rs:694`), whose loop awaits
`self.acceptor.accept()` in `AcceptDriver::accept_one` (`streams.rs:716`). When
the peer dies after the greeting and before its first open, that await never
resolves (a dead listener never accepts; `Driven` models that), the protocol arm
waits on the claim that stream would fill, and the error route has nothing to
report. The control read half sits in `Physical` (`work.rs:86`), destructured at
`work.rs:205` and returned untouched at `work.rs:237`: nothing polls it between
the greeting reads in `start.rs` and the post-descent readers, so the EOF it
already holds is never observed, and no future exists for the session to select
on.

**Resolution.** Add a fourth arm to the select in `execute`: a departure watch
over `control_read`, last in the biased order, so a completion or a violation
observed in the same poll still wins. The watch polls the half into a one-byte
buffer. Zero bytes, or an error, resolves the arm with the error below; the
supply attribution in `execute`'s tail is unchanged. One byte means the peer is
alive and has finished its own descent (the only bytes a peer writes on control
after the greeting are post-descent: the epilogue marker, or a hand-off's
trailing party frame), so the watch retains the byte and parks, never resolving.
The retained byte must reach the post-descent control readers: `execute` returns
it beside the halves; `Handshaken::reconcile`, `Reconciliation::reconcile`
(`src/peer/gossip.rs:1127`), and `bootstrap_reconcile` (`gossip.rs:1186`) carry
it; each reader (`epilogue`, `gossip.rs:1285`; `party::receive`,
`gossip.rs:761`; bootstrap's trailing-frame read) reads through
`AsyncReadExt::chain` of the byte and the erased half. All of those signatures
are private. A lookahead adapter behind the erased `DynRead` is the alternative;
prefer the retained byte unless it comes out ugly, and say which you chose.
Re-state the docs on `execute` and `AcceptDriver::run` for what IS: the driver
still parks on a supply failure, and the departure watch is why the session does
not.

The error: one new variant on the remote proxy's `Error<E>`
(`src/tree/mirror/streaming/remote/proxy/error.rs`, public as `RemoteError`,
`#[non_exhaustive]`), proposed as

```rust
/// The peer's control stream closed or failed while the session awaited
/// a data stream the peer owed: the peer departed mid-session.
#[error("peer departed mid-session: its control stream closed or failed")]
PeerDeparted(#[source] std::io::Error),
```

with an `UnexpectedEof` source for a clean close and the transport's own error
otherwise; callers see it as
`Error::Mirror(MirrorError::Server(RemoteError::PeerDeparted(_)))`, and
`honest_remote` in `tests/common/sim.rs` admits it through `honest_io`. This is
the one public change the lane may land, and it is a stop the brief names: land
it as proposed, report the variant as a numbered stop item, and the entry stays
open until Finch rules on the shape. The `Link` docs in `src/link.rs` stay
untouched (a closed control stream is not the stalled peer they describe); a
sentence you judge they need is a stop item, never an edit.

The harness: widen `arb_fault_or_vanish` in `tests/common/sim.rs` to draw
`Vanish::AtFirstConnect`, a control-half point (a new `Vanish` variant with a
byte offset; the wrappers already pass `None` as the control half's ordinal to
`VanishState::admit`), and `OnStream` as now; delete the paragraph narrowing the
draw; re-state `SESSION_DEADLINE`'s doc (a survivor that parks is a bug, never a
wait left to the caller); extend `plan_population_contains_vanishes` so every
`Vanish` variant is drawn. The survivor assertion, precisely: in the
deterministic tests, `survive_a_vanish` yields `Ok(Err(_))`, never
`Err(Stalled)` or `Err(PollBudget)`, and `assert_survivor` holds; in the arm,
every survivor of a vanish resolves `Err` before `SESSION_DEADLINE` (the
`bounded` panic never fires), `assert_survivor` holds, and the six invariants
listed on `disrupted_concurrent_gossip_upholds_party_invariants` hold. Un-ignore
`survivor_notices_a_peer_vanished_before_its_first_stream`; its expected outcome
is `Ok(Err(e))` with `e` honest: `PeerDeparted`, or a truncation or supply error
when the vanish races a stream the peer had begun.

**Acceptance.** All on the box, commands and decisive output verbatim:

1. `cargo nextest run --all-features --test disruption -E
   'test(survivor_notices_a_peer_vanished)'` passes both survivor tests, the
   first-stream one no longer ignored.
2. `cargo nextest run --all-features --test disruption` passes at the case
   counts committed on the branch (never lowered), with the widened draw and the
   extended population tripwire.
3. Negative control, a reversible mutation named in the commit message: replace
   the departure watch's future with `cancelled()` (or delete the arm). The
   un-ignored test then fails with `the survivor parked after its peer vanished
   before opening a stream: Stalled`, and the disruption arm fails at the
   deadline (`parked past SESSION_DEADLINE`) on a case drawing a first-connect
   or control-half vanish.
4. Committed unit tests in `remote/proxy/work/tests.rs` on `execute`: control at
   EOF with a silent acceptor and an unfinished protocol resolves
   `Err(PeerDeparted)`; control open and silent stays pending under
   `run_to_quiescence` (`Stalled`: no deadline of the session's own); one byte
   on control before the protocol finishes returns `Ok` with the byte retained.
   Plus one end-to-end point where the peer finishes first, writes its epilogue,
   and drops its link while the survivor's descent still runs: the survivor ends
   `Ok`; report the gap if it cannot be constructed deterministically. Negative
   control: a mutation dropping the retained byte fails that point in
   `epilogue`.
5. `cargo nextest run --all-features -E 'test(/conformance::link/)'` and `cargo
   nextest run --all-features -E 'test(/mirror::streaming/)'` pass unchanged.
6. The box gate clean, `fuzz` the only red, its line quoted.

## Hazards and stops

- Any public signature or public rustdoc change beyond `PeerDeparted` is a stop;
  so is any snapshot moving (the watch writes no wire byte).
- The deadlock-freedom argument in `src/link.rs` rests on stream independence
  and control duplex. The watch adds one pending control read beside data-stream
  traffic and an idle control write, which the control-duplex clause already
  licenses, so no clause changes. A reading under which it needs more is a stop,
  not an edit.
- The retained byte crosses every post-descent reader and the `gossip_when`
  driver's reborrowed halves (`gossip.rs:913`); losing it corrupts the epilogue
  as `InvalidData`. Check each before calling the fix done.
- `p2-link` (unmerged) edits `src/link/routed/`; touch nothing there.
  `p1-harness-crate` (unmerged) edits the in-crate suites under
  `src/tree/mirror/streaming/`, `remote/proxy/tests.rs` included; your unit
  tests go in `remote/proxy/work/tests.rs` only. The base already changes
  `src/testing.rs`, `src/testing/transport.rs`, and
  `src/conformance/link/tests.rs`; do not re-edit them.
