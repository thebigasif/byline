//! `khalid new "Title"`: a post file with correct front matter and a slug made
//! from the title in the post's language (docs/plan.md section 4.1).

use std::fs;
use std::io;
use std::path::Path;

use clap::Args;
use khalid_core::{Clock, Config, Locale, Slug, UserMessage};
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::init::quoted;
use crate::new_template::{self, Template};
use crate::{atomic, new_branch, new_message, report};

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
    /// Git mode: start a branch named post/ and the post's slug, and commit the new post on
    /// it, so merging the branch publishes the post
    #[arg(long)]
    pub(crate) branch: bool,
    /// Write an undated draft in `drafts/<language>/<slug>.md`, which
    /// readers never see until `khalid publish` takes it live
    #[arg(long)]
    pub(crate) draft: bool,
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
    /// Whether `--branch` asks for the post on its own git branch.
    pub(crate) branch: bool,
    /// Whether to write an undated draft in `drafts/` from `--draft`.
    pub(crate) draft: bool,
}

impl<'a> From<&'a NewArgs> for NewPost<'a> {
    fn from(args: &'a NewArgs) -> Self {
        Self {
            title: &args.title,
            language: args.language.as_ref(),
            template: args.template,
            branch: args.branch,
            draft: args.draft,
        }
    }
}

/// The JSON shape of `khalid new --json`. `path` is relative to the site;
/// `branch` is there with `--branch`. `date` is `null` for a draft.
#[derive(Serialize)]
struct NewBody<'a> {
    path: &'a str,
    title: &'a str,
    slug: &'a str,
    language: &'a str,
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<&'a str>,
}

/// Writes a new post in the `site` folder, dated today by `clock`, into
/// `content/<language>/posts/<date>-<slug>.md`, or with `--draft`, undated
/// into `drafts/<language>/<slug>.md` (ADR 0030).
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
    let date = (!post.draft).then(|| clock.now().date_naive().to_string());
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
    let text = post_text(title, &slug, date.as_deref(), &tail);
    let (posts, name) = match &date {
        Some(date) => (
            format!("content/{locale}/posts"),
            format!("{date}-{slug}.md"),
        ),
        None => (format!("drafts/{locale}"), format!("{slug}.md")),
    };
    let relative = format!("{posts}/{name}");
    let branch = post.branch.then(|| new_branch::branch_for(&slug));
    match written(site, (&posts, &name, &text), branch.as_deref(), title) {
        Ok(()) => {
            let body = NewBody {
                path: &relative,
                title,
                slug: slug.as_str(),
                language: locale.as_str(),
                date,
                branch: branch.as_deref(),
            };
            let language = if locale == config.default_language() {
                String::new()
            } else {
                format!(" --language {locale}")
            };
            success(output, site, &body, &language)
        }
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Writes the post `text` as `name` in the `posts` folder of `site`, and
/// with a `branch`, first starts it and then commits the post on it.
fn written(
    site: &Path,
    (posts, name, text): (&str, &str, &str),
    branch: Option<&str>,
    title: &str,
) -> Result<(), UserMessage> {
    let relative = format!("{posts}/{name}");
    if let Some(branch) = branch {
        let mut file = site.to_path_buf();
        file.push(&relative);
        if file.exists() {
            return Err(new_message::exists(&relative));
        }
        new_branch::start(site, branch)?;
    }
    write(site, posts, name, text).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            new_message::exists(&relative)
        } else {
            new_message::write(&relative, &error)
        }
    })?;
    branch.map_or(Ok(()), |_| new_branch::commit(site, &relative, title))
}

/// The front matter's closing line and an empty body.
const EMPTY_BODY: &str = "---\n\n";

/// The text of a new post: its front matter fields, then `tail`, which
/// closes the front matter and holds the body: [`EMPTY_BODY`], or a
/// starter whose guidance sits in comments above the closing line. A
/// draft has no `date`.
fn post_text(title: &str, slug: &Slug, date: Option<&str>, tail: &str) -> String {
    let date = date.map(|date| format!("date: {date}\n"));
    format!(
        "---\ntitle: {}\nslug: {}\n{}{tail}",
        quoted(title),
        quoted(slug.as_str()),
        date.unwrap_or_default()
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

/// Says what `new` wrote and what to do next: for a draft, the command
/// that publishes it, with `language`, ` --language <code>` or nothing.
fn success(output: Output, site: &Path, body: &NewBody<'_>, language: &str) -> Exit {
    let next = match (body.date.is_some(), body.branch) {
        (true, Some(_)) => String::new(),
        (true, None) => "It goes live at the next build. To keep a new post hidden, make it \
                         with --draft, which writes it in drafts/.\n"
            .to_owned(),
        (false, _) => format!(
            "Readers cannot see it until you run `khalid --site {} publish {}{language}`.\n",
            site.display(),
            body.slug
        ),
    };
    match output {
        Output::Human => {
            let mut said = format!(
                "Wrote {} in {}.\nOpen it and write the post below the front matter.\n{next}",
                body.path,
                site.display()
            );
            if let Some(branch) = body.branch {
                let next = format!(
                    "It is committed on the branch {branch}. Commit your writing there, then \
                     `git push -u origin {branch}` and open a pull request. Merging it publishes the post.\n"
                );
                said.push_str(&next);
            }
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
        let text = post_text(title, &slug, Some("2026-10-26"), EMPTY_BODY);
        let post = parse_post(Path::new("post.md"), &text).unwrap();
        assert_eq!(post.front.title.as_deref(), Some(title));
        assert_eq!(post.front.slug, Some(slug.clone()));
        assert_eq!(post.front.date.unwrap().to_string(), "2026-10-26");
        assert_eq!(post.body.trim(), "");

        let draft = post_text(title, &slug, None, EMPTY_BODY);
        let post = parse_post(Path::new("post.md"), &draft).unwrap();
        assert_eq!((post.front.slug, post.front.date), (Some(slug), None));
    }
}
