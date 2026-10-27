//! One page of the site before it is drawn, and how it is drawn.

use std::path::{Path, PathBuf};

use khalid_core::{TemplateName, UserMessage};
use khalid_render::{PageBody, PageMeta, RenderSite, Renderer};
use khalid_seo::SeoPage;

use crate::build_message;

/// One page and the file it goes to, relative to the output folder.
#[derive(Debug)]
pub(crate) struct Page {
    pub(crate) file: String,
    pub(crate) meta: PageMeta,
    pub(crate) body: PageBody,
    /// The page as the SEO outputs read it, before its language versions
    /// are linked by `build_seo::heads`.
    pub(crate) seo: SeoPage,
    /// The content file it was made from, relative to the site folder, so
    /// `post_files` can find the files beside it. `None` for lists.
    pub(crate) source: Option<PathBuf>,
    /// The theme's post template the page asks for, such as `wide`, in
    /// place of its type's own.
    pub(crate) template: Option<TemplateName>,
}

impl Page {
    /// The page, made from the content file at `path`.
    pub(crate) fn made_from(self, path: &Path) -> Self {
        Self {
            source: Some(path.to_path_buf()),
            ..self
        }
    }

    /// The page as HTML, drawn by `renderer` for `site` with the template
    /// it asks for.
    ///
    /// # Errors
    ///
    /// Returns a message naming the post and the file when the theme has
    /// no template it asks for, or when a template fails.
    pub(crate) fn rendered(
        &self,
        renderer: &Renderer,
        site: &RenderSite,
    ) -> Result<String, UserMessage> {
        let html = match &self.template {
            Some(name) if !renderer.has_template(name) => {
                let source = self
                    .source
                    .as_deref()
                    .unwrap_or_else(|| Path::new(&self.file));
                return Err(build_message::no_template(source, name));
            }
            Some(name) => renderer.render_as(name, site, &self.meta, &self.body),
            None => renderer.render(site, &self.meta, &self.body),
        };
        html.map_err(|error| build_message::render(&error))
    }
}
