//! A site's staff accounts as `khalid serve` signs people in: accounts,
//! sessions and passkeys in the site's database (plan section 4.6).

use std::sync::Arc;

use khalid_auth::{Accounts, Passkeys, PasswordHashing, RelyingParty};
use khalid_core::{Config, SystemClock};
use khalid_db::{AccountTable, Database, PasskeyTable, SessionTable, SiteId};

/// The staff accounts of `site` in `db`, with passkeys for the address in
/// `config`. Sign-in links wait for a mail sender, so they are off.
pub(crate) fn accounts(db: &Database, site: SiteId, config: &Config) -> Accounts {
    let store = Arc::new(AccountTable::new(db.clone(), site));
    let sessions = Arc::new(SessionTable::new(db.clone(), site));
    let passkeys = Passkeys::new(
        Arc::new(PasskeyTable::new(db.clone(), site)),
        RelyingParty::for_address(config.address()),
    );
    Accounts::new(
        store,
        sessions,
        PasswordHashing::new(),
        Arc::new(SystemClock),
    )
    .with_passkeys(passkeys)
}
