//! A site's settings as every command reads them: `site.toml` through
//! `khalid-core`, then its SEO values through `khalid-seo`'s rules, so a
//! wrong value stops the command before it runs (BL-RUN-2).

use std::fs;
use std::path::Path;

use khalid_core::{Config, ConfigError, Environment, UserMessage};
use khalid_seo::SeoSite;

use crate::{config_message, seo_settings};

/// Reads and checks the settings of the `site` folder, with `environment`
/// on top.
///
/// # Errors
///
/// The message for the first mistake in `site.toml`, with its line.
pub(crate) fn load(site: &Path, environment: &Environment) -> Result<Config, UserMessage> {
    let config = Config::load(site, environment).map_err(|error| config_message::of(&error))?;
    check(site, &config)?;
    Ok(config)
}

/// Checks the SEO values of `config`, read from the `site` folder, with
/// `khalid-seo`'s rules.
///
/// # Errors
///
/// The message for the first value refused, with its line in `site.toml`.
pub(crate) fn check(site: &Path, config: &Config) -> Result<(), UserMessage> {
    let mut seo = SeoSite::new(
        config.name(),
        config.address().clone(),
        config.default_language().clone(),
    );
    let Err(mistake) = seo_settings::apply(config.seo(), &mut seo) else {
        return Ok(());
    };
    let mut path = site.to_path_buf();
    path.push(Config::FILE_NAME);
    let text = fs::read_to_string(&path).unwrap_or_default();
    let line = mistake.line_in(&text);
    Err(config_message::of(&ConfigError::Field {
        path,
        line,
        field: mistake.key,
        problem: mistake.problem,
    }))
}
