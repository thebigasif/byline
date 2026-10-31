//! The command line, as `clap` parses it.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::builder::{BoolValueParser, TypedValueParser};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use khalid_core::{AbsoluteUrl, Locale, Slug};
use khalid_https::Domain;
use khalid_webhooks::Event;

pub(crate) use crate::cli_access::{KeyKindArg, KeysCommand, NewKeyArgs, OwnerArgs, StaffCommand};

/// The command line.
#[derive(Debug, Parser)]
#[command(
    name = "khalid",
    version,
    about = "An open-source blog engine with SEO defaults built in, by SerpDad.",
    long_about = None
)]
pub(crate) struct Cli {
    /// The command to run. With none, Khalid prints this help.
    #[command(subcommand)]
    pub(crate) command: Option<Command>,

    /// Print machine-readable JSON instead of sentences.
    #[arg(
        long = "json",
        global = true,
        action = ArgAction::SetTrue,
        value_parser = BoolValueParser::new().map(|json| if json { Output::Json } else { Output::Human }),
    )]
    pub(crate) output: Output,

    /// The site folder, holding site.toml. Its settings are checked first.
    #[arg(long, global = true, value_name = "FOLDER")]
    pub(crate) site: Option<PathBuf>,

    /// Test builds only: answer host names from pairs such as
    /// `bakery.test=127.0.0.1`, with loopback allowed.
    #[cfg(feature = "test-util")]
    #[arg(long, global = true, hide = true, value_name = "HOST=ADDRESS")]
    pub(crate) test_hosts: Option<String>,

    /// Test builds only: send Bing Webmaster Tools calls to this endpoint.
    #[cfg(feature = "test-util")]
    #[arg(long, global = true, hide = true, value_name = "URL")]
    pub(crate) test_bing: Option<AbsoluteUrl>,

    /// Test builds only: where `khalid update` reads releases from.
    #[cfg(feature = "test-util")]
    #[arg(long, global = true, hide = true, value_name = "URL")]
    pub(crate) test_releases: Option<String>,
}

/// How a command prints its results and messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Output {
    /// Sentences for a person.
    Human,
    /// JSON for scripts and agents, with the same fields the API uses.
    Json,
}

