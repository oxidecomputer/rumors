# Partition conformance: The public conformance suite for caller-built links and backends

## Partition summary

The partition holds the crate's two validation suites. `conformance::link` (`src/conformance/link.rs`, public under the `conformance` cargo feature) is the black-box suite a deployment runs against its own `Link` transport: seven checks, each consuming a fresh link pair from the caller's factory and probing both directions, that turn violations of the link contract into panics naming the clause (byte assertions) or into hangs the caller's timeout bounds (liveness clauses). Streams are classified in-band by a tag or index byte, never by accept order, so the probes tolerate the reordering freedom the contract grants acceptors, and `check_sessions` runs bootstrap and gossip end to end with a `CountingConnector` floor proving data streams opened in each direction. Its sibling `link/tests.rs` runs the in-memory link through the suite at the default and one-byte windows and under a reversing acceptor, and holds five contract-violating fixtures (a shared-FIFO mux, direction-coupled control halves, a lossy dequeue-then-await acceptor in both directions, a capped connector, a pooled window sized below the bound) to a deterministic `Quiescence::Stalled` verdict.

`conformance::backend` (`src/conformance/backend.rs`, `cfg(test)` and `pub(crate)`) is the storage-pricing suite. A `Charged<B>` decorator keeps every live node handle's measured bytes on a process-global ledger, checks `node_bytes` pointwise where leaves are constructed, where parents are assembled, and along the bulk `leaves` and `assemble` overrides, sweeps the cost function for monotonicity in both arguments, and differences a budgeted session's census peak against a zero-budget floor session's. Its `tests.rs` runs `Local` and a row-store reference backend (`Materializing`) whose process-global knobs give each check a `should_panic` negative control.

The partition is 3565 lines: `conformance.rs` (19), `link.rs` (1082), `link/tests.rs` (1085), `backend.rs` (791), `backend/tests.rs` (588). `link/tests.rs` and `backend/tests.rs` are test code; `backend.rs` is test-only code compiled under `cfg(test)`; `link.rs` is production code shipped behind a feature.

Overall the code is in good shape, and the link suite in particular is a model of how this crate wants a check built: every liveness probe has a committed negative control asserted to fail as `Stalled`, every legal adversity asserts its adversity fired, the "What the suite cannot see" section states the negative space, and the maintainer comments explain why a branch means what it means. No false-fail path against a conforming link was found under the first three lenses, and no panic reachable from a conforming transport was found; the late correctness lens found one false-fail path that turns on a clause the contract does not state (conformance-37). The dominant issues are on the verification side of the backend suite: the census ceiling has no liveness floor, and for `Local` the stated budget resolves to the floor window, so the end-to-end check passes vacuously today while its testdoc says the budget binds; the `children` stream (the population the window prices per depth) and the `parent` Some/None clause the `Backend` docs say this suite convicts are unchecked; and the `assemble` seam is driven only at the root over one run. The remainder is duplication (two near-identical independence probes, a fixture and a helper each with a twin in `crate::testing`), a handful of doc sentences that expired when the code they described moved, and register nits the owner's writing doctrine names.

The correctness lens ran after the other three (its reader hung during the main run and was rerun after finalization), its candidates went through the same refutation and history passes, and the six that survived are conformance-37 through conformance-42: the cancellation probe's unstated connect precondition and its lossy control's window-dependent verdict, a lock-scope nit in the `WindowedTx` fixture, and three unchecked obligations in the backend suite (the decode-slot padding the window delegates to `node_bytes`, the residency the re-tag is trusted to preserve, and the monotonicity grid's gaps); the witness pass had closed before they existed, so each is verified or assessed and none is demonstrated.

## Findings

### conformance-1: The module doc promises a suite per caller-implementable boundary; `Bookmark` has none
- Where: src/conformance.rs:3-8 (related: src/bookmark.rs:92-124, src/conformance/link.rs:10-26)
- Class / severity / confidence: feature-gap / medium / high
- Provenance: assessed (read; `pub trait Bookmark` and its `store` contract read at src/bookmark.rs:92-124; grep for a `conformance` mention in bookmark.rs or a bookmark submodule under conformance returns nothing)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (a3da46c4 made `conformance` a namespace "leaving room for future suites over other caller-built boundaries"; nothing considers or declines a Bookmark suite)
- Owner-gated: yes: a new public module, or a narrowing of a public doc sentence

The public module doc states a universal rule (one validation submodule per caller-implementable boundary) and ships one suite, for `Link`. `Bookmark` is application-supplied storage whose `store` contract carries clauses a black-box suite can check exactly the way the link suite checks its clauses: `load` is `Ok(None)` before any store, store-then-load round-trips the bytes, a second store replaces the first, and a `write` closure that returns `Err` after a partial write leaves the prior record readable rather than a torn frame. The crate classifies a violation as unspecified corruption (identity reclaimed below a transmitted frontier), and a deployment has no way to validate its commit semantics before that can happen. Prose speaks what is: either the rule is met or it is not stated.

Evidence:

         3	//! Each caller-implementable boundary gets one submodule carrying its
         4	//! validation suite. [`link`] checks that a caller-built
         5	//! [`Link`](crate::link::Link) transport delivers the stream independence,
         6	//! flow control, half-close, and cancellation tolerance the [link
         7	//! module](crate::link) requires of every implementation. Available from a
         8	//! dev-dependency with the `conformance` cargo feature enabled.

    src/bookmark.rs:
       106	    /// The crate serializes the framed record by calling `write` with a lent
       107	    /// writer. The implementor **must commit the written bytes atomically iff
       108	    /// `write` returns `Ok`** and must report an error rather than leave a
       109	    /// partial frame where the next [`load`](Self::load) could read it.

Resolution: Either add `conformance::bookmark::check(factory)` taking a factory of fresh, empty `Bookmark` instances and probing the clauses above, with a negative control (a bookmark that commits a partial frame on `Err`) asserted to fail, and a "what the suite cannot see" paragraph naming crash atomicity; or reword src/conformance.rs:3-4 so it claims only what ships. Acceptance: a `conformance::bookmark` module exists with the checks and control described, or the module doc no longer states one submodule per caller-implementable boundary.

### conformance-2: Suite summaries name the link contract's clauses in words the contract does not use
- Where: src/conformance.rs:5-7 (related: src/conformance/link.rs:4-5, src/conformance/link/tests.rs:155-156, src/conformance/link/tests.rs:1026-1027, src/link.rs:55-63, src/link.rs:107-113)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'half-close\|half close\|measured\|router' src/link.rs` returns only line 149, the routed adapter's "per-process router"; the clause at src/link.rs:55 is `**Completion.**`; src/link.rs:108-109 read "have been *observed* live" and "That is observed behavior"; `router.helper` appears crate-wide only at link/tests.rs:155)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the clause was named **Half-close** until 4be4b830 renamed it **Completion**; "measured" matched link.rs's wording until c5f1210a rewrote it to "observed"; "router-helper" cited a design-doc section a3da46c4 excised from code)
- Owner-gated: no

Three cross-references into the link contract use terms the contract no longer uses, so a reader following the link has to translate: "half-close" for the **Completion** clause (both module docs), "the link docs' measured-tolerance sentence" for a sentence that says *observed*, and "the router-helper shape" for a shape the cancellation clause describes without naming a router. Each was accurate when written and lost its referent to a later commit; the rule is established terms of art only, and a summary's anchor is the cited text's own vocabulary.

Evidence:

         5	//! [`Link`](crate::link::Link) transport delivers the stream independence,
         6	//! flow control, half-close, and cancellation tolerance the [link
         7	//! module](crate::link) requires of every implementation. Available from a

    src/conformance/link.rs:
         4	//! contract](crate::link): a full-duplex control stream, independent
         5	//! receiver-paced streams, half-close, and accept-cancellation tolerance.

    src/conformance/link/tests.rs:
       155	/// This is the router-helper shape the cancellation clause exists to
       156	/// exclude: if the `accept` future is dropped between the internal

    src/conformance/link/tests.rs:
      1026	/// The sessions run at the serialization floor, the shape the link docs'
      1027	/// measured-tolerance sentence is denominated in: a sub-bound pool couples

Resolution: Write "completion" (or "stream completion") at conformance.rs:6 and link.rs:5; write "observed-tolerance sentence" or quote its key phrase at tests.rs:1027; drop "router-helper" at tests.rs:155 (the next clause already states the shape: dequeue, then await). Acceptance: every clause name the conformance docs use appears verbatim as a clause heading in src/link.rs's contract section.

### conformance-3: The backend suite's visibility rationale is stated three times, once in public rustdoc, and omits the one fact that explains `pub(crate)`
- Where: src/conformance.rs:10-19 (related: src/conformance/backend.rs:41-47, src/conformance/backend.rs:82, src/conformance/backend.rs:141, src/conformance/backend.rs:646)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep for `conformance::backend|Charged|ChargedNode|Measure` across src, tests, benches, examples outside src/conformance returns only the doc mention at src/tree/mirror/streaming/backend.rs:18)
- Seen by: structure (two findings), prose; refutation: confirmed; history: deliberate and holds for the visibility (922db57a: "crate-gated until the backend boundary ships"; the sync-budget note records that the entry point goes public with the storage boundary), no rationale for the triplication
- Owner-gated: no

Why the backend suite is crate-internal is explained in the public `conformance` module doc (which tells a library user about a suite no build they can make contains), again in a `//` comment that points at "the module docs' visibility section" without naming the module, and a third time in backend.rs's `# Visibility` section. Meanwhile the fact that would justify `pub(crate)` on `Measure`, `Charged`, and `check`, none of which has a caller outside `src/conformance/backend*`, lives only in history: `pub(crate)` is the placeholder for a `pub` that lands with the storage boundary. One statement of record, carrying that intent, is enough.

Evidence:

        10	//! A second suite validates a storage backend's session-memory pricing;
        11	//! the storage boundary is crate-internal, so that suite runs as this
        12	//! crate's own test gate rather than as a public entry point.
        13	
        14	pub mod link;
        15	
        16	// Compiled as this crate's own gate: the storage-backend boundary is
        17	// crate-internal; see the module docs' visibility section.
        18	#[cfg(test)]
        19	pub(crate) mod backend;

Resolution: Keep backend.rs's `# Visibility` section as the statement of record and add one sentence there: the suite's items are `pub(crate)` so a backend added under src/ can run it from its own tests, and go `pub` with the storage boundary. Reword the `//` comment at conformance.rs:16-17 to point at it by name; drop the public paragraph at 10-12 (the first sentence already scopes the module to what a deployment implements itself). Acceptance: the visibility argument appears once, in backend.rs, and states why the items are `pub(crate)`; the public `conformance` module doc mentions only `link`.

### conformance-4: Intensifiers and menace adverbs without their mechanism: genuinely, real, silently, loudly, teeth, sound
- Where: src/conformance/link.rs:40-46 (related: src/conformance/link.rs:125, src/conformance/link.rs:143, src/conformance/link.rs:901, src/conformance/link.rs:1017, src/conformance/link/tests.rs:46-49, src/conformance/link/tests.rs:78, src/conformance/link/tests.rs:872-877, src/conformance/backend/tests.rs:2, src/conformance/backend/tests.rs:36, src/conformance/backend/tests.rs:127, src/conformance/backend/tests.rs:190)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (line-count grep over the partition: `genuine` 12 lines, `silently` 9, `teeth` 3, `loudly` 3, `sound` 2; sites listed)
- Seen by: prose; refutation: confirmed; history: no rationale found (the words postdate none of the writing-style rules; the 2026-07-24 prose pass did not reach these sites)
- Owner-gated: no

The partition leans on "genuine(ly)" (twelve lines), "silently" (nine), "loudly", "teeth", and "sound" in its loose sense. In nearly every sentence the word can be deleted or replaced by the mechanism already present in the next clause; "sound" has a technical meaning this crate's formal artifacts use. Some uses of "real" carry information ("real sockets" as a transport class, "a real accept" against a poll-once) and should survive.

Evidence:

        40	//! - **Cancellation mid-delivery.** The probe drops `accept` futures only
        41	//!   after a first delivery has genuinely surfaced, so an acceptor that
        42	//!   internally dequeues and then awaits is caught whenever that dequeue

    src/conformance/link/tests.rs:
       872	// ─── Negative controls: every check proves its teeth ────────────────────────
       873	//
       874	// Each violating fixture must FAIL its check, and each legal-adversity
       875	// fixture must pass with its adversity proven fired. A check whose negative
       876	// control stops failing has lost its teeth — these assertions are what make
       877	// a green suite mean anything.

    src/conformance/link/tests.rs:
        78	                // held. Swallowing an error here is sound for the wrapped

Resolution: A per-site pass: delete "genuinely" where the sentence survives (it does at every site read); replace "silently" with the mechanism ("without an error", "with the collecting accept never resolving") or drop it; "has lost its teeth" to "no longer fails its negative control"; "sound" to "correct" or "safe". Acceptance: a grep for the listed words over the partition returns only uses that carry information.

### conformance-5: Hand-maintained tallies and ratios in prose, one of them a compile-time relation the compiler could hold
- Where: src/conformance/link.rs:87-88 (related: src/link.rs:532, src/conformance/link/tests.rs:1074-1075, src/conformance/link/tests.rs:630, src/conformance/backend.rs:580-581, src/tree/mirror/streaming/window.rs:361, src/conformance/backend/tests.rs:66-68, src/conformance/backend/tests.rs:1-2)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (src/link.rs:532 `const MEMORY_STREAM_CAPACITY: usize = 8 * 1024;` is private; window.rs:361 reads `for window in [0usize, 1, 16, FAN].windows(2)`; tests.rs:630 `const CAPPED_STREAMS: usize = 4;`; tests.rs:129-132 runs `Local` as the only crate backend)
- Seen by: prose, perfapi; refutation: confirmed; history: no rationale found for four sites; the +0.7 GiB figure was placed inline deliberately by b204fe56 under the design-doc-citation ban, but the height-erasure note records that erasure "shrank every tower from the inside", so the measurement predates the code it now describes
- Owner-gated: no

Five docs restate enumerable facts the code can change without touching the prose: (a) "32 KiB is four times the in-memory reference's buffer" is a ratio to the private `MEMORY_STREAM_CAPACITY`, and if that constant grows past 32 KiB the duplex probe against `memory()` stops overrunning the buffer while `memory_link_conforms` keeps passing; (b) "the fixture's fifth open" is `CAPPED_STREAMS + 1`; (c) "a four-point `debug_assert`" counts an array literal in window.rs; (d) "+0.7 GiB of rustc peak memory per additional instantiation" is a dated measurement at a declaration site whose rationale (one type, so the tower instantiates once) stands without the number; (e) "this crate's backends" is plural for one. Principle 5: state the structure, not the tally; a number that matters lives in a mechanically enforced place.

Evidence:

        87	/// this much hidden buffering (see the module docs). 32 KiB is four times
        88	/// the in-memory reference's buffer and past common transport defaults,

    src/conformance/backend.rs:
       580	/// The derivation's own check is a four-point `debug_assert`, compiled
       581	/// out of release, so the suite sweeps the grid: every adjacent fan pair

    src/conformance/backend/tests.rs:
        66	/// backend type instantiates the whole height-indexed protocol tower
        67	/// (measured at +0.7 GiB of rustc peak memory per additional
        68	/// instantiation), so the honest and lying variants must share one type.

Resolution: (a) make `MEMORY_STREAM_CAPACITY` `pub(crate)` and pin the relation, `const _: () = assert!(CONTROL_DUPLEX_FILL > crate::link::MEMORY_STREAM_CAPACITY, "the duplex probe must overrun the reference link's control buffer");`, then let the prose state the inequality, not the ratio; (b) "the open past [`CAPPED_STREAMS`]"; (c) "a sparse `debug_assert` over a handful of fans"; (d) keep the mechanism ("a large compile-time cost per instantiation") and drop the figure; (e) "this crate's backend". Acceptance: no doc in the partition restates a count, ratio, or measurement a sibling constant or array literal owns, and a const assertion ties `CONTROL_DUPLEX_FILL` to `MEMORY_STREAM_CAPACITY`.

### conformance-6: A hang under `check` names no check
- Where: src/conformance/link.rs:147-157 (related: src/conformance/link.rs:21-26, tests/tcp_link.rs:54-61, tests/routed_link.rs:94-101)
- Class / severity / confidence: feature-gap / low / high
- Provenance: assessed (read; both in-tree consumers wrap the whole suite in one `timeout` whose expiry message is "conformance suite ran past its liveness bound")
- Seen by: perfapi; refutation: confirmed; history: no rationale found (nothing in the 2026-07-23 review considers attributing a hang to a check)
- Owner-gated: yes for a progress hook or `check_with` variant (new API); the doc fix is not gated

The module doc says the liveness clauses fail as hangs that only the caller's timeout can bound, and `check` runs seven probes sequentially under that one timeout. When it fires the caller learns that the suite hung, not which clause: both in-tree consumers report exactly that. A transport author then re-runs the focused checks by hand to bisect. The focused `check_*` functions exist and are public, but nothing tells the reader that attributing a hang is what they are for.

Evidence:

       147	/// Run the whole conformance suite against fresh pairs from `pair`.
       148	///
       149	/// Each check consumes one fresh pair (`pair` is called once per check),
       150	/// and every focused check probes both directions of its pair, so the
       151	/// suite validates an asymmetric implementation on each side's connector
       152	/// and acceptor. See the [module docs](self) for executor and timeout
       153	/// requirements.
       154	///
       155	/// # Panics
       156	///
       157	/// On the first violated contract clause, with a description of the clause.

Resolution: Minimal and non-breaking: add to `check`'s docs that a hang under the caller's timeout does not identify the check, and that the focused `check_*` functions exist so each can run under its own timeout. Owner option: a progress hook (`check_with(pair, on_check: impl FnMut(&'static str))`) reporting each check's name as it starts, so the consumer's expiry message can name the check it entered last. Acceptance: `check`'s rustdoc tells the reader how to attribute a hang, or a progress-reporting entry point exists and `tests/tcp_link.rs` and `tests/routed_link.rs` use it.

### conformance-7: Eight public signatures restate the same eight type parameters and eight bounds; the recorded reason lives only in a commit message
- Where: src/conformance/link.rs:158-169 (related: src/conformance/link.rs:191-203, src/conformance/link.rs:254-266, src/conformance/link.rs:332-344, src/conformance/link.rs:435-447, src/conformance/link.rs:709-721, src/conformance/link.rs:816-828, src/conformance/link.rs:1019-1031, src/link.rs:322-329, src/link.rs:435-478, src/link.rs:502-518)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (eight `pub async fn check*` signatures each carry the identical eight-line where clause; `pub struct Link<CR, CW, C, A>` at src/link.rs:322 carries no bounds and `into_parts` sits in the unbounded `impl<CR, CW, C, A> Link<...>` at 435-478, while `LinkParts::into_link` at 502-518 requires the full set)
- Seen by: structure, perfapi; refutation: confirmed; history: deliberate and holds (f5039abb: "Left alone deliberately: the five check_* functions' repeated 8-type-parameter signatures (the honest shape of Link's four parameters per end; a bounds-alias trait would add public API without shrinking the lists, and a macro would hide the rustdoc-visible signatures deployments read)")
- Owner-gated: yes: reopens a recorded ruling, and one alternative adds public API

The rendered rustdoc of every check is a screen of bounds that carry no per-check information; the decision to accept that was made and argued in f5039abb, but nothing at the site says so, and the sealed-helper-trait alternative (one impl, for `Link<CR, CW, C, A>`, exposing the four associated types and `into_parts`) is a different mechanism from the bounds alias that ruling rejected, since it does shrink the lists. Independently of that ruling, the bounds overstate what the focused checks need: they reach the halves through the unbounded `into_parts`, so `check_control` and `check_control_duplex` need only `AsyncRead`/`AsyncWrite + Unpin` on the halves, the four stream-only checks need no control-half bounds at all, and only `check_sessions` (through `counting` and `into_link`) needs the full set.

Evidence:

       158	pub async fn check<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
       159	    mut pair: impl AsyncFnMut() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
       160	) where
       161	    CRa: AsyncRead + Unpin + Send,
       162	    CWa: AsyncWrite + Unpin + Send,
       163	    Ca: Connector,
       164	    Aa: Acceptor,
       165	    CRb: AsyncRead + Unpin + Send,
       166	    CWb: AsyncWrite + Unpin + Send,
       167	    Cb: Connector,
       168	    Ab: Acceptor,

