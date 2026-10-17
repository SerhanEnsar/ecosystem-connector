//! TLS configuration: Ed25519 certificates, TLS 1.3 only, and certificate pinning by
//! [`DeviceId`] instead of certificate authorities.
//!
//! The custom verifiers below only replace the "is this certificate trusted?" decision.
//! The handshake signature is still checked with rustls' own implementation, which
//! proves the peer holds the private key of the certificate it presented.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use ec_core::{DeviceId, DeviceIdentity};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, WebPkiSupportedAlgorithms, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{CertificateError, DigitallySignedStruct, DistinguishedName, SignatureScheme};

use crate::Result;

/// Devices this endpoint accepts, shared between the node and its verifier.
pub(crate) type TrustedSet = Arc<RwLock<HashSet<DeviceId>>>;

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Which client certificates a server endpoint accepts.
#[derive(Debug)]
pub(crate) enum ClientPolicy {
    /// Only devices in the trust store.
    Trusted(TrustedSet),
    /// Any certificate. Used only by the pairing listener, which then requires the
    /// pairing proof before doing anything else.
    AnyForPairing,
}

#[derive(Debug)]
struct ClientVerifier {
    policy: ClientPolicy,
    algorithms: WebPkiSupportedAlgorithms,
}

impl ClientCertVerifier for ClientVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> std::result::Result<ClientCertVerified, rustls::Error> {
        if !intermediates.is_empty() {
            return Err(CertificateError::ApplicationVerificationFailure.into());
        }
        let accepted = match &self.policy {
            ClientPolicy::AnyForPairing => true,
            ClientPolicy::Trusted(set) => {
                let id = DeviceId::from_certificate(end_entity);
                // A poisoned lock means a panic elsewhere; refusing is the safe choice.
                set.read().map(|set| set.contains(&id)).unwrap_or(false)
            }
        };
        if accepted {
            Ok(ClientCertVerified::assertion())
        } else {
            Err(CertificateError::ApplicationVerificationFailure.into())
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 is not supported".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}

/// Accepts exactly one server certificate: the one whose hash is `expected`.
#[derive(Debug)]
struct PinnedServerVerifier {
    expected: DeviceId,
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for PinnedServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        if intermediates.is_empty() && DeviceId::from_certificate(end_entity) == self.expected {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(CertificateError::ApplicationVerificationFailure.into())
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 is not supported".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}

fn cert_and_key(
    identity: &DeviceIdentity,
) -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    let cert = CertificateDer::from(identity.cert_der().to_vec());
    let key = PrivatePkcs8KeyDer::from(identity.key_der().to_vec());
    (vec![cert], PrivateKeyDer::Pkcs8(key))
}

/// Builds the QUIC server configuration.
pub(crate) fn server_config(
    identity: &DeviceIdentity,
    policy: ClientPolicy,
    alpn: &[u8],
) -> Result<quinn::ServerConfig> {
    let provider = provider();
    let verifier = Arc::new(ClientVerifier {
        policy,
        algorithms: provider.signature_verification_algorithms,
    });
    let (certs, key) = cert_and_key(identity);
    let mut tls = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)?;
    tls.alpn_protocols = vec![alpn.to_vec()];
    let crypto = quinn::crypto::rustls::QuicServerConfig::try_from(tls)
        .map_err(|_| crate::Error::Protocol("no QUIC-compatible cipher suite"))?;
    let mut config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
    config.transport_config(transport());
    Ok(config)
}

/// Builds a QUIC client configuration that only accepts the server `expected`.
pub(crate) fn client_config(
    identity: &DeviceIdentity,
    expected: DeviceId,
    alpn: &[u8],
) -> Result<quinn::ClientConfig> {
    let provider = provider();
    let verifier = Arc::new(PinnedServerVerifier {
        expected,
        algorithms: provider.signature_verification_algorithms,
    });
    let (certs, key) = cert_and_key(identity);
    let mut tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(certs, key)?;
    tls.alpn_protocols = vec![alpn.to_vec()];
    let crypto = quinn::crypto::rustls::QuicClientConfig::try_from(tls)
        .map_err(|_| crate::Error::Protocol("no QUIC-compatible cipher suite"))?;
    let mut config = quinn::ClientConfig::new(Arc::new(crypto));
    config.transport_config(transport());
    Ok(config)
}

fn transport() -> Arc<quinn::TransportConfig> {
    let mut transport = quinn::TransportConfig::default();
    transport.keep_alive_interval(Some(std::time::Duration::from_secs(5)));
    if let Ok(timeout) = std::time::Duration::from_secs(20).try_into() {
        transport.max_idle_timeout(Some(timeout));
    }
    Arc::new(transport)
}

/// The device id of a connection's peer, from the certificate it authenticated with.
pub(crate) fn peer_id(conn: &quinn::Connection) -> Option<DeviceId> {
    let certs = conn
        .peer_identity()?
        .downcast::<Vec<CertificateDer<'static>>>()
        .ok()?;
    certs.first().map(|c| DeviceId::from_certificate(c))
}
