//! The FAQ block: `FAQPage`.

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError};

/// One question and its answer, as plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FaqItem {
    /// The question.
    pub question: String,
    /// The answer.
    pub answer: String,
}

impl FaqItem {
    /// A question and its answer.
    pub fn new(question: &str, answer: &str) -> Self {
        Self {
            question: question.to_owned(),
            answer: answer.to_owned(),
        }
    }
}

/// The `FAQPage` at `<page>#faq`, or `None` for an empty FAQ.
pub(crate) fn faq_node(
    items: &[FaqItem],
    page: &AbsoluteUrl,
) -> Result<Option<SchemaNode>, SeoError> {
    if items.is_empty() {
        return Ok(None);
    }
    let questions = items
        .iter()
        .map(|item| {
            let answer = SchemaNode::new("Answer").with("text", SchemaValue::text(&item.answer));
            let question = SchemaNode::new("Question")
                .with("name", SchemaValue::text(&item.question))
                .with("acceptedAnswer", SchemaValue::Node(Box::new(answer)));
            SchemaValue::Node(Box::new(question))
        })
        .collect();
    Ok(Some(
        SchemaNode::new("FAQPage")
            .with_id(node_id(page, "faq")?)
            .with("mainEntity", SchemaValue::List(questions)),
    ))
}
