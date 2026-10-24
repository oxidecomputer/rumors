use proptest::prelude::*;

use super::*;

/// Generates any member.
fn arb_member() -> impl Strategy<Value = Member> {
    (0..POOL).prop_map(|index| Member::new(index).expect("an index below POOL names a member"))
}

/// Generates either direction.
fn arb_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![Just(Direction::Add), Just(Direction::Sub)]
}

/// Generates any step, every field over its whole domain.
fn arb_step() -> impl Strategy<Value = Step> {
    let primitive = prop_oneof![
        any::<i64>().prop_map(Primitive::I64),
        any::<u64>().prop_map(Primitive::U64),
        any::<i128>().prop_map(Primitive::I128),
        any::<u128>().prop_map(Primitive::U128),
    ];
    let pattern = prop_oneof![
        Just(Pattern::Ones),
        Just(Pattern::TopBit),
        Just(Pattern::Alternating),
        Just(Pattern::Sparse),
        any::<u64>().prop_map(Pattern::Word),
        any::<u64>().prop_map(Pattern::Mixed),
    ];
    let form = prop_oneof![
        Just(Form::Borrowed),
        any::<u64>().prop_map(Form::Shifted),
        Just(Form::OwnedClone),
        Just(Form::OwnedTake),
        Just(Form::Reversed),
    ];
    let query = prop_oneof![
        Just(SignQuery::Sign),
        Just(SignQuery::IsZero),
        Just(SignQuery::IsPositive),
        Just(SignQuery::IsNegative),
        any::<u64>().prop_map(SignQuery::StableUnder),
    ];
    let read = prop_oneof![Just(Read::Signed), Just(Read::Scaled), Just(Read::Hash)];
    prop_oneof![
        (arb_member(), arb_direction(), primitive).prop_map(|(member, direction, value)| {
            Step::Primitive {
                member,
                direction,
                value,
            }
        }),
        (
            arb_member(),
            arb_direction(),
            any::<u64>(),
            pattern,
            any::<u64>()
        )
            .prop_map(|(member, direction, shift, pattern, count)| Step::Limbs {
                member,
                direction,
                shift,
                input: LimbInput { pattern, count },
            }),
        (arb_member(), arb_member(), arb_direction(), form).prop_map(
            |(receiver, operand, direction, form)| Step::Operand {
                receiver,
                operand,
                direction,
                form,
            }
        ),
        (arb_member(), any::<bool>())
            .prop_map(|(member, borrowed)| Step::Negate { member, borrowed }),
        (arb_member(), any::<u64>()).prop_map(|(member, shift)| Step::Shift { member, shift }),
        arb_member().prop_map(|member| Step::Clear { member }),
        (arb_member(), query).prop_map(|(member, query)| Step::Sign { member, query }),
        (arb_member(), any::<i128>())
            .prop_map(|(member, value)| Step::ComparePrimitive { member, value }),
        arb_member().prop_map(|member| Step::Convert { member }),
        (arb_member(), arb_member()).prop_map(|(left, right)| Step::Compare { left, right }),
        (arb_member(), read).prop_map(|(member, read)| Step::Read { member, read }),
        (arb_member(), arb_member())
            .prop_map(|(receiver, source)| Step::Clone { receiver, source }),
        (arb_member(), arb_member())
            .prop_map(|(receiver, source)| Step::CloneFrom { receiver, source }),
        (arb_member(), arb_member(), any::<bool>()).prop_map(|(receiver, operand, take)| {
            Step::AbsDiff {
                receiver,
                operand,
                take,
            }
        }),
        (
            arb_member(),
            proptest::collection::vec(arb_member(), 0..=MAX_SUM_TERMS),
            any::<bool>()
        )
            .prop_map(|(receiver, members, owned)| Step::Sum {
                receiver,
                members,
                owned,
            }),
    ]
}

/// Generates any program of up to 12 steps.
fn arb_program() -> impl Strategy<Value = Program> {
    (any::<u32>(), proptest::collection::vec(arb_step(), 0..12))
        .prop_map(|(repetitions, steps)| Program { repetitions, steps })
}

proptest! {
    /// Decoding a program's encoding returns the program, for every step kind
    /// and every value of every field, so the guest runs exactly the program
    /// the harness built.
    #[test]
    fn decoding_inverts_encoding(program in arb_program()) {
        prop_assert_eq!(Program::decode(&program.encode()), Ok(program));
    }

    /// An encoding with words removed from its end, or with a word appended,
    /// is refused rather than decoded as some other program.
    #[test]
    fn decoding_refuses_a_wrong_length(program in arb_program(), cut in 1usize..8) {
        let words = program.encode();
        let cut = cut.min(words.len());
        prop_assert_eq!(
            Program::decode(&words[..words.len() - cut]),
            Err(DecodeError::Truncated)
        );
        let mut longer = words;
        longer.push(0);
        prop_assert_eq!(Program::decode(&longer), Err(DecodeError::TrailingWords));
    }
}

/// A step naming a member outside the pool, or carrying an unknown tag, is
/// refused as malformed at its own index.
#[test]
fn decoding_refuses_a_step_that_names_nothing() {
    let program = Program {
        repetitions: 1,
        steps: vec![
            Step::Clear {
                member: Member::ALL[0],
            },
            Step::Clear {
                member: Member::ALL[1],
            },
        ],
    };
    let words = program.encode();
    let second_head = 2 + STEP_WORDS;

    let mut bad_member = words.clone();
    bad_member[second_head] = Head {
        tag: tag::CLEAR,
        first: POOL as u8,
        ..Head::default()
    }
    .word();
    assert_eq!(
        Program::decode(&bad_member),
        Err(DecodeError::Malformed { step: 1 })
    );

    let mut bad_tag = words;
    bad_tag[second_head] = Head {
        tag: tag::SUM + 1,
        ..Head::default()
    }
    .word();
    assert_eq!(
        Program::decode(&bad_tag),
        Err(DecodeError::Malformed { step: 1 })
    );
}
