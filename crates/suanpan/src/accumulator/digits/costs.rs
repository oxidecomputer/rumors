//! Derives the crate's four cost rules, and its rule for ownership, from the
//! digit and written-set implementation.
//!
//! The crate documentation states four rules: an update costs what it adds;
//! a read costs what it returns; growing storage costs once; and a shifted
//! update adds a logarithm. Every method's cost follows from them, from the
//! rule for ownership, and from the costs the crate states beside them for
//! negation, shifts, formatting, and comparing two accumulators. This module
//! holds no code: it records the argument behind each rule, one section per
//! rule, so that a change to
//! [`Digits`](super::Digits) or to
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
//! Positions are digit positions of 32 bits. The crate counts bits, or 64-bit
//! limbs (`L`), which differ from position counts by constant factors. The
//! *top* is `highest_nonzero`. A *deposit* is one call of
//! [`add_at`](super::Digits::add_at); every update makes its deposits at
//! ascending positions. The *run*, the *prefix*, and the *bitset* are the
//! parts of a written set that the [`written`](super::written) module
//! describes: the run holds the highest consecutive positions, and the prefix
//! or the bitset holds the positions below it.
//!
//! A *storage* begins when an accumulator is created, receives
//! `clone_from`, or is shifted by a nonzero count, and lasts until the next
//! of these; every other operation, `clear` included, keeps it, and an owned
//! operator's result keeps the storage of the operand it reuses. Its *width*
//! `h` is the highest position written since it began. The digit buffer
//! never shrinks within a storage: `clone_from` shortens it to the copy, and
//! a shift of a value held in digits replaces it. A shift of a small value,
//! or of a zero, keeps its buffer, so counting it as a new storage, as the
//! crate does to keep the definition in terms a caller can see, only
//! shortens a storage's life; that can only make the rules charge more, not
//! less, as "Rule 3" shows.
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
//!    insertion: a position below the run never rejoins a run, a set converts
//!    its prefix once, and a removed position returns only by being written
//!    again.
//! 3. *Nonempty words* is the number of nonzero words, at every level, of the
//!    bitset. It pays for a removal's climb, for the climbs of predecessor
//!    queries from positions that the scan then removes (see "Bitset climbs"
//!    below), and for the words a clear empties (see "Clears").
//! 4. *Uncovered height* is `max(0, T - C) / 64`, where `T` is the highest
//!    position written since the written set was created, and `C` is the
//!    number of positions the bitset's storage covers, zero without a bitset.
//!    It pays for growing the bitset's storage.
//!
//! # Facts the rules share
//!
//! ## Carries
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
//! ## Where updates write
//!
//! An update of `n` deposits writes only positions within O(`1 + log_B n`) of
//! the higher of the receiver's top when it starts and its top when it ends.
//! Together with "Rule 3", this is why the storage an update adds is paid for
//! by the distance it raises the storage width, apart from a term its own
//! deposits pay for.
//!
//! Let `H` be the higher of the receiver's starting top `t` and the update's
//! highest deposit position `x`. Every digit above `H` starts at zero and
//! changes only by carries. A carry into a position three or more past its
//! chain's deposit has magnitude at most 3, so a position three or more past
//! `H` carries out only after absorbing about `B / 2` carries since it last
//! carried, at most one from each chain. The writes past `H` therefore reach
//! O(`1 + log_B n`) positions. It remains to show that a deposit far above
//! `t` leaves the result's top near it, so that `H` lies within a constant of
//! the higher top.
//!
//! - A primitive deposits at positions up to 3.
//! - A limb stream's contributions all have one sign, and so do those of an
//!   accumulator operand at a nonzero bit offset, which streams the operand's
//!   magnitude digits ("Few writes per position" below). If the highest
//!   nonzero contribution lands at `x >= t + 3`, the result's magnitude is at
//!   least `B^x - 2.01 B^(t + 1) > B^(x - 1)`. A value whose top is `t'` has
//!   magnitude below `2.01 B^(t' + 1)`, so the result's top is at least
//!   `x - 2`.
//! - An accumulator operand is first compared with zero and skipped if zero.
//!   The comparison leaves a top digit, at position `i`, of magnitude at
//!   least [`COMPARISON_DECIDED`](super::sign::COMPARISON_DECIDED), or of
//!   magnitude at least 1 over a digit of magnitude at most `B/2`, or at
//!   position zero. In each case the operand's magnitude is at
//!   least `B^i / 3`, and the same argument places the result's top within a
//!   constant of the operand's shifted top.
//!
//! ## Bitset climbs
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
//! A climb from `p` is therefore O(`1 + log_64 (p + 1)`), and every position
//! an operation climbs from lies at or below the receiver's storage width,
//! give or take a constant: by "Where updates write", a write lands within
//! O(`1 + log_B n`) of a top the storage has reached, and `n` is at most
//! that top plus three, because the deposits lie at distinct positions no
//! higher than two above it. So every climb is O(`1 + log_64 (h + 1)`), and
//! the bitset never needs more than `MAX_LEVELS` levels, the count that
//! covers every `usize` position.
//!
//! # Rule 1: an update costs what it adds
//!
//! Beyond what the potential pays for, an update pays its own additions to
//! the potential: O(1) carry potential per deposit, one held position for each
//! position it newly inserts into the written set, the nonempty words its
//! insertions create, and the rise of `T`. It also pays its *far costs*: the
//! insertion climbs of the second fact and the run crossings of the third.
//! Each starts at a position `q` and stops no higher than the word holding
//! some held position `y < q`, at a cost of O(`1 + β(y, q)`). An accumulator
//! operand also costs its own comparison, amortized O(1) against the
//! operand's potential, and one read per stored digit, which after that
//! comparison is O(1 + the operand's width).
//!
//! **Lemma: few writes per position.** Suppose every contribution of an
//! update has one sign, or that every contribution lands at bit offset zero.
//! Then the update newly inserts each position O(1) times, and no truncation
//! follows a deposit that lands above the top. Every update satisfies one of
//! the two. A primitive and a limb stream deposit one sign. An accumulator
//! operand at a bit offset of zero deposits its stored digits as they are,
//! each at offset zero. At any other bit offset, it streams the operand's
//! magnitude instead: the Euclidean carry of normalization turns the stored
//! digits, from the lowest up, into digits below `B` of the value's sign, and
//! at the top stored position into the whole rest of the magnitude, below
//! `2^34`, which the update deposits without writing them back. A stored digit
//! of the other sign, shifted by that offset, could otherwise carry by
//! itself, and the descent after it could cross a gap once per digit.
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
//! **Far costs.** Let `x_1 < ... < x_n` be the deposit positions and `p` the
//! highest position written. A span joins the bitset only when it lies below
//! the run, and a run joins the bitset only when a jump moves it. The
//! update's climbs start at these positions `q`, each with a held `y` at or
//! below the word where it stops:
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
//! accumulator operand's at most its working width plus 3. At most one carry
//! chain reaches more than a constant past the deposit span, because a chain
//! leaves the digits it passes within `B/2` of zero and no later deposit of
//! the update recharges digits above the span. So `p - x_1` exceeds the
//! span's length `m` only by that chain, which the carry potential pays for,
//! and the far costs total O(`m + 1 + log_64 (p + 1)`) beyond it. At shift
//! 0, `p` itself exceeds the deposit span only by that chain, so the
//! logarithm is O(`m`) as well.
//!
//! So an update at shift 0 costs amortized O(1) for a primitive, which
//! deposits at positions up to 3, O(`L`) for `L` limbs, and O(1 + the
//! operand's width) for an accumulator operand: the deposit span's length,
//! which also bounds the storage it can add. At a nonzero shift, the same holds
//! with the logarithm of "Rule 4" and the growth of "Rule 3" added.
//!
//! # Rule 2: a read costs what it returns
//!
//! ## Comparisons with zero
//!
//! Every read begins with the same scan from the top. Its visits are paid by
//! held positions, its queries from held positions by nonempty words, and
//! its write-back is one deposit. The scan's truncation leaves the run ending
//! at the stopping position, or parks an emptied run just above the highest
//! position left, so the write-back extends a run, starts one, or rewrites a
//! position the bitset already holds: it adds no position to the bitset and
//! climbs no level. What remains are at most two run crossings, each
//! O(`1 + log_64 (p + 1)`) for some `p` at most the top. "Rule 4" charges
//! them to an earlier operation, so a comparison with zero costs amortized
//! O(1), and it allocates nothing.
//!
//! The scan stops at a position `i` only with a partial of magnitude at least
//! 3, which leaves the value's magnitude above `0.99 B^i`, or at position
//! zero. So after any comparison with zero, the top lies within two positions
//! of the value's own width. Every read below, every accumulator operand, and
//! every choice of the narrower operand begins with one, which is why their
//! costs are in the widths of values and never of a cancelled stored form.
//!
//! ## Clears
//!
//! A *clear* of the written set reduces it to position zero and passes every
//! other position it held to its caller. The run and the prefix are ranges,
//! so the set passes their positions one comparison each. The bitset passes
//! its positions by a walk upward from level 0 that empties each nonempty
//! word below the level it reaches, enters no empty word, and climbs past a
//! level only while a position remains above that level's word zero, a
//! position whose word at that level the walk later empties. Its module
//! states the cost: O(1), plus one word operation per word it empties and per
//! position it passes. Each passed position spends its held-positions unit,
//! and each emptied word its nonempty-words unit, so a clear costs amortized
//! O(1). It raises no term: word zero was already nonempty at every level,
//! and the set only shrinks.
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
//! An accumulator's [`clear`](super::Digits::clear) zeroes digit zero and
//! each position the set passes, then leaves the set holding zero. That
//! zeroes every digit: the set holds every nonzero digit at or below the
//! top, and every digit above the top is zero. So `clear` costs amortized
//! O(1), whatever span the positions cover, and one call costs at most
//! O(1 + `h`), since every held position and every nonempty word lies within
//! the storage width.
//!
//! After a clear, the first jump above the run moves the run `0..1` below
//! it, into the prefix or into a bitset that already holds zero, at a cost of
//! O(1): its insertion stops at once, at word zero of level 0.
//!
//! ## Magnitude reads
//!
//! A magnitude read, and a clone's normalization of its source, run one
//! comparison and then one pass from the read's starting position through the
//! top. By the comparison's bound, the pass covers at most a constant more
//! positions than the result has, which is O(`Q`) for a result of `Q`
//! positions. The pass leaves every digit below `B`, raising the carry
//! potential by at most 1/2 per position. It then clears the written set,
//! whose positions other than zero all lie in the pass's span, so that clear
//! passes at most O(`Q`) positions, and inserts one run from the lowest
//! written position to the new top, which costs O(1) by the prefix form's
//! constant cost or the clear's leaving a run at zero. It leaves O(`Q`) held
//! positions, raises the uncovered height by at most one position, and raises
//! no other term. "Rule 4" never charges it. A read therefore costs amortized
//! O(1 + `Q`), including its comparison's amortized O(1).
//!
//! ## Negation, shifts, clones, and comparisons of two accumulators
//!
//! - Negation compares with zero, then negates every digit through the top,
//!   which the comparison left within two positions of the value's width:
//!   amortized O(1 + `Q`).
//! - A comparison with a primitive or a primitive conversion asks the
//!   stability query, one scan, amortized O(1). When the query does not
//!   decide, its scan stopped at or below position 5, leaving a value of at
//!   most six positions, and the read that follows costs O(1) by "Magnitude
//!   reads". The comparison it begins with decides at once on the form the
//!   query left.
//! - A nonzero shift of a value stored in digits, in any spelling of `<<`,
//!   compares the value with zero, then streams its magnitude into new
//!   digits with one shifted update, normalizing as it goes through the
//!   Euclidean carry and leaving the old digits as they are. `<<=` and an
//!   owned `<<` then release the old storage; a borrowed `<<` leaves it with
//!   its owner, changed only by the comparison. The comparison costs
//!   amortized O(1) and leaves the top within two positions of the value's
//!   width, so the stream reads O(`S`) positions for a result of `S`
//!   positions. Every piece it deposits has the value's sign, so the update
//!   is of one sign, and it starts from zero in new digits with a new
//!   written set. As in "Updates of one sign", its top never falls, so it
//!   descends nowhere and truncates nothing, and each bitset word it fills it
//!   fills once; those words number at most one per 63 positions of the
//!   result. Its deposits, its buffer and bitset growth, and the potential it
//!   leaves are each O(`S`) as well, so the shift costs amortized O(`S`).
//!   Normalizing can fill a run of zero digits with digits of magnitude
//!   `B - 1`, where a borrow crosses it, so the update deposits at positions
//!   that the stored digits left at zero; it still deposits at most once per
//!   position of the value's width. A small value instead makes at most four
//!   deposits of one sign into the digits it activates, the receiver's
//!   retained ones for `<<=` and new ones for a borrowed `<<`, all within the
//!   shifted value's span, so the argument under "Rule 1" bounds their far
//!   costs by that span. The first of them jumps over the run `0..1` that
//!   activation or the last clear left, which costs O(1) by "Clears".
//! - A clone normalizes its source, a read, then copies the `Q` positions of
//!   the value into a written set of position zero and one run, which needs
//!   no bitset. The copy's potential, at most 1/2 per digit and one per held
//!   position, is O(`Q`), which the clone pays. `clone_from` releases the
//!   target's old written set, which its allocations paid for.
//! - Comparing two accumulators asks the wider side's stability query while
//!   one leads by more than
//!   [`CLOSE_LEAD`](crate::accumulator::compare::CLOSE_LEAD) positions. A
//!   refusal leaves the asked side at most `CLOSE_LEAD` positions wider than
//!   the other, so at most two queries refuse before the loop ends, and one
//!   more may be asked at a lead of exactly `CLOSE_LEAD`. If none decides,
//!   the comparison compares both with zero, which leaves each top within
//!   two positions of its value's width and decides at once when the signs
//!   differ, and asks the queries again if a width changed. Otherwise it
//!   subtracts both, at shift 0, into a new temporary of O(1 + the narrower
//!   width) positions, compares it with zero, and drops it. The temporary's
//!   storage is those two updates, so its whole cost, scan included, is
//!   O(1 + the narrower width).
//!
//! # Rule 3: growing storage costs once
//!
//! Growing the digit buffer to position `p` fills the new positions with
//! zeros. The buffer never shrinks within a storage, so it already reaches
//! the storage width `h`, and an update's writes reach past the higher of
//! `h` and its own ending top by O(`1 + log_B n`). The update's rise in `h`
//! and its `n` deposits therefore pay for the growth, and over a storage's
//! life the growth totals its final width. Counting a small value's shift as
//! a new storage, which keeps its buffer, resets `h` and so only charges
//! growth that costs nothing.
//!
//! Growing the bitset to cover `p` fills each new word with zeros, from level
//! 0 upward, builds any new top level from the one below it, and stops at the
//! first level already long enough. The bitset module states the cost: O(1)
//! plus amortized O(1) per word added. A growing call adds at least one word
//! at level 0, covering 64 new positions, so the uncovered-height term pays
//! for the whole call at a constant weight. It can, because the bitset
//! receives only positions at or below the top, which lie at or below `T`.
//! The written set is created only where a storage begins, so `T` already
//! reaches the storage width, and an update raises `T` by at most its rise in
//! `h` plus O(`1 + log_B n`), a normalization by at most one position, and a
//! comparison not at all. A shift or a clone builds a new written set whose
//! `T` is at most its own width, which it pays for.
//!
//! The digit buffer and each bitset level grow geometrically, as `Vec` does,
//! so the copies their reallocations make total less than twice their final
//! lengths, which the same terms pay for. That growth leaves spare capacity:
//! an operation that extends a full allocation by one position can retain as
//! much new capacity as the allocation it replaced. Growth can also fall on
//! an operation that does not raise the storage width. For example:
//! - in the bitset form, a one-limb update two positions above the top moves
//!   the run, and the top with it, into the bitset, which grows to cover it;
//! - a one-limb update that lands in the gap below the run, apart from the
//!   positions on either side, converts a prefix into a bitset, or grows an
//!   existing bitset, to cover every position up to its own. Landing just
//!   below a far run, it allocates one bit for each position the digit
//!   buffer holds below that run, about a sixty-fourth of the buffer's
//!   space, in one call whose `L` is 1 and that raises no storage width.
//!
//! The uncovered-height term, which the updates that first wrote that high
//! raised, pays for such growth, so the bound holds over the sequence and not
//! for each call. Comparisons never grow storage: their write-back lands at or
//! below the old top and adds no position to the bitset.
//!
//! ## Raising the top across gaps
//!
//! The crate's examples show this case, because a caller's cost argument
//! can rest on it. A *gap* is a range of positions below the top that the
//! written set does not hold: positions a jump skipped, or positions that a
//! truncation removed after cancellation emptied them.
//!
//! Raising the top across a gap writes nothing in it. A deposit above the top
//! starts a run where it lands, and the jump moves only the old run, whose
//! positions the held-positions term prepaid. Beyond its deposits, the update
//! pays logarithmic climbs and the growth of the digit buffer and of `T`, and
//! by the argument above those grow only past the storage width. So an
//! update that raises the top again, to a position the storage has reached,
//! adds no storage, however far its top moves.
//!
//! Descending through a gap is one run crossing, from the start of the run
//! above the gap to the highest position held below it. By the bitset's
//! distance bound it costs O(`1 + log_64 d`) for a gap of `d` positions, on
//! every crossing, and O(1) in the prefix form. The positions of the run
//! itself, which the descent visits, are paid for by the held-positions term.
//! Crossings happen in three places, and each is charged once:
//! - in an update's descent after it cancels the top, as one of the update's
//!   far costs, which "Rule 1" bounds by O(`1 + log_64 (p + 1)`) at a nonzero
//!   shift and by the deposit span's length at shift 0;
//! - in a comparison's scan or truncation, which "Rule 4" charges to the
//!   operation that wrote the highest position since the previous
//!   comparison, whether or not a digit cancelled;
//! - in the comparison a read begins with, charged the same way.
//!
//! A cycle that raises the top across a gap and cancels it again therefore
//! costs, beyond its updates' deposits, a constant number of crossings of
//! O(`1 + log_64 d`) each, which the logarithms of "Rule 4" contain, and
//! never the gap's width.
//!
//! # Rule 4: a shifted update adds a logarithm
//!
//! The logarithm pays for two kinds of work, both crossings of positions no
//! update has written: the far costs "Rule 1" leaves at a nonzero shift, and
//! the far costs of the next comparison with zero. Each is
//! O(`1 + log_64 (h + 1)`) by "Bitset climbs".
//!
//! A comparison's far costs are charged to an earlier operation. A scan
//! leaves the digits at and above its stopping position in a form that the
//! next scan decides at once. That next scan reads at most two digits,
//! rewrites at most those two, and stops within one position of the same
//! place. Its truncation, if any, crosses from the place's successor to the
//! place itself, a skip of one position, which costs O(1) by the bitset's
//! distance bound. Negation changes no magnitude and preserves this.
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
//! or normalization, or since its storage began or was last cleared if there
//! was none:
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
//! - at a nonzero shift, an update's cost includes the logarithm;
//! - a shift's own bound covers its charge, a crossing below its result's
//!   top.
//!
//! This charge is why a shifted update pays the logarithm even when it is of
//! one sign and cancels no digit. Such an update can leave the top digit
//! small enough that the next scan steps into a gap below it and stops
//! there. That scan's truncation then crosses from the run's start to the
//! bitset's highest position, through covered words that never held
//! anything, a climb of up to `log_64` of the top.
//!
//! # Ownership
//!
//! - Owned addition returns the other operand when one is known to be zero.
//!   Otherwise it compares the operand whose top is higher with zero, which
//!   only lowers that top, until the operand whose top is higher is one it
//!   has compared: at most twice, and never for a small value, whose width
//!   is exact. It reads the other into that one at shift 0, then drops it;
//!   dropping state only lowers the potential. The operand read has a value
//!   no wider than its top, which is no higher than the compared operand's
//!   top, which lies within two positions of that operand's value width, so
//!   the operand read is the narrower value, give or take two positions. A
//!   sum of owned accumulators makes one owned addition per input after the
//!   first; a sum of borrowed ones adds each input at shift 0 into a new
//!   accumulator.
//! - `a - b` is `a -= &b`: it reads `b` in full into `a`'s storage, at
//!   O(1 + `b`'s width), and reuses no storage of `b`'s, because reusing it
//!   would mean negating it. `&a - b` subtracts `a` from `b` and negates the
//!   result, and a primitive minus an accumulator negates the accumulator and
//!   adds the primitive; each part costs what its own rule says.
//! - `abs_diff` chooses the operand to read as owned addition does,
//!   subtracts it from the other, asks the result's sign, and negates a
//!   negative result, at the cost of negation, the result's width.
//!
//! # Updates of one sign
//!
//! The crate states this refinement on its own, because `before`'s `Sum`
//! rests on it. A *stretch* is the sequence of operations from the start of
//! a storage until the first operation other than an update by a primitive
//! or limbs, a `stored_bits` query, or an `is_known_zero` query; the queries
//! change nothing. Suppose every update in a stretch has contributions of one
//! sign `s`, and the value at its start is zero or has sign `s`.
//!
//! Call the stored form *sign-led* when the value is zero, or its top digit
//! has sign `s` and no digit `d` has `s · d < -B / 2`. Every storage begins
//! sign-led. `new` and `Default` leave every digit zero, and `From` deposits
//! a primitive's magnitude pieces, all of one sign, into zeroed digits.
//! `clone` and `clone_from` copy a normalized value, whose digits all have
//! its sign. A shift of a value held in digits, borrowed or not, streams the
//! value's magnitude, every piece of sign `s`, into zeroed storage, so
//! recentering is the only source of a digit of the other sign, and it
//! leaves that digit within `B / 2` of zero. A small value's shift deposits
//! the value itself, of sign `s`, into zeroed digits.
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
//! The updates' work is then the work of "Rule 1" without its far costs:
//! deposits, paid by the carry potential; positions newly inserted or moved,
//! paid by the held-positions term or prepaid by it; buffer and bitset
//! growth, paid by the rise in `h`, the deposits, and the uncovered-height
//! term; and insertion climbs. By the second fact of "Bitset climbs", an
//! insertion climbs at most one level more than the number of words it
//! fills. A word other than word zero at level `k` covers no position below
//! `64^(k + 1)`, and word zero is nonempty at every level by the first fact,
//! so every word the stretch fills covers a held position at or below the
//! stretch's highest top `P`, and there are at most `P / 63` such words. `P`
//! is at most the storage width, which is the width the stretch began with
//! plus the updates' rises in it. The operation that began the storage is
//! allowed that starting width: creation starts at one position or a
//! primitive's width, `clone_from` costs the copy's width, and a shift costs
//! its result's width whatever its actual work. A small value's `<<=` into
//! storage a clear kept writes only a few digits, yet the shift is still
//! allowed its result's width, and the fills are charged against that
//! allowance.
//!
//! So each update costs O(1) beyond its deposits, its `L`, and its rise in
//! `h`, and the fills total at most what those rises and the starting width
//! pay for. Each primitive update costs amortized O(1), and each limb stream
//! amortized O(`L`) plus its rise in `h`, at any shift. The potential the
//! stretch leaves is within the same sum. The test
//! `a_one_sign_stretch_after_a_shift_climbs_once_per_insertion_and_fill`
//! measures the fill bound with the bitset's level counters.
//!
//! The first comparison after the stretch can still have far costs: at most
//! two run crossings below its top, each O(`1 + log_64 (h + 1)`). "Rule 4"
//! would charge them to the update that wrote the highest position, whose
//! cost in the stretch no longer covers them, so that comparison pays them
//! itself, once. A top digit of 1 whose position below was never written,
//! left by a limb whose low half was zero, is such a case: the scan steps
//! below the top, stops there, and its truncation crosses to the highest
//! position held further down. Later comparisons charge operations after the
//! stretch.
//!
//! A stretch cannot begin at a clear or a read, although each leaves the form
//! sign-led. Both clear the written set, which empties the bitset's words but
//! keeps its storage, and neither begins a storage, so the words a later
//! update refills are paid for by nothing it is allowed. Repeat this round,
//! with `q = 64^m` below a position the storage reached earlier:
//!
//! 1. clear;
//! 2. add one limb at position `q + 2`, which raises the top again without
//!    raising `h`, by a jump that finds position zero held: one level;
//! 3. add one limb at position `q`, in the gap below that run and apart from
//!    position zero, which fills one word at each level from 0 to `m - 1`
//!    and stops at level `m`, at word zero: `m + 1` levels.
//!
//! The next round's clear zeroes the held positions `q` and `q + 2` and
//! examines `m + 1` levels, emptying those words again, which the limb that
//! filled them paid for. Each round costs Θ(`m`), a base-64 logarithm of the
//! storage width, while each update's `L` and rise in `h` stay constant. So
//! those updates need the logarithm of "Rule 4", and a stretch that began at
//! a clear would promise too little.
//! `clear_cycle_climbs_a_level_per_base_64_digit_each_round` in the tests
//! measures `m + 1` levels per round at each `m` it draws.
