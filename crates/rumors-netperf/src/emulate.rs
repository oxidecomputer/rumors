//! Emulation mode: each session's two peers in network namespaces of their
//! own, joined by a shaped virtual link (Linux only).
//!
//! A session runs in a *slot*, an index that keeps concurrent sessions'
//! interface names apart. [`prepare`] starts the two peer processes, joins
//! their namespaces with an unshaped veth, raises the TCP buffer limits in
//! both namespaces if the point asks, builds the pair and moves side B's
//! replica, and only then shapes the link; the two processes then wait,
//! idle, holding their replicas. [`finish`] runs the measured session and
//! returns once both processes have exited.

use std::net::{IpAddr, SocketAddr};
use std::path::Path;

use anyhow::{Result, bail};

use crate::driver::{self, Addresses, Measured, Prepared};
use crate::grid::Session;
use crate::meter::process_usage;
use crate::netns::{self, ADDRESS_A, Namespaced, Netem};
use crate::peer::{receive, send};
use crate::point::{Link, Point, TcpBuffers};
use crate::probe::{ProbeCommand, ProbeReply, ProbeResult};
use crate::run::Executor;

/// The addresses of every emulated session: side A at [`ADDRESS_A`].
pub fn addresses() -> Addresses {
    let a: IpAddr = ADDRESS_A.parse().expect("ADDRESS_A is an IPv4 address");
    Addresses {
        listen: SocketAddr::new(a, 0),
        connect: a,
    }
}

/// The most sessions one window of a run holds ([`crate::schedule`]).
///
/// Each holds two processes and their pipes, and choosing a window's
/// length weighs every prefix of it, at a cost that grows with the square
/// of its length; a window this long already holds far more preparation
/// than the workers need to stay busy.
pub const MAX_WINDOW: usize = 512;

/// File descriptors one prepared or preparing session holds open in the
/// orchestrator: each of its two processes' standard input and output.
const DESCRIPTORS_PER_SESSION: u64 = 4;

/// File descriptors each running preparation may open besides its
/// session's own: the pipes of the command it runs at that moment.
const DESCRIPTORS_PER_PREPARATION: u64 = 4;

/// File descriptors kept for everything else: the standard streams, the
/// results files, and the libraries' own.
const RESERVED_DESCRIPTORS: u64 = 64;

/// The longest window this process's descriptor limit allows with
/// `prepare_jobs` preparations running.
///
/// It is at most [`MAX_WINDOW`] and at least one, after raising the soft
/// limit toward the hard one as far as the longest window needs.
pub fn descriptor_window(prepare_jobs: usize) -> usize {
    use nix::sys::resource::{Resource, getrlimit, setrlimit};
    let reserved = RESERVED_DESCRIPTORS + DESCRIPTORS_PER_PREPARATION * prepare_jobs as u64;
    let wanted = reserved + DESCRIPTORS_PER_SESSION * MAX_WINDOW as u64;
    let soft = match getrlimit(Resource::RLIMIT_NOFILE) {
        Ok((soft, hard)) if soft < wanted => {
            let raised = wanted.min(hard);
            setrlimit(Resource::RLIMIT_NOFILE, raised, hard).map_or(soft, |()| raised)
        }
        Ok((soft, _)) => soft,
        // A platform that cannot say: assume the common default.
        Err(_) => 1024,
    };
    usize::try_from(soft.saturating_sub(reserved) / DESCRIPTORS_PER_SESSION)
        .unwrap_or(MAX_WINDOW)
        .clamp(1, MAX_WINDOW)
}

/// A run's sessions on emulated links on this host, as [`Executor`]
/// prepares and times them.
#[derive(Clone, Copy, Debug)]
pub struct Emulation<'a> {
    /// The peer executable.
    pub program: &'a Path,
    /// The host's usable TCP window.
    pub window: u64,
    /// The host's buffer payload share.
    pub payload_share: f64,
}

impl Executor for Emulation<'_> {
    type Prepared = Ready;

    /// Prepares the session in its slot ([`prepare`]).
    fn prepare(&self, session: &Session, slot: usize) -> Result<Ready> {
        let window = window(&session.point, self.window, self.payload_share);
        prepare(self.program, slot, &session.point, window)
    }

    /// Times the prepared session ([`finish`]).
    fn finish(&self, _session: &Session, prepared: Ready) -> Result<Measured> {
        finish(prepared)
    }

    /// The point's two queue limits.
    fn queues(&self, session: &Session) -> Option<(u64, u64)> {
        let window = window(&session.point, self.window, self.payload_share);
        queues(&session.point, window)
            .ok()
            .map(|(a, b)| (a.limit_packets, b.limit_packets))
    }
}

/// The smallest warm-up transfer: enough to grow any congestion window past
/// its initial ten segments many times over.
const MIN_WARM_BYTES: u64 = 1 << 20;

/// The largest warm-up transfer.
const MAX_WARM_BYTES: u64 = 256 << 20;

/// The window a connection in a session of `point` can use: the raised
/// limit's share if the point raises it, else the host's usable window.
pub fn window(point: &Point, host_window: u64, payload_share: f64) -> u64 {
    match point.tcp_buffers {
        TcpBuffers::Host => host_window,
        TcpBuffers::Raised(bytes) => (bytes as f64 * payload_share) as u64,
    }
}

