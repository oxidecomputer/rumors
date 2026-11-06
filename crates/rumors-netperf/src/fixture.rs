//! Building one session's replica pair through the library's public API.
//!
//! Side A's process builds the whole history of the pair:
//!
//! 1. A *root* peer creates the network and sends, then redacts, a
//!    randomly drawn number of empty messages (the *offset*), so that its
//!    clock starts the history at a random point.
//! 2. A joins from the root, and sends every shared message and every
//!    message one side will later redact.
//! 3. B joins from A, so it receives all of them.
//! 4. The two replicas part: A inserts its own messages and redacts its
//!    share of the common ones, and B does the same with its own.
//!
//! Side B's process then joins from that copy of B over a connection of
//! its own, before the measured session ([`join_from`]). The joined peer
//! holds exactly B's live messages and B's knowledge of what was redacted,
//! which is everything a session compares; only its identity within the
//! network differs, as it would for a peer that rejoined from its old
//! replica.
//!
//! # Why one process builds both sides
//!
//! Two processes cannot each build half of one history: every call to
//! [`Peer::seed`] draws a new network secret, and peers from different
//! networks refuse to gossip, by design. So one process seeds the network,
//! and the other joins it.
//!
//! # Why the offset
//!
//! A message's position in the tree is the hash of its version keyed by its
//! network's secret, and [`Peer::seed`] draws a fresh secret for every
//! session's root, so each session's tree is a fresh uniform draw on that
//! account alone. The offset varies the versions themselves as well: every
//! message A sends carries the root's event count in its version, so two
//! different offsets give two disjoint sets of versions. Offsets are drawn from
//! [`OFFSET_RANGE`], whose values all take the same number of bits, so the
//! versions' encoded sizes do not depend on which one is drawn.

use std::ops::Range;
use std::time::Instant;

use anyhow::{Context, Result, bail, ensure};
use rand::{Rng, RngExt};
use rumors::{Joined, Peer, Rumors, Snapshot, Version};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha3::{Digest, Sha3_256};
use tokio::io::{AsyncBufRead, AsyncWrite, BufReader, duplex, split};

use crate::point::{Point, SideSettings};
use crate::sizes::{Sampler, content_len};

/// The payload every fixture message carries: an opaque byte string.
pub type Payload = ByteBuf;

/// The range the offset is drawn from: every value takes twelve bits.
pub const OFFSET_RANGE: Range<u64> = 1 << 11..1 << 12;

/// The in-memory pipe's buffer for the sessions that build a fixture.
const PIPE_BYTES: usize = 1 << 20;

/// Messages sent per batch while building.
const SEND_BATCH: u64 = 1 << 16;

/// Which side of the pair a process keeps.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Side A: builds the pair, listens, and serves B's replica.
    A,
    /// Side B: joins its replica from side A, then connects.
    B,
}

/// What building a pair measured and drew.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct BuildReport {
    /// The offset drawn for the root's clock.
    pub offset: u64,
    /// Messages sent while building, by both sides.
    pub messages: u64,
    /// Their encoded bytes in total.
    pub message_bytes: u64,
    /// Size draws redrawn because they exceeded the cap.
    pub redrawn_above_cap: u64,
    /// Size draws redrawn because they rounded to no bytes.
    pub redrawn_below_one_byte: u64,
    /// SHA3-256 of side B's live set, which side B's joined copy must
    /// reproduce.
    pub b_fingerprint: [u8; 32],
    /// Wall seconds spent building.
    pub seconds: f64,
}

impl BuildReport {
    /// The mean encoded size of the messages drawn.
    pub fn mean_message_bytes(&self) -> f64 {
        if self.messages == 0 {
            0.0
        } else {
            self.message_bytes as f64 / self.messages as f64
        }
    }
}

/// A built pair: side A configured for its session, and side B's replica,
/// to serve once to side B's process.
#[derive(Debug)]
pub struct Pair {
    /// Side A's replica, configured with the point's settings for A.
    pub a: Rumors<Payload>,
    /// Side B's replica as the history left it.
    pub b: Rumors<Payload>,
    /// What building measured and drew.
    pub report: BuildReport,
}

