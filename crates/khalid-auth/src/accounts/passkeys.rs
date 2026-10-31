//! Adding a passkey to a signed-in account, and signing in with one.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, Utc};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use super::{Accounts, SignInError, SignedIn};
use crate::{
    AccountId, Challenge, PasskeyStore, PublicKey, Registration, RelyingParty, SavedPasskey,
    SessionToken, verify_assertion, verify_registration,
};

/// How long a passkey request may take to answer.
pub const CEREMONY: TimeDelta = TimeDelta::minutes(5);

/// The most passkey requests waiting at once, so nobody can fill the
/// memory by asking for challenges and never answering.
const MAX_PENDING: usize = 10_000;

/// The challenges waiting for an answer: what each was asked for, and when.
type Waiting = BTreeMap<Challenge, (Purpose, DateTime<Utc>)>;

/// What a waiting challenge was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Register(AccountId),
    SignIn,
}

/// What passkey sign-in needs: where passkeys are kept, the site they
/// belong to, and the challenges waiting for an answer. Cloning shares the
/// waiting challenges.
#[derive(Clone)]
pub struct Passkeys {
    store: Arc<dyn PasskeyStore>,
    party: RelyingParty,
    pending: Arc<Mutex<Waiting>>,
}

impl fmt::Debug for Passkeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Passkeys")
            .field("party", &self.party)
            .finish_non_exhaustive()
    }
}

impl Passkeys {
    /// Passkeys kept in `store`, for the site `party` names.
    pub fn new(store: Arc<dyn PasskeyStore>, party: RelyingParty) -> Self {
        Self {
            store,
            party,
            pending: Arc::default(),
        }
    }

    /// A new challenge waiting for `purpose`, made at `now`.
    fn ask(&self, purpose: Purpose, now: DateTime<Utc>) -> Result<Challenge, SignInError> {
        let challenge = Challenge::generate().map_err(SignInError::from)?;
        let mut pending = self.pending.lock().map_err(|_| SignInError::Refused)?;
        pending.retain(|_, (_, made)| now < *made + CEREMONY);
        if pending.len() >= MAX_PENDING {
            return Err(SignInError::limited(CEREMONY));
        }
        pending.insert(challenge.clone(), (purpose, now));
        Ok(challenge)
    }

    /// Takes the challenge `client_data` answers, once, when it was asked
    /// for `purpose` and is still fresh at `now`.
    fn take(&self, client_data: &[u8], purpose: Purpose, now: DateTime<Utc>) -> Option<Challenge> {
        let challenge = Challenge::answered_in(client_data)?;
        let mut pending = self.pending.lock().ok()?;
        let (asked, made) = pending.remove(&challenge)?;
        (asked == purpose && now < made + CEREMONY).then_some(challenge)
    }
}

/// What the admin passes to `navigator.credentials.create` to add a
/// passkey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationRequest {
    /// The challenge to sign.
    pub challenge: Challenge,
    /// The site's host name.
    pub rp_id: String,
    /// The account's handle for authenticators: its number, in 8 bytes.
    pub user_id: [u8; 8],
    /// The passkeys the account already has, so the same one is not added
    /// twice.
    pub exclude: Vec<Vec<u8>>,
}

impl Accounts {
    /// These accounts, able to use passkeys as `passkeys` says.
    #[must_use]
    pub fn with_passkeys(mut self, passkeys: Passkeys) -> Self {
        self.passkeys = Some(passkeys);
        self
    }

    /// Starts adding a passkey to the account `token` signs in.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] for no live session,
    /// [`SignInError::PasskeysOff`] when the site has no passkeys, and
    /// [`SignInError::Unavailable`] when a store fails.
    pub async fn begin_registration(
        &self,
        token: &SessionToken,
    ) -> Result<RegistrationRequest, SignInError> {
        let passkeys = self.passkeys.as_ref().ok_or(SignInError::PasskeysOff)?;
        let account = self.session(token).await?.account;
        let existing = passkeys.store.of(account.id()).await?;
        let challenge = passkeys.ask(Purpose::Register(account.id()), self.clock.now())?;
        Ok(RegistrationRequest {
            challenge,
            rp_id: passkeys.party.id().to_owned(),
            user_id: account.id().as_row().to_be_bytes(),
            exclude: existing
                .into_iter()
                .map(|stored| stored.passkey.credential_id().to_vec())
                .collect(),
        })
    }

