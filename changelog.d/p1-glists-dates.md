### Added

- A collection whose `routes.toml` pattern has date folders, such as `/news/{year}/{month}/{slug}/`, gets a page for each year and month at those folders, such as `/news/2026/09/`, titled "September 2026". Date pages are kept out of search results unless `[seo.index]` says otherwise (G12).
