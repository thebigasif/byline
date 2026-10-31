//! The podcast block: `PodcastEpisode`.

use std::time::Duration;

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError, SeoPage};

/// A podcast block: one episode, played or described on the post.
///
/// Google shows no rich result for an episode, so no rule checks it; the
/// name and the show it belongs to are required, as podcast apps need
/// both to place it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PodcastEpisode {
    /// The episode's title.
    pub name: String,
    /// The show it belongs to.
    pub series: String,
    /// The show's own page, if it has one.
    pub series_url: Option<AbsoluteUrl>,
    /// Its number in the show.
    pub number: Option<u32>,
    /// The audio file.
    pub audio: Option<AbsoluteUrl>,
    /// How long it plays.
    pub duration: Option<Duration>,
}

impl PodcastEpisode {
    /// The episode `name` of the show `series`.
    pub fn new(name: &str, series: &str) -> Self {
        Self {
            name: name.to_owned(),
            series: series.to_owned(),
            series_url: None,
            number: None,
            audio: None,
            duration: None,
        }
    }
}

/// The `PodcastEpisode` at `<page>#episode-<number>`, published when the
/// post is, its audio as an `AudioObject`.
pub(crate) fn episode_node(
    episode: &PodcastEpisode,
    page: &SeoPage,
    url: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let mut series =
        SchemaNode::new("PodcastSeries").with("name", SchemaValue::text(&episode.series));
    if let Some(series_url) = &episode.series_url {
        series.set("url", SchemaValue::Url(series_url.clone()));
    }
    let mut node = SchemaNode::new("PodcastEpisode")
        .with_id(node_id(url, &format!("episode-{number}"))?)
        .with("name", SchemaValue::text(&episode.name))
        .with("partOfSeries", SchemaValue::Node(Box::new(series)));
    if let Some(number) = episode.number {
        node.set("episodeNumber", SchemaValue::Integer(i64::from(number)));
    }
    if let Some(published) = page.published {
        node.set("datePublished", SchemaValue::DateTime(published));
    }
    if let Some(audio) = &episode.audio {
        let mut media =
            SchemaNode::new("AudioObject").with("contentUrl", SchemaValue::Url(audio.clone()));
        if let Some(duration) = episode.duration {
            media.set("duration", SchemaValue::Duration(duration));
        }
        node.set("associatedMedia", SchemaValue::Node(Box::new(media)));
    }
    Ok(node)
}
