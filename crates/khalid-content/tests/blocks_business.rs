//! Business cards: a business's name, address and how to reach it, which
//! give structured data (task 1.7, KH-CON-5).

use khalid_content::{
    BlockKind, BusinessBlock, StructuredBlock, parse_blocks, print_blocks, render,
};
use khalid_core::{Locale, SlugMode};

const CARD: &str = "::business{name=\"Field Notes Studio\" type=\"ProfessionalService\" street=\"12 Mill Lane\" locality=\"Leeds\" postcode=\"LS1 4AB\" country=\"GB\" phone=\"+44 113 496 0000\" email=\"hello@example.com\" href=\"https://www.example.com\"}\n";

#[cfg(test)]
fn studio() -> BusinessBlock {
    BusinessBlock {
        name: "Field Notes Studio".to_owned(),
        business_type: "ProfessionalService".to_owned(),
        street: "12 Mill Lane".to_owned(),
        locality: "Leeds".to_owned(),
        postal_code: "LS1 4AB".to_owned(),
        country: "GB".to_owned(),
        phone: Some("+44 113 496 0000".to_owned()),
        email: Some("hello@example.com".to_owned()),
        website: Some("https://www.example.com".to_owned()),
    }
}

#[test]
fn a_business_card_round_trips() {
    let blocks = parse_blocks(CARD).unwrap();
    assert_eq!(print_blocks(&blocks), CARD);
    assert_eq!(blocks[0].kind, BlockKind::Business(studio()));
}

#[test]
fn renders_a_business_card_and_its_structured_data() {
    let page = render(CARD, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap();
    assert_eq!(page.structured, [StructuredBlock::Business(studio())]);
    insta::assert_snapshot!(page.html);
}

#[test]
fn the_type_is_local_business_unless_written() {
    let blocks = parse_blocks(
        "::business{name=\"A shop\" street=\"1 High St\" locality=\"York\" postcode=\"YO1 1AA\" country=\"GB\"}\n",
    )
    .unwrap();
    let BlockKind::Business(card) = &blocks[0].kind else {
        panic!("{blocks:?}");
    };
    assert_eq!(card.business_type, "LocalBusiness");
}

#[test]
fn a_country_is_a_two_letter_code() {
    assert_eq!(
        parse_blocks(
            "::business{name=\"A shop\" street=\"1 High St\" locality=\"York\" postcode=\"YO1 1AA\" country=\"England\"}\n"
        )
        .unwrap_err()
        .to_string(),
        "line 1: a business card's country is a two-letter code in capitals, such as GB or DE"
    );
}
