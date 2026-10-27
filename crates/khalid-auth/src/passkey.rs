//! Passkeys: registering one and signing in with it, checked the way
//! `WebAuthn` Level 2 asks (sections 7.1 and 7.2), with no attestation.

mod authenticator_data;
mod public_key;

pub use authenticator_data::AuthenticatorData;
pub use public_key::PublicKey;

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
