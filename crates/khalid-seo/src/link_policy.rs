//! The site-wide link policy: which `rel` and `target` each link gets.

use std::collections::BTreeSet;

use khalid_core::AbsoluteUrl;

use crate::{SeoError, SeoSite};

/// A domain name, such as `amazon.com`, in lowercase.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(String);

impl Domain {
    /// Checks a domain name and stores it in lowercase.
    ///
    /// # Errors
    ///
    /// [`SeoError::Domain`] when the name is empty, starts or ends with a
    /// dot, or holds anything but ASCII letters, digits, `-` and `.`.
    pub fn try_new(text: &str) -> Result<Self, SeoError> {
        let lower = text.to_ascii_lowercase();
        let valid = !lower.is_empty()
            && !lower.starts_with('.')
            && !lower.ends_with('.')
            && lower
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.'));
        if valid {
            Ok(Self(lower))
        } else {
            Err(SeoError::Domain {
                domain: text.to_owned(),
            })
        }
    }

    /// The name in lowercase.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether `host` is this domain or one of its subdomains.
    fn covers(&self, host: &str) -> bool {
        host == self.0
            || host
                .strip_suffix(self.0.as_str())
                .is_some_and(|rest| rest.ends_with('.'))
    }
}

/// A `rel` value the site can put on every external link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum LinkRel {
    /// `nofollow`: the site does not vouch for the page.
    NoFollow,
    /// `sponsored`: the link is paid or an affiliate link.
    Sponsored,
    /// `ugc`: the link was written by a visitor, such as in a comment.
    Ugc,
}

impl LinkRel {
    fn as_str(self) -> &'static str {
        match self {
            Self::NoFollow => "nofollow",
            Self::Sponsored => "sponsored",
            Self::Ugc => "ugc",
        }
    }
}

/// Where an external link opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LinkTarget {
    /// In the same tab, the default.
    #[default]
    SameTab,
    /// In a new tab, with `noopener` so the new page cannot reach this one.
    NewTab,
}

/// The site's choices for links. By default no link gets anything added.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct LinkPolicy {
    /// What every external link gets.
    pub external_rel: BTreeSet<LinkRel>,
    /// Where external links open.
    pub external_target: LinkTarget,
    /// Domains whose links are always `sponsored`, with their subdomains.
    pub affiliate_domains: Vec<Domain>,
    /// Other domains that count as this site, such as the main site.
    pub internal_domains: Vec<Domain>,
}

/// The attributes the policy gives one link.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkAttributes {
    rel: BTreeSet<LinkRel>,
    target: LinkTarget,
}

impl LinkAttributes {
    /// The `rel` value, such as `nofollow sponsored noopener`, or `None`.
    pub fn rel(&self) -> Option<String> {
        let mut tokens: Vec<&str> = self.rel.iter().map(|rel| rel.as_str()).collect();
        if self.target == LinkTarget::NewTab {
            tokens.push("noopener");
        }
        (!tokens.is_empty()).then(|| tokens.join(" "))
    }

    /// The `target` value, `_blank` for a new tab, or `None`.
    pub fn target(&self) -> Option<&'static str> {
        (self.target == LinkTarget::NewTab).then_some("_blank")
    }
}

impl LinkPolicy {
    /// The attributes for a link to `href` on `site`.
    ///
    /// A relative link, a link to the site's own host with or without
    /// `www.`, or to an internal domain, gets nothing. Other `http` and
    /// `https` links get the external settings, and `sponsored` when an
    /// affiliate domain covers them. Other schemes, such as `mailto:`, get
    /// nothing.
    pub fn attributes_for(&self, site: &SeoSite, href: &str) -> LinkAttributes {
        let absolute = if href.starts_with("//") {
            AbsoluteUrl::parse(&format!("https:{href}")).ok()
        } else {
            AbsoluteUrl::parse(href).ok()
        };
        let Some(url) = absolute else {
            return LinkAttributes::default();
        };
        let host = url.host();
        let own = site.address.host();
        let bare = |name: &str| name.strip_prefix("www.").unwrap_or(name).to_owned();
        let internal = bare(host) == bare(own)
            || self
                .internal_domains
                .iter()
                .any(|domain| domain.covers(host));
        if internal {
            return LinkAttributes::default();
        }
        let mut rel = self.external_rel.clone();
        if self
            .affiliate_domains
            .iter()
            .any(|domain| domain.covers(host))
        {
            rel.insert(LinkRel::Sponsored);
        }
        LinkAttributes {
            rel,
            target: self.external_target,
        }
    }
}
