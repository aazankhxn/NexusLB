#![no_main]
use libfuzzer_sys::fuzz_target;
use nexuslb_dataplane::DataplanePipeline;

fuzz_target!(|data: &[u8]| {
    // Fuzz protocol detection with arbitrary random byte sequences
    let _ = DataplanePipeline::detect_protocol(data);
});
