use std::net::SocketAddr;

use crate::{DeviceId, Error, PairingSecret, Result};

const SCHEME: &str = "ecpair:1";
const MAX_INVITE_LEN: usize = 1024;
/// Most addresses an invite may list.
pub const MAX_INVITE_ADDRS: usize = 8;

/// What a QR code (or a copied link) carries to start pairing with a device.
///
/// Format: `ecpair:1?id=<device id hex>&s=<secret hex>&a=<addr>,<addr>`
#[derive(Clone, Debug)]
pub struct PairingInvite {
    /// The device to pair with. The initiator pins its certificate to this id.
    pub responder: DeviceId,
    /// The one-time secret.
    pub secret: PairingSecret,
    /// Addresses where the responder's pairing listener can be reached.
    pub addrs: Vec<SocketAddr>,
}

impl PairingInvite {
    /// Encodes the invite as a URI string.
    pub fn to_uri(&self) -> String {
        let addrs: Vec<String> = self.addrs.iter().map(SocketAddr::to_string).collect();
        format!(
            "{SCHEME}?id={}&s={}&a={}",
            self.responder,
            self.secret.to_hex(),
            addrs.join(",")
        )
    }

    /// Parses an invite URI. Input comes from a camera or the clipboard, so it is
    /// treated as untrusted: length and address count are limited and every field is
    /// validated.
    pub fn parse(uri: &str) -> Result<Self> {
        let uri = uri.trim();
        if uri.len() > MAX_INVITE_LEN {
            return Err(Error::InvalidInvite("too long"));
        }
        let (scheme, query) = uri
            .split_once('?')
            .ok_or(Error::InvalidInvite("missing parameters"))?;
        if scheme != SCHEME {
            return Err(Error::InvalidInvite("unsupported scheme or version"));
        }

        let (mut id, mut secret, mut addrs) = (None, None, None);
        for pair in query.split('&') {
            let (key, value) = pair
                .split_once('=')
                .ok_or(Error::InvalidInvite("malformed parameter"))?;
            let slot_taken = match key {
                "id" => id.replace(value.parse::<DeviceId>()?).is_some(),
                "s" => secret.replace(PairingSecret::from_hex(value)?).is_some(),
                "a" => addrs.replace(parse_addrs(value)?).is_some(),
                _ => false, // Unknown keys are ignored so newer versions can add fields.
            };
            if slot_taken {
                return Err(Error::InvalidInvite("duplicate parameter"));
            }
        }

        Ok(Self {
            responder: id.ok_or(Error::InvalidInvite("missing id"))?,
            secret: secret.ok_or(Error::InvalidInvite("missing secret"))?,
            addrs: addrs.ok_or(Error::InvalidInvite("missing addresses"))?,
        })
    }
}

fn parse_addrs(value: &str) -> Result<Vec<SocketAddr>> {
    let addrs = value
        .split(',')
        .map(|a| a.parse().map_err(|_| Error::InvalidInvite("bad address")))
        .collect::<Result<Vec<SocketAddr>>>()?;
    if addrs.is_empty() || addrs.len() > MAX_INVITE_ADDRS {
        return Err(Error::InvalidInvite("wrong number of addresses"));
    }
    Ok(addrs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invite() -> PairingInvite {
        PairingInvite {
            responder: DeviceId::from_bytes([7; 32]),
            secret: PairingSecret::generate(),
            addrs: vec![
                "192.168.1.20:4100".parse().unwrap(),
                "[fe80::1]:4100".parse().unwrap(),
            ],
        }
    }

    #[test]
    fn uri_round_trip() {
        let original = invite();
        let parsed = PairingInvite::parse(&original.to_uri()).unwrap();
        assert_eq!(parsed.responder, original.responder);
        assert_eq!(parsed.secret.to_hex(), original.secret.to_hex());
        assert_eq!(parsed.addrs, original.addrs);
    }

    #[test]
    fn rejects_malformed_invites() {
        let good = invite().to_uri();
        for bad in [
            String::new(),
            good.replace("ecpair:1", "ecpair:2"),
            good.replace("&a=", "&x="),
            format!("{good}&id={}", DeviceId::from_bytes([1; 32])),
            good.replace("192.168.1.20:4100", "not-an-address"),
            format!("{good}&pad={}", "a".repeat(2000)),
            good.replace(
                "&a=",
                &format!("&a={},", ["10.0.0.1:1"; MAX_INVITE_ADDRS].join(",")),
            ),
            good.replace("&s=", "&x="),
        ] {
            assert!(PairingInvite::parse(&bad).is_err(), "accepted: {bad}");
        }
    }

    #[test]
    fn ignores_unknown_parameters() {
        let uri = format!("{}&future=1", invite().to_uri());
        assert!(PairingInvite::parse(&uri).is_ok());
    }
}
