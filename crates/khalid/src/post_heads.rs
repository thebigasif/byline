//! The SEO head of each post page, which the Content API hands to a front
//! end that renders posts itself (headless mode, KH-INS-5). It is made from
//! the same SEO pages as the theme's heads, so the two cannot differ.

use std::collections::BTreeMap;

use khalid_api::resources::PostHead;
use khalid_core::UserMessage;
use khalid_seo::{PageKind, SeoPage, SeoSite, head_tags, page_graph};

use crate::build_message;

/// The head of each post page, by the page's full address.
pub(crate) type PostHeads = BTreeMap<String, PostHead>;

/// The heads of the post pages among `pages`, on `site`, after their
/// language versions are linked.
///
/// # Errors
///
/// Returns what, why and fix when a page's address cannot be built.
pub(crate) fn of(site: &SeoSite, pages: &[SeoPage]) -> Result<PostHeads, UserMessage> {
    let mut heads = PostHeads::new();
    for page in pages.iter().filter(|page| page.kind == PageKind::Post) {
        let made = site.url_for(&page.language, &page.path).and_then(|url| {
            let head = PostHead::new(&head_tags(site, page)?, &page_graph(site, page)?);
            Ok((url.as_str().to_owned(), head))
        });
        let (url, head) = made.map_err(|error| build_message::seo(&error))?;
        heads.insert(url, head);
    }
    Ok(heads)
}
