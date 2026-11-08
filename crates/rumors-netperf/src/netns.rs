//! Linux network namespaces, virtual links, and `netem` shaping, through
//! the standard tools: `unshare` and `nsenter` from util-linux, `ip` and
//! `tc` from iproute2, `ethtool`, and `sysctl` from procps.
//!
//! # The sandbox
//!
//! Emulation never touches the host's network. The tool first re-executes
//! itself in a *sandbox*: a new network namespace, inside a new user
//! namespace when it runs unprivileged (`unshare --user --map-root-user
//! --net`), where it holds the capabilities the steps below need. Inside
//! it, each peer process starts in a network namespace of its own
//! (`unshare --net`), and the two are joined by a virtual Ethernet pair
//! (a *veth*) created in the sandbox and moved into them. When the peers
//! exit, their namespaces, the veth, and its shaping vanish with them.
//!
//! # Shaping
//!
//! Each direction is shaped on its sender's end of the veth by one `netem`
//! queue: it delays every packet by half the round trip and, unless the
//! link is uncapped, serializes packets at the link's rate. Segmentation
//! offload is off on both ends, so the queue shapes wire-sized packets
//! rather than 64 KiB bursts. [`Netem::for_direction`] derives the queue's
//! limit.

use std::fmt;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::units::{Rate, Rtt};

/// The environment variable that marks a process as already inside the
/// sandbox, so it does not re-execute itself again.
pub const SANDBOX_MARKER: &str = "RUMORS_NETPERF_SANDBOX";

/// Side A's address on every virtual link; each link lives in namespaces
/// of its own, so every link reuses it.
pub const ADDRESS_A: &str = "10.77.0.1";

/// Side B's address on every virtual link.
pub const ADDRESS_B: &str = "10.77.0.2";

/// The bytes of one full-sized packet on the veth: the default Ethernet
/// MTU.
const PACKET_BYTES: f64 = 1500.0;

/// The payload of one full-sized TCP segment over IPv4 with timestamps.
const SEGMENT_BYTES: f64 = 1448.0;

/// Packets every `netem` queue holds beyond what its derivation requires,
/// so a link whose bandwidth-delay product is a handful of packets still
/// absorbs a TCP initial window's burst without loss.
const QUEUE_FLOOR_PACKETS: u64 = 32;

/// How long a new peer process may take to enter its network namespace.
const NAMESPACE_TIMEOUT: Duration = Duration::from_secs(10);

/// Whether the current process is inside the sandbox.
pub fn in_sandbox() -> bool {
    std::env::var_os(SANDBOX_MARKER).is_some()
}

/// Whether this process runs as the host's root, which needs no user
/// namespace to create network namespaces.
pub fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

/// The command that runs `program` with `args` inside a new sandbox.
pub fn sandbox_command(program: &Path, args: &[String]) -> Command {
    let mut command = Command::new("unshare");
    if !is_root() {
        command.args(["--user", "--map-root-user"]);
    }
    command.arg("--net").arg("--").arg(program).args(args);
    command.env(SANDBOX_MARKER, "1");
    command
}

