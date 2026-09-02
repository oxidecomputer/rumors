# Witness constructions

## link-28

Outcome: demonstrated

Test code:

```rust
/// Witness (link-28): a recovered idle connection a pooling dial has
/// already admitted is evicted by a later stalled arrival under the
/// single pending-header bound, and the eviction surfaces as a failed
/// stream open on a link whose peer is healthy.
///
/// Asserts the healthy behavior (the next open succeeds and the stream
/// delivers): a failure here demonstrates the finding.
#[test]
fn witness_evicted_pooled_connection_fails_the_next_open() {
    use crate::testing::run_to_quiescence;

    let net = MemoryNet::new();
    let dial = PoolingDial::new(&net);
    let a_name = MemoryName::new("a");
    let config = Config {
        pending_headers: 1,
        ..Config::default()
    };
    let (a, mut a_incoming, a_router) =
        Endpoint::new(net.listen(&a_name), a_name, dial.clone(), config)
            .expect("a valid construction");
    let b_name = MemoryName::new("b");
    let (b, _b_incoming, b_router) =
        Endpoint::new(net.listen(&b_name), b_name, dial.clone(), Config::default())
            .expect("a valid construction");
    let outcome = run_to_quiescence(drive(routers(a_router, b_router), async {
        let (linked, arrival) =
            futures::join!(b.link(MemoryName::new("a")), a_incoming.accept());
        let at_b = linked.expect("establishment succeeds");
        let (_info, mut at_a) = arrival.expect("the router delivers the link");

        // One completed stream: b's write half is recycled toward the
        // pool, a's router takes the read half back and writes READY.
        transfer_completed(&at_b, &mut at_a, b"paid by a dial").await;
        settle().await;
        let established = dial.fresh_dials();

        // A newcomer stalls mid-header at a's router: under the count
        // bound of one it displaces the oldest pending read, which is
        // the recovered connection awaiting its next header.
        let mut stalled = net.dial().dial(&MemoryName::new("a")).await.expect("dial");
        stalled
            .write_all(b"ROU")
            .await
            .expect("a partial magic writes");
        settle().await;

        // The next open draws the pooled connection.
        let (mut tx, done) = match at_b.connector.connect().await {
            Ok(opened) => opened,
            Err(error) => panic!(
                "next stream open on a healthy link failed: kind {:?}, {error}",
                error.kind()
            ),
        };
        assert_eq!(
            dial.fresh_dials(),
            established,
            "the open drew the pooled connection"
        );
        let open = async {
            tx.write_all(b"after the stall").await.expect("payload writes");
            tx.flush().await.expect("payload flushes");
            done.complete(tx);
        };
        let read = async {
            let (mut rx, done) = at_a.acceptor.accept().await.expect("stream arrives");
            let mut received = vec![0u8; b"after the stall".len()];
            rx.read_exact(&mut received)
                .await
                .expect("the completed stream delivers its bytes");
            done.complete(rx);
            received
        };
        let ((), received) = futures::join!(open, read);
        assert_eq!(received, b"after the stall");
        drop((stalled, a, b));
    }));
    outcome.expect("the scenario runs to completion without stalling");
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-run   # then:
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(witness_)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
        FAIL [   0.014s] (2/6) rumors link::routed::tests::witness_evicted_pooled_connection_fails_the_next_open
  stderr ───

    thread 'link::routed::tests::witness_evicted_pooled_connection_fails_the_next_open' (239157197) panicked at src/link/routed/tests.rs:795:27:
    next stream open on a healthy link failed: kind BrokenPipe, broken pipe
```

Explanation:

Appended to src/link/routed/tests.rs (the file's PoolingDial fixture, transfer_completed, settle, drive, routers reused; the file's establish() helper is monomorphic over MemoryDial, so establishment is inlined as the existing completed_streams_reuse_their_connection test does). Ran once (after one compile fix; no behavioral iteration). VERIFIED BY RUNNING: with a's Config { pending_headers: 1 }, after one completed stream and a settle, a raw dial writing b"ROU" (a stall mid-header) followed by a settle made the very next at_b.connector.connect() return Err(BrokenPipe) on a link whose peer endpoint, router, and listener were all live; the panic site (tests.rs:795:27) is the Err arm of the connect match, before the fresh_dials assertion. A BrokenPipe on the STREAM-header write is only producible on the pooled tokio duplex whose router-side end was dropped (a fresh duplex to the live listener cannot fail that way), which is the eviction of the recovered connection by the newer stalled arrival under the oldest-first count bound at router.rs:159-163, admitted into the pool by the READY byte written at router.rs:213-215 before the header read. ASSESSED BY READING, not run: the TCP hang variant (PoolingTcpDial + gossip session + timeout). Not constructed within the cap: it needs a new integration binary plus timing-dependent socket behavior (whether the small stream lands before the RST), and endpoint.rs:52-55 already concedes 'fails, or hangs to the caller's session timeout'; the memory-net run demonstrates the mechanism the finding rests on. Polarity: the test asserts the healthy behavior, so its failure is the demonstration.

## conformance-31

Outcome: demonstrated

Test code:

```rust
// In src/conformance/backend/tests.rs. (1) A knob, added directly after
// `static ASSEMBLE_SKIPS: Knob = Knob::new(0);`:

/// Witness knob (conformance-31): assembled nodes bulk assembly folds
/// and then silently drops; honest at zero.
static ASSEMBLED_DROPS: Knob = Knob::new(0);

// (2) In `impl Backend for Materializing`, fn assemble, the assembled node
// stream (not the leaves) is skipped: the line
//         H::assemble(self, supplied).map(|item| {
// becomes
        H::assemble(self, supplied).skip(ASSEMBLED_DROPS.get()).map(|item| {

// (3) The test, appended to the file:

/// Witness (conformance-31): bulk assembly that folds its run but
/// swallows the assembled node must be named by the assembly seam's
/// `unassembled run` check. The finding predicts the corpus builder's
/// own `expect` trips first, so the seam's name never surfaces.
#[test]
#[should_panic(expected = "unassembled run")]
fn witness_swallowed_assembled_node_is_named_by_the_seam() {
    let _dishonest = ASSEMBLED_DROPS.set(1);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(witness_)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
        FAIL [   0.087s] (5/6) rumors conformance::backend::tests::witness_swallowed_assembled_node_is_named_by_the_seam
  stdout ───
    test conformance::backend::tests::witness_swallowed_assembled_node_is_named_by_the_seam - should panic ... FAILED
    ---- conformance::backend::tests::witness_swallowed_assembled_node_is_named_by_the_seam stdout ----
    note: panic did not contain expected string
          panic message: "a non-empty corpus assembles a root"
     expected substring: "unassembled run"
  stderr ───
    thread 'conformance::backend::tests::witness_swallowed_assembled_node_is_named_by_the_seam' (239157195) panicked at src/conformance/backend.rs:778:10:
    a non-empty corpus assembles a root
```

Explanation:

