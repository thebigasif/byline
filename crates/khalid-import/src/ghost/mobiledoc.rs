//! A Mobiledoc document, the editor format of Ghost 1 to 4, written as
//! HTML for the markdown converter. Mobiledoc 0.3 is what Ghost writes;
//! the markers of 0.2, which have no type, read too.

use serde_json::Value;

use crate::ghost::cards::card;
use crate::ghost::html::Html;

/// The markups the converter carries, by the tag Mobiledoc names, with
/// the element each one opens. Others keep their words and lose the
/// format, as in Lexical.
const MARKUPS: [(&str, &str); 6] = [
    ("strong", "strong"),
    ("b", "strong"),
    ("em", "em"),
    ("i", "em"),
    ("code", "code"),
    ("a", "a"),
];

/// The tags a text section may name, with the element each becomes. Any
/// other becomes a paragraph.
const SECTIONS: [(&str, &str); 9] = [
    ("h1", "h1"),
    ("h2", "h2"),
    ("h3", "h3"),
    ("h4", "h4"),
    ("h5", "h5"),
    ("h6", "h6"),
    ("blockquote", "blockquote"),
    ("aside", "blockquote"),
    ("pull-quote", "blockquote"),
];

/// The HTML of the Mobiledoc document `json`.
///
/// # Errors
///
/// Returns the JSON reader's error when `json` is not JSON.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the Ghost plan, a later part, converts bodies")
)]
pub(crate) fn mobiledoc_html(json: &str) -> Result<Html, serde_json::Error> {
    let document: Value = serde_json::from_str(json)?;
    let list = |name: &str| document.get(name).and_then(Value::as_array);
    let doc = Document {
        markups: list("markups").map_or(&[][..], Vec::as_slice),
        atoms: list("atoms").map_or(&[][..], Vec::as_slice),
        cards: list("cards").map_or(&[][..], Vec::as_slice),
    };
    let mut html = Html::default();
    for section in list("sections").into_iter().flatten() {
        doc.section(section, &mut html);
    }
    Ok(html)
}

/// The tables a document's sections point into, by index.
struct Document<'a> {
    markups: &'a [Value],
    atoms: &'a [Value],
    cards: &'a [Value],
}

