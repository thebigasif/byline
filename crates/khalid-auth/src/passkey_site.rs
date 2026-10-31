//! Passkeys checked by `webauthn-rs`: the site they belong to, the two
//! ceremonies that add one and sign in with one, and the form a store
//! keeps (ADR "passkeys are checked by webauthn-rs").

use khalid_core::AbsoluteUrl;
use webauthn_rs::prelude::{
    CreationChallengeResponse, DiscoverableAuthentication, DiscoverableKey, Passkey,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url, Uuid, Webauthn, WebauthnBuilder, WebauthnError,
};

use crate::Account;

/// The site passkeys belong to: its host name, and the `webauthn-rs`
/// checks for pages served from its address.
#[derive(Debug, Clone)]
pub struct PasskeySite {
    webauthn: Webauthn,
    id: String,
}

impl PasskeySite {
    /// The site at `address`, the address in `site.toml`. A blog under a
    /// folder, such as `/blog/`, takes passkeys from its whole host, as
    /// browsers do.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyCheckError::NoHostName`] when the address has no
    /// domain name, such as `http://127.0.0.1/`: browsers keep passkeys
    /// only for names.
    pub fn for_address(address: &AbsoluteUrl) -> Result<Self, PasskeyCheckError> {
        let host_only = |text: &str| {
            let origin = Url::parse(text).ok()?.origin().ascii_serialization();
            Url::parse(&origin).ok()
        };
        let origin = host_only(address.as_str()).ok_or(PasskeyCheckError::NoHostName)?;
        let id = origin
            .domain()
            .ok_or(PasskeyCheckError::NoHostName)?
            .to_owned();
        let webauthn = WebauthnBuilder::new(&id, &origin)
            .and_then(WebauthnBuilder::build)
            .map_err(|_| PasskeyCheckError::NoHostName)?;
        Ok(Self { webauthn, id })
    }

    /// The host name, which the browser checks passkeys against.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Starts adding a passkey to `account`: the options for
    /// `navigator.credentials.create`, and what Khalid keeps until the
    /// answer comes. `exclude` names the passkeys the account already has,
    /// so the browser does not add one twice.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyCheckError::Other`] when `webauthn-rs` cannot make
    /// the options.
    pub fn start_registration(
        &self,
        account: &Account,
        exclude: Vec<Vec<u8>>,
    ) -> Result<(CreationChallengeResponse, Registering), PasskeyCheckError> {
        let handle = Uuid::from_u64_pair(0, account.id().as_row().cast_unsigned());
        let exclude = (!exclude.is_empty()).then_some(exclude);
        let email = account.email().as_str();
        self.webauthn
            .start_passkey_registration(handle, email, account.name(), exclude)
            .map(|(options, state)| (options, Registering(state)))
            .map_err(refusal)
    }

    /// Checks the new passkey `credential`, made for `registering`.
    ///
    /// # Errors
    ///
    /// Returns a [`PasskeyCheckError`] naming why the passkey was refused.
    pub fn finish_registration(
        &self,
        credential: &RegisterPublicKeyCredential,
        registering: &Registering,
    ) -> Result<SavedPasskey, PasskeyCheckError> {
        let passkey = self
            .webauthn
            .finish_passkey_registration(credential, &registering.0)
            .map_err(refusal)?;
        SavedPasskey::of(&passkey)
    }

    /// Starts a sign-in: the options for `navigator.credentials.get`, and
    /// what Khalid keeps until the answer comes. The browser offers
    /// whichever passkey the person has for this site.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyCheckError::Other`] when `webauthn-rs` cannot make
    /// the options.
    pub fn start_sign_in(
        &self,
    ) -> Result<(RequestChallengeResponse, SigningIn), PasskeyCheckError> {
        self.webauthn
            .start_discoverable_authentication()
            .map(|(options, state)| (options, SigningIn(state)))
            .map_err(refusal)
    }

    /// Checks `credential`, the answer to `signing_in`, against the stored
    /// passkey it names, and returns that passkey with its new counter.
    ///
    /// # Errors
    ///
    /// Returns a [`PasskeyCheckError`] naming why the answer was refused;
    /// [`PasskeyCheckError::Replayed`] when the counter went back, which
    /// means the passkey may have been copied.
    pub fn finish_sign_in(
        &self,
        credential: &PublicKeyCredential,
        signing_in: SigningIn,
        saved: &SavedPasskey,
    ) -> Result<SavedPasskey, PasskeyCheckError> {
        let mut passkey = saved.passkey()?;
        let key = DiscoverableKey::from(&passkey);
        let result = self
            .webauthn
            .finish_discoverable_authentication(credential, signing_in.0, &[key])
            .map_err(refusal)?;
        passkey.update_credential(&result);
        SavedPasskey::of(&passkey)
    }
}

