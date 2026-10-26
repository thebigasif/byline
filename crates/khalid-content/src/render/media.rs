//! The HTML of the media blocks.

use super::{Renderer, attribute};
use crate::{StructuredBlock, VideoBlock};

impl Renderer {
    /// Writes a video with controls that loads nothing until played, its
    /// file's link for browsers that cannot play it, and its caption.
    pub(super) fn video(&mut self, video: &VideoBlock) {
        let src = attribute(&video.src);
        let title = attribute(&video.title);
        let caption = attribute(video.description.as_deref().unwrap_or(&video.title));
        self.push(&[
            "<figure class=\"video\">\n<video controls preload=\"none\" poster=\"",
            &attribute(&video.poster),
            "\" aria-label=\"",
            &title,
            "\"><source src=\"",
            &src,
            "\" /><a href=\"",
            &src,
            "\">",
            &title,
            "</a></video>\n<figcaption>",
            &caption,
            "</figcaption>\n</figure>\n",
        ]);
        self.facts
            .structured
            .push(StructuredBlock::Video(video.clone()));
    }
}
