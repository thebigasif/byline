//! Review blocks: a rating of one thing, which gives structured data (task
//! 1.7, KH-CON-5).

use khalid_content::{BlockKind, ReviewBlock, StructuredBlock, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

const REVIEW: &str = "\
:::review{item=\"Ahrefs Webmaster Tools\" type=\"SoftwareApplication\" rating=\"4.5\" best=\"5\" author=\"Asif\"}
Free, fast, and the **link data** is enough for a small site.
:::
";

#[cfg(test)]
fn review(body: &str) -> ReviewBlock {
    ReviewBlock {
        item: "Ahrefs Webmaster Tools".to_owned(),
        item_type: "SoftwareApplication".to_owned(),
        rating: "4.5".to_owned(),
        best: "5".to_owned(),
        author: Some("Asif".to_owned()),
        body: body.to_owned(),
    }
}

#[cfg(test)]
fn rendered(markdown: &str) -> khalid_content::Rendered {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap()
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

#[test]
fn a_review_round_trips() {
    let blocks = parse_blocks(REVIEW).unwrap();
    assert_eq!(print_blocks(&blocks), REVIEW);
    assert_eq!(
        blocks[0].kind,
        BlockKind::Review(review(
            "Free, fast, and the **link data** is enough for a small site."
        ))
    );
}

#[test]
fn renders_a_review() {
    insta::assert_snapshot!(rendered(REVIEW).html);
}

#[test]
fn a_review_is_structured_data_in_plain_text() {
    assert_eq!(
        rendered(REVIEW).structured,
        [StructuredBlock::Review(review(
            "Free, fast, and the link data is enough for a small site."
        ))]
    );
}

#[test]
fn the_best_score_is_five_unless_written() {
    let blocks =
        parse_blocks(":::review{item=\"A book\" type=\"Book\" rating=\"4\"}\nGood.\n:::\n")
            .unwrap();
    let BlockKind::Review(review) = &blocks[0].kind else {
        panic!("{blocks:?}");
    };
    assert_eq!(review.best, "5");
}

#[test]
fn a_rating_is_a_number_up_to_the_best() {
    assert_eq!(
        error(":::review{item=\"A book\" type=\"Book\" rating=\"6\"}\nGood.\n:::\n"),
        "line 1: a review's rating is a number from 0 to its best score, such as 4.5 out of 5"
    );
}

#[test]
fn a_review_names_a_type_search_engines_show() {
    assert_eq!(
        error(":::review{item=\"A tool\" type=\"Tool\" rating=\"4\"}\nGood.\n:::\n"),
        "line 1: a review's type is one search engines show, such as Product, Book, Course, SoftwareApplication or LocalBusiness"
    );
}
