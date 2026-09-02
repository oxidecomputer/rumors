# Partition remote-proxy: The remote proxy: start, state, work (encode, progress trace, pump, queues)

## Partition summary

The remote proxy is the wire-bound participant in a streaming mirror session. `start.rs` runs the greeting exchange over a `Link`'s control halves (stamping the local codec's payload-depth limit into the outgoing greeting, requiring equality with the peer's before the equal-versions short-circuit), resolves the window and run budget from both greetings, elects roles, and allocates the claim table, error route, and accept driver. `state.rs` is the typestate chain `Connected` -> `Descending<H>` -> `Completing`, binding one incoming logical stream (a lazily claimed receiver) and one outgoing stream (a lazily connected sender) per stage and threading a one-slot `Scope` queue between stages. `work.rs`, `pump.rs`, and `encode.rs` turn each stage into two independently runnable tasks: an encoder that pairs a dequeued scope with the local walk's reply, flushes the whole wire reply, and only then publishes the derived questions; and a decoder that pairs each flushed question with the peer's reply, yields the reply into a one-slot relay, and only then publishes the derived scopes. The one irregular stage is the initiator's first descent, where an `Early` cursor lazily claims the opening-supply stream and splices each early-shipped root child, exploded to its children, into the responder's empty pairing reply. `Work::execute` drives everything under a biased `select!` whose post-select attribution makes a dead stream supply outrank its own symptoms while exempting typed backend errors. `queues.rs` names the two window-sized scope edges; `progress/` is a trace of the proxy's ordering-critical publications, a ZST outside `cfg(test)`.

I read all eleven partition files, 2504 lines. Test code: `work/progress/trace/tests.rs` (124 lines) and `work/progress/trace.rs` (197 lines, compiled only under `cfg(test)`); the sibling `tests.rs` files (`proxy/tests.rs`, `start/tests.rs`, `work/tests.rs`) are outside the partition list, and I read the portions the findings depend on. I also read the cross-partition sites the candidates rest on: the driver's election and dispatch (`streaming.rs`, `driver.rs`), the protocol traits, `streams.rs`'s error enums and `AcceptDriver::run`, the codec's placement grammar (`signal.rs`), the adapter's decode entries, the gossip-layer error lift, `window.rs`, and the walk's counterparts.

The production code holds up under every lens: I found no correctness defect. Ordering is enforced structurally (`Encoded::write_with` releases a question only after its frame flushes; `yield_reply_scopes!` binds the yield and the scope publication in one expansion, with the reason stated at the macro); every panic site is discharged by the schedule rather than by input shape; the attribution table in `execute` has a committed witness for every arm; and completeness is checked in both directions at every stage boundary. The two medium-severity structural findings are in the handshake-to-session hand-off, which does more than it needs to: it duplicates the post-exchange tail between `complete_connect` and `accept`, threads the same session facts through four positional argument lists, and elects the initiator a second time, independently of the driver, then reconciles the two elections with an enum, two `unreachable!`s, and four `debug_assert_eq!`s. The one medium verification finding is that the ordering trace has no liveness floor and passes vacuously on an empty trace. The rest is maintenance debt of the duplication kind (the same rationale restated at several sites, three near-identical decode loops, a ten-site erase-and-box expression under two aliases) plus public-rustdoc inaccuracies on `RemoteError` and a handful of register and naming nits. Nothing vestigial from the V1 or BLAKE3 removals survives here; one arm (`TerminalQuery` on the decode side) was born unreachable from wire bytes.

## Findings

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

