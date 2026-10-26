//! The sandboxed template engine that turns a page into HTML.

use minijinja::value::Serde;
use minijinja::{AutoEscape, Environment, UndefinedBehavior, context};

use crate::direction::Direction;
use crate::escape;
use crate::strings::Strings;
use crate::templates;
use crate::{PageBody, PageMeta, RenderError, RenderSite};

/// How many steps one page may take before rendering stops. The default
/// pages use a few thousand; the cap stops a theme template that loops
/// without end from holding a worker (RULES.md BL-SEC-10).
const FUEL_PER_PAGE: u64 = 1_000_000;

/// Renders pages with the default templates.
///
/// The environment is sandboxed (RULES.md BL-SEC-10): it has no loader, so
/// templates can include only each other, and no function that reads files,
/// the network or the clock. Every value is escaped for HTML unless the
/// crate marked it safe, such as a post body that was sanitised before it
/// arrived. A missing value is an error, not an empty string.
#[derive(Debug)]
pub struct Renderer {
    env: Environment<'static>,
    strings: Strings,
}

impl Renderer {
    /// A renderer with the default templates only.
    ///
    /// # Errors
    ///
    /// Returns a [`RenderError`] if a built-in template or strings file is
    /// broken, which this crate's tests rule out.
    pub fn new() -> Result<Self, RenderError> {
        let mut env = Environment::new();
        env.set_auto_escape_callback(|_| AutoEscape::Html);
        env.set_formatter(escape::format);
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        env.set_fuel(Some(FUEL_PER_PAGE));
        for source in templates::defaults() {
            env.add_template_owned(source.name.clone(), source.text)
                .map_err(|error| RenderError::compile(&source.name, error))?;
        }
        Ok(Self {
            env,
            strings: Strings::load()?,
        })
    }

    /// Renders one page of `site` with the head `meta` and the main content
    /// `body`.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Render`] when the template fails, for example
    /// a theme template that names a value the page does not have.
    pub fn render(
        &self,
        site: &RenderSite,
        meta: &PageMeta,
        body: &PageBody,
    ) -> Result<String, RenderError> {
        let name = body.template();
        let template = self
            .env
            .get_template(name)
            .map_err(|error| RenderError::render(name, error))?;
        let context = context! {
            site => Serde(site),
            meta => Serde(meta),
            t => Serde(self.strings.for_locale(&site.locale)),
            lang => site.locale.as_str(),
            dir => Serde(Direction::of(&site.locale)),
            page => body.value(),
        };
        template
            .render(context)
            .map_err(|error| RenderError::render(name, error))
    }
}
