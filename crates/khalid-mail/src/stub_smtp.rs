//! A stand-in SMTP server for tests.

use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

/// An SMTP server on loopback that keeps every email as written, with no
/// encryption or sign-in. It runs on its own thread until the test process
/// ends, so a test can point a separate `khalid serve` at it.
#[derive(Debug, Clone)]
pub struct StubSmtp {
    port: u16,
    received: Arc<Mutex<Vec<String>>>,
}

impl StubSmtp {
    /// Starts the server on a free port.
    ///
    /// # Errors
    ///
    /// The [`io::Error`] when no port can be opened.
    pub fn start() -> io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let received = Arc::new(Mutex::new(Vec::new()));
        let keeping = Arc::clone(&received);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                // A session that breaks off keeps nothing; the next one runs.
                drop(session(stream, &keeping));
            }
        });
        Ok(Self { port, received })
    }

    /// The port it listens on, at `127.0.0.1`.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Every message taken so far, oldest first, with its headers.
    #[must_use]
    pub fn received(&self) -> Vec<String> {
        self.received
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Answers one client until it quits.
fn session(stream: TcpStream, received: &Mutex<Vec<String>>) -> io::Result<()> {
    let mut write = stream.try_clone()?;
    let mut lines = BufReader::new(stream).lines();
    write.write_all(b"220 stub ESMTP\r\n")?;
    while let Some(line) = lines.next() {
        let command = line?.to_ascii_uppercase();
        if command.starts_with("EHLO") {
            write.write_all(b"250-stub\r\n250 8BITMIME\r\n")?;
        } else if command == "DATA" {
            write.write_all(b"354 go on\r\n")?;
            let mut message = Vec::new();
            for line in lines.by_ref() {
                let line = line?;
                if line == "." {
                    break;
                }
                message.push(line);
            }
            received
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(message.join("\n"));
            write.write_all(b"250 kept\r\n")?;
        } else if command == "QUIT" {
            write.write_all(b"221 bye\r\n")?;
            return Ok(());
        } else {
            write.write_all(b"250 ok\r\n")?;
        }
    }
    Ok(())
}
