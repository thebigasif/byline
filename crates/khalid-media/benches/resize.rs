//! The resize path: one variant encoded from a decoded original, and a
//! whole image derived when the cache already has every file.
//!
//! ```text
//! cargo bench -p khalid-media --bench resize
//! ```

use std::io::Cursor;
use std::sync::LazyLock;

use divan::Bencher;
use image::{ImageFormat, Rgb, RgbImage};
use khalid_media::{Format, MediaCache, Original, derive};

fn main() {
    divan::main();
}

/// A 1600 by 1000 photo stand-in, encoded as JPEG once for every benchmark.
static PHOTO: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let image = RgbImage::from_fn(1600, 1000, |x, y| {
        let channel = |value: u32| u8::try_from(value % 256).unwrap_or(0);
        Rgb([channel(x), channel(y), channel(x ^ y)])
    });
    let mut bytes = Cursor::new(Vec::new());
    let _ = image.write_to(&mut bytes, ImageFormat::Jpeg);
    bytes.into_inner()
});

/// Resize to 800 px wide and encode, in each format a page offers.
#[divan::bench(args = [Format::Webp, Format::Jpeg])]
fn encode_800_wide(bencher: Bencher<'_, '_>, format: Format) {
    let Ok(original) = Original::decode(&PHOTO) else {
        return;
    };
    let variant = original.variant(format, 800);
    bencher.bench_local(|| original.encode(divan::black_box(&variant)));
}

/// Decode the original only, the part of every call to `derive`.
#[divan::bench]
fn decode(bencher: Bencher<'_, '_>) {
    bencher.bench_local(|| Original::decode(divan::black_box(&PHOTO)));
}

/// A whole image when every file is already cached: what a republish costs.
#[divan::bench(sample_count = 20)]
fn derive_when_cached(bencher: Bencher<'_, '_>) {
    let Ok(site) = tempfile::tempdir() else {
        return;
    };
    let cache = MediaCache::new(site.path());
    if derive(&PHOTO, &cache).is_err() {
        return;
    }
    bencher.bench_local(|| derive(divan::black_box(&PHOTO), &cache));
}
