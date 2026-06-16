//! Party linearity and disjointness under arbitrary disruption.
//!
//! Two simulations, one engine of invariants (`common::sim`):
//!
//! - **Intra-process**: a fleet of peers on one multi-thread runtime,
//!   every gossip session, bootstrap, send, and redact spawned at once,
//!   over in-memory wires that may be severed at arbitrary byte offsets.
//! - **Inter-process**: peers split across genuinely separate OS
//!   processes — the test binary re-executes itself as each child — over
//!   real TCP sockets with the same fault injection on the child side,
//!   children retiring home at the end so the id-space can be audited.
//!
//! Both assert the same global properties, stated on each test below. Task
//! and process interleavings are nondeterministic, so a counterexample may
//! not replay byte-for-byte; the invariants quantify over *all*
//! interleavings, so any failure is a genuine one.

mod common;

use std::collections::BTreeSet;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use proptest::prelude::*;
use rumors::{Error, Peer, Retire, Rumors};
use tokio::net::{TcpListener, TcpStream};

use crate::common::fault::{self, FaultPlan};
use crate::common::oracle::readout;
use crate::common::sim::{
    arb_fault, arb_plan, assert_converged, assert_honest_error, assert_honest_gossip,
    assert_party_invariants, probe_disjointness, quiesce, run_plan,
};
use crate::common::wire::bootstrap_fork_async;

/// A fresh multi-thread runtime per simulation, so tasks interleave with
/// real parallelism rather than cooperative scheduling alone.
fn mt_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build multi-thread runtime")
}

// ---- intra-process ----------------------------------------------------------

proptest! {
    /// Under arbitrary concurrent gossip — overlapping sessions through
    /// cloned [`Rumors`] handles, concurrent sends and redactions,
    /// bootstraps served mid-chaos against the same shared state, and
    /// retirements, over wires cut at arbitrary byte offsets — the global
    /// party invariants hold:
    ///
    /// 1. every session failure is an injected I/O fault, never
    ///    `PartyOverlap` or a protocol violation;
    /// 2. at every probed instant the live parties are pairwise disjoint;
    /// 3. after a clean heal, all survivors converge to identical content;
    /// 4. when no hand-off was lost in flight, the surviving parties
    ///    fold-join back to exactly `Party::seed()` — the id-space is
    ///    conserved with no duplication and no leak.
    #[test]
    fn disrupted_concurrent_gossip_upholds_party_invariants(plan in arb_plan()) {
        mt_runtime().block_on(async {
            let outcome = run_plan(plan).await;
            quiesce(&outcome.peers).await;
            assert_converged(&outcome.peers);
            assert_party_invariants(&outcome.peers, outcome.possible_losses);
        });
    }
}

// ---- inter-process ----------------------------------------------------------

/// Environment protocol between the parent test and its child processes.
/// The presence of `CHILD_ADDR` is what turns the re-executed test binary
/// into a child peer.
const CHILD_ADDR: &str = "RUMORS_SIM_CHILD_ADDR";
const CHILD_INDEX: &str = "RUMORS_SIM_CHILD_INDEX";
const CHILD_SENDS: &str = "RUMORS_SIM_CHILD_SENDS";
const CHILD_BOOT: &str = "RUMORS_SIM_CHILD_BOOT";
const CHILD_SESSIONS: &str = "RUMORS_SIM_CHILD_SESSIONS";
const CHILD_RETIRE: &str = "RUMORS_SIM_CHILD_RETIRE";

/// Child exit codes: the loss-accounting back-channel. Anything else
/// (including a panic's 101) fails the parent test.
const EXIT_CLEAN: i32 = 0;
/// Retired cleanly, but an earlier faulty bootstrap attempt failed: the
/// fork served for it may be orphaned (possible loss).
const EXIT_BOOT_LOSS: i32 = 2;
/// The final retirement ended [`Retire::Uncertain`]: the party may be in
/// limbo (possible loss).
const EXIT_UNCERTAIN: i32 = 3;
/// A state the protocol promises is unreachable for this topology.
const EXIT_ANOMALY: i32 = 4;

/// Wall-clock bound on each child process.
const CHILD_DEADLINE: Duration = Duration::from_secs(60);

/// The value of child `index`'s `s`-th send: distinct per child and per
/// send, so the parent can assert that a cleanly-retired child's content
/// all made it home.
fn child_value(index: usize, s: usize) -> u64 {
    (index as u64 + 1) * 1_000_000 + s as u64
}

