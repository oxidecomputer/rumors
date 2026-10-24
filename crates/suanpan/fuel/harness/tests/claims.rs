//! `suanpan`'s focused cost claims, each a fixed program judged in fuel.
//!
//! Every test builds its programs from public operations, runs them with
//! [`measure`], which checks every answer against the exact model, and
//! judges the readings in one of the forms the `claims` module defines,
//! named in the test's doc comment. Values are written in 32-bit digits
//! where the claim is about digits: a value at digit `p` sits at bit
//! `32 · p`.

use suanpan_fuel_harness::claims::{
    assert_equal, assert_flat, assert_less, assert_mixed_below, assert_saving_grows,
    assert_spread_below, measure,
};
use suanpan_fuel_protocol::{
    observe_stable, Direction, Form, Integer, LimbInput, Member, Pattern, Primitive, Read,
    SignQuery, Step,
};

use Direction::{Add, Sub};

/// The first pool member.
const A: Member = Member::ALL[0];

/// The second pool member.
const B: Member = Member::ALL[1];

/// Returns an update of `member` by `count` limbs filled by `pattern`, at
/// `shift` bits.
fn limbs(member: Member, direction: Direction, shift: u64, pattern: Pattern, count: u64) -> Step {
    Step::Limbs {
        member,
        direction,
        shift,
        input: LimbInput { pattern, count },
    }
}

/// Returns an update of `member` by the one limb `word`, at `shift` bits.
fn word(member: Member, direction: Direction, shift: u64, word: u64) -> Step {
    limbs(member, direction, shift, Pattern::Word(word), 1)
}

/// Returns an addition of `2^bits − 1` into `member`, for `bits` a multiple
/// of 64: `bits / 32` digits of `2^32 − 1`.
fn ones(member: Member, bits: u64) -> Step {
    assert_eq!(bits % 64, 0, "a run of ones fills whole limbs");
    limbs(member, Add, 0, Pattern::Ones, bits / 64)
}

/// Returns an update of `member` by a primitive.
fn primitive(member: Member, direction: Direction, value: Primitive) -> Step {
    Step::Primitive {
        member,
        direction,
        value,
    }
}

/// Returns the two writes that store zero in `member` as a top cancelling
/// across two digits: `1` at digit `position` and `−2^32` at digit
/// `position − 1`.
///
/// The digits stay stored until a comparison with zero compacts them, so any
/// value added below `position − 1` sits under a working width greater than
/// its own, above a gap of unwritten zeros.
fn cancelled_top(member: Member, position: u64) -> [Step; 2] {
    [
        word(member, Add, 32 * position, 1),
        word(member, Sub, 32 * (position - 1), 1 << 32),
    ]
}

/// Returns the setup that stores 5 in `member` under a top cancelled at
/// digit `position`.
fn five_under_cancelled_top(member: Member, position: u64) -> Vec<Step> {
    let mut steps = cancelled_top(member, position).to_vec();
    steps.push(primitive(member, Add, Primitive::I64(5)));
    steps
}

/// Returns a sign question about `member`.
fn sign(member: Member, query: SignQuery) -> Step {
    Step::Sign { member, query }
}

/// Returns a read of `member`'s magnitude.
fn read(member: Member, read: Read) -> Step {
    Step::Read { member, read }
}

/// Returns an update of `receiver` by the accumulator `operand`.
fn operand(receiver: Member, operand: Member, direction: Direction, form: Form) -> Step {
    Step::Operand {
        receiver,
        operand,
        direction,
        form,
    }
}

/// Returns `receiver.abs_diff(operand)`, taking the operand.
fn abs_diff(receiver: Member, operand: Member) -> Step {
    Step::AbsDiff {
        receiver,
        operand,
        take: true,
    }
}

/// Returns `steps` repeated `times` times.
fn repeat(steps: &[Step], times: usize) -> Vec<Step> {
    steps
        .iter()
        .cloned()
        .cycle()
        .take(steps.len() * times)
        .collect()
}

/// Concatenates setups.
fn join(parts: &[&[Step]]) -> Vec<Step> {
    parts.concat()
}

/// (F-eq, F-lt) A scaled read and a clear cost the same wherever the value
/// sits, while an unscaled read walks the unwritten prefix below it.
///
/// A two-limb value parked at bit 32,000, 64,000, or 128,000 is read with
/// `scaled_signed_magnitude` and then cleared: both readings are equal at
/// every position, since the scaled read factors the prefix out as a shift
/// and the clear visits only the written positions. A `signed_magnitude`
/// read of the same value returns every limb of the prefix, so the scaled
/// read is cheaper, by more as the prefix grows.
#[test]
fn a_scaled_read_and_a_clear_cost_the_written_span() {
    let park = |shift| vec![limbs(A, Add, shift, Pattern::Word(7), 2)];
    let shifts = [32_000, 64_000, 128_000];
    let scaled: Vec<(u64, Vec<u64>)> = shifts
        .iter()
        .map(|&shift| {
            let measured = [read(A, Read::Scaled), Step::Clear { member: A }];
            (shift, measure(&park(shift), &measured).fuel)
        })
        .collect();
    assert_equal("a scaled read, then a clear", &scaled);
    let rows: Vec<(u64, u64, u64)> = scaled
        .iter()
        .map(|(shift, fuel)| {
            let unscaled = measure(&park(*shift), &[read(A, Read::Signed)]).total();
            (*shift, fuel[0], unscaled)
        })
        .collect();
    assert_saving_grows("the scaled read below the unscaled read", &rows);
}

