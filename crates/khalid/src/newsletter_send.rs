//! `khalid newsletter send`: emails a published post to a newsletter, once
//! (task 5.8). The writer asks by running it. An edit never sends the post
//! again, and nothing goes out until `[mail]` has a postal address.

use std::path::Path;
use std::sync::Arc;

use khalid_api::post_store::PostStore;
use khalid_api::resources::{AdminPost, PostStatus};
use khalid_content::parse_blocks;
use khalid_core::{Config, Environment, Locale, Slug, SystemClock, UserMessage};
use khalid_db::SubscriberTable;
use khalid_newsletter::{Issue, Newsletter, PostalAddress, Segment, Sent};
use serde::Serialize;

use crate::cli::{Cli, Output};
use crate::exit::Exit;
use crate::site_posts::SitePosts;
use crate::{announce, newsletter, newsletter_files, outbound, report, serve, serve_message};

/// What to send, and to whom.
pub(crate) struct Asked<'a> {
    /// The post's slug.
    pub(crate) slug: &'a Slug,
    /// Its language, or the site's default.
    pub(crate) language: Option<&'a Locale>,
    /// The newsletter, such as `main`.
    pub(crate) list: &'a Slug,
    /// The labels every subscriber who gets it carries.
    pub(crate) labels: &'a [Slug],
}

/// The JSON shape of a send.
#[derive(Serialize)]
struct SentBody<'a> {
    post: &'a str,
    list: &'a str,
    sent: u32,
    failed: u32,
}

pub(crate) type Failed = (UserMessage, Exit);

/// Sends the post `asked` names from the checked site in `folder`.
pub(crate) fn run(cli: &Cli, (folder, config): (&Path, &Config), asked: &Asked<'_>) -> Exit {
    let output = cli.output;
    let language = asked.language.unwrap_or_else(|| config.default_language());
    let name = format!("{language}/{}", asked.slug);
    match send(cli, (folder, config), asked, (language, &name)) {
        Ok(Sent::Now(totals)) => match output {
            Output::Json => {
                let body = SentBody {
                    post: &name,
                    list: asked.list.as_str(),
                    sent: totals.sent,
                    failed: totals.failed,
                };
                report::json(&body, Exit::Success)
            }
            Output::Human => {
                let refused = match totals.failed {
                    0 => String::new(),
                    failed => {
                        format!("{failed} copies could not be sent. They are not sent again.\n")
                    }
                };
                let text = format!(
                    "Sent {name} to {} subscribers of {}.\n{refused}",
                    totals.sent, asked.list
                );
                report::text(&text, Exit::Success)
            }
        },
        Ok(_) => report::failure(output, &sent_before(&name), Exit::Failure),
        Err((message, exit)) => report::failure(output, &message, exit),
    }
}

/// Checks the post, then sends it.
fn send(
    cli: &Cli,
    (folder, config): (&Path, &Config),
    asked: &Asked<'_>,
    (language, name): (&Locale, &str),
) -> Result<Sent, Failed> {
    let failed = |message| (message, Exit::Failure);
    let internal = || (serve_message::internal(), Exit::Internal);
    let to = (asked.list, asked.labels);
    let sent = deliver(cli, (folder, config), to, async |_| {
        let posts = SitePosts::new(folder, config.clone(), SystemClock);
        let post = posts.get(&format!("{language}.{}", asked.slug)).await;
        let post = post.map_err(|_| internal())?;
        let post = post.ok_or_else(|| failed(no_post(name)))?;
        if post.status() != PostStatus::Published {
            return Err(failed(not_published(name)));
        }
        Ok(Some(issue_of(config, &post, name)))
    })?;
    sent.ok_or_else(internal)
}

