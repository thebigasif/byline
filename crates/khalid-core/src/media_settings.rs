//! Where a site keeps its media library, from the `[media]` table of
//! `site.toml`.

use serde::Deserialize;

use crate::{AbsoluteUrl, Secret};

/// The media library's settings. Leave the table out to keep every file on
/// disk in the site's `media/` folder. Add `[media.s3]` to keep the files in
/// an S3-compatible bucket, such as Cloudflare R2, instead:
///
/// ```toml
/// [media.s3]
/// endpoint = "https://0c4a2b.r2.cloudflarestorage.com"
/// bucket = "field-notes-media"
/// region = "auto"
/// prefix = "blog/"
/// public_address = "https://media.example.com/"
/// ```
///
/// The bucket's two keys never go in the file. They come from the
/// environment, see [`S3Keys`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct MediaSettings {
    /// The bucket, or `None` to keep files on disk.
    pub s3: Option<S3Settings>,
}

/// An S3-compatible bucket that holds the media library's files.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct S3Settings {
    /// The service's address, such as `https://s3.eu-central-1.amazonaws.com`.
    pub endpoint: AbsoluteUrl,
    /// The bucket's name.
    pub bucket: String,
    /// The region the bucket is in. R2 takes `auto`, the default.
    #[serde(default = "auto_region")]
    pub region: String,
    /// Put before every file's name in the bucket, such as `blog/`, so one
    /// bucket can serve several sites. Empty by default.
    #[serde(default)]
    pub prefix: String,
    /// Where readers fetch the files, such as the bucket's public address
    /// or a CDN in front of it.
    pub public_address: AbsoluteUrl,
}

fn auto_region() -> String {
    "auto".to_owned()
}

impl S3Settings {
    /// The first rule the settings break, as the key it is under and what is
    /// wrong, or `None` when they are fine.
    pub(crate) fn mistake(&self) -> Option<(&'static str, &'static str)> {
        let bucket_name = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        if !(3..=63).contains(&self.bucket.len()) || !self.bucket.chars().all(bucket_name) {
            return Some((
                "bucket",
                "a bucket name is 3 to 63 lowercase letters, digits and hyphens",
            ));
        }
        if self.region.is_empty() || self.region.chars().any(char::is_whitespace) {
            return Some((
                "region",
                "the region is one word, such as auto or eu-central-1",
            ));
        }
        let prefix_ok = self.prefix.is_empty()
            || (self.prefix.ends_with('/')
                && !self.prefix.starts_with('/')
                && self.prefix.split('/').all(|part| part != ".."));
        if !prefix_ok {
            return Some((
                "prefix",
                "the prefix is empty, or a folder name ending in /, such as blog/",
            ));
        }
        None
    }
}

/// The two keys that let Khalid write to the bucket, read from the
/// environment so they never sit in `site.toml` or a backup of it.
#[derive(Debug)]
#[non_exhaustive]
pub struct S3Keys {
    /// The access key's identity, from `KHALID_S3_ACCESS_KEY_ID`.
    pub access_key_id: Secret<String>,
    /// The secret that signs each request, from `KHALID_S3_SECRET_ACCESS_KEY`.
    pub secret_access_key: Secret<String>,
}
