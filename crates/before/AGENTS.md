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

That loop never reaches the tool-side workspace members (every member outside
the root manifest's `default-members`), the amplification board's smoke suite
(nextest's default filter skips it), or the detached `fuzz` workspace; their
justfile recipes and the gate do.

## Hard rules

- No `unsafe`: the crate is `#![forbid(unsafe_code)]`. Where a capability
  needs it, depend on a crate that encapsulates it.
- No library traversal recurses on tree depth: deep walks are iterative. A
  test helper recurses only on depth bounded by construction, and states the
  bound where it recurses; the recursive tree oracle's operating envelope
  requires every harness that drives it to bound its input depth. A test that
  must drive input-controlled depth through a recursion runs it iteratively,
  or on a thread whose stack size is derived from the depth it drives.
- The public API is stable: propose additions or reshapings, with your
  reasoning, rather than making them unbidden.
