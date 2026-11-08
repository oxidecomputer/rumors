//! One side of one measured session: obtain the side's replica, open or
//! accept the TCP connection, gossip once, and report what was measured.
//!
//! The same code runs in both modes. In emulation mode the orchestrator
//! starts a peer process per side inside the side's network namespace and
//! talks to it over its standard input and output; in pair mode the `serve`
//! and `connect` commands drive it over a control connection. Either way
//! the conversation is one JSON document per line, in this order:
//!
//! 1. the driver sends both sides [`Command::Setup`]. Side A builds the
//!    pair and answers [`Reply::Built`]; side B answers [`Reply::Ready`];
//! 2. side A receives [`Command::Serve`], answers [`Reply::Listening`],
//!    serves side B's replica to one connection, frees it, and answers
//!    [`Reply::Served`]; side B receives [`Command::Fetch`], joins its
//!    replica over that connection, and answers [`Reply::Fetched`]. The
//!    driver compares the fingerprints and, in emulation mode, shapes the
//!    link only now, so the transfer runs at the kernel's speed. Both
//!    sides then wait, idle, for as long as the driver takes to send the
//!    next command: in a run, until the session's turn to be timed;
//! 3. side A receives [`Command::Listen`] and answers
//!    [`Reply::Listening`]; side B receives [`Command::Connect`]. Both
//!    gossip once over the new connection and answer [`Reply::Finished`].

use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use rumors::SessionStats;
use serde::{Deserialize, Serialize};
use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

use crate::fixture::{self, BuildReport, Side};
use crate::meter::{self, Usage};
use crate::point::{Connection, Point};
use crate::transport::{Role, SocketCounts, establish};

/// A driver's instruction to a peer.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// Prepare to measure the point as `side`.
    Setup {
        /// The point.
        point: Box<Point>,
        /// The side this process plays.
        side: Side,
        /// Bytes each side sends the other before a warm session.
        warm_bytes: u64,
    },
    /// Side A: listen on this address (port 0 picks a free port) and serve
    /// side B's replica to the first connection.
    Serve {
        /// The address to bind.
        address: SocketAddr,
    },
    /// Side B: join this side's replica from side A at this address.
    Fetch {
        /// Side A's serving address.
        address: SocketAddr,
    },
    /// Side A: listen on this address for the measured session.
    Listen {
        /// The address to bind.
        address: SocketAddr,
    },
    /// Side B: connect to side A at this address for the measured session.
    Connect {
        /// Side A's address.
        address: SocketAddr,
    },
}

/// A peer's answer to its driver.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "reply", rename_all = "snake_case")]
pub enum Reply {
    /// Side A built the pair.
    Built {
        /// What building measured and drew.
        report: BuildReport,
    },
    /// Side B is ready to fetch its replica.
    Ready,
    /// Side A listens on this port.
    Listening {
        /// The bound port.
        port: u16,
    },
    /// Side A served side B's replica and freed its copy.
    Served,
    /// Side B joined its replica.
    Fetched {
        /// SHA3-256 of the joined live set.
        fingerprint: [u8; 32],
        /// Wall seconds the join took.
        seconds: f64,
    },
    /// The session ended.
    Finished {
        /// What the side measured.
        outcome: Box<Outcome>,
    },
    /// The peer failed before finishing; the text says why.
    Failed {
        /// The error, with its causes.
        error: String,
    },
}

/// The library's own account of the session, from [`SessionStats`].
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Stats {
    /// Session latency as the library measured it, in seconds.
    pub elapsed_seconds: f64,
    /// Disputed scopes this side compared.
    pub disputed_scopes: u64,
    /// Messages learned from the peer.
    pub messages_gained: u64,
    /// Messages shed because the peer had redacted them.
    pub messages_shed: u64,
    /// Data-frame bytes sent.
    pub bytes_sent: u64,
    /// Data frames sent.
    pub frames_sent: u64,
    /// Data-frame bytes received.
    pub bytes_received: u64,
    /// Data frames received.
    pub frames_received: u64,
    /// Units held back for budget.
    pub budget_waits: u64,
}

