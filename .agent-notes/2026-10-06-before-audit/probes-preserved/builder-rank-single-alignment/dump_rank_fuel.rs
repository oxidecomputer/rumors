//! Temporary measurement probe: per-sample fuel for the rank kernels.
use fuzzfit_harness::drive::for_each_deterministic_program;
fn main() {
    for_each_deterministic_program(4096, |case, _, samples| {
        for (step, s) in samples.iter().enumerate() {
            if s.kernel == "ff_rank_add" || s.kernel == "ff_rank_checked_sub" {
                println!("{case} {step} {} {} {} {}", s.kernel, s.rejected, s.denom_bits, s.fuel);
            }
        }
    });
}
