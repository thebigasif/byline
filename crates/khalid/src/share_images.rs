//! The generated share image in `khalid build` and `khalid serve`: every
//! page that sets no image of its own and has no site default gets a drawn
//! card (PRD KH-SEO-2, ADR 0016), kept in the site's `data/share/` cache by
//! content key and served from the mount at `share/<key>.png`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Path as UrlPath;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use khalid_server::SecurityHeaders;

use khalid_core::{Config, UserMessage};
use khalid_render::{Color, Tokens};
use khalid_seo::{PageKind, SeoSite, ShareImage};
use khalid_share_image::{CardColors, Direction, Rgb, ShareCache, ShareCard, ShareKey};

use crate::build_message;
use crate::build_pages::LanguagePages;

/// The folder, under the mount and in the output folder, that holds the
/// drawn cards, beside `assets/` and `media/`.
const FOLDER: &str = "share/";

/// Gives every page of `languages` that sets no share image of its own and
/// inherits none from `site` a drawn card, drawing each missing card into
/// the site's cache. Returns where each card's PNG goes, relative to the
/// output folder, for `khalid build` to copy.
pub(crate) fn attach(
    site: &Path,
    config: &Config,
    tokens: &Tokens,
    seo_site: &SeoSite,
    languages: &mut [LanguagePages],
) -> Result<BTreeMap<String, ShareCard>, UserMessage> {
    let cache = ShareCache::new(site);
    let colors = card_colors(tokens);
    let mut cards: BTreeMap<String, ShareCard> = BTreeMap::new();
    for language in languages.iter_mut() {
        let direction = direction(&language.site.locale);
        for page in &mut language.pages {
            if !wants(page.seo.kind)
                || page.seo.share_image.is_some()
                || seo_site.share_image.is_some()
            {
                continue;
            }
            let mut card = ShareCard::new(&page.seo.title, config.name(), colors);
            card.direction = direction;
            let key = ShareKey::of(&card);
            let address = format!("{}{FOLDER}{}.png", config.mount(), key.as_str());
            let url = share_url(config, &address)?;
            page.seo.share_image = Some(ShareImage::new(
                url,
                khalid_share_image::WIDTH,
                khalid_share_image::HEIGHT,
                &page.seo.title,
            ));
            cards.insert(key.as_str().to_owned(), card);
        }
    }
    drop(cache);
    Ok(cards)
}

/// Draws every `card` into the site's cache, one by one. Meant for a
/// background task: the pages already name the cards' addresses, so the
/// drawing never blocks an answer (ADR \"khalid serve answers before the
/// full render\").
pub(crate) fn warm(site: &Path, cards: BTreeMap<String, ShareCard>) {
    let cache = ShareCache::new(site);
    for (_, card) in cards {
        if let Err(error) = cache.draw(&card) {
            tracing::warn!(?error, "a share card could not be drawn");
        }
    }
}

/// Draws `cards` now, for the pages a first answer already serves, and
/// gives back the files as the site serves them, `share/<key>.png`.
pub(crate) fn draw_now(
    site: &Path,
    cards: BTreeMap<String, ShareCard>,
) -> Result<Vec<String>, UserMessage> {
    let cache = ShareCache::new(site);
    let mut files = Vec::with_capacity(cards.len());
    for (_, card) in cards {
        cache
            .draw(&card)
            .map_err(|error| build_message::share_image(&error))?;
        let file = format!("{FOLDER}{}.png", ShareKey::of(&card).as_str());
        files.push(file);
    }
    Ok(files)
}

/// Whether a page of `kind` gets a drawn card. A post, a plain page, an
/// author, a glossary term and the home page do. The not-found page and
/// search results stay out of search, so they draw nothing.
fn wants(kind: PageKind) -> bool {
    !matches!(kind, PageKind::NotFound | PageKind::Search)
}

/// The direction `locale` is written in, as the card crate mirrors it.
fn direction(locale: &khalid_core::Locale) -> Direction {
    let tag = locale.as_str();
    let script = tag.split('-').nth(1).filter(|part| part.len() == 4);
    let rtl = match script {
        Some(script) => matches!(script, "Adlm" | "Arab" | "Hebr" | "Nkoo" | "Syrc" | "Thaa"),
        None => matches!(
            locale.language(),
            "ar" | "ckb" | "dv" | "fa" | "he" | "ps" | "sd" | "ug" | "ur" | "yi"
        ),
    };
    if rtl {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    }
}

/// The theme's colours as the card crate reads them: the page background,
/// the text colour, and the accent for the bars.
fn card_colors(tokens: &Tokens) -> CardColors {
    CardColors::new(
        rgb(&tokens.colors.background),
        rgb(&tokens.colors.text),
        rgb(&tokens.colors.accent),
    )
}

/// A colour as the card crate reads it. A `Color` always prints as
/// `#rrggbb`, so the three pairs are the channels.
fn rgb(color: &Color) -> Rgb {
    let hex = color.as_str().strip_prefix('#').unwrap_or(color.as_str());
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    Rgb([channel(0), channel(2), channel(4)])
}

/// The full address of a card on the site's host.
fn share_url(config: &Config, address: &str) -> Result<khalid_core::AbsoluteUrl, UserMessage> {
    let full = format!(
        "{}{address}",
        config.address().as_str().trim_end_matches('/')
    );
    khalid_core::AbsoluteUrl::parse(&full).map_err(|_| build_message::address(&full))
}

/// The routes that answer each drawn card under the mount, read straight
/// from the site's cache folder on the request, kept for a year because a
/// card's address carries its content's hash. The cards are never part of
/// the in-memory page set, so a large site's images never touch the idle
/// memory budget (ADR 0024, and the ADR this change adds).
pub(crate) fn router(site: &Path, config: &Config, security: SecurityHeaders) -> Router {
    let cards = Arc::new(Cards {
        site: site.to_path_buf(),
        security,
    });
    Router::new()
        .route(&format!("{}{FOLDER}{{key}}", config.mount()), get(answer))
        .with_state(cards)
}

/// What a card request reads.
struct Cards {
    site: PathBuf,
    security: SecurityHeaders,
}

/// Answers one drawn card from the cache, or 404 when no such card exists.
async fn answer(
    State(cards): State<Arc<Cards>>,
    UrlPath(key): UrlPath<String>,
) -> axum::response::Response {
    let Some(key) = ShareKey::parse(key.strip_suffix(".png").unwrap_or(&key)) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let site = cards.site.clone();
    let reading = tokio::task::spawn_blocking(move || ShareCache::new(&site).read(&key));
    let Ok(Ok(Some(png))) = reading.await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut headers = HeaderMap::new();
    cards.security.apply(&mut headers);
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    let mut response = axum::http::Response::new(Body::from(png));
    *response.headers_mut() = headers;
    response
}
