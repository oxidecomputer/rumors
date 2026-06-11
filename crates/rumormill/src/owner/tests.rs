//! Integration tests for the [`Owner`] actor: two real owners wired over
//! in-memory duplex pipes (the same gossip protocol the iroh transport
//! carries, minus the network), driven through the same [`Command`] channel
//! production uses.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rumors::Error;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use super::*;

const ALICE: PeerId = [0xaa; 32];
const BOB: PeerId = [0xbb; 32];

/// The test epoch: an arbitrary fixed wall-clock origin.
const T0: Millis = 1_000_000;

/// One spawned owner and its handles.
struct Node {
    cmd: mpsc::Sender<Command>,
    view: watch::Receiver<Arc<View>>,
    /// Warp this to move the node's wall clock.
    clock: Arc<AtomicU64>,
    #[allow(dead_code)] // held for lifetime; only the shutdown test joins it
    task: JoinHandle<(Known<Entry>, Vec<PeerId>)>,
}

fn spawn_node(known: Known<Entry>, me: PeerId, name: &str) -> Node {
    let clock = Arc::new(AtomicU64::new(T0));
    let tick = clock.clone();
    let (owner, view) = Owner::new(
        known,
        me,
        hex::encode(me),
        name.to_string(),
        Clock::from_fn(move || tick.load(Ordering::Relaxed)),
    );
    let (cmd, rx) = mpsc::channel(64);
    let task = tokio::spawn(owner.run(rx));
    Node {
        cmd,
        view,
        clock,
        task,
    }
}

/// Mint a second originating peer in `a`'s universe over a duplex pipe.
async fn bootstrap_from(mut a: Known<Entry>) -> (Known<Entry>, Known<Entry>) {
    let (sa, sb) = tokio::io::duplex(64 * 1024);
    let (mut ar, mut aw) = tokio::io::split(sa);
    let (mut br, mut bw) = tokio::io::split(sb);
    let (served, b) = tokio::join!(
        a.gossip(&mut ar, &mut aw),
        Known::<Entry>::bootstrap(&mut br, &mut bw),
    );
    served.unwrap();
    (a, b.unwrap().expect("peer served the bootstrap"))
}

/// Ask an owner for a session handle, exactly as a connection task would.
async fn handle(cmd: &mpsc::Sender<Command>) -> Broadcast<Entry> {
    let (reply, rx) = oneshot::channel();
    cmd.send(Command::Handle { reply }).await.unwrap();
    rx.await.unwrap()
}

/// One full gossip session between two owners: take a handle from each,
/// reconcile over a duplex pipe, and report the outcome — the report is
/// what triggers each owner's loss sweep, exactly as in production.
async fn gossip_pair(a: &Node, b: &Node) {
    let mut ha = handle(&a.cmd).await;
    let mut hb = handle(&b.cmd).await;
    let (sa, sb) = tokio::io::duplex(64 * 1024);
    let (mut ar, mut aw) = tokio::io::split(sa);
    let (mut br, mut bw) = tokio::io::split(sb);
    let (ra, rb) = tokio::join!(ha.gossip(&mut ar, &mut aw), hb.gossip(&mut br, &mut bw),);
    ra.unwrap();
    rb.unwrap();
    a.cmd
        .send(Command::SessionOutcome { ok: true })
        .await
        .unwrap();
    b.cmd
        .send(Command::SessionOutcome { ok: true })
        .await
        .unwrap();
}

/// Wait (bounded) until the node's published view satisfies `pred`.
async fn wait_view(node: &mut Node, pred: impl Fn(&View) -> bool) -> Arc<View> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            {
                let view = node.view.borrow_and_update();
                if pred(&view) {
                    return view.clone();
                }
            }
            node.view.changed().await.expect("owner alive");
        }
    })
    .await
    .expect("view never satisfied the predicate")
}