Ran once, no iteration. VERIFIED BY RUNNING: a Materializing backend whose assemble override skips one assembled node (the knob shape the file's existing negative controls use) makes the whole conformance check die at src/conformance/backend.rs:778:10 with 'a non-empty corpus assembles a root', the corpus builder's own expect, not with the seam's 'unassembled run' violation; the should_panic expectation on the seam's name fails with 'panic did not contain expected string'. ASSESSED BY READING: Charged::assemble does record the leftover run and calls ledger::violation("unassembled run: ...") before its stream ends (backend.rs, the `if !failed` block), but corpus() consumes the stream's None immediately at line 778 and panics before check() ever reaches ledger::take_violations, so the violation class has no route to a name in the run's failure. The same structure means 'unsupplied assembly' cannot fire from this run either (one Root-height run, one prefix), consistent with the finding; that second class was not separately constructed.

## remote-codec-14

Outcome: demonstrated

Test code:

```rust
/// Witness (remote-codec-14): the over-budget lone-record body read
/// consumes no byte beyond the declared run.
///
/// Stream 9, `Supply(End)`, declaring a four-byte run that is exactly
/// one record of one content byte, followed by a second frame
/// (`End(Stream)` on stream 9). Under a zero budget the lone-record
/// path is taken. Asserts the exactness contract: the async reader
/// accepts the frame as the sync oracle does and rests at the second
/// frame's first byte.
#[test]
fn witness_over_budget_lone_record_read_stays_within_its_frame() {
    let stream = stream(9);
    let zero = RunBudget::from_bytes(0);
    // A lone record of one content byte: tag 63, bstr(1), 0x00.
    let lone = raw_record(&[0x00]);
    assert_eq!(lone, [0xd8, 0x3f, 0x41, 0x00]);
    let mut encoded = supply(stream, Flow::End, &lone);
    assert_eq!(&encoded[..3], [0x83, 0x09, 0x07]);
    assert_eq!(&encoded[3..6], [0xd8, 0x3f, 0x44]);
    let trailing = bare_frame(stream, Signal::End(End::Stream));
    assert_eq!(trailing, [0x82, 0x09, 0x09]);
    encoded.extend_from_slice(&trailing);

    for speaker in SPEAKERS {
        // The sync oracle: one frame, the trailing frame untouched.
        let mut rest = encoded.as_slice();
        let from_sync = decode(speaker, zero, &mut rest);
        assert_eq!(
            rest,
            trailing.as_slice(),
            "the sync oracle rests at the next frame; it decoded {from_sync:?}"
        );
        let from_sync = from_sync.expect("the sync oracle accepts the lone record");

        // The async reader over the same bytes.
        let mut reader = FrameRead::new(speaker, zero, encoded.as_slice());
        let from_async = pollster::block_on(reader.frame());
        let remaining = reader.into_inner();
        assert_eq!(
            remaining,
            trailing.as_slice(),
            "the async reader consumed bytes of the next frame; it decoded {from_async:?}",
        );
        assert_eq!(
            from_async.expect("the async reader accepts the lone record"),
            Some(from_sync),
        );
    }
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(witness_)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
        FAIL [   0.012s] (1/6) rumors tree::mirror::streaming::remote::codec::decode::tests::witness_over_budget_lone_record_read_stays_within_its_frame
  stderr ───

    thread 'tree::mirror::streaming::remote::codec::decode::tests::witness_over_budget_lone_record_read_stays_within_its_frame' (239157196) panicked at src/tree/mirror/streaming/remote/codec/decode/tests.rs:1148:9:
    assertion `left == right` failed: the async reader consumed bytes of the next frame; it decoded Err(DecodeError { origin: Stream { speaker: Initiator, stream: Stream(9) }, kind: InvalidRun(NotARecord { remaining: 3, detail: "record does not open with the embedded-sequence tag" }) })
      left: []
     right: [130, 9, 9]
```

Explanation:

Appended to src/tree/mirror/streaming/remote/codec/decode/tests.rs, reusing its raw_record/supply/bare_frame helpers; the byte-level asserts confirm the constructed wire is exactly the finding's bytes ([0x83,0x09,0x07] [0xd8,0x3f,0x44] [0xd8,0x3f,0x41,0x00] then [0x82,0x09,0x09]). Ran once, no iteration. VERIFIED BY RUNNING (Speaker::Initiator; the loop failed on its first speaker, and the code path is speaker-independent): the sync oracle's two assertions passed first (it consumed exactly the 4-byte body and rested at [0x82,0x09,0x09], returning Ok), then the async FrameRead::frame returned Err(InvalidRun(NotARecord { remaining: 3, .. })) and reader.into_inner() was empty: all three bytes of the following End(Stream) frame were consumed. This is the predicted breach of read_payload's 'Never consumes a byte beyond len' (framing.rs:74, inherited by resume_payload) and FrameRead's 'no byte of the next frame is touched' (async_io.rs:54-55). ASSESSED BY READING: the cause is record_prefix's Vec::new() growing to capacity 8 across two write_head calls and resume_payload's `len() == capacity()` reserve guard letting read_buf fill spare capacity past len (framing.rs:100-106); consequence today is bounded to misclassification because a conforming record is at least 8 bytes, as the finding states.

## streaming-tests-11

Outcome: demonstrated

Test code:

```rust
/// Witness (streaming-tests-11): the stall probes' boolean collapses a
/// session that dies with a protocol violation into "did not stall".
///
/// The fan = cap + 2 shape whose completion
/// `parent_delay_single_parent_boundary` asserts, with the server
/// wrapped to commit `UnexpectedQuery` in its first reply: the probe's
/// predicate answers `false` (so the existing "must complete" assertion
/// passes) while every run's actual outcome is a violation, never a
/// completion. This test passes exactly when the finding holds.
#[test]
fn witness_stall_probe_reads_a_violation_as_completion() {
    use crate::tree::mirror::streaming::{Fault, Faulting, materialized::Violation};

    let cells: Vec<Vec<u8>> = (0..3u8).map(|radix| vec![0, radix]).collect();
    let pair = divergent_cells_pair(&cells, 1, LeafOrder::Outside);
    let mut outcomes: Vec<String> = Vec::new();
    let stalls = probe_schedules()
        .into_iter()
        .any(|(channel_schedule, backend_schedule)| {
            let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) =
                (pair.0.clone().into(), pair.1.clone().into());
            let client = Handshaking::start(Local, a).window(WindowConfig::FLOOR);
            let server = Handshaking::start(Local, b).window(WindowConfig::FLOOR);
            let server =
                Faulting::new(server, 0, Some(Fault::Reply(Violation::UnexpectedQuery)));
            with_kind_capacity(QueueKind::AssemblyLevelReturns, 1, || {
                with_schedule(channel_schedule, || {
                    with_local_schedule(backend_schedule, || {
                        let outcome = run_to_quiescence(drive_streaming(client, server));
                        let stalled = matches!(outcome, Err(Quiescence::Stalled));
                        outcomes.push(match outcome {
                            Ok(Ok(_)) => "completed".to_string(),
                            Ok(Err(error)) => format!("violation: {error:?}"),
                            Err(quiescence) => format!("{quiescence:?}"),
                        });
                        stalled
                    })
                })
            })
        });
    eprintln!("witness: probe predicate stalls = {stalls}; run outcomes = {outcomes:?}");
    assert!(
        !stalls,
        "the probe predicate reads the faulting session as 'did not stall' \
         (the existing 'must complete' assertion form): {outcomes:?}"
    );
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome.starts_with("violation: ")),
        "every run actually died with a violation, none completed: {outcomes:?}"
    );
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(/^tree::mirror::streaming::tests::capacity::witness_stall_probe_reads_a_violation_as_completion$/)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
        PASS [   0.017s] (1/2) rumors tree::mirror::streaming::tests::capacity::witness_stall_probe_reads_a_violation_as_completion
  stderr ───
    witness: probe predicate stalls = false; run outcomes = ["violation: Client(Violation(UnexpectedQuery))", "violation: Client(Violation(UnexpectedQuery))", "violation: Client(Violation(UnexpectedQuery))", "violation: Client(Violation(UnexpectedQuery))", "violation: Client(Violation(UnexpectedQuery))"]
```

Explanation:

Appended to src/tree/mirror/streaming/tests/capacity.rs; it is shape_stalls/stalls_under_any_schedule from that file with the server wrapped in Faulting::new(server, 0, Some(Fault::Reply(Violation::UnexpectedQuery))) and the run_to_quiescence outcome recorded beside the collapsed boolean. Reverse polarity, as a missing-check finding: the test PASSES exactly when the finding holds. Ran twice (the first run passed silently; the second added only the eprintln so the passing run leaves visible evidence; no logic changed). VERIFIED BY RUNNING: under all five probe schedules at capacity 1 on the internal_fan(3) shape, `matches!(run_to_quiescence(..), Err(Quiescence::Stalled))` was false, the exact form the 'must complete' assertions at capacity.rs:265-269, 274-277, 308-312, 323-329 and 191-194 consume, while every run's real outcome was Ok(Err(Client(Violation(UnexpectedQuery)))): the session died with a protocol violation and the probe read it as completion. The PollBudget collapse and the absence of any join_oracle comparison in the completion arm were assessed by reading capacity.rs:26-36 and 198-218, not separately constructed.

## tests-disruption-handshake-7

Outcome: demonstrated

Test code:

```rust
// VARIANT A, the finding's literal construction (tests/disruption.rs, the
// closure passed to sessions.spawn in run_proc_plan):
                sessions.spawn(async move {
                    // WITNESS PROBE (tests-disruption-handshake-7), literal
                    // construction: a panic as the serving task's first statement.
                    panic!("witness probe: serving-task panic");
                    #[allow(unreachable_code)]
                    // A child that dies before the listener-port swap is the
                    // same honest severance as one that dies mid-session.
                    let mut link = match tcp::link(socket).await {

// VARIANT B, conditional construction (same file; `ordinal` captures the
// accept-order index so the panic lands on the serving path of the fourth
// connection, after the port swap):
                let handle = casts[next % casts.len()].clone();
                let ordinal = next;
                next += 1;
                let serve_errors = Arc::clone(&serve_errors);
                let dishonest = Arc::clone(&dishonest);
                sessions.spawn(async move {
                    // A child that dies before the listener-port swap is the
                    // same honest severance as one that dies mid-session.
                    let mut link = match tcp::link(socket).await {
                        Ok(link) => link,
                        Err(_) => {
                            serve_errors.fetch_add(1, Ordering::Relaxed);
                            return;
                        }
                    };
                    // WITNESS PROBE (tests-disruption-handshake-7), conditional
                    // construction: a panic on the serving path after the
                    // port swap, on the fourth connection: the child's faulted
                    // retire (write cut at byte 0), whose failure the child
                    // attributes to its own injected cut.
                    if ordinal == 3 {
                        panic!("witness probe: serving-task panic on connection {ordinal}");
                    }
                    if let Err(e) = handle.gossip(&mut link).await {
```

Command:

```text
# Variant A:
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(witness_)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
# Variant B:
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(/^tree::mirror::streaming::tests::capacity::witness_stall_probe_reads_a_violation_as_completion$/)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
# Variant A (literal):
        FAIL [   0.034s] (4/6) rumors::disruption reconstructed_child_retire_cut_at_first_byte
  stderr ───

    thread 'tokio-rt-worker' (239157183) panicked at tests/disruption.rs:747:21:
    witness probe: serving-task panic
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'reconstructed_child_retire_cut_at_first_byte' (239157178) panicked at tests/disruption.rs:815:22:
    child 0 exited abnormally (Some(101)): an invariant violation or panic in the child process

# Variant B (conditional):
        PASS [   0.035s] (2/2) rumors::disruption reconstructed_child_retire_cut_at_first_byte
  stderr ───

    thread 'tokio-rt-worker' (239179846) panicked at tests/disruption.rs:761:25:
    witness probe: serving-task panic on connection 3
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Explanation:

Two runs, one per variant. VERIFIED BY RUNNING, variant A (the finding's literal construction: panic as the serving task's first statement, before tcp::link): the test FAILS, so the construction's predicted 'observe PASS' is refuted as written; but the failure is 'child 0 exited abnormally (Some(101))' from the parent's exit-code check at disruption.rs:815, i.e. the child process noticed its bootstrap connection die at its own `tcp::link(socket).await.expect("swap listener ports")` and panicked; the parent itself observed nothing of the serving-task panic. VERIFIED BY RUNNING, variant B (panic on the serving path after the port swap, on accept-order connection 3, which for this plan (boot clean, one clean session, one clean final gossip, then retire with write cut at byte 0 and a clean retry) is the child's faulted retire, whose failure the child attributes to its own injected cut): the test PASSES with 'witness probe: serving-task panic on connection 3' present only in captured stderr. That is the finding's mechanism: the JoinSet is never join_next'ed and the accept task is aborted with its result discarded (disruption.rs:733-765), so a panic in a serving session is invisible to the test. INFERRED, not observed: in the unmodified test connection 3's serving session ends in an honest I/O error and increments serve_errors (so possible_losses = 1); with the panic in its place serve_errors stays 0 and the stricter assert_party_invariants(.., 0) still passed because the child retired cleanly on its retry, so the panic also silently altered the loss accounting. Note for the durable record: an unconditional first-statement panic is not a faithful probe because the child's own expectations on clean connections surface it via exit codes; a panic mid-serving on a connection the child expects to fail is.

## conformance-24

Outcome: refuted

Verdict: CONFIRMED

Test code:

```rust
// Added near the other honesty knobs in src/conformance/backend/tests.rs:
/// WITNESS conformance-24: when nonzero, the next interior `parent` call
/// (fan > 0) returns `Ok(None)` once, then self-clears — a one-shot
/// injection of the parent Some/None fault the suite claims to convict.
static PARENT_NONE_ONCE: AtomicUsize = AtomicUsize::new(0);

// Inserted at the top of `impl Backend for Materializing`'s `parent`,
// immediately after `let fan = children.iter().filter(...).count();`:
        // WITNESS conformance-24: null exactly one interior parent.
        if fan > 0 && PARENT_NONE_ONCE.swap(0, Ordering::Relaxed) != 0 {
            return Ok(None);
        }

// Appended test:
/// WITNESS conformance-24: a `parent` override that returns `None` for
/// an interior fan (fan > 0) records NO ledger violation naming the
/// parent Some/None clause — `Charged::parent` maps only the `Some`
/// arm, so a `None` return is unaccounted. If the suite convicts at
/// all, it is on an unrelated downstream assertion (convergence /
/// reconciliation), never the parent clause. This test drives that
/// injection: whatever it does (pass, or panic with a non-parent-clause
/// message) is the demonstration; a panic naming "underpriced node" or
/// "mis-propagated" would refute the finding.
#[test]
fn witness_conformance_24_parent_none_uncharged() {
    let _serial = serialized();
    PARENT_NONE_ONCE.store(1, Ordering::Relaxed);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
    PARENT_NONE_ONCE.store(0, Ordering::Relaxed);
}
```

Command:

```text
cargo nextest run -p rumors --all-features -E 'test(witness_)'
```

Output:

```text
        FAIL [   0.609s] (4/5) rumors conformance::backend::tests::witness_conformance_24_parent_none_uncharged

    thread 'conformance::backend::tests::witness_conformance_24_parent_none_uncharged' panicked at src/conformance/backend.rs:657:5:
    the backend's node_bytes contract failed pointwise:
    bulk-assembled len: node answers 1535, its run supplied 1536 leaves
```

Explanation:

REFUTED for the construction's primary injection (parent returns None at one interior fan), with nuance. The finding predicted this fault would produce 'the panic is "the conformance session must converge both corpora to one root" (or a decoder-side panic), not a ledger violation naming the clause' and 'at best trips run's converged assertion.' Reality: the violating backend was DETERMINISTICALLY CONVICTED, but by a different committed check than the parent seam — the assemble-seam len invariant (check_assembled, backend.rs:511), panicking with 'bulk-assembled len: node answers 1535, its run supplied 1536 leaves'. Nulling one interior parent during the interactive session dropped one leaf's subtree, which surfaced one assembly step later as an off-by-one leaf count. So the suite is NOT blind to this fault; a violating backend does not slip through to a mere convergence stall. TRUE kernel that survives: the parent seam itself is silent — I verified by reading that Charged::parent (backend.rs:325-326) is `Ok(parent.map(|node| {..checks..}))`, so the None arm records nothing, and indeed the panic names no parent clause ('underpriced node'/'mis-propagated' did not fire). But the practical worry (an unconvicted violating backend) does not hold: the assemble-len check is a local per-assembly invariant (node.len() vs supplied run count), independent of the two-sided convergence oracle, so it would fire even on a symmetric fault as long as the drop flows through assemble. UNTESTED within the cap and left open: the Some-at-fan==0 sub-claim, and the two ordering sub-claims (swap the first two leaves of Materializing::leaves; wrong run order in assemble) — those are separate injections the finding bundles and this run did not exercise.

## mirror-common-8

Outcome: demonstrated

Verdict: CONFIRMED

Test code:

```rust
// Appended to src/tree/mirror/framing/tests.rs (uses the existing
// `pattern` helper and the `pub(crate)` `resume_payload`):
/// WITNESS mirror-common-8 (first clause): a caller-supplied prefix Vec
/// whose spare capacity exceeds `len` makes `resume_payload` read past
/// `len`, consuming the following frame's bytes. Asserts the contract
/// (exact `len` bytes, cursor left at the next frame); it FAILS if the
/// over-read finding is right.
#[test]
fn witness_mirror_common_8_over_reads_spare_capacity() {
    let len = 9;
    let mut transcript = pattern(len, 5);
    transcript.extend_from_slice(b"next-frame-bytes");
    let mut prefix = Vec::with_capacity(64);
    prefix.extend_from_slice(&transcript[..3]);
    let mut cursor: &[u8] = &transcript[3..];
    let decoded = pollster::block_on(resume_payload(&mut cursor, prefix, len)).unwrap();
    assert_eq!(decoded, &transcript[..len], "resume_payload read past len");
    assert_eq!(cursor, b"next-frame-bytes", "resume_payload consumed the next frame");
}

/// WITNESS mirror-common-8 (second clause): a prefix already longer than
/// `len` skips the loop and returns Ok holding the whole over-long
/// buffer. Asserts an exact read never returns more than `len` bytes; it
/// FAILS if the over-long finding is right.
#[test]
fn witness_mirror_common_8_over_long_prefix() {
    let mut cursor: &[u8] = b"unused";
    let decoded = pollster::block_on(resume_payload(&mut cursor, vec![0u8; 12], 9));
    let returned = decoded.as_ref().map(Vec::len).unwrap_or(0);
    assert!(
        returned <= 9,
        "resume_payload returned an over-long payload of {returned} bytes",
    );
}
```

Command:

```text
cargo nextest run -p rumors --all-features -E 'test(witness_)'
```

Output:

```text
// First clause (over-read past len):
    thread '...witness_mirror_common_8_over_reads_spare_capacity' panicked at src/tree/mirror/framing/tests.rs:154:5:
    assertion `left == right` failed: resume_payload read past len
      left: [5, 36, 67, 98, 129, 160, 191, 222, 253, 110, 101, 120, 116, 45, 102, 114, 97, 109, 101, 45, 98, 121, 116, 101, 115]
     right: [5, 36, 67, 98, 129, 160, 191, 222, 253]

// Second clause (over-long prefix returns Ok):
    thread '...witness_mirror_common_8_over_long_prefix' panicked at src/tree/mirror/framing/tests.rs:167:5:
    resume_payload returned an over-long payload of 12 bytes
```

Explanation:

DEMONSTRATED, both clauses. First clause: with a prefix Vec of capacity 64 holding 3 bytes and len=9, the loop's reserve guard (`payload.len() == payload.capacity()`, i.e. 3 == 64) is false, so no clamp-to-len reserve runs; read_buf then offers the reader all 61 spare bytes and the slice reader copies all 22 remaining transcript bytes. `decoded` is 25 bytes (the 9-byte payload `[5,36,67,98,129,160,191,222,253]` PLUS the entire next frame `next-frame-bytes` = `[110,101,120,116,45,102,114,97,109,101,45,98,121,116,101,115]`), and the cursor is left empty — the exactness contract ('Never consumes a byte beyond len', inherited from read_payload) is violated for this admitted input. Second clause: a prefix already longer than len (12 > 9) skips the `while payload.len() < len` loop entirely and returns Ok holding all 12 bytes — an over-long payload an exact read must never produce. Both are as the finding predicts. The finding's own note that the single production caller (record_prefix -> resume_payload) is safe by an unstated capacity margin (a length-3 Vec at std's minimum capacity 8, versus a smallest lone record of 9 bytes) is a code-reading observation I did not exercise; the demonstrated defect is in resume_payload's contract for the general admitted-input class, matching the finding's severity framing.

## remote-capture-atlas-13

Outcome: demonstrated

Verdict: CONFIRMED

Test code:

```rust
// Appended to src/tree/mirror/streaming/remote/codec/capture/tests.rs
// (uses the existing `run_lines` helper, `LeafRun`, `Version`, `Message`):
/// WITNESS remote-capture-atlas-13: two supply records whose only
/// difference is the content of an array-shaped map key render to
/// identical line sets, because the renderer collapses any container
/// key to a bare `…`. Asserts the injectivity contract (distinct wire
/// bytes render distinguishably); it FAILS if the collapse finding is
/// right.
#[test]
fn witness_capture_atlas_13_container_key_collapses() {
    use std::collections::BTreeMap;
    let mk = |key: (u8, u8)| {
        let mut run = LeafRun::new();
        run.push(&Version::new(), &Message::new(BTreeMap::from([(key, 7u8)])))
            .expect("one small record fits any run");
        run_lines(&run)
    };
    let a = mk((1, 2));
    let b = mk((3, 4));
    assert_ne!(a, b, "container-key records must render distinguishably");
}
```

Command:

```text
cargo nextest run -p rumors --all-features -E 'test(witness_)'
```

Output:

```text
    thread '...witness_capture_atlas_13_container_key_collapses' panicked at src/tree/mirror/streaming/remote/codec/capture/tests.rs:373:5:
    assertion `left != right` failed: container-key records must render distinguishably
      left: ["63(<< / embedded sequence, 1 item(s), 13 bytes /", "  63(<< / embedded sequence, 2 item(s), 10 bytes /", "    53846(h'e0') / causal version, event tree: 0 /", "    {", "      … => 7", "    }", "  >>)", ">>)"]
     right: ["63(<< / embedded sequence, 1 item(s), 13 bytes /", "  63(<< / embedded sequence, 2 item(s), 10 bytes /", "    53846(h'e0') / causal version, event tree: 0 /", "    {", "      … => 7", "    }", "  >>)", ">>)"]
```

Explanation:

DEMONSTRATED. Two supply records carry distinct wire bytes — payloads `A1 82 01 02 07` (map {[1,2]:7}) versus `A1 82 03 04 07` (map {[3,4]:7}), differing in the array key's element bytes — yet render to byte-identical line vectors. The map-key render path (`scalar(key).unwrap_or_else(|| "…".into())`, capture.rs:470) returns None for a Node::Array key (scalar() returns None for Array/Map, capture.rs:705) and emits `… => 7` for both, discarding the key's content. Both encodings are equal length (every integer a one-byte head), so even the `{} bytes` annotations match (both '2 items, 10 bytes'). This violates the module doc's injectivity contract ('two different byte streams cannot render identically', lines 20-22) on the application-payload input class the doc says only ever falls back explicitly (lines 46-47). The path is reachable through any serde payload with tuple/struct keys via the public send API (Message::new serializes tuple keys as CBOR arrays). Matching the finding: the renderer is test-only and no committed fixture carries a container key, so the hole is latent — the demonstration required constructing such a fixture.

## tests-observation-28

Outcome: demonstrated

Verdict: CONFIRMED

Test code:

```rust
// Edit to tests/common/overlap.rs, in build_overlap_schedule's
// `Choice::Close` arm — replacing the shadow's delivery call
// `sim.merge_session(&snapshot, a, b);` with a no-op that still
// consumes the bindings:
            Choice::Close { slot } => {
                let slot = slot % SLOTS;
                let Some((a, b, snapshot)) = open.remove(&slot) else {
                    continue;
                };
                // WITNESS tests-observation-28: corrupt the shadow's
                // Close-delivery. If no meta-test guards the overlap
                // shadow, the suite stays green despite this drift.
                let _ = (&snapshot, a, b);
                events.push(OverlapEvent::Close { slot });
            }
// (The existing `overlapping_schedules_converge_to_the_oracle` proptest
// in tests/session_overlap.rs, 48 cases, is then run unchanged.)
```

Command:

```text
cargo nextest run -p rumors --all-features -E 'test(overlapping_schedules_converge_to_the_oracle)'
```

Output:

```text
        PASS [   0.468s] (1/1) rumors::session_overlap overlapping_schedules_converge_to_the_oracle
     Summary [   0.469s] 1 test run: 1 passed, 841 skipped
```

Explanation:

DEMONSTRATED. Corrupting the overlap generator's Knowledge shadow so a Close delivers nothing to the shadow state leaves the whole session_overlap proptest suite green (48 cases pass). This is the verification gap the finding names: the serial-schedule shadow has a committed validity meta-test (tests/shadow_validity.rs::shadow_predicts_live_state, which drives arb_schedule_with_shadow and arb_membership_schedule_with_shadow), but the overlap generator's shadow has no `arb_overlap_*_with_shadow` twin and no meta-test, so its drift raises no failing signal — verified by reading tests/shadow_validity.rs (it never references the overlap generator) alongside the runtime result. The mechanism the finding cites also holds by reading: the executor degrades a shadow/live mismatch to a skipped Redact on BOTH sides symmetrically (overlap.rs:237-241, `if observed { peers[*peer].redact_one(version); oracle.redact(*target_event_idx) }`), and the oracle is built from replayed events independent of the shadow, so a thinned/incorrect shadow only shrinks the tested redaction space rather than breaking convergence — exactly why the corrupted run stays green. The finding's third sub-claim (the Open variant's doc 'forking both sides' working state here' at overlap.rs:178-180 is inaccurate because the real session forks working state only after the preamble exchange) is a documentation-accuracy point I assessed by reading, not by this dynamic test.

## deps-3

Outcome: demonstrated

Verdict: CONFIRMED

Test code:

```rust
// No test file: the demonstration is a rustdoc build under the pinned
// nightly the gate names (nightly-2026-06-30), with the docs.rs cfg the
// Cargo.toml metadata passes. Run at HEAD, no source edits:
//   RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-06-30 doc -p rumors --all-features --no-deps
// The attribute under test is src/lib.rs:299:
//   #![cfg_attr(docsrs, feature(doc_auto_cfg))]
```

Command:

```text
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-06-30 doc -p rumors --all-features --no-deps
```

Output:

```text
 Documenting rumors v0.1.0 (/Users/oxide/src/rumors/.claude/worktrees/wf_866cfd1a-b96-194)
error[E0557]: feature has been removed
   --> src/lib.rs:299:29
    |
299 | #![cfg_attr(docsrs, feature(doc_auto_cfg))]
    |                             ^^^^^^^^^^^^ feature has been removed
    |
    = note: removed in 1.92.0; see <https://github.com/rust-lang/rust/pull/138907> for more information
    = note: merged into `doc_cfg`

error: Compilation failed, aborting rustdoc

error: could not document `rumors`
```

Explanation:

DEMONSTRATED. Under the pinned nightly-2026-06-30 (the toolchain the gate's nightly legs name) with `--cfg docsrs` (the exact cfg Cargo.toml's [package.metadata.docs.rs] rustdoc-args passes, and what docs.rs builds with), the crate fails to document: `error[E0557]: feature has been removed` at src/lib.rs:299, naming doc_auto_cfg, with the note 'removed in 1.92.0 ... merged into `doc_cfg`' — precisely the finding's prediction. So the docs.rs configuration is broken: the very build docs.rs runs would error out, and nothing in the gate or ci ever compiles the `--cfg docsrs` path, so the breakage is invisible to the workspace's own verification. Resolution is owner-gated in spirit (it changes how the published docs render) but is a straightforward fix — RFC 3631 stabilized doc_auto_cfg into the always-on doc_cfg, so on a post-1.92 compiler the `#![feature(doc_auto_cfg)]` gate must be dropped (and the auto-cfg labelling is then on by default under a supporting rustdoc). I did not attempt the post-fix build (would require editing lib.rs), but the E0557 note text ('merged into doc_cfg') is the compiler's own directive.

## conformance-25

Outcome: demonstrated

Test code:

```rust
// Edits to src/conformance/backend/tests.rs (final form, iteration 2).
// (1) new knob, after `static ASSEMBLE_SKIPS`:
/// Witness (conformance-25): extra bytes the lazily loaded child rows keep
/// resident: honest at zero, over-holding above it.
static CHILDREN_SLACK: Knob = Knob::new(0);

// (2) in `impl Backend for Materializing`, fn children, replacing
//     `let row = ROW_HEADER + bounds_of(&node);`:
                    // Witness (conformance-25), iteration 2: over-hold only the
                    // interior children, the per-depth held references; walked
                    // leaves stay honest so the walk seam's leaf check is silent.
                    let slack = if H::HEIGHT > 0 { CHILDREN_SLACK.get() } else { 0 };
                    let row = ROW_HEADER + bounds_of(&node) + slack;
//     (iteration 1 was the finding's literal construction, slack on every child:
//      `let row = ROW_HEADER + bounds_of(&node) + CHILDREN_SLACK.get();`)

// (3) appended test:
/// Witness (conformance-25): a `children` override that over-holds 64 KiB
/// per exploded interior child row (the walk's in-flight references).
/// The finding predicts no pointwise violation is recorded at this seam,
/// so the run passes unless the differenced end-to-end peak happens to
/// cross the stated budget.
#[test]
fn zz_witness_overholding_children_passes_the_suite() {
    let _dishonest = CHILDREN_SLACK.set(BULK_OVERHOLD);
    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-run   # then, iteration 1 (slack on all children):
cargo nextest run -p rumors --all-features -E 'test(zz_witness_)' --success-output immediate --failure-output immediate --no-fail-fast
# iteration 2 (slack on interior children only):
cargo nextest run -p rumors --all-features -E 'test(zz_witness_overholding_children_passes_the_suite)' --success-output immediate --failure-output immediate
```

Output:

```text
Iteration 1 (slack on every child, the finding's literal construction):
        FAIL [   0.675s] (5/5) rumors conformance::backend::tests::zz_witness_overholding_children_passes_the_suite
    thread 'conformance::backend::tests::zz_witness_overholding_children_passes_the_suite' (239147069) panicked at src/conformance/backend.rs:657:5:
    the backend's node_bytes contract failed pointwise:
    underpriced walked leaf: measured 65644 B, node_bytes priced 108 B
    underpriced walked leaf: measured 65640 B, node_bytes priced 104 B
    ... (6144 lines, every one `underpriced walked leaf`; zero `underpriced node`, zero budget-check lines)

Iteration 2 (slack on interior children only):
        PASS [   0.644s] (1/1) rumors conformance::backend::tests::zz_witness_overholding_children_passes_the_suite
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 524 filtered out; finished in 0.63s
EXIT=0
```

Explanation:

Verified by running. The finding's literal construction (64 KiB slack on every child row) does NOT pass: it is caught, but by the walk seam, not the children seam, and only for an incidental reason. Every one of the 6144 violations is `underpriced walked leaf` (2 corpora x 1536 leaves x 2 runs); none is an `underpriced node`, and the budget assertion never fired. The reason is visible in src/tree/mirror/streaming/convert.rs:63-77: `Convert::explode` for `S<H>` drives `backend.children::<H>(prefix, node)`, and the reference backend's `Materializing::leaves` is `H::explode` (tests.rs:367-370), so leaf-height children re-enter `Charged::leaves` where the walked-leaf pointwise check (backend.rs, `underpriced walked leaf`) prices them. Interior children never pass that check. Restricting the slack to `H::HEIGHT > 0` (the per-depth held references the window prices, window.rs:372-388 per the finding) makes the whole `check(Materializing, MATERIALIZING_BUDGET)` pass: no violation at any seam and the differenced peak stayed under 4 MiB. So the claim stands in refined form: `Charged::children` has no pointwise check and no negative control; an over-holding `children` override is invisible for interior children, and for leaf-height children it is caught only because this reference backend's `leaves` happens to route through `children` (a real backend with a bulk leaf scan would not). One caveat, inferred not instrumented: I did not count how many interior children the session exploded with slack; the divergence of 1024 leaves per side under hashed paths cannot reconcile without exploding interior nodes, and the bulk walk explodes every level, so the slack was live.

Evidence paths: <session scratchpad>/witness/3/run1_unmutated.log, run2_children_interior.log, witness_25_conformance.diff, witness_25_conformance_iter2.diff

## materialized-27

Outcome: demonstrated

Test code:

```rust
// Appended to src/tree/mirror/streaming/tests.rs

/// Witness (materialized-27): a real height-1 branch of two sibling leaves
/// (paths sharing 31 bytes), classified `Between` against a counterparty
/// whose ceiling dominates one leaf and is concurrent with the other, must
/// ship exactly the concurrent leaf and honor the dominated one's deletion,
/// in both orientations. This is the shape whose verdict lands in the
/// height-0 arm of the streaming `unknown` filter.
#[test]
fn zz_witness_leaf_height_verdict_is_judged_pointwise() {
    use crate::Version;
    use crate::message::Message;
    use crate::tree::arb::nth_party;
    use crate::tree::traverse::{Action, act};
    use crate::tree::typed::Path;

    fn sibling(last: u8) -> Path {
        let mut bytes = [0u8; 32];
        bytes[31] = last;
        Path::from(bytes)
    }

    let mut v00 = Version::new();
    v00.tick(&nth_party(0));
    let mut v01 = Version::new();
    v01.tick(&nth_party(1));
    let a_node = act(
        None,
        vec![
            (sibling(0x00), v00.clone(), Action::Insert(Message::new(()))),
            (sibling(0x01), v01.clone(), Action::Insert(Message::new(()))),
        ],
        &mut |_| (),
    );
    let a = Root {
        ceiling: v00.clone() | v01.clone(),
        root: a_node,
    };

    // b holds nothing, but its ceiling dominates v00 (the 0x00 leaf is
    // redacted from b's point of view) and is concurrent with v01.
    let mut forget = Version::new();
    forget.tick(&nth_party(2));
    let b = Root {
        ceiling: v00.clone() | forget,
        root: None,
    };

    let expected = join_oracle(a.clone(), b.clone());
    assert_eq!(
        expected.root.as_ref().map(|node| node.len()),
        Some(1),
        "the oracle keeps exactly one leaf (the concurrent sibling): {expected:?}",
    );

    for (left, right) in [(a.clone(), b.clone()), (b, a)] {
        let (ours, theirs) = streaming_mirror_sides(left, right);
        assert_eq!(ours, expected, "initiating side must hold the oracle's tree");
        assert_eq!(theirs, expected, "responding side must hold the oracle's tree");
    }
}

// Mutant applied for the second run (src/tree/mirror/streaming/materialized/unknown.rs:92):
// -            let verdict = Some(node).filter(|node| !self::known(node, known));
// +            let verdict = Some(node).filter(|node| self::known(node, known));
```

Command:

```text
# HEAD (unmutated):
cargo nextest run -p rumors --all-features -E 'test(zz_witness_)' --success-output immediate --failure-output immediate --no-fail-fast
# with the `!` removed at unknown.rs:92:
cargo nextest run -p rumors --all-features -E 'test(zz_witness_leaf_height_verdict_is_judged_pointwise)' --success-output immediate --failure-output immediate
```

Output:

```text
HEAD:
        PASS [   0.018s] (2/5) rumors tree::mirror::streaming::tests::zz_witness_leaf_height_verdict_is_judged_pointwise

Mutant (`!` removed at unknown.rs:92):
        FAIL [   0.014s] (1/1) rumors tree::mirror::streaming::tests::zz_witness_leaf_height_verdict_is_judged_pointwise
    thread 'tree::mirror::streaming::tests::zz_witness_leaf_height_verdict_is_judged_pointwise' (239180594) panicked at src/tree/mirror/streaming/tests.rs:326:9:
    assertion `left == right` failed: initiating side must hold the oracle's tree
      left: Root { ceiling: (0, (0, (0, 0, 1), 1), 1), root: Some(Node { prefix: "0000000000000000000000000000000000000000000000000000000000000000", children: Leaf { version: (0, 0, 1), message: Message { serialized: "f6", .. } } }) }
     right: Root { ceiling: (0, (0, (0, 0, 1), 1), 1), root: Some(Node { prefix: "0100000000000000000000000000000000000000000000000000000000000000", children: Leaf { version: (0, (0, 0, 1), 0), message: Message { serialized: "f6", .. } } }) }
```

Explanation:

Verified by running both ways. On HEAD the constructed pair converges to the join oracle in both orientations (the oracle keeps exactly one leaf, the concurrent sibling 0x01). With the `!` at unknown.rs:92 removed, the first orientation fails: the initiating side ends holding only the leaf at the all-zero path with version `(0, 0, 1)` (v00 on party 0, the leaf b's ceiling dominates, i.e. the redacted one) and lacks the 0x01 leaf at version `(0, (0, 0, 1), 0)` (v01 on party 1); the oracle holds the reverse. That is exactly the finding's prediction ("b receives 0x00 and not 0x01; equality with join_oracle fails") and confirms the reading: this shape (two leaves sharing 31 path bytes under a `Between`-classified spine, counterparty lacking the parent) is what makes the height-0 arm the deciding verdict, and it is the redaction contract that inverts under the mutant (the redacted leaf is re-supplied and absorbed). What I did not re-verify: the finding's premise that the mutant survives the whole committed suite (the handoff note's claim); running the full suite was out of scope. What I verified is that the committed suite lacked a discriminating test for this arm and that this construction discriminates.

Evidence paths: <session scratchpad>/witness/3/run1_unmutated.log, run3_mutant27.log, mutant_27_unknown.diff, witness_27_streaming_tests_append.rs

## remote-capture-atlas-17

Outcome: demonstrated

Test code:

```rust
// Appended to src/tree/mirror/streaming/remote/codec/capture/tests.rs

// ---- Witness (remote-capture-atlas-17): generative injectivity ---------------

use proptest::prelude::*;

/// A test-local value grammar mirroring the renderer's node grammar. Tag
/// 24 / 63 content is generated as the byte string of a nested encoding
/// (`Embedded`) so the unfold path is exercised on well-formed content.
#[derive(Clone, Debug)]
enum T {
    Uint(u64),
    Nint(u64),
    Bytes(Vec<u8>),
    Text(String),
    Simple(u8),
    Float(u8, u64),
    Array(Vec<T>),
    Map(Vec<(T, T)>),
    Tag(u64, Box<T>),
    Embedded(u64, Vec<T>),
}

/// Canonical encoding of `t`: shortest-form heads through the codec's own
/// `write_head`, major-7 spelled by hand (simples below 24 inline, 32 and
/// above as one byte, floats by width).
fn encode_t(t: &T, out: &mut Vec<u8>) {
    match t {
        T::Uint(v) => cbor::write_head(out, MAJOR_UINT, *v),
        T::Nint(v) => cbor::write_head(out, 1, *v),
        T::Bytes(b) => {
            cbor::write_head(out, MAJOR_BSTR, b.len() as u64);
            out.extend_from_slice(b);
        }
        T::Text(s) => {
            cbor::write_head(out, MAJOR_TEXT, s.len() as u64);
            out.extend_from_slice(s.as_bytes());
        }
        T::Simple(v) if *v < 24 => out.push(0xe0 | *v),
        T::Simple(v) => out.extend_from_slice(&[0xf8, *v]),
        T::Float(25, bits) => {
            out.push(0xf9);
            out.extend_from_slice(&(*bits as u16).to_be_bytes());
        }
        T::Float(26, bits) => {
            out.push(0xfa);
            out.extend_from_slice(&(*bits as u32).to_be_bytes());
        }
        T::Float(_, bits) => {
            out.push(0xfb);
            out.extend_from_slice(&bits.to_be_bytes());
        }
        T::Array(items) => {
            cbor::write_head(out, MAJOR_ARRAY, items.len() as u64);
            for item in items {
                encode_t(item, out);
            }
        }
        T::Map(entries) => {
            cbor::write_head(out, MAJOR_MAP, entries.len() as u64);
            for (key, value) in entries {
                encode_t(key, out);
                encode_t(value, out);
            }
        }
        T::Tag(number, content) => {
            cbor::write_head(out, MAJOR_TAG, *number);
            encode_t(content, out);
        }
        T::Embedded(number, items) => {
            let mut inner = Vec::new();
            for item in items {
                encode_t(item, &mut inner);
            }
            cbor::write_head(out, MAJOR_TAG, *number);
            cbor::write_head(out, MAJOR_BSTR, inner.len() as u64);
            out.extend_from_slice(&inner);
        }
    }
}

/// Scalars that always render inline (no tags: protocol tags render as
/// blocks, so a tagged key would be a container for the renderer).
fn arb_scalar_t() -> BoxedStrategy<T> {
    prop_oneof![
        any::<u64>().prop_map(T::Uint),
        (0u64..1000).prop_map(T::Uint),
        any::<u64>().prop_map(T::Nint),
        prop::collection::vec(any::<u8>(), 0..8).prop_map(T::Bytes),
        prop::collection::vec(
            prop_oneof![
                Just('a'),
                Just('"'),
                Just('\\'),
                Just(' '),
                Just('0'),
                Just('…'),
                Just('(')
            ],
            0..5,
        )
        .prop_map(|chars| T::Text(chars.into_iter().collect())),
        prop_oneof![0u8..24, 32u8..=255].prop_map(T::Simple),
        (0u64..0x1_0000).prop_map(|bits| T::Float(25, bits)),
        (0u64..0x1_0000_0000).prop_map(|bits| T::Float(26, bits)),
        any::<u64>().prop_map(|bits| T::Float(27, bits)),
    ]
    .boxed()
}

/// Tag numbers the renderer names, plus generic ones.
fn arb_tag_number() -> BoxedStrategy<u64> {
    prop_oneof![
        Just(1u64),
        Just(TAG_EMBEDDED_ITEM),
        Just(TAG_CBOR_SEQUENCE),
        Just(cbor::TAG_SELF_DESCRIBED),
        Just(crate::tags::PARTY_TAG),
        Just(crate::tags::VERSION_TAG),
        Just(crate::tags::CLOCK_TAG),
        100u64..200,
    ]
    .boxed()
}

/// Recursive values. `container_keys` admits containers (arrays, maps,
/// tags, embedded strings) as map keys; when false, keys are inline scalars.
fn arb_t(container_keys: bool) -> BoxedStrategy<T> {
    arb_scalar_t()
        .prop_recursive(4, 24, 4, move |inner| {
            let key = if container_keys {
                inner.clone()
            } else {
                arb_scalar_t()
            };
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..4).prop_map(T::Array),
                prop::collection::vec((key, inner.clone()), 0..4).prop_map(T::Map),
                (arb_tag_number(), inner.clone())
                    .prop_map(|(number, content)| T::Tag(number, Box::new(content))),
                (
                    prop_oneof![Just(TAG_EMBEDDED_ITEM), Just(TAG_CBOR_SEQUENCE)],
                    prop::collection::vec(inner.clone(), 0..3),
                )
                    .prop_map(|(number, items)| T::Embedded(number, items)),
            ]
        })
        .boxed()
}

/// Number of scalar leaves in `t`, in pre-order.
fn scalar_count(t: &T) -> usize {
    match t {
        T::Array(items) | T::Embedded(_, items) => items.iter().map(scalar_count).sum(),
        T::Map(entries) => entries
            .iter()
            .map(|(key, value)| scalar_count(key) + scalar_count(value))
            .sum(),
        T::Tag(_, content) => scalar_count(content),
        _ => 1,
    }
}

/// Replace the `index`-th scalar leaf (pre-order) of `t` with `with`;
/// returns whether a leaf was replaced.
fn replace_scalar(t: &mut T, index: &mut usize, with: &T) -> bool {
    match t {
        T::Array(items) | T::Embedded(_, items) => items
            .iter_mut()
            .any(|item| replace_scalar(item, index, with)),
        T::Map(entries) => entries.iter_mut().any(|(key, value)| {
            replace_scalar(key, index, with) || replace_scalar(value, index, with)
        }),
        T::Tag(_, content) => replace_scalar(content, index, with),
        _ => {
            if *index == 0 {
                *t = with.clone();
                true
            } else {
                *index -= 1;
                false
            }
        }
    }
}

/// Encode `t` and render it through the control-item entry point.
fn render_t(t: &T) -> (Vec<u8>, String) {
    let mut bytes = Vec::new();
    encode_t(t, &mut bytes);
    let mut out = String::new();
    render_item(&bytes, "", &mut out);
    (bytes, out)
}

/// A value plus one pre-order scalar position and a replacement scalar.
fn arb_perturbation(container_keys: bool) -> BoxedStrategy<(T, usize, T)> {
    arb_t(container_keys)
        .prop_flat_map(|t| {
            let leaves = scalar_count(&t).max(1);
            (Just(t), 0..leaves, arb_scalar_t())
        })
        .boxed()
}

fn assert_perturbation_renders_distinctly(
    tree: T,
    index: usize,
    with: T,
) -> Result<(), TestCaseError> {
    let mut perturbed = tree.clone();
    let mut cursor = index;
    prop_assume!(replace_scalar(&mut perturbed, &mut cursor, &with));
    let (a_bytes, a_text) = render_t(&tree);
    let (b_bytes, b_text) = render_t(&perturbed);
    prop_assume!(a_bytes != b_bytes);
    prop_assert!(
        a_text != b_text,
        "distinct wire bytes rendered identically\n  bytes A: {}\n  bytes B: {}\n  rendering:\n{}",
        hex::encode(&a_bytes),
        hex::encode(&b_bytes),
        a_text,
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 2000,
        max_global_rejects: 100_000,
        .. ProptestConfig::default()
    })]

    /// Witness (remote-capture-atlas-17), form (b): perturbing one scalar
    /// leaf anywhere in a generated value (container map keys admitted)
    /// changes the wire bytes, so by the module doc's injectivity claim it
    /// must change the rendering.
    #[test]
    fn zz_witness_rendering_is_injective_under_leaf_perturbation(
        (tree, index, with) in arb_perturbation(true)
    ) {
        assert_perturbation_renders_distinctly(tree, index, with)?;
    }

    /// The same property with map keys restricted to inline scalars: probes
    /// for any injectivity hole other than a non-scalar map key.
    #[test]
    fn zz_witness_rendering_is_injective_with_scalar_keys(
        (tree, index, with) in arb_perturbation(false)
    ) {
        assert_perturbation_renders_distinctly(tree, index, with)?;
    }
}
```

Command:

```text
cargo nextest run -p rumors --all-features -E 'test(zz_witness_)' --success-output immediate --failure-output immediate --no-fail-fast
```

Output:

```text
        FAIL [   0.201s] (3/5) rumors tree::mirror::streaming::remote::codec::capture::tests::zz_witness_rendering_is_injective_under_leaf_perturbation
    proptest: Aborting shrinking after the PROPTEST_MAX_SHRINK_ITERS environment variable or ProptestConfig.max_shrink_iters iterations (set 8000 to a large(r) value to shrink more; current configuration: 8000 iterations)
    proptest: Saving this and future failures in /Users/oxide/src/rumors/.claude/worktrees/wf_866cfd1a-b96-195/proptest-regressions/tree/mirror/streaming/remote/codec/capture/tests.txt
    cc 1814edf5e76522544a23bf145f686515bf7a285a0d7fe2cfd2db8b9e9f5b23ca

    thread 'tree::mirror::streaming::remote::codec::capture::tests::zz_witness_rendering_is_injective_under_leaf_perturbation' (239147068) panicked at src/tree/mirror/streaming/remote/codec/capture/tests.rs:580:1:
    Test failed: distinct wire bytes rendered identically
      bytes A: a1a1fb000000ff0e084672fb89a77a28480bfd5ffaae8b2e12
      bytes B: a1a13b005ea3c56d22b8b7fb89a77a28480bfd5ffaae8b2e12
      rendering:
    {
      … => float32'ae8b2e12'
    }
     at src/tree/mirror/streaming/remote/codec/capture/tests.rs:570.
    minimal failing input: (tree, index, with) = (
        Map(
            [
                (
                    Map(
                        [
                            (
                                Float(
                                    27,
                                    1095452083826,
                                ),
                                Float(
                                    27,
                                    9919031017732832607,
                                ),
                            ),
                        ],
                    ),
                    Float(
                        26,
                        2928356882,
                    ),
                ),
            ],
        ),
        0,
        Nint(
            26638716145678519,
        ),
    )
    successes: 5
    local rejects: 0
    global rejects: 2

        PASS [   0.494s] (4/5) rumors tree::mirror::streaming::remote::codec::capture::tests::zz_witness_rendering_is_injective_with_scalar_keys
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 524 filtered out; finished in 0.49s
```

Explanation:

Verified by running. The generative form-(b) test the finding describes refutes the module doc's injectivity claim after 5 successful cases: two distinct canonical byte streams (a1a1fb... vs a1a13b..., differing in a float-vs-negative-int scalar inside a map used as a map key) render to the identical text `{ … => float32'ae8b2e12' }`. The hole is the `…` elision at capture.rs:470 (`let key = scalar(key).unwrap_or_else(|| "…".into());`), the same one the finding cites as finding 13, and the same elision exists at capture.rs:523 in `render_listing`. So the verification gap is real and consequential: the committed suite's point examples (tests.rs:3-10 commitments; each fallback pinned by one hand-built input) cannot see it, and the generative test finds it immediately. The second test, with map keys restricted to inline scalars, passed 2000 cases: within this generator's coverage (uints, nints, byte and text strings including quotes/backslashes/the `…` glyph, simples, all three float widths, arrays, maps, generic and protocol tags, tag-24/63 embedded sequences of nested encodings, depth up to 4), no injectivity hole other than the non-scalar map key surfaced. That bounds but does not prove the "next hole" claim; it does not cover non-canonical inputs (which fall back to hex by design) or the frame-level grammar (`render_frame`). Shrinking stopped at the 8000-iteration cap, so the printed counterexample is not minimal; a hand-minimal one is `{[1]: 0}` vs `{[2]: 0}`. Note for cleanup: this run wrote a proptest seed file under proptest-regressions/tree/mirror/streaming/remote/codec/capture/tests.txt, which I copied to the scratchpad (seed_capture_tests/) and deleted from the worktree.

Evidence paths: <session scratchpad>/witness/3/run1_unmutated.log, seed_capture_tests/tests.txt, witness_17_capture_tests_append.rs

## tests-bookmark-9

Outcome: demonstrated

Test code:

```rust
// Appended to tests/bookmark_causality.rs

/// Witness (tests-bookmark-9): a never-redacted message that became durable
/// (propagated to B) must survive a clean heal at every live peer.
///
/// The committed oracle (`assert_healed`, which runs `promote`'s recycle
/// check as the durable set grows and the post-heal durability witness) is
/// applied first; the survival check follows, so a run that passes the
/// former and fails the latter shows the oracle blind to a recycle that
/// destroys content.
#[test]
fn zz_witness_recycle_that_destroys_content() {
    let (a, b, c) = (2usize, 1usize, 0usize);
    let mut world = World::single_network(3);

    // m = seq 0, emitted by A.
    world.send(a);
    let v_m = world.nodes[a]
        .pending
        .last()
        .expect("the send is pending")
        .version
        .clone();
    world.gossip(a, b, FaultPlan::NONE, FaultPlan::NONE);
    let network = world.nodes[b].network;
    assert!(
        world.emissions.contains_exact(network, 0, &v_m),
        "m is durable: it propagated to B",
    );

    // C advances on its own, never seeing m.
    for _ in 0..3 {
        world.send(c);
    }

    // A crashes; the next step revives it from the lowest-index live peer,
    // C, then gossips with C (running A's bookmark update and reclaim).
    world.crash(a);
    world.gossip(a, c, FaultPlan::NONE, FaultPlan::NONE);
    for _ in 0..4 {
        world.send(a);
    }

    let a_live = world.nodes[a].live().expect("A is live after its revival");
    let frontier = a_live.snapshot().latest().clone();
    eprintln!(
        "A' frontier {frontier:?} vs v_m {v_m:?}: {:?}",
        frontier.partial_cmp(&v_m)
    );
    eprintln!("A' party: {:?}", a_live.dangerously_alias_party());
    for emission in &world.nodes[a].pending {
        eprintln!(
            "A' pending #{} version {:?} vs v_m: {:?}",
            emission.seq,
            emission.version,
            emission.version.partial_cmp(&v_m),
        );
    }

    let holds_m = |world: &World, k: usize| {
        world.nodes[k]
            .live()
            .map(|rumors| rumors.snapshot().iter().any(|(_, value)| *value == 0))
            .unwrap_or(false)
    };
    assert!(holds_m(&world, b), "m is live at B before the heal");

    world.heal();
    world.assert_healed();
    let survivors: Vec<bool> = (0..3).map(|k| holds_m(&world, k)).collect();
    eprintln!("m survives at nodes 0,1,2: {survivors:?}");
    assert!(
        survivors.iter().all(|s| *s),
        "the never-redacted durable message m was destroyed by the heal (survival per node 0,1,2: {survivors:?})",
    );
}

// Mutant applied for the second run (src/bookmark.rs:469, inside `reclaim`'s alias push):
// -            version.clone(),
// +            Version::new(),
```

Command:

```text
# HEAD (unmutated):
cargo nextest run -p rumors --all-features -E 'test(zz_witness_)' --success-output immediate --failure-output immediate --no-fail-fast
# with the reclaim alias pushed at Version::new() (src/bookmark.rs:469):
cargo nextest run -p rumors --all-features -E 'test(zz_witness_recycle_that_destroys_content)' --success-output immediate --failure-output immediate
```

Output:

```text
HEAD:
        PASS [   0.016s] (1/5) rumors::bookmark_causality zz_witness_recycle_that_destroys_content
    A' frontier (0, (0, (3, 0, 4), 0), 0) vs v_m (0, (0, 0, 1), 0): None
    A' party: Some((((0, 1), 0), 0))
    A' pending #4 version (0, (0, (3, 0, 1), 0), 0) vs v_m: None
    A' pending #5 version (0, (0, (3, 0, 2), 0), 0) vs v_m: None
    A' pending #6 version (0, (0, (3, 0, 3), 0), 0) vs v_m: None
    A' pending #7 version (0, (0, (3, 0, 4), 0), 0) vs v_m: None
    m survives at nodes 0,1,2: [true, true, true]

Mutant (alias at Version::new()):
        FAIL [   0.014s] (1/1) rumors::bookmark_causality zz_witness_recycle_that_destroys_content
    A' frontier (0, (3, 0, 3), 0) vs v_m (0, (0, 0, 1), 0): Some(Greater)
    A' party: Some((((0, 1), 1), 0))
    A' pending #4 version (0, 3, 0) vs v_m: Some(Greater)
    A' pending #5 version (0, (3, 0, 1), 0) vs v_m: Some(Greater)
    A' pending #6 version (0, (3, 0, 2), 0) vs v_m: Some(Greater)
    A' pending #7 version (0, (3, 0, 3), 0) vs v_m: Some(Greater)
    m survives at nodes 0,1,2: [false, false, false]

    thread 'zz_witness_recycle_that_destroys_content' (239189199) panicked at tests/bookmark_causality.rs:1428:5:
    the never-redacted durable message m was destroyed by the heal (survival per node 0,1,2: [false, false, false])
```

Explanation:

Verified by running both ways. On HEAD the plan is benign: revived A' does not reclaim its old region (party `(((0, 1), 0), 0)`), its emissions are concurrent with v_m (`None`), and m survives at all three nodes after the heal. Under the mutant, A' reclaims the old region on the gossip with C (party grows to `(((0, 1), 1), 0)`), and its very first post-reclaim emission #4 at version `(0, 3, 0)` compares `Some(Greater)` to v_m = `(0, (0, 0, 1), 0)`: the reclaimed region's coordinate is reused (A' never held m) while the full version dominates, exactly the shape the finding says `EmissionLog::promote` cannot flag, since it fires only on `Some(Less | Equal)` (tests/bookmark_causality.rs:140-146). Then the heal's sieve deletes m fleet-wide, and `world.assert_healed()` (the committed oracle: promote plus `assert_live_content_is_durable`) passed without complaint; the only assertion that fired is my survival check at line 1428, after `assert_healed` returned. So the committed proptest's headline claim is judged by a blind oracle for the recycle that matters, as the finding states. Two notes: the observed comparison was `Greater` rather than incomparable (the ITC tick's fill lifted A's region to height 3 in one step, so the frontier dominated v_m after a single send; the four sends were surplus), and the World harness recorded m as durable via propagation before the crash, so this is a durable-message deletion, not a pending-emission loss. I did not re-run the committed `bookmarking_never_recycles_a_version` proptest under the mutant (out of cap/scope); the demonstration is that the oracle it uses passes on a plan where content is destroyed.

Evidence paths: <session scratchpad>/witness/3/run1_unmutated.log, run4_mutant9.log, mutant_9_bookmark.diff, witness_9_bookmark_causality_append.rs

## Cleanup status

Witness agent 1 (scratchpad witness/1; findings link-28, conformance-31, remote-codec-14, streaming-tests-11, tests-disruption-handshake-7):

Worktree /Users/oxide/src/rumors/.claude/worktrees/wf_866cfd1a-b96-193 at HEAD 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (verified at start and after restore). (1) Build dir resolved from inside the worktree before any removal: /Volumes/forge/build/d2/1abd7526c381ec (recorded in scratchpad witness/1/build_directory.txt). (2) The five edited files (src/conformance/backend/tests.rs, src/link/routed/tests.rs, src/tree/mirror/streaming/remote/codec/decode/tests.rs, src/tree/mirror/streaming/tests/capacity.rs, tests/disruption.rs) restored with `git restore --source=HEAD --staged --worktree`; no new files were created inside the worktree; no proptest seed files appeared. Final `git status --porcelain` output: (empty). `git status --porcelain --ignored` after cache deletion: (empty). (3) Deleted /Volumes/forge/build/d2/1abd7526c381ec and the worktree's local target/ (both confirmed absent with ls). Test sources and evidence survive at <session scratchpad>/witness/1/ (witness_edits_round1.diff with the literal disruption probe and the pre-fix link test, witness_edits_round2.diff with the final form of all five constructions, link_28_witness.rs, build.log/build2.log/build3.log/build4.log, run1.log, run2.log, README.txt).

Witness agent 2 (scratchpad witness/2; findings conformance-24, mirror-common-8, remote-capture-atlas-13, tests-observation-28, deps-3):

git status --porcelain: (empty — clean tree at HEAD 9e5784fb). All four edited files restored via `git restore --source=HEAD --staged --worktree` (src/conformance/backend/tests.rs, src/tree/mirror/framing/tests.rs, src/tree/mirror/streaming/remote/codec/capture/tests.rs, tests/common/overlap.rs); `git diff --stat` also empty. Build dir resolved from inside the worktree (`cargo metadata --no-deps --format-version 1 | jq -r .build_directory` -> /Volumes/forge/build/3b/7198d6ae4d9dbc) and deleted, along with the worktree's local target/. Both confirmed gone. Test sources preserved as <session scratchpad>/witness/2/witness_tests.diff plus the run logs (witness_run.log, obs28_run.log, deps3_doc.log).

Witness agent 3 (scratchpad witness/3; findings conformance-25, materialized-27, remote-capture-atlas-17, tests-bookmark-9):

git status --porcelain: (empty). HEAD verified 9e5784fb4dce977cfbdfd1619886d1482b5ce764 at start and end. Both src mutants (unknown.rs:92, bookmark.rs:469) were reversed by the same string swap and each verified identical to HEAD via `git diff --quiet`; the four edited test files were restored with `git restore --source=HEAD --staged --worktree`; the proptest seed file the injectivity test wrote (proptest-regressions/tree/mirror/streaming/remote/codec/capture/tests.txt) and its directory were removed (copy kept in the scratchpad). Build dir resolved from inside the worktree as /Volumes/forge/build/42/b0879d8a5fab62 (4.0G) and deleted; the worktree's local target/ deleted; both confirmed absent afterwards. The worktree itself was left in place for the coordinator.

## Second pass

These constructions were run after the completeness critic found them cheap and unexecuted.

## tests-bookmark-12

Outcome: demonstrated

Test code:

```rust
// tests/bookmark_causality.rs (temporary edit, restored afterwards)
// 1. Added after `use crate::common::sim::arb_fault;`:

/// WITNESS (tests-bookmark-12): counts every `World::gossip` call, to fail
/// every third one with an injected non-mismatch error.
static WITNESS_GOSSIP_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// WITNESS (tests-bookmark-12): how many sessions were replaced by an
/// injected failure.
static WITNESS_INJECTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// 2. In `World::gossip`, the `let (out_a, out_b) = block_on(async { ... });` statement became:

        // WITNESS (tests-bookmark-12): simulate an intermittent crate bug that
        // fails every third session, on both sides, with a non-mismatch error
        // and no session run at all (the "honest disruption leaves both
        // replicas unchanged" shape this harness documents).
        let witness_inject =
            WITNESS_GOSSIP_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % 3 == 2;
        let (out_a, out_b) = if witness_inject {
            let n = WITNESS_INJECTED.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            eprintln!("WITNESS tests-bookmark-12: injected Err(PartyOverlap) on both sides, injection #{n}");
            (Err(Error::PartyOverlap), Err(Error::PartyOverlap))
        } else {
            block_on(async {
                let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
                let task_a = tokio::spawn(async move {
                    let mut link = fault::faulty(side_a, fault_a);
                    ra.gossip(&mut link).await
                });
                let task_b = tokio::spawn(async move {
                    let mut link = fault::faulty(side_b, fault_b);
                    rb.gossip(&mut link).await
                });
                let (out_a, out_b) = tokio::join!(task_a, task_b);
                (out_a.expect("gossip task a"), out_b.expect("gossip task b"))
            })
        };
```

Command:

```text
cargo nextest run -p rumors --all-features --no-capture -E 'test(bookmarking_prevents_party_leakage)'   (run 1 of 6; log: <scratchpad>/witness/pass2/run1_bookmark.log; diff: witness_bookmark12.diff)
```

Output:

```text
452 lines of the form `WITNESS tests-bookmark-12: injected Err(PartyOverlap) on both sides, injection #N` (counted with `grep -c`), then:

test bookmarking_prevents_party_leakage ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 3.27s

        PASS [   3.279s] (1/1) rumors::bookmark_causality bookmarking_prevents_party_leakage
────────────
     Summary [   3.280s] 1 test run: 1 passed, 837 skipped
```

Explanation:

Verified by running. Under `run_reliable_plan` (clean wires, empty fault feeds, one network) every third `World::gossip` was replaced by `Err(Error::PartyOverlap)` on both sides with no session run: a stand-in for any state-dependent, non-`NetworkMismatch` failure (a codec `InvalidData`, `Bookmark(Format)`, a panic swallowed by `let _ = serve_out`). 452 sessions failed this way across the property's cases and `bookmarking_prevents_party_leakage` passed, because `gossip` inspects results only for `NetworkMismatch` (tests/bookmark_causality.rs:530-531) and `heal`'s `clean_gossip` (which does assert) runs only after the plan and converges the fleet regardless. A harness that asserted non-mismatch errors are impossible under the fault-free regime would have failed on the first injection. Bound: I ran only the leakage property (the finding names it); the injection hook also sits under `run_plan`, which I did not run.

## conformance-28

Outcome: demonstrated

Test code:

```rust
// src/conformance/backend.rs, in `check`, inserted immediately before
// `let admitted = budget_peak.saturating_sub(floor_peak);` (temporary edit, restored):

    // WITNESS (conformance-28): the ceiling below needs a liveness floor.
    // Print both peaks and the derived per-height capacities under both
    // configurations, then require the budgeted run to admit more than the
    // floor run; identical peaks mean the two runs were the same run.
    eprintln!(
        "WITNESS conformance-28: floor_peak={floor_peak} budget_peak={budget_peak} budget_bytes={budget_bytes}"
    );
    {
        use crate::tree::mirror::streaming::window::Window;
        let len = (COMMON + DIVERGENT) as u64;
        let floor = Window::from_budget(len, len, 0, 0, 0, B::node_bytes);
        let budgeted = Window::from_budget(len, len, 0, 0, budget_bytes, B::node_bytes);
        let floor_caps: Vec<usize> = (0..=32).map(|h| floor.capacity(h)).collect();
        let budget_caps: Vec<usize> = (0..=32).map(|h| budgeted.capacity(h)).collect();
        eprintln!("WITNESS conformance-28: floor capacities (height 0..=32) = {floor_caps:?}");
        eprintln!("WITNESS conformance-28: budget capacities (height 0..=32) = {budget_caps:?}");
        eprintln!(
            "WITNESS conformance-28: capacities identical under Budget(0) and Budget({budget_bytes}): {}",
            floor_caps == budget_caps
        );
    }
    assert!(
        budget_peak > floor_peak,
        "WITNESS conformance-28: the budgeted run's peak ({budget_peak}) did not exceed the \
         floor run's peak ({floor_peak}); the census ceiling below is vacuous",
    );
```

Command:

```text
cargo nextest run -p rumors --all-features --no-capture -E 'test(local_backend_conforms) | test(materializing_backend_conforms)'   (run 2; fail-fast stopped after the Local failure; log run2_conformance.log)
cargo nextest run -p rumors --all-features --no-capture --no-fail-fast -E 'test(materializing_backend_conforms)'   (run 3; log run3_materializing.log; diff: witness_conformance28.diff)
```

Output:

```text
Run 2 (`local_backend_conforms`, budget 64 KiB):
WITNESS conformance-28: floor_peak=14496 budget_peak=14496 budget_bytes=65536
WITNESS conformance-28: floor capacities (height 0..=32) = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
WITNESS conformance-28: budget capacities (height 0..=32) = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
WITNESS conformance-28: capacities identical under Budget(0) and Budget(65536): true
thread 'conformance::backend::tests::local_backend_conforms' (241316076) panicked at src/conformance/backend.rs:684:5:
WITNESS conformance-28: the budgeted run's peak (14496) did not exceed the floor run's peak (14496); the census ceiling below is vacuous
test conformance::backend::tests::local_backend_conforms ... FAILED

Run 3 (`materializing_backend_conforms`, budget 4 MiB):
WITNESS conformance-28: floor_peak=193707 budget_peak=194740 budget_bytes=4194304
WITNESS conformance-28: floor capacities (height 0..=32) = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
WITNESS conformance-28: budget capacities (height 0..=32) = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 12, 12, 16, 25, 48, 49, 49, 49, 49, 1, 1]
WITNESS conformance-28: capacities identical under Budget(0) and Budget(4194304): false
        PASS [   0.547s] (1/1) rumors conformance::backend::tests::materializing_backend_conforms