/// (F-eq) Alternating one-limb updates across an unwritten gap cost the same
/// at every width of the gap, with or without a written digit below it.
///
/// The value `2^s`, alone or above a 5 at digit 0, takes a thousand
/// alternating subtractions and additions of the limb 1 at shift `s`; each
/// subtraction empties the top, so the next sign-relevant position lies
/// across the gap. Every step's reading is equal at `s` of 32,000, 64,000,
/// and 1,024,000 bits: no update pays for the gap's width. A one-limb stream
/// takes the word update's path, so this is also the claim that a one-limb
/// stream costs what a word costs at any shift.
#[test]
fn alternating_shifted_writes_cost_the_operand_not_the_gap() {
    const PAIRS: usize = 1_000;
    for low in [None, Some(primitive(A, Add, Primitive::I64(5)))] {
        let rows: Vec<(u64, Vec<u64>)> = [32_000, 64_000, 1_024_000]
            .iter()
            .map(|&shift| {
                let setup: Vec<Step> = low
                    .iter()
                    .cloned()
                    .chain([word(A, Add, shift, 1)])
                    .collect();
                let pair = [word(A, Sub, shift, 1), word(A, Add, shift, 1)];
                (shift, measure(&setup, &repeat(&pair, PAIRS)).fuel)
            })
            .collect();
        let name = format!(
            "alternating one-limb updates, digit 0 {}",
            if low.is_some() { "holding 5" } else { "empty" }
        );
        assert_equal(&name, &rows);
    }
}

/// (F-eq) A sign query of zero spelled across two digits above an unwritten
/// gap costs the same at every height of the gap.
///
/// `2^(32(p + 1)) − 2^32 · 2^(32p)` is zero, stored as two digits above `p`
/// unwritten digits. The query reads and clears the two digits and crosses the
/// gap in one skip, so its reading is equal at `p` of 1,000, 2,000, and
/// 4,000.
#[test]
fn a_sign_query_crosses_an_unwritten_gap_in_one_skip() {
    let rows: Vec<(u64, u64)> = [1_000, 2_000, 4_000]
        .iter()
        .map(|&position| {
            let setup = cancelled_top(A, position + 1);
            (
                position,
                measure(&setup, &[sign(A, SignQuery::Sign)]).total(),
            )
        })
        .collect();
    assert_equal("a sign query over a cancelled top", &rows);
}

/// (F-eq) An accumulator operand costs its own digits, in every spelling, at
/// any receiver width and shift.
///
/// The receiver holds `2^k − 1`, every digit full, at `k` of 2,048 and 4,096
/// bits; the operand is a two-limb value of four digits. Each spelling's
/// readings are equal at both receiver widths: `+=` and `-=` by reference
/// (the first `+=` compacting the operand, the repeated one reading the
/// compacted form), the owned sums and differences on a clone and on the
/// taken operand, the narrow side minus the wide one, the borrowed operand
/// on the left, and both differences with a negated right operand. A
/// shifted update, its inverse, and the update again follow; the first grows
/// the storage to the shift, a cost paid once that grows with the shift, and
/// the inverse and the repeated update read the same at both widths and at
/// shifts of 32,000 and 64,000 bits. This is the rule that an update costs
/// what it adds, and the ownership rule that an owned operation reads the
/// narrower operand, whichever side it stands on.
#[test]
fn an_accumulator_operand_costs_its_digits_at_any_receiver_width_and_shift() {
    let narrow = limbs(B, Add, 0, Pattern::Word((1 << 32) | 1), 2);
    let negate = |member| Step::Negate {
        member,
        borrowed: false,
    };
    let spellings: [(&str, Vec<Step>, Vec<Step>); 11] = [
        (
            "a += &b, twice",
            vec![],
            vec![
                operand(A, B, Add, Form::Borrowed),
                operand(A, B, Add, Form::Borrowed),
            ],
        ),
        ("a -= &b", vec![], vec![operand(A, B, Sub, Form::Borrowed)]),
        (
            "a + b.clone()",
            vec![],
            vec![operand(A, B, Add, Form::OwnedClone)],
        ),
        (
            "a - b.clone()",
            vec![],
            vec![operand(A, B, Sub, Form::OwnedClone)],
        ),
        ("a + b", vec![], vec![operand(A, B, Add, Form::OwnedTake)]),
        ("a - b", vec![], vec![operand(A, B, Sub, Form::OwnedTake)]),
        ("b - a", vec![], vec![operand(B, A, Sub, Form::OwnedTake)]),
        ("&b + a", vec![], vec![operand(A, B, Add, Form::Reversed)]),
        ("&b - a", vec![], vec![operand(A, B, Sub, Form::Reversed)]),
        (
            "a - (-b)",
            vec![negate(B)],
            vec![operand(A, B, Sub, Form::OwnedTake)],
        ),
        (
            "b - (-a)",
            vec![negate(A)],
            vec![operand(B, A, Sub, Form::OwnedTake)],
        ),
    ];
    for (name, extra, measured) in &spellings {
        let rows: Vec<(u64, Vec<u64>)> = [2_048, 4_096]
            .iter()
            .map(|&bits| {
                let setup = join(&[&[ones(A, bits), narrow.clone()], extra]);
                (bits, measure(&setup, measured).fuel)
            })
            .collect();
        assert_equal(&format!("`{name}` by receiver width"), &rows);
    }
    for direction in [Add, Sub] {
        let inverse = match direction {
            Add => Sub,
            Sub => Add,
        };
        let rows: Vec<((u64, u64), Vec<u64>)> = [(2_048, 32_000), (2_048, 64_000), (4_096, 32_000)]
            .iter()
            .map(|&(bits, shift)| {
                let setup = [ones(A, bits), narrow.clone()];
                let measured = [direction, inverse, direction]
                    .map(|direction| operand(A, B, direction, Form::Shifted(shift)));
                ((bits, shift), measure(&setup, &measured).fuel[1..].to_vec())
            })
            .collect();
        assert_equal(
            &format!("a shifted {direction:?} into grown storage, by (receiver width, shift)"),
            &rows,
        );
    }
}

