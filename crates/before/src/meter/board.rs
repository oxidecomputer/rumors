//! The amplification board: a red-green resource-proportionality matrix.
//!
//! Sweeps the crate's public operation surface against the adversarial input
//! families in [`meter`](crate::meter) (plus a benign pseudo-random control)
//! and prints one verdict per operation × family cell. The target contract
//! being scored: no operation materializes transient state asymptotically
//! larger than its packed operands, and every operation is amortized
//! `O(n + m)` in the packed input bits — with no bound on value magnitude,
//! tree depth, or encoded size.
//!
//! # The product over three axes
//!
//! The board is a generalized cartesian product over three declarative
//! axes, and coverage holds structurally rather than by per-cell wiring:
//!
//! - **Shapes** (the family list): each shape builds an *operand bundle* —
//!   a version, a disjoint party pair, a designated event × id cross, a
//!   fold population — and a uniform post-pass derives the rest (a
//!   cross shape's version is its event side; its id side becomes a
//!   party pair through the disjoint-mount adapter; every version gains
//!   its rank pair, and its ticked comparison counterpart wherever the
//!   shape did not build its own pairing). A shape reaches every
//!   operation its bundle supplies: adding a shape grows the product
//!   without naming any operation.
//! - **Operations** (the row table): each row declares the bundle slots
//!   its signature consumes and prepares its cell from them alone —
//!   never from the shape's identity — so adding an operation grows the
//!   product across every shape that supplies its operands.
//! - **Currencies** ([`ByCurrency`]): every judged quantity — the
//!   declarations, the readings, the scores — carries one field per
//!   metering currency, and the judgment iterates the axis itself.
//!   Adding a currency is a compile error at every declaration site
//!   until each operation answers its floor-or-NA question ([`Floors`]),
//!   so a meter can never be half-wired onto the board.
//!
//! The deterministic board always runs the whole product (the smoke test
//! pins the count); the wall-clock mirror selects with [`BenchMode`].
//!
//! # The criterion
//!
//! Each cell runs its operation at two input scales (the second twice the
//! first) and reads five deterministic meters over the body alone:
//!
//! - **peak heap bytes**, from a caller-installed counting allocator
//!   (see [`HeapMeter`]);
//! - **grown stack segments** ([`stack_segments`](crate::meter::stack_segments)),
//!   the honest stand-in for recursion-driven stack cost, which bypasses any
//!   heap meter;
//! - **big-integer limb operations**
//!   ([`limb_ops`](crate::meter::limb_ops)), only when the `limb-meter`
//!   feature compiles the counter into the arithmetic; arithmetic-width
//!   blowups are invisible to the other two meters;
//! - **packed-stream scan bits**
//!   ([`scan_bits`](crate::meter::scan_bits)), only when the `scan-meter`
//!   feature compiles the counter into the stream primitives; traversal
//!   work over the packed forms — the id-side walks and folds above all —
//!   allocates nothing, recurses nothing, and does no `Base` arithmetic, so
//!   this is the one column that sees it;
//! - **accumulator digit touches**
//!   ([`suanpan::touch_meter`]), only when
//!   the `limb-meter` feature compiles the counter into the accumulator:
//!   digit-state cost is work done *wider*, not more often — a walk that
//!   re-reads a wide running value per step allocates nothing extra,
//!   recurses nothing, does O(1) `Base` ops, and scans no extra bits, so
//!   this is the one deterministic column that sees the genre (the tick
//!   walk's width-circulation finding lived entirely in this currency).
//!
//! Per meter the board derives a **scaling exponent**
//! `log(m₂/m₁) / log(n₂/n₁)` (`n` = the cell's denominator bytes, below —
//! every exponent, on every column) and a **per-denominator-byte constant**
//! at the larger scale (the one constant not per denominator byte: the text
//! rows' limb constant is per `R` unit, below). A cell is
//! **GREEN** iff every meter's exponent is at most [`MAX_SCALING_EXPONENT`],
//! every constant is under its pinned ceiling
//! ([`MAX_HEAP_BYTES_PER_INPUT_BYTE`] over [`HEAP_FLAT_ALLOWANCE_BYTES`],
//! [`MAX_GROWN_STACK_SEGMENTS`], [`MAX_LIMB_OPS_PER_INPUT_BYTE`],
//! [`MAX_SCAN_BITS_PER_INPUT_BYTE`], [`MAX_TOUCHES_PER_INPUT_BYTE`] — or,
//! on the text rows' limb column, [`MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT`]), and
//! every committed liveness floor is met; **RED** otherwise, with the
//! offending meters named.
//!
//! # Liveness floors
//!
//! A ceiling over a counter proves the *instrumented* work is small, not
//! that the operation is cheap: four of the five judged columns are sensors
//! inside the implementation, and an implementation change can re-route work
//! around them, leaving the ceiling green over a counter that reads nothing.
//! Every cell therefore carries, per judged column, a [`Liveness`]
//! declaration the type demands at construction: either a **floor** — the
//! least the counter must read if the meter is watching the work, derived
//! from what the operation must do, never from how it does it — or an
//! explicit **not-applicable** with the reason no floor can bind. Floors
//! bind in the same pass the ceilings do, at both scales; a counter reading
//! below its floor is red, named as the column's floor with the vacuity
//! mechanism (the meter is not watching that work). The declarations render
//! per cell (`flr[...]`) and their derivations print as a legend above the
//! matrix. The conventions:
//!
//! - **Scan** is the universal leg: an operation that must examine its
//!   packed operands scans at least
//!   [`SCAN_FLOOR_BITS_PER_INPUT_BYTE`] bit per packed byte (an eighth of
//!   the stored bits); operations that may legitimately exit at the first
//!   divergence still read the root codes, floored at
//!   [`SCAN_TOUCH_FLOOR_BITS`]. Which of the two binds is derived per
//!   cell from the operands wherever the contract admits an early exit:
//!   the comparison rows floor at the root codes exactly when their pair
//!   is concurrent (a comparable pair must certify dominance over every
//!   region, so it keeps the full floor).
//!   Not-applicable is reserved for operations
//!   whose contract is a wholesale byte move or compare (encode, hash,
//!   same-form equality) or whose operands have no packed stream at all
//!   (the rank pair).
//! - **Limb** floors bind where big-integer arithmetic is semantically
//!   mandatory, at two derivations. The rows that read the stored form
//!   as-is (decode, the rank/distance/lag folds, and the tick walk)
//!   floor at the *stream's own codes*: one limb per 64 bits of every
//!   stored payload code wider than [`MACHINE_WORD_MAGNITUDE_BITS`] — a
//!   plateau of equal wide leaves stores its width once and steps by
//!   unit deltas after, and a conforming walk provably need not
//!   materialize each leaf's absolute value, so a tree-derived floor
//!   would demand limb work no conforming walk does. The value-
//!   materializing parse rows (`FromStr` must convert every spelled
//!   value) floor at the *decoded tree's* stored bases: one limb per
//!   64 bits of every base wider than the bound. Narrow cells are
//!   not-applicable (machine words suffice), as are operations whose
//!   contract forces no arithmetic at all.
//! - **Touch** floors are deterministic-liveness declarations, like the
//!   fork rows' heap floor, at two derivations. The delta-folding kernels
//!   (the comparison sweep, the merge emitters, the query rank folds, the
//!   tick walk, the text parse) land every stored delta in the running
//!   accumulator, at least one digit touch per stored delta code — the
//!   same one-per-delta floor the envelope suite's flatness pins
//!   commit. The validator batches word-scale deltas in the accumulator's
//!   lazy zone, so the decode rows floor only what it must fold digit by
//!   digit: one touch per 64 bits of every stored code wider than the
//!   machine-word bound (the stream-derived
//!   convention the tick rows' limb floor uses). Either floor is what a
//!   representation change trips deliberately: height or difference state
//!   moving off the metered accumulator into an unmetered big integer is
//!   exactly the migration this column exists to catch, so the trip is the
//!   designed stop-and-look, and an honest re-representation lowers the
//!   floor in a diff that shows the new derivation. Not-applicable genres:
//!   id-only walks (no magnitudes, no digit state), wholesale byte moves
//!   and hashes, plain big-integer arithmetic over decoded values (the
//!   rank pair), the renderer's delta-sized summaries, minimum folds and
//!   projections (word-scale bookkeeping and verbatim splices force no
//!   fold), comparisons over concurrent operands (one witness divergence
//!   per direction decides, so no fold count is forced), and operands
//!   whose streams store no delta codes.
//! - **Heap** floors bind on the codec and text rows, whose results must
//!   materialize at least their packed bytes; everywhere else allocation is
//!   not semantically forced (and the heap meter reads the process
//!   allocator, which no re-routing inside the crate can bypass).
//! - **Segments** is ceiling-only by policy: the target is walks that never
//!   grow the stack, so its honest floor is zero and a zero floor asserts
//!   nothing.
//!
//! A floor trip is a designed stop-and-look: an implementation that
//! legitimately does less work lowers the floor deliberately, in a change
//! whose diff shows the new derivation.
//!
//! Floors detect *total* vacuity, not partial rerouting: the scan floor is
//! deliberately an eighth of an honest walk's reading, so an implementation
//! that routes exactly the floor through metered primitives and the rest
//! around them still reads green. That is the derivation rule's designed
//! limit — a floor states what the operation *must* do, and partial
//! rerouting still does it — so the floors are a bypass tripwire, never a
//! full-liveness proof; the leg that bounds work no counter sees is the
//! time exponent judged over the bench suite (below).
//!
//! The rejection rows floor scan alone: their committed shapes place the
//! defect at the stream's end, and a self-delimiting stream's terminal
//! defect (or an overlap at both operands' preorder ends, under a coding
//! with no random access) is only discoverable by parsing to it, while
//! heap, limb, and touch are honestly not-applicable — rejection
//! materializes no result and forces neither value work nor an
//! accumulator fold. The text-rejection rows declare no floor on any
//! column, by the same honest derivation: no deterministic counter
//! watches text-byte consumption, and a parser may find the defect in
//! tokenization before any packed or value work — their ceilings judge
//! live readings (the shipped parsers do metered work greedily) and the
//! bench mirror times them like every row.
//!
//! Four cells are watched by neither leg, an exposure accepted here so it
//! is stated rather than silent: `version_hash`, `party_hash`,
//! `clock_hash`, and `version_eq` on the benign family. Hashing folds the
//! stored canonical bytes wholesale, and same-form equality compares them
//! wholesale, below every metered primitive — no stream walk, no forced
//! arithmetic, no forced allocation — so every floor column is honestly
//! not-applicable, and the benign operands are small enough (a few hundred
//! packed bytes across both scales) that the body never reaches the bench
//! judge's 10 µs judgment floor. The exposure is bounded by exactly those
//! two facts: sub-10 µs of word arithmetic per call over a
//! few-hundred-byte operand, with the same rows under the time leg on
//! every larger family. `version_eq`'s exposure differs from the hash
//! rows' in one respect its NA reason states on the board face: eq
//! operands grow without bound, so the time leg — under its own sub-floor
//! discipline — is the one backstop that the compare stays linear.
//!
//! # Determinism and the time leg
//!
//! Every quantity the board judges or renders is a deterministic counter,
//! so two board runs at the same scale are byte-identical under any
//! machine load: the board reads no clock, conditions nothing on timing,
//! and comparing runs needs no exclusion rules. The claim is enforced,
//! not assumed, on two legs: the runner itself measures every cell twice
//! in process and panics on any counter or denominator disagreement
//! ([`run`]'s self-verification), and the `amp-board-determinism` recipe
//! byte-compares two whole renders across processes. Time still has its own
//! judged leg — wall time is the one implementation-agnostic witness for
//! *time*, exactly as heap is for space: a kernel doing quadratic work in
//! plain machine-word arithmetic (no allocation, no recursion, no metered
//! reads) is invisible to all four counters and visible only to a clock —
//! but the clock lives where timing discipline does. The bench judge
//! (`tools/benchjudge`, `just bench-judge`) fits each cell's time exponent
//! across two scales of the board benches' criterion medians (warmup,
//! sampling, and outlier rejection are criterion's), denominated against
//! the same per-cell bytes as the board's own exponents
//! ([`BenchCell::denominator_bytes`]), and holds every judged cell to a
//! ceiling generous to scheduler noise and impassable for a quadratic's
//! ~2.0.
//!
//! # Acceptance scales and the profile of record
//!
//! Every cell runs at a size scale; the inner loop uses the default
//! (scale 1, seconds of runtime). Acceptance is [`RECORD_SCALE`]'s rule —
//! that constant owns the ×4 calibration argument and the both-scales
//! requirement — plus the bench judge green across the same two scales;
//! the enforced per-operation record remains the process-isolated
//! envelope suite in `tests/meter.rs` throughout.
//!
//! Readings of record come from the **release profile** (the `amp-board*`
//! recipes): debug assertions perform metered work — `Base` comparisons
//! through the limb shim, probe cursors that record scan bits — so a dev
//! board measures the algorithm plus its verification scaffolding, while
//! release measures the production work alone, the honest denominator. A
//! dev run remains a legitimate debugging view (the assertions themselves
//! are live there); its readings must never be pinned *on this board*.
//! (The envelope suite pins dev-profile numbers deliberately — the
//! stricter reading for its scenarios; `tests/meter.rs` argues the
//! choice. The two profiles of record differ because the two instruments
//! ask different questions.)
//!
//! # Denomination
//!
//! Most cells charge cost against **packed input bytes** alone. Two cell
//! classes have *mandatory* output asymptotically larger than any constant
//! times their input, so an input-only ceiling on them is unsatisfiable by
//! construction — and an unsatisfiable criterion degenerates into exemption
//! holes. Those cells are denominated against **total I/O bytes** `n_io`,
//! with the output side read back from the operation's actual result, never
//! assumed from its inputs:
//!
//! - **Text rows** (`Display` and `FromStr` for all three types): `n_io` is
//!   packed input + text output (`Display`) or text input + packed output
//!   (`FromStr`). Their heap and segment columns are judged per `n_io` byte
//!   at the unchanged ceilings. Their **limb column** is judged on two legs
//!   that exclude different converters. The *exponent* leg — against `n_io`,
//!   like every exponent — is what excludes quadratic conversion: any
//!   schoolbook converter's limb work is `Θ(digits × limbs)`, quadratic in
//!   the value bits however wide its chunks, and reads ~2 there \[measured\].
//!   The *constant* leg — against the radix-work denominator
//!   `R = n_io + Σᵢ (digitsᵢ × limbsᵢ +
//!   TEXT_PIPELINE_LIMB_OPS_PER_VALUE)` over the event values the text
//!   spells (the honest text cost law: schoolbook conversion plus the
//!   delta⇄absolute pipeline's measured per-value arithmetic), at the
//!   ceiling [`MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT`] — is what excludes a
//!   wasteful constant: a digit-by-digit schoolbook probe scores ~1 limb
//!   per `R` unit, over κ, while the honest kernels' worst family reads
//!   0.59 \[measured — the test suite's tripwires\]. Only a converter
//!   whose recorded limb work is near-linear in `n_io` with a
//!   pipeline-class constant reads green.
//! - **Flat-denominator exponents** (the comb-scatter shape): the shape
//!   deliberately scales tooth *count* at a fixed 1000-bit tooth
//!   magnitude, so its packed bytes are intercept-dominated — the one
//!   wide leading code plus unit delta codes per tooth — and grow only
//!   ~x1.2 while every slot's value content (and every operation's honest
//!   per-tooth work) doubles per level. A two-point power-law fit against
//!   an intercept-dominated denominator manufactures exponents out of
//!   exactly linear marginal work (log 2 / log 1.2 = 4), so the shape's
//!   input-denominated cells fit their *exponents* against the bundle's
//!   value content (the event side's summed leaf-height bits plus the id
//!   side's packed bytes — the honest scaling axis),
//!   disclosed per row as `expd[content ...]`. Constants and floors stay
//!   per packed byte, the harder reading; I/O-denominated cells keep
//!   `n_io`, whose output side already scales. The tripwire pair below
//!   pins both directions: the packed fit reads a manufactured exponent
//!   on measured flat per-tooth work, and a genuinely quadratic-in-teeth
//!   probe still reads red against the content denominator.
//! - **Output-dominated projection** (`own_version_to_version` and
//!   `clock_own_version_to_version` on the comb × scattered-party cross and the
//!   plateau-comb crosses — reveal-comb, reveal-hifloor, pure-comb):
//!   `n_io` is packed input + packed output. These crosses exist because
//!   the id keeps a wide magnitude per owned site — the scattered party a
//!   wide magnitude per kept tooth (`Θ(e·k)` mandatory output bits), the
//!   plateau ids a re-materialized `2^b`-scale code per kept site
//!   (`Θ(k·b)` output on a `Θ(k + b)` input) — and a packed output cannot
//!   be padded, so `n_io` is the honest denominator on all columns at the
//!   unchanged ceilings, with the projection sweep measured
//!   O(`n_io`)-tight on every one (exponents ≈ 1.0 against `n_io`, scan
//!   at the walk's usual 8 bits per `n_io` byte).
//!
//! The **output-honesty assertion** closes the pad-the-output door on the
//! text side: any text stream entering a denominator must satisfy
//! `text_bytes ≤` [`TEXT_BYTES_PER_RADIX_UNIT`] `× radix units` of the
//! values it spells, checked against the actual bytes.
//!
//! **Do not re-denominate** (these stay input-denominated): both binary
//! codec directions (the coding is canonical 1:1, so input bytes are the
//! honest bound); every scalar, comparison, and query row (word-sized or
//! borrowed results); and the packed-output mutator rows (`join`, `meet`,
//! `tick`, `fork`, `recv`, `sync`, `without`, and every
//! projection cell outside the output-domination cross) — their input denomination rests on output
//! coding ≤ inputs + O(1) per overlay boundary, which is pinned for
//! join/meet as the 1-Lipschitz proptest in
//! [`tier2`](crate::meter::tier2)'s test suite rather than assumed.
//!
//! **Rank operands** (`rank_pair_ops`, `rank_sum`) have no packed encoding
//! to charge against; their denominator of record is the operands' **value
//! content** `bits(num) + exp` in bytes. That content is wire-bounded:
//! every public construction path (the `rank`/`distance`/`lag` folds)
//! emits a rank whose numerator width and exponent are each linear in the
//! packed bits the fold read, so a ceiling per content byte is a ceiling
//! per wire byte up to the fold's own constant.
//!
//! # Declared per-cell models
//!
//! Two cell classes are judged against a **declared model** — a ratified
//! cost law derived at the cell — in place of one flat ceiling, because
//! the flat form is unsatisfiable on work their contracts mandate (the
//! same reasoning that re-denominates the I/O cells). Each is disclosed
//! on its row face (`decl[...]`), derived at its constant's definition
//! site (the declared-models section of the ceilings block), banded on
//! both sides so an improved kernel forces a deliberate re-declaration,
//! and tripwired in the test suite by a wrong artifact reading red:
//!
//! - **The fold rows** (`version_join_all`, `party_join_all`): the
//!   balanced reduction's documented `O(D log k)` puts a `log2(2k)`
//!   factor in the deterministic counters that no flat ceiling admits at
//!   scale. The limb/scan/touch exponent ceilings become the model's own
//!   predicted exponent plus the linear cells' slack, and the scan
//!   constant [`FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL`] per reduction
//!   level; a quadratic left fold still reads ~2 and stays red, and the
//!   log factor's own liveness is the claims suite's
//!   `fold_log_factor_is_alive` pin.
//! - **The comb-scatter projection pair** (`own_version_to_version`,
//!   `clock_own_version_to_version` on the output-domination cross): peak
//!   heap is the output builder's doubling chain anchored at the
//!   operand-size reserve — `capacity_chain_peak`'s
//!   `3·(n+m)·2^(k−1)`, ratified within 2% at every probed point. The
//!   heap reading is banded around the model at both scales
//!   ([`CAPACITY_MODEL_FLOOR`], [`CAPACITY_MODEL_CEILING`]) and the heap
//!   exponent fit is retired as unjudgeable there — the chain quantizes
//!   peak by powers of two, so a probe pair straddling a `k` step
//!   manufactures an exponent out of exactly the profile the model
//!   prices.
//!
//! # The rejection surface
//!
//! Cost claims are total: rejecting an input is an outcome with a cost,
//! bounded like any other, whether or not the caller honored the usage
//! invariants. The rejection rows price the fallible surface — overlap
//! (`*_join_overlap`, `clock_sync_overlap`, `party_join_all_overlap`),
//! the empty difference (`party_without_none`), strict decode
//! (`*_decode_truncated`/`_trailing`/`_noncanon`), and text parse
//! (`*_parse_trailing`/`_noncanon`, driving `FromStr`) — with the defect
//! **maximally deferred** in every shape: an early-exit-only measurement
//! would be the cheapest artifact that passes, so each row places its
//! defect where rejection must consume as much input as possible (the
//! last byte truncated, trailing bits after the complete stream, a
//! non-canonical pair closing at the stream's last position, the one
//! overlapping region at both operands' preorder ends, junk after the
//! whole valid text). Rejections produce no output, so every rejection
//! row is denominated against the fed stream alone — packed bytes, or
//! text bytes on the parse rows at the general (not κ) limb ceiling: the
//! radix-work term prices conversion of the accepting direction, and a
//! rejection forces no conversion. Overlap operands come from the
//! overlap-mount adapter, the disjoint-mount adapter's counterpart; its
//! outputs are semantically void by design (see the adapter's own docs).
//! The coverage list below is the durable record of which fallible
//! operations are rowed and which carry a bounded-or-delegated reason.
//!
//! # Reading the numbers
//!
//! The board runs every cell in one process and resets the peak-heap counter
//! between cells, so a cell's heap number can include allocator noise from
//! the harness itself: the board's numbers are *indicative*. The enforced
//! record is the meter test binary (`tests/meter.rs`), whose scenarios run
//! one per process under nextest and pin exact envelopes. Zero-measurement
//! cells score exponent 0; a meter that moves from 0 to a nonzero count is
//! clamped through `max(m, 1)` before the ratio, so the exponent stays
//! finite.
//!
//! # Families
//!
//! Every shape from [`meter`](crate::meter) reaches every operation its
//! operand bundle supplies (the product section above): the event shapes —
//! the dense spine, `bigroot`, `hugeleaf`, the boundary comb (`cliff`, at
//! `k = n` so its value content grows quadratically in its packed input),
//! `harmonic`, `freeze-pos` (the many-freezes spine, one query-fold
//! freeze per block), and `promo-rearm` (the many-armings spine, one
//! query-fold promotion per block) — carry a version; the diverted id-spine pair carries a
//! disjoint party pair; the eleven cross shapes (`comb-scatter` and the
//! ten tick-walk crosses) carry a version, a mounted party pair, and a
//! clock; the two version-pair shapes — `jump-pair` (wide
//! height-difference crests over a dense-position spine) and
//! `concurrent-pair` (the switch-density population) — carry a version
//! pair of their own construction, so
//! their comparison rows run the pairing the shape was built around
//! rather than the ticked counterpart; the two fold populations —
//! `scatter` and `weave` — carry fold operands alone, so exactly the two
//! fold rows run on them; `benign` — a fixed-seed pseudo-random population of forked,
//! ticked clocks, the control row that keeps the ceilings honest on
//! organic inputs — carries everything. Where an operation needs a
//! `Party` and a `Version`, the board crosses adversarial party × small
//! version, small party × adversarial version, and — on the cross
//! shapes — the designated adversarial × adversarial pairing.
//!
//! This list is deliberately narrower than the generator surface: a shape
//! earns a board column only as a whole-surface adversary, while
//! kernel-seam probes live in the envelope suite alone. The criterion and
//! the add-a-shape touch list sit on the `FAMILIES` roster below.
//!
//! Six shapes carry a genre note beyond their variant docs:
//!
//! - `freeze-pos`, built against the linear-functional rows: `Θ(s)`
//!   query-fold freezes at ever-deeper stream positions where every
//!   comb fires O(1) — the coverage the #37 review's freeze-position
//!   finding named. The committed known-bad kernel (the query fold's
//!   adequacy tripwire) reads ×1.50 per byte across this family's
//!   doubling, so a green `version_rank × freeze-pos` cell is a live
//!   verdict, not decoration.
//!
//! - `promo-rearm`, built against the linear-functional rows: `Θ(s)`
//!   query-fold promotions at O(1) stored codes each, over a consumed
//!   mass whose written span the spine keeps growing — the coverage
//!   hole freeze-pos left, its parked drift being monotone (no
//!   committed family promoted at all). The committed known-bad kernel
//!   (the query fold's span-promotion tripwire) reads ×1.74 per byte
//!   across this family's doubling, so a green
//!   `version_rank × promo-rearm` cell is a live verdict, not
//!   decoration — and the class-binding seal that holds `Linear`
//!   claims against exponent-mechanism reds is live for the promotion
//!   mechanism exactly because this column exists.
//!
//! - `comb-scatter`: the projection cross (boundary-comb version ×
//!   scattered party) whose mandatory output dominates its input — the
//!   case the small-operand crosses cannot exhibit; its two projection
//!   cells are the board's only I/O-denominated non-text cells.
//! - `harmonic` (`meter::harmonic`, a 1-leaf at every depth), built
//!   against the linear-functional rows (`rank`/`distance`/`lag`/
//!   `min_ticks`) and the rank rows (`rank_pair_ops`, `rank_sum`): its
//!   rank's numerator is as
//!   wide as the depth already walked at every level, so a fold that
//!   re-shifts its accumulated numerator per level reads limb exponent ~2
//!   here while `dense` (a one-bit numerator) stays the linear control.
//!   The query kernels' rank fold telescopes through height deltas, and
//!   `version_rank × harmonic` reads the control's linear signature
//!   \[measured — limb exponent 1.00, constant within 2% of `dense`, both
//!   scales\]: the column is the tripwire that goes red under the
//!   re-shifting genre.
//! - `scatter`, whose bundle carries fold operands alone, for the two
//!   fold rows (`version_join_all`,
//!   `party_join_all`; both also keep a `benign` control cell, folding the
//!   organic population in construction order): balanced-forked
//!   single-tick operands ordered evens before odds, so a sequential
//!   fold's accumulator holds every other leaf and never coalesces — the
//!   shape that reads exponent ~2 under a left fold. Both rows run the
//!   balanced binary-counter reduction (every input passes through
//!   O(log n) joins), and what the cells show is its log factor — on the
//!   version fold's limb and scan columns, and on the party fold's scan
//!   column alone (its walk allocates nothing, recurses nothing, and does
//!   no arithmetic, so scan is the only deterministic meter that sees
//!   it): exponents ~1.1 and constants that grow with scale, marginally
//!   over the amortized-linear bounds at some scales \[measured — both
//!   scales\]. The `benign` controls read the same
//!   signature as `scatter`, so the readings priced by the declared fold
//!   model are the reduction's own n·log n cost, not the adversarial
//!   ordering's.
//! - `weave`, the correlated fold population (the leaves of one balanced
//!   fork expansion dealt round-robin among 16 group parties,
//!   one tick each), also fold-rows-only: every operand pair is
//!   both-present at the whole shared upper skeleton while each operand
//!   alone is an organic region set, so the per-node fold costs that
//!   scale with the *other* operand — the overlap test against the
//!   accumulator above all — dominate at fixed arity. Scatter's
//!   single-leaf operands cannot reach the genre and benign reaches it
//!   only diluted.
//!
//! # Coverage: the not-applicable list
//!
//! Every public operation either has a board row or is listed here with the
//! reason it has no meaningful adversarial operand of its own:
//!
//! - **Delegations and aliases**: `Version::concurrent`
//!   is one `partial_cmp` (the `cmp` row measures the walk; `concurrent`
//!   still gets its own row since it is the documented entry point);
//!   the operator matrix (`|`, `&`, and their assign forms, over
//!   every borrow shape) routes through the same `join_view`/`meet_view`
//!   emitters and cmp walk the `join`/`meet`/`cmp` rows measure;
//!   `Clock::send` is `Clock::tick` by definition; `clock | version` and
//!   `clock |= version` fold through the same join-assign the `recv` row
//!   measures; `Party::tick` is the mirror of `Version::tick` (the
//!   `tick_adv_party` row); `Debug` for all three types delegates to
//!   `Display`.
//! - **Folds over measured operations**: `Version::join_all` has its own
//!   row (the `scatter` cell plus the `benign` control), and
//!   `Version::Sum`/`FromIterator` are that
//!   fold by definition; `Party::join_all` likewise (the party fold's
//!   cells); `Clock::join_all` is the party fold and the version
//!   fold run side by side, so the two fold rows price both of its
//!   halves. `Version::meet_all` stays NA by mechanism: meet only shrinks,
//!   so its running accumulator is bounded by the *smaller* operand at
//!   every step — the fold does at most `Σ min(|acc|, |vᵢ|)` work and
//!   cannot exhibit the growing-accumulator genre the join folds do.
//!   `Party::forks`/`Clock::forks` iterate the measured `fork`, each step
//!   on the freshly-split half (shrinking operands, same argument); a
//!   `Forks` iterator dropped mid-run rejoins its unclaimed remainder in
//!   one `join` per remaining level of the fork tree — O(log n) of the
//!   measured `join` on shrinking operands.
//! - **Bounded or trivial inputs**: `Version::new`/`Default`,
//!   `TryFrom<u64>`/tuple literals (word-sized literals),
//!   `Party::seed`/`is_seed`, `TryFrom<u8>`/`TryFrom<bool>`,
//!   `Clock::seed`/`TryFrom<(I, E)>`.
//! - **Moves, borrows, and byte copies**: `is_empty`, `as_bytes`,
//!   `encoded_bits`, `encode_to` (the `encode` row's path into a writer),
//!   `dangerously_alias` (a byte copy), `Clock::from_parts`/`into_parts`,
//!   `Clock::party`/`version`, the projection view constructors (`&v / &p`
//!   and `Clock::own_version` build a two-borrow `OwnVersion` in O(1); the
//!   view's materialization and fused comparisons have their own rows, and
//!   `From<OwnVersion> for Version` is `to_version` by definition),
//!   `Ranked::rank`/`version`/`into_parts` (borrows and moves); `Clone`
//!   (`Version`, `Rank`, `Ranked`) copies the stored bits or value content
//!   wholesale, with no walk or arithmetic in the contract; `Party`'s and
//!   `Clock`'s derived `PartialEq`/`Eq` are one bit-slice compare of the
//!   stored canonical bits (`Version`'s `==` is the same wholesale compare
//!   — the stored coding is canonical, so byte equality is causal equality
//!   and no walk is in the contract; it keeps the `version_eq` row because
//!   its operands grow without bound, the row's time leg holding the
//!   compare linear); the consuming array splits
//!   (`From<Party> for [Party; N]`, `From<Clock> for [Clock; N]`) are the
//!   `forks` machinery above plus `N` moves.
//! - **Derived pairings**: `Ranked::from` is the `rank` row plus a move; its
//!   comparisons are `Rank` comparisons plus byte equality; `Rank::cmp`,
//!   `checked_sub`, and `+` have their own row (`rank_pair_ops`, on the
//!   mismatched-exponent pair, value-content-denominated per the
//!   Denomination section); `Rank::AddAssign` is `+` in place, and the
//!   `Sum` fold has its own row (`rank_sum`, the mixed high-first
//!   population, denominated the same way); `Rank`'s `Display` (its `Debug` delegates)
//!   prints the `rank` row's output — a derived value with no packed
//!   encoding to normalize against, rendered by the same per-`Base` decimal
//!   print the `version_display` row drives.
//! - **The same comparisons under another name**: `causally`'s other
//!   constructors, `Range::placement_of`, and `Range`'s refinement methods
//!   (whose composition gate is one validating comparison) perform the
//!   identical causal comparisons the `causally_contains` row measures;
//!   `Range`'s bound accessors (including its `RangeBounds` view) are
//!   borrows.
//! - **Wrappers**: the `serde`/`borsh` impls serialize as the canonical
//!   encoding and deserialize through the strict decoder — the
//!   `encode`/`decode` rows.
//! - **Test support**: `oracle` and the `error`/`iter` modules' data types
//!   perform no computation over packed inputs; `meter`'s own surface —
//!   the generators, the counters, this board — is the measurement
//!   instrument itself, feature-gated out of production builds. The
//!   `skyline` kernel `meter` re-exports (and the `suanpan` accumulator
//!   under it) is the implementation
//!   under every public operation, public only so the envelope suite can
//!   pin its internals: every cell of this board already times it at
//!   the public boundary, its resources are pinned by the envelope
//!   scenarios in `tests/meter.rs`, and its agreement with the
//!   recursive oracle is pinned by its differential suites.
//! - **The rejection surface's bounded-or-delegated remainder** (the
//!   rejection rows above price the rest): `Clock::join_all`'s overlap
//!   hand-back runs the identical up-front indexed test against self
//!   that `party_join_all_overlap` prices, inline; clock non-canonicality
//!   — packed or text — is the component validators on the same streams
//!   the version and party non-canonical rows drive;
//!   [`Decode::Anonymous`](crate::error::Decode) is the accepting parse
//!   of the empty stream (a zero-byte operand, no scaling axis) and
//!   [`Parse::Anonymous`](crate::error::Parse) the one-token `"0"`;
//!   [`Decode::Io`](crate::error::Decode) is the caller's reader — a
//!   failing reader is a truncation carrying an error, priced by the
//!   truncated rows — and `encode_to`'s error the caller's writer, at
//!   most the encode row's work before it propagates; the `TryFrom`
//!   literal rejections have word-scale or type-bounded operands;
//!   `Version::meet_all`'s `None` is the empty iterator;
//!   `Rank::checked_sub`'s `None` is measured on the `rank_pair_ops`
//!   row, which attempts both directions; other decode non-canonicality
//!   genres (a negative running height, nonzero padding) ride the same
//!   single validator pass at the same full-parse cost as the committed
//!   maximally-deferred tails; serde/borsh deserialize errors are the
//!   strict decoder through the wrappers (the decode rejection rows).