### remote-proxy-2: PayloadDepthMismatch is publicly exported in RemoteError but never reaches a user there
- Where: src/tree/mirror/streaming/remote/proxy/error.rs:31-41 (related: src/peer/gossip.rs:1419-1431, src/error.rs:105-122, src/tree/mirror/streaming/remote/proxy/start.rs:258-268, src/lib.rs:322)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep: raised only at start.rs:262; the only production constructor of `Error::Mirror` is gossip.rs:1430, and gossip.rs:1423-1429 lifts `Server(PayloadDepthMismatch)` first; `mod tree` is private at lib.rs:322 so no caller can drive `Handshaking` directly; tests/payload_depth.rs:292-299 asserts the lifted top-level variant)
- Seen by: perfapi; refutation: confirmed; history: deliberate-and-holds for the lift (71de90c11 chose the top-level variant; gossip.rs:1419-1422 states it inline), but nothing at the variant says so
- Owner-gated: yes: moving the check out of `proxy::Error` changes the public taxonomy; and the doc-only fix re-grows the exact paragraph the owner cut in 9085bd10 (PR #38)

A user matching `Error::Mirror(Server(RemoteError::PayloadDepthMismatch { .. }))` has written a dead arm, and nothing on the variant says so; the fact is documented only at the lift site in gossip.rs. Principle: public rustdoc names nothing the API does not reach; the design is deliberate but the rationale lives away from the surface a user reads.

Evidence:

    31	    /// The peer's configured payload depth limit differs from ours.
    32	    ///
    33	    /// Detected symmetrically, after the greetings and before anything
    34	    /// else, so a mixed fleet is caught even on a converged session.
    35	    #[error("peer's payload depth limit ({remote}) differs from ours ({local})")]
    36	    PayloadDepthMismatch {

    gossip.rs:1419	    // The depth-limit mismatch is a configuration diagnosis, not a
    gossip.rs:1420	    // reconciliation failure: surface it as its own top-level variant.
    gossip.rs:1421	    // Only the proxy (the server side of every production handshake)
    gossip.rs:1422	    // detects it; the materialized participant has no wire.

Resolution: either (a) replace the second sentence of the variant doc with one that states the lift ("Surfaces to users as [`crate::Error::PayloadDepthMismatch`], never under [`Error::Mirror`]."), keeping the doc at two lines; or (b) move the check to a handshake-level result the driver maps, so the proxy enum stops carrying a configuration diagnosis and `streaming_error`'s special case disappears. Acceptance: the variant doc states the lift in one sentence, or the variant is gone from `RemoteError` and gossip.rs:1423-1429 is deleted.

### remote-proxy-3: The public error taxonomy does not say which variants diagnose the local participant and which the peer; TerminalQuery is raised for both
- Where: src/tree/mirror/streaming/remote/proxy/error.rs:63-80 (related: src/tree/mirror/streaming/remote/proxy/error.rs:6, src/tree/mirror/streaming/remote/proxy/error.rs:42-44, src/tree/mirror/streaming/remote/proxy/error.rs:51-53, src/tree/mirror/streaming/remote/proxy/work/encode.rs:64-68, src/tree/mirror/streaming/remote/proxy/work/pump.rs:393-395, src/tree/mirror/streaming/materialized/error.rs:10-12, src/tree/mirror/streaming/remote.rs:69)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep of construction sites: `MissingOpening`, `ExtraOpening`, `UnaskedLocalReply`, `UnansweredRemoteQuery`, `OpeningEncode` are raised only in encode.rs, on the local reply path; `TerminalQuery` is raised at encode.rs:67 (local) and pump.rs:394 (remote); `mod adapter` is private at remote.rs:69)
- Seen by: prose, perfapi; refutation: confirmed both; history: no-rationale-found (the dual use dates from cbfe1aff3; the owner's PR #38 directive on this enum constrains added prose to be terse)
- Owner-gated: yes: nesting or splitting variants changes the public taxonomy; the doc-only route does not

The enum doc names a private module (`adapter`) and internal vocabulary ("distinguished opening", "reply-only boundary"), and nothing classifies the variants by origin. Six variants can only fire if this crate's own protocol or adapter misbehaves; others are peer conformance faults, configuration, or transport. `TerminalQuery` is raised on both a local encode path and a remote decode path, so a reader of `Error::Mirror(Server(TerminalQuery))` cannot tell whom to file against. The walk's `Violation` states its scope up front ("The ways a counterparty can misbehave"). Principle: documentation altitude: public rustdoc answers which-one-do-I-care-about and names nothing the API does not reach; AGENTS.md frames this machinery as a conformance-bug detector, and the first triage question is whose bug it caught. Note that remote-proxy-29 shows the decode-side `TerminalQuery` site is unreachable from wire bytes, so after that lands the variant has one (local) producer and the split becomes a doc change.

Evidence:

    6	/// A protocol or adapter failure while proxying one remote counterparty.
    42	    /// The locally-produced distinguished opening could not be encoded.
    51	    /// A frame constructed by the adapter violated the reply-only boundary.
    63	    /// The local opening stream omitted its distinguished question.
    72	    /// The local protocol produced a reply which answered no remote query.
    78	    /// The terminal responder attempted to ask another leaf question.
    79	    #[error("terminal responder reply contained another query")]
    80	    TerminalQuery,

    encode.rs:66	        } else if !batch.is_empty() {
    encode.rs:67	            return Err(Error::TerminalQuery);
    pump.rs:393	                if !questions.is_empty() {
    pump.rs:394	                    Err(Error::TerminalQuery)?;

Resolution: minimum: add one short sentence to the enum doc naming the two origins ("Variants that name the local protocol diagnose this crate's own participant; the rest diagnose the peer or the transport."), reword the three summaries that name `adapter`/"distinguished"/"reply-only boundary" in user vocabulary, and after remote-proxy-29 make `TerminalQuery`'s doc name the local producer. Cleaner (owner call): nest the local-only variants under one `LocalProtocol` variant so the split is structural. Acceptance: no private module name or internal coinage in error.rs docs; the enum doc states the origin split; `grep -rn 'Error::TerminalQuery' src` shows one producer side.

### remote-proxy-4: complete_connect and accept duplicate the post-exchange tail, then thread it through four positional argument lists
- Where: src/tree/mirror/streaming/remote/proxy/start.rs:168-196 (related: src/tree/mirror/streaming/remote/proxy/start.rs:210-245, src/tree/mirror/streaming/remote/proxy/start.rs:338-350, src/tree/mirror/streaming/remote/proxy/start.rs:401-414, src/tree/mirror/streaming/remote/proxy/state.rs:41-44, src/tree/mirror/streaming/remote/proxy/state.rs:119-130, src/tree/mirror/streaming/remote/proxy/work.rs:101-112, src/tree/mirror/streaming/remote/proxy/work/encode.rs:77-79)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read)
- Seen by: structure, prose (duplicated depth-rationale comment), perfapi (arity and the `budget` copy); refutation: confirmed; history: no-rationale-found for the duplicated tail (each branch was edited in lockstep by aabdcea07, bdf74d45, 71de90c11); deliberate-and-holds for the arity ("one premise per argument", 48bc31df9, inline at start.rs:338-339 and work.rs:101-102)
- Owner-gated: no for the shared helper; the arity bundling contests an inline rationale and is listed under open questions

After the greetings cross, both trait impls stamp the codec limit, call `payload_depth_limits_match` under the same three-line comment, call `window.resolve` with the same five arguments, call `run_budget`, and pass nine positional arguments to `connected`, which passes eleven to `open`, which passes eight to `Work::new` and nine to `Connected::new`. The same session facts cross four argument lists under four `#[allow(clippy::too_many_arguments)]`, two of them (start.rs:401, state.rs:119) with no rationale and the other two with a comment that wraps from an attribute trailer onto a standalone line. `Session` additionally holds a `budget` copy whose only doc explains that the other copy lives in `Work`, which `Session` owns. Principle: one site per decision on a correctness-relevant path (the depth check and window resolution); duplicated logic edited in one branch and not the other is the concrete hazard.

Evidence:

    173	        // Payload depth limits must be equal — checked after both
    174	        // greetings are in hand and before the equal-versions resolution,
    175	        // so a mixed configuration is caught even on a converged session.
    176	        payload_depth_limits_match::<B::Error>(&self.codec, &self.versions.remote)?;
    177	        let window = self.window.resolve(
    178	            theirs.set_len,
    179	            self.versions.remote.set_len,
    180	            theirs.max_version_bytes,
    181	            self.versions.remote.max_version_bytes,
    182	            B::node_bytes,
    183	        );
    184	        let budget = run_budget(&theirs, &self.versions.remote);

    220	        // Payload depth limits must be equal — checked after both
    221	        // greetings are in hand and before the equal-versions resolution,
    222	        // so a mixed configuration is caught even on a converged session.
    223	        payload_depth_limits_match::<B::Error>(&self.codec, &remote)?;
    224	        let greeting = remote.clone();
    225	        let window = self.window.resolve(

    338	#[allow(clippy::too_many_arguments)] // The argument list is the handshake's
    339	// dataflow into the elected session, one premise per argument.
    401	#[allow(clippy::too_many_arguments)]

Resolution: add `fn connected(self, local: Greeting, remote: Greeting) -> Result<Connected<B, R, W, C, A>, Error<B::Error>>` on `impl<B, R, W, C, A, V> Handshaking<B, R, W, C, A, V>` (the free function never reads `versions`) holding the depth check, `window.resolve`, `run_budget`, and the hand-off; both impls reduce to their exchange plus `self.connected(ours, remote)`. Let the one remaining call-site comment shrink to the placement note ("before the equal-versions shortcut") and leave the full argument on `payload_depth_limits_match`. Replace `Connected::new`'s positional list with a struct literal. If remote-proxy-7 lands, `open` becomes the body of `initiator()`/`responder()` and the chain collapses further; whether the remaining premises get a named bundle is the open question below. Acceptance: one `window.resolve` and one `payload_depth_limits_match` call in start.rs; every surviving `too_many_arguments` allow carries a one-line rationale above the attribute; `start/tests.rs` and `proxy/tests.rs` pass unchanged.

### remote-proxy-5: Em-dashes in `//` comments at four production sites
- Where: src/tree/mirror/streaming/remote/proxy/start.rs:173-175 (related: src/tree/mirror/streaming/remote/proxy/start.rs:220, src/tree/mirror/streaming/remote/proxy/work.rs:250, src/tree/mirror/streaming/remote/proxy/work/pump.rs:195)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn -E '^\s*//[^/!].*—'` over the partition excluding tests matches exactly these four; the same grep over src excluding tests matches 73 lines, so this is crate practice)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the dash rule lives in the owner's global instructions, not in-tree; two of the four postdate it)
- Owner-gated: no

Doctrine prefers colons, semicolons, or spaced double-hyphens in `//` comments; four comments here use a true em-dash. No error, expect, or assert string in the partition contains one. Fixing only this partition's four leaves the practice in place crate-wide, so this is a sweep item.

Evidence:

    173	        // Payload depth limits must be equal — checked after both
    220	        // Payload depth limits must be equal — checked after both
    work.rs:250	            // supply cannot have caused it, so it is never outranked — it
    pump.rs:195	                    // the early stream carries the node — or neither does,

Resolution: replace with `--`, a colon, or a semicolon (start.rs:173 and 220 collapse to one site under remote-proxy-4); consider one crate-wide sweep rather than a per-partition fix. Acceptance: the grep above returns nothing for the partition.

### remote-proxy-6: The proxy's handshake impls switch from the trait's frame (`theirs`, `request`) to the process frame (`local`, `remote`) without saying so
- Where: src/tree/mirror/streaming/remote/proxy/start.rs:185-195 (related: src/tree/mirror/streaming/remote/proxy/start.rs:168-172, src/tree/mirror/streaming/remote/proxy/start.rs:233-243, src/tree/mirror/streaming/protocol.rs:1-8, src/tree/mirror/streaming/protocol.rs:85)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read)
- Seen by: prose; refutation: reframed (the trait names are correct in the proxied peer's frame; the cost is the silent frame switch); history: no-rationale-found (0aa29ed94 adopted the trait's name when the greeting gained its listing)
- Owner-gated: no

In `complete_connect`, `theirs` is the local walk's greeting (the code stamps the local codec's limit into it and sends it), and it is passed to `connected` as the `local` argument; `accept`'s `request` likewise. The trait speaks from the participant's own frame and the proxy stands in for the remote peer, so the parameter names are right in that frame, but the body then switches to process-frame names (`local`, `remote`) with nothing marking the switch, and a reader following `theirs` into `connected(local, remote)` has to invert it.

Evidence:

    168	    async fn complete_connect(mut self, mut theirs: Greeting) -> Result<Self::Next, Self::Error> {
    185	        Ok(connected(
    186	            self.backend,
    187	            window,
    188	            budget,
    189	            theirs,
    190	            self.versions.remote,

Resolution: either rename the parameters `local: Greeting` (trait parameter names are not part of the signature) with one doc sentence saying the greeting is the local participant's, forwarded to the wire; or keep the trait names and add a one-line comment at the `connected(...)` call stating the frame switch. Acceptance: a reader of start.rs can tell which side each greeting belongs to without consulting protocol.rs.

### remote-proxy-7: The proxy elects the initiator a second time and reconciles it with the driver's election by enum, unreachable!, and debug_assert
- Where: src/tree/mirror/streaming/remote/proxy/start.rs:356-376 (related: src/tree/mirror/streaming/remote/proxy/state.rs:93-110, src/tree/mirror/streaming/remote/proxy/state.rs:118-159, src/tree/mirror/streaming/remote/proxy/state.rs:237-252, src/tree/mirror/streaming/remote/proxy/state.rs:273, src/tree/mirror/streaming/remote/proxy/state.rs:310, src/tree/mirror/streaming/remote/proxy/state.rs:375-378, src/tree/mirror/streaming/remote/proxy/state.rs:406, src/tree/mirror/streaming/remote/proxy/state.rs:431, src/tree/mirror/streaming.rs:197-216, src/tree/mirror/streaming/driver.rs:161-171, src/tree/mirror/streaming/materialized.rs:342-356, src/observe.rs:83-92)
- Class / severity / confidence: simplification / medium / medium
- Provenance: verified (read both election sites and the dispatch: `descend` computes `message::initiates` once and calls `mirror_connected(local, remote)` or `mirror_connected(remote, local)`, whose `mirror!` block calls `initiator()` on the first argument and `responder()` on the second, so which method the driver calls on the proxy already encodes the election; grep shows `Connected::new`/`Connected::equal` have no callers outside start.rs)
- Seen by: structure; refutation: confirmed, with feasibility checked (the `elected` observer contract needs only "after the greetings, before any data stream opens", and streams open lazily inside `execute`); history: no-rationale-found (the enum and both `unreachable!`s arrived in 83edcd944, "WIP: swap over to streaming; DEADLOCK STILL PRESENT", with no message body; the four `debug_assert_eq!`s predate it)
- Owner-gated: no

`connected()` decides equality and the initiator role itself, allocates the whole session on the spot, and stores the result behind `ConnectedState::{Equal, Diverged}`. The driver makes the same two decisions from the same greetings and dispatches `complete_equal`/`initiator`/`responder`; the proxy then checks that the two agree with two `unreachable!`s and four `debug_assert_eq!`s on `session.remote`. The `Connected` doc even says the role is not yet known while the state already holds `remote: Speaker`. The materialized `Connected` is a plain struct that defers everything to the driver's call, which is the shape the protocol traits invite: `Accept::Next` and `CompleteConnect::Next` must implement all three of `CompleteEqual + Initiator + Responder`, so the participant need not know which will be called. Principle: circular justification is the tell; recompute-and-compare asserts on a deterministic function are not defense in depth once one election is the source of truth. The `debug_assert!(self.early.is_none())` at state.rs:375-378 is the same pattern one level down (every constructor below the first stage fixes `early` to `None`), and `Connected::new`/`equal` are `pub` for two callers in a private module.

Evidence:

    356	    if local.version == remote.version {
    357	        return Connected::equal(link.control_read, link.control_write);
    358	    }
    359	    // The role election of record: the smaller exchanged set initiates,
    360	    // canonical version bytes break ties (`message::initiates`).
    361	    let local = if initiates(
    362	        local.set_len,
    363	        &local.version,
    364	        remote.set_len,
    365	        &remote.version,
    366	    ) {

    state.rs:93	/// A proxy after the version exchange but before its elected role is known.
    state.rs:157	            ConnectedState::Equal(..) => unreachable!("descent opened for equal versions"),
    state.rs:249	            ConnectedState::Diverged(..) => unreachable!("equal completion for divergent versions"),
    state.rs:273	        debug_assert_eq!(session.remote, Speaker::Initiator);

    streaming.rs:208	    if message::initiates(local_len, &local_version, remote_len, &remote_version) {
    streaming.rs:209	        mirror_connected(local, remote).await
    driver.rs:161	    mirror! {
    driver.rs:162	        i.initiator;
    driver.rs:163	        r.responder;

Resolution: make `Connected` a struct holding the `Link`, backend, resolved `Window`/`RunBudget`, the remote greeting's `max_version_bytes`/`set_len`/`listing`, stats, codec, and observe handle. `complete_equal` destructures the link and returns the control halves. `Initiator::initiator` and `Responder::responder` fire `observe.elected(...)` and call `open` with `Speaker::Initiator`/`Speaker::Responder` respectively (the remote is the initiator exactly when the driver calls `initiator()` on the proxy), building `Session` there; equal sessions stay allocation-free because `open` runs only on the role paths. Delete `ConnectedState`, `Connected::new`, `Connected::equal`, `diverged()`, both `unreachable!`s, and the four `debug_assert_eq!`s. Acceptance: start.rs contains no call to `initiates` and no version comparison; state.rs contains no `unreachable!` and no `debug_assert_eq!` on `Speaker`; `just gate` clean; the proxy suites pass unchanged, including `equal_versions_return_both_roots` and `tests/observe.rs`'s election assertions.

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

### remote-proxy-9: `stream_at`'s expect message states the conclusion, not why the schedule cannot miss
- Where: src/tree/mirror/streaming/remote/proxy/state.rs:88-91 (related: src/tree/mirror/streaming/remote/codec/signal.rs:55-72)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read; `Stream::at_height` returns `None` when the height's parity does not match the speaker's phase)
- Seen by: structure (as an open question); refutation: not examined; history: not examined
- Owner-gated: no

`Stream::at_height` returns `None` for a (speaker, height) pair on the wrong parity. The `expect` fires only if the typestate chain names such a pair, which it cannot: the remote speaks at `S<H>` and the local at `S<S<H>>` in every `reply`, alternating parity by phase, and the correctness lens checked all seventeen pairs for both roles. The message asserts the conclusion rather than that argument. Doctrine: every `expect` message is a one-line proof of why it cannot fire.

Evidence:

    88	/// Find the logical stream assigned to one speaker and reply height.
    89	fn stream_at<H: Height>(speaker: Speaker) -> Stream {
    90	    Stream::at_height(speaker, H::HEIGHT).expect("every protocol reply height has one stream")
    91	}

Resolution: `.expect("the typestate binds each speaker only at heights on its own parity")` or similar. Acceptance: the message names the parity argument.

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

### remote-proxy-11: Idiom nits: a stray import group, qualified paths beside existing imports, a redundant `pin!`, and parameter rebinding
- Where: src/tree/mirror/streaming/remote/proxy/work.rs:8-9 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:27-28, src/tree/mirror/streaming/remote/proxy/start.rs:3-5, src/tree/mirror/streaming/remote/proxy/work/pump.rs:38, src/tree/mirror/streaming/remote/proxy/work/pump.rs:178, src/tree/mirror/streaming/remote/proxy/work/pump.rs:280, src/tree/mirror/streaming/remote/proxy/work/pump.rs:436, src/tree/mirror/streaming/remote/proxy/work/pump.rs:532, src/tree/mirror/streaming/remote/proxy/work/encode.rs:14, src/tree/mirror/streaming/remote/proxy/work/encode.rs:178, src/tree/mirror/streaming/remote/proxy/work.rs:212, src/tree/mirror/streaming/remote/proxy/work/encode.rs:54, src/tree/mirror/streaming/remote/proxy/work/encode.rs:94, src/tree/mirror/streaming/remote/proxy/work/encode.rs:138, src/tree/mirror/streaming/remote/proxy/work/pump.rs:223, src/tree/mirror/streaming/materialized/work.rs:121-124)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read each site; `grep -rn 'pin!(Box::pin' src` returns only work.rs:212; pump.rs imports `channel::Receiver` at line 38 and no `Sender`; encode.rs imports `Stream` at line 14 and uses it bare at line 207)
- Seen by: structure, perfapi (three overlapping candidates each); refutation: confirmed all; history: no-rationale-found (the lone `PayloadCodec` imports landed in 4356e197/71de90c1 after the files' import blocks; no rustfmt.toml regroups; c6fe4018 is a prior sweep in the same direction)
- Owner-gated: no

Four small legibility costs, all the same doctrine (imports over long qualified paths unless the qualification informs; finished code reads as evidently right): (a) `use crate::message::PayloadCodec;` sits alone ahead of the `std` group in three files, splitting each file's crate imports in two; (b) `crate::tree::mirror::streaming::channel::Sender<Scope>` is spelled out at pump.rs:178 and 280 while `channel::Receiver` is imported, `tokio::io::AsyncRead` at pump.rs:436 and 532, and `futures::Stream` at encode.rs:178 beside the import at line 14; (c) `pin!(Box::pin(..))` at work.rs:212 pins an already-`Unpin` `Pin<Box<_>>` (the walk awaits `complete` unboxed at materialized/work.rs:124, so whether the box is wanted for stack size is undocumented either way); (d) `let mut requests = requests;` at encode.rs:54, 94, 138 exists only to add `mut` to a by-value parameter, and `Vec::<Scope>::new()` at pump.rs:223 reads as a collection where an empty iterator is meant.

Evidence:

    8	use crate::message::PayloadCodec;
    9	use std::pin::{Pin, pin};

    pump.rs:178	        next_scopes: crate::tree::mirror::streaming::channel::Sender<Scope>,
    encode.rs:14	use futures::{Stream, StreamExt};
    encode.rs:178	    encoded: &mut (impl futures::Stream<Item = Result<Encoded<Q>, adapter::EncodeError<E>>> + Unpin),
    work.rs:212	            let mut protocol = pin!(Box::pin(complete(tasks, finish)));
    encode.rs:54	    let mut requests = requests;

Resolution: fold `PayloadCodec` into each file's `use crate::{...}` block; add `Sender` to pump.rs's `channel::{..}` import and `AsyncRead` to a `tokio::io` import; write `Stream` at encode.rs:178; `let mut protocol = Box::pin(complete(tasks, finish));` (or `pin!(complete(..))` if the box is not wanted); `mut requests: Replies<B::Erased>` in the three signatures; `std::iter::empty::<Scope>()` at pump.rs:223 (or nothing, under remote-proxy-25). Acceptance: one contiguous crate import group per file; no `crate::tree::mirror::streaming::channel::` or `tokio::io::AsyncRead` in pump.rs bodies; `futures::Stream` absent from encode.rs; `grep -rn 'pin!(Box::pin' src` empty; no `let mut requests = requests;`; `just fmt` and `just clippy` clean.

### remote-proxy-12: execute fuses the biased select to the failure-attribution table
- Where: src/tree/mirror/streaming/remote/proxy/work.rs:197-268 (related: src/tree/mirror/streaming/remote/proxy/work.rs:159-196, src/tree/mirror/streaming/remote/proxy/work/tests.rs:41-77, src/tree/mirror/streaming/remote/proxy/work/tests.rs:123-330)
- Class / severity / confidence: modularity / low / high
- Provenance: assessed (read; work/tests.rs's four attribution tests all drive the table through `parked_session()`)
- Seen by: structure; refutation: confirmed (extraction is behavior-preserving; the flush poll must still precede attribution since `errors` is read after it); history: no-rationale-found (the table grew in place across 54420d7f6, 3903532f6, d2b2403c)
- Owner-gated: no

`execute` runs the select, does a conditional flush poll via a two-arm match whose first arm is empty, then applies a five-arm attribution over `(outcome, errors, remote)`. The table is a pure decision on values already in hand, but it can only be exercised through a parked-session harness because it is fused to the select, and its 38-line doc has to explain both the poll order and the attribution rule. Separating "which future won" from "what the session reports" gives each half a doc that fits its mechanism and makes the table directly testable as a function of its inputs. Principle: legibility of the trickiest code in the partition.

Evidence:

    221	            match &outcome {
    225	                Ok(_) | Err(Error::Accept(_)) => {}
    226	                Err(_) => {
    231	                    let _ = futures::poll!(accept.as_mut());
    232	                }
    233	            }

    257	            Err(error) => match errors.queued_supply_closed() {
    258	                Some(supply) => Err(Error::Stream(supply)),
    259	                None => match errors.take_supply_failure() {

Resolution: extract `fn attribute<E>(outcome: Result<O, Error<E>>, errors: &mut FirstStreamError, remote: Speaker) -> Result<O, Error<E>>` holding lines 236-267 and the attribution paragraphs of the doc; `execute` keeps the select, replaces the empty-arm match with `if !matches!(outcome, Ok(_) | Err(Error::Accept(_))) { let _ = futures::poll!(accept.as_mut()); }`, and calls `attribute`. Optionally add a table-driven unit test of `attribute` beside the session-level witnesses. Acceptance: `execute`'s body is the select, the flush, and one call; the four attribution tests in work/tests.rs pass unchanged.

### remote-proxy-13: The `Ok | Accept` arm comment explains only the `Accept` pattern
- Where: src/tree/mirror/streaming/remote/proxy/work.rs:221-225
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read)
- Seen by: prose; refutation: reframed (the "five copies drift" framing overstated; the other restatements sit at distinct, defensible altitudes); history: no-rationale-found (the arm was born with this comment in 54420d7f6; it never covered `Ok`)
- Owner-gated: no

The comment on a two-pattern arm describes one pattern: why a violation-resolved accept arm must not be polled again. It says nothing about why a successful `Ok` outcome skips the flush poll (a completion needs no cause). A comment inaccurate for half its arm is the kind of thing a later reader "fixes" the wrong way.

Evidence:

    221	            match &outcome {
    222	                // A violation resolved the accept arm: the driver is
    223	                // complete and must not be polled again, and a violating
    224	                // driver never deposited (it returns instead of parking).
    225	                Ok(_) | Err(Error::Accept(_)) => {}

Resolution: "A completion needs no cause; a violation resolved the accept arm, whose driver is complete, must not be polled again, and never deposited." (Under remote-proxy-12 the arm becomes an `if` and the comment moves with it.) Acceptance: the comment is true of both patterns.

### remote-proxy-14: `mod pump` and `fn pump` name two different things in one file
- Where: src/tree/mirror/streaming/remote/proxy/work.rs:271-272 (related: src/tree/mirror/streaming/remote/proxy/work.rs:40, src/tree/mirror/streaming/remote/proxy/work/pump.rs:1, src/tree/mirror/streaming/materialized/work.rs:128-134)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed (the walk's relay is also named `pump`, so the module name is the odd one); history: no-rationale-found
- Owner-gated: no

The module holds the `Work` stage methods, the decode loops, and the `Early` cursor; the free function relays one decoded response stream to its outgoing edge; the module docs use "pump" in both senses ("the reply pumps", "decode pump"). A reader disambiguates on every mention.

Evidence:

    40	mod pump;
    271	/// Drive one decoded response stream into its outgoing relay edge.
    272	async fn pump<E: Send, Err: Send + 'static>(

Resolution: rename the module (`stages`) since the walk's relay shares the function name, and align the module docs' vocabulary. Acceptance: `pump` names exactly one thing in work.rs.

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

### remote-proxy-17: Two word choices misstate the mechanism: "acknowledged" questions and the decode-side "register"
- Where: src/tree/mirror/streaming/remote/proxy/work/encode.rs:175 (related: src/tree/mirror/streaming/remote/proxy/work/encode.rs:183-189, src/tree/mirror/streaming/remote/proxy/work/queues.rs:17, src/link/routed/header.rs:61)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read `write_reply`'s body: it collects the questions `write_encoded` releases after each frame flushes; `grep -rn acknowledg src` shows the routed link layer uses "acknowledgement" for a real wire ACK byte)
- Seen by: prose; refutation: reframed (of five candidates, "codec seam" is anchored in stats.rs:6, 29, 122 and stands; "unsound" is the owner's ruling wording; "load-bearing" is crate idiom at eleven sites); history: no-rationale-found for the two kept
- Owner-gated: no

`write_reply` says it retains the reply's "acknowledged questions", but nothing is acknowledged: the peer has not replied; the questions are those whose frame has flushed, and this crate uses "acknowledgement" elsewhere for a real wire ACK. queues.rs calls `next_scopes` "the decode-side register", an unanchored metaphor for a scope queue. Principle: established terms of art only; metaphors only where they rewrite as mechanism.

Evidence:

    175	/// Flush every frame in one reply and retain its acknowledged questions.
    queues.rs:17	//! - [`next_scopes`] is the decode-side register, also window-sized, whose

Resolution: "retain its flushed questions"; "the decode-side scope queue". Acceptance: neither phrase remains in the partition.

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

### remote-proxy-19: The proxy ordering trace has no liveness floor: an empty trace satisfies both assertions
- Where: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:24-55 (related: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:79-109, src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:188-194, src/tree/mirror/streaming/remote/proxy/work/progress.rs:10-17, src/tree/mirror/streaming/remote/proxy/tests.rs:402, src/tree/mirror/streaming/remote/proxy/tests.rs:426, src/tree/mirror/streaming/remote/proxy/tests.rs:597-606, src/tree/mirror/streaming/remote/proxy/tests.rs:610-624, src/testing.rs:375-394)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read: `Trace`'s field is private and has no length accessor; both assertions iterate `self.0` and their trailing asserts quantify over empty maps, so both pass on an empty vector; `record` pushes only when the thread-local is `Some`, i.e. on the `with_trace` caller's thread; grep of proxy/tests.rs shows the only consumers are `assert_valid` at 402 and 599 and `assert_registration_causality` at 426, with no floor; the channel instrument beside them does have one at 602-606. The refutation pass ran the three consuming tests at PROPTEST_CASES=1024: all pass, which shows the trace is populated today and says nothing about the floor.)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (the asymmetry with the channel floor is original to cbfe1aff3)
- Owner-gated: no

The three tests that consume the trace certify wire-before-question and registration causality only as long as `run_to_quiescence` happens to poll on the calling thread and `Progress::new`/`record` stay wired; if either drifts, the pins go green vacuously. Doctrine: meters need liveness floors; an ordering check over a counter passes when the counter stops counting. The cheapest artifact that passes these assertions today is a `Progress` that records nothing.

Evidence:

    24	/// A completed positive session's proxy-ordering trace.
    25	#[derive(Debug)]
    26	pub struct Trace(Vec<Event>);
    27	
    28	impl Trace {
    29	    /// Assert wire-before-question and reply-before-scope ordering.
    30	    pub fn assert_valid(&self) {
    31	        let mut questions = BTreeMap::<(usize, usize), usize>::new();
    32	        let mut scopes = BTreeMap::<(usize, usize), usize>::new();
    33	        for (index, event) in self.0.iter().enumerate() {

    188	pub fn record(work: usize, kind: Kind, height: usize) {
    189	    EVENTS.with(|events| {
    190	        if let Some(events) = events.borrow_mut().as_mut() {
    191	            events.push(Event { work, height, kind });

    tests.rs:601	    for kind in QueueKind::PROXY {
    tests.rs:602	        assert!(
    tests.rs:603	            report.kind(kind).channels > 0,

Resolution: give `Trace` a floor every divergent two-proxy session must meet and call it beside the channel floor in `instrumented_channels_cover_every_proxy_edge` and in the two proptests: for example `assert_covers_divergent_session()` requiring exactly one `DecodedReply` at `UnderRoot::HEIGHT` (the initiator-side proxy's greeting-seeded opening, `Work::initiator`, pump.rs:82-86) and exactly one `LocalQuestion` at `UnderRoot::HEIGHT` (the responder-side proxy's opening publication, encode.rs:142-143) across the trace, plus at least one `WireReply`. Commit the known-bad demonstration: `with_trace(|| ())` yields a trace the floor rejects. Acceptance: a `should_panic` test builds `Trace` from `with_trace(|| ())` and fails the floor with a named message; the three consuming tests call the floor and stay green; deleting the `progress.decoded_reply` call in `Work::initiator` turns at least one committed test red.

Construction: `let (_, trace) = with_trace(|| ()); trace.assert_valid(); trace.assert_registration_causality();` passes today, which is the demonstration that the checks are vacuous. Add the floor and assert this construction panics.

### remote-proxy-20: The trace ledgers are keyed by (endpoint, height), which two live recorders share at height zero
- Where: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:35-45 (related: src/tree/mirror/streaming/remote/proxy.rs:31-32, src/tree/mirror/streaming/remote/proxy/work/pump.rs:161-162, src/tree/mirror/streaming/remote/proxy/work/pump.rs:259-268, src/tree/mirror/streaming/remote/proxy/work/pump.rs:318-326, src/tree/mirror/streaming/remote/proxy/work/pump.rs:396, src/tree/mirror/streaming/remote/proxy/work/encode.rs:63, src/tree/mirror/streaming/remote/proxy/work/encode.rs:99, src/tree/mirror/streaming/remote/proxy/work/progress/trace/tests.rs:57-66)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read: on the responder-side proxy the `Descending<2>` decode pump records `decoded_reply(H::HEIGHT = 0, n)` and `terminal_decode_pump` records `decoded_reply(Z::HEIGHT, 0)` under the same `work`; on the initiator-side proxy `leaf_replies`' encoder records `wire_reply(Z::HEIGHT, ..)` beside `complete_initiator`'s `encode::terminal`; `yield_reply_scopes!` records before the `yield`, which suspends through the relay's `send_or_cancel`. The refutation pass ran the three trace-consuming tests at PROPTEST_CASES=1024: all pass.)
- Seen by: correctness; refutation: reframed (the `scopes` ledger's window opens at a single derived height-0 scope, not two; reaching it needs a dispute nested through every schedule stage, which the trace-asserting generators cannot produce); history: no-rationale-found; the collision-schedule test mode note (2026-08-21) would make the interleaving reachable
- Owner-gated: no

`assert_drained` requires the previous entry at `(work, height)` to be fully published before a new `WireReply`/`DecodedReply` at the same key. At height zero two recorders per endpoint are live concurrently, so a legitimate interleaving (the leaf-parent stage records a decoded reply with one derived leaf scope, yields, and before the scope publishes the terminal records its own decoded reply) trips the ledger as a false positive. It cannot occur under the committed generators because a leaf-height question needs two leaves sharing a 31-byte hash prefix, so the check is sound only by hash uniformity, and the premise is unstated. The committed negative test `rejects_next_decoded_reply_before_scopes` records exactly the event sequence that interleaving would produce. When the planned collision-schedule test mode lands and the terminal stage carries real work, this instrument starts failing on correct sessions.

Evidence:

    35	                Kind::WireReply { questions: count } => {
    36	                    assert_drained(&questions, event, index, "questions");
    37	                    questions.insert((event.work, event.height), count);
    38	                }
    39	                Kind::LocalQuestion => consume(&mut questions, event, index, "wire reply"),
    40	                Kind::DecodedReply { scopes: count } => {
    41	                    assert_drained(&scopes, event, index, "scopes");
    42	                    scopes.insert((event.work, event.height), count);
    43	                }

    proxy.rs:31	        $progress.decoded_reply($height, $count);
    proxy.rs:32	        $yielded;
    pump.rs:396	                progress.decoded_reply(Z::HEIGHT, 0);
    pump.rs:162	        let responses = self.decode_pump(questions, incoming, next_scopes, early, H::HEIGHT);

Resolution: key the ledgers by stage: add a stage tag to `Kind::WireReply`/`DecodedReply`, or record the terminal encoder and decoder under a distinct label; alternatively state the one-recorder-per-key premise and the prefix-collision argument at the ledger. Keying by stage is preferable because the premise is scheduled to expire. Acceptance: two concurrent recorders never share a ledger key, or the premise is stated at the ledger with the argument.

Construction: in trace/tests.rs, the sequence `record(0, DecodedReply { scopes: 1 }, 0); record(0, DecodedReply { scopes: 0 }, 0); record(0, NextScope, 0);` is what a correct responder-side proxy produces when its height-2 decode derives one leaf scope and the terminal decode records between the yield and the scope publication; `assert_valid` rejects it. Reaching it end to end needs a pair of trees disputing down to height 1, i.e. two leaves under one 31-byte prefix, which the collision-schedule mode is designed to construct.

### remote-proxy-21: Trace causality check mixes a named height with literal heights
- Where: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:92-98 (related: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:66-70, src/tree/typed/height.rs:81)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read; `Z::HEIGHT` is used elsewhere in the partition, pump.rs:258)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (born mixed in 675e2f536)
- Owner-gated: no

`UnderRoot::HEIGHT` names the under-root height symbolically while the leaf and leaf-parent heights are the literals `0` and `1` in the same expression; the doc above names them in words. Named constants over magic numbers, applied consistently.

Evidence:

    92	                    let available = if event.height == UnderRoot::HEIGHT {
    93	                        1
    94	                    } else if event.height == 0 {
    95	                        flushed(0) + flushed(1)
    96	                    } else {
    97	                        flushed(event.height + 1)
    98	                    };

Resolution: import `Z` and `S` beside `UnderRoot` and write `Z::HEIGHT` / `<S<Z>>::HEIGHT`. Acceptance: no bare height literals in `assert_registration_causality`; trace/tests.rs passes.

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

### remote-proxy-23: The erase-and-box of a typed request stream is spelled ten times across the proxy and the walk, under two aliases for one type
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:98-99 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:147-148, src/tree/mirror/streaming/remote/proxy/work/pump.rs:255-256, src/tree/mirror/streaming/remote/proxy/work/pump.rs:316-317, src/tree/mirror/streaming/remote/proxy/work/pump.rs:347-348, src/tree/mirror/streaming/remote/proxy/work/encode.rs:38, src/tree/mirror/streaming/materialized/work/levels.rs:45, src/tree/mirror/streaming/materialized/work/levels.rs:193, src/tree/mirror/streaming/materialized/work/levels.rs:318, src/tree/mirror/streaming/materialized/work/levels.rs:532, src/tree/mirror/streaming/materialized/work/levels.rs:638, src/tree/mirror/streaming/materialized.rs:831)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn erase_reply src` excluding erased.rs lists exactly these ten call sites; both alias definitions read: `Pin<Box<dyn Stream<Item = Reply<E>> + Send>>` and `BoxStream<'static, Reply<E>>` are the same type)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (the two aliases were born the same day in parallel erasure commits d8bef16b9 and bf1a5b4bc)
- Owner-gated: no

`Box::pin(requests.map(erased::erase_reply::<B, H>))` appears five times here and five times in the walk, and the alias for its type is defined twice. The typed-to-erased boundary is the one place the module docs say erasure happens; naming that step once beside `erase_reply` states it where the vocabulary lives. The walk sites belong to another partition's reviewer, but the helper should be shared.

Evidence:

    98	        let requests: encode::Replies<B::Erased> =
    99	            Box::pin(requests.map(erased::erase_reply::<B, UnderRoot>));

    encode.rs:38	pub type Replies<E> = Pin<Box<dyn Stream<Item = Reply<E>> + Send>>;
    levels.rs:45	type Replies<E> = BoxStream<'static, Reply<E>>;

Resolution: add `pub(crate) fn erase_requests<B, H>(requests: impl Requests<B, H>) -> BoxStream<'static, Reply<B::Erased>>` to erased.rs next to `erase_reply`, with one `pub(crate)` alias there; replace the ten call sites and delete the two local aliases. Acceptance: `grep -rn 'map(erased::erase_reply' src` returns nothing outside erased.rs; one alias for the erased request stream.

### remote-proxy-24: Three decode pumps re-thread the same ingress context and repeat the same six-argument decode call four times
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:183-187 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:198-206, src/tree/mirror/streaming/remote/proxy/work/pump.rs:227-235, src/tree/mirror/streaming/remote/proxy/work/pump.rs:283-298, src/tree/mirror/streaming/remote/proxy/work/pump.rs:377-392, src/tree/mirror/streaming/remote/proxy/work/pump.rs:417-430, src/tree/mirror/streaming/remote/proxy/work/pump.rs:493-500, src/tree/mirror/streaming/remote/proxy/work/pump.rs:104-110, src/tree/mirror/streaming/remote/proxy/work/pump.rs:152-158, src/tree/mirror/streaming/remote/proxy/work/pump.rs:260-266, src/tree/mirror/streaming/remote/proxy/work/pump.rs:319-325, src/tree/mirror/streaming/remote/proxy/work/pump.rs:352-358, src/tree/mirror/streaming/remote/adapter/decode.rs:203-209, src/tree/mirror/streaming/remote/adapter/decode.rs:289)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the three identical five-line preludes and the four identical decode argument bundles; read decode.rs: `decode_reply`/`decode_leaf_reply` take `ledger: SupplyLedger` by value and immediately borrow it at line 289, while `early_supplies` returns a `'static` boxed stream and does need an owned ledger)
- Seen by: structure, perfapi (the ledger clone); refutation: confirmed both; history: no-rationale-found (pure accretion across 165b0dd3, 08f2899b, d8bef16b9, 4356e197)
- Owner-gated: no

`decode_pump`, `leaf_decode_pump`, and `terminal_decode_pump` each open with the identical capture of `progress`, `backend`, `version_bytes`, `ledger`, `codec`, then call the adapter with the identical bundle; `Early` holds the same fields again and re-threads them into `early_supplies`; the encode spawn sites repeat `self.backend(), self.budget, ..., self.progress` five times. The bundle (backend, declared version bound, supply ledger, payload codec) is one concept, the session's ingress premises, spelled as loose values at eleven sites. A minor cost rides along: each decode iteration clones `backend` and `ledger` (an `Arc` refcount round trip each) to hand owned values to adapter entries that only borrow the ledger; the adapter signature is another partition's, the pump follows.

Evidence:

    183	        let progress = self.progress;
    184	        let backend = self.backend();
    185	        let version_bytes = self.peer_version_bytes;
    186	        let ledger = self.peer_supplies.clone();
    187	        let codec = self.codec;

    290	                let Decoded { reply, questions } = decode_leaf_reply(
    291	                    backend.clone(),
    292	                    version_bytes,
    293	                    ledger.clone(),
    294	                    scope,
    295	                    &mut incoming,
    296	                    codec,
    297	                )

Resolution: introduce a `Clone` struct in `work/` (say `Ingress<B> { backend: B, version_bytes: u64, ledger: SupplyLedger, codec: PayloadCodec }`) with `async fn decode(&self, scope, &mut incoming)` and `decode_leaf` wrappers over the adapter calls; each pump captures one `Ingress` and the `Progress`; `Early` holds an `Ingress`. Whether the adapter's entries take the same struct, and whether they take `&SupplyLedger`, is the adapter reviewer's call. Acceptance: each decode pump's prelude is two lets; the six-argument decode call appears once per adapter entry; `just clippy` clean; proxy suites pass.

### remote-proxy-25: decode_pump's early-supply branch duplicates the decode call and the yield, exits through `continue`, and re-asserts what the adapter guarantees
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:192-240 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:207, src/tree/mirror/streaming/remote/adapter/decode.rs:220-225, src/tree/mirror/streaming/remote/adapter/scope.rs:33-37)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read; the refutation pass traced that for a request scope (empty listing, `is_request`) any `Query` reaction fails `read_reply` with `ScopeError::UnpositionedQuery` via `scope.next()` on an empty listing, so `asked` is provably empty on the `Ok` path)
- Seen by: structure; refutation: confirmed, adding that the `debug_assert!` at 207 recomputes an adapter guarantee; history: no-rationale-found (55d76d5cf's shape verbatim)
- Owner-gated: no

Inside the loop, the root-request branch and the general path both call `decode_reply` with the same arguments and both end in `yield_reply_scopes!`; the branch differs only in supplementing `reply.replies` with the exploded early node and forcing zero scopes. The `continue` and the second macro invocation (with `Vec::<Scope>::new()`) make the loop body fifty lines. The forced zero is already implied: a request scope's decode cannot return questions, which is also why `debug_assert!(asked.is_empty())` is a recompute of a deterministic guarantee rather than a guard. The invariant ("a root-level request's reply is the wire reply plus the early node's children") is easier to see as one decode followed by an optional supplement.

Evidence:

    207	                    debug_assert!(asked.is_empty(), "an empty request opens no lower scope");
    220	                    yield_reply_scopes!(
    221	                        progress, height, 0;
    222	                        yield Reply { replies };
    223	                        next_scopes => Vec::<Scope>::new();
    224	                    );
    225	                    continue;
    226	                }

Resolution: before decoding, compute `let early_key = (early.armed() && scope.is_request()).then(|| scope.parent().pop());`. Decode once. If `early_key` is `Some((root, radix))`, run `advance_to` and extend `reply.replies`. Then one `yield_reply_scopes!` with `questions.len()` and `next_scopes => questions` (empty in the root-request case, so behavior is unchanged); drop the `debug_assert!`. Acceptance: one `decode_reply` call and one `yield_reply_scopes!` in `decode_pump`; no `continue`; the early-supply fixtures in `proxy/tests/{greeting,declarations,malformed}.rs` pass unchanged.

### remote-proxy-26: The early branch states that the pairing wire reply arrives empty but splices without checking
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:193-196 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:209-219, src/tree/mirror/streaming/remote/codec/signal.rs:336, src/tree/mirror/streaming/materialized/error.rs:36-41)
- Class / severity / confidence: documentation / nit / low
- Provenance: assessed (read; the refutation pass traced that a wire supply duplicated among the early children lands on a non-ascending or repeated radix that the walk rejects as `InvalidSupply`/`UnexpectedSupply`, and the ledger charges both copies)
- Seen by: correctness; refutation: reframed (downstream detection exists; what remains is diagnosis attribution and a comment stating a peer premise as fact); history: no-rationale-found (the premise is transcribed from the opening-supply design)
- Owner-gated: no

The comment says the pairing reply "arrives empty" and the code appends the early children to whatever `reply.replies` holds. The codec grammar admits supplies on this stream, so a nonconforming initiator that supplies the same subtree on both streams is caught one layer down by the walk's supply-ordering violations rather than by a proxy-level `UnaskedReply`. Under the model of record (honest peers) the comment is accurate and the gap is only where the conformance diagnosis lands; every other completeness premise in this module (`ExtraOpening`, `UnaskedLocalReply`, `reject_extra`, `Early::finish`) is enforced at the proxy.

Evidence:

    193	                    // A root-level request: its content crossed at the
    194	                    // opening, so the pairing reply here arrives empty and
    195	                    // the early stream carries the node — or neither does,
    196	                    // when pruning removed the whole subtree.
    209	                    let mut replies = reply.replies;
    210	                    if let Some(node) = early.advance_to(&backend, root, radix).await? {

Resolution: reword the comment to what the code does ("a conforming initiator's pairing reply is empty; any supplies it does carry precede the early children, and the walk's ordering checks reject a duplicate"), or add the check (`UnaskedReply` when `reply.replies` is nonempty in this branch) with a `pump/tests.rs` construction. Acceptance: the comment and the code agree.

### remote-proxy-27: Every decoded reply pays a FAN-slot channel and two boxed streams whether or not it carries a supply (adapter call, cross-partition)
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:227-235 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:290-298, src/tree/mirror/streaming/remote/proxy/work/pump.rs:384-392, src/tree/mirror/streaming/remote/adapter/decode.rs:277-291, src/tree/mirror/streaming/erased.rs:319-334, tests/decode_alloc.rs:1-12)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read decode.rs:288-291: every `decode` builds `mpsc::channel(FAN)` and joins reader and assembler; read tests/decode_alloc.rs's module doc: it meters declared-length payload reads and supply reads, not the per-reply fixed allocations. Not measured.)
- Seen by: perfapi; refutation: confirmed (with the caveat that lazy channel creation changes the join's control flow, so it is measure-first); history: no-rationale-found (4d55d4844 prices the FAN capacity, not its construction per supply-free reply)
- Owner-gated: no

In a dispute descent most replies are `Match`/`Query` only; for them the channel, the assembler, and the join are overhead. Denominator: per decoded reply. The crate already holds the encoder to zero allocations per body-free frame (`tests/encode_alloc.rs`); the decoder's per-reply fixed allocations are the ingress twin of that number and are unmetered. Instruments before cures: this is a candidate to meter first. The resolution lives in the adapter partition; the call site is here.

Evidence:

    227	                let Decoded { reply, questions } = decode_reply::<B, _>(
    228	                    backend.clone(),
    229	                    version_bytes,
    230	                    ledger.clone(),
    231	                    scope,
    232	                    &mut incoming,
    233	                    codec,
    234	                )
    235	                .await?;

    decode.rs:288	    let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
    decode.rs:290	    let assemble = assemble_supplies::<B>(backend, children_height, rx);
    decode.rs:291	    let (read, assembled) = futures::future::join(read, assemble).await;

Resolution: first, add a `stats_alloc` region around a `Match`-only reply decode (the `tests/decode_alloc.rs` pattern) and commit the current count as the baseline. Then, in the adapter: create the leaf channel and assembler lazily on the first `Supply` record, or let the pump own one channel per stage and hand it in so the allocation amortizes. Acceptance: a committed allocation meter for a supply-free reply decode; after the change its count excludes the channel and assembler allocations.

Construction: wrap `decode_reply` over a frame stream of one `Frame::Reaction(Match, Flow::End)` in a `stats_alloc::Region` and read `allocations`; expect a nonzero count today attributable to the channel, the `ReceiverStream` box, and the assembler box.

### remote-proxy-28: leaf_decode_pump and terminal_decode_pump are one loop; the encode side already unifies the same pair with an Option
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:371-401 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:276-307, src/tree/mirror/streaming/remote/proxy/work/encode.rs:41-71, src/tree/mirror/streaming/remote/proxy.rs:33-35)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read both bodies; `yield_reply_scopes!` takes `$scopes` as a bare expression used as `&$scopes`, so the unification needs an `if let Some(..)` around the publish, which the encode side already shows)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (both extracted by d8bef16b9; the encode side's `Option` shape is the original design from cbfe1aff3). The scope-A mutants note records that the terminal stage runs empty in every committed suite, so "proxy tests pass" is weak acceptance at height zero.
- Owner-gated: no

`terminal_decode_pump` is `leaf_decode_pump` with the derived questions rejected as `TerminalQuery` instead of published. `encode::terminal` handles exactly this fork for the outgoing direction with `questions: Option<Sender<Scope>>`, so the decode side carries a second function where the encode side carries one branch. Two spellings of one loop drift independently. If remote-proxy-29 deletes the `TerminalQuery` arm, the unification is a `None` sink and nothing else.

Evidence:

    393	                if !questions.is_empty() {
    394	                    Err(Error::TerminalQuery)?;
    395	                }
    396	                progress.decoded_reply(Z::HEIGHT, 0);
    397	                yield reply;

    encode.rs:64	        if let Some(questions) = &questions {
    encode.rs:65	            publish(questions, batch, progress, Z::HEIGHT).await;
    encode.rs:66	        } else if !batch.is_empty() {
    encode.rs:67	            return Err(Error::TerminalQuery);

Resolution: give `leaf_decode_pump` a `next_scopes: Option<Sender<Scope>>` parameter; when `None` and `questions` is nonempty, fail with `TerminalQuery` (or, after remote-proxy-29, nothing), else record and yield; `complete_responder` passes `None`, `leaf_replies` passes `Some(next_scopes)`. Delete `terminal_decode_pump`. Acceptance: one leaf-height decode pump in pump.rs; `instrumented_channels_cover_every_proxy_edge` and the `Trace` assertions still pass.

### remote-proxy-29: The responder-terminal TerminalQuery check is unreachable from wire bytes
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:393-395 (related: src/tree/mirror/streaming/remote/codec/signal.rs:99, src/tree/mirror/streaming/remote/codec/signal.rs:338-340, src/tree/mirror/streaming/remote/proxy/error.rs:78-80, src/tree/mirror/streaming/remote/proxy/work/encode.rs:66-68, src/tree/mirror/streaming/remote/adapter/decode.rs:178-185, src/tree/mirror/streaming/remote/proxy/state.rs:431-433)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read the chain: `complete_responder` binds `incoming::<Z>()` for `remote = Responder`; `Stream::at_height(Responder, 0)` is `MAX`; `class((Responder, MAX))` is `TerminalLeafReplies`; its placement grammar admits only `Signal::Supply(Flow::End) | Signal::End(_)`, so both query forms are rejected as `InvalidSignalPlacement` before the adapter sees a frame and `decode_leaf_reply`'s `questions` is always empty here; grep shows no test reaches the arm)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (born redundant: the grammar commit 3dac1b852 is an ancestor of the proxy landing cbfe1aff3)
- Owner-gated: no

The arm duplicates a check the codec makes one layer down, and `Error::TerminalQuery`'s doc ("The terminal responder attempted to ask another leaf question") describes a peer behavior the grammar already excludes; a peer that did it would receive `InvalidSignalPlacement`, not this. It is reachable only by in-process frame construction, the situation `read_early` documents at decode.rs:178-181. Doctrine: a guard earns its place by naming a concrete, constructible failure the existing instruments miss.

Evidence:

    393	                if !questions.is_empty() {
    394	                    Err(Error::TerminalQuery)?;
    395	                }

    signal.rs:99	            (Speaker::Responder, Self::MAX) => StreamClass::TerminalLeafReplies,
    signal.rs:338	            StreamClass::TerminalLeafReplies => {
    signal.rs:339	                matches!(self.signal, Signal::Supply(Flow::End) | Signal::End(_))
    signal.rs:340	            }
    error.rs:78	    /// The terminal responder attempted to ask another leaf question.

Resolution: delete the arm and let the codec's placement rejection be the diagnosis; the variant stays for the local encode-side use at encode.rs:67, and its doc then names the local producer (remote-proxy-3). Alternatively keep it with a comment in the style of decode.rs:178-181 stating the grammar excludes the frame and the arm exists for in-process construction only, plus a `pump/tests.rs` test that constructs the frame. Acceptance: either the arm is gone and `Error::TerminalQuery`'s doc names only the local producer, or the arm carries the in-process-only rationale and a test fires it.

### remote-proxy-30: Early encodes a four-state cursor as three Options and a bool
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:413-460 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:464-522, .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/03-proxy-adapter.md)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed; history: no-rationale-found for the representation (born in 55d76d5cf); the scope-A mutants note already designs a module split for `Early` with a pairing property test, not yet landed
- Owner-gated: no

`Early` has `receiver: Option`, `supplies: Option`, `lookahead: Option`, `exhausted: bool`: sixteen nominal combinations of which four are reachable (unarmed; armed but unclaimed; streaming with an optional lookahead; exhausted). `armed()` is a three-way disjunction, `advance_to` interleaves the state transitions with the radix pairing, and the `.expect("an unarmed cursor resolves no request")` exists to rule out a combination the representation admits (it is discharged today by the `armed()` guard at the single call site). Types-first: make the invalid states unrepresentable so the pairing logic reads without a mental table.

Evidence:

    423	    receiver: Option<StreamReceiver<Rx>>,
    424	    supplies:
    425	        Option<Pin<Box<dyn Stream<Item = Result<(u8, B::Erased), DecodeError<B::Error>>> + Send>>>,
    426	    lookahead: Option<(u8, B::Erased)>,
    427	    exhausted: bool,
    458	    fn armed(&self) -> bool {
    459	        self.receiver.is_some() || self.supplies.is_some() || self.lookahead.is_some()
    460	    }
    489	                    let receiver = self
    490	                        .receiver
    491	                        .take()
    492	                        .expect("an unarmed cursor resolves no request");

Resolution: `enum EarlyCursor<B, Rx> { Unarmed, Armed(StreamReceiver<Rx>), Streaming { supplies: BoxStream<..>, lookahead: Option<(u8, B::Erased)> }, Exhausted }`; `armed()` is `!matches!(self, Unarmed)`; `advance_to` matches on the state; `finish` is `Streaming { lookahead: Some(_) } => Err`, `Streaming { .. } => poll once`, else `Ok`. Land it together with the scope-A note's module split and pairing property (remote-proxy-31). Acceptance: `Early` has one state field; no `.expect` in its impl; the `UnaskedReply` path pinned in `proxy/tests/malformed.rs:180` still fires.

### remote-proxy-31: Nine proxy error arms have no committed test that fires them
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:479-521 (related: src/tree/mirror/streaming/remote/proxy/work/encode.rs:60, src/tree/mirror/streaming/remote/proxy/work/encode.rs:67, src/tree/mirror/streaming/remote/proxy/work/encode.rs:96, src/tree/mirror/streaming/remote/proxy/work/encode.rs:139-140, src/tree/mirror/streaming/remote/proxy/work/encode.rs:170, src/tree/mirror/streaming/remote/proxy/work/encode.rs:210, src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:164-184, src/tree/mirror/streaming/remote/proxy/tests/harness.rs, .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/03-proxy-adapter.md)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep over src, tests, benches, examples: `UnansweredRemoteQuery`, `MissingOpening`, `OpeningEncode`, `ExtraOpening`, `UnaskedLocalReply` appear only at their encode.rs construction sites; `TerminalQuery` only at encode.rs:67 and pump.rs:394; the one committed proxy `UnaskedReply` assertion, `duplicated_reply_is_rejected_as_unasked`, lands on `reject_extra` at pump.rs:536, not the three `Early` arms)
- Seen by: correctness; refutation: confirmed; history: already-known in part (the scope-A note records the `Early::finish` arms and the `terminal` guard as surviving mutants with designed dispositions: an `Early` pairing property after a module split plus one full-proxy wire witness, and a scripted-channel contract family over `encode::terminal`; nothing has landed since 2026-08-21)
- Owner-gated: no

The early-supply cursor rejects a nonconforming initiator on three arms (a supply group behind the request cursor, a leftover lookahead at `finish`, an unread group at `finish`), and the encoders reject a nonconforming local participant on six. No test reaches any of them. These are conformance-bug detectors, not a security boundary, but the three cursor arms are the only detectors for an initiator whose early set disagrees with the responder's requests, and a detector no test fires is one whose drift nothing would notice. Doctrine: every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

    479	                // Behind the request cursor: this group was never asked
    480	                // about at the root, so nothing will ever absorb it.
    481	                return Err(Error::UnaskedReply);
    511	    async fn finish(&mut self) -> Result<(), Error<B::Error>> {
    512	        if self.lookahead.is_some() {
    513	            return Err(Error::UnaskedReply);
    514	        }
    515	        if let Some(supplies) = &mut self.supplies
    516	            && !self.exhausted
    517	            && supplies.next().await.transpose()?.is_some()
    518	        {
    519	            return Err(Error::UnaskedReply);
    520	        }

    encode.rs:210	        return Err(Error::UnaskedLocalReply);

Resolution: remote-reachable group (pump.rs:481, 513, 519): land the scope-A note's `Early` pairing property (for ascending radix sets R requested and S supplied, `advance_to` over R against a scripted supply stream of S then `finish` yields each r answered iff r in S and ends in `UnaskedReply` iff S is not a subset of R), plus one full-proxy wire witness: extend the harness's greeting rewrite to drop one radix from the responder's listing as the initiator hears it, so the initiator early-ships a radix the responder never requests; place the orphan below a requested radix for the behind-cursor arm, as the highest early radix with a request following for the leftover-lookahead arm, and with no request after it for the unread-group arm. Local-only group (encode.rs sites): a `mirror(scripted_participant, proxy)` where a scripted in-process participant yields one reply too many, one too few, an opening that is not a query, or a terminal reply that asks a question; or, if these are judged unreachable by construction of the crate's own walk, say so at each site. Acceptance: each listed arm is reached by a committed test asserting the exact variant, or carries a comment stating why it is a local-programmer-error detector with no constructible peer input.

Construction: in `proxy/tests/harness.rs`, add a greeting rewrite that drops a chosen root radix `r` from the responder's greeting as received by the initiator, with `r` held by both trees; run the rewritten reconciliation; the initiator's `encode::opening` computes `early = true` for `r` and ships it, the responder never requests `r`, and the responder-side proxy's `Early` returns `UnaskedReply` from whichever arm the radix ordering selects. Assert the receiving endpoint's error is `RemoteError::UnaskedReply` and select the arm by choosing `r` relative to the requested radices.

### remote-proxy-32: queues.rs points readers to the window docs for "the slack", which the window docs no longer derive
- Where: src/tree/mirror/streaming/remote/proxy/work/queues.rs:31-36 (related: src/tree/mirror/streaming/window.rs:78-126)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i "slack\|undercount\|dequeued\|flushing batch" src/tree/mirror/streaming/window.rs` matches only line 726, "the Bernstein slack", in the statistical envelope section; `git show b76a31f38:...window.rs` carries the derivation at lines 91-95 and `git show d27cb5aa5:...window.rs` does not)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (b76a31f38 wrote both the pointer and the derivation; d27cb5aa5, the same day, rewrote window.rs and dropped the bullet)
- Owner-gated: no

The doc promises that the occupancy bound, its reachability, and the slack are all derived in the window module docs. The window docs carry the bound and the reachability argument; the slack statement exists only here. A cross-reference is a promise; the reader makes the round trip and comes back to the only statement there is, which reads like a summary of something more rigorous.

Evidence:

    31	/// a full round trip later. Tracks, not equals: occupancy undercounts the
    32	/// wire by a bounded slack (a flushing batch rides the wire before
    33	/// publication; the decoder holds one dequeued entry while its reply
    34	/// decodes). The canonical derivation — the occupancy bound, its
    35	/// reachability, and the slack — is in the
    36	/// [`window`](crate::tree::mirror::streaming::window) module docs.

Resolution: drop "and the slack" from the pointer so this doc is the slack's home ("The occupancy bound and its reachability are derived in the window module docs."), or restore the slack bullet to window.rs's flushed-question section (out of partition). Acceptance: every item the pointer names is present at its target.

## Positives

- `Progress` (progress.rs) is a zero-cost instrumentation seam done right: a `Copy` type that is a ZST outside `cfg(test)`, passed by value everywhere, with the trace machinery behind `#[cfg(test)] mod trace`; both ledgers (`assert_valid`, `assert_registration_causality`) have committed `should_panic` demonstrations in trace/tests.rs for six of their eight assertion paths, and both run inside the proptests over arbitrary divergence and adversarial channel schedules.
- `yield_reply_scopes!` (proxy.rs:18-38) keeps publish-after-yield in one place and states exactly why it must be a macro: the `yield` has to be lowered by `async_stream` before expansion. The rejected alternative is named at the declaration, and the walk's analogue is cross-referenced.
- `encode::write_encoded` releasing the question only after `Encoded::write_with` reports a successful flush turns the wire-before-publication liveness rule into the natural API order; the ordering is enforced by types and dataflow, not by discipline.
- `Work::execute`'s attribution logic (work.rs:197-268) is argued arm by arm in its doc and the code matches every clause; work/tests.rs pins the racing-consequence, queued-`SupplyClosed`, backend-exemption, and parked-protocol cases, and the contract those arms implement is stated on `Error` itself (error.rs:8-15), so code and promise are checkable against each other.
- Every panic site is discharged by construction rather than by input shape: `stream_at`'s expect and `Claims::take`'s expect are exercised for all seventeen (speaker, height) pairs synchronously by every divergent session, so a wrong mapping would fail every proptest at once; `Early::advance_to`'s expect follows from `armed()` at its single call site; `parent.pop()` at pump.rs:208 only ever sees height-31 scopes.
- The erased-body/typed-boundary discipline (pump.rs module doc; `Work::respond` as the single re-tag point) is applied uniformly: every pump body instantiates once per backend, and the typed heights survive only as `QueueRole` labels and prefix lengths.
- The listing-based `early` predicate in `encode::opening` (encode.rs:146-152) is a single linear merge over two already-sorted listings, and it is the right design: it lets the responder distinguish "all pruned away" (an empty reply on an open stream) from "no exclusive children" (no stream); the proxy's early splice matches the walk's `early_survivors` contract, so the join oracle is a meaningful differential check across the two implementations.
- Completeness is checked in both directions at every stage boundary: `UnansweredRemoteQuery`/`UnaskedLocalReply` for the local side, `reject_extra` and `Early::finish` for the remote side, `ExtraOpening`/`MissingOpening` for the opening.
- `Early`'s doc (pump.rs:404-412) is a model maintainer comment: it states the pairing invariant (both sides ascend in radix order, one lookahead slot suffices), names both failure directions, and derives the lazy-claim consequence. `encode::opening`'s doc explains why publish-before-supply is required rather than narrating the two calls beneath it.
- `payload_depth_limits_match` (start.rs:248-257) documents the rejected alternative (negotiation) and the concrete reason it fails; `run_budget` saturates on a 32-bit `usize` instead of panicking (start.rs:277), the preferred behavior at a tolerated corner.
- `queues.rs` justifies two one-line constructors by being the one place each edge's capacity rationale lives, with the occupancy derivation deferred rather than restated.
- No design-doc or agent-note citation anywhere in the partition, and no em-dash in any error, expect, or assert string (both grep-verified).
- The surrounding suite is strong where it counts: `run_to_quiescence` is a deterministic deadlock witness, the proptests run under adversarial channel schedules and one-byte transports, `transport_failures_are_exact_and_fail_fast` sweeps every I/O surface, and `declarations.rs` covers the greeting-declaration matrix in both election directions.

## Open questions for Finch

- Arity bundling (remote-proxy-4). The inline rationale "one premise per argument" (start.rs:338-339, work.rs:101-102) argues against a named bundle for the greeting-derived premises, while four `too_many_arguments` allows on one dataflow and a duplicated `budget` field argue for one. Recommendation: after remote-proxy-7 collapses `connected`/`open` into the role entry points, name the surviving premise set (`Negotiated { window, budget, peer_version_bytes, peer_set_len, peer_listing, codec }`) and drop `Session::budget`; the "premise" reading survives as the struct's field list.
- `send_or_cancel` (proxy.rs:11-16) parks forever when the consumer is gone, whereas the walk's `pump` returns `Ok(())` on the same condition (materialized/work.rs:147-149). The plausible reason is that proxy tasks own transport streams whose early drop would present the peer with a torn stream (`cancelled()`'s doc: "Retain cancellation-sensitive resources until their owner is dropped"). Recommendation: state that reason in `send_or_cancel`'s doc; if it is not the reason, the two participants should agree.
- `Progress` is a ZST field threaded through production signatures, while the walk uses `#[cfg(test)]` parameters and a `trace_id` field. Recommendation: keep the proxy's shape (it costs nothing and avoids `cfg` in signatures) and consider moving the walk toward it; no action in this partition.
- `Descending.early: Option<StreamReceiver<A::Rx>>` is `Some` only for the first initiator-representing stage and `debug_assert!`ed absent thereafter (state.rs:375-378). A type-level marker on the first `Descending` deletes the `Option` and the assert at the cost of a second `Reply` impl. Recommendation: fold into remote-proxy-7's refactor if the second impl stays small; otherwise leave the `Option` and delete the assert alone.
- Error taxonomy (remote-proxy-2, remote-proxy-3): nest the local-protocol variants under one `LocalProtocol` variant and move `PayloadDepthMismatch` to a handshake-level result, or take the doc-only route within the PR #38 parsimony directive. Recommendation: the structural route, pre-release, since both changes make the public enum say what the model of record says (a conformance-bug detector naming whose bug it caught).
- The illumos gate run (remote-proxy-22) is a real practice recorded only in commit eb4e0e1ba's message. Recommendation: one line in AGENTS.md's Commands section naming it, so the eight comments can cite something.
- `futures-util` is a direct dependency alongside `futures` (Cargo.toml:139-140), and start.rs:219 uses `futures_util::future::try_join` where work.rs imports from `futures`. Crate-wide; recommendation: one dependency, `futures`.
- `Handshaking`'s `Connect`/`CompleteConnect` impls (proxy as client, receive-before-send) are exercised only by the test topology; production always makes the proxy the server. Two proxy-clients paired would both wait to receive first. Recommendation: a one-line doc on the impls saying the client role is a test topology.
- Em-dashes in `//` comments (remote-proxy-5) are crate practice at 73 sites. Recommendation: one crate-wide sweep, or a decision that comments may keep them.

## Dropped

- [12] Session::incoming/outgoing docs name a nonexistent parameter: duplicate of [21]; merged into remote-proxy-8.
- [20] Payload-depth rationale at four sites: folded into remote-proxy-4, whose shared helper leaves one call site; the history pass corrected the message.rs:108-112 cite (it is `EncodeError`'s admission doc, not a fifth copy).
- [27] `too_many_arguments` allow placement: folded into remote-proxy-4 (the sites it names are the ones that refactor reshapes).
- [38] Session premises through four allow sites, budget duplicated: overlaps [1]; the arity part contests an inline rationale, so it is an open question rather than a finding; the `budget` duplication is noted in remote-proxy-4.
- [39] Fully qualified channel::Sender, [40] stray PayloadCodec import group, [41] pin!(Box::pin): duplicates of [8] and [9]; merged into remote-proxy-11.
- [42] "the peer's payload codec": folded into remote-proxy-10 as the wording to use when the codec doc collapses to one home; history shows "peer" means the `Peer` type.
- [43] Per-reply backend and ledger clones: folded into remote-proxy-24 as a related cost; the adapter signature is another partition's.
- [26] three of five word choices dropped: "codec seam" is anchored vocabulary in stats.rs:6, 29, 122 (refuted); "unsound" is the owner's ruling wording (71de90c11, payload-depth decision record); "load-bearing" is crate idiom at eleven sites. "acknowledged" and "register" survive as remote-proxy-17.
- [18] "five copies drift" framing: reframed by the refutation pass (the other restatements sit at distinct altitudes and show no drift); the arm-comment gap survives as remote-proxy-13.
- [13]'s attribution of "greeting frames" to 4dd2053c9: corrected by blame to 0aa29ed94; the finding survives in remote-proxy-1.
- [32] downgraded from verification-gap to a documentation nit (remote-proxy-26): the walk's supply-ordering violations and the ledger catch the duplication one layer down.
- [34] mechanism corrected by the refutation pass (the `scopes` ledger window opens at one derived scope, not two); survives as remote-proxy-20.
- perfapi's `#[must_use]` on `Handshaking` builders: below the bar; neither type is reachable outside the crate.
- Refutation's new observations 1 and 2 (`pub` constructors with two callers; `debug_assert!(early.is_none())`) folded into remote-proxy-7; observation 3 (`debug_assert!(asked.is_empty())`) into remote-proxy-25; observation 4 (copy count) into remote-proxy-22; observation 5 into remote-proxy-3 and remote-proxy-29.
