//! `khalid import wordpress https://...`: reads a live `WordPress` site
//! through its public REST API, through the one outbound client (RULES.md
//! BL-RUN-8), page by page, into the answers `khalid-import` reads.

use khalid_core::{AbsoluteUrl, UserMessage};
use khalid_import::wordpress::RestAnswers;
use khalid_outbound::Outbound;

use crate::cli::Cli;
use crate::exit::Exit;
use crate::{db_message, outbound};

/// The most pages Khalid asks for in one list, so a site that never ends
/// its list cannot keep an import going for ever.
const MOST_PAGES: u32 = 1000;

/// The lists Khalid asks for, with how many rows a page holds. Posts and
/// pages carry their bodies, so their pages are small enough to answer
/// within the client's time limit.
const LISTS: [(&str, usize); 5] = [
    ("posts", 20),
    ("pages", 20),
    ("users", 100),
    ("categories", 100),
    ("tags", 100),
];

/// Whether `export` names a live site rather than a file.
pub(crate) fn is_address(export: &str) -> bool {
    export.starts_with("https://") || export.starts_with("http://")
}

/// Every answer the site at `site` gives to the lists Khalid reads.
pub(crate) fn answers(cli: &Cli, site: &AbsoluteUrl) -> Result<RestAnswers, (UserMessage, Exit)> {
    let client =
        outbound::client(cli).map_err(|e| (not_reached(site, &e.to_string()), Exit::Failure))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| (db_message::internal(), Exit::Internal))?;
    runtime.block_on(fetch_all(&client, site))
}

async fn fetch_all(
    client: &Outbound,
    site: &AbsoluteUrl,
) -> Result<RestAnswers, (UserMessage, Exit)> {
    let base = site.as_str().trim_end_matches('/');
    let mut answers = RestAnswers::default();
    answers.root = fetch(client, &format!("{base}/wp-json/"))
        .await?
        .ok_or_else(|| {
            (
                not_reached(site, "it has no REST API at /wp-json/"),
                Exit::Failure,
            )
        })?;
    for (list, size) in LISTS {
        let mut pages = Vec::new();
        for page in 1..=MOST_PAGES {
            let address = format!("{base}/wp-json/wp/v2/{list}?per_page={size}&page={page}");
            // WordPress answers 400 past the last page, and some sites keep
            // their list of people private.
            let Some(text) = fetch(client, &address).await? else {
                break;
            };
            let rows =
                serde_json::from_str::<Vec<serde_json::Value>>(&text).map_or(0, |rows| rows.len());
            pages.push(text);
            if rows < size {
                break;
            }
        }
        match list {
            "posts" => answers.posts = pages,
            "pages" => answers.pages = pages,
            "users" => answers.users = pages,
            "categories" => answers.categories = pages,
            _ => answers.tags = pages,
        }
    }
    Ok(answers)
}

/// The text at `address`, or none when the site answers with an error
/// status a reader would get too.
async fn fetch(client: &Outbound, address: &str) -> Result<Option<String>, (UserMessage, Exit)> {
    let url = AbsoluteUrl::parse(address)
        .map_err(|e| (not_reached_text(address, &e.to_string()), Exit::Usage))?;
    let fetched = client
        .get(&url)
        .await
        .map_err(|e| (not_reached(&url, &e.to_string()), Exit::Failure))?;
    match fetched.status() {
        200 => Ok(Some(String::from_utf8_lossy(fetched.body()).into_owned())),
        400 | 401 | 403 | 404 => Ok(None),
        status => Err((
            not_reached(&url, &format!("it answered {status}")),
            Exit::Failure,
        )),
    }
}

fn not_reached(site: &AbsoluteUrl, why: &str) -> UserMessage {
    not_reached_text(site.as_str(), why)
}

fn not_reached_text(site: &str, why: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not read the WordPress site at {site}: {why}."),
        "Nothing was imported.",
        "Check that the address opens in a browser, or export a WXR file under Tools, Export and import that.",
    )
}