fn channel_bodies(view: &View, channel: &str) -> Vec<String> {
    view.channels
        .iter()
        .find(|c| c.name == channel)
        .map(|c| c.messages.iter().map(|m| m.body.clone()).collect())
        .unwrap_or_default()
}

fn has_chat(view: &View, channel: &str, body: &str) -> bool {
    channel_bodies(view, channel).iter().any(|b| b == body)
}

/// Chat propagates in both directions through the snapshot → gossip →
/// join-back cycle, and each side displays it causally after its own
/// earlier traffic.
#[tokio::test(flavor = "current_thread")]
async fn chat_propagates_both_ways() {
    let (ka, kb) = bootstrap_from(Known::seed()).await;
    let mut a = spawn_node(ka, ALICE, "alice");
    let mut b = spawn_node(kb, BOB, "bob");

    a.cmd
        .send(Command::SendChat {
            channel: HOME_CHANNEL.into(),
            body: "from alice".into(),
        })
        .await
        .unwrap();
    b.cmd
        .send(Command::SendChat {
            channel: HOME_CHANNEL.into(),
            body: "from bob".into(),
        })
        .await
        .unwrap();
    wait_view(&mut a, |v| has_chat(v, HOME_CHANNEL, "from alice")).await;
    wait_view(&mut b, |v| has_chat(v, HOME_CHANNEL, "from bob")).await;

    gossip_pair(&a, &b).await;
    wait_view(&mut a, |v| has_chat(v, HOME_CHANNEL, "from bob")).await;
    wait_view(&mut b, |v| has_chat(v, HOME_CHANNEL, "from alice")).await;
}

/// An expiry on one peer is a redaction, and the other peer's screen loses
/// the message after one round of gossip — through the removal diff, since
/// joins observe gains only.
#[tokio::test(flavor = "current_thread")]
async fn expiry_redaction_reaches_the_peer() {
    let (ka, kb) = bootstrap_from(Known::seed()).await;
    let mut a = spawn_node(ka, ALICE, "alice");
    let mut b = spawn_node(kb, BOB, "bob");

    a.cmd
        .send(Command::SendChat {
            channel: HOME_CHANNEL.into(),
            body: "ephemeral".into(),
        })
        .await
        .unwrap();
    wait_view(&mut a, |v| has_chat(v, HOME_CHANNEL, "ephemeral")).await;
    gossip_pair(&a, &b).await;
    let view = wait_view(&mut b, |v| has_chat(v, HOME_CHANNEL, "ephemeral")).await;

    // Fire the expiry on A as the wheel would.
    let key = view
        .channels
        .iter()
        .find(|c| c.name == HOME_CHANNEL)
        .and_then(|c| c.messages.iter().find(|m| m.body == "ephemeral"))
        .map(|m| m.key)
        .unwrap();
    a.cmd.send(Command::ExpiryDue { key }).await.unwrap();
    wait_view(&mut a, |v| !has_chat(v, HOME_CHANNEL, "ephemeral")).await;

    gossip_pair(&a, &b).await;
    wait_view(&mut b, |v| !has_chat(v, HOME_CHANNEL, "ephemeral")).await;
}

/// Heartbeats populate the roster across gossip; a peer that stops beating
/// is evicted by the staleness sweep; a revived peer reappears with a fresh
/// beat.
#[tokio::test(flavor = "current_thread")]
async fn staleness_evicts_and_revival_restores() {
    let (ka, kb) = bootstrap_from(Known::seed()).await;
    let mut a = spawn_node(ka, ALICE, "alice");
    let mut b = spawn_node(kb, BOB, "bob");

    // First heartbeats fire on spawn; after gossip, each sees the other.
    gossip_pair(&a, &b).await;
    wait_view(&mut a, |v| v.roster.iter().any(|p| p.peer == BOB)).await;
    wait_view(&mut b, |v| v.roster.iter().any(|p| p.peer == ALICE)).await;

    // Bob goes silent; alice's clock passes the staleness threshold and her
    // next sweep evicts him.
    let stale = timers::PRESENCE_STALE.as_millis() as u64;
    a.clock.store(T0 + stale + 1, Ordering::Relaxed);
    a.cmd.send(Command::HeartbeatTick).await.unwrap();
    wait_view(&mut a, |v| !v.roster.iter().any(|p| p.peer == BOB)).await;

    // Bob revives: a fresh beat (fresh key) survives the earlier redaction
    // and the roster heals after gossip.
    b.clock.store(T0 + stale + 1, Ordering::Relaxed);
    b.cmd.send(Command::HeartbeatTick).await.unwrap();
    gossip_pair(&a, &b).await;
    wait_view(&mut a, |v| v.roster.iter().any(|p| p.peer == BOB)).await;
}

