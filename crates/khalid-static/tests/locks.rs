//! Static mode's page locks: every form that sends to the site itself
//! becomes a short note in the page's language, and nothing else changes.

use khalid_core::Locale;
use khalid_static::{Locks, ServerFeature};

const SUBSCRIBE: &str = "<form class=\"subscribe\" method=\"post\" action=\"/subscribe\">\n<p><label for=\"subscribe-1-email\">Email</label>\n<input id=\"subscribe-1-email\" name=\"email\" type=\"email\" required /></p>\n<p><button type=\"submit\">Join</button></p>\n</form>\n";

const CONTACT: &str = "<form class=\"form\" method=\"post\" action=\"/forms/contact\">\n<p><label for=\"contact-name\">Name</label>\n<input id=\"contact-name\" name=\"name\" type=\"text\" /></p>\n</form>\n";

const SEARCH: &str = "<form class=\"search\" role=\"search\" action=\"/blog/search/\" method=\"get\">\n<input name=\"q\" type=\"search\" />\n</form>";

#[cfg(test)]
fn locale(tag: &str) -> Locale {
    Locale::try_new(tag).unwrap()
}

#[test]
fn a_page_without_forms_is_left_as_it_is() {
    let locks = Locks::load().unwrap();
    let html =
        "<main><h1>Title</h1><p>Plain <b>text</b>, <a href=\"/formats/\">formats</a>.</p></main>";
    let page = locks.page(html, &locale("en")).unwrap();
    assert_eq!(page.html, html);
    assert!(page.locked.is_empty());
}

#[test]
fn each_kind_of_form_becomes_its_note() {
    let locks = Locks::load().unwrap();
    let html = format!("<main>\n{SUBSCRIBE}<p>Between.</p>\n{CONTACT}{SEARCH}\n</main>");
    let page = locks.page(&html, &locale("en")).unwrap();
    assert_eq!(
        page.html,
        "<main>\n<p class=\"locked\" data-feature=\"newsletter\">Signing up by email is not open on this site.</p>\n<p>Between.</p>\n<p class=\"locked\" data-feature=\"forms\">This form cannot be sent from this site.</p>\n<p class=\"locked\" data-feature=\"search\">Search is not available on this site.</p>\n</main>"
    );
    assert_eq!(
        page.locked.into_iter().collect::<Vec<_>>(),
        [
            ServerFeature::Newsletter,
            ServerFeature::Forms,
            ServerFeature::Search
        ]
    );
}

#[test]
fn a_german_page_gets_german_notes_and_a_missing_language_english_ones() {
    let locks = Locks::load().unwrap();
    let german = locks.page(SUBSCRIBE, &locale("de-AT")).unwrap();
    assert_eq!(
        german.html,
        "<p class=\"locked\" data-feature=\"newsletter\">Anmeldungen per E-Mail sind auf dieser Website nicht möglich.</p>\n"
    );
    let french = locks.page(SUBSCRIBE, &locale("fr")).unwrap();
    assert!(
        french.html.contains("Signing up by email"),
        "{}",
        french.html
    );
}

#[test]
fn a_form_that_posts_to_a_provider_stays() {
    let locks = Locks::load().unwrap();
    let html = "<FORM method='post' action='https://buttondown.example/api/subscribe'><input name=email></FORM>";
    let page = locks.page(html, &locale("en")).unwrap();
    assert_eq!(page.html, html);
    assert!(page.locked.is_empty());
}

#[test]
fn upper_case_tags_and_quoted_angle_brackets_are_read_as_one_form() {
    let locks = Locks::load().unwrap();
    let html =
        "<p>a</p><FORM data-x=\"a>b\" ACTION=/comments/new><textarea></textarea></Form ><p>b</p>";
    let page = locks.page(html, &locale("en")).unwrap();
    assert_eq!(
        page.html,
        "<p>a</p><p class=\"locked\" data-feature=\"comments\">Comments are not open on this site.</p><p>b</p>"
    );
}

#[test]
fn a_form_with_no_end_is_refused_with_its_place() {
    let locks = Locks::load().unwrap();
    let error = locks
        .page("<p>x</p><form action=\"/subscribe\"><input>", &locale("en"))
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the form at byte 8 has no closing </form> tag"
    );
}

#[test]
fn every_feature_says_its_name_once() {
    let names: Vec<&str> = ServerFeature::ALL.iter().map(|f| f.name()).collect();
    assert_eq!(
        names,
        [
            "comments",
            "newsletter",
            "forms",
            "search",
            "admin",
            "scheduled_publish"
        ]
    );
}
