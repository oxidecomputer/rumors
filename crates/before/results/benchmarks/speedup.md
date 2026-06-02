# Benchmark speedups: optimized impl vs. reference oracle

Median time at the largest tree size benchmarked, and the oracle/before ratio (>1 ⇒ the impl is faster).

| Type | Operation | n | before (impl) | oracle (ref) | speedup |
|------|-----------|--:|--------------:|-------------:|--------:|
| Party | fork | 32768 | 169.84 µs | 961.61 µs | 5.7× |
| Party | join | 32768 | 568.79 µs | 1.39 ms | 2.4× |
| Party | is_disjoint | 32768 | 136.78 µs | 66.72 µs | 0.5× |
| Party | partial_cmp: ancestor | 32768 | 107.70 µs | 31.67 µs | 0.3× |
| Party | partial_cmp: equal | 32768 | 8.77 µs | 102.07 µs | 11.6× |
| Version | tick | 32768 | 1.84 ms | 1.53 ms | 0.8× |
| Version | merge  ( | , least-upper-bound) | 32768 | 2.12 ms | 2.10 ms | 1.0× |
| Version | partial_cmp: concurrent | 32768 | 265.42 ns | 149.11 ns | 0.6× |
| Version | partial_cmp: ordered | 32768 | 902.38 µs | 318.60 µs | 0.4× |
| Version | partial_cmp: equal | 32768 | 20.31 µs | 634.37 µs | 31.2× |
| Clock | tick | 32768 | 2.02 ms | 1.54 ms | 0.8× |
| Clock | fork | 32768 | 155.82 µs | 2.37 ms | 15.2× |
| Clock | join | 32768 | 2.79 ms | 3.99 ms | 1.4× |
| Clock | sync | 32768 | 3.04 ms | 8.15 ms | 2.7× |
| Clock | send | 32768 | 1.86 ms | 2.90 ms | 1.6× |
| Clock | receive | 32768 | 2.95 ms | 3.70 ms | 1.3× |
