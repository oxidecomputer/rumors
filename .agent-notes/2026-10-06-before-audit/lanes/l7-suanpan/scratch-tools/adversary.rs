
    /// Scale every shift in a program by `factor`, keeping its structure.
    fn scaled(ops: &[Op], factor: u64) -> Vec<Op> {
        ops.iter()
            .map(|op| {
                let mut op = op.clone();
                match &mut op {
                    Op::Limbs { shift, .. } | Op::Acc { shift, .. } | Op::Shl { shift, .. } => {
                        *shift *= factor
                    }
                    Op::Park { index, .. } => *index *= factor,
                    Op::Stable { bits, .. } => *bits = bits.saturating_mul(factor),
                    _ => {}
                }
                op
            })
            .collect()
    }

    /// Hill-climb on the touch-to-work ratio, then rescale the best program.
    ///
    /// Investigative: prints the best ratios; asserts nothing.
    #[test]
    #[ignore]
    fn l7_adversarial_touch_search() {
        use proptest::strategy::ValueTree;
        use proptest::test_runner::{Config, TestRunner};
        let iterations: usize = std::env::var("L7_ITERS").ok().and_then(|s| s.parse().ok()).unwrap_or(2_000);
        let seeds: u64 = std::env::var("L7_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
        for seed in 0..seeds {
            let mut runner = TestRunner::new_with_rng(
                Config::default(),
                proptest::test_runner::TestRng::from_seed(
                    proptest::test_runner::RngAlgorithm::ChaCha,
                    &[seed as u8; 32],
                ),
            );
            let weights = arb_weights().new_tree(&mut runner).unwrap().current();
            let op_strategy = arb_op(weights);
            let mut fresh = |runner: &mut TestRunner| op_strategy.new_tree(runner).unwrap().current();
            let mut best: Vec<Op> = (0..60).map(|_| fresh(&mut runner)).collect();
            let fitness = |ops: &[Op]| {
                let (t, w, _) = meter_program(ops);
                t as f64 / w as f64
            };
            let mut best_ratio = fitness(&best);
            for _ in 0..iterations {
                let mut candidate = best.clone();
                let r = runner.rng().next_u32() as usize;
                let len = candidate.len();
                match r % 5 {
                    0 if len > 0 => candidate[r / 5 % len] = fresh(&mut runner),
                    1 if len < 400 => candidate.insert(r / 5 % (len + 1), fresh(&mut runner)),
                    2 if len > 1 => {
                        candidate.remove(r / 5 % len);
                    }
                    3 if len > 0 && len < 300 => {
                        // Repeat a segment: oscillations are repetitions.
                        let start = r / 5 % len;
                        let end = (start + 1 + (r / 7) % 8).min(len);
                        let segment: Vec<Op> = candidate[start..end].to_vec();
                        let times = 1 + (r / 11) % 8;
                        for _ in 0..times {
                            candidate.splice(end..end, segment.iter().cloned());
                        }
                    }
                    _ => {
                        if len > 0 {
                            candidate[r / 5 % len] = fresh(&mut runner);
                        }
                    }
                }
                let ratio = fitness(&candidate);
                if ratio >= best_ratio {
                    best_ratio = ratio;
                    best = candidate;
                }
            }
            let ratios: Vec<String> = [1u64, 2, 4, 8]
                .iter()
                .map(|&f| {
                    let (t, w, worst) = meter_program(&scaled(&best, f));
                    format!("x{f}: {t}/{w}={:.3} worst-step {worst:.1}", t as f64 / w as f64)
                })
                .collect();
            eprintln!("ADVERSARY seed {seed}: best {best_ratio:.3} len {} | {}", best.len(), ratios.join(" | "));
            if std::env::var("L7_DUMP").is_ok() {
                eprintln!("ADVERSARY program seed {seed}: {best:?}");
            }
        }
    }
