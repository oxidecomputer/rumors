# Sush compatibility branch

Worktree: `/Users/oxide/src/sush/.worktrees/rumors-compat`, branch
`codex/rumors-compat`, based on `origin/locker`. Keep this worktree until the
final Sush review. The [plan](README.md) owns the ongoing workflow.

The branch uses Rumors-owned connection pooling, supplies Sush's routing
deadline, and handles the current message admission API. Implementation and
validation details belong in its commits, not in a duplicate patch here.

While Rumors changes are uncommitted, the ignored
`.cargo/rumors-local.toml` patches the Rumors Git dependency to the active
Rumors worktree. Pass it to the Cargo subcommand, for example:

```sh
cargo check --config .cargo/rumors-local.toml --workspace --all-targets
cargo clippy --config .cargo/rumors-local.toml --workspace --all-targets -- --no-deps --deny warnings
cargo nextest run --config .cargo/rumors-local.toml --workspace --run-ignored all --no-fail-fast
```

The override rewrites `Cargo.lock` for local paths. Preserve the Git-resolved
lockfile before running with it, then restore that lockfile before committing.
Never commit the override or a lockfile resolved against local paths. The Git
pin still needs to advance to the reviewed Rumors changes once they have a
commit; final validation must use that revision without an override.

Run TLS conformance after parallel build jobs settle to avoid test dial timeouts.

Outstanding validation: run the full process/job suite on Linux. On this Mac,
PTY-path assertions and the job address-space limit fail; transport, gossip,
and message-admission checks pass.

Cargo's build directory for this worktree is
`/Volumes/forge/build/a5/fd255fb3310972`. Keep it across Rumors batches.

Temporary checks to remove after the routed-pooling merge, along with their
specific build directories:

| Checkout | Build directory |
| --- | --- |
| `/private/tmp/rumors-pooling-sush` | `/Volumes/forge/build/1d/619946551a5011` |
| `/private/tmp/rumors-runtime-independence` | `/Volumes/forge/build/e3/ec172cffa4b2f2` |
| `/private/tmp/rumors-routed-timeout` | `/Volumes/forge/build/13/920b1318964d9c` |

Also remove `/private/tmp/routed-pooling-review-base` after merge.
