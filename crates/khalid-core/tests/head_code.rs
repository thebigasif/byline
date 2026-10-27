//! Head code keeps what a page head may hold and refuses the rest.

use khalid_core::{HeadCode, HeadCodeError};

#[cfg(test)]
fn refused(html: &str) -> HeadCodeError {
    HeadCode::try_new(html).unwrap_err()
}

#[test]
fn an_analytics_snippet_is_kept_and_gets_the_nonce() {
    let pasted = "<!-- Google tag -->\n<script async src='https://www.googletagmanager.com/gtag/js?id=G-1'></script>\n<SCRIPT>gtag('config', 'G-1');</SCRIPT>";
    let code = HeadCode::try_new(pasted).unwrap();
    assert_eq!(
        code.to_string(),
        "<script async src=\"https://www.googletagmanager.com/gtag/js?id=G-1\"></script>\n<script>gtag('config', 'G-1');</script>"
    );
    assert_eq!(
        code.to_html("abc"),
        "<script async src=\"https://www.googletagmanager.com/gtag/js?id=G-1\" nonce=\"abc\"></script>\n<script nonce=\"abc\">gtag('config', 'G-1');</script>"
    );
}

#[test]
fn the_checked_form_reads_back_the_same() {
    let code = HeadCode::try_new("<meta name=verify content=x><link rel=me href=\"https://example.com/\"><style>p{color:red}</style>").unwrap();
    let again = HeadCode::try_new(&code.to_string()).unwrap();
    assert_eq!(again, code);
}

#[test]
fn quotes_and_brackets_in_values_cannot_open_a_tag() {
    let code = HeadCode::try_new("<meta name='a\"><script>x</script>' content=y>").unwrap();
    assert_eq!(
        code.to_string(),
        "<meta name=\"a&quot;&gt;&lt;script&gt;x&lt;/script&gt;\" content=\"y\">"
    );
}

#[test]
fn a_pasted_nonce_is_dropped() {
    let code = HeadCode::try_new("<script nonce=old>x()</script>").unwrap();
    assert_eq!(code.to_html("new"), "<script nonce=\"new\">x()</script>");
}

#[test]
fn what_could_break_or_take_the_page_away_is_refused() {
    let element = |name: &str| HeadCodeError::Element {
        name: name.to_owned(),
    };
    let cases = [
        ("<div>hi</div>", element("div")),
        ("<base href=/x/>", element("base")),
        ("</head><body>", element("</head>")),
        ("<noscript><img src=x></noscript>", element("noscript")),
        (
            "hello",
            HeadCodeError::Text {
                text: "hello".to_owned(),
            },
        ),
        (
            "<meta http-equiv=refresh content='0;url=/x'>",
            HeadCodeError::HttpEquiv,
        ),
        (
            "<link href=a onload=steal()>",
            HeadCodeError::EventAttribute {
                name: "onload".to_owned(),
            },
        ),
        ("<script>x()", HeadCodeError::Unclosed { name: "script" }),
        ("<!-- open", HeadCodeError::Unclosed { name: "comment" }),
        (
            "<meta name='x>",
            HeadCodeError::Unclosed {
                name: "attribute value",
            },
        ),
        (
            "<script><!--</script>",
            HeadCodeError::CommentInCode { name: "script" },
        ),
    ];
    for (html, want) in cases {
        assert_eq!(refused(html), want, "{html}");
    }
    let long = format!("<script>{}</script>", "x".repeat(HeadCode::MAX_LEN));
    assert!(matches!(refused(&long), HeadCodeError::TooLong { .. }));
    assert!(HeadCode::try_new(" <!-- later -->\n").unwrap().is_empty());
}
