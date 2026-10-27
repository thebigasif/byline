# 0000. The media library keeps each file with its details, named by its hash

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 4, plan sections 4.3 and 5.15, KH-EDT-5.

## Context

Plan section 5.15 asks for a media library: folders, duplicate detection on upload, a focal point that drives every crop, alt text and a caption that live on the file, and storage on disk or in an S3-compatible bucket such as Cloudflare R2. Section 4.3 puts originals in the site's `media/` folder. Section 4.2 says files are the source of truth, and RULES.md BL-SEC-6 says uploads are checked by content, images are re-encoded, and executable types are refused. Nothing decided yet how a file and its details are stored, how the API sends a file, or where the bucket's keys live.

## Decision

- **A file is named by a hash of its bytes.** Its identity is the 16-digit content key `khalid-media` already uses for derived images. A second upload of the same bytes is a duplicate by definition: the API answers 200 with the file already in the library, instead of 201 with a new one. That also makes an upload safe to retry (BL-API-7).
- **Each file keeps its details beside it, as text.** `media/<key>/item.toml` holds the name, folder, type, size, pixel size, alt text, caption, focal point, and who uploaded it when. Local storage keeps the bytes beside it as `media/<key>/<name>.<ext>`, so a file and its details move, back up and sit in git together. The library reads every `item.toml` once at start; there is no database table to drift from the files.
- **Folders are a detail, not a place.** A file's folder, such as `guides/2026`, is a field in `item.toml`. Moving a file to another folder never changes its address, so no post's image breaks.
- **The address keeps the file's name for search engines**: `/media/library/<key>/<name>.<ext>` under the blog's mount, such as `/blog/media/library/3f2a9c1e0b57d0aa/choosing-a-crm.jpg`. It sits in its own folder because `/media/<key>/` already serves the sizes made of post images, so the two can never answer for one address. On disk the file stays in `media/<key>/`. Since the key changes whenever the bytes do, every file is cached as immutable.
- **The type comes from the bytes.** Images: JPEG, PNG and WebP, each decoded and encoded again in its own format, which drops hidden data such as GPS positions after turning the pixels upright. Video: MP4 and WebM. Audio: MP3, M4A, Ogg, WAV and FLAC. Files: PDF, ZIP and plain text. Everything else is refused, including programs, scripts, HTML and SVG. SVG can carry scripts, and there is no SVG sanitiser on the dependency list yet. The extension is set from the type, never taken from the upload.
- **Size limits per kind:** images 20 MB, audio 50 MB, video 100 MB, files 20 MB. An upload is read into memory whole, which keeps this simple and matches what one S3 request can send. Longer videos belong on a video host and an embed block.
- **The API takes the raw bytes.** `POST /api/admin/v1/media/?name=choosing-a-crm.jpg&folder=guides` with the file as the body, as `curl --data-binary @choosing-a-crm.jpg` sends it. No multipart parser is needed. Alt text, caption, focal point and folder change with `PUT /api/admin/v1/media/{id}/`, which carries `updated_at` and answers 409 on a stale copy.
- **A bucket is set in `[media.s3]` of `site.toml`, and its keys only in the environment**, as `KHALID_S3_ACCESS_KEY_ID` and `KHALID_S3_SECRET_ACCESS_KEY`. The settings refuse to load a bucket without them. Khalid signs each request with AWS Signature Version 4 through `khalid-outbound`, so the private-network guard applies and a bucket must have a public address. Requests use the path style, `<endpoint>/<bucket>/<key>`, which R2, AWS S3 and other services accept. Readers fetch files from `public_address`.
- **Focal points are fractions of the width and height in thousandths**, from 0 to 1000, with 500 by 500, the centre, as the default. A crop to any shape keeps the focal point as close to the middle as the picture allows.

## Consequences

- No database migration is needed, and a restore from a backup of the site folder brings back the library whole, with every file's details.
- With a bucket, `media/` holds only `item.toml` files, and the bytes are in the bucket. Losing the bucket loses the files but not their details.
- Starting reads one small file per media item. Ten thousand files is ten thousand reads at start, a few hundred milliseconds. If that grows too slow, the library can keep a cache of the list in `data/`.
- SVG, GIF and AVIF uploads are refused for now. Each is one change to the type list once its sanitiser or encoder passes the dependency policy.
- Deleting a file, and replacing a file's bytes while keeping its address, are not in this task.
