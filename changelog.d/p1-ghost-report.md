### Added

- A Ghost import has a dry-run report in plain sentences: what comes in, which slugs were kept or changed, how many old addresses are mapped, what waits for front matter, which members-only posts became drafts, which code injection and cards are never carried, and what failed or stayed behind with the reason. `ghost::Plan::write` writes the import into a site folder, each file atomically, and nothing when any of its files already exists (KH-API-7, BL-RUN-9).
