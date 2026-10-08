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
T=version::tests::rank_decode_is_independent_of_the_read_schedule
export PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 PROPTEST_MAX_SHRINK_ITERS=0
echo "== 256-case passes, three fresh seeds each"
for m in 0 11 2 20 21 22 30 31 32 40 41 42 10 12 50 51 52 53 54 55 56; do
  for i in 1 2 3; do
    out=$(RANK_MUTANT=$m "$BIN" --exact "$T" --test-threads=1 -q 2>&1) && r=pass || r=FAIL
    msg=$(printf '%s\n' "$out" | grep -m1 -E "Test failed|panicked at|assertion" | cut -c1-260)
    echo "m=$m run=$i $r | $msg"
  done
done
echo "== 4096-case passes"
for m in 0 55 56; do
  out=$(PROPTEST_CASES=4096 RANK_MUTANT=$m "$BIN" --exact "$T" --test-threads=1 -q 2>&1) && r=pass || r=FAIL
  echo "m=$m cases=4096 $r | $(printf '%s\n' "$out" | grep -m1 -E 'Test failed|panicked at' | cut -c1-260)"
done
echo "== per-case kill rate, PROPTEST_CASES=1"
for spec in 21:1500 2:500 52:500; do
  m=${spec%%:*}; n=${spec##*:}; k=0
  for i in $(seq 1 $n); do
    PROPTEST_CASES=1 RANK_MUTANT=$m "$BIN" --exact "$T" --test-threads=1 -q >/dev/null 2>&1 || k=$((k+1))
  done
  echo "m=$m single-case runs=$n killed=$k"
done
uptime
