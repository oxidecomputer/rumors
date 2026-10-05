# before — Interval Tree Clocks

The documentation of record is the rustdoc and, for the algorithms, the ITC
paper (`reference/itc2008.md`). Read the crate docs first for the model and its
safety rules. Each operation's public type module is the entry point to its
implementation.

## Verifying a change

The workspace's root `justfile` and `AGENTS.md` govern verification: get
`just gate` fully clean before every commit. For a loop scoped to this crate:

- Test: `cargo nextest run -p before --all-features`
- Lint: `cargo clippy -p before --all-targets --all-features -- -D warnings`
- Format: `cargo fmt -p before`

That loop never reaches the detached workspaces in this directory (each
subdirectory with its own `[workspace]` manifest); the gate does.

## Hard rules

- No `unsafe`: the crate is `#![forbid(unsafe_code)]`. Where a capability
  needs it, depend on a crate that encapsulates it, as the test-only stack
  guard does with `stacker`.
- No library traversal recurses on tree depth: deep walks are iterative. A
  deliberately recursive test helper routes each descent through
  `recurse::descend!`.
- The public API is stable: propose additions or reshapings, with your
  reasoning, rather than making them unbidden.
