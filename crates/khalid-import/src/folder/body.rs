//! A markdown body from Hugo, Jekyll, Astro or Eleventy as Khalid
//! markdown. The markdown itself is kept; template tags are not markdown,
//! so the common ones become Khalid blocks (ADR 0012) and the rest are
//! named and left out.

use std::io;

use khalid_content::BlockError;
use khalid_core::{AbsoluteUrl, Locale, SlugMode};

use crate::markdown::is_safe;
use crate::pieces::{Converted, Html, converted};

/// Why a body could not be made into Khalid markdown.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BodyError {
    /// HTML in the body could not be read.
    #[error("its body could not be read as HTML")]
    Html(#[from] io::Error),
    /// The body made a block Khalid cannot read.
    #[error("its body holds a block Khalid cannot read")]
    Markdown(#[from] BlockError),
}

/// A template tag on a line of its own: Hugo's `{{< name args >}}` or
/// `{{% name args %}}`, or Liquid's `{% name args %}`.
struct Tag<'a> {
    name: &'a str,
    args: &'a str,
}

/// The markdown of `body` for a site written in `locale` with slugs in
/// `slugs`. A `youtube`, `vimeo` or `figure` shortcode becomes a block, a
/// `highlight` pair becomes fenced code, and any other template tag, on
/// its own line or inside one, is left out and named in
/// [`Converted::left`].
pub(crate) fn folder_body(
    body: &str,
    locale: &Locale,
    slugs: SlugMode,
) -> Result<Converted, BodyError> {
    let mut html = Html::default();
    let mut text = String::new();
    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let Some(tag) = line_tag(line) else {
            text.push_str(&without_tags(line, &mut html));
            text.push('\n');
            continue;
        };
        if !text.trim().is_empty() {
            html.markdown(&text);
        }
        text.clear();
        match tag.name {
            "highlight" => {
                let code: Vec<&str> = lines
                    .by_ref()
                    .take_while(|line| {
                        !line_tag(line).is_some_and(|end| {
                            end.name.starts_with("end") || end.name == "/highlight"
                        })
                    })
                    .collect();
                let language = tag.args.split_whitespace().next().unwrap_or("");
                html.code(language, &code.join("\n"));
            }
            "youtube" | "vimeo" => video(&tag, &mut html),
            "figure" => figure(&tag, &mut html),
            // The end of a pair whose start was already named.
            end if end.starts_with('/') || end.starts_with("end") => {}
            other => html.leave(&format!("{other} tag")),
        }
    }
    if !text.trim().is_empty() {
        html.markdown(&text);
    }
    converted(html, locale, slugs)
}

/// The template tag a whole line holds, if it holds one.
fn line_tag(line: &str) -> Option<Tag<'_>> {
    let line = line.trim();
    let inner = [("{{<", ">}}"), ("{{%", "%}}"), ("{%", "%}")]
        .into_iter()
        .find_map(|(open, close)| line.strip_prefix(open)?.strip_suffix(close))?;
    let inner = inner
        .trim()
        .trim_start_matches('-')
        .trim_end_matches('-')
        .trim();
    let (name, args) = inner.split_once(char::is_whitespace).unwrap_or((inner, ""));
    Some(Tag {
        name,
        args: args.trim(),
    })
}

/// `line` with every template tag or output inside it taken out, each
/// named as left out.
fn without_tags(line: &str, html: &mut Html) -> String {
    let mut kept = String::new();
    let mut rest = line;
    while let Some(start) = rest.find("{{").into_iter().chain(rest.find("{%")).min() {
        let (before, from) = rest.split_at(start);
        let close = if from.starts_with("{%") { "%}" } else { "}}" };
        let Some((_, after)) = from.split_once(close) else {
            break;
        };
        kept.push_str(before);
        html.leave("template tag");
        rest = after;
    }
    kept.push_str(rest);
    kept
}

/// A named value in a tag's arguments, such as `src` in `src="rye.jpg"`.
fn value<'a>(args: &'a str, name: &str) -> Option<&'a str> {
    let (_, rest) = args.split_once(&format!("{name}="))?;
    match rest.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next(),
        None => rest.split_whitespace().next(),
    }
}

/// A `youtube` or `vimeo` shortcode as an `embed` block.
fn video(tag: &Tag<'_>, html: &mut Html) {
    let id = value(tag.args, "id")
        .or_else(|| tag.args.split_whitespace().next())
        .map(|id| id.trim_matches('"'))
        .filter(|id| {
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        });
    let page = match (tag.name, id) {
        ("youtube", Some(id)) => format!("https://www.youtube.com/watch?v={id}"),
        ("vimeo", Some(id)) => format!("https://vimeo.com/{id}"),
        _ => return html.leave(&format!("{} tag", tag.name)),
    };
    match AbsoluteUrl::parse(&page) {
        Ok(src) => {
            let title = value(tag.args, "title").unwrap_or(src.host()).to_owned();
            html.block("embed", &[("src", src.as_str()), ("title", &title)], None);
        }
        Err(_) => html.leave(&format!("{} tag", tag.name)),
    }
}

/// A `figure` shortcode as a `figure` block with its caption.
fn figure(tag: &Tag<'_>, html: &mut Html) {
    match value(tag.args, "src").filter(|src| is_safe(src)) {
        Some(src) => {
            let alt = value(tag.args, "alt").unwrap_or("");
            let caption = value(tag.args, "caption").unwrap_or("");
            html.block("figure", &[("src", src), ("alt", alt)], Some(caption));
        }
        None => html.leave("figure tag"),
    }
}

#[cfg(test)]
mod tests {
    use khalid_content::parse_blocks;

    use super::*;

    fn markdown(body: &str) -> Converted {
        let locale = Locale::try_new("en").unwrap();
        let converted = folder_body(body, &locale, SlugMode::Ascii).unwrap();
        parse_blocks(&converted.markdown).unwrap();
        converted
    }

    #[test]
    fn markdown_is_kept_and_shortcodes_become_blocks() {
        let body = markdown(concat!(
            "Rye needs **time**.\n\n",
            "{{< figure src=\"/images/rye.jpg\" alt=\"A rye loaf\" caption=\"Out of the oven\" >}}\n\n",
            "{{< youtube id=\"abc123\" title=\"Shaping rye\" >}}\n",
            "{% highlight python %}\nfeed(starter)\n{% endhighlight %}\n",
            "## Mixing\n",
        ));
        let want = concat!(
            "Rye needs **time**.\n\n",
            ":::figure{src=\"/images/rye.jpg\" alt=\"A rye loaf\"}\nOut of the oven\n:::\n\n",
            "::embed{src=\"https://www.youtube.com/watch?v=abc123\" title=\"Shaping rye\"}\n\n",
            "```python\nfeed(starter)\n```\n\n",
            "## Mixing\n",
        );
        assert_eq!(body.markdown, want);
        assert!(body.left.is_empty(), "{:?}", body.left);
    }

    #[test]
    fn other_template_tags_are_named_and_left_out() {
        let body = markdown(concat!(
            "{% include newsletter.html %}\n",
            "See {{< ref \"spring.md\" >}} and {{ site.title }}.\n",
            "{{< notice >}}\nKeep it warm.\n{{< /notice >}}\n",
            "{{< figure src=\"javascript:alert(1)\" >}}\n",
        ));
        assert_eq!(body.markdown, "See and .\n\nKeep it warm.\n");
        let left = ["include tag", "template tag", "notice tag", "figure tag"];
        assert_eq!(body.left, left);
    }
}
