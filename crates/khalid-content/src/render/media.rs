//! The HTML of the media blocks: video, audio, file download and gallery.

use super::{Image, Link, Renderer, attribute};
use crate::{GalleryImage, StructuredBlock, VideoBlock};

impl Renderer<'_> {
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

    /// Writes a sound file with controls under its title.
    pub(super) fn audio(&mut self, src: &str, title: &str) {
        let src = attribute(src);
        let title = attribute(title);
        self.push(&[
            "<figure class=\"audio\">\n<figcaption>",
            &title,
            "</figcaption>\n<audio controls preload=\"none\" src=\"",
            &src,
            "\"><a href=\"",
            &src,
            "\">",
            &title,
            "</a></audio>\n</figure>\n",
        ]);
    }

    /// Writes a link that downloads a file.
    pub(super) fn file(&mut self, src: &str, title: &str) {
        self.push(&[
            "<p class=\"file\"><a href=\"",
            &attribute(src),
            "\" download>",
            &attribute(title),
            "</a></p>\n",
        ]);
        self.facts.links.push(Link {
            href: src.to_owned(),
            text: title.to_owned(),
            internal: !src.contains(':'),
        });
    }

    /// Writes a grid of pictures, each loaded as it scrolls into view.
    pub(super) fn gallery(&mut self, images: &[GalleryImage]) {
        self.html.push_str("<div class=\"gallery\">\n");
        for image in images {
            self.push(&[
                "<figure><img src=\"",
                &attribute(&image.src),
                "\" alt=\"",
                &attribute(&image.alt),
                "\" loading=\"lazy\" decoding=\"async\" /></figure>\n",
            ]);
            self.facts.images.push(Image {
                src: image.src.clone(),
                alt: Some(image.alt.clone()),
            });
        }
        self.html.push_str("</div>\n");
    }
}
