//! Reading the `WordPress` export format, WXR: an RSS feed with its own
//! fields beside the standard ones.

use chrono::NaiveDateTime;
use khalid_core::AbsoluteUrl;
use roxmltree::{Document, Node};

use crate::wordpress::{Author, Export, Item, ItemKind, ItemStatus, Meta, Site, Term, WxrError};

/// The XML namespaces an export uses, matched by what their address starts
/// with, since WXR 1.0, 1.1 and 1.2 each carry their version in it.
#[derive(Debug, Clone, Copy)]
enum Ns {
    /// RSS's own elements, which have no namespace.
    Rss,
    /// The fields of `WordPress` itself, `wp:`.
    Wp,
    /// The hand-written summary, `excerpt:`.
    Excerpt,
    /// The body, `content:`.
    Content,
    /// The creator, `dc:`.
    Dc,
}

impl Ns {
    fn holds(self, uri: Option<&str>) -> bool {
        const WP: &str = "http://wordpress.org/export/";
        match (self, uri) {
            (Self::Rss, None) => true,
            (Self::Wp, Some(uri)) => uri.starts_with(WP) && !uri.ends_with("/excerpt/"),
            (Self::Excerpt, Some(uri)) => uri.starts_with(WP) && uri.ends_with("/excerpt/"),
            (Self::Content, Some(uri)) => uri == "http://purl.org/rss/1.0/modules/content/",
            (Self::Dc, Some(uri)) => uri == "http://purl.org/dc/elements/1.1/",
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
/// [`WxrError::BadValue`] with the line when an id, a date or an address is
/// not what `WordPress` writes.
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
        items: Vec::new(),
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
        } else if is(node, Ns::Rss, "item") {
            export.items.push(reader.item(node)?);
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

    fn item(&self, node: Node<'_, '_>) -> Result<Item, WxrError> {
        let id = child(node, Ns::Wp, "post_id").ok_or_else(|| self.bad(node, "wp:post_id"))?;
        let mut item = Item {
            id: self.number(id)?,
            kind: ItemKind::from_name(&text(node, Ns::Wp, "post_type")),
            status: ItemStatus::from_name(&text(node, Ns::Wp, "status")),
            title: text(node, Ns::Rss, "title"),
            name: text(node, Ns::Wp, "post_name"),
            link: self.maybe_address(child(node, Ns::Rss, "link"))?,
            creator: text(node, Ns::Dc, "creator"),
            published: child(node, Ns::Wp, "post_date")
                .map(|date| self.date(date))
                .transpose()?
                .flatten(),
            content: text(node, Ns::Content, "encoded"),
            excerpt: text(node, Ns::Excerpt, "encoded"),
            sticky: text(node, Ns::Wp, "is_sticky") == "1",
            parent: child(node, Ns::Wp, "post_parent")
                .map(|parent| self.number(parent))
                .transpose()?
                .filter(|parent| *parent != 0),
            attachment_url: self.maybe_address(child(node, Ns::Wp, "attachment_url"))?,
            categories: Vec::new(),
            tags: Vec::new(),
            meta: Vec::new(),
        };
        for field in node.children().filter(Node::is_element) {
            if is(field, Ns::Rss, "category") {
                let slug = field.attribute("nicename").unwrap_or_default().to_owned();
                // Other taxonomies, such as post formats, have no Khalid
                // equivalent and stay behind.
                match field.attribute("domain") {
                    Some("category") => item.categories.push(slug),
                    Some("post_tag") => item.tags.push(slug),
                    _ => {}
                }
            } else if is(field, Ns::Wp, "postmeta") {
                item.meta.push(Meta {
                    key: text(field, Ns::Wp, "meta_key"),
                    value: text(field, Ns::Wp, "meta_value"),
                });
            }
        }
        Ok(item)
    }

    /// A date, or none for the zeros `WordPress` writes when there is none.
    fn date(&self, node: Node<'_, '_>) -> Result<Option<NaiveDateTime>, WxrError> {
        let value = own_text(node);
        if value.is_empty() || value.starts_with("0000-00-00") {
            return Ok(None);
        }
        NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S")
            .map(Some)
            .map_err(|_| self.bad(node, &qualified(node)))
    }

    fn maybe_address(&self, node: Option<Node<'_, '_>>) -> Result<Option<AbsoluteUrl>, WxrError> {
        match node {
            Some(node) if !own_text(node).is_empty() => self.address(node).map(Some),
            _ => Ok(None),
        }
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
