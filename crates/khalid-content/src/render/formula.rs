//! The maths block as a figure of `MathML`, which needs no script.

use super::{Renderer, attribute};
use crate::maths;

impl Renderer<'_> {
    /// Writes a formula on its own as block `MathML` in a figure, with its
    /// label as the caption. Reading the block checked the formula, so it
    /// draws; should it not, the source shows as escaped text.
    pub(super) fn maths(&mut self, label: Option<&str>, formula: &str) {
        let drawn = maths::display(formula).unwrap_or_else(|_| attribute(formula));
        self.push(&["<figure class=\"maths\">\n", &drawn, "\n"]);
        if let Some(label) = label {
            self.push(&["<figcaption>", &attribute(label), "</figcaption>\n"]);
        }
        self.html.push_str("</figure>\n");
    }
}
