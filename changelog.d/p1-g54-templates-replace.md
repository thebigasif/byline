### Added

- `PUT /api/admin/v1/schema-templates/` puts a new list in place of the site's schema templates. Each template is checked as `khalid serve` checks `site.toml` at start, so a template Google could not read is refused with the template it is in, and a change made on an old copy is a 409 (KH-SEO-4).