mod currency;
#[cfg(test)]
mod tests;

pub use currency::{ByCurrency, Currency, Floors, Liveness};

use std::any::Any;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::rc::Rc;

use crate::codec::{self, Base};
use crate::error::{Decode, Parse};
use crate::{causally, Clock, Party, Rank, Version};

// ─── the pinned ceilings ────────────────────────────────────────────────────
//
// Several ceilings below argue their calibration from a measured worst
// honest reader and name it. Those witnesses are load-bearing prose, and
// nothing mechanical guards them: a new family (or kernel change) that
// reads heavier than a named witness falsifies the calibration argument
// while its cell still reads green, since every ceiling sits well above
// its witness. Whoever lands such a change re-measures and re-words the
// constants whose witness moved.

/// Green requires every meter's scaling exponent at or below this.
///
/// The contract is amortized-linear; 1.15 leaves room for measurement noise
/// (allocator rounding, `Vec` doubling) without admitting a real log factor
/// at these input sizes.
pub const MAX_SCALING_EXPONENT: f64 = 1.15;

/// Green requires peak transient heap at most this many bytes per packed
/// input byte, over the flat allowance.
pub const MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 16.0;

/// Heap bytes ignored before the per-byte constant is computed: fixed-size
/// scaffolding (format machinery, hasher state, container headers) that does
/// not scale with the input.
pub const HEAP_FLAT_ALLOWANCE_BYTES: usize = 8_192;

/// Green requires at most this many grown stack segments, as an absolute
/// count: the target is walks that never grow the stack, so the ceiling is
/// flat, not per-byte.
pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;

/// Green requires at most this many big-integer limb operations per packed
/// input byte (asserted only when the `limb-meter` feature is lit).
///
/// Calibrated against the benign control: an amortized-linear walk records a
/// handful of unit-limb operations per node (tens per packed byte at ~2 bits
/// per node, over a hundred for multi-walk operations like `distance`), and
/// that per-node arithmetic is exactly the contract's linear regime. The
/// ceiling sits above it; width blowups are caught by the exponent bound
/// long before the constant.
pub const MAX_LIMB_OPS_PER_INPUT_BYTE: f64 = 128.0;

/// Green requires at most this many packed-stream scan bits per denominator
/// byte (asserted only when the `scan-meter` feature is lit).
///
/// Calibrated against the benign control and the green adversarial
/// families: a single walk over packed operands scans ~8 bits per byte,
/// multi-walk operations (`distance` runs join, meet, and two rank folds;
/// `sync` joins in both directions) scan a small multiple, and the text
/// parsers re-scan their packed output through the strict validator. The
/// worst honest reader measured is well under 64; the ceiling sits at 96 so
/// only a walk that re-scans state growing with the input — the fold genre —
/// goes red on this column.
pub const MAX_SCAN_BITS_PER_INPUT_BYTE: f64 = 96.0;

/// Green requires at most this many accumulator digit touches per
/// denominator byte (asserted only when the `limb-meter` feature is lit).
///
/// Calibrated against the benign control and the green adversarial
/// families at the release profile of record: the delta-folding kernels
/// (validate, sweep, emit, the query folds, the text parse) touch a
/// handful of digits per delta code — single digits per packed byte on
/// organic shapes — and the heaviest honest reader measured is the
/// mirror-narrow tick cross (the memo machinery's per-site resolution) at
/// 30.8 touches per input byte at the default scale, 24.3 at the record
/// scale \[measured 2026-07-26, release, both scales\]. The ceiling sits
/// at 96, scan's own margin convention, so only a walk that re-reads
/// digit state growing with the input — the width-circulation genre —
/// goes red on this column's constant.
pub const MAX_TOUCHES_PER_INPUT_BYTE: f64 = 96.0;

/// Scan liveness floor: an operation that must examine its packed operands
/// scans at least this many bits per packed input byte.
///
/// One bit per byte is an eighth of the stored bits: far below any honest
/// full walk (measured ~8 bits per byte across the board), and far above a
/// counter that has stopped watching (which reads ~0).
pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;

/// Scan liveness floor for legitimate early-exit operations: even an
/// immediate divergence answer reads the operands' root codes.
pub const SCAN_TOUCH_FLOOR_BITS: u64 = 2;

/// Scan liveness floor for the tick-cross rows: all 8 bits of every
/// input byte.
///
/// The paired fill walk examines every topology bit and payload code of
/// both operands at least once; `WHY_SCAN_TICK_WALK` carries the
/// row-face wording.
const TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8;

/// Magnitudes at most this wide may legitimately be handled in machine
/// words; wider values force big-integer arithmetic, so limb floors bind
/// only on them.
pub const MACHINE_WORD_MAGNITUDE_BITS: u64 = 128;

/// The fixed count the `version_ticks` cell registers per measurement.
///
/// Fixed so the cell's judged axis is the packed input alone: the
/// count's whole contribution is the boundary codes' gamma width (the
/// flatness rows of `tests/meter.rs` pin that axis point to point), and
/// 512 sits far enough past the single tick that an implementation
/// iterating even a fraction of the count would blow the scaling
/// ceiling rather than hide in a constant.
pub const TICKS_BOARD_COUNT: u64 = 512;

/// Text rows only: green requires at most this many limb operations per
/// radix-work unit
/// `R = n_io + Σᵢ (digitsᵢ × limbsᵢ + TEXT_PIPELINE_LIMB_OPS_PER_VALUE)`
/// over the spelled event values (κ).
///
/// κ carries the text limb column's *constant* leg only; the *exponent* leg
/// is judged against `n_io`, never against `R` — `R` is the honest text
/// cost law itself (schoolbook conversion plus the per-value pipeline
/// term), so an exponent against it reads a flat ~1 on exactly the
/// quadratic converters the bound exists to catch. The legs exclude
/// different converters, and a constant ceiling cannot enforce a complexity
/// class: a `u32`-chunked schoolbook converter scores ~0.11 limb per `R`
/// unit — under κ, and wider chunks only lower it — while its limb work
/// stays quadratic in the value bits and reads exponent ~2 against `n_io`
/// \[measured — the chunked tripwire in the test suite\]; the exponent leg
/// is what excludes it. What κ excludes is a wasteful constant. It is
/// pinned from the production kernels' observed meter at record scale
/// (release, the profile of record): the honest cells read at most 0.59
/// limb per `R` unit (the staircase pipeline, both directions), so κ
/// leaves the worst honest family ~27% headroom while a digit-by-digit
/// schoolbook probe's measured ~1 limb per `R` unit still exceeds it, and
/// the production parser — radix conversion delegated to the backend's
/// divide-and-conquer parser, one width-proportional limb record per
/// materialized value — reads orders under it on the conversion-dominated
/// families \[measured — the schoolbook and delegating-parser pins in the
/// test suite\]. The test suite pins three legs — the schoolbook probe
/// exceeds κ; the delegating parser stays under κ over a liveness floor;
/// the chunked probe slips under κ and trips the exponent leg — so none
/// can silently soften.
pub const MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT: f64 = 0.75;

/// Radix units each spelled event value contributes to the text limb
/// denominator beyond its conversion term: the delta⇄absolute pipeline's
/// per-value arithmetic allowance.
///
/// The text kernels do mandatory per-value big-integer work that is not
/// radix conversion — the render derives each printed base from
/// delta-sized relative summaries, the parse re-derives delta codes from
/// spelled bases — and `Σ digits × limbs` under-weights it to nothing on
/// small-value trees (a one-digit value is one radix unit; the pipeline
/// around it is not free). The allowance is pinned just above the
/// production kernels' measured honest range \[measured, release, record
/// scale: 5–9 limb ops per spelled value across the small-value families,
/// both directions; the ceiling κ then leaves the worst family ~27%
/// headroom\]. Id tokens contribute nothing: an id tree spells booleans
/// and forces no arithmetic.
pub const TEXT_PIPELINE_LIMB_OPS_PER_VALUE: u64 = 10;

/// Any text stream entering a denominator must hold at most this many bytes
/// per radix unit of the values it spells (the output-honesty ceiling).
///
/// Denominating against I/O bytes opens a door: pad the output, inflate the
/// denominator, read green. The ceiling closes it \[derived\], and its
/// basis is the radix-unit sum (`Σ digits × limbs`, computed from the
/// values outside the render) rather than wire bits, because the skyline
/// wire coding spends O(1) bits on a leaf whose spelled value is wide —
/// no constant per wire bit bounds honest text. Per rendered value the
/// grammar spends its exact decimal digits (`digits ≤ digits × limbs`, one
/// radix unit minimum per value) plus at most 6 syntax bytes (`(`, `)`,
/// and two `, ` separators), and every value contributes at least one
/// radix unit — so honest text stays under 7 bytes per radix unit and
/// padding trips the assertion.
pub const TEXT_BYTES_PER_RADIX_UNIT: f64 = 8.0;

/// Exponent legs are fitted only where the denominator pair grows at
/// least this much between the cell's two probes.
///
/// The fit divides by `log(denominator growth)`: the families' probe
/// pairs double their scaled dimension by construction, so a pair growing
/// less than this says the operand does not scale with the knob at all
/// (the benign rank pair moves 6 -> 7 bytes) and the division manufactures
/// exponents out of word-scale reading noise (log 2 / log 7/6 amplifies
/// x4.5). An unjudged exponent renders `-.--` and the cell rides its
/// constants and floors, which bound single-size cost regardless
/// \[derived; the sub-scaling tripwire in the test suite pins both
/// directions\].
pub const MIN_EXPONENT_DENOM_GROWTH: f64 = 1.5;

// ─── declared per-cell models ────────────────────────────────────────────────
//
// Two cell classes carry a *declared model* in place of one flat ceiling:
// a ratified cost law, derived and priced at the cell, that the readings
// must match — the flat ceiling would otherwise be unsatisfiable by
// construction on work the operation's own contract mandates. A declared
// model is disclosed on the row face (`decl[...]`), replaces only the
// legs it names, and is banded on both sides: a reading over the model is
// the regression the ceiling exists to catch, and a reading under its
// floor means the model has gone stale against an improved kernel and
// must be re-declared in a diff that shows the new derivation (the same
// ratchet as a liveness floor).

/// The fold rows' declared scan model: at most this many scan bits per
/// input byte per balanced-reduction level, `log2(2k)` levels over `k`
/// fold operands.
///
/// The fold rows run the balanced binary-counter reduction, whose
/// documented class is `O(D log k)` (`FoldLog` in the claims roster):
/// every input passes through `O(log k)` joins, and each join level
/// re-scans the operands it merges, so scan work per input byte grows by
/// a constant per level — never flat, at any implementation of the
/// balanced reduction. Derivation of the constant: the fold cells read
/// 9.1–10.0 scan bits per byte per level across both scales and both
/// committed populations \[measured 2026-07-28, release: the benign
/// control at k = 512 reads 100.1 bits/B over 10 levels, at k = 2048
/// reads 116.9 over 12\]; 12 leaves the worst honest reading ~20%
/// headroom while a fold whose per-level constant regresses by a third
/// still reads red.
pub const FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL: f64 = 12.0;

/// The scan bits one metered `IdIndex` table probe records: one `u32`
/// table word per probe.
///
/// The party fold's declared search allowance is `32·⌈log2(t+1)⌉`
/// probes' worth per both-present node of each tested input, `t` the
/// accumulator's table size.
///
/// Derivation: `Party::join_all` overlap-tests every input against the
/// fixed accumulator through a per-call table of the accumulator's
/// both-present nodes, and each both-present node the test visits runs
/// one binary search over at most `t` entries — at most `⌈log2(t+1)⌉`
/// probes of one table word each. The allowance is that bound summed
/// over the inputs' both-present nodes, computed from the operands at
/// prepare; it is tight-ish where the searches dominate \[measured
/// 2026-07-28, release, the weave family: readings sit within ~10% of
/// the fold model plus this allowance at both scales\], zero on
/// populations with no both-present structure (scatter's single-leaf
/// operands), and absent from the version fold, which runs no overlap
/// test. The decision to keep the index and price its searches — over
/// reverting to the per-input cursor walk the #37 review's F4 weighed —
/// is the design doc's dated F4 entry: the committed overlap
/// instruments pin the index's asymptotic win (a cursor discipline
/// reads quadratic on the overlap rows and trips the flatness pin),
/// and the index ties or wins wall time on every committed fold
/// population.
pub const INDEX_PROBE_SCAN_BITS: u64 = 32;

/// A packed id operand's both-present node count: the size of the
/// `IdIndex` table a fold builds over it, and the per-input factor of
/// the declared search allowance. One 2-bit presence tag per node.
fn both_present_nodes(p: &Party) -> u64 {
    let bits = p.as_bits();
    let mut count = 0u64;
    let mut i = 0;
    while i + 1 < bits.len() {
        count += u64::from(bits[i] && bits[i + 1]);
        i += 2;
    }
    count
}

/// The declared-model band: a modeled reading must sit within
/// `[CAPACITY_MODEL_FLOOR, CAPACITY_MODEL_CEILING] × model`.
///
/// The capacity-chain model fits the committed probe points within 2%
/// \[measured 2026-07-28, release: measured/model 1.005–1.017 across
/// teeth 128–1024\], so ±10% absorbs the walk's small non-chain
/// allocations while a regressed builder — an unanchored doubling chain,
/// an extra buffer copy — overshoots the ceiling, and an improved
/// builder undershoots the floor and forces a deliberate re-declaration.
pub const CAPACITY_MODEL_CEILING: f64 = 1.10;
/// The declared-model band's lower edge; see [`CAPACITY_MODEL_CEILING`].
pub const CAPACITY_MODEL_FLOOR: f64 = 0.90;

/// The ratified capacity-chain peak-heap model for the output-dominated
/// projection's builder: `3·(n+m)·2^(k−1)` bytes.
///
/// `k = ⌈log2(output/(n+m))⌉`, clamped to at least 1 — the committed
/// shapes sit at output ≥ 32× input, so the clamp never binds and
/// exists only to keep the formula total.
///
/// Derivation: the projection's output is not size-derivable from its
/// operands (mandatory `Θ(|v|·|p|)` output on `Θ(|v|+|p|)` input), so no
/// reserve-once bound exists; the output builder anchors its buffer at
/// the operand-size reserve `n+m` and doubles `k` times to reach the
/// output, and peak heap is the last realloc's old+new coexistence:
/// `(n+m)·2^(k−1) + (n+m)·2^k = 3·(n+m)·2^(k−1)`. The board's exponent
/// fit is honestly unjudgeable across this chain — a probe pair
/// straddling a `k` step manufactures an exponent out of the quantized
/// capacity, one inside a step reads sublinear — which is exactly why
/// these cells are judged against the model instead (owner ratification
/// 2026-07-27: the doubling-chain band is the accepted stated-band
/// residual — no pre-walk, no segmented output).
fn capacity_chain_peak(input_bytes: usize, output_bytes: usize) -> f64 {
    let anchor = input_bytes as f64;
    let k = (output_bytes as f64 / anchor).log2().ceil().max(1.0);
    3.0 * anchor * (k - 1.0).exp2()
}

/// The fold rows' declared exponent ceiling over the fold currencies
/// (limb, scan, touch): the `FoldLog` model's own predicted exponent plus
/// the global noise slack.
///
/// Work `c·D·log2(2k)` fitted across the cell's two probes
/// (`D₁, k₁) → (D₂, k₂`) reads exponent
/// `1 + log2(log2(2k₂)/log2(2k₁)) / log2(D₂/D₁)` — the log factor's
/// marginal, ~1.14–1.17 at the committed populations — so the ceiling is
/// that prediction plus the same slack [`MAX_SCALING_EXPONENT`] grants
/// linear cells (0.15). A quadratic fold reads ~2 against any committed
/// arity pair and stays red; the model's own liveness is the
/// `fold_log_factor_is_alive` pin, which reads red the day the reduction
/// stops paying its log factor.
fn fold_exponent_ceiling(k1: u64, k2: u64, n1: usize, n2: usize) -> f64 {
    let levels1 = (2.0 * k1 as f64).log2();
    let levels2 = (2.0 * k2 as f64).log2();
    let denom_growth = (n2 as f64 / n1 as f64).log2();
    1.0 + (levels2 / levels1).log2() / denom_growth + (MAX_SCALING_EXPONENT - 1.0)
}

/// The acceptance scale of record: the size multiplier of the record-mode
/// board run (`just amp-board-record`).
///
/// The default-scale board under-detects segment amplifiers: stacker grows
/// a segment only past ~1 MiB of frames, so a recursion-frame amplifier
/// whose onset sits above the default depths reads a false green there.
/// ×4 is the witnessed calibration floor — the scale at which every known
/// segment-onset amplifier read red under pre-fix code — so acceptance runs
/// pin it. **Campaign acceptance is all cells green at BOTH the default
/// scale and this one, one run each under the determinism tripwire**; a
/// record run is
/// acceptance-time only (the inner loop stays at the default scale, and the
/// enforced per-operation record remains the envelope suite in
/// `tests/meter.rs` regardless of board onset).
pub const RECORD_SCALE: f64 = 4.0;

// ─── family sizes at scale 1.0 ──────────────────────────────────────────────

/// Dense event spine depth at scale 1.0 (packed size ~4 KiB).
const DENSE_BASE_DEPTH: usize = 8_000;

/// Bigroot root magnitude in bits at scale 1.0.
const BIGROOT_BASE_MAGNITUDE_BITS: usize = 8_000;

/// Bigroot spine depth at scale 1.0 (packed size ~3 KiB with the magnitude).
const BIGROOT_BASE_DEPTH: usize = 2_000;

/// Hugeleaf magnitude in bits at scale 1.0 (packed size ~4 KiB).
const HUGELEAF_BASE_MAGNITUDE_BITS: usize = 16_000;

/// Id spine depth at scale 1.0 (packed pair ~6 KiB).
const ID_BASE_DEPTH: usize = 12_000;

