//! The HTML of the blocks that post to the server. Each is a real form that
//! works with scripts off, with a spam trap people never see.

use super::{Renderer, attribute};
use crate::FormField;
use crate::directive::TRAP;

impl Renderer {
    /// Writes a form that posts to `/forms/{id}`, one labelled field each.
    pub(super) fn form(&mut self, id: &str, button: &str, fields: &[FormField]) {
        let id = attribute(id);
        self.push(&[
            "<form class=\"form\" method=\"post\" action=\"/forms/",
            &id,
            "\">\n",
        ]);
        for field in fields {
            let name = attribute(&field.name);
            let label = attribute(&field.label);
            let field_id = [id.as_str(), "-", &name].concat();
            let required = if field.required { " required" } else { "" };
            self.push(&["<p><label for=\"", &field_id, "\">", &label, "</label>\n"]);
            match field.kind.input_type() {
                Some(kind) => self.push(&[
                    "<input id=\"",
                    &field_id,
                    "\" name=\"",
                    &name,
                    "\" type=\"",
                    kind,
                    "\"",
                    required,
                    " />",
                ]),
                None => self.push(&[
                    "<textarea id=\"",
                    &field_id,
                    "\" name=\"",
                    &name,
                    "\"",
                    required,
                    "></textarea>",
                ]),
            }
            self.html.push_str("</p>\n");
        }
        self.trap(&id);
        self.push(&[
            "<p><button type=\"submit\">",
            &attribute(button),
            "</button></p>\n</form>\n",
        ]);
    }

    /// Writes the spam trap: a field hidden from people and screen readers
    /// that only robots fill in, so the server can drop what they send.
    fn trap(&mut self, form: &str) {
        let trap_id = [form, "-", TRAP].concat();
        self.push(&[
            "<p class=\"form-trap\" aria-hidden=\"true\"><label for=\"",
            &trap_id,
            "\">Leave this empty</label>\n<input id=\"",
            &trap_id,
            "\" name=\"",
            TRAP,
            "\" type=\"text\" tabindex=\"-1\" autocomplete=\"off\" /></p>\n",
        ]);
    }
}
