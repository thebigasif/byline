//! The software block: `SoftwareApplication` and its mobile and web kinds.

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::schema::product::review_node;
use crate::{AggregateRating, Offer, ProductReview, SchemaNode, SchemaValue, SeoError};

/// Where an app runs, which picks its schema.org type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AppKind {
    /// A program installed on a computer: `SoftwareApplication`.
    #[default]
    Software,
    /// An app for a phone or tablet: `MobileApplication`.
    Mobile,
    /// An app used in a browser: `WebApplication`.
    Web,
}

impl AppKind {
    fn schema_type(self) -> &'static str {
        match self {
            Self::Software => "SoftwareApplication",
            Self::Mobile => "MobileApplication",
            Self::Web => "WebApplication",
        }
    }
}

/// A software block: an app the post reviews or recommends.
///
/// Google shows an app only with its price, a free one at 0, and with a
/// review or readers' average, so an app with neither score blocks
/// publishing.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SoftwareApplication {
    /// The app.
    pub name: String,
    /// Where it runs.
    pub kind: AppKind,
    /// Its price.
    pub offer: Offer,
    /// Its schema.org category, such as `BusinessApplication` or `GameApplication`.
    pub category: Option<String>,
    /// The systems it runs on, such as `Linux, macOS, Windows`.
    pub operating_system: Option<String>,
    /// The writer's score, if the post gives one.
    pub review: Option<ProductReview>,
    /// Readers' average score, if the site collects them.
    pub aggregate: Option<AggregateRating>,
}

impl SoftwareApplication {
    /// An app called `name`, sold at `offer`.
    pub fn new(name: &str, offer: Offer) -> Self {
        Self {
            name: name.to_owned(),
            kind: AppKind::Software,
            offer,
            category: None,
            operating_system: None,
            review: None,
            aggregate: None,
        }
    }
}

/// The app at `<page>#app-<number>`.
pub(crate) fn app_node(
    app: &SoftwareApplication,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let mut node = SchemaNode::new(app.kind.schema_type())
        .with_id(node_id(page, &format!("app-{number}"))?)
        .with("name", SchemaValue::text(&app.name));
    let labels = [
        ("applicationCategory", &app.category),
        ("operatingSystem", &app.operating_system),
    ];
    for (property, label) in labels {
        if let Some(label) = label {
            node.set(property, SchemaValue::text(label));
        }
    }
    node.set("offers", SchemaValue::Node(Box::new(app.offer.node())));
    if let Some(review) = &app.review {
        node.set("review", review_node(review));
    }
    if let Some(aggregate) = app.aggregate {
        node.set(
            "aggregateRating",
            SchemaValue::Node(Box::new(aggregate.node())),
        );
    }
    Ok(node)
}
