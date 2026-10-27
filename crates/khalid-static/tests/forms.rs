//! The forms on a rendered page: where each one starts and ends, and what
//! it sends to the site.

use khalid_static::{FormKind, PageForm, forms};

const SUBSCRIBE: &str = "<form class=\"subscribe\" method=\"post\" action=\"/subscribe\">\n<p><label for=\"subscribe-1-email\">Email</label>\n<input id=\"subscribe-1-email\" name=\"email\" type=\"email\" required /></p>\n<p><button type=\"submit\">Join</button></p>\n</form>";

const CONTACT: &str = "<form class=\"form\" method=\"post\" action=\"/forms/contact\">\n<p><label for=\"contact-name\">Name</label>\n<input id=\"contact-name\" name=\"name\" type=\"text\" /></p>\n</form>";

const SEARCH: &str = "<form class=\"search\" role=\"search\" action=\"/blog/search/\" method=\"get\">\n<input name=\"q\" type=\"search\" />\n</form>";

/// What each form found in `html` covers, and its kind.
#[cfg(test)]
fn found(html: &str) -> Vec<(&str, Option<FormKind>)> {
    forms(html)
        .unwrap()
        .into_iter()
        .map(
            |PageForm {
                 start, end, kind, ..
             }| (html.get(start..end).unwrap(), kind),
        )
        .collect()
}

#[test]
fn a_page_without_forms_has_none() {
    let html =
        "<main><h1>Title</h1><p>Plain <b>text</b>, <a href=\"/formats/\">formats</a>.</p></main>";
    assert_eq!(found(html), []);
}

#[test]
fn each_form_is_found_whole_with_its_kind() {
    let html = format!("<main>\n{SUBSCRIBE}\n<p>Between.</p>\n{CONTACT}{SEARCH}\n</main>");
    assert_eq!(
        found(&html),
        [
            (SUBSCRIBE, Some(FormKind::Newsletter)),
            (CONTACT, Some(FormKind::Forms)),
            (SEARCH, Some(FormKind::Search)),
        ]
    );
}

#[test]
fn a_form_that_posts_to_a_provider_sends_nothing_to_the_site() {
    for html in [
        "<FORM method='post' action='https://buttondown.example/api/subscribe'><input name=email></FORM>",
        "<form action=\"mailto:hello@example.com\"><textarea></textarea></form>",
    ] {
        assert_eq!(found(html), [(html, None)]);
    }
}

#[test]
fn upper_case_tags_and_quoted_angle_brackets_are_read_as_one_form() {
    let form = "<FORM data-x=\"a>b\" ACTION=/comments/new><textarea></textarea></Form >";
    let html = format!("<p>a</p>{form}<p>b</p>");
    assert_eq!(found(&html), [(form, Some(FormKind::Comments))]);
}

#[test]
fn a_form_with_no_end_is_refused_with_its_place() {
    for html in [
        "<p>x</p><form action=\"/subscribe\"><input>",
        "<p>x</p><form action=\"/subscribe\"",
    ] {
        assert_eq!(
            forms(html).unwrap_err().to_string(),
            "the form at byte 8 has no closing </form> tag",
            "{html}"
        );
    }
}

#[test]
fn every_kind_needs_its_feature() {
    let features: Vec<&str> = [
        FormKind::Comments,
        FormKind::Newsletter,
        FormKind::Forms,
        FormKind::Search,
    ]
    .into_iter()
    .map(|kind| kind.feature().name())
    .collect();
    assert_eq!(features, ["comments", "newsletter", "forms", "search"]);
}
