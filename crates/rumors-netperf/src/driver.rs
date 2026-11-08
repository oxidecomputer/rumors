//! Driving one measured session through its two peer processes, wherever they
//! run: both local in network namespaces (emulation mode), or one local and one
//! on another host behind `serve` (pair mode).
//!
//! The driver holds each peer's control channel and runs the conversation
//! [`crate::peer`] describes in two halves. [`prepare`] builds the pair, moves
//! side B's replica to its process, and checks it arrived whole; [`finish`]
//! runs the measured session. Calibration prepares several sessions and then
//! finishes them together, so they gossip at the same moment; a run prepares
//! many sessions while none is timed, and then finishes them in its order
//! ([`crate::schedule`]). Between the two halves both peers wait, idle, holding
//! their replicas.

use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::fixture::{BuildReport, Side};
use crate::meter::ProcessUsage;
use crate::peer::{Command, Outcome, Reply, receive, send};
use crate::point::Point;

/// A peer's control channel.
pub trait Channel {
    /// Sends one command.
    fn send(&mut self, command: &Command) -> Result<()>;
    /// Receives one reply.
    fn receive(&mut self) -> Result<Reply>;
}

/// A control channel over a pair of byte streams.
#[derive(Debug)]
pub struct Lines<R, W> {
    /// The reply stream.
    pub input: R,
    /// The command stream.
    pub output: W,
}

impl<R: BufRead, W: Write> Channel for Lines<R, W> {
    /// Writes the command as a line.
    fn send(&mut self, command: &Command) -> Result<()> {
        send(&mut self.output, command)
    }

    /// Reads a reply line; a peer's [`Reply::Failed`] becomes an error.
    fn receive(&mut self) -> Result<Reply> {
        match receive(&mut self.input)? {
            Reply::Failed { error } => bail!("the peer failed: {error}"),
            reply => Ok(reply),
        }
    }
}

impl Channel for crate::netns::Namespaced {
    /// Writes the command to the process's standard input.
    fn send(&mut self, command: &Command) -> Result<()> {
        send(&mut self.stdin, command)
    }

    /// Reads a reply from the process's standard output.
    fn receive(&mut self) -> Result<Reply> {
        match receive(&mut self.stdout)? {
            Reply::Failed { error } => bail!("the peer failed: {error}"),
            reply => Ok(reply),
        }
    }
}

/// A control connection to a remote `serve`, which relays to a peer process it
/// started for this session.
pub type Remote = Lines<BufReader<TcpStream>, TcpStream>;

/// Where each side listens and how the other side reaches it.
#[derive(Clone, Copy, Debug)]
pub struct Addresses {
    /// The address side A binds, port 0.
    pub listen: SocketAddr,
    /// The address side B connects to, without a port.
    pub connect: IpAddr,
}

/// A session whose pair is built and in place on both sides.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Prepared {
    /// What building the pair measured and drew.
    pub build: BuildReport,
    /// Wall seconds side B took to join its replica from side A.
    pub fetch_seconds: f64,
    /// What sides A and B used over the preparation, where the driver can read
    /// it: in emulation, where both processes are its own children.
    pub usage: Option<[ProcessUsage; 2]>,
}

/// What both sides measured over one session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Measured {
    /// The preparation.
    pub prepared: Prepared,
    /// Side A's outcome.
    pub a: Outcome,
    /// Side B's outcome.
    pub b: Outcome,
}

/// Builds the pair and moves side B's replica to its process.
///
/// # Errors
///
/// If either peer fails or answers out of order, or side B's joined replica
/// differs from the one side A built.
pub fn prepare(
    point: &Point,
    warm_bytes: u64,
    a: &mut dyn Channel,
    b: &mut dyn Channel,
    addresses: Addresses,
) -> Result<Prepared> {
    let setup = |side| Command::Setup {
        point: Box::new(*point),
        side,
        warm_bytes,
    };
    a.send(&setup(Side::A))?;
    b.send(&setup(Side::B))?;
    let Reply::Built { report: build } = a.receive().context("side A building the pair")? else {
        bail!("side A answered `setup` out of order");
    };
    let Reply::Ready = b.receive()? else {
        bail!("side B answered `setup` out of order");
    };
    a.send(&Command::Serve {
        address: addresses.listen,
    })?;
    let Reply::Listening { port } = a.receive()? else {
        bail!("side A answered `serve` out of order");
    };
    b.send(&Command::Fetch {
        address: SocketAddr::new(addresses.connect, port),
    })?;
    let Reply::Fetched {
        fingerprint,
        seconds,
    } = b.receive().context("side B fetching its replica")?
    else {
        bail!("side B answered `fetch` out of order");
    };
    let Reply::Served = a.receive()? else {
        bail!("side A answered the fetch out of order");
    };
    ensure!(
        fingerprint == build.b_fingerprint,
        "side B's joined replica differs from the one side A built: the join lost or altered messages"
    );
    Ok(Prepared {
        build,
        fetch_seconds: seconds,
        usage: None,
    })
}

/// Runs the measured session of a prepared pair.
///
/// # Errors
///
/// If either peer fails or answers out of order. A failed session is not an
/// error: each side's [`Outcome::error`] records it.
pub fn finish(
    prepared: Prepared,
    a: &mut dyn Channel,
    b: &mut dyn Channel,
    addresses: Addresses,
) -> Result<Measured> {
    a.send(&Command::Listen {
        address: addresses.listen,
    })?;
    let Reply::Listening { port } = a.receive()? else {
        bail!("side A answered `listen` out of order");
    };
    b.send(&Command::Connect {
        address: SocketAddr::new(addresses.connect, port),
    })?;
    let Reply::Finished { outcome: b_outcome } = b.receive().context("side B's session")? else {
        bail!("side B answered `connect` out of order");
    };
    let Reply::Finished { outcome: a_outcome } = a.receive().context("side A's session")? else {
        bail!("side A answered the session out of order");
    };
    Ok(Measured {
        prepared,
        a: *a_outcome,
        b: *b_outcome,
    })
}
