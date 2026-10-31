//! How often one key may call the API, and how many wrong keys one
//! address may send, before the API answers 429 (RULES.md BL-SEC-5).

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use axum::extract::{ConnectInfo, Request};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, TimeDelta, Utc};
use khalid_core::{Clock, UserMessage};

use crate::keys::KeyId;
use crate::problem::{Problem, ProblemType};

/// The time each limit is counted over.
const SPAN: TimeDelta = TimeDelta::minutes(1);

/// How many keys and addresses are remembered at most. Past it, the ones
/// whose limit has filled up again are forgotten first, so requests from
/// many addresses cannot grow memory without end.
const MAX_BUCKETS: usize = 50_000;

/// What one limit counts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Bucket {
    /// Wrong keys from one network: an IPv4 address, or an IPv6 /64.
    Refused([u8; 16]),
    /// Requests from one key, or from one staff member's session.
    Key(KeyId),
}

/// Whether a look at a bucket takes a request from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Take {
    /// Only says whether a request is left.
    Look,
    /// Takes the request when one is left.
    Count,
}

/// Which API a request came through, since each has its own limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Api {
    /// The Content API, read by themes and front ends for every reader.
    Content,
    /// The Admin API, which changes the site.
    Admin,
}

/// The request limits both APIs share, kept in memory.
///
/// Each limit allows its whole minute's worth of requests at once and
/// earns one back each sixtieth of the minute's count, so a key that
/// keeps under its rate is never refused. A request is counted against
/// its key only once the key is accepted. A wrong key is counted against
/// the network it came from, and an address with no wrong keys left is
/// refused before its key is checked, since checking an Argon2id hash is
/// slow on purpose. A request with no key costs nothing and is not counted.
#[derive(Debug)]
pub struct ApiLimits {
    clock: Box<dyn Clock>,
    /// When each bucket is full again. A time already past means full.
    full_at: Mutex<BTreeMap<Bucket, DateTime<Utc>>>,
}

impl ApiLimits {
    /// How many Admin API requests one key or staff session may make a minute.
    pub const ADMIN_PER_MINUTE: i32 = 600;

    /// How many Content API requests one content key may make a minute.
    pub const CONTENT_PER_MINUTE: i32 = 6000;

    /// How many wrong keys one address may send a minute.
    pub const REFUSED_PER_MINUTE: i32 = 10;

    /// Limits that read the time from `clock`.
    pub fn new(clock: impl Clock + 'static) -> Self {
        Self {
            clock: Box::new(clock),
            full_at: Mutex::new(BTreeMap::new()),
        }
    }

    /// How long `address` must wait before it may send a key again, or
    /// `None` when it may now. Counts nothing.
    pub(crate) fn paused(&self, address: IpAddr) -> Option<Duration> {
        let bucket = Bucket::Refused(network(address));
        self.take(bucket, Self::REFUSED_PER_MINUTE, Take::Look)
            .err()
    }

    /// Counts a wrong key from `address`.
    pub(crate) fn refused(&self, address: IpAddr) {
        let bucket = Bucket::Refused(network(address));
        // A refusal past the limit only happens when two arrive together;
        // the next one is paused either way.
        let _ = self.take(bucket, Self::REFUSED_PER_MINUTE, Take::Count);
    }

    /// Counts a request from the accepted `key` through `api`.
    ///
    /// # Errors
    ///
    /// How long the key must wait when it has no requests left; the
    /// refused request is not counted.
    pub(crate) fn admit(&self, key: &KeyId, api: Api) -> Result<(), Duration> {
        let per_minute = match api {
            Api::Content => Self::CONTENT_PER_MINUTE,
            Api::Admin => Self::ADMIN_PER_MINUTE,
        };
        self.take(Bucket::Key(key.clone()), per_minute, Take::Count)
    }

    /// Whether `bucket`, which holds `per_minute`, has a request left, and
    /// with [`Take::Count`], takes it.
    fn take(&self, bucket: Bucket, per_minute: i32, how: Take) -> Result<(), Duration> {
        let now = self.clock.now();
        let mut full_at = self.full_at.lock().unwrap_or_else(PoisonError::into_inner);
        let from = full_at.get(&bucket).copied().filter(|&at| at > now);
        let next = from.unwrap_or(now) + SPAN / per_minute;
        let wait = next - now - SPAN;
        if wait > TimeDelta::zero() {
            return Err(wait.to_std().unwrap_or_default());
        }
        if how == Take::Count {
            let known = full_at.contains_key(&bucket);
            if full_at.len() >= MAX_BUCKETS && !known {
                full_at.retain(|_, at| *at > now);
            }
            if full_at.len() < MAX_BUCKETS || known {
                full_at.insert(bucket, next);
            }
        }
        Ok(())
    }
}

/// The network `address` is in: itself for IPv4, its /64 for IPv6, which
/// one home or server usually has whole.
fn network(address: IpAddr) -> [u8; 16] {
    match address.to_canonical() {
        IpAddr::V4(v4) => v4.to_ipv6_mapped().octets(),
        IpAddr::V6(v6) => {
            let mut octets = v6.octets();
            octets.iter_mut().skip(8).for_each(|octet| *octet = 0);
            octets
        }
    }
}

