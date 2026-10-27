//! Allocation checks for the composite codecs.
//!
//! A [`Span`] encodes as its two endpoints' encodings concatenated, and a
//! [`Clock`] as its party's encoding followed by its version's. Each encoder
//! allocates its output once, at the summed length of its parts, so it copies
//! each part's bytes exactly once and never holds a second buffer.
//!
//! Decoding is the other half of the same claim. Every decode entry point
//! stores each part of the value it returns in a buffer of its own, sized to
//! that part's encoding, so a part handed on alone keeps neither the input nor
//! its sibling part alive.

use std::fmt::{Debug, Display};
use std::io::{self, Read};
use std::str::FromStr;

use super::power_of_two;
use alloc_meter::Stats;
use before::error::Decode;
use before::{Clock, Party, Ranked, Span, Version};
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;
#[cfg(feature = "serde")]
use serde::{de::DeserializeOwned, Serialize};

/// One step of a generated history over a pool of clocks forked from one seed.
#[derive(Clone, Debug)]
enum Step {
    /// The selected clock records one event.
    Tick(Index),
    /// The selected clock records `2^k` events at once, so payloads can grow
    /// past the machine word.
    Leap(Index, usize),
    /// The first selected clock receives a send from the second.
    Recv(Index, Index),
}

/// Generates one history step, weighted toward the steps that interleave
/// parties.
fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        2 => any::<Index>().prop_map(Step::Tick),
        1 => (any::<Index>(), 0usize..=96).prop_map(|(i, k)| Step::Leap(i, k)),
        3 => (any::<Index>(), any::<Index>()).prop_map(|(i, j)| Step::Recv(i, j)),
    ]
}

/// What a generated history leaves behind.
struct History {
    /// The pool of clocks, each holding one fork share of the seed party.
    clocks: Vec<Clock>,
    /// The version each step left at the clock it advanced, in step order.
    versions: Vec<Version>,
}

/// Runs `steps` over `clocks` clocks forked from one seed.
fn history(clocks: usize, steps: &[Step]) -> History {
    let mut seed = Clock::seed();
    let mut pool: Vec<Clock> = (1..clocks).map(|_| seed.fork()).collect();
    pool.push(seed);
    let versions = steps
        .iter()
        .map(|step| match step {
            Step::Tick(i) => pool[i.index(clocks)].tick().clone(),
            Step::Leap(i, k) => pool[i.index(clocks)].ticks(power_of_two(*k)).clone(),
            Step::Recv(i, j) => {
                let sent = pool[j.index(clocks)].send().clone();
                pool[i.index(clocks)].recv(&sent).clone()
            }
        })
        .collect();
    History {
        clocks: pool,
        versions,
    }
}

/// The reading of an encoder that allocates its output once, at exactly
/// `len` bytes, and returns it.
fn one_allocation_of(len: usize) -> Stats {
    Stats {
        allocations: 1,
        reallocations: 0,
        over_aligned: 0,
        bytes_allocated: len,
        peak_bytes: len,
        net_bytes: isize::try_from(len).expect("an encoding in memory fits isize"),
    }
}

/// A reader that implements only `read`, so `read_to_end` grows its buffer by
/// doubling, as it would reading from a socket or a decompressor.
struct ReadOnly<'a>(&'a [u8]);

/// Reads from the wrapped slice, and nothing more.
impl Read for ReadOnly<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

/// One decode entry point, holding its prepared input.
struct EntryPoint<T> {
    /// How failure messages name the entry point.
    name: &'static str,
    /// Decodes the prepared input afresh on each call.
    decode: Box<dyn Fn() -> T>,
}

impl<T> EntryPoint<T> {
    /// Names `decode` as the entry point `name`.
    fn new(name: &'static str, decode: impl Fn() -> T + 'static) -> Self {
        Self {
            name,
            decode: Box::new(decode),
        }
    }
}

/// The byte entry points every type offers: `decode` from a slice, and from a
/// reader that implements only `read`, each given `encoding`.
fn byte_entry_points<T: 'static>(
    encoding: Vec<u8>,
    decode: fn(&mut dyn Read) -> Result<T, Decode>,
) -> Vec<EntryPoint<T>> {
    let slice = encoding.clone();
    vec![
        EntryPoint::new("decode from a slice", move || {
            decode(&mut &slice[..]).expect("a value's own encoding decodes")
        }),
        EntryPoint::new("decode from a read-only reader", move || {
            decode(&mut ReadOnly(&encoding)).expect("a value's own encoding decodes")
        }),
    ]
}

/// The entry point that parses `value`'s `Display` text.
fn text_entry_point<T>(value: &T) -> EntryPoint<T>
where
    T: Display + FromStr,
    T::Err: Debug,
{
    let text = value.to_string();
    EntryPoint::new("FromStr", move || {
        text.parse().expect("a value's own text parses")
    })
}

