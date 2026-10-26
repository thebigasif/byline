//! Rules about how a post's links are written: their `rel` words, their
//! scheme and their number.

use crate::{Hit, LinkKind, LintPost, Rule, RuleId, Severity, SiteView};

/// An internal link whose `rel` holds `nofollow`, in any case.
#[derive(Debug)]
pub(crate) struct NofollowInternalLink;

impl Rule for NofollowInternalLink {
    fn id(&self) -> RuleId {
        RuleId::new("nofollow-internal-link")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        post.links
            .iter()
            .filter(|link| link.kind == LinkKind::Internal)
            .filter(|link| link.rel.iter().any(|w| w.eq_ignore_ascii_case("nofollow")))
            .map(|link| Hit::about(link.target.clone()))
            .collect()
    }
}

/// A link over `http://` that stays on this site: one marked internal, or
/// one to the site's own host, with or without `www.`. The renderer marks
/// every link with a scheme as external, so the host check finds the rest.
#[derive(Debug)]
pub(crate) struct InsecureInternalLink;

impl Rule for InsecureInternalLink {
    fn id(&self) -> RuleId {
        RuleId::new("insecure-internal-link")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let own = site.host().map(bare_host);
        post.links
            .iter()
            .filter(|link| match http_host(&link.target) {
                None => false,
                Some(_) if link.kind == LinkKind::Internal => true,
                Some(host) => own.as_deref() == Some(bare_host(host).as_str()),
            })
            .map(|link| Hit::about(link.target.clone()))
            .collect()
    }
}

/// The host of an `http://` address, as written, or `None` for any other
/// address.
fn http_host(target: &str) -> Option<&str> {
    host_after(target, "http://")
}

/// The host of an `http://` or `https://` address, as written, or `None`
/// for a path or any other scheme.
pub(crate) fn web_host(target: &str) -> Option<&str> {
    host_after(target, "https://").or_else(|| http_host(target))
}

/// The host of `target` when it starts with `scheme`, in any case.
fn host_after<'t>(target: &'t str, scheme: &str) -> Option<&'t str> {
    let written = target.get(..scheme.len())?;
    if !written.eq_ignore_ascii_case(scheme) {
        return None;
    }
    let rest = target.get(scheme.len()..)?;
    rest.split(['/', ':', '?', '#']).next()
}

/// `host` in lowercase without a leading `www.`, so `www.Example.com` and
/// `example.com` compare equal.
pub(crate) fn bare_host(host: &str) -> String {
    let lower = host.to_ascii_lowercase();
    match lower.strip_prefix("www.") {
        Some(rest) => rest.to_owned(),
        None => lower,
    }
}

/// A post whose body holds more links than the site's limit, counting
/// internal and external links together.
#[derive(Debug)]
pub(crate) struct TooManyLinks {
    /// The most links a post may hold, from `Thresholds::max_links`.
    pub(crate) max: usize,
}

impl Rule for TooManyLinks {
    fn id(&self) -> RuleId {
        RuleId::new("too-many-links")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let count = post.links.len();
        (count > self.max)
            .then(|| Hit::about(format!("{count} links")))
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_http_addresses_have_an_http_host() {
        assert_eq!(http_host("http://example.com"), Some("example.com"));
        assert_eq!(http_host("Http://example.com:8080/a"), Some("example.com"));
        assert_eq!(http_host("https://example.com/"), None);
        assert_eq!(http_host("/blog/http://"), None);
        assert_eq!(http_host("http:"), None);
    }

    #[test]
    fn web_hosts_come_from_either_scheme_only() {
        assert_eq!(
            web_host("HTTPS://shop.example.org"),
            Some("shop.example.org")
        );
        assert_eq!(
            web_host("http://shop.example.org/a"),
            Some("shop.example.org")
        );
        assert_eq!(web_host("ftp://shop.example.org/"), None);
        assert_eq!(web_host("/blog/a/"), None);
    }

    #[test]
    fn hosts_compare_without_case_or_www() {
        assert_eq!(bare_host("WWW.Example.com"), "example.com");
        assert_eq!(bare_host("wwwexample.com"), "wwwexample.com");
    }
}
