# reviewer-readout-classes: resumption notes
Reviewing audit/suanpan-readout-class-table @ 8fc5f716 in /Users/oxide/src/rumors-slot-25 (HEAD verified, clean, signed G).
State: read.rs carries REVIEW schema (swap.py schema apply). MUST revert: python3 -I swap.py schema revert; git diff empty.
Run 1: run1.sh -> run1.log (none, touch:-2:true, value:2:true, low-first, low-last, carry-once, parity-touch; NEW and REST).
Predictions: table passes low-first, carry-once, parity-touch; catches low-last ((-2,nz) has M=2, top digit 0).
Prose finding (pre-existing, not branch): digits.rs:23-24 says highest_nonzero is zero "when the value is zero"; (0,zero) row contradicts.
- run2 started: widths+row repairs applied to metered.rs (revert: swap.py row revert; swap.py widths revert)
- schema reverted (read.rs diff empty); prose swap applied; run3 = verify repair (revert: prose, row, widths)
- all swaps reverted; git diff empty; status clean. Remaining: report.
