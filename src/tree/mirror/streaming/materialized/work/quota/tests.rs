//! A reply's quota against the room it prices and the chunks it lets park, on a
//! handle type small enough that a covered set costs more than the reaction
//! slot it frees.
//!
//! The in-memory backend's reaction is at least as large as a covered set,
//! which makes the set's net price zero there; the handle `()` makes a reaction
//! smaller than a set, so these tests price the set's excess too. Each test
//! charges the room on a ledger of its own and builds every chunk the quota
//! accepts as the decoder would, at exactly its lengths, so the ledger's count
//! of bytes items needed beyond the room is an oracle independent of how the
//! quota prices what is left.

use proptest::prelude::*;

use super::{Overdraft, ReplyQuota, parked_room};
use crate::tree::mirror::streaming::{
    FAN,
    byte_set::ByteSet,
    erased::{Reaction, Reply},
    holding::Holding,
    ledger::{Ledger, ReplyBound, min},
    materialized::work::charges::{covered_set_price, parked_bytes},
    message::{Carried, Grandchildren, LaterMatches, Listed},
};
use crate::tree::typed::Hash;

/// A handle that holds nothing, so a reaction is smaller than a covered set.
type Small = ();

/// The small handle prices a covered set above the reaction slot it frees, so
/// the tests below exercise a nonzero net set price.
const _: () = assert!(
    covered_set_price::<Small>() > 0,
    "the small handle must make a covered set cost more than a reaction",
);

/// One frame of a generated reply, or a chunk boundary.
#[derive(Clone, Debug)]
enum Step {
    /// A match at `radix`, covering the radixes above it whose offsets from it
    /// this mask's bits pick, the lowest bit an offset of one, with child
    /// counts of `children` together.
    Match {
        radix: Radix,
        mask: u32,
        children: usize,
    },
    /// A query at `radix` listing `entries` children, ending a chunk unless it
    /// is the reply's last frame and `ends` is false.
    Query {
        radix: Radix,
        entries: usize,
        ends: bool,
    },
    /// A supply's run at `radix`, its node holding `children` children.
    Supply { radix: Radix, children: usize },
}

/// Where a generated reaction sits: just above the reply's latest one, so that
/// most generated replies get far, or at any radix.
#[derive(Clone, Copy, Debug)]
enum Radix {
    /// This many radixes above the latest reaction's, or above the lowest radix
    /// before the first.
    Above(u8),
    /// This radix.
    At(u8),
}

impl Radix {
    /// The radix this stands for, after a latest reaction at `floor`.
    fn resolve(self, floor: Option<u8>) -> u8 {
        match self {
            Self::Above(gap) => floor.map_or(gap - 1, |floor| floor.saturating_add(gap)),
            Self::At(radix) => radix,
        }
    }
}

fn radix() -> impl Strategy<Value = Radix> {
    prop_oneof![
        4 => (1..=24u8).prop_map(Radix::Above),
        1 => any::<u8>().prop_map(Radix::At),
    ]
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (radix(), any::<u32>(), 0..=2 * FAN).prop_map(|(radix, mask, children)| Step::Match {
            radix,
            mask,
            children
        }),
        (
            radix(),
            prop_oneof![3 => 0..=8usize, 1 => 0..=FAN + 8],
            prop::bool::weighted(0.8),
        )
            .prop_map(|(radix, entries, ends)| Step::Query {
                radix,
                entries,
                ends
            }),
        (radix(), 1..=FAN).prop_map(|(radix, children)| Step::Supply { radix, children }),
    ]
}

/// Bounds of any reaction slots up to a fan and any listing up to a few fans,
/// with positions up to a fan.
fn bounds() -> impl Strategy<Value = (ReplyBound, usize)> {
    (0..=FAN, 0..=4 * FAN, 0..=FAN)
        .prop_map(|(reactions, listing, positions)| (ReplyBound { reactions, listing }, positions))
}

/// A listing of `entries` children, allocated at exactly its length, as the
/// codec decodes one.
fn listing(entries: usize) -> Vec<Listed> {
    let mut listing = Vec::with_capacity(entries);
    listing.extend((0..entries).map(|radix| Listed {
        radix: radix as u8,
        hash: Hash::default(),
        grandchildren: Grandchildren::ONE,
        children_less_one: 0,
    }));
    listing
}

/// What one reply delivered against its quota, as the chunks it parked hold it.
#[derive(Debug, Default)]
struct Delivered {
    /// The reaction slots the chunks' reactions take: one per reaction, and one
    /// per child their match covers.
    slots: usize,
    /// The listing entries their queries carry.
    listing: usize,
    /// The positions their matches and queries take, and the children their
    /// match covers.
    positions: usize,
}