    /// Adds the passkey the browser made for [`Self::begin_registration`]
    /// to the account `token` signs in, under `name`.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Passkey`] naming why the answer was refused,
    /// [`SignInError::Refused`] for no live session or a challenge that was
    /// not asked for this account, used or too old, and
    /// [`SignInError::Unavailable`] when a store fails.
    pub async fn finish_registration(
        &self,
        token: &SessionToken,
        answer: (&[u8], &[u8]),
        name: &str,
    ) -> Result<(), SignInError> {
        let (client_data, attestation) = answer;
        let passkeys = self.passkeys.as_ref().ok_or(SignInError::PasskeysOff)?;
        let account = self.session(token).await?.account;
        let now = self.clock.now();
        let purpose = Purpose::Register(account.id());
        let challenge = passkeys
            .take(client_data, purpose, now)
            .ok_or(SignInError::Refused)?;
        let registration =
            verify_registration(&passkeys.party, &challenge, client_data, attestation)
                .map_err(SignInError::Passkey)?;
        let added = passkeys
            .store
            .add(account.id(), &saved(&registration), name, now)
            .await?;
        added.then_some(()).ok_or(SignInError::Refused)
    }

    /// Starts a passkey sign-in: the challenge for
    /// `navigator.credentials.get`, with the site's host name. The browser
    /// offers whichever passkey the person has for this site.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::PasskeysOff`] when the site has no passkeys,
    /// and [`SignInError::Limited`] when too many requests are waiting.
    pub fn begin_sign_in(&self) -> Result<(Challenge, String), SignInError> {
        let passkeys = self.passkeys.as_ref().ok_or(SignInError::PasskeysOff)?;
        let challenge = passkeys.ask(Purpose::SignIn, self.clock.now())?;
        Ok((challenge, passkeys.party.id().to_owned()))
    }

    /// Signs in with the passkey `credential_id` names, from the browser's
    /// answer to [`Self::begin_sign_in`]: its client data, authenticator
    /// data and signature.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] for an unknown passkey, a challenge
    /// that was not asked, used or too old, or any check that fails, one
    /// answer for every reason; [`SignInError::PasskeysOff`] when the site
    /// has no passkeys; and [`SignInError::Unavailable`] when a store fails.
    pub async fn finish_sign_in(
        &self,
        credential_id: &[u8],
        answer: (&[u8], &[u8], &[u8]),
    ) -> Result<(SessionToken, SignedIn), SignInError> {
        let (client_data, authenticator_data, signature) = answer;
        let passkeys = self.passkeys.as_ref().ok_or(SignInError::PasskeysOff)?;
        let now = self.clock.now();
        let challenge = passkeys.take(client_data, Purpose::SignIn, now);
        let challenge = challenge.ok_or(SignInError::Refused)?;
        let stored = passkeys.store.find(credential_id).await?;
        let stored = stored.ok_or(SignInError::Refused)?;
        let (public_key, held) = held(&stored.passkey).ok_or(SignInError::Refused)?;
        let count = verify_assertion(
            &passkeys.party,
            &challenge,
            client_data,
            authenticator_data,
            signature,
            (&public_key, held),
        )
        .map_err(|_| SignInError::Refused)?;
        let registration = Registration {
            credential_id: credential_id.to_vec(),
            public_key,
            sign_count: count,
        };
        passkeys.store.used(&saved(&registration), now).await?;
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

/// The old checks' passkey in the store's form. The next part moves the
/// ceremonies onto `PasskeySite`, whose passkeys are already in it.
fn saved(registration: &Registration) -> SavedPasskey {
    let key = URL_SAFE_NO_PAD.encode(registration.public_key.to_stored());
    let json = serde_json::json!({ "key": key, "count": registration.sign_count });
    SavedPasskey::from_store(registration.credential_id.clone(), json.to_string())
}

/// The key and counter [`saved`] wrote.
fn held(passkey: &SavedPasskey) -> Option<(PublicKey, u32)> {
    let json: serde_json::Value = serde_json::from_str(passkey.json()).ok()?;
    let key = URL_SAFE_NO_PAD.decode(json.get("key")?.as_str()?).ok()?;
    let count = u32::try_from(json.get("count")?.as_u64()?).ok()?;
    Some((PublicKey::from_stored(&key).ok()?, count))
}
