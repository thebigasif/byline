//! `reviewedBy`: the people who checked a page, an E-E-A-T signal.

use khalid_core::AbsoluteUrl;

use crate::schema::article::person_node;
use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError, SeoPage};

/// The `reviewedBy` value of `page`, the one reference or the list of them,
/// and a `Person` node per reviewer. A reviewer with an author page is
/// `<author page>#person`, the id their own page and posts use; one
/// without is `<page>#reviewer-1`, `#reviewer-2` and so on.
///
/// # Errors
///
/// [`SeoError::Url`] when an id cannot be built.
pub(crate) fn reviewers(
    page: &SeoPage,
    page_url: &AbsoluteUrl,
) -> Result<(Option<SchemaValue>, Vec<SchemaNode>), SeoError> {
    let mut references = Vec::with_capacity(page.reviewers.len());
    let mut people = Vec::with_capacity(page.reviewers.len());
    for (index, profile) in page.reviewers.iter().enumerate() {
        let mut person = person_node(profile, page_url)?;
        if profile.url.is_none() {
            let fragment = format!("reviewer-{}", index.saturating_add(1));
            person = person.with_id(node_id(page_url, &fragment)?);
        }
        if let Some(id) = person.id() {
            references.push(SchemaValue::Reference(id.clone()));
        }
        people.push(person);
    }
    let value = if references.len() > 1 {
        Some(SchemaValue::List(references))
    } else {
        references.pop()
    };
    Ok((value, people))
}