```

Explanation:

Verified by running. For `Local` at the 64 KiB budget `local_backend_conforms` passes, the floor run and the budgeted run have identical peaks (14496 measured bytes each), so `admitted` is 0 and the ceiling `admitted <= budget_bytes` cannot fail; the added liveness floor `budget_peak > floor_peak` fails at once. The derived window is all-ones under both `Budget(0)` and `Budget(65536)` (computed with `Window::from_budget` at the corpus sizes `COMMON + DIVERGENT` and version-bytes 0; the session's real version bounds are larger, which can only shrink capacities further), confirming the testdoc's "a tight budget that genuinely binds at this scale" is false: the budget resolves to the floor window. The finding's side remark about `Materializing` also holds: at 4 MiB its budgeted run admits 1033 bytes above the floor (193707 -> 194740) and the capacities differ from height 22 upward, so a floor assertion would pass there. Note: run 2's log records the materializing test as skipped by nextest's fail-fast after the Local failure, hence run 3.

## remote-proxy-tests-10

Outcome: demonstrated

Test code:

```rust
// Appended to src/tree/mirror/streaming/remote/proxy/tests.rs (second iteration; temporary, restored).
// Uses only items the file already imports (Arc, SessionHandle, TreeRoot, Tree, Action, Message,
// nth_party, Handshaking, RemoteHandshaking, Local, Root, WindowConfig, PayloadCodec,
// PayloadDepthLimit, memory_with_capacity, join, mirror, run_to_quiescence, reconcile_locally,
// reconcile_symmetric_accepts, TRANSPORT_CAPACITY).

