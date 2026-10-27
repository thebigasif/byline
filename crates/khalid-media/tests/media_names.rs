//! The names a media item is filed and served under: its key, file name,
//! folder and focal point, each checked once when it is made.

use khalid_core::Locale;
use khalid_media::{ContentKey, FileName, FocalPoint, Folder, MediaType};

#[test]
fn a_content_key_is_sixteen_lowercase_hex_digits() {
    assert_eq!(ContentKey::of(b"x").as_str().len(), 16);
    let key = ContentKey::of(b"choosing a crm");
    assert_eq!(ContentKey::parse(key.as_str()), Some(key));
    for bad in [
        "",
        "3f2a9c1e0b57d0a",
        "3F2A9C1E0B57D0AA",
        "3f2a9c1e0b57d0ag",
        "../../etc/passwd",
    ] {
        assert_eq!(ContentKey::parse(bad), None, "{bad}");
    }
}

#[test]
fn a_file_name_is_a_slug_of_the_upload_with_the_types_extension() {
    let cases = [
        ("Choosing a CRM.JPG", MediaType::Jpeg, "choosing-a-crm.jpg"),
        (
            "C:\\Users\\asif\\Größe Tabelle.png",
            MediaType::Png,
            "groesse-tabelle.png",
        ),
        ("../../etc/passwd", MediaType::Text, "passwd.txt"),
        ("report.pdf.exe", MediaType::Pdf, "report-pdf.pdf"),
        ("    .jpg", MediaType::Jpeg, "file.jpg"),
        ("东京", MediaType::Webp, "file.webp"),
    ];
    let german = Locale::try_new("de").unwrap();
    for (upload, media_type, want) in cases {
        assert_eq!(FileName::new(upload, media_type, &german).as_str(), want);
    }
    let english = Locale::try_new("en").unwrap();
    assert_eq!(
        FileName::new("Q3 numbers", MediaType::Zip, &english).as_str(),
        "q3-numbers.zip"
    );
}

#[test]
fn a_stored_file_name_must_be_a_slug_with_a_known_extension() {
    for good in ["choosing-a-crm.jpg", "q3-numbers.zip"] {
        assert!(FileName::try_from(good.to_owned()).is_ok(), "{good}");
    }
    for bad in ["choosing a crm.jpg", "run.exe", "noextension", "../x.jpg"] {
        assert!(FileName::try_from(bad.to_owned()).is_err(), "{bad}");
    }
}

#[test]
fn a_folder_is_up_to_five_slugs() {
    assert_eq!(Folder::root().as_str(), "");
    assert_eq!(Folder::try_new("").unwrap(), Folder::root());
    assert_eq!(
        Folder::try_new("/guides/2026/").unwrap().as_str(),
        "guides/2026"
    );
    for bad in [
        "../up",
        "guides//2026",
        "Guides",
        "a/b/c/d/e/f",
        "with space",
    ] {
        assert!(Folder::try_new(bad).is_err(), "{bad}");
    }
}

#[test]
fn a_focal_point_is_in_thousandths_and_defaults_to_the_centre() {
    assert_eq!(
        FocalPoint::default(),
        FocalPoint::try_new(500, 500).unwrap()
    );
    assert!(FocalPoint::try_new(1000, 0).is_ok());
    assert!(FocalPoint::try_new(0, 1001).is_err());
}