impl From<SessionStats> for Stats {
    /// Copies the library's counters.
    fn from(stats: SessionStats) -> Self {
        Self {
            elapsed_seconds: stats.elapsed.as_secs_f64(),
            disputed_scopes: stats.disputed_scopes,
            messages_gained: stats.messages_gained,
            messages_shed: stats.messages_shed,
            bytes_sent: stats.bytes_sent,
            frames_sent: stats.frames_sent,
            bytes_received: stats.bytes_received,
            frames_received: stats.frames_received,
            budget_waits: stats.budget_waits,
        }
    }
}

/// What one side measured over its session.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Outcome {
    /// Wall seconds from the start of the session (for side B, the start of
    /// its `connect`; for a warm session, the end of the warm-up) until its
    /// `gossip_once` returned.
    pub wall_seconds: f64,
    /// Wall seconds side B's `connect` took, which [`wall_seconds`](Self::wall_seconds)
    /// includes: one round trip on a fresh connection. Zero on side A and on
    /// a warm connection, whose session starts after the connect.
    pub connect_seconds: f64,
    /// What this side's socket saw during the session, session setup and
    /// completion included: bytes each way, and the write and read
    /// operations beneath every buffer and TLS.
    pub socket: SocketCounts,
    /// The library's counters.
    pub stats: Stats,
    /// The process's resource use over the session.
    pub usage: Usage,
    /// The live messages this side holds after the session.
    pub live_after: u64,
    /// The memory budget this side's peer runs its sessions at, as the
    /// library reports it in
    /// [`SynchronizationSettings`](rumors::SynchronizationSettings).
    pub effective_budget: u64,
    /// Why the session failed, if it did.
    pub error: Option<String>,
}

/// Runs one peer to completion over a line-oriented control channel.
///
/// # Errors
///
/// If the control channel fails or carries something out of order. A
/// failure of the session itself is not an error here: it is reported to
/// the driver as [`Reply::Finished`] with [`Outcome::error`] set.
pub fn serve_one(control_in: &mut impl BufRead, control_out: &mut impl Write) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("starting the peer's runtime")?;
    let result = runtime.block_on(run(control_in, control_out));
    if let Err(error) = &result {
        // The driver may already be gone; the error is returned regardless.
        let _ = send(
            control_out,
            &Reply::Failed {
                error: format!("{error:#}"),
            },
        );
    }
    result
}

/// The peer's conversation, on its runtime.
async fn run(control_in: &mut impl BufRead, control_out: &mut impl Write) -> Result<()> {
    let Command::Setup {
        point,
        side,
        warm_bytes,
    } = receive(control_in)?
    else {
        bail!("a peer's first command must be `setup`");
    };
    let rumors = match side {
        Side::A => {
            let pair = fixture::build(&point).await?;
            send(
                control_out,
                &Reply::Built {
                    report: pair.report,
                },
            )?;
            let Command::Serve { address } = receive(control_in)? else {
                bail!("side A's second command must be `serve`");
            };
            let listener = bind(address).await?;
            send(
                control_out,
                &Reply::Listening {
                    port: listener.local_addr()?.port(),
                },
            )?;
            let (stream, _) = listener
                .accept()
                .await
                .context("accepting side B's fetch")?;
            stream.set_nodelay(true)?;
            let (read, write) = stream.into_split();
            pair.b
                .gossip_once(BufReader::new(read), write)
                .await
                .context("serving side B's replica")?;
            drop(pair.b);
            send(control_out, &Reply::Served)?;
            pair.a
        }
        Side::B => {
            send(control_out, &Reply::Ready)?;
            let Command::Fetch { address } = receive(control_in)? else {
                bail!("side B's second command must be `fetch`");
            };
            let began = Instant::now();
            let stream = TcpStream::connect(address).await.with_context(|| {
                format!("connecting to side A at {address} to fetch the replica")
            })?;
            stream.set_nodelay(true)?;
            let (read, write) = stream.into_split();
            let rumors = fixture::join_from(BufReader::new(read), write, &point).await?;
            let fingerprint = fixture::fingerprint(&rumors.snapshot());
            send(
                control_out,
                &Reply::Fetched {
                    fingerprint,
                    seconds: began.elapsed().as_secs_f64(),
                },
            )?;
            rumors
        }
    };

    let outcome = match (side, receive(control_in)?) {
        (Side::A, Command::Listen { address }) => {
            let listener = bind(address).await?;
            send(
                control_out,
                &Reply::Listening {
                    port: listener.local_addr()?.port(),
                },
            )?;
            let (stream, _) = listener.accept().await.context("accepting side B")?;
            session(&rumors, stream, &point, warm_bytes, Side::A, 0.0).await
        }
        (Side::B, Command::Connect { address }) => {
            let began = Instant::now();
            let stream = TcpStream::connect(address)
                .await
                .with_context(|| format!("connecting to side A at {address}"))?;
            let connect_seconds = began.elapsed().as_secs_f64();
            session(
                &rumors,
                stream,
                &point,
                warm_bytes,
                Side::B,
                connect_seconds,
            )
            .await
        }
        (side, command) => bail!("side {side:?} cannot follow its replica with {command:?}"),
    };
    let outcome = Outcome {
        live_after: rumors.snapshot().len() as u64,
        effective_budget: u64::try_from(rumors.synchronization_settings().sync_memory_budget)
            .unwrap_or(u64::MAX),
        ..outcome
    };
    send(
        control_out,
        &Reply::Finished {
            outcome: Box::new(outcome),
        },
    )
}

