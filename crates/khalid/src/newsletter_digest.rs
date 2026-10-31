//! `khalid newsletter digest`: RSS to email (G39). One email lists the
//! posts published since the last digest, each with its title and link,
//! like a feed reader would. Run it from a timer, such as once a week; a
//! run with nothing new sends nothing.

use std::path::Path;

use chrono::{NaiveDate, TimeDelta};
use khalid_core::{Clock, Config, Locale, Slug, SystemClock, UserMessage};
use khalid_db::{ContentIndex, IndexedPost};
use khalid_newsletter::{Issue, NewsletterWords, SendNumbers, Sent, SubscriberStore};

use crate::cli::{Cli, Output};
use crate::exit::Exit;
use crate::newsletter_send::{self, Failed};
use crate::{build_pages, index, report};

/// How far back the first digest of a newsletter looks.
const FIRST_LOOK_BACK: TimeDelta = TimeDelta::days(7);

/// Who the digest goes to, and the posts of which language it lists.
pub(crate) struct Asked<'a> {
    /// The newsletter.
    pub(crate) list: &'a Slug,
    /// The labels every subscriber who gets it carries.
    pub(crate) labels: &'a [Slug],
    /// The language of the posts, or the site's default.
    pub(crate) language: Option<&'a Locale>,
}

/// Sends the digest `asked` names from the checked site in `folder`.
pub(crate) fn run(cli: &Cli, (folder, config): (&Path, &Config), asked: &Asked<'_>) -> Exit {
    let output = cli.output;
    let language = asked.language.unwrap_or_else(|| config.default_language());
    let index = match index::of(folder) {
        Ok(index) => index,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let today = SystemClock.now().date_naive();
    let to = (asked.list, asked.labels);
    let sent = newsletter_send::deliver(cli, (folder, config), to, async |store| {
        let sends = store.sends().await.map_err(|_| unreadable())?;
        let since = last_digest(&sends, asked.list).unwrap_or(today - FIRST_LOOK_BACK);
        let posts = new_posts(&index, language, (since, today));
        Ok(issue(config, asked.list, language, &posts))
    });
    match (sent, output) {
        (Ok(Some(Sent::Now(totals))), Output::Human) => report::text(
            &format!(
                "Sent the new posts to {} subscribers of {}.\n",
                totals.sent, asked.list
            ),
            Exit::Success,
        ),
        (Ok(None), Output::Human) => report::text(
            "Nothing is new since the last digest, so nothing was sent.\n",
            Exit::Success,
        ),
        (Ok(sent), Output::Json) => {
            #[derive(serde::Serialize)]
            struct DigestBody {
                sent: u32,
            }
            let sent = match sent {
                Some(Sent::Now(totals)) => totals.sent,
                _ => 0,
            };
            report::json(&DigestBody { sent }, Exit::Success)
        }
        (Ok(Some(_)), Output::Human) => report::text("Sent before.\n", Exit::Success),
        (Err((message, exit)), _) => report::failure(output, &message, exit),
    }
}

/// The date of the newest post in the last digest of `list`, from its key.
fn last_digest(sends: &[SendNumbers], list: &Slug) -> Option<NaiveDate> {
    let prefix = format!("digest:{list}:");
    sends
        .iter()
        .filter_map(|send| send.key.strip_prefix(&prefix))
        .filter_map(|date| date.parse().ok())
        .max()
}

/// The posts in `language` dated after `since` and no later than `today`,
/// oldest first.
fn new_posts<'a>(
    index: &'a ContentIndex,
    language: &Locale,
    (since, today): (NaiveDate, NaiveDate),
) -> Vec<&'a IndexedPost> {
    let mut posts: Vec<&IndexedPost> = index
        .posts
        .iter()
        .filter(|post| post.collection == "posts" && &post.locale == language)
        .filter(|post| post.date.is_some_and(|date| date > since && date <= today))
        .collect();
    posts.sort_by(|a, b| (a.date, &a.path).cmp(&(b.date, &b.path)));
    posts
}

/// The digest of `posts` for `list`, or `None` when there are none. Its key
/// names the newest post's date, so the next digest starts after it.
fn issue(config: &Config, list: &Slug, language: &Locale, posts: &[&IndexedPost]) -> Option<Issue> {
    let newest = posts.last()?.date?;
    let words = NewsletterWords::load().ok()?;
    let (subject, intro) = words.digest(language, config.name());
    let base = config.address().as_str().trim_end_matches('/');
    let lines: Vec<String> = posts
        .iter()
        .map(|post| {
            let path = build_pages::path_of(config, &post.locale, &format!("{}/", post.slug));
            let title = post.title.as_deref().unwrap_or(post.slug.as_str());
            format!("{title}\n{base}{path}\n")
        })
        .collect();
    let text = format!("{intro}\n\n{}", lines.join("\n"));
    Some(Issue::new(format!("digest:{list}:{newest}"), subject, text))
}

fn unreadable() -> Failed {
    let message = UserMessage::new(
        "Khalid could not read the newsletter's past emails in data/site.db.",
        "It needs them to know which posts are new, so nothing was sent.",
        "Check the disk has free space and nothing else holds data/site.db, then run it again.",
    );
    (message, Exit::Failure)
}