/// Builds the pair `point` describes from a freshly drawn history.
///
/// # Errors
///
/// If a fixture session fails, or a side does not hold the live messages
/// the point's counts require; either is a defect in the library or in
/// this tool, and the error names it.
pub async fn build(point: &Point) -> Result<Pair> {
    build_from(point, Peer::seed(), &mut rand::rng()).await
}

/// Builds the pair `point` describes in the network `root` founds, drawing
/// the history (the offset, and every message's size and contents) from
/// `rng`.
///
/// [`build`] passes a fresh network and the thread's random source; the
/// tests pass a network and a stream they derive from one proptest-drawn
/// seed, so a failing case replays its whole history.
///
/// # Errors
///
/// As [`build`].
async fn build_from(point: &Point, root: Peer<Payload>, rng: &mut impl Rng) -> Result<Pair> {
    let began = Instant::now();
    let replicas = point.replicas;
    let mut sampler = point.sizes.sampler();
    let mut report = BuildReport {
        offset: rng.random_range(OFFSET_RANGE),
        ..BuildReport::default()
    };
    let root = root.into_rumors();
    root.send_all((0..report.offset).map(|_| ByteBuf::new()))
        .context("sending the root's offset messages")?;
    let offset_versions: Vec<Version> = root.snapshot().versions().cloned().collect();
    root.redact_all(&offset_versions);

    let a = bootstrap_in_memory(&root)
        .await
        .context("joining A from the root")?;
    drop(root);
    send_drawn(&a, replicas.common(), &mut sampler, rng, &mut report)
        .context("sending the common messages")?;
    let b = bootstrap_in_memory(&a).await.context("joining B from A")?;

    let ranked = ranked_versions(&a.snapshot());
    let (redacted_by_a, rest) = ranked.split_at(to_index(replicas.redact_a)?);
    let redacted_by_b = &rest[..to_index(replicas.redact_b)?];

    send_drawn(&a, replicas.insert_a, &mut sampler, rng, &mut report)
        .context("sending A's insertions")?;
    send_drawn(&b, replicas.insert_b, &mut sampler, rng, &mut report)
        .context("sending B's insertions")?;
    a.redact_all(redacted_by_a);
    b.redact_all(redacted_by_b);
    drop(ranked);

    for (side, rumors, expected) in [
        (Side::A, &a, replicas.live_a()),
        (Side::B, &b, replicas.live_b()),
    ] {
        let live = rumors.snapshot().len() as u64;
        ensure!(
            live == expected,
            "side {side:?} holds {live} live messages after building, but the point requires {expected}"
        );
    }

    report.b_fingerprint = fingerprint(&b.snapshot());
    report.redrawn_above_cap = sampler.redrawn_above_cap;
    report.redrawn_below_one_byte = sampler.redrawn_below_one_byte;
    let a = configure(a, &point.settings.a, point).await?;
    report.seconds = began.elapsed().as_secs_f64();
    Ok(Pair { a, b, report })
}

/// `count` as an index into the common messages.
fn to_index(count: u64) -> Result<usize> {
    usize::try_from(count).context("a redaction count exceeds this platform's address space")
}

/// Sends `count` messages whose sizes `sampler` draws and whose contents
/// `rng` fills, adding them to `report`.
///
/// Messages go out in batches of [`SEND_BATCH`], so the pending batch, not
/// the whole set, is what sits in memory twice.
fn send_drawn(
    rumors: &Rumors<Payload>,
    count: u64,
    sampler: &mut Sampler,
    rng: &mut impl Rng,
    report: &mut BuildReport,
) -> Result<()> {
    let mut remaining = count;
    while remaining > 0 {
        let batch = remaining.min(SEND_BATCH);
        let mut messages = Vec::with_capacity(batch as usize);
        for _ in 0..batch {
            let size = sampler.draw(rng);
            let len = content_len(size).expect("the sampler draws only encodable sizes");
            let mut bytes = vec![0; usize::try_from(len).context("a message size exceeds memory")?];
            rng.fill_bytes(&mut bytes);
            messages.push(ByteBuf::from(bytes));
            report.message_bytes += size;
        }
        rumors.send_all(messages)?;
        report.messages += batch;
        remaining -= batch;
    }
    Ok(())
}

