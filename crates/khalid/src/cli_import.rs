//! What `khalid import` takes on the command line.

use std::path::PathBuf;

use clap::builder::{BoolValueParser, TypedValueParser};
use clap::{ArgAction, Args, ValueEnum};
use khalid_core::AbsoluteUrl;

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
