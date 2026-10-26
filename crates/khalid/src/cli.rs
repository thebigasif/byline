//! The command line, as `clap` parses it.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::builder::{BoolValueParser, TypedValueParser};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use khalid_api::keys::{KeyId, Role, Scope};
use khalid_core::{AbsoluteUrl, Locale, Slug};

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
    Serve {
        /// The address and port to answer on
        #[arg(long, value_name = "ADDRESS", default_value = "127.0.0.1:8080")]
        listen: SocketAddr,
    },
    /// Create a post file with correct front matter
    New {
        /// The post's title
        title: String,
        /// The language the post is written in [default: the site's default language]
        #[arg(long, value_name = "CODE")]
        language: Option<Locale>,
    },
    /// Render every page of the site as HTML into a folder
    Build {
        /// The folder to write the pages to [default: public in the site folder]
        #[arg(long, value_name = "FOLDER")]
        out: Option<PathBuf>,
    },
    /// Check one post or the whole site against the SEO rules
    Lint {
        /// One post to check. Leave it out to check the whole site.
        path: Option<PathBuf>,
        /// Make the safe repairs in the files: heading order, image
        /// descriptions from captions and cleaner addresses, then check again.
        #[arg(long)]
        fix: bool,
    },
    /// Lint a draft, then publish it. Refuses when a rule blocks it
    Publish {
        /// The slug of the draft to publish
        slug: Slug,
    },
    /// Pull the main site's look again, or check a template override
    Theme {
        /// What to do with the theme
        #[command(subcommand)]
        command: ThemeCommand,
    },
    /// Bring in a blog from another platform
    Import {
        /// Where the blog comes from
        source: ImportSource,
    },
    /// Write the site as plain HTML to public/, for static mode
    Export,
    /// Speak the Model Context Protocol on stdin and stdout, for agents
    Mcp,
    /// Check the install: DNS, HTTPS, proxy rule, sitemap, robots and more
    Doctor,
    /// Write one archive of the site folder
    Backup,
    /// Crawl the site the way SEO tools do and score it
    Audit,
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
    /// Bring the site's database up to date, or rebuild its index from the files
    Db {
        /// What to do with the database
        #[command(subcommand)]
        command: DbCommand,
    },
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub(crate) enum DbCommand {
    /// Make the database if it is missing and apply every new migration
    Migrate,
    /// Empty the index and fill it again from the content files, keeping visitor data
    Rebuild,
}

/// The key commands. Keys are stored as hashes in `data/site.db`.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum KeysCommand {
    /// Make a key and show it once. Khalid keeps only a hash of it
    Create(NewKeyArgs),
    /// Stop a key from working, at once and for good
    Revoke {
        /// The key's identity: the 16 characters after the first `_`
        #[arg(value_name = "ID", value_parser = |text: &str| KeyId::try_new(text))]
        id: KeyId,
    },
}

/// What `khalid keys create` takes.
#[derive(Debug, Clone, Args)]
pub(crate) struct NewKeyArgs {
    /// Who or what uses the key, such as a publishing tool or a name
    #[arg(long)]
    pub(crate) name: String,

    /// What the key is for
    #[arg(long, value_enum, default_value_t = KeyKindArg::Integration)]
    pub(crate) kind: KeyKindArg,

    /// What an integration key may do, such as posts:write. Repeat for more
    #[arg(long = "scope", value_name = "SCOPE")]
    pub(crate) scopes: Vec<Scope>,

    /// A staff token's role: owner, admin, editor, writer or contributor
    #[arg(long)]
    pub(crate) role: Option<Role>,
}

/// The kinds of key, as `--kind` spells them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum KeyKindArg {
    /// Reads published posts through the Content API. Safe in a web page
    Content,
    /// Lets a tool, such as a publishing service, use the Admin API, with scopes
    Integration,
    /// Lets one staff member use the Admin API, with a role
    Staff,
}

/// The theme commands.
#[derive(Debug, Subcommand)]
pub(crate) enum ThemeCommand {
    /// Read the main site's colours and fonts into theme/tokens.toml
    Sync {
        /// The main site's address, such as `https://www.example.com`
        url: AbsoluteUrl,
    },
    /// Check a template override before it goes live
    Check,
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
            Self::Serve { .. } => "serve",
            Self::New { .. } => "new",
            Self::Build { .. } => "build",
            Self::Lint { .. } => "lint",
            Self::Publish { .. } => "publish",
            Self::Theme {
                command: ThemeCommand::Sync { .. },
            } => "theme sync",
            Self::Theme {
                command: ThemeCommand::Check,
            } => "theme check",
            Self::Import { .. } => "import",
            Self::Export => "export",
            Self::Mcp => "mcp",
            Self::Doctor => "doctor",
            Self::Backup => "backup",
            Self::Audit => "audit",
            Self::Update { .. } => "update",
            Self::Pull => "pull",
            Self::Keys {
                command: KeysCommand::Create(_),
            } => "keys create",
            Self::Keys {
                command: KeysCommand::Revoke { .. },
            } => "keys revoke",
            Self::Db {
                command: DbCommand::Migrate,
            } => "db migrate",
            Self::Db {
                command: DbCommand::Rebuild,
            } => "db rebuild",
        }
    }
}
