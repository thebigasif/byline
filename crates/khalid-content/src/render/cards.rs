//! The HTML of the business card, and its structured data.

use super::{Link, Renderer, attribute};
use crate::{BusinessBlock, StructuredBlock};

impl Renderer {
    /// Writes a business card as an `<address>`: name, street, town and
    /// postcode, then phone, email and website as links.
    pub(super) fn business(&mut self, card: &BusinessBlock) {
        let mut lines = vec![
            ["<strong>", &attribute(&card.name), "</strong>"].concat(),
            attribute(&card.street),
            [attribute(&card.locality), attribute(&card.postal_code)].join(" "),
        ];
        if let Some(phone) = &card.phone {
            let dial: String = phone.chars().filter(|c| !c.is_whitespace()).collect();
            lines.push(link(&["tel:", &dial].concat(), phone));
        }
        if let Some(email) = &card.email {
            lines.push(link(&["mailto:", email].concat(), email));
        }
        if let Some(website) = &card.website {
            lines.push(link(website, website));
            self.facts.links.push(Link {
                href: website.clone(),
                text: website.clone(),
                internal: !website.contains(':'),
            });
        }
        self.push(&[
            "<address class=\"business\">\n",
            &lines.join("<br />\n"),
            "\n</address>\n",
        ]);
        self.facts
            .structured
            .push(StructuredBlock::Business(card.clone()));
    }
}

/// An `<a>` to `href` with `text`.
fn link(href: &str, text: &str) -> String {
    [
        "<a href=\"",
        &attribute(href),
        "\">",
        &attribute(text),
        "</a>",
    ]
    .concat()
}
