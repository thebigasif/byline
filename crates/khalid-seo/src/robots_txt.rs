//! `robots.txt`: which crawlers may read which paths, and where the
//! sitemap is.

use std::collections::BTreeMap;
use std::fmt;

use khalid_core::AbsoluteUrl;

use crate::{SeoError, SeoSite, sitemap_index_url};

/// The user-agent tokens of crawlers that gather text for AI models or AI
/// answers, as their operators publish them, for a site that wants to
/// choose for each one.
pub const KNOWN_AI_CRAWLERS: [&str; 14] = [
    "GPTBot",
    "OAI-SearchBot",
    "ChatGPT-User",
    "ClaudeBot",
    "Claude-SearchBot",
    "Claude-User",
    "PerplexityBot",
    "Perplexity-User",
    "Google-Extended",
    "Applebot-Extended",
    "CCBot",
    "Bytespider",
    "Amazonbot",
    "meta-externalagent",
];

/// A crawler's user-agent token, such as `GPTBot`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserAgent(String);

impl UserAgent {
    /// Checks a token and wraps it.
    ///
    /// # Errors
    ///
    /// [`SeoError::UserAgent`] when the token is empty or holds anything but
    /// ASCII letters, digits, `-` and `_`, which is all robots.txt allows.
    pub fn try_new(text: &str) -> Result<Self, SeoError> {
        let valid = !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(SeoError::UserAgent {
                agent: text.to_owned(),
            })
        }
    }

    /// The token as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A robots.txt path rule inside the mount folder, such as `/preview/` or
/// `/*.pdf$`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RobotsPath(String);

impl RobotsPath {
    /// Checks a path rule and wraps it.
    ///
    /// # Errors
    ///
    /// [`SeoError::RobotsPath`] when the rule does not start with `/` or
    /// holds whitespace or a control character.
    pub fn try_new(text: &str) -> Result<Self, SeoError> {
        if text.starts_with('/') && !text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            Ok(Self(text.to_owned()))
        } else {
            Err(SeoError::RobotsPath {
                path: text.to_owned(),
            })
        }
    }
}

/// Whether a crawler may read the blog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BotAccess {
    /// It may, apart from the paths blocked for every crawler.
    Allowed,
    /// It may not read any page of the blog.
    Blocked,
}

/// The site's choices for crawlers. By default every crawler may read every page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CrawlerPolicy {
    /// Paths inside the mount folder that no crawler may read.
    pub disallow: Vec<RobotsPath>,
    /// Crawlers with their own setting, by token.
    pub bots: BTreeMap<UserAgent, BotAccess>,
}

/// One group of lines: the crawlers it names, then its rules.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    agents: Vec<String>,
    disallow: Vec<String>,
    allow: Option<String>,
}

/// The blog's `robots.txt`, ready to serve or to merge into the main site's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotsTxt {
    groups: Vec<Group>,
    sitemap: AbsoluteUrl,
}

impl RobotsTxt {
    /// The robots.txt for `site`: a group for every crawler, a group for the
    /// allowed crawlers named in the policy, one for the blocked ones, and
    /// the sitemap index. Every path sits under the mount folder, so a blog
    /// in a folder says nothing about the rest of the site.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when the sitemap address cannot be built.
    pub fn for_site(site: &SeoSite) -> Result<Self, SeoError> {
        let mount = site.mount.as_str();
        let under_mount =
            |rule: &RobotsPath| format!("{}{}", site.mount.without_trailing_slash(), rule.0);
        let disallow: Vec<String> = site.crawlers.disallow.iter().map(under_mount).collect();
        let named = |access: BotAccess| -> Vec<String> {
            site.crawlers
                .bots
                .iter()
                .filter(|(_, setting)| **setting == access)
                .map(|(agent, _)| agent.as_str().to_owned())
                .collect()
        };
        let mut groups = vec![Group {
            agents: vec!["*".to_owned()],
            disallow: disallow.clone(),
            allow: Some(mount.to_owned()),
        }];
        let allowed = named(BotAccess::Allowed);
        if !allowed.is_empty() {
            groups.push(Group {
                agents: allowed,
                disallow,
                allow: Some(mount.to_owned()),
            });
        }
        let blocked = named(BotAccess::Blocked);
        if !blocked.is_empty() {
            groups.push(Group {
                agents: blocked,
                disallow: vec![mount.to_owned()],
                allow: None,
            });
        }
        Ok(Self {
            groups,
            sitemap: sitemap_index_url(site)?,
        })
    }
}

impl fmt::Display for RobotsTxt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for group in &self.groups {
            for agent in &group.agents {
                writeln!(f, "User-agent: {agent}")?;
            }
            for path in &group.disallow {
                writeln!(f, "Disallow: {path}")?;
            }
            if let Some(path) = &group.allow {
                writeln!(f, "Allow: {path}")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "Sitemap: {}", self.sitemap)
    }
}
