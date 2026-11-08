//! In-process mode: each point's session over an in-memory duplex stream,
//! timed by each side's own thread CPU clock.
//!
//! Emulation measures what a link costs a session; this mode measures what
//! the session costs each side's processor, with no kernel TCP stack, no
//! link, and no second process. It builds each point's pair as emulation
//! does ([`fixture::build`]), sets up the point's transport (TLS 1.3 by
//! default) over a [`tokio::io::duplex`] stream, and runs the session with
//! each side on a thread and a current-thread runtime of its own, as two
//! peer processes would. Each side reads its thread's CPU clock, user and
//! system time separately, around its session alone, and a [`Counting`]
//! wrapper beneath TLS counts the writes and reads each side's end of the
//! stream saw. The process's CPU time from the start until both sessions
//! have ended is printed too, so that work a session runs on other threads
//! still shows; each replica is dropped only after every reading.
//!
//! The stream holds `--capacity` bytes in each direction, so a side that
//! writes faster than its peer reads waits, as it would on a socket whose
//! buffers are full. Nothing crosses a kernel, so a side's system time is
//! what its own allocations and wake-ups cost.
//!
//! The point's link and connection axes do not apply and are ignored. Each
//! session prints one tab-separated line to standard output, after a
//! header line; nothing is written to disk. Per-thread CPU time needs
//! Linux; elsewhere the mode refuses to run.

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use rumors::{Rumors, SessionStats};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, DuplexStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::fixture::{self, Payload};
use crate::point::Point;
use crate::transport::{self, Counting, SocketCounts, Tally};

/// What one side's session cost and did.
struct Side {
    /// User CPU time over the session.
    user: Duration,
    /// System CPU time over the session.
    system: Duration,
    /// The session's own account of what it did.
    stats: SessionStats,
    /// What this side's end of the stream saw, beneath TLS.
    counts: SocketCounts,
    /// When the session started on this side.
    began: Instant,
    /// When it ended on this side.
    ended: Instant,
    /// The replica, handed back so that dropping its tree falls outside
    /// every measurement.
    replica: Rumors<Payload>,
}

/// What a session cost the whole process: user, then system CPU time.
///
/// Each side's thread clock sees only that thread; a session whose work
/// runs on other threads shows here.
type ProcessCpu = (Duration, Duration);

/// Runs `repetitions` sessions of each point, printing a line per session.
///
/// # Errors
///
/// If this platform has no per-thread CPU clock, or a fixture, handshake
/// or session fails.
pub fn run(points: &[(String, Point)], repetitions: u32, capacity: usize) -> Result<()> {
    ensure!(
        cfg!(target_os = "linux"),
        "in-process mode reads each side's thread CPU clock, which needs Linux"
    );
    println!(
        "point\trep\twall_s\ta_user_s\ta_sys_s\tb_user_s\tb_sys_s\tproc_user_s\tproc_sys_s\ta_frames\tb_frames\ta_bytes\tb_bytes\ta_writes\tb_writes\ta_gained\tb_gained\tdisputed\tbuild_s"
    );
    for (group, point) in points {
        for rep in 0..repetitions {
            let built = Instant::now();
            let (a, b) = build(point)?;
            let build_seconds = built.elapsed().as_secs_f64();
            let (wall, (proc_user, proc_system), [a, b]) = session(a, b, point, capacity)?;
            println!(
                "{group}: {}\t{rep}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{build_seconds:.2}",
                point.key(),
                wall.as_secs_f64(),
                a.user.as_secs_f64(),
                a.system.as_secs_f64(),
                b.user.as_secs_f64(),
                b.system.as_secs_f64(),
                proc_user.as_secs_f64(),
                proc_system.as_secs_f64(),
                a.stats.frames_sent,
                b.stats.frames_sent,
                a.stats.bytes_sent,
                b.stats.bytes_sent,
                a.counts.writes,
                b.counts.writes,
                a.stats.messages_gained,
                b.stats.messages_gained,
                a.stats.disputed_scopes,
            );
            drop((a.replica, b.replica));
        }
    }
    Ok(())
}

/// Builds `point`'s pair, both sides configured with the point's settings.
fn build(point: &Point) -> Result<(Rumors<Payload>, Rumors<Payload>)> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .context("starting the fixture's runtime")?;
    runtime.block_on(async {
        let pair = fixture::build(point).await?;
        let b = fixture::configure(pair.b, &point.settings.b, point).await?;
        Ok((pair.a, b))
    })
}

