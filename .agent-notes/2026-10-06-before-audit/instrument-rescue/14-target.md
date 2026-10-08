<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, generated from one ranked list of every instrument in sections 01 to 09; for Finch's review. -->

# 14. Target-dependence instruments

These instruments make 32-bit behavior and `usize` invariance visible
([`00-baseline.md`](00-baseline.md) section 6). Most run in the wasm32-pins
workspace, where one case costs tens to hundreds of seconds and #57 gives
the workspace a 20-minute limit. Your ruling that neither crate promises an
input size removes most of the value of the decoder-growth rows.

Each row condenses the linked entry in sections 01 to 09, which gives the
full account and marks every figure as verified, reported, or inferred; a
row's figures carry the linked entry's marks. "Rank" orders this category;
"Overall" is the instrument's position in the single ranking in
[`../instrument-rescue.md`](../instrument-rescue.md), section 3, and both
rank by the coverage an instrument adds against the work of folding it in.
`#n` is entry `n` of `QUESTIONS.md`; "slot 23", "slot 26", "slot 33",
"slot 45", "slot 46", and "slot 47" are ruled branches not yet ready. Where two lanes
built overlapping instruments, [`17-overlaps.md`](17-overlaps.md) says which
belong together or which subsumes which.

This category holds 17 of the 159 rows.

| Rank | Overall | Instrument | Adds beyond `main` and the ready branches | Evidence | Fold-in work and runtime | Depends on |
|---:|---:|---|---|---|---|---|
| 1 | 16 | [08.6](08-adequacy.md) `usize` signature census (`usize_census.py`) | Lists every public item whose signature mentions `usize` (30, of which 9 are production API); nothing committed reads signature types, so a new `usize` parameter passes unreported. | Found no offending parameter. | Port 61 lines of Python to a Rust census in `surfacecheck`, reconciled both ways. Negligible runtime. | None. |
| 2 | 51 | [08.2](08-adequacy.md) wasm32 pin injection table and driver | The only demonstration that the 32-bit pins fail on the narrowings their docs name (7 of 11), and where they cannot. | Seven caught; four misses traced to the pins' inputs. | A `just` recipe outside the landing check (about 11 minutes); all fifteen swaps apply at `main`; new pins lack injections. | None. |
| 3 | 55 | [07.5](07-suanpan.md) wasm32 landing case 5 | A limb index between `2^31` and `2^32` at shift 0, which no committed or #54 case reaches. | Caught nothing; the defect it would catch is contrived. | One more case in the existing loop. About 14 s. | #31, #54. |
| 4 | 56 | [09.A11](09-build-and-review-probes.md) Linear-memory reading after each wasm32 pin case | Pages held per case (3.06 to 3.56 GiB of 4); the headroom #29 claims is unchecked. | #63 cites it. | Read `memory.size` after each call; assert a ceiling per check. | None. |
| 5 | 57 | [04.16](04-measures.md) wasm32 memory-size readout | The same reading as 09.A11, built separately in the lane's prototype harness. | Produced the page counts behind observation O6. | Add the page count to the harness result. | #31. |
| 6 | 60 | [09.A24](09-build-and-review-probes.md) Guest panic records under memory exhaustion | What #31's panic record captures in each case (literal panics, `expect`, `Option::unwrap`, overflow, abort). | The evidence #31's documentation cites. | One guest case each, after #58. | #31, #58. |
| 7 | 61 | [06.2](06-codecs.md) wasm32 borsh probe over a wide one-leaf version | The only borsh decode on a 32-bit target, past bit position `2^32` in borsh's stream cursor; the guest builds without `borsh` today. | Demonstrated a growth panic your ruling dissolved; no narrowing injected. | Reviewed rewrite `8208efaeb` exists; restate it as a bit-position check, new variant after #58. About 150 s per case. | #58; keeping `fix/before-wasm32-buffer-growth`. |
| 8 | 62 | [09.A14](09-build-and-review-probes.md) wasm32 differential over accumulator histories at the stability-width boundary | The only wasm32 test over varied histories: 600 histories at eight widths around the boundary; failed 498 of 4,800 at the defect's base. | The reviewer judged (without a run) that its oracle blesses the truncating mutant #50's pin catches. | Two `Check` variants after #58, two guest functions, a harness test. About 70 s. | #58, #50. |
| 9 | 125 | [09.A29](09-build-and-review-probes.md) Compare pin grown past bit `2^32` | A comparison decided past bit `2^32` on 32-bit. | Did not catch its target narrowing. | A guest case, about 1 GiB, 36 s. | #58. |
| 10 | 126 | [09.A30](09-build-and-review-probes.md) Rank alignment cases on each side of a `2^32` gap | Adds the gap `2^32 + 1` to #63's pin. | The committed cases already catch the only mutant. | Cases in the existing check; slow. | #63, #28. |
| 11 | 127 | [04.15](04-measures.md) wasm32 `Sum` footprint and replay cases | Observation O6: a deep-first `Sum` aborts on allocation on wasm32. | The reproduction of record for O6. | An acceptance check for a growth-policy change, not a passing test today. | #28, #31. |
| 12 | 128 | [06.11](06-codecs.md) wasm32 rank group-stream probe | One doubling past the committed `RankDecode` pin from a lazy source. | Demonstrated a dissolved growth panic. | Reviewed rewrite on `8208efaeb`; about 130 s per case. | #58; keeping `fix/before-wasm32-buffer-growth`. |
| 13 | 135 | [09.A34](09-build-and-review-probes.md) 32-bit decoder-growth instruments (`60d534ed`, `d0a6205f`, `8208efae`) | Decoders past the 1 GiB doubling limit on 32-bit, which your ruling made unpromised. | Two findings outlive the ruling (dense heights past `2^32` bits; `Chain::read_to_end`). | Not applicable under the ruling; the streaming synthesizer is reusable. | Your input-size ruling. |
| 14 | 136 | [06.13](06-codecs.md) wasm32 reader probe over a wide one-leaf version | `std`'s fallible `read_to_end` refusing growth on wasm32. | Evidence for #53's `Decode::Io` sentence. | Not recommended as a check. | None. |
| 15 | 137 | [08.13](08-adequacy.md) `num-bigint` formatting probe | The only executable demonstration that `count_display`'s ranking depends on the target. | Became `ddfabe4cb`. | Vendors about 15,000 lines; diagnostic only. | None. |
| 16 | 138 | [01.14](01-identity.md) Unoptimized-build deep-test configuration (MB3) | Stack safety without tail-call elimination. | Caught M25. | You ruled against it (question 68). | None. |
| 17 | 150 | [06.16](06-codecs.md) wasm32 `Vec` growth diagnostic | A fact about `std`. | Established a dissolved defect's mechanism. | Not recommended. | None. |