/// (F-lt, F-bound) An owned addition of two values with equal digit counts
/// reads its right operand and keeps the left one's storage.
///
/// Of two `n`-limb values whose top digits share a position, one is dense,
/// with all `2n` digits nonzero, and one is sparse, with only its lowest and
/// highest digits nonzero. Reading an operand into a receiver costs a
/// deposit per nonzero digit of the operand, and an insertion into the
/// receiver's record of written positions for each digit it writes that the
/// receiver never wrote. So the sum with the sparse value on the right is
/// the cheaper one, and by more at 32 limbs than at 16; were a tie routed to
/// read the left operand, the order of the two programs would reverse.
///
/// Each owned sum also lies closer to `left += &right`, the same reading
/// routed through a borrow, than to `right += &left`, the reading the other
/// routing would do: its distance from the first is less than half the
/// distance between the two. That bound sits midway between the two
/// routings, so it holds whatever fixed cost the owned addition adds, as
/// long as that cost stays below half of what the routings differ by.
#[test]
fn an_owned_addition_on_a_tie_reads_the_right_operand() {
    let mut rows = Vec::new();
    for count in [16, 32] {
        let dense = |member| vec![limbs(member, Add, 0, Pattern::Word(0x5_0000_0005), count)];
        let sparse = |member| {
            vec![
                word(member, Add, 0, 5),
                word(member, Add, 64 * count - 32, 5),
            ]
        };
        let mut owned_sums = Vec::new();
        for (name, setup) in [
            (
                "a dense left and a sparse right",
                join(&[&dense(A), &sparse(B)]),
            ),
            (
                "a sparse left and a dense right",
                join(&[&sparse(A), &dense(B)]),
            ),
        ] {
            let reading = |step| measure(&setup, &[step]).total();
            let owned = reading(operand(A, B, Add, Form::OwnedTake));
            let reads_right = reading(operand(A, B, Add, Form::Borrowed));
            let reads_left = reading(operand(B, A, Add, Form::Borrowed));
            assert_less(
                &format!(
                    "twice the owned sum's distance from `left += &right` vs the routings' \
                     distance, {name}, {count} limbs"
                ),
                2 * owned.abs_diff(reads_right),
                reads_left.abs_diff(reads_right),
            );
            owned_sums.push(owned);
        }
        rows.push((count, owned_sums[0], owned_sums[1]));
    }
    assert_saving_grows(
        "the tie reading the sparse right operand below the tie reading the dense one",
        &rows,
    );
}

