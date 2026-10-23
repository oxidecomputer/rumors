//! Temporary probe (auditor-o1): live heap a party keeps after it leaves a
//! decoded clock. Prints readings; asserts nothing. Not for commit.

use std::hint::black_box;
use std::io::Read;

use alloc_meter::{measure, reading};
use before::{Clock, Party, Version};

/// A reader implementing only `read`, so `read_to_end` takes std's default
/// doubling growth, as a socket or decompressor would.
struct Plain<'a>(&'a [u8]);

impl Read for Plain<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

/// Pack preorder two-bit party tags, then the marker and zero padding.
fn pack(tags: impl IntoIterator<Item = (bool, bool)>) -> Vec<u8> {
    let mut bits = Vec::new();
    for (l, r) in tags {
        bits.push(l);
        bits.push(r);
    }
    bits.push(true);
    while bits.len() % 8 != 0 {
        bits.push(false);
    }
    bits.chunks(8)
        .map(|c| c.iter().fold(0u8, |acc, &b| (acc << 1) | u8::from(b)))
        .collect()
}

/// `[1 - 2^-d, 1)`.
fn right_cell_bytes(d: usize) -> Vec<u8> {
    pack(
        std::iter::repeat_n((false, true), d)
            .chain([(false, false)])
            .collect::<Vec<_>>(),
    )
}

/// `[1 - 2^-d, 1)`.
fn right_cell(d: usize) -> Party {
    Party::decode(&right_cell_bytes(d)[..]).expect("canonical")
}

/// `[0, 1 - 2^-d)`, the complement of `right_cell(d)`.
fn right_comb(d: usize) -> Party {
    let mut tags = Vec::new();
    for _ in 0..d - 1 {
        tags.push((true, true));
        tags.push((false, false));
    }
    tags.push((true, false));
    tags.push((false, false));
    Party::decode(&pack(tags)[..]).expect("canonical")
}

/// A version whose events sit only in the deep right cell: depth `d`.
fn deep_version(d: usize) -> Version {
    let mut v = Version::new();
    right_cell(d).ticks(&mut v, 2u8);
    v
}

/// Run `route` inside one reading and report the live heap it leaves (the
/// value it returns is the only survivor) beside the value's own size.
fn held<T>(route: impl FnOnce() -> T, own: impl FnOnce(&T) -> usize) -> (isize, usize) {
    let (_, out) = measure(|| {
        let value = black_box(route());
        let live = reading().net_bytes;
        let own = own(&value);
        drop(value);
        (live, own)
    });
    out
}

fn party_len(p: &Party) -> usize {
    p.as_bytes().len()
}

fn report(route: &str, d: usize, clock_len: usize, (live, own): (isize, usize)) {
    println!(
        "O1 | {route:<44} | d={d:>6} | clock={clock_len:>6} B | own={own:>6} B | live={live:>6} B | excess={:>6} B",
        live - own as isize
    );
}

fn decode(bytes: &[u8]) -> Clock {
    Clock::decode(bytes).expect("own encoding")
}

fn party_of(clock: Clock) -> Party {
    let (p, v) = clock.into_parts();
    drop(v);
    p
}

