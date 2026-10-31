//! `khalid init`: a site folder from four answers, the website's address,
//! where the blog lives, the languages and the mode (PRD KH-INS-2).

use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use khalid_core::{AbsoluteUrl, Config, Environment, Locale, UserMessage};
use khalid_mount::{MountError, MountPath, Stack, proxy_rule};
use khalid_routes::Routes;
use serde::Serialize;

use crate::cli::{InitArgs, Output, Prompts, SiteMode};
use crate::exit::Exit;
use crate::{config_message, init_message, init_prompt, report};

/// The language a site writes in when nothing says otherwise.
const DEFAULT_LANGUAGE: &str = "en";

/// Where `khalid serve` answers when nothing says otherwise, which a proxy
/// on the same machine passes the blog to.
pub(crate) const LOCAL_SERVE: &str = "http://127.0.0.1:8080";

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
    pub(crate) stack: Option<Stack>,
    pub(crate) blog: Option<AbsoluteUrl>,
}

/// The JSON shape of `khalid init --json`: the site folder, and the files and
/// folders made in it.
#[derive(Serialize)]
struct InitBody {
    site: String,
    created: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<ProxyBody>,
}

/// The proxy rule `init` printed, as `khalid init --json` reports it.
#[derive(Serialize)]
struct ProxyBody {
    stack: String,
    mount: String,
    rule: String,
    /// The blog address in the rule when Khalid guessed it, to be changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    guessed: Option<String>,
}

