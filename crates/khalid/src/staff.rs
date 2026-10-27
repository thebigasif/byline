//! A site's staff accounts as `khalid serve` signs people in: accounts,
//! sessions, passkeys and sign-in links in the site's database (plan
//! section 4.6).

use std::sync::Arc;

use khalid_auth::{Accounts, Links, Passkeys, PasswordHashing, RelyingParty};
use khalid_core::{Config, SystemClock};
use khalid_db::{AccountTable, Database, LinkTable, PasskeyTable, SessionTable, SiteId};

/// The staff accounts of `site` in `db`, with passkeys for the address in
/// `config`. Sign-in links made by `khalid staff owner` work; mailing links
/// waits for a mail sender.
pub(crate) fn accounts(db: &Database, site: SiteId, config: &Config) -> Accounts {
    let store = Arc::new(AccountTable::new(db.clone(), site));
    let sessions = Arc::new(SessionTable::new(db.clone(), site));
    let passkeys = Passkeys::new(
        Arc::new(PasskeyTable::new(db.clone(), site)),
        RelyingParty::for_address(config.address()),
    );
    let links = Links {
        store: Arc::new(LinkTable::new(db.clone(), site)),
        sender: None,
        admin: admin_address(config),
    };
    Accounts::new(
        store,
        sessions,
        PasswordHashing::new(),
        Arc::new(SystemClock),
    )
    .with_passkeys(passkeys)
    .with_links(links)
}

/// Where the admin is served: `admin/` under the site's address, such as
/// `https://example.com/blog/admin` (plan section 4.6).
pub(crate) fn admin_address(config: &Config) -> String {
    format!("{}/admin", config.address().as_str().trim_end_matches('/'))
}
