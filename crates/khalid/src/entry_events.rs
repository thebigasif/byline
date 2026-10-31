//! The webhooks `khalid serve` fires as the files of pages, glossary terms,
//! case studies and changelog entries are saved (audit brief G25). These
//! entries have no publish time, so an entry whose file is there is live: a
//! new file fires `.published`, a changed one `.published.edited` and a
//! removed one `.deleted`. A removed file that left a draft of the same
//! name under `drafts/` fires `.unpublished` instead.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use khalid_content::{ContentHash, Document, SiteContent};
use khalid_core::Slug;
use khalid_webhooks::Event;
use serde::Serialize;

use crate::announce::SiteAnnounce;
use crate::index;

/// Which kind of entry a file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Page,
    GlossaryTerm,
    CaseStudy,
    ChangelogEntry,
}

/// What happened to an entry between two renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    Published,
    Edited,
    Unpublished,
    Deleted,
}

/// The folder of each kind under `content/<language>/`, which a draft of
/// that kind keeps under `drafts/<language>/`.
const KIND_FOLDERS: [&str; 4] = ["pages", "glossary", "case-studies", "changelog"];

impl Kind {
    /// The name the entry goes under in a delivery's body.
    fn key(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::GlossaryTerm => "glossary_term",
            Self::CaseStudy => "case_study",
            Self::ChangelogEntry => "changelog_entry",
        }
    }

    /// The event for `change` to an entry of this kind.
    fn event(self, change: Change) -> Event {
        match (self, change) {
            (Self::Page, Change::Published) => Event::PagePublished,
            (Self::Page, Change::Edited) => Event::PagePublishedEdited,
            (Self::Page, Change::Unpublished) => Event::PageUnpublished,
            (Self::Page, Change::Deleted) => Event::PageDeleted,
            (Self::GlossaryTerm, Change::Published) => Event::GlossaryTermPublished,
            (Self::GlossaryTerm, Change::Edited) => Event::GlossaryTermPublishedEdited,
            (Self::GlossaryTerm, Change::Unpublished) => Event::GlossaryTermUnpublished,
            (Self::GlossaryTerm, Change::Deleted) => Event::GlossaryTermDeleted,
            (Self::CaseStudy, Change::Published) => Event::CaseStudyPublished,
            (Self::CaseStudy, Change::Edited) => Event::CaseStudyPublishedEdited,
            (Self::CaseStudy, Change::Unpublished) => Event::CaseStudyUnpublished,
            (Self::CaseStudy, Change::Deleted) => Event::CaseStudyDeleted,
            (Self::ChangelogEntry, Change::Published) => Event::ChangelogEntryPublished,
            (Self::ChangelogEntry, Change::Edited) => Event::ChangelogEntryPublishedEdited,
            (Self::ChangelogEntry, Change::Unpublished) => Event::ChangelogEntryUnpublished,
            (Self::ChangelogEntry, Change::Deleted) => Event::ChangelogEntryDeleted,
        }
    }
}

/// One entry as a delivery carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Entry {
    /// `<language>.<slug>`, as posts are named.
    id: String,
    language: String,
    slug: String,
    /// The title, the glossary term, or the changelog entry's version when
    /// it has no title.
    title: Option<String>,
    /// The file, relative to the site folder, with `/` between folders.
    file: String,
}

/// An entry and the hash of its file, as one render read it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    kind: Kind,
    entry: Entry,
    hash: ContentHash,
}

/// Every entry of one render, by file, and the entry files that have a
/// draft under `drafts/`.
#[derive(Debug, Clone, Default)]
pub(crate) struct Entries(BTreeMap<String, Seen>, BTreeSet<String>);

impl Entries {
    /// The entries of `content`. A file whose name is not valid UTF-8 is
    /// left out.
    pub(crate) fn of(content: &SiteContent) -> Self {
        let pages = content.pages.iter().map(|doc| {
            let front = &doc.front;
            seen(Kind::Page, doc, front.slug.as_ref(), front.title.clone())
        });
        let terms = content.glossary.iter().map(|doc| {
            let front = &doc.front;
            seen(
                Kind::GlossaryTerm,
                doc,
                front.slug.as_ref(),
                front.term.clone(),
            )
        });
        let studies = content.case_studies.iter().map(|doc| {
            let front = &doc.front;
            seen(
                Kind::CaseStudy,
                doc,
                front.slug.as_ref(),
                front.title.clone(),
            )
        });
        let changelog = content.changelog.iter().map(|doc| {
            let title = doc
                .front
                .title
                .clone()
                .or_else(|| doc.front.version.clone());
            seen(Kind::ChangelogEntry, doc, None, title)
        });
        let all = pages.chain(terms).chain(studies).chain(changelog);
        Self(all.flatten().collect(), BTreeSet::new())
    }

    /// These entries, knowing which entry files have a draft in the site
    /// `folder`: the entry at `content/<language>/<kind>/<name>.md` has one
    /// when `drafts/<language>/<kind>/<name>.md` is there (ADR 0030).
    /// Hidden files and folders that cannot be read are left out. Reads
    /// folders, so it runs where the content is read.
    pub(crate) fn with_drafts(mut self, folder: &Path) -> Self {
        let mut drafts = folder.to_path_buf();
        drafts.push("drafts");
        let Ok(languages) = fs::read_dir(&drafts) else {
            return self;
        };
        for language in languages.filter_map(Result::ok) {
            let Ok(name) = language.file_name().into_string() else {
                continue;
            };
            for kind in KIND_FOLDERS {
                let mut path = language.path();
                path.push(kind);
                let Ok(files) = fs::read_dir(&path) else {
                    continue;
                };
                let names = files.filter_map(|file| file.ok()?.file_name().into_string().ok());
                let kept = names.filter(|file| is_markdown(file) && !file.starts_with('.'));
                self.1
                    .extend(kept.map(|file| format!("content/{name}/{kind}/{file}")));
            }
        }
        self
    }

