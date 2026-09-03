<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T86, T128, and T132 (T63's row named, not landed) in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: one home per mechanism in the streaming core

## Goal

The streaming core (mirror common, the materialized walk, the backend
and window, the channel module) has one home per mechanism: the
channel-type swap lives in `channel.rs` and its consumers carry no
`cfg` fork, in the full form that lets `tokio-stream` leave the
manifest (T128); the core's import cycle closes with the target
threaded into the walk (T86); the handshake phases share one greeting
derivation and one opening; and the poll-delay scheduler, the boxing
rationale, the pump, `node_bytes`, the re-export shim, the double box,
the cloned greeting, and the positional solves each have one spelling.
No wire byte moves. Effort: high.

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
  run in the background redirected to a log under `<scratchpad>/p4-streaming-simplify/`,
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

1. **deps-2 in its full form first** (T128), then materialized-16,
   mirror-common-25, and module-graph-3 collapse onto it in one commit;
   `materialized/common.rs` dissolves. Oracle:
   `git grep -n 'cfg(not(test))' -- src/tree/mirror/streaming` returns
   only `channel.rs` and `backend/local.rs`;
   `git grep -n tokio_stream -- src Cargo.toml` empty;
   `cargo tree -p rumors -e normal --depth 1` omits it. Standing check:
   the manifest (a reintroduced `tokio_stream::` path fails
   `cargo check --locked`).
2. **T86**: module-graph-2 (a) through (d), with (c) threading the
   target; `analyze.py`'s sibling-subtree section quoted from the
   note's `evidence/` tree.
3. **Mirror common and the walk**: materialized-11, mirror-common-16,
   -17, -32, -6, module-graph-8, -13, materialized-36, -4.
4. **Backend, window, and the in-crate tests**:
   streaming-backend-window-13, -20, -28, -29, -31, -8;
   streaming-tests-12, -14.

Closing oracle: `git grep -n -E 'pub use .*\*;' -- src` empty;
`git grep -n 'Box::pin(handshaken.reconcile())'` empty; one
`Greeting { .. }` literal and one `window.resolve(` in
`materialized.rs`; no bare `.min(2)`;
`grep -rn '16_384\|2_048' src/tree/mirror/streaming/tests*` hits only
definitions; the greeting snapshots unchanged (`just test`).

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **module-graph-2** (medium; T86; T86: (c) threads the target into the walk; `analyze.py` is in this note's `evidence/` tree, run from there and quoted (no standing check)). `src/tree/mirror/streaming/erased.rs:203-206`. Resolution: (a) Move `children_of` from `materialized/common.rs:17-36` into `erased::ops` (its only consumer) or `backend.rs` beside `Backend::children`, and delete the re-export and its comment at `materialized.rs:186-188`. (b) Move the `SupplyLedger` struct with `new` and `charge` (`materialized.rs:197-241`) to a neutral module (`streaming/ledger.rs`, or `message.rs` beside the `Greeting` whose `set_len` it enforces); `absorb` depends on `materialized::Error`/`Violation`, so it stays in `materialized.rs` as an inherent impl on the moved type, which Rust permits within one crate. (c) `DEFAULT_TARGET_MESSAGE_SIZE` stays in `codec::budget` (its derivation is codec vocabulary); either thread the target through `materialized::Handshaking::start` so the walk stops defaulting it, or state at `materialized.rs:114` that the walk adopts the codec's default. (d) State at `window.rs:123` that the `Resolve` import exists for `REFERENCE_SLOT_BYTES`'s `size_of` pricing. Acceptance: `analyze.py`'s sibling-subtree section under `tree::mirror::streaming` lists at most the two documented edges (`window → materialized`, and `materialized → remote` if (c) takes the documenting option), and `common.rs` holds only `ok_channel` or dissolves with module-graph-3.
- **deps-2** (medium; T128; T128: the full form (b), so `tokio-stream` leaves the manifest; T86's cycle work depends on it). `src/tree/mirror/streaming/channel.rs:75-76`. Resolution: Move the boundary into channel.rs. (a) Minimal: channel.rs exports under both cfgs a `pub type ReceiverStream<T>` (tokio-stream's wrapper in production, `Receiver<T>` under test) and `pub fn into_stream<T>(rx: Receiver<T>) -> ReceiverStream<T>`; erased.rs:141-159 and common.rs:47-65 collapse to one line each and lose their duals; assembly.rs:7 becomes `futures::StreamExt`. (b) Full: in production channel.rs defines `pub struct Receiver<T>(tokio::sync::mpsc::Receiver<T>)` implementing `Stream` via `poll_recv` and forwarding `recv` (the only method channel.rs consumers call: materialized.rs:874, levels.rs:140/381/565/662, encode.rs:59/95, pump.rs:191/289/383), which makes both variants `Stream`; decode.rs's two raw channels then either route through channel.rs (which also brings the adapter's leaf channel under the instrumented channel's test observation) or take a local `poll_recv`-based wrapper, after which `tokio-stream` leaves rumors' `[dependencies]` and the workspace table. Acceptance: erased.rs and common.rs contain no `#[cfg(test)]`/`#[cfg(not(test))]` pair around the receiver-stream type; `grep -rn 'tokio_stream::StreamExt' src/` is empty; under (b), `grep -rn tokio_stream src/` is empty and `cargo tree -p rumors -e normal --depth 1` omits tokio-stream.
- **materialized-11** (low; T132; T40 (`p2-walk`, merged) rewrote `materialized.rs`: re-anchor the three sites from the quoted evidence). `src/tree/mirror/streaming/materialized.rs:563-602`. Resolution: (1) `accept` becomes `let (greeting, next) = Connect::connect(self).await?; Ok((greeting, CompleteConnect::complete_connect(next, request).await?))`; if the owner prefers to keep the two impls independent, hoist the literal into `fn greeting(&self, fan: &[(u8, B::Erased)]) -> Greeting` carrying the two comments. (2) `fn open(self) -> Opened<B>` on `Handshaking<B, Connected<B>>` returning `{ ceiling, their_version, their_listing, fan, ledger, work }`, called by both roles. (3) Remove `our_version` from the three version structs (`Start` becomes a unit struct) and read `self.root.ceiling` at the greeting and the join. Acceptance: one `Greeting { .. }` literal and one `window.resolve(` in `materialized.rs`; `our_version` absent; the greeting snapshots (`tests/gossip_snapshot.rs`) unchanged, since the bytes derive from the same fields.
- **materialized-16** (low; T132). `src/tree/mirror/streaming/materialized/common.rs:38-65`. Resolution: In `channel.rs` add one `ReceiverStreamOf<T>` alias and `pub fn into_stream<T>(rx: Receiver<T>) -> ReceiverStreamOf<T>` beside the existing cfg swaps; in `queues.rs` define `ok_channel(role, buffer) = { let (tx, rx) = channel(role, buffer); (tx, into_stream(rx).map(Ok)) }` and one `OkReceiverStream` alias; have `erased::reply_channel` use the same `into_stream`; delete `common.rs` and the `use common::*` glob. Acceptance: no `common.rs` under `materialized`; `#[cfg(test)]`/`#[cfg(not(test))]` pairs concerning channel types appear only in `channel.rs`.
- **materialized-36** (low; T132; `resolver.rs` was edited by `p2-walk`: re-anchor). `src/tree/mirror/streaming/materialized/work/resolver.rs:61-65`. Resolution: Return `Option<Dispute<B::Erased>>` with `struct Dispute<E> { child_prefix: ErasedPrefix, radix: u8, node: E, listing: Vec<(u8, Hash)> }`, field docs stating each role, and a doc on `react` naming its three outcomes; drop the `allow`. Acceptance: no `prefix.push(radix)` following `react` in levels.rs; `react` documented.
- **materialized-4** (nit; T132). ``src/tree/mirror/streaming/materialized.rs:228-241``: `SupplyLedger::charge` reports its overdraw as a bare `u64` Resolution: Introduce `pub(crate) struct Overdrawn { pub declared: u64 }` as the `Err` type
- **mirror-common-16** (low; T132). `src/tree/mirror/streaming.rs:88-92`. Resolution: In `handshake`, take `peer_version` and `peer_len` before `complete_connect(peer)` and store them in place of `peer: Greeting`, symmetric with `our_version`/`our_len`; rename `peer()` to `peer_version()` (three gossip.rs readers). Optionally name the pair (`ElectionKey { set_len: u64, version: Version }`) so `descend(local, remote, ours, theirs)` and `initiates(ours, theirs)` speak one vocabulary. Acceptance: no `peer.clone()` in streaming.rs; `Handshaken` holds no `Greeting`; gossip.rs compiles against `peer_version()`; the streaming suites pass unchanged.
- **mirror-common-17** (low; T132). `src/tree/mirror/streaming.rs:110-116`. Resolution: Keep the inner box (it is what satisfies `clippy::large_futures` at the await sites) and delete the callers' re-boxing: `let (root, (read, write)) = handshaken.reconcile().await.map_err(streaming_error)?;` at gossip.rs:1164-1165, :1223-1224, and gossip/tests.rs:218-219. State the reason at `reconcile`'s doc ("Boxed: the descent state machine is a large future; the box keeps every await of it under the crate's `large_futures` ceiling."). Acceptance: exactly one `Box::pin` stands between `reconcile`'s body and each `.await`; `grep -rn 'Box::pin(handshaken.reconcile())'` is empty; gossip, bootstrap, and `tests/future_size.rs` pass unchanged.
- **mirror-common-25** (low; T132). `src/tree/mirror/streaming/erased.rs:141-159`. Resolution: In `channel.rs`, export one stream-typed receiver under both cfgs (`pub type ReceiverStream<T> = tokio_stream::wrappers::ReceiverStream<T>;` / `= instrumented::Receiver<T>;`) and a `stream_channel(role, capacity) -> (Sender<T>, ReceiverStream<T>)`. Then `erased.rs` drops `ReceiverStreamOf`, `receiver_stream`, and its `tokio_stream` import, and `common.rs` reduces `OkReceiverStream` to one alias over `channel::ReceiverStream<T>` and `ok_channel_with` to `(tx, rx.map(Ok))`. Acceptance: no `cfg(test)` on a channel type outside `channel.rs`; `tokio_stream::wrappers::ReceiverStream` is named only in `channel.rs` and `adapter/decode.rs`.
- **mirror-common-32** (nit; T132). ``src/tree/mirror/streaming/protocol.rs:116-118``: The "rustc explodes" boxing rationale is pasted three times instead of stated once at `BoxResponses` Resolution: State the boxing rationale once on `BoxResponses`'s doc and delete the three pasted comments
- **mirror-common-6** (low; T132). `src/tree/mirror/framing.rs:55-65`. Resolution: Move `LengthOverflow` beside `checked_run_len` (or into `remote/codec/error.rs`); repoint the `pub use` at remote/error.rs:19 so `rumors::error::LengthOverflow` is unchanged. Acceptance: framing.rs defines only `PAYLOAD_CHUNK_LEN`, `chunk_boundary_cuts`, `read_payload`, `resume_payload`; the public path still resolves.
- **module-graph-13** (nit; T132). ``src/tree/mirror/streaming/materialized/work.rs:134-153``: Two near-identical `pump` helpers diverge on a closed receiver without saying why Resolution: Lift one `pump` into `tasks.rs` parameterized by the closed-receiver policy, or comment at each site why the walk returns where the proxy parks
- **module-graph-3** (low; T132). `src/tree/mirror/streaming/channel.rs:75-90`. Resolution: Export from `channel.rs`, under both cfgs, one `ReceiverStream<T>` type and `fn into_stream(rx: Receiver<T>) -> ReceiverStream<T>` (production wraps `tokio_stream::wrappers::ReceiverStream`; the test arm is the identity because the instrumented `Receiver` implements `Stream`), then delete `erased.rs:141-159` and `common.rs:47-65`'s forks in its favour. Leave `local.rs`'s four adversarial forks unless a no-op `adversarial` shim reads better than four visible forks. Acceptance: `grep -rn 'cfg(not(test))' src/tree/mirror/streaming` returns only `channel.rs` and `local.rs`.
- **module-graph-8** (low; T132). `src/tree/mirror/streaming/materialized/channel.rs:1-6`. Resolution: Delete `materialized/channel.rs` and repoint its six consumers at `crate::tree::mirror::streaming::channel`; replace `remote.rs:85` with the explicit list (moving `remote/error.rs:1-6`'s doc onto it) or keep `error.rs` and re-export it by name. Acceptance: `grep -rn 'pub use .*\*;' src` returns nothing; `materialized.rs` declares no `channel` module.
- **streaming-backend-window-13** (low; T132). `src/tree/mirror/streaming/backend/local/adversarial.rs:17-44`. Resolution: Extract one test-support module (for example `streaming/testing/schedule.rs`) with a `ScheduleCell` around a `thread_local!` `RefCell<Option<Schedule>>` exposing `with` and `next`, plus a `Countdown(Option<u8>)` whose one method performs the wake-and-`Pending` step; instantiate two cells (backend, channel). Name the cap (`MAX_SCHEDULED_DELAY: u8 = 2`) with one sentence on why delays are bounded, and use it at transport.rs:180 too. Add `RoleStats::absorb(&mut self, other: RoleStats)` and call it from both folds. Acceptance: one definition each of the schedule struct, the restore guard, the next-delay read, the countdown step, and the `RoleStats` merge; both cells still independently settable; no bare `.min(2)`.
- **streaming-backend-window-20** (nit; T132). ``src/tree/mirror/streaming/convert.rs:83-91``: `S<H>::assemble` relays `fold_parents` through a pass-through generator left over from watermark stripping Resolution: `Box::pin(fold_parents(backend, H::assemble(backend.clone(), leaves)))`, the form the parent of 748325407 compiled
- **streaming-backend-window-28** (low; T132; the proxy call site (`start.rs`) is `p4-remote-simplify`'s file: change the one line here in the same commit and announce it in the merge queue). `src/tree/mirror/streaming/window.rs:338-345`. Resolution: Introduce a small `Copy` struct (`Corpus { messages: u64, version_bytes: u64 }`, or reuse the greeting's pair) and take `local: Corpus, remote: Corpus`; derive it from `Root<B>` on the walk side and from the two greetings on the proxy side; shrink the test helpers accordingly. Acceptance: no call site passes four bare `u64`s to the solve; window/tests.rs compiles against the struct form and still passes.
- **streaming-backend-window-29** (low; T132; take the first arm (delete the assert; cite the conformance sweep `node_bytes_monotone`, landed by T6)). `src/tree/mirror/streaming/window.rs:360-370`. Resolution: Delete the assert and re-state the three prose references (backend.rs:113 "debug-asserted when a session derives its window", window.rs:337 "spot-checked here in debug builds", conformance/backend.rs:580-581) to name the conformance sweep alone; or keep it and say at the site that it guards test-supplied pricing closures, which is the only space the sweep misses. Acceptance: either no `debug_assert` on `node_bytes` in `from_budget` and the three sites cite `node_bytes_monotone`, or the assert's site names what it catches.
- **streaming-backend-window-31** (nit; T132). ``src/tree/mirror/streaming/window.rs:478-480``: `Window::capacity` clamps an out-of-range height that only programmer error can produce; two `unwrap_or`s guard conversions that cannot fail Resolution: Index directly with a one-line comment that typed heights bound the argument (or `debug_assert!(height <= KEY_DEPTH)` and index)
- **streaming-backend-window-8** (nit; T132). ``src/tree/mirror/streaming/backend/local.rs:103-129``: `Local::node_bytes` exists twice: an inherent fn and a trait impl that delegates to it Resolution: Move the doc onto the `Backend` impl's `node_bytes`, delete the inherent fn, and have testing.rs write `<Local as Backend>::node_bytes`
- **streaming-tests-12** (low; T132). `src/tree/mirror/streaming/tests/capacity.rs:42-52`. Resolution: In tests.rs define `const MAX_DELAY: u8 = 2` (asserted equal to the consumers' clamp, or exported from them), `const SCHEDULE_LEN: usize = 16_384` with a one-line derivation, a `fn round_robin(modulus: u8) -> Vec<u8>`, and one `fn standard_schedules()` used by `assert_capacity_case`, `probe_schedules`, and `honors_redaction_under_leaf_parent_dispute`; rewrite or delete the `% 5` row. Optionally have `with_schedule` return the steps consumed so a test can assert `consumed <= SCHEDULE_LEN`. Acceptance: `grep -rn '16_384\|2_048' src/tree/mirror/streaming/tests*` hits only the constant definitions; the `% 5` row is gone or stated in terms of `MAX_DELAY`.
- **streaming-tests-14** (nit; T132). ``src/tree/mirror/streaming/tests/capacity.rs:116-123``: Statements that carry nothing: a root returned to be dropped, a bound implied by the line above it, a conditional implied by reflexivity Resolution: Return `()` from the closure and delete `drop(pair)`

## Named here, landed elsewhere

These rows belong to this lane's pattern and are counted in its roster, but another ruling's lane lands them; this lane verifies the state at base, quotes it in the report, and edits nothing at their sites.

- **materialized-10** (xref, T86): T86 cross-reference of module-graph-2 (the constant half); landed by module-graph-2's commit, no separate work.
- **mirror-common-10** (medium, T63): T63 (P6): `Preamble::decode` over the fixed array reopens ruling R2 there; verify at base and do not touch `handshake.rs:111-159`.

## Hazards and stops

- T40 (`p2-walk`, merged) rewrote `materialized.rs`, `levels.rs`, and
  the resolver: re-anchor materialized-11, -36, and module-graph-13.
- mirror-common-10 is T63's (P6); `Preamble::decode` stays as it is.
- T118 (P8) adopts the proxy's `Progress` in the walk: not this lane.
- streaming-backend-window-28's proxy call site is one line in
  `start.rs`; land it here and announce it in the merge queue.
- materialized-12 (`+ Sync`) is `p4-leftovers`': do not touch.
