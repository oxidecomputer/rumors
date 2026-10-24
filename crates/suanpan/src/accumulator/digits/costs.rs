//! Derives the crate-level time and space bounds from the digit and
//! written-set implementation.
//!
//! The crate documentation promises a cost for every operation. This module
//! holds no code: it records the argument behind each promise, so that a
//! change to [`Digits`](super::Digits) or to
//! [`WrittenPositions`](super::written::WrittenPositions) can be checked
//! against the argument it must preserve. If you change how a write, a scan,
//! or the written set spends its work, find the step below that charges that
//! work and confirm that it still holds.
//!
//! # What is counted
//!
//! Time counts word operations: reading or writing a digit or a bitset word,
//! and each digit or word that growing a buffer fills with zeros or copies. An
//! allocator call counts as one operation, and releasing an allocation is
//! charged to the call that made it. Debug assertions are not counted; some of
//! them query the written set.
//!
//! Positions are digit positions of 32 bits. The public quantities count bits
//! (`V`, `W`, `A`, `G`, `Q`, `S`) or 64-bit limbs (`L`), which differ from
//! position counts by constant factors. The *top* is `highest_nonzero`. A
//! *deposit* is one call of [`add_at`](super::Digits::add_at); every update
//! makes its deposits at ascending positions. The *run*, the *prefix*, and the
//! *bitset* are the parts of a written set that the
//! [`written`](super::written) module describes: the run holds the highest
//! consecutive positions, and the prefix or the bitset holds the positions
//! below it.
//!
//! # The potential
//!
//! An amortized cost is an operation's actual cost plus the change it causes
//! in a *potential*: a nonnegative function of the stored state that is zero
//! for a new accumulator. Over any sequence of operations, the amortized costs
//! then sum to at least the actual costs (Tarjan's potential method). The
//! potential of a program is the sum over its accumulators, so an operation
//! that reads another accumulator may spend that accumulator's potential. Each
//! accumulator's potential is a weighted sum of four terms, and each term pays
//! for one kind of deferred work.
//!
//! 1. *Carry potential* is the sum of `max(0, |d| - B/2) / B` over the stored
//!    digits `d`, with `B = 2^32`. A digit never holds more than 3/2 of it,
//!    because `|d| < 2B`. It pays for carry chains.
//! 2. *Held positions* is the number of positions in the written set. Each
//!    position pays for one later move from the run to the positions below
//!    it, one move from a prefix into a new bitset, and one visit and removal
//!    by a descending scan or by a clear. Each happens at most once per
//!    insertion: a position below the run never rejoins a run, a set
//!    converts its prefix once, and a removed position returns only by being
//!    written again.
//! 3. *Nonempty words* is the number of nonzero words, at every level, of the
//!    bitset. It pays for a removal's climb, for the climbs of predecessor
//!    queries from positions that the scan then removes (see "Bitset climbs"
//!    below), and for the words a clear empties (see "Clears").
//! 4. *Uncovered height* is `max(0, T - C) / 64`, where `T` is the highest
//!    position written since the written set was created, and `C` is the
//!    number of positions the bitset's storage covers, zero without a bitset.
//!    It pays for growing the bitset's storage.
//!
//! # Carries
//!
//! A deposit's contribution has magnitude at most `2^96 = B^3`. Each carry is
//! the nearest integer to its position's total over `B`, and every stored
//! digit has magnitude below `2B`. So carries shrink by a factor of `B` per
//! position, and every carry out of the third position or later has
//! magnitude at most 3. A carry of magnitude at most 3 passes a digit `d` only
//! when `|d| >= 2B - 3`, and recentering then leaves `|d| <= B/2`. That
//! digit's carry potential falls from at least `3/2 - 3/B` to zero, which pays
//! for the step. At most four other positions per deposit, the first three
//! and the last, each raise the potential by at most 3/2. A comparison's
//! write-back has magnitude below `4B` and behaves the same way. Each deposit
//! therefore costs amortized O(1) digit work.
//!
//! # Where updates write
//!
//! An update of `n` deposits writes only positions within O(`1 + log_B n`) of
//! the higher of the receiver's top when it starts and its top when it ends.
//! Together with "Storage growth" below, this is why `G` bounds the storage
//! an update adds, apart from a term its own deposits pay for.
//!
//! Let `H` be the higher of the receiver's starting top `t` and the update's
//! highest deposit position `h`. Every digit above `H` starts at zero and
//! changes only by carries. A carry into a position three or more past its
//! chain's deposit has magnitude at most 3, so a position three or more past
//! `H` carries out only after absorbing about `B / 2` carries since it last
//! carried, at most one from each chain. The writes past `H` therefore reach
//! O(`1 + log_B n`) positions. It remains to show that a deposit far above
//! `t` leaves the result's top near it, so that `H` lies within a constant of
//! the higher top.
//!
//! - A primitive deposits at positions up to 3.
//! - A limb stream's contributions all have one sign. If its highest nonzero
//!   piece lands at `h >= t + 3`, the result's magnitude is at least
//!   `B^h - 2.01 B^(t + 1) > B^(h - 1)`. A value whose top is `t'` has
//!   magnitude below `2.01 B^(t' + 1)`, so the result's top is at least
//!   `h - 2`.
//! - An accumulator operand is first compared with zero and skipped if zero.
//!   The comparison leaves a top digit, at position `i`, of magnitude at
//!   least [`COMPARISON_DECIDED`](super::sign::COMPARISON_DECIDED), or of
//!   magnitude at least 1 over a digit of magnitude at most `B/2`, or at
//!   position zero. In each case the operand's magnitude is at
//!   least `B^i / 3`, and the same argument places the result's top within a
//!   constant of the operand's shifted top.
//!
//! # Bitset climbs
//!
//! In the prefix form, every insertion, query, and truncation of the written
//! set costs O(1). Climbs arise only in the bitset form, whose module
//! documents how insertion, removal, and predecessor queries climb its levels.
//! A word at level `k` covers `64^(k + 1)` positions, so two positions `y < q`
//! lie in different words at level `k` exactly when a multiple of
//! `64^(k + 1)` lies in `(y, q]`. Write `β(y, q)` for the number of levels at
//! which they do. Three facts about climbs, and one about their sums, carry
//! the argument.
//!
//! First, the bitset holds position zero from its creation on. While the
//! buffer holds digits, the set holds position zero, so when a prefix
//! converts, either the prefix holds zero or the run starts at zero. The run
//! starts at zero only while nothing lies below it, and then a jump folds the
//! run into the prefix instead of converting it, so every conversion moves a
//! prefix that holds zero. A truncation removes only positions above another
//! position, and a clear keeps zero, so zero stays. Word zero is therefore
//! nonempty at every level, and an insertion or removal at position `p` climbs
//! at most `1 + log_64 (p + 1)` levels before it reaches a word it shares with
//! position zero.
//!
//! Second, an insertion climbs at most one level more than the number of
//! words it turns from empty to nonempty, and a removal at most one more than
//! it turns from nonempty to empty, because each also reads the word where it
//! stops. Removals are thus paid by the nonempty-words term. An insertion of
//! `q` stops no higher than the word holding the nearest held position `y`
//! below `q`, so it costs O(`1 + β(y, q)`), which whoever inserts pays.
//!
//! Third, a predecessor query from `q` whose answer is `y` stops climbing by
//! the first level at which `y` lies in `q`'s word, because by level `k` it
//! has searched all of `q`'s level-`k` word below `q`. So it costs
//! O(`1 + β(y, q)`); the bitset module also bounds it by
//! O(`1 + log_64 (q - y)`). Suppose `q` is held, and the operation then
//! truncates the set where its scan or descent stops, which removes `q` and
//! everything above it. At each of the `β(y, q)` levels where `y` lies in a
//! different word, `q`'s word then holds nothing: its positions all lie above
//! `y`, and none between `y` and `q` was held. That word held `q`, so the
//! removal that emptied it pays for the query's visit too. Distinct queries
//! of one scan count distinct words: each later query starts at or below an
//! earlier answer `y`, which lies in a different word from that query's `q`
//! at every level the earlier query counts.
//!
//! The exception is a *run crossing*: a query from the start of a run above
//! position zero, which the bitset does not hold, whether the run holds
//! positions or a truncation has left it empty. It can climb through words
//! that never held anything. A run crossing from `q` costs
//! O(`1 + β(y, q)`), at most O(`1 + log_64 (q + 1)`) by the first fact. It
//! happens at most twice per truncation: once in the scan or descent, and
//! once in the truncation itself. A run that starts at zero, as a clear
//! leaves it, needs no crossing: every query from above zero stops in the
//! run, and nothing lies below it.
//!
//! Finally, climbs telescope. Suppose intervals `(y_i, q_i]` lie within
//! `[a, p]` and cover each position at most `c` times. An interval of length
//! `p - a` holds at most `(p - a) / 64^(k + 1) + 1` multiples of
//! `64^(k + 1)`, and none for a level with `64^(k + 1) > p`. Each multiple
//! lies in at most `c` of the intervals, so summing over the levels gives
//! `Σ β(y_i, q_i) <= c ((p - a) / 63 + log_64 (p + 1))`.
//!
//! A bitset covering `C` positions has O(`1 + log_64 (C + 1)`) levels. It
//! covers at most one word past the highest position it has held, which lies
//! at or below `T`. By "Where updates write", `T` exceeds the greatest top the
//! receiver has reached by O(`1 + log_B n`) for some update of `n` deposits,
//! and `n` is at most that top plus three, because the deposits lie at
//! distinct positions no higher than two above it. So every climb, and the
//! number of levels, is O(log(`W` + 1)).
//!
//! # Storage growth
//!
//! Growing the digit buffer to position `p` fills the new positions with
//! zeros. The crate's `G` counts growth past the greatest working width
//! reached in the receiver's *history*, the time since its storage was
//! created or last received `clone_from`, and since its last nonzero `<<=`;
//! call that span `G`'s *window*. The buffer never shrinks within a window:
//! `clone_from` shortens it to the copy, and a shift of a value held in
//! digits replaces it. The public list of events is deliberately broader
//! than these: a shift of a small value keeps its storage, but counting it
//! only shortens the window, which can only enlarge `G`, and it keeps the
//! definition in terms a caller can see. So the buffer already reaches the
//! greatest position written in the window, and an update's writes reach
//! past the higher of the window's greatest top and its own ending top by
//! O(`1 + log_B n`). The update's `G` and its `n` deposits therefore pay for
//! the growth.
//!
//! Growing the bitset to cover `p` fills each new word with zeros, from level
//! 0 upward, builds any new top level from the one below it, and stops at the
//! first level already long enough. The bitset module states the cost: O(1)
//! plus amortized O(1) per word added. A growing call adds at least one word
//! at level 0, covering 64 new positions, so the uncovered-height term pays
//! for the whole call at a constant weight. It can, because the bitset
//! receives only positions at or below the top, which lie at or below `T`.
//! The written set is created only where a window starts, so `T` already
//! reaches the window's greatest top, and an update raises `T` by at most its
//! `G` plus O(`1 + log_B n`), a normalization by at most one position, and a
//! comparison not at all. A shift or a clone builds a new written set whose
//! `T` is at most its own width, which it pays for.
//!
//! The digit buffer and each bitset level grow geometrically, as `Vec` does,
//! so the copies their reallocations make total less than twice their final
//! lengths, which the same terms pay for. That growth leaves spare capacity:
//! an operation that extends a full allocation by one position can retain as
//! much new capacity as the allocation it replaced. Growth can also fall on
//! an operation that does not raise the working width. For example:
//! - in the bitset form, a one-limb update two positions above the top moves
//!   the run, and the top with it, into the bitset, which grows to cover it;
//! - a one-limb update that lands in the gap below the run, apart from the
//!   positions on either side, converts a prefix into a bitset, or grows an
//!   existing bitset, to cover every position up to its own. Landing just
//!   below a far run, it allocates one bit for each position the digit
//!   buffer holds below that run, about a sixty-fourth of the buffer's
//!   space, in one call whose `L` is 1 and whose `G` is zero.
//!
//! The uncovered-height term, which the updates that first wrote that high
//! raised, pays for such growth, so the bound holds over the sequence and not
//! for each call. This is why the crate documents retained space as amortized
//! for every operation that can grow storage. Comparisons never grow storage:
//! their write-back lands at or below the old top and adds no position to the
//! bitset.
//!
//! # Updates
//!
//! Beyond what the potential pays for, an update pays its own additions to
//! the potential: O(1) carry potential per deposit, one held position for each
//! position it newly inserts into the written set, the nonempty words its
//! insertions create, and the rise of `T`. It also pays its *far costs*: the
//! insertion climbs of the second fact and the run crossings of the third.
//! Each starts at a position `q` and stops no higher than the word holding
//! some held position `y < q`, at a cost of O(`1 + β(y, q)`). An accumulator
//! operand also costs its own comparison, amortized O(1) against the
//! operand's potential, and one read per stored digit.
//!
//! **Lemma: few writes per position.** Suppose every contribution of an
//! update has one sign, as for a primitive or a limb stream, or that every
//! contribution lands at bit offset zero, as for an accumulator operand at a
//! shift that is a multiple of 32. Then the update newly inserts each
//! position O(1) times, and no truncation follows a deposit that lands above
//! the top.
//!
//! - With one sign, every carry has that sign too: a total of a digit of
//!   magnitude below `2B` and an addend of one sign reaches `2B` only in that
//!   sign's direction. The top therefore falls only when a write cancels
//!   exactly a top digit of the other sign. Once any write lands above the
//!   top, the highest position written holds a nonzero digit of the update's
//!   sign: every later write to it adds that sign, and a carry out of it
//!   lands higher with that sign. So the top never falls again. A truncation
//!   removes only positions above its new top, and a removed position returns
//!   to the set only through a write at it, which lands above the top, after
//!   which nothing is removed. So the update removes each position at most
//!   once and newly inserts it at most twice.
//! - At bit offset zero, every contribution is one operand digit of
//!   magnitude below `2B`, so every carry has magnitude at most 4. A deposit
//!   above the top lands on a zero digit, leaves a nonzero digit, and carries
//!   nothing, and each later deposit does the same higher up, so the top never
//!   falls again. Before that, the top can fall more than once, and a carry
//!   chain out of a truncation's new top `t` can write `t + 1` again, so a
//!   later descent can cross a run the update created. Writes still stay few:
//!   by induction upward from the first deposit position, each position
//!   receives at most two carries and carries out at most twice. Every carry
//!   it receives comes from a chain of a lower deposit, so it arrives before
//!   the position's own deposit. After carrying out once, the digit lies
//!   within `B/2` of zero, and one more carry of magnitude at most 4 cannot
//!   bring it to `2B`. Its own deposit can make it carry out once more, and
//!   nothing writes it afterwards, because later deposits lie higher and
//!   chains climb. So the update writes each position at most three times.
//!
//! **Far costs under that lemma.** Let `x_1 < ... < x_n` be the deposit
//! positions and `p` the highest position written. A span joins the bitset
//! only when it lies below the run, and a run joins the bitset only when a
//! jump moves it. The update's climbs start at these positions `q`, each with
//! a held `y` at or below the word where it stops:
//!
//! - a position after the first of a span or moved run that the update
//!   inserts into the bitset, with `y = q - 1`, inserted just before it;
//! - the first position of such a span or run: a deposit position `x_i`, with
//!   `y >= x_(i-1)` while `x_(i-1)` is held; the start `t + 1` of a run that
//!   a truncation to `t` left empty, with `y = t`; or the receiver's run
//!   start;
//! - a run crossing from such a `t + 1`, with `y = t`, because a truncation
//!   that removed `t` would remove that run too; or from the receiver's run
//!   start.
//!
//! Every run the update creates starts at such a `t + 1` or at a jump's
//! destination, and no crossing reaches the latter: a jump lands above the
//! top, which no truncation follows. Three kinds of climb are exceptions,
//! each O(`1 + log_64 (p + 1)`) by the first fact: those from `x_1`; those
//! from the receiver's run start; and those from the one deposit position
//! that follows a truncation leaving the top below it. There is at most one
//! such deposit, because it lands above the top. The receiver's run also
//! moves its other positions once, which the held-positions term prepaid,
//! and their climbs telescope within that run.
//!
//! Every other interval `(y, q]` lies within `[x_1, p]`, and together they
//! cover each position O(1) times:
//! - `(q - 1, q]` serves once each time the update newly inserts `q` or moves
//!   it into the bitset, and a position enters a run only when newly
//!   inserted;
//! - `(t, t + 1]` serves at most twice per truncation to `t`. Between two
//!   truncations to `t`, the update writes `t + 1`, because the top can
//!   otherwise rise above `t` only through a deposit above the top, which no
//!   truncation follows. Under the lemma that happens O(1) times: with one
//!   sign, that write lands above the top, and at bit offset zero, the update
//!   writes each position at most three times;
//! - the intervals `(x_(i-1), x_i]` are disjoint, and each serves O(1)
//!   climbs.
//!
//! The climbs number O(1) per newly inserted position and per truncation, at
//! most one of which follows each deposit. So the far costs total
//! O(`n + (p - x_1) + 1 + log_64 (p + 1)`), by the telescoping fact.
//!
//! The positions the update writes lie in its *deposit span*, from its first
//! deposit position to three past its last, or along a carry chain beyond
//! that span. Counting the positions of zero pieces it skips, a limb
//! stream's deposit span covers at most `2L + 3` positions and an
//! accumulator operand's at most `A / 32 + 3`. At most one carry chain
//! reaches more than a constant past the deposit span, because a chain
//! leaves the digits it passes within `B/2` of zero and no later deposit of
//! the update recharges digits above the span. So `p - x_1` exceeds the
//! span's length `m` only by that chain, which the carry potential pays for,
//! and the far costs total O(`m + 1 + log_64 (p + 1)`) beyond it. At shift
//! 0, `p` itself exceeds the deposit span only by that chain, so the
//! logarithm is O(`m`) as well.
//!
//! **Rows.** Combining the above with the per-deposit carry bound:
//!
//! - A primitive makes a constant number of deposits, at positions up to 3,
//!   so it costs amortized O(1).
//! - A limb stream makes up to `2L` deposits. At shift 0 it costs amortized
//!   O(`L`). Its `G` is then O(`L`) as well, which is why `G` does not
//!   appear. At a nonzero shift it costs amortized
//!   O(`L` + `G` + log(`W` + 1)).
//! - An accumulator operand makes up to `A / 32` deposits, so the same holds
//!   with `A` in place of `L` at shift 0. At a nonzero shift that is a
//!   multiple of 32 bits, it costs amortized O(`A` + `G` + log(`W` + 1)). The
//!   crate promises only the weaker bound below for every nonzero shift.
//!
//! **Mixed signs at a nonzero bit offset.** An accumulator operand's digits
//! can have mixed signs, and shifting by a bit offset `b > 0` lets one digit
//! carry by itself. Take operand digits `[B, -1]` repeated, then a top digit
//! of 3, which keeps the operand nonzero and leaves its comparison nothing to
//! compact, at shift `32s + 1`. Let the receiver's top lie below `s`, and let
//! its bitset cover positions past the operand's span but hold nothing near
//! `s`, as a write far above it and the write's cancellation leave. Each pair
//! of digits makes two deposits:
//! - the first writes `0` and `2` above the top, a jump that starts a run;
//! - the second writes `-2` onto the `2`, then descends through the new run
//!   and crosses into the bitset, climbing about `log_64 s` levels.
//!
//! With `A` proportional to `s`, the update costs Θ(`A` log(`W` + 1)) bitset
//! work, while `A + G` is of order `s`. No bound of the form
//! O(`A` + `G` + log(`W` + 1)) holds. The coverage matters: a query from
//! beyond the bitset's storage reads its last stored word whole, so without
//! it each crossing costs O(1). The crate states amortized
//! O(`A` log(`W` + 1) + `G`) for every nonzero shift, which does not need
//! the lemma. Each deposit causes at most one descent, with at most two run
//! crossings. Beyond its first three positions, its carry chain passes only
//! digits of magnitude at least `2B - 3`, which are nonzero and so already
//! held, so the deposit newly inserts O(1) positions. It moves at most one
//! run, whose positions the held-positions term prepaid and whose climbs
//! after the first telescope within the run. By the first fact, each
//! crossing, each newly inserted position, and each moved run's first
//! position climbs O(`1 + log_64 (p + 1)`) levels.
//!
//! # Comparisons
//!
//! Both comparisons run the same scan. Its visits are paid by held positions,
//! its queries from held positions by nonempty words, and its write-back is
//! one deposit. The scan's truncation leaves the run ending at the stopping
//! position, or parks an emptied run just above the highest position left,
//! so the write-back extends a run, starts one, or rewrites a position the
//! bitset already holds: it adds no position to the bitset and climbs no
//! level. What remains are at most two run crossings, each
//! O(`1 + log_64 (p + 1)`) for some `p` at most the top.
//!
//! These far costs are charged to an earlier operation. A scan leaves the
//! digits at and above its stopping position in a form that the next scan
//! decides at once. That next scan reads at most two digits, rewrites at most
//! those two, and stops within one position of the same place. Its
//! truncation, if any, crosses from the place's successor to the place
//! itself, a skip of one position, which costs O(1) by the bitset's distance
//! bound. Negation changes no magnitude and preserves this.
//!
//! A normalization leaves such a form too. Write `D` for
//! [`COMPARISON_DECIDED`](super::sign::COMPARISON_DECIDED), the partial at
//! which a scan stops. A normalization's own comparison runs first, and
//! leaves the lowest written position below the top whenever the normalized
//! top digit, at a position `t > 0`, is below `D` in magnitude. If some
//! normalized digit below `t` is nonzero, the pass wrote it, at or above the
//! lowest written position. Otherwise the value is that small top digit
//! times `B^t`, and the comparison cannot have stopped above `t`, where no
//! partial reaches `D`. It stopped at `t` only with a partial of at least
//! `D`, which needs a written nonzero digit below `t` to bring the value back
//! down; and it stopped below `t` only by writing back there, or at the old
//! top, a written position. After the pass every digit has the value's sign
//! and magnitude below `B`, so the next scan stops at the top or, when the
//! top digit is below `D`, one position lower, where the partial reaches `B`.
//! Both positions lie in the run from the lowest written position to the
//! top, so that scan's truncation stays in the run and costs O(1). At
//! `t = 0` the next scan stops at position zero, with nothing below it to
//! cross.
//!
//! So a scan with far costs needs one of these since the previous comparison
//! or normalization, or since construction or clear if there was none:
//! - an update that wrote at or above the position below the top that the
//!   previous comparison or normalization left;
//! - a shift.
//!
//! Charge the far costs to whichever of those operations wrote the highest
//! position. The top is at most a constant above that position. Each
//! operation is charged at most once, by the next comparison, and each can
//! afford its charge:
//! - at shift 0, an update's positions are bounded by its deposit span and
//!   carry chain;
//! - at a nonzero shift, an update's bound already includes log(`W` + 1);
//! - a shift's own bound covers its charge, a crossing below its result's
//!   top.
//!
//! This charge is why the nonzero-shift rows keep log(`W` + 1) even for an
//! update of one sign that cancels no digit. Such an update can leave the
//! top digit small enough that the next scan steps into a gap below it and
//! stops there. That scan's truncation then crosses from the run's start to
//! the bitset's highest position, through covered words that never held
//! anything, a climb of up to `log_64` of the top.
//!
//! Comparisons therefore cost amortized O(1), and they allocate nothing.
//!
//! # Raising the top across gaps
//!
//! The crate states this case on its own, because a caller's cost argument
//! can rest on it. A *gap* is a range of positions below the top that the
//! written set does not hold: positions a jump skipped, or positions that a
//! truncation removed after cancellation emptied them.
//!
//! Raising the top across a gap writes nothing in it. A deposit above the top
//! starts a run where it lands, and the jump moves only the old run, whose
//! positions the held-positions term prepaid. Beyond its deposits, the update
//! pays logarithmic climbs and the growth of the digit buffer and of `T`, and
//! by "Storage growth" those grow only past the window's greatest top. So
//! an update that raises the top again, to a width reached earlier in the
//! window, has a `G` of zero, however far its top moves.
//!
//! Descending through a gap is one run crossing, from the start of the run
//! above the gap to the highest position held below it. By the bitset's
//! distance bound it costs O(`1 + log_64 d`) for a gap of `d` positions, on
//! every crossing, and O(1) in the prefix form. The positions of the run
//! itself, which the descent visits, are paid for by the held-positions term.
//! Crossings happen in three places, and each is charged once:
//! - in an update's descent after it cancels the top, as one of the update's
//!   far costs, which "Updates" bounds by O(`1 + log_64 (p + 1)`) at a
//!   nonzero shift and by the deposit span's length at shift 0;
//! - in a comparison's scan or truncation, which "Comparisons" charges to the
//!   operation that wrote the highest position since the previous comparison,
//!   whether or not a digit cancelled;
//! - in the comparison a read begins with, charged the same way.
//!
//! A cycle that raises the top across a gap and cancels it again therefore
//! costs, beyond its updates' deposits, a constant number of crossings of
//! O(`1 + log_64 d`) each, which the nonzero-shift rows' log(`W` + 1) terms
//! contain, and never the gap's width.
//!
//! # Updates of one sign
//!
//! The crate states this case on its own, because `before`'s `Sum` rests on
//! it. A *stretch* begins where `G`'s window begins: at creation, including
//! the creation of a borrowed `<<`'s result, at `clone_from`, or at a nonzero
//! shift by `<<=` or an owned `<<`. Every update
//! in it is a primitive or a limb stream whose contributions all have one
//! sign `s`, and the value at its start is zero or has sign `s`. Nothing else
//! happens in it but working-width and known-zero queries, which change
//! nothing.
//!
//! Call the stored form *sign-led* when the value is zero, or its top digit
//! has sign `s` and no digit `d` has `s · d < -B / 2`. Every stretch begins
//! sign-led. A new accumulator and a shift of zero leave every digit zero,
//! and a primitive's construction deposits that primitive, of sign `s`, into
//! zeroed digits. `clone_from` copies a normalized value, whose digits all
//! have its sign. A nonzero shift in any spelling, a borrowed `<<`'s result
//! included, deposits the value's normalized digits, all of sign `s`, into
//! zeroed storage, so recentering is the only source of a digit of the other
//! sign, and it leaves that digit within `B / 2` of zero. A small value's
//! shift deposits the value itself, of sign `s`, into zeroed digits.
//!
//! A one-sign update keeps the form sign-led. Its contributions and carries
//! all have sign `s`, so each digit moves in direction `s` until it
//! recenters, which leaves it in `[-B / 2, B / 2)` and carries in sign `s`
//! to the position above. The top digit therefore keeps sign `s`: a carry
//! out of it lands higher and starts a new top of sign `s`. So the top never
//! falls: no update in the stretch descends, and nothing truncates the
//! written set. With no truncation and no clear, no bitset word empties
//! during the stretch, so each word turns nonempty at most once, and the
//! predecessor queries and run crossings of "Bitset climbs" never occur.
//!
//! The updates' work is then the work of "Updates" without its far costs:
//! deposits, paid by the carry potential; positions newly inserted or moved,
//! paid by the held-positions term or prepaid by it; buffer and bitset
//! growth, paid by `G`, the deposits, and the uncovered-height term; and
//! insertion climbs. By the second fact of "Bitset climbs", an insertion
//! climbs at most one level more than the number of words it fills. A word
//! other than word zero at level `k` covers no position below
//! `64^(k + 1)`, and word zero is nonempty at every level by the first fact,
//! so every word the stretch fills covers a held position at or below the
//! stretch's highest top `P`, and there are at most `P / 63` such words. `P`
//! is at most the window's greatest width, which is the width the stretch
//! began with plus the updates' `G`. The operation that began the stretch is
//! allowed that starting width: creation starts at one position or a
//! primitive's width, `clone_from`'s row includes O(`Q`) for the copy, and a
//! shift's row is amortized O(`S`) whatever its actual work. A small value's
//! `<<=` into storage a clear kept writes only a few digits, yet its row
//! still allows O(`S`), and the fills are charged against that allowance.
//!
//! So each update costs O(1) beyond its deposits, its `L`, and its `G`, and
//! the fills total at most what the updates' `G` and the starting width pay
//! for. Each primitive update costs amortized O(1), and each limb stream
//! amortized O(`L` + `G`), at any shift. The potential the stretch leaves is
//! within the same sum. The test
//! `a_one_sign_stretch_after_a_shift_climbs_once_per_insertion_and_fill`
//! measures the fill bound with the bitset's level counters.
//!
//! The first comparison after the stretch can still have far costs: at most
//! two run crossings below its top, each O(`1 + log_64 (V + 1)`).
//! "Comparisons" would charge them to the update that wrote the highest
//! position, whose bound in the stretch no longer covers them, so that
//! comparison pays them itself, once. A top digit of 1 whose position below
//! was never written, left by a limb whose low half was zero, is such a
//! case: the scan steps below the top, stops there, and its truncation
//! crosses to the highest position held further down. Later comparisons
//! charge operations after the stretch.
//!
//! A stretch cannot begin at a clear or a read, although each leaves the form
//! sign-led. Both clear the written set, which empties the bitset's words but
//! keeps its storage, and both leave `G`'s window running, so the words a
//! later update refills are paid for by nothing it is allowed. Repeat this
//! round, with `q = 64^m` below a width the window reached earlier:
//!
//! 1. clear;
//! 2. add one limb at position `q + 2`, which re-raises the top with a `G` of
//!    zero and a jump that finds position zero held, one level;
//! 3. add one limb at position `q`, in the gap below that run and apart from
//!    position zero, which fills one word at each level from 0 to `m - 1`
//!    and stops at level `m`, at word zero: `m + 1` levels.
//!
//! The next round's clear zeroes the held positions `q` and `q + 2` and
//! examines `m + 1` levels, emptying those words again, which the limb that
//! filled them paid for. Each round costs Θ(`m`), a base-64 logarithm of
//! the width, while each update's `L` + `G` stays constant. So those updates need the limb row's
//! log(`W` + 1), and a stretch that began at a clear would promise too
//! little. `clear_cycle_climbs_a_level_per_base_64_digit_each_round` in the
//! tests measures `m + 1` levels per round at each `m` it draws.
//!
//! # Clears
//!
//! A read and a clear each *clear* the written set: they reduce it to
//! position zero, held by the run `0..1` and, in the bitset form, by the
//! bitset too, and the set passes every other position it held to its
//! caller. The run and the prefix are ranges, so the set passes their
//! positions one step each. The bitset passes its positions by a walk upward
//! from level 0 that empties each nonempty word below the level it reaches,
//! enters no empty word, and climbs past a level only while a position
//! remains above that level's word zero, a position whose word at that level
//! the walk later empties. Its module states the cost: O(1), plus one word
//! operation per word it empties and per position it passes. Each passed
//! position spends its held-positions unit, and each emptied word its
//! nonempty-words unit, so a clear costs amortized O(1). It raises no term:
//! word zero was already nonempty at every level, and the set only shrinks.
//!
//! The words a clear empties were paid for when they turned nonempty. An
//! insertion that fills a word pays the nonempty-words term for it. Growth
//! never fills a word other than word zero: it appends zero words, and a new
//! top level summarizes a level whose only nonempty word is word zero, the
//! one-word top before the growth began or a level the growth created, so
//! only the new level's word zero gains a bit, which the growing call pays
//! for. A copy of the stored form, which the accumulator's `Debug` makes,
//! copies filled words one word operation each, which pays for them.
//!
//! After a clear, the first jump above the run moves the run `0..1` below
//! it, into the prefix or into a bitset that already holds zero, at a cost of
//! O(1): its insertion stops at once, at word zero of level 0.
//!
//! # Reads
//!
//! A magnitude read, and a clone's normalization of its source, run one
//! comparison and then one pass from the read's starting position through the
//! top. The comparison leaves a magnitude of at least `B^t / 3`, with `t` the
//! top, so the pass covers at most a constant more positions than the result
//! has, which is O(`Q`). The pass leaves every digit below `B`, raising the
//! carry potential by at most 1/2 per position. It then clears the written
//! set, whose positions other than zero all lie in the pass's span, so that
//! clear passes O(`Q`) positions, and inserts one run from the lowest
//! written position to the new top, which costs O(1) by the same section. It leaves O(`Q`) held positions, raises
//! the uncovered height by at most one position, and raises no other term.
//! "Comparisons" never charges it. A read therefore costs amortized
//! O(1 + `Q`), including its comparison's amortized O(1).
//!
//! # Clear
//!
//! An accumulator's clear zeroes digit zero and each position the written
//! set passes as it clears. That zeroes every digit: the set holds every
//! nonzero digit at or below the top, and every digit above the top is zero.
//! Each zeroing is paid by the position's held-positions unit, which the
//! set's clear spends, so the accumulator's clear costs amortized O(1). One
//! call can take time proportional to the positions and nonempty words it
//! finds, at most O(1 + `W`), work the writes that made them paid for.
//!
//! # Other operations
//!
//! - Negation touches only the digits through the top, O(`V`).
//! - A comparison with a primitive or a primitive conversion asks the
//!   stability query, one scan, amortized O(1); its far costs are charged,
//!   as for any comparison, to the operation that wrote the highest position
//!   before it. When the query does not decide, its scan stopped at or below
//!   position 5, leaving a value of at most six positions, and the read that
//!   follows costs O(1) by "Reads". The comparison it begins with decides at
//!   once on the form the query left.
//! - A nonzero shift of a value stored in digits, in any spelling of `<<`,
//!   normalizes the value in place, then moves it into new digits with one
//!   shifted accumulator update. `<<=` and an owned `<<` then release the old
//!   storage; a borrowed `<<` leaves it, normalized, with its owner. The
//!   normalization is a read of the whole value, amortized O(1 + `S`) by
//!   "Reads", since the value's width is at most `S`. Every normalized digit
//!   has the value's sign, so the update is of one sign, and it starts from
//!   zero in new digits with a new written set. As in "Updates of one sign",
//!   its top never falls, so it descends nowhere and truncates nothing, and
//!   each bitset word it fills it fills once; those words number at most one
//!   per 63 positions of the result. Its deposits, its buffer and bitset
//!   growth, and the potential it leaves are each O(`S`) as well, so the shift
//!   costs amortized O(`S`). Normalizing can fill a run of zero digits with
//!   digits of magnitude `B - 1`, where a borrow crosses it, so the update
//!   deposits at positions that the stored digits left at zero; it still
//!   deposits at most once per position of the value's width. The normalization
//!   can also add one digit to the old buffer, which reallocates a full buffer;
//!   that buffer then ends at the old top, so the copy and the two buffers held
//!   during it are the value's width, within O(`S`). A small value instead
//!   makes at most four deposits of one sign into the digits it activates, the
//!   receiver's retained ones for `<<=` and new ones for a borrowed `<<`, all
//!   within the shifted value's span, so the argument under "Updates" bounds
//!   their far costs by that span. The first of them jumps over the run `0..1`
//!   that activation or the last clear left, which costs O(1) by "Clears".
//! - A clone normalizes its source, a read, then copies the `Q` positions of
//!   the value into a written set of position zero and one run, which needs
//!   no bitset. The copy's potential, at most 1/2 per digit and one per held
//!   position, is O(`Q`), which the clone pays. `clone_from` releases the
//!   target's old written set, which its allocations paid for.
//! - Owned addition reads the operand with the smaller working width into the
//!   other at shift 0 and drops it; dropping state only lowers the potential.
//!   `abs_diff` does the same, asks for the sign, and negates a negative
//!   result. The sign read leaves the result's top within a constant of its
//!   own width, so the negation costs the result's width: O(`A`) when both
//!   operands are nonnegative, and at most O(`V`) of the wider operand.
//! - Comparing two accumulators asks the wider side's stability query while
//!   one leads by more than
//!   [`CLOSE_LEAD`](crate::accumulator::compare::CLOSE_LEAD) positions. A
//!   refusal leaves the asked side at most `CLOSE_LEAD` positions wider than
//!   the other, so at most two queries refuse before the loop ends, and one
//!   more may be asked at a lead of exactly `CLOSE_LEAD`.
//!   Otherwise the comparison subtracts both, at shift 0, into a new
//!   temporary of O(`A`) positions, compares it with zero, and drops it. The
//!   temporary's own history is those two updates, so its whole cost, scan
//!   included, is O(`A`).
//! - A sum of `K` primitives makes `K` primitive updates into a new
//!   accumulator, O(`K`). A sum of accumulators adds each input at shift 0
//!   into a new accumulator, or, for owned inputs, the narrower of the running
//!   total and the next input into the wider. Each step reads at most the
//!   next input's width, so the sum costs O(`T`), with `T` the total of the
//!   inputs' working widths, each at least one position.
