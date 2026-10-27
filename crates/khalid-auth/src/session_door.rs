//! The Admin API's door for staff sessions: a cookie with its CSRF token
//! becomes the [`Principal`] a staff token with the same role would be.

use khalid_api::content_store::BoxFuture;
use khalid_api::keys::{Access, KeyId, KeyStoreError, Principal};
use khalid_api::session_check::SessionCheck;

use crate::{Account, Accounts, SessionToken, SignInError};

impl SessionCheck for Accounts {
    fn check<'a>(
        &'a self,
        cookies: &'a str,
        csrf: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Option<Principal>, KeyStoreError>> {
        Box::pin(async move {
            let Some(token) = SessionToken::from_cookie_header(cookies) else {
                return Ok(None);
            };
            if !csrf.is_some_and(|csrf| token.csrf().matches(csrf)) {
                return Ok(None);
            }
            match self.session(&token).await {
                Ok(signed_in) => Ok(Some(principal(&signed_in.account)?)),
                Err(error @ SignInError::Unavailable { .. }) => Err(KeyStoreError::new(error)),
                Err(_) => Ok(None),
            }
        })
    }
}

/// The principal of a signed-in account: its role, and its row as 16 hex
/// digits, the shape of a key's identity, so a post edited by a person
/// names them the way one edited by a key names the key.
fn principal(account: &Account) -> Result<Principal, KeyStoreError> {
    let id = KeyId::try_new(&format!("{:016x}", account.id().as_row()))
        .map_err(|_| KeyStoreError::new(SessionDoorError))?;
    Ok(Principal::new(id, Access::Staff(account.role())))
}

/// An account row that cannot be written as a key identity.
#[derive(Debug, thiserror::Error)]
#[error("the account has no identity a key layer can carry")]
struct SessionDoorError;
