//! Composite encodings: clocks, spans, and ranked keys, built from component
//! encodings and their own relational defects.

use num_bigint::BigInt;
use proptest::prelude::*;
use proptest::sample::Index;

use super::damage::{arb_composite_edit, arb_damage, dirty_padding, flip_bit, ByteEdit, Damage};
use super::rank::arb_rank_encoding;
use super::trees::{
    arb_party_plant, arb_party_tree, arb_version_plant, arb_version_tree, VersionPlant,
};
use super::{damaged, Expect, NearValid, Regime, DEEP_LEVELS};
use crate::testing::oracles::codec::{self, encode_rank, seal};

/// What construction settles about two concatenated components, the first
/// self-delimiting, before any edit to the whole.
///
/// A defective first component is a complete tree with a planted defect, so
/// decoding rejects it whatever follows. A canonical first component hands
/// the decoder exactly the second component's bytes, which keep their own
/// expectation.
fn concatenated(first: Expect, second: Expect) -> Expect {
    match (first, second) {
        (Expect::Defective, _) => Expect::Defective,
        (Expect::Canonical, second @ (Expect::Canonical | Expect::Defective)) => second,
        _ => Expect::Open,
    }
}

/// Near-valid clock encodings: a party encoding, canonical or damaged, then
/// a version encoding, then possibly a byte edit.
///
/// Regime: [`Regime::ClockVersionDefect`].
pub(crate) fn arb_clock_encoding() -> impl Strategy<Value = NearValid> {
    (
        arb_party_tree(),
        arb_damage(arb_party_plant),
        arb_version_tree(),
        arb_damage(arb_version_plant),
        arb_composite_edit(),
    )
        .prop_map(|(party, party_damage, version, version_damage, edit)| {
            let party = damaged(&party, &party_damage);
            let version = damaged(&version, &version_damage);
            let bytes = [party.bytes, version.bytes].concat();
            let (bytes, expect) = match edit {
                Some(edit) => (edit.apply(bytes), Expect::Open),
                None => (bytes, concatenated(party.expect, version.expect)),
            };
            let version_defect = expect == Expect::Defective && party.expect == Expect::Canonical;
            NearValid {
                bytes,
                expect,
                regimes: version_defect
                    .then_some(Regime::ClockVersionDefect)
                    .into_iter()
                    .collect(),
            }
        })
}

/// How a span's endpoints derive from two canonical versions `a` and `b`.
#[derive(Clone, Copy, Debug)]
enum Endpoints {
    /// `a` below `a ∨ b`: ordered.
    Ordered,
    /// `a` and `a`: equal.
    Equal,
    /// `a ∨ b` above `a ∧ b`: reversed unless `a` equals `b`.
    Reversed,
    /// `a` and `b` as drawn: ordered, reversed, or concurrent.
    Independent,
}

impl Endpoints {
    /// The lower and upper endpoint trees.
    fn of(self, a: codec::Version, b: codec::Version) -> (codec::Version, codec::Version) {
        let join = |x: &BigInt, y: &BigInt| x.max(y).clone();
        let meet = |x: &BigInt, y: &BigInt| x.min(y).clone();
        match self {
            Endpoints::Ordered => {
                let hi = a.zip(&b, join);
                (a, hi)
            }
            Endpoints::Equal => (a.clone(), a),
            Endpoints::Reversed => (a.zip(&b, join), a.zip(&b, meet)),
            Endpoints::Independent => (a, b),
        }
    }
}

/// Endpoint damage one time in six.
fn arb_endpoint_damage() -> impl Strategy<Value = Option<Damage<VersionPlant>>> {
    prop_oneof![5 => Just(None), 1 => arb_damage(arb_version_plant).prop_map(Some)]
}

/// A span's encoding from its endpoint trees, each possibly damaged, and a
/// possible byte edit.
fn span_encoding(
    (lo, hi): (codec::Version, codec::Version),
    lo_damage: Option<Damage<VersionPlant>>,
    hi_damage: Option<Damage<VersionPlant>>,
    edit: Option<ByteEdit>,
) -> NearValid {
    let lo_input = damaged(&lo, &lo_damage.unwrap_or_else(Damage::none));
    let hi_input = damaged(&hi, &hi_damage.unwrap_or_else(Damage::none));
    let lo_canonical = lo_input.expect == Expect::Canonical;
    let bytes = [lo_input.bytes, hi_input.bytes].concat();
    let (bytes, expect) = match edit {
        Some(edit) => (edit.apply(bytes), Expect::Open),
        None => match concatenated(lo_input.expect, hi_input.expect) {
            Expect::Canonical if !lo.le(&hi) => (bytes, Expect::Defective),
            expect => (bytes, expect),
        },
    };
    let both_canonical = lo_canonical && hi_input.expect == Expect::Canonical;
    let mut regimes = Vec::new();
    if expect == Expect::Defective && both_canonical {
        regimes.push(Regime::UnorderedSpan);
    }
    if expect == Expect::Canonical && lo.depth() > DEEP_LEVELS {
        regimes.push(Regime::DeepLowerEndpoint);
    }
    NearValid {
        bytes,
        expect,
        regimes,
    }
}

/// A structural defect in a span's upper endpoint, at or after its padding.
#[derive(Clone, Debug)]
enum TailDefect {
    /// Keeps a strict prefix of the upper endpoint's bytes, possibly none.
    Cut(Index),
    /// Sets one padding bit after the marker, or clears the marker when the
    /// marker ends the last byte.
    DirtyPadding(Index),
    /// Clears the marker bit.
    ClearMarker,
    /// Appends one to three bytes.
    Append(Vec<u8>),
}