// ===== WITNESS (remote-proxy-tests-10), second iteration =====

/// WITNESS: per-index frame counts on the data streams one side opened
/// (sent) over its link, keyed by the stream's wire index.
type WitnessOpened = Arc<std::sync::Mutex<std::collections::BTreeMap<u8, usize>>>;

struct WitnessStreams(WitnessOpened);

impl crate::observe::Observer for WitnessStreams {
    fn session(
        &self,
        _: &crate::observe::SessionInfo,
    ) -> Option<Box<dyn crate::observe::SessionObserver>> {
        Some(Box::new(WitnessSession(self.0.clone())))
    }
}

struct WitnessSession(WitnessOpened);

impl crate::observe::SessionObserver for WitnessSession {
    fn stream(
        &self,
        stream: &crate::observe::StreamInfo,
    ) -> Option<Box<dyn crate::observe::StreamObserver>> {
        match stream.id {
            crate::observe::StreamId::Data { index, .. }
                if stream.direction == crate::observe::Direction::Sent =>
            {
                self.0.lock().unwrap().entry(index).or_insert(0);
                Some(Box::new(WitnessStream {
                    index,
                    opened: self.0.clone(),
                }))
            }
            _ => None,
        }
    }
}

struct WitnessStream {
    index: u8,
    opened: WitnessOpened,
}