/// Print the live heap every route from a decoded clock to a party leaves.
#[test]
fn o1_probe_routes() {
    use borsh::BorshDeserialize;

    for d in [100usize, 1_000, 10_000, 100_000] {
        let v = deep_version(d);
        let seed_clock = Clock::from_parts(Party::seed(), v.clone());
        let cb = seed_clock.encode();
        let n = cb.len();
        println!("O1 # d={d}: version {} B, clock {} B", v.as_bytes().len(), n);

        // Entry points that yield a decoded clock, then `into_parts`.
        report("decode(&[u8]) -> into_parts", d, n, held(|| party_of(decode(&cb)), party_len));
        report(
            "decode(plain reader) -> into_parts",
            d,
            n,
            held(|| party_of(Clock::decode(Plain(&cb)).unwrap()), party_len),
        );
        let pc = postcard::to_allocvec(&seed_clock).unwrap();
        report(
            "serde postcard -> into_parts",
            d,
            n,
            held(|| party_of(postcard::from_bytes::<Clock>(&pc).unwrap()), party_len),
        );
        let mut cbor = Vec::new();
        ciborium::into_writer(&seed_clock, &mut cbor).unwrap();
        report(
            "serde ciborium -> into_parts",
            d,
            n,
            held(
                || party_of(ciborium::from_reader::<Clock, _>(&cbor[..]).unwrap()),
                party_len,
            ),
        );
        let js = serde_json::to_string(&seed_clock).unwrap();
        report(
            "serde_json (human-readable) -> into_parts",
            d,
            n,
            held(|| party_of(serde_json::from_str::<Clock>(&js).unwrap()), party_len),
        );
        let bb = borsh::to_vec(&seed_clock).unwrap();
        report(
            "borsh -> into_parts",
            d,
            n,
            held(|| party_of(Clock::try_from_slice(&bb).unwrap()), party_len),
        );

        // Pass-through routes from a slice-decoded clock.
        report(
            "party().dangerously_alias(), clock dropped",
            d,
            n,
            held(
                || {
                    let c = decode(&cb);
                    let p = c.party().dangerously_alias();
                    drop(c);
                    p
                },
                party_len,
            ),
        );
        report(
            "tick() then into_parts",
            d,
            n,
            held(
                || {
                    let mut c = decode(&cb);
                    c.tick();
                    party_of(c)
                },
                party_len,
            ),
        );
        report(
            "tick(): whole clock (own = party + version)",
            d,
            n,
            held(
                || {
                    let mut c = decode(&cb);
                    c.tick();
                    c
                },
                |c| c.party().as_bytes().len() + c.version().as_bytes().len(),
            ),
        );
        report(
            "forks(0) residual",
            d,
            n,
            held(
                || {
                    let mut c = decode(&cb);
                    assert_eq!(c.forks(0u8).count(), 0);
                    party_of(c)
                },
                party_len,
            ),
        );
        report(
            "sync_all(empty)",
            d,
            n,
            held(
                || {
                    let mut c = decode(&cb);
                    c.sync_all([] as [&mut Clock; 0]).unwrap();
                    party_of(c)
                },
                party_len,
            ),
        );
        report(
            "<[Clock; 1]>::from",
            d,
            n,
            held(
                || {
                    let [c]: [Clock; 1] = decode(&cb).into();
                    party_of(c)
                },
                party_len,
            ),
        );

        // Rebuilding routes from a slice-decoded clock.
        report(
            "fork(): kept party",
            d,
            n,
            held(
                || {
                    let mut c = decode(&cb);
                    drop(c.fork());
                    party_of(c)
                },
                party_len,
            ),
        );
        report(
            "fork(): child party",
            d,
            n,
            held(
                || {
                    let mut c = decode(&cb);
                    let child = c.fork();
                    drop(c);
                    party_of(child)
                },
                party_len,
            ),
        );
        report(
            "forks(1): residual",
            d,
            n,
            held(
                || {
                    let mut c = decode(&cb);
                    drop(c.forks(1u8).collect::<Vec<_>>());
                    party_of(c)
                },
                party_len,
            ),
        );
        report(
            "<[Clock; 2]>::from: first",
            d,
            n,
            held(
                || {
                    let [a, b]: [Clock; 2] = decode(&cb).into();
                    drop(b);
                    party_of(a)
                },
                party_len,
            ),
        );

        // Two-clock routes: a deep comb and its complementary cell, each with
        // the deep version.
        let comb_cb = Clock::from_parts(right_comb(d), v.clone()).encode();
        let cell_cb = Clock::from_parts(right_cell(d), v.clone()).encode();
        let n2 = comb_cb.len();
        report(
            "join Ok (comb + cell)",
            d,
            n2,
            held(
                || {
                    let mut a = decode(&comb_cb);
                    a.join(decode(&cell_cb)).unwrap();
                    party_of(a)
                },
                party_len,
            ),
        );
        report(
            "join Err: returned operand (cell, cell)",
            d,
            cell_cb.len(),
            held(
                || {
                    let mut a = decode(&cell_cb);
                    let back = a.join(decode(&cell_cb)).unwrap_err();
                    drop(a);
                    party_of(back)
                },
                party_len,
            ),
        );
        report(
            "sync: receiver party (comb, cell)",
            d,
            n2,
            held(
                || {
                    let mut a = decode(&comb_cb);
                    let mut b = decode(&cell_cb);
                    a.sync(&mut b).unwrap();
                    drop(b);
                    party_of(a)
                },
                party_len,
            ),
        );
        report(
            "sync: other party (comb, cell)",
            d,
            n2,
            held(
                || {
                    let mut a = decode(&comb_cb);
                    let mut b = decode(&cell_cb);
                    a.sync(&mut b).unwrap();
                    drop(a);
                    party_of(b)
                },
                party_len,
            ),
        );
        report(
            "sync_all([other]): receiver party",
            d,
            n2,
            held(
                || {
                    let mut a = decode(&comb_cb);
                    let mut b = decode(&cell_cb);
                    a.sync_all([&mut b]).unwrap();
                    drop(b);
                    party_of(a)
                },
                party_len,
            ),
        );
        let comb = right_comb(d);
        report(
            "into_parts then without(disjoint comb)",
            d,
            cell_cb.len(),
            held(
                || party_of(decode(&cell_cb)).without(&comb).unwrap(),
                party_len,
            ),
        );

        // The dual: a version leaving a clock whose party is deep.
        let mut small = Version::new();
        Party::seed().tick(&mut small);
        let dual_cb = Clock::from_parts(right_comb(d), small).encode();
        report(
            "DUAL version: decode -> into_parts, party dropped",
            d,
            dual_cb.len(),
            held(
                || {
                    let (p, v) = decode(&dual_cb).into_parts();
                    drop(p);
                    v
                },
                |v| v.as_bytes().len(),
            ),
        );
    }

    // Adjacent: standalone decodes, which share no buffer but adopt
    // `read_to_end`'s vector as is.
    for d in [1usize, 10, 30, 100, 1_000, 10_000, 100_000] {
        let pb = right_cell_bytes(d);
        let n = pb.len();
        report(
            "ADJ Party::decode(&[u8])",
            d,
            n,
            held(|| Party::decode(&pb[..]).unwrap(), party_len),
        );
        report(
            "ADJ Party::decode(plain reader)",
            d,
            n,
            held(|| Party::decode(Plain(&pb)).unwrap(), party_len),
        );
        let text = right_cell(d).to_string();
        report(
            "ADJ Party::from_str",
            d,
            n,
            held(|| text.parse::<Party>().unwrap(), party_len),
        );
        let party = right_cell(d);
        let pc = postcard::to_allocvec(&party).unwrap();
        report(
            "ADJ Party serde postcard",
            d,
            n,
            held(|| postcard::from_bytes::<Party>(&pc).unwrap(), party_len),
        );
        let mut cbor = Vec::new();
        ciborium::into_writer(&party, &mut cbor).unwrap();
        report(
            "ADJ Party serde ciborium",
            d,
            n,
            held(|| ciborium::from_reader::<Party, _>(&cbor[..]).unwrap(), party_len),
        );
        if d >= 100 {
            let vb = deep_version(d).encode();
            report(
                "ADJ Version::decode(plain reader)",
                d,
                vb.len(),
                held(|| Version::decode(Plain(&vb)).unwrap(), |v| v.as_bytes().len()),
            );
        }
    }
}
