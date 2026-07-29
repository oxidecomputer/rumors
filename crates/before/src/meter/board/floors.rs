//! The liveness vocabulary: the floor derivations and not-applicable
//! reasons every cell commits, shared across rows so the rendered legend
//! stays small and uniform.
//!
//! A floor states the least a watching counter can honestly read,
//! derived from what the operation must do, never from how it does it;
//! the board module doc's Liveness floors section carries the criterion
//! and what a trip means. The conventions, per currency:
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
//!   stored payload code wider than [`MACHINE_WORD_MAGNITUDE_BITS`](super::ceilings::MACHINE_WORD_MAGNITUDE_BITS) — a
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
//!   text parse) land every *nonzero* stored delta of their one stream
//!   in the running accumulator, at least one digit touch per nonzero
//!   delta code — a zero delta decodes but folds nothing, so a
//!   plateau-heavy stream legitimately reads near zero — the same
//!   one-per-nonzero-delta floor the envelope suite's flatness
//!   pins commit. The pair walks (the comparison sweep and the merge
//!   emitters and pair queries riding it) fold per *overlay boundary*:
//!   a boundary both operands step lands both step codes in one fold
//!   of the single running difference, so the honest pair floor is one
//!   touch per stepping boundary — at least the larger operand's
//!   stored-delta count, and legitimately half the naive two-stream
//!   delta sum on a boundary-aligned pair (the tooth-tail family is
//!   the committed demonstration; [`touch_pair_fold`] carries the
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
//!   sweep), and operands whose streams store no fold-forcing delta
//!   codes.
//! - **Heap** floors bind on the codec and text rows, whose results must
//!   materialize at least their packed bytes; everywhere else allocation is
//!   not semantically forced (and the heap meter reads the process
//!   allocator, which no re-routing inside the crate can bypass).
//! - **Segments** is ceiling-only by policy: the target is walks that never
//!   grow the stack, so its honest floor is zero and a zero floor asserts
//!   nothing.
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

use std::cmp::Ordering;

use crate::Version;

use super::ceilings::{
    SCAN_FLOOR_BITS_PER_INPUT_BYTE, SCAN_TOUCH_FLOOR_BITS, TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE,
};
use super::currency::{Floors, Liveness};
use super::operand::{mandatory_limbs_stream, stored_deltas, stored_nonzero_deltas};

/// Scan floor: the operation must examine its packed operands in full.
pub(super) const WHY_SCAN_EXAMINES: &str =
    "must examine its packed operands: at least one scanned bit per packed byte";
/// Scan floor: early exit is legitimate, but the root codes are still read.
const WHY_SCAN_TOUCH: &str =
    "may answer at the first divergence: still reads the operands' root codes";
/// Scan NA (`version_eq`'s own): same-form equality is decided on the
/// stored canonical bytes wholesale, and the operands grow without bound.
pub(super) const NA_SCAN_EQ_BYTES: &str =
    "decides same-form equality on the stored canonical bytes \
     wholesale (the compare may legitimately stop at the first differing byte): no stream walk \
     is in the contract; unlike the hash rows' small-operand exposure, eq operands grow without \
     bound, so the bench judge's time leg is the backstop that the compare stays linear";
/// Scan NA: the contract is a wholesale byte move.
pub(super) const NA_SCAN_BYTE_COPY: &str =
    "moves or hashes the stored canonical bytes wholesale: no stream walk is in the contract";
/// Scan NA: the operands carry no packed stream.
pub(super) const NA_SCAN_NO_STREAM: &str =
    "operands are decoded rank values: no packed stream exists";
/// Scan NA: a trivial (seed) operand stores no bits to scan.
pub(super) const NA_SCAN_SEED_PARTY: &str =
    "the forked party is the seed: its packed form is empty";
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
pub(super) const WHY_LIMB_RANK_PAIR: &str =
    "the mismatched pair's sum carries a numerator as wide as the \
     wider operand's value content: one limb write per 64 content bits";
