//! The TypeScript SDK is generated from `sdk/typescript/openapi.json`
//! (task 1.20, KH-API-4). This test keeps that file equal to the document
//! the API serves, so the SDK can never drift from the routes.
//!
//! When the file is out of date the test rewrites it and fails, so a local
//! run fixes it and CI, which starts from the committed file, goes red.

mod common;

use std::sync::Arc;

use khalid_api::api;
use khalid_api::keys::MemoryKeyStore;
use khalid_api::post_store::MemoryPostStore;

/// The document the SDK is generated from, relative to the repository.
const DOCUMENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../sdk/typescript/openapi.json"
);

/// The document the API serves, as pretty JSON ending in a newline.
#[cfg(test)]
fn served() -> String {
    let (_, document) = api::router(
        Arc::new(common::store()),
        Arc::new(MemoryPostStore::default()),
        Arc::new(MemoryKeyStore::new(Vec::new())),
    );
    let mut json = serde_json::to_string_pretty(&document).unwrap();
    json.push('\n');
    json
}

#[test]
fn the_sdk_document_is_the_one_the_api_serves() {
    let served = served();
    let committed = std::fs::read_to_string(DOCUMENT).unwrap_or_default();
    if committed != served {
        std::fs::write(DOCUMENT, &served).unwrap();
        panic!(
            "sdk/typescript/openapi.json was out of date, so this test rewrote it. \
             The TypeScript SDK is generated from it. \
             Regenerate the SDK as sdk/typescript/README.md says, then commit both."
        );
    }
}

#[test]
fn the_sdk_document_lists_the_posts_route() {
    let served = served();
    assert!(served.contains("\"/api/content/v1/posts/\""), "{served}");
}
