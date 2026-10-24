//! Near-valid encodings: canonical encodings of deep and wide trees, and the
//! same encodings with one constraint violated.
//!
//! The committed generators draw canonical values at most
//! [`ARB_DEPTH`](super::ARB_DEPTH) levels deep, and the committed rejection
//! tests damage such values. These strategies spell trees through the
//! independent encoder ([`codec`](crate::testing::oracles::codec)), which
//! writes non-canonical trees as faithfully as canonical ones, so they reach
//! what neither can:
//!
//! - **Depth.** Version and party trees are bushy, up to
//!   [`BUSHY_DEPTH`](trees::BUSHY_DEPTH) levels, or spines of
//!   [`SPINE_LEVELS`](trees::SPINE_LEVELS) levels that lean left or right
//!   level by level ([`trees`]). Most spines pass [`DEEP_LEVELS`], where a
//!   validator's path state spills past one word.
//! - **Width.** Heights mix small values, where equal siblings are common,
//!   with values at the `2^64` boundary and powers of two hundreds of bits
//!   wide. Ranks reach fractions thousands of digits long, past the rank
//!   decoder's buffer ([`rank`]).
//! - **Damage.** A canonical encoding is then damaged, usually in exactly one
//!   way ([`damage`]): a non-canonical spelling *planted* in the tree (a
//!   sibling pair that should have collapsed, or a running height below
//!   zero), a live bit flipped, inserted, deleted, or the live bits cut, a
//!   sealing defect, or a byte flipped, cut, or appended. Rank encodings get
//!   their format's own defects: an oversized width header, an all-zero final
//!   group, or set padding bits. Composite encodings ([`composites`])
//!   concatenate component encodings, canonical or damaged, and add their own
//!   relational defects: crossed or concurrent span endpoints, a ranked key's
//!   rank belonging to another version, and a structural defect in a span
//!   whose endpoints are also out of order.
//!
//! Each [`NearValid`] input carries what construction settles about it
//! ([`Expect`]) and the regimes it reaches ([`Regime`]). The tests module's
//! census holds each strategy's mass on its regimes above a floor.

mod composites;
mod damage;
mod rank;
mod trees;

mod tests;

use proptest::prelude::*;

pub(crate) use composites::{arb_clock_encoding, arb_ranked_encoding, arb_span_encoding};
pub(crate) use rank::arb_rank_encoding;
pub(crate) use trees::SPINE_LEVELS;

use damage::{arb_damage, BitEdit, Damage};
use trees::{arb_party_plant, arb_party_tree, arb_version_plant, arb_version_tree, Plantable};

/// The depth past which a tree's validation keeps path state in more than
/// one 64-bit word.
///
/// The version and party validators record each open level of the tree in
/// bit stacks, which hold 64 bits in a register and spill whole words to the
/// heap beyond that, so validating a tree deeper than this spills its path
/// state at least once.
pub(crate) const DEEP_LEVELS: u64 = u64::BITS as u64;

/// What construction settles about how a correct decoder treats an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Expect {
    /// The bytes are a canonical encoding, which every decoder accepts.
    Canonical,
    /// The bytes carry a defect every decoder rejects. Construction does not
    /// settle which error variant reports it.
    Defective,
    /// The bytes are a span whose endpoints are out of order and whose upper
    /// endpoint also has a structural defect.
    ///
    /// [`Span::decode`](crate::Span::decode) documents that structural errors
    /// take precedence over endpoint ordering, so it reports `Truncated` or
    /// `TrailingBits`. `in_stream` says the defect lies inside the encoding,
    /// not in bytes appended after it, so a borsh reader, which leaves the
    /// bytes after a value unread, meets it too.
    StructuralOverOrdering {
        /// Whether the defect lies inside the encoding.
        in_stream: bool,
    },
    /// Construction does not settle the verdict: the damage may or may not
    /// leave a canonical encoding.
    Open,
}

