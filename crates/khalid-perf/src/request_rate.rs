//! How many requests a second `khalid serve` answers for one cached post
//! page, read from outside the process the way `oha` reads it.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use khalid_core::Clock;

use crate::ServeError;

/// How long one response may take before the run counts as failed.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// The responses `khalid serve` answered over a run, and how long it was.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestRate {
    answered: u64,
    seconds: f64,
}

impl RequestRate {
    /// A measure already taken: `answered` whole responses in `seconds`.
    pub fn new(answered: u64, seconds: f64) -> Self {
        Self { answered, seconds }
    }

    /// Opens `connections` keep-alive connections to `host` and, on each
    /// at once, asks for `address` again as soon as the last response has
    /// fully arrived, until `duration` has passed on `clock`.
    ///
    /// # Errors
    ///
    /// [`ServeError::Request`] when a connection fails, or a response is
    /// not a `200` with a length.
    pub fn measure(
        host: &str,
        address: &str,
        (connections, duration): (usize, Duration),
        clock: &dyn Clock,
    ) -> Result<Self, ServeError> {
        let started = clock.now();
        let until = started
            + chrono::Duration::from_std(duration).unwrap_or_else(|_| chrono::Duration::zero());
        let each: Vec<io::Result<u64>> = thread::scope(|scope| {
            let runs: Vec<_> = (0..connections.max(1))
                .map(|_| scope.spawn(|| keep_asking(host, address, until, clock)))
                .collect();
            runs.into_iter()
                .map(|run| {
                    run.join()
                        .unwrap_or_else(|_| Err(io::Error::other("stopped")))
                })
                .collect()
        });
        let mut answered = 0;
        for run in each {
            answered += run.map_err(|source| ServeError::Request {
                address: address.to_owned(),
                source,
            })?;
        }
        Ok(Self::new(answered, seconds_since(clock, started)))
    }

    /// How many whole responses arrived.
    pub fn answered(&self) -> u64 {
        self.answered
    }

    /// Whole responses a second over the run, or 0 for a run of no length.
    pub fn per_second(&self) -> f64 {
        if self.seconds > 0.0 {
            // No run answers 2^32 responses, so the count is exact.
            f64::from(u32::try_from(self.answered).unwrap_or(u32::MAX)) / self.seconds
        } else {
            0.0
        }
    }

    /// The measure as the JSON object `scripts/perf.sh` merges into
    /// `perf/current.json`, with the name `perf/baseline.json` uses.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"cached_post_requests_per_second\": {:.2}}}\n",
            self.per_second()
        )
    }
}

/// Reads one whole response from `reader`: its status line, which must be
/// `200`, its headers, and as many body bytes as its `content-length`.
///
/// # Errors
///
/// [`io::ErrorKind::InvalidData`] for another status or a response with no
/// length, and the read's own error when the connection ends early.
pub fn read_response(reader: &mut impl BufRead) -> io::Result<()> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line.split_whitespace().nth(1) != Some("200") {
        return Err(invalid(format!("the server answered {}", line.trim())));
    }
    let mut length = None;
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse::<u64>().ok();
        }
    }
    let length = length.ok_or_else(|| invalid("a response without a length".to_owned()))?;
    let copied = io::copy(&mut reader.by_ref().take(length), &mut io::sink())?;
    if copied < length {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(())
}

/// One connection's run: asks for `address` until `until`, and gives back
/// how many whole responses arrived.
fn keep_asking(
    host: &str,
    address: &str,
    until: DateTime<Utc>,
    clock: &dyn Clock,
) -> io::Result<u64> {
    let stream = TcpStream::connect(host)?;
    stream.set_read_timeout(Some(RESPONSE_TIMEOUT))?;
    stream.set_nodelay(true)?;
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let request =
        format!("GET {address} HTTP/1.1\r\nHost: {host}\r\nAccept-Encoding: br, gzip\r\n\r\n");
    let mut answered = 0;
    while clock.now() < until {
        writer.write_all(request.as_bytes())?;
        read_response(&mut reader)?;
        answered += 1;
    }
    Ok(answered)
}

/// Seconds from `start` to now on `clock`.
fn seconds_since(clock: &dyn Clock, start: DateTime<Utc>) -> f64 {
    let micros = (clock.now() - start).num_microseconds().unwrap_or(i64::MAX);
    f64::from(u32::try_from(micros).unwrap_or(u32::MAX)) / 1e6
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
