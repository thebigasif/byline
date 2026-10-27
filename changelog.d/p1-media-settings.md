### Added

- `site.toml` takes a `[media.s3]` table to keep the media library in an S3-compatible bucket such as Cloudflare R2, with the bucket's keys read from `KHALID_S3_ACCESS_KEY_ID` and `KHALID_S3_SECRET_ACCESS_KEY`; without it, media stays on disk in `media/`.
