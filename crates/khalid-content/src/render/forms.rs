//! The HTML of the blocks that post to the server. Each is a real form that
//! works with scripts off, with a spam trap people never see.

use khalid_core::Slug;

use super::{Renderer, attribute, inside};
use crate::directive::TRAP;
use crate::{Block, FormField};

/// How a body's `:::gated` blocks are written (task 5.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Gated<'page> {
    /// As a note and a sign-up form, never the body: the public page.
    Locked {
        /// The address of the page, sent with the form so the reader
        /// comes back to it once they confirm. `None` leaves it out.
        back: Option<&'page str>,
    },
    /// As the body: the page a confirmed subscriber reads.
    Open,
}

impl Renderer<'_> {
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

    /// Writes an email field that posts to `/subscribe`, numbered so two
    /// on one page keep their own ids. With `list`, the form signs the
    /// reader up to that list. With `back`, the form names the page a
    /// confirmed reader returns to.
    pub(super) fn subscribe(
        &mut self,
        (label, button): (&str, &str),
        list: Option<&Slug>,
        back: Option<&str>,
    ) {
        self.subscribe_forms = self.subscribe_forms.saturating_add(1);
        let id = format!("subscribe-{}", self.subscribe_forms);
        self.html
            .push_str("<form class=\"subscribe\" method=\"post\" action=\"/subscribe\">\n");
        if let Some(back) = back {
            self.push(&[
                "<input type=\"hidden\" name=\"back\" value=\"",
                &attribute(back),
                "\" />\n",
            ]);
        }
        self.push(&[
            "<p><label for=\"",
            &id,
            "-email\">",
            &attribute(label),
            "</label>\n<input id=\"",
            &id,
            "-email\" name=\"email\" type=\"email\" autocomplete=\"email\" required /></p>\n",
        ]);
        if let Some(list) = list {
            self.push(&[
                "<input type=\"hidden\" name=\"list\" value=\"",
                list.as_str(),
                "\" />\n",
            ]);
        }
        self.trap(&id);
        self.push(&[
            "<p><button type=\"submit\">",
            &attribute(button),
            "</button></p>\n</form>\n",
        ]);
    }

    /// Writes gated content as the renderer's [`Gated`] says. Locked, it is
    /// a lock card, its note and a subscribe form, and the body is never
    /// written, so the page cannot leak it. Open, it is the body.
    pub(super) fn gated(&mut self, block: &Block, note: &str, label: &str, button: &str) {
        self.facts.gated = true;
        let back = match self.gated {
            Gated::Open => {
                self.html.push_str("<section class=\"gated gated-open\">\n");
                self.markdown(&inside(&block.source));
                self.html.push_str("</section>\n");
                return;
            }
            Gated::Locked { back } => back,
        };
        self.push(&[
            "<section class=\"gated\">\n<p class=\"gated-note\">",
            &attribute(note),
            "</p>\n",
        ]);
        self.subscribe((label, button), None, back);
        self.html.push_str("</section>\n");
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
