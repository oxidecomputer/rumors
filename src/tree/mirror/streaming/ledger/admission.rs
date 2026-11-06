//! Admission: when a walk may build its next unit of work, and what of it
//! the budget counts.
//!
//! A *unit* is the work the walk builds in one step of an answer. Each
//! answer is one [`Unit::Answer`], its own items: its resolution and the
//! reply's reactions that ask nothing. Each question the answer asks is one
//! [`Unit::Question`]: the question's built items (its query, its chunk,
//! and its record) and the reservation for the peer's reply. The
//! reservation's largest part is the room the reply's chunks park in as
//! they arrive, *the reply's room*; its other parts hold the records the
//! reply's questions derive and the parent resolution that reassembles the
//! scope asked about. A unit is admitted with a *grant* `c`, a charge of
//! the most its items can hold; the walk builds the items out of the grant
//! ([`Charge::take`](super::Charge::take)), so every item it builds is a
//! part split off that grant, and drops what is left.
//!
//! # Kinds and depths, bounds, and the excess
//!
//! - A unit's *kind and depth* `k` is its kind (answer or question) and the
//!   depth of the scope it concerns, the root's 0 through the leaves' 32,
//!   one for each kind at each depth: an answer the scope it answers, a
//!   question the scope it asks about. We write `k = (question, 3)`.
//! - `held[k]` is the bytes every live charge built out of grants to units
//!   of kind and depth `k` holds together: the grants themselves and every
//!   part split off them, until each drops.
//! - `F[k]` is the *bound* of `k`: the most any grant to a unit of `k` can
//!   hold, from the walk's types ([`Bounds`], built from
//!   [`grant_bound`](crate::tree::mirror::streaming::materialized::grant_bound)).
//!   On the in-memory backend an answer's bound is about 38 KiB, and a
//!   question's about 1.6 MiB, almost all of it the listing the reply may
//!   carry.
//! - The *excess* of `k` is `max(0, held[k] − F[k])`: the bytes `k` holds
//!   beyond its bound. The ledger's excess `E` is the sum of every kind and
//!   depth's excess. *The budget counts the excess, and nothing else.*
//! - `B` is the ledger's *budget*. `L` is everything the ledger holds,
//!   `Σ_k held[k]`.
//! - `A` is the *floor allowance*: the larger, over the two sides of a
//!   session, of the sum of `F[k]` over the kinds and depths that side uses
//!   ([`floor_allowance`](crate::tree::mirror::streaming::materialized::floor_allowance)).
//! - A unit is *exempt* when it is admitted without asking anything of the
//!   budget (the rule's step 1).
//! - A *waiter* is a unit that asked for admission and has not been granted
//!   it. A *critical section* is one hold of the admission queue's lock.
//!
//! # The rule
//!
//! The ledger keeps `held[k]` for every kind and depth and the excess `E`,
//! both changed only under the admission queue's lock ([`Counts`]). The
//! rule lives in [`decide`] and nowhere else. A unit of kind and depth
//! `k = (kind, d)` with grant `c` is decided this way, under that lock, with
//! `open` true unless an earlier step of the same critical section closed
//! the budget:
//!
//! 1. **Exempt.** If `held[k] = 0`, or `held[k] + c ≤ F[k]`, the unit is
//!    granted. It claims nothing under the invitation share, and neither the
//!    closed budget nor the share can refuse it.
//! 2. **Room.** Otherwise, if `open` is false, or if the excess with the
//!    unit added exceeds the budget,
//!
//!    > `E′ = E − max(0, held[k] − F[k]) + max(0, held[k] + c − F[k]) > B`,
//!
//!    the unit waits *for room*, and `open` becomes false: every shallower
//!    unit decided later in this critical section may be granted only by
//!    step 1.
//! 3. **Share.** Otherwise, if the unit is a question under the invitation
//!    share (one an internal answer asks,
//!    [`Ledger::invite`](super::Ledger::invite)), it waits *for its depth's
//!    replies* unless
//!
//!    > `I_d + c + α · (N_d + n) ≤ B`,
//!
//!    where `I_d = max(0, held[k] − F[k])` is the excess of the depth's
//!    questions before this one, `n` its predicted need, `N_d` the needs its
//!    depth has claimed and not withdrawn, and `α` the depth's pace
//!    ([below](#the-invitation-share)). A unit refused here leaves `open` as
//!    it was.
//! 4. **Grant.** Otherwise the unit is granted against the budget. A
//!    question under the share adds `n` to `N_d`, and its charges record
//!    that it claimed ([`Charge::claimed`](super::Charge::claimed)).
//!
//! A unit refused at step 2 or 3 is decided again whenever the ledger
//! releases bytes, a reply finishes, or a waiter withdraws. Each
//! reconsideration decides every waiter in one critical section, deepest
//! first and, within a depth, in the order the units asked, with `open`
//! starting true ([`admit_waiting`]). A unit that asks while nothing waits
//! is decided at once, by the same steps, with `open` true ([`Admit::new`]).
//!
//! Releasing `b` bytes of a charge of `k` lowers `held[k]` by `b`. A
//! *shortfall*, an item that needs more than its grant has left, raises
//! `held[k]` by the difference. A split moves bytes between two charges of
//! the same `k` and changes nothing here. The excess changes with each
//! change to `held[k]`, from `h` to `h′`, by
//! `max(0, h′ − F[k]) − max(0, h − F[k])`, so every decision is a constant
//! number of comparisons.
//!
//! **Why `held[k] = 0` is its own clause.** Under every grant the walk
//! makes, `c ≤ F[k]` (premise (P1) below), so `held[k] = 0` implies
//! `held[k] + c ≤ F[k]`, and step 1 is simply *the unit fits within its
//! kind and depth's bound*. The clause `held[k] = 0` is written separately
//! so that a table of bounds smaller than some grant, whether a test's or a
//! defect's, keeps every unit live ([below](#every-unit-is-admitted)) and
//! costs only the bound ([boundaries](#boundaries)).
//!
//! **What step 1 means.** Under (P1), a unit granted at step 1 adds nothing
//! to the excess, so it takes no room from the budget. That is why neither
//! the deeper-first ordering, which allots budget room, nor the share, which
//! limits what a depth claims on the budget, has anything to refuse it for.
//! A unit refused for room still closes the budget to shallower units until
//! it is granted, even where only releases of its own kind and depth can
//! help it: letting it wait without closing would break the monotonicity
//! below ([what must stay](#what-must-stay)).
//!
//! # Premises
//!
//! Each claim below names the premises its argument uses. Each premise says
//! where it is established.
//!
//! - **(P1) Every grant is within its bound:** a unit of `k` is granted at
//!   most `F[k]`. Each grant formula grows with each of its arguments, and
//!   [`grant_bound`](crate::tree::mirror::streaming::materialized::grant_bound)
//!   evaluates it at the largest arguments the walk can pass a unit of that
//!   kind at that depth. Established by construction; the test kit checks it
//!   against every admission of every session it runs.
//! - **(P2) One ledger holds one side's kinds and depths.** A question about
//!   a scope is asked by an answer about its parent on the same side and
//!   answered on the other side. The initiator asks the opening question, at
//!   depth 0, so it asks only at even depths and answers only at odd ones;
//!   the responder does the reverse. Each side's ledger is fresh per session.
//!   The test kit checks it beside (P1).
//! - **(P3) Every byte of a charge belongs to one kind and depth for its
//!   whole life.** Charges are created only by grants and by splitting a
//!   charge, which keeps its origin; no charge is ever merged into another.
//!   The opening question takes the charges of the one question it carries,
//!   and fails on a second or on none, so no fold of one unit's charge into
//!   another's returns bytes to the wrong kind and depth.
//! - **(P4) The ledger's counts change only under its lock.** Every grant,
//!   release, and shortfall changes `held` and `E` inside one critical
//!   section of the admission queue's lock, and every decision reads them
//!   there ([`Counts`]). Charges with no unit, which only tests make
//!   (`Ledger::charge`), count in a class of their own whose bound is
//!   zero.
//! - **(P5) Every item fits its grant:** no shortfall. A shortfall is a
//!   conformance bug in pricing, counted in the ledger's `short`, and the
//!   session-budget suite fails on it.
//! - **(P6) One producer per kind and depth.** On each side, the stage that
//!   processes the replies to questions about scopes at depth `d − 1` is the
//!   only producer of answers at depth `d` and of questions at depth
//!   `d + 1`. It awaits each admission before it asks for the next unit, and
//!   it asks for all of an answer's questions before it asks for the next
//!   answer. The opening question and the responder's root answer each have
//!   one producer too. In the walk, that stage is one level's
//!   `Work::answering_walk`, which `internal_level` and `leaf_parent_level`
//!   start: it admits each answer, then each of the answer's questions
//!   through its level's `AnsweringLevel::ask` (the `Internal` and
//!   `LeafParent` impls), awaiting each admission in turn.
//! - **(P7) Release needs only deeper work.** A unit admitted at depth `d`
//!   releases every byte it was granted once the units its release waits on
//!   have been admitted and have released theirs, provided the pipe drains.
//!   Those units are deeper than `d`, on either side, or, for a question,
//!   the peer's answer to it at depth `d`. Each is asked for by a stage that
//!   processes deeper replies than the unit's own producer does, or by the
//!   unit's own producer before it moves on from the unit's answer (an
//!   answer's questions). None is a unit of the unit's own kind and depth
//!   that asked after it.
//!
//!   In detail: a question's chunk returns once the multiplexer has written
//!   the chunk's last frame (a wait on the pipe alone), its record when the
//!   reply's last chunk is taken, the reply's room chunk by chunk as this
//!   side's stage at depth `d` uses the reply's chunks and the rest as the
//!   reply shows it cannot be used, the records part as this side's encoder
//!   hands over its answers at depth `d + 1` to the reply's questions, and
//!   the parent resolution's part as the scope asked about is assembled. An
//!   answer's charges return once its questions' subtrees are assembled and
//!   its chunks' last frames are written.
//! - **(P8) The pipe drains.** The receive path never waits on the walk,
//!   and the send path ends because the peer's receive path drains the pipe,
//!   so only admission can hold the walk (the pipe module's wait table).
//!   This, with (P6) and (P7), is what *conforming traffic* means here.
//!
//! # The memory bound
//!
//! **Claim.** Under (P1) to (P5), at every moment outside admission's
//! critical sections,
//!
//! > `E ≤ B`, and so `L = Σ_k min(held[k], F[k]) + E ≤ A + B`.
//!
//! That is: the bytes the budget counts never exceed the budget, and what
//! the ledger holds is at most the budget beside one bound's worth for each
//! kind and depth this side uses.
//!
//! **Argument.**
//!
//! 1. *Each kind and depth splits into a part within its bound and its
//!    excess.* For every `h ≥ 0`, `min(h, F) + max(0, h − F) = h`. Summing
//!    over `k`, `L = Σ_k min(held[k], F[k]) + E`.
//! 2. *`E ≤ B` holds initially:* a new ledger holds nothing, so `E = 0`.
//! 3. *`E ≤ B` is preserved by every change.* By (P3) and (P4), `held` and
//!    `E` change only in these ways, each inside a critical section:
//!    - *A grant at step 1 with `held[k] + c ≤ F[k]`:* `held[k]` stays
//!      within `F[k]`, so the excess of `k` stays 0 and `E` is unchanged.
//!    - *A grant at step 1 with `held[k] = 0`:* by (P1), `c ≤ F[k]`, which
//!      is the previous case.
//!    - *A grant at step 4:* step 2 passed, so the new `E′ ≤ B`.
//!    - *A release:* `held[k]` falls, so the excess of `k` does not rise.
//!    - *A shortfall:* excluded by (P5).
//!    - *A split:* both parts keep their kind and depth (P3), so `held` is
//!      unchanged.
//! 4. *Only one side's bounds can be in use.* If `held[k] > 0` then `k` is
//!    one of this side's kinds and depths (P2), and
//!    `min(held[k], F[k]) ≤ F[k]`, so
//!    `Σ_k min(held[k], F[k]) ≤ Σ_{k of this side} F[k] ≤ A`.
//! 5. From 1, 3 and 4, `L ≤ A + B`. ∎
//!
//! ## What is exact and what is approximate
//!
//! The ledger counts exactly what the walk charges, and the claim bounds
//! that count exactly: `E ≤ B` and `L ≤ A + B` hold at every moment, not on
//! average. The stronger form, `L ≤ B + Σ_k min(held[k], F[k])`, is the one
//! a test asserts, as two separate checks, `E ≤ B` and `L − E ≤ A`.
//!
//! What a session allocates in all is approximate. A session that may hold
//! `M`, as a peer's memory budget sets it, gives its ledger
//! `B = M − A − X`, where `X` is the *session allowance*, the most the
//! state it holds outside the ledger can hold whatever its budget, its wire
//! pools at its message-size target included
//! ([`session_allowance`](crate::tree::mirror::streaming::materialized::session_allowance)).
//! The ledger's part of `M` is exact; the rest is a model: `X` is computed
//! from the types at the message-size target, and the budget's documented
//! exclusions sit outside it. So the budget acts as a limiter, not a
//! ceiling: admission applies backpressure exactly when the excess would
//! pass `B`, which a session reaches when it holds `B` beside what its
//! kinds and depths hold within their bounds. That second term is a few
//! mebibytes of the `A` set aside (about 3 to 4 MiB where measured
//! sessions reached their budget), so the backpressure begins somewhat
//! below `M`, never above it, for everything the ledger counts.
//! An `M` at or below `A + X` gives the ledger a budget of zero: the session
//! then pipelines within its bounds alone.
//!
//! ## Boundaries
//!
//! - **A grant above its bound** ((P1) broken, a defect). Step 1 still
//!   admits it at an empty kind and depth, so the session stays live, and
//!   `E` may exceed `B` by at most that grant's overshoot, `c − F[k]`, for
//!   as long as the unit holds it: the next unit of `k` is not exempt while
//!   `held[k] > F[k]`, and step 2 counts the overshoot. One such unit per
//!   kind and depth can be outstanding at a time.
//! - **A ledger holding both sides' kinds** ((P2) broken). The bound
//!   becomes the sum of both sides' bounds beside `B`.
//! - **A shortfall** ((P5) broken). It raises `held[k]` beyond what
//!   admission approved, and `E` may exceed `B` by the shortfall. The
//!   ledger counts it in `short`; the bound is promised only without it.
//! - **An unlimited budget.** No unit waits, and the bound says only that
//!   `E` is finite.
//!
//! # Monotone in the budget
//!
//! Two budgets' sessions can first differ only by an admission the larger
//! budget makes. This section states the property of the rule that gives it,
//! and shows that a stronger property one might read into "a larger budget
//! admits at least as soon" is false.
//!
//! Order the outcomes of one decision from most to least permissive:
//! *grant*, then *wait for the share* (which leaves the budget open), then
//! *wait for room* (which closes it).
//!
//! **Claim (M).** Let a critical section start in one ledger state at two
//! budgets `B ≤ B′`, with the same waiters. Decide them in the rule's order
//! at each budget. Then up to the first decision whose outcome differs,
//! every decision is the same at both budgets and leaves the same state;
//! and at that first difference, the outcome at `B′` is the more
//! permissive.
//!
//! **Consequence.** Two sessions at `B ≤ B′` that have made the same
//! decisions so far are in the same state, since the state (`held`, `E`, the
//! claims, the pace, the waiters) is a function of the schedule alone and
//! nothing in it reads the budget. So the first admission one makes and the
//! other does not is made by the larger budget. A difference in which the
//! larger budget waits for the share where the smaller waits for room
//! changes no state, only which budget stays open, and the next differing
//! admission after it again belongs to the larger budget.
//!
//! **Argument.**
//!
//! 1. *Each decision is a function of the state, the unit, `open`, and the
//!    budget, and only steps 2 and 3 read the budget.* Step 1 reads
//!    `held[k]`, `c` and `F[k]`. Step 2 reads `open` and compares `E′`, a
//!    function of the state and the unit, with `B`. Step 3 compares a
//!    function of the state and the unit with `B`.
//! 2. *Per decision, a larger budget and a budget at least as open give an
//!    outcome at least as permissive.* Fix the state and the unit, and let
//!    `B ≤ B′` and `open ≤ open′`.
//!    - Step 1 grants at both or at neither.
//!    - If step 2 refuses at `B′`, then `open′` is false or `E′ > B′`; so
//!      `open` is false or `E′ > B`, and step 2 refuses at `B`.
//!    - If step 3 refuses at `B′` (so step 2 passed there), then it refuses
//!      at `B` too, or step 2 refused at `B` first: at `B` the outcome is a
//!      share wait or a room wait, never a grant.
//!    - If the outcome at `B` is a grant, steps 2 and 3 passed at `B`, so
//!      they pass at `B′`.
//! 3. *Equal decisions leave equal states.* A grant changes `held`, `E` and
//!    possibly `N_d` by amounts that depend only on the unit; a refusal
//!    changes only `open`, and equal outcomes change it equally.
//! 4. *Induction along the critical section.* Before the first difference,
//!    states and `open` flags agree (3), so by (2) the first differing
//!    outcome is the more permissive at `B′`. ∎
//!
//! ## The no-cost point
//!
//! Let `Λ` be the largest `E′` any step 2 weighs in the session with no
//! budget, and the *share peak* the largest sum any step 3 weighs there.
//! The session's *no-cost point* is the larger of the two. At every budget
//! of at least that point, every check along that session's schedule passes
//! in the same state, no unit waits, and by induction the session is the
//! unlimited one, event for event. Below it, the first check along that
//! schedule that exceeds the budget refuses its unit, so some unit waits.
//! `Λ` is the peak excess, which is zero in every session whose kinds and
//! depths each stay within their bounds; there the no-cost point is the
//! share peak.
//!
//! Identity is promised at the no-cost point, not at `Λ`. A rule that holds
//! back the first round's questions, the burst the share exists to stop,
//! refuses some question that fits; to stay identical at every budget of at
//! least `Λ` it would have to know, at the first round trip, that the
//! budget lies below `Λ`, which the session cannot know until it has run.
//!
//! ## What must stay
//!
//! - **Step 2 precedes step 3.** If the share were checked first, a unit
//!   could wait for the share at `B` (leaving the budget open) and for room
//!   at `B′` (closing it), and a shallower unit could then be granted at
//!   `B` and refused at `B′`.
//! - **Step 1 reads no budget.** An exemption that depended on `B` would let
//!   the two budgets grant different units in one state for a reason the
//!   argument's step 2 does not cover.
//! - **No refinement of the room wait by the unit's own size.** It is
//!   tempting to say that a unit whose own contribution to the excess,
//!   `Δ = max(0, held[k] + c − F[k]) − max(0, held[k] − F[k])`, exceeds
//!   `B` cannot be helped by any other kind's release, and so should wait
//!   without closing the budget, like a share wait. That breaks (M). Take
//!   `B = 100`, `B′ = 110`, `E = 50` held by another kind, a deeper unit
//!   `W` with `Δ = 105`, and a shallower unit `V` with `Δ = 40`. At `B`,
//!   `W` waits without closing, and `V` is granted (`50 + 40 ≤ 100`). At
//!   `B′`, `W` fits its own test (`105 ≤ 110`) but not the room check
//!   (`155 > 110`), so it waits for room and closes the budget, and `V` is
//!   refused. The first difference is two refusals, and the larger
//!   budget's is the one that closes. Such a unit occurs only at budgets
//!   below the largest grant, since `Δ ≤ c`, and it becomes admissible as
//!   releases at its own kind and depth shrink `Δ` or make it exempt.
//!
//! ## Not promised: every unit admitted at least as soon
//!
//! (M) signs only the first difference. A particular unit can be admitted
//! later at the larger budget, within one critical section and with nothing
//! but the rule involved. Take `B = 100`, `B′ = 110`, `E = 0`, and two
//! waiters, neither exempt: `W`, a question under the share at depth 5, with
//! `Δ = 20` and a share sum of 105; and `V`, an answer at depth 4, with
//! `Δ = 100`. At `B`, `W` passes step 2 (`20 ≤ 100`) and waits for the
//! share (`105 > 100`), leaving the budget open, and `V` is granted
//! (`0 + 100 ≤ 100`). At `B′`, `W` is granted (`105 ≤ 110`), and `V` then
//! waits for room (`20 + 100 > 110`). So `V` is admitted now at `B` and
//! later at `B′`. The first difference, `W`, is the larger budget's
//! admission, as (M) requires. What follows a first difference is greedy
//! repacking, as in Graham's anomalies of list scheduling, and the rule
//! signs none of it: the budget criterion's bound of one round trip on such
//! inversions is a measured claim, not a derived one.
//!
//! # Every unit is admitted
//!
//! **Claim.** Under conforming traffic ((P6) to (P8), and (P3) to (P5)), every
//! unit either side asks admission for is eventually admitted, at every pair of
//! budgets the two sides may have, zero included, and for every table of
//! bounds, a table of zeros included. The argument uses no budget above zero:
//! every step leans on step 1 alone.
//!
//! **Argument.**
//!
//! 1. *A unit decided while its kind and depth hold nothing is granted.*
//!    Step 1 grants it before the closed budget or the share is consulted,
//!    and the grant compares nothing with `B` or `F[k]`.
//! 2. *At most one unit of each kind and depth waits, and every other unit
//!    of its kind and depth was granted before it asked.* Its producer
//!    awaits each admission before asking for the next unit (P6).
//! 3. *While a unit `W` of `k` waits, `held[k]` never rises.* Only a grant
//!    at `k` or a shortfall raises it. No unit of `k` is granted while `W`
//!    waits (2), and there is no shortfall (P5).
//! 4. *No unit is left waiting at an empty kind and depth.* Every release
//!    reconsiders every waiter in the critical section that lowered `held`
//!    (P4). Grants in that pass raise only their own kind and depth's
//!    `held`, and by (2) `W` is its kind and depth's only waiter, so if
//!    `held[k]` reaches zero in a critical section, `W` is decided with
//!    `held[k] = 0` in that same section and granted by (1). A unit asking
//!    while `held[k] = 0` is granted as it asks, for the same reason.
//! 5. *Induction on depth, deepest first, over both sides at once.* At each
//!    depth, one side answers and the other asks (P2). Take the claim
//!    `Q(d)`: every unit at a depth of `d` or more, on either side, is
//!    eventually admitted and eventually releases every byte it was granted.
//!    - *The answers at depth `d`.* An answering side's unit `W` at
//!      `(answer, d)` that waits finds `held[k] > 0`, held by units of its
//!      kind and depth granted before it (2). Each of those releases its
//!      bytes once the deeper work `Q(d + 1)` grants has happened (P7), and
//!      that work includes their questions at `d + 1`, all of which were
//!      asked before `W`, since a stage asks all of an answer's questions
//!      before its next answer (P6). So `held[k]` falls to zero (3), and `W`
//!      is granted (4). Once granted, `W` releases by (P7).
//!    - *The questions at depth `d`.* The same argument applies to a
//!      waiting question, with one more dependency in (P7): the peer's
//!      answer at depth `d`, which the previous case grants.
//!    - The base case is the leaves, where (P7) needs nothing deeper.
//! 6. Since only admission can hold the walk (P8), and every unit is
//!    admitted and releases its bytes, every session runs to completion. ∎
//!
//! Step 1 resolves the three schedules in which budget room alone would
//! leave a unit waiting forever:
//!
//! - **The deeper-first closure.** A deeper unit waiting for room closes
//!   the budget to shallower units, and the room it waits for may be held by
//!   units whose release needs a shallower unit. The deeper unit does not
//!   need the room: it is granted at step 1 once the earlier units of its
//!   own kind and depth release, which needs only deeper work (P7). A
//!   shallower unit at an empty kind and depth bypasses the closure at
//!   step 1.
//! - **Hold and wait.** An answer keeps its resolution until its scope is
//!   assembled, which needs its later questions; if what cannot be released
//!   beside a question's grant exceeds `B`, budget room would never come. The
//!   question waits only while its kind and depth hold bytes, all of them
//!   earlier questions' (argument, step 2), which release without it. The
//!   answer's own bytes are another kind and depth, and never stand in its way.
//! - **The share's lone question.** A question at a depth with nothing
//!   outstanding whose own claim `c + α · n` exceeds `B`, reachable at any
//!   budget since `α` has no upper bound, has no replies to wait for. Step 3
//!   cannot refuse an exempt unit, and a question it refuses becomes exempt
//!   once its depth's earlier questions release theirs.
//!
//! So what must stay is exactly step 1's position and reach: decided first,
//! at an empty kind and depth at least, and bypassing both the closure and
//! the share; with one producer per kind and depth (P6) and the
//! reconsideration on every release (P4).
//!
//! **Why kinds and depths stay separate.** An answer's own items cannot
//! share a kind and depth with its questions: at a budget of zero, with zero
//! bounds, the answer's resolution would hold the kind and depth until its
//! scope is assembled, which waits on its later questions, which would wait
//! for that kind and depth to empty. Nor can a question share one with the
//! answers about the scope it asks about; that is moot under (P2), which
//! puts them on different sides.
//!
//! # Boundary cases
//!
//! - **A ledger budget of 0, with the walk's bounds.** Step 2 refuses every
//!   unit that is not exempt, so the rule reduces to step 1: each kind and
//!   depth may hold up to its bound, and an empty one always admits. The
//!   share and the closure then refuse nothing that step 2 would not, so
//!   they change no outcome. Sessions still pipeline, within `A`.
//! - **A ledger budget of 0 with every bound zero**, the *serial
//!   configuration* for tests ([`Bounds::ZERO`]). Step 1 admits exactly the
//!   units that find their kind and depth empty, and nothing else is
//!   admitted: one unit of each kind and depth at a time, the configuration
//!   the liveness argument rests on at its most serial. (P1) fails for a
//!   zero table, so the memory bound does not apply; the ledger holds at
//!   most one grant per kind and depth. It is a test configuration, not a
//!   public one.
//! - **An unlimited budget.** Steps 2 and 3 never refuse, nothing waits,
//!   and the session is the unlimited one.
//! - **A residue.** A unit that has returned all but a few bytes still
//!   holds its kind and depth. The next unit is exempt if it fits beside the
//!   residue within the bound; it never waits for the residue to clear
//!   unless it does not fit.
//! - **A zero-byte grant** changes no `held` and is irrelevant to every
//!   decision.
//! - **The opening question.** It is the only unit of `(question, 0)`, and
//!   the first unit the initiator's ledger decides, so step 1 grants it at
//!   once whatever the budget, the share, or the queue. Its bytes stay
//!   within `F[(question, 0)]` and never touch the budget.
//! - **Leaf requests and the opening** are admitted by
//!   [`Ledger::admit`](super::Ledger::admit), so step 3 never applies to
//!   them, and they never claim.
//! - **A withdrawn waiter** (its [`Admit`] dropped) leaves the queue,
//!   returns any grant it was handed, and the waiters are reconsidered,
//!   since it may have closed the budget for them.
//! - **Different budgets on the two sides.** Each side's ledger decides
//!   against its own budget and bounds; the proofs are per ledger, and the
//!   liveness induction holds for every pair, since it uses no budget.
//!
//! # The invitation share
//!
//! A question *invites* its reply: it holds the reply's room from the
//! moment it is admitted until this side's stage has processed the reply.
//! A stage processes its replies one at a time, in the order its questions
//! were asked, and processing a reply admits units of its own: an answer
//! one level down for each child the reply disputes, and a question two
//! levels down for each grandchild it lists that this side does not match.
//! Room held by a reply that waits its turn overlaps no round trip, and
//! takes the budget from the processing that would let the stage reach it.
//! Without a limit, the answers at the top of a session ask their questions
//! all at once, before any deeper work exists; with a budget smaller than
//! the session's whole need, their replies fill it and then wait behind
//! one stage, while the rest of the session runs in what is left. So a
//! depth should invite no more replies than the budget would also let it
//! process.
//!
//! Each question the share grants against the budget therefore carries a
//! *claim*: its grant, `c`, and `α · n`, where `n` is the bytes processing
//! its reply is predicted to admit and `α` the depth's *pace* (both below).
//! A depth admits a question only while its outstanding claims, the new one
//! included, fit in the budget together, the rule's step 3:
//!
//! > `I_d + c + α · (N_d + n) ≤ B`
//!
//! where `I_d` is the excess of the depth's questions, the room their
//! invitations hold against the budget, and `N_d` the predicted needs of
//! those that claimed and whose replies this side's stage has not finished,
//! the depth's *outstanding claims*. The check follows Dijkstra's banker in
//! its simplest form, lending only while every customer's declared credit
//! fits in the capital at once: a question is admitted only if its depth
//! could also afford to process every reply it has invited. A question's
//! room returns as its reply is processed, and its need withdraws as the
//! stage finishes the reply
//! ([`Ledger::processed`](super::Ledger::processed)), so the depth's claims
//! shrink as its stage reaches them.
//!
//! An exempt question takes no budget room and claims none of its
//! processing. That is consistent with what the share protects, budget room
//! that a waiting reply would take from processing, but means a depth's
//! first `F[k]` bytes of questions invite their replies without the
//! banker's check.
//!
//! No term depends on round trips, delays, or scheduling. A claim states
//! what an invitation could cost the budget, never how long it holds it, so
//! the check reads the same on every link.
//!
//! - **The predicted need, `n`, is a lower bound.** The answers a reply's
//!   processing builds one level down are sized by this side's children of
//!   the node's children, which the walk reads only as it builds the
//!   question, after admitting it, and by the reply's entries, which only
//!   the reply carries. So the counts known at admission bound the need only
//!   at the type maximum. The walk predicts the least it can be instead: the
//!   answers that a reply listing every grandchild its bound allows must
//!   make processing build, as few as can hold the entries, each at the
//!   least it can cost (the walk's `charges::processing_need`, which the
//!   walk passes to [`Ledger::invite`](super::Ledger::invite)). A reply that
//!   can list nothing, such as one answering a request, is predicted to need
//!   nothing, which leaves its depth the budget's whole room.
//! - **The pace, `α`, corrects the prediction from what processing admits**
//!   ([`Pace`], [`Share::pace`]): per depth, the bytes the answers one level
//!   down have kept out of their grants over the needs predicted for the
//!   replies that built them, both read as each reply finishes. It is a
//!   ratio of sums, one until the first reply finishes. It falls below one
//!   where replies list less than their bounds allow, or where the answers
//!   return room they did not use, and rises above one where this side
//!   holds children the peer lacks. Since `n` bounds below, `α = 1` takes
//!   the prediction as it stands and bounds nothing.
//! - **The claims nest.** A depth-`d` claim prices only the answers at
//!   `d + 1` its reply's processing builds. The questions at `d + 2` that
//!   processing sends claim their own room, and their own replies'
//!   processing, at their own depth when they are asked. So each byte
//!   counts toward one depth's sum at most: a question's toward its own
//!   depth's, an answer's toward the depth above it. Pooling the depths
//!   into one check fails by construction rather than by degree: a claim
//!   would have to cover the whole subtree its reply leads to, since the
//!   questions it covers carry claims of their own two levels further down,
//!   so the pooled sum outgrows the budget and refuses every later question
//!   at every depth. The per-depth check is the structure the claims have.
//! - **It applies at every budget.** The share governs every question an
//!   internal answer asks that is not exempt
//!   ([`Ledger::invite`](super::Ledger::invite)). Each depth's check stops
//!   binding by its own form, at the depth's *endpoint*: the sum, over every
//!   question the depth would have outstanding at once, of `c + α · n`, its
//!   questions beside the processing of all their replies. A budget at or
//!   above it admits every question at that depth that fits the budget. For
//!   the root's questions, which a session asks together, the endpoint is
//!   about a fan of them, each beside its reply's predicted processing.
//!
//! The share and its pace pace admissions only: the memory bound and the
//! liveness argument never read them.

