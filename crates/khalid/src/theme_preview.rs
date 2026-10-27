//! The theme preview: a sample post drawn with synced tokens (plan section
//! 4.9). It lives in the binary, which may use both `khalid-theme` and
//! `khalid-render` (ADR 0020).

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use khalid_core::{AbsoluteUrl, Locale, SitePath, SitePathError, UserMessage};
use khalid_render::{
    BodyHtml, Features, Link, Nav, NavLink, PageBody, PageKind, PageMeta, Person, ReadingTime,
    RenderError, RenderPost, RenderSite, Renderer, Robots, Tokens, stylesheet,
};
use khalid_theme::ThemeTokens;
use tempfile::NamedTempFile;

/// Where the preview is written. It is derived, so it lives under `data/`,
/// never under `content/` (RULES.md BL-FS-2).
const FOLDER: &str = "data/theme-preview";

/// The page's file name, which its own links point at.
const PAGE: &str = "index.html";

/// The stylesheet's file name, beside the page.
const STYLESHEET: &str = "site.css";

/// The sample post's body. Each part says which token it shows, so the
/// person checking the preview knows what to look at.
const SAMPLE_BODY: &str = r#"<p>This paragraph is body text, in the body font and the text colour. <a href="index.html">A link looks like this</a>, in the link colour.</p>
<h2>A heading in the heading font</h2>
<p>Text runs no wider than the theme's line length, so it stays easy to read on a wide screen.</p>
<blockquote><p>A quote stands apart, so readers see it is someone else's words.</p></blockquote>
<h3>A list and some code</h3>
<ul><li>Lists keep the body font.</li><li>Lines between parts of the page use the border colour.</li></ul>
<pre><code>khalid theme sync https://northwind.example/</code></pre>
<p>Code uses the code font on the surface colour, with the theme's corners.</p>"#;

/// Why the preview could not be made.
#[derive(Debug)]
pub(crate) enum PreviewError {
    /// The tokens did not load with the renderer's rules, or a template failed.
    Render(RenderError),
    /// The preview folder leads outside the site through a symlink, or the
    /// site folder cannot be read.
    Path(SitePathError),
    /// A file could not be written.
    Write { path: PathBuf, source: io::Error },
}

impl PreviewError {
    /// What the person reads: what happened, why it matters, what to do
    /// (RULES.md BL-ERR-4).
    pub(crate) fn message(&self) -> UserMessage {
        match self {
            Self::Render(error) => UserMessage::new(
                format!("Khalid could not draw the theme preview: {error}."),
                "The preview shows how the blog will look with the copied colours and fonts.",
                "Fix the value or template the message names, then run the sync again.",
            ),
            Self::Path(error) => UserMessage::new(
                format!("Khalid will not write the preview to {FOLDER}: {error}."),
                "Khalid writes only inside the site folder, so no other file can change.",
                format!(
                    "Make {FOLDER} a plain folder inside the site that you may read, then run the sync again."
                ),
            ),
            Self::Write { path, source } => UserMessage::new(
                format!("Khalid could not write {}: {source}.", path.display()),
                "The preview exists only once its files are written.",
                "Check that you may write to the site folder, then run the sync again.",
            ),
        }
    }
}

/// A rendered preview page and the stylesheet it links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Preview {
    html: String,
    css: Vec<u8>,
}

/// The site the tokens were copied from, as the preview names it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Source<'a> {
    /// The site's name from its home page, when theme sync found one.
    pub(crate) name: Option<&'a str>,
    /// The main site's address.
    pub(crate) url: &'a AbsoluteUrl,
    /// The language the blog's own words are shown in.
    pub(crate) locale: &'a Locale,
}