/// (F-lin, F-eq) A magnitude read, `<<=`, and a clear cost the held digits,
/// `<<=` costs the same at a shift of 32 bits and of 33, and a wide shift
/// costs the result's width.
///
/// `2^k − 1` fills `k / 32` digits. A `signed_magnitude` read, a shift by 32
/// bits, and a clear each cost at most 1.25 times as much per held digit at
/// 128 digits as at 64. A shift by 33 bits, which moves every digit by one
/// bit more than a digit boundary, costs what a shift by 32 bits costs.
/// Shifting the 64 digits by 128,000 bits costs at most 1.25 times as much
/// per result digit as shifting them by 64,000, since `<<=` costs the width
/// of the value it produces, its zeroed low digits included.
#[test]
fn width_producing_operations_cost_the_held_digits() {
    let readings = |bits: u64| {
        let measured = [
            read(A, Read::Signed),
            Step::Shift {
                member: A,
                shift: 32,
            },
            Step::Clear { member: A },
        ];
        measure(&[ones(A, bits)], &measured).fuel
    };
    let (small, large) = (readings(2_048), readings(4_096));
    for (index, name) in ["a signed read", "<<= 32", "a clear"].iter().enumerate() {
        assert_flat(
            &format!("{name} per held digit"),
            (small[index], 64),
            (large[index], 128),
        );
    }
    let rows: Vec<(u64, u64)> = [32, 33]
        .iter()
        .map(|&shift| {
            let measured = [Step::Shift { member: A, shift }];
            (shift, measure(&[ones(A, 2_048)], &measured).total())
        })
        .collect();
    assert_equal("<<= by shift, at 64 held digits", &rows);
    let per_result_digit = |shift: u64| {
        let measured = [Step::Shift { member: A, shift }];
        let fuel = measure(&[ones(A, 2_048)], &measured).total();
        (fuel, (2_048 + shift) / 32)
    };
    assert_flat(
        "<<= per result digit, by shift",
        per_result_digit(64_000),
        per_result_digit(128_000),
    );
}

/// (F-eq) Negating an owned accumulator held in digits costs the same at any
/// width and in either digit form, and subtracting it from a primitive costs
/// the same at any width.
///
/// The values are `2^k − 1` filling 64 and 128 digits, and 5 under a top
/// that cancels at those digits, uncompacted. Owned negation flips the
/// digits' flag, reading no digit: all four readings are equal. `5 − a`
/// negates `a` and adds 5: its readings are equal at both widths for each
/// form. A value small enough to need no digits takes a path of its own,
/// with no width to compare.
#[test]
fn negation_costs_the_same_at_any_width() {
    #[allow(
        clippy::type_complexity,
        reason = "a name for a pair of a label and a setup would carry no meaning"
    )]
    let forms: [(&str, fn(u64) -> Vec<Step>); 2] = [
        ("2^k - 1", |digits| vec![ones(A, 32 * digits)]),
        ("5 under a cancelled top", |digits| {
            five_under_cancelled_top(A, digits)
        }),
    ];
    let mut negations = Vec::new();
    for (name, setup) in forms {
        let mut differences = Vec::new();
        for digits in [64, 128] {
            let negate = Step::Negate {
                member: A,
                borrowed: false,
            };
            negations.push(((name, digits), measure(&setup(digits), &[negate]).total()));
            let difference = Step::ReversedPrimitive {
                member: A,
                direction: Sub,
                value: Primitive::U64(5),
                borrowed: false,
            };
            differences.push((digits, measure(&setup(digits), &[difference]).total()));
        }
        assert_equal(&format!("5 minus {name}, by digits"), &differences);
    }
    assert_equal("owned negation, by (form, digits)", &negations);
}

/// (F-eq) Updates by `u64::MAX` at the top of a full run of digits, with a
/// sign query after each, cost the same at every cliff height, and every
/// round after the first costs the same.
///
/// `u64::MAX` is the widest word update and starts the deepest carry a word
/// can: added to `2^k − 1`, it carries out of the run. Rounds of `+=
/// u64::MAX`, a sign query, `-= u64::MAX`, and a sign query cost the same,
/// step for step, over the first thousand rounds at `k` of 4,096 and 8,192
/// bits, and in a run of two thousand rounds at 8,192 bits, every round
/// after the first costs what the second does: each round repays exactly
/// what it disturbs, so the cost is flat in the stream's length.
#[test]
fn u64_comb_rounds_are_flat_in_cliff_height_and_stream_length() {
    const ROUNDS: usize = 1_000;
    let round = [
        primitive(A, Add, Primitive::U64(u64::MAX)),
        sign(A, SignQuery::Sign),
        primitive(A, Sub, Primitive::U64(u64::MAX)),
        sign(A, SignQuery::Sign),
    ];
    let low = measure(&[ones(A, 4_096)], &repeat(&round, ROUNDS)).fuel;
    let high = measure(&[ones(A, 8_192)], &repeat(&round, 2 * ROUNDS)).fuel;
    assert_equal(
        "the first thousand comb rounds, by cliff height",
        &[(4_096, low.clone()), (8_192, high[..low.len()].to_vec())],
    );
    let rounds: Vec<(usize, u64)> = high
        .chunks(round.len())
        .map(|round| round.iter().sum())
        .enumerate()
        .skip(1)
        .collect();
    assert_equal("each comb round after the first, by round", &rounds);
}

