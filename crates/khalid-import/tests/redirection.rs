//! The rules of the Redirection plugin, read from its JSON export.

use std::fs;

use khalid_import::wordpress::{
    Redirection, RedirectionError, RuleTarget, SkipReason, read_redirection,
};

#[cfg(test)]
fn fixture() -> Redirection {
    let dir = env!("CARGO_MANIFEST_DIR");
    let text = fs::read_to_string(format!("{dir}/fixtures/wordpress/redirection.json")).unwrap();
    read_redirection(&text).unwrap()
}

#[test]
fn carries_every_rule_khalid_can_answer() {
    let rules: Vec<String> = fixture()
        .rules
        .iter()
        .map(|rule| match (&rule.from, &rule.to) {
            (from, RuleTarget::Moved { to, status }) => format!("{from:?} {to} {status:?}"),
            (from, to) => format!("{from:?} {to:?}"),
        })
        .collect();
    assert_eq!(
        rules,
        [
            r#"Path("/bread-recipes/") /recipes/ Permanent"#,
            r#"Pattern("^/(\\d{4})/(\\d{2})/(.*)$") /blog/$3 Permanent"#,
            r#"Path("/summer-sale/") /offers/ Found"#,
            r#"Path("/opening-hours/") https://northfield.example.com/visit/ Temporary"#,
            r#"Path("/rye/") /rye-bread/ Permanent"#,
            r#"Path("/gluten-free-myths/") Gone"#,
            r#"Path("/court-order/") Unavailable"#,
            r#"Path("/flour-guide/") /flour/ Permanent"#,
        ]
    );
}

#[test]
fn lists_every_rule_it_leaves_behind_with_the_reason() {
    let redirection = fixture();
    let skipped: Vec<(u64, &str, &SkipReason)> = redirection
        .skipped
        .iter()
        .map(|rule| (rule.id, rule.url.as_str(), &rule.reason))
        .collect();
    assert_eq!(
        skipped,
        [
            (8, "/old-menu/", &SkipReason::AlreadyNotFound),
            (
                9,
                "/members/",
                &SkipReason::Conditional {
                    match_type: "login".to_owned()
                }
            ),
            (10, "/draft-page/", &SkipReason::Disabled),
            (11, "/shop/", &SkipReason::Disabled),
            (
                12,
                "/ping/",
                &SkipReason::NotARedirect {
                    action: "nothing".to_owned()
                }
            ),
        ]
    );
}

#[test]
fn a_rule_with_an_unknown_status_is_left_behind() {
    let text = r#"{"redirects":[{"id":1,"url":"/teapot/","action_code":418,"action_type":"error","match_type":"url","regex":false,"group_id":1,"enabled":true}]}"#;
    let redirection = read_redirection(text).unwrap();
    assert_eq!(redirection.rules.len(), 0);
    assert_eq!(
        redirection.skipped[0].reason,
        SkipReason::Status { code: 418 }
    );
}

#[test]
fn a_redirect_with_no_target_is_left_behind() {
    let text = r#"{"redirects":[{"id":1,"url":"/a/","action_code":301,"action_type":"url","action_data":{"url":""},"match_type":"url","enabled":true}]}"#;
    let redirection = read_redirection(text).unwrap();
    assert_eq!(redirection.skipped[0].reason, SkipReason::NoTarget);
}

#[test]
fn refuses_a_file_that_is_not_a_redirection_export() {
    for text in [
        "",
        "{",
        r#"{"posts":[]}"#,
        r#"{"redirects":[{"id":"one"}]}"#,
        r#"{"redirects":[{"id":1,"url":"/a/","action_code":"moved","action_type":"url","match_type":"url","enabled":true}]}"#,
    ] {
        let error = read_redirection(text).unwrap_err();
        assert!(
            matches!(error, RedirectionError::NotRedirection { .. }),
            "{text}"
        );
    }
}
