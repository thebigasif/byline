//! `khalid new "Title"`: a post file with correct front matter and a slug made
//! from the title in the post's language (docs/plan.md section 4.1).

use std::fs;
use std::io;
use std::path::Path;

use clap::Args;
use khalid_core::{Clock, Config, Locale, Slug};
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::init::quoted;
use crate::new_author::{self, Byline};
use crate::new_template::{self, Template};
use crate::{atomic, new_message, report};

/// What `khalid new` takes on the command line.
#[derive(Debug, Args)]
pub(crate) struct NewArgs {
    /// The post's title
    pub(crate) title: String,
    /// The language the post is written in [default: the site's default language]
    #[arg(long, value_name = "CODE")]
    pub(crate) language: Option<Locale>,
    /// Start from the shape of a kind of post, with its sections and table
    /// in place and guidance that never shows on the page. The title must
    /// read as that kind, such as "Ahrefs vs Semrush" for a comparison
    #[arg(long, value_name = "KIND")]
    pub(crate) template: Option<Template>,
    /// The writer's name, such as "Sam Lee". Khalid makes their profile in
    /// `content/<language>/authors/` when there is none. Left out, the post
    /// names the language's only author, if it has one
    #[arg(long, value_name = "NAME")]
    pub(crate) author: Option<String>,
}

/// What `new` writes in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NewPost<'a> {
    /// The headline, as typed.
    pub(crate) title: &'a str,
    /// The language from `--language`, or `None` for the site's default.
    pub(crate) language: Option<&'a Locale>,
    /// The kind of post from `--template`, or `None` for an empty body.
    pub(crate) template: Option<Template>,
    /// The writer's name from `--author`, or `None` to take the language's
    /// only profile.
    pub(crate) author: Option<&'a str>,
}

impl<'a> From<&'a NewArgs> for NewPost<'a> {
    fn from(args: &'a NewArgs) -> Self {
        Self {
            title: &args.title,
            language: args.language.as_ref(),
            template: args.template,
            author: args.author.as_deref(),
        }
    }
}

/// The JSON shape of `khalid new --json`. `path` is relative to the site.
#[derive(Serialize)]
struct NewBody<'a> {
    path: &'a str,
    title: &'a str,
    slug: &'a str,
    language: &'a str,
    date: String,
    author: Option<&'a str>,
    /// The author profile `new` made, relative to the site.
    profile: Option<&'a str>,
}

/// Writes a new post in the `site` folder, dated today by `clock`, into
/// `content/<language>/posts/<date>-<slug>.md`.
pub(crate) fn run(
    site: &Path,
    config: &Config,
    post: NewPost<'_>,
    clock: &dyn Clock,
    output: Output,
) -> Exit {
    let locale = post.language.unwrap_or(config.default_language());
    if !config.languages().contains(locale) {
        let message = new_message::language(locale, config.languages());
        return report::failure(output, &message, Exit::Failure);
    }
    let title = post.title.trim();
    let Ok(slug) = Slug::from_title(title, locale, config.slugs()) else {
        return report::failure(output, &new_message::no_slug(title), Exit::Failure);
    };
    let date = clock.now().date_naive().to_string();
    let tail = match post.template {
        None => EMPTY_BODY.to_owned(),
        Some(template) => {
            let Some(starter) = new_template::body(template, title, locale) else {
                let message = new_message::not_kind(title, template);
                return report::failure(output, &message, Exit::Failure);
            };
            starter
        }
    };
    let byline = match new_author::choose(site, config, locale, post.author) {
        Ok(byline) => byline,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let text = post_text(title, (&slug, byline.slug()), &date, &tail);
    let posts = format!("content/{locale}/posts");
    let name = format!("{date}-{slug}.md");
    let relative = format!("{posts}/{name}");
    match write(site, &posts, &name, &text) {
        Ok(()) => {
            // The profile is made only once the post is written, so a
            // refused post leaves nothing behind.
            let profile = match new_author::make_profile(site, locale, &byline) {
                Ok(profile) => profile,
                Err(message) => return report::failure(output, &message, Exit::Failure),
            };
            let body = NewBody {
                path: &relative,
                title,
                slug: slug.as_str(),
                language: locale.as_str(),
                date,
                author: byline.slug().map(Slug::as_str),
                profile: profile.as_deref(),
            };
            success(output, site, &body, &byline)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            report::failure(output, &new_message::exists(&relative), Exit::Failure)
        }
        Err(error) => report::failure(
            output,
            &new_message::write(&relative, &error),
            Exit::Failure,
        ),
    }
}

/// The front matter's closing line and an empty body.
const EMPTY_BODY: &str = "---\n\n";

/// The text of a new post: its front matter fields, with `author` when one
/// was chosen, then `tail`, which closes the front matter and holds the
/// body: [`EMPTY_BODY`], or a starter whose guidance sits in comments above
/// the closing line.
fn post_text(
    title: &str,
    (slug, author): (&Slug, Option<&Slug>),
    date: &str,
    tail: &str,
) -> String {
    let author = author.map_or_else(String::new, |author| {
        format!("author: {}\n", quoted(author.as_str()))
    });
    format!(
        "---\ntitle: {}\nslug: {}\ndate: {date}\n{author}{tail}",
        quoted(title),
        quoted(slug.as_str())
    )
}

/// Makes the `folder` in `site` when missing, then writes `text` to the new
/// file `name` in it, atomically (RULES.md BL-RUN-9).
pub(crate) fn write(site: &Path, folder: &str, name: &str, text: &str) -> io::Result<()> {
    let mut path = site.to_path_buf();
    path.push(folder);
    fs::create_dir_all(&path)?;
    atomic::create(&path, name, text)
}

/// Says what `new` wrote, who it names as the author, and what to do next.
fn success(output: Output, site: &Path, body: &NewBody<'_>, byline: &Byline<'_>) -> Exit {
    match output {
        Output::Human => {
            let mut said = format!("Wrote {} in {}.\n", body.path, site.display());
            said.push_str(&new_message::byline(byline, body.profile));
            said.push_str("Open it and write the post below the front matter.\n");
            report::text(&said, Exit::Success)
        }
        Output::Json => report::json(body, Exit::Success),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use khalid_content::parse_post;

    use super::*;

    #[test]
    fn the_post_text_reads_back_with_the_same_fields() {
        let title = "Links: \"why\" \\ how\u{7}";
        let slug = Slug::try_new("links-why-how").unwrap();
        let author = Slug::try_new("sam-lee").unwrap();
        let text = post_text(title, (&slug, Some(&author)), "2026-10-26", EMPTY_BODY);
        let post = parse_post(Path::new("post.md"), &text).unwrap();
        assert_eq!(post.front.title.as_deref(), Some(title));
        assert_eq!(post.front.slug, Some(slug));
        assert_eq!(post.front.author, Some(author));
        assert_eq!(post.front.date.unwrap().to_string(), "2026-10-26");
        assert_eq!(post.body.trim(), "");
    }
}
