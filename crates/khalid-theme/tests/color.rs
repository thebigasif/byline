//! Reading CSS colour values and measuring WCAG contrast.

use khalid_theme::Rgb;

#[test]
fn reads_every_way_a_stylesheet_writes_an_opaque_colour() {
    let blue = Some(Rgb::new(0x0b, 0x57, 0xd0));
    for text in [
        "#0b57d0",
        "#0B57D0",
        "#0b57d0ff",
        "rgb(11, 87, 208)",
        "rgb(11 87 208)",
        "rgba(11,87,208,1)",
        "rgb(11 87 208 / 100%)",
        "RGB(11, 87, 208)",
    ] {
        assert_eq!(Rgb::from_css(text), blue, "{text}");
    }
    assert_eq!(Rgb::from_css("#fff"), Some(Rgb::new(255, 255, 255)));
    assert_eq!(Rgb::from_css("#000f"), Some(Rgb::new(0, 0, 0)));
    assert_eq!(Rgb::from_css("white"), Some(Rgb::new(255, 255, 255)));
    assert_eq!(
        Rgb::from_css("rgb(100%, 50%, 0%)"),
        Some(Rgb::new(255, 128, 0))
    );
    assert_eq!(
        Rgb::from_css("hsl(0, 100%, 50%)"),
        Some(Rgb::new(255, 0, 0))
    );
    assert_eq!(
        Rgb::from_css("hsl(210deg 50% 40%)"),
        Some(Rgb::new(51, 102, 153))
    );
}

#[test]
fn faint_keyword_and_broken_values_are_not_colours() {
    for text in [
        "transparent",
        "currentColor",
        "inherit",
        "rgba(0, 0, 0, 0.1)",
        "#00000010",
        "#12",
        "#ggg",
        "rgb(1, 2)",
        "var(--brand)",
        "1px",
        "",
    ] {
        assert_eq!(Rgb::from_css(text), None, "{text}");
    }
}

#[test]
fn finds_the_colours_inside_a_shorthand_value() {
    let found = Rgb::all_in("1px solid #d0d7de, 0 0 0 3px rgba(11, 87, 208, 0.9) url(x.png)");

    assert_eq!(found, [Rgb::new(0xd0, 0xd7, 0xde), Rgb::new(11, 87, 208)]);
}

#[test]
fn contrast_follows_the_wcag_formula() {
    let white = Rgb::new(255, 255, 255);
    let black = Rgb::new(0, 0, 0);

    assert!((white.contrast(black) - 21.0).abs() < 1e-9);
    assert!((white.contrast(white) - 1.0).abs() < 1e-9);
    let grey = Rgb::new(0x76, 0x76, 0x76);
    assert!(
        (grey.contrast(white) - 4.54).abs() < 0.01,
        "{}",
        grey.contrast(white)
    );
    assert_eq!(grey.contrast(white), white.contrast(grey));
}

#[test]
fn writes_lowercase_six_digit_hex() {
    assert_eq!(Rgb::new(0x0b, 0x57, 0xd0).to_hex(), "#0b57d0");
    assert_eq!(
        Rgb::from_css("#ABC").map(Rgb::to_hex),
        Some("#aabbcc".to_owned())
    );
}
