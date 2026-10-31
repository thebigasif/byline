//! Each command's name as a person types it, for messages. Kept apart from
//! `cli.rs` so that file stays under 600 lines (BL-SIZE-1).

use crate::cli::{
    BingCommand, Command, DbCommand, KeysCommand, RedirectsCommand, SearchConsoleCommand,
    StaffCommand, ThemeCommand, WebhooksCommand,
};

impl Command {
    /// The command as a person types it, such as `theme sync`.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Init(_) => "init",
            Self::Serve(_) => "serve",
            Self::New(_) => "new",
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
            Self::Newsletter { command } => command.name(),
            Self::Bing {
                command: BingCommand::Sitemap,
            } => "bing sitemap",
            Self::Bing {
                command: BingCommand::Report,
            } => "bing report",
            Self::SearchConsole { command } => command.name(),
        }
    }
}

impl SearchConsoleCommand {
    /// The command as a person types it, such as `search-console check`.
    fn name(&self) -> &'static str {
        match self {
            Self::Check => "search-console check",
            Self::Report => "search-console report",
            Self::Post { .. } => "search-console post",
            Self::Index => "search-console index",
            Self::Email => "search-console email",
        }
    }
}