/// Bytes each side sends before a warm session.
///
/// The warm-up carries two bandwidth-delay products, so slow start ends and
/// the window opens to what the link sustains, within [`MIN_WARM_BYTES`]
/// and [`MAX_WARM_BYTES`]; on an uncapped link, two windows.
pub fn warm_bytes(point: &Point, window: u64) -> u64 {
    let Link::Emulated { rate, rtt } = point.link else {
        return MIN_WARM_BYTES;
    };
    let bytes = match rate.fastest_bytes_per_second() {
        Some(bytes_per_second) => 2.0 * bytes_per_second * rtt.seconds(),
        None => 2.0 * window as f64,
    };
    (bytes as u64).clamp(MIN_WARM_BYTES, MAX_WARM_BYTES)
}

/// The two directions' queues for `point`.
pub fn queues(point: &Point, window: u64) -> Result<(Netem, Netem)> {
    let Link::Emulated { rate, rtt } = point.link else {
        bail!("an emulated session needs an emulated link");
    };
    Ok((
        Netem::for_direction(rate.a_to_b, rtt, window),
        Netem::for_direction(rate.b_to_a, rtt, window),
    ))
}

/// A session prepared: both peers running and idle, the pair in place, and
/// the link shaped.
#[derive(Debug)]
pub struct Ready {
    /// Side A's process.
    a: Namespaced,
    /// Side B's process.
    b: Namespaced,
    /// The preparation's report.
    prepared: Prepared,
}

/// Prepares a session of `point` in slot `slot`, with `program` as the peer
/// executable, and reads what each process used doing it.
///
/// The slot names the link's interfaces only while they are created in
/// the sandbox, before each end moves to its peer's namespace, so
/// preparations running at once need distinct slots, and a prepared
/// session holds none.
///
/// # Errors
///
/// If a namespace, link, or shaping step fails (the error names the
/// command), or the preparation fails.
pub fn prepare(program: &Path, slot: usize, point: &Point, window: u64) -> Result<Ready> {
    let mut a = Namespaced::spawn(program, &["peer"])?;
    let mut b = Namespaced::spawn(program, &["peer"])?;
    netns::connect(slot, a.pid(), b.pid())?;
    if let TcpBuffers::Raised(bytes) = point.tcp_buffers {
        netns::raise_tcp_buffers(a.pid(), bytes)?;
        netns::raise_tcp_buffers(b.pid(), bytes)?;
    }
    let mut prepared = driver::prepare(
        point,
        warm_bytes(point, window),
        &mut a,
        &mut b,
        addresses(),
    )?;
    let (a_to_b, b_to_a) = queues(point, window)?;
    netns::shape(slot, a.pid(), b.pid(), &a_to_b, &b_to_a)?;
    prepared.usage = process_usage(a.pid())
        .zip(process_usage(b.pid()))
        .map(|(a, b)| [a, b]);
    Ok(Ready { a, b, prepared })
}

/// Runs a prepared session's measured exchange, and returns once both
/// peers have exited after it, so their memory is free again; their
/// namespaces and link vanish with them.
///
/// # Errors
///
/// If a peer fails or answers out of order.
pub fn finish(mut ready: Ready) -> Result<Measured> {
    let measured = driver::finish(ready.prepared, &mut ready.a, &mut ready.b, addresses());
    // Dropping the processes waits for each to exit.
    drop(ready);
    measured
}

impl Ready {
    /// What the preparation measured.
    pub fn prepared(&self) -> &Prepared {
        &self.prepared
    }
}

/// A link probe prepared in its slot: both probe processes running on a
/// shaped link.
#[derive(Debug)]
pub struct ReadyProbe {
    /// Side A's probe process.
    a: Namespaced,
    /// Side B's probe process.
    b: Namespaced,
}

/// Prepares a raw TCP probe of `point`'s link in slot `slot`.
///
/// # Errors
///
/// If a namespace, link, or shaping step fails.
pub fn prepare_probe(
    program: &Path,
    slot: usize,
    point: &Point,
    window: u64,
) -> Result<ReadyProbe> {
    let a = Namespaced::spawn(program, &["probe-peer"])?;
    let b = Namespaced::spawn(program, &["probe-peer"])?;
    netns::connect(slot, a.pid(), b.pid())?;
    if let TcpBuffers::Raised(bytes) = point.tcp_buffers {
        netns::raise_tcp_buffers(a.pid(), bytes)?;
        netns::raise_tcp_buffers(b.pid(), bytes)?;
    }
    let (a_to_b, b_to_a) = queues(point, window)?;
    netns::shape(slot, a.pid(), b.pid(), &a_to_b, &b_to_a)?;
    Ok(ReadyProbe { a, b })
}

/// Runs a prepared probe: a bulk transfer of `bulk_bytes` from B to A, then
/// `pings` round trips.
///
/// # Errors
///
/// If a probe process fails or answers out of order.
pub fn finish_probe(mut ready: ReadyProbe, bulk_bytes: u64, pings: u32) -> Result<ProbeResult> {
    let addresses = addresses();
    send(
        &mut ready.a.stdin,
        &ProbeCommand::Listen {
            address: addresses.listen,
        },
    )?;
    let ProbeReply::Listening { port } = receive(&mut ready.a.stdout)? else {
        bail!("the probe's side A answered `listen` out of order");
    };
    let address = SocketAddr::new(addresses.connect, port);
    send(
        &mut ready.b.stdin,
        &ProbeCommand::Run {
            address,
            bulk_bytes,
            pings,
        },
    )?;
    let ProbeReply::Probed { result } = receive(&mut ready.b.stdout)? else {
        bail!("the probe's side B answered `run` out of order");
    };
    let ProbeReply::Served = receive(&mut ready.a.stdout)? else {
        bail!("the probe's side A answered the probe out of order");
    };
    Ok(result)
}
