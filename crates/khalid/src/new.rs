//! `khalid new "Title"`: a post file with correct front matter and a slug made
//! from the title in the post's language (docs/plan.md section 4.1).

use std::fs;
use std::io;
use std::path::Path;

use khalid_core::{Clock, Config, Locale, Slug};
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::init::quoted;
use crate::{atomic, new_message, report};

/// What `new` writes in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NewPost<'a> {
    /// The headline, as typed.
    pub(crate) title: &'a str,
    /// The language from `--language`, or `None` for the site's default.
    pub(crate) language: Option<&'a Locale>,
}

/// The JSON shape of `khalid new --json`. `path` is relative to the site.
#[derive(Serialize)]
struct NewBody<'a> {
    path: &'a str,
    title: &'a str,
    slug: &'a str,
    language: &'a str,
    date: String,
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
    let posts = format!("content/{locale}/posts");
    let name = format!("{date}-{slug}.md");
    let relative = format!("{posts}/{name}");
    match write(site, &posts, &name, &post_text(title, &slug, &date)) {
        Ok(()) => {
            let body = NewBody {
                path: &relative,
                title,
                slug: slug.as_str(),
                language: locale.as_str(),
                date,
            };
            success(output, site, &body)
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

/// The text of a new post: its front matter and an empty body.
fn post_text(title: &str, slug: &Slug, date: &str) -> String {
    format!(
        "---\ntitle: {}\nslug: {}\ndate: {date}\n---\n\n",
        quoted(title),
        quoted(slug.as_str())
    )
}

/// Makes the `posts` folder in `site` when missing, then writes `text` to the
/// new file `name` in it, atomically (RULES.md BL-RUN-9).
fn write(site: &Path, posts: &str, name: &str, text: &str) -> io::Result<()> {
    let mut folder = site.to_path_buf();
    folder.push(posts);
    fs::create_dir_all(&folder)?;
    atomic::create(&folder, name, text)
}

/// Says what `new` wrote and what to do next.
fn success(output: Output, site: &Path, body: &NewBody<'_>) -> Exit {
    match output {
        Output::Human => report::text(
            &format!(
                "Wrote {} in {}.\nOpen it and write the post below the front matter.\n",
                body.path,
                site.display()
            ),
            Exit::Success,
        ),
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
        let text = post_text(title, &slug, "2026-10-26");
        let post = parse_post(Path::new("post.md"), &text).unwrap();
        assert_eq!(post.front.title.as_deref(), Some(title));
        assert_eq!(post.front.slug, Some(slug));
        assert_eq!(post.front.date.unwrap().to_string(), "2026-10-26");
        assert_eq!(post.body.trim(), "");
    }
}