impl Preview {
    /// Renders the sample post with `tokens` through `renderer`, which holds
    /// the site's own templates.
    ///
    /// The tokens pass through the text of `theme/tokens.toml` and the
    /// renderer's loader, so a preview exists only for a file the blog can
    /// load.
    ///
    /// # Errors
    ///
    /// Returns [`PreviewError::Render`] when the tokens break a renderer
    /// rule or a theme template fails.
    pub(crate) fn render(
        renderer: &Renderer,
        tokens: &ThemeTokens,
        source: Source<'_>,
    ) -> Result<Self, PreviewError> {
        let tokens = Tokens::from_toml(&tokens.to_toml()).map_err(PreviewError::Render)?;
        let css = stylesheet(&tokens).map_err(PreviewError::Render)?;
        let site = sample_site(source);
        let meta = PageMeta {
            title: format!("How {} will look", site.name),
            description: "A sample post in the colours and fonts copied from the main site."
                .to_owned(),
            canonical: source.url.clone(),
            path: PAGE.to_owned(),
            kind: PageKind::Article,
            robots: Robots::NoIndex,
            image: None,
            alternates: Vec::new(),
            x_default: None,
            seo_head: None,
        };
        let post = PageBody::Post(Box::new(sample_post(&site.name, &site.locale)));
        let html = renderer
            .render(&site, &meta, &post)
            .map_err(PreviewError::Render)?;
        Ok(Self {
            html,
            css: css.body().to_vec(),
        })
    }

    /// Writes the page and its stylesheet to `data/theme-preview/` inside
    /// `site_root`, each atomically (RULES.md BL-RUN-9), and returns the
    /// page's path, ready to open in a browser.
    ///
    /// # Errors
    ///
    /// Returns [`PreviewError::Path`] when the folder leads outside the site,
    /// and [`PreviewError::Write`] when a file cannot be written.
    pub(crate) fn write(&self, site_root: &Path) -> Result<PathBuf, PreviewError> {
        write_file(site_root, STYLESHEET, &self.css)?;
        write_file(site_root, PAGE, self.html.as_bytes())
    }
}

/// Writes `bytes` to `name` in the preview folder: a temporary file beside
/// it, flushed to disk, then renamed over the old one.
fn write_file(site_root: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, PreviewError> {
    let path = SitePath::within(site_root, &format!("{FOLDER}/{name}"))
        .and_then(|path| path.resolve())
        .map_err(PreviewError::Path)?;
    let write = || -> io::Result<()> {
        let folder = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
        fs::create_dir_all(folder)?;
        let mut file = NamedTempFile::new_in(folder)?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        file.persist(&path).map_err(|error| error.error)?;
        Ok(())
    };
    write().map_err(|source| PreviewError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// The site around the sample post: one menu link and a link back to the
/// main site.
fn sample_site(source: Source<'_>) -> RenderSite {
    let host = source.url.host();
    RenderSite {
        name: source.name.unwrap_or(host).to_owned(),
        locale: source.locale.clone(),
        home: PAGE.to_owned(),
        stylesheet: STYLESHEET.to_owned(),
        nav: Nav {
            primary: vec![NavLink {
                label: "Sample post".to_owned(),
                href: PAGE.to_owned(),
            }],
            footer: Vec::new(),
        },
        main_site: Some(NavLink {
            label: format!("Back to {host}"),
            href: source.url.to_string(),
        }),
        share_image: None,
        features: Features::all(),
        search: None,
        script: None,
    }
}

/// A post that shows every token, written by the site itself.
fn sample_post(author: &str, locale: &Locale) -> RenderPost {
    let link = |name: &str| Link {
        name: name.to_owned(),
        href: PAGE.to_owned(),
    };
    let body = BodyHtml::from_sanitised(SAMPLE_BODY.to_owned());
    RenderPost {
        title: "How this blog will look".to_owned(),
        date: NaiveDate::from_ymd_opt(2026, 10, 26).unwrap_or_default(),
        updated: None,
        reading_time: ReadingTime::of(&body, locale),
        authors: vec![Person {
            name: author.to_owned(),
            href: PAGE.to_owned(),
            role: None,
            image: None,
            bio: None,
            profiles: Vec::new(),
        }],
        reviewers: Vec::new(),
        category: Some(link("Guides")),
        tags: vec![link("Theme"), link("Colours")],
        image: None,
        contents: None,
        originals: BTreeMap::new(),
        related: Vec::new(),
        body,
    }
}

#[cfg(test)]
#[path = "theme_preview_tests.rs"]
mod tests;
