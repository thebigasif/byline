//! The safe repairs to a post's markdown, block by block.
//!
//! A repair rewrites only the blocks it fixes and leaves every other byte of
//! the body as the writer wrote it, blank lines included.

use khalid_content::{Block, BlockKind, parse_blocks, render};
use khalid_core::{Locale, SlugMode};

/// The rule a heading repair answers.
pub(crate) const HEADING_ORDER: &str = "heading-order";

/// The rule a description repair answers.
pub(crate) const MISSING_ALT: &str = "missing-alt";

/// The rule a secure address repair answers.
pub(crate) const INSECURE_INTERNAL_LINK: &str = "insecure-internal-link";

/// Every rule a repair answers.
pub(crate) const RULES: [&str; 3] = [HEADING_ORDER, MISSING_ALT, INSECURE_INTERNAL_LINK];

/// The scheme a link to this site should not use, and the one it should.
const INSECURE: &str = "http://";
const SECURE: &str = "https://";

/// One block a repair changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    /// The rule whose finding it answers.
    pub(crate) rule: &'static str,
    /// The heading or image it changed.
    pub(crate) subject: String,
    /// What changed, as one plain sentence.
    pub(crate) sentence: String,
}

/// `body` with the repairs for `rules`, and the changes, or `None` when
/// nothing changed or the body cannot be rewritten in place. `insecure`
/// holds the `http://` addresses to this site the linter found, as written.
/// The post's `locale` and the site's `slugs` are how its captions render.
pub(crate) fn repaired_body(
    body: &str,
    rules: &[&str],
    insecure: &[&str],
    locale: &Locale,
    slugs: SlugMode,
) -> Option<(String, Vec<Change>)> {
    let before = parse_blocks(body).ok()?;
    let mut after = before.clone();
    let mut changes = Vec::new();
    if rules.contains(&HEADING_ORDER) {
        changes.extend(heading_order(&mut after));
    }
    if rules.contains(&MISSING_ALT) {
        changes.extend(alt_from_caption(&mut after, locale, slugs));
    }
    if rules.contains(&INSECURE_INTERNAL_LINK) {
        changes.extend(secure_links(&mut after, insecure));
    }
    if changes.is_empty() {
        return None;
    }
    Some((splice(body, &before, &after)?, changes))
}

/// Moves each heading that skips a level up to one level below the heading
/// it belongs under, and the headings under it with it. A `#` heading is a
/// second main headline, which is the writer's call, so it stays.
fn heading_order(blocks: &mut [Block]) -> Vec<Change> {
    // The open headings as (level written, level after the repair). The
    // post's title is the level 1 every first heading follows.
    let mut open: Vec<(u8, u8)> = Vec::new();
    let mut changes = Vec::new();
    for block in blocks {
        let BlockKind::Heading { level } = block.kind else {
            continue;
        };
        if level == 1 {
            open.clear();
            continue;
        }
        while open.last().is_some_and(|&(written, _)| written >= level) {
            open.pop();
        }
        let fixed = open.last().map_or(1, |&(_, fixed)| fixed).saturating_add(1);
        open.push((level, fixed));
        if fixed == level {
            continue;
        }
        let Some((source, text)) = relevel(&block.source, fixed) else {
            continue;
        };
        block.source = source;
        block.kind = BlockKind::Heading { level: fixed };
        changes.push(Change {
            rule: HEADING_ORDER,
            sentence: format!(
                "\"{text}\" is now a level {fixed} heading, one below the heading before it."
            ),
            subject: text,
        });
    }
    changes
}

/// A `#` heading's markdown at `level`, and its text, or `None` for a
/// heading underlined with `---` or `===`, which only levels 1 and 2 use.
fn relevel(source: &str, level: u8) -> Option<(String, String)> {
    let line = source.trim_start_matches(' ');
    let text = line.strip_prefix('#')?.trim_start_matches('#');
    let words = text.trim().trim_end_matches('#').trim_end().to_owned();
    let marks = "#".repeat(usize::from(level));
    Some((format!("{marks}{text}"), words))
}

/// Gives each figure with no description its caption as plain text, so
/// people who cannot see the image hear what the caption says.
fn alt_from_caption(blocks: &mut [Block], locale: &Locale, slugs: SlugMode) -> Vec<Change> {
    let mut changes = Vec::new();
    for block in blocks {
        let BlockKind::Figure { src, alt } = &block.kind else {
            continue;
        };
        if !alt.trim().is_empty() {
            continue;
        }
        let lines: Vec<&str> = block.source.lines().collect();
        let Some((opening, rest)) = lines.split_first() else {
            continue;
        };
        let caption = rest
            .split_last()
            .map_or_else(String::new, |(_, inside)| inside.join("\n"));
        let Ok(rendered) = render(&caption, locale, slugs) else {
            continue;
        };
        // A directive value cannot hold `"`, `{` or `}`.
        let words = plain(&rendered.html)
            .replace('"', "'")
            .replace('{', "(")
            .replace('}', ")");
        if words.is_empty() {
            continue;
        }
        let colons = opening.len() - opening.trim_start_matches(':').len();
        let fence = ":".repeat(colons);
        let mut source = format!("{fence}figure{{src=\"{src}\" alt=\"{words}\"}}");
        for line in rest {
            source.push('\n');
            source.push_str(line);
        }
        let Ok([described]) = <[Block; 1]>::try_from(parse_blocks(&source).unwrap_or_default())
        else {
            continue;
        };
        changes.push(Change {
            rule: MISSING_ALT,
            subject: src.clone(),
            sentence: format!("The image {src} now has its caption as its description."),
        });
        *block = described;
    }
    changes
}

