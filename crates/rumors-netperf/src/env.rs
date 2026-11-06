//! The environment a run measures in: the host, its kernel and TCP
//! limits, and the build of this tool. A run records it once, in
//! `environment.json`, and the calibration cache keys on its fingerprint.
//!
//! # Which build ran
//!
//! A build is identified by a SHA-256 of its executable
//! ([`Build::executable`]), which covers everything compiled into the
//! tool: the library, the tool's own code, its dependencies, the compiler,
//! and the build's settings. Resuming a run and reusing a calibration both
//! compare it, so neither accepts a different build, even one that differs
//! from the run's only in the library or in uncommitted edits. The source
//! revision ([`Build::revision`]) says, for the people who read the
//! results, which commit the build came from; two builds of one revision
//! can differ, and only the digest tells them apart.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::thread;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sha3::{Digest, Sha3_256};

/// This build of the tool: its executable's digest, and what the build
/// fixed at compile time.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Build {
    /// The tool's version.
    pub version: String,
    /// The source revision the build came from, for people to read: the
    /// commit, with `-dirty` when a tracked file differed from it; the
    /// commit a tree without git names in its `COMMIT` file, marked as
    /// such; or `unknown`.
    pub revision: String,
    /// The compiler that built it.
    pub rustc: String,
    /// The target it was built for.
    pub target: String,
    /// Whether the build is optimized: `release`, or `debug` for a build
    /// with debug assertions, which the tool refuses to measure with.
    pub profile: String,
    /// The SHA-256 of the executable, in hexadecimal: the build's identity,
    /// as [the module docs](self#which-build-ran) describe. A record
    /// written by a tool that kept no digest has none.
    #[serde(default)]
    pub executable: Option<String>,
}

impl Build {
    /// This build.
    ///
    /// # Errors
    ///
    /// If the running executable cannot be read to compute its digest
    /// ([`executable_digest`]).
    pub fn current() -> Result<Self> {
        Ok(Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            revision: env!("NETPERF_REVISION").to_string(),
            rustc: env!("NETPERF_RUSTC").to_string(),
            target: env!("NETPERF_TARGET").to_string(),
            profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
            executable: Some(
                executable_digest().context("reading this executable to identify the build")?,
            ),
        })
    }
}

impl fmt::Display for Build {
    /// The build in one line: version, revision, profile, target, compiler,
    /// and the executable's digest.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rumors-netperf {} at revision `{}`, {} {}, {}, executable SHA-256 {}",
            self.version,
            self.revision,
            self.profile,
            self.target,
            self.rustc,
            self.executable.as_deref().unwrap_or("unrecorded")
        )
    }
}

/// The file of the running executable.
///
/// Linux names the image the process runs `/proc/self/exe`, which still
/// opens that image after a rebuild replaces the file at its path.
/// illumos names it `/proc/self/path/a.out`, which opens the image only
/// while its file is in place: once a rebuild replaces the file, opening
/// it fails, so the digest is an error, never another build's. Elsewhere
/// the path is the best name there is, and a replacement between the
/// process's start and this read goes unseen. Only the first read counts,
/// since [`executable_digest`] keeps its result for the process's life.
fn running_executable() -> io::Result<PathBuf> {
    if cfg!(target_os = "linux") {
        Ok(PathBuf::from("/proc/self/exe"))
    } else if cfg!(target_os = "illumos") {
        Ok(PathBuf::from("/proc/self/path/a.out"))
    } else {
        std::env::current_exe()
    }
}

/// The SHA-256 of the running executable, in lower-case hexadecimal.
///
/// The first call reads the file, which takes tens of milliseconds for a
/// release build; later calls in the same process return the same digest.
///
/// # Errors
///
/// If the executable cannot be read; a later call tries again.
pub fn executable_digest() -> io::Result<String> {
    static DIGEST: OnceLock<String> = OnceLock::new();
    if let Some(digest) = DIGEST.get() {
        return Ok(digest.clone());
    }
    let mut file = File::open(running_executable()?)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => hasher.update(&buffer[..read]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    let digest = hex(&hasher.finalize());
    Ok(DIGEST.get_or_init(|| digest).clone())
}

/// The TCP limits that bound how much a connection can keep in flight.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TcpLimits {
    /// The platform's settings that bound the window, by their own names,
    /// with their values as the platform prints them.
    pub settings: Vec<(String, String)>,
    /// The largest receive buffer autotuning may grow to, in bytes.
    pub max_receive_buffer: Option<u64>,
    /// The largest send buffer autotuning may grow to, in bytes.
    pub max_send_buffer: Option<u64>,
    /// The share of a buffer that holds payload rather than bookkeeping,
    /// as this tool estimates it for the platform.
    pub payload_share: f64,
    /// The largest window a connection can use, in bytes: the smaller
    /// buffer times the payload share. A link whose bandwidth-delay product
    /// exceeds it cannot be kept busy by one connection.
    pub usable_window: Option<u64>,
}