fn encode_cut(cut: Option<usize>) -> String {
    cut.map_or_else(|| "-".to_owned(), |n| n.to_string())
}

fn decode_cut(s: &str) -> Option<usize> {
    (s != "-").then(|| s.parse().expect("malformed cut budget"))
}

fn encode_fault(fault: &FaultPlan) -> String {
    format!(
        "{}:{}",
        encode_cut(fault.write_cut),
        encode_cut(fault.read_cut)
    )
}

fn decode_fault(s: &str) -> FaultPlan {
    let (write, read) = s.split_once(':').expect("malformed fault plan");
    FaultPlan {
        write_cut: decode_cut(write),
        read_cut: decode_cut(read),
    }
}

/// One child process's script: how many sends, the fault plan for an
/// initial (deliberately lossy) bootstrap attempt, per-session fault
/// plans, and the fault plan for its final retirement.
#[derive(Debug, Clone)]
struct ChildPlan {
    n_sends: usize,
    boot: FaultPlan,
    sessions: Vec<FaultPlan>,
    retire: FaultPlan,
}

#[derive(Debug, Clone)]
struct ProcPlan {
    n_parent_peers: usize,
    seed_messages: Vec<u64>,
    children: Vec<ChildPlan>,
}

fn arb_child_plan(faults: bool) -> impl Strategy<Value = ChildPlan> {
    (
        0usize..6,
        arb_fault(faults),
        prop::collection::vec(arb_fault(faults), 1..4),
        arb_fault(faults),
    )
        .prop_map(|(n_sends, boot, sessions, retire)| ChildPlan {
            n_sends,
            boot,
            sessions,
            retire,
        })
}

/// As in `arb_plan`, the leading `bool` turns fault injection off for half
/// of all plans, so the sharp seed-reconstitution check runs often.
fn arb_proc_plan() -> impl Strategy<Value = ProcPlan> {
    any::<bool>().prop_flat_map(|faults| {
        (
            1usize..=2,
            prop::collection::vec(any::<u64>(), 0..4),
            prop::collection::vec(arb_child_plan(faults), 1..=3),
        )
            .prop_map(|(n_parent_peers, seed_messages, children)| ProcPlan {
                n_parent_peers,
                seed_messages,
                children,
            })
    })
}

/// Kill (and reap) a child process if the parent unwinds before it exits.
struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    /// The same four invariants as the intra-process simulation, with the
    /// fleet split across OS processes gossiping over real TCP sockets
    /// severed at arbitrary byte offsets: child processes bootstrap from
    /// the parent, gossip concurrently with it (and with its own
    /// in-process sessions), then retire home. Cleanly-retired children
    /// must additionally leave every one of their sends in the parent's
    /// converged content, and a loss-free run must fold the parent's
    /// surviving parties back to exactly `Party::seed()`.
    #[test]
    fn inter_process_disruption_upholds_party_invariants(plan in arb_proc_plan()) {
        mt_runtime().block_on(run_proc_plan(plan));
    }
}