/// The serde entry points, each given `value` serialized: postcard and
/// ciborium read the binary form, and serde_json the human-readable one.
#[cfg(feature = "serde")]
fn serde_entry_points<T: Serialize + DeserializeOwned>(value: &T) -> Vec<EntryPoint<T>> {
    let postcard = postcard::to_allocvec(value).expect("postcard serializes the value");
    let mut cbor = Vec::new();
    ciborium::into_writer(value, &mut cbor).expect("ciborium serializes the value");
    let json = serde_json::to_string(value).expect("serde_json serializes the value");
    vec![
        EntryPoint::new("serde via postcard", move || {
            postcard::from_bytes(&postcard).expect("postcard deserializes its own output")
        }),
        EntryPoint::new("serde via ciborium", move || {
            ciborium::from_reader(&cbor[..]).expect("ciborium deserializes its own output")
        }),
        EntryPoint::new("serde via serde_json", move || {
            serde_json::from_str(&json).expect("serde_json deserializes its own output")
        }),
    ]
}

/// No serde entry points exist without the `serde` feature.
#[cfg(not(feature = "serde"))]
fn serde_entry_points<T>(_: &T) -> Vec<EntryPoint<T>> {
    Vec::new()
}

/// The borsh entry point, given `value` serialized.
#[cfg(feature = "borsh")]
fn borsh_entry_points<T: BorshSerialize + BorshDeserialize>(value: &T) -> Vec<EntryPoint<T>> {
    let bytes = borsh::to_vec(value).expect("borsh serializes the value");
    vec![EntryPoint::new("borsh", move || {
        T::try_from_slice(&bytes).expect("borsh deserializes its own output")
    })]
}

/// No borsh entry point exists without the `borsh` feature.
#[cfg(not(feature = "borsh"))]
fn borsh_entry_points<T>(_: &T) -> Vec<EntryPoint<T>> {
    Vec::new()
}

/// The live heap `make` leaves behind: what the value it returns holds.
fn heap_held<P>(make: impl FnOnce() -> P) -> (usize, P) {
    let (stats, value) = alloc_meter::measure(make);
    let held = usize::try_from(stats.net_bytes)
        .expect("the measured closure frees only what it allocates");
    (held, value)
}

/// Decodes through each entry point, extracts one part with `take`, and
/// records a failure for each part holding more heap than `budget` allows it.
fn check_part<T, P>(
    entry_points: &[EntryPoint<T>],
    part: &str,
    take: impl Fn(T) -> P,
    budget: impl Fn(&P) -> usize,
    failures: &mut Vec<String>,
) {
    for entry in entry_points {
        let (held, value) = heap_held(|| take((entry.decode)()));
        let allowed = budget(&value);
        if held > allowed {
            failures.push(format!(
                "{part} through {}: holds {held} bytes of heap, allowed {allowed}",
                entry.name
            ));
        }
    }
}

