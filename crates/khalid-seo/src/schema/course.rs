//! The course block: `Course`.

use khalid_core::AbsoluteUrl;

use crate::schema::{node_id, publisher_id};
use crate::{SchemaNode, SchemaValue, SeoError, SeoSite};

/// A course block: lessons the site teaches, in one or more posts.
///
/// Google stopped showing course rich results on 2025-09-09, so no rule
/// checks it; the name, description and provider it asked for stay
/// required, as schema.org readers still expect them.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Course {
    /// The course.
    pub name: String,
    /// What it teaches, in plain text.
    pub description: String,
}

impl Course {
    /// A course called `name` that teaches `description`.
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
        }
    }
}

/// The `Course` at `<page>#course-<number>`, provided by the site's
/// publisher.
pub(crate) fn course_node(
    course: &Course,
    site: &SeoSite,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    Ok(SchemaNode::new("Course")
        .with_id(node_id(page, &format!("course-{number}"))?)
        .with("name", SchemaValue::text(&course.name))
        .with("description", SchemaValue::text(&course.description))
        .with("provider", SchemaValue::Reference(publisher_id(site)?)))
}
