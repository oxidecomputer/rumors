unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24
uptime
BIN=$(cargo test -p before --all-features --locked --lib --no-run --message-format=json | python3 -c '
import sys, json
for line in sys.stdin:
    try:
        d = json.loads(line)
    except Exception:
        continue
    if d.get("reason") == "compiler-artifact" and d.get("executable") and d["target"]["kind"] == ["lib"]:
        print(d["executable"])
')
echo "BIN=$BIN"
P=version::tests::rank_decode_is_independent_of_the_read_schedule
D=version::tests::rank_decode_retries_interrupts_at_every_read
export PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 PROPTEST_MAX_SHRINK_ITERS=0
echo "== deterministic test, once per mutant"
for m in 0 11 2 20 21 22 30 31 32 40 41 42 10 12 50 51 52 53 54 55 56; do
  out=$(RANK_MUTANT=$m "$BIN" --exact "$D" --test-threads=1 -q 2>&1) && r=pass || r=FAIL
  echo "D m=$m $r | $(printf '%s\n' "$out" | grep -m1 -E 'Test failed|panicked at|Error:' | cut -c1-200)"
done
echo "== repaired property, 256 cases, three fresh seeds each"
for m in 0 11 2 20 21 22 30 31 32 40 41 42 10 12 50 51 52 53 54 55 56; do
  for i in 1 2 3; do
    out=$(RANK_MUTANT=$m "$BIN" --exact "$P" --test-threads=1 -q 2>&1) && r=pass || r=FAIL
    echo "P m=$m run=$i $r | $(printf '%s\n' "$out" | grep -m1 -E 'Test failed|panicked at' | cut -c1-200)"
  done
done
echo "== repaired property, 4096 cases"
for m in 0 56; do
  out=$(PROPTEST_CASES=4096 RANK_MUTANT=$m "$BIN" --exact "$P" --test-threads=1 -q 2>&1) && r=pass || r=FAIL
  echo "P m=$m cases=4096 $r | $(printf '%s\n' "$out" | grep -m1 -E 'Test failed|panicked at' | cut -c1-200)"
done
echo "== full output samples"
for m in 21 55; do
  echo "--- sample m=$m"
  RANK_MUTANT=$m "$BIN" --exact "$P" --test-threads=1 2>&1 | tail -25 | cut -c1-400 || true
done
echo "== m=55 single-case kill rate under the repaired property"
k=0; for i in $(seq 1 500); do PROPTEST_CASES=1 RANK_MUTANT=55 "$BIN" --exact "$P" --test-threads=1 -q >/dev/null 2>&1 || k=$((k+1)); done
echo "m=55 single-case runs=500 killed=$k"
uptime
