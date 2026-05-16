use borsh::{BorshDeserialize, BorshSerialize};
use imbl::{OrdMap, OrdSet};

use crate::imbl_borsh::{
    deserialize_ordmap, deserialize_ordset, serialize_ordmap, serialize_ordset,
};
use crate::tree::typed::{
    Hash, Node, Prefix,
    height::{Height, Pred, Root, S, Z},
};

/// The initiator's opening message: a single hash at the empty (root) prefix,
/// namely our root hash.
///
/// Carries the same shape as [`Opening`]: an `uncertain` map at `Root` height,
/// populated with one entry. Distinct from `Opening` only by height -- and from
/// [`Exchange`] by the absence of `providing` / `requested`, which can't be
/// populated until at least one round has passed.
#[derive(Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct Initiate {
    #[borsh(
        serialize_with = "crate::imbl_borsh::serialize_ordmap",
        deserialize_with = "crate::imbl_borsh::deserialize_ordmap"
    )]
    pub uncertain: OrdMap<Prefix<Root>, Hash>,
}

/// The responder's opening message: one hash per child of the responder's root,
/// listed unconditionally because the responder has not yet learned what the
/// initiator holds.
///
/// Distinct from [`Exchange`] by the absence of `providing` and `requested`:
/// the responder has not yet been asked for anything, nor seen any of the
/// initiator's `uncertain` to react to. Encoding the asymmetry in the type
/// system makes the initiator's first call
/// ([`super::exchange::Exchange::open_initiator`]) a separate entry point from
/// the steady-state `exchange`, so the latter can assume every uncertain hash
/// describes a parent the receiver has already acknowledged.
#[derive(Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct Opening {
    #[borsh(
        serialize_with = "crate::imbl_borsh::serialize_ordmap",
        deserialize_with = "crate::imbl_borsh::deserialize_ordmap"
    )]
    pub uncertain: OrdMap<Prefix<UnderRoot>, Hash>,
}

/// The steady-state message: carries all three channels (see the
/// asymmetry-matrix table in the [`super`] module docs).
#[derive(Clone)]
pub struct Exchange<P, T, H>
where
    P: Clone + Ord + AsRef<[u8]>,
    S<H>: Height,
    H: Height,
{
    /// Subtrees the counterparty does not have. Populated from two sources:
    /// nodes they `requested` in the previous round, and nodes we unilaterally
    /// know they lack (because they did not list them in the previous round's
    /// `uncertain`).
    ///
    /// In both cases the subtrees are filtered against the counterparty's
    /// version vector: anything causally `<=` their version has either been
    /// already-seen or already-forgotten on their side, so the receiver's view
    /// must agree with ours by treating the absence as a deletion.
    pub providing: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    /// Prefixes the counterparty listed in the previous round's `uncertain`
    /// that we lack entirely. We ask them to send the subtrees so we can insert
    /// them into our zipper.
    pub requested: OrdSet<Prefix<S<H>>>,
    /// Hashes of our subtrees at this round's frontier, for the counterparty
    /// to compare against their own. Each entry routes to one cell of the
    /// asymmetry matrix on the receiving side.
    pub uncertain: OrdMap<Prefix<H>, Hash>,
}

impl<P, T, H> BorshSerialize for Exchange<P, T, H>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize,
    T: Clone,
    S<H>: Height,
    H: Height,
{
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        serialize_ordmap(&self.providing, writer)?;
        serialize_ordset(&self.requested, writer)?;
        serialize_ordmap(&self.uncertain, writer)?;
        Ok(())
    }
}

// `Node<P, T, S<H>>: BorshDeserialize` reduces inductively to
// `Node<P, T, H>: BorshDeserialize` and bottoms at `Z`, so with `H` left
// generic the proof obligation doesn't terminate during inference. We
// thread `Node<P, T, S<H>>: BorshDeserialize` through as an explicit
// bound so the caller — who knows `H` concretely — discharges it.
impl<P, T, H> BorshDeserialize for Exchange<P, T, H>
where
    P: Clone + Ord + AsRef<[u8]> + BorshDeserialize,
    T: Clone + BorshDeserialize,
    S<H>: Height,
    H: Height,
    Node<P, T, S<H>>: BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let providing = deserialize_ordmap(reader)?;
        let requested = deserialize_ordset(reader)?;
        let uncertain = deserialize_ordmap(reader)?;
        Ok(Self {
            providing,
            requested,
            uncertain,
        })
    }
}

