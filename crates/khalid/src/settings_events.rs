//! The `site.changed` webhook `khalid serve` fires as `site.toml` is saved
//! (audit brief G25). The server reads its settings once at start, so a
//! change counts from the next start; the body says so, and names the
//! settings that changed without their values, which may be private.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use khalid_webhooks::Event;
use serde::Serialize;

use crate::announce::SiteAnnounce;

/// The file the settings live in, at the top of the site folder.
const FILE: &str = "site.toml";

/// The settings in `site.toml` as one render read them: `None` when the
/// file could not be read or is not TOML, such as while it is half saved.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Settings(Option<toml::Table>);

/// The body of `site.changed`, under `settings`.
#[derive(Debug, Serialize)]
struct Body {
    file: &'static str,
    /// The top-level settings and tables that changed, such as `name` or
    /// `seo`, in name order.
    changed: Vec<String>,
    /// Always `true`: a running server keeps the settings it started with.
    restart_needed: bool,
}

impl Settings {
    /// The settings in the site `folder`'s `site.toml`. Reads the file, so
    /// it runs where the content is read.
    pub(crate) fn read(folder: &Path) -> Self {
        let mut file = folder.to_path_buf();
        file.push(FILE);
        let text = fs::read_to_string(&file).ok();
        Self(text.and_then(|text| text.parse().ok()))
    }

    /// The settings to compare the next render with: `now` when it was
    /// read, else these, so a broken save is skipped and the next good one
    /// is compared with the last good one.
    pub(crate) fn latest(self, now: Self) -> Self {
        if now.0.is_some() { now } else { self }
    }

    /// Fires `site.changed` on `announce` when some setting differs in
    /// `self` since `before`.
    pub(crate) fn fire_since(&self, before: &Self, announce: &SiteAnnounce) {
        let changed = self.changed_since(before);
        if changed.is_empty() {
            return;
        }
        let body = Body {
            file: FILE,
            changed,
            restart_needed: true,
        };
        announce.fire(Event::SiteChanged, BTreeMap::from([("settings", body)]));
    }

    /// The names of the top-level settings that were added, changed or
    /// removed in `self` since `before`, or none when either side was not
    /// read.
    fn changed_since(&self, before: &Self) -> Vec<String> {
        let (Some(now), Some(was)) = (&self.0, &before.0) else {
            return Vec::new();
        };
        let names: BTreeSet<&String> = now.keys().chain(was.keys()).collect();
        names
            .into_iter()
            .filter(|name| now.get(*name) != was.get(*name))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(text: &str) -> Settings {
        Settings(text.parse().ok())
    }

    const SITE: &str =
        "name = \"Field Notes\"\n[seo]\nindexnow = false\n[mail]\nfrom = \"a@example.com\"\n";

    #[test]
    fn the_demo_settings_are_read() {
        let mut folder = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        folder.push("../../fixtures/demo-site");
        let read = Settings::read(&folder);
        assert!(read.0.is_some_and(|table| table.contains_key("name")));
        let missing = tempfile::tempdir().unwrap();
        assert_eq!(Settings::read(missing.path()), Settings::default());
    }

    #[test]
    fn changed_names_each_added_changed_or_removed_setting_only() {
        let before = settings(SITE);
        let now = settings(
            "name = \"Field Notes\"\n# A comment.\n[seo]\nindexnow = true\n[search]\non = true\n",
        );
        assert_eq!(now.changed_since(&before), ["mail", "search", "seo"]);
        assert_eq!(before.changed_since(&settings(SITE)), Vec::<String>::new());
    }

    #[test]
    fn a_broken_save_changes_nothing_and_is_skipped() {
        let before = settings(SITE);
        let broken = settings("name = \"Field");
        assert_eq!(broken.changed_since(&before), Vec::<String>::new());
        let kept = before.clone().latest(broken);
        assert_eq!(kept, before);
        let fixed = settings("name = \"Field Notes 2\"\n");
        assert_eq!(fixed.changed_since(&kept), ["mail", "name", "seo"]);
    }

    #[test]
    fn the_body_names_the_file_and_the_settings_but_no_value() {
        let body = Body {
            file: FILE,
            changed: vec!["mail".to_owned()],
            restart_needed: true,
        };
        let json = serde_json::to_value(BTreeMap::from([("settings", body)])).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"settings": {"file": "site.toml", "changed": ["mail"], "restart_needed": true}})
        );
    }
}
