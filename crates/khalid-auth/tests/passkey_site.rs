//! Passkeys checked by `webauthn-rs`, registered and used by its software
//! passkey the way a phone or laptop would, and the refusals Khalid names.

use khalid_api::keys::Role;
use khalid_auth::{
    Account, AccountId, PasskeyCheckError, PasskeySite, PublicKeyCredential,
    RequestChallengeResponse, SavedPasskey,
};
use khalid_core::AbsoluteUrl;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_authenticator_rs::prelude::Url;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs_proto::AllowCredentials;

const ORIGIN: &str = "https://blog.example.com";

#[cfg(test)]
fn site(address: &str) -> PasskeySite {
    PasskeySite::for_address(&AbsoluteUrl::parse(address).unwrap()).unwrap()
}

#[cfg(test)]
fn asif() -> Account {
    let email = "asif@example.com".parse().unwrap();
    Account::new(AccountId::from_row(1), email, "Asif".into(), Role::Owner)
}

/// A software passkey that unlocks itself, as a fingerprint would.
#[cfg(test)]
fn soft() -> SoftPasskey {
    SoftPasskey::new(true)
}

/// Registers `soft` on `site` from the page at `origin`.
#[cfg(test)]
fn register(
    site: &PasskeySite,
    soft: &mut SoftPasskey,
    origin: &str,
) -> Result<SavedPasskey, PasskeyCheckError> {
    let (options, registering) = site.start_registration(&asif(), Vec::new()).unwrap();
    let credential = soft
        .do_registration(Url::parse(origin).unwrap(), options)
        .unwrap();
    site.finish_registration(&credential, &registering)
}

/// What the browser offers for a discoverable sign-in. The software
/// passkey cannot discover its own credentials, so the test names it.
#[cfg(test)]
fn offer(
    soft: &mut SoftPasskey,
    mut options: RequestChallengeResponse,
    saved: &SavedPasskey,
    origin: &str,
) -> PublicKeyCredential {
    options.public_key.allow_credentials.push(AllowCredentials {
        type_: "public-key".into(),
        id: saved.credential_id().to_vec(),
        transports: None,
    });
    soft.do_authentication(Url::parse(origin).unwrap(), options)
        .unwrap()
}

/// Signs `soft` in on `site` with the passkey `saved`, from `origin`.
#[cfg(test)]
fn sign_in(
    site: &PasskeySite,
    soft: &mut SoftPasskey,
    saved: &SavedPasskey,
    origin: &str,
) -> Result<SavedPasskey, PasskeyCheckError> {
    let (options, signing_in) = site.start_sign_in().unwrap();
    let credential = offer(soft, options, saved, origin);
    site.finish_sign_in(&credential, signing_in, saved)
}

#[test]
fn a_passkey_registers_signs_in_and_keeps_its_counter_through_the_store() {
    let site = site("https://blog.example.com/");
    let mut soft = soft();
    let saved = register(&site, &mut soft, ORIGIN).unwrap();
    assert_eq!(
        saved.credential_id().len(),
        32,
        "the software passkey names it in 32 bytes"
    );
    let stored = SavedPasskey::from_store(saved.credential_id().to_vec(), saved.json().into());
    let after = sign_in(&site, &mut soft, &stored, ORIGIN).unwrap();
    assert_ne!(after, stored, "the counter moved, so the store saves it");
    let again = sign_in(&site, &mut soft, &after, ORIGIN).unwrap();
    assert_eq!(again.credential_id(), saved.credential_id());
}

#[test]
fn a_counter_that_goes_back_means_a_copied_passkey() {
    let site = site(ORIGIN);
    let mut soft = soft();
    let saved = register(&site, &mut soft, ORIGIN).unwrap();
    // The store saw this passkey count to 100; the one answering is at 1.
    let mut json: serde_json::Value = serde_json::from_str(saved.json()).unwrap();
    json["cred"]["counter"] = 100.into();
    let ahead = SavedPasskey::from_store(saved.credential_id().to_vec(), json.to_string());
    let refused = sign_in(&site, &mut soft, &ahead, ORIGIN);
    assert_eq!(refused, Err(PasskeyCheckError::Replayed));
}

#[test]
fn an_answer_signed_by_another_key_is_refused() {
    let site = site(ORIGIN);
    let (mut mine, mut theirs) = (soft(), soft());
    let saved = register(&site, &mut mine, ORIGIN).unwrap();
    let other = register(&site, &mut theirs, ORIGIN).unwrap();
    let (options, signing_in) = site.start_sign_in().unwrap();
    let mut credential = offer(&mut theirs, options, &other, ORIGIN);
    credential.raw_id = saved.credential_id().to_vec();
    let refused = site.finish_sign_in(&credential, signing_in, &saved);
    assert_eq!(refused, Err(PasskeyCheckError::BadSignature));
}

#[test]
fn a_passkey_asked_for_by_a_page_of_another_origin_is_refused() {
    let site = site(ORIGIN);
    // A phone or laptop never answers a page of another host, so the
    // page that slips through is the same host on another port.
    let refused = register(&site, &mut soft(), "https://blog.example.com:8443");
    assert_eq!(refused, Err(PasskeyCheckError::WrongSite));
    let mut soft = soft();
    let saved = register(&site, &mut soft, ORIGIN).unwrap();
    let elsewhere = sign_in(&site, &mut soft, &saved, "https://blog.example.com:8443");
    assert_eq!(elsewhere, Err(PasskeyCheckError::WrongSite));
}

#[test]
fn an_answer_to_another_challenge_is_refused() {
    let site = site(ORIGIN);
    let mut soft = soft();
    let saved = register(&site, &mut soft, ORIGIN).unwrap();
    let (options, _) = site.start_sign_in().unwrap();
    let (_, other) = site.start_sign_in().unwrap();
    let credential = offer(&mut soft, options, &saved, ORIGIN);
    let refused = site.finish_sign_in(&credential, other, &saved);
    assert_eq!(refused, Err(PasskeyCheckError::WrongChallenge));
}

#[test]
fn a_site_under_a_folder_or_on_a_port_takes_passkeys_from_its_host() {
    let mut soft = soft();
    for (address, origin) in [
        ("https://example.com/blog/", "https://example.com"),
        ("http://localhost:4000/", "http://localhost:4000"),
    ] {
        let site = site(address);
        let saved = register(&site, &mut soft, origin).unwrap();
        assert!(
            sign_in(&site, &mut soft, &saved, origin).is_ok(),
            "{address}"
        );
    }
    let address = AbsoluteUrl::parse("http://127.0.0.1:4000/").unwrap();
    assert_eq!(
        PasskeySite::for_address(&address).err(),
        Some(PasskeyCheckError::NoHostName)
    );
}

#[test]
fn a_stored_passkey_that_cannot_be_read_signs_nobody_in() {
    let site = site(ORIGIN);
    let mut soft = soft();
    let saved = register(&site, &mut soft, ORIGIN).unwrap();
    let broken = SavedPasskey::from_store(saved.credential_id().to_vec(), "{}".into());
    let other_id = SavedPasskey::from_store(vec![1; 16], saved.json().into());
    for stored in [broken, other_id] {
        let (options, signing_in) = site.start_sign_in().unwrap();
        let credential = offer(&mut soft, options, &saved, ORIGIN);
        let refused = site.finish_sign_in(&credential, signing_in, &stored);
        assert_eq!(refused, Err(PasskeyCheckError::Stored), "{stored:?}");
    }
}