/// Every command in `docs/plan.md` section 4.1.
#[derive(Debug, Subcommand)]
#[expect(
    dead_code,
    reason = "the arguments are read once each command is built, in later phases"
)]
pub(crate) enum Command {
    /// Set up a site: address, mount, languages, mode
    Init(InitArgs),
    /// Run the blog and the admin, showing each saved edit at once
    Serve(ServeArgs),
    /// Create a post file with correct front matter
    New {
        /// The post's title
        title: String,
        /// The language the post is written in [default: the site's default language]
        #[arg(long, value_name = "CODE")]
        language: Option<Locale>,
    },
    /// Render every page of the site as HTML into a folder, or nothing if a dated post is blocked.
    /// A headless site gets its feeds, sitemaps and redirects and no HTML.
    /// A static site gets a note in place of each form, and a list of the
    /// features that need a server, which are off
    Build {
        /// The folder to write the pages to [default: public in the site folder]
        #[arg(long, value_name = "FOLDER")]
        out: Option<PathBuf>,
    },
    /// Check one post or the whole site against the SEO rules, and score
    /// each post out of 100
    Lint {
        /// One post to check. Leave it out to check the whole site.
        path: Option<PathBuf>,
        /// Make the safe repairs in the files: heading order, image
        /// descriptions from captions, cleaner addresses and https for links
        /// to this site, then check again.
        #[arg(long)]
        fix: bool,
    },
    /// Lint a draft, then publish it and tell the site's webhooks and
    /// search engines. Refuses when a rule blocks it
    Publish {
        /// The slug of the draft to publish
        slug: Slug,
        /// The language the draft is written in [default: the site's default language]
        #[arg(long, value_name = "CODE")]
        language: Option<Locale>,
    },
    /// Pull the main site's look again, or check a template override
    Theme {
        /// What to do with the theme
        #[command(subcommand)]
        command: ThemeCommand,
    },
    /// Bring in a blog from another platform, keeping every slug and
    /// storing a redirect for each address that moves
    Import(ImportArgs),
    /// Write the site as plain HTML to public/, for static mode
    Export,
    /// Speak the Model Context Protocol on stdin and stdout, for agents.
    /// Acts with the key in the `KHALID_KEY` environment variable
    Mcp,
    /// Print the guide for AI agents: the files, the commands and the MCP tools
    Agents,
    /// Check the site and its public address, and say how to fix each problem
    Doctor {
        /// Check only the files, without fetching the blog's public address
        /// or the host's robots.txt
        #[arg(
            long = "offline",
            action = ArgAction::SetTrue,
            value_parser = BoolValueParser::new().map(|offline| if offline { Network::Offline } else { Network::Fetch }),
        )]
        network: Network,
    },
    /// Write one archive of the site folder, with a safe copy of its
    /// database, leaving out public/ and data/media/, which Khalid makes again
    Backup {
        /// The archive to write, a new .tar.gz file. Without it, Khalid writes
        /// `<folder>-backup-<date>-<time>.tar.gz` in the current folder
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
    /// Build the site, crawl it the way Ahrefs and Semrush do, and score its
    /// health out of 100. Keeps the issues, the internal linking report and
    /// the score's trend in data/audit/, and exits with 1 on any error.
    /// Broken links on the blog go to the owner as an alert, once
    Audit {
        /// Ask this running server for the site's pages instead of
        /// building them, such as `http://127.0.0.1:8080` for `khalid serve`
        /// on this machine. Addresses stay the site's own
        #[arg(long, value_name = "ADDRESS")]
        server: Option<AbsoluteUrl>,
        /// Leave links to other sites unchecked, so the audit fetches
        /// nothing from the internet. Alerts still go to the site's webhooks
        #[arg(
            long = "offline",
            action = ArgAction::SetTrue,
            value_parser = BoolValueParser::new().map(|offline| if offline { Network::Offline } else { Network::Fetch }),
        )]
        network: Network,
    },
    /// Update the binary, or go back to the previous version
    Update {
        /// Go back to the version before the last update
        #[arg(
            long = "rollback",
            action = ArgAction::SetTrue,
            value_parser = BoolValueParser::new().map(|back| if back { UpdateTarget::Previous } else { UpdateTarget::Latest }),
        )]
        target: UpdateTarget,
    },
    /// Fetch the content repository and rebuild, in git mode
    Pull,
    /// Create and revoke API keys for integrations, staff and web pages
    Keys {
        /// What to do with the keys
        #[command(subcommand)]
        command: KeysCommand,
    },
    /// Make the site's owner, and sign the owner in with a link shown once
    Staff {
        /// What to do with staff accounts
        #[command(subcommand)]
        command: StaffCommand,
    },
    /// Bring the site's database up to date, or rebuild its index from the files
    Db {
        /// What to do with the database
        #[command(subcommand)]
        command: DbCommand,
    },
    /// Bring in, take out and explain the site's stored redirects
    Redirects {
        /// What to do with the redirects
        #[command(subcommand)]
        command: RedirectsCommand,
    },
    /// Add, list and switch off the webhooks that tell other tools what happened
    Webhooks {
        /// What to do with the webhooks
        #[command(subcommand)]
        command: WebhooksCommand,
    },
    /// Hand Bing Webmaster Tools the sitemap, or read how Bing crawls the
    /// site. Uses the key in the `KHALID_BING_API_KEY` environment variable
    Bing {
        /// What to ask Bing
        #[command(subcommand)]
        command: BingCommand,
    },
}

/// The Bing Webmaster Tools commands, for the site at the address in
/// `site.toml`, as verified in Bing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub(crate) enum BingCommand {
    /// Hand Bing the site's sitemap, so it reads every page from it
    Sitemap,
    /// Show Bing's last crawl day, the sitemaps it holds and the addresses
    /// it had trouble with
    Report,
}

/// The webhook commands. Webhooks live in `data/site.db`; a running
/// `khalid serve` sees a change at once.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum WebhooksCommand {
    /// Send one event to one address, signed with a new secret shown once
    Add {
        /// The event to send, such as post.published or comment.added
        #[arg(long, value_name = "EVENT", value_parser = |text: &str| text.parse::<Event>())]
        event: Event,
        /// Where to send it: a full http or https address
        #[arg(long, value_name = "URL")]
        url: String,
    },
    /// Show every webhook that is on, without its secret
    List,
    /// Switch a webhook off. Its past deliveries stay in the log
    Remove {
        /// The webhook's identity, as list shows it
        #[arg(value_name = "ID")]
        id: i64,
    },
}

/// The redirect commands. Stored redirects live in `data/site.db`;
/// `khalid serve` reads them when it starts.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum RedirectsCommand {
    /// Store every redirect in a CSV file with the columns from,to,status,match, or none
    Import {
        /// The CSV file, such as the import/wordpress-redirects.csv an import wrote
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Print every stored redirect as CSV, to keep or open in a spreadsheet
    Export,
    /// Say which stored redirect an address matches, and where a reader ends up
    Explain {
        /// The address from the site root, such as /blog/old-links/
        #[arg(value_name = "PATH")]
        path: String,
    },
}

