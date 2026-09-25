#![no_main]
use libfuzzer_sys::fuzz_target;
use nexuslb_config::load_from_str;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = load_from_str(s);
    }
});
