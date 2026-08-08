# ITC space-consumption results

Reproduction of Figure 1 from the ITC 2008 paper (§6, "Exercising ITCs"), run
against this crate's encoding rather than the paper's Appendix A encoding.

## Files

- `space.csv` — raw measurements (100 runs, paper parameters). Columns:
  `scenario,entities,iteration,mean_bytes,std_bytes,runs`.
- `itc_space_consumption.png` / `.svg` — the two-panel figure.

## Regenerating

```sh
# Data (paper parameters: 100 runs, 100k/25k iterations; long-running):
cargo run --release --example space_consumption \
    > crates/before/results/space_consumption/space.csv

# Plot (needs matplotlib: `pip install matplotlib`):
python3 crates/before/scripts/plot_space_consumption.py \
    crates/before/results/space_consumption/space.csv
```

See the [`space_consumption` example](../../examples/space_consumption.rs) for the
operation model and how it maps to the paper.

## How these compare to the paper

| Scenario           | Population | Final size (this crate) | Paper (Appendix A encoding) |
|--------------------|-----------:|------------------------:|-----------------------------|
| Data, 100k iters   |        128 |                ~3262 B  | "< 2900 B" (chart ~3000–4000) |
| Data               |          4 |                  ~14 B  | ~13–15 B                    |
| Process, 25k iters |        128 |                 ~146 B  | "slightly above 170 B"      |
| Process            |          4 |                   ~8 B  | ~5–7 B                      |

The curve shapes — rapid early growth then stabilization with a faint
logarithmic creep — reproduce the paper's result. On absolute byte counts,
the process/static case runs well below the paper's (event-component growth
dominates there, and this crate's packed event coding is where it wins); the
data/dynamic case lands within the paper's charted band, slightly above its
quoted floor.
