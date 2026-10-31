//! The generated files in `sdk/typescript/` match `openapi.json` (task
//! 1.20, KH-API-4). When the document changes, this test rewrites the files
//! and fails once, so a local run fixes them and CI, which starts from the
//! committed files, goes red until they are committed.

use std::fs;
use std::path::PathBuf;

/// The SDK folder, relative to this crate.
#[cfg(test)]
fn sdk() -> PathBuf {
    let mut sdk = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    sdk.push("../../sdk/typescript");
    sdk
}

#[test]
fn the_generated_sdk_files_match_openapi_json() {
    let mut document = sdk();
    document.push("openapi.json");
    let document = fs::read_to_string(document).unwrap();
    let mut stale = Vec::new();
    for file in khalid_sdk::generate(&document).unwrap() {
        let mut path = sdk();
        path.push(file.path);
        if fs::read_to_string(&path).unwrap_or_default() != file.contents {
            fs::write(&path, &file.contents).unwrap();
            stale.push(file.path);
        }
    }
    assert!(
        stale.is_empty(),
        "sdk/typescript/{stale:?} did not match openapi.json, so this test rewrote them. \
         Commit the new files."
    );
}

#[test]
fn the_sdk_can_list_posts_with_a_content_key() {
    let mut routes = sdk();
    routes.push("src/operations.js");
    let routes = fs::read_to_string(routes).unwrap();
    let want = "  browse_posts: { method: \"GET\", path: \"/api/content/v1/posts/\", key: \"content\", body: null },\n";
    assert!(routes.contains(want), "{routes}");
}