async fn run_proc_plan(plan: ProcPlan) {
    // Parent fleet: the seed and its clean forks, as shared-state handles
    // so inbound sessions can overlap arbitrarily.
    let seed = Peer::<u64>::seed().into_rumors();
    {
        let mut batch = seed.batch();
        for &v in &plan.seed_messages {
            batch.send(v);
        }
    }
    let mut casts: Vec<Rumors<u64>> = vec![seed];
    for _ in 1..plan.n_parent_peers {
        let fork = bootstrap_fork_async(&casts[0]).await;
        casts.push(fork);
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind simulation listener");
    let addr = listener.local_addr().expect("listener address");

    // Serve every inbound connection with plain gossip, which transparently
    // handles a bootstrapping or retiring counterparty. Errors are expected
    // (the children sever wires); *dishonest* errors are recorded for the
    // final assertion rather than panicking inside a detached task, and
    // every error conservatively counts as a possible in-flight loss (a
    // dying session may have been a bootstrap holding a donated fork).
    let serve_errors = Arc::new(AtomicUsize::new(0));
    let dishonest = Arc::new(Mutex::new(Vec::<String>::new()));
    let accept = {
        let casts = casts.clone();
        let serve_errors = Arc::clone(&serve_errors);
        let dishonest = Arc::clone(&dishonest);
        tokio::spawn(async move {
            let mut sessions = tokio::task::JoinSet::new();
            let mut next = 0usize;
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    break;
                };
                let handle = casts[next % casts.len()].clone();
                next += 1;
                let serve_errors = Arc::clone(&serve_errors);
                let dishonest = Arc::clone(&dishonest);
                sessions.spawn(async move {
                    let (mut r, mut w) = tokio::io::split(socket);
                    if let Err(e) = handle.gossip(&mut r, &mut w).await {
                        serve_errors.fetch_add(1, Ordering::Relaxed);
                        if !matches!(e, Error::Io(_)) {
                            dishonest
                                .lock()
                                .expect("dishonest log")
                                .push(format!("{e:?}"));
                        }
                    }
                });
            }
        })
    };

    // Probe parent-side party disjointness while the children hammer it.
    let done = Arc::new(AtomicBool::new(false));
    let prober = tokio::spawn(probe_disjointness(casts.clone(), Arc::clone(&done)));

    // Spawn the children: this same test binary, re-executed straight into
    // `sim_child` with its script in the environment.
    let exe = std::env::current_exe().expect("current test binary");
    let mut children = Vec::new();
    for (index, child) in plan.children.iter().enumerate() {
        let sessions: Vec<String> = child.sessions.iter().map(encode_fault).collect();
        let process = std::process::Command::new(&exe)
            .args(["--exact", "sim_child", "--ignored"])
            .env(CHILD_ADDR, addr.to_string())
            .env(CHILD_INDEX, index.to_string())
            .env(CHILD_SENDS, child.n_sends.to_string())
            .env(CHILD_BOOT, encode_fault(&child.boot))
            .env(CHILD_SESSIONS, sessions.join(","))
            .env(CHILD_RETIRE, encode_fault(&child.retire))
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn child process");
        children.push(KillOnDrop(process));
    }

    // Reap the children, folding their exit codes into the loss accounting.
    let deadline = tokio::time::Instant::now() + CHILD_DEADLINE;
    let mut possible_losses = 0usize;
    let mut clean_children = vec![false; plan.children.len()];
    for (index, child) in children.iter_mut().enumerate() {
        let status = loop {
            if let Some(status) = child.0.try_wait().expect("poll child") {
                break status;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "child {index} did not finish within {CHILD_DEADLINE:?}"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        };
        match status.code() {
            Some(EXIT_CLEAN) => clean_children[index] = true,
            Some(EXIT_BOOT_LOSS) | Some(EXIT_UNCERTAIN) => possible_losses += 1,
            other => panic!(
                "child {index} exited abnormally ({other:?}): an invariant \
                 violation or panic in the child process"
            ),
        }
    }
    possible_losses += serve_errors.load(Ordering::Relaxed);

    // Wind down: stop the prober and the accept loop (dropping its
    // `JoinSet` aborts any straggling serve task), then reclaim the
    // parent's `Peer`s — `try_into_peer` resolves once every serving clone
    // is gone, so this is the synchronization point proving quiescence.
    // The heal phase below runs on the data plane, so each reclaimed
    // `Peer` converts straight back out.
    done.store(true, Ordering::Release);
    prober.await.expect("prober task");
    accept.abort();
    let _ = accept.await;
    let mut survivors = Vec::new();
    for cast in casts {
        survivors.push(
            cast.try_into_peer()
                .await
                .expect("all serving clones dropped")
                .into_rumors(),
        );
    }

    assert!(
        dishonest.lock().expect("dishonest log").is_empty(),
        "serving sessions surfaced non-fault errors: {:?}",
        dishonest.lock().expect("dishonest log")
    );

    quiesce(&survivors).await;
    assert_converged(&survivors);

    // Every cleanly-retired child's sends must have survived into the
    // parent's converged content: its final retirement reconciled before
    // the party hand-off, so nothing it published may be lost.
    let live: BTreeSet<u64> = readout(&survivors[0].snapshot()).into_values().collect();
    for (index, child) in plan.children.iter().enumerate() {
        if clean_children[index] {
            for s in 0..child.n_sends {
                assert!(
                    live.contains(&child_value(index, s)),
                    "send {s} of cleanly-retired child {index} was lost"
                );
            }
        }
    }

    assert_party_invariants(&survivors, possible_losses);
}

// ---- the child process ------------------------------------------------------

