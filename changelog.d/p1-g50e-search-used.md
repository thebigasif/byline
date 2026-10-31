### Added

- Admin API photo and GIF search (G50, KH-EDT-1): `GET /api/admin/v1/media/search/` says which of Unsplash and GIPHY search the owner turned on and, given `source` and `words`, finds 24 pictures a page through the blog, so the keys never reach the browser. `POST /api/admin/v1/media/search/used/` tells Unsplash a photo went into a post, as its API terms ask. A search that is off, has no key or whose source fails answers 503 under the new `unavailable` problem type, saying what to set.
