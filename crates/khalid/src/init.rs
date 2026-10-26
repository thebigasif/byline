//! `khalid init`: a site folder from four answers, the website's address,
//! where the blog lives, the languages and the mode (PRD KH-INS-2).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use khalid_core::{AbsoluteUrl, Config, Environment, Locale};
use khalid_routes::Routes;
use serde::Serialize;

use crate::cli::{InitArgs, Output, SiteMode};
use crate::exit::Exit;
use crate::{init_message, report};

/// The language a site writes in when nothing says otherwise.
const DEFAULT_LANGUAGE: &str = "en";

/// `routes.toml` for a new site: the defaults, which need no settings.
const ROUTES_TOML: &str = "\
# Collections, channels and address patterns. Empty means the defaults (docs/plan.md 4.11).
";

/// The four answers that make a site. `languages` is never empty, and its
/// first is the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Answers {
    pub(crate) address: AbsoluteUrl,
    pub(crate) mount: String,
    pub(crate) languages: Vec<Locale>,
    pub(crate) mode: SiteMode,
}

/// The JSON shape of `khalid init --json`: the site folder, and the files and
/// folders made in it.
#[derive(Serialize)]
struct InitBody {
    site: String,
    created: Vec<String>,
}

/// Makes a site in `folder` from the flags in `args`.
pub(crate) fn run(folder: &Path, args: &InitArgs, output: Output) -> Exit {
    let mut settings = folder.to_path_buf();
    settings.push(Config::FILE_NAME);
    if settings.exists() {
        return report::failure(output, &init_message::exists(&settings), Exit::Failure);
    }
    let Some(answers) = Answers::from_flags(args) else {
        return report::failure(output, &init_message::needs_address(), Exit::Usage);
    };
    let text = answers.site_toml();
    if let Err(error) = Config::from_toml(&settings, &text, &Environment::empty()) {
        return report::failure(output, &init_message::refused(&error), Exit::Failure);
    }
    match write(folder, &answers, &text) {
        Ok(created) => success(output, folder, &answers, created),
        Err(error) => report::failure(output, &init_message::write(folder, &error), Exit::Failure),
    }
}

impl Answers {
    /// The answers the flags give, with the defaults for the rest, or `None`
    /// without `--address`, which has no default.
    pub(crate) fn from_flags(args: &InitArgs) -> Option<Self> {
        Some(Self {
            address: args.address.clone()?,
            mount: mount_path(args.mount.as_deref().unwrap_or("/")),
            languages: if args.languages.is_empty() {
                vec![Locale::try_new(DEFAULT_LANGUAGE).ok()?]
            } else {
                args.languages.clone()
            },
            mode: args.mode.unwrap_or(SiteMode::Server),
        })
    }

    /// The text of `site.toml`. The site's name starts as the website's host.
    fn site_toml(&self) -> String {
        let languages: Vec<String> = self
            .languages
            .iter()
            .map(|locale| quoted(locale.as_str()))
            .collect();
        let default = self.languages.first().map_or_else(
            || quoted(DEFAULT_LANGUAGE),
            |locale| quoted(locale.as_str()),
        );
        format!(
            "# The site's settings, made by `khalid init`.\n\n\
             name = {}\naddress = {}\nmount = {}\nmode = {}\n\
             default_language = {default}\nlanguages = [{}]\n",
            quoted(self.address.host()),
            quoted(self.address.as_str()),
            quoted(&self.mount),
            quoted(self.mode.as_str()),
            languages.join(", "),
        )
    }
}

/// `text` as a mount path: starting and ending with `/`, so `blog` becomes
/// `/blog/` and an empty answer means the whole site.
pub(crate) fn mount_path(text: &str) -> String {
    let inner = text.trim().trim_start_matches('/').trim_end_matches('/');
    if inner.is_empty() {
        "/".to_owned()
    } else {
        format!("/{inner}/")
    }
}

/// `text` as a TOML basic string, with quotes, backslashes and control
/// characters escaped.
fn quoted(text: &str) -> String {
    let escaped: String = text
        .chars()
        .map(|c| match c {
            '"' => "\\\"".to_owned(),
            '\\' => "\\\\".to_owned(),
            c if c.is_control() => format!("\\u{:04X}", u32::from(c)),
            c => c.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

/// Makes the site's folders and files, `site.toml` last so a failed run can be
/// repeated, keeping any `routes.toml`. Returns what it made, relative to `folder`.
fn write(folder: &Path, answers: &Answers, settings: &str) -> io::Result<Vec<String>> {
    let mut created = vec![Config::FILE_NAME.to_owned(), Routes::FILE_NAME.to_owned()];
    created.push("theme/templates/".to_owned());
    created.extend(
        answers
            .languages
            .iter()
            .map(|locale| format!("content/{locale}/posts/")),
    );
    for relative in created.iter().skip(2) {
        let mut path = folder.to_path_buf();
        path.push(relative);
        fs::create_dir_all(path)?;
    }
    if let Err(error) = create(folder, Routes::FILE_NAME, ROUTES_TOML)
        && error.kind() != io::ErrorKind::AlreadyExists
    {
        return Err(error);
    }
    create(folder, Config::FILE_NAME, settings)?;
    Ok(created)
}

/// Writes `text` to a new file `name` in `folder`, and never replaces one.
/// Both files sit at the site's root, outside the folders BL-RUN-9 covers.
fn create(folder: &Path, name: &str, text: &str) -> io::Result<()> {
    let mut path = folder.to_path_buf();
    path.push(name);
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()
}

/// Says what `init` made and what to do next.
fn success(output: Output, folder: &Path, answers: &Answers, created: Vec<String>) -> Exit {
    let shown = folder.display().to_string();
    match output {
        Output::Human => report::text(
            &format!(
                "Made a Khalid site in {shown} for {}.\nNext, write your first post with `khalid --site {shown} new \"Your title\"`.\n",
                answers.address
            ),
            Exit::Success,
        ),
        Output::Json => report::json(
            &InitBody {
                site: shown,
                created,
            },
            Exit::Success,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_paths_gain_their_slashes_and_strings_their_escapes() {
        assert_eq!(mount_path(" news/en "), "/news/en/");
        assert_eq!(mount_path(""), "/");
        assert_eq!(quoted("ü\"\\\n"), "\"ü\\\"\\\\\\u000A\"");
    }
}
