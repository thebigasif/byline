//! A link on every H2 and H3 of a post's body, so a reader can share the
//! address of one section.

use std::fmt::Write as _;

use minijinja::value::Value;
use minijinja::{Error, ErrorKind};

use crate::escape;

/// The headings that get a link: how each opens, up to its `id`, and how it
/// closes.
const HEADINGS: [(&str, &str); 2] = [("<h2 id=\"", "</h2>"), ("<h3 id=\"", "</h3>")];

/// The `heading_links(label)` filter: `body` with a `#` link at the end of
/// each `h2` and `h3` that has an `id`, pointing at that heading and named
/// `label` for screen readers.
///
/// The body is HTML the content crate wrote and sanitised, where every
/// heading id is a slug, so the id needs no escaping. The label is escaped.
///
/// # Errors
///
/// An [`ErrorKind::InvalidOperation`] error when `body` is not text.
pub(crate) fn heading_links(body: &Value, label: &str) -> Result<Value, Error> {
    let html = body.as_str().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidOperation,
            "heading_links needs the body's HTML",
        )
    })?;
    let label = escape::attribute(label);
    let mut out = String::with_capacity(html.len().saturating_add(html.len() / 8));
    let mut rest = html;
    while let Some((at, (open, close))) = HEADINGS
        .iter()
        .filter_map(|&tags| rest.find(tags.0).map(|at| (at, tags)))
        .min()
    {
        let (before, heading) = rest.split_at(at);
        out.push_str(before);
        let after = heading.strip_prefix(open).unwrap_or(heading);
        let (Some((id, _)), Some((inside, tail))) =
            (after.split_once('"'), after.split_once(close))
        else {
            rest = heading;
            break;
        };
        let _ = write!(
            out,
            "{open}{inside} <a class=\"heading-link\" href=\"#{id}\" aria-label=\"{label}\">#</a>{close}"
        );
        rest = tail;
    }
    out.push_str(rest);
    Ok(Value::from_safe_string(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linked(html: &str) -> String {
        let body = Value::from_safe_string(html.to_owned());
        heading_links(&body, "Link to this section")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn each_h2_and_h3_with_an_id_links_to_itself() {
        assert_eq!(
            linked("<p>Hi.</p>\n<h2 id=\"why\">Why</h2>\n<h3 id=\"how\">How <em>now</em></h3>"),
            "<p>Hi.</p>\n<h2 id=\"why\">Why <a class=\"heading-link\" href=\"#why\" aria-label=\"Link to this section\">#</a></h2>\n\
             <h3 id=\"how\">How <em>now</em> <a class=\"heading-link\" href=\"#how\" aria-label=\"Link to this section\">#</a></h3>"
        );
    }

    #[test]
    fn other_headings_and_headings_without_an_id_stay_as_they_are() {
        let html = "<h2>Plain</h2><h4 id=\"deep\">Deep</h4><header>Top</header>";
        assert_eq!(linked(html), html);
    }

    #[test]
    fn a_heading_that_never_closes_is_left_as_it_is() {
        let html = "<h2 id=\"open\">Open";
        assert_eq!(linked(html), html);
    }

    #[test]
    fn the_label_is_escaped() {
        let body = Value::from_safe_string("<h2 id=\"a\">A</h2>".to_owned());
        let got = heading_links(&body, "Link \"here\" & <now>").unwrap();
        assert!(
            got.as_str()
                .unwrap()
                .contains("aria-label=\"Link &quot;here&quot; &amp; &lt;now&gt;\"")
        );
    }
}
