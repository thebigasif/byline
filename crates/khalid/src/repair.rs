//! The safe repairs to a post's markdown, block by block.
//!
//! A repair rewrites only the blocks it fixes and leaves every other byte of
//! the body as the writer wrote it, blank lines included.

use khalid_content::{Block, BlockKind, parse_blocks};

/// The rule a heading repair answers.
pub(crate) const HEADING_ORDER: &str = "heading-order";

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

/// `body` with its headings in order, and the changes, or `None` when
/// nothing changed or the body cannot be rewritten in place.
pub(crate) fn repaired_body(body: &str) -> Option<(String, Vec<Change>)> {
    let before = parse_blocks(body).ok()?;
    let mut after = before.clone();
    let changes = heading_order(&mut after);
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

    fn fixed(body: &str) -> String {
        repaired_body(body).map_or_else(|| body.to_owned(), |(text, _)| text)
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
        let (text, changes) = repaired_body("#### *Deep* ####\n").unwrap();
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
        assert!(repaired_body(body).is_none());
    }

    #[test]
    fn text_between_blocks_that_is_not_blank_stops_the_repair() {
        let block = |source: &str| parse_blocks(source).unwrap().remove(0);
        let (old, new) = (block("#### B"), block("### B"));
        assert!(splice("stray\n\n#### B\n", &[old], &[new]).is_none());
    }
}
