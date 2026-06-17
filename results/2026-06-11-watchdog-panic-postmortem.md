# Postmortem: `just gate` kernel-panicking the workstation

*Investigated and resolved 2026-06-11. Fix landed as `030b4ba`; guard rail as
`90df422`.*

## Summary

Running `just gate` (and, it turned out, any build that reached codegen)
hard-crashed the machine four times in twelve hours. The crashes were
deliberate kernel panics: rustc memory consumption outran the macOS jetsam
mechanism, the VM compressor exhausted its segment limit, userspace —
including `watchdogd` — starved, and after 94 seconds without check-ins the
kernel panicked the machine.

The root cause was a **monomorphization bomb** introduced the night before in
commit `abf5c3a` ("Remove unnecessary callbacks and make more traversals
synchronous where possible"): a `.collect::<Vec<_>>()` in
`src/tree/traverse/act.rs` was replaced with a lazy iterator that was then fed
into `Act`'s type-level height recursion. Each of the 32 `Height` levels wove
the previous level's iterator type into its own, the type compounded
exponentially, and rustc's monomorphization collector consumed 25+ GiB *per
leaf crate* that links `rumors`. Four such rustc processes in a parallel
build allocate on the order of 100 GiB in tens of seconds — faster than
jetsam can respond on a 128 GiB machine.

Two properties made it vicious:

- `cargo check` and clippy never run codegen, so every fast-feedback signal
  stayed green. Only test/example/bench builds detonated, which is exactly
  what `just gate` reaches after its check-based stages pass.
- The blowup happens in *downstream* crates: `librumors` itself compiled
  fine, because the generic `Act::act` is only instantiated when a leaf crate
  uses the tree with concrete types.

The same change also introduced a correctness bug: the `Forget` short-circuit
consumed actions from the very iterator it then passed into the recursion, so
mixed action groups arrived missing everything up to their first non-`Forget`
action. Restoring the collect fixed both.

## Timeline (2026-06-11, EDT)

| Time | Event |
|------|-------|
| 00:32–01:41 | Commit series lands; `abf5c3a` (01:22) arms the bomb |
| 00:58 | `just gate` #1 passes — it ran against pre-bomb code |
| ~01:16–01:18 | Overnight Claude session runs `cargo nextest run` against the working tree already containing the (then-uncommitted) `abf5c3a` changes; user runs `killall cargo` at 01:18:31 fighting the hang |
| ~01:19 | First machine reset (boot 01:20:06) |
| 08:25–08:28 | A long-running Claude session (started 01:31) runs `just gate` |
| 08:32:46 | **Watchdog panic #1** (after 7.2 h of otherwise healthy uptime) |
| ~08:54 | Boot; user runs `claude -c` at 08:55, resuming that same session, which immediately re-runs `just gate` at 08:56 |
| ~08:59 | Crash #3 (boot 08:59:48) |
| 09:02 | User's `just gate` #2 wedges by 09:03:28; killed by hand ~09:08 |
| 09:10 | `sudo spctl developer-mode enable-terminal` — chasing exec stalls that were actually paging symptoms (red herring) |
| 09:12:19 | `just gate` #3; build artifacts freeze at 09:12:30 |
| 09:15:59 | **Watchdog panic #2** (88 watchdogd check-ins ≈ 15 min of uptime; 81 swapfiles accumulated in a 16-minute session) |
| 09:16:18 | Final boot; investigation begins 09:34 |

All four crashes attribute to builds of post-`abf5c3a` code. The two
recovered panic reports are identical in kind:

```
panic(cpu N caller ...): watchdog timeout: no checkins from watchdogd in 94 seconds
...
Compressor Info: 26% of compressed pages limit (OK) and 100% of segments
limit (BAD) with 81 swapfiles and OK swap space
```

## Diagnostic chain

1. **System evidence of panic, not freeze or power loss.** NVRAM held
   `panicmedic-telemetry`/`panicmedic-timestamps` (Apple Silicon panic
   recovery), decoding to the 09:16 recovery boot.
2. **Build forensics.** `target/` mtimes showed the 09:12 gate's incremental
   dep-graph directories frozen mid-write (`-working` state) at 09:12:29–30 —
   the machine died ~15 s into the *compile* phase, before any test ran. The
   ~3.5-minute gap to the next kernel boot matched the userspace watchdog's
   timeout window.
3. **Shell archaeology.** fish history (with correct `cmd:`/`when:` pairing)
   placed three gate runs at 00:58, 09:02, 09:12 and the `killall cargo` at
   01:18. `ResetCounter` diagnostics revealed four boots (01:20, 08:54,
   09:00, 09:33), not one.
4. **Panic reports** (`/Library/Logs/DiagnosticReports/panic-full-*.panic`,
   required sudo) named the mechanism: watchdog timeout plus compressor
   segment exhaustion, with boot/calendar epochs pinning panics to 08:32:46
   (7.2 h uptime) and 09:15:59 (16 min uptime).
5. **Jetsam history** (June 4 reports) established the chronic backdrop:
   zed at 52–55 GiB resident, Spotlight peaking at 89 GiB indexing a 500+ GiB
   `target/`, mds_stores at 9.4 GiB, Backblaze ~8 GiB — and one acute event
   where a scratch experiment (`_scratch_membership`, see below) hit
   106.7 GiB in 37 CPU-seconds and jetsam killed 7,069 processes with
   `vm-compressor-space-shortage`.
6. **Claude transcript archaeology** attributed the "idle" 08:32 panic and
   the ~08:59 crash to background agent sessions running `just gate` —
   closing the loop: every crash followed a codegen build of post-`abf5c3a`
   code.
7. **Controlled reproduction.** A watchdog wrapper (now `tools/memwatch`)
   killed rustc at a memory cap instead of letting it take the machine down.
   A single-target probe (`cargo build --test multi_peer --all-features`)
   blew past 10 GiB in ~20 s at HEAD, deterministically.
8. **Bisection** in a throwaway worktree, same probe: `aa321d2` good (8 s),
   `262568f` good (6 s), `abf5c3a` **bad** — first bad commit. A parallel
   diff-analysis agent independently converged on the same hunk.
9. **Fix verification.** Restoring the collect at HEAD: probe builds in 5 s;
   the full `--workspace --all-targets --all-features` build finishes in 35 s
   with a ~1 GiB peak.

## Root cause, precisely

`Act` recurses over tree height at the type level (`S<H>` → `H`, 32 levels;
one trait instantiation per level). Inside each level, actions are grouped by
radix with itertools (`sorted_by_key` → `chunk_by`), and each group is mapped
and passed to the next level's `Act::act<T, F, I>`.

With the collect, `I` is `Vec<(Path<H>, Version, Action<T>)>` at every level:
flat, fixed size per height. Without it, the next level's `I` is a
`Map<Group<...>, closure>` whose closures capture the *enclosing*
instantiation's generics — including the incoming `I`. Each level's type
therefore contains the previous level's type; size compounds geometrically
across 32 levels, and monomorphization explodes at codegen. The restored
collect carries a comment in `act.rs` marking it load-bearing.

## Secondary findings

- **`_scratch_membership` (June 4).** A throwaway example written during an
  earlier agent session to stress fork/join membership. Its 107 GiB blowup
  was a *harness* bug, not a library one: fork cloned the full `Vec<Key>`
  bookkeeping and join concatenated without dedup, so fork-then-rejoin cycles
  doubled a lineage's key list — exponential duplication of 32-byte keys.
  The parties never gossiped inside the loop, so the trees stayed small.
  ITC clock growth under fork/join churn is therefore *not established* by
  that incident; if worth probing, rerun the experiment with deduped key
  lists and bounded steps, under memwatch. (Source is recoverable from the
  June 4 session transcript, `c8a43c8a`, lines 213/217.)
- **Chronic memory pressure** made the machine fragile: a zed leak holding
  50+ GiB resident across a day, and Spotlight/Time Machine/Backblaze
  churning over a 500+ GiB `target/`. Remediated by excluding `src/` from
  all three and `cargo clean`; a periodic clean (or `cargo-sweep`) keeps the
  target bounded. A zed update/restart habit is worth keeping until the leak
  is gone.
- **Red herring:** Gatekeeper/`spctl` was blamed for exec stalls mid-incident;
  those stalls were paging latency.

## Remediation

- `030b4ba` — restores the collect (fixing both the codegen explosion and
  the consumed-iterator correctness bug), with an explanatory comment.
- `90df422` — adds `tools/memwatch` and runs the codegen-bearing recipes
  (`test`, `doctest`, `bench-build`, `fuzz-build`) under it. A recurrence of
  this bug class now fails the build naming the offending crate instead of
  panicking the machine. `check`/`clippy` skip codegen and run bare. Limits
  are overridable per invocation (`PROC_LIMIT_GB=16 just test`).

## Residual risks and open items

- Nothing structurally prevents a future lazy iterator from re-entering a
  height-recursive traversal; memwatch converts that from a machine-killer
  into a named build failure, but a lint or a comment-convention in the
  traversal modules is the only compile-time guard today.
- The zed leak is unaddressed at its source.
- The ITC clock-growth question (fork/join churn without retire) remains
  open and is worth a bounded, instrumented experiment.
- If CI is ever added, a memwatch-style RSS cap there would have caught this
  on the first push.
