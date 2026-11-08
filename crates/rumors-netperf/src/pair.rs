//! Pair mode: measuring over the network between two hosts, on any platform
//! Rumors supports.
//!
//! `serve` runs on one host and waits. `connect` runs on the other, drives the
//! grid, and writes the results. Each peer process still runs one session and
//! exits, as in emulation: for every process the connecting side needs on the
//! serving host, it opens a fresh TCP connection to `serve`, which starts the
//! process and relays its control lines over that connection until the process
//! exits. Side A of every session runs on the serving host, side B on the
//! connecting one.
//!
//! Nothing is shaped: the link is whatever the network is. Before the grid,
//! `connect` probes it ([`crate::probe`]) in each direction, and the report
//! reads every session against the measured round trip and goodput.

use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::driver::{self, Addresses, Lines, Measured, Prepared, Remote};
use crate::env::{Build, Environment};
use crate::grid::Session;
use crate::peer::{receive, send};
use crate::probe::{ProbeCommand, ProbeReply, ProbeResult};
use crate::run::Executor;

/// The port `serve` listens on unless told otherwise.
pub const DEFAULT_PORT: u16 = 7979;

/// The kind of process a relay connection asks `serve` to start.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    /// A session peer.
    Peer,
    /// A link probe.
    Probe,
}

/// The first line on every relay connection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Open {
    /// The process to start, or `None` to exchange environments only.
    pub kind: Option<ProcessKind>,
    /// The connecting tool's build, whose version and revision must match the
    /// server's.
    pub build: Build,
}

/// The server's answer to [`Open`].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "opened", rename_all = "snake_case")]
pub enum Opened {
    /// The process started; the server's environment follows.
    Started {
        /// The serving host's environment.
        environment: Box<Environment>,
    },
    /// The server refused; the text says why.
    Refused {
        /// Why.
        reason: String,
    },
}

/// A local child process spoken to over its standard input and output, stopped
/// when dropped.
#[derive(Debug)]
pub struct LocalChild {
    /// The process.
    child: Child,
    /// Its control channel.
    pub lines: Lines<BufReader<ChildStdout>, ChildStdin>,
}

impl LocalChild {
    /// Starts `program` with `argument` (`peer` or `probe-peer`).
    pub fn spawn(program: &Path, argument: &str) -> Result<Self> {
        let mut child = Command::new(program)
            .arg(argument)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("starting a local `{argument}` process"))?;
        let input = BufReader::new(child.stdout.take().expect("stdout is piped"));
        let output = child.stdin.take().expect("stdin is piped");
        Ok(Self {
            child,
            lines: Lines { input, output },
        })
    }
}

impl Drop for LocalChild {
    /// Stops the process if it still runs.
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// The argument that starts a process of `kind`.
fn argument(kind: ProcessKind) -> &'static str {
    match kind {
        ProcessKind::Peer => "peer",
        ProcessKind::Probe => "probe-peer",
    }
}

/// Serves relay connections on `listen` until interrupted, starting a process
/// of the requested kind for each and relaying its control lines.
///
/// # Errors
///
/// If the listener cannot bind.
pub fn serve(program: &Path, listen: SocketAddr, mut log: impl FnMut(&str) + Send) -> Result<()> {
    let listener = TcpListener::bind(listen).with_context(|| format!("binding {listen}"))?;
    log(&format!(
        "serving on {}; run `rumors-netperf connect <this host>:{}` on the other host",
        listener.local_addr()?,
        listener.local_addr()?.port()
    ));
    let environment = Environment::capture()?;
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                log(&format!("accepting a connection failed: {error}"));
                continue;
            }
        };
        let peer = stream
            .peer_addr()
            .map_or("unknown".into(), |a| a.to_string());
        // One connection at a time: a session's two connections arrive in
        // sequence, and serial service keeps the serving host's own load
        // out of the measurement.
        if let Err(error) = relay(program, stream, &environment) {
            log(&format!("connection from {peer}: {error:#}"));
        }
    }
    Ok(())
}

/// Starts the requested process and relays one connection to it.
fn relay(program: &Path, stream: TcpStream, environment: &Environment) -> Result<()> {
    stream.set_nodelay(true)?;
    let mut input = BufReader::new(stream.try_clone()?);
    let mut output = stream;
    let open: Open = receive(&mut input)?;
    let ours = &environment.build;
    if open.build.version != ours.version || open.build.revision != ours.revision {
        let reason = format!(
            "this server runs rumors-netperf {} ({}), the client {} ({}): build the same revision on both hosts",
            ours.version, ours.revision, open.build.version, open.build.revision
        );
        send(
            &mut output,
            &Opened::Refused {
                reason: reason.clone(),
            },
        )?;
        bail!(reason);
    }
    let Some(kind) = open.kind else {
        return send(
            &mut output,
            &Opened::Started {
                environment: Box::new(environment.clone()),
            },
        );
    };
    let mut child = Command::new(program)
        .arg(argument(kind))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("starting the requested process")?;
    send(
        &mut output,
        &Opened::Started {
            environment: Box::new(environment.clone()),
        },
    )?;
    let mut to_child = child.stdin.take().expect("stdin is piped");
    let mut from_child = BufReader::new(child.stdout.take().expect("stdout is piped"));
    let mut back = output.try_clone()?;
    thread::scope(|scope| {
        // Lines from the client go to the process until either side closes.
        scope.spawn(move || {
            let mut line = String::new();
            while matches!(input.read_line(&mut line), Ok(n) if n > 0) {
                if to_child
                    .write_all(line.as_bytes())
                    .and_then(|()| to_child.flush())
                    .is_err()
                {
                    break;
                }
                line.clear();
            }
        });
        // Lines from the process go to the client until the process exits;
        // closing the connection then ends the other direction too.
        let mut line = String::new();
        while matches!(from_child.read_line(&mut line), Ok(n) if n > 0) {
            if back
                .write_all(line.as_bytes())
                .and_then(|()| back.flush())
                .is_err()
            {
                break;
            }
            line.clear();
        }
        let _ = output.shutdown(std::net::Shutdown::Both);
    });
    if let Ok(None) = child.try_wait() {
        let _ = child.kill();
    }
    let _ = child.wait();
    Ok(())
}

