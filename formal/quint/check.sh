#!/usr/bin/env bash
# Phase A/B checkpoint runner: every expectation, positive and inverted.
#
# A control instance PASSES when the checker FINDS a stuck state; a
# positive instance passes when none is reachable and Terminal is. Tiers:
#
#   ./check.sh          simulator tier: quint run, SAMPLES schedules per
#                       instance (seconds; run on every model edit)
#   ./check.sh verify   exhaustive tier: Apalache BMC at each instance's
#                       run-length bound (minutes to hours; run before
#                       accepting a model change as validated)
#   ./check.sh induction [instance ...]
#                       Phase B tier: check indInv inductively on the
#                       named positive instances (default: smokeChain).
#                       Per instance this runs the consecution obligation
#                       once per process family (indInv and family-step
#                       implies indInv') in parallel on dedicated Apalache
#                       servers, plus the implication obligation
#                       (indInv implies phaseAInvariant). All eight checks
#                       must pass. Tens of minutes per instance; heap via
#                       JVM_ARGS (default -Xmx10G per server).
#
# The BMC depth per instance is the spec's own totalSteps bound: every
# action consumes a finite skeleton-derived budget, so runs are bounded
# and BMC at that depth is exhaustive for reachability (MODEL.md §7).
# The runner computes the bound from the spec via the REPL and refuses a
# hand-picked constant.
set -u
cd "$(dirname "$0")"
export PATH="/opt/homebrew/opt/openjdk/bin:$PATH"

QUINT="npx quint"
SAMPLES="${SAMPLES:-800}"
MODE="${1:-run}"
# Base port for the induction tier's Apalache servers; 8822 is quint's
# default and may hold a stale small-heap server — never reuse it.
IND_PORT_BASE="${IND_PORT_BASE:-8872}"

# instance:expectation. safe = no stuck state reachable AND terminal
# reachable; stuck = a stuck state must be reachable.
CASES="
smokeChain:safe
rMix:safe
comb6:safe
fanDepthPositive:safe
pyramidFull:safe
pyramidC2:safe
pyramidC1:stuck
n1DropW:stuck
n2fan6:stuck
n2fan5:safe
n2unrestricted:stuck
n3Internal:safe
n3Reduced:safe
n4DropD2:safe
ledgerGap:stuck
"

bound() {
  # totalSteps for one instance, computed by the spec itself.
  printf 'totalSteps\n.exit\n' \
    | $QUINT -r instances.qnt::"$1" --backend=typescript 2>/dev/null \
    | grep -oE '>>> [0-9]+' | grep -oE '[0-9]+' | tail -1
}

well_formed() {
  printf 'wellFormed\n.exit\n' \
    | $QUINT -r instances.qnt::"$1" --backend=typescript 2>/dev/null \
    | grep -oE '>>> (true|false)' | grep -oE 'true|false' | tail -1
}

# The consecution obligation is split by process family: step is the
# union of these actions (see streamingMirror.qnt), so passing all of
# them is exactly "indInv and step implies indInv'", in SMT problems
# small enough to iterate on. "stutter" doubles as the implication run's
# trivial step.
IND_FAMILIES="stepIOpenF stepROpenF stepWalkF stepAsmF absorbStep finStep"

if [ "$MODE" = "induction" ]; then
  shift
  instances="${*:-smokeChain}"
  export JVM_ARGS="${JVM_ARGS:--Xmx10G}"
  fail=0
  for m in $instances; do
    if [ "$(well_formed "$m")" != "true" ]; then
      echo "FAIL  $m: skeleton is not well-formed"
      fail=1
      continue
    fi
    logdir="$(mktemp -d)"
    port="$IND_PORT_BASE"
    for fam in $IND_FAMILIES; do
      $QUINT verify --main "$m" --inductive-invariant indInv --step "$fam" \
        --server-endpoint "localhost:$port" instances.qnt \
        > "$logdir/$fam.log" 2>&1 &
      port=$((port + 1))
    done
    $QUINT verify --main "$m" --inductive-invariant indInv --step stutter \
      --invariant phaseAInvariant --server-endpoint "localhost:$port" \
      instances.qnt > "$logdir/implication.log" 2>&1 &
    wait
    for log in "$logdir"/*.log; do
      name="$(basename "$log" .log)"
      if grep -qE '^\[ok\]' "$log"; then
        echo "pass  $m/$name"
      else
        echo "FAIL  $m/$name (log: $log)"
        grep -E '^\[violation\]|error:' "$log" | tail -2
        fail=1
      fi
    done
  done
  exit "$fail"
fi

fail=0
for case in $CASES; do
  m="${case%%:*}"
  expect="${case##*:}"
  b="$(bound "$m")"
  if ! [ "$b" -gt 0 ] 2>/dev/null; then
    echo "FAIL  $m: could not compute run-length bound (got: '$b')"
    fail=1
    continue
  fi
  if [ "$(well_formed "$m")" != "true" ]; then
    echo "FAIL  $m: skeleton is not well-formed (init would be vacuous)"
    fail=1
    continue
  fi

  if [ "$MODE" = "verify" ]; then
    inv="phaseAInvariant"; [ "$expect" = "stuck" ] && inv="safe"
    out="$($QUINT verify --main "$m" --invariant "$inv" --max-steps "$b" instances.qnt 2>&1)"
    verdict=$(printf '%s' "$out" | grep -cE 'violation|Invariant violated' || true)
  else
    inv="phaseAInvariant"; [ "$expect" = "stuck" ] && inv="safe"
    out="$($QUINT run --main "$m" --invariant "$inv" --witnesses reachedTerminal \
      --max-samples "$SAMPLES" --max-steps "$b" instances.qnt 2>&1)"
    verdict=$(printf '%s' "$out" | grep -cE '^\[violation\]|Invariant violated' || true)
  fi

  if [ "$expect" = "stuck" ]; then
    if [ "$verdict" -gt 0 ]; then
      echo "pass  $m (stuck state found, as required; bound $b)"
    else
      echo "FAIL  $m: expected a reachable stuck state, checker found none"
      fail=1
    fi
  else
    if [ "$verdict" -gt 0 ]; then
      echo "FAIL  $m: stuck state reachable in a positive instance"
      printf '%s\n' "$out" | tail -5
      fail=1
      continue
    fi
    if [ "$MODE" = "verify" ]; then
      # Terminal reachability, exhaustively: expect a "violation" of
      # not(terminal) — i.e. terminal is reachable within the bound.
      tout="$($QUINT verify --main "$m" --invariant 'not(terminal)' --max-steps "$b" instances.qnt 2>&1)"
      if printf '%s' "$tout" | grep -qE 'violation|Invariant violated'; then
        echo "pass  $m (safe to depth $b; terminal reachable)"
      else
        echo "FAIL  $m: terminal not reachable within bound $b (bound bug or progress bug)"
        fail=1
      fi
    else
      if printf '%s' "$out" | grep -qE 'reachedTerminal was witnessed in [1-9]'; then
        echo "pass  $m (no stuck state in $SAMPLES schedules; terminal witnessed; bound $b)"
      else
        echo "FAIL  $m: no sampled schedule reached terminal"
        fail=1
      fi
    fi
  fi
done

exit "$fail"
