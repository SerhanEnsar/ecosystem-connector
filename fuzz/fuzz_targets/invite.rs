//! Pairing invites come from a camera or the clipboard.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data)
        && let Ok(invite) = ec_core::PairingInvite::parse(text)
    {
        // Whatever parses must survive a round trip.
        let again = ec_core::PairingInvite::parse(&invite.to_uri()).expect("round trip");
        assert_eq!(again.responder, invite.responder);
        assert_eq!(again.addrs, invite.addrs);
    }
});