use std::cmp::Reverse;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use super::{Account, Charge, widen};
use crate::counter;
use crate::tree::mirror::streaming::holding::Holding;

/// The depths a scope can sit at, the root's 0 through the leaves' 32.
pub(super) const DEPTHS: usize = 33;

/// The kinds and depths of unit one ledger counts: one per kind of unit at
/// each depth.
const KIND_DEPTHS: usize = 2 * DEPTHS;

/// The two kinds of unit, each counted apart at every depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Unit {
    /// An answer's own items: everything it builds but its questions.
    Answer,
    /// One question's built items and the reservation for its reply.
    Question,
}

impl Unit {
    /// The index of this kind at `depth` among a ledger's
    /// [`KIND_DEPTHS`], which `depth` must be below [`DEPTHS`] to fit.
    const fn index(self, depth: usize) -> usize {
        let row = match self {
            Self::Answer => 0,
            Self::Question => 1,
        };
        row * DEPTHS + depth
    }

    /// The place a grant of this kind is charged to until the walk builds
    /// its items out of it.
    fn place(self) -> Holding {
        match self {
            Self::Answer => Holding::Resolutions,
            Self::Question => Holding::Questions,
        }
    }
}

/// The bound `F[k]` of every kind and depth `k`: the most a grant to one
/// unit of `k` holds, which bytes of `k` within it never count against the
/// budget ([the rule](self#the-rule)).
///
/// A session's ledger takes the walk's bounds, from the types
/// ([`grant_bounds`](crate::tree::mirror::streaming::materialized::grant_bounds));
/// a test may pass [`Bounds::ZERO`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bounds([usize; KIND_DEPTHS]);

