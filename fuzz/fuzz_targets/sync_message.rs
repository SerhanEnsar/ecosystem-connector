//! Sync messages for linked files come from paired devices.
#![no_main]

use ec_sync::SyncMessage;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() <= ec_net::MAX_FRAME_LEN
        && let Ok(msg) = ec_net::decode::<SyncMessage>(data)
        && let Ok(SyncMessage::Link { name, .. }) = msg.validate()
    {
        // A validated name is one safe path component.
        assert!(!name.is_empty() && name != ".." && !name.contains('/') && !name.contains('\\'));
    }
});