/// Runs a command to completion, failing with its command line and its
/// standard error if it fails.
pub fn run(program: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(program).args(args).output().with_context(|| {
        format!("running `{program}`: is it installed? (`rumors-netperf doctor` checks)")
    })?;
    if !output.status.success() {
        bail!(
            "`{program} {}` failed ({}): {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Runs a command inside the network namespace of process `pid`.
fn run_in(pid: u32, program: &str, args: &[&str]) -> Result<()> {
    let pid = pid.to_string();
    let mut full = vec!["--target", pid.as_str(), "--net", "--", program];
    full.extend_from_slice(args);
    run("nsenter", &full)
}

/// A process started in a network namespace of its own, spoken to over its
/// standard input and output.
#[derive(Debug)]
pub struct Namespaced {
    /// The process.
    pub child: Child,
    /// Its standard input.
    pub stdin: ChildStdin,
    /// Its standard output, line-buffered.
    pub stdout: BufReader<ChildStdout>,
}

impl Namespaced {
    /// Starts `program` with `args` in a new network namespace.
    pub fn spawn(program: &Path, args: &[&str]) -> Result<Self> {
        let mut child = Command::new("unshare")
            .arg("--net")
            .arg("--")
            .arg(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("starting a peer in a network namespace of its own")?;
        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = BufReader::new(child.stdout.take().expect("stdout is piped"));
        let spawned = Self {
            child,
            stdin,
            stdout,
        };
        spawned.await_own_namespace()?;
        Ok(spawned)
    }

    /// Waits until the process has entered its new network namespace.
    ///
    /// The process exists before `unshare` has created the namespace, and
    /// a device moved to it in that window lands in the sandbox's namespace
    /// instead, so every step that names the process's namespace waits for
    /// its namespace link to differ from this process's own.
    fn await_own_namespace(&self) -> Result<()> {
        let ours = fs::read_link("/proc/self/ns/net")
            .context("reading this process's network namespace")?;
        let path = format!("/proc/{}/ns/net", self.pid());
        let began = Instant::now();
        while began.elapsed() < NAMESPACE_TIMEOUT {
            match fs::read_link(&path) {
                Ok(theirs) if theirs != ours => return Ok(()),
                Ok(_) => thread::sleep(Duration::from_millis(1)),
                Err(error) => bail!("a peer process exited before entering its namespace: {error}"),
            }
        }
        bail!("a peer process did not enter its network namespace within {NAMESPACE_TIMEOUT:?}")
    }

    /// The process's identifier, which names its namespace to `ip` and
    /// `nsenter`.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Namespaced {
    /// Stops the process if it still runs, so its namespace goes with it.
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// The interface names of slot `slot`'s veth, A's end then B's.
fn interface_names(slot: usize) -> (String, String) {
    (format!("nb{slot}a"), format!("nb{slot}b"))
}

/// Joins the namespaces of processes `a` and `b` with a veth, addressed
/// [`ADDRESS_A`] and [`ADDRESS_B`], segmentation offload off, unshaped.
pub fn connect(slot: usize, a: u32, b: u32) -> Result<()> {
    let (end_a, end_b) = interface_names(slot);
    run(
        "ip",
        &[
            "link", "add", &end_a, "type", "veth", "peer", "name", &end_b,
        ],
    )?;
    run("ip", &["link", "set", &end_a, "netns", &a.to_string()])?;
    run("ip", &["link", "set", &end_b, "netns", &b.to_string()])?;
    for (pid, end, address) in [(a, &end_a, ADDRESS_A), (b, &end_b, ADDRESS_B)] {
        run_in(pid, "ip", &["link", "set", "lo", "up"])?;
        run_in(
            pid,
            "ip",
            &["addr", "add", &format!("{address}/24"), "dev", end],
        )?;
        run_in(
            pid,
            "ethtool",
            &["-K", end, "tso", "off", "gso", "off", "gro", "off"],
        )?;
        run_in(pid, "ip", &["link", "set", end, "up"])?;
    }
    Ok(())
}

/// One direction's `netem` queue.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Netem {
    /// The delay every packet takes: half the round trip.
    pub delay: Duration,
    /// The rate packets leave at.
    pub rate: Rate,
    /// The most packets the queue holds; it drops packets beyond them.
    pub limit_packets: u64,
}

impl Netem {
    /// The queue for one direction of a link at `rate` with round trip
    /// `rtt`, for a connection whose window can reach `window` bytes.
    ///
    /// The queue holds every packet in flight on the delay line, since
    /// `netem` keeps delayed packets in its queue. On a rate-limited
    /// direction that is `rate × rtt / 2` bytes, and the limit adds one
    /// bandwidth-delay product, `rate × rtt`, as the bottleneck's buffer,
    /// the classic sizing for a router serving long flows. On an uncapped
    /// direction nothing queues for the rate, and what the delay line holds
    /// is bounded by the window instead: the window's segments and an
    /// acknowledgement for each, which the queue therefore never overflows.
    /// Both add [`QUEUE_FLOOR_PACKETS`].
    pub fn for_direction(rate: Rate, rtt: Rtt, window: u64) -> Self {
        let delay = rtt.duration() / 2;
        let packets = match rate.bytes_per_second() {
            Some(bytes_per_second) => {
                (1.5 * bytes_per_second * rtt.seconds() / PACKET_BYTES).ceil() as u64
            }
            None => 2 * (window as f64 / SEGMENT_BYTES).ceil() as u64,
        };
        Self {
            delay,
            rate,
            limit_packets: packets + QUEUE_FLOOR_PACKETS,
        }
    }
}

impl fmt::Display for Netem {
    /// Prints the `tc` arguments that install the queue.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.tc_args().join(" "))
    }
}

impl Netem {
    /// The `tc qdisc replace` arguments after the device.
    fn tc_args(&self) -> Vec<String> {
        let mut args = vec![
            "root".to_string(),
            "netem".to_string(),
            "delay".to_string(),
            format!("{}us", self.delay.as_micros()),
            "limit".to_string(),
            self.limit_packets.to_string(),
        ];
        if let Rate::Bits(bits) = self.rate {
            args.extend(["rate".to_string(), format!("{bits}bit")]);
        }
        args
    }
}

/// Installs `netem` on the sending end of each direction of slot `slot`'s
/// veth: `a_to_b` on A's end, `b_to_a` on B's.
pub fn shape(slot: usize, a: u32, b: u32, a_to_b: &Netem, b_to_a: &Netem) -> Result<()> {
    let (end_a, end_b) = interface_names(slot);
    for (pid, end, netem) in [(a, &end_a, a_to_b), (b, &end_b, b_to_a)] {
        let mut args = vec![
            "qdisc".to_string(),
            "replace".to_string(),
            "dev".to_string(),
            end.clone(),
        ];
        args.extend(netem.tc_args());
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_in(pid, "tc", &args)?;
    }
    Ok(())
}

/// Sets the largest autotuned TCP send and receive buffer to `bytes`
/// inside the namespace of process `pid`, keeping the minimum and default
/// the namespace started with.
///
/// The host's limits are untouched: these settings belong to each network
/// namespace (Linux 4.15 and later).
pub fn raise_tcp_buffers(pid: u32, bytes: u64) -> Result<()> {
    for name in ["tcp_rmem", "tcp_wmem"] {
        let current = fs::read_to_string(format!("/proc/sys/net/ipv4/{name}"))
            .with_context(|| format!("reading net.ipv4.{name}"))?;
        let fields: Vec<&str> = current.split_whitespace().collect();
        let [min, default, _] = fields[..] else {
            bail!(
                "net.ipv4.{name} reads `{}`, not three numbers",
                current.trim()
            );
        };
        run_in(
            pid,
            "sysctl",
            &[
                "-q",
                "-w",
                &format!("net.ipv4.{name}={min} {default} {bytes}"),
            ],
        )?;
    }
    Ok(())
}

/// Cumulative processor time on this host, for the share of it spent busy.
#[derive(Clone, Copy, Debug)]
pub struct CpuTimes {
    /// Ticks spent busy, over every processor.
    busy: u64,
    /// Ticks in total, over every processor.
    total: u64,
}

impl CpuTimes {
    /// Reads the aggregate line of `/proc/stat`.
    pub fn read() -> Option<Self> {
        let stat = fs::read_to_string("/proc/stat").ok()?;
        let line = stat.lines().find(|line| line.starts_with("cpu "))?;
        let ticks: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|t| t.parse().ok())
            .collect();
        // user nice system idle iowait irq softirq steal: idle and iowait
        // are the idle ticks.
        let idle = ticks.get(3)? + ticks.get(4).copied().unwrap_or(0);
        let total: u64 = ticks.iter().take(8).sum();
        Some(Self {
            busy: total - idle,
            total,
        })
    }

    /// The share of processor time spent busy since `earlier`.
    pub fn busy_share_since(&self, earlier: &Self) -> f64 {
        let total = self.total.saturating_sub(earlier.total);
        if total == 0 {
            return 0.0;
        }
        self.busy.saturating_sub(earlier.busy) as f64 / total as f64
    }
}

#[cfg(test)]
mod tests;
