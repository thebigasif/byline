//! [`Video`]: a video on a page, for the video sitemap and `VideoObject`.

use std::time::Duration;

use chrono::{DateTime, Utc};
use khalid_core::AbsoluteUrl;

/// Where a video plays.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VideoSource {
    /// The video file itself, such as an `.mp4`.
    File(AbsoluteUrl),
    /// A player page or an embed address from a video host.
    Player(AbsoluteUrl),
    /// Both a file and a player.
    FileAndPlayer {
        /// The video file.
        file: AbsoluteUrl,
        /// The player.
        player: AbsoluteUrl,
    },
}

/// A video on a page, filled from the video's own data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Video {
    /// The video's title.
    pub title: String,
    /// What the video shows.
    pub description: String,
    /// The still image shown before it plays.
    pub thumbnail: AbsoluteUrl,
    /// Where it plays.
    pub source: VideoSource,
    /// How long it runs.
    pub duration: Option<Duration>,
    /// When it was first published.
    pub uploaded: Option<DateTime<Utc>>,
}

impl Video {
    /// A video with its title, description, thumbnail and source.
    pub fn new(
        title: &str,
        description: &str,
        thumbnail: AbsoluteUrl,
        source: VideoSource,
    ) -> Self {
        Self {
            title: title.to_owned(),
            description: description.to_owned(),
            thumbnail,
            source,
            duration: None,
            uploaded: None,
        }
    }

    /// The video file, if there is one.
    pub fn file(&self) -> Option<&AbsoluteUrl> {
        match &self.source {
            VideoSource::File(file) | VideoSource::FileAndPlayer { file, .. } => Some(file),
            VideoSource::Player(_) => None,
        }
    }

    /// The player, if there is one.
    pub fn player(&self) -> Option<&AbsoluteUrl> {
        match &self.source {
            VideoSource::Player(player) | VideoSource::FileAndPlayer { player, .. } => Some(player),
            VideoSource::File(_) => None,
        }
    }
}