/// Boundary-comb tooth magnitude (bits) and tooth count at scale 1.0
/// (packed size ~4 KiB); one parameter drives both, mirroring the meter
/// suite's `k = n` convention.
///
/// Scaling `k` with `n` is the separating choice: it keeps the comb's
/// absolute value content growing quadratically in the packed input, so a
/// sweep that materializes running leaf values in a plain big integer
/// reads a superlinear exponent here instead of hiding a `k`-sized
/// constant under a fixed magnitude.
const CLIFF_BASE_SCALE: usize = 128;

/// Comb-scatter tooth count at scale 1.0 (packed cross ~32 KiB).
///
/// Scale drives the tooth count (and with it the scattered party's fragment
/// count, half the teeth); the tooth magnitude stays at
/// [`CROSS_TOOTH_MAGNITUDE_BITS`], so the operands grow linearly and the
/// output-domination ratio holds at every scale.
const CROSS_BASE_TEETH: usize = 128;

/// Comb-scatter tooth magnitude in bits (fixed across scales).
const CROSS_TOOTH_MAGNITUDE_BITS: usize = 1_000;

/// Harmonic spine depth at scale 1.0 (packed size ~6 KiB, matching the
/// dense spine's depth).
const HARMONIC_BASE_DEPTH: usize = 8_000;

/// Scatter population at scale 1.0: balanced-forked parties, one tick
/// each (~10 KiB of packed single-tick versions).
const SCATTER_BASE_CLOCKS: usize = 1_024;

/// Nested-full-sibling depth at scale 1.0 (packed pair ~1.5 KiB).
///
/// Deep enough that a per-level re-scan genre reads its exponent
/// across the level doubling, small enough that the quadratic pin
/// stays inside the board's runtime budget at the record scale.
const NESTED_BASE_DEPTH: usize = 1_500;

/// Nested-wide depth and root-magnitude bits at scale 1.0 (equal, so
/// the doubling scales width and depth together — the cross's cost
/// genre is their product; packed pair ~1.5 KiB).
///
/// Small enough that even a width × depth kernel stays inside the
/// record-scale runtime budget; the red reading rides the exponent
/// leg, not the constant ceiling.
const NESTED_WIDE_BASE: usize = 1_000;

/// Mirror-wide depth and tail-magnitude bits at scale 1.0 (equal, as
/// above; packed pair ~1 KiB). The memo arm's chains grow steeper than
/// the right-full arm's, so the base sits lower.
const MIRROR_WIDE_BASE: usize = 500;

/// Mirror-narrow depth at scale 1.0 (packed pair ~1.5 KiB): the
/// nested-full base, mirrored — the memo machinery at the same depth
/// the right-full cells walk.
const MIRROR_NARROW_BASE_DEPTH: usize = 1_500;

/// Staircase depth at scale 1.0 (packed pair ~2 KiB): deep enough that
/// per-level minimum bookkeeping would read its exponent across the
/// doubling, all values word-scale.
const STAIRCASE_BASE_DEPTH: usize = 1_500;

/// Reveal-comb site count and plateau-magnitude bits at scale 1.0
/// (equal; packed pair ~1 KiB).
///
/// One parameter drives both, so the doubling scales the site count
/// and the circulated width together — the cycle's cost genre is
/// their product. The close-reveal cycle's per-site cost is steeper
/// than the mirror families' chains, so the base sits at the
/// mirror-wide level.
const REVEAL_COMB_BASE: usize = 500;

/// Pure-comb level count and leaf-magnitude bits at scale 1.0 (equal,
/// as above; packed pair ~1 KiB).
///
/// The base watermark stack's own cycle runs at ~2 wide folds per
/// level — a tenth of the reveal comb's constant — so the base sits
/// higher for comparable work.
const PURE_COMB_BASE: usize = 1_000;

/// Ascending-cliff spine length and leaf-magnitude bits at scale 1.0
/// (equal, so the doubling scales the hop count and the residue width
/// together — the cascade's cost genre is their product; packed pair
/// ~1 KiB).
///
/// The cascade runs at ~4 touches per input byte on the cured fold
/// direction — the leveled control's constant — so the base sits at
/// the pure-comb level for comparable work.
const ASCEND_CLIFF_BASE: usize = 1_000;

/// Ticks behind the integer (exponent-zero) rank of the `rank_pair_ops`
/// row: small, so the pair's cost is carried entirely by the mismatch.
const RANK_PAIR_INTEGER_TICKS: u64 = 3;

/// Probes per accumulator byte (as a divisor) on the
/// `party_join_all_overlap` row.
///
/// The probe count scales with the accumulator so the row's exponent
/// judges the fold against a denominator both sides of which double
/// together — work scaling with the fixed accumulator per input reads
/// quadratic there — and the divisor keeps the row inside the board's
/// runtime budget.
const OVERLAP_FOLD_INPUT_DIVISOR: usize = 64;

/// Two-operand jump-comb teeth at scale 1.0 (packed pair ~35 KiB, the
/// teeth operand's per-level wide codes dominating).
///
/// One knob drives the tooth count and, through
/// [`JUMP_PAIR_DIGIT_DIVISOR`], the isolated-position digit count, at
/// the fixed tooth magnitude [`JUMP_PAIR_MAGNITUDE_BITS`]: an
/// absolute-position freeze accounting pays teeth × digits × magnitude
/// here, so the doubling scales the crest count and the position
/// density together while the packed pair grows linearly — the
/// separating choice that makes any such accounting read on the
/// exponent leg rather than hide in a constant.
const JUMP_PAIR_BASE_TEETH: usize = 256;

/// Tooth magnitude (bits) of the two-operand jump comb, fixed across
/// scales: comfortably over the freeze allowance's 256-bit digit bound,
/// so every cheap fold behind a wide difference crest parks the drift.
const JUMP_PAIR_MAGNITUDE_BITS: usize = 512;

/// Isolated-position digits per tooth (as a divisor) on the two-operand
/// jump comb.
///
/// The digit count scales with the teeth at an eighth: deep enough that
/// any per-freeze absolute-position work reads its exponent across the
/// doubling, shallow enough that the shared spine stays a small
/// fraction of the packed pair.
const JUMP_PAIR_DIGIT_DIVISOR: usize = 8;

/// Freeze-position blocks at scale 1.0 (packed version ~74 KiB, the
/// per-block wide drop codes dominating).
///
/// The scale of the `skyline_flatness` freeze-position band's small
/// run: the committed known-bad accounting reads ×1.50 per-byte growth
/// across this regime's doubling (the adequacy tripwire's measurement
/// of record), so the board's default pair straddles exactly what the
/// family exists to catch. The base is a multiple of 16 deliberately:
/// the family's rank exponent is `2s − 1` (one trailing zero strips —
/// exactly one leaf term, the odd `2^L + 1` at weight `2^1`, has
/// 2-adic valuation one), and `rank_sum` lands each small summand at
/// bit remainder `exp mod 32`, where a remainder near the digit top
/// makes most landings span two digits instead of one — an honest
/// amortized-O(1) constant, but one that flips with the remainder, and
/// an exponent fitted across two scales with different remainders
/// reads the flip as growth (measured: e 1.65 from a 1.0 → 1.57
/// per-summand constant at remainders 15 → 31). `16 | s` keeps
/// `2s ≡ 0 (mod 32)`, so every doubling preserves the remainder and
/// the exponent leg compares like against like.
const FREEZE_POS_BASE_BLOCKS: usize = 1_024;

/// Promotion re-arm blocks at scale 1.0 (packed version ~128 KiB, the
/// per-block wide arming codes dominating).
///
/// Half the `skyline_flatness` promotion re-arm band's small run: the
/// committed span-reading promotion reads ×1.74 per-byte growth across
/// that regime's doubling (the span-promotion tripwire's measurement
/// of record), so the board's default pair straddles what the family
/// exists to catch. The base is a multiple of 8 deliberately: the
/// family's rank exponent is `36s`, and `rank_sum` lands its small
/// summands at bit remainder `exp mod 32` (an honest amortized-O(1)
/// constant that flips with the remainder — the freeze-position base's
/// derivation carries the mechanism); `8 | s` keeps `36s ≡ 0 (mod 32)`,
/// so every doubling compares like against like.
const PROMO_REARM_BASE_BLOCKS: usize = 512;

/// Concurrent-pair forked-party count at scale 1.0, rounded up to a
/// power of two at every scale (the balanced fork and the alternating
/// dominance schedule both need it; the level doubling then doubles it
/// exactly).
const CONCURRENT_BASE_LEAVES: usize = 1_024;

/// Benign clock population at scale 1.0.
const BENIGN_BASE_CLOCKS: usize = 256;

/// The weave fold population's leaf count at scale 1.0 (rounded up to a
/// power of two by construction).
///
/// 4096 leaves woven into [`WEAVE_GROUPS`] parties give each operand
/// ~256 scattered leaves under a fully shared upper skeleton — deep
/// enough that the both-present-rich cost terms (the indexed overlap
/// test's per-node table searches, the version joins' interleaved
/// merges) dominate each cell, small enough that the default-scale
/// board stays seconds-fast.
const WEAVE_BASE_LEAVES: usize = 4_096;

/// How many parties the weave population folds: fixed across scales, so
/// the family's scaling axis is operand *size* (the both-present
/// richness per merge), not arity — scatter and benign already own the
/// arity axis.
///
/// 16 groups keep every internal node of the shared skeleton above the
/// last four levels both-present in every operand while each operand
/// stays an organic, individually well-formed region set.
const WEAVE_GROUPS: usize = 16;

/// Floor on every scaled size parameter, so extreme scale-down (the smoke
/// test) still builds valid shapes and a nonempty benign population.
///
/// The floor preserves positivity, never a *relation* between two
/// parameters (an even count, a width strictly under a depth, an ascent
/// inside a code band). Shapes whose generator asserts a relation
/// therefore drive every related parameter from one knob — the
/// `ascend_cliff(s, s)` / `reveal_comb(s, s)` convention — or repair at
/// the call site as [`CONCURRENT_BASE_LEAVES`]'s power-of-two rounding
/// does; a two-knob shape with a floored relation panics in the smoke
/// test's extreme scale-down.
const MIN_SIZE_PARAM: usize = 4;

/// Fixed seed for the benign family's pseudo-random construction: the
/// control row must be deterministic run to run.
const BENIGN_RNG_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

// ─── caller-supplied heap meter ─────────────────────────────────────────────

/// The peak-heap meter the board reads, supplied by the binary that runs it.
///
/// A counting global allocator is per-binary state the library cannot own,
/// so the runner (the `amp_board` example, the smoke test) installs one and
/// passes readers in. All three read the runner's allocator: `reset_peak`
/// clears the peak high-water mark, `peak` reads it, `current` reads live
/// bytes (the baseline subtracted from the peak).
pub struct HeapMeter {
    /// Clear the peak high-water mark down to current usage.
    pub reset_peak: fn(),
    /// The peak live bytes since the last reset.
    pub peak: fn() -> usize,
    /// The currently live bytes.
    pub current: fn() -> usize,
}

// ─── board outcome ──────────────────────────────────────────────────────────

/// The board's bottom line: how many cells scored green and red.
#[derive(Debug, Clone, Copy)]
pub struct Summary {
    /// Cells within every ceiling and exponent bound.
    pub green: usize,
    /// Cells over at least one bound, i.e. amplification findings.
    pub red: usize,
}

// ─── input families ─────────────────────────────────────────────────────────

/// The input families, one column group of the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FamilyKind {
    /// The dense event spine `S(d)`: node count and depth maximizer.
    Dense,
    /// `bigroot(B, d)`: a huge root magnitude over a long spine.
    Bigroot,
    /// `hugeleaf(B)`: one node, maximal bits per node.
    Hugeleaf,
    /// The boundary comb `C(k, n)` at `k = n`: leaf values oscillating
    /// across a `2^k` carry cliff, every crossing paid by a stored code.
    Cliff,
    /// The diverted id-spine pair `I(d, ·)`: full-lockstep two-party walks.
    IdPair,
    /// The output-domination cross: boundary comb × scattered party.
    CombScatter,
    /// The harmonic spine `H(d)`: the rank fold's wide-numerator
    /// adversary, designed against the linear-functional rows and the
    /// rank pair.
    Harmonic,
    /// The scatter-ordered fold population: balanced-forked single-tick
    /// operands whose join accumulator never coalesces; its bundle
    /// carries fold operands alone, so only the fold rows apply.
    Scatter,
    /// The weave fold population: the leaves of one balanced fork tree
    /// dealt round-robin among [`WEAVE_GROUPS`] parties, one tick each.
    ///
    /// Every operand is individually benign — an organic region set any
    /// retire/reunite call site could hold — while every internal node
    /// of the shared upper skeleton is both-present in every operand
    /// pair, so the fold's per-node costs that scale with the *other*
    /// operand (the overlap test against the accumulator, the join
    /// merges over interleaved trees) dominate. Scatter cannot reach
    /// this genre (its operands are single leaves) and benign reaches
    /// it only diluted; the arity is fixed so the scaling axis is
    /// both-present richness alone. Its bundle carries fold operands
    /// alone, so only the fold rows apply.
    Weave,
    /// The nested-full-sibling cross `N(d)` × the dense spine `S(d)`.
    ///
    /// Every level a right-full shortcut site, the deepest stacking of
    /// the walk's deferred right-full decisions and raise bookkeeping
    /// on narrow values — the designated cross of the two tick rows.
    NestedFull,
    /// The wide right-full cross: `bigroot(b, d)` × `N(d)`.
    ///
    /// The stream's first payload is coded absolute, so the deepest
    /// subtree's net movement carries the root's full magnitude and
    /// every level's bookkeeping meets it — width × depth through the
    /// right-full arm. The designated cross of the two tick rows.
    NestedWide,
    /// The wide left-full (memo) cross: `wide_tail(b, d)` × `M(d)`.
    ///
    /// Every proper subtree nets the tail's full magnitude while every
    /// level is a memoized pre-scan site — width × depth through the
    /// left-full arm and the pre-scan's own chains. The designated cross
    /// of the two tick rows.
    MirrorWide,
    /// The narrow left-full (memo) cross: `wide_tail(1, d)` × `M(d)`.
    ///
    /// The memoized pre-scan machinery itself, all values word-scale.
    /// The designated cross of the two tick rows.
    MirrorNarrow,
    /// The descending staircase `D(d)` × the unary id spine `I(d)`.
    ///
    /// Every consumed leaf undercuts every open range's minimum —
    /// full-penetration minimum updates at every level, all values
    /// word-scale. The designated cross of the two tick rows.
    Staircase,
    /// The reveal-comb cross: `reveal_comb(s, s)` × its own id.
    ///
    /// `s` sibling left-full sites share one `2^s`-wide minimum over a
    /// zero floor, and the left-leaning spine closes each site's frame
    /// back into the floor frame between consecutive consumes: the
    /// width-`s` boundary difference is minted at every consume and
    /// popped at every close — the unfunded width circulation, in the
    /// touch currency these columns do not carry (the gate pins in
    /// `tests/meter.rs` enforce it; the bench mirror's time leg sees
    /// it). The designated cross of the two tick rows.
    RevealComb,
    /// The reveal-comb control: `reveal_comb_hifloor(s, s)` × the
    /// reveal-comb id.
    ///
    /// Identical forest and close-reveal cycle with the floor raised
    /// to `2^s − 2`, so the circulated boundary difference is O(1)
    /// wide: the gap control. The designated cross of the two tick rows.
    RevealHifloor,
    /// The pure-comb cross: `pure_comb(s, s)` × its own id.
    ///
    /// The reveal comb's cycle with no left-full site anywhere — no
    /// memo, no pre-scan, no site consume: the base watermark stack's
    /// own arm-move + close-pop width circulation, isolated from the
    /// frame ledger. The designated cross of the two tick rows.
    PureComb,
    /// The ascending-cliff cross: `ascend_cliff(s, s)` × its own id.
    ///
    /// `s` ascending wide leaves stack `s − 1` nonzero unit boundary
    /// differences and a terminal 0-cliff drives one width-`s` undercut
    /// residue through all of them — the cascade whose per-hop fold
    /// direction the gate pins in `tests/meter.rs` price in the touch
    /// currency these columns do not carry. The designated cross of the
    /// two tick rows.
    AscendCliff,
    /// The ascending-cliff control: `ascend_cliff_plateau(s, s)` × the
    /// ascending-cliff id.
    ///
    /// Identical spine, arming schedule, and cliff undercut with every
    /// leaf leveled, so the difference stack is one compressed zero run
    /// the residue passes whole in O(1): the hop-schedule control.
    /// The designated cross of the two tick rows.
    AscendPlateau,
    /// The two-operand jump comb `jump_pair(k, m, d)`: wide
    /// height-difference crests over a dense-position spine.
    ///
    /// The overlay interleaves one operand's wide teeth with the
    /// other's cheap codes, so the pair rows park wide drift at the
    /// other operand's boundaries `2m` times while every absolute
    /// position stays `d` digits dense — the shape that separates
    /// segment-anchored freeze accounting (flat) from absolute-position
    /// accounting (superlinear), with each operand certified-linear
    /// alone (the generator doc carries the mechanism).
    JumpPair,
    /// The freeze-position spine `freeze_position(s)`: the
    /// many-freezes sentinel.
    ///
    /// `2s` descending wide leaves alternate a ten-digit drop and a
    /// unit drop down a right spine, so a query fold freezes `Θ(s)`
    /// times at ever-deeper stream positions — every comb fires O(1)
    /// freezes, which was exactly the coverage hole — and any freeze
    /// accounting that reads an absolute position (or any
    /// whole-history state) per freeze goes quadratic here while the
    /// family's positions compact to O(1) digits. The committed
    /// known-bad kernel reads ×1.50 per byte across the doubling on
    /// this shape (the query fold's adequacy tripwire); the
    /// anchored-segment discipline reads flat (the `skyline_flatness`
    /// freeze-position band). Designed against the linear-functional
    /// query rows.
    FreezePos,
    /// The promotion re-arm spine `promotion_rearm(s)`: the
    /// many-armings sentinel.
    ///
    /// `32s` span-building levels grow the consumed mass's written
    /// span, then `s` four-node blocks each park a wide drift and
    /// promote it at a narrow one — `Θ(s)` query-fold promotions at
    /// O(1) stored codes each, where every comb promotes never and the
    /// freeze-position spine's parked drift is monotone. Any promotion
    /// accounting that re-reads whole-history state per arming goes
    /// quadratic here while the family's suffix masses compact to O(1)
    /// balanced terms. The committed known-bad kernel reads ×1.74 per
    /// byte across the doubling on this shape (the query fold's
    /// span-promotion tripwire); the promotion ledger reads flat (the
    /// `skyline_flatness` promotion re-arm bands). Designed against
    /// the linear-functional query rows.
    PromoRearm,
    /// The concurrent pair `concurrent_pair(n)`: the emit side-switch
    /// density population.
    ///
    /// Organically forked and ticked so the sweep's side switch fires at
    /// every one of the `n − 1` overlay boundaries, join and meet alike
    /// — the pairing the ticked counterpart cannot reach.
    ConcurrentPair,
    /// The fixed-seed organic control population.
    Benign,
}

/// Every family, in display order.
///
/// Adding a shape: the array length and the [`FamilyData::build`] and
/// [`designed`] match arms are compiler-forced from here.
/// What the compiler cannot force, in the order it is otherwise found by
/// luck: the shape's base-size constant (the block above, with its
/// derivation doc), the module doc's `# Families` prose and any
/// cardinality it carries, the cell-count pin and its derivation comment
/// (`tests/amp_board_smoke.rs`), the envelope rows in `tests/meter.rs`
/// (the enforced record), the ceiling-calibration witnesses (the pinned
/// ceilings section's header comment), and — only if a cell is expected
/// red — the rider list ([`BOARD_RED_BENCH_RIDERS`]) plus the roster and
/// its membership pin (`tools/benchjudge-expected.json`,
/// `tests/bench_judge_roster.rs`). And not every shape belongs here: a
/// whole-surface adversary earns a board family, while a kernel-seam
/// shape lives in the envelope suite alone, as `wide_tooth_comb`,
/// `alt_spine`, and the `memo_*` shapes do.
const FAMILIES: [FamilyKind; 24] = [
    FamilyKind::Dense,
    FamilyKind::Bigroot,
    FamilyKind::Hugeleaf,
    FamilyKind::Cliff,
    FamilyKind::IdPair,
    FamilyKind::CombScatter,
    FamilyKind::Harmonic,
    FamilyKind::Scatter,
    FamilyKind::Weave,
    FamilyKind::NestedFull,
    FamilyKind::NestedWide,
    FamilyKind::MirrorWide,
    FamilyKind::MirrorNarrow,
    FamilyKind::Staircase,
    FamilyKind::RevealComb,
    FamilyKind::RevealHifloor,
    FamilyKind::PureComb,
    FamilyKind::AscendCliff,
    FamilyKind::AscendPlateau,
    FamilyKind::JumpPair,
    FamilyKind::FreezePos,
    FamilyKind::PromoRearm,
    FamilyKind::ConcurrentPair,
    FamilyKind::Benign,
];

/// One shape instantiated at one scale: the operand bundle every row's
/// `prepare` decodes fresh (outside measurement).
///
/// The bundle is the shape axis's declaration: each slot a shape fills
/// flows to every operation whose signature consumes it, so a shape's
/// reach is structural — build a shape with a version and it appears on
/// every version row; give it an id side and it appears on every party
/// row through the disjoint-mount adapter. The derived slots (`version2`,
/// `rank_pair`, a cross shape's `version` and `parties`) are filled by
/// one uniform post-pass in [`FamilyData::build`], never per shape.
struct FamilyData {
    kind: FamilyKind,
    name: &'static str,
    /// The shape's primary packed version (a cross shape's event side).
    version: Option<Vec<u8>>,
    /// The comparison counterpart: `version` plus one seed tick, packed.
    ///
    /// Derived uniformly by the post-pass — except on the pair shapes
    /// (jump-pair, concurrent-pair), whose build arms fill it with the
    /// pairing the shape was constructed around and the post-pass leaves
    /// in place.
    version2: Option<Vec<u8>>,
    /// A disjoint packed party pair within one universe: natural for the
    /// id pair and the benign halves, minted by the disjoint-mount
    /// adapter from a cross shape's id side.
    parties: Option<(Vec<u8>, Vec<u8>)>,
    /// The designated packed (event version, id party) cross: the pairing
    /// the shape was built around, driving the tick rows' walk floors and
    /// the clock rows' operand choice.
    ///
    /// Each cross shape's variant doc states the arm and cost genre its
    /// cross drives.
    cross: Option<(Vec<u8>, Vec<u8>)>,
    /// Whether the cross's mandatory projection output dominates its
    /// input (the comb-scatter and plateau-comb shapes): the projection
    /// rows I/O-denominate exactly these cells (the module doc's
    /// Denomination section).
    output_dominated: bool,
    /// The bundle's value content in bytes, `Some` only on the
    /// flat-denominator shape (comb-scatter).
    ///
    /// The denominator every input-denominated cell's *exponent* is
    /// fitted against; constants and floors stay per packed byte (the
    /// module doc's Denomination section derives the split).
    content_bytes: Option<usize>,
    /// The packed fold operands (versions, parties), consumed by the two
    /// fold rows alone: the scatter shape's adversarially ordered
    /// population and the benign shape's organic control.
    #[allow(clippy::type_complexity)]
    fold: Option<(Vec<Vec<u8>>, Vec<Vec<u8>>)>,
    /// An overlapping packed party pair within one universe: the
    /// rejection rows' operands.
    ///
    /// Minted by the overlap-mount adapter from the same id source as
    /// `parties` (the post-pass); semantically void by design — see
    /// [`overlap_mounted_pair`].
    overlap: Option<(Vec<u8>, Vec<u8>)>,
    /// The mismatched rank pair, derived from `version` in the post-pass.
    ///
    /// Precomputed here (shape-derived rank, small integer rank) so the
    /// `rank_pair_ops` and `rank_sum` prepares clone their operands instead
    /// of re-running the rank fold:
    /// the bench harness calls prepare once per timed iteration, and the
    /// fold costs orders of magnitude more than the pair operations it
    /// feeds.
    rank_pair: Option<(Rank, Rank)>,
}

impl FamilyData {
    /// A bundle with every slot empty, for a build arm to fill with what
    /// the shape honestly has.
    fn bare(kind: FamilyKind, name: &'static str) -> FamilyData {
        FamilyData {
            kind,
            name,
            version: None,
            version2: None,
            parties: None,
            cross: None,
            output_dominated: false,
            content_bytes: None,
            fold: None,
            overlap: None,
            rank_pair: None,
        }
    }

