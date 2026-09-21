**1. Running narration**

I read only the three named files. The comments on the appendix concern its organization, instructions, and internal consistency; I have not checked its claims against code.

**[README.md](/private/tmp/claude-506/-Users-oxide-src-rumors/5acd525e-86be-4ee4-9781-3810a2a15e79/scratchpad/codex-review/README.md)**

“A design, not yet implemented” is an excellent opening. Together with “no number in it has been measured yet,” it establishes the status honestly. Keep both.

The first paragraph then gives me more implementation vocabulary than orientation: “own window,” “one fan,” decoded-reply queues, and `Link`, all before I know the conversation being bounded. I understand the proposed mechanism broadly, but cannot yet distinguish its essential insight from its implementation.

Three promises subsequently need correction:

- “under a fixed memory bound” initially sounds like a guaranteed byte bound, whereas §7 describes statistical sizing that can exceed the estimate.
- “a resizing, not a re-plumbing” understates the appendix’s scope-publication changes and separately scheduled opening decoder.
- “plus one fan” omits the additional slot that the exposition later calls essential.

“Rewritten whole after each of two fresh-eyes reading rounds” does not help me understand or assess this version. The revision history can go.

The distinction between exposition and implementation plan is useful. But “the public-API shape is proposed for the owner’s ruling” leaves the decision state unclear once I reach the recorded rulings and the appendix’s “Ruled 2026-09-20.”

“QUIC loss isolation buys nothing for logically coupled streams” immediately raises a technical objection. A final join does not eliminate the benefit of allowing other work to advance while one input recovers from loss. That rationale needs evidence or qualification even if the decision to remove the bundle is settled.

The prior-note links are acceptable provenance. I did not follow them, and the argument should not require them.

**[exposition.md](/private/tmp/claude-506/-Users-oxide-src-rumors/5acd525e-86be-4ee4-9781-3810a2a15e79/scratchpad/codex-review/exposition.md), §1 — The claim**

The first paragraph establishes a concrete problem and why anyone should care. “One level’s stream [must] keep flowing while another’s is blocked” is the right conceptual introduction.

The boxed counting claim is also effective: it tells me what result to look for without giving me a formula before its terms exist.

“A socket that is always drained never couples one level to another” is too broad. Continuous drainage can remove a protocol-induced circular wait; serialization and loss still couple progress. Section 8 acknowledges those effects, but this opening states the stronger proposition.

“The room already exists” also briefly misleads me: the *queue* exists, but the additional storage does not come free. The distinction matters because memory is one of the advertised guarantees.

“No queue is added and no edge is re-plumbed” is a claim I carried forward and eventually had to retract when reading the appendix. The clean conceptual story should be preserved, but the actual prerequisite is stronger than resizing one queue.

The roadmap is useful and earns its place.

**§2 — The conversation**

The opening tree description gives me the expected foundation. Defining a fan in the course of describing the tree is natural and effective.

I stop at “thirty-two levels deep, with single-child runs compressed away.” Later, every query advances exactly one level. I need to know whether “level” counts address bytes, retained tree nodes, or protocol steps, and how the protocol crosses a compressed run. Otherwise the fixed-depth arithmetic and alternating ownership are not yet grounded.

“A version advances on every send and every redaction, so equal versions mean the same history and the same set” skips an invariant. Advancing on each event does not, by itself, establish that equal versions identify equal histories. If this causal version identifies the complete relevant history, say that directly.

The three reactions are a good teaching device. The supply explanation also does the right thing with deletion semantics: it gives me enough context, then explicitly removes that mechanism from the subsequent argument.

The query definition is where I first need a concrete exchange. A nonempty query means “compare these children”; an empty query means “send me the node I lack.” Those are understandable, but the change of perspective between asker and responder is easy to lose.

“A held node’s listing is never empty” appears false for a leaf under the tree description given so far. It also leaves the empty and singleton tree cases unresolved. There may be a terminal representation that makes the statement correct, but it has not been introduced.

My first viable mental model forms at:

> “A query is the next question, one level down…”

That sentence explains why a reply is also the vehicle for the next step. It is the conceptual hinge of the protocol. Keep it prominent.

The level convention immediately following it is precise, although dense. I reread it once to distinguish the depth of the node being asked about from the level assigned to its question. A small example would make that reread unnecessary.

The opening shortcut is understandable on its own. “The initiator answers each with nothing further” would be clearer if it explicitly named the empty reply that still participates in positional pairing. “Nothing further” initially sounds like no response at all.

“The walk ends at the disjoint frontier” needs qualification: matching subtrees are held by both sides and are also places where traversal stops. I infer that this frontier concerns the remaining disputed branches, after matching branches have been removed from consideration.

