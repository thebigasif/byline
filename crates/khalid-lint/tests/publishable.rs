//! `khalid-lint` and `khalid-core` carry what crates.io needs (KH-LNT-6).
//!
//! `cargo publish --dry-run` in `scripts/check.sh` proves the packages build
//! from the registry copy. This test runs offline, so CI catches a manifest
//! that slips back to `publish = false` or `0.0.0` on every pull request.

const LINT_MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
const CORE_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../khalid-core/Cargo.toml"
));
const WORKSPACE_MANIFEST: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"));

#[cfg(test)]
fn package(manifest: &str) -> toml::Table {
    let parsed: toml::Table = toml::from_str(manifest).unwrap();
    parsed["package"].as_table().unwrap().clone()
}

#[cfg(test)]
fn str_field<'a>(package: &'a toml::Table, key: &str) -> Option<&'a str> {
    package.get(key).and_then(toml::Value::as_str)
}

#[test]
fn both_crates_are_marked_for_publishing() {
    for manifest in [LINT_MANIFEST, CORE_MANIFEST] {
        let package = package(manifest);
        let name = str_field(&package, "name").unwrap();
        assert_eq!(
            package.get("publish").and_then(toml::Value::as_bool),
            Some(true),
            "{name} must say publish = true"
        );
        let version = str_field(&package, "version").unwrap();
        assert_ne!(version, "0.0.0", "{name} needs a real version");
        for key in ["description", "readme", "documentation", "homepage"] {
            assert!(str_field(&package, key).is_some(), "{name} needs {key}");
        }
        assert!(package.contains_key("keywords"), "{name} needs keywords");
        assert!(
            package.contains_key("categories"),
            "{name} needs categories"
        );
    }
}

#[test]
fn both_crates_keep_the_licence_the_adr_chose() {
    assert_eq!(env!("CARGO_PKG_LICENSE"), "AGPL-3.0-only");
    let workspace: toml::Table = toml::from_str(WORKSPACE_MANIFEST).unwrap();
    assert_eq!(
        workspace["workspace"]["package"]["license"].as_str(),
        Some("AGPL-3.0-only")
    );
    for manifest in [LINT_MANIFEST, CORE_MANIFEST] {
        let package = package(manifest);
        assert_eq!(
            package["license"]["workspace"].as_bool(),
            Some(true),
            "the licence comes from the workspace, so the two crates never drift"
        );
    }
}

#[test]
fn khalid_lint_asks_for_the_khalid_core_version_it_is_built_with() {
    let core_version = str_field(&package(CORE_MANIFEST), "version")
        .unwrap()
        .to_owned();
    let workspace: toml::Table = toml::from_str(WORKSPACE_MANIFEST).unwrap();
    let core_dep = &workspace["workspace"]["dependencies"]["khalid-core"];
    assert_eq!(core_dep["version"].as_str(), Some(core_version.as_str()));
    assert_eq!(env!("CARGO_PKG_VERSION"), core_version);
}