impl Document<'_> {
    /// Writes one section.
    fn section(&self, section: &Value, html: &mut Html) {
        let part = |n: usize| section.get(n);
        match part(0).and_then(Value::as_u64) {
            Some(1) => {
                let tag = part(1).and_then(Value::as_str).unwrap_or_default();
                let element = SECTIONS
                    .iter()
                    .find(|(name, _)| tag.eq_ignore_ascii_case(name))
                    .map_or("p", |(_, element)| element);
                html.tag(&format!("<{element}>"));
                self.markers(part(2), html);
                html.tag(&format!("</{element}>"));
            }
            Some(2) => {
                let src = part(1).and_then(Value::as_str);
                card("image", &serde_json::json!({ "src": src }), html);
            }
            Some(3) => {
                let ordered = part(1).and_then(Value::as_str) == Some("ol");
                let list = if ordered { "ol" } else { "ul" };
                html.tag(&format!("<{list}>"));
                for item in part(2).and_then(Value::as_array).into_iter().flatten() {
                    html.tag("<li>");
                    self.markers(Some(item), html);
                    html.tag("</li>");
                }
                html.tag(&format!("</{list}>"));
            }
            Some(10) => {
                let entry = part(1)
                    .and_then(Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
                    .and_then(|n| self.cards.get(n));
                let name = entry.and_then(|c| c.get(0)).and_then(Value::as_str);
                let payload = entry.and_then(|c| c.get(1)).unwrap_or(&Value::Null);
                let name = name.unwrap_or("card");
                if !card(name, payload, html) {
                    html.leave(name);
                }
            }
            _ => html.leave("section"),
        }
    }

    /// Writes a section's markers: text inside the markups each one opens,
    /// closing as many as it says after it.
    fn markers(&self, markers: Option<&Value>, html: &mut Html) {
        let mut open: Vec<&str> = Vec::new();
        for marker in markers.and_then(Value::as_array).into_iter().flatten() {
            let Some(parts) = marker.as_array() else {
                continue;
            };
            // 0.3 starts each marker with its type; 0.2 has no type.
            let (kind, rest) = match parts.len() {
                3 => (0, parts.as_slice()),
                _ => match parts.split_first() {
                    Some((kind, rest)) => (kind.as_u64().unwrap_or(0), rest),
                    None => continue,
                },
            };
            let markups = rest.first().and_then(Value::as_array);
            for index in markups.into_iter().flatten() {
                open.push(self.open(index, html));
            }
            let value = rest.get(2);
            if kind == 1 {
                self.atom(value, html);
            } else {
                html.escaped(value.and_then(Value::as_str).unwrap_or_default());
            }
            let closed = rest.get(1).and_then(Value::as_u64).unwrap_or(0);
            for _ in 0..closed {
                match open.pop() {
                    Some("") | None => {}
                    Some(element) => html.tag(&format!("</{element}>")),
                }
            }
        }
        for element in open.iter().rev().filter(|element| !element.is_empty()) {
            html.tag(&format!("</{element}>"));
        }
    }

    /// Opens the markup at `index` and returns the element to close, or an
    /// empty name for a markup whose format is not carried.
    fn open(&self, index: &Value, html: &mut Html) -> &'static str {
        let markup = index
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .and_then(|n| self.markups.get(n));
        let tag = markup.and_then(|m| m.get(0)).and_then(Value::as_str);
        let Some(element) = tag.and_then(|tag| {
            MARKUPS
                .iter()
                .find(|(name, _)| tag.eq_ignore_ascii_case(name))
                .map(|(_, element)| *element)
        }) else {
            return "";
        };
        if element == "a" {
            // Attributes are a flat list of names and values.
            let attributes = markup.and_then(|m| m.get(1)).and_then(Value::as_array);
            let pairs: Vec<&Value> = attributes.into_iter().flatten().collect();
            let href = pairs
                .chunks(2)
                .find(|pair| pair.first().and_then(|name| name.as_str()) == Some("href"))
                .and_then(|pair| pair.get(1))
                .and_then(|value| value.as_str());
            html.tag("<a href=\"");
            html.escaped(href.unwrap_or_default());
            html.tag("\">");
        } else {
            html.tag(&format!("<{element}>"));
        }
        element
    }

    /// Writes an atom: a soft return is a line break; any other keeps its
    /// text and is named.
    fn atom(&self, index: Option<&Value>, html: &mut Html) {
        let atom = index
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .and_then(|n| self.atoms.get(n));
        let name = atom.and_then(|a| a.get(0)).and_then(Value::as_str);
        if name == Some("soft-return") {
            html.tag("<br>");
        } else {
            html.leave(name.unwrap_or("atom"));
            html.escaped(
                atom.and_then(|a| a.get(1))
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use khalid_core::{Locale, SlugMode};

    use super::*;
    use crate::ghost::body::converted;

    /// The markdown of a Mobiledoc document with these tables, and what it
    /// left out.
    fn markdown(markups: &str, atoms: &str, cards: &str, sections: &str) -> (String, Vec<String>) {
        let json = format!(
            r#"{{"version":"0.3.1","markups":[{markups}],"atoms":[{atoms}],"cards":[{cards}],"sections":[{sections}]}}"#
        );
        let locale = Locale::try_new("en").unwrap();
        let body = converted(mobiledoc_html(&json).unwrap(), &locale, SlugMode::Ascii).unwrap();
        (body.markdown, body.left)
    }

    #[test]
    fn sections_and_markups_become_markdown() {
        let (text, _) = markdown(
            r#"["strong"],["a",["rel","nofollow","href","https://northfield.example.com/rye/"]],["em"]"#,
            r#"["soft-return","",{}]"#,
            "",
            concat!(
                r#"[1,"h2",[[0,[],0,"Feeding"]]],"#,
                r#"[1,"p",[[0,[0],0,"Feed it "],[0,[1],1,"daily"],[0,[],1,"."],[1,[],0,0],[0,[2],1,"Or not."]]],"#,
                r#"[3,"ol",[[[0,[],0,"Mix"]],[[0,[],0,"Wait"]]]],"#,
                r#"[1,"aside",[[0,[],0,"Time <is> flour."]]]"#,
            ),
        );
        assert_eq!(
            text,
            "## Feeding\n\n**Feed it [daily](https://northfield.example.com/rye/).**  \n*Or not.*\n\n1.  Mix\n2.  Wait\n\n> Time \\<is> flour.\n"
        );
    }

    #[test]
    fn cards_and_image_sections_use_the_shared_cards() {
        let (text, left) = markdown(
            "",
            "",
            r#"["markdown",{"markdown":"Use **rye**."}],["code",{"code":"  knead()","language":"js"}],["signup",{}]"#,
            r#"[10,0],[10,1],[2,"https://northfield.example.com/a.jpg"],[10,2],[10,9]"#,
        );
        assert_eq!(
            text,
            "Use **rye**.\n\n```js\n  knead()\n```\n\n:::figure{src=\"https://northfield.example.com/a.jpg\" alt=\"\"}\n:::\n"
        );
        assert_eq!(left, ["signup", "card"]);
        khalid_content::parse_blocks(&text).unwrap();
    }

    #[test]
    fn version_two_markers_and_unknown_markups_and_atoms_still_read() {
        let (text, left) = markdown(
            r#"["u"],["code"]"#,
            r#"["mention","@mara",{}]"#,
            "",
            r#"[1,"p",[[[0],1,"under"],[[1],1,"code"]]],[1,"p",[[1,[],0,0]]],[7,"x"]"#,
        );
        assert_eq!(text, "under`code`\n\n@mara\n");
        assert_eq!(left, ["mention", "section"]);
        assert!(mobiledoc_html("{").is_err());
    }
}
