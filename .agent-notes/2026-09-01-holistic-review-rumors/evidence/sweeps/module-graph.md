# Sweep module-graph: Module dependency graph and layering

## Method and coverage

The sweep built the module graph of `src/` with a script (`scratchpad/sweeps/module-graph/`: `modgraph.py` extracts every `mod` declaration and every `use crate::`/`super::`/`self::` path and resolves each to a module; `analyze.py` computes SCCs, top-level layers, and sibling-subtree cycles). I did not re-run the script; I read its `analysis.txt` and then opened every site every finding cites, at HEAD `9e5784fb4dce977cfbdfd1619886d1482b5ce764` in a clean tree.

Mechanical checks I ran myself, read-only:

- `grep` for every use site of `Inner`, `Protocol`, `children_of`, `SupplyLedger`, `DEFAULT_TARGET_MESSAGE_SIZE`, `alternating`, `Levels`, `untyped::Range`, `traverse::act`, `traverse::unknown`, `mirror::remote`, `mpsc`, and the `materialized::channel` shim's consumers.
- A shell loop over every non-test `.rs` under `src/` printing the first non-blank line where it is not `//!`: 27 files, matching the sweep's count.
- A grep for brace-bodied `mod` declarations, and a brace-matching script for their spans: six inline test modules (matching the sweep) and, beyond the sweep's three, two more inline production modules (`untyped::census`, 27 lines; `decode::fan_probe`, 37 lines) plus `height::sealed` (6 lines, the sealed-trait idiom, exempt).
- `git ls-tree HEAD~60 src/tree/mirror/` (shows `alternating.rs` beside `streaming.rs`), `git log --grep=alternating`, `git log -1 368da2a5` and `c13c21b4`, `git log --follow` on the shim and glob files, `git blame` on `remote.rs:85`.
- `.agent-notes/2026-09-01-v1-retirement/README.md` for the rulings on `Protocol`; `.agent-notes/2026-08-20-cbor-wire-review/REVIEW.md` for the error re-export routing ruling; grep of both review packets for anything on the sites below.
- `tools/` and the `justfile`: nothing mechanical checks test-module placement, `//!` presence, or the `#[cfg(test)]` convention; `doclint` checks summary length and `include_str!` separation only; there is no `[lints]` table anywhere in the workspace.

Not run: cargo, just, or any test (no finding rests on a correctness claim). Not seen: `benches/`, `examples/`, and the sibling crates, except where a finding names them.

## Findings

### module-graph-1: Two single-site upward imports into `error` close the top-level cycle
- Where: src/tree/mirror/party.rs:6-11 (related: src/tree/mirror/party.rs:76-77, :102, :119-120, :131-137, :153-159; src/link.rs:385-388; src/peer/gossip.rs:959, :1021-1026, :1246; src/tree/mirror/handshake.rs:172; src/error.rs:32-50, :292-314)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (read every site; grep of `LinkPoisoned`, `HandOff`, `party::`, `handshake::` across `src/peer/`)
- Verification: confirmed; history: no-rationale-found (the neighbouring `handshake` module shows the intended shape and `error.rs:292` lifts it)
- Owner-gated: no

The only production edges from the `tree` and `link` subtrees into the top-level `error` module are `party.rs` constructing `Error::HandOffMalformed`/`HandOffTruncated`/`Io` and `link.rs`'s `begin` returning `crate::Error::LinkPoisoned`. The sibling parser `handshake.rs` defines a local `pub(crate) enum Error` that `error.rs` lifts by a `From` impl; these two sites are the asymmetry that folds `batch, bookmark, error, link, peer, rumors, snapshot, tree` into one SCC. Beyond the graph, `link.rs` naming the public error sum makes the transport contract recompile on every variant that sum aggregates (`error.rs:32-50` imports from `bookmark`, `peer`, and the whole mirror tree).

