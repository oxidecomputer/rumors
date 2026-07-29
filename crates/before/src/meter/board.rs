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
//!   fork rows' heap floor, at three derivations. The single-operand
//!   delta-folding kernels (the query rank folds, the tick walk, the
//!   text parse) land every stored delta of their one stream in the
//!   running accumulator, at least one digit touch per stored delta
//!   code — the same one-per-delta floor the envelope suite's flatness
//!   pins commit. The pair walks (the comparison sweep and the merge
//!   emitters and pair queries riding it) fold per *overlay boundary*:
//!   a boundary both operands step lands both step codes in one fold
//!   of the single running difference, so the honest pair floor is one
//!   touch per stepping boundary — at least the larger operand's
//!   stored-delta count, and legitimately half the naive two-stream
//!   delta sum on a boundary-aligned pair (the tooth-tail family is
//!   the committed demonstration; `touch_pair_fold` carries the
//!   derivation, and the n-ary fold row floors what its first-level
//!   merges alone force under the same premise). The validator batches word-scale deltas in the accumulator's
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
//!   per direction decides, so no fold count is forced), operand pairs
//!   equal byte for byte (canonical identity answers them before any
//!   sweep), and operands whose streams store no delta codes.
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
//! (scale 1, seconds of runtime). Acceptance is [`ACCEPTANCE_SCALE`]'s rule —
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
//! Some cells are judged against a **declared model** — a ratified
//! cost law derived at the cell, with a dated owner rationale committed
//! at the declaring constant — in place of one global ceiling, because
//! the global form is unsatisfiable on work their contracts mandate (the
//! same reasoning that re-denominates the I/O cells). A modeled cell
//! reads green because its behavior is *intended and modeled*; red is
//! reserved for untriaged contradictions (the red-triage buffer,
//! [`BOARD_EXPECTED_REDS`], is empty on the settled tree). Each model is
//! disclosed on its row face (`decl[...]`), derived at its constant's
//! definition site (the declared-models section of the ceilings block),
//! and held honest on the under side — banded floors where the model
//! predicts a quantity, committed liveness pins where it declares a
//! class — so an improved kernel forces a deliberate re-declaration,
//! and tripwired in the test suite by a wrong artifact reading red:
//!
//! - **The fold rows** (`version_join_all`, `version_meet_all`,
//!   `party_join_all`): the
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
//! - **Family-stated heap ceilings** (the tooth-tail parse cell, the
//!   ascend-cliff tick trio, and the ascend-cliff `version_min_ticks`
//!   cell): honest flat-exponent work state a ratified derivation puts
//!   over the global heap allowance — the densest committed
//!   node-per-text-byte parse stream, the zero-run ledger's certificate
//!   memory on the one shape that defeats consumption, and the anchor
//!   web's `Θ(k)` live reign records on the one shape that defeats
//!   batching. The heap *constant* is judged at the stated ceiling
//!   (each declaring constant carries its derivation and measured
//!   profile); the exponent leg stays at the global bound, so a
//!   flat-constant declaration can never absorb growth.
//! - **The mirror-wide display pair** (`version_display`,
//!   `clock_display` on the mirror-wide cross): the render merge's
//!   documented superlinear time class, judge-rostered on the wall leg,
//!   honestly reads a superlinear limb exponent and an over-κ limb
//!   constant on exactly this cross. Both limb legs are judged at the
//!   stated ceilings ([`MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING`],
//!   [`MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT`]); the class's
//!   liveness is the claims suite's
//!   `render_merge_superlinearity_is_alive` pin, which forces this
//!   declaration's re-derivation the day a render-merge cure lands.
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
//! freeze per block), `promo-rearm` (the many-armings spine, one
//! query-fold promotion per block), `weight-comb` (the many-jumps
//! spine, one accumulator-top jump and settle per block pair), and
//! `freeze-parade` (the deep-segment freeze spine, one scaled segment
//! read per block) — carry a version; the diverted id-spine pair carries a
//! disjoint party pair; the eleven cross shapes (`comb-scatter` and the
//! ten tick-walk crosses) carry a version, a mounted party pair, and a
//! clock; the two version-pair shapes — `jump-pair` (wide
//! height-difference crests over a dense-position spine) and
//! `concurrent-pair` (the switch-density population) — carry a version
//! pair of their own construction, so
//! their comparison rows run the pairing the shape was built around
//! rather than the ticked counterpart, and `tooth-tail` (the
//! boundary-aligned exact-`top` pair) carries its generator pair the same
//! way; the three fold populations —
//! `scatter`, `weave`, and `stagger` — carry fold operands alone, so
//! exactly the three
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
//! Ten shapes carry a genre note beyond their variant docs:
//!
//! - `freeze-pos`, built against the linear-functional rows: `Θ(s)`
//!   query-fold freezes at ever-deeper stream positions where every
//!   comb fires O(1). The committed known-bad kernel (the query fold's
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
//! - `weight-comb` and `freeze-parade`, the accumulator skip-mechanism
//!   families (the zero-run certificate ledger's and the write
//!   watermark's, respectively): each is a public-API stream that
//!   stays flat only through its mechanism, and each mechanism's
//!   absence reads ~×2 per byte across the family's doubling (the
//!   committed probe-build measurements in the `skyline_flatness` band
//!   ceilings, `tests/meter.rs` — the enforcement stays there; the
//!   columns exist so the dashboard is never structurally blind to the
//!   genre, every cell a live verdict over the mechanism that holds it
//!   flat).
//!
//! - `tooth-tail`, the third skip mechanism's family (exact-`top`
//!   maintenance): the boundary-aligned pair whose cancelled spike
//!   leaves `Θ(m)` post-cancellation sign reads over a `g`-digit dead
//!   buffer — flat with the settled top, `Θ(m·g)` with a high-water
//!   bound (the `skyline_flatness` tooth-tail band carries both
//!   readings; enforcement stays there). The pair is also the
//!   committed demonstration behind the comparison rows' per-boundary
//!   touch floor: same-shape operands share every overlay boundary,
//!   so the fused sweep honestly folds ~once per boundary against two
//!   stored deltas, and its parse rows are the board's densest
//!   node-per-text-byte streams (the family-declared parse heap
//!   ceiling at [`TOOTH_TAIL_PARSE_HEAP_BYTES_PER_TEXT_BYTE`] carries
//!   the derivation).
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
//! - `scatter`, whose bundle carries fold operands alone, for the three
//!   fold rows (`version_join_all`, `version_meet_all`,
//!   `party_join_all`; all also keep a `benign` control cell, folding the
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
//! - `stagger`, the reduction-loading fold population, also
//!   fold-rows-only: `n` operands of `m` unit teeth each, every
//!   operand's teeth in the gaps of every other's, fed in
//!   bit-reversed order so each binary-counter merge pairs operand
//!   groups whose slots diverge at the top address bit — every
//!   internal merge, at every level, swells to near the sum of its
//!   inputs' sizes, the declared `O(D log k)` model's intermediate-
//!   swell worst case with no coalescing until the last level.
//!   Scatter scales arity at single-leaf operands and weave scales
//!   operand size at fixed arity; this population is the joint axis
//!   \[measured — scan 8.6 bits/B per reduction level, constant
//!   across `n` and `m` doublings alike, under the declared 12\].
//!
//! # Coverage: the board tiling
//!
//! Every public operation — every row of `before::surface`'s method and
//! family rosters — either is priced by at least one board row (its
//! claim in the complexity-claims roster cites the rows by name) or
//! appears in [`BOARD_NOT_APPLICABLE`] with the mechanism-based reason
//! it has no meaningful adversarial operand of its own. The two sides
//! are disjoint and jointly total, enforced by the tiling test in the
//! complexity-claims suite (`board_coverage_tiles_the_public_surface`),
//! so a new public operation cannot land unpriced and unexcused.
//!
//! The wall-time mirror rides the same axes: the bench suite's
//! criterion IDs are exactly the board's op × family cell names
//! ([`bench_cells`] is the board's own table), so board coverage is
//! bench coverage cell for cell, with no second enumeration. Wall
//! benching pays criterion's warmup and sampling per cell, so the judge
//! cadence times the rule-derived pinned subset (each shape's
//! designed-stress pairings, the organic control, and the
//! declared-model riders) while `BOARD_BENCH_MODE=full` times the whole
//! product for final verdicts — the subset is a rule over the product,
//! never a hand-maintained cell list.
//!
//! Rows price delegations at their shared mechanism, so several surface
//! rows legitimately cite one row: `Clock::send` is `Clock::tick` by
//! definition; `clock | version` (either operand order, `|=` included)
//! folds through the same join-assign the `recv` row measures;
//! `Party::tick` is `Version::tick`'s mirror (the `tick_adv_party`
//! row); the operator matrix (`|`, `&`, and their assign forms, over
//! every borrow shape) routes through the same `join_view`/`meet_view`
//! emitters and cmp walk the `join`/`meet`/`cmp` rows measure;
//! `Version::concurrent` is one `partial_cmp` and keeps its own row as
//! the documented entry point; the serde/borsh wrappers serialize as
//! the canonical encoding and deserialize through the strict decoder
//! (the `encode`/`decode` rows); `Party::ticks` and `Clock::ticks` run
//! the same fused kernel as `version_ticks` through their own
//! spellings. Derived surfaces with no roster row of their own ride
//! the same cells: `Clone` copies stored bits or value content
//! wholesale with no walk in the contract, `Debug` delegates to
//! `Display`, and the byte-compare `Eq`s are the `eq`/`hash` rows'
//! wholesale compares.
//!
//! Which rows run on which shapes is decided by the operand bundles
//! (the product section above); the recurring carrier classes, named
//! here because no single declaration spells them out: the 19
//! version-carrying shapes (all but `id-pair` and the three fold
//! populations) run every version row; the party-pair carriers
//! (`id-pair`, `comb-scatter`, the ten tick crosses, `benign`) run the
//! party rows; every clock-carrying shape (the version carriers plus
//! `id-pair`) runs the clock rows; the projection rows add the
//! output-domination cross; and the three fold populations (`scatter`,
//! `weave`, `stagger`) plus the `benign` control carry fold operands,
//! so exactly the fold rows run on them.
//!
//! Two coverage notes that are dispositions of *error paths*, not
//! operations (so they live here rather than in the table): the
//! rejection rows above price the fallible surface, and **the
//! rejection surface's bounded-or-delegated remainder** is:
//! `Clock::join_all`'s overlap hand-back runs the identical up-front
//! indexed test against self that `party_join_all_overlap` prices,
//! inline; clock non-canonicality — packed or text — is the component
//! validators on the same streams the version and party non-canonical
//! rows drive; [`Decode::Anonymous`](crate::error::Decode) is the
//! accepting parse of the empty stream (a zero-byte operand, no scaling
//! axis) and [`Parse::Anonymous`](crate::error::Parse) the one-token
//! `"0"`; [`Decode::Io`](crate::error::Decode) is the caller's reader —
//! a failing reader is a truncation carrying an error, priced by the
//! truncated rows — and `encode_to`'s error the caller's writer, at
//! most the encode row's work before it propagates; the `TryFrom`
//! literal rejections have word-scale or type-bounded operands;
//! `Version::meet_all`'s `None` is the empty iterator;
//! `Rank::checked_sub`'s `None` is measured on the `rank_pair_ops`
//! row, which attempts both directions; other decode non-canonicality
//! genres (a negative running height, nonzero padding) ride the same
//! single validator pass at the same full-parse cost as the committed
//! maximally-deferred tails; serde/borsh deserialize errors are the
//! strict decoder through the wrappers (the decode rejection rows).
//! `Debug` for all three types delegates to `Display`.

