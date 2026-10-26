//! The HTML of the FAQ block, and its structured data.

use comrak::nodes::NodeValue;
use comrak::{Arena, Options, parse_document};

use super::{Renderer, attribute};
use crate::{Question, StructuredBlock};

impl Renderer {
    /// Writes each question as a summary a reader opens to see the answer,
    /// which works with scripts off.
    pub(super) fn faq(&mut self, questions: &[Question]) {
        self.html.push_str("<section class=\"faq\">\n");
        for question in questions {
            self.push(&[
                "<details>\n<summary>",
                &attribute(&question.text),
                "</summary>\n",
            ]);
            self.markdown(&question.answer);
            self.html.push_str("</details>\n");
        }
        self.html.push_str("</section>\n");
        let plain = questions
            .iter()
            .map(|question| Question {
                text: question.text.clone(),
                answer: plain(&question.answer),
            })
            .collect();
        self.facts.structured.push(StructuredBlock::Faq(plain));
    }
}

/// The words of `markdown` without its markup, one space between them, for
/// structured data.
fn plain(markdown: &str) -> String {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &Options::default());
    let mut text = String::new();
    for node in root.descendants() {
        match &node.data.borrow().value {
            NodeValue::Text(words) => text.push_str(words),
            NodeValue::Code(code) => text.push_str(&code.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak | NodeValue::Paragraph => text.push(' '),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
