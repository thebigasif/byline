//! The AI info page (phase GEO, P3): who runs the site, what it covers,
//! how to cite it and how to reach the team, from the site's own settings.

use std::path::Path;

use chrono::NaiveDate;
use khalid_core::{Config, Locale, UserMessage};
use khalid_render::{BodyHtml, PageBody, PageKind, RenderPage};
use khalid_seo::PageKind as SeoKind;

use crate::build_pages::{Page, meta, page, path_of};
use crate::build_seo;
use crate::post_lock::escaped;

/// Where the page sits under each language's address.
pub(crate) const REST: &str = "ai/";

/// The AI info page of `locale`, at `ai/`, in plain HTML that needs no
/// scripts, with its own `AboutPage` schema, updated `today`. `None` when
/// `[seo.ai_page] off = true`.
pub(crate) fn ai_page(
    config: &Config,
    locale: &Locale,
    today: NaiveDate,
) -> Result<Option<Page>, UserMessage> {
    let settings = config.ai_page();
    if settings.off {
        return Ok(None);
    }
    let title = "About this site for readers and AI".to_owned();
    let publisher = config.seo().publisher.name.clone().unwrap_or_default();
    let who = if publisher.is_empty() {
        config.name().to_owned()
    } else {
        publisher
    };
    let tagline = &config.seo().titles.tagline;
    let covers = settings.covers.clone().unwrap_or_else(|| {
        if tagline.is_empty() {
            format!("{who} publishes notes and guides.")
        } else {
            tagline.clone()
        }
    });
    let contact = settings.contact.clone().unwrap_or_else(|| {
        config
            .seo()
            .publisher
            .email
            .clone()
            .unwrap_or_else(|| "the address in the site's contact page".to_owned())
    });
    let cite = settings
        .cite
        .clone()
        .unwrap_or_else(|| format!("Name {who} and link the post you quote."));
    // The settings are plain text, so each is escaped into the page.
    let body = [
        format!("<p>{} runs this site.</p>", escaped(&who)),
        format!("<p>{}</p>", escaped(&covers)),
        format!("<p>{}</p>", escaped(&cite)),
        format!("<p>To reach the team: {}.</p>", escaped(&contact)),
    ]
    .join("\n");
    let description =
        format!("Who runs {who}, what it covers, how to cite it and how to reach the team.");
    let path = path_of(config, locale, REST);
    let mut seo = build_seo::page(SeoKind::AiPage, locale, REST, &title)?;
    seo.excerpt.clone_from(&description);
    let meta = meta(config, title.clone(), description, path, PageKind::Website)?;
    let body = RenderPage {
        title,
        updated: Some(today),
        body: BodyHtml::from_sanitised(body),
    };
    Ok(Some(
        page(locale, config, REST, meta, PageBody::Page(body), seo)
            .made_from(Path::new("site.toml")),
    ))
}