/// Each tail defect equally often, at any position.
fn arb_tail_defect() -> impl Strategy<Value = TailDefect> {
    prop_oneof![
        any::<Index>().prop_map(TailDefect::Cut),
        any::<Index>().prop_map(TailDefect::DirtyPadding),
        Just(TailDefect::ClearMarker),
        prop::collection::vec(any::<u8>(), 1..=3).prop_map(TailDefect::Append),
    ]
}

/// A span with out-of-order canonical endpoints and a structural defect at
/// or after the upper endpoint's padding.
///
/// The endpoints are `a` and `b` when `a` is not below `b`, and otherwise
/// `a` raised by one everywhere and then `a` itself, a reversed pair.
fn structural_over_ordering(a: codec::Version, b: codec::Version, defect: TailDefect) -> NearValid {
    let (lo, hi) = if a.le(&b) {
        (a.zip(&a, |x: &BigInt, _: &BigInt| x + 1), a)
    } else {
        (a, b)
    };
    let live = hi.live_bits();
    let marker = live.len();
    let mut hi_bytes = seal(&live);
    let padding = hi_bytes.len() * 8 - marker - 1;
    let in_stream = match defect {
        TailDefect::Cut(at) => {
            hi_bytes.truncate(at.index(hi_bytes.len()));
            true
        }
        TailDefect::DirtyPadding(at) if padding > 0 => {
            dirty_padding(&mut hi_bytes, marker + 1, 0x80 >> at.index(padding));
            true
        }
        TailDefect::DirtyPadding(_) | TailDefect::ClearMarker => {
            flip_bit(&mut hi_bytes, marker);
            true
        }
        TailDefect::Append(extra) => {
            hi_bytes.extend(extra);
            false
        }
    };
    NearValid {
        bytes: [lo.encode(), hi_bytes].concat(),
        expect: Expect::StructuralOverOrdering { in_stream },
        regimes: vec![Regime::StructuralOverOrdering],
    }
}

/// Near-valid span encodings: endpoint pairs of every order, each endpoint
/// possibly damaged, four times in five; otherwise an out-of-order pair with
/// a structural defect in its upper endpoint.
///
/// Regimes: [`Regime::UnorderedSpan`], [`Regime::DeepLowerEndpoint`], and
/// [`Regime::StructuralOverOrdering`].
pub(crate) fn arb_span_encoding() -> impl Strategy<Value = NearValid> {
    let endpoints = prop_oneof![
        Just(Endpoints::Ordered),
        Just(Endpoints::Equal),
        Just(Endpoints::Reversed),
        Just(Endpoints::Independent),
    ];
    prop_oneof![
        4 => (
            arb_version_tree(),
            arb_version_tree(),
            endpoints,
            arb_endpoint_damage(),
            arb_endpoint_damage(),
            arb_composite_edit(),
        )
            .prop_map(|(a, b, endpoints, lo_damage, hi_damage, edit)| {
                span_encoding(endpoints.of(a, b), lo_damage, hi_damage, edit)
            }),
        1 => (arb_version_tree(), arb_version_tree(), arb_tail_defect())
            .prop_map(|(a, b, defect)| structural_over_ordering(a, b, defect)),
    ]
}

/// Where a ranked key's rank comes from.
#[derive(Clone, Debug)]
enum RankSource {
    /// The canonical rank of the key's own version.
    Own,
    /// The canonical rank of another drawn version.
    Other,
    /// A near-valid rank encoding.
    NearValid(NearValid),
}

/// Near-valid ranked-key encodings: a rank, then a version encoding,
/// canonical or damaged, then possibly a byte edit.
///
/// Regime: [`Regime::MismatchedRank`].
pub(crate) fn arb_ranked_encoding() -> impl Strategy<Value = NearValid> {
    let source = prop_oneof![
        3 => Just(RankSource::Own),
        1 => Just(RankSource::Other),
        1 => arb_rank_encoding().prop_map(RankSource::NearValid),
    ];
    (
        arb_version_tree(),
        arb_version_tree(),
        source,
        arb_endpoint_damage(),
        arb_composite_edit(),
    )
        .prop_map(|(version, other, source, damage, edit)| {
            let input = damaged(&version, &damage.unwrap_or_else(Damage::none));
            let own = version.rank();
            let (rank, rank_expect, mismatched) = match source {
                RankSource::Own => (encode_rank(&own.0, own.1), Expect::Canonical, false),
                RankSource::Other => {
                    let theirs = other.rank();
                    (
                        encode_rank(&theirs.0, theirs.1),
                        Expect::Canonical,
                        theirs != own,
                    )
                }
                RankSource::NearValid(rank) => (rank.bytes, Expect::Open, false),
            };
            let bytes = [rank, input.bytes].concat();
            let (bytes, expect) = match edit {
                Some(edit) => (edit.apply(bytes), Expect::Open),
                None => match concatenated(rank_expect, input.expect) {
                    Expect::Canonical if mismatched => (bytes, Expect::Defective),
                    expect => (bytes, expect),
                },
            };
            let mismatch_only = mismatched && input.expect == Expect::Canonical;
            NearValid {
                bytes,
                expect,
                regimes: (expect == Expect::Defective && mismatch_only)
                    .then_some(Regime::MismatchedRank)
                    .into_iter()
                    .collect(),
            }
        })
}
