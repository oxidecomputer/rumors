use proptest::prelude::*;

use super::*;

/// The most members a generated sum names.
const SUM_TERMS: usize = 5;

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
            proptest::collection::vec(arb_member(), 0..=SUM_TERMS),
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

    /// An encoding with any number of bytes removed from its end is refused
    /// as truncated, and one with a byte appended is refused as trailing,
    /// rather than either decoding as some other program.
    #[test]
    fn decoding_refuses_a_wrong_length(
        program in arb_program(),
        cut in any::<proptest::sample::Index>(),
    ) {
        let bytes = program.encode();
        let cut = cut.index(bytes.len()) + 1;
        prop_assert_eq!(
            Program::decode(&bytes[..bytes.len() - cut]),
            Err(DecodeError::Truncated)
        );
        let mut longer = bytes;
        longer.push(0);
        prop_assert_eq!(Program::decode(&longer), Err(DecodeError::TrailingBytes));
    }

    /// The limb pattern's lowest and highest limbs do not depend on the
    /// operand's length, for every pattern and every pair of lengths of at
    /// least two limbs, so an
    /// operand's ends behave alike at every scale.
    #[test]
    fn a_patterns_end_limbs_do_not_depend_on_its_length(
        pattern in prop_oneof![
            Just(Pattern::Ones),
            Just(Pattern::TopBit),
            Just(Pattern::Alternating),
            Just(Pattern::Sparse),
            any::<u64>().prop_map(Pattern::Word),
            any::<u64>().prop_map(Pattern::Mixed),
        ],
        short in 2u64..1 << 20,
        long in 2u64..1 << 40,
    ) {
        let (a, b) = (LimbInput { pattern, count: short }, LimbInput { pattern, count: long });
        prop_assert_eq!(a.limb(0), b.limb(0));
        prop_assert_eq!(a.limb(short - 1), b.limb(long - 1));
    }
}

/// Returns the encoding of a program holding one clear of `member`, and the
/// position of the step's tag byte and of its member byte.
fn one_clear(member: Member) -> (Vec<u8>, usize, usize) {
    let bytes = Program {
        repetitions: 1,
        steps: vec![Step::Clear { member }],
    }
    .encode();
    // The repetition count and the step count are one byte each here, so
    // the tag and the member are the last two bytes.
    let tag = bytes.len() - 2;
    (bytes, tag, tag + 1)
}

/// Every value of a step's member byte decodes exactly when it names a pool
/// member, and then to that member.
#[test]
fn a_member_byte_decodes_exactly_when_it_names_a_member() {
    let (bytes, _, member_at) = one_clear(Member::ALL[0]);
    for value in 0..=u8::MAX {
        let mut mutated = bytes.clone();
        mutated[member_at] = value;
        let decoded = Program::decode(&mutated);
        match Member::new(usize::from(value)) {
            Some(member) => assert_eq!(
                decoded.map(|program| program.steps),
                Ok(vec![Step::Clear { member }]),
                "member byte {value}"
            ),
            None => assert_eq!(decoded, Err(DecodeError::Malformed), "member byte {value}"),
        }
    }
}

/// Every value of a step's tag byte either is refused or decodes to a step
/// whose encoding is exactly the bytes decoded, so changing a step's tag
/// never decodes to a step whose encoding differs from the bytes.
#[test]
fn a_tag_byte_decodes_only_to_the_step_it_encodes() {
    let (bytes, tag_at, _) = one_clear(Member::ALL[0]);
    let mut accepted = 0;
    for value in 0..=u8::MAX {
        let mut mutated = bytes.clone();
        mutated[tag_at] = value;
        if let Ok(program) = Program::decode(&mutated) {
            accepted += 1;
            assert_eq!(program.encode(), mutated, "tag byte {value}");
        }
    }
    assert!(accepted >= 1, "the unmutated tag byte decodes");
}
