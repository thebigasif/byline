//! The files HTTPS keeps in `data/https/`: the certificate and its key.

use khalid_https::{HttpsError, HttpsFolder, Issued};
use rcgen::{CertificateParams, KeyPair, date_time_ymd};

/// A certificate for `name` valid through the first three months of 2026,
/// as it is issued: the chain, then its key.
#[cfg(test)]
fn issued(name: &str) -> Issued {
    let mut params = CertificateParams::new(vec![name.to_owned()]).unwrap();
    params.not_before = date_time_ymd(2026, 1, 1);
    params.not_after = date_time_ymd(2026, 4, 1);
    let key = KeyPair::generate().unwrap();
    let chain = params.self_signed(&key).unwrap().pem();
    Issued {
        chain,
        key: key.serialize_pem(),
    }
}

#[tokio::test]
async fn an_empty_folder_has_no_account_and_no_certificate() {
    let site = tempfile::tempdir().unwrap();
    let folder = HttpsFolder::new(site.path().to_path_buf());

    assert!(folder.account().await.unwrap().is_none());
    assert!(folder.certificate().await.unwrap().is_none());
}

#[tokio::test]
async fn a_saved_certificate_is_read_back_and_only_its_owner_can_read_the_key() {
    let site = tempfile::tempdir().unwrap();
    let mut path = site.path().to_path_buf();
    path.push("https");
    let folder = HttpsFolder::new(path.clone());

    let saved = folder.save_certificate(&issued("blog.test")).await.unwrap();
    let read = folder.certificate().await.unwrap().unwrap();

    assert_eq!(read.not_after(), saved.not_after());
    assert!(read.covers("blog.test"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.push("key.pem");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "{mode:o}");
    }
}

#[tokio::test]
async fn a_certificate_that_cannot_be_read_is_refused_before_anything_is_written() {
    let site = tempfile::tempdir().unwrap();
    let folder = HttpsFolder::new(site.path().to_path_buf());
    let broken = Issued {
        chain: "not a certificate".to_owned(),
        key: issued("blog.test").key,
    };

    let error = folder.save_certificate(&broken).await.unwrap_err();

    assert!(matches!(error, HttpsError::NoCertificate), "{error:?}");
    assert!(folder.certificate().await.unwrap().is_none());
}