/// Limb floor: the rank fold's sum spans its widest summand's content.
pub(super) const WHY_LIMB_RANK_SUM: &str =
    "the fold's sum carries a numerator as wide as its widest \
     summand's value content: one limb write per 64 content bits";
/// Limb NA: every operand magnitude fits machine words.
pub(super) const NA_LIMB_NARROW: &str =
    "no operand magnitude exceeds the machine-word bound: word arithmetic suffices";
/// Limb NA: the contract forces no arithmetic.
pub(super) const NA_LIMB_NOT_FORCED: &str =
    "magnitudes may be moved or compared without arithmetic: no limb work is in the contract";
/// Limb NA: id trees have no magnitudes at all.
pub(super) const NA_LIMB_ID_TREE: &str =
    "id trees store no magnitudes: there is no arithmetic to meter";
/// Limb NA: the work runs below the shim, in the dependency.
pub(super) const NA_LIMB_DEPENDENCY: &str =
    "the decimal conversion runs inside the bignum dependency, \
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
pub(super) const WHY_HEAP_FORK_HALF: &str =
    "deterministic-liveness: the forked child materializes its \
     own packed id bits today (fork builds both halves, not an in-place edit); a shared-buffer \
     representation would lower this floor deliberately";
/// Heap NA: allocation is not semantically forced.
pub(super) const NA_HEAP_IN_PLACE: &str =
    "may compute in place or return word-scale results: allocation \
     is not semantically forced (the process allocator itself cannot be re-routed around)";
/// Scan floor: the tick walk examines its whole input.
const WHY_SCAN_TICK_WALK: &str = "the paired fill walk examines every topology bit and payload \
     code of both operands at least once: 8 bits per input byte, with the measured tick-walk \
     constants 2–5× above";
/// Touch floor (deterministic-liveness): the kernel folds every
/// *nonzero* stored delta code through the metered accumulator.
const WHY_TOUCH_DELTA_FOLD: &str = "deterministic-liveness: the kernel folds each nonzero \
     stored delta code of its version operands through the metered accumulator today, at least \
     one digit touch per nonzero delta (a zero delta folds nothing: an accumulator add of zero \
     is a no-op); digit state moving to an unmetered representation lowers this floor \
     deliberately";
/// Touch floor (deterministic-liveness): a pair walk folds per overlay
/// boundary, and the overlay steps at least as often as the larger
/// operand's stored-delta count.
const WHY_TOUCH_PAIR_FOLD: &str = "deterministic-liveness: the fused sweep folds each overlay \
     boundary's step deltas into the one running difference accumulator today, at least one \
     digit touch per stepping boundary, and the overlay steps at least as often as the larger \
     operand stores deltas; digit state moving to an unmetered representation lowers this \
     floor deliberately";
/// Touch NA: canonical byte identity answers an equal pair before any
/// sweep runs.
const NA_TOUCH_EQUAL_PAIR: &str = "the operands are byte-identical: canonical equality answers \
     the pair before any sweep, so no accumulator fold is forced";
/// Touch floor (deterministic-liveness): the n-ary fold's first-level
/// merges each walk their two input streams' common refinement.
const WHY_TOUCH_FOLD_MERGES: &str = "deterministic-liveness: the balanced reduction's \
     first-level merges each emit over their two inputs' common refinement — at least the \
     larger input's stored-delta count per byte-distinct arrival-adjacent pair, nothing for an \
     equal pair (canonical equality answers it); later levels merge derived groups and are \
     deliberately un-floored";
/// Touch NA: no first-level merge of the reduction is forced into a fold.
const NA_TOUCH_FOLD_UNFORCED: &str = "no arrival-adjacent input pair is byte-distinct with \
     stored deltas: the reduction's first level forces no fold, and later levels merge derived \
     groups the operands do not determine";