/// Every live version of `snapshot`, in [`Version::ranked`] order: a total
/// order fixed by the versions alone.
///
/// Redacting a prefix of it removes messages scattered uniformly over the
/// tree, since a message's place in the tree is its version's keyed hash.
fn ranked_versions(snapshot: &Snapshot<Payload>) -> Vec<Version> {
    let mut versions: Vec<Version> = snapshot.versions().cloned().collect();
    versions.sort_by(|x, y| x.ranked().cmp(&y.ranked()));
    versions
}

/// SHA3-256 over the live set of `snapshot`: each message's CBOR-encoded
/// version and its contents, in [`Version::ranked`] order.
pub fn fingerprint(snapshot: &Snapshot<Payload>) -> [u8; 32] {
    let mut entries: Vec<_> = snapshot.iter().collect();
    entries.sort_by(|x, y| x.0.ranked().cmp(&y.0.ranked()));
    let mut hasher = Sha3_256::new();
    let mut encoded = Vec::new();
    for (version, message) in entries {
        encoded.clear();
        ciborium::into_writer(version, &mut encoded)
            .expect("encoding a version into memory cannot fail");
        hasher.update((encoded.len() as u64).to_le_bytes());
        hasher.update(&encoded);
        hasher.update((message.len() as u64).to_le_bytes());
        hasher.update(message.as_slice());
    }
    hasher.finalize().into()
}

/// Joins a new peer from `server` over an in-memory pipe.
async fn bootstrap_in_memory(server: &Rumors<Payload>) -> Result<Rumors<Payload>> {
    let (near, far) = duplex(PIPE_BYTES);
    let (far_read, far_write) = split(far);
    let (near_read, near_write) = split(near);
    let (served, joined) = tokio::join!(
        server.gossip_once(BufReader::new(far_read), far_write),
        join(BufReader::new(near_read), near_write)
    );
    served.context("the serving side of a fixture join failed")?;
    joined
}

/// Joins a new peer over `read` and `write`.
async fn join<R, W>(read: R, write: W) -> Result<Rumors<Payload>>
where
    R: AsyncBufRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    match Peer::<Payload>::bootstrap().join(read, write).await {
        Joined::Joined { peer, .. } => Ok(peer.into_rumors()),
        Joined::Failed { error, .. } => Err(error).context("a join failed"),
        // The fixture's own connection failed after the identity arrived;
        // a measurement never runs on a session that did not complete.
        Joined::Unconfirmed { error, .. } => {
            Err(error).context("a join's completion was not confirmed")
        }
        Joined::Bailed { .. } => bail!("a join ended without joining: the server bailed"),
        Joined::Unbookmarked { .. } => {
            bail!("a join without a bookmark reported a bookmark failure")
        }
    }
}

/// Side B's process: joins side B's replica from side A's process over
/// `read` and `write`, and configures it with the point's settings for B.
///
/// # Errors
///
/// If the join fails.
pub async fn join_from<R, W>(read: R, write: W, point: &Point) -> Result<Rumors<Payload>>
where
    R: AsyncBufRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    let joined = join(read, write).await?;
    configure(joined, &point.settings.b, point).await
}

/// Applies the point's `Peer` settings for one side.
pub async fn configure(
    rumors: Rumors<Payload>,
    side: &SideSettings,
    point: &Point,
) -> Result<Rumors<Payload>> {
    let mut peer = rumors
        .try_into_peer()
        .await
        .context("the built replica still had another handle")?;
    if let Some(budget) = side.budget {
        peer = peer.sync_memory_budget(usize::try_from(budget).unwrap_or(usize::MAX));
    }
    if let Some(bytes) = side.target_message_size {
        peer = peer.target_message_size(usize::try_from(bytes).unwrap_or(usize::MAX));
    }
    if let Some(limit) = point.settings.payload_depth_limit {
        peer = peer.payload_depth_limit(limit);
    }
    if let Some(deadline) = point.settings.session_deadline {
        peer = peer.session_deadline(move || tokio::time::sleep(deadline));
    }
    Ok(peer.into_rumors())
}

#[cfg(test)]
mod tests;
