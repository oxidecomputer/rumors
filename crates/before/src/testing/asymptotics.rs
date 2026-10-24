//! Liveness pins for the documented asymptotics: the growth behaviors
//! the public rustdoc's `# Complexity` sections claim, held alive
//! by exact value identities.
//!
//! A documented non-linear cost is a claim about a mechanism — here the
//! settle's multiplication-bound worst case — and a cure or rewiring that
//! removes the mechanism must reach the documentation. Each pin here asserts
//! an exact value identity on a committed input family showing the documented
//! behavior still exists, so the change that removes it flips a pin red and
//! the rustdoc moves in the same commit. The pins cover the value-dependent
//! work in rank, distance, lag, and ranked keys; the balanced folds' log
//! factors are fuel claims in the `focused-fuel` harness's fold suite.

use crate::testing::meter::registry::Shape;
use num_bigint::BigUint;

/// The exact-rank fold still embeds a product whose two factors carry
/// independent content.
///
/// The plateau-puncture rank equals the closed form `2·x·y + 1` at
/// scale `2^(66d)` over the family's committed factors, computed here
/// through an independent backend multiplication — the value
/// structure behind the fold's `Ω(M(|v|))` lower bound:
/// the exact answer is a wide × dense integer product whose factors the
/// input funds separately (the arbitrary-factor reduction is the
/// query fold's `arbitrary_factors_embed_their_product_in_exact_rank`
/// proptest; this pin holds the committed measured instance). Width
/// scaling alone does not make the instance hard — a power-of-two
/// plateau's parked factor is an all-ones run the settle's own
/// balanced-digit spelling compacts to two signed digits, and a
/// fixed-stride mass telescopes as a geometric series — so the pin
/// guards the factors' content: the parked plunge `x − 1` must stay
/// `Θ(w)` terms under that same compaction, and the mass's `d` digits
/// must stay isolated by more than a full digit (incompressible)
/// at non-uniform jitter. A representation or content change that
/// weakens the embedding reads red here. The board's fuel cells and the
/// `focused-fuel` claims check the implementation's measured cost separately.
#[cfg(feature = "meter")]
#[test]
fn mul_bound_embedding_is_alive() {
    /// The count of nonzero balanced signed digits the settle's own
    /// compaction (`mul_into`'s recentering, replicated) spells a
    /// magnitude into.
    fn balanced_terms(value: &BigUint) -> usize {
        let bytes = value.to_bytes_le();
        let digits = bytes
            .chunks(4)
            .map(|c| {
                let mut d = [0u8; 4];
                d[..c.len()].copy_from_slice(c);
                u64::from(u32::from_le_bytes(d))
            })
            .collect::<Vec<u64>>();
        let mut terms = 0usize;
        let mut carry = 0u64;
        for digit in digits {
            let t = digit + carry;
            if t > 1 << 31 {
                if (1u64 << 32) != t {
                    terms += 1;
                }
                carry = 1;
            } else {
                if t != 0 {
                    terms += 1;
                }
                carry = 0;
            }
        }
        terms + usize::from(carry == 1)
    }

    let (w, d) = (64usize, 48usize);
    let v = Shape::PlateauPuncture.build2(w, d).version();
    let (x, y) = crate::testing::meter::plateau_puncture_factors(w, d);
    assert_eq!(
        (x.bits(), y.bits()),
        ((32 * w) as u64, (66 * d - 1) as u64),
        "both factors must scale with the family parameters: a degenerate \
         factor would make the embedded product one-sided"
    );
    // The parked factor's content: its 64 digits compact to 65
    // balanced terms — the recentering splits high digits into a
    // negative arm and a carry (measured at this content; a plateau
    // of 2^(32w) would read 2).
    assert_eq!(
        balanced_terms(&(&x - 1u8)),
        65,
        "the parked plunge x − 1 must stay incompressible under the \
         settle's own balanced-digit compaction"
    );
    // The mass's content: exactly d isolated bits, pairwise more than
    // a full base-2^32 digit apart (incompressible), and not an
    // arithmetic progression (no geometric-series closed form).
    let positions: Vec<u64> = (0..y.bits()).filter(|&b| y.bit(b)).collect();
    assert_eq!(positions.len(), d, "the mass spells one bit per turn");
    assert!(
        positions.windows(2).all(|p| p[1] - p[0] > 32),
        "every mass gap must exceed a full digit: the compaction could \
         merge closer terms"
    );
    let strides: std::collections::BTreeSet<u64> =
        positions.windows(2).map(|p| p[1] - p[0]).collect();
    assert!(
        strides.len() > 1,
        "a fixed-stride mass is a geometric series: x·y telescopes to \
         shifts and one short division, and the instance stops witnessing \
         the floor"
    );
    let expected = crate::Rank::from_raw(((&x * &y) << 1usize) + 1u8, (66 * d) as u64);
    assert_eq!(
        v.rank(),
        expected,
        "the plateau-puncture rank must be the plateau times the punctured \
         turn mass: the answer no longer embeds the product, so the \
         documented Ω(M(·)) floor lost its witness"
    );
}

