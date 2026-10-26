//! Errors leave the API as RFC 9457 problem details, with `fix` and
//! `rule` where they apply (RULES.md BL-ERR-5), written in plain words.

use axum::body::to_bytes;
use axum::response::IntoResponse;
use khalid_api::problem::{Problem, ProblemType};
use khalid_api::query::{ListQuery, RawListQuery};
use khalid_core::UserMessage;
use serde_json::{Value, json};

#[cfg(test)]
fn no_author() -> Problem {
    let message = UserMessage::new(
        "This post has no author.",
        "Readers and Google trust posts with a named author.",
        "Add author: to the header.",
    );
    Problem::new(ProblemType::LintBlocked, &message)
        .with_rule("no-author")
        .with_instance("/api/admin/v1/posts/why-internal-links-matter/")
}

#[test]
fn a_problem_has_the_shape_from_the_api_standard() {
    let json = serde_json::to_value(no_author()).unwrap();
    assert_eq!(
        json,
        json!({
            "type": "https://khalid.dev/problems/lint-blocked",
            "title": "The post cannot be published",
            "status": 422,
            "detail": "This post has no author. Readers and Google trust posts with a named author.",
            "fix": "Add author: to the header.",
            "rule": "no-author",
            "instance": "/api/admin/v1/posts/why-internal-links-matter/",
        })
    );
}

#[test]
fn absent_parts_are_left_out() {
    let message = UserMessage::new(
        "No post has that slug.",
        "It may have moved.",
        "Check the address.",
    );
    let json = serde_json::to_value(Problem::new(ProblemType::NotFound, &message)).unwrap();
    assert_eq!(json.get("rule"), None);
    assert_eq!(json.get("instance"), None);
    assert_eq!(json["status"], 404);
}

#[tokio::test]
async fn a_problem_is_a_response_with_its_status_and_media_type() {
    let response = no_author().into_response();
    assert_eq!(response.status(), 422);
    let media = response.headers().get("content-type").unwrap();
    assert_eq!(media, "application/problem+json");
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["rule"], "no-author");
}

#[cfg(test)]
fn query_problem(name: &str, value: &str) -> Value {
    let mut raw = RawListQuery::default();
    match name {
        "limit" => raw.limit = Some(value.to_owned()),
        "filter" => raw.filter = Some(value.to_owned()),
        "formats" => raw.formats = Some(value.to_owned()),
        "page" => raw.page = Some(value.to_owned()),
        "include" => raw.include = Some(value.to_owned()),
        "fields" => raw.fields = Some(value.to_owned()),
        _ => raw.order = Some(value.to_owned()),
    }
    let error = ListQuery::try_from(&raw).unwrap_err();
    serde_json::to_value(Problem::from(error)).unwrap()
}

#[test]
fn a_query_mistake_becomes_a_400_with_a_fix() {
    let json = query_problem("limit", "0");
    assert_eq!(json["type"], "https://khalid.dev/problems/invalid-query");
    assert_eq!(json["status"], 400);
    assert_eq!(json["title"], "The list could not be read");
    assert_eq!(
        json["detail"],
        "The limit parameter must be a whole number from 1 to 100, or all. The list cannot be sent until it is fixed."
    );
    assert_eq!(json["fix"], "Use a number from 1 to 100, or all.");
    assert_eq!(json["rule"], "limit");
}

#[test]
fn every_query_mistake_reads_as_plain_sentences() {
    let cases = [
        ("filter", "tag:"),
        ("limit", "ten"),
        ("page", "0"),
        ("formats", "pdf"),
        ("include", "Tags"),
        ("fields", ""),
        ("order", "title sideways"),
        ("order", "title asc desc"),
        ("order", "a,b,c,d,e,f"),
    ];
    for (name, value) in cases {
        let json = query_problem(name, value);
        for part in ["detail", "fix"] {
            let text = json[part].as_str().unwrap();
            assert!(text.ends_with('.'), "{name}={value} {part}: {text}");
            assert!(!text.contains('\u{2014}'), "{name}={value} {part}: {text}");
            assert!(
                text.starts_with(char::is_uppercase),
                "{name}={value} {part}: {text}"
            );
        }
        let fix = json["fix"].as_str().unwrap();
        assert!(fix.split_whitespace().count() < 25, "{fix}");
    }
}