/// The heap one endpoint holds when `take` extracts it from [`Span::at`] over
/// the version decoded from `encoding`: the endpoint's buffer, which the point
/// span shares between its two endpoints.
fn point_span_endpoint_heap(encoding: &[u8], take: fn(Span<'static>) -> Version) -> usize {
    let (held, _endpoint) = heap_held(|| {
        let version = Version::decode(encoding).expect("an endpoint's own encoding decodes");
        take(Span::at(version))
    });
    held
}

proptest! {
    /// `Span::encode` allocates its output once, at exactly the summed length
    /// of the two endpoints' encodings.
    ///
    /// The reading is compared whole: one allocation, no reallocation, and
    /// requested, peak, and retained bytes all equal to that length. The spans
    /// are hulls of two snapshots drawn from one generated multi-party history,
    /// comparable or concurrent.
    #[test]
    fn span_encode_allocates_once_at_its_length(
        clocks in 1usize..=4,
        steps in vec(step(), 1..48),
        picks in (any::<Index>(), any::<Index>()),
    ) {
        let versions = history(clocks, &steps).versions;
        let span = versions[picks.0.index(versions.len())]
            .span(&versions[picks.1.index(versions.len())]);
        let len = span.lo().as_bytes().len() + span.hi().as_bytes().len();
        let (stats, bytes) = alloc_meter::measure(|| span.encode());
        prop_assert_eq!(bytes.len(), len);
        prop_assert_eq!(stats, one_allocation_of(len));
    }

    /// `Clock::encode` allocates its output once, at exactly the summed length
    /// of its party's and its version's encodings.
    ///
    /// The reading is compared whole, as for spans. The clocks are the pool a
    /// generated multi-party history leaves behind, so their parties are fork
    /// shares of one seed and their versions carry received events.
    #[test]
    fn clock_encode_allocates_once_at_its_length(
        clocks in 1usize..=8,
        steps in vec(step(), 0..48),
        pick in any::<Index>(),
    ) {
        let pool = history(clocks, &steps).clocks;
        let clock = &pool[pick.index(clocks)];
        let len = clock.party().as_bytes().len() + clock.version().as_bytes().len();
        let (stats, bytes) = alloc_meter::measure(|| clock.encode());
        prop_assert_eq!(bytes.len(), len);
        prop_assert_eq!(stats, one_allocation_of(len));
    }

    /// Every decode entry point gives each part of a decoded value a buffer
    /// of its own, holding no more heap than that part's encoding.
    ///
    /// The entry points are `decode` from a slice and from a reader that
    /// implements only `read`, `FromStr` where the type has a text form, serde
    /// through postcard, ciborium, and serde_json, and borsh. A part is a
    /// party or version taken from a clock by `into_parts`, the party of a
    /// clock that ticked first, an endpoint taken from a span by `into_parts`,
    /// or a whole party, version, or ranked view. A ranked view holds only its
    /// version: the rank prefix is checked and discarded. Each part's heap is
    /// read live after its siblings are dropped, so a part that kept a slice
    /// of a shared buffer, or spare capacity, reads above its encoding.
    ///
    /// The one sharing allowed is a point span's: `Span::at` stores its
    /// endpoint once, as two handles on one buffer, so a decoded point span's
    /// endpoint may hold what the same endpoint of `Span::at` over a decoded
    /// version holds. A point span's endpoint equals the version the same case
    /// checks on its own, so a decode that retains too much fails that check
    /// even where it inflates the allowance.
    ///
    /// The values come from one generated multi-party history: fork shares of
    /// one seed, versions carrying received events, and spans over two of its
    /// versions, comparable or concurrent.
    #[test]
    fn decoded_parts_hold_only_their_encodings(
        clocks in 1usize..=8,
        steps in vec(step(), 0..48),
        picks in (any::<Index>(), any::<Index>()),
    ) {
        let pool = history(clocks, &steps).clocks;
        let clock = &pool[picks.0.index(clocks)];
        let (party, version) = (clock.party(), clock.version());
        let ranked = Ranked::from(version.clone());
        let span = version.span(pool[picks.1.index(clocks)].version());
        let mut failures = Vec::new();

        let entry_points: Vec<_> = byte_entry_points(party.encode(), |r| Party::decode(r))
            .into_iter()
            .chain([text_entry_point(party)])
            .chain(serde_entry_points(party))
            .chain(borsh_entry_points(party))
            .collect();
        check_part(&entry_points, "Party", |p| p, |p| p.as_bytes().len(), &mut failures);

        let entry_points: Vec<_> = byte_entry_points(version.encode(), |r| Version::decode(r))
            .into_iter()
            .chain([text_entry_point(version)])
            .chain(serde_entry_points(version))
            .chain(borsh_entry_points(version))
            .collect();
        check_part(&entry_points, "Version", |v| v, |v| v.as_bytes().len(), &mut failures);

        let entry_points: Vec<_> = byte_entry_points(clock.encode(), |r| Clock::decode(r))
            .into_iter()
            .chain(serde_entry_points(clock))
            .chain(borsh_entry_points(clock))
            .collect();
        check_part(
            &entry_points,
            "Clock party, after into_parts",
            |c| c.into_parts().0,
            |p| p.as_bytes().len(),
            &mut failures,
        );
        check_part(
            &entry_points,
            "Clock version, after into_parts",
            |c| c.into_parts().1,
            |v| v.as_bytes().len(),
            &mut failures,
        );
        check_part(
            &entry_points,
            "Clock party, after tick and into_parts",
            |mut c| {
                c.tick();
                c.into_parts().0
            },
            |p| p.as_bytes().len(),
            &mut failures,
        );

        let entry_points: Vec<_> = byte_entry_points(ranked.encode(), |r| Ranked::decode(r))
            .into_iter()
            .chain(serde_entry_points(&ranked))
            .chain(borsh_entry_points(&ranked))
            .collect();
        check_part(
            &entry_points,
            "Ranked",
            |r| r,
            |r| r.version().as_bytes().len(),
            &mut failures,
        );

        let entry_points: Vec<_> = byte_entry_points(span.encode(), |r| Span::decode(r))
            .into_iter()
            .chain(serde_entry_points(&span))
            .chain(borsh_entry_points(&span))
            .collect();
        let shared_allowance = |take: fn(Span<'static>) -> Version| {
            if span.lo() == span.hi() {
                point_span_endpoint_heap(&span.lo().encode(), take)
            } else {
                0
            }
        };
        let (lo_shared, hi_shared) = (
            shared_allowance(|s| s.into_parts().0),
            shared_allowance(|s| s.into_parts().1),
        );
        check_part(
            &entry_points,
            "Span lo, after into_parts",
            |s| s.into_parts().0,
            |lo| lo.as_bytes().len().max(lo_shared),
            &mut failures,
        );
        check_part(
            &entry_points,
            "Span hi, after into_parts",
            |s| s.into_parts().1,
            |hi| hi.as_bytes().len().max(hi_shared),
            &mut failures,
        );

        prop_assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