impl Bounds {
    /// Every bound zero: with a budget of zero, the *serial configuration*,
    /// in which only a unit whose kind and depth hold nothing is admitted
    /// ([Boundary cases](self#boundary-cases)).
    pub(crate) const ZERO: Self = Self([0; KIND_DEPTHS]);

    /// These bounds, with that of `unit` kind at `depth` set to `bound`.
    ///
    /// # Panics
    ///
    /// Panics if `depth` exceeds 32, the depth of a leaf: a programmer
    /// error, since every scope is a prefix of a 32-byte path.
    pub(crate) const fn with(mut self, unit: Unit, depth: usize, bound: usize) -> Self {
        assert!(depth < DEPTHS, "a scope sits at most 32 bytes deep");
        self.0[unit.index(depth)] = bound;
        self
    }

    /// The bound of `unit` kind at `depth`, which must be below [`DEPTHS`].
    fn of(&self, unit: Unit, depth: usize) -> usize {
        self.0[unit.index(depth)]
    }
}

/// The units waiting for one ledger's admission, what each kind and depth
/// holds and the excess the budget counts, and the invitation share's
/// claims and pace.
pub(super) struct Queue {
    /// The units waiting, in no particular order.
    waiting: Vec<Arc<Waiter>>,
    /// The order number the next waiting unit takes.
    next: u64,
    /// What each kind and depth holds, and the ledger's excess, which only
    /// admission's critical sections touch.
    counts: Counts,
    /// The invitation share's state, which only admission's critical
    /// sections touch.
    share: Share,
}

