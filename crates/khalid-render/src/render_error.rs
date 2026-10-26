//! What can go wrong while loading or rendering templates.

use std::io;
use std::path::PathBuf;

use khalid_core::SitePathError;

/// Why a template could not be loaded or a page could not be rendered.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// A template has a syntax mistake.
    #[error("template {name} could not be read as a template")]
    Compile {
        /// The template's name, such as `post.html`.
        name: String,
        /// The line of the mistake, when the engine knows it.
        line: Option<usize>,
        /// What the template engine reported.
        #[source]
        source: Box<minijinja::Error>,
    },
    /// A template failed while it was filled in, for example by naming a
    /// value the page does not have.
    #[error("template {name} failed while rendering")]
    Render {
        /// The template's name, such as `post.html`.
        name: String,
        /// The line that failed, when the engine knows it.
        line: Option<usize>,
        /// What the template engine reported.
        #[source]
        source: Box<minijinja::Error>,
    },
    /// A file in `theme/templates/` has a name no default template has.
    #[error("theme/templates/{name} does not replace any default template")]
    UnknownTemplate {
        /// The file name that matched nothing.
        name: String,
    },
    /// A theme template could not be read from disk.
    #[error("could not read {}", path.display())]
    ReadTemplate {
        /// The file that could not be read.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
    /// The theme folder could not be resolved inside the site folder.
    #[error("the theme folder could not be used")]
    ThemePath(#[from] SitePathError),
    /// A built-in strings file is not valid TOML. A build mistake, caught by
    /// this crate's tests.
    #[error("the built-in strings file for {language} is broken")]
    Strings {
        /// The language of the broken file.
        language: String,
        /// What the TOML parser reported.
        #[source]
        source: toml::de::Error,
    },
    /// `theme/tokens.toml` has a value that is not a colour, a font list or
    /// a length, a field no token has, or broken TOML.
    #[error("theme/tokens.toml has a mistake{}", line.map(|line| format!(" on line {line}")).unwrap_or_default())]
    Tokens {
        /// The line of the mistake, when the parser knows it.
        line: Option<usize>,
        /// What the TOML parser reported, which names the field.
        #[source]
        source: toml::de::Error,
    },
    /// A built asset could not be compressed.
    #[error("an asset could not be compressed")]
    Compress {
        /// What the compressor reported.
        #[source]
        source: io::Error,
    },
    /// `theme/tokens.toml` exists but could not be read.
    #[error("could not read {}", path.display())]
    ReadTokens {
        /// The file that could not be read.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
}

impl RenderError {
    pub(crate) fn compile(name: &str, source: minijinja::Error) -> Self {
        Self::Compile {
            name: name.to_owned(),
            line: source.line(),
            source: Box::new(source),
        }
    }

    pub(crate) fn render(name: &str, source: minijinja::Error) -> Self {
        Self::Render {
            name: name.to_owned(),
            line: source.line(),
            source: Box::new(source),
        }
    }
}
