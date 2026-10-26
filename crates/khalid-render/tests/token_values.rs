//! Token values: each one checked so it cannot end its CSS declaration.

use khalid_render::{Color, FontStack, Length};

#[test]
fn colours_are_kept_as_lowercase_rrggbb() {
    assert_eq!(
        Color::try_from("#123".to_owned()).unwrap().as_str(),
        "#112233"
    );
    assert_eq!(
        Color::try_from("#0A58CA".to_owned()).unwrap().as_str(),
        "#0a58ca"
    );
}

#[test]
fn anything_else_is_not_a_colour() {
    for text in ["black", "#12345", "#ggg", "123456", "#1234567", "#12;}"] {
        assert!(Color::try_from(text.to_owned()).is_err(), "{text}");
    }
}

#[test]
fn black_on_white_is_the_largest_contrast_and_a_colour_on_itself_the_least() {
    let black = Color::try_from("#000".to_owned()).unwrap();
    let white = Color::try_from("#fff".to_owned()).unwrap();
    assert!((black.contrast(&white) - 21.0).abs() < 1e-9);
    assert!(
        (white.contrast(&black) - 21.0).abs() < 1e-9,
        "order does not matter"
    );
    assert!((white.contrast(&white) - 1.0).abs() < 1e-9);
}

#[test]
fn font_lists_hold_names_commas_and_paired_quotes() {
    let stack = r#"Inter, "Segoe UI", Noto Sans JP, sans-serif"#;
    assert_eq!(
        FontStack::try_from(stack.to_owned()).unwrap().as_str(),
        stack
    );
    for text in [
        "",
        "  ",
        r#"Inter, "Segoe UI"#,
        "Inter; } body { display: none",
        "a/*b*/",
        "url(x)",
    ] {
        assert!(FontStack::try_from(text.to_owned()).is_err(), "{text:?}");
    }
}

#[test]
fn lengths_are_a_number_and_a_known_unit() {
    for text in ["4px", "0.375rem", "1.5em", "68ch", ".5rem"] {
        assert_eq!(Length::try_from(text.to_owned()).unwrap().as_str(), text);
    }
    for text in ["4vw", "rem", "1.2.3rem", "-1px", "4 px", "calc(1px)"] {
        assert!(Length::try_from(text.to_owned()).is_err(), "{text}");
    }
}