“Work is proportional to the difference” is a broader performance claim than the preceding explanation establishes. The traversal also visits shared prefixes and inspects fans; the statement needs a cost model or softer wording.

“For two fully divergent sets of a million messages the frontier lies about five levels down” is useful intuition, but I cannot tell whether it describes a typical path or the deepest remaining overlap. Those are different statistics.

The positional-pairing paragraph is particularly good. “To interpret the reply, the asker must have kept the question” makes the retained local state necessary, rather than arbitrary.

However, a *question* has now meant both a transmitted digest listing and a retained collection of local handles. That distinction is central to the later count and deserves explicit wording here.

“Every large message is a supply” is not literally compatible with the later 1.5 MB reply made of query listings. The intended distinction seems to be that supplies contain arbitrarily large subtree content, while other reply content has a structural bound.

The Rust sketch earns its space by making the positional-versus-named distinction concrete. It does not quite replace the missing worked exchange.

**§3 — V1: a level at a time**

This section is short, and the memory disadvantage is easy to understand.

The latency claim does not follow from the protocol described:

> “And each level costs a full round trip…”

You have just described the responder sending level 1, the initiator sending level 2, the responder sending level 3, and so on. That is one crossing per level. Since replies already contain the next questions, I see no extra return crossing at each level.

Consequently, “`2(L + 1)` one-way crossings” and the later claimed halving of that count are unsupported as written. Either V1 has an additional exchange phase that has been omitted, or the comparison is wrong. Waiting to aggregate a whole level can increase elapsed time without doubling the number of causally necessary network crossings.

“At any moment exactly one message is in flight” also needs to be restricted to this main descent, because the earlier greeting exchange and opening supplies complicate the literal session-wide statement.

“Simple and hand-verifiably correct” asks for agreement rather than adding an explanation. The mechanics already establish the useful kind of simplicity.

**§4 — V2: every level at once**

“One reply per question rather than one message per level” is a clear explanation of the change. This teaches something that the V1 setup makes easier to understand.

“Roughly half of V1’s” inherits the unresolved latency problem. I believe the benefits of streaming and overlapping work; I do not yet believe the stated factor of two.

“Each reply is sent as soon as its question has been read” should be understood subject to the stage’s other blocking operations. As written, it sounds stronger than the window and publication rules subsequently allow.

**§4.1 — Stages**

This is the densest paragraph in the document. It introduces pairing, reaction processing, outgoing replies, resolutions, deeper questions, and assemblers together.

I particularly stumble over:

> “for the node it has just processed”

Which node: the one addressed by the incoming question, or a child whose query has just been answered? One incoming reply can contain many queries. The proof later depends on recording the questions from *each outgoing reply* before sending another. I need that loop granularity here.

The stage diagram is useful. It makes the odd/even ownership much easier to retain than the prose alone.

Its opening is misleading as a temporal diagram, however. It puts the initiator’s early supplies above its greeting and never shows the responder’s greeting, although both listings are required before those supplies can be selected. Mark a completed greeting exchange before showing the shortcut, or explicitly say the drawing is not chronological there.

“Seen whole, the session is one chain of stages” is a useful view of the wire-facing spine, but not of the complete wait graph just described: local question queues skip a peer stage, and assemblers return results upward. The proof must account for those additional edges.

**§4.2 — The window**

This section works well. It explains why the window exists, what a slot retains, and the throughput–memory tradeoff.

“A level whose disputes outrun the estimate waits for slots” is an especially helpful distinction between estimating useful concurrency and depending on that estimate for correctness.

I would keep this section mostly intact. The later memory discussion should make equally clear that limiting the *number* of entries does not guarantee that their estimated *byte price* is accurate.

**§4.3 — Why it cannot deadlock**

This is the first place where the document presents a proof that I cannot reconstruct.

> “whenever a stage is blocked reading, the item it needs is owed by a stage that already holds everything required to produce it”

That is not established for every read. A stage can be waiting for its next local question before its producer has received the input needed to create it. Perhaps the intended proof concerns only reads associated with an already-issued obligation; that restriction needs to be stated.

“Every chain of waits descends” therefore does not follow yet. The text explicitly introduced waits toward earlier stages, then appears to eliminate them in one sentence without showing how each relevant case reduces to a deeper wait.

“The bottom never waits” is also too strong. The useful property would be that it never requires a result from a deeper stage.

The return-queue paragraph identifies the right complication, but I cannot derive its occupancy bound from the publication order alone without a clearer account of assembler inputs, outputs, and processing order.

Finally:

> “each assembler’s return queue is one fan wide”

followed by:

> “One slot per queue therefore suffices for progress”

is inconsistent literally. Specify which queue classes can have capacity one.

The independence requirement at the end is excellent. It states exactly the property that naive multiplexing can violate, and it prepares §5 well.