/// (F-eq) A limb update costs its operand's limbs at any width of the value
/// it updates.
///
/// Adding and then subtracting a two-limb operand into `2^k − 1` cost the
/// same, step for step, at `k` of 2,048 and 4,096 bits.
#[test]
fn limb_writes_cost_the_operand_at_any_held_width() {
    let rows: Vec<(u64, Vec<u64>)> = [2_048, 4_096]
        .iter()
        .map(|&bits| {
            let measured = [
                limbs(A, Add, 0, Pattern::Word(3), 2),
                limbs(A, Sub, 0, Pattern::Word(3), 2),
            ];
            (bits, measure(&[ones(A, bits)], &measured).fuel)
        })
        .collect();
    assert_equal("a two-limb update, by held width", &rows);
}

/// (F-eq) Repeated stability queries cost the same after the first, and the
/// same at every height of the value.
///
/// The value is `2^t`, at `t` of 2,048 and 4,096 bits. The first query, at
/// 128 bits, simplifies the stored top; a thousand queries at 64 bits
/// follow. Every query answers that the sign is stable, every reading is
/// equal at both heights, and the thousand later readings are equal to each
/// other.
#[test]
fn repeated_stability_queries_cost_the_same_after_the_first() {
    const QUERIES: usize = 1_000;
    let measured: Vec<Step> = [sign(A, SignQuery::StableUnder(128))]
        .into_iter()
        .chain(repeat(&[sign(A, SignQuery::StableUnder(64))], QUERIES))
        .collect();
    let rows: Vec<(u64, Vec<u64>)> = [2_048, 4_096]
        .iter()
        .map(|&height| {
            let run = measure(&[word(A, Add, height, 1)], &measured);
            let stable = observe_stable(Some(std::cmp::Ordering::Greater));
            assert!(
                run.observations.iter().all(|&answer| answer == stable),
                "2^{height} is stable under 2^128"
            );
            (height, run.fuel)
        })
        .collect();
    let readings = assert_equal("stability queries, by height", &rows);
    let later: Vec<(usize, u64)> = readings.into_iter().enumerate().skip(1).collect();
    assert_equal("each later stability query, by index", &later);
}

/// (F-eq, F-lt) A top coefficient large enough to decide stability decides
/// before reading the digits below it, and a smaller one reads further.
///
/// The value is 5 at digit `j + 4` above `j` digits of `2^32 − 1`, at `j` of
/// 64 and 128. A query at `96 + 32j` bits answers stable, and one at
/// `128 + 32j` bits, one digit short, refuses; each reads the same at both
/// `j`. With nothing below, `2 · 2^128` refuses the query at 96 bits that
/// `5 · 2^128` answers, and its refusal costs more, since the coefficient 2
/// cannot decide alone.
#[test]
fn a_decision_bound_top_decides_on_the_first_touch() {
    let stable = observe_stable(Some(std::cmp::Ordering::Greater));
    let refused = observe_stable(None);
    for (offset, expected) in [(96, stable), (128, refused)] {
        let rows: Vec<(u64, u64)> = [64, 128]
            .iter()
            .map(|&digits| {
                let setup = [ones(A, 32 * digits), word(A, Add, 32 * (digits + 4), 5)];
                let query = [sign(A, SignQuery::StableUnder(offset + 32 * digits))];
                let run = measure(&setup, &query);
                assert_eq!(
                    run.observations,
                    [expected],
                    "the query {offset} bits above the lower digits"
                );
                (digits, run.total())
            })
            .collect();
        assert_equal(
            &format!("a query {offset} bits above the lower digits, by their count"),
            &rows,
        );
    }
    let query = [sign(A, SignQuery::StableUnder(96))];
    let five = measure(&[word(A, Add, 128, 5)], &query);
    let two = measure(&[word(A, Add, 128, 2)], &query);
    assert_eq!(
        (five.observations[0], two.observations[0]),
        (stable, refused)
    );
    assert_less(
        "the coefficient 5 deciding below the coefficient 2 refusing",
        five.total(),
        two.total(),
    );
}

/// (F-eq) After a sign query compacts a cancelling prefix, every later sign
/// query costs the same at every width of the prefix.
///
/// `2^k − (2^k − 1)` is 1, spelled across `k / 32` digits that cancel. The
/// first sign query pays to compact them, once; each of the thousand that
/// follow reads the same at `k` of 2,048 and 4,096 bits, and the same as
/// every other. A fold that did not compact would walk the prefix on every
/// query.
#[test]
fn sign_queries_after_the_first_do_not_rescan_a_cancelled_prefix() {
    const QUERIES: usize = 1_000;
    let rows: Vec<(u64, Vec<u64>)> = [2_048, 4_096]
        .iter()
        .map(|&bits| {
            let setup = [
                word(A, Add, bits, 1),
                limbs(A, Sub, 0, Pattern::Ones, bits / 64),
            ];
            let queries = repeat(&[sign(A, SignQuery::Sign)], QUERIES + 1);
            (bits, measure(&setup, &queries).fuel[1..].to_vec())
        })
        .collect();
    let later = assert_equal("each sign query after the first, by prefix width", &rows);
    let by_index: Vec<(usize, u64)> = later.into_iter().enumerate().collect();
    assert_equal("each sign query after the first, by index", &by_index);
}

