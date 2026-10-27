//! Where each SEO plugin keeps its fields, and how it writes robots settings.

use crate::wordpress::seo_text::Syntax;
use crate::wordpress::{Item, SeoPlugin, php};

/// Robots words, or the custom field that could not be read.
pub(crate) type RobotsWords = Result<Vec<String>, &'static str>;

/// Where one plugin keeps each field, each as the custom fields to try.
#[derive(Debug)]
pub(crate) struct Fields {
    pub(crate) plugin: SeoPlugin,
    pub(crate) syntax: Syntax,
    pub(crate) title: &'static [&'static str],
    pub(crate) description: &'static [&'static str],
    pub(crate) canonical: &'static [&'static str],
    pub(crate) focus: &'static [&'static str],
    pub(crate) primary_category: &'static [&'static str],
    /// The field named when the robots words contradict each other.
    pub(crate) robots_key: &'static str,
    pub(crate) robots: fn(&Item) -> RobotsWords,
}

/// Every plugin, in the order their fields win. Yoast comes last: the other
/// plugins import its fields when a site switches, so its copy is the older.
pub(crate) const PLUGINS: [Fields; 2] = [
    Fields {
        plugin: SeoPlugin::RankMath,
        syntax: Syntax::Single,
        title: &["rank_math_title"],
        description: &["rank_math_description"],
        canonical: &["rank_math_canonical_url"],
        focus: &["rank_math_focus_keyword"],
        primary_category: &["rank_math_primary_category"],
        robots_key: "rank_math_robots",
        robots: rank_math_robots,
    },
    Fields {
        plugin: SeoPlugin::Yoast,
        syntax: Syntax::Double,
        title: &["_yoast_wpseo_title"],
        description: &["_yoast_wpseo_metadesc"],
        canonical: &["_yoast_wpseo_canonical"],
        focus: &["_yoast_wpseo_focuskw"],
        primary_category: &["_yoast_wpseo_primary_category"],
        robots_key: "_yoast_wpseo_meta-robots-noindex",
        robots: yoast_robots,
    },
];

/// Rank Math keeps a list such as `noindex` and `nofollow`, and a map of
/// limits such as `max-snippet` to `160`, both PHP-serialised.
fn rank_math_robots(item: &Item) -> RobotsWords {
    let list = serialised(item, "rank_math_robots")?;
    let limits = serialised(item, "rank_math_advanced_robots")?;
    let list = list.into_iter().map(|(_, word)| word);
    let limits = limits
        .into_iter()
        .map(|(name, value)| format!("{name}:{value}"));
    Ok(list.chain(limits).collect())
}

/// The pairs of a serialised custom field, none when it is not set, or the
/// field's name when it cannot be read.
fn serialised(item: &Item, key: &'static str) -> Result<Vec<(String, String)>, &'static str> {
    match item.meta(key).filter(|text| !text.is_empty()) {
        Some(text) => php::pairs(text).ok_or(key),
        None => Ok(Vec::new()),
    }
}

/// Yoast writes 1 for noindex and 2 for an explicit index, 1 for nofollow,
/// and the rest as a list such as `noarchive,nosnippet`.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every plugin's robots reader has the one signature the table holds"
)]
fn yoast_robots(item: &Item) -> RobotsWords {
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
    Ok(words.into_iter().map(str::to_owned).collect())
}
