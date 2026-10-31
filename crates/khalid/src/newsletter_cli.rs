//! `khalid newsletter`: labels, segments and the embeddable form from the
//! command line (G39, task 5.10).
//! Subscribers live in `data/site.db`; a running `khalid serve` sees a
//! change at once. Counts are totals, never a list of people.

use std::path::Path;

use clap::builder::{BoolValueParser, TypedValueParser};
use clap::{ArgAction, Subcommand};
use khalid_core::{Clock, Config, Locale, Slug, SystemClock, UserMessage};
use khalid_db::{Database, SubscriberTable};
use khalid_newsletter::{EmailAddress, Labelled, Pages, Segment, SubscriberStore};
use serde::Serialize;

use crate::cli::{Cli, Output};
use crate::exit::Exit;
use crate::{db, db_message, newsletter, newsletter_files, newsletter_send, report, serve_message};

/// The newsletter commands.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum NewsletterCommand {
    /// Give a subscriber a label, such as paid, or take it away with --remove
    Label {
        /// The subscriber's email address
        #[arg(value_name = "EMAIL")]
        email: String,
        /// The label, in lowercase letters, digits and hyphens
        #[arg(value_name = "LABEL")]
        label: Slug,
        /// Take the label away instead
        #[arg(
            long = "remove",
            action = ArgAction::SetTrue,
            value_parser = BoolValueParser::new().map(|remove| if remove { Change::Remove } else { Change::Add }),
        )]
        change: Change,
    },
    /// Count the confirmed subscribers of one newsletter who carry every
    /// label given
    Segment {
        /// The newsletter, such as weekly
        #[arg(long, value_name = "NAME", default_value = "main")]
        list: Slug,
        /// A label they must carry; give it more than once for several
        #[arg(long = "label", value_name = "LABEL")]
        labels: Vec<Slug>,
    },
    /// Print a sign-up form to paste into another website, such as your
    /// main site. It works with scripts off
    Embed {
        /// The newsletter it signs readers up for
        #[arg(long, value_name = "NAME", default_value = "main")]
        list: Slug,
    },
    /// Email a published post to a newsletter's confirmed subscribers,
    /// once. An edit never sends it again
    Send {
        /// The slug of the post
        slug: Slug,
        /// The language the post is written in [default: the site's default language]
        #[arg(long, value_name = "CODE")]
        language: Option<Locale>,
        /// The newsletter, such as weekly
        #[arg(long, value_name = "NAME", default_value = "main")]
        list: Slug,
        /// Send only to subscribers with this label; give it more than once for several
        #[arg(long = "label", value_name = "LABEL")]
        labels: Vec<Slug>,
    },
}

/// Whether `label` adds a label or takes it away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    /// Give the subscriber the label.
    Add,
    /// Take the label away.
    Remove,
}

impl NewsletterCommand {
    /// The command as a person types it, after `khalid`.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Label { .. } => "newsletter label",
            Self::Segment { .. } => "newsletter segment",
            Self::Send { .. } => "newsletter send",
            Self::Embed { .. } => "newsletter embed",
        }
    }
}

/// The JSON shape of `khalid newsletter segment --json`.
#[derive(Serialize)]
struct SegmentBody<'a> {
    list: &'a str,
    labels: Vec<&'a str>,
    subscribers: usize,
}

type Done = Result<Exit, (UserMessage, Exit)>;

/// Runs `command` on the checked site in `folder` with `config`.
pub(crate) fn run(
    cli: &Cli,
    (folder, config): (&Path, &Config),
    command: &NewsletterCommand,
) -> Exit {
    let output = cli.output;
    let done = match command {
        NewsletterCommand::Send {
            slug,
            language,
            list,
            labels,
        } => {
            let asked = newsletter_send::Asked {
                slug,
                language: language.as_ref(),
                list,
                labels,
            };
            return newsletter_send::run(cli, (folder, config), &asked);
        }
        NewsletterCommand::Embed { list } => Ok(embed((folder, config), list, output)),
        NewsletterCommand::Label {
            email,
            label,
            change,
        } => match EmailAddress::parse(email) {
            Ok(email) => on_store(folder, async |store| {
                label_one(store, (&email, label), *change, output).await
            }),
            Err(_) => Err((wrong_email(), Exit::Usage)),
        },
        NewsletterCommand::Segment { list, labels } => {
            let segment = Segment::new(list.clone(), labels.iter().cloned());
            on_store(folder, async |store| count(store, &segment, output).await)
        }
    };
    done.flatten()
        .unwrap_or_else(|(message, exit)| report::failure(output, &message, exit))
}

