### Added

- The SerpDad draft target: `POST /api/admin/v1/posts/` answers a new draft with `lint`, the publish gate's findings on it with its blocks and warnings counted, and `admin_path`, where it opens in the admin. A new post also takes an `excerpt`, kept as its `description`. A finished article sent from SerpDad lands as a linted draft in under 5 seconds (KH-API-6).
