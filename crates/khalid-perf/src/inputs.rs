//! A site read once and turned into what the renderer and the linter take.
//!
//! The server's own wiring (task 1.15) is not on `main` yet, so this is a
//! small stand-in for it: enough of each record that rendering and linting
//! do their full work, built before any clock starts.

use std::path::Path;

use khalid_content::{ContentTree, Document, PostFront, read_content, render};
use khalid_core::{AbsoluteUrl, Config, Environment};
use khalid_lint::{Heading, HeadingLevel, LinkKind, LintLink, LintPost, LintSite};
use khalid_render::{
    BodyHtml, Link, Nav, PageBody, PageKind, PageMeta, RenderPost, RenderSite, Robots,
};

use crate::PrepareError;

/// One post page, ready to render.
#[derive(Debug, Clone)]
pub struct Page {
    /// The head of the page.
    pub meta: PageMeta,
    /// The post itself.
    pub body: PageBody,
}

/// A generated site's posts as render and lint inputs, in file order.
#[derive(Debug, Clone)]
pub struct Prepared {
    /// The site every page is rendered in.
    pub site: RenderSite,
    /// Every post page.
    pub pages: Vec<Page>,
    /// Every post in the linter's model.
    pub lint: LintSite,
}

impl Prepared {
    /// Reads the site in `dir`: its settings, every content file, and the
    /// rendered body of every post.
    ///
    /// # Errors
    ///
    /// [`PrepareError`] when the settings, a content file or a post's body
    /// cannot be read, or the site address cannot form a page address.
    pub fn read(dir: &Path) -> Result<Self, PrepareError> {
        let config = Config::load(dir, &Environment::empty())?;
        let content = read_content(&ContentTree::walk(dir)?)?;
        let site = RenderSite {
            name: config.name().to_owned(),
            locale: config.default_language().clone(),
            home: config.mount().to_owned(),
            stylesheet: format!("{}assets/site.css", config.mount()),
            nav: Nav::default(),
            main_site: None,
            share_image: None,
        };
        let mut prepared = Self {
            site,
            pages: Vec::new(),
            lint: LintSite::default(),
        };
        for post in &content.posts {
            prepared.add(post, &config)?;
        }
        prepared.lint.locales = config.languages().to_vec();
        Ok(prepared)
    }

    fn add(&mut self, post: &Document<PostFront>, config: &Config) -> Result<(), PrepareError> {
        let front = &post.front;
        let (Some(slug), Some(date)) = (&front.slug, front.date) else {
            return Ok(());
        };
        let rendered = render(&post.body, &post.locale, config.slugs())?;
        let path = format!("{}{slug}/", config.mount());
        let title = front.title.clone().unwrap_or_default();
        let author = front
            .author
            .as_ref()
            .map(|author| author.as_str().to_owned());
        let record = RenderPost {
            title: title.clone(),
            date,
            updated: None,
            authors: author
                .iter()
                .map(|name| Link {
                    name: name.clone(),
                    href: format!("{}authors/{name}/", config.mount()),
                })
                .collect(),
            category: None,
            tags: Vec::new(),
            image: None,
            body: BodyHtml::from_sanitised(rendered.html.clone()),
        };
        let meta = PageMeta {
            title: title.clone(),
            description: front.description.clone().unwrap_or_default(),
            canonical: AbsoluteUrl::parse(&format!("https://www.example.com{path}"))?,
            path: path.clone(),
            kind: PageKind::Article,
            robots: Robots::Index,
            image: None,
            x_default: None,
            alternates: Vec::new(),
        };
        self.pages.push(Page {
            meta,
            body: PageBody::Post(Box::new(record)),
        });
        let mut lint = LintPost::new(post.locale.clone(), slug.clone());
        lint.path = Some(path);
        lint.title = Some(title);
        lint.description.clone_from(&front.description);
        lint.date = Some(date);
        lint.author = author;
        lint.headings = rendered
            .headings
            .iter()
            .filter_map(|h| {
                let level = match h.level {
                    1 => HeadingLevel::H1,
                    2 => HeadingLevel::H2,
                    3 => HeadingLevel::H3,
                    4 => HeadingLevel::H4,
                    5 => HeadingLevel::H5,
                    6 => HeadingLevel::H6,
                    _ => return None,
                };
                Some(Heading::new(level, h.text.clone()))
            })
            .collect();
        lint.links = rendered
            .links
            .iter()
            .map(|link| {
                let kind = if link.internal {
                    LinkKind::Internal
                } else {
                    LinkKind::External
                };
                LintLink::new(kind, link.href.clone(), link.text.clone())
            })
            .collect();
        lint.word_count = rendered.words;
        lint.body_text = plain_text(&rendered.html);
        self.lint.posts.push(lint);
        Ok(())
    }
}

/// The words of `html`, with every tag replaced by a space.
fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text
}
