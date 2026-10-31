//! The address lists search and AI companies publish for their crawlers,
//! so a visit that claims a bot's name can be checked (decision D2).

use std::collections::BTreeMap;
use std::net::IpAddr;

use serde::Deserialize;

use crate::{BotCheck, Crawler};

/// One company's published list, and the bots it covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RangeList {
    /// The names of the bots whose addresses the list holds, as
    /// [`Crawler::name`] gives them.
    pub crawlers: &'static [&'static str],
    /// Where the company publishes it.
    pub url: &'static str,
    /// The file it is kept in between fetches, such as `googlebot.json`.
    pub file: &'static str,
}

/// The longest list read, in bytes. Real ones are under 100 KB.
pub const LONGEST_LIST: usize = 1024 * 1024;

/// A list that held no ranges, or something other than the published
/// shape.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RangeListError {
    /// The text is not the JSON shape these lists are published in.
    #[error("the crawler address list is not in the published shape")]
    Shape(#[source] serde_json::Error),
    /// A range is not an address followed by a prefix length.
    #[error("the crawler address list has a range that is not an address and length: {0}")]
    Range(String),
    /// The list names no range at all.
    #[error("the crawler address list names no range")]
    Empty,
}

/// The published lists that were read, by the bots they cover.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrawlerRanges {
    by_url: BTreeMap<&'static str, Vec<Range>>,
}

impl CrawlerRanges {
    /// Every list this version reads. Google's common crawlers share one
    /// list. Anthropic, Apple and the other bots publish none this
    /// version knows of.
    pub const LISTS: &[RangeList] = &[
        RangeList {
            crawlers: &[
                "Googlebot",
                "Google-InspectionTool",
                "Storebot-Google",
                "GoogleOther",
            ],
            url: "https://developers.google.com/static/search/apis/ipranges/googlebot.json",
            file: "googlebot.json",
        },
        RangeList {
            crawlers: &["Bingbot"],
            url: "https://www.bing.com/toolbox/bingbot.json",
            file: "bingbot.json",
        },
        RangeList {
            crawlers: &["GPTBot"],
            url: "https://openai.com/gptbot.json",
            file: "gptbot.json",
        },
        RangeList {
            crawlers: &["ChatGPT-User"],
            url: "https://openai.com/chatgpt-user.json",
            file: "chatgpt-user.json",
        },
        RangeList {
            crawlers: &["OAI-SearchBot"],
            url: "https://openai.com/searchbot.json",
            file: "searchbot.json",
        },
        RangeList {
            crawlers: &["PerplexityBot"],
            url: "https://www.perplexity.ai/perplexitybot.json",
            file: "perplexitybot.json",
        },
        RangeList {
            crawlers: &["Perplexity-User"],
            url: "https://www.perplexity.ai/perplexity-user.json",
            file: "perplexity-user.json",
        },
    ];

    /// Reads `body` as the text of `list`, in place of any earlier copy,
    /// and gives how many ranges it holds. A body that is not a list
    /// changes nothing.
    ///
    /// # Errors
    ///
    /// [`RangeListError`] when `body` is not the published shape, has a
    /// range that does not read, or names none.
    pub fn add(&mut self, list: &RangeList, body: &[u8]) -> Result<usize, RangeListError> {
        let file: ListFile = serde_json::from_slice(body).map_err(RangeListError::Shape)?;
        let ranges = file
            .prefixes
            .iter()
            .map(|prefix| {
                let text = prefix.ipv4.as_deref().or(prefix.ipv6.as_deref());
                let text = text.unwrap_or_default();
                Range::parse(text).ok_or_else(|| RangeListError::Range(text.to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if ranges.is_empty() {
            return Err(RangeListError::Empty);
        }
        let count = ranges.len();
        self.by_url.insert(list.url, ranges);
        Ok(count)
    }

    /// Whether the full `address` is on the list of the bot `crawler`
    /// claims to be.
    pub fn check(&self, crawler: Crawler, address: IpAddr) -> BotCheck {
        let list = Self::LISTS
            .iter()
            .find(|list| list.crawlers.contains(&crawler.name()));
        let Some(ranges) = list.and_then(|list| self.by_url.get(list.url)) else {
            return BotCheck::NoList;
        };
        let address = address.to_canonical();
        if ranges.iter().any(|range| range.contains(address)) {
            BotCheck::Listed
        } else {
            BotCheck::NotListed
        }
    }
}

/// The published shape: `{"prefixes": [{"ipv4Prefix": "66.249.64.0/27"}]}`.
#[derive(Deserialize)]
struct ListFile {
    prefixes: Vec<Prefix>,
}

#[derive(Deserialize)]
struct Prefix {
    #[serde(rename = "ipv4Prefix")]
    ipv4: Option<String>,
    #[serde(rename = "ipv6Prefix")]
    ipv6: Option<String>,
}

/// A block of addresses: the bits an address must share with `network`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Range {
    V4(u32, u32),
    V6(u128, u128),
}

impl Range {
    /// `66.249.64.0/27` or `2001:4860:4801:10::/64`.
    fn parse(text: &str) -> Option<Self> {
        let (address, bits) = text.split_once('/')?;
        let bits: u32 = bits.parse().ok()?;
        match address.parse().ok()? {
            IpAddr::V4(v4) if bits <= 32 => {
                let mask = u32::MAX.checked_shl(32 - bits).unwrap_or(0);
                Some(Self::V4(v4.to_bits() & mask, mask))
            }
            IpAddr::V6(v6) if bits <= 128 => {
                let mask = u128::MAX.checked_shl(128 - bits).unwrap_or(0);
                Some(Self::V6(v6.to_bits() & mask, mask))
            }
            _ => None,
        }
    }

    fn contains(self, address: IpAddr) -> bool {
        match (self, address) {
            (Self::V4(network, mask), IpAddr::V4(v4)) => v4.to_bits() & mask == network,
            (Self::V6(network, mask), IpAddr::V6(v6)) => v6.to_bits() & mask == network,
            _ => false,
        }
    }
}
