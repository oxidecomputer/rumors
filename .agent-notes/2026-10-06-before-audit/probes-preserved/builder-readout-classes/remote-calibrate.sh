unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24
NEW='test(readout_high_part_costs_one_touch_in_every_carry_class)'
nice -n 10 cargo nextest run --locked -p suanpan --all-features --no-run
for kind in touch value; do
  for class in -3:false -2:false -2:true -1:false -1:true 0:true 0:false 1:true 1:false 2:true 2:false; do
    spec="$kind:$class"
    echo "=== SPEC $spec NEW"
    READOUT_MUTANT="$spec" nice -n 10 cargo nextest run --locked -p suanpan --all-features --no-fail-fast -E "$NEW" 2>&1 || echo "=== STATUS $spec NEW $?"
    echo "=== SPEC $spec REST"
    READOUT_MUTANT="$spec" nice -n 10 cargo nextest run --locked -p suanpan --all-features --no-fail-fast -E "not $NEW" 2>&1 || echo "=== STATUS $spec REST $?"
  done
done
echo "=== DONE"
