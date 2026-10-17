//! Transfer control messages come from paired devices.
#![no_main]

use ec_transfer::proto::Control;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() <= ec_net::MAX_FRAME_LEN
        && let Ok(Control::File { path, .. }) = ec_net::decode::<Control>(data)
    {
        // A decoded path has passed validation; it must never climb out.
        assert!(path.components().iter().all(|c| c != ".." && !c.contains('/')));
    }
});
