//! One listener type for `khalid serve`, plain or over TLS, so the server
//! is started the same way either way.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::serve::Listener;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;

use crate::{Domain, TlsListener};

/// The port browsers use for HTTPS when an address names none.
const HTTPS_PORT: u16 = 443;

/// Where `khalid serve` answers: plain HTTP, or HTTPS with the site's own
/// certificate.
#[derive(Debug)]
pub enum Listening {
    /// Plain HTTP, such as on a computer of one's own or behind a proxy.
    Plain(TcpListener),
    /// HTTPS with the site's own certificate.
    Tls(TlsListener),
}

impl Listening {
    /// The address visitors use: `http://127.0.0.1:8080` for plain HTTP,
    /// and `https://blog.example.com` over TLS for `domain`, with the port
    /// when it is not 443. Over TLS with no domain, the listener's own.
    ///
    /// # Errors
    ///
    /// Returns the system's error when the listener's address cannot be
    /// read.
    pub fn address(&self, domain: Option<&Domain>) -> io::Result<String> {
        let local = Listener::local_addr(self)?;
        Ok(match (self, domain) {
            (Self::Plain(_), _) => format!("http://{local}"),
            (Self::Tls(_), None) => format!("https://{local}"),
            (Self::Tls(_), Some(domain)) if local.port() == HTTPS_PORT => {
                format!("https://{domain}")
            }
            (Self::Tls(_), Some(domain)) => format!("https://{domain}:{}", local.port()),
        })
    }
}

/// One connection, plain or over TLS.
#[derive(Debug)]
pub enum Connection {
    /// A plain TCP connection.
    Plain(TcpStream),
    /// A TLS connection whose handshake is done.
    Tls(Box<TlsStream<TcpStream>>),
}

impl Listener for Listening {
    type Io = Connection;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self {
            Self::Plain(listener) => {
                let (stream, address) = Listener::accept(listener).await;
                (Connection::Plain(stream), address)
            }
            Self::Tls(listener) => {
                let (stream, address) = listener.accept().await;
                (Connection::Tls(Box::new(stream)), address)
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        match self {
            Self::Plain(listener) => listener.local_addr(),
            Self::Tls(listener) => Listener::local_addr(listener),
        }
    }
}

impl AsyncRead for Connection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_shutdown(cx),
        }
    }
}