/// (F-eq, F-lin) A magnitude read costs the result's width, not a cancelled
/// working width, and a repeated read costs the same as the second.
///
/// `2^(32r) − 1` sits under a top that cancels at digit `p`. Three reads in
/// a row, by `signed_magnitude` or by `scaled_signed_magnitude`, read the
/// same, step for step, at `p` of 1,000 and 2,000; the first compacts the
/// cancelled top and the third costs what the second does, since the read
/// keeps its compacted form. The repeated read costs at most 1.25 times as
/// much per result digit at `r` of 128 as at 64.
#[test]
fn magnitude_reads_cost_the_result_width() {
    for kind in [Read::Signed, Read::Scaled] {
        let mut repeated = Vec::new();
        for digits in [64, 128] {
            let rows: Vec<(u64, Vec<u64>)> = [1_000, 2_000]
                .iter()
                .map(|&position| {
                    let setup = join(&[&cancelled_top(A, position), &[ones(A, 32 * digits)]]);
                    (position, measure(&setup, &repeat(&[read(A, kind)], 3)).fuel)
                })
                .collect();
            let reads = assert_equal(
                &format!("three {kind:?} reads of {digits} digits, by cancelled height"),
                &rows,
            );
            assert_equal(
                &format!("the second and third {kind:?} reads of {digits} digits"),
                &[(2, reads[1]), (3, reads[2])],
            );
            repeated.push((reads[1], digits));
        }
        assert_flat(
            &format!("a repeated {kind:?} read per result digit"),
            repeated[0],
            repeated[1],
        );
    }
}

/// (F-eq) A conversion to any primitive type costs what it reads, not the
/// working width.
///
/// `2^(32p)` fits no primitive: three conversions in a row to each of
/// `i64`, `u64`, `i128`, and `u128` refuse, and read the same, step for
/// step, at `p` of 1,000 and 2,000, the second and third the same as each
/// other, since the first returns the value compacted. 5 under a top that
/// cancels at digit `p` converts to each type at the same cost at both `p`.
#[test]
fn primitive_conversions_cost_what_they_read() {
    for target in [Integer::I64, Integer::U64, Integer::I128, Integer::U128] {
        let convert = Step::Convert { member: A, target };
        let refusals: Vec<(u64, Vec<u64>)> = [1_000, 2_000]
            .iter()
            .map(|&position| {
                let setup = [word(A, Add, 32 * position, 1)];
                (
                    position,
                    measure(&setup, &repeat(std::slice::from_ref(&convert), 3)).fuel,
                )
            })
            .collect();
        let readings = assert_equal(
            &format!("three refused {target:?} conversions, by height"),
            &refusals,
        );
        assert_equal(
            &format!("the second and third refused {target:?} conversions"),
            &[(2, readings[1]), (3, readings[2])],
        );
        let successes: Vec<(u64, u64)> = [1_000, 2_000]
            .iter()
            .map(|&position| {
                let setup = five_under_cancelled_top(A, position);
                (
                    position,
                    measure(&setup, std::slice::from_ref(&convert)).total(),
                )
            })
            .collect();
        assert_equal(
            &format!("converting 5 under a cancelled top to {target:?}, by height"),
            &successes,
        );
    }
}

/// (F-eq) An accumulator operand costs its value's width, not a cancelled
/// working width, and adding it again costs only its compacted form.
///
/// The operand stores 5 under a top that cancels at digit `p`; the receiver
/// holds `2^2048 − 1`. Three `+=` by reference read the same, step for step,
/// at `p` of 1,000 and 2,000, and the third the same as the second.
#[test]
fn accumulator_operands_cost_their_value_width() {
    let rows: Vec<(u64, Vec<u64>)> = [1_000, 2_000]
        .iter()
        .map(|&position| {
            let setup = join(&[&five_under_cancelled_top(B, position), &[ones(A, 2_048)]]);
            let additions = repeat(&[operand(A, B, Add, Form::Borrowed)], 3);
            (position, measure(&setup, &additions).fuel)
        })
        .collect();
    let readings = assert_equal("three additions of 5 under a cancelled top", &rows);
    assert_equal(
        "the second and third additions",
        &[(2, readings[1]), (3, readings[2])],
    );
}