impl Default for Queue {
    /// Returns a queue with no unit waiting, nothing held, and no claim or
    /// finished reply.
    fn default() -> Self {
        Self {
            waiting: Vec::new(),
            next: 0,
            counts: Counts {
                held: [0; KIND_DEPTHS],
                excess: 0,
            },
            share: Share {
                claimed: [0; DEPTHS],
                predicted: [0; DEPTHS],
                actual: [0; DEPTHS],
            },
        }
    }
}

/// What each kind and depth holds, `held[k]`, and the ledger's excess `E`,
/// the bytes the budget counts ([Kinds and depths](self#kinds-and-depths-bounds-and-the-excess)).
///
/// It lives in the queue, so every change and every read holds the queue's
/// lock (premise (P4)): every function that changes it takes it, which only
/// the lock reaches ([`Account::add`], [`Account::sub`]).
///
/// The excess is a running total, `E = Σ_k max(0, held[k] − F[k])`, beside
/// the bytes of charges with no unit, which only tests make and which count
/// in a class of their own whose bound is zero: all of their bytes are
/// excess. Each change moves it by the change in its own kind and depth's
/// excess, so no decision sums the table.
pub(super) struct Counts {
    /// The bytes every live charge built out of grants to units of each kind
    /// and depth holds, indexed by [`Unit::index`].
    held: [usize; KIND_DEPTHS],
    /// The excess, `E`, which the rule's step 2 weighs ([`decide`]).
    excess: usize,
}

