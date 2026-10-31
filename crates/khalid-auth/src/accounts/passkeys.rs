//! Adding a passkey to a signed-in account, and signing in with one.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, Utc};

use super::{Accounts, SignInError, SignedIn};
use crate::{
    AccountId, CreationChallengeResponse, PasskeySite, PasskeyStore, PublicKeyCredential,
    RegisterPublicKeyCredential, Registering, RequestChallengeResponse, SessionToken, SigningIn,
};

/// How long a passkey request may take to answer.
pub const CEREMONY: TimeDelta = TimeDelta::minutes(5);

/// The most passkey requests waiting at once, so nobody can fill the
/// memory by asking for challenges and never answering.
const MAX_PENDING: usize = 10_000;

/// The requests waiting for an answer, by the challenge each sent: what
/// each was asked for, and when.
type Pending = BTreeMap<Vec<u8>, (Waiting, DateTime<Utc>)>;

/// What a waiting challenge was asked for.
#[derive(Debug, Clone)]
enum Waiting {
    Register(AccountId, Registering),
    SignIn(SigningIn),
}

/// What passkey sign-in needs: where passkeys are kept, the site they
/// belong to, and the requests waiting for an answer. Cloning shares the
/// waiting requests.
#[derive(Clone)]
pub struct Passkeys {
    store: Arc<dyn PasskeyStore>,
    site: PasskeySite,
    pending: Arc<Mutex<Pending>>,
}

impl fmt::Debug for Passkeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Passkeys")
            .field("site", &self.site.id())
            .finish_non_exhaustive()
    }
}

impl Passkeys {
    /// Passkeys kept in `store`, for `site`.
    pub fn new(store: Arc<dyn PasskeyStore>, site: PasskeySite) -> Self {
        Self {
            store,
            site,
            pending: Arc::default(),
        }
    }

    /// Keeps `waiting` under `challenge`, made at `now`.
    fn keep(
        &self,
        challenge: Vec<u8>,
        waiting: Waiting,
        now: DateTime<Utc>,
    ) -> Result<(), SignInError> {
        let mut pending = self.pending.lock().map_err(|_| SignInError::Refused)?;
        pending.retain(|_, (_, made)| now < *made + CEREMONY);
        if pending.len() >= MAX_PENDING {
            return Err(SignInError::limited(CEREMONY));
        }
        pending.insert(challenge, (waiting, now));
        Ok(())
    }

    /// Takes what waits under `challenge`, once, when it is still fresh at
    /// `now`.
    fn take(&self, challenge: &[u8], now: DateTime<Utc>) -> Option<Waiting> {
        let mut pending = self.pending.lock().ok()?;
        let (waiting, made) = pending.remove(challenge)?;
        (now < made + CEREMONY).then_some(waiting)
    }
}

impl Accounts {
    /// These accounts, able to use passkeys as `passkeys` says.
    #[must_use]
    pub fn with_passkeys(mut self, passkeys: Passkeys) -> Self {
        self.passkeys = Some(passkeys);
        self
    }

    /// Starts adding a passkey to the account `token` signs in: the
    /// options for `navigator.credentials.create`, which name the passkeys
    /// the account already has so the same one is not added twice.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] for no live session,
    /// [`SignInError::PasskeysOff`] when the site has no passkeys,
    /// [`SignInError::Limited`] when too many requests are waiting, and
    /// [`SignInError::Unavailable`] when a store fails.
    pub async fn begin_registration(
        &self,
        token: &SessionToken,
    ) -> Result<CreationChallengeResponse, SignInError> {
        let passkeys = self.passkeys.as_ref().ok_or(SignInError::PasskeysOff)?;
        let account = self.session(token).await?.account;
        let existing = passkeys.store.of(account.id()).await?;
        let exclude = existing
            .iter()
            .map(|stored| stored.passkey.credential_id().to_vec())
            .collect();
        let (options, registering) = passkeys.site.start_registration(&account, exclude)?;
        let waiting = Waiting::Register(account.id(), registering);
        let challenge = options.public_key.challenge.clone();
        passkeys.keep(challenge, waiting, self.clock.now())?;
        Ok(options)
    }

    /// Adds `credential`, the passkey the browser made for the options
    /// whose challenge is `challenge`, to the account `token` signs in,
    /// under `name`.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Passkey`] naming why the passkey was refused,
    /// [`SignInError::Refused`] for no live session, a challenge that was
    /// not asked for this account, used or too old, or a passkey already
    /// added, and [`SignInError::Unavailable`] when a store fails.
    pub async fn finish_registration(
        &self,
        token: &SessionToken,
        challenge: &[u8],
        credential: &RegisterPublicKeyCredential,
        name: &str,
    ) -> Result<(), SignInError> {
        let passkeys = self.passkeys.as_ref().ok_or(SignInError::PasskeysOff)?;
        let account = self.session(token).await?.account;
        let now = self.clock.now();
        let Some(Waiting::Register(asked, registering)) = passkeys.take(challenge, now) else {
            return Err(SignInError::Refused);
        };
        if asked != account.id() {
            return Err(SignInError::Refused);
        }
        let saved = passkeys
            .site
            .finish_registration(credential, &registering)
            .map_err(SignInError::Passkey)?;
        let added = passkeys.store.add(account.id(), &saved, name, now).await?;
        added.then_some(()).ok_or(SignInError::Refused)
    }

    /// Starts a passkey sign-in: the options for
    /// `navigator.credentials.get`. The browser offers whichever passkey
    /// the person has for this site.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::PasskeysOff`] when the site has no passkeys,
    /// and [`SignInError::Limited`] when too many requests are waiting.
    pub fn begin_sign_in(&self) -> Result<RequestChallengeResponse, SignInError> {
        let passkeys = self.passkeys.as_ref().ok_or(SignInError::PasskeysOff)?;
        let (options, signing_in) = passkeys.site.start_sign_in()?;
        let challenge = options.public_key.challenge.clone();
        passkeys.keep(challenge, Waiting::SignIn(signing_in), self.clock.now())?;
        Ok(options)
    }

    /// Signs in with `credential`, the browser's answer to the options
    /// whose challenge is `challenge`.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] for an unknown passkey, a challenge
    /// that was not asked, used or too old, or any check that fails, one
    /// answer for every reason; [`SignInError::PasskeysOff`] when the site
    /// has no passkeys; and [`SignInError::Unavailable`] when a store fails.
    pub async fn finish_sign_in(
        &self,
        challenge: &[u8],
        credential: &PublicKeyCredential,
    ) -> Result<(SessionToken, SignedIn), SignInError> {
        let passkeys = self.passkeys.as_ref().ok_or(SignInError::PasskeysOff)?;
        let now = self.clock.now();
        let Some(Waiting::SignIn(signing_in)) = passkeys.take(challenge, now) else {
            return Err(SignInError::Refused);
        };
        let stored = passkeys.store.find(credential.get_credential_id()).await?;
        let stored = stored.ok_or(SignInError::Refused)?;
        let saved = passkeys
            .site
            .finish_sign_in(credential, signing_in, &stored.passkey)
            .map_err(|_| SignInError::Refused)?;
        passkeys.store.used(&saved, now).await?;
        let account = self.store.find(stored.account).await?;
        let account = account.ok_or(SignInError::Refused)?;
        let session = self.begin(&account).await?;
        let csrf = session.csrf();
        let signed_in = SignedIn {
            account,
            csrf,
            signed_in_at: now,
        };
        Ok((session, signed_in))
    }
}