Evidence:

         6	use crate::{
         7	    Error,
         8	    observe::{CaptureRead, SessionHandle},
         9	    tags::PARTY_TAG,
        10	    tree::mirror::cbor::{self, HeadError, MAJOR_BSTR},
        11	};
    ...
       102	    let malformed = |defect| Error::HandOffMalformed { defect };
    ...
       119	            std::io::ErrorKind::UnexpectedEof => Error::HandOffTruncated,
       120	            _ => Error::Io(e),

    src/link.rs:
       385	    pub(crate) fn begin(&mut self) -> Result<u8, crate::Error> {
       386	        if self.poisoned {
       387	            return Err(crate::Error::LinkPoisoned);
       388	        }

    src/tree/mirror/handshake.rs (the template):
       170	/// A malformed, incompatible, or truncated preamble.
       171	#[derive(Debug, thiserror::Error)]
       172	pub(crate) enum Error {

    src/peer/gossip.rs (the funnel already owns the poison semantics):
       959	                        return Some((Err(Error::LinkPoisoned.widen()), drive));
    ...
      1021	                    let epoch = match drive.state.begin() {
      1022	                        Ok(epoch) => epoch,
      1023	                        Err(e) => {
      1024	                            drive.done = true;
      1025	                            return Some((Err(e.widen()), drive));
    ...
      1246	    let epoch = link.session.begin()?;

Resolution: In `party.rs` define `pub(crate) enum Error { Io(io::Error), Malformed(HandOffDefect), Truncated }` mirroring `handshake::Error`, and add `impl From<party::Error> for Error<NoBookmark>` beside the existing `From<handshake::Error>` at `error.rs:292`; the three call sites in `gossip.rs` (`:320`, `:761`, `:782`) already use `?` or `match`. In `link.rs` have `begin` return `Result<u8, Poisoned>` (a unit struct) and map it at the two `gossip.rs` sites (`:1021-1026`, `:1246`), which sit beside the site (`:959`) that constructs `LinkPoisoned` directly today. Acceptance: `analyze.py` reports no edge from `tree::*` or `link::*` into `error`; the top-level SCC dissolves into layers with `error` above `tree` and `link`.

### module-graph-2: The streaming core 4-cycle is closed by single-item imports, two of them misplaced items
- Where: src/tree/mirror/streaming/erased.rs:203-206 (related: src/tree/mirror/streaming/materialized.rs:114, :186-188, :197-251, :438, :487; src/tree/mirror/streaming/materialized/common.rs:17-36; src/tree/mirror/streaming/remote/proxy/work.rs:19; src/tree/mirror/streaming/remote/adapter/decode.rs:15, :166, :373; src/tree/mirror/streaming/remote/proxy/work/pump.rs:40; src/tree/mirror/streaming/remote/codec/budget.rs:64-71; src/tree/mirror/streaming/window.rs:123, :154-156; src/tree/mirror/streaming.rs:10-24)
- Class / severity / confidence: modularity / medium / high on the facts, medium on the shape of the fix
- Provenance: verified (read every site; grep of every `children_of`, `SupplyLedger`, `.absorb(`, `.charge(`, and `DEFAULT_TARGET_MESSAGE_SIZE` use)
- Verification: reframed: the facts hold, but two of the sweep's three moves do not work as written (see below); history: no-rationale-found
- Owner-gated: no

Among the direct children of `tree::mirror::streaming`, `erased → materialized` (`children_of`), `materialized → remote` (`DEFAULT_TARGET_MESSAGE_SIZE`), `remote → materialized` (`SupplyLedger`, three files), and `window → materialized` (`Resolve`) form the only non-trivial cycle in the protocol core, and it defeats the layer order `streaming.rs:10-24` draws. Two of the four closing items live in the wrong module: `children_of` is a generic `Backend` helper whose single consumer is `erased::ops` (the re-export comment at `materialized.rs:186-187` credits the remote proxy, which in fact reaches it through `erased::ops::children_of` at `pump.rs:211`); `SupplyLedger` is greeting-derived session state that its own doc says is shared by the walk and the wire decoder. The other two edges are cheaper than the sweep's resolution assumed: `DEFAULT_TARGET_MESSAGE_SIZE` is derived from codec frame constants and cannot leave `codec`, and the `window → materialized` edge prices `REFERENCE_SLOT_BYTES` by `size_of` a real `Resolve` slot, which is deliberate.

Evidence:

    src/tree/mirror/streaming/erased.rs:
       203	    use crate::tree::{
       204	        mirror::streaming::{
       205	            backend::BoxNodeStream, materialized::children_of as children_of_typed,
       206	        },

    src/tree/mirror/streaming/materialized.rs:
       186	// The remote proxy explodes early-supplied whole root children into the
       187	// same per-child shape the walks consume, with the walks' own helper.
       188	pub(crate) use common::children_of;
    ...
       197	/// The session-total supplied-leaf ledger: the ingestion-side counterpart
       198	/// of the greeting's declared set length, shared by every stage that
       199	/// absorbs supplies.
    ...
       245	    pub(crate) fn absorb<E>(&self, leaves: u64) -> Result<(), Error<E>> {
       246	        match self.charge(leaves) {
       247	            Ok(()) => Ok(()),
       248	            Err(_) => violation(Violation::OverdrawnSupply),

    src/tree/mirror/streaming/remote/codec/budget.rs:
        71	pub const DEFAULT_TARGET_MESSAGE_SIZE: usize = FAN * FULL_FAN_QUERY_FRAME_LEN;

    src/tree/mirror/streaming/materialized.rs (the walk's only use of it):
       437	            window: WindowConfig::default(),
       438	            target_message_size: DEFAULT_TARGET_MESSAGE_SIZE as u64,

    src/tree/mirror/streaming/window.rs:
       123	use super::{Backend, Local, materialized::Resolve};
    ...
       154	const REFERENCE_SLOT_BYTES: usize = std::mem::size_of::<(u8, typed::Node<Z>)>()
       155	    + std::mem::size_of::<(u8, Resolve<<Local as Backend>::Erased>)>()
       156	    + std::mem::size_of::<(u8, typed::Hash)>();

Resolution: (a) Move `children_of` from `materialized/common.rs:17-36` into `erased::ops` (its only consumer) or `backend.rs` beside `Backend::children`, and delete the re-export and its comment at `materialized.rs:186-188`. (b) Move the `SupplyLedger` struct with `new` and `charge` (`materialized.rs:197-241`) to a neutral module (`streaming/ledger.rs`, or `message.rs` beside the `Greeting` whose `set_len` it enforces); `absorb` depends on `materialized::Error`/`Violation`, so it stays in `materialized.rs` as an inherent impl on the moved type, which Rust permits within one crate. (c) `DEFAULT_TARGET_MESSAGE_SIZE` stays in `codec::budget` (its derivation is codec vocabulary); either thread the target through `materialized::Handshaking::start` so the walk stops defaulting it, or state at `materialized.rs:114` that the walk adopts the codec's default. (d) State at `window.rs:123` that the `Resolve` import exists for `REFERENCE_SLOT_BYTES`'s `size_of` pricing. Acceptance: `analyze.py`'s sibling-subtree section under `tree::mirror::streaming` lists at most the two documented edges (`window → materialized`, and `materialized → remote` if (c) takes the documenting option), and `common.rs` holds only `ok_channel` or dissolves with module-graph-3.

### module-graph-3: `channel.rs` swaps channel types by `cfg(test)`, and the split leaks `cfg` forks into three other files, two of them duplicating one adapter
- Where: src/tree/mirror/streaming/channel.rs:75-90 (related: src/tree/mirror/streaming/erased.rs:46-47, :141-159; src/tree/mirror/streaming/materialized/common.rs:4-5, :47-65; src/tree/mirror/streaming/backend/local.rs:142-145, :165-173, :185-188, :222-225; Cargo.toml:145)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (all sites read; `grep -rn mpsc src` for channel bypasses)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`Sender`/`Receiver`/`channel` are Tokio's types under `cfg(not(test))` and the instrumented wrapper under `cfg(test)`. Tokio's `Receiver` is not a `Stream` and the wrapper is, so `erased.rs` and `materialized/common.rs` each carry their own cfg-forked receiver-as-stream alias and constructor, and `backend/local.rs` carries four `#[cfg(test)] return adversarial::...; #[cfg(not(test))] value` forks. The integration-test build compiles the production arms (the self dev-dependency at `Cargo.toml:145` enables `test-internals` without `cfg(test)`), so this is a legibility and duplication cost, not a coverage gap.

Evidence:

    src/tree/mirror/streaming/channel.rs:
        75	#[cfg(not(test))]
        76	pub use tokio::sync::mpsc::{Receiver, Sender};
    ...
        84	#[cfg(test)]
        85	pub use instrumented::{
        86	    ChannelReport, Receiver, Sender, channel, with_kind_capacity, with_observation, with_schedule,
        87	};

    src/tree/mirror/streaming/erased.rs:
       143	#[cfg(test)]
       144	type ReceiverStreamOf<E> = Receiver<E>;
       145	/// The channel receiver as a stream, uniform across the test and
       146	/// production channel types.
       147	#[cfg(not(test))]
       148	type ReceiverStreamOf<E> = ReceiverStream<E>;

    src/tree/mirror/streaming/materialized/common.rs:
        61	#[cfg(test)]
        62	pub type OkReceiverStream<T, E> = stream::Map<Receiver<T>, fn(T) -> Result<T, E>>;
        63	/// The type of a receiver stream wrapping items in `Ok`.
        64	#[cfg(not(test))]
        65	pub type OkReceiverStream<T, E> = stream::Map<ReceiverStream<T>, fn(T) -> Result<T, E>>;

    src/tree/mirror/streaming/backend/local.rs:
       142	        #[cfg(test)]
       143	        return adversarial::stream(adversarial::Role::Children { height: H::HEIGHT }, children);
       144	        #[cfg(not(test))]
       145	        children

    Cargo.toml:
       145	rumors = { workspace = true, features = ["test-internals", "conformance"] }

Resolution: Export from `channel.rs`, under both cfgs, one `ReceiverStream<T>` type and `fn into_stream(rx: Receiver<T>) -> ReceiverStream<T>` (production wraps `tokio_stream::wrappers::ReceiverStream`; the test arm is the identity because the instrumented `Receiver` implements `Stream`), then delete `erased.rs:141-159` and `common.rs:47-65`'s forks in its favour. Leave `local.rs`'s four adversarial forks unless a no-op `adversarial` shim reads better than four visible forks. Acceptance: `grep -rn 'cfg(not(test))' src/tree/mirror/streaming` returns only `channel.rs` and `local.rs`.

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

Resolution: Rewrite `protocol.rs:1` as the wire-version vocabulary one release speaks (the enum's own doc at `:7-11` already says the right thing about frozen wire formats and new variants); re-word `error.rs:14` and `peer.rs:604` to "both ends must run releases speaking the same protocol version"; change the two `Display` strings to "we speak {local_protocol:?}"; drop "selected" at `handshake.rs:204`. Acceptance: `grep -rn -i 'select' src/protocol.rs src/error.rs src/peer.rs src/tree/mirror/handshake.rs` returns only `Peer::payload_depth_limit`'s "select the same" (a real configuration knob) or nothing.

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

### module-graph-6: Six inline `#[cfg(test)] mod tests { ... }` blocks breach the sibling `tests.rs` convention
- Where: src/tree/mirror/streaming/driver.rs:203-204 (related: src/testing.rs:396-397; src/testing/transport.rs:801-802; src/tree/mirror/streaming/backend/local/adversarial.rs:127-128; src/tree/mirror/streaming/testing/failing.rs:267-268; src/tree/arb.rs:672-673)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -rn -E '^\s*(pub(\([^)]*\))?\s+)?mod\s+tests?\s*\{' src`; `tools/` and `justfile` grepped for any placement check: none)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

AGENTS.md's testing section states the convention ("Unit and protocol tests live in a sibling file: `mod tests;` in the source, `tests.rs` next to it") and its reason (brevity of the implementation file). Six files carry a brace-bodied test module instead, one named `test` in the singular. Nothing mechanical enforces placement.

Evidence:

    src/tree/mirror/streaming/driver.rs:
       203	#[cfg(test)]
       204	mod tests {

    src/tree/arb.rs:
       672	#[cfg(test)]
       673	mod test {

    (the same two-line shape at testing.rs:396-397, testing/transport.rs:801-802,
     backend/local/adversarial.rs:127-128, streaming/testing/failing.rs:267-268)

Resolution: Move each body to a sibling file (`driver/tests.rs`, `testing/tests.rs`, `testing/transport/tests.rs`, `backend/local/adversarial/tests.rs`, `streaming/testing/failing/tests.rs`, `tree/arb/tests.rs`), leaving `#[cfg(test)] mod tests;`; rename `arb`'s `test` to `tests`. Pure moves. Acceptance: the grep above returns nothing.

### module-graph-7: `testing.rs` mixes a forwarding facade, a table renderer, and a deadlock detector, with imports mid-file
- Where: src/testing.rs:194-291 (related: src/testing.rs:1, :335-343, :345-394, :396-442; src/lib.rs:319-321)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (file read in full)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The `test-internals` facade holds three responsibilities: one-line forwarders to crate-internal meters (each naming the suite that consumes it), a 100-line markdown renderer that `examples/window_tradeoff.rs` prints and the window suite byte-compares, and the `run_to_quiescence` detector with its own two tests, with a `use std::{...}` block after 330 lines of items.

Evidence:

         1	//! Executor-agnostic test support shared across protocol and API suites.
    ...
       207	pub fn window_tradeoff_table() -> String {
    ...
       263	        "<!-- Generated by `just window-tradeoff`; do not edit. -->"
    ...
       335	use std::{
       336	    future::Future,
       337	    pin::pin,
    ...
       375	pub fn run_to_quiescence<F: Future>(future: F) -> Result<F::Output, Quiescence> {

Resolution: Split into `testing/quiescence.rs` (`Quiescence`, `WakeFlag`, `run_to_quiescence`, its two tests in a sibling `tests.rs`) and either `testing/tradeoff.rs` or `window_tradeoff_table` moved into `window.rs` under `#[cfg(any(test, feature = "test-internals"))]` beside `Window::from_budget`; keep `testing.rs` as the facade with `pub use` of both; hoist the imports. The module is `#[doc(hidden)]` and feature-gated, so no API moves. Acceptance: `testing.rs` contains only `mod`/`pub use` lines and forwarders; every `use` precedes every item.

### module-graph-8: Redundant re-export layers: a six-line shim over `streaming::channel`, and a re-export list that `remote.rs` glob-re-exports
- Where: src/tree/mirror/streaming/materialized/channel.rs:1-6 (related: src/tree/mirror/streaming/materialized.rs:173, :184; src/tree/mirror/streaming/materialized/common.rs:15; src/tree/mirror/streaming/materialized/work.rs:28-36; src/tree/mirror/streaming/materialized/work/queues.rs:30; src/tree/mirror/streaming/materialized/work/levels.rs:28; src/tree/mirror/streaming/materialized/work/assembly.rs:14; src/tree/mirror/streaming/remote.rs:85; src/tree/mirror/streaming/remote/error.rs:1-19; src/error.rs:44-50)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (files read in full; `grep -rn 'channel::' src/tree/mirror/streaming/materialized*` lists the six shim consumers; `git log --follow` shows the shim unchanged since `83edcd944 WIP: swap over to streaming`; `git blame` dates `remote.rs:85` to `cbfe1aff3`)
- Verification: confirmed; history: no-rationale-found (the cbor wire review's R3 ruling routes public error types by *named* re-export lists through `codec` → `remote` → `error.rs`, which the glob at `remote.rs:85` sits inside)
- Owner-gated: no

`materialized/channel.rs` exports nothing of its own: every name is `streaming::channel`'s, re-spelled so materialized's children write `super::channel::` while `work.rs:28-36` already imports `erased`, `protocol`, `tasks`, and `window` by crate path in the same block. `remote/error.rs` is `pub use` lines only, and `remote.rs` re-exports it with the crate's only glob re-export, so a codec error type reaches `crate::error` through three hops and `remote.rs`'s reader cannot see which names it exports.

Evidence:

    src/tree/mirror/streaming/materialized/channel.rs (whole file):
         1	//! Materialized protocol access to the shared named-channel infrastructure.
         2	
         3	pub use super::super::channel::{QueueKind, QueueRole, Receiver, Sender, channel};
         4	
         5	#[cfg(test)]
         6	pub use super::super::channel::{with_kind_capacity, with_observation, with_schedule};

    src/tree/mirror/streaming/materialized/work.rs:
        28	use crate::tree::{
        29	    mirror::streaming::{
        30	        Backend, Leaf, erased,
        31	        materialized::{Error, channel::Sender},
        32	        protocol::BoxResponses,

    src/tree/mirror/streaming/remote.rs:
        85	pub use error::*;

    src/tree/mirror/streaming/remote/error.rs:
        17	pub use super::proxy::Error as RemoteError;
        18	pub use super::streams::{AcceptError, ReplyFrameError, SendError, StreamError};
        19	pub use crate::tree::mirror::framing::LengthOverflow;

Resolution: Delete `materialized/channel.rs` and repoint its six consumers at `crate::tree::mirror::streaming::channel`; replace `remote.rs:85` with the explicit list (moving `remote/error.rs:1-6`'s doc onto it) or keep `error.rs` and re-export it by name. Acceptance: `grep -rn 'pub use .*\*;' src` returns nothing; `materialized.rs` declares no `channel` module.

### module-graph-9: `streaming` is the one path segment whose contrast retired with V1
- Where: src/tree/mirror.rs:3-5 (related: src/tree/mirror.rs:19-27; src/tree/mirror/streaming.rs:1; AGENTS.md:23)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified (`git ls-tree --name-only HEAD~60 src/tree/mirror/` lists `alternating.rs` beside `streaming.rs`; `368da2a5` deletes the alternating tower; `mirror.rs` and `streaming.rs` read in full)
- Verification: confirmed on the history, downgraded to nit: the name still describes the mechanism ("reconciliation over lazy node streams"), so what remains is the cost of a segment that no longer distinguishes anything at its level; history: no-rationale-found (the V1 plan never discusses the segment)
- Owner-gated: yes: a rename or fold touches every import and rustdoc path in the subtree, AGENTS.md, and the agent notes

`mirror.rs` now holds shared infrastructure (`cbor`, `framing`, `handshake`, `party`, `Error<C, S>`, `contained`) plus one protocol module, and its own doc calls `streaming` "the wire protocol". Every other segment of the deepest chain names a distinction a maintainer would miss; `streaming` alone names none at its level, and costs a path element in every import and link under it.

Evidence:

    src/tree/mirror.rs:
         3	//! [`streaming`] is the wire protocol; its behavioral oracle in this
         4	//! crate's tests is the in-memory merge (`Tree::join`), which routes
         5	//! deletion honoring through the same filter the mirror does.
    ...
        19	pub mod streaming;
    ...
        24	pub(crate) mod cbor;
        25	pub(crate) mod framing;
        26	pub(crate) mod handshake;
        27	pub(crate) mod party;

    src/tree/mirror/streaming.rs:
         1	//! The streaming mirror: fixed-memory reconciliation over lazy node streams.

Resolution: Owner call. Option A: fold `streaming`'s children up into `tree::mirror`, with `mirror.rs` adopting `streaming.rs:1-40`'s layer map. Option B: keep the level and re-word `mirror.rs:3` so the name reads as mechanism, not contrast. Acceptance: whichever is chosen, `mirror.rs`'s doc states the relationship in present-tense terms without implying a second protocol.

### module-graph-10: `pub(crate) use peer::Inner;` at the crate root, and seven inline `crate::Inner<T>` spellings
- Where: src/lib.rs:339 (related: src/rumors/causal.rs:67, :88; src/rumors/changes.rs:68; src/rumors/unordered.rs:64, :71, :84, :97; src/batch.rs:8; src/tests.rs:23; src/bookmark.rs:223)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn '\bInner\b' src`)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The crate root aliases `peer`'s private state type between the public re-exports, and `rumors/{causal,changes,unordered}.rs` then write the qualified `crate::Inner<T>` at seven sites instead of importing it.

Evidence:

    src/lib.rs:
       338	pub use network::Network;
       339	pub(crate) use peer::Inner;
       340	pub use peer::{

    src/rumors/unordered.rs:
        64	    Pin<Box<dyn Future<Output = (bool, watch::Receiver<crate::Inner<T>>)> + Send>>;

    src/batch.rs:
         8	use crate::{Inner, Version};

Resolution: Add `use crate::peer::Inner;` to `causal.rs`, `changes.rs`, and `unordered.rs` and drop the inline qualifications; optionally remove `lib.rs:339` and import `crate::peer::Inner` at `batch.rs:8`, `src/tests.rs:23`, and the `bookmark.rs:223` doc link. Acceptance: `grep -rn 'crate::Inner' src` returns nothing.

### module-graph-11: Redundant `#[cfg(test)]` gates under test-only parents, and a `pub(crate)` that widens nothing
- Where: src/conformance.rs:18-19 (related: src/conformance/backend.rs:790-791; src/tree/mirror/streaming/remote/proxy/work/progress.rs:58-59 with progress/trace.rs:196-197; src/tree/mirror/streaming/materialized.rs:176-177 with materialized/progress.rs:534-535; src/tree/mirror/streaming/backend/local.rs:27-28 with local/adversarial.rs:127-128; src/tree/mirror/streaming.rs:56-57 with testing/failing.rs:267-268; src/tree.rs:714-715 with arb.rs:672-673)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (each parent and child declaration read; `grep -rn 'conformance::backend' src tests` returns only the doc mention at `streaming/backend.rs:18`; `tools/` grepped for the pattern: nothing)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Six `#[cfg(test)]` attributes sit on `mod tests;`/`mod test {` declarations whose parent chain is already `#[cfg(test)]`, so they guard nothing; the parent gates (`progress.rs:58`, whose parent `proxy::work::progress` is production, among them) are the load-bearing ones. `conformance::backend` is `pub(crate)` but nothing outside its own `tests` names it. No tool relies on the inner gates, so if they are a deliberate uniform convention it is unrecorded.

Evidence:

    src/conformance.rs:
        18	#[cfg(test)]
        19	pub(crate) mod backend;

    src/conformance/backend.rs:
       790	#[cfg(test)]
       791	mod tests;

    src/tree/mirror/streaming/remote/proxy/work/progress.rs:
        58	#[cfg(test)]
        59	mod trace;
    src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:
       196	#[cfg(test)]
       197	mod tests;

Resolution: Either delete the six inner gates or record once, in AGENTS.md's testing section, that every `mod tests;` carries `#[cfg(test)]` regardless of context. Narrow `conformance.rs:19` to `mod backend;`. Acceptance: a reader can tell from a declaration whether its gate is load-bearing.

### module-graph-12: `pub` versus `pub(crate)` on module declarations under the private `tree` encodes no visibility difference
- Where: src/tree/mirror/streaming.rs:45-58 (related: src/lib.rs:322; src/tree/mirror.rs:19; src/tree/typed.rs:13-17, :22; src/tree/typed/untyped.rs:10)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (`grep -rn -E '^pub mod ' src/tree`: ten sites; no `[lints]` table or `unreachable_pub` anywhere in the workspace)
- Verification: reframed: the facts hold, but `unreachable_pub` fires on every `pub` item not reachable from the crate root, not only on `pub mod` declarations, so adopting it is a crate-wide sweep whose diagnostic volume the sweep underestimated ("the ten `pub mod` sites ... are the expected hits" is wrong); history: no-rationale-found
- Owner-gated: no, though the lint's volume makes it a deliberate campaign rather than a one-line change

`mod tree;` is private, so `pub mod materialized`/`remote`/`stats` and `pub mod hash`/`height`/`node`/`path`/`prefix`/`fan` are reachable exactly as far as their `pub(crate)` siblings. The mixed spelling reads as an API boundary that is not there.

Evidence:

    src/lib.rs:
       322	mod tree;

    src/tree/mirror/streaming.rs:
        47	pub(crate) mod convert;
        48	mod driver;
        49	mod erased;
        50	pub mod materialized;
        51	pub(crate) mod message;
        52	mod protocol;
        53	pub mod remote;
        54	pub mod stats;

Resolution: Either normalize the ten `pub mod` sites to `pub(crate)` by hand (cheap, scoped), or adopt `#![warn(unreachable_pub)]` at `lib.rs:295-302` and follow every diagnostic (thorough, large). Acceptance: `grep -rn -E '^pub mod ' src/tree` returns nothing, or the lint is on and clean.

### module-graph-13: Two near-identical `pump` helpers diverge on a closed receiver without saying why
- Where: src/tree/mirror/streaming/materialized/work.rs:134-153 (related: src/tree/mirror/streaming/remote/proxy/work.rs:272-282; src/tree/mirror/streaming/remote/proxy.rs:12-16; src/tree/mirror/streaming/tasks.rs:46-51)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (both functions and `send_or_cancel` read; whether the divergence is intended is not stated anywhere I read)
- Verification: confirmed as a question; history: no-rationale-found
- Owner-gated: no

Both pumps forward erased replies into a `Sender` and park after a published error; on a closed receiver the walk's returns `Ok(())` while the proxy's parks through `send_or_cancel`. The walk's pump also carries a `#[cfg(test)]` transcript parameter, so one shared helper would need to accommodate that.

Evidence:

    src/tree/mirror/streaming/materialized/work.rs:
       146	        let failed = item.is_err();
       147	        if send.send(item).await.is_err() {
       148	            return Ok(());
       149	        }
       150	        park_after_published_error(failed).await;

    src/tree/mirror/streaming/remote/proxy/work.rs:
       276	    while let Some(message) = messages.next().await {
       277	        let failed = message.is_err();
       278	        send_or_cancel(&send, message).await;
       279	        park_after_published_error(failed).await;
       280	    }

    src/tree/mirror/streaming/remote/proxy.rs:
        12	async fn send_or_cancel<T>(sender: &Sender<T>, value: T) {
        13	    if sender.send(value).await.is_err() {
        14	        cancelled().await;

Resolution: Either lift one `pump` into `tasks.rs` parameterized by the closed-receiver policy, or add a one-line comment at each site naming why the walk returns where the proxy parks. Acceptance: a reader of either site learns whether the fork is a decision.

### module-graph-14: Inline bodied production modules in implementation files
- Where: src/tree.rs:628-712 (related: src/tree/mirror/streaming/erased.rs:199-335; src/tree/typed/untyped.rs:50-76; src/tree/mirror/streaming/remote/adapter/decode.rs:562-598; src/conformance/backend.rs:88-130; exempt: src/tree/typed/height.rs:168-173)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn -E '^\s*(pub(\([^)]*\))?\s+)?mod\s+[a-z_]+\s*\{' src`, then a brace-matching script for each span)
- Verification: confirmed and extended: the sweep's three sites plus `untyped::census` (27 lines) and `decode::fan_probe` (37 lines); `height::sealed` (6 lines) is the sealed-trait idiom and stays; history: no-rationale-found
- Owner-gated: no

`tree.rs` carries two `#[cfg(test)]` module bodies (`meter`, 22 lines; `panic_injection`, 44 lines; about 85 lines with their docs) that are test hooks the implementation calls, not tests; `erased.rs`'s `ops` is a 137-line inline production module; `census`, `fan_probe`, and `ledger` are smaller inline modules of the same kind. Sibling files would leave each implementation file to its implementation, the motive AGENTS.md gives for sibling `tests.rs`.

Evidence:

    src/tree.rs:
       628	#[cfg(test)]
       629	pub(crate) mod meter {
    ...
       668	#[cfg(test)]
       669	pub(crate) mod panic_injection {

    src/tree/mirror/streaming/erased.rs:
       199	pub(crate) mod ops {

    src/tree/typed/untyped.rs:
        50	pub(crate) mod census {

    src/tree/mirror/streaming/remote/adapter/decode.rs:
       562	pub(super) mod fan_probe {

    src/conformance/backend.rs:
        88	mod ledger {

Resolution: Move to `tree/meter.rs`, `tree/panic_injection.rs` (declared `#[cfg(test)] pub(crate) mod ...;`), `erased/ops.rs`, `typed/untyped/census.rs`, `adapter/decode/fan_probe.rs`, and `conformance/backend/ledger.rs`. Pure moves. Acceptance: the grep above returns only `height.rs:168` and the sibling-file test modules of module-graph-6 once moved.

### module-graph-15: Twenty-seven non-test files open without a `//!` module doc
- Where: src/tree/mirror/streaming/materialized/common.rs:1 (related: src/rumors.rs:1; src/rumors/causal.rs:1; src/rumors/changes.rs:1; src/rumors/unordered.rs:1; src/tree/mirror/streaming/protocol/peer.rs:1; src/tree/mirror/streaming/remote/adapter/decode.rs:1; src/tree/mirror/streaming/remote/adapter/encode.rs:1; src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:1; src/tree/typed/node.rs:1; src/tree/typed/untyped.rs:1; src/tree/mirror/streaming/backend/local.rs:1)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (shell loop over every non-test `.rs` under `src/`: 27 files, the same count the sweep reports)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

A maintainer orienting by module docs finds none in these files. For single-type files (`batch.rs`, `snapshot.rs`, `message.rs`, `materialized/error.rs`) the type's own doc carries the load; for multi-item modules (`rumors.rs`, `adapter/decode.rs`, `adapter/encode.rs`, `typed/node.rs`, `typed/untyped.rs`, `protocol/peer.rs`, `progress/trace.rs`, `common.rs`, `backend/local.rs`) a one-line `//!` is the missing map that `streaming.rs:10-24` shows the crate already values.

Evidence:

    src/tree/mirror/streaming/materialized/common.rs:
         1	use std::pin::pin;
    src/rumors.rs:
         1	mod causal;
    src/tree/mirror/streaming/protocol/peer.rs:
         1	use crate::tree::{
    src/tree/mirror/streaming/remote/adapter/decode.rs:
         1	use crate::message::PayloadCodec;

Resolution: Add a one-sentence `//!` to the multi-item files; leave the single-type files. Acceptance: the shell loop lists only files whose first item is the file's one public type.

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

## Positives

- Every `.rs` file under `src/` is declared exactly once (183 files, 182 `mod` declarations plus the root); no orphan files, no duplicate declarations. (The sweep's count from its `mod_decls.txt`; I did not recount it.)
- The top-level layering reads cleanly once the two single-site upward edges of module-graph-1 are set aside: `message`/`tags`/`network`/`protocol` at the bottom, `link` and `observe` above, `tree` above those, `batch`/`bookmark`/`snapshot`, then `peer`/`rumors`/`error`.
- `streaming.rs:1-40` is a real orientation map: each child module is named with the one reason it is separate, and it says where to start reading.
- `handshake.rs:170-198` defines a local `pub(crate) enum Error` that `error.rs:292-314` lifts by an exhaustive `From` impl: the right layering shape and the template for module-graph-1.
- `define_peer!` (`protocol/peer.rs:12-106`) plus the type-level height descent in `protocol.rs:163-203` make the exchange schedule a compile-time fact: a wrong round count cannot build. (This is what lets module-graph-5 drop the sweep's proposed const-assert.)
- The cargo features each gate something real: `conformance` gates `pub mod conformance` and the link suite; `test-internals` gates `testing`, the capture renderer, the node census, and the window introspection; `meter` gates exactly the metering test modules. Every gate's comment in `Cargo.toml:108-122` says who lights it and why.
- `testing.rs`'s forwarders each name the integration suite that consumes them (`tests/decode_alloc.rs`, `tests/encode_alloc.rs`, `tests/dispute_wire.rs`), so the `test-internals` boundary documents itself even where its contents should split (module-graph-7).
- `tree::mirror::cbor` states why it is hand-written and exactly what it owns, and `bookmark::format` reuses that boundary without pulling in the wire protocol.
- `budget.rs:64-71` derives `DEFAULT_TARGET_MESSAGE_SIZE` from the wire constants rather than measuring it, and says so; `window.rs:139-156` prices slots by `size_of` the real types for the same reason. Both are the "no hand-maintained number" rule applied to memory pricing.
- The V1 retirement is recorded end to end: the plan note carries the three rulings, `368da2a5` names everything deleted and simplified, and `c13c21b4` names what it re-denominated and what it deliberately left. The residue in module-graph-4 is small against that.

## Open questions for Finch

1. `window ↔ materialized`: `window.rs:154-156` prices `REFERENCE_SLOT_BYTES` by `size_of::<(u8, Resolve<...>)>` (the sweep attributed this to `FAN_SLOT_BYTES`, which uses only `typed` types). Is that pricing edge intended as the one deliberate mutual dependence in the streaming core? If so, a sentence at `window.rs:123` closes the question, and module-graph-2's other moves can proceed independently.
2. Is the closed-receiver divergence between the walk's `pump` (returns `Ok(())`) and the proxy's `pump` (parks via `send_or_cancel`) a decision? (module-graph-13)
3. `streaming` as a path segment: fold into `tree::mirror`, rename to what it now distinguishes, or keep and re-word `mirror.rs:3`? (module-graph-9)
4. `adapter/decode.rs:83` and `:288` build their reader/assembler fan edges with raw `tokio::sync::mpsc::channel(FAN)` and a private `cfg(test)` `fan_probe`, outside `streaming::channel`'s named-queue instrumentation. Deliberate (a transient edge inside one decode call, not a protocol edge the schedule tests perturb), or an edge the `with_schedule` machinery should also reach? I did not pursue this beyond noting it; it is a test-coverage question more than a graph one.
5. `unreachable_pub`: worth the crate-wide diagnostic sweep it implies, or is normalizing the ten `pub mod` sites by hand the right size? (module-graph-12)

## Dropped

- Sweep [1] resolution (c), "move `DEFAULT_TARGET_MESSAGE_SIZE` to `message.rs`": the constant is `FAN * FULL_FAN_QUERY_FRAME_LEN`, derived from codec frame constants at `budget.rs:52-71`; moving it would invert a `codec → message` edge. Replaced by the thread-through or document options in module-graph-2.
- Sweep [1] resolution (b) as written, "move `SupplyLedger` whole": `absorb` (`materialized.rs:245-250`) returns `materialized::Error` and constructs `Violation::OverdrawnSupply`; only `new`/`charge` can move, with `absorb` staying as an inherent impl in `materialized`. Reframed in module-graph-2.
- Sweep [3]'s option to relocate `Protocol` into `tree::mirror::handshake`: the V1 plan ruled the enum stays public as wire vocabulary, and moving its definition behind the same root re-export changes nothing a reader sees. Kept only the prose fix (module-graph-4).
- Sweep [4]'s "derive both sites from one constant or add a `const _: () = assert!(...)`": the type-level height descent (`protocol.rs:163-168`, `:198`) already makes a mismatched round count a compile error. Kept only the location and layout corrections (module-graph-5).
- Sweep [11]'s "the ten `pub mod` sites under src/tree are the expected hits" for `unreachable_pub`: the lint fires on every `pub` item unreachable from the crate root, so the hit count is far larger. Reframed in module-graph-12.
- Sweep open question's attribution of the `Resolve` pricing edge to `FAN_SLOT_BYTES` (`window.rs:139-142`): it is `REFERENCE_SLOT_BYTES` at `:154-156`. Corrected in open question 1.
- A suspected ghost link `[`queues`]` at `remote/proxy/work/pump.rs:25`: `proxy/work.rs:41` declares `mod queues;`, so the link resolves. Not a finding.
