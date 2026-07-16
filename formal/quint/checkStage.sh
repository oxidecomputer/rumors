#!/usr/bin/env bash
# Open-stage (openStage.qnt) checkpoint runner: the assume-guarantee
# module's expectations, positive and inverted. Companion to check.sh
# (the full-pipeline runner); same conventions.
#
#   ./checkStage.sh          simulator tier + symbolic jam search
#   ./checkStage.sh verify   exhaustive tier: Apalache BMC at each
#                            instance's run-length bound (hours-scale)
#
# A control instance PASSES when the checker FINDS a stage jam. Random
# simulation can miss a jam: the committed-choice linearizations that
# reach one are narrow (stageNoW's was not found in 500 samples), so jam
# cases escalate to a bounded symbolic search — a find is a find at any
# depth; only exhaustive-safety claims need the computed bound.
#
# stageFan256 is the honest exception on both tiers: its run-length bound
# is ~660k steps, so it gets a sampled sweep plus a reduced-depth verify,
# and the pass message records exactly what was covered.
set -u
cd "$(dirname "$0")"
export PATH="/opt/homebrew/opt/openjdk/bin:$PATH"

QUINT="npx quint"
SAMPLES="${SAMPLES:-500}"
JAM_SAMPLES="${JAM_SAMPLES:-20000}"  # jam finds want many short schedules
JAM_SIM_STEPS="${JAM_SIM_STEPS:-100}"
# stageFan256 sampling: both evaluators buffer each sample's whole trace,
# and an F=256 state carries three 512-entry maps — step caps much above
# ~2000 overflow the node heap. Done-reachability rides on the small
# structures the havoc also draws (about 40% of samples finish under this
# cap); big structures still get their prefixes invariant-checked.
SAMPLES_FAN="${SAMPLES_FAN:-60}"
FAN_STEPS="${FAN_STEPS:-2000}"      # sim step cap for stageFan256 (< bound)
JAM_DEPTH="${JAM_DEPTH:-25}"        # symbolic search depth for jam finds

# instance:expectation. safe = no stage jam reachable AND done reachable;
# jam = a stage jam must be reachable.
CASES="
stageAll:safe
stageNoD1:safe
stageNoD3:jam
stageNoW:jam
stageFan256:safe
"

bound() {
  printf 'totalStepsStage\n.exit\n' \
    | $QUINT -r stageInstances.qnt::"$1" --backend=typescript 2>/dev/null \
    | grep -oE '>>> [0-9]+' | grep -oE '[0-9]+' | tail -1
}

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

  if [ "$expect" = "jam" ]; then
    if [ "${1:-run}" = "verify" ]; then
      out="$($QUINT verify --main "$m" --invariant safeStage --max-steps "$JAM_DEPTH" stageInstances.qnt 2>&1)"
      verdict=$(printf '%s' "$out" | grep -cE 'violation|Invariant violated' || true)
    else
      # Stage jams are shallow (committed-choice cycles bite within a
      # scope), so many short schedules beat few bound-length ones; a
      # deeper jam would still be caught by the verify escalation below.
      out="$($QUINT run --main "$m" --invariant safeStage \
        --max-samples "$JAM_SAMPLES" --max-steps "$JAM_SIM_STEPS" stageInstances.qnt 2>&1)"
      verdict=$(printf '%s' "$out" | grep -cE '^\[violation\]|Invariant violated' || true)
      if [ "$verdict" -eq 0 ]; then
        out="$($QUINT verify --main "$m" --invariant safeStage --max-steps "$JAM_DEPTH" stageInstances.qnt 2>&1)"
        verdict=$(printf '%s' "$out" | grep -cE 'violation|Invariant violated' || true)
      fi
    fi
    if [ "$verdict" -gt 0 ]; then
      echo "pass  $m (stage jam found, as required)"
    else
      echo "FAIL  $m: expected a reachable stage jam, checker found none"
      fail=1
    fi
    continue
  fi

  # positive instances
  if [ "$m" = "stageFan256" ]; then
    out="$($QUINT run --main "$m" --invariant stageInvariant --witnesses reachedDone \
      --max-samples "$SAMPLES_FAN" --max-steps "$FAN_STEPS" stageInstances.qnt 2>&1)"
    if printf '%s' "$out" | grep -qE '^\[violation\]|Invariant violated'; then
      echo "FAIL  $m: stage jam reachable in a positive instance"
      printf '%s\n' "$out" | tail -5
      fail=1
    elif printf '%s' "$out" | grep -qE 'reachedDone was witnessed in [1-9]'; then
      echo "pass  $m (SAMPLED ONLY: $SAMPLES_FAN schedules to $FAN_STEPS steps, bound $b not exhausted)"
    else
      echo "FAIL  $m: no sampled schedule reached done"
      fail=1
    fi
    if [ "${1:-run}" = "verify" ]; then
      # Known infeasible (2026-07-15): Apalache exhausts a 32 GB heap
      # after ~30 min on this instance even at depth 30 — the symbolic
      # init havocs three 512-entry maps. The F=256 stage result rests on
      # the sampled tier plus the K=2/F=4 symbolic runs; the parametric
      # fan claim is Phase C's (Lean). Kept as a skip so the gap is loud.
      echo "skip  $m (verify tier infeasible at F=256; see comment)"
    fi
    continue
  fi

  if [ "${1:-run}" = "verify" ]; then
    out="$($QUINT verify --main "$m" --invariant stageInvariant --max-steps "$b" stageInstances.qnt 2>&1)"
    if printf '%s' "$out" | grep -qE 'violation|Invariant violated'; then
      echo "FAIL  $m: stage jam reachable in a positive instance"
      printf '%s\n' "$out" | tail -5
      fail=1
    else
      echo "pass  $m (no jam to exhaustive depth $b)"
    fi
  else
    out="$($QUINT run --main "$m" --invariant stageInvariant --witnesses reachedDone \
      --max-samples "$SAMPLES" --max-steps "$b" stageInstances.qnt 2>&1)"
    if printf '%s' "$out" | grep -qE '^\[violation\]|Invariant violated'; then
      echo "FAIL  $m: stage jam reachable in a positive instance"
      printf '%s\n' "$out" | tail -5
      fail=1
    elif printf '%s' "$out" | grep -qE 'reachedDone was witnessed in [1-9]'; then
      echo "pass  $m (no jam in $SAMPLES schedules; done witnessed; bound $b)"
    else
      echo "FAIL  $m: no sampled schedule reached done"
      fail=1
    fi
  fi
done

exit "$fail"