/// The excess of a kind and depth that holds `held` under the bound
/// `bound`: what it holds beyond the bound.
fn excess_beyond(held: usize, bound: usize) -> usize {
    held.saturating_sub(bound)
}

impl Counts {
    /// Count `bytes` more held by a charge of `unit`, under `bounds`: a
    /// grant, or an item's shortfall.
    pub(super) fn add(&mut self, bounds: &Bounds, unit: Option<(Unit, usize)>, bytes: usize) {
        let Some((unit, depth)) = unit else {
            self.excess += bytes;
            return;
        };
        let (held, bound) = (&mut self.held[unit.index(depth)], bounds.of(unit, depth));
        let before = excess_beyond(*held, bound);
        *held += bytes;
        self.excess += excess_beyond(*held, bound) - before;
    }

    /// Count `bytes` fewer held by a charge of `unit`, under `bounds`: a
    /// charge returning bytes it holds.
    pub(super) fn sub(&mut self, bounds: &Bounds, unit: Option<(Unit, usize)>, bytes: usize) {
        let Some((unit, depth)) = unit else {
            self.excess -= bytes;
            return;
        };
        let (held, bound) = (&mut self.held[unit.index(depth)], bounds.of(unit, depth));
        let before = excess_beyond(*held, bound);
        *held -= bytes;
        self.excess -= before - excess_beyond(*held, bound);
    }

    /// The ledger's excess, `E`, for observation to copy.
    #[cfg(any(test, feature = "test-internals"))]
    pub(super) fn excess(&self) -> usize {
        self.excess
    }
}

