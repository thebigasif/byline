//! The content index a rebuild writes, read from the site's content tree.

use std::path::{Component, Path};

use khalid_content::{ContentTree, Entry, parse_post};
use khalid_core::UserMessage;
use khalid_db::{ContentIndex, IndexedPost};

use crate::content_message;

/// The collection whose entries are posts. Other collections get their own
/// schemas and join the index with them.
const POSTS: &str = "posts";

/// Every post of every language in the `site` folder, ready for the database.
pub(crate) fn of(site: &Path) -> Result<ContentIndex, UserMessage> {
    let content = |error| content_message::of(site, &error);
    let tree = ContentTree::walk(site).map_err(content)?;
    let mut index = ContentIndex::default();
    for language in tree.languages() {
        let Some(collection) = language.collection(POSTS) else {
            continue;
        };
        for entry in collection.entries() {
            let front = parse_post(entry.path(), entry.text())
                .map_err(content)?
                .front;
            let at = format!("{}/{}", site.display(), entry.path().display());
            let path = slashed(entry.path()).ok_or_else(|| unreadable_name(&at))?;
            let slug = front.slug.ok_or_else(|| no_slug(&at))?;
            let mut post =
                IndexedPost::new(&path, language.locale().clone(), POSTS, slug, bytes(entry));
            post.title = front.title;
            post.description = front.description;
            post.date = front.date;
            post.author = front.author;
            post.category = front.category;
            post.translation = front.translation;
            index.posts.push(post);
        }
    }
    Ok(index)
}

/// The entry's path with `/` between folders, the form the index stores on
/// every system, or `None` when a name is not valid UTF-8.
pub(crate) fn slashed(path: &Path) -> Option<String> {
    let parts: Option<Vec<&str>> = path
        .components()
        .map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    parts.map(|parts| parts.join("/"))
}

/// The message for a post whose front matter has no slug.
fn no_slug(at: &str) -> UserMessage {
    UserMessage::new(
        format!("{at} has no slug."),
        "The slug is the last part of the post's address, so every post needs one.",
        format!("Add a `slug:` line to the front matter of {at}."),
    )
}

/// The message for a file name that is not valid UTF-8.
fn unreadable_name(at: &str) -> UserMessage {
    UserMessage::new(
        format!("The name of {at} has characters Khalid cannot read."),
        "Every file under content/ needs a name that is valid text to be indexed.",
        "Rename the file using letters, digits and hyphens.",
    )
}

/// The entry's content hash as bytes. `ContentHash` shows itself as 64
/// lowercase hex digits and has no byte accessor yet.
fn bytes(entry: &Entry) -> [u8; 32] {
    let hex = entry.hash().to_string();
    let mut bytes = [0; 32];
    for (byte, pair) in bytes.iter_mut().zip(hex.as_bytes().chunks(2)) {
        let digits = std::str::from_utf8(pair).unwrap_or_default();
        *byte = u8::from_str_radix(digits, 16).unwrap_or_default();
    }
    bytes
}