/// Binds a listener on `address`.
async fn bind(address: SocketAddr) -> Result<TcpListener> {
    TcpListener::bind(address)
        .await
        .with_context(|| format!("binding {address}"))
}

/// Gossips once over `stream`, through the point's transport stack, and
/// measures it.
///
/// `connect_seconds` is the time side B already spent connecting: the
/// session's wall time counts from the start of the connect on a fresh
/// connection, as an application that connects to synchronize sees it. A
/// TLS handshake belongs to connecting: side B's connect time includes it,
/// and side A's clock starts after it. On a warm connection, `warm_bytes`
/// first cross each way beneath the read buffer, and both clocks start
/// after them ([`establish`] orders the warm-up so the socket's tally
/// counts none of it).
async fn session(
    rumors: &rumors::Rumors<fixture::Payload>,
    stream: TcpStream,
    point: &Point,
    warm_bytes: u64,
    side: Side,
    connect_seconds: f64,
) -> Outcome {
    let mut outcome = Outcome::default();
    let role = match side {
        Side::A => Role::Accepted,
        Side::B => Role::Connected,
    };
    let warm = point.connection == Connection::Warm;
    let began = Instant::now();
    let established = establish(stream, point.transport, role, warm.then_some(warm_bytes)).await;
    let (mut read, mut write, tally) = match established {
        Ok(halves) => halves,
        Err(error) => {
            outcome.error = Some(format!("{error:#}"));
            return outcome;
        }
    };
    let prior = match side {
        Side::B if !warm => connect_seconds + began.elapsed().as_secs_f64(),
        Side::A | Side::B => 0.0,
    };
    outcome.connect_seconds = prior;
    let mark = meter::start();
    let began = Instant::now();
    let result = rumors.gossip_once(&mut read, &mut write).await;
    outcome.wall_seconds = prior + began.elapsed().as_secs_f64();
    outcome.usage = mark.usage();
    outcome.socket = tally.counts();
    match result {
        Ok((gossiped, _, _)) => outcome.stats = gossiped.stats.into(),
        Err(error) => outcome.error = Some(error.to_string()),
    }
    outcome
}

/// Writes one control message as a line.
pub fn send<M: Serialize>(out: &mut impl Write, message: &M) -> Result<()> {
    serde_json::to_writer(&mut *out, message)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// Reads one control message from a line.
pub fn receive<M: for<'de> Deserialize<'de>>(input: &mut impl BufRead) -> Result<M> {
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        bail!("the control channel closed");
    }
    serde_json::from_str(&line).with_context(|| format!("reading a control message: {line:?}"))
}
