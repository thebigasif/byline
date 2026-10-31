//! The webhook event catalogue.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Declares [`Event`] once: each variant with its wire name, the catalogue in
/// order, and the name lookup, so the three can never disagree.
macro_rules! events {
    ($($(#[$doc:meta])* $variant:ident => $name:literal,)+) => {
        /// Something that happened on a site, announced to webhook receivers.
        ///
        /// The wire name is `resource.verb`, as in `post.published`
        /// (`docs/standards/api.md`). Collection entries use their resource in
        /// the singular with hyphens: `glossary-term`, `case-study` and
        /// `changelog-entry`. Serde reads and writes the wire name.
        ///
        /// # Examples
        ///
        /// ```
        /// use khalid_webhooks::Event;
        ///
        /// let event: Event = "post.published.edited".parse()?;
        /// assert_eq!(event, Event::PostPublishedEdited);
        /// assert_eq!(event.as_str(), "post.published.edited");
        /// # Ok::<(), khalid_webhooks::ParseEventError>(())
        /// ```
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[non_exhaustive]
        pub enum Event {
            $($(#[$doc])* #[serde(rename = $name)] $variant,)+
        }

        impl Event {
            /// Every event, in catalogue order.
            pub const ALL: [Self; [$($name),+].len()] = [$(Self::$variant),+];

            /// The wire name, such as `post.published`.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }
        }
    };
}

events! {
    /// A post went live.
    PostPublished => "post.published",
    /// A live post was changed.
    PostPublishedEdited => "post.published.edited",
    /// A live post was taken down.
    PostUnpublished => "post.unpublished",
    /// A post was given a future publish time.
    PostScheduled => "post.scheduled",
    /// A post was deleted.
    PostDeleted => "post.deleted",
    /// A page went live.
    PagePublished => "page.published",
    /// A live page was changed.
    PagePublishedEdited => "page.published.edited",
    /// A live page was taken down.
    PageUnpublished => "page.unpublished",
    /// A page was given a future publish time.
    PageScheduled => "page.scheduled",
    /// A page was deleted.
    PageDeleted => "page.deleted",
    /// A glossary term went live.
    GlossaryTermPublished => "glossary-term.published",
    /// A live glossary term was changed.
    GlossaryTermPublishedEdited => "glossary-term.published.edited",
    /// A live glossary term was taken down.
    GlossaryTermUnpublished => "glossary-term.unpublished",
    /// A glossary term was given a future publish time.
    GlossaryTermScheduled => "glossary-term.scheduled",
    /// A glossary term was deleted.
    GlossaryTermDeleted => "glossary-term.deleted",
    /// A case study went live.
    CaseStudyPublished => "case-study.published",
    /// A live case study was changed.
    CaseStudyPublishedEdited => "case-study.published.edited",
    /// A live case study was taken down.
    CaseStudyUnpublished => "case-study.unpublished",
    /// A case study was given a future publish time.
    CaseStudyScheduled => "case-study.scheduled",
    /// A case study was deleted.
    CaseStudyDeleted => "case-study.deleted",
    /// A changelog entry went live.
    ChangelogEntryPublished => "changelog-entry.published",
    /// A live changelog entry was changed.
    ChangelogEntryPublishedEdited => "changelog-entry.published.edited",
    /// A live changelog entry was taken down.
    ChangelogEntryUnpublished => "changelog-entry.unpublished",
    /// A changelog entry was given a future publish time.
    ChangelogEntryScheduled => "changelog-entry.scheduled",
    /// A changelog entry was deleted.
    ChangelogEntryDeleted => "changelog-entry.deleted",
    /// A tag was created.
    TagAdded => "tag.added",
    /// A visitor sent a form.
    FormSubmitted => "form.submitted",
    /// Someone subscribed.
    SubscriberAdded => "subscriber.added",
    /// A visitor left a comment.
    CommentAdded => "comment.added",
    /// The linter refused to publish a post.
    LintBlocked => "lint.blocked",
    /// A site audit finished.
    AuditFinished => "audit.finished",
    /// Site settings changed.
    SiteChanged => "site.changed",
    /// Something on the site needs the owner: broken links, a burst of
    /// 404s, a certificate near its end, a scheduled post that did not go
    /// live, or a post left unchanged for a long time.
    AlertRaised => "alert.raised",
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Event {
    type Err = ParseEventError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|event| event.as_str() == name)
            .ok_or_else(|| ParseEventError {
                name: name.to_owned(),
            })
    }
}

/// A name that is not in the event catalogue.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{name} is not a webhook event")]
pub struct ParseEventError {
    name: String,
}

impl ParseEventError {
    /// The name as given.
    pub fn name(&self) -> &str {
        &self.name
    }
}
