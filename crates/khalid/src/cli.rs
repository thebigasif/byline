//! The command line, as `clap` parses it.

use std::path::PathBuf;

use clap::builder::{BoolValueParser, TypedValueParser};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use khalid_core::Slug;

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
    Init,
    /// Run the blog and the admin
    Serve,
    /// Create a post file with correct front matter
    New {
        /// The post's title
        title: String,
    },
    /// Check one post or the whole site against the SEO rules
    Lint {
        /// One post to check. Leave it out to check the whole site.
        path: Option<PathBuf>,
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
    /// Create and revoke API keys for integrations
    Keys,
}

/// The theme commands.
#[derive(Debug, Subcommand)]
#[expect(
    dead_code,
    reason = "the arguments are read once each command is built, in later phases"
)]
pub(crate) enum ThemeCommand {
    /// Pull colours, fonts and logo from the main site again and show what changed
    Sync {
        /// The main site's address
        url: String,
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
            Self::Init => "init",
            Self::Serve => "serve",
            Self::New { .. } => "new",
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
            Self::Keys => "keys",
        }
    }
}