impl crate::observe::StreamObserver for WitnessStream {
    fn message(&mut self, _bytes: &[u8]) {
        *self.opened.lock().unwrap().entry(self.index).or_insert(0) += 1;
    }
}

fn witness_handle() -> (SessionHandle, WitnessOpened) {
    let opened: WitnessOpened = Default::default();
    let mut attachment = crate::observe::Attachment::default();
    attachment.attach(Arc::new(WitnessStreams(opened.clone())));
    (
        attachment.begin(crate::observe::SessionKind::Gossip),
        opened,
    )
}

/// WITNESS: [`reconcile_symmetric_accepts`]`::<()>` with both proxies
/// observed and both sides' outcomes returned (as `Debug` text on error)
/// rather than unwrapped, plus the per-index frame counts each side sent.
async fn witness_reconcile(
    a: TreeRoot,
    b: TreeRoot,
    transport_capacity: usize,
) -> (
    Result<TreeRoot, String>,
    Result<TreeRoot, String>,
    WitnessOpened,
    WitnessOpened,
) {
    let (handle_a, opened_a) = witness_handle();
    let (handle_b, opened_b) = witness_handle();
    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
    let (a_link, b_link) = memory_with_capacity(transport_capacity);
    let remote_b = RemoteHandshaking::start(
        Local,
        a_link,
        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
    )
    .window(WindowConfig::FLOOR)
    .observe(handle_a);
    let remote_a = RemoteHandshaking::start(
        Local,
        b_link,
        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
    )
    .window(WindowConfig::FLOOR)
    .observe(handle_b);
    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(b, remote_a)));
    let a = a
        .map(|(a, _control)| TreeRoot::from(a))
        .map_err(|e| format!("{e:?}"));
    let b = b
        .map(|(b, _control)| TreeRoot::from(b))
        .map_err(|e| format!("{e:?}"));
    (a, b, opened_a, opened_b)
}

