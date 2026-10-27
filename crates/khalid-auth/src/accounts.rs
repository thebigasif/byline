//! Signing in and out: the one place passwords, sessions and the limiter
//! meet, which the Admin API routes call.

use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use khalid_core::Clock;

mod links;
mod passkeys;
mod take_back;

pub use links::Links;
pub use passkeys::{CEREMONY, Passkeys, RegistrationRequest};
pub use take_back::FRESH;

use crate::{
    Account, AccountStore, AccountStoreError, CsrfToken, Email, LinkStoreError, PasskeyError,
    PasskeyStoreError, Password, PasswordError, PasswordHashing, SendLinkError, SessionError,
    SessionStore, SessionStoreError, SessionToken, SignInLimiter,
};

/// A signed-in session, new or checked: whose it is, and its CSRF token.
#[derive(Debug, Clone)]
pub struct SignedIn {
    /// The account.
    pub account: Account,
    /// The session's CSRF token.
    pub csrf: CsrfToken,
    /// When the session began.
    pub signed_in_at: DateTime<Utc>,
}

/// The staff accounts of one site and how they sign in. Cloning shares
/// the stores and the limiter.
#[derive(Clone)]
pub struct Accounts {
    store: Arc<dyn AccountStore>,
    sessions: Arc<dyn SessionStore>,
    hashing: PasswordHashing,
    clock: Arc<dyn Clock>,
    limiter: Arc<SignInLimiter>,
    links: Option<Links>,
    passkeys: Option<Passkeys>,
}

impl fmt::Debug for Accounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Accounts")
            .field("clock", &self.clock)
            .finish_non_exhaustive()
    }
}

impl Accounts {
    /// Accounts in `store`, sessions in `sessions`, passwords hashed
    /// with `hashing`, time from `clock`.
    pub fn new(
        store: Arc<dyn AccountStore>,
        sessions: Arc<dyn SessionStore>,
        hashing: PasswordHashing,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            sessions,
            hashing,
            clock,
            limiter: Arc::new(SignInLimiter::new()),
            links: None,
            passkeys: None,
        }
    }

    /// Signs in with an email and password from address `from`, and
    /// returns the new session's token with who it belongs to.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] for a wrong email or password, the
    /// same answer in the same time for both; [`SignInError::Limited`]
    /// after too many failures; and [`SignInError::Unavailable`] when a
    /// store fails.
    pub async fn sign_in(
        &self,
        email: &Email,
        password: Password,
        from: Option<IpAddr>,
    ) -> Result<(SessionToken, SignedIn), SignInError> {
        let now = self.clock.now();
        if let Some(until) = self.limiter.blocked_until(email, from, now) {
            return Err(SignInError::limited(until - now));
        }
        let stored = self.store.find_by_email(email).await?;
        let (account, hash) = match stored {
            Some(stored) => (Some(stored.account), stored.password),
            None => (None, None),
        };
        let matches = self.hashing.verify(password, hash).await?;
        let Some(account) = account.filter(|_| matches) else {
            self.limiter.failed(email, from, now);
            return Err(SignInError::Refused);
        };
        self.limiter.succeeded(email);
        let token = self.begin(&account).await?;
        let signed_in = SignedIn {
            account,
            csrf: token.csrf(),
            signed_in_at: now,
        };
        Ok((token, signed_in))
    }

    /// Starts a session for `account`, once it has proven who it is.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Unavailable`] when the token cannot be made
    /// or stored.
    pub async fn begin(&self, account: &Account) -> Result<SessionToken, SignInError> {
        let token = SessionToken::generate()?;
        let now = self.clock.now();
        self.sessions.begin(token.hash(), account.id(), now).await?;
        Ok(token)
    }

    /// Who `token` signs in, if its session is live, and records the use.
    /// An expired session is ended on the way.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] for no live session, and
    /// [`SignInError::Unavailable`] when a store fails.
    pub async fn session(&self, token: &SessionToken) -> Result<SignedIn, SignInError> {
        let hash = token.hash();
        let now = self.clock.now();
        let Some(session) = self.sessions.find(hash).await? else {
            return Err(SignInError::Refused);
        };
        if !session.is_live(now) {
            self.sessions.end(hash).await?;
            return Err(SignInError::Refused);
        }
        let Some(account) = self.store.find(session.account).await? else {
            return Err(SignInError::Refused);
        };
        self.sessions.touch(hash, now).await?;
        Ok(SignedIn {
            account,
            csrf: token.csrf(),
            signed_in_at: session.created_at,
        })
    }

    /// Ends the session of `token`.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Unavailable`] when the store fails.
    pub async fn sign_out(&self, token: &SessionToken) -> Result<(), SignInError> {
        self.sessions.end(token.hash()).await?;
        Ok(())
    }

    /// The account store, for the routes that manage accounts.
    #[must_use]
    pub fn store(&self) -> &dyn AccountStore {
        self.store.as_ref()
    }

    /// The clock every account change is stamped with.
    #[must_use]
    pub fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }
}

/// Signing in, or checking a session, did not work.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SignInError {
    /// Not a known email with this password, or no live session. One answer
    /// for every reason, so a refusal teaches a guesser nothing.
    #[error("the sign-in was not accepted")]
    Refused,
    /// Too many failed attempts; try again in this many minutes.
    #[error("too many failed sign-ins; try again in {minutes} minutes")]
    Limited {
        /// Minutes until the next attempt is counted.
        minutes: i64,
    },
    /// This site cannot send sign-in links: it has no mail sender.
    #[error("this site cannot send sign-in links")]
    LinksOff,
    /// This site has no passkeys set up.
    #[error("this site has no passkey sign-in")]
    PasskeysOff,
    /// A passkey answer was refused, for this reason.
    #[error("the passkey was refused: {0}")]
    Passkey(#[source] PasskeyError),
    /// A store, the random source or the password check failed.
    #[error("the sign-in could not be checked")]
    Unavailable {
        /// What failed.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl SignInError {
    /// Too many attempts, for `wait` more: rounded up to whole minutes.
    fn limited(wait: TimeDelta) -> Self {
        Self::Limited {
            minutes: wait.num_minutes().saturating_add(1),
        }
    }
}

macro_rules! unavailable_from {
    ($($error:ty),*) => {$(
        impl From<$error> for SignInError {
            fn from(error: $error) -> Self {
                Self::Unavailable { source: Box::new(error) }
            }
        }
    )*};
}

unavailable_from!(
    AccountStoreError,
    SessionStoreError,
    PasswordError,
    SessionError,
    LinkStoreError,
    SendLinkError,
    PasskeyError,
    PasskeyStoreError
);
