//! Exploratory differential suite for the version algebra (audit lane L2).
//!
//! Public API only, against an independent leaf-list model with its own
//! canonical encoder. Generators reach deep random topology, heights on the
//! narrow/wide code boundaries, and correlated pairs (perturbations and
//! join/meet decompositions) that force ties and collapse cascades.

mod gen;
mod model;

use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use before::shape::{combine, Rise};
use before::{Clock, Dominance, Endpoint, Party, Placement, Precedence, Span, Version};
use num_bigint::BigUint;
use proptest::prelude::*;

use gen::{Limits, Tape};
use model::VM;

/// Decode a model through the public strict decoder.
fn dec(m: &VM) -> Version {
    let bytes = model::encode_version(m);
    Version::decode(&bytes[..])
        .unwrap_or_else(|e| panic!("model encoding rejected ({e:?}): {m:?} -> {bytes:02x?}"))
}

fn redecode(v: &Version) -> Version {
    Version::decode(v.as_bytes()).expect("a version's bytes decode")
}

fn hash_of<T: Hash>(t: &T) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

fn show(m: &VM) -> String {
    let parts: Vec<String> = m
        .iter()
        .map(|(d, h)| {
            if h.bits() > 20 {
                format!("{d}:2^{}~", h.bits() - 1)
            } else {
                format!("{d}:{h}")
            }
        })
        .collect();
    format!("[{}]", parts.join(" "))
}

fn assert_version(got: &Version, want: &VM, what: &str, ctx: &str) {
    let want_bytes = model::encode_version(want);
    assert!(
        got.as_bytes() == &want_bytes[..],
        "{what} differs from the model\n  ctx: {ctx}\n  want {}\n  got  {:?}",
        show(want),
        got
    );
}

/// Every binary lattice entry point against the model, with the size bound.
fn check_lattice(a: &Version, b: &Version, ma: &VM, mb: &VM, ctx: &str) {
    let mj = model::join(ma, mb);
    let mm = model::meet(ma, mb);
    let cells: [(&str, Version); 6] = [
        ("join", a.join(b)),
        ("a | b", a | b),
        ("a.clone() | b", a.clone() | b),
        ("a | b.clone()", a | b.clone()),
        ("a.clone() | b.clone()", a.clone() | b.clone()),
        ("|=", {
            let mut x = a.clone();
            x |= b;
            x
        }),
    ];
    for (what, v) in &cells {
        assert_version(v, &mj, what, ctx);
    }
    let mut x = a.clone();
    x |= b.clone();
    assert_version(&x, &mj, "|= owned", ctx);
    let cells: [(&str, Version); 6] = [
        ("meet", a.meet(b)),
        ("a & b", a & b),
        ("a.clone() & b", a.clone() & b),
        ("a & b.clone()", a & b.clone()),
        ("a.clone() & b.clone()", a.clone() & b.clone()),
        ("&=", {
            let mut x = a.clone();
            x &= b;
            x
        }),
    ];
    for (what, v) in &cells {
        assert_version(v, &mm, what, ctx);
    }
    let mut x = a.clone();
    x &= b.clone();
    assert_version(&x, &mm, "&= owned", ctx);

    let span = a.span(b);
    assert_version(span.lo(), &mm, "span lo", ctx);
    assert_version(span.hi(), &mj, "span hi", ctx);
    let span2 = a ^ b;
    assert_version(span2.lo(), &mm, "^ lo", ctx);
    assert_version(span2.hi(), &mj, "^ hi", ctx);

    let joined = a.join(b);
    let met = a.meet(b);
    for (what, out) in [("join", &joined), ("meet", &met)] {
        assert!(
            out.as_bytes().len() <= a.as_bytes().len() + b.as_bytes().len(),
            "{what} bytes exceed the operands' bytes: ctx {ctx}"
        );
        assert!(
            out.encoded_bits() <= a.encoded_bits() + b.encoded_bits(),
            "{what} bits exceed the operands' bits: {} > {} + {}; ctx {ctx}",
            out.encoded_bits(),
            a.encoded_bits(),
            b.encoded_bits()
        );
    }
    assert_eq!(
        model::version_bits(ma),
        a.encoded_bits(),
        "model bit length: {ctx}"
    );
}