fn witness_report(label: &str, opened_a: &WitnessOpened, opened_b: &WitnessOpened) -> u8 {
    let a = opened_a.lock().unwrap().clone();
    let b = opened_b.lock().unwrap().clone();
    let max = a.keys().chain(b.keys()).copied().max().unwrap_or(0);
    eprintln!(
        "WITNESS remote-proxy-tests-10 [{label}]: frames per sent stream index: side A {a:?}; \
         side B {b:?}; highest stream index opened = {max}"
    );
    max
}

/// WITNESS: the finding's construction verbatim, through the unmodified
/// production-topology helper (no observer attached): the deep dispute
/// fixture must reconcile over the wire to the union it states. The
/// in-process materialized protocol is checked against the same union first,
/// so a failure here localizes to the codec/transport tier.
#[test]
fn zz_witness_leaf_parent_dispute_pair_over_wire() {
    let (a, b, union) = crate::tree::arb::leaf_parent_dispute_pair();
    let local = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
        .expect("in-process reconciliation of the deep dispute remains live");
    assert_eq!(
        local,
        (union.clone(), union.clone()),
        "in-process materialized protocol must reach the fixture's union"
    );
    let (left, right) =
        run_to_quiescence(reconcile_symmetric_accepts::<()>(a, b, TRANSPORT_CAPACITY))
            .expect("the deep dispute must reconcile over the wire");
    assert_eq!((left, right), (union.clone(), union));
}

