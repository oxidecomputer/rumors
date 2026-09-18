# Style standard for `before`

This is the local reference requested for the `before` triage. It carries the
Rumors triage's writing rules into this effort and specializes them for the
library's public API, algorithm modules, tests, and instruments.

## Write for the reader

First identify who is reading and what they need:

- Crate documentation explains why the library exists, the model a caller must
  respect, the main types, and the shortest path to correct use.
- Public type and method documentation states meaning, behavior, obligations,
  guarantees, errors, panics, and relevant costs.
- Module documentation orients a maintainer to purpose, boundaries, invariants,
  and the relationships needed to change the module safely.
- Private item documentation states the helper's purpose and any non-obvious
  invariant or representation choice.
- Test documentation states the behavior and invariant the test protects.

Do not expose an internal mechanism in public rustdoc unless understanding it
changes how a caller should use the API. Do not bury a public contract in a
private comment or a test.

`before`'s documentation stands on its own as the documentation of a general
library. Do not name downstream crates or explain an API by cataloguing its
consumers. When downstream use reveals a missing guarantee, state that
guarantee at the `before` API that owns it.

## Prefer deletion

Before rewriting a sentence, ask whether it should exist. Delete:

- restatements of the signature or body;
- history and provenance that Git already records;
- counts, readings, dates, line numbers, and rosters that can drift;
- duplicated contracts and explanations;
- motivation that changes no decision for the reader;
- claims without a proof, test, measurement, or clear argument;
- stale API names and references to removed code;
- ornamental metaphors and terms coined for one implementation.

Keep enough explanation to teach the contract or invariant. Concision is not
telegraphy: a short passage should still supply the premise needed to trust its
conclusion.

## Use plain structure

State the purpose in the first sentence. Use ordinary words and default word
order. Put qualifications beside the claim they qualify. Prefer a short
paragraph or direct list to a long sentence carrying several parenthetical
arguments.

Use terminology already defined by the public model or the ITC paper. Define a
specialized term at one stable owner before using it elsewhere. Avoid terms
such as “door,” “seam,” “mint,” “honest,” “genre,” “sentry,” “keystone,” and
similar shorthand unless the term is truly necessary and precisely defined.
When touching a comment, rustdoc, test description, or assertion message,
rewrite the affected prose plainly; do not preserve nearby jargon through a
minimal edit.

Do not call an id or Party “packed.” There is only one representation, and it
is packed. When the representation matters, name the id encoding or its bits.

State current behavior in the present tense. Do not say “formerly,”
“superseded,” “removed,” or name deleted APIs. Do not call an input
“adversarial” when “worst case,” “deep,” “wide,” or the exact shape is clearer.

## Respect abstraction boundaries

Place each explanation at the level that owns it:

- A module owns its invariants and guarantees, not a catalogue of its current
  helper layout.
- A type owns its meaning and representation invariants.
- A method owns its caller-visible contract and cost.
- A local comment owns a particular implementation choice or proof step.
- A test owns the claim established by its assertions, not the architecture of
  the entire verification system.

Do not enumerate every caller of an item or list every current item in a
module. Explain a relationship only when the reader needs it to understand the
contract or safely make a change; source navigation and compiler-visible
structure own exhaustive inventories.

Prefer representations and APIs that make prose unnecessary. If a roster can
be derived from a type or a test table, derive it. If a comment must keep two
copies synchronized, remove the duplication.

## Document every definition usefully

Every function, type, trait, implementation, constant, static, macro, module,
and test helper needs at least a brief doc comment explaining its purpose.
Trait implementations should explain what the trait means for the type when
that is not immediate.

The requirement is semantic, not mechanical. “Returns the value” on a function
named `value` is not useful documentation. A one-sentence purpose is enough
when the signature and surrounding module carry the rest.

## Review tests as proofs

Test legibility is a standing requirement, not a cleanup phase. Whenever a
change touches a test, leave its claim and method easier to audit. Preserve its
coverage and semantics unless the test is wrong; then correct it so its setup
and assertions establish the stated invariant.

A test name and doc comment should state one behavior. Its setup should reach
that behavior without incidental machinery, and its assertions should make the
protected invariant visible. Prefer properties when the claim ranges over a
boundary, ordering, algebra, schedule, or family of shapes.

Remove vacuous assertions, source-text phrase checks, copied rosters, and tests
that verify only their own fixture construction. Share generators and helpers
when doing so makes the property clearer; do not centralize unrelated setup
behind an abstraction that hides what a test proves.

Test arbitrary-precision behavior at the mathematical and public-API level.
Pin canonical values, accepted forms, observable costs, and complexity classes;
do not pin a dependency's multiplication algorithm, allocation strategy, arm
threshold, or other private implementation choice.

## Final prose pass

Before presenting a batch, read changed code and prose together and ask:

1. Is every statement true of the current implementation?
2. Is it at the right reader altitude?
3. Does it respect the abstraction boundary?
4. Is every term necessary and defined?
5. Can any sentence, helper, or test be removed without losing meaning or
   confidence?
6. Does the remaining text read naturally on the first pass?