/// The pair entry point's answer-embedded-product liveness.
///
/// The multiplication-bound pair claims (distance, lag) enter the settle
/// through the pair co-sweep, a distinct entry point from rank's single-stream
/// fold. The `Ω(M(a + b))` floor therefore needs an embedding family built
/// through the pair operations themselves rather than inferred from rank.
///
/// Against the empty version, the valuation identities collapse to `distance(v,
/// ∅) = lag(∅, v) = rank(v)` and `lag(v, ∅) = 0`, so the plateau-puncture
/// closed form (the factors' scaling and incompressibility are
/// [`mul_bound_embedding_is_alive`]'s preconditions, asserted there at the same
/// dimensions) must reproduce exactly through `Version::distance` and
/// `Version::lag`. The co-operand is empty, *not equal*: the pair entries' only
/// fast path is canonical equality, so both directed calls and the symmetric
/// one run the pair integrator whole. An answer that stops embedding the
/// product — or a pair-entry point rewiring that stops reaching the shared
/// integrator exactly — loses the pair claims their floor witness here.
#[cfg(feature = "meter")]
#[test]
fn mul_bound_pair_embedding_is_alive() {
    let (w, d) = (64usize, 48usize);
    let v = Shape::PlateauPuncture.build2(w, d).version();
    let empty = crate::Version::new();
    assert!(
        !v.is_empty() && empty.is_empty(),
        "the pair must be unequal: an equal pair would answer through \
         the canonical-equality rung without running the pair co-sweep"
    );
    let (x, y) = crate::testing::meter::plateau_puncture_factors(w, d);
    let expected = crate::Rank::from_raw(((&x * &y) << 1usize) + 1u8, (66 * d) as u64);
    assert_eq!(
        v.distance(&empty),
        expected,
        "distance against the empty version must be the plateau times the \
         punctured turn mass: the pair operation's answer no longer embeds the \
         product, so the pair claims lost their floor witness"
    );
    assert_eq!(
        empty.lag(&v),
        expected,
        "the dominated side's lag must be the whole plateau-puncture rank: \
         the pair operation's answer no longer embeds the product"
    );
    assert_eq!(
        v.lag(&empty),
        crate::Rank::ZERO,
        "the dominating side lags the empty version by nothing: the \
         directed functional's zero side must stay exact"
    );
}

/// The key entry points' answer-embedded-product liveness.
///
/// The multiplication-bound key claims (`Ranked::encode_rank`,
/// `Ranked::encode`, `Ranked::decode`) emit or verify the rank through their
/// own fused entry points — distinct entry points from `Version::rank`'s, which
/// [`mul_bound_embedding_is_alive`] pins — so the embedding family must
/// reproduce through them directly, not by composing the committed `encode_rank
/// == rank().encode()` law (whose sampled generators do not reach this family)
/// with rank's pin.
///
/// On the plateau-puncture instance the key's rank component must decode back
/// to the closed-form product rank, and the composite key must survive its own
/// strict decode — whose verifying rank fold is the `Ranked::decode` claim's
/// multiplication-bound term, here demonstrated firing on the embedding family
/// itself. An encode entry point that stops emitting the product's digits, or a
/// decode entry point that stops verifying them, loses the key claims their
/// floor witness here.
#[cfg(feature = "meter")]
#[test]
fn mul_bound_key_embedding_is_alive() {
    let (w, d) = (64usize, 48usize);
    let v = Shape::PlateauPuncture.build2(w, d).version();
    let (x, y) = crate::testing::meter::plateau_puncture_factors(w, d);
    let expected = crate::Rank::from_raw(((&x * &y) << 1usize) + 1u8, (66 * d) as u64);
    let rank_key = crate::Ranked::from(&v).encode_rank();
    assert_eq!(
        crate::Rank::decode(&rank_key[..]).expect("the fused rank key is canonical"),
        expected,
        "the fused rank-key emission must carry the plateau times the \
         punctured turn mass: the key operation's answer no longer embeds the \
         product, so the key claims lost their floor witness"
    );
    let decoded = crate::Ranked::decode(&crate::Ranked::from(&v).encode()[..])
        .expect("the composite key round-trips through its verifying decode");
    assert_eq!(
        decoded.version(),
        &v,
        "the composite key's strict decode must recover the embedding \
         family's version through its own verifying rank fold"
    );
}
