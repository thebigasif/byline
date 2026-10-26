//! The `AbsoluteUrl` rules: http or https, a host, and no credentials.

use khalid_core::{AbsoluteUrl, ParseUrlError};

#[test]
fn accepts_web_addresses_and_keeps_their_text() {
    for (input, want) in [
        ("https://serpdad.com", "https://serpdad.com/"),
        (
            "https://www.example.com/blog/",
            "https://www.example.com/blog/",
        ),
        ("http://localhost:4000/", "http://localhost:4000/"),
        ("HTTPS://Example.COM/Blog", "https://example.com/Blog"),
    ] {
        let url = AbsoluteUrl::parse(input).unwrap();
        assert_eq!(url.as_str(), want, "input {input:?}");
        assert_eq!(url.to_string(), want);
        assert_eq!(input.parse::<AbsoluteUrl>().unwrap(), url);
    }
}

#[test]
fn refuses_text_that_is_not_a_url() {
    assert!(matches!(
        AbsoluteUrl::parse("serpdad.com"),
        Err(ParseUrlError::Invalid { .. })
    ));
    assert!(matches!(
        AbsoluteUrl::parse(""),
        Err(ParseUrlError::Invalid { .. })
    ));
}

#[test]
fn refuses_schemes_other_than_http_and_https() {
    for input in [
        "ftp://example.com/",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "mailto:a@example.com",
    ] {
        assert!(
            matches!(AbsoluteUrl::parse(input), Err(ParseUrlError::Scheme { .. })),
            "input {input:?}"
        );
    }
}

#[test]
fn refuses_credentials_in_the_address() {
    assert_eq!(
        AbsoluteUrl::parse("https://admin:hunter2@example.com/"),
        Err(ParseUrlError::Credentials)
    );
    assert_eq!(
        AbsoluteUrl::parse("https://admin@example.com/"),
        Err(ParseUrlError::Credentials)
    );
}

#[test]
fn a_credential_never_appears_in_the_error() {
    let error = AbsoluteUrl::parse("https://admin:hunter2@example.com/")
        .unwrap_err()
        .to_string();
    assert!(!error.contains("hunter2"), "{error}");
    assert!(!error.contains("admin"), "{error}");
}
