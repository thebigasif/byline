//! Form blocks: labelled fields posted to the server, with a spam trap,
//! working with scripts off (task 1.7, KH-CON-5).

use khalid_content::{BlockKind, FieldKind, FormField, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

const FORM: &str = "\
:::form{id=\"contact\" button=\"Send\"}
- text name: Your name
- email reply_to: Your email address
- textarea message: What can we help with?
- text company?: Company
:::
";

/// A field named `name`, optional when the name ends in `?`.
#[cfg(test)]
fn field(kind: FieldKind, name: &str, label: &str) -> FormField {
    FormField {
        kind,
        name: name.trim_end_matches('?').to_owned(),
        label: label.to_owned(),
        required: !name.ends_with('?'),
    }
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

#[test]
fn a_form_round_trips() {
    let blocks = parse_blocks(FORM).unwrap();
    assert_eq!(print_blocks(&blocks), FORM);
    assert_eq!(
        blocks[0].kind,
        BlockKind::Form {
            id: "contact".to_owned(),
            button: "Send".to_owned(),
            fields: vec![
                field(FieldKind::Text, "name", "Your name"),
                field(FieldKind::Email, "reply_to", "Your email address"),
                field(FieldKind::Textarea, "message", "What can we help with?"),
                field(FieldKind::Text, "company?", "Company"),
            ],
        }
    );
}

#[test]
fn renders_a_form_that_posts_with_a_spam_trap() {
    insta::assert_snapshot!(
        render(FORM, &Locale::try_new("en").unwrap(), SlugMode::Ascii)
            .unwrap()
            .html
    );
}

#[test]
fn a_field_is_a_type_a_name_and_a_label() {
    assert_eq!(
        error(":::form{id=\"contact\" button=\"Send\"}\n- Your name\n:::\n"),
        "line 2: each line of a form is a field, such as - email reply_to: Your email address"
    );
}

#[test]
fn field_names_are_unique() {
    assert_eq!(
        error(
            ":::form{id=\"contact\" button=\"Send\"}\n- text name: Name\n- text name: Name again\n:::\n"
        ),
        "line 3: each field of a form has its own name, and homepage is kept for the spam trap"
    );
}
