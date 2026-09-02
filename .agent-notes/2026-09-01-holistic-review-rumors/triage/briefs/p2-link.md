<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the routed link's pooling bound, name length, and read ids

## Goal

The routed adapter keeps one `pending_headers` bound over two populations:
fresh connections awaiting a header and a pooling dialer's already-admitted
idle connections. Under pressure it evicts the admitted idle ones first, so
a healthy link's next stream open fails with `BrokenPipe` (demonstrated on
the in-memory network under `pending_headers: 1`); at four pooling peers
under the default `Config` the failure is reachable. Beside it: the link
header writes the advertised name's length through an `as u8` cast behind
a debug-only guard, and the router's read-id counter overflows in debug
after 2^64 arrivals. The invariants restored: an admitted pooled connection
is closed only by its dialer or by transport failure, never by count; the
name's length bound lives in a type; and the id counter owes distinctness
only among live entries.

## Ground rules

These apply to every P2 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `62b30e1f` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `62b30e1f`, fast-forward; if it has diverged, stop and report. Never call
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
  run in the background redirected to a log under `<scratchpad>/p2-link/`,
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

## Members

### link-28 (medium): ruling T31

Resolution: separate the two populations. Give recovered connections their own bound and apply it before `READY` is written: a return that finds the recovered budget full is dropped pre-`READY` (the dialer's pool then never admits it, since the pre-`READY` drop path already exists in embryo at router.rs:229-231), and an admitted connection is thereafter closed only by the dialer or by transport failure, never by count. The smallest form is a counter of recovered connections currently in `pending`, checked before the `READY` write. Keep oldest-first count eviction for fresh mid-header connections only, whose dialer is still inside its open and observes the drop as a failed open. Make routed.rs:77-79 and endpoint.rs:52-55 agree whichever design is chosen. If the owner prefers one bound, at minimum re-denominate `DEFAULT_PENDING_HEADERS`'s rationale for the pooled idle population and land the constructed test as the documented failure's witness. Acceptance: a committed test with a pooling `Dial` in which recovered idle connections exceed `pending_headers` completes a later session with no failed or hung stream (new design), plus a negative control showing a pre-`READY` drop leaves the pool without that connection; or, under the current design, the constructed test pins the failure and the default's doc states the pooled sizing rule.

Ruled (T31): the new design (a pre-`READY` bound for recovered
connections; pooling is first-class). The witness pass's test
(`evidence/witness.md`, section `link-28`) is the committed pin of the old
failure, inverted to assert the healthy behavior. The TCP-pool hang
variant was never run; run it once under `tokio::time::timeout` as the
construction describes and report the outcome, before and after.

Public surface this ruling authorizes: a new field on `Config` for the
recovered-connection bound with its `Default` sized for pooling (name it
in the report), and the `Dial`/`Conn`/`Endpoint` rustdoc changes that
describe the two bounds. Any other `pub` change is a stop. Ordering hazard
from TRIAGE.md: this lands before any router counters (owner decision 22,
P6).

### link-14 (medium): ruling T31

Resolution: in the example `TcpDial::dial` call `stream.set_nodelay(true)?` before returning, and in the example `TcpListen::accept` for the accepted connection (whose writes are the ACK/READY bytes and the control stream's frames). Name `TCP_NODELAY` in the `Dial` docs' policy list (routed.rs:214-216) and add a sentence to the `Conn` docs (routed.rs:193-197) that Nagle is the kernel-side form of the hidden write buffering the clause warns about: liveness survives, latency does not. Apply the same to `tests/common/routed_tcp.rs` (both dials) and `tests/common/tcp.rs` so the crate's own TCP runs measure the intended configuration. Acceptance: the example, the `Dial` docs, and both TCP test harnesses set or name `TCP_NODELAY`; a before/after wall-clock timing of `tests/routed_link.rs`'s pooled mutual-gossip test over loopback shows the per-frame stall gone. If the measurement shows no difference, the finding reduces to the documentation change alone.

"The example" is the routed TCP example in the `Dial` docs; T27 deleted
the swarm example, so if the only in-tree `TcpDial` lives in
`tests/common`, apply the change there and to the doc's inline example.
The before/after timing is one run each way, reported with the load
average; do not iterate on it.

### link-25 (nit): ruling T44, resolution amended

Resolution: `bytes.push(u8::try_from(addr.len()).expect("the endpoint validates the advertised name's length at construction"));` and drop the `debug_assert!`; or carry the validated encoding as a newtype so the bound is in the type. Acceptance: no `as u8` in header.rs; the `expect` message names the construction-time check.

Ruled (T44): the newtype. The advertised name is carried as a type whose
constructor is the one place its length is checked against
`MAX_ADDR_LEN`; `link_header` takes that type and writes its length
without a cast or a guard; the endpoint's construction-time validation
moves into the constructor. If the newtype reaches a `pub` signature
(`Endpoint::new`'s name parameter, or a `LinkInfo` accessor), that is a
stop: report the signature and leave the entry open with the `try_from`
form landed as the interim, saying so.

### link-29 (nit): ruling T45

Resolution: `next_id = next_id.wrapping_add(1);` with a one-line comment that at most `pending_headers` ids are live at once. Acceptance: no unchecked increment on the id counter.

## Hazards and stops

- `src/link/routed/` is production code with a committed wire pin for the
  routed header (`tests/routed_link.rs` and the bookmark-style snapshots,
  if any); the header bytes must not change. A snapshot moving is a stop.
- The `Config` default for the new bound is a number you derive from the
  pooling arithmetic in the entry (four pooling peers under the default),
  stated in the field's doc with its derivation.
