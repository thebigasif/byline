//! What the linter reads from a post's body besides its parsed parts: the
//! wikilinks as written, the plain words of the rendered HTML, and the
//! lists and tables under each heading. Kept apart from `lint_input.rs` so
//! that file stays under 600 lines (BL-SIZE-1).

use khalid_lint::SectionShape;

/// The target of each `[[target]]` and `[[target|text]]` in `body`, as
/// written, in order. Code, fenced or inline, holds none.
pub(crate) fn wikilinks(body: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut fence: Option<&str> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        match (fence, marker) {
            (None, Some(marker)) => fence = Some(marker),
            (Some(open), Some(marker)) if open == marker => fence = None,
            (None, None) => {
                // Text between backticks is inline code: every other piece.
                for prose in line.split('`').step_by(2) {
                    let mut rest = prose;
                    while let Some((_, after)) = rest.split_once("[[") {
                        let Some((inside, tail)) = after.split_once("]]") else {
                            break;
                        };
                        let target = inside.split('|').next().unwrap_or_default().trim();
                        if !target.is_empty() {
                            targets.push(target.to_owned());
                        }
                        rest = tail;
                    }
                }
            }
            _ => {}
        }
    }
    targets
}

/// The elements whose end also ends a line of text, such as a heading or a
/// list item written without a full stop.
const BLOCKS: &[&str] = &[
    "p",
    "li",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "td",
    "th",
    "dt",
    "dd",
    "blockquote",
    "figcaption",
    "pre",
];

/// Whether the tag written `tag`, without its `<` and `>`, ends a line:
/// the end of a block, or a `br`.
fn ends_line(tag: &str) -> bool {
    let (closing, rest) = match tag.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, tag),
    };
    let name = rest.split([' ', '/']).next().unwrap_or_default();
    name.eq_ignore_ascii_case("br") || (closing && BLOCKS.contains(&name))
}

/// The words of rendered HTML with every tag and every entity, such as
/// `&quot;`, replaced by a space, so neither is read as a word. The end of
/// a block becomes a line break, so a heading never runs into the next
/// sentence. A formula's `MathML` is left out, so its letters and signs are
/// never read as words; a maths block's caption stays.
pub(crate) fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut tag = String::new();
    let mut skip_until = None;
    let mut in_maths = false;
    for c in html.chars() {
        match (skip_until, c) {
            (Some(end), _) if c == end => {
                skip_until = None;
                if end == '>' && tag == "/math" {
                    in_maths = false;
                } else if end == '>' && tag.split(' ').next() == Some("math") {
                    in_maths = true;
                }
                text.push(if end == '>' && ends_line(&tag) {
                    '\n'
                } else {
                    ' '
                });
                tag.clear();
            }
            (Some('>'), _) => tag.push(c),
            (Some(_), _) => {}
            (None, '<') => skip_until = Some('>'),
            (None, '&') => skip_until = Some(';'),
            (None, _) if in_maths => {}
            (None, _) => text.push(c),
        }
    }
    text
}

/// The lists and tables under each heading of `html`, in heading order:
/// what sits between one `<h1>` to `<h6>` and the next. The plain text
/// the rules read has lost the bullets and pipes, so W6 reads these.
pub(crate) fn section_shapes(html: &str) -> Vec<Vec<SectionShape>> {
    let mut sections = Vec::new();
    let mut rest = html.get(next_heading(html).unwrap_or(html.len())..);
    while let Some(section) = rest.and_then(|from| from.get(1..)) {
        let end = next_heading(section).unwrap_or(section.len());
        let inside = section.get(..end).unwrap_or_default();
        let mut holds = Vec::new();
        if inside.contains("<ul") || inside.contains("<ol") {
            holds.push(SectionShape::List);
        }
        if inside.contains("<table") {
            holds.push(SectionShape::Table);
        }
        sections.push(holds);
        rest = section.get(end..).filter(|left| !left.is_empty());
    }
    sections
}

/// Where the next heading tag of `html` starts.
fn next_heading(html: &str) -> Option<usize> {
    ["<h1", "<h2", "<h3", "<h4", "<h5", "<h6"]
        .iter()
        .filter_map(|tag| html.find(tag))
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_heading_holds_the_lists_and_tables_before_the_next() {
        let html = "<p>Intro</p><ul><li>not under a heading</li></ul>\
                    <h2>Types</h2><ul><li>a</li></ul><h3>Versus</h3><p>none</p>\
                    <h2>Both</h2><ol><li>a</li></ol><table><tr><td>b</td></tr></table>";
        assert_eq!(
            section_shapes(html),
            [
                vec![SectionShape::List],
                vec![],
                vec![SectionShape::List, SectionShape::Table],
            ]
        );
    }

    #[test]
    fn plain_text_drops_tags_and_entities() {
        let html = "<p>Write a <em>&quot;title&quot;</em> tag &amp; test it.</p>";
        let text = plain_text(html);
        let words: Vec<&str> = text.split_whitespace().collect();
        assert_eq!(words, ["Write", "a", "title", "tag", "test", "it."]);
    }

    #[test]
    fn plain_text_ends_a_line_at_each_block_only() {
        let html = "<h2 id=\"why\">Why links</h2><ul><li>One <a href=\"/a/\">link</a></li><li>Two<br/>three</li></ul>";
        let lines: Vec<String> = plain_text(html)
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(lines, ["Why links", "One link", "Two", "three"]);
    }

    #[test]
    fn plain_text_leaves_out_formulas_and_keeps_their_captions() {
        let html = "<p>Growth <math><mi>x</mi></math> compounds.</p>\n<figure class=\"maths\">\n<math display=\"block\"><mi>A</mi><mo>=</mo><mi>P</mi></math>\n<figcaption>Compound growth</figcaption>\n</figure>\n";
        let words: Vec<String> = plain_text(html)
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        assert_eq!(words, ["Growth", "compounds.", "Compound", "growth"]);
    }

    #[test]
    fn wikilinks_are_read_as_written_outside_code() {
        let body = "See [[why-links]] and [[link-text|good link text]].\n\n\
                    Write `[[not-a-link]]` like this.\n\n```\n[[in-a-fence]]\n```\n\n\
                    ~~~\n```\n[[still-fenced]]\n~~~\n\nLast [[ seo ]] and [[]] and [[open";
        assert_eq!(wikilinks(body), ["why-links", "link-text", "seo"]);
    }
}
