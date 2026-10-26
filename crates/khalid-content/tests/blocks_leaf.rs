//! Toggle, table of contents and button blocks, and the one-line leaf form
//! `::name{...}` for blocks with no body (task 1.7, KH-CON-5).

use khalid_content::{BlockKind, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

#[cfg(test)]
fn round_trip(markdown: &str, kind: &BlockKind) {
    let blocks = parse_blocks(markdown).unwrap();
    assert_eq!(blocks.len(), 1, "{markdown:?} gave {blocks:?}");
    assert_eq!(&blocks[0].kind, kind, "{markdown:?}");
    assert_eq!(print_blocks(&blocks), markdown);
}

#[cfg(test)]
fn html(markdown: &str) -> String {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii)
        .unwrap()
        .html
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

#[test]
fn a_toggle_round_trips() {
    round_trip(
        ":::toggle{summary=\"Why does this matter?\"}\nBecause readers skim.\n:::\n",
        &BlockKind::Toggle {
            summary: "Why does this matter?".to_owned(),
        },
    );
}

#[test]
fn a_table_of_contents_round_trips() {
    round_trip("::toc\n", &BlockKind::Toc);
}

#[test]
fn a_button_round_trips() {
    round_trip(
        "::button{href=\"/pricing/\" label=\"See prices\"}\n",
        &BlockKind::Button {
            href: "/pricing/".to_owned(),
            label: "See prices".to_owned(),
        },
    );
}

#[test]
fn renders_a_toggle_a_contents_list_and_a_button() {
    let markdown = "\
::toc

## Title tags

:::toggle{summary=\"What is a title tag?\"}
The **blue link** in search results.
:::

## Sitemaps

::button{href=\"/guides/sitemaps/\" label=\"Read the sitemap guide\"}
";
    insta::assert_snapshot!(html(markdown));
}

#[test]
fn a_button_refuses_a_script_link() {
    assert_eq!(
        error("::button{href=\"javascript:alert(1)\" label=\"Click\"}\n"),
        "line 1: a button links to a page, a #section, https:, http: or mailto:, not javascript:alert(1)"
    );
}

#[test]
fn a_button_needs_a_label() {
    assert_eq!(
        error("::button{href=\"/pricing/\"}\n"),
        "line 1: a button needs label, the words on the button"
    );
}

#[test]
fn a_toggle_needs_a_summary() {
    assert_eq!(
        error(":::toggle\nHidden text.\n:::\n"),
        "line 1: a toggle needs summary, the line a reader clicks to open it"
    );
}

#[test]
fn a_block_in_the_wrong_form_is_refused_with_the_right_one() {
    assert_eq!(
        error(":::toc\n:::\n"),
        "line 1: toc has no body; write it on one line as ::toc"
    );
    assert_eq!(
        error("::toggle{summary=\"Why?\"}\n"),
        "line 1: toggle has a body; open it with :::toggle and close it with :::"
    );
}
