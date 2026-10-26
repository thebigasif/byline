//! Maths: `$...$` and `$$...$$` become `MathML` at build, which browsers draw
//! with scripts off.

use khalid_content::render;
use khalid_core::{Locale, SlugMode};

const FIXTURE: &str = include_str!("../../../fixtures/maths/compound-growth.md");

#[cfg(test)]
fn html(markdown: &str) -> String {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii)
        .unwrap()
        .html
}

#[test]
fn the_maths_fixture_renders_to_mathml_with_no_script() {
    let html = html(FIXTURE);
    assert_eq!(
        html.matches("<math").count(),
        8,
        "six inline, two block: {html}"
    );
    assert_eq!(html.matches("<math display=\"block\"").count(), 2, "{html}");
    assert!(!html.contains("<script"), "{html}");
    assert!(html.contains("costs $5 a month"), "{html}");
    insta::assert_snapshot!(html);
}

#[test]
fn dollar_prices_stay_text() {
    assert_eq!(
        html("It costs $5 a month, or $10 with support.\n"),
        "<p>It costs $5 a month, or $10 with support.</p>\n"
    );
}

#[test]
fn maths_text_cannot_carry_markup() {
    let html = html("$\\text{<script>alert(1)</script>} < x$\n");
    assert!(!html.contains("<script"), "{html}");
    assert!(html.contains("&lt;"), "{html}");
}

#[test]
fn maths_in_code_stays_code() {
    assert_eq!(
        html("Write `$x^2$` for a square.\n"),
        "<p>Write <code>$x^2$</code> for a square.</p>\n"
    );
}

#[test]
fn maths_that_cannot_be_drawn_shows_its_source_as_text() {
    let html = html("$\\nosuchcommand{x}$\n");
    assert!(!html.contains("<math"), "{html}");
    assert!(html.contains("\\nosuchcommand{x}"), "{html}");
}