The explanation of seventeen streams remains incomplete. I can count sixteen odd or even address levels, but I cannot place the extra leaf stream and the special opening consistently from the current description.

**§5 — One socket, and the deadlock**

This is one of the strongest sections. It gives head-of-line blocking a concrete dependency cycle rather than treating it as a generic performance concern.

The wedge’s level arithmetic works: stage 2 produces questions at 4; stage 3 answers them at 4 and introduces questions at 5; stage 4 answers at 5 and records questions at 6.

I want slightly more buffer bookkeeping in steps 3–5. After stage 4 blocks while publishing a resolution, the decoded-reply queue can still accept a reply before the demultiplexer itself blocks. Showing where the remaining supplies sit would make the example fully reconstructible and explain the choice of six.

“Those slots fill from the initiator’s replies at level 6” compresses one useful link: stage 6 processes those replies, and its assembled results satisfy stage 4’s pending slots. State that link explicitly.

“The opening batch makes the same shape” is plausible but not demonstrated. Since the opening subsequently needs a special decoder task and buffer, give its corresponding dependency in another sentence or two.

The three remedies explain the design space reasonably well. The historical assertions about earlier attempts are less important than the mechanisms.

“The third was rejected on a miscount” is an effective transition, but §6 later says the quantity was “left … open.” An unresolved bound and an incorrect count are different histories; choose the accurate description.

**§6.1 — The count**

This is the best argument in the document.

The partition into queued questions, one question held by the consumer, and one not-yet-recorded fan is easy to understand. In particular:

> “it sends no further reply until every question of the current one is recorded”

is the sentence that finally makes the bound reconstructible. It belongs in the stage description as well, because the counting argument is leaning on a scheduling rule, not merely a queue capacity.

There are three loose ends:

- The greeting’s level-1 question is not covered by “stage ℓ − 2, or at level 2 by the opening.”
- “Consumed” needs a precise event: pairing/dequeue, or completion of processing? “Its reply awaited or in hand” spans those events.
- “Replies are all there is” should be scoped to ordinary descent traffic and expressed in reply units, not as an unqualified statement about everything on the wire.

The count itself looks like a sound **sufficient upper bound under the stated sequential-production rules**. I do not see a proof here that it is the minimum capacity needed for liveness.

**§6.2 — Parking**

The central move is convincing: if an arriving reply always has a slot, receiving it cannot wait for its stage to advance.

But the crucial implementation premise arrives as an assertion:

> “The decoder’s only dependency is the storage backend, never a stage…”

The appendix reveals that this is something the implementation must be changed to establish. Moving scope publication and independently driving the opening decoder are part of the liveness mechanism. The exposition cannot make them disappear into “the same queue, on the same edge.”

The sender paragraph correctly states a non-starvation requirement. Keep that qualification.

“A queue that is never full” is not the property the count proves. A queue can hold exactly its capacity harmlessly, provided no further legal reply can arrive until an entry is consumed. The document needs to distinguish:

- occupancy reaching capacity;
- a reply arriving when occupancy is already at capacity.

The later proposed tests expose this distinction directly.

“A parking depth fixed on its own … fails as soon as a session’s window … exceeds it” overstates the conclusion. Exceeding the chosen depth removes the proposed general guarantee; it does not make every such session fail.

“Not the window” also needs qualification: §7 explicitly changes the price calculation and therefore the granted window.

**§7 — Memory**

“‘Decoded’ does the work” is a strong opening. The distinction between retaining subtree bytes and retaining a backend handle is essential, and it is explained clearly.

It is also a premise of §6’s drainage claim, so a short version belongs before that claim is made.

“The content it carried is content the replica is about to hold in any case” explains custody, but not a total memory bound. In an in-memory backend, absorbing content still consumes memory. State explicitly that the budget covers protocol working state and excludes retained replica content, including content awaiting commit.

“Nothing for a match” should mean no payload beyond the reaction representation, not zero allocation cost.

“About 1.5 MB” needs its digest or listing-entry size. The 256-bit address does not tell me the digest width.

“A shape that exists only near the root of a very large tree” is false as a structural statement. Clustered addresses can produce a full fan of full fans deeper in the tree. It may be a statement about likely shapes under the uniform model.

The paragraph beginning “Parking is not a second window” works very well. It explains why the receiver’s own admission of work can size its replies without learning the peer’s window.

The fixed-floor discussion needs the structural cap’s actual formula in the exposition. The “few megabytes” result depends substantially on that cap; its role should not be discoverable only in the implementation appendix.

The paragraph separating absolute reply counts from statistical byte estimates is excellent and necessary. It comes much too late relative to the initial memory promise.

“Pure bandwidth-delay storage, with no protocol state in it” overstates what has been established about transport buffers. They still hold protocol bytes, and their residence time can include decoding and backend service. The useful claim is that their capacity no longer has to enforce per-level consumption pacing.

