//! The HTML of the FAQ, how-to, review and product blocks, and their
//! structured data.

use comrak::nodes::NodeValue;
use comrak::{Arena, Options, parse_document};

use super::{Image, Link};
use super::{Renderer, attribute};
use crate::{HowToBlock, ProductBlock, Question, ReviewBlock, Step, StructuredBlock};

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

    /// Writes a review: what it rates, the score out of the best, and the
    /// review itself.
    pub(super) fn review(&mut self, review: &ReviewBlock) {
        self.push(&[
            "<section class=\"review\">\n<p class=\"review-item\">",
            &attribute(&review.item),
            "</p>\n<p class=\"review-rating\">",
            &attribute(&review.rating),
            " / ",
            &attribute(&review.best),
            "</p>\n",
        ]);
        self.markdown(&review.body);
        self.html.push_str("</section>\n");
        self.facts
            .structured
            .push(StructuredBlock::Review(ReviewBlock {
                body: plain(&review.body),
                ..review.clone()
            }));
    }

    /// Writes a product card: picture, name, score, price, description and
    /// button.
    pub(super) fn product(&mut self, product: &ProductBlock) {
        let name = attribute(&product.name);
        self.html.push_str("<section class=\"product\">\n");
        if let Some(image) = &product.image {
            self.push(&["<img src=\"", &attribute(image), "\" alt=\"", &name]);
            self.html
                .push_str("\" loading=\"lazy\" decoding=\"async\" />\n");
            self.facts.images.push(Image {
                src: image.clone(),
                alt: Some(product.name.clone()),
            });
        }
        self.push(&["<p class=\"product-name\">", &name, "</p>\n"]);
        if let Some(rating) = &product.rating {
            let best = attribute(&product.best);
            self.push(&[
                "<p class=\"product-rating\">",
                &attribute(rating),
                " / ",
                &best,
                "</p>\n",
            ]);
        }
        if let (Some(price), Some(currency)) = (&product.price, &product.currency) {
            let (price, currency) = (attribute(price), attribute(currency));
            self.push(&[
                "<p class=\"product-price\"><data value=\"",
                &price,
                "\">",
                &currency,
                " ",
                &price,
                "</data></p>\n",
            ]);
        }
        self.markdown(&product.description);
        if let (Some(href), Some(button)) = (&product.href, &product.button) {
            self.push(&[
                "<p><a class=\"button\" href=\"",
                &attribute(href),
                "\">",
                &attribute(button),
                "</a></p>\n",
            ]);
            self.facts.links.push(Link {
                href: href.clone(),
                text: button.clone(),
                internal: !href.contains(':'),
            });
        }
        self.html.push_str("</section>\n");
        self.facts
            .structured
            .push(StructuredBlock::Product(ProductBlock {
                description: plain(&product.description),
                ..product.clone()
            }));
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