/// Every comparison spelling against the model relation.
fn check_order(a: &Version, b: &Version, want: Option<Ordering>, ctx: &str) {
    let le = matches!(want, Some(Ordering::Less | Ordering::Equal));
    let ge = matches!(want, Some(Ordering::Greater | Ordering::Equal));
    let lt = want == Some(Ordering::Less);
    let gt = want == Some(Ordering::Greater);
    let eq = want == Some(Ordering::Equal);
    assert_eq!(a.partial_cmp(b), want, "partial_cmp: {ctx}");
    assert_eq!(
        b.partial_cmp(a),
        want.map(Ordering::reverse),
        "partial_cmp rev: {ctx}"
    );
    assert_eq!(a.partial_cmp(&b), want, "partial_cmp(&&): {ctx}");
    assert_eq!((&a).partial_cmp(b), want, "(&a).partial_cmp: {ctx}");
    assert_eq!(
        (a <= b, a < b, a >= b, a > b, a == b, a != b),
        (le, lt, ge, gt, eq, !eq),
        "operators: {ctx}"
    );
    assert_eq!(
        (b <= a, b < a, b >= a, b > a, b == a),
        (ge, gt, le, lt, eq),
        "operators rev: {ctx}"
    );
    assert_eq!(
        (&a <= &b, &a < &b, &a >= &b, &a > &b, &a == &b),
        (le, lt, ge, gt, eq),
        "ref operators: {ctx}"
    );
    assert_eq!(
        (*a <= b, *a < b, *a >= b, *a > b, *a == b),
        (le, lt, ge, gt, eq),
        "owned-vs-ref operators: {ctx}"
    );
    assert_eq!(
        (a <= *b, a < *b, a >= *b, a > *b, a == *b),
        (le, lt, ge, gt, eq),
        "ref-vs-owned operators: {ctx}"
    );
    assert_eq!(a.concurrent(b), want.is_none(), "concurrent: {ctx}");
    assert_eq!(b.concurrent(a), want.is_none(), "concurrent rev: {ctx}");
    if eq {
        assert_eq!(hash_of(a), hash_of(b), "equal versions hash equally: {ctx}");
    }
}

/// Projection, its materialization, and every view comparison cell.
fn check_projection(
    a: &Version,
    b: &Version,
    ma: &VM,
    mb: &VM,
    p: &Party,
    mp: &model::PM,
    q: &Party,
    mq: &model::PM,
    ctx: &str,
) {
    let pa = model::project(ma, mp);
    let pb = model::project(mb, mq);
    let view_a = a / p;
    assert_version(&view_a.to_version(), &pa, "project to_version", ctx);
    assert_version(&Version::from(a.project(p)), &pa, "project From", ctx);
    let view_b = b / q;
    assert_version(&view_b.to_version(), &pb, "project q", ctx);

    // view vs version
    let want = model::relation(&pa, mb);
    let le = matches!(want, Some(Ordering::Less | Ordering::Equal));
    let ge = matches!(want, Some(Ordering::Greater | Ordering::Equal));
    assert_eq!(view_a.partial_cmp(b), want, "view<>version cmp: {ctx}");
    assert_eq!(
        b.partial_cmp(&view_a),
        want.map(Ordering::reverse),
        "version<>view cmp: {ctx}"
    );
    assert_eq!(
        (
            view_a <= *b,
            view_a < *b,
            view_a >= *b,
            view_a > *b,
            view_a == *b
        ),
        (
            le,
            want == Some(Ordering::Less),
            ge,
            want == Some(Ordering::Greater),
            want == Some(Ordering::Equal)
        ),
        "view<>version ops: {ctx}"
    );
    assert_eq!(
        (
            *b >= view_a,
            *b > view_a,
            *b <= view_a,
            *b < view_a,
            *b == view_a
        ),
        (
            le,
            want == Some(Ordering::Less),
            ge,
            want == Some(Ordering::Greater),
            want == Some(Ordering::Equal)
        ),
        "version<>view ops: {ctx}"
    );
    assert_eq!(
        b.concurrent(&view_a),
        want.is_none(),
        "concurrent with a view: {ctx}"
    );

    // view vs view
    let want = model::relation(&pa, &pb);
    let le = matches!(want, Some(Ordering::Less | Ordering::Equal));
    let ge = matches!(want, Some(Ordering::Greater | Ordering::Equal));
    assert_eq!(view_a.partial_cmp(&view_b), want, "view<>view cmp: {ctx}");
    assert_eq!(
        (
            view_a <= view_b,
            view_a < view_b,
            view_a >= view_b,
            view_a > view_b,
            view_a == view_b
        ),
        (
            le,
            want == Some(Ordering::Less),
            ge,
            want == Some(Ordering::Greater),
            want == Some(Ordering::Equal)
        ),
        "view<>view ops: {ctx}"
    );
    // A view against its own materialization is equal.
    let mat = view_a.to_version();
    assert!(
        view_a == mat && mat == view_a,
        "view equals its materialization: {ctx}"
    );
}

fn rise_model(r: &Option<Rise>) -> Option<(bool, BigUint)> {
    r.as_ref().map(|r| match r {
        Rise::Up(c) => (true, c.to_string().parse().unwrap()),
        Rise::Down(c) => (false, c.to_string().parse().unwrap()),
    })
}

