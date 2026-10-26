//! Content keys, the widths an image is offered in, and the cache on disk.

use std::fs;

use khalid_media::{ContentKey, Format, MediaCache, Variant, widths};

#[test]
fn the_same_bytes_get_the_same_key_and_any_change_a_new_one() {
    let photo = b"pretend these are the bytes of hero.jpg".to_vec();
    let key = ContentKey::of(&photo);
    assert_eq!(key, ContentKey::of(&photo));
    assert_eq!(key.as_str().len(), 16);
    assert!(
        key.as_str()
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    let mut edited = photo;
    edited.push(0);
    assert_ne!(key, ContentKey::of(&edited));
}

#[test]
fn a_large_image_gets_every_breakpoint_up_to_the_widest() {
    assert_eq!(widths(4000), [320, 480, 640, 800, 1024, 1280, 1600, 2048]);
    assert_eq!(widths(2048), [320, 480, 640, 800, 1024, 1280, 1600, 2048]);
}

#[test]
fn an_image_is_never_scaled_up() {
    assert_eq!(widths(900), [320, 480, 640, 800, 900]);
    assert_eq!(widths(300), [300]);
    assert_eq!(widths(320), [320]);
    assert_eq!(widths(0), [0_u32; 0]);
}

#[test]
fn variants_are_named_by_width_and_format() {
    let variant = Variant {
        format: Format::Webp,
        width: 800,
        height: 450,
    };
    assert_eq!(variant.file_name(), "800.webp");
    assert_eq!(Format::Jpeg.extension(), "jpg");
    assert_eq!(Format::Webp.media_type(), "image/webp");
}

#[test]
fn the_cache_keeps_files_under_data_media_by_key() {
    let site = tempfile::tempdir().unwrap();
    let cache = MediaCache::new(site.path());
    let key = ContentKey::of(b"hero");
    let variant = Variant {
        format: Format::Webp,
        width: 480,
        height: 270,
    };
    assert_eq!(
        cache.read(&key, &variant).unwrap(),
        None,
        "nothing made yet"
    );

    let path = cache.write(&key, &variant, b"webp bytes").unwrap();
    let real_site = fs::canonicalize(site.path()).unwrap();
    assert_eq!(
        path.strip_prefix(&real_site).unwrap(),
        format!("data/media/{key}/480.webp").as_str()
    );
    assert_eq!(
        cache.read(&key, &variant).unwrap().as_deref(),
        Some(&b"webp bytes"[..])
    );

    let folder = path.parent().unwrap();
    let names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["480.webp"], "no partial file is left behind");
}

#[cfg(unix)]
#[test]
fn a_cache_folder_linked_outside_the_site_is_refused() {
    let site = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), {
        let mut link = site.path().to_path_buf();
        link.push("data");
        link
    })
    .unwrap();
    let cache = MediaCache::new(site.path());
    let variant = Variant {
        format: Format::Png,
        width: 320,
        height: 320,
    };
    assert!(
        cache
            .write(&ContentKey::of(b"logo"), &variant, b"png")
            .is_err()
    );
}
