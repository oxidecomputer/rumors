# reviewer-worst-case-pin-always NOTES
Branch proposal/worst-case-pin-always, worktree /Users/oxide/src/rumors-slot-40, HEAD 54212196 (signature good) on 8b28bbd81. Worktree restored clean.
Done. Verdict: changes required (one blocking: SIGINT during acceptance starts the pin; fix = `trap 'exit 130' INT`, verified in toy and fmt/dry-run on real justfile; diff in repair.diff).
Evidence: toy/justfile (old/new/new-trap/new-s*/gate-sim recipes), sigint.py, sig.py, sigint-gate.py. No box run used.
Non-blocking: overrides dropped on ci-instruments path; -d breaks nested just; commit message overclaims "every fix branch's failing-test commit reds the board" (80d4072be touches only forks/tests.rs); job is `instruments` not `ci-instruments`; pre-existing gate-streams comment omits that the board stream shares root target/.
