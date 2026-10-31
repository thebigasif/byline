//! Pages that still get search traffic though readers cannot use them:
//! addresses in the site's 404 log, and posts marked `noindex` (plan 5.11,
//! the 404 log joined with Search Console).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use khalid_api::not_found_log::{MAX_MISSES, NotFoundLog as _};
use khalid_core::Slug;
use khalid_db::{Database, NotFoundTable};
use khalid_search_console::PageNumbers;
use serde::Serialize;

use crate::bing::{Failed, block_on};
use crate::exit::Exit;
use crate::search_console_report::LivePost;
use crate::{db, db_message};

/// The JSON shape of a page that still gets search traffic.
#[derive(Debug, Serialize)]
pub(crate) struct LostBody {
    url: String,
    clicks: f64,
    impressions: f64,
    /// `missing` for an address that answers 404, `noindex` for a post
    /// marked so.
    why: &'static str,
    /// How many requests the 404 log counted, for a missing page.
    not_found_hits: Option<u64>,
}

/// Each address in the 404 log of the site in `folder`, with how many
/// requests it answered.
pub(crate) fn misses(folder: &Path) -> Result<BTreeMap<String, u64>, Failed> {
    let path = db::data_file(folder).map_err(|message| (message, Exit::Failure))?;
    let main = Slug::try_new(db::MAIN).map_err(|_| (db_message::internal(), Exit::Internal))?;
    let limit = u32::try_from(MAX_MISSES).unwrap_or(u32::MAX);
    block_on(async {
        let database = Database::open(&path)
            .await
            .map_err(|error| (db_message::of(&path, &error), Exit::Failure))?;
        let opened = async {
            database.migrate().await?;
            database.site(&main).await
        };
        let read = match opened.await {
            Ok(id) => NotFoundTable::new(database.clone(), id)
                .misses(limit)
                .await
                .map_err(|_| (db_message::internal(), Exit::Internal)),
            Err(error) => Err((db_message::of(&path, &error), Exit::Failure)),
        };
        database.close().await;
        read
    })?
    .map(|found| {
        found
            .into_iter()
            .map(|miss| (miss.path, miss.hits))
            .collect()
    })
}

/// The `pages` with clicks or impressions that answer 404 by `misses`, or
/// are `posts` marked `noindex`, for the site whose address starts with
/// `root`.
pub(crate) fn lost(
    root: &str,
    posts: &[LivePost],
    pages: &[PageNumbers],
    misses: &BTreeMap<String, u64>,
) -> Vec<LostBody> {
    let seen = pages
        .iter()
        .filter(|page| page.clicks > 0.0 || page.impressions > 0.0);
    seen.filter_map(|page| {
        let hits = page
            .key
            .strip_prefix(root)
            .and_then(|path| misses.get(path));
        let hidden = posts
            .iter()
            .any(|post| post.noindex && post.address == page.key);
        let why = match (hits, hidden) {
            (Some(_), _) => "missing",
            (None, true) => "noindex",
            (None, false) => return None,
        };
        Some(LostBody {
            url: page.key.clone(),
            clicks: page.clicks,
            impressions: page.impressions,
            why,
            not_found_hits: hits.copied(),
        })
    })
    .collect()
}

/// `lost` as sentences for a person, added to `text`.
pub(crate) fn sentences(lost: &[LostBody], text: &mut String) {
    if !lost.is_empty() {
        text.push_str("\nThese still get search traffic, but readers cannot use them:\n");
    }
    for page in lost {
        let fix = match page.not_found_hits {
            Some(hits) => format!(
                "It answers 404, not found, to {hits} requests so far. Add a redirect to the page that replaced it."
            ),
            None => "It is marked noindex, so Google will drop it. Remove noindex if it should stay in search.".to_owned(),
        };
        // Writing to a String cannot fail, so the result is dropped.
        let _ = writeln!(
            text,
            "- {}: {:.0} clicks, {:.0} impressions.\n  {fix}",
            page.url, page.clicks, page.impressions
        );
    }
}