async fn label_one(
    store: &SubscriberTable,
    (email, label): (&EmailAddress, &Slug),
    change: Change,
    output: Output,
) -> Done {
    let done = match change {
        Change::Add => store.label(email, label, SystemClock.now()).await,
        Change::Remove => store.unlabel(email, label).await,
    };
    match (done.map_err(|_| unreachable())?, change) {
        (Labelled::NotSubscribed, _) => Err((not_subscribed(), Exit::Failure)),
        (_, Change::Add) => Ok(said(output, &format!("Added the label {label}.\n"))),
        (_, Change::Remove) => Ok(said(output, &format!("Took the label {label} away.\n"))),
    }
}

async fn count(store: &SubscriberTable, segment: &Segment, output: Output) -> Done {
    let found = store.audience(segment).await.map_err(|_| unreachable())?;
    let labels: Vec<&str> = segment.labels().map(Slug::as_str).collect();
    Ok(match output {
        Output::Json => {
            let body = SegmentBody {
                list: segment.list().as_str(),
                labels,
                subscribers: found.len(),
            };
            report::json(&body, Exit::Success)
        }
        Output::Human => {
            let carrying = match labels.as_slice() {
                [] => String::new(),
                labels => format!(" with the labels {}", labels.join(", ")),
            };
            let text = format!(
                "{} confirmed subscribers of {}{carrying}.\n",
                found.len(),
                segment.list()
            );
            report::text(&text, Exit::Success)
        }
    })
}

/// The sign-up form for `list` of the site in `folder`, as HTML to paste.
fn embed((folder, config): (&Path, &Config), list: &Slug, output: Output) -> Done {
    #[derive(Serialize)]
    struct EmbedBody<'a> {
        list: &'a str,
        html: &'a str,
    }
    let Some(list) = newsletter_files::lists(folder).find(Some(list.as_str())) else {
        return Err((no_list(list), Exit::Failure));
    };
    let pages = Pages::new(newsletter::site(config));
    let pages = pages.map_err(|_| (serve_message::internal(), Exit::Internal))?;
    let html = pages.embed(&list);
    Ok(match output {
        Output::Json => report::json(
            &EmbedBody {
                list: list.as_str(),
                html: &html,
            },
            Exit::Success,
        ),
        Output::Human => report::text(&html, Exit::Success),
    })
}

fn no_list(list: &Slug) -> UserMessage {
    UserMessage::new(
        format!("The site has no newsletter called {list}."),
        "Each newsletter but main is a folder in content/newsletters/.",
        format!("Make the folder content/newsletters/{list}/, or leave --list out for main."),
    )
}

/// `text` for a person, or `{"done":true}` for a program.
fn said(output: Output, text: &str) -> Exit {
    #[derive(Serialize)]
    struct DoneBody {
        done: bool,
    }
    match output {
        Output::Json => report::json(&DoneBody { done: true }, Exit::Success),
        Output::Human => report::text(text, Exit::Success),
    }
}

/// Opens the site's database, brings it up to date, runs `work` on its
/// subscribers, then closes the database.
fn on_store<T>(
    site: &Path,
    work: impl AsyncFnOnce(&SubscriberTable) -> T,
) -> Result<T, (UserMessage, Exit)> {
    let failed = |message| (message, Exit::Failure);
    let path = db::data_file(site).map_err(failed)?;
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(db::MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Err(failed(db_message::internal()));
    };
    runtime.block_on(async {
        let database = Database::open(&path)
            .await
            .map_err(|error| failed(db_message::of(&path, &error)))?;
        let opened = async {
            database.migrate().await?;
            database.site(&main).await
        }
        .await;
        let done = match opened {
            Ok(id) => Ok(work(&SubscriberTable::new(database.clone(), id)).await),
            Err(error) => Err(failed(db_message::of(&path, &error))),
        };
        database.close().await;
        done
    })
}

fn wrong_email() -> UserMessage {
    UserMessage::new(
        "That is not an email address.",
        "An address needs one @ and a domain, like name@example.com.",
        "Type the address again.",
    )
}

fn not_subscribed() -> UserMessage {
    UserMessage::new(
        "No one subscribed has that email address.",
        "Labels belong to subscribers, so there was nothing to change.",
        "Check the spelling, or ask the reader to sign up first.",
    )
}

fn unreachable() -> (UserMessage, Exit) {
    let message = UserMessage::new(
        "Khalid could not read or change the subscribers in data/site.db.",
        "Nothing was changed.",
        "Check the disk has free space and nothing else holds data/site.db, then run it again.",
    );
    (message, Exit::Failure)
}
