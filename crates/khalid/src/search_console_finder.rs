//! The Search Console follow-ups `khalid serve` finds for the Admin API
//! (GEO M3), with the finder `khalid search-console follow-ups` runs. The
//! connection is read once at start; the posts are read on each request,
//! so a heading saved since counts.

use std::path::{Path, PathBuf};

use khalid_api::content_store::BoxFuture;
use khalid_api::follow_ups::{FollowUpError, FollowUpFinder, FollowUps};
use khalid_content::{ContentTree, read_content};
use khalid_core::{Clock, Config, Environment, SystemClock, UserMessage};
use khalid_outbound::Outbound;
use khalid_search_console::SearchConsole;

use crate::cli::Cli;
use crate::search_console::{self, last_month};
use crate::search_console_follow_ups::found;

/// The follow-ups of the site in one folder.
#[derive(Debug)]
pub(crate) struct SiteFollowUps {
    folder: PathBuf,
    config: Config,
    /// The connection and the client that reaches Google, or `None` when
    /// the site has no Search Console connection or no Google sign-in.
    connection: Option<(SearchConsole, Outbound)>,
}

impl SiteFollowUps {
    /// The follow-ups of the site in `folder` with `config`, signing in
    /// with the secrets in `env`.
    pub(crate) fn new(folder: &Path, config: &Config, env: &Environment, cli: &Cli) -> Self {
        Self {
            folder: folder.to_path_buf(),
            config: config.clone(),
            connection: search_console::connect(config, env, cli).ok(),
        }
    }

    /// The follow-ups Google shares for the last month whose numbers are
    /// final, per post of the site as it is now.
    async fn find(&self) -> Result<FollowUps, FollowUpError> {
        let Some((console, outbound)) = &self.connection else {
            return Err(FollowUpError::NotConnected(not_connected()));
        };
        let month = last_month(SystemClock.now().date_naive());
        let searches = async { console.sign_in(outbound).await?.searches(month).await };
        let searches = searches.await.map_err(|error| {
            FollowUpError::Google(search_console::message(console.property(), &error))
        })?;
        let (folder, config) = (self.folder.clone(), self.config.clone());
        let property = console.property().to_owned();
        let read = tokio::task::spawn_blocking(move || {
            let content = ContentTree::walk(&folder)
                .and_then(|tree| read_content(&tree))
                .map_err(|error| FollowUpError::Internal(Box::new(error)))?;
            found((&folder, &config, &content), &searches, (&property, month))
                .map_err(|(message, _)| unreadable(&message))
        });
        read.await
            .map_err(|error| FollowUpError::Internal(Box::new(error)))?
    }
}

impl FollowUpFinder for SiteFollowUps {
    fn follow_ups(&self) -> BoxFuture<'_, Result<FollowUps, FollowUpError>> {
        Box::pin(self.find())
    }
}

/// The message when the site has no Search Console connection, or no
/// Google sign-in for it.
fn not_connected() -> UserMessage {
    UserMessage::new(
        "This site is not connected to Google Search Console, Google's report of how a site does in search.",
        "Khalid asks Google nothing until site.toml opts in under [search_console] and the server has the owner's Google sign-in.",
        "Follow the steps in docs/deploy.md under Google Search Console, then restart khalid serve.",
    )
}

/// The site's posts could not be listed, as `message` says.
fn unreadable(message: &UserMessage) -> FollowUpError {
    FollowUpError::Internal(std::io::Error::other(message.to_string()).into())
}
