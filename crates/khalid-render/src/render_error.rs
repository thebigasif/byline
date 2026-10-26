//! What can go wrong while loading or rendering templates.

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