/// What `khalid serve` takes: plain HTTP on an address, or HTTPS for a
/// domain with its own certificate.
#[derive(Debug, Args)]
pub(crate) struct ServeArgs {
    /// The address and port to answer on over plain HTTP
    #[arg(long, value_name = "ADDRESS", default_value = "127.0.0.1:8080")]
    pub(crate) listen: SocketAddr,
    /// Answer over HTTPS for this domain on ports 443 and 80, with a free
    /// certificate from Let's Encrypt that Khalid gets and renews
    #[arg(long, value_name = "DOMAIN", conflicts_with = "listen")]
    pub(crate) domain: Option<Domain>,
    /// Agree to Let's Encrypt's subscriber agreement without being asked,
    /// such as in a service or a container
    #[arg(long, requires = "domain")]
    pub(crate) accept_terms: bool,
    /// The address for HTTPS instead of port 443 on every address
    #[arg(long, value_name = "ADDRESS", requires = "domain", hide = true)]
    pub(crate) https_listen: Option<SocketAddr>,
    /// The address for plain HTTP instead of port 80 on every address
    #[arg(long, value_name = "ADDRESS", requires = "domain", hide = true)]
    pub(crate) http_listen: Option<SocketAddr>,
    /// Another ACME directory than Let's Encrypt's, such as its staging one
    #[arg(long, value_name = "URL", requires = "domain", hide = true)]
    pub(crate) acme_directory: Option<String>,
    /// Test builds only: run the alert checks this many seconds apart
    /// instead of hourly.
    #[cfg(feature = "test-util")]
    #[arg(long, hide = true, value_name = "SECONDS")]
    pub(crate) alert_every: Option<u64>,
}

/// What `khalid init` takes. Each flag answers one of the four questions, and
/// a question with a flag is not asked.
#[derive(Debug, Args)]
pub(crate) struct InitArgs {
    /// The folder to make the site in, made when missing [default: --site, or here]
    #[arg(value_name = "FOLDER")]
    pub(crate) folder: Option<PathBuf>,

    /// The website's address, such as `https://www.example.com`
    #[arg(long, value_name = "URL")]
    pub(crate) address: Option<AbsoluteUrl>,

    /// The folder of that website the blog lives under, such as /blog/ [default: /]
    #[arg(long, value_name = "PATH")]
    pub(crate) mount: Option<String>,

    /// The languages you write in, the first being the default, such as en,de [default: en]
    #[arg(long, value_name = "CODES", value_delimiter = ',')]
    pub(crate) languages: Vec<Locale>,

    /// Run a server, or write plain files to upload [default: server]
    #[arg(long, value_enum)]
    pub(crate) mode: Option<SiteMode>,

    /// The stack the main site runs on, for the proxy rule a mounted blog needs
    #[arg(long, value_name = "NAME")]
    pub(crate) stack: Option<khalid_mount::Stack>,

    /// Where the blog itself answers, when it is not the website's address [default: blog.host]
    #[arg(long, value_name = "URL")]
    pub(crate) blog: Option<AbsoluteUrl>,

    /// Ask nothing: take the flags, and the defaults for the rest
    #[arg(
        long = "yes",
        short = 'y',
        action = ArgAction::SetTrue,
        value_parser = BoolValueParser::new().map(|yes| if yes { Prompts::Never } else { Prompts::Ask }),
    )]
    pub(crate) prompts: Prompts,
}

/// Whether a command may ask questions (RULES.md BL-CLI-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Prompts {
    /// Ask in the terminal when there is one.
    Ask,
    /// Never ask, because `--yes` was given.
    Never,
}

/// How the site runs, as `--mode` spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum SiteMode {
    /// Khalid runs the blog as a server
    Server,
    /// Khalid writes plain files to upload anywhere
    Static,
}

impl SiteMode {
    /// The word `site.toml` uses.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Static => "static",
        }
    }
}

/// The database commands. The database is `data/site.db` in the site folder.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum DbCommand {
    /// Make the database if it is missing and apply every new migration
    Migrate,
    /// Empty the index and fill it again from the content files, keeping visitor data
    Rebuild,
    /// Write every table of the site as one CSV and one JSON file each, for
    /// spreadsheets and Looker Studio. Sign-in material and secrets stay out
    Export {
        /// The folder to write the files to [default: data/export in the site folder]
        #[arg(long, value_name = "FOLDER")]
        out: Option<PathBuf>,
    },
}

