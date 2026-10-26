### Added

- `khalid lint --fix` takes filler words and years out of a post's web address. It writes the new `slug:` and adds the old address to `aliases`, so the old address redirects and links keep working. It leaves the address alone if another post or page already has the new one (KH-LNT-3).