Resolution: Owner's call among three: (1) state f5039abb's rationale in a maintainer comment above `check` and leave the shape; (2) a sealed `LinkEnd` trait implemented once for `Link<CR, CW, C, A>` under the existing bounds, so each check reads `check_x<A: LinkEnd, B: LinkEnd>(a: A, b: B)`; (3) the non-breaking loosening: drop `Send` and the unused control-half bounds from the focused checks, keeping the full set on `check` and `check_sessions`. Acceptance: the shape is either argued at the site or changed; if (3), each focused check demands only the bounds its body uses and the in-tree consumers compile unchanged.

### conformance-8: `check_control`'s first sentence claims independence, the next check's clause
- Where: src/conformance/link.rs:186-190 (related: src/conformance/link.rs:243, src/link.rs:31-32)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read both bodies: `check_control` at 204-240 is a sequential ping-pong; `check_control_duplex` at 267-281 is the concurrent fill; src/link.rs:31-32 names the **Control duplex** clause with the word "independent")
- Seen by: prose; refutation: confirmed; history: no rationale found (the sentence predates `check_control_duplex`, whose first sentence 32813f55 also wrote as "independent")
- Owner-gated: no

The check proves ordered delivery in each direction and that the halves are not looped back; "independent" is the control-duplex clause's word and the very next check's first sentence. A doc comment's first sentence stands alone in the module listing and should say which-of-these-do-I-want.

Evidence:

       186	/// The control halves form two independent ordered byte pipes.
       187	///
       188	/// Each direction carries its own distinct probe bytes, so a wiring that
       189	/// loops a side's control write back to its own read fails the byte
       190	/// assertion instead of surfacing only as a hang on the other side.

Resolution: `/// The control halves deliver each direction's bytes, in order, to the peer.` Acceptance: the two adjacent checks' first sentences name distinct properties.

### conformance-9: The two independence probes are one probe parameterized by the stalled count
- Where: src/conformance/link.rs:465-678 (related: src/conformance/link.rs:521-524, src/conformance/link.rs:573-575, src/conformance/link.rs:631-636, src/conformance/link/tests.rs:889-896, src/conformance/link/tests.rs:993-1000)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read both probes in full; the receive loops at 529-560 and 641-668 differ only in `Option` versus `Vec` and the final count, the send halves only in how many stalled streams the pressure `join_all`s)
- Seen by: structure; refutation: confirmed; history: no rationale found (32813f55 added the pooled shape whole beside the existing probe and argued why the shape exists, not why it is a second function)
- Owner-gated: no

