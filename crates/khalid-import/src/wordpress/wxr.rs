//! Reading the `WordPress` export format, WXR: an RSS feed with its own
//! fields beside the standard ones.

use khalid_core::AbsoluteUrl;
use roxmltree::{Document, Node};

use crate::wordpress::{Author, Export, Site, Term, WxrError};

/// The XML namespaces an export uses, matched by what their address starts
/// with, since WXR 1.0, 1.1 and 1.2 each carry their version in it.
#[derive(Debug, Clone, Copy)]
enum Ns {
    /// RSS's own elements, which have no namespace.
    Rss,
    /// The fields of `WordPress` itself, `wp:`.
    Wp,
}

impl Ns {
    fn holds(self, uri: Option<&str>) -> bool {
        const WP: &str = "http://wordpress.org/export/";
        match (self, uri) {
            (Self::Rss, None) => true,
            (Self::Wp, Some(uri)) => uri.starts_with(WP) && !uri.ends_with("/excerpt/"),
            _ => false,
        }
    }
}

/// Reads a `WordPress` export file into an [`Export`].
///
/// # Errors
///
/// Returns [`WxrError::NotXml`] when the text is not complete XML,
/// [`WxrError::NotWordPress`] when it is not a `WordPress` export, and
/// [`WxrError::BadValue`] with the line when an id or an address is not
/// what `WordPress` writes.
pub fn read_wxr(file: &str) -> Result<Export, WxrError> {
    let doc = Document::parse(file).map_err(|source| WxrError::NotXml { source })?;
    let root = doc.root_element();
    let channel = Some(root)
        .filter(|root| is(*root, Ns::Rss, "rss"))
        .and_then(|root| child(root, Ns::Rss, "channel"))
        .filter(|channel| child(*channel, Ns::Wp, "wxr_version").is_some())
        .ok_or(WxrError::NotWordPress)?;
    let reader = Reader { doc: &doc };
    let mut export = Export {
        site: reader.site(channel)?,
        authors: Vec::new(),
        categories: Vec::new(),
        tags: Vec::new(),
    };
    for node in channel.children().filter(Node::is_element) {
        if is(node, Ns::Wp, "author") {
            export.authors.push(author(node));
        } else if is(node, Ns::Wp, "category") {
            export
                .categories
                .push(reader.term(node, "category_nicename", "cat_name")?);
        } else if is(node, Ns::Wp, "tag") {
            export.tags.push(reader.term(node, "tag_slug", "tag_name")?);
        }
    }
    Ok(export)
}

/// Reads values that need the document to name a line in an error.
struct Reader<'doc, 'input> {
    doc: &'doc Document<'input>,
}

impl Reader<'_, '_> {
    fn site(&self, channel: Node<'_, '_>) -> Result<Site, WxrError> {
        let link = child(channel, Ns::Rss, "link").ok_or(WxrError::NotWordPress)?;
        Ok(Site {
            title: text(channel, Ns::Rss, "title"),
            address: self.address(link)?,
            language: Some(text(channel, Ns::Rss, "language")).filter(|tag| !tag.is_empty()),
        })
    }

    /// A category or tag, whose slug and name sit in the fields named.
    fn term(&self, node: Node<'_, '_>, slug: &str, name: &str) -> Result<Term, WxrError> {
        Ok(Term {
            id: child(node, Ns::Wp, "term_id")
                .map(|id| self.number(id))
                .transpose()?,
            slug: text(node, Ns::Wp, slug),
            name: text(node, Ns::Wp, name),
            parent: Some(text(node, Ns::Wp, "category_parent")).filter(|p| !p.is_empty()),
        })
    }

    fn number(&self, node: Node<'_, '_>) -> Result<u64, WxrError> {
        own_text(node)
            .parse()
            .map_err(|_| self.bad(node, &qualified(node)))
    }

    fn address(&self, node: Node<'_, '_>) -> Result<AbsoluteUrl, WxrError> {
        AbsoluteUrl::parse(&own_text(node)).map_err(|_| self.bad(node, &qualified(node)))
    }

    fn bad(&self, node: Node<'_, '_>, field: &str) -> WxrError {
        let row = self.doc.text_pos_at(node.range().start).row;
        WxrError::BadValue {
            line: usize::try_from(row).unwrap_or(usize::MAX),
            field: field.to_owned(),
            value: own_text(node),
        }
    }
}

fn author(node: Node<'_, '_>) -> Author {
    Author {
        login: text(node, Ns::Wp, "author_login"),
        display_name: text(node, Ns::Wp, "author_display_name"),
    }
}

fn is(node: Node<'_, '_>, ns: Ns, name: &str) -> bool {
    node.is_element() && node.tag_name().name() == name && ns.holds(node.tag_name().namespace())
}

fn child<'doc, 'input>(node: Node<'doc, 'input>, ns: Ns, name: &str) -> Option<Node<'doc, 'input>> {
    node.children().find(|child| is(*child, ns, name))
}

/// The text of the child called `name`, trimmed, or empty when there is none.
fn text(node: Node<'_, '_>, ns: Ns, name: &str) -> String {
    child(node, ns, name).map(own_text).unwrap_or_default()
}

fn own_text(node: Node<'_, '_>) -> String {
    node.text().unwrap_or_default().trim().to_owned()
}

/// The element's name with the `wp:` prefix, as a person sees it in the file.
fn qualified(node: Node<'_, '_>) -> String {
    let name = node.tag_name().name();
    if Ns::Wp.holds(node.tag_name().namespace()) {
        format!("wp:{name}")
    } else {
        name.to_owned()
    }
}
