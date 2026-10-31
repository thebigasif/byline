//! One page of the site before it is drawn, and how it is drawn.

use std::path::{Path, PathBuf};

use khalid_content::LeadImage;
use khalid_core::{HeadCode, TemplateName, UserMessage};
use khalid_render::{HeadHtml, PageBody, PageMeta, RenderSite, Renderer};
use khalid_seo::SeoPage;
use khalid_server::{MediaType, NONCE_SLOT, PagePath};

use crate::build_message;
use crate::page_code::PageCode;

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
    /// The post's lead image as written, which `post_images` draws once
    /// it has made its sizes. `None` on every other page.
    pub(crate) lead: Option<LeadImage>,
}

/// `html`, a rendered page, as `khalid serve` keeps it at `address`:
/// filled per request when it holds a nonce slot, for a post's head code
/// or the site's own.
///
/// # Errors
///
/// Returns a message when `address` cannot be a page's address.
pub(crate) fn served_at(address: &str, html: String) -> Result<khalid_server::Page, UserMessage> {
    let path = PagePath::try_new(address).map_err(|_| build_message::address(address))?;
    Ok(if html.contains(NONCE_SLOT) {
        khalid_server::Page::with_nonce_slot(path, MediaType::Html, html)
    } else {
        khalid_server::Page::new(path, MediaType::Html, html)
    })
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
    /// it asks for, with its head code and the site's own `code` placed as
    /// `code` says: after the SEO head, and before the body ends.
    ///
    /// # Errors
    ///
    /// Returns a message naming the post and the file when the theme has
    /// no template it asks for, or when a template fails.
    pub(crate) fn rendered(
        &self,
        renderer: &Renderer,
        site: &RenderSite,
        code: &PageCode,
    ) -> Result<String, UserMessage> {
        let head = code.head(self.head_code.as_ref());
        let html = if head.is_empty() {
            self.drawn(renderer, site, &self.meta)?
        } else {
            let mut meta = self.meta.clone();
            let seo = meta.seo_head.as_ref().map_or("", HeadHtml::as_str);
            // Head code was checked by HeadCode, which writes every
            // attribute quoted and keeps only head elements.
            meta.seo_head = Some(HeadHtml::from_trusted(format!("{seo}\n{head}")));
            self.drawn(renderer, site, &meta)?
        };
        let page = (renderer, &site.locale);
        Ok(code.with_body_end(html, page, &self.meta.path))
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
