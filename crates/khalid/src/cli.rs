//! The command line, as `clap` parses it.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::builder::{BoolValueParser, TypedValueParser};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use khalid_api::keys::{KeyId, Role, Scope};
use khalid_core::{AbsoluteUrl, Locale, Slug};
use khalid_https::Domain;

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

/// The staff commands. Accounts are kept in `data/site.db`.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum StaffCommand {
    /// Make the owner if the site has none, and show a sign-in link once
    Owner(OwnerArgs),
}

/// What `khalid staff owner` takes.
#[derive(Debug, Clone, Args)]
pub(crate) struct OwnerArgs {
    /// The owner's email address, which they sign in with
    #[arg(long)]
    pub(crate) email: String,

    /// The owner's name, as the admin shows it
    #[arg(long)]
    pub(crate) name: String,
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
            Self::Staff {
                command: StaffCommand::Owner(_),
            } => "staff owner",
            Self::Db {
                command: DbCommand::Migrate,
            } => "db migrate",
            Self::Db {
                command: DbCommand::Rebuild,
            } => "db rebuild",
            Self::Redirects {
                command: RedirectsCommand::Import { .. },
            } => "redirects import",
            Self::Redirects {
                command: RedirectsCommand::Export,
            } => "redirects export",
            Self::Redirects {
                command: RedirectsCommand::Explain { .. },
            } => "redirects explain",
        }
    }
}
