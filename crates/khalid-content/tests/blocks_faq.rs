//! FAQ blocks, which nest one directive in another and give structured data
//! (task 1.7, KH-CON-5, ADR 0012).

use khalid_content::{BlockKind, Question, StructuredBlock, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

const FAQ: &str = "\
::::faq
:::question{text=\"What is a title tag?\"}
The **blue link** in search results.
:::

:::question{text=\"How long should it be?\"}
About 50 to 60 characters,
so it is not cut off.
:::
::::
";

#[cfg(test)]
fn round_trip(markdown: &str) -> BlockKind {
    let blocks = parse_blocks(markdown).unwrap();
    assert_eq!(blocks.len(), 1, "{markdown:?} gave {blocks:?}");
    assert_eq!(print_blocks(&blocks), markdown);
    blocks[0].kind.clone()
}

#[cfg(test)]
fn rendered(markdown: &str) -> khalid_content::Rendered {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap()
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

#[cfg(test)]
fn question(text: &str, answer: &str) -> Question {
    Question {
        text: text.to_owned(),
        answer: answer.to_owned(),
    }
}

#[test]
fn an_faq_round_trips() {
    assert_eq!(
        round_trip(FAQ),
        BlockKind::Faq {
            questions: vec![
                question(
                    "What is a title tag?",
                    "The **blue link** in search results."
                ),
                question(
                    "How long should it be?",
                    "About 50 to 60 characters,\nso it is not cut off."
                ),
            ],
        }
    );
}

#[test]
fn an_faq_holds_only_questions() {
    assert_eq!(
        error("::::faq\nWhat is a title tag?\n::::\n"),
        "line 2: an FAQ holds only questions, each written :::question{text=\"...\"}, its answer, then :::"
    );
}

#[test]
fn a_question_must_be_closed() {
    assert_eq!(
        error("::::faq\n:::question{text=\"Why?\"}\nBecause.\n::::\n"),
        "line 2: the question block opened here is never closed with a ::: line"
    );
}

#[test]
fn renders_an_faq() {
    insta::assert_snapshot!(rendered(FAQ).html);
}

#[test]
fn an_faq_is_structured_data_in_plain_text() {
    assert_eq!(
        rendered(FAQ).structured,
        [StructuredBlock::Faq(vec![
            question("What is a title tag?", "The blue link in search results."),
            question(
                "How long should it be?",
                "About 50 to 60 characters, so it is not cut off."
            ),
        ])]
    );
}