impl Delivered {
    /// Read off what `chunks` hold.
    fn of(chunks: &[Reply<Small>]) -> Self {
        let mut delivered = Self::default();
        for reaction in chunks.iter().flat_map(|chunk| &chunk.reactions) {
            delivered.slots += 1;
            match reaction {
                Reaction::Match(later) => {
                    let covered = later.set();
                    delivered.slots += covered.len();
                    delivered.positions += 1 + covered.len();
                }
                Reaction::Query(listing) => {
                    delivered.positions += 1;
                    delivered.listing += listing.len();
                }
                Reaction::Supply(..) => {}
            }
        }
        delivered
    }
}

/// Take everything `ready` holds as one chunk out of `quota`, ending the reply
/// if `last`, onto `chunks`.
///
/// It checks that the chunk's bytes come out of the price: the price falls by
/// at least the chunk's bytes, or, for the last chunk, was at least the chunk's
/// bytes.
fn take(
    quota: &mut ReplyQuota<Small>,
    ready: &mut Vec<Reaction<Small>>,
    chunks: &mut Vec<Reply<Small>>,
    last: bool,
) {
    let before = quota.price();
    // Allocated at exactly its length, as the decoder allocates a chunk.
    let mut reactions = Vec::with_capacity(ready.len());
    reactions.append(ready);
    let mut chunk = Reply {
        reactions,
        charge: Default::default(),
        last,
        carried: Carried::Nothing,
    };
    chunk.charge = quota.chunk(parked_bytes(&chunk), last);
    // The last chunk returns the rest of the room, so what the price would
    // count after it is moot.
    let after = if last { 0 } else { quota.price() };
    assert!(
        after + chunk.charge.bytes() <= before,
        "a chunk takes its bytes out of the price",
    );
    chunks.push(chunk);
}

/// The quota's state, as far as its price and checks can tell.
fn observed(quota: &ReplyQuota<Small>) -> (super::Deliverable, bool) {
    (quota.deliverable(), quota.is_untouched())
}

proptest! {
    /// A quota's room covers every chunk it lets park, the room after
    /// each frame is exactly its price, and that price never grows.
    ///
    /// The reply here is any sequence of matches with any covered sets,
    /// queries with any listing, and supplies with any child counts, mostly
    /// in ascending radix order, against any bound and positions. Every
    /// frame the quota refuses leaves it as it was; every frame it
    /// accepts joins the reply, whose chunks end at a query, as the
    /// decoder's do. Then:
    ///
    /// - no chunk needs more than the room left for it, on the ledger's
    ///   own count;
    /// - the room left after each release is the quota's price, and no
    ///   frame or chunk raises the price;
    /// - what the chunks deliver lies within the bound and positions, its
    ///   reactions in strictly ascending radixes and none at a radix the
    ///   match covers, so the quota accepted nothing its room was not
    ///   priced for;
    /// - once the last chunk is taken, the chunks' charges are all the
    ///   ledger holds.
    #[test]
    fn the_room_covers_every_chunk_and_the_price_never_grows(
        (bound, positions) in bounds(),
        steps in prop::collection::vec(step(), 0..=24),
    ) {
        let ledger = Ledger::default();
        let room = ledger.charge(Holding::ParkedReplies, parked_room::<Small>(bound, positions));
        let mut quota = ReplyQuota::<Small>::new(bound, positions, room);
        prop_assert_eq!(quota.price(), parked_room::<Small>(bound, positions));

        let mut chunks: Vec<Reply<Small>> = Vec::new();
        let mut ready: Vec<Reaction<Small>> = Vec::new();
        let mut floor = None;
        let mut radixes = Vec::new();
        let mut all_covered = ByteSet::EMPTY;
        let charged = |chunks: &[Reply<Small>]| -> usize {
            chunks.iter().map(|chunk| chunk.charge.bytes()).sum()
        };

        for step in steps {
            let before = observed(&quota);
            let price = quota.price();
            let accepted = match step {
                Step::Match { radix, mask, children } => {
                    let radix = radix.resolve(floor);
                    let covered: ByteSet = (0..32u8)
                        .filter(|offset| mask & (1 << offset) != 0)
                        .filter_map(|offset| radix.checked_add(offset + 1))
                        .collect();
                    quota.matched(radix, covered, children).map(|()| {
                        radixes.push(radix);
                        all_covered = covered;
                        floor = Some(radix);
                        ready.push(Reaction::Match(LaterMatches::new(covered)));
                    })
                }
                Step::Query { radix, entries, ends } => {
                    let radix = radix.resolve(floor);
                    quota.query(radix, entries).map(|()| {
                        radixes.push(radix);
                        floor = Some(radix);
                        ready.push(Reaction::Query(listing(entries)));
                        if ends {
                            take(&mut quota, &mut ready, &mut chunks, false);
                        }
                    })
                }
                Step::Supply { radix, children } => {
                    let radix = radix.resolve(floor);
                    quota.supply(radix).map(|()| {
                        radixes.push(radix);
                        floor = Some(radix);
                        quota.supplied(children);
                        ready.push(Reaction::Supply(radix, ()));
                    })
                }
            };
            match accepted {
                Ok(()) => prop_assert!(quota.price() <= price, "a frame never raises the price"),
                Err(_) => prop_assert_eq!(observed(&quota), before, "a refused frame changes nothing"),
            }
            quota.release();
            prop_assert_eq!(
                ledger.held_for(Holding::ParkedReplies) - charged(&chunks),
                quota.price(),
                "after a release the room is the price",
            );
        }
        take(&mut quota, &mut ready, &mut chunks, true);

        prop_assert_eq!(ledger.short(), 0, "no chunk needs more than its room");
        let delivered = Delivered::of(&chunks);
        prop_assert!(delivered.slots <= bound.reactions);
        prop_assert!(delivered.listing <= bound.listing);
        prop_assert!(delivered.positions <= positions);
        prop_assert!(
            radixes.windows(2).all(|pair| pair[0] < pair[1]),
            "reactions ascend by radix: {:?}",
            radixes,
        );
        prop_assert!(
            radixes.iter().all(|&radix| !all_covered.contains(radix)),
            "no reaction sits at a radix the match covers",
        );
        prop_assert_eq!(
            ledger.held_for(Holding::ParkedReplies),
            charged(&chunks),
            "once the reply ends, only its chunks' charges remain",
        );
    }

    /// A fresh quota's price is exactly the room its question sets
    /// aside, for any bound and positions, including bounds whose listing
    /// exceeds a fan for each question the reply can ask, which both hold
    /// to that many fans.
    #[test]
    fn a_fresh_quota_costs_exactly_its_room((bound, positions) in bounds()) {
        let quota = ReplyQuota::<Small>::new(bound, positions, Default::default());
        let room = parked_room::<Small>(bound, positions);
        prop_assert_eq!(quota.price(), room);
        let questions = min(positions, bound.reactions);
        prop_assert_eq!(
            quota.deliverable().listing,
            min(bound.listing, FAN * questions),
            "the listing priced is a fan per question, within the bound",
        );
    }
}