/// A region of the input space a strategy claims to reach, which the census
/// floors in the tests module keep alive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Regime {
    /// A canonical encoding of a tree deeper than [`DEEP_LEVELS`].
    DeepCanonical,
    /// A non-canonical spelling planted deeper than [`DEEP_LEVELS`], and no
    /// other damage.
    DeepNonCanonical,
    /// A planted sibling pair that canonical form collapses (equal heights,
    /// or two owned leaves), and no other damage.
    CollapsiblePair,
    /// A planted negative running height, and no other damage.
    NegativeHeight,
    /// Live bits cut strictly inside the tree.
    CutInsideTree,
    /// A canonical rank encoding longer than the rank decoder's buffer.
    LongRank,
    /// A rank whose width header declares a width the format excludes.
    TooWideRankHeader,
    /// A rank whose final fraction group is all zeros.
    ZeroFinalGroup,
    /// A rank with a set bit in its padding or a byte after it.
    DirtyRankPadding,
    /// A clock whose canonical party precedes a version with a planted
    /// non-canonical spelling.
    ClockVersionDefect,
    /// A span whose canonical endpoints are crossed or concurrent.
    UnorderedSpan,
    /// A span whose endpoints are out of order and whose upper endpoint has
    /// a structural defect ([`Expect::StructuralOverOrdering`]).
    StructuralOverOrdering,
    /// A canonical span whose lower endpoint is deeper than
    /// [`DEEP_LEVELS`].
    DeepLowerEndpoint,
    /// A ranked key with a canonical version and the canonical rank of a
    /// version of different rank.
    MismatchedRank,
}

/// One generated input: its bytes, what construction settles about them, and
/// the regimes it reaches.
#[derive(Clone, Debug)]
pub(crate) struct NearValid {
    /// The bytes every decoder under test reads.
    pub(crate) bytes: Vec<u8>,
    /// How a correct decoder treats the bytes, as far as construction
    /// settles it.
    pub(crate) expect: Expect,
    /// The census regimes this input reaches.
    pub(crate) regimes: Vec<Regime>,
    /// For a span whose lower endpoint is exactly a canonical encoding, the
    /// byte offset where its upper endpoint starts.
    ///
    /// Splitting there gives the endpoints of the span's human-readable
    /// record, which decodes through `Span::new` rather than the binary
    /// decoder's single pass.
    pub(crate) split: Option<usize>,
}

impl NearValid {
    /// Arbitrary bytes, about which construction settles nothing.
    fn unstructured(bytes: Vec<u8>) -> Self {
        NearValid {
            bytes,
            expect: Expect::Open,
            regimes: Vec::new(),
            split: None,
        }
    }
}

/// Arbitrary bytes, one input in thirteen, beside a structured strategy.
fn with_unstructured(
    structured: impl Strategy<Value = NearValid>,
) -> impl Strategy<Value = NearValid> {
    prop_oneof![
        12 => structured,
        1 => prop::collection::vec(any::<u8>(), 0..24).prop_map(NearValid::unstructured),
    ]
}

/// A canonical tree's encoding under `damage`, with its regimes.
///
/// A deep tree reaches [`Regime::DeepCanonical`] only undamaged, and a plant
/// reaches [`Regime::DeepNonCanonical`] only when its planted leaves lie
/// deeper than [`DEEP_LEVELS`], where a validator meets them with its path
/// state spilled.
fn damaged<T: Plantable>(tree: &T, damage: &Damage<T::Plant>) -> NearValid {
    let expect = damage.expect();
    let mut regimes = Vec::new();
    let spelled = match &damage.plant {
        Some(plant) => {
            let (planted, depth) = tree.planted(plant);
            if expect == Expect::Defective {
                if depth > DEEP_LEVELS {
                    regimes.push(Regime::DeepNonCanonical);
                }
                regimes.push(T::plant_regime(plant));
            }
            planted
        }
        None => {
            if expect == Expect::Canonical && tree.depth() > DEEP_LEVELS {
                regimes.push(Regime::DeepCanonical);
            }
            tree.clone()
        }
    };
    if matches!(damage.bits, Some(BitEdit::Cut(_))) {
        regimes.push(Regime::CutInsideTree);
    }
    NearValid {
        bytes: damage.spell(spelled.live_bits()),
        expect,
        regimes,
        split: None,
    }
}

/// Near-valid version encodings.
///
/// Regimes: [`Regime::DeepCanonical`], [`Regime::DeepNonCanonical`],
/// [`Regime::CollapsiblePair`], [`Regime::NegativeHeight`], and
/// [`Regime::CutInsideTree`].
pub(crate) fn arb_version_encoding() -> impl Strategy<Value = NearValid> {
    with_unstructured(
        (arb_version_tree(), arb_damage(arb_version_plant))
            .prop_map(|(tree, damage)| damaged(&tree, &damage)),
    )
}

/// Near-valid party encodings.
///
/// Regimes: [`Regime::DeepCanonical`], [`Regime::DeepNonCanonical`],
/// [`Regime::CollapsiblePair`], and [`Regime::CutInsideTree`].
pub(crate) fn arb_party_encoding() -> impl Strategy<Value = NearValid> {
    with_unstructured(
        (arb_party_tree(), arb_damage(arb_party_plant))
            .prop_map(|(tree, damage)| damaged(&tree, &damage)),
    )
}