**§8 — Costs, removals, and evidence**

The distinction between adding no credit exchange and retaining the same dependency-hop count is useful. It does not justify literal immediacy in “the sender writes each reply the moment it is produced”: a multiplexer may have other frames ahead of it.

“Kept off the critical path” is wrong as a general consequence of preferring question-bearing frames. Priority cannot preempt a frame already being transmitted or bytes already queued in the transport. The frame bound limits one frame’s size, not necessarily the total backlog ahead of a newly ready reply.

The loss-coupling discussion repeats the README’s unsupported reasoning in weaker form. A session waiting for its slowest component can still benefit from other components doing useful work during that wait.

The API-removal discussion is understandable, but most of the by-value/by-reference detail belongs in the appendix. The exposition needs the new transport contract and failure rule, not the full API rationale.

The proposed negative control—same wedge, inadequate versus derived parking—is strong evidence to seek.

The tightness test is a separate problem:

> “parks exactly `K(ℓ) + 257` replies and stalls at one less”

A reachable peak occupancy does not establish that a smaller queue deadlocks. It may merely make one enqueue wait until a runnable consumer executes. That distinction is especially important when the extra slot represents the consumer’s already-dequeued question.

The final Lean paragraph establishes that no existing formal result is being invoked, which is useful status information. The model-assistance provenance and comparative trust commentary do little for this argument and could be consolidated into a brief note.

**[appendix-implementation-plan.md](/private/tmp/claude-506/-Users-oxide-src-rumors/5acd525e-86be-4ee4-9781-3810a2a15e79/scratchpad/codex-review/appendix-implementation-plan.md)**

The overall order is good: instrument, modify reception on the existing transport, demonstrate the single pipe, then change the API. Commit boundaries and named acceptance conditions make the plan usable.

The introductory height definition needs an explicit mapping, not just “where the exposition said level.” A question’s level refers to its listed children; the node it asks about is one level above. That makes a casual coordinate substitution dangerous.

The “What does not change” inventory is appropriate for this audience. I cannot verify it, but it gives the implementer a clear intended boundary.

**Step 0.** The quantity is much more reviewable because its events are named. However, “dequeued and matched with a reply” is not the same event as receiving a question from `InternalChildQueries`: the exposition says the stage dequeues the question and then waits for its reply. The two proposed measurements need reconciliation.

**Step 1.** This is where the exposition’s simplifying claims stop matching the plan. The changes include multiple capacities, publication timing, and a new independently driven buffer.

“Capacity monotonicity” can justify widening a channel; it does not, by itself, justify moving publication or changing `next_scopes` to a capacity that may be smaller than its former window-derived capacity. Separate those obligations.

The scope-order explanation needs especially careful naming. The new scopes are published “immediately before the reply is handed to the walk,” while the retained ordering role is called “reply before its scopes.” These may refer to different events or different replies, but the text makes them sound contradictory.

Item 5 contains a derivation, a worked numerical estimate, allocation-size instructions, benchmark regeneration, test rebasing, and documentation changes. Split it into shorter actionable units.

The numerical example should identify what `n` represents and whether “3.4 MB in all” is a per-party conservative charge or aggregate session memory.

**Step 2.** The known-bad/cure pairing is well designed as a plan.

The occupancy requirements contradict each other directly: “no … queue ever reaches its capacity” and “parks exactly … capacity.” That must be settled before someone implements the tests.

The proposed delayed-consumer construction also does not yet distinguish a permanent stall from temporarily withholding scheduling from a runnable consumer.

“One `tokio::io::duplex` pair per direction” leaves the number and use of pipes unclear. State the topology explicitly.

The fallback that folds this demonstration into step 3 is a reasonable contingency, but it weakens the milestone promised by “before touching the API.” Make the contingency an explicit change to the step ordering and acceptance conditions.

“Any stall … is a finding about the count, not about the transport” is too restrictive as a diagnostic instruction. A stall could also expose a violated decoder-independence premise or a mux/demux progress defect.

**Step 3.** The breakdown into link, proxy, wire, drivers, and prose is useful for a codebase-aware reader.

“A failed session keeps the halves” is imprecise ownership language: distinguish not returning owned halves from retaining them somewhere. Likewise, a caller passing references retains access after the borrow ends; that is different from the function returning them in an error result.

The prose sweep names `link.rs` after the plan has deleted it. Clarify whether that entry means migrating its documentation.

The code/prose commit split should specify which documentation must move atomically to preserve the earlier promise that every intermediate tree is gate-clean.

**Step 4.** “Round-robin at frame granularity is live under any policy” is unclear. Use the exposition’s non-starvation condition.

The exposition describes question priority as part of the solution, whereas this step makes it a possible later optimization. Align those descriptions.