/// One unit asking for admission: what the rule decides it by.
#[derive(Clone, Copy)]
struct Request {
    /// The unit's kind.
    unit: Unit,
    /// The depth of the scope it concerns.
    depth: usize,
    /// The most its items can hold.
    bytes: u32,
    /// For a question under the invitation share, its need: the bytes
    /// processing its reply is predicted to admit one level down, which its
    /// claim weighs ([`decide`]); `None` for every other unit.
    need: Option<usize>,
}

/// A unit's grant, and whether the rule's step 1 made it.
struct Granted {
    /// The grant.
    charge: Charge,
    /// Whether the unit was exempt: admitted at step 1, asking nothing of
    /// the budget.
    exempt: bool,
}

/// What the rule decides for one unit.
enum Decision {
    /// The unit is admitted with this grant.
    Grant(Granted),
    /// The unit waits for room in the budget, and keeps every shallower
    /// unit that is not exempt waiting for it too.
    Room,
    /// The unit's grant fits the budget, but its claim does not fit beside
    /// its depth's outstanding claims.
    ///
    /// The invitation share holds it back until the depth's stage processes
    /// more of its replies; shallower units may still take the budget.
    Share,
}

/// The pace, `α`, of one depth's replies: the factor that scales each
/// predicted need in the depth's claims, as a ratio of two byte totals
/// ([`Share::pace`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Pace {
    /// The bytes the answers built by processing the depth's finished
    /// replies have kept.
    pub(super) actual: u64,
    /// The needs predicted for those replies; never zero.
    pub(super) predicted: u64,
}

impl Pace {
    /// α of one: the prediction taken as it stands.
    pub(super) const PREDICTED: Self = Self {
        actual: 1,
        predicted: 1,
    };

    /// `need` scaled by α, rounded down.
    pub(super) fn scale(self, need: usize) -> u128 {
        need as u128 * u128::from(self.actual) / u128::from(self.predicted)
    }
}

/// The invitation share's state for each depth: its outstanding claims and
/// the two sums of its pace
/// ([The invitation share](self#the-invitation-share)).
///
/// It lives in the queue, so every change and every check holds the queue's
/// lock: a check reads a depth's claims and pace as whole admissions and
/// whole finished replies left them, never a finish half applied.
struct Share {
    /// The needs of each depth's questions that claimed (the rule's step 4)
    /// and whose replies this side's stage has not finished: each depth's
    /// outstanding claims, `N_d` in the share's check.
    ///
    /// A need joins as its question is granted and withdraws as the stage
    /// finishes the reply ([`processed`]). A question admitted but never
    /// built, which happens only as its session ends, leaves its need behind
    /// in a ledger nothing admits against again.
    claimed: [usize; DEPTHS],
    /// The needs predicted for the replies to questions about scopes at
    /// each depth, over the replies this side's stage has finished: the
    /// pace's predicted bytes for each depth.
    predicted: [u64; DEPTHS],
    /// Bytes the answers one level below each depth had kept as its latest
    /// reply finished: what processing its finished replies built, the
    /// pace's actual bytes for each depth.
    actual: [u64; DEPTHS],
}

impl Share {
    /// Add a question's `need` to the outstanding claims at `depth`, as the
    /// invitation share grants it against the budget.
    ///
    /// The sum saturates rather than wraps: reaching `usize::MAX` takes more
    /// live questions than an address space holds, and a saturated sum only
    /// holds every later question at the depth until it is exempt.
    fn claim(&mut self, depth: usize, need: usize) {
        self.claimed[depth] = self.claimed[depth].saturating_add(need);
    }

    /// Note that this side's stage has finished a reply to a question about
    /// a scope at `depth`, whose need was `need`.
    ///
    /// The question claimed its need if `claimed`, and the answers one level
    /// down have kept `answers` bytes
    /// ([`Ledger::processed`](super::Ledger::processed)).
    ///
    /// A question that claimed withdraws its need; any other claimed
    /// nothing. A need withdraws only after it was claimed, both under the
    /// queue's lock, so the claims never fall below it; the subtraction
    /// saturates so that a saturated sum ([`claim`](Self::claim)) cannot
    /// wrap on its way down. The predicted sum saturates too, for the same
    /// reason.
    fn finish(&mut self, depth: usize, need: usize, claimed: bool, answers: u64) {
        if claimed {
            let outstanding = &mut self.claimed[depth];
            debug_assert!(
                *outstanding >= need,
                "a finished reply withdraws only a need its question claimed",
            );
            *outstanding = outstanding.saturating_sub(need);
        }
        self.predicted[depth] = self.predicted[depth].saturating_add(counter::widen(need));
        self.actual[depth] = answers;
    }

    /// The pace of the replies to questions about scopes at `depth`: the one
    /// place `α` is chosen ([`Pace`]).
    ///
    /// A need predicts the answers one level down that processing a reply
    /// builds, and nothing but that processing builds answers there. So `α`
    /// is the bytes those answers had kept out of their grants as the latest
    /// reply finished, over the needs predicted for the replies finished so
    /// far ([`Ledger::processed`](super::Ledger::processed)): a ratio of
    /// sums, the mean and not a typical reply, since memory adds. Reading
    /// both at a reply's finish keeps a reply in progress out of both sums,
    /// so its prediction never stands against answers it has yet to build.
    /// `α` is one until a reply has finished, the prediction taken as it
    /// stands, and may exceed one: the prediction is a lower bound only for
    /// replies that list every grandchild their bounds allow, and answers
    /// can keep more than their least.
    ///
    /// It follows only what this side's own processing admits, so it is
    /// deterministic wherever the session is, and it paces admissions only:
    /// the bound and the liveness argument never read it.
    fn pace(&self, depth: usize) -> Pace {
        let predicted = self.predicted[depth];
        if predicted == 0 {
            return Pace::PREDICTED;
        }
        Pace {
            actual: self.actual[depth],
            predicted,
        }
    }
}

/// One unit waiting for admission.
struct Waiter {
    /// The unit asking.
    request: Request,
    /// Its order among the units its ledger queued, which breaks ties
    /// between waiting units at one depth.
    order: u64,
    /// The grant once admitted, and the task to wake then.
    state: Mutex<State>,
}

/// What a waiting unit has received.
#[derive(Default)]
struct State {
    /// The grant, once the unit is admitted and until its task takes it.
    grant: Option<Granted>,
    /// The task awaiting the grant.
    waker: Option<Waker>,
}

/// Lock `mutex`, whose data no panic can leave inconsistent: each critical
/// section here only flips flags and moves values.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The admission rule: return the grant that gives a unit permission to
/// build and send, or why it waits.
///
/// The unit `request` names is of its kind, concerns a scope at its depth,
/// and holds at most its bytes. The steps are the module's
/// [rule](self#the-rule), in its order; `budget_open` is false once a
/// deeper unit in this critical section waits for room.
///
/// The caller holds the queue's lock and passes the counts and the share's
/// state it keeps, so every value the steps read is one the ledger holds,
/// and a grant counts its bytes before the lock is released.
fn decide(
    account: &Arc<Account>,
    counts: &mut Counts,
    share: &mut Share,
    budget_open: bool,
    request: Request,
) -> Decision {
    let Request {
        unit,
        depth,
        bytes,
        need,
    } = request;
    let grant = widen(bytes);
    let held = counts.held[unit.index(depth)];
    let bound = account.bounds.of(unit, depth);
    // 1. Exempt: the kind and depth holds nothing, or the unit fits beside
    // what it holds within the bound. Nothing below can refuse it.
    if held == 0 || held + grant <= bound {
        let charge = Charge::granted(account, counts, unit.place(), bytes, (unit, depth), false);
        return Decision::Grant(Granted {
            charge,
            exempt: true,
        });
    }
    // 2. Room: the excess with the unit added must fit the budget, and no
    // deeper unit may be waiting for room.
    let with_unit = counts.excess - excess_beyond(held, bound) + excess_beyond(held + grant, bound);
    if !budget_open || with_unit > account.budget {
        return Decision::Room;
    }
    // 3. Share: a question under the invitation share also needs its
    // depth's claims, with its own, to fit the budget: the excess its
    // depth's questions hold and the needs of their unfinished replies,
    // scaled by the depth's pace, beside its own grant and need.
    if let Some(need) = need {
        let invited = excess_beyond(held, bound);
        let claimed = share.claimed[depth];
        let pace = share.pace(depth);
        let sum = invited as u128 + grant as u128 + pace.scale(claimed.saturating_add(need));
        #[cfg(any(test, feature = "test-internals"))]
        account.weigh(super::SharePeak {
            sum: usize::try_from(sum).unwrap_or(usize::MAX),
            depth,
            invited,
            grant,
            claimed,
            need,
            pace_actual: pace.actual,
            pace_predicted: pace.predicted,
            excess: counts.excess,
        });
        if sum > account.budget as u128 {
            return Decision::Share;
        }
        share.claim(depth, need);
    }
    // 4. Grant against the budget; a question under the share claimed.
    let charge = Charge::granted(
        account,
        counts,
        unit.place(),
        bytes,
        (unit, depth),
        need.is_some(),
    );
    Decision::Grant(Granted {
        charge,
        exempt: false,
    })
}

