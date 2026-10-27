//! A listener that turns each accepted connection into a TLS one.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, io};

use axum::serve::Listener;
use rustls::ServerConfig;
use rustls::crypto::ring::default_provider;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use crate::{Certificates, HttpsError};

/// How long a client may take to finish the TLS handshake before its
/// connection is closed, so a slow or idle client cannot hold a slot.
const HANDSHAKE: Duration = Duration::from_secs(10);

/// How many handshakes may run at once. A connection past this is closed
/// at once, so a flood of half-open connections cannot use up memory.
const MAX_HANDSHAKES: usize = 1024;

/// How long to wait after the system refuses a connection, such as when
/// the process has no file handles left, before accepting again.
const ACCEPT_PAUSE: Duration = Duration::from_secs(1);

/// The protocol the server speaks inside TLS, offered in the handshake.
const HTTP_1_1: &[u8] = b"http/1.1";

/// A TCP listener whose connections are answered over TLS with the
/// certificate in [`Certificates`]. Hand it to `khalid_server::serve`.
///
/// Handshakes run beside each other, so a slow client never holds up the
/// next one, and a connection whose handshake fails or takes too long is
/// closed without reaching the server.
pub struct TlsListener {
    tcp: TcpListener,
    acceptor: TlsAcceptor,
    handshakes: JoinSet<Option<(TlsStream<TcpStream>, SocketAddr)>>,
}

impl fmt::Debug for TlsListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TlsListener")
            .field("tcp", &self.tcp)
            .field("handshakes", &self.handshakes.len())
            .finish_non_exhaustive()
    }
}

impl TlsListener {
    /// Answers the connections `tcp` accepts over TLS with the current
    /// certificate in `certificates`.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::Tls`] when the TLS settings cannot be built.
    pub fn new(tcp: TcpListener, certificates: Certificates) -> Result<Self, HttpsError> {
        let mut config = ServerConfig::builder_with_provider(Arc::new(default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|source| HttpsError::Tls { source })?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(certificates));
        config.alpn_protocols = vec![HTTP_1_1.to_vec()];
        Ok(Self {
            tcp,
            acceptor: TlsAcceptor::from(Arc::new(config)),
            handshakes: JoinSet::new(),
        })
    }
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            tokio::select! {
                accepted = self.tcp.accept() => match accepted {
                    Ok((stream, address)) if self.handshakes.len() < MAX_HANDSHAKES => {
                        self.handshakes.spawn(handshake(self.acceptor.clone(), stream, address));
                    }
                    Ok((_, address)) => {
                        tracing::warn!(%address, "too many TLS handshakes at once, so a connection was closed");
                    }
                    Err(error) => {
                        tracing::warn!(%error, "a connection could not be accepted");
                        tokio::time::sleep(ACCEPT_PAUSE).await;
                    }
                },
                Some(done) = self.handshakes.join_next() => {
                    if let Ok(Some(ready)) = done {
                        return ready;
                    }
                }
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.tcp.local_addr()
    }
}

/// Runs the TLS handshake on `stream` within its time limit.
async fn handshake(
    acceptor: TlsAcceptor,
    stream: TcpStream,
    address: SocketAddr,
) -> Option<(TlsStream<TcpStream>, SocketAddr)> {
    match tokio::time::timeout(HANDSHAKE, acceptor.accept(stream)).await {
        Ok(Ok(tls)) => Some((tls, address)),
        Ok(Err(error)) => {
            tracing::debug!(%address, %error, "a TLS handshake failed");
            None
        }
        Err(_) => {
            tracing::debug!(%address, "a TLS handshake took too long");
            None
        }
    }
}
