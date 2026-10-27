//! The share image cache: drawn cards kept on disk by content key, so a
//! second build never draws the same card twice (ADR 0016).

use std::fs;

use khalid_share_image::{CardColors, Direction, Rgb, ShareCache, ShareCard, ShareKey};

/// A card like the renderer tests', with a settable title.
fn card(title: &str) -> ShareCard {
    let colors = CardColors::new(
        Rgb([0xfb, 0xf7, 0xf0]),
        Rgb([0x2b, 0x1d, 0x0e]),
        Rgb([0xb4, 0x42, 0x1b]),
    );
    ShareCard::new(title, "Rye and Barley Bakery", colors)
}

/// `card` with its `direction` set.
fn directed(card: &ShareCard, direction: Direction) -> ShareCard {
    let mut card = card.clone();
    card.direction = direction;
    card
}

/// `card` with its colours replaced by `colors`.
fn recolored(card: &ShareCard, colors: CardColors) -> ShareCard {
    let mut card = card.clone();
    card.colors = colors;
    card
}

/// How many files the cache folder holds.
fn files_of(cache: &ShareCache, key: &ShareKey) -> usize {
    let Ok(path) = cache.path(key) else {
        return 0;
    };
    let Some(entries) = path.parent().and_then(|folder| fs::read_dir(folder).ok()) else {
        return 0;
    };
    entries.count()
}

#[test]
fn equal_cards_share_one_file_and_the_second_draw_reads_it() {
    let site = tempfile::tempdir().unwrap();
    let cache = ShareCache::new(site.path());
    let first = cache.draw(&card("How we bake sourdough")).unwrap();
    assert!(!first.from_cache, "the first draw cannot read a cache");
    let second = cache.draw(&card("How we bake sourdough")).unwrap();
    assert!(second.from_cache, "the second draw must read the cache");
    assert_eq!(first.key, second.key);
    assert_eq!(first.png, second.png);
    assert_eq!(files_of(&cache, &first.key), 1, "one file, not two");
}

#[test]
fn a_changed_field_gives_a_new_file_so_no_stale_image_is_served() {
    let site = tempfile::tempdir().unwrap();
    let cache = ShareCache::new(site.path());
    let first = cache.draw(&card("How we bake sourdough")).unwrap();
    let edited = cache.draw(&card("How we bake rye")).unwrap();
    assert_ne!(first.key, edited.key);
    assert!(!edited.from_cache);
    assert_ne!(first.png, edited.png);
    assert_eq!(files_of(&cache, &first.key), 2);
}

#[test]
fn the_key_covers_every_field_and_reads_back_as_file_name_hex() {
    let base = card("How we bake sourdough");
    let colors = base.colors;
    let changed = [
        card("How we bake rye"),
        ShareCard::new("How we bake sourdough", "Another Bakery", colors),
        recolored(
            &base,
            CardColors::new(Rgb([0x00, 0x00, 0x00]), colors.text, colors.accent),
        ),
        recolored(
            &base,
            CardColors::new(colors.background, Rgb([0xff, 0xff, 0xff]), colors.accent),
        ),
        recolored(
            &base,
            CardColors::new(colors.background, colors.text, Rgb([0x00, 0x00, 0x00])),
        ),
        directed(&base, Direction::RightToLeft),
    ];
    let key = ShareKey::of(&base);
    for changed in changed {
        assert_ne!(ShareKey::of(&changed), key, "{changed:?}");
    }
    let text = key.as_str().to_owned();
    assert_eq!(
        ShareKey::parse(&text),
        Some(key),
        "the key reads back as its own file name"
    );
    for refused in ["", "xyz", "ABCDEF0123456789", "0123456789abcdef0"] {
        assert_eq!(ShareKey::parse(refused), None, "{refused}");
    }
    insta::assert_snapshot!(text);
}

#[test]
fn the_key_of_a_set_card_is_pinned_so_a_layout_change_is_deliberate() {
    let key = ShareKey::of(&card("How we bake sourdough"));
    assert_eq!(key.as_str(), "b7bb6adbdbea0307");
}

#[test]
fn a_png_already_in_the_cache_is_returned_without_drawing() {
    let site = tempfile::tempdir().unwrap();
    let cache = ShareCache::new(site.path());
    let key = ShareKey::of(&card("How we bake sourdough"));
    fs::create_dir_all(cache.path(&key).unwrap().parent().unwrap()).unwrap();
    fs::write(cache.path(&key).unwrap(), b"sentinel png").unwrap();
    let drawn = cache.draw(&card("How we bake sourdough")).unwrap();
    assert!(drawn.from_cache);
    assert_eq!(drawn.png, b"sentinel png");
}

#[test]
fn a_cache_folder_that_cannot_be_made_is_an_error_not_a_panic() {
    let site = tempfile::tempdir().unwrap();
    // `data` exists as a file, so `data/share/<key>.png` cannot be made.
    let mut data = site.path().to_path_buf();
    data.push("data");
    fs::write(&data, b"").unwrap();
    let cache = ShareCache::new(site.path());
    assert!(cache.draw(&card("How we bake sourdough")).is_err());
}