/// Admit every waiting unit the ledger now has room for, deepest first.
///
/// For the session-budget meter, everything this allocates is admission's
/// own ([`Holding::Admission`]), whichever place's release called it.
fn reconsider(account: &Arc<Account>) {
    let queue = lock(&account.admission);
    Holding::Admission.create(|| admit_waiting(account, queue));
}

/// Return the `bytes` a dropped charge for `place` holds, then admit every
/// waiting unit the ledger has room for ([`Charge`]'s drop).
///
/// The charge was built out of a grant to a unit of the kind and depth
/// `unit` gives, if admission granted it.
///
/// Both happen in one critical section of the queue's lock, so no decision sees
/// the bytes counted twice or not at all, and a release that empties a kind and
/// depth decides its waiter in the same section (step 4 of the liveness
/// argument).
pub(super) fn release(
    account: &Arc<Account>,
    place: Holding,
    bytes: usize,
    unit: Option<(Unit, usize)>,
) {
    let mut queue = lock(&account.admission);
    account.sub(&mut queue.counts, place, bytes, unit);
    Holding::Admission.create(|| admit_waiting(account, queue));
}

/// Note that this side's stage has finished a reply to a question about a
/// scope at `depth`, then admit every waiting unit the ledger has room for.
///
/// The question's need was `need`, and it claimed that need if `claimed`
/// ([`Ledger::processed`](super::Ledger::processed)). The claims and the
/// pace change under the queue's lock ([`Share`]), the pace's actual bytes
/// read as the reply finishes: what the answers one level down have kept.
pub(super) fn processed(account: &Arc<Account>, depth: usize, need: usize, claimed: bool) {
    let mut queue = lock(&account.admission);
    let answers = account
        .kept
        .get(depth + 1)
        .map_or(0, |kept| kept.load(Ordering::Relaxed));
    queue.share.finish(depth, need, claimed, answers);
    Holding::Admission.create(|| admit_waiting(account, queue));
}

/// Add the `bytes` an item for `place` needed beyond the grant it was built
/// out of, for a charge of `unit` ([`Charge::take`](super::Charge::take)).
///
/// It holds the queue's lock as it adds, as every change to the counts does
/// ([`Counts`]). An item falls short only where a grant fails to bound its
/// unit, which admission requires it to, so the lock costs a conforming
/// session nothing. The caller holds no admission lock.
pub(super) fn shortfall(
    account: &Account,
    place: Holding,
    bytes: usize,
    unit: Option<(Unit, usize)>,
) {
    let mut queue = lock(&account.admission);
    account.add(&mut queue.counts, place, bytes, unit);
}

/// Charge `bytes` for `place` outside admission
/// ([`Ledger::charge`](super::Ledger::charge)), under the queue's lock, as
/// the account takes every charge: a charge with no unit, whose bytes are
/// all excess.
#[cfg(test)]
pub(super) fn charge(account: &Arc<Account>, place: Holding, bytes: u32) -> Charge {
    let mut queue = lock(&account.admission);
    Charge::loose(account, &mut queue.counts, place, bytes)
}

/// The ledger's excess, `E`: what the budget counts ([`Counts`]).
#[cfg(test)]
pub(super) fn excess(account: &Account) -> usize {
    lock(&account.admission).counts.excess
}

/// The bytes charges built out of grants to units of `unit` kind at
/// `depth` hold, `held[k]`.
#[cfg(test)]
pub(super) fn held(account: &Account, unit: Unit, depth: usize) -> usize {
    lock(&account.admission).counts.held[unit.index(depth)]
}

/// The outstanding claims of the questions about scopes at `depth`.
#[cfg(test)]
pub(super) fn claimed(account: &Account, depth: usize) -> usize {
    lock(&account.admission).share.claimed[depth]
}

/// The invitation share's pace at `depth` ([`Share::pace`]).
#[cfg(test)]
pub(super) fn pace(account: &Account, depth: usize) -> Pace {
    lock(&account.admission).share.pace(depth)
}

/// Decide every waiting unit under the lock `queue` holds, deepest first
/// and then in the order they asked, granting each the rule admits, then
/// wake the units granted.
///
/// A unit that waits for room closes the budget to every shallower unit
/// decided after it in this critical section, which then only step 1 can
/// admit.
fn admit_waiting(account: &Arc<Account>, mut queue: MutexGuard<'_, Queue>) {
    let mut woken = Vec::new();
    let Queue {
        waiting,
        counts,
        share,
        ..
    } = &mut *queue;
    if !waiting.is_empty() {
        waiting.sort_by_key(|waiter| (Reverse(waiter.request.depth), waiter.order));
        let mut budget_open = true;
        waiting.retain(|waiter| {
            match decide(account, counts, share, budget_open, waiter.request) {
                Decision::Grant(grant) => {
                    let mut state = lock(&waiter.state);
                    state.grant = Some(grant);
                    woken.extend(state.waker.take());
                    false
                }
                Decision::Room => {
                    budget_open = false;
                    true
                }
                Decision::Share => true,
            }
        });
    }
    drop(queue);
    for waker in woken {
        waker.wake();
    }
}

/// One unit's admission, as the process's admission hook sees it: the unit
/// starting to wait, taking its grant, or its grant returning what the unit
/// did not use.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug)]
pub struct Admission {
    /// The ledger that admitted the unit, identified as in
    /// [`LedgerChange::ledger`](super::LedgerChange::ledger).
    pub ledger: usize,
    /// The unit's order among the units its ledger queued, which names it
    /// for as long as the ledger lives.
    ///
    /// It is `None` for a unit admitted at once, which was never queued and
    /// never waits, and in a report of a grant's return, which names no
    /// unit.
    pub order: Option<u64>,
    /// Whether this reports the unit starting to wait, because the ledger
    /// did not admit it as it was queued; its admission is reported later,
    /// with `waited` set.
    pub waiting: bool,
    /// Whether the unit is a question; otherwise it is an answer's own
    /// items.
    pub question: bool,
    /// The depth of the scope the unit concerns: the root's 0 through the
    /// leaves' 32.
    pub depth: usize,
    /// The grant's bytes: the most the unit's items can hold.
    pub bytes: usize,
    /// Whether the unit was exempt: admitted because its kind and depth held
    /// nothing or it fit within their bound, asking nothing of the budget.
    ///
    /// It is false in a report of a unit starting to wait and in a report
    /// of a grant's return.
    pub exempt: bool,
    /// Whether the unit waited for admission rather than being admitted
    /// at once or as it was queued.
    pub waited: bool,
    /// Whether this reports the grant's unused bytes returning once its
    /// unit was built, rather than the grant itself: `bytes` then counts
    /// what returned, and `waited` is false.
    pub leftover: bool,
    /// Whether this leftover is a part the grant returned before its unit
    /// was built, as soon as the unit was known not to need it; the rest
    /// returns, in a report without this flag, once the unit is built.
    pub partial: bool,
}