    /// Fires on `announce` the event of each entry that is new, changed or
    /// gone in `self` since `before`.
    pub(crate) fn fire_since(&self, before: &Self, announce: &SiteAnnounce) {
        for (event, kind, entry) in self.changes_since(before) {
            announce.fire(event, BTreeMap::from([(kind.key(), entry.clone())]));
        }
    }

    /// Each entry that is new, changed or gone in `self` since `before`,
    /// with its event: a gone entry as it was last, unpublished when
    /// `self` saw a draft of it and deleted when not.
    fn changes_since<'a>(&'a self, before: &'a Self) -> Vec<(Event, Kind, &'a Entry)> {
        let now = self.0.iter().filter_map(|(file, seen)| {
            let change = match before.0.get(file) {
                None => Change::Published,
                Some(was) if was.hash != seen.hash => Change::Edited,
                Some(_) => return None,
            };
            Some((seen.kind.event(change), seen.kind, &seen.entry))
        });
        let gone = before
            .0
            .iter()
            .filter(|(file, _)| !self.0.contains_key(*file))
            .map(|(file, was)| {
                let change = if self.1.contains(file) {
                    Change::Unpublished
                } else {
                    Change::Deleted
                };
                (was.kind.event(change), was.kind, &was.entry)
            });
        now.chain(gone).collect()
    }
}

/// Whether the file `name` ends in `.md`, in any case, as the content tree
/// reads them.
fn is_markdown(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// `doc` as an entry of `kind`, by its file. The slug is `slug` when the
/// front matter names one, else the file's name.
fn seen<F>(
    kind: Kind,
    doc: &Document<F>,
    slug: Option<&Slug>,
    title: Option<String>,
) -> Option<(String, Seen)> {
    let file = index::slashed(&doc.path)?;
    let slug = match slug {
        Some(slug) => slug.to_string(),
        None => doc.path.file_stem()?.to_str()?.to_owned(),
    };
    let language = doc.locale.as_str().to_owned();
    let entry = Entry {
        id: format!("{language}.{slug}"),
        language,
        slug,
        title,
        file: file.clone(),
    };
    let hash = doc.hash;
    Some((file, Seen { kind, entry, hash }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use khalid_content::{ContentTree, read_content};

    use super::*;

    fn demo() -> Entries {
        let mut folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        folder.push("../../fixtures/demo-site");
        let content = read_content(&ContentTree::walk(&folder).unwrap()).unwrap();
        Entries::of(&content)
    }

    fn names(changes: &[(Event, Kind, &Entry)]) -> Vec<String> {
        let named = changes
            .iter()
            .map(|(event, _, entry)| format!("{event} {}", entry.id));
        named.collect()
    }

    #[test]
    fn the_demo_entries_are_read_with_their_names() {
        let entries = demo();
        let about = &entries.0["content/en/pages/about.md"];
        assert_eq!(about.kind, Kind::Page);
        assert_eq!(about.entry.id, "en.about");
        let changelog = &entries.0["content/en/changelog/2026-10.md"];
        assert_eq!(changelog.entry.slug, "2026-10");
        assert!(changelog.entry.title.is_some(), "{changelog:?}");
        assert!(
            !entries.0.keys().any(|file| file.contains("/posts/")),
            "posts fire through the publish door"
        );
    }

    #[test]
    fn a_render_with_nothing_changed_fires_nothing() {
        assert_eq!(names(&demo().changes_since(&demo())), Vec::<String>::new());
    }

    #[test]
    fn new_changed_and_gone_entries_fire_their_events() {
        let before = demo();
        let mut now = demo();
        let about = now.0.remove("content/en/pages/about.md").unwrap();
        let mut term = now
            .0
            .remove("content/en/glossary/topical-authority.md")
            .unwrap();
        term.hash = about.hash;
        now.0
            .insert("content/en/glossary/topical-authority.md".to_owned(), term);
        let mut study = before.0["content/en/case-studies/northfield-bakery.md"].clone();
        study.entry.id = "en.harbour-cafe".to_owned();
        now.0
            .insert("content/en/case-studies/harbour-cafe.md".to_owned(), study);

        assert_eq!(
            names(&now.changes_since(&before)),
            [
                "case-study.published en.harbour-cafe",
                "glossary-term.published.edited en.topical-authority",
                "page.deleted en.about",
            ]
        );
    }

    #[test]
    fn an_entry_moved_to_drafts_is_unpublished_and_one_removed_is_deleted() {
        let site = tempfile::tempdir().unwrap();
        for draft in [
            "drafts/en/pages/about.md",
            "drafts/en/glossary/.topical-authority.md",
            "drafts/en/case-studies/notes.txt",
            "drafts/en/a-post-draft.md",
        ] {
            let mut file = site.path().to_path_buf();
            file.push(draft);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, "---\ntitle: Kept back\n---\n").unwrap();
        }
        let before = demo();
        let mut now = demo().with_drafts(site.path());
        now.0.remove("content/en/pages/about.md").unwrap();
        now.0
            .remove("content/en/glossary/topical-authority.md")
            .unwrap();

        assert_eq!(
            names(&now.changes_since(&before)),
            [
                "glossary-term.deleted en.topical-authority",
                "page.unpublished en.about",
            ]
        );
    }
}
