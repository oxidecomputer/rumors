# Partition remote-adapter-streams: The remote adapter (decode/encode/scope/errors) and the data-stream supply

## Partition summary

This partition is the two lowest layers of the streaming mirror's remote proxy, at commit 9e5784fb. `adapter/*` converts between the walk's height-erased `Reply<E>` (backend node handles, positional `Match`/`Query`, radix-keyed `Supply`, prefix omitted) and the wire's prefix-free frames. `encode.rs` renders one reply as a one-frame-lookahead stream of `Encoded` frames so the reply-ending `Flow::End` lands on the last frame, flattens each supplied node through `Backend::leaves` into byte-budgeted `LeafRun`s, and attaches each newly asked `Scope` to the frame whose successful write makes it publishable (`Encoded::write_with`). `decode.rs` reads exactly one reply (or the initiator's single opening-supply reply, incrementally, in `early_supplies`), recomputes every supplied leaf's path from its version, enforces scope containment, strict path order, strictly ascending run radices, and the peer's greeting-declared `max_version_bytes` and `set_len` in `SupplyRuns::observe` and the `SupplyLedger` charge, hands each leaf to `Leaf::leaf`, and streams leaves through a FAN-slot channel into `Backend::assemble` while retaining only a reply skeleton that `reify` fills afterwards. `scope.rs` is the retained question (parent prefix plus positional radices; the prefix's byte length is the height witness). `error.rs` is the typed rejection taxonomy. `streams.rs` binds the protocol's logical streams one to one onto a `Link`'s transport streams: `StreamSender` connects lazily on its first frame and writes a two-head CBOR label, `AcceptDriver` is the sole acceptor reader that validates labels and delivers streams into take-once oneshot claim slots, `StreamReceiver` claims lazily on first poll and yields frames until the `End::Stream` control, and every incoming failure is published to a one-slot `ErrorRoute` and parked, with a separate deposit slot for the acceptor's own transport failure that the session terminal consumes.

I read all seven files in full with line numbers, 2542 lines. `streams/tests.rs` (571 lines) is the test code; `decode.rs` also carries the `#[cfg(test)]` `fan_probe` module (lines 550-598) and `encode.rs` a `#[cfg(test)]` `Encoded::into_parts`. The lens reports, refutation pass, and history pass were read whole; every anchor and excerpt below was re-checked against the files at this commit, and the related sites outside the partition (proxy/work/encode.rs, pump.rs, work.rs, prefix.rs, erased.rs, backend.rs, local.rs, convert.rs, conformance/backend.rs, window.rs, cbor.rs, link.rs, codec/*.rs, stats.rs, observe.rs, the adapter test suites, and the agent notes the history pass cited) were read where a verdict rests on them.

The partition is in good shape. The types do the protocol's work where it matters: `Encoded::write_with` makes "wire before internal publication" the only order the API admits; `ReplyFrame` keeps the stream lifecycle control out of `StreamSender::frame`'s signature; the take-once claim slots make head-of-line coupling structurally absent. Peer input is judged in one place (`SupplyRuns::observe`) with typed errors, the declared-`set_len` charge lands before custody on both decode paths, and nothing reachable from wire bytes panics. The maintainer prose states invariants and the why at the branches that need them, and the hazards that bite (`StreamSender::frame`'s cancel safety, `AcceptDriver`'s detection latitude, the supply-failure deferral) are argued where the code is. Every testdoc in `streams/tests.rs` is accurate except one.

The dominant issues are residue and duplication, not misdesign. Two structural findings matter most: the height-erasure commit collapsed `Scope` to one type but left the `Q`/`N`/`D` type parameters and four copies of the scope-derivation rule behind (F5), and `read_early` re-implements `read_reply`'s frame and record loops, including a verbatim five-line comment, when `read_reply` over an empty root scope already produces its exact rejections, which is how the encode side already handles early supplies (F3). One owner-gated structural item: `ReplyFrame`'s exclusion is a runtime `TryFrom` on frames the adapter never produces, leaving a public error variant no session can fire (F19). One owner-gated design question: the decode channel's capacity is justified as "load-bearing for liveness" in two places without a mechanism, and none is derivable from the code (F6). The verification gaps are real but bounded: the label parser's rejection arms have no committed test, `early_supplies`' post-error withholding is not pinned, and the backend-contract asserts are promised in docs but never demonstrated to fire; two of these were already dispositioned in the scope-A mutation campaign and not yet landed. The rest are low-severity state-machine simplifications in `streams.rs`, doc lines one layer behind today's code, and idiom nits.

## Findings

### remote-adapter-streams-1: Adapter module doc describes the pre-erasure `Reply<B, H>` boundary and names `Convert::assemble` where `Backend::assemble` runs
- Where: src/tree/mirror/streaming/remote/adapter.rs:4-14 (related: src/tree/mirror/streaming/remote/adapter.rs:52, src/tree/mirror/streaming/remote/adapter.rs:56-58, src/tree/mirror/streaming/remote.rs:59, src/tree/mirror/streaming/remote/adapter/scope.rs:5-7, src/tree/mirror/streaming/remote/adapter/decode.rs:14, src/tree/mirror/streaming/remote/adapter/decode.rs:88, src/tree/mirror/streaming/remote/adapter/encode.rs:84, src/tree/mirror/streaming/erased.rs:319-334, src/tree/mirror/streaming/backend.rs:198-203, src/tree/mirror/streaming/backend/local.rs:191)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `Reply<B` across remote/; read the entry-point signatures, `erased::ops::assemble`, `Backend::assemble`'s default body, and `Local`'s override)
- Seen by: structure ([13]), prose ([18], [19]); refutation: confirmed (all three); history: deliberate-but-expired (d8bef16b's diffstat excludes adapter.rs; 48bc31df edited the diagram line, dropped `T`, kept `H`; 88a76a71 added `Backend::assemble` over the `Convert::assemble` the doc still names)
- Owner-gated: no

The module doc links `Reply` to the height-typed `message::Reply`, draws the boundary as `Reply<B, H>`, speaks of grouping leaves by "their height-`H` prefix", and says records flow "into the existing `Convert::assemble` fold". Every adapter entry point takes or returns the erased form (`decode.rs:14` imports `erased::{Reaction as ProtocolReaction, Reply, ops}`; `encode_reply` takes `reply: Reply<B::Erased>` at encode.rs:84; `decode_reply` returns `Decoded<B::Erased, Vec<Scope>>` at decode.rs:210), the height is a runtime witness recovered from the scope's prefix length (which scope.rs:5-7 states), and the decoder calls `ops::assemble` (decode.rs:88, 408), which dispatches to `Backend::assemble` (erased.rs:327-331); `Convert::assemble` is only that method's default body (backend.rs:202) and `Local` overrides it (local.rs:191), so under the in-memory backend the named fold never runs. Principle 5: a document reads as if written today against today's code; "existing" is a dated qualifier in one word.

Evidence:

         4	//! levels. In memory, one [`Reply`](super::super::message::Reply) contains
        ...
        11	//! ```text
        12	//! Reply<B, H> -- encode + explode --> Frame leaves
        13	//! Reply<B, H> <-- decode + assemble -- Frame leaves
        14	//! ```
        ...
        52	//! height-`H` prefix. Strict path and run ordering make those group
        ...
        56	//! The decoder yields each record as it is decoded through a fan-bounded
        57	//! channel into the existing
        58	//! [`Convert::assemble`](super::super::convert::Convert::assemble) fold,

Resolution: Link `Reply` to `erased::Reply`, draw the diagram as `Reply<E>` (or `erased::Reply<B::Erased>`) with one clause saying the height rides as the retained scope's prefix length; reword line 52 to "the prefix at the scope's children height"; rewrite 56-58 as "through a fan-bounded channel into [`Backend::assemble`] (dispatched at the scope's children height by `erased::ops::assemble`)", dropping "existing", and apply the same fix to remote.rs:59 ("the backend's existing conversion fold"). Acceptance: no `Reply<B, H>` or bare `H` parameter remains in adapter.rs prose; the `Reply` intra-doc link resolves to the erased type the signatures use; adapter.rs names `Backend::assemble` for the decode side as it names `Backend::leaves` (line 44) for the encode side; `grep -n existing` over adapter.rs and remote.rs is empty.

### remote-adapter-streams-2: `early_supplies`' post-error withholding is stated in a comment but no committed test can distinguish it from `try_collect`
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:109-113 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:83-131, src/tree/mirror/streaming/remote/adapter/tests/opening.rs:153-165, src/tree/mirror/streaming/remote/adapter/tests/opening.rs:203-211, src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:136-146, src/tree/mirror/streaming/remote/proxy/work/pump.rs:417-431)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; every `early_supplies` consumer in `opening.rs` and `fan_occupancy.rs` is `try_collect`, confirmed by grep)
- Seen by: correctness ([28]); refutation: reframed (the protected hazard is the assembler's end-of-input flush of the truncated final group, not the withheld complete group); history: no-rationale-found (the mechanism has been inline since 55d76d5c; the test gap is unrecorded)
- Owner-gated: no

The joint reader/assembler poll returns `Ready(None)` the moment the reader has failed, so nothing the assembler still holds is yielded. What that protects: when `read_early` returns `Err`, its `leaves` sender drops, the channel looks cleanly ended, and `ops::assemble` flushes the group still in assembly as if complete (backend.rs:191: "one node per maximal run"), so the truncated final group would otherwise surface as a node. Every committed consumer uses `try_collect`, which stops at the first `Err` regardless, so a refactor that polled `assembled` before consulting `read_result` (yielding the flushed group ahead of the error) passes the whole suite while handing `Early::advance_to` a node the reader never finished vouching for. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

       109	                // A reader error poisons everything after it: the group in
       110	                // assembly may be a truncation, so nothing more is yielded.
       111	                if matches!(read_result, Some(Err(_))) {
       112	                    return Poll::Ready(None);
       113	                }

Resolution: Add a test to `adapter/tests/opening.rs` built like `opening_supplies_decode_by_radix_group` (three first-byte groups a < b < c, one supply frame each), a paced source (`.then(|f| async { yield_now().await; f })` as `fan_occupancy.rs:172-175` does), and a failure inside c (a `SupplyLedger::new(|a| + |b| + 1)` allowance, or a repeated record for `LeafOrder`); drive with `next()` and assert the exact sequence `[Some(Ok((a, _))), Some(Err(_)), None]`. Acceptance: the new test fails when the two statements in the `poll_fn` closure are reordered so `assembled` is polled before `read_result` is checked, and passes at HEAD.
Construction: Under a current-thread runtime with the paced source the schedule is deterministic: poll 1 reads a's frame and pends; poll 2 reads b's frame, the assembler consumes a's leaves and b's first leaf and yields a; poll 3 reads c until the failing record, `read_result` becomes `Some(Err)`, and the closure returns `Ready(None)` before the assembler is polled again, so b (complete in the assembler) and c (flushed by the dropped sender) never surface. With the statements reordered, poll 3 polls the assembler first, which pulls the remaining channel contents and yields b before the error is consulted.

### remote-adapter-streams-3: `read_early` re-implements `read_reply`; `read_reply` over an empty root scope already produces its exact rejections
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:136-200 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:68-131, src/tree/mirror/streaming/remote/adapter/decode.rs:306-392, src/tree/mirror/streaming/remote/adapter/decode.rs:396-414, src/tree/mirror/streaming/remote/adapter/decode.rs:550-560, src/tree/mirror/streaming/remote/proxy/work/encode.rs:153-160, src/tree/mirror/streaming/erased.rs:319-323)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (traced both loops arm by arm against `Scope::new`/`Scope::next` at scope.rs:20-26, 40-44; not executed)
- Seen by: structure ([1]), correctness ([31]), perfapi ([36]), prose ([24], the decode.rs pair); refutation: confirmed ([1], [36]), reframed ([31]: the missing `debug_assert!` is not drift but a copy made without it: 8e1ed47a7 added the assert to `read_reply` five days before 55d76d5cf wrote `read_early`); history: no-rationale-found (55d76d5c explains why `early_supplies` yields incrementally, never why the frame reader is a second copy; 6bac3e1a noticed the twin shape and extended the fan probe to both copies instead of unifying; 08f2899b then threaded the ledger into both)
- Owner-gated: no

`read_early` (136-200) duplicates `read_reply`'s frame loop and its per-record admission (157-176 against 359-383: `records(codec)`, `observe`, `ledger.charge(1)`, `Leaf::leaf`, the test probe, the channel send), including the five-line "set-length half of the greeting's priced premises" comment verbatim at 160-164 and 367-371. With `Scope::new(parent, &[])` and the interior question closure, `read_reply` produces exactly `read_early`'s outcomes: `Match` fails `scope.next()` (empty children) as `UnpositionedMatch` (339 against 182-184); `Query` fails the closure's `scope.next()` as `UnpositionedQuery` (344, 222 against 185-187); `End(Reply)` with an empty skeleton breaks and after a reaction is `BareEndAfterReaction` (327, 329 against 188-189; the first supply record of a reply always starts a run, so `skeleton.is_empty()` and `!any` agree on every wire-reachable input); `End(Stream)` is `UnexpectedStreamEnd` (328 against 190); stream exhaustion is `TruncatedReply` (322-324 against 151-153). The encode side already does this: proxy/work/encode.rs:154-159 renders the early supplies with `encode_reply(..., Scope::opening(&[]), Reply { replies: supplies })`. The one behavioral difference is the in-process-only empty-run `debug_assert!` (352-355) that `read_early` lacks, which the unification closes. Separately, `early_supplies` (83-92) and `assemble_supplies` (404-408) both hand-build the `ReceiverStream` + `#[cfg(test)] inspect(fan_probe::on_recv)` + `Box::pin` + `ops::assemble` pipeline, and `ops::assemble` already returns `Pin<Box<dyn Stream + Send>>` (erased.rs:323), so both `pin!`s (88, 408) are redundant. Two copies of one trust-boundary step are two places every future premise must land; the encode/decode asymmetry is the tell that the bespoke reader is incidental.

Evidence:

       157	                for record in records.records(codec) {
       158	                    let (version, message) = record.map_err(DecodeError::Record)?;
       159	                    let (leaf_prefix, _) = supplies.observe::<B::Error>(parent, &version)?;
       160	                    // The set-length half of the greeting's priced
       161	                    // premises, charged per record before the payload
       162	                    // takes backend custody: a peer supplying past its
       163	                    // declaration fails at the offending record, while
       164	                    // the reply is still open.
       165	                    ledger
       166	                        .charge(1)
       167	                        .map_err(|declared| DecodeError::OverdrawnSupply { declared })?;
       168	                    let leaf = <B::Node<Z> as Leaf>::leaf(version, message)
       169	                        .await
       170	                        .map_err(DecodeError::Backend)?;
       171	                    #[cfg(test)]
       172	                    fan_probe::on_send();
       173	                    if leaves.send(Ok((leaf_prefix, leaf))).await.is_err() {
       174	                        return Ok(());
       175	                    }
       176	                }

    (decode.rs:359-383 is the same sequence with `read.supplies` for `supplies` and a skeleton push at 364-366; the comment at 367-371 is byte-identical to 160-164.)

Resolution: Replace `read_early`'s body with `let read = read_reply::<B, _, _, Scope>(version_bytes, ledger, Scope::new(parent, &[]), &mut frames, <the interior question closure>, leaves, codec).await?; if read.is_none() { return Ok(()); } if frames.next().await.is_some() { return Err(DecodeError::ExtraOpeningReply); } Ok(())`, discarding the skeleton and (necessarily empty) questions. Extract `fn assembly<B>(backend: B, height: usize, rx: mpsc::Receiver<..>) -> Pin<Box<dyn Stream<Item = Result<(ErasedPrefix, B::Erased), B::Error>> + Send>>` holding the `ReceiverStream`/probe/`ops::assemble` setup; `assemble_supplies` becomes `assembly(...).map_err(DecodeError::Backend).try_collect().await` and `early_supplies` pins the same helper; drop both `pin!`s. The `#[cfg(test)]` probe hooks fall from four sites to two, and the `fan_probe` module doc (553) no longer needs to say both paths "hook the same counter". Acceptance: `read_early` is gone or is a handful of lines delegating to `read_reply`; `ledger.charge(1)` and `<B::Node<Z> as Leaf>::leaf` each appear once in decode.rs; `adapter/tests/{opening,malformed,fan_occupancy}.rs` pass unchanged (they pin `UnpositionedMatch`/`UnpositionedQuery`/`BareEndAfterReaction`/`ExtraOpeningReply` on the early stream and the FAN + 1 occupancy ceiling on both channels).

### remote-adapter-streams-4: The peer's decode premises (`version_bytes`, `ledger`, `codec`) travel as three loose parameters through six signatures and two structs
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:203-210 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:68-75, src/tree/mirror/streaming/remote/adapter/decode.rs:231-238, src/tree/mirror/streaming/remote/adapter/decode.rs:261-269, src/tree/mirror/streaming/remote/proxy/work/pump.rs:184-190, src/tree/mirror/streaming/remote/proxy/work/pump.rs:198-205, src/tree/mirror/streaming/remote/proxy/work/pump.rs:227-234, src/tree/mirror/streaming/remote/proxy/work/pump.rs:290-297, src/tree/mirror/streaming/remote/proxy/work/pump.rs:384-391, src/tree/mirror/streaming/remote/proxy/work/pump.rs:417-430, src/tree/mirror/streaming/remote/proxy/work.rs:54-65, src/tree/mirror/streaming/remote/streams.rs:423-431)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read every pump.rs and work.rs site listed; the trio is read together at pump.rs:184-187, passed unchanged at four call sites, and re-stored in `Early` at 417-430)
- Seen by: structure ([7]), perfapi ([35]); refutation: confirmed ([7]), reframed ([35]: the stream-side half is only partly right, since sender and receiver contexts overlap on speaker/stats/observe only); history: no-rationale-found (each parameter was threaded on its own: 165b0dd3, 08f2899b, 4356e197; the one nearby arity argument, proxy/work/encode.rs:77-78, concerns per-edge dataflow, not shared constants)
- Owner-gated: no

`version_bytes: u64`, `ledger: SupplyLedger`, and `codec: PayloadCodec` are per-session constants derived from the peer's greeting. They are threaded together through `early_supplies`, `decode_reply`, `decode_leaf_reply`, `decode`, `read_reply`, and `read_early`, read together from `Work` (pump.rs:185-187), passed unchanged at four call sites, and stored together again in `Early` (pump.rs:417-430), each with its own copy of the same field doc. Three values always created, cloned, and consumed together are one thing without a name; a bare `u64` in the middle of a six-argument list is the types-first smell. The steelman at proxy/work/encode.rs:77-78 ("bundling edges into a struct would only rename the arity") argues against bundling channel edges; these are shared constants, so it does not apply. On the stream side, `read_frames` carries `#[allow(clippy::too_many_arguments)]` (streams.rs:423) for seven positional parameters of which five (speaker, budget, route, stats, observe) are session-scoped; a receiver-side context would drop the allow, but the sender shares only three of them, so one shared struct is less clean than the decode-side bundle.

Evidence:

       203	pub async fn decode_reply<B, F>(
       204	    backend: B,
       205	    version_bytes: u64,
       206	    ledger: SupplyLedger,
       207	    scope: Scope,
       208	    frames: &mut F,
       209	    codec: PayloadCodec,
       210	) -> Result<Decoded<B::Erased, Vec<Scope>>, DecodeError<B::Error>>

Resolution: Define a small `#[derive(Clone)]` struct in the adapter (the greeting's priced premises plus the peer's payload codec) with the three fields and their existing per-field docs; `Work` constructs it once; `Early` holds one field; `decode_reply(backend, premises: &Premises, scope, frames)`; `SupplyRuns::new(premises.version_bytes)`. Optionally a receiver-scoped context in streams.rs for `read_frames`. Acceptance: each adapter decode entry point takes four or fewer parameters; `Early` stores one premises field; the pump call sites shrink; adapter and proxy tests pass unchanged.

### remote-adapter-streams-5: `Encoded<Q>`, `Decoded<E, Q>`, `decode<.., Q, N>`, `render<.., D>` and four scope-derivation closures are height-erasure residue
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:261-273 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:28-31, src/tree/mirror/streaming/remote/adapter/decode.rs:221-224, src/tree/mirror/streaming/remote/adapter/decode.rs:249-255, src/tree/mirror/streaming/remote/adapter/encode.rs:21-24, src/tree/mirror/streaming/remote/adapter/encode.rs:45, src/tree/mirror/streaming/remote/adapter/encode.rs:94-106, src/tree/mirror/streaming/remote/adapter/encode.rs:125-140, src/tree/mirror/streaming/remote/adapter/encode.rs:144-156, src/tree/mirror/streaming/remote/adapter/encode.rs:183, src/tree/mirror/streaming/remote/proxy/work/encode.rs:176-190, src/tree/mirror/streaming/remote/proxy/work/encode.rs:217-230)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`git grep -n 'Encoded<\|Decoded<\|Frames<' -- src/` at HEAD: every instantiation is `Scope` or `Vec<Scope>` (encode.rs:85, 116, 150; decode.rs:210, 238), and proxy/work/encode.rs:176-190 and 217-230 carry a `Q` parameter that only ever binds `Scope`; the pre-erasure signatures are as the history pass quotes from `git show d8bef16b^`)
- Seen by: structure ([0]), prose ([24], the encode.rs comment pair); refutation: confirmed; history: deliberate-but-expired (the `Q`/`N`/`D` parameters existed so one body could map `Scope<S<H>>` to `Scope<H>` at every height; d8bef16b erased `Scope` to one type and carried the closure shape forward without mention)
- Owner-gated: no

Before erasure, `Scope<H>` was height-typed and the generic carriers were how one body served every height. `Scope` is now one type, so `Encoded<Q>`, `Decoded<E, Q>`, `Frames<E, Q>`, `N`, and the `D`/`Q` closure parameters have exactly one instantiation each, and the closures at decode.rs:221-224 and 249-255 and encode.rs:94-106 and 125-140 spell the same two rules four times (interior: `scope.next()` then `Scope::new(prefix, listing)`; leaf: reject nonempty, `scope.next()` then `Scope::leaf(prefix)`), with the encode pair also duplicating the `Match` arm and its "Symmetric with decode" comment (96-97, 127-128). The generics ripple into `write_reply<C, Q, E>` and `write_encoded<C, Q, E>` and force `Send + 'static` bounds on `D`. A type parameter earns its place by varying; these vary nowhere. Circular justification is the tell, and the erasure commit is exactly the phase boundary where the audit recurs. Four copies of one rule are four places for the encode/decode symmetry the module doc leans on to drift.

Evidence:

       261	async fn decode<B, F, Q, N>(
       262	    backend: B,
       263	    version_bytes: u64,
       264	    ledger: SupplyLedger,
       265	    scope: Scope,
       266	    frames: &mut F,
       267	    question: Q,
       268	    codec: PayloadCodec,
       269	) -> Result<Decoded<B::Erased, Vec<N>>, DecodeError<B::Error>>
       270	where
       271	    B: Backend<Node<Z>: Leaf>,
       272	    F: Stream<Item = Frame> + Unpin,
       273	    Q: FnMut(&mut Scope, &[(u8, Hash)]) -> Result<N, ScopeError>,

        21	pub struct Encoded<Q> {
        22	    frame: Frame,
        23	    question: Option<Q>,
        24	}

Resolution: Introduce a two-variant `enum Level { Interior, Leaf }` (or two named functions with one signature) and one `fn derive_question(level: Level, scope: &mut Scope, listing: &[(u8, Hash)]) -> Result<Scope, ScopeError>` used by both `render` and `read_reply`; `render` handles `Match` and `Supply` itself as `read_reply` already does, so the `debug_assert!(question.is_none())` at encode.rs:183 dissolves (a `Supply` structurally derives no question). Make `Encoded { frame, question: Option<Scope> }`, `Decoded<E> { reply, questions: Vec<Scope> }`, `Frames<E>`; keep `encode_reply`/`encode_leaf_reply`/`decode_reply`/`decode_leaf_reply` as thin wrappers passing the level. Drop `Q` from `write_reply`/`write_encoded` in proxy/work/encode.rs. Acceptance: no type parameter in adapter/{encode,decode}.rs has a single instantiation; `git grep -n 'Symmetric with decode'` returns nothing; the scope-derivation rule appears once; `adapter/tests/{properties,malformed,opening}.rs` pass unchanged except for `into_parts` tuple types.

### remote-adapter-streams-6: The decode channel's capacity is justified as "load-bearing for liveness" in two places with no mechanism named, and none is derivable; the pull-based alternative is a measure-first proposal
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:278-291 (related: src/tree/mirror/streaming/window.rs:129-132, src/tree/mirror/streaming/window.rs:390-399, src/tree/mirror/streaming/window.rs:178-192, src/tree/mirror/streaming/remote/adapter/decode.rs:83-131, src/tree/mirror/streaming/remote/adapter/decode.rs:380, src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:130-183, src/tree/mirror/streaming/backend/local.rs:201-221, src/tree/mirror/streaming/convert.rs:79-92)
- Class / severity / confidence: performance / medium / medium
- Provenance: assessed (read both `assemble` implementations and the joined drive; verified the history mechanically: `git show 8aeed2dd3 -- decode.rs` shows the sentence "it exists to amortize the reader/assembler waker round trip" replaced by "The capacity is load-bearing for liveness", with no liveness argument in the commit)
- Seen by: perfapi ([33]); refutation: reframed (no stall derivable from the joined-drive shape; settling it needs a sub-FAN construction, a code change; the pull-based reader is a separate backend-dependent trade); history: no-rationale-found for the liveness sentence (8aeed2dd's message discusses custody and pricing, never liveness); the pricing half is grounded (d6537a7b pinned FAN + 1 as a never-exceeds premise)
- Owner-gated: yes: the window's supply-decode pre-charge term (window.rs:397-399) would move, and the reader/assembler shape is a documented design

The comment at 280-282 and window.rs:395-396 say the FAN-slot capacity "is load-bearing for liveness" and that "no configuration may shrink it", and window.rs:129-131 calls it a "hard capacity floor". No mechanism is named. Both halves are driven by one task (`futures::future::join` at 291; the `poll_fn` at 103-116), `ReceiverStream` drains one item per poll, `Local::assemble` (local.rs:205-217) and the `fold_parents` chain (convert.rs:83-91) each pull one leaf at a time, so a full channel yields to the drain and progress does not depend on the capacity for any capacity of at least one; capacity governs wakeups (the amortization the previous comment stated) and read-ahead, not progress. Meanwhile the channel's residency is pre-charged into every session budget as the flat supply-decode envelope (`STREAM_COUNT * (FAN + 1) * ...`, window.rs:191-192, 397-399), every decoded reply, supply-free disputes included, allocates a FAN-slot channel plus two boxed streams and a `join`, and every supplied record pays a bounded-channel send/recv pair. A stated invariant whose only defense is itself is the circular-justification tell; a number that governs a budget term is a hypothesis until its premise is named. Steelman of the channel: one fan of read-ahead lets a slow persistent `assemble` overlap wire parsing; under `Local` it buys nothing. The sign is backend- and workload-dependent, so replacing it is measure-first (Principle 4).

Evidence:

       278	    // One fan of buffered leaves, amortizing the reader/assembler waker
       279	    // round trip over runs of consecutive leaves instead of paying it per
       280	    // leaf. The capacity is load-bearing for liveness: the channel must
       281	    // admit one full fan of records while the assembler holds a parent
       282	    // group open, so no configuration may shrink it. Its residency is
        ...
       288	    let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
       289	    let read = read_reply::<B, _, _, _>(version_bytes, &ledger, scope, frames, question, tx, codec);
       290	    let assemble = assemble_supplies::<B>(backend, children_height, rx);
       291	    let (read, assembled) = futures::future::join(read, assemble).await;

    window.rs:
       394	        // so `node_bytes(0, ·)` is its whole resident price). Width
       395	        // cannot shrink this term — the fan capacity is load-bearing for
       396	        // liveness — so it comes off the budget before the solve.

Resolution: 1. Settle the liveness claim: construct `decode` with capacity 1 (a test-only constant or a parameter) and drive an eager source of `4 * FAN` records under `Local`; if it completes, rewrite decode.rs:280-282 and window.rs:394-396 to the true rationale (waker amortization and one fan of read-ahead, priced as a never-exceeds ceiling pinned by `fan_occupancy.rs`), and if it stalls, name the stall in the comment and commit the construction as a test. 2. Then, as an owner decision, prototype the pull-based reader (`read_reply` as a stream of `(Prefix<Z>, B::Node<Z>)` the assembler polls directly, skeleton and questions parked in a slot read after assembly ends; `early_supplies` becomes the same stream fed to `ops::assemble`), measure at the parent commit and after on `benches/gossip_fixed.rs` for a supply-heavy and a supply-free workload plus a per-reply allocation meter, and if adopted re-derive the window's supply-decode term and replace the fan-occupancy pins with a meter for whatever residency the new shape has. Acceptance: either a committed test constructing the stall a sub-FAN capacity causes and a comment naming it, or the two comments state the amortization/read-ahead rationale and drop "liveness"; the pull-based reader lands only with before/after numbers and a re-derived pricing term.
Construction: For the liveness question only (no defect asserted): `mpsc::channel(1)` in `decode` with `stream::iter(frames(&leaves(4 * FAN)))` under `Local`, on the `fan_occupancy.rs` runtime; completion refutes the liveness claim for the joined shape.

### remote-adapter-streams-7: Em-dashes in `//` comments (two partition sites of a crate-wide pattern)
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:283-284 (related: src/tree/mirror/streaming/remote/streams.rs:445)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the seven files hits exactly these two sites; the same grep over src/, tests/, benches/, examples/ hits 169 lines)
- Seen by: prose ([23]); refutation: confirmed, scope widened to crate-wide; history: no-rationale-found (in-repo dash enforcement has been scoped to messages: link-transport review R76, a53f056e; the CLAUDE.md rule covers comments but the tree does not follow it)
- Owner-gated: no (but the fix is a crate-wide ruling, not a two-line edit)

The house rule is spaced double-hyphens in `//` comments, em-dashes only in rendered prose. Two `//` comments in the partition use true em-dashes; no assert, expect, panic, or `#[error]` message in the partition does. The pattern is crate-wide (169 lines), so the disposition is one decision: a sweep, or an explicit tolerance recorded where the rule lives.

Evidence:

       283	    // charged: each slot holds a backend-priced node — the payload's
       284	    // custody already passed to the backend at `Leaf::leaf` — and the

    streams.rs:
       445	            // wins selection — a report that loses the terminal's race must

Resolution: Rule crate-wide; if sweeping, replace ` — ` with ` -- ` in `//` comments (a mechanical sed over the 169 lines), leaving `//!` and `///` untouched. Acceptance: `grep -rn --include='*.rs' -E '^\s*//[^/!].*—' src/ tests/ benches/ examples/` is empty, or the tolerance is recorded.

### remote-adapter-streams-8: The version-bound rationale is stated three times (variant doc, field doc, ingress comment)
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:492-496 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:460-461, src/tree/mirror/streaming/remote/adapter/error.rs:84-98)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the three sites side by side)
- Seen by: prose ([24]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`OversizedVersion`'s variant doc (error.rs:86-94) states the priced premise in full; the `SupplyRuns::version_bytes` field doc (460-461) states it in one sentence; the comment at 492-496 restates it in five lines above the check. Three copies of one rationale drift independently, and the second and third show nothing the first did not. (The two other duplicated blocks the prose lens listed, the set-length charge comment at decode.rs:160-164/367-371 and the "Symmetric with decode" pair at encode.rs:96-97/127-128, dissolve with findings 3 and 5.)

Evidence:

       492	        // The declared aggregate covers every version the peer's tree
       493	        // materializes, so every version it supplies must encode within
       494	        // it; one arriving over the declaration voids the premise the
       495	        // window solve priced this session with, and fails the session
       496	        // before the record is admitted.

Resolution: Keep the full rationale on the variant; reduce 492-496 to a one-line pointer ("// Checked at ingress, before admission: see `DecodeError::OversizedVersion`."). Acceptance: the rationale has one authoritative home in error.rs and the check site points at it.

### remote-adapter-streams-9: `Prefix<Z>` to `[u8; 32]` spelled as a fallible `try_into().expect(..)` where an infallible `From` exists
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:520-526 (related: src/tree/typed/prefix.rs:18, src/tree/typed/prefix.rs:113-123, src/tree/mirror/streaming/remote/adapter/encode.rs:249-252, src/tree/mirror/streaming/remote/adapter/decode.rs:463)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (prefix.rs:18 declares `pub struct Prefix<H: Height = Z>`, so `impl From<Prefix> for [u8; 32]` at prefix.rs:119-123 is `From<Prefix<Z>>`; `previous_leaf` is `Option<Prefix<Z>>` at decode.rs:463; encode.rs:250 already uses `Path::from(current)` on the same type)
- Seen by: structure ([5]); refutation: confirmed; history: no-rationale-found (the `expect` dates to 00b29d32, when a two-step infallible route via `Path` already existed; the one-step `From` arrived in 9c73d7b4)
- Owner-gated: no

The wire-input rejection path converts a `Prefix<Z>` to `[u8; 32]` through a slice under an `expect`. The compiler already knows a `Prefix<Z>` is 32 bytes; re-deriving that at runtime trades an evidently correct conversion for one that needs a proof string. Types-first: the sanctioned use of `expect` is a one-line proof of programmer error, and here no proof is needed.

Evidence:

       520	            return Err(DecodeError::LeafOrder {
       521	                previous: previous
       522	                    .as_bytes()
       523	                    .try_into()
       524	                    .expect("a leaf prefix occupies a full content path"),
       525	                current: path.into(),
       526	            });

Resolution: `previous: previous.into(),`. Acceptance: no `try_into().expect` remains in decode.rs; `malformed.rs`'s `LeafOrder` pin passes unchanged.

### remote-adapter-streams-10: `SupplyOrder`'s "preceded" half is unreachable; `LeafOrder` fires first
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:530-539 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:507-528, src/tree/mirror/streaming/remote/adapter/error.rs:81-83, src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:685-719)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (traced `observe`: `previous_leaf`'s byte at `parent_len` equals `previous_radix` whenever both are `Some`, because every leaf either starts the run whose radix it carries or continues it, and `interrupt()` clears only `current`)
- Seen by: correctness ([30]); refutation: confirmed; history: no-rationale-found (the ordering of the two checks and the "reused or preceded" doc both date to 00b29d32; b3b72d248 names the live case as "the run-split-across-interrupt case")
- Owner-gated: no

`observe` checks strict leaf-path order (`previous_leaf >= leaf_prefix` at 516-527) before run detection. A supply whose radix is smaller than the previous run's radix, under the same parent, has a lexicographically smaller path than every leaf of that run, so `LeafOrder` fires first and the `previous > radix` half of `*previous >= radix` never executes. Only `previous == radix` with `current == None` (a run resumed after `interrupt()`) reaches `SupplyOrder`, which is exactly what `a_supply_run_cannot_resume_after_another_reaction` (malformed.rs:687-718) pins. The variant doc "reused or preceded an earlier run's radix" and the message "does not follow" over-promise: the message only ever renders two equal radices. Finished code should read as evidently right; a dead comparison invites the reader to look for a case that cannot occur.

Evidence:

       530	        let run = if self.current != Some(node_prefix) {
       531	            if let Some(previous) = self.previous_radix.filter(|previous| *previous >= radix) {
       532	                return Err(DecodeError::SupplyOrder { previous, radix });
       533	            }

    error.rs:
        81	    /// A later supplied run reused or preceded an earlier run's radix.
        82	    #[error("supplied radix {radix:#04x} does not follow {previous:#04x}")]
        83	    SupplyOrder { previous: u8, radix: u8 },

Resolution: Either narrow the filter to `*previous == radix` and reword the variant to the reachable case ("a supply run resumed a radix already closed by a positional reaction", message "supply run for radix {radix:#04x} resumed after another reaction"), or keep `>=` and add a one-line comment at 531 that the strict-descent case is rejected by the path-order check above. Acceptance: the `SupplyOrder` doc and message describe exactly the reachable case, or the check's comment names why `<` cannot arrive.

### remote-adapter-streams-11: The clippy-allow rationale names a gate platform the verification recipes do not name, and moralizes the lint flag
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:565-568 (related: rust-toolchain.toml:31, tests/common/wire.rs:22-25, and eight further copies of the same comment across the workspace per `grep -rn 'illumos among' src/ tests/ crates/`)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -rn illumos justfile .github/ rust-toolchain.toml Cargo.toml .cargo/ .config/` is empty; rust-toolchain.toml:31 is `targets = ["wasm32-unknown-unknown"]`; the comment recurs at ten sites)
- Seen by: prose ([21]); refutation: confirmed (eb4e0e1ba's message shows the claim is operationally true as a manual re-check on ox-east-1); history: deliberate-and-holds for the rationale (stated in eb4e0e1b), but propagated to ten copies (f924463a), and "honest" has no recorded rationale
- Owner-gated: no for the wording; the roster question (encode the illumos run in the recipes, or drop the platform claim) is an owner call (see open questions)

The comment justifies the allow by naming illumos as one of "the gate's targets". Nothing in the tree says so: the justfile, CI workflows, and the toolchain pin name only `wasm32-unknown-unknown`. The gate is run on illumos by hand, which is operational provenance a reader cannot corroborate from the tree (Principle 8). "keeps `-D warnings` honest" is moralized code for "clean". The fix is workspace-wide (ten copies), so a partition-local rewording would leave nine divergent copies.

Evidence:

       565	    // clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
       566	    // fallback-TLS lowering (illumos among the gate's targets) and denies
       567	    // initializers that already sit in `const` blocks; the allow keeps
       568	    // `-D warnings` honest on every platform the gate runs.

Resolution: State the platform-independent mechanism without the roster, at all ten sites: "clippy's `missing_const_for_thread_local` fires on `const {}` initializers when `thread_local!` lowers to fallback TLS; the allow keeps `-D warnings` clean under either lowering." If illumos is meant to be a gate platform, make it one in the justfile or CI so the claim becomes checkable. Deduplicating the ten copies (a shared `thread_local!` helper or a crate-level allow) is a separate question. Acceptance: the comment names no platform the verification recipes do not name, or the recipes name it; "honest" is gone from every copy.

### remote-adapter-streams-12: `render` repeats the flush-pending-then-yield block four times; two suffice
- Where: src/tree/mirror/streaming/remote/adapter/encode.rs:161-233 (related: src/tree/mirror/streaming/remote/adapter/encode.rs:105, src/tree/mirror/streaming/remote/adapter/encode.rs:139, src/tree/mirror/streaming/remote/adapter/encode.rs:183)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; the block appears at 163-170, 173-180, 212-219, 224-231, and the Supply arm's `question` is asserted `None` at 183 because both `derive` closures return `Ok(None)` for `Supply` at 105 and 139)
- Seen by: structure ([6]); refutation: confirmed; history: no-rationale-found (the Supply-arm copies came with f94f2056, whose message states the flush rule but not the code shape)
- Owner-gated: no

The `Match`, `Query`, mid-run `Supply` flush, and end-of-`Supply` arms each contain the same eight lines: `if let Some((previous, question)) = pending.replace((<wire>, <question>)) { yield Encoded { frame: Frame::Reaction(previous, Flow::Continue), question } }`. Because the Supply arm's final `replace` uses `None` as its question and `question` is `None` for `Supply`, all three per-reaction replaces are one statement over a `wire: WireReaction` the match can produce; only the mid-run flush must stay inside the leaf loop. `yield` cannot be factored into a helper inside `try_stream!`, which is why the repetition accreted, but the match can yield the wire reaction and one trailing replace-and-yield can follow it. The one-frame lookahead is the whole idea of `render`; it reads more clearly stated once than four times, and each copy is a place for the Continue/End discipline to drift.

Evidence:

       162	                ProtocolReaction::Match => {
       163	                    if let Some((previous, question)) =
       164	                        pending.replace((WireReaction::Match, question))
       165	                    {
       166	                        yield Encoded {
       167	                            frame: Frame::Reaction(previous, Flow::Continue),
       168	                            question,
       169	                        };
       170	                    }
       171	                }

Resolution: `let wire = match reaction { Match => WireReaction::Match, Query(listing) => WireReaction::Query(listing), Supply(radix, node) => { <leaf loop with the mid-run flush yield>; assert!(!run.is_empty(), ..); WireReaction::Supply(run) } }; if let Some((previous, question)) = pending.replace((wire, question)) { yield Encoded { frame: Frame::Reaction(previous, Flow::Continue), question }; }`. Acceptance: two `yield Encoded { frame: Frame::Reaction(.., Flow::Continue), .. }` sites in `render`; `adapter/tests/{runs,properties}.rs` (frame sequences and Continue/End placement) pass unchanged; wire snapshots unchanged.

### remote-adapter-streams-13: Assert and expect messages that name the thing rather than the proof
- Where: src/tree/mirror/streaming/remote/adapter/encode.rs:183 (related: src/tree/mirror/streaming/remote/streams.rs:522, src/tree/mirror/streaming/remote/streams.rs:555)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the three sites and their neighbors: decode.rs:427, 524, streams.rs:187, 225, 391, 615, 629 all carry a proof)
- Seen by: prose ([25]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`debug_assert!(question.is_none())` carries no message; the proof is that both `derive` closures return `Ok(None)` for `Supply`. `.expect("supply failure lock")` at streams.rs:522 and 555 names the lock, not why poisoning cannot occur (the only critical sections are a `get_or_insert` and a `take`, neither of which can panic). The rest of the partition's messages carry their proof, so the standard is evidenced in the same files. (Finding 5's resolution deletes the encode.rs site outright.)

Evidence:

       183	                    debug_assert!(question.is_none());

    streams.rs:
       522	        let mut slot = self.supply_failure.lock().expect("supply failure lock");

Resolution: encode.rs:183: `debug_assert!(question.is_none(), "a supply derives no question")` (or delete per finding 5). streams.rs:522, 555: `.expect("no holder of the deposit lock panics: its critical sections are a get_or_insert and a take")`, or `unwrap_or_else(PoisonError::into_inner)` and no message. Acceptance: every assert/expect in the partition has a message stating why it cannot fire.

### remote-adapter-streams-14: Backend-contract panics are promised in `Backend`'s docs but never demonstrated to fire, and the conformance suite does not check run order or containment
- Where: src/tree/mirror/streaming/remote/adapter/encode.rs:249-262 (related: src/tree/mirror/streaming/remote/adapter/encode.rs:223, src/tree/mirror/streaming/remote/adapter/decode.rs:292-300, src/tree/mirror/streaming/remote/adapter/decode.rs:417-441, src/tree/mirror/streaming/backend.rs:158-203, src/conformance/backend.rs:379-490, src/lib.rs:322, src/conformance.rs:18-19)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn should_panic src/tree/mirror/streaming src/conformance` hits only `materialized/progress/tests.rs`; read the conformance `leaves` wrapper (379-428: pricing, aggregate coverage, walked count) and `assemble` wrapper (430-490: runs keyed by `Prefix::<H>::containing` in a `BTreeMap`, leftovers and unsupplied nodes checked), neither of which examines order or containment)
- Seen by: correctness ([29]); refutation: reframed (only `Backend::assemble`'s doc cites the conformance suite; `Backend` is crate-private, so panic is the right shape and the finding is about demonstration, not error typing); history: already-known for `validate_leaf` (scope-A campaign dispositioned `validate_leaf → ()` as a misbehaving-backend adequacy family, not yet landed); the `reify` asserts and the decode.rs:299 `unreachable!` have no recorded disposition
- Owner-gated: no

`validate_leaf` asserts containment and strict path order for every leaf a backend enumerates, `render` asserts at least one leaf per node (223), `reify` asserts one assembled node per run at the run's prefix (425-439, which also enforces run order since the skeleton is walked in order), and `decode` has an `unreachable!` premised on the assembler consuming its whole input (292-300). `Backend::leaves`'s doc (backend.rs:162-163) says "the wire encoder enforces each by panic" and `Backend::assemble`'s (188-197) says "the reply decoder enforces each by panic" and that the conformance suite "convicts a violating override in the backend's own tests". The conformance `assemble` wrapper convicts merged, split, skipped, and wrong-prefix nodes but is order-insensitive, and its `leaves` wrapper checks counts and pricing, never order or containment. No `#[should_panic]` demonstration exists for any of the three encoder/decoder messages. A guard earns its place by naming a constructible failure it catches; every criterion needs a committed demonstration that a known-bad mechanism fails it. `Backend` is crate-private (`mod tree;` at lib.rs:322; `conformance::backend` is `#[cfg(test)] pub(crate)`), so these are programmer-error panics by the crate's own standard and panic is the right shape; what is missing is the demonstration and an accurate doc.

Evidence:

       249	fn validate_leaf(expected: ErasedPrefix, previous: Option<Prefix<Z>>, current: Prefix<Z>) {
       250	    let path = Path::from(current);
       251	    assert_eq!(
       252	        &<[u8; 32]>::from(path)[..expected.as_bytes().len()],
       253	        expected.as_bytes(),
       254	        "a backend enumerates leaves beneath the requested node prefix",
       255	    );
       256	    if let Some(previous) = previous {
       257	        assert!(
       258	            previous < current,
       259	            "a backend enumerates leaves in strict path order",
       260	        );
       261	    }
       262	}

    backend.rs:
       195	    /// The backend conformance suite (see [`crate::conformance`]) convicts
       196	    /// a violating override in the backend's own tests, before a live
       197	    /// session can meet it.

Resolution: Land the scope-A disposition: a test backend wrapping `Local` whose `leaves` override swaps two adjacent yields (or displaces one prefix) and whose `assemble` override drops the last node or ends early while leaves remain, with `#[should_panic(expected = ..)]` tests for each of the three encoder/decoder messages and the decode.rs:299 `unreachable!`. Either add order and containment checks to the conformance `leaves`/`assemble` wrappers so "convicts a violating override" is true for every listed clause, or narrow backend.rs:195-197 to the clauses the suite checks. Acceptance: a committed test panics with each message under a deliberately misbehaving backend; `Backend::leaves`/`assemble` rustdoc lists exactly the clauses that are enforced somewhere, and names where.
Construction: Wrap `Local` in a test backend whose `leaves` collects the inner stream, reverses two adjacent items, and re-yields; call `encode_reply` on a two-leaf node under `#[should_panic(expected = "strict path order")]`. For `reify`, an `assemble` override that drops its last yielded node reaches the `expect` at decode.rs:427; one that ends its stream after the first node while leaves remain reaches decode.rs:299.

### remote-adapter-streams-15: Mux-era vocabulary survives the mux's deletion: "demultiplexer" and "reintroduce"
- Where: src/tree/mirror/streaming/remote/adapter/error.rs:69-71 (related: src/tree/mirror/streaming/remote/streams.rs:4, src/tree/mirror/streaming/remote/streams.rs:640, src/tree/mirror/streaming/remote/streams.rs:478-482; outside the partition: src/tree/mirror/streaming/remote/codec.rs:32, src/tree/mirror/streaming/remote/codec/signal.rs:31)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'multiplex\|reintroduc'` over the partition and codec files: error.rs:69, streams.rs:4 ("Nothing multiplexes"), streams.rs:640, codec.rs:32, signal.rs:31)
- Seen by: prose ([22]); refutation: confirmed; history: deliberate-but-expired (error.rs:69 written in 24b187a8 when the mux/demux layer existed; b3b877d9 deleted it "outright"; the sweeps 1a07a090 and 339561aa fixed sibling sites and missed this one; streams.rs:640 was written in b3b877d9 naming the design it deleted)
- Owner-gated: no

`UnexpectedStreamEnd`'s doc says control "leaked through the demultiplexer", while the streams module doc opens "Nothing multiplexes" (streams.rs:4). `AcceptDriver`'s doc says a shared reader "would reintroduce" head-of-line coupling, a verb that parses only if the reader knows a mux once existed. Both sit under AGENTS.md's hard rule that nothing in the codebase refers to code that no longer exists. Since `StreamReceiver` consumes `End::Stream` (streams.rs:478-482), the variant is in fact reachable only from in-process frame streams, which the doc could say, as decode.rs:179-181 does for its sibling case.

Evidence:

        69	    /// Transport control leaked through the demultiplexer into reply decoding.
        70	    #[error("a stream-end control reached the protocol reply decoder")]
        71	    UnexpectedStreamEnd,

    streams.rs:
       640	/// shared reader would reintroduce is structurally absent. The driver runs

Resolution: error.rs:69: "A stream-end control reached reply decoding; [`StreamReceiver`] consumes it on the wire path, so this is reachable only from frame streams constructed in process." streams.rs:640: "would introduce". For the codec partition's owner: codec.rs:32 ("The session demultiplexer") and codec/signal.rs:31 ("Logical streams multiplexed into each transport direction"). Acceptance: `grep -n 'multiplex\|reintroduc'` over adapter/ and streams.rs matches only streams.rs:4.

### remote-adapter-streams-16: `Scope`'s positional radix list is a heap `Vec` per question; a 256-bit set makes `Scope` `Copy` and allocation-free
- Where: src/tree/mirror/streaming/remote/adapter/scope.rs:12-26 (related: src/tree/mirror/streaming/remote/adapter/scope.rs:40-44, src/tree/mirror/streaming/remote/adapter/scope.rs:55-67, src/tree/mirror/streaming/remote/adapter/encode.rs:103, src/tree/mirror/streaming/remote/adapter/decode.rs:223, src/tree/mirror/streaming/remote/codec/error.rs:55-61)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read; the precondition that wire listings are strictly ascending by radix is enforced by the codec's `QueryOrderError`, codec/error.rs:55-61, so a bitset walked in ascending order reproduces `Scope::next`'s positional order)
- Seen by: perfapi ([34]); refutation: confirmed; history: no-rationale-found (`children: Vec<u8>` dates to 00b29d32; d8bef16b rewrote `Scope`'s type parameters and doc and kept the representation without comment)
- Owner-gated: no (private type)

Every question created on either side (`Scope::new` from the encoder's `derive` and the decoder's question closures, plus `Scope::opening` and `Scope::leaf`) collects the listing's radices into a `Vec<u8>`: one heap allocation per disputed scope per peer, and `Scope` is `Clone` rather than `Copy`. The radices are strictly ascending bytes consumed positionally, which is a 256-bit set walked by a cursor. Strict deletion of redundant work with a fixed sign: a `[u64; 4]` plus a `u16` cursor holds the same information in 34 bytes inline, `next()` becomes a `trailing_zeros` walk over at most four words, `Scope` gains `Copy`, and the two proxy scope queues move a fixed-size value. Denominator: one allocation per question on each side; small beside the listing `Vec<(u8, Hash)>` the codec allocates for the same question, hence low.

Evidence:

        12	pub struct Scope {
        13	    parent: ErasedPrefix,
        14	    children: Vec<u8>,
        15	    next: usize,
        16	}
        ...
        20	    pub fn new(parent: ErasedPrefix, listing: &[(u8, Hash)]) -> Self {
        21	        Self {
        22	            parent,
        23	            children: listing.iter().map(|(radix, _)| *radix).collect(),
        24	            next: 0,
        25	        }
        26	    }

Resolution: Replace `children: Vec<u8>` with a `RadixSet([u64; 4])` newtype (`insert`, `next_above(cursor)`), keep the cursor as `u16`, derive `Copy`, keep `is_request` as `set.is_empty()`. Acceptance: `Scope` is `Copy`; `adapter/tests/properties.rs` and `malformed.rs` pass unchanged; a per-question allocation meter (or a `stats_alloc` inspection of `decode_reply` on a fan-wide `Query` reply) shows one fewer allocation per question.

### remote-adapter-streams-17: Stream count hand-written as "17" in prose where a named constant exists
- Where: src/tree/mirror/streaming/remote/streams.rs:3 (related: src/tree/mirror/streaming/remote/streams.rs:70-71, src/link.rs:161-169, src/tree/mirror/streaming/remote/codec/signal.rs:31-32; outside the partition: src/tree/mirror/streaming/remote.rs:8, src/tree/mirror/streaming/remote.rs:13, src/tree/mirror/streaming/remote/codec.rs:19)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read link.rs:161-169 with its `ceil(32 / 2) + 1 = 17` derivation and signal.rs:32 `pub const COUNT: u8 = 17;`; streams.rs:70-71 defines a private `STREAM_COUNT` from `Stream::COUNT`; grep for the literal in remote.rs and codec.rs)
- Seen by: structure ([14]), prose ([16]); refutation: confirmed (both); history: no-rationale-found (b3b877d9 wrote "17-per-direction" in the same commit that defined `STREAM_COUNT = 17`; 3327a92b writes "one of 17" fresh today, so the fix is a cross-file decision)
- Owner-gated: no

The module doc restates the per-direction stream count as a literal. The number is defined and derived in `link::STREAM_COUNT` (link.rs:165-169) and `Stream::COUNT` (signal.rs:32), pinned equal by test, and this file already names it two screens down. No hand-maintained counts: a number the code owns is cited by name from prose, never retyped where a wire-schedule change would orphan it. The same literal appears in remote.rs:8, 13 and codec.rs:19 (other partitions).

Evidence:

         3	//! This layer binds the protocol's 17-per-direction logical streams onto a

        70	/// Number of logical streams per direction, as an array dimension.
        71	const STREAM_COUNT: usize = Stream::COUNT as usize;

Resolution: "binds the protocol's [`Stream::COUNT`] logical streams per direction onto ..." (or link `crate::link::STREAM_COUNT`); likewise at the remote.rs and codec.rs sites. Acceptance: the literal 17 appears in prose only at its derivation site in link.rs; the intra-doc link resolves under `just docs`.

### remote-adapter-streams-18: Long qualified paths, function-local imports, an unimported `crate::Version`, an out-of-group import, and one implicit head width
- Where: src/tree/mirror/streaming/remote/streams.rs:62-68 (related: src/tree/mirror/streaming/remote/streams.rs:185, src/tree/mirror/streaming/remote/streams.rs:245, src/tree/mirror/streaming/remote/streams.rs:252, src/tree/mirror/streaming/remote/streams.rs:293, src/tree/mirror/streaming/remote/streams.rs:335, src/tree/mirror/streaming/remote/streams.rs:410, src/tree/mirror/streaming/remote/streams.rs:434, src/tree/mirror/streaming/remote/streams.rs:506, src/tree/mirror/streaming/remote/streams.rs:521, src/tree/mirror/streaming/remote/streams.rs:532, src/tree/mirror/streaming/remote/streams.rs:552, src/tree/mirror/streaming/remote/streams.rs:587, src/tree/mirror/streaming/remote/streams.rs:758-760, src/tree/mirror/streaming/remote/streams.rs:769, src/tree/mirror/streaming/remote/streams.rs:784, src/tree/mirror/streaming/remote/adapter/decode.rs:1-2, src/tree/mirror/streaming/remote/adapter/decode.rs:490, src/tree/mirror/streaming/remote/streams/tests.rs:119, src/tree/mirror/streaming/remote/streams/tests.rs:204, src/tree/mirror/streaming/remote/streams/tests.rs:228-230, src/tree/mirror/streaming/remote/streams/tests.rs:248, src/tree/mirror/streaming/remote/streams/tests.rs:429, src/tree/mirror/streaming/remote/streams/tests.rs:484, src/link.rs:156)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep over the partition files for `std::io::`, `tokio::io::`, `std::mem::`, `std::sync::`, `crate::Version`, function-local `use crate::tree::mirror::cbor`, and `crate::link::Memory*`; import-order pattern via `grep -rn -B1 -A1 '^use crate::message::PayloadCodec' src/`)
- Seen by: structure ([12]), perfapi ([38]); refutation: confirmed ([12]), reframed ([38]: `futures::Stream` at 408 and 432 must stay qualified because `codec::Stream` is imported at line 54, the informing case the doctrine exempts; the `+ 1` at line 64 is a `Vec::with_capacity` hint whose meaning is implicit); history: no-rationale-found (the one import sweep, c6fe4018, was serde-only)
- Owner-gated: no

streams.rs spells `tokio::io::AsyncRead` four times (335, 410, 434, 758), `std::io::Error`/`ErrorKind` on nine lines where link.rs:156's convention is `use std::io;` then `io::Error`, `std::sync::Arc<std::sync::Mutex<...>>` twice plus its construction (506, 532, 587), `std::mem::replace` (185), and imports `crate::tree::mirror::cbor` inside two function bodies (63, 760) rather than once at file level; the label buffer's capacity spells the stream-index head's width as a literal `+ 1` where `cbor::head_len(u64::from(stream.index()))` would state it. decode.rs:490 writes `&crate::Version` with no import (the adapter tests import `before::Version`), and decode.rs:1 places `use crate::message::PayloadCodec;` above the std group (pump.rs:27 and work.rs:8 share the quirk). The test file qualifies `crate::link::MemoryConnector`/`MemoryAcceptor` (228, 248), `tokio::io::DuplexStream` four times, and `std::io::ErrorKind` (484). Imports over long qualified paths except where the qualification informs: `futures::Stream` here is the informing case and stays; the rest are not.

Evidence:

        62	fn label(epoch: u8, stream: Stream) -> Vec<u8> {
        63	    use crate::tree::mirror::cbor::{self, MAJOR_UINT};
        64	    let mut label = Vec::with_capacity(cbor::head_len(u64::from(epoch)) + 1);
        ...
       506	    supply_failure: std::sync::Arc<std::sync::Mutex<Option<std::io::Error>>>,

Resolution: Add `use std::{io, mem, sync::{Arc, Mutex}}; use tokio::io::AsyncRead; use crate::tree::mirror::cbor;` at the top of streams.rs and rewrite the sites (`io::Error`, `mem::replace`, `Arc<Mutex<..>>`); write the capacity as `cbor::head_len(u64::from(epoch)) + cbor::head_len(u64::from(stream.index()))`; in decode.rs import `Version` and move the `PayloadCodec` import into the crate group (same at pump.rs and work.rs); in tests.rs import `MemoryAcceptor`, `MemoryConnector`, `DuplexStream`, and `io::ErrorKind`. Finding 25's newtype removes three of the `std::sync` sites on its own. Acceptance: `grep -n 'std::sync::\|std::mem::\|tokio::io::AsyncRead\|std::io::\|crate::Version' <partition files>` returns only `use` lines; `just fmt` clean.

### remote-adapter-streams-19: `ReplyFrame`'s "static" exclusion is a runtime `TryFrom` on frames the adapter never produces, leaving a public error variant no session can fire
- Where: src/tree/mirror/streaming/remote/streams.rs:73-106 (related: src/tree/mirror/streaming/remote/streams.rs:169, src/tree/mirror/streaming/remote/proxy/work/encode.rs:224-229, src/tree/mirror/streaming/remote/proxy/error.rs:51-53, src/tree/mirror/streaming/remote/error.rs:18, src/error.rs:48, src/tree/mirror/streaming/remote/adapter/encode.rs:157-246, src/tree/mirror/streaming/remote/streams/tests.rs:562-571)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`git grep -n ReplyFrame -- src/ tests/`: the single production construction is `ReplyFrame::try_from(frame).map_err(Error::ReplyFrame)?` at proxy/work/encode.rs:226, applied to `render`'s output; read `render` end to end: it yields `Frame::Reaction(_, Flow::Continue)` at 166-169, 176-179, 215-218, 227-230, `Frame::Reaction(_, Flow::End)` at 237-240, and `Frame::End(End::Reply)` at 241-244, never `End::Stream`; `ReplyFrameError` is re-exported at remote/error.rs:18 and src/error.rs:48)
- Seen by: structure ([2]), prose ([26], the "smuggle" wording); refutation: confirmed (with the caveat that "statically excluding" is true at `StreamSender::frame`'s signature; the substance is the guard with no constructible failure and the public variant no session can produce); history: no-rationale-found (the type, its `TryFrom`, and the doc line arrived verbatim in 24b187a8 and moved in b3b877d9; the choice of a runtime `TryFrom` over infallible constructors is argued nowhere)
- Owner-gated: yes: deleting `ReplyFrameError` and `RemoteError::ReplyFrame` changes the public error taxonomy re-exported through `crate::error`

`ReplyFrame` does keep `End::Stream` out of `StreamSender::frame`'s signature (169), but the only way to construct one is the fallible `TryFrom<Frame>`, and its only production caller applies it to `Encoded.frame` values that `render` only ever builds as reactions or `End::Reply`. The check therefore never fails, and `ReplyFrameError` plus `RemoteError::ReplyFrame` (whose own doc, proxy/error.rs:51, calls it "A frame constructed by the adapter violated the reply-only boundary") are public taxonomy entries no session can produce. The exclusion is not static at the adapter because `ReplyFrame` lives in `streams`, which the adapter does not depend on, so the adapter cannot emit the typed frame. The cheapest artifact that satisfies the check is the one the code already produces, so the check catches nothing; a guard must name a concrete, constructible failure. The doc's "smuggle" also imports an adversary for what is an in-process programmer error (the model of record has no hostile party here).

Evidence:

        73	/// A protocol reply frame, statically excluding stream-end transport control.
        74	///
        75	/// Stream end is a lifecycle event owned by [`StreamSender::finish`]; a
        76	/// producer cannot smuggle one into the middle of its replies.
        77	#[derive(Debug, Clone, PartialEq, Eq)]
        78	pub struct ReplyFrame(Frame);
        79	
        80	impl TryFrom<Frame> for ReplyFrame {
        81	    type Error = ReplyFrameError;
        82	
        83	    /// Check that a general wire frame belongs to a protocol reply.
        84	    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        85	        if matches!(frame, Frame::End(End::Stream)) {
        86	            Err(ReplyFrameError::StreamEnd)
        87	        } else {
        88	            Ok(Self(frame))
        89	        }
        90	    }
        91	}

    proxy/work/encode.rs:
       226	            let frame = ReplyFrame::try_from(frame).map_err(Error::ReplyFrame)?;

Resolution: Move `ReplyFrame` beside `Frame`/`End` in `codec::frame` with infallible constructors (`ReplyFrame::reaction(Reaction, Flow)`, `ReplyFrame::reply_end()`) and `From<ReplyFrame> for Frame`; have `render` yield `Encoded { frame: ReplyFrame, .. }` so `write_encoded` passes it straight to `StreamSender::frame`. Delete `TryFrom<Frame>`, `ReplyFrameError`, `RemoteError::ReplyFrame` (owner decision), and the `stream_end_is_not_a_reply_frame` test that pins the runtime check; adapter tests using `into_parts()` compare via `Frame::from`. If the type is kept as is, at least reword 75-76 to "a producer cannot emit one mid-reply". Acceptance: `git grep ReplyFrameError` returns nothing; `Encoded`'s frame field is `ReplyFrame`; no `TryFrom<Frame>` exists; `cargo doc` links resolve; wire snapshots unchanged (no byte moves).

### remote-adapter-streams-20: `SendState` forces two `unreachable!`s that `Option`-shaped state removes
- Where: src/tree/mirror/streaming/remote/streams.rs:179-234 (related: src/tree/mirror/streaming/remote/streams.rs:131-134)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure ([4]); refutation: confirmed (failure semantics preserved: a failed end-control write drops the opened state, the contract's abort, exactly as `finish(mut self)` drops `self` today); history: no-rationale-found (at b3b877d9 only `write`'s `unreachable!` existed; 4be4b830, a WIP-labeled snapshot, added `Done` to the `Open` variant and introduced the second `unreachable!` plus the `mem::replace` without arguing the encoding)
- Owner-gated: no

`finish` and `write` each end with a `let SendState::Open(..) = ... else { unreachable!(..) }` whose proof is "the open state was just stored/written through". With `state: Option<Opened<Tx>>` (a two-field struct for the writer and its `Done`), `write` binds `None => state.insert(Opened { .. })` (`Option::insert` returns `&mut T`, reusing the same disjoint-field borrow the current `state @ SendState::Unopened` arm relies on) and `finish` becomes `let Some(mut opened) = self.state.take() else { return Ok(()) }; opened.frame(stream, Frame::End(End::Stream)).await?; opened.done.complete(opened.write.into_inner().into_inner())` once the frame write moves into `Opened::frame`. Both `unreachable!`s and the `mem::replace` disappear. Panics reachable only by programmer error are permitted, but a state encoding that needs two of them to say "I just set this" is less evidently correct than one where the type carries the fact.

Evidence:

       183	                self.write(Frame::End(End::Stream)).await?;
       184	                let SendState::Open(write, done) =
       185	                    std::mem::replace(&mut self.state, SendState::Unopened)
       186	                else {
       187	                    unreachable!("the open state was just written through");
       188	                };
        ...
       224	                let SendState::Open(write, _) = state else {
       225	                    unreachable!("the open state was just stored");
       226	                };

Resolution: Replace `enum SendState<Tx>` with `struct Opened<Tx> { write: FrameWrite<CountedWrite<Tx>>, done: Done<Tx> }` and `state: Option<Opened<Tx>>`; factor the frame write into `Opened::frame(&mut self, stream: Stream, frame: Frame)`; restructure `write`/`finish` as above. Acceptance: `git grep -n 'unreachable!' src/tree/mirror/streaming/remote/streams.rs` returns nothing from `StreamSender`; `unopened_sender_finishes_without_connecting`, `truncated_stream_is_reported_not_ended`, and `frames_flow_sender_to_claimed_receiver` pass unchanged.

### remote-adapter-streams-21: No frame counts in `SessionStats` although every frame crosses this layer
- Where: src/tree/mirror/streaming/remote/streams.rs:230-233 (related: src/tree/mirror/streaming/remote/streams.rs:478-483, src/tree/mirror/streaming/stats.rs:39-41, src/tree/mirror/streaming/stats.rs:99-132, src/observe.rs:109-117, tests/session_stats.rs)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: assessed (read `SessionStats`: `bytes_sent`/`bytes_received` at stats.rs:118, 132, no frame counter, `#[non_exhaustive]` at 40; `StreamSender::write` and `read_frames` are the per-frame choke points)
- Seen by: perfapi ([40]); refutation: confirmed (facts; a proposal, not a defect); history: no-rationale-found (487e17ea designed `SessionStats` with bytes at the codec seam and neither proposed nor declined frame counts; the observe hook is the current way to count frames)
- Owner-gated: yes: public API addition

`SessionStats` reports bytes each direction carried but not frames. `StreamSender::write` and `read_frames` are the single choke points every reconciliation frame crosses, so a `frames_sent`/`frames_received` pair costs one `Relaxed` increment per frame on an already-shared `Recorder`. What a user does with it: `bytes_sent / frames_sent` is the achieved supply batching, the direct feedback for tuning `Peer::target_message_size` (today inferable only by attaching a `StreamObserver` and counting `message` callbacks, observe.rs:109-117); `frames_received` against `messages_gained` and `disputed_scopes` shows how chatty a session was relative to what it moved. `SessionStats` is `#[non_exhaustive]`, so the addition is non-breaking.

Evidence:

       230	        write
       231	            .frame(&(stream, frame))
       232	            .await
       233	            .map_err(SendError::Frame)

Resolution: Add `frames_sent`/`frames_received: u64` with `Recorder::frame_sent()`/`frame_received()` called after a successful `frame` in `StreamSender::write` and on each yielded frame in `read_frames` (whether the consumed `End::Stream` control counts, stated in the field doc); extend `tests/session_stats.rs` with a constructed-corpus expectation. Acceptance: the two fields exist with seam-naming docs; `tests/session_stats.rs` pins their values on a corpus where the expected frame count is derivable; `just gate` clean.

### remote-adapter-streams-22: `SendError` and the adapter's `EncodeError` are exhaustive while their incoming twins `StreamError`, `AcceptError`, and `DecodeError` are `#[non_exhaustive]`
- Where: src/tree/mirror/streaming/remote/streams.rs:238-239 (related: src/tree/mirror/streaming/remote/streams.rs:264-266, src/tree/mirror/streaming/remote/streams.rs:794-796, src/tree/mirror/streaming/remote/adapter/error.rs:43-44, src/tree/mirror/streaming/remote/adapter/error.rs:57-59, src/error.rs:44-50)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (`grep -rn non_exhaustive src/tree/mirror/streaming/remote/ src/error.rs`: present on `StreamError` (streams.rs:265), `AcceptError` (795), adapter `DecodeError` (error.rs:58); absent on `SendError` (238-239) and adapter `EncodeError` (43-44); all five are re-exported through `crate::error`)
- Seen by: perfapi ([37]), structure (open question); refutation: confirmed; history: no-rationale-found (1e458d69 names six enums it opened and says "the contract-outcome and wire-grammar enums stay exhaustive deliberately" without placing `SendError` or `EncodeError` on either side; the CBOR review shows Finch rules this per enum)
- Owner-gated: yes: a public attribute

The public error taxonomy marks the incoming side open and the outgoing side closed without saying why. `SendError` is the outgoing transport-failure taxonomy (connect, label, frame) and grows with the stream lifecycle, the same class as `StreamError`; a downstream exhaustive `match` on it compiles today and breaks on the first added variant. Either answer is fine; the asymmetry is unexplained at the declaration, which is where 1e458d69's own practice puts the ruling.

Evidence:

       238	#[derive(Debug, thiserror::Error)]
       239	pub enum SendError {

    adapter/error.rs:
        43	#[derive(Debug, thiserror::Error)]
        44	pub enum EncodeError<E> {

Resolution: Either add `#[non_exhaustive]` to `SendError` (and `EncodeError<E>` if the same reasoning holds) or state at each declaration that its variant set is closed and why. Acceptance: every public error enum in `crate::error` either carries `#[non_exhaustive]` or a declaration-site sentence saying why it is closed.

### remote-adapter-streams-23: `SupplyClosed.source`'s doc says the session "reports" it, while `ErrorRoute::report` is the path that never carries it
- Where: src/tree/mirror/streaming/remote/streams.rs:286-289 (related: src/tree/mirror/streaming/remote/streams.rs:442-450, src/tree/mirror/streaming/remote/streams.rs:500-505, src/tree/mirror/streaming/remote/streams.rs:516-518, src/tree/mirror/streaming/remote/streams.rs:569-577, src/tree/mirror/streaming/remote/proxy/work.rs:243-266, src/tree/mirror/streaming/remote/streams/tests.rs:471-484)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the reporter at 447-450, which always writes `source: None`; the deposit's attachment at 572 and work.rs:246, 259-263; the test at tests.rs:474 asserting `source: None` on a report in a run where the acceptor observed `UnexpectedEof` at 484)
- Seen by: prose ([20]); refutation: reframed (the doc's own clause "on the error the session surfaces as its cause" already scopes `source` to the surfaced regime, and every surfacing path attaches the deposit, so the public contract is accurate; what remains is one vocabulary collision); history: deliberate-and-holds (54420d7f wrote the paragraph and the two-phase mechanism together; the mechanism is stated inline at 442-446 and 500-505)
- Owner-gated: no

The variant doc uses "reports" for the surfaced value; `ErrorRoute::report` (516) is the internal path that always carries `None` (447-450). A maintainer tracing the reporter and then reading the variant sees the two disagree until they find the deposit mechanism at 442-446 or 500-505. One word, or a pointer, closes the gap.

Evidence:

       286	    /// `source` carries the supply's own transport failure when the session
       287	    /// observed one; a session reports it exactly once, on the error the
       288	    /// session surfaces as its cause. `None` means the supply closed
       289	    /// without an observed transport failure.

Resolution: Replace "reports" with "surfaces" and add a pointer: "Reporters leave `source` empty; the session terminal attaches the deposit ([`FirstStreamError::take_supply_failure`]) before surfacing." Acceptance: the variant doc names the attachment site; the test's `source: None` assertion reads as consistent with the doc.

### remote-adapter-streams-24: `StreamReceiver`'s two-`Option` state and `ReceiverStart` dissolve: the `stream!` generator is already lazy
- Where: src/tree/mirror/streaming/remote/streams.rs:304-331 (related: src/tree/mirror/streaming/remote/streams.rs:338-359, src/tree/mirror/streaming/remote/streams.rs:367-375, src/tree/mirror/streaming/remote/streams.rs:377-396, src/tree/mirror/streaming/remote/streams.rs:408-417, src/tree/mirror/streaming/remote/streams.rs:436-452)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read `read_frames`: every side effect, `claim.await` at 437 and the observer creation at 453-455 included, sits inside the `stream!` block, which runs only when first polled)
- Seen by: structure ([3]); refutation: confirmed (two costs added: `Box::pin` moves to `new` for every receiver including never-polled ones, one small allocation per stream per session, bounded by `STREAM_COUNT`; `StreamReceiver<Rx>`'s type parameter would become phantom); history: no-rationale-found (verbatim from b3b877d9; the deadlock design specifies when a receiver claims, not the struct shape that stages it)
- Owner-gated: no

`StreamReceiver` holds `start: Option<ReceiverStart<Rx>>` and `frames: Option<BoxStream>`, with `frames()` moving seven fields out of one into the other on first poll behind an `expect`. `read_frames` is an `async_stream::stream!` whose body runs only when first polled, so building the boxed stream in `new` preserves lazy claiming exactly. The only thing the two-`Option` shape buys is the "was it ever polled" bit that `finish` needs, which is one `bool` set in `poll_next`. An invariant ("exactly one of the two `Option`s is `Some`") held by convention plus an `expect`, and a seven-field struct that exists only to be destructured once, are incidental complexity when the generator already provides the laziness.

Evidence:

       304	pub struct StreamReceiver<Rx> {
       305	    /// The claim and identity, consumed to build `frames` on first poll.
       306	    start: Option<ReceiverStart<Rx>>,
       307	    /// `Some` exactly once the stream has been claimed: the first poll
       308	    /// builds it, and [`finish`](Self::finish) reads its absence as "this
       309	    /// level was never needed".
       310	    frames: Option<BoxStream<'static, Frame>>,
       311	}

Resolution: `StreamReceiver { frames: BoxStream<'static, Frame>, claimed: bool }`; `new` calls `Box::pin(read_frames(..))` directly; `poll_next` sets `claimed = true` before polling; `finish` checks `!self.claimed`. Delete `ReceiverStart` and `frames()`; move the per-field docs (stats and observe rationale) onto `read_frames`' parameters. Decide whether `Rx` stays as a phantom or the type loses the parameter (callers in state.rs name `StreamReceiver<A::Rx>`). Acceptance: no `expect` remains in `StreamReceiver`; `unpolled_receiver_finishes_vacuously`, `frames_flow_sender_to_claimed_receiver`, and `supply_failure_reaches_the_awaiting_receiver` pass unchanged.

### remote-adapter-streams-25: The supply-failure deposit slot is spelled twice with two lock sites; a newtype names it once
- Where: src/tree/mirror/streaming/remote/streams.rs:506 (related: src/tree/mirror/streaming/remote/streams.rs:521-524, src/tree/mirror/streaming/remote/streams.rs:532, src/tree/mirror/streaming/remote/streams.rs:552-557, src/tree/mirror/streaming/remote/streams.rs:587)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the five sites)
- Seen by: structure ([9]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`std::sync::Arc<std::sync::Mutex<Option<std::io::Error>>>` appears verbatim in `ErrorRoute` and `FirstStreamError`, is constructed at 587, and is locked with the same `.expect("supply failure lock")` in `supply_failed` and `take_supply_failure`. The slot's protocol (first deposit wins; the terminal is the sole consumer) is prose on two structs rather than a type with two methods. Newtypes over synonyms.

Evidence:

       506	    supply_failure: std::sync::Arc<std::sync::Mutex<Option<std::io::Error>>>,

Resolution: `struct SupplyFailureSlot(Arc<Mutex<Option<io::Error>>>)` with `deposit(&self, io::Error)` (`get_or_insert`) and `take(&self) -> Option<io::Error>`; both structs hold one; the two lock-and-expect sites collapse into its two methods (and finding 13's message fix lands once). Acceptance: one `lock().expect(..)` in streams.rs; `supply_failure_reaches_the_awaiting_receiver` passes unchanged.

### remote-adapter-streams-26: `claims()` builds the slots through a `Vec` and a fallible `try_into` where two `array::from_fn` calls suffice
- Where: src/tree/mirror/streaming/remote/streams.rs:620-631
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read; `std::array::from_fn` takes `FnMut`, so the second closure may write into a pre-declared array)
- Seen by: structure ([10]), perfapi ([39]); refutation: confirmed (both); history: no-rationale-found (verbatim from b3b877d9)
- Owner-gated: no

The receivers are built with `std::array::from_fn` while the senders go through `Vec::with_capacity`, `push`, and `try_into().unwrap_or_else(|_| unreachable!(..))`. Declaring one array first and filling it from the other's `from_fn` closure needs no fallible conversion and no `unreachable!`. An array of fixed dimension should be constructed as one, not reconstructed from a `Vec` with a proof string.

Evidence:

       620	pub fn claims<Rx>() -> (ClaimSlots<Rx>, Claims<Rx>) {
       621	    let mut senders = Vec::with_capacity(STREAM_COUNT);
       622	    let receivers = std::array::from_fn(|_| {
       623	        let (send, receive) = oneshot::channel();
       624	        senders.push(Some(send));
       625	        Some(receive)
       626	    });
       627	    let slots = senders
       628	        .try_into()
       629	        .unwrap_or_else(|_| unreachable!("one sender exists for every stream"));
       630	    (ClaimSlots { slots }, Claims { slots: receivers })
       631	}

Resolution: `let mut receivers: [Option<oneshot::Receiver<_>>; STREAM_COUNT] = std::array::from_fn(|_| None); let slots = std::array::from_fn(|index| { let (send, receive) = oneshot::channel(); receivers[index] = Some(receive); Some(send) }); (ClaimSlots { slots }, Claims { slots: receivers })`. Acceptance: `claims()` contains no `Vec` and no `unreachable!`; `streams/tests.rs` passes unchanged.

### remote-adapter-streams-27: `ClaimSlots` lacks the `take` its mirror `Claims` has; the driver reaches into its field
- Where: src/tree/mirror/streaming/remote/streams.rs:736-741 (related: src/tree/mirror/streaming/remote/streams.rs:600-617)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read both sites)
- Seen by: structure ([11]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`Claims::take(stream)` (612-616) encapsulates indexing by `stream.index()`; `accept_one` performs the same operation on `ClaimSlots` inline as `self.slots.slots[usize::from(stream.index())].take()`. The two halves of one allocation should expose the same shape; the index-by-stream rule lives in one method on one half and inline on the other.

Evidence:

       736	        let slot =
       737	            self.slots.slots[usize::from(stream.index())]
       738	                .take()
       739	                .ok_or(AcceptError::Duplicate {
       740	                    origin: Origin::stream(self.speaker, stream),
       741	                })?;

Resolution: `impl<Rx> ClaimSlots<Rx> { fn take(&mut self, stream: Stream) -> Option<oneshot::Sender<(Rx, Done<Rx>)>> }`; the driver calls `self.slots.take(stream).ok_or(..)`. Acceptance: no direct `.slots[` indexing outside the two `take` methods.

### remote-adapter-streams-28: `label_item`'s two rejection arms and two EOF-deferral arms have no committed test
- Where: src/tree/mirror/streaming/remote/streams.rs:756-778 (related: src/tree/mirror/streaming/remote/streams.rs:159-168, src/tree/mirror/streaming/remote/streams.rs:694-704, src/tree/mirror/streaming/remote/streams/tests.rs:227-236, src/tree/mirror/streaming/remote/streams/tests.rs:415-448, src/tree/mirror/streaming/remote/streams/tests.rs:461-485, src/tree/mirror/streaming/remote/proxy/tests/failures.rs:28-53, src/tree/mirror/streaming/remote/proxy/tests/failures.rs:167-243)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`git grep -n 'AcceptError::Label\|label item is not an unsigned int\|label head is not canonical\|label_item' -- src/ tests/` hits only streams.rs itself and the capture harness's unrelated `label_item`; the committed tests reach `SupplyFailed` only through `acceptor.accept()` failing (tests.rs:47-51, 467), never through `label_item`'s `Ok(None)` or `Err(Io)` arms; the transport-fault proptest injects errors carrying `InjectedIo` (failures.rs:29-53) and never a clean close)
- Seen by: correctness ([27]); refutation: confirmed, severity lowered to low (the arms decide error classification and termination timing under a nonconforming peer or a dying transport, a conformance-bug detector, not a data outcome); history: already-known (the scope-A mutation campaign recorded `label_item`'s `MAJOR_UINT` guard as a survivor and dispositioned it with an exhaustive initial-byte matrix against `cbor::read_head` plus an EOF witness; not landed at HEAD)
- Owner-gated: no

Four wire-reachable arms are untested: a label item that is not an unsigned int, a non-canonical head, a clean close before an item (`Ok(None)` classified as `SupplyFailed(UnexpectedEof)`), and an I/O failure inside a head. Every sibling `AcceptError` arm (`Epoch`, `UnknownStream`, `Duplicate`, `Unexpected`) is pinned in `streams/tests.rs`. The EOF-deferral arms are the mechanism behind the peer-side consequence `StreamSender::frame`'s `# Cancel safety` section promises (a short label read classified as a failed supply that drops every undelivered claim slot), so the file's one operational hazard claim rests on unpinned code. A wire-reachable rejection with no test is a criterion the wrong implementation also passes. The campaign's recorded family covers both `Label` arms and the clean-close arm; the `Err(Io)` arm and the cancel-safety cross-reference are additions.

Evidence:

       761	    match cbor::read_head_async(rx).await {
       762	        Ok(Some(head)) if head.major == cbor::MAJOR_UINT => Ok(head.value),
       763	        Ok(Some(_)) => Err(AcceptError::Label {
       764	            origin: Origin::direction(speaker),
       765	            detail: "label item is not an unsigned int",
       766	        }
       767	        .into()),
       768	        Ok(None) => Err(AcceptFate::SupplyFailed(
       769	            std::io::ErrorKind::UnexpectedEof.into(),
       770	        )),
       771	        Err(cbor::HeadReadError::Io(io)) => Err(AcceptFate::SupplyFailed(io)),
       772	        Err(cbor::HeadReadError::Malformed(_)) => Err(AcceptError::Label {
       773	            origin: Origin::direction(speaker),
       774	            detail: "label head is not canonical",
       775	        }
       776	        .into()),
       777	    }

Resolution: Land the recorded disposition beside `accept_driver_rejects_unknown_stream_index`, using `raw_labeled`-style raw connects on a `memory()` link: (1) write `[0x40, 0x03]` (a byte-string head where the epoch belongs) and assert `AcceptError::Label { detail: "label item is not an unsigned int", .. }`; (2) write `[0x18, 0x00, 0x03]` (epoch 0 spelled with a one-byte argument) and assert `AcceptError::Label { detail: "label head is not canonical", .. }`; (3) write `[EPOCH]` alone and drop the writer, drive a receiver awaiting its claim through `first_reported_error`, and assert `StreamError::SupplyClosed { source: None, .. }` with `take_supply_failure()` yielding `UnexpectedEof`; (4) write `[0x18]` (the first byte of a two-byte head) and drop, asserting the same deferral. Cross-reference (3) from `StreamSender::frame`'s cancel-safety section so the doc claim names its pin. Acceptance: the four tests exist, each fails when its arm in `label_item` is replaced by a different `AcceptFate`, and the `frame()` cancel-safety text names the test that pins the peer-side classification.
Construction: As in the resolution; all four are buildable from the existing `raw_labeled`, `claims`, `error_route`, and `first_reported_error` helpers.

### remote-adapter-streams-29: The label testdoc states a point ("exactly two bytes") as the law; the label is three bytes from epoch 24
- Where: src/tree/mirror/streaming/remote/streams/tests.rs:22-27 (related: src/tree/mirror/streaming/remote/streams.rs:57-68, src/tree/mirror/cbor.rs:80-83, src/tree/mirror/cbor.rs:105-112, src/link.rs:352, src/link.rs:391, tests/reuse.rs:184-226, src/tree/mirror/streaming/remote/streams/tests.rs:424)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (cbor.rs:81-83 gives head widths `0..=23 => 1, 24..=0xff => 2`; `render_head` at 112 emits `[major | 24, value as u8]` for 24..=255; the epoch is `u8` (link.rs:352) advanced by `wrapping_add(1)` (391); `label` writes two heads (streams.rs:65-66) and sizes its buffer `head_len(epoch) + 1` (64); the construction below is derived from `render_head`, not executed)
- Seen by: prose ([17]), correctness ([32]); refutation: confirmed (both); history: already-known (true when written in b3b877d9, when the label was `[epoch, stream.index()]`; expired at 4dd2053c when the label became two CBOR heads; the CBOR review recorded the rot as R4 with a `two-byte` sweep that does not match this testdoc's `two bytes`, and S1 records the owner-endorsed count-free wording for such fixes)
- Owner-gated: no

The testdoc claims the label is exactly two bytes. From the 25th session on a reused link the epoch head is two bytes and the label three; the code's own capacity computation already accounts for it, and `tests/reuse.rs::epoch_wrap_keeps_the_pair_in_lockstep` exercises epochs 253 through 2 end to end, so the label itself is right and the defect is the testdoc and the missing family statement. AGENTS.md: an inaccurate testdoc is a bug in the test; a claim over `epoch: u8` is a family and belongs to a proptest. A related fragility: tests.rs:424 hand-spells a label as `&[EPOCH, Stream::COUNT]`, which is a valid one-byte head only because both values are below 24.

Evidence:

        22	/// The label is exactly two bytes: the session epoch then the stream index.
        23	#[test]
        24	fn label_is_epoch_then_stream() {
        25	    let stream = Stream::new(3).expect("stream 3 exists");
        26	    assert_eq!(label(7, stream), [7, 3]);
        27	}

Resolution: Reword the testdoc count-free per S1 ("The label is the epoch head then the stream-index head, each a shortest-form CBOR unsigned int; both below 24 encode as one byte each"), keep the literal case as the readable example, and add a proptest over `epoch in any::<u8>()` and every `Stream` asserting `label(epoch, stream).len() == cbor::head_len(epoch) + 1` and that two `cbor::read_head` calls recover `(epoch, index)` and exhaust the input; spell tests.rs:424 through `cbor::write_head`. Acceptance: the testdoc is true for every `u8` epoch; a committed proptest exercises epochs at and above 24; any seed file that appears is committed.
Construction: `label(24, Stream::new(3).unwrap())` is `[0x18, 0x18, 0x03]` by `render_head` (major 0, info 24, argument byte 24; then `0x03`), three bytes, refuting "exactly two bytes".

### remote-adapter-streams-30: Six identical `StreamSender::new` and four `StreamReceiver::new` spellings hide each test's distinguishing inputs
- Where: src/tree/mirror/streaming/remote/streams/tests.rs:36-43 (related: src/tree/mirror/streaming/remote/streams/tests.rs:64-71, src/tree/mirror/streaming/remote/streams/tests.rs:86-94, src/tree/mirror/streaming/remote/streams/tests.rs:122-130, src/tree/mirror/streaming/remote/streams/tests.rs:144-151, src/tree/mirror/streaming/remote/streams/tests.rs:189-196, src/tree/mirror/streaming/remote/streams/tests.rs:255-263, src/tree/mirror/streaming/remote/streams/tests.rs:337-344, src/tree/mirror/streaming/remote/streams/tests.rs:500-507, src/tree/mirror/streaming/remote/streams/tests.rs:530-538)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (line ranges enumerated from the full read)
- Seen by: structure ([15]); refutation: confirmed; history: no-rationale-found (the dedup pass 59827c7a hoisted `raw_labeled` and `first_reported_error` when both constructors took four arguments; the arity grew afterwards with no revisit)
- Owner-gated: no

The six-argument `StreamSender::new(a.connector.clone(), <epoch>, Speaker::Initiator, <stream>, Recorder::default(), SessionHandle::default())` appears six times varying only epoch and stream; the seven-argument `StreamReceiver::new(..)` four times varying only stream and route. A reader scanning for what differs between `accept_driver_rejects_wrong_epoch` and `frames_flow_sender_to_claimed_receiver` should see the epoch, not eight identical lines. The file already does this well for violations (`raw_labeled`, `first_reported_error`).

Evidence:

        36	        let sender: StreamSender<_> = StreamSender::new(
        37	            a.connector.clone(),
        38	            0,
        39	            Speaker::Initiator,
        40	            Stream::new(1).expect("stream 1 exists"),
        41	            Recorder::default(),
        42	            SessionHandle::default(),
        43	        );

Resolution: Add `sender(&connector, epoch, stream)` and `receiver(&mut claims, stream, route)` beside `reply_frame`; rewrite the ten sites. Acceptance: each test body constructs its sender or receiver in one line.

## Positives

- `Encoded::write_with` (encode.rs:26-36) makes the walk's "wire before internal publication" liveness rule the only order the API admits: the question is unreachable until the writer's future resolves `Ok`. The module doc (adapter.rs:37-40) says exactly why.
- The adapter module doc's closing section "Why this is sufficient" (adapter.rs:69-74) names the four protocol properties the conversion rests on in four clauses and states that the adapter adds no identity or ordering authority of its own. Negative-space specification a maintainer needs and rarely gets.
- `SupplyRuns::observe` (decode.rs:487-541) is the single place peer-supplied leaf records are judged; every wire-reachable failure there is a typed `DecodeError` variant carrying the offending bytes, and nothing from the wire reaches a panic. The declared-`set_len` charge lands before `Leaf::leaf` takes custody on both decode paths, and `malformed.rs` proves it with a node census.
- Positional overruns fail eagerly at the offending frame (`UnpositionedMatch`/`UnpositionedQuery` before the skeleton grows; decode.rs:335-339 states why) and a new supply run requires a strictly larger radix, so the decoded skeleton is bounded by the fan by construction.
- The encoder serializes each leaf straight out of the borrowed node (encode.rs:197-206) with no `Version` clone and no `Arc` bump, and the comment says why the span rides a local; `LeafRun` stays encoded on both sides so the decode bound is one run's bytes (decode.rs:356-358), and `Frame`s move rather than clone along the whole path.
- `Scope` (scope.rs) survived height erasure as a small, fully documented type whose parent prefix is the height witness, stated at the type that carries it (scope.rs:5-7); each method carries a one-sentence contract and nothing more.
- `StreamSender::frame` (streams.rs:157-168) carries a `# Cancel safety` section that states not just "not cancel safe" but the exact peer-side consequence (a truncated label read as a failed supply, misattributed session-wide) and what the caller must do.
- `AcceptDriver`'s type doc (streams.rs:643-650) names the two fates of an unasked stream, admits neither is prompt, and says why that is detection latitude and not a safety gap (never absorbable; memory bounded by the link stream's own buffers).
- The supply-failure deferral (`AcceptDriver::run` docs 678-693, the deposit slot 500-506, `queued_supply_closed` 559-577) is a subtle design argued carefully in prose, and `supply_failure_reaches_the_awaiting_receiver` and `supply_failure_after_delivery_lets_the_session_finish` pin both the report-and-deposit and the park paths deterministically.
- Every failure path in `read_frames` is report-then-park with the transport half retained until teardown (streams.rs:485-492 says why the hand-back is a framing judgment), so a consumer never observes a truncation as a clean end; `truncated_stream_is_reported_not_ended` pins it with the origin checked in full and says why (tests.rs:362-364).
- `label()` (streams.rs:57-68) is declared the canonical spelling and the capture harness's `stream_label` (codec/capture.rs:117-122) parses through the same head grammar, so the wire label has one definition and one decoder; I checked the claim.
- `ERROR_ROUTE_CAPACITY` (streams.rs:580-582) is a named constant whose comment states the invariant it encodes (the first failure is the cause, later ones cascade), not just its value.
- The `debug_assert!` at decode.rs:348-355 justifies itself the way a guard should: it names the concrete failure the wire cannot produce but in-process construction can, and what the loop below would do wrong without it. The comment at decode.rs:179-181 correctly attributes the unpositioned-form rejections on the opening stream to in-process construction only.
- `fan_occupancy.rs` is a model liveness-floor meter: it asserts the reader/assembler channel reaches exactly FAN + 1 on both decode paths under an eager source and stays far under it under a paced source, so the probe is shown live rather than constant.

## Open questions for Finch

- Finding 19: delete `ReplyFrameError` and `RemoteError::ReplyFrame` (public taxonomy) in favor of infallible `ReplyFrame` constructors yielded by `render`? Recommendation: yes, pre-release; the variant has no producer and the doc's "statically" then becomes true at the adapter as well as at `frame()`.
- Finding 6: is the FAN channel capacity liveness-critical by a mechanism I could not derive from the joined drive? Recommendation: construct the sub-FAN run first (one test-only parameter); if it completes, rewrite the two comments to the amortization/read-ahead rationale and treat the pull-based reader as a measure-first experiment gated on a persistent backend actually wanting read-ahead. The answer decides whether the ~205 KB supply-decode pre-charge is a floor or a cost of the current shape.
- Finding 22: should `SendError` (and the adapter's `EncodeError<E>`) follow `StreamError`/`DecodeError` into `#[non_exhaustive]`, or is their closed variant set a deliberate contract to state at the declaration? Recommendation: open them now, pre-release, matching 1e458d69's criterion (variant sets that grow as enforcement grows).
- Finding 11: is the illumos gate run meant to be a property of the tree? Recommendation: either name it in the justfile or CI so the ten comment copies become checkable, or drop the platform roster from the comment and state the lowering-independent mechanism.
- Finding 21: add `frames_sent`/`frames_received` to `SessionStats`? Recommendation: yes; it is the direct feedback for `target_message_size` tuning and costs one `Relaxed` increment per frame at an existing choke point.
- Finding 7: crate-wide dash rule in `//` comments (169 sites): sweep, or record a tolerance where the rule lives? Recommendation: one mechanical sweep, since the rule is already applied to messages.
- Finding 17: the literal 17 in prose spans streams.rs, remote.rs, and codec.rs and was written fresh again in 3327a92b; cite `Stream::COUNT`/`STREAM_COUNT` by name everywhere? Recommendation: yes, in one cross-file pass.
- Cross-cutting, from the refuted candidate [8]: `AcceptError::Label { detail: &'static str }` follows the codec's convention (`DecodeErrorKind::Malformed`, `FrameShape`, eleven literal details in codec/decode.rs and frame.rs). Should present-but-malformed wire items become enums crate-wide, or stay strings? Recommendation: leave as is unless a consumer needs to match; it is a crate-wide question, not a partition inconsistency.
- `Backend` is crate-private today, which is what makes the encoder/decoder contract asserts (finding 14) programmer-error panics. If the trait is ever exposed for out-of-crate storage backends, those asserts need `# Panics` sections on the trait methods or conversion into typed errors; worth recording wherever the trait's visibility is decided.

## Dropped

- [8] `AcceptError::Label` carries a stringly-typed defect: refuted; the codec's sibling taxonomy models the same kind of defect with `detail: &'static str` at eleven sites (codec/error.rs:133, 142; codec/decode.rs:222-330; codec/frame.rs:216, 392), so this follows a crate convention; raised as an open question instead.
- [13] "the existing ... fold" is a dated qualifier: merged into remote-adapter-streams-1 (same sentence as [19]).
- [18] adapter doc describes `Reply<B, H>`: merged into remote-adapter-streams-1.
- [19] module doc names `Convert::assemble`: merged into remote-adapter-streams-1.
- [31] per-record ingest duplicated and drifted: merged into remote-adapter-streams-3; the refutation showed the `debug_assert!` asymmetry is a copy made without it (8e1ed47a7 precedes 55d76d5cf), not drift, and the finding text says so.
- [36] per-record ingestion block duplicated: duplicate of [1]; merged into remote-adapter-streams-3.
- [24] verbatim-duplicated comment blocks: the decode.rs charge-comment pair and the encode.rs "Symmetric with decode" pair dissolve with findings 3 and 5; the remaining `OversizedVersion` restatement is remote-adapter-streams-8.
- [35] session constants threaded positionally: decode-side half merged into remote-adapter-streams-4; the stream-side half kept there in reframed form (a receiver-scoped context; sender and receiver contexts overlap only partially).
- [16] hand-maintained "17": duplicate of [14]; merged into remote-adapter-streams-17.
- [38] qualified paths, repeated bounds, function-local imports: merged into remote-adapter-streams-18 minus the `futures::Stream` item, which must stay qualified because `codec::Stream` is imported at streams.rs:54.
- [39] `claims()` detours through a `Vec`: duplicate of [10]; merged into remote-adapter-streams-26.
- [32] label test pins one literal: duplicate of [17]; merged into remote-adapter-streams-29.
- [26] "smuggle" register transplant: folded into remote-adapter-streams-19's resolution (the doc block is rewritten or deleted there); the wording fix stands even if the deletion is declined.
- Refutation's new item, decode.rs:88 `pin!` also redundant: folded into remote-adapter-streams-3.
- Cross-partition ghosts surfaced while verifying [22] (codec.rs:32 "The session demultiplexer", codec/signal.rs:31 "Logical streams multiplexed"): listed as related sites in remote-adapter-streams-15 for the codec partition's owner, not filed here.
- The open question from the correctness lens about `early_supplies`' doc ("a later group's bulk never gates an earlier group's absorption") and the pruned-radix latency case: below the bar; the doc describes groups on the wire, and the pruned case is a latency observation with no stated contract breached.
- The correctness lens's note that decode.rs:299's `unreachable!` rests on an unlisted `assemble` clause: below the bar as a separate item; an assembler that stops pulling early yields no node for later runs, which "never skipped" (backend.rs:191-192) already forbids. The demonstration gap is covered by remote-adapter-streams-14.