/// The address `request` came from: the peer's, or, when the peer is a
/// proxy on this machine or the local network, the last address its
/// `X-Forwarded-For` names. `None` when the server passed no peer, as in
/// tests that call the router directly.
pub(crate) fn client(request: &Request) -> Option<IpAddr> {
    let ConnectInfo(peer) = request.extensions().get::<ConnectInfo<SocketAddr>>()?;
    Some(behind_proxy(peer.ip(), request.headers()))
}

/// The same rule as the comment form's limit in `khalid-comments`.
fn behind_proxy(peer: IpAddr, headers: &HeaderMap) -> IpAddr {
    let local = match peer.to_canonical() {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    };
    if !local {
        return peer;
    }
    let forwarded = headers.get_all("x-forwarded-for").into_iter().next_back();
    forwarded
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next())
        .and_then(|last| last.trim().parse().ok())
        .unwrap_or(peer)
}

/// The 429 for a key with no requests left for `wait`.
pub(crate) fn busy(wait: Duration) -> Response {
    let seconds = whole_seconds(wait);
    let message = UserMessage::new(
        "Too many requests came from this key in the last minute.",
        "Each key has a limit, so one busy or leaked key cannot slow the site for everyone.",
        format!(
            "Wait {}, then send fewer requests at a time.",
            in_seconds(seconds)
        ),
    );
    too_many(&message, seconds)
}

/// The 429 for an address that sent too many wrong keys, for `wait`.
pub(crate) fn paused(wait: Duration) -> Response {
    let seconds = whole_seconds(wait);
    let message = UserMessage::new(
        "Too many wrong keys came from this address in the last minute.",
        "Checking a key takes the server real work, so the API pauses an address that sends many wrong ones.",
        format!("Check the key, then try again in {}.", in_seconds(seconds)),
    );
    too_many(&message, seconds)
}

fn too_many(message: &UserMessage, seconds: u64) -> Response {
    let mut response = Problem::new(ProblemType::TooManyRequests, message).into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
    response
}

/// `wait` rounded up to whole seconds, at least one, as `Retry-After` takes.
fn whole_seconds(wait: Duration) -> u64 {
    let part = u64::from(wait.subsec_nanos() > 0);
    wait.as_secs().saturating_add(part).max(1)
}

fn in_seconds(seconds: u64) -> String {
    if seconds == 1 {
        "1 second".to_owned()
    } else {
        format!("{seconds} seconds")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::TimeZone;

    use super::*;

    /// A clock the test moves by hand.
    #[derive(Debug, Clone)]
    struct Hand(Arc<Mutex<DateTime<Utc>>>);

    impl Hand {
        fn start() -> Self {
            let at = Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap();
            Self(Arc::new(Mutex::new(at)))
        }

        fn wait(&self, by: TimeDelta) {
            *self.0.lock().unwrap() += by;
        }
    }

    impl Clock for Hand {
        fn now(&self) -> DateTime<Utc> {
            *self.0.lock().unwrap()
        }
    }

    fn key() -> KeyId {
        KeyId::try_new("0123456789abcdef").unwrap()
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn a_key_spends_its_minute_at_once_and_earns_one_back_each_tenth_of_a_second() {
        let clock = Hand::start();
        let limits = ApiLimits::new(clock.clone());
        for _ in 0..ApiLimits::ADMIN_PER_MINUTE {
            limits.admit(&key(), Api::Admin).unwrap();
        }
        let wait = limits.admit(&key(), Api::Admin).unwrap_err();
        assert_eq!(wait, Duration::from_millis(100));
        clock.wait(TimeDelta::milliseconds(99));
        assert!(
            limits.admit(&key(), Api::Admin).is_err(),
            "refusals are not counted"
        );
        clock.wait(TimeDelta::milliseconds(1));
        limits.admit(&key(), Api::Admin).unwrap();
        assert!(limits.admit(&key(), Api::Admin).is_err());
    }

    #[test]
    fn the_content_api_allows_ten_times_the_admin_api() {
        let limits = ApiLimits::new(Hand::start());
        for _ in 0..ApiLimits::CONTENT_PER_MINUTE {
            limits.admit(&key(), Api::Content).unwrap();
        }
        let wait = limits.admit(&key(), Api::Content).unwrap_err();
        assert_eq!(wait, Duration::from_millis(10));
    }

    #[test]
    fn an_address_is_paused_after_its_wrong_keys_and_looking_counts_nothing() {
        let clock = Hand::start();
        let limits = ApiLimits::new(clock.clone());
        let address = ip("203.0.113.7");
        for _ in 0..ApiLimits::REFUSED_PER_MINUTE {
            assert_eq!(limits.paused(address), None);
            limits.refused(address);
        }
        assert_eq!(limits.paused(address), Some(Duration::from_secs(6)));
        assert_eq!(limits.paused(ip("203.0.113.8")), None);
        clock.wait(TimeDelta::seconds(6));
        assert_eq!(limits.paused(address), None);
    }
}
