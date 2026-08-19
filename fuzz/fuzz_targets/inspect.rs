#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../crates/zstdscope/tests/support.rs"]
mod support;

fuzz_target!(|data: &[u8]| {
    if let Ok(file) = zstdscope::inspect(data) {
        support::assert_model_invariants(data, &file);
    }
});
