//! File names chosen by the sending device.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|parts: Vec<String>| {
    if let Ok(path) = ec_transfer::RelPath::new(parts) {
        for c in path.components() {
            assert!(!c.is_empty() && c != "." && c != "..");
            assert!(!c.contains(['/', '\\', ':', '\0']));
        }
    }
});