/// Shape and combine against the model.
fn check_shape(a: &Version, b: &Version, c: &Version, ma: &VM, mb: &VM, mc: &VM, ctx: &str) {
    let got: Vec<_> = a.shape().map(|p| (rise_model(&p.rise), p.depth)).collect();
    assert_eq!(got, model::plateaus(ma), "shape: {ctx}");
    let got: Vec<_> = combine([a, b, c])
        .map(|cell| {
            (
                cell.depth,
                cell.rises.iter().map(rise_model).collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(got, model::cells(&[ma, mb, mc]), "combine3: {ctx}");
    let got: Vec<_> = combine([b, a])
        .map(|cell| {
            (
                cell.depth,
                cell.rises.iter().map(rise_model).collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(got, model::cells(&[mb, ma]), "combine2: {ctx}");
    let got: Vec<_> = combine::<0>([]).map(|cell| cell.depth).collect();
    assert_eq!(got, vec![0], "combine0: {ctx}");
}

/// The n-ary folds over a list drawn from a pool, with clones (shared
/// storage), re-decodes (separate storage), and repeats.
fn check_folds(t: &mut Tape, pool: &[(Version, VM)], ctx: &str) {
    let n = t.below(41);
    let recv_i = t.below(pool.len());
    let recv = &pool[recv_i].0;
    let mut items: Vec<Version> = Vec::with_capacity(n);
    let mut picks = Vec::with_capacity(n);
    for _ in 0..n {
        let i = if t.chance(80) && !picks.is_empty() {
            picks[picks.len() - 1]
        } else {
            t.below(pool.len())
        };
        picks.push(i);
        items.push(if t.chance(128) {
            pool[i].0.clone()
        } else {
            redecode(&pool[i].0)
        });
    }
    let mut mj = pool[recv_i].1.clone();
    let mut mm = pool[recv_i].1.clone();
    let mut sum: VM = vec![(0, BigUint::ZERO)];
    for &i in &picks {
        mj = model::join(&mj, &pool[i].1);
        mm = model::meet(&mm, &pool[i].1);
        sum = model::join(&sum, &pool[i].1);
    }
    let ctx = format!("{ctx}; fold n={n} picks={picks:?} recv={recv_i}");
    assert_version(&recv.join_all(&items), &mj, "join_all borrowed", &ctx);
    assert_version(&recv.join_all(items.clone()), &mj, "join_all owned", &ctx);
    assert_version(&recv.meet_all(&items), &mm, "meet_all borrowed", &ctx);
    assert_version(&recv.meet_all(items.clone()), &mm, "meet_all owned", &ctx);
    let span = recv.span_all(&items);
    assert_version(span.lo(), &mm, "span_all lo", &ctx);
    assert_version(span.hi(), &mj, "span_all hi", &ctx);
    let span = recv.span_all(items.clone());
    assert_version(span.lo(), &mm, "span_all owned lo", &ctx);
    assert_version(span.hi(), &mj, "span_all owned hi", &ctx);
    assert_version(&items.iter().sum::<Version>(), &sum, "sum borrowed", &ctx);
    assert_version(
        &items.clone().into_iter().sum::<Version>(),
        &sum,
        "sum owned",
        &ctx,
    );
    assert_version(
        &items.iter().collect::<Version>(),
        &sum,
        "collect borrowed",
        &ctx,
    );
    assert_version(
        &items.clone().into_iter().collect::<Version>(),
        &sum,
        "collect owned",
        &ctx,
    );
}

fn model_placement(lo: Option<Ordering>, hi: Option<Ordering>) -> Placement {
    use Ordering::*;
    match (lo, hi) {
        (Some(Less), _) => Placement::Before,
        (Some(Equal), Some(Equal)) => Placement::At(Endpoint::Both),
        (Some(Equal), _) => Placement::At(Endpoint::Start),
        (Some(Greater), Some(Less)) => Placement::Between,
        (Some(Greater), Some(Equal)) => Placement::At(Endpoint::End),
        (Some(Greater), Some(Greater)) => Placement::After,
        (Some(Greater), None) => Placement::Concurrent(Endpoint::End),
        (None, None) => Placement::Concurrent(Endpoint::Both),
        (None, _) => Placement::Concurrent(Endpoint::Start),
    }
}

/// Span placement and its coarsenings against the model relations, over
/// spans with shared, equal-but-unshared, and distinct endpoints.
fn check_spans(versions: &[(Version, VM)], ctx: &str) {
    let mut spans: Vec<(Span<'static>, &VM, &VM, &'static str)> = Vec::new();
    for (i, (v, mv)) in versions.iter().enumerate() {
        spans.push((Span::at(v.clone()), mv, mv, "at"));
        spans.push((
            Span::new(v, &redecode(v)).unwrap().into_owned(),
            mv,
            mv,
            "unshared point",
        ));
        for (w, mw) in &versions[i + 1..] {
            spans.push((v.span(w), mv, mw, "hull"));
        }
    }
    for (span, mx, my, kind) in &spans {
        let lo = model::meet(mx, my);
        let hi = model::join(mx, my);
        assert_version(span.lo(), &lo, "span lo", ctx);
        assert_version(span.hi(), &hi, "span hi", ctx);
        for (p, mp) in versions {
            let rl = model::relation(mp, &lo);
            let rh = model::relation(mp, &hi);
            let want = model_placement(rl, rh);
            assert_eq!(span.place(p), want, "place ({kind}): {ctx}");
            let le_lo = matches!(rl, Some(Ordering::Less | Ordering::Equal));
            let le_hi = matches!(rh, Some(Ordering::Less | Ordering::Equal));
            let ge_lo = matches!(rl, Some(Ordering::Greater | Ordering::Equal));
            let ge_hi = matches!(rh, Some(Ordering::Greater | Ordering::Equal));
            let dominance = if ge_hi {
                Dominance::After
            } else if ge_lo {
                Dominance::Between
            } else {
                Dominance::Before
            };
            assert_eq!(span.dominance(p), dominance, "dominance ({kind}): {ctx}");
            let precedence = if le_lo {
                Precedence::Before
            } else if le_hi {
                Precedence::Between
            } else {
                Precedence::After
            };
            assert_eq!(span.precedence(p), precedence, "precedence ({kind}): {ctx}");
            assert_eq!(span.contains(p), ge_lo && le_hi, "contains ({kind}): {ctx}");
        }
    }
}

fn limits(t: &mut Tape) -> Limits {
    match t.below(5) {
        4 => Limits {
            splits: 900,
            depth: 300,
        },
        0 => Limits {
            splits: 6,
            depth: 4,
        },
        1 => Limits {
            splits: 40,
            depth: 12,
        },
        2 => Limits {
            splits: 120,
            depth: 140,
        },
        _ => Limits {
            splits: 300,
            depth: 70,
        },
    }
}

fn run_case(tape: &[u8]) {
    let mut t = Tape::new(tape);
    let lim = limits(&mut t);
    let lv = gen::levels(&mut t);
    let (ma, mb, family) = gen::pair(&mut t, &lv, lim);
    let a = dec(&ma);
    let b = dec(&mb);
    let ctx = format!("{family} a={} b={}", show(&ma), show(&mb));
    check_lattice(&a, &b, &ma, &mb, &ctx);
    check_lattice(&b, &a, &mb, &ma, &ctx);
    check_lattice(&a, &a.clone(), &ma, &ma, &ctx);
    check_lattice(&a, &redecode(&a), &ma, &ma, &ctx);
    let want = model::relation(&ma, &mb);
    check_order(&a, &b, want, &ctx);
    check_order(&a, &a.clone(), Some(Ordering::Equal), &ctx);
    check_order(&a, &redecode(&a), Some(Ordering::Equal), &ctx);
    let mj = model::join(&ma, &mb);
    let mm = model::meet(&ma, &mb);
    let j = dec(&mj);
    let m = dec(&mm);

    // Endpoints decoded from one span buffer share an allocation at
    // different offsets.
    let wire = Span::decode(&a.span(&b).encode()[..]).expect("a span round-trips");
    let (lo, hi) = (wire.lo().clone(), wire.hi().clone());
    assert_version(&lo, &mm, "decoded span lo", &ctx);
    assert_version(&hi, &mj, "decoded span hi", &ctx);
    check_order(&lo, &hi, model::relation(&mm, &mj), &ctx);
    check_lattice(&lo, &hi, &mm, &mj, &ctx);
    assert_version(
        &lo.join_all([&hi, &lo, &hi]),
        &mj,
        "join_all over span endpoints",
        &ctx,
    );
    assert_version(
        &[lo.clone(), hi.clone(), lo.clone()]
            .into_iter()
            .sum::<Version>(),
        &mj,
        "sum over span endpoints",
        &ctx,
    );
    check_order(&a, &j, model::relation(&ma, &mj), &ctx);
    check_order(&m, &b, model::relation(&mm, &mb), &ctx);
    check_lattice(&j, &m, &mj, &mm, &ctx);

    let (mp, pbytes) = gen::party(&mut t, lim);
    let (mq, qbytes) = gen::party(&mut t, lim);
    let p = Party::decode(&pbytes[..]).unwrap_or_else(|e| panic!("party rejected {e:?}: {mp:?}"));
    let q = Party::decode(&qbytes[..]).unwrap_or_else(|e| panic!("party rejected {e:?}: {mq:?}"));
    check_projection(&a, &b, &ma, &mb, &p, &mp, &q, &mq, &ctx);
    check_projection(&j, &a, &mj, &ma, &q, &mq, &p, &mp, &ctx);

    let mc = gen::perturb(&mut t, &ma, &lv, lim);
    let c = dec(&mc);
    check_shape(&a, &b, &c, &ma, &mb, &mc, &ctx);

    // The clock overlay: plateaus refined by ownership.
    let clock = Clock::from_parts(Party::decode(&pbytes[..]).unwrap(), a.clone());
    let got: Vec<_> = clock
        .shape()
        .map(|(pl, owned)| (rise_model(&pl.rise), pl.depth, owned))
        .collect();
    assert_eq!(got, model::overlay(&ma, &mp), "clock shape: {ctx}");
    let regions: Vec<_> = p.shape().map(|r| (r.depth, r.owned)).collect();
    assert_eq!(regions, model::canon_party(&mp), "party shape: {ctx}");

    // Emptiness and the default.
    let empty_model = ma.len() == 1 && ma[0].1 == BigUint::ZERO;
    assert_eq!(a.is_empty(), empty_model, "is_empty: {ctx}");
    assert_eq!(Version::default(), Version::new());
    assert!(Version::default().is_empty());

    // Chained production results as operands: (a | b) & c, (a & b) | c, and
    // the projection's materialization joined back.
    let jc = &(&a | &b) & &c;
    assert_version(
        &jc,
        &model::meet(&model::join(&ma, &mb), &mc),
        "(a|b)&c",
        &ctx,
    );
    let mc2 = &(&a & &b) | &c;
    assert_version(
        &mc2,
        &model::join(&model::meet(&ma, &mb), &mc),
        "(a&b)|c",
        &ctx,
    );
    check_order(
        &jc,
        &mc2,
        model::relation(
            &model::meet(&model::join(&ma, &mb), &mc),
            &model::join(&model::meet(&ma, &mb), &mc),
        ),
        &ctx,
    );
    let proj = (&a / &p).to_version();
    let back = &proj | &(&a / &q).to_version();
    assert_version(
        &back,
        &model::join(&model::project(&ma, &mp), &model::project(&ma, &mq)),
        "pa|qa",
        &ctx,
    );

    let pool = vec![
        (a.clone(), ma.clone()),
        (b.clone(), mb.clone()),
        (j, mj),
        (m, mm),
        (c, mc),
        (Version::new(), vec![(0, BigUint::ZERO)]),
    ];
    check_folds(&mut t, &pool, &ctx);
    check_spans(&pool, &ctx);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(256),
        max_shrink_iters: std::env::var("PROPTEST_MAX_SHRINK_ITERS").ok().and_then(|s| s.parse().ok()).unwrap_or(20_000),
        .. ProptestConfig::default()
    })]

    /// The version algebra agrees with the independent leaf-list model on
    /// deep, wide, correlated inputs.
    #[test]
    fn algebra_matches_model(tape in proptest::collection::vec(any::<u8>(), 0..6000)) {
        run_case(&tape);
    }
}

/// Parallel copies of the main property for long investigative runs; each
/// draws its own random tapes. Ignored by default.
macro_rules! parallel_copies {
    ($($name:ident),*) => {$(
        proptest! {
            #![proptest_config(ProptestConfig {
                cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(256),
                max_shrink_iters: std::env::var("PROPTEST_MAX_SHRINK_ITERS").ok().and_then(|s| s.parse().ok()).unwrap_or(20_000),
                failure_persistence: None,
                .. ProptestConfig::default()
            })]
            #[test]
            #[ignore = "long investigative run"]
            fn $name(tape in proptest::collection::vec(any::<u8>(), 0..12000)) {
                run_case(&tape);
            }
        }
    )*};
}
parallel_copies!(
    par00, par01, par02, par03, par04, par05, par06, par07, par08, par09, par10, par11, par12,
    par13, par14, par15
);

