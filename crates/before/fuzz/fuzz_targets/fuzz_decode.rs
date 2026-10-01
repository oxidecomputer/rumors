//! Coverage-guided entry point for canonical raw decoding.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    before_fuzz::under_heap_cap(|| before_fuzz::targets::decode::run(data));
});
