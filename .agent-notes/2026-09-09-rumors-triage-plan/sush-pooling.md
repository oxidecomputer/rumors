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
pin still needs to advance to the reviewed Rumors changes once their commit is available from the Git remote; final validation must
use that revision without an override.

Run native process/job validation on `ox-east-1-agent` (Helios), using the
[building-on-illumos skill](/Users/oxide/.claude/skills/building-on-illumos/SKILL.md).
Standing owner authorization (2026-09-10): sync the current Rumors and Sush
worktrees to this host and run validation jobs.
Sync both worktrees. Resolve a temporary path-patched lockfile locally, then
use `--locked` with the remote Rumors path; restore the portable local lockfile
after syncing. Seed missing Git dependency commits from the local Cargo cache.
The Mac's PTY and job address-space behavior is not the native test baseline.

Cargo's build directory for this worktree is
`/Volumes/forge/build/a5/fd255fb3310972`. Keep it across Rumors batches.