/// Everything a run records about where it ran.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Environment {
    /// The tool's build.
    pub build: Build,
    /// The host's name.
    pub hostname: String,
    /// The operating system.
    pub os: String,
    /// The kernel's release.
    pub kernel_release: String,
    /// The kernel's version string.
    pub kernel_version: String,
    /// The processor's model, as the platform names it.
    pub cpu_model: String,
    /// The processors this process may run on.
    pub logical_cpus: usize,
    /// Physical memory, in bytes, where the platform reports it.
    pub memory_bytes: Option<u64>,
    /// The TCP limits.
    pub tcp: TcpLimits,
    /// The congestion control algorithm new connections use, where the
    /// platform reports it.
    pub congestion_control: Option<String>,
    /// The load averages over one, five, and fifteen minutes when the run
    /// started.
    pub load_average: Option<[f64; 3]>,
}

impl Environment {
    /// Reads the environment of the current host and this build.
    ///
    /// # Errors
    ///
    /// If this build cannot be identified ([`Build::current`]).
    pub fn capture() -> Result<Self> {
        let uname = nix::sys::utsname::uname().ok();
        let field = |f: fn(&nix::sys::utsname::UtsName) -> &std::ffi::OsStr| {
            uname
                .as_ref()
                .map(|u| f(u).to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown".to_string())
        };
        Ok(Self {
            build: Build::current()?,
            hostname: field(|u| u.nodename()),
            os: field(|u| u.sysname()),
            kernel_release: field(|u| u.release()),
            kernel_version: field(|u| u.version()),
            cpu_model: cpu_model(),
            logical_cpus: thread::available_parallelism().map_or(1, |n| n.get()),
            memory_bytes: memory_bytes(),
            tcp: tcp_limits(),
            congestion_control: read_trimmed("/proc/sys/net/ipv4/tcp_congestion_control"),
            load_average: load_average(),
        })
    }

    /// Everything that identifies the environment a measurement holds for,
    /// each part named: the host, its kernel, its processors and memory,
    /// its TCP settings, and the build.
    ///
    /// The build's part is its one-line description, which names the
    /// executable's digest beside the revision. The load is left out, since
    /// it changes from run to run on any host.
    fn identity(&self) -> [(&'static str, String); 9] {
        [
            ("host name", self.hostname.clone()),
            ("operating system", self.os.clone()),
            ("kernel release", self.kernel_release.clone()),
            ("kernel version", self.kernel_version.clone()),
            ("processor", self.cpu_model.clone()),
            ("logical processors", self.logical_cpus.to_string()),
            ("memory", format!("{:?}", self.memory_bytes)),
            ("TCP settings", format!("{:?}", self.tcp.settings)),
            ("build", self.build.to_string()),
        ]
    }

    /// A digest of everything that could change a calibration's verdict:
    /// the parts [`Environment::differences`] compares.
    pub fn fingerprint(&self) -> String {
        let mut hasher = Sha3_256::new();
        for (_, part) in self.identity() {
            hasher.update(part.as_bytes());
            hasher.update([0]);
        }
        hex(&hasher.finalize()[..8])
    }

    /// Each part of the environment's identity in which `self` differs
    /// from `other`, naming the part and both values.
    ///
    /// It is empty exactly when the two have the same
    /// [`Environment::fingerprint`], up to a collision of the digest.
    pub fn differences(&self, other: &Environment) -> Vec<String> {
        self.identity()
            .into_iter()
            .zip(other.identity())
            .filter(|((_, ours), (_, theirs))| ours != theirs)
            .map(|((name, ours), (_, theirs))| {
                format!("the {name} is `{ours}` here but `{theirs}` in the run")
            })
            .collect()
    }
}

/// Lower-case hexadecimal.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A file's contents, trimmed, if it can be read.
fn read_trimmed(path: &str) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// A command's standard output, trimmed, if it ran and succeeded.
pub fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The processor model.
fn cpu_model() -> String {
    if let Ok(info) = fs::read_to_string("/proc/cpuinfo") {
        let field = |key: &str| {
            info.lines()
                .find(|line| line.starts_with(key))
                .and_then(|line| line.split_once(':'))
                .map(|(_, value)| value.trim().to_string())
        };
        if let Some(model) = field("model name")
            .or_else(|| field("Hardware"))
            .or_else(|| field("Model"))
        {
            return model;
        }
        // Arm kernels name the processor only by its implementer and part
        // codes.
        if let (Some(implementer), Some(part)) = (field("CPU implementer"), field("CPU part")) {
            return format!(
                "{} (implementer {implementer}, part {part})",
                std::env::consts::ARCH
            );
        }
    }
    command_output("sysctl", &["-n", "machdep.cpu.brand_string"])
        .or_else(|| {
            command_output("psrinfo", &["-pv"])
                .and_then(|out| out.lines().last().map(|line| line.trim().to_string()))
        })
        .unwrap_or_else(|| std::env::consts::ARCH.to_string())
}

/// Physical memory in bytes.
fn memory_bytes() -> Option<u64> {
    if let Ok(info) = fs::read_to_string("/proc/meminfo") {
        let kib: u64 = info
            .lines()
            .find(|line| line.starts_with("MemTotal:"))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()?;
        return Some(kib * 1024);
    }
    if let Some(bytes) = command_output("sysctl", &["-n", "hw.memsize"]) {
        return bytes.parse().ok();
    }
    // illumos: `prtconf -m` prints megabytes.
    command_output("prtconf", &["-m"])?
        .parse::<u64>()
        .ok()
        .map(|mib| mib << 20)
}

/// The load averages, from `/proc/loadavg` or from `uptime`'s last three
/// fields.
pub fn load_average() -> Option<[f64; 3]> {
    let text = read_trimmed("/proc/loadavg").or_else(|| {
        let out = command_output("uptime", &[])?;
        let (_, tail) = out.rsplit_once("load average")?;
        Some(tail.trim_start_matches(['s', ':', ' ']).replace(',', " "))
    })?;
    let mut fields = text.split_whitespace().map(|f| f.parse::<f64>());
    Some([
        fields.next()?.ok()?,
        fields.next()?.ok()?,
        fields.next()?.ok()?,
    ])
}

/// The share of a Linux socket buffer that holds payload: the kernel
/// reserves `1 / 2^tcp_adv_win_scale` of the space for bookkeeping.
///
/// Kernels without that setting measure the ratio per connection and start
/// at a half, which this tool assumes, since it is the smallest such
/// kernels use before they have measured.
fn linux_payload_share() -> f64 {
    match read_trimmed("/proc/sys/net/ipv4/tcp_adv_win_scale").and_then(|v| v.parse::<i32>().ok()) {
        Some(scale) if scale > 0 => 1.0 - 0.5f64.powi(scale),
        Some(scale) if scale < 0 => 0.5f64.powi(-scale),
        _ => 0.5,
    }
}

/// The maximum (third) field of a Linux `tcp_rmem` or `tcp_wmem` setting.
pub fn third_field(value: &str) -> Option<u64> {
    value.split_whitespace().nth(2)?.parse().ok()
}

/// Reads the TCP limits for this platform.
pub fn tcp_limits() -> TcpLimits {
    let mut limits = TcpLimits {
        payload_share: 1.0,
        ..TcpLimits::default()
    };
    if let Some(rmem) = read_trimmed("/proc/sys/net/ipv4/tcp_rmem") {
        let wmem = read_trimmed("/proc/sys/net/ipv4/tcp_wmem").unwrap_or_default();
        limits.max_receive_buffer = third_field(&rmem);
        limits.max_send_buffer = third_field(&wmem);
        limits.payload_share = linux_payload_share();
        limits.settings.push(("net.ipv4.tcp_rmem".into(), rmem));
        limits.settings.push(("net.ipv4.tcp_wmem".into(), wmem));
        for name in [
            "tcp_adv_win_scale",
            "tcp_window_scaling",
            "tcp_slow_start_after_idle",
        ] {
            if let Some(value) = read_trimmed(&format!("/proc/sys/net/ipv4/{name}")) {
                limits.settings.push((format!("net.ipv4.{name}"), value));
            }
        }
    } else if let Some(receive) = command_output("sysctl", &["-n", "net.inet.tcp.autorcvbufmax"]) {
        // macOS: autotuning grows each buffer up to these limits.
        let send =
            command_output("sysctl", &["-n", "net.inet.tcp.autosndbufmax"]).unwrap_or_default();
        limits.max_receive_buffer = receive.parse().ok();
        limits.max_send_buffer = send.parse().ok();
        limits
            .settings
            .push(("net.inet.tcp.autorcvbufmax".into(), receive));
        limits
            .settings
            .push(("net.inet.tcp.autosndbufmax".into(), send));
        if let Some(max) = command_output("sysctl", &["-n", "kern.ipc.maxsockbuf"]) {
            limits.settings.push(("kern.ipc.maxsockbuf".into(), max));
        }
    } else if let Some(receive) = command_output(
        "ipadm",
        &["show-prop", "-c", "-o", "current", "-p", "recv_buf", "tcp"],
    ) {
        // illumos: a connection's buffers start at these sizes, and the
        // application may raise them to `max_buf`.
        let send = command_output(
            "ipadm",
            &["show-prop", "-c", "-o", "current", "-p", "send_buf", "tcp"],
        )
        .unwrap_or_default();
        let max = command_output(
            "ipadm",
            &["show-prop", "-c", "-o", "current", "-p", "max_buf", "tcp"],
        )
        .unwrap_or_default();
        limits.max_receive_buffer = receive.parse().ok();
        limits.max_send_buffer = send.parse().ok();
        limits.settings.push(("tcp recv_buf".into(), receive));
        limits.settings.push(("tcp send_buf".into(), send));
        limits.settings.push(("tcp max_buf".into(), max));
    }
    limits.usable_window = limits
        .max_receive_buffer
        .zip(limits.max_send_buffer)
        .map(|(r, s)| (r.min(s) as f64 * limits.payload_share) as u64);
    limits
}

#[cfg(test)]
mod tests;
