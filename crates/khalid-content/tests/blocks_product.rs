//! Product cards: a picture, a price, a rating and a button, which give
//! structured data (task 1.7, KH-CON-5).

use khalid_content::{
    BlockKind, ProductBlock, StructuredBlock, parse_blocks, print_blocks, render,
};
use khalid_core::{Locale, SlugMode};

const PRODUCT: &str = "\
:::product{name=\"Site Audit Kit\" image=\"/media/audit-kit.png\" price=\"49.00\" currency=\"USD\" rating=\"4.5\" href=\"https://shop.example.com/audit-kit\" button=\"Buy the kit\"}
Checklists and **sheets** for a full site audit.
:::
";

#[cfg(test)]
fn kit(description: &str) -> ProductBlock {
    ProductBlock {
        name: "Site Audit Kit".to_owned(),
        image: Some("/media/audit-kit.png".to_owned()),
        price: Some("49.00".to_owned()),
        currency: Some("USD".to_owned()),
        rating: Some("4.5".to_owned()),
        best: "5".to_owned(),
        href: Some("https://shop.example.com/audit-kit".to_owned()),
        button: Some("Buy the kit".to_owned()),
        description: description.to_owned(),
    }
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

#[test]
fn a_product_round_trips() {
    let blocks = parse_blocks(PRODUCT).unwrap();
    assert_eq!(print_blocks(&blocks), PRODUCT);
    assert_eq!(
        blocks[0].kind,
        BlockKind::Product(kit("Checklists and **sheets** for a full site audit."))
    );
}

#[test]
fn renders_a_product_and_its_structured_data() {
    let page = render(PRODUCT, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap();
    assert_eq!(
        page.structured,
        [StructuredBlock::Product(kit(
            "Checklists and sheets for a full site audit."
        ))]
    );
    assert_eq!(page.links[0].href, "https://shop.example.com/audit-kit");
    insta::assert_snapshot!(page.html);
}

#[test]
fn a_price_needs_its_currency() {
    assert_eq!(
        error(":::product{name=\"Kit\" price=\"49\"}\nA kit.\n:::\n"),
        "line 1: a product's price is a number such as 49.00 with a three-letter currency such as USD"
    );
}

#[test]
fn a_link_needs_the_words_of_its_button() {
    assert_eq!(
        error(":::product{name=\"Kit\" href=\"/kit/\"}\nA kit.\n:::\n"),
        "line 1: a product with href needs button, the words on its button"
    );
}