/// A server to connect to.
#[derive(Clone, Debug)]
pub struct Server {
    /// Its relay address.
    pub address: SocketAddr,
}

impl Server {
    /// Resolves `HOST[:PORT]`, with [`DEFAULT_PORT`] when the port is left out.
    pub fn resolve(text: &str) -> Result<Self> {
        let has_port = text.parse::<SocketAddr>().is_ok()
            || text
                .rsplit_once(':')
                .is_some_and(|(host, port)| !host.contains(':') && port.parse::<u16>().is_ok());
        let with_port = if has_port {
            text.to_string()
        } else {
            format!("{text}:{DEFAULT_PORT}")
        };
        let address = with_port
            .to_socket_addrs()
            .with_context(|| format!("resolving `{text}`"))?
            .next()
            .with_context(|| format!("`{text}` resolves to no address"))?;
        Ok(Self { address })
    }

    /// The server's environment.
    pub fn environment(&self) -> Result<Environment> {
        self.connect(None).map(|(_, environment)| environment)
    }

    /// Opens a relay connection that starts a process of `kind` on the server,
    /// returning its channel and the server's environment.
    pub fn open(&self, kind: ProcessKind) -> Result<(Remote, Environment)> {
        self.connect(Some(kind))
    }

    /// Opens a relay connection, starting a process of `kind` if one is given.
    fn connect(&self, kind: Option<ProcessKind>) -> Result<(Remote, Environment)> {
        let stream = TcpStream::connect(self.address).with_context(|| {
            format!(
                "connecting to `rumors-netperf serve` at {}: is it running there?",
                self.address
            )
        })?;
        stream.set_nodelay(true)?;
        let mut input = BufReader::new(stream.try_clone()?);
        let mut output = stream;
        send(
            &mut output,
            &Open {
                kind,
                build: Build::current()?,
            },
        )?;
        match receive(&mut input)? {
            Opened::Started { environment } => Ok((Lines { input, output }, *environment)),
            Opened::Refused { reason } => bail!("the server refused: {reason}"),
        }
    }

    /// The addresses of a session: side A binds every interface on the server,
    /// side B connects to the server's address.
    pub fn addresses(&self) -> Addresses {
        let unspecified = match self.address.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        };
        Addresses {
            listen: SocketAddr::new(unspecified, 0),
            connect: self.address.ip(),
        }
    }
}

/// Probes the network between this host and the server: `bulk_bytes` from this
/// host to the server, and `pings` round trips.
///
/// # Errors
///
/// If a probe process fails.
pub fn probe(program: &Path, server: &Server, bulk_bytes: u64, pings: u32) -> Result<ProbeResult> {
    let (mut remote, _) = server.open(ProcessKind::Probe)?;
    let mut local = LocalChild::spawn(program, "probe-peer")?;
    let addresses = server.addresses();
    send(
        &mut remote.output,
        &ProbeCommand::Listen {
            address: addresses.listen,
        },
    )?;
    let ProbeReply::Listening { port } = receive(&mut remote.input)? else {
        bail!("the server's probe answered `listen` out of order");
    };
    let address = SocketAddr::new(addresses.connect, port);
    send(
        &mut local.lines.output,
        &ProbeCommand::Run {
            address,
            bulk_bytes,
            pings,
        },
    )?;
    let ProbeReply::Probed { result } = receive(&mut local.lines.input)? else {
        bail!("the local probe answered `run` out of order");
    };
    Ok(result)
}

/// A session prepared across the network: side A's process on the server, side
/// B's here, both holding their replicas.
#[derive(Debug)]
pub struct Ready {
    /// The relay to side A's process.
    remote: Remote,
    /// Side B's process.
    local: LocalChild,
    /// The preparation's report.
    prepared: Prepared,
}

/// A run's sessions across the network, as [`Executor`] prepares and times
/// them: side A on the server, side B here.
///
/// The server serves one relay connection at a time, and each prepared session
/// holds one open until its timed session ends, so a run in pair mode prepares
/// one session at a time and times it before the next.
#[derive(Clone, Copy, Debug)]
pub struct Pair<'a> {
    /// The peer executable.
    pub program: &'a Path,
    /// The server.
    pub server: &'a Server,
}

impl Executor for Pair<'_> {
    type Prepared = Ready;

    /// Starts both processes and prepares the session across the network.
    fn prepare(&self, session: &Session, _slot: usize) -> Result<Ready> {
        let (mut remote, _) = self.server.open(ProcessKind::Peer)?;
        let mut local = LocalChild::spawn(self.program, "peer")?;
        let prepared = driver::prepare(
            &session.point,
            4 << 20,
            &mut remote,
            &mut local.lines,
            self.server.addresses(),
        )?;
        Ok(Ready {
            remote,
            local,
            prepared,
        })
    }

    /// Runs the prepared session's measured exchange, and stops side B's
    /// process if it still runs.
    fn finish(&self, _session: &Session, mut ready: Ready) -> Result<Measured> {
        driver::finish(
            ready.prepared,
            &mut ready.remote,
            &mut ready.local.lines,
            self.server.addresses(),
        )
    }
}
