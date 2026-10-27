### Added

- `khalid-db` can bring the content index up to date after a save by writing again only the posts that changed, in one transaction that leaves the index as it was if any row breaks a rule (KH-API-1, KH-OPS-4).
