-- The speed and uptime watch (C6): each look `khalid serve` takes at a
-- page of the site's public address, as a reader would, with the HTTP
-- status it got, or NULL when nothing answered, and how many milliseconds
-- it took. Looks older than 30 days are dropped. Times are UTC, written by
-- Khalid's clock as RFC 3339 text to the second. Migrations are
-- forward-only (RULES.md BL-DB-2).
CREATE TABLE site_watch (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    page TEXT NOT NULL CHECK (length(page) > 0),
    looked_at TEXT NOT NULL CHECK (julianday(looked_at) IS NOT NULL),
    status INTEGER CHECK (status IS NULL OR status BETWEEN 100 AND 599),
    millis INTEGER NOT NULL CHECK (millis >= 0),
    PRIMARY KEY (site_id, page, looked_at)
) STRICT, WITHOUT ROWID;
