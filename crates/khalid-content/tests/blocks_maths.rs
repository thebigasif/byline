//! The maths block, `:::maths`, a formula on its own drawn as `MathML` with no
//! script (G37, KH-CON-5, KH-WEB-9).

use khalid_content::{BlockKind, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

#[cfg(test)]
fn round_trip(markdown: &str, kind: &BlockKind) {
    let blocks = parse_blocks(markdown).unwrap();
    assert_eq!(blocks.len(), 1, "{markdown:?} gave {blocks:?}");
    assert_eq!(&blocks[0].kind, kind, "{markdown:?}");
    assert_eq!(print_blocks(&blocks), markdown);
}

#[cfg(test)]
fn html(markdown: &str) -> String {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii)
        .unwrap()
        .html
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

#[test]
fn a_maths_block_round_trips_with_its_label() {
    round_trip(
        ":::maths{label=\"Compound growth\"}\nA = P\\left(1 + \\frac{r}{n}\\right)^{nt}\n:::\n",
        &BlockKind::Maths {
            label: Some("Compound growth".to_owned()),
            formula: "A = P\\left(1 + \\frac{r}{n}\\right)^{nt}".to_owned(),
        },
    );
}

#[test]
fn a_maths_block_round_trips_over_several_lines_without_a_label() {
    round_trip(
        ":::maths\n\\begin{aligned}\na &= b + c \\\\\nd &= e\n\\end{aligned}\n:::\n",
        &BlockKind::Maths {
            label: None,
            formula: "\\begin{aligned}\na &= b + c \\\\\nd &= e\n\\end{aligned}".to_owned(),
        },
    );
}

#[test]
fn a_maths_block_renders_to_block_mathml_in_a_figure_with_no_script() {
    let html = html(
        "Money grows like this.\n\n:::maths{label=\"Compound growth\"}\nA = P\\left(1 + \\frac{r}{n}\\right)^{nt}\n:::\n",
    );
    assert!(
        html.contains("<figure class=\"maths\">\n<math display=\"block\""),
        "{html}"
    );
    assert!(
        html.contains("<figcaption>Compound growth</figcaption>\n</figure>\n"),
        "{html}"
    );
    assert!(!html.contains("<script"), "{html}");
    insta::assert_snapshot!(html);
}

#[test]
fn a_maths_block_without_a_label_has_no_caption() {
    let html = html(":::maths\nE = mc^2\n:::\n");
    assert!(
        html.starts_with("<figure class=\"maths\">\n<math display=\"block\""),
        "{html}"
    );
    assert!(!html.contains("figcaption"), "{html}");
}

#[test]
fn a_formula_is_not_counted_as_words() {
    let rendered = render(
        "Two words.\n\n:::maths\nx + y + z\n:::\n",
        &Locale::try_new("en").unwrap(),
        SlugMode::Ascii,
    )
    .unwrap();
    assert_eq!(rendered.words, 2);
}

#[test]
fn a_label_and_a_formula_cannot_carry_markup() {
    let html =
        html(":::maths{label=\"<b>bold</b>\"}\n\\text{<script>alert(1)</script>} < x\n:::\n");
    assert!(!html.contains("<script"), "{html}");
    assert!(!html.contains("<b>"), "{html}");
}

#[test]
fn refuses_a_maths_block_with_no_formula() {
    assert_eq!(
        error("Intro.\n\n:::maths\n\n:::\n"),
        "line 3: a maths block needs a formula between its ::: lines, such as E = mc^2"
    );
}

#[test]
fn refuses_a_formula_that_cannot_be_drawn_on_its_line() {
    let message = error("Intro.\n\n:::maths\nx = \\nosuchcommand{y}\n:::\n");
    assert!(
        message.starts_with("line 4: the maths cannot be drawn: "),
        "{message}"
    );
}

#[test]
fn refuses_an_attribute_a_maths_block_does_not_take() {
    assert_eq!(
        error(":::maths{caption=\"Growth\"}\nx\n:::\n"),
        "line 1: a maths block does not take caption; it takes label"
    );
}

#[test]
fn refuses_a_maths_block_written_on_one_line() {
    assert_eq!(
        error("::maths{label=\"Growth\"}\n"),
        "line 1: maths has a body; open it with :::maths and close it with :::"
    );
}
