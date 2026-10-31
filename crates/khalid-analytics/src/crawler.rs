//! Which requests come from bots, named by the user agent they send.

/// The longest user agent read, in bytes. Real ones are far shorter; the
/// rest of a longer one is not searched.
const LONGEST: usize = 512;

/// Known bots: a lower-case piece of the user agent, the name the log
/// shows, and the kind. The first match wins, so a longer piece comes
/// before a shorter one it contains.
const KNOWN: &[(&str, &str, CrawlerKind)] = &[
    ("googlebot", "Googlebot", CrawlerKind::Search),
    (
        "google-inspectiontool",
        "Google-InspectionTool",
        CrawlerKind::Search,
    ),
    ("storebot-google", "Storebot-Google", CrawlerKind::Search),
    ("googleother", "GoogleOther", CrawlerKind::Search),
    ("bingbot", "Bingbot", CrawlerKind::Search),
    ("applebot", "Applebot", CrawlerKind::Search),
    ("duckduckbot", "DuckDuckBot", CrawlerKind::Search),
    ("yandexbot", "YandexBot", CrawlerKind::Search),
    ("baiduspider", "Baiduspider", CrawlerKind::Search),
    ("petalbot", "PetalBot", CrawlerKind::Search),
    ("seznambot", "SeznamBot", CrawlerKind::Search),
    ("gptbot", "GPTBot", CrawlerKind::Ai),
    ("chatgpt-user", "ChatGPT-User", CrawlerKind::Ai),
    ("oai-searchbot", "OAI-SearchBot", CrawlerKind::Ai),
    ("claudebot", "ClaudeBot", CrawlerKind::Ai),
    ("claude-user", "Claude-User", CrawlerKind::Ai),
    ("claude-searchbot", "Claude-SearchBot", CrawlerKind::Ai),
    ("anthropic-ai", "anthropic-ai", CrawlerKind::Ai),
    ("perplexitybot", "PerplexityBot", CrawlerKind::Ai),
    ("perplexity-user", "Perplexity-User", CrawlerKind::Ai),
    ("duckassistbot", "DuckAssistBot", CrawlerKind::Ai),
    ("mistralai-user", "MistralAI-User", CrawlerKind::Ai),
    ("meta-externalagent", "Meta-ExternalAgent", CrawlerKind::Ai),
    ("amazonbot", "Amazonbot", CrawlerKind::Ai),
    ("bytespider", "Bytespider", CrawlerKind::Ai),
    ("ccbot", "CCBot", CrawlerKind::Ai),
    ("cohere-ai", "cohere-ai", CrawlerKind::Ai),
    ("youbot", "YouBot", CrawlerKind::Ai),
    ("ahrefsbot", "AhrefsBot", CrawlerKind::Seo),
    ("semrushbot", "SemrushBot", CrawlerKind::Seo),
    ("mj12bot", "MJ12bot", CrawlerKind::Seo),
    ("dotbot", "DotBot", CrawlerKind::Seo),
    ("rogerbot", "rogerbot", CrawlerKind::Seo),
    ("screaming frog", "Screaming Frog", CrawlerKind::Seo),
    ("dataforseobot", "DataForSeoBot", CrawlerKind::Seo),
    ("serpstatbot", "serpstatbot", CrawlerKind::Seo),
    ("barkrowler", "Barkrowler", CrawlerKind::Seo),
    ("facebookexternalhit", "Facebook", CrawlerKind::Social),
    ("facebookcatalog", "Facebook", CrawlerKind::Social),
    ("twitterbot", "Twitterbot", CrawlerKind::Social),
    ("linkedinbot", "LinkedInBot", CrawlerKind::Social),
    ("slackbot", "Slackbot", CrawlerKind::Social),
    ("slack-imgproxy", "Slackbot", CrawlerKind::Social),
    ("discordbot", "Discordbot", CrawlerKind::Social),
    ("telegrambot", "TelegramBot", CrawlerKind::Social),
    ("whatsapp", "WhatsApp", CrawlerKind::Social),
    ("pinterestbot", "Pinterestbot", CrawlerKind::Social),
    ("redditbot", "redditbot", CrawlerKind::Social),
    ("bluesky", "Bluesky", CrawlerKind::Social),
    ("mastodon", "Mastodon", CrawlerKind::Social),
];

