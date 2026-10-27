//! One page of the site before it is drawn, and how it is drawn.

use std::path::{Path, PathBuf};

use khalid_core::{HeadCode, TemplateName, UserMessage};
use khalid_render::{HeadHtml, PageBody, PageMeta, RenderSite, Renderer};
use khalid_seo::SeoPage;
use khalid_server::NONCE_SLOT;

use crate::build_message;

/// One page and the file it goes to, relative to the output folder.
#[derive(Debug, Clone)]
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
    /// The post's own head code, added after the SEO head. `None` when it
    /// has none, or only comments.
    pub(crate) head_code: Option<HeadCode>,
}

/// Where a page's head code gets its nonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeadNonce {
    /// From the server on each request, through [`NONCE_SLOT`].
    PerRequest,
    /// Nowhere: a static host sends its own policy, or none.
    Static,
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
    /// it asks for, and its head code after the SEO head, with `nonce`.
    ///
    /// # Errors
    ///
    /// Returns a message naming the post and the file when the theme has
    /// no template it asks for, or when a template fails.
    pub(crate) fn rendered(
        &self,
        renderer: &Renderer,
        site: &RenderSite,
        nonce: HeadNonce,
    ) -> Result<String, UserMessage> {
        let Some(code) = &self.head_code else {
            return self.drawn(renderer, site, &self.meta);
        };
        let code = match nonce {
            HeadNonce::PerRequest => code.to_html(NONCE_SLOT),
            HeadNonce::Static => code.to_string(),
        };
        let mut meta = self.meta.clone();
        let seo = meta.seo_head.as_ref().map_or("", HeadHtml::as_str);
        // The head code was checked by HeadCode, which writes every
        // attribute quoted and keeps only head elements.
        meta.seo_head = Some(HeadHtml::from_trusted(format!("{seo}\n{code}")));
        self.drawn(renderer, site, &meta)
    }

    /// The page as HTML with the head `meta`.
    fn drawn(
        &self,
        renderer: &Renderer,
        site: &RenderSite,
        meta: &PageMeta,
    ) -> Result<String, UserMessage> {
        let html = match &self.template {
            Some(name) if !renderer.has_template(name) => {
                let source = self
                    .source
                    .as_deref()
                    .unwrap_or_else(|| Path::new(&self.file));
                return Err(build_message::no_template(source, name));
            }
            Some(name) => renderer.render_as(name, site, meta, &self.body),
            None => renderer.render(site, meta, &self.body),
        };
        html.map_err(|error| build_message::render(&error))
    }
}