/// WITNESS: the same construction for the deep redaction fixture.
#[test]
fn zz_witness_leaf_parent_redaction_pair_over_wire() {
    let (a, b, union) = crate::tree::arb::leaf_parent_redaction_pair();
    let local = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
        .expect("in-process reconciliation of the deep redaction remains live");
    assert_eq!(
        local,
        (union.clone(), union.clone()),
        "in-process materialized protocol must reach the fixture's union"
    );
    let (left, right) =
        run_to_quiescence(reconcile_symmetric_accepts::<()>(a, b, TRANSPORT_CAPACITY))
            .expect("the deep redaction must reconcile over the wire");
    assert_eq!((left, right), (union.clone(), union));
}

/// WITNESS (diagnostic): both sides' outcomes and per-stream frame counts
/// for the content-addressed baseline and both deep fixtures, at the suite's
/// small transport capacity and at a generous one, without unwrapping.
#[test]
fn zz_witness_deep_fixtures_diagnostic() {
    let mut a = Tree::<()>::new();
    let mut b = Tree::<()>::new();
    for _ in 0..24 {
        a.act(&nth_party(0), [Action::Insert(Message::new(()))]);
        b.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    }
    let baseline = (a.root, b.root);
    let dispute = {
        let (a, b, _) = crate::tree::arb::leaf_parent_dispute_pair();
        (a, b)
    };
    let redaction = {
        let (a, b, _) = crate::tree::arb::leaf_parent_redaction_pair();
        (a, b)
    };
    for (label, (a, b)) in [
        ("content-addressed baseline", baseline),
        ("leaf_parent_dispute_pair", dispute),
        ("leaf_parent_redaction_pair", redaction),
    ] {
        for capacity in [TRANSPORT_CAPACITY, 64 * 1024] {
            let label = format!("{label}, capacity {capacity}");
            match run_to_quiescence(witness_reconcile(a.clone(), b.clone(), capacity)) {
                Ok((left, right, opened_a, opened_b)) => {
                    witness_report(&label, &opened_a, &opened_b);
                    eprintln!(
                        "WITNESS remote-proxy-tests-10 [{label}]: side A {}; side B {}",
                        match &left {
                            Ok(_) => "Ok".to_string(),
                            Err(e) => format!("Err({e})"),
                        },
                        match &right {
                            Ok(_) => "Ok".to_string(),
                            Err(e) => format!("Err({e})"),
                        },
                    );
                }
                Err(stopped) => eprintln!(
                    "WITNESS remote-proxy-tests-10 [{label}]: session became quiescent: {stopped:?}"
                ),
            }
        }
    }
}