/// A passkey being added, kept until the browser answers.
#[derive(Debug, Clone)]
pub struct Registering(PasskeyRegistration);

/// A sign-in under way, kept until the browser answers.
#[derive(Debug, Clone)]
pub struct SigningIn(DiscoverableAuthentication);

/// A passkey as a store keeps it: the authenticator's name for it, and the
/// passkey as `webauthn-rs` saves it, in JSON. Never anything that could
/// sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPasskey {
    credential_id: Vec<u8>,
    json: String,
}

impl SavedPasskey {
    /// A passkey read back from a store. It is checked when it is used.
    #[must_use]
    pub fn from_store(credential_id: Vec<u8>, json: String) -> Self {
        Self {
            credential_id,
            json,
        }
    }

    /// The authenticator's name for the passkey.
    #[must_use]
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    /// The passkey as `webauthn-rs` saves it.
    #[must_use]
    pub fn json(&self) -> &str {
        &self.json
    }

    fn of(passkey: &Passkey) -> Result<Self, PasskeyCheckError> {
        let json = serde_json::to_string(passkey).map_err(|_| PasskeyCheckError::Stored)?;
        Ok(Self::from_store(passkey.cred_id().clone(), json))
    }

    /// The passkey, when the JSON reads and names the same credential.
    fn passkey(&self) -> Result<Passkey, PasskeyCheckError> {
        let passkey: Passkey =
            serde_json::from_str(&self.json).map_err(|_| PasskeyCheckError::Stored)?;
        if *passkey.cred_id() != self.credential_id {
            return Err(PasskeyCheckError::Stored);
        }
        Ok(passkey)
    }
}

/// A passkey was refused, or could not be checked.
#[derive(Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PasskeyCheckError {
    /// The site address has no domain name for passkeys to belong to.
    #[error("the site address has no host name a passkey can belong to")]
    NoHostName,
    /// The browser's answer is not what `WebAuthn` describes.
    #[error("the passkey answer cannot be read")]
    Malformed,
    /// The answer is to another challenge.
    #[error("the passkey answered a different request")]
    WrongChallenge,
    /// The page that asked was on another site, or the passkey belongs to
    /// another site.
    #[error("the passkey belongs to another site, or another site asked for it")]
    WrongSite,
    /// Nobody unlocked the passkey with a fingerprint, face or PIN.
    #[error("the passkey was not unlocked by its owner")]
    NotVerified,
    /// The signature does not match the stored key.
    #[error("the passkey's signature does not match")]
    BadSignature,
    /// The counter went back, so the passkey may have been copied.
    #[error("the passkey's counter went back")]
    Replayed,
    /// The stored passkey cannot be read.
    #[error("the stored passkey cannot be read")]
    Stored,
    /// Any other check failed, with what `webauthn-rs` reported.
    #[error("the passkey was refused")]
    Other(#[source] WebauthnError),
}

/// The plain reason for what `webauthn-rs` refused.
fn refusal(error: WebauthnError) -> PasskeyCheckError {
    match &error {
        WebauthnError::MismatchedChallenge | WebauthnError::InvalidClientDataType => {
            PasskeyCheckError::WrongChallenge
        }
        WebauthnError::InvalidRPOrigin
        | WebauthnError::InvalidRPIDHash
        | WebauthnError::CredentialCrossOrigin => PasskeyCheckError::WrongSite,
        WebauthnError::UserNotVerified | WebauthnError::UserNotPresent => {
            PasskeyCheckError::NotVerified
        }
        WebauthnError::AuthenticationFailure | WebauthnError::SignatureInvalid => {
            PasskeyCheckError::BadSignature
        }
        WebauthnError::CredentialPossibleCompromise => PasskeyCheckError::Replayed,
        WebauthnError::ParseBase64Failure(_)
        | WebauthnError::ParseCBORFailure(_)
        | WebauthnError::ParseJSONFailure(_)
        | WebauthnError::ParseNOMFailure
        | WebauthnError::ParseInsufficientBytesAvailable => PasskeyCheckError::Malformed,
        _ => PasskeyCheckError::Other(error),
    }
}
