//! The block model: markdown to blocks to markdown, byte for byte, for every
//! standard block (RULES.md BL-FS-5, BL-TST-5, KH-CON-5).

use khalid_content::{Block, BlockKind, parse_blocks, print_blocks};
use proptest::prelude::{Just, Strategy, prop_assert_eq, prop_oneof, proptest};

/// Parses `markdown`, checks it is one block of `kind`, and that printing it
/// gives the same text back.
#[cfg(test)]
fn round_trip(markdown: &str, kind: &BlockKind) {
    let blocks = parse_blocks(markdown);
    assert_eq!(blocks.len(), 1, "{markdown:?} gave {blocks:?}");
    assert_eq!(&blocks[0].kind, kind, "{markdown:?}");
    assert_eq!(print_blocks(&blocks), markdown);
}

#[test]
fn a_heading_round_trips() {
    round_trip(
        "## Why internal links matter\n",
        &BlockKind::Heading { level: 2 },
    );
}

#[test]
fn a_paragraph_round_trips() {
    round_trip(
        "Internal links help readers find the rest of your site.\nThey help search engines too.\n",
        &BlockKind::Paragraph,
    );
}

#[test]
fn a_quote_round_trips() {
    round_trip(
        "> Link from new posts to old ones.\n> Then go back the other way.\n",
        &BlockKind::Quote,
    );
}

#[test]
fn lists_round_trip() {
    round_trip("- title tags\n- sitemaps\n", &BlockKind::List);
    round_trip("1. measure\n2. fix the biggest cause\n", &BlockKind::List);
    round_trip("- [x] add one link\n- [ ] add an image\n", &BlockKind::List);
}

#[test]
fn code_round_trips_with_its_language() {
    round_trip(
        "```rust\nlet links = 3;\n\nlet more = 4;\n```\n",
        &BlockKind::Code {
            language: Some("rust".to_owned()),
        },
    );
    round_trip("```\nplain\n```\n", &BlockKind::Code { language: None });
}

#[test]
fn a_divider_round_trips() {
    round_trip("---\n", &BlockKind::Divider);
}

#[test]
fn a_table_round_trips() {
    round_trip(
        "| Page | Links in |\n|---|---|\n| Home | 12 |\n",
        &BlockKind::Table,
    );
}

#[test]
fn a_footnote_round_trips_where_it_is_written() {
    let post = "A claim.[^1]\n\n[^1]: The source, with a link to [the study](https://example.com/study).\n\nMore text.\n";
    let blocks = parse_blocks(post);
    let kinds: Vec<&BlockKind> = blocks.iter().map(|block| &block.kind).collect();
    assert_eq!(
        kinds,
        [
            &BlockKind::Paragraph,
            &BlockKind::Footnote,
            &BlockKind::Paragraph
        ]
    );
    assert_eq!(print_blocks(&blocks), post);
}

#[test]
fn an_html_block_round_trips() {
    round_trip("<aside>\nA note.\n</aside>\n", &BlockKind::Html);
}

#[test]
fn a_whole_post_splits_into_blocks_in_order() {
    let post = "## Title tags\n\nThe title tag is the blue link.\n\n- Put the topic first.\n- Keep it short.\n\n---\n";
    let kinds: Vec<BlockKind> = parse_blocks(post)
        .into_iter()
        .map(|block: Block| block.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            BlockKind::Heading { level: 2 },
            BlockKind::Paragraph,
            BlockKind::List,
            BlockKind::Divider
        ]
    );
    assert_eq!(print_blocks(&parse_blocks(post)), post);
}

#[test]
fn a_list_with_blank_lines_inside_stays_one_block() {
    round_trip(
        "- first\n\n  still the first item\n\n- second\n",
        &BlockKind::List,
    );
}

#[test]
fn extra_blank_lines_print_in_canonical_form() {
    assert_eq!(
        print_blocks(&parse_blocks("One.\n\n\n\nTwo.")),
        "One.\n\nTwo.\n"
    );
}

/// One block of each standard kind, as canonical markdown.
#[cfg(test)]
fn any_block() -> impl Strategy<Value = String> {
    let words = "[a-z]{1,8}( [a-z]{1,8}){0,6}";
    prop_oneof![
        (1..=6_usize, words).prop_map(|(level, text)| format!("{} {text}", "#".repeat(level))),
        words.prop_map(|text| format!("A {text}.")),
        words.prop_map(|text| format!("> {text}")),
        (words, words).prop_map(|(a, b)| format!("- {a}\n- {b}")),
        words.prop_map(|text| format!("```rust\n{text}\n```")),
        Just("---".to_owned()),
        (words, words).prop_map(|(a, b)| format!("| {a} |\n|---|\n| {b} |")),
    ]
}

proptest! {
    #[test]
    fn any_canonical_document_round_trips(
        blocks in proptest::collection::vec(any_block(), 1..8).prop_filter(
            "two lists in a row are one list in markdown",
            |blocks| !blocks.windows(2).any(|pair| pair.iter().all(|b| b.starts_with("- "))),
        )
    ) {
        let markdown = format!("{}\n", blocks.join("\n\n"));
        let parsed = parse_blocks(&markdown);
        prop_assert_eq!(parsed.len(), blocks.len());
        prop_assert_eq!(print_blocks(&parsed), markdown);
    }
}
