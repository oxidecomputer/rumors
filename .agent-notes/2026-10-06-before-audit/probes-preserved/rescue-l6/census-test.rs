
/// Instrument-rescue census, not a check: how often the span and clock
/// generators produce a lone first field that exactly fills its buffer, and
/// whether the harness's judge rejects a `Truncated` report on those inputs.
#[test]
fn rescue_l6_census() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::{RngAlgorithm, TestRng};

    fn env(name: &str, default: u64) -> u64 {
        std::env::var(name)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    fn tally(c: &mut BTreeMap<String, u64>, key: String) {
        *c.entry(key).or_default() += 1;
    }

    fn lone_first<T>(p: &TreeParse<T>, bits: &[bool], len: usize) -> &'static str {
        if p.first_violation().is_some() {
            return "first field non-canonical";
        }
        let Some((_, end)) = &p.tree else {
            return "first field incomplete";
        };
        let eb = (end + 1).div_ceil(8);
        if eb > len {
            return "first field short of its padding byte";
        }
        if eb < len {
            return "second field present";
        }
        match padding(bits, *end) {
            None => "lone first field, clean padding",
            Some(_) => "lone first field, dirty padding",
        }
    }

    fn shard(seed: u64, n: u64) -> BTreeMap<String, u64> {
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(&seed.to_le_bytes());
        let mut runner = TestRunner::new_with_rng(
            Config::default(),
            TestRng::from_seed(RngAlgorithm::ChaCha, &key),
        );
        let mut c = BTreeMap::new();
        let span_in = arb_span_input();
        for _ in 0..n {
            let bytes = span_in.new_tree(&mut runner).unwrap().current();
            let bits = unpack(&bytes);
            let p = sv_parse(&bits);
            let kind = lone_first(&p, &bits, bytes.len());
            tally(&mut c, format!("span: {kind}"));
            let verdict = span_verdict(&bytes);
            if kind == "lone first field, dirty padding" {
                let prod = seen(&Span::decode(&bytes[..]));
                let killed = judge(
                    "census",
                    &verdict,
                    &Seen::Err(Class::Truncated),
                    &mut Stats::default(),
                )
                .is_err();
                tally(
                    &mut c,
                    format!("span: lone dirty: production {prod:?}; judge rejects a Truncated report: {killed}"),
                );
            }
            if let Verdict::Reject {
                applicable,
                required: Some(r),
                ..
            } = &verdict
            {
                tally(
                    &mut c,
                    format!(
                        "span: documented precedence narrows the report: {}",
                        applicable.len() > r.len()
                    ),
                );
            }
            if let Some((t, _)) = &p.tree {
                let depth = t.leaves().iter().map(|(d, _)| *d).max().unwrap();
                tally(&mut c, format!("span: first field depth > 32: {}", depth > 32));
            }
        }
        let clock_in = arb_clock_input();
        for _ in 0..n {
            let bytes = clock_in.new_tree(&mut runner).unwrap().current();
            let bits = unpack(&bytes);
            let p = sp_parse(&bits);
            let kind = lone_first(&p, &bits, bytes.len());
            tally(&mut c, format!("clock: {kind}"));
            if kind == "lone first field, dirty padding" {
                let verdict = clock_verdict(&bytes);
                let prod = seen(&Clock::decode(&bytes[..]));
                let killed = judge(
                    "census",
                    &verdict,
                    &Seen::Err(Class::Truncated),
                    &mut Stats::default(),
                )
                .is_err();
                tally(
                    &mut c,
                    format!("clock: lone dirty: production {prod:?}; judge rejects a Truncated report: {killed}"),
                );
            }
        }
        c
    }

    let per_thread = env("RESCUE_N", 2000);
    let threads = env("RESCUE_THREADS", 8);
    let shards: Vec<BTreeMap<String, u64>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|i| s.spawn(move || shard(i + 1, per_thread)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut total: BTreeMap<String, u64> = BTreeMap::new();
    for m in shards {
        for (k, v) in m {
            *total.entry(k).or_default() += v;
        }
    }

    let mut runner = TestRunner::deterministic();
    let rank_in = arb_rank_input();
    for _ in 0..per_thread {
        let bytes = rank_in.new_tree(&mut runner).unwrap().current();
        let accepted = rank_verdict(&bytes).is_accept();
        tally(
            &mut total,
            format!(
                "rank: longer than 64 bytes: {}; spec accepts: {accepted}",
                bytes.len() > 64
            ),
        );
    }
    let enc = arb_sv_nonneg();
    for _ in 0..per_thread {
        let t = enc.new_tree(&mut runner).unwrap().current().normalize();
        let depth = t.leaves().iter().map(|(d, _)| *d).max().unwrap();
        let bucket = match depth {
            0..=4 => "0..=4",
            5..=32 => "5..=32",
            33..=128 => "33..=128",
            _ => "129..",
        };
        tally(&mut total, format!("encoder operand depth after normalize: {bucket}"));
    }

    println!(
        "== rescue census: span and clock {} draws each ({threads} shards of {per_thread}); rank and encoder operands {per_thread} each ==",
        threads * per_thread
    );
    for (k, v) in &total {
        println!("  {v:>9}  {k}");
    }
}