/// Two independently seeded universes meet: gossip surfaces a symmetric
/// `NetworkMismatch`, exactly one side wins the merge rule, the loser
/// bootstraps and resets — and its old content is gone while the winner's
/// survives. A late `Reset` whose verdict was computed against the
/// already-abandoned universe is declined.
#[tokio::test(flavor = "current_thread")]
async fn network_merge_resets_the_loser() {
    let mut a = spawn_node(Known::seed(), ALICE, "alice");
    let mut b = spawn_node(Known::seed(), BOB, "bob");
    a.cmd
        .send(Command::SendChat {
            channel: HOME_CHANNEL.into(),
            body: "from alice".into(),
        })
        .await
        .unwrap();
    b.cmd
        .send(Command::SendChat {
            channel: HOME_CHANNEL.into(),
            body: "from bob".into(),
        })
        .await
        .unwrap();
    wait_view(&mut a, |v| has_chat(v, HOME_CHANNEL, "from alice")).await;
    wait_view(&mut b, |v| has_chat(v, HOME_CHANNEL, "from bob")).await;

    // The contact attempt: both sides gossip and both fail symmetrically
    // with the other's network and event floor.
    let mut handle_a = handle(&a.cmd).await;
    let mut handle_b = handle(&b.cmd).await;
    let ours_a = (handle_a.latest().min_ticks(), handle_a.network());
    let ours_b = (handle_b.latest().min_ticks(), handle_b.network());
    let (sa, sb) = tokio::io::duplex(64 * 1024);
    let (mut ar, mut aw) = tokio::io::split(sa);
    let (mut br, mut bw) = tokio::io::split(sb);
    let (ra, rb) = tokio::join!(
        handle_a.gossip(&mut ar, &mut aw),
        handle_b.gossip(&mut br, &mut bw),
    );
    let theirs_a = match ra {
        Err(Error::NetworkMismatch {
            remote_network,
            remote_min_events,
        }) => (remote_min_events, remote_network),
        other => panic!("expected NetworkMismatch, got {other:?}"),
    };
    let theirs_b = match rb {
        Err(Error::NetworkMismatch {
            remote_network,
            remote_min_events,
        }) => (remote_min_events, remote_network),
        other => panic!("expected NetworkMismatch, got {other:?}"),
    };
    // Both sides see the same pair and the rule picks exactly one winner.
    assert_eq!(theirs_a, ours_b);
    assert_eq!(theirs_b, ours_a);
    assert_ne!(ours_a > theirs_a, ours_b > theirs_b);

    let (winner_handle, loser, abandoned, winner_body, loser_body) = if ours_a > theirs_a {
        (handle_a, &mut b, ours_b.1, "from alice", "from bob")
    } else {
        (handle_b, &mut a, ours_a.1, "from bob", "from alice")
    };

    // The loser bootstraps from the winner over a fresh pipe and resets.
    // (The owner's fresh observer replays the adopted content; no
    // observation list rides the command.)
    let mut serve = winner_handle;
    let (sw, sl) = tokio::io::duplex(64 * 1024);
    let (mut wr, mut ww) = tokio::io::split(sw);
    let (mut lr, mut lw) = tokio::io::split(sl);
    let (served, fresh) = tokio::join!(
        serve.gossip(&mut wr, &mut ww),
        Known::<Entry>::bootstrap(&mut lr, &mut lw),
    );
    served.unwrap();
    let fresh = fresh.unwrap().expect("winner served the bootstrap");
    let expected_network = network_short(fresh.network());
    loser
        .cmd
        .send(Command::Reset {
            known: Box::new(fresh),
            abandoned,
        })
        .await
        .unwrap();

    let view = wait_view(loser, |v| {
        v.network == expected_network && has_chat(v, HOME_CHANNEL, winner_body)
    })
    .await;
    // Total reset: the loser's pre-merge content is gone, and the merge is
    // surfaced.
    assert!(!has_chat(&view, HOME_CHANNEL, loser_body));
    assert_eq!(view.stats.merges, 1);
    assert!(view.merged_notice.is_some());

    // A late Reset raced from a session in the *abandoned* universe is
    // declined — we already left it, so its verdict no longer applies and a
    // future session would re-arbitrate. The view keeps the adopted network.
    loser
        .cmd
        .send(Command::Reset {
            known: Box::new(Known::seed()),
            abandoned,
        })
        .await
        .unwrap();
    loser.cmd.send(Command::HeartbeatTick).await.unwrap();
    let view = wait_view(loser, |v| {
        v.stats.merges == 1 && v.network == expected_network
    })
    .await;
    assert!(has_chat(&view, HOME_CHANNEL, winner_body));
}

