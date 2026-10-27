//! An account with a certificate authority over ACME, against a small test
//! service, through the one outbound client.

#[cfg(test)]
mod acme_service;

use khalid_https::{AccountKey, Acme, AgreedToTerms, Directory, HttpsError};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};

/// The outbound client, reaching the test service on loopback.
#[cfg(test)]
fn outbound() -> Outbound {
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    Outbound::with_resolver(Limits::default(), resolver).unwrap()
}

#[tokio::test]
async fn a_new_account_agrees_to_the_terms_and_gets_its_address() {
    let service = acme_service::start().await;
    let directory = Directory::Other(service.directory.clone());

    let (acme, key) = Acme::sign_up(&outbound(), &directory, AgreedToTerms)
        .await
        .unwrap();

    assert!(acme.id().ends_with("/account/1"), "{}", acme.id());
    assert_eq!(service.accounts(), 1);
    assert_eq!(
        format!("{key:?}"),
        "AccountKey(hidden)",
        "the key never shows"
    );
}

#[tokio::test]
async fn a_saved_account_key_signs_in_again_without_a_new_account() {
    let service = acme_service::start().await;
    let directory = Directory::Other(service.directory.clone());
    let (first, key) = Acme::sign_up(&outbound(), &directory, AgreedToTerms)
        .await
        .unwrap();

    let saved = key.to_json().unwrap();
    let again = Acme::sign_in(&outbound(), AccountKey::from_json(&saved).unwrap())
        .await
        .unwrap();

    assert_eq!(again.id(), first.id());
    assert_eq!(service.accounts(), 1);
}

#[test]
fn a_file_that_is_not_a_saved_key_is_refused() {
    let error = AccountKey::from_json(b"{\"id\": 1}").unwrap_err();

    assert!(matches!(error, HttpsError::AccountFile { .. }), "{error:?}");
}

#[tokio::test]
async fn an_inward_directory_is_refused_by_the_outbound_guard() {
    let public_only = Outbound::with_resolver(
        Limits::default(),
        Resolver::for_tests(Vec::new(), Reach::Public),
    )
    .unwrap();
    let directory = Directory::Other("http://127.0.0.1:9/dir".to_owned());

    let error = Acme::sign_up(&public_only, &directory, AgreedToTerms)
        .await
        .unwrap_err();

    assert!(matches!(error, HttpsError::Acme { .. }), "{error:?}");
}
