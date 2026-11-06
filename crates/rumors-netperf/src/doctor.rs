//! `doctor`: what this host can do, and for each thing it cannot, why and
//! what to change.

use std::process::Command;

use crate::env::Environment;
use crate::netns::is_root;
use crate::units::human_bytes;

/// One check's verdict.
#[derive(Debug)]
struct Check {
    /// What was checked.
    name: &'static str,
    /// Whether it passed.
    ok: bool,
    /// What was found.
    detail: String,
    /// What to do about a failure.
    fix: Option<String>,
}

/// Runs `script` with `sh` inside a throwaway sandbox like the one
/// emulation uses, returning its standard error on failure.
fn in_sandbox(script: &str) -> Result<(), String> {
    let mut command = Command::new("unshare");
    if !is_root() {
        command.args(["--user", "--map-root-user"]);
    }
    let output = command
        .args(["--net", "--", "sh", "-c", script])
        .output()
        .map_err(|e| format!("could not run `unshare`: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Whether `program` runs at all.
fn installed(program: &str, probe: &str) -> bool {
    Command::new(program).arg(probe).output().is_ok()
}

/// Runs every check, prints the verdicts, and returns whether emulation
/// mode can run here.
pub fn doctor() -> bool {
    let environment = match Environment::capture() {
        Ok(environment) => environment,
        Err(error) => {
            println!("cannot identify this build of rumors-netperf: {error:#}");
            return false;
        }
    };
    println!("{}", environment.build);
    println!(
        "host {}: {} {}, {}, {} processors, {}",
        environment.hostname,
        environment.os,
        environment.kernel_release,
        environment.cpu_model,
        environment.logical_cpus,
        environment
            .memory_bytes
            .map_or("memory unknown".into(), |b| format!(
                "{} memory",
                human_bytes(b as f64)
            ))
    );
    let mut checks = Vec::new();
    checks.push(Check {
        name: "optimized build",
        ok: environment.build.profile == "release",
        detail: format!("this binary is a {} build", environment.build.profile),
        fix: Some("build with `cargo build --release -p rumors-netperf`; debug timings say nothing about production".into()),
    });
    checks.push(Check {
        name: "production library",
        ok: !rumors::TEST_INTERNALS,
        detail: if rumors::TEST_INTERNALS {
            "the library carries its `test-internals` feature".into()
        } else {
            "the library carries no test-only feature".into()
        },
        fix: Some("build the tool on its own, with `cargo build --release -p rumors-netperf`, never beside a crate that enables `test-internals`".into()),
    });
    let linux = cfg!(target_os = "linux");
    if linux {
        checks.extend(linux_checks());
    }
    let tcp = &environment.tcp;
    println!();
    for check in &checks {
        println!(
            "{} {}: {}",
            if check.ok { "ok  " } else { "FAIL" },
            check.name,
            check.detail
        );
        if !check.ok
            && let Some(fix) = &check.fix
        {
            println!("     fix: {fix}");
        }
    }
    println!();
    println!("TCP limits:");
    for (name, value) in &tcp.settings {
        println!("  {name} = {value}");
    }
    match tcp.usable_window {
        Some(window) => {
            println!(
                "  one connection can keep about {} in flight; links whose bandwidth-delay product exceeds that",
                human_bytes(window as f64)
            );
            println!(
                "  (for example {:.0} Mbit/s at 100 ms, or {:.0} Mbit/s at 250 ms) cannot be kept busy at these limits.",
                window as f64 * 8.0 / 0.1 / 1e6,
                window as f64 * 8.0 / 0.25 / 1e6
            );
            if linux {
                println!(
                    "  `run` reruns such points with the limits raised inside their own namespaces, never on the host."
                );
            }
        }
        None => println!("  unknown on this platform"),
    }
    let emulation = linux && checks.iter().all(|c| c.ok || c.name == "optimized build");
    println!();
    println!(
        "emulation mode (`run`, `calibrate`): {}",
        if emulation {
            "available"
        } else if linux {
            "unavailable until the failures above are fixed"
        } else {
            "unavailable: it needs Linux network namespaces"
        }
    );
    println!("pair mode (`serve`, `connect`): available");
    emulation
}

/// The checks emulation mode needs on Linux.
fn linux_checks() -> Vec<Check> {
    let mut checks = Vec::new();
    let tools = [
        ("unshare", "--version", "util-linux"),
        ("nsenter", "--version", "util-linux"),
        ("ip", "-V", "iproute2"),
        ("tc", "-V", "iproute2"),
        ("ethtool", "--version", "ethtool"),
        ("sysctl", "--version", "procps"),
    ];
    let missing: Vec<&str> = tools
        .iter()
        .filter(|(p, a, _)| !installed(p, a))
        .map(|(p, _, _)| *p)
        .collect();
    checks.push(Check {
        name: "tools",
        ok: missing.is_empty(),
        detail: if missing.is_empty() {
            "unshare, nsenter, ip, tc, ethtool, and sysctl are installed".into()
        } else {
            format!("missing: {}", missing.join(", "))
        },
        fix: Some("install util-linux, iproute2, ethtool, and procps (Debian: `apt install util-linux iproute2 ethtool procps`)".into()),
    });
    let namespaces = in_sandbox("true");
    checks.push(Check {
        name: "network namespaces",
        ok: namespaces.is_ok(),
        detail: match &namespaces {
            Ok(()) if is_root() => "root can create network namespaces".into(),
            Ok(()) => "an unprivileged user namespace with its own network namespace works".into(),
            Err(error) => format!("creating one failed: {error}"),
        },
        fix: Some(
            "allow unprivileged user namespaces (`sysctl kernel.unprivileged_userns_clone=1` on Debian kernels, \
             `user.max_user_namespaces` above zero, and on Ubuntu `kernel.apparmor_restrict_unprivileged_userns=0`), \
             or run as root; in Docker, run with `--security-opt seccomp=unconfined --security-opt systempaths=unconfined` \
             as an ordinary user, or with `--cap-add NET_ADMIN --cap-add SYS_ADMIN --security-opt systempaths=unconfined` as root"
                .into(),
        ),
    });
    if namespaces.is_err() {
        return checks;
    }
    let veth = in_sandbox(
        "ip link add nbdoctora type veth peer name nbdoctorb && ethtool -K nbdoctora tso off gso off gro off",
    );
    checks.push(Check {
        name: "virtual links",
        ok: veth.is_ok(),
        detail: veth.as_ref().map_or_else(
            |e| format!("creating a veth failed: {e}"),
            |()| "veth pairs and offload settings work".into(),
        ),
        fix: Some("the kernel needs veth support (`modprobe veth`)".into()),
    });
    let netem = in_sandbox(
        "ip link set lo up && tc qdisc add dev lo root netem delay 1ms rate 10mbit limit 100",
    );
    checks.push(Check {
        name: "netem",
        ok: netem.is_ok(),
        detail: netem.as_ref().map_or_else(|e| format!("installing netem failed: {e}"), |()| "netem shapes rate and delay".into()),
        fix: Some(
            "load the netem queueing discipline on the host (`sudo modprobe sch_netem`; on some distributions it ships \
             in a separate package such as `kernel-modules-extra` or `linux-modules-extra-$(uname -r)`); \
             inside a user namespace the kernel cannot load it on demand"
                .into(),
        ),
    });
    let sysctl = in_sandbox("echo '4096 131072 67108864' > /proc/sys/net/ipv4/tcp_rmem");
    checks.push(Check {
        name: "per-namespace TCP limits",
        ok: sysctl.is_ok(),
        detail: sysctl.as_ref().map_or_else(
            |e| format!("raising tcp_rmem inside a namespace failed: {e}"),
            |()| "TCP buffer limits can be raised inside a namespace without touching the host".into(),
        ),
        fix: Some(
            "the kernel must be 4.15 or later and /proc/sys writable; in Docker add `--security-opt systempaths=unconfined`"
                .into(),
        ),
    });
    checks
}