/// (F-eq) A cancelled top never widens what an operation reads when it
/// chooses an operand by width.
///
/// At `k` of 1,000 and 2,000 digits: three rounds on one accumulator each
/// rebuild a cancelled top at digit `k` above the 5 and compare the 5 with
/// `2^(32k)`; the comparisons, and the second and third rounds' writes, read
/// the same at both `k`. An owned addition and an `abs_diff` meet 5 under a
/// top cancelled at `k` with `2^(32(k/2))`, which is wider in value but
/// narrower in working width; each reads the same at both `k`, where
/// reading the operand of narrower working width would read `k / 2` digits.
#[test]
fn a_cancelled_top_never_widens_what_a_choice_of_operand_reads() {
    let rounds: Vec<(u64, Vec<u64>)> = [1_000, 2_000]
        .iter()
        .map(|&position| {
            let setup = [
                primitive(A, Add, Primitive::I64(5)),
                word(B, Add, 32 * position, 1),
            ];
            let round = join(&[
                &cancelled_top(A, position),
                &[Step::Compare { left: A, right: B }],
            ]);
            let fuel = measure(&setup, &repeat(&round, 3)).fuel;
            (position, fuel[round.len()..].to_vec())
        })
        .collect();
    assert_equal(
        "the second and third rebuild-and-compare rounds, by height",
        &rounds,
    );
    let first_compares: Vec<(u64, u64)> = [1_000, 2_000]
        .iter()
        .map(|&position| {
            let setup = [
                primitive(A, Add, Primitive::I64(5)),
                word(B, Add, 32 * position, 1),
            ];
            let round = join(&[
                &cancelled_top(A, position),
                &[Step::Compare { left: A, right: B }],
            ]);
            (position, measure(&setup, &round).fuel[2])
        })
        .collect();
    assert_equal("the first round's comparison, by height", &first_compares);
    for (name, step) in [
        ("an owned addition", operand(A, B, Add, Form::OwnedTake)),
        ("abs_diff", abs_diff(A, B)),
    ] {
        let rows: Vec<(u64, u64)> = [1_000, 2_000]
            .iter()
            .map(|&position| {
                let setup = join(&[
                    &five_under_cancelled_top(A, position),
                    &[word(B, Add, 32 * (position / 2), 1)],
                ]);
                (
                    position,
                    measure(&setup, std::slice::from_ref(&step)).total(),
                )
            })
            .collect();
        assert_equal(
            &format!("{name} of 5 under a cancelled top and 2^(32k/2), by k"),
            &rows,
        );
    }
}

/// (F-eq) Comparing with a far narrower accumulator reads only the wider
/// one's top.
///
/// `5 · 2^(32k)` against `2^100`, in both orders, reads the same, step for
/// step, at `k` of 1,000 and 2,000: the top digit decides at once, where a
/// comparison that subtracted would read `k` digits.
#[test]
fn far_wider_comparisons_read_only_the_wider_top() {
    let rows: Vec<(u64, Vec<u64>)> = [1_000, 2_000]
        .iter()
        .map(|&digits| {
            let setup = [
                word(A, Add, 32 * digits, 5),
                primitive(B, Add, Primitive::U128(1 << 100)),
            ];
            let measured = [
                Step::Compare { left: A, right: B },
                Step::Compare { left: B, right: A },
            ];
            (digits, measure(&setup, &measured).fuel)
        })
        .collect();
    assert_equal("far-wider comparisons in both orders, by k", &rows);
}

/// (F-eq, F-lin) A comparison decides from a lead of two digits when the
/// wider side's top decides, and otherwise subtracts at a cost linear in the
/// narrower side.
///
/// The narrow operand is `n` digits of `2^32 − 1`; the wide one is a single
/// top digit `t` at the given lead above it. Leading by three digits, or by
/// two with `t = 5`, the top decides: the comparison reads the same at `n`
/// of 4 and 40, in either order. Leading by two with `t = 2`, or by one
/// with `t = 5`, the comparison subtracts, and costs at most 1.25 times as
/// much per narrow digit at `n` of 80 as at 40, in either order.
#[test]
fn comparisons_decide_from_a_two_digit_lead() {
    let compare = |lead: u64, top: u64, digits: u64, reversed: bool| {
        let setup = [
            limbs(B, Add, 0, Pattern::Ones, digits / 2),
            word(A, Add, 32 * (digits + lead - 1), top),
        ];
        let (left, right) = if reversed { (B, A) } else { (A, B) };
        measure(&setup, &[Step::Compare { left, right }]).total()
    };
    for reversed in [false, true] {
        for (lead, top) in [(3, 5), (2, 5)] {
            let rows: Vec<(u64, u64)> = [4, 40]
                .iter()
                .map(|&digits| (digits, compare(lead, top, digits, reversed)))
                .collect();
            assert_equal(
                &format!("a lead of {lead} with top {top}, reversed {reversed}, by n"),
                &rows,
            );
        }
        for (lead, top) in [(2, 2), (1, 5)] {
            assert_flat(
                &format!("a lead of {lead} with top {top}, reversed {reversed}, per narrow digit"),
                (compare(lead, top, 40, reversed), 40),
                (compare(lead, top, 80, reversed), 80),
            );
        }
    }
}

