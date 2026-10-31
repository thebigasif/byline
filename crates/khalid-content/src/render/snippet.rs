//! Writing a body's blocks, and the snippet block, which writes the blocks
//! of a snippet file in its place.

use super::{Renderer, is_markdown};
use crate::{Block, parse_blocks};

impl Renderer<'_> {
    /// Writes `blocks` in order: runs of markdown together, so footnotes
    /// resolve within a run, and each directive on its own.
    pub(super) fn blocks(&mut self, blocks: &[Block]) {
        let mut run: Vec<&str> = Vec::new();
        for block in blocks {
            if is_markdown(&block.kind) {
                run.push(&block.source);
            } else {
                self.markdown(&run.join("\n\n"));
                run.clear();
                self.directive(block);
            }
        }
        self.markdown(&run.join("\n\n"));
    }

    /// Writes the blocks of the snippet called `slug`, as the site holds
    /// it now. A snippet the site does not have writes nothing, since
    /// checking for one is the content reader's job, and so does a snippet
    /// placed inside another, so a snippet can never place itself forever.
    pub(super) fn snippet(&mut self, slug: &str) {
        let links = self.wikilinks;
        let Some(body) = links.snippet(slug).filter(|_| !self.in_snippet) else {
            return;
        };
        let Ok(blocks) = parse_blocks(body) else {
            return;
        };
        self.in_snippet = true;
        self.blocks(&blocks);
        self.in_snippet = false;
    }
}