**Step 5, acceptance, and risks.** Recording the resulting decision and correcting historical summaries are reasonable final tasks. The acceptance list should explicitly carry forward the corrected occupancy invariant, rather than leaving the most important property implicit in the general gate.

The byte-budget qualification deserves visibility here too: the count can be correct while actual working memory exceeds the statistical estimate.

---

**2. Structural critique**

The V1 → V2 → single-socket sequence does teach, but unevenly. The transition from streaming to the deadlock wedge is particularly effective. The transition from V1 to V2 currently teaches an unsupported latency story.

My first protocol model formed at:

> “A query is the next question, one level down…”

My operational model improved substantially at the stage diagram. My model became sufficient to reconstruct the count only at:

> “it sends no further reply until every question of the current one is recorded.”

That last fact arrives too late. It is a defining scheduling constraint of V2, not a fact that should first appear while proving the new result.

The material preceding the first hinge sentence is mostly necessary. The tree, listing, and reactions earn their place. The greeting’s complete inventory of bounds is less useful at that moment; some of it could wait until framing or memory sizing needs it.

Every main section has a legitimate role. I would preserve the overall order, with these changes:

1. Repair V1’s exchange trace before making any latency comparison.
2. Make the exact per-outgoing-reply publication sequence explicit in §4.1.
3. Make §4.3 either a reconstructible progress argument or an explicitly stated prerequisite lemma with clearly identified assumptions.
4. Introduce incremental absorption and decoder independence before concluding that parking permits continuous drainage.
5. State the memory accounting boundary and statistical qualification near the initial claim.

The biggest structural problem is that **three distinct claims are repeatedly treated as one**:

- There is a bounded number of replies.
- Receiving a reply can finish without waiting for the walk.
- The resulting working state fits a particular byte budget.

The count proves the first. Scope-publication changes and incremental backend absorption support the second. The memory model supports the third, statistically. Separating these obligations would make the argument shorter *and* stronger.

Some repetition is helpful: the claim preview, formal count, and memory interpretation address different reader needs. Other repetition is not:

- “Only resizing/no re-plumbing” is repeated despite being incomplete.
- Transport-abstraction removal occupies the README, the opening claim, and the conclusion before receiving its full appendix treatment.
- Historical campaign and formal-development commentary interrupts the otherwise direct argument.
- The extra-slot story is repeated without distinguishing sufficient capacity, peak occupancy, and necessary capacity.

On conventions: **the interior level arithmetic is consistent; the endpoint conventions are incomplete.** The prose, diagram, and wedge agree that stage ℓ receives replies at ℓ, sends replies at ℓ + 1, and records questions at ℓ + 2. The unresolved parts are compressed paths, leaf questions, the greeting’s exceptional producer, the extra stream, and the appendix’s height conversion.

A small worked exchange would earn its space. It should identify one node, its children, one incoming question, the outgoing reply, and the next questions. It would resolve more confusion than additional general description.

---

**3. The argument itself**

My restatement is:

At a given level, this receiver limits how many questions it can cause to become outstanding. Its local queue can hold `K` question records; its consuming stage can hold one additional question; and its producing stage can have sent at most one further batch of at most `F = 256` questions before blocking on recording them. Therefore:

```text
outstanding questions ≤ K + 1 + F
```

A conforming peer sends at most one reply per question. Consequently, no more than that many corresponding replies can be outstanding.

Allocate that many decoded-reply slots. If a new reply arrives, it is one of the bounded outstanding replies, so the parking queue cannot already contain the entire bound *in addition to that reply*. Thus the new reply has room.

If decoding can finish independently of stage progress, the receiver can continue through frames for blocked stages and reach frames that unblock deeper work. That removes the additional circular wait introduced by sharing one ordered stream. The existing walk’s progress argument can then apply, assuming its other prerequisites remain satisfied.

Large supplied subtrees do not have to occupy parking storage as byte strings: their content is absorbed incrementally into the backend, leaving bounded reply metadata and handles. The number of parked replies has a deterministic bound; the proposed economical byte sizing uses statistical tree-shape estimates.

That is a strong central idea. These are the points I still have to take on faith, or reject as stated:

1. **The protocol’s terminal and compressed-tree cases fit the stated units.**  
   The text has not shown how a held leaf has a nonempty listing, how compressed runs advance through levels, or how these cases map to streams. The document can close this with explicit endpoint rules and one small example.

2. **The count’s partition is exhaustive at the actual scheduling boundary.**  
   I accept the arithmetic once there is exactly one producer batch not yet recorded and at most one question held by the consumer. The stage description should establish those facts before the count, including what “sent” and “consumed” mean.

3. **The initial level fits the invariant.**  
   The greeting’s question needs its own one-question base case. The opening supply batch is a separate bounded exception, as the text already recognizes.

