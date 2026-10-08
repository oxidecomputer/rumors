//! Scratch probe (never committed): peak heap of the board's
//! `party_forks_full` cell on the `scatter` family across many sizes.

use before::Party;
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// The joined even-indexed half of a balanced `n`-leaf fork expansion.
fn joined_half(n: usize) -> (Party, usize) {
    let mut parties = vec![Party::seed()];
    while parties.len() < n {
        let mut next = Vec::with_capacity(parties.len() * 2);
        for mut p in parties {
            let q = p.fork();
            next.push(p);
            next.push(q);
        }
        parties = next;
    }
    parties.truncate(n);
    let evens: Vec<Vec<u8>> = parties
        .iter()
        .step_by(2)
        .map(Party::encode)
        .collect();
    let arity = n / 2;
    let mut decoded = evens.iter().take(arity).map(|b| Party::decode(&b[..]).unwrap());
    let mut joined = decoded.next().unwrap();
    joined.join_all(decoded).unwrap();
    (joined, arity)
}

/// Print one row per size.
#[test]
fn probe() {
    println!("n\tin_bytes\tout_bytes\tpeak\tpeak_per_io_byte\tlive_after\tshare_cap_sum");
    let mut n = 64;
    while n <= 1 << 16 {
        let (mut party, arity) = joined_half(n);
        let in_bytes = party.as_bytes().len();
        let child_count = arity - 1;
        let mut children: Vec<Party> = Vec::with_capacity(child_count);
        HEAP.reset_peak_usage();
        let baseline = HEAP.current_usage();
        children.extend(party.forks(child_count));
        let peak = HEAP.peak_usage() - baseline;
        let live = HEAP.current_usage() - baseline;
        let out_bytes: usize =
            party.as_bytes().len() + children.iter().map(|c| c.as_bytes().len()).sum::<usize>();
        println!(
            "{n}\t{in_bytes}\t{out_bytes}\t{peak}\t{:.3}\t{live}\t-",
            peak as f64 / (in_bytes + out_bytes) as f64
        );
        drop(children);
        drop(party);
        n *= 2;
    }
}