/// The process's admission hook, called as each unit takes its grant and
/// as each grant returns what its unit did not use.
#[cfg(any(test, feature = "test-internals"))]
pub type AdmissionHook = fn(Admission);

/// The admission hook every ledger in the process reports through, once
/// installed.
#[cfg(any(test, feature = "test-internals"))]
static ADMISSION_HOOK: std::sync::OnceLock<AdmissionHook> = std::sync::OnceLock::new();

/// Install `hook` as the process's admission hook, returning whether it is
/// now the installed one; the first installation wins.
#[cfg(any(test, feature = "test-internals"))]
pub fn observe_admissions(hook: AdmissionHook) -> bool {
    std::ptr::fn_addr_eq(*ADMISSION_HOOK.get_or_init(|| hook), hook)
}

/// Report a unit starting to wait to the installed hook.
#[cfg(any(test, feature = "test-internals"))]
fn waiting(account: &Account, waiter: &Waiter) {
    if let Some(hook) = ADMISSION_HOOK.get() {
        let Request {
            unit, depth, bytes, ..
        } = waiter.request;
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            order: Some(waiter.order),
            waiting: true,
            question: unit == Unit::Question,
            depth,
            bytes: widen(bytes),
            exempt: false,
            waited: false,
            leftover: false,
            partial: false,
        });
    }
}

/// Report a unit starting to wait: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
fn waiting(_: &Account, _: &Waiter) {}

/// Report the unit `request` names taking its grant to the installed hook,
/// with its queue order if it was queued, exempt if `exempt`, and count it
/// among the account's invited questions if it is one.
#[cfg(any(test, feature = "test-internals"))]
fn admitted(account: &Account, request: Request, order: Option<u64>, exempt: bool, waited: bool) {
    // The share weighs the questions about scopes below the root and above
    // the leaves; the opening and the leaf requests are admitted outside it.
    if request.unit == Unit::Question && !exempt && (1..DEPTHS - 1).contains(&request.depth) {
        account.invited.fetch_add(1, Ordering::Relaxed);
    }
    if let Some(hook) = ADMISSION_HOOK.get() {
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            order,
            waiting: false,
            question: request.unit == Unit::Question,
            depth: request.depth,
            bytes: widen(request.bytes),
            exempt,
            waited,
            leftover: false,
            partial: false,
        });
    }
}

/// Report a grant of `unit` kind at `depth` returning the `bytes` its unit
/// did not use to the installed hook, before its unit is built if
/// `partial`.
#[cfg(any(test, feature = "test-internals"))]
pub(super) fn settled(account: &Account, unit: Unit, depth: usize, bytes: usize, partial: bool) {
    if let Some(hook) = ADMISSION_HOOK.get() {
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            order: None,
            waiting: false,
            question: unit == Unit::Question,
            depth,
            bytes,
            exempt: false,
            waited: false,
            leftover: true,
            partial,
        });
    }
}

/// Report a unit taking its grant: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
fn admitted(_: &Account, _: Request, _: Option<u64>, _: bool, _: bool) {}

/// Wait for a unit's admission ([`Ledger::admit`](super::Ledger::admit)).
pub(crate) struct Admit {
    /// The ledger admitting the unit.
    account: Arc<Account>,
    /// The unit's grant or its place among the waiting units, until its
    /// task takes the grant.
    state: Option<Admitting>,
    /// Whether the unit was not admitted as it was queued.
    waited: bool,
}

/// Where an [`Admit`] stands before its task takes the grant.
enum Admitting {
    /// Admitted at once, with no unit waiting: the grant, which no other
    /// task can reach, and the unit it admits.
    Granted(Request, Granted),
    /// Queued among the waiting units, where [`reconsider`] leaves the grant.
    Queued(Arc<Waiter>),
}

impl Admit {
    /// Admit a unit of `unit` kind concerning a scope at `depth`, whose
    /// items take at most `bytes`, at once if nothing waits and the rule
    /// allows, or queue it and admit what the ledger has room for.
    ///
    /// For the session-budget meter, a waiting unit is admission's own
    /// ([`Holding::Admission`]); a unit admitted at once allocates nothing.
    ///
    /// A question whose `need` is given is governed by the invitation share,
    /// its reply predicted to need that many bytes processed.
    pub(super) fn new(
        account: Arc<Account>,
        unit: Unit,
        depth: usize,
        bytes: u32,
        need: Option<usize>,
    ) -> Self {
        assert!(depth < DEPTHS, "a scope sits at most 32 bytes deep");
        let request = Request {
            unit,
            depth,
            bytes,
            need,
        };
        Holding::Admission.create(|| Self::queue(account, request))
    }

    /// Admit a unit at once or queue it ([`Admit::new`]).
    fn queue(account: Arc<Account>, request: Request) -> Self {
        let waiter = {
            let mut queue = lock(&account.admission);
            // With no unit waiting, this unit is the only one `reconsider`
            // would consider, with the budget open, so deciding it here
            // admits exactly what queueing it would, without building a
            // waiter. The lock is held from the decision to the queueing,
            // so a unit this refuses queues as if it had never been tried.
            let Queue {
                waiting,
                counts,
                share,
                ..
            } = &mut *queue;
            if waiting.is_empty()
                && let Decision::Grant(grant) = decide(&account, counts, share, true, request)
            {
                drop(queue);
                return Self {
                    account,
                    state: Some(Admitting::Granted(request, grant)),
                    waited: false,
                };
            }
            let order = queue.next;
            queue.next += 1;
            let waiter = Arc::new(Waiter {
                request,
                order,
                state: Mutex::default(),
            });
            queue.waiting.push(waiter.clone());
            waiter
        };
        reconsider(&account);
        let waited = lock(&waiter.state).grant.is_none();
        if waited {
            account.waited.fetch_add(1, Ordering::Relaxed);
            waiting(&account, &waiter);
        }
        Self {
            account,
            state: Some(Admitting::Queued(waiter)),
            waited,
        }
    }
}

impl Future for Admit {
    type Output = Charge;

    /// Return the grant once the unit is admitted.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Charge> {
        let (request, order, grant) = match self
            .state
            .take()
            .expect("an admission is not polled after it returns its grant")
        {
            Admitting::Granted(request, grant) => (request, None, grant),
            Admitting::Queued(waiter) => {
                let mut state = lock(&waiter.state);
                let Some(grant) = state.grant.take() else {
                    state.waker = Some(Holding::Admission.create(|| cx.waker().clone()));
                    drop(state);
                    self.state = Some(Admitting::Queued(waiter));
                    return Poll::Pending;
                };
                (waiter.request, Some(waiter.order), grant)
            }
        };
        let Granted { charge, exempt } = grant;
        admitted(&self.account, request, order, exempt, self.waited);
        Poll::Ready(charge)
    }
}

impl Drop for Admit {
    /// Withdraw a unit no longer awaited, returning a grant it never took.
    fn drop(&mut self) {
        let Some(Admitting::Queued(waiter)) = self.state.take() else {
            // A grant made at once returns as it drops here, which
            // reconsiders the waiting units itself.
            return;
        };
        lock(&self.account.admission)
            .waiting
            .retain(|queued| !Arc::ptr_eq(queued, &waiter));
        let unclaimed = lock(&waiter.state).grant.take();
        drop(unclaimed);
        // A withdrawn unit may have closed the budget for shallower ones.
        reconsider(&self.account);
    }
}

#[cfg(test)]
mod tests;