4. **The baseline walk is live under independent reply queues.**  
   Section 4.3 does not yet prove this to me. A compact account of each queue’s producer, consumer, capacity, and publication order, followed by a precise wait argument, could close the gap. No formalization is necessary.

5. **Decoded reception has no dependency that can reintroduce the cycle.**  
   The appendix indicates that this requires work, especially around scopes and the opening decoder. The exposition should identify those dependencies and explain why the proposed changes remove them.

6. **Backend operations and transport tasks eventually make progress.**  
   “Its only dependency is the storage backend” does not logically imply “it always finishes.” State the ordinary liveness assumptions: backend operations complete independently of walk progress; ready transport work is scheduled; the peer conforms.

7. **The selected capacity is necessary, rather than merely sufficient.**  
   This is not established. In particular, if the extra outstanding question is already held by a stage awaiting its reply, filling parking while withholding that stage’s scheduling can show a peak occupancy. It does not show a permanent deadlock with one fewer slot. Supply a closed wait cycle under a fair scheduler, or drop the necessity claim.

8. **The byte accounting covers all newly retained state.**  
   The exposition prices reply reactions, while the appendix also changes scope custody and auxiliary queues. Identify the complete accounting boundary. The requested-budget guarantee remains statistical; a much larger structural worst-case byte ceiling may also exist and should not be confused with that estimate.

9. **V1 has twice the depth-dependent crossing count.**  
   As described, it does not: alternating level messages already advance one level per crossing. Show the missing exchange or correct the claim.

10. **Priority removes frame delay from the critical path.**  
    It cannot generally do that. Existing transmission and queued bytes remain ahead of a newly ready reply. Distinguish a maximum frame size from a bound on aggregate queued delay.

11. **Logical coupling makes loss isolation valueless.**  
    This is false in general. Independent work can overlap loss recovery and shorten later critical-path work even when all branches ultimately join. The decision may be reasonable, but this justification does not establish it.

12. **Tests constitute proofs of the universal claims.**  
    The proposed tests are useful evidence and checks of correspondence with the implementation. They do not establish the invariant for every execution or establish minimum necessary capacity merely by reaching a high-water mark.

I am comfortable accepting the implementation names, historical account, and current-code descriptions as author-supplied facts for this reading. I cannot independently confirm them, and the argument should distinguish those premises from deductions made in the document.

---

**4. Complete ranked list of fixes**

Here, **R** means README, **E** means exposition, and **A** means appendix. Earlier entries affect correctness or the central explanation; later entries concern presentation and polish.

1. **E §3: “each level costs a full round trip.”** The described alternating exchange uses one crossing per level, so show the missing phase or correct both this count and the claimed V2 speedup. **[argument]**

2. **E §6.2: “The decoder’s only dependency is the storage backend, never a stage.”** Establish this prerequisite explicitly, including the scope and opening-decoder changes disclosed in the appendix. **[argument]**

3. **E §4.3: “Every chain of waits descends.”** The preceding treatment does not eliminate all upward read dependencies or account sufficiently for assembler edges. **[argument]**

4. **A Step 1: “asserts it never reaches capacity”; A Step 2: “parks exactly … `+ FAN + 1` replies.”** These requirements contradict each other; assert that no arriving reply finds an already-full queue instead. **[clarity]**

5. **E §8: “stalls at one less, the proof that the `+ 1` is real.”** Peak occupancy establishes neither permanent deadlock nor minimum necessary capacity; require a closed wait cycle or withdraw the tightness claim. **[argument]**

6. **R: “under a fixed memory bound.”** State upfront whether this means a deterministic structural ceiling or the statistical byte estimate later allowed to be exceeded. **[argument]**

7. **E §2: “A held node’s listing is never empty.”** Explain the leaf representation and empty/singleton cases before relying on this distinction. **[argument]**

8. **E §2: “with single-child runs compressed away.”** Explain how compressed paths coexist with queries advancing exactly one level and with fixed level parity. **[clarity]**

9. **E §1: “No queue is added and no edge is re-plumbed.”** Reconcile this with the new opening buffer/task and changed scope-publication timing, and distinguish the conceptual cure from the complete change. **[clarity]**

10. **E §8: “kept off the critical path.”** Question priority cannot bypass an in-progress frame or existing transport backlog, so replace the asserted guarantee with the actual scheduling benefit. **[argument]**

11. **R: “QUIC loss isolation buys nothing for logically coupled streams.”** A final dependency join does not prevent useful work from overlapping loss recovery. **[argument]**

12. **E §4.3: “One slot per queue therefore suffices for progress.”** Identify the queue classes covered, because return queues were just required to hold a fan. **[argument]**

13. **E §4.1: “for the node it has just processed.”** Identify whether this is the incoming question’s node or an individual queried child, and specify the per-outgoing-reply loop boundary. **[clarity]**

