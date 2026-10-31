//! The listicle: `ItemList`.

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError};

/// One entry of a list: its name and the address that shows it, which
/// may be a heading on the same post, such as `.../best-tools/#ahrefs`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListEntry {
    /// The entry's name.
    pub name: String,
    /// Where it is shown; Google needs it to show the list as a carousel.
    pub url: AbsoluteUrl,
}

impl ListEntry {
    /// An entry called `name`, shown at `url`.
    pub fn new(name: &str, url: AbsoluteUrl) -> Self {
        Self {
            name: name.to_owned(),
            url,
        }
    }
}

/// A listicle: a post that ranks or gathers several things, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ItemList {
    /// The list's name, such as `Five tools for internal links`.
    pub name: String,
    /// The entries, first to last.
    pub entries: Vec<ListEntry>,
}

impl ItemList {
    /// A list called `name` of `entries`, in order.
    pub fn new(name: &str, entries: Vec<ListEntry>) -> Self {
        Self {
            name: name.to_owned(),
            entries,
        }
    }
}

/// The `ItemList` at `<page>#list-<number>`, each entry a `ListItem` with
/// its position counted from 1.
pub(crate) fn item_list_node(
    list: &ItemList,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let items: Vec<SchemaValue> = (1..)
        .zip(&list.entries)
        .map(|(position, entry)| {
            let item = SchemaNode::new("ListItem")
                .with("position", SchemaValue::Integer(position))
                .with("name", SchemaValue::text(&entry.name))
                .with("url", SchemaValue::Url(entry.url.clone()));
            SchemaValue::Node(Box::new(item))
        })
        .collect();
    let count = i64::try_from(items.len()).unwrap_or(i64::MAX);
    Ok(SchemaNode::new("ItemList")
        .with_id(node_id(page, &format!("list-{number}"))?)
        .with("name", SchemaValue::text(&list.name))
        .with("numberOfItems", SchemaValue::Integer(count))
        .with("itemListElement", SchemaValue::List(items)))
}