/// Child-process entry point for `inter_process_disruption_*`: **not a
/// test**. The parent re-executes this binary with `--exact sim_child
/// --ignored` and the script in the environment; without `CHILD_ADDR` set
/// (e.g. under `--run-ignored`) it is a no-op. Outcomes travel back as
/// exit codes (`EXIT_*`); invariant violations panic, which the parent
/// sees as an abnormal exit.
#[test]
#[ignore = "child-process entry point for the inter-process simulation, not a test"]
fn sim_child() {
    let Ok(addr) = std::env::var(CHILD_ADDR) else {
        return;
    };
    let code = mt_runtime().block_on(child_main(addr));
    if code != EXIT_CLEAN {
        std::process::exit(code);
    }
}

async fn child_main(addr: String) -> i32 {
    let env = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));
    let index: usize = env(CHILD_INDEX).parse().expect("child index");
    let n_sends: usize = env(CHILD_SENDS).parse().expect("send count");
    let boot = decode_fault(&env(CHILD_BOOT));
    let sessions: Vec<FaultPlan> = {
        let raw = env(CHILD_SESSIONS);
        raw.split(',')
            .filter(|s| !s.is_empty())
            .map(decode_fault)
            .collect()
    };
    let retire_fault = decode_fault(&env(CHILD_RETIRE));

    // Join the universe. A faulty plan first attempts a bootstrap over a
    // cut wire: on failure the fork served for it may be orphaned, which
    // the parent must hear about (the exit code), and the child joins for
    // real over a clean connection.
    let mut had_boot_loss = false;
    let mut known: Option<Peer<u64>> = None;
    if !boot.is_clean() {
        let socket = TcpStream::connect(&addr)
            .await
            .expect("connect for faulty bootstrap");
        let (mut r, mut w) = fault::faulty(socket, boot);
        match Peer::<u64>::bootstrap(&mut r, &mut w).await {
            Ok(Some(k)) => known = Some(k),
            Ok(None) => panic!("the parent never bootstraps"),
            Err(e) => {
                assert_honest_error(&e);
                had_boot_loss = true;
            }
        }
    }
    let known = match known {
        Some(k) => k,
        None => {
            let socket = TcpStream::connect(&addr)
                .await
                .expect("connect for bootstrap");
            let (mut r, mut w) = tokio::io::split(socket);
            Peer::<u64>::bootstrap(&mut r, &mut w)
                .await
                .expect("clean bootstrap")
                .expect("the parent serves every bootstrap")
        }
    };

    // Chaos: local sends concurrent with possibly-severed gossip sessions
    // back to the parent.
    let cast = known.into_rumors();
    let sender = {
        let handle = cast.clone();
        tokio::spawn(async move {
            for s in 0..n_sends {
                handle.send(child_value(index, s));
                tokio::task::yield_now().await;
            }
        })
    };
    for fault in sessions {
        let socket = TcpStream::connect(&addr)
            .await
            .expect("connect for session");
        let (mut r, mut w) = fault::faulty(socket, fault);
        let handle = cast.clone();
        assert_honest_gossip(&handle.gossip(&mut r, &mut w).await);
    }
    sender.await.expect("sender task");

    // One clean session so everything this child published is home even
    // before the retirement reconciles.
    {
        let socket = TcpStream::connect(&addr)
            .await
            .expect("connect for final gossip");
        let (mut r, mut w) = tokio::io::split(socket);
        cast.gossip(&mut r, &mut w)
            .await
            .expect("clean final gossip");
    }

    // Retire home, possibly through a cut wire; a recovered retiree gets
    // one clean retry. Outcomes map to the exit-code protocol.
    let mut known = cast
        .try_into_peer()
        .await
        .expect("sender finished; sole handle");
    let mut fault = retire_fault;
    for _attempt in 0..2 {
        let socket = TcpStream::connect(&addr).await.expect("connect for retire");
        let (mut r, mut w) = fault::faulty(socket, fault);
        match known.retire(&mut r, &mut w).await {
            Retire::Retired => {
                return if had_boot_loss {
                    EXIT_BOOT_LOSS
                } else {
                    EXIT_CLEAN
                };
            }
            Retire::Recovered {
                peer: recovered,
                error,
            } => {
                assert_honest_error(&error);
                known = recovered;
                fault = FaultPlan::NONE;
            }
            Retire::Uncertain { error } => {
                assert_honest_error(&error);
                return EXIT_UNCERTAIN;
            }
            Retire::Declined { .. } => return EXIT_ANOMALY,
        }
    }
    // A clean retry can only end `Retired`; reaching here is an anomaly.
    EXIT_ANOMALY
}
