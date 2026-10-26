//! [`SchemaBlock`]: the editor blocks that write their own schema (task 1.7).

use crate::schema::faq::faq_node;
use crate::{FaqItem, SchemaNode, SeoError, SeoPage, SeoSite};

/// An editor block on a page that writes its own schema node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaBlock {
    /// An FAQ block: `FAQPage`.
    Faq(Vec<FaqItem>),
}

/// The schema nodes of the page's blocks, in the order of the blocks.
pub(crate) fn block_nodes(site: &SeoSite, page: &SeoPage) -> Result<Vec<SchemaNode>, SeoError> {
    let url = site.url_for(&page.language, &page.path)?;
    let mut nodes = Vec::new();
    for block in &page.blocks {
        match block {
            SchemaBlock::Faq(items) => nodes.extend(faq_node(items, &url)?),
        }
    }
    Ok(nodes)
}
