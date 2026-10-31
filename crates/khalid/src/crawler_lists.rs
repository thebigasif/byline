//! The crawler address lists the visitor log checks bots against
//! (decision D2): read from `data/crawler-ranges/` at start, then fetched
//! from each company through the shared client every day, and kept there
//! again when they read.

use std::path::{Path, PathBuf};
use std::time::Duration;

use khalid_analytics::{CrawlerRanges, LONGEST_LIST};
use khalid_core::AbsoluteUrl;
use khalid_outbound::Outbound;
use tokio::sync::watch;
use tokio::task::JoinHandle;

/// How often the lists are fetched again.
const FETCH_EVERY: Duration = Duration::from_hours(24);

/// The folder under `data/` the lists are kept in between fetches.
const FOLDER: &str = "crawler-ranges";

/// Starts the task that reads the lists kept under `site`, then fetches
/// them through `outbound`, and gives where the latest lists are read.
/// The task runs until it is aborted.
pub(crate) fn start(
    site: &Path,
    outbound: &Outbound,
) -> (JoinHandle<()>, watch::Receiver<CrawlerRanges>) {
    let (ranges, reading) = watch::channel(CrawlerRanges::default());
    let task = tokio::spawn(fetch_lists(folder(site), outbound.clone(), ranges));
    (task, reading)
}

/// `data/crawler-ranges/` in the site folder.
fn folder(site: &Path) -> PathBuf {
    let mut folder = site.to_path_buf();
    folder.push("data");
    folder.push(FOLDER);
    folder
}

/// Reads the kept copy of each list, then fetches each list again every
/// day, keeping a copy of each one that reads.
async fn fetch_lists(folder: PathBuf, outbound: Outbound, ranges: watch::Sender<CrawlerRanges>) {
    for list in CrawlerRanges::LISTS {
        let mut path = folder.clone();
        path.push(list.file);
        if let Ok(body) = tokio::fs::read(&path).await {
            ranges.send_modify(|ranges| {
                if let Err(error) = ranges.add(list, &body) {
                    tracing::warn!(%error, file = list.file, "a kept crawler address list was skipped");
                }
            });
        }
    }
    loop {
        for list in CrawlerRanges::LISTS {
            let Some(body) = fetch(&outbound, list.url).await else {
                continue;
            };
            let mut read = ranges.borrow().clone();
            if let Err(error) = read.add(list, &body) {
                tracing::warn!(%error, url = list.url, "a crawler address list was skipped");
                continue;
            }
            ranges.send_replace(read);
            keep(&folder, list.file, &body).await;
        }
        tokio::time::sleep(FETCH_EVERY).await;
    }
}

/// The body at `url`, when it answers 200 within [`LONGEST_LIST`] bytes.
async fn fetch(outbound: &Outbound, url: &str) -> Option<Vec<u8>> {
    let url = AbsoluteUrl::parse(url).ok()?;
    match outbound.get(&url).await {
        Ok(fetched) if fetched.status() == 200 && fetched.body().len() <= LONGEST_LIST => {
            Some(fetched.body().to_vec())
        }
        Ok(fetched) => {
            let status = fetched.status();
            tracing::warn!(%url, status, "a crawler address list was not fetched");
            None
        }
        Err(error) => {
            tracing::warn!(%url, %error, "a crawler address list was not fetched");
            None
        }
    }
}

/// Keeps `body` as `file` in `folder` for the next start.
async fn keep(folder: &Path, file: &str, body: &[u8]) {
    let mut path = folder.to_path_buf();
    path.push(file);
    let kept = async {
        tokio::fs::create_dir_all(folder).await?;
        tokio::fs::write(&path, body).await
    };
    if let Err(error) = kept.await {
        tracing::warn!(%error, file, "a crawler address list was not kept");
    }
}