mod ceilings;
mod cell;
mod coverage;
mod currency;
mod defect;
mod export;
mod family;
mod floors;
mod judge;
mod measure;
mod operand;
mod ops;
mod render;
#[cfg(test)]
mod tests;

pub use ceilings::{
    ACCEPTANCE_SCALE, ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE,
    ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE, CAPACITY_MODEL_CEILING, CAPACITY_MODEL_FLOOR,
    FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL, HEAP_FLAT_ALLOWANCE_BYTES, INDEX_PROBE_SCAN_BITS,
    MACHINE_WORD_MAGNITUDE_BITS, MAX_GROWN_STACK_SEGMENTS, MAX_HEAP_BYTES_PER_INPUT_BYTE,
    MAX_LIMB_OPS_PER_INPUT_BYTE, MAX_SCALING_EXPONENT, MAX_SCAN_BITS_PER_INPUT_BYTE,
    MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT, MAX_TOUCHES_PER_INPUT_BYTE, MIN_EXPONENT_DENOM_GROWTH,
    MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING, MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT,
    SCAN_FLOOR_BITS_PER_INPUT_BYTE, SCAN_TOUCH_FLOOR_BITS, TEXT_BYTES_PER_RADIX_UNIT,
    TEXT_PIPELINE_LIMB_OPS_PER_VALUE, TICKS_BOARD_COUNT, TOOTH_TAIL_PARSE_HEAP_BYTES_PER_TEXT_BYTE,
};
pub use coverage::{ExpectedRed, BOARD_EXPECTED_REDS, BOARD_NOT_APPLICABLE};
pub use currency::{ByCurrency, Currency, Floors, Liveness};
pub use export::{bench_cells, BenchCell, BenchMode, BOARD_DECLARED_BENCH_RIDERS};
pub use measure::HeapMeter;
pub use render::{run, Summary};
