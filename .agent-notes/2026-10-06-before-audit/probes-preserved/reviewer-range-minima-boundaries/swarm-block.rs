
// REVIEWER-SWARM-BEGIN
proptest! {
    /// Reviewer copy of the branch property with a per-case close bound of 3 or 5.
    #[test]
    fn reviewer_swarm_nested_operations(
        steps in prop_oneof![Just(3usize), Just(5usize)].prop_flat_map(|close_bound| {
            prop::collection::vec((step_value(), 0u64..4, 0usize..close_bound), 1..60)
        }),
    ) {
        let mut minima = RangeMinima::<BigInt>::new();
        let mut model = Vec::<BigInt>::new();
        let mut height = BigInt::from(0);
        for (step_value, open_count, close_count) in steps {
            let value = step_value.resolve(&model);
            minima.fold_height(&(&value - &height));
            height = value.clone();
            // An empty tracker needs a new range before it can observe a value.
            let open_count = if model.is_empty() { open_count.max(1) } else { open_count };
            let previous = model.last().cloned();
            let mut expected_retired = Vec::new();
            if open_count > 0 && previous.as_ref().is_some_and(|minimum| &value < minimum) {
                expected_retired.push(previous.clone().unwrap());
            }
            for adjacent in model.windows(2).rev() {
                if adjacent[0] < adjacent[1] && adjacent[0] >= value {
                    expected_retired.push(adjacent[0].clone());
                }
            }

            let mut events = PayloadEvents::default();
            if open_count > 0 {
                minima.open(open_count);
                minima.arm_at_height(
                    &mut events,
                    |events| {
                        events.created += 1;
                        previous.clone().expect("first arming never constructs a payload")
                    },
                    |payload, events| events.retired.push(payload),
                );
                let distinct = previous.as_ref().is_some_and(|minimum| minimum != &value);
                prop_assert_eq!(events.created, usize::from(distinct));
            } else {
                let undercuts = value < *model.last().unwrap();
                prop_assert_eq!(minima.undercuts_here(), undercuts);
                if undercuts {
                    minima.undercut(&mut events, |payload, events| events.retired.push(payload));
                }
            }
            prop_assert_eq!(events.retired, expected_retired);

            for minimum in &mut model {
                *minimum = minimum.clone().min(value.clone());
            }
            model.extend(core::iter::repeat_n(value, open_count as usize));
            for _ in 0..close_count.min(model.len()) {
                close_against_model(&mut minima, &mut model);
            }
            prop_assert_eq!(minima.armed(), !model.is_empty());
            prop_assert!(!minima.has_pending());
        }
        while !model.is_empty() {
            close_against_model(&mut minima, &mut model);
        }
    }
}
// REVIEWER-SWARM-END