/// Pieces that mark any other bot or script: the words bots use for
/// themselves, the common HTTP libraries, and headless browsers.
const ANY_BOT: &[&str] = &[
    "bot",
    "crawler",
    "spider",
    "curl/",
    "wget/",
    "python-requests",
    "python-urllib",
    "aiohttp",
    "go-http-client",
    "okhttp",
    "java/",
    "apache-httpclient",
    "node-fetch",
    "axios/",
    "scrapy",
    "headlesschrome",
    "phantomjs",
];

/// What kind of bot a crawler is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CrawlerKind {
    /// A search engine's crawler, such as Googlebot or Bingbot.
    Search,
    /// An AI company's crawler or an assistant fetching for a person, such
    /// as `GPTBot`, `ClaudeBot` or `PerplexityBot`.
    Ai,
    /// An SEO tool's crawler, such as `AhrefsBot` or `SemrushBot`.
    Seo,
    /// A link preview, such as Slack or `LinkedIn` showing a card.
    Social,
    /// Any other bot, script or headless browser.
    Other,
}

impl CrawlerKind {
    /// Every kind, in the order a report lists them.
    pub const ALL: [Self; 5] = [Self::Search, Self::Ai, Self::Seo, Self::Social, Self::Other];

    /// The name stored in the crawl log.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Ai => "ai",
            Self::Seo => "seo",
            Self::Social => "social",
            Self::Other => "other",
        }
    }

    /// The kind stored as `name`, or `None` for a name this version does
    /// not know.
    pub fn from_stored(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }
}

/// A bot, as its user agent names it. Anyone can send any user agent, so
/// this is what the request claimed to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Crawler {
    name: &'static str,
    kind: CrawlerKind,
}

impl Crawler {
    /// The bot that sent `user_agent`, or `None` when it reads as a
    /// browser. A request with no user agent is a bot: every browser sends
    /// one.
    pub fn recognise(user_agent: Option<&str>) -> Option<Self> {
        let user_agent = user_agent.map(str::trim).unwrap_or_default();
        if user_agent.is_empty() {
            return Some(Self::new("No user agent", CrawlerKind::Other));
        }
        let read: String = user_agent
            .chars()
            .scan(0, |bytes, c| {
                *bytes += c.len_utf8();
                (*bytes <= LONGEST).then_some(c)
            })
            .collect::<String>()
            .to_ascii_lowercase();
        if let Some((_, name, kind)) = KNOWN.iter().find(|(piece, ..)| read.contains(piece)) {
            return Some(Self::new(name, *kind));
        }
        ANY_BOT
            .iter()
            .any(|piece| read.contains(piece))
            .then(|| Self::new("Other bot", CrawlerKind::Other))
    }

    /// The bot the log names `name`, such as `GPTBot`, or `None` for a
    /// name this version does not know.
    pub fn named(name: &str) -> Option<Self> {
        let other = [
            ("Other bot", CrawlerKind::Other),
            ("No user agent", CrawlerKind::Other),
        ];
        KNOWN
            .iter()
            .map(|(_, known, kind)| (*known, *kind))
            .chain(other)
            .find(|(known, _)| *known == name)
            .map(|(known, kind)| Self::new(known, kind))
    }

    /// The bot's name as the log shows it, such as `Googlebot`, or
    /// `Other bot` for one with no entry of its own.
    pub fn name(self) -> &'static str {
        self.name
    }

    /// What kind of bot it is.
    pub fn kind(self) -> CrawlerKind {
        self.kind
    }

    fn new(name: &'static str, kind: CrawlerKind) -> Self {
        Self { name, kind }
    }
}
