-- The SerpDad connection (KH-API-6, docs/decisions/2026-10-31-connect-to-serpdad.md).
-- serpdad_links holds the identity of the one integration key SerpDad sends
-- drafts with, never the key; no row means the connection is off.
-- serpdad_drafts records each draft that arrived with that key, keeping the
-- newest 20 per site. A draft's lint counts are NULL when the linter could
-- not run. Times are UTC, written by Khalid's clock as RFC 3339 text to the
-- second. Migrations are forward-only (RULES.md BL-DB-2).
CREATE TABLE serpdad_links (
    site_id INTEGER PRIMARY KEY REFERENCES sites (id) ON DELETE CASCADE,
    key_id TEXT NOT NULL CHECK (length(key_id) = 16 AND key_id NOT GLOB '*[^0-9a-f]*'),
    connected_at TEXT NOT NULL CHECK (julianday(connected_at) IS NOT NULL)
) STRICT;

CREATE TABLE serpdad_drafts (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    post_id TEXT NOT NULL CHECK (length(post_id) > 0),
    title TEXT NOT NULL,
    blocks INTEGER CHECK (blocks IS NULL OR blocks >= 0),
    warnings INTEGER CHECK (warnings IS NULL OR warnings >= 0),
    received_at TEXT NOT NULL CHECK (julianday(received_at) IS NOT NULL),
    CHECK ((blocks IS NULL) = (warnings IS NULL))
) STRICT;

CREATE INDEX serpdad_drafts_by_site ON serpdad_drafts (site_id, id);
