//! `[media]` in `site.toml`: files on disk by default, or in an
//! S3-compatible bucket whose keys come from the environment.

use std::path::Path;

use khalid_core::{Config, Environment};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
default_language = "en"
languages = ["en"]
"#;

const BUCKET: &str = r#"
[media.s3]
endpoint = "https://0c4a2b.r2.cloudflarestorage.com"
bucket = "field-notes-media"
prefix = "blog/"
public_address = "https://media.example.com/"
"#;

/// Two values made up for these tests, never a real key.
#[cfg(test)]
fn keys() -> Environment {
    Environment::from_pairs(&[
        (Environment::S3_ACCESS_KEY_ID, "made-for-this-test-id"),
        (
            Environment::S3_SECRET_ACCESS_KEY,
            "made-for-this-test-secret",
        ),
    ])
}

#[cfg(test)]
fn load(text: &str, env: &Environment) -> Result<Config, String> {
    Config::from_toml(Path::new("site.toml"), text, env).map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_table_keeps_files_on_disk() {
    let config = load(SITE, &Environment::empty()).unwrap();
    assert_eq!(config.media().s3, None);
}

#[test]
fn reads_a_bucket_with_its_defaults() {
    let config = load(&format!("{SITE}{BUCKET}"), &keys()).unwrap();
    let s3 = config.media().s3.as_ref().unwrap();
    assert_eq!(
        s3.endpoint.as_str(),
        "https://0c4a2b.r2.cloudflarestorage.com/"
    );
    assert_eq!(s3.bucket, "field-notes-media");
    assert_eq!(s3.region, "auto");
    assert_eq!(s3.prefix, "blog/");
    assert_eq!(s3.public_address.as_str(), "https://media.example.com/");
}

#[test]
fn refuses_a_bucket_without_its_keys() {
    let only_id = Environment::from_pairs(&[(Environment::S3_ACCESS_KEY_ID, "made-up")]);
    for env in [Environment::empty(), only_id] {
        assert_eq!(
            load(&format!("{SITE}{BUCKET}"), &env).unwrap_err(),
            "site.toml, line 8, field \"bucket\": a bucket needs KHALID_S3_ACCESS_KEY_ID and KHALID_S3_SECRET_ACCESS_KEY set in the environment"
        );
    }
}

#[test]
fn refuses_a_bucket_name_s3_would_refuse() {
    let text = format!("{SITE}{BUCKET}").replace("field-notes-media", "Field_Notes");
    assert_eq!(
        load(&text, &keys()).unwrap_err(),
        "site.toml, line 8, field \"bucket\": a bucket name is 3 to 63 lowercase letters, digits and hyphens"
    );
}

#[test]
fn refuses_a_prefix_that_is_not_a_folder() {
    for prefix in ["blog", "/blog/", "../blog/"] {
        let text = format!("{SITE}{BUCKET}").replace("\"blog/\"", &format!("{prefix:?}"));
        assert_eq!(
            load(&text, &keys()).unwrap_err(),
            "site.toml, line 9, field \"prefix\": the prefix is empty, or a folder name ending in /, such as blog/",
            "prefix {prefix}"
        );
    }
}

#[test]
fn refuses_a_bucket_with_no_public_address() {
    let text =
        format!("{SITE}{BUCKET}").replace("public_address = \"https://media.example.com/\"\n", "");
    let message = load(&text, &keys()).unwrap_err();
    assert!(
        message.contains("missing field `public_address`"),
        "{message}"
    );
}

#[test]
fn the_keys_come_from_the_environment_and_never_print() {
    let env = keys();
    let shown = format!("{env:?}");
    assert!(!shown.contains("made-for-this-test-secret"), "{shown}");
    assert!(shown.contains("made-for-this-test-id"), "{shown}");
    let found = env.s3_keys().unwrap();
    assert_eq!(
        found.secret_access_key.expose(),
        "made-for-this-test-secret"
    );
    assert_eq!(format!("{found:?}").matches("[redacted]").count(), 2);
}
