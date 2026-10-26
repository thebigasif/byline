//! The Atom 1.0 writer for [`Feed`].

use khalid_core::AbsoluteUrl;

use crate::escape::escape_markup;
use crate::social::rfc3339;
use crate::{Feed, FeedFormat, SeoError};

fn element(out: &mut String, indent: &str, name: &str, text: &str) {
    out.push_str(indent);
    out.push('<');
    out.push_str(name);
    out.push('>');
    escape_markup(text, out);
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
}

fn link(out: &mut String, indent: &str, rel: &str, kind: &str, href: &AbsoluteUrl) {
    out.push_str(indent);
    out.push_str("<link rel=\"");
    out.push_str(rel);
    out.push_str("\" type=\"");
    out.push_str(kind);
    out.push_str("\" href=\"");
    escape_markup(href.as_str(), out);
    out.push_str("\"/>\n");
}

impl Feed {
    /// The feed as an Atom 1.0 document. The site is the feed's author, so
    /// an entry without an author of its own is still valid.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when the feed's own address cannot be built.
    pub fn to_atom(&self) -> Result<String, SeoError> {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\" xml:lang=\"",
        );
        escape_markup(self.language.as_str(), &mut out);
        out.push_str("\">\n");
        element(&mut out, "  ", "title", &self.title);
        if !self.description.is_empty() {
            element(&mut out, "  ", "subtitle", &self.description);
        }
        element(&mut out, "  ", "id", self.home.as_str());
        link(&mut out, "  ", "alternate", "text/html", &self.home);
        link(
            &mut out,
            "  ",
            "self",
            "application/atom+xml",
            &self.url(FeedFormat::Atom)?,
        );
        element(&mut out, "  ", "updated", &rfc3339(&self.updated));
        out.push_str("  <author>\n");
        element(&mut out, "    ", "name", &self.site_name);
        out.push_str("  </author>\n");
        for item in &self.items {
            out.push_str("  <entry>\n");
            element(&mut out, "    ", "title", &item.title);
            element(&mut out, "    ", "id", item.url.as_str());
            link(&mut out, "    ", "alternate", "text/html", &item.url);
            element(&mut out, "    ", "published", &rfc3339(&item.published));
            element(
                &mut out,
                "    ",
                "updated",
                &rfc3339(&item.updated.unwrap_or(item.published)),
            );
            if let Some(author) = &item.author {
                out.push_str("    <author>\n");
                element(&mut out, "      ", "name", author);
                out.push_str("    </author>\n");
            }
            if let Some(category) = &item.category {
                out.push_str("    <category term=\"");
                escape_markup(category, &mut out);
                out.push_str("\"/>\n");
            }
            if !item.summary.is_empty() {
                element(&mut out, "    ", "summary", &item.summary);
            }
            out.push_str("    <content type=\"html\">");
            escape_markup(&item.content_html, &mut out);
            out.push_str("</content>\n  </entry>\n");
        }
        out.push_str("</feed>\n");
        Ok(out)
    }
}
