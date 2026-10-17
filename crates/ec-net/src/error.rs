use ec_core::DeviceId;

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Identity or trust store error.
    #[error(transparent)]
    Core(#[from] ec_core::Error),
    /// TLS configuration error.
    #[error("TLS: {0}")]
    Tls(#[from] rustls::Error),
    /// Socket error.
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    /// Starting a connection failed.
    #[error("connect: {0}")]
    Connect(#[from] quinn::ConnectError),
    /// The connection failed or was closed.
    #[error("connection: {0}")]
    Connection(#[from] quinn::ConnectionError),
    /// Writing to a stream failed.
    #[error("write: {0}")]
    Write(#[from] quinn::WriteError),
    /// Reading from a stream failed.
    #[error("read: {0}")]
    Read(#[from] quinn::ReadExactError),
    /// A message could not be decoded.
    #[error("decode: {0}")]
    Decode(#[from] postcard::Error),
    /// mDNS error.
    #[error("mDNS: {0}")]
    Mdns(#[from] mdns_sd::Error),
    /// A peer announced a message larger than [`crate::MAX_FRAME_LEN`].
    #[error("message of {0} bytes exceeds the limit")]
    FrameTooLarge(usize),
    /// The peer broke the protocol.
    #[error("protocol violation: {0}")]
    Protocol(&'static str),
    /// Tried to connect to a device that is not paired.
    #[error("device {} is not paired", .0.short())]
    NotPaired(DeviceId),
    /// Pairing did not complete.
    #[error("pairing failed: {0}")]
    PairingFailed(&'static str),
    /// None of the addresses could be reached.
    #[error("device could not be reached")]
    Unreachable,
    /// The operation took too long.
    #[error("timed out")]
    Timeout,
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Self::Timeout
    }
}

/// Result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;
