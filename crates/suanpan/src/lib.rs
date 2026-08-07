//! Cliff-immune signed accumulators: redundant balanced signed digits
//! with no carry cliffs anywhere — machine-word deltas and sign reads
//! amortized O(1), wide deltas amortized O(operand limbs), on every input
//! sequence.
//!
//! [`Accumulator`] holds a running signed integer — a running total, a
//! running difference of two totals, a running weighted sum — under
//! interleaved adds, subtracts, and sign reads:
//!
//! ```
//! use core::cmp::Ordering;
//! use suanpan::{Accumulator, UBig};
//!
//! let mut acc = Accumulator::new();
//! acc.add_wide(&(UBig::from(1u8) << 512usize)); // park a wide total on a carry boundary
//! for _ in 0..1_000 {
//!     acc.sub_small(1);                         // oscillate across it: amortized O(1) each
//!     assert_eq!(acc.sign(), Ordering::Greater);
//!     acc.add_small(1);
//! }
//! let (sign, magnitude) = acc.sign_magnitude(); // one carry pass, at the very end
//! assert_eq!(sign, Ordering::Greater);
//! assert_eq!(magnitude, UBig::from(1u8) << 512usize);
//! ```
//!
//! Every cost this page quotes holds on adversarial input sequences —
//! the amortized bounds are worst-case over the whole sequence, not
//! average-case claims — and every one is *derived*: the three arguments
//! that carry them (the lazy zone, the collapsing sign fold, the
//! zero-run ledger) are below, in full.
//!
//! # The problem: carry cliffs
//!
//! Keep a running total in a normalized big integer and park its value at
//! `2^k − 1`. Adding 1 then subtracting it back, over and over, propagates
//! a full k-bit carry and then a full k-bit borrow per pair: Θ(k) limb
//! work bought by O(1) bits of delta — and when the stream itself built
//! the k-bit total, quadratic in the stream's length. The cliff is not a
//! quirk of one library; it is the price of *normal form*. A normalized
//! representation spells each value exactly one way, so two values that
//! differ by 1 can differ in every digit — and a type that must always
//! hold the normal spelling must pay the full rewrite every time a small
//! delta crosses a carry boundary. Any workload whose deltas mix signs
//! near such a boundary inherits the cost; an adversarial workload seeks
//! it out.
//!
//! # The representation
//!
//! Value storage is two-tier. Every accumulator begins in the *quick
//! register*: the exact value in one `i128`, held while it and every
//! operand stay word-scale (magnitudes to `2^96`, shifts to 30 bits) —
//! there, an add is one machine addition and a sign read one
//! comparison. The first wide operand, wide shift, or outgrown sum
//! spills the register into the digit representation below, once per
//! [`reset`](Accumulator::reset) epoch and at O(1) cost. The spill is
//! one-way, so the register is not the two-zone design rejected later
//! in this section: there is no boundary a delta stream can oscillate
//! across — crossing it retires it — and every amortized bound below
//! holds with the register in front, since register operations are
//! exact and O(1). The digit representation is the accumulator's
//! load-bearing tier:
//!
//! An accumulator stores little-endian signed digits `dᵢ: i64` denoting
//! `value = Σ dᵢ · 2^(32·i)`, each digit kept in the *lazy zone*
//! `|dᵢ| < 2^33` — twice the digit base, and symmetric about zero. The
//! representation is *redundant*: a value has many spellings, no
//! operation requires the normal one, and nothing eagerly normalizes. It
//! is *balanced*: digits carry their own signs, so a subtraction is just
//! a negated addition and no borrow machinery exists.
//!
//! Every deposit a write makes lands in one digit (a machine-word delta
//! is one deposit; a wide delta makes one per limb), forming the sum `t`
//! in wider (128-bit) intermediate arithmetic so nothing overflows. If
//! `t` is in the zone, it becomes the digit and that is the whole write.
//! If not,
//! the digit *recenters*: it carries `c = (t + 2^31) >> 32` upward (an
//! arithmetic shift) and keeps the remainder `t − c·2^32`, which lands in
//! `[−2^31, 2^31)`. Two facts make this cheap \[derived\]: a freshly
//! recentered digit must absorb at least `2^33 − 2^31` of further net
//! inflow before it can carry again, and a carry chain attenuates fast —
//! the first carry out of a word-scale write is at most about `2^32`, and
//! the next is already a handful of units, tiny against the inflow the
//! digit above needs before it carries on. So sustained carry traffic
//! thins out geometrically with height, and the total carry work is
//! dominated by the deltas that entered below. The write bounds are
//! amortized: a single call can be caught repaying a run of digits that
//! earlier writes parked near the zone's edge, but never more than those
//! writes prepaid — over any sequence, total digit work stays O(1) per
//! machine-word call and O(operand limbs) per wide call.
//!
//! Machine-word deltas are therefore amortized O(1) digit work. A wide
//! delta enters limb by limb — throughout this page a *limb* is one
//! 64-bit word of the operand's value, independent of the backend's
//! internal word size — each limb landing as two contributions at the
//! digit positions it spans, for amortized O(operand limbs) total:
//! independent of how wide the *held* value is, and of any power-of-two
//! shift applied on the way in.
//!
//! Because *every* write recenters, no region of the representation is
//! ever kept in normal form — hence no boundary an adversarial delta
//! stream can oscillate across at less than the cost the stream itself
//! paid, at any delta width. The obvious halfway design fails exactly
//! there: a two-zone form (a normalized prefix plus a fixed-width lazy
//! window over the low digits) has a boundary at the window's top, and a
//! stream of deltas one digit wider than the window forces the normalized
//! prefix through a full carry per delta. Widening the window moves the
//! boundary; only having no normalized region removes it.
//!
//! # Reading the sign
//!
//! The sign of a redundant value is not visible in any one digit — high
//! digits may cancel lower ones. [`Accumulator::sign`] folds digits from
//! the top: at digit index `i` the running partial
//! `s = Σ_{j≥i} dⱼ · 2^(32·(j−i))` is the scanned suffix's exact value in
//! units of `2^(32·i)`, while the unscanned digits below contribute less
//! than `2.01 · 2^(32·i)` in magnitude (a geometric series — each digit
//! under `2^33`, each level down worth `2^32` times less — summing to
//! just over `2 · 2^(32·i)`; `2.01` is that bound rounded up for slack).
//! So once `|s| ≥ 3`, the suffix dominates everything below —
//! `3 > 2.01` — and the fold stops. While `|s| < 3` it must descend, but
//! the partial stays small enough for machine arithmetic at every step,
//! and if it reaches digit 0 the partial *is* the value, exactly.
//!
//! A cancelling prefix — high digits summing to a tiny net value, as built
//! by `+2^k` then `−(2^k − 1)` — forces the fold below the top digit. The
//! fold therefore *collapses* what it scanned: the scanned digits are
//! zeroed and their exact partial is re-deposited at the scan's floor
//! (recentering upward like any write when the partial exceeds the zone),
//! so the next sign read re-reads none of them — the re-deposited digit is
//! that next fold's first step, inside its O(1) budget. A digit is scanned
//! at most once per write that made it nonzero, so sign reads amortize
//! against the writes that built the prefix — amortized O(1) however sign
//! reads and writes interleave. This is why the sign queries take
//! `&mut self`: they may rewrite the representation. The rewrite is always
//! value-preserving — the digits change, the integer they denote never
//! does.
//!
//! # The zero-run ledger
//!
//! Keeping the top digit index exact has a scan to pay: when a write
//! zeroes the highest nonzero digit, the new top is the next nonzero
//! digit below, and something must find it. Between a shifted write's
//! landing site and the digits below it lies a run of never-written
//! zeros; a scan that walked it would do work no operand limb funded,
//! and an alternating pair of shifted writes would make it walk again,
//! forever, at a price that grows with the shift. The shifted rows of
//! the cost table are true only because that walk never happens.
//!
//! The accumulator instead keeps a *zero-run ledger*: certificates
//! `(lo, hi)`, each stating that every digit strictly between `lo` and
//! `hi` is zero. A write that lands above the current top leaves
//! exactly one such run behind and records it — one O(1) entry,
//! whatever the run's width. A scan that reaches a certified run
//! consumes the certificate and skips to `lo` whole, one touch instead
//! of one per digit; the sign fold does the same when its running
//! partial is zero (a nonzero partial decides within one step, so a
//! fold never walks into a certified run while carrying value). A
//! write whose carries land inside a certified run splits the
//! certificate around the digits actually written, keeping both
//! remnants.
//!
//! The amortization is a potential argument over the ledger
//! \[derived\]: at every moment, every digit position at or below the
//! top is either inside some certificate's run or funded by one scan
//! credit deposited by the metered write that most recently touched
//! it. A plain scan step spends the credit at its position; a skip
//! consumes a certificate; each certificate is created once, by the
//! write that jumped the run, and consumed at most once. For a scan to
//! reach a position twice, the top must rise back above it in between,
//! and each way it can — a carry run writing through the position, or
//! a write jumping over it and recording a fresh certificate — re-arms
//! the accounting. So top maintenance never exceeds the metered work
//! that funded it: amortized O(1) per write beyond the write's own
//! deposits, at any shift, on any schedule.
//!
//! The ledger itself is bookkeeping, not digit work: certificates live
//! in an ordered map costing O(log ledger size) machine-word
//! operations per write, never counted as digit touches and never
//! reading or writing a digit. Disjoint runs cap the ledger at half
//! the held digit positions, so its memory is O(held digits) — the
//! digit buffer's own order, at a few machine words per certificate
//! where a digit costs one.
//!
//! # Domination certificates
//!
//! A comparison between totals of wildly different scales should not cost
//! the wide one's width. [`Accumulator::sign_dominates_at`] returns the
//! (always exact) sign of the held value `v`, plus a *certificate*:
//! `decided = true` guarantees `sign(v + a) = sign(v)` and `|v| > |a|`
//! for every adjustment `a` with `|a| < 2^(32·(floor + 1))` — and
//! moreover for any accumulator held in digits `0..=floor`: its
//! redundant spelling can exceed that, bounded by
//! `2.01 · 2^(32·(floor + 1))` (the same rounded geometric bound), and
//! the decision margin covers that too. So the caller compares against
//! anything at or below the floor's scale without ever folding it in:
//!
//! ```
//! use core::cmp::Ordering;
//! use suanpan::{Accumulator, UBig};
//!
//! let mut watermark = Accumulator::new();
//! watermark.add_wide(&(UBig::from(1u8) << 300usize));
//! // Could any adjustment below 2^128 flip the watermark's sign?
//! // floor = 128.div_ceil(32) - 1 = 3: certainty without a wide fold.
//! let (sign, decided) = watermark.sign_dominates_at(3);
//! assert_eq!((sign, decided), (Ordering::Greater, true));
//! ```
//!
//! For `u64`-scale adjustments,
//! [`sign_dominates_word`](Accumulator::sign_dominates_word) is the
//! shorthand.
//!
//! # The operations
//!
//! All costs in digit touches, derived above; `|x|` is the size of `x`
//! in bytes — the operand `|delta|`/`|other|`, or the accumulator's own
//! held digits `|self|` — so the limb-denominated derivations above
//! read as `|x|` up to the constant word width. *Amortized* bounds hold
//! over the whole operation sequence — one write can be caught repaying
//! carries that earlier writes parked near the zone's edge, never more
//! than they prepaid; unmarked rows are worst-case per call.
//!
//! | Operation | Cost |
//! |---|---|
//! | [`add_small`](Accumulator::add_small), [`sub_small`](Accumulator::sub_small), [`add_u64`](Accumulator::add_u64), [`sub_u64`](Accumulator::sub_u64) | amortized O(1) |
//! | [`add_wide`](Accumulator::add_wide), [`sub_wide`](Accumulator::sub_wide) | amortized O(\|delta\|), whatever the held width |
//! | [`add_wide_shl`](Accumulator::add_wide_shl), [`sub_wide_shl`](Accumulator::sub_wide_shl) | amortized O(\|delta\|), independent of the shift |
//! | [`add_u64_shl`](Accumulator::add_u64_shl), [`sub_u64_shl`](Accumulator::sub_u64_shl) | amortized O(1), independent of the shift |
//! | [`add_magnitude`](Accumulator::add_magnitude), [`sub_magnitude`](Accumulator::sub_magnitude) | word-scale: amortized O(1); wide: amortized O(\|delta\|) |
//! | [`add_magnitude_shl`](Accumulator::add_magnitude_shl), [`sub_magnitude_shl`](Accumulator::sub_magnitude_shl) | as [`add_magnitude`](Accumulator::add_magnitude)/[`sub_magnitude`](Accumulator::sub_magnitude), at any shift |
//! | [`add_accum`](Accumulator::add_accum), [`sub_accum`](Accumulator::sub_accum) | amortized O(\|other\|) |
//! | [`add_accum_shl`](Accumulator::add_accum_shl), [`sub_accum_shl`](Accumulator::sub_accum_shl) | amortized O(\|other\|), independent of the shift |
//! | [`merge_into_wider`](Accumulator::merge_into_wider) | amortized O(min(\|self\|, \|other\|)) |
//! | [`sign`](Accumulator::sign), [`is_negative`](Accumulator::is_negative), [`sign_dominates_word`](Accumulator::sign_dominates_word), [`sign_dominates_at`](Accumulator::sign_dominates_at) | amortized O(1) |
//! | [`is_literally_zero`](Accumulator::is_literally_zero) (one-sided: `true` means zero, `false` means unknown), [`digit_count`](Accumulator::digit_count) | O(1) |
//! | [`shl`](Accumulator::shl), [`negate`](Accumulator::negate), [`reset`](Accumulator::reset), [`sign_magnitude`](Accumulator::sign_magnitude) | O(\|self\|) |
//! | [`sign_magnitude_shl`](Accumulator::sign_magnitude_shl) | O(w), w the written span since the last reset |
//!
//! Digit touches are shift-independent; memory is not. A shifted entry
//! point grows the digit buffer to cover the shifted position, so memory
//! is O(shift / 32) plus the operand's own digits (the zero-run ledger
//! adds at most one entry per write that lands above the held top,
//! bounded by half the held digit positions). The *written span* is
//! every digit from the lowest position written since the last reset up
//! to the top, never-written gaps between writes included: parking one
//! value far above another prices the scaled read at the distance
//! between them, however few digits the writes themselves touched.
//!
//! The `*_magnitude` entry points are generic over [`Magnitude`], the seam
//! for a caller's own stored-magnitude type: the operand reports whether
//! it fits a machine word, and the accumulator dispatches to the small or
//! wide path accordingly. There is no from-value constructor: build with
//! [`new`](Accumulator::new) (or `Default`) and a single `add_*` call,
//! read out with [`sign_magnitude`](Accumulator::sign_magnitude).
//!
//! # When not to reach for it
//!
//! The accumulator spends representation slack to buy worst-case bounds;
//! when nothing exploits the slack, simpler types win. If the total fits
//! `i64`/`i128`, use `i64`/`i128`. If the deltas never change sign, a
//! plain big integer is already amortized O(1) per delta (the binary
//! counter argument: each carry clears a bit an earlier increment set, so
//! carries never outnumber increments) and needs no slack. The
//! accumulator earns its keep when deltas mix signs — when the total can
//! be driven onto a carry boundary and oscillated — or when sign reads
//! interleave with cancelling updates. And this is an accumulator, not a
//! number type: it adds, subtracts, scales by powers of two (left only —
//! a right shift would need normalization), reads its sign, and converts
//! out through [`sign_magnitude`](Accumulator::sign_magnitude) — no
//! multiplication, no division, and no ordering between two accumulators
//! except by subtracting one from the other and reading the difference's
//! sign (subtract from a [`clone`](Clone::clone) when the receiver's
//! value must survive the comparison) — or, when the scales differ
//! wildly, a domination certificate
//! ([`sign_dominates_at`](Accumulator::sign_dominates_at) with
//! `floor = other.digit_count() - 1`) that decides without folding.
//!
//! # Metering
//!
//! The `touch-meter` feature counts every digit read-modify-write (plus
//! one per operand limb read by a wide operation, and one per zero digit
//! a top-settlement scan steps or skips past — a certificate skip is one
//! touch however wide the certified run, because the run's digits are
//! neither read nor written) into the [`touch_meter`] module's
//! process-global counter. The quick register holds no digits, yet its
//! work is metered too: a delta, sign query, negation, or shift the
//! register absorbs counts exactly one touch, a register read-out
//! ([`sign_magnitude`](Accumulator::sign_magnitude) and its scaled twin)
//! counts the value's [`digit_count`](Accumulator::digit_count), and the
//! spill prices only the deposit of the register's few digits — so
//! touch-count floors derived from the digit engine's shapes survive
//! the register fast path. The counts are **exact**: for a fixed
//! operation sequence the reading is a deterministic function of that
//! sequence, and this exactness is a public contract — a change to any
//! operation's count is a breaking change of this crate, never
//! measurement noise. Digit-touch cost is invisible to heap meters
//! and step counters — the work is wider, not more frequent — so this
//! counter is what a caller's resource envelopes should pin; the
//! zero-run ledger's own upkeep is machine-word bookkeeping outside the
//! digit denomination (its bound is stated in the ledger section). Off
//! by default, and without the feature the module is absent and the
//! counting compiles to nothing; with it, each touch is one relaxed
//! atomic increment.
//!
//! # Interop
//!
//! [`UBig`] is `dashu_int::UBig` (compiled against `dashu-int` 0.5;
//! bumping that dependency is a breaking change to this crate's API),
//! re-exported so callers can name exactly the type this crate compiled
//! against. The crate requires `std`; no `no_std` build is offered.
//! [`Accumulator`] is `Clone`, `Default`, `Debug`, and `Send + Sync` —
//! though `Sync` buys less than usual: every amortized-O(1) sign query
//! takes `&mut self`, so the value reads available behind a shared
//! reference are [`is_literally_zero`](Accumulator::is_literally_zero),
//! [`digit_count`](Accumulator::digit_count), the O(held digits)
//! [`sign_magnitude`](Accumulator::sign_magnitude) (and its scaled twin
//! [`sign_magnitude_shl`](Accumulator::sign_magnitude_shl)), and a
//! [`clone`](Clone::clone) — wrap in a lock for shared sign reads. It
//! is deliberately not `PartialEq`: two spellings of one value would
//! compare unequal, so compare by subtracting and reading the
//! difference's sign. `touch-meter` is the crate's only feature.
//!
//! # Testing
//!
//! Differential proptests drive mixed small/wide operation streams
//! against an exact signed big-integer oracle, comparing the sign after
//! every operation and the full value at periodic snapshots; deterministic
//! adversarial streams pin the shapes the representation exists to
//! survive — the boundary comb (a ±1 oscillation parked on a `2^k` carry
//! boundary), wide teeth (±2^w strides across a higher boundary),
//! cancelling-prefix chains (repeated falls from `2^k` to 1 and back,
//! each forcing the sign fold below the top digit), and alternating
//! shifted pairs (a one-limb value blinking on and off far above every
//! other written digit, the schedule whose top maintenance the zero-run
//! ledger prices).
//!
//! # Traditions, and the name
//!
//! Nothing here is novel so much as assembled. Signed-digit redundancy is
//! Avizienis (1961), and it is the trick inside hardware carry-save
//! adders: spend representation slack, defer carry propagation. Redundant
//! *number representations* as an amortization device are the theme of
//! Okasaki's purely functional data structures. Accumulating wide addends
//! at their own offsets into a fixed-radix array is the Kulisch long
//! accumulator. And unsaturated-limb big-integer pipelines in
//! cryptographic code leave headroom bits in every limb so carries can
//! batch. The representation, in short, is known technique that no
//! package ships as a reusable accumulator — that absence is why this
//! crate exists. The query layer is the novel part: a sign read over a
//! redundant value that pays for itself by collapsing the prefix it
//! scanned, and domination certificates that answer cross-scale
//! comparisons without folding either side.
//!
//! A *suanpan* is the Chinese abacus. Each rod carries two heaven beads
//! (worth five) and five earth beads (worth one): a rod holds 0–15,
//! though a decimal digit needs only 0–9. The slack is the point — a
//! skilled operator parks intermediate values in the redundant range and
//! defers carries until a convenient moment. The Japanese soroban keeps
//! one heaven and four earth beads — a rod holds exactly 0–9, no slack —
//! which is exactly the normalization this crate refuses. A suanpan rod
//! holds more than a digit so the carries can wait; so do ours.

#![forbid(unsafe_code)]

mod accumulator;
mod limbs;
mod magnitude;
#[cfg(feature = "touch-meter")]
pub mod touch_meter;

pub use dashu_int::UBig;

pub use accumulator::Accumulator;
pub use limbs::Limbs;
pub use magnitude::Magnitude;

#[cfg(test)]
mod claims;