/// Generator census: what the tape generators actually produce.
#[test]
#[ignore = "investigative census; run explicitly"]
fn census() {
    use rand::{RngCore, SeedableRng};
    use std::collections::BTreeMap;
    let n: u64 = std::env::var("CENSUS_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let mut hist: BTreeMap<String, u64> = BTreeMap::new();
    let mut bump = |k: String| *hist.entry(k).or_default() += 1;
    let mut totals = model::CanonStats::default();
    for seed in 0..n {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let len = (rng.next_u32() % 6000) as usize;
        let mut tape = vec![0u8; len];
        rng.fill_bytes(&mut tape);
        let mut t = Tape::new(&tape);
        let lim = limits(&mut t);
        let lv = gen::levels(&mut t);
        let (ma, mb, family) = gen::pair(&mut t, &lv, lim);
        bump(format!("family {family}"));
        bump(format!("limits depth<={}", lim.depth));
        bump(format!("relation {:?}", model::relation(&ma, &mb)));
        let depth = ma.iter().map(|(d, _)| *d).max().unwrap();
        let bucket = match depth {
            0 => "0",
            1..=4 => "1-4",
            5..=16 => "5-16",
            17..=64 => "17-64",
            _ => "65+",
        };
        bump(format!("a max depth {bucket}"));
        let leaves = match ma.len() {
            1 => "1",
            2..=8 => "2-8",
            9..=64 => "9-64",
            _ => "65+",
        };
        bump(format!("a leaves {leaves}"));
        for (name, f) in [("join", true), ("meet", false)] {
            let cells = model::refine(&[model::vdepths(&ma), model::vdepths(&mb)]);
            let raw: Vec<_> = cells
                .into_iter()
                .map(|(d, w)| {
                    let (x, y) = (&ma[w[0].0].1, &mb[w[1].0].1);
                    (
                        d,
                        if f {
                            x.max(y).clone()
                        } else {
                            x.min(y).clone()
                        },
                    )
                })
                .collect();
            let (_, st) = model::canon_stats(raw);
            for (k, v) in [
                ("direct_narrow", st.direct_narrow),
                ("direct_wide", st.direct_wide),
                ("cascade_narrow", st.cascade_narrow),
                ("cascade_wide", st.cascade_wide),
                ("cascade_narrow_after_wide", st.cascade_narrow_after_wide),
                ("wide_codes", st.wide_codes),
            ] {
                if v > 0 {
                    bump(format!("{name} has {k}"));
                }
            }
            totals.direct_wide += st.direct_wide;
            totals.cascade_wide += st.cascade_wide;
            totals.cascade_narrow_after_wide += st.cascade_narrow_after_wide;
        }
        let (mp, _) = gen::party(&mut t, lim);
        let pa = model::project(&ma, &mp);
        bump(format!("proj relation {:?}", model::relation(&pa, &mb)));
        bump(format!("proj trivial {}", pa == ma));
    }
    for (k, v) in &hist {
        println!("{k:40} {v}");
    }
    println!("totals {totals:?}");
}

/// A depth-`d` left-spine party whose single owned region is the tip
/// (`right_tip = false`) or the tip's right sibling (`right_tip = true`), built
/// directly as canonical bytes: two presence bits per node, `00` for the owned
/// leaf.
fn spine_party(d: usize, right_tip: bool) -> Party {
    let mut bits = model::Bits::default();
    for i in 0..d {
        let last = i + 1 == d;
        let (l, r) = if last && right_tip {
            (false, true)
        } else {
            (true, false)
        };
        bits.0.push(l);
        bits.0.push(r);
    }
    bits.0.push(false);
    bits.0.push(false);
    Party::decode(&bits.finish()[..]).expect("a spine party is canonical")
}

/// The version-algebra surfaces the committed deep-tree stress tests do not
/// drive run at depth 100k without overflowing the test thread's stack: the
/// concurrent-pair hull, the receiverless folds, the shape walks, the masked
/// view comparisons, and projection onto a concurrent region.
#[test]
fn deep_surfaces_are_stack_safe() {
    const DEPTH: usize = 100_000;
    let p = spine_party(DEPTH, false);
    let q = spine_party(DEPTH, true);
    let mut vp = Version::new();
    p.tick(&mut vp);
    let mut vq = Version::new();
    q.tick(&mut vq);
    assert!(vp.concurrent(&vq), "sibling tips tick concurrently");

    let joined = &vp | &vq;
    let met = &vp & &vq;
    assert!(met.is_empty());
    let span = vp.span(&vq);
    assert_eq!(span.hi(), &joined);
    assert_eq!(span.lo(), &met);
    let all = vp.span_all([&vq, &joined, &vp]);
    assert_eq!(all.hi(), &joined);
    assert_eq!(
        [vp.clone(), vq.clone()].into_iter().sum::<Version>(),
        joined
    );
    assert_eq!([&vp, &vq].into_iter().collect::<Version>(), joined);
    assert_eq!(vp.meet_all([&vq, &joined]), met);

    assert_eq!(vp.shape().count(), DEPTH + 1);
    assert_eq!(combine([&vp, &vq, &joined]).count(), DEPTH + 1);
    let clock = Clock::from_parts(spine_party(DEPTH, true), vp.clone());
    assert_eq!(clock.shape().count(), DEPTH + 1);
    assert_eq!(p.shape().count(), DEPTH + 1);

    assert!((&joined / &p) == vp);
    assert!((&joined / &q).partial_cmp(&(&joined / &p)).is_none());
    assert!((&vp / &q).to_version().is_empty());
    assert_eq!((&joined / &q).to_version(), vq);
}

/// Calibration for `deep_surfaces_are_stack_safe`: a minimal recursive walk,
/// one frame per level, over the same depth overflows a 2 MiB thread stack
/// (the process aborts). Run explicitly; a pass would mean the depth is too
/// shallow to catch a recursive rewrite.
#[test]
#[ignore = "calibration: expected to abort with a stack overflow"]
fn deep_calibration_recursion_overflows() {
    fn descend(levels: u64, v: &Version) -> u64 {
        if levels == 0 {
            return v.as_bytes().len() as u64;
        }
        1 + std::hint::black_box(descend(levels - 1, v))
    }
    let mut vp = Version::new();
    spine_party(100_000, false).tick(&mut vp);
    let depth = vp.shape().count() as u64 - 1;
    let handle = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || descend(depth, &vp))
        .unwrap();
    let _ = handle.join();
}

