//! Passkeys, registered and used by a software authenticator built on
//! `ring`, the way a phone or laptop would, and every way a passkey answer
//! can be refused.

use ciborium::Value;
use khalid_auth::{
    Challenge, PasskeyError, PublicKey, Registration, RelyingParty, verify_registration,
};
use ring::rand::SystemRandom;
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha256};

const ORIGIN: &str = "https://blog.example.com";
const RP: &str = "blog.example.com";

/// User present, user verified, attested credential data follows.
const UP: u8 = 0x01;
const UV: u8 = 0x04;
const AT: u8 = 0x40;

/// The key a software authenticator signs with.
enum Key {
    Es256(EcdsaKeyPair),
    Ed25519(Ed25519KeyPair),
}

/// An authenticator in software: one credential, a counter, and the flags
/// it sets.
struct Soft {
    key: Key,
    id: Vec<u8>,
    count: u32,
    /// How much the counter moves per sign-in: 0 for a synced passkey.
    step: u32,
    flags: u8,
    rp: &'static str,
}

impl Soft {
    /// A counting authenticator for this site, whose person unlocks it.
    #[cfg(test)]
    fn new(key: Key) -> Self {
        let (id, count, step, flags, rp) = (vec![7; 16], 0, 1, UP | UV, RP);
        Self {
            key,
            id,
            count,
            step,
            flags,
            rp,
        }
    }

    #[cfg(test)]
    fn es256() -> Self {
        let (alg, rng) = (&ECDSA_P256_SHA256_ASN1_SIGNING, SystemRandom::new());
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(alg, &rng).unwrap();
        Self::new(Key::Es256(
            EcdsaKeyPair::from_pkcs8(alg, pkcs8.as_ref(), &rng).unwrap(),
        ))
    }

    #[cfg(test)]
    fn ed25519() -> Self {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        Self::new(Key::Ed25519(
            Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap(),
        ))
    }

    /// The public key as a CBOR map, the way authenticators send it.
    #[cfg(test)]
    fn credential_key(&self) -> Vec<u8> {
        let int = |n: i64| Value::Integer(n.into());
        let map = match &self.key {
            Key::Es256(pair) => {
                let point = pair.public_key().as_ref();
                vec![
                    (int(1), int(2)),
                    (int(3), int(-7)),
                    (int(-1), int(1)),
                    (int(-2), Value::Bytes(point.get(1..33).unwrap().to_vec())),
                    (int(-3), Value::Bytes(point.get(33..).unwrap().to_vec())),
                ]
            }
            Key::Ed25519(pair) => vec![
                (int(1), int(1)),
                (int(3), int(-8)),
                (int(-1), int(6)),
                (int(-2), Value::Bytes(pair.public_key().as_ref().to_vec())),
            ],
        };
        let mut bytes = Vec::new();
        ciborium::into_writer(&Value::Map(map), &mut bytes).unwrap();
        bytes
    }

    #[cfg(test)]
    fn authenticator_data(&self, flags: u8) -> Vec<u8> {
        let mut data = Sha256::digest(self.rp.as_bytes()).to_vec();
        data.push(flags);
        data.extend(self.count.to_be_bytes());
        if flags & AT != 0 {
            data.extend([0; 16]);
            data.extend(u16::try_from(self.id.len()).unwrap().to_be_bytes());
            data.extend(&self.id);
            data.extend(self.credential_key());
        }
        data
    }

    /// What `navigator.credentials.create` returns: client data and the
    /// attestation object, with no attestation.
    #[cfg(test)]
    fn register(&self, challenge: &Challenge, origin: &str) -> (Vec<u8>, Vec<u8>) {
        let client = client_data("webauthn.create", challenge, origin);
        let object = Value::Map(vec![
            (Value::Text("fmt".into()), Value::Text("none".into())),
            (Value::Text("attStmt".into()), Value::Map(Vec::new())),
            (
                Value::Text("authData".into()),
                Value::Bytes(self.authenticator_data(self.flags | AT)),
            ),
        ]);
        let mut bytes = Vec::new();
        ciborium::into_writer(&object, &mut bytes).unwrap();
        (client, bytes)
    }

    /// What `navigator.credentials.get` returns: client data,
    /// authenticator data and the signature over both.
    #[cfg(test)]
    fn sign_in(&mut self, challenge: &Challenge, origin: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        self.count += self.step;
        let client = client_data("webauthn.get", challenge, origin);
        let data = self.authenticator_data(self.flags);
        let signed = [data.as_slice(), Sha256::digest(&client).as_slice()].concat();
        let signature = match &self.key {
            Key::Es256(pair) => pair
                .sign(&SystemRandom::new(), &signed)
                .unwrap()
                .as_ref()
                .to_vec(),
            Key::Ed25519(pair) => pair.sign(&signed).as_ref().to_vec(),
        };
        (client, data, signature)
    }
}

#[cfg(test)]
fn client_data(kind: &str, challenge: &Challenge, origin: &str) -> Vec<u8> {
    serde_json::json!({ "type": kind, "challenge": challenge.to_base64(), "origin": origin })
        .to_string()
        .into_bytes()
}

#[cfg(test)]
fn party() -> RelyingParty {
    RelyingParty::new(RP.to_owned(), ORIGIN.to_owned())
}

/// Registers `soft` for a challenge the page at `origin` asked for, and
/// checks the answer against `asked`, or that challenge when `None`.
#[cfg(test)]
fn register(
    soft: &Soft,
    origin: &str,
    asked: Option<&Challenge>,
) -> Result<Registration, PasskeyError> {
    let challenge = Challenge::generate().unwrap();
    let (client, object) = soft.register(&challenge, origin);
    verify_registration(&party(), asked.unwrap_or(&challenge), &client, &object)
}

#[test]
fn a_passkey_registers_with_the_key_its_signatures_check_against() {
    for mut soft in [Soft::es256(), Soft::ed25519()] {
        let registered = register(&soft, ORIGIN, None).unwrap();
        assert_eq!(registered.credential_id, soft.id);
        let key = PublicKey::from_stored(&registered.public_key.to_stored()).unwrap();
        let challenge = Challenge::generate().unwrap();
        let (client, data, signature) = soft.sign_in(&challenge, ORIGIN);
        let signed = [data.as_slice(), Sha256::digest(&client).as_slice()].concat();
        assert!(key.verifies(&signed, &signature));
    }
}

#[test]
fn a_registration_for_another_site_or_challenge_is_refused() {
    let refused = |soft: &Soft, origin, asked| register(soft, origin, asked).err();
    let wrong = Some(PasskeyError::WrongOrigin);
    assert_eq!(refused(&Soft::es256(), "https://evil.example", None), wrong);
    let other = Challenge::generate().unwrap();
    let wrong = Some(PasskeyError::WrongChallenge);
    assert_eq!(refused(&Soft::es256(), ORIGIN, Some(&other)), wrong);
    let elsewhere = Soft {
        rp: "evil.example",
        ..Soft::es256()
    };
    assert_eq!(
        refused(&elsewhere, ORIGIN, None),
        Some(PasskeyError::WrongRelyingParty)
    );
    let unverified = Soft {
        flags: UP,
        ..Soft::es256()
    };
    assert_eq!(
        refused(&unverified, ORIGIN, None),
        Some(PasskeyError::NotVerified)
    );
    let (client, _) = Soft::es256().register(&other, ORIGIN);
    let garbage = verify_registration(&party(), &other, &client, b"not cbor");
    assert_eq!(garbage.err(), Some(PasskeyError::Malformed));
}
