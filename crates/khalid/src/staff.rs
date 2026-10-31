//! A site's staff accounts as `khalid serve` signs people in: accounts,
//! sessions, passkeys and sign-in links in the site's database (plan
//! section 4.6).

use std::sync::Arc;

use khalid_auth::{Accounts, Links, PasskeySite, Passkeys, PasswordHashing};
use khalid_core::{Config, SystemClock};
use khalid_db::{AccountTable, Database, LinkTable, PasskeyTable, SessionTable, SiteId};

/// The staff accounts of `site` in `db`, with passkeys for the address in
/// `config` when it has a host name. Sign-in links made by `khalid staff
/// owner` work; mailing links waits for a mail sender.
pub(crate) fn accounts(db: &Database, site: SiteId, config: &Config) -> Accounts {
    let store = Arc::new(AccountTable::new(db.clone(), site));
    let sessions = Arc::new(SessionTable::new(db.clone(), site));
    let links = Links {
        store: Arc::new(LinkTable::new(db.clone(), site)),
        sender: None,
        admin: admin_address(config),
    };
    let accounts = Accounts::new(
        store,
        sessions,
        PasswordHashing::new(),
        Arc::new(SystemClock),
    )
    .with_links(links);
    // An address such as http://127.0.0.1/ has no host name for a passkey
    // to belong to; the site then answers that it has no passkeys.
    match PasskeySite::for_address(config.address()) {
        Ok(passkey_site) => {
            let store = Arc::new(PasskeyTable::new(db.clone(), site));
            accounts.with_passkeys(Passkeys::new(store, passkey_site))
        }
        Err(_) => accounts,
    }
}

/// Where the admin is served: `admin/` under the site's address, such as
/// `https://example.com/blog/admin` (plan section 4.6).
pub(crate) fn admin_address(config: &Config) -> String {
    format!("{}/admin", config.address().as_str().trim_end_matches('/'))
}
