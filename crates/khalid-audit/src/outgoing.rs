//! Dead links to other sites: every address outside the blog a page links
//! to or loads, asked for once through the public client.

use std::collections::{BTreeMap, BTreeSet};

use khalid_core::AbsoluteUrl;
use khalid_outbound::Outbound;
use tokio::task::JoinSet;

use crate::{Check, Crawl, Issue};

/// How many addresses are asked for at once, so a page of many links
/// neither waits long nor floods one host.
const AT_ONCE: usize = 8;

/// Asks `outbound` for every address outside the blog that `crawl` found,
/// and returns an issue on each page that links to or loads one that is
/// broken: 400 and above, or no answer. `outbound` should be the public
/// client, which follows redirects within its cap and never reaches
/// inward.
pub async fn check_outside(crawl: &Crawl, outbound: &Outbound) -> Vec<Issue> {
    let mut from: BTreeMap<&AbsoluteUrl, BTreeSet<&AbsoluteUrl>> = BTreeMap::new();
    for visit in crawl.visits() {
        let Some(page) = visit.page() else { continue };
        let links = page.links().iter().map(crate::Link::to);
        for to in links.chain(page.resources()) {
            if !crawl.is_inside(to) {
                from.entry(to).or_default().insert(visit.url());
            }
        }
    }
    let mut broken = BTreeMap::new();
    let mut asking = JoinSet::new();
    let mut waiting = from.keys().map(|url| (*url).clone());
    loop {
        while asking.len() < AT_ONCE {
            let Some(url) = waiting.next() else { break };
            let client = outbound.clone();
            asking.spawn(async move {
                let answer = client.get(&url).await;
                (url, answer.map(|fetched| fetched.status()))
            });
        }
        let Some(done) = asking.join_next().await else {
            break;
        };
        if let Ok((url, answer)) = done {
            match answer {
                Ok(status) if status < 400 => {}
                Ok(status) => {
                    broken.insert(url, status.to_string());
                }
                Err(error) => {
                    broken.insert(url, error.to_string());
                }
            }
        }
    }
    let mut issues = Vec::new();
    for (url, why) in &broken {
        for page in from.get(url).into_iter().flatten() {
            let detail = format!("{url} ({why})");
            issues.push(Issue::new(Check::BrokenExternal, page, detail));
        }
    }
    issues
}
