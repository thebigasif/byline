//! A posted comment form is read into a comment, a problem list, or bait.

use khalid_comments::{CommentAuthor, CommentForm, CommentId, FieldProblem, ReaderEmail, Reading};

fn form(name: &str, email: &str, comment: &str) -> CommentForm {
    CommentForm {
        page: "/blog/why-links-matter/".to_owned(),
        parent: None,
        name: name.to_owned(),
        email: email.to_owned(),
        comment: comment.to_owned(),
        website: String::new(),
    }
}

#[test]
fn a_whole_form_is_a_reader_comment() {
    let Reading::Comment(read) = form(" Lena ", "", "Great post. ").read() else {
        panic!("a whole form was not read as a comment");
    };
    assert_eq!(read.page, "/blog/why-links-matter/");
    assert_eq!(read.parent, None);
    let CommentAuthor::Reader { name, email } = &read.author else {
        panic!("a form comment is a reader's");
    };
    assert_eq!(name.as_str(), "Lena");
    assert_eq!(*email, None);
    assert_eq!(read.text.as_str(), "Great post.");
}

#[test]
fn an_email_is_kept_when_given() {
    let Reading::Comment(read) = form("Lena", " lena@example.com ", "Hi").read() else {
        panic!("not read");
    };
    let CommentAuthor::Reader { email, .. } = &read.author else {
        panic!("not a reader");
    };
    let email = email.as_ref().map(ReaderEmail::as_str);
    assert_eq!(email, Some("lena@example.com"));
}

#[test]
fn a_reply_names_its_parent() {
    let mut reply = form("Lena", "", "Agreed.");
    reply.parent = Some("17".to_owned());
    let Reading::Comment(read) = reply.read() else {
        panic!("not read");
    };
    assert_eq!(read.parent, Some(CommentId::new(17)));
}

#[test]
fn a_filled_honeypot_is_bait_whatever_else_it_holds() {
    let mut bot = form("", "not an email", "");
    bot.website = "https://cheap-pills.example".to_owned();
    assert_eq!(bot.read(), Reading::Bait);
}

#[test]
fn every_problem_is_listed_at_once_in_form_order() {
    let mut broken = form(" ", "lena@", "\n \t");
    broken.parent = Some("first".to_owned());
    assert_eq!(
        broken.read(),
        Reading::Problems(vec![
            FieldProblem::NameMissing,
            FieldProblem::EmailInvalid,
            FieldProblem::TextMissing,
            FieldProblem::ParentInvalid,
        ])
    );
}