14. **A Step 0: “dequeued and matched with a reply” / “read the `InternalChildQueries` receive count.”** These describe different events under the exposition’s dequeue-then-wait rule, so define one exact measurement. **[clarity]**

15. **A Step 1: “immediately before the reply is handed to the walk” / “reply before its scopes.”** Name the relevant replies and publication events so these ordering instructions no longer appear reversed. **[clarity]**

16. **A Step 1: “Capacity monotonicity means widening a receive-side channel cannot cost liveness.”** Separate that justification from the obligations created by publication moves and potentially smaller `next_scopes` capacity. **[clarity]**

17. **E §7: “content the replica is about to hold in any case.”** State that backend content, including uncommitted content, is outside the protocol-working-memory budget being discussed. **[clarity]**

18. **E §7: “The bound on their bytes is statistical.”** Specify what confidence or estimation guarantee is intended and distinguish it from the finite structural worst-case bound. **[argument]**

19. **E §6.1: “They are asked by stage ℓ − 2, or at level 2 by the opening.”** Add the initiator greeting’s level-1 base case. **[argument]**

20. **E §6.1: “its reply awaited or in hand.”** Define when a reply counts as consumed so the invariant, pairing instrumentation, and parked occupancy use consistent lifetimes. **[clarity]**

21. **R: “plus one fan.”** Include the extra slot or explicitly label this as an informal summary of `K + FAN + 1`. **[clarity]**

22. **E §6.1: “replies are all there is.”** Restrict the claim to ordinary descent traffic and make clear that the bound counts reply objects rather than wire bytes or control items. **[clarity]**

23. **E §7: “a shape that exists only near the root of a very large tree.”** Qualify this as a statistical observation, because clustered addresses can produce the shape deeper down. **[argument]**

24. **E §2: “Every large message is a supply.”** Say that only supplies carry arbitrarily large subtree content; query-bearing replies can themselves be large. **[clarity]**

25. **E §8: “Round trips are unchanged.”** Define the dependency-hop metric and distinguish it from elapsed latency under serialization, queuing, and loss. **[clarity]**

26. **E §4.3: “The bottom never waits.”** Replace this with the narrower property that terminal work does not depend on a deeper stage, and state the external progress assumptions. **[argument]**

27. **E §6.2: “fails as soon as a session’s window at that level exceeds it.”** A larger configured window invalidates the proposed universal guarantee but does not force every such session to fail. **[argument]**

28. **E §5: “then six fat supplies.”** Show the relevant queue occupancies through the blocked state, including the decoded-reply slot, so the chosen wedge can be reconstructed. **[clarity]**

29. **E §5: “Those slots fill from the initiator’s replies at level 6.”** Include stage 6’s processing and return to stage 4’s assembler in the dependency chain. **[clarity]**

30. **E §5: “The opening batch makes the same shape.”** State the opening-specific cycle because its remedy is a separate part of the implementation. **[argument]**

31. **E §4.1 diagram: “opening: supplies for the root children.”** Show both greetings before this action or mark the opening drawing as nonchronological. **[clarity]**

32. **E §4.3: “sixteen levels … and one for the leaves.”** Enumerate the endpoints and special stream roles so seventeen streams follows from the stated level model. **[clarity]**

33. **A introduction: “where the exposition said level.”** Give an explicit height/level mapping that distinguishes a question’s level from the depth of its subject node. **[clarity]**

34. **E §2: “must have kept the question: its own handles.”** Distinguish the transmitted listing from the retained local question record when the latter is introduced. **[clarity]**

35. **E §6.1: “it sends no further reply until every question of the current one is recorded.”** Introduce this scheduling rule in §4.1, where the reader first needs to understand stage behavior. **[structure]**

36. **E §6.2: “not the window.”** Say that the window mechanism remains, while its pricing and resulting granted capacity change. **[clarity]**

37. **E §7: “capped by the number of nodes a level can hold at all.”** Put the structural cap formula in the exposition because it materially supports the small-floor claim. **[clarity]**

38. **E §7: “about 1.5 MB of digests.”** Supply the digest/listing-entry byte size and distinguish payload size from total allocated reply size. **[clarity]**

39. **E §7: “nothing for a match.”** Use “no additional payload” so the statement does not erase the reaction slot’s representation cost. **[clarity]**

40. **E §7: “now priced beside the local half.”** Identify where retained scopes, changed auxiliary queues, and opening parking enter the working-state accounting. **[clarity]**

41. **E §7: “pure bandwidth-delay storage, with no protocol state in it.”** State the narrower result that transport buffers no longer have to enforce per-level consumption pacing. **[clarity]**

42. **E §2: “A version advances … so equal versions mean the same history.”** Name the history-identification invariant rather than deriving it merely from advancement. **[argument]**

