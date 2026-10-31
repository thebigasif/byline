//! `khalid serve` checks every body for broken wikilinks on a thread of
//! its own while it sets up its runtime, its database and its first pages,
//! which take about as long, and waits for the answer before it listens.
//! A broken wikilink still stops the server before it says where it
//! answers, as when the check ran first (docs/standards/performance.md,
//! cold start).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use khalid_content::{SiteContent, check_bodies};
use khalid_core::{Config, UserMessage};
use khalid_routes::Routes;

use crate::{content_message, serve_message};

/// The check of a site's bodies, running or done.
#[derive(Debug)]
pub(crate) enum BodyCheck {
    /// Running on its own thread.
    Beside(JoinHandle<Result<(), UserMessage>>),
    /// Done already, where no thread could be started.
    Done(Result<(), UserMessage>),
}

impl BodyCheck {
    /// Starts checking the bodies of `content`, read from the `site`
    /// folder with its `config` and `routes`.
    pub(crate) fn start(
        site: &Path,
        content: &Arc<SiteContent>,
        (config, routes): (&Config, &Routes),
    ) -> Self {
        let work = (site.to_path_buf(), Arc::clone(content));
        let settings = (config.clone(), routes.clone());
        let started = thread::Builder::new()
            .name("khalid-body-check".to_owned())
            .spawn(move || checked(&work, &settings));
        match started {
            Ok(running) => Self::Beside(running),
            Err(_) => Self::Done(checked(
                &(site.to_path_buf(), Arc::clone(content)),
                &(config.clone(), routes.clone()),
            )),
        }
    }

    /// Waits for the check. The message says which file and line holds a
    /// broken wikilink.
    ///
    /// # Errors
    ///
    /// That message, or an internal one when the check's thread stopped.
    pub(crate) fn finish(self) -> Result<(), UserMessage> {
        match self {
            Self::Beside(running) => running
                .join()
                .unwrap_or_else(|_| Err(serve_message::internal())),
            Self::Done(result) => result,
        }
    }
}

/// The check itself, with the message `khalid --site` gives.
fn checked(
    (site, content): &(PathBuf, Arc<SiteContent>),
    (config, routes): &(Config, Routes),
) -> Result<(), UserMessage> {
    check_bodies(content, config, routes).map_err(|error| content_message::of(site, &error))
}