/// The theme commands.
#[derive(Debug, Subcommand)]
pub(crate) enum ThemeCommand {
    /// Read the main site's colours, fonts, logo and icon into
    /// theme/tokens.toml, and draw a preview in data/theme-preview/
    Sync {
        /// The main site's address, such as `https://www.example.com`
        url: AbsoluteUrl,
        /// Take the colours and fonts from this CSS file instead of the
        /// site, such as its variables copied from a browser, for a site
        /// that draws its pages with JavaScript. Nothing is fetched.
        #[arg(long, value_name = "FILE")]
        from_css: Option<PathBuf>,
    },
    /// Compare theme/tokens.toml with the main site and check its contrast
    Check {
        /// The main site's address [default: the address in site.toml]
        url: Option<AbsoluteUrl>,
    },
}

/// What `khalid import` takes.
#[derive(Debug, Args)]
pub(crate) struct ImportArgs {
    /// Where the blog comes from
    pub(crate) source: ImportSource,

    #[arg(
        value_name = "EXPORT",
        help = "The export: a WordPress or Ghost file, a WordPress site's address for its published posts, the unpacked Medium or Substack folder, or a folder of markdown"
    )]
    pub(crate) export: PathBuf,

    /// Say what would come in and what would stay behind, and write nothing
    #[arg(long = "dry-run", action = ArgAction::SetTrue, value_parser = BoolValueParser::new().map(|dry| if dry { Run::Dry } else { Run::Write }))]
    pub(crate) run: Run,

    #[arg(
        long,
        value_name = "URL",
        help = "Where the blog lived, for Ghost, Substack and markdown, such as https://rye.substack.com [default: the address in site.toml]"
    )]
    pub(crate) old_site: Option<AbsoluteUrl>,

    #[arg(
        long,
        value_name = "FILE",
        help = "The Redirection plugin's JSON export, whose rules come in with a WordPress export"
    )]
    pub(crate) redirection: Option<PathBuf>,
}

/// Whether `khalid import` writes, or only says what it would write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Run {
    Dry,
    Write,
}

/// The platforms `khalid import` reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ImportSource {
    #[value(help = "A WordPress export file")]
    Wordpress,
    /// A Ghost export file
    Ghost,
    /// A Medium export archive
    Medium,
    /// A Substack export archive
    Substack,
    /// A folder of markdown files
    Markdown,
}

/// Whether `khalid doctor` fetches the blog's public address and the host's
/// `robots.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Network {
    /// Fetch it, to check the blog answers there.
    Fetch,
    /// Stay offline, because `--offline` was given.
    Offline,
}

/// Which version `khalid update` installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpdateTarget {
    /// The newest release.
    Latest,
    /// The version before the last update.
    Previous,
}

impl Command {
    /// The command as a person types it, such as `theme sync`.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Init(_) => "init",
            Self::Serve(_) => "serve",
            Self::New { .. } => "new",
            Self::Build { .. } => "build",
            Self::Lint { .. } => "lint",
            Self::Publish { .. } => "publish",
            Self::Theme {
                command: ThemeCommand::Sync { .. },
            } => "theme sync",
            Self::Theme {
                command: ThemeCommand::Check { .. },
            } => "theme check",
            Self::Import(_) => "import",
            Self::Export => "export",
            Self::Mcp => "mcp",
            Self::Agents => "agents",
            Self::Doctor { .. } => "doctor",
            Self::Backup { .. } => "backup",
            Self::Audit { .. } => "audit",
            Self::Update { .. } => "update",
            Self::Pull => "pull",
            Self::Keys {
                command: KeysCommand::Create(_),
            } => "keys create",
            Self::Keys {
                command: KeysCommand::Revoke { .. },
            } => "keys revoke",
            Self::Staff {
                command: StaffCommand::Owner(_),
            } => "staff owner",
            Self::Db {
                command: DbCommand::Migrate,
            } => "db migrate",
            Self::Db {
                command: DbCommand::Rebuild,
            } => "db rebuild",
            Self::Db {
                command: DbCommand::Export { .. },
            } => "db export",
            Self::Redirects {
                command: RedirectsCommand::Import { .. },
            } => "redirects import",
            Self::Redirects {
                command: RedirectsCommand::Export,
            } => "redirects export",
            Self::Redirects {
                command: RedirectsCommand::Explain { .. },
            } => "redirects explain",
            Self::Webhooks {
                command: WebhooksCommand::Add { .. },
            } => "webhooks add",
            Self::Webhooks {
                command: WebhooksCommand::List,
            } => "webhooks list",
            Self::Webhooks {
                command: WebhooksCommand::Remove { .. },
            } => "webhooks remove",
            Self::Bing {
                command: BingCommand::Sitemap,
            } => "bing sitemap",
            Self::Bing {
                command: BingCommand::Report,
            } => "bing report",
        }
    }
}
