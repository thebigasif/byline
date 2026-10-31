//! What every page of one language shows around its content: the site's
//! name, stylesheet and reader script, from `site.toml`.

use khalid_core::{Config, Locale};
use khalid_render::{Nav, RenderSite};

use crate::build_pages::path_of;
use crate::{assets, build_reader};

/// The site as pages in `locale` show it, linking the stylesheet at
/// `stylesheet` and the reader script when a feature on uses it.
pub(crate) fn render_site(config: &Config, locale: &Locale, stylesheet: &str) -> RenderSite {
    let features = build_reader::features(config.reader());
    RenderSite {
        name: config.name().to_owned(),
        locale: locale.clone(),
        home: path_of(config, locale, ""),
        stylesheet: stylesheet.to_owned(),
        nav: Nav::default(),
        main_site: None,
        share_image: None,
        script: assets::script_tag(config, &features),
        features,
        search: None,
    }
}