/// The fork-based construction of the same deep concurrent pair: the two
/// halves of a forked deep spine party tick concurrent versions one level
/// deeper, and every surface of `deep_surfaces_are_stack_safe` still holds.
#[test]
fn deep_fork_halves_are_stack_safe() {
    const DEPTH: usize = 100_000;
    let mut keeper = spine_party(DEPTH, false);
    let half = keeper.fork();
    let mut a = Version::new();
    keeper.tick(&mut a);
    let mut b = Version::new();
    half.tick(&mut b);
    assert!(a.concurrent(&b));
    let joined = &a | &b;
    let span = a.span(&b);
    assert_eq!(span.hi(), &joined);
    assert!(span.lo().is_empty());
    assert_eq!([a.clone(), b.clone()].into_iter().sum::<Version>(), joined);
    assert_eq!([&a, &b].into_iter().collect::<Version>(), joined);
    assert!(a.shape().count() > DEPTH);
    assert!(combine([&a, &b]).count() > DEPTH);
    let clock = Clock::from_parts(half, a.clone());
    assert!(clock.shape().count() > DEPTH);
    assert!(keeper.shape().count() > DEPTH);
    assert!((&joined / &keeper) == a);
    assert!((&joined / &keeper).partial_cmp(&b).is_none());
}

