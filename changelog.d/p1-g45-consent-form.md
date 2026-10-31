### Added

- `khalid serve` answers the consent banner's form at `<mount>_consent/`: it keeps the reader's yes or no in a cookie for 180 days and sends them back to their page. A form posted from another site gets 403, and a return address outside the blog goes to the blog's home (G45, KH-SEO-12).

### Changed

- `serve_with` hands its first render to its own function, so it is back under the 80-line limit.