/// Reconciles `a` and `b` once over a duplex stream of `capacity` bytes
/// each way, returning the wall time, the process's CPU time, and what
/// each side measured.
///
/// The wall time runs from the first side's start to the last side's end;
/// the process's CPU time from the start until both sessions have ended.
fn session(
    a: Rumors<Payload>,
    b: Rumors<Payload>,
    point: &Point,
    capacity: usize,
) -> Result<(Duration, ProcessCpu, [Side; 2])> {
    let (near, far): (DuplexStream, DuplexStream) = tokio::io::duplex(capacity);
    let start = Arc::new(Barrier::new(3));
    let tls = point.transport.tls;
    let a = spawn(a, far, tls.then_some(transport::Role::Accepted), &start)?;
    let b = spawn(b, near, tls.then_some(transport::Role::Connected), &start)?;
    let before = process_cpu()?;
    start.wait();
    let a = a.join().expect("side A's thread does not panic")?;
    let b = b.join().expect("side B's thread does not panic")?;
    let after = process_cpu()?;
    let wall = a.ended.max(b.ended) - a.began.min(b.began);
    let process = (
        after.0.saturating_sub(before.0),
        after.1.saturating_sub(before.1),
    );
    Ok((wall, process, [a, b]))
}

/// Runs one side on a thread of its own: completes its end of the TLS
/// handshake as `role` (none for plain streams), waits at `start`, and
/// gossips once.
fn spawn(
    peer: Rumors<Payload>,
    stream: DuplexStream,
    role: Option<transport::Role>,
    start: &Arc<Barrier>,
) -> Result<thread::JoinHandle<Result<Side>>> {
    let start = Arc::clone(start);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .context("starting a side's runtime")?;
    Ok(thread::spawn(move || {
        runtime.block_on(async move {
            let tally = Arc::new(Tally::default());
            let stream = Counting::new(stream, Arc::clone(&tally));
            match role {
                Some(transport::Role::Accepted) => {
                    let mut tls = TlsAcceptor::from(transport::server_config()?)
                        .accept(stream)
                        .await
                        .context("serving the TLS handshake")?;
                    tls.flush().await.context("flushing the TLS handshake")?;
                    gossip(peer, tls, &tally, &start).await
                }
                Some(transport::Role::Connected) => {
                    let name = ServerName::try_from(transport::SERVER_NAME)
                        .expect("the server name is valid");
                    let mut tls = TlsConnector::from(transport::client_config())
                        .connect(name, stream)
                        .await
                        .context("completing the TLS handshake")?;
                    tls.flush().await.context("flushing the TLS handshake")?;
                    gossip(peer, tls, &tally, &start).await
                }
                None => gossip(peer, stream, &tally, &start).await,
            }
        })
    }))
}

/// Gossips once over `stream`, after every side has reached `start`,
/// reading this thread's CPU clock, the time, and `tally` around the
/// session alone: dropping the replica afterwards is not the session's.
async fn gossip<S>(peer: Rumors<Payload>, stream: S, tally: &Tally, start: &Barrier) -> Result<Side>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (read, write) = tokio::io::split(stream);
    start.wait();
    tally.reset();
    let (user, system) = thread_cpu()?;
    let began = Instant::now();
    let (gossiped, ..) = peer
        .gossip_once(BufReader::new(read), write)
        .await
        .context("gossiping over the duplex stream")?;
    let ended = Instant::now();
    let (user_after, system_after) = thread_cpu()?;
    Ok(Side {
        user: user_after.saturating_sub(user),
        system: system_after.saturating_sub(system),
        stats: gossiped.stats,
        counts: tally.counts(),
        began,
        ended,
        replica: peer,
    })
}

/// The CPU time the calling thread has used: user, then system.
#[cfg(target_os = "linux")]
fn thread_cpu() -> Result<(Duration, Duration)> {
    usage(nix::sys::resource::UsageWho::RUSAGE_THREAD)
}

/// Without a per-thread CPU clock, [`run`] refuses before any session.
#[cfg(not(target_os = "linux"))]
fn thread_cpu() -> Result<(Duration, Duration)> {
    anyhow::bail!("in-process mode reads each side's thread CPU clock, which needs Linux")
}

/// The CPU time the whole process has used: user, then system.
fn process_cpu() -> Result<(Duration, Duration)> {
    usage(nix::sys::resource::UsageWho::RUSAGE_SELF)
}

/// The user and system CPU time `who` has used.
fn usage(who: nix::sys::resource::UsageWho) -> Result<(Duration, Duration)> {
    use nix::sys::time::TimeVal;
    let usage = nix::sys::resource::getrusage(who).context("reading CPU time")?;
    let time = |t: TimeVal| {
        Duration::new(
            u64::try_from(t.tv_sec()).unwrap_or(0),
            u32::try_from(t.tv_usec()).unwrap_or(0) * 1_000,
        )
    };
    Ok((time(usage.user_time()), time(usage.system_time())))
}