/// (F-eq) `abs_diff` of a far wider value and a narrow one reads only the
/// wider value's top and the narrower operand, at any distance between
/// them.
///
/// `5 · 2^(32k)` and `n` digits of `2^32 − 1`, with either as the receiver:
/// the reading is equal at `k` of 1,000 and 2,000, for `n` of 4 and of 40.
#[test]
fn far_abs_diff_reads_the_wider_top_and_the_narrower_operand() {
    for digits in [4, 40] {
        for reversed in [false, true] {
            let rows: Vec<(u64, u64)> = [1_000, 2_000]
                .iter()
                .map(|&k| {
                    let setup = [
                        word(A, Add, 32 * k, 5),
                        limbs(B, Add, 0, Pattern::Ones, digits / 2),
                    ];
                    let step = if reversed {
                        abs_diff(B, A)
                    } else {
                        abs_diff(A, B)
                    };
                    (k, measure(&setup, &[step]).total())
                })
                .collect();
            assert_equal(
                &format!("abs_diff with {digits} narrow digits, reversed {reversed}, by k"),
                &rows,
            );
        }
    }
}

/// (F-eq, F-bound) `abs_diff` costs the narrower operand alone, whichever
/// operand is the smaller value, at any width of the wider.
///
/// The narrow operand is `n` digits of `2^32 − 1`. 5 under a top cancelled
/// at digit `p` is wider in working width and smaller in value; once
/// compacted it is the narrower operand. Against it, `abs_diff` reads the
/// same at `p` of 1,000 and 2,000 for each `n` of 4, 40, 400, and 4,000, and
/// across those `n` its readings spread by less than half of `4,000 − 4`: a
/// read of the `n`-digit operand would cost at least one instruction per
/// digit, and so would spread by at least `4,000 − 4`. `−2^(32p)`, wider and
/// smaller, reads the same at both `p` for each `n` below 1,000, where it
/// stays the wider operand.
#[test]
fn abs_diff_costs_the_narrower_operand_whichever_is_smaller() {
    const NARROW_DIGITS: [u64; 4] = [4, 40, 400, 4_000];
    const POSITIONS: [u64; 2] = [1_000, 2_000];
    let narrow = |digits: u64| limbs(B, Add, 0, Pattern::Ones, digits / 2);
    let fives: Vec<(u64, u64)> = NARROW_DIGITS
        .iter()
        .map(|&digits| {
            let rows: Vec<(u64, u64)> = POSITIONS
                .iter()
                .map(|&position| {
                    let setup = join(&[&five_under_cancelled_top(A, position), &[narrow(digits)]]);
                    (position, measure(&setup, &[abs_diff(A, B)]).total())
                })
                .collect();
            let name = format!("abs_diff of 5 under a cancelled top and {digits} digits, by p");
            (digits, assert_equal(&name, &rows))
        })
        .collect();
    let widest = NARROW_DIGITS[NARROW_DIGITS.len() - 1];
    assert_spread_below(
        "abs_diff of 5 under a cancelled top, by n",
        &fives,
        (widest - NARROW_DIGITS[0]) / 2,
    );
    for digits in NARROW_DIGITS
        .into_iter()
        .filter(|&digits| digits < POSITIONS[0])
    {
        let rows: Vec<(u64, u64)> = POSITIONS
            .iter()
            .map(|&position| {
                let setup = [word(A, Sub, 32 * position, 1), narrow(digits)];
                (position, measure(&setup, &[abs_diff(A, B)]).total())
            })
            .collect();
        assert_equal(
            &format!("abs_diff of -2^(32p) and {digits} digits, by p"),
            &rows,
        );
    }
}

/// (F-mix) A sign flip across a wide gap has no rounds-by-width product.
///
/// Holding `−1`, each round adds `2^d`, asks the sign, subtracts `2^d`, and
/// asks again, so every query flips the sign of a value whose deciding
/// digits lie `d` bits apart. The grid runs `n` of 64 and 128 rounds and `d`
/// of `2^22` and `2^23` bits. A fixed cost per round and storage growth to
/// `d`, paid once, are additive and add nothing to the mixed second
/// difference. A rescan of the gap on one query per round, at one fuel per
/// digit, would add `n · d / 32` to it; the bound is half that. The gap is
/// wide and the rounds few so that such a rescan would outweigh the rounds'
/// fixed cost in every cell.
#[test]
fn a_sign_flip_across_a_gap_has_no_width_product() {
    let total = |rounds: usize, gap: u64| {
        let round = [
            word(A, Add, gap, 1),
            sign(A, SignQuery::Sign),
            word(A, Sub, gap, 1),
            sign(A, SignQuery::Sign),
        ];
        let setup = [primitive(A, Sub, Primitive::U64(1))];
        measure(&setup, &repeat(&round, rounds)).total()
    };
    let (rounds, gap) = (64, 1 << 22);
    let rescan = i128::try_from(rounds).expect("a round count fits i128") * i128::from(gap / 32);
    assert_mixed_below(
        "a sign flip across a gap, over (rounds, gap)",
        [
            total(rounds, gap),
            total(2 * rounds, gap),
            total(rounds, 2 * gap),
            total(2 * rounds, 2 * gap),
        ],
        rescan / 2,
    );
}
