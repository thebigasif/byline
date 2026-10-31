//! `khalid mcp`: the Model Context Protocol on stdin and stdout, for agents
//! such as Claude Code. Each tool sends the request an HTTP client would
//! through the router `khalid serve` mounts, so it calls the same
//! functions the API calls (ADR "the MCP server calls the API in process").

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use khalid_api::api::{self, AdminDesks};
use khalid_api::keys::{KeyError, KeyKind, KeyToken, authenticate};
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::session_check::SessionCheck;
use khalid_core::{Config, Environment, Secret, SystemClock, UserMessage};
use khalid_db::{Database, KeyTable, SiteId, WebhookTable};
use khalid_mcp::{ApiTools, Server, SessionKeys};
use serde_json::Value;
use tokio::io::BufReader;

use crate::cli::Output;
use crate::exit::Exit;
use crate::mcp_content::FreshContent;
use crate::mcp_tools::SiteTools;
use crate::site_posts::SitePosts;
use crate::{keys_message, mcp_message, report, serve, serve_message, staff};

/// What an agent reads when it connects.
const INSTRUCTIONS: &str = "This is a Khalid blog. Each tool is one operation of its API, named \
    by its operationId. List published posts with browse_posts and read any post with \
    read_admin_post. Write new posts as drafts with create_admin_post, change them with \
    edit_admin_post, check them with lint_post, and publish with publish_post, which the linter \
    may refuse with findings to fix. Run `khalid agents` for the full guide.";

/// Serves the protocol for the `site` folder, whose settings were checked,
/// acting with the key in `environment`, until the agent closes stdin.
/// Every message to a person goes to stderr, whatever `--json` says,
/// because stdout carries only the protocol.
pub(crate) fn run(site: &Path, config: &Config, environment: &Environment) -> Exit {
    let output = Output::Human;
    let Some(key) = environment.key() else {
        return report::failure(output, &mcp_message::no_key(), Exit::Failure);
    };
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return report::failure(output, &serve_message::internal(), Exit::Internal);
    };
    let (database, id) = match serve::open(site, &runtime) {
        Ok(opened) => opened,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let served = runtime.block_on(session(site, config, key, (&database, id)));
    runtime.block_on(database.close());
    match served {
        Ok(()) => Exit::Success,
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Checks the key, then answers on stdin and stdout.
async fn session(
    site: &Path,
    config: &Config,
    key: Secret<String>,
    (database, id): (&Database, SiteId),
) -> Result<(), UserMessage> {
    let keys = KeyTable::new(database.clone(), id, SystemClock);
    let kind = match key.expose().parse::<KeyToken>().map(|token| token.kind()) {
        Ok(KeyKind::Content) => return Err(mcp_message::content_key()),
        Ok(kind) => kind,
        Err(_) => return Err(mcp_message::refused()),
    };
    let principal = match authenticate(&keys, key.expose(), kind).await {
        Ok(principal) => principal,
        Err(KeyError::Rejected) => return Err(mcp_message::refused()),
        Err(_) => return Err(mcp_message::unchecked()),
    };
    let (keys, reader) = SessionKeys::new(keys).map_err(|_| keys_message::no_randomness())?;
    let posts = SitePosts::new(site, config.clone(), SystemClock);
    let (router, document) = api_router((site, config), (database, id), keys, posts.clone())?;
    let api = ApiTools::new(router, &document, key, reader);
    let tools = SiteTools::new(api, posts, principal);
    let server = Server::new("khalid", env!("CARGO_PKG_VERSION"), INSTRUCTIONS, &tools);
    let input = BufReader::new(tokio::io::stdin());
    server
        .serve(input, tokio::io::stdout())
        .await
        .map_err(|_| mcp_message::stopped())
}

/// The API as `khalid mcp` calls it: every route `khalid serve` mounts,
/// so every operation in its document is a tool, with the staff accounts
/// and webhooks of site `id`. Comments, media and stored redirects are
/// kept in memory by a running `khalid serve`, so their routes answer
/// that the site has none here (ADR "khalid mcp holds the stores serve
/// does not keep in memory").
fn api_router(
    (site, config): (&Path, &Config),
    (database, id): (&Database, SiteId),
    keys: SessionKeys<KeyTable>,
    posts: SitePosts,
) -> Result<(Router, Value), UserMessage> {
    let content = FreshContent::new(site, config.clone());
    let door = Arc::new(PublishDoor::new(Arc::new(posts), Arc::new(Silent)));
    let accounts = staff::accounts(database, id, config);
    let sessions: Arc<dyn SessionCheck> = Arc::new(accounts.clone());
    let staff = (khalid_auth::staff_routes(accounts), sessions);
    let webhooks = WebhookTable::new(database.clone(), id, SystemClock);
    let desks = AdminDesks {
        webhooks: Some(Arc::new(webhooks)),
        ..AdminDesks::none()
    };
    let (router, document) =
        api::router_for_serve(Arc::new(content), door, Arc::new(keys), desks, staff);
    let document = serde_json::to_value(&document).map_err(|_| mcp_message::stopped())?;
    Ok((router, document))
}
