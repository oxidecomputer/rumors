<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), the audit coordinator, from the round-1 review of `audit/wasm32-pins-check-roundtrip`. -->

# wasm32-pins protocol: decoders that cannot disagree with the numbering

## The finding

The wasm32-pins harness sends a `Check` to the guest as a number, and the
guest returns `PASS` (0) or a `Failure` code. Both enums carry explicit
numbers, and each had a hand-written decoder (`Check`'s `TryFrom<u32>`,
`Failure::from_code`). Several unlanded branches add a `Check` variant at the
same number, so later landings renumber; a decoder arm left on a stale number
runs the wrong check, and a pin can pass while testing nothing it claims.

The branch `audit/wasm32-pins-check-roundtrip` (`789936bc`) added two
whole-domain sweeps and two list-driven tests. Round 1's reviewer showed one
silent gap: a `Failure` numbered `PASS`, named in the wildcard-free `match`
but left out of the hand list `FAILURES`, passes all four tests, and the
harness reads a return of `PASS` as `Passed` before decoding, so a guest
returning that failure is reported as a pass. The compiler cannot force a
variant into a list.

The reviewer's repair derives both decoders with `strum_macros::FromRepr`, so
no hand-written arm exists, and rejects a failure numbered `PASS` with a
compile-time assertion on `Failure::from_repr(PASS)`. The repair and its runs
are in the coordinator's session scratchpad under
`reviewer-check-roundtrip/repair/`.

## Owner's ruling

Adopt the dependency: `strum_macros` in the detached wasm32-pins protocol
crate, with the derived decoders and the compile-time `PASS` assertion
replacing the hand-written decoders and their tests.
