//! The group a view is counted under, from the `Referer` header and the
//! `utm_source` a link carries.

/// The sites whose readers count as AI search: `chatgpt.com`,
/// Perplexity, Gemini, Copilot and Claude, then a few more assistants. A host matches
/// a site or any address under it.
const AI_SITES: &[&str] = &[
    "chatgpt.com",
    "chat.openai.com",
    "perplexity.ai",
    "gemini.google.com",
    "bard.google.com",
    "copilot.microsoft.com",
    "copilot.cloud.microsoft",
    "claude.ai",
    "chat.deepseek.com",
    "chat.mistral.ai",
    "meta.ai",
    "grok.com",
    "you.com",
    "phind.com",
];

/// The `utm_source` names assistants put on their links, besides the
/// sites in [`AI_SITES`].
const AI_NAMES: &[&str] = &[
    "chatgpt",
    "openai",
    "perplexity",
    "gemini",
    "copilot",
    "claude",
];

/// Search engines by the name in their address, so every country's
/// `google.co.uk` or `bing.de` counts.
const SEARCH_NAMES: &[&str] = &[
    "google",
    "bing",
    "yahoo",
    "yandex",
    "baidu",
    "duckduckgo",
    "ecosia",
    "startpage",
    "qwant",
    "kagi",
    "naver",
    "seznam",
];

/// Search engines whose name is not enough to tell them apart.
const SEARCH_SITES: &[&str] = &["search.brave.com"];

/// The Google app on Android, which sends `android-app://` referrers.
const ANDROID_SEARCH: &str = "com.google.android.googlequicksearchbox";

/// Social sites and link aggregators.
const SOCIAL_SITES: &[&str] = &[
    "t.co",
    "twitter.com",
    "x.com",
    "facebook.com",
    "fb.com",
    "instagram.com",
    "threads.net",
    "linkedin.com",
    "lnkd.in",
    "reddit.com",
    "news.ycombinator.com",
    "bsky.app",
    "mastodon.social",
    "lobste.rs",
    "pinterest.com",
    "youtube.com",
    "tiktok.com",
];

/// Where a reader came from, as the group a view is counted under. The
/// referring address itself is never kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferrerGroup {
    /// No referrer: a typed address, a bookmark, or an app that sends none.
    Direct,
    /// Another page of the same site.
    Internal,
    /// A search engine.
    Search,
    /// A social site or link aggregator.
    Social,
    /// An AI assistant: `chatgpt.com`, Perplexity, Gemini, Copilot, Claude
    /// and others.
    Ai,
    /// Any other site.
    Other,
}

impl ReferrerGroup {
    /// Every group, in the order a report lists them.
    pub const ALL: [Self; 6] = [
        Self::Direct,
        Self::Internal,
        Self::Search,
        Self::Social,
        Self::Ai,
        Self::Other,
    ];

    /// The group of a view with `referer`, the `Referer` header as sent, on
    /// the site at `own_host`, from a link that carried `utm_source`.
    pub fn of(referer: Option<&str>, own_host: Option<&str>, utm_source: Option<&str>) -> Self {
        if utm_source.is_some_and(is_ai_source) {
            return Self::Ai;
        }
        let referer = referer.map(str::trim).unwrap_or_default();
        if referer.is_empty() {
            return Self::Direct;
        }
        let Some((scheme, host)) = scheme_and_host(referer) else {
            return Self::Other;
        };
        if scheme.eq_ignore_ascii_case("android-app") {
            return if host == ANDROID_SEARCH {
                Self::Search
            } else {
                Self::Other
            };
        }
        if own_host.map(bare_host).is_some_and(|own| own == host) {
            Self::Internal
        } else if under_any(&host, AI_SITES) {
            Self::Ai
        } else if is_search(&host) {
            Self::Search
        } else if under_any(&host, SOCIAL_SITES) {
            Self::Social
        } else {
            Self::Other
        }
    }

    /// The name stored in `analytics_counts.referrer_group`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Internal => "internal",
            Self::Search => "search",
            Self::Social => "social",
            Self::Ai => "ai",
            Self::Other => "other",
        }
    }

    /// The group stored as `name`, or `None` for a name this version does
    /// not know.
    pub fn from_stored(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|group| group.as_str() == name)
    }
}

/// Whether a `utm_source` names an AI assistant.
fn is_ai_source(source: &str) -> bool {
    let source = bare_host(source);
    AI_NAMES.contains(&source.as_str()) || under_any(&source, AI_SITES)
}

/// The scheme and the bare host of an absolute address, or `None` when it
/// has no host.
fn scheme_and_host(address: &str) -> Option<(&str, String)> {
    let (scheme, rest) = address.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = bare_host(authority.rsplit('@').next()?);
    let clean = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    clean.then_some((scheme, host))
}

/// `host` in lower case, without a port, a trailing dot or a leading
/// `www.`.
fn bare_host(host: &str) -> String {
    let host = host.split(':').next().unwrap_or_default();
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    match host.strip_prefix("www.") {
        Some(rest) => rest.to_owned(),
        None => host,
    }
}

/// Whether `host` is one of `sites` or an address under one.
fn under_any(host: &str, sites: &[&str]) -> bool {
    sites.iter().any(|site| {
        host.strip_suffix(site)
            .is_some_and(|rest| rest.is_empty() || rest.ends_with('.'))
    })
}

/// Whether `host` is a search engine: one of [`SEARCH_SITES`], or a
/// [`SEARCH_NAMES`] label followed by at most two short country labels,
/// as in `google.co.uk`.
fn is_search(host: &str) -> bool {
    if under_any(host, SEARCH_SITES) {
        return true;
    }
    let labels: Vec<&str> = host.split('.').collect();
    labels.iter().enumerate().any(|(at, label)| {
        let tail = labels.get(at + 1..).unwrap_or_default();
        SEARCH_NAMES.contains(label)
            && (1..=2).contains(&tail.len())
            && tail.iter().all(|part| (2..=3).contains(&part.len()))
    })
}
