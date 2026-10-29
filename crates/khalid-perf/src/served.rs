//! `khalid serve`, started and measured from outside the process: cold
//! start to first byte, idle memory, and an edited post's page updating.

use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use khalid_core::Clock;

use crate::idle_memory::field_mb;
use crate::{MemorySplit, ServeError};

/// How long a request may wait for the server before it counts as failed.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// How long an edit may take to show before the measure gives up: long
/// enough that a slow rebuild on the 10,000-post site is still reported.
const EDIT_TIMEOUT_S: u64 = 60;

/// How often the edited page is asked for while waiting for the edit.
const POLL_EVERY: Duration = Duration::from_millis(1);

/// A running `khalid serve`. Dropping it stops the process.
#[derive(Debug)]
pub struct Served {
    child: Child,
    host: String,
    mount: String,
}

impl Served {
    /// Starts `binary` serving `site` on a free local port, and waits for
    /// the first byte of the site's home page. Gives back the server and
    /// how long, in milliseconds, that took from the start of the process.
    ///
    /// # Errors
    ///
    /// [`ServeError::Io`] when the binary cannot start,
    /// [`ServeError::NoAddress`] when it stops before saying where it
    /// answers, and [`ServeError::Request`] when the first request fails.
    pub fn start(binary: &Path, site: &Path, clock: &dyn Clock) -> Result<(Self, f64), ServeError> {
        let started = clock.now();
        let child = Command::new(binary)
            // The allocator setting the deployment image carries (see
            // docs/standards/performance.md): free a dead thread's pages
            // at once instead of waiting for an allocation that never
            // comes on an idle server. The measured server runs with the
            // setting production runs with.
            .env("MIMALLOC_ABANDONED_PAGE_PURGE", "1")
            .arg("--site")
            .arg(site)
            .args(["--json", "serve", "--listen", "127.0.0.1:0"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| io_error(binary, source))?;
        let mut served = Self {
            child,
            host: String::new(),
            mount: String::new(),
        };
        let line = served.announcement()?;
        (served.host, served.mount) = announced(&line).ok_or(ServeError::NoAddress)?;
        let mount = served.mount.clone();
        served.first_byte(&mount)?;
        Ok((served, elapsed_ms(clock, started)))
    }

    /// The site's address under the host, such as `/blog/`.
    pub fn mount(&self) -> &str {
        &self.mount
    }

    /// The process's resident memory now, in megabytes.
    ///
    /// # Errors
    ///
    /// [`ServeError::NoMemoryReport`] where `/proc` does not report it.
    pub fn resident_mb(&self) -> Result<f64, ServeError> {
        let status = fs::read_to_string(format!("/proc/{}/status", self.child.id()))
            .map_err(|_| ServeError::NoMemoryReport)?;
        resident_mb(&status).ok_or(ServeError::NoMemoryReport)
    }

    /// The process's thread count now, from `/proc/<pid>/status`, which
    /// Linux writes beside the memory fields. `None` where the status
    /// file does not say, such as on another platform, where the settle
    /// rests on the resident reading alone.
    pub fn thread_count(&self) -> Option<usize> {
        let status = fs::read_to_string(format!("/proc/{}/status", self.child.id())).ok()?;
        let line = status.lines().find(|line| line.starts_with("Threads:"))?;
        line.split_whitespace().nth(1)?.parse().ok()
    }

    /// Where the process's resident memory sits now. `None` where `/proc`
    /// does not say.
    pub fn memory_split(&self) -> Option<MemorySplit> {
        let read = |name: &str| fs::read_to_string(format!("/proc/{}/{name}", self.child.id()));
        let (status, rollup) = (
            read("status").ok()?,
            read("smaps_rollup").unwrap_or_default(),
        );
        MemorySplit::parse(&status, &rollup)
    }

    /// The whole response to a request for `address`, headers and body.
    ///
    /// # Errors
    ///
    /// [`ServeError::Request`] when the request fails.
    pub fn get(&self, address: &str) -> Result<String, ServeError> {
        let mut response = String::new();
        self.request(address)
            .and_then(|mut stream| stream.read_to_string(&mut response))
            .map_err(|source| request_error(address, source))?;
        Ok(response)
    }

    /// Writes `edited` over the file `post`, then asks for the page at
    /// `address` until it holds `marker`. Gives back how long, in
    /// milliseconds, that took from the write.
    ///
    /// # Errors
    ///
    /// [`ServeError::Io`] when the file cannot be written,
    /// [`ServeError::Request`] when a request fails, and
    /// [`ServeError::EditNotShown`] when the page never shows the edit.
    pub fn edit(
        &self,
        post: &Path,
        edited: &str,
        (address, marker): (&str, &str),
        clock: &dyn Clock,
    ) -> Result<f64, ServeError> {
        fs::write(post, edited).map_err(|source| io_error(post, source))?;
        let started = clock.now();
        let deadline = Duration::from_secs(EDIT_TIMEOUT_S);
        while elapsed_ms(clock, started) < deadline.as_secs_f64() * 1000.0 {
            if self.get(address)?.contains(marker) {
                return Ok(elapsed_ms(clock, started));
            }
            thread::sleep(POLL_EVERY);
        }
        Err(ServeError::EditNotShown {
            address: address.to_owned(),
            seconds: EDIT_TIMEOUT_S,
        })
    }

    /// The first line the server prints, which says where it answers.
    fn announcement(&mut self) -> Result<String, ServeError> {
        let stdout = self.child.stdout.take().ok_or(ServeError::NoAddress)?;
        let mut line = String::new();
        BufReader::new(stdout)
            .read_line(&mut line)
            .map_err(|_| ServeError::NoAddress)?;
        Ok(line)
    }

    /// Reads one byte of the response to a request for `address`.
    fn first_byte(&self, address: &str) -> Result<(), ServeError> {
        let mut byte = [0_u8; 1];
        self.request(address)
            .and_then(|mut stream| stream.read_exact(&mut byte))
            .map_err(|source| request_error(address, source))
    }

    /// Sends a `GET` for `address` that closes the connection after the
    /// response.
    fn request(&self, address: &str) -> io::Result<TcpStream> {
        let mut stream = TcpStream::connect(&self.host)?;
        stream.set_read_timeout(Some(REQUEST_TIMEOUT))?;
        let host = &self.host;
        write!(
            stream,
            "GET {address} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        )?;
        Ok(stream)
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        // A server that already stopped has nothing left to stop.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The host and the site's path from the line `khalid serve --json` prints,
/// `{"address":"http://127.0.0.1:8080/blog/"}`.
pub fn announced(line: &str) -> Option<(String, String)> {
    let (_, rest) = line.split_once("\"http://")?;
    let (url, _) = rest.split_once('"')?;
    let slash = url.find('/')?;
    let (host, mount) = url.split_at(slash);
    Some((host.to_owned(), mount.to_owned()))
}

/// The resident memory in megabytes from the text of `/proc/<pid>/status`,
/// whose `VmRSS` line counts kilobytes.
pub fn resident_mb(status: &str) -> Option<f64> {
    field_mb(status, "VmRSS:")
}

/// Milliseconds from `start` to now on `clock`.
fn elapsed_ms(clock: &dyn Clock, start: DateTime<Utc>) -> f64 {
    let micros = (clock.now() - start).num_microseconds().unwrap_or(i64::MAX);
    f64::from(u32::try_from(micros).unwrap_or(u32::MAX)) / 1000.0
}

fn io_error(path: &Path, source: io::Error) -> ServeError {
    ServeError::Io {
        path: PathBuf::from(path),
        source,
    }
}

fn request_error(address: &str, source: io::Error) -> ServeError {
    ServeError::Request {
        address: address.to_owned(),
        source,
    }
}
