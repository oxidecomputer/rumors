# Machinery brief: give the detached workspaces a terminating nextest timeout

## The failure class it catches

A hung test in a detached workspace hangs the gate instead of failing it.
The root workspace's `.config/nextest.toml` states the rule ("Liveness
regressions must fail CI, not hang it") and sets `slow-timeout = { period =
"60s", terminate-after = 3 }`. Nextest reads its configuration from the
workspace root, so the detached workspaces the gate runs nextest in get
nextest's built-in default instead: flag as slow every 60 s, never terminate.

Verified on ox-east-1: a test that sleeps 200 s, added to a probe copy of the
`wasm32-pins` harness (`l8/probe/`, never `crates/`), ran to completion:

```
SLOW [> 60.000s] ... l8_sleeps_past_the_root_limit
SLOW [>120.000s] ... l8_sleeps_past_the_root_limit
SLOW [>180.000s] ... l8_sleeps_past_the_root_limit
PASS [ 200.059s] (1/1) ... l8_sleeps_past_the_root_limit
```

(log: w32/nextest-timeout-probe.log).

For `wasm32-pins` nothing else bounds a test either: the harness builds its
engine with `Engine::default()` (`wasm32-pins/harness/src/lib.rs:72`), with
no fuel and no epoch interruption, so a guest loop that never ends runs
forever. That is the defect class the pins exist for: a position or length
wrapped at 2^32 can turn a `while position < len` loop infinite. The gate's
`wasm` stream would then never complete, and `gate-streams` waits on it.

## Scope (verified by `find . -name nextest.toml`)

| workspace | runs nextest in | config |
|---|---|---|
| root | `test-all` and others | `.config/nextest.toml` |
| `crates/before/wasm32-pins` | `wasm32-pins` recipe (justfile:644), gate `wasm` leg | none |
| `crates/before/surfacecheck` | `surface-totality` recipe (`cargo nextest run`), gate `surface` leg | none |
| `crates/before/fuzzfit` | `fuzzfit` recipe (justfile:608), gate `wasm` leg | none (workspace out of the audit's scope; same gap) |
| `crates/before-fuelscape` | `fuelscape-test` (justfile:663), gate `wasm` leg | none (out of scope; same gap) |

## Proposed machinery

Add `.config/nextest.toml` to each detached workspace that the gate runs
nextest in, with a terminating slow timeout sized from that workspace's
measured maximum, and a comment pointing at the root file's rationale rather
than restating it:

- `wasm32-pins`: the slowest pin measured 79-84 s on ox-east-1 under load
  (`rank_arithmetic_straddles_the_usize_alignment_limit`; baseline run here
  78.985 s). `period = "60s"`, `terminate-after = 4` (240 s) leaves about 3x
  headroom; the owner may prefer a per-test override for the deep pins.
- `surfacecheck`: its tests are fast; the root's 180 s is ample.

An alternative or complement for `wasm32-pins`: enable wasmtime epoch
interruption or fuel in the harness with a generous bound, so a looping
guest becomes a `Trapped` outcome that names the check. That is more work and
duplicates the timeout's job; the config file alone closes the hang.

## Calibration

The sleep probe above is the calibration: with the proposed config it must
end as `TIMEOUT` (nextest's terminated status) after 240 s instead of
passing at 200 s plus. A builder can rerun it from a probe copy, or with any
test that sleeps past the chosen limit, and must not commit it.
