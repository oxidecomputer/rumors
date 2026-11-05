# rumors-netperf

<!-- cargo-rdme start -->

Measure how fast [Rumors](https://github.com/oxidecomputer/rumors)
replicas synchronize over network links, on your own hardware.

`rumors-netperf` builds pairs of replicas that differ as you describe,
runs one gossip session between them over TCP, and records how long it
took, how many bytes crossed in each direction, how much CPU and memory
each side used, and why the session took as long as it did: whether the
link's bandwidth, its round trip, or the computation set the pace. It
measures the library as an application uses it, a production build
with no test hooks, and writes a report you can read on its own.

Use it to size a deployment: to learn how long replicas of your size,
diverged as yours diverge, take to converge over your links, and what
memory budget and TCP settings let them do it as fast as the link
allows.

## Quick start

```text
cargo build --release -p rumors-netperf
target/release/rumors-netperf doctor
target/release/rumors-netperf run --grid crates/rumors-netperf/grids/quick.toml
```

`doctor` says what this host can do and, for anything it cannot, what to
change. `run` measures the grid file it is given, here the quick grid of
a few minutes that ships beside this crate, prints progress as it goes,
and leaves a results directory (`netperf-<time>/` unless you pass
`--out`) holding `report.md`.

Build the tool on its own, as above. It refuses to measure with a debug
build, and with a library that carries its test-only `test-internals`
feature, which cargo compiles in when one command builds the tool beside
a crate that enables it.

## Two modes

- **Emulation** (`run`, Linux only) measures across emulated links on one
  host. Each session's two peers run as separate processes in network
  namespaces of their own, joined by a virtual link that `netem` shapes
  to a rate and a round trip in each direction. Many links, from an
  intra-rack backplane to an intercontinental path, fit in one run. It
  needs no root: it runs in an unprivileged user namespace and never
  touches the host's network.
- **Pair** (`serve` on one host, `connect` on another) measures over the
  network between two hosts, on any platform Rumors supports, with
  nothing shaped. It first probes the network's round trip and goodput,
  and reads every session against them.

Both modes run the same sessions and write the same report.

A third mode, **in-process** (`duplex`, Linux only), answers a narrower
question: what a session costs each side's processor, with no link and no
kernel TCP stack. It builds each point's pair as emulation does, runs
both sides in one process over an in-memory duplex stream, each on its
own thread, and prints each side's thread CPU time, user and system
separately, with the session's counts, one line per session. It writes
no report, and ignores the link and connection axes.

## Describing what to measure

A *point* is one replica pair, its message sizes, the settings each
`Peer` runs with, and the link. A pair is described by
five counts:

| count | meaning |
|---|---|
| `shared` (*s*) | messages live on both sides |
| `insert_a`, `insert_b` (*i_A*, *i_B*) | messages one side inserted that the other never saw |
| `redact_a`, `redact_b` (*r_A*, *r_B*) | messages one side redacted that the other still holds |

Message sizes follow a lognormal distribution set by its mean and its
coefficient of variation (standard deviation over mean); a coefficient
of zero is a fixed size. Draws above the cap, 64 KiB unless you raise
it, are redrawn rather than clamped, and the results count them.

Every setting `Peer` offers that can change a session is a parameter
too: each side's memory budget and target message size, the payload
depth limit, and a session deadline. So is the transport: by default
TLS 1.3 over TCP, as deployments run it, and plain TCP with `tls = off`,
each set up as the library advises. Every session's row records the
values it ran with, and counts the bytes and the write and read
operations each side's socket saw beneath every buffer and TLS: over
TLS the bytes include each record's framing, and on plain TCP the
operations are its system calls.

By default each session runs on a *warm* connection: one already open,
which has carried a bulk transfer each way so its congestion window is
open, and the session's clock starts after that setup. That is the
session an application sees when it keeps a connection to each partner
with `Rumors::gossip`, as it should. Choose `fresh` instead if your
application connects in order to synchronize: the session then opens its
connection, and its time includes connecting, with the TLS handshake
over TLS, and slow start.

On the command line, each parameter is a flag that takes one value or a
comma-separated list, and the run measures every combination:

```text
rumors-netperf run --shared 1e5 --insert-a 1000 --insert-b 1000 \
    --rate 100Mbit,1Gbit,uncapped --rtt 1ms,10ms,100ms --budget default,32MiB
```

For anything larger, write a grid file: groups of axes, each group
measuring every combination of its axes, and a `pairs` array for counts
that move together. Each `pairs` entry describes its replica pair whole:
any count it leaves out is zero. `rumors-netperf run --help` lists
every axis.

```toml
repetitions = 5

[[group]]
name = "a peer that fell behind"
pairs = [
    { shared = 100000, insert_a = 1000 },
    { shared = 1000000, insert_a = 100000 },
]
size_mean = 200
size_cv = 1.0
rate = ["100Mbit", "1Gbit", "uncapped"]
rtt = ["0.1ms", "10ms", "100ms", "250ms"]
budget = ["default", "128MiB", "32MiB"]
```

```text
rumors-netperf run --grid my-grid.toml
```

The grid the Rumors project publishes its own figures from ships beside
this crate as `grids/reference.toml`; running it is how you check your
hardware against those figures.

## Reading the report

For each point, the report gives the median session time with an exact
confidence interval, and names what bound it: *bandwidth* when the busier
direction's bytes over the link's rate explain the time, *latency* when
the round trips the protocol needs do, and *computation* when the
busier side's CPU time does. It compares the round trips the session
spent against the fewest the protocol needs, shows how busy each
direction of the link was, how much slower each smaller budget made the
session, how much memory the session held against its budget, and, for
links whose bandwidth-delay product exceeds what the host's TCP limits
let one connection keep in flight, how much raising those limits
changes.

## TCP buffer limits

A TCP connection keeps at most one window in flight per round trip, and
the host's buffer limits cap the window. From those limits the tool
computes the largest window a connection can use, and reruns, with the
limits raised, exactly the points whose bandwidth-delay product and
busier direction both exceed it. It raises them inside the session's
own network namespaces only: the host keeps its settings, and the report
tells you whether yours need raising.

## Parallelism

Each session has two parts. Its *preparation* builds the pair and
moves side B's replica to its process, which is most of a run's work
and measures nothing; its *timed session* is the gossip the row
measures. The tool prepares many sessions at once and times them apart
from any preparation, alternating between the two: it prepares a
window of the next sessions to time, then times them in the run's
order, then prepares the next window. A prepared session waits with
its replicas in its two processes, idle, until its turn, so a window
holds only as many sessions as fit the memory bound: half of physical
memory by default, by a memory model calibration fits, or
`--prepare-memory`.

Timed sessions that share a host can slow each other's computation or
distort the emulated links. Before a run, the tool calibrates: it runs
the run's heaviest session on its fastest link alone several times to
measure its natural spread, then 2, 4, 8, … at once, and times as many
sessions at once as the largest count whose sessions stay within that
spread. It checks the emulator with raw TCP transfers on the fastest
shaped link, independently of Rumors, and without TLS even when the
sessions run over it: that check judges the link alone, while TLS's
cost belongs to the sessions, which measure it. It also measures how
many processors one preparation keeps busy, and prepares as many
sessions at once as fill the host's processors (or `--prepare-jobs`).
During the run it repeats the heaviest session as a sentinel and flags
drift if the host slows down. Calibrations are cached per host and
build, a build being identified by a digest of its executable, so a
change to the library or to the tool calibrates afresh; `--jobs N`
skips calibration, and `--recalibrate` discards the cache.
`rumors-netperf calibrate --grid` calibrates without running the grid
and predicts how long a run of it takes.

## Splitting a run across hosts

When the emulator itself saturates while processors sit idle, the
kernel's shaping is the limit, and the tool says so. Split a large run
across hosts with `--shard k/n`: the same grid on every host, `k` from 1
to `n`. Each shard calibrates on its own host and writes its own results
directory, and `check` and `report` read the directories as one run.

A point's repetitions fall to different shards, so the hosts must
measure alike: they must agree in everything `environment.json` records
but their names and their load, which is the operating system and
kernel, the processors, the memory, the TCP settings and congestion
control, and the build of the tool, by its executable's digest.
Two builds of one revision can differ, so build once and copy the
executable. For two hosts, `host-1` with a checkout and `host-2` with
none:

```text
host-1$ cargo build --release --locked -p rumors-netperf
host-1$ scp target/release/rumors-netperf crates/rumors-netperf/grids/reference.toml host-2:
host-1$ target/release/rumors-netperf doctor
host-2$ ./rumors-netperf doctor
host-1$ target/release/rumors-netperf run --grid crates/rumors-netperf/grids/reference.toml --shard 1/2 --out reference-1
host-2$ ./rumors-netperf run --grid reference.toml --shard 2/2 --out reference-2
host-1$ scp -r host-2:reference-2 .
host-1$ target/release/rumors-netperf check reference-1 reference-2
```

Each `doctor` prints its host's build, whose executable digests must
match. `check` judges the two directories as one run and renders their
one report into `reference-1/check/`; `report` renders it anywhere else:

```text
rumors-netperf report reference-1 reference-2 --out reference
```

## Checking a run

`check` says whether a run, or every shard of one, finished cleanly. It
judges facts any clean run satisfies on any host, never thresholds on
timings, and exits non-zero, naming each failure, unless all of them
hold:

- the results are whole: the shards given make up one run exactly once,
  and each finished;
- the shards measured alike: every directory's environment matches the
  first's in all but the host's name, the build's executable digest
  included;
- every session the run planned has exactly one row, across every part
  of a run that was cut short and resumed, and every point the grid
  skips is recorded with its reason;
- no session failed;
- every side that requested a budget below the library's minimum,
  `MIN_SYNC_MEMORY_BUDGET`, reports
  that it ran at that minimum;
- if the host's TCP limits leave a window smaller than some point keeps
  in flight, at least one rerun with raised limits finished;
- every row records when its preparation and its timed session ran, no
  timed session overlapped any preparation, and no more timed sessions
  overlapped than the run's job count;
- the report renders; `check` renders it into `check/` inside the first
  directory unless you pass `--out`.

```text
rumors-netperf check netperf-shard-1 netperf-shard-2
```

## Finishing a run cut short

A run records each session as it finishes, appending its row to
`sessions.csv` and syncing it to the disk, so a run cut short, by a
killed process or a crash of the host, keeps every session it
finished. To finish it, resume it in its results directory:

```text
rumors-netperf run --resume netperf-1767225600
```

A resumed run takes no grid, shard, or job flags: it runs the
directory's plan as the run began it, with the same job counts,
memory bound, and calibration, and runs every session the directory
has no row for, appending to `sessions.csv`. A session that failed has
a row, so it is not run again. If the run was cut short while writing a
row, the resumed run removes that torn row, says so, and runs its
session again.
It refuses to resume with another build of the tool, on another host,
or under another kernel, naming what differs, since those rows would
not measure the same thing as the rest. A build is identified by a
SHA-256 of its executable, so any change to the library, the tool, or
the compiler makes another build, uncommitted edits included. `run.json` records each
resumption, and the report and `check` read the parts as one run.

The smoke grid that ships beside this crate, `grids/smoke.toml`, runs
every kind of point the reference grid does, each group at its
smallest, beside the reference run's heaviest session, and calibrates
as a reference run does: run it and check it before a long run.

## Setting up a Linux host

Emulation needs `unshare` and `nsenter` (util-linux), `ip` and `tc`
(iproute2), `ethtool`, and `sysctl` (procps); unprivileged user
namespaces; and the `sch_netem` kernel module. `doctor` checks each and
names the fix. In Docker, run the tool as an ordinary user in a
container started with
`--security-opt seccomp=unconfined --security-opt systempaths=unconfined`:
`NET_ADMIN` alone does not allow creating network namespaces or
raising TCP limits inside them.

## Pair mode

```text
server$ rumors-netperf serve
client$ rumors-netperf connect server.example:7979 --grid my-grid.toml
```

Both hosts must run the tool built from the same revision; the server
refuses a client whose version or revision differs. Side A of every
session runs on the server, side B on the client, and the client writes
the results. The server only needs its port reachable; each session also
opens one ephemeral port on it. Rate and round-trip axes do not apply,
since the network sets both. The server starts one process at a time, so
pair mode prepares each session just before timing it.

## What the measurements are

- Every session builds a fresh, independent pair: the trees two
  sessions compare are separate random draws.
- A timed session runs with no preparation beside it, and with no other
  timed session beyond calibration's job count. Its replicas were built
  a while before, while other sessions were prepared, so it starts with
  processor caches that hold nothing of its own. Each row's
  `build_seconds` and `fetch_seconds` were measured with other
  preparations sharing the host; the rows' intervals say which.
- Timings come from an optimized build of the tool and a default-feature
  build of the library. The tool counts heap through a thin wrapper
  around the system allocator, which adds two uncontended atomic
  operations per allocation.
- Emulated links add delay and a rate, not loss; the queue in front of
  each rate holds one bandwidth-delay product of packets, as a
  router's buffer would.
- `environment.json` records the host, its kernel and TCP limits, and
  the build; every row of `sessions.csv` records its settings, so results
  from different hosts or builds can be compared outside the tool.
  `plan.json` records what the run set out to measure, the points its
  grid skips among them, with their reasons.

<!-- cargo-rdme end -->