/// Touch floor (deterministic-liveness): the rank fold lands every summand
/// in the running accumulator.
pub(super) const WHY_TOUCH_RANK_SUM: &str =
    "deterministic-liveness: the fold lands every summand in the \
     running accumulator today, at least one digit touch per summand; digit state moving to an \
     unmetered representation lowers this floor deliberately";
/// Touch NA: id trees carry no digit state at all.
pub(super) const NA_TOUCH_ID_TREE: &str =
    "id trees store no magnitudes: there is no digit state to meter";
/// Touch NA: the contract forces no accumulator fold.
pub(super) const NA_TOUCH_NOT_FORCED: &str =
    "magnitudes may be moved, hashed, or compared wholesale \
     without a running fold: no accumulator work is in the contract";
/// Touch NA: decoded rank values combine through plain big-integer
/// arithmetic (the limb column's work).
pub(super) const NA_TOUCH_RANK_ARITHMETIC: &str =
    "decoded rank values combine through big-integer \
     arithmetic the limb column prices: no accumulator is in the contract";
/// Touch NA: the renderer's summaries are delta-sized values, not a
/// running accumulator.
pub(super) const NA_TOUCH_RENDER_SUMMARIES: &str = "the renderer derives its printed bases from \
     delta-sized relative summaries without a running accumulator: no digit state is in the \
     contract (the parse direction carries the floor)";
/// Touch NA: the operand streams store no delta codes that force a fold.
const NA_TOUCH_NO_DELTAS: &str =
    "the operand streams store no fold-forcing delta codes: there is no fold to meter";
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
pub(super) const NA_TOUCH_PROJECTION: &str =
    "the projection may keep owned regions verbatim and re-base \
     boundaries through plain arithmetic: no accumulator fold is forced";

