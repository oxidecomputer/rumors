# Sush compatibility branch

Worktree: `/Users/oxide/src/sush/.worktrees/rumors-compat`, branch
`codex/rumors-compat`, based on `origin/main`. Keep this worktree until the
final Sush review. The [plan](README.md) owns the ongoing workflow.

[Draft PR #84](https://github.com/oxidecomputer/sush/pull/84) tracks this work
against Sush `main`; keep it a draft until the final compatibility review.
Push only after an approved Rumors merge, following the
[publication sequence](README.md#2-how-each-change-proceeds).

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
Never commit the override or a lockfile resolved against local paths.
After an approved Rumors merge is pushed, advance the Git pin and validate against it without an override before pushing
the compatibility branch.

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
