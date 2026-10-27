//! The TypeScript SDK is generated from `sdk/typescript/openapi.json`
//! (task 1.20, KH-API-4). This test keeps that file equal to the document
//! `khalid serve` serves, the Admin API with its staff routes, so the SDK
//! can never drift from the routes. It lives here because only this crate
//! sees both `khalid-api` and the staff routes.
//!
//! When the file is out of date the test rewrites it and fails, so a local
//! run fixes it and CI, which starts from the committed file, goes red.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::MemoryKeyStore;
use khalid_api::post_store::MemoryPostStore;
use khalid_auth::{Accounts, MemoryAccounts, MemorySessions, PasswordHashing, staff_routes};
use khalid_core::FixedClock;

/// The document the SDK is generated from, relative to the repository.
const DOCUMENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../sdk/typescript/openapi.json"
);

/// The document `khalid serve` serves, as pretty JSON ending in a newline.
#[cfg(test)]
fn served() -> String {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let accounts = Accounts::new(
        Arc::new(MemoryAccounts::new()),
        Arc::new(MemorySessions::new()),
        PasswordHashing::fast_for_tests(),
        Arc::new(FixedClock::new(at)),
    );
    let (_, document) = api::router_with_staff(
        Arc::new(MemoryContentStore::new(Vec::new())),
        Arc::new(MemoryPostStore::default()),
        Arc::new(MemoryKeyStore::new(Vec::new())),
        staff_routes(accounts.clone()),
        Arc::new(accounts),
    );
    let mut json = serde_json::to_string_pretty(&document).unwrap();
    json.push('\n');
    json
}

#[test]
fn the_sdk_document_is_the_one_khalid_serve_serves() {
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
fn the_sdk_document_lists_the_posts_route_and_signing_in() {
    let served = served();
    assert!(served.contains("\"/api/content/v1/posts/\""), "{served}");
    assert!(served.contains("\"/api/admin/v1/session/\""), "{served}");
}
