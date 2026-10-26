//! The share image: every byte pinned per layout, and inputs that must not fail.

use insta::assert_binary_snapshot;
use khalid_share_image::{CardColors, Direction, Rgb, ShareCard, render};

#[cfg(test)]
fn card(title: &str, direction: Direction) -> ShareCard {
    let colors = CardColors::new(
        Rgb([0xfb, 0xf7, 0xf0]),
        Rgb([0x2b, 0x1d, 0x0e]),
        Rgb([0xb4, 0x42, 0x1b]),
    );
    let mut card = ShareCard::new(title, "Rye and Barley Bakery", colors);
    card.direction = direction;
    card
}

#[test]
fn a_short_title_in_english() {
    let title = "How we bake sourdough in a home oven";
    let png = render(&card(title, Direction::LeftToRight)).unwrap();

    assert_binary_snapshot!("short-title.png", png);
}

#[test]
fn a_long_title_wraps_to_three_lines_and_ends_with_an_ellipsis() {
    let title = "Why we stopped buying flour from the big mills, what it cost us in the first \
                 year, and the seven small farms that now grow every grain we bake with";
    let png = render(&card(title, Direction::LeftToRight)).unwrap();

    assert_binary_snapshot!("long-title.png", png);
}

#[test]
fn an_arabic_title_is_shaped_and_set_from_the_right() {
    let png = render(&card("كيف نخبز خبز العجين المخمر", Direction::RightToLeft)).unwrap();

    assert_binary_snapshot!("arabic-title.png", png);
}

#[test]
fn markup_control_characters_and_one_endless_word_are_drawn_without_failing() {
    let titles = [
        "Bread & <butter> \"tips\" for 'beginners'",
        "Proofing\u{0} times\u{7} explained",
        &"sourdough".repeat(40),
        "",
    ];

    for title in titles {
        let png = render(&card(title, Direction::LeftToRight));
        assert!(png.is_ok(), "{title:?}: {png:?}");
    }
}