/// Moves each address in `targets` from `http://` to `https://` wherever it
/// is written whole, in every block but fenced code, where an address is an
/// example to copy as it stands. The host and path stay as written.
fn secure_links(blocks: &mut [Block], targets: &[&str]) -> Vec<Change> {
    let mut changes = Vec::new();
    for target in targets {
        let Some(rest) = target
            .get(..INSECURE.len())
            .filter(|scheme| scheme.eq_ignore_ascii_case(INSECURE))
            .and_then(|_| target.get(INSECURE.len()..))
        else {
            continue;
        };
        let secure = format!("{SECURE}{rest}");
        let mut moved = false;
        for block in blocks.iter_mut() {
            if matches!(block.kind, BlockKind::Code { .. }) {
                continue;
            }
            if let Some(source) = replaced_whole(&block.source, target, &secure) {
                block.source = source;
                moved = true;
            }
        }
        if moved {
            changes.push(Change {
                rule: INSECURE_INTERNAL_LINK,
                subject: (*target).to_owned(),
                sentence: format!("The link to {target} now opens {secure}, the secure address."),
            });
        }
    }
    changes
}

/// `source` with every whole `target` replaced by `new`, or `None` when it
/// holds none. Whole means the address starts after a space, a bracket or
/// a quote and ends before one, or before closing punctuation, so a longer
/// address that starts the same way, or one inside backticks, stays.
fn replaced_whole(source: &str, target: &str, new: &str) -> Option<String> {
    let starts = |before: &str| {
        before
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || "(<[\"'".contains(c))
    };
    let ends = |after: &str| {
        let mut next = after.chars();
        match next.next() {
            None => true,
            Some(c) if c.is_whitespace() || ")>]\"'".contains(c) => true,
            Some(c) if ".,;:!?".contains(c) => next.next().is_none_or(char::is_whitespace),
            Some(_) => false,
        }
    };
    let mut text = String::with_capacity(source.len());
    let mut rest = source;
    let mut found = false;
    while let Some(at) = rest.find(target) {
        let (before, tail) = rest.split_at_checked(at)?;
        let after = tail.get(target.len()..)?;
        text.push_str(before);
        let whole = starts(&text) && ends(after);
        text.push_str(if whole { new } else { target });
        found |= whole;
        rest = after;
    }
    text.push_str(rest);
    found.then_some(text)
}

/// The text of rendered HTML: tags dropped, the entities `render` writes
/// read back, and runs of spaces and lines made one space.
fn plain(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&#x27;", "'"),
        ("&amp;", "&"),
    ]
    .iter()
    .fold(text, |text, (entity, character)| {
        text.replace(entity, character)
    });
    let words: Vec<&str> = text.split_whitespace().collect();
    words.join(" ")
}

