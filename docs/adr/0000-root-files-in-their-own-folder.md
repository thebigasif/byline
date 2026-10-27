# 0000. Root files go in their own folder, and the IndexNow key in the blog's

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 1, audit brief G3 (KH-SEO-5, KH-SEO-11, KH-INS-3).

## Context

Search engines read `robots.txt` and verification files only at the root of a host, such as `https://www.example.com/robots.txt`. A blog mounted at `/blog/` writes its whole output for that folder, so its `robots.txt` lands at `/blog/robots.txt`, where no crawler looks, and the sitemap line in it is never read.

The blog does not own the host's root. The main site does, and it may already have a `robots.txt`. Khalid cannot write there, and must not pretend it can.

The `IndexNow` key file has the opposite need. `IndexNow` accepts addresses inside the folder that holds the key file, so the file belongs in the blog's own folder, at `IndexNow::key_location`, as the ADR "IndexNow from `khalid-seo`, and no sitemap pings" decided. Nothing wrote it, so every engine refused the pings.

## Decision

- **The key file is a page of the blog.** When `[seo.indexnow]` is set, `khalid build` writes `<key>.txt` at the top of the output folder and `khalid serve` answers it at the same address, `/blog/<key>.txt` on the demo site. It holds the key and nothing else.
- **Root files go in `_root/` inside the output folder** when the mount is not `/`. The folder holds the host's `robots.txt` and any verification files from the settings. The build says where the folder is and what to do with it, in the text output and as `root` in `--json`.
- **Inside the output, not beside it.** A folder beside `public/` would be written outside the folder the person named with `--out`, which surprises them and leaves files behind. The cost: a host that uploads the whole output also serves `/blog/_root/robots.txt`, which crawlers ignore, since only the host root counts.
- **The root `robots.txt` is the blog's.** Every line in it is about the mount folder, plus the sitemap line, so it is safe as the whole file for a host with none, and safe to paste into the main site's file. It opens with a comment that says so.
- **`khalid serve` answers `/robots.txt` too** when the mount is not `/`, so a main site can pass that one address through to the blog with the same proxy rule.
- **`khalid doctor` checks the host's `/robots.txt`.** It fetches it through `Outbound` and reports a problem when it is missing or does not name the blog's sitemap index. The check is skipped with `--offline`, and when the mount is `/`, where the blog's own file is the host's.

## Consequences

- A blog at the root of its host sees no change: its `robots.txt` was already in the right place, and no `_root/` folder is written.
- Verification files come from the settings once G11 adds them to `site.toml`; the folder and the serve route take whatever `SeoSite::root_files` holds.
- The person still has to copy the files to the main site, or add a proxy rule. Doctor tells them when they have not.
