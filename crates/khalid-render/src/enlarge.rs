//! Click to enlarge: a body image shown smaller than its original links to
//! its largest version.

use std::fmt::Write as _;

use minijinja::value::Value;
use minijinja::{Error, ErrorKind};

use crate::escape;

/// How every body image starts, up to its address.
const IMAGE: &str = "<img src=\"";

/// The `enlarge(originals)` filter: `body` with each image whose address is
/// a key of `originals` wrapped in a link to the address that key maps to.
///
/// The wiring lists only images that are drawn smaller than their largest
/// version and are not already inside a link, so no link is ever nested.
/// Keys are addresses as the sanitised body writes them; links are escaped
/// here.
///
/// # Errors
///
/// An [`ErrorKind::InvalidOperation`] error when `body` is not text.
pub(crate) fn enlarge(body: &Value, originals: &Value) -> Result<Value, Error> {
    let html = body
        .as_str()
        .ok_or_else(|| Error::new(ErrorKind::InvalidOperation, "enlarge needs the body's HTML"))?;
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find(IMAGE) {
        let (before, image) = rest.split_at(at);
        out.push_str(before);
        let after = image.strip_prefix(IMAGE).unwrap_or(image);
        let (Some((src, _)), Some((tag, tail))) = (after.split_once('"'), image.split_once('>'))
        else {
            rest = image;
            break;
        };
        match originals
            .get_item(&Value::from(src))
            .ok()
            .filter(|v| !v.is_undefined())
        {
            Some(largest) => {
                let href = escape::attribute(&largest.to_string());
                let _ = write!(out, "<a class=\"enlarge\" href=\"{href}\">{tag}></a>");
            }
            None => {
                let _ = write!(out, "{tag}>");
            }
        }
        rest = tail;
    }
    out.push_str(rest);
    Ok(Value::from_safe_string(out))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn enlarged(html: &str) -> String {
        let originals = BTreeMap::from([(
            "sitemap.png".to_owned(),
            "/blog/media/3f2a9c1e-2400.png".to_owned(),
        )]);
        let body = Value::from_safe_string(html.to_owned());
        enlarge(&body, &Value::from(originals))
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn a_listed_image_links_to_its_largest_version() {
        assert_eq!(
            enlarged(
                "<figure>\n<img src=\"sitemap.png\" alt=\"A sitemap\" loading=\"lazy\" />\n</figure>"
            ),
            "<figure>\n<a class=\"enlarge\" href=\"/blog/media/3f2a9c1e-2400.png\"><img src=\"sitemap.png\" alt=\"A sitemap\" loading=\"lazy\" /></a>\n</figure>"
        );
    }

    #[test]
    fn an_image_not_listed_stays_as_it_is() {
        let html = "<p><img src=\"small.png\" alt=\"Small\" /> and <img src=\"sitemap.png\" alt=\"Big\" /></p>";
        assert_eq!(
            enlarged(html),
            "<p><img src=\"small.png\" alt=\"Small\" /> and <a class=\"enlarge\" href=\"/blog/media/3f2a9c1e-2400.png\"><img src=\"sitemap.png\" alt=\"Big\" /></a></p>"
        );
    }

    #[test]
    fn the_link_is_escaped() {
        let originals = BTreeMap::from([("a.png".to_owned(), "/x\"onclick=\"y".to_owned())]);
        let body = Value::from_safe_string("<img src=\"a.png\" alt=\"A\" />".to_owned());
        let got = enlarge(&body, &Value::from(originals)).unwrap();
        assert!(
            got.as_str()
                .unwrap()
                .contains("href=\"/x&quot;onclick=&quot;y\"")
        );
    }

    #[test]
    fn a_broken_image_tag_is_left_as_it_is() {
        let html = "<p>Text <img src=\"sitemap.png";
        assert_eq!(enlarged(html), html);
    }
}
