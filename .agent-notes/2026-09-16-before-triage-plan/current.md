# Current triage state

Work proceeds directly on `main`, with one uncommitted production-structure
increment active.

## Active outcome

Make Party and Version algorithms operate on their domain rather than raw bit
storage. Party traversal and construction now use bounded cursors, subtree and
path handles, and canonical builders. Version traversal, validation, and
construction use the corresponding version I/O boundary. Preserve canonical
bytes, single-pass bulk paths, compact depth state, and all semantic coverage.

Name internal structures for the domain role they perform, not an implementation
metaphor. In particular, `min_ticks` computes leaf-height contributions and
nested subtree minima: its modules and types should speak in terms of minima,
height prefixes, and contributions. Its entry point must teach the identity
and tree walk before exposing the compact accounting needed to preserve the
linear bound.

Keep each public type's inherent API, rustdoc, and deliberate listing order in
its owning file (`party.rs`, `version.rs`, and so on). Keep those method bodies
short by delegating to domain readers, writers, walks, or construction on the
result type. Do not introduce a second representation-level method on the
public type merely to hold the implementation. Give each substantial operation
its own private module when that makes the implementation easier to locate and
understand.

Use a helper struct when it owns meaningful state that evolves across a walk,
or when the caller genuinely retains it as a builder or iterator. Do not turn
an algorithm's initial arguments into a frozen struct merely so a later
`run`/`finish` method can unpack them; expose the operation directly and keep
any necessary walk state inside its implementation.

Before review, finish the independent-review corrections, run the full gate,
and present the complete diff with a focused review order. Do not commit until
the owner says `lgtm`.

After this increment, choose again by the README's priority rules. The leading
open outcomes are the remaining public-contract audit, resource-instrument
consolidation and deletion, semantic-suite consolidation, and further Version
and codec simplification.
