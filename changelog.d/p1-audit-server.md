### Added

- `khalid audit --server <address>` crawls a running server, such as `khalid serve` on this machine or the live site, instead of building the site. Each request goes to that server while every address stays the site's own, so canonicals and translations still match. Over HTTP the audit also checks that HTTPS pages send `Strict-Transport-Security` (KH-LNT-5).
