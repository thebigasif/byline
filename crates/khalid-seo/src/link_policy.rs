//! The site-wide link policy: which `rel` and `target` each link gets.

use std::collections::BTreeSet;

use khalid_core::{AbsoluteUrl, DomainName, ExternalRel, LinkOpening, LinkSettings};

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

    /// `html` with each `<a>` tag given the `rel` and `target` this policy
    /// gives its `href`, by [`LinkPolicy::attributes_for`]. A tag that
    /// already has `rel` or `target` keeps its own. Everything else is left
    /// as it is.
    ///
    /// `html` is a rendered body, whose attribute values are quoted with
    /// `"` and escape `"` and `>`, so a tag ends at the first `>`.
    pub fn apply_to_html(&self, site: &SeoSite, html: &str) -> String {
        if self == &Self::default() {
            return html.to_owned();
        }
        let mut out = String::with_capacity(html.len() + html.len() / 8);
        let mut rest = html;
        while let Some(start) = rest.find("<a ") {
            let (before, from_tag) = rest.split_at(start);
            out.push_str(before);
            let Some(end) = from_tag.find('>') else {
                rest = from_tag;
                break;
            };
            let (tag, after) = from_tag.split_at(end);
            out.push_str(tag);
            if let Some(href) = attribute(tag, "href") {
                let link = self.attributes_for(site, &href.replace("&amp;", "&"));
                if let Some(rel) = link.rel().filter(|_| attribute(tag, "rel").is_none()) {
                    out.push_str(" rel=\"");
                    out.push_str(&rel);
                    out.push('"');
                }
                if let Some(target) = link.target().filter(|_| attribute(tag, "target").is_none()) {
                    out.push_str(" target=\"");
                    out.push_str(target);
                    out.push('"');
                }
            }
            rest = after;
        }
        out.push_str(rest);
        out
    }
}

/// The value of the attribute `name` in the opening tag `tag`, as written.
fn attribute<'tag>(tag: &'tag str, name: &str) -> Option<&'tag str> {
    let (_, value) = tag.split_once(&format!(" {name}=\""))?;
    value.split_once('"').map(|(value, _)| value)
}

impl From<&LinkSettings> for LinkPolicy {
    fn from(settings: &LinkSettings) -> Self {
        let domains = |names: &[DomainName]| -> Vec<Domain> {
            names
                .iter()
                .map(|name| Domain(name.as_str().to_owned()))
                .collect()
        };
        Self {
            external_rel: settings
                .rel
                .iter()
                .map(|rel| match rel {
                    ExternalRel::NoFollow => LinkRel::NoFollow,
                    ExternalRel::Sponsored => LinkRel::Sponsored,
                    ExternalRel::Ugc => LinkRel::Ugc,
                })
                .collect(),
            external_target: match settings.open {
                LinkOpening::SameTab => LinkTarget::SameTab,
                LinkOpening::NewTab => LinkTarget::NewTab,
            },
            affiliate_domains: domains(&settings.affiliate_domains),
            internal_domains: domains(&settings.internal_domains),
        }
    }
}