    /// Build a shape's operand bundle at `scale`, doubled `level` times.
    ///
    /// `level` 0 and 1 are the two measurement scales of every cell. The
    /// arm fills the slots the shape natively has; the post-pass below
    /// derives the rest uniformly (a cross shape's version is its event
    /// side, its party pair is the disjoint-mount adapter over its id
    /// side; every version gains its rank pair, and its ticked
    /// counterpart wherever the arm built no pairing of its own), so a
    /// new shape reaches every operation its bundle supplies without
    /// naming any.
    fn build(kind: FamilyKind, scale: f64, level: u32) -> FamilyData {
        let size = |base: usize| -> usize {
            let scaled = ((base as f64) * scale).round() as usize;
            scaled.max(MIN_SIZE_PARAM) << level
        };
        let mut data = match kind {
            FamilyKind::Dense => Self::event(
                kind,
                "dense",
                super::dense(size(DENSE_BASE_DEPTH)).version().encode(),
            ),
            FamilyKind::Bigroot => Self::event(
                kind,
                "bigroot",
                super::bigroot(size(BIGROOT_BASE_MAGNITUDE_BITS), size(BIGROOT_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyKind::Hugeleaf => Self::event(
                kind,
                "hugeleaf",
                super::hugeleaf(size(HUGELEAF_BASE_MAGNITUDE_BITS))
                    .version()
                    .encode(),
            ),
            FamilyKind::Cliff => {
                let scale = size(CLIFF_BASE_SCALE);
                Self::event(
                    kind,
                    "cliff",
                    super::cliff_comb(scale, scale).version().encode(),
                )
            }
            FamilyKind::IdPair => {
                let mut data = Self::bare(kind, "id-pair");
                data.parties = Some((
                    super::id_spine(size(ID_BASE_DEPTH), false).bytes,
                    super::id_spine(size(ID_BASE_DEPTH), true).bytes,
                ));
                data
            }
            FamilyKind::CombScatter => {
                let teeth = size(CROSS_BASE_TEETH);
                let mut data = Self::bare(kind, "comb-scatter");
                data.cross = Some((
                    super::cliff_comb(CROSS_TOOTH_MAGNITUDE_BITS, teeth)
                        .version()
                        .encode(),
                    super::scattered_id(teeth / 2).bytes,
                ));
                data.output_dominated = true;
                let (v, p) = data.cross.as_ref().expect("just set");
                data.content_bytes = Some(value_content_bytes(&decode_version(v)) + p.len());
                data
            }
            FamilyKind::Harmonic => Self::event(
                kind,
                "harmonic",
                super::harmonic(size(HARMONIC_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyKind::Scatter => Self::scatter(size(SCATTER_BASE_CLOCKS)),
            FamilyKind::Weave => Self::weave(size(WEAVE_BASE_LEAVES)),
            FamilyKind::NestedFull => {
                let d = size(NESTED_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "nested-full",
                    super::dense(d).version().encode(),
                    super::nested_full_id(d).bytes,
                )
            }
            FamilyKind::NestedWide => {
                let s = size(NESTED_WIDE_BASE);
                Self::cross_family(
                    kind,
                    "nested-wide",
                    super::bigroot(s, s).version().encode(),
                    super::nested_full_id(s).bytes,
                )
            }
            FamilyKind::MirrorWide => {
                let s = size(MIRROR_WIDE_BASE);
                Self::cross_family(
                    kind,
                    "mirror-wide",
                    super::wide_tail(s, s).version().encode(),
                    super::nested_left_full_id(s).bytes,
                )
            }
            FamilyKind::MirrorNarrow => {
                let d = size(MIRROR_NARROW_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "mirror-narrow",
                    super::wide_tail(1, d).version().encode(),
                    super::nested_left_full_id(d).bytes,
                )
            }
            FamilyKind::Staircase => {
                let d = size(STAIRCASE_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "staircase",
                    super::staircase(d).version().encode(),
                    super::id_spine(d, false).bytes,
                )
            }
            FamilyKind::RevealComb => {
                let s = size(REVEAL_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    "reveal-comb",
                    super::reveal_comb(s, s).version().encode(),
                    super::reveal_comb_id(s).bytes,
                );
                // Projecting the shared-wide-plateau event through its
                // site-owning comb id re-materializes a wide absolute
                // value per kept site: mandatory output Theta(k*b) on a
                // Theta(k + b) input, the same output domination the
                // comb-scatter cross declares [measured: output x4 per
                // input doubling, every work column within x4 of it].
                data.output_dominated = true;
                data
            }
            FamilyKind::RevealHifloor => {
                let s = size(REVEAL_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    "reveal-hifloor",
                    super::reveal_comb_hifloor(s, s).version().encode(),
                    super::reveal_comb_id(s).bytes,
                );
                // The raised floor changes the consume-time gap, not the
                // projection's re-materialized wide sites: the same
                // output domination as reveal-comb.
                data.output_dominated = true;
                data
            }
            FamilyKind::PureComb => {
                let s = size(PURE_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    "pure-comb",
                    super::pure_comb(s, s).version().encode(),
                    super::pure_comb_id(s).bytes,
                );
                // Bare wide leaves under the site-owning id: the masked
                // skyline spells a wide code per owned site, the same
                // output domination as reveal-comb.
                data.output_dominated = true;
                data
            }
            FamilyKind::AscendCliff => {
                let s = size(ASCEND_CLIFF_BASE);
                Self::cross_family(
                    kind,
                    "ascend-cliff",
                    super::ascend_cliff(s, s).version().encode(),
                    super::ascend_cliff_id(s).bytes,
                )
            }
            FamilyKind::AscendPlateau => {
                let s = size(ASCEND_CLIFF_BASE);
                Self::cross_family(
                    kind,
                    "ascend-plateau",
                    super::ascend_cliff_plateau(s, s).version().encode(),
                    super::ascend_cliff_id(s).bytes,
                )
            }
            FamilyKind::JumpPair => {
                let m = size(JUMP_PAIR_BASE_TEETH);
                let d = (m / JUMP_PAIR_DIGIT_DIVISOR).max(1);
                let (a, b) = super::jump_pair(JUMP_PAIR_MAGNITUDE_BITS, m, d);
                let mut data = Self::event(kind, "jump-pair", a.version().encode());
                data.version2 = Some(b.version().encode());
                data
            }
            FamilyKind::FreezePos => Self::event(
                kind,
                "freeze-pos",
                super::freeze_position(size(FREEZE_POS_BASE_BLOCKS))
                    .version()
                    .encode(),
            ),
            FamilyKind::PromoRearm => Self::event(
                kind,
                "promo-rearm",
                super::promotion_rearm(size(PROMO_REARM_BASE_BLOCKS))
                    .version()
                    .encode(),
            ),
            FamilyKind::ConcurrentPair => {
                let n = size(CONCURRENT_BASE_LEAVES).next_power_of_two();
                let (v, w) = super::concurrent_pair(n);
                let mut data = Self::event(kind, "concurrent-pair", v.encode());
                data.version2 = Some(w.encode());
                data
            }
            FamilyKind::Benign => Self::benign(size(BENIGN_BASE_CLOCKS)),
        };
        // ── the bundle post-pass: the derived slots, uniform across shapes ──
        // A cross shape's primary version is its event side.
        if data.version.is_none() {
            data.version = data.cross.as_ref().map(|(v, _)| v.clone());
        }
        // Every version gains its ticked comparison counterpart (where
        // the shape did not build its own pairing) and its mismatched
        // rank pair (shape-derived rank against a small integer rank,
        // the pair whose exponent mismatch the rank rows price).
        if let Some(bytes) = &data.version {
            let v = decode_version(bytes);
            if data.version2.is_none() {
                let mut w = v.clone();
                w.tick(&Party::seed());
                data.version2 = Some(w.encode());
            }
            let b = Version::try_from(RANK_PAIR_INTEGER_TICKS)
                .expect("a small integer version is valid")
                .rank();
            data.rank_pair = Some((v.rank(), b));
        }
        // A cross shape's id side becomes a disjoint party pair through
        // the mount adapter.
        if data.parties.is_none() {
            if let Some((_, id)) = &data.cross {
                data.parties = Some(disjoint_mounted_pair(id));
            }
        }
        // Every id source also mints an overlapping pair through the
        // overlap-mount adapter, for the rejection rows: the cross id
        // where the shape has one, the first natural party otherwise.
        if data.overlap.is_none() {
            let id = data
                .cross
                .as_ref()
                .map(|(_, id)| id)
                .or_else(|| data.parties.as_ref().map(|(a, _)| a));
            if let Some(id) = id {
                data.overlap = Some(overlap_mounted_pair(id));
            }
        }
        data
    }

    /// Build the scatter fold population: `n` balanced-forked parties, one
    /// tick each, ordered evens before odds so a sequential fold's
    /// accumulator holds every other leaf and never coalesces.
    fn scatter(n: usize) -> FamilyData {
        let mut parties = vec![Party::seed()];
        while parties.len() < n {
            let mut next = Vec::with_capacity(parties.len() * 2);
            for mut p in parties {
                let q = p.fork();
                next.push(p);
                next.push(q);
            }
            parties = next;
        }
        // Dropping the tail keeps `n` honest at non-power-of-two scales;
        // a dropped party's region simply goes unowned.
        parties.truncate(n);
        let scatter_order = |v: Vec<Vec<u8>>| -> Vec<Vec<u8>> {
            let (evens, odds): (Vec<_>, Vec<_>) =
                v.into_iter().enumerate().partition(|(i, _)| i % 2 == 0);
            evens
                .into_iter()
                .chain(odds)
                .map(|(_, bytes)| bytes)
                .collect()
        };
        let versions = scatter_order(
            parties
                .iter()
                .map(|p| {
                    let mut v = Version::new();
                    v.tick(p);
                    v.encode()
                })
                .collect(),
        );
        let parties = scatter_order(parties.iter().map(Party::encode).collect());
        let mut data = Self::bare(FamilyKind::Scatter, "scatter");
        data.fold = Some((versions, parties));
        data
    }

    /// Build the weave fold population.
    ///
    /// The `leaves` (rounded up to a power of two) leaf parties of one
    /// balanced fork expansion are dealt round-robin into
    /// [`WEAVE_GROUPS`] group parties, each group carrying its own
    /// single-tick version.
    ///
    /// Dealing leaf `i` to group `i % WEAVE_GROUPS` puts leaves of every
    /// group under every skeleton node above the last `log2(WEAVE_GROUPS)`
    /// levels, so each operand pair is both-present at the whole shared
    /// skeleton — the correlated-population genre — while each group on
    /// its own is an ordinary scattered region set.
    fn weave(leaves: usize) -> FamilyData {
        let leaves = leaves.next_power_of_two().max(WEAVE_GROUPS * 2);
        let mut parties = vec![Party::seed()];
        while parties.len() < leaves {
            let mut next = Vec::with_capacity(parties.len() * 2);
            for mut p in parties {
                let q = p.fork();
                next.push(p);
                next.push(q);
            }
            parties = next;
        }
        // Deal the leaves round-robin: each group accumulates its party
        // by joining every WEAVE_GROUPS-th leaf, and its version by one
        // tick per dealt leaf — a single-leaf party forces the event onto
        // that leaf, so the group's version is height one exactly over
        // its scattered region, a deep tree sharing the whole upper
        // skeleton with every other group's.
        let mut group_parties: Vec<Option<Party>> = (0..WEAVE_GROUPS).map(|_| None).collect();
        let mut group_versions: Vec<Version> = (0..WEAVE_GROUPS).map(|_| Version::new()).collect();
        for (i, leaf) in parties.into_iter().enumerate() {
            let r = i % WEAVE_GROUPS;
            group_versions[r].tick(&leaf);
            match &mut group_parties[r] {
                slot @ None => *slot = Some(leaf),
                Some(group) => group
                    .join(leaf)
                    .expect("leaves of one fork expansion are pairwise disjoint"),
            }
        }
        let versions = group_versions.iter().map(Version::encode).collect();
        let parties = group_parties
            .into_iter()
            .map(|g| g.expect("every group received leaves").encode())
            .collect();
        let mut data = Self::bare(FamilyKind::Weave, "weave");
        data.fold = Some((versions, parties));
        data
    }

    /// Wrap a cross shape: a packed (event, id) pair built as one
    /// adversarial pairing.
    ///
    /// The cross drives the tick rows' walk floors and the clock rows'
    /// operand choice directly; the post-pass derives the shape's version
    /// (the event side) and its disjoint party pair (the mounted id side),
    /// so the shape also reaches every version and party row.
    fn cross_family(
        kind: FamilyKind,
        name: &'static str,
        version: Vec<u8>,
        id: Vec<u8>,
    ) -> FamilyData {
        let mut data = Self::bare(kind, name);
        data.cross = Some((version, id));
        data
    }

    /// Wrap an event shape's wire bytes.
    fn event(kind: FamilyKind, name: &'static str, bytes: Vec<u8>) -> FamilyData {
        let mut data = Self::bare(kind, name);
        data.version = Some(bytes);
        data
    }

    /// Build the benign control: `n` clocks forked at random from a seed,
    /// each ticked one to three times, folded into one version and two
    /// disjoint half-population parties.
    fn benign(n: usize) -> FamilyData {
        let mut rng = XorShift(BENIGN_RNG_SEED);
        let mut clocks = vec![Clock::seed()];
        while clocks.len() < n {
            let i = (rng.next() as usize) % clocks.len();
            let child = clocks[i].fork();
            clocks.push(child);
        }
        for clock in &mut clocks {
            for _ in 0..(rng.next() % 3 + 1) {
                clock.tick();
            }
        }
        let version = Version::join_all(clocks.iter().map(|c| c.version().clone()));
        // The fold rows' organic control: the population's own versions and
        // parties in construction order (the adversarial ordering belongs to
        // the scatter family alone).
        let fold = Some((
            clocks.iter().map(|c| c.version().encode()).collect(),
            clocks.iter().map(|c| c.party().encode()).collect(),
        ));
        let mut parties = clocks.into_iter().map(|c| c.into_parts().0);
        let mut a = parties.next().expect("the population is nonempty");
        let mut b = parties
            .next()
            .expect("MIN_SIZE_PARAM keeps at least two clocks in the population");
        for (i, p) in parties.enumerate() {
            // Alternate the halves so both operand parties scatter across
            // the whole id tree rather than owning one contiguous region.
            let half = if i % 2 == 0 { &mut a } else { &mut b };
            half.join(p).expect("forked parties are pairwise disjoint");
        }
        let mut data = Self::bare(FamilyKind::Benign, "benign");
        data.version = Some(version.encode());
        data.parties = Some((a.encode(), b.encode()));
        data.fold = fold;
        data
    }

    /// The primary version, decoded fresh, with its packed byte length.
    fn version(&self) -> Option<(Version, usize)> {
        let bytes = self.version.as_ref()?;
        Some((decode_version(bytes), bytes.len()))
    }

    /// Both versions decoded fresh, with their combined packed byte length.
    fn version_pair(&self) -> Option<(Version, Version, usize)> {
        let (v, n) = self.version()?;
        let bytes2 = self.version2.as_ref()?;
        Some((v, decode_version(bytes2), n + bytes2.len()))
    }

    /// The disjoint party pair decoded fresh, with combined byte length.
    fn party_pair(&self) -> Option<(Party, Party, usize)> {
        let (a, b) = self.parties.as_ref()?;
        Some((decode_party(a), decode_party(b), a.len() + b.len()))
    }

    /// The designated cross decoded fresh (event version, id party), with
    /// combined packed byte length.
    fn cross(&self) -> Option<(Version, Party, usize)> {
        let (v, p) = self.cross.as_ref()?;
        Some((decode_version(v), decode_party(p), v.len() + p.len()))
    }

    /// One clock per shape, from the bundle's slots.
    ///
    /// A cross shape pairs its own id and event sides; a version-bearing
    /// shape pairs the seed party with the adversarial version; a
    /// party-only shape pairs the adversarial party with the empty
    /// version.
    fn clock(&self) -> Option<(Clock, usize)> {
        if let Some((v, p, n)) = self.cross() {
            return Some((Clock::from_parts(p, v), n));
        }
        if let Some((v, n)) = self.version() {
            return Some((Clock::from_parts(Party::seed(), v), n + 1));
        }
        let (a, _, _) = self.party_pair()?;
        let n = self.parties.as_ref().map(|(a, _)| a.len())?;
        Some((Clock::from_parts(a, Version::new()), n + 1))
    }

    /// Two joinable clocks (disjoint parties), with combined operand
    /// bytes, from the bundle's slots.
    ///
    /// A shape with both a party pair and versions crosses them; a
    /// party-only pair rides empty versions; a version-only shape forks
    /// a seed pair around its version pair.
    fn clock_pair(&self) -> Option<(Clock, Clock, usize)> {
        match (self.parties.is_some(), self.version.is_some()) {
            (true, true) => {
                let (a, b, np) = self.party_pair()?;
                let (v, w, nv) = self.version_pair()?;
                Some((Clock::from_parts(a, v), Clock::from_parts(b, w), np + nv))
            }
            (true, false) => {
                let (a, b, n) = self.party_pair()?;
                Some((
                    Clock::from_parts(a, Version::new()),
                    Clock::from_parts(b, Version::new()),
                    n + 2,
                ))
            }
            (false, true) => {
                let (v, w, n) = self.version_pair()?;
                let mut p = Party::seed();
                let q = p.fork();
                Some((Clock::from_parts(p, v), Clock::from_parts(q, w), n + 2))
            }
            (false, false) => None,
        }
    }
}

/// The disjoint-mount adapter: lift one packed id shape into a disjoint
/// party pair inside a single universe.
///
/// The pair mounts the shape under opposite children of a fresh root —
/// `(shape, ·)` and `(·, shape)` — so the halves are disjoint by
/// construction and joining them merely reunites the root's two subtrees:
/// two independently-generated id shapes are never asked to share a
/// universe (linearity of parties is the invariant everything rests on —
/// the crate docs' safety rules). Each half is the shape itself one level
/// deeper, so party cells on a mounted shape measure the shape plus one
/// root tag. Runs at bundle build, outside any measurement, and asserts
/// the disjointness it mints.
fn disjoint_mounted_pair(id: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let shape = decode_party(id);
    let mount = |left: bool| -> Vec<u8> {
        let mut bits = codec::Bits::with_capacity(shape.as_bits().len() + 2);
        bits.push(left);
        bits.push(!left);
        bits.extend_from_bitslice(shape.as_bits());
        codec::zero_dead_bits(&mut bits);
        bits.into_vec()
    };
    let (a, b) = (mount(true), mount(false));
    assert!(
        decode_party(&a).is_disjoint(&decode_party(&b)),
        "the disjoint-mount adapter must mint a disjoint pair"
    );
    (a, b)
}

/// The overlap-mount adapter: lift one packed id shape into an
/// *overlapping* party pair whose single shared region sits at both
/// operands' preorder ends — the disjoint-mount adapter's counterpart,
/// for the rejection rows.
///
/// `a` mounts the shape under a fresh root's left child and a marker
/// under its right; `b` mounts the shape under the right child alone.
/// The marker is a single-child chain along the shape's rightmost-present
/// path ending in a terminal at the shape's preorder-last owned position,
/// so the pair's one overlap is the last position a lockstep walk over
/// `b`'s side reaches, with every earlier region disjoint — rejection
/// consumes essentially both streams before the witnessing pair meets.
///
/// The outputs are **semantically void by design**: a well-formed pair
/// that no legal fork/join history produces (two claims on one region),
/// built on purpose because the crate's cost claims are total — the
/// rejection rows price what rejecting such a pair costs, and nothing
/// downstream treats the pair as meaningful. Runs at bundle build,
/// outside any measurement, and asserts the overlap it mints (both
/// halves decode canonically on the way).
fn overlap_mounted_pair(id: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let shape = decode_party(id);
    let bits = shape.as_bits();
    let path = rightmost_terminal_path(bits);
    assert!(
        !path.is_empty(),
        "the overlap-mount adapter needs a non-terminal shape: a full shape's mount would \
         not be normal form"
    );
    let mut a = codec::Bits::with_capacity(bits.len() + 2 * path.len() + 4);
    a.push(true); // root: both children present
    a.push(true);
    a.extend_from_bitslice(bits); // left: the shape
    for &go_right in &path {
        // right: the marker chain, one single-child node per level
        a.push(!go_right);
        a.push(go_right);
    }
    a.push(false); // the marker's terminal, at the shape's last owned position
    a.push(false);
    codec::zero_dead_bits(&mut a);
    let mut b = codec::Bits::with_capacity(bits.len() + 2);
    b.push(false); // root: right child only
    b.push(true);
    b.extend_from_bitslice(bits); // right: the shape
    codec::zero_dead_bits(&mut b);
    let (a, b) = (a.into_vec(), b.into_vec());
    assert!(
        !decode_party(&a).is_disjoint(&decode_party(&b)),
        "the overlap-mount adapter must mint an overlapping pair"
    );
    (a, b)
}

/// The branch choices (`false` left, `true` right) from an id tree's root
/// to its preorder-last terminal: at every node, the last present child.
///
/// Preorder lays each subtree's bits contiguously, so the stream's final
/// tag belongs to the node reached by always taking the rightmost present
/// child; left subtrees along the way are skipped (each exactly once, so
/// the walk is linear). Runs at bundle build, outside any measurement.
fn rightmost_terminal_path(bits: &codec::BitsSlice) -> Vec<bool> {
    let mut pos = 0usize;
    let mut path = Vec::new();
    loop {
        let left = bits[pos];
        let right = bits[pos + 1];
        pos += 2;
        if !left && !right {
            return path; // the terminal
        }
        if right {
            if left {
                pos = crate::idbits::skip_subtree(pos, |at| {
                    let children = usize::from(bits[at]) + usize::from(bits[at + 1]);
                    (children, at + 2)
                });
            }
            path.push(true);
        } else {
            path.push(false);
        }
    }
}

/// The overlap fold's probe: a right-mounted full leaf — `(0, 1)`, one
/// packed byte — overlapping the a-mount's whole right half (the marker's
/// region).
///
/// The `party_join_all_overlap` row's per-input operand. The witnessing
/// pair sits in the right half, behind the accumulator's whole left
/// shape, so a per-input overlap test priced in the accumulator — a
/// cursor walk skip-scanning the left shape to reach the witness — reads
/// Θ(accumulator) scan per O(1)-byte input and turns the row quadratic;
/// the fold's per-call accumulator index answers the same test in
/// O(probe), which is the separation the row watches.
fn overlap_fold_probe() -> Vec<u8> {
    let mut probe = codec::Bits::with_capacity(4);
    probe.push(false); // root: right child only
    probe.push(true);
    probe.push(false); // the right child: a full leaf
    probe.push(false);
    codec::zero_dead_bits(&mut probe);
    probe.into_vec()
}

/// `bytes` with its last byte dropped.
///
/// A strict prefix of a preorder stream has an open subtree at every
/// position before its true end, so this is the maximally-deferred
/// [`Truncated`](crate::error::Decode) defect — discoverable only by
/// parsing to the cut.
fn truncated_bytes(bytes: &[u8]) -> Vec<u8> {
    assert!(
        bytes.len() >= 2,
        "a truncation row needs a stream of at least two bytes"
    );
    bytes[..bytes.len() - 1].to_vec()
}

/// `bytes` with a `0xFF` byte appended after the complete valid stream:
/// the maximally-deferred [`TrailingBits`](crate::error::Decode) defect —
/// the whole tree parses before the nonzero tail is seen.
fn trailing_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out.push(0xFF);
    out
}

/// The bit position of a version stream's preorder-last leaf flag.
///
/// Iterative over the packed form, outside any measurement; the last
/// node of a preorder event stream is always a leaf (an internal node's
/// children would follow it).
fn last_leaf_flag_pos(v: &Version) -> usize {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut last = 0usize;
    while pending > 0 {
        pending -= 1;
        let flag = pos;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (_, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        last = flag;
    }
    last
}

/// `v`'s stream with its preorder-last leaf split into an equal-sibling
/// pair.
///
/// The left child keeps the old leaf's delta code (same predecessor,
/// same value); the right child's delta is zero — the minimality
/// violation the validator can only judge at that pair's close, the
/// stream's last position. The maximally-deferred
/// [`NotCanonical`](crate::error::Decode) defect.
fn version_noncanonical_bytes(v: &Version) -> Vec<u8> {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let leaf = last_leaf_flag_pos(v);
    let mut out = codec::Bits::with_capacity(bits.len() + 4);
    out.extend_from_bitslice(&bits[..leaf]);
    out.push(false); // the old leaf's position becomes an internal node
    out.extend_from_bitslice(&bits[leaf..]); // left child: the old leaf verbatim
    out.push(true); // right child: a leaf equal to its sibling
    codec::encode_int(&mut out, &Base::from(0u32)); // zero delta
    codec::zero_dead_bits(&mut out);
    out.into_vec()
}

/// `p`'s stream with its preorder-last terminal split into a collapsible
/// `(1, 1)`.
///
/// Two full children, judged non-normal at the node's close — the
/// stream's last position: the maximally-deferred
/// [`NotCanonical`](crate::error::Decode) defect on the id side.
fn party_noncanonical_bytes(p: &Party) -> Vec<u8> {
    let bits = p.as_bits();
    let end = bits.len();
    assert!(
        !bits[end - 2] && !bits[end - 1],
        "a preorder id stream ends in a terminal tag"
    );
    let mut out = codec::Bits::with_capacity(end + 4);
    out.extend_from_bitslice(&bits[..end - 2]);
    out.push(true); // the last terminal becomes a node with both children
    out.push(true);
    for _ in 0..2 {
        out.push(false); // each child a terminal: the collapsible (1, 1)
        out.push(false);
    }
    codec::zero_dead_bits(&mut out);
    out.into_vec()
}

/// `text` with junk appended after the complete valid notation: the
/// parser consumes the whole text before the trailing defect is seen
/// ([`Parse::Syntax`](crate::error::Parse)).
fn trailing_text(text: &str) -> String {
    let mut out = text.to_owned();
    out.push('x');
    out
}

/// A clock's text with junk inserted before the closing paren, inside the
/// version component.
///
/// The clock parser's outer-paren check rejects *appended* junk in O(1),
/// so the deferred defect rides the version side, which parses in full
/// first.
fn clock_trailing_text(text: &str) -> String {
    let mut out = text.to_owned();
    assert_eq!(out.pop(), Some(')'), "a clock renders as (id, event)");
    out.push_str("x)");
    out
}

/// `text` with its last spelled value `t` re-spelled `(0, t, t)`: equal
/// sibling leaves, well-formed and judged non-canonical at that node's
/// close — the text's end
/// ([`Parse::NotCanonical`](crate::error::Parse)).
fn version_noncanonical_text(text: &str) -> String {
    let end = text
        .rfind(|c: char| c.is_ascii_digit())
        .expect("a version's text spells at least one value")
        + 1;
    let start = text[..end]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    let t = &text[start..end];
    format!("{}(0, {t}, {t}){}", &text[..start], &text[end..])
}

/// `text` with its last `1` token re-spelled `(1, 1)`: the collapsible
/// pair, judged non-normal at the node's close, at the text's end
/// ([`Parse::NotCanonical`](crate::error::Parse)).
fn party_noncanonical_text(text: &str) -> String {
    let at = text
        .rfind('1')
        .expect("a party's text spells at least one owned leaf");
    format!("{}(1, 1){}", &text[..at], &text[at + 1..])
}

/// Decode packed bytes the board itself generated.
fn decode_version(bytes: &[u8]) -> Version {
    Version::decode(bytes).expect("board-generated version bytes are canonical")
}

/// Decode packed party bytes the board itself generated.
fn decode_party(bytes: &[u8]) -> Party {
    Party::decode(bytes).expect("board-generated party bytes are canonical")
}

/// A tiny xorshift64 generator: deterministic, dependency-free randomness
/// for the benign control family.
struct XorShift(u64);

impl XorShift {
    /// The next pseudo-random word.
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

// ─── liveness floors ────────────────────────────────────────────────────────

// The floor derivations and not-applicable reasons, shared across rows so
// the rendered legend stays small and uniform.

/// Scan floor: the operation must examine its packed operands in full.
const WHY_SCAN_EXAMINES: &str =
    "must examine its packed operands: at least one scanned bit per packed byte";
/// Scan floor: early exit is legitimate, but the root codes are still read.
const WHY_SCAN_TOUCH: &str =
    "may answer at the first divergence: still reads the operands' root codes";
/// Scan NA (`version_eq`'s own): same-form equality is decided on the
/// stored canonical bytes wholesale, and the operands grow without bound.
const NA_SCAN_EQ_BYTES: &str = "decides same-form equality on the stored canonical bytes \
     wholesale (the compare may legitimately stop at the first differing byte): no stream walk \
     is in the contract; unlike the hash rows' small-operand exposure, eq operands grow without \
     bound, so the bench judge's time leg is the backstop that the compare stays linear";
/// Scan NA: the contract is a wholesale byte move.
const NA_SCAN_BYTE_COPY: &str =
    "moves or hashes the stored canonical bytes wholesale: no stream walk is in the contract";
/// Scan NA: the operands carry no packed stream.
const NA_SCAN_NO_STREAM: &str = "operands are decoded rank values: no packed stream exists";
/// Scan NA: a trivial (seed) operand stores no bits to scan.
const NA_SCAN_SEED_PARTY: &str = "the forked party is the seed: its packed form is empty";
/// Limb floor: the parse rows materialize every wide spelled value.
const WHY_LIMB_WIDE: &str = "a magnitude wider than the machine-word bound must be materialized \
     or folded limb by limb: one op per 64 magnitude bits (the parse direction converts every \
     spelled value, so the decoded tree's stored bases are all mandatory)";
/// Limb floor: a walk over the stored form decodes every wide payload code.
const WHY_LIMB_STREAM: &str = "every payload code of the stored stream wider than the \
     machine-word bound must be decoded limb by limb: one op per 64 code bits (the stream's \
     own codes, not the decoded tree's values — a plateau of equal wide leaves stores its \
     width once)";
/// Limb floor: the rank pair's sum spans the wider operand's content.
const WHY_LIMB_RANK_PAIR: &str = "the mismatched pair's sum carries a numerator as wide as the \
     wider operand's value content: one limb write per 64 content bits";
/// Limb floor: the rank fold's sum spans its widest summand's content.
const WHY_LIMB_RANK_SUM: &str = "the fold's sum carries a numerator as wide as its widest \
     summand's value content: one limb write per 64 content bits";
/// Limb NA: every operand magnitude fits machine words.
const NA_LIMB_NARROW: &str =
    "no operand magnitude exceeds the machine-word bound: word arithmetic suffices";
/// Limb NA: the contract forces no arithmetic.
const NA_LIMB_NOT_FORCED: &str =
    "magnitudes may be moved or compared without arithmetic: no limb work is in the contract";
/// Limb NA: id trees have no magnitudes at all.
const NA_LIMB_ID_TREE: &str = "id trees store no magnitudes: there is no arithmetic to meter";
/// Limb NA: the work runs below the shim, in the dependency.
const NA_LIMB_DEPENDENCY: &str = "the decimal conversion runs inside the bignum dependency, \
     below the limb shim: the bench judge's time leg, and its wide-display pair at \
     conversion-dominated widths, judge this row";
/// Heap floor: the result materializes at least its packed bytes.
const WHY_HEAP_MATERIALIZES: &str =
    "materializes a result at least as large as the packed bytes it codes";
/// Heap floor (deterministic-liveness): a forked child copies the version.
const WHY_HEAP_FORK_CHILD: &str = "deterministic-liveness: the forked child carries its own \
     copy of the version's packed bits today; a shared-buffer representation would lower this \
     floor deliberately";
/// Heap floor (deterministic-liveness): a forked party materializes its
/// child half's packed id bits.
const WHY_HEAP_FORK_HALF: &str = "deterministic-liveness: the forked child materializes its \
     own packed id bits today (fork builds both halves, not an in-place edit); a shared-buffer \
     representation would lower this floor deliberately";
/// Heap NA: allocation is not semantically forced.
const NA_HEAP_IN_PLACE: &str = "may compute in place or return word-scale results: allocation \
     is not semantically forced (the process allocator itself cannot be re-routed around)";
/// Scan floor: the tick walk examines its whole input.
const WHY_SCAN_TICK_WALK: &str = "the paired fill walk examines every topology bit and payload \
     code of both operands at least once: 8 bits per input byte, with the measured tick-walk \
     constants 2–5× above";
/// Touch floor (deterministic-liveness): the kernel folds every stored
/// delta code through the metered accumulator.
const WHY_TOUCH_DELTA_FOLD: &str = "deterministic-liveness: the kernel folds each stored delta \
     code of its version operands through the metered accumulator today, at least one digit \
     touch per delta; digit state moving to an unmetered representation lowers this floor \
     deliberately";
/// Touch floor (deterministic-liveness): the rank fold lands every summand
/// in the running accumulator.
const WHY_TOUCH_RANK_SUM: &str = "deterministic-liveness: the fold lands every summand in the \
     running accumulator today, at least one digit touch per summand; digit state moving to an \
     unmetered representation lowers this floor deliberately";
/// Touch NA: id trees carry no digit state at all.
const NA_TOUCH_ID_TREE: &str = "id trees store no magnitudes: there is no digit state to meter";
/// Touch NA: the contract forces no accumulator fold.
const NA_TOUCH_NOT_FORCED: &str = "magnitudes may be moved, hashed, or compared wholesale \
     without a running fold: no accumulator work is in the contract";
/// Touch NA: decoded rank values combine through plain big-integer
/// arithmetic (the limb column's work).
const NA_TOUCH_RANK_ARITHMETIC: &str = "decoded rank values combine through big-integer \
     arithmetic the limb column prices: no accumulator is in the contract";
/// Touch NA: the renderer's summaries are delta-sized values, not a
/// running accumulator.
const NA_TOUCH_RENDER_SUMMARIES: &str = "the renderer derives its printed bases from \
     delta-sized relative summaries without a running accumulator: no digit state is in the \
     contract (the parse direction carries the floor)";
/// Touch NA: the operand streams store no delta codes to fold.
const NA_TOUCH_NO_DELTAS: &str =
    "the operand streams store no delta codes: there is no fold to meter";
/// Touch floor (deterministic-liveness): the validator folds wide stored
/// codes through the accumulator digit by digit.
const WHY_TOUCH_WIDE_STREAM: &str = "deterministic-liveness: the validator's running height \
     folds every stored payload code wider than the machine-word bound through the metered \
     accumulator today, at least one digit touch per 64 code bits (word-scale deltas \
     legitimately batch in the accumulator's lazy zone); digit state moving to an unmetered \
     representation lowers this floor deliberately";
/// Touch NA: every stored code batches in the accumulator's lazy zone.
const NA_TOUCH_LAZY_BATCH: &str = "every stored code fits the machine-word bound: word-scale \
     deltas batch in the accumulator's lazy zone and force no digit touches";
/// Touch NA: a projection may splice owned regions verbatim.
const NA_TOUCH_PROJECTION: &str = "the projection may keep owned regions verbatim and re-base \
     boundaries through plain arithmetic: no accumulator fold is forced";

/// The decode rows' touch floor: one digit touch per 64 bits of every
/// stored code wider than the machine-word bound, or NA when every code
/// is word-scale.
///
/// This is the stream-derived convention the tick rows' limb floor uses:
/// a tree-derived floor would demand fold work no conforming validator
/// does.
fn touch_wide_stream(v: &Version) -> Liveness {
    let limbs = mandatory_limbs_stream(v);
    if limbs == 0 {
        na(NA_TOUCH_LAZY_BATCH)
    } else {
        Liveness::Floor {
            min: limbs,
            why: WHY_TOUCH_WIDE_STREAM,
        }
    }
}
/// Touch NA: a tick against the seed party raises in place.
const NA_TOUCH_SEED_RAISE: &str = "a tick whose party owns the whole tree raises bases in \
     place through plain arithmetic: no delta fold is in the contract";
/// Touch NA: an empty version's tick is pure id-directed growth.
const NA_TOUCH_GROW: &str = "the empty version's tick is id-directed growth: the grow kernel \
     runs no accumulator";

/// Scan floor (rejection rows): the defect sits at the stream's end.
const WHY_SCAN_REJECT_END: &str = "rejection with the defect at the stream's end by \
     construction: a self-delimiting stream's truncation, trailing bits, or non-canonical \
     tail is only discoverable by parsing to it";
/// Scan floor (overlap rejection rows): the witnessing overlap sits at
/// the operands' preorder ends.
const WHY_SCAN_OVERLAP_END: &str = "the pair's one overlapping region sits at both \
     operands' preorder ends by construction, and the packed coding has no random access: \
     any correct rejection scans to it";
/// Heap NA on rejection rows: no result is materialized.
const NA_HEAP_REJECTION: &str = "a rejecting or empty outcome materializes no result, and \
     buffering the fed stream is not semantically forced: allocation stays the \
     implementation's choice";
/// Limb NA on rejection rows: value work may be deferred past the defect.
const NA_LIMB_REJECTION: &str = "rejection forces no value materialization: a strict \
     validator may defer magnitude work past the walk that finds the defect";
/// Touch NA on rejection rows: no accumulator fold is forced.
const NA_TOUCH_REJECTION: &str = "rejection forces no accumulator fold: digit-state work \
     may be deferred past the walk that finds the defect";
/// Scan NA on text-rejection rows: nothing forces packed work before the
/// text defect is found.
const NA_SCAN_TEXT_REJECTION: &str = "rejection of malformed text forces no packed-stream \
     work: the defect may be found in tokenization before any packed validation runs (no \
     deterministic counter watches text-byte consumption; the ceilings judge these cells' \
     live readings and the bench mirror carries their time leg)";

/// The packed-stream rejection rows' floors.
///
/// Scan is floored at one bit per fed byte under `why` (the
/// defect-placement derivation); everything else is honestly
/// not-applicable — rejection materializes no result and forces neither
/// value work nor an accumulator fold.
fn rejection_floors(fed_bytes: usize, why: &'static str) -> Floors {
    Floors {
        heap: na(NA_HEAP_REJECTION),
        limb: na(NA_LIMB_REJECTION),
        segments: seg_ceiling_only(),
        scan: Liveness::Floor {
            min: (fed_bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64,
            why,
        },
        touch: na(NA_TOUCH_REJECTION),
    }
}

/// The id-side rejection rows' floors: as [`rejection_floors`], with the
/// stronger id-tree reasons on the value columns (id trees store no
/// magnitudes at all, rejected or not).
fn id_rejection_floors(fed_bytes: usize, why: &'static str) -> Floors {
    Floors {
        heap: na(NA_HEAP_REJECTION),
        limb: na(NA_LIMB_ID_TREE),
        segments: seg_ceiling_only(),
        scan: Liveness::Floor {
            min: (fed_bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64,
            why,
        },
        touch: na(NA_TOUCH_ID_TREE),
    }
}

/// Scan floor (clock overlap rows): the rejection gate is the party join,
/// so the floor covers the id bytes alone.
const WHY_SCAN_OVERLAP_CLOCK: &str = "the pair's one overlapping region sits at both id \
     operands' preorder ends by construction and the packed coding has no random access, \
     so any correct rejection scans the id streams to it; the version operands ride unread \
     (the party join is the rejection gate), so the floor covers the id bytes alone";

/// The clock overlap rows' floors.
///
/// The scan floor derives from the id bytes alone (the party join is the
/// rejection gate; the version operands are fed but rejection never
/// reads them); everything else is the rejection convention.
fn clock_overlap_floors(id_bytes: usize) -> Floors {
    Floors {
        heap: na(NA_HEAP_REJECTION),
        limb: na(NA_LIMB_REJECTION),
        segments: seg_ceiling_only(),
        scan: Liveness::Floor {
            min: (id_bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64,
            why: WHY_SCAN_OVERLAP_CLOCK,
        },
        touch: na(NA_TOUCH_REJECTION),
    }
}

/// The text-rejection rows' floors: none, by honest derivation.
///
/// No deterministic counter watches text-byte consumption, and a parser
/// may find the defect before any packed or value work; `limb`/`touch`
/// take the caller's operand-specific reason (id trees have no values at
/// all).
fn text_rejection_floors(limb: Liveness, touch: Liveness) -> Floors {
    Floors {
        heap: na(NA_HEAP_REJECTION),
        limb,
        segments: seg_ceiling_only(),
        scan: na(NA_SCAN_TEXT_REJECTION),
        touch,
    }
}

/// A delta-fold touch floor over `deltas` stored delta codes, or NA when
/// the operand streams store none.
fn touch_delta_fold(deltas: u64) -> Liveness {
    if deltas == 0 {
        na(NA_TOUCH_NO_DELTAS)
    } else {
        Liveness::Floor {
            min: deltas,
            why: WHY_TOUCH_DELTA_FOLD,
        }
    }
}

/// The stored delta codes of a version's packed stream: every leaf's
/// payload code after the first (the absolute root height).
///
/// The delta-folding kernels (validate, sweep, emit, the query folds, the
/// text parse) land each of these in the running accumulator, so the count
/// is the touch column's deterministic-liveness floor. Iterative over the
/// packed form, outside any measurement.
fn stored_deltas(v: &Version) -> u64 {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut leaves = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (_, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        leaves += 1;
    }
    leaves.saturating_sub(1)
}

/// A full-examination scan floor over `packed_bytes` of operand.
fn scan_examines(packed_bytes: usize) -> Liveness {
    Liveness::Floor {
        min: (packed_bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64,
        why: WHY_SCAN_EXAMINES,
    }
}

/// The early-exit scan floor: the root codes are always read.
fn scan_touch() -> Liveness {
    Liveness::Floor {
        min: SCAN_TOUCH_FLOOR_BITS,
        why: WHY_SCAN_TOUCH,
    }
}

/// A stored-stream limb floor (one limb per 64 bits of every wide payload
/// code), or NA when every code fits machine words.
///
/// The honest floor for rows that read the stored form as-is (decode, the
/// query folds, the tick walk), which provably need not materialize the
/// decoded tree's absolute values.
fn limb_stream(mandatory_limbs: u64) -> Liveness {
    if mandatory_limbs == 0 {
        Liveness::NotApplicable {
            reason: NA_LIMB_NARROW,
        }
    } else {
        Liveness::Floor {
            min: mandatory_limbs,
            why: WHY_LIMB_STREAM,
        }
    }
}

/// A wide-magnitude limb floor over the decoded tree's stored bases, or NA
/// when every base fits machine words: the honest floor for the
/// value-materializing parse rows alone.
fn limb_wide(mandatory_limbs: u64) -> Liveness {
    if mandatory_limbs == 0 {
        Liveness::NotApplicable {
            reason: NA_LIMB_NARROW,
        }
    } else {
        Liveness::Floor {
            min: mandatory_limbs,
            why: WHY_LIMB_WIDE,
        }
    }
}

/// A materialization heap floor over `packed_bytes`.
fn heap_materializes(packed_bytes: usize) -> Liveness {
    Liveness::Floor {
        min: packed_bytes as u64,
        why: WHY_HEAP_MATERIALIZES,
    }
}

/// The fork rows' heap declaration: the child clock's version copy floors
/// the heap at the version's whole stored bytes, or NA when the version is
/// word-scale (the id-pair cross forks around an empty version).
fn heap_fork_child(version: &Version) -> Liveness {
    let stored_bytes = (version.encoded_bits() / 8) as u64;
    if stored_bytes == 0 {
        na(NA_HEAP_IN_PLACE)
    } else {
        Liveness::Floor {
            min: stored_bytes,
            why: WHY_HEAP_FORK_CHILD,
        }
    }
}

/// Shorthand for a not-applicable declaration.
fn na(reason: &'static str) -> Liveness {
    Liveness::NotApplicable { reason }
}

/// Segments NA: the policy declaration every cell carries on the segments
/// currency.
const NA_SEG_CEILING_ONLY: &str = "ceiling-only by policy: the target is walks that never grow \
     the stack, so the honest floor is zero and a zero floor asserts nothing";

/// The segments currency's declaration: ceiling-only by policy, on every
/// cell.
fn seg_ceiling_only() -> Liveness {
    na(NA_SEG_CEILING_ONLY)
}

/// The floors of the many rows that must walk their operands but are forced
/// into neither allocation nor arithmetic: scan floored, heap and limb NA.
///
/// The touch declaration is the caller's: each walk row answers the
/// accumulator question for its own kernel.
fn walk_floors(packed_bytes: usize, touch: Liveness) -> Floors {
    Floors {
        heap: na(NA_HEAP_IN_PLACE),
        limb: na(NA_LIMB_NOT_FORCED),
        segments: seg_ceiling_only(),
        scan: scan_examines(packed_bytes),
        touch,
    }
}

/// Touch NA on comparison rows whose operands are concurrent: no
/// delta-fold count is forced when one witness divergence per direction
/// decides the answer.
const NA_TOUCH_CONCURRENT_OPERANDS: &str = "the operands are concurrent, so the comparison \
     may decide at one witness divergence per direction: no delta-fold count is forced";

/// The comparison rows' floors, derived from the operands themselves
/// (outside any measurement).
///
/// A comparable pair must be walked to the end — certifying dominance
/// means checking every region — so the full-examination scan floor and
/// the every-stored-delta touch floor bind; a concurrent pair may be
/// decided at one witness divergence per direction, so only the
/// root-codes scan floor does.
fn comparison_floors(v: &Version, w: &Version, packed_bytes: usize) -> Floors {
    if v.partial_cmp(w).is_some() {
        walk_floors(
            packed_bytes,
            touch_delta_fold(stored_deltas(v) + stored_deltas(w)),
        )
    } else {
        Floors {
            heap: na(NA_HEAP_IN_PLACE),
            limb: na(NA_LIMB_NOT_FORCED),
            segments: seg_ceiling_only(),
            scan: scan_touch(),
            touch: na(NA_TOUCH_CONCURRENT_OPERANDS),
        }
    }
}

/// The fused projected-comparison rows' floors, from the verdict the cell
/// will produce (computed at prepare, outside measurement).
///
/// A comparable projected pair must certify dominance over every region,
/// so the walk consumes both event streams whole: full-examination scan
/// and one accumulator touch per stored event delta (the id streams store
/// no deltas). A concurrent pair may exit at its witnessing divergences,
/// so only the root-code scan floor binds.
fn masked_cmp_floors(
    verdict: &Option<Ordering>,
    v: &Version,
    w: &Version,
    packed_bytes: usize,
) -> Floors {
    if verdict.is_some() {
        walk_floors(
            packed_bytes,
            touch_delta_fold(stored_deltas(v) + stored_deltas(w)),
        )
    } else {
        Floors {
            heap: na(NA_HEAP_IN_PLACE),
            limb: na(NA_LIMB_NOT_FORCED),
            segments: seg_ceiling_only(),
            scan: scan_touch(),
            touch: na(NA_TOUCH_CONCURRENT_OPERANDS),
        }
    }
}

/// The tick-cross rows' floors: full-examination scan, per-stored-code
/// limb, in-place heap.
///
/// The paired fill walk examines every bit of both packed operands (a
/// full-examination scan floor, 8 bits per byte — the measured
/// tick-walk constants sit 2–5× above it), and every wide payload code
/// of the version's own stored stream must be decoded limb by limb
/// (the mandatory limb floor; NA on the word-scale families). The limb
/// floor derives from the stream's codes, not the decoded tree's
/// min-lifted bases: a plateau of equal wide leaves stores its width
/// once and steps by unit deltas after, and the walk provably need not
/// materialize each leaf's absolute value — a tree-derived floor would
/// demand limb work no conforming walk does.
fn tick_walk_floors(version: &Version, packed_bytes: usize) -> Floors {
    Floors {
        heap: na(NA_HEAP_IN_PLACE),
        limb: limb_stream(mandatory_limbs_stream(version)),
        segments: seg_ceiling_only(),
        scan: Liveness::Floor {
            min: (packed_bytes as u64).saturating_mul(TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE),
            why: WHY_SCAN_TICK_WALK,
        },
        touch: touch_delta_fold(stored_deltas(version)),
    }
}

/// The mandatory limb count of a version's stored stream: one limb per
/// 64 bits of every payload code wider than
/// [`MACHINE_WORD_MAGNITUDE_BITS`].
///
/// A walk over the stream must decode each stored code to fold it, and
/// decoding a wide code cannot touch fewer limbs than the code has;
/// narrower codes may legitimately live in machine words and count
/// zero. Unlike [`mandatory_limbs_version`], this counts the stream's
/// own delta codes, never the decoded tree's absolute values: it is
/// the honest floor for operations that read the stored form as-is.
/// Iterative over the packed form, outside any measurement.
fn mandatory_limbs_stream(v: &Version) -> u64 {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut limbs = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let width = code.bits();
        if width > MACHINE_WORD_MAGNITUDE_BITS {
            limbs += width.div_ceil(64);
        }
    }
    limbs
}

/// A version's value content in bytes: the summed bit widths of its
/// absolute leaf heights (one bit minimum per leaf), rounded to bytes.
///
/// This is the content that delta coding lets ride behind
/// asymptotically fewer wire bits, and the scaling denominator of
/// the flat-denominator shape's exponent fits: the boundary comb at fixed
/// tooth magnitude doubles its value content (and every operation's honest
/// per-tooth work) per level while its packed bytes grow only by the unit
/// delta codes over a fixed wide intercept. Iterative over the packed
/// form, outside any measurement.
fn value_content_bytes(v: &Version) -> usize {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut last: Option<Base> = None;
    let mut content = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let value = match last {
            None => code,
            Some(prev) => {
                let odd = code.bit(0);
                let magnitude = if odd {
                    (code + 1u32) >> 1u32
                } else {
                    code >> 1u32
                };
                if odd {
                    prev - &magnitude
                } else {
                    prev + &magnitude
                }
            }
        };
        content += value.bits().max(1);
        last = Some(value);
    }
    (content.div_ceil(8)) as usize
}

/// The mandatory limb count of a version's stored magnitudes: one limb per
/// 64 bits of every base wider than [`MACHINE_WORD_MAGNITUDE_BITS`].
///
/// Materializing or folding such a value cannot touch fewer limbs than the
/// value has, whatever the representation; narrower values may legitimately
/// live in machine words and count zero. This is the floor for the
/// value-materializing parse rows alone (`FromStr` converts every spelled
/// base); rows that read the stored form as-is floor at
/// [`mandatory_limbs_stream`], whose derivation explains the split. The
/// walk mirrors [`radix_units_version`]: iterative over the packed form,
/// outside any measurement.
fn mandatory_limbs_version(v: &Version) -> u64 {
    let mut limbs = 0u64;
    for base in stored_bases(v) {
        let width = base.bits();
        if width > MACHINE_WORD_MAGNITUDE_BITS {
            limbs += width.div_ceil(64);
        }
    }
    limbs
}

/// The min-lifted stored bases of a version's canonical event tree, in
/// preorder: the values the paper notation renders and any base-per-node
/// representation must hold.
///
/// Reconstructed from the stored skyline stream in three linear passes
/// (absolute leaf heights, bottom-up subtree floors, per-node relative
/// bases), entirely outside any measurement.
fn stored_bases(v: &Version) -> Vec<Base> {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    // Pass 1: topology flags and absolute leaf heights.
    let mut pos = 0usize;
    let mut topology: Vec<bool> = Vec::new();
    let mut heights: Vec<Base> = Vec::new();
    let mut pending = 1usize;
    while pending > 0 {
        pending -= 1;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        topology.push(internal);
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let value = match heights.last() {
            None => code,
            Some(prev) => {
                let odd = code.bit(0);
                let magnitude = if odd {
                    (code + 1u32) >> 1u32
                } else {
                    code >> 1u32
                };
                if odd {
                    prev.clone() - &magnitude
                } else {
                    prev + &magnitude
                }
            }
        };
        heights.push(value);
    }
    // Pass 2: per-node floors (minimum leaf height in the subtree),
    // bottom-up over the preorder topology.
    let nodes = topology.len();
    let mut floors: Vec<Base> = vec![Base::ZERO; nodes];
    let mut open: Vec<(usize, Option<Base>)> = Vec::new();
    let mut next_leaf = 0usize;
    for (index, &internal) in topology.iter().enumerate() {
        if internal {
            open.push((index, None));
            continue;
        }
        floors[index] = heights[next_leaf].clone();
        next_leaf += 1;
        let mut summary = floors[index].clone();
        loop {
            match open.pop() {
                None => break,
                Some((parent, None)) => {
                    open.push((parent, Some(summary)));
                    break;
                }
                Some((parent, Some(left))) => {
                    let floor = if left <= summary { left } else { summary };
                    floors[parent] = floor.clone();
                    summary = floor;
                }
            }
        }
    }
    // Pass 3: each node's stored base is its floor minus its parent's.
    let mut bases = Vec::with_capacity(nodes);
    let mut parent_floors: Vec<Base> = vec![Base::ZERO];
    for (index, &internal) in topology.iter().enumerate() {
        let parent = parent_floors
            .pop()
            .expect("preorder supplies one inherited floor per node");
        bases.push(floors[index].clone() - &parent);
        if internal {
            parent_floors.push(floors[index].clone());
            parent_floors.push(floors[index].clone());
        }
    }
    bases
}

// ─── operations ─────────────────────────────────────────────────────────────

/// One prepared cell run: the operand bytes it charges against, the
/// denomination rule, and the body to measure.
///
/// `prepare` builds (and decodes) operands outside measurement; the body's
/// result is boxed and kept alive until the meters are read, so peak heap
/// includes the fully materialized output.
struct Cell {
    /// The packed (or, on `FromStr` rows, text) operand bytes.
    input_bytes: usize,
    /// How the meters are denominated (the module doc's criterion).
    denom: Denom,
    /// The cell's liveness declarations, one per floored column.
    floors: Floors,
    /// The fold rows' operand count at this scale: `Some` on the two
    /// n-ary fold rows only, where it drives the declared `FoldLog`
    /// model (the declared-models section above).
    fold_arity: Option<u64>,
    /// The party fold's declared search allowance at this scale, in
    /// scan bits ([`INDEX_PROBE_SCAN_BITS`]'s derivation).
    ///
    /// Added to the declared scan ceiling; zero on the version fold (no
    /// overlap test) and wherever the operands carry no both-present
    /// structure.
    fold_search_bits: u64,
    /// Whether the heap column is judged against the ratified
    /// capacity-chain model ([`capacity_chain_peak`]) instead of the
    /// flat ceiling: the output-dominated projection on the
    /// comb-scatter cross only.
    capacity_model: bool,
    /// The measured body; its result stays alive until the meters are read.
    #[allow(clippy::type_complexity)]
    body: Box<dyn FnOnce() -> Box<dyn Any>>,
}

/// A cell's denomination rule (see the module doc's list of which rows get
/// which).
enum Denom {
    /// Input bytes alone: the default, and the only rule most rows may use.
    Input,
    /// Total I/O bytes: input plus the actual output, read back from the
    /// measured result after the meters are captured.
    Io(IoSpec),
}

/// The I/O-denomination data for a mandatory-output cell.
struct IoSpec {
    /// Read the actual output's byte size from the boxed result.
    output_bytes: fn(&dyn Any) -> usize,
    /// The text rows' extra terms; `None` for packed-output cells.
    text: Option<TextSpec>,
}

/// The text rows' radix-work term and output-honesty data.
struct TextSpec {
    /// `Σ digitsᵢ × limbsᵢ` over the values the text spells.
    ///
    /// The limb column is judged against `R = n_io +` this `+` the
    /// pipeline term below, at the κ ceiling; the output-honesty ceiling
    /// is asserted against these units alone (the pipeline term must not
    /// loosen it).
    radix_units: u64,
    /// The spelled event values, each granting
    /// [`TEXT_PIPELINE_LIMB_OPS_PER_VALUE`] radix units in `R`.
    ///
    /// Zero on id-only text (boolean tokens force no arithmetic), and
    /// the version side's node count on clock rows for the same reason.
    spelled_values: u64,
    /// Whether the measured *output* is the text side (`Display`); the
    /// honesty assertion then runs against the actual output bytes.
    /// `FromStr`'s text is input and is asserted at prepare.
    output_is_text: bool,
}

impl Cell {
    /// Package an input-denominated body with its operand byte count and its
    /// liveness declarations.
    fn new<R: Any>(input_bytes: usize, floors: Floors, body: impl FnOnce() -> R + 'static) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Input,
            floors,
            fold_arity: None,
            fold_search_bits: 0,
            capacity_model: false,
            body: Box::new(move || Box::new(body())),
        }
    }

    /// Declare this cell's readings judged under the fold rows' `FoldLog`
    /// model at operand count `arity` (the declared-models section).
    fn with_fold_arity(mut self, arity: u64) -> Cell {
        self.fold_arity = Some(arity);
        self
    }

    /// Declare the party fold's search allowance in scan bits
    /// ([`INDEX_PROBE_SCAN_BITS`]'s derivation).
    fn with_fold_search(mut self, bits: u64) -> Cell {
        self.fold_search_bits = bits;
        self
    }

    /// Declare this cell's heap judged against the ratified
    /// capacity-chain model (the declared-models section).
    fn with_capacity_model(mut self) -> Cell {
        self.capacity_model = true;
        self
    }

    /// Package an I/O-denominated packed-output body: the output side of
    /// `n_io` is read back from the actual result.
    fn io<R: Any>(
        input_bytes: usize,
        floors: Floors,
        output_bytes: fn(&dyn Any) -> usize,
        body: impl FnOnce() -> R + 'static,
    ) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Io(IoSpec {
                output_bytes,
                text: None,
            }),
            floors,
            fold_arity: None,
            fold_search_bits: 0,
            capacity_model: false,
            body: Box::new(move || Box::new(body())),
        }
    }

    /// Package a text-row body: I/O-denominated, with the limb column judged
    /// against the radix-work denominator.
    fn text<R: Any>(
        input_bytes: usize,
        floors: Floors,
        output_bytes: fn(&dyn Any) -> usize,
        spec: TextSpec,
        body: impl FnOnce() -> R + 'static,
    ) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Io(IoSpec {
                output_bytes,
                text: Some(spec),
            }),
            floors,
            fold_arity: None,
            fold_search_bits: 0,
            capacity_model: false,
            body: Box::new(move || Box::new(body())),
        }
    }
}

/// Assert the output-honesty ceiling on one text stream.
///
/// Every text stream entering a denominator passes through here: text bytes
/// at most [`TEXT_BYTES_PER_RADIX_UNIT`] per radix unit of the values the
/// text spells, so padding the text side of `n_io` trips the run instead
/// of greening a cell.
fn assert_honest_text(what: &'static str, text_bytes: usize, radix_units: u64) {
    assert!(
        text_bytes as f64 <= TEXT_BYTES_PER_RADIX_UNIT * radix_units as f64,
        "output honesty: {what}: {text_bytes} text bytes exceed \
         {TEXT_BYTES_PER_RADIX_UNIT} per radix unit over {radix_units} units"
    );
}

/// `Σ digits × limbs` over the decimal values an event tree's text spells:
/// every node's stored base, exactly what `Display` renders and `FromStr`
/// parses.
///
/// `digits` is the value's exact decimal length; `limbs` its 64-bit limb
/// count (at least 1, so single-digit zeros still cost a unit). The walk is
/// iterative over the packed form; only the per-value `digits × limbs`
/// products enter the denominator, so the term prices schoolbook conversion
/// work without assuming any converter.
fn radix_units_version(v: &Version) -> u64 {
    let mut units = 0u64;
    for base in stored_bases(v) {
        let digits = base.to_string().len() as u64;
        let limbs = base.bits().div_ceil(64).max(1);
        units += digits * limbs;
    }
    units
}

/// `Σ digits × limbs` over an id tree's text: one unit per rendered `0`/`1`
/// token (terminals and absent children), each a single digit of a
/// single-limb value.
fn radix_units_party(p: &Party) -> u64 {
    let bits = p.as_bits();
    if bits.is_empty() {
        return 1; // the empty id renders one `0` token
    }
    let mut pos = 0usize;
    let mut pending = 1u64;
    let mut units = 0u64;
    while pending > 0 {
        pending -= 1;
        let left = bits[pos];
        let right = bits[pos + 1];
        pos += 2;
        if !left && !right {
            units += 1; // a terminal renders `1`
            continue;
        }
        for present in [left, right] {
            if present {
                pending += 1;
            } else {
                units += 1; // an absent child renders `0`
            }
        }
    }
    units
}

/// `Σ digits × limbs` over a clock's text: its party's and version's terms.
fn radix_units_clock(c: &Clock) -> u64 {
    radix_units_party(c.party()) + radix_units_version(c.version())
}

/// The packed byte size of a version produced by a measured body.
fn version_output_bytes(v: &Version) -> usize {
    v.encoded_bits().div_ceil(8)
}

/// One board row: a public operation and how to instantiate it per family.
struct Op {
    /// The row label, `type_operation`.
    name: &'static str,
    /// The signature group the row belongs to on the operation axis.
    group: OpGroup,
    /// Build the cell for one shape, or `None` where the shape's bundle
    /// supplies no operand for this operation's signature.
    prepare: fn(&FamilyData) -> Option<Cell>,
}

/// The operation axis's signature groups.
///
/// A group names the operand signature a row consumes; the bench mirror's
/// pinned subset pairs each shape with the groups it was designed to
/// stress ([`designed`]), so the subset is a rule over the same two axes
/// the board's product runs on, never a second cell list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpGroup {
    /// Rows over a shape's version operands (codec, comparison, merge,
    /// text, hash rows).
    Version,
    /// The linear-functional query rows: `rank`, `distance`, `lag`,
    /// `min_ticks`.
    Measure,
    /// The rank-value rows: `rank_pair_ops`, `rank_sum`.
    Rank,
    /// The tick rows, driven through a cross shape's designated pairing.
    Tick,
    /// The projection rows: the explicit materializations and the fused
    /// lazy comparisons.
    ///
    /// `own_version_to_version` and `clock_own_version_to_version` price
    /// the explicit materialization; `own_version_cmp` and
    /// `own_version_pair_cmp` price the fused comparisons, which stay
    /// input-denominated on every shape — a comparison never
    /// materializes the projection.
    Projection,
    /// The fold rows: `version_join_all`, `party_join_all`.
    Fold,
    /// Rows over a shape's disjoint party pair.
    Party,
    /// Rows over a shape's clock (the tick and projection clock rows
    /// carry their own groups above).
    Clock,
}

/// The shape × operation-group pairings each shape was designed to
/// stress: the bench mirror's diagonal.
///
/// Declared per shape, on the shape axis, so the pinned bench subset is
/// derived — a shape added to the axis must answer which groups it was
/// built against (the exhaustive match), and the subset follows. The
/// deterministic board itself never consults this: it runs the whole
/// product.
fn designed(kind: FamilyKind, group: OpGroup) -> bool {
    match kind {
        // The original full-surface adversaries and the organic control,
        // plus the two population shapes (whose bundles already narrow
        // them to their party/fold rows).
        FamilyKind::Dense | FamilyKind::Benign | FamilyKind::IdPair | FamilyKind::Scatter => true,
        // The magnitude shapes predate the rank rows' mismatch pair and
        // were never its designed adversary.
        FamilyKind::Bigroot | FamilyKind::Hugeleaf | FamilyKind::Cliff => group != OpGroup::Rank,
        // The rank fold's wide-numerator adversary.
        FamilyKind::Harmonic => matches!(group, OpGroup::Measure | OpGroup::Rank),
        // The output-domination cross.
        FamilyKind::CombScatter => group == OpGroup::Projection,
        // The correlated fold population, built against the fold rows.
        FamilyKind::Weave => group == OpGroup::Fold,
        // The tick-walk crosses.
        FamilyKind::NestedFull
        | FamilyKind::NestedWide
        | FamilyKind::MirrorWide
        | FamilyKind::MirrorNarrow
        | FamilyKind::Staircase
        | FamilyKind::RevealComb
        | FamilyKind::RevealHifloor
        | FamilyKind::PureComb
        | FamilyKind::AscendCliff
        | FamilyKind::AscendPlateau => group == OpGroup::Tick,
        // The query-fold adversaries, built against the
        // linear-functional rows: wide difference crests over a
        // dense-position spine, the many-freezes spine, the
        // many-armings spine, and the switch-density population.
        FamilyKind::JumpPair
        | FamilyKind::FreezePos
        | FamilyKind::PromoRearm
        | FamilyKind::ConcurrentPair => group == OpGroup::Measure,
    }
}

/// The operation table: every public operation with a meaningful packed
/// operand (the module doc lists the rest).
#[allow(clippy::too_many_lines)]
fn ops() -> Vec<Op> {
    vec![
        // ── Version ────────────────────────────────────────────────────
        Op {
            name: "version_decode",
            group: OpGroup::Version,
            prepare: |f| {
                let bytes = f.version.clone()?;
                let v = decode_version(&bytes);
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(bytes.len()),
                    touch: touch_wide_stream(&v),
                };
                Some(Cell::new(bytes.len(), floors, move || {
                    decode_version(&bytes)
                }))
            },
        },
        Op {
            name: "version_encode",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (v.encode(), v)))
            },
        },
        Op {
            name: "version_cmp",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = comparison_floors(&v, &w, n);
                Some(Cell::new(n, floors, move || {
                    let ord: Option<Ordering> = v.partial_cmp(&w);
                    (ord, v, w)
                }))
            },
        },
        Op {
            name: "version_eq",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_EQ_BYTES),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (v == w, v, w)))
            },
        },
        Op {
            name: "version_concurrent",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = comparison_floors(&v, &w, n);
                Some(Cell::new(n, floors, move || (v.concurrent(&w), v, w)))
            },
        },
        Op {
            name: "version_join",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let touch = touch_delta_fold(stored_deltas(&v) + stored_deltas(&w));
                Some(Cell::new(n, walk_floors(n, touch), move || (&v | &w, v, w)))
            },
        },
        Op {
            name: "version_join_assign",
            group: OpGroup::Version,
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                let touch = touch_delta_fold(stored_deltas(&v) + stored_deltas(&w));
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    v |= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_meet",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let touch = touch_delta_fold(stored_deltas(&v) + stored_deltas(&w));
                Some(Cell::new(n, walk_floors(n, touch), move || (&v & &w, v, w)))
            },
        },
        Op {
            name: "version_meet_assign",
            group: OpGroup::Version,
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                let touch = touch_delta_fold(stored_deltas(&v) + stored_deltas(&w));
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    v &= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_tick",
            group: OpGroup::Tick,
            prepare: |f| {
                // The tick-walk families carry their own (event, id)
                // pair; every other family ticks its version with the
                // seed.
                if let Some((mut v, party, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    return Some(Cell::new(n, floors, move || {
                        v.tick(&party);
                        (v, party)
                    }));
                }
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(
                    n + 1,
                    walk_floors(n, na(NA_TOUCH_SEED_RAISE)),
                    move || {
                        v.tick(&party);
                        v
                    },
                ))
            },
        },
        Op {
            name: "version_ticks",
            group: OpGroup::Tick,
            prepare: |f| {
                // The fused multi-tick at a fixed count: the same walk
                // and splice as the tick rows, with the count's gamma
                // width the only n-dependence — so the cell must scale
                // exactly as the tick cell above it (the flatness rows
                // of tests/meter.rs pin the n axis; this cell pins the
                // input axis).
                if let Some((mut v, party, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    return Some(Cell::new(n, floors, move || {
                        v.ticks(&party, TICKS_BOARD_COUNT);
                        (v, party)
                    }));
                }
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(
                    n + 1,
                    walk_floors(n, na(NA_TOUCH_SEED_RAISE)),
                    move || {
                        v.ticks(&party, TICKS_BOARD_COUNT);
                        v
                    },
                ))
            },
        },
        Op {
            name: "version_tick_adv_party",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                Some(Cell::new(
                    n + 1,
                    walk_floors(n, na(NA_TOUCH_GROW)),
                    move || {
                        v.tick(&a);
                        (v, a)
                    },
                ))
            },
        },
        Op {
            name: "version_rank",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_deltas(&v)),
                };
                Some(Cell::new(n, floors, move || (v.rank(), v)))
            },
        },
        Op {
            name: "rank_pair_ops",
            group: OpGroup::Rank,
            prepare: |f| {
                // The mismatched pair: a family-derived rank (maximal
                // exponent on the spines) against a small integer rank, on
                // the spine families that maximize the mismatch plus the
                // benign control. Ranks are built at family construction,
                // outside measurement; the denominator is the pair's value
                // content (the module doc's rank denomination).
                let (a, b) = f.rank_pair.clone()?;
                let n = (a.content_bits() + b.content_bits()).div_ceil(8) as usize;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: Liveness::Floor {
                        min: a.content_bits().max(b.content_bits()).div_ceil(64),
                        why: WHY_LIMB_RANK_PAIR,
                    },
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_NO_STREAM),
                    touch: na(NA_TOUCH_RANK_ARITHMETIC),
                };
                Some(Cell::new(n, floors, move || {
                    let ord = a.cmp(&b);
                    // One direction of the pair dominates; keep whichever
                    // difference exists so the subtraction always runs.
                    let diff = a.checked_sub(&b).or_else(|| b.checked_sub(&a));
                    let sum = &a + &b;
                    (ord, diff, sum, a, b)
                }))
            },
        },
        Op {
            name: "rank_sum",
            group: OpGroup::Rank,
            prepare: |f| {
                // The mixed fold: the family-derived rank (maximal exponent
                // on the spines) summed high-first with one small integer
                // rank per packed byte of the family's measure operand, so
                // both sides of the value content scale together. High-first
                // is the adversarial order of record: `Sum` accepts arbitrary
                // order, and under a fold that re-normalizes per element it
                // is the order that makes every later add a full-width
                // operation. The denominator is the summands' total value
                // content (the module doc's rank denomination).
                let (a, _) = f.rank_pair.clone()?;
                let (_, k) = f.version()?;
                let ones: Vec<Rank> = (0..k)
                    .map(|i| {
                        Version::try_from(i as u64 % 7 + 1)
                            .expect("a small integer version is valid")
                            .rank()
                    })
                    .collect();
                let n = (a.content_bits().div_ceil(8) as usize)
                    + ones
                        .iter()
                        .map(|r| r.content_bits().div_ceil(8) as usize)
                        .sum::<usize>();
                let wide = a.content_bits();
                let limb = if wide > MACHINE_WORD_MAGNITUDE_BITS {
                    Liveness::Floor {
                        min: wide.div_ceil(64),
                        why: WHY_LIMB_RANK_SUM,
                    }
                } else {
                    na(NA_LIMB_NARROW)
                };
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb,
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_NO_STREAM),
                    touch: Liveness::Floor {
                        min: ones.len() as u64 + 1,
                        why: WHY_TOUCH_RANK_SUM,
                    },
                };
                Some(Cell::new(n, floors, move || {
                    std::iter::once(a).chain(ones).sum::<Rank>()
                }))
            },
        },
        Op {
            name: "version_distance",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v) + mandatory_limbs_stream(&w)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_deltas(&v) + stored_deltas(&w)),
                };
                Some(Cell::new(n, floors, move || (v.distance(&w), v, w)))
            },
        },
        Op {
            name: "version_lag",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v) + mandatory_limbs_stream(&w)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_deltas(&v) + stored_deltas(&w)),
                };
                Some(Cell::new(n, floors, move || (v.lag(&w), v, w)))
            },
        },
        Op {
            name: "version_min_ticks",
            group: OpGroup::Measure,
            prepare: |f| {
                // The exact fold walks the whole stream, decodes every
                // stored code, and folds heights and minima on
                // accumulators: the rank fold's floor spec exactly.
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_deltas(&v)),
                };
                Some(Cell::new(n, floors, move || (v.min_ticks(), v)))
            },
        },
        Op {
            name: "version_join_all",
            group: OpGroup::Fold,
            prepare: |f| {
                let (versions, _) = f.fold.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let versions: Vec<Version> = versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                let touch = touch_delta_fold(versions.iter().map(stored_deltas).sum());
                Some(
                    Cell::new(n, walk_floors(n, touch), move || {
                        Version::join_all(versions)
                    })
                    .with_fold_arity(arity),
                )
            },
        },
        Op {
            name: "own_version_to_version",
            group: OpGroup::Projection,
            prepare: |f| {
                // The explicit materialization `(&v / &p).to_version()`:
                // the one projection spelling that pays the product-growth
                // output. Adversarial × adversarial with mandatory
                // dominating output: the declared output-domination cross,
                // I/O-denominated.
                if f.output_dominated {
                    let (v_bytes, p_bytes) = f.cross.as_ref()?;
                    let n = v_bytes.len() + p_bytes.len();
                    let v = decode_version(v_bytes);
                    let p = decode_party(p_bytes);
                    let cell = Cell::io(
                        n,
                        walk_floors(n, na(NA_TOUCH_PROJECTION)),
                        |r| {
                            let (out, _, _) = r
                                .downcast_ref::<(Version, Version, Party)>()
                                .expect("the cross projection body yields (out, v, p)");
                            version_output_bytes(out)
                        },
                        move || ((&v / &p).to_version(), v, p),
                    );
                    // The comb-scatter cross's builder runs the ratified
                    // capacity chain (the declared-models section); the
                    // plateau-comb crosses stay flat-judged and green.
                    return Some(if matches!(f.kind, FamilyKind::CombScatter) {
                        cell.with_capacity_model()
                    } else {
                        cell
                    });
                }
                // A cross shape without output domination materializes its
                // event side through its id side, input-denominated (the
                // module doc's do-not-re-denominate list).
                if let Some((v, p, n)) = f.cross() {
                    return Some(Cell::new(
                        n,
                        walk_floors(n, na(NA_TOUCH_PROJECTION)),
                        move || ((&v / &p).to_version(), v, p),
                    ));
                }
                // Small (half-interval) party × adversarial version.
                if f.version.is_some() {
                    let (v, n) = f.version()?;
                    let half = Party::seed().fork();
                    return Some(Cell::new(
                        n + 1,
                        walk_floors(n, na(NA_TOUCH_PROJECTION)),
                        move || ((&v / &half).to_version(), v, half),
                    ));
                }
                // Adversarial party × small version.
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                v.tick(&a);
                let input = n + v.encode().len();
                Some(Cell::new(
                    input,
                    walk_floors(input, na(NA_TOUCH_PROJECTION)),
                    move || ((&v / &a).to_version(), v, a),
                ))
            },
        },
        Op {
            name: "own_version_cmp",
            group: OpGroup::Projection,
            prepare: |f| {
                // The fused three-stream comparison `(v / p) ⋚ w`: lazy at
                // every spelling, so the cell stays input-denominated on
                // every shape — the output-domination crosses included,
                // which is the point: comparing a projection never pays
                // its materialization.
                let (v, p, w, n) = if let Some((v, p, np)) = f.cross() {
                    let (_, w, _) = f.version_pair()?;
                    let nw = f.version2.as_ref()?.len();
                    (v, p, w, np + nw)
                } else if f.version.is_some() {
                    // Half-interval party × the shape's version pair.
                    let (v, w, n) = f.version_pair()?;
                    (v, Party::seed().fork(), w, n + 1)
                } else {
                    // Adversarial party × small versions ticked on it.
                    let (a, _, _) = f.party_pair()?;
                    let np = f.parties.as_ref().map(|(a, _)| a.len())?;
                    let mut v = Version::new();
                    v.tick(&a);
                    let mut w = v.clone();
                    w.tick(&Party::seed());
                    let n = np + v.encode().len() + w.encode().len();
                    (v, a, w, n)
                };
                let floors = masked_cmp_floors(&(&v / &p).partial_cmp(&w), &v, &w, n);
                Some(Cell::new(n, floors, move || {
                    let ord = (&v / &p).partial_cmp(&w);
                    (ord, v, p, w)
                }))
            },
        },
        Op {
            name: "own_version_pair_cmp",
            group: OpGroup::Projection,
            prepare: |f| {
                // The fused four-stream comparison `(v/a) ⋚ (w/b)`:
                // input-denominated everywhere, as the three-stream row.
                let (v, a, w, b, n) = match (f.parties.is_some(), f.version.is_some()) {
                    (true, true) => {
                        let (a, b, np) = f.party_pair()?;
                        let (v, w, nv) = f.version_pair()?;
                        (v, a, w, b, np + nv)
                    }
                    (false, true) => {
                        // Seed fork halves around the shape's version pair.
                        let (v, w, nv) = f.version_pair()?;
                        let mut a = Party::seed();
                        let b = a.fork();
                        (v, a, w, b, nv + 2)
                    }
                    (true, false) => {
                        // The party pair's own single-tick histories.
                        let (a, b, np) = f.party_pair()?;
                        let mut v = Version::new();
                        v.tick(&a);
                        let mut w = Version::new();
                        w.tick(&b);
                        let n = np + v.encode().len() + w.encode().len();
                        (v, a, w, b, n)
                    }
                    (false, false) => return None,
                };
                let floors = masked_cmp_floors(&(&v / &a).partial_cmp(&(&w / &b)), &v, &w, n);
                Some(Cell::new(n, floors, move || {
                    let ord = (&v / &a).partial_cmp(&(&w / &b));
                    (ord, v, a, w, b)
                }))
            },
        },
        Op {
            name: "version_display",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let spec = TextSpec {
                    radix_units: radix_units_version(&v),
                    spelled_values: stored_bases(&v).len() as u64,
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_DEPENDENCY),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_RENDER_SUMMARIES),
                };
                Some(Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Version)>()
                            .expect("the display body yields (text, v)")
                            .0
                            .len()
                    },
                    spec,
                    move || (v.to_string(), v),
                ))
            },
        },
        Op {
            name: "version_from_str",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let s = v.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_version(&v),
                    spelled_values: stored_bases(&v).len() as u64,
                    output_is_text: false,
                };
                assert_honest_text("version_from_str input", s.len(), spec.radix_units);
                let packed = version_output_bytes(&v);
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: limb_wide(mandatory_limbs_version(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                    touch: touch_delta_fold(stored_deltas(&v)),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        version_output_bytes(
                            r.downcast_ref::<Version>()
                                .expect("the parse body yields a version"),
                        )
                    },
                    spec,
                    move || {
                        s.parse::<Version>()
                            .expect("a displayed version parses back")
                    },
                ))
            },
        },
        Op {
            name: "version_hash",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    v.hash(&mut hasher);
                    (hasher.finish(), v)
                }))
            },
        },
        Op {
            name: "causally_contains",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = comparison_floors(&v, &w, n);
                Some(Cell::new(n, floors, move || {
                    let hit = causally::since(&v).contains(&w);
                    (hit, v, w)
                }))
            },
        },
        // ── Party ──────────────────────────────────────────────────────
        Op {
            name: "party_decode",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b) = f.parties.clone()?;
                let n = a.len() + b.len();
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || {
                    (decode_party(&a), decode_party(&b))
                }))
            },
        },
        Op {
            name: "party_encode",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.encode(), a)))
            },
        },
        Op {
            name: "party_fork",
            group: OpGroup::Party,
            prepare: |f| {
                let (mut a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                // Fork builds both halves, so the child's own packed bytes
                // floor the heap (probed on a fresh decode, outside
                // measurement); the generic in-place NA would misstate
                // what fork does.
                let child_bytes = {
                    let bytes = f.parties.as_ref().map(|(a, _)| a.clone())?;
                    let mut probe = decode_party(&bytes);
                    (probe.fork().encoded_bits() / 8) as u64
                };
                let floors = Floors {
                    heap: if child_bytes == 0 {
                        na(NA_HEAP_IN_PLACE)
                    } else {
                        Liveness::Floor {
                            min: child_bytes,
                            why: WHY_HEAP_FORK_HALF,
                        }
                    },
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: if a.is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || {
                    let child = a.fork();
                    (a, child)
                }))
            },
        },
        Op {
            name: "party_join",
            group: OpGroup::Party,
            prepare: |f| {
                let (mut a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "party_join_all",
            group: OpGroup::Fold,
            prepare: |f| {
                let (_, parties) = f.fold.as_ref()?;
                let n = parties.iter().map(Vec::len).sum();
                let arity = parties.len() as u64;
                let mut parties = parties.iter().map(|b| decode_party(b));
                let acc = parties.next().expect("the scatter population is nonempty");
                let rest: Vec<Party> = parties.collect();
                // The declared search allowance: the accumulator's table
                // size prices each tested input's both-present nodes
                // (INDEX_PROBE_SCAN_BITS carries the derivation).
                let table = both_present_nodes(&acc);
                let probes_per_node = u64::from((table + 1).next_power_of_two().trailing_zeros());
                let search_bits = INDEX_PROBE_SCAN_BITS
                    * probes_per_node
                    * rest.iter().map(both_present_nodes).sum::<u64>();
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(
                    Cell::new(n, floors, move || {
                        let mut acc = acc;
                        acc.join_all(rest)
                            .expect("fold operands are forked parties, pairwise disjoint");
                        acc
                    })
                    .with_fold_arity(arity)
                    .with_fold_search(search_bits),
                )
            },
        },
        Op {
            name: "party_covers",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_touch(),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.covers(&b), a, b)))
            },
        },
        Op {
            name: "party_disjoint",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.is_disjoint(&b), a, b)))
            },
        },
        Op {
            name: "party_without",
            group: OpGroup::Party,
            prepare: |f| {
                let (_, b, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(_, b)| b.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n + 1, floors, move || {
                    (Party::seed().without(&b), b)
                }))
            },
        },
        Op {
            name: "party_display",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let spec = TextSpec {
                    radix_units: radix_units_party(&a),
                    spelled_values: 0,
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Party)>()
                            .expect("the display body yields (text, party)")
                            .0
                            .len()
                    },
                    spec,
                    move || (a.to_string(), a),
                ))
            },
        },
        Op {
            name: "party_from_str",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let s = a.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_party(&a),
                    spelled_values: 0,
                    output_is_text: false,
                };
                assert_honest_text("party_from_str input", s.len(), spec.radix_units);
                let packed = a.encoded_bits().div_ceil(8);
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        r.downcast_ref::<Party>()
                            .expect("the parse body yields a party")
                            .encoded_bits()
                            .div_ceil(8)
                    },
                    spec,
                    move || s.parse::<Party>().expect("a displayed party parses back"),
                ))
            },
        },
        Op {
            name: "party_hash",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    a.hash(&mut hasher);
                    (hasher.finish(), a)
                }))
            },
        },
        // ── Clock ──────────────────────────────────────────────────────
        Op {
            name: "clock_decode",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let bytes = clock.encode();
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
                    limb: limb_stream(mandatory_limbs_stream(clock.version())),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(bytes.len()),
                    touch: touch_wide_stream(clock.version()),
                };
                Some(Cell::new(bytes.len(), floors, move || {
                    Clock::decode(&bytes[..]).expect("an encoded clock decodes back")
                }))
            },
        },
        Op {
            name: "clock_encode",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (clock.encode(), clock)))
            },
        },
        Op {
            name: "clock_tick",
            group: OpGroup::Tick,
            prepare: |f| {
                // The tick-walk families tick their own (id, event)
                // clock; they reach no other clock row.
                if let Some((v, p, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    let mut clock = Clock::from_parts(p, v);
                    return Some(Cell::new(n, floors, move || {
                        clock.tick();
                        clock
                    }));
                }
                let (mut clock, n) = f.clock()?;
                // A version-bearing shape's clock ticks its seed party (an
                // in-place raise); the id pair's clock ticks an empty
                // version (pure growth). Neither runs the accumulator.
                let touch = if clock.version().is_empty() {
                    na(NA_TOUCH_GROW)
                } else {
                    na(NA_TOUCH_SEED_RAISE)
                };
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    clock.tick();
                    clock
                }))
            },
        },
        Op {
            name: "clock_fork",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut clock, n) = f.clock()?;
                let floors = Floors {
                    heap: heap_fork_child(clock.version()),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: if clock.party().is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || {
                    let child = clock.fork();
                    (clock, child)
                }))
            },
        },
        Op {
            name: "clock_join",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut a, b, n) = f.clock_pair()?;
                let touch =
                    touch_delta_fold(stored_deltas(a.version()) + stored_deltas(b.version()));
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "clock_sync",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut a, mut b, n) = f.clock_pair()?;
                let touch =
                    touch_delta_fold(stored_deltas(a.version()) + stored_deltas(b.version()));
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    let synced = a.sync(&mut b).is_ok();
                    (synced, a, b)
                }))
            },
        },
        Op {
            name: "clock_recv",
            group: OpGroup::Clock,
            prepare: |f| {
                // Small clock × adversarial received version.
                if let Some((v, n)) = f.version() {
                    let mut clock = Clock::seed();
                    let touch = touch_delta_fold(stored_deltas(&v));
                    return Some(Cell::new(n + 2, walk_floors(n, touch), move || {
                        clock.recv(&v);
                        (clock, v)
                    }));
                }
                // Adversarial party × small received version.
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut clock = Clock::from_parts(a, Version::new());
                let msg = Version::try_from(1u64).expect("a one-tick version is valid");
                let touch = touch_delta_fold(stored_deltas(&msg));
                Some(Cell::new(n + 2, walk_floors(n, touch), move || {
                    clock.recv(&msg);
                    (clock, msg)
                }))
            },
        },
        Op {
            name: "clock_own_version_to_version",
            group: OpGroup::Projection,
            prepare: |f| {
                // The clock spelling of the explicit materialization:
                // `clock.own_version()` is an O(1) view (no cell of its
                // own — nothing scales), and this row prices its
                // `.to_version()`. Adversarial × adversarial with
                // mandatory dominating output: a clock holding the cross's
                // event side whose party is its id side, I/O-denominated
                // (the module doc's output-domination cross).
                if f.output_dominated {
                    let (v_bytes, p_bytes) = f.cross.as_ref()?;
                    let n = v_bytes.len() + p_bytes.len();
                    let clock = Clock::from_parts(decode_party(p_bytes), decode_version(v_bytes));
                    let cell = Cell::io(
                        n,
                        walk_floors(n, na(NA_TOUCH_PROJECTION)),
                        |r| {
                            let (out, _) = r
                                .downcast_ref::<(Version, Clock)>()
                                .expect("the own_version body yields (out, clock)");
                            version_output_bytes(out)
                        },
                        move || (clock.own_version().to_version(), clock),
                    );
                    // The same ratified capacity chain as the version
                    // spelling of this materialization.
                    return Some(if matches!(f.kind, FamilyKind::CombScatter) {
                        cell.with_capacity_model()
                    } else {
                        cell
                    });
                }
                let (clock, n) = f.clock()?;
                Some(Cell::new(
                    n,
                    walk_floors(n, na(NA_TOUCH_PROJECTION)),
                    move || (clock.own_version().to_version(), clock),
                ))
            },
        },
        Op {
            name: "clock_display",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let spec = TextSpec {
                    radix_units: radix_units_clock(&clock),
                    spelled_values: stored_bases(clock.version()).len() as u64,
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_DEPENDENCY),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_RENDER_SUMMARIES),
                };
                Some(Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Clock)>()
                            .expect("the display body yields (text, clock)")
                            .0
                            .len()
                    },
                    spec,
                    move || (clock.to_string(), clock),
                ))
            },
        },
        Op {
            name: "clock_from_str",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let s = clock.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_clock(&clock),
                    spelled_values: stored_bases(clock.version()).len() as u64,
                    output_is_text: false,
                };
                assert_honest_text("clock_from_str input", s.len(), spec.radix_units);
                let packed = clock.encoded_bits().div_ceil(8);
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: limb_wide(mandatory_limbs_version(clock.version())),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                    touch: touch_delta_fold(stored_deltas(clock.version())),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        r.downcast_ref::<Clock>()
                            .expect("the parse body yields a clock")
                            .encoded_bits()
                            .div_ceil(8)
                    },
                    spec,
                    move || s.parse::<Clock>().expect("a displayed clock parses back"),
                ))
            },
        },
        Op {
            name: "clock_hash",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    clock.hash(&mut hasher);
                    (hasher.finish(), clock)
                }))
            },
        },
        // ── the rejection surface (the module doc's rejection section) ─
        Op {
            name: "version_decode_truncated",
            group: OpGroup::Version,
            prepare: |f| {
                let bytes = f.version.clone()?;
                let fed = truncated_bytes(&bytes);
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Version::decode(&fed[..]).expect_err("a truncated stream is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "version_decode_trailing",
            group: OpGroup::Version,
            prepare: |f| {
                let bytes = f.version.clone()?;
                let fed = trailing_bytes(&bytes);
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Version::decode(&fed[..]).expect_err("a trailing-bits stream is rejected");
                    assert!(
                        matches!(err, Decode::TrailingBits),
                        "the placed defect is the appended tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "version_decode_noncanon",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let fed = version_noncanonical_bytes(&v);
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Version::decode(&fed[..]).expect_err("a non-canonical tail is rejected");
                    assert!(
                        matches!(err, Decode::NotCanonical),
                        "the placed defect is the equal-sibling tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "version_parse_trailing",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let fed = trailing_text(&v.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_REJECTION), na(NA_TOUCH_REJECTION));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Version>()
                        .expect_err("trailing junk after valid text is rejected");
                    assert!(
                        matches!(err, Parse::Syntax),
                        "the placed defect is the trailing junk, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "version_parse_noncanon",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let fed = version_noncanonical_text(&v.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_REJECTION), na(NA_TOUCH_REJECTION));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Version>()
                        .expect_err("a non-canonical tail is rejected");
                    assert!(
                        matches!(err, Parse::NotCanonical),
                        "the placed defect is the equal-sibling tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_decode_truncated",
            group: OpGroup::Party,
            prepare: |f| {
                let bytes = f.parties.as_ref().map(|(a, _)| a.clone())?;
                let fed = truncated_bytes(&bytes);
                let n = fed.len();
                let floors = id_rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err = Party::decode(&fed[..]).expect_err("a truncated stream is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_decode_trailing",
            group: OpGroup::Party,
            prepare: |f| {
                let bytes = f.parties.as_ref().map(|(a, _)| a.clone())?;
                let fed = trailing_bytes(&bytes);
                let n = fed.len();
                let floors = id_rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Party::decode(&fed[..]).expect_err("a trailing-bits stream is rejected");
                    assert!(
                        matches!(err, Decode::TrailingBits),
                        "the placed defect is the appended tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_decode_noncanon",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let fed = party_noncanonical_bytes(&a);
                let n = fed.len();
                let floors = id_rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Party::decode(&fed[..]).expect_err("a non-canonical tail is rejected");
                    assert!(
                        matches!(err, Decode::NotCanonical),
                        "the placed defect is the collapsible (1, 1) tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_parse_trailing",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let fed = trailing_text(&a.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_ID_TREE), na(NA_TOUCH_ID_TREE));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Party>()
                        .expect_err("trailing junk after valid text is rejected");
                    assert!(
                        matches!(err, Parse::Syntax),
                        "the placed defect is the trailing junk, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_parse_noncanon",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let fed = party_noncanonical_text(&a.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_ID_TREE), na(NA_TOUCH_ID_TREE));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Party>()
                        .expect_err("a non-canonical tail is rejected");
                    assert!(
                        matches!(err, Parse::NotCanonical),
                        "the placed defect is the collapsible (1, 1) tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "clock_decode_truncated",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let fed = truncated_bytes(&clock.encode());
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err = Clock::decode(&fed[..]).expect_err("a truncated stream is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "clock_decode_trailing",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let fed = trailing_bytes(&clock.encode());
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Clock::decode(&fed[..]).expect_err("a trailing-bits stream is rejected");
                    assert!(
                        matches!(err, Decode::TrailingBits),
                        "the placed defect is the appended tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "clock_parse_trailing",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let fed = clock_trailing_text(&clock.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_REJECTION), na(NA_TOUCH_REJECTION));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Clock>()
                        .expect_err("junk inside the stamp's parens is rejected");
                    assert!(
                        matches!(err, Parse::Syntax),
                        "the placed defect is the inner junk, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_join_overlap",
            group: OpGroup::Party,
            prepare: |f| {
                let (a_bytes, b_bytes) = f.overlap.clone()?;
                let n = a_bytes.len() + b_bytes.len();
                let mut a = decode_party(&a_bytes);
                let b = decode_party(&b_bytes);
                let floors = id_rejection_floors(n, WHY_SCAN_OVERLAP_END);
                Some(Cell::new(n, floors, move || {
                    let back = a
                        .join(b)
                        .expect_err("the overlap-mounted pair must be rejected");
                    (back, a)
                }))
            },
        },
        Op {
            name: "clock_join_overlap",
            group: OpGroup::Clock,
            prepare: |f| {
                let (a_bytes, b_bytes) = f.overlap.clone()?;
                let id_bytes = a_bytes.len() + b_bytes.len();
                // Versions ride along where the bundle has them (empty
                // otherwise); rejection does no version work — the party
                // join is the gate — so the scan floor covers the ids.
                let (v, w, nv) = match f.version_pair() {
                    Some(pair) => pair,
                    None => (Version::new(), Version::new(), 2),
                };
                let n = id_bytes + nv;
                let mut a = Clock::from_parts(decode_party(&a_bytes), v);
                let b = Clock::from_parts(decode_party(&b_bytes), w);
                Some(Cell::new(n, clock_overlap_floors(id_bytes), move || {
                    let back = a
                        .join(b)
                        .expect_err("the overlap-mounted pair must be rejected");
                    (back, a)
                }))
            },
        },
        Op {
            name: "clock_sync_overlap",
            group: OpGroup::Clock,
            prepare: |f| {
                let (a_bytes, b_bytes) = f.overlap.clone()?;
                let id_bytes = a_bytes.len() + b_bytes.len();
                let (v, w, nv) = match f.version_pair() {
                    Some(pair) => pair,
                    None => (Version::new(), Version::new(), 2),
                };
                let n = id_bytes + nv;
                let mut a = Clock::from_parts(decode_party(&a_bytes), v);
                let mut b = Clock::from_parts(decode_party(&b_bytes), w);
                Some(Cell::new(n, clock_overlap_floors(id_bytes), move || {
                    let err = a
                        .sync(&mut b)
                        .expect_err("the overlap-mounted pair must be rejected");
                    (err, a, b)
                }))
            },
        },
        Op {
            name: "party_join_all_overlap",
            group: OpGroup::Fold,
            prepare: |f| {
                // One large accumulator, many one-byte probes each
                // overlapping its right half behind the whole left shape:
                // every probe is tested against the fixed accumulator and
                // handed back, and the probe count scales with the
                // accumulator (the divisor's rustdoc), so any per-input
                // work scaling with the accumulator reads quadratic here
                // while the indexed test's O(probe) checks read linear.
                let (a_bytes, _) = f.overlap.clone()?;
                let acc = decode_party(&a_bytes);
                let probe = overlap_fold_probe();
                assert!(
                    !acc.is_disjoint(&decode_party(&probe)),
                    "the fold probe overlaps the a-mount's right half"
                );
                let count = (a_bytes.len() / OVERLAP_FOLD_INPUT_DIVISOR).max(MIN_SIZE_PARAM);
                let inputs: Vec<Party> = (0..count).map(|_| decode_party(&probe)).collect();
                let n = a_bytes.len() + count * probe.len();
                let floors = id_rejection_floors(n, WHY_SCAN_EXAMINES);
                Some(Cell::new(n, floors, move || {
                    let mut acc = acc;
                    let back = acc
                        .join_all(inputs)
                        .expect_err("every probe overlaps the accumulator");
                    assert_eq!(back.len(), count, "every probe is handed back");
                    (back, acc)
                }))
            },
        },
        Op {
            name: "party_without_none",
            group: OpGroup::Party,
            prepare: |f| {
                // Identical-region operands: the diff walks both streams in
                // full, and the empty remainder is known only at the end.
                let bytes = f.parties.as_ref().map(|(a, _)| a.clone())?;
                let n = bytes.len() * 2;
                let a = decode_party(&bytes);
                let b = decode_party(&bytes);
                let floors = id_rejection_floors(n, WHY_SCAN_EXAMINES);
                Some(Cell::new(n, floors, move || {
                    let gone = a.without(&b);
                    assert!(gone.is_none(), "removing a covering region leaves nothing");
                    (gone, b)
                }))
            },
        },
    ]
}

// ─── measurement ────────────────────────────────────────────────────────────

/// One measured run of a cell body: every meter and its denominators.
struct Sample {
    /// The denominator of the heap and segment constants (and, on most
    /// cells, of every exponent): packed input bytes, or `n_io` for the
    /// I/O-denominated cells.
    denom_bytes: usize,
    /// The exponent legs' denominator.
    ///
    /// `denom_bytes` everywhere except the flat-denominator shape's
    /// input-denominated cells, where it is the bundle's value content:
    /// the packed denominator is intercept-dominated there, and a
    /// two-point power-law fit against an intercept-dominated denominator
    /// manufactures exponents out of exactly linear marginal work.
    exp_denom_bytes: usize,
    /// The limb *constant*'s denominator: `denom_bytes`, or `R` for the
    /// text rows (the limb exponent is judged against `denom_bytes` on
    /// every row).
    limb_denom: u64,
    /// Whether the limb column is judged at the text ceiling κ.
    text_row: bool,
    /// The cell's liveness declarations; each sample carries its own since
    /// floors scale with the sample's operands.
    floors: Floors,
    /// The fold rows' operand count at this sample's scale, for the
    /// declared `FoldLog` model.
    fold_arity: Option<u64>,
    /// The party fold's declared search allowance at this sample's
    /// scale, in scan bits.
    fold_search_bits: u64,
    /// The capacity-chain model's predicted peak heap for this sample
    /// ([`capacity_chain_peak`] over the actual input and output bytes),
    /// on the cells that declare it.
    heap_model: Option<f64>,
    /// Every currency's counter reading over the body; `None` where the
    /// counter is not compiled in (the feature-gated limb, scan, and
    /// touch columns render `off` and are exempt from judgment).
    readings: ByCurrency<Option<u64>>,
}

/// Run one prepared cell under all meters.
///
/// The denominators are settled after the meters are read and before the
/// result is dropped: an I/O-denominated cell's output side comes from the
/// actual result (never from a prediction), and a text output is checked
/// against the honesty ceiling right here.
fn measure(heap: &HeapMeter, op: &'static str, cell: Cell, content: Option<usize>) -> Sample {
    super::reset_stack_segments();
    reset_limb();
    reset_scan();
    reset_touch();
    (heap.reset_peak)();
    let baseline = (heap.current)();
    let result = (cell.body)();
    let peak_heap = (heap.peak)().saturating_sub(baseline);
    let segments = super::stack_segments();
    let limb = read_limb();
    let scan = read_scan();
    let touch = read_touch();
    let mut heap_model = None;
    let (denom_bytes, exp_denom_bytes, limb_denom, text_row) = match cell.denom {
        // The flat-denominator shape's content denominator carries the
        // exponent legs of its input-denominated cells alone: an
        // I/O-denominated cell's output side already scales.
        Denom::Input => {
            let exp = content.unwrap_or(cell.input_bytes);
            (cell.input_bytes, exp, cell.input_bytes as u64, false)
        }
        Denom::Io(spec) => {
            let output_bytes = (spec.output_bytes)(result.as_ref());
            if cell.capacity_model {
                heap_model = Some(capacity_chain_peak(cell.input_bytes, output_bytes));
            }
            let n_io = cell.input_bytes + output_bytes;
            match spec.text {
                None => (n_io, n_io, n_io as u64, false),
                Some(text) => {
                    if text.output_is_text {
                        assert_honest_text(op, output_bytes, text.radix_units);
                    }
                    let pipeline = TEXT_PIPELINE_LIMB_OPS_PER_VALUE * text.spelled_values;
                    (n_io, n_io, n_io as u64 + text.radix_units + pipeline, true)
                }
            }
        }
    };
    drop(result);
    Sample {
        denom_bytes,
        exp_denom_bytes,
        limb_denom,
        text_row,
        floors: cell.floors,
        fold_arity: cell.fold_arity,
        fold_search_bits: cell.fold_search_bits,
        heap_model,
        readings: ByCurrency {
            heap: Some(peak_heap as u64),
            segments: Some(segments),
            limb,
            scan,
            touch,
        },
    }
}

/// Reset the limb counter when the `limb-meter` feature carries one.
#[cfg(feature = "limb-meter")]
fn reset_limb() {
    super::reset_limb_ops();
}

/// Without the `limb-meter` feature there is no counter to reset.
#[cfg(not(feature = "limb-meter"))]
fn reset_limb() {}

/// Read the limb counter, or `None` without the `limb-meter` feature.
#[cfg(feature = "limb-meter")]
fn read_limb() -> Option<u64> {
    Some(super::limb_ops())
}

/// Without the `limb-meter` feature the limb column is absent.
#[cfg(not(feature = "limb-meter"))]
fn read_limb() -> Option<u64> {
    None
}

/// Reset the touch counter when the `limb-meter` feature carries one.
#[cfg(feature = "limb-meter")]
fn reset_touch() {
    suanpan::touch_meter::reset();
}

/// Without the `limb-meter` feature there is no touch counter to reset.
#[cfg(not(feature = "limb-meter"))]
fn reset_touch() {}

/// Read the touch counter, or `None` without the `limb-meter` feature.
#[cfg(feature = "limb-meter")]
fn read_touch() -> Option<u64> {
    Some(suanpan::touch_meter::touches())
}

/// Without the `limb-meter` feature the touch column is absent.
#[cfg(not(feature = "limb-meter"))]
fn read_touch() -> Option<u64> {
    None
}

/// Reset the scan counter when the `scan-meter` feature carries one.
#[cfg(feature = "scan-meter")]
fn reset_scan() {
    super::reset_scan_bits();
}

/// Without the `scan-meter` feature there is no counter to reset.
#[cfg(not(feature = "scan-meter"))]
fn reset_scan() {}

/// Read the scan counter, or `None` without the `scan-meter` feature.
#[cfg(feature = "scan-meter")]
fn read_scan() -> Option<u64> {
    Some(super::scan_bits())
}

/// Without the `scan-meter` feature the scan column is absent.
#[cfg(not(feature = "scan-meter"))]
fn read_scan() -> Option<u64> {
    None
}

/// The scaling exponent `log(m2/m1) / log(n2/n1)`, clamped finite.
///
/// A meter that reads zero at both scales scores 0; a zero at one scale is
/// clamped through `max(m, 1)` so the ratio stays defined. Degenerate input
/// sizes (`n2 <= n1`, possible only at extreme scale-down) score 0 rather
/// than dividing by a vanishing log.
fn exponent(m1: u64, m2: u64, n1: usize, n2: usize) -> f64 {
    if (m1 == 0 && m2 == 0) || n2 <= n1 {
        return 0.0;
    }
    let growth = (m2.max(1) as f64) / (m1.max(1) as f64);
    growth.ln() / ((n2 as f64) / (n1 as f64)).ln()
}

/// A liveness-floor trip's rendered message: the column and the vacuity
/// mechanism.
const HEAP_FLOOR_TRIP: &str =
    "heap floor: counter reads below floor: the meter is not watching this work";
/// The segments column's floor-trip message (unreachable while segments is
/// ceiling-only by policy; the judgment loop still carries it so a future
/// segments floor binds without a code change).
const SEG_FLOOR_TRIP: &str =
    "segments floor: counter reads below floor: the meter is not watching this work";
/// The limb column's floor-trip message.
const LIMB_FLOOR_TRIP: &str =
    "limb floor: counter reads below floor: the meter is not watching this work";
/// The scan column's floor-trip message.
const SCAN_FLOOR_TRIP: &str =
    "scan floor: counter reads below floor: the meter is not watching this work";
/// The touch column's floor-trip message.
const TOUCH_FLOOR_TRIP: &str =
    "touch floor: counter reads below floor: the meter is not watching this work";

/// Whether `count` sits below a committed floor (an NA declaration never
/// trips).
fn below_floor(liveness: Liveness, count: u64) -> bool {
    match liveness {
        Liveness::Floor { min, .. } => count < min,
        Liveness::NotApplicable { .. } => false,
    }
}

/// Panic unless two in-process measurements of one cell agree on every
/// counter reading and denominator.
///
/// The in-process leg of the board's determinism tripwire ([`run`]'s
/// self-verification); the cross-process leg is the `amp-board-determinism`
/// recipe, which byte-compares two whole renders.
fn assert_deterministic(op: &str, family: &str, a: &Sample, b: &Sample) {
    assert_eq!(
        (a.denom_bytes, a.exp_denom_bytes, a.limb_denom),
        (b.denom_bytes, b.exp_denom_bytes, b.limb_denom),
        "determinism: {op} x {family}: two in-process measurements disagree on denominators"
    );
    for ((currency, first), (_, second)) in a.readings.each().into_iter().zip(b.readings.each()) {
        assert_eq!(
            first,
            second,
            "determinism: {op} x {family}: two in-process measurements disagree on the {} \
             counter",
            currency.label()
        );
    }
}

/// One judged column's derived scores: the fitted exponent and the larger
/// scale's per-unit constant (`None` where the counter is off).
#[derive(Clone, Copy)]
struct Score {
    exp: Option<f64>,
    /// Whether the exponent leg is judged.
    ///
    /// False where the denominator pair does not scale
    /// ([`MIN_EXPONENT_DENOM_GROWTH`]) or, on the heap column, where both
    /// readings sit inside the flat allowance the constant leg already
    /// forgives (a sub-allowance exponent is allocator size-class noise,
    /// not scaling).
    exp_judged: bool,
    per_unit: Option<f64>,
}

/// One evaluated cell: both samples, per-currency scores, and the verdict.
struct CellResult {
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
    scores: ByCurrency<Score>,
    /// The meters over their bounds; empty means green.
    red: Vec<&'static str>,
}

/// Score a cell's two samples against the exponent bound, the ceilings,
/// and the liveness floors, per currency.
///
/// Every exponent — the limb column's included — is judged against the
/// denominator bytes (packed input, or `n_io` on the I/O-denominated
/// cells), never against `R`: `R` is the schoolbook cost law, so a limb
/// exponent against it reads a flat ~1 on exactly the quadratic converters
/// the bound exists to catch. Constants are judged per denominator byte,
/// except segments (an absolute count: the target is walks that never grow
/// the stack) and the text rows' limb constant, which is per `R` unit
/// under the κ ceiling. The loops run over the currency axis itself
/// ([`ByCurrency::each`]), so a currency added to the axis is judged on
/// every cell or the destructuring fails to compile.
fn evaluate(op: &'static str, family: &'static str, s1: Sample, s2: Sample) -> CellResult {
    let denom_scales =
        s2.exp_denom_bytes as f64 >= s1.exp_denom_bytes as f64 * MIN_EXPONENT_DENOM_GROWTH;
    // The declared models, resolved for this cell (the declared-models
    // section): the fold exponent ceiling needs both samples' arities,
    // and the capacity-chain judgment both samples' predictions.
    let fold_exp_ceiling = match (s1.fold_arity, s2.fold_arity) {
        (Some(k1), Some(k2)) if denom_scales => Some(fold_exponent_ceiling(
            k1,
            k2,
            s1.exp_denom_bytes,
            s2.exp_denom_bytes,
        )),
        _ => None,
    };
    let capacity_model = s1.heap_model.is_some() && s2.heap_model.is_some();
    let score = |c: Currency| -> Score {
        let (Some(m1), Some(m2)) = (*s1.readings.get(c), *s2.readings.get(c)) else {
            return Score {
                exp: None,
                exp_judged: false,
                per_unit: None,
            };
        };
        let exp = exponent(m1, m2, s1.exp_denom_bytes, s2.exp_denom_bytes);
        // A capacity-model cell's heap exponent is honestly unjudgeable:
        // the doubling chain quantizes the peak by powers of two, so a
        // probe pair straddling a k step manufactures an exponent and one
        // inside a step reads sublinear; the model judgment below is what
        // binds instead.
        let exp_judged = denom_scales
            && (c != Currency::Heap
                || (!capacity_model && m1.max(m2) > HEAP_FLAT_ALLOWANCE_BYTES as u64));
        let per_unit = match c {
            Currency::Heap => {
                m2.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64) as f64 / s2.denom_bytes as f64
            }
            Currency::Segments => m2 as f64,
            Currency::Limb => m2 as f64 / s2.limb_denom as f64,
            Currency::Scan | Currency::Touch => m2 as f64 / s2.denom_bytes as f64,
        };
        Score {
            exp: Some(exp),
            exp_judged,
            per_unit: Some(per_unit),
        }
    };
    let scores = ByCurrency {
        heap: score(Currency::Heap),
        segments: score(Currency::Segments),
        limb: score(Currency::Limb),
        scan: score(Currency::Scan),
        touch: score(Currency::Touch),
    };

    let mut red = Vec::new();
    for (c, s) in scores.each() {
        let (mut ceiling, exp_label, const_label) = match c {
            Currency::Heap => (
                MAX_HEAP_BYTES_PER_INPUT_BYTE,
                "heap exponent",
                "heap constant",
            ),
            Currency::Segments => (
                MAX_GROWN_STACK_SEGMENTS as f64,
                "segments exponent",
                "segments count",
            ),
            Currency::Limb => (
                if s2.text_row {
                    MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT
                } else {
                    MAX_LIMB_OPS_PER_INPUT_BYTE
                },
                "limb exponent",
                "limb constant",
            ),
            Currency::Scan => (
                MAX_SCAN_BITS_PER_INPUT_BYTE,
                "scan exponent",
                "scan constant",
            ),
            Currency::Touch => (
                MAX_TOUCHES_PER_INPUT_BYTE,
                "touch exponent",
                "touch constant",
            ),
        };
        // The capacity-model heap leg: both samples' readings must sit
        // inside the declared band around the model — over the ceiling is
        // the regression the model prices, under the floor is a stale
        // model that must be re-declared against the improved kernel.
        if c == Currency::Heap && capacity_model {
            let banded = |sample: &Sample, edge: f64| -> Option<bool> {
                let reading = (*sample.readings.get(c))? as f64;
                let model = sample.heap_model?;
                Some(if edge > 1.0 {
                    reading > model * edge
                } else {
                    reading < model * edge
                })
            };
            if [&s1, &s2]
                .iter()
                .any(|s| banded(s, CAPACITY_MODEL_CEILING).is_some_and(|over| over))
            {
                red.push("heap capacity-model ceiling");
            }
            if [&s1, &s2]
                .iter()
                .any(|s| banded(s, CAPACITY_MODEL_FLOOR).is_some_and(|under| under))
            {
                red.push("heap capacity-model floor (stale model)");
            }
            continue;
        }
        // The fold rows' declared exponent ceiling (limb, scan, touch)
        // and scan-constant model.
        let exp_ceiling = match (c, fold_exp_ceiling) {
            (Currency::Limb | Currency::Scan | Currency::Touch, Some(ceiling)) => ceiling,
            _ => MAX_SCALING_EXPONENT,
        };
        if c == Currency::Scan {
            if let Some(k2) = s2.fold_arity {
                ceiling = FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL * (2.0 * k2 as f64).log2()
                    + s2.fold_search_bits as f64 / s2.denom_bytes as f64;
            }
        }
        if s.exp_judged && s.exp.is_some_and(|e| e > exp_ceiling) {
            red.push(exp_label);
        }
        if s.per_unit.is_some_and(|v| v > ceiling) {
            red.push(const_label);
        }
    }
    // The liveness floors bind in this same pass, at both scales: a counter
    // reading below the least a watching meter could honestly read means the
    // meter is not watching the work the ceilings claim to bound.
    for (c, _) in scores.each() {
        let trip = match c {
            Currency::Heap => HEAP_FLOOR_TRIP,
            Currency::Segments => SEG_FLOOR_TRIP,
            Currency::Limb => LIMB_FLOOR_TRIP,
            Currency::Scan => SCAN_FLOOR_TRIP,
            Currency::Touch => TOUCH_FLOOR_TRIP,
        };
        if [&s1, &s2].iter().any(|s| {
            s.readings
                .get(c)
                .is_some_and(|r| below_floor(*s.floors.get(c), r))
        }) {
            red.push(trip);
        }
    }

    CellResult {
        op,
        family,
        s1,
        s2,
        scores,
        red,
    }
}

// ─── rendering ──────────────────────────────────────────────────────────────

/// Render one liveness declaration's floor value: the committed minimum, or
/// `-` for a not-applicable column.
fn floor_value(liveness: Liveness) -> String {
    match liveness {
        Liveness::Floor { min, .. } => min.to_string(),
        Liveness::NotApplicable { .. } => "-".to_string(),
    }
}

/// A red cell's mechanism tag: the judgment kinds present on its red
/// list, in a fixed order.
///
/// An `exponent` red is a scaling-class finding; a `constant` red (flat
/// or declared-model ceilings, the segments count) is a proportionality
/// finding at exponent ~1; a `floor` red is a liveness vacuity (a meter
/// not watching the work) or a stale declared model.
fn mechanism(red: &[&'static str]) -> String {
    let mut kinds = Vec::new();
    if red.iter().any(|label| label.contains("exponent")) {
        kinds.push("exponent");
    }
    if red.iter().any(|label| {
        label.contains("constant") || label.contains("count") || label.contains("ceiling")
    }) {
        kinds.push("constant");
    }
    if red.iter().any(|label| label.contains("floor")) {
        kinds.push("floor");
    }
    kinds.join("+")
}

/// Render one result row.
///
/// The byte range is the cell's denominator (packed input, or `n_io` on the
/// I/O-denominated cells); a text row's limb constant reads `/R` (its
/// exponent, like every exponent, is against the denominator bytes),
/// everything else `/B`. The `flr` column shows the larger scale's committed
/// liveness floors per judged column (`-` where not applicable; derivations
/// in the legend above the matrix).
fn row(out: &mut dyn Write, r: &CellResult) -> io::Result<()> {
    let verdict = if r.red.is_empty() { "GREEN" } else { "RED" };
    // An exponent the guards leave unjudged renders -.-- : printing the
    // fitted digits would invite reading noise as a measurement.
    let exp_text = |s: &Score| -> String {
        match s.exp {
            Some(e) if s.exp_judged => format!("{e:5.2}"),
            Some(_) => " -.--".to_string(),
            None => "     ".to_string(),
        }
    };
    let limb = match (r.scores.limb.exp, r.scores.limb.per_unit) {
        (Some(_), Some(c)) => {
            let unit = if r.s2.text_row { "/R" } else { "/B" };
            format!("limb[e{} {c:>10.1}{unit}]", exp_text(&r.scores.limb))
        }
        _ => "limb[      off      ]".to_string(),
    };
    let scan = match (r.scores.scan.exp, r.scores.scan.per_unit) {
        (Some(_), Some(c)) => format!("scan[e{} {c:>10.1}/B]", exp_text(&r.scores.scan)),
        _ => "scan[      off      ]".to_string(),
    };
    let touch = match (r.scores.touch.exp, r.scores.touch.per_unit) {
        (Some(_), Some(c)) => format!("touch[e{} {c:>10.1}/B]", exp_text(&r.scores.touch)),
        _ => "touch[      off      ]".to_string(),
    };
    // A red cell's mechanism tag: which judgment kinds put it on the red
    // list (the class-binding seal in `testing::complexity_claims` keys
    // on the exponent kind).
    let reasons = if r.red.is_empty() {
        String::new()
    } else {
        format!("  mech[{}]  <- {}", mechanism(&r.red), r.red.join(", "))
    };
    // A cell whose exponents are fitted against a different denominator
    // than its constants discloses the pair on its own row.
    let expd = if r.s2.exp_denom_bytes == r.s2.denom_bytes {
        String::new()
    } else {
        format!(
            "  expd[content {e1}->{e2} B]",
            e1 = r.s1.exp_denom_bytes,
            e2 = r.s2.exp_denom_bytes,
        )
    };
    // A declared per-cell model is disclosed on the row it judges; the
    // legend above the matrix carries the derivations.
    let decl = match (r.s1.heap_model, r.s2.heap_model, r.s2.fold_arity) {
        (Some(m1), Some(m2), _) => {
            format!("  decl[heap cap-chain {m1:.0}->{m2:.0} B]")
        }
        (_, _, Some(k2)) => {
            let k1 = r.s1.fold_arity.expect("fold cells declare both scales");
            if r.s2.fold_search_bits > 0 {
                format!(
                    "  decl[fold k {k1}->{k2} search {s1}->{s2} bits]",
                    s1 = r.s1.fold_search_bits,
                    s2 = r.s2.fold_search_bits,
                )
            } else {
                format!("  decl[fold k {k1}->{k2}]")
            }
        }
        _ => String::new(),
    };
    writeln!(
        out,
        "{verdict:<5} {op:<24} {family:<12} {n1:>8}->{n2:<8} B  \
         heap[e{he} {hc:>10.1}/B]  seg[e{se} {sc:>4}]  {limb}  {scan}  {touch}  \
         flr[h {fh:>6} l {fl:>6} s {fs:>6} t {ft:>6}]{expd}{decl}{reasons}",
        op = r.op,
        family = r.family,
        n1 = r.s1.denom_bytes,
        n2 = r.s2.denom_bytes,
        he = exp_text(&r.scores.heap),
        hc = r.scores.heap.per_unit.unwrap_or(0.0),
        se = exp_text(&r.scores.segments),
        sc = r.s2.readings.segments.unwrap_or(0),
        fh = floor_value(r.s2.floors.heap),
        fl = floor_value(r.s2.floors.limb),
        fs = floor_value(r.s2.floors.scan),
        ft = floor_value(r.s2.floors.touch),
    )
}

/// Run the whole board and render the matrix to `out`.
///
/// `scale` multiplies every family's base size (1.0 is the seconds-scale
/// default; the smoke test passes a small fraction). Cells run at the scaled
/// size and its double. Red rows print first.
///
/// # Panics
///
/// Panics if `scale` is not strictly positive.
pub fn run(scale: f64, heap: &HeapMeter, out: &mut dyn Write) -> io::Result<Summary> {
    assert!(
        scale > 0.0 && scale.is_finite(),
        "amp-board: scale must be a positive finite number"
    );

    let families: Vec<(FamilyData, FamilyData)> = FAMILIES
        .iter()
        .map(|&kind| {
            (
                FamilyData::build(kind, scale, 0),
                FamilyData::build(kind, scale, 1),
            )
        })
        .collect();

    let mut results = Vec::new();
    for op in ops() {
        for (small, large) in &families {
            let Some(c1) = (op.prepare)(small) else {
                continue;
            };
            let c2 = (op.prepare)(large)
                .expect("a cell's applicability depends on the family, never the size");
            let s1 = measure(heap, op.name, c1, small.content_bytes);
            let s2 = measure(heap, op.name, c2, large.content_bytes);
            // The runner self-verifies: every cell is measured twice in
            // process and every counter reading and denominator must
            // agree exactly — the board's judged quantities are
            // deterministic domain counters, so any disagreement is a
            // nondeterminism bug in a meter or a body, stopped here
            // rather than laundered into a verdict.
            for (level, first) in [(small, &s1), (large, &s2)] {
                let again = (op.prepare)(level)
                    .expect("a cell's applicability depends on the family, never the size");
                let second = measure(heap, op.name, again, level.content_bytes);
                assert_deterministic(op.name, small.name, first, &second);
            }
            results.push(evaluate(op.name, small.name, s1, s2));
        }
    }

    writeln!(
        out,
        "amplification board: transient cost vs denominator bytes (packed input; total I/O on \
         the text and cross cells), each cell at two scales"
    )?;
    writeln!(
        out,
        "green iff every meter's exponent <= {MAX_SCALING_EXPONENT}, constants within: \
         heap <= {MAX_HEAP_BYTES_PER_INPUT_BYTE} B/B over {HEAP_FLAT_ALLOWANCE_BYTES} B flat, \
         segments <= {MAX_GROWN_STACK_SEGMENTS}, \
         limb <= {MAX_LIMB_OPS_PER_INPUT_BYTE} ops/B \
         (text rows: <= {MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT} ops/R), \
         scan <= {MAX_SCAN_BITS_PER_INPUT_BYTE} bits/B, \
         touch <= {MAX_TOUCHES_PER_INPUT_BYTE} touches/B; \
         and every committed liveness floor met (flr[...]: a counter below its floor is red: \
         the meter is not watching that work; segments is ceiling-only by policy, its honest \
         floor is zero). exponent legs are fitted only where the denominator pair scales \
         (>= x{MIN_EXPONENT_DENOM_GROWTH} between probes) and, on heap, where a reading \
         clears the flat allowance the constant leg already forgives; an unjudged exponent \
         renders -.-- and the cell rides its constants and floors. every judged quantity is \
         a deterministic counter: the time-exponent leg lives in the bench judge \
         (just bench-judge)"
    )?;
    writeln!(out)?;
    writeln!(out, "liveness declarations on this board:")?;
    let mut legend = std::collections::BTreeSet::new();
    for r in &results {
        for (currency, liveness) in r.s2.floors.each() {
            legend.insert(match liveness {
                Liveness::Floor { why, .. } => format!("  {} floor: {why}", currency.label()),
                Liveness::NotApplicable { reason } => {
                    format!("  {} n/a: {reason}", currency.label())
                }
            });
        }
    }
    for line in &legend {
        writeln!(out, "{line}")?;
    }
    if results.iter().any(|r| r.s2.fold_arity.is_some()) {
        writeln!(
            out,
            "  declared fold model (decl[fold ...] rows): the balanced reduction's O(D log k) \
             class - exponent ceilings on limb/scan/touch at the model's predicted exponent \
             plus the linear cells' slack, scan constant at \
             {FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL} bits/B per log2(2k) reduction level"
        )?;
    }
    if results.iter().any(|r| r.s2.heap_model.is_some()) {
        writeln!(
            out,
            "  declared capacity model (decl[heap ...] rows): peak = 3(n+m)2^(k-1) B, \
             k = ceil(log2(output/(n+m))) - the output builder's doubling chain anchored at \
             the operand-size reserve; readings banded within x{CAPACITY_MODEL_FLOOR} to \
             x{CAPACITY_MODEL_CEILING} of the model at both scales"
        )?;
    }
    writeln!(out)?;

    let red: Vec<&CellResult> = results.iter().filter(|r| !r.red.is_empty()).collect();
    let green: Vec<&CellResult> = results.iter().filter(|r| r.red.is_empty()).collect();
    for r in red.iter().chain(green.iter()) {
        row(out, r)?;
    }

    writeln!(out)?;
    writeln!(
        out,
        "amp-board: {} green / {} red ({} cells)",
        green.len(),
        red.len(),
        results.len()
    )?;
    Ok(Summary {
        green: green.len(),
        red: red.len(),
    })
}

// ─── the bench export ───────────────────────────────────────────────────────

/// One board cell exposed for wall-clock benchmarking.
///
/// The bench suite (`benches/board.rs`) is the board's wall-time shadow: its
/// criterion group and function IDs are exactly [`BenchCell::op`] and
/// [`BenchCell::family`], so a board cell names the bench that times it and
/// a criterion filter selects a cell. The bench judge (`tools/benchjudge`)
/// reads those same IDs out of criterion's saved estimates to run the time
/// leg the module doc describes.
pub struct BenchCell {
    /// The board row's operation name: the bench group ID.
    pub op: &'static str,
    /// The input family's name: the bench function ID within the group.
    pub family: &'static str,
    /// The family operands the row's prepare reads, shared across cells.
    data: Rc<FamilyData>,
    /// The board row's prepare, re-run per measured body.
    prepare: fn(&FamilyData) -> Option<Cell>,
}

impl BenchCell {
    /// Build one fresh run of the cell's measured body.
    ///
    /// Operands are decoded anew on every call — the board's prepare
    /// discipline — so a bench harness rebuilds destructive operands in its
    /// untimed setup and times the returned closure alone. The closure is
    /// exactly what the board meters: same operands, same operation, same
    /// kept-alive result.
    pub fn body(&self) -> Box<dyn FnOnce() -> Box<dyn Any>> {
        (self.prepare)(&self.data)
            .expect("cell applicability was settled at construction")
            .body
    }

    /// The cell's denominator bytes at its scale: packed input, or total
    /// I/O on the I/O-denominated rows, or the bundle's value content on
    /// the flat-denominator shape's input-denominated rows.
    ///
    /// The content denominator is the same one the board fits those
    /// cells' exponents against: the judge's fitted time exponents must
    /// not re-manufacture what the board's re-denomination corrected.
    ///
    /// Runs one untimed body to read the output side back from the actual
    /// result, exactly as the board's measurement does (a prediction never
    /// substitutes for the result, and a text output is checked against the
    /// honesty ceiling on the way). The bench judge denominates its fitted
    /// time exponents against these bytes — the board's own convention — so
    /// a family whose packed bytes grow faster than the scale knob (the
    /// cliff comb's value content is quadratic in its parameter) is judged
    /// against what the operation actually reads and writes, never against
    /// the knob.
    pub fn denominator_bytes(&self) -> usize {
        let cell =
            (self.prepare)(&self.data).expect("cell applicability was settled at construction");
        let result = (cell.body)();
        match cell.denom {
            Denom::Input => self.data.content_bytes.unwrap_or(cell.input_bytes),
            Denom::Io(spec) => {
                let output_bytes = (spec.output_bytes)(result.as_ref());
                if let Some(text) = spec.text {
                    if text.output_is_text {
                        assert_honest_text(self.op, output_bytes, text.radix_units);
                    }
                }
                cell.input_bytes + output_bytes
            }
        }
    }
}

/// Which slice of the shape × operation product a bench run times.
///
/// The deterministic board always runs the whole product (its cells are
/// cheap counters); wall-clock benching is not, so the mirror has two
/// modes, both derived from the same axis declarations — the subset is a
/// rule over the product, never a second hand-maintained cell list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchMode {
    /// The pinned rule-derived subset: every operation on the benign
    /// control, each shape's designed-stress pairings (declared per
    /// shape on the shape axis), and the board-red riders
    /// ([`BOARD_RED_BENCH_RIDERS`]).
    Pinned,
    /// The whole product: the mode for final verdicts.
    Full,
}

/// One standing board red's committed expectation: the cell and the
/// judgment mechanisms that put it on the red list, unioned over the two
/// acceptance scales.
///
/// `exponent` is a scaling-class finding (some counter's growth exceeds
/// its ceiling — flat or declared); `constant` a proportionality finding
/// at exponent ~1 (a per-byte constant, a segments count, or a
/// declared-model band). The tags are the render's `mech[...]` column as
/// committed data: the class-binding seal in
/// `testing::complexity_claims` forbids any linear rustdoc claim from
/// citing an operation with a standing exponent-mechanism red, and
/// requires every counter-superlinear claim to keep one.
pub struct ExpectedRed {
    /// The board row's operation name.
    pub op: &'static str,
    /// The input family.
    pub family: &'static str,
    /// Whether the cell reads red on an exponent mechanism at either
    /// acceptance scale.
    pub exponent: bool,
    /// Whether the cell reads red on a constant mechanism at either
    /// acceptance scale.
    pub constant: bool,
}

/// The board's standing red cells with their mechanism tags: the
/// committed expectation the acceptance renders are compared against,
/// and the class-binding seal's data.
///
/// Realized 2026-07-28 against the release boards of record at both
/// scales (the render's `mech[...]` tags, unioned across scales). Every
/// entry names exactly one live board cell; a cured cell leaves this
/// roster in the same change that cures it, and a new red enters it (or
/// is cured) before acceptance — the acceptance protocol diffs the
/// rendered red set against this list.
pub const BOARD_EXPECTED_REDS: &[ExpectedRed] = &[
    // The ascending-cliff tick trio's heap constants (the spec's round-7
    // stated-band residual).
    ExpectedRed {
        op: "version_tick",
        family: "ascend-cliff",
        exponent: false,
        constant: true,
    },
    ExpectedRed {
        op: "version_ticks",
        family: "ascend-cliff",
        exponent: false,
        constant: true,
    },
    ExpectedRed {
        op: "clock_tick",
        family: "ascend-cliff",
        exponent: false,
        constant: true,
    },
    // The min_ticks anchor-web fold's reign state on the one family
    // that defeats batching — k simultaneously-open minima force Θ(k)
    // live records (the accepted stated-band residual; the anchor-web
    // cure removed this cell's exponent mechanism in the same change).
    ExpectedRed {
        op: "version_min_ticks",
        family: "ascend-cliff",
        exponent: false,
        constant: true,
    },
    // The render merge's wide-summary re-fold (the display pair's
    // SuperlinearTime mechanism, held alive by
    // render_merge_superlinearity_is_alive).
    ExpectedRed {
        op: "version_display",
        family: "mirror-wide",
        exponent: true,
        constant: true,
    },
    ExpectedRed {
        op: "clock_display",
        family: "mirror-wide",
        exponent: true,
        constant: true,
    },
];

/// Board-red cells outside the designed pairings that the pinned bench
/// subset must still time: the deterministic board's standing reds each
/// keep a time leg.
///
/// Membership is by `(operation, family)` cell name, expectations live in
/// the judge's roster as ever; a red cured on the board leaves this list
/// in the same change that cures it.
///
/// The current membership (the census re-realized 2026-07-28 against
/// [`BOARD_EXPECTED_REDS`] at the cure-round merge, where the
/// query-fold and finalize-arena cures land together): every standing
/// red whose cell the designed pairings do not already time. The
/// anchor-web cure left `version_min_ticks` red only on the
/// ascend-cliff cross (a heap-constant stated band, a row that shape
/// was never designed to stress — the tick trio's ascend-cliff reds
/// need no rider: the tick group is those crosses' designed diagonal),
/// and the finalize-arena cure removed the eleven cured display
/// riders, leaving the mirror-wide display pair (the render merge's
/// standing SuperlinearTime mechanism) as the display rows' only reds.
pub const BOARD_RED_BENCH_RIDERS: &[(&str, &str)] = &[
    ("version_min_ticks", "ascend-cliff"),
    ("version_display", "mirror-wide"),
    ("clock_display", "mirror-wide"),
];

/// Every board cell of the chosen [`BenchMode`] at `scale`, in board row
/// order.
///
/// `scale` multiplies the family base sizes exactly as [`run`]'s does; the
/// cells are op × family pairings applicable at that scale, at one
/// measurement level (a bench varies repetition, not size).
///
/// # Panics
///
/// Panics if `scale` is not a strictly positive finite number.
pub fn bench_cells(scale: f64, mode: BenchMode) -> Vec<BenchCell> {
    assert!(
        scale > 0.0 && scale.is_finite(),
        "bench cells: scale must be a positive finite number"
    );
    let families: Vec<Rc<FamilyData>> = FAMILIES
        .iter()
        .map(|&kind| Rc::new(FamilyData::build(kind, scale, 0)))
        .collect();
    let mut cells = Vec::new();
    for op in ops() {
        for family in &families {
            let include = match mode {
                BenchMode::Full => true,
                BenchMode::Pinned => {
                    designed(family.kind, op.group)
                        || BOARD_RED_BENCH_RIDERS.contains(&(op.name, family.name))
                }
            };
            if include && (op.prepare)(family).is_some() {
                cells.push(BenchCell {
                    op: op.name,
                    family: family.name,
                    data: Rc::clone(family),
                    prepare: op.prepare,
                });
            }
        }
    }
    cells
}
