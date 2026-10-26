//! [`SchemaBlock`]: the editor blocks that write their own schema (task 1.7).

use crate::schema::faq::faq_node;
use crate::schema::howto::how_to_node;
use crate::{FaqItem, HowTo, SchemaNode, SeoError, SeoPage, SeoSite};

/// An editor block on a page that writes its own schema node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaBlock {
    /// An FAQ block: `FAQPage`.
    Faq(Vec<FaqItem>),
    /// A how-to block: `HowTo`.
    HowTo(HowTo),
}

/// The schema nodes of the page's blocks, in the order of the blocks. A
/// block numbered `n` on the page gets an id ending in `-n` where it needs one.
pub(crate) fn block_nodes(site: &SeoSite, page: &SeoPage) -> Result<Vec<SchemaNode>, SeoError> {
    let url = site.url_for(&page.language, &page.path)?;
    let mut nodes = Vec::new();
    for (number, block) in (1..).zip(&page.blocks) {
        match block {
            SchemaBlock::Faq(items) => nodes.extend(faq_node(items, &url)?),
            SchemaBlock::HowTo(how_to) => nodes.push(how_to_node(how_to, &url, number)?),
        }
    }
    Ok(nodes)
}