/// Rescue census (scratch, uncommitted): what every tape draw of `run_case`
/// reaches, in `run_case`'s own draw order.
#[test]
#[ignore = "rescue census; run explicitly"]
fn rescue_census() {
    use rand::{RngCore, SeedableRng};
    fn bump(h: &mut std::collections::BTreeMap<String, u64>, k: String) {
        *h.entry(k).or_default() += 1;
    }
    fn sample(m: &mut std::collections::BTreeMap<String, Vec<u64>>, k: &str, v: u64) {
        m.entry(k.to_string()).or_default().push(v);
    }
    fn vdepth(m: &VM) -> u64 {
        m.iter().map(|(d, _)| *d).max().unwrap()
    }
    fn widest(m: &VM) -> u64 {
        m.iter().map(|(_, h)| h.bits()).max().unwrap()
    }
    fn has_code(m: &VM, w: u64) -> bool {
        (0..m.len()).any(|i| model::code_bits(m, i) == w)
    }
    /// Collapses whose surviving left code is exactly `w` bits: (direct, cascade).
    fn collapses_at(cells: Vec<(u64, BigUint)>, w: u64) -> (u64, u64) {
        let mut out: VM = Vec::with_capacity(cells.len());
        let mut stack: Vec<(u64, bool)> = Vec::new();
        let (mut direct, mut cascade) = (0u64, 0u64);
        for (d, h) in cells {
            out.push((d, h));
            stack.push((d, true));
            let mut first = true;
            while stack.len() >= 2 && stack[stack.len() - 1].0 == stack[stack.len() - 2].0 {
                let (d, s2) = stack.pop().unwrap();
                let (_, s1) = stack.pop().unwrap();
                let n = out.len();
                if s1 && s2 && out[n - 1].1 == out[n - 2].1 {
                    if model::code_bits(&out, n - 2) == w {
                        if first {
                            direct += 1;
                        } else {
                            cascade += 1;
                        }
                    }
                    out.pop();
                    out[n - 2].0 = d - 1;
                    stack.push((d - 1, true));
                } else {
                    stack.push((d - 1, false));
                }
                first = false;
            }
        }
        (direct, cascade)
    }
    let n: u64 = std::env::var("CENSUS_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    let mut hist: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut nums: std::collections::BTreeMap<String, Vec<u64>> = std::collections::BTreeMap::new();
    for seed in 0..n {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let len = (rng.next_u32() % 6000) as usize;
        let mut tape = vec![0u8; len];
        rng.fill_bytes(&mut tape);
        let mut t = Tape::new(&tape);
        let lim = limits(&mut t);
        let tier = lim.splits;
        let lv = gen::levels(&mut t);
        let (ma, mb, family) = gen::pair(&mut t, &lv, lim);
        let ex_p = t.pos() >= len;
        let (mp, _) = gen::party(&mut t, lim);
        let ex_q = t.pos() >= len;
        let (mq, _) = gen::party(&mut t, lim);
        let ex_c = t.pos() >= len;
        let mc = gen::perturb(&mut t, &ma, &lv, lim);
        let ex_f = t.pos() >= len;
        let arity = t.below(41) as u64;

        sample(&mut nums, "tape len", len as u64);
        sample(&mut nums, "tape pos after pair", t.pos() as u64);
        bump(&mut hist, format!("family {family}"));
        if seed < 2000 {
            bump(&mut hist, format!("first2000 family {family}"));
            bump(&mut hist, format!("first2000 relation {:?}", model::relation(&ma, &mb)));
        }
        bump(&mut hist, format!("relation {:?}", model::relation(&ma, &mb)));
        for (tag, ex) in [("p", ex_p), ("q", ex_q), ("c", ex_c), ("fold", ex_f)] {
            bump(&mut hist, format!("exhausted before {tag:4} all       {ex}"));
            bump(&mut hist, format!("exhausted before {tag:4} tier {tier:>3} {ex}"));
        }
        for (name, m) in [("a", &ma), ("b", &mb), ("c", &mc)] {
            sample(&mut nums, &format!("{name} depth"), vdepth(m));
            sample(&mut nums, &format!("{name} leaves"), m.len() as u64);
            sample(&mut nums, &format!("{name} encoded bits"), model::version_bits(m));
            sample(&mut nums, &format!("{name} widest height bits"), widest(m));
        }
        bump(&mut hist, format!("c equals a {}", mc == ma));
        let both_deep = vdepth(&ma) >= 5 && vdepth(&mb) >= 5;
        bump(&mut hist, format!("a and b both depth>=5 {both_deep}"));
        for (name, p) in [("p", &mp), ("q", &mq)] {
            let cp = model::canon_party(p);
            let pd = cp.iter().map(|(d, _)| *d).max().unwrap();
            let owned = cp.iter().filter(|(_, o)| *o).count() as u64;
            let seed_party = cp == vec![(0, true)];
            sample(&mut nums, &format!("{name} party depth"), pd);
            sample(&mut nums, &format!("{name} party regions"), cp.len() as u64);
            sample(&mut nums, &format!("{name} party owned regions"), owned);
            bump(&mut hist, format!("{name} is the seed party {seed_party}"));
            bump(
                &mut hist,
                format!("{name} nonseed and depth>=5 {}", !seed_party && pd >= 5),
            );
        }
        let bucket = match arity {
            0 => "0",
            1..=8 => "1-8",
            9..=17 => "9-17",
            _ => "18-40",
        };
        bump(&mut hist, format!("fold arity {bucket}"));
        for (name, m) in [("a", &ma), ("b", &mb)] {
            bump(&mut hist, format!("input {name} has a 63-bit code {}", has_code(m, 63)));
            bump(&mut hist, format!("input {name} has a 65-bit code {}", has_code(m, 65)));
        }
        for (op, join) in [("join", true), ("meet", false)] {
            let cells = model::refine(&[model::vdepths(&ma), model::vdepths(&mb)]);
            let raw: Vec<(u64, BigUint)> = cells
                .iter()
                .map(|(d, w)| {
                    let (x, y) = (&ma[w[0].0].1, &mb[w[1].0].1);
                    (*d, if join { x.max(y).clone() } else { x.min(y).clone() })
                })
                .collect();
            let out = model::canon(raw.clone());
            bump(&mut hist, format!("{op} output has a 63-bit code {}", has_code(&out, 63)));
            bump(&mut hist, format!("{op} output has a 65-bit code {}", has_code(&out, 65)));
            let (d63, c63) = collapses_at(raw, 63);
            bump(&mut hist, format!("{op} direct collapse with 63-bit left code {}", d63 > 0));
            bump(&mut hist, format!("{op} cascade collapse with 63-bit left code {}", c63 > 0));
        }
    }
    println!("rescue census over {n} tapes");
    for (k, v) in &hist {
        println!("{k:60} {v}");
    }
    for (k, mut v) in nums {
        v.sort_unstable();
        let at = |p: f64| v[((v.len() as f64 - 1.0) * p).round() as usize];
        println!(
            "{k:30} n={} min={} p50={} p90={} p99={} max={}",
            v.len(),
            v[0],
            at(0.5),
            at(0.9),
            at(0.99),
            v[v.len() - 1]
        );
    }
}
