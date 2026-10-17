//! Cryptographic pieces of pairing.
//!
//! Pairing happens over a QUIC connection where the initiator (the device that scanned
//! the QR code) already pins the responder's certificate, because the QR code carries
//! the responder's [`DeviceId`]. What is left to prove is that the initiator really saw
//! that QR code. The QR code carries a one-time [`PairingSecret`]; the initiator proves
//! it knows the secret with [`proof`], bound to both device ids and to the TLS session
//! (`channel_binding`, exported keying material). Both devices then show the same
//! [`verification_code`] and the user confirms on each.

use std::fmt;

use zeroize::Zeroizing;

use crate::{DeviceId, Error};

const PROOF_LABEL: &[u8] = b"ecosystem-connector pairing proof v1";
const CODE_LABEL: &[u8] = b"ecosystem-connector pairing code v1";

/// Label for the TLS keying material exporter used as channel binding.
pub const CHANNEL_BINDING_LABEL: &[u8] = b"EXPORTER-ecosystem-connector-pairing";

/// A random 256-bit secret that is valid for one pairing attempt.
#[derive(Clone)]
pub struct PairingSecret(Zeroizing<[u8; 32]>);

impl PairingSecret {
    /// Generates a new secret from the operating system's secure random source.
    pub fn generate() -> Self {
        Self(Zeroizing::new(rand::random()))
    }

    /// Parses the 64-character hex form used in invites.
    pub fn from_hex(s: &str) -> Result<Self, Error> {
        let mut bytes = Zeroizing::new([0u8; 32]);
        hex::decode_to_slice(s, bytes.as_mut_slice())
            .map_err(|_| Error::InvalidInvite("secret must be 64 hex characters"))?;
        Ok(Self(bytes))
    }

    /// The hex form used in invites.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0.as_slice())
    }

    fn hasher(&self, label: &[u8]) -> blake3::Hasher {
        let mut hasher = blake3::Hasher::new_keyed(&self.0);
        hasher.update(label);
        hasher
    }
}

impl fmt::Debug for PairingSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairingSecret(..)")
    }
}

fn transcript(
    mut hasher: blake3::Hasher,
    initiator: &DeviceId,
    responder: &DeviceId,
    channel_binding: &[u8],
) -> blake3::Hash {
    hasher.update(initiator.as_bytes());
    hasher.update(responder.as_bytes());
    hasher.update(&(channel_binding.len() as u64).to_le_bytes());
    hasher.update(channel_binding);
    hasher.finalize()
}

/// The proof the initiator sends to show it knows the secret from the invite.
pub fn proof(
    secret: &PairingSecret,
    initiator: &DeviceId,
    responder: &DeviceId,
    channel_binding: &[u8],
) -> [u8; 32] {
    *transcript(
        secret.hasher(PROOF_LABEL),
        initiator,
        responder,
        channel_binding,
    )
    .as_bytes()
}

/// Checks a proof in constant time.
pub fn verify_proof(
    secret: &PairingSecret,
    initiator: &DeviceId,
    responder: &DeviceId,
    channel_binding: &[u8],
    received: &[u8; 32],
) -> bool {
    // `blake3::Hash` compares in constant time.
    transcript(
        secret.hasher(PROOF_LABEL),
        initiator,
        responder,
        channel_binding,
    ) == blake3::Hash::from_bytes(*received)
}

/// The six-digit code both devices display, formatted as `"123 456"`.
pub fn verification_code(
    secret: &PairingSecret,
    initiator: &DeviceId,
    responder: &DeviceId,
    channel_binding: &[u8],
) -> String {
    let hash = transcript(
        secret.hasher(CODE_LABEL),
        initiator,
        responder,
        channel_binding,
    );
    let mut first = [0u8; 8];
    first.copy_from_slice(&hash.as_bytes()[..8]);
    let n = u64::from_le_bytes(first) % 1_000_000;
    format!("{:03} {:03}", n / 1000, n % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (DeviceId, DeviceId) {
        (DeviceId::from_bytes([1; 32]), DeviceId::from_bytes([2; 32]))
    }

    #[test]
    fn proof_verifies_only_with_matching_inputs() {
        let (i, r) = ids();
        let secret = PairingSecret::generate();
        let p = proof(&secret, &i, &r, b"binding");
        assert!(verify_proof(&secret, &i, &r, b"binding", &p));
        assert!(
            !verify_proof(&secret, &r, &i, b"binding", &p),
            "swapped roles"
        );
        assert!(
            !verify_proof(&secret, &i, &r, b"other", &p),
            "other session"
        );
        assert!(!verify_proof(
            &PairingSecret::generate(),
            &i,
            &r,
            b"binding",
            &p
        ));
    }

    #[test]
    fn code_is_six_digits_and_deterministic() {
        let (i, r) = ids();
        let secret = PairingSecret::generate();
        let code = verification_code(&secret, &i, &r, b"x");
        assert_eq!(code.len(), 7);
        assert!(code.chars().enumerate().all(|(n, c)| if n == 3 {
            c == ' '
        } else {
            c.is_ascii_digit()
        }));
        assert_eq!(code, verification_code(&secret, &i, &r, b"x"));
    }

    #[test]
    fn secret_hex_round_trip_and_debug_is_redacted() {
        let secret = PairingSecret::generate();
        let parsed = PairingSecret::from_hex(&secret.to_hex()).unwrap();
        assert_eq!(parsed.to_hex(), secret.to_hex());
        assert_eq!(format!("{secret:?}"), "PairingSecret(..)");
        assert!(PairingSecret::from_hex("abc").is_err());
    }
}
