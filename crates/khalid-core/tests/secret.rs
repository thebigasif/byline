//! A secret never shows itself when formatted, alone or inside other values.

use khalid_core::Secret;

/// A value made for this test. It is not, and never was, a real credential.
fn generated() -> String {
    format!("test-token-{}", std::process::id())
}

#[test]
fn debug_and_display_print_redacted() {
    let value = generated();
    let secret = Secret::new(value.clone());
    for shown in [
        format!("{secret:?}"),
        format!("{secret:#?}"),
        format!("{secret}"),
    ] {
        assert_eq!(shown, "[redacted]");
        assert!(!shown.contains(&value));
    }
}

#[test]
fn a_secret_inside_a_logged_struct_stays_hidden() {
    #[derive(Debug)]
    struct Connection {
        site: &'static str,
        token: Secret<String>,
    }
    let value = generated();
    let connection = Connection {
        site: "serpdad.com",
        token: Secret::new(value.clone()),
    };
    assert_eq!(connection.site, "serpdad.com");
    let logged = format!("{connection:?}");
    assert!(logged.contains("serpdad.com"), "{logged}");
    assert!(logged.contains("[redacted]"), "{logged}");
    assert!(!logged.contains(&value), "{logged}");
    assert_eq!(connection.token.expose(), &value);
}