43. **E §2: “the cut below which every subtree is held by one side only.”** Restrict the disjoint frontier to unresolved branches after matching subtrees have been pruned. **[clarity]**

44. **E §2: “work is proportional to the difference between the replicas.”** Specify the work measure and treatment of shared-prefix traversal, or soften the performance claim. **[clarity]**

45. **E §2: “the frontier lies about five levels down.”** Identify whether this describes typical or deepest overlap and give a brief basis for the estimate. **[clarity]**

46. **E §4: “keeps V1’s messages and changes their granularity.”** Say it keeps the logical reactions or exchange rules, since the next sentence changes the message boundaries. **[clarity]**

47. **E §3: “at any moment exactly one message is in flight.”** Restrict this statement to the serialized descent rather than the entire opening and session. **[clarity]**

48. **E §1: “TCP needs a connection per stream.”** Qualify this relative to the chosen transport contract, since the later explicit-credit alternative can multiplex over TCP. **[clarity]**

49. **E §5: “rejected on a miscount.”** Reconcile this with §6’s description of the quantity as left open. **[clarity]**

50. **A Step 2: “Any stall … is a finding about the count, not about the transport.”** Allow investigation of violated decoder-independence and mux/demux progress premises as well as the arithmetic bound. **[clarity]**

51. **A Step 2: “one `tokio::io::duplex` pair per direction.”** State the exact endpoint and pipe topology so the demonstration’s relationship to one duplex socket is unambiguous. **[clarity]**

52. **A Step 2: “fold this demonstration into step 3’s first commit.”** Make this an explicit alternative ordering with corresponding acceptance conditions. **[structure]**

53. **A Step 4: “Round-robin at frame granularity is live under any policy.”** Use the non-starvation condition consistently and reconcile optional later priority with the exposition’s assumed priority policy. **[clarity]**

54. **A introduction: “proposals for the owner’s ruling, not decisions.”** Mark which API choices remain open and which are covered by the recorded ruling. **[structure]**

55. **A Step 1: “The window prices the parked half.”** Split this long item into derivation, numerical example, implementation, and verification tasks. **[structure]**

56. **A Step 1: “About 3.4 MB in all.”** Identify the population parameter and whether the figure is per-party accounting or aggregate session memory. **[clarity]**

57. **A Step 1: “will report the real figure.”** Distinguish a sizing-model result from a measurement of actual allocated memory. **[clarity]**

58. **A Step 3: “A failed session keeps the halves.”** Describe ownership and non-return precisely, including the separate case where the caller supplied mutable references. **[clarity]**

59. **A Step 3 prose list: “`link.rs`.”** Explain the migration of its documentation rather than listing a deleted file as a later prose-edit target. **[structure]**

60. **A: “every intermediate tree is gate-clean.”** Specify which documentation changes belong in the atomic code commit and which can safely follow in the prose sweep. **[structure]**

61. **A Acceptance: “After step 3, `just gate` is clean.”** Explicitly include the corrected arrival-capacity invariant and identify the statistical memory qualification among the acceptance records. **[structure]**

62. **E §8: “the sender writes each reply the moment it is produced.”** Distinguish submitting a reply to the mux from its eventual transmission. **[clarity]**

63. **E §8: “the criterion’s proof that it can fail.”** Replace this convoluted phrase with a direct statement that the same fixture must fail without the cure. **[style]**

64. **E §8: “it was produced with heavy model assistance.”** Keep the formal result’s scope and status, but move provenance that does not bear on this argument out of the conclusion. **[concision]**

65. **R: “Rewritten whole after each of two fresh-eyes reading rounds.”** Remove revision-process history that does not help the intended reader. **[concision]**

66. **E §2: “hold them lightly until they are needed.”** State what the two properties will support instead of instructing the reader how firmly to remember them. **[style]**

67. **A: “re-denominates to the parking bound.”** Use a concrete verb such as “updates” or “rewrites” to describe the documentation task. **[style]**

68. **E §1: “a session takes the two halves of a byte stream and nothing else.”** Shorten the opening’s API detail so the initial claim emphasizes the protocol insight before introducing implementation consequences. **[concision]**

69. **E §2: “An empty listing means instead that I do not hold this child.”** Add one small worked exchange showing the changing asker/responder perspective, positional pairing, and next question level. **[clarity]**

70. **E §3: “simple and hand-verifiably correct.”** Remove the self-assessment and let the exchange trace establish the relevant simplicity. **[concision]**

**5. One-sentence assessment**

After one reading I could explain the bounded-parking idea and its three-term count to a colleague, but I could not present the single-socket result as proved without resolving decoder independence, the baseline progress argument, and the distinction between sufficient capacity and a capacity whose reduction actually deadlocks.