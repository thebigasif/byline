//! A bucket keeps the files' bytes when `[media.s3]` is set. These tests
//! talk to a fake S3 on this machine, never the network (RULES.md
//! BL-TST-3).

use std::path::Path;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use khalid_core::{Config, Environment, FixedClock, Locale};
use khalid_media::{ContentKey, FileAddress, FileName, MediaError, MediaType, Storage};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use wiremock::matchers::{body_bytes, header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PDF: &[u8] = b"%PDF-1.7\nA checklist for choosing a CRM\n%%EOF\n";

/// The object the PDF is stored as, under the bucket and its prefix.
const OBJECT: &str = "/field-notes-media/blog/9f6a0d0c8c7c2b1e/crm-checklist.pdf";

/// A bucket on `server`, with keys made up for this test.
#[cfg(test)]
fn bucket(server: &MockServer) -> Storage {
    let text = format!(
        "name = \"Field Notes\"\naddress = \"https://www.example.com\"\n\
         default_language = \"en\"\nlanguages = [\"en\"]\n\n[media.s3]\n\
         endpoint = \"{}\"\nbucket = \"field-notes-media\"\nprefix = \"blog/\"\n\
         public_address = \"https://media.example.com/\"\n",
        server.uri()
    );
    let env = Environment::from_pairs(&[
        (Environment::S3_ACCESS_KEY_ID, "made-up-id"),
        (Environment::S3_SECRET_ACCESS_KEY, "made-up-secret"),
    ]);
    let config = Config::from_toml(Path::new("site.toml"), &text, &env).unwrap();
    let settings = config.media().s3.clone().unwrap();
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 8, 0, 0).unwrap());
    Storage::s3(&settings, env.s3_keys().unwrap(), outbound, Arc::new(clock))
}

#[cfg(test)]
fn file() -> (ContentKey, FileName) {
    let english = Locale::try_new("en").unwrap();
    let name = FileName::new("CRM checklist.pdf", MediaType::Pdf, &english);
    (ContentKey::parse("9f6a0d0c8c7c2b1e").unwrap(), name)
}

#[tokio::test]
async fn puts_a_signed_file_in_the_bucket() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(OBJECT))
        .and(body_bytes(PDF))
        .and(header("content-type", "application/pdf"))
        .and(header("x-amz-date", "20261027T080000Z"))
        .and(header_regex("x-amz-content-sha256", "^[0-9a-f]{64}$"))
        .and(header_regex(
            "authorization",
            "^AWS4-HMAC-SHA256 Credential=made-up-id/20261027/auto/s3/aws4_request, \
             SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date, \
             Signature=[0-9a-f]{64}$",
        ))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let (key, name) = file();
    bucket(&server)
        .put(&key, &name, MediaType::Pdf, PDF.to_vec())
        .await
        .unwrap();
}

#[tokio::test]
async fn reads_a_file_back_and_a_missing_one_as_none() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(OBJECT))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(PDF))
        .mount(&server)
        .await;
    let (key, name) = file();
    let storage = bucket(&server);
    assert_eq!(storage.get(&key, &name).await.unwrap(), Some(PDF.to_vec()));
    let other = ContentKey::parse("0000000000000000").unwrap();
    assert_eq!(storage.get(&other, &name).await.unwrap(), None);
}

#[tokio::test]
async fn a_bucket_that_refuses_says_so_with_its_status() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let (key, name) = file();
    let error = bucket(&server)
        .put(&key, &name, MediaType::Pdf, PDF.to_vec())
        .await
        .unwrap_err();
    assert!(
        matches!(error, MediaError::Bucket { status: 403 }),
        "{error:?}"
    );
    assert_eq!(error.to_string(), "the bucket answered with status 403");
}

#[tokio::test]
async fn readers_fetch_a_bucket_file_from_its_public_address() {
    let server = MockServer::start().await;
    let (key, name) = file();
    assert_eq!(
        bucket(&server).address(&key, &name),
        FileAddress::Elsewhere(
            "https://media.example.com/blog/9f6a0d0c8c7c2b1e/crm-checklist.pdf".to_owned()
        )
    );
}