/// `body` with each block of `before` replaced by the same block of `after`,
/// found in order, so the blank lines between blocks stay as written. `None`
/// when anything but blank lines sits between two blocks.
fn splice(body: &str, before: &[Block], after: &[Block]) -> Option<String> {
    let mut text = String::with_capacity(body.len());
    let mut rest = body;
    for (old, new) in before.iter().zip(after) {
        let (gap, tail) = rest.split_at_checked(rest.find(old.source.as_str())?)?;
        if !gap.trim().is_empty() {
            return None;
        }
        text.push_str(gap);
        text.push_str(&new.source);
        rest = tail.get(old.source.len()..)?;
    }
    text.push_str(rest);
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `body` with every repair, in an English, ASCII site.
    fn all(body: &str) -> Option<(String, Vec<Change>)> {
        let english = Locale::try_new("en").unwrap();
        repaired_body(body, &RULES, &[], &english, SlugMode::Ascii)
    }

    /// `body` with only the secure address repair, for `targets`.
    fn secured(body: &str, targets: &[&str]) -> Option<(String, Vec<Change>)> {
        let english = Locale::try_new("en").unwrap();
        let rules = [INSECURE_INTERNAL_LINK];
        repaired_body(body, &rules, targets, &english, SlugMode::Ascii)
    }

    #[test]
    fn an_own_address_moves_to_https_with_its_host_as_written() {
        let target = "HTTP://www.Example.com/a/";
        let body = "See [a](HTTP://www.Example.com/a/), <HTTP://www.Example.com/a/>.\n";
        let (text, changes) = secured(body, &[target]).unwrap();
        assert_eq!(
            text,
            "See [a](https://www.Example.com/a/), <https://www.Example.com/a/>.\n"
        );
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].subject, target);
        assert_eq!(
            changes[0].sentence,
            "The link to HTTP://www.Example.com/a/ now opens https://www.Example.com/a/, the secure address."
        );
    }

    #[test]
    fn a_longer_address_code_and_code_spans_stay() {
        let target = "http://example.com/a";
        let body = "[b](http://example.com/ab) and `http://example.com/a`.\n\n```\n[a](http://example.com/a)\n```\n";
        assert!(secured(body, &[target]).is_none());
    }

    #[test]
    fn a_bare_address_before_a_full_stop_moves() {
        let body = "Read http://example.com/a. Then stop.\n";
        let (text, _) = secured(body, &["http://example.com/a"]).unwrap();
        assert_eq!(text, "Read https://example.com/a. Then stop.\n");
    }

    fn fixed(body: &str) -> String {
        all(body).map_or_else(|| body.to_owned(), |(text, _)| text)
    }

    #[test]
    fn a_skipped_level_moves_up_with_the_headings_under_it() {
        let body = "## A\n\n#### B\n\n##### C\n\n#### D\n\n### E\n";
        assert_eq!(fixed(body), "## A\n\n### B\n\n#### C\n\n### D\n\n### E\n");
    }

    #[test]
    fn a_second_main_headline_stays_and_starts_over() {
        assert_eq!(
            fixed("### Zero\n\n# One\n\n### Two\n"),
            "## Zero\n\n# One\n\n## Two\n"
        );
    }

    #[test]
    fn blank_lines_between_blocks_stay_as_written() {
        let body = "Intro.\n\n\n## A\n\n#### B\n";
        assert_eq!(fixed(body), "Intro.\n\n\n## A\n\n### B\n");
    }

    #[test]
    fn closing_marks_and_inline_markup_stay() {
        let (text, changes) = all("#### *Deep* ####\n").unwrap();
        assert_eq!(text, "## *Deep* ####\n");
        assert_eq!(changes[0].subject, "*Deep*");
        assert_eq!(
            changes[0].sentence,
            "\"*Deep*\" is now a level 2 heading, one below the heading before it."
        );
    }

    #[test]
    fn headings_in_order_and_headings_in_code_change_nothing() {
        let body = "## A\n\n### B\n\n```\n#### not a heading\n```\n";
        assert!(all(body).is_none());
    }

    #[test]
    fn text_between_blocks_that_is_not_blank_stops_the_repair() {
        let block = |source: &str| parse_blocks(source).unwrap().remove(0);
        let (old, new) = (block("#### B"), block("### B"));
        assert!(splice("stray\n\n#### B\n", &[old], &[new]).is_none());
    }

    #[test]
    fn a_figure_with_no_description_takes_its_caption_as_plain_text() {
        let body =
            ":::figure{src=\"/door.jpg\" alt=\" \"}\nA *blue* door & a [cat](/cats/).\n:::\n";
        let (text, changes) = all(body).unwrap();
        assert_eq!(
            text,
            ":::figure{src=\"/door.jpg\" alt=\"A blue door & a cat.\"}\nA *blue* door & a [cat](/cats/).\n:::\n"
        );
        assert_eq!(changes[0].rule, MISSING_ALT);
        assert_eq!(changes[0].subject, "/door.jpg");
    }

    #[test]
    fn quotes_and_braces_in_a_caption_become_safe_in_the_description() {
        let body = "::::figure{src=\"/a.png\" alt=\"\"}\nThe \"Dawn\" {draft} cover.\n::::\n";
        assert_eq!(
            fixed(body),
            "::::figure{src=\"/a.png\" alt=\"The 'Dawn' (draft) cover.\"}\nThe \"Dawn\" {draft} cover.\n::::\n"
        );
    }

    #[test]
    fn a_figure_with_a_description_or_no_caption_stays() {
        let described = ":::figure{src=\"/a.png\" alt=\"A cat\"}\nOur cat.\n:::\n";
        let uncaptioned = ":::figure{src=\"/a.png\" alt=\"\"}\n:::\n";
        assert!(all(described).is_none());
        assert!(all(uncaptioned).is_none());
    }

    #[test]
    fn only_the_rules_asked_for_are_repaired() {
        let english = Locale::try_new("en").unwrap();
        let body = "#### Deep\n\n:::figure{src=\"/a.png\" alt=\"\"}\nOur cat.\n:::\n";
        let repaired = repaired_body(body, &[MISSING_ALT], &[], &english, SlugMode::Ascii);
        let (text, _) = repaired.unwrap();
        assert!(text.starts_with("#### Deep\n"), "{text}");
        assert!(text.contains("alt=\"Our cat.\""), "{text}");
    }
}