/// Makes a site in `folder`, asking for what the flags in `args` leave out
/// when a person is at a terminal.
pub(crate) fn run(folder: &Path, args: &InitArgs, output: Output) -> Exit {
    let mut settings = folder.to_path_buf();
    settings.push(Config::FILE_NAME);
    if settings.exists() {
        return report::failure(output, &init_message::exists(&settings), Exit::Failure);
    }
    let answers = match gather(args) {
        Ok(answers) => answers,
        Err((message, exit)) => return report::failure(output, &message, exit),
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

/// Makes a site in `folder` when it has none, for `serve --init` on a host
/// that starts Khalid on an empty disk. The address is the one
/// `KHALID_ADDRESS` gives, and the rest are `init`'s defaults. A folder that
/// holds a site already is left as it is.
///
/// # Errors
///
/// The message to show when `KHALID_ADDRESS` is unset or not an address, or
/// when a file of the new site cannot be written.
pub(crate) fn first_start(folder: &Path, environment: &Environment) -> Result<(), UserMessage> {
    let mut settings = folder.to_path_buf();
    settings.push(Config::FILE_NAME);
    if settings.exists() {
        return Ok(());
    }
    let answers = start_address(&settings, environment)
        .and_then(|address| defaults_at(address).ok_or_else(init_message::needs_start_address))?;
    write(folder, &answers, &answers.site_toml())
        .map(drop)
        .map_err(|error| init_message::write(folder, &error))
}

/// The address `KHALID_ADDRESS` gives, read the way the settings read it:
/// a stand-in address in `site.toml` is replaced by the variable's.
fn start_address(settings: &Path, environment: &Environment) -> Result<AbsoluteUrl, UserMessage> {
    let stand_in = AbsoluteUrl::parse("https://stand-in.invalid/")
        .ok()
        .and_then(defaults_at)
        .ok_or_else(init_message::needs_start_address)?;
    let config = Config::from_toml(settings, &stand_in.site_toml(), environment)
        .map_err(|error| config_message::of(&error))?;
    Some(config.address().clone())
        .filter(|address| *address != stand_in.address)
        .ok_or_else(init_message::needs_start_address)
}

/// `init`'s answers when only the address is given.
fn defaults_at(address: AbsoluteUrl) -> Option<Answers> {
    Answers::from_flags(&InitArgs {
        folder: None,
        address: Some(address),
        mount: None,
        languages: Vec::new(),
        mode: None,
        stack: None,
        blog: None,
        prompts: Prompts::Never,
    })
}

/// Takes the answers from the flags when Khalid may not ask, because of
/// `--yes` or because there is no terminal, and asks otherwise (RULES.md BL-CLI-4).
fn gather(args: &InitArgs) -> Result<Answers, (UserMessage, Exit)> {
    let stdin = io::stdin();
    if args.prompts == Prompts::Never || !stdin.is_terminal() {
        return Answers::from_flags(args)
            .ok_or_else(|| (init_message::needs_address(), Exit::Usage));
    }
    init_prompt::ask(args, &mut stdin.lock(), &mut io::stderr().lock())
        .map_err(|_| (init_message::no_answer(), Exit::Failure))
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
            stack: args.stack,
            blog: args.blog.clone(),
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

/// The proxy rule a mounted blog needs, for `stack`.
pub(crate) struct Proxy {
    stack: Stack,
    rule: String,
    /// The blog address the rule names when Khalid had to guess it.
    guessed: Option<AbsoluteUrl>,
}

impl Answers {
    /// The proxy rule for a blog mounted under a folder of another stack,
    /// or `None` when the blog is the whole site or no stack was given.
    /// With no `--blog`, a proxy on the same machine is sent to
    /// [`LOCAL_SERVE`], and a hosted one to a guessed `blog.` address.
    pub(crate) fn proxy_rule(&self) -> Result<Option<Proxy>, UserMessage> {
        // `init` keeps the mount with a trailing slash; the rule has none.
        let folder = self.mount.trim_end_matches('/');
        if folder.is_empty() {
            return Ok(None);
        }
        let mount = MountPath::try_new(folder).map_err(|_| init_message::mount(&self.mount))?;
        let Some(stack) = self.stack else {
            return Ok(None);
        };
        let (blog, guessed) = match &self.blog {
            Some(blog) => (blog.clone(), None),
            None if on_one_machine(stack) => {
                let local = AbsoluteUrl::parse(LOCAL_SERVE);
                (local.map_err(|_| init_message::address(LOCAL_SERVE))?, None)
            }
            None => {
                let blog = blog_origin(&self.address)?;
                (blog.clone(), Some(blog))
            }
        };
        let rule = proxy_rule(stack, &mount, &blog).map_err(|error| of_mount_error(&error))?;
        Ok(Some(Proxy {
            stack,
            rule,
            guessed,
        }))
    }
}

/// Whether `stack` is a proxy that usually runs on the same machine as
/// `khalid serve`, rather than a hosted platform that needs a public address.
pub(crate) fn on_one_machine(stack: Stack) -> bool {
    matches!(
        stack,
        Stack::Nginx | Stack::Caddy | Stack::Apache | Stack::Traefik
    )
}

/// A guess at where the blog itself answers, for a hosted stack: `blog.`
/// before the website's host, such as `blog.example.com`.
fn blog_origin(address: &AbsoluteUrl) -> Result<AbsoluteUrl, UserMessage> {
    let scheme = address
        .as_str()
        .split_once("://")
        .map_or("https", |(s, _)| s);
    let host = address
        .host()
        .strip_prefix("www.")
        .unwrap_or(address.host());
    let text = format!("{scheme}://blog.{host}");
    AbsoluteUrl::parse(&text).map_err(|_| init_message::address(&text))
}

/// A stack error as the person reads it.
fn of_mount_error(error: &MountError) -> UserMessage {
    match error {
        MountError::UnknownStack { name } => init_message::stack(name),
        other => init_message::mount(&other.to_string()),
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
/// characters escaped. The same text is a YAML double-quoted string.
pub(crate) fn quoted(text: &str) -> String {
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

/// Makes a server site in `folder` with the usual answers for `address`:
/// the whole domain, in English. A folder that has a `site.toml` already
/// is kept as it is.
///
/// # Errors
///
/// Returns the error when a folder or file cannot be made.
pub(crate) fn make_site(folder: &Path, address: AbsoluteUrl) -> io::Result<()> {
    let answers = Answers {
        address,
        mount: "/".to_owned(),
        languages: vec![Locale::try_new(DEFAULT_LANGUAGE).map_err(io::Error::other)?],
        mode: SiteMode::Server,
        stack: None,
        blog: None,
    };
    match write(folder, &answers, &answers.site_toml()) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => Err(error),
        _ => Ok(()),
    }
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
    created.extend(
        answers
            .languages
            .iter()
            .map(|locale| format!("drafts/{locale}/")),
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
    let proxy = answers.proxy_rule().unwrap_or(None);
    match output {
        Output::Human => report::text(
            &format!(
                "Made a Khalid site in {shown} for {}.\n{}Next, write your first post with \
                 `khalid --site {shown} new --draft \"Your title\"`. \
                 It waits in drafts/ until `khalid --site {shown} publish <slug>` takes it live.\n",
                answers.address,
                proxy.as_ref().map_or(String::new(), proxy_text),
            ),
            Exit::Success,
        ),
        Output::Json => report::json(
            &InitBody {
                site: shown,
                created,
                proxy: proxy.map(|proxy| ProxyBody {
                    stack: proxy.stack.name().to_owned(),
                    mount: answers.mount.clone(),
                    rule: proxy.rule,
                    guessed: proxy.guessed.map(|blog| blog.as_str().to_owned()),
                }),
            },
            Exit::Success,
        ),
    }
}

/// The proxy rule as `init` prints it, saying when its blog address is a
/// guess to change.
fn proxy_text(proxy: &Proxy) -> String {
    let guess = proxy.guessed.as_ref().map_or(String::new(), |blog| {
        let blog = blog.as_str().trim_end_matches('/');
        format!(
            "Khalid does not know where the blog answers, so it guessed. \
             Change {blog} in the rule to the address where `khalid serve` runs.\n"
        )
    });
    format!(
        "The blog lives under a folder, so {} must pass it to Khalid. Add this rule:\n\n{}\n{guess}",
        proxy.stack, proxy.rule
    )
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