`probe_independence` (one stalled stream, `STREAM_COUNT - 1` live) and `probe_independence_pooled` (`STREAM_COUNT - 1` stalled, one live) are near-clones: about ninety lines kept in sync by hand, and already drifted. The never-completing `select` arm is `Either::Left((never, _)) => never` at 522 (a no-op, since the pressure block's output unifies with `()`) and `unreachable!(...)` at 632-634; `STALLED_COMPLEMENT` sits mid-file at 575 while every sibling constant lives in the 69-145 block. The suite is public production code and the deadlock-freedom argument rests on this clause; one probe whose only free variable is the stalled count makes the two coupling classes read as two points on one axis.

Evidence:

       529	        for _ in 0..STREAM_COUNT {
       530	            let (mut rx, _) = acceptor
       531	                .accept()
       532	                .await
       533	                .expect("contract: later streams are accepted beside a stalled one");
       534	            let mut tag = [0u8; 1];
       535	            rx.read_exact(&mut tag)
       536	                .await
       537	                .expect("contract: every stream's first byte is delivered");
       538	            match tag[0] {

       641	        for _ in 0..STREAM_COUNT {
       642	            let (mut rx, _) = acceptor
       643	                .accept()
       644	                .await
       645	                .expect("contract: later streams are accepted beside stalled ones");
       646	            let mut tag = [0u8; 1];
       647	            rx.read_exact(&mut tag)
       648	                .await
       649	                .expect("contract: every stream's first byte is delivered");
       650	            match tag[0] {

Resolution: Collapse to `async fn probe_independence<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A, stalled: usize)`. Sender: open `stalled` tagged streams into a `Vec`, `join_all` the pressure loops over them (a one-element `join_all` is the single case), open `STREAM_COUNT - stalled` live streams, one `select` with a single never-completing arm. Receiver: hold stalled receivers in a `Vec`, assert `held.len() == stalled` and `live_seen == STREAM_COUNT - stalled` (subsuming the single case's exactly-one assertion). `check_independence` calls it with `1` and `STALLED_COMPLEMENT` per direction; move `STALLED_COMPLEMENT` into the constant block; carry the two shapes' rationale as the parameter's doc plus one comment per call site. Acceptance: `check_independence` still runs four probes; `memory_link_conforms`, `one_byte_windows_conform`, `reordering_acceptor_passes_independence`, `never_binding_pooled_budget_conforms` pass; `shared_mux_coupling_is_caught` and `pooled_budget_below_the_bound_is_caught` still report `Err(Quiescence::Stalled)`.

### conformance-10: The `contract:` message prefix decorates a structural impossibility and is missing from a sibling assertion
- Where: src/conformance/link.rs:632-634 (related: src/conformance/link.rs:606-616, src/conformance/link.rs:1060-1069)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the pressure closures at 606-616: `loop { ... }` with no `break`, exiting only by `expect` panic, so `join_all` over them never completes)
- Seen by: prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

Throughout the suite `contract:` marks a panic message as a link-contract clause the transport violated. At 633 it prefixes an `unreachable!` no link behavior can reach, misfiling a programmer-error proof as a clause; a user reading the panic learns the wrong thing. At 1060-1064 the length assertion lacks the prefix its sibling at 1065-1069 carries for the same outcome.

Evidence:

       632	            Either::Left((_, _)) => {
       633	                unreachable!("contract: the pressure loops never complete")
       634	            }

      1060	    assert_eq!(
      1061	        seed.snapshot().len(),
      1062	        (2 * SESSION_PAYLOADS) as usize,
      1063	        "reconciliation over the link converged",
      1064	    );
      1065	    assert_eq!(
      1066	        seed.snapshot(),
      1067	        newcomer.snapshot(),
      1068	        "contract: reconciliation over the link converged on the same set",
      1069	    );

Resolution: At 633: `unreachable!("the pressure loops never break, so join_all over them never completes")` (dissolves with conformance-9's collapse). At 1063: add the prefix, or remove it from 1068. Acceptance: every `contract:`-prefixed message names a clause a transport could violate; the two convergence assertions are prefixed alike.

### conformance-11: `yield_once` is byte-identical to `testing::transport`'s copy, and both give a reason for existing that does not survive checking
- Where: src/conformance/link.rs:680-696 (related: src/testing/transport.rs:723-741, src/conformance/link/tests.rs:172, Cargo.toml:112, Cargo.toml:137)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (bodies at link.rs:685-696 and transport.rs:730-741 are identical; in tokio 1.52.3 from Cargo.lock, `mod yield_now; pub use yield_now::yield_now;` sits inside `cfg_rt! { ... }` at task/mod.rs:277-291 and `context::defer` at runtime/context.rs:168-176 falls back to `waker.wake_by_ref()` outside a runtime; Cargo.toml:137 gives the library tokio `io-util, macros, sync` and Cargo.toml:112 `test-internals = ["tokio/rt"]`)
- Seen by: structure; refutation: confirmed; history: no rationale found beyond the doc sentences themselves
- Owner-gated: no

Both docs justify the hand-roll as "Runtime-agnostic ... unlike `tokio::task::yield_now`", but tokio's `yield_now` is runtime-agnostic too. What excludes it from link.rs is that the library's tokio feature set omits `rt`, which gates `yield_now`; in transport.rs, where `test-internals` lights `rt`, tokio's own function is available. A reader who trusts the doc believes tokio's yield is unusable here for a reason that is false and never learns the true constraint that decides where a shared helper may live.

Evidence:

       680	/// Yield to the executor exactly once: `Pending` with an immediate
       681	/// self-wake.
       682	///
       683	/// Runtime-agnostic (the suite runs on the caller's executor, which may be
       684	/// no runtime at all), unlike `tokio::task::yield_now`.
       685	async fn yield_once() {

    src/testing/transport.rs:
       726	/// Runtime-agnostic (the deterministic driver is no runtime at all), unlike
       727	/// `tokio::task::yield_now`; a copy of `conformance`'s helper, on the same

Resolution: One `pub(crate) async fn yield_once()` in a module both callers reach (src/link.rs is the natural host, gated `#[cfg(any(test, feature = "conformance", feature = "test-internals"))]`), with the doc stating the real constraint: `tokio::task::yield_now` needs tokio's `rt` feature, which the library build does not enable, and the deterministic driver is not a tokio runtime, so the helper self-wakes. Import it at both sites; `LossyAcceptor` at link/tests.rs:172 follows. Acceptance: one definition of `yield_once` in src/; both suites compile under `--no-default-features --features conformance` and under `test-internals` alone; no doc claims `yield_now` is runtime-bound.

### conformance-12: The concurrency check's docs assert backpressured elders that exist only below the probe size
- Where: src/conformance/link.rs:700-708 (related: src/conformance/link.rs:47-49, src/conformance/link.rs:731-734, src/conformance/link.rs:751-754, src/link.rs:532, src/link.rs:550-551)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`PROBE` is 24 bytes by `printf | wc -c`; `memory()` builds every stream as `tokio::io::duplex(MEMORY_STREAM_CAPACITY)` with the constant 8 KiB at src/link.rs:532; each elder writes one index byte then `PROBE` at 745 and 756)
- Seen by: prose; refutation: confirmed; history: no rationale found (32813f55 wrote the unqualified doc and the qualified inline comment in the same commit)
- Owner-gated: no

The public `check_concurrency` doc, the private `probe_concurrency` doc, and the module-doc bullet all state that the youngest stream drains past elders sitting backpressured mid-write. Under `memory()` every elder's 24-byte write completes into an 8 KiB buffer and nothing sits mid-write; only `memory_with_capacity(1)` produces the described state. The inline comment at 751-754 carries the correct qualifier ("at small windows"); the docs above it claim a stronger probe than the one run at the default capacity, in a module whose "What the suite cannot see" section is careful about exactly this.

Evidence:

       700	/// This validates the concurrency clause's quantitative bound: all
       701	/// [`STREAM_COUNT`] streams held open at once, with the last-opened
       702	/// stream's bytes flowing to completion past its still-open,
       703	/// backpressured elders: the progress-beside-siblings the session's

       751	        // Every stream then writes its payload concurrently. The receiver
       752	        // drains the last-opened stream first, so at small windows the
       753	        // elder writers sit backpressured, on their own streams only,
       754	        // while the youngest completes.

Resolution: Qualify the three docs the way the inline comment does ("past its still-open elders, which sit backpressured where the window is smaller than the probe"), or strengthen the probe so elders always backpressure (write `STALL_FILL`-sized payloads on the elders) and keep the docs; the latter changes the public suite's behavior and is an owner call. Acceptance: the docs describe the probe at both tested capacities, or the probe backpressures elders at every capacity.

### conformance-13: `index as u8` ties the in-band index to `STREAM_COUNT <= 256` with nothing at compile time
- Where: src/conformance/link.rs:745-745 (related: src/conformance/link.rs:777-783, src/link.rs:169)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (`pub const STREAM_COUNT: usize = 17;` at src/link.rs:169; the read side at 778 is `usize::from(index[0])`)
- Seen by: refutation (new); refutation: raised; history: not examined
- Owner-gated: no

The truncating cast is correct today because `STREAM_COUNT` is 17, a fact defined in another file. A larger constant would alias indexes and fail the "exactly one stream carries each index" assertion at 780-783, so the failure is loud, but the reader has to go and check. The bar for finished code is obviously correct, not correct after a lookup.

Evidence:

       745	            tx.write_all(&[index as u8])

Resolution: `u8::try_from(index).expect("STREAM_COUNT fits an index byte")`, or a `const _: () = assert!(STREAM_COUNT <= u8::MAX as usize + 1);` beside the probe. Acceptance: the cast's precondition is stated where the cast is.

### conformance-14: Hand-rolled poll-once where `now_or_never` and `futures::poll!` are the crate's idiom
- Where: src/conformance/link.rs:847-853 (related: src/conformance/link.rs:61, src/conformance/link.rs:908-923, src/conformance/link/tests.rs:72-77, src/link/routed/tests.rs:430, src/tree/mirror/streaming/remote/proxy/work.rs:231, src/testing/transport.rs:828, Cargo.toml:52)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (grep locates `now_or_never` at src/link/routed/tests.rs:430 and across src/rumors/{changes,unordered,causal}.rs, and `futures::poll!` at proxy/work.rs:231 and transport.rs:828; Cargo.toml:52 enables futures' `async-await`, which `poll!` needs; equivalence of the rewrites assessed by reading, not compiled)
- Seen by: structure; refutation: confirmed; history: no rationale found; one constraint on record: d263a91d and the comment at link.rs:908-909 require the poll-drop cycle to use the real waker, which `futures::poll!` preserves and `now_or_never` would not
- Owner-gated: no

Three sites build a noop waker or a `poll_fn` to poll a future exactly once: link.rs:847-853 (assert a fresh accept is pending), link.rs:916-923 (poll a fresh accept once with the real waker), and link/tests.rs:72-77 (`ReversingAcceptor` draining what is `Ready`). `FutureExt::now_or_never` is the first and third shape; `futures::poll!` is the second. A reader who knows those idioms from elsewhere in rumors has to re-derive that these seven lines mean the same thing.

Evidence:

       847	        let mut pending = pin!(acceptor.accept());
       848	        let waker = futures::task::noop_waker();
       849	        let mut cx = Context::from_waker(&waker);
       850	        assert!(
       851	            pending.as_mut().poll(&mut cx).is_pending(),
       852	            "no stream was opened yet",
       853	        );

       916	            let polled_once = std::future::poll_fn(|cx| {
       917	                let mut accept = pin!(acceptor.accept());
       918	                Poll::Ready(match accept.as_mut().poll(cx) {
       919	                    Poll::Ready(rx) => Some(rx),
       920	                    Poll::Pending => None,
       921	                })
       922	            })
       923	            .await;

Resolution: 847-853: `assert!(acceptor.accept().now_or_never().is_none(), "no stream was opened yet");`. 916-923: `let polled_once = futures::poll!(pin!(acceptor.accept()));` as its own statement (so the fresh accept future drops at the statement's end, before the yield, preserving the current drop timing), then match `Poll::Ready`/`Poll::Pending`. link/tests.rs:72-77 dissolves with conformance-18, else `match self.inner.accept().now_or_never() { Some(Ok(rx)) => ..., _ => break }`. Prune the then-unused `Context` import at link.rs:61. Acceptance: no `noop_waker` or `poll_fn` remains in src/conformance/link.rs; `memory_link_conforms`, `lossy_accept_cancellation_is_caught`, and `asymmetric_lossiness_is_caught` keep their verdicts.

### conformance-15: Manual `Clone` impls identical to the derive, in a file whose other connectors derive it
- Where: src/conformance/link.rs:964-971 (related: src/conformance/link/tests.rs:573-580, src/conformance/link/tests.rs:263, src/conformance/link/tests.rs:746)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read: `CountingConnector<C>` holds `inner: C` and `opened: Arc<AtomicUsize>`; `CappedConnector<C>` holds `inner: C` and `permits: Arc<Semaphore>`; `MuxConnector` and `WindowedConnector` carry `#[derive(Clone)]`)
- Seen by: structure, correctness, perfapi; refutation: confirmed; history: no rationale found (32813f55 wrote both manual impls and both derives)
- Owner-gated: no

A hand-written `impl Clone` tells the reader the derive would be wrong (a looser bound, a non-field clone); here the bound and body are exactly what `#[derive(Clone)]` emits, and two sibling fixtures in the same test file derive.

Evidence:

       964	impl<C: Clone> Clone for CountingConnector<C> {
       965	    fn clone(&self) -> Self {
       966	        Self {
       967	            inner: self.inner.clone(),
       968	            opened: self.opened.clone(),
       969	        }
       970	    }
       971	}

Resolution: Replace both with `#[derive(Clone)]`. Acceptance: no `impl<C: Clone> Clone for` remains in src/conformance; `check_sessions` and `capped_stream_supply_is_caught` unchanged.

### conformance-16: Five `LinkParts` rebuilds spell the same five fields
- Where: src/conformance/link.rs:985-1007 (related: src/conformance/link/tests.rs:96-109, src/conformance/link/tests.rs:541-563, src/conformance/link/tests.rs:633-648, src/conformance/link/tests.rs:796-828, src/link.rs:481-519)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified (grep for `.into_link()` lists the sites; each read in full)
- Seen by: structure; refutation: confirmed, severity lowered (a struct literal must name every field and `SessionState` is only obtainable from `parts`, so a decorator cannot drop `session`, only mis-source it); history: no rationale found (f5039abb extracted `with_acceptor` and stopped; R40 made `SessionState`'s fields private, which removed the forging hazard)
- Owner-gated: no (a public `map_*` combinator on `LinkParts` is an owner option)

`counting`, `with_acceptor`, `coupled`, `capped`, and `windowed_pair`'s `rebuild` each write the five-field literal, changing one or two fields and copying the rest; struct-update syntax cannot help because the type parameters change. `with_acceptor` already abstracts the acceptor case; `capped` and `counting` hand-roll the symmetric connector case.

Evidence:

       995	    let parts = link.into_parts();
       996	    LinkParts {
       997	        control_read: parts.control_read,
       998	        control_write: parts.control_write,
       999	        connector: CountingConnector {
      1000	            inner: parts.connector,
      1001	            opened,
      1002	        },
      1003	        acceptor: parts.acceptor,
      1004	        session: parts.session,
      1005	    }
      1006	    .into_link()

Resolution: Local option: a `with_connector` twin of `with_acceptor`; since `counting` is production code, host both helpers privately in link.rs (`super::` reaches them from the tests). Owner option: `map_connector`, `map_acceptor`, `map_control` on `LinkParts`, which would also serve the crate-wide rebuilds in tests/ and src/testing. Acceptance: no fixture in the partition spells more than the fields it changes.

### conformance-17: The session-check comment claims many streams per side; the constant it sizes says one or two
- Where: src/conformance/link.rs:1045-1045 (related: src/conformance/link.rs:139-145, src/conformance/link.rs:1070-1078)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites and the final assertion `opened >= 1` per side at 1070-1078)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the comment predates 32813f55's measurement of "two on side a, one on side b", which restated the constant doc "at that strength" and left the comment)
- Owner-gated: no

Two statements of one fact disagree, and the constant doc is the one the final assertion is calibrated to. The reader is left to reconcile what the code cannot settle for them.

Evidence:

      1045	    // Divergence wide and deep enough to exercise many streams per side.

       139	/// Stream count follows the reconciled tree's depth, not the payload
       140	/// count: hashed leaf paths keep a corpus this size one or two levels
       141	/// deep, opening one or two streams per direction. The check's final
       142	/// assertion pins exactly what the sizing buys (every direction opened at
       143	/// least one data stream in-session), so it cannot rot silently; the
       144	/// many-streams regime is [`check_concurrency`]'s job.

Resolution: `// Divergence wide enough to open data streams in each direction; the many-streams regime is check_concurrency's job (see SESSION_PAYLOADS).` Acceptance: the inline comment and the `SESSION_PAYLOADS` doc make the same claim.

### conformance-18: `ReversingAcceptor` duplicates `testing::ReorderingAcceptor`, which this file already imports from
- Where: src/conformance/link/tests.rs:41-124 (related: src/conformance/link/tests.rs:24, src/conformance/link/tests.rs:134-150, src/conformance/link/tests.rs:907-920, src/testing.rs:7-12, src/testing/transport.rs:646-720, src/testing/transport.rs:667-669, src/lib.rs:319-321)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (src/testing.rs:10 re-exports `ReorderingAcceptor, reorder_accepts`; src/lib.rs:319-321 compiles `testing` under `cfg(any(test, feature = "test-internals"))`; link/tests.rs:24 already imports `crate::testing::{Quiescence, run_to_quiescence}`; transport.rs:646-720 read in full. The suite's behavior under the patience-waiting variant is assessed, not run.)
- Seen by: structure; refutation: confirmed; history: deliberate and holds (f5039abb lists the duplication under "Left alone deliberately", citing transport.rs's feature-isolation rationale; cbc4a0aa made `ReorderingAcceptor` wait because the Ready-only drain "was empirically pass-through" in the proxy topology)
- Owner-gated: yes: reopens a recorded ruling

The recorded rationale is one-directional: it keeps `testing` from depending on the public `conformance` feature. The direction the conformance tests need runs the other way, and this file already takes it at line 24. The one behavioral difference is that `ReorderingAcceptor` waits up to `REORDER_PATIENCE` yields for company while `ReversingAcceptor` takes only what is `Ready`; the patient variant subsumes the Ready-only one (it forms batches wherever the Ready-only one does), both are legal acceptors, and the `reordered > 0` assertions at 146-149 and 916-919 already guard against degeneration. Two reordering adversities with subtly different semantics is one more than the crate needs, and if the two are kept, the transport.rs comment should carry the actual reason they differ.

Evidence:

        50	struct ReversingAcceptor<A: Acceptor> {
        51	    inner: A,
        52	    held: VecDeque<(A::Rx, Done<A::Rx>)>,
        53	    /// Arrivals buffered before each reversed release.
        54	    batch: usize,
        55	    /// Batches of two or more released: genuine inversions.
        56	    reordered: Arc<AtomicUsize>,
        57	}

    src/testing/transport.rs:
       667	/// A sibling of the conformance suite's `ReversingAcceptor`
       668	/// (`src/conformance/link/tests.rs`), duplicated so this crate-internal seam
       669	/// does not depend on the public `conformance` feature.

Resolution: Delete `ReversingAcceptor` and `reversing`; in `reordered_accepts_conform` and `reordering_acceptor_passes_independence` call `crate::testing::reorder_accepts(a, 3, counter.clone())`, confirming `reordered > 0` still holds and the negative controls keep their verdicts under the patient wait. Then restate the transport.rs sibling comment without the reference (outside this partition). If the owner keeps two, the comment at transport.rs:667-669 should say why they differ (Ready-only drain for concurrently connected probes, patient wait for the proxy topology). Acceptance: `grep -rn ReversingAcceptor src` is empty and the two tests pass with nonzero `reordered`, or the rationale for two fixtures is stated where the duplicate is declared.

### conformance-19: Em-dashes in line comments (nine sites here, a crate-wide convention to rule on once)
- Where: src/conformance/link/tests.rs:66-68 (related: src/conformance/link/tests.rs:189, src/conformance/link/tests.rs:652, src/conformance/link/tests.rs:876, src/conformance/backend.rs:256, src/conformance/backend.rs:339, src/conformance/backend/tests.rs:252, src/conformance/backend/tests.rs:365, src/conformance/backend/tests.rs:386)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for the em-dash over the five files filtered to non-doc `//` lines returns exactly the nine sites; the same filter over src/ outside the partition returns 107 more lines)
- Seen by: prose; refutation: confirmed and scoped up (crate-wide); history: no rationale found (the owner's writing-style doctrine rules that the spaced double-hyphen is the dash of code comments; no gate leg checks it)
- Owner-gated: yes: a crate-wide convention, and a `tools/` lint leg is a gate decision

Nine `//` comments in the partition use a true em-dash; the convention is the spaced double-hyphen in code comments, the em-dash reserved for rendered rustdoc. With 107 further sites elsewhere in src/, the resolution is an owner ruling plus a mechanical check, not nine edits.

Evidence:

        66	        // Await one arrival, then swallow whatever else is immediately
        67	        // ready — without blocking, so a lone stream still flows — and
        68	        // release the accumulated batch newest-first.

Resolution: Rule once; then replace each `—` in a `//` comment with ` -- ` (or restructure with a colon or parentheses), and add a `tools/` lint leg so the convention is enforced rather than remembered. Acceptance: `grep -n '—' <file> | grep -v -E '^[0-9]+:\s*(///|//!)'` returns nothing for the five partition files, and a gate leg holds it.

### conformance-20: `check_control`, `check_streams`, and `check_sessions` have no negative control, against the module's own rule
- Where: src/conformance/link/tests.rs:872-877 (related: src/conformance/link.rs:72-80, src/conformance/link.rs:186-190, src/conformance/link.rs:325-331, src/conformance/link.rs:1009-1018)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the negative-controls section 872-1085 in full: mux and pooled budget for independence, two lossy tests for cancellation, coupled halves for control duplex, capped connector for concurrency; grep for `check_control\b|check_streams\b|check_sessions\b` outside link.rs returns nothing)
- Seen by: correctness, perfapi (open question); refutation: confirmed and extended to `check_sessions`; history: a standing owner ruling from the 2026-07-23 triage says "Every conformance check gets a negative control"; the older checks were never retrofitted
- Owner-gated: no

The section header claims every check proves its teeth; four of seven do. In particular the distinct-payload design of `check_control` exists so that a looped-back control half "fails the byte assertion loudly instead of only hanging" (link.rs:74-76, 188-190), a claim no fixture demonstrates, and `check_streams`'s abort-surfaces-as-EOF and exact-bytes clauses have no known-bad counterpart. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

       872	// ─── Negative controls: every check proves its teeth ────────────────────────
       873	//
       874	// Each violating fixture must FAIL its check, and each legal-adversity
       875	// fixture must pass with its adversity proven fired. A check whose negative
       876	// control stops failing has lost its teeth — these assertions are what make
       877	// a green suite mean anything.

Resolution: Add a looped-back control fixture (rebuild one end with its own `control_write` joined to its `control_read` through `tokio::io::duplex`) and a `#[should_panic(expected = "not this side's own")]` test on `check_control`; a truncating `Tx` wrapper that drops the final byte on shutdown or drop with a `should_panic(expected = "exact bytes")` test on `check_streams`, optionally an `Rx` wrapper that never surfaces EOF asserted `Stalled`. For `check_sessions`, either a fixture that passes every focused probe and fails sessions, or reword the header to name the checks it covers. Acceptance: each new test fails its check as asserted; the header's claim is true of every check in `check`.
Construction: wire `a.control_write` to `a.control_read` via `tokio::io::duplex` and run `check_control`: the a-side `read_exact` returns `CONTROL_PROBE_AB` and the `assert_eq!` against `CONTROL_PROBE_BA` panics.

### conformance-21: Vestigial braces around `send_all`, and qualified paths where the production sibling imports
- Where: src/conformance/link/tests.rs:1049-1057 (related: src/conformance/link/tests.rs:1037-1040, src/conformance/link/tests.rs:1058, src/conformance/link/tests.rs:8-24, src/conformance/link.rs:63-67)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`git blame -L 1049,1057` attributes the braces to 32813f55 and the `send_all` lines to 212c6914; `git show ce27df86 -- src/conformance/link/tests.rs` shows the blocks scoping `let mut batch = seed.batch();` before that commit)
- Seen by: structure, perfapi; refutation: confirmed; history: deliberate but expired (the braces scoped a `Batch` guard's `Drop` commit; ce27df86 deleted that `Drop` and 212c6914 replaced the closure with `send_all`)
- Owner-gated: no

A bare block tells the reader a drop point matters here; when nothing it encloses needs a scope, the reader searches for the guard. The same test spells `crate::Rumors<u64>`, `crate::Peer::seed()`, `crate::Peer::<u64>::bootstrap()`, and `futures::future::join` in full where link.rs imports them, because the test module never added the imports.

Evidence:

      1049	        {
      1050	            seed.send_all(0..2048u64)
      1051	                .expect("flat test payloads are within any depth limit");
      1052	        }
      1053	        {
      1054	            newcomer
      1055	                .send_all(2048..4096u64)
      1056	                .expect("flat test payloads are within any depth limit");
      1057	        }

Resolution: Remove the braces; add `use crate::{Peer, Rumors};` and `use futures::future::join;` to the test module's imports and use the short names at 1037-1040 and 1058. Acceptance: no bare single-statement blocks remain in `starved_pool_degrades_latency_not_liveness`; no `crate::` path in the test body.

### conformance-22: "seam" as the backend suite's structural vocabulary where every seam is a named trait method
- Where: src/conformance/backend.rs:17-20 (related: src/conformance/backend.rs:253, src/conformance/backend.rs:493, src/conformance/backend.rs:518-521, src/conformance/backend.rs:627, src/conformance/backend.rs:643, src/conformance/backend.rs:698, src/conformance/backend.rs:728, src/conformance/backend/tests.rs:229-231, src/conformance/backend/tests.rs:430-444, src/conformance/backend/tests.rs:452-517, src/tree/mirror/streaming/erased.rs:1)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -c -i seam`: 9 lines in backend.rs, 12 in backend/tests.rs, 27 in src/ outside the partition; the one place the crate defines the word is erased.rs:1, "The height-erased seam")
- Seen by: prose; refutation: confirmed; history: no rationale found (the writing-style doctrine lists "seam" as default dialect; the height-erasure use is the one anchored to an artifact)
- Owner-gated: yes: the usage is crate-wide

"The leaf seam", "the walk seam", "the assembly seam", "the parent seam", "bulk seams": each is a trait method (`Leaf::leaf`, `Backend::leaves`, `Backend::assemble`, `Backend::parent`) that names and links. The reader maps the metaphor to the method every time; the method name is shorter and carries no less.

Evidence:

        17	//! - **Bulk seams**: the backend's own [`leaves`](Backend::leaves) and
        18	//!   [`assemble`](Backend::assemble) overrides — the paths the wire codec
        19	//!   runs — are delegated to, their yields priced on the same census and
        20	//!   held to the walked or assembled node's aggregates.

    src/conformance/backend/tests.rs:
       229	        // The aggregate-lying knobs: a deflated answer must be caught by
       230	        // the assembly seam's floor, and an inflated leaf answer by the
       231	        // walk seam's aggregate-membership check.

Resolution: Where a seam is one method, name it ("at `Backend::leaves`", "the `assemble` floor", "the `parent` recurrence"); reserve "seam" for the height-erasure boundary erased.rs defines, if anywhere. Since the term is crate-wide, one owner ruling is cheaper than per-partition edits. Acceptance: `grep -n -i seam src/conformance` returns nothing, or only uses that link to a definition.

### conformance-23: The process-global ledger states its consequence (serialize tests) but not its cause (no handle reaches `Leaf::leaf`)
- Where: src/conformance/backend.rs:35-36 (related: src/conformance/backend.rs:87-96, src/conformance/backend.rs:247-265, src/conformance/backend.rs:631-634, src/tree/mirror/streaming/backend.rs:340-345)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`Leaf::leaf(version, message)` at streaming/backend.rs:340-345 is an associated function with no `self` and no backend argument; `Backend::node_bytes` and `Measure::measure` likewise)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (at 922db57a leaves charged zero and every charging site had `self`, so this was not the original reason; it is the current binding constraint since 8aeed2dd made `leaf` charge measured bytes)
- Owner-gated: no

A maintainer reading the suite expects the census to be a per-run `Arc<Ledger>` carried inside `Charged<B>`, and the module explains only that the global forces test serialization. The reason a handle cannot be threaded is that leaf construction receives no backend value, so `ChargedNode::leaf` (and `Drop`) can reach a ledger only through a static. That rejected alternative is what a maintainer-facing doc records, so the next reader does not attempt the refactor and hit the wall.

Evidence:

        35	//! The ledger is process-global, so checks in one process must not
        36	//! overlap (this module's tests hold one lock across each test body).

Resolution: One sentence at the ledger's declaration or in the accounting-premises paragraph: the census is static because leaf construction (`Leaf::leaf`) and the cost function are associated functions with no backend value to carry a per-run ledger through. Acceptance: the `ledger` module doc or the premises paragraph names the no-handle signatures as the reason.

### conformance-24: `Charged` leaves unchecked the `parent` Some/None clause and the bulk seams' ordering clauses the `Backend` docs say this suite convicts
- Where: src/conformance/backend.rs:325-326 (related: src/conformance/backend.rs:389-427, src/conformance/backend.rs:438-489, src/conformance/backend.rs:717-724, src/tree/mirror/streaming/backend.rs:120-135, src/tree/mirror/streaming/backend.rs:158-167, src/tree/mirror/streaming/backend.rs:179-197, src/conformance/backend/tests.rs:319-338)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read the `Backend` trait docs: `parent` at streaming/backend.rs:129-135 says "Given at least one real child, construction **must** yield a parent ... The backend conformance suite ... convicts a violating implementation in the backend's own tests"; `assemble` at 191-197 says "exactly one node per maximal run, in run order ... The backend conformance suite ... convicts a violating override"; `leaves` at 163-167 lists containment and ascending order with encoder-panic enforcement only. Read `Charged::parent`, `leaves`, and `assemble`.)
- Seen by: correctness (two findings); refutation: confirmed (parent), reframed (leaves' doc claims only encoder enforcement; assemble's run-order clause is the one claimed and unchecked); history: no rationale found (b3b72d248, a docs-only commit, wrote the conviction sentences; the suite's checks are those 5117498d landed, none of which covers these clauses)
- Owner-gated: no

Two `Backend` docs state that this suite convicts violations it does not check. `Charged::parent` computes `fan` and maps over the result: a `None` at `fan > 0` (or a `Some` at `fan == 0`) records no violation; in-session such a backend at best breaks convergence and trips `run`'s `converged` assertion with a different message, never a by-name conviction, and no knob demonstrates either direction. `Charged::assemble` looks runs up by prefix in a `BTreeMap`, which detects a wrong prefix but not a wrong order, so "in run order" is unchecked. `Charged::leaves` checks price, aggregate membership, and count, but not containment or ascending order (the trait doc claims only encoder-panic enforcement for those, so that pair is a plain gap, not a false claim). Statement faithfulness: prose that says a check exists must be true of the code; the production consequence, a decoder panic mid-session, is what a conformance check exists to catch first. This compounds with conformance-30: because the convergence oracle is agreement between two sides running the same backend, a `parent` fault that fires symmetrically evades both.

Evidence:

       325	        let parent = self.inner.parent(prefix, children).await?;
       326	        Ok(parent.map(|node| {

    src/tree/mirror/streaming/backend.rs:
       129	    /// reply to a request — and resolves to `None` the same way. Given at least
       130	    /// one real child, construction **must** yield a parent: under the default
       131	    /// [`assemble`](Self::assemble), a `None` here becomes a missing assembled
       132	    /// node, which the reply decoder treats as a backend contract violation
       133	    /// and enforces by panic. The backend conformance suite (see
       134	    /// [`crate::conformance`]) convicts a violating implementation in the
       135	    /// backend's own tests, before a live session can meet it.

    src/tree/mirror/streaming/backend.rs:
       191	    /// - exactly one node per maximal run, in run order (never merged,
       192	    ///   split, or skipped);
       193	    /// - each node at its own run's height-`H` prefix.
       194	    ///
       195	    /// The backend conformance suite (see [`crate::conformance`]) convicts
       196	    /// a violating override in the backend's own tests, before a live
       197	    /// session can meet it.

Resolution: In `Charged::parent`, after the inner call, record `ledger::violation("parent contract: fan {fan} yielded {Some/None}")` when `parent.is_some() != (fan > 0)`. In `Charged::assemble`, track the previous yielded `Prefix<H>` and record `unordered assembly` when not strictly ascending; in `Charged::leaves`, track the previous `Prefix<Z>` and the requested prefix, recording `unordered leaf walk` and `escaped leaf walk`. Add knobs to `Materializing` (`PARENT_DROPS` returning `Ok(None)` for one interior fan; a swap of the first two yielded items in `leaves` and `assemble`) with `should_panic` controls. Surface pending ledger violations in the message of `run`'s `converged` assertion, so the by-name report is not masked by the convergence panic. Acceptance: each new control fails by name; honest backends pass; both `Backend` conviction sentences are true of the code.
Construction: add a knob making `Materializing::parent` return `Ok(None)` for one interior fan and run `check(Materializing, MATERIALIZING_BUDGET)`: today the panic is "the conformance session must converge both corpora to one root" (or a decoder-side panic), not a ledger violation naming the clause. For order: add a knob that swaps the first two leaves of `Materializing::leaves`'s output; no violation is recorded (count and prices are unchanged).

### conformance-25: The `children` stream, the walk's in-flight references, is charged but never priced pointwise
- Where: src/conformance/backend.rs:363-377 (related: src/conformance/backend.rs:14-16, src/tree/mirror/streaming/materialized/common.rs:18-36, src/tree/mirror/streaming/window.rs:321-323, src/tree/mirror/streaming/window.rs:372-388, src/conformance/backend/tests.rs:340-356)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read `Charged::children`; window.rs:372-388 prices held references per depth at `node_bytes(children_quantile, version_bound)` and says a scope's "held references are the children of one depth-(d−1) parent"; the in-memory session reaches them through `children_of` at materialized/common.rs:28, `backend.clone().children(prefix, node)`)
- Seen by: correctness; refutation: confirmed (an honest `Local`, 8 <= 8, and an honest `Materializing`, header + bounds <= header + 24 * min(len, FAN) + bounds, pass the proposed check); history: no rationale found (the design note named assembled parents only; 8aeed2dd added the leaf check, 5117498d the bulk seams; nothing names `children`)
- Owner-gated: no

`Charged::children` measures each exploded child and puts it on the census, but never compares the measurement to `node_bytes`. The window prices exactly these values per depth, and in the walk-to-walk conformance session the only explode path is `children_of`, so the one population the budget derivation prices per reference is the one the pointwise check skips. An over-holding `children` override (a backend eagerly loading a child's full record) is caught only if the differenced end-to-end peak happens to exceed the budget, which is loose. The module doc's pointwise clause ("every node the session assembles is measured") leaves exploded nodes outside the check, and no negative control exists for them.

Evidence:

       368	        stream! {
       369	            let mut children = pin!(self.inner.children(prefix, parent.into_inner()));
       370	            while let Some(child) = children.next().await {
       371	                yield child.map(|(prefix, node)| {
       372	                    let measured = B::measure(&node);
       373	                    (prefix, ChargedNode::wrap(node, measured))
       374	                });
       375	            }

Resolution: In the `children` stream, price each yielded child at `B::node_bytes(node.len().min(FAN), bound_bytes(&node))` (the fan is invisible here; use the monotone cap `check_assembled` uses and argues) and record `underpriced child` when `measured > priced`. Add a `CHILDREN_SLACK` knob to `Materializing::children` (resize the lazily loaded row, like `WALK_SLACK`) with a `#[should_panic(expected = "underpriced child")]` control at `BULK_OVERHOLD`. Restate the module doc's pointwise bullet to include exploded children. Acceptance: the new control fails by name; `local_backend_conforms` and `materializing_backend_conforms` still pass.
Construction: set a `CHILDREN_SLACK` knob to 64 KiB in `Materializing::children` and run `check(Materializing, MATERIALIZING_BUDGET)`: today no violation is recorded at this seam, and the run passes unless the differenced peak crosses 4 MiB.

### conformance-26: `BOUND_SWEEP_CEILING`'s doc names a largest bound the grid exceeds by one
- Where: src/conformance/backend.rs:552-554 (related: src/conformance/backend.rs:561-569)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read `sweep_bounds`: `while power <= BOUND_SWEEP_CEILING { bounds.extend([power - 1, power, power + 1]); ... }`, so the last entry is `(1 << 20) + 1`)
- Seen by: correctness; refutation: confirmed; history: no rationale found (8d959048's message describes "powers of two with both neighbors to 1 MiB", consistent with the finding's reading)
- Owner-gated: no

The constant is the power-of-two ceiling, and the neighbor above it is swept too; the `sweep_bounds` doc already says each power comes with both neighbors.

Evidence:

       552	/// The sweep's largest version bound: 1 MiB, far past any canonical
       553	/// encoding the suite's corpus scale reaches, sampled at powers of two.
       554	const BOUND_SWEEP_CEILING: usize = 1 << 20;

Resolution: "The sweep's power-of-two ceiling: each power of two up to 1 MiB, with both neighbors, far past any canonical encoding the suite's corpus scale reaches." Acceptance: the doc matches `sweep_bounds`'s output.

### conformance-27: Redundant `+ Clone` on `B: Measure` at five sites; `node_bytes_monotone` demands a `Debug` bound it never uses
- Where: src/conformance/backend.rs:584-588 (related: src/conformance/backend.rs:646-649, src/conformance/backend.rs:673-676, src/conformance/backend.rs:729-732, src/conformance/backend.rs:747-753, src/conformance/backend.rs:82, src/tree/mirror/streaming/backend.rs:46)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`pub trait Backend: Clone + Send + Sync + 'static` at streaming/backend.rs:46; `pub(crate) trait Measure: Backend<Node<Z>: Leaf>` at backend.rs:82; `node_bytes_monotone`'s body at 589-614 calls only `B::node_bytes`)
- Seen by: correctness, perfapi; refutation: confirmed; history: no rationale found (redundant since the suite's first commit; 48bc31df carried it forward mechanically)
- Owner-gated: no

`Measure: Backend` and `Backend: Clone`, so `B: Measure + Clone` restates an implied bound and makes the reader check whether `Measure` might not imply `Clone`. The sweep's `B::Error: Debug` clause implies an `.expect` that is not there; `check`, `run`, `walk`, and `corpus` do `.expect()` on `B::Error` and keep theirs.

Evidence:

       584	fn node_bytes_monotone<B>()
       585	where
       586	    B: Measure + Clone,
       587	    B::Error: std::fmt::Debug,
       588	{

Resolution: Drop `+ Clone` at 586, 648, 675, 731, 752; drop the `Debug` clause from `node_bytes_monotone`. Acceptance: `grep -n 'Measure + Clone' src/conformance/backend.rs` returns nothing; the module compiles under `just test conformance`.

### conformance-28: The census ceiling has no liveness floor, and for `Local` the budget resolves to the floor window, so the end-to-end check passes vacuously while its testdoc says the budget binds
- Where: src/conformance/backend.rs:663-668 (related: src/conformance/backend/tests.rs:122-132, src/conformance/backend/tests.rs:189-191, src/tree/mirror/streaming/window.rs:175-176, src/tree/mirror/streaming/window.rs:190-192, src/tree/mirror/streaming/window.rs:397-399, src/tree/mirror/streaming/window.rs:436-473, src/tree/mirror/streaming/window.rs:535-542, src/tree/mirror/streaming/backend/local.rs:103-110, src/tree/typed/prefix.rs:17-21, src/link.rs:169)
- Class / severity / confidence: verification-gap / high / high
- Provenance: demonstrated (second witness pass: with `assert!(budget_peak > floor_peak)` inserted in `check`, `local_backend_conforms` failed at once, floor_peak and budget_peak both 14496 and the derived capacities all ones under `Budget(0)` and `Budget(65536)` alike; `materializing_backend_conforms` at 4 MiB admitted 1033 bytes above the floor, 193707 to 194740, so the floor holds there; before the pass, verified by arithmetic from constants read in the tree: `STREAM_COUNT = 17` (src/link.rs:169); `FAN = 256` (window.rs:132); `Local::node_bytes` is `size_of::<typed::Node<Z>>()`, const-asserted pointer-sized at local.rs:110, so 8; `Prefix<Z>` is `repr(transparent)` over tinyvec 1.11.0's `ArrayVec { len: u16, data: [u8; 32] }`, 34 bytes at alignment 2, so `(Prefix<Z>, Node<Z>)` is 48 and `FAN_SLOT_BYTES` is 40; `supply_fans = 17 × 257 × (8 + 40) = 209,712`, matching the value the window tests pin for `SUPPLY_DECODE_ENVELOPE_BYTES` at window/tests.rs:233-244 and the sync-budget note records; `64 * 1024 = 65,536`. The search at window.rs:448-456 starts at `lo = 1` and widens only when `charge(mid) <= budget`, and `charge` starts from `supply_fans`, so for `Budget(65,536)` as for `Budget(0)` every `charge(mid) > budget`, `lo` stays 1, and every capacity at 458-472 floors at one.)
- Seen by: prose, correctness, perfapi (open question); refutation: confirmed, folding the general statement into the constructed instance; history: deliberate but expired (the 64 KiB budget and the "genuinely binds" testdoc landed in 922db57a on 2026-07-22 23:09, before `from_budget` had a flat term; the flat pre-charge landed in b0304e711 on 2026-07-23 13:19 and the budget was never re-sized against it; `check` has had no floor since 922db57a)
- Owner-gated: no

`check` asserts only `admitted <= budget_bytes` over `budget_peak.saturating_sub(floor_peak)`. Under `Local` pricing the flat decode-fan pre-charge alone is 209,712 bytes, three times the 65,536-byte budget `local_backend_conforms` passes, so `WindowConfig::Budget(64 KiB)` resolves to the same all-ones window as `Budget(0)`, the two `run`s are identical, `admitted` is 0, and the ceiling cannot fail. The testdoc's "a tight budget that genuinely binds at this scale" is false, and nothing in `check` would have exposed it. Principle 2: a ceiling over a counter passes vacuously when the counter stops counting, so every ceiling needs a positive floor; and an inaccurate testdoc is a bug in the test. The `Materializing` run at 4 MiB is above its own flat term (demonstrated in the second witness pass: its budgeted peak exceeds the floor run's by 1033 bytes and the capacities differ from height 22 upward), so the floor holds there.

Evidence:

       663	    let admitted = budget_peak.saturating_sub(floor_peak);
       664	    assert!(
       665	        admitted <= budget_bytes,
       666	        "widening the window from the floor admitted {admitted} measured \
       667	         bytes at peak; the stated budget is {budget_bytes}",
       668	    );

    src/conformance/backend/tests.rs:
       124	/// `Local` is the trivial case — handles into a resident tree — so the
       125	/// suite's pointwise check reduces to the pointer-size constant, and the
       126	/// end-to-end census confirms the window's byte admittance under a tight
       127	/// budget that genuinely binds at this scale.
       128	#[test]
       129	fn local_backend_conforms() {
       130	    let _serial = serialized();
       131	    pollster::block_on(check(Local, 64 * 1024));

    src/tree/mirror/streaming/window.rs:
       397	        let supply_fans = (STREAM_COUNT as u128)
       398	            * (FAN as u128 + 1)
       399	            * (node_bytes(0, version_bound) as u128 + FAN_SLOT_BYTES as u128);

Resolution: In `check`, pair the ceiling with a floor: `assert!(budget_peak > floor_peak, "the budgeted window admitted nothing above the floor: the budget does not bind at this corpus scale")`, or resolve both `WindowConfig`s against the corpora and assert the budget window's widest capacity exceeds one. Name the `Local` budget (`LOCAL_BUDGET`, beside `MATERIALIZING_BUDGET`) and size it above the flat term, for example `SUPPLY_DECODE_ENVELOPE_BYTES + 64 * 1024` (both in scope under `cfg(test)`), with the constant's doc stating the term it must clear; restate the testdoc from the measured behavior. Re-check `MATERIALIZING_BUDGET` against its own flat term with the same floor. Acceptance: with the floor in place, the current `Local` budget fails the floor assertion; with the re-sized budget both conformance tests pass and the floor holds; the testdocs describe exactly what the body asserts.
Construction: add `assert!(budget_peak > floor_peak)` to `check` and run `local_backend_conforms`: it fails, demonstrating the two runs were identical. Alternatively instrument `run` to print the derived window's capacities for both configs and observe all ones in both.

Witness: the second witness pass ran the first form of this construction (`witness/results.md`, `## conformance-28`): `assert!(budget_peak > floor_peak, ..)` was inserted in `check` immediately before `let admitted = budget_peak.saturating_sub(floor_peak);`, together with `eprintln!`s of both peaks and of the capacities `Window::from_budget` derives at the corpus sizes under `Budget(0)` and under the test's budget; then `local_backend_conforms` and `materializing_backend_conforms` were run with `cargo nextest run -p rumors --all-features --no-capture`. Decisive output, `local_backend_conforms` (budget 64 KiB):

    WITNESS conformance-28: floor_peak=14496 budget_peak=14496 budget_bytes=65536
    WITNESS conformance-28: floor capacities (height 0..=32) = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
    WITNESS conformance-28: capacities identical under Budget(0) and Budget(65536): true
    WITNESS conformance-28: the budgeted run's peak (14496) did not exceed the floor run's peak (14496); the census ceiling below is vacuous
    test conformance::backend::tests::local_backend_conforms ... FAILED

and `materializing_backend_conforms` (budget 4 MiB):

    WITNESS conformance-28: floor_peak=193707 budget_peak=194740 budget_bytes=4194304
    WITNESS conformance-28: budget capacities (height 0..=32) = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 12, 12, 16, 25, 48, 49, 49, 49, 49, 1, 1]
    WITNESS conformance-28: capacities identical under Budget(0) and Budget(4194304): false
            PASS [   0.547s] (1/1) rumors conformance::backend::tests::materializing_backend_conforms

For `Local` the floor run and the budgeted run have identical peaks, so `admitted` is 0 and the ceiling cannot fail, while the added floor fails at once; the derived window is all ones under both budgets (computed with version-bytes 0, which can only overstate capacities). For `Materializing` at 4 MiB the budgeted run admits 1033 bytes above the floor and the capacities differ from height 22 upward, so the floor would pass there. The edit was restored afterwards.

### conformance-29: `run`'s doc says it returns the peak above the resting corpora; the ledger's peak is absolute
- Where: src/conformance/backend.rs:671-672 (related: src/conformance/backend.rs:114-121, src/conformance/backend.rs:663, src/conformance/backend.rs:708-715)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`reset_peak` stores `LIVE` into `PEAK` at 115-117; `run` calls it after the corpora exist at 708 and returns `ledger::peak()` at 715 and 725; the subtraction is `check`'s at 663)
- Seen by: prose; refutation: confirmed; history: no rationale found (doc and arithmetic born together in 922db57a)
- Owner-gated: no

`run` returns the high-water mark of absolute live bytes since `reset_peak` seeded it with the resting corpora included; the differencing that yields bytes above the corpora happens in `check`, between two absolute peaks. A maintainer reasoning from this sentence expects `run` to return a small number and misreads `check`'s `saturating_sub`.

Evidence:

       671	/// One controlled-divergence reconciliation; returns the ledger's peak
       672	/// measured bytes above the resting corpora.

Resolution: "One controlled-divergence reconciliation; returns the ledger's peak live bytes during the session, the resting corpora included. `check` differences two such peaks to isolate the window's own admittance." Acceptance: the doc names what the function returns and attributes the differencing to `check`.

### conformance-30: `run`'s convergence oracle is agreement between the two sides, not the expected union
- Where: src/conformance/backend.rs:717-724 (related: src/conformance/backend.rs:617-621, src/conformance/backend.rs:636-645)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; `COMMON` and `DIVERGENT` at 617-621 make the expected size `COMMON + 2 * DIVERGENT`)
- Seen by: correctness; refutation: confirmed; history: no rationale found (the sync-budget note records root-hash equality as the mechanism, not a reason against a stronger oracle)
- Owner-gated: no

Convergence is witnessed by root-hash equality alone. Two sides running the same backend through the same code that both lose the same subtree still agree; the expected result is known and comparing against it costs one line. The `check` doc lists "the session failed to converge the corpora" as a panic cause, which the current check only partially delivers, and conformance-24's symmetric `parent` fault would pass it.

Evidence:

       717	    let converged = match (&ours.root, &theirs.root) {
       718	        (Some(left_root), Some(right_root)) => left_root.hash() == right_root.hash(),
       719	        _ => false,
       720	    };

Resolution: Also assert `left_root.len() == COMMON + 2 * DIVERGENT`, or build `corpus(common ⊕ left_tail ⊕ right_tail)` once and compare its root hash with both sides'. Acceptance: the assertion holds on the honest runs and fails on a session that drops any leaf on both sides.
Construction: wrap both `Handshaking::start` roots so each side's corpus omits the same shared leaf: today the check passes (equal hashes) while the reconciled set is short one message.

### conformance-31: The `assemble` seam is exercised only at the root over one run; two of its violation classes have never fired
- Where: src/conformance/backend.rs:770-774 (related: src/conformance/backend.rs:466-469, src/conformance/backend.rs:478-488, src/conformance/backend.rs:775-783, src/tree/mirror/streaming/erased.rs:319-334, src/tree/mirror/streaming/remote/adapter/decode.rs:88, src/tree/mirror/streaming/remote/adapter/decode.rs:408, src/tree/mirror/streaming/materialized/work/assembly.rs:88, src/tree/mirror/streaming/backend/local.rs:201-220, src/conformance/backend/tests.rs:159-164)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (grep for `.assemble::<|ops::assemble` outside conformance: `erased::ops::assemble` at erased.rs:319-334 is the only route to `Backend::assemble`, and its callers are the wire decoder at decode.rs:88 and 408; the in-memory session's assembly folds through `ops::parent` at work/assembly.rs:88; so in the conformance run the only `Charged::assemble` call is `corpus`'s)
- Seen by: correctness; refutation: confirmed; history: no rationale found (5117498d's six negative controls include neither `unsupplied assembly` nor `unassembled run`; nothing records the sub-root, multi-run regime as out of scope)
- Owner-gated: no

The only call into `Charged::assemble` is `corpus`'s `assemble::<height::Root>` over one sorted run, so the run ledger ever holds one entry. `unsupplied assembly` (a node yielded at a prefix with no run) cannot fire because at `Root` there is one possible prefix; `unassembled run` cannot surface by name because a wholly dropped root trips the `expect` at 778 first. Neither has a knob or a control. Meanwhile the wire decoder runs assembly at scope heights over many runs, the regime the module doc says this seam covers ("the paths the wire codec runs"), and `Local::assemble`'s run boundary (`current != Some(target)` at local.rs:208) is never driven by this suite. A criterion the bad implementation also passes is decoration.

Evidence:

       770	    let mut assembled = pin!(
       771	        charged
       772	            .clone()
       773	            .assemble::<height::Root>(Box::pin(futures_stream::iter(leaves.into_iter().map(Ok))))
       774	    );

Resolution: In `run`, before `reset_peak`, also drive `charged.clone().assemble::<H>(...)` over each corpus's sorted leaves at a sub-root `Convert` height (a one-byte prefix yields up to 256 runs at this corpus scale) and drain it. Add two knobs to `Materializing::assemble`: drop the k-th assembled node (`unassembled run`) and re-tag one node's prefix to a neighbor (`unsupplied assembly`), each with a `should_panic` control. The `bulk-assembled len` and over-hold controls then also run in the multi-run regime. Acceptance: both new controls fail by name; the honest suites pass; the multi-run `Local::assemble` boundary is covered.
Construction: add a knob to `Materializing::assemble` that `skip(1)`s the assembled node stream (not the leaves) and run `check`: today the run panics at "a non-empty corpus assembles a root", not with `unassembled run`.

### conformance-32: "honest", "lying", and `Dishonest` as the reference fixture's vocabulary collide with the model's term of art
- Where: src/conformance/backend/tests.rs:62-72 (related: src/conformance/backend/tests.rs:29-36, src/conformance/backend/tests.rs:101-112, src/conformance/backend/tests.rs:134-177, src/conformance/backend.rs:577, src/link.rs:538, src/tests.rs:558)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (about fifty matching lines in backend/tests.rs, anchored by the type `Dishonest` at 103 and the field `Knob.honest`; elsewhere "honest" is the peer or tree sense: src/link.rs:538 "an honest peer", src/tests.rs:558 "Honest, causally concurrent divergence")
- Seen by: prose; refutation: confirmed; history: no rationale found (922db57a coined "honesty knob", d8afb610 added `Dishonest`; nothing weighs the term against AGENTS.md's "authenticated-honest-peer")
- Owner-gated: yes: a rename across the fixture

A backend author whose cost function is wrong is not a dishonest peer; `Dishonest` reads as the hostile-peer regime AGENTS.md declares off-model. The intended notion is the accuracy of a self-report, and moralized code is a listed tell.

Evidence:

        62	/// A reference-backend honesty knob: process-global state resting at an
        63	/// honest value.
        64	///
        65	/// State rather than a const parameter deliberately: every distinct
        66	/// backend type instantiates the whole height-indexed protocol tower
        67	/// (measured at +0.7 GiB of rustc peak memory per additional
        68	/// instantiation), so the honest and lying variants must share one type.

Resolution: Rename to accuracy terms: `Knob.honest` to `resting` or `accurate`, `Dishonest` to `Skewed` (or `Misstated`), "lying tests" to "skewed runs", "a lie" to "a misstatement"; the type names anchor the vocabulary so the rename is mechanical. Acceptance: `grep -n -i -E 'honest|lying|dishonest|\blie\b' src/conformance/backend` returns nothing, or only the reworded "pointwise-accurate" at backend.rs:577.

### conformance-33: `bounds_of` in the tests duplicates `bound_bytes` in the suite
- Where: src/conformance/backend/tests.rs:260-263 (related: src/conformance/backend/tests.rs:255, src/conformance/backend/tests.rs:335, src/conformance/backend/tests.rs:351, src/conformance/backend.rs:268-275, src/conformance/backend.rs:348-351, src/conformance/backend.rs:522-526, src/tree/mirror/streaming/backend/local.rs:34-57, src/tree/typed/node.rs:178-182)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`impl<H: Height> Node for typed::Node<H>` at local.rs:34-57 delegates `span()` to the inherent method, documented at node.rs:178-179 as "the memoized `[floor, ceiling]` pair", so `span().hi()`/`lo()` are `ceiling()`/`floor()` and the two functions compute one number)
- Seen by: structure; refutation: confirmed; history: no rationale found (both born in 922db57a; 68194d19 rewrote `bound_bytes` onto `span()` and left `bounds_of`)
- Owner-gated: no

The reference backend's row pricing should visibly use the same bound arithmetic the suite checks it against, so a reader cannot wonder whether the two differ; a child module reaches the parent's private fn as `super::bound_bytes`. backend.rs also spells "max of the two bound encodings" twice (349-351, 523-526).

Evidence:

       260	/// The two encoded bounds of a freshly built node, in bytes.
       261	fn bounds_of<H: Height>(node: &typed::Node<H>) -> usize {
       262	    node.ceiling().as_bytes().len() + node.floor().as_bytes().len()
       263	}

    src/conformance/backend.rs:
       268	/// The two encoded version bounds a node keeps resident, in bytes.
       269	fn bound_bytes<N>(node: &N) -> usize
       270	where
       271	    N: Node,
       272	{
       273	    let bounds = node.span();
       274	    bounds.hi().as_bytes().len() + bounds.lo().as_bytes().len()
       275	}

Resolution: Delete `bounds_of`; call `super::bound_bytes(&node)` at 255, 335, 351. In backend.rs, name the repeated max once (`fn bound_version_bytes(node: &impl Node) -> usize`) and use it at 349-351 and 523-526. Acceptance: one bound-bytes helper and one bound-max helper in the module; `materializing_backend_conforms` and its controls unchanged.

### conformance-34: The underpricing testdoc places detection at "the moment" of assembly; the report lands at run end, and the leaf check records the same lie first
- Where: src/conformance/backend/tests.rs:417-422 (related: src/conformance/backend.rs:16, src/conformance/backend.rs:258-263, src/conformance/backend.rs:330-336, src/conformance/backend.rs:656-661)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (violations are pushed to the ledger and reported after both runs at backend.rs:656-661; with `PRICED_HEADER` at 0 the leaf check at 258-263 records `underpriced leaf` at corpus construction before any parent assembles; `run` builds the corpora before the session at 695-712)
- Seen by: prose; refutation: reframed (imprecise, not false: "caught" describes recording, and "by name" is the crate's idiom for the named clause); history: deliberate but expired for the "moment" wording (accurate at 922db57a when leaves were unchecked; 8aeed2dd added the leaf check)
- Owner-gated: no

The doc describes detection but not reporting: the lie is recorded when a parent assembles and surfaces only when `check` completes both runs, and the leaf seam records it before any session. A maintainer debugging a late-reported violation would look for an assembly-time panic that does not exist.

Evidence:

       417	/// An underpricing cost function fails the run by name.
       418	///
       419	/// The same backend with a header priced below the row's real header
       420	/// must be caught by the pointwise check the moment a session assembles
       421	/// a node — this is the suite's reason to exist, so its detection is
       422	/// itself pinned.

Resolution: "... is recorded on the ledger when a session assembles a node (the leaf check at construction records the same lie first) and fails the run by name when it completes ...". Acceptance: the testdoc's account of when the failure surfaces matches `check`'s reporting point.

### conformance-35: The ledger test's doc claims drops settle the balance, but the body observes settlement only through an unexplained side effect, and not at all for the second pair
- Where: src/conformance/backend/tests.rs:543-547 (related: src/conformance/backend/tests.rs:561-568, src/conformance/backend/tests.rs:576-587, src/conformance/backend.rs:98-121, src/conformance/backend.rs:191-197)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the `ledger` module at backend.rs:88-130 exposes no `LIVE` accessor; `PEAK` is monotone between resets and `reset_peak` copies `LIVE` into it; every assertion in the test reads `ledger::peak()`)
- Seen by: structure, prose; refutation: reframed (a no-op `Drop` would fail this test at 576-580 with `before + 216` and the message "two live handles charge twice", so the first pair's settlement is checked indirectly and mis-attributed; the second pair's is never checked); history: no rationale found (8aeed2dd reworded the testdoc to claim settlement without adding a live read)
- Owner-gated: no

A `Drop` that fails to discharge is the census's most consequential bug (every later peak inflates), and this is the test named for catching it; it does so for the first pair only because a failure would inflate the second pair's peak, which nothing in the test explains, and the final assertion at 583-587 pins peak persistence, a different property. The testdoc states an invariant the body should check directly; `reset_peak` seeds `PEAK` from `LIVE`, so a reset-then-read is the live read the test needs.

Evidence:

       543	/// The decorator's ledger accounting is exact over wrap, clone, and drop.
       544	///
       545	/// A wrapped leaf charges its measured post-custody bytes, a cloned
       546	/// handle charges its bytes again, and drops settle to the starting
       547	/// balance — the arithmetic the end-to-end census rests on.

       583	    assert_eq!(
       584	        ledger::peak(),
       585	        before + 200,
       586	        "the peak persists after handles settle",
       587	    );

Resolution: After each drop pair, `ledger::reset_peak(); assert_eq!(ledger::peak(), before, "drops settle the ledger to the starting balance");` (or add a `ledger::live()` accessor and assert it). Keep the persistence assertion only if peak persistence is meant to be pinned; then say so in the doc. Acceptance: deleting the `ledger::discharge` call in `impl Drop for ChargedNode` fails this test at a direct settle assertion; the doc's three clauses each map to an assertion.
Construction: delete `ledger::discharge(self.bytes)` in `ChargedNode::drop` (backend.rs:191-197): today the test fails only at 576-580, with a message about charging twice, not settling.

### conformance-36: Dead `Charged` value in the ledger test, and a long path where `Z` is already imported
- Where: src/conformance/backend/tests.rs:570-573 (related: src/conformance/backend/tests.rs:14, src/conformance/backend/tests.rs:24, src/conformance/backend/tests.rs:555, src/conformance/backend.rs:145-149)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git blame -L 570,573` attributes 572-573 to 922db57a, and `git show 922db57a:src/conformance/backend/tests.rs` has the same dead pair at its lines 244-245; `Z` is imported at line 24 and used bare at 555; `Charged::new` at backend.rs:145-149 charges nothing)
- Seen by: structure, correctness, perfapi; refutation: confirmed; history: no rationale found (dead from the suite's first commit)
- Owner-gated: no

`charged` is constructed and then only borrowed into `let _`; nothing reads it, and its presence makes a reader check whether `Charged::new` charges the ledger (it does not). Removing it retires the `Charged` import at line 14. The line above spells `crate::tree::typed::height::Z` where `Z` is imported and used bare fifteen lines earlier.

Evidence:

       570	    let node: typed::Node<crate::tree::typed::height::Z> =
       571	        typed::Node::leaf(Version::new(), Message::new(7));
       572	    let charged = Charged::<Local>::new(Local);
       573	    let _ = &charged;

Resolution: Delete lines 572-573; drop `Charged` from the `use super::{...}` at line 14; write `typed::Node<Z>` at 570. Acceptance: `ledger_settles_over_clone_and_drop` compiles without the `Charged` import and passes; no `let _ = &` remains in the file.

### conformance-37: The cancellation probe requires two connects to complete while the peer's acceptor is unpolled, a precondition the link contract does not state
- Where: src/conformance/link.rs:867-872 (related: src/conformance/link.rs:884-887, src/conformance/link.rs:855-859, src/conformance/link.rs:28-55, src/conformance/link.rs:805-815, src/link.rs:51-54, src/link.rs:534-539, src/link.rs:563-564, src/link.rs:598-605, src/link/routed/router.rs:105, src/link/routed/router.rs:249, src/tree/mirror/streaming/remote/proxy/work.rs:212-220, src/tree/mirror/streaming/remote/streams.rs:694-706)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: verified (read the probe's ordering at 861-887; `grep -n -i 'backlog\|rendezvous\|unaccepted' src/link.rs` hits only the `MEMORY_STREAM_BACKLOG` block at 534-539, so the contract section at 24-68 has no such clause; both in-tree connectors queue `STREAM_COUNT` unaccepted opens, `mpsc::channel(MEMORY_STREAM_BACKLOG)` at src/link.rs:563-564 with `MEMORY_STREAM_BACKLOG = STREAM_COUNT` and `mpsc::channel(STREAM_COUNT)` at router.rs:105 and :249; the protocol's own acceptor is polled beside the session in the biased `select!` at work.rs:212-220 and loops on `accept_one` at streams.rs:694-706)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed (the refutation prefers resolution (2): the protocol never needs the backlog); history: no rationale found (the connect-before-signal ordering is deliberate and argued inline at link.rs:855-859, from cbc4a0aa and a3da46c4, and the first-real-accept bridge came from the link-transport review; nothing considers the precondition that ordering imposes on the transport, and the routed-link record assumes a backlog exists without elevating it to a clause)
- Owner-gated: yes: whether the precondition becomes a contract clause in src/link.rs or a stated requirement of the suite

`probe_cancellation` connects `CANCELLED_DELIVERIES` streams and only then signals the receiving half, which awaits that signal before its first `accept`, so both opens must complete while the peer's acceptor is unpolled: the transport has to hold at least two opened, unaccepted streams. The contract's Concurrency clause forbids serializing an open behind *unrelated* stream progress and says nothing about the peer's acceptance of the same stream, and the memory link's backlog comment says the protocol never relies on one ("the accept loop drains continuously"), which the protocol's code bears out: its acceptor is polled beside the session in the biased `select!` at work.rs:212-220 and loops at streams.rs:694-706. A transport whose `connect` completes only when the peer accepts (a rendezvous over a zero-capacity channel) satisfies every stated clause and hangs this check as an unattributed liveness failure. No committed transport exercises the case: the memory and routed connectors each queue `STREAM_COUNT` unaccepted opens, and TCP has the kernel's listen backlog. A public suite's preconditions must be the contract's clauses, and the module keeps a "What the suite cannot see" section for statements of exactly this kind; what the suite additionally *requires* belongs beside it.

Evidence:

       867	        let mut streams = Vec::with_capacity(CANCELLED_DELIVERIES);
       868	        for _ in 0..CANCELLED_DELIVERIES {
       869	            let (tx, _) = connector.connect().await.expect("contract: connect");
       870	            streams.push(tx);
       871	        }
       872	        let _ = connected.send(());

    src/conformance/link.rs:
       884	    let receive = async {
       885	        in_flight
       886	            .await
       887	            .expect("the sending half signals after connecting");

    src/link.rs:
        51	//! - **Concurrency.** Up to [`STREAM_COUNT`] streams per direction may be
        52	//!   open at once, while [`Connector::connect`] calls arrive sparsely and
        53	//!   mid-session. An instantiation must not require a full complement of
        54	//!   streams, nor serialize an open behind unrelated stream progress.

    src/link.rs:
       536	/// A session opens at most [`STREAM_COUNT`] streams, sessions are
       537	/// serialized, and the accept loop drains continuously, so this bound is
       538	/// never the limiting factor for an honest peer.

Resolution: Owner decision between (1) adding a clause to the contract in src/link.rs stating that `connect` completes without the peer polling `accept`, for at least `STREAM_COUNT` outstanding opens per direction (the bound both in-tree connectors already provide), cited from the probe; and (2) stating the probe's precondition in the module docs' requirements section and in `check_accept_cancellation`'s rustdoc, so a transport author knows why an otherwise conforming rendezvous connect hangs this check. The refutation and history passes both point at (2), because the protocol's acceptor loop never needs the backlog, so the requirement is the probe's alone. Either way, the sentence at src/link.rs:536-538 and the new statement should agree on whether the protocol itself relies on a backlog. Acceptance: either src/link.rs's contract section carries a clause the probe's precondition follows from, or the conformance module docs name the precondition beside the other liveness caveats; `memory_link_conforms`, `tests/tcp_link.rs`, and `tests/routed_link.rs` unchanged.

### conformance-38: The lossy-acceptor control's verdict is window-dependent: below the probe's length the sender panics with `contract: stream write`, blaming the connector
- Where: src/conformance/link.rs:877-882 (related: src/conformance/link.rs:805-815, src/conformance/link.rs:916-923, src/conformance/link/tests.rs:168-174, src/conformance/link/tests.rs:922-938, src/conformance/link/tests.rs:33-39, src/link.rs:532, src/link.rs:598-605, src/testing.rs:375-394)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (the poll sequence traced under `run_to_quiescence` against tokio 1.52.3's `src/io/util/mem.rs` in the registry copy Cargo.lock pins: `impl Drop for DuplexStream` calls `close_read` at 175-181, `close_read` sets `is_closed` and wakes the parked writer at 240-246, and `poll_write_internal` returns `BrokenPipe` when `is_closed` at 278-279; `PROBE` is 24 bytes; not run)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed (traced the same tokio path independently); history: no rationale found (d263a91d pinned the lossy control as a stall at the default window and "fails here as a hang on a collecting accept" arrived with a3da46c4; one-byte windows were considered on the passing path only, in `one_byte_windows_conform`)
- Owner-gated: no

`check_accept_cancellation`'s doc says a lossy acceptor "fails here as a hang on a collecting accept", and `lossy_accept_cancellation_is_caught` pins `Err(Quiescence::Stalled)` at `memory()`'s 8 KiB window, where both 24-byte `PROBE` writes and both `tx` drops complete inside the sender's first poll, so nothing is mid-write when a poll-drop cycle drops the dequeued `rx`. At any capacity smaller than `PROBE.len()` the second writer is still parked on its pipe when the cycle drops the `LossyAcceptor` future holding that stream's `rx`: `DuplexStream`'s `Drop` calls `close_read`, which sets `is_closed` and wakes the parked writer, whose next `poll_write` returns `BrokenPipe`, and the probe panics at `expect("contract: stream write")`, a message that indicts the connector for the acceptor's loss. The committed control runs at one capacity, so nothing pins which verdict a small-window transport gets, and the attribution the suite promises (`check`'s `# Panics`: "with a description of the clause") is exactly what this path gets wrong.

Evidence:

       877	        join_all(streams.into_iter().map(|mut tx| async move {
       878	            tx.write_all(PROBE).await.expect("contract: stream write");
       879	            tx.flush().await.expect("contract: stream flush");
       880	            drop(tx);
       881	        }))
       882	        .await;

    src/conformance/link.rs:
       810	/// in flight. An acceptor that internally dequeues a delivery and then
       811	/// awaits before returning it drops the dequeued stream with the
       812	/// cancelled future, and fails here as a hang on a collecting accept. A

    src/conformance/link/tests.rs:
       930	#[test]
       931	fn lossy_accept_cancellation_is_caught() {
       932	    let (a, b) = memory();
       933	    assert_eq!(
       934	        run_to_quiescence(super::check_accept_cancellation(a, lossy(b))),
       935	        Err(Quiescence::Stalled),
       936	        "the lost delivery must surface as a stall at the collecting accept",
       937	    );
       938	}

Resolution: Run the lossy controls at both tested capacities (a second test over `memory_with_capacity(1)` beside `lossy_accept_cancellation_is_caught`, or one test iterating both), and decide the verdict deliberately: either accept the sender-side failure and have the write's `expect` name both causes ("contract: stream write failed while the link is healthy: the transport errored, or the acceptor dropped a delivery under it"), or make the cancellation probe's writers tolerate `BrokenPipe` so the loss always surfaces at the collecting accept as documented. Update `check_accept_cancellation`'s doc to describe the failure at every tested capacity. Acceptance: a committed control runs the lossy fixture at a window smaller than `PROBE.len()` and asserts the chosen verdict; `check_accept_cancellation`'s rustdoc describes the failure a lossy acceptor produces at both tested capacities.
Construction: in src/conformance/link/tests.rs, `let (a, b) = memory_with_capacity(1); run_to_quiescence(super::check_accept_cancellation(a, lossy(b)))`. Expected today: a panic carrying "contract: stream write" rather than `Err(Quiescence::Stalled)`. If it stalls instead, the reading of the pipe's close semantics is wrong and this entry should be dropped.

### conformance-39: `WindowedTx` grants and debits the shared window in separate lock scopes; two writers interleaving between them double-spend and the second debit underflows
- Where: src/conformance/link/tests.rs:691-703 (related: src/conformance/link/tests.rs:650-658, src/conformance/link/tests.rs:722-742, src/conformance/link/tests.rs:993-1000, src/conformance/link/tests.rs:1009-1015, src/conformance/link/tests.rs:1034-1068, src/testing.rs:366-394)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read; every committed use runs under `run_to_quiescence`, a single-threaded poll loop in which one `poll_write` is atomic against every other poll, so no committed test reaches the interleaving)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed; history: no rationale found (`WindowedTx::poll_write` is byte-identical since 32813f55, whose message describes what the fixture pins and says nothing about lock scoping; the fixture comment at 650-658 states no threading premise)
- Owner-gated: no

The grant is computed under one lock acquisition, the lock is released, the inner `poll_write` runs, and the debit `window.available -= written` runs under a second acquisition. Two `WindowedTx` writers on different tasks can both read `available = n`, both write `n`, and the second debit computes `0 - n`: a panic in debug builds, a wrap to near `usize::MAX` in release, which turns the starving pool the negative control depends on into one that never binds. Every committed use runs under `run_to_quiescence`, where one `poll_write` is atomic against every other poll, so the tests cannot reach it; but the fixture's section comment presents the window as a model of the connection-level flow control real multiplexed transports use and states no single-poll premise. Correct for all inputs applies to a fixture that models a transport property, and the fix is one scope.

Evidence:

       691	        let grant = {
       692	            let mut window = self.window.lock().expect("window lock");
       693	            if window.available == 0 {
       694	                window.writers.push(cx.waker().clone());
       695	                return Poll::Pending;
       696	            }
       697	            window.available.min(buf.len())
       698	        };
       699	        let result = Pin::new(&mut self.inner).poll_write(cx, &buf[..grant]);
       700	        if let Poll::Ready(Ok(written)) = &result {
       701	            let mut window = self.window.lock().expect("window lock");
       702	            window.available -= written;
       703	        }

Resolution: Hold the one lock across the inner poll: take the guard, park and return `Pending` when `available == 0`, compute the grant, call the inner `poll_write`, and debit under the same guard (the inner `DuplexStream` never touches the window, so there is no re-entrancy). Alternatively `checked_sub(written).expect("a writer never debits more than it was granted")` makes a double-spend loud in release too, or the fixture comment at 650-658 states the single-poll premise. Acceptance: no path in `WindowedTx::poll_write` releases the window lock between reading `available` and debiting it, or the premise is stated at the fixture; `pooled_budget_below_the_bound_is_caught`, `never_binding_pooled_budget_conforms`, and `starved_pool_degrades_latency_not_liveness` keep their verdicts.

### conformance-40: The decode-slot padding obligation `window.rs` places on the backend's price is checked by nothing in the suite
- Where: src/conformance/backend.rs:258-263 (related: src/conformance/backend.rs:398-407, src/tree/mirror/streaming/window.rs:139-156, src/tree/mirror/streaming/window.rs:165-176, src/tree/mirror/streaming/window.rs:397-399, src/tree/mirror/streaming/backend.rs:90-118, src/tree/mirror/streaming/backend/local.rs:108-110, src/tree/typed/prefix.rs:17-21, src/tree/mirror/streaming/window.rs:132, src/link.rs:169)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (arithmetic from types read in the tree: `Prefix<Z>` is `repr(transparent)` over tinyvec 1.11.0's `ArrayVec<[u8; 32]>` (34 bytes at alignment 2), `typed::Node<Z>` is pointer-sized by the const assertion at local.rs:110, so `(Prefix<Z>, typed::Node<Z>)` is 48 bytes and `FAN_SLOT_BYTES` is 40; a 16-byte node at 16-byte alignment makes the pair 64 bytes and the real slot excess 48; `STREAM_COUNT = 17` and `FAN = 256`, so the unaccounted term is 17 × 257 × 8 = 34,952 bytes; `grep -rn 'size_of::<(Prefix' src` finds only window.rs:176, and `grep -n size_of src/conformance/backend.rs` finds nothing)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed and widened (`REFERENCE_SLOT_BYTES` at window.rs:139-156 carries the same clause and is equally unchecked; a `const` assertion would forbid what the window docs ask `node_bytes` to price, so the fold-into-the-comparison form matches the stated obligation); history: no rationale found (the clause was added by 4add7f8a as a provenance note closing the size_of-derivation ruling of the 2026-07-23 review; the suite's stated scope at backend.rs:1-47 lists shape, pointwise, bulk, and end-to-end and has no "what the suite cannot see" section)
- Owner-gated: no

The window charges every decode-fan slot at `node_bytes(0, bound) + FAN_SLOT_BYTES`, where `FAN_SLOT_BYTES` is the size of the `(Prefix<Z>, typed::Node<Z>)` pair less the node under the in-memory backend, and its doc states that a backend whose `Node<Z>` demands wider alignment "owes that padding to its own `node_bytes` price"; `REFERENCE_SLOT_BYTES` carries the same clause for the per-level slots. The suite's leaf checks compare `measure(node)` (the node value alone) to `node_bytes(0, bound)`; neither side includes the slot pair's padding, the census counts node values only, and `Backend::node_bytes`'s own docs never mention slot padding. A backend whose leaf handle is 16 bytes at 16-byte alignment (an inline `u128`, say) makes the real pair 64 bytes against the 48 the constant assumes: 8 unaccounted bytes per occupant, 17 × 257 × 8 = 34,952 bytes per session, and every check here passes. An obligation stated in one module and checked in none is a criterion with no demonstration (Principle 2), and it is a compile-time fact per backend, so the check costs one comparison.

Evidence:

       258	        let priced = <N::Backend as Backend>::node_bytes(0, bound_bytes(&node));
       259	        if measured > priced {
       260	            ledger::violation(format!(
       261	                "underpriced leaf: measured {measured} B, node_bytes priced {priced} B",
       262	            ));
       263	        }

    src/tree/mirror/streaming/window.rs:
       171	/// The derivation is exact for pointer-class node handles; a backend
       172	/// whose `Node<Z>` demands wider alignment pads the real slot beyond
       173	/// `node_bytes + FAN_SLOT_BYTES` and owes that padding to its own
       174	/// `node_bytes` price.
       175	const FAN_SLOT_BYTES: usize =
       176	    std::mem::size_of::<(Prefix<Z>, typed::Node<Z>)>() - std::mem::size_of::<typed::Node<Z>>();

    src/tree/mirror/streaming/window.rs:
       150	/// parameters, so the leaf instantiation prices every level. Exact for
       151	/// pointer-class node handles; a backend whose `Node` demands a wider
       152	/// layout pads the real slots beyond this constant and owes that padding
       153	/// to its own `node_bytes` price.

Resolution: Fold the slot excess into the leaf comparisons at 258-263 and 401-407: `let slot_excess = size_of::<(Prefix<Z>, B::Node<Z>)>() - size_of::<B::Node<Z>>() - FAN_SLOT_BYTES` (with `FAN_SLOT_BYTES` made `pub(crate)`), recording `underpriced leaf` when `measured + slot_excess > priced`; do the same for `REFERENCE_SLOT_BYTES` against `(u8, B::Node<Z>)` where the per-level references are priced (`Charged::children`, once conformance-25 prices them). Prefer this form over a `const` assertion, because the window docs ask `node_bytes` to price the padding rather than forbid it. Add a negative control in backend/tests.rs: a `#[repr(align(16))]` wrapper node whose `node_bytes` omits the padding, asserted to fail by name. If measurement is not added, the backend module doc needs the "What the suite cannot see" section it lacks, naming slot padding. Acceptance: a backend whose `Node<Z>` alignment exceeds the pointer's fails `check` by name unless its `node_bytes` covers the extra slot padding; `Local` and `Materializing` pass unchanged.
Construction: define a test backend whose node wraps `typed::Node<Z>` beside a `u128` (alignment 16, so the pair excess is 48 rather than 40) with `node_bytes(c, b) = size_of::<ThatNode>() + b`, and run `check` under it: every pointwise check passes and the census counts node values only, so the 8-byte-per-slot shortfall appears in no assertion.

### conformance-41: `Charged::erase` and `assume` carry the pre-conversion measurement across the re-tag, transcribing the `Backend` re-tag clause without checking it
- Where: src/conformance/backend.rs:285-297 (related: src/conformance/backend.rs:82-85, src/conformance/backend.rs:132-139, src/conformance/backend.rs:363-369, src/tree/mirror/streaming/backend.rs:54-60, src/tree/mirror/streaming/backend.rs:77-88, src/tree/mirror/streaming/materialized.rs:481-491, src/tree/mirror/streaming/erased.rs:254-266, src/conformance/backend/tests.rs:286-303)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; the in-memory conformance session reaches the re-tag through `greeting_fan`, `B::erase(node)` at materialized.rs:487, and `children_of`, `B::assume::<H>(node)` at erased.rs:259 and `B::erase(child)` at :264; `Charged::children` consumes the assumed parent through `into_inner` at backend.rs:369 without measuring it; `Measure` has no method over `Erased`)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: reframed (the carried measurement transcribes a stated trait clause, streaming/backend.rs:57-60 and :79-80, so the gap is that the clause has no demonstration; the check the clause calls for is equality of a post-`assume` measurement with the carried bytes, not a re-pricing at a capped fan; `erase` has no oracle without a `Measure` method over `Erased`); history: deliberate and holds for the shape (e5392c7c: "Charged settles and reopens its ledger entry, so the census peak is untouched"; the height-erasure record states that erasure re-tags rather than re-represents), with the premise unchecked
- Owner-gated: no

Both conversions re-wrap the converted handle with the *old* `bytes`. The comment argues the census peak is untouched, which is true only when the backend's erased and typed representations have identical residency, and that is exactly what the `Backend` trait's re-tag clause promises (a backend "can forget the tag ... and restore it ... without changing the value"; "`assume::<H>(erase::<H>(node)) == node` is the whole contract"). The suite transcribes the clause instead of checking it: a backend whose `assume` materializes (loads a child table into the typed handle) is charged at the erased size and compared to nothing, and the `Charged` doc's "keeping each node value's measured bytes on the ledger" does not hold across this path. The path is live in the conformance session: `greeting_fan` erases the root, `children_of` assumes it and erases every child, and `Charged::children` then consumes the assumed parent through `into_inner` without measuring it, so no knob can make the suite see a re-tag that changes residency. Principle 8 applied to the suite's own ledger: a carried number is a hypothesis and a measured one is a measurement, and the suite exists so a backend proves its account through `Measure`.

Evidence:

       285	    // Both conversions settle the wrapper's ledger entry and open an
       286	    // identical one around the re-tagged handle: the running total dips by
       287	    // one node's bytes between the two calls and never rises, so the
       288	    // census peak is untouched.
       289	    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
       290	        let bytes = node.bytes;
       291	        ChargedNode::wrap(B::erase(node.into_inner()), bytes)
       292	    }
       293	
       294	    fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H> {
       295	        let bytes = erased.bytes;
       296	        ChargedNode::wrap(B::assume(erased.into_inner()), bytes)
       297	    }

    src/tree/mirror/streaming/backend.rs:
        57	    /// The height parameter on [`Node`](Self::Node) is a compile-time tag
        58	    /// over runtime data that already knows its place in the tree, so a
        59	    /// backend can forget the tag ([`erase`](Self::erase)) and restore it
        60	    /// ([`assume`](Self::assume)) without changing the value. The

    src/tree/mirror/streaming/backend.rs:
        79	    /// `H` must be the height the node was erased at:
        80	    /// `assume::<H>(erase::<H>(node)) == node` is the whole contract, and

Resolution: In `assume`, measure the result (`B::measure::<H>(&node)`), record `ledger::violation("re-tagged node changed residency: erased {bytes} B, assumed {measured} B")` when it differs from the carried bytes, and wrap at the measured value. `erase` has no oracle unless `Measure` gains a method over `Erased`, so either add `fn measure_erased(node: &Self::Erased) -> usize` and check symmetrically, or state at 285-288 that erasure is trusted on the trait's clause. Add an `ASSUME_SLACK` knob to `Materializing::assume` that grows the row, with a `#[should_panic(expected = "re-tagged node changed residency")]` control. Restate the comment: the peak is untouched when the re-tag leaves residency unchanged, which is the trait's clause and which the measurement after `assume` checks. If measurement is not added, the trust belongs in the backend module doc's accounting premises. Acceptance: the new control fails by name; `local_backend_conforms` and `materializing_backend_conforms` pass; the comment at 285-288 cites the trait clause it relies on.
Construction: add `static ASSUME_SLACK: Knob = Knob::new(0);` and in `Materializing::assume` do `row.resize(row.len() + ASSUME_SLACK.get(), 0)`; set it to `BULK_OVERHOLD` and run `check(Materializing, MATERIALIZING_BUDGET)`: today no violation is recorded (nothing measures or prices the assumed node), the ledger under-reports the slack, and the run passes.

### conformance-42: The monotonicity sweep compares adjacent grid points only, so a dip strictly inside a gap passes while the window evaluates the cost at the greeting's arbitrary bound
- Where: src/conformance/backend.rs:602-614 (related: src/conformance/backend.rs:548-569, src/conformance/backend.rs:571-583, src/conformance/backend.rs:636-641, src/tree/mirror/streaming/window.rs:355-370, src/tree/mirror/streaming/window.rs:383-384, src/tree/mirror/streaming/window.rs:397-399, src/tree/mirror/streaming/backend.rs:110-115, src/conformance/backend/tests.rs:175-181, src/conformance/backend/tests.rs:305-317, src/conformance/backend/tests.rs:502-514)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read `sweep_bounds`: dense `0..=64`, then `power - 1, power, power + 1` for each power of two to `1 << 20`, so the gaps `65..=126` and `2^k + 2 ..= 2^(k+1) - 2` contain no grid point; the bound loop at 602-614 compares `bounds.windows(2)` only; `from_budget` evaluates `node_bytes` at `version_bound = 2 * (local + remote version bytes)` (window.rs:355-358), an arbitrary even integer, and its own `debug_assert` samples four fans and `version_bound / 2` against `version_bound` (360-370); `grep -rn 'proptest' src/tree/mirror/streaming/window` finds no property over `node_bytes`; `PRICED_DIP` is the sole monotonicity control and dips in fan at `DIP_FAN = 7`)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed, correcting the resolution (a proptest over a `2^20` range almost never samples a point dip at one bound; a step-shaped dip is what a property test catches reliably, and a denser grid catches the point dip at no cost since `node_bytes` is pure arithmetic); history: no rationale found (the grid's shape is deliberate, 8d959048, and its doc names round-size special-casing as the hazard the neighbors catch; nothing argues that a dip inside a gap is acceptable, and the sweep's own rustdoc names exactly that hazard)
- Owner-gated: no

`sweep_bounds` is dense to `BOUND_DENSE_CEILING` (64) and then each power of two with both neighbors, and the bound loop compares `node_bytes(fan, pair[0]) <= node_bytes(fan, pair[1])` for adjacent grid entries only, so a cost function that dips and recovers strictly inside a gap (`65..=126`, or `2^k + 2 ..= 2^(k+1) - 2`) passes; the fan dimension is exhaustive, so the gap is bound-only. `from_budget` evaluates `node_bytes(held, version_bound)` and `node_bytes(0, version_bound)` at the greeting's `version_bound`, twice the sum of the two sides' version bytes, an arbitrary even integer the grid need not contain, and its own `debug_assert` samples four fans and `version_bound / 2`. The realistic dip is a threshold: a backend that stores small bounds inline under a wider header and boxes large ones (`if bound <= 100 { 32 + bound } else { 16 + bound }`) prices 101 below 100 and recovers by 117, and the grid's 64 and 127 bracket it. The claim is a family (monotone over every pair `b1 <= b2`), the only committed control (`PRICED_DIP`) is a fan dip at a grid point, and the `Backend` doc and `check`'s `# Panics` scope the guarantee to "the swept grid", so the gap is stated without being admitted as one.

Evidence:

       602	    for pair in bounds.windows(2) {
       603	        for fan in 0..=FAN {
       604	            let here = B::node_bytes(fan, pair[0]);
       605	            let there = B::node_bytes(fan, pair[1]);
       606	            assert!(
       607	                here <= there,
       608	                "node_bytes must be monotone in version bound: bound {} prices {here} B, \
       609	                 bound {} prices {there} B, at fan {fan}",
       610	                pair[0],
       611	                pair[1],
       612	            );
       613	        }
       614	    }

    src/conformance/backend.rs:
       561	fn sweep_bounds() -> Vec<usize> {
       562	    let mut bounds: Vec<usize> = (0..=BOUND_DENSE_CEILING).collect();
       563	    let mut power = BOUND_DENSE_CEILING << 1;
       564	    while power <= BOUND_SWEEP_CEILING {
       565	        bounds.extend([power - 1, power, power + 1]);
       566	        power <<= 1;
       567	    }
       568	    bounds
       569	}

    src/tree/mirror/streaming/window.rs:
       355	        let version_bound = usize::try_from(
       356	            2 * (u128::from(local_version_bytes) + u128::from(remote_version_bytes)),
       357	        )
       358	        .unwrap_or(usize::MAX);

Resolution: Two changes, per the refutation's correction. Raise the dense ceiling (a few thousand bounds costs nothing, since `node_bytes` is pure arithmetic), which catches point dips. Add a `proptest!` in backend/tests.rs over `(fan in 0..=FAN, b1 in 0..=BOUND_SWEEP_CEILING, delta in 0..=BOUND_SWEEP_CEILING)` asserting `node_bytes(fan, b1) <= node_bytes(fan, b1.saturating_add(delta))` for `Local` and `Materializing`, with a step-shaped `PRICED_BOUND_STEP` knob (a header that shrinks above a threshold bound) and a control showing the grid sweep alone passes it while the property test fails it; commit the seed file the failing case writes. State in `node_bytes_monotone`'s doc that the grid is a sample and the property test covers the family. Acceptance: a step-shaped bound dip fails a committed test by name; a point dip at a non-grid bound below the raised dense ceiling fails the sweep; the sweep's doc states what it samples.
Construction: add `static PRICED_BOUND_DIP: Knob = Knob::new(0);` and in `Materializing::node_bytes` subtract it when `version_bound == 100`; set it to 1 and run `check(Materializing, MATERIALIZING_BUDGET)`: `node_bytes_monotone` compares (64, 127) and never evaluates at 100, so no assertion fires.

## Positives

- Every liveness check in the link suite has a committed negative control asserted to fail with the deterministic `Err(Quiescence::Stalled)` witness rather than a wall-clock timeout (mux coupling, coupled control duplex, lossy acceptor in both directions, capped connector, under-sized pooled window), and every legal-adversity fixture asserts its adversity fired (`reordered > 0`). The suite proves its own teeth instead of asserting them, and `never_binding_pooled_budget_conforms` pins the contract's pool bound from the passing side.
- Streams are classified in-band (`STALLED_TAG`/`LIVE_TAG`, the per-stream index byte), never by accept order, matching the contract's reordering freedom exactly; `reordering_acceptor_passes_independence` pins that the probes tolerate it.
- The stalled receivers in both independence shapes are returned from the receive future so they outlive the sender's `select` (link.rs:566-568, 673-675); without that hold a conforming pipe would report BrokenPipe and the probe would false-fail. The reasoning is written at the site.
- `probe_cancellation`'s ordering (connect all, signal, then write; collect the first delivery with a real accept before any poll-drop cycle) is argued step by step and makes the probe sound on real sockets as well as the in-memory link; `CANCEL_DROP_PATIENCE` expiring weakens rather than fails the probe, and the docs say so.
- `check_sessions` asserts snapshot equality rather than equal sizes and pins a liveness floor (`opened >= 1` per side), so the end-to-end check cannot degenerate into control-only traffic as `SESSION_PAYLOADS` or the tree's depth changes: the meters-need-liveness-floors doctrine applied to the suite itself, which is exactly what the backend suite's census ceiling lacks.
- The "What the suite cannot see" section (link.rs:28-55) states the black-box limits of each probe in terms a transport author can act on; this negative-space section is rare and well done.
- Constants carry the failure they prevent, not just their value: `STALLED_TAG` (in-band classification because acceptors may reorder), `CANCEL_DROP_PATIENCE` (why expiry weakens rather than fails), `SESSION_PAYLOADS` (which assertion keeps it from rotting), `CONTROL_DUPLEX_FILL` (what a pass proves only up to).
- Maintainer comments state the why at branches rather than the next line: why the pressure loop yields (link.rs:495-500), why the cancellation signal precedes the writes (861-866), why the demux lock is held across the routing send (link/tests.rs:230-233), why `ChargedNode` holds an `Option` (backend.rs:151-155), why the knobs are state rather than type parameters (backend/tests.rs:65-72, the rejected alternative named).
- `Measure`'s doc closes the cheapest-passing-artifact path in one sentence: "it must not consult the cost function it validates" (backend.rs:80-81).
- The `Charged` ledger is exactly balanced across wrap, clone, `into_inner`, drop, `erase`, and `assume`, with the erase/assume peak-neutrality argument stated where it holds (backend.rs:285-288). `check_assembled` prices at `run.leaves.min(FAN)` and justifies the cap by monotonicity in fan, while `node_bytes_monotone` sweeps that property before any session runs: the argument and its premise are both enforced.
- The `Knob` design (one backend type shared by accurate and skewed variants, knobs resting at their accurate value, a guard that holds `SERIAL` and restores on drop, a lock that clears poison from `should_panic` tests) makes plain `cargo test` sound, not merely nextest. `leaf_underpricing_fails_at_construction` is non-vacuous by construction: an empty ledger would panic with a message the `should_panic(expected)` does not match.
- `yield_once` keeps the whole suite runtime-agnostic, which is what lets the crate's own tests run it under `run_to_quiescence` and turn a hang into a deterministic `Stalled` verdict.
- No panic in `src/conformance/link.rs` is reachable by a transport that satisfies the contract as written: every `expect` on a connect, accept, write, flush, or read names the clause a violation breaches; the two internal `expect`s (`every slot was filled`, `the sending half signals after connecting`) are structurally unreachable; and `probe_concurrency` looks an in-band index up with `held.get_mut(..).expect(..)` (link.rs:777-779) rather than indexing, so an out-of-range byte fails by name.
- Every early `Done` drop the probes perform (`let (mut rx, _) = accept`, `drop((tx, done))`) is the contract's abort case, and `probe_completed_streams` (link.rs:383-420) is the one place completion is exercised, reading exactly the payload and handing the halves back without probing for end-of-stream: the two legal endings the Completion clause distinguishes are kept distinct in the suite.
- The fixtures are cancel-safe on the clause they test around: `ReversingAcceptor` keeps its batch in `self.held` (link/tests.rs:50-57), so a dropped `accept` future loses nothing, and `MuxAcceptor` releases the demux lock before awaiting the pump (link/tests.rs:437-457). A fixture that violated the cancellation clause while probing independence would confound two clauses. `Serialized::drop` (backend/tests.rs:45-49) drains the violation ledger, so a `should_panic` test that leaves violations behind cannot make its successor fail by name for a lie it never told.

## Open questions for Finch

- Does `MATERIALIZING_BUDGET` (4 MiB) widen the window above the floor at the suite's corpus scale? The flat pre-charge under `Materializing` pricing is `17 × 257 × (node_bytes(0, version_bound) + 40)` with `node_bytes(0, vb) = 32 + 64 + vb`; a rough estimate puts it near 850 KiB, leaving room, but only the floor proposed in conformance-28 says so mechanically. Recommendation: land the floor first and let it answer.
- conformance-7 reopens f5039abb's ruling on the eightfold signatures. My recommendation is the non-breaking loosening (drop `Send` and the unused control-half bounds from the focused checks) plus a one-line comment stating why the shape is otherwise kept; the sealed-trait collapse trades legibility for a one-implementor abstraction, which the doctrine also flags.
- conformance-18 reopens f5039abb's ruling on the `ReversingAcceptor`/`ReorderingAcceptor` pair. Recommendation: unify on `testing::reorder_accepts` if `reordered > 0` and the five `Stalled` verdicts hold under the patient wait; otherwise keep two and have transport.rs's comment state the actual behavioral reason rather than the feature-isolation one.
- conformance-1: ship a `conformance::bookmark` suite, or narrow the module doc's "each caller-implementable boundary" sentence? Recommendation: ship it; the `store` contract's commit-iff-`Ok` clause is exactly the kind of thing a deployment gets wrong over a file or KV store, and the crate names the consequence as unspecified corruption.
- conformance-19, conformance-22, conformance-32: three register rulings (em-dashes in `//` comments, "seam", "honest"/"lying") that are crate-wide or fixture-wide. Recommendation: rule once each; add a `tools/` lint for the dash convention since nothing enforces it today.
- Does `check_control` (sequential ping-pong) earn its place beside `check_control_duplex`? No link that passes the duplex probe and fails the sequential one was constructed; its remaining value is a legible first failure (looped-back halves) before the 32 KiB fill. Recommendation: keep it, and give it the negative control conformance-20 asks for, which is the one thing that would justify it.
- conformance-12: qualify the concurrency docs, or strengthen the probe so elders always backpressure? Recommendation: qualify the docs now (no behavior change); consider the stronger probe as a separate owner decision, since it changes what a pass certifies for every downstream transport.
- conformance-37: does the protocol's deadlock-freedom argument assume `Connector::connect` completes independently of the peer polling `accept`? If yes, the contract in src/link.rs should say so and the cancellation probe's precondition follows from it; if no (the protocol's acceptor loop at streams.rs:694-706 is polled continuously beside the session, and the memory link's backlog comment says the bound never binds), the probe requires more of a transport than the protocol does. Recommendation: state the precondition in the suite's requirements and in `check_accept_cancellation`'s rustdoc, and promote it to a clause only if transports are to be held to what the in-tree instantiations already provide.
- conformance-38: should every link negative control run at both tested capacities (8 KiB and one byte), the way the passing side does in `memory_link_conforms` and `one_byte_windows_conform`? The lossy control's verdict differs by capacity; whether the mux, coupled, capped, and pooled fixtures do is unknown, and a loop over `[MEMORY_STREAM_CAPACITY, 1]` in each control would answer it mechanically. Recommendation: yes; the two capacities are already the suite's own convention for the passing side.
- conformance-40: is the census meant to model slot residency, or node values only? `window.rs` charges `node_bytes + FAN_SLOT_BYTES` per decode slot and delegates alignment padding to the backend's price; the suite measures node values only. Recommendation: fold the slot excess into the leaf comparison, since the obligation is stated on `node_bytes` and the suite is where `node_bytes` is held to account.
- conformance-41: may a backend's `Erased` representation differ in residency from its typed one? The `Backend` trait's clause (streaming/backend.rs:57-60) says the re-tag does not change the value. Recommendation: hold the suite to the clause by measuring after `assume`; `erase` stays trusted until `Measure` gains a method over `Erased`, and the trust is stated at the site.
- Should `conformance::backend`'s module doc gain a "What the suite cannot see" section like `conformance::link`'s (link.rs:28-55)? conformance-40, conformance-41, and conformance-42 each name a limit the backend suite neither checks nor admits; whichever of them is not closed by measurement lands a one-line admission there. Recommendation: add the section in the same commit that closes or admits each.

## Dropped

- Candidate [4] "Testdoc claims drops settle the ledger, but the test observes only the peak": construction was wrong (a no-op `Drop` does fail the test, at the second pair's assertion); the accurate version is conformance-35 (from [18]).
- Candidate [13] "Census budget is asserted to bind in the testdocs but check has no liveness floor": folded into conformance-28, which carries the constructed instance and the dated expiry.
- Candidates [35], [47] dead `Charged` binding: duplicates of conformance-36 (from [3]).
- Candidates [36], [43] manual `Clone` impls: duplicates of conformance-15 (from [9]).
- Candidate [46] vestigial braces: duplicate of conformance-21 (from [8]).
- Candidate [41] eight-bound signatures: merged into conformance-7 with [10]; its verified non-breaking alternative is the recommended resolution.
- Candidate [26] visibility stated three times: merged into conformance-3 with [11] and [12]; its point that the public doc names an unreachable suite is kept.
- Candidate [12] `pub(crate)` wider than use: merged into conformance-3 as "state the placeholder intent", since history records the intent (goes `pub` with the storage boundary) as deliberate and holding.
- Candidate [32] bulk seams' structural clauses: merged into conformance-24 with [29] as one pattern (clauses the `Backend` docs claim convicted but the suite does not check), with the refutation's reframe applied (leaves' doc claims only encoder enforcement).
- Candidate [42] `CONTROL_DUPLEX_FILL` relation held by prose: merged into conformance-5 as site (a), where it becomes the const-assert resolution.
- Candidate [38] `node_bytes_monotone` bounds: subsumed by conformance-27 (from [44]), which covers all five `Measure + Clone` sites.
- Refutation new nit 1 (`check(Local, 64 * 1024)` inline magic number): folded into conformance-28's resolution (name `LOCAL_BUDGET` and size it above the flat term).
- Refutation new item 3 (`check_sessions` has no negative control): folded into conformance-20.
- Refutation new item 4 (em-dash convention breached crate-wide): folded into conformance-19 as its scope and owner-gating.
- Correctness lens open question on `Charged::assemble` discharging leaves on entry and charging the node on exit (a backend's run buffer invisible between them): not a finding; the module's stated accounting premise (the fans' flat pre-charge covers assembly transients) answers it, and the seam is not exercised in-session today (conformance-31). Recorded here so it is not lost.
- Correctness lens open question on `check_control`'s "equal-length payloads" sentence: nothing in the code depends on the equality (each side reads its own expected length); below the bar as a standalone finding, noted under open questions on whether `check_control` earns its place.
- Structure lens open question on `Charged` deriving `Copy` and `Default` unused: harmless mirrors of `Local`'s derives; below the bar.
- Late correctness lens: the refutation pass's new item (`let charged = Charged::<Local>::new(Local); let _ = &charged;` at backend/tests.rs:572-573 constructs a value used nowhere): duplicate of conformance-36.
- Late correctness lens: the refutation pass's new item (the two never-completing `select` arms at link.rs:522 and 632-634 are written in different idioms): already covered by conformance-9 (the drift between the two probes) and conformance-10 (the `contract:` prefix on the `unreachable!`).
- Late correctness lens: the refutation pass's new item (`REFERENCE_SLOT_BYTES` at window.rs:139-156 carries the same padding clause as `FAN_SLOT_BYTES`): folded into conformance-40's resolution, which covers both slot constants.
- Late correctness lens: no candidate was refuted, by reading or by construction; the witness pass had closed before the lens reran, and none of the six survivors was built and run, so none carries `demonstrated` provenance. The constructions each entry states remain to be run.
