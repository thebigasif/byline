//! The dataset block: `Dataset`.

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError, SeoPage};

/// A dataset block: data the post publishes for others to use.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Dataset {
    /// The dataset.
    pub name: String,
    /// What it holds and how it was gathered, in plain text.
    pub description: String,
    /// Words people would search for it by.
    pub keywords: Vec<String>,
    /// The licence it is shared under, such as a Creative Commons address.
    pub license: Option<AbsoluteUrl>,
    /// Where the data itself can be downloaded, if anywhere.
    pub download: Option<AbsoluteUrl>,
}

impl Dataset {
    /// A dataset called `name`, described by `description`; Google needs both.
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            keywords: Vec::new(),
            license: None,
            download: None,
        }
    }
}

/// The `Dataset` at `<page>#dataset-<number>`. Its `url` is the post,
/// the page that describes it, and its creator is the post's author.
pub(crate) fn dataset_node(
    dataset: &Dataset,
    page: &SeoPage,
    url: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let mut node = SchemaNode::new("Dataset")
        .with_id(node_id(url, &format!("dataset-{number}"))?)
        .with("name", SchemaValue::text(&dataset.name))
        .with("description", SchemaValue::text(&dataset.description))
        .with("url", SchemaValue::Url(url.clone()));
    if !dataset.keywords.is_empty() {
        let keywords = dataset.keywords.iter().map(|word| SchemaValue::text(word));
        node.set("keywords", SchemaValue::List(keywords.collect()));
    }
    if let Some(license) = &dataset.license {
        node.set("license", SchemaValue::Url(license.clone()));
    }
    if let Some(author) = &page.author {
        let person = SchemaNode::new("Person").with("name", SchemaValue::text(author));
        node.set("creator", SchemaValue::Node(Box::new(person)));
    }
    if let Some(download) = &dataset.download {
        let file =
            SchemaNode::new("DataDownload").with("contentUrl", SchemaValue::Url(download.clone()));
        node.set("distribution", SchemaValue::Node(Box::new(file)));
    }
    Ok(node)
}
