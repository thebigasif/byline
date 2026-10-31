//! `khalid publish`: takes a draft live through the publish door the Admin
//! API uses, so the lint gate runs first and the site's webhooks and
//! `IndexNow` are told, as for any post that goes live (KH-OPS-2, audit
//! brief G28).

use std::path::Path;
use std::sync::Arc;

use khalid_api::post_store::UpdatePostError;
use khalid_api::publish_door::PublishDoor;
use khalid_api::resources::{AdminPost, PostEdit, PostStatus};
use khalid_core::{Config, Locale, Slug, SystemClock, UserMessage};
use khalid_db::ScheduleTable;
use khalid_lint::Report;
use serde::Serialize;

use crate::cli::{Cli, Output};
use crate::exit::Exit;
use crate::site_posts::SitePosts;
use crate::{announce, lint, outbound, report, serve, serve_message};

/// Who the command's edits are made by, as `updated_by` shows.
const BY: &str = "khalid publish";

/// The JSON shape of a published post, as the Admin API answers it.
#[derive(Serialize)]
struct PostsBody<'a> {
    posts: [&'a AdminPost; 1],
}

/// The JSON shape of a refused post: the message, then its findings,
/// warnings included, as the `lint.blocked` webhook carries them.
#[derive(Serialize)]
struct RefusedBody<'a> {
    error: &'a UserMessage,
    lint: &'a Report,
}

/// What the door did with the post.
enum Done {
    /// The post went live.
    Published(Box<AdminPost>),
    /// The linter refused the post, which stays a draft.
    Refused(Report),
}

/// Publishes the draft `slug` in `language`, or the site's default
/// language, of the checked site in `folder`.
pub(crate) fn run(
    cli: &Cli,
    (folder, config): (&Path, &Config),
    (slug, language): (&Slug, Option<&Locale>),
) -> Exit {
    let output = cli.output;
    let language = language.unwrap_or_else(|| config.default_language());
    let name = format!("{language}/{slug}");
    let id = format!("{language}.{slug}");
    match through_door(cli, (folder, config), (&id, &name)) {
        Ok(Done::Published(post)) => match output {
            Output::Json => report::json(&PostsBody { posts: [&*post] }, Exit::Success),
            Output::Human => report::text(&format!("Published {name}.\n"), Exit::Success),
        },
        Ok(Done::Refused(found)) => refuse(output, &name, &found),
        Err((message, exit)) => report::failure(output, &message, exit),
    }
}

type Failed = (UserMessage, Exit);

/// Opens the site's database, builds the door `khalid serve` builds, and
/// publishes post `id`, called `name` in messages, through it. Waits for
/// the announcements to be sent.
fn through_door(
    cli: &Cli,
    (folder, config): (&Path, &Config),
    (id, name): (&str, &str),
) -> Result<Done, Failed> {
    let internal = (serve_message::internal(), Exit::Internal);
    let outbound = outbound::client(cli).map_err(|_| internal.clone())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| internal)?;
    let (database, site) =
        serve::open(folder, &runtime).map_err(|message| (message, Exit::Failure))?;
    let done = runtime.block_on(async {
        let (telling, announcing) = announce::once(config, (&database, site), &outbound)
            .map_err(|message| (message, Exit::Failure))?;
        let posts = SitePosts::new(folder, config.clone(), SystemClock);
        let schedules = Arc::new(ScheduleTable::new(database.clone(), site));
        let door = PublishDoor::new(Arc::new(posts), Arc::new(telling))
            .with_schedules(schedules, SystemClock);
        let done = publish(&door, (id, name)).await;
        drop(door);
        announcing.stop().await;
        done
    });
    runtime.block_on(database.close());
    done
}

/// Publishes post `id` through `door`.
async fn publish(door: &PublishDoor, (id, name): (&str, &str)) -> Result<Done, Failed> {
    let failed = |message| (message, Exit::Failure);
    let before = door.get(id).await;
    let before = before.map_err(|_| failed(unreadable(name)))?;
    let before = before.ok_or_else(|| failed(no_post(name)))?;
    if before.status() == PostStatus::Published {
        return Err(failed(already_live(name)));
    }
    let edit = PostEdit::status_only(PostStatus::Published, before.updated_at());
    match door.edit(id, edit, BY).await {
        Ok(post) if post.status() == PostStatus::Published => Ok(Done::Published(Box::new(post))),
        Err(UpdatePostError::Refused(lint)) => Ok(Done::Refused(lint)),
        Err(UpdatePostError::NotFound) => Err(failed(no_post(name))),
        Err(UpdatePostError::Stale { .. }) => Err(failed(changed(name))),
        // A door that answers with a post still not live published nothing.
        Ok(_) | Err(_) => Err(failed(unreadable(name))),
    }
}

/// Explains why post `name` stays a draft: every finding `found`, then
/// what, why and fix, as a refused `khalid build` does. Ends with exit
/// code 1.
fn refuse(output: Output, name: &str, found: &Report) -> Exit {
    let message = not_ready(name);
    match output {
        Output::Json => report::json(
            &RefusedBody {
                error: &message,
                lint: found,
            },
            Exit::Failure,
        ),
        Output::Human => {
            let mut text = String::new();
            for finding in found.findings() {
                lint::write_finding(&mut text, finding);
            }
            match report::text(&text, Exit::Failure) {
                Exit::Failure => report::failure(output, &message, Exit::Failure),
                unwritten => unwritten,
            }
        }
    }
}

fn not_ready(name: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid did not publish {name}, because the linter found problems that block it."),
        "The post stays a draft, so readers never see it with those problems.",
        format!("Fix each block, then run `khalid publish` for {name} again."),
    )
}

/// `name` is `<language>/<slug>`, so its draft file is `drafts/<name>.md`.
fn already_live(name: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid did not publish {name}, because it is already live."),
        "Its file in content/ has a date, so readers can see it now.",
        format!("Nothing changed. To take it down, move its file to drafts/{name}.md."),
    )
}

fn no_post(name: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid found no post {name}."),
        "A draft is a file in drafts/<language>/ named by its slug, or a post in content/ with no date.",
        "Check the slug, give --language for a post in another language, or make one with `khalid new`.",
    )
}

fn changed(name: &str) -> UserMessage {
    UserMessage::new(
        format!("The post {name} changed while Khalid was publishing it."),
        "Khalid publishes only the copy it checked, so nothing was published.",
        "Run the same command again.",
    )
}

fn unreadable(name: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not read or write the post {name}."),
        "A file may be locked, or the disk may be full, so nothing was published.",
        "Check the site folder can be written to, then run it again.",
    )
}
