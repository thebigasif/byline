//! The four colours theme sync looks for, from rules on the page's own
//! elements first and from named custom properties next.

use khalid_theme::{Palette, Rgb};

#[cfg(test)]
fn hex(text: &str) -> Rgb {
    Rgb::from_css(text).unwrap()
}

#[test]
fn rules_on_body_and_links_come_first_with_references_resolved() {
    let palette = Palette::from_css(&[
        ":root { --wp--preset--color--base: #fdfcf8; --wp--preset--color--contrast: #1d1d1b; }\n\
         body { background-color: var(--wp--preset--color--base); color: var(--wp--preset--color--contrast) }\n\
         a, .button-link { color: #a33a1f; }\n\
         a:hover { color: #000 }",
        "a:link { color: rgb(163, 58, 31) } body { background: #fff url(paper.png) }",
    ]);

    assert_eq!(
        palette.background,
        Some(hex("#ffffff")),
        "the later rule wins"
    );
    assert_eq!(palette.text, Some(hex("#1d1d1b")));
    assert_eq!(
        palette.link,
        Some(hex("#a33a1f")),
        "hover is not the link colour"
    );
    assert_eq!(
        palette.accent, palette.link,
        "the link colour doubles as accent"
    );
}

#[test]
fn frequency_decides_when_nothing_is_named() {
    let palette = Palette::from_css(&[
        ".x1a{color:#374151}.x1b{color:#374151}.x9c{color:#9ca3af}.x9d{color:#9ca3af}\
         .x9e{color:#9ca3af}.b1{background-color:#fff}.b2{background-color:#fff}\
         .b3{background-color:#f3f4f6}.btn_3fa{background-color:#4f46e5;color:#fff}\
         .ring_7{box-shadow:0 0 0 2px #4f46e5}.tag{color:#0e7490}",
    ]);

    assert_eq!(palette.background, Some(hex("#ffffff")));
    assert_eq!(palette.text, Some(hex("#374151")), "#9ca3af is too faint");
    assert_eq!(palette.accent, Some(hex("#4f46e5")));
    assert_eq!(palette.link, None);
}

#[test]
fn named_properties_fill_what_the_rules_leave_out() {
    let palette = Palette::from_css(&[":root {\n\
         --bs-primary: #0d6efd; --bs-primary-rgb: 13, 110, 253; --bs-primary-light: #cfe2ff;\n\
         --bs-body-bg: #fff; --bs-body-color: #212529;\n\
         --bs-link-color: var(--bs-primary); --bs-link-hover-color: #0a58ca;\n\
         --bs-secondary-bg: #e9ecef;\n\
         }"]);

    assert_eq!(palette.background, Some(hex("#ffffff")));
    assert_eq!(palette.text, Some(hex("#212529")));
    assert_eq!(palette.link, Some(hex("#0d6efd")));
    assert_eq!(palette.accent, Some(hex("#0d6efd")));
}

#[test]
fn the_shortest_name_wins_among_properties_for_one_colour() {
    let palette = Palette::from_css(&[
        ":root { --brand-500: #e4572e; --brand: #c2410c; --color-text-subtle: #888; --color-text: #111 }",
    ]);

    assert_eq!(palette.accent, Some(hex("#c2410c")));
    assert_eq!(palette.text, Some(hex("#111111")));
}

#[test]
fn a_stylesheet_with_none_of_the_colours_finds_nothing() {
    let palette = Palette::from_css(&[".grid { display: grid } a { text-decoration: underline }"]);

    assert_eq!(palette, Palette::default());
}
