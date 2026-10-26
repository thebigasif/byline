//! The RSS 2.0 writer for [`Feed`].

use crate::escape::escape_markup;
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

impl Feed {
    /// The feed as an RSS 2.0 document, with the full content in
    /// `content:encoded`, the author in `dc:creator` and a self link.
    ///
    /// RSS needs a channel description, so the feed's title stands in when
    /// the site has no tagline.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when the feed's own address cannot be built.
    pub fn to_rss(&self) -> Result<String, SeoError> {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\" \
             xmlns:content=\"http://purl.org/rss/1.0/modules/content/\" \
             xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n  <channel>\n",
        );
        let description = if self.description.is_empty() {
            &self.title
        } else {
            &self.description
        };
        element(&mut out, "    ", "title", &self.title);
        element(&mut out, "    ", "link", self.home.as_str());
        element(&mut out, "    ", "description", description);
        element(&mut out, "    ", "language", self.language.as_str());
        element(
            &mut out,
            "    ",
            "lastBuildDate",
            &self.updated.to_rfc2822(),
        );
        out.push_str("    <atom:link href=\"");
        escape_markup(self.url(FeedFormat::Rss)?.as_str(), &mut out);
        out.push_str("\" rel=\"self\" type=\"application/rss+xml\"/>\n");
        for item in &self.items {
            out.push_str("    <item>\n");
            element(&mut out, "      ", "title", &item.title);
            element(&mut out, "      ", "link", item.url.as_str());
            out.push_str("      <guid isPermaLink=\"true\">");
            escape_markup(item.url.as_str(), &mut out);
            out.push_str("</guid>\n");
            element(&mut out, "      ", "pubDate", &item.published.to_rfc2822());
            if let Some(author) = &item.author {
                element(&mut out, "      ", "dc:creator", author);
            }
            if let Some(category) = &item.category {
                element(&mut out, "      ", "category", category);
            }
            if !item.summary.is_empty() {
                element(&mut out, "      ", "description", &item.summary);
            }
            element(&mut out, "      ", "content:encoded", &item.content_html);
            out.push_str("    </item>\n");
        }
        out.push_str("  </channel>\n</rss>\n");
        Ok(out)
    }
}
