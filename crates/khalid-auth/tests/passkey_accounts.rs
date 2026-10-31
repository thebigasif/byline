//! Passkeys through a site's accounts and over the Admin API, registered
//! and used by `webauthn-rs`'s software passkey the way a phone or laptop
//! would.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{
    Account, AccountStore, Accounts, CEREMONY, MemoryAccounts, MemoryPasskeys, MemorySessions,
    PasskeyCheckError, PasskeySite, Passkeys, PasswordHashing, PublicKeyCredential,
    RequestChallengeResponse, SessionToken, SignInError,
};
use khalid_core::{AbsoluteUrl, Clock};
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_authenticator_rs::prelude::Url;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs_proto::AllowCredentials;

const ORIGIN: &str = "https://blog.example.com";

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
    let address = AbsoluteUrl::parse(ORIGIN).unwrap();
    let passkey_site = PasskeySite::for_address(&address).unwrap();
    let passkeys = Passkeys::new(Arc::new(MemoryPasskeys::new()), passkey_site);
    let hashing = PasswordHashing::fast_for_tests();
    let accounts = Accounts::new(store, Arc::new(MemorySessions::new()), hashing, shared)
        .with_passkeys(passkeys);
    let token = accounts.begin(&asif).await.unwrap();
    (accounts, clock, asif, token)
}

#[cfg(test)]
fn url(origin: &str) -> Url {
    Url::parse(origin).unwrap()
}

/// The browser's answer to a sign-in. The software passkey cannot
/// discover its own credentials, so the test names the one it holds.
#[cfg(test)]
fn answer(
    soft: &mut SoftPasskey,
    mut options: RequestChallengeResponse,
    id: &[u8],
) -> PublicKeyCredential {
    options.public_key.allow_credentials.push(AllowCredentials {
        type_: "public-key".into(),
        id: id.to_vec(),
        transports: None,
    });
    soft.do_authentication(url(ORIGIN), options).unwrap()
}

/// Adds a passkey of `soft` to the account `token` signs in, and returns
/// its credential identity.
#[cfg(test)]
async fn add(accounts: &Accounts, token: &SessionToken, soft: &mut SoftPasskey) -> Vec<u8> {
    let options = accounts.begin_registration(token).await.unwrap();
    let challenge = options.public_key.challenge.clone();
    let credential = soft.do_registration(url(ORIGIN), options).unwrap();
    accounts
        .finish_registration(token, &challenge, &credential, "Laptop")
        .await
        .unwrap();
    credential.raw_id
}

#[tokio::test]
async fn an_owner_adds_a_passkey_and_signs_in_with_it_once_per_challenge() {
    let (accounts, _, asif, token) = site().await;
    let mut soft = SoftPasskey::new(true);
    let id = add(&accounts, &token, &mut soft).await;
    let options = accounts.begin_registration(&token).await.unwrap();
    let excluded = options.public_key.exclude_credentials.unwrap_or_default();
    let excluded: Vec<Vec<u8>> = excluded.into_iter().map(|c| c.id).collect();
    assert_eq!(
        excluded,
        std::slice::from_ref(&id),
        "the browser is told not to add it twice"
    );
    let options = accounts.begin_sign_in().unwrap();
    let challenge = options.public_key.challenge.clone();
    let credential = answer(&mut soft, options, &id);
    let (session, signed_in) = accounts
        .finish_sign_in(&challenge, &credential)
        .await
        .unwrap();
    assert_eq!(signed_in.account, asif);
    assert!(accounts.session(&session).await.is_ok());
    let again = accounts.finish_sign_in(&challenge, &credential).await;
    assert!(
        matches!(again, Err(SignInError::Refused)),
        "a challenge is used once: {again:?}"
    );
}

#[tokio::test]
async fn a_registration_answers_only_its_own_fresh_challenge() {
    let (accounts, clock, _, token) = site().await;
    let mut soft = SoftPasskey::new(true);
    let options = accounts.begin_registration(&token).await.unwrap();
    let challenge = options.public_key.challenge.clone();
    let elsewhere = url("https://blog.example.com:8443");
    let credential = soft.do_registration(elsewhere, options).unwrap();
    let refused = accounts
        .finish_registration(&token, &challenge, &credential, "Laptop")
        .await;
    assert!(
        matches!(
            refused,
            Err(SignInError::Passkey(PasskeyCheckError::WrongSite))
        ),
        "{refused:?}"
    );
    let sign_in = accounts.begin_sign_in().unwrap().public_key.challenge;
    let refused = accounts
        .finish_registration(&token, &sign_in, &credential, "Laptop")
        .await;
    assert!(
        matches!(refused, Err(SignInError::Refused)),
        "a sign-in challenge: {refused:?}"
    );
    let options = accounts.begin_registration(&token).await.unwrap();
    let challenge = options.public_key.challenge.clone();
    *clock.0.lock().unwrap() += CEREMONY;
    let credential = soft.do_registration(url(ORIGIN), options).unwrap();
    let late = accounts
        .finish_registration(&token, &challenge, &credential, "Laptop")
        .await;
    assert!(
        matches!(late, Err(SignInError::Refused)),
        "too late: {late:?}"
    );
}

#[tokio::test]
async fn an_unknown_passkey_or_a_site_without_passkeys_signs_nobody_in() {
    let (accounts, _, _, token) = site().await;
    let mut soft = SoftPasskey::new(true);
    let id = add(&accounts, &token, &mut soft).await;
    let (other, _, _, other_token) = site().await;
    add(&other, &other_token, &mut SoftPasskey::new(true)).await;
    let options = other.begin_sign_in().unwrap();
    let challenge = options.public_key.challenge.clone();
    let credential = answer(&mut soft, options, &id);
    let unknown = other.finish_sign_in(&challenge, &credential).await;
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
async fn a_passkey_is_added_and_used_over_the_admin_api_in_standard_json() {
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
    let mut soft = SoftPasskey::new(true);
    let at = |path: &str| format!("/api/admin/v1/session/{path}");
    let (status, options) = call(
        &router,
        &at("passkeys/options/"),
        Some(&token),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, 200, "{options}");
    let challenge = options["publicKey"]["challenge"].clone();
    let credential = soft
        .do_registration(url(ORIGIN), serde_json::from_value(options).unwrap())
        .unwrap();
    let id = credential.raw_id.clone();
    let body =
        serde_json::json!({ "name": "Laptop", "challenge": challenge, "credential": credential });
    let (status, json) = call(&router, &at("passkeys/"), Some(&token), body.clone()).await;
    assert_eq!(status, 201, "{json}");
    let (status, _) = call(&router, &at("passkeys/"), None, body).await;
    assert_eq!(status, 401, "adding a passkey needs the session");
    let (status, options) = call(
        &router,
        &at("passkey/options/"),
        None,
        serde_json::json!({}),
    )
    .await;
    assert_eq!(
        (status, options["publicKey"]["rpId"].as_str()),
        (200, Some("blog.example.com")),
        "{options}"
    );
    let challenge = options["publicKey"]["challenge"].clone();
    let credential = answer(&mut soft, serde_json::from_value(options).unwrap(), &id);
    let body = serde_json::json!({ "challenge": challenge, "credential": credential });
    let (status, json) = call(&router, &at("passkey/"), None, body.clone()).await;
    assert_eq!(
        (status, json["account"]["role"].as_str()),
        (200, Some("owner")),
        "{json}"
    );
    let (status, _) = call(&router, &at("passkey/"), None, body).await;
    assert_eq!(status, 401, "a challenge is answered once");
}
