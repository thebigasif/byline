-- The alerts raised for a site's owner (KH-GRO-6), one row per finding,
-- such as stale-post:en.old-post. A key here is not raised again until a
-- check finds it cleared and it is dropped. Keys older than 30 days are
-- dropped too. Times are UTC, written by Khalid's clock as RFC 3339 text
-- to the second. Migrations are forward-only (RULES.md BL-DB-2).
CREATE TABLE alerts_raised (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    key TEXT NOT NULL CHECK (length(key) > 0),
    raised_at TEXT NOT NULL CHECK (julianday(raised_at) IS NOT NULL),
    PRIMARY KEY (site_id, key)
) STRICT, WITHOUT ROWID;