// The first iteration (run 5, saved as witness_proxy10_fragment.rs / witness_proxy10.diff) was one
// test `zz_witness_deep_fixtures_over_wire` that unwrapped side A first; it produced the same baseline
// census and the same `SupplyClosed` on Stream(16), and was replaced by the three tests above so both
// sides' errors and the unobserved helper could be seen.
```

Command:

```text
cargo nextest run -p rumors --all-features --no-capture --no-fail-fast -E 'test(zz_witness_deep_fixtures_over_wire) | test(zz_witness_payload_drop_reenters_replica_lock)'   (run 5, first iteration; log run5_proxy_drop.log; diff witness_proxy10.diff)
cargo nextest run -p rumors --all-features --no-capture --no-fail-fast -E 'test(/proxy::tests::zz_witness_/)'   (run 6, second iteration; log run6_proxy_v2.log; diff witness_proxy10_v2.diff)
Mechanical check of the committed snapshots (no build): grep -ho "stream [0-9]* (height [0-9]*)" tests/snapshots/*.snap | sort | uniq -c
```

Output:

```text
Committed snapshots (grep over tests/snapshots/*.snap):
  22 stream 0 (height 31)
   2 stream 1 (height 29)
   7 stream 1 (height 30)

Run 6, diagnostic test (PASS, prints only):
WITNESS remote-proxy-tests-10 [content-addressed baseline, capacity 37]: frames per sent stream index: side A {0: 19, 1: 27}; side B {0: 43, 1: 5}; highest stream index opened = 1
WITNESS remote-proxy-tests-10 [content-addressed baseline, capacity 37]: side A Ok; side B Ok
WITNESS remote-proxy-tests-10 [content-addressed baseline, capacity 65536]: frames per sent stream index: side A {0: 19, 1: 27}; side B {0: 43, 1: 5}; highest stream index opened = 1
WITNESS remote-proxy-tests-10 [leaf_parent_dispute_pair, capacity 37]: frames per sent stream index: side A {1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 2, 8: 2, 9: 2, 10: 2, 11: 2, 12: 2, 13: 2, 14: 2, 15: 2, 16: 4}; side B {0: 2, 1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 2, 8: 2, 9: 2, 10: 2, 11: 2, 12: 2, 13: 2, 14: 2, 15: 2}; highest stream index opened = 16
WITNESS remote-proxy-tests-10 [leaf_parent_dispute_pair, capacity 37]: side A Err(Server(Stream(SupplyClosed { origin: Stream { speaker: Responder, stream: Stream(16) }, source: Some(Custom { kind: UnexpectedEof, error: "peer link is gone" }) }))); side B Err(Server(Decode(LeafOutsideScope { expected: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], actual: [84, 185, 194, 209, 144, 126, 178, 114, 158, 214, 57, 99, 201, 76, 40, 254, 51, 214, 176, 221, 37, 187, 136, 235, 47, 123, 221, 17, 135, 50, 207, 39] })))
(identical census and errors at capacity 65536)
WITNESS remote-proxy-tests-10 [leaf_parent_redaction_pair, capacity 37]: frames per sent stream index: side A {1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 2, 8: 2, 9: 2, 10: 2, 11: 2, 12: 2, 13: 2, 14: 2, 15: 2, 16: 2}; side B {0: 2, 1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 2, 8: 2, 9: 2, 10: 2, 11: 2, 12: 2, 13: 2, 14: 2, 15: 2, 16: 2}; highest stream index opened = 16
WITNESS remote-proxy-tests-10 [leaf_parent_redaction_pair, capacity 37]: side A Err(Server(Decode(LeafOutsideScope { expected: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], actual: [84, 185, 194, 209, 144, 126, 178, 114, 158, 214, 57, 99, 201, 76, 40, 254, 51, 214, 176, 221, 37, 187, 136, 235, 47, 123, 221, 17, 135, 50, 207, 39] }))); side B Ok
        PASS [   0.037s] (1/3) rumors tree::mirror::streaming::remote::proxy::tests::zz_witness_deep_fixtures_diagnostic

Run 6, the finding's construction (unobserved helper), both fixtures:
thread 'tree::mirror::streaming::remote::proxy::tests::zz_witness_leaf_parent_dispute_pair_over_wire' (241400215) panicked at src/tree/mirror/streaming/remote/proxy/tests.rs:108:27:
endpoint A should reconcile through its proxy: Server(Stream(SupplyClosed { origin: Stream { speaker: Responder, stream: Stream(16) }, source: Some(Custom { kind: UnexpectedEof, error: "peer link is gone" }) }))
        FAIL [   0.016s] (2/3) rumors tree::mirror::streaming::remote::proxy::tests::zz_witness_leaf_parent_dispute_pair_over_wire
        FAIL [   0.016s] (3/3) rumors tree::mirror::streaming::remote::proxy::tests::zz_witness_leaf_parent_redaction_pair_over_wire
     Summary [   0.069s] 3 tests run: 1 passed, 2 failed, 838 skipped
```

Explanation:

The gap is demonstrated, and the construction's closing branch is unavailable. Verified by running: (1) a content-addressed divergence of 24 leaves a side opens only stream indices 0 and 1 on either side (the frame census above), matching the committed public snapshots, which name only `stream 0` and `stream 1` (grep, mechanical); (2) both deep fixtures pass the in-process materialized oracle (`reconcile_locally` matched the fixture's stated union; the panic is at line 108 of the file, inside the wire helper, after that assertion) and open streams 0..=16 over the link, so the deep-stream tiers are reachable in principle; (3) over the wire they fail, at capacity 37 and 65536 alike, with the receiving side's primary error `Decode(LeafOutsideScope { expected: [0; 31], actual: [84, 185, ...] })` (the other side's `SupplyClosed`/`UnexpectedEof` is secondary, after the failed side drops its link). Assessed by reading, not run: `LeafOutsideScope` is raised in src/tree/mirror/streaming/remote/adapter/decode.rs:503-512 where the receiver derives the leaf's path with `Path::for_leaf(version)` and checks it against the reply scope, and the fixtures place leaves at `leaf_sibling_path` (src/tree/arb.rs:503-513: "A path all-zero except its final byte ... Real leaves are version-addressed ... these hand-picked paths let a test construct that shape deliberately"). So the fixtures violate the version-addressing invariant the wire relies on: no honest peer can hold them, the wire correctly refuses the leaf, and this is a fixture/model mismatch rather than the "wire-only defect" the construction's failure branch names. Consequence for the finding: the deep-stream tiers (indices >= 2), the leaf-tier decode pump on live traffic, and the terminal-stream reply grammar remain exercised over a link by no committed test, and closing the gap needs a different vehicle than `leaf_parent_*_pair` (an honestly content-addressed deep divergence found by hash-prefix search, reachable only to modest depth; or a codec/stream-tier harness that feeds deep-indexed frames directly). Bound: the observer used for the census is attached through the crate-private `Attachment::begin`/`Handshaking::observe`; the two failing construction tests use the unmodified `reconcile_symmetric_accepts` with no observer, so the observer is not the cause.

## async-hazards-3

Outcome: demonstrated

Test code:

```rust
// tests/zz_witness_drop_reentry.rs (new integration test binary; deleted after the run)

//! WITNESS (async-hazards-3): a payload destructor that reads its own replica
//! re-enters the watch channel's `RwLock` from the thread holding the commit's
//! write guard.
//!
//! The sequence: `send(value)`, drop every `Snapshot`, then `redact(&version)`.
//! `Batch::commit` runs `Tree::act` inside `watch::Sender::send_if_modified`
//! (write guard held); the commit point drops the pre-image, the displaced
//! payload becomes uniquely held, and its `Drop` calls `snapshot()`, which is
//! `inner.borrow()`: a read acquisition on the same lock, same thread.

use std::cell::RefCell;
use std::sync::mpsc;
use std::time::Duration;

use rumors::{Peer, Rumors};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Payload(u64);

thread_local! {
    /// The replica handle the destructor reads through, installed only once
    /// the message is in the tree (so the send-time round-trip copy's drop
    /// reads nothing).
    static HANDLE: RefCell<Option<Rumors<Payload>>> = const { RefCell::new(None) };
}

impl Drop for Payload {
    fn drop(&mut self) {
        // `try_with`: tolerate TLS teardown and the not-yet-armed case.
        let _ = HANDLE.try_with(|slot| {
            if let Some(rumors) = slot.borrow().as_ref() {
                eprintln!(
                    "WITNESS async-hazards-3: Payload::drop({}) calling snapshot()",
                    self.0
                );
                let snapshot = rumors.snapshot();
                eprintln!(
                    "WITNESS async-hazards-3: snapshot() returned inside the destructor, len={}",
                    snapshot.len()
                );
            }
        });
    }
}

/// WITNESS: `redact` of a payload whose destructor calls `snapshot()` on the
/// same replica must return; a hang (or a re-entrant-lock panic) is the
/// hazard the finding describes.
#[test]
fn zz_witness_payload_drop_reenters_replica_lock() {
    let (done_tx, done_rx) = mpsc::channel::<&'static str>();
    let worker = std::thread::spawn(move || {
        let rumors = Peer::<Payload>::seed().into_rumors();
        rumors.send(Payload(7)).expect("payload encodes");
        let version = {
            let snapshot = rumors.snapshot();
            let (version, _arc) = snapshot.iter().next().expect("one message is held");
            version.clone()
        };
        HANDLE.with(|slot| *slot.borrow_mut() = Some(rumors.clone()));
        done_tx.send("armed").unwrap();
        rumors.redact(&version);
        done_tx.send("redacted").unwrap();
    });
    assert_eq!(
        done_rx.recv_timeout(Duration::from_secs(10)).ok(),
        Some("armed"),
        "worker did not arm within 10s"
    );
    match done_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(message) => {
            eprintln!(
                "WITNESS async-hazards-3: worker reports `{message}`: no re-entry hazard observed"
            );
            worker.join().expect("worker exits cleanly");
        }
        Err(mpsc::RecvTimeoutError::Timeout) => panic!(
            "WITNESS async-hazards-3: redact() did not return within 10s: the destructor's \
             snapshot() blocked on the lock the commit holds"
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            let outcome = worker.join();
            panic!("WITNESS async-hazards-3: worker died before reporting: {outcome:?}");
        }
    }
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-capture --no-fail-fast -E 'test(zz_witness_deep_fixtures_over_wire) | test(zz_witness_payload_drop_reenters_replica_lock)'   (run 5; log run5_proxy_drop.log; source saved as zz_witness_drop_reentry.rs)
```

Output:

```text
WITNESS async-hazards-3: Payload::drop(7) calling snapshot()
thread 'zz_witness_payload_drop_reenters_replica_lock' (241377500) panicked at tests/zz_witness_drop_reentry.rs:78:49:
WITNESS async-hazards-3: redact() did not return within 10s: the destructor's snapshot() blocked on the lock the commit holds
test zz_witness_payload_drop_reenters_replica_lock ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.01s

        FAIL [  10.017s] (2/2) rumors::zz_witness_drop_reentry zz_witness_payload_drop_reenters_replica_lock

(The line `WITNESS async-hazards-3: snapshot() returned inside the destructor, ...` never appeared.)
```

Explanation:

Verified by running, through the public API only (`Peer::seed`, `into_rumors`, `send`, `snapshot`, `iter`, `redact`). The destructor was entered from inside `redact` (the `calling snapshot()` line printed), `snapshot()` never returned, and `redact` had not returned after 10 s: the read acquisition in `Peer::snapshot` (`self.inner.borrow()`, src/peer.rs:678-680) blocked behind the write guard `Batch::commit`'s `send_if_modified` holds around `Tree::act` and its `drop(pre_image)` (src/batch.rs:132-144, src/tree.rs:537-539). The send-time round-trip copy's drop did not interfere because the handle is installed only after the message is in the tree. The observed form is a hang, not a panic, consistent with a blocking (non-diagnosing) re-entrant read on the watch channel's std `RwLock` (assessed, not verified: tokio is built here without the `parking_lot` feature, and std's lock does not detect re-entrancy). The test's own 10 s timeout is what turned the hang into a failure; without it the test would hang until nextest's 180 s terminate budget.

## mirror-common-3

Outcome: demonstrated

Test code:

```rust
// 1. The mutation, src/tree/mirror/cbor.rs `extension_len` (temporary, restored). Lines 277-278:
//    before:
        28..=30 => Err(HeadReadError::Malformed(HeadError::Reserved)),
        _ => Err(HeadReadError::Malformed(HeadError::Indefinite)),
//    after:
        28..=30 => Err(HeadReadError::Malformed(HeadError::Indefinite)),
        _ => Err(HeadReadError::Malformed(HeadError::Reserved)),

// 2. A witness test appended to src/tree/mirror/cbor/tests.rs (temporary, restored), to show
//    what a test that does catch the swap looks like:

/// WITNESS (mirror-common-3): the async reader's own additional-information
/// table classifies reserved (28..=30) and indefinite (31) initial bytes
/// exactly as the slice reader does; a swap of `extension_len`'s two error
/// arms must fail this.
#[test]
fn zz_witness_async_reader_classifies_reserved_and_indefinite() {
    for major in 0u8..8 {
        for (info, expected) in [
            (28u8, HeadError::Reserved),
            (29, HeadError::Reserved),
            (30, HeadError::Reserved),
            (31, HeadError::Indefinite),
        ] {
            let bytes = [(major << 5) | info];
            let result = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("runtime builds")
                .block_on(async {
                    let mut read = bytes.as_slice();
                    read_head_async(&mut read).await
                });
            match result {
                Err(HeadReadError::Malformed(actual)) => assert_eq!(
                    actual, expected,
                    "initial byte {:#04x}: async reader classified {actual:?}, slice reader says {expected:?}",
                    bytes[0]
                ),
                other => panic!(
                    "initial byte {:#04x}: expected Malformed({expected:?}), got {other:?}",
                    bytes[0]
                ),
            }
        }
    }
}
```

Command:

```text
cargo nextest list -p rumors --all-features -E 'test(/tree::mirror::cbor::/) | test(/mirror::streaming::remote::codec::/) | test(/proxy::tests::malformed/) | binary(gossip_snapshot) | binary(decode_alloc)'   (listing only, 112 tests; log run4_list.log)
cargo nextest run -p rumors --all-features --no-fail-fast -E 'test(/tree::mirror::cbor::/) | test(/mirror::streaming::remote::codec::/) | test(/mirror::streaming::remote::streams::/) | test(/proxy::tests::malformed/) | test(/tree::mirror::party::/) | binary(gossip_snapshot) | binary(decode_alloc)'   (run 4; log run4_cbor_swap.log; diff witness_mirror_common3.diff, swap script swap_cbor.py)
```

Output:

```text
    Starting 136 tests across 60 binaries (703 tests skipped)
        FAIL [   0.017s] ( 11/136) rumors tree::mirror::cbor::tests::zz_witness_async_reader_classifies_reserved_and_indefinite
    thread 'tree::mirror::cbor::tests::zz_witness_async_reader_classifies_reserved_and_indefinite' (241362622) panicked at src/tree/mirror/cbor/tests.rs:139:58:
    assertion `left == right` failed: initial byte 0x1c: async reader classified Indefinite, slice reader says Reserved
      left: Indefinite
     right: Reserved
     Summary [   8.722s] 136 tests run: 135 passed, 1 failed, 703 skipped
```

Explanation:

Demonstrated within the stated bound. With `extension_len`'s two error arms swapped, all 135 existing tests in the selected set passed; only the added witness failed. Not run (by instruction): the whole suite. The selection was chosen from the callers of the mutated function: `read_head_async` and `extension_len` are reached in production from src/tree/mirror/party.rs:151, src/tree/mirror/streaming/remote/streams.rs:761, remote/codec/greeting.rs:240 and :254, remote/codec/decode/async_io.rs:302 and :503, and (test-only `read_head_io`) remote/codec/decode.rs:87 and :199 (grep, mechanical), so the filter covered the `cbor`, every `remote::codec` submodule, `remote::streams`, `party`, and `proxy::tests::malformed` unit modules plus the `gossip_snapshot` and `decode_alloc` integration binaries. Mechanically (grep), the only tests naming `HeadError::Reserved`/`Indefinite` are src/tree/mirror/cbor/tests.rs:49-52, which call the slice reader `read_head` (its own copy of the table); the only test callers of `read_head_async` are cbor/tests.rs:100 and :111, on canonical heads and the empty stream; the indefinite-length bytes in remote/codec/decode/tests.rs:521-548 go through ciborium payload/atom decoding and expect success, and capture/tests.rs:192 uses 0xff as byte-string content, not a head. The `1 + (input.len() - rest.len() - 1)` reading at cbor.rs:191 is assessed by reading only (it is an identity; no test constructed).

## Cleanup status (second pass)

Worktree /Users/oxide/src/rumors/.claude/worktrees/wf_8986b141-8e4-2 at HEAD 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (verified at start). Every temporary edit was restored with `git restore --source=HEAD --staged --worktree <path>` (tests/bookmark_causality.rs, src/conformance/backend.rs, src/tree/mirror/cbor.rs, src/tree/mirror/cbor/tests.rs, src/tree/mirror/streaming/remote/proxy/tests.rs) and the one created file tests/zz_witness_drop_reentry.rs was deleted. Build directory resolved from inside the worktree before removal: /Volumes/forge/build/6a/384e0512798256; it and the local target/ were deleted and confirmed absent (`ls` reports "No such file or directory" for both). Final `git status --porcelain` output: (empty).
