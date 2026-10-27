//! Requests to a bucket are signed with AWS Signature Version 4, checked
//! against AWS's own published test suite.

use chrono::{TimeZone, Utc};
use khalid_core::Environment;
use khalid_media::SigV4;

/// AWS's published example keys from its signing test suite
/// (aws-c-auth, `tests/aws-signing-test-suite/v4/get-vanilla`). They are
/// documentation values that open nothing.
#[cfg(test)]
fn example_signer(service: &str) -> SigV4 {
    let env = Environment::from_pairs(&[
        (Environment::S3_ACCESS_KEY_ID, "AKIDEXAMPLE"),
        (
            Environment::S3_SECRET_ACCESS_KEY,
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
        ),
    ]);
    SigV4::new(env.s3_keys().unwrap(), "us-east-1", service)
}

/// SHA-256 of no bytes, in hex.
const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn matches_the_get_vanilla_case_of_the_aws_test_suite() {
    let at = Utc.with_ymd_and_hms(2015, 8, 30, 12, 36, 0).unwrap();
    let uri = "https://example.amazonaws.com/".parse().unwrap();
    let signed = example_signer("service")
        .sign("GET", &uri, &[], EMPTY, at)
        .unwrap();
    assert_eq!(signed.amz_date, "20150830T123600Z");
    assert_eq!(
        signed.authorization,
        "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
         SignedHeaders=host;x-amz-date, \
         Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
    );
}

#[test]
fn signs_every_header_it_is_given_in_order_and_the_port() {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 8, 0, 0).unwrap();
    let uri = "http://127.0.0.1:9000/field-notes-media/blog/3f2a9c1e0b57d0aa/crm.pdf"
        .parse()
        .unwrap();
    let headers = [
        ("x-amz-content-sha256", EMPTY),
        ("content-type", "application/pdf"),
    ];
    let s3 = example_signer("s3");
    let signed = s3.sign("PUT", &uri, &headers, EMPTY, at).unwrap();
    assert!(
        signed.authorization.contains(
            "/20261027/us-east-1/s3/aws4_request, \
             SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date, "
        ),
        "{}",
        signed.authorization
    );
    let other_port = "http://127.0.0.1:9001/field-notes-media/blog/3f2a9c1e0b57d0aa/crm.pdf"
        .parse()
        .unwrap();
    let moved = s3.sign("PUT", &other_port, &headers, EMPTY, at).unwrap();
    assert_ne!(
        moved.authorization, signed.authorization,
        "the port is signed"
    );
}

#[test]
fn the_signer_never_prints_its_keys() {
    let shown = format!("{:?}", example_signer("s3"));
    assert!(!shown.contains("EXAMPLEKEY"), "{shown}");
    assert!(!shown.contains("AKIDEXAMPLE"), "{shown}");
}
