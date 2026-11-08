//! What a peer process measures about itself around its session: CPU time,
//! heap, resident memory, and TCP retransmissions.
//!
//! # The heap meter
//!
//! The binary installs [`peak_alloc::PeakAlloc`] as its global allocator. It
//! forwards every request to the system allocator and keeps two process-wide
//! counters, live bytes and their peak, at the cost of two uncontended atomic
//! operations per allocation and one per free. A peer process runs its session
//! on one thread, so the counters see exactly the session's own heap. The peak
//! resets just before the session starts, so it reports the most the session
//! held, not the fixture build's peak.
//!
//! # Resident memory
//!
//! On Linux the process also resets the kernel's resident high-water mark
//! (writing `5` to `/proc/self/clear_refs`) and reads `VmHWM` after the
//! session. Resident memory includes pages the allocator holds without using,
//! so it is never smaller than the heap the session needed; it is what a
//! deployment's memory limit actually meets.
//!
//! # Another process's preparation
//!
//! The orchestrator also reads each peer process's resident high-water mark and
//! CPU time from outside, once its preparation finishes ([`process_usage`]).
//! The process is fresh, so its high-water mark is the most it held while
//! building or joining its replica: the bytes the run's memory bound accounts
//! for.

use std::fs;
use std::time::Duration;

use nix::sys::resource::{UsageWho, getrusage};
use peak_alloc::PeakAlloc;
use serde::{Deserialize, Serialize};

/// The process's global allocator and heap meter.
#[global_allocator]
pub static HEAP: PeakAlloc = PeakAlloc;

/// A reading of the process's own resource use at one moment.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    /// User CPU time so far.
    user: Duration,
    /// System CPU time so far.
    system: Duration,
    /// Live heap bytes.
    heap: usize,
    /// Segments this network namespace retransmitted so far, where the platform
    /// exposes the count.
    retransmits: Option<u64>,
}

/// A process's resource use over one session.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Usage {
    /// User CPU seconds.
    pub user_seconds: f64,
    /// System CPU seconds.
    pub system_seconds: f64,
    /// Live heap bytes when the session started.
    pub heap_before: u64,
    /// The most live heap bytes during the session.
    pub heap_peak: u64,
    /// Live heap bytes when the session ended.
    pub heap_after: u64,
    /// The resident high-water mark during the session, on Linux.
    pub rss_peak: Option<u64>,
    /// TCP segments retransmitted during the session, where the platform
    /// exposes the count (Linux).
    pub retransmits: Option<u64>,
}

/// Reads the process's current marks and resets both peaks, so a later
/// [`Mark::usage`] reports the peaks since now.
pub fn start() -> Mark {
    HEAP.reset_peak_usage();
    reset_rss_peak();
    let (user, system) = cpu();
    Mark {
        user,
        system,
        heap: HEAP.current_usage(),
        retransmits: retransmitted_segments(),
    }
}

impl Mark {
    /// The resource use since this mark.
    pub fn usage(&self) -> Usage {
        let (user, system) = cpu();
        Usage {
            user_seconds: user.saturating_sub(self.user).as_secs_f64(),
            system_seconds: system.saturating_sub(self.system).as_secs_f64(),
            heap_before: self.heap as u64,
            heap_peak: HEAP.peak_usage() as u64,
            heap_after: HEAP.current_usage() as u64,
            rss_peak: rss_peak(),
            retransmits: retransmitted_segments()
                .zip(self.retransmits)
                .map(|(now, then)| now.saturating_sub(then)),
        }
    }
}

/// The process's user and system CPU time so far.
fn cpu() -> (Duration, Duration) {
    match getrusage(UsageWho::RUSAGE_SELF) {
        Ok(usage) => {
            let time = |t: nix::sys::time::TimeVal| {
                Duration::new(
                    t.tv_sec().max(0) as u64,
                    (t.tv_usec().max(0) as u32) * 1_000,
                )
            };
            (time(usage.user_time()), time(usage.system_time()))
        }
        Err(_) => (Duration::ZERO, Duration::ZERO),
    }
}

/// What another process used from its start until now: its resident high-water
/// mark and its CPU time, as Linux reports them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessUsage {
    /// The resident high-water mark, in bytes.
    pub rss_peak: u64,
    /// User and system CPU seconds.
    pub cpu_seconds: f64,
}

/// What process `pid` used from its start until now, on Linux; `None`
/// elsewhere, or if the process has gone.
pub fn process_usage(pid: u32) -> Option<ProcessUsage> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let kib: u64 = status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name, in parentheses, may hold spaces, so the fields
    // count from its closing parenthesis: utime and stime are the 14th and
    // 15th fields of the line, the 12th and 13th after the name.
    let mut fields = stat.rsplit_once(')')?.1.split_whitespace().skip(11);
    let user: u64 = fields.next()?.parse().ok()?;
    let system: u64 = fields.next()?.parse().ok()?;
    let ticks = nix::unistd::sysconf(nix::unistd::SysconfVar::CLK_TCK)
        .ok()
        .flatten()
        .filter(|ticks| *ticks > 0)?;
    Some(ProcessUsage {
        rss_peak: kib * 1024,
        cpu_seconds: (user + system) as f64 / ticks as f64,
    })
}

/// Resets the kernel's resident high-water mark for this process, where the
/// platform supports it (Linux 4.0 and later).
fn reset_rss_peak() {
    if cfg!(target_os = "linux") {
        // A kernel without the reset leaves the mark at the process's
        // lifetime peak; `rss_peak` then over-reports, never under-reports.
        let _ = fs::write("/proc/self/clear_refs", "5");
    }
}

/// The resident high-water mark since the last reset, on Linux.
fn rss_peak() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

/// The TCP segments retransmitted in this process's network namespace so far,
/// from `/proc/self/net/snmp`, on Linux.
///
/// In emulation mode each peer has a namespace of its own with one connection
/// in it, so the count is that connection's. In pair mode the namespace is the
/// host's, and the count includes every other connection the host carries; the
/// report says which.
pub fn retransmitted_segments() -> Option<u64> {
    let snmp = fs::read_to_string("/proc/self/net/snmp").ok()?;
    let mut tcp = snmp.lines().filter(|line| line.starts_with("Tcp:"));
    let names = tcp.next()?;
    let values = tcp.next()?;
    let index = names
        .split_whitespace()
        .position(|name| name == "RetransSegs")?;
    values.split_whitespace().nth(index)?.parse().ok()
}
