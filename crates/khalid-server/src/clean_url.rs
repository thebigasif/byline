//! Which page an address asks for, with one clean address per page.

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

use crate::{Page, PagePath, Pages};

/// The characters encoded when a decoded path goes back into a `Location`
/// header: controls, the space, characters that end a path, and everything
/// outside ASCII, which the encoder always encodes.
const PATH: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// What an address leads to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Resolved<'p> {
    /// The page at that address.
    Page(&'p Page),
    /// A page lives at the clean form of the address, given here encoded
    /// and with the query kept.
    Redirect(String),
    /// The address moved for good to this one, encoded, with the query kept.
    Moved(String),
    /// Nothing lives there.
    NotFound,
}

/// Finds the page for `path` and `query` as they arrived in the request.
///
/// `/blog/post` and `/blog/post/index.html` redirect to `/blog/post/` when
/// that page exists, so each page has one address search engines index.
/// Redirects only ever point at an existing page, whose path cannot begin
/// with `//`, so they never leave the site.
pub(crate) fn resolve<'p>(pages: &'p Pages, path: &str, query: Option<&str>) -> Resolved<'p> {
    let Ok(decoded) = percent_decode_str(path).decode_utf8() else {
        return Resolved::NotFound;
    };
    if let Some(page) = pages.get(&decoded) {
        return Resolved::Page(page);
    }
    if let Some(to) = pages.moved(&decoded) {
        return Resolved::Moved(location(to, query));
    }
    let clean = match decoded.strip_suffix("index.html") {
        Some(folder) if folder.ends_with('/') => folder.to_owned(),
        Some(_) => return Resolved::NotFound,
        None if decoded.ends_with('/') => return Resolved::NotFound,
        None => format!("{decoded}/"),
    };
    let Some(page) = pages.get(&clean) else {
        return Resolved::NotFound;
    };
    Resolved::Redirect(location(page.path(), query))
}

/// `path` encoded for a `Location` header, with `query` kept.
fn location(path: &PagePath, query: Option<&str>) -> String {
    let mut location = utf8_percent_encode(path.as_str(), PATH).to_string();
    if let Some(query) = query {
        location.push('?');
        location.push_str(query);
    }
    location
}
