//! The SEO fields that plugins keep in a post's custom fields: the title
//! and description for search results, the canonical address, the robots
//! directives, the focus keywords and the primary category.

use khalid_core::{AbsoluteUrl, RobotsDirectives};

use crate::wordpress::seo_text::{Syntax, Vars, fill};
use crate::wordpress::{Item, Site};

/// An SEO plugin whose fields the importer reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SeoPlugin {
    /// Yoast SEO.
    Yoast,
}

/// What the SEO plugins say about one post.
///
/// A site that changed plugins can carry fields from several. Each field
/// comes from the first plugin, in [`SeoPlugin`] order, that set it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SeoMeta {
    /// The title for search results, with its variables filled in.
    pub title: Option<String>,
    /// The description for search results, with its variables filled in.
    pub description: Option<String>,
    /// The address search engines should credit instead of the post.
    pub canonical: Option<AbsoluteUrl>,
    /// The robots directives, when they differ from the defaults.
    pub robots: Option<RobotsDirectives>,
    /// The focus keywords, the main one first.
    pub focus_keywords: Vec<String>,
    /// The id of the category breadcrumbs should show.
    pub primary_category: Option<u64>,
    /// Fields the plugins set that Khalid cannot carry over as they are,
    /// so the report can list them.
    pub unread: Vec<UnreadField>,
}

/// A plugin field that could not be carried over.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnreadField {
    /// The plugin that wrote it.
    pub plugin: SeoPlugin,
    /// The custom field, such as `_yoast_wpseo_canonical`.
    pub key: String,
    /// Its value as written.
    pub value: String,
}

/// Where one plugin keeps each field.
struct Fields {
    plugin: SeoPlugin,
    syntax: Syntax,
    title: &'static [&'static str],
    description: &'static [&'static str],
    canonical: &'static [&'static str],
    focus: &'static [&'static str],
    primary_category: &'static [&'static str],
    /// The field named when the robots words cannot be read.
    robots_key: &'static str,
    robots: fn(&Item) -> Vec<&str>,
}

const PLUGINS: [Fields; 1] = [Fields {
    plugin: SeoPlugin::Yoast,
    syntax: Syntax::Double,
    title: &["_yoast_wpseo_title"],
    description: &["_yoast_wpseo_metadesc"],
    canonical: &["_yoast_wpseo_canonical"],
    focus: &["_yoast_wpseo_focuskw"],
    primary_category: &["_yoast_wpseo_primary_category"],
    robots_key: "_yoast_wpseo_meta-robots-noindex",
    robots: yoast_robots,
}];

impl SeoMeta {
    /// Reads what the SEO plugins stored for `item`, on `site`.
    pub fn read(item: &Item, site: &Site) -> Self {
        let vars = Vars {
            title: &item.title,
            site: &site.title,
            excerpt: &item.excerpt,
        };
        let mut seo = Self::default();
        for fields in &PLUGINS {
            seo.take(item, fields, &vars);
        }
        seo
    }

    /// Fills each field still empty from one plugin's custom fields.
    fn take(&mut self, item: &Item, fields: &Fields, vars: &Vars<'_>) {
        let plugin = fields.plugin;
        if self.title.is_none() {
            self.title = self.text(item, fields, fields.title, vars);
        }
        if self.description.is_none() {
            self.description = self.text(item, fields, fields.description, vars);
        }
        if self.canonical.is_none()
            && let Some((key, value)) = set(item, fields.canonical)
        {
            self.canonical = AbsoluteUrl::parse(value).ok();
            if self.canonical.is_none() {
                self.unread(plugin, key, value);
            }
        }
        if self.focus_keywords.is_empty()
            && let Some((_, value)) = set(item, fields.focus)
        {
            let words = value.split(',').map(str::trim).filter(|w| !w.is_empty());
            self.focus_keywords = words.map(str::to_owned).collect();
        }
        if self.primary_category.is_none()
            && let Some((_, value)) = set(item, fields.primary_category)
        {
            // Plugins write "none" or nothing when no category was chosen.
            self.primary_category = value.parse().ok();
        }
        let words = (fields.robots)(item);
        if self.robots.is_none() && !words.is_empty() {
            let text = words.join(", ");
            match text.parse::<RobotsDirectives>() {
                Ok(robots) if robots == RobotsDirectives::default() => {}
                Ok(robots) => self.robots = Some(robots),
                Err(_) => self.unread(plugin, fields.robots_key, &text),
            }
        }
    }

    /// A title or description with its variables filled in, or none, with
    /// the field listed as unread when a variable cannot be filled.
    fn text(
        &mut self,
        item: &Item,
        fields: &Fields,
        keys: &[&'static str],
        vars: &Vars<'_>,
    ) -> Option<String> {
        let (key, value) = set(item, keys)?;
        let filled = fill(value, fields.syntax, vars);
        if filled.is_none() {
            self.unread(fields.plugin, key, value);
        }
        filled.filter(|text| !text.is_empty())
    }

    fn unread(&mut self, plugin: SeoPlugin, key: &str, value: &str) {
        self.unread.push(UnreadField {
            plugin,
            key: key.to_owned(),
            value: value.to_owned(),
        });
    }
}

/// The first of `keys` the item sets to something, with its value trimmed.
fn set<'item>(item: &'item Item, keys: &[&'static str]) -> Option<(&'static str, &'item str)> {
    keys.iter()
        .find_map(|key| item.meta(key).map(|value| (*key, value.trim())))
        .filter(|(_, value)| !value.is_empty())
}

/// Yoast writes 1 for noindex and 2 for an explicit index, 1 for nofollow,
/// and the rest as a list such as `noarchive,nosnippet`.
fn yoast_robots(item: &Item) -> Vec<&str> {
    let mut words = Vec::new();
    match item.meta("_yoast_wpseo_meta-robots-noindex") {
        Some("1") => words.push("noindex"),
        Some("2") => words.push("index"),
        _ => {}
    }
    if item.meta("_yoast_wpseo_meta-robots-nofollow") == Some("1") {
        words.push("nofollow");
    }
    if let Some(advanced) = item.meta("_yoast_wpseo_meta-robots-adv") {
        let listed = advanced.split(',').map(str::trim);
        words.extend(listed.filter(|word| !["", "none", "-"].contains(word)));
    }
    words
}
