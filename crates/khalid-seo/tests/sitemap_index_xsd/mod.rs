//! The rules of `siteindex.xsd` 0.9 (sitemaps.org), checked in tests.

#![cfg(test)]

use roxmltree::{Document, Node};

use crate::sitemap_xsd::{SITEMAP_NS, check_lastmod, check_loc};

/// An index lists at most 50,000 sitemaps.
const MAX_SITEMAPS: usize = 50_000;

fn in_sitemap_ns(node: Node<'_, '_>, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node.tag_name().namespace() == Some(SITEMAP_NS)
}

/// Checks a `sitemapindex` document: the declaration, the root in the
/// sitemap namespace, and `sitemap` children that each hold `loc` and an
/// optional `lastmod`, in that order and nothing else. Returns the count.
pub(crate) fn check_sitemapindex(xml: &str) -> Result<usize, String> {
    if !xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n") {
        return Err("the file does not start with the UTF-8 XML declaration".to_owned());
    }
    let document = Document::parse(xml).map_err(|error| error.to_string())?;
    let root = document.root_element();
    if !in_sitemap_ns(root, "sitemapindex") {
        return Err("the root is not sitemapindex in the sitemap namespace".to_owned());
    }
    let mut count = 0;
    for sitemap in root.children().filter(Node::is_element) {
        if !in_sitemap_ns(sitemap, "sitemap") {
            return Err(format!("sitemapindex holds {:?}", sitemap.tag_name()));
        }
        let children: Vec<Node<'_, '_>> = sitemap.children().filter(Node::is_element).collect();
        match children.as_slice() {
            [loc] if in_sitemap_ns(*loc, "loc") => check_loc(*loc)?,
            [loc, lastmod] if in_sitemap_ns(*loc, "loc") && in_sitemap_ns(*lastmod, "lastmod") => {
                check_loc(*loc)?;
                check_lastmod(*lastmod)?;
            }
            _ => return Err("a sitemap must hold loc, then an optional lastmod".to_owned()),
        }
        count += 1;
    }
    if count > MAX_SITEMAPS {
        return Err(format!("{count} sitemaps is over the limit of 50,000"));
    }
    Ok(count)
}