impl<P, T> From<Opening> for Exchange<P, T, UnderRoot>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    fn from(Opening { uncertain }: Opening) -> Self {
        Exchange {
            uncertain,
            ..Default::default()
        }
    }
}

impl<P, T, H> Default for Exchange<P, T, H>
where
    P: Clone + Ord + AsRef<[u8]>,
    S<H>: Height,
    H: Height,
{
    fn default() -> Self {
        Self {
            providing: Default::default(),
            requested: Default::default(),
            uncertain: Default::default(),
        }
    }
}

/// The initiator's closing message: a final `providing`/`requested` pair at
/// `S<Z>`, emitted by [`super::exchange::Exchange::close_initiator`].
///
/// Distinct from [`Exchange`] by the absence of `uncertain`: at leaf height,
/// any two parties either have a leaf at the same path (in which case the leaf
/// hashes match: they are the same all-ones sentinel) or one of them lacks it
/// (in which case the receiver routes the missing prefix to `requested`, never
/// `uncertain`). Encoding the vacuity in the type system lets
/// [`super::exchange::Exchange::complete_responder`] consume `Closing`
/// directly, without a runtime check against an out-of-spec initiator.
#[derive(Clone)]
pub struct Closing<P: Clone + Ord + AsRef<[u8]>, T> {
    pub providing: OrdMap<Prefix<S<Z>>, Node<P, T, S<Z>>>,
    pub requested: OrdSet<Prefix<S<Z>>>,
}

impl<P, T> BorshSerialize for Closing<P, T>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize,
    T: Clone,
{
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        serialize_ordmap(&self.providing, writer)?;
        serialize_ordset(&self.requested, writer)?;
        Ok(())
    }
}

impl<P, T> BorshDeserialize for Closing<P, T>
where
    P: Clone + Ord + AsRef<[u8]> + BorshDeserialize,
    T: Clone + BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let providing = deserialize_ordmap(reader)?;
        let requested = deserialize_ordset(reader)?;
        Ok(Self {
            providing,
            requested,
        })
    }
}

impl<P, T> From<Exchange<P, T, Z>> for Closing<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    fn from(
        Exchange {
            providing,
            requested,
            uncertain: _,
        }: Exchange<P, T, Z>,
    ) -> Self {
        Closing {
            providing,
            requested,
        }
    }
}

impl<P, T> Default for Closing<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    fn default() -> Self {
        Self {
            providing: Default::default(),
            requested: Default::default(),
        }
    }
}

/// The responder's closing message: the final `providing` at leaf height,
/// emitted by [`super::exchange::Exchange::complete_responder`] for the
/// initiator to absorb in [`super::exchange::Exchange::complete_initiator`].
///
/// No `requested` (the initiator never replies after this) and no `uncertain`
/// (vacuous at leaf height, same reasoning as [`Closing`]).
#[derive(Clone)]
pub struct Complete<P: Clone + Ord + AsRef<[u8]>, T> {
    pub providing: OrdMap<Prefix<Z>, Node<P, T, Z>>,
}

impl<P, T> BorshSerialize for Complete<P, T>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize,
    T: Clone,
{
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        serialize_ordmap(&self.providing, writer)
    }
}

impl<P, T> BorshDeserialize for Complete<P, T>
where
    P: Clone + Ord + AsRef<[u8]> + BorshDeserialize,
    T: Clone + BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let providing = deserialize_ordmap(reader)?;
        Ok(Self { providing })
    }
}

impl<P, T> Default for Complete<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    fn default() -> Self {
        Self {
            providing: Default::default(),
        }
    }
}

/// The height just under the root, i.e. 31. The responder's opening message
/// carries hashes at this height -- one for each child of its root.
pub type UnderRoot = <Root as Pred>::Pred;

/// The height two levels under the root, i.e. 30.
pub type UnderUnderRoot = <UnderRoot as Pred>::Pred;
