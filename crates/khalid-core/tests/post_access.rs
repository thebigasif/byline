//! Access, template names and password hashes read only good values.

use khalid_core::{PasswordHash, PostAccess, TemplateName};

#[test]
fn access_reads_as_it_is_written_and_defaults_to_public() {
    assert_eq!(PostAccess::default(), PostAccess::Public);
    for access in PostAccess::ALL {
        let json = serde_json::to_value(access).unwrap();
        assert_eq!(json, access.as_str());
        assert_eq!(serde_json::from_value::<PostAccess>(json).unwrap(), access);
    }
    assert!(serde_json::from_str::<PostAccess>("\"paid\"").is_err());
}

#[test]
fn a_template_name_stays_inside_the_templates_folder() {
    let wide = TemplateName::try_new("wide-2").unwrap();
    assert_eq!(wide.file_name(), "post-wide-2.html");
    for bad in [
        "", "../post", "Wide", "a b", "-a", "a-", "a--b", "a/b", "a.html",
    ] {
        assert!(
            TemplateName::try_new(bad).is_err(),
            "{bad:?} was let through"
        );
    }
    assert!(TemplateName::try_new(&"a".repeat(TemplateName::MAX_LEN + 1)).is_err());
    let message = TemplateName::try_new("Wide").unwrap_err().to_string();
    assert!(message.contains("post-wide.html"), "{message}");
}

#[test]
fn a_password_hash_must_be_argon2id() {
    let phc = "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$aGFzaGhhc2g";
    assert_eq!(PasswordHash::try_new(phc).unwrap().as_str(), phc);
    for bad in ["hunter2", "$argon2i$v=19$x", "$argon2id$", "$argon2id$a b"] {
        assert!(
            PasswordHash::try_new(bad).is_err(),
            "{bad:?} was let through"
        );
    }
}
