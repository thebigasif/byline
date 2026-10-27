### Added

- `khalid serve` keeps a media library: files uploaded through the Admin API are served from the blog's own address under `media/`, cached by browsers for a year since a changed file gets a new address, or redirected to the bucket's public address when `[media.s3]` is set.
