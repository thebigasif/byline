//! The doctor check for photo and GIF search in the editor: each is off, on
//! with its key, or on without it (decision E14 in
//! `docs/decisions/2026-10-31-editor-library-pictures-and-crop.md`). It only
//! looks at whether a key is set; it never sends one anywhere.

use khalid_core::{Config, Environment, UserMessage};

use crate::doctor_check::{Check, Finding};

/// One of the two searches, as doctor names it.
struct Search {
    /// Its name as writers know it.
    name: &'static str,
    /// What a writer looks for there.
    thing: &'static str,
    /// The `[media]` key that turns it on.
    setting: &'static str,
    /// The environment variable that holds its key.
    variable: &'static str,
    /// What the key is, for the fix.
    key: &'static str,
}

const UNSPLASH: Search = Search {
    name: "Photo search on Unsplash",
    thing: "photo",
    setting: "unsplash",
    variable: Environment::UNSPLASH_ACCESS_KEY,
    key: "the access key of an Unsplash app",
};

const GIPHY: Search = Search {
    name: "GIF search on GIPHY",
    thing: "GIF",
    setting: "gifs",
    variable: Environment::GIPHY_API_KEY,
    key: "the API key of a GIPHY app",
};

/// Checks each search `config` turns on for its key in `env`. Skipped when
/// the settings did not load.
pub(crate) fn check(config: Option<&Config>, env: &Environment) -> Finding {
    let Some(config) = config else {
        return Finding::skipped(
            Check::Pictures,
            "Khalid reads which picture searches are on from the settings, so fix those first.",
        );
    };
    let media = config.media();
    let searches = [
        (UNSPLASH, media.unsplash, env.unsplash_key().is_some()),
        (GIPHY, media.gifs, env.giphy_key().is_some()),
    ];
    if let Some((search, ..)) = searches.iter().find(|(_, on, key)| *on && !*key) {
        return Finding::problem(Check::Pictures, missing(search));
    }
    if !media.unsplash && !media.gifs {
        return Finding::ok(
            Check::Pictures,
            format!(
                "{} and {} are off. Turn them on with unsplash = true or gifs = true under [media] in site.toml.",
                UNSPLASH.name, GIPHY.name
            ),
        );
    }
    let said: Vec<String> = searches
        .iter()
        .map(|(search, on, _)| {
            if *on {
                format!("{} is on, with its key.", search.name)
            } else {
                format!("{} is off.", search.name)
            }
        })
        .collect();
    Finding::ok(Check::Pictures, said.join(" "))
}

/// The problem for `search` turned on without its key.
fn missing(search: &Search) -> UserMessage {
    UserMessage::new(
        format!("{} is on, but {} is not set.", search.name, search.variable),
        format!(
            "Writers who look for a {} see a message instead of {}s.",
            search.thing, search.thing
        ),
        format!(
            "Set {} to {}, or set {} = false under [media] in site.toml.",
            search.variable, search.key, search.setting
        ),
    )
}
