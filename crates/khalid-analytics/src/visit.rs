//! One row of the visitor log, and the parts of a visitor it keeps: a
//! shortened address and a code that changes every day.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::{Crawler, ReferrerGroup};

/// How many bytes of the hash a visitor code keeps.
const CODE_BYTES: usize = 8;

/// How many leading bits of an IPv6 address are kept: the part that names
/// a network rather than a device.
const IPV6_KEPT_BITS: u32 = 48;

/// One visit, as the visitor log keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    /// When the visit came, in UTC.
    pub at: DateTime<Utc>,
    /// The address asked for, with no query string, such as `/blog/geo/`.
    pub path: String,
    /// The status the request was answered with, such as 200 or 404.
    pub status: u16,
    /// The visitor's network address with its end set to zero.
    pub address: ShortAddress,
    /// The code that groups one visitor's visits on one day.
    pub code: VisitorCode,
    /// Whether a person or a bot came, and what is known of it.
    pub visitor: Visitor,
}

/// Who made a visit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visitor {
    /// A person's browser, sent from a group of sites.
    Person(ReferrerGroup),
    /// A bot, as its user agent named it, and whether its address is on
    /// the list its company publishes.
    Bot(Crawler, BotCheck),
}

/// Whether a bot's address is on the list its company publishes, the only
/// way to tell a real crawler from one that copies its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BotCheck {
    /// The address is on the list: it is the bot it claims to be.
    Listed,
    /// The address is not on the list: something else used the name.
    NotListed,
    /// There is no list to check against, for this bot or for now.
    NoList,
}

impl BotCheck {
    /// Every answer, in the order a report lists them.
    pub const ALL: [Self; 3] = [Self::Listed, Self::NotListed, Self::NoList];

    /// The name stored in the visitor log.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Listed => "listed",
            Self::NotListed => "not-listed",
            Self::NoList => "no-list",
        }
    }

    /// The answer stored as `name`, or `None` for a name this version does
    /// not know.
    pub fn from_stored(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|check| check.as_str() == name)
    }
}

/// A network address with its end set to zero, so it names a network of
/// up to 256 IPv4 addresses, or an IPv6 site, and never one device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShortAddress(IpAddr);

impl ShortAddress {
    /// `address` shortened: an IPv4 address keeps its first three parts,
    /// an IPv6 address its first 48 bits. An IPv4 address written inside
    /// IPv6 is shortened as IPv4.
    pub fn of(address: IpAddr) -> Self {
        match address.to_canonical() {
            IpAddr::V4(v4) => {
                let [a, b, c, _] = v4.octets();
                Self(IpAddr::V4(Ipv4Addr::new(a, b, c, 0)))
            }
            IpAddr::V6(v6) => {
                let kept = u128::MAX << (128 - IPV6_KEPT_BITS);
                Self(IpAddr::V6(Ipv6Addr::from(v6.to_bits() & kept)))
            }
        }
    }
}

impl fmt::Display for ShortAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The text was not a shortened address.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a shortened network address")]
pub struct ParseShortAddressError;

impl FromStr for ShortAddress {
    type Err = ParseShortAddressError;

    /// Reads an address written by [`ShortAddress`]'s `Display`, refusing
    /// one whose end is not zero, so a full address never reads as short.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let address: IpAddr = text.parse().map_err(|_| ParseShortAddressError)?;
        let short = Self::of(address);
        (short.0 == address)
            .then_some(short)
            .ok_or(ParseShortAddressError)
    }
}

/// The day's secret key for visitor codes. It lives only in memory and is
/// replaced every day, so codes from different days cannot be linked and
/// no code can be turned back into an address. `Debug` never shows it.
#[derive(Clone)]
pub struct VisitorKey([u8; 32]);

impl VisitorKey {
    /// A key from 32 random bytes, which the caller draws.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The code of the visitor at the full `address` with `user_agent`.
    pub fn code(&self, address: IpAddr, user_agent: Option<&str>) -> VisitorCode {
        let mut hash = Sha256::new();
        hash.update(self.0);
        hash.update(address.to_canonical().to_string());
        hash.update([0]);
        hash.update(user_agent.unwrap_or_default());
        let mut code = [0; CODE_BYTES];
        for (to, from) in code.iter_mut().zip(hash.finalize()) {
            *to = from;
        }
        VisitorCode(code)
    }
}

impl fmt::Debug for VisitorKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VisitorKey(hidden)")
    }
}

/// A code that groups one visitor's visits on one day, written as 16
/// hexadecimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisitorCode([u8; CODE_BYTES]);

impl fmt::Display for VisitorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

/// The text was not a visitor code.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a visitor code of 16 hexadecimal digits")]
pub struct ParseVisitorCodeError;

impl FromStr for VisitorCode {
    type Err = ParseVisitorCodeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != CODE_BYTES * 2 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ParseVisitorCodeError);
        }
        let mut code = [0; CODE_BYTES];
        for (byte, pair) in code.iter_mut().zip(text.as_bytes().chunks(2)) {
            let pair = std::str::from_utf8(pair).map_err(|_| ParseVisitorCodeError)?;
            *byte = u8::from_str_radix(pair, 16).map_err(|_| ParseVisitorCodeError)?;
        }
        Ok(Self(code))
    }
}
