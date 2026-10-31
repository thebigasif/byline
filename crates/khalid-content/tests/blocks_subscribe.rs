//! Newsletter and gated content blocks: subscribe forms that work with
//! scripts off (task 1.7, KH-CON-5), and gated content opened for a
//! confirmed subscriber (G41, task 5.11).

use khalid_content::{
    BlockKind, Gated, Wikilinks, parse_blocks, print_blocks, render, render_gated,
};
use khalid_core::{Locale, SlugMode};

const NEWSLETTER: &str =
    "::newsletter{label=\"Your email address\" button=\"Get the weekly links\"}\n";

const GATED: &str = "\
:::gated{note=\"Subscribers read the rest of this guide.\" label=\"Your email address\" button=\"Read the rest\"}
The **secret** checklist, with [a link](/guides/secret/).
:::
";

#[test]
fn a_newsletter_round_trips() {
    let blocks = parse_blocks(NEWSLETTER).unwrap();
    assert_eq!(print_blocks(&blocks), NEWSLETTER);
    assert_eq!(
        blocks[0].kind,
        BlockKind::Newsletter {
            label: "Your email address".to_owned(),
            button: "Get the weekly links".to_owned(),
        }
    );
}

#[test]
fn gated_content_round_trips() {
    let blocks = parse_blocks(GATED).unwrap();
    assert_eq!(print_blocks(&blocks), GATED);
    assert_eq!(
        blocks[0].kind,
        BlockKind::Gated {
            note: "Subscribers read the rest of this guide.".to_owned(),
            label: "Your email address".to_owned(),
            button: "Read the rest".to_owned(),
        }
    );
}

#[test]
fn renders_subscribe_forms_and_never_the_gated_body() {
    let page = render(
        &format!("{NEWSLETTER}\n{GATED}"),
        &Locale::try_new("en").unwrap(),
        SlugMode::Ascii,
    )
    .unwrap();
    assert!(!page.html.contains("secret"), "{}", page.html);
    assert_eq!(page.links, []);
    assert_eq!(page.words, 0);
    insta::assert_snapshot!(page.html);
}

#[test]
fn gated_content_needs_a_note() {
    assert_eq!(
        parse_blocks(":::gated{label=\"Email\" button=\"Read\"}\nHidden.\n:::\n")
            .unwrap_err()
            .to_string(),
        "line 1: gated content needs note, the line that tells readers how to read the rest"
    );
}

#[test]
fn a_locked_gated_form_carries_the_page_it_returns_to() {
    let page = render_gated(
        GATED,
        &Locale::try_new("en").unwrap(),
        SlugMode::Ascii,
        &Wikilinks::new(),
        Gated::Locked {
            back: Some("/guides/checklist/"),
        },
    )
    .unwrap();
    assert!(!page.html.contains("secret"), "{}", page.html);
    assert!(page.gated);
    assert!(
        page.html
            .contains("<input type=\"hidden\" name=\"back\" value=\"/guides/checklist/\" />"),
        "{}",
        page.html
    );
}

#[test]
fn an_open_gated_block_shows_its_body_and_counts_it() {
    let page = render_gated(
        GATED,
        &Locale::try_new("en").unwrap(),
        SlugMode::Ascii,
        &Wikilinks::new(),
        Gated::Open,
    )
    .unwrap();
    assert!(page.gated);
    assert!(!page.html.contains("<form"), "{}", page.html);
    assert_eq!(page.links.len(), 1);
    assert!(page.words >= 6, "{}", page.words);
    insta::assert_snapshot!(page.html);
}

#[test]
fn a_body_without_gated_content_says_so() {
    let page = render(NEWSLETTER, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap();
    assert!(!page.gated);
    assert!(!page.html.contains("name=\"back\""), "{}", page.html);
}