/// The decode rows' touch floor: one digit touch per 64 bits of every
/// stored code wider than the machine-word bound, or NA when every code
/// is word-scale.
///
/// This is the stream-derived convention the tick rows' limb floor uses:
/// a tree-derived floor would demand fold work no conforming validator
/// does.
pub(super) fn touch_wide_stream(v: &Version) -> Liveness {
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
pub(super) const NA_TOUCH_SEED_RAISE: &str =
    "a tick whose party owns the whole tree raises bases in \
     place through plain arithmetic: no delta fold is in the contract";
/// Touch NA: an empty version's tick is pure id-directed growth.
pub(super) const NA_TOUCH_GROW: &str =
    "the empty version's tick is id-directed growth: the grow kernel \
     runs no accumulator";

/// Scan floor (rejection rows): the defect sits at the stream's end.
pub(super) const WHY_SCAN_REJECT_END: &str = "rejection with the defect at the stream's end by \
     construction: a self-delimiting stream's truncation, trailing bits, or non-canonical \
     tail is only discoverable by parsing to it";
/// Scan floor (overlap rejection rows): the witnessing overlap sits at
/// the operands' preorder ends.
pub(super) const WHY_SCAN_OVERLAP_END: &str = "the pair's one overlapping region sits at both \
     operands' preorder ends by construction, and the packed coding has no random access: \
     any correct rejection scans to it";
/// Heap NA on rejection rows: no result is materialized.
const NA_HEAP_REJECTION: &str = "a rejecting or empty outcome materializes no result, and \
     buffering the fed stream is not semantically forced: allocation stays the \
     implementation's choice";
/// Limb NA on rejection rows: value work may be deferred past the defect.
pub(super) const NA_LIMB_REJECTION: &str = "rejection forces no value materialization: a strict \
     validator may defer magnitude work past the walk that finds the defect";
/// Touch NA on rejection rows: no accumulator fold is forced.
pub(super) const NA_TOUCH_REJECTION: &str =
    "rejection forces no accumulator fold: digit-state work \
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
pub(super) fn rejection_floors(fed_bytes: usize, why: &'static str) -> Floors {
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
pub(super) fn id_rejection_floors(fed_bytes: usize, why: &'static str) -> Floors {
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
pub(super) fn clock_overlap_floors(id_bytes: usize) -> Floors {
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
pub(super) fn text_rejection_floors(limb: Liveness, touch: Liveness) -> Floors {
    Floors {
        heap: na(NA_HEAP_REJECTION),
        limb,
        segments: seg_ceiling_only(),
        scan: na(NA_SCAN_TEXT_REJECTION),
        touch,
    }
}

/// A delta-fold touch floor over `deltas` *nonzero* stored delta codes
/// (pass [`stored_nonzero_deltas`]), or NA when the operand streams
/// store none: the single-operand kernels' premise (every nonzero
/// stored delta of the one stream is folded individually; zero deltas
/// fold nothing, so a count that included them would demand touch work
/// no conforming fold does).
pub(super) fn touch_delta_fold(deltas: u64) -> Liveness {
    if deltas == 0 {
        na(NA_TOUCH_NO_DELTAS)
    } else {
        Liveness::Floor {
            min: deltas,
            why: WHY_TOUCH_DELTA_FOLD,
        }
    }
}

/// The pair-walk touch floor: one accumulator touch per overlay boundary
/// at which either operand's stream steps.
///
/// Derived from the fused sweep's mechanism (every two-operand
/// comparison, merge emission, and pair query rides it): the walk visits
/// each boundary of the two streams' common refinement and folds that
/// boundary's step deltas into the one running difference accumulator,
/// so a boundary both operands step lands both codes in a single fold.
/// The naive two-stream delta sum therefore over-counts a
/// boundary-aligned pair by ×2 — the tooth-tail family is the committed
/// demonstration (same-shape operands, every boundary shared, measured
/// ~one touch per boundary) — while the *larger* operand's stored-delta
/// count is sound for every pair, aligned or not: each of its deltas
/// marks a distinct stepping boundary of the overlay. The floor stays
/// strictly positive wherever either operand stores a delta at all, so a
/// dead touch meter still trips it on every committed pair family. Equal
/// operands are answered by canonical byte identity before any sweep
/// runs (`a ∨ a = a`, `a ∧ a = a`, ordering by equality), so they force
/// no fold and declare NA.
pub(super) fn touch_pair_fold(v: &Version, w: &Version) -> Liveness {
    if v == w {
        return na(NA_TOUCH_EQUAL_PAIR);
    }
    let boundaries = stored_deltas(v).max(stored_deltas(w));
    if boundaries == 0 {
        na(NA_TOUCH_NO_DELTAS)
    } else {
        Liveness::Floor {
            min: boundaries,
            why: WHY_TOUCH_PAIR_FOLD,
        }
    }
}

/// The n-ary join fold's touch floor: what the balanced reduction's
/// first level alone forces.
///
/// The binary-counter reduction merges arrival-adjacent inputs first,
/// and each such merge is a pair walk over its two input streams
/// ([`touch_pair_fold`]'s premise): at least the larger input's
/// stored-delta count for a byte-distinct pair, nothing for an equal
/// pair (canonical equality answers it without a sweep) or an unpaired
/// tail input. Later levels merge *derived* groups whose streams the
/// operands do not determine cheaply, so they are deliberately
/// un-floored — the declaration is the sound first-level sum, strictly
/// positive on every committed fold population.
pub(super) fn touch_fold_first_merges(versions: &[Version]) -> Liveness {
    let first_level: u64 = versions
        .chunks(2)
        .map(|pair| match pair {
            [a, b] if a != b => stored_deltas(a).max(stored_deltas(b)),
            _ => 0,
        })
        .sum();
    if first_level == 0 {
        na(NA_TOUCH_FOLD_UNFORCED)
    } else {
        Liveness::Floor {
            min: first_level,
            why: WHY_TOUCH_FOLD_MERGES,
        }
    }
}

/// A full-examination scan floor over `packed_bytes` of operand.
pub(super) fn scan_examines(packed_bytes: usize) -> Liveness {
    Liveness::Floor {
        min: (packed_bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64,
        why: WHY_SCAN_EXAMINES,
    }
}

/// The early-exit scan floor: the root codes are always read.
pub(super) fn scan_touch() -> Liveness {
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
pub(super) fn limb_stream(mandatory_limbs: u64) -> Liveness {
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
pub(super) fn limb_wide(mandatory_limbs: u64) -> Liveness {
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
pub(super) fn heap_materializes(packed_bytes: usize) -> Liveness {
    Liveness::Floor {
        min: packed_bytes as u64,
        why: WHY_HEAP_MATERIALIZES,
    }
}

/// The fork rows' heap declaration: the child clock's version copy floors
/// the heap at the version's whole stored bytes, or NA when the version is
/// word-scale (the id-pair cross forks around an empty version).
pub(super) fn heap_fork_child(version: &Version) -> Liveness {
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
pub(super) fn na(reason: &'static str) -> Liveness {
    Liveness::NotApplicable { reason }
}

/// Segments NA: the policy declaration every cell carries on the segments
/// currency.
const NA_SEG_CEILING_ONLY: &str = "ceiling-only by policy: the target is walks that never grow \
     the stack, so the honest floor is zero and a zero floor asserts nothing";

/// The segments currency's declaration: ceiling-only by policy, on every
/// cell.
pub(super) fn seg_ceiling_only() -> Liveness {
    na(NA_SEG_CEILING_ONLY)
}

/// The floors of the many rows that must walk their operands but are forced
/// into neither allocation nor arithmetic: scan floored, heap and limb NA.
///
/// The touch declaration is the caller's: each walk row answers the
/// accumulator question for its own kernel.
pub(super) fn walk_floors(packed_bytes: usize, touch: Liveness) -> Floors {
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
/// A *distinct* comparable pair must be walked to the end — certifying
/// dominance means checking every region — so the full-examination scan
/// floor and the per-overlay-boundary touch floor
/// ([`touch_pair_fold`]'s premise) bind; an equal pair is answered by
/// canonical byte identity before any sweep, so neither does; a
/// concurrent pair may be decided at one witness divergence per
/// direction, so only the root-codes scan floor does.
pub(super) fn comparison_floors(v: &Version, w: &Version, packed_bytes: usize) -> Floors {
    if v == w {
        return Floors {
            heap: na(NA_HEAP_IN_PLACE),
            limb: na(NA_LIMB_NOT_FORCED),
            segments: seg_ceiling_only(),
            scan: na(NA_SCAN_EQ_BYTES),
            touch: na(NA_TOUCH_EQUAL_PAIR),
        };
    }
    if v.partial_cmp(w).is_some() {
        walk_floors(packed_bytes, touch_pair_fold(v, w))
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
/// and one accumulator touch per overlay boundary at which either event
/// stream steps ([`touch_pair_fold`]'s premise — the projected sweep
/// rides the same fused walk, so an aligned pair honestly folds both
/// step codes per boundary at once; the id streams store no deltas). A
/// concurrent pair may exit at its witnessing divergences, so only the
/// root-code scan floor binds.
pub(super) fn masked_cmp_floors(
    verdict: &Option<Ordering>,
    v: &Version,
    w: &Version,
    packed_bytes: usize,
) -> Floors {
    if verdict.is_some() {
        walk_floors(packed_bytes, touch_pair_fold(v, w))
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
pub(super) fn tick_walk_floors(version: &Version, packed_bytes: usize) -> Floors {
    Floors {
        heap: na(NA_HEAP_IN_PLACE),
        limb: limb_stream(mandatory_limbs_stream(version)),
        segments: seg_ceiling_only(),
        scan: Liveness::Floor {
            min: (packed_bytes as u64).saturating_mul(TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE),
            why: WHY_SCAN_TICK_WALK,
        },
        touch: touch_delta_fold(stored_nonzero_deltas(version)),
    }
}
