# Persistent replicas and causal iteration

Status: design proposal for review. Written and reviewed 2026-09-08 against
repository commit `eb224bc35ef809fae74d97554b924e4e6a7d124b`.

This plan adds persistent storage and causal iteration to Rumors. A replica
should hold more messages than fit in RAM, recover its state and identity
after a crash, and yield causes before their effects without sorting the
whole live set in memory. Large updates must leave other synchronization
free to proceed.

The design stores immutable objects and prepares new replica states
concurrently. Rumors manages their reference counts, snapshots, and cleanup;
the engine supplies ordered bytes and small atomic write batches. A finished
state becomes current by replacing a small root record. Ownership is recorded
as objects are built, so that final replacement has no update-sized backlog
of reference-count changes.

The explanation separates three questions:

1. Which messages are live, and what history has the replica observed?
2. Who may issue future events under each part of the peer's identity?
3. Which stored objects must remain available to a reader or unfinished job?

The chapters introduce these concerns separately, then show how they fit
together. They assume familiarity with Rust and basic concurrent programming;
the library's causal and storage concepts are introduced as needed.

## Reading order

1. [Purpose, model, and guarantees](01-model.md): the foundations, including
   what a durable cursor does and does not promise.
2. [Storage and ownership](02-storage.md): immutable objects, references,
   snapshots, bounded construction, and reclamation.
3. [Index, commits, and identity](03-operations.md): causal ordering,
   concurrent reconciliation, publication, restart, and identity transfer.
4. [Implementation and verification](04-implementation.md): ordered work
   packages, acceptance criteria, API sketches, and decisions to validate.
5. [Appendix: review of the earlier proposal](05-review.md): the decisions
   retained, specific failure cases, and the reasons for changes.

The first four chapters are a standalone implementation plan. The appendix
restates the earlier proposal's relevant choices before reviewing them, so
it can also be read without that document. Section 4.11 collects the choices
that still need measurements or review.

## Scope and evidence

Immutable storage, library-managed ownership, and bounded write batches are
requirements throughout. The design requires neither engine snapshots nor
locks held through whole-update preparation. Sharding, a durable history log,
and tolerance of dishonest authorized peers are outside its scope.

Names and signatures are proposed APIs. Wire-format changes require the
repository's owner review and deliberate snapshot-test updates. Source links
identify the code inspected; the appendix records the design's provenance.

## Print edition

[Download the continuous PDF](rumors-persistence-plan.pdf). It contains the
overview, all four chapters, and the comparison appendix, with a contents
list, continuous page numbers, and US Letter margins for annotation. The main
text is justified, with 11.5-point type and expanded line spacing; table
columns and code remain left aligned.

The Markdown files are the editable originals. To regenerate the PDF from
the repository root, with Pandoc and Typst installed:

```sh
sh .agent-notes/2026-09-08-persistence-and-causal-index/print/render.sh
```

The print template and navigation filter are in `print/`. The build needs
the Charter, Helvetica Neue, and DejaVu Sans Mono fonts; it was checked with
Pandoc 3.10.2 and Typst 0.15.1. `print/manuscript.typ` is generated, not an
additional source to edit.
