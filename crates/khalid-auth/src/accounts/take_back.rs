//! Taking an account back from someone who has its cookie: signing out
//! everywhere, and changing the password.

use chrono::TimeDelta;

use super::{Accounts, SignInError};
use crate::{Password, SessionToken};

/// How long after signing in an account with no password may set one
/// without typing an old one: long enough to follow a magic link and
/// choose a password, too short for a stolen cookie found later.
pub const FRESH: TimeDelta = TimeDelta::minutes(15);

impl Accounts {
    /// Ends every session of the account `token` signs in, this one too,
    /// for a lost laptop or a stolen cookie. Returns how many ended.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] when `token` is not live, and
    /// [`SignInError::Unavailable`] when a store fails.
    pub async fn sign_out_everywhere(&self, token: &SessionToken) -> Result<u64, SignInError> {
        let signed_in = self.session(token).await?;
        Ok(self.sessions.end_all(signed_in.account.id(), None).await?)
    }

    /// Changes the password of the account `token` signs in, and ends
    /// every other session. An account with a password proves it is them
    /// with `current`; one without may set its first password only within
    /// [`FRESH`] of signing in, so an old stolen cookie cannot.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] for a dead session, a wrong or
    /// missing current password or a session too old to set a first one,
    /// and [`SignInError::Unavailable`] when a store fails.
    pub async fn change_password(
        &self,
        token: &SessionToken,
        current: Option<Password>,
        new: Password,
    ) -> Result<(), SignInError> {
        let signed_in = self.session(token).await?;
        let account = signed_in.account;
        let stored = self.store.find_by_email(account.email()).await?;
        let old = stored.and_then(|stored| stored.password);
        let proven = match (old, current) {
            (Some(old), Some(current)) => self.hashing.verify(current, Some(old)).await?,
            (Some(_), None) => false,
            (None, _) => self.clock.now() < signed_in.signed_in_at + FRESH,
        };
        if !proven {
            return Err(SignInError::Refused);
        }
        let hash = self.hashing.hash(new).await?;
        self.store.set_password(account.id(), &hash).await?;
        self.sessions
            .end_all(account.id(), Some(token.hash()))
            .await?;
        Ok(())
    }
}
