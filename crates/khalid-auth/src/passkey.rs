//! Passkeys: registering one and signing in with it, checked the way
//! `WebAuthn` Level 2 asks (sections 7.1 and 7.2), with no attestation.

mod authenticator_data;
mod public_key;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ciborium::Value;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub use authenticator_data::AuthenticatorData;
pub use public_key::PublicKey;

/// The longest credential identity `WebAuthn` allows (section 5.8.3).
const MAX_CREDENTIAL_ID: usize = 1023;

/// The site passkeys belong to: its host name, and the origin pages that
/// ask for a passkey are served from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelyingParty {
    id: String,
    origin: String,
}

impl RelyingParty {
    /// The relying party for a site at `origin`, such as
    /// `https://blog.example.com`, whose host name `id` is.
    #[must_use]
    pub fn new(id: String, origin: String) -> Self {
        Self { id, origin }
    }

    /// The host name, which the browser checks passkeys against.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// 256 random bits the browser must sign, used once.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Challenge([u8; 32]);

impl Challenge {
    /// A new challenge from the system's random source.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyError::Random`] when there is no random source.
    pub fn generate() -> Result<Self, PasskeyError> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| PasskeyError::Random)?;
        Ok(Self(bytes))
    }

    /// The challenge as the browser gets it: base64url, unpadded.
    #[must_use]
    pub fn to_base64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }
}

/// A passkey as registration made it, for the store to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// The authenticator's name for the credential.
    pub credential_id: Vec<u8>,
    /// Its public key.
    pub public_key: PublicKey,
    /// Its signature counter so far.
    pub sign_count: u32,
}

/// Checks a new passkey: `client_data` and `attestation_object` are the
/// bytes `navigator.credentials.create` returned for `challenge`.
///
/// # Errors
///
/// Returns a [`PasskeyError`] naming the first check that failed.
pub fn verify_registration(
    party: &RelyingParty,
    challenge: &Challenge,
    client_data: &[u8],
    attestation_object: &[u8],
) -> Result<Registration, PasskeyError> {
    check_client_data(party, challenge, client_data, "webauthn.create")?;
    let object: Value =
        ciborium::from_reader(attestation_object).map_err(|_| PasskeyError::Malformed)?;
    let auth_data = object
        .as_map()
        .and_then(|map| {
            map.iter()
                .find(|(key, _)| key.as_text() == Some("authData"))
        })
        .and_then(|(_, value)| value.as_bytes())
        .ok_or(PasskeyError::Malformed)?;
    let data = check_authenticator_data(party, auth_data)?;
    let sign_count = data.sign_count();
    let (credential_id, public_key) = data.into_credential().ok_or(PasskeyError::Malformed)?;
    if credential_id.is_empty() || credential_id.len() > MAX_CREDENTIAL_ID {
        return Err(PasskeyError::Malformed);
    }
    Ok(Registration {
        credential_id,
        public_key,
        sign_count,
    })
}

/// What the browser says it was asked, in `clientDataJSON`.
#[derive(Debug, Deserialize)]
struct ClientData {
    #[serde(rename = "type")]
    kind: String,
    challenge: String,
    origin: String,
    #[serde(default, rename = "crossOrigin")]
    cross_origin: bool,
}

fn check_client_data(
    party: &RelyingParty,
    challenge: &Challenge,
    bytes: &[u8],
    kind: &str,
) -> Result<(), PasskeyError> {
    let data: ClientData = serde_json::from_slice(bytes).map_err(|_| PasskeyError::Malformed)?;
    if data.kind != kind {
        return Err(PasskeyError::WrongCeremony);
    }
    if data.challenge != challenge.to_base64() {
        return Err(PasskeyError::WrongChallenge);
    }
    if data.origin != party.origin || data.cross_origin {
        return Err(PasskeyError::WrongOrigin);
    }
    Ok(())
}

fn check_authenticator_data(
    party: &RelyingParty,
    bytes: &[u8],
) -> Result<AuthenticatorData, PasskeyError> {
    let data = AuthenticatorData::parse(bytes)?;
    if *data.rp_id_hash() != <[u8; 32]>::from(Sha256::digest(party.id.as_bytes())) {
        return Err(PasskeyError::WrongRelyingParty);
    }
    if !data.user_verified() {
        return Err(PasskeyError::NotVerified);
    }
    Ok(data)
}

/// A passkey was refused, or could not be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PasskeyError {
    /// The bytes are not what `WebAuthn` describes.
    #[error("the passkey answer cannot be read")]
    Malformed,
    /// The key is not ES256 or Ed25519.
    #[error("this kind of passkey is not supported")]
    UnsupportedKey,
    /// A registration was sent to sign in, or the other way round.
    #[error("the passkey answered a different request")]
    WrongCeremony,
    /// The challenge is not the one Khalid sent, or it was used already.
    #[error("the passkey answered an old or unknown challenge")]
    WrongChallenge,
    /// The page that asked was on another site.
    #[error("the passkey was asked for by another site")]
    WrongOrigin,
    /// The passkey belongs to another site.
    #[error("the passkey belongs to another site")]
    WrongRelyingParty,
    /// Nobody unlocked the passkey with a fingerprint, face or PIN.
    #[error("the passkey was not unlocked by its owner")]
    NotVerified,
    /// The signature does not match the stored key.
    #[error("the passkey's signature does not match")]
    BadSignature,
    /// The counter went back, so the passkey may have been copied.
    #[error("the passkey's counter went back")]
    Replayed,
    /// The system has no random source.
    #[error("the system's random source is not available")]
    Random,
}
