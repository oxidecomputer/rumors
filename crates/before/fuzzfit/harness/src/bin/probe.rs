//! Feasibility probe: fuel determinism and per-call overhead, end-to-end on
//! one operation (`Version::tick`).
//!
//! Verifies, before anything is built on top: (1) the guest builds and
//! instantiates; (2) the same input yields byte-identical fuel across two
//! fresh instances in-process (and across process invocations — compare two
//! runs' stdout); (3) a random input round-trips host → guest → host with
//! the guest's result byte-equal to the native mirror's.

use before::Clock;
use fuzzfit_harness::wasm::Guest;

fn main() {
    // A modest organic operand, built through the public API only.
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let mut carol = bob.fork();
    for _ in 0..100 {
        alice.tick();
        carol.tick();
        bob.recv(alice.send());
    }
    let (party, mut version) = bob.into_parts();
    let v_bytes = version.encode();
    let p_bytes = party.encode();

    // Native mirror of the measured op.
    party.tick(&mut version);
    let expected = version.encode();

    let mut run = |label: &str| {
        let mut guest = Guest::new();
        let nop = guest.call("ff_nop", &[]);
        guest.stage_write(&v_bytes);
        let dec_v = guest.call("ff_version_decode", &[0]);
        assert_eq!(dec_v.ret, 0, "version decode failed");
        guest.stage_write(&p_bytes);
        let dec_p = guest.call("ff_party_decode", &[1]);
        assert_eq!(dec_p.ret, 0, "party decode failed");
        let tick = guest.call("ff_version_tick", &[0, 1]);
        assert_eq!(tick.ret, 0, "tick failed");
        let enc = guest.call("ff_version_encode", &[0]);
        assert_eq!(enc.ret, 0, "encode failed");
        let got = guest.stage_read();
        assert_eq!(
            got, expected,
            "guest result diverges from native mirror (differential failure)"
        );
        println!(
            "{label}: nop={} decode_v={} decode_p={} tick={} encode={} (v {} bytes, p {} bytes)",
            nop.fuel,
            dec_v.fuel,
            dec_p.fuel,
            tick.fuel,
            enc.fuel,
            v_bytes.len(),
            p_bytes.len()
        );
        (nop.fuel, dec_v.fuel, dec_p.fuel, tick.fuel, enc.fuel)
    };

    let a = run("run A");
    let b = run("run B");
    assert_eq!(a, b, "fuel is not deterministic across fresh instances");
    println!("determinism: two fresh in-process instances agree exactly");
}