/// Checks the mail settings, the postal address and the newsletter `list`,
/// opens the site's database, then sends the issue `make` writes to the
/// confirmed subscribers of `list` who carry every one of `labels`.
/// `None` when `make` has nothing to send.
pub(crate) fn deliver(
    cli: &Cli,
    (folder, config): (&Path, &Config),
    (list, labels): (&Slug, &[Slug]),
    make: impl AsyncFnOnce(&SubscriberTable) -> Result<Option<Issue>, Failed>,
) -> Result<Option<Sent>, Failed> {
    let failed = |message| (message, Exit::Failure);
    let internal = || (serve_message::internal(), Exit::Internal);
    let mail = config.mail().ok_or_else(|| failed(no_mail()))?;
    let postal = mail.postal_address.as_deref().and_then(PostalAddress::new);
    let postal = postal.ok_or_else(|| failed(no_postal()))?;
    let found = newsletter_files::lists(folder).find(Some(list.as_str()));
    let found = found.ok_or_else(|| failed(no_list(list)))?;
    let segment = Segment::new(found, labels.iter().cloned());
    let secret = serve::mail_secret(&Environment::from_process(), config);
    let outbound = outbound::client(cli).map_err(|_| internal())?;
    let mailer = newsletter::mailer(config, secret, &outbound).map_err(failed)?;
    let mailer = mailer.ok_or_else(|| failed(no_mail()))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| internal())?;
    let (database, site) = serve::open(folder, &runtime).map_err(failed)?;
    let done = runtime.block_on(async {
        let store = SubscriberTable::new(database.clone(), site);
        let Some(issue) = make(&store).await? else {
            return Ok(None);
        };
        let clock = Arc::new(SystemClock);
        let flow = Newsletter::new(Arc::new(store), mailer, clock, newsletter::site(config));
        let flow = flow.map_err(|_| internal())?;
        let sent = flow.send(&issue, &segment, &postal).await;
        sent.map(Some).map_err(|_| failed(not_sent()))
    });
    runtime.block_on(database.close());
    done
}

/// The email of `post`, called `name`: its title as the subject, a link
/// to read it on the site, and its text without the blocks that only work
/// on a page, such as forms.
fn issue_of(config: &Config, post: &AdminPost, name: &str) -> Issue {
    let markdown = post.markdown();
    let text = match parse_blocks(markdown) {
        Ok(blocks) => blocks
            .iter()
            .map(|block| block.source.as_str())
            .filter(|source| !source.trim_start().starts_with("::"))
            .collect::<Vec<_>>()
            .join("\n\n"),
        Err(_) => markdown.to_owned(),
    };
    let link = announce::post_url(config, post)
        .map(|url| format!("Read it on the site: {}\n\n", url.as_str()))
        .unwrap_or_default();
    let title = post.title();
    Issue::new(
        format!("post:{name}"),
        title.to_owned(),
        format!("{title}\n\n{link}{}\n", text.trim_end()),
    )
}

fn no_mail() -> UserMessage {
    UserMessage::new(
        "This site cannot send email yet.",
        "A newsletter goes out through the way [mail] in site.toml names, and there is none.",
        "Add a [mail] table to site.toml, as docs/deploy.md shows under \"Sending email\".",
    )
}

fn no_postal() -> UserMessage {
    UserMessage::new(
        "Khalid sends no newsletter until the site has a postal address.",
        "Anti-spam laws ask every newsletter email to show where the sender can be reached by post.",
        "Add postal_address = \"...\" under [mail] in site.toml, then run it again.",
    )
}

fn no_list(list: &Slug) -> UserMessage {
    UserMessage::new(
        format!("The site has no newsletter called {list}."),
        "Each newsletter but main is a folder in content/newsletters/.",
        format!("Make the folder content/newsletters/{list}/, or send to --list main."),
    )
}

fn no_post(name: &str) -> UserMessage {
    UserMessage::new(
        format!("There is no post {name}."),
        "Only a post in the site's content can be sent.",
        "Check the slug and --language, as the post's front matter writes them.",
    )
}

fn not_published(name: &str) -> UserMessage {
    UserMessage::new(
        format!("The post {name} is not published."),
        "Subscribers get only posts that readers can open on the site.",
        "Publish it first with `khalid publish`, then send it.",
    )
}

fn sent_before(name: &str) -> UserMessage {
    UserMessage::new(
        format!("The post {name} went out to subscribers before, so nothing was sent."),
        "Each post is emailed once; an edit never sends it again.",
        "Write a new post to tell subscribers about the change.",
    )
}

fn not_sent() -> UserMessage {
    UserMessage::new(
        "Khalid could not finish sending the post.",
        "The subscribers in data/site.db could not be read or written.",
        "Run `khalid newsletter numbers` to see what went out. The post is not sent again.",
    )
}