/// A frame past each limit is refused with the limit it would pass, and leaves
/// the quota as it was.
///
/// The bound allows two reaction slots and three listing entries against three
/// positions. A query at radix 5 listing two entries fits; then a query listing
/// two more passes the listing, one at radix 5 again breaks the order, a supply
/// at radix 9 fits, and a match covering radix 12 would take a third and fourth
/// slot.
#[test]
fn each_limit_refuses_the_frame_that_would_pass_it() {
    let bound = ReplyBound {
        reactions: 2,
        listing: 3,
    };
    let mut quota = ReplyQuota::<Small>::new(bound, 3, Default::default());
    assert_eq!(quota.query(5, 2), Ok(()));
    assert_eq!(quota.query(6, 2), Err(Overdraft::Listing { bound: 3 }));
    assert_eq!(
        quota.query(5, 0),
        Err(Overdraft::Unordered { after: 5, radix: 5 })
    );
    assert_eq!(quota.supply(9), Ok(()));
    assert_eq!(
        quota.matched(10, std::iter::once(12).collect(), 2),
        Err(Overdraft::Reactions { bound: 2 })
    );
}

/// A match takes a position for itself and each child it covers, a second match
/// is refused, and a supply at a covered radix is refused.
///
/// Against two positions, a match covering one more child takes both, so a
/// later query finds none.
#[test]
fn a_match_takes_the_positions_it_covers() {
    let bound = ReplyBound {
        reactions: 4,
        listing: 0,
    };
    let mut quota = ReplyQuota::<Small>::new(bound, 2, Default::default());
    assert_eq!(
        quota.matched(1, [2, 3].into_iter().collect(), 0),
        Err(Overdraft::UnpositionedMatch)
    );
    assert_eq!(quota.matched(1, std::iter::once(2).collect(), 0), Ok(()));
    assert_eq!(
        quota.matched(3, ByteSet::EMPTY, 0),
        Err(Overdraft::RepeatedMatch)
    );
    assert_eq!(quota.supply(2), Err(Overdraft::CoveredSupply { radix: 2 }));
    assert_eq!(quota.query(4, 0), Err(Overdraft::UnpositionedQuery));
}
