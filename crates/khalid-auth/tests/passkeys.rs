//! Passkeys, registered and used by a software authenticator built on
//! `ring`, the way a phone or laptop would, and every way a passkey answer
//! can be refused, first alone and then through a site's accounts.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use ciborium::Value;
use khalid_api::keys::Role;
use khalid_auth::{
    Account, AccountStore, Accounts, CEREMONY, Challenge, MemoryAccounts, MemoryPasskeys,
    MemorySessions, PasskeyError, Passkeys, PasswordHashing, PublicKey, Registration, RelyingParty,
    SessionToken, SignInError, verify_assertion, verify_registration,
};
use khalid_core::Clock;
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

/// Signs `soft` in and checks the answer against `key` at `stored`.
#[cfg(test)]
fn sign_in(soft: &mut Soft, key: &PublicKey, stored: u32) -> Result<u32, PasskeyError> {
    let challenge = Challenge::generate().unwrap();
    let (client, data, signature) = soft.sign_in(&challenge, ORIGIN);
    verify_assertion(
        &party(),
        &challenge,
        &client,
        &data,
        &signature,
        (key, stored),
    )
}

#[test]
fn a_passkey_registers_and_then_signs_in_with_either_kind_of_key() {
    for mut soft in [Soft::es256(), Soft::ed25519()] {
        let registered = register(&soft, ORIGIN, None).unwrap();
        assert_eq!(registered.credential_id, soft.id);
        let key = PublicKey::from_stored(&registered.public_key.to_stored()).unwrap();
        assert_eq!(sign_in(&mut soft, &key, 0), Ok(1));
        assert_eq!(sign_in(&mut soft, &key, 1), Ok(2));
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

#[test]
fn a_bad_signature_another_key_a_replay_or_the_wrong_ceremony_is_refused() {
    let mut soft = Soft::es256();
    let key = register(&soft, ORIGIN, None).unwrap().public_key;
    let other = register(&Soft::es256(), ORIGIN, None).unwrap().public_key;
    assert_eq!(
        sign_in(&mut soft, &other, 0),
        Err(PasskeyError::BadSignature)
    );
    assert_eq!(
        sign_in(&mut soft, &key, 5),
        Err(PasskeyError::Replayed),
        "the counter went back"
    );
    let challenge = Challenge::generate().unwrap();
    let (_, data, mut signature) = soft.sign_in(&challenge, ORIGIN);
    let (client, _) = soft.register(&challenge, ORIGIN);
    let wrong = verify_assertion(&party(), &challenge, &client, &data, &signature, (&key, 0));
    assert_eq!(wrong, Err(PasskeyError::WrongCeremony));
    let client = client_data("webauthn.get", &challenge, ORIGIN);
    if let Some(last) = signature.last_mut() {
        *last ^= 1;
    }
    let forged = verify_assertion(&party(), &challenge, &client, &data, &signature, (&key, 0));
    assert_eq!(forged, Err(PasskeyError::BadSignature));
}

#[test]
fn a_counter_that_stays_at_zero_is_a_synced_passkey_and_allowed() {
    let mut soft = Soft {
        step: 0,
        ..Soft::es256()
    };
    let key = register(&soft, ORIGIN, None).unwrap().public_key;
    assert_eq!(sign_in(&mut soft, &key, 0), Ok(0));
    assert_eq!(sign_in(&mut soft, &key, 0), Ok(0));
}

#[test]
fn a_site_under_a_folder_has_the_origin_of_its_whole_host() {
    let party = |text| RelyingParty::for_address(&khalid_core::AbsoluteUrl::parse(text).unwrap());
    let want = |id: &str, origin: &str| RelyingParty::new(id.to_owned(), origin.to_owned());
    assert_eq!(
        party("https://example.com/blog/"),
        want("example.com", "https://example.com")
    );
    assert_eq!(
        party("http://localhost:4000/"),
        want("localhost", "http://localhost:4000")
    );
}

/// A clock the test moves by hand.
#[derive(Debug)]
struct Hands(Mutex<DateTime<Utc>>);

impl Clock for Hands {
    #[expect(clippy::expect_used, reason = "a test clock")]
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().expect("lock poisoned")
    }
}

/// A site with passkeys, its clock, and Asif, the owner, signed in.
#[cfg(test)]
async fn site() -> (Accounts, Arc<Hands>, Account, SessionToken) {
    let start = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let clock = Arc::new(Hands(Mutex::new(start)));
    let store = Arc::new(MemoryAccounts::new());
    let email = "asif@example.com".parse().unwrap();
    let asif = store
        .create(&email, "Asif", Role::Owner, start)
        .await
        .unwrap();
    let shared: Arc<dyn Clock> = Arc::<Hands>::clone(&clock);
    let hashing = PasswordHashing::fast_for_tests();
    let accounts = Accounts::new(store, Arc::new(MemorySessions::new()), hashing, shared)
        .with_passkeys(Passkeys::new(Arc::new(MemoryPasskeys::new()), party()));
    let token = accounts.begin(&asif).await.unwrap();
    (accounts, clock, asif, token)
}

#[tokio::test]
async fn an_owner_adds_a_passkey_and_signs_in_with_it_once_per_challenge() {
    let (accounts, _, asif, token) = site().await;
    let mut soft = Soft::es256();
    let request = accounts.begin_registration(&token).await.unwrap();
    assert_eq!((request.rp_id.as_str(), request.exclude.len()), (RP, 0));
    let (client, object) = soft.register(&request.challenge, ORIGIN);
    accounts
        .finish_registration(&token, (&client, &object), "Laptop")
        .await
        .unwrap();
    let request = accounts.begin_registration(&token).await.unwrap();
    assert_eq!(
        request.exclude,
        [soft.id.clone()],
        "the browser is told not to add it twice"
    );
    let (challenge, _) = accounts.begin_sign_in().unwrap();
    let (client, data, signature) = soft.sign_in(&challenge, ORIGIN);
    let answer = (client.as_slice(), data.as_slice(), signature.as_slice());
    let (session, signed_in) = accounts.finish_sign_in(&soft.id, answer).await.unwrap();
    assert_eq!(signed_in.account, asif);
    assert!(accounts.session(&session).await.is_ok());
    let again = accounts.finish_sign_in(&soft.id, answer).await;
    assert!(
        matches!(again, Err(SignInError::Refused)),
        "a challenge is used once: {again:?}"
    );
}

#[tokio::test]
async fn a_registration_answers_only_its_own_fresh_challenge() {
    let (accounts, clock, _, token) = site().await;
    let soft = Soft::es256();
    let request = accounts.begin_registration(&token).await.unwrap();
    let (client, object) = soft.register(&request.challenge, "https://evil.example");
    let refused = accounts
        .finish_registration(&token, (&client, &object), "Laptop")
        .await;
    assert!(
        matches!(
            refused,
            Err(SignInError::Passkey(PasskeyError::WrongOrigin))
        ),
        "{refused:?}"
    );
    let (sign_in, _) = accounts.begin_sign_in().unwrap();
    let (client, object) = soft.register(&sign_in, ORIGIN);
    let refused = accounts
        .finish_registration(&token, (&client, &object), "Laptop")
        .await;
    assert!(
        matches!(refused, Err(SignInError::Refused)),
        "a sign-in challenge: {refused:?}"
    );
    let request = accounts.begin_registration(&token).await.unwrap();
    *clock.0.lock().unwrap() += CEREMONY;
    let (client, object) = soft.register(&request.challenge, ORIGIN);
    let late = accounts
        .finish_registration(&token, (&client, &object), "Laptop")
        .await;
    assert!(
        matches!(late, Err(SignInError::Refused)),
        "too late: {late:?}"
    );
}

#[tokio::test]
async fn an_unknown_passkey_or_a_site_without_passkeys_signs_nobody_in() {
    let (accounts, _, _, _) = site().await;
    let mut soft = Soft::es256();
    let (challenge, _) = accounts.begin_sign_in().unwrap();
    let (client, data, signature) = soft.sign_in(&challenge, ORIGIN);
    let answer = (client.as_slice(), data.as_slice(), signature.as_slice());
    let unknown = accounts.finish_sign_in(&soft.id, answer).await;
    assert!(matches!(unknown, Err(SignInError::Refused)), "{unknown:?}");
    let start = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let plain = Accounts::new(
        Arc::new(MemoryAccounts::new()),
        Arc::new(MemorySessions::new()),
        PasswordHashing::fast_for_tests(),
        Arc::new(khalid_core::FixedClock::new(start)),
    );
    assert!(matches!(
        plain.begin_sign_in(),
        Err(SignInError::PasskeysOff)
    ));
}

/// The challenge in an options answer, read the way a browser would.
#[cfg(test)]
fn challenge_in(options: &serde_json::Value) -> Challenge {
    let text = options["challenge"].as_str().unwrap();
    let client = serde_json::json!({ "type": "webauthn.get", "challenge": text, "origin": ORIGIN });
    Challenge::answered_in(client.to_string().as_bytes()).unwrap()
}

#[cfg(test)]
async fn call(
    router: &axum::Router,
    uri: &str,
    session: Option<&SessionToken>,
    body: serde_json::Value,
) -> (u16, serde_json::Value) {
    let mut request = axum::http::Request::post(uri).header("content-type", "application/json");
    if let Some(token) = session {
        let cookie = token.set_cookie();
        let pair = cookie.expose().split(';').next().unwrap().to_owned();
        request = request
            .header("cookie", pair)
            .header("x-khalid-csrf", token.csrf().as_str());
    }
    let request = request
        .body(axum::body::Body::from(body.to_string()))
        .unwrap();
    let response = tower::ServiceExt::oneshot(router.clone(), request)
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

#[tokio::test]
async fn a_passkey_is_added_and_used_over_the_admin_api() {
    let (accounts, _, _, token) = site().await;
    let (router, _) = khalid_api::api::router_with_staff(
        Arc::new(khalid_api::content_store::MemoryContentStore::new(
            Vec::new(),
        )),
        Arc::new(khalid_api::post_store::MemoryPostStore::default()),
        Arc::new(khalid_api::keys::MemoryKeyStore::new(Vec::new())),
        khalid_auth::staff_routes(accounts.clone()),
        Arc::new(accounts),
    );
    let mut soft = Soft::es256();
    let b64 = |bytes: &[u8]| {
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
    };
    let (status, options) = call(
        &router,
        "/api/admin/v1/session/passkeys/options/",
        Some(&token),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(
        (status, options["algorithms"].clone()),
        (200, serde_json::json!([-7, -8])),
        "{options}"
    );
    let (client, object) = soft.register(&challenge_in(&options), ORIGIN);
    let body = serde_json::json!({ "name": "Laptop", "client_data": b64(&client), "attestation_object": b64(&object) });
    let (status, json) = call(
        &router,
        "/api/admin/v1/session/passkeys/",
        Some(&token),
        body.clone(),
    )
    .await;
    assert_eq!(status, 201, "{json}");
    let (status, _) = call(&router, "/api/admin/v1/session/passkeys/", None, body).await;
    assert_eq!(status, 401, "adding a passkey needs the session");
    let (status, options) = call(
        &router,
        "/api/admin/v1/session/passkey/options/",
        None,
        serde_json::json!({}),
    )
    .await;
    assert_eq!((status, options["rp_id"].as_str()), (200, Some(RP)));
    let (client, data, signature) = soft.sign_in(&challenge_in(&options), ORIGIN);
    let body = serde_json::json!({
        "credential_id": b64(&soft.id), "client_data": b64(&client),
        "authenticator_data": b64(&data), "signature": b64(&signature),
    });
    let (status, json) = call(
        &router,
        "/api/admin/v1/session/passkey/",
        None,
        body.clone(),
    )
    .await;
    assert_eq!(
        (status, json["account"]["role"].as_str()),
        (200, Some("owner")),
        "{json}"
    );
    let (status, _) = call(&router, "/api/admin/v1/session/passkey/", None, body).await;
    assert_eq!(status, 401, "a challenge is answered once");
}
