//! Newsletter and gated content blocks: subscribe forms that work with
//! scripts off (task 1.7, KH-CON-5).

use khalid_content::{BlockKind, parse_blocks, print_blocks, render};
use khalid_core::{Locale, Slug, SlugMode};

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
            list: None,
        }
    );
}

#[test]
fn a_newsletter_names_its_list_and_round_trips() {
    let text = "::newsletter{label=\"Email\" button=\"Join\" list=\"weekly\"}\n";
    let blocks = parse_blocks(text).unwrap();
    assert_eq!(print_blocks(&blocks), text);
    assert_eq!(
        blocks[0].kind,
        BlockKind::Newsletter {
            label: "Email".to_owned(),
            button: "Join".to_owned(),
            list: Some(Slug::try_new("weekly").unwrap()),
        }
    );
    let page = render(text, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap();
    assert!(
        page.html
            .contains("<input type=\"hidden\" name=\"list\" value=\"weekly\" />"),
        "{}",
        page.html
    );
}

#[test]
fn a_newsletter_list_must_be_a_slug() {
    assert_eq!(
        parse_blocks("::newsletter{label=\"Email\" button=\"Join\" list=\"Weekly News\"}\n")
            .unwrap_err()
            .to_string(),
        "line 1: list is the newsletter's name in lowercase letters, digits and hyphens, such as weekly"
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
