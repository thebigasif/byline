//! The HTML of the FAQ and how-to blocks, and their structured data.

use comrak::nodes::NodeValue;
use comrak::{Arena, Options, parse_document};

use super::{Renderer, attribute};
use crate::{HowToBlock, Question, Step, StructuredBlock};

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

    /// Writes a how-to as a numbered list of named steps.
    pub(super) fn how_to(&mut self, how_to: &HowToBlock) {
        self.html.push_str("<ol class=\"howto\">\n");
        for step in &how_to.steps {
            self.push(&[
                "<li>\n<p class=\"howto-step\">",
                &attribute(&step.name),
                "</p>\n",
            ]);
            self.markdown(&step.text);
            self.html.push_str("</li>\n");
        }
        self.html.push_str("</ol>\n");
        let steps = how_to
            .steps
            .iter()
            .map(|step| Step {
                name: step.name.clone(),
                text: plain(&step.text),
            })
            .collect();
        self.facts
            .structured
            .push(StructuredBlock::HowTo(HowToBlock {
                name: how_to.name.clone(),
                time: how_to.time,
                steps,
            }));
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