/// The rumor set does not grow without bound: heartbeats supersede (and
/// redact) their predecessors, so repeated ticks leave the live count
/// unchanged — channel creation entries are the only durable kind.
#[tokio::test(flavor = "current_thread")]
async fn heartbeats_do_not_accumulate() {
    let mut node = spawn_node(Known::seed(), ALICE, "alice");
    // Startup originates #general + the online notice; the first tick adds
    // one presence. Live = 3.
    let view = wait_view(&mut node, |v| v.stats.live_entries == 3).await;
    assert_eq!(view.roster.len(), 1);

    for _ in 0..5 {
        node.cmd.send(Command::HeartbeatTick).await.unwrap();
    }
    node.cmd
        .send(Command::SendChat {
            channel: HOME_CHANNEL.into(),
            body: "x".into(),
        })
        .await
        .unwrap();
    // The chat is the 4th live entry; if beats accumulated we would see 8+.
    let view = wait_view(&mut node, |v| has_chat(v, HOME_CHANNEL, "x")).await;
    assert_eq!(view.stats.live_entries, 4);
    assert_eq!(view.roster.len(), 1);
}

/// Shutdown says goodbye, redacts our presence, and returns the `Known`
/// with retire candidates ordered by recency.
#[tokio::test(flavor = "current_thread")]
async fn shutdown_returns_known_and_candidates() {
    let (ka, kb) = bootstrap_from(Known::seed()).await;
    let mut a = spawn_node(ka, ALICE, "alice");
    let b = spawn_node(kb, BOB, "bob");
    gossip_pair(&a, &b).await;
    wait_view(&mut a, |v| v.roster.iter().any(|p| p.peer == BOB)).await;

    a.cmd.send(Command::Shutdown).await.unwrap();
    let (known, candidates) = a.task.await.unwrap();
    assert_eq!(candidates, vec![BOB]);
    // Our own presence is redacted; the goodbye notice is live.
    let snapshot = known.snapshot();
    let live: Vec<&Entry> = snapshot.iter().map(|(_, _, e)| e.as_ref()).collect();
    assert!(
        !live
            .iter()
            .any(|e| matches!(e, Entry::Presence { peer, .. } if *peer == ALICE))
    );
    assert!(
        live.iter()
            .any(|e| matches!(e, Entry::System { body, .. } if body == "alice left"))
    );
}
